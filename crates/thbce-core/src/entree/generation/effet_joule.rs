// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : générateurs directs à effet Joule (§8.18).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Th-BCE 2020 §8.18.2, tableau 112 — générateur direct à effet Joule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EffetJouleSaisi {
    /// « idfougen » : fonction du générateur, chauffage ou ECS ; jamais les deux (§8.18.3.1).
    pub fonction: FonctionEffetJoule,
    /// « Pngen » : puissance nominale d'un générateur, en W.
    pub pn_gen_w: f64,
}

/// « idfougen » d'un générateur direct à effet Joule (§8.18.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FonctionEffetJoule {
    /// 1 : chauffage.
    Chauffage,
    /// 3 : production instantanée d'ECS (§9.1).
    Ecs,
}
