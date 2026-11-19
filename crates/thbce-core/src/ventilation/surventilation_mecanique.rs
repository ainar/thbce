// SPDX-License-Identifier: GPL-3.0-or-later
//! Fiche `C_VEN_Fonctions de rafraichissement par surventilation mécanique des systèmes de
//! ventilation` (Th-BCE 2020 §6.12, p. 461 à 469, éq. 758 à 780) : régulation saisonnière et
//! horaire du rafraîchissement, surfaces de conduit. Calculs Th-C et Th-D seulement ; le
//! branchement sur le système de ventilation (débits, bypass, air neuf, traitements coupés,
//! ventilateurs) est dans [`super::ventilation_projet`]. Lectures : `docs/questions/Q230.md`.

use crate::climat::calendrier::case_scenario;
use crate::entree::{ConsignesSurventilation, SurventilationMecanique, TypeSurventilation};
use crate::ventilation::bouche_conduit::RAT_DEB_COND;

/// Consignes d'une saison, prêtes pour le calcul.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Consignes {
    /// « hleg,deb_rafnoc » et « hleg,fin_rafnoc » d'un rafraîchissement nocturne, en h.
    pub periode_h: Option<(u8, u8)>,
    /// « θsd_dec_rafnoc », en °C.
    pub theta_sd_dec_c: f64,
    /// « θsd_arr_rafnoc », en °C.
    pub theta_sd_arr_c: f64,
    /// « Δθlim_rafnoc », en K.
    pub delta_theta_lim_k: f64,
    /// « θext_rafnoc », en °C.
    pub theta_ext_c: f64,
}

impl Consignes {
    fn depuis(c: &ConsignesSurventilation) -> Self {
        Self {
            periode_h: c.h_leg_deb_h.zip(c.h_leg_fin_h),
            theta_sd_dec_c: c.theta_sd_dec_c,
            theta_sd_arr_c: c.theta_sd_arr_c,
            delta_theta_lim_k: c.delta_theta_lim_k,
            theta_ext_c: c.theta_ext_c,
        }
    }
}

/// Rafraîchissement par surventilation mécanique d'un système, résolu depuis la saisie, avec
/// l'état de sa régulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurventilationResolue {
    /// « israfnoc ».
    pub type_surventilation: TypeSurventilation,
    /// Consignes d'indice « été ».
    pub ete: Consignes,
    /// Consignes d'indice « mis » (types 2 et 4) ; celles de l'été sinon (sans emploi).
    pub mi_saison: Consignes,
    /// « Pvent_rafnoc_rep » et « Pvent_rafnoc_souf », en W (éq. 779).
    pub p_vent_w: (f64, f64),
    /// « irafnoc_comm(h−1) ».
    pub i_comm: bool,
    /// « irafnoc_activé(h) » du pas de temps en cours.
    pub actif: bool,
}

impl SurventilationResolue {
    /// Résolution de la saisie validée.
    pub fn nouvelle(s: &SurventilationMecanique) -> Self {
        let ete = Consignes::depuis(&s.ete);
        Self {
            type_surventilation: s.type_surventilation,
            ete,
            mi_saison: s.mi_saison.as_ref().map_or(ete, Consignes::depuis),
            p_vent_w: (s.p_vent_rafnoc_rep_w, s.p_vent_rafnoc_souf_w),
            i_comm: false,
            actif: false,
        }
    }

