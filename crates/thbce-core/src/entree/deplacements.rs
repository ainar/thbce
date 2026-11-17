// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : parties communes et déplacements — ascenseurs (§10.1) et parcs de
//! stationnement.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::*;

/// Th-BCE 2020 §10.1.2, tableau 309 — « TechMac » : typologie d'ascenseur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TechMac {
    /// 0 : traction avec réduction.
    TractionAvecReduction,
    /// 1 : traction sans réduction.
    TractionSansReduction,
    /// 2 : hydraulique.
    Hydraulique,
    /// 3 : autre.
    Autre,
}

/// Th-BCE 2020 §10.1.2, tableau 309 — une cabine d'ascenseur : paramètres intrinsèques
/// (`TechMac`, `Q`, `V`, `Cp`, `ScVeille` et ses cinq paramètres de veille) et d'intégration
/// (`H`, `netage`, zones desservies `C`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Ascenseur {
    /// Identifiant de la cabine.
    pub id: Id,
    /// « TechMac » : typologie.
    pub tech_mac: TechMac,
    /// « Q » : charge utile de la cabine au sens de l'EN 81-1, en kg (au moins 75).
    pub q_kg: f64,
    /// « V » : vitesse nominale au sens de l'EN 81-1, en m/s (au plus 2,5).
    pub v_m_s: f64,
    /// « H » : course de l'ascenseur, sous-sols compris, en m.
    pub h_m: f64,
    /// « netage » : nombre d'étages desservis, point bas non compté (au moins 2) : 4 pour un
    /// R+4 sans sous-sol, 5 pour un R+4 avec un niveau de sous-sol.
    pub netage: u32,
    /// « Cp » : équilibrage de la machine (−1,2 à 0,5). Absent : valeur du tableau 314,
    /// signalée dans les résultats. Pour un ascenseur hydraulique, −1,2 est obligatoire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cp: Option<f64>,
    /// « C » : identifiants des zones desservies (au moins une, de logement collectif).
    pub zones_desservies: Vec<Id>,
    /// « ScVeille » : le scénario de mise en veille de la cabine est connu. Faux par défaut :
    /// la cabine n'a alors pas de veille (`dP1 = dP2 = 0`). Vrai : `dp1_w`, `t1_s`, `dp2_w` et
    /// `t2_s` sont employés, avec les valeurs par défaut du §10.1.3.3 pour ceux qui manquent.
    #[serde(default)]
    pub sc_veille: bool,
    /// « Pti » : puissance totale de veille cabine immobile, en W, mesurée après fermeture des
    /// portes. Absent : valeur par défaut de l'éq. (2274).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pti_w: Option<f64>,
    /// « dP1 » : premier décrément de la puissance de veille, en W.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dp1_w: Option<f64>,
    /// « T1 » : durée avant le premier décrément, en s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t1_s: Option<f64>,
    /// « dP2 » : second décrément de la puissance de veille, en W.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dp2_w: Option<f64>,
    /// « T2 » : durée avant le second décrément à partir du premier, en s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t2_s: Option<f64>,
}

/// Th-BCE 2020 §10.3 et §10.4, tableau 322 — « Type » : parking intérieur (fermé) ou
/// extérieur (ouvert).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeParking {
    /// « int » : accès à la lumière du jour nul en permanence.
    Interieur,
    /// « ext » : accès à la lumière du jour parfait.
    Exterieur,
}

/// Th-BCE 2020 §10.3.2 et §10.4.2 — parc de stationnement associé au bâtiment. Le « Typeusage »
/// se déduit de l'usage des zones : « habitat » pour les usages 1 et 2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parking {
    /// Identifiant du parking.
    pub id: Id,
    /// « Type » : intérieur ou extérieur.
    pub type_parking: TypeParking,
    /// « Npl » : nombre de places, éventuellement non entier : une place de deux-roues motorisé
    /// compte 0,33 place (arrondi à l'entier supérieur pour l'ensemble), un deux-roues non
    /// motorisé 0 (§10.3.3).
    pub npl: f64,
    /// « Net » : nombre d'étages du parking.
    pub net: u32,
    /// Ventilation forcée (« Vent » = oui). Absent : pas de ventilation forcée. Sans effet pour
    /// un parking extérieur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ventilation: Option<VentilationParking>,
    /// Éclairage du parking (§10.4). Requis.
    pub eclairage: EclairageParking,
    /// Jours de l'année (de 1 à 365) où le parking est fermé (« NbjO(j) = 0 »). Absent : ouvert
    /// tous les jours.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jours_fermes: Vec<u16>,
}

/// Th-BCE 2020 §10.3.2 — ventilation forcée d'un parking d'habitat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VentilationParking {
    /// « Reg » : la ventilation est régulée (vrai) ou permanente 24 h/24 (faux).
    pub regulee: bool,
    /// « Pvent600 » : puissance électrique active de l'installation par place, au point de
    /// fonctionnement de 600 m³/h par place, en W/place. Absent : 40 W/place (éq. 2326).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent600_w_par_place: Option<f64>,
}

/// Th-BCE 2020 §10.4.2 — éclairage d'un parking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EclairageParking {
    /// « Pecins » : puissance totale installée, en W. Absent : 75 W par place en intérieur, 8 W
    /// par place en extérieur (éq. 2349 et 2350).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_installee_w: Option<f64>,
    /// « PlagDse » : plages de fonctionnement en détection de présence du lundi au vendredi,
    /// au plus trois, en couples (heure légale de début, heure légale de fin) : l'éclairage est
    /// en détection de `debut` h à `fin` h. Absent : pas de détection.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plages_detection_semaine: Vec<PlageHoraire>,
    /// « PlagDwe » : plages de détection du samedi et du dimanche.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plages_detection_week_end: Vec<PlageHoraire>,
    /// « Ex » : extinction de l'éclairage quand le parking est fermé (vrai) ou marche forcée
    /// (faux). Requis si le parking a des jours de fermeture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extinction_en_fermeture: Option<bool>,
}

/// Plage horaire en heures légales, de `debut` (inclus) à `fin` (exclue), de 0 à 24.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlageHoraire {
    /// Heure légale de début, de 0 à 24.
    pub debut_h: u8,
    /// Heure légale de fin, de 0 à 24.
    pub fin_h: u8,
}
