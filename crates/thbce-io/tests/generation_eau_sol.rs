// SPDX-License-Identifier: GPL-3.0-or-later
//! Pompes à chaleur et refroidisseurs sur eau et sur sol (fiche 8.23) et leurs sources amont
//! (fiches 8.26 et 8.27, lot 2d), de bout en bout sur les exemples `maison-pac-geothermie-ademe`,
//! `maison-pac-sol-eau-ademe`, `maison-pac-nappe-reversible-ademe` et
//! `maison-pac-boucle-tour-ademe`, sur la météo ouverte ADEME (CI publique) et sur la météo
//! réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test generation_eau_sol -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les tests vérifient des propriétés (bilans, sens
//! des effets des paramètres des sources, auxiliaires amont) et les refus ; les calculs à la
//! main sont dans `chauffage_refroidissement/sources_amont.rs` et
//! `chauffage_refroidissement/thermodynamique_elec.rs`.

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

const GEOTHERMIE: &str = "maison-pac-geothermie-ademe";
const SOL: &str = "maison-pac-sol-eau-ademe";
const NAPPE: &str = "maison-pac-nappe-reversible-ademe";
const BOUCLE_TOUR: &str = "maison-pac-boucle-tour-ademe";

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

/// Premier générateur de la première génération.
fn pac(r: &Resultats) -> &ResultatsGenerateur {
    &r.batiments[0].generations[0].generateurs[0]
}

/// Source amont du premier générateur, modifiable.
fn source(v: &mut serde_json::Value) -> &mut serde_json::Value {
    &mut v["batiments"][0]["generations"][0]["generateurs"][0]["nature"]
        ["thermodynamique_electrique"]["source_amont"]
}

/// Bilans : chaque générateur fournit la demande de son poste, la génération consomme ce que
/// ses générateurs consomment ; signe de l'énergie échangée avec la source amont.
#[test]
fn bilans_des_exemples_ademe() {
    for nom in [GEOTHERMIE, SOL, NAPPE, BOUCLE_TOUR] {
        let r = calcul(&json_du_cas(nom), &meteo_ademe);
        let g = &r.batiments[0].generations[0];
        let somme: f64 = g
            .generateurs
            .iter()
            .map(|x| x.consommations.total_kwh())
            .sum();
        assert!(
            (g.consommations.annuel.total_kwh() - somme).abs() < 1e-6,
            "{nom}"
        );
        assert!((g.q_fou_ch_kwh - g.q_ch_kwh).abs() < 1.0, "{nom}");
        assert!(g.q_fou_ch_kwh > 0.0, "{nom}");
        for x in g.generateurs.iter().filter(|x| x.id != "appoint-electrique") {
            // Éq. (1332) à (1334) : rejet négatif en chauffage, positif en refroidissement.
            if x.q_fou_fr_kwh == 0.0 {
                assert!(x.phi_rejet_kwh < 0.0, "{nom} : {}", x.id);
            }
            if x.q_fou_ch_kwh == 0.0 {
                assert!(x.phi_rejet_kwh > x.q_fou_fr_kwh, "{nom} : {}", x.id);
            }
        }
    }
}

