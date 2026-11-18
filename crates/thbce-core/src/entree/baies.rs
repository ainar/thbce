// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : baies, ouvrants et protections mobiles.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::*;

/// Th-BCE 2020 §5.10.2 — paramètres intrinsèques d'une baie vitrée.
///
/// Les facteurs d'un store vénitien (matrices `M_suntracking_*` et `M_fixe_*` du §5.10.3.6.2) se
/// saisissent dans `store_venitien` (lot 1-env, `docs/questions/Q101.md`).
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
    /// Facteurs solaires et lumineux de la partie de la baie protégée par un store vénitien
    /// (`type_pm = store_venitien`), selon la hauteur du soleil et l'inclinaison des lames
    /// (§5.10.3.6.2). Requis dans ce cas, et seulement dans ce cas ; `avec_protection` n'est
    /// alors pas utilisé.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store_venitien: Option<LamesStoreVenitien>,
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
    /// « Type-PM » : type de la première protection mobile.
    pub type_pm: TypePm,
    /// « Type-GPM » : gestion d'un volet ou d'un store enroulable. Requise pour ces deux types,
    /// absente pour un store vénitien (voir `gestion_sv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gestion: Option<GestionPm>,
    /// « Type-GPM-sv » : gestion d'un store vénitien. Requise pour un store vénitien, absente
    /// sinon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gestion_sv: Option<GestionSv>,
    /// « Detec_pres » : présence d'un détecteur de présence pour la dérogation manuelle d'un
    /// volet ou d'un store enroulable en gestion automatique. Requis dans ce cas, sans objet
    /// sinon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detecteur_presence: Option<bool>,
    /// Paramètres de l'automatisme (gestion automatique d'un volet, d'un store enroulable ou
    /// d'un store vénitien) : seuils, horloge jour/nuit, matrice des ratios de fermeture. Requis
    /// en gestion automatique, absent sinon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automatisme: Option<Automatisme>,
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
    /// 3 : store vénitien (à lames orientables, baie verticale).
    StoreVenitien,
}

/// « Type-GPM » (§5.9.2) : gestion d'un volet ou d'un store enroulable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GestionPm {
    /// 1 : automatique avec dérogation (fiche d'application FA01 : pilotage programmable selon
    /// l'éclairement incident, la température intérieure, l'occupation et la saison).
    AutomatiqueAvecDerogation,
    /// 2 : manuelle non motorisée.
    ManuelleNonMotorisee,
    /// 3 : manuelle motorisée (y compris avec commande générale dérogeable, §5.9.3.1).
    ManuelleMotorisee,
}

/// « Type-GPM-sv » (§5.9.2) : gestion d'un store vénitien.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GestionSv {
    /// 1 : gestion « sun tracking » avec dérogation : les lames restent perpendiculaires au
    /// rayonnement solaire (`αSV = 90° − γ'` en Th-BC). Gestion automatique.
    SuiviSoleilAvecDerogation,
    /// 2 : gestion manuelle, position fixe des lames (`αSV = 45°` en Th-BC).
    ManuellePositionFixe,
    /// 3 : gestion automatique, lames à 45° (position fixe), avec dérogation.
    Automatique45AvecDerogation,
}

impl GestionSv {
    /// Vrai pour les gestions automatiques (`Type-GPM-sv` = 1 ou 3).
    pub fn est_automatique(self) -> bool {
        !matches!(self, Self::ManuellePositionFixe)
    }

    /// Vrai si les lames suivent le soleil en Th-BC (`Type-GPM-sv` = 1).
    pub fn suit_le_soleil(self) -> bool {
        matches!(self, Self::SuiviSoleilAvecDerogation)
    }
}

/// Distinction du jour et de la nuit en gestion automatique (« Type_horl » et « Type_horl_sv »,
/// §5.9.3.2.7 ; fiche d'application FA01, niveaux 0 à 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeHorloge {
    /// 0 : pas de distinction du jour et de la nuit (« Jour » toujours vrai). Valeur par défaut.
    #[default]
    Aucune,
    /// 1 : horloge crépusculaire, « Jour » vrai si l'éclairement incident sur la baie est non
    /// nul.
    Crepusculaire,
    /// 2 : horloge personnalisable, « Jour » défini par les horaires `horaires_jour`.
    Personnalisable,
}

