// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : données de la fixation des exigences (annexe de l'article R. 172-4 du
//! CCH) : catégorie de contraintes extérieures et zone de bruit.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Données de la fixation des exigences d'un bâtiment (annexe de l'article R. 172-4 du CCH).
/// Elles s'appliquent à toutes ses zones.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParametresExigences {
    /// Catégorie de contraintes extérieures du bâtiment (chapitre V de l'annexe, partie II).
    /// Elle module `Cep_max`, `Cep,nr_max` et `Icénergie_max` (`Mccat`) et fixe `DH_max`.
    ///
    /// Un local est de catégorie 2 si, cumulativement, il est climatisé, ses baies sont exposées
    /// au bruit BR2 ou BR3 et le bâtiment est construit en zone H2d ou H3 à moins de 400 m
    /// d'altitude ; une zone est de catégorie 2 si tous ses locaux autres qu'à occupation
    /// passagère le sont. La catégorie 3 est réservée à des usages non résidentiels. La
    /// validation refuse une catégorie que ces règles excluent ; la catégorie reste saisie, faute
    /// de description des locaux dans l'entrée.
    pub categorie_contraintes_exterieures: CategorieContraintes,
    /// Classe d'exposition au bruit du bâtiment (chapitre V de l'annexe, partie I) : module
    /// `Bbio_max` (`Mbbruit`). Saisie : elle dépend des infrastructures voisines et du plan
    /// d'exposition au bruit.
    ///
    /// Règle de détermination : la classe de chaque baie (BR1, BR2 ou BR3) se lit dans les
    /// tableaux de la partie I selon la catégorie des infrastructures de transports terrestres
    /// voisines (arrêté préfectoral, article R. 571-38 du code de l'environnement), leur distance
    /// à la façade et la vue qu'en a la baie (directe, partielle, masquée par des obstacles peu
    /// ou très protecteurs, arrière), et selon la zone du plan d'exposition au bruit de
    /// l'aérodrome (zones A à C : BR3 ; D : BR2 ; hors zone : BR1) ; la classe la plus
    /// défavorable est retenue ; au-delà de la distance maximale de prise en compte (700, 500,
    /// 250, 100 ou 30 m selon la catégorie 1 à 5) et hors plan d'exposition, toutes les baies
    /// sont BR1. Une partie de bâtiment est BR3 si toutes ses baies le sont, BR2 si toutes sont
    /// BR2 ou BR3 et l'une au moins BR2, BR1 sinon.
    pub zone_bruit: ZoneBruit,
    /// Année de dépôt de la demande de permis de construire (2022 ou après) : fixe
    /// `Icénergie_maxmoyen`. Les dérogations datées du texte sont appliquées à l'année : « avant
    /// le 31 décembre 2023 » (maisons, raccordement au gaz) est lu « année ≤ 2023 », « avant le
    /// 31 décembre 2027 » (réseau classé) « année ≤ 2027 » (Q54).
    pub annee_depot_permis: u16,
    /// « HSPmoy » : hauteur sous plafond moyenne des locaux d'habitation, en m (chapitre I,
    /// §XIII) : modulations `MbHSP` et `McHSP`.
    pub hsp_moy_m: f64,
    /// « Scombles » : surface de plancher des combles aménagés dont la hauteur sous plafond est
    /// inférieure à 1,8 m, en m² (maisons individuelles ou accolées). Absente : pas de tels
    /// combles.
    #[serde(default)]
    pub s_combles_m2: f64,
    /// Réseau de chaleur auquel le bâtiment est raccordé classé en application de l'article
    /// L. 712-1 du code de l'énergie : jusqu'en 2027, `Icénergie_maxmoyen` des années 2022 à
    /// 2024.
    #[serde(default)]
    pub reseau_chaleur_classe: bool,
    /// Maison dont la parcelle est concernée par un permis d'aménager délivré, ou comprise dans
    /// une zone d'aménagement concerté dont le dossier de réalisation a été approuvé, avant le
    /// 1er janvier 2022 et prévoyant un raccordement au réseau de gaz : `Icénergie_maxmoyen` de
    /// 280 kg éq. CO2/m² pour un permis déposé avant le 31 décembre 2023.
    #[serde(default)]
    pub parcelle_raccordement_gaz: bool,
}

/// Catégorie de contraintes extérieures (annexe de l'article R. 172-4, chapitre V).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CategorieContraintes {
    /// Catégorie 1.
    #[serde(rename = "categorie_1")]
    Categorie1,
    /// Catégorie 2.
    #[serde(rename = "categorie_2")]
    Categorie2,
    /// Catégorie 3 : locaux climatisés de certains usages non résidentiels (bureaux, hôtels,
    /// commerces…). Reconnue pour être refusée explicitement en habitation.
    #[serde(rename = "categorie_3")]
    Categorie3,
}

/// Zone de bruit (annexe de l'article R. 172-4, chapitre V).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ZoneBruit {
    /// BR1.
    Br1,
    /// BR2.
    Br2,
    /// BR3.
    Br3,
}
