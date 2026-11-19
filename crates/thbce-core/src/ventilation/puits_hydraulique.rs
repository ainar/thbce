// SPDX-License-Identifier: GPL-3.0-or-later
//! Puits hydraulique, ou puits canadien à eau glycolée (Th-BCE 2020 §6.9, avec le prétraitement
//! de l'efficacité de la batterie du §6.3.3.3.4, éq. 399 à 408, et la température d'air neuf
//! des éq. 415 à 417 et 489 à 491).
//!
//! Une batterie eau/air préchauffe ou prérafraîchit l'air neuf d'un simple flux par
//! insufflation, d'un double flux ou d'une centrale de traitement d'air ; l'eau glycolée
//! échange avec le sol dans un conduit enterré, et une pompe la fait circuler. À chaque pas, le
//! puits est d'abord calculé comme s'il était activé (température du sol, éq. 732 à 734,
//! efficacité moyenne des batteries, éq. 735 à 740, températures de l'eau, éq. 741 à 745), puis
//! la gestion de la pompe l'active ou non (éq. 746 et 747), puis la température de l'air neuf
//! (éq. 415) et la consommation de la pompe (éq. 748 et 749) en découlent.
//! Lectures : `docs/questions/Q233.md` (unité du débit, un système par puits, saisons).

use std::f64::consts::PI;

use super::mecanique_sf::C_PA_J_KG_K;
use super::puits_climatique::Sol;
use crate::entree::{GestionPuitsHydraulique, PuitsHydraulique};

/// « ρeau_gl » : masse volumique de l'eau glycolée, en kg/m³ (tableau 71).
pub const RHO_EAU_GL_KG_M3: f64 = 1040.0;
/// « λeau_gl » : conductivité thermique de l'eau glycolée, en W/(m.K) (tableau 71).
pub const LAMBDA_EAU_GL_W_M_K: f64 = 0.496;
/// « μeau_gl » : viscosité dynamique de l'eau glycolée, en kg/(m.s) (tableau 71).
pub const MU_EAU_GL_KG_M_S: f64 = 1.516e-3;
/// « Cpeau_gl » : capacité calorifique massique de l'eau glycolée, en J/(kg.K) (tableau 71).
pub const CP_EAU_GL_J_KG_K: f64 = 3751.0;
/// « Cpair » du tableau 71 : capacité calorifique massique de l'air, en J/(kg.K).
pub const CP_AIR_PH_J_KG_K: f64 = 1004.0;
/// « εbatt_PH_défaut » : efficacité par défaut de la batterie eau/air (éq. 408).
pub const EPS_BATT_DEFAUT: f64 = 0.65;

/// « hi » : coefficient de surface interne entre l'eau glycolée et le conduit, en W/(m².K),
/// formule de Colburn (régime turbulent établi) au débit maximal de la pompe.
///
/// Th-BCE 2020 §6.9.3.2.2, éq. (745). Le débit `Qmax` est en m³/h : le facteur 3 600 de
/// l'équation le ramène à des m³/s (`docs/questions/Q233.md` point 1).
pub fn h_i_w_m2_k(p: &PuitsHydraulique) -> f64 {
    let reynolds =
        RHO_EAU_GL_KG_M3 * 4.0 * p.q_max_m3_h / (3600.0 * PI * p.di_m * MU_EAU_GL_KG_M_S);
    let prandtl = MU_EAU_GL_KG_M_S * CP_EAU_GL_J_KG_K / LAMBDA_EAU_GL_W_M_K;
    0.023 * LAMBDA_EAU_GL_W_M_K / p.di_m * libm::pow(reynolds, 0.8) * libm::pow(prandtl, 0.30)
}

/// « Ug » : coefficient d'échange global entre l'eau glycolée et le sol, en W/(m.K).
///
/// Th-BCE 2020 §6.9.3.2.2, éq. (744).
pub fn u_g_w_m_k(p: &PuitsHydraulique) -> f64 {
    let r_tube = libm::log((p.di_m + 2.0 * p.ep_m) / p.di_m) / (2.0 * PI * p.lambda_tube_w_m_k);
    1.0 / (r_tube + 1.0 / (PI * p.di_m * h_i_w_m2_k(p)))
}

