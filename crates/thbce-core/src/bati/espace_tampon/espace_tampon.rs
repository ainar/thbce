// SPDX-License-Identifier: GPL-3.0-or-later
//! Espaces tampons : fiche `Espaces tampons` (Th-BCE 2020 §5.4, A3 p. 118 à 127).
//!
//! Un espace tampon est un local non chauffé en contact avec l'enveloppe d'un ou de plusieurs
//! groupes. Il est caractérisé par trois coefficients de réduction, `btherm(h)`, `bsolaire(h)`
//! et `becl(h)`, appliqués aux parois opaques, ponts thermiques et baies qui le séparent du
//! volume chauffé :
//! - espace **non solarisé** (garage, comble, cellier) : `btherm` est le coefficient `b` des
//!   règles Th-Bat, corrigé des rejets des générateurs thermodynamiques d'ECS qui y puisent
//!   leur chaleur (éq. 109 à 111) ; `bsolaire = becl = 0` (éq. 112 et 113) ;
//! - espace **solarisé** (véranda, serre, jardin d'hiver) : modèle dynamique à une capacité
//!   (réseau RC, éq. 115 à 125), d'où `btherm` (éq. 126) ; `bsolaire` et `becl` dépendent de
//!   la position des protections mobiles de ses parois (éq. 127 à 129).

/// « ηapp_sol_et » : rendement de récupération des apports solaires par l'espace tampon
/// solarisé (tableau 19, constantes, p. 122).
pub const ETA_APP_SOL_ET: f64 = 0.8;
/// « ηatt_Sw » : facteur correctif d'atténuation du calcul de `bsolaire` (tableau 19).
pub const ETA_ATT_SW: f64 = 0.9;
/// « ηatt_Tl » : facteur correctif d'atténuation du calcul de `becl` (tableau 19).
pub const ETA_ATT_TL: f64 = 0.9;
/// « Cv_air » : capacité thermique volumique de l'air, en Wh/(m³.K) (tableau 19). C'est le
/// « 0.34 » des éq. (123) et (124).
pub const CV_AIR_WH_M3K: f64 = 0.34;
/// « KkJ_Wh » : passage du kJ au Wh (tableau 19 : 1/3,6).
pub const K_KJ_WH: f64 = 1.0 / 3.6;
/// « Cw » : constante de prise en compte du vent dans les débits par les ouvertures
/// (tableau 19).
pub const C_W: f64 = 0.001;
/// « Cst » : constante de prise en compte du tirage thermique, en m/(°C.s²) (tableau 19).
pub const C_ST: f64 = 0.0035;
/// « Cm_et_surf » conventionnelle : capacité thermique quotidienne de l'espace tampon par
/// unité de surface au sol, en kJ/(m².K) (tableau 19, colonne « Conv. », p. 120 : 200).
pub const CM_ET_SURF_CONV_KJ_M2K: f64 = 200.0;
/// « Qv,et,base » par défaut, en m³/h par m² de surface au sol (§5.4.3.3.1.4 : « Par défaut,
/// on retiendra 1.0 m3/h par m2 de surface au sol de l'espace tampon »).
pub const QV_ET_BASE_DEFAUT_M3_H_M2: f64 = 1.0;
/// Valeur basse de `btherm` (§5.4.3.3.1.6, éq. 126 : « Si btherm(h) < −15, alors
/// btherm(h) = −15 »).
pub const B_THERM_MIN: f64 = -15.0;
/// Valeur initiale de « θet(h−1) », en °C (tableau 19 : « Valeur initiale = 10°C »).
pub const THETA_ET_INITIALE_C: f64 = 10.0;
/// Valeur initiale de « θi,fin^gr(h−1) » vue par l'espace tampon, en °C (tableau 19 : « Valeur
/// initiale = 19°C »).
pub const THETA_I_FIN_INITIALE_ET_C: f64 = 19.0;

