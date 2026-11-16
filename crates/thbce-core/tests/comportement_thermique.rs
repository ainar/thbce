// SPDX-License-Identifier: GPL-3.0-or-later
//! Comportement thermique d'un groupe (Th-BCE 2020 §5.21) et ventilation du Bbio (§6.1).
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::bati::comportement_thermique::{
    inertie_quotidienne, AirEntrant, AirEquivalent, GroupeThermique, LectureFluxRadiatifs,
    Sollicitations, Temperatures, F_SA, H_IS_W_M2K, LECTURE_FLUX_RADIATIFS,
};
use thbce_core::entree::InertieQuotidienne;
use thbce_core::ventilation::ven_bbio::{h_vent_w_k, omega_air_souffle_kg_kg, t_air_souffle_c};

fn proche(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * (1.0 + b.abs())
}

macro_rules! assert_proche {
    ($a:expr, $b:expr) => {
        assert!(proche($a, $b), "{} obtenu, {} attendu", $a, $b)
    };
}

/// Groupe de 100 m², 10 m² de baies, inertie moyenne.
fn groupe() -> GroupeThermique {
    GroupeThermique::nouveau(100.0, 10.0, &InertieQuotidienne::Moyenne)
}

/// 0,04 kg/s d'air à 12 °C et 6 g/kg ; 5 °C dehors ; HTH,k+pt = 49 W/K ; Hges = 13 W/K ;
/// φsl = 100 W ; φsh = 50 W ; Fs1 = 800 W ; Fs3 = 20 W ; 300 W convectifs, 200 W radiatifs.
fn sollicitations() -> Sollicitations {
    Sollicitations {
        air: AirEquivalent::melange(&[AirEntrant {
            qm_kg_s: 0.04,
            theta_c: 12.0,
            omega_kg_kg: 0.006,
        }]),
        theta_e_c: 5.0,
        h_th_k_pt_w_k: 49.0,
        hg_es_w_k: 13.0,
        phi_sl_w: 100.0,
        phi_sh_w: 50.0,
        fs1_w: 800.0,
        fs3_w: 20.0,
        phi_conv_w: 300.0,
        phi_rad_w: 200.0,
    }
}

fn assert_temperatures(t: &Temperatures, attendu: [f64; 5]) {
    assert_proche!(t.theta_i_c, attendu[0]);
    assert_proche!(t.theta_s_c, attendu[1]);
    assert_proche!(t.theta_mq_c, attendu[2]);
    assert_proche!(t.theta_rm_c, attendu[3]);
    assert_proche!(t.theta_op_c, attendu[4]);
}

#[test]
fn parametres_du_modele() {
    // Tableau 53.
    assert_eq!(
        inertie_quotidienne(&InertieQuotidienne::TresLegere),
        (2.5, 80.0)
    );
    assert_eq!(
        inertie_quotidienne(&InertieQuotidienne::Legere),
        (2.5, 110.0)
    );
    assert_eq!(
        inertie_quotidienne(&InertieQuotidienne::Moyenne),
        (2.5, 165.0)
    );
    assert_eq!(
        inertie_quotidienne(&InertieQuotidienne::Lourde),
        (3.0, 260.0)
    );
    assert_eq!(
        inertie_quotidienne(&InertieQuotidienne::TresLourde),
        (3.5, 370.0)
    );
    let perso = InertieQuotidienne::Personnalisee {
        amq_surf_m2_m2: 2.8,
        cmq_surf_kj_k_m2: 200.0,
    };
    assert_eq!(inertie_quotidienne(&perso), (2.8, 200.0));
    // his = 2,5 + 1,2 × 5,5 = 9,1 W/(m².K). Pour 100 m² : Atparois = 450 m² ; Amq = 250 m² ;
    // Cmq = 16 500 kJ/K ; Hgis = 450/(1/2,5 − 1/9,1) = 1 551,136 W/K ; Hgmqs = 9,1 × 250 = 2 275 W/K.
    assert_proche!(H_IS_W_M2K, 9.1);
    let g = groupe();
    assert_eq!(
        (g.a_tparois_m2, g.a_mq_m2, g.c_mq_kj_k),
        (450.0, 250.0, 16_500.0)
    );
    assert_proche!(g.hg_is_w_k, 1_551.136_363_636_363_5);
    assert_proche!(g.hg_mqs_w_k, 2_275.0);
    // Éq. (339) : Hgemq = 1/(1/49 − 1/2275) = 50,0786 W/K ; nul sans paroi déperditive ; sans
    // objet si l'enveloppe conduit plus que l'échange superficiel.
    assert_proche!(g.hg_emq_w_k(49.0).unwrap(), 50.078_616_352_201_26);
    assert_eq!(g.hg_emq_w_k(0.0), Some(0.0));
    assert_eq!(g.hg_emq_w_k(2_275.0), None);
    assert!(g
        .resoudre(
            &Sollicitations {
                h_th_k_pt_w_k: 3_000.0,
                ..sollicitations()
            },
            19.0,
            0.0,
            0.0,
            0.0
        )
        .is_none());
}