    /// Régulation du pas de temps `h` (§6.12.3.2 et 6.12.3.3, éq. 762 à 774) : met à jour
    /// `irafnoc_comm` et `irafnoc_activé`.
    ///
    /// `theta_an_c` : « θan(h) », température de l'air neuf ; `theta_op_c` : « θop,fin^gr(h−1) »
    /// des groupes desservis ; `aut_fr_pro` : « MIN Autfr_pro^gr(j) » des groupes desservis
    /// (lecture `docs/questions/Q230.md`, point 2).
    pub fn reguler(
        &mut self,
        h: usize,
        theta_an_c: f64,
        theta_op_c: impl Iterator<Item = f64> + Clone,
        aut_fr_pro: bool,
    ) {
        let annee = self.type_surventilation.toute_l_annee();
        // Éq. (762) à (765) : autorisation saisonnière et jeu de consignes.
        let aut_saison = annee || aut_fr_pro;
        let c = if aut_fr_pro { self.ete } else { self.mi_saison };
        // Éq. (768) à (770) : heure légale au début du pas de temps, de 0 à 23.
        let aut_tempo = if self.type_surventilation.est_nocturne() {
            c.periode_h
                .is_some_and(|(deb, fin)| aut_tempo(hleg_debut_pas(h), deb, fin))
        } else {
            true
        };
        // Éq. (771) et (772).
        let act = theta_op_c.clone().fold(f64::NEG_INFINITY, f64::max);
        let desact = theta_op_c.fold(f64::INFINITY, f64::min);
        if !act.is_finite() {
            // Aucun groupe desservi : rien à rafraîchir.
            self.i_comm = false;
            self.actif = false;
            return;
        }
        // Éq. (773) et (774).
        let aut_rafnoc = aut_rafnoc(theta_an_c, c.theta_ext_c, act, c.delta_theta_lim_k);
        self.i_comm = i_comm(act, desact, c.theta_sd_dec_c, c.theta_sd_arr_c, self.i_comm);
        // Éq. (766).
        self.actif = aut_saison && aut_tempo && aut_rafnoc && self.i_comm;
    }
}

/// « hleg(h) » de la fiche : heure légale au début du pas de temps `h`, de 0 à 23 (tableau
/// 75). Le calendrier donne l'heure légale de fin de pas de temps, de 1 à 24 (§4.1.3).
pub fn hleg_debut_pas(h: usize) -> u8 {
    case_scenario(h) - 1
}

/// « Auttempo(h) » d'un rafraîchissement nocturne (éq. 768 et 769).
pub fn aut_tempo(hleg: u8, deb: u8, fin: u8) -> bool {
    if fin >= deb {
        // Éq. (768).
        hleg >= deb && hleg <= fin
    } else {
        // Éq. (769).
        hleg >= deb || hleg <= fin
    }
}

/// « Autrafnoc(h) » : autorisation selon la température extérieure (éq. 773) :
/// `θan ≥ θext_rafnoc` et `θan ≤ θop,act − Δθlim_rafnoc`.
pub fn aut_rafnoc(theta_an_c: f64, theta_ext_c: f64, theta_op_act_c: f64, d_lim_k: f64) -> bool {
    theta_an_c >= theta_ext_c && theta_an_c <= theta_op_act_c - d_lim_k
}

/// « irafnoc_comm(h) » : besoin de rafraîchissement selon la température intérieure, avec
/// hystérésis (éq. 774, figure 81) : `θop,desact ≥ θsd_arr` et (`θop,act ≥ θsd_dec` ou
/// `irafnoc_comm(h−1)`).
pub fn i_comm(
    theta_op_act_c: f64,
    theta_op_desact_c: f64,
    theta_sd_dec_c: f64,
    theta_sd_arr_c: f64,
    i_comm_prec: bool,
) -> bool {
    theta_op_desact_c >= theta_sd_arr_c && (theta_op_act_c >= theta_sd_dec_c || i_comm_prec)
}

/// Heure de la période nuit (21 h à 8 h) admise pour le début ou la fin du rafraîchissement
/// nocturne (éq. 767).
pub fn heure_de_nuit(h: u8) -> bool {
    h >= 21 && h <= 23 || h <= 8
}

/// Test de cohérence de la période nocturne en début de simulation (éq. 767) : `Some(message)`
/// si elle est refusée.
pub fn periode_incoherente(deb: u8, fin: u8) -> Option<&'static str> {
    if !heure_de_nuit(deb) {
        return Some(
            "l'heure légale de début de période de rafraîchissement nocturne doit être comprise \
             entre 21 h et 8 h",
        );
    }
    if !heure_de_nuit(fin) {
        return Some(
            "l'heure légale de fin de période de rafraîchissement nocturne doit être comprise \
             entre 21 h et 8 h",
        );
    }
    let fin_avant = (fin >= 21 && deb >= 21 && fin < deb)
        || (fin <= 8 && deb <= 8 && fin < deb)
        || (fin >= 21 && deb <= 8);
    fin_avant.then_some(
        "sur la période nuit (21 h à 8 h), l'heure légale de fin de période de rafraîchissement \
         nocturne doit être ultérieure à l'heure légale de début",
    )
}

