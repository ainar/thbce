// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats de l'enveloppe : synthèse par groupe, baies et espaces tampons.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ValeurParDefaut;
use crate::entree::Id;

/// Résultats d'un espace tampon (Th-BCE 2020 §5.4) : synthèse annuelle de ses sorties
/// horaires dans chacun des deux calculs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsEspaceTampon {
    /// Identifiant de l'espace tampon.
    pub id: Id,
    /// Calcul du Bbio (mode Th-B).
    pub th_b: Option<ResultatsEspaceTamponMode>,
    /// Calcul du confort d'été (mode Th-D).
    pub th_d: Option<ResultatsEspaceTamponMode>,
    /// Valeurs par défaut du texte appliquées (`Cm_et_surf`, `Qv,et,base`).
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// Synthèse annuelle d'un espace tampon dans un mode de calcul.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsEspaceTamponMode {
    /// Moyenne annuelle de « btherm(h) ».
    pub b_therm_moyen: f64,
    /// Moyenne annuelle de « θet(h) », en °C.
    pub theta_et_moy_c: f64,
    /// Plus basse « θet(h) », en °C.
    pub theta_et_min_c: f64,
    /// Plus haute « θet(h) », en °C.
    pub theta_et_max_c: f64,
    /// Nombre d'heures où les parois de l'espace tampon sont ouvertes (surventilation).
    pub nbh_surventilation: u32,
}

/// Résultats d'une baie.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsBaie {
    /// Identifiant de la baie.
    pub id: Id,
    /// Valeurs par défaut de la méthode appliquées aux paramètres non saisis de la baie ; liste
    /// vide si tout a été saisi.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// Grandeurs d'enveloppe d'un groupe (Th-BCE 2020 §5.16.3 et §5.20.3). Les coefficients de
/// transmission sont ceux des éléments à leur inclinaison réelle (§5.17.3, éq. 294).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsEnveloppe {
    /// « Aop » : surface des parois opaques, en m² (éq. 286).
    pub a_op_m2: f64,
    /// « Aopv » : surface des parois opaques verticales, en m² (éq. 287).
    pub a_opv_m2: f64,
    /// « Aophh » : surface des parois opaques horizontales tournées vers le haut, en m² (éq. 287).
    pub a_ophh_m2: f64,
    /// « Aophb » : surface des parois opaques horizontales tournées vers le bas, en m² (éq. 287).
    pub a_ophb_m2: f64,
    /// « HTh » des parois opaques, en W/K (éq. 283).
    pub h_th_op_w_k: f64,
    /// « Abaies » : surface des baies, en m² (§5.14.3, éq. 282).
    pub a_baies_m2: f64,
    /// « Abaies-v » : surface des baies verticales, d'inclinaison au moins égale à 60°, en m²
    /// (éq. 279).
    pub a_baies_v_m2: f64,
    /// « Abaies-h » : surface des baies horizontales, d'inclinaison inférieure à 60°, en m²
    /// (éq. 281).
    pub a_baies_h_m2: f64,
    /// « Hges » des baies sans protection mobile en place, en W/K (§5.10.3.5, éq. 210 avec
    /// `Rprot = 0`). La valeur horaire dépend de la position des protections mobiles.
    pub h_ges_sp_w_k: f64,
    /// « Af,perm » : surface des parois déperditives verticales, opaques et vitrées, en m²,
    /// utile aux calculs de perméabilité (§5.7.2 ; « Af » du §5.6.3.1.5).
    pub a_f_perm_m2: f64,
    /// « Atoit,perm » : surface des parois déperditives horizontales hors plancher bas, opaques
    /// et vitrées, en m², utile aux calculs de perméabilité (§5.7.2 ; « At » du §5.6.3.1.5).
    pub a_toit_perm_m2: f64,
    /// « Ll » : longueur des ponts thermiques, en m (éq. 310).
    pub l_pt_m: f64,
    /// « HTh » des ponts thermiques, en W/K (éq. 307).
    pub h_th_pt_w_k: f64,
}
