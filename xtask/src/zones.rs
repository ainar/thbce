// SPDX-License-Identifier: GPL-3.0-or-later
//! Extraction de la table département → zone climatique.

use std::fmt::Write as _;

/// Extraction texte de l'arrêté du 26 octobre 2010 (annexe I), relative à la racine du dépôt.
pub const SOURCE: &str = "docs/reference/texte/arrete-26-octobre-2010-rt2012.txt";
/// Fichier généré, relatif à la racine du dépôt.
pub const SORTIE: &str = "crates/thbce-data/src/zones_climatiques/table.rs";

const ZONES: [&str; 8] = ["H1a", "H1b", "H1c", "H2a", "H2b", "H2c", "H2d", "H3"];

/// Lit les lignes « NN ― Nom du département        Hxx » de l'annexe I.
pub fn extraire(texte: &str) -> Result<Vec<(String, String, String)>, String> {
    let mut lignes = Vec::new();
    for ligne in texte.lines() {
        let Some((gauche, nom_et_zone)) = ligne.split_once(" ― ") else {
            continue;
        };
        let code = gauche.trim();
        let est_code = code == "2A"
            || code == "2B"
            || (code.len() == 2 && code.bytes().all(|b| b.is_ascii_digit()));
        if !est_code {
            continue;
        }
        let Some((nom, zone)) = nom_et_zone.trim_end().rsplit_once(char::is_whitespace) else {
            continue;
        };
        if !ZONES.contains(&zone) {
            continue;
        }
        lignes.push((code.to_owned(), nom.trim().to_owned(), zone.to_owned()));
    }
    lignes.sort();
    for paire in lignes.windows(2) {
        if paire[0].0 == paire[1].0 {
            return Err(format!(
                "département {} présent deux fois dans la source",
                paire[0].0
            ));
        }
    }
    if lignes.len() != 96 {
        return Err(format!(
            "{} départements lus dans {SOURCE}, 96 attendus",
            lignes.len()
        ));
    }
    Ok(lignes)
}

/// Produit le source Rust de la table.
pub fn generer(lignes: &[(String, String, String)]) -> String {
    let mut s = String::new();
    s.push_str("// SPDX-License-Identifier: GPL-3.0-or-later\n");
    s.push_str("// Fichier généré par `cargo run -p xtask -- zones-climatiques` : ne pas modifier à la main.\n");
    s.push_str("// Source : arrêté du 26 octobre 2010, annexe I\n");
    let _ = writeln!(s, "// ({SOURCE}).");
    s.push_str("\nuse super::ZoneClimatique;\n\n");
    s.push_str("/// Zone climatique de chaque département, triée par code de département.\n");
    let _ = writeln!(
        s,
        "pub const ZONE_PAR_DEPARTEMENT: [(&str, ZoneClimatique); {}] = [",
        lignes.len()
    );
    for (code, nom, zone) in lignes {
        // Commentaires alignés comme le fait rustfmt, pour que le fichier généré soit stable.
        let entree = format!("(\"{code}\", ZoneClimatique::{zone}),");
        let _ = writeln!(s, "    {entree:<28} // {nom}");
    }
    s.push_str("];\n");
    s
}