/// Sondes (éq. 1408 à 1410, 1427, 1445) : un sol plus chaud fait baisser la consommation ;
/// un captage plus court (rejet plus pénalisant, `Rb/L`) la fait monter ; les pompes à vitesse
/// variable consomment entre 0,3 et 1 fois leur puissance pendant les heures de marche.
#[test]
fn sondes_sens_des_effets() {
    let base = calcul(&json_du_cas(GEOTHERMIE), &meteo_ademe);
    let c0 = pac(&base).consommations.total_kwh();
    let mut v = json_du_cas(GEOTHERMIE);
    source(&mut v)["captage"]["sol"]["theta_min_c"] = 13.0.into();
    source(&mut v)["captage"]["sol"]["theta_max_c"] = 18.0.into();
    assert!(pac(&calcul(&v, &meteo_ademe)).consommations.total_kwh() < c0);
    let mut v = json_du_cas(GEOTHERMIE);
    source(&mut v)["captage"]["l_m"] = 20.0.into();
    assert!(pac(&calcul(&v, &meteo_ademe)).consommations.total_kwh() > c0);
    // Pompes : sans elles, les auxiliaires baissent d'au plus 60 W pendant les heures de marche.
    let mut v = json_du_cas(GEOTHERMIE);
    source(&mut v)["captage"]["pompes"]["p_pompes_w"] = 0.0.into();
    let sans = calcul(&v, &meteo_ademe);
    let pompes = pac(&base).w_aux_kwh - pac(&sans).w_aux_kwh;
    assert!(pompes > 0.0 && pompes <= 0.060 * 8760.0, "{pompes}");
    // Les écarts par défaut (5 K) sont signalés.
    assert!(pac(&base)
        .valeurs_par_defaut
        .iter()
        .any(|d| d.champ == "source_amont.captage.ecarts.delta_theta_ch_k" && d.valeur == 5.0));
}

/// Sol (fiche 8.27) : pas d'auxiliaires amont (éq. 1465), un sol plus chaud fait baisser la
/// consommation jusqu'au plafond du tableau 245 (10 °C, bain d'essai à 6,5 °C).
#[test]
fn sol_sens_des_effets() {
    let base = calcul(&json_du_cas(SOL), &meteo_ademe);
    let c0 = pac(&base).consommations.total_kwh();
    let mut v = json_du_cas(SOL);
    source(&mut v)["sol"]["sol"]["theta_min_c"] = 8.0.into();
    let chaud = calcul(&v, &meteo_ademe);
    assert!(pac(&chaud).consommations.total_kwh() < c0);
    // Au-delà de 10 °C de sol corrigé, la température de bain reste 6,5 °C : deux sols très
    // chauds (Rb nul) donnent le même résultat.
    let mut v = json_du_cas(SOL);
    source(&mut v)["sol"]["rb_k_m_w"] = 0.0.into();
    source(&mut v)["sol"]["sol"]["theta_min_c"] = 20.0.into();
    source(&mut v)["sol"]["sol"]["theta_max_c"] = 20.0.into();
    let a = pac(&calcul(&v, &meteo_ademe)).consommations.total_kwh();
    source(&mut v)["sol"]["sol"]["theta_min_c"] = 30.0.into();
    source(&mut v)["sol"]["sol"]["theta_max_c"] = 30.0.into();
    let b = pac(&calcul(&v, &meteo_ademe)).consommations.total_kwh();
    assert_eq!(a, b);
}

/// Nappe avec échangeur de barrage (éq. 1411 à 1424, 1444 et 1446) : les pompes en marche
/// permanente consomment leur puissance à chaque heure de la saison, davantage qu'à la
/// demande ; sans échangeur, la machine voit directement la nappe (éq. 1425).
#[test]
fn nappe_pompes_et_echangeur() {
    let base = calcul(&json_du_cas(NAPPE), &meteo_ademe);
    let mut v = json_du_cas(NAPPE);
    source(&mut v)["nappe"]["pompes"]["gestion"] = "demande_tout_ou_rien".into();
    let demande = calcul(&v, &meteo_ademe);
    assert!(pac(&demande).w_aux_kwh < pac(&base).w_aux_kwh);
    // Sans pompes, l'écart des auxiliaires est celui des 60 W permanents : un multiple entier
    // d'heures de saison, au plus 8 760.
    let mut v = json_du_cas(NAPPE);
    source(&mut v)["nappe"]["pompes"]["p_pompes_w"] = 0.0.into();
    source(&mut v)["nappe"]["echangeur"]["p_pompes_inter_w"] = 0.0.into();
    let sans = calcul(&v, &meteo_ademe);
    let heures = (pac(&base).w_aux_kwh - pac(&sans).w_aux_kwh) / 0.060;
    assert!((heures - heures.round()).abs() < 1e-6 && heures <= 8760.0, "{heures}");
    // Sans échangeur : la source n'est plus atténuée par l'échangeur ni par les rejets.
    let mut v = json_du_cas(NAPPE);
    source(&mut v)["nappe"]
        .as_object_mut()
        .unwrap()
        .remove("echangeur");
    let directe = calcul(&v, &meteo_ademe);
    assert_ne!(
        pac(&directe).consommations.total_kwh(),
        pac(&base).consommations.total_kwh()
    );
}

