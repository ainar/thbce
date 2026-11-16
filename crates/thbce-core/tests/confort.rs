// SPDX-License-Identifier: GPL-3.0-or-later
//! Indicateurs de confort (Th-BCE 2020 §4.5) : valeurs calculées à la main.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::confort::indicateurs_confort::{
    indicateurs, indicateurs_selon, theta_op_inc_max_c, CumulsConfort, LectureEcarts, CAT_AMB,
};

fn proche(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * (1.0 + b.abs())
}

macro_rules! assert_proche {
    ($a:expr, $b:expr) => {
        assert!(proche($a, $b), "{} obtenu, {} attendu", $a, $b)
    };
}

#[test]
fn seuil_d_inconfort_chaud() {
    // Éq. (58), consigne de 26 °C. θrm = 20 °C : 0,33 × 20 + 18,8 = 25,4 ; catégories 1 à 3 :
    // 27,4, 28,4 et 29,4 °C.
    assert_proche!(theta_op_inc_max_c(26.0, 20.0, 1), 27.4);
    assert_proche!(theta_op_inc_max_c(26.0, 20.0, 2), 28.4);
    assert_proche!(theta_op_inc_max_c(26.0, 20.0, 3), 29.4);
    // θrm = 10 °C : 3,3 + 18,8 + 2 = 24,1 °C, inférieur à la consigne, qui l'emporte
    // (figure 13 : le seuil de la catégorie 1 décolle de 26 °C vers θrm = 15,8 °C).
    assert_eq!(theta_op_inc_max_c(26.0, 10.0, 1), 26.0);
    assert_proche!(theta_op_inc_max_c(26.0, (26.0 - 20.8) / 0.33, 1), 26.0);
    // Consigne réduite de 30 °C en inoccupation : elle borne le seuil par le bas.
    assert_eq!(theta_op_inc_max_c(30.0, 20.0, 3), 30.0);
}

#[test]
fn inconfort_froid() {
    // Éq. (56) et (60) : occupation, θop = 17,5 °C sous une consigne de 19 °C.
    let i = indicateurs(true, 19.0, 26.0, 5.0, 17.5);
    assert_eq!(i.i_inconf_cons_ch, 1);
    assert_proche!(i.delta_theta_op_inc_cons_ch_c, 1.5);
    assert_eq!((i.i_inconf_cons_fr, i.i_inconf_c_fr), (0, [0, 0, 0]));
    assert_eq!(i.delta_theta_op_inc_cons_fr_c, 0.0);
    assert_eq!(i.delta_theta_op_inc_c_fr_c, [0.0; 3]);
    // À la consigne exactement : pas d'inconfort (inégalité stricte).
    let i = indicateurs(true, 19.0, 26.0, 5.0, 19.0);
    assert_eq!(i.i_inconf_cons_ch, 0);
    assert_eq!(i.delta_theta_op_inc_cons_ch_c, 0.0);
}

#[test]
fn inconfort_chaud() {
    // Éq. (59) et (61), θrm = 20 °C : seuils de 27,4, 28,4 et 29,4 °C ; θop = 28,9 °C.
    let i = indicateurs(true, 19.0, 26.0, 20.0, 28.9);
    assert_eq!(i.i_inconf_cons_ch, 0);
    assert_eq!(i.i_inconf_cons_fr, 1);
    assert_eq!(i.i_inconf_c_fr, [1, 1, 0]);
    assert_proche!(i.delta_theta_op_inc_cons_fr_c, 2.9);
    assert_proche!(i.delta_theta_op_inc_c_fr_c[0], 1.5);
    assert_proche!(i.delta_theta_op_inc_c_fr_c[1], 0.5);
    assert_eq!(i.delta_theta_op_inc_c_fr_c[2], 0.0);
    // Tableaux 9 et 10 du §4.6.3.2 : catégorie d'ambiance 1 pour tous les usages.
    assert_eq!(CAT_AMB, 1);
    assert_proche!(i.delta_theta_op_inc_fr_c(CAT_AMB), 1.5);
    assert_proche!(i.delta_theta_op_inc_fr_c(0), 2.9);
    assert_proche!(i.delta_theta_op_inc_fr_c(2), 0.5);
}

