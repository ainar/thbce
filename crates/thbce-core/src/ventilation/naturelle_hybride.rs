// SPDX-License-Identifier: GPL-3.0-or-later
//! Ventilation naturelle par conduits et ventilation hybride : fiche
//! `C_VEN_Ventilation naturelle et hybride par conduits` (Th-BCE 2020 §6.6, éq. 673 à 714).
//!
//! Le débit d'un conduit équivalent est la racine du bilan des pressions (éq. 673) : pertes de
//! charge de la bouche (fixe ou autoréglable, éq. 674 à 682), du conduit (éq. 683 à 687), de
//! l'extracteur statique (éq. 688 à 697), moins le tirage thermique (éq. 698 à 700), avec la
//! température, la masse volumique et la pression de la zone du pas précédent. Il en découle les
//! débits repris ou refoulés (éq. 701 à 705), l'assistance mécanique d'une ventilation hybride
//! (éq. 706 à 708) et la température de l'air repris (éq. 709 à 714).
//!
//! Convention de la fiche : débit de conduit positif en extraction ; débits du groupe repris
//! négatifs, soufflés (refoulement) positifs, comme dans le reste de la méthode.
//! Lectures : `docs/questions/Q234.md` à `Q239.md`.

use crate::entree::{AssistanceMecanique, BoucheNaturelle};

use super::bouche_conduit::{Debits, EXPOSANT_FUITES};
use super::debits_air_cep::{C_D, G_M_S2, RHO_REF_KG_M3};

/// « λcond » par défaut (tableau 65).
pub const LAMBDA_COND_DEFAUT: f64 = 0.05;
/// « χcoude » par défaut (tableau 65).
pub const CHI_COUDE_DEFAUT: f64 = 1.15;
/// « dPref » par défaut, en Pa (tableau 65).
pub const D_PREF_DEFAUT_PA: f64 = 20.0;
/// « Vvent,réf » par défaut, en m/s (tableau 65 et §6.6.3.1.4.1).
pub const V_VENT_REF_DEFAUT_M_S: f64 = 8.0;
/// « vcond,s,j », j de 1 à 8 : vitesses d'air dans le conduit des essais de l'extracteur, en m/s
/// (§6.6.3.1.4.1).
pub const V_COND_ESSAIS_M_S: [f64; 8] = [0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 8.0];
/// Différence d'altitude conventionnelle entre la bouche et le sol, ou sortie de toiture au-dessus
/// du plancher haut, en m (éq. 699 et 700).
pub const DECALAGE_TIRAGE_M: f64 = 2.0;
/// « Crdbnr » en maison individuelle ou accolée et en logement collectif (§6.6.3.2.1).
pub const CRDBNR: f64 = 1.0;
/// Nombre d'heures d'une semaine (éq. 674 et 706).
const NB_HEURES_SEMAINE: f64 = 168.0;
/// Pas de la recherche de `vcond,s_lim2` (éq. 692) : borne de `n`.
const N_MAX_LIM2: u32 = 20;

/// Conduit équivalent d'un système de ventilation naturelle ou hybride, prêt pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct ConduitResolu {
    /// « Nid ».
    pub nid: f64,
    /// « Percond », en m.
    pub per_cond_m: f64,
    /// « hcond », en m.
    pub h_cond_m: f64,
    /// « hmot », en m.
    pub h_mot_m: f64,
    /// « Acond », en m².
    pub a_cond_m2: f64,
    /// « λcond ».
    pub lambda_cond: f64,
    /// « χcoude ».
    pub chi_coude: f64,
    /// « dPref », en Pa.
    pub d_pref_pa: f64,
    /// Bouche d'extraction équivalente.
    pub bouche: BoucheNaturelle,
    /// « Aboucheeq », en m² : bouche fixe seulement (éq. 674), nul sinon.
    pub a_bouche_eq_m2: f64,
    /// « Cextr,j ».
    pub c_extr: [f64; 8],
    /// « ξextr ».
    pub xi_extr: f64,
    /// « Vvent,réf », en m/s.
    pub v_vent_ref_m_s: f64,
    /// « Kres », en m³/(s.m²) sous 1 Pa.
    pub k_res: f64,
    /// « Ratfuitevc ».
    pub ratfuitevc: f64,
    /// « Rrep », en m².K/W.
    pub r_rep_m2k_w: f64,
    /// « b ».
    pub b: f64,
    /// « Cdep ».
    pub cdep: f64,
    /// « Dugd », en h par semaine.
    pub dugd_h: f64,
}

/// Conditions d'un pas de temps : extérieur du pas, intérieur du pas précédent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionsConduit {
    /// « θext(h) », en °C.
    pub theta_ext_c: f64,
    /// « ρext(h) », en kg/m³.
    pub rho_ext_kg_m3: f64,
    /// « vvent,c(h) », en m/s.
    pub v_vent_c_m_s: f64,
    /// « θi,g,fin(h−1) », en °C.
    pub theta_int_prec_c: f64,
    /// « ρint,fin(h−1) », en kg/m³.
    pub rho_int_prec_kg_m3: f64,
    /// « Pib(h−1) », en Pa.
    pub p_ib_prec_pa: f64,
}

/// « Aboucheeq » d'une bouche fixe, en m².
///
/// Th-BCE 2020 §6.6.3.1.1, éq. (674) : sections en cm² ; moyenne pondérée par la durée
/// d'utilisation du grand débit `Dugd` (h par semaine).
pub fn a_bouche_eq_m2(dugd_h: f64, a_sect_pointe_cm2: f64, a_sect_base_cm2: f64) -> f64 {
    0.0001 * (dugd_h * a_sect_pointe_cm2 + (NB_HEURES_SEMAINE - dugd_h) * a_sect_base_cm2)
        / NB_HEURES_SEMAINE
}

/// Débit de fuites du réseau sous la pression `d_p_pa`, en m³/h.
///
/// Th-BCE 2020 §6.6.3.1.1, éq. (675) et (678) : `3600 × Kres × Acond × dP^0,667`.
pub fn q_fuites_dp_m3_h(k_res: f64, a_cond_m2: f64, d_p_pa: f64) -> f64 {
    3600.0 * k_res * a_cond_m2 * libm::pow(d_p_pa, EXPOSANT_FUITES)
}

/// Débit d'une bouche fixe sous la pression `d_p_pa`, en m³/h.
///
/// Th-BCE 2020 §6.6.3.1.1, éq. (676) : `3600 × Cd × Aboucheeq × (2/ρref × dP)^0,5`.
pub fn q_bouche_dp_m3_h(a_bouche_eq_m2: f64, d_p_pa: f64) -> f64 {
    3600.0 * C_D * a_bouche_eq_m2 * libm::sqrt(2.0 / RHO_REF_KG_M3 * d_p_pa)
}

