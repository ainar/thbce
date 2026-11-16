// SPDX-License-Identifier: GPL-3.0-or-later
//! Bilan aéraulique d'une zone (Th-BCE 2020 §5.6) : valeurs calculées à la main d'après les
//! équations.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{AutoreglageEntreesAir, EntreesAir, Projet, UsageZone};
use thbce_core::valider;
use thbce_core::ventilation::debits_air_cep::{
    altitudes, coefficient_entree_air, coefficient_permeabilite, coefficients_pression,
    delta_p_comp_pa, flux_entrants, h_moy_m, hall_fictif, permea_verticale, q_v_def, rho_kg_m3,
    trav_zone_conventionnel, EntreeAir, EtatGroupe, Exterieur, GroupeAeraulique, LectureAltitudes,
    ZoneAeraulique,
};

const ENTREE: &str = include_str!("../../../tests/cas/squelette-maison/entree.json");

fn projet() -> Projet {
    serde_json::from_str(ENTREE).unwrap()
}

fn erreurs(p: &Projet) -> Vec<String> {
    valider(p).erreurs.into_iter().map(|e| e.chemin).collect()
}

fn proche(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() <= tol, "{a} au lieu de {b}");
}

fn groupe(a_f: f64, a_t: f64, ea: Option<EntreeAir>) -> GroupeAeraulique {
    GroupeAeraulique {
        a_f_m2: a_f,
        a_t_m2: a_t,
        q4pa_surf_m3_h_m2: 0.6,
        entree_air: ea,
    }
}

fn etat(theta: f64, repris: f64, souffle: f64) -> EtatGroupe {
    EtatGroupe {
        theta_i_fin_c: theta,
        omega_i_fin_kg_kg: 0.0,
        q_spec_repris_m3_h: repris,
        q_spec_souffle_m3_h: souffle,
        t_air_souffle_c: theta,
        omega_air_souffle_kg_kg: 0.0,
        theta_an_c: None,
    }
}

fn exterieur(theta: f64, vent: f64) -> Exterieur {
    Exterieur {
        theta_ext_c: theta,
        omega_e_kg_kg: 0.0,
        v_ventc_m_s: vent,
    }
}

/// Maison traversante d'un seul groupe, de plain-pied : plancher au niveau du sol, 2,5 m de haut.
fn maison(g: GroupeAeraulique) -> ZoneAeraulique {
    ZoneAeraulique::nouvelle(&[g], true, true, 0.0, 0.0, 2.5)
}

#[test]
fn masse_volumique_de_l_air() {
    // Éq. (131) : ρ = 1,2 × 292/(273 + θ) × (1 + ω)/(1 + ω × 28,976/18).
    assert_eq!(rho_kg_m3(19.0, 0.0), 1.2);
    proche(rho_kg_m3(0.0, 0.0), 1.2 * 292.0 / 273.0, 1e-15);
    proche(rho_kg_m3(0.0, 0.0), 1.283_516_483_5, 1e-9);
    // ω = 0,005 kg/kg : facteur 1,005/(1 + 0,005 × 1,609 777…) = 1,005/1,008 048 888… = 0,996 975…
    proche(
        rho_kg_m3(19.0, 0.005),
        1.2 * 1.005 / (1.0 + 0.005 * 28.976 / 18.0),
        1e-15,
    );
    proche(rho_kg_m3(19.0, 0.005), 1.196_370_546, 1e-8);
}

#[test]
fn difference_de_pression_d_un_composant() {
    // Éq. (135) à (137), Cp = 0,25, ρext = 1,25, V = 4 m/s, z = 2 m, ρint = 1,2, Pib = −1 Pa :
    // Pext = 0,5 × 0,25 × 1,25 × 16 − 9,81 × 2 × 1,25 = 2,5 − 24,525 = −22,025 ;
    // Pint = −1 − 1,2 × 9,81 × 2 = −24,544 ; ΔP = 2,519 Pa.
    proche(
        delta_p_comp_pa(0.25, 1.25, 4.0, 2.0, 1.2, -1.0),
        2.519,
        1e-12,
    );
}

