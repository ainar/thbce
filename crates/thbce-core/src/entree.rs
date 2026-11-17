// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : la description du projet telle que l'utilisateur la fournit.
//!
//! Chaque champ vient d'une nomenclature de l'annexe III, citée en commentaire, et en reprend
//! le nom en `snake_case` suffixé par l'unité. Aucun champ n'est ajouté s'il ne figure pas dans
//! le texte ; les fiches non encore lues en détail (systèmes, stores vénitiens) complèteront ces
//! types au moment de leur implémentation.
//!
//! La méthode distingue les paramètres intrinsèques d'un composant de ses paramètres
//! d'intégration dans le projet (§1.2). Les premiers sont décrits une fois dans le
//! [`Catalogue`] ; les seconds sont portés par les instances, qui référencent le catalogue par
//! identifiant.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thbce_data::{Departement, ZoneClimatique};

/// Identifiant textuel choisi par l'utilisateur, unique parmi les objets de même nature.
pub type Id = String;

/// Version du format d'entrée décrite par ce module (champ `version_format` d'un projet).
pub const VERSION_FORMAT: u32 = 1;

/// Niveau « projet » : données communes à tous les bâtiments (Th-BCE 2020 §5.1.1, note 1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Projet {
    /// Adresse du JSON Schema du format (`thbce schema`), pour l'aide à la saisie des éditeurs.
    /// Facultative ; ignorée par le moteur.
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// Version du format d'entrée. Facultative ; si elle est donnée, elle doit valoir
    /// [`VERSION_FORMAT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_format: Option<u32>,
    /// Nom libre du projet.
    pub nom: String,
    /// Situation géographique.
    pub localisation: Localisation,
    /// Composants décrits par leurs paramètres intrinsèques.
    #[serde(default)]
    pub catalogue: Catalogue,
    /// Bâtiments du projet.
    pub batiments: Vec<Batiment>,
}

/// Th-BCE 2020 §3.1.2 et §5.1.2 — paramètres d'intégration du climat et du bâtiment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Localisation {
    /// « département ». Fixe la zone climatique et la température de base.
    pub departement: Departement,
    /// « Alt » : altitude du projet, en m.
    pub alt_m: f64,
    /// « Dmer » : distance à la mer, en km.
    pub dmer_km: f64,
    /// « Durb » : distance à l'îlot urbain, en km.
    pub durb_km: f64,
    /// Zone climatique saisie explicitement. Facultative : le moteur la déduit du département
    /// (arrêté du 26 octobre 2010, annexe I). Une valeur qui contredit le département est
    /// conservée et signalée par un avertissement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone_climatique: Option<ZoneClimatique>,
}

impl Localisation {
    /// Zone climatique retenue : celle saisie, sinon celle du département.
    pub fn zone_climatique_retenue(&self) -> ZoneClimatique {
        self.zone_climatique
            .unwrap_or_else(|| ZoneClimatique::du_departement(self.departement))
    }
}

/// Composants décrits une fois par leurs paramètres intrinsèques.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    /// Types de parois opaques.
    #[serde(default)]
    pub parois: Vec<ParoiType>,
    /// Types de baies vitrées.
    #[serde(default)]
    pub baies: Vec<BaieType>,
    /// Masques solaires.
    #[serde(default)]
    pub masques: Vec<MasqueDefinition>,
}

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

/// Th-BCE 2020 §5.4 — espace tampon.
///
/// « Un espace tampon est défini au niveau du bâtiment, et doit nécessairement être associé à au
/// moins un composant d'enveloppe (paroi opaque, baie) d'un groupe » (§5.4.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EspaceTampon {
    /// Identifiant de l'espace tampon, unique dans le bâtiment.
    pub id: Id,
    /// Nature de l'espace tampon (« isolarise »).
    pub nature: NatureEspaceTampon,
}

/// « isolarise » : espace tampon solarisé ou non (§5.4.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum NatureEspaceTampon {
    /// Espace non solarisé (`isolarise = 0`) : combles, garages, celliers non chauffés,
    /// parkings privés, circulations hors volume chauffé, chaufferies.
    NonSolarise(EspaceTamponNonSolarise),
    /// Espace solarisé (`isolarise = 1`) : l'essentiel de son enveloppe en contact avec l'air
    /// extérieur, hors plancher, est vitré (vérandas, serres, atriums, jardins d'hiver).
    Solarise(EspaceTamponSolarise),
}

/// Th-BCE 2020 §5.4.2, tableau 19 — espace tampon non solarisé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EspaceTamponNonSolarise {
    /// « bet_ns » : coefficient `b` de réduction des déperditions, tel que défini dans les
    /// règles Th-Bat (de −5 à 1).
    pub b_et_ns: f64,
}

/// Th-BCE 2020 §5.2.1 et §5.4.2, tableaux 14 et 19 — espace tampon solarisé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EspaceTamponSolarise {
    /// « Aet » : surface au sol, en m².
    pub a_et_m2: f64,
    /// « Uet_pb » : coefficient de déperdition équivalent par le plancher bas, en W/(m².K).
    pub u_et_pb_w_m2k: f64,
    /// « Httfet » : hauteur de tirage thermique en surventilation naturelle, en m.
    pub h_ttf_et_m: f64,
    /// « Cm_et_surf » : capacité thermique quotidienne par m² de surface au sol, en
    /// kJ/(m².K). Absente : 200, valeur conventionnelle du tableau 19, signalée dans les
    /// résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cm_et_surf_kj_m2k: Option<f64>,
    /// « Qv,et,base » : débit d'air de base hors surventilation (défauts d'étanchéité et
    /// entrées d'air), en m³/h. Absent : 1 m³/h par m² de surface au sol (§5.4.3.3.1.4),
    /// signalé dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qv_et_base_m3_h: Option<f64>,
    /// Protections mobiles des parois vitrées (« idprot »). Absent : pas de protection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protections_mobiles: Option<ProtectionsEspaceTampon>,
    /// Parois de l'espace tampon donnant sur l'extérieur (au moins une).
    pub parois: Vec<ParoiEspaceTampon>,
}

/// Th-BCE 2020 §5.3.2, tableau 15 — protections mobiles des parois d'un espace tampon :
/// « conventionnellement de type stores enroulables intérieurs ou extérieurs » (§5.3.3.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProtectionsEspaceTampon {
    /// « Protet_ext » : protections extérieures (vrai) ou intérieures (faux).
    pub exterieures: bool,
}

/// Th-BCE 2020 §5.4.2, tableau 19 — paroi d'un espace tampon solarisé donnant sur
/// l'extérieur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParoiEspaceTampon {
    /// Identifiant de la paroi.
    pub id: Id,
    /// « Aue » : surface, en m².
    pub a_ue_m2: f64,
    /// « Uue » : coefficient de transmission thermique, en W/(m².K).
    pub u_ue_w_m2k: f64,
    /// « αwe », « βwe » et masques : orientation, inclinaison et masques de la paroi
    /// (fiche « Environnement proche », §3.2).
    pub exposition: Exposition,
    /// « Swe_sp » : facteur solaire global sans protection mobile.
    pub swe_sp: f64,
    /// « Swe_ap » : facteur solaire global avec protection mobile en place. Requis si l'espace
    /// tampon a des protections mobiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swe_ap: Option<f64>,
    /// « Tle_sp » : transmission lumineuse globale sans protection mobile.
    pub tle_sp: f64,
    /// « Tle_ap » : transmission lumineuse globale avec protection mobile en place. Requise si
    /// l'espace tampon a des protections mobiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tle_ap: Option<f64>,
    /// « Rouv_max » : ratio d'ouverture maximale de la paroi (0 pour une paroi fixe).
    pub r_ouv_max: f64,
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

/// Th-BCE 2020 §6.2 à 6.5 — système de ventilation mécanique d'une zone : nature du système
/// (« Type_ventilation_mecaniq », §6.3.3.1.1), ventilateurs et réseau de conduits. Les bouches
/// et conduits sont décrits par groupe (`bouches_conduits`) et rattachés à ce système.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Ventilation {
    /// Identifiant du système.
    pub id: Id,
    /// Nature du système et paramètres propres.
    pub systeme: SystemeVentilation,
    /// Réseau de conduits : étanchéité et parties hors volume chauffé (§6.2.3.4, §6.3.3.3.2,
    /// §6.5.3.4).
    pub reseau: ReseauAeraulique,
    /// Raccordement à un puits climatique ou hydraulique (§6.8 à 6.11) : non pris en charge
    /// dans cette version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub puits: Option<Puits>,
}

/// Nature d'un système de ventilation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SystemeVentilation {
    /// Ventilation mécanique simple flux par extraction (§6.5, `Isouf = 0`), autoréglable ou
    /// hygroréglable selon les bouches.
    SimpleFluxExtraction(VentilateurSimpleFlux),
    /// Ventilation mécanique simple flux par insufflation (§6.5, `Isouf = 1`).
    SimpleFluxInsufflation(VentilateurSimpleFlux),
    /// Extraction mécanique basse pression, pour les logements collectifs à conduits individuels
    /// ou de type shunt (§6.4).
    MecaniqueBassePression(VentilateurSimpleFlux),
    /// Ventilation mécanique double flux (§6.3, `Type_ventilation_mecaniq = 1`) : seul système
    /// double flux compatible avec l'usage résidentiel (§6.3.3.1.3).
    DoubleFlux(DoubleFlux),
    /// Ventilation naturelle par conduits (§6.6) : non prise en charge dans cette version.
    NaturelleParConduits,
    /// Ventilation hybride (§6.6) : non prise en charge dans cette version.
    Hybride,
    /// Aération par ouverture des fenêtres (§6.7) : non prise en charge dans cette version.
    Aeration,
    /// Centrale de traitement d'air (§6.3, `Type_ventilation_mecaniq` de 2 à 4) : réservée au
    /// non résidentiel (§6.2.3.1.2 : « La CTA est employée uniquement dans le non
    /// résidentiel »).
    CentraleTraitementAir,
}

/// Th-BCE 2020 §6.4.3 et §6.5.2 — ventilateur d'un système simple flux : puissances en débit
/// de base et de pointe en résidentiel, en occupation et en inoccupation en non résidentiel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VentilateurSimpleFlux {
    /// « Pventbase » : puissance électrique du ventilateur en débit de base, en W. Requise en
    /// maison individuelle ou accolée et en logement collectif ; ne pas la saisir dans les
    /// autres usages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_base_w: Option<f64>,
    /// « Pventpointe » : puissance électrique du ventilateur en débit de pointe, en W. Requise
    /// en maison individuelle ou accolée et en logement collectif.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_pointe_w: Option<f64>,
    /// « Pventocc » : puissance électrique du ventilateur en occupation, en non résidentiel,
    /// en W (§6.5.3.3, éq. 636 et 638). Requise dans les usages autres que la maison
    /// individuelle ou accolée et le logement collectif ; ne pas la saisir en habitation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_occ_w: Option<f64>,
    /// « Pventinocc » : puissance électrique du ventilateur en inoccupation, en non
    /// résidentiel, en W (éq. 637 et 639). Requise comme `p_vent_occ_w` ; 0 si le ventilateur
    /// est arrêté en inoccupation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_inocc_w: Option<f64>,
}

/// Th-BCE 2020 §6.3.2, tableau 61 — groupe de ventilation double flux en habitation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DoubleFlux {
    /// « Pvent,base,rep » : puissance électrique du ventilateur de reprise en débit de base, en W.
    pub p_vent_base_rep_w: f64,
    /// « Pvent,pointe,rep » : puissance électrique du ventilateur de reprise en débit de pointe,
    /// en W.
    pub p_vent_pointe_rep_w: f64,
    /// « Pvent,base,sou » : puissance électrique du ventilateur de soufflage en débit de base, en
    /// W.
    pub p_vent_base_sou_w: f64,
    /// « Pvent,pointe,sou » : puissance électrique du ventilateur de soufflage en débit de
    /// pointe, en W.
    pub p_vent_pointe_sou_w: f64,
    /// Échangeur statique. Absent : pas d'échangeur (`type_échangeur = 0`, éq. 496).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echangeur: Option<Echangeur>,
    /// Batterie de préchauffage (« Is_prechaud ») : non prise en charge dans cette version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prechauffage: Option<bool>,
    /// Humidificateur d'hiver (« Is_ChaudHR ») : non pris en charge dans cette version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub humidification: Option<bool>,
    /// Rafraîchissement évaporatif (« Type_humidification_Eté ») : non pris en charge dans
    /// cette version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rafraichissement_evaporatif: Option<bool>,
    /// Rafraîchissement par surventilation mécanique (§6.12) : non pris en charge dans cette
    /// version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surventilation_mecanique: Option<bool>,
}

/// Th-BCE 2020 §6.3.2, tableau 61, et §6.3.3.4.6 — échangeur d'un double flux.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Echangeur {
    /// Représentation de l'échangeur (« idniveau_detail_echangeur »).
    pub representation: RepresentationEchangeur,
    /// Échangeur rotatif (« Pech », échangeur à roue) : non pris en charge dans cette version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotatif: Option<bool>,
    /// Bypass de l'échangeur (« isbypass = 1 ») et sa régulation. Absent : pas de bypass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bypass: Option<Bypass>,
    /// « isantigel » : sécurité antigel par préchauffage de l'air neuf. Absent ou faux :
    /// l'échangeur est contourné quand un risque de givrage est identifié (éq. 509).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub antigel: Option<bool>,
    /// Génération qui fournit l'énergie de la sécurité antigel (« Wag », éq. 508) : identifiant
    /// d'une génération du bâtiment pourvue d'un générateur de chauffage. La demande lui parvient
    /// comme celle d'un réseau de distribution associé à l'antigel (§8.11.3, éq. 939). Requise
    /// avec `antigel`, sans objet sinon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_antigel: Option<Id>,
    /// « θs,ech,rep,LIM » : température limite basse de sortie côté rejet, en °C (de −10 à
    /// 10). Absente : 5 °C, valeur par défaut du tableau 62 (« autres cas »), signalée dans les
    /// résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theta_s_ech_rep_lim_c: Option<f64>,
}

/// « idniveau_detail_echangeur » : représentation simplifiée (efficacité) ou détaillée
/// (coefficient d'échange global).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RepresentationEchangeur {
    /// Représentation simplifiée : efficacité saisie, corrigée selon son statut (§6.3.3.3.3).
    Simplifiee(EchangeurSimplifie),
    /// Représentation détaillée : efficacité calculée à chaque pas de temps par la méthode NUT
    /// (éq. 499 à 505).
    Detaillee(EchangeurDetaille),
}

/// Échangeur en représentation simplifiée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EchangeurSimplifie {
    /// « εsaisi » : efficacité de l'échangeur en ventilation seule, mesurée aux débits de la
    /// zone neutre en occupation (de 0 à 1).
    pub epsilon_saisi: f64,
    /// « idstatut_echangeur ». Absent : valeur déclarée par le fabricant, « par défaut » selon
    /// le tableau 61, signalé dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statut: Option<StatutEchangeur>,
}

/// « idstatut_echangeur » (§6.3.3.3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatutEchangeur {
    /// 0 : valeur déclarée par le fabricant (éq. 398).
    Declare,
    /// 1 : valeur justifiée par un essai d'un laboratoire accrédité (éq. 397).
    Justifie,
    /// 2 : valeur certifiée (éq. 396).
    Certifie,
}

/// Échangeur en représentation détaillée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EchangeurDetaille {
    /// « UA_saisi » : coefficient d'échange global, en W/K.
    pub ua_w_k: f64,
    /// « idtype_echangeur » : nature des écoulements.
    pub ecoulement: EcoulementEchangeur,
}

/// « idtype_echangeur » (tableau 61).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EcoulementEchangeur {
    /// 1 : écoulements à contre-courant.
    ContreCourant,
    /// 2 : écoulements à courants parallèles.
    CourantsParalleles,
    /// 3 : écoulements à courants croisés, les deux fluides brassés.
    CourantsCroisesBrasses,
}