/// Tour de refroidissement (éq. 1428 à 1436, 1449 à 1451) : une tour humide refroidit l'eau
/// sous la température sèche, le refroidisseur consomme moins qu'avec une tour sèche ; les
/// auxiliaires de la tour sont proportionnels à la charge.
#[test]
fn tour_humide_et_seche() {
    let base = calcul(&json_du_cas(BOUCLE_TOUR), &meteo_ademe);
    let froid = |r: &Resultats| r.batiments[0].generations[0].generateurs[1].clone();
    let mut v = json_du_cas(BOUCLE_TOUR);
    v["batiments"][0]["generations"][0]["generateurs"][1]["nature"]["thermodynamique_electrique"]
        ["source_amont"]["tour_de_refroidissement"]["nature"] = "seche".into();
    let seche = calcul(&v, &meteo_ademe);
    assert!(
        froid(&base).consommations.total_kwh() < froid(&seche).consommations.total_kwh()
    );
    assert!(froid(&base).q_fou_fr_kwh > 0.0);
    // ΔθCond_FR par défaut signalé.
    assert!(froid(&base)
        .valeurs_par_defaut
        .iter()
        .any(|d| d.champ == "source_amont.tour_de_refroidissement.delta_theta_cond_fr_k"));
}

/// Refus et erreurs de saisie.
#[test]
fn refus_et_erreurs() {
    let erreurs = |v: &serde_json::Value| thbce_core::valider(&projet(v));
    // Triple service : encore refusé (code 3).
    let mut v = json_du_cas(GEOTHERMIE);
    let t = &mut v["batiments"][0]["generations"][0]["generateurs"][0]["nature"]
        ["thermodynamique_electrique"];
    t["refroidissement"] = serde_json::json!({
        "technologie": "eau_eau",
        "performances": {"pivot_defaut": {"pabs_kw": 1.0}},
        "typo_emetteur": "plancher_ou_plafond_inertie_forte"
    });
    t["ecs"] = serde_json::json!({
        "technologie": "eau_glycolee_eau",
        "performances": {"pivot_defaut": {"pabs_kw": 1.0}}
    });
    let r = erreurs(&v);
    assert!(r
        .erreurs
        .iter()
        .any(|e| e.hors_perimetre && e.message.contains("triple service")));
    // Air extrait / air neuf : refusé (code 3).
    let mut v = json_du_cas(NAPPE);
    v["batiments"][0]["generations"][0]["generateurs"][0]["nature"]["thermodynamique_electrique"]
        ["chauffage"]["technologie"] = "air_extrait_air_neuf".into();
    assert!(erreurs(&v).erreurs.iter().any(|e| e.hors_perimetre));
    // Source incohérente avec la technologie (lecture Q145).
    let mut v = json_du_cas(GEOTHERMIE);
    *source(&mut v) = serde_json::json!({"sol": {
        "sol": {"theta_min_c": 5.0, "theta_max_c": 10.0, "mois_minimum": 2},
        "rb_k_m_w": 0.1, "l_m": 100.0}});
    let r = erreurs(&v);
    assert!(!r.est_valide() && !r.seulement_hors_perimetre());
    assert!(r.erreurs.iter().any(|e| e.message.contains("incohérente")));
    // Tour de refroidissement pour un générateur qui chauffe.
    let mut v = json_du_cas(NAPPE);
    *source(&mut v) = serde_json::json!({"tour_de_refroidissement": {
        "nature": "seche", "delta_theta_tour_k": 5.0, "theta_es_consigne_c": 20.0,
        "p_pompes_tour_w": 10.0, "p_vent_tour_w": 10.0}});
    assert!(erreurs(&v)
        .erreurs
        .iter()
        .any(|e| e.message.contains("froid seul")));
    // Mois et longueur hors plage.
    let mut v = json_du_cas(GEOTHERMIE);
    source(&mut v)["captage"]["sol"]["mois_minimum"] = 13.into();
    source(&mut v)["captage"]["l_m"] = 0.0.into();
    let r = erreurs(&v);
    assert!(r.erreurs.iter().any(|e| e.chemin.ends_with("mois_minimum")));
    assert!(r.erreurs.iter().any(|e| e.chemin.ends_with("l_m")));
    // Combinaison absente du tableau 146.
    let mut v = json_du_cas(NAPPE);
    v["batiments"][0]["generations"][0]["generateurs"][0]["nature"]["thermodynamique_electrique"]
        ["refroidissement"]["technologie"] = "eau_de_nappe_eau".into();
    assert!(erreurs(&v)
        .erreurs
        .iter()
        .any(|e| e.message.contains("tableau 146")));
    // Chauffe-eau thermodynamique sur eau : erreur de saisie.
    let mut v = json_du_cas(GEOTHERMIE);
    v["batiments"][0]["generations"][1]["chauffe_eau_thermodynamiques"][0]["source_amont"] =
        serde_json::json!({"nappe": {
            "nappe": {"theta_min_c": 10.0, "theta_max_c": 12.0, "mois_minimum": 3},
            "pompes": {"gestion": "marche_permanente", "p_pompes_w": 10.0}}});
    let r = erreurs(&v);
    assert!(!r.est_valide() && !r.seulement_hors_perimetre());
}

