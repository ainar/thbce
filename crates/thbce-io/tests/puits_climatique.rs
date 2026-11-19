// SPDX-License-Identifier: GPL-3.0-or-later
//! Puits climatique (fiches 6.8, 6.10 et 6.11, lot 1-ven8) de bout en bout : maison double flux
//! (`tests/cas/maison-puits-climatique-ademe`) et centrale de traitement d'air
//! (`tests/cas/bureaux-cta-puits-climatique-ademe`), sur la météo ouverte ADEME (CI publique)
//! et sur la météo réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test puits_climatique -- --ignored --nocapture
//! ```
//!
//! Un puits d'efficacité quasi nulle (longueur quasi nulle) laisse les résultats arrondis
//! inchangés ; les effets sur le DH et le préchauffage sont des propriétés physiques, aucun cas de
//! référence n'étant disponible.

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
use thbce_core::resultats::{Resultats, ResultatsPuitsClimatique};
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

fn bbio(r: &Resultats) -> f64 {
    r.batiments[0].bbio_pts.unwrap()
}

fn dh(r: &Resultats) -> f64 {
    r.batiments[0].dh_c_h.unwrap()
}

fn cep(r: &Resultats) -> f64 {
    r.batiments[0].cep_kwhep_m2.unwrap()
}

/// Fonctionnement du puits en Th-C et en Th-D.
fn puits(r: &Resultats) -> (ResultatsPuitsClimatique, ResultatsPuitsClimatique) {
    let v = r.batiments[0].zones[0].ventilation.as_ref().unwrap();
    (
        v.th_c.as_ref().unwrap().puits_climatique.unwrap(),
        v.th_d.as_ref().unwrap().puits_climatique.unwrap(),
    )
}

/// Résultats arrondis (§1.3.1) en JSON, sans le fonctionnement du puits, ni les noms et
/// libellés qui distinguent les deux projets comparés. Un puits d'efficacité de l'ordre de
/// 10⁻¹¹ écarte les résultats en pleine précision de moins de 10⁻⁹ : la comparaison se fait sur
/// la restitution arrondie à une décimale.
fn json_hors_puits(projet: &Projet, m: &MeteoProjet) -> serde_json::Value {
    let r = calculer(projet, m, &Options { arrondir: true }).unwrap();
    fn nettoyer(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                o.retain(|k, _| !matches!(k.as_str(), "puits_climatique" | "nom" | "regle"));
                o.values_mut().for_each(nettoyer);
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(nettoyer),
            _ => {}
        }
    }
    let mut v = serde_json::from_str(&json::ecrire_resultats(&r)).unwrap();
    nettoyer(&mut v);
    v
}

/// Même projet avec un puits de longueur quasi nulle : `ε ≈ 0`, l'air neuf reste à `θext`.
fn avec_puits_inefficace(nom: &str) -> Projet {
    let mut p = cas(nom);
    let v = p.batiments[0].zones[0].ventilation.as_mut().unwrap();
    v.puits_climatique.as_mut().unwrap().l_m = 1e-9;
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);
    p
}

fn verifier(nom: &str, reference: &str, m: &MeteoProjet, titre: &str) {
    let p = cas(nom);
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);
    let r = brut(&p, m);
    let sans = brut(&cas(reference), m);
    // Th-B : pas de puits dans le Bbio (éq. 756 et 757, §6.11.3).
    assert_eq!(bbio(&r), bbio(&sans), "{titre} {nom}");
    let (c, d) = puits(&r);
    for x in [c, d] {
        assert!(x.nbh_actif > 0 && x.nbh_actif <= 8760, "{titre} {nom}");
        assert!(x.q_prechauffage_kwh > 0.0, "{titre} {nom}");
        assert!(x.q_prerefroidissement_kwh < 0.0, "{titre} {nom}");
    }
    // Air neuf prérafraîchi l'été et préchauffé l'hiver : moins d'inconfort d'été et moins de
    // besoins.
    assert!(dh(&r) < dh(&sans), "{titre} {nom}");
    assert!(cep(&r) < cep(&sans), "{titre} {nom}");
    println!(
        "{titre} : {nom}, Bbio {:.1}, DH {:.1} (sans puits {:.1}), Cep {:.1} ({:.1}), Th-C {} h et {:.1}/{:.1} kWh, Th-D {} h",
        bbio(&r),
        dh(&r),
        dh(&sans),
        cep(&r),
        cep(&sans),
        c.nbh_actif,
        c.q_prechauffage_kwh,
        c.q_prerefroidissement_kwh,
        d.nbh_actif
    );
    // Un puits sans effet (ε ≈ 0) donne les résultats de l'exemple sans puits.
    assert_eq!(
        json_hors_puits(&avec_puits_inefficace(nom), m),
        json_hors_puits(&cas(reference), m),
        "{titre} {nom}"
    );
}

#[test]
fn puits_climatique_sur_la_meteo_ademe() {
    let m = meteo_ademe(&cas("maison-puits-climatique-ademe"));
    verifier(
        "maison-puits-climatique-ademe",
        "maison-double-flux-ademe",
        &m,
        "ADEME",
    );
    verifier(
        "bureaux-cta-puits-climatique-ademe",
        "bureaux-cta-ademe",
        &m,
        "ADEME",
    );
    // La centrale voit l'air neuf du puits : le préchauffage demande moins d'énergie (éq. 521
    // à 525, `θsouf2` plus chaude) ; la consommation des ventilateurs reste celle de la
    // centrale (le puits n'a pas de ventilateur propre, Q231).
    let r = brut(&cas("bureaux-cta-puits-climatique-ademe"), &m);
    let sans = brut(&cas("bureaux-cta-ademe"), &m);
    let q = |r: &Resultats| {
        r.batiments[0].zones[0]
            .ventilation
            .as_ref()
            .unwrap()
            .th_c
            .as_ref()
            .unwrap()
            .q_prechaud_kwh
            .unwrap()
    };
    assert!(q(&r) < q(&sans));
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn puits_climatique_sur_la_meteo_reglementaire() {
    let m = meteo_reglementaire(&cas("maison-puits-climatique-ademe"));
    // Valeurs phares des deux exemples (nombres agrégés annuels, calculés par le moteur le
    // 19/11/2026, non comparés à un logiciel de référence) : Bbio, Cep, DH.
    for (nom, attendus) in [
        ("maison-puits-climatique-ademe", [52.7, 119.5, 709.4]),
        ("bureaux-cta-puits-climatique-ademe", [82.9, 70.0, 208.9]),
    ] {
        let r = brut(&cas(nom), &m);
        for (obtenu, attendu) in [bbio(&r), cep(&r), dh(&r)].into_iter().zip(attendus) {
            assert!(
                (obtenu - attendu).abs() < 0.05,
                "{nom} : {obtenu} au lieu de {attendu}"
            );
        }
    }
    verifier(
        "maison-puits-climatique-ademe",
        "maison-double-flux-ademe",
        &m,
        "réglementaire",
    );
    verifier(
        "bureaux-cta-puits-climatique-ademe",
        "bureaux-cta-ademe",
        &m,
        "réglementaire",
    );
}
