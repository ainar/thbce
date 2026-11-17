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
    /// Message de l'analyseur, en français, sans la position.
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
        let categorie = interne.classify();
        // Un document tronqué laisse un dernier segment inconnu (`?`) dans le chemin.
        let mut chemin = chemin.as_str();
        while let Some(c) = chemin.strip_suffix(".?").or(chemin.strip_suffix('?')) {
            chemin = c;
        }
        ErreurLecture {
            chemin: if chemin.is_empty() {
                ".".into()
            } else {
                chemin.into()
            },
            ligne: interne.line(),
            colonne: interne.column(),
            message: message_en_francais(&interne, categorie),
        }
    })
}

/// Message d'une erreur de `serde_json` en français, sans la position (« at line … column … »),
/// restituée à part dans [`ErreurLecture`].
fn message_en_francais(e: &serde_json::Error, categorie: serde_json::error::Category) -> String {
    use serde_json::error::Category;
    let brut = e.to_string();
    let message = match brut.rfind(" at line ") {
        Some(i) => &brut[..i],
        None => brut.as_str(),
    };
    match categorie {
        Category::Eof => "fin de fichier inattendue : le document JSON est incomplet".into(),
        Category::Io => format!("lecture impossible : {message}"),
        Category::Syntax => format!("JSON mal formé : {}", syntaxe(message)),
        Category::Data => donnees(message),
    }
}

fn syntaxe(m: &str) -> String {
    let traductions = [
        ("expected value", "valeur attendue"),
        ("expected ident", "mot-clé attendu (true, false ou null)"),
        ("expected `,` or `]`", "`,` ou `]` attendu"),
        ("expected `,` or `}`", "`,` ou `}` attendu"),
        ("expected `:`", "`:` attendu"),
        ("trailing comma", "virgule en trop"),
        ("trailing characters", "caractères après la fin du document"),
        (
            "key must be a string",
            "une clé doit être un texte entre guillemets",
        ),
        ("invalid number", "nombre invalide"),
        ("number out of range", "nombre hors limites"),
        ("invalid escape", "séquence d'échappement invalide"),
        ("control character", "caractère de contrôle dans un texte"),
        ("recursion limit exceeded", "imbrication trop profonde"),
    ];
    traductions
        .iter()
        .find(|(en, _)| m.starts_with(en))
        .map_or_else(|| m.to_owned(), |(_, fr)| (*fr).to_owned())
}

/// Messages des erreurs de données de serde (champ inconnu ou manquant, type ou valeur
/// invalide). Les messages propres au moteur, déjà en français, sont conservés.
fn donnees(m: &str) -> String {
    if let Some(reste) = m.strip_prefix("unknown field ") {
        let (champ, attendus) = coupe_attendu(reste);
        return match attendus {
            Some(a) => format!("champ inconnu {champ} ; champs admis : {}", liste(a)),
            None => format!("champ inconnu {champ}"),
        };
    }
    if let Some(reste) = m.strip_prefix("unknown variant ") {
        let (valeur, attendus) = coupe_attendu(reste);
        return match attendus {
            Some(a) => format!("valeur inconnue {valeur} ; valeurs admises : {}", liste(a)),
            None => format!("valeur inconnue {valeur}"),
        };
    }
    if let Some(champ) = m.strip_prefix("missing field ") {
        return format!("champ obligatoire absent : {champ}");
    }
    if let Some(champ) = m.strip_prefix("duplicate field ") {
        return format!("champ en double : {champ}");
    }
    if let Some(reste) = m.strip_prefix("invalid type: ") {
        let (recu, attendu) = reste.split_once(", expected ").unwrap_or((reste, ""));
        return format!(
            "type invalide : {} reçu, {} attendu",
            recu_fr(recu),
            attendu_fr(attendu)
        );
    }
    if let Some(reste) = m.strip_prefix("invalid value: ") {
        let (recu, attendu) = reste.split_once(", expected ").unwrap_or((reste, ""));
        return format!(
            "valeur invalide : {} reçu, {} attendu",
            recu_fr(recu),
            attendu_fr(attendu)
        );
    }
    if let Some(reste) = m.strip_prefix("invalid length ") {
        let (n, attendu) = reste.split_once(", expected ").unwrap_or((reste, ""));
        return format!(
            "longueur invalide : {n} élément(s), {} attendu",
            attendu_fr(attendu)
        );
    }
    if m.starts_with("data did not match any variant") {
        return "valeur qui ne correspond à aucune des formes admises (voir `thbce schema`)".into();
    }
    m.to_owned()
}

