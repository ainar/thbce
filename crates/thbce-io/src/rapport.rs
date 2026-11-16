// SPDX-License-Identifier: GPL-3.0-or-later
//! Mise en forme lisible des rapports de validation.

use std::fmt::Write as _;

use thbce_core::validation::{Probleme, RapportValidation};

fn ligne(s: &mut String, nature: &str, p: &Probleme) {
    let _ = match &p.valeur {
        Some(v) => writeln!(s, "{nature} {} = {v} : {}", p.chemin, p.message),
        None => writeln!(s, "{nature} {} : {}", p.chemin, p.message),
    };
}

/// Rapport en texte : une ligne par problème, puis un bilan.
pub fn en_texte(rapport: &RapportValidation) -> String {
    let mut s = String::new();
    for p in &rapport.erreurs {
        ligne(&mut s, "erreur       ", p);
    }
    for p in &rapport.avertissements {
        ligne(&mut s, "avertissement", p);
    }
    let _ = writeln!(
        s,
        "{} : {} erreur(s), {} avertissement(s)",
        if rapport.est_valide() {
            "entrée valide"
        } else {
            "entrée invalide"
        },
        rapport.erreurs.len(),
        rapport.avertissements.len()
    );
    s
}
