// SPDX-License-Identifier: GPL-3.0-or-later
//! Espaces tampons (Th-BCE 2020 §5.2 à 5.4) : validation de la saisie, premier pas de temps
//! calculé à la main sur le jeu météo synthétique, coefficients appliqués aux composants. Les
//! calculs annuels sur une météo réaliste sont dans `thbce-io` (`tests/espaces_tampons.rs`).
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::bati::assemblage_batiment::BatimentHoraire;
use thbce_core::bati::assemblage_groupe::EnveloppeResolue;
use thbce_core::bati::baie::{BaieResolue, Protections};
use thbce_core::bati::espace_tampon::espace_tampon::{
    CoefficientsEt, CV_AIR_WH_M3K, K_KJ_WH, THETA_ET_INITIALE_C, THETA_I_FIN_INITIALE_ET_C,
};
use thbce_core::bati::{ConditionsExterieures, FluxOpaques};
use thbce_core::climat::environnement_proche::Soleil;
use thbce_core::climat::{ClimatExterieur, ConfortAdaptatif, MeteoProjet};
use thbce_core::conventions::ModeCalcul;
use thbce_core::entree::{NatureEspaceTampon, Projet};
use thbce_core::{calculer, valider, Options};
use thbce_data::ZoneClimatique;

const ENTREE: &str = include_str!("../../../tests/cas/maison-veranda-ademe/entree.json");

fn projet() -> Projet {
    serde_json::from_str(ENTREE).expect("la variante de la maison d'exemple doit être lisible")
}

fn erreurs(p: &Projet) -> Vec<String> {
    valider(p).erreurs.into_iter().map(|e| e.chemin).collect()
}

#[test]
fn la_variante_est_valide() {
    let r = valider(&projet());
    assert!(r.est_valide(), "{:?}", r.erreurs);
}

#[test]
fn regles_de_saisie_des_espaces_tampons() {
    let c = "batiments[0]";
    let g = format!("{c}.zones[0].groupes[0]");
    // Référence inconnue.
    let mut p = projet();
    p.batiments[0].zones[0].groupes[0].ponts_thermiques[0].espace_tampon = Some("cave".into());
    assert_eq!(
        erreurs(&p),
        [format!("{g}.ponts_thermiques[0].espace_tampon")]
    );
    // Une baie ne donne que sur un espace tampon solarisé (§5.10.3).
    let mut p = projet();
    p.batiments[0].zones[0].groupes[0].baies[1].espace_tampon = Some("garage".into());
    assert_eq!(erreurs(&p), [format!("{g}.baies[1].espace_tampon")]);
    // Un espace tampon sans composant d'enveloppe (§5.4.1) ; les entrées d'air ne suffisent pas.
    let mut p = projet();
    p.batiments[0].zones[0].groupes[0].parois_opaques[1].espace_tampon = None;
    p.batiments[0].zones[0].groupes[0]
        .entrees_air
        .as_mut()
        .unwrap()
        .espace_tampon = Some("garage".into());
    assert_eq!(erreurs(&p), [format!("{c}.espaces_tampons[1].id")]);
    // Plages des tableaux 14 et 19 ; facteurs « avec protection » requis avec des protections.
    let mut p = projet();
    for e in &mut p.batiments[0].espaces_tampons {
        match &mut e.nature {
            NatureEspaceTampon::NonSolarise(n) => n.b_et_ns = 1.5,
            NatureEspaceTampon::Solarise(s) => {
                s.a_et_m2 = 0.0;
                s.parois[0].swe_ap = None;
                s.parois[1].r_ouv_max = 2.0;
            }
        }
    }
    assert_eq!(
        erreurs(&p),
        [
            format!("{c}.espaces_tampons[0].nature.solarise.a_et_m2"),
            format!("{c}.espaces_tampons[0].nature.solarise.parois[0].swe_ap"),
            format!("{c}.espaces_tampons[0].nature.solarise.parois[1].r_ouv_max"),
            format!("{c}.espaces_tampons[1].nature.non_solarise.b_et_ns"),
        ]
    );
    // Identifiants uniques.
    let mut p = projet();
    p.batiments[0].espaces_tampons[1].id = "veranda".into();
    assert!(erreurs(&p).contains(&format!("{c}.espaces_tampons[1].id")));
}

