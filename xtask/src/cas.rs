// SPDX-License-Identifier: GPL-3.0-or-later
//! Calcul d'un ou de plusieurs exemples de `tests/cas/` et comparaison à leur `attendu.json`,
//! pour la boucle de développement (`cargo t-cas <nom>`, voir `docs/architecture.md`).
//!
//! ```text
//! cargo run --profile rapide -p xtask -- cas [<nom>…]
//! ```
//!
//! Sans nom, tous les exemples. Les règles sont celles du harnais
//! `crates/thbce-io/tests/cas.rs` (format dans `tests/cas/README.md`) : seules les valeurs
//! présentes dans `attendu.json` sont comparées, avec la tolérance relative du cas (champ, puis
//! `defaut`, puis 0,5 %), absolue pour une valeur attendue nulle ; les résultats sont arrondis
//! comme à la restitution. **Le harnais fait foi** : cette commande en est une copie pour
//! lancer un seul exemple, et toute évolution des règles du harnais doit y être reportée. Un
//! exemple `meteo = "re2020"` est ignoré si `THBCE_METEO` ne désigne pas le fichier
//! réglementaire. La commande affiche aussi la synthèse des indicateurs obtenus (utile pour
//! écrire l'`attendu.json` d'un nouvel exemple).

use std::path::{Path, PathBuf};

use serde_json::Value;
use thbce_core::climat::MeteoProjet;
use thbce_core::entree::Projet;
use thbce_core::{calculer_avec, Demandes, Options};
use thbce_io::{json, meteo, rapport};

use crate::racine;

/// Tolérance relative par défaut : 0,5 % (comme le harnais).
const TOLERANCE_DEFAUT: f64 = 0.005;

fn dossier_des_cas() -> PathBuf {
    racine().join("tests/cas")
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

/// Compare récursivement (mêmes règles que `comparer` du harnais) ; ajoute une ligne par écart.
fn comparer(
    chemin: &str,
    champ: &str,
    attendu: &Value,
    obtenu: &Value,
    tol: Option<&toml::Table>,
    ecarts: &mut Vec<String>,
) {
    let mut ecart = |detail: String| {
        ecarts.push(format!(
            "  {chemin:<50} attendu {:>12}  obtenu {:>12}  {detail}",
            attendu.to_string(),
            obtenu.to_string()
        ));
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
                    None => ecarts.push(format!(
                        "  {sous_chemin:<50} attendu {:>12}  obtenu {:>12}  champ absent des résultats",
                        valeur.to_string(),
                        "(absent)"
                    )),
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

/// Jeu météo d'un exemple ; `None` pour un exemple réglementaire sans `THBCE_METEO`.
fn meteo_du_cas(nature: &str, projet: &Projet) -> Result<Option<MeteoProjet>, String> {
    let zone = projet.localisation.zone_climatique_retenue();
    let lire = |chemin: &Path| -> Result<MeteoProjet, String> {
        let octets = std::fs::read(chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        meteo::lire(&octets, zone).map_err(|e| format!("{} : {e}", chemin.display()))
    };
    match nature {
        "synthetique" => Ok(Some(MeteoProjet::synthetique(zone))),
        "re2020" => match std::env::var_os("THBCE_METEO") {
            Some(c) => lire(Path::new(&c)).map(Some),
            None => Ok(None),
        },
        "ademe" => lire(
            &racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv"),
        )
        .map(Some),
        autre => Err(format!(
            "cas.toml : meteo = \"{autre}\" inconnu (\"re2020\", \"ademe\" ou \"synthetique\")"
        )),
    }
}

/// Calcule un exemple et le compare ; `Ok(true)` s'il est conforme ou ignoré.
fn un_cas(nom: &str) -> Result<bool, String> {
    let dossier = dossier_des_cas().join(nom);
    let lire = |f: &str| {
        std::fs::read_to_string(dossier.join(f)).map_err(|e| format!("tests/cas/{nom}/{f} : {e}"))
    };
    let cas: toml::Table = lire("cas.toml")?
        .parse()
        .map_err(|e| format!("tests/cas/{nom}/cas.toml : {e}"))?;
    let projet = json::lire_projet(&lire("entree.json")?)
        .map_err(|e| format!("tests/cas/{nom}/entree.json : {e}"))?;
    let attendu: Value = serde_json::from_str(&lire("attendu.json")?)
        .map_err(|e| format!("tests/cas/{nom}/attendu.json : {e}"))?;
    let nature = cas
        .get("meteo")
        .and_then(toml::Value::as_str)
        .unwrap_or("re2020");
    let Some(donnees) = meteo_du_cas(nature, &projet)? else {
        println!("{nom} : IGNORÉ (météo réglementaire requise, THBCE_METEO non défini)");
        return Ok(true);
    };
    let resultats = calculer_avec(
        &projet,
        &donnees,
        &Options::default(),
        &demandes_du_cas(&cas),
    )
    .map_err(|e| format!("{nom} : le calcul a échoué : {e}"))?;
    print!("{}", rapport::synthese_indicateurs(&resultats));
    let obtenu = serde_json::to_value(&resultats).map_err(|e| format!("{nom} : {e}"))?;
    let tolerances = cas.get("tolerances").and_then(toml::Value::as_table);
    let mut ecarts = Vec::new();
    comparer("", "", &attendu, &obtenu, tolerances, &mut ecarts);
    if ecarts.is_empty() {
        println!("{nom} : conforme");
        Ok(true)
    } else {
        println!("{nom} : {} écart(s)", ecarts.len());
        for e in &ecarts {
            println!("{e}");
        }
        Ok(false)
    }
}

/// `xtask cas [<nom>…]`.
pub fn commande(noms: &[&str]) -> Result<(), String> {
    let mut tous: Vec<String> = Vec::new();
    if noms.is_empty() {
        let lecture =
            std::fs::read_dir(dossier_des_cas()).map_err(|e| format!("tests/cas : {e}"))?;
        for e in lecture.flatten() {
            if e.path().join("entree.json").is_file() {
                tous.push(e.file_name().to_string_lossy().into_owned());
            }
        }
        tous.sort();
    } else {
        tous = noms
            .iter()
            .map(|n| n.trim_end_matches('/').to_owned())
            .collect();
    }
    let mut echecs = Vec::new();
    for nom in &tous {
        // Accepte `tests/cas/<nom>` comme `<nom>`.
        let nom = nom.rsplit('/').next().unwrap_or(nom);
        if !dossier_des_cas().join(nom).join("entree.json").is_file() {
            return Err(format!("tests/cas/{nom}/entree.json introuvable"));
        }
        if !un_cas(nom)? {
            echecs.push(nom.to_owned());
        }
    }
    if echecs.is_empty() {
        Ok(())
    } else {
        Err(format!("exemples en écart : {}", echecs.join(", ")))
    }
}

/// Sorties facultatives que le cas demande (`sorties_detaillees = true` dans `cas.toml`).
fn demandes_du_cas(cas: &toml::Table) -> Demandes {
    Demandes {
        sorties_detaillees: cas
            .get("sorties_detaillees")
            .and_then(toml::Value::as_bool)
            .unwrap_or(false),
        ..Demandes::default()
    }
}
