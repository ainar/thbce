// SPDX-License-Identifier: GPL-3.0-or-later
//! Distributions de chauffage et de refroidissement du groupe et intergroupes (Th-BCE 2020
//! §8.7 à 8.10) : valeurs calculées à la main.
#![allow(clippy::float_cmp, clippy::unwrap_used)]

use thbce_core::chauffage_refroidissement::distribution::{
    mod_circ, pas_hydraulique_groupe, ApportReseauGroupe, DistributionIntergroupesResolue,
};
use thbce_core::entree::{
    Circulateur, DistributionIntergroupes, FonctionReseau, ReseauGroupe, ReseauHydrauliqueGroupe,
};

macro_rules! assert_proche {
    ($a:expr, $b:expr) => {{
        let (a, b): (f64, f64) = ($a, $b);
        assert!((a - b).abs() <= 1e-9, "{a} ≠ {b}");
    }};
}

const RHO_CP: f64 = 998.0 * 1.163;

fn reseau(json: &str) -> ReseauHydrauliqueGroupe {
    serde_json::from_str(json).unwrap()
}

fn plancher() -> ReseauHydrauliqueGroupe {
    reseau(
        r#"{"regulation_debit": "debit_variable", "regulation_temperature": "depart_constant",
            "theta_dep_dim_c": 35, "delta_theta_em_dim_k": 5, "q_nom_m3_h": 1.0,
            "q_resid_m3_h": 0.2, "circulateur": "pression_constante", "p_aux_w": 30,
            "l_vc_m": 20, "l_hvc_m": 10, "u_vc_w_mk": 0.2, "u_hvc_w_mk": 0.3}"#,
    )
}

#[test]
fn reseau_du_groupe_a_debit_variable_eq_855_859_871_875_880() {
    let r = plancher();
    // 2 000 Wh : qreq = 2 000/(1 160,67 × 5) = 0,3446 m³/h > qresid ; Δθem = 5 K.
    let s = pas_hydraulique_groupe(
        &r,
        FonctionReseau::Chauffage,
        2000.0,
        20.0,
        0.0,
        -5.0,
        0.5,
        false,
    );
    let q = 2000.0 / (RHO_CP * 5.0);
    assert_proche!(s.q_eff_m3_h, q);
    assert_eq!((s.theta_dep_c, s.mod_pertes), (35.0, 1.0));
    assert_proche!(s.theta_ret_c, 30.0);
    // Pertes : θmoy = 32,5 ; vc 0,2 × 20 × 12,5 = 50 Wh ; θhvc = 0,5 × 0 + 0,5 × 20 = 10 °C,
    // hvc 0,3 × 10 × 22,5 = 67,5 Wh.
    assert_proche!(s.phi_pertes_vc_wh, 50.0);
    assert_proche!(s.phi_pertes_hvc_wh, 67.5);
    assert_proche!(s.q_sys_ch_wh, 2117.5);
    // Circulateur à pression constante : 30 × (q/1)^(2/3) ; la moitié au volume chauffé.
    assert_proche!(s.w_aux_wh, 30.0 * libm::pow(q, 2.0 / 3.0));
    assert_proche!(s.phi_aux_vc_wh, s.w_aux_wh / 2.0);
    // Faible demande : débit résiduel, écart réduit.
    let s = pas_hydraulique_groupe(
        &r,
        FonctionReseau::Chauffage,
        100.0,
        20.0,
        0.0,
        -5.0,
        1.0,
        false,
    );
    assert_proche!(s.q_eff_m3_h, 0.2);
    assert_proche!(s.theta_ret_c, 35.0 - 100.0 / (RHO_CP * 0.2));
    // À l'arrêt : départ et retour à la température de l'air, ni pertes ni circulateur.
    let s = pas_hydraulique_groupe(
        &r,
        FonctionReseau::Chauffage,
        0.0,
        20.0,
        0.0,
        -5.0,
        1.0,
        false,
    );
    assert_eq!(
        (s.theta_dep_c, s.theta_ret_c, s.q_sys_ch_wh, s.w_aux_wh),
        (20.0, 20.0, 0.0, 0.0)
    );
}

#[test]
fn intermittence_et_loi_d_eau_eq_856_861() {
    let r = reseau(
        r#"{"regulation_debit": "debit_constant_intermittent", "regulation_temperature": "loi_d_eau",
            "theta_dep_dim_c": 45, "delta_theta_em_dim_k": 10, "q_nom_m3_h": 0.5,
            "circulateur": "vitesse_constante", "p_aux_w": 20,
            "l_vc_m": 10, "l_hvc_m": 0, "u_vc_w_mk": 0.25, "u_hvc_w_mk": 0}"#,
    );
    // Modpertes = 2 000/(1 160,67 × 0,5 × 10) = 0,3446.
    let m = 2000.0 / (RHO_CP * 0.5 * 10.0);
    // Loi d'eau entre la base (−5 °C) et 15 °C : à 5 °C, 45 + (20 − 45)/20 × 10 = 32,5 °C.
    let s = pas_hydraulique_groupe(
        &r,
        FonctionReseau::Chauffage,
        2000.0,
        20.0,
        5.0,
        -5.0,
        1.0,
        false,
    );
    assert_proche!(s.mod_pertes, m);
    assert_proche!(s.theta_dep_c, 32.5);
    assert_proche!(s.w_aux_wh, 20.0 * m);
    // Au-delà de 15 °C : max(θi + Δθem ; 20) = 30 °C ; sous la base : 45 °C.
    let s = pas_hydraulique_groupe(
        &r,
        FonctionReseau::Chauffage,
        2000.0,
        20.0,
        16.0,
        -5.0,
        1.0,
        false,
    );
    assert_proche!(s.theta_dep_c, 30.0);
    let s = pas_hydraulique_groupe(
        &r,
        FonctionReseau::Chauffage,
        2000.0,
        20.0,
        -8.0,
        -5.0,
        1.0,
        false,
    );
    assert_proche!(s.theta_dep_c, 45.0);
}

