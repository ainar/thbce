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

/// La maison d'exemple équipée d'une pompe à chaleur air/air réversible : unités intérieures
/// soufflant l'air (Pem 0,95 en chaud et en froid, classes B2 et B), régulation par défaut avec
/// arrêt total, ventilateurs à régulation automatique avec arrêt (30, 20 et 10 W ; 600, 400 et
/// 200 m³/h), batterie à température constante ; réseaux fictifs (détente directe).
fn climatisee() -> Projet {
    let mut p = cas("maison-climat-ademe");
    let b = &mut p.batiments[0];
    b.distributions_intergroupes = serde_json::from_str(
        r#"[{"id": "intergroupes-chauffage", "fonction": "chauffage",
             "generation": "pac-air-air", "reseau": "fictif"},
            {"id": "intergroupes-froid", "fonction": "refroidissement",
             "generation": "pac-air-air", "reseau": "fictif"}]"#,
    )
    .unwrap();
    b.generations = serde_json::from_str(
        r#"[{"id": "pac-air-air", "raccordement_reseaux": "avec_isolement"}]"#,
    )
    .unwrap();
    let g = &mut b.zones[0].groupes[0];
    g.iclim = Some(true);
    g.programmation = Some(
        serde_json::from_str(r#"{"chauffage": "horloge", "refroidissement": "horloge"}"#).unwrap(),
    );
    g.distributions = serde_json::from_str(
        r#"[{"id": "d-ch", "fonction": "chauffage",
             "distribution_intergroupes": "intergroupes-chauffage", "reseau": "fictif"},
            {"id": "d-fr", "fonction": "refroidissement",
             "distribution_intergroupes": "intergroupes-froid", "reseau": "fictif"}]"#,
    )
    .unwrap();
    g.emetteurs = serde_json::from_str(
        r#"[{"id": "unites-interieures",
             "chauffage": {"type_emetteur": "soufflage_air_chaud",
               "variation_spatiale": {"classe": {"classe": "B2", "hsp_m": 2.5}},
               "variation_temporelle": {"defaut": {"arret_total": true}},
               "effet_joule": false, "rats": 1, "ratt": 1, "distribution": "d-ch"},
             "refroidissement": {"type_emetteur": "soufflage_air_froid",
               "variation_spatiale": {"classe": {"classe": "B", "hsp_m": 2.5}},
               "variation_temporelle": {"defaut": {"arret_total": true}},
               "rats": 1, "ratt": 1, "distribution": "d-fr"},
             "ventilateurs": {"gestion": "automatique_avec_arret",
               "p_gv_w": 30, "p_mv_w": 20, "p_pv_w": 10,
               "q_recirc_gv_m3_h": 600, "q_recirc_mv_m3_h": 400, "q_recirc_pv_m3_h": 200,
               "regulation_batterie": "temperature_constante"}}]"#,
    )
    .unwrap();
    p
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
    let loc = &projet.localisation;
    let climat = ClimatExterieur::nouveau(&meteo.th_d, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    let mut b = BatimentHoraire::nouveau_selon(
        &projet.batiments[0],
        &projet.catalogue,
        ModeCalcul::ThD,
        Lectures {
            emission_th_d: EmissionThD::ConventionnelleBbio,
            ..Lectures::default()
        },
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
    // Générations idéales : la demande de la génération est celle des émetteurs, les réseaux
    // fictifs n'ayant ni pertes ni auxiliaires (éq. 872, 873, 919 et 920).
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
    let dh_conv = dh_emission_conventionnelle(&maison, &m);
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
    let veranda = cas("maison-veranda-ademe");
    let rv = brut(&veranda, &m);
    let dh_v = rv.batiments[0].dh_c_h.unwrap();
    let dh_v_conv = dh_emission_conventionnelle(&veranda, &m);
    // L'émission du projet ne chauffe pas en période de confort adaptatif : effet négligeable.
    assert!((dh_v - dh_v_conv).abs() < 1.0);
    println!(
        "{titre} : variante véranda, Bbio = {:.2} points, DH = {dh_v:.1} °C.h (émission \
         conventionnelle : {dh_v_conv:.1}), chauffage Th-C {:.1} kWh",
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
    // Sans émetteur de chauffage.
    let mut p = cas("maison-climat-ademe");
    p.batiments[0].zones[0].groupes[0].emetteurs.clear();
    assert!(messages(&p)
        .iter()
        .any(|m| m.contains("émetteur de chauffage")));
}
