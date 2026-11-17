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

/// Synthèse lisible des indicateurs réglementaires de chaque bâtiment : Bbio, Cep, Cep,nr,
/// Ic énergie et DH, avec, quand les exigences sont calculées, la valeur maximale, la marge et
/// le respect de chaque exigence.
pub fn synthese_indicateurs(r: &thbce_core::resultats::Resultats) -> String {
    let mut s = String::new();
    let opt = |x: Option<f64>| x.map_or_else(|| "non calculé".to_owned(), |v| format!("{v:.1}"));
    for b in &r.batiments {
        let _ = writeln!(
            s,
            "bâtiment {} : Bbio {} points ; Cep {} kWhep/(m².an) ; Cep,nr {} kWhep/(m².an) ; \
             Ic énergie {} kg éq. CO2/m² ; DH {} °C.h",
            b.id,
            opt(b.bbio_pts),
            opt(b.cep_kwhep_m2),
            opt(b.cep_nr_kwhep_m2),
            opt(b.ic_energie_kgco2_m2),
            opt(b.dh_c_h)
        );
        let Some(e) = b.indicateurs.as_ref().and_then(|i| i.exigences.as_ref()) else {
            continue;
        };
        for v in &e.verifications {
            let _ = writeln!(
                s,
                "  {:<11}{} {:>8.1} ≤ {:>8.1} : marge {:>8.1} ({:+.1} %) {}",
                v.indicateur,
                v.groupe
                    .as_ref()
                    .map_or_else(String::new, |g| format!(" ({g})")),
                v.valeur,
                v.maximum,
                v.marge,
                v.marge_pct,
                if v.respectee { "respectée" } else { "NON RESPECTÉE" }
            );
        }
        let _ = writeln!(
            s,
            "  exigences de résultat (Bbio, Cep, Cep,nr, Ic énergie, DH) : {}",
            if e.conforme {
                "respectées"
            } else {
                "non respectées"
            }
        );
    }
    s
}
