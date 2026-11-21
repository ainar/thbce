// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : chaudières au gaz, au fioul ou au bois (§8.19), poêles et inserts (§8.22).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::entree::*;

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

/// « idraccord_gnr » (§8.20.2 et §8.21.2) : raccordement hydraulique d'un générateur à
/// combustion ou d'un cogénérateur au reste de la génération, pour un mode de gestion avec
/// priorité (cascade ou alternance).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RaccordementGenerateur {
    /// 0 : raccordé en permanence : le générateur a des pertes à l'arrêt à chaque pas de
    /// temps où il ne fonctionne pas.
    Permanent,
    /// 1 : avec isolement (possible) : le générateur n'a de pertes à l'arrêt que s'il a
    /// fonctionné au pas de temps précédent (valeur par défaut du tableau 123).
    AvecIsolement,
}

/// Évacuation des fumées d'un radiateur, poêle ou insert au gaz sans ventilateur côté
/// combustion (tableau 129).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvacuationFumees {
    /// Par micro-ventouse (`fmaj` = 1,04).
    MicroVentouse,
    /// Par une cheminée (`fmaj` = 1,06).
    Cheminee,
}

/// Type d'un autre générateur à combustion (§8.20.3.1, tableau 124) ; les déclinaisons
/// précisent les lignes des tableaux 126 à 130.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TypeAutreGenerateurCombustion {
    /// Type 103 : radiateur, poêle ou insert au gaz (chauffage, sur air ambiant).
    RadiateurGaz {
        /// Présence d'un ventilateur côté combustion (tableaux 126 et 129).
        ventilateur_combustion: bool,
        /// Présence d'un ventilateur côté émission (tableau 128 : 40 W par ventilateur).
        ventilateur_emission: bool,
        /// Évacuation des fumées : requise sans ventilateur côté combustion (tableau 129).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        evacuation_fumees: Option<EvacuationFumees>,
    },
    /// Type 104 : chauffe-eau au gaz (ECS, sur eau).
    ChauffeEauGaz,
    /// Type 105 : accumulateur au gaz (ECS, sur eau).
    AccumulateurGaz {
        /// Accumulateur à condensation (tableau 126).
        condensation: bool,
        /// Appareil de plus de 200 L dont le temps de montée en température est inférieur à
        /// 45 min (tableau 127 : `E` = 1,7 au lieu de 1,5).
        #[serde(default)]
        plus_de_200_l_montee_rapide: bool,
    },
    /// Types 106 et 107 : générateur d'air chaud au gaz (chauffage, sur air ambiant).
    AirChaud {
        /// Générateur à condensation (type 107 ; standard sinon, type 106).
        condensation: bool,
        /// Présence d'un ventilateur côté émission (tableau 128).
        ventilateur_emission: bool,
    },
    /// Type 108 : tube radiant au gaz (chauffage, sur air ambiant).
    TubeRadiant {
        /// Présence d'un ventilateur côté émission (tableau 128 ; sans ventilateur, le
        /// tableau ne donne pas de valeur par défaut : `w_aux_nom_w` est requis).
        ventilateur_emission: bool,
    },
    /// Type 109 : panneau radiant au gaz (chauffage, sur air ambiant ; pas de valeur par
    /// défaut des auxiliaires au tableau 128 : `w_aux_nom_w` est requis).
    PanneauRadiant,
}

/// Statut des rendements d'un autre générateur à combustion (§8.20.3.6.1.1, §8.20.3.6.2.1 et
/// §8.20.3.7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatutRendementCombustion {
    /// Certifié (NF EN ISO/CEI 17065) : valeur employée telle quelle.
    Certifie,
    /// Justifié par un essai (NF EN ISO/CEI 17025) : valeur diminuée de 10 %. Seulement pour
    /// les appareils de chauffage (types 103 et 106 à 109).
    Justifie,
    /// Déclaré par le fabricant : `min(0,8 × valeur ; 86 %)` à pleine charge et
    /// `min(0,8 × valeur ; 81 %)` à charge intermédiaire (éq. 1129, 1130 et 1132). Seulement
    /// pour les appareils de chauffage.
    Declare,
}

/// Rendements sur PCI d'un autre générateur à combustion.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RendementsCombustion {
    /// Statut des rendements.
    pub statut: StatutRendementCombustion,
    /// « Rpn » : rendement sur PCI à la puissance nominale, en %.
    pub r_pn_pct: f64,
    /// « Rpint » : rendement sur PCI à charge intermédiaire, en % ; générateurs d'air chaud
    /// seulement (absent : valeur par défaut du tableau 126).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r_pint_pct: Option<f64>,
}

/// Th-BCE 2020 §8.20.2, tableau 123 — autre générateur à combustion au gaz (types 103 à 109 :
/// radiateurs, chauffe-eau, accumulateurs, générateurs d'air chaud, tubes et panneaux
/// radiants). La fonction (« idfougen ») et le fluide aval (« idfluide_aval ») sont ceux du
/// tableau 124 ; la position de la génération fixe « idpos_gen ».
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AutreGenerateurCombustionSaisi {
    /// Type du générateur (tableau 124).
    pub type_generateur: TypeAutreGenerateurCombustion,
    /// Gaz de pétrole liquéfié (PCSI de 1,09, tableau 125) ; gaz naturel sinon (1,11).
    #[serde(default)]
    pub gpl: bool,
    /// « Pngen » : puissance utile nominale, en kW.
    pub pn_gen_kw: f64,
    /// Rendements mesurés ; absents : valeurs par défaut (§8.20.3.4, tableau 126).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendements: Option<RendementsCombustion>,
    /// « Pint » : puissance utile intermédiaire d'un générateur d'air chaud, en kW. Absent :
    /// 30 % de `Pngen` (rendement « Rpint » défini à 30 % de charge, tableau 123), valeur par
    /// défaut signalée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_int_kw: Option<f64>,
    /// « Waux,nom » : puissance électrique des auxiliaires à la puissance nominale, en W.
    /// Absent : « Paux » du tableau 128 (éq. 1124), quand le tableau en donne une.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w_aux_nom_w: Option<f64>,
    /// « Wveille » : puissance électrique à charge nulle, en W (le texte n'en donne pas de
    /// valeur par défaut).
    pub w_veille_w: f64,
    /// Puissance de la veilleuse d'allumage, en W, s'il y en a une : la consommation de veille
    /// est augmentée de cette puissance divisée par 2,58 (§8.20.3.4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_veilleuse_w: Option<f64>,
    /// « Qpo30 » : pertes à l'arrêt pour un écart de 30 K entre l'eau ou l'air chauds et
    /// l'ambiance, en W (générateurs d'air chaud, accumulateurs et chauffe-eau). Absent :
    /// éq. (1123) et tableau 127.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_po_30_w: Option<f64>,
    /// « θfonct_min » : température minimale de fonctionnement d'un générateur sur eau
    /// (accumulateur, chauffe-eau), en °C (de 20 à 100). Absent : 20 °C (§8.20.3.2.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theta_fonct_min_c: Option<f64>,
    /// « idpertes_parois » (tableau 131).
    pub pertes_parois: PertesParoisChaudiere,
    /// « idraccord_gnr » : raccordement à la génération, pour une gestion en cascade ou en
    /// alternance. Absent : avec isolement (valeur par défaut du tableau 123).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raccordement: Option<RaccordementGenerateur>,
}