/// Perte de charge d'une bouche fixe au débit de conduit `q_v_cond_m3_h`, en Pa.
///
/// Th-BCE 2020 §6.6.3.1.1, éq. (677) : `dPref × q×|q| / (qbouche,dPref + qfuites,dPref)²`, la
/// bouche et les fuites se comportant selon la loi de la bouche.
pub fn d_p_bouche_fixe_pa(c: &ConduitResolu, q_v_cond_m3_h: f64) -> f64 {
    let q_bouche = q_bouche_dp_m3_h(c.a_bouche_eq_m2, c.d_pref_pa);
    let q_fuites = q_fuites_dp_m3_h(c.k_res, c.a_cond_m2, c.d_pref_pa);
    let den = q_bouche + q_fuites;
    if den > 0.0 {
        c.d_pref_pa * q_v_cond_m3_h * q_v_cond_m3_h.abs() / (den * den)
    } else {
        0.0
    }
}

/// Perte de charge d'une bouche autoréglable au débit de conduit `q_v_cond_m3_h`, en Pa.
///
/// Th-BCE 2020 §6.6.3.1.2, éq. (678) à (682) : courbe du tableau 66 (point 0 : `dP0 = 0`,
/// `qv0 = 0`), débits de fuites aux pressions de mesure, interpolation linéaire entre les points
/// (débit de la bouche plus fuites) ; au-delà du dernier point et en refoulement, loi quadratique
/// du dernier ou du premier point.
pub fn d_p_bouche_autoreglable_pa(
    c: &ConduitResolu,
    d_p_pa: &[f64; 6],
    q_v_m3_h: &[f64; 6],
    q_v_cond_m3_h: f64,
) -> f64 {
    let fuites = |j: usize| q_fuites_dp_m3_h(c.k_res, c.a_cond_m2, d_p_pa[j]);
    let carre = |q: f64, q_ref: f64, f: f64| {
        let r = q / (q_ref + f);
        r * r
    };
    if q_v_cond_m3_h > q_v_m3_h[5] + fuites(5) {
        // Éq. (679).
        d_p_pa[5] * carre(q_v_cond_m3_h, q_v_m3_h[5], fuites(5))
    } else if q_v_cond_m3_h <= 0.0 {
        // Éq. (680) : refoulement.
        -d_p_pa[0] * carre(q_v_cond_m3_h, q_v_m3_h[0], fuites(0))
    } else if q_v_cond_m3_h <= q_v_m3_h[0] + fuites(0) {
        // Éq. (681).
        d_p_pa[0] * carre(q_v_cond_m3_h, q_v_m3_h[0], fuites(0))
    } else {
        // Éq. (682) : `j` est l'indice (de 1 à 6) du premier point dont débit et fuites dépassent
        // le débit du conduit ; les points 0 valent `dP0 = qv0 = 0`.
        let mut j = 1;
        while j < 5 && q_v_cond_m3_h > q_v_m3_h[j] + fuites(j) {
            j += 1;
        }
        let (dp_j, q_j, f_j) = (d_p_pa[j], q_v_m3_h[j], fuites(j));
        let (dp_k, q_k, f_k) = (d_p_pa[j - 1], q_v_m3_h[j - 1], fuites(j - 1));
        dp_k + (dp_j - dp_k) / (q_j + f_j - q_k - f_k) * (q_v_cond_m3_h - q_k - f_k)
    }
}

/// Part du débit de conduit qui passe par les fuites du réseau, pour une perte de charge de la
/// bouche `d_p_bouche_pa`.
///
/// Th-BCE 2020 §6.6.3.2.1, éq. (701) et (704) :
/// `Kres·Acond·|dP|^0,167 / (Cd·Aboucheeq·(2/ρref)^0,5 + Kres·Acond·|dP|^0,167)`. Bouche
/// autoréglable (`Aboucheeq` n'est défini que pour une bouche fixe) : lue comme
/// `qfuites / (qbouche + qfuites)` à `|dP|`, le débit de la bouche étant celui de la courbe du
/// tableau 66 (Q235 point 2).
pub fn part_fuites(c: &ConduitResolu, d_p_bouche_pa: f64) -> f64 {
    let d_p = d_p_bouche_pa.abs();
    match &c.bouche {
        BoucheNaturelle::Fixe { .. } => {
            let f = c.k_res * c.a_cond_m2 * libm::pow(d_p, 0.167);
            let b = C_D * c.a_bouche_eq_m2 * libm::sqrt(2.0 / RHO_REF_KG_M3);
            if b + f > 0.0 {
                f / (b + f)
            } else {
                0.0
            }
        }
        BoucheNaturelle::Autoreglable { d_p_pa, q_v_m3_h } => {
            let f = q_fuites_dp_m3_h(c.k_res, c.a_cond_m2, d_p);
            let b = q_bouche_courbe_m3_h(d_p_pa, q_v_m3_h, d_p);
            if b + f > 0.0 {
                f / (b + f)
            } else {
                0.0
            }
        }
    }
}

/// Débit d'une bouche autoréglable sous la pression `d_p`, en m³/h : interpolation linéaire de la
/// courbe du tableau 66 (point 0 compris), loi en racine de la pression au-delà du dernier point.
fn q_bouche_courbe_m3_h(d_p_pa: &[f64; 6], q_v_m3_h: &[f64; 6], d_p: f64) -> f64 {
    if d_p >= d_p_pa[5] {
        return q_v_m3_h[5] * libm::sqrt(d_p / d_p_pa[5]);
    }
    let mut j = 0;
    let mut dp_k = 0.0;
    let mut q_k = 0.0;
    while j < 6 && d_p > d_p_pa[j] {
        dp_k = d_p_pa[j];
        q_k = q_v_m3_h[j];
        j += 1;
    }
    let j = j.min(5);
    q_k + (q_v_m3_h[j] - q_k) * (d_p - dp_k) / (d_p_pa[j] - dp_k)
}

/// « Asection,cond » : section du conduit, en m².
///
/// Th-BCE 2020 §6.6.3.1.3, éq. (683) : `Percond² / (4π)`.
pub fn section_conduit_m2(per_cond_m: f64) -> f64 {
    per_cond_m * per_cond_m / (4.0 * core::f64::consts::PI)
}

/// « Dcond » : diamètre du conduit, en m.
///
/// Th-BCE 2020 §6.6.3.1.3, éq. (684) : `4 × Asection,cond / Percond`.
pub fn diametre_conduit_m(per_cond_m: f64) -> f64 {
    4.0 * section_conduit_m2(per_cond_m) / per_cond_m
}

/// « vcond,s » : vitesse de l'air dans le conduit, en m/s.
///
/// Th-BCE 2020 §6.6.3.1.3, éq. (685) : `qv,rep,cond / (Asection,cond × 3600)`.
pub fn vitesse_conduit_m_s(q_v_cond_m3_h: f64, per_cond_m: f64) -> f64 {
    q_v_cond_m3_h / (section_conduit_m2(per_cond_m) * 3600.0)
}

