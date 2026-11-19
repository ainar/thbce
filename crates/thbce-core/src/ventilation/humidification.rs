// SPDX-License-Identifier: GPL-3.0-or-later
//! Humidification d'hiver et rafraîchissement évaporatif des centrales double flux et de
//! traitement d'air : fiche `C_VEN_CTA et Double_flux` (Th-BCE 2020 §6.3), fonctions pures.
//!
//! - activation du rafraîchissement adiabatique (§6.3.3.4.2, « Aut_adiab ») ;
//! - profil à hystérésis des débits des centrales à débit variable (éq. 426 à 428 et 449 à
//!   450, figure 73) et puissance des ventilateurs (éq. 466 et 467) ;
//! - procédure FADIAB (éq. 476 et 477) ;
//! - rafraîchissement évaporatif indirect sur l'air extrait (éq. 478 à 486) et direct sur
//!   l'air soufflé (éq. 549 à 559) ;
//! - humidificateur d'hiver à vapeur ou à pulvérisation (éq. 526 à 537) ;
//! - consommation des pompes (éq. 566 à 569, 615 à 617).
//!
//! Lectures : `docs/questions/Q096.md` et `docs/questions/Q097.md`.

use super::cta_et_double_flux::{omega_sat_kg_kg, A_SAT, B_SAT, C_SAT, L_V_J_KG};
use super::mecanique_sf::C_PA_J_KG_K;

/// Valeur minimale de « θi_base », en °C (§6.3.3.4.2 : « fixée conventionnellement à 22 °C »).
pub const THETA_I_BASE_MIN_C: f64 = 22.0;

/// Efficacité conventionnelle des humidificateurs à pulvérisation et des humidificateurs
/// d'été (« ηhum », « ηhum_indir », « ηhum_dir » : 0,8, tableau 61 et éq. 530).
pub const ETA_HUM_CONV: f64 = 0.8;

/// Seuil d'humidité relative des groupes au-delà duquel le rafraîchissement direct est
/// suspendu (éq. 549 et 550 : 70 %).
pub const HR_MAX_DIRECT: f64 = 0.7;

/// Nombre maximal d'itérations de la procédure FADIAB (garde-fou, le texte n'en fixe pas) :
/// 1 000 pas de 0,1 g/kg couvrent 100 g/kg.
const FADIAB_ITERATIONS_MAX: usize = 1000;

/// « Aut_adiab » : autorisation du rafraîchissement adiabatique (§6.3.3.4.2) :
/// `θi,moy > θi_base` et `Autfr(j) = 1` et `irafinocc_activé(h) = 0`.
pub fn aut_adiab(theta_i_moy_c: f64, theta_i_base_c: f64, aut_fr: bool, rafinocc: bool) -> bool {
    theta_i_moy_c > theta_i_base_c && aut_fr && !rafinocc
}

/// Températures de consigne du profil à hystérésis, en °C (éq. 426) :
/// `[θhum_1, θhum_2, θhum_3, θhum_4]`.
pub fn consignes_hysteresis_c(theta_i_base_c: f64, d1_k: f64, d2_k: f64) -> [f64; 4] {
    [
        theta_i_base_c + d1_k,
        theta_i_base_c + d1_k + d2_k,
        theta_i_base_c + d2_k,
        theta_i_base_c,
    ]
}

/// Position du débit d'une bouche dans le profil à hystérésis (éq. 427 et 449), entre 0
/// (débit de zone neutre) et 1 (débit du mode actif, indice CH).
///
/// `theta_c` : `θi,fin(h−1)` du groupe de la bouche ; `theta_prec_c` : la même au pas
/// précédent, qui dit si la température monte ou descend ; `x_prec` : position au pas
/// précédent. La position remplace le débit massique `qm,sou,cond(h−1)` du texte (lecture Q96
/// point 4).
pub fn position_hysteresis(
    theta_c: f64,
    theta_prec_c: f64,
    x_prec: f64,
    consignes: [f64; 4],
) -> f64 {
    let [h1, h2, h3, h4] = consignes;
    let rampe = |t0: f64, t1: f64| {
        if t1 > t0 {
            ((theta_c - t0) / (t1 - t0)).clamp(0.0, 1.0)
        } else {
            1.0
        }
    };
    if theta_c <= h4 {
        0.0
    } else if theta_c >= h2 {
        1.0
    } else if theta_prec_c <= theta_c {
        // Augmentation de la température.
        if theta_c <= h1 {
            x_prec
        } else {
            x_prec.max(rampe(h1, h2))
        }
    } else if theta_c >= h3 {
        // Diminution de la température.
        x_prec
    } else {
        x_prec.min(rampe(h4, h3))
    }
}

