// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : production solaire d'ECS et de chauffage — chauffe-eau solaires
//! individuels et collectifs, systèmes solaires combinés, capteurs et boucles solaires (§9.15 à
//! 9.23).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::*;

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
