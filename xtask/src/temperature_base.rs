// SPDX-License-Identifier: GPL-3.0-or-later
//! Extraction du tableau des températures extérieures de base (Th-BCE 2020 §3.1.3).

use std::fmt::Write as _;

/// Extraction texte de l'annexe III, relative à la racine du dépôt.
pub const SOURCE: &str = "docs/reference/texte/annexe-iii-th-bce-2020.txt";
/// Fichier généré, relatif à la racine du dépôt.
pub const SORTIE: &str = "crates/thbce-data/src/temperature_base/table.rs";

const DEBUT: &str = "base_ex_0";
const FIN: &str = "La température de base est constante";

fn est_code(jeton: &str) -> bool {
    jeton == "2A"
        || jeton == "2B"
        || (jeton.len() == 2 && jeton.bytes().all(|b| b.is_ascii_digit()))
}

/// Lit les listes de départements et la température de base de chacune.
///
/// Dans le texte, chaque liste se termine par un point et sa température figure en fin de
/// première ligne.
pub fn extraire(texte: &str) -> Result<Vec<(String, i32)>, String> {
    let debut = texte.find(DEBUT).ok_or("début du tableau introuvable")? + DEBUT.len();
    let fin = texte[debut..]
        .find(FIN)
        .ok_or("fin du tableau introuvable")?
        + debut;
    let mut table = Vec::new();
    let mut codes: Vec<String> = Vec::new();
    let mut valeur: Option<i32> = None;
    let mut en_tete = true;
    let mut vider = |codes: &mut Vec<String>, valeur: &mut Option<i32>| -> Result<(), String> {
        if codes.is_empty() {
            return Ok(());
        }
        let v = valeur
            .take()
            .ok_or("liste de départements sans température")?;
        table.extend(codes.drain(..).map(|c| (c, v)));
        Ok(())
    };
    for jeton in texte[debut..fin].split(|c: char| c == ',' || c.is_whitespace()) {
        let fin_de_liste = jeton.ends_with('.');
        let jeton = jeton.trim_end_matches('.');
        if jeton.is_empty() {
            continue;
        }
        if est_code(jeton) {
            en_tete = false;
            codes.push(jeton.to_owned());
        } else if let Some(v) = jeton.strip_prefix('-').and_then(|n| n.parse::<i32>().ok()) {
            if valeur.replace(-v).is_some() {
                return Err("deux températures pour une même liste".into());
            }
        } else if en_tete {
            // Suite de l'en-tête du tableau (« de base »), avant la première liste.
            continue;
        } else {
            return Err(format!("jeton inattendu « {jeton} » dans le tableau"));
        }
        if fin_de_liste {
            vider(&mut codes, &mut valeur)?;
        }
    }
    vider(&mut codes, &mut valeur)?;
    table.sort();
    for paire in table.windows(2) {
        if paire[0].0 == paire[1].0 {
            return Err(format!("département {} présent deux fois", paire[0].0));
        }
    }
    if table.len() != 96 {
        return Err(format!("{} départements lus, 96 attendus", table.len()));
    }
    Ok(table)
}

/// Produit le source Rust de la table.
pub fn generer(table: &[(String, i32)]) -> String {
    let mut s = String::new();
    s.push_str("// SPDX-License-Identifier: GPL-3.0-or-later\n");
    s.push_str("// Fichier généré par `cargo run -p xtask -- temperature-base` : ne pas modifier à la main.\n");
    s.push_str(
        "// Source : Th-BCE 2020 §3.1.3, tableau des températures de base par département\n",
    );
    let _ = writeln!(s, "// ({SOURCE}).");
    s.push_str("\n/// Température extérieure de base à l'altitude 0 m, en °C, triée par code de département.\n");
    let _ = writeln!(
        s,
        "pub const BASE_EXT_0_PAR_DEPARTEMENT: [(&str, f64); {}] = [",
        table.len()
    );
    for (code, v) in table {
        let _ = writeln!(s, "    (\"{code}\", {v}.0),");
    }
    s.push_str("];\n");
    s
}
