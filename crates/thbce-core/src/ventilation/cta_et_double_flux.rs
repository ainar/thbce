// SPDX-License-Identifier: GPL-3.0-or-later
//! Ventilation mécanique double flux : fiche `C_VEN_CTA et Double_flux` (Th-BCE 2020 §6.3),
//! pour le seul système compatible avec l'usage résidentiel, le groupe de ventilation double
//! flux (`Type_ventilation_mecaniq = 1`, §6.3.3.1.3), en mode « zone neutre » (§6.3.3.4).
//!
//! Périmètre :
//! - préprocesseur : puissance moyenne des ventilateurs en habitation (éq. 390 à 394),
//!   coefficients de transfert des conduits (éq. 395), statut de l'efficacité (éq. 396 à 398) ;
//! - au pas horaire : débits massiques soufflés et repris (éq. 420, 429 à 432, 443, 451 à 453,
//!   456), température de l'air repris à l'entrée de la centrale (éq. 469 à 473), ventilateur
//!   d'extraction (éq. 474 et 475), bypass (éq. 492, 494 et 495), efficacité de l'échangeur
//!   (éq. 496 à 505), sécurité antigel (éq. 506 à 511), sorties de l'échangeur (éq. 512 à
//!   514), ventilateur de soufflage (éq. 517 à 520), pertes des conduits de soufflage
//!   (éq. 560 à 564), consommation (éq. 565 et 570).
//!
//! Hors de la première version, refusés par la validation : centrales de traitement d'air
//! (types 2 à 4), échangeur rotatif (`Pech`), batterie de préchauffage, humidification,
//! rafraîchissement évaporatif, surventilation mécanique, puits climatique ou hydraulique.
//! L'air neuf est donc l'air extérieur (éq. 413) et, sans boîte de mélange ni batterie, l'air
//! sort de la centrale à la température de sortie du ventilateur de soufflage (éq. 517, 518,
//! 535, 546 et 560).

use crate::entree::{Bypass, EcoulementEchangeur, StatutEchangeur};

use super::mecanique_sf::C_PA_J_KG_K;

/// « εutile_max » : efficacité maximale de l'échangeur en l'absence de valeur certifiée ou
/// justifiée (tableau 61).
pub const EPSILON_UTILE_MAX: f64 = 0.5;
/// « θs,ech,rep,LIM » par défaut, en °C : « autres cas » du tableau 62.
pub const THETA_S_ECH_REP_LIM_DEFAUT_C: f64 = 5.0;

/// « εt » : efficacité de l'échangeur en représentation simplifiée, après prise en compte du
/// statut de la valeur saisie.
///
/// Th-BCE 2020 §6.3.3.3.3, éq. (396) à (398) : certifiée, `εsaisi` ; justifiée,
/// `0,9 × εsaisi` ; déclarée, `MIN(0,8 × εsaisi ; εutile_max)`.
pub fn epsilon_t(epsilon_saisi: f64, statut: StatutEchangeur) -> f64 {
    match statut {
        StatutEchangeur::Certifie => epsilon_saisi,
        StatutEchangeur::Justifie => 0.9 * epsilon_saisi,
        StatutEchangeur::Declare => (0.8 * epsilon_saisi).min(EPSILON_UTILE_MAX),
    }
}

/// « ε(h) » d'un échangeur en représentation détaillée, ramenée aux écarts de températures du
/// côté de l'air neuf.
///
/// Th-BCE 2020 §6.3.3.4.6.3, éq. (499) à (505), à partir des débits capacitifs
/// `Cair_neuf = Cpa × qm,air_neuf` et `Cair_rejeté = Cpa × qm,air_extrait` (W/K). Nulle si l'un
/// des deux débits est nul.
pub fn epsilon_detaille(
    ua_w_k: f64,
    ecoulement: EcoulementEchangeur,
    c_air_neuf_w_k: f64,
    c_air_rejete_w_k: f64,
) -> f64 {
    let c_min = c_air_neuf_w_k.min(c_air_rejete_w_k);
    let c_max = c_air_neuf_w_k.max(c_air_rejete_w_k);
    if c_min <= 0.0 {
        return 0.0;
    }
    // Éq. (499) et (500).
    let c = c_min / c_max;
    let nut = ua_w_k / c_min;
    let epsilon = match ecoulement {
        // Éq. (501) et (502).
        EcoulementEchangeur::ContreCourant => {
            if (1.0 - c).abs() < 1e-12 {
                nut / (nut + 1.0)
            } else {
                let e = libm::exp(-nut * (1.0 - c));
                (1.0 - e) / (1.0 - c * e)
            }
        }
        // Éq. (503).
        EcoulementEchangeur::CourantsParalleles => (1.0 - libm::exp(-nut * (1.0 + c))) / (1.0 + c),
        // Éq. (504).
        EcoulementEchangeur::CourantsCroisesBrasses => {
            if nut <= 0.0 {
                0.0
            } else {
                1.0 / (1.0 / (1.0 - libm::exp(-nut)) + c / (1.0 - libm::exp(-nut * c)) - 1.0 / nut)
            }
        }
    };
    // Éq. (505).
    c_min / c_air_neuf_w_k * epsilon
}