/// « dPcond » : pertes de charge linéaires du conduit, en Pa.
///
/// Th-BCE 2020 §6.6.3.1.3, éq. (686) : `0,5 × λcond × hcond/Dcond × ρint × v × |v|`.
pub fn d_p_conduit_pa(c: &ConduitResolu, rho_int_kg_m3: f64, v_m_s: f64) -> f64 {
    0.5 * c.lambda_cond
        * (c.h_cond_m / diametre_conduit_m(c.per_cond_m))
        * rho_int_kg_m3
        * v_m_s
        * v_m_s.abs()
}

/// « dPcoude » : pertes de charge singulières du conduit, en Pa.
///
/// Th-BCE 2020 §6.6.3.1.3, éq. (687) : `0,5 × χcoude × ρint × v × |v|`.
pub fn d_p_coude_pa(c: &ConduitResolu, rho_int_kg_m3: f64, v_m_s: f64) -> f64 {
    0.5 * c.chi_coude * rho_int_kg_m3 * v_m_s * v_m_s.abs()
}

/// Limites du modèle de l'extracteur pour un vent non nul : « vcond,s_lim1 », « dPextr_lim1 »,
/// « vcond,s_lim2 » et « dPextr_lim2 ».
#[derive(Debug, Clone, Copy, PartialEq)]
struct LimitesExtracteur {
    v_lim1: f64,
    d_p_lim1: f64,
    v_lim2: f64,
    d_p_lim2: f64,
}

/// Limites de l'extracteur pour le vent `v_vent_c_m_s` > 0.
///
/// Th-BCE 2020 §6.6.3.1.4.1, éq. (689) à (692). Éq. (689) : lue `v8 × vvent,c / Vvent,réf`, la
/// vitesse du conduit dont la vitesse corrigée (éq. 688) vaut `v8` ; le texte écrit le rapport
/// inverse (Q234 point 1).
fn limites_extracteur(c: &ConduitResolu, cond: &ConditionsConduit) -> LimitesExtracteur {
    let v_lim1 = V_COND_ESSAIS_M_S[7] * cond.v_vent_c_m_s / c.v_vent_ref_m_s;
    let d_p_lim1 = 0.5 * cond.rho_ext_kg_m3 * c.c_extr[7] * cond.v_vent_c_m_s * cond.v_vent_c_m_s;
    let rho_moy = 0.5 * (cond.rho_int_prec_kg_m3 + cond.rho_ext_kg_m3);
    let sans_vent = |v: f64| 0.5 * rho_moy * c.xi_extr * v * v;
    let mut n: u32 = 2;
    while sans_vent(f64::from(n) * v_lim1) < d_p_lim1 && n < N_MAX_LIM2 {
        n += 1;
    }
    let v_lim2 = f64::from(n) * v_lim1;
    LimitesExtracteur {
        v_lim1,
        d_p_lim1,
        v_lim2,
        d_p_lim2: sans_vent(v_lim2),
    }
}

/// « dPextr » : dépression (ou perte de charge) de l'extracteur statique, en Pa, pour la vitesse
/// `v_m_s` dans le conduit.
///
/// Th-BCE 2020 §6.6.3.1.4.2, éq. (693) à (697).
pub fn d_p_extracteur_pa(c: &ConduitResolu, cond: &ConditionsConduit, v_m_s: f64) -> f64 {
    let vent = cond.v_vent_c_m_s;
    let rho_moy = 0.5 * (cond.rho_int_prec_kg_m3 + cond.rho_ext_kg_m3);
    if v_m_s <= 0.0 {
        // Éq. (693) : refoulement, `Cextr = Cextr,1`.
        return 0.5 * cond.rho_ext_kg_m3 * c.c_extr[0] * vent * vent;
    }
    if vent > 0.0 {
        let l = limites_extracteur(c, cond);
        if v_m_s <= l.v_lim1 {
            // Éq. (694) et (695) : interpolation de `Cextr` à la vitesse corrigée (éq. 688).
            let v_corr = v_m_s * c.v_vent_ref_m_s / vent;
            let mut j = 1;
            while j < 7 && v_corr > V_COND_ESSAIS_M_S[j] {
                j += 1;
            }
            let c_extr = c.c_extr[j - 1]
                + (c.c_extr[j] - c.c_extr[j - 1])
                    / (V_COND_ESSAIS_M_S[j] - V_COND_ESSAIS_M_S[j - 1])
                    * (v_corr - V_COND_ESSAIS_M_S[j - 1]);
            return 0.5 * cond.rho_ext_kg_m3 * c_extr * vent * vent;
        }
        if v_m_s <= l.v_lim2 {
            // Éq. (696) : transition entre les deux modèles.
            return (v_m_s - l.v_lim1) / (l.v_lim2 - l.v_lim1) * (l.d_p_lim2 - l.d_p_lim1)
                + l.d_p_lim1;
        }
    }
    // Éq. (697) : modèle sans vent.
    0.5 * rho_moy * c.xi_extr * v_m_s * v_m_s
}

/// « dPmot » : tirage thermique, en Pa.
///
/// Th-BCE 2020 §6.6.3.1.5, éq. (698) : `hmot × g × (ρext(h) − ρint,fin(h−1)) + Pib(h−1)`.
pub fn d_p_mot_pa(c: &ConduitResolu, cond: &ConditionsConduit) -> f64 {
    c.h_mot_m * G_M_S2 * (cond.rho_ext_kg_m3 - cond.rho_int_prec_kg_m3) + cond.p_ib_prec_pa
}

/// « hmot » par défaut, en m.
///
/// Th-BCE 2020 §6.6.3.1.5, éq. (699) et (700) : `hcond + 2` sans transferts d'air verticaux
/// (`δpermea_verticale = 0`), `hzone + 2` avec.
pub fn h_mot_defaut_m(permea_verticale: bool, h_cond_m: f64, h_zone_m: f64) -> f64 {
    if permea_verticale {
        h_zone_m + DECALAGE_TIRAGE_M
    } else {
        h_cond_m + DECALAGE_TIRAGE_M
    }
}

/// « F(qv,cond) » : bilan des pressions du conduit.
///
/// Th-BCE 2020 §6.6.3.1, éq. (673) : `dPextr + dPcond + dPcoude + dPbouche − dPmot`.
pub fn bilan_pressions_pa(c: &ConduitResolu, cond: &ConditionsConduit, q_v_cond_m3_h: f64) -> f64 {
    let v = vitesse_conduit_m_s(q_v_cond_m3_h, c.per_cond_m);
    let rho_int = cond.rho_int_prec_kg_m3;
    let d_p_bouche = d_p_bouche_pa(c, q_v_cond_m3_h);
    d_p_extracteur_pa(c, cond, v)
        + d_p_conduit_pa(c, rho_int, v)
        + d_p_coude_pa(c, rho_int, v)
        + d_p_bouche
        - d_p_mot_pa(c, cond)
}