/// Régulation du bypass de l'échangeur (§6.3.3.4.6.2, éq. 494).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Bypass {
    /// « θext,bypass,hiver » : température d'air neuf au-dessus de laquelle l'échangeur peut
    /// être contourné en saison de chauffage, en °C.
    pub theta_ext_bypass_hiver_c: f64,
    /// « θext,bypass,été » : la même hors saison de chauffage, en °C.
    pub theta_ext_bypass_ete_c: f64,
    /// « θint,bypass,hiver » : température intérieure au-dessus de laquelle l'échangeur peut
    /// être contourné en saison de chauffage, en °C.
    pub theta_int_bypass_hiver_c: f64,
    /// « θint,bypass,été » : la même hors saison de chauffage, en °C.
    pub theta_int_bypass_ete_c: f64,
}

/// Th-BCE 2020 §6.2.2, §6.3.2 et §6.5.2 — réseau de conduits d'un système de ventilation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReseauAeraulique {
    /// « Cletres » : classe d'étanchéité du réseau. Absente : valeur par défaut du tableau 59
    /// (`Kres = 0,0675·10⁻³ m³/(s.m²)` sous 1 Pa), signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classe_etancheite: Option<ClasseEtancheite>,
    /// « Ratfuitevc » : part des fuites en volume chauffé (de 0 à 1). Requise quand les
    /// surfaces des conduits sont saisies (`a_cond_m2` des bouches, `Is_surf_cond_def = 0`) ;
    /// sinon valeur par défaut du tableau 60 (0,25 en maison individuelle, 0,5 en bâtiment
    /// collectif), signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ratfuitevc: Option<f64>,
    /// « Rrep » : résistance thermique de la partie des conduits de reprise située hors volume
    /// chauffé, en m².K/W. Requise si le système a des bouches de reprise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r_rep_m2k_w: Option<f64>,
    /// « Rsouf » : résistance thermique de la partie des conduits de soufflage située hors
    /// volume chauffé, en m².K/W. Requise si le système a des bouches de soufflage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r_souf_m2k_w: Option<f64>,
    /// « b » (« bet ») : ratio de prise en compte des locaux non chauffés que traversent les
    /// conduits (de 0 à 1 ; 1 pour des conduits à l'extérieur, éq. 649 et 471).
    pub b: f64,
}

/// « Cletres » : classe d'étanchéité d'un réseau aéraulique (tableau 59).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ClasseEtancheite {
    /// Classe A.
    A,
    /// Classe B.
    B,
    /// Classe C.
    C,
}

/// Puits climatique ou hydraulique (§6.8 à 6.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Puits {
    /// Puits climatique (§6.8).
    Climatique,
    /// Puits hydraulique (§6.9).
    Hydraulique,
}

/// Th-BCE 2020 §4.1.3, tableau 4 — usages d'une zone.
///
/// Les 28 usages sont reconnus à la lecture. La première version du moteur ne calcule que les
/// usages 1 et 2 (décision Q4) ; les autres sont signalés « non pris en charge ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UsageZone {
    /// 1. Maisons individuelles ou accolées.
    MaisonIndividuelle,
    /// 2. Logements collectifs.
    LogementCollectif,
    /// 3. Bureaux.
    Bureaux,
    /// 4. Enseignement primaire.
    EnseignementPrimaire,
    /// 5. Enseignement secondaire.
    EnseignementSecondaire,
    /// 6. Médiathèques et bibliothèques.
    MediathequeBibliotheque,
    /// 7. Bâtiments universitaires d'enseignement et de recherche, enseignements atypiques.
    Universitaire,
    /// 8. Hôtels 0, 1 et 2 étoiles (partie nuit).
    Hotel012Nuit,
    /// 9. Hôtels 3, 4 et 5 étoiles (partie nuit).
    Hotel345Nuit,
    /// 10. Hôtels 0, 1 et 2 étoiles (partie jour).
    Hotel012Jour,
    /// 11. Hôtels 3, 4 et 5 étoiles (partie jour).
    Hotel345Jour,
    /// 12. Établissements d'accueil de la petite enfance.
    PetiteEnfance,
    /// 13. Restaurants en continu, 18 heures par jour, 7 jours sur 7.
    RestaurantContinu,
    /// 14. Restaurants, 1 repas par jour, 5 jours sur 7.
    Restaurant1Repas5Jours,
    /// 15. Restaurants, 2 repas par jour, 7 jours sur 7.
    Restaurant2Repas7Jours,
    /// 16. Restaurants, 2 repas par jour, 6 jours sur 7.
    Restaurant2Repas6Jours,
    /// 17. Commerces.
    Commerces,
    /// 18. Vestiaires seuls.
    Vestiaires,
    /// 19. Établissements sanitaires avec hébergement.
    SanitaireAvecHebergement,
    /// 20. Établissements de santé (partie nuit).
    SanteNuit,
    /// 21. Établissements de santé (partie jour).
    SanteJour,
    /// 22. Aérogares.
    Aerogare,
    /// 23. Industries ou artisanats 3x8h.
    Industrie3x8,
    /// 24. Industries ou artisanats 8h à 18h.
    Industrie8h18h,
    /// 25. Établissements sportifs municipaux ou scolaires.
    SportMunicipalScolaire,
    /// 26. Restaurants scolaires, 1 repas par jour, 5 jours sur 7.
    RestaurantScolaire1Repas,
    /// 27. Restaurants scolaires, 3 repas par jour, 5 jours sur 7.
    RestaurantScolaire3Repas,
    /// 28. Établissements sportifs privés.
    SportPrive,
}

impl UsageZone {
    /// Numéro d'usage du tableau 4 (§4.1.3).
    pub fn numero(self) -> u8 {
        self as u8 + 1
    }

    /// Usages d'habitation au sens de la surface de référence (§13.1.3 : « Usage = 1 ou 2 »).
    pub fn est_habitation(self) -> bool {
        matches!(self, Self::MaisonIndividuelle | Self::LogementCollectif)
    }

    /// Usage calculé par la version courante du moteur (décision Q4).
    pub fn est_pris_en_charge(self) -> bool {
        self.est_habitation()
    }

    /// « ihebergement » : usage d'habitation ou d'hébergement (§4.1.3, tableau 4) — usages 1 et
    /// 2, hôtels en partie nuit (8 et 9), établissements sanitaires avec hébergement (19) et
    /// établissements de santé en partie nuit (20).
    pub fn i_hebergement(self) -> bool {
        matches!(
            self,
            Self::MaisonIndividuelle
                | Self::LogementCollectif
                | Self::Hotel012Nuit
                | Self::Hotel345Nuit
                | Self::SanitaireAvecHebergement
                | Self::SanteNuit
        )
    }

    /// « ienseignement » : usage d'enseignement ou affilié (§4.1.3, tableau 4) — usages 4, 5,
    /// 7, 26 et 27.
    pub fn i_enseignement(self) -> bool {
        matches!(
            self,
            Self::EnseignementPrimaire
                | Self::EnseignementSecondaire
                | Self::Universitaire
                | Self::RestaurantScolaire1Repas
                | Self::RestaurantScolaire3Repas
        )
    }

    /// Usages « de type habitation » au sens du confort d'été (§13.5.3.3, éq. 2544) : maison
    /// individuelle, logement collectif, établissement sanitaire avec hébergement ; la
    /// température d'inconfort chaud y est ramenée à la consigne pendant la période de sommeil.
    pub fn sommeil_confort_ete(self) -> bool {
        matches!(
            self,
            Self::MaisonIndividuelle | Self::LogementCollectif | Self::SanitaireAvecHebergement
        )
    }
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
    /// Locaux du groupe au sens de l'éclairage (§7.1), pour les usages autres que la maison
    /// individuelle ou accolée et le logement collectif, dont l'éclairage est conventionnel :
    /// ne pas les saisir en habitation. Requis dans les autres usages ; la somme des
    /// `ratio_local` vaut 1.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locaux: Vec<LocalEclairage>,
}

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

/// Th-BCE 2020 §5.6.2 et §5.6.3.2 — entrées d'air d'un groupe.
///
/// La méthode forme une entrée d'air équivalente pour la zone, somme de celles des groupes, puis
/// la répartit entre les groupes au prorata de leurs modules (éq. 147 à 149) et, dans chaque
/// groupe, par quarts entre le haut et le bas des façades au vent et sous le vent (tableau 26).
/// Le groupe se décrit donc par la somme des modules de ses entrées d'air.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EntreesAir {
    /// « M » : somme des modules des entrées d'air du groupe, en m³/h (débit sous 20 Pa).
    pub m_m3_h: f64,
    /// Comportement autoréglable ; absent pour des entrées d'air fixes. Les pressions `ΔP1` et
    /// `ΔP2` sont les mêmes pour tous les groupes d'une zone (§5.6.3.2.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoreglable: Option<AutoreglageEntreesAir>,
    /// « r » : atténuation du débit par un dispositif anti-retour, de 0 à 1 (éq. 146). Absent :
    /// pas de dispositif anti-retour (`r = 1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r: Option<f64>,
    /// Identifiant de l'espace tampon par lequel transite l'air neuf des entrées d'air
    /// (§5.6.3.1, éq. 134 : l'air neuf entre alors à la température de l'espace tampon). Absent :
    /// l'air neuf vient directement de l'extérieur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
}

/// Th-BCE 2020 §5.6.3.2.1, figure 21 — comportement autoréglable d'une entrée d'air.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AutoreglageEntreesAir {
    /// « ΔP1 » : pression du premier changement de comportement, en Pa.
    pub delta_p1_pa: f64,
    /// « ΔP2 » : pression du second changement de comportement, en Pa.
    pub delta_p2_pa: f64,
    /// « M′ » : somme des modules du défaut d'autorégulabilité, en m³/h.
    pub m_prime_m3_h: f64,
}

/// Th-BCE 2020 §6.2.2 — ensemble « bouche et conduit » d'un groupe, en maison individuelle ou
/// accolée et en logement collectif : un circuit de reprise ou un circuit de soufflage.
///
/// Les débits sont les débits d'hygiène du circuit pour le groupe, en pointe (grand débit) et en
/// base. La méthode ne donne pas les débits réglementaires par nombre de pièces : ils se
/// saisissent. Pour un système hygroréglable, le débit de pointe vient des débits maximaux de
/// l'avis technique en cuisine et en WC et des débits d'hygiène des autres pièces ; le débit de
/// base se déduit de `qvarep_spec_m3_h` (§6.2.3.1.1.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoucheConduit {
    /// Identifiant, unique dans le groupe.
    pub id: Id,
    /// « Isouf » : sens du circuit.
    pub isouf: Isouf,
    /// « qspec,rep,conv_pointe » ou « qspec,souffle,conv_pointe » : débit volumique spécifique
    /// conventionnel en pointe, en m³/h, positif. Requis en maison individuelle ou accolée et
    /// en logement collectif ; ne pas le saisir dans les autres usages, qui se décrivent par
    /// `q_spec_occ_m3_h` et `q_spec_inocc_m3_h`.
    #[serde(default, skip_serializing_if = "est_nul")]
    pub q_spec_conv_pointe_m3_h: f64,
    /// « qspec,rep,occ » ou « qspec,soufflé,occ » : débit volumique total repris ou soufflé en
    /// occupation, en m³/h, positif, pour les usages autres que la maison individuelle ou
    /// accolée et le logement collectif (§6.2.3.1.1.1, éq. 383 et 384). Requis dans ces usages.
    /// Le calcul du Bbio ne l'emploie pas : ses débits sont conventionnels (§6.2.3.1.3,
    /// tableau 56-1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_spec_occ_m3_h: Option<f64>,
    /// « qspec,rep,inocc » ou « qspec,soufflé,inocc » : débit volumique total repris ou soufflé
    /// en inoccupation, en m³/h, positif ou nul (éq. 385 et 386). Requis comme
    /// `q_spec_occ_m3_h`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_spec_inocc_m3_h: Option<f64>,
    /// « Crdbnr » : coefficient de réduction des débits en occupation, en non résidentiel
    /// (§6.2.3.2.1, tableau 57) : 1 sans régulation des débits, 0,9 avec un dispositif de
    /// détection d'utilisation du local, 0,8 avec un comptage d'occupants ou des sondes de CO2 ;
    /// une autre valeur si elle vient d'un Avis Technique. Absent : 1, aucune régulation,
    /// signalé dans les résultats. Sans objet en habitation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crdbnr: Option<f64>,
    /// « qspec,rep,conv_base » ou « qspec,souffle,conv_base » : débit volumique spécifique
    /// conventionnel en base, en m³/h, positif et au plus égal au débit de pointe. À saisir sauf
    /// pour un système hygroréglable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_spec_conv_base_m3_h: Option<f64>,
    /// « Qvarepspec » : débit de l'avis technique d'un système hygroréglable, en m³/h
    /// (§6.2.3.1.1.2). Le débit de base en est déduit ; ne pas saisir alors
    /// `q_spec_conv_base_m3_h`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qvarep_spec_m3_h: Option<f64>,
    /// « Dugd » : durée d'utilisation du grand débit, en h par semaine (§6.2.3.2.2.2, tableau
    /// 57) : 14 pour un dispositif à gestion manuelle, 7 avec temporisation ; davantage pour un
    /// passage en grand débit commandé depuis plusieurs pièces (note 3). Absente : 14, gestion
    /// manuelle « par défaut », signalée dans les résultats. Le calcul du Bbio emploie toujours
    /// 14.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dugd_h: Option<f64>,
    /// « Cdep » : coefficient de dépassement (§6.2.3.3.1, tableau 58) : 1,15 pour des
    /// composants autoréglables certifiés, la valeur de l'évaluation pour des composants
    /// hygroréglables. Absent : 1,30, valeur par défaut, signalée dans les résultats. Le calcul
    /// du Bbio emploie toujours 1 (éq. 414).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdep: Option<f64>,
    /// « Acond » : surface externe du conduit, en m², saisie par l'utilisateur
    /// (`Is_surf_cond_def = 0`, §6.2.3.4) ; en maison individuelle, cette possibilité est
    /// conditionnée à un test d'étanchéité à la réception (note 6). Absente : surface par défaut
    /// (éq. 421 à 427). Toutes les bouches d'un système la portent, ou aucune.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a_cond_m2: Option<f64>,
}

/// « Isouf » (§6.2.2) : sens d'un circuit de ventilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Isouf {
    /// 0 : extraction (bouches et conduits de reprise).
    Reprise,
    /// 1 : insufflation (bouches et conduits de soufflage).
    Soufflage,
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

/// Th-BCE 2020 §5.21.2 et §5.21.3 — inertie du groupe sur trois horizons.
///
/// `Ams_surf` et `Ama_surf` « sont saisies dans tous les cas » (§5.21.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Inertie {
    /// Inertie quotidienne (« Typeinertie_quo », tableau 53).
    pub quotidienne: InertieQuotidienne,
    /// Inertie séquentielle (« Typeinertie_seq », tableau 54).
    pub sequentielle: InertieSequentielle,
    /// « Ams_surf » : surface d'échange équivalente au sens de l'inertie séquentielle, en m²/m².
    pub ams_surf_m2_m2: f64,
    /// Inertie annuelle (« Typeinertie_ann »).
    pub annuelle: InertieAnnuelle,
    /// « Ama_surf » : surface d'échange équivalente au sens de l'inertie annuelle, en m²/m².
    pub ama_surf_m2_m2: f64,
}

/// « Typeinertie_quo » (§5.21.3, tableau 53).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InertieQuotidienne {
    /// 0 : personnalisée, saisie directe de `Amq_surf` (m²/m²) et `Cmq_surf` (kJ/(K.m²)).
    Personnalisee {
        /// « Amq_surf ».
        amq_surf_m2_m2: f64,
        /// « Cmq_surf ».
        cmq_surf_kj_m2k: f64,
    },
    /// 1 : très légère.
    TresLegere,
    /// 2 : légère.
    Legere,
    /// 3 : moyenne.
    Moyenne,
    /// 4 : lourde.
    Lourde,
    /// 5 : très lourde.
    TresLourde,
}