/// « Het_th_int^gr » : coefficient de transmission d'un groupe vers l'espace tampon, en W/K.
///
/// Th-BCE 2020 §5.4.3.1, éq. (106) : `Σ Hges_set^b + Σ HTH_set^k + Σ HTH_set^l` sur les baies,
/// parois opaques et ponts thermiques du groupe en contact avec l'espace tampon, coefficients
/// « sans les espaces tampons ». Calculé en début de simulation.
pub fn h_et_th_int_gr_w_k(
    hges_set_w_k: impl IntoIterator<Item = f64>,
    hth_set_parois_w_k: impl IntoIterator<Item = f64>,
    hth_set_ponts_w_k: impl IntoIterator<Item = f64>,
) -> f64 {
    hges_set_w_k.into_iter().sum::<f64>()
        + hth_set_parois_w_k.into_iter().sum::<f64>()
        + hth_set_ponts_w_k.into_iter().sum::<f64>()
}

/// « θi_eq(h) » : température intérieure équivalente vue par l'espace tampon, en °C.
///
/// Th-BCE 2020 §5.4.3.1, éq. (107) et (108) : moyenne des `θi,fin^gr(h−1)` pondérée par les
/// `Het_th_int^gr`, dont la somme est `Het_th_int_eq`. `groupes` donne
/// `(Het_th_int^gr, θi,fin^gr(h−1))`. `None` si `Het_th_int_eq` est nul.
pub fn theta_i_eq_c(groupes: impl IntoIterator<Item = (f64, f64)>) -> Option<f64> {
    let (mut h, mut h_theta) = (0.0, 0.0);
    for (h_gr, theta) in groupes {
        h += h_gr;
        h_theta += h_gr * theta;
    }
    (h > 0.0).then(|| h_theta / h)
}

/// Coefficients de réduction et température d'un espace tampon à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoefficientsEt {
    /// « btherm(h) » : réduction des déperditions.
    pub b_therm: f64,
    /// « bsolaire(h) » : réduction des flux solaires.
    pub b_solaire: f64,
    /// « becl(h) » : réduction de l'éclairement (baies seulement).
    pub b_ecl: f64,
    /// « θet(h) » : température de l'air de l'espace tampon, en °C.
    pub theta_et_c: f64,
    /// « Aouv_et(h) » : surface d'ouverture des parois de l'espace tampon, en m² (nulle pour
    /// un espace non solarisé).
    pub a_ouv_et_m2: f64,
}

impl CoefficientsEt {
    /// Vrai si l'espace tampon est surventilé par ouverture de ses parois : le coefficient
    /// `btherm` des baies qui donnent sur lui vaut alors 1 (§5.8, « Description de
    /// l'assemblage », p. 178).
    pub fn surventile(&self) -> bool {
        self.a_ouv_et_m2 > 0.0
    }
}

/// « Ccorr_therm(h) » : correction du `b` d'un espace tampon non solarisé pour les rejets
/// froids des générateurs thermodynamiques d'ECS qui y puisent.
///
/// Th-BCE 2020 §5.4.3.2.2, éq. (110), relue sur l'image de la p. 124 : 1 si `Φrejet(h) = 0` ;
/// sinon `[θi_eq − θext − (1 − bet_ns)/bet_ns · Φrejet/Het_th_int_eq]/(θi_eq − θext)`.
/// `Φrejet(h)` est la somme des puissances des évaporateurs au pas précédent (éq. 109). Les
/// générateurs thermodynamiques relèvent du lot 6 : le calcul l'appelle avec `Φrejet = 0`.
/// Si l'écart de température ou `bet_ns` est nul, la correction n'est pas définie : 1.
pub fn c_corr_therm(
    theta_i_eq_c: f64,
    theta_ext_c: f64,
    b_et_ns: f64,
    phi_rejet_w: f64,
    h_et_th_int_eq_w_k: f64,
) -> f64 {
    let ecart = theta_i_eq_c - theta_ext_c;
    #[allow(clippy::float_cmp)]
    if phi_rejet_w == 0.0 || ecart == 0.0 || b_et_ns == 0.0 || h_et_th_int_eq_w_k <= 0.0 {
        return 1.0;
    }
    (ecart - (1.0 - b_et_ns) / b_et_ns * phi_rejet_w / h_et_th_int_eq_w_k) / ecart
}

