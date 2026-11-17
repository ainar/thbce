// SPDX-License-Identifier: GPL-3.0-or-later
//! Indicateurs de confort d'un groupe : fiche `C_Ein_Indicateurs de confort`
//! (Th-BCE 2020 §4.5).
//!
//! À chaque pas de temps, la fiche compare la température opérative du groupe en évolution
//! libre, `θop,moy(0;0)`, à la consigne de chauffage, à la consigne de refroidissement et aux
//! seuils d'inconfort chaud des trois catégories d'ambiance. Les écarts servent à la
//! détermination des saisons (§4.6) ; les nombres d'heures et les degrés-heures sont des sorties.
//!
//! La moyenne glissante de la température extérieure, `θrm(j)` (éq. 57), est celle de la fiche
//! `C_EEX_Climat extérieur` (éq. 1) : elle est calculée par
//! [`crate::climat::ConfortAdaptatif`]. L'éq. (57) écrit « si j = 1, θrm(j) = 0 », ce qui
//! confirme la lecture retenue pour l'éq. (1) (décision Q11).

use libm::fabs;

/// « Δθop_inc_C1 », « Δθop_inc_C2 » et « Δθop_inc_C3 » : paramètres conventionnels du seuil
/// d'inconfort chaud des catégories d'ambiance 1, 2 et 3, en °C (§4.5.2).
pub const DELTA_THETA_OP_INC_C: [f64; 3] = [2.0, 3.0, 4.0];

/// Pente du seuil d'inconfort chaud en fonction de `θrm` (éq. 58).
pub const PENTE_THETA_RM: f64 = 0.33;

/// Ordonnée à l'origine du seuil d'inconfort chaud, en °C (éq. 58).
pub const ORDONNEE_SEUIL_C: f64 = 18.8;

/// « θop_inc_max_Cn(h) » : seuil de température opérative d'inconfort chaud de la catégorie
/// d'ambiance `n` (de 1 à 3), en °C.
///
/// Th-BCE 2020 §4.5.3.2.1, éq. (58) :
/// `θop_inc_max_Cn = max(θiifr(h) ; 0,33 × θrm(j) + 18,8 + Δθop_inc_Cn)`.
pub fn theta_op_inc_max_c(theta_ii_fr_c: f64, theta_rm_c: f64, categorie: u8) -> f64 {
    let delta = DELTA_THETA_OP_INC_C[usize::from(categorie.clamp(1, 3)) - 1];
    theta_ii_fr_c.max(PENTE_THETA_RM * theta_rm_c + ORDONNEE_SEUIL_C + delta)
}

/// Lecture du facteur des éq. (60) et (61) (`docs/questions/Q038.md`, point 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LectureEcarts {
    /// Lecture retenue : chaque écart est pondéré par son propre indicateur, `i × (i + 1)/2`,
    /// qui vaut 1 en inconfort et 0 sinon (y compris en inoccupation, où `i = −1`). L'écart
    /// d'inconfort chaud est alors l'écart au seuil, comme le dit le texte (« écart à la
    /// température de confort »).
    #[default]
    IndicateurPropre,
    /// Équations imprimées : les écarts d'inconfort chaud sont pondérés par
    /// `i_fr × (iinconf_cons,ch + 1)/2`, avec l'indicateur d'inconfort **froid**. En occupation
    /// et en inconfort chaud, celui-ci vaut 0 : l'écart est divisé par deux.
    Litterale,
}

/// Indicateurs de confort d'un groupe à un pas de temps (§4.5.2, sorties). Les indicateurs
/// valent 1 en occupation et en inconfort, 0 en occupation sans inconfort, −1 en inoccupation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IndicateursConfort {
    /// « iinconf_cons,ch(h) » : inconfort froid par rapport à la consigne de chauffage.
    pub i_inconf_cons_ch: i8,
    /// « iinconf_cons,fr(h) » : inconfort chaud par rapport à la consigne de refroidissement.
    pub i_inconf_cons_fr: i8,
    /// « iinconf_C1,fr(h) » à « iinconf_C3,fr(h) » : inconfort chaud par rapport au seuil des
    /// catégories d'ambiance 1 à 3.
    pub i_inconf_c_fr: [i8; 3],
    /// « Δθop_inc_cons,ch(h) » : écart à la consigne de chauffage en inconfort froid, en °C.
    pub delta_theta_op_inc_cons_ch_c: f64,
    /// « Δθop_inc_cons,fr(h) » : écart à la consigne de refroidissement en inconfort chaud, en
    /// °C.
    pub delta_theta_op_inc_cons_fr_c: f64,
    /// « Δθop_inc_C1,fr(h) » à « Δθop_inc_C3,fr(h) » : écart au seuil d'inconfort chaud des
    /// catégories d'ambiance 1 à 3, en °C.
    pub delta_theta_op_inc_c_fr_c: [f64; 3],
}