#[test]
fn caractere_traversant_et_coefficients_de_pression() {
    // Tableau 22.
    assert_eq!(trav_zone_conventionnel(1), Some(true));
    assert_eq!(trav_zone_conventionnel(2), None);
    assert_eq!(trav_zone_conventionnel(3), Some(true));
    for usage in [8, 9, 19] {
        assert_eq!(trav_zone_conventionnel(usage), Some(false));
    }
    assert_eq!(trav_zone_conventionnel(20), Some(true));
    // Tableau 23, écrantage normal, toit incliné de plus de 80° par rapport à la verticale.
    let c = coefficients_pression(true, 5.0);
    assert_eq!((c.c_p_v, c.c_p_s, c.c_p_t), (0.25, -0.50, -0.60));
    let c = coefficients_pression(true, 30.0);
    assert_eq!((c.c_p_v, c.c_p_s, c.c_p_t), (0.45, -0.50, -0.60));
    // Au-delà de 50 m, seule ligne du tableau : site ouvert.
    let c = coefficients_pression(true, 60.0);
    assert_eq!((c.c_p_v, c.c_p_s, c.c_p_t), (0.80, -0.70, -0.70));
    // Tableau 24.
    let c = coefficients_pression(false, 5.0);
    assert_eq!((c.c_p_v, c.c_p_s, c.c_p_t), (0.05, -0.05, 0.0));
    // Hauteur moyenne : milieu de la zone au-dessus du sol.
    assert_eq!(h_moy_m(3.0, 6.0), 6.0);
}

#[test]
fn permeabilite_verticale_et_altitudes() {
    // Éq. (138).
    assert!(!permea_verticale(true, 10.0));
    assert!(!permea_verticale(false, 2.9));
    assert!(permea_verticale(false, 3.0));
    // Sans perméabilité verticale, z = 100 m, plancher à 1 m du sol, zone de 5 m :
    // min(5, 3) = 3 ; zb = 101 + 0,75 ; zh = 101 + 2,25 ; zt = 104 ; entrées d'air à une altitude
    // unique, 101 + 0,92 × 3 = 103,76.
    let a = altitudes(100.0, 1.0, 5.0, false);
    proche(a.z_b_m, 101.75, 1e-12);
    proche(a.z_h_m, 103.25, 1e-12);
    proche(a.z_t_m, 104.0, 1e-12);
    proche(a.z_ea_b_m, 103.76, 1e-12);
    assert_eq!(a.z_ea_b_m, a.z_ea_h_m);
    // Avec perméabilité verticale, zone de 20 m : min(20, 15) = 15 ; zb = 3,75 ; zh = 11,25 ;
    // zt = 15 ; entrées d'air aux mêmes altitudes basse et haute.
    let a = altitudes(0.0, 0.0, 20.0, true);
    assert_eq!((a.z_b_m, a.z_h_m, a.z_t_m), (3.75, 11.25, 15.0));
    assert_eq!((a.z_ea_b_m, a.z_ea_h_m), (3.75, 11.25));
    // Zone basse : hzone = 2,5 m.
    let a = altitudes(0.0, 0.0, 2.5, false);
    assert_eq!((a.z_b_m, a.z_h_m, a.z_t_m), (0.625, 1.875, 2.5));
    proche(a.z_ea_b_m, 2.3, 1e-12);
}

