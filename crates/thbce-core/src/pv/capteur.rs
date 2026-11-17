// SPDX-License-Identifier: GPL-3.0-or-later
//! Fiche `C_PV_capteur_PV` (Th-BCE 2020 §12.3) : ensemble des modules connectés à un même
//! onduleur.
//!
//! Calcule, au pas horaire, la puissance MPPT `PPV` des modules à partir du rayonnement
//! incident efficace sur leur plan et de leur température. Aucune fonction du module n'alloue :
//! elles sont appelées dans la boucle horaire.

use libm::{cos, log};

use super::entree::{ConfinementPv, PerformancesPv, TechnologiePv};
use crate::resultats::ValeurParDefaut;

/// « τα » : coefficient de transmission-absorption solaire des modules (tableau 327).
pub const TAU_ALPHA: f64 = 0.9;
/// « Fm » : coefficient de perte par connectique et mismatch (tableau 327).
pub const FM: f64 = 0.97;
/// « U2 » : coefficient d'échange entre le capteur et la voûte céleste, en W/(m².K) (éq. 2368).
pub const U2_W_M2K: f64 = 5.0;
/// « ΔTc,r » : écart de température de référence entre l'air ambiant et la voûte céleste, en °C
/// (tableau 327, valeur conventionnelle −13 °C).
pub const DELTA_T_CR_C: f64 = -13.0;
/// « NOCT_util_min » : valeur minimale de la température d'équilibre thermique, en °C
/// (§12.3.3.1).
pub const NOCT_UTIL_MIN_C: f64 = 40.0;
/// Borne basse de `G` dans le logarithme de l'éq. (2372), en W/m² (« 110⁻⁴ », lu 1·10⁻⁴ :
/// décision Q51 point 2).
pub const G_MIN_LOG_W_M2: f64 = 1e-4;
/// Angle d'incidence au-delà duquel `Fopt` n'évolue plus, en degrés (éq. de `Fopt`).
pub const THETA_MAX_FOPT_DEG: f64 = 87.0;

/// « μ_util_min » : valeur minimale du coefficient de température selon la technologie, en °C⁻¹
/// (§12.3.3.1).
pub fn mu_util_min(techno: TechnologiePv) -> f64 {
    match techno {
        TechnologiePv::MonoCristallinSi => 0.004_25,
        TechnologiePv::MultiCristallinSi | TechnologiePv::Autre => 0.004_33,
        TechnologiePv::CdTe => 0.002_08,
        TechnologiePv::Cis => 0.003_25,
        TechnologiePv::AmorpheSi => 0.001_75,
    }
}

/// « Γ » : correctif basse lumière du rendement, valeur par défaut du tableau 329 (la colonne
/// « source » est « ND »).
pub fn gamma_basse_lumiere(techno: TechnologiePv) -> f64 {
    match techno {
        TechnologiePv::CdTe | TechnologiePv::AmorpheSi => 0.0,
        TechnologiePv::MonoCristallinSi
        | TechnologiePv::MultiCristallinSi
        | TechnologiePv::Cis
        | TechnologiePv::Autre => 0.07,
    }
}

