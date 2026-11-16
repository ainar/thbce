// SPDX-License-Identifier: GPL-3.0-or-later
//! Harnais des cas de validation : parcourt `tests/cas/`, calcule chaque entrée et compare aux
//! résultats attendus avec les tolérances du cas. Format décrit dans `tests/cas/README.md`.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use serde_json::Value;
use thbce_core::climat::MeteoProjet;
use thbce_core::{calculer, Options};
use thbce_io::{json, meteo};

/// Tolérance relative par défaut : 0,5 %.
const TOLERANCE_DEFAUT: f64 = 0.005;

fn dossier_des_cas() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/cas")
}

struct Ecart {
    chemin: String,
    attendu: String,
    obtenu: String,
    detail: String,
}

fn tolerance(tolerances: Option<&toml::Table>, champ: &str) -> f64 {
    let lire = |cle: &str| {
        tolerances
            .and_then(|t| t.get(cle))
            .and_then(toml::Value::as_float)
    };
    lire(champ)
        .or_else(|| lire("defaut"))
        .unwrap_or(TOLERANCE_DEFAUT)
}

/// Compare récursivement : seules les valeurs présentes dans `attendu` sont vérifiées.
fn comparer(
    chemin: &str,
    champ: &str,
    attendu: &Value,
    obtenu: &Value,
    tol: Option<&toml::Table>,
    ecarts: &mut Vec<Ecart>,
) {
    let mut ecart = |detail: String| {
        ecarts.push(Ecart {
            chemin: chemin.to_owned(),
            attendu: attendu.to_string(),
            obtenu: obtenu.to_string(),
            detail,
        });
    };
    match (attendu, obtenu) {
        (Value::Object(a), Value::Object(o)) => {
            for (cle, valeur) in a {
                let sous_chemin = if chemin.is_empty() {
                    cle.clone()
                } else {
                    format!("{chemin}.{cle}")
                };
                match o.get(cle) {
                    Some(v) => comparer(&sous_chemin, cle, valeur, v, tol, ecarts),
                    None => ecarts.push(Ecart {
                        chemin: sous_chemin,
                        attendu: valeur.to_string(),
                        obtenu: "(absent)".into(),
                        detail: "champ absent des résultats".into(),
                    }),
                }
            }
        }
        (Value::Array(a), Value::Array(o)) => {
            if a.len() != o.len() {
                ecart(format!(
                    "{} éléments attendus, {} obtenus",
                    a.len(),
                    o.len()
                ));
                return;
            }
            for (i, (va, vo)) in a.iter().zip(o).enumerate() {
                comparer(&format!("{chemin}[{i}]"), champ, va, vo, tol, ecarts);
            }
        }
        (Value::Number(a), Value::Number(o)) => {
            let (a, o) = (
                a.as_f64().unwrap_or(f64::NAN),
                o.as_f64().unwrap_or(f64::NAN),
            );
            let t = tolerance(tol, champ);
            // Écart relatif à la valeur attendue ; absolu si elle est nulle.
            let e = if a == 0.0 {
                (o - a).abs()
            } else {
                ((o - a) / a).abs()
            };
            if e.is_nan() || e > t {
                ecart(format!(
                    "écart {:.3} % > tolérance {:.3} %",
                    e * 100.0,
                    t * 100.0
                ));
            }
        }
        (a, o) if a == o => {}
        _ => ecart("valeurs différentes".into()),
    }
}

/// Jeu météo d'un cas ; `None` si le cas demande la météo réglementaire et qu'elle est absente.
fn meteo_du_cas(nature: &str, projet: &thbce_core::entree::Projet) -> Option<MeteoProjet> {
    let zone = projet.localisation.zone_climatique_retenue();
    match nature {
        "synthetique" => Some(MeteoProjet::synthetique(zone)),
        "re2020" => {
            let chemin = std::env::var_os("THBCE_METEO")?;
            let octets = std::fs::read(&chemin).unwrap_or_else(|e| panic!("THBCE_METEO : {e}"));
            Some(meteo::lire(&octets, zone).unwrap_or_else(|e| panic!("THBCE_METEO : {e}")))
        }
        "ademe" => {
            let chemin = dossier_des_cas()
                .join("../../docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
            let octets =
                std::fs::read(&chemin).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()));
            Some(meteo::lire(&octets, zone).unwrap_or_else(|e| panic!("fichier ADEME : {e}")))
        }
        autre => panic!(
            "cas.toml : meteo = \"{autre}\" inconnu (\"re2020\", \"ademe\" ou \"synthetique\")"
        ),
    }
}