/// « εsol-eau » : efficacité équivalente du capteur enterré, constante au cours de la
/// simulation (débit maximal de la pompe).
///
/// Th-BCE 2020 §6.9.3.2.2, éq. (743).
pub fn efficacite_sol_eau(p: &PuitsHydraulique) -> f64 {
    1.0 - libm::exp(
        -3600.0 * u_g_w_m_k(p) * p.l_m / (p.q_max_m3_h * RHO_EAU_GL_KG_M3 * CP_EAU_GL_J_KG_K),
    )
}

/// « Rdc » : ratio des débits calorifiques, le plus petit sur le plus grand, de l'air neuf
/// (`qm_an_kg_s`, en kg/s) et de l'eau glycolée au débit maximal de la pompe.
///
/// Th-BCE 2020 §6.9.3.2.2, éq. (736) à (738) ; `DCeau` = `Qmax·ρ·Cp/3600`.
pub fn rdc(p: &PuitsHydraulique, qm_an_kg_s: f64) -> f64 {
    let dc_air = qm_an_kg_s * CP_AIR_PH_J_KG_K;
    let dc_eau = p.q_max_m3_h * RHO_EAU_GL_KG_M3 * CP_EAU_GL_J_KG_K / 3600.0;
    let (min, max) = if dc_air < dc_eau {
        (dc_air, dc_eau)
    } else {
        (dc_eau, dc_air)
    };
    if max > 0.0 {
        min / max
    } else {
        0.0
    }
}

/// « θeau-in-PH » : température de l'eau glycolée entrant dans le puits (sortie de la
/// batterie), pour une efficacité moyenne des batteries `eps_batt`, une efficacité du capteur
/// `eps_sol_eau`, la température du sol `theta_sol_c` et la température extérieure.
///
/// Th-BCE 2020 §6.9.3.2.2, éq. (741).
pub fn theta_eau_in_c(eps_sol_eau: f64, eps_batt: f64, theta_sol_c: f64, theta_ext_c: f64) -> f64 {
    let den = eps_sol_eau + eps_batt - eps_batt * eps_sol_eau;
    if den > 0.0 {
        ((eps_sol_eau - eps_batt * eps_sol_eau) * theta_sol_c + eps_batt * theta_ext_c) / den
    } else {
        theta_sol_c
    }
}

/// « θeau-out-PH » : température de l'eau glycolée en sortie du puits (entrée de la batterie).
///
/// Th-BCE 2020 §6.9.3.2.2, éq. (742).
pub fn theta_eau_out_c(eps_sol_eau: f64, theta_eau_in_c: f64, theta_sol_c: f64) -> f64 {
    theta_eau_in_c + eps_sol_eau * (theta_sol_c - theta_eau_in_c)
}

/// « QPH(h) > 0 » : la pompe tourne, selon la saison et la gestion choisie.
///
/// Th-BCE 2020 §6.9.3.2.3, éq. (746) en période de chauffage (`θext < θext,arrêt^hiver` et, en
/// gestion sur les deux températures, `θext < θeau-out`) et éq. (747) hors période de chauffage
/// (`θext > θext,arrêt^été` et, en gestion sur les deux températures, `θext > θeau-out`). Le
/// texte écrit « hiver » au lieu de « été » à l'éq. (747), coquille lue « été »
/// (`docs/questions/Q233.md` point 3).
pub fn pompe_en_marche(
    p: &PuitsHydraulique,
    chauffage: bool,
    theta_ext_c: f64,
    theta_eau_out_c: f64,
) -> bool {
    let sur_eau = p.gestion == GestionPuitsHydraulique::TemperaturesExterieureEtEau;
    if chauffage {
        theta_ext_c < p.theta_ext_arret_hiver_c && (!sur_eau || theta_ext_c < theta_eau_out_c)
    } else {
        theta_ext_c > p.theta_ext_arret_ete_c && (!sur_eau || theta_ext_c > theta_eau_out_c)
    }
}

