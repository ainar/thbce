// SPDX-License-Identifier: GPL-3.0-or-later
//! Programmation des relances des systèmes : fiche `C_Ein_Programmation des relances des
//! systèmes` (Th-BCE 2020 §8.5).
//!
//! La relance bascule les consignes à leur valeur d'occupation quelques heures avant le
//! basculement prévu par les scénarios. Elle n'est appliquée qu'en calcul des consommations.
//!
//! Lectures : `docs/questions.md`, Q43.

use crate::entree::{ProgrammationChauffage, ProgrammationRefroidissement};

use super::emission::ConsignesRelance;

/// « θext_reg_sup » : limite de température extérieure au-delà de laquelle la durée de relance
/// de l'optimiseur est nulle, en °C (tableau 91, valeur conventionnelle).
pub const THETA_EXT_REG_SUP_C: f64 = 15.0;

/// Nombre de pas de temps à venir des indicateurs de consigne lus par la relance : `{pch(t)}`
/// pour `t = h` à `h + 6` (tableau 91).
pub const HORIZON_RELANCE: usize = 7;

/// « Δtrelance_ch(h) », en heures (tableau 92 et éq. 844).
///
/// `p_ch` est l'indicateur de consigne du pas de temps courant (0 : inoccupation courte ;
/// −1 : inoccupation prolongée). La durée retenue est celle du type d'inoccupation en cours.
pub fn duree_relance_ch_h(
    t: ProgrammationChauffage,
    p_ch: f64,
    theta_ext_c: f64,
    theta_ext_base_c: f64,
) -> usize {
    let prolongee = p_ch < 0.0;
    match (t, prolongee) {
        (ProgrammationChauffage::Horloge, false)
        | (ProgrammationChauffage::HorlogeControleAmbiance, false) => 2,
        (ProgrammationChauffage::Horloge, true) => 6,
        (ProgrammationChauffage::HorlogeControleAmbiance, true) => 4,
        (ProgrammationChauffage::Optimiseur, false) => 1,
        (ProgrammationChauffage::Optimiseur, true) => {
            // Éq. (844) : variation linéaire entre 0 et 3 h, arrondie à l'entier le plus proche.
            let d = (3.0 / (theta_ext_base_c - THETA_EXT_REG_SUP_C)
                * (theta_ext_c - THETA_EXT_REG_SUP_C))
                .round();
            // Conversion d'un entier compris entre 0 et 3.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let d = d.clamp(0.0, 3.0) as usize;
            d
        }
    }
}

/// « Δtrelance_fr(h) », en heures (tableau 93) ; 0 sans horloge (fonctionnement permanent).
pub fn duree_relance_fr_h(t: ProgrammationRefroidissement, p_fr: f64) -> usize {
    let prolongee = p_fr < 0.0;
    match (t, prolongee) {
        (ProgrammationRefroidissement::Horloge, false)
        | (ProgrammationRefroidissement::HorlogeControleAmbiance, false) => 1,
        (ProgrammationRefroidissement::Horloge, true) => 3,
        (ProgrammationRefroidissement::HorlogeControleAmbiance, true) => 2,
        (ProgrammationRefroidissement::SansHorloge, _) => 0,
    }
}

/// Conditions d'un pas de temps pour la relance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionsRelance {
    /// « Autch,eff(j) » et « Autfr,eff(j) ».
    pub aut_ch: bool,
    /// Voir `aut_ch`.
    pub aut_fr: bool,
    /// `{pch(t)}` pour `t = h` à `h + 6`.
    pub p_ch: [f64; HORIZON_RELANCE],
    /// `{pfr(t)}` pour `t = h` à `h + 6`.
    pub p_fr: [f64; HORIZON_RELANCE],
    /// « θext(h) », en °C.
    pub theta_ext_c: f64,
    /// « θext_base », en °C.
    pub theta_ext_base_c: f64,
    /// « θiich_+ » et « θiifr_+ » : consignes en occupation normale, en °C.
    pub theta_ii_ch_plus_c: f64,
    /// Voir `theta_ii_ch_plus_c`.
    pub theta_ii_fr_plus_c: f64,
}

/// Programmation des relances d'un groupe et son état d'un pas de temps au suivant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Relance {
    /// « Typepgrm_ch ».
    pub chauffage: ProgrammationChauffage,
    /// « Typepgrm_fr » ; `None` pour un groupe non climatisé.
    pub refroidissement: Option<ProgrammationRefroidissement>,
    /// Consignes de relance du pas de temps précédent (éq. 845 au premier pas de temps).
    pub precedent: ConsignesRelance,
}

impl Relance {
    /// Programmation au début du calcul (éq. 845).
    pub fn nouvelle(
        chauffage: ProgrammationChauffage,
        refroidissement: Option<ProgrammationRefroidissement>,
    ) -> Self {
        Self {
            chauffage,
            refroidissement,
            precedent: ConsignesRelance::SANS_RELANCE,
        }
    }

    /// Consignes de relance du pas de temps (éq. 846 et 847) ; l'état avance d'un pas.
    pub fn pas(&mut self, c: &ConditionsRelance) -> ConsignesRelance {
        let prec = self.precedent;
        // Éq. (846).
        let ch = if !c.aut_ch || c.p_ch[0] >= 1.0 {
            0.0
        } else if prec.theta_ii_ch_relance_c < c.theta_ii_ch_plus_c {
            let dt =
                duree_relance_ch_h(self.chauffage, c.p_ch[0], c.theta_ext_c, c.theta_ext_base_c);
            if declenche(&c.p_ch, dt) {
                c.theta_ii_ch_plus_c
            } else {
                0.0
            }
        } else {
            prec.theta_ii_ch_relance_c
        };
        // Éq. (847).
        let fr = match self.refroidissement {
            Some(t) if c.aut_fr && c.p_fr[0] < 1.0 => {
                if t == ProgrammationRefroidissement::SansHorloge {
                    c.theta_ii_fr_plus_c
                } else if prec.theta_ii_fr_relance_c > c.theta_ii_fr_plus_c {
                    if declenche(&c.p_fr, duree_relance_fr_h(t, c.p_fr[0])) {
                        c.theta_ii_fr_plus_c
                    } else {
                        100.0
                    }
                } else {
                    prec.theta_ii_fr_relance_c
                }
            }
            _ => 100.0,
        };
        self.precedent = ConsignesRelance {
            theta_ii_ch_relance_c: ch,
            theta_ii_fr_relance_c: fr,
        };
        self.precedent
    }
}

/// « p(h + Δt) = 1 et p(h + Δt − 1) < 1 » (éq. 846 et 847). Avec `Δt = 0`, `p(h)` est
/// inférieur à 1 dans la branche qui appelle ce test : pas de relance.
fn declenche(p: &[f64; HORIZON_RELANCE], dt: usize) -> bool {
    (1..HORIZON_RELANCE).contains(&dt) && p[dt] >= 1.0 && p[dt - 1] < 1.0
}

/// « irelance^gr(h) » (éq. 848) : vrai si `θiich_relance(h) > 0` ou `θiifr_relance(h) < 100`.
pub fn en_relance(c: &ConsignesRelance) -> bool {
    c.theta_ii_ch_relance_c > 0.0 || c.theta_ii_fr_relance_c < 100.0
}