/// « Typeinertie_seq » (§5.21.3, tableau 54).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InertieSequentielle {
    /// 0 : personnalisée, saisie directe de `Cms_surf` (kJ/(K.m²)).
    Personnalisee {
        /// « Cms_surf ».
        cms_surf_kj_m2k: f64,
    },
    /// 1 : par défaut.
    Defaut,
    /// 2 : très légère.
    TresLegere,
    /// 3 : légère.
    Legere,
    /// 4 : moyenne.
    Moyenne,
    /// 5 : lourde.
    Lourde,
}

/// « Typeinertie_ann » (§5.21.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InertieAnnuelle {
    /// 0 : personnalisée, saisie directe de `Cma_surf` (kJ/(K.m²)).
    Personnalisee {
        /// « Cma_surf ».
        cma_surf_kj_m2k: f64,
    },
    /// 1 : par défaut, même valeur que l'inertie séquentielle (éq. 316).
    Defaut,
}

/// Th-BCE 2020 §3.2.2 — paramètres d'intégration d'une surface exposée au rayonnement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Exposition {
    /// « α » : azimut, en degrés (0 : sud ; 90 : ouest ; 180 : nord ; 270 : est).
    pub azimut_deg: f64,
    /// « β » : inclinaison, en degrés (0 : horizontale vers le haut ; 90 : verticale ;
    /// 180 : horizontale vers le bas).
    pub inclinaison_deg: f64,
    /// « lpb » : largeur de la paroi, en m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lpb_m: Option<f64>,
    /// « hpb » : hauteur de la paroi, en m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hpb_m: Option<f64>,
    /// « Id_masque » : identifiants des masques protégeant la surface (« Nmasque », 0 à 7).
    #[serde(default)]
    pub masques: Vec<Id>,
}

/// Masque du catalogue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MasqueDefinition {
    /// Identifiant du masque.
    pub id: Id,
    /// Géométrie du masque.
    pub masque: Masque,
}

/// Th-BCE 2020 §3.2 — masques solaires (identifiants du texte : `E_AZ`, `E_Az_cad`, `E_LV`,
/// `vd`, `vg`, `h`).
///
/// Les masques proches ne portent que sur les composants verticaux (§3.2.3) ; les masques
/// lointains portent sur toute inclinaison.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Masque {
    /// Masque lointain défini par tranches azimutales (`E_AZ`).
    LointainAzimutal {
        /// « γi » : hauteur de l'horizon vu du centre du composant, en degrés (0 à 90), pour
        /// chacune des 36 tranches de 10° (§3.2.3). Les tranches sont repérées par rapport à la
        /// normale au composant : la tranche 1 commence à l'opposé de la normale (Φ = −180°) et
        /// les tranches 18 et 19 encadrent la normale (figure 10).
        gamma_i_deg: Vec<f64>,
    },
    /// Masque lointain constitué d'arbres à feuilles caduques (`E_Az_cad`) : mêmes tranches,
    /// actif pendant la seule saison végétative conventionnelle.
    LointainArbresCaducs {
        /// « γi », comme pour le masque lointain azimutal.
        gamma_i_deg: Vec<f64>,
    },
    /// Masque lointain de type plan vertical éloigné.
    LointainPlanVertical {
        /// « dE » : distance entre le masque et la paroi, en m.
        de_m: f64,
        /// « hpE » : hauteur du masque par rapport au centre de la paroi, en m.
        hpe_m: f64,
    },
    /// Masque proche vertical droit.
    ProcheVerticalDroit {
        /// « dvd » : profondeur du masque, en m.
        dvd_m: f64,
        /// « dpd » : distance entre la paroi et le masque, en m.
        dpd_m: f64,
    },
    /// Masque proche vertical gauche.
    ProcheVerticalGauche {
        /// « dvg » : profondeur du masque, en m.
        dvg_m: f64,
        /// « dpg » : distance entre la paroi et le masque, en m.
        dpg_m: f64,
    },
    /// Masque proche horizontal.
    ProcheHorizontal {
        /// « dhm » : profondeur du masque, en m.
        dhm_m: f64,
        /// « dhp » : distance entre la paroi et le masque, en m.
        dhp_m: f64,
    },
}

/// Th-BCE 2020 §5.17.2 — paramètres intrinsèques d'une paroi opaque.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParoiType {
    /// Identifiant du type de paroi.
    pub id: Id,
    /// « Uk » : coefficient de transmission thermique, en W/(m².K), dans la position `incl_k`.
    pub u_k_w_m2k: f64,
    /// « Inclk » : position pour laquelle `Uk` est donné. Obligatoire, sans valeur par défaut :
    /// une position supposée changerait U en silence. Le moteur en déduit le coefficient de la
    /// paroi dans son inclinaison réelle (§5.17.3, éq. 294). Correspondance avec les règles
    /// Th-Bat, d'après les résistances superficielles cumulées que le texte retire de `1/Uk` :
    /// `verticale` pour un U de paroi verticale (0,17 m².K/W) ; `horizontale_flux_ascendant` pour
    /// un U de plancher haut (0,14) ; `horizontale_flux_descendant` pour un U de plancher bas
    /// (0,21).
    pub incl_k: PositionParoi,
    /// « Sf-c,k » : facteur solaire pour le calcul du Cep (paroi non végétalisée).
    pub sf_c_k: f64,
    /// « Sf-e,k » : facteur solaire pour le calcul de DH (paroi non végétalisée).
    pub sf_e_k: f64,
    /// Toiture végétalisée (« Typevegetal » = 1) et ses facteurs solaires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vegetalisation: Option<Vegetalisation>,
}

/// « Inclk » (§5.17.2) : position de référence du coefficient `Uk` d'une paroi opaque.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PositionParoi {
    /// 1 : paroi verticale (Th-Bat : paroi verticale, flux horizontal).
    Verticale,
    /// 2 : paroi horizontale, flux ascendant (Th-Bat : plancher haut).
    HorizontaleFluxAscendant,
    /// 3 : paroi horizontale, flux descendant (Th-Bat : plancher bas).
    HorizontaleFluxDescendant,
}

/// Facteurs solaires d'une toiture végétalisée (§5.17.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Vegetalisation {
    /// « Sf-c,v,k » : facteur solaire pour le calcul du Cep.
    pub sf_c_v_k: f64,
    /// « Sf-e,v,k » : facteur solaire pour le calcul de DH.
    pub sf_e_v_k: f64,
}

/// Paroi opaque d'un groupe : paramètres d'intégration (§5.17.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParoiOpaque {
    /// Identifiant de la paroi.
    pub id: Id,
    /// Référence à un [`ParoiType`] du catalogue.
    pub type_paroi: Id,
    /// « Ak » : surface de la paroi, en m².
    pub a_k_m2: f64,
    /// Orientation, inclinaison et masques.
    pub exposition: Exposition,
    /// Identifiant de l'espace tampon sur lequel donne la paroi (§5.17.3, « Prise en compte des
    /// espaces tampons »). Absent : la paroi donne sur l'extérieur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
}

/// Th-BCE 2020 §5.10.2 — paramètres intrinsèques d'une baie vitrée.
///
/// Les matrices de facteurs des stores vénitiens (`M_Sw_BC`, `M_Sw_E`…) seront ajoutées avec la
/// fiche 5.10 (lot 4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BaieType {
    /// Identifiant du type de baie.
    pub id: Id,
    /// « Usp,vert,b » : U de la baie verticale sans protection mobile en place, en W/(m².K).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub u_sp_vert_w_m2k: Option<f64>,
    /// « Usp,horiz,b » : U de la baie horizontale sans protection mobile en place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub u_sp_horiz_w_m2k: Option<f64>,
    /// « Uap,vert,b » : U de la baie verticale avec protection mobile en place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub u_ap_vert_w_m2k: Option<f64>,
    /// « Uap,horiz,b » : U de la baie horizontale avec protection mobile en place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub u_ap_horiz_w_m2k: Option<f64>,
    /// Facteurs de la baie sans protection mobile en place (indice « sp »).
    pub sans_protection: FacteursBaie,
    /// Facteurs de la baie avec protection mobile de type volet ou store enroulable en place
    /// (indice « ap »). Requis si `protection_mobile` est renseignée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avec_protection: Option<FacteursBaie>,
    /// Première protection mobile de la baie et sa gestion (§5.9.2). Absente : baie sans
    /// protection mobile (`Type-PM = 0`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protection_mobile: Option<ProtectionMobile>,
    /// Ouvrant de la baie (§5.13.2, « Est_Ouvrable »). Absent : baie non ouvrable, qui ne
    /// contribue pas à la surventilation naturelle. La gestion de l'ouverture est manuelle ; la
    /// gestion automatique n'est pas prise en charge dans cette version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ouvrant: Option<Ouvrant>,
}

/// Th-BCE 2020 §5.13.3.2.3 — ouvrant d'une baie. Renseigner soit `r_ouv_max`, soit
/// `type_ouvrant`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Ouvrant {
    /// « Rouvmax,b » : rapport de la surface maximale d'ouverture à la surface en tableau de la
    /// baie, parties fixes comprises, entre 0 et 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r_ouv_max: Option<f64>,
    /// Type d'ouvrant. Sans `r_ouv_max`, le moteur applique la valeur par défaut du tableau 44
    /// et la signale dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_ouvrant: Option<TypeOuvrant>,
}

/// Type d'ouvrant (§5.13.3.2.3, tableau 44).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeOuvrant {
    /// Ouvrant à la française ou à l'anglaise, angle d'ouverture de 90° : 80 %.
    FrancaiseOuAnglaise,
    /// Ouvrant à l'italienne ou à soufflet, angle d'ouverture de 15° : 40 %.
    ItalienneOuSoufflet,
    /// Soufflet d'oscillo-battant, angle d'ouverture de 7° : 20 %.
    SouffletOscilloBattant,
    /// Coulissant : 40 %.
    Coulissant,
}

/// « expo_BR » (§5.13.2 ; fiche d'application §5.11) : classe d'exposition au bruit d'une baie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExpoBr {
    /// BR1.
    Br1,
    /// BR2.
    Br2,
    /// BR3.
    Br3,
}

/// Th-BCE 2020 §5.9.2 et §5.12.2 — protection mobile d'une baie et sa gestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProtectionMobile {
    /// « Type-PM » : type de la première protection mobile. Le store vénitien n'est pas pris en
    /// charge dans cette version : la validation le refuse.
    pub type_pm: TypePm,
    /// « Type-GPM » : gestion de la première protection mobile. La gestion automatique n'est pas
    /// prise en charge dans cette version : la validation la refuse.
    pub gestion: GestionPm,
    /// « Protext » : vrai si la protection est extérieure, faux si elle est intérieure.
    pub exterieure: bool,
    /// « PM2 » : présence d'une seconde protection mobile, intérieure, gérée manuellement, dont
    /// la seule fonction est d'éviter l'éblouissement.
    pub seconde_protection: bool,
    /// « Typo_PM » : typologie de la protection pour le passage de l'air quand elle est fermée
    /// (§5.12.3.1, tableau 40). Absente : typologie 0, « valeur par défaut » du texte (taux de
    /// passage de 0 %), signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typo_pm: Option<TypoPm>,
}

/// « Type-PM » (§5.9.2) : type de la première protection mobile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypePm {
    /// 1 : volet.
    Volet,
    /// 2 : store enroulable.
    StoreEnroulable,
    /// 3 : store vénitien (à lames orientables). Non pris en charge dans cette version.
    StoreVenitien,
}

/// « Type-GPM » (§5.9.2) : gestion d'un volet ou d'un store enroulable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GestionPm {
    /// 1 : automatique avec dérogation. Non prise en charge dans cette version.
    AutomatiqueAvecDerogation,
    /// 2 : manuelle non motorisée.
    ManuelleNonMotorisee,
    /// 3 : manuelle motorisée (y compris avec commande générale dérogeable, §5.9.3.1).
    ManuelleMotorisee,
}

/// « Typo_PM » (§5.12.3.1, tableau 40) : typologie d'une protection mobile selon son taux de
/// passage d'air en position fermée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypoPm {
    /// 0 : valeur par défaut, volets isolants ; 0 %.
    Etanche,
    /// 1 : tablier non ou faiblement ajouré (moins de 10 % de sa surface) ; 10 %. Exemples du
    /// texte : volets roulants, battants ou coulissants sans projection, stores enroulables
    /// opaques.
    FaiblementAjouree,
    /// 2 : tablier ajouré (10 à 30 %) ; 25 %. Exemples : volets roulants à agrafes ou à
    /// micro-lames ajourées.
    Ajouree,
    /// 3 : tablier fortement ajouré (30 à 50 %) ; 50 %. Exemples : volets battants persiennés
    /// sans projection, moustiquaires.
    FortementAjouree,
    /// 4 : tablier très fortement ajouré (plus de 50 %) ou protection à projection ; 75 %.
    TresFortementAjouree,
}

/// Facteurs solaires et lumineux d'une baie dans un état de protection (§5.10.2).
///
/// Trois grandeurs ont une valeur par défaut donnée par la méthode (§5.10.3.6.3) et sont
/// facultatives : les facteurs solaires « -c » (éq. 215 et 216) et `tlid` (éq. 218). Toute
/// valeur par défaut appliquée est listée dans les résultats (`valeurs_par_defaut`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FacteursBaie {
    /// « Sw1-c » : composante courte longueur d'onde (directe), calcul du Cep.
    ///
    /// Les trois composantes « -c » se saisissent ensemble ou pas du tout. Absentes, la valeur
    /// par défaut du texte s'applique au facteur solaire global `Sw-c = Sw1-c + Sw2-c + Sw3-c` :
    /// sans protection mobile, `Swsp-c = Swsp-e − 0,1` (éq. 215) ; avec protection mobile,
    /// `Swap-c = Swap-e` (éq. 216). Le texte renvoie aux règles Th-S pour la décomposition en
    /// trois composantes : le moteur garde les proportions des composantes « -e » (question
    /// Q30).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sw1_c: Option<f64>,
    /// « Sw2-c » : composante grande longueur d'onde et convective, calcul du Cep. Voir `sw1_c`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sw2_c: Option<f64>,
    /// « Sw3-c » : composante liée à la lame d'air intérieure ventilée, calcul du Cep. Voir
    /// `sw1_c`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sw3_c: Option<f64>,
    /// « Sw1-e » : composante courte longueur d'onde (directe), calcul de DH.
    pub sw1_e: f64,
    /// « Sw2-e » : composante grande longueur d'onde et convective, calcul de DH.
    pub sw2_e: f64,
    /// « Sw3-e » : composante liée à la lame d'air intérieure ventilée, calcul de DH.
    pub sw3_e: f64,
    /// « Tli » : facteur de transmission lumineuse global.
    pub tli: f64,
    /// « Tlid » : transmission lumineuse du flux direct transmis sous forme diffuse. Absente :
    /// 0, valeur par défaut du texte (éq. 218).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tlid: Option<f64>,
}

/// Baie vitrée d'un groupe : paramètres d'intégration (§5.10.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Baie {
    /// Identifiant de la baie.
    pub id: Id,
    /// Référence à un [`BaieType`] du catalogue.
    pub type_baie: Id,
    /// « Ab » : surface de la baie, en m².
    pub a_b_m2: f64,
    /// Orientation, inclinaison et masques.
    pub exposition: Exposition,
    /// « expo_BR » : classe d'exposition au bruit de la baie (§5.13.2). Requise si le type de
    /// baie a un ouvrant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expo_br: Option<ExpoBr>,
    /// Identifiant de l'espace tampon sur lequel donne la baie (§5.10.3, « IdET »). Une baie
    /// ne peut donner que sur un espace tampon solarisé. Absent : la baie donne sur
    /// l'extérieur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
}

