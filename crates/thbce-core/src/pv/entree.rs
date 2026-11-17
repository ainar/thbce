// SPDX-License-Identifier: GPL-3.0-or-later
//! Entrées de la production photovoltaïque (Th-BCE 2020 §12.1 à 12.4).
//!
//! Une installation est un champ photovoltaïque intégré au bâtiment et connecté au réseau : un
//! ensemble de modules connectés à un même onduleur (§12.1.1). « Dans un projet, cet ensemble
//! doit être utilisé pour chaque type de panneaux qui différeraient par leurs caractéristiques
//! d'installation ou par leurs caractéristiques propres » (§12.2.1) : une installation par
//! orientation, inclinaison, technologie ou onduleur.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::entree::{Exposition, Id};

/// Th-BCE 2020 §12.2.2 — installation photovoltaïque intégrée au bâtiment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstallationPv {
    /// Identifiant de l'installation.
    pub id: Id,
    /// « α », « β », « lpb », « hpb » et masques (`Az`, `γi`, `dE`, `hpE` : masques du catalogue,
    /// §3.2) : orientation du plan des modules. L'inclinaison va de 0 à 90°. « alb » vaut 0,2
    /// (§3.2.2).
    pub exposition: Exposition,
    /// « N » : nombre de modules du champ, au moins 1.
    pub nb_modules: u32,
    /// Modules : technologie, puissance crête, surface et performances (§12.3.3.1).
    pub modules: ModulesPv,
    /// « CT » : confinement thermique de la face arrière des modules (§12.3.3.2). Valeur par
    /// défaut : face arrière confinée.
    #[serde(default)]
    pub confinement: ConfinementPv,
    /// Onduleur (§12.4). Sans saisie : aucune information de rendement, puissance nominale par
    /// défaut.
    #[serde(default)]
    pub onduleur: OnduleurPv,
}

/// « Type_Techno_Capteur » (tableau 328).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TechnologiePv {
    /// Silicium monocristallin.
    MonoCristallinSi,
    /// Silicium multicristallin.
    MultiCristallinSi,
    /// Tellurure de cadmium.
    CdTe,
    /// Cuivre-indium-sélénium.
    Cis,
    /// Silicium amorphe.
    AmorpheSi,
    /// Autre technologie.
    Autre,
}

/// « CT » : degré de confinement thermique de la face arrière des modules (§12.3.3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConfinementPv {
    /// Face arrière libre, `CT = 1` (par exemple un brise-soleil, face arrière directement en
    /// contact avec l'air ambiant).
    Libre,
    /// Autre cas, `CT = 1,5`.
    Autre,
    /// Face arrière confinée, `CT = 2` (intégration sur toiture isolée sans ventilation
    /// aménagée spécifiquement pour le champ, ou double vitrage en face arrière). Valeur par
    /// défaut.
    #[default]
    Confine,
}

impl ConfinementPv {
    /// Valeur de `CT`.
    pub fn ct(self) -> f64 {
        match self {
            Self::Libre => 1.0,
            Self::Autre => 1.5,
            Self::Confine => 2.0,
        }
    }
}

/// Th-BCE 2020 §12.3.2 — caractéristiques d'un module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModulesPv {
    /// « Type_Techno_Capteur ».
    pub technologie: TechnologiePv,
    /// « S » : surface ensoleillée d'un module en œuvre, en m².
    pub s_m2: f64,
    /// « Pc », « μ » et « NOCT » selon leur origine (§12.3.3.1).
    pub performances: PerformancesPv,
}

/// Origine des performances « Pc », « μ » et « NOCT » (§12.3.3.1) : elle fixe les valeurs de
/// calcul, réduites par rapport aux valeurs saisies.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "statut", rename_all = "snake_case", deny_unknown_fields)]
pub enum PerformancesPv {
    /// Valeurs certifiées par un organisme accrédité (NF EN 61215 ou 61646) : reprises telles
    /// quelles.
    Certifie {
        /// « Pc » : puissance crête d'un module en conditions STC, en W.
        pc_w: f64,
        /// « μ » : coefficient de température de la puissance crête, en °C⁻¹.
        mu_par_c: f64,
        /// « NOCT » : température d'équilibre thermique du module, en °C.
        noct_c: f64,
    },
    /// Valeurs justifiées par un essai d'un laboratoire accrédité : `0,9 × Pc`, `1,10 × μ`,
    /// `1,10 × NOCT`.
    Justifie {
        /// « Pc » justifiée, en W.
        pc_w: f64,
        /// « μ » justifié, en °C⁻¹.
        mu_par_c: f64,
        /// « NOCT » justifiée, en °C.
        noct_c: f64,
    },
    /// Valeurs déclarées : `0,8 × Pc`, `max(1,20 × μ ; μ_util_min)`,
    /// `max(1,20 × NOCT ; NOCT_util_min)`.
    Declare {
        /// « Pc » déclarée, en W.
        pc_w: f64,
        /// « μ » déclaré, en °C⁻¹.
        mu_par_c: f64,
        /// « NOCT » déclarée, en °C.
        noct_c: f64,
    },
    /// Valeur par défaut : seule la puissance crête est saisie ; `0,8 × Pc`, `1,20 × μ_util_min`,
    /// `1,20 × NOCT_util_min`.
    Defaut {
        /// « Pc » saisie, en W.
        pc_w: f64,
    },
}

impl PerformancesPv {
    /// « Pc » saisie d'un module, en W.
    pub fn pc_saisie_w(&self) -> f64 {
        match *self {
            Self::Certifie { pc_w, .. }
            | Self::Justifie { pc_w, .. }
            | Self::Declare { pc_w, .. }
            | Self::Defaut { pc_w } => pc_w,
        }
    }
}

/// Th-BCE 2020 §12.4.2 — onduleur.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OnduleurPv {
    /// « PAC NOM » : puissance nominale alternative, en W. Sans saisie : `0,8 × N × Pc`
    /// (§12.4.3), avec `Pc` la valeur de calcul.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_ac_nom_w: Option<f64>,
    /// Information de rendement disponible (cas n° 1 à 3 du §12.4.3).
    #[serde(default)]
    pub rendement: RendementOnduleur,
}

/// Information de rendement d'un onduleur (§12.4.3, « Définition de la fonction ηOND »).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "cas", rename_all = "snake_case", deny_unknown_fields)]
pub enum RendementOnduleur {
    /// Cas n° 1 : aucune information, `ηONDBASE = 0,9`. Valeur par défaut.
    #[default]
    Defaut,
    /// Cas n° 2 : rendement européen `ηEU` seul.
    Europeen {
        /// « ηEU » : rendement européen, entre 0 et 1.
        eta_eu: f64,
    },
    /// Cas n° 3 : courbe de rendement à la tension nominale, aux points de passage obligés
    /// `Xi = 0,05 ; 0,1 ; 0,25 ; 0,5 ; 0,75 ; 1` (CEI 61683) au minimum.
    Courbe {
        /// Points `(Xi, ηi)`, `Xi` étant le rapport `PPV / PAC NOM`.
        points: Vec<PointRendementOnduleur>,
    },
}

/// Point de la courbe de rendement d'un onduleur.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PointRendementOnduleur {
    /// « Xi » : `PPV / PAC NOM`.
    pub x: f64,
    /// « ηi » : rendement à ce point, entre 0 et 1.
    pub eta: f64,
}