/// Espace tampon non solarisé, à un pas de temps.
///
/// Th-BCE 2020 §5.4.3.2.2 : éq. (111) `btherm = bet_ns·Ccorr_therm` ; éq. (112) et (113)
/// `bsolaire = becl = 0` ; éq. (114) `θet = btherm·θext + (1 − btherm)·θi_eq`.
pub fn non_solarise(
    b_et_ns: f64,
    theta_ext_c: f64,
    theta_i_eq_c: f64,
    phi_rejet_w: f64,
    h_et_th_int_eq_w_k: f64,
) -> CoefficientsEt {
    let b_therm = b_et_ns
        * c_corr_therm(
            theta_i_eq_c,
            theta_ext_c,
            b_et_ns,
            phi_rejet_w,
            h_et_th_int_eq_w_k,
        );
    CoefficientsEt {
        b_therm,
        b_solaire: 0.0,
        b_ecl: 0.0,
        theta_et_c: b_therm * theta_ext_c + (1.0 - b_therm) * theta_i_eq_c,
        a_ouv_et_m2: 0.0,
    }
}

/// Paroi d'un espace tampon solarisé donnant sur l'extérieur (§5.4.2, tableau 19).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParoiEt {
    /// « Aue^n », en m².
    pub a_ue_m2: f64,
    /// « Uue^n », en W/(m².K).
    pub u_ue_w_m2k: f64,
    /// « Swe_sp^n » : facteur solaire sans protection mobile.
    pub swe_sp: f64,
    /// « Swe_ap^n » : facteur solaire avec protection mobile en place.
    pub swe_ap: f64,
    /// « Tle_sp^n » : transmission lumineuse sans protection mobile.
    pub tle_sp: f64,
    /// « Tle_ap^n » : transmission lumineuse avec protection mobile en place.
    pub tle_ap: f64,
    /// « Rouv_max^n » : ratio d'ouverture maximale.
    pub r_ouv_max: f64,
}

/// « Het_th_ext » : coefficient de transmission de l'espace tampon vers l'extérieur, en W/K.
///
/// Th-BCE 2020 §5.4.3.3.1.1, éq. (115) : `Σ Aue^n·Uue^n + Uet_pb·Aet` ; les ponts thermiques de
/// l'enveloppe de l'espace tampon ne sont pas pris en compte.
pub fn h_et_th_ext_w_k(parois: &[ParoiEt], u_et_pb_w_m2k: f64, a_et_m2: f64) -> f64 {
    parois.iter().map(|p| p.a_ue_m2 * p.u_ue_w_m2k).sum::<f64>() + u_et_pb_w_m2k * a_et_m2
}

/// « Cm_et » : capacité thermique de l'espace tampon, en Wh/K.
///
/// Th-BCE 2020 §5.4.3.3.1.2, éq. (116) : `Cm_et_surf·Aet·KkJ_Wh`.
pub fn c_m_et_wh_k(cm_et_surf_kj_m2k: f64, a_et_m2: f64) -> f64 {
    cm_et_surf_kj_m2k * a_et_m2 * K_KJ_WH
}

