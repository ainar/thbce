// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests de la fiche `C_EEX_environnement_proche` (Th-BCE 2020 §3.2) sur des valeurs calculées
//! à la main à partir des formules du texte, et tests de propriétés.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use proptest::prelude::*;
use thbce_core::climat::environnement_proche::*;
use thbce_core::entree::{Exposition, Masque};

fn surface(azimut_deg: f64, inclinaison_deg: f64, lp_m: f64, hp_m: f64) -> SurfaceExposee {
    SurfaceExposee::depuis_exposition(&Exposition {
        azimut_deg,
        inclinaison_deg,
        lpb_m: Some(lp_m),
        hpb_m: Some(hp_m),
        masques: vec![],
    })
}

fn soleil(gamma_deg: f64, psi_deg: f64) -> Soleil {
    Soleil {
        gamma_rad: gamma_deg.to_radians(),
        psi_rad: psi_deg.to_radians(),
    }
}

fn proche(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * (1.0 + b.abs())
}

macro_rules! assert_proche {
    ($a:expr, $b:expr) => {
        assert!(proche($a, $b), "{} obtenu, {} attendu", $a, $b)
    };
}

const SUD: f64 = 0.0;
const OUEST: f64 = 90.0;
const NORD: f64 = 180.0;

#[test]
fn paroi_horizontale() {
    // β = 0 : cos(θ) = sin(γ). γ = 30°, IDn = 800, Idi = 100 :
    // Drp = 800 × 0,5 = 400 ; Dfp = 100 × 0,5 × 2 = 100 ; Rrp = 0 (1 − |cos 0| = 0).
    let f = flux_sans_masque(
        &surface(SUD, 0.0, 1.0, 1.0),
        &soleil(30.0, 40.0),
        800.0,
        100.0,
        80_000.0,
        10_000.0,
    );
    assert_proche!(f.theta_rad, 60f64.to_radians());
    assert_proche!(f.drp_w_m2, 400.0);
    assert_proche!(f.dfp_w_m2, 100.0);
    assert_proche!(f.rrp_w_m2, 0.0);
    assert_proche!(f.isr_w_m2(), 500.0);
    assert_proche!(f.erp_lux, 40_000.0);
    assert_proche!(f.efp_lux, 10_000.0);
    assert_proche!(f.etp_lux(), 50_000.0);
}

#[test]
fn paroi_verticale_face_au_soleil() {
    // Sud, β = 90°, soleil au sud à γ = 30° : cos(θ) = cos(30°) = 0,8660254038.
    // Drp = 692,8203230 ; Dfp = 100 × 0,5 = 50 ; IDh = 400 ; Rrp = (400 + 100) × 0,2 × 0,5 = 50.
    let f = flux_sans_masque(
        &surface(SUD, 90.0, 1.0, 1.0),
        &soleil(30.0, 0.0),
        800.0,
        100.0,
        0.0,
        0.0,
    );
    assert_proche!(f.theta_rad, 30f64.to_radians());
    assert_proche!(f.drp_w_m2, 692.820_323_027_551);
    assert_proche!(f.dfp_w_m2, 50.0);
    assert_proche!(f.rrp_w_m2, 50.0);
}

#[test]
fn paroi_verticale_dos_au_soleil_et_paroi_ouest() {
    // Nord, soleil au sud : l'arc cosinus dépasse π/2, θ est borné à π/2 et Drp est nul.
    let f = flux_sans_masque(
        &surface(NORD, 90.0, 1.0, 1.0),
        &soleil(30.0, 0.0),
        800.0,
        100.0,
        0.0,
        0.0,
    );
    assert_proche!(f.theta_rad, std::f64::consts::FRAC_PI_2);
    assert!(f.drp_w_m2.abs() < 1e-9);
    assert_proche!(f.dfp_w_m2, 50.0);
    // Ouest, soleil à l'ouest (ψ = +90°) à γ = 45° : cos(θ) = cos(45°).
    let f = flux_sans_masque(
        &surface(OUEST, 90.0, 1.0, 1.0),
        &soleil(45.0, 90.0),
        100.0,
        0.0,
        0.0,
        0.0,
    );
    assert_proche!(f.drp_w_m2, 100.0 * std::f64::consts::FRAC_1_SQRT_2);
    // Le matin (ψ = −90°), la même paroi est à l'ombre.
    let f = flux_sans_masque(
        &surface(OUEST, 90.0, 1.0, 1.0),
        &soleil(45.0, -90.0),
        100.0,
        0.0,
        0.0,
        0.0,
    );
    assert!(f.drp_w_m2.abs() < 1e-9);
}