/// Efficacité de la batterie en préchauffage, déduite de la puissance déclarée `p_ch_kw` au
/// débit d'air neuf de référence `qm_an_hiv_kg_s` (air à −10 °C, eau à 10 °C) ; erreur si elle
/// dépasse 1 (puissance supérieure à ce que le débit admet).
///
/// Th-BCE 2020 §6.3.3.3.4, éq. (401) et (402).
pub fn efficacite_batterie_ch(p_ch_kw: f64, qm_an_hiv_kg_s: f64) -> Result<f64, String> {
    let theta_out = -10.0 + 1000.0 * p_ch_kw / (C_PA_J_KG_K * qm_an_hiv_kg_s);
    let eps = (theta_out + 10.0) / 20.0;
    controle_efficacite(eps, "préchauffage", p_ch_kw)
}

/// Efficacité de la batterie en prérefroidissement (air à 30 °C, eau à 10 °C).
///
/// Th-BCE 2020 §6.3.3.3.4, éq. (406) et (407).
pub fn efficacite_batterie_fr(p_fr_kw: f64, qm_an_ete_kg_s: f64) -> Result<f64, String> {
    let theta_out = 30.0 - 1000.0 * p_fr_kw / (C_PA_J_KG_K * qm_an_ete_kg_s);
    let eps = (30.0 - theta_out) / 20.0;
    controle_efficacite(eps, "prérefroidissement", p_fr_kw)
}

fn controle_efficacite(eps: f64, mode: &str, p_kw: f64) -> Result<f64, String> {
    if eps.is_finite() && eps <= 1.0 {
        Ok(eps)
    } else {
        Err(format!(
            "la puissance de batterie du puits hydraulique en {mode} ({p_kw} kW) est supérieure à \
             la puissance maximale physiquement admissible au débit de ventilation renseigné \
             (efficacité > 1, Th-BCE 2020 §6.3.3.3.4)"
        ))
    }
}

/// Sortie du puits hydraulique pour un débit d'air neuf donné.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SortiePuitsHydraulique {
    /// « θan(h) », en °C : air neuf après la batterie (éq. 415), `θext` si la pompe est
    /// arrêtée (éq. 417).
    pub theta_an_c: f64,
    /// « θeau-out-PH(h) », en °C (éq. 742).
    pub theta_eau_out_c: f64,
    /// « QPH(h) > 0 » : la pompe tourne (éq. 746 et 747).
    pub en_marche: bool,
}

/// État d'un puits hydraulique au cours de la simulation d'un système.
#[derive(Debug, Clone, PartialEq)]
pub struct EtatPuitsHydraulique {
    /// Description saisie.
    pub description: PuitsHydraulique,
    /// « εbatt_PH_ch » et « εbatt_PH_fr » : efficacités de la batterie en saison de chauffage et
    /// hors saison (éq. 402, 407 ou 408).
    eps_batt: (f64, f64),
    eps_sol_eau: f64,
    sol: Option<Sol>,
    theta_sol_c: f64,
    chauffage: bool,
    /// Sortie du pas, calculée avec le débit d'air neuf de la fin du pas précédent (éq. 415 :
    /// « avec les débits d'air neuf calculés à la fin du pas de temps précédent »).
    sortie: SortiePuitsHydraulique,
    /// Débit massique d'air neuf de la fin du pas précédent, en kg/s ; nul : inconnu.
    qm_prec_kg_s: f64,
}

impl EtatPuitsHydraulique {
    /// Puits décrit, avant le premier pas, avec les efficacités de sa batterie.
    pub fn nouveau(description: PuitsHydraulique, eps_batt_ch: f64, eps_batt_fr: f64) -> Self {
        Self {
            eps_sol_eau: efficacite_sol_eau(&description),
            description,
            eps_batt: (eps_batt_ch, eps_batt_fr),
            sol: None,
            theta_sol_c: 0.0,
            chauffage: false,
            sortie: SortiePuitsHydraulique {
                theta_an_c: 0.0,
                theta_eau_out_c: 0.0,
                en_marche: false,
            },
            qm_prec_kg_s: 0.0,
        }
    }

