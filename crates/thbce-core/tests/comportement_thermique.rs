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
        cmq_surf_kj_m2k: 200.0,
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
    // Instantané de non-régression : valeurs calculées par le code pour θmq(h−1) = 19 °C et 500 W
    // émis sous forme convective (éq. 342 à 371). Elles incluent le terme Hges·θes dans l'éq. (366),
    // absent du texte imprimé (question Q34, point 6), et la lecture retenue des flux radiatifs
    // (question Q34, point 1). Recalcul indépendant : θmq,fin = 19,16533 °C (19,13053 °C avec
    // l'éq. 366 littérale). Ordre : θi, θs, θmq, θrm, θop.
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

#[test]
fn temperature_de_masse_avec_un_flux_d_inertie_sequentielle() {
    // Éq. (365), calculée à la main avec φmq_seq + φmq_ann = 1 000 W, à partir de l'équilibre à
    // 10 °C de `equilibre_sans_sollicitation` : sans autre apport, φmtot vaut
    // (Hgemq + U3)·10 °C, si bien que
    // θmq(h) = [10·(Cmq/3,6 − D) + 10·2D + 1 000]/(Cmq/3,6 + D) = 10 + 1 000/(Cmq/3,6 + D),
    // avec D = 0,5·(U3 + Hgemq).
    // Groupe de 100 m², inertie moyenne (tableau 53) : Amq = 2,5 × 100 = 250 m²,
    // Cmq = 165 × 100 = 16 500 kJ/K ; his = 2,5 + 1,2 × 5,5 = 9,1 W/(m².K) ;
    // Hgmqs = 9,1 × 250 = 2 275 W/K (éq. 326) ; Hgemq = 1/(1/49 − 1/2 275) (éq. 339) ;
    // Hgis = 4,5 × 100/(1/2,5 − 1/9,1) (éq. 323) ; Hgei = 0,04 × 1 006 = 40,24 W/K (éq. 333) ;
    // U1 = 1/(1/Hgei + 1/Hgis), U2 = U1 + 13, U3 = 1/(1/U2 + 1/Hgmqs) (éq. 362 à 364).
    let hg_mqs = 9.1 * 250.0;
    let hg_emq = 1.0 / (1.0 / 49.0 - 1.0 / hg_mqs);
    let hg_is = 450.0 / (1.0 / 2.5 - 1.0 / 9.1);
    let hg_ei = 0.04 * 1006.0;
    let u1 = 1.0 / (1.0 / hg_ei + 1.0 / hg_is);
    let u3 = 1.0 / (1.0 / (u1 + 13.0) + 1.0 / hg_mqs);
    let d = 0.5 * (u3 + hg_emq);
    let attendu: f64 = 10.0 + 1000.0 / (16_500.0 / 3.6 + d);
    // Ordre de grandeur : 1 kW pendant une heure sur 4,6 MJ/K, environ 0,22 K.
    assert!((attendu - 10.2158).abs() < 1e-4, "{attendu}");
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
    let r = groupe().resoudre(&s, 10.0, 1000.0, 0.0, 0.0).unwrap();
    assert_proche!(r.fin.theta_mq_c, attendu);
    // Température de masse moyenne du pas de temps (question Q34, point 3).
    assert_proche!(r.moy.theta_mq_c, (10.0 + attendu) / 2.0);
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
fn les_parts_radiatives_somment_a_un_dans_chaque_usage() {
    // Décision Q34. Trois répartitions, chacune complète.
    for (a_baies, inertie) in [
        (10.0, InertieQuotidienne::Moyenne),
        (0.0, InertieQuotidienne::TresLourde),
        (35.0, InertieQuotidienne::TresLegere),
    ] {
        let g = GroupeThermique::nouveau(100.0, a_baies, &inertie);
        let (at, amq, ab) = (g.a_tparois_m2, g.a_mq_m2, g.a_baies_m2);
        // 1. Flux solaire de courte longueur d'onde : fsa à l'air ; le reste entre les parois
        //    lourdes (frmd, éq. 352) et les parois légères (frld, éq. 351), rien sur les baies.
        let (fr_md, fr_ld) = (amq / (at - ab), (at - amq - ab) / (at - ab));
        assert_proche!(fr_md + fr_ld, 1.0);
        assert_proche!(F_SA + (1.0 - F_SA) * (fr_md + fr_ld), 1.0);
        // 2. Apports radiatifs internes : parois lourdes (frm, éq. 350), baies (frl,baies,
        //    éq. 349) et parois légères (frs).
        let (fr_m, fr_l_baies, fr_s) = (amq / at, ab / at, (at - amq - ab) / at);
        assert_proche!(fr_m + fr_l_baies + fr_s, 1.0);
        // 3. Puissance radiative du système d'émission (éq. 372) : θs et θmq seulement.
        assert_proche!(GroupeThermique::part_emission_noeud_s(fr_m) + fr_m, 1.0);
        // Les parts employées par le calcul : sans déperdition par les baies (Hges = 0), la part
        // reçue à θs est le complément de celle des parois lourdes, pour le solaire comme pour
        // les apports radiatifs.
        let (s_rad, s_sol) = g.parts_noeud_s(0.0, LectureFluxRadiatifs::Complete);
        assert_proche!(s_rad + fr_m, 1.0);
        assert_proche!(s_sol + fr_md, 1.0);
        // Avec des baies déperditives, l'éq. (361) retire Hges/(his·Atparois) de la part des baies.
        if ab > 0.0 {
            let (s_rad_13, _) = g.parts_noeud_s(13.0, LectureFluxRadiatifs::Complete);
            assert_proche!(s_rad - s_rad_13, 13.0 / (H_IS_W_M2K * at));
        }
    }
}

#[test]
fn la_puissance_radiative_emise_entre_entierement_dans_le_bilan() {
    // Éq. (372), frs = 1 − frm : groupe refroidi par l'air seul, 1 000 W émis sous forme
    // radiative ; en régime permanent, l'air emporte 1 000 W.
    let g = groupe();
    let s = Sollicitations {
        h_th_k_pt_w_k: 0.0,
        hg_es_w_k: 0.0,
        phi_sl_w: 0.0,
        phi_sh_w: 0.0,
        fs1_w: 0.0,
        fs3_w: 0.0,
        phi_conv_w: 0.0,
        phi_rad_w: 0.0,
        ..sollicitations()
    };
    let mut theta_mq = 15.0;
    let mut t = Temperatures::default();
    for _ in 0..20_000 {
        t = g.resoudre(&s, theta_mq, 0.0, 0.0, 1_000.0).unwrap().fin;
        theta_mq = t.theta_mq_c;
    }
    let emporte = s.air.hg_ei_w_k * (t.theta_i_c - s.air.theta_ei_eq_c);
    assert!((emporte - 1_000.0).abs() < 1e-6, "{emporte} W");
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
