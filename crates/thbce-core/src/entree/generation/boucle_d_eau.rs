// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : boucles d'eau et pompes à chaleur sur boucle d'eau (Th-BCE 2020 §8.15).
//!
//! Une boucle d'eau est décrite sous la forme d'une génération (§8.15.3.1) : des pompes à
//! chaleur sur boucle d'eau (type 507, [`PacSurBoucleDEauSaisie`]), chacune associée à un groupe,
//! dont la source amont est la boucle (`boucle_d_eau`), et le générateur de chauffage et le
//! générateur de froid (ou la tour de refroidissement) qui maintiennent la boucle en
//! température ([`BoucleDEauGeneration`]).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::entree::*;

/// Th-BCE 2020 §8.15.1 et §8.15.3.1 — pompe à chaleur sur boucle d'eau (type 507), traitée
/// comme un groupe frigorifique eau / air (§8.23).
///
/// La machine se décrit comme un générateur thermodynamique à compression électrique : source
/// amont `boucle_d_eau` (la température de la boucle, éq. 1437 à 1439, est commune à toutes les
/// pompes à chaleur de la génération), technologie « eau de boucle / air recyclé » en chauffage
/// et « eau / air » en refroidissement, sans ECS. `rdim` de la génération donne le nombre de
/// machines identiques « Npbe » du groupe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PacSurBoucleDEauSaisie {
    /// Identifiant du groupe desservi par la pompe à chaleur (lien d'interface, §8.15.3.1) :
    /// la machine assure la demande de la génération pour ce groupe (éq. 1002 et 1003).
    pub groupe: Id,
    /// Machine : générateur thermodynamique à compression électrique (§8.23) sur la boucle.
    pub pac: ThermodynamiqueElectriqueSaisi,
}

/// Th-BCE 2020 §8.15.2 et §8.15.3.5 — boucle d'eau d'une génération : maintien en température
/// de la boucle par un générateur de chauffage et un générateur de froid de la génération (types
/// autres que 507, fluide aval eau), ou par une tour de refroidissement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoucleDEauGeneration {
    /// « θmax_rech_BE » : température de fonctionnement maximale de chauffage de la boucle, en
    /// °C (de 20 à 100, éq. 969). Requise si la génération a un générateur de chauffage de la
    /// boucle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theta_max_rech_c: Option<f64>,
    /// « θmin_fr_BE » : température de fonctionnement minimale de refroidissement de la
    /// boucle, en °C (éq. 975). Requise si la génération a un générateur de froid de la boucle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theta_min_fr_c: Option<f64>,
    /// Tour de refroidissement de la boucle (« idtour » 1 ou 2) ; absente : la boucle est
    /// refroidie par un générateur de froid de la génération (« idtour = 0 »).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tour: Option<TourBoucleSaisie>,
}

/// Th-BCE 2020 §8.15.3.7.1 — tour de refroidissement d'une boucle d'eau (éq. 972 à 974),
/// décrite dans la source amont de la boucle et traitée comme un générateur de froid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TourBoucleSaisie {
    /// Identifiant de la tour dans les résultats des générateurs, distinct de ceux des
    /// générateurs de la génération.
    pub id: Id,
    /// « idtour » : tour humide ou sèche.
    pub nature: NatureTourRefroidissement,
    /// « Δθtour » : écart entre l'eau en sortie de tour et la température humide (tour humide)
    /// ou sèche (tour sèche) de l'air extérieur, en K (éq. 1432 et 1434).
    pub delta_theta_tour_k: f64,
    /// « θes_tour_consigne » : consigne de température d'eau en sortie de tour, en °C
    /// (éq. 1433 et 1435).
    pub theta_es_consigne_c: f64,
    /// « qv,nom,tour » : débit nominal d'eau à refroidir circulant dans la tour, en m³/h
    /// (éq. 972).
    pub qv_nom_m3_h: f64,
    /// « Ppompes_tour » : puissance électrique des pompes du circuit de la tour, en W
    /// (éq. 973).
    pub p_pompes_tour_w: f64,
    /// « Pvent_tour » : puissance électrique des ventilateurs de la tour, en W (éq. 973).
    pub p_vent_tour_w: f64,
}
