// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : chaudières au gaz, au fioul ou au bois (§8.19), poêles et inserts (§8.22).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::entree::*;

/// Déclinaison d'une chaudière (§8.19.3.1, tableau 115).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeChaudiereSaisi {
    /// Type 100 : chaudière standard au gaz.
    GazStandard,
    /// Type 101 : chaudière basse température au gaz (traitée comme une chaudière standard,
    /// le tableau 115 n'a pas de ligne propre : Q49).
    GazBasseTemperature,
    /// Type 102 : chaudière à condensation au gaz.
    GazCondensation,
    /// Type 200 : chaudière standard au fioul.
    FioulStandard,
    /// Type 201 : chaudière à condensation au fioul.
    FioulCondensation,
    /// Type 400 : chaudière au bois (`combustible_bois` requis).
    Bois,
}

/// « TypeCombustibleBois » (§8.19.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CombustibleBoisSaisi {
    /// 1 : bûches.
    Buches,
    /// 2 : granulés.
    Granules,
    /// 3 : plaquettes.
    Plaquettes,
}

/// « Alim_Chaudière_Bois » (§8.19.2, tableau 119).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AlimentationBoisSaisie {
    /// 0 : chargement manuel, tirage naturel.
    ManuelTirageNaturel,
    /// 1 : chargement manuel, air pulsé.
    ManuelAirPulse,
    /// 2 : chargement automatique, tirage naturel.
    AutomatiqueTirageNaturel,
    /// 3 : chargement automatique, air pulsé.
    AutomatiqueAirPulse,
}

/// Statut des rendements d'une chaudière (§8.19.3.1) : une valeur certifiée est employée telle
/// quelle, une valeur justifiée est réduite de 10 %, une valeur déclarée de 20 %.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatutRendementChaudiere {
    /// Certifié (NF EN ISO/CEI 17065).
    Certifie,
    /// Justifié par un essai (NF EN ISO/CEI 17025) : réduction de 10 %.
    Justifie,
    /// Déclaré par le fabricant : réduction de 20 %.
    Declare,
}

/// Rendements d'une chaudière sur PCS : `R_pn`, `R_pint` et `Pint`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RendementsChaudiere {
    /// Statut des deux rendements.
    pub statut: StatutRendementChaudiere,
    /// « R_pn » : rendement sur PCS à la puissance nominale en régime haute température, en %.
    pub r_pn_pct: f64,
    /// « R_pint » : rendement sur PCS à 30 % de charge en régime basse température, en %.
    pub r_pint_pct: f64,
    /// « Pint » : puissance thermique utile à 30 % de charge, en kW. Absent : 30 % de `Pn_gen`
    /// (valeur par défaut signalée).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_int_kw: Option<f64>,
}

/// Auxiliaires d'une chaudière, en W ; chaque valeur absente prend sa valeur par défaut
/// (éq. 1075 à 1077), signalée dans les résultats.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuxiliairesChaudiere {
    /// « Waux_nom » : puissance électrique absorbée à pleine charge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w_aux_nom_w: Option<f64>,
    /// « Waux_int » : puissance électrique absorbée à charge intermédiaire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w_aux_int_w: Option<f64>,
    /// « Wveille » : puissance électrique absorbée à charge nulle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w_veille_w: Option<f64>,
}

/// « idpertes_parois » (§8.19.3.5, tableau 121).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PertesParoisChaudiere {
    /// 1 : absence de ventilateur ou autre dispositif de circulation d'air ou de fumées
    /// (`p_Qp.g_arret` = 0,50).
    SansVentilateur,
    /// 2 : présence d'un ventilateur (0,75).
    AvecVentilateur,
    /// 3 : présence d'un clapet sur le conduit de fumées (1).
    AvecClapet,
}

/// Th-BCE 2020 §8.19.2, tableau 114 — chaudière au gaz, au fioul ou au bois. La position de la
/// génération (`position`) fixe « idpos_gen ».
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChaudiereSaisie {
    /// Déclinaison de la chaudière.
    pub type_chaudiere: TypeChaudiereSaisi,
    /// « TypeCombustibleBois » : requis pour une chaudière au bois.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combustible_bois: Option<CombustibleBoisSaisi>,
    /// Gaz de pétrole liquéfié (PCSI de 1,09 au lieu de 1,11, tableau 116) ; gaz naturel
    /// sinon. Le GPL relève des « autres énergies » de l'article 9 et du facteur d'émission
    /// « gaz butane / propane » de l'article 10 (lot 7).
    #[serde(default)]
    pub gpl: bool,
    /// « Id_fou_gen » : fonction du générateur.
    pub fonction: FonctionChaleur,
    /// « Pn_gen » : puissance thermique utile nominale en régime haute température, en kW.
    pub pn_gen_kw: f64,
    /// Rendements mesurés ; absents : valeurs par défaut (éq. 1072 et 1073).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendements: Option<RendementsChaudiere>,
    /// « Alim_Chaudière_Bois » : requis pour une chaudière au bois dont les auxiliaires sont
    /// par défaut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alimentation_bois: Option<AlimentationBoisSaisie>,
    /// Auxiliaires.
    #[serde(default)]
    pub auxiliaires: AuxiliairesChaudiere,
    /// « Q_po_30 » : pertes thermiques en régime stabilisé pour un écart de 30 K, en W ; absent :
    /// valeur par défaut (éq. 1074).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_po_30_w: Option<f64>,
    /// « idpertes_parois ».
    pub pertes_parois: PertesParoisChaudiere,
}

/// Th-BCE 2020 §8.22.2, tableau 141 — poêle ou insert à bois (chauffage seul).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PoeleSaisi {
    /// « Pngen » : puissance nominale d'un appareil, en kW (de 0 exclu à 100).
    pub pn_gen_kw: f64,
    /// « ηH,sys,n » : rendement moyen selon la norme de l'appareil, en %.
    pub eta_pct: f64,
    /// « Paux,vent » : puissance du ventilateur, en W (0 sans ventilateur).
    pub p_aux_vent_w: f64,
    /// « Type_Combustible_Bois » (§13.2.2) : combustible, qui fixe le facteur d'émission du bois
    /// dans l'Ic énergie (article 10 de l'arrêté). Obligatoire, comme pour une chaudière au bois.
    pub combustible_bois: CombustibleBoisSaisi,
}