/// Heures de début et de fin du jour pour une saison (`Hj_debut` et `Hj_fin`, matrices
/// `M_horl_jour` et `M_horl_jour_sv`) : il fait jour quand `Hj_debut < Hleg ≤ Hj_fin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HorairesJour {
    /// Heure légale de début de jour (1 à 24, exclue).
    pub debut_h: u8,
    /// Heure légale de fin de jour (1 à 24, incluse).
    pub fin_h: u8,
}

/// Les deux valeurs d'une cellule de la matrice `M_Rprot1_auto` (figure 31, hystérésis).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RatiosHysteresis {
    /// Ratio de fermeture pour `Toph-1limh < θop,fin(h−1)`.
    pub haute: f64,
    /// Ratio de fermeture pour `θop,fin(h−1) < Toph-1limb`.
    pub basse: f64,
}

/// Ligne (saison et occupation) de la matrice `M_Rprot1_auto` d'un volet ou d'un store
/// enroulable (figure 48).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LigneAutoVolet {
    /// Jour, `Etp** < Eclim_auto`.
    pub jour_sous_seuil: RatiosHysteresis,
    /// Jour, `Etp** ≥ Eclim_auto`.
    pub jour_au_dessus: RatiosHysteresis,
    /// Nuit.
    pub nuit: RatiosHysteresis,
}

/// Matrice `M_Rprot1_auto` d'un volet ou d'un store enroulable : trois lignes (hiver,
/// mi-saison, été) en occupation, trois en inoccupation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MatriceAutoVolet {
    /// Groupe en occupation : hiver, mi-saison, été.
    pub occupation: [LigneAutoVolet; 3],
    /// Groupe en inoccupation : hiver, mi-saison, été.
    pub inoccupation: [LigneAutoVolet; 3],
}

/// Ligne de la matrice `M_Rprot1_auto_sv` d'un store vénitien (§5.9.3.4.3.1, figure 54).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LigneAutoSv {
    /// Jour, `Etp** < Eclim_auto_sv`.
    pub jour_sous_seuil: f64,
    /// Jour, `Etp** ≥ Eclim_auto_sv`.
    pub jour_au_dessus: f64,
    /// Nuit.
    pub nuit: f64,
}

/// Matrice `M_Rprot1_auto_sv` : hiver, mi-saison, été, en occupation puis en inoccupation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MatriceAutoSv {
    /// Groupe en occupation : hiver, mi-saison, été.
    pub occupation: [LigneAutoSv; 3],
    /// Groupe en inoccupation : hiver, mi-saison, été.
    pub inoccupation: [LigneAutoSv; 3],
}

/// Paramètres de l'automatisme d'une protection mobile en gestion automatique (§5.9.2,
/// tableau 33 ; fiche d'application FA01). Seuils sans valeur conventionnelle dans le texte :
/// à saisir. Les matrices de ratios ont des valeurs conventionnelles (figures 44 à 47), prises
/// par défaut et signalées dans les résultats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Automatisme {
    /// « Eclim_auto » (volet, store enroulable) ou « Eclim_auto_sv » (store vénitien) : seuil
    /// d'éclairement incident, en lux.
    pub eclairement_limite_lux: f64,
    /// « Toph-1limh_auto » : limite haute de la température opérative de l'heure précédente, en
    /// °C (volet, store enroulable ; requise pour ces types).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature_limite_haute_c: Option<f64>,
    /// « Toph-1limb_auto » : limite basse de la température opérative de l'heure précédente, en
    /// °C (volet, store enroulable ; requise pour ces types, au plus égale à la limite haute).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature_limite_basse_c: Option<f64>,
    /// « Vventlim_auto » ou « Vventlim_auto_sv » : vitesse du vent corrigée, en m/s, à partir de
    /// laquelle une protection extérieure est remontée. Requise pour un store enroulable ou
    /// vénitien extérieur ; sans objet pour un volet (§5.9.3.4.1.1, éq. 186 : aucune limite de
    /// vent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vent_limite_m_s: Option<f64>,
    /// Distinction jour/nuit (« Type_horl » ou « Type_horl_sv »). Absente : aucune (niveau 0).
    #[serde(default)]
    pub horloge: TypeHorloge,
    /// « M_horl_jour » ou « M_horl_jour_sv » : horaires du jour par saison (hiver, mi-saison,
    /// été), requis pour l'horloge personnalisable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horaires_jour: Option<[HorairesJour; 3]>,
    /// « M_Rprot1_auto » (volet, store enroulable). Absente : matrice conventionnelle de
    /// l'usage (figures 44 et 45), signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrice: Option<MatriceAutoVolet>,
    /// « M_Rprot1_auto_sv » (store vénitien). Absente : matrice conventionnelle de l'usage
    /// (figures 46 et 47), signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrice_sv: Option<MatriceAutoSv>,
}

