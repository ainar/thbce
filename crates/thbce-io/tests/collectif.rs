// SPDX-License-Identifier: GPL-3.0-or-later
//! Postes du logement collectif et des parkings (lot 6e) : propriétés sur l'immeuble d'exemple,
//! météo ouverte ADEME. Les valeurs sont vérifiées par `tests/cas/immeuble-collectif-ademe`.

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

fn immeuble() -> Projet {
    let chemin = racine().join("tests/cas/immeuble-collectif-ademe/entree.json");
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn calcul(p: &Projet) -> thbce_core::resultats::Resultats {
    let zone = p.localisation.zone_climatique_retenue();
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    let m: MeteoProjet = meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap();
    calculer(p, &m, &Options { arrondir: false }).unwrap()
}

#[test]
fn les_postes_se_somment_et_les_mois_redonnent_l_annee() {
    let r = calcul(&immeuble());
    let b = &r.batiments[0];
    let c = b.consommations_collectif.as_ref().unwrap();
    let z = b.zones[0].consommations_collectif.as_ref().unwrap();
    for (bat, zone) in [
        (&c.eclairage_parties_communes, &z.eclairage_parties_communes),
        (&c.ascenseurs, &z.ascenseurs),
        (&c.ventilation_parkings, &z.ventilation_parkings),
        (&c.eclairage_parkings, &z.eclairage_parkings),
    ] {
        assert!((bat.annuel_kwh - zone.annuel_kwh).abs() < 1e-9);
        assert!((bat.mensuel_kwh.iter().sum::<f64>() - bat.annuel_kwh).abs() < 1e-9);
        assert_eq!(bat.mensuel_kwh.len(), 12);
    }
    // Le parking et l'ascenseur sont ceux du bâtiment.
    assert!((b.parkings[0].e_ecl_kwh - c.eclairage_parkings.annuel_kwh).abs() < 1e-9);
    assert!((b.ascenseurs[0].e_tot_kwh - c.ascenseurs.annuel_kwh).abs() < 1e-9);
}

#[test]
fn sans_ascenseur_ni_parking_il_reste_les_parties_communes_et_le_bbio_est_inchange() {
    let avec = calcul(&immeuble());
    let mut p = immeuble();
    p.batiments[0].ascenseurs.clear();
    p.batiments[0].parkings.clear();
    let sans = calcul(&p);
    let (a, s) = (&avec.batiments[0], &sans.batiments[0]);
    let c = s.consommations_collectif.as_ref().unwrap();
    assert_eq!(c.ascenseurs.annuel_kwh, 0.0);
    assert_eq!(c.eclairage_parkings.annuel_kwh, 0.0);
    assert!(c.eclairage_parties_communes.annuel_kwh > 0.0);
    // Aucun apport interne : thermique inchangée.
    assert_eq!(a.bbio_pts, s.bbio_pts);
    assert_eq!(a.dh_c_h, s.dh_c_h);
}

#[test]
fn maison_individuelle_sans_parking_n_a_aucun_poste_collectif() {
    let chemin = racine().join("tests/cas/maison-climat-ademe/entree.json");
    let p = json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap();
    let r = calcul(&p);
    assert!(r.batiments[0].consommations_collectif.is_none());
}

fn erreurs(p: &Projet) -> Vec<String> {
    valider(p)
        .erreurs
        .into_iter()
        .map(|e| format!("{} : {}", e.chemin, e.message))
        .collect()
}

#[test]
fn la_validation_refuse_les_saisies_incoherentes() {
    assert!(erreurs(&immeuble()).is_empty());
    let mut p = immeuble();
    p.batiments[0].ascenseurs[0].q_kg = 50.0;
    p.batiments[0].ascenseurs[0].v_m_s = 3.0;
    p.batiments[0].ascenseurs[0].netage = 1;
    p.batiments[0].ascenseurs[0].zones_desservies = vec!["inconnue".into()];
    p.batiments[0].parkings[0].jours_fermes = vec![0, 366];
    let e = erreurs(&p).join("\n");
    for attendu in [
        "q_kg",
        "v_m_s",
        "netage",
        "zones_desservies[0]",
        "jours_fermes",
        "extinction_en_fermeture",
    ] {
        assert!(e.contains(attendu), "{attendu} absent de :\n{e}");
    }
    let mut p = immeuble();
    p.batiments[0].ascenseurs[0].tech_mac = thbce_core::entree::TechMac::Hydraulique;
    p.batiments[0].ascenseurs[0].cp = Some(0.5);
    assert!(erreurs(&p).join("\n").contains("hydraulique"));
}
