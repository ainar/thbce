// SPDX-License-Identifier: GPL-3.0-or-later
//! Séries horaires restituées sur demande (Q118) : cohérence avec les consommations annuelles,
//! restitution par défaut inchangée, formats CSV et JSON. Météo ouverte ADEME.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::Projet;
use thbce_core::{calculer, calculer_avec_series, Options};
use thbce_io::{horaire, json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cas(nom: &str) -> (Projet, MeteoProjet) {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    let p = json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap();
    let zone = p.localisation.zone_climatique_retenue();
    let m = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    let m = meteo::lire(&std::fs::read(m).unwrap(), zone).unwrap();
    (p, m)
}

const SANS_ARRONDI: Options = Options { arrondir: false };

#[test]
fn restitution_par_defaut_inchangee_et_series_a_la_demande() {
    let (p, m) = cas("commerce-escalators-ademe");
    let defaut = calculer(&p, &m, &SANS_ARRONDI).unwrap();
    assert!(defaut.batiments[0].series_horaires.is_none());
    assert!(!horaire::presentes(&defaut));
    let mut avec = calculer_avec_series(&p, &m, &SANS_ARRONDI).unwrap();
    assert!(horaire::presentes(&avec));
    // Hors séries, les résultats sont identiques.
    avec.batiments[0].series_horaires = None;
    assert_eq!(
        json::ecrire_resultats(&defaut),
        json::ecrire_resultats(&avec)
    );
}

#[test]
fn series_coherentes_avec_les_consommations_annuelles() {
    let (p, m) = cas("commerce-escalators-ademe");
    let r = calculer_avec_series(&p, &m, &SANS_ARRONDI).unwrap();
    let b = &r.batiments[0];
    let s = b.series_horaires.as_ref().unwrap();
    let somme = |nom: &str| -> f64 {
        let c = s.colonnes.iter().find(|c| c.nom == nom).expect(nom);
        assert_eq!(c.valeurs.len(), 8760, "{nom}");
        c.valeurs.iter().sum::<f64>() / 1000.0
    };
    let indic = b.indicateurs.as_ref().unwrap();
    let e = &indic.electricite_consommee;
    // Le forfait de refroidissement (§13.3.3.2, éq. 2494 à 2496) est annuel : il est compris dans
    // l'indicateur, pas dans la série horaire.
    let forfait: f64 = indic
        .forfaits_refroidissement
        .iter()
        .map(|f| f.cef_kwh)
        .sum();
    for (nom, kwh) in [
        ("elec.ch", e.chauffage_kwh),
        ("elec.fr", e.refroidissement_kwh - forfait),
        ("elec.ecs", e.ecs_kwh),
        ("elec.ecl", e.eclairage_kwh),
        ("elec.aux_vent", e.auxiliaires_ventilation_kwh),
        ("elec.aux_dist", e.auxiliaires_distribution_kwh),
        ("elec.depl", e.deplacements_kwh),
        ("elec.mob", e.mobilier_kwh),
    ] {
        assert!(
            (somme(nom) - kwh).abs() < 1e-6 * kwh.abs().max(1.0),
            "{nom}"
        );
    }
    // Le total est la somme des postes.
    let postes = e.chauffage_kwh
        + e.refroidissement_kwh
        + e.ecs_kwh
        + e.eclairage_kwh
        + e.auxiliaires_ventilation_kwh
        + e.auxiliaires_distribution_kwh
        + e.deplacements_kwh
        + e.mobilier_kwh
        - forfait;
    assert!((somme("elec.total") - postes).abs() < 1e-6 * postes);
    // Température extérieure du site : en °C, d'une moyenne annuelle plausible.
    let te = &s.colonnes[0];
    assert_eq!(te.nom, "te");
    assert_eq!(te.unite, "C");
    let moyenne = te.valeurs.iter().sum::<f64>() / 8760.0;
    assert!((5.0..20.0).contains(&moyenne), "{moyenne}");
}

#[test]
fn csv_et_json() {
    let (p, m) = cas("commerce-escalators-ademe");
    let r = calculer_avec_series(&p, &m, &Options::default()).unwrap();
    let csv = horaire::en_csv(&r);
    let lignes: Vec<&str> = csv.lines().collect();
    assert_eq!(lignes.len(), 8761);
    assert!(lignes[0].starts_with("heure,commerce.te [C],commerce."));
    assert!(lignes[1].starts_with("1,"));
    assert!(lignes[8760].starts_with("8760,"));
    let nb = lignes[0].split(',').count();
    assert!(lignes.iter().all(|l| l.split(',').count() == nb));
    let j: serde_json::Value = serde_json::from_str(&horaire::en_json(&r)).unwrap();
    assert_eq!(j["batiments"][0]["id"], "commerce");
    assert_eq!(
        j["batiments"][0]["colonnes"][0]["valeurs"]
            .as_array()
            .unwrap()
            .len(),
        8760
    );
}