#[test]
fn paroi_tournee_vers_le_bas() {
    // β = 180° : ni direct ni diffus (1 + cos β = 0), ni réfléchi (1 − |cos β| = 0).
    let f = flux_sans_masque(
        &surface(SUD, 180.0, 1.0, 1.0),
        &soleil(50.0, 10.0),
        700.0,
        150.0,
        0.0,
        0.0,
    );
    assert!(f.drp_w_m2.abs() < 1e-9 && f.dfp_w_m2.abs() < 1e-9 && f.rrp_w_m2.abs() < 1e-9);
}

#[test]
fn soleil_a_l_horizon_avec_direct_non_nul() {
    // Question Q19 : γ = 0 et IDn > 0. Les formules restent définies : rien sur l'horizontale
    // (cos θ = sin 0), incidence normale sur une verticale face au soleil.
    let s = soleil(0.0, 0.0);
    assert!(
        flux_sans_masque(&surface(SUD, 0.0, 1.0, 1.0), &s, 100.0, 0.0, 0.0, 0.0)
            .drp_w_m2
            .abs()
            < 1e-9
    );
    assert_proche!(
        flux_sans_masque(&surface(SUD, 90.0, 1.0, 1.0), &s, 100.0, 0.0, 0.0, 0.0).drp_w_m2,
        100.0
    );
    assert!(!jour_astro(&s));
    assert!(jour_astro(&Soleil {
        gamma_rad: 0.0001,
        psi_rad: 0.0
    }));
}

#[test]
fn masque_proche_horizontal() {
    // Casquette dhm = 1 m, dhp = 0,5 m, baie de hauteur hp = 2 m, plein sud.
    let s = surface(SUD, 90.0, 3.0, 2.0);
    let m = MasqueResolu::ProcheHorizontal {
        dhm_m: 1.0,
        dhp_m: 0.5,
    };
    // γ = 45°, Φ = 0 : dh = 1 × tg(45°) = 1 ; F = 1 − (1 − 0,5)/2 = 0,75.
    let f = affaiblissement(&m, &s, &soleil(45.0, 0.0), 0);
    assert_proche!(f.dir, 0.75);
    // Diffus : θg = arctan((0,5 + 1)/1) = 56,30993247° ; F = θg/90 = 0,6256659164.
    assert_proche!(f.dif, 0.625_665_916_335_157);
    // γ = 20° : dh = 0,364 < dhp, la baie est entièrement éclairée.
    assert_proche!(affaiblissement(&m, &s, &soleil(20.0, 0.0), 0).dir, 1.0);
    // γ = 80° : dh = 5,67 ; 1 − 5,17/2 < 0, borné à 0.
    assert_proche!(affaiblissement(&m, &s, &soleil(80.0, 0.0), 0).dir, 0.0);
    // Φ = 60° : dh = tg(45°)/cos(60°) = 2 ; F = 1 − 1,5/2 = 0,25.
    assert_proche!(affaiblissement(&m, &s, &soleil(45.0, 60.0), 0).dir, 0.25);
    // Soleil derrière la paroi : direct non affaibli, diffus toujours affaibli.
    let f = affaiblissement(&m, &s, &soleil(45.0, 180.0), 0);
    assert_proche!(f.dir, 1.0);
    assert_proche!(f.dif, 0.625_665_916_335_157);
    // Composant non vertical : sans effet.
    assert_eq!(
        affaiblissement(&m, &surface(SUD, 45.0, 3.0, 2.0), &soleil(45.0, 0.0), 0),
        Affaiblissement::SANS_EFFET
    );
}