    /// Prépare le pas `h` : sol (calculé au premier pas sur `te_c`), saison (éq. 740 et 416 :
    /// chauffage dès qu'un groupe desservi est en saison de chauffage) et sortie avec le débit de
    /// la fin du pas précédent.
    pub fn preparer(&mut self, h: usize, te_c: &[f64], theta_ext_c: f64, chauffage: bool) {
        let p = self.description;
        let sol = *self
            .sol
            .get_or_insert_with(|| Sol::pour(p.type_sol, p.profondeur_m, te_c));
        self.theta_sol_c = sol.theta_c(h);
        self.chauffage = chauffage;
        self.sortie = if self.qm_prec_kg_s > 0.0 {
            self.sortie_pour(theta_ext_c, self.qm_prec_kg_s)
        } else {
            SortiePuitsHydraulique {
                theta_an_c: theta_ext_c,
                theta_eau_out_c: self.theta_sol_c,
                en_marche: false,
            }
        };
    }

    /// Sortie du puits au débit d'air neuf `qm_kg_s` (éq. 735 à 749 pour un système).
    pub fn sortie_pour(&self, theta_ext_c: f64, qm_kg_s: f64) -> SortiePuitsHydraulique {
        let p = &self.description;
        let eps_air = if self.chauffage {
            self.eps_batt.0
        } else {
            self.eps_batt.1
        };
        // Éq. (739) et (740), un seul système : `εbatt = εbatt^S × Rdc`.
        let eps_batt = eps_air * rdc(p, qm_kg_s);
        let t_in = theta_eau_in_c(self.eps_sol_eau, eps_batt, self.theta_sol_c, theta_ext_c);
        let t_out = theta_eau_out_c(self.eps_sol_eau, t_in, self.theta_sol_c);
        let en_marche = pompe_en_marche(p, self.chauffage, theta_ext_c, t_out);
        SortiePuitsHydraulique {
            // Éq. (415), (416) et (417).
            theta_an_c: if en_marche {
                theta_ext_c + eps_air * (t_out - theta_ext_c)
            } else {
                theta_ext_c
            },
            theta_eau_out_c: t_out,
            en_marche,
        }
    }

    /// Sortie du pas, avec le débit de la fin du pas précédent.
    pub fn sortie(&self) -> SortiePuitsHydraulique {
        self.sortie
    }

    /// Aucun débit n'est connu : le premier pas se calcule avec son propre débit d'air neuf.
    pub fn sans_historique(&self) -> bool {
        self.qm_prec_kg_s <= 0.0
    }

    /// Retient le débit massique d'air neuf de la fin du pas.
    pub fn retenir_debit(&mut self, qm_kg_s: f64) {
        self.qm_prec_kg_s = qm_kg_s.max(0.0);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::entree::TypeSolPuits;

    fn puits() -> PuitsHydraulique {
        PuitsHydraulique {
            q_max_m3_h: 1.2,
            p_max_w: 60.0,
            l_m: 100.0,
            lambda_tube_w_m_k: 0.4,
            ep_m: 0.003,
            di_m: 0.032,
            profondeur_m: 1.5,
            type_sol: TypeSolPuits::SolHumide,
            gestion: GestionPuitsHydraulique::TemperatureExterieure,
            theta_ext_arret_hiver_c: 12.0,
            theta_ext_arret_ete_c: 22.0,
            batterie: None,
            surventilation_synchronisee: false,
        }
    }

    fn proche(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() < tol, "{a} différent de {b}");
    }

    /// Valeurs calculées à la main (`/home/claude/notes/ven9-calc.py` : mêmes équations,
    /// autre langage) : Re = 9 098,57 ; hi = 1 089,05 W/(m².K) ; Ug = 12,9014 W/(m.K) ;
    /// εsol-eau = 0,629222.
    #[test]
    fn echange_sol_eau_a_la_main() {
        let p = puits();
        proche(h_i_w_m2_k(&p), 1089.0488, 1e-3);
        proche(u_g_w_m_k(&p), 12.901416, 1e-5);
        proche(efficacite_sol_eau(&p), 0.629222, 1e-6);
    }