/// « dPbouche » au débit de conduit `q_v_cond_m3_h`, en Pa (éq. 677 ou 679 à 682).
pub fn d_p_bouche_pa(c: &ConduitResolu, q_v_cond_m3_h: f64) -> f64 {
    match &c.bouche {
        BoucheNaturelle::Fixe { .. } => d_p_bouche_fixe_pa(c, q_v_cond_m3_h),
        BoucheNaturelle::Autoreglable { d_p_pa, q_v_m3_h } => {
            d_p_bouche_autoreglable_pa(c, d_p_pa, q_v_m3_h, q_v_cond_m3_h)
        }
    }
}

/// « qv,cond(h) » : débit d'un conduit, en m³/h, positif en extraction, négatif en refoulement.
///
/// Th-BCE 2020 §6.6.3.1, éq. (673) « résolue numériquement » : encadrement de la racine de
/// `F` puis dichotomie (la fiche ne prescrit pas de méthode ; Q234 point 3). `F` croît avec le
/// débit pour des pertes de charge croissantes, et les termes quadratiques l'emportent aux grands
/// débits, si bien que le signe de `F` s'inverse sur l'encadrement.
pub fn debit_conduit_m3_h(c: &ConduitResolu, cond: &ConditionsConduit) -> f64 {
    let f = |q: f64| bilan_pressions_pa(c, cond, q);
    let mut borne = 100.0;
    let mut n = 0;
    while !(f(-borne) <= 0.0 && f(borne) >= 0.0) && n < 40 {
        borne *= 2.0;
        n += 1;
    }
    let (mut bas, mut haut) = (-borne, borne);
    for _ in 0..200 {
        let milieu = 0.5 * (bas + haut);
        if f(milieu) < 0.0 {
            bas = milieu;
        } else {
            haut = milieu;
        }
    }
    0.5 * (bas + haut)
}

/// Débits repris et soufflés d'un système de ventilation naturelle, en m³/h, signés comme dans la
/// méthode (repris négatif, soufflé positif) : « qrep,dep », « qrep,fuites », « qsouf,dep »,
/// « qsouf,fuites » du groupe.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DebitsNaturelle {
    /// « qrep,fuites(h) ».
    pub q_rep_fuites_m3_h: f64,
    /// « qrep,dep(h) ».
    pub q_rep_dep_m3_h: f64,
    /// « qsouf,fuites(h) ».
    pub q_souf_fuites_m3_h: f64,
    /// « qsouf,dep(h) ».
    pub q_souf_dep_m3_h: f64,
}

/// Débits repris ou refoulés pour le débit de conduit `q_v_cond_m3_h` et la perte de charge de la
/// bouche `d_p_bouche_pa`.
///
/// Th-BCE 2020 §6.6.3.2.1, éq. (701) à (705), `Crdbnr = 1`.
pub fn debits_naturelle(
    c: &ConduitResolu,
    q_v_cond_m3_h: f64,
    d_p_bouche_pa: f64,
) -> DebitsNaturelle {
    let part = part_fuites(c, d_p_bouche_pa);
    let total = c.nid * q_v_cond_m3_h;
    if q_v_cond_m3_h >= 0.0 {
        // Éq. (701) et (702), puis (703).
        let fuites = -part * total;
        DebitsNaturelle {
            q_rep_fuites_m3_h: fuites,
            q_rep_dep_m3_h: -c.cdep * CRDBNR * (total + fuites),
            ..DebitsNaturelle::default()
        }
    } else {
        // Éq. (704), puis (705).
        let fuites = -part * total;
        DebitsNaturelle {
            q_souf_fuites_m3_h: fuites,
            q_souf_dep_m3_h: -c.cdep * CRDBNR * (total + fuites),
            ..DebitsNaturelle::default()
        }
    }
}

impl DebitsNaturelle {
    /// « qv,spec,rep(h) » : débit repris spécifique du groupe, en m³/h (négatif).
    ///
    /// Th-BCE 2020 §6.6.3.2.3, éq. (707) : `qrep,dep + Ratfuitevc × qrep,fuites`.
    pub fn q_spec_rep_m3_h(&self, ratfuitevc: f64) -> f64 {
        self.q_rep_dep_m3_h + ratfuitevc * self.q_rep_fuites_m3_h
    }

    /// « qv,spec,souf(h) » : débit spécifique soufflé en refoulement, en m³/h (positif).
    ///
    /// Th-BCE 2020 §6.6.3.2.3 : la fiche ne donne d'équation que pour le débit repris (éq. 707) ;
    /// le débit soufflé s'en déduit de même, `qsouf,dep + Ratfuitevc × qsouf,fuites` (Q235
    /// point 1).
    pub fn q_spec_souf_m3_h(&self, ratfuitevc: f64) -> f64 {
        self.q_souf_dep_m3_h + ratfuitevc * self.q_souf_fuites_m3_h
    }
}

/// Créneau de grand débit de la cuisine : vrai si l'heure légale `h_leg` (début du pas, de 0 à
/// 23) est dans un des deux créneaux du tableau 67.
///
/// Th-BCE 2020 §6.6.3.2.2, tableau 67 : `Dugd = 0` : non appliqué ; sinon maison individuelle
/// ou accolée 11 h à 12 h et 18 h à 19 h, logement collectif 11 h à 13 h et 18 h à 20 h. Un pas
/// de début `h_leg` est dans `[deb ; fin[` (Q236 point 2).
pub fn est_grand_debit(dugd_h: f64, logement_collectif: bool, h_leg: u8) -> bool {
    if dugd_h <= 0.0 {
        return false;
    }
    let fin = if logement_collectif { 2 } else { 1 };
    (11..11 + fin).contains(&h_leg) || (18..18 + fin).contains(&h_leg)
}

/// Condition de l'assistance mécanique hors créneau de grand débit : vent inférieur et
/// température extérieure supérieure aux consignes.
///
/// Th-BCE 2020 §6.6.3.2.2, éq. (706) et (708).
pub fn besoin_assistance(a: &AssistanceMecanique, v_vent_c_m_s: f64, theta_ext_c: f64) -> bool {
    v_vent_c_m_s < a.v_vent_lim_m_s && theta_ext_c > a.theta_ext_lim_c
}

/// Débits de l'assistance mécanique : débit repris de base ou de pointe d'un simple flux standard
/// (`q_rep_dep_m3_h`, déjà pondéré par `Dugd`, `Cdep` et `Crdbnr`) et fuites sous `dPref`.
///
/// Th-BCE 2020 §6.6.3.2.2, éq. (706) : `qrep,fuites = −Nid × 3600 × Kres × Acond × dPref^0,667` ;
/// débits soufflés nuls.
pub fn debits_assistance(c: &ConduitResolu, q_rep_dep_standard_m3_h: f64) -> DebitsNaturelle {
    DebitsNaturelle {
        q_rep_dep_m3_h: q_rep_dep_standard_m3_h,
        q_rep_fuites_m3_h: -c.nid * q_fuites_dp_m3_h(c.k_res, c.a_cond_m2, c.d_pref_pa),
        ..DebitsNaturelle::default()
    }
}