#[test]
fn cas_de_validation() {
    let mut dossiers: Vec<PathBuf> = std::fs::read_dir(dossier_des_cas())
        .expect("dossier tests/cas")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("entree.json").is_file())
        .collect();
    dossiers.sort();
    assert!(!dossiers.is_empty(), "aucun cas dans tests/cas");

    let mut echecs = Vec::new();
    for dossier in &dossiers {
        let nom = dossier
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_owned();
        let lire = |f: &str| {
            std::fs::read_to_string(dossier.join(f)).unwrap_or_else(|e| panic!("{nom}/{f} : {e}"))
        };
        let cas: toml::Table = lire("cas.toml")
            .parse()
            .unwrap_or_else(|e| panic!("{nom}/cas.toml : {e}"));
        let projet = json::lire_projet(&lire("entree.json"))
            .unwrap_or_else(|e| panic!("{nom}/entree.json : {e}"));
        let attendu: Value = serde_json::from_str(&lire("attendu.json"))
            .unwrap_or_else(|e| panic!("{nom}/attendu.json : {e}"));
        let nature = cas
            .get("meteo")
            .and_then(toml::Value::as_str)
            .unwrap_or("re2020");
        let Some(donnees) = meteo_du_cas(nature, &projet) else {
            println!("{nom} : IGNORÉ (météo réglementaire requise, THBCE_METEO non défini)");
            continue;
        };
        let resultats = match calculer(&projet, &donnees, &Options::default()) {
            Ok(r) => r,
            Err(e) => {
                echecs.push(format!("{nom} : le calcul a échoué : {e}"));
                continue;
            }
        };
        let obtenu = serde_json::to_value(&resultats).expect("résultats sérialisables");
        let tolerances = cas.get("tolerances").and_then(toml::Value::as_table);
        let mut ecarts = Vec::new();
        comparer("", "", &attendu, &obtenu, tolerances, &mut ecarts);
        if ecarts.is_empty() {
            println!("{nom} : conforme");
        } else {
            let mut texte = format!("{nom} : {} écart(s)\n", ecarts.len());
            for e in &ecarts {
                texte.push_str(&format!(
                    "  {:<50} attendu {:>12}  obtenu {:>12}  {}\n",
                    e.chemin, e.attendu, e.obtenu, e.detail
                ));
            }
            echecs.push(texte);
        }
    }
    assert!(echecs.is_empty(), "\n{}", echecs.join("\n"));
}

#[test]
fn le_harnais_detecte_un_ecart() {
    let attendu: Value =
        serde_json::json!({"a": {"bbio_pts": 100.0, "x": [1.0, 2.0]}, "nom": "m", "absent": 1});
    let obtenu: Value =
        serde_json::json!({"a": {"bbio_pts": 100.4, "x": [1.0, 2.5]}, "nom": "m", "autre": 3});
    let mut ecarts = Vec::new();
    comparer("", "", &attendu, &obtenu, None, &mut ecarts);
    let chemins: Vec<&str> = ecarts.iter().map(|e| e.chemin.as_str()).collect();
    // 0,4 % d'écart sur bbio_pts passe à 0,5 % de tolérance ; x[1] et le champ absent échouent.
    assert_eq!(chemins, ["a.x[1]", "absent"]);

    let serre: toml::Table = "bbio_pts = 0.001".parse().unwrap();
    let mut ecarts = Vec::new();
    comparer("", "", &attendu, &obtenu, Some(&serre), &mut ecarts);
    assert_eq!(ecarts.len(), 3);
}
