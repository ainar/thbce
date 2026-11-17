// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats par zone et par groupe.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    Consommations, ConsommationsCollectif, ResultatsBaie, ResultatsBbio, ResultatsConfort,
    ResultatsConfortEte, ResultatsDebitsBbio, ResultatsEnveloppe, ResultatsSaisons,
    ResultatsSystemesGroupe, ResultatsVentilationGroupe, ResultatsVentilationZone, ValeurParDefaut,
};
use crate::entree::Id;

/// Résultats d'une zone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsZone {
    /// Identifiant de la zone.
    pub id: Id,
    /// « SREF_zn », en m² (§13.1.3, éq. 2377).
    pub sref_m2: f64,
    /// « Nadeq » : nombre d'adultes équivalents de la zone, en habitation (§4.1.3, éq. 32 et 36).
    pub nadeq: Option<f64>,
    /// Besoins mensuels et annuels et Bbio de la zone (§13.1, « Sortie_Zone_B »).
    pub bbio: Option<ResultatsBbio>,
    /// « DH » du groupe le plus défavorable de la zone, en °C.h (§13.5.3, éq. 2551).
    pub dh_c_h: Option<f64>,
    /// Système de ventilation du projet (§6.2 à 6.5) ; absent hors habitation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ventilation: Option<ResultatsVentilationZone>,
    /// Consommations en énergie finale des générations attribuées à la zone : somme de celles
    /// de ses groupes. Absent sans génération.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consommations: Option<Consommations>,
    /// Postes propres au logement collectif et aux parkings attribués à la zone (§7.2, §10.1,
    /// §10.3, §10.4). Absent sans logement collectif ni parking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consommations_collectif: Option<ConsommationsCollectif>,
    /// Résultats par groupe.
    pub groupes: Vec<ResultatsGroupe>,
}

/// Résultats d'un groupe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsGroupe {
    /// Identifiant du groupe.
    pub id: Id,
    /// « SREF_gr », en m² (§13.1.3, éq. 2375 et 2376).
    pub sref_m2: f64,
    /// Grandeurs d'enveloppe du groupe.
    pub enveloppe: ResultatsEnveloppe,
    /// Débits du système de ventilation conventionnel du Bbio ; `null` si le groupe ne décrit
    /// aucune bouche de ventilation ou hors habitation.
    pub debits_bbio: Option<ResultatsDebitsBbio>,
    /// Besoins mensuels et annuels et Bbio du groupe (§13.1, « Sortie_Groupe_B »).
    pub bbio: Option<ResultatsBbio>,
    /// Saisons propres du groupe en calcul du Bbio (§4.6).
    pub saisons_bbio: Option<ResultatsSaisons>,
    /// Indicateurs de confort du groupe en calcul du Bbio (§4.5), en évolution libre.
    pub confort_bbio: Option<ResultatsConfort>,
    /// Confort d'été du groupe, issu du calcul Th-D (§13.5).
    pub confort_ete: Option<ResultatsConfortEte>,
    /// Saisons propres du groupe en calcul Th-D (§4.6).
    pub saisons_th_d: Option<ResultatsSaisons>,
    /// Ventilation du projet vue du groupe : débits et consommation des ventilateurs ; absente
    /// hors habitation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ventilation: Option<ResultatsVentilationGroupe>,
    /// Systèmes d'émission du groupe, issus du calcul des consommations (Th-C) ; absents sans
    /// émetteur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub systemes: Option<ResultatsSystemesGroupe>,
    /// Consommations en énergie finale des générations attribuées au groupe, par poste et par
    /// type d'énergie, au prorata de ses demandes (§8.29, éq. 1484 à 1486, 1499, 1506 et 1513) ;
    /// matrice « Qcef[poste ; énergie] » des sorties du mode Th-C (§13.2, éq. 2421). Absent sans
    /// génération.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consommations: Option<Consommations>,
    /// Résultats par baie.
    pub baies: Vec<ResultatsBaie>,
    /// Valeurs par défaut de la méthode appliquées aux paramètres non saisis du groupe ; liste
    /// vide si aucune.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}