/// « Cvent(h) » : consommation du ventilateur d'assistance sur le pas d'une heure, en Wh.
///
/// Th-BCE 2020 §6.6.3.3, éq. (708) : `Pvent,pointe` en grand débit ; sinon, si le vent est
/// inférieur à `vvent,c,lim,hyb` et la température supérieure à `θext,lim,hyb`, `Pvent,k,l` avec
/// `k` et `l` repérés par les tant-que de l'algorithme ; sinon 0.
pub fn c_vent_wh(
    a: &AssistanceMecanique,
    grand_debit: bool,
    v_vent_c_m_s: f64,
    theta_ext_c: f64,
) -> f64 {
    if grand_debit {
        return a.p_vent_pointe_w;
    }
    if !besoin_assistance(a, v_vent_c_m_s, theta_ext_c) {
        return 0.0;
    }
    let k_max = a.v_vent_reg_m_s.len();
    let l_max = a.theta_ext_reg_c.len();
    // Indices de 0 à `max − 1` ; l'algorithme compte de 1.
    let mut k = 0;
    while k + 1 < k_max && v_vent_c_m_s < a.v_vent_reg_m_s[k + 1] {
        k += 1;
    }
    let mut l = 0;
    while l + 1 < l_max && theta_ext_c > a.theta_ext_reg_c[l + 1] {
        l += 1;
    }
    a.p_vent_w
        .get(k)
        .and_then(|ligne| ligne.get(l))
        .copied()
        .unwrap_or(0.0)
}

/// « Textr1(h) » : température de l'air repris après les pertes du conduit, en °C.
///
/// Th-BCE 2020 §6.6.3.4.1, éq. (709) à (712) : `θi,g,fin(h−1) − (θi,g,fin(h−1) − θet) ×
/// (1 − exp(−Hcond,rep,ext / (0,34 × |qv,cond|)))` avec `θet = (1 − b) × θi + b × θext`,
/// `Hcond,rep,ext = (1 − Ratfuitevc) × Acond / (Rrep + 0,02)`. Un débit nul laisse la
/// température du groupe.
pub fn t_extr1_c(c: &ConduitResolu, cond: &ConditionsConduit, q_v_cond_m3_h: f64) -> f64 {
    let theta_i = cond.theta_int_prec_c;
    let theta_et = (1.0 - c.b) * theta_i + c.b * cond.theta_ext_c;
    let a_ext = (1.0 - c.ratfuitevc) * c.a_cond_m2;
    let h = a_ext / (c.r_rep_m2k_w + 0.02);
    let q = q_v_cond_m3_h.abs();
    if q <= 0.0 {
        return theta_i;
    }
    theta_i - (theta_i - theta_et) * (1.0 - libm::exp(-h / (0.34 * q)))
}

/// Cumuls annuels d'un système de ventilation naturelle ou hybride.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CumulsNaturelle {
    /// Nombre d'heures d'extraction naturelle (`qv,cond > 0`).
    pub nbh_extraction: u32,
    /// Nombre d'heures de refoulement (`qv,cond < 0`).
    pub nbh_refoulement: u32,
    /// Nombre d'heures de fonctionnement de l'assistance mécanique (éq. 706).
    pub nbh_assistance: u32,
    /// Nombre d'heures de grand débit de la cuisine (tableau 67).
    pub nbh_grand_debit: u32,
    /// Consommation du ventilateur d'assistance, en Wh (éq. 708).
    pub w_vent_wh: f64,
    /// Somme des débits spécifiques repris, en m³/h (négatifs).
    pub somme_q_rep_m3_h: f64,
    /// Somme des débits spécifiques soufflés, en m³/h.
    pub somme_q_souf_m3_h: f64,
    /// Somme des débits repris vus par le conduit (fuites comprises), en m³/h (négatifs).
    pub somme_q_rep_cond_m3_h: f64,
    /// Somme des débits soufflés vus par le conduit (fuites comprises), en m³/h.
    pub somme_q_souf_cond_m3_h: f64,
    /// Nombre de pas de temps calculés.
    pub nb_pas: u32,
}

/// Température, masse volumique et pression de l'ambiance et du vent au pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionsPas {
    /// Conditions de l'extérieur et de l'intérieur du pas précédent.
    pub conduit: ConditionsConduit,
    /// Heure légale du début du pas, de 0 à 23.
    pub h_leg: u8,
}

/// État d'un système de ventilation naturelle ou hybride au cours de la simulation d'une zone.
#[derive(Debug, Clone, PartialEq)]
pub struct EtatNaturelle {
    /// Conduit équivalent.
    pub conduit: ConduitResolu,
    /// Assistance mécanique ; `None` en ventilation naturelle seule.
    pub assistance: Option<AssistanceMecanique>,
    /// Indice du groupe desservi.
    pub groupe: usize,
    /// Logement collectif (tableau 67) ; sinon maison individuelle ou accolée.
    pub logement_collectif: bool,
    /// Débit repris des bouches de reprise du groupe comme d'un simple flux standard, en m³/h
    /// (négatif) : `Cdep × Crdbnr × (qpointe × Dugd + qbase × (168 − Dugd)) / 168` de l'éq. (706).
    pub q_rep_dep_standard_m3_h: f64,
    /// « qv,cond(h) » du pas, en m³/h.
    pub q_v_cond_m3_h: f64,
    /// Débits spécifiques du groupe au pas (éq. 707), repris négatif et soufflé positif.
    pub debits: Debits,
    /// Débits vus par le conduit au pas (fuites comprises).
    pub debits_cond: Debits,
    /// L'assistance mécanique est en service au pas.
    pub assiste: bool,
    /// Grand débit de la cuisine au pas.
    pub grand_debit: bool,
    /// « Cvent(h) » du pas, en Wh.
    pub c_vent_wh: f64,
    /// « Textr1(h) » du pas, en °C (éq. 709 ; sans autre usage que la restitution : l'air extrait
    /// d'un système sans récupération n'est pas réutilisé).
    pub t_extr1_c: f64,
    /// Cumuls annuels.
    pub cumuls: CumulsNaturelle,
}

impl EtatNaturelle {
    /// Système avant le premier pas de temps.
    pub fn nouveau(
        conduit: ConduitResolu,
        assistance: Option<AssistanceMecanique>,
        groupe: usize,
        logement_collectif: bool,
        q_rep_dep_standard_m3_h: f64,
    ) -> Self {
        Self {
            conduit,
            assistance,
            groupe,
            logement_collectif,
            q_rep_dep_standard_m3_h,
            q_v_cond_m3_h: 0.0,
            debits: Debits::default(),
            debits_cond: Debits::default(),
            assiste: false,
            grand_debit: false,
            c_vent_wh: 0.0,
            t_extr1_c: 0.0,
            cumuls: CumulsNaturelle::default(),
        }
    }

