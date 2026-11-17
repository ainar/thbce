// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : locaux et systèmes d'éclairage des usages autres que l'habitation
//! (Th-BCE 2020 §7.1).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::*;

/// Th-BCE 2020 §7.1 — local d'un groupe au sens de l'éclairage : « un local a une puissance
/// d'éclairage artificiel spécifique » et « un seul mode de mise en marche et d'extinction »
/// (§7.1.1). Des locaux de même type et de même système d'éclairage se regroupent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalEclairage {
    /// Identifiant, unique dans le groupe.
    pub id: Id,
    /// « type_local,l » : type de local, parmi ceux que le tableau 78 donne à l'usage de la
    /// zone ; il fixe le coefficient `C1` et l'éclairement de référence `Eiref`.
    pub type_local: thbce_data::eclairage::TypeLocal,
    /// « Ratiolocal,l » : part de la surface du local dans la surface utile du groupe, de 0 à 1.
    pub ratio_local: f64,
    /// « Ratioécl_nat,l » : part du local qui a un accès effectif à la lumière naturelle, de 0
    /// à 1 (§7.1.3.6.1) : parties situées à moins de `2,5 × (hLi − hTa)` d'une baie non
    /// horizontale (`hLi` : hauteur du linteau, `hTa` : hauteur du plan de travail), sous
    /// réserve, dans un local plus profond, que leurs luminaires soient commandés
    /// indépendamment ; 0 pour un local sans baie.
    pub ratio_ecl_nat: f64,
    /// Système d'éclairage du projet (§7.1.3.2). Le calcul du Bbio l'ignore : il emploie une
    /// puissance et une gestion conventionnelles (§7.1.3.1).
    pub eclairage: EclairageLocal,
}

/// Th-BCE 2020 §7.1.2 — système d'éclairage d'un local.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EclairageLocal {
    /// « Pecl_tot,l » : puissance surfacique totale d'éclairage installée à poste fixe dans le
    /// local, en W/m² : lampes et accessoires, système de gestion intégré aux luminaires,
    /// éclairages d'appoint des postes de travail ; hors éclairage de sécurité, éclairage
    /// extérieur, éclairage de mise en valeur et éclairage de process (§7.1.1). Dans un local de
    /// bureaux d'une zone de bureaux, c'est la puissance de l'éclairage immobilier
    /// (« Pecl_immo_projet », §7.1.3.2.3).
    pub p_ecl_tot_w_m2: f64,
    /// « Pecl_aux,l » : puissance surfacique des appareillages et périphériques de gestion
    /// quand les lampes sont éteintes (horloges, gradateurs, détecteurs, boutons-poussoirs
    /// alimentés ; hors ballasts, transformateurs et drivers), en W/m². Requise avec une
    /// détection de présence ou d'absence (`gest_ecl`) ou une gestion automatique selon la
    /// lumière naturelle (`grad_ecl`) ; sinon elle est nulle par convention : ne pas la saisir.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_ecl_aux_w_m2: Option<f64>,
    /// « Gest_ecl,l » : mode de commande de l'éclairage artificiel.
    pub gest_ecl: GestEcl,
    /// « Grad_ecl,l » : mode de gestion de l'éclairage artificiel selon la lumière naturelle.
    pub grad_ecl: GradEcl,
    /// « Fr_Grad_ecl,l » : vrai si la gestion selon la lumière naturelle est fractionnée (2),
    /// c'est-à-dire si la partie du local qui a accès à la lumière naturelle est gérée
    /// indépendamment de celle qui n'y a pas accès ; faux sinon (1).
    pub fractionne: bool,
    /// « Eff_ecl_immo_projet » : densité de puissance de l'éclairage immobilier, en W/m² pour
    /// 100 lux. Requise pour un local de bureaux d'une zone de bureaux dont la puissance
    /// immobilière est inférieure à 10 W/m² (§7.1.3.2.3) ; sans objet sinon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eff_ecl_immo_projet_w_m2_100lux: Option<f64>,
    /// « Eiproj » : éclairement requis du projet, en lux. Requis dans le même cas que
    /// `eff_ecl_immo_projet_w_m2_100lux` : si l'éclairage immobilier ne l'atteint pas,
    /// l'éclairage mobilier assure le complément, à raison de 1 W/m² pour 100 lux.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ei_proj_lux: Option<f64>,
}

/// « Gest_ecl,l » (§7.1.2 et §7.1.3.6.1) : mode de commande de l'éclairage artificiel d'un
/// local.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GestEcl {
    /// 0 : ni commande manuelle ni détection d'occupation (éclairage permanent pendant les
    /// plages d'occupation).
    Permanent,
    /// 1 : interrupteur manuel marche/arrêt.
    InterrupteurManuel,
    /// 2 : interrupteur manuel marche/arrêt et programmation horaire (horloge).
    InterrupteurEtHorloge,
    /// 3 : marche et arrêt automatiques par détection de présence et d'absence (extinction au
    /// plus 15 min après la dernière présence).
    MarcheEtArretAutomatiques,
    /// 4 : marche manuelle et arrêt automatique par détection d'absence.
    MarcheManuelleArretAutomatique,
}

/// « Grad_ecl,l » (§7.1.2 et §7.1.3.6.1) : mode de gestion de l'éclairage artificiel d'un local
/// selon la lumière naturelle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GradEcl {
    /// 0 : gestion impossible avec la lumière du jour (ni système de gestion ni interrupteur
    /// manuel).
    Aucune,
    /// 1 : gestion manuelle par interrupteur marche/arrêt.
    Manuelle,
    /// 2 : gradation automatique assurant un éclairement constant sur la zone de tâche.
    GradationAutomatique,
    /// 3 : allumage et extinction automatiques par détection du franchissement d'un seuil
    /// d'éclairement.
    AllumageEtExtinctionAutomatiques,
    /// 4 : extinction automatique par détection du franchissement d'un seuil d'éclairement.
    ExtinctionAutomatique,
}
