// SPDX-License-Identifier: GPL-3.0-or-later
//! Surventilation naturelle par ouverture des baies (Th-BCE 2020 §5.13) : valeurs calculées à la
//! main.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::bati::surventilation::{
    a_ouv_baie_m2, c_pr, mod_ext, mod_ext_man, qm_aj_kg_s, qv_surv_m3_h, r_ouv_manuelle,
    r_ouv_max_par_defaut, r_ouv_theta_op_man, r_ouv_theta_op_man_selon, rho_ext_kg_m3, s_ors_m2,
    seuils_ouverture, traversant_conventionnel, ConditionsOuverture, LectureHysteresis,
    OuvertureBaie, LECTURE_HYSTERESIS,
};
use thbce_core::conventions::ModeCalcul;
use thbce_core::entree::{ExpoBr, TypeOuvrant};

fn proche(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * (1.0 + b.abs())
}

macro_rules! assert_proche {
    ($a:expr, $b:expr) => {
        assert!(proche($a, $b), "{} obtenu, {} attendu", $a, $b)
    };
}

/// Th-D, zone occupée à 15 h, hors saison de chauffage ; consigne de refroidissement de 26 °C,
/// sans écart de confort adaptatif ; 20 °C dehors ; température opérative en hausse à 24 °C.
fn conditions() -> ConditionsOuverture {
    ConditionsOuverture {
        mode: ModeCalcul::ThD,
        i_occ_zone: true,
        hleg: 15,
        saison_de_chauffage: false,
        refroidissement_autorise: true,
        theta_ext_c: 20.0,
        theta_op_h1_c: 24.0,
        theta_op_h2_c: 23.5,
        theta_iifr_c: 26.0,
        delta_theta_conf_adapt_c: 0.0,
    }
}

#[test]
fn donnees_conventionnelles() {
    // Tableau 44.
    assert_eq!(r_ouv_max_par_defaut(TypeOuvrant::FrancaiseOuAnglaise), 0.80);
    assert_eq!(r_ouv_max_par_defaut(TypeOuvrant::ItalienneOuSoufflet), 0.40);
    assert_eq!(
        r_ouv_max_par_defaut(TypeOuvrant::SouffletOscilloBattant),
        0.20
    );
    assert_eq!(r_ouv_max_par_defaut(TypeOuvrant::Coulissant), 0.40);
    // Tableau 43 : maison traversante par convention ; logement collectif selon le groupe.
    assert_eq!(traversant_conventionnel(1), Some(true));
    assert_eq!(traversant_conventionnel(2), None);
}

#[test]
fn coefficient_cpr() {
    // Tableau 42.
    assert_eq!(c_pr(ExpoBr::Br1, false, false, 15), 0.0);
    for expo in [ExpoBr::Br1, ExpoBr::Br2, ExpoBr::Br3] {
        assert_eq!(c_pr(expo, true, true, 15), 0.3);
        assert_eq!(c_pr(expo, true, true, 2), 0.3);
    }
    assert_eq!(c_pr(ExpoBr::Br1, true, false, 15), 1.0);
    assert_eq!(c_pr(ExpoBr::Br1, true, false, 2), 1.0);
    // Journée de 7 h à 21 h, nuit de 22 h à 6 h.
    for (hleg, attendu) in [(6, 0.3), (7, 0.7), (21, 0.7), (22, 0.3), (24, 0.3)] {
        assert_eq!(
            c_pr(ExpoBr::Br2, true, false, hleg),
            attendu,
            "hlég = {hleg}"
        );
        assert_eq!(c_pr(ExpoBr::Br3, true, false, hleg), attendu);
    }
}

#[test]
fn moderation_selon_la_temperature_exterieure() {
    // Figure 61, seuils de 10 et 18 °C, limite haute de 28 °C.
    let m = |t| mod_ext(t, 10.0, 18.0, 28.0);
    assert_eq!(m(5.0), 0.0);
    assert_eq!(m(10.0), 0.0);
    assert_proche!(m(14.0), 0.5);
    assert_eq!(m(18.0), 1.0);
    assert_eq!(m(28.0), 1.0);
    assert_eq!(m(28.1), 0.0);
    // Éq. (241) : Δθext_int = −3 °C ; θop(h−1) = 24 °C, donc ouverture jusqu'à 27 °C dehors.
    let c = |t| ConditionsOuverture {
        theta_ext_c: t,
        ..conditions()
    };
    assert_eq!(mod_ext_man(&c(27.0)), 1.0);
    assert_eq!(mod_ext_man(&c(27.5)), 0.0);
    assert_proche!(mod_ext_man(&c(12.0)), 0.25);
}

