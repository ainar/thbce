// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : bâtiment, zones et groupes (Th-BCE 2020 §5.1).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::*;

/// Niveau « bâtiment » (Th-BCE 2020 §5.1) : niveau où s'expriment Bbio, Cep et Cep,nr.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Batiment {
    /// Identifiant du bâtiment.
    pub id: Id,
    /// Zones : parties de bâtiment de même usage, donc de mêmes scénarios (§5.1.3).
    pub zones: Vec<Zone>,
    /// Espaces tampons du bâtiment (§5.2 à 5.4) : locaux non chauffés en contact avec
    /// l'enveloppe d'un ou de plusieurs groupes. Les parois opaques, ponts thermiques, baies et
    /// entrées d'air qui donnent sur un espace tampon le désignent par son identifiant
    /// (`espace_tampon`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub espaces_tampons: Vec<EspaceTampon>,
    /// Générations du bâtiment (§8.4 et §8.14). Requises dès qu'un réseau intergroupes y est
    /// rattaché.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generations: Vec<Generation>,
    /// Réseaux de distribution intergroupes de chaud et de froid (§8.9 et §8.10).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub distributions_intergroupes: Vec<DistributionIntergroupes>,
    /// Réseaux de distribution d'ECS intergroupes (§9.8).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub distributions_ecs_intergroupes: Vec<DistributionEcsIntergroupe>,
    /// Installations photovoltaïques intégrées au bâtiment (§12.1 et §12.2). Facultatif.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub installations_pv: Vec<crate::pv::InstallationPv>,
    /// Ascenseurs du bâtiment (§10.1), une entrée par cabine ; logement collectif seulement.
    /// L'éclairage conventionnel des parties communes (§7.2) ne demande aucune saisie : il
    /// s'applique à toute zone de logement collectif.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ascenseurs: Vec<Ascenseur>,
    /// Parcs de stationnement associés au bâtiment (§10.3 et §10.4).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parkings: Vec<Parking>,
    /// Données de la fixation des exigences (annexe de l'article R. 172-4 du code de la
    /// construction et de l'habitation, chapitres II et III). Facultatives : sans elles, les
    /// indicateurs sont calculés mais ni les exigences ni la conformité.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exigences: Option<ParametresExigences>,
}

/// Niveau « zone » (Th-BCE 2020 §5.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Zone {
    /// Identifiant de la zone.
    pub id: Id,
    /// « Usage zone » (§4.1.2).
    pub usage: UsageZone,
    /// « Nb_lgt » (§4.1.2) : nombre de logements, en logement collectif et pour les maisons
    /// accolées. Requis en logement collectif ; 1 par défaut en maison individuelle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nb_logements: Option<u32>,
    /// « hzone » : hauteur entre le point le plus bas et le point le plus haut de la zone, en m
    /// (§5.6.2). Requise : elle fixe l'altitude des défauts d'étanchéité et des entrées d'air.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h_zone_m: Option<f64>,
    /// « hplancher » : hauteur du point le plus bas de la zone par rapport au sol extérieur, en
    /// m (§5.6.2) ; négative si la zone descend sous le niveau du sol. Requise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h_plancher_m: Option<f64>,
    /// « Q4Pa_surf » : perméabilité à l'air de l'enveloppe sous 4 Pa, en m³/(h.m²) de parois
    /// déperditives hors plancher bas (§5.6.2), la même pour tous les groupes de la zone.
    /// Requise. Une valeur mesurée par échantillonnage se saisit multipliée par 1,2 ; si des
    /// travaux pouvant affecter la perméabilité restent à réaliser après la livraison, on ajoute
    /// 0,3 m³/(h.m²) (§5.6.3.3.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q4pa_surf_m3_h_m2: Option<f64>,
    /// « δtrav_zone » : caractère traversant de la zone au sens des conditions d'hiver, baies et
    /// portes intérieures fermées (§5.6.3.1.3, tableau 22). Conventionnel pour tous les usages
    /// sauf le logement collectif : ne pas le saisir. En logement collectif, requis : vrai si
    /// les logements sont traversants au sens du confort d'été et si les portes intérieures
    /// d'une façade à l'autre sont détalonnées.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trav_zone: Option<bool>,
    /// Système de ventilation de la zone (Th-BCE 2020 §6 : « la modélisation du système de
    /// ventilation s'effectue au niveau d'une zone »). Requis en habitation : il sert aux
    /// calculs Th-C et Th-D ; le calcul du Bbio emploie le système conventionnel de la fiche 6.1
    /// (§6.1.3). En logement collectif, un système qui dessert plusieurs zones se décrit dans
    /// chacune, ses débits et ses puissances étant répartis au prorata des surfaces des zones
    /// (§6, introduction).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ventilation: Option<Ventilation>,
    /// Groupes de la zone.
    pub groupes: Vec<Groupe>,
}

