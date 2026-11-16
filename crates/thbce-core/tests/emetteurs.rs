// SPDX-License-Identifier: GPL-3.0-or-later
//! Émetteurs du projet, relances, saisons de fonctionnement et bilan hydrique avec
//! déshumidification (Th-BCE 2020 §8.1, §8.3, §8.4 et §8.5) : valeurs calculées à la main.
#![allow(clippy::float_cmp, clippy::unwrap_used, clippy::disallowed_methods)]

use thbce_core::chauffage_refroidissement::bilan_hydrique::{
    bilan_th_c, calcul_humidite_spe_fin, calcul_humidite_spe_moy, calcul_temps, omega_sat_kg_kg,
    BatterieFroide, EchangesHydriques,
};
use thbce_core::chauffage_refroidissement::emetteurs::{
    besoins_emetteur_wh, delta_theta_vs_ch_classe_k, delta_theta_vs_fr_classe_k, delta_theta_vt_k,
    regime_ventilateurs, ConditionsVentilateurs, EmetteurResolu, EmissionEquivalente,
    RegimeVentilateurs,
};
use thbce_core::chauffage_refroidissement::relances::{
    duree_relance_ch_h, en_relance, ConditionsRelance, Relance,
};
use thbce_core::chauffage_refroidissement::saisons_fonctionnement::{
    autorisations_effectives, DesserteGroupe,
};
use thbce_core::entree::{
    ClasseEmetteurChaud, ClasseEmetteurFroid, Emetteur, GestionVentilateurs,
    ProgrammationChauffage, ProgrammationRefroidissement, VariationTemporelle,
};

macro_rules! assert_proche {
    ($a:expr, $b:expr) => {
        assert_proche!($a, $b, 1e-9)
    };
    ($a:expr, $b:expr, $tol:expr) => {{
        let (a, b): (f64, f64) = ($a, $b);
        assert!((a - b).abs() <= $tol, "{a} ≠ {b}");
    }};
}

fn emetteur(json: &str) -> EmetteurResolu {
    let e: Emetteur = serde_json::from_str(json).unwrap();
    EmetteurResolu::nouveau(&e, |_| Some(0))
}

fn plancher() -> EmetteurResolu {
    // Plancher chauffant (Pem = 0,5, classe A : 0 K), régulation certifiée 0,5 K, desservant
    // 60 % du groupe ; pertes au dos Ri·Ue = 0,15 × 0,2 = 0,03.
    emetteur(
        r#"{"id": "plancher",
            "chauffage": {"type_emetteur": "plancher_chauffant",
              "variation_spatiale": {"classe": {"classe": "A", "hsp_m": 2.5}},
              "variation_temporelle": {"certifiee": {"delta_theta_k": 0.5}},
              "effet_joule": false, "rats": 0.6, "ratt": 1.0, "distribution": "d"},
            "pertes_au_dos": {"r_i_m2k_w": 0.15, "u_e_w_m2k": 0.2}}"#,
    )
}

fn convecteurs() -> EmetteurResolu {
    // Convecteurs (Pem = 0,95, classe C : 0,4 K à 2,5 m), tableau 87 sans arrêt total : 2 K,
    // 40 % du groupe, détection de présence.
    emetteur(
        r#"{"id": "convecteurs",
            "chauffage": {"type_emetteur": "soufflage_air_chaud",
              "variation_spatiale": {"classe": {"classe": "C", "hsp_m": 2.5}},
              "variation_temporelle": {"defaut": {"arret_total": false}},
              "effet_joule": true, "rats": 0.4, "ratt": 1.0, "distribution": "d"},
            "detection_presence": true}"#,
    )
}

#[test]
fn tableaux_82_84_et_87() {
    assert_eq!(
        delta_theta_vs_ch_classe_k(ClasseEmetteurChaud::B2, 3.99),
        0.0
    );
    assert_eq!(
        delta_theta_vs_ch_classe_k(ClasseEmetteurChaud::B2, 4.0),
        0.6
    );
    assert_eq!(delta_theta_vs_ch_classe_k(ClasseEmetteurChaud::C, 9.0), 2.8);
    assert_eq!(delta_theta_vs_fr_classe_k(ClasseEmetteurFroid::A, 6.0), 0.8);
    assert_eq!(
        delta_theta_vt_k(&VariationTemporelle::Defaut { arret_total: true }, false),
        -1.8
    );
    assert_eq!(
        delta_theta_vt_k(&VariationTemporelle::Justifiee { delta_theta_k: 0.4 }, true),
        0.9
    );
}

