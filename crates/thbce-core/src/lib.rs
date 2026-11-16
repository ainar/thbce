// SPDX-License-Identifier: GPL-3.0-or-later
//! Moteur de calcul conforme à la méthode Th-BCE 2020 (RE2020).
//!
//! Bibliothèque pure et déterministe : aucune entrée-sortie, aucune horloge, aucun aléa.
//! Les références « Th-BCE 2020 § » renvoient à l'annexe III de l'arrêté du 4 août 2021 modifié,
//! versionnée dans `docs/reference/arrete/`.
//!
//! État : le calcul annuel au pas horaire est en place pour la maison individuelle et le logement
//! collectif, en mode Th-B (besoins de chauffage, de refroidissement et d'éclairage, Bbio) puis
//! en mode Th-D (confort d'été, indicateur DH), puis en mode Th-C pour les demandes des systèmes
//! de chauffage, de refroidissement et d'ECS et les consommations en énergie finale des
//! générations du projet. Cep, Cep,nr et Ic énergie ne sont pas encore
//! calculés et valent `None`. L'avancement fiche par fiche est tenu dans `docs/couverture.md`.
//!
//! Ce moteur n'est pas le moteur réglementaire officiel et ne vaut pas attestation de conformité.

pub mod bati;
pub mod calcul;
pub mod chauffage_refroidissement;
pub mod climat;
pub mod confort;
pub mod conventions;
pub mod eclairage;
pub mod ecs;
pub mod entree;
pub mod erreur;
pub mod indicateurs;
pub mod pertes;
pub mod resultats;
pub mod scenarios;
pub mod validation;
pub mod ventilation;

pub use calcul::{calculer, series_horaires, Options, SerieHoraireGroupe};
pub use erreur::Erreur;
pub use validation::{valider, RapportValidation};

/// Version du moteur (celle du workspace Cargo).
pub const VERSION_MOTEUR: &str = env!("CARGO_PKG_VERSION");

/// Version de la méthode implémentée.
pub const VERSION_METHODE: &str =
    "Th-BCE 2020 — annexe III de l'arrêté du 4 août 2021, version consolidée au 19 mars 2026";
