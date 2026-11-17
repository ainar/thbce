// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : espaces tampons (vérandas, locaux non chauffés).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::*;

/// Th-BCE 2020 §5.4 — espace tampon.
///
/// « Un espace tampon est défini au niveau du bâtiment, et doit nécessairement être associé à au
/// moins un composant d'enveloppe (paroi opaque, baie) d'un groupe » (§5.4.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EspaceTampon {
    /// Identifiant de l'espace tampon, unique dans le bâtiment.
    pub id: Id,
    /// Nature de l'espace tampon (« isolarise »).
    pub nature: NatureEspaceTampon,
}

/// « isolarise » : espace tampon solarisé ou non (§5.4.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum NatureEspaceTampon {
    /// Espace non solarisé (`isolarise = 0`) : combles, garages, celliers non chauffés,
    /// parkings privés, circulations hors volume chauffé, chaufferies.
    NonSolarise(EspaceTamponNonSolarise),
    /// Espace solarisé (`isolarise = 1`) : l'essentiel de son enveloppe en contact avec l'air
    /// extérieur, hors plancher, est vitré (vérandas, serres, atriums, jardins d'hiver).
    Solarise(EspaceTamponSolarise),
}

/// Th-BCE 2020 §5.4.2, tableau 19 — espace tampon non solarisé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EspaceTamponNonSolarise {
    /// « bet_ns » : coefficient `b` de réduction des déperditions, tel que défini dans les
    /// règles Th-Bat (de −5 à 1).
    pub b_et_ns: f64,
}

/// Th-BCE 2020 §5.2.1 et §5.4.2, tableaux 14 et 19 — espace tampon solarisé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EspaceTamponSolarise {
    /// « Aet » : surface au sol, en m².
    pub a_et_m2: f64,
    /// « Uet_pb » : coefficient de déperdition équivalent par le plancher bas, en W/(m².K).
    pub u_et_pb_w_m2k: f64,
    /// « Httfet » : hauteur de tirage thermique en surventilation naturelle, en m.
    pub h_ttf_et_m: f64,
    /// « Cm_et_surf » : capacité thermique quotidienne par m² de surface au sol, en
    /// kJ/(m².K). Absente : 200, valeur conventionnelle du tableau 19, signalée dans les
    /// résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cm_et_surf_kj_m2k: Option<f64>,
    /// « Qv,et,base » : débit d'air de base hors surventilation (défauts d'étanchéité et
    /// entrées d'air), en m³/h. Absent : 1 m³/h par m² de surface au sol (§5.4.3.3.1.4),
    /// signalé dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qv_et_base_m3_h: Option<f64>,
    /// Protections mobiles des parois vitrées (« idprot »). Absent : pas de protection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protections_mobiles: Option<ProtectionsEspaceTampon>,
    /// Parois de l'espace tampon donnant sur l'extérieur (au moins une).
    pub parois: Vec<ParoiEspaceTampon>,
}

/// Th-BCE 2020 §5.3.2, tableau 15 — protections mobiles des parois d'un espace tampon :
/// « conventionnellement de type stores enroulables intérieurs ou extérieurs » (§5.3.3.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProtectionsEspaceTampon {
    /// « Protet_ext » : protections extérieures (vrai) ou intérieures (faux).
    pub exterieures: bool,
}

/// Th-BCE 2020 §5.4.2, tableau 19 — paroi d'un espace tampon solarisé donnant sur
/// l'extérieur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParoiEspaceTampon {
    /// Identifiant de la paroi.
    pub id: Id,
    /// « Aue » : surface, en m².
    pub a_ue_m2: f64,
    /// « Uue » : coefficient de transmission thermique, en W/(m².K).
    pub u_ue_w_m2k: f64,
    /// « αwe », « βwe » et masques : orientation, inclinaison et masques de la paroi
    /// (fiche « Environnement proche », §3.2).
    pub exposition: Exposition,
    /// « Swe_sp » : facteur solaire global sans protection mobile.
    pub swe_sp: f64,
    /// « Swe_ap » : facteur solaire global avec protection mobile en place. Requis si l'espace
    /// tampon a des protections mobiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swe_ap: Option<f64>,
    /// « Tle_sp » : transmission lumineuse globale sans protection mobile.
    pub tle_sp: f64,
    /// « Tle_ap » : transmission lumineuse globale avec protection mobile en place. Requise si
    /// l'espace tampon a des protections mobiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tle_ap: Option<f64>,
    /// « Rouv_max » : ratio d'ouverture maximale de la paroi (0 pour une paroi fixe).
    pub r_ouv_max: f64,
}
