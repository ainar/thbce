// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats de l'eau chaude sanitaire : stockages, chauffe-eau thermodynamiques, émetteurs
//! et distributions intergroupes d'ECS.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ResultatsBoucleSolaire, ResultatsSsc, ValeurParDefaut};
use crate::conventions::arrondir_restitution;
use crate::entree::Id;

/// Assemblage avec ballon de stockage d'ECS, calcul Th-C (§9.9, §9.10 et §9.14).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsStockage {
    /// Identifiant de l'assemblage ou du chauffe-eau thermodynamique.
    pub id: Id,
    /// « Qfou,ECS » : énergie fournie par les ballons à la distribution d'ECS, en kWh
    /// (éq. 1954).
    pub q_fou_ecs_kwh: f64,
    /// « Qfou_sto_base » : énergie fournie aux ballons par le générateur de base, en kWh.
    pub q_fou_sto_base_kwh: f64,
    /// « Qfou_sto_ap » : énergie fournie aux ballons par l'appoint intégré, en kWh.
    pub q_fou_sto_ap_kwh: f64,
    /// « Φpertes » : pertes thermiques des ballons, en kWh (éq. 1771).
    pub pertes_kwh: f64,
    /// « Vp » : volume d'eau puisé dans les ballons, en L.
    pub v_puise_l: f64,
    /// Plus basse température de la zone haute en fin de pas de temps, en °C.
    pub theta_haut_min_c: f64,
    /// Plus longue suite d'heures où la zone haute est restée sous la consigne (éq. 1959) ; le
    /// texte juge le ballon incohérent avec le projet au-delà de 168 heures.
    pub nbh_temp_insuff_max: u32,
    /// « nbhreport_e » : nombre d'heures de l'année où l'énergie demandée n'a pas pu être
    /// entièrement puisée dans le ballon (éq. 1780) ; le texte affiche une alerte au-delà de
    /// 24 heures.
    #[serde(default)]
    pub nbh_report_e: u32,
    /// « Vz » : volumes des quatre zones d'un ballon, de bas en haut, en L (un des assemblages
    /// identiques).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub v_zones_l: Vec<f64>,
    /// « θbz » : températures des zones au début du calcul, de bas en haut, en °C.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub theta_zones_debut_c: Vec<f64>,
    /// « θbz » : températures des zones à la fin du calcul, de bas en haut, en °C. Avec
    /// `v_zones_l` et `theta_zones_debut_c`, elles donnent la variation du stock de chaleur des
    /// ballons sur l'année, qui ferme leur bilan.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub theta_zones_fin_c: Vec<f64>,
    /// Boucle solaire d'un chauffe-eau solaire (§9.15), pour l'ensemble des assemblages
    /// identiques.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solaire: Option<ResultatsBoucleSolaire>,
    /// Boucle de chauffage d'un système solaire combiné (fiches 9.18 à 9.20).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssc: Option<ResultatsSsc>,
    /// Paramètres Th-BCE identifiés à partir de l'essai d'un chauffe-eau thermodynamique
    /// (§9.22).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idcet: Option<ResultatsIdCet>,
    /// Échangeur d'un accumulateur d'ECS en eau technique (fiches 9.11 et 9.12).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echangeur: Option<ResultatsEchangeurEcs>,
    /// Valeurs par défaut de la méthode appliquées à l'assemblage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// Échangeur d'un accumulateur d'ECS en eau technique (§9.12), pour l'ensemble des assemblages
/// identiques.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsEchangeurEcs {
    /// « UAech » : coefficient de transfert de l'échangeur d'un assemblage, en W/K, déduit de sa
    /// puissance nominale (éq. 1876 à 1882 ou 1923 à 1926).
    pub ua_ech_w_k: f64,
    /// « θcons » : consigne du ballon, en °C (tableau 288).
    pub theta_cons_c: f64,
    /// « Wcirc,prim » : consommation du circulateur primaire d'un échangeur externe, en kWh
    /// d'électricité (éq. 1907 et 1908).
    pub w_circ_prim_kwh: f64,
}