/// « δbypass(h) » : activation du bypass de l'échangeur.
///
/// Th-BCE 2020 §6.3.3.4.6.2, éq. (494) et (495) : trois conditions simultanées, avec les
/// consignes « hiver » en saison de chauffage et « été » le reste de l'année :
/// `θan < θi,fin(h−1)`, `θan > θext,bypass` et `θi,fin(h−1) > θint,bypass`.
pub fn delta_bypass(
    bypass: &Bypass,
    saison_de_chauffage: bool,
    theta_an_c: f64,
    theta_i_fin_c: f64,
) -> bool {
    let (theta_ext_bypass, theta_int_bypass) = if saison_de_chauffage {
        (
            bypass.theta_ext_bypass_hiver_c,
            bypass.theta_int_bypass_hiver_c,
        )
    } else {
        (bypass.theta_ext_bypass_ete_c, bypass.theta_int_bypass_ete_c)
    };
    theta_an_c < theta_i_fin_c && theta_an_c > theta_ext_bypass && theta_i_fin_c > theta_int_bypass
}

/// « θeech,min(h) » : température d'air neuf en deçà de laquelle l'échangeur givre, en °C.
///
/// Th-BCE 2020 §6.3.3.4.6.4, éq. (506) : `θextr3 + (θs,ech,rep,LIM − θextr3)/ε`, pour `ε > 0`.
pub fn theta_e_ech_min_c(theta_extr3_c: f64, theta_s_ech_rep_lim_c: f64, epsilon: f64) -> f64 {
    theta_extr3_c + (theta_s_ech_rep_lim_c - theta_extr3_c) / epsilon
}

/// Effet de la sécurité antigel ou du risque de givrage sur l'échangeur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Antigel {
    /// « ε(h) » après prise en compte du risque de givrage.
    pub epsilon: f64,
    /// « θsouf,ag(h) » : température de l'air neuf à l'entrée de l'échangeur, en °C.
    pub theta_souf_ag_c: f64,
    /// « Wag(h) » : énergie fournie pour la fonction antigel, en Wh (éq. 508 : `Cpa × qm ×
    /// ΔT` sur un pas de temps d'une heure).
    pub w_ag_wh: f64,
}

/// Sécurité antigel de l'échangeur.
///
/// Th-BCE 2020 §6.3.3.4.6.4 : avec un échangeur d'efficacité non nulle, en saison de chauffage
/// effective et avec un antigel, l'air neuf est préchauffé jusqu'à `θeech,min` (éq. 507 et 508) ;
/// sans antigel ou hors saison de chauffage, l'échangeur est contourné quand l'air neuf est
/// plus froid que `θeech,min` (éq. 509 à 511). `qm_air_neuf_kg_s` : débit d'air neuf (éq. 432).
pub fn antigel(
    epsilon: f64,
    is_antigel: bool,
    saison_de_chauffage: bool,
    theta_an_c: f64,
    theta_extr3_c: f64,
    theta_s_ech_rep_lim_c: f64,
    qm_air_neuf_kg_s: f64,
) -> Antigel {
    let sans_effet = Antigel {
        epsilon,
        theta_souf_ag_c: theta_an_c,
        w_ag_wh: 0.0,
    };
    // « si ε est nul, l'algorithme correspondant à la sécurité antigel n'est pas appelé ».
    if epsilon <= 0.0 {
        return sans_effet;
    }
    let theta_min = theta_e_ech_min_c(theta_extr3_c, theta_s_ech_rep_lim_c, epsilon);
    if is_antigel && saison_de_chauffage {
        // Éq. (507) et (508).
        let theta_souf_ag_c = theta_an_c.max(theta_min);
        Antigel {
            epsilon,
            theta_souf_ag_c,
            w_ag_wh: C_PA_J_KG_K * qm_air_neuf_kg_s * (theta_souf_ag_c - theta_an_c).max(0.0),
        }
    } else if theta_an_c < theta_min {
        // Éq. (509) à (511).
        Antigel {
            epsilon: 0.0,
            ..sans_effet
        }
    } else {
        sans_effet
    }
}

