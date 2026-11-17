// SPDX-License-Identifier: GPL-3.0-or-later
//! Indicateur de confort d'été : fiche `C_BAT_Confort_d'été` (Th-BCE 2020 §13.5).
//!
//! L'indicateur degrés-heures (DH) cumule, sur les pas de temps occupés de la période de
//! confort adaptatif, le dépassement de la température d'inconfort chaud par la température
//! opérative du groupe. Il est issu du calcul Th-D et exprimé par groupe (§13.5.1).
//!
//! La fiche définit aussi `Δθconf_adapt(h)`, écart entre la température d'inconfort chaud et
//! la consigne de refroidissement, que la gestion de l'ouverture des baies emploie (§5.13.3.1.1,
//! éq. 242 et 243).
//!
//! Périmètre : usages d'habitation ; sans système modifiant la sensation thermique
//! (`Δθcorr_syst = 0` : brasseurs d'air et humidification hors de la première version).
//!
//! Lectures : `docs/questions.md`, Q40.

use crate::confort::indicateurs_confort::theta_op_inc_max_c;
use crate::conventions::ModeCalcul;

/// « Δθop_min_max » : écart entre les températures opératives d'inconfort chaud minimale et
/// maximale, en °C (§13.5.2, constantes).
pub const DELTA_THETA_OP_MIN_MAX_C: f64 = 2.0;

/// « Cat_amb » : catégorie d'ambiance retenue pour le confort d'été.
///
/// Th-BCE 2020 §13.5.3.3, tableau 340 : 1 pour les bâtiments à usage d'habitation comme pour
/// les autres usages.
pub const CAT_AMB: u8 = 1;

/// Dernière heure légale de la nuit au sens de l'éq. (2544) : la journée va de `hleg > 6` à
/// `hleg ≤ 22`.
pub const HLEG_FIN_NUIT: u8 = 6;
/// Dernière heure légale de la journée au sens de l'éq. (2544).
pub const HLEG_FIN_JOURNEE: u8 = 22;

/// « Seuil bas » de degrés-heures d'inconfort, en °C.h : en dessous, le groupe est jugé
/// confortable sans refroidissement complémentaire.
///
/// Th-BCE 2020 §13.3.2, tableau 334 (fiche du forfait de consommations de refroidissement). Le
/// « seuil haut » (`DH_max = DH_maxcat`) est fixé par l'annexe de l'article R. 172-4 du code de
/// la construction et de l'habitation, présente dans le dépôt
/// (`docs/reference/texte/annexe-r172-4-chapitres-1-a-3.txt`, chapitre III, §IV « Valeurs de
/// DH_maxcat » : 1 250 °C.h en catégorie de contraintes extérieures 1 et 1 850 °C.h en
/// catégorie 2 pour les maisons ; valeurs par surface moyenne des logements pour le collectif) ;
/// il relève du lot 7 (exigences et forfait de refroidissement).
pub const SEUIL_BAS_DH_C_H: f64 = 350.0;