/// Th-BCE 2020 §5.19.2 — pont thermique.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PontThermique {
    /// Identifiant du pont thermique.
    pub id: Id,
    /// « Ψl » : coefficient de transmission thermique linéique, en W/(m.K).
    pub psi_l_w_mk: f64,
    /// « ll » : longueur, en m.
    pub l_l_m: f64,
    /// « Sf-c,l » : facteur solaire pour le calcul du Cep, en m⁻¹.
    pub sf_c_l: f64,
    /// « Sf-e,l » : facteur solaire pour le calcul de DH, en m⁻¹ (le texte dit « Tic » :
    /// décision Q14).
    pub sf_e_l: f64,
    /// Orientation, inclinaison et masques du pont thermique (§5.18.3). Champ facultatif.
    /// Présent : description détaillée. **Absent : la description simplifiée s'applique**, où le
    /// pont thermique est réparti en quatre ponts verticaux orientés au sud, à l'ouest, au nord
    /// et à l'est, d'un quart de la longueur chacun, sans masque (éq. 298).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exposition: Option<Exposition>,
    /// Identifiant de l'espace tampon sur lequel donne le pont thermique (§5.19.3). Absent : le
    /// pont thermique donne sur l'extérieur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
}

// ---------------------------------------------------------------------------------------------
// Systèmes de chauffage et de refroidissement (Th-BCE 2020 §8.1 à 8.10) : émetteurs du groupe,
// programmation des relances, distributions du groupe et intergroupes, générations.
// ---------------------------------------------------------------------------------------------

/// Th-BCE 2020 §8.14, §8.17.2 et §8.4.2 — génération : ensemble de générateurs qui reçoit les
/// demandes d'un ou de plusieurs réseaux de distribution intergroupes de chauffage, de
/// refroidissement ou d'ECS, et les répartit entre ses générateurs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Generation {
    /// Identifiant de la génération, unique dans le bâtiment.
    pub id: Id,
    /// « idraccord_reseau » (§8.4.2) : type de raccordement de la génération aux réseaux de
    /// distribution. `permanent` : les groupes desservis suivent la saison de fonctionnement de
    /// la génération (éq. 840) ; `avec_isolement` : chaque groupe garde ses propres saisons
    /// (éq. 841).
    pub raccordement_reseaux: RaccordementReseaux,
    /// « idpos » (§8.17.2) : emplacement de la génération, en ou hors volume chauffé. Il fixe
    /// la température d'ambiance des générateurs et des ballons (§8.17.3.5.8) et le caractère
    /// récupérable de leurs pertes (éq. 1058, fiche 11.1).
    pub position: PositionGeneration,
    /// « idtype_priorite » (§8.17.1) : mode de gestion des priorités entre les générateurs.
    /// Requis dès que deux générateurs assurent le même poste ; sans objet sinon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priorite: Option<TypePriorite>,
    /// « idgestion_ch » et « θwm_ch » (§8.17.3.5.10.1) : gestion de la température de
    /// fonctionnement en chauffage. Requise si la génération alimente des réseaux hydrauliques
    /// de chauffage ; sans objet pour une génération sur air ambiant (réseaux fictifs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gestion_temperature_chauffage: Option<GestionTemperatureGeneration>,
    /// « idgestion_fr » et « θwm_fr » : de même en refroidissement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gestion_temperature_refroidissement: Option<GestionTemperatureGeneration>,
    /// Générateurs de la génération (tableau 111). La génération doit pouvoir assurer chacun
    /// des postes de ses réseaux (§8.17.3.1) ; elle compte au moins un générateur ou un
    /// chauffe-eau thermodynamique.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generateurs: Vec<GenerateurSaisi>,
    /// Assemblages de production d'ECS avec stockage (§9.2 et §9.14) : un ballon, un générateur
    /// de base de la génération et, le cas échéant, un appoint intégré. Chaque assemblage est
    /// le premier sollicité pour l'ECS (§8.17.3.6).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stockages: Vec<StockageSaisi>,
    /// Chauffe-eau thermodynamiques décrits par les résultats de leur essai selon la norme
    /// NF EN 16147 (§9.22) : le moteur en déduit le ballon et la pompe à chaleur d'un
    /// assemblage « base seule ».
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chauffe_eau_thermodynamiques: Vec<ChauffeEauThermodynamique>,
    /// Chauffe-eau solaires individuels (§9.14 et §9.15, kits CESI §9.23) : ballon dont le
    /// générateur de base est une boucle solaire, avec un appoint intégré, un appoint séparé
    /// instantané ou sans appoint. Comme les autres assemblages avec stockage, ils sont les
    /// premiers sollicités pour l'ECS (§8.17.3.6).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chauffe_eau_solaires: Vec<ChauffeEauSolaire>,
    /// Systèmes solaires combinés (fiches 9.18 à 9.20) : un ballon à quatre zones chauffé par
    /// une boucle solaire, qui assure l'ECS et une partie du chauffage par l'échangeur d'une
    /// boucle de chauffage en zone 2, avec un appoint intégré en zone 3. Premiers sollicités pour
    /// l'ECS et pour le chauffage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub systemes_solaires_combines: Vec<SystemeSolaireCombine>,
    /// Productions solaires collectives à appoints décentralisés du logement collectif
    /// (fiches 9.16 et 9.17) : CESCI (capteurs collectifs, ballons individuels à appoint
    /// intégré ou séparé) et CESCAI (capteurs et ballon solaire collectifs, ballons
    /// individuels à appoint intégré). Une génération en porte au plus une, qui assure toute son
    /// ECS.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chauffe_eau_solaires_collectifs: Vec<ChauffeEauSolaireCollectif>,
}

/// Production centralisée à appoints décentralisés (« PCAD », fiches 9.16 et 9.17) : un champ
/// de capteurs et une boucle solaire collectifs, des ensembles de ballons individuels
/// identiques et, pour un CESCAI, un ballon solaire central relié aux ballons individuels par
/// le réseau d'ECS intergroupe bouclé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChauffeEauSolaireCollectif {
    /// Identifiant, unique parmi les générateurs et assemblages de la génération.
    pub id: Id,
    /// « Type_PCAD ».
    pub type_pcad: TypePcad,
    /// Orientation, inclinaison et masques du plan des capteurs (fiche 3.2).
    pub exposition_capteurs: Exposition,
    /// Capteurs solaires du champ collectif (« A » : superficie totale du champ).
    pub capteurs: CapteursSolaires,
    /// Boucle solaire collective ; ses échangeurs sont ceux de tous les ballons qu'elle
    /// chauffe. Un CESCI est régulé sur l'ensoleillement (éq. 1981).
    pub boucle: BoucleSolaireSaisie,
    /// Ballon solaire central d'un CESCAI (« ballon base seule », éq. 2065 et 2066) ; sans
    /// objet pour un CESCI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ballon_central: Option<BallonSaisi>,
    /// Ensembles de ballons individuels strictement identiques (« nbens », au moins un).
    pub ensembles: Vec<EnsembleBallons>,
}

/// « Type_PCAD » (tableaux 294 et 295).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypePcad {
    /// 0 : CESCI, chauffe-eau solaire collectif individualisé, appoint intégré aux ballons
    /// individuels (« Typeprod_stockage = 1 », éq. 1979).
    Cesci,
    /// 1 : CESCAI, chauffe-eau solaire collectif à appoint individuel : ballon solaire central,
    /// ballons individuels à appoint intégré (éq. 2063 à 2065).
    Cescai,
    /// 2 : CESCI-AS, appoint séparé instantané (« Typeprod_stockage = 3 », éq. 1979) : les
    /// générateurs instantanés d'ECS de la génération complètent l'eau préchauffée.
    CesciAppointSepare,
}

/// Ensemble de ballons individuels strictement identiques d'une production collective
/// (« {B} », fiches 9.16 et 9.17).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnsembleBallons {
    /// Identifiant, unique dans la production.
    pub id: Id,
    /// « nbsto^B » : nombre de ballons de l'ensemble (au moins 1).
    pub nb_ballons: u32,
    /// Ballon individuel ; l'échangeur solaire ou du réseau intergroupe et sa sonde sont en zone
    /// basse (éq. 1980 et 2066).
    pub ballon: BallonSaisi,
    /// Appoint intégré de chaque ballon (CESCI et CESCAI) : générateur de la génération
    /// assurant l'ECS, dont `rdim` est le nombre de ballons ; sans objet pour un CESCI à appoint
    /// séparé.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appoint: Option<AppointBallon>,
}

/// Système solaire combiné (fiches 9.19 et 9.20), un assemblage par objet (« nbassembl = 1 »,
/// éq. 2133 et 2147).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SystemeSolaireCombine {
    /// Identifiant, unique parmi les assemblages de la génération.
    pub id: Id,
    /// Orientation, inclinaison et masques du plan des capteurs (fiche 3.2).
    pub exposition_capteurs: Exposition,
    /// Capteurs solaires.
    pub capteurs: CapteursSolaires,
    /// Boucle solaire (échangeur en zone 1, éq. 2128 et 2142).
    pub boucle: BoucleSolaireSaisie,
    /// Ballon de stockage.
    pub ballon: BallonSaisi,
    /// Appoint intégré en zone 3, sonde en zone 3 (éq. 2130 et 2144), consigne de 55 °C.
    pub appoint: AppointSsc,
    /// Appoint de chauffage : indépendant (« Typeprod_stockage = 4 », fiche 9.19) ou raccordé
    /// à l'assemblage (« = 5 », fiche 9.20).
    pub raccordement: RaccordementSsc,
}

/// Appoint intégré d'un système solaire combiné.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AppointSsc {
    /// Identifiant d'un générateur de la génération : générateur à effet Joule de fonction
    /// `ecs` avec un appoint de chauffage indépendant ; générateur assurant le chauffage et
    /// l'ECS (chaudière, pompe à chaleur double service) avec un appoint raccordé.
    pub generateur: Id,
    /// « typegest_ap ».
    pub programmation: ProgrammationBallon,
    /// « faux » : fraction du volume chauffée par l'appoint, strictement entre 0 et 1.
    /// Absente : 0,5 (§9.9.3.1), signalée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub f_aux: Option<f64>,
    /// « Δθap » : hystérésis, en K. Absente : 2 K.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_theta_k: Option<f64>,
}

/// Raccordement de l'appoint de chauffage d'un système solaire combiné.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RaccordementSsc {
    /// 4 : appoint de chauffage par un système indépendant (les autres générateurs de
    /// chauffage de la génération) ; la boucle de chauffage fonctionne si la zone de son
    /// échangeur dépasse l'ambiance de `Δθreg^bcl_ch` (éq. 2120).
    Independant {
        /// « Δθreg^bcl_ch », en K (sans valeur conventionnelle dans le texte).
        delta_theta_reg_boucle_chauffage_k: f64,
    },
    /// 5 : appoint de chauffage par le générateur d'appoint du ballon, appelé une seconde fois
    /// (éq. 2150) ; la boucle de chauffage ne fonctionne que si le ballon couvre toute la
    /// demande (éq. 2123 à 2125).
    Raccorde,
}

/// Chauffe-eau solaire individuel : assemblage « ballon base solaire » (§9.14, fiche 9.15),
/// décrit composant par composant ou, pour un kit caractérisé selon la norme NF EN 12976, par
/// les paramètres de l'outil IdCESI (§9.23).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChauffeEauSolaire {
    /// Identifiant, unique parmi les assemblages de la génération.
    pub id: Id,
    /// « Nbassemblage » : nombre de chauffe-eau identiques (au moins 1). Un générateur
    /// d'appoint intégré désigné représente autant de générateurs identiques (`rdim`).
    pub nb_assemblages: u32,
    /// Orientation, inclinaison et masques du plan des capteurs (fiche 3.2).
    pub exposition_capteurs: Exposition,
    /// Description du chauffe-eau.
    pub description: DescriptionCesi,
    /// Appoint.
    pub appoint: AppointCesi,
}

/// Description d'un chauffe-eau solaire individuel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum DescriptionCesi {
    /// Boucle solaire (§9.15) et ballon (§9.9) décrits séparément (produits caractérisés selon
    /// la norme NF EN 12977, §9.23.3.1).
    Composants {
        /// Capteurs solaires.
        capteurs: CapteursSolaires,
        /// Boucle solaire.
        boucle: BoucleSolaireSaisie,
        /// Ballon de stockage.
        ballon: BallonSaisi,
    },
    /// Kit caractérisé selon les normes NF EN 12976-2 et ISO 9459-5 (« TypeCES = 1 »),
    /// traduit par les éq. (2263) à (2273).
    Kit(KitCesi),
}

/// Capteurs solaires d'une boucle (tableau 292).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapteursSolaires {
    /// « A » : superficie d'entrée des capteurs d'un chauffe-eau, en m².
    pub surface_m2: f64,
    /// « η0 », « a1 », « a2 » : valeurs certifiées ou valeurs par défaut du tableau 293.
    pub performances: PerformancesCapteurs,
    /// « Kθ » : facteur d'angle d'incidence, de 0 à 1 (sans valeur par défaut dans le texte).
    pub k_theta: f64,
}

/// Performances des capteurs (§9.15.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PerformancesCapteurs {
    /// Valeurs certifiées selon les normes NF EN 12975-1 et 12975-2.
    Certifie {
        /// « η0 » : rendement optique, de 0 à 1.
        eta0: f64,
        /// « a1 » : coefficient de pertes du premier ordre, en W/(m².K).
        a1_w_m2k: f64,
        /// « a2 » : coefficient de pertes du second ordre, en W/(m².K²).
        a2_w_m2k2: f64,
    },
    /// Valeurs par défaut du tableau 293, signalées dans les résultats.
    Defaut {
        /// Type de capteur.
        type_capteur: TypeCapteurSolaire,
    },
}

/// Type de capteur (tableau 293).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeCapteurSolaire {
    /// Capteur non vitré.
    NonVitre,
    /// Capteur plan vitré.
    Vitre,
    /// Capteur à tubes sous vide.
    Tubulaire,
}

/// Boucle solaire (tableau 292).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoucleSolaireSaisie {
    /// « (UA)te » : pertes des tuyauteries vers l'extérieur, en W/K.
    pub ua_te_w_k: f64,
    /// « (UA)ti » : pertes des tuyauteries vers l'intérieur du bâtiment, en W/K.
    pub ua_ti_w_k: f64,
    /// « Pnp » : puissance nominale de la pompe, en W.
    pub p_np_w: f64,
    /// Transfert vers le ballon : échangeur (« (UA)hx », 100 W/K par m² de capteurs par
    /// défaut, tableau 292) ou sans échangeur (§9.15.3.1.2).
    pub transfert: TransfertBoucle,
    /// « Typeregul_BS » : régulation de la boucle.
    pub regulation: RegulationBoucleSaisie,
    /// « αp » : part de la puissance de la pompe transmise au fluide. Absente : 0,5, valeur
    /// conventionnelle du tableau 292, signalée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha_p: Option<f64>,
    /// « ṁCp » : débit calorifique de la boucle, en W/K. Absent : 58 W/K par m² de capteurs,
    /// valeur conventionnelle du tableau 292, signalée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub m_cp_w_k: Option<f64>,
    /// « hrelech_base » : hauteur relative de l'échangeur solaire à partir du bas du ballon,
    /// de 0 à 1 (éq. 1781 à 1785). Absente : échangeur ponctuel en zone basse.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h_rel_ech: Option<f64>,
}

/// Transfert de la boucle vers le ballon (§9.15.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TransfertBoucle {
    /// Échangeur dans le ballon.
    Echangeur {
        /// « (UA)hx », en W/K. Absent : valeur conventionnelle `100·A`, signalée.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ua_hx_w_k: Option<f64>,
    },
    /// Sans échangeur : le fluide de la boucle est l'eau du ballon.
    SansEchangeur,
}

/// « Typeregul_BS » (§9.15.3.2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RegulationBoucleSaisie {
    /// 0 : sur l'écart de température entre la sortie des capteurs et le ballon, avec
    /// « Δθreg » conventionnel (1 K en habitation, éq. 1968).
    Temperature,
    /// 1 : sur l'ensoleillement.
    Irradiance {
        /// « Isrmin », en W/m². Absent : 200 W/m², valeur conventionnelle du tableau 292,
        /// signalée.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        isr_min_w_m2: Option<f64>,
    },
}