/// « Fs_et(h) » : flux solaire transmis à l'espace tampon, en W.
///
/// Th-BCE 2020 §5.4.3.3.1.3 : éq. (117) `Isr*^n = Drp*^n + Dfp*^n + Rrp*^n` (donné par
/// `isr_w_m2`) ; éq. (118) `Swe^n = Rprotet·Swe_ap^n + (1 − Rprotet)·Swe_sp^n` ; éq. (119)
/// `Fs_et = ηapp_sol_et·Σ Swe^n·Aue^n·Isr*^n`.
pub fn fs_et_w(parois: &[ParoiEt], isr_w_m2: &[f64], r_prot_et: f64) -> f64 {
    ETA_APP_SOL_ET
        * parois
            .iter()
            .zip(isr_w_m2)
            .map(|(p, isr)| swe(p, r_prot_et) * p.a_ue_m2 * isr)
            .sum::<f64>()
}

/// Éq. (118) : facteur solaire d'une paroi selon la position des protections.
fn swe(p: &ParoiEt, r_prot_et: f64) -> f64 {
    r_prot_et * p.swe_ap + (1.0 - r_prot_et) * p.swe_sp
}

/// « Aouv_et(h) » : surface d'ouverture des parois de l'espace tampon, en m².
///
/// Th-BCE 2020 §5.4.3.3.1.4, éq. (120) : `Rouvet_eff·Σ Rouv_max^n·Aue^n`.
pub fn a_ouv_et_m2(parois: &[ParoiEt], r_ouv_et_eff: f64) -> f64 {
    r_ouv_et_eff * parois.iter().map(|p| p.r_ouv_max * p.a_ue_m2).sum::<f64>()
}

/// « Qv_surv_ouv(h) » : débit de surventilation par ouverture des parois, en m³/h.
///
/// Th-BCE 2020 §5.4.3.3.1.4, éq. (121), relue sur l'image de la p. 126 :
/// `1800·Aouv_et·[Cw·Vventc² + Cst·Httf_et·|θext(h) − θet(h−1)|]^0,5`.
pub fn qv_surv_ouv_m3_h(
    a_ouv_et_m2: f64,
    v_ventc_m_s: f64,
    h_ttf_et_m: f64,
    theta_ext_c: f64,
    theta_et_h1_c: f64,
) -> f64 {
    1800.0
        * a_ouv_et_m2
        * libm::sqrt(
            C_W * v_ventc_m_s * v_ventc_m_s
                + C_ST * h_ttf_et_m * (theta_ext_c - theta_et_h1_c).abs(),
        )
}

/// Bilan thermique d'un espace tampon solarisé à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BilanEt {
    /// « Φet(h) », en W (éq. 123).
    pub phi_et_w: f64,
    /// « Het(h) », en W/K (éq. 124).
    pub h_et_w_k: f64,
    /// « θet(h) », en °C (éq. 125).
    pub theta_et_c: f64,
    /// Température moyenne de l'air de l'espace tampon sur le pas de temps, en °C, solution
    /// exacte de l'équation dont l'éq. (125) est l'intégrale : sert au bilan d'énergie.
    pub theta_et_moy_c: f64,
}