#[test]
fn froid_a_retour_constant_eq_864_869_882() {
    let r = reseau(
        r#"{"regulation_debit": "debit_constant_continu", "regulation_temperature": "retour_constant",
            "theta_ret_dim_c": 12, "delta_theta_em_dim_k": -5, "q_nom_m3_h": 0.5,
            "circulateur": "aucun", "p_aux_w": 0,
            "l_vc_m": 10, "l_hvc_m": 0, "u_vc_w_mk": 0.2, "u_hvc_w_mk": 0}"#,
    );
    // Δθem = −1 000/(1 160,67 × 0,5) = −1,723 K ; θdep = 12 − 1,723 ; θret = 12.
    let s = pas_hydraulique_groupe(
        &r,
        FonctionReseau::Refroidissement,
        -1000.0,
        26.0,
        30.0,
        -5.0,
        1.0,
        false,
    );
    let dt = -1000.0 / (RHO_CP * 0.5);
    assert_proche!(s.theta_dep_c, 12.0 + dt);
    assert_proche!(s.theta_ret_c, 12.0);
    // Pertes négatives (gains de chaleur du réseau froid), éq. (882).
    assert_proche!(s.phi_pertes_vc_wh, 0.2 * 10.0 * (12.0 + dt / 2.0 - 26.0));
    assert!(s.q_sys_fr_wh < -1000.0);
}

#[test]
fn modulation_des_circulateurs_eq_862() {
    assert_eq!(mod_circ(Circulateur::Aucun, 1.0, 0.5, 1.0), 0.0);
    assert_eq!(mod_circ(Circulateur::VitesseConstante, 0.4, 0.5, 1.0), 0.4);
    assert_proche!(
        mod_circ(Circulateur::PressionVariable, 1.0, 0.5, 1.0),
        libm::pow(0.25 + 0.125, 2.0 / 3.0)
    );
}

#[test]
fn reseau_intergroupes_collectif_eq_904_a_928() {
    let d: DistributionIntergroupes = serde_json::from_str(
        r#"{"id": "dp", "fonction": "chauffage", "generation": "g",
            "reseau": {"hydraulique_collectif": {"l_vc_m": 30, "l_hvc_m": 10,
              "u_vc_w_mk": 0.2, "u_hvc_w_mk": 0.3, "p_aux_w": 60,
              "circulateur": "vitesse_constante"}}}"#,
    )
    .unwrap();
    let mut dp = DistributionIntergroupesResolue::nouvelle(&d, |_| Some(0), |_| None);
    let r = plancher();
    dp.ajouter_reseau_groupe(&ReseauGroupe::Hydraulique(r.clone()));
    dp.ajouter_reseau_groupe(&ReseauGroupe::Hydraulique(r.clone()));
    assert_proche!(dp.q_resid_m3_h, 0.4);
    let a = pas_hydraulique_groupe(
        &r,
        FonctionReseau::Chauffage,
        2000.0,
        20.0,
        0.0,
        -5.0,
        1.0,
        false,
    );
    let b = pas_hydraulique_groupe(
        &r,
        FonctionReseau::Chauffage,
        0.0,
        21.0,
        0.0,
        -5.0,
        1.0,
        true,
    );
    let aval = [
        ApportReseauGroupe {
            sorties: a,
            rat_surf: 0.5,
            theta_i_moy_c: 20.0,
        },
        ApportReseauGroupe {
            sorties: b,
            rat_surf: 0.5,
            theta_i_moy_c: 21.0,
        },
    ];
    let s = dp.pas(&aval, 0.0, 1.0);
    // Départ du réseau qui débite ; qeff = max(0,3446 ; 0,4) = 0,4 : la décharge mélange
    // 0,0554 m³/h à 35 °C au retour à 30 °C (éq. 918).
    assert_eq!(s.theta_dep_c, 35.0);
    let q = 2000.0 / (RHO_CP * 5.0);
    assert_proche!(s.theta_ret_c, (q * 30.0 + (0.4 - q) * 35.0) / 0.4);
    assert!(s.relance);
    // Pertes à 20 °C en volume chauffé (θvc), à l'extérieur hors volume chauffé.
    let moy = (35.0 + s.theta_ret_c) / 2.0;
    assert_proche!(s.phi_pertes_vc_wh, 0.2 * 30.0 * (moy - 20.0));
    assert_proche!(s.phi_pertes_hvc_wh, 0.3 * 10.0 * moy);
    assert_proche!(
        s.q_sys_ch_wh,
        a.q_sys_ch_wh + s.phi_pertes_vc_wh + s.phi_pertes_hvc_wh
    );
    assert_eq!((s.w_aux_wh, s.phi_aux_vc_wh), (60.0, 0.0));
}
