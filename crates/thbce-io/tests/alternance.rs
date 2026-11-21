// SPDX-License-Identifier: GPL-3.0-or-later
//! Gestion des générateurs en alternance (Th-BCE 2020 §8.17.3.7.2.3, éq. 1035 à 1052 ; lot
//! 2a, lectures Q125) de bout en bout sur l'exemple `tests/cas/maison-chaudieres-alternance-ademe`,
//! sur la météo ouverte ADEME (CI publique) et sur la météo réglementaire (test ignoré) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo.csv cargo test -p thbce-io --test alternance -- --ignored --nocapture
//! ```
//!
//! L'algorithme est vérifié à la main dans les tests unitaires de la gestion
//! (`chauffage_refroidissement::generation::tests`, tests `alternance_*`) ; ici : bilans fermés,
//! indépendance à l'ordre de la saisie, équivalence avec la cascade pour un générateur unique,
//! contrôles de cohérence du montage.

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
use thbce_core::resultats::Resultats;
use thbce_core::{calculer, Options};
use thbce_io::{json, meteo};

const EXEMPLE: &str = "maison-chaudieres-alternance-ademe";

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json_du_cas(nom: &str) -> serde_json::Value {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    serde_json::from_str(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn projet(v: &serde_json::Value) -> Projet {
    json::lire_projet(&v.to_string()).unwrap()
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

fn calcul(p: &Projet, meteo_de: &dyn Fn(&Projet) -> MeteoProjet) -> Resultats {
    let r = thbce_core::valider(p);
    assert!(r.est_valide(), "{:?}", r.erreurs);
    calculer(p, &meteo_de(p), &Options { arrondir: false }).unwrap()
}

/// Énergies fournies `[chauffage, ECS]` de chaque générateur de la génération, en kWh.
fn fournies(r: &Resultats) -> Vec<(String, [f64; 2])> {
    r.batiments[0].generations[0]
        .generateurs
        .iter()
        .map(|g| (g.id.clone(), [g.q_fou_ch_kwh, g.q_fou_ecs_kwh]))
        .collect()
}

fn verifier(meteo_de: &dyn Fn(&Projet) -> MeteoProjet, titre: &str) -> Resultats {
    let v = json_du_cas(EXEMPLE);
    let r = calcul(&projet(&v), meteo_de);
    let g = &r.batiments[0].generations[0];
    // Bilans : la génération consomme ce que consomment ses générateurs ; la demande est
    // fournie ou reportée (éq. 1052).
    let generateurs: f64 = g
        .generateurs
        .iter()
        .map(|x| x.consommations.total_kwh())
        .sum();
    assert!((g.consommations.annuel.total_kwh() - generateurs).abs() < 1e-6);
    let demande = g.q_ch_kwh + g.q_fr_kwh + g.q_ecs_kwh;
    let fourni = g.q_fou_ch_kwh + g.q_fou_fr_kwh + g.q_fou_ecs_kwh;
    assert!((demande - fourni - g.q_rep_fin_kwh).abs() < 1e-6, "{titre}");
    let par_generateur: f64 = g
        .generateurs
        .iter()
        .map(|x| x.q_fou_ch_kwh + x.q_fou_ecs_kwh)
        .sum();
    assert!((par_generateur - fourni).abs() < 1e-6, "{titre}");
    // La chaudière de 24 kW, sollicitée en premier, fournit pendant 19 heures où la demande
    // tient dans les 12 kW de la suivante, puis est coupée à la 20ᵉ : la suivante fournit l'essentiel de l'énergie.
    let f = fournies(&r);
    let petite = f[0].1[0] + f[0].1[1];
    let grande = f[1].1[0] + f[1].1[1];
    println!("{titre} : 12 kW {petite:.1} kWh, 24 kW {grande:.1} kWh");
    assert!(grande > 0.0, "{titre}");
    assert!(petite > 20.0 * grande, "{titre}");

    // L'ordre de la saisie est sans effet : les générateurs sont triés par puissance
    // décroissante (§8.17.3.7.2.3).
    let mut inverse = v.clone();
    inverse["batiments"][0]["generations"][0]["generateurs"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let ri = calcul(&projet(&inverse), meteo_de);
    let mut fi = fournies(&ri);
    fi.reverse();
    assert_eq!(fi, f, "{titre}");
    assert_eq!(
        ri.batiments[0].indicateurs.as_ref().unwrap().cep_kwhep_m2,
        r.batiments[0].indicateurs.as_ref().unwrap().cep_kwhep_m2
    );

    // Un générateur seul en alternance n'est jamais coupé quand il y a une demande (pas de
    // générateur suivant) : mêmes énergies fournies qu'en cascade.
    let seul = |priorite: &str| {
        let mut w = v.clone();
        let gen = &mut w["batiments"][0]["generations"][0];
        gen["priorite"] = priorite.into();
        gen["generateurs"].as_array_mut().unwrap().truncate(1);
        gen["generateurs"][0]["priorite_chauffage"] = 1.into();
        gen["generateurs"][0]["priorite_ecs"] = 1.into();
        calcul(&projet(&w), meteo_de)
    };
    let (a, c) = (seul("alternance"), seul("cascade"));
    let (ga, gc) = (
        &a.batiments[0].generations[0],
        &c.batiments[0].generations[0],
    );
    assert!((ga.q_fou_ch_kwh - gc.q_fou_ch_kwh).abs() < 1e-6, "{titre}");
    assert!(
        (ga.q_fou_ecs_kwh - gc.q_fou_ecs_kwh).abs() < 1e-6,
        "{titre}"
    );
    r
}

#[test]
fn alternance_sur_la_meteo_ademe() {
    verifier(&meteo_ademe, "météo ADEME");
}

#[test]
#[ignore = "météo réglementaire requise : THBCE_METEO"]
fn alternance_sur_meteo_reglementaire() {
    let r = verifier(&meteo_reglementaire, "météo réglementaire");
    let b = &r.batiments[0];
    let i = b.indicateurs.as_ref().unwrap();
    let a_une_decimale = |x: f64| (x * 10.0).round() / 10.0;
    println!(
        "Bbio {:.1}, DH {:.1}, Cep {:.1}, Cep,nr {:.1}, Ic énergie {:.1}",
        b.bbio_pts.unwrap(),
        b.dh_c_h.unwrap(),
        i.cep_kwhep_m2,
        i.cep_nr_kwhep_m2,
        i.ic_energie_kgco2_m2.unwrap()
    );
    // Valeurs phares (lot 2a), garde-fou contre une régression.
    assert_eq!(a_une_decimale(b.bbio_pts.unwrap()), PHARES[0]);
    assert_eq!(a_une_decimale(b.dh_c_h.unwrap()), PHARES[1]);
    assert_eq!(a_une_decimale(i.cep_kwhep_m2), PHARES[2]);
    assert_eq!(a_une_decimale(i.ic_energie_kgco2_m2.unwrap()), PHARES[3]);
}

/// Bbio, DH, Cep et Ic énergie de l'exemple sur la météo réglementaire (H2b).
const PHARES: [f64; 4] = [59.7, 1018.0, 75.4, 509.9];

/// Contrôles de cohérence du montage propres à l'alternance (§8.17.3.7.2.3 et §8.17.3.6).
#[test]
fn coherence_du_montage_en_alternance() {
    let messages = |v: &serde_json::Value| {
        let r = thbce_core::valider(&projet(v));
        assert!(
            r.erreurs.iter().all(|e| !e.hors_perimetre),
            "{:?}",
            r.erreurs
        );
        r.erreurs
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
    };
    let v = json_du_cas(EXEMPLE);
    assert!(messages(&v).is_empty());
    // Fonctions différentes : la seconde chaudière ne fait que le chauffage.
    let mut w = v.clone();
    w["batiments"][0]["generations"][0]["generateurs"][1]["nature"]["chaudiere"]["fonction"] =
        "chauffage".into();
    let m = messages(&w);
    assert!(m.iter().any(|m| m.contains("même fonction")), "{m:?}");
    // Ballon de stockage : la génération est régulée en cascade (§8.17.3.6).
    let mut w = json_du_cas("maison-chaudiere-gaz-ademe");
    w["batiments"][0]["generations"][0]["priorite"] = "alternance".into();
    let m = messages(&w);
    assert!(m.iter().any(|m| m.contains("régulée en cascade")), "{m:?}");
}

/// La PAC hybride n'est pas un générateur du tableau 111 : la fiche d'application du §16.4 la
/// décrit par ses composants (lecture Q126).
#[test]
fn pac_hybride_renvoie_a_la_fiche_d_application() {
    let mut v = json_du_cas(EXEMPLE);
    v["batiments"][0]["generations"][0]["generateurs"][0]["nature"] = "pac_hybride".into();
    let r = thbce_core::valider(&projet(&v));
    assert!(r.seulement_hors_perimetre(), "{:?}", r.erreurs);
    assert!(r.erreurs.iter().any(|e| e.message.contains("§16.4")));
}

/// Un générateur hors périmètre seul sur un poste donne le seul refus de code 3 : le contrôle
/// de montage du §8.17.3.1 est sauté (pas d'erreur « aucun générateur n'assure le poste »).
#[test]
fn generateur_hors_perimetre_seul_refuse_en_code_3_seul() {
    // Natures encore refusées (code 3) : la PAC hybride saisie en un seul générateur (Q126).
    let nature = "pac_hybride";
    let mut v = json_du_cas(EXEMPLE);
    let g = &mut v["batiments"][0]["generations"][0];
    g["generateurs"].as_array_mut().unwrap().truncate(1);
    g["generateurs"][0]["nature"] = nature.into();
    g.as_object_mut().unwrap().remove("priorite");
    let r = thbce_core::valider(&projet(&v));
    assert!(!r.erreurs.is_empty());
    assert!(r.seulement_hors_perimetre(), "{nature} : {:?}", r.erreurs);
}