    /// `DCair` = 0,07 × 1 004 = 70,28 W/K ; `DCeau` = 1,2 × 1 040 × 3 751 / 3 600 = 1 300,3467 W/K ;
    /// Rdc = 0,0540471.
    #[test]
    fn rapport_des_debits_calorifiques() {
        let p = puits();
        proche(rdc(&p, 0.07), 0.0540471, 1e-6);
        // Air plus capacitif que l'eau : le ratio reste le plus petit sur le plus grand.
        proche(
            rdc(&p, 2.0),
            1.2 * 1040.0 * 3751.0 / 3600.0 / (2.0 * 1004.0),
            1e-12,
        );
        assert_eq!(rdc(&p, 0.0), 0.0);
    }

    /// Éq. (741) et (742) : εbatt = 0,65 × Rdc = 0,0351306, θsol = 10 °C, θext = −2 °C :
    /// θeau-in = 9,343606 °C, θeau-out = 9,756624 °C, θan = −2 + 0,65 × 11,756624 = 5,641805 °C.
    #[test]
    fn temperatures_de_l_eau_et_de_l_air_a_la_main() {
        let es = 0.629222093958;
        let eb = 0.65 * 0.0540471258946;
        let t_in = theta_eau_in_c(es, eb, 10.0, -2.0);
        proche(t_in, 9.343606, 1e-5);
        let t_out = theta_eau_out_c(es, t_in, 10.0);
        proche(t_out, 9.756624, 1e-5);
        proche(-2.0 + 0.65 * (t_out + 2.0), 5.641805, 1e-5);
        // Batterie nulle : l'eau entre dans le puits à la température du sol.
        proche(theta_eau_in_c(es, 0.0, 10.0, -2.0), 10.0, 1e-12);
    }

    /// Éq. (746) et (747), les deux gestions.
    #[test]
    fn gestion_de_la_pompe() {
        let mut p = puits();
        // Chauffage : arrêt au-dessus de 12 °C.
        assert!(pompe_en_marche(&p, true, 5.0, 0.0));
        assert!(!pompe_en_marche(&p, true, 12.0, 0.0));
        // Hors chauffage : arrêt en dessous de 22 °C.
        assert!(pompe_en_marche(&p, false, 30.0, 40.0));
        assert!(!pompe_en_marche(&p, false, 22.0, 0.0));
        // Gestion sur les deux températures : la pompe ne tourne que si l'eau est plus chaude
        // (hiver) ou plus fraîche (été) que l'air extérieur.
        p.gestion = GestionPuitsHydraulique::TemperaturesExterieureEtEau;
        assert!(pompe_en_marche(&p, true, 5.0, 9.0));
        assert!(!pompe_en_marche(&p, true, 5.0, 4.0));
        assert!(pompe_en_marche(&p, false, 30.0, 15.0));
        assert!(!pompe_en_marche(&p, false, 30.0, 35.0));
    }

    /// Éq. (401) à (407) : Pch = 1,2 kW à 0,07 kg/s : θout = −10 + 1 200/(1 006 × 0,07) =
    /// 7,0407 °C, ε = 0,852031 ; Pfr = 1,2 kW : θout = 30 − 17,0407 = 12,9593 °C, ε = 0,852031.
    #[test]
    fn efficacite_de_la_batterie_a_la_main() {
        proche(efficacite_batterie_ch(1.2, 0.07).unwrap(), 0.852031, 1e-6);
        proche(efficacite_batterie_fr(1.2, 0.07).unwrap(), 0.852031, 1e-6);
        // Puissance supérieure à la puissance maximale (ε > 1) : erreur.
        assert!(efficacite_batterie_ch(1.5, 0.07).is_err());
        assert!(efficacite_batterie_fr(1.5, 0.07).is_err());
        assert!(efficacite_batterie_ch(1.2, 0.0).is_err());
    }
}
