// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : sous-stations de réseaux de chaleur ou de froid (§8.28).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::entree::*;

/// Type d'un réseau de chaleur (§8.28.3.1, tableaux 247 et 248).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeReseauChaleurSaisi {
    /// Eau chaude basse température (primaire à 105 °C).
    EauChaudeBasseTemperature,
    /// Eau chaude haute température (150 °C).
    EauChaudeHauteTemperature,
    /// Vapeur basse pression (110 °C).
    VapeurBassePression,
    /// Vapeur haute pression (180 °C).
    VapeurHautePression,
}

/// Th-BCE 2020 §8.28.2, tableau 246 — sous-station d'un réseau de chaleur (type 600) ou de
/// froid (type 601).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ReseauChaleurFroidSaisi {
    /// Réseau de chaleur.
    Chaleur {
        /// Identifiant du réseau dans l'annexe 7 de l'arrêté du 15 septembre 2006 (quatre
        /// chiffres suivis de « C »), qui donne le facteur d'émission et le taux d'énergie
        /// renouvelable et de récupération (lot 7). Absent : lignes « autres réseaux de
        /// chaleur » de l'annexe.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        identifiant_reseau: Option<String>,
        /// « idfougen » : chauffage, ECS ou les deux.
        fonction: FonctionChaleur,
        /// Type de réseau.
        type_reseau: TypeReseauChaleurSaisi,
        /// Classe d'isolation (EN ISO 12241) du réseau secondaire, de 1 à 4 ; le primaire est
        /// isolé d'une classe de plus (§8.28.3.1).
        classe_isolation_secondaire: u8,
        /// « PEss » : puissance d'échange d'une sous-station dans les conditions nominales, en
        /// kW.
        p_ess_kw: f64,
    },
    /// Réseau de froid (refroidissement seul, pertes de sous-station nulles : éq. 1483).
    Froid {
        /// Identifiant du réseau dans l'annexe 7 (quatre chiffres suivis de « F »).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        identifiant_reseau: Option<String>,
        /// « PEss » : puissance d'échange d'une sous-station, en kW.
        p_ess_kw: f64,
    },
}