#[test]
fn masques_proches_verticaux() {
    let s = surface(SUD, 90.0, 2.0, 1.5);
    let gauche = MasqueResolu::ProcheVerticalGauche {
        dvg_m: 1.0,
        dpg_m: 0.2,
    };
    let droit = MasqueResolu::ProcheVerticalDroit {
        dvd_m: 1.0,
        dpd_m: 0.0,
    };
    // Φ = +45° : dlg = 1 × tg(45°) = 1 ; Fvg = 1 − (1 − 0,2)/2 = 0,6. Le masque droit ne joue pas.
    assert_proche!(
        affaiblissement(&gauche, &s, &soleil(30.0, 45.0), 0).dir,
        0.6
    );
    assert_proche!(affaiblissement(&droit, &s, &soleil(30.0, 45.0), 0).dir, 1.0);
    // Φ = −45° : dld = 1 ; Fvd = 1 − 1/2 = 0,5. Le masque gauche ne joue pas.
    assert_proche!(
        affaiblissement(&droit, &s, &soleil(30.0, -45.0), 0).dir,
        0.5
    );
    assert_proche!(
        affaiblissement(&gauche, &s, &soleil(30.0, -45.0), 0).dir,
        1.0
    );
    // Le diffus n'est pas affaibli par les masques verticaux.
    assert_proche!(
        affaiblissement(&gauche, &s, &soleil(30.0, 45.0), 0).dif,
        1.0
    );
    // Profondeur nulle, soleil derrière, composant incliné : sans effet.
    let nul = MasqueResolu::ProcheVerticalGauche {
        dvg_m: 0.0,
        dpg_m: 0.2,
    };
    assert_eq!(
        affaiblissement(&nul, &s, &soleil(30.0, 45.0), 0),
        Affaiblissement::SANS_EFFET
    );
    assert_eq!(
        affaiblissement(&gauche, &s, &soleil(30.0, 120.0), 0),
        Affaiblissement::SANS_EFFET
    );
    assert_eq!(
        affaiblissement(
            &gauche,
            &surface(SUD, 60.0, 2.0, 1.5),
            &soleil(30.0, 45.0),
            0
        ),
        Affaiblissement::SANS_EFFET
    );
}

#[test]
fn masque_lointain_plan_vertical() {
    // dE = 10 m, hpE = 5 m. γ = 30° : dhE = 5,77 > 5, éclairé ; γ = 20° : dhE = 3,64 < 5, ombre.
    let m = MasqueResolu::LointainPlanVertical {
        de_m: 10.0,
        hpe_m: 5.0,
    };
    let s = surface(SUD, 90.0, 1.0, 1.0);
    assert_proche!(affaiblissement(&m, &s, &soleil(30.0, 0.0), 0).dir, 1.0);
    assert_proche!(affaiblissement(&m, &s, &soleil(20.0, 0.0), 0).dir, 0.0);
    // Φ = 60° : dhE = 10 × tg(20°)/0,5 = 7,28 > 5, éclairé.
    assert_proche!(affaiblissement(&m, &s, &soleil(20.0, 60.0), 0).dir, 1.0);
    // Valable pour toute inclinaison ; sans effet soleil derrière ; diffus non affaibli.
    assert_proche!(
        affaiblissement(&m, &surface(SUD, 30.0, 1.0, 1.0), &soleil(20.0, 0.0), 0).dir,
        0.0
    );
    assert_eq!(
        affaiblissement(&m, &s, &soleil(20.0, 170.0), 0),
        Affaiblissement::SANS_EFFET
    );
    assert_proche!(affaiblissement(&m, &s, &soleil(20.0, 0.0), 0).dif, 1.0);
}