    /// Calcule les débits, la consommation et la température d'extraction du pas de temps
    /// (éq. 673 à 714) et les ajoute aux cumuls.
    pub fn pas(&mut self, x: &ConditionsPas) {
        let c = &self.conduit;
        let cond = &x.conduit;
        let grand_debit = self
            .assistance
            .as_ref()
            .is_some_and(|_| est_grand_debit(c.dugd_h, self.logement_collectif, x.h_leg));
        let assiste = self.assistance.as_ref().is_some_and(|a| {
            grand_debit || besoin_assistance(a, cond.v_vent_c_m_s, cond.theta_ext_c)
        });
        let (q_v_cond, d) = if assiste {
            // Éq. (706) : débit repris d'un simple flux standard et fuites sous dPref ; débit
            // vu par un conduit pour l'éq. (709).
            let d = debits_assistance(c, self.q_rep_dep_standard_m3_h);
            (-(d.q_rep_dep_m3_h + d.q_rep_fuites_m3_h) / c.nid, d)
        } else {
            // Éq. (673), puis (701) à (705).
            let q = debit_conduit_m3_h(c, cond);
            (q, debits_naturelle(c, q, d_p_bouche_pa(c, q)))
        };
        self.q_v_cond_m3_h = q_v_cond;
        self.assiste = assiste;
        self.grand_debit = grand_debit;
        self.debits = Debits {
            q_rep_m3_h: d.q_spec_rep_m3_h(c.ratfuitevc),
            q_souffle_m3_h: d.q_spec_souf_m3_h(c.ratfuitevc),
        };
        self.debits_cond = Debits {
            q_rep_m3_h: d.q_rep_dep_m3_h + d.q_rep_fuites_m3_h,
            q_souffle_m3_h: d.q_souf_dep_m3_h + d.q_souf_fuites_m3_h,
        };
        self.c_vent_wh = self.assistance.as_ref().map_or(0.0, |a| {
            c_vent_wh(a, grand_debit, cond.v_vent_c_m_s, cond.theta_ext_c)
        });
        self.t_extr1_c = t_extr1_c(c, cond, q_v_cond);
        let k = &mut self.cumuls;
        k.nbh_extraction += u32::from(q_v_cond > 0.0 && !assiste);
        k.nbh_refoulement += u32::from(q_v_cond < 0.0 && !assiste);
        k.nbh_assistance += u32::from(assiste);
        k.nbh_grand_debit += u32::from(grand_debit);
        k.w_vent_wh += self.c_vent_wh;
        k.somme_q_rep_m3_h += self.debits.q_rep_m3_h;
        k.somme_q_souf_m3_h += self.debits.q_souffle_m3_h;
        k.somme_q_rep_cond_m3_h += self.debits_cond.q_rep_m3_h;
        k.somme_q_souf_cond_m3_h += self.debits_cond.q_souffle_m3_h;
        k.nb_pas += 1;
    }

    /// Débits spécifiques moyens du groupe sur les pas calculés, en m³/h, pour la restitution
    /// (nuls avant le premier pas).
    pub fn debits_moyens(&self, g: usize) -> Debits {
        let n = f64::from(self.cumuls.nb_pas.max(1));
        if g != self.groupe || self.cumuls.nb_pas == 0 {
            return Debits::default();
        }
        Debits {
            q_rep_m3_h: self.cumuls.somme_q_rep_m3_h / n,
            q_souffle_m3_h: self.cumuls.somme_q_souf_m3_h / n,
        }
    }

    /// Débits moyens vus par le conduit (fuites comprises) sur les pas calculés, en m³/h.
    pub fn debits_cond_moyens(&self, g: usize) -> Debits {
        let n = f64::from(self.cumuls.nb_pas.max(1));
        if g != self.groupe || self.cumuls.nb_pas == 0 {
            return Debits::default();
        }
        Debits {
            q_rep_m3_h: self.cumuls.somme_q_rep_cond_m3_h / n,
            q_souffle_m3_h: self.cumuls.somme_q_souf_cond_m3_h / n,
        }
    }