/// Facteurs d'un store vénitien pour un flux incident (direct, diffus ou réfléchi) et une
/// position des lames : trois composantes du facteur solaire et transmission lumineuse (cases
/// colorées des matrices des p. 235 et 236).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FacteursLames {
    /// « Sw1 » : composante courte longueur d'onde.
    pub sw1: f64,
    /// « Sw2 » : composante grande longueur d'onde et convective.
    pub sw2: f64,
    /// « Sw3 » : composante liée à la lame d'air intérieure ventilée.
    pub sw3: f64,
    /// « Tl_global » : transmission lumineuse globale (`Tliap_dir`, `Tldap_dif` ou `Tldap_ref`).
    pub tl_global: f64,
    /// « Tlid » : part du flux incident transmise sous forme diffuse (`Tlidap_dir` ou
    /// `Tlidap_ref`) ; sans objet pour le flux diffus. Absente : 0, valeur par défaut du texte
    /// (éq. 218), signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tlid: Option<f64>,
}

/// Facteurs d'un store vénitien en Th-BC (« -c », Th-B et Th-C), matrices `M_suntracking_Sw_BC`
/// et `M_suntracking_Tl_BCE` (suivi du soleil) ou `M_fixe_Sw_BC` et `M_fixe_Tl_BCE` (position
/// fixe).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LamesThBc {
    /// Flux direct, pour les hauteurs de soleil corrigées γ' de 0, 15, 30, 45, 60 et 75°
    /// (6 éléments, dans cet ordre). En suivi du soleil, les lames sont à `αSV = 90° − γ'` ;
    /// en position fixe, à 45°.
    pub direct: Vec<FacteursLames>,
    /// Flux diffus. Suivi du soleil : 6 éléments, pour `αSV = 90° − γ'` avec γ' de 0 à 75°.
    /// Position fixe : 1 élément (`αSV = 45°`).
    pub diffus: Vec<FacteursLames>,
    /// Flux réfléchi par le sol : mêmes éléments que `diffus`.
    pub reflechi: Vec<FacteursLames>,
}

/// Facteurs d'un store vénitien en Th-D (« -e »), lames conventionnellement verticales
/// (`αSV = 90°`), indépendants de la hauteur du soleil (éq. 213) : matrices `M_*_Sw_E` et
/// `M_*_Tl_BCE`, ligne 90°.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LamesThD {
    /// Flux direct.
    pub direct: FacteursLames,
    /// Flux diffus.
    pub diffus: FacteursLames,
    /// Flux réfléchi par le sol.
    pub reflechi: FacteursLames,
}

/// Th-BCE 2020 §5.10.3.6.2 — facteurs solaires et lumineux de la partie de la baie protégée
/// par un store vénitien.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LamesStoreVenitien {
    /// Calculs Th-B et Th-C.
    pub th_bc: LamesThBc,
    /// Calcul Th-D.
    pub th_d: LamesThD,
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