#[test]
fn caracteristique_d_une_entree_d_air() {
    // §5.6.3.2.1 : A = 10⁻⁴ × M × (10/20)^0,5 ; C = 0,68 × (2/1,2)^0,5 × A.
    // M = 30 m³/h : C = 0,68 × 1,290 994 × 0,003 × 0,707 107 = 1,862 257 × 10⁻³ m³/s sous 1 Pa.
    let c = coefficient_entree_air(30.0);
    proche(c, 1.862_257e-3, 1e-9);
    // Entrée fixe : sous 20 Pa, C × 20^0,5 = 8,328 265 × 10⁻³ m³/s, soit 29,98 m³/h : le module.
    let fixe = EntreeAir::nouvelle(30.0, None, 1.0);
    proche(fixe.q_v_m3_s(20.0), 8.328_265e-3, 1e-9);
    proche(3600.0 * fixe.q_v_m3_s(20.0), 29.98, 0.01);
    // Sans anti-retour, la caractéristique est impaire (éq. 146 avec r = 1).
    proche(fixe.q_v_m3_s(-20.0), -fixe.q_v_m3_s(20.0), 1e-18);
    assert_eq!(fixe.q_v_m3_s(0.0), 0.0);
    assert!(fixe.est_monotone());
    // Autoréglable : ΔP1 = 20 Pa, ΔP2 = 100 Pa, M′ = 15 m³/h, donc C′ = C/2.
    // qv,1 = C × 20^0,5 = 4,472 136 C ; qv,2 = C′ × 100^0,5 = 5 C.
    // Sous 60 Pa, milieu du tronçon linéaire : (4,472 136 + 5)/2 × C = 4,736 068 C (éq. 145).
    // Sous 400 Pa : C′ × 20 = 10 C (éq. 142). Anti-retour r = 0,3 : −0,3 × C × 20^0,5 sous −20 Pa.
    let ar = EntreeAir::nouvelle(30.0, Some((20.0, 100.0, 15.0)), 0.3);
    proche(ar.q_v_m3_s(5.0), c * 5.0_f64.sqrt(), 1e-15);
    proche(ar.q_v_m3_s(20.0), c * 20.0_f64.sqrt(), 1e-15);
    proche(ar.q_v_m3_s(60.0), 4.736_068 * c, 1e-8);
    proche(ar.q_v_m3_s(100.0), 5.0 * c, 1e-15);
    proche(ar.q_v_m3_s(400.0), 10.0 * c, 1e-15);
    proche(ar.q_v_m3_s(-20.0), -0.3 * c * 20.0_f64.sqrt(), 1e-15);
    assert!(ar.est_monotone());
    // M′ = 12 : qv,2 = 4 C < qv,1 : la caractéristique décroîtrait, ce que le texte interdit.
    assert!(!EntreeAir::nouvelle(30.0, Some((20.0, 100.0, 12.0)), 1.0).est_monotone());
}

#[test]
fn debit_d_un_defaut_d_etancheite() {
    // Éq. (150) et (151) : sous 4 Pa, le débit vaut A × Q4Pa_surf. 100 m² à 0,6 m³/(h.m²) :
    // 60 m³/h ; sous 32 Pa, 60 × 8^(2/3) = 240 m³/h ; sous −4 Pa, −60 m³/h.
    let c = coefficient_permeabilite(100.0, 0.6);
    proche(q_v_def(c, 4.0), 60.0, 1e-12);
    proche(q_v_def(c, 32.0), 240.0, 1e-10);
    proche(q_v_def(c, -4.0), -60.0, 1e-12);
    assert_eq!(q_v_def(c, 0.0), 0.0);
}

#[test]
fn repos_sans_vent_ni_ecart_de_temperature() {
    // Air intérieur et extérieur à 19 °C, pas de vent, double flux équilibrée : aucune
    // différence de pression, aucun débit par l'enveloppe.
    let z = maison(groupe(100.0, 100.0, None));
    let b = z
        .bilan(&exterieur(19.0, 0.0), &[etat(19.0, -111.25, 111.25)])
        .unwrap();
    proche(b.p_ib_pa, 0.0, 1e-9);
    let g = &b.groupes[0];
    proche(g.q_m_neuf_kg_s, 0.0, 1e-9);
    proche(g.q_m_traversant_kg_s, 0.0, 1e-9);
    // Éq. (156) et (157) : 1,2 × 111,25/3600 = 0,037 083 kg/s.
    proche(g.q_m_spec_souffle_kg_s, 1.2 * 111.25 / 3600.0, 1e-15);
    proche(g.q_m_spec_repris_kg_s, -1.2 * 111.25 / 3600.0, 1e-15);
    proche(g.q_m_hall_kg_s, 0.0, 1e-9);
}

