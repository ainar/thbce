// SPDX-License-Identifier: GPL-3.0-or-later
//! Escalators et tapis roulants (fiche 10.2, Q115) de bout en bout sur le commerce d'exemple
//! (météo ouverte ADEME). Relations du texte vérifiées (éq. 2317, 2318, 2320 à 2324), non des
//! constantes tirées du moteur.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::Projet;
use thbce_core::{calculer, valider, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cas() -> Projet {
    let chemin = racine().join("tests/cas/commerce-escalators-ademe/entree.json");
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn calcul(p: &Projet) -> thbce_core::resultats::Resultats {
    let zone = p.localisation.zone_climatique_retenue();
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    let m: MeteoProjet = meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap();
    calculer(p, &m, &Options { arrondir: false }).unwrap()
}

#[test]
fn escalators_eq_2317_2318_et_puissances() {
    let r = calcul(&cas());
    let b = &r.batiments[0];
    let e = b.escalators.as_ref().unwrap();
    // Éq. (2318) : M = 75 · BVNB ; tableau 310 : Bv(17) = 4158.
    assert_eq!(e.m_kg, 75.0 * e.bvnb);
    // BVNB = Bv · NB (éq. 2317) : multiple de 4158 par le nombre d'occupants maximal (154,8).
    assert!((e.bvnb / 4158.0 - 154.8).abs() < 1e-6);
    // Puissances à vide cumulées : montée 3000 + tapis 1200 ; descente (basse vitesse) 900.
    assert_eq!(e.p_noload_m_w, 4200.0);
    assert_eq!(e.p_noload_d_w, 900.0);
    assert_eq!(e.h_m_m, 4.5);
    // Arrêt : 88 (défaut, veille inconnue) + 60 + 120 (arrêt sans passager).
    assert_eq!(e.p_off_w, 268.0);
    // Le poste « déplacements » du relevé électrique porte la consommation des escalators.
    let c = b.consommations_collectif.as_ref().unwrap();
    let kwh = c.escalators.as_ref().unwrap().annuel_kwh;
    assert!((kwh - e.e_tot_kwh).abs() < 1e-6);
    let indic = b.indicateurs.as_ref().unwrap();
    let dep = indic.electricite_consommee.deplacements_kwh;
    assert!((dep - kwh).abs() < 1e-6);
    // Borne basse : les heures hors occupation consomment au moins 268 W.
    let nb = e.nb_h_occupation as f64;
    assert!(kwh >= (8760.0 - nb) * 268.0 / 1000.0);
}

#[test]
fn escalator_valeurs_requises_et_zone_inconnue() {
    let mut p = cas();
    {
        let e = &mut p.batiments[0].escalators[1];
        e.p_slowspeed_w = None; // slowspeed sans puissance : refusé
        p.batiments[0].escalators[0].zones_desservies = vec!["absente".into()];
    }
    let chemins: Vec<String> = valider(&p)
        .erreurs
        .iter()
        .map(|e| e.chemin.clone())
        .collect();
    assert!(chemins
        .iter()
        .any(|c| c.ends_with("escalators[1].p_slowspeed_w")));
    assert!(chemins
        .iter()
        .any(|c| c.contains("escalators[0].zones_desservies")));
}