#[test]
fn tranches_azimutales_de_la_figure_10() {
    // Les tranches 18 et 19 encadrent la normale ; un soleil à Φ = 25° est dans la tranche 21.
    for (phi_deg, tranche) in [
        (-180.0, 1),
        (-175.0, 1),
        (-5.0, 18),
        (0.0, 19),
        (5.0, 19),
        (25.0, 21),
        (179.0, 36),
        (180.0, 1),
    ] {
        assert_eq!(
            tranche_azimutale(f64::to_radians(phi_deg)) + 1,
            tranche,
            "Φ = {phi_deg}°"
        );
    }
}

#[test]
fn masque_lointain_azimutal_et_arbres_caducs() {
    // Horizon à 20° partout sauf la tranche 19 (Φ de 0 à 10°), dégagée.
    let mut gamma_i_deg = vec![20.0; 36];
    gamma_i_deg[18] = 0.0;
    let azimutal = MasqueResolu::depuis_masque(&Masque::LointainAzimutal {
        gamma_i_deg: gamma_i_deg.clone(),
    });
    let s = surface(SUD, 90.0, 1.0, 1.0);
    assert_proche!(
        affaiblissement(&azimutal, &s, &soleil(30.0, -40.0), 0).dir,
        1.0
    );
    assert_proche!(
        affaiblissement(&azimutal, &s, &soleil(10.0, -40.0), 0).dir,
        0.0
    );
    // « Si γ est supérieur à γi » : à l'égalité, le composant est à l'ombre.
    assert_proche!(
        affaiblissement(&azimutal, &s, &soleil(20.0, -40.0), 0).dir,
        0.0
    );
    assert_proche!(
        affaiblissement(&azimutal, &s, &soleil(10.0, 5.0), 0).dir,
        1.0
    );
    assert_proche!(
        affaiblissement(&azimutal, &s, &soleil(10.0, -40.0), 0).dif,
        1.0
    );
    // La tranche se repère par rapport à la normale : paroi ouest, soleil à ψ = 95°, Φ = 5°.
    assert_proche!(
        affaiblissement(
            &azimutal,
            &surface(OUEST, 90.0, 1.0, 1.0),
            &soleil(10.0, 95.0),
            0
        )
        .dir,
        1.0
    );

    // Arbres caducs : masque seulement si 2161 < Tsimul < 6553.
    let caducs = MasqueResolu::depuis_masque(&Masque::LointainArbresCaducs { gamma_i_deg });
    let bas = soleil(10.0, -40.0);
    for (h, attendu) in [
        (0, 1.0),
        (2161, 1.0),
        (2162, 0.0),
        (6552, 0.0),
        (6553, 1.0),
        (8759, 1.0),
    ] {
        assert_proche!(affaiblissement(&caducs, &s, &bas, h).dir, attendu);
    }
}

#[test]
fn facteurs_globaux_et_flux_atteignant_la_paroi() {
    // Éq. (23) et (24) : produit des facteurs. Casquette (0,75 ; 0,6257) et joue gauche (0,6 à
    // Φ = 45° — ici Φ = 0, donc 1 − (0 − 0,2)/3 borné à 1).
    let s = surface(SUD, 90.0, 3.0, 2.0);
    let masques = [
        MasqueResolu::ProcheHorizontal {
            dhm_m: 1.0,
            dhp_m: 0.5,
        },
        MasqueResolu::ProcheVerticalGauche {
            dvg_m: 1.0,
            dpg_m: 0.2,
        },
        MasqueResolu::LointainPlanVertical {
            de_m: 10.0,
            hpe_m: 5.0,
        },
    ];
    let soleil_45 = soleil(45.0, 0.0);
    let ff = affaiblissement_global(&masques, &s, &soleil_45, 0);
    assert_proche!(ff.dir, 0.75);
    assert_proche!(ff.dif, 0.625_665_916_335_157);
    assert_eq!(
        affaiblissement_global(&[], &s, &soleil_45, 0),
        Affaiblissement::SANS_EFFET
    );

    let brut = flux_sans_masque(&s, &soleil_45, 800.0, 100.0, 80_000.0, 10_000.0);
    let net = flux_atteignant_la_paroi(
        &s, &masques, &soleil_45, 800.0, 100.0, 80_000.0, 10_000.0, 0,
    );
    assert_proche!(net.drp_w_m2, brut.drp_w_m2 * 0.75);
    assert_proche!(net.dfp_w_m2, brut.dfp_w_m2 * 0.625_665_916_335_157);
    // Éq. (25) : le réfléchi n'est pas affaibli.
    assert_proche!(net.rrp_w_m2, brut.rrp_w_m2);
    assert_proche!(net.erp_lux, brut.erp_lux * 0.75);
    assert_proche!(net.errp_lux, brut.errp_lux);
}