#[test]
fn extraction_seule_compensee_par_les_defauts_d_etancheite() {
    // 100 m² de façade et 100 m² de toit à 0,6 m³/(h.m²) : 120 m³/h sous 4 Pa au total. Sans
    // vent ni écart de température, tous les composants voient ΔP = −Pib. Une extraction de
    // 120 m³/h met donc la zone en dépression de 4 Pa (éq. 167) : Pib = −4 Pa.
    let z = maison(groupe(100.0, 100.0, None));
    let b = z
        .bilan(&exterieur(19.0, 0.0), &[etat(19.0, -120.0, 0.0)])
        .unwrap();
    proche(b.p_ib_pa, -4.0, 1e-8);
    let g = &b.groupes[0];
    // Un quart de la façade par position (tableau 29) : 15 m³/h chacune ; le toit : 60 m³/h.
    for q in &g.q_m_def_kg_s[..4] {
        proche(*q, 1.2 * 15.0 / 3600.0, 1e-10);
    }
    proche(g.q_m_def_kg_s[4], 1.2 * 60.0 / 3600.0, 1e-10);
    proche(g.q_v_def_entr_m3_h, 120.0, 1e-6);
    assert_eq!(g.q_v_def_sort_m3_h, 0.0);
    // Éq. (171) : air neuf = 1,2 × 120/3600 = 0,04 kg/s ; éq. (172) : rien ne sort par
    // l'enveloppe.
    proche(g.q_m_neuf_kg_s, 0.04, 1e-9);
    assert_eq!(g.q_m_traversant_kg_s, 0.0);
    // Éq. (34) du §5.6.3.4.4 : 1006 × 0,04 = 40,24 W/K.
    proche(b.h_v_def_w_k, 40.24, 1e-6);
    // Avec des entrées d'air de module total 120 m³/h, la dépression tombe : sous ΔP, les
    // défauts laissent passer 120 × (ΔP/4)^(2/3) et les entrées d'air 4 × ¼ × C × ΔP^0,5.
    let ea = EntreeAir::nouvelle(120.0, None, 1.0);
    let z = maison(groupe(100.0, 100.0, Some(ea)));
    let b = z
        .bilan(&exterieur(19.0, 0.0), &[etat(19.0, -120.0, 0.0)])
        .unwrap();
    let dp = -b.p_ib_pa;
    assert!(dp > 0.0 && dp < 4.0);
    let g = &b.groupes[0];
    proche(
        g.q_v_def_entr_m3_h,
        120.0 * libm::pow(dp / 4.0, 2.0 / 3.0),
        1e-6,
    );
    proche(g.q_v_ea_entr_m3_h, 3600.0 * ea.q_v_m3_s(dp), 1e-6);
    proche(g.q_v_def_entr_m3_h + g.q_v_ea_entr_m3_h, 120.0, 1e-6);
}

