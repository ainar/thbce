// SPDX-License-Identifier: GPL-3.0-or-later
//! Échangeur de géocooling d'une génération de refroidissement (Th-BCE 2020 §8.30, fiche
//! « C_GEN_Echangeur_Geocooling », éq. 1569 à 1595).
//!
//! Générateur de refroidissement équivalent, placé en tête d'une génération de froid en
//! cascade, qui refroidit l'eau du bâtiment avec l'eau d'une source géothermique (sondes ou
//! nappe, fiche 8.26) à travers un échangeur à contre-courant. Sa source amont est celle des
//! générateurs thermodynamiques sur eau (`sources_amont.rs`) : température amont, état d'un pas
//! au suivant et auxiliaires amont sont calculés par la boucle commune des générations.
//! Lectures : `docs/questions/Q252.md`.

use super::calculs_generation::{Energie, Poste};
use super::generation::{EntreesAppel, FonctionGenerateur, SortiesAppel};
use super::sources_amont::efficacite_echangeur;
use crate::entree::EcoulementsEchangeur;

/// « ρe » : masse volumique de l'eau, en kg/m³ (tableau 253).
pub const RHO_E_KG_M3: f64 = 1000.0;
/// « ce » : capacité calorifique massique de l'eau, en J/(kg.K) (tableau 253).
pub const C_E_J_KG_K: f64 = 4190.0;
/// « θr,aval » imposée pour la puissance nominale, en °C (§8.30.3.7).
pub const THETA_R_AVAL_NOM_C: f64 = 20.0;
/// « θr,amont » imposée pour la puissance nominale, en °C (§8.30.3.7).
pub const THETA_R_AMONT_NOM_C: f64 = 10.0;

/// « Type_Association » (tableau 253).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Association {
    /// Base et complément.
    BaseEtComplement,
    /// Alterné (éq. 1587).
    Alterne,
}

/// Lecture du cas sans échangeur intermédiaire (`docs/questions/Q252.md`, point 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LectureSansEchangeur {
    /// Lecture retenue, littérale : « il n'est jamais utilisé », `Pmax = 0` (éq. 1588).
    #[default]
    JamaisUtilise,
    /// Variante : l'eau de la source passe directement dans le réseau, `θd,aval = θr,amont`
    /// (éq. 1585), et `Pmax` suit l'éq. (1581).
    CircuitDirect,
}

/// Échangeur de géocooling résolu (§8.30.2, tableau 253) ; grandeurs d'une machine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EchangeurGeocooling {
    /// « qv,aval,nom », en m³/h.
    pub qv_aval_nom_m3_h: f64,
    /// « Δθaval », en K.
    pub delta_theta_aval_k: f64,
    /// « qv,amont,nom », en m³/h.
    pub qv_amont_nom_m3_h: f64,
    /// « Δθamont », en K.
    pub delta_theta_amont_k: f64,
    /// « θr,source,lim », en °C.
    pub theta_r_source_lim_c: f64,
    /// « UAgeo,nom », en W/K ; `None` sans échangeur intermédiaire (éq. 1570).
    pub ua_geo_nom_w_k: Option<f64>,
    /// « Pcprim,nom », en W.
    pub p_cprim_nom_w: f64,
    /// « Type_Association ».
    pub association: Association,
    /// « Rdim ».
    pub rdim: f64,
    /// Lecture du cas sans échangeur.
    pub lecture: LectureSansEchangeur,
}

/// Détail d'un pas de temps (une machine).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PasGeocooling {
    /// « Is_On(h) » (éq. 1573).
    pub is_on: bool,
    /// « qv,aval(h) », en m³/h (éq. 1574).
    pub qv_aval_m3_h: f64,
    /// « θd,aval,pc(h) », en °C (éq. 1580).
    pub theta_d_aval_pc_c: f64,
    /// « Pmax(h) », en W (éq. 1581, 1587, 1588).
    pub p_max_w: f64,
    /// « Qfou(h) », en Wh (éq. 1589).
    pub q_fou_wh: f64,
    /// « Waux,pro(h) », en Wh (éq. 1593).
    pub w_aux_pro_wh: f64,
}