/// Valeurs phares sur la météo réglementaire (Cep, Cep,nr, Ic énergie, DH ; arrondis à 0,1).
#[test]
#[ignore = "requiert la météo réglementaire (THBCE_METEO)"]
fn exemples_meteo_reglementaire() {
    let mut ecarts = Vec::new();
    for (nom, attendu) in [
        (GEOTHERMIE, [53.6, 53.6, 64.6, 1030.9]),
        (SOL, [59.3, 59.3, 72.3, 1030.9]),
        (NAPPE, [103.0, 103.0, 123.0, 1042.9]),
        (BOUCLE_TOUR, [104.4, 104.4, 125.7, 1042.9]),
    ] {
        let r = calcul(&json_du_cas(nom), &meteo_reglementaire);
        let generateur = pac(&r);
        println!(
            "  {} : Qfou_ch {:.1} kWh, Qfou_fr {:.1} kWh, ηeff_ch {:?}, Waux {:.1} kWh",
            generateur.id,
            generateur.q_fou_ch_kwh,
            generateur.q_fou_fr_kwh,
            generateur.eta_eff_ch_an,
            generateur.w_aux_kwh
        );
        let b = &r.batiments[0];
        let i = b.indicateurs.as_ref().unwrap();
        let obtenu = [
            b.cep_kwhep_m2.unwrap(),
            b.cep_nr_kwhep_m2.unwrap(),
            b.ic_energie_kgco2_m2.unwrap(),
            b.dh_c_h.unwrap(),
        ];
        println!(
            "{nom} : Cep {:.1}, Cep,nr {:.1}, Ic énergie {:.1}, DH {:.1}, conforme {:?}",
            obtenu[0], obtenu[1], obtenu[2], obtenu[3], i.exigences.as_ref().map(|e| e.conforme)
        );
        ecarts.extend(
            obtenu
                .iter()
                .zip(attendu)
                .filter(|(o, a)| (*o - a).abs() > 0.06)
                .map(|(o, a)| format!("{nom} : {o} au lieu de {a}")),
        );
    }
    assert!(ecarts.is_empty(), "{ecarts:#?}");
}
