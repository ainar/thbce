// SPDX-License-Identifier: GPL-3.0-or-later
//! Lot 1-env : gestion automatique des protections mobiles (Th-BCE 2020 §5.9.3, éq. 179 à 205,
//! figures 31 et 44 à 47) et stores vénitiens (§5.9, §5.10.3), de bout en bout sur les
//! exemples `tests/cas/bureaux-volets-auto-ademe` et `tests/cas/bureaux-stores-venitiens-ademe`
//! (variantes du cas `bureaux-simple-flux-ademe`), météo ouverte ADEME.
//!
//! Valeurs phares sur la météo réglementaire :
//! THBCE_METEO=/chemin/vers/meteo.csv cargo t-regl enveloppe_pm_sur_meteo_reglementaire

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{GestionPm, Projet};
use thbce_core::resultats::Resultats;
use thbce_core::{calculer, valider, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cas(nom: &str) -> Projet {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn meteo_ademe(p: &Projet) -> MeteoProjet {
    let zone = p.localisation.zone_climatique_retenue();
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

fn meteo_reglementaire(p: &Projet) -> MeteoProjet {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO pour lancer ce test");
    let zone = p.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

/// Bbio, DH, Cep, Cep,nr, Ic énergie.
fn phares(r: &Resultats) -> [f64; 5] {
    let b = &r.batiments[0];
    [
        b.bbio_pts.unwrap(),
        b.dh_c_h.unwrap(),
        b.cep_kwhep_m2.unwrap(),
        b.cep_nr_kwhep_m2.unwrap(),
        b.ic_energie_kgco2_m2.unwrap(),
    ]
}

fn calcul(p: &Projet, m: &MeteoProjet) -> Resultats {
    calculer(p, m, &Options { arrondir: false }).unwrap()
}

#[test]
fn les_exemples_sont_valides_et_se_calculent() {
    for nom in [
        "bureaux-volets-auto-ademe",
        "bureaux-stores-venitiens-ademe",
    ] {
        let p = cas(nom);
        let rapport = valider(&p);
        assert!(rapport.est_valide(), "{nom} : {:?}", rapport.erreurs);
        let r = calcul(&p, &meteo_ademe(&p));
        let [bbio, dh, ..] = phares(&r);
        assert!(bbio.is_finite() && dh.is_finite(), "{nom}");
    }
}

#[test]
fn sens_des_effets_de_la_gestion_des_volets() {
    // Q72 : sur l'exemple de bureaux (ADEME), la motorisation d'un volet manuel relève la
    // DH ; la gestion automatique, qui ferme les volets quand l'éclairement ou la
    // température opérative sont élevés, la fait baisser nettement.
    let base = cas("bureaux-simple-flux-ademe");
    let m = meteo_ademe(&base);
    let motorise = phares(&calcul(&base, &m))[1];
    let mut manuel = base.clone();
    manuel.catalogue.baies[0]
        .protection_mobile
        .as_mut()
        .unwrap()
        .gestion = Some(GestionPm::ManuelleNonMotorisee);
    let non_motorise = phares(&calcul(&manuel, &m))[1];
    let auto = phares(&calcul(&cas("bureaux-volets-auto-ademe"), &m))[1];
    println!(
        "DH : non motorisé {non_motorise:.1} ; motorisé {motorise:.1} ; automatique {auto:.1}"
    );
    assert!(non_motorise < motorise);
    assert!(auto < non_motorise);
}

#[test]
fn les_matrices_conventionnelles_sont_signalees() {
    let p = cas("bureaux-volets-auto-ademe");
    let r = calcul(&p, &meteo_ademe(&p));
    let baie = &r.batiments[0].zones[0].groupes[0].baies[0];
    assert!(
        baie.valeurs_par_defaut
            .iter()
            .any(|v| v.champ == "protection_mobile.automatisme.matrice"),
        "{:?}",
        baie.valeurs_par_defaut
    );
}

/// Valeurs phares des deux exemples sur la météo réglementaire (H2b) : nombres annuels seulement.
#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn enveloppe_pm_sur_meteo_reglementaire() {
    let proche = |a: f64, b: f64| (a - b).abs() <= 0.05 + 1e-9;
    for (nom, attendu, non_respectees) in [
        (
            "bureaux-volets-auto-ademe",
            [78.5, 510.1, 61.0, 61.0, 314.0],
            vec!["ic_energie"],
        ),
        (
            "bureaux-stores-venitiens-ademe",
            [95.8, 988.8, 65.3, 65.3, 318.7],
            vec!["bbio", "ic_energie"],
        ),
    ] {
        let p = cas(nom);
        let r = calcul(&p, &meteo_reglementaire(&p));
        assert!(r.reglementaire);
        let v = phares(&r);
        println!(
            "{nom} : Bbio {:.1} ; DH {:.1} ; Cep {:.1} ; Cep,nr {:.1} ; Ic {:.1}",
            v[0], v[1], v[2], v[3], v[4]
        );
        for (a, b) in v.iter().zip(attendu) {
            assert!(proche(*a, b), "{nom} : {a} au lieu de {b}");
        }
        let ex = r.batiments[0]
            .indicateurs
            .as_ref()
            .unwrap()
            .exigences
            .as_ref()
            .unwrap();
        let non: Vec<&str> = ex
            .verifications
            .iter()
            .filter(|v| !v.respectee)
            .map(|v| v.indicateur.as_str())
            .collect();
        assert_eq!(non, non_respectees, "{nom}");
        assert!(!ex.conforme);
    }
}
