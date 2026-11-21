// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : générateurs thermodynamiques à compression électrique (§8.23) et leurs
//! sources amont (§8.26).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::entree::*;

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
    /// 4 : PAC eau de nappe / eau. Températures amont : 3,5, 8,5, 13,5 et 18,5 °C ; pivot à
    /// 8,5 °C (source `nappe`).
    EauDeNappeEau,
    /// 5 : PAC sol / eau. Températures amont : −4, 1,5, 4 et 6,5 °C ; pivot à 4 °C (source
    /// `sol`).
    SolEau,
    /// 6 : PAC eau glycolée / eau. Températures amont : −6,5, −1,5, 3,5, 8,5 et 13,5 °C ; pivot
    /// à −1,5 °C (source `captage`).
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

/// Source amont d'un générateur thermodynamique (§8.26 et §8.27) : sur air
/// (« idamont-air-type », §8.26.3.3.1), sur eau (« idamont-eau-type », §8.26.3.3.2) ou sur sol
/// (fiche 8.27).
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
    /// Eau, « idamont-eau-type = 1 » : captage par sondes ou capteurs enterrés parcourus d'eau
    /// glycolée (éq. 1408 à 1410 et 1427 ; pompes : éq. 1444, 1445 et 1448). Sources des
    /// technologies « eau glycolée », et des refroidisseurs sur eau ou sur « eau de nappe (ou
    /// captage) » (lecture Q145).
    Captage {
        /// « θmin_source », « θmax_source » et « idmois mini » : température de l'interface
        /// avec le sol naturel (éq. 1408 et 1409).
        sol: TemperatureSinusoidale,
        /// « Rb » : résistance thermique équivalente entre le fluide et l'interface avec le
        /// sol naturel, en K.m/W (éq. 1410).
        rb_k_m_w: f64,
        /// « L » : longueur du conduit de captage, en m (éq. 1410).
        l_m: f64,
        /// Pompes de captage (éq. 1444 et 1445).
        pompes: PompesCaptage,
        /// « ΔθEvap_CH » et « ΔθCond_FR » (éq. 1427).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ecarts: Option<EcartsSourceEau>,
    },
    /// Eau, « idamont-eau-type = 4 ou 5 » : eau de nappe, avec ou sans échangeur de barrage
    /// (éq. 1408, 1409, 1411 à 1427 ; pompes : éq. 1444 à 1448).
    Nappe {
        /// « θmin_source », « θmax_source » et « idmois mini » : température de l'eau de
        /// nappe (éq. 1408 et 1409).
        nappe: TemperatureSinusoidale,
        /// Pompes de captage (« Ppompes_captage », éq. 1444 et 1445).
        pompes: PompesCaptage,
        /// Échangeur de barrage et circuit intermédiaire (« idamont-eau-type = 4 ») ; absent :
        /// nappe sans échangeur (« idamont-eau-type = 5 », éq. 1425 et 1426).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        echangeur: Option<EchangeurNappe>,
        /// « ΔθEvap_CH » et « ΔθCond_FR » (éq. 1427).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ecarts: Option<EcartsSourceEau>,
    },
    /// Eau, « idamont-eau-type = 2 » : tour de refroidissement, en production de froid seul
    /// (éq. 1428 à 1436, 1449 à 1451).
    TourDeRefroidissement {
        /// « idtour » : tour humide ou sèche.
        nature: NatureTourRefroidissement,
        /// « Δθtour » : écart entre l'eau en sortie de tour et la température humide (tour
        /// humide) ou sèche (tour sèche) de l'air extérieur, en K (éq. 1432 et 1434).
        delta_theta_tour_k: f64,
        /// « θes_tour_consigne » : consigne de température d'eau en sortie de tour, en °C
        /// (éq. 1433 et 1435).
        theta_es_consigne_c: f64,
        /// « Ppompes_tour » : puissance électrique des pompes du circuit de refroidissement,
        /// en W (éq. 1449).
        p_pompes_tour_w: f64,
        /// « Pvent_tour » : puissance électrique des ventilateurs de la tour, en W (éq. 1450).
        p_vent_tour_w: f64,
        /// « ΔθCond_FR » : écart de température aux bornes du condenseur, en K (éq. 1436).
        /// Absent : 5 K, signalé dans les résultats.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        delta_theta_cond_fr_k: Option<f64>,
    },
    /// Eau, « idamont-eau-type = 3 » : boucle d'eau dont la température suit une évolution
    /// annuelle sinusoïdale (éq. 1437 à 1439 ; pompes : éq. 1444, 1445 et 1448). Le bilan
    /// d'une boucle d'eau générateur (fiche 8.15) n'est pas calculé.
    BoucleDEau {
        /// « θmin_boucle », « θmax_boucle » et mois de la température minimale (éq. 1437 et
        /// 1438).
        boucle: TemperatureSinusoidale,
        /// Pompes de boucle (« Ppompes_captage », éq. 1444 et 1445).
        pompes: PompesCaptage,
        /// « ΔθCond_CH » et « ΔθCond_FR » (éq. 1439).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ecarts: Option<EcartsSourceEau>,
    },
    /// Sol, « idfluide-amont = 3 » (fiche 8.27) : capteurs à détente directe dans le sol des
    /// PAC sol / eau et sol / sol, en chauffage et en ECS (éq. 1461 à 1466, tableau 245). Pas
    /// d'auxiliaires amont (éq. 1465).
    Sol {
        /// « θmin_sol », « θmax_sol » et « id_mois_mini_sol » : température de l'interface
        /// entre le sol et les tubes de fluide frigorigène (éq. 1461 et 1462).
        sol: TemperatureSinusoidale,
        /// « Rb » : résistance thermique équivalente, en K.m/W (éq. 1463).
        rb_k_m_w: f64,
        /// « L » : longueur du conduit de captage, en m (éq. 1463).
        l_m: f64,
    },
}