/// « θop_conf_ch(h) » : température d'inconfort chaud, en °C.
///
/// Th-BCE 2020 §13.5.3.3 :
/// - éq. (2543) : `θop_inc_max_C1(j) = max(θiifr+ ; 0,33·θrm(j) + 18,8 + Δθop_inc_C1)`
///   (identique à l'éq. 58 de la fiche des indicateurs de confort, catégorie d'ambiance 1 du
///   tableau 340) ;
/// - « valeur maximale » : la température d'inconfort chaud est limitée à
///   `Δθop_min_max = 2 °C` au-dessus de la consigne de refroidissement ;
/// - éq. (2544), usages d'habitation : si `6 < hleg ≤ 22`,
///   `θop_conf_ch = min(θiifr+ + Δθop_min_max ; θop_inc_max_C1)` ; sinon (période de sommeil),
///   `θop_conf_ch = θiifr+` ;
/// - éq. (2545), autres usages : la première formule à toute heure.
///
/// Les éq. (2544) et (2545) sont imprimées
/// « min(θiifr+ + Δθop_inc_max_C1 op_min_max()) » : les deux arguments du minimum se sont
/// télescopés. La lecture ci-dessus est celle du texte qui les précède (Q40).
///
/// `theta_ii_fr_plus_c` est la consigne de refroidissement en occupation ; `theta_rm_c`, la
/// moyenne glissante de la température extérieure du jour ; `hleg`, l'heure légale de fin du
/// pas de temps, de 0 (minuit) à 23 ([`crate::climat::calendrier::hleg`]).
pub fn theta_op_conf_ch_c(
    habitation: bool,
    hleg: u8,
    theta_ii_fr_plus_c: f64,
    theta_rm_c: f64,
) -> f64 {
    let sommeil = habitation && !(hleg > HLEG_FIN_NUIT && hleg <= HLEG_FIN_JOURNEE);
    if sommeil {
        theta_ii_fr_plus_c
    } else {
        (theta_ii_fr_plus_c + DELTA_THETA_OP_MIN_MAX_C).min(theta_op_inc_max_c(
            theta_ii_fr_plus_c,
            theta_rm_c,
            CAT_AMB,
        ))
    }
}

/// « Δθconf_adapt(h) » : écart entre la température d'inconfort chaud et la consigne de
/// refroidissement en occupation, en °C.
///
/// Th-BCE 2020 §13.5.3.4, éq. (2546) : « si Mode = Th-D »,
/// `Δθconf_adapt = max(0 ; θop_conf_ch(h) − θiifr+)`. « Par construction, cet écart est nul
/// hors période de confort adaptatif » : la formule ne l'annule pourtant pas d'elle-même (à
/// `θrm = 15,9 °C`, `0,33·θrm + 20,8` dépasse 26 °C) ; il est ici annulé hors période, comme
/// le dit le texte (Q40). Hors calcul Th-D, il est nul (décision Q39 point 5).
pub fn delta_theta_conf_adapt_c(
    mode: ModeCalcul,
    is_conf_adapt: bool,
    theta_op_conf_ch_c: f64,
    theta_ii_fr_plus_c: f64,
) -> f64 {
    if mode == ModeCalcul::ThD && is_conf_adapt {
        (theta_op_conf_ch_c - theta_ii_fr_plus_c).max(0.0)
    } else {
        0.0
    }
}

/// « θop_conf_ch_corr(h) » : température d'inconfort chaud corrigée de l'effet des systèmes qui
/// modifient la sensation thermique, en °C.
///
/// Th-BCE 2020 §13.5.3.5, éq. (2547) : `θop_conf_ch_corr = θop_conf_ch + Δθcorr_syst`.
pub fn theta_op_conf_ch_corr_c(theta_op_conf_ch_c: f64, delta_theta_corr_syst_c: f64) -> f64 {
    theta_op_conf_ch_c + delta_theta_corr_syst_c
}

/// Confort d'été d'un groupe à un pas de temps du calcul Th-D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PasConfortEte {
    /// « Isconf_adapt(h) » : pas de temps en période de confort adaptatif.
    pub is_conf_adapt: bool,
    /// « Iocc_zone(h) » : zone occupée.
    pub i_occ_zone: bool,
    /// « θop(h) » : température opérative du groupe à la fin du pas de temps, en °C.
    pub theta_op_c: f64,
    /// « θop_conf_ch_corr(h) » : température d'inconfort chaud corrigée, en °C.
    pub theta_op_conf_ch_corr_c: f64,
    /// Contribution du pas de temps au DH, en °C.h : `max(0 ; θop − θop_conf_ch_corr)` en
    /// occupation et en période de confort adaptatif, 0 sinon (éq. 2551).
    pub dh_c_h: f64,
}

