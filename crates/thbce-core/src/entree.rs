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

/// Niveau « projet » : données communes à tous les bâtiments (Th-BCE 2020 §5.1.1, note 1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Projet {
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
    pub nb_lgt: Option<u32>,
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

/// Th-BCE 2020 §6.4.3 et §6.5.2 — ventilateur d'un système simple flux en résidentiel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VentilateurSimpleFlux {
    /// « Pventbase » : puissance électrique du ventilateur en débit de base, en W.
    pub p_vent_base_w: f64,
    /// « Pventpointe » : puissance électrique du ventilateur en débit de pointe, en W.
    pub p_vent_pointe_w: f64,
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
    /// conventionnel en pointe, en m³/h, positif.
    pub q_spec_conv_pointe_m3_h: f64,
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
        cmq_surf_kj_k_m2: f64,
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
        cms_surf_kj_k_m2: f64,
    },
    /// 1 : par défaut.
    ParDefaut,
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
        cma_surf_kj_k_m2: f64,
    },
    /// 1 : par défaut, même valeur que l'inertie séquentielle (éq. 316).
    ParDefaut,
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