/// Température d'une source qui suit une évolution annuelle sinusoïdale, mois par mois
/// (§8.26.3.3.2, éq. 1408, 1409, 1437 et 1438 ; §8.27.3.2, éq. 1461 et 1462). Une source de
/// température constante se décrit avec `theta_min_c = theta_max_c`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TemperatureSinusoidale {
    /// Température minimale annuelle, en °C.
    pub theta_min_c: f64,
    /// Température maximale annuelle, en °C.
    pub theta_max_c: f64,
    /// « idmois mini » : numéro du mois de la température minimale, de 1 à 12.
    pub mois_minimum: u8,
}

/// « idgest_captage » et « idgest_pompe_captage_cont_var » (§8.26.3.4.2) : pompes d'une source
/// sur eau.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PompesCaptage {
    /// Gestion des pompes.
    pub gestion: GestionPompesCaptage,
    /// « Ppompes_captage » : puissance électrique des pompes de captage, de nappe ou de
    /// boucle, en W (selon le type, toute la puissance ou la part qui vainc les pertes de
    /// charge des conduits).
    pub p_pompes_w: f64,
}

/// « idgest_captage » et « idgest_pompe_captage_cont_var ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GestionPompesCaptage {
    /// « idgest_captage = 1 » : marche permanente pendant la saison de chauffage ou de
    /// refroidissement (éq. 1411, 1444 et 1446).
    MarchePermanente,
    /// « idgest_captage = 2 », « idgest_pompe_captage_cont_var = 1 » : marche en cas de
    /// demande, tout ou rien (éq. 1412 et 1445).
    DemandeToutOuRien,
    /// « idgest_captage = 2 », « idgest_pompe_captage_cont_var = 2 » : marche en cas de
    /// demande, à vitesse variable (éq. 1413 et 1445).
    DemandeVitesseVariable,
}

/// Échangeur de barrage d'une source sur eau de nappe et circuit intermédiaire (§8.26.3.3.2.1,
/// « idamont-eau-type = 4 », éq. 1411 à 1424).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EchangeurNappe {
    /// « Qv_nappe_nom » : débit nominal d'eau de nappe, en m³/h.
    pub qv_nappe_nom_m3_h: f64,
    /// « Qv_inter_nom » : débit total nominal du fluide du circuit intermédiaire, en m³/h.
    pub qv_inter_nom_m3_h: f64,
    /// « ρinter » : masse volumique du fluide du circuit intermédiaire, en kg/m³.
    pub rho_inter_kg_m3: f64,
    /// « Cpeinter » : chaleur spécifique du fluide du circuit intermédiaire, en J/(kg.K).
    pub cp_inter_j_kg_k: f64,
    /// « UA » : coefficient de l'échangeur, en W/K.
    pub ua_w_k: f64,
    /// « type_echangeur » : nature des écoulements.
    pub ecoulements: EcoulementsEchangeur,
    /// « Ppompes_inter » : puissance électrique des pompes du circuit intermédiaire, en W
    /// (éq. 1446 et 1447).
    pub p_pompes_inter_w: f64,
}

