// SPDX-License-Identifier: GPL-3.0-or-later
//! Brasseurs d'air (fiche 8.32, lot 1-emi) de bout en bout sur la maison d'exemple
//! `tests/cas/maison-brasseurs-ademe`, sur la météo ouverte ADEME (CI publique) et sur la météo
//! réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test brasseurs -- --ignored --nocapture
//! ```
//!
//! Les valeurs de l'effet sont calculées à la main dans les tests du modèle
//! (`chauffage_refroidissement/brasseurs_air.rs`) ; ici, bornes calculées à la main à partir du
//! texte et sens des effets.

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

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cas() -> Projet {
    let chemin = racine().join("tests/cas/maison-brasseurs-ademe/entree.json");
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn sans_brasseurs() -> Projet {
    let mut p = cas();
    p.batiments[0].zones[0].groupes[0].brasseurs_air.clear();
    p
}

fn meteo_ademe(projet: &Projet) -> MeteoProjet {
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    let zone = projet.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

fn calcul(p: &Projet, m: &MeteoProjet) -> Resultats {
    let r = thbce_core::valider(p);
    assert!(r.est_valide(), "{:?}", r.erreurs);
    calculer(p, m, &Options { arrondir: false }).unwrap()
}

/// Bornes et sens des effets, calculés à la main :
///
/// - puissance maximale : 3 × 60 W (certifiée) + 2 × 40 × 1,1 W (justifiée) = 268 W ;
///   puissance minimale d'un type qui tourne : 2 × 40 × 1,1 × 0,6 = 52,8 W ; la consommation
///   est comprise entre `nbh × 52,8 W` et `nbh × 268 W` ;
/// - effet sur la température opérative : séjour, `Ratbr = min(15 × 3 / 96,44 ; 0,45) = 0,45`,
///   `τ = 36 000 / (241,1 × 0,45) = 331,8 vol/h`, `v = 1,062 m/s` ; à `θrm − θi` voisin de 0 à
///   1 K, `Δθop^BA_p` vaut 3,16 à 3,28 K, ramené par l'éq. (1660) à `0,45 / 0,6 = 0,75` de sa
///   valeur : 2,37 à 2,46 K.
fn verifier(r: &Resultats, base: &Resultats) {
    let g = &r.batiments[0].zones[0].groupes[0];
    let b = g.brasseurs.as_ref().expect("brasseurs restitués");
    assert!(b.nbh_fonctionnement > 0 && b.nbh_fonctionnement_th_d > 0);
    let nbh = f64::from(b.nbh_fonctionnement);
    assert!(
        b.w_abs_kwh >= nbh * 52.8e-3 && b.w_abs_kwh <= nbh * 268.0e-3,
        "{} kWh pour {} h",
        b.w_abs_kwh,
        nbh
    );
    assert!(
        (2.3..=2.5).contains(&b.delta_theta_op_max_k),
        "{}",
        b.delta_theta_op_max_k
    );
    // Sens des effets : moins d'inconfort, plus de consommation d'auxiliaires.
    let dh = r.batiments[0].dh_c_h.unwrap();
    let dh_base = base.batiments[0].dh_c_h.unwrap();
    assert!(dh < dh_base, "DH {dh} contre {dh_base} sans brasseur");
    // Le Bbio ne dépend pas des brasseurs (fiche 8.32 : Th-D et Th-C seulement).
    assert_eq!(r.batiments[0].bbio_pts, base.batiments[0].bbio_pts);
    assert!(base.batiments[0].zones[0].groupes[0].brasseurs.is_none());
    // La chaleur des brasseurs est récupérée par le groupe (§11.1) : le calcul des consommations
    // en comptabilise davantage.
    let recup = |r: &Resultats| {
        r.batiments[0].zones[0].groupes[0]
            .systemes
            .as_ref()
            .unwrap()
            .recup_kwh
    };
    assert!(recup(r) > recup(base));
    // Valeurs par défaut signalées : `RatUs_BA` du tableau 257 (maison : 0,6 le jour, 0,4 la nuit).
    let defauts: Vec<_> = g
        .valeurs_par_defaut
        .iter()
        .filter(|v| v.champ.starts_with("brasseurs_air"))
        .collect();
    assert_eq!(defauts.len(), 2);
    assert_eq!(defauts[0].valeur, 0.6);
    assert_eq!(defauts[1].valeur, 0.4);
}

#[test]
fn brasseurs_meteo_ademe() {
    let p = cas();
    let m = meteo_ademe(&p);
    let base = calcul(&sans_brasseurs(), &m);
    let r = calcul(&p, &m);
    verifier(&r, &base);
}

#[test]
fn brasseurs_exemple_inchange_sans_brasseur() {
    // Sans brasseur, le résultat est celui de la maison à chaudière gaz de départ.
    let p = sans_brasseurs();
    let m = meteo_ademe(&p);
    let r = calcul(&p, &m);
    let chemin = racine().join("tests/cas/maison-chaudiere-gaz-ademe/entree.json");
    let d = json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap();
    let r0 = calcul(&d, &m);
    assert_eq!(r.batiments[0].dh_c_h, r0.batiments[0].dh_c_h);
    assert_eq!(r.batiments[0].cep_kwhep_m2, r0.batiments[0].cep_kwhep_m2);
}

#[test]
fn refus_de_saisie() {
    // Groupe climatisé.
    let mut p = cas();
    p.batiments[0].zones[0].groupes[0].iclim = Some(true);
    let r = thbce_core::valider(&p);
    assert!(r.erreurs.iter().any(|e| e.chemin.ends_with(".iclim")));
    // Surface couverte supérieure à la surface occupée (jour : 0,6) : 5 brasseurs couvrent
    // `min(75/96,44 ; 0,45)` = 0,45 ; deux types de 0,45 dépassent 0,6.
    let mut p = cas();
    let g = &mut p.batiments[0].zones[0].groupes[0];
    let mut deuxieme = g.brasseurs_air[0].clone();
    deuxieme.id = "autre".into();
    g.brasseurs_air.push(deuxieme);
    let r = thbce_core::valider(&p);
    assert!(
        r.erreurs
            .iter()
            .any(|e| e.message.contains("surface occupée")),
        "{:?}",
        r.erreurs
    );
    // Un usage « jour et nuit » exclut les autres.
    let mut p = cas();
    p.batiments[0].zones[0].groupes[0].brasseurs_air[0].usage =
        thbce_core::entree::UsageBrasseur::JourEtNuit;
    let r = thbce_core::valider(&p);
    assert!(r.erreurs.iter().any(|e| e.message.contains("jour et nuit")));
}

/// Valeurs phares sur la météo réglementaire (nombres annuels seulement).
#[test]
#[ignore = "nécessite la météo réglementaire (THBCE_METEO)"]
fn brasseurs_meteo_reglementaire() {
    let p = cas();
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO");
    let zone = p.localisation.zone_climatique_retenue();
    let m = meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap();
    let base = calcul(&sans_brasseurs(), &m);
    let r = calcul(&p, &m);
    let b = r.batiments[0].zones[0].groupes[0]
        .brasseurs
        .as_ref()
        .unwrap();
    println!(
        "DH {:.1} (sans brasseur {:.1}) ; Cep {:.2} (sans {:.2}) ; W {:.1} kWh ; {} h Th-C ; {} h Th-D ; Δθmax {:.2} K",
        r.batiments[0].dh_c_h.unwrap(),
        base.batiments[0].dh_c_h.unwrap(),
        r.batiments[0].cep_kwhep_m2.unwrap(),
        base.batiments[0].cep_kwhep_m2.unwrap(),
        b.w_abs_kwh,
        b.nbh_fonctionnement,
        b.nbh_fonctionnement_th_d,
        b.delta_theta_op_max_k
    );
    // Valeurs phares (arrondies à une décimale) : garde-fou contre une régression.
    let a1 = |x: f64| (x * 10.0).round() / 10.0;
    assert_eq!(a1(base.batiments[0].dh_c_h.unwrap()), 1103.3);
    assert_eq!(a1(r.batiments[0].dh_c_h.unwrap()), 724.8);
    assert_eq!(a1(b.w_abs_kwh), 16.1);
    assert_eq!(
        (b.nbh_fonctionnement, b.nbh_fonctionnement_th_d),
        (219, 456)
    );
}