impl EchangeurGeocooling {
    /// Débit-capacité de l'eau pour un débit en m³/h, en W/K.
    fn capacite_w_k(qv_m3_h: f64) -> f64 {
        qv_m3_h * RHO_E_KG_M3 * C_E_J_KG_K / 3600.0
    }

    /// « θd,aval,pc » : température de départ aval à débit nominal côté source, en °C.
    // Th-BCE 2020 §8.30.3.4, éq. (1576) à (1580).
    fn theta_d_aval_pc_c(
        &self,
        ua_w_k: f64,
        qv_aval_m3_h: f64,
        theta_r_aval_c: f64,
        theta_r_amont_c: f64,
    ) -> f64 {
        let q_min = qv_aval_m3_h.min(self.qv_amont_nom_m3_h);
        let q_max = qv_aval_m3_h.max(self.qv_amont_nom_m3_h);
        // Éq. (1576) : « 3600 × UA / (ρe × ce × min(qv)) », débits en m³/h.
        let nut = ua_w_k / Self::capacite_w_k(q_min);
        // Éq. (1577) à (1579) : échangeur à contre-courant.
        let c = q_min / q_max;
        let eps = efficacite_echangeur(EcoulementsEchangeur::ContreCourant, nut, c);
        // Éq. (1580).
        theta_r_aval_c + eps * q_min / qv_aval_m3_h * (theta_r_amont_c - theta_r_aval_c)
    }

    /// « Pn,gen,fr » d'une machine, en W : méthode du §8.30.3.4 aux débits nominaux, avec
    /// `θr,aval = 20 °C` et `θr,amont = 10 °C` (§8.30.3.7).
    pub fn pn_w(&self) -> f64 {
        let q = self.qv_aval_nom_m3_h;
        if q <= 0.0 {
            return 0.0;
        }
        let theta_d = match (self.ua_geo_nom_w_k, self.lecture) {
            (Some(ua), _) => self.theta_d_aval_pc_c(ua, q, THETA_R_AVAL_NOM_C, THETA_R_AMONT_NOM_C),
            (None, LectureSansEchangeur::CircuitDirect) => THETA_R_AMONT_NOM_C,
            (None, LectureSansEchangeur::JamaisUtilise) => THETA_R_AVAL_NOM_C,
        };
        (Self::capacite_w_k(q) * (THETA_R_AVAL_NOM_C - theta_d)).max(0.0)
    }