/// Indicateur à trois valeurs des éq. (56) et (59).
fn indicateur(i_occ_zone: bool, inconfort: bool) -> i8 {
    match (i_occ_zone, inconfort) {
        (false, _) => -1,
        (true, true) => 1,
        (true, false) => 0,
    }
}

/// Écart à un critère d'inconfort, en °C.
///
/// Th-BCE 2020 §4.5.3.3, éq. (60) et (61) : `Δθ = i × (iinconf_cons,ch + 1)/2 × |seuil − θop|`.
/// Pour l'inconfort froid (éq. 60), `i` est `iinconf_cons,ch` lui-même : le facteur
/// `i × (i + 1)/2` vaut 1 en inconfort, 0 sinon. Pour l'inconfort chaud (éq. 61), le texte
/// garde `iinconf_cons,ch` dans la fraction : voir [`LectureEcarts`].
fn ecart(
    i_propre: i8,
    i_inconf_cons_ch: i8,
    seuil_c: f64,
    theta_op_c: f64,
    lecture: LectureEcarts,
) -> f64 {
    let i_fraction = match lecture {
        LectureEcarts::IndicateurPropre => i_propre,
        LectureEcarts::Litterale => i_inconf_cons_ch,
    };
    f64::from(i_propre) * (f64::from(i_fraction) + 1.0) / 2.0 * fabs(seuil_c - theta_op_c)
}

/// Indicateurs de confort d'un groupe à un pas de temps, avec la lecture retenue des écarts.
pub fn indicateurs(
    i_occ_zone: bool,
    theta_ii_ch_c: f64,
    theta_ii_fr_c: f64,
    theta_rm_c: f64,
    theta_op_moy_00_c: f64,
) -> IndicateursConfort {
    indicateurs_selon(
        LectureEcarts::default(),
        i_occ_zone,
        theta_ii_ch_c,
        theta_ii_fr_c,
        theta_rm_c,
        theta_op_moy_00_c,
    )
}

/// Indicateurs de confort d'un groupe à un pas de temps.
///
/// Th-BCE 2020 §4.5.3 :
/// - éq. (56) : en occupation, inconfort froid si `θop,moy(0;0)(h) < θiich(h)` ;
/// - éq. (59) : en occupation, inconfort chaud si `θop,moy(0;0)(h) > θiifr(h)` (critère de la
///   consigne) ou `> θop_inc_max_Cn(h)` (catégorie d'ambiance `n`, éq. 58) ;
/// - en inoccupation, tous les indicateurs valent −1 ;
/// - éq. (60) et (61) : écarts aux critères, nuls hors inconfort.
///
/// `theta_op_moy_00_c` est la température opérative moyenne du groupe à puissances des systèmes
/// nulles ; `theta_rm_c`, la moyenne glissante de la température extérieure du jour.
pub fn indicateurs_selon(
    lecture: LectureEcarts,
    i_occ_zone: bool,
    theta_ii_ch_c: f64,
    theta_ii_fr_c: f64,
    theta_rm_c: f64,
    theta_op_moy_00_c: f64,
) -> IndicateursConfort {
    let theta_op = theta_op_moy_00_c;
    let i_ch = indicateur(i_occ_zone, theta_op < theta_ii_ch_c);
    let i_fr = indicateur(i_occ_zone, theta_op > theta_ii_fr_c);
    let mut i_c = [0_i8; 3];
    let mut delta_c = [0.0; 3];
    for (n, categorie) in (1..=3_u8).enumerate() {
        let seuil = theta_op_inc_max_c(theta_ii_fr_c, theta_rm_c, categorie);
        i_c[n] = indicateur(i_occ_zone, theta_op > seuil);
        delta_c[n] = ecart(i_c[n], i_ch, seuil, theta_op, lecture);
    }
    IndicateursConfort {
        i_inconf_cons_ch: i_ch,
        i_inconf_cons_fr: i_fr,
        i_inconf_c_fr: i_c,
        delta_theta_op_inc_cons_ch_c: ecart(
            i_ch,
            i_ch,
            theta_ii_ch_c,
            theta_op,
            LectureEcarts::IndicateurPropre,
        ),
        delta_theta_op_inc_cons_fr_c: ecart(i_fr, i_ch, theta_ii_fr_c, theta_op, lecture),
        delta_theta_op_inc_c_fr_c: delta_c,
    }
}