#[test]
fn air_entrant_equivalent() {
    // Éq. (333) : un seul flux, Hgei = 0,04 × (1006 + 1830 × 0,006) = 40,6792 W/K ; θei,eq = 12 °C.
    let a = sollicitations().air;
    assert_proche!(a.hg_ei_w_k, 40.6792);
    assert_proche!(a.theta_ei_eq_c, 12.0);
    assert_proche!(a.omega_eq_kg_kg, 0.006);
    // Deux flux secs de même débit à 0 et 20 °C : 10 °C. Éq. (334) à (338).
    let sec = |theta_c| AirEntrant {
        qm_kg_s: 0.01,
        theta_c,
        omega_kg_kg: 0.0,
    };
    let m = AirEquivalent::melange(&[sec(0.0), sec(20.0)]);
    assert_proche!(m.qm_eq_kg_s, 0.02);
    assert_proche!(m.theta_ei_eq_c, 10.0);
    assert_proche!(m.hg_ei_w_k, 20.12);
    // Flux humides : 0,03 kg/s à 20 °C et 10 g/kg, 0,01 kg/s à 0 °C et 2 g/kg.
    // ωeq = (0,0003 + 0,00002)/0,04 = 0,008.
    // h1 = (1006 + 18,3) × 20 + 25 000 = 45 486 ; h2 = 5 000 ; heq = (1 364,58 + 50)/0,04 = 35 364,5.
    // θei,eq = (35 364,5 − 20 000)/(1006 + 14,64) = 15,0538 °C.
    let m = AirEquivalent::melange(&[
        AirEntrant {
            qm_kg_s: 0.03,
            theta_c: 20.0,
            omega_kg_kg: 0.010,
        },
        AirEntrant {
            qm_kg_s: 0.01,
            theta_c: 0.0,
            omega_kg_kg: 0.002,
        },
    ]);
    assert_proche!(m.omega_eq_kg_kg, 0.008);
    assert_proche!(m.theta_ei_eq_c, 15_364.5 / 1_020.64);
    assert_proche!(m.hg_ei_w_k, 0.04 * 1_020.64);
    // Sans débit.
    assert_eq!(AirEquivalent::melange(&[]), AirEquivalent::default());
}

