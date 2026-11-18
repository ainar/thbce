// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : systèmes de chauffage et de refroidissement du groupe et intergroupes
//! (Th-BCE 2020 §8.1 à 8.10) — émetteurs du groupe, programmation des relances, distributions du
//! groupe et intergroupes.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::*;

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
    /// Émetteur contenu dans les bouches de soufflage de la centrale de traitement d'air de la
    /// zone (« émetteur contenu dans le composant bouches-conduits », §6.3.2, tableau 61) :
    /// sa demande est assurée par la batterie régulée sur la charge de la centrale (mode actif,
    /// §6.3.3.5) et passe au réseau de CTA (éq. 940), non au réseau `distribution`
    /// (`docs/questions/Q095.md`). Absent : faux.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub centrale: Option<bool>,
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
    /// Émetteur contenu dans les bouches de soufflage de la centrale de traitement d'air de la
    /// zone (« émetteur contenu dans le composant bouches-conduits », §6.3.2, tableau 61) :
    /// sa demande est assurée par la batterie régulée sur la charge de la centrale (mode actif,
    /// §6.3.3.5) et passe au réseau de CTA (éq. 940), non au réseau `distribution`
    /// (`docs/questions/Q095.md`). Absent : faux.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub centrale: Option<bool>,
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
