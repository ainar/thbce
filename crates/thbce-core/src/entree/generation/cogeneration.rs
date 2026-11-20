// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : système de cogénération (micro-cogénérateur, §8.21).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::entree::*;

/// « type_combustible » d'un cogénérateur et de son appoint intégré (§8.21.3.1.1, tableau 136).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CombustibleCogeneration {
    /// Gaz naturel (PCSI 1,11).
    GazNaturel,
    /// GPL, butane et propane (PCSI 1,09).
    Gpl,
    /// Fioul domestique (PCSI 1,07).
    Fioul,
    /// Bois bûches (PCSI 1,08).
    BoisBuches,
    /// Bois granulés (PCSI 1,08).
    BoisGranules,
    /// Bois plaquettes (PCSI 1,08).
    BoisPlaquettes,
}

/// « type_chp » : technologie du micro-cogénérateur (§8.21.1 et tableau 138).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeCogenerateur {
    /// 0 : moteur à combustion interne.
    MoteurCombustionInterne,
    /// 1 : moteur Stirling.
    Stirling,
    /// 2 : micro-turbine.
    MicroTurbine,
}

/// « Statut_chp100sup0 », « Statut_chp100sup100 » : statut des performances à pleine charge
/// (§8.21.3.1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatutPerformancesCogeneration {
    /// 0 : par défaut : rendements égaux à 0,8 × les valeurs utiles maximales des tableaux 138
    /// et 139, auxiliaires à 1,2 × la valeur utile maximale.
    Defaut,
    /// 1 : déclarée : `min(0,8 × valeur ; valeur utile maximale)` pour les rendements,
    /// `max(1,2 × valeur ; valeur utile maximale)` pour les auxiliaires.
    Declaree,
    /// 2 : justifiée (essai NF EN ISO/CEI 17025) : rendements × 0,9, auxiliaires × 1,1.
    Justifiee,
    /// 3 : certifiée (NF EN ISO/CEI 17065) : valeurs employées telles quelles.
    Certifiee,
}

/// Performances à 100 % de charge du module de cogénération, avec l'appoint intégré à 0 % ou à
/// 100 % de charge (« chp100+sup0 », « chp100+sup100 »).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PerformancesCogeneration {
    /// « Pth;chp_100+sup_x,nom » : puissance thermique nominale, en kW (au plus 70).
    pub p_th_kw: f64,
    /// Statut des rendements et des auxiliaires.
    pub statut: StatutPerformancesCogeneration,
    /// « ηth;chp_100+sup_x,nom » : rendement thermique sur PCS, en % ; requis si le statut n'est
    /// pas « par défaut ».
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta_th_pct: Option<f64>,
    /// « ηel;chp_100+sup_x,nom » : rendement de production électrique sur PCS, en % ; requis si
    /// le statut n'est pas « par défaut ».
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta_el_pct: Option<f64>,
    /// « PWaux,chp_100+sup_x » : puissance électrique des auxiliaires à pleine charge, en kW ;
    /// requise si le statut n'est pas « par défaut ».
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_w_aux_kw: Option<f64>,
}

/// Th-BCE 2020 §8.21.2, tableau 135 — système de cogénération (micro-cogénérateur de débit
/// calorifique inférieur à 70 kW et de puissance électrique inférieure à 50 kW : moteur à
/// combustion interne, moteur Stirling, micro-turbine ; les piles à combustible sont exclues).
/// La position de la génération fixe « idpos_gen ».
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CogenerationSaisie {
    /// « idfougen » : chauffage, ECS, ou chauffage et ECS.
    pub fonction: FonctionChaleur,
    /// « type_combustible ».
    pub combustible: CombustibleCogeneration,
    /// « ind_condensation » : micro-cogénérateur à condensation (tableau 137).
    pub condensation: bool,
    /// « type_chp ».
    pub type_cogenerateur: TypeCogenerateur,
    /// « chp100+sup0 » : module de cogénération seul à pleine charge.
    pub module_seul: PerformancesCogeneration,
    /// « chp100+sup100 » : module et brûleur d'appoint intégré à pleine charge
    /// (« idapp_inte » = 1) ; absent sans appoint intégré.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avec_appoint: Option<PerformancesCogeneration>,
    /// « Pth,chp_min » : puissance thermique minimale de fonctionnement stabilisé du module,
    /// en kW. Absent : 60 % de « Pth;chp100+sup0 » (tableau 140).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_th_min_kw: Option<f64>,
    /// « φth,stab,30 » : pertes en régime stabilisé pour une eau à 50 °C et une ambiance à
    /// 20 °C, en W. Absent : tableau 140.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phi_th_stab_30_w: Option<f64>,
    /// « PWaux,veille » : puissance électrique des auxiliaires en veille, en W. Absent : 15 W
    /// (tableau 140).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_w_aux_veille_w: Option<f64>,
    /// « idraccord_gnr » : raccordement hydraulique à la génération (le tableau 135 ne donne
    /// pas de valeur par défaut) ; raccordé en permanence, le cogénérateur compte ses pertes à
    /// l'arrêt (éq. 1220, lecture Q157).
    pub raccordement: RaccordementGenerateur,
}
