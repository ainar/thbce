// SPDX-License-Identifier: GPL-3.0-or-later
//! Éclairage en habitation (Th-BCE 2020 §7.1) : valeurs calculées à la main.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::eclairage::{
    c2_habitation, eclairage_habitation, einat_vn_lux, k_vn, ro_moyen_vn, AutonomieLumineuse,
};

fn proche(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * (1.0 + b.abs())
}

macro_rules! assert_proche {
    ($a:expr, $b:expr) => {
        assert!(proche($a, $b), "{} obtenu, {} attendu", $a, $b)
    };
}

#[test]
fn coefficients_de_l_eclairement_naturel() {
    // Romoyen = (0,2 + 2,5 × 0,5 + 0,7)/(2 + 2,5) = 2,15/4,5 = 0,477778.
    assert_proche!(ro_moyen_vn(), 2.15 / 4.5);
    // Interréflexions : I = Romoyen/(1 − Romoyen)/4,5 = 0,914894/4,5 = 0,203310.
    // K1vn = 0,2 × I = 0,040662 ; K2vn = 0,4 + I = 0,603310 ; K3vn = 0,7 × 0,8 + 0,7 × I = 0,702317.
    let [k1, k2, k3] = k_vn();
    assert_proche!(k1, 0.040_661_938_534_278_96);
    assert_proche!(k2, 0.603_309_692_671_394_8);
    assert_proche!(k3, 0.702_316_784_869_976_3);
}

#[test]
fn eclairement_naturel_d_un_volume_normal() {
    // Flux de la baie sud de 8 m² du test des baies (sans protection) : Flt1 = 332 553,755 lm,
    // Flt2 = 28 800 lm, Flt3 = 24 960 lm ; Aeclnat2 = 96,44 m².
    // Einat = (0,040662 × 332 553,755 + 0,603310 × 28 800 + 0,702317 × 24 960)/96,44 = 502,15 lux.
    let e = einat_vn_lux(332_553.755_053_224_45, 28_800.0, 24_960.0, 96.44);
    assert_proche!(e, 502.150_834_162_263_1);
    // Linéaire en flux ; nul sans surface ayant accès à la lumière naturelle.
    assert_proche!(
        einat_vn_lux(0.0, 28_800.0, 0.0, 96.44),
        0.603_309_692_671_394_8 * 28_800.0 / 96.44
    );
    assert_eq!(einat_vn_lux(1.0e5, 1.0e4, 1.0e4, 0.0), 0.0);
}

#[test]
fn coefficient_c2_en_habitation() {
    // Figure 86 : A (0 ; 1), B (100 ; 1), G (200 ; 0,05), J (2800 ; 0).
    assert_eq!(c2_habitation(0.0), 1.0);
    assert_eq!(c2_habitation(100.0), 1.0);
    assert_proche!(c2_habitation(150.0), 0.525);
    assert_proche!(c2_habitation(200.0), 0.05);
    assert_proche!(c2_habitation(1_500.0), 0.025);
    assert_eq!(c2_habitation(2_800.0), 0.0);
    assert_eq!(c2_habitation(10_000.0), 0.0);
}

#[test]
fn consommation_et_apports_d_un_groupe_d_habitation() {
    // 100 m², 150 lux : 1 W/m² × 0,9 × 0,525 × 100 m² = 47,25 W, soit 0,04725 kWh sur l'heure ;
    // apports récupérables : moitié convectif, moitié radiatif.
    let e = eclairage_habitation(true, 150.0, 100.0);
    assert_proche!(e.c_ecl_kwh, 0.047_25);
    assert_proche!(e.f_ecl_c_w, 23.625);
    assert_proche!(e.f_ecl_r_w, 23.625);
    // De nuit (Einat = 0) : 0,9 W/m², soit 0,09 kWh pour 100 m².
    assert_proche!(eclairage_habitation(true, 0.0, 100.0).c_ecl_kwh, 0.09);
    // Éclairage non autorisé par le scénario (IEcl = 0), ou plein jour au-delà de 2 800 lux.
    assert_eq!(eclairage_habitation(false, 0.0, 100.0).c_ecl_kwh, 0.0);
    assert_eq!(eclairage_habitation(true, 3_000.0, 100.0).c_ecl_kwh, 0.0);
}

#[test]
fn ordre_de_grandeur_et_unites_des_apports_d_eclairage() {
    // Décision Q33, point 1. Logement de 100 m², éclairage autorisé, sans lumière naturelle :
    // éq. (789), 1 W/m² × 0,9 × 1 × 100 m² / 1000 = 0,09 kWh sur l'heure. Les apports transmis au
    // modèle thermique sont en watts : 0,09 kWh sur une heure = 90 W, dont 45 W convectifs et
    // 45 W radiatifs (§7.1.3.7).
    let e = eclairage_habitation(true, 0.0, 100.0);
    assert_proche!(e.c_ecl_kwh, 0.09);
    assert_proche!(e.f_ecl_c_w, 45.0);
    assert_proche!(e.f_ecl_r_w, 45.0);
    assert_proche!(e.f_ecl_c_w + e.f_ecl_r_w, 90.0);
    // Cohérence des unités : la somme des apports, en W, vaut 1000 fois la consommation en kWh.
    assert_proche!(e.f_ecl_c_w + e.f_ecl_r_w, 1_000.0 * e.c_ecl_kwh);
}

#[test]
fn autonomie_en_lumiere_naturelle() {
    // Éq. (785), seuil de 300 lux.
    let mut a = AutonomieLumineuse::default();
    assert_eq!(a.taux_occ_einat_sup_pct(), None);
    a.compter(false, true, 1_000.0);
    a.compter(true, false, 0.0);
    a.compter(true, true, 299.9);
    a.compter(true, true, 300.0);
    a.compter(true, true, 800.0);
    a.compter(true, true, 50.0);
    assert_eq!(
        (
            a.nbh_ecl_non_aut,
            a.nbh_occ_nuit,
            a.nbh_occ_einat_inf,
            a.nbh_occ_einat_sup
        ),
        (1, 1, 2, 2)
    );
    assert_proche!(a.taux_occ_einat_sup_pct().unwrap(), 50.0);
}
