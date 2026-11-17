// SPDX-License-Identifier: GPL-3.0-or-later
//! Besoins bioclimatiques (Bbio), saisons et confort (DH).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::conventions::arrondir_restitution;

/// Besoins et besoin bioclimatique d'un mois ou de l'année (Th-BCE 2020 §13.1.2, sorties).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct BesoinsBbio {
    /// « B_Ch » : besoin de chauffage, en kWh par m² de surface de référence (éq. 2379 et
    /// 2382).
    pub b_ch_kwh_m2: f64,
    /// « B_Fr » : besoin de refroidissement, en kWh par m² de surface de référence, compté
    /// positivement (éq. 2380 et 2383).
    pub b_fr_kwh_m2: f64,
    /// « B_Ecl » : besoin d'éclairage, en kWh par m² de surface de référence (éq. 2381 et
    /// 2384).
    pub b_ecl_kwh_m2: f64,
    /// « Bbio_pts » : besoin bioclimatique, en points (éq. 2385 et 2386 ; décision Q10).
    pub bbio_pts: f64,
}

/// Sorties du mode Th-B d'un groupe, d'une zone ou d'un bâtiment (Th-BCE 2020 §13.1).
///
/// Sans arrondi de restitution, le Bbio annuel est la somme des douze Bbio mensuels (éq. 2386) ;
/// avec l'arrondi, chaque valeur est arrondie séparément à une décimale (décision Q8) et la
/// somme des valeurs mensuelles arrondies peut différer de la valeur annuelle arrondie.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsBbio {
    /// Besoins et Bbio de l'année.
    pub annuel: BesoinsBbio,
    /// Besoins et Bbio de chacun des douze mois de l'année conventionnelle (§4.1.3 : mois de 4
    /// ou 5 semaines, au sens des scénarios).
    pub mensuel: Vec<BesoinsBbio>,
}

/// Saisons propres d'un groupe (Th-BCE 2020 §4.6.3.6, éq. 92) : jours de l'année conventionnelle
/// de 1 à 365, au sens des scénarios (le jour 57 est le premier jour de mars). `null` si
/// l'événement n'a pas eu lieu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsSaisons {
    /// « jfin_ch » : dernier jour où la saison de chauffage s'est arrêtée.
    pub j_fin_ch: Option<u16>,
    /// « jdebut_ch » : dernier jour où la saison de chauffage a commencé (hors début d'année,
    /// où le chauffage est autorisé d'emblée).
    pub j_debut_ch: Option<u16>,
    /// « jdebut_fr » : dernier jour où la saison de refroidissement a commencé.
    pub j_debut_fr: Option<u16>,
    /// « jfin_fr » : dernier jour où la saison de refroidissement s'est arrêtée.
    pub j_fin_fr: Option<u16>,
}

/// Indicateurs de confort d'un groupe sur l'année (Th-BCE 2020 §4.5.3.4 et 4.5.3.5), calculés
/// en occupation sur la température opérative en évolution libre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsConfort {
    /// « Nbhinc_cons,ch » : heures d'inconfort froid par rapport à la consigne de chauffage.
    pub nbh_inc_cons_ch: u32,
    /// « DHinc_cons,ch » : degrés-heures d'inconfort froid, en K.h (éq. 64).
    pub dh_inc_cons_ch_k_h: f64,
    /// « Nbhinc_cons,fr » : heures d'inconfort chaud par rapport à la consigne de
    /// refroidissement.
    pub nbh_inc_cons_fr: u32,
    /// « DHinc_cons,fr » : degrés-heures d'inconfort chaud par rapport à la consigne de
    /// refroidissement, en K.h (éq. 65).
    pub dh_inc_cons_fr_k_h: f64,
    /// « Nbhinc_C1,fr » à « Nbhinc_C3,fr » : heures d'inconfort chaud par rapport au seuil des
    /// catégories d'ambiance 1 à 3.
    pub nbh_inc_c_fr: [u32; 3],
    /// « DHinc_C1,fr » à « DHinc_C3,fr », en K.h (éq. 65).
    pub dh_inc_c_fr_k_h: [f64; 3],
}

/// Confort d'été d'un groupe (Th-BCE 2020 §13.5), issu du calcul Th-D : jeu météo avec séquence
/// caniculaire, sans refroidissement. Les indicateurs portent sur les pas de temps occupés de
/// la période de confort adaptatif.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsConfortEte {
    /// « DH » : degrés-heures d'inconfort, en °C.h (éq. 2551).
    pub dh_c_h: f64,
    /// « Nbhinconf » : heures où la température opérative atteint la température d'inconfort
    /// chaud (éq. 2548).
    pub nbh_inconf: u32,
    /// « Nbhinconf+1C » : heures où elle la dépasse d'au moins 1 °C (éq. 2549).
    pub nbh_inconf_1c: u32,
    /// « Nbhinconf+2C » : heures où elle la dépasse d'au moins 2 °C (éq. 2550).
    pub nbh_inconf_2c: u32,
    /// Nombre d'heures occupées de la période de confort adaptatif, sur lesquelles portent les
    /// indicateurs.
    pub nbh_occ_conf_adapt: u32,
    /// Température opérative maximale du groupe sur l'année, en °C (fin de pas de temps).
    pub theta_op_max_c: f64,
    /// Température opérative maximale du groupe en occupation pendant la période de confort
    /// adaptatif, en °C ; `null` si la période est vide.
    pub theta_op_max_occ_c: Option<f64>,
}

impl ResultatsBbio {
    /// Applique l'arrondi de restitution à chaque valeur (§1.3.1, décision Q8).
    pub(super) fn arrondir(&mut self) {
        for b in std::iter::once(&mut self.annuel).chain(&mut self.mensuel) {
            b.b_ch_kwh_m2 = arrondir_restitution(b.b_ch_kwh_m2);
            b.b_fr_kwh_m2 = arrondir_restitution(b.b_fr_kwh_m2);
            b.b_ecl_kwh_m2 = arrondir_restitution(b.b_ecl_kwh_m2);
            b.bbio_pts = arrondir_restitution(b.bbio_pts);
        }
    }
}