#[test]
fn vent_seul_sur_une_zone_traversante() {
    // 100 m² de façade, pas de toit, air à 19 °C partout, vent de 4 m/s : pression dynamique
    // 0,5 × 1,2 × 16 = 9,6 Pa. Façades au vent : ΔP = 0,25 × 9,6 − Pib ; sous le vent :
    // ΔP = −0,5 × 9,6 − Pib. Les quatre défauts ont le même coefficient : l'équilibre demande
    // des ΔP opposés, soit Pib = −0,125 × 9,6 = −1,2 Pa et ΔP = ±3,6 Pa.
    // Débit entrant : 2 × ¼ × 60 × (3,6/4)^(2/3) = 30 × 0,932 170 = 27,965 m³/h.
    let z = maison(groupe(100.0, 0.0, None));
    let b = z
        .bilan(&exterieur(19.0, 4.0), &[etat(19.0, 0.0, 0.0)])
        .unwrap();
    proche(b.p_ib_pa, -1.2, 1e-8);
    let g = &b.groupes[0];
    proche(g.q_v_def_entr_m3_h, 27.965, 1e-3);
    proche(g.q_v_def_entr_m3_h, 30.0 * libm::pow(0.9, 2.0 / 3.0), 1e-6);
    proche(g.q_v_def_sort_m3_h, -g.q_v_def_entr_m3_h, 1e-6);
    // Au vent, l'air entre ; sous le vent, il sort ; le toit est absent.
    assert!(g.q_m_def_kg_s[0] > 0.0 && g.q_m_def_kg_s[2] > 0.0);
    assert!(g.q_m_def_kg_s[1] < 0.0 && g.q_m_def_kg_s[3] < 0.0);
    assert_eq!(g.q_m_def_kg_s[4], 0.0);
    proche(g.q_m_neuf_kg_s + g.q_m_traversant_kg_s, 0.0, 1e-10);
    // Zone non traversante (tableau 24) : ΔP = ±0,05 × 9,6 = ±0,48 Pa ;
    // 30 × (0,48/4)^(2/3) = 30 × 0,243 288 = 7,299 m³/h.
    let z = ZoneAeraulique::nouvelle(&[groupe(100.0, 0.0, None)], false, true, 0.0, 0.0, 2.5);
    let b = z
        .bilan(&exterieur(19.0, 4.0), &[etat(19.0, 0.0, 0.0)])
        .unwrap();
    proche(b.p_ib_pa, 0.0, 1e-8);
    proche(b.groupes[0].q_v_def_entr_m3_h, 7.299, 1e-3);
}

#[test]
fn tirage_thermique_seul() {
    // 0 °C dehors, 20 °C dedans, pas de vent, pas de toit : l'air entre par les défauts du bas
    // (zb = 0,625 m) et sort par ceux du haut (zh = 1,875 m). Écart de masse volumique :
    // 1,2 × 292 × (1/273 − 1/293) = 0,087 612 kg/m³. Entre le bas et le haut, la différence de
    // pression varie de g × 1,25 × Δρ = 1,0743 Pa = ΔPbas + |ΔPhaut|. L'air entre à
    // ρext = 1,2835 et sort à ρint = 1,1959 (éq. 161) : l'égalité des débits massiques donne
    // ΔPbas/|ΔPhaut| = (1,1959/1,2835)^(3/2) = 0,8994, soit ΔPbas = 0,5087 Pa.
    // Débit entrant : 2 × 15 × (0,5087/4)^(2/3) = 30 × 0,2529 = 7,587 m³/h.
    let z = maison(groupe(100.0, 0.0, None));
    let ext = exterieur(0.0, 0.0);
    let e = [etat(20.0, 0.0, 0.0)];
    let b = z.bilan(&ext, &e).unwrap();
    let g = &b.groupes[0];
    assert!(g.q_m_def_kg_s[2] > 0.0 && g.q_m_def_kg_s[3] > 0.0);
    assert!(g.q_m_def_kg_s[0] < 0.0 && g.q_m_def_kg_s[1] < 0.0);
    // Conservation de la masse (éq. 167).
    proche(g.q_m_neuf_kg_s + g.q_m_traversant_kg_s, 0.0, 1e-10);
    proche(g.q_v_def_entr_m3_h, 7.587, 5e-3);
    // Lecture littérale des altitudes : l'altitude du lieu décale Pib sans changer les débits
    // d'une zone à un seul groupe, Pib′ = Pib − g × z × (ρext − ρint).
    let haut = ZoneAeraulique::nouvelle_selon(
        &[groupe(100.0, 0.0, None)],
        true,
        true,
        500.0,
        0.0,
        2.5,
        LectureAltitudes::Litterale,
    );
    let bh = haut.bilan(&ext, &e).unwrap();
    proche(bh.groupes[0].q_v_def_entr_m3_h, g.q_v_def_entr_m3_h, 1e-6);
    let d_rho = rho_kg_m3(0.0, 0.0) - rho_kg_m3(20.0, 0.0);
    proche(d_rho, 0.087_612, 1e-6);
    proche(bh.p_ib_pa - b.p_ib_pa, -9.81 * 500.0 * d_rho, 1e-6);
}