/// « type_echangeur » (§8.26.3.3.2.1, éq. 1418 à 1421).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EcoulementsEchangeur {
    /// 1 : écoulements à contre-courant (éq. 1418 et 1419).
    ContreCourant,
    /// 2 : écoulements à courants parallèles (éq. 1420).
    CourantsParalleles,
    /// 3 : écoulements à courants croisés, les deux fluides brassés (éq. 1421).
    CourantsCroises,
}

/// « idtour » (§8.26.3.3.2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NatureTourRefroidissement {
    /// 1 : tour humide (éq. 1428 à 1433).
    Humide,
    /// 2 : tour sèche (éq. 1434 et 1435).
    Seche,
}

/// « ΔθEvap_CH » (ou « ΔθCond_CH ») et « ΔθCond_FR » : écarts de température aux bornes de
/// l'échangeur côté source en chauffage et en refroidissement (§8.26.3.3.2, éq. 1427 et 1439).
/// Chacun, absent, vaut 5 K par défaut, signalé dans les résultats.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EcartsSourceEau {
    /// Écart en chauffage et en ECS, en K.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_theta_ch_k: Option<f64>,
    /// Écart en refroidissement, en K.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_theta_fr_k: Option<f64>,
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
    /// 4 : PAC eau de nappe / eau. Températures aval : 23,5, 32,5, 42,5, 51 et 60 °C ; amont :
    /// 3,5, 8,5, 13,5 et 18,5 °C ; pivot : 8,5 °C et 32,5 °C (source `nappe`).
    EauDeNappeEau,
    /// 5 : PAC eau glycolée / eau. Températures amont : −6,5, −1,5, 3,5, 8,5 et 13,5 °C ;
    /// pivot : −1,5 °C et 32,5 °C (source `captage`).
    EauGlycoleeEau,
    /// 6 : PAC eau de nappe / air recyclé. Températures aval (air intérieur) : 5, 10, 15, 20
    /// et 25 °C ; amont : 3,5, 8,5, 13,5 et 18,5 °C ; pivot : 8,5 °C et 20 °C (source `nappe`).
    EauDeNappeAirRecycle,
    /// 7 : PAC eau de boucle / air recyclé. Températures amont : 8,5, 13,5, 18,5, 23,5 et
    /// 28,5 °C ; pivot : 18,5 °C et 20 °C (source `boucle_d_eau`).
    EauDeBoucleAirRecycle,
    /// 8 : PAC sol / eau. Températures amont : −4, 1,5, 4 et 6,5 °C ; pivot : 4 °C et 32,5 °C
    /// (source `sol`).
    SolEau,
    /// 9 : PAC sol / sol. Température aval unique, 35 °C ; amont : −4, 1,5, 4 et 6,5 °C ;
    /// pivot : 4 °C (source `sol`).
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
    /// 4 : refroidisseur eau / eau et eau glycolée / eau. Températures aval : 4, 9,5, 15,
    /// 20,5 et 26 °C ; amont : 2,5, 12,5, 22,5, 32,5 et 42,5 °C ; pivot : 32,5 °C et 9,5 °C
    /// (source sur eau).
    EauEau,
    /// 5 : refroidisseur eau / air et eau de boucle / air recyclé. Températures aval : 22, 27,
    /// 32 et 37 °C ; amont comme `eau_eau` ; pivot : 32,5 °C et 27 °C (source sur eau).
    EauAir,
    /// 6 : refroidisseur eau de nappe (ou captage) / air recyclé. Températures aval : 22, 27,
    /// 32 et 37 °C ; amont : 7,5, 12,5, 17,5 et 22,5 °C ; pivot : 12,5 °C et 27 °C (source
    /// `nappe` ou `captage`).
    EauDeNappeAirRecycle,
    /// 7 : refroidisseur eau de nappe (ou captage) / eau. Températures aval : 4, 9,5, 15, 20,5
    /// et 26 °C ; amont : 7,5, 12,5, 17,5 et 22,5 °C ; pivot : 12,5 °C et 9,5 °C (source
    /// `nappe` ou `captage`).
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
