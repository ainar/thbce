// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : système de chauffage par défaut à effet Joule des travaux réservés par
//! l'acquéreur d'un immeuble vendu en l'état futur d'achèvement (Th-BCE 2020 §8.33).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Id;

/// Th-BCE 2020 §8.33 — chauffage par défaut d'un logement livré sans générateur de chauffage,
/// lorsque le chauffage relève des travaux réservés par l'acquéreur (arrêté du 28 octobre 2019).
///
/// Le moteur ajoute au bâtiment les composants figés par la fiche (tableaux 261 à 266) : une
/// génération à un générateur direct à effet Joule de type 500, un réseau intergroupes fictif
/// sans pertes, et dans chaque groupe desservi une distribution fictive et un convecteur à
/// soufflage d'air chaud. Seule la puissance est saisie (« seul le choix du dimensionnement en
/// puissance est modifiable par l'utilisateur »), dans le respect de l'article R. 171-11 du
/// code de la construction et de l'habitation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChauffageParDefautVefa {
    /// « Pngen » : puissance nominale du générateur à effet Joule par défaut, en W (tableau 262).
    pub pn_gen_w: f64,
    /// Groupes desservis, chacun livré sans émetteur de chauffage.
    pub groupes: Vec<GroupeVefa>,
}

/// Groupe desservi par le chauffage par défaut (§8.33, tableau 265).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GroupeVefa {
    /// Identifiant du groupe.
    pub groupe: Id,
    /// Hauteur sous plafond des locaux, en m (« Carac_Haut_Plafond » du tableau 265 : moins de
    /// 4 m, de 4 à 6 m, de 6 à 8 m, plus de 8 m), qui fixe la variation spatiale de
    /// l'émetteur.
    pub hsp_m: f64,
}
