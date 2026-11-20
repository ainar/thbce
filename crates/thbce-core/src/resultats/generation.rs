// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats des générations et de leurs générateurs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    est_nul, Consommations, ConsommationsPostes, ResultatsSolaireCollectif, ResultatsStockage,
    ValeurParDefaut,
};
use crate::entree::Id;

/// Génération, calcul Th-C (§8.14, §8.17 et §8.29).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsGeneration {
    /// Identifiant de la génération.
    pub id: Id,
    /// Énergie de chauffage demandée par les réseaux, sécurité antigel des doubles flux
    /// comprise, en kWh (éq. 993).
    pub q_ch_kwh: f64,
    /// Énergie de refroidissement demandée (valeur absolue), en kWh (éq. 994).
    pub q_fr_kwh: f64,
    /// Énergie d'ECS demandée, en kWh (éq. 995).
    pub q_ecs_kwh: f64,
    /// Énergie de chauffage fournie par les générateurs, en kWh.
    pub q_fou_ch_kwh: f64,
    /// Énergie de refroidissement fournie, en kWh.
    pub q_fou_fr_kwh: f64,
    /// Énergie d'ECS fournie, en kWh.
    pub q_fou_ecs_kwh: f64,
    /// Demande restant reportée à la fin de l'année, tous postes confondus, en kWh (éq. 1028
    /// et 1034) : nulle si la génération a tout fourni.
    pub q_rep_fin_kwh: f64,
    /// Consommations en énergie finale de la génération, par poste et par type d'énergie
    /// (éq. 1056 et 1057).
    pub consommations: Consommations,
    /// « φvc_tot » : pertes et consommations d'auxiliaires transmises au volume chauffé, en kWh
    /// (éq. 1058), avant application de la part récupérable (fiche 11.1).
    pub pertes_vc_kwh: f64,
    /// Plus longue suite d'heures où la demande de chauffage ou d'ECS n'a pas été couverte,
    /// hors relance (éq. 1059) ; le texte y voit un sous-dimensionnement au-delà de 6 heures
    /// et un sous-dimensionnement critique au-delà de 72 heures (éq. 1060 et 1061).
    pub nbh_sousdim_ch_ecs_max: u32,
    /// De même en refroidissement (éq. 1062 à 1064).
    pub nbh_sousdim_fr_max: u32,
    /// Générateurs de la génération ; la pompe à chaleur d'un chauffe-eau thermodynamique
    /// décrit par son essai y figure sous l'identifiant du chauffe-eau.
    pub generateurs: Vec<ResultatsGenerateur>,
    /// Assemblages avec ballon de stockage d'ECS (§9.14) ; absent sans ballon.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stockages: Vec<ResultatsStockage>,
    /// Productions solaires collectives à appoints décentralisés (CESCI, CESCAI) ; absent sans
    /// production.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub solaires_collectifs: Vec<ResultatsSolaireCollectif>,
}

/// Générateur, calcul Th-C (§8.29.3.8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsGenerateur {
    /// Identifiant du générateur.
    pub id: Id,
    /// « Eef_fou_ch » : énergie fournie en chauffage, en kWh (éq. 1525).
    pub q_fou_ch_kwh: f64,
    /// « Eef_fou_fr » : énergie fournie en refroidissement, en kWh.
    pub q_fou_fr_kwh: f64,
    /// « Eef_fou_ecs » : énergie fournie en ECS, en kWh.
    pub q_fou_ecs_kwh: f64,
    /// Consommations annuelles en énergie finale, par poste et par type d'énergie (éq. 1494 à
    /// 1509).
    pub consommations: ConsommationsPostes,
    /// « ηeff_ch_an » : efficacité annuelle en chauffage (éq. 1526) — COP annuel d'un
    /// générateur thermodynamique, rendement des autres ; absente sans consommation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta_eff_ch_an: Option<f64>,
    /// « ηeff_fr_an » : efficacité annuelle en refroidissement (EER annuel).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta_eff_fr_an: Option<f64>,
    /// « ηeff_ecs_an » : efficacité annuelle en ECS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta_eff_ecs_an: Option<f64>,
    /// « Cef_aux » : consommation des auxiliaires du générateur, propres et amont, en kWh
    /// d'énergie finale électrique (éq. 1527 et 1528) ; elle est comprise dans `consommations`.
    pub w_aux_kwh: f64,
    /// « φrejet » : énergie échangée par un générateur thermodynamique avec sa source amont, en
    /// kWh (négative : chaleur puisée ; positive : chaleur rejetée en refroidissement).
    #[serde(default, skip_serializing_if = "est_nul")]
    pub phi_rejet_kwh: f64,
    /// « Eef_prelec » : électricité produite par un cogénérateur, en kWh (éq. 1516) ; absente
    /// pour les autres générateurs.
    #[serde(default, skip_serializing_if = "est_nul")]
    pub q_prelec_kwh: f64,
    /// Valeurs par défaut de la méthode appliquées au générateur.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}