#[test]
fn vent_voute_celeste_et_angle_de_profil() {
    assert_proche!(ventc_m_s(4.0), 3.6);
    // Éq. (17) : toiture horizontale, Teciel = −5 °C, Te = 10 °C : 5,5 × (−15) × 1 = −82,5 W/m².
    assert_proche!(qer_w_m2(&surface(SUD, 0.0, 1.0, 1.0), -5.0, 10.0), -82.5);
    // β = 60° : cos = 0,5 ; paroi verticale ou tournée vers le bas : nul.
    assert_proche!(qer_w_m2(&surface(SUD, 60.0, 1.0, 1.0), -5.0, 10.0), -41.25);
    assert!(qer_w_m2(&surface(SUD, 90.0, 1.0, 1.0), -5.0, 10.0).abs() < 1e-9);
    assert_proche!(qer_w_m2(&surface(SUD, 180.0, 1.0, 1.0), -5.0, 10.0), 0.0);
    // Éq. (4) : γ = 30°, Φ = 60° : arctan(tg(30°)/0,5) = arctan(1,1547005) = 49,10660535°.
    let s = surface(SUD, 90.0, 1.0, 1.0);
    assert_proche!(
        hauteur_soleil_corrigee_composant_deg(&s, &soleil(30.0, 60.0)),
        49.106_605_350_869_1
    );
    assert_proche!(
        hauteur_soleil_corrigee_composant_deg(&s, &soleil(30.0, 0.0)),
        30.0
    );
    assert_proche!(
        hauteur_soleil_corrigee_composant_deg(&s, &soleil(30.0, 90.0)),
        90.0
    );
    assert_proche!(
        hauteur_soleil_corrigee_composant_deg(&s, &soleil(30.0, -135.0)),
        90.0
    );
    // Φ est ramené dans [−π ; π[ : paroi à 170°, soleil à −170°, soit Φ = 20°.
    assert_proche!(
        phi_rad(&surface(170.0, 90.0, 1.0, 1.0), &soleil(30.0, -170.0)),
        20f64.to_radians()
    );
}

fn masque_quelconque() -> impl Strategy<Value = MasqueResolu> {
    let d = 0.0..5.0f64;
    prop_oneof![
        (d.clone(), d.clone())
            .prop_map(|(a, b)| MasqueResolu::ProcheVerticalGauche { dvg_m: a, dpg_m: b }),
        (d.clone(), d.clone())
            .prop_map(|(a, b)| MasqueResolu::ProcheVerticalDroit { dvd_m: a, dpd_m: b }),
        (d.clone(), d.clone())
            .prop_map(|(a, b)| MasqueResolu::ProcheHorizontal { dhm_m: a, dhp_m: b }),
        (0.0..200.0f64, 0.0..50.0f64)
            .prop_map(|(a, b)| MasqueResolu::LointainPlanVertical { de_m: a, hpe_m: b }),
        proptest::collection::vec(0.0..90.0f64, 36).prop_map(|g| MasqueResolu::depuis_masque(
            &Masque::LointainAzimutal { gamma_i_deg: g }
        )),
        proptest::collection::vec(0.0..90.0f64, 36).prop_map(|g| MasqueResolu::depuis_masque(
            &Masque::LointainArbresCaducs { gamma_i_deg: g }
        )),
    ]
}