/// Bilan thermique d'un espace tampon solarisé.
///
/// Th-BCE 2020 §5.4.3.3.1.5, relu sur l'image de la p. 127 :
/// - éq. (123) : `Φet = Fs_et + (Het_th_ext + 0,34·Qv_et_ent)·θext + Het_th_int_eq·θi_eq` ;
/// - éq. (124) : `Het = Het_th_ext + 0,34·Qv_et_ent + Het_th_int_eq` ;
/// - éq. (125) : `θet(h) = Φet/Het − (Φet/Het − θet(h−1))·exp(−Het/Cm_et)`.
///
/// L'éq. (125) est la solution exacte, sur une heure, de `Cm_et·dθ/dt = Φet − Het·θ` : la
/// température moyenne du pas de temps en découle, `θ∞ − (θ∞ − θet(h−1))·Cm_et/Het·(1 − e)`.
/// `Het` est supposé strictement positif (la validation exige des parois extérieures ou un
/// débit d'air). Sans capacité, `θet = θ∞`.
#[allow(clippy::too_many_arguments)]
pub fn bilan_solarise(
    fs_et_w: f64,
    h_et_th_ext_w_k: f64,
    qv_et_ent_m3_h: f64,
    theta_ext_c: f64,
    h_et_th_int_eq_w_k: f64,
    theta_i_eq_c: f64,
    c_m_et_wh_k: f64,
    theta_et_h1_c: f64,
) -> BilanEt {
    let h_air = CV_AIR_WH_M3K * qv_et_ent_m3_h;
    let phi_et_w =
        fs_et_w + (h_et_th_ext_w_k + h_air) * theta_ext_c + h_et_th_int_eq_w_k * theta_i_eq_c;
    let h_et_w_k = h_et_th_ext_w_k + h_air + h_et_th_int_eq_w_k;
    let theta_inf = phi_et_w / h_et_w_k;
    let (theta_et_c, theta_et_moy_c) = if c_m_et_wh_k > 0.0 {
        let tau = h_et_w_k / c_m_et_wh_k;
        let e = libm::exp(-tau);
        (
            theta_inf - (theta_inf - theta_et_h1_c) * e,
            theta_inf - (theta_inf - theta_et_h1_c) * (1.0 - e) / tau,
        )
    } else {
        (theta_inf, theta_inf)
    };
    BilanEt {
        phi_et_w,
        h_et_w_k,
        theta_et_c,
        theta_et_moy_c,
    }
}

/// « btherm(h) » d'un espace tampon solarisé.
///
/// Th-BCE 2020 §5.4.3.3.1.6, éq. (126) : `MIN(1 ; (θi_eq − θet)/(θi_eq − θext))`, borné à −15.
/// Quand `θi_eq = θext`, le rapport n'est pas défini : sa limite est retenue (1 si
/// `θi_eq ≥ θet`, −15 sinon ; décision Q41 point 6).
pub fn b_therm_solarise(theta_i_eq_c: f64, theta_et_c: f64, theta_ext_c: f64) -> f64 {
    let num = theta_i_eq_c - theta_et_c;
    let den = theta_i_eq_c - theta_ext_c;
    #[allow(clippy::float_cmp)]
    let rapport = if den == 0.0 {
        if num >= 0.0 {
            1.0
        } else {
            B_THERM_MIN
        }
    } else {
        num / den
    };
    rapport.clamp(B_THERM_MIN, 1.0)
}

/// « bsolaire(h) » d'un espace tampon solarisé.
///
/// Th-BCE 2020 §5.4.3.3.2, éq. (118) et (127) : `ηatt_Sw·Σ Swe^n·Aue^n/Σ Aue^n`.
pub fn b_solaire(parois: &[ParoiEt], r_prot_et: f64) -> f64 {
    let a: f64 = parois.iter().map(|p| p.a_ue_m2).sum();
    if a <= 0.0 {
        return 0.0;
    }
    ETA_ATT_SW
        * parois
            .iter()
            .map(|p| swe(p, r_prot_et) * p.a_ue_m2)
            .sum::<f64>()
        / a
}

/// « becl(h) » d'un espace tampon solarisé.
///
/// Th-BCE 2020 §5.4.3.3.2, éq. (128) et (129) :
/// `Tle^n = Rprotet·Tle_ap^n + (1 − Rprotet)·Tle_sp^n` ; `becl = ηatt_Tl·Σ Tle^n·Aue^n/Σ Aue^n`.
pub fn b_ecl(parois: &[ParoiEt], r_prot_et: f64) -> f64 {
    let a: f64 = parois.iter().map(|p| p.a_ue_m2).sum();
    if a <= 0.0 {
        return 0.0;
    }
    ETA_ATT_TL
        * parois
            .iter()
            .map(|p| (r_prot_et * p.tle_ap + (1.0 - r_prot_et) * p.tle_sp) * p.a_ue_m2)
            .sum::<f64>()
        / a
}