/// Sépare « `x`, expected one of `a`, `b` » en « `x` » et « `a`, `b` ».
fn coupe_attendu(reste: &str) -> (&str, Option<&str>) {
    if let Some((x, a)) = reste.split_once(", expected one of ") {
        (x, Some(a))
    } else if let Some((x, a)) = reste.split_once(", expected ") {
        (x, Some(a))
    } else if let Some((x, _)) = reste.split_once(", there are no ") {
        (x, None)
    } else {
        (reste, None)
    }
}

fn liste(a: &str) -> String {
    a.replace(" or ", " ou ")
}

fn recu_fr(r: &str) -> String {
    let paires = [
        ("string ", "texte "),
        ("integer ", "entier "),
        ("floating point ", "nombre "),
        ("boolean ", "booléen "),
        ("char ", "caractère "),
    ];
    for (en, fr) in paires {
        if let Some(v) = r.strip_prefix(en) {
            return format!("{fr}{v}");
        }
    }
    match r {
        "null" => "null".into(),
        "map" => "objet".into(),
        "sequence" => "liste".into(),
        "unit value" => "valeur vide".into(),
        autre => autre.into(),
    }
}

fn attendu_fr(a: &str) -> String {
    match a {
        "f64" | "f32" => "nombre".into(),
        "u8" | "u16" | "u32" | "u64" | "usize" => "entier positif ou nul".into(),
        "i8" | "i16" | "i32" | "i64" | "isize" => "entier".into(),
        "a boolean" => "booléen".into(),
        "a string" => "texte".into(),
        "a sequence" => "liste".into(),
        "a map" => "objet".into(),
        "unit" => "valeur vide".into(),
        _ if a.starts_with("struct ") => "objet".into(),
        _ if a.starts_with("enum ") || a.starts_with("internally tagged enum") => {
            "valeur d'énumération (voir `thbce schema`)".into()
        }
        _ if a.starts_with("an array of length ") || a.starts_with("a tuple of size ") => {
            format!("liste de {} éléments", a.rsplit(' ').next().unwrap_or("?"))
        }
        autre => autre.into(),
    }
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

/// JSON Schema des résultats, généré depuis les types Rust tels qu'ils sont écrits : un champ y
/// est requis s'il figure toujours dans les résultats.
pub fn schema_resultats() -> String {
    let schema = schemars::generate::SchemaSettings::default()
        .for_serialize()
        .into_generator()
        .into_root_schema_for::<Resultats>();
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
        assert!(
            e.message.starts_with("champ inconnu `surprise`"),
            "{}",
            e.message
        );
        assert!(!e.message.contains("line"), "{}", e.message);
    }

    #[test]
    fn messages_en_francais() {
        let e = lire_projet(r#"{"nom": 3}"#).unwrap_err();
        assert_eq!(e.chemin, "nom");
        assert_eq!(e.message, "type invalide : entier `3` reçu, texte attendu");
        let e =
            lire_projet(r#"{"nom": "x", "localisation": {"departement": "37", "alt_m": "haut""#)
                .unwrap_err();
        assert_eq!(
            e.message,
            "type invalide : texte \"haut\" reçu, nombre attendu"
        );
        let e = lire_projet(r#"{"nom": "x", "localisation": {"#).unwrap_err();
        assert_eq!(
            e.message,
            "fin de fichier inattendue : le document JSON est incomplet"
        );
        assert!(!e.chemin.contains('?'), "{}", e.chemin);
        let e = lire_projet("").unwrap_err();
        assert_eq!(e.chemin, ".");
        let e = lire_projet(r#"{"nom": "x",}"#).unwrap_err();
        assert!(e.message.starts_with("JSON mal formé"), "{}", e.message);
        let e = lire_projet(r#"{"nom": "x"}"#).unwrap_err();
        assert_eq!(e.message, "champ obligatoire absent : `localisation`");
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
