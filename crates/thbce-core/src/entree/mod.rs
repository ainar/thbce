// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : la description du projet telle que l'utilisateur la fournit.
//!
//! Chaque champ vient d'une nomenclature de l'annexe III, citée en commentaire, et en reprend
//! le nom en `snake_case` suffixé par l'unité. Aucun champ n'est ajouté s'il ne figure pas dans
//! le texte ; les fiches non encore lues en détail (systèmes, stores vénitiens) complèteront ces
//! types au moment de leur implémentation.
//!
//! La méthode distingue les paramètres intrinsèques d'un composant de ses paramètres
//! d'intégration dans le projet (§1.2). Les premiers sont décrits une fois dans le
//! [`Catalogue`] ; les seconds sont portés par les instances, qui référencent le catalogue par
//! identifiant.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thbce_data::{Departement, ZoneClimatique};

// Un fichier par famille ; chaque type est réexporté ici, de sorte que le chemin
// `crate::entree::Type` reste celui de l'API. Un lot ajoute ses types dans le fichier de sa
// famille, et une nouvelle famille par un `mod` et un `pub use` ci-dessous.
mod acv;
mod baies;
mod batiment;
mod brasseurs_air;
mod chauffage_vefa;
mod deplacements;
mod eclairage;
mod ecs;
mod emission_distribution;
mod enveloppe;
mod espaces_tampons;
mod exigences;
mod generation;
mod solaire;
mod usages;
mod ventilation;

pub use acv::*;
pub use baies::*;
pub use batiment::*;
pub use brasseurs_air::*;
pub use chauffage_vefa::*;
pub use deplacements::*;
pub use eclairage::*;
pub use ecs::*;
pub use emission_distribution::*;
pub use enveloppe::*;
pub use espaces_tampons::*;
pub use exigences::*;
pub use generation::*;
pub use solaire::*;
pub use usages::*;
pub use ventilation::*;

/// Identifiant textuel choisi par l'utilisateur, unique parmi les objets de même nature.
pub type Id = String;

/// Version du format d'entrée décrite par ce module (champ `version_format` d'un projet).
pub const VERSION_FORMAT: u32 = 1;

/// Niveau « projet » : données communes à tous les bâtiments (Th-BCE 2020 §5.1.1, note 1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Projet {
    /// Adresse du JSON Schema du format (`thbce schema`), pour l'aide à la saisie des éditeurs.
    /// Facultative ; ignorée par le moteur.
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// Version du format d'entrée. Facultative ; si elle est donnée, elle doit valoir
    /// [`VERSION_FORMAT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_format: Option<u32>,
    /// Nom libre du projet.
    pub nom: String,
    /// Situation géographique.
    pub localisation: Localisation,
    /// Composants décrits par leurs paramètres intrinsèques.
    #[serde(default)]
    pub catalogue: Catalogue,
    /// Bâtiments du projet.
    pub batiments: Vec<Batiment>,
}

/// Th-BCE 2020 §3.1.2 et §5.1.2 — paramètres d'intégration du climat et du bâtiment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Localisation {
    /// « département ». Fixe la zone climatique et la température de base.
    pub departement: Departement,
    /// « Alt » : altitude du projet, en m.
    pub alt_m: f64,
    /// « Dmer » : distance à la mer, en km.
    pub dmer_km: f64,
    /// « Durb » : distance à l'îlot urbain, en km.
    pub durb_km: f64,
    /// Zone climatique saisie explicitement. Facultative : le moteur la déduit du département
    /// (arrêté du 26 octobre 2010, annexe I). Une valeur qui contredit le département est
    /// conservée et signalée par un avertissement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone_climatique: Option<ZoneClimatique>,
}

impl Localisation {
    /// Zone climatique retenue : celle saisie, sinon celle du département.
    pub fn zone_climatique_retenue(&self) -> ZoneClimatique {
        self.zone_climatique
            .unwrap_or_else(|| ZoneClimatique::du_departement(self.departement))
    }
}

/// Composants décrits une fois par leurs paramètres intrinsèques.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    /// Types de parois opaques.
    #[serde(default)]
    pub parois: Vec<ParoiType>,
    /// Types de baies vitrées.
    #[serde(default)]
    pub baies: Vec<BaieType>,
    /// Masques solaires.
    #[serde(default)]
    pub masques: Vec<MasqueDefinition>,
}