impl ResultatsStockage {
    /// Arrondit les valeurs restituées (Q8).
    pub fn arrondir(&mut self) {
        let a = arrondir_restitution;
        let s = self;
        for x in [
            &mut s.q_fou_ecs_kwh,
            &mut s.q_fou_sto_base_kwh,
            &mut s.q_fou_sto_ap_kwh,
            &mut s.pertes_kwh,
            &mut s.v_puise_l,
            &mut s.theta_haut_min_c,
        ] {
            *x = a(*x);
        }
        if let Some(e) = &mut s.echangeur {
            e.ua_ech_w_k = a(e.ua_ech_w_k);
            e.w_circ_prim_kwh = a(e.w_circ_prim_kwh);
        }
        if let Some(x) = &mut s.ssc {
            x.q_puise_ch_kwh = a(x.q_puise_ch_kwh);
            x.q_fou_ch_appoint_kwh = a(x.q_fou_ch_appoint_kwh);
        }
        if let Some(b) = &mut s.solaire {
            for x in [
                &mut b.e_sol_kwh,
                &mut b.w_pompe_kwh,
                &mut b.pertes_vc_kwh,
                &mut b.isr_kwh_m2,
                &mut b.ecretement_kwh,
            ] {
                *x = a(*x);
            }
            for x in &mut b.e_sol_mensuel_kwh {
                *x = a(*x);
            }
        }
        if let Some(i) = &mut s.idcet {
            // Paramètres identifiés restitués avec quatre décimales.
            for x in [&mut i.ua_s_w_k, &mut i.cop_pivot, &mut i.pabs_pivot_kw] {
                *x = (*x * 1e4).round() / 1e4;
            }
        }
    }
}

/// Identification d'un chauffe-eau thermodynamique (§9.22.3.4) : entrées du modèle Th-BCE
/// déduites de l'essai, avant correction de statut.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsIdCet {
    /// « UAs,util » : coefficient de pertes du ballon, en W/K.
    pub ua_s_w_k: f64,
    /// « ValCOPpivot » : valeur pivot du COP.
    pub cop_pivot: f64,
    /// « ValPabspivot » : valeur pivot de la puissance absorbée, en kW.
    pub pabs_pivot_kw: f64,
    /// « ε » : écart relatif entre l'essai reproduit par le modèle et l'essai saisi (éq. 2248).
    pub epsilon: f64,
}

/// Émetteur d'ECS équivalent, calcul Th-C (§9.5 à 9.7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsEcsEmetteur {
    /// Identifiant.
    pub id: Id,
    /// « Agr,em-e » : surface desservie, en m² (éq. 1674).
    pub a_m2: f64,
    /// « Nadeq-e » : adultes équivalents (éq. 1685 et 1689).
    pub nadeq: f64,
    /// « a » : besoins unitaires, en litres à 40 °C par adulte équivalent et par semaine.
    pub a_l: f64,
    /// « corr-e » : coefficient correctif des émetteurs et des appareils (éq. 1678).
    pub corr: f64,
    /// « Vuw_hebdo_corr » : volume hebdomadaire corrigé, en litres à 40 °C (éq. 1692).
    pub v_uw_hebdo_corr_l: f64,
    /// « Qw » : besoins annuels, en kWh (éq. 1693).
    pub q_w_kwh: f64,
    /// « Qw_bruts » : besoins annuels sans correction des émetteurs, en kWh (éq. 1694).
    pub q_w_bruts_kwh: f64,
    /// « Qw_2nd-e » : besoins augmentés des pertes de la distribution du groupe, en kWh.
    pub q_w_2nd_kwh: f64,
    /// Pertes de la distribution du groupe en volume chauffé, en kWh (récupérables à 60 %).
    pub pertes_vc_kwh: f64,
    /// Pertes de la distribution du groupe hors volume chauffé, en kWh.
    pub pertes_hvc_kwh: f64,
    /// Valeurs par défaut de la méthode appliquées.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// Distribution d'ECS intergroupe, calcul Th-C (§9.8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsEcsIntergroupe {
    /// Identifiant.
    pub id: Id,
    /// Génération qui produit l'ECS.
    pub generation: Id,
    /// « Qw_prim-e » : besoins transmis à la génération, en kWh.
    pub q_w_prim_kwh: f64,
    /// Valeurs mensuelles de `q_w_prim_kwh`, en kWh.
    pub q_w_prim_mensuel_kwh: Vec<f64>,
    /// Pertes en volume chauffé, en kWh.
    pub pertes_vc_kwh: f64,
    /// Pertes hors volume chauffé, en kWh.
    pub pertes_hvc_kwh: f64,
    /// Circulateurs ou traceurs, en kWh d'énergie finale électrique.
    pub w_aux_kwh: f64,
    /// Réchauffeur de boucle, en kWh d'énergie finale électrique.
    pub w_rechauf_kwh: f64,
    /// Température de départ demandée à la génération, en °C.
    pub theta_depart_c: Option<f64>,
}
