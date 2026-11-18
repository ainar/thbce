// SPDX-License-Identifier: GPL-3.0-or-later
//! Récupérateur de chaleur sur les eaux grises (fiche 9.21, lot 1-ecs2, Q108) de bout en bout :
//! exemple `tests/cas/maison-recuperateur-eaux-grises-ademe` (position 0, individuel), les trois
//! positions, l'efficacité nulle (sans effet), le récupérateur collectif, la pompe de relevage,
//! les refus de la validation et les variantes de la lecture Q108.
//!
//! Valeurs phares sur la météo réglementaire :
//! THBCE_METEO=/chemin/vers/meteo.csv cargo t-regl recuperateur_sur_meteo_reglementaire

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::bati::assemblage_batiment::BatimentHoraire;
use thbce_core::bati::assemblage_groupe::{LecturePreechauffageEcs, Lectures};
use thbce_core::chauffage_refroidissement::calculs_generation::{Energie, Poste};
use thbce_core::climat::{ClimatExterieur, ConfortAdaptatif, MeteoProjet};
use thbce_core::conventions::ModeCalcul;
use thbce_core::entree::{
    ConfigurationRecuperateur, EchangeurEauxGrises, OrigineEfficaciteEauxGrises,
    PositionRecuperateur, Projet, RecuperateurCollectifEauxGrises, RecuperateurEauxGrises,
    UniteRecuperateur,
};
use thbce_core::resultats::{Resultats, ResultatsEcsEmetteur};
use thbce_core::{calculer, valider, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cas(nom: &str) -> Projet {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

/// Maison d'exemple sans récupérateur (chaudière gaz et ballon de 200 L).
fn sans_recuperateur() -> Projet {
    cas("maison-chaudiere-gaz-ademe")
}

fn echangeur(efficacite: f64) -> EchangeurEauxGrises {
    EchangeurEauxGrises {
        origine: Some(OrigineEfficaciteEauxGrises::Certifiee),
        efficacite: Some(efficacite),
        c_trans: Some(1.0),
    }
}

fn recuperateur(position: PositionRecuperateur, efficacite: f64) -> RecuperateurEauxGrises {
    RecuperateurEauxGrises {
        configuration: ConfigurationRecuperateur::Individuelle,
        position: Some(position),
        recuperateur_collectif: None,
        unites: vec![UniteRecuperateur {
            nb_echangeurs: 1,
            nb_douches_relies: 1,
            nb_douches_non_relies: None,
            echangeur: Some(echangeur(efficacite)),
            p_relevage_nominale_w: None,
        }],
        l_evac_vc_m: 3.0,
        l_evac_vnc_m: 0.0,
    }
}

fn avec(r: RecuperateurEauxGrises) -> Projet {
    let mut p = sans_recuperateur();
    p.batiments[0].zones[0].groupes[0].emetteurs_ecs[0].recuperateur = Some(r);
    p
}

fn meteo_ademe(p: &Projet) -> MeteoProjet {
    let zone = p.localisation.zone_climatique_retenue();
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

fn calcul(p: &Projet) -> Resultats {
    calculer(p, &meteo_ademe(p), &Options { arrondir: false }).unwrap()
}

fn emetteur(r: &Resultats) -> &ResultatsEcsEmetteur {
    &r.batiments[0].zones[0].groupes[0]
        .systemes
        .as_ref()
        .unwrap()
        .ecs[0]
}

/// Gaz consommé pour l'ECS, en kWh, et énergie finale totale de l'ECS.
fn gaz_ecs(r: &Resultats) -> f64 {
    r.batiments[0]
        .consommations
        .as_ref()
        .unwrap()
        .annuel
        .ecs
        .gaz_kwh
}

/// Gaz de l'ECS en Th-C avec une lecture donnée du préchauffage, en Wh.
fn gaz_ecs_selon(p: &Projet, lectures: Lectures) -> f64 {
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
    b.systemes.cumuls_cef_groupes[0]
        .annuel()
        .valeur(Poste::Ecs, Energie::Gaz)
}

#[test]
fn l_exemple_est_valide_et_calculable() {
    let p = cas("maison-recuperateur-eaux-grises-ademe");
    assert!(valider(&p).est_valide(), "{:?}", valider(&p));
    let r = calcul(&p);
    let e = emetteur(&r);
    let rec = e.recuperateur.as_ref().unwrap();
    // Position 0 : le volume d'eau chaude appelé baisse (éq. 2184), donc les besoins (éq. 2186).
    assert!(e.q_w_kwh < rec.q_w_sans_recuperateur_kwh);
    // Les besoins sans récupérateur sont ceux de la maison sans récupérateur (éq. 1693).
    let base = calcul(&sans_recuperateur());
    assert!((rec.q_w_sans_recuperateur_kwh - emetteur(&base).q_w_kwh).abs() < 1e-9);
    // Eau froide préchauffée entre 10 et 40 °C (θeg = 37 °C moins les pertes du conduit).
    let t = rec.theta_entrant_moyen_c.unwrap();
    assert!((14.0..37.0).contains(&t), "{t}");
    assert!(gaz_ecs(&r) < gaz_ecs(&base));
}

/// Efficacité nulle : l'eau froide n'est pas préchauffée (éq. 2218 : `θep = θef`), toutes les
/// éq. (2181), (2184) et (2187) retrouvent celles de l'émetteur sans récupérateur.
#[test]
fn efficacite_nulle_sans_effet() {
    let base = calcul(&sans_recuperateur());
    for pos in [
        PositionRecuperateur::AmontMitigeursEtBallon,
        PositionRecuperateur::AmontBallon,
        PositionRecuperateur::AmontMitigeurs,
    ] {
        let r = calcul(&avec(recuperateur(pos, 0.0)));
        assert!(
            (emetteur(&r).q_w_kwh - emetteur(&base).q_w_kwh).abs() < 1e-9,
            "{pos:?}"
        );
        assert!((gaz_ecs(&r) - gaz_ecs(&base)).abs() < 1e-6, "{pos:?}");
    }
}

/// À la lettre du texte, la position 1 ne change que `θentrant` : la chaleur de l'eau froide
/// préchauffée est reprise au ballon par le même volume prélevé, sans gain pour la génération
/// (Q108 point 6). La variante qui déduit la chaleur récupérée de la demande la chiffre.
#[test]
fn position_1_sans_effet_a_la_lettre() {
    let p = avec(recuperateur(PositionRecuperateur::AmontBallon, 0.5));
    let base = gaz_ecs_selon(&sans_recuperateur(), Lectures::default());
    let texte = gaz_ecs_selon(&p, Lectures::default());
    // Écart résiduel de l'ordre de 2e-6 (pertes du ballon, températures de zones).
    assert!((texte - base).abs() < 1e-5 * base, "{texte} {base}");
    let deduit = gaz_ecs_selon(
        &p,
        Lectures {
            preechauffage_ecs: LecturePreechauffageEcs::Deduit,
            ..Lectures::default()
        },
    );
    assert!(deduit < 0.97 * base, "{deduit} {base}");
    println!("position 1 : texte {texte:.0} Wh, variante {deduit:.0} Wh, sans {base:.0} Wh");
}

#[test]
fn les_positions_0_et_2_reduisent_la_consommation() {
    let base = gaz_ecs(&calcul(&sans_recuperateur()));
    let p0 = gaz_ecs(&calcul(&avec(recuperateur(
        PositionRecuperateur::AmontMitigeursEtBallon,
        0.5,
    ))));
    let p2 = gaz_ecs(&calcul(&avec(recuperateur(
        PositionRecuperateur::AmontMitigeurs,
        0.5,
    ))));
    assert!(p0 < p2 && p2 < base, "{p0} {p2} {base}");
}

/// Un récupérateur collectif relié à un seul émetteur, conduit intergroupe presque nul, donne la
/// température d'entrée de l'échangeur individuel en position 1 (mêmes débits, mêmes éq.).
#[test]
fn collectif_equivalent_a_l_individuel_en_position_1() {
    let indiv = calcul(&avec(recuperateur(PositionRecuperateur::AmontBallon, 0.5)));
    let mut p = sans_recuperateur();
    p.batiments[0].recuperateurs_eaux_grises = vec![RecuperateurCollectifEauxGrises {
        id: "central".into(),
        echangeur: echangeur(0.5),
        l_evac_vc_m: 0.001,
        l_evac_vnc_m: 0.0,
    }];
    let mut r = recuperateur(PositionRecuperateur::AmontBallon, 0.5);
    r.configuration = ConfigurationRecuperateur::Collective;
    r.position = None;
    r.recuperateur_collectif = Some("central".into());
    r.unites[0].echangeur = None;
    p.batiments[0].zones[0].groupes[0].emetteurs_ecs[0].recuperateur = Some(r);
    assert!(valider(&p).est_valide(), "{:?}", valider(&p));
    let coll = calcul(&p);
    let t = |r: &Resultats| {
        r.batiments[0].distributions_ecs_intergroupes[0]
            .theta_entrant_moyen_c
            .unwrap()
    };
    assert!(
        (t(&coll) - t(&indiv)).abs() < 1e-2,
        "{} {}",
        t(&coll),
        t(&indiv)
    );
    assert_eq!(
        coll.batiments[0].recuperateurs_eaux_grises[0].epsilon_ech_eg,
        0.5
    );
}

/// Pompe de relevage (éq. 2220 à 2224) : consommation d'électricité ajoutée aux auxiliaires,
/// donc au Cep.
#[test]
fn pompe_de_relevage_ajoutee_au_cep() {
    let sans = calcul(&avec(recuperateur(
        PositionRecuperateur::AmontMitigeursEtBallon,
        0.5,
    )));
    let mut r = recuperateur(PositionRecuperateur::AmontMitigeursEtBallon, 0.5);
    r.unites[0].p_relevage_nominale_w = Some(100.0);
    let avec_pompe = calcul(&avec(r));
    let w = emetteur(&avec_pompe)
        .recuperateur
        .as_ref()
        .unwrap()
        .w_relevage_kwh;
    assert!(w > 0.0);
    assert_eq!(
        emetteur(&sans)
            .recuperateur
            .as_ref()
            .unwrap()
            .w_relevage_kwh,
        0.0
    );
    let b = |r: &Resultats| r.batiments[0].cep_kwhep_m2.unwrap();
    // 2,3 kWhep par kWh d'électricité, rapportés à 96,4 m².
    let attendu = w * 2.3 / 96.4;
    assert!(((b(&avec_pompe) - b(&sans)) - attendu).abs() < 0.01 * attendu + 1e-6);
}

#[test]
fn refus_de_la_validation() {
    let rapport = |f: &dyn Fn(&mut RecuperateurEauxGrises)| {
        let mut r = recuperateur(PositionRecuperateur::AmontBallon, 0.5);
        f(&mut r);
        valider(&avec(r))
    };
    assert!(!rapport(&|r| r.position = None).est_valide());
    assert!(!rapport(&|r| r.unites.clear()).est_valide());
    assert!(!rapport(&|r| {
        r.configuration = ConfigurationRecuperateur::Collective;
        r.position = None;
        r.recuperateur_collectif = Some("inconnu".into());
    })
    .est_valide());
    assert!(!rapport(&|r| {
        r.l_evac_vc_m = 0.0;
        r.l_evac_vnc_m = 0.0;
    })
    .est_valide());
    assert!(
        !rapport(&|r| r.unites[0].echangeur.as_mut().unwrap().efficacite = Some(1.5)).est_valide()
    );
    // « Irecup » = 1 sans description.
    let mut p = sans_recuperateur();
    p.batiments[0].zones[0].groupes[0].emetteurs_ecs[0].recuperateur_eaux_grises = Some(true);
    assert!(!valider(&p).est_valide());
    // Valeur par défaut signalée quand l'efficacité n'est pas renseignée.
    let mut r = recuperateur(PositionRecuperateur::AmontBallon, 0.5);
    r.unites[0].echangeur = None;
    let res = calcul(&avec(r));
    assert!(emetteur(&res)
        .valeurs_par_defaut
        .iter()
        .any(|v| v.champ == "recuperateur.unites[0].echangeur" && (v.valeur - 0.12).abs() < 1e-12));
}

/// Valeurs phares sur la météo réglementaire (nombres annuels seulement).
#[test]
#[ignore = "demande THBCE_METEO"]
fn recuperateur_sur_meteo_reglementaire() {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO");
    let calc = |p: &Projet| {
        let zone = p.localisation.zone_climatique_retenue();
        let m = meteo::lire(&std::fs::read(&chemin).unwrap(), zone).unwrap();
        calculer(p, &m, &Options { arrondir: false }).unwrap()
    };
    let base = calc(&sans_recuperateur());
    for (nom, pos) in [
        ("position 0", PositionRecuperateur::AmontMitigeursEtBallon),
        ("position 1", PositionRecuperateur::AmontBallon),
        ("position 2", PositionRecuperateur::AmontMitigeurs),
    ] {
        let r = calc(&avec(recuperateur(pos, 0.5)));
        println!(
            "{nom} : Qw {:.1} kWh (sans {:.1}), gaz ECS {:.1} kWh (sans {:.1}), Cep {:.2} (sans {:.2})",
            emetteur(&r).q_w_kwh,
            emetteur(&base).q_w_kwh,
            gaz_ecs(&r),
            gaz_ecs(&base),
            r.batiments[0].cep_kwhep_m2.unwrap(),
            base.batiments[0].cep_kwhep_m2.unwrap()
        );
        assert!(gaz_ecs(&r) <= gaz_ecs(&base) + 1e-6);
    }
}