#[test]
fn un_pas_de_temps_selon_les_equations_du_texte() {
    // Valeurs obtenues en appliquant littéralement les éq. (342) à (371), avec θes, θem et la
    // division par Hgei, θmq(h−1) = 19 °C et 500 W émis sous forme convective. Ordre : θi, θs,
    // θmq, θrm, θop. Lecture retenue des flux radiatifs (question Q34).
    let g = groupe();
    let r = g
        .resoudre(&sollicitations(), 19.0, 0.0, 500.0, 0.0)
        .unwrap();
    assert_temperatures(
        &r.moy,
        [
            19.848_998_616_424_41,
            19.474_622_108_274_36,
            19.082_665_190_807,
            19.332_812_824_884_186,
            19.590_905_720_654_3,
        ],
    );
    assert_temperatures(
        &r.fin,
        [
            19.927_729_579_244_968,
            19.555_417_823_529_233,
            19.165_330_381_613_998,
            19.414_390_643_333_878,
            19.671_060_111_289_42,
        ],
    );
    // Équations imprimées telles quelles : la part des parois légères n'est injectée nulle part,
    // les températures sont plus basses.
    let lit = g
        .resoudre_selon(
            LectureFluxRadiatifs::TexteLitteral,
            &sollicitations(),
            19.0,
            0.0,
            500.0,
            0.0,
        )
        .unwrap();
    assert_temperatures(
        &lit.moy,
        [
            19.643_779_426_401_35,
            19.264_020_959_146_674,
            19.040_972_862_751_88,
            19.120_173_054_883_54,
            19.381_976_240_642_445,
        ],
    );
    assert_temperatures(
        &lit.fin,
        [
            19.682_802_295_670_49,
            19.304_067_219_486_253,
            19.081_945_725_503_76,
            19.160_606_963_355_86,
            19.421_704_629_513_176,
        ],
    );
    assert_eq!(LECTURE_FLUX_RADIATIFS, LectureFluxRadiatifs::Complete);
}

#[test]
fn equilibre_sans_sollicitation() {
    // Tout à 10 °C, sans apport : le groupe reste à 10 °C.
    let s = Sollicitations {
        air: AirEquivalent::melange(&[AirEntrant {
            qm_kg_s: 0.04,
            theta_c: 10.0,
            omega_kg_kg: 0.0,
        }]),
        theta_e_c: 10.0,
        phi_sl_w: 0.0,
        phi_sh_w: 0.0,
        fs1_w: 0.0,
        fs3_w: 0.0,
        phi_conv_w: 0.0,
        phi_rad_w: 0.0,
        ..sollicitations()
    };
    let r = groupe().resoudre(&s, 10.0, 0.0, 0.0, 0.0).unwrap();
    assert_temperatures(&r.fin, [10.0; 5]);
    assert_temperatures(&r.moy, [10.0; 5]);
}

/// Fait tourner le modèle à sollicitations constantes jusqu'au régime permanent.
fn regime_permanent(
    g: &GroupeThermique,
    s: &Sollicitations,
    lecture: LectureFluxRadiatifs,
) -> Temperatures {
    let mut theta_mq = 15.0;
    let mut t = Temperatures::default();
    for _ in 0..20_000 {
        t = g
            .resoudre_selon(lecture, s, theta_mq, 0.0, 0.0, 0.0)
            .unwrap()
            .fin;
        theta_mq = t.theta_mq_c;
    }
    t
}

#[test]
fn bilan_des_trois_noeuds_en_regime_permanent() {
    // En régime permanent, la somme des flux est nulle à chacun des trois nœuds du réseau de la
    // figure 65. Les flux aux nœuds sont recalculés ici à partir des éq. (346) à (352) et (361).
    let (g, s) = (groupe(), sollicitations());
    let t = regime_permanent(&g, &s, LectureFluxRadiatifs::Complete);
    let (at, amq, ab) = (g.a_tparois_m2, g.a_mq_m2, g.a_baies_m2);
    let clo = (1.0 - F_SA) * s.fs1_w;
    let phi_i = F_SA * s.fs1_w + s.fs3_w + s.phi_conv_w;
    let phi_mq = amq / (at - ab) * clo + amq / at * s.phi_rad_w;
    let phi_s = (1.0 - s.hg_es_w_k / (ab * H_IS_W_M2K)) * (ab / at * s.phi_rad_w)
        + (at - amq - ab) / at * s.phi_rad_w
        + (at - amq - ab) / (at - ab) * clo;
    let hg_emq = g.hg_emq_w_k(s.h_th_k_pt_w_k).unwrap();
    let noeud_i = s.air.hg_ei_w_k * (s.air.theta_ei_eq_c - t.theta_i_c)
        + g.hg_is_w_k * (t.theta_s_c - t.theta_i_c)
        + phi_i;
    let noeud_s = g.hg_is_w_k * (t.theta_i_c - t.theta_s_c)
        + g.hg_mqs_w_k * (t.theta_mq_c - t.theta_s_c)
        + s.hg_es_w_k * (s.theta_e_c - t.theta_s_c)
        + s.phi_sl_w
        + phi_s;
    let noeud_mq = g.hg_mqs_w_k * (t.theta_s_c - t.theta_mq_c)
        + hg_emq * (s.theta_e_c - t.theta_mq_c)
        + s.phi_sh_w * hg_emq / s.h_th_k_pt_w_k
        + phi_mq;
    for (nom, bilan) in [("θi", noeud_i), ("θs", noeud_s), ("θmq", noeud_mq)] {
        assert!(bilan.abs() < 1e-6, "nœud {nom} : {bilan} W");
    }
}