    /// Calcul d'une machine au pas de temps (§8.30.3, éq. 1571 à 1593) : `q_req_wh` est la
    /// demande d'une machine, `theta_amont_c` la température de la source amont, `theta_aval_c`
    /// la température aval de la génération (moyenne départ/retour du réseau d'eau froide).
    pub fn pas(&self, q_req_wh: f64, theta_amont_c: f64, theta_aval_c: f64) -> PasGeocooling {
        // Éq. (1571) et (1572).
        let theta_r_aval = theta_aval_c + self.delta_theta_aval_k / 2.0;
        let theta_r_amont = theta_amont_c - self.delta_theta_amont_k / 2.0;
        // Éq. (1573).
        let is_on = theta_r_amont < self.theta_r_source_lim_c && theta_r_amont < theta_r_aval;
        // Éq. (1574) et (1582) : débit aval d'après la demande, plafonné au débit nominal.
        let qv_aval = (q_req_wh.max(0.0) / Self::capacite_w_k(1.0) / self.delta_theta_aval_k)
            .min(self.qv_aval_nom_m3_h);
        let mut p = PasGeocooling {
            is_on,
            qv_aval_m3_h: qv_aval,
            theta_d_aval_pc_c: theta_r_aval,
            ..PasGeocooling::default()
        };
        if is_on && qv_aval > 0.0 {
            let theta_d = match (self.ua_geo_nom_w_k, self.lecture) {
                (Some(ua), _) => {
                    Some(self.theta_d_aval_pc_c(ua, qv_aval, theta_r_aval, theta_r_amont))
                }
                // Variante : éq. (1585).
                (None, LectureSansEchangeur::CircuitDirect) => Some(theta_r_amont),
                // Éq. (1588).
                (None, LectureSansEchangeur::JamaisUtilise) => None,
            };
            if let Some(theta_d) = theta_d {
                p.theta_d_aval_pc_c = theta_d;
                // Éq. (1581) : positive ou nulle.
                p.p_max_w = (Self::capacite_w_k(qv_aval) * (theta_r_aval - theta_d)).max(0.0);
            }
        }
        // Éq. (1587) : association alternée, arrêt si la puissance ne suffit pas.
        if self.association == Association::Alterne && q_req_wh > p.p_max_w {
            p.p_max_w = 0.0;
        }
        // Éq. (1589).
        p.q_fou_wh = q_req_wh.max(0.0).min(p.p_max_w);
        // Éq. (1593) : circulateur du circuit primaire au prorata du débit aval, quand
        // l'échangeur fonctionne (lecture Q252, point 3).
        if p.p_max_w > 0.0 && self.qv_aval_nom_m3_h > 0.0 {
            p.w_aux_pro_wh = qv_aval / self.qv_aval_nom_m3_h * self.p_cprim_nom_w;
        }
        p
    }

    /// « idfougen = 2 » (tableau 253).
    pub fn fonction(&self) -> FonctionGenerateur {
        FonctionGenerateur::Refroidissement
    }

    /// « Rdim × Pn,gen,fr », en W.
    pub fn pn_gen_tot_w(&self, poste: Poste) -> f64 {
        if poste == Poste::Refroidissement {
            self.rdim * self.pn_w()
        } else {
            0.0
        }
    }