/// Kit CESI caractérisé selon la norme NF EN 12976 (« TypeCES = 1 », §9.23.3.2.2, tableau 308).
/// Le §9.23.3.2.2 le traite comme un « ballon base seule solaire avec appoint intégré » : son
/// appoint est `integre`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KitCesi {
    /// « A » : superficie d'entrée des capteurs, en m².
    pub surface_m2: f64,
    /// « Vtot » : volume du ballon, en L.
    pub v_tot_l: f64,
    /// « Pnp » : puissance nominale de la pompe, en W.
    pub p_np_w: f64,
    /// « AC* » : superficie de captage équivalente, en m².
    pub ac_etoile_m2: f64,
    /// « UC* » : coefficient de pertes de captage rapporté à `AC*`, en W/(m².K).
    pub uc_etoile_w_m2k: f64,
    /// « US » : coefficient de pertes de stockage, en W/K.
    pub us_w_k: f64,
    /// « faux » : fraction effective du ballon concernée par l'appoint, strictement entre 0 et
    /// 1 (requise avec un appoint intégré, sans objet sinon).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub f_aux: Option<f64>,
    /// « θmax » : température maximale du ballon, en °C (de 55 à 100).
    pub theta_max_c: f64,
    /// « Kθ » : facteur d'angle d'incidence, de 0 à 1, sortie de l'outil IdCESI
    /// (§9.23.3.2.3). Requis : le texte ne lui donne aucune valeur (lecture Q52).
    pub k_theta: f64,
}

/// Appoint d'un chauffe-eau solaire (« Type_prod_stockage », §9.14.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum AppointCesi {
    /// 0 : ballon base sans appoint.
    Aucun,
    /// 1 : appoint intégré au ballon.
    Integre(AppointBallon),
    /// 3 : appoint séparé instantané, en série du ballon : l'eau préchauffée est puisée
    /// (éq. 1800 à 1807) et le complément est fourni par les générateurs instantanés d'ECS de la
    /// génération.
    Separe,
}

/// Th-BCE 2020 §9.14.3.1, tableau 291 — assemblage d'un ballon de stockage d'ECS et de ses
/// générateurs : « base seule » sans `appoint`, « base et appoint intégré » avec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StockageSaisi {
    /// Identifiant de l'assemblage, unique dans la génération.
    pub id: Id,
    /// « Nbassemblage » : nombre d'assemblages identiques (au moins 1) ; le générateur de base
    /// désigné représente alors autant de générateurs identiques (`rdim`).
    pub nb_assemblages: u32,
    /// Ballon de stockage (§9.9).
    pub ballon: BallonSaisi,
    /// Générateur de base, dont l'échangeur ou la résistance est en partie basse du ballon
    /// (« zbase = 1 », éq. 1947).
    pub base: GenerateurBallon,
    /// Générateur d'appoint intégré au ballon (« Type_prod_stockage = 1 ») ; absent : base
    /// seule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appoint: Option<AppointBallon>,
}

/// Th-BCE 2020 §9.9.2, tableau 281 — ballon de stockage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BallonSaisi {
    /// « Vtot » : volume du ballon, en L.
    pub v_tot_l: f64,
    /// « (UA)s » et « Statut_UA » : coefficient de pertes thermiques du ballon.
    pub pertes: PertesBallon,
    /// « θmax » : température maximale du ballon, en °C (de 0 à 100).
    pub theta_max_c: f64,
}

/// « (UA)s » et « Statut_UA » (§9.9.3.1, éq. 1766 à 1768).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PertesBallon {
    /// Valeur certifiée, employée telle quelle (éq. 1766).
    Certifie {
        /// « UAs », en W/K.
        ua_s_w_k: f64,
    },
    /// Valeur justifiée par un essai, augmentée de 10 % (éq. 1767).
    Justifie {
        /// « UAs », en W/K.
        ua_s_w_k: f64,
    },
    /// Valeur par défaut selon la nature du ballon (tableau 282 et éq. 1768), signalée dans les
    /// résultats.
    Defaut {
        /// Nature du ballon.
        nature: NatureBallonSaisie,
    },
}

/// Nature d'un ballon pour ses pertes par défaut (tableau 282).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NatureBallonSaisie {
    /// Ballon d'ECS à effet Joule horizontal.
    EffetJouleHorizontal,
    /// Ballon d'ECS à effet Joule vertical.
    EffetJouleVertical,
    /// Autres ballons.
    Autre,
}

/// « typegest » (§9.10.3.2.1) : programmation du thermostat d'un générateur de ballon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProgrammationBallon {
    /// 0 : fonctionnement permanent.
    Permanent,
    /// 1 : de nuit seulement (heure légale inférieure à 5 h, éq. 1811).
    Nuit,
    /// 2 : de jour seulement (heure légale entre 10 h et 17 h, éq. 1812).
    Jour,
}

/// Générateur de base d'un ballon et sa régulation (§9.10.2 et §9.14.3.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateurBallon {
    /// Identifiant d'un générateur de la génération assurant l'ECS : générateur à effet Joule
    /// (résistance) ou générateur thermodynamique pourvu d'un mode `ecs`.
    pub generateur: Id,
    /// « typegest » : programmation du thermostat.
    pub programmation: ProgrammationBallon,
    /// « zreg » : numéro de la zone du ballon, de 1 (bas) à 4 (haut), qui porte la sonde de
    /// régulation.
    pub z_reg: u8,
    /// « Δθ » : hystérésis du thermostat, en K. Absente : 2 K, valeur par défaut du
    /// §9.10.3.2.2, signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_theta_k: Option<f64>,
    /// « hrelech » : hauteur relative de l'échangeur à partir du bas de sa zone, de 0 à 1.
    /// Requise pour un générateur thermodynamique (elle fixe la température vue par son
    /// condenseur, éq. 1781 à 1785) ; sans objet pour une résistance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h_rel_ech: Option<f64>,
}

/// Générateur d'appoint intégré à un ballon (§9.14.3.1, « Type_prod_stockage = 1 »).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AppointBallon {
    /// Identifiant d'un générateur à effet Joule de la génération assurant l'ECS.
    pub generateur: Id,
    /// « typegest_ap ».
    pub programmation: ProgrammationBallon,
    /// « zap » : numéro de la zone la plus basse chauffée par l'appoint, de 1 à 4 (3 pour un
    /// appoint à mi-hauteur).
    pub z_ap: u8,
    /// « zreg_ap » : numéro de la zone de la sonde de régulation de l'appoint.
    pub z_reg: u8,
    /// « faux » : fraction du volume du ballon chauffée par l'appoint, strictement entre 0 et
    /// 1. Absente : 0,5, valeur par défaut du §9.9.3.1, signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub f_aux: Option<f64>,
    /// « Δθap » : hystérésis, en K. Absente : 2 K.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_theta_k: Option<f64>,
}

/// Th-BCE 2020 §9.22 — chauffe-eau thermodynamique à compression électrique décrit par son
/// essai selon la norme NF EN 16147 (étapes C à F en fonctionnement thermodynamique pur).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChauffeEauThermodynamique {
    /// Identifiant, unique dans la génération.
    pub id: Id,
    /// « Nbassemblage » : nombre de chauffe-eau identiques (au moins 1).
    pub nb_assemblages: u32,
    /// « VtotIdCET » : volume du ballon, en L.
    pub v_tot_l: f64,
    /// « θmax » : température maximale du ballon, en °C.
    pub theta_max_c: f64,
    /// Résultats de l'essai.
    pub essai: EssaiEn16147,
    /// Source amont de la pompe à chaleur (« Typesource_chaleur », éq. 2256 à 2258) : air
    /// extérieur, air ambiant d'un espace tampon ou air extrait.
    pub source_amont: SourceAmontSaisie,
    /// « typegest_base » : programmation du thermostat.
    pub programmation: ProgrammationBallon,
    /// « Lim_Theta » de la pompe à chaleur en ECS. Absent : pas de limite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limites: Option<LimitesPac>,
}

/// Résultats d'un essai selon la norme NF EN 16147 (§9.22.2, tableau 304).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EssaiEn16147 {
    /// « Idcycle » : cycle de soutirage.
    pub cycle: CycleSoutirage,
    /// « th » : durée de mise en température, en h (au plus 48).
    pub t_h_h: f64,
    /// « Pes » : puissance absorbée en régime stabilisé, en W.
    pub p_es_w: f64,
    /// « COPDHW » : coefficient de performance sur le cycle de soutirage.
    pub cop_dhw: f64,
    /// « θ'WH » : température d'eau chaude de référence, en °C.
    pub theta_wh_c: f64,
    /// « Isaux » : vrai si les auxiliaires de la pompe à chaleur sont compris dans les
    /// résultats de l'essai ; sinon `Taux = 0,02` s'applique (éq. 2250 et 2251).
    pub auxiliaires_compris: bool,
    /// Statut des résultats : certifiés ou justifiés (éq. 2252 à 2255).
    pub statut: StatutMesure,
}

/// « Idcycle » (§9.22.3.2.3, tableau 305).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CycleSoutirage {
    /// Cycle S.
    S,
    /// Cycle M.
    M,
    /// Cycle L.
    L,
    /// Cycle XL.
    Xl,
    /// Cycle XXL.
    Xxl,
}

/// « idpos » (§8.17.2) : emplacement d'une génération.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PositionGeneration {
    /// 1 : en volume chauffé.
    EnVolumeChauffe,
    /// 0 : hors volume chauffé. `espace_tampon` désigne l'espace tampon qui abrite la
    /// génération, dont le coefficient `btherm(h)` entre dans la température d'ambiance
    /// (§8.17.3.5.8) ; absent, la génération est à l'extérieur (`btherm = 1`).
    HorsVolumeChauffe {
        /// Identifiant de l'espace tampon qui abrite la génération.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        espace_tampon: Option<Id>,
    },
}

/// « idtype_priorite » (§8.17.1) : mode de gestion des priorités entre générateurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypePriorite {
    /// 1 : sans priorité ; la demande est répartie au prorata des puissances nominales.
    SansPriorite,
    /// 2 : en cascade ; le générateur prioritaire est sollicité jusqu'à sa puissance maximale,
    /// le reste passe au suivant (pompe à chaleur et appoint, par exemple).
    Cascade,
    /// 3 : en alternance (§8.17.3.7.2.3) : non pris en charge dans cette version.
    Alternance,
}

/// « idgestion » et « θwm » (§8.17.3.5.10.1) : température de fonctionnement d'une génération
/// reliée à des réseaux hydrauliques.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GestionTemperatureGeneration {
    /// 1 : fonctionnement à température moyenne constante.
    Constante {
        /// « θwm » : température moyenne de fonctionnement, en °C (de 0 à 100).
        theta_wm_c: f64,
    },
    /// 2 : fonctionnement à la température moyenne des réseaux de distribution.
    Reseaux,
}

/// Générateur d'une génération (§8.17.2, tableau 111).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateurSaisi {
    /// Identifiant du générateur, unique dans la génération.
    pub id: Id,
    /// « Rdim » (« Ratdim ») : nombre de générateurs identiques (au moins 1).
    pub rdim: u32,
    /// « idpriorite_ch » : rang du générateur en chauffage dans une gestion en cascade (1 : le
    /// premier sollicité). Requis en cascade si plusieurs générateurs chauffent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priorite_chauffage: Option<u32>,
    /// « idpriorite_fr » : de même en refroidissement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priorite_refroidissement: Option<u32>,
    /// « idpriorite_ecs » : de même en ECS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priorite_ecs: Option<u32>,
    /// « idtype » : nature du générateur et ses caractéristiques.
    pub nature: NatureGenerateur,
}

/// « idtype » (§8.17.3.2, tableau 111) : nature d'un générateur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum NatureGenerateur {
    /// Type 500 : générateur direct à effet Joule (convecteurs, panneaux rayonnants, résistances
    /// d'appoint… ; §8.18).
    EffetJoule(EffetJouleSaisi),
    /// Type 503 : générateur thermodynamique à compression électrique (pompe à chaleur,
    /// climatiseur, refroidisseur ; §8.23).
    ThermodynamiqueElectrique(Box<ThermodynamiqueElectriqueSaisi>),
    /// Types 100 à 102, 200, 201 et 400 : chaudière au gaz, au fioul ou au bois (§8.19).
    Chaudiere(Box<ChaudiereSaisie>),
    /// Types 403 et 404 : poêle ou insert à bois (§8.22).
    PoeleOuInsert(PoeleSaisi),
    /// Types 600 et 601 : sous-station d'un réseau de chaleur ou de froid (§8.28).
    ReseauDeChaleurOuDeFroid(ReseauChaleurFroidSaisi),
    /// Types 103 à 109 : radiateurs, chauffe-eau et accumulateurs au gaz, générateurs d'air
    /// chaud, tubes et panneaux radiants (§8.20) : hors de la première version.
    AutreGenerateurACombustion,
    /// Type 504 et pompes à chaleur à moteur gaz (§8.24 et §8.25) : hors de la première
    /// version.
    ThermodynamiqueGaz,
    /// Types 507 à 509 : pompe à chaleur sur boucle d'eau, thermo-frigo-pompe, générateur DRV
    /// (§8.15 et §8.16) : hors de la première version.
    BoucleDEauOuDrv,
    /// Type 700 : système de cogénération (§8.21) : hors de la première version.
    Cogeneration,
    /// Pompe à chaleur hybride (association d'une pompe à chaleur et d'une chaudière dans un
    /// même produit) : hors de la première version.
    PacHybride,
}

/// Déclinaison d'une chaudière (§8.19.3.1, tableau 115).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeChaudiereSaisi {
    /// Type 100 : chaudière standard au gaz.
    GazStandard,
    /// Type 101 : chaudière basse température au gaz (traitée comme une chaudière standard,
    /// le tableau 115 n'a pas de ligne propre : Q49).
    GazBasseTemperature,
    /// Type 102 : chaudière à condensation au gaz.
    GazCondensation,
    /// Type 200 : chaudière standard au fioul.
    FioulStandard,
    /// Type 201 : chaudière à condensation au fioul.
    FioulCondensation,
    /// Type 400 : chaudière au bois (`combustible_bois` requis).
    Bois,
}

/// « TypeCombustibleBois » (§8.19.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CombustibleBoisSaisi {
    /// 1 : bûches.
    Buches,
    /// 2 : granulés.
    Granules,
    /// 3 : plaquettes.
    Plaquettes,
}

/// « Alim_Chaudière_Bois » (§8.19.2, tableau 119).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AlimentationBoisSaisie {
    /// 0 : chargement manuel, tirage naturel.
    ManuelTirageNaturel,
    /// 1 : chargement manuel, air pulsé.
    ManuelAirPulse,
    /// 2 : chargement automatique, tirage naturel.
    AutomatiqueTirageNaturel,
    /// 3 : chargement automatique, air pulsé.
    AutomatiqueAirPulse,
}

/// Fonction d'une chaudière ou d'un réseau de chaleur (« idfougen » 1, 3 ou 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FonctionChaleur {
    /// 1 : chauffage.
    Chauffage,
    /// 3 : ECS.
    Ecs,
    /// 4 : chauffage et ECS.
    ChauffageEcs,
}

/// Statut des rendements d'une chaudière (§8.19.3.1) : une valeur certifiée est employée telle
/// quelle, une valeur justifiée est réduite de 10 %, une valeur déclarée de 20 %.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatutRendementChaudiere {
    /// Certifié (NF EN ISO/CEI 17065).
    Certifie,
    /// Justifié par un essai (NF EN ISO/CEI 17025) : réduction de 10 %.
    Justifie,
    /// Déclaré par le fabricant : réduction de 20 %.
    Declare,
}