/// Puissance d'un ventilateur d'une centrale à débit variable en rafraîchissement
/// adiabatique, en W (éq. 466) : `Pzn + (Pch − Pzn)·(0,6315·R³ + 0,1966·R² + 0,1719·R)`, avec
/// `R` le rapport des écarts aux débits de zone neutre (éq. 467).
pub fn p_vent_adiab_w(p_zn_occ_w: f64, p_ch_w: f64, r: f64) -> f64 {
    p_zn_occ_w + (p_ch_w - p_zn_occ_w) * (0.6315 * r * r * r + 0.1966 * r * r + 0.1719 * r)
}

/// « FADIAB(θ, ω) » : augmentation d'humidité spécifique maximale théorique d'une
/// transformation adiabatique depuis `(θ, ω)`, en kg/kg (éq. 476 et 477).
///
/// La procédure du texte travaille en g/kg d'air sec (pas de 0,1 g/kg, `1000 × ω` dans la
/// valeur initiale, constantes de l'éq. 477 qui donnent `ωsat` en g/kg) et `log` est le
/// logarithme népérien (celui de l'éq. 541) ; le résultat, en g/kg, est converti en kg/kg
/// pour les éq. (478), (530) et (550) (lecture Q96 point 1). Sans itération (`Ω ≤ 0` dès le
/// départ), la valeur initiale de `Δω` est rendue.
pub fn fadiab_kg_kg(theta_c: f64, omega_kg_kg: f64) -> f64 {
    let w = 1000.0 * omega_kg_kg;
    let omega_f = |dw: f64| {
        let x = w + dw;
        if x > 0.0 {
            (theta_c + C_SAT - 2.5 * dw) * (A_SAT - libm::log(x)) - B_SAT
        } else {
            f64::INFINITY
        }
    };
    let mut dw = (0.2545 * theta_c - 0.3636 * w).max(0.0);
    let mut om = omega_f(dw);
    if om <= 0.0 {
        return dw / 1000.0;
    }
    let mut prev = om;
    let mut n = 0;
    while om > 0.0 && n < FADIAB_ITERATIONS_MAX {
        dw += 0.1;
        prev = om;
        om = omega_f(dw);
        n += 1;
    }
    let d = om - prev;
    let fadiab = if d.is_finite() && d != 0.0 {
        dw - 0.1 * om / d
    } else {
        dw
    };
    fadiab.max(0.0) / 1000.0
}

/// Air en sortie d'un humidificateur adiabatique (rafraîchissement évaporatif indirect sur
/// l'air extrait ou direct sur l'air soufflé, éq. 478 à 485 et 550 à 558).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AirHumidifie {
    /// Température en sortie, en °C.
    pub theta_c: f64,
    /// Humidité spécifique en sortie, en kg/kg.
    pub omega_kg_kg: f64,
}

