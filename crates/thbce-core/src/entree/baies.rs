// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : baies, ouvrants et protections mobiles.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::*;

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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ExpoBr {
    /// BR1.
    Br1,
    /// BR2.
    Br2,
    /// BR3.
    Br3,
}

/// Détermination détaillée de la classe d'exposition au bruit d'une baie (fiche d'application
/// 5.11 ; annexe de l'article R. 172-4, chapitre V, partie I, point 3). **Facultative** : elle
/// remplace la saisie directe de `expo_br`.
///
/// La catégorie sonore des voies (arrêté préfectoral de classement, article R. 571-38 du code
/// de l'environnement) et le plan d'exposition au bruit de l'aérodrome (arrêté préfectoral) sont
/// des **données à saisir** : le moteur ne peut ni les déterminer, ni les retrouver. Il en
/// déduit la classe de la baie (tableaux de la partie I, configurations 1 et 2) et, de là, celle
/// des groupes et des zones (point 4) quand `exigences.zone_bruit` n'est pas saisi.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BruitBaie {
    /// Infrastructures de transports terrestres vues de la baie, une entrée par infrastructure
    /// (distance et vue propres à chacune). Plusieurs infrastructures : la classe la plus
    /// défavorable est retenue. Une infrastructure au-delà de sa distance maximale de prise en
    /// compte (700, 500, 250, 100 ou 30 m selon la catégorie 1 à 5) donne BR1 : il est inutile
    /// de la saisir.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub infrastructures: Vec<InfrastructureBruit>,
    /// Zone du plan d'exposition au bruit de l'aérodrome le plus proche où se situe le
    /// bâtiment (zones A à C : BR3, D : BR2, hors zone : BR1, « toutes vues »). Absente : aucun
    /// aérodrome à prendre en compte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone_peb: Option<ZonePeb>,
    /// Identifiant de la façade (libre, commun aux baies d'une même façade du bâtiment). « À
    /// défaut d'une détermination détaillée, la classe BR d'une baie d'une façade est la classe
    /// la plus élevée des baies de cette façade » : sans `infrastructures` ni `zone_peb`, la baie
    /// prend la classe la plus élevée des autres baies de la même façade (déterminées en détail
    /// ou par `expo_br`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facade: Option<Id>,
}

/// Une infrastructure de transports terrestres vue d'une baie (annexe de l'article R. 172-4,
/// chapitre V, partie I, points 2 et 3 ; fiche 5.11).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InfrastructureBruit {
    /// Catégorie de l'infrastructure (1 à 5), donnée par l'arrêté préfectoral de classement des
    /// voies : donnée à saisir.
    pub categorie: CategorieInfrastructure,
    /// Distance de la façade à l'infrastructure, en m : plus courte distance au bord extérieur
    /// de la chaussée la plus proche, en projection en plan (fiche 5.11.4).
    pub distance_m: f64,
    /// Vue de l'infrastructure depuis la baie.
    pub vue: VueInfrastructure,
}

/// Catégorie d'une infrastructure de transports terrestres (arrêté du 30 mai 1996).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CategorieInfrastructure {
    /// Catégorie 1 (distance maximale de prise en compte : 700 m).
    #[serde(rename = "categorie_1")]
    Categorie1,
    /// Catégorie 2 (500 m).
    #[serde(rename = "categorie_2")]
    Categorie2,
    /// Catégorie 3 (250 m).
    #[serde(rename = "categorie_3")]
    Categorie3,
    /// Catégorie 4 (100 m).
    #[serde(rename = "categorie_4")]
    Categorie4,
    /// Catégorie 5 (30 m).
    #[serde(rename = "categorie_5")]
    Categorie5,
}

/// Vue de l'infrastructure depuis la baie (chapitre V, partie I, point 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VueInfrastructure {
    /// Vue directe : vue en plan de l'infrastructure de plus de 90°, après déduction des
    /// obstacles très protecteurs.
    Directe,
    /// Vue partielle : vue en plan inférieure à 90°, après déduction des obstacles très
    /// protecteurs.
    Partielle,
    /// Vue masquée par des obstacles peu protecteurs (altitude inférieure à celle de l'étage
    /// considéré, tout en masquant l'infrastructure).
    MasqueePeuProtecteurs,
    /// Vue masquée par des obstacles très protecteurs (altitude supérieure ou égale à celle de
    /// l'étage considéré).
    MasqueeTresProtecteurs,
    /// Vue arrière : façade arrière du bâtiment par rapport à l'infrastructure.
    Arriere,
}

/// Zone du plan d'exposition au bruit (PEB) d'un aérodrome (code de l'urbanisme, articles
/// R. 112-4 à R. 112-17) : donnée à saisir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ZonePeb {
    /// Zone A (BR3).
    A,
    /// Zone B (BR3).
    B,
    /// Zone C (BR3).
    C,
    /// Zone D (BR2).
    D,
    /// Hors zone du plan d'exposition au bruit (BR1).
    HorsZone,
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

/// Affectation d'une baie à un local pour l'éclairement naturel (§5.10.3.2, « type de baie »).
///
/// Seule compte la distinction entre local de grand volume et local de volume normal ; pour un
/// grand volume, il faut savoir si la baie est un éclairant de toiture réparti de façon
/// uniforme (distance entre deux éclairants inférieure à la hauteur du local, distance des
/// éclairants périphériques à la périphérie inférieure à la moitié de cette hauteur).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AffectationEclairage {
    /// Baie d'un local de volume normal (type 2, « VN »). Valeur par défaut.
    VolumeNormal,
    /// Baie d'un local de grand volume : baie verticale, ou éclairant de toiture non réparti de
    /// façon uniforme (type 0, « GV »).
    GrandVolume,
    /// Éclairant de toiture (baie horizontale, inclinaison nulle) réparti de façon uniforme sur
    /// la toiture d'un local de grand volume (type 1, « GVU »).
    GrandVolumeToitureUniforme,
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
    /// baie a un ouvrant, sauf si `bruit` permet de la déduire ; si les deux sont saisies, la
    /// validation vérifie leur cohérence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expo_br: Option<ExpoBr>,
    /// Détermination détaillée de la classe d'exposition au bruit (fiche 5.11) : catégorie et
    /// distance des infrastructures, vue, plan d'exposition au bruit. Facultative.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bruit: Option<BruitBaie>,
    /// Affectation de la baie à un local de grand volume ou de volume normal, pour l'éclairement
    /// naturel (§5.10.3.2). Absente : volume normal. Un groupe qui a des baies de grand volume
    /// doit avoir un local de grand volume (salle de sport, aire de production, aire de vente de
    /// plus de 300 m²).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affectation_eclairage: Option<AffectationEclairage>,
    /// Identifiant de l'espace tampon sur lequel donne la baie (§5.10.3, « IdET »). Une baie
    /// ne peut donner que sur un espace tampon solarisé. Absent : la baie donne sur
    /// l'extérieur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
}
