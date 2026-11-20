// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : générateurs thermodynamiques au gaz (type 504) — pompes à chaleur et
//! refroidisseurs à absorption (§8.24) et pompes à chaleur à moteur gaz (§8.25).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::entree::*;

/// Th-BCE 2020 §8.24 et §8.25 — générateur thermodynamique au gaz.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ThermodynamiqueGazSaisi {
    /// §8.24 : pompe à chaleur à absorption au gaz (GAHP) en chauffage ou en ECS, ou
    /// refroidisseur de liquide à absorption. Une machine assure un seul mode (« Idfougen »).
    Absorption(Box<AbsorptionGazSaisie>),
    /// §8.25 : pompe à chaleur à compression entraînée par un moteur au gaz naturel (PAC MG),
    /// réversible, sur air extérieur.
    MoteurGaz(Box<PacMoteurGazSaisie>),
}

/// §8.24.2, tableau 217 — machine à absorption au gaz, sur air extérieur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AbsorptionGazSaisie {
    /// « Idfougen » et technologie : mode unique de la machine.
    pub mode: ModeAbsorptionGaz,
    /// Performances à pleine charge : GUE, puissance gaz absorbée (« débit calorifique ») et
    /// puissance électrique des auxiliaires.
    pub performances: PerformancesGaz,
    /// « Lim_Theta » : limites de fonctionnement (§8.24.3.3.1.5). Absent : pas de limite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limites: Option<LimitesPac>,
    /// « Fonc_brûl » : régulation du brûleur. Absent : tout ou rien (§8.24.3.6.4.1, lecture
    /// Q137), signalé dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bruleur: Option<FonctionnementBruleur>,
    /// « Idechangeur » : présence d'un échangeur eau/fumées. Absent : pas d'échangeur
    /// (§8.24.3.6.4.1), signalé dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echangeur_fumees: Option<bool>,
    /// « LRcontmin » et « CcpLRcontmin » d'un brûleur modulant, certifiés ou justifiés
    /// (éq. 1357 et 1358). Absent : 0,70 et 0,90 en chauffage et en refroidissement, 0,65 et
    /// 0,75 en ECS (éq. 1362 et 1363), signalés dans les résultats. Sans objet en tout ou rien.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fonctionnement_continu: Option<FonctionnementContinu>,
    /// « Paux0 » : puissance des auxiliaires à charge nulle d'une machine, en W. Absent :
    /// valeur par défaut selon la puissance nominale (éq. 1364).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_aux_0: Option<ValeurMesuree>,
    /// « Pertes40°C » : pertes à l'arrêt pour 40 K d'écart avec l'ambiance, en W. Absent :
    /// valeur par défaut selon la puissance nominale (éq. 1365).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pertes_40c: Option<ValeurMesuree>,
    /// « Rdtcomb_pivot » : rendement de combustion sur PCI au point pivot (sans dimension).
    /// Absent : valeur par défaut (éq. 1366).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rdt_comb_pivot: Option<ValeurMesuree>,
}

/// Valeur certifiée ou justifiée d'une caractéristique de charge partielle (§8.24.3.6.4.1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ValeurMesuree {
    /// Valeur mesurée.
    pub valeur: f64,
    /// Statut : une valeur justifiée est corrigée selon la règle de la grandeur (éq. 1359 à
    /// 1361).
    pub statut: StatutMesure,
}

/// « Fonc_brûl » (tableau 217).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FonctionnementBruleur {
    /// 1 : fonctionnement en mode continu du brûleur, puis en cycles marche-arrêt sous le taux
    /// de charge minimal.
    Modulant,
    /// 2 : cycles marche-arrêt du brûleur (`LRcontmin = 1`).
    ToutOuRien,
}

/// Mode unique d'une machine à absorption (« Idfougen » 1, 2 ou 3) et sa technologie.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ModeAbsorptionGaz {
    /// 1 : chauffage (§8.24.3.3).
    Chauffage {
        /// « Syst_Thermo_CH ».
        technologie: TechnologieChauffageGahp,
        /// « Typo_emetteur » : inertie du système d'émission, qui fixe « Dfou0 » (figure 167).
        typo_emetteur: TypoEmetteur,
    },
    /// 2 : refroidissement (refroidisseur de liquide, §8.24.3.5).
    Refroidissement {
        /// « Syst_Thermo_FR ».
        technologie: TechnologieRefroidissementGahp,
        /// « Type_cycle_FR » : cycle de la machine, qui fixe la valeur utile maximale de l'EER
        /// pivot (0,7 ou 1,0). Pas de valeur par défaut.
        cycle: CycleAbsorption,
        /// « Typo_emetteur » (figure 167).
        typo_emetteur: TypoEmetteur,
    },
    /// 3 : ECS (§8.24.3.4). La machine est le générateur de base d'un assemblage avec
    /// stockage (§8.24.1).
    Ecs {
        /// « Syst_Thermo_ECS ».
        technologie: TechnologieEcsGahp,
    },
}

