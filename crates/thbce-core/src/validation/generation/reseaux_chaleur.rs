// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des sous-stations de réseaux de chaleur ou de froid (Th-BCE 2020 §8.28).

use crate::entree::ReseauChaleurFroidSaisi;
use crate::validation::RapportValidation;

/// Th-BCE 2020 §8.28 : sous-station d'un réseau de chaleur ou de froid.
pub(super) fn valider_reseau(r: &mut RapportValidation, c: &str, x: &ReseauChaleurFroidSaisi) {
    let (id, p_ess, froid) = match x {
        ReseauChaleurFroidSaisi::Chaleur {
            identifiant_reseau,
            classe_isolation_secondaire,
            p_ess_kw,
            ..
        } => {
            if !(1..=4).contains(classe_isolation_secondaire) {
                r.erreur(
                    format!("{c}.chaleur.classe_isolation_secondaire"),
                    Some(classe_isolation_secondaire.to_string()),
                    "classe d'isolation du réseau secondaire de 1 à 4 (Th-BCE 2020 §8.28.3.1, \
                     tableau 247)",
                );
            }
            (identifiant_reseau, *p_ess_kw, false)
        }
        ReseauChaleurFroidSaisi::Froid {
            identifiant_reseau,
            p_ess_kw,
        } => (identifiant_reseau, *p_ess_kw, true),
    };
    r.strictement_positif(format!("{c}.p_ess_kw"), p_ess);
    if let Some(id) = id {
        let lettre = if froid { 'F' } else { 'C' };
        let bon =
            id.len() == 5 && id[..4].chars().all(|ch| ch.is_ascii_digit()) && id.ends_with(lettre);
        if bon && thbce_data::reseaux_chaleur::reseau(id).is_none() {
            r.erreur(
                format!("{c}.identifiant_reseau"),
                Some(id.clone()),
                "identifiant absent de l'annexe 7 de l'arrêté du 15 septembre 2006 : omettre le \
                 champ pour employer les lignes « autres réseaux »",
            );
        }
        if !bon {
            r.erreur(
                format!("{c}.identifiant_reseau"),
                Some(id.clone()),
                format!(
                    "identifiant de réseau de {} : quatre chiffres suivis de « {lettre} » \
                     (annexe 7 de l'arrêté du 15 septembre 2006)",
                    if froid { "froid" } else { "chaleur" }
                ),
            );
        }
    }
}