/// Rendements d'une chaudière sur PCS : `R_pn`, `R_pint` et `Pint`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RendementsChaudiere {
    /// Statut des deux rendements.
    pub statut: StatutRendementChaudiere,
    /// « R_pn » : rendement sur PCS à la puissance nominale en régime haute température, en %.
    pub r_pn_pct: f64,
    /// « R_pint » : rendement sur PCS à 30 % de charge en régime basse température, en %.
    pub r_pint_pct: f64,
    /// « Pint » : puissance thermique utile à 30 % de charge, en kW. Absent : 30 % de `Pn_gen`
    /// (valeur par défaut signalée).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_int_kw: Option<f64>,
}

/// Auxiliaires d'une chaudière, en W ; chaque valeur absente prend sa valeur par défaut
/// (éq. 1075 à 1077), signalée dans les résultats.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuxiliairesChaudiere {
    /// « Waux_nom » : puissance électrique absorbée à pleine charge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w_aux_nom_w: Option<f64>,
    /// « Waux_int » : puissance électrique absorbée à charge intermédiaire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w_aux_int_w: Option<f64>,
    /// « Wveille » : puissance électrique absorbée à charge nulle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w_veille_w: Option<f64>,
}

/// « idpertes_parois » (§8.19.3.5, tableau 121).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PertesParoisChaudiere {
    /// 1 : absence de ventilateur ou autre dispositif de circulation d'air ou de fumées
    /// (`p_Qp.g_arret` = 0,50).
    SansVentilateur,
    /// 2 : présence d'un ventilateur (0,75).
    AvecVentilateur,
    /// 3 : présence d'un clapet sur le conduit de fumées (1).
    AvecClapet,
}

/// Th-BCE 2020 §8.19.2, tableau 114 — chaudière au gaz, au fioul ou au bois. La position de la
/// génération (`position`) fixe « idpos_gen ».
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChaudiereSaisie {
    /// Déclinaison de la chaudière.
    pub type_chaudiere: TypeChaudiereSaisi,
    /// « TypeCombustibleBois » : requis pour une chaudière au bois.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combustible_bois: Option<CombustibleBoisSaisi>,
    /// Gaz de pétrole liquéfié (PCSI de 1,09 au lieu de 1,11, tableau 116) ; gaz naturel
    /// sinon. Le GPL relève des « autres énergies » de l'article 9 et du facteur d'émission
    /// « gaz butane / propane » de l'article 10 (lot 7).
    #[serde(default)]
    pub gpl: bool,
    /// « Id_fou_gen » : fonction du générateur.
    pub fonction: FonctionChaleur,
    /// « Pn_gen » : puissance thermique utile nominale en régime haute température, en kW.
    pub pn_gen_kw: f64,
    /// Rendements mesurés ; absents : valeurs par défaut (éq. 1072 et 1073).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendements: Option<RendementsChaudiere>,
    /// « Alim_Chaudière_Bois » : requis pour une chaudière au bois dont les auxiliaires sont
    /// par défaut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alimentation_bois: Option<AlimentationBoisSaisie>,
    /// Auxiliaires.
    #[serde(default)]
    pub auxiliaires: AuxiliairesChaudiere,
    /// « Q_po_30 » : pertes thermiques en régime stabilisé pour un écart de 30 K, en W ; absent :
    /// valeur par défaut (éq. 1074).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_po_30_w: Option<f64>,
    /// « idpertes_parois ».
    pub pertes_parois: PertesParoisChaudiere,
}

/// Th-BCE 2020 §8.22.2, tableau 141 — poêle ou insert à bois (chauffage seul).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PoeleSaisi {
    /// « Pngen » : puissance nominale d'un appareil, en kW (de 0 exclu à 100).
    pub pn_gen_kw: f64,
    /// « ηH,sys,n » : rendement moyen selon la norme de l'appareil, en %.
    pub eta_pct: f64,
    /// « Paux,vent » : puissance du ventilateur, en W (0 sans ventilateur).
    pub p_aux_vent_w: f64,
    /// « Type_Combustible_Bois » (§13.2.2) : combustible, qui fixe le facteur d'émission du bois
    /// dans l'Ic énergie (article 10 de l'arrêté). Obligatoire, comme pour une chaudière au bois.
    pub combustible_bois: CombustibleBoisSaisi,
}

/// Type d'un réseau de chaleur (§8.28.3.1, tableaux 247 et 248).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeReseauChaleurSaisi {
    /// Eau chaude basse température (primaire à 105 °C).
    EauChaudeBasseTemperature,
    /// Eau chaude haute température (150 °C).
    EauChaudeHauteTemperature,
    /// Vapeur basse pression (110 °C).
    VapeurBassePression,
    /// Vapeur haute pression (180 °C).
    VapeurHautePression,
}

/// Th-BCE 2020 §8.28.2, tableau 246 — sous-station d'un réseau de chaleur (type 600) ou de
/// froid (type 601).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ReseauChaleurFroidSaisi {
    /// Réseau de chaleur.
    Chaleur {
        /// Identifiant du réseau dans l'annexe 7 de l'arrêté du 15 septembre 2006 (quatre
        /// chiffres suivis de « C »), qui donne le facteur d'émission et le taux d'énergie
        /// renouvelable et de récupération (lot 7). Absent : lignes « autres réseaux de
        /// chaleur » de l'annexe.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        identifiant_reseau: Option<String>,
        /// « idfougen » : chauffage, ECS ou les deux.
        fonction: FonctionChaleur,
        /// Type de réseau.
        type_reseau: TypeReseauChaleurSaisi,
        /// Classe d'isolation (EN ISO 12241) du réseau secondaire, de 1 à 4 ; le primaire est
        /// isolé d'une classe de plus (§8.28.3.1).
        classe_isolation_secondaire: u8,
        /// « PEss » : puissance d'échange d'une sous-station dans les conditions nominales, en
        /// kW.
        p_ess_kw: f64,
    },
    /// Réseau de froid (refroidissement seul, pertes de sous-station nulles : éq. 1483).
    Froid {
        /// Identifiant du réseau dans l'annexe 7 (quatre chiffres suivis de « F »).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        identifiant_reseau: Option<String>,
        /// « PEss » : puissance d'échange d'une sous-station, en kW.
        p_ess_kw: f64,
    },
}

/// Th-BCE 2020 §8.18.2, tableau 112 — générateur direct à effet Joule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EffetJouleSaisi {
    /// « idfougen » : fonction du générateur, chauffage ou ECS ; jamais les deux (§8.18.3.1).
    pub fonction: FonctionEffetJoule,
    /// « Pngen » : puissance nominale d'un générateur, en W.
    pub pn_gen_w: f64,
}

/// Th-BCE 2020 §8.23.2, tableau 143 — générateur thermodynamique à compression électrique.
///
/// Les modes décrits fixent la catégorie du générateur (§8.23.3.2) : chauffage seul ou
/// refroidissement seul (non réversible), chauffage et refroidissement (réversible).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ThermodynamiqueElectriqueSaisi {
    /// « Fonc_compr » : régulation de puissance du compresseur (§8.23.3.7.4).
    pub compresseur: FonctionnementCompresseurSaisi,
    /// « Taux » et « Statut_taux » : part de la puissance des auxiliaires à charge nulle dans
    /// la puissance absorbée au point pivot (éq. 1277). Absent : valeur par défaut, 0,02 pour
    /// un générateur qui chauffe, 0,01 pour un refroidisseur seul (éq. 1278 et 1279), signalée
    /// dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taux_auxiliaires: Option<TauxAuxiliaires>,
    /// Source amont (§8.26), cohérente avec la technologie de chaque mode.
    pub source_amont: SourceAmontSaisie,
    /// Mode chauffage ; absent : le générateur ne chauffe pas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chauffage: Option<ModeChauffagePac>,
    /// Mode refroidissement ; absent : le générateur ne refroidit pas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refroidissement: Option<ModeRefroidissementPac>,
    /// Mode ECS ; absent : le générateur ne produit pas d'ECS. Un générateur thermodynamique
    /// d'ECS est le générateur de base d'un assemblage avec stockage (§8.23.3.2) ; avec le mode
    /// `chauffage`, il est double service.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ecs: Option<ModeEcsPac>,
}

/// Mode ECS d'un générateur thermodynamique (§8.23.3.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModeEcsPac {
    /// « Sys_thermo_Ecs » : technologie en ECS (tableau 147).
    pub technologie: TechnologieEcsPac,
    /// Performances à pleine charge.
    pub performances: PerformancesPac,
    /// « Lim_Theta » : température aval maximale et température amont minimale. Absent : pas
    /// de limite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limites: Option<LimitesPac>,
}

/// « Sys_thermo_Ecs » (§8.23.3.2, tableau 147). Températures aval de toutes les technologies :
/// 5, 15, 25, 35, 45, 55 et 65 °C, valeur pivot à 45 °C.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TechnologieEcsPac {
    /// 1 : PAC air extérieur / eau. Températures amont : −7, 2, 7, 20 et 35 °C ; pivot à 7 °C.
    AirExterieurEau,
    /// 2 : PAC air extrait / eau. Températures amont : 5, 10, 15, 20, 25 et 30 °C ; pivot à
    /// 20 °C.
    AirExtraitEau,
    /// 3 : PAC air ambiant / eau. Températures amont : 5, 10, 15, 20, 25 et 30 °C ; pivot à
    /// 15 °C.
    AirAmbiantEau,
    /// 4 : PAC eau de nappe / eau : non prise en charge dans cette version.
    EauDeNappeEau,
    /// 5 : PAC sol / eau : non prise en charge dans cette version.
    SolEau,
    /// 6 : PAC eau glycolée / eau : non prise en charge dans cette version.
    EauGlycoleeEau,
}

/// « Fonc_compr » (§8.23.3.7.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FonctionnementCompresseurSaisi {
    /// 1 : puissance variable (compresseur à vitesse variable), puis cycles marche-arrêt sous
    /// le taux de charge minimal.
    PuissanceVariable,
    /// 2 : tout ou rien, cycles marche-arrêt.
    ToutOuRien,
}

/// Statut d'une valeur mesurée (§8.23.3.1) : certifiée par un organisme accrédité selon la
/// norme NF EN ISO/CEI 17065, ou justifiée par un essai d'un laboratoire accrédité selon la
/// norme NF EN ISO/CEI 17025.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatutMesure {
    /// Valeur certifiée : employée telle quelle.
    Certifie,
    /// Valeur justifiée : pénalisée selon la règle de la grandeur.
    Justifie,
}

/// « Taux » et « Statut_taux » (§8.23.3.6, éq. 1277).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TauxAuxiliaires {
    /// « Taux », de 0 à 1.
    pub valeur: f64,
    /// Statut : une valeur justifiée est multipliée par 1,1.
    pub statut: StatutMesure,
}

/// « idamont-air-type » (§8.26.3.3.1) : source amont sur air.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceAmontSaisie {
    /// 1 : air extérieur (éq. 1405).
    AirExterieur {
        /// « Pvent_gainé » : pour une machine gainée, puissance des ventilateurs nécessaire
        /// pour vaincre les pertes de charge des conduits, en W (éq. 1443). Absent : machine
        /// non gainée, auxiliaires compris dans les performances (éq. 1442).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        p_vent_gaine_w: Option<f64>,
    },
    /// 2 : air ambiant d'un volume non chauffé (éq. 1406), en production d'ECS seulement.
    AirAmbiant {
        /// Espace tampon dans lequel la machine puise sa chaleur.
        espace_tampon: Id,
        /// « Pvent_gainé », comme pour l'air extérieur.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        p_vent_gaine_w: Option<f64>,
    },
    /// 3 : air extrait par le système de ventilation d'une zone (éq. 1407).
    AirExtrait {
        /// Zone dont le système de ventilation irrigue la source.
        zone: Id,
        /// « Tair_lim » : température limite de l'air rejeté, en °C (éq. 1454).
        t_air_lim_c: f64,
        /// « Pvent_gainé », comme pour l'air extérieur.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        p_vent_gaine_w: Option<f64>,
    },
}

/// Mode chauffage d'un générateur thermodynamique (§8.23.3.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModeChauffagePac {
    /// « Sys_thermo_Ch » : technologie en chauffage (tableau 145).
    pub technologie: TechnologieChauffagePac,
    /// Performances à pleine charge.
    pub performances: PerformancesPac,
    /// « Lim_Theta » : limites de fonctionnement — température aval maximale et température
    /// amont minimale. Absent : pas de limite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limites: Option<LimitesPac>,
    /// « Typo_emetteur_ch » : inertie du système d'émission principal (tableau 216).
    pub typo_emetteur: TypoEmetteur,
    /// « LRcontmin » et « CcpLRcontmin » d'un compresseur à puissance variable. Absent : 0,4 et
    /// 1 (éq. 1280 et 1281), signalés dans les résultats. Sans objet en tout ou rien.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fonctionnement_continu: Option<FonctionnementContinu>,
}

/// Mode refroidissement d'un générateur thermodynamique (§8.23.3.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModeRefroidissementPac {
    /// « Sys_thermo_Fr » : technologie en refroidissement (tableau 145).
    pub technologie: TechnologieRefroidissementPac,
    /// Performances à pleine charge ; `performance` y désigne l'EER.
    pub performances: PerformancesPac,
    /// « Lim_Theta » : limites de fonctionnement — température aval minimale et température
    /// amont maximale. Absent : pas de limite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limites: Option<LimitesPac>,
    /// « Typo_emetteur_fr » : inertie du système d'émission principal (tableau 216).
    pub typo_emetteur: TypoEmetteur,
    /// « LRcontmin » et « CcpLRcontmin » d'un compresseur à puissance variable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fonctionnement_continu: Option<FonctionnementContinu>,
}

/// « Sys_thermo_Ch » (§8.23.3.2, tableau 145).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TechnologieChauffagePac {
    /// 1 : PAC air extérieur / eau. Températures aval (moyenne du départ et du retour) : 23,5,
    /// 32,5, 42,5, 51 et 60 °C ; amont : −15, −7, 2, 7 et 20 °C ; pivot : 7 °C et 32,5 °C.
    AirExterieurEau,
    /// 2 : PAC air extérieur / air recyclé. Températures aval (air intérieur) : 5, 10, 15, 20
    /// et 25 °C ; amont : −15, −7, 2, 7 et 20 °C ; pivot : 7 °C et 20 °C.
    AirExterieurAirRecycle,
    /// 3 : PAC air extrait / air neuf : non prise en charge dans cette version.
    AirExtraitAirNeuf,
    /// 4 : PAC eau de nappe / eau : non prise en charge dans cette version.
    EauDeNappeEau,
    /// 5 : PAC eau glycolée / eau : non prise en charge dans cette version.
    EauGlycoleeEau,
    /// 6 : PAC eau de nappe / air recyclé : non prise en charge dans cette version.
    EauDeNappeAirRecycle,
    /// 7 : PAC eau de boucle / air recyclé : non prise en charge dans cette version.
    EauDeBoucleAirRecycle,
    /// 8 : PAC sol / eau : non prise en charge dans cette version.
    SolEau,
    /// 9 : PAC sol / sol : non prise en charge dans cette version.
    SolSol,
}

/// « Sys_thermo_Fr » (§8.23.3.2, tableau 145).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TechnologieRefroidissementPac {
    /// 1 : refroidisseur air extérieur / eau. Températures aval (moyenne du départ et du
    /// retour) : 4, 9,5, 15, 20,5 et 26 °C ; amont : 5, 15, 25, 35 et 45 °C ; pivot : 35 °C et
    /// 9,5 °C.
    AirExterieurEau,
    /// 2 : refroidisseur air extérieur / air recyclé. Températures aval (air intérieur) : 22,
    /// 27, 32 et 37 °C ; amont : 5, 15, 25, 35 et 45 °C ; pivot : 35 °C et 27 °C.
    AirExterieurAirRecycle,
    /// 3 : refroidisseur air extrait / air neuf : non pris en charge dans cette version.
    AirExtraitAirNeuf,
    /// 4 : refroidisseur eau / eau et eau glycolée / eau : non pris en charge dans cette
    /// version.
    EauEau,
    /// 5 : refroidisseur eau / air et eau de boucle / air recyclé : non pris en charge dans
    /// cette version.
    EauAir,
    /// 6 : refroidisseur eau de nappe / air recyclé : non pris en charge dans cette version.
    EauDeNappeAirRecycle,
    /// 7 : refroidisseur eau de nappe / eau : non pris en charge dans cette version.
    EauDeNappeEau,
}