/// Humidificateur adiabatique de rendement `eta` sur un air `(θ, ω)` (éq. 478 à 483 pour
/// l'indirect, 550 à 556 pour le direct) :
/// `ωhum = ω + η·FADIAB(θ, ω)`, borné par `ωhum,lim = η·ωsat(θ)` ; si `ωhum,lim < ω`,
/// l'air n'est pas modifié (lecture Q96 point 2) ; `θ' = θ − Lv/Cpa·(ωhum − ω)`.
pub fn humidificateur_adiabatique(theta_c: f64, omega_kg_kg: f64, eta: f64) -> AirHumidifie {
    let lim = eta * omega_sat_kg_kg(theta_c);
    let omega_hum = if lim < omega_kg_kg {
        omega_kg_kg
    } else {
        (omega_kg_kg + eta * fadiab_kg_kg(theta_c, omega_kg_kg)).min(lim)
    };
    AirHumidifie {
        theta_c: theta_c - L_V_J_KG / C_PA_J_KG_K * (omega_hum - omega_kg_kg),
        omega_kg_kg: omega_hum,
    }
}

/// Condition du rafraîchissement direct (éq. 549 et 550) : chaque groupe relié a une
/// humidité `ωi,fin(h−1)` inférieure à 70 % de l'humidité de saturation à `θi,fin(h−1)`.
pub fn groupe_assez_sec(theta_i_c: f64, omega_i_kg_kg: f64) -> bool {
    omega_i_kg_kg < HR_MAX_DIRECT * omega_sat_kg_kg(theta_i_c)
}

/// Type d'humidificateur d'hiver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TypeHumidificateur {
    /// À vapeur : transformation isotherme (éq. 526 à 529).
    Vapeur,
    /// À pulvérisation, ruissellement ou évaporation, d'efficacité `ηhum` : transformation
    /// adiabatique (éq. 530 à 534).
    Pulverisation {
        /// « ηhum ».
        eta: f64,
    },
}

/// Sorties de l'humidificateur d'hiver au pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HumidificationHiver {
    /// « θsouf,3(h) », en °C.
    pub theta_souf3_c: f64,
    /// « ωsouf,3(h) », en kg/kg.
    pub omega_souf3_kg_kg: f64,
    /// « QHR(h) » : énergie à fournir, en Wh (éq. 529, 533, 537).
    pub q_hr_wh: f64,
    /// « qm,eau_hiver^pompe(h) » : débit d'eau fourni à l'air, en kg/s (éq. 534).
    pub qm_eau_kg_s: f64,
}

/// Humidificateur d'hiver (§6.3.3.4.9.2, éq. 526 à 537), en saison de chauffage seulement :
/// `θpre` et `ωsouf2`, air à l'entrée (après la batterie de préchauffage) ; `ωcons`, consigne ;
/// `qm`, débit soufflé par la centrale en kg/s.
pub fn humidification_hiver(
    type_hum: TypeHumidificateur,
    saison_de_chauffage: bool,
    theta_pre_c: f64,
    omega_souf2_kg_kg: f64,
    omega_cons_kg_kg: f64,
    qm_sou_cond_kg_s: f64,
) -> HumidificationHiver {
    let sans = HumidificationHiver {
        theta_souf3_c: theta_pre_c,
        omega_souf3_kg_kg: omega_souf2_kg_kg,
        q_hr_wh: 0.0,
        qm_eau_kg_s: 0.0,
    };
    if !saison_de_chauffage {
        // Éq. (535) à (537).
        return sans;
    }
    let consigne = omega_souf2_kg_kg.max(omega_cons_kg_kg);
    match type_hum {
        TypeHumidificateur::Vapeur => {
            // Éq. (526) à (529).
            let lim = omega_sat_kg_kg(theta_pre_c);
            let omega3 = lim.min(consigne);
            HumidificationHiver {
                omega_souf3_kg_kg: omega3,
                q_hr_wh: qm_sou_cond_kg_s * L_V_J_KG * (omega3 - omega_souf2_kg_kg).max(0.0),
                ..sans
            }
        }
        TypeHumidificateur::Pulverisation { eta } => {
            // Éq. (530) à (534).
            let lim = omega_souf2_kg_kg + eta * fadiab_kg_kg(theta_pre_c, omega_souf2_kg_kg);
            let omega3 = lim.min(consigne);
            HumidificationHiver {
                theta_souf3_c: theta_pre_c - L_V_J_KG / C_PA_J_KG_K * (omega3 - omega_souf2_kg_kg),
                omega_souf3_kg_kg: omega3,
                q_hr_wh: 0.0,
                qm_eau_kg_s: qm_sou_cond_kg_s * (omega3 - omega_souf2_kg_kg).max(0.0),
            }
        }
    }
}