#[test]
fn moderation_en_th_b() {
    let b = |t, autorise| ConditionsOuverture {
        mode: ModeCalcul::ThB,
        theta_ext_c: t,
        refroidissement_autorise: autorise,
        ..conditions()
    };
    // Éq. (244) : hors période d'autorisation de refroidissement.
    assert_eq!(mod_ext_man(&b(20.0, false)), 0.0);
    // Consigne de 26 °C : la limite haute devient min(25 ; 24 + 3) = 25 °C.
    assert_eq!(mod_ext_man(&b(25.0, true)), 1.0);
    assert_eq!(mod_ext_man(&b(25.5, true)), 0.0);
    // Consigne de 18 °C : le seuil haut devient min(17 ; 18) = 17 °C ; à 13,5 °C, (13,5 − 10)/7.
    let froid = ConditionsOuverture {
        theta_iifr_c: 18.0,
        ..b(13.5, true)
    };
    assert_proche!(mod_ext_man(&froid), 0.5);
}

#[test]
fn seuils_et_hysteresis() {
    // Éq. (242) : θouv2 = 26 + 0 − 1 = 25 ; θouv1 = θfer1 = 23 ; θfer2 = 22.
    let s = seuils_ouverture(&conditions());
    assert_eq!(
        (s.ouv1_c, s.ouv2_c, s.fer1_c, s.fer2_c),
        (23.0, 25.0, 23.0, 22.0)
    );
    // Éq. (243), saison de chauffage : θouv2 = 27. Confort adaptatif de 2 °C : θouv2 = 27.
    let chauffage = ConditionsOuverture {
        saison_de_chauffage: true,
        ..conditions()
    };
    assert_eq!(seuils_ouverture(&chauffage).ouv2_c, 27.0);
    let adaptatif = ConditionsOuverture {
        delta_theta_conf_adapt_c: 2.0,
        ..conditions()
    };
    assert_eq!(seuils_ouverture(&adaptatif).ouv2_c, 27.0);

    // Figure 62. En hausse à 24 °C : (24 − 23)/(25 − 23) = 0,5.
    assert_proche!(r_ouv_theta_op_man(0.0, &conditions()), 0.5);
    let c = |h1, h2| ConditionsOuverture {
        theta_op_h1_c: h1,
        theta_op_h2_c: h2,
        ..conditions()
    };
    assert_eq!(r_ouv_theta_op_man(0.0, &c(22.9, 22.0)), 0.0);
    assert_eq!(r_ouv_theta_op_man(0.0, &c(25.5, 25.0)), 1.0);
    // En baisse : ouvert jusqu'à 23 °C, puis (22,5 − 22)/(23 − 22) = 0,5, fermé à 22 °C.
    assert_eq!(r_ouv_theta_op_man(1.0, &c(24.0, 24.5)), 1.0);
    assert_proche!(r_ouv_theta_op_man(1.0, &c(22.5, 23.0)), 0.5);
    assert_eq!(r_ouv_theta_op_man(1.0, &c(21.9, 22.5)), 0.0);
    // Lectures de la question Q32 : température stable, ou hors de la plage 8 h – 22 h : le
    // ratio précédent est conservé.
    assert_eq!(r_ouv_theta_op_man(0.3, &c(24.0, 24.0)), 0.3);
    for hleg in [7, 23, 24, 3] {
        let nuit = ConditionsOuverture {
            hleg,
            ..conditions()
        };
        assert_eq!(r_ouv_theta_op_man(0.3, &nuit), 0.3, "hlég = {hleg}");
    }
    for hleg in [8, 22] {
        let jour = ConditionsOuverture {
            hleg,
            ..conditions()
        };
        assert_proche!(r_ouv_theta_op_man(0.3, &jour), 0.5);
    }
    // Éq. (245) : en Th-B, nul en inoccupation ; en Th-D, conservé.
    let inoccupe = ConditionsOuverture {
        i_occ_zone: false,
        ..conditions()
    };
    assert_eq!(r_ouv_theta_op_man(0.3, &inoccupe), 0.3);
    let th_b = ConditionsOuverture {
        mode: ModeCalcul::ThB,
        ..inoccupe
    };
    assert_eq!(r_ouv_theta_op_man(0.3, &th_b), 0.0);
}