    /// Fonction d'appel du générateur (§8.30.3.7, éq. 1589 à 1595).
    pub fn appel(&self, e: &EntreesAppel) -> SortiesAppel {
        let rdim = self.rdim;
        let p = if e.fonction == Poste::Refroidissement {
            self.pas(e.q_req_wh / rdim, e.theta_amont_c, e.theta_aval_c)
        } else {
            PasGeocooling::default()
        };
        let q_fou = p.q_fou_wh * rdim;
        let w_aux = p.w_aux_pro_wh * rdim;
        SortiesAppel {
            energie: Energie::Electricite,
            // Éq. (1594).
            q_cef_wh: w_aux,
            q_cons_wh: w_aux,
            q_fou_wh: q_fou,
            // Éq. (1590).
            q_rest_wh: (e.q_req_wh - q_fou).max(0.0),
            // Éq. (1591).
            tau_charge: if p.p_max_w > 0.0 {
                (p.q_fou_wh / p.p_max_w).min(1.0)
            } else {
                0.0
            },
            phi_vc_wh: 0.0,
            w_aux_pro_wh: w_aux,
            // Éq. (1595), hors auxiliaires amont (ajoutés par la génération).
            eta_eff: if w_aux > 0.0 { q_fou / w_aux } else { 0.0 },
            // Éq. (1592) : positive pour un générateur de refroidissement.
            phi_rejet_wh: q_fou,
            q_prelec_wh: 0.0,
            r_fonct_ecs: 0.0,
            q_cef_aux_elec_wh: 0.0,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn geo(ua: Option<f64>, association: Association) -> EchangeurGeocooling {
        EchangeurGeocooling {
            qv_aval_nom_m3_h: 3.6,
            delta_theta_aval_k: 5.0,
            qv_amont_nom_m3_h: 3.6,
            delta_theta_amont_k: 3.0,
            theta_r_source_lim_c: 16.0,
            ua_geo_nom_w_k: ua,
            p_cprim_nom_w: 100.0,
            association,
            rdim: 1.0,
            lecture: LectureSansEchangeur::JamaisUtilise,
        }
    }

    fn proche(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9 * b.abs().max(1.0), "{a} ≠ {b}");
    }

    #[test]
    fn puissance_nominale_a_la_main() {
        // 3,6 m³/h de part et d'autre : capacité 1 000 × 4 190 × 3,6 / 3 600 = 4 190 W/K ;
        // UA = 4 190 W/K : NUT = 1, C = 1, ε = 1/2 (éq. 1578) ; θd = 20 + 0,5 × (10 − 20) = 15 °C ;
        // Pn = 4 190 × (20 − 15) = 20 950 W.
        let g = geo(Some(4190.0), Association::BaseEtComplement);
        proche(g.pn_w(), 20_950.0);
        // Sans échangeur, lecture retenue : jamais utilisé (éq. 1588).
        assert_eq!(geo(None, Association::BaseEtComplement).pn_w(), 0.0);
    }

    #[test]
    fn pas_a_la_main() {
        let g = geo(Some(4190.0), Association::BaseEtComplement);
        // θaval = 17,5 °C : θr,aval = 20 °C ; θamont = 11,5 °C : θr,amont = 10 °C (éq. 1571,
        // 1572). Demande 10 475 Wh : qv,aval = 10 475 / (1 000 × 4 190 / 3 600) / 5 = 1,8 m³/h
        // (éq. 1574). min = 1,8, max = 3,6 : C = 0,5 ; NUT = 4 190 / 2 095 = 2 ;
        // ε = (1 − e^(−1)) / (1 − 0,5 e^(−1)) (éq. 1579) ; θd = 20 + ε × (10 − 20) (éq. 1580) ;
        // Pmax = 2 095 × 10 ε (éq. 1581).
        let e1 = libm::exp(-1.0);
        let eps = (1.0 - e1) / (1.0 - 0.5 * e1);
        let p = g.pas(10_475.0, 11.5, 17.5);
        assert!(p.is_on);
        proche(p.qv_aval_m3_h, 1.8);
        proche(p.theta_d_aval_pc_c, 20.0 - 10.0 * eps);
        proche(p.p_max_w, 20_950.0 * eps);
        // ε ≈ 0,7746 : Pmax ≈ 16 228 W > demande : tout est fourni.
        proche(p.q_fou_wh, 10_475.0);
        // Éq. (1593) : 1,8 / 3,6 × 100 W.
        proche(p.w_aux_pro_wh, 50.0);
        // Source trop chaude (θr,amont = 16,5 ≥ 16) : arrêt (éq. 1573), pas de circulateur.
        let p = g.pas(10_475.0, 18.0, 17.5);
        assert!(!p.is_on);
        assert_eq!((p.p_max_w, p.q_fou_wh, p.w_aux_pro_wh), (0.0, 0.0, 0.0));
        // Alterné : la demande dépasse Pmax, l'échangeur est arrêté (éq. 1587).
        let a = geo(Some(4190.0), Association::Alterne);
        let p = a.pas(30_000.0, 11.5, 17.5);
        assert_eq!(p.q_fou_wh, 0.0);
        let b = g.pas(30_000.0, 11.5, 17.5);
        // Base et complément : Pmax au débit nominal 3,6 m³/h, C = 1, NUT = 1 : ε = 0,5 ;
        // 4 190 × 10 × 0,5 = 20 950 W.
        proche(b.q_fou_wh, 20_950.0);
    }

    #[test]
    fn sans_echangeur() {
        let mut g = geo(None, Association::BaseEtComplement);
        assert_eq!(g.pas(10_475.0, 11.5, 17.5).q_fou_wh, 0.0);
        // Variante : circuit direct, θd = θr,amont = 10 °C : Pmax = 2 095 × 10 = 20 950 W.
        g.lecture = LectureSansEchangeur::CircuitDirect;
        let p = g.pas(10_475.0, 11.5, 17.5);
        proche(p.p_max_w, 20_950.0);
        proche(p.q_fou_wh, 10_475.0);
    }
}