    /// Puissance moyenne du ventilateur d'assistance sur les pas calculés, en W.
    pub fn puissance_moyenne_w(&self) -> f64 {
        if self.cumuls.nb_pas == 0 {
            0.0
        } else {
            self.cumuls.w_vent_wh / f64::from(self.cumuls.nb_pas)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proche(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} != {b} (tolérance {tol})");
    }

    /// Conduit de 0,4 m de périmètre, bouche fixe de 20 cm², sans perte de charge hors bouche ni
    /// fuites, pour les cas où le débit se calcule à la main.
    fn conduit() -> ConduitResolu {
        ConduitResolu {
            nid: 1.0,
            per_cond_m: 0.4,
            h_cond_m: 4.0,
            h_mot_m: 6.0,
            a_cond_m2: 5.0,
            lambda_cond: 0.0,
            chi_coude: 0.0,
            d_pref_pa: 20.0,
            bouche: BoucheNaturelle::Fixe {
                a_sect_base_cm2: 20.0,
                a_sect_pointe_cm2: 20.0,
            },
            a_bouche_eq_m2: 0.002,
            c_extr: [-0.2, -0.1, 0.0, 0.1, 0.2, 0.4, 0.6, 1.0],
            xi_extr: 0.0,
            v_vent_ref_m_s: 8.0,
            k_res: 0.0,
            ratfuitevc: 0.25,
            r_rep_m2k_w: 1.0,
            b: 0.5,
            cdep: 1.3,
            dugd_h: 14.0,
        }
    }

    fn conditions() -> ConditionsConduit {
        ConditionsConduit {
            theta_ext_c: 0.0,
            rho_ext_kg_m3: 1.2,
            v_vent_c_m_s: 0.0,
            theta_int_prec_c: 20.0,
            rho_int_prec_kg_m3: 1.2,
            p_ib_prec_pa: 0.0,
        }
    }

    #[test]
    fn geometrie_et_bouche_fixe_a_la_main() {
        // Éq. (674) : (14 × 40 + 154 × 20) / 168 × 1e-4.
        proche(a_bouche_eq_m2(14.0, 40.0, 20.0), 0.0021666667, 1e-9);
        // Éq. (683), (684), (685) : Per = 0,4 m.
        proche(
            section_conduit_m2(0.4),
            0.16 / (4.0 * core::f64::consts::PI),
            1e-12,
        );
        proche(diametre_conduit_m(0.4), 0.4 / core::f64::consts::PI, 1e-12);
        proche(vitesse_conduit_m_s(100.0, 0.4), 2.1816616, 1e-6);
        // Éq. (676) : 3600 × 0,68 × 0,002 × (2/1,2 × 20)^0,5 = 28,26707 m³/h.
        proche(q_bouche_dp_m3_h(0.002, 20.0), 28.267069, 1e-5);
        // Éq. (677) sans fuites : dP = dPref à ce débit, dPref/4 à la moitié, symétrique.
        let c = conduit();
        proche(d_p_bouche_fixe_pa(&c, 28.267069), 20.0, 1e-5);
        proche(d_p_bouche_fixe_pa(&c, 14.1335346), 5.0, 1e-5);
        proche(d_p_bouche_fixe_pa(&c, -14.1335346), -5.0, 1e-5);
        // Éq. (675) : 3600 × 1e-4 × 5 × 20^0,667 = 13,27576 m³/h.
        proche(q_fuites_dp_m3_h(1e-4, 5.0, 20.0), 13.275764, 1e-5);
        // Éq. (677) avec fuites : (28,26707 + 13,27576) = 41,54283 m³/h donne dPref.
        let mut f = conduit();
        f.k_res = 1e-4;
        proche(d_p_bouche_fixe_pa(&f, 41.542833), 20.0, 1e-5);
    }

    #[test]
    fn bouche_autoreglable_a_la_main() {
        let d_p = [10.0, 20.0, 30.0, 50.0, 100.0, 150.0];
        let q_v = [20.0, 30.0, 40.0, 50.0, 60.0, 70.0];
        let mut c = conduit();
        c.bouche = BoucheNaturelle::Autoreglable {
            d_p_pa: d_p,
            q_v_m3_h: q_v,
        };
        // Éq. (680) : refoulement, −dP1 × (q/qv1)².
        proche(d_p_bouche_pa(&c, -10.0), -2.5, 1e-12);
        // Éq. (681) : 10 × (15/20)².
        proche(d_p_bouche_pa(&c, 15.0), 5.625, 1e-12);
        // Éq. (682) : entre (20 ; 10) et (30 ; 20) à 25 m³/h, puis entre (40 ; 30) et (50 ; 50).
        proche(d_p_bouche_pa(&c, 25.0), 15.0, 1e-12);
        proche(d_p_bouche_pa(&c, 45.0), 40.0, 1e-12);
        // Éq. (679) : au-delà du dernier point, 150 × (80/70)².
        proche(d_p_bouche_pa(&c, 80.0), 195.918367, 1e-5);
        // Avec fuites (Kres = 1e-4, Acond = 5 : 8,3613 ; 13,2758 ; 17,3985 ; 24,4616 m³/h aux
        // quatre premiers points), à 60 m³/h le premier point dont débit et fuites dépassent le
        // débit du conduit est le quatrième (74,4616) : interpolation entre le troisième
        // (57,3985) et lui.
        c.k_res = 1e-4;
        proche(d_p_bouche_pa(&c, 60.0), 33.049241, 1e-5);
        // Éq. (701), lecture autoréglable : part des fuites sous 25 Pa, bouche à 35 m³/h par la
        // courbe (point 0 compris), fuites 1,8 × 25^0,667 = 15,4063 m³/h.
        proche(part_fuites(&c, 25.0), 0.305642, 1e-5);
    }

    #[test]
    fn debit_du_conduit_a_la_main() {
        // Sans perte de charge hors bouche ni fuites : dPbouche = dPmot, donc
        // q = qbouche,dPref × (dPmot/dPref)^0,5 ; dPmot = 6 × 9,81 × (1,25 − 1,2) = 2,943 Pa.
        let c = conduit();
        let mut x = conditions();
        x.rho_ext_kg_m3 = 1.25;
        proche(d_p_mot_pa(&c, &x), 2.943, 1e-12);
        proche(debit_conduit_m3_h(&c, &x), 10.843286, 1e-6);
        // Air extérieur plus léger que l'air intérieur : refoulement de même valeur absolue.
        x.rho_ext_kg_m3 = 1.15;
        proche(debit_conduit_m3_h(&c, &x), -10.843286, 1e-6);
        // Même tirage avec une pression intérieure Pib(h−1) de −2,943 Pa : la zone en dépression
        // augmente le débit (éq. 698 : dPmot = tirage + Pib, ici tirage 2,943 − 2,943 = 0).
        x.rho_ext_kg_m3 = 1.25;
        x.p_ib_prec_pa = -2.943;
        proche(debit_conduit_m3_h(&c, &x), 0.0, 1e-6);
        // Avec fuites : q = (qb + qf) × (dPmot/dPref)^0,5 = 41,542833 × 0,383598.
        let mut f = conduit();
        f.k_res = 1e-4;
        let mut x = conditions();
        x.rho_ext_kg_m3 = 1.25;
        proche(debit_conduit_m3_h(&f, &x), 15.935887, 1e-6);
        // Le bilan s'annule à la racine, avec toutes les pertes de charge et du vent.
        let mut g = conduit();
        g.lambda_cond = 0.05;
        g.chi_coude = 1.15;
        g.xi_extr = 2.0;
        let mut y = conditions();
        y.rho_ext_kg_m3 = 1.25;
        y.v_vent_c_m_s = 3.0;
        let q = debit_conduit_m3_h(&g, &y);
        assert!(q > 0.0);
        proche(bilan_pressions_pa(&g, &y, q), 0.0, 1e-9);
    }

    #[test]
    fn extracteur_statique_a_la_main() {
        let mut c = conduit();
        c.xi_extr = 2.0;
        let mut x = conditions();
        // Sans vent, éq. (697) : 0,5 × 1,2 × 2 × 1,5² = 2,7 Pa.
        proche(d_p_extracteur_pa(&c, &x, 1.5), 2.7, 1e-12);
        // Refoulement, éq. (693) : Cextr,1 = −0,2, vent de 4 m/s : 0,5 × 1,2 × (−0,2) × 16.
        x.v_vent_c_m_s = 4.0;
        proche(d_p_extracteur_pa(&c, &x, -1.0), -1.92, 1e-12);
        // Courbe, éq. (694) et (695) : vitesse corrigée 8/4 = 2 → Cextr = 0,2 à v = 1 m/s ;
        // 0,5 × 1,2 × 0,2 × 16 = 1,92 Pa.
        proche(d_p_extracteur_pa(&c, &x, 1.0), 1.92, 1e-12);
        // Interpolation entre 0,5 et 1 m/s corrigés : 0,375 m/s → 0,75 → Cextr = −0,05.
        proche(d_p_extracteur_pa(&c, &x, 0.375), -0.48, 1e-12);
        // Limites : vcond,s_lim1 = 8 × 4/8 = 4 m/s, dPextr_lim1 = 0,5 × 1,2 × 1 × 16 = 9,6 Pa ;
        // n = 2 déjà (0,5 × 1,2 × 2 × 8² = 76,8 ≥ 9,6) : lim2 = 8 m/s, 76,8 Pa.
        proche(d_p_extracteur_pa(&c, &x, 4.0), 9.6, 1e-12);
        // Transition, éq. (696) : à 6 m/s, mi-chemin : 9,6 + 0,5 × (76,8 − 9,6) = 43,2 Pa.
        proche(d_p_extracteur_pa(&c, &x, 6.0), 43.2, 1e-12);
        // Au-delà, éq. (697) : 0,5 × 1,2 × 2 × 10² = 120 Pa.
        proche(d_p_extracteur_pa(&c, &x, 10.0), 120.0, 1e-12);
        // Sans vent et refoulement : dépression nulle.
        x.v_vent_c_m_s = 0.0;
        proche(d_p_extracteur_pa(&c, &x, -1.0), 0.0, 1e-12);
    }

    #[test]
    fn pertes_de_charge_du_conduit_a_la_main() {
        let mut c = conduit();
        c.lambda_cond = 0.05;
        c.chi_coude = 1.15;
        // Per = 0,4 m, D = 0,127324 m, hcond = 4 m, v = 2 m/s, ρ = 1,2 :
        // éq. (686) : 0,5 × 0,05 × (4/0,127324) × 1,2 × 4 = 3,769911 Pa ;
        // éq. (687) : 0,5 × 1,15 × 1,2 × 4 = 2,76 Pa.
        proche(d_p_conduit_pa(&c, 1.2, 2.0), 3.769911, 1e-6);
        proche(d_p_coude_pa(&c, 1.2, 2.0), 2.76, 1e-12);
        proche(d_p_coude_pa(&c, 1.2, -2.0), -2.76, 1e-12);
        // Éq. (699) et (700).
        proche(h_mot_defaut_m(false, 4.0, 5.0), 6.0, 1e-12);
        proche(h_mot_defaut_m(true, 4.0, 5.0), 7.0, 1e-12);
    }

    #[test]
    fn debits_repris_et_refoules_a_la_main() {
        let mut c = conduit();
        c.k_res = 1e-4;
        c.nid = 2.0;
        // dPbouche = 2 Pa : part des fuites = f/(b + f), f = 1e-4 × 5 × 2^0,167 = 5,6137e-4,
        // b = 0,68 × 0,002 × (2/1,2)^0,5 = 1,755752e-3 : 0,242267.
        proche(part_fuites(&c, 2.0), 0.242267, 1e-6);
        // Extraction, 20 m³/h par conduit, Nid = 2, Cdep = 1,3 : éq. (701) et (702).
        let d = debits_naturelle(&c, 20.0, 2.0);
        proche(d.q_rep_fuites_m3_h, -9.690691, 1e-5);
        proche(d.q_rep_dep_m3_h, -39.402101, 1e-5);
        assert_eq!((d.q_souf_fuites_m3_h, d.q_souf_dep_m3_h), (0.0, 0.0));
        // Éq. (707) : −39,402101 + 0,25 × (−9,690691) = −41,824774.
        proche(d.q_spec_rep_m3_h(0.25), -41.824774, 1e-5);
        // Refoulement, éq. (704) et (705) : signes opposés, débits repris nuls.
        let r = debits_naturelle(&c, -20.0, -2.0);
        proche(r.q_souf_fuites_m3_h, 9.690691, 1e-5);
        proche(r.q_souf_dep_m3_h, 39.402101, 1e-5);
        assert_eq!((r.q_rep_fuites_m3_h, r.q_rep_dep_m3_h), (0.0, 0.0));
        proche(r.q_spec_souf_m3_h(0.25), 41.824774, 1e-5);
    }

    #[test]
    fn assistance_mecanique_a_la_main() {
        let a = AssistanceMecanique {
            theta_ext_lim_c: 12.0,
            v_vent_lim_m_s: 4.0,
            v_vent_reg_m_s: vec![3.0, 2.0, 1.0],
            theta_ext_reg_c: vec![16.0, 20.0],
            p_vent_w: vec![vec![10.0, 11.0], vec![20.0, 21.0], vec![30.0, 31.0]],
            p_vent_pointe_w: 80.0,
        };
        // Tableau 67 : maison, 11 h et 18 h ; logement collectif, 11 h et 12 h, 18 h et 19 h ;
        // sans grand débit (Dugd = 0), jamais.
        assert!(est_grand_debit(14.0, false, 11));
        assert!(!est_grand_debit(14.0, false, 12));
        assert!(est_grand_debit(14.0, false, 18));
        assert!(est_grand_debit(14.0, true, 12));
        assert!(est_grand_debit(14.0, true, 19));
        assert!(!est_grand_debit(14.0, true, 20));
        assert!(!est_grand_debit(0.0, true, 11));
        // Éq. (706) : assistance si vent < 4 m/s et θext > 12 °C.
        assert!(besoin_assistance(&a, 3.9, 12.5));
        assert!(!besoin_assistance(&a, 4.0, 12.5));
        assert!(!besoin_assistance(&a, 3.0, 12.0));
        // Éq. (708) : grand débit, puis matrice, puis rien.
        proche(c_vent_wh(&a, true, 10.0, -5.0), 80.0, 0.0);
        proche(c_vent_wh(&a, false, 2.5, 18.0), 10.0, 0.0);
        proche(c_vent_wh(&a, false, 1.5, 21.0), 21.0, 0.0);
        proche(c_vent_wh(&a, false, 0.5, 21.0), 31.0, 0.0);
        proche(c_vent_wh(&a, false, 0.5, 18.0), 30.0, 0.0);
        proche(c_vent_wh(&a, false, 5.0, 18.0), 0.0, 0.0);
        // Éq. (706) : fuites sous dPref, −2 × 3600 × 1e-4 × 5 × 20^0,667 = −26,551527.
        let mut c = conduit();
        c.k_res = 1e-4;
        c.nid = 2.0;
        let d = debits_assistance(&c, -50.0);
        proche(d.q_rep_fuites_m3_h, -26.551527, 1e-5);
        proche(d.q_spec_rep_m3_h(0.25), -50.0 - 0.25 * 26.551527, 1e-5);
    }

    #[test]
    fn temperature_apres_les_pertes_du_conduit_a_la_main() {
        // Éq. (709) à (712) : θi = 20 °C, θext = 0, b = 0,5 → θet = 10 °C ; Aext = 0,75 × 8 = 6 m²,
        // H = 6/(1 + 0,02) = 5,882353 W/K ; q = 50 m³/h : exp(−5,882353/17) = 0,707447 ;
        // 20 − 10 × (1 − 0,707447) = 17,07498 °C.
        let c = conduit();
        let mut c8 = c.clone();
        c8.a_cond_m2 = 8.0;
        proche(t_extr1_c(&c8, &conditions(), 50.0), 17.07498, 1e-5);
        proche(t_extr1_c(&c8, &conditions(), -50.0), 17.07498, 1e-5);
        proche(t_extr1_c(&c8, &conditions(), 0.0), 20.0, 0.0);
    }
}