/// Performances à pleine charge d'un mode (§8.23.3.1) : « Statut_données_PC » et
/// « Statut_val_pivot ».
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PerformancesPac {
    /// Valeurs certifiées ou justifiées (« Statut_données_PC = 1 »), dont la valeur pivot : une
    /// valeur pour chaque couple du croisement de températures aval et de températures amont
    /// prises dans l'ordre de priorité de la technologie (« M_θ_Aval » et « M_θ_Amont »). Les
    /// autres cases de la matrice sont complétées par les coefficients de la méthode.
    Mesurees(Vec<PointPerformance>),
    /// Valeur pivot déclarée par le fabricant (« Statut_val_pivot = 1 ») : réduite de 20 % et
    /// plafonnée à la valeur utile maximale de la technologie.
    PivotDeclare {
        /// « Val_COP » ou « Val_EER » : COP ou EER déclaré au point pivot.
        performance: f64,
        /// « Val_Pabs » : puissance absorbée au point pivot, en kW.
        pabs_kw: f64,
    },
    /// Valeur pivot par défaut (« Statut_val_pivot = 2 ») : la valeur utile maximale de la
    /// technologie réduite de 20 %, signalée dans les résultats.
    PivotDefaut {
        /// « Val_Pabs » : puissance absorbée au point pivot, en kW.
        pabs_kw: f64,
    },
}

/// Valeur mesurée d'une matrice de performances (§8.23.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PointPerformance {
    /// Température aval de la mesure, en °C : une des températures de la technologie.
    pub theta_aval_c: f64,
    /// Température amont de la mesure, en °C : une des températures de la technologie.
    pub theta_amont_c: f64,
    /// COP (chauffage, ECS) ou EER (refroidissement) mesuré à pleine charge.
    pub performance: f64,
    /// Puissance absorbée mesurée à pleine charge, en kW.
    pub pabs_kw: f64,
    /// Statut : un COP ou un EER justifié est réduit de 10 % (éq. 1238).
    pub statut: StatutMesure,
}

/// « Lim_Theta » (§8.23.3.7.3) : limites de fonctionnement d'un mode.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LimitesPac {
    /// Combinaison des deux limites.
    pub combinaison: CombinaisonLimites,
    /// Limite sur la température aval, en °C : maximale en chauffage et en ECS
    /// (« θmax_av »), minimale en refroidissement (« θmin_av »).
    pub theta_aval_c: f64,
    /// Limite sur la température amont, en °C : minimale en chauffage et en ECS
    /// (« θmin_am »), maximale en refroidissement (« θmax_am »).
    pub theta_amont_c: f64,
}

/// « Lim_Theta » : 1 ou 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CombinaisonLimites {
    /// 1 : la machine s'arrête dès que l'une des deux limites est franchie.
    LUneOuLAutre,
    /// 2 : la machine s'arrête si les deux limites sont franchies à la fois.
    LUneEtLAutre,
}

/// « Typo_emetteur » (§8.23.3.6, tableau 216) : inertie du système d'émission, qui fixe la
/// durée de fonctionnement à charge tendant vers zéro « Dfou0 ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypoEmetteur {
    /// 1 : plancher et plafond chauffants ou rafraîchissants intégrés au bâti, d'inertie forte
    /// (32 minutes).
    PlancherOuPlafondInertieForte,
    /// 2 : radiateurs ; plafonds chauffants ou rafraîchissants d'inertie moyenne (19 minutes).
    RadiateursOuPlafondInertieMoyenne,
    /// 3 : ventilo-convecteurs ; plafonds d'inertie faible ; valeur par défaut pour la
    /// distribution à eau (6 minutes).
    VentiloConvecteursOuInertieFaible,
    /// 4 : systèmes à air (2 minutes).
    SystemesAAir,
}

/// « LRcontmin », « CcpLRcontmin » et « Statut_Fonctionnement_Continu » (§8.23.3.6).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FonctionnementContinu {
    /// « LRcontmin » : taux de charge minimal en fonctionnement continu, de 0 à 1.
    pub lr_contmin: f64,
    /// « CcpLRcontmin » : rapport du COP (ou de l'EER) au taux de charge `LRcontmin` sur le COP
    /// à pleine charge.
    pub ccp_lr_contmin: f64,
    /// Statut : pour des valeurs justifiées, `LRcontmin` est augmenté de 0,05 et
    /// `CcpLRcontmin` multiplié par 0,9.
    pub statut: StatutMesure,
}

/// « idfougen » d'un générateur direct à effet Joule (§8.18.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FonctionEffetJoule {
    /// 1 : chauffage.
    Chauffage,
    /// 3 : production instantanée d'ECS (§9.1).
    Ecs,
}

/// « idraccord_reseau » (§8.4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RaccordementReseaux {
    /// 0 : avec possibilité d'isolement.
    AvecIsolement,
    /// 1 : permanent.
    Permanent,
}

/// Fonction d'un réseau de distribution (« idfonction », §8.7.2, §8.9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FonctionReseau {
    /// 1 : chauffage.
    Chauffage,
    /// 2 : refroidissement.
    Refroidissement,
}

/// Th-BCE 2020 §8.9 et §8.10 — réseau de distribution intergroupes de chaud ou de froid,
/// relié à une et une seule génération (§8.10.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DistributionIntergroupes {
    /// Identifiant du réseau, unique dans le bâtiment.
    pub id: Id,
    /// « idfonction » : chauffage ou refroidissement. Un réseau « 2 tubes réversible » se
    /// décrit par deux réseaux, l'un de chauffage, l'autre de refroidissement (§8.9.1).
    pub fonction: FonctionReseau,
    /// Identifiant de la génération qui alimente le réseau.
    pub generation: Id,
    /// Nature du réseau (« idtype »).
    pub reseau: ReseauIntergroupes,
}

/// « idtype » d'un réseau intergroupes (§8.9.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ReseauIntergroupes {
    /// 0 : réseau fictif, sans pertes ni auxiliaires, pour les systèmes sans fluide caloporteur
    /// (effet Joule, poêles et inserts, pompes à chaleur à détente directe ; §8.9.3.5.1).
    Fictif,
    /// 1 : réseau hydraulique collectif (§8.9.3.5.2 et §8.10.3.2).
    HydrauliqueCollectif(ReseauHydrauliqueIntergroupes),
    /// 2 : réseaux hydrauliques individuels uniquement : pas de pertes intergroupes
    /// (`Modpertes = 0`, éq. 905 et 914). Une installation individuelle sans branche
    /// intergroupes se décrit aussi par un réseau hydraulique de longueur nulle, sans
    /// circulateur (note du §8.9.3.5).
    HydrauliqueIndividuel(ReseauHydrauliqueIntergroupes),
}

/// Th-BCE 2020 §8.9.2 et §8.10.2 — paramètres d'un réseau intergroupes hydraulique.
///
/// Classement des longueurs en ou hors volume chauffé : fiche d'application 8.13 (un réseau
/// intégré à une paroi est en volume chauffé si la résistance thermique qui le sépare du volume
/// chauffé est inférieure à la moitié de celle de la paroi ; en gaine palière, selon la
/// circulation au sens des règles Th-Bat ; en gaine logement, en volume chauffé).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReseauHydrauliqueIntergroupes {
    /// « Lvc » : longueur en volume chauffé, en m.
    pub l_vc_m: f64,
    /// « Lhvc » : longueur hors volume chauffé, en m.
    pub l_hvc_m: f64,
    /// « Umoy_vc » : coefficient de déperdition linéique moyen en volume chauffé, en W/(m.K).
    pub u_vc_w_mk: f64,
    /// « Umoy_hvc » : coefficient de déperdition linéique moyen hors volume chauffé, en W/(m.K).
    pub u_hvc_w_mk: f64,
    /// « Paux » : puissance du circulateur, en W.
    pub p_aux_w: f64,
    /// « idcirc » : régulation du circulateur.
    pub circulateur: Circulateur,
    /// Espace tampon traversé par la partie hors volume chauffé (« btherm », éq. 924 et 932).
    /// Absent : la partie hors volume chauffé est à l'extérieur (`btherm = 1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
}

/// « idcirc » (§8.7.2, §8.9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Circulateur {
    /// 0 : pas de circulateur.
    Aucun,
    /// 1 : vitesse constante.
    VitesseConstante,
    /// 2 : vitesse variable, pression différentielle constante (réseau à débit variable).
    PressionConstante,
    /// 3 : vitesse variable réduisant la pression différentielle (réseau à débit variable).
    PressionVariable,
}

/// « iddebit » (§8.7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RegulationDebit {
    /// 1 : débit constant, fonctionnement continu.
    DebitConstantContinu,
    /// 2 : débit constant, fonctionnement intermittent.
    DebitConstantIntermittent,
    /// 3 : débit variable.
    DebitVariable,
}

/// « idgest » (§8.7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RegulationTemperature {
    /// 1 : température de départ constante.
    DepartConstant,
    /// 2 : température de retour constante.
    RetourConstant,
    /// 3 : température de départ fonction de la température extérieure (chauffage seul).
    LoiDEau,
}

/// Th-BCE 2020 §8.7.2 et §8.8.2 — paramètres d'un réseau hydraulique du groupe.
///
/// Classement des longueurs en ou hors volume chauffé : fiche d'application 8.13 (voir
/// [`ReseauHydrauliqueIntergroupes`]) ; les planchers chauffants ou rafraîchissants n'en
/// relèvent pas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReseauHydrauliqueGroupe {
    /// « iddebit » : régulation du débit.
    pub regulation_debit: RegulationDebit,
    /// « idgest » : régulation de la température.
    pub regulation_temperature: RegulationTemperature,
    /// « θdep_dim » : température de départ de dimensionnement, en °C ; requise pour une
    /// température de départ constante ou une loi d'eau.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theta_dep_dim_c: Option<f64>,
    /// « θret_dim » : température de retour de dimensionnement, en °C ; requise pour une
    /// température de retour constante.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theta_ret_dim_c: Option<f64>,
    /// « Δθem_dim » : écart nominal de température entre départ et retour, en K (négatif en
    /// refroidissement).
    pub delta_theta_em_dim_k: f64,
    /// « qnom » : débit volumique nominal, en m³/h.
    pub q_nom_m3_h: f64,
    /// « qresid » : débit résiduel minimal d'un réseau à débit variable, en m³/h ; requis pour
    /// un réseau à débit variable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_resid_m3_h: Option<f64>,
    /// « idcirc » : régulation du circulateur.
    pub circulateur: Circulateur,
    /// « Paux » : puissance du circulateur, en W.
    pub p_aux_w: f64,
    /// « Lvc » : longueur en volume chauffé, en m.
    pub l_vc_m: f64,
    /// « Lhvc » : longueur hors volume chauffé, en m.
    pub l_hvc_m: f64,
    /// « Umoyen_vc » : coefficient de déperdition linéique moyen en volume chauffé, en W/(m.K).
    pub u_vc_w_mk: f64,
    /// « Umoyen_hvc » : coefficient de déperdition linéique moyen hors volume chauffé, en
    /// W/(m.K).
    pub u_hvc_w_mk: f64,
    /// Espace tampon traversé par la partie hors volume chauffé (« btherm », éq. 876 et 883).
    /// Absent : la partie hors volume chauffé est à l'extérieur (`btherm = 1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
}

/// Th-BCE 2020 §8.7 et §8.8 — réseau de distribution du groupe, de chaud ou de froid, connecté
/// à un et un seul réseau intergroupes de même fonction et de même type (éq. 850 et 851).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DistributionGroupe {
    /// Identifiant du réseau, unique dans le groupe.
    pub id: Id,
    /// « idfonction » : chauffage ou refroidissement.
    pub fonction: FonctionReseau,
    /// Identifiant du réseau intergroupes du bâtiment auquel le réseau est connecté.
    pub distribution_intergroupe: Id,
    /// Nature du réseau (« idtype »).
    pub reseau: ReseauGroupe,
}

/// « idtype » d'un réseau de distribution du groupe (§8.7.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ReseauGroupe {
    /// 0 : réseau fictif, sans pertes ni auxiliaires (§8.7.3.3.1 et §8.8.3.1) : effet Joule,
    /// générateurs d'air chaud, poêles et inserts, pompes à chaleur à détente directe.
    Fictif,
    /// 1 : réseau hydraulique (§8.7.3.3.2, §8.7.3.3.3, §8.8.3.2 et §8.8.3.3).
    Hydraulique(ReseauHydrauliqueGroupe),
}

/// Th-BCE 2020 §8.5 — programmation des relances d'un groupe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Programmation {
    /// « Typepgrm_ch » : programmation de la relance du chauffage (tableau 92).
    pub chauffage: ProgrammationChauffage,
    /// « Typepgrm_fr » : programmation de la relance du refroidissement (tableau 93). Requise
    /// si le groupe est climatisé (`iclim`), sans objet sinon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refroidissement: Option<ProgrammationRefroidissement>,
}

/// « Typepgrm_ch » (§8.5.3, tableau 92).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProgrammationChauffage {
    /// 1 : horloge à heure fixe.
    Horloge,
    /// 2 : horloge à heure fixe associée à un contrôle d'ambiance.
    HorlogeControleAmbiance,
    /// 3 : optimiseur.
    Optimiseur,
}

/// « Typepgrm_fr » (§8.5.3, tableau 93).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProgrammationRefroidissement {
    /// 1 : horloge à heure fixe.
    Horloge,
    /// 2 : horloge à heure fixe associée à un contrôle d'ambiance.
    HorlogeControleAmbiance,
    /// 3 : pas d'horloge (fonctionnement permanent).
    SansHorloge,
}

/// Th-BCE 2020 §8.1 — émetteur de chaud et/ou de froid d'un groupe (« em »).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Emetteur {
    /// Identifiant de l'émetteur, unique dans le groupe.
    pub id: Id,
    /// Fonction de chauffage (« idem_chaud = 1 ») et ses caractéristiques. Absent : l'émetteur
    /// ne chauffe pas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chauffage: Option<EmissionChaud>,
    /// Fonction de refroidissement (« idem_froid = 1 ») et ses caractéristiques. Absent :
    /// l'émetteur ne refroidit pas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refroidissement: Option<EmissionFroid>,
    /// « Pper » : ratio des pertes au dos sur l'énergie émise (§8.1.3.1.4). Absent : pas de
    /// pertes au dos. Pour un émetteur intégré à une paroi donnant sur l'extérieur, un vide
    /// sanitaire, le sol ou un local non chauffé, le texte le calcule par l'éq. (795),
    /// `Pper = Ri·Ue` : voir `pertes_au_dos`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pertes_au_dos: Option<PertesAuDos>,
    /// « Iddetection_presence » (§8.1.3.1.6, éq. 799) : détection de présence pièce par pièce
    /// (chauffage). Absent : faux.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detection_presence: Option<bool>,
    /// Ventilateurs locaux (émetteurs gainés ou carrossés de type ventilo-convecteur,
    /// §8.1.3.8). Absent : « GestVCV = 0 », pas de ventilateur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ventilateurs: Option<VentilateursLocaux>,
}

/// Th-BCE 2020 §8.1.3.1.4, éq. (795) — pertes au dos d'un émetteur intégré à une paroi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PertesAuDos {
    /// « Ri » : résistance thermique de la paroi entre l'émetteur et l'intérieur, résistance
    /// superficielle intérieure comprise, en m².K/W.
    pub r_i_m2k_w: f64,
    /// « Ue » : coefficient de transmission thermique équivalent de la paroi, en W/(m².K).
    pub u_e_w_m2k: f64,
}

