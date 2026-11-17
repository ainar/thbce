// SPDX-License-Identifier: GPL-3.0-or-later
//! Systèmes de chauffage et de refroidissement du projet (lot 6b) de bout en bout sur la
//! maison d'exemple (`tests/cas/maison-climat-ademe`, panneaux rayonnants électriques) et sur
//! une variante climatisée (pompe à chaleur air/air réversible, unités intérieures à
//! ventilateurs), sur la météo ouverte ADEME (CI publique) et sur la météo réglementaire
//! (tests ignorés par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test systemes -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les tests vérifient des propriétés (bilans
//! fermés, Bbio inchangé, demandes des générations égales à celles des émetteurs, froid et
//! relances) et non des valeurs.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::bati::assemblage_batiment::BatimentHoraire;
use thbce_core::bati::assemblage_groupe::{EmissionThD, Lectures};
use thbce_core::climat::{ClimatExterieur, ConfortAdaptatif, MeteoProjet};
use thbce_core::conventions::ModeCalcul;
use thbce_core::entree::Projet;
use thbce_core::resultats::Resultats;
use thbce_core::{calculer, valider, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cas(nom: &str) -> Projet {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn meteo_ademe(projet: &Projet) -> MeteoProjet {
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    let zone = projet.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

fn meteo_reglementaire(projet: &Projet) -> MeteoProjet {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO pour lancer ce test");
    let zone = projet.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

fn brut(projet: &Projet, meteo: &MeteoProjet) -> Resultats {
    calculer(projet, meteo, &Options { arrondir: false }).unwrap()
}

/// La maison d'exemple équipée d'une pompe à chaleur air/air réversible
/// (`tests/cas/maison-pac-air-air-ademe`) : unités intérieures soufflant l'air (Pem 0,95 en chaud
/// et en froid, classes B2 et B), régulation par défaut avec arrêt total, ventilateurs à
/// régulation automatique avec arrêt (30, 20 et 10 W ; 600, 400 et 200 m³/h), batterie à
/// température constante ; réseaux fictifs (détente directe).
fn climatisee() -> Projet {
    cas("maison-pac-air-air-ademe")
}

/// Bilan d'énergie du modèle thermique de chaque groupe en Th-C, en Wh.
fn residu_th_c(projet: &Projet, meteo: &MeteoProjet) -> f64 {
    let loc = &projet.localisation;
    let climat = ClimatExterieur::nouveau(&meteo.th_bc, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    let mut b = BatimentHoraire::nouveau(
        &projet.batiments[0],
        &projet.catalogue,
        ModeCalcul::ThC,
        loc.alt_m,
        climat.we_kg_kg[0],
    )
    .unwrap();
    b.simuler(&climat, &confort).unwrap();
    let bilan = b.zones[0].groupes[0].cumuls.bilan;
    assert!(bilan.chauffage_wh > 0.0);
    bilan.residu_wh() / bilan.entrees_wh()
}

fn dh_emission_conventionnelle(projet: &Projet, meteo: &MeteoProjet) -> f64 {
    dh_selon(
        projet,
        meteo,
        Lectures {
            emission_th_d: EmissionThD::ConventionnelleBbio,
            ..Lectures::default()
        },
    )
}

fn dh_sans_recuperation(projet: &Projet, meteo: &MeteoProjet) -> f64 {
    dh_selon(
        projet,
        meteo,
        Lectures {
            recuperation_th_d: false,
            ..Lectures::default()
        },
    )
}

fn dh_selon(projet: &Projet, meteo: &MeteoProjet, lectures: Lectures) -> f64 {
    let loc = &projet.localisation;
    let climat = ClimatExterieur::nouveau(&meteo.th_d, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    let mut b = BatimentHoraire::nouveau_selon(
        &projet.batiments[0],
        &projet.catalogue,
        ModeCalcul::ThD,
        lectures,
        loc.alt_m,
        climat.we_kg_kg[0],
    )
    .unwrap();
    b.simuler(&climat, &confort).unwrap();
    b.zones[0].groupes[0].cumuls.confort_ete.dh_c_h
}

fn verifier(meteo_de: impl Fn(&Projet) -> MeteoProjet, titre: &str) {
    let maison = cas("maison-climat-ademe");
    let clim = climatisee();
    let m = meteo_de(&maison);
    let rm = brut(&maison, &m);
    let rc = brut(&clim, &m);
    let b = &rm.batiments[0];
    let sy = b.zones[0].groupes[0].systemes.as_ref().unwrap();
    let syc = rc.batiments[0].zones[0].groupes[0]
        .systemes
        .as_ref()
        .unwrap();

    // Bbio indépendant des systèmes (Th-B, émission conventionnelle).
    assert_eq!(b.bbio_pts, rc.batiments[0].bbio_pts);

    // La demande de la génération est celle des émetteurs, les réseaux fictifs n'ayant ni
    // pertes ni auxiliaires (éq. 872, 873, 919 et 920).
    assert!((b.generations[0].q_ch_kwh - sy.q_sys_ch_kwh).abs() < 1e-9);
    assert!((b.distributions_intergroupes[0].q_ch_kwh - sy.emetteurs[0].q_ch_kwh).abs() < 1e-9);
    // Consignes du projet : δθvs + δθvt = 0,2 + 1,8 K au-dessus de la consigne et relances :
    // plus de chauffage qu'en Bbio.
    let besoin_bbio_kwh = b.bbio.as_ref().unwrap().annuel.b_ch_kwh_m2 * b.sref_m2;
    assert!(sy.q_sys_ch_kwh > besoin_bbio_kwh);
    assert!(sy.nbh_relance > 0);
    assert_eq!(sy.q_sys_fr_kwh, 0.0);
    // Variante climatisée : froid, ventilateurs locaux, générations communes.
    assert!(syc.q_sys_fr_kwh > 0.0);
    assert!(syc.w_vent_loc_kwh > 0.0);
    assert!((rc.batiments[0].generations[0].q_fr_kwh - syc.q_sys_fr_kwh).abs() < 1e-9);
    // Bilans d'énergie du modèle thermique fermés en Th-C.
    for p in [&maison, &clim] {
        let r = residu_th_c(p, &m);
        assert!(r.abs() < 1e-6, "{titre} : résidu relatif {r}");
    }
    // ECS (§9.6) : 392 L à 40 °C par adulte équivalent et par semaine (éq. 1686), 1,95 adulte
    // équivalent pour 96,4 m² (éq. 1685), corrigés de 0,97 par les mitigeurs (éq. 1678) : 741 L
    // par semaine, réchauffés d'environ 27 K depuis l'eau froide, soit de l'ordre de
    // 1,163 × 741 × 52 × 27 ≈ 1 200 kWh par an.
    let e = &sy.ecs[0];
    assert!((900.0..1600.0).contains(&e.q_w_kwh), "{titre} : {e:?}");
    assert!(e.q_w_kwh < e.q_w_bruts_kwh);
    assert!((e.q_w_2nd_kwh - e.q_w_kwh - e.pertes_vc_kwh - e.pertes_hvc_kwh).abs() < 1e-6);
    assert!((b.generations[1].q_ecs_kwh - e.q_w_2nd_kwh).abs() < 1e-6);
    // Pertes récupérées (§11.1) : 60 % des pertes en volume chauffé de la distribution d'ECS ;
    // 60 % des pertes du ballon d'ECS, placé en volume chauffé (éq. 1961 et 2362) ; les
    // générateurs à effet Joule et les réseaux fictifs n'en ont pas.
    let pertes_ballon = b.generations[1].pertes_vc_kwh;
    assert!(pertes_ballon > 0.0);
    assert!((sy.recup_kwh - 0.6 * (e.pertes_vc_kwh + pertes_ballon)).abs() < 1e-6);
    let dh_conv = dh_emission_conventionnelle(&maison, &m);
    let dh_sans_recup = dh_sans_recuperation(&maison, &m);
    println!(
        "{titre} : ECS {:.1} kWh (bruts {:.1}), distribution {:.1} kWh dont {:.1} kWh de pertes en \
         volume chauffé, {:.1} kWh récupérés ; DH sans récupération en Th-D : {dh_sans_recup:.1} \
         °C.h",
        e.q_w_kwh,
        e.q_w_bruts_kwh,
        e.q_w_2nd_kwh,
        e.pertes_vc_kwh,
        sy.recup_kwh,
    );
    println!(
        "{titre} : Bbio = {:.2} points, DH = {:.1} °C.h (émission conventionnelle : {:.1}) ; \
         Th-C : chauffage {:.1} kWh ({:.1} kWh en Bbio), {} h de relance ; climatisée : \
         chauffage {:.1} kWh, froid {:.1} kWh, ventilateurs {:.1} kWh, DH = {:.1} °C.h",
        b.bbio_pts.unwrap(),
        b.dh_c_h.unwrap(),
        dh_conv,
        sy.q_sys_ch_kwh,
        besoin_bbio_kwh,
        sy.nbh_relance,
        syc.q_sys_ch_kwh,
        syc.q_sys_fr_kwh,
        syc.w_vent_loc_kwh,
        rc.batiments[0].dh_c_h.unwrap(),
    );
    // Variante à plancher chauffant à eau : la génération reçoit la demande des émetteurs
    // augmentée des pertes du réseau du groupe (éq. 880) ; réseau intergroupes individuel sans
    // pertes (éq. 905).
    let plancher = cas("maison-plancher-ademe");
    let rp = brut(&plancher, &m);
    let bp = &rp.batiments[0];
    let syp = bp.zones[0].groupes[0].systemes.as_ref().unwrap();
    let d = &syp.distributions[0];
    assert_eq!(bp.bbio_pts, b.bbio_pts);
    assert!(d.pertes_vc_kwh > 0.0 && d.w_aux_kwh > 0.0);
    assert!((bp.generations[0].q_ch_kwh - syp.q_sys_ch_kwh - d.pertes_vc_kwh).abs() < 1e-6);
    assert_eq!(bp.distributions_intergroupes[0].pertes_vc_kwh, 0.0);
    // Récupération : pertes du réseau du groupe (100 %), 60 % de la part du circulateur
    // transmise au volume chauffé (50 %), 60 % des pertes de l'ECS en volume chauffé et de
    // celles du ballon du chauffe-eau thermodynamique.
    let ep = &syp.ecs[0];
    let attendu = d.pertes_vc_kwh
        + 0.6 * 0.5 * d.w_aux_kwh
        + 0.6 * (ep.pertes_vc_kwh + bp.generations[1].pertes_vc_kwh);
    assert!((syp.recup_kwh - attendu).abs() < 1e-6, "{titre}");
    println!(
        "{titre} : plancher chauffant, demande {:.1} kWh, émetteurs {:.1} kWh, pertes en volume \
         chauffé {:.1} kWh, circulateur {:.1} kWh, DH = {:.1} °C.h",
        bp.generations[0].q_ch_kwh,
        syp.q_sys_ch_kwh,
        d.pertes_vc_kwh,
        d.w_aux_kwh,
        bp.dh_c_h.unwrap(),
    );
    let veranda = cas("maison-veranda-ademe");
    let rv = brut(&veranda, &m);
    let dh_v = rv.batiments[0].dh_c_h.unwrap();
    let dh_v_conv = dh_emission_conventionnelle(&veranda, &m);
    // Écart dû aux seules pertes récupérées de l'ECS en été (l'émission ne chauffe pas en
    // période de confort adaptatif).
    assert!((dh_v - dh_v_conv).abs() < 1.0);
    let dh_v_sans = dh_sans_recuperation(&veranda, &m);
    assert!(dh_v > dh_v_sans);
    println!(
        "{titre} : variante véranda, Bbio = {:.2} points, DH = {dh_v:.1} °C.h (émission \
         conventionnelle : {dh_v_conv:.1} ; sans récupération : \
         {dh_v_sans:.1}), chauffage Th-C {:.1} kWh",
        rv.batiments[0].bbio_pts.unwrap(),
        rv.batiments[0].generations[0].q_ch_kwh,
    );
}

#[test]
fn systemes_du_projet_sur_la_meteo_ademe() {
    verifier(meteo_ademe, "ADEME");
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn systemes_du_projet_sur_la_meteo_reglementaire() {
    verifier(meteo_reglementaire, "réglementaire");
}

#[test]
fn validation_des_systemes() {
    let messages = |p: &Projet| -> Vec<String> {
        valider(p)
            .erreurs
            .iter()
            .map(|e| format!("{} : {}", e.chemin, e.message))
            .collect()
    };
    assert!(messages(&cas("maison-climat-ademe")).is_empty());
    assert!(messages(&climatisee()).is_empty());
    // Sans mode refroidissement, la génération n'assure plus le poste de son réseau de froid
    // (§8.17.3.1).
    let mut p = climatisee();
    let v = serde_json::to_value(&p.batiments[0].generations[0]).unwrap();
    let mut v2 = v;
    v2["generateurs"][0]["nature"]["thermodynamique_electrique"]
        .as_object_mut()
        .unwrap()
        .remove("refroidissement");
    p.batiments[0].generations[0] = serde_json::from_value(v2).unwrap();
    assert!(messages(&p)
        .iter()
        .any(|m| m.contains("n'assure le poste refroidissement")));
    // iclim absent, ou contraire aux émetteurs.
    let mut p = cas("maison-climat-ademe");
    p.batiments[0].zones[0].groupes[0].iclim = None;
    assert!(messages(&p).iter().any(|m| m.contains("iclim")));
    let mut p = cas("maison-climat-ademe");
    p.batiments[0].zones[0].groupes[0].iclim = Some(true);
    assert!(messages(&p)
        .iter()
        .any(|m| m.contains("si et seulement si")));
    // Référence inconnue et somme des Rat supérieure à 1.
    let mut p = climatisee();
    let g = &mut p.batiments[0].zones[0].groupes[0];
    g.emetteurs[0].chauffage.as_mut().unwrap().distribution = "inconnu".into();
    g.emetteurs.push(g.emetteurs[0].clone());
    g.emetteurs[1].id = "second".into();
    let m = messages(&p);
    assert!(m
        .iter()
        .any(|m| m.contains("réseau de distribution inconnu")));
    assert!(m.iter().any(|m| m.contains("ne peut dépasser 1")));
    // Générateurs thermodynamiques (§8.23) : technologie hors de la version, incohérence avec
    // les réseaux, ordre de priorité des valeurs mesurées, charge partielle incohérente.
    let avec = |f: &dyn Fn(&mut serde_json::Value)| -> Vec<String> {
        let mut p = cas("maison-plancher-ademe");
        let mut v = serde_json::to_value(&p.batiments[0].generations[0]).unwrap();
        f(&mut v["generateurs"][0]["nature"]["thermodynamique_electrique"]);
        p.batiments[0].generations[0] = serde_json::from_value(v).unwrap();
        messages(&p)
    };
    assert!(avec(&|_| {}).is_empty());
    let m = avec(&|pac| pac["chauffage"]["technologie"] = "eau_glycolee_eau".into());
    assert!(m.iter().any(|m| m.contains("non pris en charge")), "{m:?}");
    let m = avec(&|pac| pac["chauffage"]["technologie"] = "air_exterieur_air_recycle".into());
    assert!(m.iter().any(|m| m.contains("réseaux fictifs")), "{m:?}");
    let m = avec(&|pac| {
        // Sans la valeur pivot (7 °C et 32,5 °C).
        pac["chauffage"]["performances"]["mesurees"]
            .as_array_mut()
            .unwrap()
            .remove(0);
    });
    assert!(
        m.iter().any(|m| m.contains("requises pour le calcul")),
        "{m:?}"
    );
    let m = avec(&|pac| {
        pac["chauffage"]["fonctionnement_continu"] = serde_json::json!(
            {"lr_contmin": 0.05, "ccp_lr_contmin": 1.0, "statut": "certifie"}
        );
    });
    assert!(m.iter().any(|m| m.contains("limites tolérées")), "{m:?}");
    let m = avec(&|pac| {
        pac["source_amont"] = serde_json::json!({"air_extrait": {"zone": "x", "t_air_lim_c": 5}});
    });
    assert!(m.iter().any(|m| m.contains("zone inconnue")), "{m:?}");
    assert!(
        m.iter().any(|m| m.contains("source amont incohérente")),
        "{m:?}"
    );
    // Ballons de stockage et chauffe-eau thermodynamiques (§9.9, §9.14 et §9.22).
    let ballon = |f: &dyn Fn(&mut serde_json::Value)| -> Vec<String> {
        let mut p = cas("maison-climat-ademe");
        let mut v = serde_json::to_value(&p.batiments[0].generations[1]).unwrap();
        f(&mut v);
        p.batiments[0].generations[1] = serde_json::from_value(v).unwrap();
        messages(&p)
    };
    assert!(ballon(&|_| {}).is_empty());
    let m = ballon(&|g| g["stockages"][0]["base"]["generateur"] = "inconnu".into());
    assert!(m.iter().any(|m| m.contains("générateur inconnu")), "{m:?}");
    let m = ballon(&|g| g["stockages"][0]["base"]["z_reg"] = 5.into());
    assert!(m.iter().any(|m| m.contains("numéro de zone")), "{m:?}");
    let m = ballon(&|g| g["stockages"][0]["ballon"]["theta_max_c"] = 50.into());
    assert!(m.iter().any(|m| m.contains("theta_max_c")), "{m:?}");
    let m = ballon(&|g| g["priorite"] = "sans_priorite".into());
    assert!(m.iter().any(|m| m.contains("régulée en cascade")), "{m:?}");
    let m = ballon(&|g| g["stockages"][0]["nb_assemblages"] = 2.into());
    assert!(m.iter().any(|m| m.contains("rdim")), "{m:?}");
    let cet = |f: &dyn Fn(&mut serde_json::Value)| -> Vec<String> {
        let mut p = cas("maison-plancher-ademe");
        let mut v = serde_json::to_value(&p.batiments[0].generations[1]).unwrap();
        f(&mut v["chauffe_eau_thermodynamiques"][0]);
        p.batiments[0].generations[1] = serde_json::from_value(v).unwrap();
        messages(&p)
    };
    assert!(cet(&|_| {}).is_empty());
    // Essai incohérent : 0,5 h pour chauffer 200 L avec un COP de 2,7 et 5 W en régime
    // stabilisé.
    let m = cet(&|c| {
        c["essai"]["t_h_h"] = 0.05.into();
        c["essai"]["p_es_w"] = 0.01.into();
    });
    assert!(m.iter().any(|m| m.contains("aucun triplet")), "{m:?}");
    let m =
        cet(&|c| c["source_amont"] = serde_json::json!({"air_ambiant": {"espace_tampon": "x"}}));
    assert!(
        m.iter().any(|m| m.contains("espace tampon inconnu")),
        "{m:?}"
    );
    // Générateurs : au moins un, natures hors de la version, sécurité antigel.
    let mut p = cas("maison-climat-ademe");
    p.batiments[0].generations[0].generateurs.clear();
    assert!(messages(&p)
        .iter()
        .any(|m| m.contains("au moins un générateur")));
    let mut p = cas("maison-climat-ademe");
    p.batiments[0].generations[0].generateurs[0].nature =
        serde_json::from_str(r#""autre_generateur_a_combustion""#).unwrap();
    let m = messages(&p);
    assert!(m.iter().any(|m| m.contains("§8.20")), "{m:?}");
    let mut p = cas("maison-climat-ademe");
    let g = &mut p.batiments[0].generations[0];
    g.generateurs.push(g.generateurs[0].clone());
    g.generateurs[1].id = "appoint".into();
    assert!(messages(&p).iter().any(|m| m.contains("idtype_priorite")));
    p.batiments[0].generations[0].priorite = serde_json::from_str(r#""cascade""#).unwrap();
    assert!(messages(&p)
        .iter()
        .any(|m| m.contains("rang de priorité requis")));
    p.batiments[0].generations[0].priorite = serde_json::from_str(r#""alternance""#).unwrap();
    assert!(messages(&p).iter().any(|m| m.contains("alternance")));
    let mut p = cas("maison-double-flux-ademe");
    let v = serde_json::to_value(&p.batiments[0].zones[0].ventilation).unwrap();
    let mut v2 = v.clone();
    v2["systeme"]["double_flux"]["echangeur"]["antigel"] = true.into();
    p.batiments[0].zones[0].ventilation = serde_json::from_value(v2).unwrap();
    assert!(messages(&p)
        .iter()
        .any(|m| m.contains("génération associée à la sécurité antigel requise")));
    // Sans émetteur de chauffage.
    let mut p = cas("maison-climat-ademe");
    p.batiments[0].zones[0].groupes[0].emetteurs.clear();
    assert!(messages(&p)
        .iter()
        .any(|m| m.contains("émetteur de chauffage")));
}
