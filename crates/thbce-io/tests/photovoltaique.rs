// SPDX-License-Identifier: GPL-3.0-or-later
//! Production photovoltaïque (lot 7a) de bout en bout : maison d'exemple avec 3 kWc en toiture
//! sud. Météo ouverte ADEME (CI publique) ; météo réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test photovoltaique -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence : propriétés (bilans, ordres de grandeur, effet de l'orientation).

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::Projet;
use thbce_core::{calculer, series_pv, Options};
use thbce_io::{json, meteo};

fn projet(azimut: f64, inclinaison: f64) -> Projet {
    let chemin =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/cas/maison-pv-ademe/entree.json");
    let mut p = json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap();
    let e = &mut p.batiments[0].installations_pv[0].exposition;
    e.azimut_deg = azimut;
    e.inclinaison_deg = inclinaison;
    p
}

fn meteo_ademe(p: &Projet) -> MeteoProjet {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    meteo::lire(
        &std::fs::read(chemin).unwrap(),
        p.localisation.zone_climatique_retenue(),
    )
    .unwrap()
}

fn production(p: &Projet, m: &MeteoProjet) -> f64 {
    let r = calculer(p, m, &Options { arrondir: false }).unwrap();
    r.batiments[0].production_pv.as_ref().unwrap().e_prod_pv_kwh
}

fn verifier(p: &Projet, m: &MeteoProjet) -> f64 {
    let r = calculer(p, m, &Options { arrondir: false }).unwrap();
    let pv = r.batiments[0].production_pv.as_ref().unwrap();
    let mois: f64 = pv.e_prod_pv_mensuel_kwh.iter().sum();
    assert!((mois - pv.e_prod_pv_kwh).abs() < 1e-6);
    // La série horaire livrée au lot 7 redonne la production annuelle.
    let s = series_pv(p, m).unwrap();
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].e_prod_pv_wh.len(), 8760);
    let annuel: f64 = s[0].e_prod_pv_wh.iter().sum::<f64>() / 1000.0;
    assert!((annuel - pv.e_prod_pv_kwh).abs() < 1e-6);
    assert!(s[0].e_prod_pv_wh.iter().all(|x| *x >= 0.0));
    // Aucune production la nuit de minuit (1er janvier, h = 0).
    assert_eq!(s[0].e_prod_pv_wh[0], 0.0);
    pv.e_prod_pv_kwh
}

#[test]
fn bilans_et_orientation_meteo_ademe() {
    let p = projet(0.0, 30.0);
    let m = meteo_ademe(&p);
    let sud = verifier(&p, &m);
    // 3 kWc : entre 500 et 1 500 kWh/kWc, soit 1 500 à 4 500 kWh.
    assert!((1500.0..4500.0).contains(&sud), "{sud}");
    let nord = production(&projet(180.0, 30.0), &m);
    let vertical_sud = production(&projet(0.0, 90.0), &m);
    assert!(nord < sud && vertical_sud < sud);
}

#[test]
#[ignore = "demande la météo réglementaire (THBCE_METEO)"]
fn ordre_de_grandeur_meteo_reglementaire() {
    let p = projet(0.0, 30.0);
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO");
    let m = meteo::lire(
        &std::fs::read(chemin).unwrap(),
        p.localisation.zone_climatique_retenue(),
    )
    .unwrap();
    let e = verifier(&p, &m);
    println!("3 kWc sud 30° en H2b : {e:.0} kWh");
    // H2b, 3 kWc certifiés : de l'ordre de 900 à 1 000 kWh/kWc après onduleur à 0,9.
    assert!((2500.0..3200.0).contains(&e), "{e}");
}
