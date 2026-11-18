// SPDX-License-Identifier: GPL-3.0-or-later
//! Accumulateurs d'ECS en eau technique (fiches 9.11 et 9.12, lot 1-ecs, Q106) : exemple
//! `tests/cas/restaurant-accumulateur-ademe` (échangeur externe, valeurs figées par son
//! `attendu.json`), variantes à échangeur interne et à circulateur à vitesse constante,
//! recoupements à la main, bilan d'énergie du ballon, refus ciblés.
//!
//! Valeurs phares sur la météo réglementaire :
//! THBCE_METEO=/chemin/vers/meteo.csv cargo t-regl accumulateur_sur_meteo_reglementaire

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::bati::assemblage_batiment::BatimentHoraire;
use thbce_core::bati::assemblage_groupe::{LecturePuisageAccumulateur, Lectures};
use thbce_core::chauffage_refroidissement::calculs_generation::{Energie, Poste};
use thbce_core::climat::{ClimatExterieur, ConfortAdaptatif, MeteoProjet};
use thbce_core::conventions::ModeCalcul;
use thbce_core::entree::{
    ApplicationAccumulateurSaisie, EchangeurEcsSaisi, NatureEchangeurSaisie, Projet,
    ReseauEcsIntergroupe,
};
use thbce_core::resultats::{Resultats, ResultatsStockage};
use thbce_core::{calculer, valider, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn exemple(nom: &str) -> Projet {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn restaurant() -> Projet {
    exemple("restaurant-accumulateur-ademe")
}

fn echangeur(p: &mut Projet) -> &mut EchangeurEcsSaisi {
    p.batiments[0].generations[1].stockages[0]
        .echangeur
        .as_mut()
        .unwrap()
}

fn meteo_ademe(p: &Projet) -> MeteoProjet {
    let zone = p.localisation.zone_climatique_retenue();
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

fn calcul(p: &Projet) -> Resultats {
    calculer(p, &meteo_ademe(p), &Options { arrondir: false }).unwrap()
}

fn ballon(r: &Resultats) -> &ResultatsStockage {
    &r.batiments[0].generations[1].stockages[0]
}

/// Bilan d'énergie du ballon : énergie injectée par la base, moins les pertes, moins l'énergie
/// livrée à la distribution, égale à la variation du stock de chaleur (un ballon).
fn ecart_de_bilan_kwh(b: &ResultatsStockage) -> f64 {
    let stock: f64 = b
        .v_zones_l
        .iter()
        .zip(b.theta_zones_fin_c.iter().zip(&b.theta_zones_debut_c))
        .map(|(v, (fin, debut))| 1.163 * v * (fin - debut) / 1000.0)
        .sum();
    b.q_fou_sto_base_kwh + b.q_fou_sto_ap_kwh - b.pertes_kwh - b.q_fou_ecs_kwh - stock
}

/// Échangeur externe, application collective par défaut (500 L : consigne de 60 °C, tableau
/// 288) : `UAech` retrouvé à la main par inversion de la méthode du NUT (éq. 1876 à 1882).
#[test]
fn echangeur_externe_a_la_main() {
    let r = calcul(&restaurant());
    let e = ballon(&r).echangeur.unwrap();
    assert_eq!(e.theta_cons_c, 60.0);
    // Calcul indépendant : P = 60 kW, qprim,nom = 3 m³/h, θcons = 60 °C.
    let q_ecs = 1000.0 * 60.0 / (1163.0 * 40.0);
    let eps = 40.0 / 45.0;
    let rd = q_ecs / 3.0;
    let y = (1.0 - eps) / (1.0 - eps * rd);
    // −ln(y), calculé à part (Python, math.log) : 1,7157.
    let moins_ln_y = 1.715709448927928_f64;
    assert!(y > 0.17 && y < 0.19);
    let nut = moins_ln_y / (1.0 - rd);
    let ua = nut * 1163.0 * q_ecs;
    assert!((e.ua_ech_w_k - ua).abs() < 1e-3, "{} {ua}", e.ua_ech_w_k);
    // Les quatre zones sont de volumes égaux (éq. 1874 et 1875).
    assert_eq!(ballon(&r).v_zones_l, vec![125.0; 4]);
    // La valeur par défaut de l'application est signalée.
    assert!(ballon(&r)
        .valeurs_par_defaut
        .iter()
        .any(|v| v.champ == "echangeur.application" && v.valeur == 60.0));
    assert!(ecart_de_bilan_kwh(ballon(&r)).abs() < 1e-3);
    assert!(e.w_circ_prim_kwh > 0.0);
}

/// Application individuelle à 55 °C : consigne du tableau 288 et ballon moins chaud, donc
/// moins de pertes que l'application collective.
#[test]
fn application_individuelle_et_collective() {
    let collectif = calcul(&restaurant());
    let mut p = restaurant();
    echangeur(&mut p).application = Some(ApplicationAccumulateurSaisie::Individuel);
    let individuel = calcul(&p);
    assert_eq!(ballon(&individuel).echangeur.unwrap().theta_cons_c, 55.0);
    assert!(ballon(&individuel).pertes_kwh < ballon(&collectif).pertes_kwh);
    assert!(ecart_de_bilan_kwh(ballon(&individuel)).abs() < 1e-3);
    // Saisie explicite : plus de valeur par défaut signalée pour l'application.
    assert!(!ballon(&individuel)
        .valeurs_par_defaut
        .iter()
        .any(|v| v.champ == "echangeur.application"));
}

/// Échangeur interne : `UAech = ρ c qECS,nom × (−ln(1 − εnom))` (éq. 1923 à 1926), pas de
/// circulateur primaire.
#[test]
fn echangeur_interne() {
    let mut p = restaurant();
    echangeur(&mut p).nature = NatureEchangeurSaisie::Interne;
    let v = valider(&p);
    assert!(v.est_valide(), "{:?}", v.erreurs);
    let r = calcul(&p);
    let e = ballon(&r).echangeur.unwrap();
    let q_ecs = 1000.0 * 60.0 / (1163.0 * 40.0);
    // −ln(1 − 40/45) = ln 9.
    let ua = 1163.0 * q_ecs * 2.1972245773362196;
    assert!((e.ua_ech_w_k - ua).abs() < 1e-6);
    assert_eq!(e.w_circ_prim_kwh, 0.0);
    assert!(ecart_de_bilan_kwh(ballon(&r)).abs() < 1e-3);
    // Toute la demande est puisée dans l'année (aucun report en fin de calcul).
    assert!(r.batiments[0].generations[1].q_rep_fin_kwh < 1e-6);
}

/// Un circulateur à vitesse constante consomme plus qu'un circulateur à vitesse variable de
/// même puissance nominale (éq. 1907 et 1908).
#[test]
fn circulateur_constant_et_variable() {
    let variable = calcul(&restaurant());
    let mut p = restaurant();
    if let NatureEchangeurSaisie::Externe {
        circulateur_vitesse_variable,
        ..
    } = &mut echangeur(&mut p).nature
    {
        *circulateur_vitesse_variable = false;
    }
    let constant = calcul(&p);
    let (v, c) = (
        ballon(&variable).echangeur.unwrap().w_circ_prim_kwh,
        ballon(&constant).echangeur.unwrap().w_circ_prim_kwh,
    );
    assert!(c > v && v > 0.0, "{c} {v}");
}

/// Refus et erreurs de saisie propres à la fiche.
#[test]
fn refus() {
    // Réseau d'ECS bouclé : réchauffage de boucle hors de cette version (code 3).
    let mut p = exemple("restaurant-scolaire-ademe");
    p.batiments[0].generations[1].stockages[0].echangeur = Some(EchangeurEcsSaisi {
        application: None,
        p_ech_ecs_kw: 60.0,
        nature: NatureEchangeurSaisie::Interne,
    });
    assert!(matches!(
        p.batiments[0].distributions_ecs_intergroupes[0].reseau,
        ReseauEcsIntergroupe::Boucle(_)
    ));
    let v = valider(&p);
    assert!(v.seulement_hors_perimetre(), "{:?}", v.erreurs);
    assert!(v.erreurs[0].chemin.ends_with("stockages[0].echangeur"));
    // « faux » sans objet : quatre zones égales.
    let mut p = restaurant();
    p.batiments[0].generations[1].stockages[0].appoint = Some(thbce_core::entree::AppointBallon {
        generateur: "resistance".into(),
        programmation: thbce_core::entree::ProgrammationBallon::Permanent,
        z_ap: 3,
        z_reg: 3,
        f_aux: Some(0.5),
        delta_theta_k: None,
    });
    let v = valider(&p);
    assert!(v
        .erreurs
        .iter()
        .any(|e| e.chemin.ends_with("appoint.f_aux") && !e.hors_perimetre));
    // Température maximale du ballon sous la consigne de 60 °C (collectif).
    let mut p = restaurant();
    p.batiments[0].generations[1].stockages[0]
        .ballon
        .theta_max_c = 58.0;
    assert!(valider(&p)
        .erreurs
        .iter()
        .any(|e| e.chemin.ends_with("ballon.theta_max_c")));
    // Puissances et débits strictement positifs.
    let mut p = restaurant();
    echangeur(&mut p).p_ech_ecs_kw = 0.0;
    assert!(!valider(&p).est_valide());
}

/// Th-C du restaurant avec une lecture donnée du puisage : consommation d'ECS (toutes énergies,
/// Wh), énergie de l'ECS fournie par le ballon et consommation du circulateur primaire (Wh).
fn th_c_selon(p: &Projet, lectures: Lectures) -> (f64, f64, f64) {
    let loc = &p.localisation;
    let m = meteo_ademe(p);
    let climat = ClimatExterieur::nouveau(&m.th_bc, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    let mut b = BatimentHoraire::nouveau_selon(
        &p.batiments[0],
        &p.catalogue,
        ModeCalcul::ThC,
        lectures,
        loc.alt_m,
        climat.we_kg_kg[0],
    )
    .unwrap();
    b.simuler(&climat, &confort).unwrap();
    let ecs: f64 = Energie::TOUTES
        .iter()
        .map(|&e| {
            b.systemes.cumuls_cef_groupes[0]
                .annuel()
                .valeur(Poste::Ecs, e)
        })
        .sum();
    let st = &b.systemes.generations_horaires[1].stockages[0].cumuls;
    (ecs, st.q_fou_ecs_wh, st.w_circ_prim_wh)
}

/// Variante littérale de Q106 point 3 (p. 1134 : la fiche 9.12.3.2.4 n'est appliquée que si
/// Vef,ecs > 0) : aucun puisage ni circulateur aux heures sans puisage, l'énergie reportée
/// restant à fournir. Les chiffres sont imprimés pour être repris dans Q106.
#[test]
fn puisage_litteral_sans_vef() {
    let p = restaurant();
    let (ecs, fou, circ) = th_c_selon(&p, Lectures::default());
    let (ecs_l, fou_l, circ_l) = th_c_selon(
        &p,
        Lectures {
            puisage_accumulateur: LecturePuisageAccumulateur::Litteral,
            ..Lectures::default()
        },
    );
    println!(
        "puisage : retenu ECS {ecs:.0} Wh, ballon {fou:.0} Wh, circulateur {circ:.0} Wh ; littéral {ecs_l:.0} Wh, {fou_l:.0} Wh, {circ_l:.0} Wh"
    );
    // Sur l'exemple, aucune heure n'a d'énergie reportée sans puisage : les deux lectures
    // coïncident. Un échangeur sous-dimensionné fait apparaître des reports.
    assert_eq!((ecs, fou, circ), (ecs_l, fou_l, circ_l));
    for kw in [20.0, 10.0, 5.0] {
        let mut p = restaurant();
        echangeur(&mut p).p_ech_ecs_kw = kw;
        let (e1, f1, c1) = th_c_selon(&p, Lectures::default());
        let (e2, f2, c2) = th_c_selon(
            &p,
            Lectures {
                puisage_accumulateur: LecturePuisageAccumulateur::Litteral,
                ..Lectures::default()
            },
        );
        println!(
            "échangeur {kw} kW : retenu ECS {e1:.0} Wh, ballon {f1:.0} Wh, circ {c1:.0} Wh ; littéral {e2:.0} Wh, {f2:.0} Wh, {c2:.0} Wh"
        );
        assert!(c2 <= c1 + 1e-9);
        if kw == 20.0 {
            // Chiffres de Q106 point 3 (en kWh), à ±0,5 %.
            let proche = |wh: f64, kwh: f64| {
                assert!(
                    (wh / 1000.0 - kwh).abs() < 0.005 * kwh,
                    "{} kWh != {kwh}",
                    wh / 1000.0
                );
            };
            proche(f1, 12_541.0);
            proche(f2, 9_887.0);
            proche(c1, 143.8);
            proche(c2, 19.3);
        }
    }
    // Sans puisage ni circulateur hors des heures de puisage, le circulateur ne consomme pas
    // davantage et l'énergie fournie par le ballon n'est pas supérieure.
    assert!(circ_l <= circ + 1e-9);
    assert!(fou_l <= fou * 1.0001);
}

/// Valeurs phares du restaurant à accumulateur (H2b) sur la météo réglementaire : nombres
/// annuels seulement.
#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn accumulateur_sur_meteo_reglementaire() {
    let p = restaurant();
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO");
    let zone = p.localisation.zone_climatique_retenue();
    let m = meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap();
    let r = calculer(&p, &m, &Options { arrondir: false }).unwrap();
    assert!(r.reglementaire);
    let b = &r.batiments[0];
    let (cep, dh) = (b.cep_kwhep_m2.unwrap(), b.dh_c_h.unwrap());
    let e = ballon(&r).echangeur.unwrap();
    println!(
        "accumulateur réglementaire : Bbio {:.1} ; DH {dh:.1} ; Cep {cep:.1} ; circulateur {:.1} kWh ; pertes {:.1} kWh",
        b.bbio_pts.unwrap(),
        e.w_circ_prim_kwh,
        ballon(&r).pertes_kwh,
    );
    assert!(ecart_de_bilan_kwh(ballon(&r)).abs() < 1e-3);
    // Lot 1-ecs (18 novembre 2026) ; le même restaurant à chauffe-eau direct : Bbio 248,4,
    // DH 1 878,7, Cep 352,9 (tests/restauration.rs).
    assert!((b.bbio_pts.unwrap() - 248.4).abs() <= 0.05);
    assert!((dh - 1881.1).abs() <= 0.05 && (cep - 353.9).abs() <= 0.05);
}