#[test]
fn difference_entre_les_deux_lectures_de_l_hysteresis() {
    // Décision Q32. Seuils : θfer2 = 22, θouv1 = θfer1 = 23, θouv2 = 25.
    // La température descend de 24,5 à 22,8 °C : branche de fermeture, (22,8 − 22)/1 = 0,8.
    // Elle repart ensuite légèrement à la hausse, à 22,9 °C, à l'intérieur de la bande.
    let c = |h1, h2| ConditionsOuverture {
        theta_op_h1_c: h1,
        theta_op_h2_c: h2,
        ..conditions()
    };
    let sens = LectureHysteresis::SensDeVariation;
    let bande = LectureHysteresis::BandeAMemoire;
    assert_eq!(LECTURE_HYSTERESIS, sens);
    for lecture in [sens, bande] {
        assert_proche!(r_ouv_theta_op_man_selon(lecture, 1.0, &c(22.8, 24.5)), 0.8);
    }
    // Lecture retenue : la hausse fait passer sur la branche d'ouverture, nulle en dessous de
    // θouv1 = 23 °C ; le ratio redescend de 0,8 à 0.
    assert_eq!(r_ouv_theta_op_man(0.8, &c(22.9, 22.8)), 0.0);
    assert_eq!(r_ouv_theta_op_man_selon(sens, 0.8, &c(22.9, 22.8)), 0.0);
    // Variante à mémoire : 0,8 reste entre la branche d'ouverture (0) et la branche de fermeture
    // (0,9) ; le ratio est conservé.
    assert_proche!(r_ouv_theta_op_man_selon(bande, 0.8, &c(22.9, 22.8)), 0.8);
    // Hors de la bande, les deux lectures se rejoignent.
    for (h1, h2, attendu) in [(25.5, 25.0, 1.0), (21.5, 22.0, 0.0)] {
        for lecture in [sens, bande] {
            assert_eq!(r_ouv_theta_op_man_selon(lecture, 0.8, &c(h1, h2)), attendu);
        }
    }
    // La variante suit la branche d'ouverture en montée depuis zéro et la branche de fermeture
    // en descente depuis un : (24 − 23)/2 = 0,5 et (22,5 − 22)/1 = 0,5.
    assert_proche!(r_ouv_theta_op_man_selon(bande, 0.0, &c(24.0, 23.5)), 0.5);
    assert_proche!(r_ouv_theta_op_man_selon(bande, 1.0, &c(22.5, 23.0)), 0.5);
}

#[test]
fn ratio_d_ouverture_et_surface_equivalente() {
    // Éq. (240) : Pocc = 0,5 ; Cpr = 0,7 (BR2, journée) ; Mod_ext = 1 ; Rouv_θop = 0,5 → 0,175.
    let c = conditions();
    assert_proche!(r_ouv_manuelle(&c, 0.5, ExpoBr::Br2, 0.5), 0.175);
    let inoccupe = ConditionsOuverture {
        i_occ_zone: false,
        ..c
    };
    assert_eq!(r_ouv_manuelle(&inoccupe, 0.5, ExpoBr::Br2, 0.5), 0.0);
    // Baie de 2 m² à la française : Aouvmax = 1,6 m². Volet à moitié fermé, taux de passage de
    // 10 % : Aouvmax_ac_PM = 1,6 × (0,5 × 0,1 + 0,5) = 0,88 m² (éq. 239).
    // Éq. (248) : Rouv_corrige = min(1 ; 0,175 × 1,6/0,88) = 0,3182 ; éq. (249) : Aouv = 0,28 m²,
    // c'est-à-dire Rouv × Aouvmax tant que le plafond n'est pas atteint.
    assert_proche!(a_ouv_baie_m2(0.175, 1.6, 0.88), 0.28);
    // Plafond : Rouv = 0,9 → Rouv_corrige = 1, Aouv = 0,88 m².
    assert_proche!(a_ouv_baie_m2(0.9, 1.6, 0.88), 0.88);
    // Protection étanche entièrement fermée : rien ne passe.
    assert_eq!(a_ouv_baie_m2(0.5, 1.6, 0.0), 0.0);
}

fn ouverture(a: f64, alpha: f64, beta: f64) -> OuvertureBaie {
    OuvertureBaie {
        a_ouv_m2: a,
        alpha_deg: alpha,
        beta_deg: beta,
    }
}

