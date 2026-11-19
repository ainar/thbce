// SPDX-License-Identifier: GPL-3.0-or-later
//! Réseau de CTA hydraulique (fiches 8.11 et 8.12, lot 1-emi) de bout en bout sur
//! `tests/cas/bureaux-cta-reseau-ademe`, météo ouverte ADEME : bornes calculées à la main et
//! comparaison au même bâtiment avec réseau fictif (`bureaux-cta-ademe`).

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::Projet;
use thbce_core::resultats::Resultats;
use thbce_core::{calculer, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn projet(nom: &str) -> Projet {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn calcul(p: &Projet) -> Resultats {
    let r = thbce_core::valider(p);
    assert!(r.est_valide(), "{:?}", r.erreurs);
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    let m: MeteoProjet = meteo::lire(
        &std::fs::read(chemin).unwrap(),
        p.localisation.zone_climatique_retenue(),
    )
    .unwrap();
    calculer(p, &m, &Options { arrondir: false }).unwrap()
}

/// Circulateur de 40 W : au plus 40 W × 8 760 h = 350,4 kWh ; pertes en volume chauffé
/// de 20 m × 0,2 W/(m.K) × (60 − 20) K = 160 W au plus, soit 1 401,6 kWh sur l'année ; le réseau
/// n'ajoute que de la consommation au bâtiment sans réseau (même Bbio, DH inchangé).
#[test]
fn reseau_chaud_borne_a_la_main() {
    let r = calcul(&projet("bureaux-cta-reseau-ademe"));
    let b = &r.batiments[0];
    assert_eq!(b.reseaux_cta.len(), 1);
    let n = &b.reseaux_cta[0];
    assert_eq!(n.fonction, "chauffage");
    assert!(n.nbh_fonct > 0 && n.nbh_fonct <= 8760);
    assert!(n.w_aux_kwh > 0.0 && n.w_aux_kwh <= 350.4 + 1e-6);
    assert!(n.pertes_vc_kwh > 0.0 && n.pertes_vc_kwh <= 1401.6 + 1e-6);
    assert!(n.pertes_hvc_kwh.abs() < 1e-9);
    assert!(n.q_kwh > 0.0);

    let f = calcul(&projet("bureaux-cta-ademe"));
    let fb = &f.batiments[0];
    assert!(fb.reseaux_cta.is_empty());
    assert!((b.bbio_pts.unwrap() - fb.bbio_pts.unwrap()).abs() < 1e-9);
    assert!(b.cep_kwhep_m2 > fb.cep_kwhep_m2);
}

/// Un réseau sans fonction reliée à une génération est refusé.
#[test]
fn reseau_sans_objet_refuse() {
    let mut p = projet("bureaux-cta-ademe");
    let v = p.batiments[0].zones[0].ventilation.as_mut().unwrap();
    let r = projet("bureaux-cta-reseau-ademe").batiments[0].zones[0]
        .ventilation
        .as_ref()
        .unwrap()
        .reseau_cta_chaud
        .clone();
    v.reseau_cta_froid = r;
    let rapport = thbce_core::valider(&p);
    assert!(!rapport.est_valide());
}
