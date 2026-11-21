// SPDX-License-Identifier: GPL-3.0-or-later
//! DRV et thermo-frigo-pompes (fiche 8.16, lot 2f) de bout en bout : exemple
//! `bureaux-drv-ademe` (groupes extérieurs DRV d'un immeuble de bureaux) sur la météo ouverte
//! ADEME (CI publique) et sur la météo réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo.csv cargo test -p thbce-io --test drv -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les tests vérifient un pas de temps à la main
//! (récupération du rejet, éq. 978 et 979), l'égalité avec une pompe à chaleur réversible
//! quand la génération n'a jamais à la fois une demande de chaud et de froid, et les refus de
//! la validation.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::chauffage_refroidissement::calculs_generation::{Energie, Poste};
use thbce_core::chauffage_refroidissement::generation::{
    ConditionsSource, EntreesGeneration, GenerationHoraire, ReseauVu,
};
use thbce_core::climat::MeteoProjet;
use thbce_core::entree::Projet;
use thbce_core::resultats::Resultats;
use thbce_core::{calculer, Options};
use thbce_io::{json, meteo};

const EXEMPLE: &str = "bureaux-drv-ademe";

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json_du_cas(nom: &str) -> serde_json::Value {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    serde_json::from_str(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn projet(v: &serde_json::Value) -> Projet {
    json::lire_projet(&v.to_string()).unwrap()
}

fn meteo_ademe(p: &Projet) -> MeteoProjet {
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    meteo::lire(
        &std::fs::read(chemin).unwrap(),
        p.localisation.zone_climatique_retenue(),
    )
    .unwrap()
}

fn calcul(v: &serde_json::Value) -> Resultats {
    let p = projet(v);
    let r = thbce_core::valider(&p);
    assert!(r.est_valide(), "{:?}", r.erreurs);
    calculer(&p, &meteo_ademe(&p), &Options { arrondir: false }).unwrap()
}

/// Remplace la nature `drv` (ou `thermofrigopompe`) du premier générateur de la première
/// génération par `autre`, mêmes données.
fn changer_nature(v: &mut serde_json::Value, de: &str, vers: &str) {
    let n = &mut v["batiments"][0]["generations"][0]["generateurs"][0]["nature"];
    let t = n[de].take();
    *n = serde_json::json!({ vers: t });
}

fn reseau(poste: Poste, q_sys_wh: f64, theta_c: f64) -> ReseauVu {
    ReseauVu {
        poste,
        hydraulique: false,
        q_sys_wh,
        theta_moy_c: theta_c,
        theta_dep_c: theta_c,
        theta_i_aval_eq_c: theta_c,
        relance: false,
        rat_surf_dess: 1.0,
        phi_pertes_prim_wh: 0.0,
    }
}

fn pas(gen: &mut GenerationHoraire, q_ch_wh: f64, q_fr_wh: f64) {
    let reseaux = [
        reseau(Poste::Chauffage, q_ch_wh, 20.0),
        reseau(Poste::Refroidissement, q_fr_wh, 26.0),
    ];
    let sources = [ConditionsSource {
        theta_amont_c: 15.0,
        air_extrait_w_k: None,
    }];
    gen.pas(
        &EntreesGeneration {
            reseaux: &reseaux,
            aut_ch: true,
            aut_fr: true,
            id_encl: true,
            theta_ext_c: 15.0,
            b_therm: 1.0,
            w_rechauf_wh: 0.0,
            theta_cw_c: 12.0,
            hleg: 12,
            sources: &sources,
            climat: None,
            h: 2400,
            w_ext_kg_kg: 0.005,
            demandes_groupes: &[],
        },
        3,
    );
}

/// Un pas de temps avec une demande de froid de 8 kWh et une demande de chaud : le DRV est
/// d'abord appelé en refroidissement, puis son rejet exploitable `Qfou + 0,6·Qcons` (éq. 978)
/// assure le chauffage s'il suffit, sinon le DRV passe en chauffage (éq. 979).
#[test]
fn recuperation_du_rejet_a_la_main() {
    let p = projet(&json_du_cas(EXEMPLE));
    let base = GenerationHoraire::resoudre(&p.batiments[0].generations[0], &[], &[], &p.catalogue);
    assert!(base.generateurs[0].transfert_locaux);

    // Référence : le froid seul, pour connaître `Qcons` de l'appel en refroidissement.
    let mut gen = base.clone();
    pas(&mut gen, 0.0, 8000.0);
    let j = gen.generateurs[0].jeu;
    let (q_cons, phi_fr) = (j.q_cons_wh, j.phi_rejet_wh);
    assert!((j.q_fou_wh[1] - 8000.0).abs() < 1e-6 && q_cons > 0.0);
    let phi_exp = 8000.0 + 0.6 * q_cons;
    let cef_fr = j.q_cef.valeur(Poste::Refroidissement, Energie::Electricite);

    // 1. Le rejet suffit (5 kWh ≤ Φrejet_expCH) : chauffage fourni sans consommation
    // supplémentaire ; rejet diminué d'autant.
    let mut gen = base.clone();
    pas(&mut gen, 5000.0, 8000.0);
    let g = &gen.generateurs[0];
    assert!(5000.0 <= phi_exp);
    assert_eq!(g.jeu.q_fou_wh[0], 5000.0);
    assert_eq!(g.jeu.q_fou_ch_recup_wh, 5000.0);
    assert!((g.jeu.q_fou_wh[1] - 8000.0).abs() < 1e-6);
    assert_eq!(g.jeu.q_cef.total_poste(Poste::Chauffage), 0.0);
    assert_eq!(
        g.jeu
            .q_cef
            .valeur(Poste::Refroidissement, Energie::Electricite),
        cef_fr
    );
    assert!((g.jeu.phi_rejet_wh - (phi_fr - 5000.0)).abs() < 1e-9);
    assert_eq!(gen.sorties.q_rep_wh, [0.0, 0.0, 0.0]);
    assert_eq!(gen.sorties.q_fou_wh[0], 5000.0);

    // 2. Le rejet ne suffit pas : sorties du froid effacées, l'énergie fournie en froid
    // conservée ; appel en chauffage de toute la demande.
    let q_ch = phi_exp + 1000.0;
    let mut gen = base.clone();
    pas(&mut gen, q_ch, 8000.0);
    let g = &gen.generateurs[0];
    assert!((g.jeu.q_fou_wh[1] - 8000.0).abs() < 1e-6);
    assert!((g.jeu.q_fou_wh[0] - q_ch).abs() < 1e-6);
    assert_eq!(g.jeu.q_fou_ch_recup_wh, 0.0);
    assert_eq!(g.jeu.q_cef.total_poste(Poste::Refroidissement), 0.0);
    assert!(g.jeu.q_cef.total_poste(Poste::Chauffage) > 0.0);
    assert!(!g.jeu.ia_refroidi);
    // Même consommation que le chauffage seul de la même demande ; rejet corrigé par le froid
    // fourni (lecture Q243 point 2).
    let mut seul = base.clone();
    pas(&mut seul, q_ch, 0.0);
    let s = &seul.generateurs[0].jeu;
    assert_eq!(g.jeu.q_cef, s.q_cef);
    assert!((g.jeu.phi_rejet_wh - (s.phi_rejet_wh + 8000.0)).abs() < 1e-9);

    // 3. Sans demande de froid, appel en chauffage ordinaire.
    let mut gen = base.clone();
    pas(&mut gen, 3000.0, 0.0);
    assert!((gen.generateurs[0].jeu.q_fou_wh[0] - 3000.0).abs() < 1e-6);
    assert_eq!(gen.generateurs[0].jeu.q_fou_ch_recup_wh, 0.0);
}

/// Les deux postes ne sont jamais demandés à la même heure dans l'exemple (un seul groupe) :
/// le DRV se comporte comme la pompe à chaleur réversible de mêmes données.
#[test]
fn bureaux_drv_comme_une_pac_reversible() {
    let v = json_du_cas(EXEMPLE);
    let r = calcul(&v);
    let mut pac = v.clone();
    changer_nature(&mut pac, "drv", "thermodynamique_electrique");
    let rp = calcul(&pac);
    let (a, b) = (&r.batiments[0], &rp.batiments[0]);
    assert_eq!(a.cep_kwhep_m2, b.cep_kwhep_m2);
    assert_eq!(a.ic_energie_kgco2_m2, b.ic_energie_kgco2_m2);
    let ga = &a.generations[0].generateurs[0];
    let gb = &b.generations[0].generateurs[0];
    assert_eq!(ga.consommations, gb.consommations);
    assert_eq!(ga.q_fou_ch_recup_kwh, Some(0.0));
    assert_eq!(gb.q_fou_ch_recup_kwh, None);
    assert!(ga.q_fou_ch_kwh > 0.0 && ga.q_fou_fr_kwh > 0.0);
}

/// Thermo-frigo-pompe (type 508) : machine eau / eau réversible ; sur l'hôtel d'exemple, qui
/// n'a que des réseaux de chauffage, elle se comporte comme la pompe à chaleur réversible de
/// mêmes données.
#[test]
fn thermofrigopompe_sur_l_hotel() {
    let mut v = json_du_cas("hotel-pac-ademe");
    let pac = &mut v["batiments"][0]["generations"][0]["generateurs"][0]["nature"]
        ["thermodynamique_electrique"];
    pac["refroidissement"] = serde_json::json!({
        "technologie": "air_exterieur_eau",
        "performances": {"pivot_defaut": {"pabs_kw": 1.4}},
        "typo_emetteur": "plancher_ou_plafond_inertie_forte"
    });
    let r = calcul(&v);
    let mut tfp = v.clone();
    changer_nature(&mut tfp, "thermodynamique_electrique", "thermofrigopompe");
    let rt = calcul(&tfp);
    assert_eq!(r.batiments[0].cep_kwhep_m2, rt.batiments[0].cep_kwhep_m2);
    assert_eq!(
        rt.batiments[0].generations[0].generateurs[0].q_fou_ch_recup_kwh,
        Some(0.0)
    );
}

#[test]
fn validation_des_drv_et_thermofrigopompes() {
    let v = json_du_cas(EXEMPLE);
    let messages = |v: &serde_json::Value| -> Vec<String> {
        thbce_core::valider(&projet(v))
            .erreurs
            .iter()
            .map(|e| format!("{} : {}", e.chemin, e.message))
            .collect()
    };
    assert!(messages(&v).is_empty(), "{:?}", messages(&v));
    let drv = |v: &mut serde_json::Value| -> serde_json::Value {
        v["batiments"][0]["generations"][0]["generateurs"][0]["nature"]["drv"].clone()
    };
    let poser = |v: &mut serde_json::Value, d: serde_json::Value| {
        v["batiments"][0]["generations"][0]["generateurs"][0]["nature"]["drv"] = d;
    };
    // Réversible : les deux modes sont requis.
    let mut x = v.clone();
    let mut d = drv(&mut x);
    d.as_object_mut().unwrap().remove("refroidissement");
    poser(&mut x, d);
    assert!(messages(&x).iter().any(|m| m.contains("réversible")));
    // Fluide aval : air recyclé pour un DRV.
    let mut x = v.clone();
    let mut d = drv(&mut x);
    d["chauffage"]["technologie"] = "air_exterieur_eau".into();
    poser(&mut x, d);
    assert!(messages(&x).iter().any(|m| m.contains("air recyclé")));
    // Eau pour une thermo-frigo-pompe.
    let mut x = v.clone();
    changer_nature(&mut x, "drv", "thermofrigopompe");
    assert!(messages(&x)
        .iter()
        .any(|m| m.contains("« machine eau / eau »")));
    // Pas d'ECS.
    let mut x = v.clone();
    let mut d = drv(&mut x);
    d["ecs"] = serde_json::json!({
        "technologie": "air_exterieur_eau",
        "performances": {"pivot_defaut": {"pabs_kw": 1.0}}
    });
    poser(&mut x, d);
    assert!(messages(&x)
        .iter()
        .any(|m| m.contains("ne produit pas d'ECS")));
}

#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn drv_sur_meteo_reglementaire() {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO");
    let octets = std::fs::read(chemin).unwrap();
    let p = projet(&json_du_cas(EXEMPLE));
    let m = meteo::lire(&octets, p.localisation.zone_climatique_retenue()).unwrap();
    let r = calculer(&p, &m, &Options { arrondir: false }).unwrap();
    let b = &r.batiments[0];
    let i = b.indicateurs.as_ref().unwrap();
    println!(
        "{EXEMPLE} : Bbio {:.1}, DH {:.1}, Cep {:.1}, Cep,nr {:.1}, Ic énergie {:.1}",
        b.bbio_pts.unwrap(),
        b.dh_c_h.unwrap(),
        i.cep_kwhep_m2,
        i.cep_nr_kwhep_m2,
        i.ic_energie_kgco2_m2.unwrap(),
    );
    let d = |x: f64| (x * 10.0).round() / 10.0;
    assert_eq!(
        (
            d(b.bbio_pts.unwrap()),
            d(b.dh_c_h.unwrap()),
            d(i.cep_kwhep_m2),
            d(i.ic_energie_kgco2_m2.unwrap()),
        ),
        // Valeurs phares au lot 2f (arrondies à une décimale).
        (82.9, 688.7, 71.8, 87.5)
    );
}