#[test]
fn surface_equivalente_traversante() {
    // Éq. (252) : 1 m² au sud et 1 m² au nord, Aouv = 2 m². Chaque secteur :
    // 1/√(1/1² + 1/1²) = 0,7071 ; Sors = ¼ × 1,4142 = 0,35355 m², pour les deux distributions.
    let sud_nord = [ouverture(1.0, 0.0, 90.0), ouverture(1.0, 180.0, 90.0)];
    assert_proche!(s_ors_m2(&sud_nord), 0.353_553_390_593_273_73);
    // Avec 2 m² de fenêtres de toit, comptées dans Aouv seulement (Aouv = 4 m²) :
    // 1/√(1 + 1/9) = 0,9487 par secteur ; Sors = ¼ × 1,8974 = 0,47434 m².
    let avec_toit = [
        ouverture(1.0, 0.0, 90.0),
        ouverture(1.0, 180.0, 90.0),
        ouverture(2.0, 0.0, 30.0),
    ];
    assert_proche!(s_ors_m2(&avec_toit), 0.474_341_649_025_256_9);
    // Exemple du texte (p. 270) : deux baies à 40° et 80°. Distribution S-O-N-E : deux secteurs.
    // Distribution décalée de 45° : un seul secteur, terme nul. Le minimum est retenu : 0.
    let voisines = [ouverture(1.0, 40.0, 90.0), ouverture(1.0, 80.0, 90.0)];
    assert_eq!(s_ors_m2(&voisines), 0.0);
    // Bornes des secteurs : 315° (soit −45°) appartient au secteur sud, 45° au secteur ouest.
    let bornes = [ouverture(1.0, 315.0, 90.0), ouverture(1.0, 44.0, 90.0)];
    assert_eq!(s_ors_m2(&bornes), 0.0);
    // Une seule façade, ou aucune baie : pas d'effet traversant.
    assert_eq!(s_ors_m2(&[ouverture(3.0, 0.0, 90.0)]), 0.0);
    assert_eq!(s_ors_m2(&[]), 0.0);
}

#[test]
fn debit_de_surventilation() {
    // Aouv = 2 m², Httf = 1,5 m, 26 °C dehors et 30 °C dedans : Cst·Httf·|Δθ| = 0,0035 × 1,5 × 4
    // = 0,021.
    // Non traversant, éq. (251). Vent de 2 m/s : Cw·V² = 0,004 < 0,021 →
    // 3600 × 1 × √0,021 = 521,69 m³/h. Vent de 5 m/s : 0,025 → 3600 × √0,025 = 569,21 m³/h.
    assert_proche!(
        qv_surv_m3_h(2.0, None, 2.0, 1.5, 26.0, 30.0),
        521.689_562_862_819_8
    );
    assert_proche!(
        qv_surv_m3_h(2.0, None, 5.0, 1.5, 26.0, 30.0),
        569.209_978_830_308_3
    );
    // Traversant, éq. (254), Sors = 0,35355 m². Vent de 2 m/s :
    // 0,6 × 0,35355 × 2 × √0,75 = 0,36742 > 1 × √0,021 = 0,14491 → 1 322,72 m³/h.
    // Vent de 5 m/s, plafonné à 3 m/s : 1 984,09 m³/h.
    let s = Some(0.353_553_390_593_273_73);
    assert_proche!(
        qv_surv_m3_h(2.0, s, 2.0, 1.5, 26.0, 30.0),
        1_322.724_461_102_916
    );
    assert_proche!(
        qv_surv_m3_h(2.0, s, 5.0, 1.5, 26.0, 30.0),
        1_984.086_691_654_374
    );
    // Sans vent, le tirage thermique reste : même débit que dans le cas non traversant.
    assert_proche!(
        qv_surv_m3_h(2.0, s, 0.0, 1.5, 26.0, 30.0),
        521.689_562_862_819_8
    );
    // Éq. (255) : ρext(30 °C) = 1,2 × 292/303 = 1,15644 kg/m³ ; ρext(19 °C) = 1,2.
    assert_proche!(rho_ext_kg_m3(30.0), 1.156_435_643_564_356_4);
    assert_proche!(rho_ext_kg_m3(19.0), 1.2);
    // Éq. (256) : maximum du débit de surventilation et du débit d'aération.
    assert_proche!(qm_aj_kg_s(3_600.0, 19.0, 0.5), 1.2);
    assert_proche!(qm_aj_kg_s(3_600.0, 19.0, 2.0), 2.0);
}
