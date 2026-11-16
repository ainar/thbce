// SPDX-License-Identifier: GPL-3.0-or-later
//! Période de confort adaptatif (Th-BCE 2020 §3.1.3, « Détermination de la période de confort
//! adaptatif », éq. 1 et 2).
//!
//! **Lecture retenue (décision Q11, hypothèse du mainteneur, à confirmer par un cas de
//! validation).** Le texte écrit « si IHJ = 1, θrm(j) = 0 ; sinon θrm(j) = 0,8·θrm(j−1) +
//! 0,2·θei,moy(j) », ce qui, pris à la lettre, annulerait la moyenne chaque jour. Ici :
//!
//! - la branche `θrm = 0` initialise la récurrence au premier jour de simulation ;
//! - les autres jours, la récurrence est évaluée au premier pas de temps du jour (IHJ = 1) et sa
//!   valeur est conservée toute la journée ;
//! - le premier et le dernier jour de la période sont le premier et le dernier jour de l'année
//!   où `θrm(j) ≥ 16 °C`.
//!
//! Ce module est isolé à dessein : c'est le premier point à revoir si les cas de validation
//! divergent sur l'indicateur DH.

use crate::conventions::NB_HEURES_AN;

/// Seuil de `θrm` qui ouvre la période de confort adaptatif, en °C.
pub const SEUIL_THETA_RM_C: f64 = 16.0;

/// Poids de la veille dans la moyenne glissante (éq. 1).
const POIDS_VEILLE: f64 = 0.8;
/// Poids de la moyenne journalière dans la moyenne glissante (éq. 1).
const POIDS_JOUR: f64 = 0.2;

/// Résultat de la détermination de la période de confort adaptatif.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfortAdaptatif {
    /// « θrm(j) » : température extérieure moyenne journalière glissante, en °C, pour chaque
    /// jour `j` à partir de 0.
    pub theta_rm_c: Vec<f64>,
    /// « Premier_jour » : indice (à partir de 0) du premier jour où `θrm ≥ 16 °C` ; `None` si
    /// le seuil n'est jamais atteint.
    pub premier_jour: Option<usize>,
    /// « Dernier_jour » : indice du dernier jour où `θrm ≥ 16 °C`.
    pub dernier_jour: Option<usize>,
}

impl ConfortAdaptatif {
    /// Calcule `θrm(j)` et la période à partir des températures extérieures horaires du site
    /// (`Te`, en °C, corrigées de l'altitude).
    pub fn nouveau(te_c: &[f64]) -> Self {
        let nb_jours = te_c.len().min(NB_HEURES_AN) / 24;
        let mut theta_rm_c = Vec::with_capacity(nb_jours);
        for j in 0..nb_jours {
            let theta_rm = if j == 0 {
                // Initialisation de la récurrence.
                0.0
            } else {
                // θei,moy(j) : moyenne des 24 températures horaires du jour j − 1.
                let veille = &te_c[(j - 1) * 24..j * 24];
                let theta_ei_moy = veille.iter().sum::<f64>() / 24.0;
                POIDS_VEILLE * theta_rm_c[j - 1] + POIDS_JOUR * theta_ei_moy
            };
            theta_rm_c.push(theta_rm);
        }
        let premier_jour = theta_rm_c.iter().position(|t| *t >= SEUIL_THETA_RM_C);
        let dernier_jour = theta_rm_c.iter().rposition(|t| *t >= SEUIL_THETA_RM_C);
        Self {
            theta_rm_c,
            premier_jour,
            dernier_jour,
        }
    }

    /// « Isconf_adapt(h) » (éq. 2) : vrai si le jour du pas de temps `h` est compris entre le
    /// premier et le dernier jour de la période, bornes incluses.
    pub fn is_conf_adapt(&self, h: usize) -> bool {
        match (self.premier_jour, self.dernier_jour) {
            (Some(premier), Some(dernier)) => (premier..=dernier).contains(&(h / 24)),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recurrence_calculee_a_la_main() {
        // Te constante à 20 °C : θrm = 0 ; 4 ; 7,2 ; 9,76 ; 11,808 ; 13,4464 ; 14,75712 ;
        // 15,805696 ; 16,6445568 … Le seuil de 16 °C est franchi au jour d'indice 8.
        let c = ConfortAdaptatif::nouveau(&vec![20.0; NB_HEURES_AN]);
        assert_eq!(c.theta_rm_c.len(), 365);
        let attendus = [
            0.0,
            4.0,
            7.2,
            9.76,
            11.808,
            13.4464,
            14.757_12,
            15.805_696,
            16.644_556_8,
        ];
        for (j, attendu) in attendus.iter().enumerate() {
            assert!((c.theta_rm_c[j] - attendu).abs() < 1e-12, "jour {j}");
        }
        assert_eq!(c.premier_jour, Some(8));
        assert_eq!(c.dernier_jour, Some(364));
        assert!(!c.is_conf_adapt(8 * 24 - 1));
        assert!(c.is_conf_adapt(8 * 24));
        assert!(c.is_conf_adapt(8759));
    }

    #[test]
    fn la_moyenne_du_jour_porte_sur_la_veille() {
        // Jour 0 à 24 °C, ensuite 0 °C : θrm(1) = 0,2 × 24 = 4,8 ; θrm(2) = 0,8 × 4,8 = 3,84.
        let mut te = vec![0.0; NB_HEURES_AN];
        te[..24].fill(24.0);
        let c = ConfortAdaptatif::nouveau(&te);
        assert!((c.theta_rm_c[1] - 4.8).abs() < 1e-12);
        assert!((c.theta_rm_c[2] - 3.84).abs() < 1e-12);
    }

    #[test]
    fn sans_periode_si_le_seuil_n_est_jamais_atteint() {
        let c = ConfortAdaptatif::nouveau(&vec![15.9; NB_HEURES_AN]);
        assert_eq!((c.premier_jour, c.dernier_jour), (None, None));
        assert!(!(0..NB_HEURES_AN).any(|h| c.is_conf_adapt(h)));
    }

    #[test]
    fn periode_bornee_par_le_premier_et_le_dernier_franchissement() {
        // Été chaud du jour 150 au jour 249, froid ailleurs ; un redoux isolé en fin d'année
        // repousse le dernier jour, la période restant d'un seul tenant (éq. 2).
        let mut te = vec![5.0; NB_HEURES_AN];
        te[150 * 24..250 * 24].fill(25.0);
        te[340 * 24..350 * 24].fill(30.0);
        let c = ConfortAdaptatif::nouveau(&te);
        let (premier, dernier) = (c.premier_jour.unwrap(), c.dernier_jour.unwrap());
        assert!((151..160).contains(&premier), "{premier}");
        // Après le redoux (θrm ≈ 27 °C au jour 350), θrm retombe sous 16 °C en quatre jours :
        // 5 + 22,3 × 0,8ⁿ < 16 pour n ≥ 4.
        assert!((350..358).contains(&dernier), "{dernier}");
        assert!(c.is_conf_adapt(300 * 24), "la période est d'un seul tenant");
        assert!(!c.is_conf_adapt(100 * 24));
    }
}
