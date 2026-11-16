// SPDX-License-Identifier: GPL-3.0-or-later
//! Moteur de calcul conforme à la méthode Th-BCE 2020 (RE2020).
//!
//! Bibliothèque pure et déterministe : aucune entrée-sortie, aucune horloge, aucun aléa.
//! Les références « Th-BCE 2020 § » renvoient à l'annexe III de l'arrêté du 4 août 2021 modifié,
//! versionnée dans `docs/reference/arrete/`.
//!
//! État : squelette (phase 2). Les types d'entrée, la validation et la structure des résultats
//! sont en place ; les indicateurs réglementaires ne sont pas encore calculés et valent `None`.
//! L'avancement fiche par fiche est tenu dans `docs/couverture.md`.
//!
//! Ce moteur n'est pas le moteur réglementaire officiel et ne vaut pas attestation de conformité.

pub mod bati;
pub mod calcul;
pub mod climat;
pub mod confort;
pub mod conventions;
pub mod eclairage;
pub mod entree;
pub mod erreur;
pub mod resultats;
pub mod scenarios;
pub mod validation;
pub mod ventilation;

pub use calcul::{calculer, Options};
pub use erreur::Erreur;
pub use validation::{valider, RapportValidation};

/// Version du moteur (celle du workspace Cargo).
pub const VERSION_MOTEUR: &str = env!("CARGO_PKG_VERSION");

/// Version de la méthode implémentée.
pub const VERSION_METHODE: &str =
    "Th-BCE 2020 — annexe III de l'arrêté du 4 août 2021, version consolidée au 19 mars 2026";