/// « Syst_Thermo_CH » (tableau 217).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TechnologieChauffageGahp {
    /// 1 : GAHP air / eau classique. Températures aval : 30, 35, 45 et 50 °C ; amont : −15,
    /// −7, 2, 7 et 20 °C ; pivot : 7 °C et 45 °C (figure 140).
    AirEauClassique,
    /// 2 : GAHP air / eau haute température. Températures aval : 30, 35, 45, 50 et 60 °C
    /// (figure 143).
    AirEauHauteTemperature,
    /// 3 : GAHP eau glycolée / eau classique : non pris en charge (source amont sur eau).
    EauGlycoleeEauClassique,
    /// 4 : GAHP eau glycolée / eau haute température : non pris en charge.
    EauGlycoleeEauHauteTemperature,
    /// 5 : GAHP eau / eau : non pris en charge.
    EauEau,
}

/// « Syst_Thermo_FR » (tableau 217).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TechnologieRefroidissementGahp {
    /// 1 : refroidisseur air / eau. Températures aval : 4, 9,5, 15, 20,5 et 26 °C ; amont : 5,
    /// 15, 25, 35 et 45 °C ; pivot : 35 °C et 9,5 °C (figure 161).
    AirEau,
    /// 2 : refroidisseur eau / eau (eau de tour) : non pris en charge.
    EauEau,
}

/// « Syst_Thermo_ECS » (tableau 217).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TechnologieEcsGahp {
    /// 1 : GAHP air / eau. Températures aval : 30, 35, 45, 50 et 60 °C ; amont : −15, −7, 2,
    /// 7 et 20 °C ; pivot : 7 °C et 60 °C (figure 155).
    AirEau,
    /// 2 : GAHP eau glycolée / eau : non pris en charge.
    EauGlycoleeEau,
    /// 3 : GAHP eau / eau : non pris en charge.
    EauEau,
}

/// « Type_cycle_FR » (tableau 217).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CycleAbsorption {
    /// 1 : simple effet ou GAX (`Val_util_max = 0,7`).
    SimpleEffetOuGax,
    /// 2 : double effet (`Val_util_max = 1,0`).
    DoubleEffet,
}

/// Performances à pleine charge d'un générateur thermodynamique au gaz (§8.24.3 et
/// §8.25.3.1) : « Statut_données », « statut_valeur_pivot » et « statut_Paux_pivot ».
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PerformancesGaz {
    /// Valeurs certifiées ou justifiées (« Statut_données = 1 »), dont la valeur pivot, pour
    /// chaque couple du croisement de températures aval et amont prises dans l'ordre de
    /// priorité de la technologie. Les autres cases sont complétées par les coefficients
    /// « Cnn » de la technologie.
    Mesurees(Vec<PointPerformanceGaz>),
    /// Valeur pivot déclarée (« statut_valeur_pivot = 1 ») : GUE réduit de 20 % et plafonné à
    /// la valeur utile maximale de la technologie.
    PivotDeclare {
        /// GUE (ou EER d'une PAC à moteur gaz en refroidissement) déclaré au point pivot.
        performance: f64,
        /// Puissance gaz absorbée au point pivot, en kW (sur PCI).
        pabs_kw: f64,
        /// Puissance électrique des auxiliaires au point pivot, en W, déclarée : majorée selon
        /// la fiche. Absent : valeur par défaut de la fiche, signalée dans les résultats.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        paux_w: Option<f64>,
    },
    /// Valeur pivot par défaut (« statut_valeur_pivot = 2 ») : 80 % de la valeur utile
    /// maximale de la technologie, signalée dans les résultats.
    PivotDefaut {
        /// Puissance gaz absorbée au point pivot, en kW (sur PCI).
        pabs_kw: f64,
        /// Puissance électrique des auxiliaires au point pivot, en W, comme pour
        /// `pivot_declare`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        paux_w: Option<f64>,
    },
}

