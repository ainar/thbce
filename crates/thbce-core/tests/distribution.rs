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

mod ecs {
    use thbce_core::ecs::besoins_ecs::{corr_e, BesoinsHebdomadaires};
    use thbce_core::ecs::distribution_ecs::{pas_intergroupe, DistributionEcsGroupeResolue};
    use thbce_core::entree::{EmetteurEcs, ReseauEcsIntergroupe, UsageZone};
    use thbce_core::pertes::pertes_recuperables::{recuperation_wh, FluxVolumeChauffe};

    fn emetteur() -> EmetteurEcs {
        serde_json::from_str(
            r#"{"id": "e", "rat_em": 1, "nb_logements": 1, "part_melangeur": 0.4,
                "part_mitigeur": 0.6, "part_temporisateur": 0, "appareils": "douche_seule",
                "distribution": {"d_int_m": 0.016, "l_hvc_m": 2, "nb_dist": 1},
                "distribution_intergroupe": "dp"}"#,
        )
        .unwrap()
    }

    /// Bureaux : tableaux 274 et 277, §9.6.3.2.2.1.
    #[test]
    fn besoins_hebdomadaires_en_bureaux() {
        let e: EmetteurEcs = serde_json::from_str(
            r#"{"id": "e", "rat_em": 0.5, "part_melangeur": 0, "part_mitigeur": 0,
                "part_temporisateur": 1, "appareils": "douche_seule",
                "distribution": {"d_int_m": 0.016, "l_hvc_m": 2, "nb_dist": 1},
                "distribution_intergroupe": "dp"}"#,
        )
        .unwrap();
        // corr = (1 − 1 × 0,07) × (1 − 0,5 × 0,05) = 0,93 × 0,975 (Ratdouches-bains = 50 %).
        assert_proche!(corr_e(&e, UsageZone::Bureaux), 0.93 * 0.975);
        // Autres usages : Ratdouches-bains = 0, seuls les émetteurs corrigent.
        assert_proche!(corr_e(&e, UsageZone::Commerces), 0.93);
        // Groupe de 800 m² utiles, émetteur desservant la moitié : Nu = 400 m² ;
        // a = 1,25 L/(m².semaine) ; Vuw_hebdo = 500 L.
        let b = BesoinsHebdomadaires::nouveaux(&e, UsageZone::Bureaux, 800.0).unwrap();
        assert_proche!(b.a_m2, 400.0);
        assert_proche!(b.nadeq, 400.0);
        assert_proche!(b.a_l, 1.25);
        assert_proche!(b.v_uw_hebdo_l, 500.0);
        assert_proche!(b.v_uw_hebdo_corr_l, 500.0 * 0.93 * 0.975);
        // Qw = 1,163 × 500 × 0,93 × 0,975 × 0,012 × (40 − 12).
        assert_proche!(
            b.q_w_wh(0.012, 12.0),
            1.163 * 500.0 * 0.93 * 0.975 * 0.012 * 28.0
        );
        // Nombre d'unités saisi : il remplace la surface desservie.
        let e2 = EmetteurEcs {
            nu: Some(250.0),
            ..e
        };
        let b = BesoinsHebdomadaires::nouveaux(&e2, UsageZone::Bureaux, 800.0).unwrap();
        assert_proche!(b.v_uw_hebdo_l, 312.5);
    }

    #[test]
    fn besoins_hebdomadaires_eq_1676_a_1692() {
        let e = emetteur();
        // corr = (1 − 0,6 × 0,05) × (1 − 0,8 × 0,05) = 0,97 × 0,96.
        assert_proche!(corr_e(&e, UsageZone::MaisonIndividuelle), 0.97 * 0.96);
        // Maison de 100 m² : Nmax = 2,5 ; Nadeq = 1,75 + 0,3 × 0,75 = 1,975 ;
        // a = min(392 ; 40 × 100/1,975) = 392 L.
        let b = BesoinsHebdomadaires::nouveaux(&e, UsageZone::MaisonIndividuelle, 100.0).unwrap();
        assert_proche!(b.nadeq, 1.975);
        assert_proche!(b.a_l, 392.0);
        assert_proche!(b.v_uw_hebdo_corr_l, 392.0 * 1.975 * 0.97 * 0.96);
        // Logement de 30 m² en collectif : Nmax = 1,75 − 0,01875 × 20 = 1,375 ;
        // a = min(392 ; 40 × 30/1,375) = 392 ; 60 m² sur 4 logements : 15 m² chacun,
        // Nmax = 1,75 − 0,01875 × 35 = 1,09375 ; a = 40 × 60/(4 × 1,09375) = 548,6 → 392.
        let b = BesoinsHebdomadaires::nouveaux(&e, UsageZone::LogementCollectif, 30.0).unwrap();
        assert_proche!(b.nadeq, 1.375);
        // Qw = 1,163 × Vcorr × ah × (40 − θcw).
        assert_proche!(
            b.q_w_wh(0.01, 10.0),
            1.163 * b.v_uw_hebdo_corr_l * 0.01 * 30.0
        );
    }

    #[test]
    fn distribution_du_groupe_eq_1729_a_1734() {
        let e = emetteur();
        let mut d = DistributionEcsGroupeResolue::nouvelle(
            &e.distribution,
            UsageZone::MaisonIndividuelle,
            80.0,
            None,
        );
        // Longueur par défaut 6 × 80/80 = 6 m ; V = 6 × π × 0,016²/4 × 1000 = 1,2064 L.
        let v_vc = 6.0 * core::f64::consts::PI * 0.016 * 0.016 / 4.0 * 1000.0;
        assert_proche!(d.v_vc_l, v_vc);
        // Premier puisage : Is = 1, 3 bouchons ; 53 °C, groupe à 20 °C, hors volume chauffé à
        // l'extérieur à 0 °C.
        let s = d.pas(1000.0, 20.0, 0.0, 1.0);
        assert_proche!(s.phi_pertes_vc_wh, 1.163 * v_vc * 33.0 * 3.0);
        let v_hvc = 2.0 * core::f64::consts::PI * 0.016 * 0.016 / 4.0 * 1000.0;
        assert_proche!(s.phi_pertes_hvc_wh, 1.163 * v_hvc * 53.0 * 3.0);
        assert_proche!(
            s.q_w_2nd_wh,
            1000.0 + s.phi_pertes_vc_wh + s.phi_pertes_hvc_wh
        );
        // Puisage suivant : Is = 2/3 ; sans puisage : pas de pertes.
        let s = d.pas(500.0, 20.0, 0.0, 1.0);
        assert_proche!(s.phi_pertes_vc_wh, 1.163 * v_vc * 33.0 * 2.0);
        assert_eq!(d.pas(0.0, 20.0, 0.0, 1.0).q_w_2nd_wh, 0.0);
    }

    #[test]
    fn distribution_intergroupe_eq_1740_a_1764() {
        let boucle: ReseauEcsIntergroupe = serde_json::from_str(
            r#"{"boucle": {"l_vc_m": 20, "l_hvc_m": 10, "u_w_mk": 0.3, "rechauffeur": false,
                "p_circ_w": 40}}"#,
        )
        .unwrap();
        // Départ 55 °C, retour 50 °C, moyenne 52,5 °C : vc 0,3 × 20 × 32,5 = 195 Wh ;
        // hvc à l'extérieur (0 °C) : 0,3 × 10 × 52,5 = 157,5 Wh.
        let s = pas_intergroupe(
            &boucle,
            &[(1000.0, 55.0), (500.0, 53.0)],
            true,
            10.0,
            0.0,
            1.0,
        );
        assert_eq!((s.theta_depart_c, s.theta_retour_c), (55.0, 50.0));
        assert_proche!(s.phi_pertes_vc_wh, 195.0);
        assert_proche!(s.phi_pertes_hvc_wh, 157.5);
        assert_proche!(s.q_w_prim_wh, 1500.0 + 195.0 + 157.5);
        assert_eq!((s.w_aux_wh, s.phi_aux_vc_wh), (40.0, 0.0));
        // Réseau tracé : pertes compensées par le traceur, non ajoutées aux besoins.
        let trace: ReseauEcsIntergroupe =
            serde_json::from_str(r#"{"trace": {"l_vc_m": 10, "l_hvc_m": 0, "u_w_mk": 0.2}}"#)
                .unwrap();
        let s = pas_intergroupe(&trace, &[(1000.0, 55.0)], true, 10.0, 0.0, 1.0);
        assert_proche!(s.w_aux_wh, 0.2 * 10.0 * 35.0);
        assert_eq!((s.q_w_prim_wh, s.theta_retour_c), (1000.0, 10.0));
    }

    /// Éq. (47), (1736), (1744), (1747) et (1748) : réseau intergroupe arrêté pendant les
    /// vacances d'une zone d'enseignement (lecture Q71).
    #[test]
    fn distribution_intergroupe_arretee_en_vacances() {
        use thbce_core::ecs::distribution_ecs::{id_encl, w_circ_prim_wh};
        // Éq. (1736) : le réseau fonctionne dès qu'une distribution reliée fonctionne.
        assert!(id_encl([false, true]));
        assert!(!id_encl([false, false]));
        let avec_gestion: ReseauEcsIntergroupe = serde_json::from_str(
            r#"{"boucle": {"l_vc_m": 20, "l_hvc_m": 10, "u_w_mk": 0.3, "rechauffeur": true,
                "p_circ_w": 40, "arret_circulateurs_vacances": true}}"#,
        )
        .unwrap();
        // Réseau arrêté : ni pertes (éq. 1744), ni réchauffeur (éq. 1746), ni circulateurs
        // (éq. 1747) ; les besoins des groupes passent tels quels.
        let s = pas_intergroupe(&avec_gestion, &[(300.0, 55.0)], false, 10.0, 0.0, 1.0);
        assert_eq!(
            (
                s.phi_pertes_vc_wh,
                s.phi_pertes_hvc_wh,
                s.w_rechauf_wh,
                s.w_aux_wh
            ),
            (0.0, 0.0, 0.0, 0.0)
        );
        assert_eq!(s.q_w_prim_wh, 300.0);
        // En service : 0,3 × 20 × 32,5 = 195 Wh et 0,3 × 10 × 52,5 = 157,5 Wh compensés par le
        // réchauffeur ; 40 Wh de circulateurs.
        let s = pas_intergroupe(&avec_gestion, &[(300.0, 55.0)], true, 10.0, 0.0, 1.0);
        assert_proche!(s.w_rechauf_wh, 195.0 + 157.5);
        assert_eq!((s.w_aux_wh, s.q_w_prim_wh), (40.0, 300.0));
        // Sans gestion (« Typegest_circ-e = 0 », éq. 1748) : circulateurs permanents, pertes
        // nulles pendant l'arrêt (éq. 1744).
        let sans_gestion: ReseauEcsIntergroupe = serde_json::from_str(
            r#"{"boucle": {"l_vc_m": 20, "l_hvc_m": 10, "u_w_mk": 0.3, "rechauffeur": false,
                "p_circ_w": 40, "arret_circulateurs_vacances": false}}"#,
        )
        .unwrap();
        let s = pas_intergroupe(&sans_gestion, &[(300.0, 55.0)], false, 10.0, 0.0, 1.0);
        assert_eq!((s.w_aux_wh, s.phi_pertes_vc_wh), (40.0, 0.0));
        if let ReseauEcsIntergroupe::Boucle(b) = &sans_gestion {
            assert_eq!(w_circ_prim_wh(b, false), 40.0);
        }
        if let ReseauEcsIntergroupe::Boucle(b) = &avec_gestion {
            assert_eq!(
                (w_circ_prim_wh(b, false), w_circ_prim_wh(b, true)),
                (0.0, 40.0)
            );
        }
        // Réseau tracé arrêté : traceur à l'arrêt (pertes nulles).
        let trace: ReseauEcsIntergroupe =
            serde_json::from_str(r#"{"trace": {"l_vc_m": 10, "l_hvc_m": 0, "u_w_mk": 0.2}}"#)
                .unwrap();
        let s = pas_intergroupe(&trace, &[(1000.0, 55.0)], false, 10.0, 0.0, 1.0);
        assert_eq!(s.w_aux_wh, 0.0);
    }

    #[test]
    fn recuperation_eq_2355_a_2365() {
        let f = FluxVolumeChauffe {
            vent_loc_wh: 10.0,
            aux_dgr_wh: 5.0,
            pertes_dgr_wh: 100.0,
            pertes_dgr_ecs_wh: 50.0,
            pertes_dintgr_wh: 20.0,
            ..FluxVolumeChauffe::default()
        };
        // Récupéré : 10 + 0,6 × 5 + 100 + 0,6 × 50 + 0,6 × 20 = 155 Wh ;
        // convectif : 10 + 0,5 × 145 = 82,5 Wh.
        let (conv, rad) = recuperation_wh(&f);
        assert_proche!(conv, 82.5);
        assert_proche!(rad, 72.5);
    }
}
