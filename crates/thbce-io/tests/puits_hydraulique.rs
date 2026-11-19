// SPDX-License-Identifier: GPL-3.0-or-later
//! Puits hydraulique (fiche 6.9 et prétraitement du §6.3.3.3.4, lot 1-ven9) de bout en bout :
//! maison double flux (`tests/cas/maison-puits-hydraulique-ademe`) et centrale de traitement
//! d'air (`tests/cas/bureaux-cta-puits-hydraulique-ademe`), sur la météo ouverte ADEME (CI
//! publique) et sur la météo réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test puits_hydraulique -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les propriétés vérifiées sont physiques (sens des
//! effets) ou des égalités du texte (consommation de la pompe, éq. 748 et 749).

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{GestionPuitsHydraulique, Projet, Puits};
use thbce_core::resultats::{Resultats, ResultatsPuitsHydraulique};
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

fn aux_vent(r: &Resultats) -> f64 {
    r.batiments[0].zones[0]
        .ventilation
        .as_ref()
        .unwrap()
        .c_aux_vent_kwh
}

/// Fonctionnement du puits en Th-C et en Th-D.
fn puits(r: &Resultats) -> (ResultatsPuitsHydraulique, ResultatsPuitsHydraulique) {
    let v = r.batiments[0].zones[0].ventilation.as_ref().unwrap();
    (
        v.th_c.as_ref().unwrap().puits_hydraulique.unwrap(),
        v.th_d.as_ref().unwrap().puits_hydraulique.unwrap(),
    )
}

fn p_max_w(p: &Projet) -> f64 {
    p.batiments[0].zones[0]
        .ventilation
        .as_ref()
        .unwrap()
        .puits_hydraulique
        .unwrap()
        .p_max_w
}

fn verifier(nom: &str, reference: &str, m: &MeteoProjet, titre: &str, aux_exacts: bool) {
    let p = cas(nom);
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);
    let r = brut(&p, m);
    let sans = brut(&cas(reference), m);
    // Th-B : pas de puits dans le Bbio.
    assert_eq!(bbio(&r), bbio(&sans), "{titre} {nom}");
    let (c, d) = puits(&r);
    for x in [c, d] {
        assert!(x.nbh_actif > 0 && x.nbh_actif <= 8760, "{titre} {nom}");
        assert!(x.q_prechauffage_kwh > 0.0, "{titre} {nom}");
        assert!(x.q_prerefroidissement_kwh < 0.0, "{titre} {nom}");
        // Éq. (748) : la pompe consomme sa puissance nominale pendant chaque heure de marche
        // (un seul système sur le puits, éq. 749 : la totalité).
        assert!(
            (x.w_pompe_kwh - p_max_w(&p) * f64::from(x.nbh_actif) / 1000.0).abs() < 1e-9,
            "{titre} {nom}"
        );
    }
    // La pompe est comptée dans les auxiliaires de ventilation (Th-C). Avec une centrale de
    // traitement d'air, l'air neuf change aussi le bypass et le fonctionnement de l'échangeur :
    // la différence n'est alors qu'à peu près la consommation de la pompe.
    let ecart = aux_vent(&r) - aux_vent(&sans) - c.w_pompe_kwh;
    if aux_exacts {
        assert!(ecart.abs() < 1e-6, "{titre} {nom} : {ecart}");
    } else {
        assert!(
            ecart.abs() < 0.05 * c.w_pompe_kwh,
            "{titre} {nom} : {ecart}"
        );
    }
    println!(
        "{titre} : {nom}, Bbio {:.1}, DH {:.1} (sans puits {:.1}), Cep {:.1} ({:.1}), Th-C {} h, {:.1}/{:.1} kWh, pompe {:.1} kWh, Th-D {} h",
        bbio(&r),
        dh(&r),
        dh(&sans),
        cep(&r),
        cep(&sans),
        c.nbh_actif,
        c.q_prechauffage_kwh,
        c.q_prerefroidissement_kwh,
        c.w_pompe_kwh,
        d.nbh_actif
    );
}