impl PasConfortEte {
    /// Th-BCE 2020 §13.5.3.8, éq. (2551) : terme de la somme, compté si
    /// `Is_occ_zone(h) = 1` et `Is_conf_adapt(h) = 1`.
    pub fn nouveau(
        is_conf_adapt: bool,
        i_occ_zone: bool,
        theta_op_c: f64,
        theta_op_conf_ch_corr_c: f64,
    ) -> Self {
        let dh_c_h = if is_conf_adapt && i_occ_zone {
            (theta_op_c - theta_op_conf_ch_corr_c).max(0.0)
        } else {
            0.0
        };
        Self {
            is_conf_adapt,
            i_occ_zone,
            theta_op_c,
            theta_op_conf_ch_corr_c,
            dh_c_h,
        }
    }
}

/// Cumuls annuels du confort d'été d'un groupe (§13.5.3.7 et 13.5.3.8).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CumulsConfortEte {
    /// « DH » : degrés-heures d'inconfort, en °C.h (éq. 2551).
    pub dh_c_h: f64,
    /// « Nbhinconf » : heures où la température opérative atteint la température d'inconfort,
    /// en h (éq. 2548).
    pub nbh_inconf: u32,
    /// « Nbhinconf+1C » : heures où elle la dépasse d'au moins 1 °C, en h (éq. 2549).
    pub nbh_inconf_1c: u32,
    /// « Nbhinconf+2C » : heures où elle la dépasse d'au moins 2 °C, en h (éq. 2550).
    pub nbh_inconf_2c: u32,
    /// Nombre de pas de temps occupés de la période de confort adaptatif, en h : ceux sur
    /// lesquels portent les indicateurs.
    pub nbh_occ_conf_adapt: u32,
    /// Température opérative maximale atteinte sur l'année, en °C (fin de pas de temps).
    pub theta_op_max_c: f64,
    /// Température opérative maximale atteinte en occupation pendant la période de confort
    /// adaptatif, en °C ; `−∞` si aucun pas de temps n'est concerné.
    pub theta_op_max_occ_c: f64,
}

impl Default for CumulsConfortEte {
    fn default() -> Self {
        Self {
            dh_c_h: 0.0,
            nbh_inconf: 0,
            nbh_inconf_1c: 0,
            nbh_inconf_2c: 0,
            nbh_occ_conf_adapt: 0,
            theta_op_max_c: f64::NEG_INFINITY,
            theta_op_max_occ_c: f64::NEG_INFINITY,
        }
    }
}

