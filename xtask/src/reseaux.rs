// SPDX-License-Identifier: GPL-3.0-or-later
//! Extraction des réseaux de chaleur et de froid de l'annexe 7 de l'arrêté du 15 septembre 2006.

use std::fmt::Write as _;

/// Extraction texte de l'annexe 7, relative à la racine du dépôt.
pub const SOURCE: &str = "docs/reference/texte/annexe-7-arrete-15-sept-2006-reseaux-chaleur.txt";
/// Fichier généré, relatif à la racine du dépôt.
pub const SORTIE: &str = "crates/thbce-data/src/reseaux_chaleur/table.rs";

/// Un réseau : identifiant, contenu CO2 ACV (5e colonne), taux d'EnR&R en % (6e colonne).
pub type Reseau = (String, f64, Option<f64>);

/// Réseaux lus, ligne « autres réseaux de chaleur » (CO2 ACV, taux) et « autres réseaux de froid ».
pub type Table = (Vec<Reseau>, (f64, f64), f64);

/// Lit un nombre à virgule décimale française.
fn nombre(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse().ok()
}

/// Lit les lignes du tableau : identifiant (quatre chiffres et « C » ou « F »), puis les trois
/// derniers champs numériques (CO2, CO2 ACV, taux) et l'année de référence du taux.
pub fn extraire(texte: &str) -> Result<Table, String> {
    let mut reseaux: Vec<Reseau> = Vec::new();
    let mut autres_chaleur = None;
    let mut autres_froid = None;
    for ligne in texte.lines() {
        let champs: Vec<&str> = ligne.split_whitespace().collect();
        let Some(premier) = champs.first() else {
            continue;
        };
        let est_id = premier.len() == 5
            && premier[..4].bytes().all(|b| b.is_ascii_digit())
            && matches!(premier.as_bytes()[4], b'C' | b'F');
        if est_id {
            if champs.len() < 5 {
                return Err(format!("ligne illisible : {ligne}"));
            }
            // Champs : …, CO2, CO2 ACV, taux (« 51,1% » ou « - »), année, parfois suivie de texte
            // parasite (« Moyenne CO₂ ») : on part du dernier champ de taux.
            let Some(pos) = champs.iter().rposition(|c| c.ends_with('%') || *c == "-") else {
                return Err(format!("taux introuvable : {ligne}"));
            };
            let taux = champs[pos];
            let acv = champs
                .get(pos.wrapping_sub(1))
                .and_then(|c| nombre(c))
                .ok_or_else(|| format!("CO2 ACV illisible : {ligne}"))?;
            let taux = if taux == "-" {
                None
            } else {
                Some(
                    nombre(taux.trim_end_matches('%'))
                        .ok_or_else(|| format!("taux illisible : {ligne}"))?,
                )
            };
            let id = (*premier).to_owned();
            // Le taux d'un réseau de froid est « - » ; celui d'un réseau de chaleur est lu.
            if (id.ends_with('C')) != taux.is_some() {
                return Err(format!("taux incohérent avec le type du réseau : {ligne}"));
            }
            reseaux.push((id, acv, taux));
        } else if ligne.trim_start().starts_with("AUTRES R") && ligne.contains("CHALEUR") {
            let v: Vec<f64> = champs.iter().filter_map(|c| nombre(c)).collect();
            if v.len() == 2 {
                autres_chaleur = Some((v[0], v[1]));
            }
        } else if ligne.trim_start().starts_with("AUTRES R") && ligne.contains("FROID") {
            autres_froid = champs.iter().find_map(|c| nombre(c));
        }
    }
    reseaux.sort_by(|a, b| a.0.cmp(&b.0));
    for paire in reseaux.windows(2) {
        if paire[0].0 == paire[1].0 {
            return Err(format!("réseau {} présent deux fois", paire[0].0));
        }
    }
    if reseaux.len() != 1086 {
        return Err(format!(
            "{} réseaux lus dans {SOURCE}, 1086 attendus",
            reseaux.len()
        ));
    }
    let chaleur = autres_chaleur.ok_or("ligne « autres réseaux de chaleur » introuvable")?;
    let froid = autres_froid.ok_or("ligne « autres réseaux de froid » introuvable")?;
    Ok((reseaux, chaleur, froid))
}

/// Produit le source Rust de la table.
pub fn generer(reseaux: &[Reseau], chaleur: (f64, f64), froid: f64) -> String {
    let mut s = String::new();
    s.push_str("// SPDX-License-Identifier: GPL-3.0-or-later\n");
    s.push_str("// Fichier généré par `cargo run -p xtask -- reseaux-chaleur` : ne pas modifier à la main.\n");
    s.push_str("// Source : annexe 7 de l'arrêté du 15 septembre 2006, version en vigueur au 25 mai 2026\n");
    let _ = writeln!(s, "// ({SOURCE}).\n");
    s.push_str("// Valeurs de table : certaines ressemblent à des constantes mathématiques.\n#![allow(clippy::approx_constant)]\n\n");
    let _ = writeln!(
        s,
        "/// « Autres réseaux de chaleur » : contenu CO2 ACV en kg éq. CO2/kWh et taux d'énergie\n/// renouvelable et de récupération en %.\npub const AUTRES_RESEAUX_DE_CHALEUR: (f64, f64) = ({:.3}, {:.1});\n",
        chaleur.0, chaleur.1
    );
    let _ = writeln!(
        s,
        "/// « Autres réseaux de froid » : contenu CO2 ACV en kg éq. CO2/kWh.\npub const AUTRES_RESEAUX_DE_FROID: f64 = {froid:.3};\n"
    );
    s.push_str("/// Réseaux triés par identifiant : contenu CO2 ACV en kg éq. CO2/kWh (5e colonne, arrêté du\n/// 4 août 2021, article 10) et taux d'énergie renouvelable et de récupération en %\n/// (6e colonne, article 9 ; `None` pour un réseau de froid).\n");
    let _ = writeln!(
        s,
        "pub static RESEAUX: [(&str, f64, Option<f64>); {}] = [",
        reseaux.len()
    );
    for (id, acv, taux) in reseaux {
        match taux {
            Some(t) => {
                let _ = writeln!(s, "    (\"{id}\", {acv:.3}, Some({t:.1})),");
            }
            None => {
                let _ = writeln!(s, "    (\"{id}\", {acv:.3}, None),");
            }
        }
    }
    s.push_str("];\n");
    s
}
