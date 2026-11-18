// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats de la ventilation, par zone et par groupe.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ValeurParDefaut;
use crate::entree::Id;

/// Système de ventilation du projet d'une zone (Th-BCE 2020 §6.2 à 6.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsVentilationZone {
    /// Identifiant du système.
    pub id: Id,
    /// Puissance moyenne des ventilateurs, en W (éq. 391 à 394, 620, 641 et 643), constante
    /// en habitation.
    pub p_vent_w: f64,
    /// Consommation annuelle des ventilateurs, en kWh d'énergie finale électrique : la
    /// puissance moyenne sommée heure par heure (éq. 565, 621 et 645).
    pub c_aux_vent_kwh: f64,
    /// Fonctionnement du système dans le calcul Th-D ; absent si le calcul Th-D n'emploie pas
    /// le système du projet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub th_d: Option<ResultatsVentilationMode>,
    /// Fonctionnement du système dans le calcul Th-C ; l'énergie de la sécurité antigel y est
    /// demandée à la génération désignée par `generation_antigel`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub th_c: Option<ResultatsVentilationMode>,
    /// Valeurs par défaut de la méthode appliquées au système et à ses bouches ; champ relatif
    /// à la zone.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// Fonctionnement annuel du système de ventilation du projet dans un calcul.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsVentilationMode {
    /// Nombre d'heures où l'échangeur d'un double flux est contourné par son bypass (éq. 494).
    pub nbh_bypass: u32,
    /// Énergie fournie pour la sécurité antigel de l'échangeur, en kWh (éq. 508).
    pub q_ag_kwh: f64,
    /// Énergie fournie par la batterie de préchauffage, en kWh (éq. 523) ; absente sans
    /// batterie. Demandée à la génération désignée en calcul des consommations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_prechaud_kwh: Option<f64>,
    /// Énergie fournie par la batterie de prérefroidissement, en kWh, négative (éq. 545,
    /// lecture Q91) ; absente sans batterie.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_prefroid_kwh: Option<f64>,
}

/// Ventilation du projet vue d'un groupe (Th-BCE 2020 §6.2 à 6.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsVentilationGroupe {
    /// « qspec_repris » : débit spécifique repris au groupe, fuites en volume chauffé
    /// comprises, en m³/h, négatif (éq. 428).
    pub q_spec_repris_m3_h: f64,
    /// « qspec_soufflé » : débit spécifique soufflé dans le groupe, fuites en volume chauffé
    /// comprises, en m³/h (éq. 429).
    pub q_spec_souffle_m3_h: f64,
    /// « qrepris,cond » : débit repris vu par le ventilateur, en m³/h, négatif (éq. 430).
    pub q_repris_cond_m3_h: f64,
    /// « qsoufflé,cond » : débit soufflé vu par le ventilateur, en m³/h (éq. 431).
    pub q_souffle_cond_m3_h: f64,
    /// Puissance des ventilateurs attribuée au groupe, en W (éq. 570, 642 et 644).
    pub p_vent_w: f64,
    /// Consommation annuelle des auxiliaires de ventilation du groupe, en kWh d'énergie finale
    /// électrique (« Waux,v^gr », éq. 570 ; « Cvent^g », éq. 646).
    pub c_aux_vent_kwh: f64,
    /// La même, par m² de surface de référence du groupe, en kWh/m².
    pub c_aux_vent_kwh_m2: f64,
    /// Consommations mensuelles, en kWh, mois au sens des scénarios (§4.1.3).
    pub c_aux_vent_mensuel_kwh: Vec<f64>,
}

/// Débits spécifiques d'un groupe pour le calcul du Bbio (Th-BCE 2020 §6.1 et §6.2), constants
/// sur l'année en habitation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsDebitsBbio {
    /// « qspec_repris » : débit volumique spécifique repris, en m³/h, négatif (§6.2.3.2.2.2,
    /// éq. 403, avec `Crdbnr = 1`, `Dugd = 14` h par semaine et `Cdep = 1`).
    pub q_spec_repris_m3_h: f64,
    /// « qspec_soufflé » : débit volumique spécifique soufflé, en m³/h, positif.
    pub q_spec_souffle_m3_h: f64,
}