/// Consommation d'une pompe d'humidificateur, en Wh (éq. 566, 568 et 615), pour un débit
/// d'eau `qm,eau^pompe` en kg/s, unité de son équation de définition (éq. 486, 534 et 559 ;
/// lecture Q97) ; nulle sans débit.
pub fn w_aux_pompe_wh(qm_eau_kg_s: f64) -> f64 {
    let q = qm_eau_kg_s;
    if q <= 0.0 {
        return 0.0;
    }
    let num = (-0.0057 * q * q - 0.7842 * q + 18778.0) * q / 3600.0;
    let den = -3e-7 * q * q + 0.0004 * q + 0.0031;
    if den > 0.0 {
        num / den
    } else {
        0.0
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn proche(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} ≠ {b}");
    }

    #[test]
    fn fadiab_a_la_main() {
        // θ = 30 °C, ω = 10 g/kg : Δω0 = 0,2545 × 30 − 3,636 = 3,999 g/kg ;
        // Ω(Δω) = (265 − 2,5·Δω)·(18,8161 − ln(10 + Δω)) − 4110,34.
        let f = |dw: f64| (265.0 - 2.5 * dw) * (18.8161 - libm::log(10.0 + dw)) - 4110.34;
        // Δω de 3,999 en 3,999 + 0,1·k jusqu'au premier Ω ≤ 0, puis interpolation linéaire.
        let mut dw = 0.2545 * 30.0 - 0.3636 * 10.0;
        let mut prev = f(dw);
        assert!(prev > 0.0);
        let mut om = prev;
        while om > 0.0 {
            dw += 0.1;
            prev = om;
            om = f(dw);
        }
        let attendu = (dw - 0.1 * om / (om - prev)) / 1000.0;
        proche(fadiab_kg_kg(30.0, 0.010), attendu, 1e-12);
        // Le point d'arrivée est sur la courbe de saturation : ωsat(θ − 2,5·Δω) = ω + Δω, à la
        // tolérance de l'interpolation (Δω ≈ 4,25 g/kg, θ humide ≈ 19,4 °C, ωsat ≈ 14,3 g/kg).
        let d = fadiab_kg_kg(30.0, 0.010);
        proche(omega_sat_kg_kg(30.0 - 2500.0 * d), 0.010 + d, 2e-4);
        assert!(d > 0.0042 && d < 0.0043, "{d}");
        // Air saturé : rien à ajouter.
        let sat = omega_sat_kg_kg(20.0);
        assert!(fadiab_kg_kg(20.0, sat) < 1e-4);
    }

    #[test]
    fn humidificateurs_a_la_main() {
        // Direct ou indirect, η = 0,8 sur 30 °C, 10 g/kg : ωhum = 10 + 0,8·FADIAB, borné par
        // 0,8 × ωsat(30 °C) = 0,8 × 27,2 = 21,8 g/kg (non atteint).
        let d = fadiab_kg_kg(30.0, 0.010);
        let a = humidificateur_adiabatique(30.0, 0.010, 0.8);
        proche(a.omega_kg_kg, 0.010 + 0.8 * d, 1e-15);
        proche(a.theta_c, 30.0 - 2_501_000.0 / 1006.0 * 0.8 * d, 1e-12);
        // Air humide (lim = 0,8·ωsat < ω) : inchangé.
        let w = 0.9 * omega_sat_kg_kg(25.0);
        let a = humidificateur_adiabatique(25.0, w, 0.8);
        assert_eq!((a.theta_c, a.omega_kg_kg), (25.0, w));
        // Hiver, vapeur : 1 kg/s de 2 à 5 g/kg sous 20 °C (ωsat = 14,7 g/kg) :
        // QHR = 1 × 2 501 000 × 0,003 = 7 503 Wh, température inchangée.
        let h = humidification_hiver(TypeHumidificateur::Vapeur, true, 20.0, 0.002, 0.005, 1.0);
        proche(h.q_hr_wh, 7503.0, 1e-6);
        proche(h.omega_souf3_kg_kg, 0.005, 1e-15);
        assert_eq!(h.theta_souf3_c, 20.0);
        // Pulvérisation : énergie nulle, refroidissement de 2 501/1,006 × 3 g/kg = 7,46 K si la
        // limite adiabatique le permet (FADIAB(20 °C, 2 g/kg) ≈ 5,6 g/kg).
        let h = humidification_hiver(
            TypeHumidificateur::Pulverisation { eta: 0.8 },
            true,
            20.0,
            0.002,
            0.005,
            1.0,
        );
        assert_eq!(h.q_hr_wh, 0.0);
        proche(h.theta_souf3_c, 20.0 - 2_501_000.0 / 1006.0 * 0.003, 1e-9);
        proche(h.qm_eau_kg_s, 0.003, 1e-15);
        // Hors saison de chauffage : rien.
        let h = humidification_hiver(TypeHumidificateur::Vapeur, false, 20.0, 0.002, 0.005, 1.0);
        assert_eq!((h.q_hr_wh, h.omega_souf3_kg_kg), (0.0, 0.002));
    }

    #[test]
    fn pompe_et_ventilateurs_a_la_main() {
        // q = 0,003 kg/s : (−0,0057·9e-6 − 0,7842·0,003 + 18 778) × 0,003/3 600
        // / (−3e-7·9e-6 + 0,0004·0,003 + 0,0031).
        let q: f64 = 0.003;
        let num = (-0.0057 * q * q - 0.7842 * q + 18778.0) * q / 3600.0;
        let den = -3e-7 * q * q + 0.0004 * q + 0.0031;
        proche(w_aux_pompe_wh(q), num / den, 1e-12);
        proche(w_aux_pompe_wh(q), 5.0459, 1e-3);
        assert_eq!(w_aux_pompe_wh(0.0), 0.0);
        // Éq. (466) : R = 0,5 → 0,6315/8 + 0,1966/4 + 0,1719/2 = 0,214 037 5.
        proche(
            p_vent_adiab_w(500.0, 1500.0, 0.5),
            500.0 + 1000.0 * 0.214_037_5,
            1e-9,
        );
        proche(p_vent_adiab_w(500.0, 1500.0, 1.0), 1500.0, 1e-9);
    }

    #[test]
    fn hysteresis_a_la_main() {
        // θbase 24 °C, Δθ1 = 2 K, Δθ2 = 1 K : θhum = [26, 27, 25, 24].
        let c = consignes_hysteresis_c(24.0, 2.0, 1.0);
        assert_eq!(c, [26.0, 27.0, 25.0, 24.0]);
        assert_eq!(position_hysteresis(23.0, 22.0, 0.7, c), 0.0);
        assert_eq!(position_hysteresis(27.5, 22.0, 0.0, c), 1.0);
        // Montée : sous θhum_1, débit gardé ; entre θhum_1 et θhum_2, rampe ou débit gardé.
        assert_eq!(position_hysteresis(25.5, 25.0, 0.3, c), 0.3);
        proche(position_hysteresis(26.5, 26.0, 0.2, c), 0.5, 1e-12);
        proche(position_hysteresis(26.5, 26.0, 0.8, c), 0.8, 1e-12);
        // Descente : au-dessus de θhum_3, débit gardé ; entre θhum_4 et θhum_3, rampe ou gardé.
        assert_eq!(position_hysteresis(25.5, 26.0, 0.9, c), 0.9);
        proche(position_hysteresis(24.25, 24.5, 0.9, c), 0.25, 1e-12);
        proche(position_hysteresis(24.25, 24.5, 0.1, c), 0.1, 1e-12);
        // Autorisation.
        assert!(aut_adiab(25.0, 24.0, true, false));
        assert!(!aut_adiab(24.0, 24.0, true, false));
        assert!(!aut_adiab(25.0, 24.0, false, false));
    }
}
