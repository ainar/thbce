// SPDX-License-Identifier: GPL-3.0-or-later
//! Format d'entrée JSON natif.

use thbce_core::entree::Projet;
use thbce_core::resultats::Resultats;

/// Erreur de lecture d'une entrée JSON.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{chemin} : {message} (ligne {ligne}, colonne {colonne})")]
pub struct ErreurLecture {
    /// Chemin de l'élément fautif, ou `.` pour la racine.
    pub chemin: String,
    /// Message de l'analyseur.
    pub message: String,
    /// Ligne dans le texte d'entrée (0 si inconnue).
    pub ligne: usize,
    /// Colonne dans le texte d'entrée (0 si inconnue).
    pub colonne: usize,
}

/// Lit la description d'un projet au format JSON natif.
///
/// Les champs inconnus sont refusés, pour qu'une faute de frappe ne passe pas inaperçue.
pub fn lire_projet(texte: &str) -> Result<Projet, ErreurLecture> {
    let mut de = serde_json::Deserializer::from_str(texte);
    serde_path_to_error::deserialize(&mut de).map_err(|e| {
        let chemin = e.path().to_string();
        let interne = e.into_inner();
        ErreurLecture {
            chemin,
            ligne: interne.line(),
            colonne: interne.column(),
            message: interne.to_string(),
        }
    })
}

/// Lit des résultats au format JSON (cas de validation).
pub fn lire_resultats(texte: &str) -> Result<Resultats, serde_json::Error> {
    serde_json::from_str(texte)
}

/// Écrit des résultats en JSON indenté, terminé par un saut de ligne.
pub fn ecrire_resultats(resultats: &Resultats) -> String {
    // La sérialisation de `Resultats` ne peut pas échouer : ni clé non textuelle ni valeur non finie
    // ne sont produites par le moteur. En cas d'échec, l'erreur est restituée comme document JSON.
    let mut s = serde_json::to_string_pretty(resultats)
        .unwrap_or_else(|e| format!("{{\"erreur_de_serialisation\": {:?}}}", e.to_string()));
    s.push('\n');
    s
}

/// JSON Schema du format d'entrée, généré depuis les types Rust.
pub fn schema_entree() -> String {
    let schema = schemars::schema_for!(Projet);
    let mut s = serde_json::to_string_pretty(&schema).unwrap_or_default();
    s.push('\n');
    s
}

/// JSON Schema des résultats, généré depuis les types Rust.
pub fn schema_resultats() -> String {
    let schema = schemars::schema_for!(Resultats);
    let mut s = serde_json::to_string_pretty(&schema).unwrap_or_default();
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signale_le_chemin_d_une_erreur() {
        let e = lire_projet(r#"{"nom": "x", "localisation": {"departement": "20"}}"#).unwrap_err();
        assert_eq!(e.chemin, "localisation.departement");
        assert!(e.message.contains("département"), "{}", e.message);
    }

    #[test]
    fn refuse_un_champ_inconnu() {
        let e = lire_projet(r#"{"nom": "x", "surprise": 1}"#).unwrap_err();
        assert!(e.message.contains("surprise"), "{}", e.message);
    }

    #[test]
    fn le_schema_decrit_le_projet() {
        let s = schema_entree();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["title"], "Projet");
        assert!(s.contains("zone_climatique"));
        assert!(schema_resultats().contains("reglementaire"));
    }
}
