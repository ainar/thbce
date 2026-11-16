// SPDX-License-Identifier: GPL-3.0-or-later
//! Scripts reproductibles du dépôt.
//!
//! ```text
//! cargo run -p xtask -- zones-climatiques [--verifier]
//! cargo run -p xtask -- meteo-synthetique <fichier.csv>
//! ```
//!
//! - `zones-climatiques` régénère `crates/thbce-data/src/zones_climatiques/table.rs` depuis
//!   l'extraction texte de l'arrêté du 26 octobre 2010. Avec `--verifier`, n'écrit rien et
//!   échoue si le fichier versionné diffère (utilisé par la CI).
//! - `meteo-synthetique` écrit un jeu météo constant pour les huit zones, au format CSV long,
//!   pour les tests de bout en bout. Ce jeu n'est pas réglementaire.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use thbce_core::climat::donnees::DESCRIPTION_SYNTHETIQUE;
use thbce_core::climat::MeteoProjet;
use thbce_data::ZoneClimatique;

mod zones;

fn racine() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

fn zones_climatiques(verifier: bool) -> Result<(), String> {
    let racine = racine();
    let source = racine.join(zones::SOURCE);
    let sortie = racine.join(zones::SORTIE);
    let texte =
        std::fs::read_to_string(&source).map_err(|e| format!("{} : {e}", source.display()))?;
    let genere = zones::generer(&zones::extraire(&texte)?);
    if verifier {
        let actuel =
            std::fs::read_to_string(&sortie).map_err(|e| format!("{} : {e}", sortie.display()))?;
        // Comparaison insensible aux fins de ligne (extraction sous Windows).
        if actuel.replace("\r\n", "\n") != genere {
            return Err(format!(
                "{} n'est pas à jour : relancer `cargo run -p xtask -- zones-climatiques`",
                zones::SORTIE
            ));
        }
        println!("{} est à jour", zones::SORTIE);
    } else {
        std::fs::write(&sortie, genere).map_err(|e| format!("{} : {e}", sortie.display()))?;
        println!("{} écrit", zones::SORTIE);
    }
    Ok(())
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
        ["meteo-synthetique", sortie] => meteo_synthetique(sortie),
        _ => Err(
            "usage : xtask zones-climatiques [--verifier] | xtask meteo-synthetique <fichier.csv>"
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