/// Caractéristiques d'un émetteur en chauffage (§8.1.2 et §8.1.3.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmissionChaud {
    /// Typologie de l'émetteur : fixe la part convective `Pemconv_ch` (tableau 85).
    pub type_emetteur: TypeEmetteurChaud,
    /// « δθvs_ch » : variation spatiale en chauffage (tableaux 82 et 83).
    pub variation_spatiale: VariationSpatiale,
    /// « δθvt_ch » : variation temporelle en chauffage (§8.1.3.1.5, tableaux 87 et 88).
    pub variation_temporelle: VariationTemporelle,
    /// Émetteur à effet Joule : une valeur certifiée de variation temporelle ne peut pas être
    /// inférieure à 0,2 K, contre 0,4 K pour les autres émetteurs de chauffage (§8.1.3.1.5).
    pub effet_joule: bool,
    /// « Rats_ch » : part de la surface utile du groupe desservie par l'émetteur.
    pub rats: f64,
    /// « Ratt_ch » : part de l'énergie de chaud fournie à cette surface assurée par l'émetteur.
    pub ratt: f64,
    /// Identifiant du réseau de distribution du groupe (fonction chauffage) qui alimente
    /// l'émetteur (§8.7.1).
    pub distribution: Id,
}

/// Caractéristiques d'un émetteur en refroidissement (§8.1.2 et §8.1.3.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmissionFroid {
    /// Typologie de l'émetteur : fixe la part convective `Pemconv_fr` (tableau 86).
    pub type_emetteur: TypeEmetteurFroid,
    /// « δθvs_fr » : variation spatiale en refroidissement (tableau 84).
    pub variation_spatiale: VariationSpatialeFroid,
    /// « δθvt_fr » : variation temporelle en refroidissement (§8.1.3.1.5, tableau 87), négative.
    pub variation_temporelle: VariationTemporelle,
    /// « Rats_fr » : part de la surface utile du groupe desservie par l'émetteur.
    pub rats: f64,
    /// « Ratt_fr » : part de l'énergie de froid fournie à cette surface assurée par l'émetteur.
    pub ratt: f64,
    /// Identifiant du réseau de distribution du groupe (fonction refroidissement).
    pub distribution: Id,
}

/// Typologie d'émetteur de chaud (§8.1.3.1.3, tableau 85).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeEmetteurChaud {
    /// Soufflage d'air chaud (convecteurs, ventilo-convecteurs, aérothermes…) : 0,95.
    SoufflageAirChaud,
    /// Émetteurs muraux rayonnants (panneaux rayonnants, radiateurs à eau chaude…) : 0,70.
    MurauxRayonnants,
    /// Planchers chauffants, tubes rayonnants gaz basse température, panneaux radiants
    /// lumineux gaz, inserts, poêles à bois (autres qu'à accumulation) : 0,50.
    PlancherChauffant,
    /// Murs chauffants, panneaux rayonnants de plafond, cassettes rayonnantes basse ou moyenne
    /// température, poêles à accumulation : 0,35.
    MurChauffant,
    /// Plafond chauffant, radiant électrique infrarouge moyen ou court : 0,20.
    PlafondChauffant,
}

/// Typologie d'émetteur de froid (§8.1.3.1.3, tableau 86).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeEmetteurFroid {
    /// Soufflage d'air froid (ventilo-convecteurs…) : 0,95.
    SoufflageAirFroid,
    /// Poutre froide : 0,80.
    PoutreFroide,
    /// Plafond rafraîchissant, panneaux rafraîchissants de plafond : 0,50.
    PlafondRafraichissant,
    /// Mur rafraîchissant : 0,35.
    MurRafraichissant,
    /// Plancher rafraîchissant : 0,20.
    PlancherRafraichissant,
}

/// « δθvs_ch » (§8.1.3.1.2) : classe d'émetteur et hauteur sous plafond (tableau 82), cas des
/// poêles et inserts (tableau 83), ou valeur justifiée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum VariationSpatiale {
    /// Classe de l'émetteur (A : plancher chauffant ; B1 : émetteurs à forte induction ; B2 :
    /// diffusion d'air, poutres climatiques, ventilo-convecteurs ; B3 : émetteurs rayonnants
    /// autres que plancher, plafond chauffant ; C : autres cas) et hauteur sous plafond du local
    /// desservi, en m.
    Classe {
        /// Classe de l'émetteur.
        classe: ClasseEmetteurChaud,
        /// Hauteur sous plafond, en m.
        hsp_m: f64,
    },
    /// Poêle ou insert : appareil desservant un ou deux niveaux (tableau 83).
    PoeleOuInsert {
        /// Nombre de niveaux desservis : 1 ou 2.
        niveaux: u8,
    },
    /// Valeur justifiée, en K.
    Valeur {
        /// Variation spatiale, en K.
        delta_theta_k: f64,
    },
}

/// Classe d'un émetteur de chaud (tableau 82).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ClasseEmetteurChaud {
    /// Plancher chauffant.
    A,
    /// Émetteurs à forte induction.
    B1,
    /// Diffusion d'air, poutres climatiques, ventilo-convecteurs.
    B2,
    /// Émetteurs rayonnants (autres que plancher), plafond chauffant.
    B3,
    /// Autres cas.
    C,
}

/// « δθvs_fr » (§8.1.3.1.2, tableau 84).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum VariationSpatialeFroid {
    /// Classe de l'émetteur (A : plancher froid, déplacement d'air ; B : diffusion d'air,
    /// brassage de l'air ambiant, poutres climatiques ; C : plafond rafraîchissant, autres cas)
    /// et hauteur sous plafond, en m.
    Classe {
        /// Classe de l'émetteur.
        classe: ClasseEmetteurFroid,
        /// Hauteur sous plafond, en m.
        hsp_m: f64,
    },
    /// Valeur justifiée, en K.
    Valeur {
        /// Variation spatiale, en K.
        delta_theta_k: f64,
    },
}

/// Classe d'un émetteur de froid (tableau 84).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ClasseEmetteurFroid {
    /// Plancher froid, système à déplacement d'air.
    A,
    /// Diffusion d'air, émetteur avec brassage de l'air ambiant, poutres climatiques.
    B,
    /// Plafond rafraîchissant, autres cas.
    C,
}

/// « δθvt » (§8.1.3.1.5) : variation temporelle du couple régulateur/émetteur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum VariationTemporelle {
    /// Valeur certifiée, en K (négative en froid).
    Certifie {
        /// Variation temporelle, en K.
        delta_theta_k: f64,
    },
    /// Valeur justifiée par un essai d'un laboratoire accrédité, en K (négative en froid) ; le
    /// moteur l'augmente de 0,5 K en chaud (la diminue de 0,5 K en froid).
    Justifie {
        /// Variation temporelle justifiée, en K.
        delta_theta_k: f64,
    },
    /// À défaut de valeur certifiée : valeur du tableau 87, selon que le couple
    /// régulateur/émetteur permet ou non un arrêt total de l'émission.
    Defaut {
        /// Vrai si le couple permet un arrêt total de l'émission.
        arret_total: bool,
    },
    /// Poêles et inserts (tableau 88) : valeur conventionnelle selon la régulation.
    PoeleOuInsert {
        /// Vrai avec thermostat d'ambiance, faux en régulation manuelle.
        thermostat: bool,
    },
}

/// Th-BCE 2020 §8.1.3.8 — ventilateurs locaux d'un émetteur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VentilateursLocaux {
    /// « GestVCV » : mode de gestion des ventilateurs (1 à 3).
    pub gestion: GestionVentilateurs,
    /// « ispv » : régime de super petite vitesse automatique.
    #[serde(default)]
    pub super_petite_vitesse: bool,
    /// « PVCV_GV » : puissance en grande vitesse, en W.
    pub p_gv_w: f64,
    /// « PVCV_MV » : puissance en moyenne vitesse, en W.
    pub p_mv_w: f64,
    /// « PVCV_PV » : puissance en petite vitesse, en W.
    pub p_pv_w: f64,
    /// « PVCV_SPV » : puissance en super petite vitesse, en W. Requise avec
    /// `super_petite_vitesse`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_spv_w: Option<f64>,
    /// « Qv_recirc_GV » : débit de recirculation en grande vitesse, en m³/h.
    pub q_recirc_gv_m3_h: f64,
    /// « Qv_recirc_MV » : débit de recirculation en moyenne vitesse, en m³/h.
    pub q_recirc_mv_m3_h: f64,
    /// « Qv_recirc_PV » : débit de recirculation en petite vitesse, en m³/h.
    pub q_recirc_pv_m3_h: f64,
    /// « idregul_batt » : régulation de la batterie de refroidissement (sans objet pour un
    /// émetteur qui ne refroidit pas).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regulation_batterie: Option<RegulationBatterie>,
}

/// « GestVCV » (§8.1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GestionVentilateurs {
    /// 1 : régulation manuelle uniquement.
    Manuelle,
    /// 2 : régulation automatique nécessitant un fonctionnement permanent.
    AutomatiquePermanente,
    /// 3 : régulation automatique permettant l'arrêt total des ventilateurs.
    AutomatiqueAvecArret,
}

/// « idregul_batt » (§8.1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RegulationBatterie {
    /// 0 : batterie à débit d'eau régulé de façon progressive.
    DebitRegule,
    /// 1 : autre cas (température de batterie constante).
    TemperatureConstante,
}

// ---------------------------------------------------------------------------------------------
// Eau chaude sanitaire : émission et distributions (Th-BCE 2020 §9.5 à 9.8).
// ---------------------------------------------------------------------------------------------

/// Th-BCE 2020 §9.5 et §9.6 — émetteur d'ECS équivalent d'un groupe : émetteurs et appareils
/// sanitaires d'un même groupe reliés à une même génération (§9.5.3.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmetteurEcs {
    /// Identifiant, unique dans le groupe.
    pub id: Id,
    /// « Ratem » : part de la surface utile du groupe desservie ; la somme sur les émetteurs
    /// d'ECS du groupe vaut 1 (éq. 1673).
    pub rat_em: f64,
    /// « Nbmaison » (maison individuelle ou accolée : nombre de maisons desservies, plus de 1
    /// pour des maisons accolées) ou « Nblgt-e » (logement collectif : nombre de logements
    /// desservis), tableau 272. Requis, au moins 1, en maison individuelle ou accolée et en
    /// logement collectif ; sans objet dans les autres usages.
    #[serde(default, skip_serializing_if = "est_zero")]
    pub nb_logements: u32,
    /// « Nu » : nombre d'unités caractéristiques desservies, pour les usages autres que la
    /// maison individuelle ou accolée et le logement collectif (§9.6.3.2.2.1, tableau 277 : en
    /// m² de surface utile pour tous ces usages). Absent : la surface utile que l'émetteur
    /// dessert, `rat_em × su_m2` (éq. 1682). Ne pas saisir en habitation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nu: Option<f64>,
    /// « partmélangeur » : part des besoins passant par des mélangeurs, mitigeurs mécaniques et
    /// autres émetteurs (gain 0 %, tableau 273).
    pub part_melangeur: f64,
    /// « partmitigeur » : part passant par des mitigeurs thermostatiques ou mécaniques économes
    /// (classe C3 ou CH3 ; gain 5 %).
    pub part_mitigeur: f64,
    /// « parttemporisateur » : part passant par des robinets électroniques ou des temporisateurs
    /// (gain 7 %).
    pub part_temporisateur: f64,
    /// « App_ecs » : appareils sanitaires (tableau 275) ; s'il y en a de natures différentes,
    /// le plus défavorable.
    pub appareils: AppareilsEcs,
    /// « Irecup » : récupérateur de chaleur sur les eaux grises (fiche 9.21) : non pris en
    /// charge dans cette version. Absent : faux.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recuperateur_eaux_grises: Option<bool>,
    /// Réseau de distribution d'ECS du groupe, propre à l'émetteur (§9.7).
    pub distribution: DistributionEcsGroupe,
    /// Identifiant du réseau de distribution d'ECS intergroupe du bâtiment (§9.8).
    pub distribution_intergroupe: Id,
}

fn est_zero(n: &u32) -> bool {
    *n == 0
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn est_nul(x: &f64) -> bool {
    *x == 0.0
}

/// « App_ecs » (§9.5.3, tableau 275).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AppareilsEcs {
    /// Douches seules : gain de 5 %.
    DoucheSeule,
    /// Baignoire sabot (125 L au plus) : 2,5 %.
    BaignoireSabot,
    /// Baignoire standard (plus de 125 L et 175 L au plus) et autre : 0 %.
    BaignoireStandard,
    /// Grande baignoire (plus de 175 L) : −2,5 %.
    GrandeBaignoire,
}

/// Th-BCE 2020 §9.7.2 — distribution d'ECS du groupe : réseau sans circulateur ni isolant,
/// supposé à température constante.
///
/// Classement des longueurs en ou hors volume chauffé : fiche d'application 8.13 (en gaine
/// logement, en volume chauffé).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DistributionEcsGroupe {
    /// « θ2nd-e » : température de la distribution, en °C, au moins égale à celle de l'eau
    /// mitigée (40 °C). Absent : 53 °C, valeur conventionnelle du tableau 278 (lecture Q45),
    /// signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theta_2nd_e_c: Option<f64>,
    /// « Dint_2nd-e » : diamètre intérieur, en m.
    pub d_int_m: f64,
    /// « Lvc_2nd-e » : longueur en volume chauffé d'une distribution, en m. Absent : valeur par
    /// défaut de l'éq. (1729), `6 × A/80` en habitation pour la surface desservie (« δlvc = 0 »),
    /// signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l_vc_m: Option<f64>,
    /// « Lhvc_2nd-e » : longueur hors volume chauffé d'une distribution, en m.
    pub l_hvc_m: f64,
    /// « nbdist_2nd-e » : nombre de distributions identiques reliées à l'émetteur.
    pub nb_dist: u32,
    /// Espace tampon traversé par la partie hors volume chauffé (« btherm », éq. 1733). Absent :
    /// à l'extérieur (`btherm = 1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
}

/// Th-BCE 2020 §9.8 — distribution d'ECS intergroupe, reliée à une génération.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DistributionEcsIntergroupe {
    /// Identifiant, unique dans le bâtiment.
    pub id: Id,
    /// Identifiant de la génération qui produit l'ECS.
    pub generation: Id,
    /// « Typeréseauintergroupe-e ».
    pub reseau: ReseauEcsIntergroupe,
}

/// « Typeréseauintergroupe-e » (§9.8.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ReseauEcsIntergroupe {
    /// 0 : pas de réseau intergroupe (production individuelle) : ni pertes ni auxiliaires
    /// (éq. 1762 à 1765).
    Aucun,
    /// 1 : réseau bouclé (§9.8.3.3).
    Boucle(BoucleEcs),
    /// 2 : réseau tracé, à ruban chauffant (§9.8.3.4).
    Trace(ReseauEcsTrace),
}

/// Réseau d'ECS intergroupe bouclé (§9.8.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoucleEcs {
    /// « Lvc_prim_bcl-e » : longueur en volume chauffé, en m.
    pub l_vc_m: f64,
    /// « Lhvc_prim_bcl-e » : longueur hors volume chauffé, en m.
    pub l_hvc_m: f64,
    /// « Uprim-e » : coefficient linéique, en W/(m.K).
    pub u_w_mk: f64,
    /// « Isrechauf_bcl-e » : réchauffeur de boucle électrique.
    pub rechauffeur: bool,
    /// « Pcirc-e » : puissance des circulateurs, en W (fonctionnement permanent en habitation).
    pub p_circ_w: f64,
    /// Espace tampon traversé par la partie hors volume chauffé (« btherm », éq. 1744).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
}

/// Réseau d'ECS intergroupe tracé (§9.8.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReseauEcsTrace {
    /// « Lvc_prim_trac-e » : longueur en volume chauffé, en m.
    pub l_vc_m: f64,
    /// « Lhvc_prim_trac-e » : longueur hors volume chauffé, en m.
    pub l_hvc_m: f64,
    /// « Uprim-e » : coefficient linéique, en W/(m.K).
    pub u_w_mk: f64,
    /// Espace tampon traversé par la partie hors volume chauffé (« btherm », éq. 1756).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
}

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
