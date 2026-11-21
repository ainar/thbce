// SPDX-License-Identifier: GPL-3.0-or-later
//! Échangeur de géocooling d'une génération de froid (fiche 8.30, lot 2g, lecture Q252), de
//! bout en bout sur l'exemple `maison-geocooling-ademe`, sur la météo ouverte ADEME (CI
//! publique) et sur la météo réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test geocooling -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les tests vérifient des propriétés (bilans, sens
//! des effets) et les refus ; les calculs à la main sont dans
//! `chauffage_refroidissement/echangeur_geocooling.rs`.

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
use thbce_core::resultats::{Resultats, ResultatsGeneration};
use thbce_core::{calculer, Options};
use thbce_io::{json, meteo};

const GEO: &str = "maison-geocooling-ademe";

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

fn calcul(v: &serde_json::Value, meteo_de: &dyn Fn(&Projet) -> MeteoProjet) -> Resultats {
    let p = projet(v);
    let r = thbce_core::valider(&p);
    assert!(r.est_valide(), "{:?}", r.erreurs);
    calculer(&p, &meteo_de(&p), &Options { arrondir: false }).unwrap()
}

fn froid(r: &Resultats) -> &ResultatsGeneration {
    r.batiments[0]
        .generations
        .iter()
        .find(|g| g.id == "generation-froid")
        .unwrap()
}

/// Saisie du géocooling, modifiable.
fn geo(v: &mut serde_json::Value) -> &mut serde_json::Value {
    &mut v["batiments"][0]["generations"][2]["generateurs"][0]["nature"]["geocooling"]
}

/// Le géocooling, prioritaire, fournit une part de la demande de froid ; le refroidisseur
/// complète ; sa seule consommation propre est celle du circulateur primaire (éq. 1593),
/// celle des pompes des sondes s'y ajoute (auxiliaires amont).
#[test]
fn base_et_complement() {
    let r = calcul(&json_du_cas(GEO), &meteo_ademe);
    let g = froid(&r);
    let (geo, frigo) = (&g.generateurs[0], &g.generateurs[1]);
    assert!(geo.q_fou_fr_kwh > 0.0 && frigo.q_fou_fr_kwh > 0.0);
    assert!((geo.q_fou_fr_kwh + frigo.q_fou_fr_kwh - g.q_fou_fr_kwh).abs() < 1e-6);
    assert!((g.q_fou_fr_kwh - g.q_fr_kwh).abs() < 1e-6 * g.q_fr_kwh);
    // Éq. (1592) : l'énergie rejetée à la source est l'énergie fournie.
    assert!((geo.phi_rejet_kwh - geo.q_fou_fr_kwh).abs() < 1e-6);
    // Circulateur primaire de 20 W au plus, plus les pompes des sondes de 60 W au plus.
    let elec = geo.consommations.refroidissement.electricite_kwh;
    assert!(elec > 0.0 && elec < 80.0e-3 * 8760.0);
    // Le géocooling consomme bien moins que le refroidisseur pour l'énergie qu'il fournit.
    assert!(geo.eta_eff_fr_an.unwrap() > 3.0 * frigo.eta_eff_fr_an.unwrap());
}

/// Sens des effets : association alternée (éq. 1587), retour de source limité plus bas
/// (éq. 1573), échangeur moins performant (éq. 1576 à 1581) ; sans échangeur, jamais utilisé
/// (éq. 1588, lecture Q252).
#[test]
fn sens_des_effets() {
    let base = calcul(&json_du_cas(GEO), &meteo_ademe);
    let q = |r: &Resultats| froid(r).generateurs[0].q_fou_fr_kwh;
    let variante = |f: &dyn Fn(&mut serde_json::Value)| {
        let mut v = json_du_cas(GEO);
        f(geo(&mut v));
        calcul(&v, &meteo_ademe)
    };
    let alterne = variante(&|g| g["association"] = "alterne".into());
    assert!(q(&alterne) < q(&base), "{} {}", q(&alterne), q(&base));
    let limite = variante(&|g| g["theta_r_source_lim_c"] = 11.0.into());
    assert!(q(&limite) < q(&base));
    let petit = variante(&|g| g["echangeur_intermediaire"]["ua_geo_nom_w_k"] = 200.0.into());
    assert!(q(&petit) < q(&base));
    let sans = variante(&|g| {
        g.as_object_mut().unwrap().remove("echangeur_intermediaire");
    });
    assert_eq!(q(&sans), 0.0);
    // Sans association saisie : base et complément, valeur conventionnelle signalée.
    let defaut = variante(&|g| {
        g.as_object_mut().unwrap().remove("association");
    });
    assert_eq!(q(&defaut), q(&base));
    assert!(froid(&defaut).generateurs[0]
        .valeurs_par_defaut
        .iter()
        .any(|d| d.champ == "association"));
}

/// Refus et erreurs de saisie.
#[test]
fn erreurs() {
    let erreurs = |v: &serde_json::Value| thbce_core::valider(&projet(v));
    let msg = |v: &serde_json::Value, texte: &str| {
        let r = erreurs(v);
        assert!(
            r.erreurs
                .iter()
                .any(|e| !e.hors_perimetre && e.message.contains(texte)),
            "{texte} : {:?}",
            r.erreurs
        );
    };
    // Source sur sol (capteurs à détente directe) : pas d'eau.
    let mut v = json_du_cas(GEO);
    geo(&mut v)["source_amont"] = serde_json::json!({"sol": {
        "sol": {"theta_min_c": 5.0, "theta_max_c": 10.0, "mois_minimum": 2},
        "rb_k_m_w": 0.1, "l_m": 100.0}});
    msg(&v, "géothermique sur eau");
    // Génération sans priorité.
    let mut v = json_du_cas(GEO);
    v["batiments"][0]["generations"][2]["priorite"] = "sans_priorite".into();
    msg(&v, "cascade");
    // Le refroidisseur au même rang que le géocooling.
    let mut v = json_du_cas(GEO);
    v["batiments"][0]["generations"][2]["generateurs"][1]["priorite_refroidissement"] = 1.into();
    msg(&v, "prioritaire");
    // Plages.
    let mut v = json_du_cas(GEO);
    geo(&mut v)["delta_theta_aval_k"] = 0.0.into();
    geo(&mut v)["theta_r_source_lim_c"] = 30.0.into();
    let r = erreurs(&v);
    assert!(r
        .erreurs
        .iter()
        .any(|e| e.chemin.ends_with("delta_theta_aval_k")));
    assert!(r
        .erreurs
        .iter()
        .any(|e| e.chemin.ends_with("theta_r_source_lim_c")));
}

/// Valeurs phares sur la météo réglementaire (nombres annuels seulement).
#[test]
#[ignore = "météo réglementaire (THBCE_METEO)"]
fn geocooling_sur_meteo_reglementaire() {
    let r = calcul(&json_du_cas(GEO), &meteo_reglementaire);
    let b = &r.batiments[0];
    let g = froid(&r);
    println!(
        "{GEO} : Bbio {:.1}, DH {:.1}, Cep {:.1}, Cep,nr {:.1}, Ic énergie {:.1} ; froid \
         {:.1} kWh, dont géocooling {:.1} kWh",
        b.bbio_pts.unwrap(),
        b.dh_c_h.unwrap(),
        b.cep_kwhep_m2.unwrap(),
        b.cep_nr_kwhep_m2.unwrap(),
        b.ic_energie_kgco2_m2.unwrap(),
        g.q_fou_fr_kwh,
        g.generateurs[0].q_fou_fr_kwh
    );
    assert!(g.generateurs[0].q_fou_fr_kwh > 0.0);
}
