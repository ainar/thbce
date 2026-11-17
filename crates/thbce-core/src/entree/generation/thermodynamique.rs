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
