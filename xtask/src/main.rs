// SPDX-License-Identifier: GPL-3.0-or-later
//! Scripts reproductibles du dépôt.
//!
//! ```text
//! cargo run -p xtask -- zones-climatiques [--verifier]
//! cargo run -p xtask -- temperature-base [--verifier]
//! cargo run -p xtask -- verifier
//! cargo run -p xtask -- meteo-synthetique <fichier.csv>
//! ```
//!
//! - `zones-climatiques` régénère `crates/thbce-data/src/zones_climatiques/table.rs` depuis
//!   l'extraction texte de l'arrêté du 26 octobre 2010. Avec `--verifier`, n'écrit rien et
//!   échoue si le fichier versionné diffère (utilisé par la CI).
//! - `temperature-base` fait de même pour `crates/thbce-data/src/temperature_base/table.rs`,
//!   depuis le tableau du §3.1.3 de l'annexe III.
//! - `verifier` contrôle tous les fichiers générés.
//! - `meteo-synthetique` écrit un jeu météo constant pour les huit zones, au format CSV long,
//!   pour les tests de bout en bout. Ce jeu n'est pas réglementaire.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use thbce_core::climat::donnees::DESCRIPTION_SYNTHETIQUE;
use thbce_core::climat::MeteoProjet;
use thbce_data::ZoneClimatique;

mod temperature_base;
mod zones;

fn racine() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

/// Génère un fichier versionné, ou vérifie qu'il est à jour.
fn produire(sortie: &str, genere: String, commande: &str, verifier: bool) -> Result<(), String> {
    let chemin = racine().join(sortie);
    if verifier {
        let actuel =
            std::fs::read_to_string(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        // Comparaison insensible aux fins de ligne (extraction sous Windows).
        if actuel.replace("\r\n", "\n") != genere {
            return Err(format!(
                "{sortie} n'est pas à jour : relancer `cargo run -p xtask -- {commande}`"
            ));
        }
        println!("{sortie} est à jour");
    } else {
        std::fs::write(&chemin, genere).map_err(|e| format!("{} : {e}", chemin.display()))?;
        println!("{sortie} écrit");
    }
    Ok(())
}

fn lire_source(source: &str) -> Result<String, String> {
    let chemin = racine().join(source);
    std::fs::read_to_string(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))
}

fn zones_climatiques(verifier: bool) -> Result<(), String> {
    let genere = zones::generer(&zones::extraire(&lire_source(zones::SOURCE)?)?);
    produire(zones::SORTIE, genere, "zones-climatiques", verifier)
}

fn temperature_de_base(verifier: bool) -> Result<(), String> {
    let table = temperature_base::extraire(&lire_source(temperature_base::SOURCE)?)?;
    produire(
        temperature_base::SORTIE,
        temperature_base::generer(&table),
        "temperature-base",
        verifier,
    )
}

fn meteo_synthetique(sortie: &str) -> Result<(), String> {
    let jeux: Vec<MeteoProjet> = ZoneClimatique::TOUTES
        .into_iter()
        .map(MeteoProjet::synthetique)
        .collect();
    let csv = thbce_io::meteo::ecrire_csv(&jeux, DESCRIPTION_SYNTHETIQUE);
    if let Some(parent) = Path::new(sortie)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|e| format!("{} : {e}", parent.display()))?;
    }
    std::fs::write(sortie, csv).map_err(|e| format!("{sortie} : {e}"))?;
    println!("{sortie} écrit");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let resultat = match args.as_slice() {
        ["zones-climatiques"] => zones_climatiques(false),
        ["zones-climatiques", "--verifier"] => zones_climatiques(true),
        ["temperature-base"] => temperature_de_base(false),
        ["temperature-base", "--verifier"] => temperature_de_base(true),
        ["verifier"] => zones_climatiques(true).and_then(|()| temperature_de_base(true)),
        ["meteo-synthetique", sortie] => meteo_synthetique(sortie),
        _ => Err(
            "usage : xtask zones-climatiques [--verifier] | temperature-base [--verifier] | verifier | meteo-synthetique <fichier.csv>"
                .to_owned(),
        ),
    };
    match resultat {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("erreur : {message}");
            ExitCode::FAILURE
        }
    }
}
