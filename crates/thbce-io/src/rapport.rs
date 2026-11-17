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
        ligne(
            &mut s,
            if p.hors_perimetre {
                "non calculé  "
            } else {
                "erreur       "
            },
            p,
        );
    }
    for p in &rapport.avertissements {
        ligne(&mut s, "avertissement", p);
    }
    let _ = writeln!(
        s,
        "{} : {} erreur(s), {} avertissement(s)",
        if rapport.est_valide() {
            "entrée valide"
        } else if rapport.seulement_hors_perimetre() {
            "entrée correcte mais non calculable par cette version (voir docs/couverture.md)"
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
        // Libellés alignés (le DH porte le nom du groupe vérifié) ; marges recalculées à partir
        // des valeurs affichées, arrondies à une décimale, pour que l'affichage reste cohérent.
        let libelle = |v: &thbce_core::indicateurs::resultats::VerificationExigence| match &v.groupe
        {
            Some(g) => format!("{} ({g})", v.indicateur),
            None => v.indicateur.clone(),
        };
        let largeur = e
            .verifications
            .iter()
            .map(|v| libelle(v).chars().count())
            .max()
            .unwrap_or(0);
        for v in &e.verifications {
            let valeur = arrondi_1(v.valeur);
            let maximum = arrondi_1(v.maximum);
            let marge = arrondi_1(maximum - valeur);
            let marge_pct = if maximum == 0.0 {
                0.0
            } else {
                100.0 * marge / maximum
            };
            let l = libelle(v);
            let _ = writeln!(
                s,
                "  {l}{} {valeur:>8.1} ≤ {maximum:>8.1} : marge {marge:>8.1} ({marge_pct:+.1} %) {}",
                " ".repeat(largeur - l.chars().count()),
                if v.respectee {
                    "respectée"
                } else {
                    "NON RESPECTÉE"
                }
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

/// Arrondi à une décimale, sans « -0,0 ».
fn arrondi_1(x: f64) -> f64 {
    let r = (x * 10.0).round() / 10.0;
    if r == 0.0 {
        0.0
    } else {
        r
    }
}
