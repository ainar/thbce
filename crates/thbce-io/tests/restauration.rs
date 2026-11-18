// SPDX-License-Identifier: GPL-3.0-or-later
//! Restaurants (usages 13 à 16) et restaurants scolaires (26 et 27) : lot 0c3. Exemples
//! `tests/cas/restaurant-pac-ademe` (usage 13) et `tests/cas/restaurant-scolaire-ademe`
//! (usage 26) sur la météo ouverte ADEME (leurs valeurs sont vérifiées par leur
//! `attendu.json`) ; calculs à la main ; sens des effets ; autres usages du lot par
//! substitution ; besoin d'ECS de l'usage 16 (Q57) ; refus.
//!
//! Valeurs phares sur la météo réglementaire :
//! THBCE_METEO=/chemin/vers/meteo.csv cargo t-regl restauration_sur_meteo_reglementaire

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{GestionPm, Projet, ReseauEcsIntergroupe, UsageZone};
use thbce_core::resultats::Resultats;
use thbce_core::scenarios::scenarios_conventionnels::ScenariosZone;
use thbce_core::{calculer, valider, Options};
use thbce_data::eclairage::TypeLocal;
use thbce_data::protections_mobiles::{famille_et_parts, FamilleGpm};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn exemple(nom: &str) -> Projet {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn restaurant() -> Projet {
    exemple("restaurant-pac-ademe")
}

fn scolaire() -> Projet {
    exemple("restaurant-scolaire-ademe")
}

fn meteo_ademe(p: &Projet) -> MeteoProjet {
    let zone = p.localisation.zone_climatique_retenue();
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

fn calcul(p: &Projet) -> Resultats {
    calculer(p, &meteo_ademe(p), &Options { arrondir: false }).unwrap()
}

/// Indicateurs d'un calcul : Bbio, DH, Cep, Cep,nr, Ic énergie.
fn phares(r: &Resultats) -> [f64; 5] {
    let b = &r.batiments[0];
    [
        b.bbio_pts.unwrap(),
        b.dh_c_h.unwrap(),
        b.cep_kwhep_m2.unwrap(),
        b.cep_nr_kwhep_m2.unwrap(),
        b.ic_energie_kgco2_m2.unwrap(),
    ]
}

fn elec(r: &Resultats) -> &thbce_core::indicateurs::resultats::PostesElectricite {
    &r.batiments[0]
        .indicateurs
        .as_ref()
        .unwrap()
        .electricite_consommee
}

fn boucle(p: &mut Projet) -> &mut thbce_core::entree::BoucleEcs {
    match &mut p.batiments[0].distributions_ecs_intergroupes[0].reseau {
        ReseauEcsIntergroupe::Boucle(b) => b,
        _ => panic!("le restaurant scolaire a une boucle d'ECS"),
    }
}

/// Valeurs recalculées à la main, indépendamment du code des postes concernés : surfaces,
/// débits conventionnels du Bbio (tableau 56-1), forfait de refroidissement (éq. 2493,
/// tableau 335), circulateurs et pertes de la boucle d'ECS du restaurant scolaire (éq. 1736,
/// 1744 et 1747), familles des protections mobiles (figure 34).
#[test]
fn restaurants_a_la_main() {
    let (pr, ps) = (restaurant(), scolaire());
    for p in [&pr, &ps] {
        let v = valider(p);
        assert!(v.est_valide(), "{:?}", v.erreurs);
        assert!(v.avertissements.is_empty(), "{:?}", v.avertissements);
    }
    let (rr, rs) = (calcul(&pr), calcul(&ps));
    for r in [&rr, &rs] {
        let [bbio, dh, cep, cep_nr, ic] = phares(r);
        println!(
            "ADEME : Bbio {bbio:.1} ; DH {dh:.1} ; Cep {cep:.1} ; Cep,nr {cep_nr:.1} ; Ic {ic:.1}"
        );
        // Chaudière au gaz ou PAC : pas d'énergie renouvelable comptée, Cep = Cep,nr.
        assert!((cep - cep_nr).abs() < 1e-9);
    }
    let (br, bs) = (&rr.batiments[0], &rs.batiments[0]);
    // §13.1.3, éq. (2376) : SREF = SU hors habitation.
    assert_eq!((br.sref_m2, bs.sref_m2), (400.0, 600.0));

    // Tableau 56-1 (p. 335) : restaurants 10,0 et 0,5 m³/(h.m²), restaurants scolaires 8,0 et
    // 0,3. En occupation : 10 × 400 = 4 000 et 8 × 600 = 4 800 m³/h.
    for (b, q) in [(br, 4000.0), (bs, 4800.0)] {
        let d = b.zones[0].groupes[0].debits_bbio.as_ref().unwrap();
        assert_eq!((d.q_spec_repris_m3_h, d.q_spec_souffle_m3_h), (-q, q));
    }

    // Éq. (2493) : Cef = Coef_kWh_fr_par_DH × (min(DH ; seuil haut) − seuil bas)
    // × Coef_zone_clim_et_alt / CoefEP[électricité], par m² de SREF. Seuil bas 350 °C.h ;
    // seuil haut : DH_max du restaurant, 2 500 °C.h. Coefficient 0,009 (tableau 335, p. 1383) ;
    // 1,0 en H2b à 80 m (tableau 336) ; CoefEP de l'électricité 2,3. Le DH du restaurant
    // dépasse ce seuil ; celui du restaurant scolaire (32,5 °C.h) reste sous 350 : pas de forfait.
    assert!(br.dh_c_h.unwrap() > 2500.0);
    let attendu = 0.009 * (2500.0 - 350.0) * 1.0 / 2.3 * 400.0;
    assert!(
        (elec(&rr).refroidissement_kwh - attendu).abs() < 1e-6,
        "{}",
        elec(&rr).refroidissement_kwh
    );
    assert!(bs.dh_c_h.unwrap() < 350.0);
    assert_eq!(elec(&rs).refroidissement_kwh, 0.0);

    // Boucle d'ECS du restaurant scolaire : éq. (47), le réseau primaire est arrêté pendant les
    // vacances d'une zone affiliée à l'enseignement (usage 26, `ienseignement = 1`) ; éq. (1736).
    let sc = ScenariosZone::nouveau(UsageZone::RestaurantScolaire1Repas, &[600.0], 1).unwrap();
    let vacances = (0..8760).filter(|&h| sc.evaluer(h).i_vac <= 0.0).count();
    assert_eq!(vacances, 2712);
    let en_service = (8760 - vacances) as f64;
    let ecs = &bs.distributions_ecs_intergroupes[0];
    // Éq. (1747) : circulateurs de 60 W arrêtés pendant les vacances.
    assert!(
        (ecs.w_aux_kwh - 0.060 * en_service).abs() < 1e-6,
        "{}",
        ecs.w_aux_kwh
    );
    // Éq. (1742) à (1745) : moyenne de 50,5 °C (départ 53 °C, retour 48 °C), boucle de 50 m à
    // 0,25 W/(m.K) : 0,25 × 50 × (50,5 − 20) = 381,25 W en service.
    assert!(
        (ecs.pertes_vc_kwh - 0.38125 * en_service).abs() < 1e-6,
        "{}",
        ecs.pertes_vc_kwh
    );
    assert_eq!(ecs.pertes_hvc_kwh, 0.0);

    // Figure 34 : famille « restauration », Pocc = 0,9 et Pderog = 0,1, pour les six usages.
    for u in [13, 14, 15, 16, 26, 27] {
        let (f, p) = famille_et_parts(u).unwrap();
        assert_eq!(
            (f, p.p_occ, p.p_derog),
            (FamilleGpm::Restauration, 0.9, 0.1)
        );
    }
}

/// Variantes de sens (météo ADEME).
#[test]
fn sens_des_effets() {
    // Restaurant : volets motorisés (figure 37-1, colonne « Restauration » du tableau 34).
    let base = restaurant();
    let ref_ = calcul(&base);
    let [bbio, dh, cep, _, ic] = phares(&ref_);

    // 1. Plus d'isolant (U des parois divisés par deux) : Bbio, Cep et Ic énergie baissent.
    let mut p = base.clone();
    for t in &mut p.catalogue.parois {
        t.u_k_w_m2k /= 2.0;
    }
    let [bbio_i, _, cep_i, _, ic_i] = phares(&calcul(&p));
    println!("isolant : Bbio {bbio:.1} → {bbio_i:.1} ; Cep {cep:.1} → {cep_i:.1} ; Ic {ic:.1} → {ic_i:.1}");
    assert!(bbio_i < bbio && cep_i < cep && ic_i < ic);
    assert!((bbio - 272.1).abs() < 0.05 && (bbio_i - 258.0).abs() < 0.05);

    // 2. Volets manuels non motorisés au lieu de motorisés : le DH change (matrices de la
    //    figure 37-1 et plancher du tableau 34 selon `Type_GPM`).
    let mut p = base.clone();
    p.catalogue.baies[0]
        .protection_mobile
        .as_mut()
        .unwrap()
        .gestion = GestionPm::ManuelleNonMotorisee;
    let dh_manuel = phares(&calcul(&p))[1];
    println!("volets non motorisés : DH {dh:.1} → {dh_manuel:.1}");
    assert_ne!(dh_manuel, dh);
    assert!((dh - 2670.9).abs() < 0.05 && (dh_manuel - 2619.2).abs() < 0.05);

    // 3. Puissance d'éclairage de la cuisine doublée : l'éclairage augmente, le Bbio non (puissance
    //    conventionnelle, éq. 782).
    let mut p = base.clone();
    let l = &mut p.batiments[0].zones[0].groupes[0].locaux[1];
    assert_eq!(l.type_local, TypeLocal::Cuisine);
    l.eclairage.p_ecl_tot_w_m2 *= 2.0;
    let r = calcul(&p);
    println!(
        "cuisine : éclairage {:.0} → {:.0} kWh",
        elec(&ref_).eclairage_kwh,
        elec(&r).eclairage_kwh
    );
    assert_eq!(phares(&r)[0], bbio);
    assert!(elec(&r).eclairage_kwh > elec(&ref_).eclairage_kwh);
    assert!(
        (elec(&ref_).eclairage_kwh - 17332.3).abs() < 0.5
            && (elec(&r).eclairage_kwh - 22326.0).abs() < 1.0
    );

    // 4. Restaurant scolaire : circulateurs de la boucle en fonctionnement permanent
    //    (« Typegest_circ-e = 0 ») : 8 760 h au lieu des heures hors vacances ; pertes
    //    inchangées (éq. 1744, lecture Q71).
    let base = scolaire();
    let ref_ = calcul(&base);
    let mut p = base.clone();
    boucle(&mut p).arret_circulateurs_vacances = Some(false);
    let r = calcul(&p);
    let (a, b) = (
        &ref_.batiments[0].distributions_ecs_intergroupes[0],
        &r.batiments[0].distributions_ecs_intergroupes[0],
    );
    println!(
        "circulateurs permanents : {:.1} → {:.1} kWh",
        a.w_aux_kwh, b.w_aux_kwh
    );
    assert!((b.w_aux_kwh - 0.060 * 8760.0).abs() < 1e-6);
    assert!(b.w_aux_kwh > a.w_aux_kwh);
    assert_eq!(a.pertes_vc_kwh, b.pertes_vc_kwh);
    assert!(phares(&r)[2] > phares(&ref_)[2]);

    // 5. Restaurant scolaire : stores enroulables manuels non motorisés au lieu de motorisés
    //    (figure 40-1) : le DH change.
    let dh_s = phares(&ref_)[1];
    let mut p = base.clone();
    p.catalogue.baies[0]
        .protection_mobile
        .as_mut()
        .unwrap()
        .gestion = GestionPm::ManuelleNonMotorisee;
    let dh_s2 = phares(&calcul(&p))[1];
    println!("stores non motorisés : DH {dh_s:.2} → {dh_s2:.2}");
    assert_ne!(dh_s2, dh_s);
    assert!((dh_s - 32.8).abs() < 0.05 && (dh_s2 - 35.5).abs() < 0.06);
}

/// Les autres usages du lot se calculent : restaurant en usages 14, 15 et 16 (mêmes locaux),
/// restaurant scolaire en usage 27 ; usage 16 et besoin d'ECS de 4,7 L/m² (Q57).
#[test]
fn usages_14_15_16_et_27() {
    let ref_ = calcul(&restaurant());
    let ecs_13 = elec(&ref_).ecs_kwh;
    let mut ecs = Vec::new();
    for (usage, numero) in [
        (UsageZone::Restaurant1Repas5Jours, 14),
        (UsageZone::Restaurant2Repas7Jours, 15),
        (UsageZone::Restaurant2Repas6Jours, 16),
    ] {
        let mut p = restaurant();
        p.batiments[0].zones[0].usage = usage;
        assert_eq!(usage.numero(), numero);
        let v = valider(&p);
        assert!(v.est_valide(), "{usage:?} : {:?}", v.erreurs);
        let r = calcul(&p);
        let [bbio, dh, cep, _, _] = phares(&r);
        println!(
            "{usage:?} : Bbio {bbio:.1} ; DH {dh:.1} ; Cep {cep:.1} ; ECS {:.0} kWh",
            elec(&r).ecs_kwh
        );
        // Tableau 56-1 : 10,0 m³/(h.m²) en occupation pour les usages 13 à 16.
        let d = r.batiments[0].zones[0].groupes[0]
            .debits_bbio
            .as_ref()
            .unwrap();
        assert!((d.q_spec_souffle_m3_h - 4000.0).abs() < 1e-9);
        // Usage 16 (Q57, besoin de 4,7 L/m²) : Cep 189,6 et ECS 4 344 kWh.
        if numero == 16 {
            assert!((cep - 189.6).abs() < 0.05, "{cep}");
            assert!(
                (elec(&r).ecs_kwh - 4344.0).abs() < 1.0,
                "{}",
                elec(&r).ecs_kwh
            );
        }
        ecs.push(elec(&r).ecs_kwh);
    }
    // Besoins unitaires du classeur (L par m² et par semaine) : 13 → 19,5 ; 14 → 2,2 ;
    // 15 → 5,3 ; 16 → 4,7 (Q57 : le tableau 277 écrit 19,5 pour l'usage 16, lecture non
    // retenue) ; 26 → 4,5 ; 27 → 9,8.
    for (u, a) in [
        (13, 19.5),
        (14, 2.2),
        (15, 5.3),
        (16, 4.7),
        (26, 4.5),
        (27, 9.8),
    ] {
        assert_eq!(
            thbce_data::scenarios::usage(u)
                .unwrap()
                .ecs
                .volume_l_semaine_unite,
            Some(a),
            "usage {u}"
        );
    }
    // Les besoins d'ECS suivent ces valeurs : usage 16 (4,7) bien en dessous de l'usage 13
    // (19,5) et de l'usage 15 (5,3), au-dessus de l'usage 14 (2,2).
    println!("ECS : 13 → {ecs_13:.0} ; 14, 15, 16 → {ecs:?}");
    assert!(ecs[2] < ecs_13 / 2.0 && ecs[1] > ecs[0]);

    // Restaurant scolaire en usage 27 (3 repas par jour, 5 jours sur 7) : mêmes locaux, 9,8
    // L/m² d'ECS ; tableau 56-1 : 8,0 m³/(h.m²).
    let mut p = scolaire();
    p.batiments[0].zones[0].usage = UsageZone::RestaurantScolaire3Repas;
    let v = valider(&p);
    assert!(v.est_valide(), "{:?}", v.erreurs);
    let r = calcul(&p);
    let r26 = calcul(&scolaire());
    println!(
        "usage 27 : Bbio {:.1} ; DH {:.1} ; Cep {:.1} ; ECS {:.0} → {:.0} kWh",
        phares(&r)[0],
        phares(&r)[1],
        phares(&r)[2],
        elec(&r26).ecs_kwh,
        elec(&r).ecs_kwh
    );
    let d = r.batiments[0].zones[0].groupes[0]
        .debits_bbio
        .as_ref()
        .unwrap();
    assert!((d.q_spec_souffle_m3_h - 4800.0).abs() < 1e-9);
    assert!(elec(&r).ecs_kwh > elec(&r26).ecs_kwh);
    assert!((elec(&r26).ecs_kwh - 6527.0).abs() < 1.0 && (elec(&r).ecs_kwh - 10436.0).abs() < 1.0);
    assert!((phares(&r)[2] - 188.3).abs() < 0.05);
}

/// Refus et erreurs de saisie propres au lot.
#[test]
fn refus() {
    // Boucle d'ECS desservant un restaurant scolaire : la gestion des circulateurs est requise
    // (usages 26 et 27 affiliés à l'enseignement, « Typegest_circ-e », Q71).
    let mut p = scolaire();
    boucle(&mut p).arret_circulateurs_vacances = None;
    let v = valider(&p);
    assert!(
        v.erreurs.iter().any(|e| e
            .chemin
            .ends_with("reseau.boucle.arret_circulateurs_vacances")),
        "{:?}",
        v.erreurs
    );
    // Restaurant (usage 13, non affilié) : l'arrêt des circulateurs en vacances est refusé.
    let mut p = scolaire();
    p.batiments[0].zones[0].usage = UsageZone::RestaurantContinu;
    assert!(!valider(&p).est_valide());
    // Pas de chambre dans un restaurant (tableau 78).
    let mut p = restaurant();
    p.batiments[0].zones[0].groupes[0].locaux[0].type_local = TypeLocal::ChambreSansCuisine;
    let v = valider(&p);
    assert!(
        v.erreurs
            .iter()
            .any(|e| e.chemin.ends_with("locaux[0].type_local") && !e.hors_perimetre),
        "{:?}",
        v.erreurs
    );
    // Permis de 2025 : usage non soumis à la RE2020 avant le 1er mai 2026 (Q68).
    let mut p = restaurant();
    let x = p.batiments[0].exigences.as_mut().unwrap();
    x.annee_depot_permis = 2025;
    x.mois_depot_permis = None;
    assert!(!valider(&p).est_valide());
}

fn meteo_reglementaire(p: &Projet) -> MeteoProjet {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO pour lancer ce test");
    let zone = p.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

/// Valeurs phares du restaurant (H2b) et du restaurant scolaire (H1b) sur la météo
/// réglementaire : nombres annuels seulement.
#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn restauration_sur_meteo_reglementaire() {
    let restaurant_nr: &[&str] = &["bbio", "cep_nr"];
    let scolaire_nr: &[&str] = &["ic_energie"];
    for (p, attendu, non_resp) in [
        (restaurant(), RESTAURANT, restaurant_nr),
        (scolaire(), SCOLAIRE, scolaire_nr),
    ] {
        let r = calculer(&p, &meteo_reglementaire(&p), &Options { arrondir: false }).unwrap();
        assert!(r.reglementaire);
        let v = phares(&r);
        let [bbio, dh, cep, cep_nr, ic] = v;
        println!(
            "{} réglementaire : Bbio {bbio:.1} ; DH {dh:.1} ; Cep {cep:.1} ; Cep,nr {cep_nr:.1} ; Ic {ic:.1}",
            p.batiments[0].id
        );
        for (x, a) in v.iter().zip(attendu) {
            assert!((x - a).abs() <= 0.05 + 1e-9, "{v:?} ≠ {attendu:?}");
        }
        let ex = r.batiments[0]
            .indicateurs
            .as_ref()
            .unwrap()
            .exigences
            .as_ref()
            .unwrap();
        let nr: Vec<&str> = ex
            .verifications
            .iter()
            .filter(|v| !v.respectee)
            .map(|v| v.indicateur.as_str())
            .collect();
        println!("non respectées : {nr:?}");
        assert_eq!(nr, non_resp);
        assert_eq!(ex.conforme, non_resp.is_empty());
    }
}

/// Restaurant, météo réglementaire H2b (lot 0c3, 18 novembre 2026) : Bbio, DH, Cep, Cep,nr, Ic
/// énergie.
const RESTAURANT: [f64; 5] = [248.4, 1878.7, 352.9, 352.9, 416.4];
/// Restaurant scolaire, météo réglementaire H1b.
const SCOLAIRE: [f64; 5] = [80.7, 14.8, 125.5, 125.5, 714.4];