/// Premier pas de temps, jeu synthétique (température extérieure constante, sans soleil) :
/// éq. (106) à (108), (114) pour le garage, (115) à (126) pour la véranda.
#[test]
fn premier_pas_de_temps_calcule_a_la_main() {
    let p = projet();
    let loc = &p.localisation;
    let meteo = MeteoProjet::synthetique(ZoneClimatique::H2b);
    let climat = ClimatExterieur::nouveau(&meteo.th_bc, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    let mut b = BatimentHoraire::nouveau(
        &p.batiments[0],
        &p.catalogue,
        ModeCalcul::ThB,
        loc.alt_m,
        climat.we_kg_kg[0],
    )
    .unwrap();
    // Éq. (106) : véranda, mur sud 22 × 0,2 et baie sud 8 × 1,3 ; garage, mur ouest 20 × 0,2.
    let (v, g) = (&b.espaces_tampons[0], &b.espaces_tampons[1]);
    assert!((v.h_et_th_int_eq_w_k - 14.8).abs() < 1e-9);
    assert!((g.h_et_th_int_eq_w_k - 4.0).abs() < 1e-9);
    b.pas(0, &climat, &confort).unwrap();
    let te = climat.te_c[0];
    // Garage, éq. (111) et (114) : θi_eq = 19 °C au premier pas de temps (tableau 19).
    let cg = b.coefficients[1];
    assert_eq!((cg.b_therm, cg.b_solaire, cg.b_ecl), (0.95, 0.0, 0.0));
    let theta_i = THETA_I_FIN_INITIALE_ET_C;
    assert!((cg.theta_et_c - (0.95 * te + 0.05 * theta_i)).abs() < 1e-12);
    // Véranda, sans soleil ni ouverture : Het_th_ext = 81,5 × 1,4 + 0,5 × 36 = 132,1 W/K ;
    // débit de base 36 m³/h ; Cm_et = 200 × 36/3,6 Wh/K.
    let h_ext = 81.5 * 1.4 + 0.5 * 36.0;
    let h_air = CV_AIR_WH_M3K * 36.0;
    let h_et = h_ext + h_air + 14.8;
    let c = 200.0 * 36.0 * K_KJ_WH;
    let theta_inf = ((h_ext + h_air) * te + 14.8 * theta_i) / h_et;
    let theta_et = theta_inf - (theta_inf - THETA_ET_INITIALE_C) * libm::exp(-h_et / c);
    let cv = b.coefficients[0];
    assert!((cv.theta_et_c - theta_et).abs() < 1e-12);
    assert!((cv.b_therm - (theta_i - theta_et) / (theta_i - te)).abs() < 1e-12);
    // Éq. (127) et (129), protections relevées : 0,9 × moyenne des facteurs pondérée par les
    // surfaces.
    let bs = 0.9 * (0.6 * 45.0 + 0.5 * 36.5) / 81.5;
    let be = 0.9 * (0.7 * 45.0 + 0.6 * 36.5) / 81.5;
    assert!((cv.b_solaire - bs).abs() < 1e-12);
    assert!((cv.b_ecl - be).abs() < 1e-12);
    assert_eq!(cv.a_ouv_et_m2, 0.0);
}

fn conditions() -> ConditionsExterieures {
    ConditionsExterieures {
        soleil: Soleil {
            gamma_rad: 30_f64.to_radians(),
            psi_rad: 0.0,
        },
        idn_w_m2: 800.0,
        idi_w_m2: 100.0,
        edn_lux: 80_000.0,
        edi_lux: 12_000.0,
        te_c: 5.0,
        teciel_c: -5.0,
        h: 4000,
        mode: ModeCalcul::ThB,
        is_conf_adapt: false,
    }
}

/// Coefficients appliqués aux composants (§5.10.3, §5.17.3, §5.19.3).
#[test]
fn coefficients_appliques_aux_composants() {
    let p = projet();
    let g = &p.batiments[0].zones[0].groupes[0];
    let ids: Vec<String> = p.batiments[0]
        .espaces_tampons
        .iter()
        .map(|e| e.id.clone())
        .collect();
    let env =
        EnveloppeResolue::de_dans_batiment(g, &p.catalogue, p.batiments[0].zones[0].usage, &ids);
    assert_eq!(env.parois[0].espace_tampon, Some(0));
    assert_eq!(env.parois[1].espace_tampon, Some(1));
    assert_eq!(env.parois[2].espace_tampon, None);
    assert_eq!(env.baies[0].espace_tampon, Some(0));
    // Éq. (106) depuis l'enveloppe.
    assert!((env.h_et_th_int_w_k(0) - 14.8).abs() < 1e-9);
    assert!((env.h_et_th_int_w_k(1) - 4.0).abs() < 1e-9);
    let c = conditions();
    let et = CoefficientsEt {
        b_therm: 0.4,
        b_solaire: 0.5,
        b_ecl: 0.6,
        theta_et_c: 12.0,
        a_ouv_et_m2: 0.0,
    };
    // Paroi : HTH × btherm, Φsh_s × bsolaire, Φsh_vc nul (bvoute = 0).
    let mur = &env.parois[0];
    let f = mur.flux(&c);
    assert!(f.phi_sh_vc_w != 0.0);
    assert_eq!(
        mur.flux_reduit(&c, Some(&et)),
        FluxOpaques {
            phi_sh_s_w: f.phi_sh_s_w * 0.5,
            phi_sh_vc_w: 0.0,
        }
    );
    assert!((mur.h_th_reduit_w_k(Some(&et)) - 0.4 * 4.4).abs() < 1e-12);
    assert_eq!(mur.flux_reduit(&c, None), f);
    // Baie : Hges × btherm, Fs × bsolaire, Flt × becl, Ftvc nul.
    let baie: &BaieResolue = &env.baies[0];
    let incident = baie.incident(&c);
    let f = baie.flux_depuis_incident(&incident, &c, &Protections::OUVERTES);
    let r = baie.flux_reduit(&incident, &c, &Protections::OUVERTES, Some(&et));
    assert!((r.h_ges_w_k - 0.4 * f.h_ges_w_k).abs() < 1e-12);
    assert!((r.fs1_w - 0.5 * f.fs1_w).abs() < 1e-9);
    assert!((r.fs2_w - 0.5 * f.fs2_w).abs() < 1e-9);
    assert_eq!(r.ftvc_w, 0.0);
    assert!(f.ftvc_w != 0.0);
    assert!((r.flt2_lm[2] - 0.6 * f.flt2_lm[2]).abs() < 1e-6);
    // Espace tampon surventilé : btherm de la baie égal à 1 (§5.8, p. 178).
    let ouvert = CoefficientsEt {
        a_ouv_et_m2: 2.0,
        ..et
    };
    let r = baie.flux_reduit(&incident, &c, &Protections::OUVERTES, Some(&ouvert));
    assert_eq!(r.h_ges_w_k, f.h_ges_w_k);
    assert_eq!(BaieResolue::b_therm(Some(&ouvert)), 1.0);
    assert_eq!(BaieResolue::b_therm(None), 1.0);
}

/// Un calcul complet sur le jeu synthétique aboutit et restitue les espaces tampons.
#[test]
fn resultats_des_espaces_tampons() {
    let p = projet();
    let r = calculer(
        &p,
        &MeteoProjet::synthetique(ZoneClimatique::H2b),
        &Options { arrondir: false },
    )
    .unwrap();
    let ets = &r.batiments[0].espaces_tampons;
    assert_eq!(ets.len(), 2);
    assert_eq!(ets[0].id, "veranda");
    assert_eq!(ets[0].valeurs_par_defaut.len(), 2);
    let garage = ets[1].th_b.as_ref().unwrap();
    assert!((garage.b_therm_moyen - 0.95).abs() < 1e-12);
    assert_eq!(garage.nbh_surventilation, 0);
    // Sans soleil, la véranda reste entre l'extérieur et l'intérieur.
    let v = ets[0].th_b.as_ref().unwrap();
    assert!(v.b_therm_moyen > 0.0 && v.b_therm_moyen < 1.0);
    // Sans espace tampon, aucun bloc n'est restitué.
    let mut sans = p.clone();
    sans.batiments[0].espaces_tampons.clear();
    for c in &mut sans.batiments[0].zones[0].groupes[0].parois_opaques {
        c.espace_tampon = None;
    }
    for c in &mut sans.batiments[0].zones[0].groupes[0].baies {
        c.espace_tampon = None;
    }
    let r = calculer(
        &sans,
        &MeteoProjet::synthetique(ZoneClimatique::H2b),
        &Options::default(),
    )
    .unwrap();
    assert!(r.batiments[0].espaces_tampons.is_empty());
    let json = serde_json::to_string(&r).unwrap();
    assert!(!json.contains("espaces_tampons"));
}