/// « Acond,rep » ou « Acond,souf » recalculée en présence de surventilation mécanique (éq. 760
/// et 761) : `MAX(|qmax| ; |qrafnoc|) × Ratdebcond`, en m².
pub fn a_cond_rafnoc_m2(q_max_m3_h: f64, q_rafnoc_m3_h: f64) -> f64 {
    q_max_m3_h.abs().max(q_rafnoc_m3_h.abs()) * RAT_DEB_COND
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporisation_a_la_main() {
        // Éq. (768) : de 0 h à 6 h.
        assert!(aut_tempo(0, 0, 6) && aut_tempo(6, 0, 6) && !aut_tempo(7, 0, 6));
        // Éq. (769) : de 22 h à 6 h, à cheval sur minuit.
        assert!(aut_tempo(22, 22, 6) && aut_tempo(23, 22, 6) && aut_tempo(3, 22, 6));
        assert!(!aut_tempo(21, 22, 6) && !aut_tempo(7, 22, 6) && !aut_tempo(12, 22, 6));
    }

    #[test]
    fn coherence_de_la_periode() {
        // Éq. (767).
        assert!(periode_incoherente(22, 6).is_none());
        assert!(periode_incoherente(0, 6).is_none());
        assert!(periode_incoherente(21, 23).is_none());
        assert!(periode_incoherente(12, 6).is_some());
        assert!(periode_incoherente(22, 10).is_some());
        assert!(periode_incoherente(23, 22).is_some());
        assert!(periode_incoherente(6, 2).is_some());
        assert!(periode_incoherente(2, 22).is_some());
    }

    #[test]
    fn hysteresis_a_la_main() {
        // Figure 81 : déclenchement à 26 °C, arrêt à 22 °C.
        let f = |act: f64, desact: f64, prec: bool| i_comm(act, desact, 26.0, 22.0, prec);
        assert!(!f(25.0, 24.0, false));
        assert!(f(26.0, 24.0, false));
        assert!(f(25.0, 24.0, true));
        assert!(!f(25.0, 21.9, true));
        // Éq. (773) : θan entre θext_rafnoc = 12 °C et θop,act − Δθlim = 25 − 3 = 22 °C.
        assert!(aut_rafnoc(12.0, 12.0, 25.0, 3.0) && aut_rafnoc(22.0, 12.0, 25.0, 3.0));
        assert!(!aut_rafnoc(11.9, 12.0, 25.0, 3.0) && !aut_rafnoc(22.1, 12.0, 25.0, 3.0));
    }

    #[test]
    fn surface_de_conduit() {
        // Éq. (760) : MAX(90 ; 300) × 0,05 = 15 m².
        assert_eq!(a_cond_rafnoc_m2(-90.0, 300.0), 15.0);
        assert_eq!(a_cond_rafnoc_m2(400.0, 300.0), 20.0);
    }

    #[test]
    fn heure_au_debut_du_pas() {
        // Pas de temps 0 : de 0 h à 1 h (heure d'hiver), début 0 h.
        assert_eq!(hleg_debut_pas(0), 0);
        assert_eq!(hleg_debut_pas(23), 23);
    }

    #[test]
    fn regulation_a_la_main() {
        let c = Consignes {
            periode_h: Some((22, 6)),
            theta_sd_dec_c: 26.0,
            theta_sd_arr_c: 22.0,
            delta_theta_lim_k: 3.0,
            theta_ext_c: 12.0,
        };
        let mut s = SurventilationResolue {
            type_surventilation: TypeSurventilation::NocturneEte,
            ete: c,
            mi_saison: c,
            p_vent_w: (50.0, 60.0),
            i_comm: false,
            actif: false,
        };
        // Pas de temps 0 : 0 h, dans la période ; θop 27 et 24 °C, θan 18 °C ≤ 27 − 3.
        s.reguler(0, 18.0, [27.0, 24.0].into_iter(), true);
        assert!(s.i_comm && s.actif);
        // Hors saison de refroidissement (type 1) : pas d'activation, besoin conservé.
        s.reguler(1, 18.0, [25.0, 24.0].into_iter(), false);
        assert!(s.i_comm && !s.actif);
        // Midi : hors période nocturne.
        s.reguler(12, 18.0, [27.0, 24.0].into_iter(), true);
        assert!(s.i_comm && !s.actif);
        // Freecooling toute l'année : consignes de mi-saison hors saison.
        s.type_surventilation = TypeSurventilation::FreecoolingAnnee;
        s.mi_saison.theta_sd_arr_c = 25.0;
        s.reguler(12, 18.0, [27.0, 24.0].into_iter(), false);
        assert!(!s.i_comm && !s.actif);
    }
}