#[test]
fn emetteur_equivalent_eq_797_a_799() {
    let e = [plancher(), convecteurs()];
    let eq = EmissionEquivalente::nouvelle(&e);
    assert_proche!(eq.rat_em_eq_ch, 1.0);
    assert_eq!(eq.rat_eff_ch, vec![0.6, 0.4]);
    // Pem = 0,6 × 0,5 + 0,4 × 0,95 = 0,68 ; δθvs = 0,4 × 0,4 = 0,16 ;
    // δθvt = 0,6 × 0,5 + 0,4 × 2 = 1,1 ; δθpresence = 0,4 × (−0,15) = −0,06.
    assert_proche!(eq.chaud.p_em_conv, 0.68);
    assert_proche!(eq.chaud.delta_theta_vs_k, 0.16);
    assert_proche!(eq.chaud.delta_theta_vt_k, 1.1);
    assert_proche!(eq.chaud.delta_theta_presence_k, -0.06);
    assert_proche!(eq.chaud.p_sd, 0.5);
    // Éq. (810) : 1 000 W utiles ; plancher 0,6 × 1 000/(1 − 0,03) = 618,56 Wh.
    let (ch, fr) = besoins_emetteur_wh(&e[0], 0.6, 0.0, 1000.0);
    assert_proche!(ch, 600.0 / 0.97);
    assert_eq!(fr, 0.0);
}

#[test]
fn rats_partiels_normes_eq_797() {
    // Un seul émetteur desservant la moitié du groupe : Rateff = 1 (éq. 797).
    let mut p = plancher();
    p.chaud.as_mut().unwrap().rat = 0.5;
    let eq = EmissionEquivalente::nouvelle(&[p]);
    assert_eq!((eq.rat_em_eq_ch, eq.rat_eff_ch[0]), (0.5, 1.0));
}

fn conditions_relance(p_ch: [f64; 7]) -> ConditionsRelance {
    ConditionsRelance {
        aut_ch: true,
        aut_fr: true,
        p_ch,
        p_fr: [1.0; 7],
        theta_ext_c: 0.0,
        theta_ext_base_c: -5.0,
        theta_ii_ch_plus_c: 19.0,
        theta_ii_fr_plus_c: 26.0,
    }
}

#[test]
fn relance_horloge_deux_heures_avant_l_occupation() {
    let mut r = Relance::nouvelle(ProgrammationChauffage::Horloge, None);
    // Occupation dans 3 h : pas encore de relance (Δt = 2 h).
    let c = r.pas(&conditions_relance([0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]));
    assert!(!en_relance(&c));
    // Occupation dans 2 h : relance déclenchée, puis maintenue jusqu'à l'occupation.
    let c = r.pas(&conditions_relance([0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0]));
    assert_eq!(c.theta_ii_ch_relance_c, 19.0);
    let c = r.pas(&conditions_relance([0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]));
    assert_eq!(c.theta_ii_ch_relance_c, 19.0);
    let c = r.pas(&conditions_relance([1.0; 7]));
    assert_eq!(c.theta_ii_ch_relance_c, 0.0);
    // Froid non programmé (groupe non climatisé) : 100 °C.
    assert_eq!(c.theta_ii_fr_relance_c, 100.0);
}

#[test]
fn optimiseur_eq_844() {
    // Δt = ARRONDI(3/(θbase − 15) × (θext − 15)) borné à [0 ; 3] : base −5 °C.
    let d = |t| duree_relance_ch_h(ProgrammationChauffage::Optimiseur, -1.0, t, -5.0);
    assert_eq!(
        (d(-5.0), d(5.0), d(8.0), d(15.0), d(-20.0)),
        (3, 2, 1, 0, 3)
    );
    // Inoccupation courte : 1 h.
    assert_eq!(
        duree_relance_ch_h(ProgrammationChauffage::Optimiseur, 0.0, 0.0, -5.0),
        1
    );
}

#[test]
fn refroidissement_sans_horloge_eq_847() {
    let mut r = Relance::nouvelle(
        ProgrammationChauffage::Horloge,
        Some(ProgrammationRefroidissement::SansHorloge),
    );
    let mut c = conditions_relance([1.0; 7]);
    c.p_fr = [0.0; 7];
    assert_eq!(r.pas(&c).theta_ii_fr_relance_c, 26.0);
    c.aut_fr = false;
    assert_eq!(r.pas(&c).theta_ii_fr_relance_c, 100.0);
}

#[test]
fn saisons_effectives_eq_839_a_841() {
    // Deux groupes desservis par une génération à raccordement permanent ; le second n'est pas
    // climatisé ; un troisième groupe est indépendant (génération avec isolement).
    let d = |chaud: Vec<usize>, froid: Vec<usize>, i_clim| DesserteGroupe {
        chaud,
        froid,
        i_clim,
    };
    let propres = [(true, false), (false, true), (false, false)];
    let dessertes = [
        d(vec![0], vec![0], true),
        d(vec![0], vec![], false),
        d(vec![1], vec![], false),
    ];
    let (eff, gen) = autorisations_effectives(&propres, &dessertes, &[true, false]);
    // Génération 0 : chauffe (groupe 1), ne refroidit pas (le groupe 2 n'est pas climatisé).
    assert_eq!(gen, vec![(true, false), (false, false)]);
    assert_eq!(eff, vec![(true, false), (true, false), (false, false)]);
}