#[test]
fn fermeture_du_bilan_d_energie_selon_la_lecture() {
    // Groupe sans baie ni paroi déperditive, refroidi par l'air seul : en régime permanent, l'air
    // emporte exactement les apports qui entrent dans le bilan.
    let g = GroupeThermique::nouveau(100.0, 0.0, &InertieQuotidienne::Moyenne);
    let s = Sollicitations {
        h_th_k_pt_w_k: 0.0,
        hg_es_w_k: 0.0,
        phi_sl_w: 0.0,
        phi_sh_w: 0.0,
        fs1_w: 0.0,
        fs3_w: 0.0,
        phi_conv_w: 300.0,
        phi_rad_w: 450.0,
        ..sollicitations()
    };
    let emporte = |t: &Temperatures| s.air.hg_ei_w_k * (t.theta_i_c - s.air.theta_ei_eq_c);
    // Lecture retenue : les 750 W d'apports sont évacués.
    let t = regime_permanent(&g, &s, LectureFluxRadiatifs::Complete);
    assert!((emporte(&t) - 750.0).abs() < 1e-6);
    // Équations imprimées : des 450 W radiatifs, seule la part des parois lourdes,
    // Amq/Atparois = 250/450, entre dans le bilan : 300 + 250 = 550 W. 200 W disparaissent.
    let t = regime_permanent(&g, &s, LectureFluxRadiatifs::TexteLitteral);
    assert!((emporte(&t) - 550.0).abs() < 1e-6);
}

#[test]
fn matrice_de_comportement() {
    // §5.21.3.14 : évolution libre, 10 W/m² convectifs, 10 W/m² radiatifs (1 000 W pour 100 m²).
    let (g, s) = (groupe(), sollicitations());
    let [libre, conv, rad] = g.matrice(&s, 19.0, 0.0).unwrap();
    assert_eq!(conv, g.resoudre(&s, 19.0, 0.0, 1_000.0, 0.0).unwrap());
    assert_eq!(rad, g.resoudre(&s, 19.0, 0.0, 0.0, 1_000.0).unwrap());
    // Une émission réchauffe le groupe ; l'émission convective agit d'abord sur l'air,
    // l'émission radiative sur les surfaces.
    assert!(conv.fin.theta_i_c > libre.fin.theta_i_c && rad.fin.theta_i_c > libre.fin.theta_i_c);
    assert!(conv.fin.theta_i_c > rad.fin.theta_i_c);
    assert!(rad.fin.theta_mq_c > conv.fin.theta_mq_c);
    assert!(rad.fin.theta_rm_c > conv.fin.theta_rm_c);
    // Le modèle est linéaire en puissance émise : la réponse à 500 W est à mi-chemin.
    let demi = g.resoudre(&s, 19.0, 0.0, 500.0, 0.0).unwrap();
    assert_proche!(
        demi.fin.theta_op_c,
        0.5 * (libre.fin.theta_op_c + conv.fin.theta_op_c)
    );
}

#[test]
fn ventilation_conventionnelle_du_bbio() {
    // Éq. (378) à (380), ε = 0,5 : 0 °C dehors, 20 °C dedans au pas précédent → 10 °C soufflés.
    assert_proche!(t_air_souffle_c(0.0, 20.0), 10.0);
    assert_proche!(t_air_souffle_c(30.0, 26.0), 28.0);
    // Éq. (381) et (382).
    assert_eq!(omega_air_souffle_kg_kg(0.004), 0.004);
    assert_proche!(h_vent_w_k(0.04), 20.12);
}