#[test]
fn echanges_entre_groupes_par_le_hall() {
    // Deux groupes identiques à 19 °C et 21 °C, dehors à 19 °C sans vent ; le premier souffle
    // 100 m³/h, le second en extrait autant en masse. Éq. (168) : le premier cède son air au
    // hall (débit négatif), le second le reçoit (débit positif) ; la somme est nulle (éq. 167).
    let g = groupe(50.0, 0.0, None);
    let z = ZoneAeraulique::nouvelle(&[g, g], true, true, 0.0, 0.0, 2.5);
    let ext = exterieur(19.0, 0.0);
    let rho_2 = rho_kg_m3(21.0, 0.0);
    let etats = [
        etat(19.0, 0.0, 100.0),
        etat(21.0, -100.0 * 1.2 / rho_2, 0.0),
    ];
    let b = z.bilan(&ext, &etats).unwrap();
    let (a, c) = (&b.groupes[0], &b.groupes[1]);
    proche(a.q_m_spec_souffle_kg_s, 1.2 * 100.0 / 3600.0, 1e-15);
    proche(c.q_m_spec_repris_kg_s, -1.2 * 100.0 / 3600.0, 1e-12);
    proche(a.q_m_hall_kg_s + c.q_m_hall_kg_s, 0.0, 1e-10);
    assert!(a.q_m_hall_kg_s < 0.0 && c.q_m_hall_kg_s > 0.0);
    // Les fuites sont faibles devant le transfert : l'essentiel des 0,0333 kg/s passe par le hall.
    proche(c.q_m_hall_kg_s, 1.2 * 100.0 / 3600.0, 2e-3);
    // Hall fictif (éq. 130) : seul le premier groupe lui cède de l'air ; il est à 19 °C.
    let hall = hall_fictif(&b, &etats).unwrap();
    assert_eq!(hall, (19.0, 0.0));
    // Flux entrants du second groupe (éq. 173) : air neuf, air soufflé (nul), air du hall.
    let flux = flux_entrants(c, &ext, &etats[1], Some(hall));
    assert_eq!(flux.len(), 3);
    assert_eq!(flux[0].qm_kg_s, c.q_m_neuf_kg_s);
    assert_eq!(flux[0].theta_c, 19.0);
    assert_eq!(flux[1].qm_kg_s, 0.0);
    assert_eq!((flux[2].qm_kg_s, flux[2].theta_c), (c.q_m_hall_kg_s, 19.0));
    // Le premier groupe ne reçoit rien du hall.
    assert_eq!(flux_entrants(a, &ext, &etats[0], Some(hall)).len(), 2);
    // Bilan de masse de chaque groupe : ce qui entre égale ce qui sort.
    for g in &b.groupes {
        let entre = g.q_m_neuf_kg_s + g.q_m_spec_souffle_kg_s + g.q_m_hall_kg_s.max(0.0);
        let sort = g.q_m_traversant_kg_s + g.q_m_spec_repris_kg_s + g.q_m_hall_kg_s.min(0.0);
        proche(entre + sort, 0.0, 1e-10);
    }
}

