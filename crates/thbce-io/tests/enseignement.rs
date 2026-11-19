// SPDX-License-Identifier: GPL-3.0-or-later
//! Usages d'enseignement et affiliés (lot 0c1) : enseignement primaire (4) et secondaire (5),
//! médiathèques et bibliothèques (6), bâtiments universitaires (7), petite enfance (12).
//! Exemples `tests/cas/ecole-primaire-ademe` et `tests/cas/creche-pac-ademe` sur la météo
//! ouverte ADEME (leurs valeurs sont vérifiées par leur `attendu.json`) ; calculs à la main ;
//! sens des effets ; arrêt de l'ECS pendant les vacances scolaires ; refus.
//!
//! Valeurs phares sur la météo réglementaire :
//! THBCE_METEO=/chemin/vers/meteo.csv cargo t-regl enseignement_sur_meteo_reglementaire

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{GestEcl, GradEcl, Projet, ReseauEcsIntergroupe, UsageZone};
use thbce_core::resultats::Resultats;
use thbce_core::scenarios::scenarios_conventionnels::ScenariosZone;
use thbce_core::{calculer, valider, Options};
use thbce_data::eclairage::TypeLocal;
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn exemple(nom: &str) -> Projet {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn ecole() -> Projet {
    exemple("ecole-primaire-ademe")
}

fn creche() -> Projet {
    exemple("creche-pac-ademe")
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

fn eclairage_kwh(r: &Resultats) -> f64 {
    r.batiments[0]
        .indicateurs
        .as_ref()
        .unwrap()
        .electricite_consommee
        .eclairage_kwh
}

fn boucle(p: &mut Projet) -> &mut thbce_core::entree::BoucleEcs {
    match &mut p.batiments[0].distributions_ecs_intergroupes[0].reseau {
        ReseauEcsIntergroupe::Boucle(b) => b,
        _ => panic!("l'école a une boucle d'ECS"),
    }
}

/// Valeurs recalculées à la main, indépendamment du code des postes concernés : débits
/// conventionnels du Bbio (tableau 56-1), forfait de refroidissement (éq. 2493, tableau 335),
/// circulateurs et pertes de la boucle d'ECS de l'école (éq. 1736, 1744 et 1747).
#[test]
fn ecole_et_creche_a_la_main() {
    let (pe, pc) = (ecole(), creche());
    for p in [&pe, &pc] {
        let v = valider(p);
        assert!(v.est_valide(), "{:?}", v.erreurs);
        assert!(v.avertissements.is_empty(), "{:?}", v.avertissements);
    }
    let (re, rc) = (calcul(&pe), calcul(&pc));
    for r in [&re, &rc] {
        let [bbio, dh, cep, cep_nr, ic] = phares(r);
        println!(
            "ADEME : Bbio {bbio:.1} ; DH {dh:.1} ; Cep {cep:.1} ; Cep,nr {cep_nr:.1} ; Ic {ic:.1}"
        );
        // Gaz, électricité : pas d'énergie renouvelable comptée, Cep = Cep,nr.
        assert!((cep - cep_nr).abs() < 1e-9);
    }
    let (be, bc) = (&re.batiments[0], &rc.batiments[0]);
    // §13.1.3, éq. (2376) : SREF = SU hors habitation.
    assert_eq!((be.sref_m2, bc.sref_m2), (1500.0, 400.0));

    // Tableau 56-1 (p. 334) : enseignement primaire 7,2 et 0,38 m³/(h.m²) ; petite enfance 4,7
    // et 0,75. En occupation : 7,2 × 1 500 = 10 800 et 4,7 × 400 = 1 880 m³/h.
    let d = be.zones[0].groupes[0].debits_bbio.as_ref().unwrap();
    assert_eq!(
        (d.q_spec_repris_m3_h, d.q_spec_souffle_m3_h),
        (-10_800.0, 10_800.0)
    );
    let d = bc.zones[0].groupes[0].debits_bbio.as_ref().unwrap();
    assert!((d.q_spec_souffle_m3_h - 1880.0).abs() < 1e-9);

    // Éq. (2493) : Cef = Coef_kWh_fr_par_DH × (min(DH ; seuil haut) − seuil bas)
    // × Coef_zone_clim_et_alt / CoefEP[électricité], par m² de SREF. Seuil bas 350 °C.h ;
    // seuil haut : DH_max de la catégorie 1, 900 °C.h pour l'enseignement primaire (hors H2d et
    // H3) et 550 °C.h pour la petite enfance (annexe de l'article R. 172-4, chapitre III,
    // tableaux DH_maxcat) ; les deux DH dépassent ce seuil. Coefficients 0,016 (usage 4) et
    // 0,009 (usage 12) du tableau 335 (p. 1383) ; 1,0 en H1b à 210 m et en H2b à 80 m
    // (tableau 336) ; CoefEP de l'électricité 2,3.
    let fr = |r: &Resultats| {
        r.batiments[0]
            .indicateurs
            .as_ref()
            .unwrap()
            .electricite_consommee
            .refroidissement_kwh
    };
    assert!(be.dh_c_h.unwrap() > 900.0 && bc.dh_c_h.unwrap() > 550.0);
    let attendu_e = 0.016 * (900.0 - 350.0) * 1.0 / 2.3 * 1500.0;
    let attendu_c = 0.009 * (550.0 - 350.0) * 1.0 / 2.3 * 400.0;
    assert!(
        (fr(&re) - attendu_e).abs() < 1e-6,
        "{} ≠ {attendu_e}",
        fr(&re)
    );
    assert!(
        (fr(&rc) - attendu_c).abs() < 1e-6,
        "{} ≠ {attendu_c}",
        fr(&rc)
    );

    // Boucle d'ECS de l'école : éq. (47), le réseau primaire est arrêté pendant les vacances
    // d'une zone d'enseignement (`Ivac = 0`) ; éq. (1736), la boucle ne dessert que cette zone.
    // Heures de vacances comptées sur les scénarios conventionnels de l'usage 4.
    let sc = ScenariosZone::nouveau(UsageZone::EnseignementPrimaire, &[1500.0], 1).unwrap();
    let vacances = (0..8760).filter(|&h| sc.evaluer(h).i_vac <= 0.0).count();
    assert!((1000..2500).contains(&vacances), "{vacances}");
    let en_service = (8760 - vacances) as f64;
    // Les besoins d'ECS ne dépendent pas de `iecs` (lecture Q71) : la clé de répartition `ah`
    // de l'usage 4 est nulle pendant presque toutes les heures de vacances, pas toutes.
    let puisages_en_vacances = (0..8760)
        .filter(|&h| {
            let s = sc.evaluer(h);
            s.i_vac <= 0.0 && s.ah > 0.0
        })
        .count();
    println!("vacances : {vacances} h, dont {puisages_en_vacances} h de puisage");
    let ecs = &be.distributions_ecs_intergroupes[0];
    // Éq. (1747) : circulateurs de 60 W arrêtés pendant les vacances (« Typegest_circ-e = 1 »).
    assert!(
        (ecs.w_aux_kwh - 0.060 * en_service).abs() < 1e-6,
        "{}",
        ecs.w_aux_kwh
    );
    // Éq. (1742) à (1745) : départ 53 °C (distribution du groupe, tableau 278), retour 48 °C,
    // moyenne 50,5 °C ; 0,25 W/(m.K) × 80 m × (50,5 − 20) = 610 W hors vacances, 0 pendant.
    assert!(
        (ecs.pertes_vc_kwh - 0.610 * en_service).abs() < 1e-6,
        "{}",
        ecs.pertes_vc_kwh
    );
    assert_eq!(ecs.pertes_hvc_kwh, 0.0);
}

/// Arrêt de l'ECS pendant les vacances scolaires et autres variantes de sens (météo ADEME).
#[test]
fn sens_des_effets() {
    let base = ecole();
    let ref_ = calcul(&base);
    let [bbio, dh, cep, _, ic] = phares(&ref_);

    // 1. Circulateurs de la boucle en fonctionnement permanent (« Typegest_circ-e = 0 ») :
    //    8 760 h au lieu des heures hors vacances ; les pertes de la boucle ne changent pas,
    //    l'éq. (1744) les annulant pendant les vacances quelle que soit la gestion (lecture Q71).
    let mut p = base.clone();
    boucle(&mut p).arret_circulateurs_vacances = Some(false);
    let r = calcul(&p);
    let (a, b) = (
        &ref_.batiments[0].distributions_ecs_intergroupes[0],
        &r.batiments[0].distributions_ecs_intergroupes[0],
    );
    println!(
        "circulateurs permanents : {:.1} → {:.1} kWh ; Cep {cep:.2} → {:.2}",
        a.w_aux_kwh,
        b.w_aux_kwh,
        phares(&r)[2]
    );
    assert!((b.w_aux_kwh - 0.060 * 8760.0).abs() < 1e-6);
    assert!(b.w_aux_kwh > a.w_aux_kwh);
    assert_eq!(a.pertes_vc_kwh, b.pertes_vc_kwh);
    assert!(phares(&r)[2] > cep);

    // 2. La même école en bureaux (usage 3, sans vacances scolaires) : la boucle fonctionne
    //    toute l'année et ses pertes augmentent. L'arrêt des circulateurs y est refusé.
    let mut p = base.clone();
    p.batiments[0].zones[0].usage = UsageZone::Bureaux;
    assert!(!valider(&p).est_valide());
    boucle(&mut p).arret_circulateurs_vacances = None;
    for l in &mut p.batiments[0].zones[0].groupes[0].locaux {
        l.type_local = match l.type_local {
            TypeLocal::SalleClasse | TypeLocal::SalleRepos => TypeLocal::BureauStandard,
            TypeLocal::SanitairesVestiaires => TypeLocal::SanitairesCollectifs,
            t => t,
        };
        if l.type_local == TypeLocal::BureauStandard && l.eclairage.p_ecl_tot_w_m2 < 10.0 {
            // Éclairage mobilier des bureaux (§7.1.3.2.3, Q59).
            l.eclairage.eff_ecl_immo_projet_w_m2_100lux = Some(1.5);
            l.eclairage.ei_proj_lux = Some(500.0);
        }
    }
    let v = valider(&p);
    assert!(v.est_valide(), "{:?}", v.erreurs);
    let r = calcul(&p);
    let b = &r.batiments[0].distributions_ecs_intergroupes[0];
    assert!((b.pertes_vc_kwh - 0.610 * 8760.0).abs() < 1e-6);
    assert!((b.w_aux_kwh - 0.060 * 8760.0).abs() < 1e-6);

    // 3. Plus d'isolant (U des murs, de la toiture et du plancher divisés par deux) : le Bbio,
    //    le Cep et l'Ic énergie baissent.
    let mut p = base.clone();
    for t in &mut p.catalogue.parois {
        t.u_k_w_m2k /= 2.0;
    }
    let r = calcul(&p);
    let [bbio_isole, _, cep_isole, _, ic_isole] = phares(&r);
    println!("isolant : Bbio {bbio:.1} → {bbio_isole:.1} ; Cep {cep:.1} → {cep_isole:.1} ; Ic {ic:.1} → {ic_isole:.1}");
    assert!(bbio_isole < bbio && cep_isole < cep && ic_isole < ic);

    // 4. Éclairage des salles de classe sans gradation ni détection : davantage de
    //    consommation ; le Bbio (puissance et gestion conventionnelles, éq. 782) ne bouge pas.
    let mut p = base.clone();
    let e = &mut p.batiments[0].zones[0].groupes[0].locaux[0].eclairage;
    e.gest_ecl = GestEcl::InterrupteurManuel;
    e.grad_ecl = GradEcl::Aucune;
    e.p_ecl_aux_w_m2 = None;
    e.fractionne = false;
    let r = calcul(&p);
    println!(
        "classes sans gestion : éclairage {:.0} → {:.0} kWh ; Cep {cep:.1} → {:.1}",
        eclairage_kwh(&ref_),
        eclairage_kwh(&r),
        phares(&r)[2]
    );
    assert_eq!(phares(&r)[0], bbio);
    assert!(eclairage_kwh(&r) > eclairage_kwh(&ref_) && phares(&r)[2] > cep);

    // 5. Protections mobiles : stores enroulables motorisés au lieu de manuels (figure 40,
    //    famille « enseignement ») : le DH change.
    let mut p = base.clone();
    if let Some(pm) = p.catalogue.baies[0].protection_mobile.as_mut() {
        pm.gestion = Some(thbce_core::entree::GestionPm::ManuelleMotorisee);
    }
    let r = calcul(&p);
    println!("stores motorisés : DH {dh:.1} → {:.1}", phares(&r)[1]);
    assert_ne!(phares(&r)[1], dh);
}

/// Les autres usages du lot se calculent : la crèche en médiathèque (usage 6, famille
/// « bureau » des protections mobiles, figure 34), l'école en secondaire (5) et en bâtiment
/// universitaire (7), avec les locaux du tableau 78 de chaque usage.
#[test]
fn usages_5_6_et_7() {
    // Médiathèque : locaux de l'usage 6 (tableau 78, p. 488).
    let mut p = creche();
    p.batiments[0].zones[0].usage = UsageZone::MediathequeBibliotheque;
    let types = [
        TypeLocal::CentreDocumentation,
        TypeLocal::LocalService,
        TypeLocal::BureauStandard,
        TypeLocal::SalleReunion,
        TypeLocal::CirculationAccueil,
        TypeLocal::SanitairesCollectifs,
    ];
    for (l, t) in p.batiments[0].zones[0].groupes[0]
        .locaux
        .iter_mut()
        .zip(types)
    {
        l.type_local = t;
    }
    let v = valider(&p);
    assert!(v.est_valide(), "{:?}", v.erreurs);
    let r = calcul(&p);
    let [bbio, dh, cep, _, _] = phares(&r);
    println!("médiathèque : Bbio {bbio:.1} ; DH {dh:.1} ; Cep {cep:.1}");
    assert!(bbio > 0.0 && cep > 0.0);
    // Tableau 56-1 : 5,6 m³/(h.m²) en occupation.
    let d = r.batiments[0].zones[0].groupes[0]
        .debits_bbio
        .as_ref()
        .unwrap();
    assert!((d.q_spec_souffle_m3_h - 5.6 * 400.0).abs() < 1e-9);

    // Secondaire et universitaire : locaux de l'usage (tableau 78, p. 488).
    for (usage, types, param) in [
        (
            UsageZone::EnseignementSecondaire,
            [
                TypeLocal::SalleClasse,
                TypeLocal::BureauStandard,
                TypeLocal::SalleReunion,
                TypeLocal::SalleProfesseurs,
                TypeLocal::CirculationAccueil,
                TypeLocal::SanitairesCollectifs,
            ],
            4.7,
        ),
        (
            UsageZone::Universitaire,
            [
                TypeLocal::SalleClasse,
                TypeLocal::BureauStandard,
                TypeLocal::SalleReunion,
                TypeLocal::SalleConference,
                TypeLocal::CirculationAccueil,
                TypeLocal::SanitairesCollectifs,
            ],
            7.4,
        ),
    ] {
        let mut p = ecole();
        p.batiments[0].zones[0].usage = usage;
        if usage == UsageZone::Universitaire {
            // Usage soumis à partir du 1er mai 2026 (Q68).
            let x = p.batiments[0].exigences.as_mut().unwrap();
            x.annee_depot_permis = 2027;
        }
        for (l, t) in p.batiments[0].zones[0].groupes[0]
            .locaux
            .iter_mut()
            .zip(types)
        {
            l.type_local = t;
        }
        let v = valider(&p);
        assert!(v.est_valide(), "{usage:?} : {:?}", v.erreurs);
        let r = calcul(&p);
        let [bbio, dh, cep, _, _] = phares(&r);
        println!("{usage:?} : Bbio {bbio:.1} ; DH {dh:.1} ; Cep {cep:.1}");
        let d = r.batiments[0].zones[0].groupes[0]
            .debits_bbio
            .as_ref()
            .unwrap();
        assert!((d.q_spec_souffle_m3_h - param * 1500.0).abs() < 1e-9);
        // L'usage 7 n'a pas de vacances (grille annuelle sans semaine inoccupée) : la boucle
        // fonctionne toute l'année ; le secondaire a ses vacances.
        let w = r.batiments[0].distributions_ecs_intergroupes[0].w_aux_kwh;
        if usage == UsageZone::Universitaire {
            assert!((w - 0.060 * 8760.0).abs() < 1e-6, "{w}");
        } else {
            assert!(w < 0.060 * 8760.0 - 1.0, "{w}");
        }
    }
}

/// Refus et erreurs de saisie propres au lot.
#[test]
fn refus() {
    let erreurs = |p: &Projet| -> Vec<String> {
        valider(p)
            .erreurs
            .iter()
            .map(|e| e.chemin.clone())
            .collect()
    };
    // Boucle d'ECS desservant une zone d'enseignement : la gestion des circulateurs est requise
    // (« Typegest_circ-e », sans valeur conventionnelle).
    let mut p = ecole();
    boucle(&mut p).arret_circulateurs_vacances = None;
    let e = erreurs(&p);
    assert!(
        e.iter()
            .any(|c| c.ends_with("reseau.boucle.arret_circulateurs_vacances")),
        "{e:?}"
    );
    // Usage 7 : pas de « local de service » dans le tableau 78 (lecture Q70).
    let mut p = ecole();
    p.batiments[0].zones[0].usage = UsageZone::Universitaire;
    p.batiments[0].zones[0].groupes[0].locaux[3].type_local = TypeLocal::LocalService;
    let v = valider(&p);
    assert!(
        v.erreurs
            .iter()
            .any(|e| e.chemin.ends_with("locaux[3].type_local") && !e.hors_perimetre),
        "{:?}",
        v.erreurs
    );
    // Petite enfance : pas de salle de classe (tableau 78).
    let mut p = creche();
    p.batiments[0].zones[0].groupes[0].locaux[0].type_local = TypeLocal::SalleClasse;
    assert!(!valider(&p).est_valide());
    // Crèche avec un permis de 2025 : usage non soumis à la RE2020 avant le 1er mai 2026 (Q68).
    let mut p = creche();
    if let Some(x) = p.batiments[0].exigences.as_mut() {
        x.annee_depot_permis = 2025;
        x.mois_depot_permis = None;
    }
    assert!(!valider(&p).est_valide());
    // Ascenseurs et parkings restent hors périmètre hors habitation : inchangé (lot 0a).
}

fn meteo_reglementaire(p: &Projet) -> MeteoProjet {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO pour lancer ce test");
    let zone = p.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

/// Valeurs phares de l'école (H1b) et de la crèche (H2b) sur la météo réglementaire : nombres
/// annuels seulement.
#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn enseignement_sur_meteo_reglementaire() {
    let ecole_nr: &[&str] = &["ic_energie"];
    let creche_nr: &[&str] = &["bbio", "dh"];
    for (p, attendu, non_resp) in [(ecole(), ECOLE, ecole_nr), (creche(), CRECHE, creche_nr)] {
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
        // Exigences : l'école (chaudière au gaz) ne dépasse que l'Ic énergie ; la crèche
        // dépasse son Bbio et son DH.
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
        assert!(!ex.conforme);
    }
}

/// École, météo réglementaire H1b (lot 0c1, 18 novembre 2026) : Bbio, DH, Cep, Cep,nr, Ic énergie.
const ECOLE: [f64; 5] = [80.8, 862.9, 72.5, 72.5, 351.8];
/// Crèche, météo réglementaire H2b.
const CRECHE: [f64; 5] = [152.4, 586.3, 113.0, 113.0, 141.7];