#[test]
fn puits_hydraulique_sur_la_meteo_ademe() {
    let m = meteo_ademe(&cas("maison-puits-hydraulique-ademe"));
    verifier(
        "maison-puits-hydraulique-ademe",
        "maison-double-flux-ademe",
        &m,
        "ADEME",
        true,
    );
    verifier(
        "bureaux-cta-puits-hydraulique-ademe",
        "bureaux-cta-ademe",
        &m,
        "ADEME",
        false,
    );
    // Air neuf préchauffé et prérafraîchi : moins d'inconfort d'été dans la maison.
    let r = brut(&cas("maison-puits-hydraulique-ademe"), &m);
    let sans = brut(&cas("maison-double-flux-ademe"), &m);
    assert!(dh(&r) < dh(&sans));
    // La centrale voit l'air neuf de la batterie : le préchauffage demande moins d'énergie
    // (éq. 521 à 525, `θsouf2` plus chaude).
    let r = brut(&cas("bureaux-cta-puits-hydraulique-ademe"), &m);
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
fn gestion_sur_les_deux_temperatures_et_batterie_sans_effet() {
    let m = meteo_ademe(&cas("maison-puits-hydraulique-ademe"));
    let nom = "maison-puits-hydraulique-ademe";
    let heures = |p: &Projet| puits(&brut(p, &m)).0.nbh_actif;
    // Gestion sur la seule température extérieure : la pompe tourne plus souvent que sur les
    // deux températures (éq. 746 et 747 : la seconde condition ne fait que retirer des heures).
    let mut p = cas(nom);
    let v = p.batiments[0].zones[0].ventilation.as_mut().unwrap();
    assert_eq!(
        v.puits_hydraulique.unwrap().gestion,
        GestionPuitsHydraulique::TemperaturesExterieureEtEau
    );
    let h_deux = heures(&p);
    let v = p.batiments[0].zones[0].ventilation.as_mut().unwrap();
    v.puits_hydraulique.as_mut().unwrap().gestion = GestionPuitsHydraulique::TemperatureExterieure;
    assert!(heures(&p) > h_deux);
    // Une batterie d'efficacité quasi nulle (puissances de 10⁻⁹ kW) laisse l'air neuf à `θext` :
    // le DH est celui de l'exemple sans puits.
    let v = p.batiments[0].zones[0].ventilation.as_mut().unwrap();
    v.puits_hydraulique.as_mut().unwrap().batterie =
        Some(thbce_core::entree::BatteriePuitsHydraulique {
            p_ch_kw: 1e-9,
            p_fr_kw: 1e-9,
        });
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);
    let r = brut(&p, &m);
    let sans = brut(&cas("maison-double-flux-ademe"), &m);
    assert!((dh(&r) - dh(&sans)).abs() < 1e-6);
}

#[test]
fn refus_et_erreurs_de_saisie() {
    // Puits climatique et hydraulique ensemble : erreur de saisie (éq. 386).
    let mut p = cas("maison-puits-hydraulique-ademe");
    let v = p.batiments[0].zones[0].ventilation.as_mut().unwrap();
    v.puits_climatique = cas("maison-puits-climatique-ademe").batiments[0].zones[0]
        .ventilation
        .as_ref()
        .unwrap()
        .puits_climatique;
    let r = valider(&p);
    assert!(!r.est_valide() && !r.seulement_hors_perimetre());
    // Puissance de batterie supérieure à ce que le débit admet : erreur de saisie.
    let mut p = cas("maison-puits-hydraulique-ademe");
    let v = p.batiments[0].zones[0].ventilation.as_mut().unwrap();
    v.puits_hydraulique.as_mut().unwrap().batterie =
        Some(thbce_core::entree::BatteriePuitsHydraulique {
            p_ch_kw: 5.0,
            p_fr_kw: 0.7,
        });
    let r = valider(&p);
    assert!(!r.est_valide() && !r.seulement_hors_perimetre(), "{r:?}");
    assert!(r.erreurs[0].message.contains("efficacité > 1"));
    // Puits hydraulique sans description.
    let mut p = cas("maison-puits-hydraulique-ademe");
    p.batiments[0].zones[0]
        .ventilation
        .as_mut()
        .unwrap()
        .puits_hydraulique = None;
    assert!(!valider(&p).est_valide());
    assert_eq!(
        p.batiments[0].zones[0].ventilation.as_ref().unwrap().puits,
        Some(Puits::Hydraulique)
    );
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn puits_hydraulique_sur_la_meteo_reglementaire() {
    let m = meteo_reglementaire(&cas("maison-puits-hydraulique-ademe"));
    // Valeurs phares des deux exemples (nombres agrégés annuels, calculés par le moteur le
    // 19/11/2026, non comparés à un logiciel de référence) : Bbio, Cep, DH.
    for (nom, attendus) in [
        ("maison-puits-hydraulique-ademe", [52.7, 127.5, 657.9]),
        ("bureaux-cta-puits-hydraulique-ademe", [82.9, 72.9, 180.2]),
    ] {
        let r = brut(&cas(nom), &m);
        println!("{nom} : {:.1} {:.1} {:.1}", bbio(&r), cep(&r), dh(&r));
        for (obtenu, attendu) in [bbio(&r), cep(&r), dh(&r)].into_iter().zip(attendus) {
            assert!(
                (obtenu - attendu).abs() < 0.05,
                "{nom} : {obtenu} au lieu de {attendu}"
            );
        }
    }
    verifier(
        "maison-puits-hydraulique-ademe",
        "maison-double-flux-ademe",
        &m,
        "réglementaire",
        true,
    );
    verifier(
        "bureaux-cta-puits-hydraulique-ademe",
        "bureaux-cta-ademe",
        &m,
        "réglementaire",
        false,
    );
}