proptest! {
    /// Invariants physiques : flux positifs ou nuls, direct borné par le direct normal, diffus
    /// borné par le diffus horizontal, réfléchi borné par l'albédo du global horizontal.
    #[test]
    fn les_flux_sont_bornes(
        azimut in 0.0..360.0f64, inclinaison in 0.0..=180.0f64,
        gamma in 0.0..90.0f64, psi in -180.0..180.0f64,
        idn in 0.0..1370.0f64, idi in 0.0..1370.0f64,
    ) {
        let f = flux_sans_masque(&surface(azimut, inclinaison, 1.0, 1.0), &soleil(gamma, psi), idn, idi, idn * 100.0, idi * 120.0);
        let eps = 1e-9;
        prop_assert!((0.0..=std::f64::consts::FRAC_PI_2).contains(&f.theta_rad));
        prop_assert!(f.drp_w_m2 >= -eps && f.drp_w_m2 <= idn + eps);
        prop_assert!(f.dfp_w_m2 >= -eps && f.dfp_w_m2 <= idi + eps);
        prop_assert!(f.rrp_w_m2 >= -eps && f.rrp_w_m2 <= ALB * (idn + idi) + eps);
        prop_assert!(f.erp_lux >= -eps && f.efp_lux >= -eps && f.errp_lux >= -eps);
    }

    /// Un masque n'augmente jamais un flux : chaque facteur est compris entre 0 et 1, et le
    /// flux atteignant la paroi ne dépasse pas le flux sans masque.
    #[test]
    fn un_masque_n_augmente_jamais_le_flux(
        masques in proptest::collection::vec(masque_quelconque(), 0..7),
        azimut in 0.0..360.0f64, verticale in any::<bool>(), inclinaison in 0.0..=180.0f64,
        lp in 0.1..10.0f64, hp in 0.1..10.0f64,
        gamma in 0.0..90.0f64, psi in -180.0..180.0f64, h in 0usize..8760,
    ) {
        let s = surface(azimut, if verticale { 90.0 } else { inclinaison }, lp, hp);
        let sol = soleil(gamma, psi);
        for m in &masques {
            let f = affaiblissement(m, &s, &sol, h);
            prop_assert!((0.0..=1.0).contains(&f.dir), "dir = {}", f.dir);
            prop_assert!((0.0..=1.0).contains(&f.dif), "dif = {}", f.dif);
        }
        let brut = flux_sans_masque(&s, &sol, 800.0, 200.0, 80_000.0, 24_000.0);
        let net = flux_atteignant_la_paroi(&s, &masques, &sol, 800.0, 200.0, 80_000.0, 24_000.0, h);
        prop_assert!(net.drp_w_m2 <= brut.drp_w_m2 + 1e-9);
        prop_assert!(net.dfp_w_m2 <= brut.dfp_w_m2 + 1e-9);
        prop_assert!(net.isr_w_m2() <= brut.isr_w_m2() + 1e-9);
        prop_assert_eq!(net.rrp_w_m2, brut.rrp_w_m2);
    }

    /// Monotonie : approfondir une casquette ne peut qu'ombrer davantage.
    #[test]
    fn une_casquette_plus_profonde_ombre_davantage(
        dhm in 0.01..3.0f64, plus in 0.0..3.0f64, dhp in 0.0..2.0f64, hp in 0.5..4.0f64,
        gamma in 0.0..89.0f64, psi in -89.0..89.0f64,
    ) {
        let s = surface(SUD, 90.0, 2.0, hp);
        let sol = soleil(gamma, psi);
        let courte = affaiblissement(&MasqueResolu::ProcheHorizontal { dhm_m: dhm, dhp_m: dhp }, &s, &sol, 0);
        let longue = affaiblissement(&MasqueResolu::ProcheHorizontal { dhm_m: dhm + plus, dhp_m: dhp }, &s, &sol, 0);
        prop_assert!(longue.dir <= courte.dir + 1e-12);
        prop_assert!(longue.dif <= courte.dif + 1e-12);
    }
}
