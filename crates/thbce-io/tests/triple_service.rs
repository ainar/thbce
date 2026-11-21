// SPDX-License-Identifier: GPL-3.0-or-later
//! Pompes à chaleur électriques triple service (chauffage, refroidissement et ECS : fiche 8.23,
//! tableau 149, éq. 1293, 1294 et 1326 ; lot 2g, lecture Q250), de bout en bout sur l'exemple
//! `maison-pac-triple-service-ademe`, sur la météo ouverte ADEME (CI publique) et sur la météo
//! réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test triple_service -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les tests vérifient des propriétés (bilans, sens
//! des effets) ; les calculs à la main sont dans `chauffage_refroidissement/generation/tests.rs`.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::Projet;
use thbce_core::resultats::{Resultats, ResultatsGenerateur};
use thbce_core::{calculer, Options};
use thbce_io::{json, meteo};

const TRIPLE: &str = "maison-pac-triple-service-ademe";
const REVERSIBLE: &str = "maison-pac-air-air-ademe";

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json_du_cas(nom: &str) -> serde_json::Value {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    serde_json::from_str(&std::fs::read_to_string(chemin).unwrap()).unwrap()
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

fn calcul(v: &serde_json::Value, meteo_de: &dyn Fn(&Projet) -> MeteoProjet) -> Resultats {
    let p = json::lire_projet(&v.to_string()).unwrap();
    let r = thbce_core::valider(&p);
    assert!(r.est_valide(), "{:?}", r.erreurs);
    calculer(&p, &meteo_de(&p), &Options { arrondir: false }).unwrap()
}

fn pac(r: &Resultats) -> &ResultatsGenerateur {
    &r.batiments[0].generations[0].generateurs[0]
}

/// La PAC assure les trois postes ; l'ECS du ballon lui est entièrement demandée ; le chauffage
/// et le refroidissement fournis sont ceux de la PAC réversible de l'exemple d'origine, à la
/// demande près (les pertes du ballon changent les apports internes).
#[test]
fn trois_postes_assures() {
    let t = calcul(&json_du_cas(TRIPLE), &meteo_ademe);
    let r = calcul(&json_du_cas(REVERSIBLE), &meteo_ademe);
    let (pt, pr) = (pac(&t), pac(&r));
    assert!(pt.q_fou_ecs_kwh > 1000.0, "{}", pt.q_fou_ecs_kwh);
    assert!(pt.consommations.ecs.electricite_kwh > 0.0);
    assert!((pt.q_fou_ch_kwh - pr.q_fou_ch_kwh).abs() < 0.02 * pr.q_fou_ch_kwh);
    assert!((pt.q_fou_fr_kwh - pr.q_fou_fr_kwh).abs() < 0.02 * pr.q_fou_fr_kwh);
    // Le temps pris par l'ECS (et le basculement vers le froid, éq. 1294) réduit la puissance
    // disponible : à énergie fournie égale, la consommation de froid augmente (lecture Q47,
    // point 4).
    let fr = |g: &ResultatsGenerateur| g.consommations.refroidissement.electricite_kwh;
    assert!(fr(pt) > fr(pr), "{} {}", fr(pt), fr(pr));
}

/// Valeurs phares sur la météo réglementaire (nombres annuels seulement).
#[test]
#[ignore = "météo réglementaire (THBCE_METEO)"]
fn triple_service_sur_meteo_reglementaire() {
    let t = calcul(&json_du_cas(TRIPLE), &meteo_reglementaire);
    let b = &t.batiments[0];
    let p = pac(&t);
    println!(
        "{TRIPLE} : Bbio {:.1}, DH {:.1}, Cep {:.1}, Cep,nr {:.1}, Ic énergie {:.1} ; \
         PAC : ch {:.1} kWh, fr {:.1} kWh, ECS {:.1} kWh fournis",
        b.bbio_pts.unwrap(),
        b.dh_c_h.unwrap(),
        b.cep_kwhep_m2.unwrap(),
        b.cep_nr_kwhep_m2.unwrap(),
        b.ic_energie_kgco2_m2.unwrap(),
        p.q_fou_ch_kwh,
        p.q_fou_fr_kwh,
        p.q_fou_ecs_kwh
    );
    assert!(p.q_fou_ecs_kwh > 1000.0);
}

/// Tableau 149 : les quatre combinaisons du triple service sont admises (ni refus ni erreur de
/// combinaison), avec la source amont que fixe leur technologie (lecture Q145) ; les réseaux
/// fictifs de l'exemple ne conviennent qu'aux machines sur air (contrôle du fluide aval, hors
/// sujet ici).
#[test]
fn quatre_combinaisons_du_tableau_149() {
    let captage = json_du_cas("maison-pac-geothermie-ademe")["batiments"][0]["generations"][0]
        ["generateurs"][0]["nature"]["thermodynamique_electrique"]["source_amont"]
        .clone();
    let nappe = json_du_cas("maison-pac-nappe-reversible-ademe")["batiments"][0]["generations"][0]
        ["generateurs"][0]["nature"]["thermodynamique_electrique"]["source_amont"]
        .clone();
    let air = serde_json::json!({"air_exterieur": {}});
    for (ch, ecs, fr, source) in [
        (
            "air_exterieur_eau",
            "air_exterieur_eau",
            "air_exterieur_eau",
            &air,
        ),
        (
            "eau_de_nappe_eau",
            "eau_de_nappe_eau",
            "eau_de_nappe_eau",
            &nappe,
        ),
        ("eau_glycolee_eau", "eau_glycolee_eau", "eau_eau", &captage),
        (
            "air_exterieur_air_recycle",
            "air_exterieur_eau",
            "air_exterieur_air_recycle",
            &air,
        ),
    ] {
        let mut v = json_du_cas(TRIPLE);
        let t = &mut v["batiments"][0]["generations"][0]["generateurs"][0]["nature"]
            ["thermodynamique_electrique"];
        t["chauffage"]["technologie"] = ch.into();
        t["ecs"]["technologie"] = ecs.into();
        t["refroidissement"]["technologie"] = fr.into();
        t["source_amont"] = source.clone();
        let r = thbce_core::valider(&json::lire_projet(&v.to_string()).unwrap());
        assert!(
            r.erreurs.iter().all(|e| !e.hors_perimetre
                && !e.message.contains("tableau 14")
                && !e.chemin.contains("source_amont")),
            "{ch} / {ecs} / {fr} : {:?}",
            r.erreurs
        );
        // La quatrième combinaison est celle de l'exemple : entrée entièrement valide.
        if ch == "air_exterieur_air_recycle" {
            assert!(r.est_valide(), "{:?}", r.erreurs);
        }
    }
}