impl CumulsConfortEte {
    /// Ajoute un pas de temps.
    ///
    /// Th-BCE 2020 §13.5.3.7, éq. (2548) à (2550) : en période de confort adaptatif et en
    /// occupation, les compteurs sont incrémentés si `θop ≥ θop_conf_ch_corr`, `+ 1` et `+ 2` ;
    /// §13.5.3.8, éq. (2551) : cumul des dépassements.
    pub fn ajouter(&mut self, p: &PasConfortEte) {
        self.theta_op_max_c = self.theta_op_max_c.max(p.theta_op_c);
        if !(p.is_conf_adapt && p.i_occ_zone) {
            return;
        }
        self.nbh_occ_conf_adapt += 1;
        self.theta_op_max_occ_c = self.theta_op_max_occ_c.max(p.theta_op_c);
        let ecart = p.theta_op_c - p.theta_op_conf_ch_corr_c;
        self.nbh_inconf += u32::from(ecart >= 0.0);
        self.nbh_inconf_1c += u32::from(ecart >= 1.0);
        self.nbh_inconf_2c += u32::from(ecart >= 2.0);
        self.dh_c_h += p.dh_c_h;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn temperature_d_inconfort_chaud_calculee_a_la_main() {
        // Habitation, consigne de 26 °C. Éq. (2543) : θop_inc_max_C1 = max(26 ; 0,33·θrm + 20,8).
        // θrm = 18 °C : 0,33 × 18 + 20,8 = 26,74 °C, sous le plafond de 28 °C.
        assert!((theta_op_conf_ch_c(true, 12, 26.0, 18.0) - 26.74).abs() < 1e-12);
        // θrm = 25 °C : 29,05 °C, plafonné à 26 + 2 = 28 °C (« valeur maximale »).
        assert_eq!(theta_op_conf_ch_c(true, 12, 26.0, 25.0), 28.0);
        // θrm = 10 °C : 24,1 °C, relevé à la consigne par l'éq. (2543).
        assert_eq!(theta_op_conf_ch_c(true, 12, 26.0, 10.0), 26.0);
        // Éq. (2544) : la journée va de hleg = 7 à hleg = 22 ; en période de sommeil, la
        // consigne.
        assert_eq!(theta_op_conf_ch_c(true, 6, 26.0, 25.0), 26.0);
        assert_eq!(theta_op_conf_ch_c(true, 7, 26.0, 25.0), 28.0);
        assert_eq!(theta_op_conf_ch_c(true, 22, 26.0, 25.0), 28.0);
        assert_eq!(theta_op_conf_ch_c(true, 23, 26.0, 25.0), 26.0);
        assert_eq!(theta_op_conf_ch_c(true, 0, 26.0, 25.0), 26.0);
        // Éq. (2545) : hors habitation, pas de période de sommeil.
        assert_eq!(theta_op_conf_ch_c(false, 3, 26.0, 25.0), 28.0);
    }

    #[test]
    fn ecart_de_confort_adaptatif() {
        // Éq. (2546) : défini en Th-D seulement, nul hors période de confort adaptatif.
        let d = |mode, periode, seuil| delta_theta_conf_adapt_c(mode, periode, seuil, 26.0);
        assert_eq!(d(ModeCalcul::ThD, true, 28.0), 2.0);
        assert!((d(ModeCalcul::ThD, true, 26.74) - 0.74).abs() < 1e-12);
        assert_eq!(d(ModeCalcul::ThD, true, 26.0), 0.0);
        assert_eq!(d(ModeCalcul::ThD, false, 28.0), 0.0);
        assert_eq!(d(ModeCalcul::ThB, true, 28.0), 0.0);
        assert_eq!(d(ModeCalcul::ThC, true, 28.0), 0.0);
        // Éq. (2547).
        assert_eq!(theta_op_conf_ch_corr_c(28.0, 1.5), 29.5);
    }

    #[test]
    fn cumuls_calcules_a_la_main() {
        let mut c = CumulsConfortEte::default();
        let mut ajouter = |periode, occupe, theta_op, seuil| {
            let p = PasConfortEte::nouveau(periode, occupe, theta_op, seuil);
            c.ajouter(&p);
            p.dh_c_h
        };
        // Hors période ou en inoccupation : rien n'est compté, sauf le maximum annuel.
        assert_eq!(ajouter(false, true, 35.0, 26.0), 0.0);
        assert_eq!(ajouter(true, false, 34.0, 26.0), 0.0);
        // En période et en occupation : 27,5 °C pour un seuil de 28 °C, pas d'inconfort.
        assert_eq!(ajouter(true, true, 27.5, 28.0), 0.0);
        // 28 °C pour 28 °C : l'heure est comptée (« ≥ », éq. 2548), sans degré-heure.
        assert_eq!(ajouter(true, true, 28.0, 28.0), 0.0);
        // 29,5 °C pour 28 °C : 1,5 °C.h ; compté dans Nbhinconf et Nbhinconf+1C.
        assert_eq!(ajouter(true, true, 29.5, 28.0), 1.5);
        // 28,25 °C pour 26 °C (nuit) : 2,25 °C.h ; compté dans les trois.
        assert_eq!(ajouter(true, true, 28.25, 26.0), 2.25);
        assert_eq!(c.dh_c_h, 3.75);
        assert_eq!((c.nbh_inconf, c.nbh_inconf_1c, c.nbh_inconf_2c), (3, 2, 1));
        assert_eq!(c.nbh_occ_conf_adapt, 4);
        assert_eq!(c.theta_op_max_c, 35.0);
        assert_eq!(c.theta_op_max_occ_c, 29.5);
    }
}