/// Température ambiante d'un composant de système placé hors du volume chauffé, dans un
/// espace tampon, en °C.
///
/// Forme commune aux fiches des systèmes (par exemple la génération, §10, p. 670 :
/// `θamb(h) = btherm(h)·θext(h) + [1 − btherm(h)]·θamb_ch`) : interpolation entre l'extérieur
/// et la température de référence du volume chauffé. Préparée pour le calcul Th-C (lot 6).
pub fn theta_ambiance_hors_volume_chauffe_c(
    b_therm: f64,
    theta_ext_c: f64,
    theta_ref_c: f64,
) -> f64 {
    b_therm * theta_ext_c + (1.0 - b_therm) * theta_ref_c
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn veranda() -> Vec<ParoiEt> {
        vec![
            ParoiEt {
                a_ue_m2: 10.0,
                u_ue_w_m2k: 1.4,
                swe_sp: 0.6,
                swe_ap: 0.1,
                tle_sp: 0.7,
                tle_ap: 0.05,
                r_ouv_max: 0.5,
            },
            ParoiEt {
                a_ue_m2: 6.0,
                u_ue_w_m2k: 1.4,
                swe_sp: 0.5,
                swe_ap: 0.1,
                tle_sp: 0.6,
                tle_ap: 0.05,
                r_ouv_max: 0.0,
            },
        ]
    }

    #[test]
    fn coefficient_et_temperature_equivalents() {
        // Éq. (106) : 2 × 1,3 + 10 × 0,2 + 4 × 0,1.
        let h = h_et_th_int_gr_w_k([2.6], [2.0], [0.4]);
        assert!((h - 5.0).abs() < 1e-12);
        // Éq. (108) : (5 × 20 + 15 × 18)/20 = 18,5.
        assert!((theta_i_eq_c([(5.0, 20.0), (15.0, 18.0)]).unwrap() - 18.5).abs() < 1e-12);
        assert!(theta_i_eq_c([]).is_none());
    }

    #[test]
    fn espace_non_solarise() {
        // Sans rejet : btherm = bet_ns ; θet = 0,6 × 0 + 0,4 × 20 = 8 °C.
        let c = non_solarise(0.6, 0.0, 20.0, 0.0, 30.0);
        assert_eq!(c.b_therm, 0.6);
        assert_eq!((c.b_solaire, c.b_ecl), (0.0, 0.0));
        assert!((c.theta_et_c - 8.0).abs() < 1e-12);
        // Éq. (110) avec Φrejet = 300 W : Ccorr = (20 − (0,4/0,6) × 300/30)/20 = 2/3.
        let k = c_corr_therm(20.0, 0.0, 0.6, 300.0, 30.0);
        assert!((k - (20.0 - 0.4 / 0.6 * 10.0) / 20.0).abs() < 1e-12);
        assert_eq!(c_corr_therm(20.0, 20.0, 0.6, 300.0, 30.0), 1.0);
    }

    #[test]
    fn bilan_d_un_espace_solarise_a_la_main() {
        let p = veranda();
        // Éq. (115) : 16 × 1,4 + 0,5 × 12 = 28,4 W/K.
        let h_ext = h_et_th_ext_w_k(&p, 0.5, 12.0);
        assert!((h_ext - 28.4).abs() < 1e-12);
        // Éq. (116) : 200 × 12/3,6 = 666,67 Wh/K.
        let c = c_m_et_wh_k(200.0, 12.0);
        assert!((c - 2400.0 / 3.6).abs() < 1e-9);
        // Éq. (119), protections à moitié : Swe = 0,35 et 0,3 ; 0,8 × (0,35 × 10 × 400
        // + 0,3 × 6 × 200) = 1 408 W.
        let fs = fs_et_w(&p, &[400.0, 200.0], 0.5);
        assert!((fs - 1408.0).abs() < 1e-9);
        // Éq. (120) et (121) : Aouv = 0,4 × 5 = 2 m² ; vent 2 m/s, Httf 2 m, écart 5 °C :
        // 1 800 × 2 × (0,001 × 4 + 0,0035 × 2 × 5)^0,5 = 3 600 × 0,039^0,5.
        let a = a_ouv_et_m2(&p, 0.4);
        assert!((a - 2.0).abs() < 1e-12);
        let q = qv_surv_ouv_m3_h(a, 2.0, 2.0, 25.0, 20.0);
        assert!((q - 3600.0 * libm::sqrt(0.004 + 0.035)).abs() < 1e-9);
        // Éq. (123) à (125), sans soleil ni air : θ∞ = (28,4 × 0 + 20 × 20)/48,4.
        let b = bilan_solarise(0.0, h_ext, 0.0, 0.0, 20.0, 20.0, c, 10.0);
        let theta_inf = 400.0 / 48.4;
        assert!(
            (b.theta_et_c - (theta_inf - (theta_inf - 10.0) * libm::exp(-48.4 / c))).abs() < 1e-12
        );
        // Éq. (126) : btherm = (20 − θet)/20, entre 0 et 1.
        let bt = b_therm_solarise(20.0, b.theta_et_c, 0.0);
        assert!((bt - (20.0 - b.theta_et_c) / 20.0).abs() < 1e-12);
        assert!(bt > 0.0 && bt < 1.0);
    }

    #[test]
    fn bilan_d_energie_d_un_pas_de_temps() {
        // C·Δθ = (Φ − H·θmoy)·1 h : l'éq. (125) intègre exactement l'équation différentielle.
        let c = 500.0;
        let b = bilan_solarise(2000.0, 30.0, 20.0, 5.0, 15.0, 19.0, c, 12.0);
        let stock = c * (b.theta_et_c - 12.0);
        let echange = b.phi_et_w - b.h_et_w_k * b.theta_et_moy_c;
        assert!((stock - echange).abs() < 1e-9);
        // Sans capacité, l'équilibre est atteint dans le pas de temps.
        let s = bilan_solarise(2000.0, 30.0, 20.0, 5.0, 15.0, 19.0, 0.0, 12.0);
        assert!((s.theta_et_c - s.phi_et_w / s.h_et_w_k).abs() < 1e-12);
    }

    #[test]
    fn bornes_de_btherm() {
        // Espace plus chaud que l'intérieur : btherm négatif.
        assert!((b_therm_solarise(20.0, 30.0, 0.0) + 0.5).abs() < 1e-12);
        // Plafond à 1, plancher à −15.
        assert_eq!(b_therm_solarise(20.0, -5.0, 0.0), 1.0);
        assert_eq!(b_therm_solarise(20.0, 40.0, 19.0), B_THERM_MIN);
        // θi_eq = θext.
        assert_eq!(b_therm_solarise(20.0, 18.0, 20.0), 1.0);
        assert_eq!(b_therm_solarise(20.0, 22.0, 20.0), B_THERM_MIN);
    }

    #[test]
    fn coefficients_solaire_et_d_eclairement() {
        let p = veranda();
        // Éq. (127) sans protection : 0,9 × (0,6 × 10 + 0,5 × 6)/16.
        assert!((b_solaire(&p, 0.0) - 0.9 * 9.0 / 16.0).abs() < 1e-12);
        // Protections baissées : 0,9 × 0,1.
        assert!((b_solaire(&p, 1.0) - 0.09).abs() < 1e-12);
        // Éq. (129).
        assert!((b_ecl(&p, 0.0) - 0.9 * (7.0 + 3.6) / 16.0).abs() < 1e-12);
        assert!((b_ecl(&p, 1.0) - 0.045).abs() < 1e-12);
        assert_eq!(b_solaire(&[], 0.0), 0.0);
    }

    #[test]
    fn ambiance_d_un_systeme_hors_volume_chauffe() {
        assert!((theta_ambiance_hors_volume_chauffe_c(0.25, 0.0, 20.0) - 15.0).abs() < 1e-12);
    }
}
