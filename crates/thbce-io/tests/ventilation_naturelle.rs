// SPDX-License-Identifier: GPL-3.0-or-later
//! Ventilation naturelle et hybride par conduits (fiche 6.6, lot 1-ven10) de bout en bout :
//! `tests/cas/maison-ventilation-naturelle-ademe` et `tests/cas/maison-ventilation-hybride-ademe`,
//! sur la météo ouverte ADEME (CI publique) et sur la météo réglementaire (test ignoré par
//! défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test ventilation_naturelle -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les propriétés vérifiées sont des égalités du texte
//! (consommation de l'assistance, éq. 708 ; Th-B inchangé) ou des ordres de grandeur.

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
use thbce_core::resultats::{Resultats, ResultatsNaturelle};
use thbce_core::{calculer, valider, Options};
use thbce_io::{json, meteo};

const NATURELLE: &str = "maison-ventilation-naturelle-ademe";
const HYBRIDE: &str = "maison-ventilation-hybride-ademe";

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

/// Restitution du système en Th-C et en Th-D.
fn naturelle(r: &Resultats) -> (ResultatsNaturelle, ResultatsNaturelle) {
    let v = r.batiments[0].zones[0].ventilation.as_ref().unwrap();
    (
        v.th_c.as_ref().unwrap().naturelle.unwrap(),
        v.th_d.as_ref().unwrap().naturelle.unwrap(),
    )
}

fn aux_vent(r: &Resultats) -> f64 {
    r.batiments[0].zones[0]
        .ventilation
        .as_ref()
        .unwrap()
        .c_aux_vent_kwh
}

fn verifier(m: &MeteoProjet, titre: &str) {
    let pn = cas(NATURELLE);
    let ph = cas(HYBRIDE);
    for p in [&pn, &ph] {
        assert!(valider(p).est_valide(), "{:?}", valider(p).erreurs);
    }
    let rn = brut(&pn, m);
    let rh = brut(&ph, m);
    // Th-B : le Bbio emploie le système conventionnel, quel que soit le système du projet.
    assert_eq!(
        rn.batiments[0].bbio_pts, rh.batiments[0].bbio_pts,
        "{titre} : Bbio"
    );
    // Ventilation naturelle : pas de ventilateur.
    assert_eq!(aux_vent(&rn), 0.0, "{titre}");
    let (nc, nd) = naturelle(&rn);
    for x in [nc, nd] {
        assert!(x.nbh_extraction > 0, "{titre}");
        assert!(x.nbh_extraction + x.nbh_refoulement <= 8760, "{titre}");
        assert!(x.nbh_assistance.is_none(), "{titre}");
    }
    // Ventilation hybride : l'assistance (éq. 708) est la seule consommation de ventilation.
    let (hc, hd) = naturelle(&rh);
    for x in [hc, hd] {
        let w = x.w_assistance_kwh.unwrap();
        assert!(w > 0.0, "{titre}");
        assert!(x.nbh_assistance.unwrap() > 0, "{titre}");
        assert!(x.nbh_grand_debit.unwrap() > 0, "{titre}");
    }
    assert!(
        (aux_vent(&rh) - hc.w_assistance_kwh.unwrap()).abs() < 1e-6,
        "{titre}"
    );
    println!(
        "{titre} : naturelle DH {:.1} Cep {:.1} extraction {} h refoulement {} h ; hybride DH {:.1} Cep {:.1} assistance {} h {:.1} kWh",
        rn.batiments[0].dh_c_h.unwrap(),
        rn.batiments[0].cep_kwhep_m2.unwrap(),
        nc.nbh_extraction,
        nc.nbh_refoulement,
        rh.batiments[0].dh_c_h.unwrap(),
        rh.batiments[0].cep_kwhep_m2.unwrap(),
        hc.nbh_assistance.unwrap(),
        hc.w_assistance_kwh.unwrap(),
    );
}

#[test]
fn ventilation_naturelle_et_hybride_sur_la_meteo_ademe() {
    let m = meteo_ademe(&cas(NATURELLE));
    verifier(&m, "ADEME");
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn ventilation_naturelle_et_hybride_sur_la_meteo_reglementaire() {
    let m = meteo_reglementaire(&cas(NATURELLE));
    verifier(&m, "réglementaire");
}