/// Valeurs de calcul de `Pc`, `μ` et `NOCT` (§12.3.3.1).
#[derive(Debug, Clone, PartialEq)]
pub struct PerformancesCalcul {
    /// « Pc » de calcul d'un module, en W.
    pub pc_w: f64,
    /// « μ » de calcul, en °C⁻¹.
    pub mu_par_c: f64,
    /// « NOCT » de calcul, en °C.
    pub noct_c: f64,
    /// Valeurs par défaut du texte appliquées (statut « défaut »).
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// Valeurs de calcul selon l'origine des performances.
///
/// Th-BCE 2020 §12.3.3.1 :
/// - certifiées : valeurs saisies ;
/// - justifiées : `0,9 × Pc`, `1,10 × μ`, `1,10 × NOCT` ;
/// - déclarées : `0,8 × Pc`, `max(1,20 × μ ; μ_util_min)`, `max(1,20 × NOCT ; NOCT_util_min)` ;
/// - par défaut : `0,8 × Pc`, `1,20 × μ_util_min`, `1,20 × NOCT_util_min`.
pub fn performances_de_calcul(
    techno: TechnologiePv,
    perf: &PerformancesPv,
) -> PerformancesCalcul {
    let mu_min = mu_util_min(techno);
    let regle = |valeur: &str| {
        format!("Th-BCE 2020 §12.3.3.1, valeur par défaut : {valeur}")
    };
    match *perf {
        PerformancesPv::Certifie {
            pc_w,
            mu_par_c,
            noct_c,
        } => PerformancesCalcul {
            pc_w,
            mu_par_c,
            noct_c,
            valeurs_par_defaut: Vec::new(),
        },
        PerformancesPv::Justifie {
            pc_w,
            mu_par_c,
            noct_c,
        } => PerformancesCalcul {
            pc_w: 0.9 * pc_w,
            mu_par_c: 1.10 * mu_par_c,
            noct_c: 1.10 * noct_c,
            valeurs_par_defaut: Vec::new(),
        },
        PerformancesPv::Declare {
            pc_w,
            mu_par_c,
            noct_c,
        } => PerformancesCalcul {
            pc_w: 0.8 * pc_w,
            mu_par_c: (1.20 * mu_par_c).max(mu_min),
            noct_c: (1.20 * noct_c).max(NOCT_UTIL_MIN_C),
            valeurs_par_defaut: Vec::new(),
        },
        PerformancesPv::Defaut { pc_w } => PerformancesCalcul {
            pc_w: 0.8 * pc_w,
            mu_par_c: 1.20 * mu_min,
            noct_c: 1.20 * NOCT_UTIL_MIN_C,
            valeurs_par_defaut: vec![
                ValeurParDefaut {
                    champ: "modules.performances.mu_par_c".to_owned(),
                    valeur: 1.20 * mu_min,
                    regle: regle("1,20 × μ_util_min de la technologie"),
                },
                ValeurParDefaut {
                    champ: "modules.performances.noct_c".to_owned(),
                    valeur: 1.20 * NOCT_UTIL_MIN_C,
                    regle: regle("1,20 × NOCT_util_min, soit 48 °C"),
                },
            ],
        },
    }
}

/// `Fr = (1 + cos 30°) / 2`.
///
/// Th-BCE 2020 §12.3.3.1, éq. (2369).
pub fn fr() -> f64 {
    (1.0 + cos(30.0_f64.to_radians())) / 2.0
}

/// `F = (1 + cos β) / 2`.
///
/// Th-BCE 2020 §12.3.3.1, éq. (2370).
pub fn f_voute(beta_rad: f64) -> f64 {
    (1.0 + cos(beta_rad)) / 2.0
}

/// `ηSTC = Pc / (S × 1000)`.
///
/// Th-BCE 2020 §12.3.3.1, éq. (2366). `Pc` est la valeur de calcul (question Q51 point 1).
pub fn eta_stc(pc_w: f64, s_m2: f64) -> f64 {
    pc_w / (s_m2 * 1000.0)
}

/// « U1 » : coefficient d'échange entre le capteur et l'ambiance, en W/(m².K).
///
/// Th-BCE 2020 §12.3.3.1, éq. (2367) :
/// `U1 = (1/CT) · [τα·800/(NOCT − 20) − Fr·U2·(NOCT − ΔTc,r − 20)/(NOCT − 20)]`.
pub fn u1_w_m2k(noct_c: f64, ct: f64) -> f64 {
    let ecart = noct_c - 20.0;
    (TAU_ALPHA * 800.0 / ecart - fr() * U2_W_M2K * (noct_c - DELTA_T_CR_C - 20.0) / ecart) / ct
}

/// « Fopt(x) » : coefficient de perte optique par réflexion, `x` en degrés.
///
/// Th-BCE 2020 §12.3.3.2 : `Fopt(x) = 1 − 0,05 · (1 / cos(min(87 ; x)·π/180) − 1)`.
pub fn fopt(x_deg: f64) -> f64 {
    let x = x_deg.min(THETA_MAX_FOPT_DEG);
    1.0 - 0.05 * (1.0 / cos(x.to_radians()) - 1.0)
}

/// « θ1 » : angle moyen du rayonnement diffus de ciel par rapport à la normale aux modules,
/// en degrés, `β` en degrés.
///
/// Th-BCE 2020 §12.3.3.2 : `θ1 = 59,7 − 0,13888·β + 0,001497·β²`.
pub fn theta1_deg(beta_deg: f64) -> f64 {
    59.7 - 0.13888 * beta_deg + 0.001_497 * beta_deg * beta_deg
}

/// « θ2 » : angle moyen du rayonnement diffus d'albédo par rapport à la normale aux modules,
/// en degrés, `β` en degrés.
///
/// Th-BCE 2020 §12.3.3.2 : `θ2 = 90 − 0,5788·β + 0,002693·β²`.
pub fn theta2_deg(beta_deg: f64) -> f64 {
    90.0 - 0.5788 * beta_deg + 0.002_693 * beta_deg * beta_deg
}

/// « G » : rayonnement incident efficace dans le plan des modules, en W/m².
///
/// Th-BCE 2020 §12.3.3.2 : `G = Fopt(θ)·Drp* + Fopt(θ1)·Dfp* + Fopt(θ2)·Rrp*`.
pub fn g_efficace_w_m2(
    theta_deg: f64,
    beta_deg: f64,
    drp_w_m2: f64,
    dfp_w_m2: f64,
    rrp_w_m2: f64,
) -> f64 {
    fopt(theta_deg) * drp_w_m2
        + fopt(theta1_deg(beta_deg)) * dfp_w_m2
        + fopt(theta2_deg(beta_deg)) * rrp_w_m2
}

/// Champ de modules prêt pour le calcul horaire.
#[derive(Debug, Clone, PartialEq)]
pub struct CapteurPv {
    /// « N » : nombre de modules.
    pub n: f64,
    /// Valeurs de calcul des performances.
    pub performances: PerformancesCalcul,
    /// « Γ » : correctif basse lumière.
    pub gamma_bl: f64,
    /// « ηSTC ».
    pub eta_stc: f64,
    /// « β » : inclinaison, en degrés.
    pub beta_deg: f64,
    /// « U1 », en W/(m².K).
    pub u1_w_m2k: f64,
    /// « F · U2 », en W/(m².K).
    pub f_u2_w_m2k: f64,
}

/// Sorties d'un pas de temps du capteur.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SortiesPasCapteur {
    /// « G » : rayonnement incident efficace, en W/m².
    pub g_w_m2: f64,
    /// « Tm » : température des modules, en °C.
    pub tm_c: f64,
    /// « Ppv » : puissance MPPT du champ, en W.
    pub p_pv_w: f64,
}

