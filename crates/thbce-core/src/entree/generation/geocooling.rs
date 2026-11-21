// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : échangeur de géocooling d'une génération de refroidissement (Th-BCE 2020
//! §8.30).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::SourceAmontSaisie;

/// Th-BCE 2020 §8.30.2, tableau 253 — échangeur de géocooling : générateur de refroidissement
/// équivalent, alimenté par une source géothermique sur eau (sondes ou nappe, fiche 8.26), placé
/// en tête d'une génération de froid en cascade (« systématiquement prioritaire par rapport aux
/// autres générateurs de froid »).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeocoolingSaisi {
    /// « qv,aval,nom » : débit d'eau froide nominal de la production d'eau froide, en m³/h.
    pub qv_aval_nom_m3_h: f64,
    /// « Δθaval » : écart départ-retour nominal de la production d'eau froide, en K (0 exclu à
    /// 8).
    pub delta_theta_aval_k: f64,
    /// « qv,amont,nom » : débit nominal côté source amont, en m³/h.
    pub qv_amont_nom_m3_h: f64,
    /// « Δθamont » : écart nominal aux bornes de l'échangeur côté source amont, en K (0 à 8).
    pub delta_theta_amont_k: f64,
    /// « θr,source,lim » : température limite de retour d'eau côté source au-delà de laquelle
    /// le géocooling est désactivé, en °C (0 à 25).
    pub theta_r_source_lim_c: f64,
    /// « Pcprim,nom » : puissance du circulateur propre au circuit primaire de l'échangeur, en W
    /// (0 s'il n'y en a pas).
    pub p_cprim_nom_w: f64,
    /// Échangeur intermédiaire entre le circuit des capteurs et le circuit d'eau du bâtiment
    /// (« Is_Echangeur_Intermediaire = 1 ») ; absent : un seul circuit d'eau
    /// (« Is_Echangeur_Intermediaire = 0 »).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echangeur_intermediaire: Option<EchangeurGeocooling>,
    /// « Type_Association » ; absent : base et complément (valeur conventionnelle du tableau
    /// 253, signalée).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub association: Option<AssociationGeocooling>,
    /// Source amont géothermique sur eau : captage par sondes ou nappe (fiche 8.26).
    pub source_amont: SourceAmontSaisie,
}

/// Échangeur intermédiaire de géocooling (§8.30.3.4).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EchangeurGeocooling {
    /// « UAgeo,nom » : coefficient de transfert de l'échangeur aux débits nominaux, en W/K.
    pub ua_geo_nom_w_k: f64,
}

/// « Type_Association » (tableau 253) : association avec les autres générateurs de froid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssociationGeocooling {
    /// 0 : base et complément : le géocooling refroidit l'eau de quelques degrés, un
    /// générateur de froid complète.
    BaseEtComplement,
    /// 1 : alterné : le géocooling n'est utilisé que s'il couvre seul la demande (éq. 1587).
    Alterne,
}
