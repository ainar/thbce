// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : données de la fixation des exigences (annexe de l'article R. 172-4 du
//! CCH) : catégorie de contraintes extérieures et zone de bruit, pour le bâtiment, ses zones et
//! ses groupes (Q54, Q66), date de dépôt du permis (Q68).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Id;

/// Données de la fixation des exigences d'un bâtiment (annexe de l'article R. 172-4 du CCH).
/// Elles s'appliquent à toutes ses zones.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParametresExigences {
    /// Catégorie de contraintes extérieures du bâtiment (chapitre V de l'annexe, partie II).
    /// Elle module `Cep_max`, `Cep,nr_max` et `Icénergie_max` (`Mccat`), `Bbio_max` (`Mbbruit`,
    /// catégorie 3) et fixe `DH_max`.
    ///
    /// Un local est de catégorie 2 si, cumulativement, il est climatisé, ses baies sont exposées
    /// au bruit BR2 ou BR3 et le bâtiment est construit en zone H2d ou H3 à moins de 400 m
    /// d'altitude ; une zone est de catégorie 2 si tous ses locaux autres qu'à occupation
    /// passagère le sont. La catégorie 3 est réservée aux locaux climatisés de certains usages
    /// non résidentiels (voir [`CategorieContraintes::Categorie3`]). La validation refuse une
    /// catégorie que ces règles excluent ; la catégorie reste saisie, faute de description des
    /// locaux dans l'entrée. Elle vaut pour toutes les zones et tous les groupes, sauf ceux qui
    /// ont la leur dans `zones` (Q66).
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
    /// Mois (1 à 12) du dépôt de la demande de permis de construire, facultatif : il départage
    /// les versions de l'annexe dont la date d'effet tombe en cours d'année (Q68) : 1er juillet
    /// 2022 (bureaux et enseignement), 1er mai 2026 (usages 6 à 28), 1er juillet 2026
    /// (modulations `MbHSP` et `McHSP` du décret n° 2026-200). **Obligatoire** pour un permis
    /// déposé en 2026, et en 2022 avec une zone d'usage 3 à 5 ; facultatif les autres années,
    /// où l'année suffit (Q68).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mois_depot_permis: Option<u8>,
    /// « HSPmoy » : hauteur sous plafond moyenne des locaux d'habitation, en m (chapitre I,
    /// §XIII) : modulations `MbHSP` et `McHSP`, introduites par le décret n° 2026-200 pour les
    /// permis déposés à partir du 1er juillet 2026 (Q69). Obligatoire dans ce cas si le
    /// bâtiment comprend une zone d'habitation (usages 1 et 2) ; sans effet pour les autres
    /// usages, dont `MbHSP` et `McHSP` sont nuls, et pour les permis antérieurs ; une valeur
    /// saisie hors de la plage 1,8 à 10 m reste une erreur, même sans effet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hsp_moy_m: Option<f64>,
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
    /// Catégorie de contraintes extérieures et classe d'exposition au bruit propres à une zone,
    /// quand elles diffèrent de celles du bâtiment (chapitre V : elles se définissent pour
    /// chaque partie de bâtiment ou zone ; Q66). Une zone absente de la liste prend les valeurs
    /// du bâtiment.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<ExigencesZoneSaisie>,
}

impl ParametresExigences {
    /// Vrai si la demande de permis est déposée à partir du 1er jour du mois `mois` de l'année
    /// `annee` ; sans mois saisi, le dépôt est réputé fait en fin d'année (Q68).
    pub fn depot_a_partir_de(&self, annee: u16, mois: u8) -> bool {
        (
            self.annee_depot_permis,
            self.mois_depot_permis.unwrap_or(12),
        ) >= (annee, mois)
    }

    /// Catégorie de contraintes extérieures du groupe `groupe` de la zone `zone` : celle saisie
    /// pour le groupe, sinon celle de la zone (Q66).
    pub fn categorie_groupe(&self, zone: &str, groupe: &str) -> CategorieContraintes {
        self.zone(zone)
            .and_then(|z| z.groupes.iter().find(|g| g.groupe.as_str() == groupe))
            .map_or_else(
                || self.categorie_zone(zone),
                |g| g.categorie_contraintes_exterieures,
            )
    }

    /// Données propres à la zone `id`, si elles sont saisies.
    fn zone(&self, id: &str) -> Option<&ExigencesZoneSaisie> {
        self.zones.iter().find(|z| z.zone.as_str() == id)
    }

    /// Catégorie de contraintes extérieures de la zone `id` : celle de la zone si elle est
    /// saisie, celle du bâtiment sinon.
    pub fn categorie_zone(&self, id: &str) -> CategorieContraintes {
        self.zone(id)
            .and_then(|z| z.categorie_contraintes_exterieures)
            .unwrap_or(self.categorie_contraintes_exterieures)
    }

    /// Classe d'exposition au bruit de la zone `id` : celle de la zone si elle est saisie, celle
    /// du bâtiment sinon.
    pub fn zone_bruit_zone(&self, id: &str) -> ZoneBruit {
        self.zone(id)
            .and_then(|z| z.zone_bruit)
            .unwrap_or(self.zone_bruit)
    }
}

/// Données de la fixation des exigences propres à une zone (annexe de l'article R. 172-4,
/// chapitre V).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExigencesZoneSaisie {
    /// Identifiant de la zone du bâtiment.
    pub zone: Id,
    /// Catégorie de contraintes extérieures de la zone ; absente : celle du bâtiment. Elle
    /// module `Mbbruit` (catégorie 3), `Mccat` et fixe `DH_max` des groupes de la zone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categorie_contraintes_exterieures: Option<CategorieContraintes>,
    /// Classe d'exposition au bruit de la zone ; absente : celle du bâtiment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone_bruit: Option<ZoneBruit>,
    /// Catégories propres à certains groupes (parties de bâtiment thermiquement homogènes) :
    /// elles fixent leur `DH_max` (chapitre II, IV). Un groupe absent prend la catégorie de la
    /// zone.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groupes: Vec<ExigencesGroupeSaisie>,
}

/// Catégorie de contraintes extérieures propre à un groupe (annexe de l'article R. 172-4,
/// chapitre V, partie II).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExigencesGroupeSaisie {
    /// Identifiant du groupe dans la zone.
    pub groupe: Id,
    /// Catégorie de contraintes extérieures du groupe.
    pub categorie_contraintes_exterieures: CategorieContraintes,
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
    /// Catégorie 3 : zone dont tous les locaux autres qu'à occupation passagère sont climatisés,
    /// d'un usage cité par le chapitre V (bureaux, médiathèques et bibliothèques, bâtiments
    /// universitaires et d'enseignements atypiques, hôtels, petite enfance, restaurants,
    /// commerces, santé partie jour, aérogares), et dont les baies ne peuvent être ouvertes
    /// (règles d'hygiène et de sécurité) ou situés dans un immeuble de grande hauteur.
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