#[test]
fn origine_des_altitudes_entre_deux_groupes() {
    // Deux groupes identiques à 19 °C et 20 °C, sans vent ni ventilation, dehors à 19 °C, à
    // 500 m d'altitude. Altitudes comptées depuis le plancher (lecture retenue) : seul le tirage
    // sur la hauteur de la zone joue, les débits entre groupes sont faibles et ne dépendent pas
    // de l'altitude du lieu. Lecture littérale : l'écart de masse volumique entre les groupes,
    // 1,2 × 292 × (1/292 − 1/293) = 0,004 096 kg/m³, crée entre eux un écart de pression de
    // 9,81 × 500 × 0,004 096 = 20,1 Pa et un débit de plusieurs dizaines de m³/h.
    let g = groupe(100.0, 0.0, None);
    let ext = exterieur(19.0, 0.0);
    let etats = [etat(19.0, 0.0, 0.0), etat(20.0, 0.0, 0.0)];
    let zone = |z: f64, lecture| {
        ZoneAeraulique::nouvelle_selon(&[g, g], true, true, z, 0.0, 2.5, lecture)
            .bilan(&ext, &etats)
            .unwrap()
    };
    let plancher = zone(500.0, LectureAltitudes::DepuisLePlancher);
    assert_eq!(plancher, zone(0.0, LectureAltitudes::DepuisLePlancher));
    assert_eq!(plancher, zone(0.0, LectureAltitudes::Litterale));
    // Au plus 9,81 × 1,875 × 0,004 096 = 0,075 Pa sur les défauts du haut :
    // 30 × (0,075/4)^(2/3) = 2,1 m³/h, borne large du débit échangé.
    assert!(3600.0 * plancher.groupes[1].q_m_hall_kg_s.abs() / 1.2 < 2.1);
    let litterale = zone(500.0, LectureAltitudes::Litterale);
    // Le groupe chaud perd par ses façades l'air qu'il reçoit du hall : plus de 30 m³/h.
    let q_m3_h = 3600.0 * litterale.groupes[1].q_m_hall_kg_s / 1.2;
    assert!(q_m3_h > 30.0, "{q_m3_h}");
    assert_eq!(
        ZoneAeraulique::nouvelle(&[g, g], true, true, 500.0, 0.0, 2.5),
        ZoneAeraulique::nouvelle_selon(
            &[g, g],
            true,
            true,
            500.0,
            0.0,
            2.5,
            LectureAltitudes::DepuisLePlancher
        )
    );
}

#[test]
fn zone_etanche_aux_debits_desequilibres() {
    // Ni perméabilité ni entrée d'air : une extraction seule n'a pas d'équilibre.
    let g = GroupeAeraulique {
        a_f_m2: 100.0,
        a_t_m2: 0.0,
        q4pa_surf_m3_h_m2: 0.0,
        entree_air: None,
    };
    let z = maison(g);
    assert!(z
        .bilan(&exterieur(19.0, 0.0), &[etat(19.0, -100.0, 0.0)])
        .is_none());
    // Nombre d'états différent du nombre de groupes.
    assert!(z.bilan(&exterieur(19.0, 0.0), &[]).is_none());
}

#[test]
fn regles_de_saisie_de_la_zone() {
    let z = "batiments[0].zones[0]";
    assert!(erreurs(&projet()).is_empty());
    // Hauteurs et perméabilité requises (§5.6.2).
    let mut p = projet();
    p.batiments[0].zones[0].h_zone_m = None;
    p.batiments[0].zones[0].h_plancher_m = None;
    p.batiments[0].zones[0].q4pa_surf_m3_h_m2 = None;
    assert_eq!(
        erreurs(&p),
        [
            format!("{z}.h_zone_m"),
            format!("{z}.h_plancher_m"),
            format!("{z}.q4pa_surf_m3_h_m2")
        ]
    );
    let mut p = projet();
    p.batiments[0].zones[0].h_zone_m = Some(0.0);
    p.batiments[0].zones[0].q4pa_surf_m3_h_m2 = Some(-0.1);
    assert_eq!(
        erreurs(&p),
        [format!("{z}.h_zone_m"), format!("{z}.q4pa_surf_m3_h_m2")]
    );
    // Un plancher sous le niveau du sol est admis.
    let mut p = projet();
    p.batiments[0].zones[0].h_plancher_m = Some(-1.2);
    assert!(erreurs(&p).is_empty());
    // Tableau 22 : conventionnel en maison individuelle, à saisir en logement collectif.
    let mut p = projet();
    p.batiments[0].zones[0].trav_zone = Some(true);
    assert_eq!(erreurs(&p), [format!("{z}.trav_zone")]);
    let mut p = projet();
    p.batiments[0].zones[0].usage = UsageZone::LogementCollectif;
    p.batiments[0].zones[0].nb_lgt = Some(2);
    p.batiments[0].zones[0].groupes[0].traversant = Some(true);
    assert_eq!(erreurs(&p), [format!("{z}.trav_zone")]);
    p.batiments[0].zones[0].trav_zone = Some(false);
    assert!(erreurs(&p).is_empty());
}