impl IndicateursConfort {
    /// « Δθop_inc,fr(h) » à retenir pour la détermination des saisons selon la catégorie
    /// d'ambiance du groupe, de 0 à 3.
    ///
    /// Th-BCE 2020 §4.6.3.2, tableau 9 : `Cat_amb = 0`, `Δθop_inc_cons,fr` ; 1, 2 et 3,
    /// `Δθop_inc_C1,fr`, `Δθop_inc_C2,fr` et `Δθop_inc_C3,fr`.
    pub fn delta_theta_op_inc_fr_c(&self, cat_amb: u8) -> f64 {
        match cat_amb {
            0 => self.delta_theta_op_inc_cons_fr_c,
            n => self.delta_theta_op_inc_c_fr_c[usize::from(n.min(3)) - 1],
        }
    }
}

/// « Cat_amb » : catégorie d'ambiance retenue pour la détermination des saisons.
///
/// Th-BCE 2020 §4.6.3.2, tableau 10 : 1 pour les bâtiments à usage d'habitation comme pour les
/// autres usages.
pub const CAT_AMB: u8 = 1;

/// Cumuls annuels des indicateurs de confort d'un groupe (§4.5.3.4 et 4.5.3.5).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CumulsConfort {
    /// « Nbhinc_cons,ch » : nombre d'heures d'inconfort froid, en h (éq. 62).
    pub nbh_inc_cons_ch: u32,
    /// « Nbhinc_cons,fr » : nombre d'heures d'inconfort chaud par rapport à la consigne de
    /// refroidissement, en h (éq. 63).
    pub nbh_inc_cons_fr: u32,
    /// « Nbhinc_C1,fr » à « Nbhinc_C3,fr », en h (éq. 63).
    pub nbh_inc_c_fr: [u32; 3],
    /// « DHinc_cons,ch » : degrés-heures d'inconfort froid, en K.h (éq. 64).
    pub dh_inc_cons_ch_k_h: f64,
    /// « DHinc_cons,fr » : degrés-heures d'inconfort chaud par rapport à la consigne de
    /// refroidissement, en K.h (éq. 65).
    pub dh_inc_cons_fr_k_h: f64,
    /// « DHinc_C1,fr » à « DHinc_C3,fr », en K.h (éq. 65).
    pub dh_inc_c_fr_k_h: [f64; 3],
}

impl CumulsConfort {
    /// Ajoute un pas de temps.
    ///
    /// Th-BCE 2020 §4.5.3.4, éq. (62) et (63) : un compteur d'heures est incrémenté quand son
    /// indicateur vaut 1. §4.5.3.5, éq. (64) et (65) : somme des écarts.
    pub fn ajouter(&mut self, i: &IndicateursConfort) {
        self.nbh_inc_cons_ch += u32::from(i.i_inconf_cons_ch == 1);
        self.nbh_inc_cons_fr += u32::from(i.i_inconf_cons_fr == 1);
        self.dh_inc_cons_ch_k_h += i.delta_theta_op_inc_cons_ch_c;
        self.dh_inc_cons_fr_k_h += i.delta_theta_op_inc_cons_fr_c;
        for n in 0..3 {
            self.nbh_inc_c_fr[n] += u32::from(i.i_inconf_c_fr[n] == 1);
            self.dh_inc_c_fr_k_h[n] += i.delta_theta_op_inc_c_fr_c[n];
        }
    }
}