fn echanges() -> EchangesHydriques {
    EchangesHydriques {
        somme_q_maj_kg_s: 0.03,
        somme_q_omega_maj_kg_s: 0.03 * 0.012,
        a_omega_int_kg_s: 0.1 / 3600.0,
        v_m3: 250.0,
        rho_i_kg_m3: 1.2,
    }
}

#[test]
fn fonctions_du_bilan_hydrique_eq_831_a_833() {
    let e = echanges();
    // ω converge vers (0,03 × 0,012 + 0,1/3600)/0,03 = 0,012926 ; constante B = 0,03/300.
    let fin = calcul_humidite_spe_fin(1.0, 0.0, 0.0, 3600.0, 0.010, &e);
    let limite = (0.03 * 0.012 + 0.1 / 3600.0) / 0.03;
    assert_proche!(
        fin,
        0.010 + (limite - 0.010) * (1.0 - (-0.36_f64).exp()),
        1e-12
    );
    // CalculTemps inverse CalculHumiditeSpeFin.
    let t = calcul_temps(1.0, 0.0, 0.0, 0.010, fin, &e).unwrap();
    assert_proche!(t, 3600.0, 1e-6);
    // La moyenne est comprise entre l'état initial et l'état final.
    let moy = calcul_humidite_spe_moy(1.0, 0.0, 0.0, 3600.0, 0.010, &e);
    assert!(0.010 < moy && moy < fin);
    // Éq. (823) : 0,0148 kg/kg à 20 °C, environ 0,0072 kg/kg à 9 °C.
    assert_proche!(
        omega_sat_kg_kg(20.0),
        1e-3 * (18.8161 - 4110.34 / 255.0_f64).exp()
    );
    assert!((omega_sat_kg_kg(9.0) - 0.0072).abs() < 3e-4);
}

#[test]
fn deshumidification_et_energie_latente_eq_835_836() {
    let e = echanges();
    // Batterie à température constante de 9 °C, 1 000 Wh de froid sensible à 26 °C :
    // Qm_eff = 1 000/(1 006 × 17) = 0,05847 kg/s (éq. 822).
    let b = BatterieFroide::nouvelle(1.0, 0.1, -1000.0, 9.0, 26.0, false);
    assert_proche!(b.q_m_recirc_eff_kg_s, 1000.0 / (1006.0 * 17.0));
    let (fin, latent) = bilan_th_c(0.012, &[(1.0, Some(b))], true, &e);
    // L'air est au-dessus de la saturation de la batterie : déshumidification, énergie
    // latente négative, humidité finale sous celle de l'évolution sans batterie.
    assert!(latent[0] < 0.0);
    let sans = calcul_humidite_spe_fin(1.0, 0.0, 0.0, 3600.0, 0.012, &e);
    assert!(fin < sans);
    // Sans sollicitation du froid, le bilan est celui du Bbio.
    let (fin0, latent0) = bilan_th_c(0.012, &[(1.0, Some(b))], false, &e);
    assert_eq!((fin0, latent0[0]), (sans, 0.0));
}

#[test]
fn ventilateurs_locaux_eq_811_812() {
    let e = emetteur(
        r#"{"id": "vc",
            "chauffage": {"type_emetteur": "soufflage_air_chaud",
              "variation_spatiale": {"classe": {"classe": "B2", "hsp_m": 2.5}},
              "variation_temporelle": {"defaut": {"arret_total": true}},
              "effet_joule": false, "rats": 1, "ratt": 1, "distribution": "d"},
            "ventilateurs": {"gestion": "automatique_avec_arret",
              "p_gv_w": 30, "p_mv_w": 20, "p_pv_w": 10,
              "q_recirc_gv_m3_h": 600, "q_recirc_mv_m3_h": 400, "q_recirc_pv_m3_h": 200}}"#,
    );
    let v = e.ventilateurs.unwrap();
    let c = ConditionsVentilateurs {
        aut_ch: true,
        aut_fr: false,
        relance: false,
        i_occ: true,
        i_occ_h1: true,
        idb_ch: true,
        idb_fr: false,
        a_gr_m2: 100.0,
    };
    let r = |q: f64, c: &ConditionsVentilateurs| {
        regime_ventilateurs(
            &e,
            &v,
            (1.0, 0.0),
            (q, 0.0),
            c,
            RegimeVentilateurs::default(),
        )
    };
    // Seuil : 1 × 100 m² × 20 Wh/m² = 2 000 Wh.
    assert_eq!(r(2500.0, &c).w_vent_loc_wh, 20.0);
    assert_eq!(r(1500.0, &c).w_vent_loc_wh, 10.0);
    assert_eq!(
        r(0.0, &ConditionsVentilateurs { idb_ch: false, ..c }).w_vent_loc_wh,
        0.0
    );
    assert_eq!(
        r(0.0, &ConditionsVentilateurs { relance: true, ..c }).q_v_recirc_m3_h,
        600.0
    );
    // Hors saison : arrêt (éq. 811).
    assert_eq!(
        r(2500.0, &ConditionsVentilateurs { aut_ch: false, ..c }),
        RegimeVentilateurs::default()
    );
    let _ = GestionVentilateurs::Manuelle;
}
