// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : brasseurs d'air fixés au plafond (Th-BCE 2020 §8.32, lot 1-emi).
//!
//! Un composant est créé pour chaque type de brasseur (§8.32.1) ; les types d'un groupe sont
//! saisis dans `Groupe::brasseurs_air`. Les brasseurs ne s'introduisent que dans des groupes non
//! climatisés (§8.32.3.1). Lectures : `docs/questions/Q120.md`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::*;

/// Caractère de la valeur d'un paramètre de brasseur (§8.32.3.2.1) : le texte corrige la valeur
/// saisie selon qu'elle est certifiée, justifiée ou déclarée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CaractereValeurBrasseur {
    /// Valeur certifiée : appliquée telle quelle.
    Certifiee,
    /// Valeur justifiée : puissance majorée de 10 %, débit minoré de 10 %.
    Justifiee,
    /// Valeur déclarée : puissance majorée de 20 %, débit minoré de 20 %.
    Declaree,
}

/// « Usp » : usage « jour » ou « nuit » d'un type de brasseur (§8.32.3.2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UsageBrasseur {
    /// 0 : usage nuit (de 22 h à 6 h légales).
    Nuit,
    /// 1 : usage jour (de 6 h à 22 h légales).
    Jour,
    /// 2 : jour et nuit, pour les locaux qui ne distinguent pas espaces « jour » et espaces
    /// « nuit » (studios, petits appartements) ; aucun autre usage ne peut alors être saisi dans
    /// le groupe.
    JourEtNuit,
}

/// « mode_gestion_br_p » (§8.32.3.2.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GestionBrasseur {
    /// 1 : gestion manuelle de la vitesse par les occupants ; consignes conventionnelles
    /// (tableau 258, éq. 1646 et 1648).
    Manuelle,
    /// 2 : gestion automatique avec thermostats ; les écarts de consigne sont ceux du fabricant
    /// (éq. 1647 et 1649).
    AutomatiqueThermostat(ParametresHysteresisBrasseur),
    /// 3 : gestion toute automatique, sans ajustement manuel des consignes (éq. 1650) ; la
    /// température de déclenchement est propre au système.
    Automatique(ParametresHysteresisBrasseurAutomatique),
}

/// Écarts de l'hystérésis d'une gestion automatique avec thermostats (« Δθop_1_br_p_th_aut »,
/// « Δθop_2_br_p_th_aut », « Δθop_3_br_p_th_aut »), en K.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParametresHysteresisBrasseur {
    /// « Δθop_1 » : écart entre la température d'arrêt et la température de déclenchement, en K.
    pub delta_theta_op_1_k: f64,
    /// « Δθop_2 » : écart entre le passage à la vitesse intermédiaire et le passage à la vitesse
    /// maximale, en K.
    pub delta_theta_op_2_k: f64,
    /// « Δθop_3 » : écart de la température de déclenchement à la consigne de refroidissement
    /// corrigée, en K.
    pub delta_theta_op_3_k: f64,
}

/// Paramètres d'une gestion toute automatique (éq. 1650).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParametresHysteresisBrasseurAutomatique {
    /// « θop_dec_br_p_aut » : température opérative de déclenchement, en °C.
    pub theta_op_dec_c: f64,
    /// « Δθop_1_br_p_aut », en K.
    pub delta_theta_op_1_k: f64,
    /// « Δθop_2_br_p_aut », en K.
    pub delta_theta_op_2_k: f64,
    /// « Δθop_3_br_p_aut », en K ; 0 par défaut (§8.32.3.2.4.3), valeur signalée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_theta_op_3_k: Option<f64>,
}

/// Th-BCE 2020 §8.32 — type de brasseur d'air et ensemble des brasseurs de ce type installés
/// dans un groupe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BrasseurAir {
    /// Identifiant du type de brasseur, unique dans le groupe.
    pub id: Id,
    /// « Nbr_p » : nombre de brasseurs de ce type dans le groupe.
    pub nombre: u32,
    /// « Ratbr_p » : part de la surface utile du groupe, au titre des pièces cloisonnées
    /// couvertes par les brasseurs de ce type (0 à 1) ; ramenée à `15·Nbr_p/Agr` au plus
    /// (une surface de 15 m² par brasseur, éq. de §8.32.3.2.3).
    pub ratio_surface_brassee: f64,
    /// « Usp » : usage jour, nuit, ou jour et nuit.
    pub usage: UsageBrasseur,
    /// « RatUs_BA_p » : part de la surface utile occupée pendant l'usage jour ou nuit du type.
    /// Absent : valeur par défaut du tableau 257 selon l'usage de la zone (1 pour un usage
    /// « jour et nuit »), signalée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ratio_surface_occupee: Option<f64>,
    /// « Pelec_br_p_max » : puissance électrique individuelle maximale, en W.
    pub p_elec_max_w: f64,
    /// « type_Pelec_br_p_max » : caractère de la puissance maximale.
    pub p_elec_max_caractere: CaractereValeurBrasseur,
    /// « Qvair_br_p_max » : débit volumique individuel maximal, en m³/h.
    pub q_v_max_m3_h: f64,
    /// « type_Qvair_br_p_max » : caractère du débit maximal.
    pub q_v_max_caractere: CaractereValeurBrasseur,
    /// « mode_gestion_br_p » : gestion de la vitesse.
    pub gestion: GestionBrasseur,
}
