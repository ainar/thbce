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
    // Jeu synthétique à 10 °C sans soleil : le besoin de chauffage, donc le Bbio, est positif.
    assert!(r["batiments"][0]["bbio_pts"].as_f64().unwrap() > 0.0);
    // Indicateurs du lot 7 : Cep calculé, synthèse sur la sortie d'erreur.
    assert!(r["batiments"][0]["cep_kwhep_m2"].as_f64().unwrap() > 0.0);
    assert!(r["batiments"][0]["indicateurs"]["cep_nr_kwhep_m2"].is_number());
    // Le jeu « Th-D » synthétique n'a pas de période de confort adaptatif : DH calculé, nul.
    assert_eq!(r["batiments"][0]["dh_c_h"].as_f64(), Some(0.0));
    let groupe = &r["batiments"][0]["zones"][0]["groupes"][0];
    assert_eq!(groupe["confort_ete"]["dh_c_h"].as_f64(), Some(0.0));
    assert!(groupe["confort_ete"]["theta_op_max_c"].as_f64().unwrap() > 15.0);
    let erreurs = String::from_utf8_lossy(&s.stderr);
    assert!(erreurs.contains("non réglementaire"));
    assert!(erreurs.contains("Cep,nr"), "{erreurs}");
    assert!(!erreurs.contains("DH calculé"));
    assert!(!erreurs.contains("générations idéales"));
    // Calcul Th-C : demande de chauffage positive par temps froid, fournie par les panneaux
    // rayonnants, dont la consommation d'électricité est égale à l'énergie fournie (§8.18).
    let generation = &r["batiments"][0]["generations"][0];
    assert!(generation["q_ch_kwh"].as_f64().unwrap() > 0.0);
    assert_eq!(
        generation["consommations"]["annuel"]["chauffage"]["electricite_kwh"],
        generation["q_fou_ch_kwh"]
    );
    assert!(
        r["batiments"][0]["consommations"]["annuel"]["ecs"]["electricite_kwh"]
            .as_f64()
            .unwrap()
            > 0.0
    );

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
    let bureaux = texte.replace("maison_individuelle", "bureaux");
    let bureaux = fichier("bureaux.json", &bureaux);
    let h2b = meteo("h2b-bis.csv", ZoneClimatique::H2b);
    let s = thbce(&["calcul", bureaux.to_str().unwrap(), "--meteo", &h2b]);
    assert_eq!(
        s.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&s.stderr)
    );
    // `valider` sur la même entrée : correcte mais non calculable, code 3 et message explicite.
    let s = thbce(&["valider", bureaux.to_str().unwrap()]);
    assert_eq!(s.status.code(), Some(3));
    let erreur = String::from_utf8_lossy(&s.stderr).into_owned();
    assert!(
        erreur.contains("non pris en charge par cette version (voir docs/couverture.md)"),
        "{erreur}"
    );
    assert!(erreur.contains("usage = bureaux"), "{erreur}");
    // Une erreur de saisie l'emporte sur un élément hors périmètre : code 2.
    let mixte = fichier(
        "mixte.json",
        &texte
            .replace("maison_individuelle", "bureaux")
            .replace("\"shab_m2\": 96.44", "\"shab_m2\": -1"),
    );
    assert_eq!(
        thbce(&["valider", mixte.to_str().unwrap()]).status.code(),
        Some(2)
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

#[test]
fn option_version() {
    for option in ["--version", "-V"] {
        let s = thbce(&[option]);
        assert_eq!(s.status.code(), Some(0));
        let texte = String::from_utf8_lossy(&s.stdout).into_owned();
        assert!(texte.starts_with("thbce "), "{texte}");
        assert!(texte.contains("Th-BCE 2020"), "{texte}");
    }
}

#[test]
fn messages_en_francais() {
    // Option retirée : erreur d'usage, en français.
    let s = thbce(&["calcul", &entree(), "--meteo", "m.csv", "--horaire"]);
    assert_eq!(s.status.code(), Some(1));
    let e = String::from_utf8_lossy(&s.stderr).into_owned();
    assert!(e.contains("argument inattendu : --horaire"), "{e}");
    // Aide en français.
    let s = thbce(&["--help"]);
    let aide = String::from_utf8_lossy(&s.stdout).into_owned();
    assert!(aide.contains("Utilisation : thbce"), "{aide}");
    assert!(aide.contains("Affiche l'aide"), "{aide}");
    // Fichier introuvable : message français, sans « No such file or directory ».
    let s = thbce(&["valider", "inexistant.json"]);
    let e = String::from_utf8_lossy(&s.stderr).into_owned();
    assert!(e.contains("fichier ou dossier introuvable"), "{e}");
    // JSON tronqué : fin de fichier inattendue, sans chemin en « ? ».
    let texte = std::fs::read_to_string(entree()).unwrap();
    let tronque = fichier("tronque.json", &texte[..texte.len() / 2]);
    let s = thbce(&["valider", tronque.to_str().unwrap()]);
    assert_eq!(s.status.code(), Some(2));
    let e = String::from_utf8_lossy(&s.stderr).into_owned();
    assert!(e.contains("fin de fichier inattendue"), "{e}");
    assert!(!e.contains('?'), "{e}");
    // Champ inconnu, une seule position.
    let inconnu = fichier(
        "inconnu.json",
        &texte.replacen('{', "{\"surprise\": 1, ", 1),
    );
    let e = String::from_utf8_lossy(&thbce(&["valider", inconnu.to_str().unwrap()]).stderr)
        .into_owned();
    assert!(e.contains("champ inconnu `surprise`"), "{e}");
    assert_eq!(e.matches("ligne").count(), 1, "{e}");
    // Conversion météo d'un fichier qui n'est pas un classeur.
    let s = thbce(&["meteo", "convertir", &entree(), "-o", "x.csv"]);
    assert_eq!(s.status.code(), Some(2));
    let e = String::from_utf8_lossy(&s.stderr).into_owned();
    assert!(e.contains("attendu : le classeur .xlsx officiel"), "{e}");
}

#[test]
fn altitude_bornee() {
    let texte = std::fs::read_to_string(entree()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&texte).unwrap();
    let mut v = v;
    v["localisation"]["alt_m"] = 4500.into();
    let haut = fichier("haut.json", &v.to_string());
    let s = thbce(&["valider", haut.to_str().unwrap()]);
    assert_eq!(s.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&s.stderr).contains("localisation.alt_m"));
}
