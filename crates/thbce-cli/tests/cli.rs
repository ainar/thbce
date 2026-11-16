// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests de bout en bout de l'exécutable : commandes et codes de retour.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use thbce_core::climat::donnees::DESCRIPTION_SYNTHETIQUE;
use thbce_core::climat::MeteoProjet;
use thbce_data::ZoneClimatique;

fn thbce(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_thbce"))
        .args(args)
        .output()
        .expect("lancement de thbce")
}

fn entree() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/cas/squelette-maison/entree.json")
        .to_str()
        .unwrap()
        .to_owned()
}

/// Écrit un fichier dans un dossier temporaire propre au test.
fn fichier(nom: &str, contenu: &str) -> PathBuf {
    let dossier = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli");
    std::fs::create_dir_all(&dossier).unwrap();
    let chemin = dossier.join(nom);
    std::fs::write(&chemin, contenu).unwrap();
    chemin
}

fn meteo(nom: &str, zone: ZoneClimatique) -> String {
    let csv =
        thbce_io::meteo::ecrire_csv(&[MeteoProjet::synthetique(zone)], DESCRIPTION_SYNTHETIQUE);
    fichier(nom, &csv).to_str().unwrap().to_owned()
}

#[test]
fn calcul_ecrit_les_resultats_en_json() {
    let s = thbce(&[
        "calcul",
        &entree(),
        "--meteo",
        &meteo("h2b.csv", ZoneClimatique::H2b),
    ]);
    assert_eq!(
        s.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&s.stderr)
    );
    let r: serde_json::Value = serde_json::from_slice(&s.stdout).unwrap();
    assert_eq!(r["batiments"][0]["sref_m2"], 96.4);
    assert_eq!(r["reglementaire"], false);
    assert_eq!(r["batiments"][0]["bbio_pts"], serde_json::Value::Null);
    assert!(String::from_utf8_lossy(&s.stderr).contains("non réglementaire"));

    let s = thbce(&[
        "calcul",
        &entree(),
        "--meteo",
        &meteo("h2b.csv", ZoneClimatique::H2b),
        "--sans-arrondi",
    ]);
    let r: serde_json::Value = serde_json::from_slice(&s.stdout).unwrap();
    assert_eq!(r["batiments"][0]["sref_m2"], 96.44);
}

#[test]
fn la_meteo_est_obligatoire() {
    let s = thbce(&["calcul", &entree()]);
    assert_eq!(s.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&s.stderr).contains("--meteo"));
}

#[test]
fn codes_de_retour() {
    // 1 : fichier introuvable.
    assert_eq!(
        thbce(&["valider", "inexistant.json"]).status.code(),
        Some(1)
    );
    // 2 : entrée illisible, entrée invalide, météo d'une autre zone.
    let illisible = fichier("illisible.json", "{");
    assert_eq!(
        thbce(&["valider", illisible.to_str().unwrap()])
            .status
            .code(),
        Some(2)
    );
    let texte = std::fs::read_to_string(entree()).unwrap();
    let invalide = fichier(
        "invalide.json",
        &texte.replace("\"shab_m2\": 96.44", "\"shab_m2\": -1"),
    );
    let s = thbce(&["valider", invalide.to_str().unwrap()]);
    assert_eq!(s.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&s.stderr).contains("groupes[0].shab_m2 = -1"));
    let h1a = meteo("h1a.csv", ZoneClimatique::H1a);
    assert_eq!(
        thbce(&["calcul", &entree(), "--meteo", &h1a]).status.code(),
        Some(2)
    );
    // 3 : usage hors périmètre de cette version.
    let bureaux = texte
        .replace("maison_individuelle", "bureaux")
        .replace("shab_m2", "su_m2");
    let bureaux = fichier("bureaux.json", &bureaux);
    let h2b = meteo("h2b-bis.csv", ZoneClimatique::H2b);
    let s = thbce(&["calcul", bureaux.to_str().unwrap(), "--meteo", &h2b]);
    assert_eq!(
        s.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&s.stderr)
    );
    // 0 : validation réussie.
    assert_eq!(thbce(&["valider", &entree()]).status.code(), Some(0));
}

#[test]
fn schema_et_version() {
    let s = thbce(&["schema"]);
    assert_eq!(s.status.code(), Some(0));
    let schema: serde_json::Value = serde_json::from_slice(&s.stdout).unwrap();
    assert_eq!(schema["title"], "Projet");
    let s = thbce(&["version"]);
    let texte = String::from_utf8_lossy(&s.stdout).into_owned();
    assert!(texte.starts_with("thbce "), "{texte}");
    assert!(texte.contains("Th-BCE 2020"), "{texte}");
}
