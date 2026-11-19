// SPDX-License-Identifier: GPL-3.0-or-later
//! Moteur de calcul conforme à la méthode Th-BCE 2020 (RE2020).
//!
//! Bibliothèque pure et déterministe : aucune entrée-sortie, aucune horloge, aucun aléa.
//! Les références « Th-BCE 2020 § » renvoient à l'annexe III de l'arrêté du 4 août 2021 modifié,
//! versionnée dans `docs/reference/arrete/`.
//!
//! Périmètre de la version 0.1 : calcul annuel au pas horaire de la maison individuelle ou
//! accolée et du logement collectif, en mode Th-B (besoins de chauffage, de refroidissement et
//! d'éclairage, Bbio), Th-D (confort d'été, DH) et Th-C (consommations des systèmes du projet),
//! puis Cep, Cep,nr et Ic énergie, exigences maximales et conformité
//! ([`indicateurs`]). Les éléments hors du périmètre sont refusés par la validation
//! ([`validation::Probleme::hors_perimetre`]). L'avancement fiche par fiche est tenu dans
//! `docs/couverture.md`.
//!
//! Le format JSON d'entrée et de résultats, la ligne de commande et l'API JavaScript sont
//! considérés comme stables ; l'API Rust de cette bibliothèque ne l'est pas. La version 1.0.0
//! sera publiée après validation contre le moteur réglementaire COMETH, au moyen d'un logiciel
//! certifié ; les résultats peuvent changer à la suite de cette validation.
//!
//! Ce moteur n'est pas le moteur réglementaire officiel et ne vaut pas attestation de conformité.

pub mod acv;
pub mod bati;
pub mod calcul;
pub mod chauffage_refroidissement;
pub mod climat;
pub mod confort;
pub mod conventions;
pub mod deplacements;
pub mod eclairage;
pub mod ecs;
pub mod entree;
pub mod erreur;
pub mod indicateurs;
pub mod pertes;
pub mod pv;
pub mod resultats;
pub mod scenarios;
pub mod validation;
pub mod ventilation;

pub use calcul::{
    calculer, calculer_avec_series, series_horaires, series_pv, Options, SerieHoraireGroupe,
    SeriePvBatiment,
};
pub use erreur::Erreur;
pub use validation::{valider, RapportValidation};

/// Version du moteur (celle du workspace Cargo).
pub const VERSION_MOTEUR: &str = env!("CARGO_PKG_VERSION");

/// Version de la méthode implémentée.
pub const VERSION_METHODE: &str =
    "Th-BCE 2020 — annexe III de l'arrêté du 4 août 2021, version consolidée au 19 mars 2026";