#[test]
fn regles_de_saisie_des_entrees_d_air() {
    let c = "batiments[0].zones[0].groupes[0].entrees_air";
    let avec = |e: Option<EntreesAir>| {
        let mut p = projet();
        p.batiments[0].zones[0].groupes[0].entrees_air = e;
        erreurs(&p)
    };
    let autoreglable = |dp1: f64, dp2: f64, m_prime: f64| EntreesAir {
        m_m3_h: 120.0,
        autoreglable: Some(AutoreglageEntreesAir {
            delta_p1_pa: dp1,
            delta_p2_pa: dp2,
            m_prime_m3_h: m_prime,
        }),
        r: Some(0.5),
        espace_tampon: None,
    };
    // Les entrées d'air sont facultatives.
    assert!(avec(None).is_empty());
    assert!(avec(Some(autoreglable(20.0, 100.0, 60.0))).is_empty());
    // Module positif, r entre 0 et 1.
    assert_eq!(
        avec(Some(EntreesAir {
            m_m3_h: -1.0,
            autoreglable: None,
            r: Some(1.5),
            espace_tampon: None,
        })),
        [format!("{c}.m_m3_h"), format!("{c}.r")]
    );
    // ΔP2 > ΔP1 > 0.
    assert_eq!(
        avec(Some(autoreglable(20.0, 20.0, 60.0))),
        [format!("{c}.autoreglable.delta_p2_pa")]
    );
    assert_eq!(
        avec(Some(autoreglable(0.0, 100.0, 60.0))),
        [format!("{c}.autoreglable.delta_p1_pa")]
    );
    // Caractéristique croissante : M′ × 100^0,5 ≥ 120 × 20^0,5, soit M′ ≥ 53,67 m³/h.
    assert_eq!(
        avec(Some(autoreglable(20.0, 100.0, 53.0))),
        [format!("{c}.autoreglable.m_prime_m3_h")]
    );
    assert!(avec(Some(autoreglable(20.0, 100.0, 54.0))).is_empty());
    // Mêmes pressions ΔP1 et ΔP2 dans tous les groupes de la zone (§5.6.3.2.2).
    let mut p = projet();
    let mut second = p.batiments[0].zones[0].groupes[0].clone();
    second.id = "second".into();
    p.batiments[0].zones[0].groupes[0].entrees_air = Some(autoreglable(20.0, 100.0, 60.0));
    second.entrees_air = Some(autoreglable(20.0, 80.0, 70.0));
    p.batiments[0].zones[0].groupes.push(second);
    assert_eq!(
        erreurs(&p),
        ["batiments[0].zones[0].groupes[1].entrees_air.autoreglable"]
    );
}

#[test]
fn surfaces_de_permeabilite_de_la_maison_de_demonstration() {
    // §5.6.3.1.5 et §5.7.2 : façades = parois opaques verticales (90 m²) et baies verticales
    // (10 m²) ; toit = parois horizontales hors plancher bas (100 m²).
    let meteo = MeteoProjet::synthetique(thbce_data::ZoneClimatique::H2b);
    let r =
        thbce_core::calculer(&projet(), &meteo, &thbce_core::Options { arrondir: false }).unwrap();
    let e = &r.batiments[0].zones[0].groupes[0].enveloppe;
    assert_eq!((e.a_f_perm_m2, e.a_toit_perm_m2), (100.0, 100.0));
}
