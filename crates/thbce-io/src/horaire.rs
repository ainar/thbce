// SPDX-License-Identifier: GPL-3.0-or-later
//! Mise en forme des séries horaires restituées sur demande (Q118).
//!
//! - CSV : séparateur `,`, point décimal, une ligne d'en-tête puis 8 760 lignes ; première
//!   colonne `heure` (rang de l'heure de l'année, de 1 à 8 760), puis une colonne par série,
//!   nommée `<bâtiment>.<série> [<unité>]` (`C` ou `Wh`).
//! - JSON : `{ "batiments": [ { "id", "colonnes": [ { "nom", "unite", "valeurs" } ] } ] }`.
//!
//! Les noms de séries et leur signification sont décrits dans `docs/modele-donnees.md`.

use std::fmt::Write as _;

use thbce_core::resultats::Resultats;

/// Vrai si `resultats` porte des séries horaires.
pub fn presentes(resultats: &Resultats) -> bool {
    resultats
        .batiments
        .iter()
        .any(|b| b.series_horaires.is_some())
}

/// Séries horaires de tous les bâtiments, au format CSV.
pub fn en_csv(resultats: &Resultats) -> String {
    let colonnes: Vec<(String, &str, &[f64])> = resultats
        .batiments
        .iter()
        .filter_map(|b| b.series_horaires.as_ref().map(|s| (&b.id, s)))
        .flat_map(|(id, s)| {
            s.colonnes
                .iter()
                .map(move |c| (format!("{id}.{}", c.nom), c.unite.as_str(), &c.valeurs[..]))
        })
        .collect();
    let nb = colonnes.iter().map(|c| c.2.len()).max().unwrap_or(0);
    let mut s = String::from("heure");
    for (nom, unite, _) in &colonnes {
        let _ = write!(s, ",{nom} [{unite}]");
    }
    s.push('\n');
    for h in 0..nb {
        let _ = write!(s, "{}", h + 1);
        for (_, _, v) in &colonnes {
            let _ = write!(s, ",{}", v.get(h).copied().unwrap_or(f64::NAN));
        }
        s.push('\n');
    }
    s
}

/// Séries horaires de tous les bâtiments, au format JSON indenté.
pub fn en_json(resultats: &Resultats) -> String {
    let batiments: Vec<_> = resultats
        .batiments
        .iter()
        .filter_map(|b| {
            b.series_horaires
                .as_ref()
                .map(|s| serde_json::json!({ "id": b.id, "colonnes": s.colonnes }))
        })
        .collect();
    let mut s = serde_json::to_string_pretty(&serde_json::json!({ "batiments": batiments }))
        .unwrap_or_default();
    s.push('\n');
    s
}