/// Niveau « groupe » (Th-BCE 2020 §5.7) : local ou ensemble de locaux dont l'air est supposé
/// homogène en température et en humidité (§5.21.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Groupe {
    /// Identifiant du groupe.
    pub id: Id,
    /// « SHAB » : surface habitable, en m². Requise pour les usages d'habitation, dont elle est
    /// la surface de référence (§13.1.3, éq. 2375).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shab_m2: Option<f64>,
    /// « Agr » : surface utile du groupe, en m² (§4.1.2). Requise pour tous les usages : elle
    /// répartit les apports internes entre groupes. Hors habitation, c'est aussi la surface de
    /// référence « SU » (§13.1.3, éq. 2376 ; décision Q28). En maison individuelle, on saisit en
    /// pratique `su_m2 = shab_m2`. En logement collectif, le calcul en déduit sa propre surface
    /// habitable, `Rat_hab · Az` (éq. 33), distincte de la SHAB saisie comme surface de référence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub su_m2: Option<f64>,
    /// « V » : volume du groupe, en m³ (§8.3.2). Requis : le bilan hydrique du groupe en tire la
    /// masse d'air dans laquelle se diluent les apports d'humidité (§8.3.3, éq. 831). Le texte ne
    /// lui donne pas de valeur par défaut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_m3: Option<f64>,
    /// « Categorie_CE1_CE2 » (§13.1.2).
    pub categorie_ce1_ce2: CategorieCe,
    /// Inertie thermique du groupe (§5.21.2).
    pub inertie: Inertie,
    /// Parois opaques (§5.15 à 5.17).
    #[serde(default)]
    pub parois_opaques: Vec<ParoiOpaque>,
    /// Baies vitrées (§5.8, 5.10).
    #[serde(default)]
    pub baies: Vec<Baie>,
    /// Ponts thermiques (§5.18 à 5.20).
    #[serde(default)]
    pub ponts_thermiques: Vec<PontThermique>,
    /// « δtrav_surv » : caractère traversant du groupe au sens de la surventilation par
    /// ouverture des baies, c'est-à-dire la possibilité d'ouvrir les portes intérieures entre
    /// deux façades d'orientations différentes (§5.13.3.2.1, tableau 43). Conventionnel en
    /// maison individuelle (traversant) : ne pas le saisir. Requis en logement collectif si le
    /// groupe a des baies ouvrables.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traversant: Option<bool>,
    /// « Httf » : hauteur pour le tirage thermique, en m (§5.13.3.2.2). Absent : 1,5 m, valeur
    /// du cas général, signalée dans les résultats. À saisir seulement en maison individuelle
    /// quand la différence d'altitude entre le bas de l'ouvrant le plus bas et le haut de
    /// l'ouvrant le plus haut dépasse 4 m, dans la limite de 15 m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h_ttf_m: Option<f64>,
    /// Bouches et conduits de ventilation du groupe (§6.2) : débits d'hygiène de la ventilation
    /// installée. Au moins une en habitation : le calcul du Bbio en tire les débits du système
    /// de ventilation conventionnel (§6.1 et §6.2.3.1.3).
    #[serde(default)]
    pub bouches_conduits: Vec<BoucheConduit>,
    /// Entrées d'air du groupe (§5.6.3.2), décrites par leur module total. Absent : le groupe
    /// n'a pas d'entrée d'air.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entrees_air: Option<EntreesAir>,
    /// « iclim » : vrai si le groupe est climatisé, c'est-à-dire refroidi par un émetteur de
    /// froid (§8.1.2, §8.4.2). Requis en habitation. Employé par le calcul des consommations
    /// (Th-C) : sans climatisation, pas de saison de refroidissement (§4.6.3.2) ni de besoin de
    /// froid (éq. 807). Le calcul du Bbio l'ignore (décision Q38 point 3) et le calcul Th-D
    /// passe tout groupe en non climatisé (§13.5.3.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iclim: Option<bool>,
    /// Émetteurs de chaud et de froid du groupe (§8.1 et §8.2). Au moins un émetteur de
    /// chauffage en habitation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emetteurs: Vec<Emetteur>,
    /// Programmation des relances du groupe (§8.5). Requise en habitation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub programmation: Option<Programmation>,
    /// Réseaux de distribution du groupe de chaud et de froid (§8.7 et §8.8), auxquels les
    /// émetteurs sont rattachés.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub distributions: Vec<DistributionGroupe>,
    /// Émetteurs d'ECS équivalents du groupe (§9.5 à 9.7). Au moins un en habitation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emetteurs_ecs: Vec<EmetteurEcs>,
}

/// Catégorie de contrainte extérieure du groupe (§13.1.2 : « 1 = CE1 / 2 = CE2 »).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum CategorieCe {
    /// CE1.
    #[serde(rename = "CE1")]
    Ce1,
    /// CE2.
    #[serde(rename = "CE2")]
    Ce2,
}
