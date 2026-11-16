// SPDX-License-Identifier: GPL-3.0-or-later
//! Lecture des données météo fournies par l'utilisateur.
//!
//! Le moteur n'embarque aucune donnée météo. Trois formats sont lus, tous vers la même
//! structure interne ([`MeteoProjet`]) ; [`lire`] reconnaît le format d'après le contenu.
//!
//! | Format | Module | Statut des résultats |
//! |---|---|---|
//! | CSV long dérivé du classeur réglementaire | [`csv_re2020`] | réglementaire |
//! | Classeur réglementaire `.xlsx`, 16 onglets `<zone>_<mode>` | [`classeur`] | réglementaire |
//! | Fichiers ouverts ADEME / Météo-France | [`ademe`] | **non réglementaire** |
//!
//! **Les fichiers réglementaires ne doivent jamais être versionnés ni diffusés** : leurs
//! conditions d'utilisation l'interdisent (`docs/reference/README.md`).
//!
//! ## Provenance
//!
//! Un fichier au format réglementaire est tenu pour le fichier réglementaire, sauf un CSV qui
//! porte une ligne de commentaire `# thbce-origine: <description>`. Cette ligne marque les jeux
//! d'essai et toute donnée retravaillée : les résultats portent alors `reglementaire = false`.
//! Le moteur ne peut pas vérifier l'authenticité d'un fichier ; la sincérité de la provenance
//! relève de l'utilisateur.

use thbce_core::climat::MeteoProjet;
use thbce_core::conventions::NB_HEURES_AN;
use thbce_data::ZoneClimatique;

pub mod ademe;
#[cfg(feature = "classeur")]
pub mod classeur;
pub mod csv_re2020;

pub use csv_re2020::{ecrire_csv, lire_csv_re2020, COLONNES, MARQUE_ORIGINE};

/// Erreur de lecture d'un fichier météo.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurMeteo {
    /// Le fichier n'est pas du texte UTF-8.
    #[error("le fichier météo n'est pas un texte UTF-8")]
    Encodage,
    /// L'en-tête ne correspond pas au format attendu.
    #[error("en-tête inattendu « {0} »")]
    EnTete(String),
    /// Une ligne est mal formée.
    #[error("ligne {ligne} : {message}")]
    Ligne {
        /// Numéro de ligne dans le fichier ou l'onglet, à partir de 1.
        ligne: usize,
        /// Nature du défaut.
        message: String,
    },
    /// Le fichier ne contient pas l'année complète demandée.
    #[error("zone {zone}, jeu {jeu} : {lues} heures lues, {NB_HEURES_AN} attendues")]
    Incomplet {
        /// Zone demandée.
        zone: ZoneClimatique,
        /// Jeu concerné.
        jeu: &'static str,
        /// Nombre d'heures trouvées.
        lues: usize,
    },
    /// Le classeur ne peut pas être ouvert ou n'a pas la structure attendue.
    #[error("classeur météo : {0}")]
    Classeur(String),
    /// Le format du fichier n'est pas pris en charge par cette compilation.
    #[error("format de fichier météo non pris en charge : {0}")]
    FormatNonPrisEnCharge(&'static str),
}

/// Format d'un fichier météo, reconnu d'après son contenu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatMeteo {
    /// CSV long dérivé du classeur réglementaire.
    CsvRe2020,
    /// Classeur réglementaire `.xlsx`.
    ClasseurRe2020,
    /// Fichier ouvert ADEME / Météo-France (CSV à point-virgule).
    CsvAdeme,
}

/// Reconnaît le format : archive ZIP pour un classeur, en-tête `STATION;` pour un fichier
/// ADEME, CSV long sinon.
pub fn detecter(octets: &[u8]) -> FormatMeteo {
    let sans_bom = octets.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(octets);
    if octets.starts_with(b"PK\x03\x04") {
        FormatMeteo::ClasseurRe2020
    } else if sans_bom.starts_with(ademe::DEBUT_EN_TETE.as_bytes()) {
        FormatMeteo::CsvAdeme
    } else {
        FormatMeteo::CsvRe2020
    }
}

/// Lit les deux jeux météo d'une zone, quel que soit le format du fichier.
pub fn lire(octets: &[u8], zone: ZoneClimatique) -> Result<MeteoProjet, ErreurMeteo> {
    match detecter(octets) {
        FormatMeteo::CsvRe2020 => lire_csv_re2020(octets, zone),
        FormatMeteo::CsvAdeme => ademe::lire_csv_ademe(octets, zone),
        #[cfg(feature = "classeur")]
        FormatMeteo::ClasseurRe2020 => classeur::lire_classeur_re2020(octets, zone),
        #[cfg(not(feature = "classeur"))]
        FormatMeteo::ClasseurRe2020 => Err(ErreurMeteo::FormatNonPrisEnCharge(
            "classeur .xlsx (fonctionnalité « classeur » désactivée) ; fournir le CSV dérivé",
        )),
    }
}