impl CapteurPv {
    /// Prépare un champ de modules.
    ///
    /// `beta_deg` est l'inclinaison des modules, en degrés ; `s_m2` la surface d'un module.
    pub fn nouveau(
        n: f64,
        techno: TechnologiePv,
        perf: &PerformancesPv,
        s_m2: f64,
        ct: ConfinementPv,
        beta_deg: f64,
    ) -> Self {
        let performances = performances_de_calcul(techno, perf);
        Self {
            n,
            gamma_bl: gamma_basse_lumiere(techno),
            eta_stc: eta_stc(performances.pc_w, s_m2),
            beta_deg,
            u1_w_m2k: u1_w_m2k(performances.noct_c, ct.ct()),
            f_u2_w_m2k: f_voute(beta_deg.to_radians()) * U2_W_M2K,
            performances,
        }
    }

    /// « Tm » : température des modules, en °C.
    ///
    /// Th-BCE 2020 §12.3.3.2, éq. (2373) :
    /// `Tm = U1·Text/(U1 + F·U2) + F·U2·Tciel/(U1 + F·U2) + G·(τα − ηSTC)/(U1 + F·U2)`.
    pub fn temperature_module_c(&self, g_w_m2: f64, text_c: f64, tciel_c: f64) -> f64 {
        let somme = self.u1_w_m2k + self.f_u2_w_m2k;
        (self.u1_w_m2k * text_c + self.f_u2_w_m2k * tciel_c + g_w_m2 * (TAU_ALPHA - self.eta_stc))
            / somme
    }

    /// « Pmpp(G, Tm) » : puissance d'un module au point de puissance maximale, en W.
    ///
    /// Th-BCE 2020 §12.3.3.2, éq. (2372) :
    /// `Pmpp = Pc · G/1000 · max(0 ; 1 + Γ·ln(max(10⁻⁴ ; G)/1000)) · (1 − μ·(Tm − 25))`.
    /// Le terme « 0,1 » du texte est lu « 0 ; 1 » : décision Q51 point 2.
    pub fn p_mpp_module_w(&self, g_w_m2: f64, tm_c: f64) -> f64 {
        let basse_lumiere =
            (1.0 + self.gamma_bl * log(g_w_m2.max(G_MIN_LOG_W_M2) / 1000.0)).max(0.0);
        self.performances.pc_w
            * (g_w_m2 / 1000.0)
            * basse_lumiere
            * (1.0 - self.performances.mu_par_c * (tm_c - 25.0))
    }

    /// Un pas de temps : puissance MPPT du champ.
    ///
    /// Th-BCE 2020 §12.3.3.2, éq. (2371) : `Ppv = Pmpp(G, Tm) · N · Fm`. En l'absence de
    /// rayonnement incident (`G ≤ 0`), la puissance est nulle.
    pub fn pas(
        &self,
        theta_deg: f64,
        drp_w_m2: f64,
        dfp_w_m2: f64,
        rrp_w_m2: f64,
        text_c: f64,
        tciel_c: f64,
    ) -> SortiesPasCapteur {
        let g_w_m2 = g_efficace_w_m2(theta_deg, self.beta_deg, drp_w_m2, dfp_w_m2, rrp_w_m2);
        if g_w_m2 <= 0.0 {
            return SortiesPasCapteur {
                g_w_m2: 0.0,
                tm_c: text_c,
                p_pv_w: 0.0,
            };
        }
        let tm_c = self.temperature_module_c(g_w_m2, text_c, tciel_c);
        let p_pv_w = (self.p_mpp_module_w(g_w_m2, tm_c) * self.n * FM).max(0.0);
        SortiesPasCapteur {
            g_w_m2,
            tm_c,
            p_pv_w,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn u1_noct_45_confine() {
        // τα·800/25 = 28,8 ; Fr·U2·(45 + 13 − 20)/25 ; Fr = 0,9330127.
        let attendu = (28.8 - fr() * 5.0 * 38.0 / 25.0) / 2.0;
        assert!((u1_w_m2k(45.0, 2.0) - attendu).abs() < 1e-12);
        assert!((u1_w_m2k(45.0, 2.0) - 10.855).abs() < 1e-3);
    }
}