/// Valeur mesurée d'une matrice de performances au gaz.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PointPerformanceGaz {
    /// Température aval de la mesure, en °C : une des températures de la technologie.
    pub theta_aval_c: f64,
    /// Température amont de la mesure, en °C : une des températures de la technologie.
    pub theta_amont_c: f64,
    /// GUE (ou EER) mesuré à pleine charge.
    pub performance: f64,
    /// Puissance gaz absorbée mesurée à pleine charge, en kW (jamais corrigée).
    pub pabs_kw: f64,
    /// Puissance électrique des auxiliaires mesurée à pleine charge, en W (unité extérieure
    /// d'une PAC à moteur gaz).
    pub paux_w: f64,
    /// Statut : un GUE justifié est réduit de 10 %, une puissance d'auxiliaires justifiée
    /// majorée de 10 %.
    pub statut: StatutMesure,
}

/// §8.25.2, tableau 229 — pompe à chaleur à moteur gaz (PAC MG) sur air extérieur, réversible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PacMoteurGazSaisie {
    /// « Fonct_moteur ». Absent : cycles marche-arrêt (valeur par défaut du tableau 229),
    /// signalé dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moteur: Option<FonctionnementMoteurGaz>,
    /// « LRcontmin » et « CcpLRcontmin » d'un moteur à vitesse variable, certifiés ou justifiés
    /// (§8.25.3.1). Absent : 0,4 et 1,0, signalés dans les résultats. Sans objet en cycles
    /// marche-arrêt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fonctionnement_continu: Option<FonctionnementContinu>,
    /// « Paux0 » : puissance électrique des auxiliaires à charge nulle, déclarée, en W.
    /// Absent : 0,12 kW (§8.25.3.1), signalé dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_aux_0_w: Option<f64>,
    /// Mode chauffage ; absent : la machine ne chauffe pas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chauffage: Option<ModePacMoteurGaz>,
    /// Mode refroidissement ; absent : la machine ne refroidit pas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refroidissement: Option<ModePacMoteurGaz>,
    /// Production indirecte d'ECS par récupération de chaleur sur le moteur (§8.25.3.4.5) :
    /// non prise en charge dans cette version (lecture Q144).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_ecs_indirecte: Option<ProductionEcsIndirecteSaisie>,
}

/// « Fonct_moteur » (tableau 229).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FonctionnementMoteurGaz {
    /// 0 : cycles marche-arrêt du moteur (`LRcontmin = 1`).
    CyclesMarcheArret,
    /// 1 : vitesse variable, puis cycles marche-arrêt sous le taux de charge minimal.
    VitesseVariable,
}

/// Mode chauffage ou refroidissement d'une PAC à moteur gaz (§8.25.3.2 et §8.25.3.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModePacMoteurGaz {
    /// « Syst_thermo_ch » ou « Syst_thermo_fr ».
    pub technologie: TechnologiePacMoteurGaz,
    /// Performances à pleine charge (GUE ou EER, puissance gaz absorbée, puissance des
    /// auxiliaires de l'unité extérieure « Paux,uext »).
    pub performances: PerformancesGaz,
    /// « Paux,uint,pc » : puissance électrique des unités intérieures à pleine charge, déclarée,
    /// en W. Absent : 6 % de la puissance gaz absorbée au point pivot (éq. 1369), signalé dans
    /// les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_aux_uint_w: Option<f64>,
}

/// « Syst_thermo_ch/fr » (tableau 229).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TechnologiePacMoteurGaz {
    /// PAC MG air extérieur / eau (fluide aval eau). Chauffage : aval 23,5, 32,5, 42,5, 51 et
    /// 60 °C, pivot 7 °C et 32,5 °C ; refroidissement : aval 4, 9,5, 15, 20,5 et 26 °C, pivot
    /// 35 °C et 9,5 °C.
    AirExterieurEau,
    /// PAC MG air extérieur / air recyclé (fluide aval air). Chauffage : aval 5, 10, 15, 20 et
    /// 25 °C, pivot 7 °C et 20 °C ; refroidissement : aval 22, 27, 32 et 37 °C, pivot 35 °C et
    /// 27 °C.
    AirExterieurAirRecycle,
}

/// Production indirecte d'ECS d'une PAC à moteur gaz (tableau 229) : reconnue, non calculée.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProductionEcsIndirecteSaisie {
    /// « ηth,ecs » pivot : efficacité de la production indirecte ramenée à la puissance gaz.
    /// Absent : valeur par défaut de la fiche.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta_th_ecs: Option<f64>,
    /// « Ppompe,ECS » : puissance de la pompe de charge, en W.
    pub p_pompe_ecs_w: f64,
}
