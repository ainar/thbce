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
    /// Nombre d'heures de fonctionnement en mode actif d'une centrale à charge variable
    /// pourvue de batteries régulées sur la charge (§6.3.3.5) ; absent sans ces batteries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nbh_mode_actif: Option<u32>,
    /// Nombre d'heures de sous-dimensionnement de ces batteries (soufflage requis au-delà de
    /// 60 °C ou en deçà de la surface de la batterie froide, éq. 597 et 605) ; absent sans elles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nbh_sous_dimensionnement: Option<u32>,
    /// Plus longue série d'heures consécutives de sous-dimensionnement de ces batteries
    /// (« Nbsousdim_DAV », éq. 597 et 605) ; au-delà de 6 heures, un avertissement invite à
    /// revoir le dimensionnement ; absente sans ces batteries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nbh_sous_dimensionnement_consecutives: Option<u32>,
    /// Énergie fournie par la batterie chaude régulée sur la charge, en kWh (« Qchaud(rec) »,
    /// éq. 599) ; absente sans elle. Demandée à la génération désignée (éq. 940).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_chaud_kwh: Option<f64>,
    /// Énergie fournie par la batterie froide régulée sur la charge, latente comprise, en kWh,
    /// négative (« Qfroid(rec) », éq. 611) ; absente sans elle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_froid_kwh: Option<f64>,
    /// Énergie de l'humidificateur d'hiver, en kWh (« QHR », éq. 529 ; nulle pour un
    /// humidificateur à pulvérisation, éq. 533) ; absente sans humidificateur. Demandée à la
    /// génération désignée pour un humidificateur à vapeur (éq. 939).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_hr_kwh: Option<f64>,
    /// Consommation de la pompe de l'humidificateur d'hiver, en kWh (« Waux,chaudHR », éq.
    /// 568) ; absente sans humidificateur. Comprise dans `c_aux_vent_kwh`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w_aux_chaudhr_kwh: Option<f64>,
    /// Nombre d'heures où le rafraîchissement évaporatif est autorisé (« Aut_adiab = 1 »,
    /// §6.3.3.4.2) ; absent sans rafraîchissement évaporatif.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nbh_adiab: Option<u32>,
    /// Consommation de la pompe du rafraîchissement évaporatif, en kWh (« Waux,adiab », éq.
    /// 566 et 617) ; absente sans rafraîchissement évaporatif. Comprise dans `c_aux_vent_kwh`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w_aux_adiab_kwh: Option<f64>,
    /// Nombre d'heures où le rafraîchissement par surventilation mécanique est en service
    /// (« irafnoc_activé(h) = 1 », §6.12.3.3, éq. 766) ; absent sans surventilation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nbh_rafnoc: Option<u32>,
    /// Fonctionnement du puits climatique de l'air neuf (fiches 6.8, 6.10 et 6.11) ; absent
    /// sans puits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub puits_climatique: Option<ResultatsPuitsClimatique>,
    /// Fonctionnement du puits hydraulique de l'air neuf (fiche 6.9) ; absent sans puits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub puits_hydraulique: Option<ResultatsPuitsHydraulique>,
    /// Fonctionnement d'une ventilation naturelle par conduits ou hybride (fiche 6.6) ; absent
    /// pour les autres systèmes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub naturelle: Option<ResultatsNaturelle>,
}

/// Fonctionnement annuel d'une ventilation naturelle par conduits ou hybride dans un calcul. Les
/// débits spécifiques moyens du groupe desservi sont ceux de `groupes[].ventilation`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsNaturelle {
    /// Nombre d'heures d'extraction par le conduit (« qv,cond > 0 », éq. 701 et 702), hors
    /// assistance mécanique.
    pub nbh_extraction: u32,
    /// Nombre d'heures de refoulement dans le conduit (« qv,cond < 0 », éq. 704), hors
    /// assistance mécanique.
    pub nbh_refoulement: u32,
    /// Nombre d'heures de fonctionnement de l'assistance mécanique, grand débit compris
    /// (éq. 706) ; absent en ventilation naturelle seule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nbh_assistance: Option<u32>,
    /// Nombre d'heures de grand débit de la cuisine (tableau 67) ; absent en ventilation
    /// naturelle seule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nbh_grand_debit: Option<u32>,
    /// Consommation du ventilateur d'assistance, en kWh (« Cvent », éq. 708), comprise dans
    /// `c_aux_vent_kwh` ; absente en ventilation naturelle seule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w_assistance_kwh: Option<f64>,
}

/// Fonctionnement annuel d'un puits hydraulique dans un calcul.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsPuitsHydraulique {
    /// Nombre d'heures où la pompe tourne (« QPH(h) > 0 », éq. 746 et 747) et où l'air neuf
    /// traverse la batterie (débit d'air neuf non nul).
    pub nbh_actif: u32,
    /// Énergie apportée à l'air neuf en préchauffage, en kWh : somme positive de
    /// `Cpa × qm,air_neuf × (θan − θext)` (éq. 415).
    pub q_prechauffage_kwh: f64,
    /// Énergie retirée à l'air neuf en prérafraîchissement, en kWh, négative.
    pub q_prerefroidissement_kwh: f64,
    /// Consommation électrique de la pompe, en kWh (éq. 748 et 749), comptée dans les
    /// auxiliaires de ventilation.
    pub w_pompe_kwh: f64,
}

/// Fonctionnement annuel d'un puits climatique dans un calcul.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsPuitsClimatique {
    /// Nombre d'heures où l'air neuf traverse le puits : non contourné (éq. 750 à 752) et
    /// débit d'air neuf non nul.
    pub nbh_actif: u32,
    /// Énergie apportée à l'air neuf en préchauffage, en kWh : somme positive de
    /// `Cpa × qm,air_neuf × (θan − θext)` (éq. 731, 728).
    pub q_prechauffage_kwh: f64,
    /// Énergie retirée à l'air neuf en prérafraîchissement, en kWh, négative.
    pub q_prerefroidissement_kwh: f64,
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