/// « θsouf1(h) » et « θair,rejeté(h) » : températures de l'air soufflé et de l'air rejeté à la
/// sortie de l'échangeur, en °C.
///
/// Th-BCE 2020 §6.3.3.4.6.5, éq. (512) et (513).
pub fn sorties_echangeur_c(theta_souf_ag_c: f64, theta_extr3_c: f64, epsilon: f64) -> (f64, f64) {
    let ecart = theta_extr3_c - theta_souf_ag_c;
    (
        theta_souf_ag_c + epsilon * ecart,
        theta_extr3_c - epsilon * ecart,
    )
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn statut_de_l_efficacite() {
        assert_eq!(epsilon_t(0.85, StatutEchangeur::Certifie), 0.85);
        assert!((epsilon_t(0.85, StatutEchangeur::Justifie) - 0.765).abs() < 1e-12);
        // Déclarée : MIN(0,68 ; 0,5).
        assert_eq!(epsilon_t(0.85, StatutEchangeur::Declare), 0.5);
        assert!((epsilon_t(0.5, StatutEchangeur::Declare) - 0.4).abs() < 1e-12);
    }

    #[test]
    fn efficacite_detaillee_a_la_main() {
        // Débits équilibrés (C = 1), NUT = 3 à contre-courant : 3/4.
        let e = epsilon_detaille(150.0, EcoulementEchangeur::ContreCourant, 50.0, 50.0);
        assert!((e - 0.75).abs() < 1e-12);
        // Courants parallèles, C = 1, NUT = 3 : (1 − e⁻⁶)/2.
        let e = epsilon_detaille(150.0, EcoulementEchangeur::CourantsParalleles, 50.0, 50.0);
        assert!((e - (1.0 - libm::exp(-6.0)) / 2.0).abs() < 1e-12);
        // Contre-courant, C = 0,5 (air rejeté le plus faible), NUT = 2 : ε sur Cmin, puis
        // ramenée à l'air neuf (éq. 505) × 0,5.
        let e = epsilon_detaille(50.0, EcoulementEchangeur::ContreCourant, 50.0, 25.0);
        let x = libm::exp(-2.0 * 0.5);
        assert!((e - 0.5 * (1.0 - x) / (1.0 - 0.5 * x)).abs() < 1e-12);
        // Croisés brassés, C = 1, NUT = 1 : 1/(2/(1 − e⁻¹) − 1).
        let e = epsilon_detaille(
            50.0,
            EcoulementEchangeur::CourantsCroisesBrasses,
            50.0,
            50.0,
        );
        let y = 1.0 - libm::exp(-1.0);
        assert!((e - 1.0 / (2.0 / y - 1.0)).abs() < 1e-12);
        assert_eq!(
            epsilon_detaille(50.0, EcoulementEchangeur::ContreCourant, 0.0, 50.0),
            0.0
        );
    }

    #[test]
    fn bypass_selon_la_saison() {
        let b = Bypass {
            theta_ext_bypass_hiver_c: 16.0,
            theta_ext_bypass_ete_c: 12.0,
            theta_int_bypass_hiver_c: 24.0,
            theta_int_bypass_ete_c: 22.0,
        };
        // Été : air neuf à 15 °C, intérieur à 25 °C.
        assert!(delta_bypass(&b, false, 15.0, 25.0));
        // Hiver : air neuf sous la consigne de 16 °C.
        assert!(!delta_bypass(&b, true, 15.0, 25.0));
        // Air neuf plus chaud que l'intérieur.
        assert!(!delta_bypass(&b, false, 26.0, 25.0));
        // Intérieur sous la consigne d'été.
        assert!(!delta_bypass(&b, false, 15.0, 21.0));
    }

    #[test]
    fn antigel_a_la_main() {
        // Air repris à 20 °C, ε = 0,8, limite à 5 °C : θeech,min = 20 + (5 − 20)/0,8 = 1,25 °C.
        assert!((theta_e_ech_min_c(20.0, 5.0, 0.8) - 1.25).abs() < 1e-12);
        // Avec antigel en saison de chauffage, air neuf à −5 °C : préchauffé à 1,25 °C.
        let a = antigel(0.8, true, true, -5.0, 20.0, 5.0, 0.04);
        assert!((a.theta_souf_ag_c - 1.25).abs() < 1e-12);
        assert!((a.w_ag_wh - 1006.0 * 0.04 * 6.25).abs() < 1e-9);
        assert_eq!(a.epsilon, 0.8);
        // Sans antigel : échangeur contourné.
        let a = antigel(0.8, false, true, -5.0, 20.0, 5.0, 0.04);
        assert_eq!(a.epsilon, 0.0);
        assert_eq!(a.theta_souf_ag_c, -5.0);
        assert_eq!(a.w_ag_wh, 0.0);
        // Air neuf plus chaud que le seuil : rien.
        let a = antigel(0.8, false, true, 5.0, 20.0, 5.0, 0.04);
        assert_eq!(a.epsilon, 0.8);
        // Sorties de l'échangeur : 1,25 + 0,8 × 18,75 = 16,25 °C ; rejet 20 − 15 = 5 °C.
        let (s, r) = sorties_echangeur_c(1.25, 20.0, 0.8);
        assert!((s - 16.25).abs() < 1e-12);
        assert!((r - 5.0).abs() < 1e-12);
    }
}
