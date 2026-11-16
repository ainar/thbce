// SPDX-License-Identifier: GPL-3.0-or-later
//! Département d'implantation du projet.
//!
//! Th-BCE 2020 §3.1.2 et §5.1.2 : paramètre d'intégration « département ». Le texte donne la
//! plage « 1 à 95 », mais le tableau des températures de base (§3.1.3) et la table des zones
//! climatiques distinguent 2A et 2B. Décision Q13 (`docs/questions.md`) : la liste valide est
//! 01 à 95 sans 20, plus 2A et 2B, soit 96 départements.

use core::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Code d'un département de France métropolitaine : `"01"` à `"95"` (sans `"20"`), `"2A"`, `"2B"`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(try_from = "String", into = "String")]
#[schemars(
    with = "String",
    description = "Code du département : 01 à 95 (sans 20), 2A ou 2B"
)]
pub struct Departement([u8; 2]);

/// Code de département refusé.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("département « {0} » inconnu : attendu 01 à 95 (sans 20), 2A ou 2B")]
pub struct ErreurDepartement(pub String);

impl Departement {
    /// Construit un département à partir de son code. Les codes à un chiffre sont acceptés
    /// (`"1"` vaut `"01"`), ainsi que les minuscules (`"2a"`).
    pub fn nouveau(code: &str) -> Result<Self, ErreurDepartement> {
        let erreur = || ErreurDepartement(code.to_owned());
        let c = code.trim().to_ascii_uppercase();
        let octets = match c.as_bytes() {
            [u] => [b'0', *u],
            [d, u] => [*d, *u],
            _ => return Err(erreur()),
        };
        if octets == *b"2A" || octets == *b"2B" {
            return Ok(Self(octets));
        }
        if !octets.iter().all(u8::is_ascii_digit) {
            return Err(erreur());
        }
        let numero = (octets[0] - b'0') * 10 + (octets[1] - b'0');
        if !(1..=95).contains(&numero) || numero == 20 {
            return Err(erreur());
        }
        Ok(Self(octets))
    }

    /// Code sur deux caractères, par exemple `"01"` ou `"2A"`.
    pub fn code(&self) -> &str {
        // Invariant du constructeur : deux caractères ASCII.
        core::str::from_utf8(&self.0).unwrap_or("??")
    }
}

impl fmt::Display for Departement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl TryFrom<String> for Departement {
    type Error = ErreurDepartement;
    fn try_from(code: String) -> Result<Self, Self::Error> {
        Self::nouveau(&code)
    }
}

impl From<Departement> for String {
    fn from(d: Departement) -> Self {
        d.code().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepte_les_96_departements() {
        let mut n = 0;
        for i in 1..=95u8 {
            let code = format!("{i:02}");
            match Departement::nouveau(&code) {
                Ok(d) => {
                    assert_eq!(d.code(), code);
                    n += 1;
                }
                Err(_) => assert_eq!(i, 20),
            }
        }
        for code in ["2A", "2B"] {
            assert_eq!(Departement::nouveau(code).unwrap().code(), code);
            n += 1;
        }
        assert_eq!(n, 96);
    }

    #[test]
    fn normalise_la_saisie() {
        assert_eq!(Departement::nouveau("1").unwrap().code(), "01");
        assert_eq!(Departement::nouveau(" 2a ").unwrap().code(), "2A");
    }

    #[test]
    fn refuse_les_codes_hors_liste() {
        for code in ["", "0", "00", "20", "96", "971", "2C", "A1", "é"] {
            assert!(Departement::nouveau(code).is_err(), "{code}");
        }
    }

    #[test]
    fn serialise_en_chaine() {
        let d: Departement = serde_json::from_str("\"2B\"").unwrap();
        assert_eq!(serde_json::to_string(&d).unwrap(), "\"2B\"");
        assert!(serde_json::from_str::<Departement>("\"20\"").is_err());
    }
}