#[test]
fn inoccupation() {
    // Tous les indicateurs valent −1 et tous les écarts sont nuls, quelle que soit la
    // température : i × (i + 1)/2 = −1 × 0 = 0.
    for theta_op in [10.0, 22.0, 35.0] {
        let i = indicateurs(false, 16.0, 30.0, 25.0, theta_op);
        assert_eq!((i.i_inconf_cons_ch, i.i_inconf_cons_fr), (-1, -1));
        assert_eq!(i.i_inconf_c_fr, [-1; 3]);
        assert_eq!(i.delta_theta_op_inc_cons_ch_c, 0.0);
        assert_eq!(i.delta_theta_op_inc_cons_fr_c, 0.0);
        assert_eq!(i.delta_theta_op_inc_c_fr_c, [0.0; 3]);
    }
}

#[test]
fn difference_entre_les_deux_lectures_des_ecarts() {
    // Q38 point 1. Les éq. (61) imprimées pondèrent l'écart d'inconfort chaud par
    // (iinconf_cons,ch + 1)/2, qui vaut 1/2 en occupation sans inconfort froid : l'écart est
    // divisé par deux. La lecture retenue emploie l'indicateur propre à chaque écart.
    let args = (true, 19.0, 26.0, 20.0, 28.9);
    let retenue = indicateurs_selon(
        LectureEcarts::IndicateurPropre,
        args.0,
        args.1,
        args.2,
        args.3,
        args.4,
    );
    let litterale = indicateurs_selon(
        LectureEcarts::Litterale,
        args.0,
        args.1,
        args.2,
        args.3,
        args.4,
    );
    assert_proche!(retenue.delta_theta_op_inc_c_fr_c[0], 1.5);
    assert_proche!(litterale.delta_theta_op_inc_c_fr_c[0], 0.75);
    assert_proche!(litterale.delta_theta_op_inc_cons_fr_c, 1.45);
    // L'inconfort froid ne dépend pas de la lecture.
    let froid = |l| indicateurs_selon(l, true, 19.0, 26.0, 5.0, 17.0).delta_theta_op_inc_cons_ch_c;
    assert_proche!(froid(LectureEcarts::IndicateurPropre), 2.0);
    assert_proche!(froid(LectureEcarts::Litterale), 2.0);
}

#[test]
fn cumuls_d_heures_et_de_degres_heures() {
    // Éq. (62) à (65) sur quatre pas de temps : froid de 1,5 K ; confort ; chaud (28,9 °C) ;
    // inoccupation.
    let mut c = CumulsConfort::default();
    c.ajouter(&indicateurs(true, 19.0, 26.0, 5.0, 17.5));
    c.ajouter(&indicateurs(true, 19.0, 26.0, 5.0, 21.0));
    c.ajouter(&indicateurs(true, 19.0, 26.0, 20.0, 28.9));
    c.ajouter(&indicateurs(false, 16.0, 30.0, 20.0, 35.0));
    assert_eq!(c.nbh_inc_cons_ch, 1);
    assert_eq!(c.nbh_inc_cons_fr, 1);
    assert_eq!(c.nbh_inc_c_fr, [1, 1, 0]);
    assert_proche!(c.dh_inc_cons_ch_k_h, 1.5);
    assert_proche!(c.dh_inc_cons_fr_k_h, 2.9);
    assert_proche!(c.dh_inc_c_fr_k_h[0], 1.5);
    assert_proche!(c.dh_inc_c_fr_k_h[1], 0.5);
    assert_eq!(c.dh_inc_c_fr_k_h[2], 0.0);
}
