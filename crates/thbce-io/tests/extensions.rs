// SPDX-License-Identifier: GPL-3.0-or-later
//! Extensions et identification de l'usage (lot 0d, fiches 4.2 à 4.4 ; Q85 à Q88) de bout en
//! bout, sur l'exemple `tests/cas/extension-maison-ademe` : météo ouverte ADEME (CI publique)
//! et météo réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test extensions -- --ignored --nocapture
//! ```

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{ExtensionSaisie, Projet, UsageZone};
use thbce_core::indicateurs::resultats::{
    FamilleUsage, NatureTravaux, RegimeExigences, ResultatsExigences,
};
use thbce_core::{calculer, Options};
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

fn exigences(
    p: &Projet,
    meteo_de: &dyn Fn(&Projet) -> MeteoProjet,
) -> (f64, f64, ResultatsExigences) {
    let r = thbce_core::valider(p);
    assert!(r.est_valide(), "{:?}", r.erreurs);
    let res = calculer(p, &meteo_de(p), &Options { arrondir: false }).unwrap();
    let b = &res.batiments[0];
    let e = b.indicateurs.as_ref().unwrap().exigences.clone().unwrap();
    (b.bbio_pts.unwrap(), b.dh_c_h.unwrap(), e)
}

fn proche(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} ≠ {b}");
}

/// Extension de maison de 60 m² : exigences de l'article R. 172-4 calculées sur la `Sref` de
/// l'extension (Q87) et régime intermédiaire (Bbio et DH, article 50-3, II ; Q86, Q88).
fn verifier_extension(meteo_de: &dyn Fn(&Projet) -> MeteoProjet) -> (f64, f64) {
    let p = cas("extension-maison-ademe");
    let (bbio, dh, e) = exigences(&p, meteo_de);
    proche(e.bbio_max_pts, 82.6);
    proche(e.cep_max_kwhep_m2, 97.5);
    proche(e.cep_nr_max_kwhep_m2, 71.5);
    proche(e.ic_energie_max_kgco2_m2, 208.0);
    let a = e.regime_alternatif.clone().unwrap();
    assert_eq!(a.nature, NatureTravaux::Extension);
    assert_eq!(a.famille, FamilleUsage::MaisonIndividuelle);
    assert_eq!(a.regime, RegimeExigences::Intermediaire);
    proche(a.sref_consideree_m2, 60.0);
    let noms: Vec<&str> = a
        .verifications
        .iter()
        .map(|v| v.indicateur.as_str())
        .collect();
    assert_eq!(noms, ["bbio", "dh"]);
    proche(a.verifications[0].valeur, bbio);
    proche(a.verifications[1].valeur, dh);
    proche(a.verifications[1].maximum, 1250.0);
    assert_eq!(a.conforme, Some(bbio <= 82.6 && dh <= 1250.0));

    // Avec 30 m² d'autres extensions du même bâtiment : 90 m² cumulés, régime complet ; les
    // exigences restent celles de l'extension décrite.
    let mut p2 = p.clone();
    let x = p2.batiments[0].exigences.as_mut().unwrap();
    x.extension = Some(ExtensionSaisie {
        surelevation: false,
        sref_existant_m2: None,
        sref_autres_extensions_m2: 30.0,
    });
    let (_, _, e2) = exigences(&p2, meteo_de);
    let a2 = e2.regime_alternatif.unwrap();
    assert_eq!(a2.regime, RegimeExigences::Complet);
    proche(a2.sref_consideree_m2, 90.0);
    assert_eq!(a2.conforme, Some(e2.conforme));
    assert!(a2.verifications.is_empty());
    proche(e2.bbio_max_pts, 82.6);

    // Sans `extension` : construction neuve de 60 m², pas de régime alternatif.
    let mut p3 = p.clone();
    p3.batiments[0].exigences.as_mut().unwrap().extension = None;
    let (_, _, e3) = exigences(&p3, meteo_de);
    assert!(e3.regime_alternatif.is_none());
    (bbio, dh)
}

#[test]
fn extension_de_maison_sur_meteo_ademe() {
    let (bbio, dh) = verifier_extension(&meteo_ademe);
    println!("ADEME : Bbio {bbio:.1}, DH {dh:.1}");
}

#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn extension_de_maison_sur_meteo_reglementaire() {
    let (bbio, dh) = verifier_extension(&meteo_reglementaire);
    println!("réglementaire : Bbio {bbio:.1}, DH {dh:.1}");
    let p = cas("extension-maison-ademe");
    let res = calculer(&p, &meteo_reglementaire(&p), &Options::default()).unwrap();
    let b = &res.batiments[0];
    println!(
        "réglementaire arrondi : {:?} {:?} {:?} {:?} {:?}",
        b.bbio_pts, b.dh_c_h, b.cep_kwhep_m2, b.cep_nr_kwhep_m2, b.ic_energie_kgco2_m2
    );
    // Valeurs phares (annuelles) sur la météo réglementaire, arrondies à une décimale.
    assert_eq!(b.bbio_pts, Some(BBIO_REGL));
    assert_eq!(b.dh_c_h, Some(DH_REGL));
    assert_eq!(b.cep_kwhep_m2, Some(CEP_REGL));
    assert_eq!(b.cep_nr_kwhep_m2, Some(CEP_NR_REGL));
    assert_eq!(b.ic_energie_kgco2_m2, Some(IC_REGL));
    let e = b.indicateurs.as_ref().unwrap().exigences.as_ref().unwrap();
    assert!(!e.conforme);
    assert_eq!(e.regime_alternatif.as_ref().unwrap().conforme, Some(true));
}

// Exigences : régime intermédiaire respecté (Bbio 46,7 ≤ 82,6 ; DH 1 030,9 ≤ 1 250) ; régime
// complet non respecté (Cep 131,4 > 97,5 ; Cep,nr 131,4 > 71,5).
const BBIO_REGL: f64 = 46.7;
const DH_REGL: f64 = 1030.9;
const CEP_REGL: f64 = 131.4;
const CEP_NR_REGL: f64 = 131.4;
const IC_REGL: f64 = 160.6;

/// Surélévation climatisée de maison de 75 m² en H2d (Vaucluse), à 450 m, permis de 2025 :
/// régime intermédiaire, `DH_max` du groupe climatisé porté à 1 400 °C.h (article 50-3, II ;
/// Q88). Recalculs : classe d'altitude 400-800 m, `Mbgéo` = 0,1, `Mcgéo` = 0 ;
/// `Mbsurf_moy` = (49 − 0,49 × 75)/63 = 12,25/63 : `Bbio_max` = 63 × 1,1 + 12,25 = 81,55 ;
/// `Mcsurf_moy` = (49,5 − 0,55 × 75)/55 = 0,15 : `Cep_max` = 86,25, `Cep,nr_max` = 63,25,
/// `Icénergie_max` = 184. Restitution arrondie : 63,25 → 63,3 (arrondi robuste).
#[test]
fn surelevation_climatisee_en_h2d() {
    let mut p = cas("maison-pac-air-air-ademe");
    p.localisation.departement = thbce_data::Departement::nouveau("84").unwrap();
    p.localisation.alt_m = 450.0;
    p.batiments[0].zones[0].groupes[0].shab_m2 = Some(75.0);
    p.batiments[0].zones[0].groupes[0].su_m2 = Some(75.0);
    p.batiments[0].exigences.as_mut().unwrap().extension = Some(ExtensionSaisie {
        surelevation: true,
        sref_existant_m2: None,
        sref_autres_extensions_m2: 0.0,
    });
    let (_, dh, e) = exigences(&p, &meteo_ademe);
    let proche3 = |a: f64, b: f64| assert!((a - b).abs() < 1e-9, "{a} ≠ {b}");
    proche3(e.bbio_max_pts, 81.55);
    proche3(e.cep_max_kwhep_m2, 86.25);
    proche3(e.cep_nr_max_kwhep_m2, 63.25);
    proche3(e.ic_energie_max_kgco2_m2, 184.0);
    // Régime complet : DH_max 1 250 ; régime intermédiaire : 1 400.
    let dh_complet = e
        .verifications
        .iter()
        .find(|v| v.indicateur == "dh")
        .unwrap();
    proche3(dh_complet.maximum, 1250.0);
    let a = e.regime_alternatif.clone().unwrap();
    assert_eq!(a.nature, NatureTravaux::Surelevation);
    assert_eq!(a.regime, RegimeExigences::Intermediaire);
    let v = a
        .verifications
        .iter()
        .find(|v| v.indicateur == "dh")
        .unwrap();
    proche3(v.maximum, 1400.0);
    proche3(v.valeur, dh);
    // Restitution arrondie.
    let res = calculer(&p, &meteo_ademe(&p), &Options::default()).unwrap();
    let e = res.batiments[0]
        .indicateurs
        .as_ref()
        .unwrap()
        .exigences
        .as_ref()
        .unwrap();
    assert_eq!(e.cep_nr_max_kwhep_m2, 63.3);
    assert_eq!(e.bbio_max_pts, 81.6);
    assert_eq!(e.cep_max_kwhep_m2, 86.3);
}

fn erreurs(p: &Projet) -> Vec<String> {
    thbce_core::valider(p)
        .erreurs
        .iter()
        .map(|e| format!("{} : {}", e.chemin, e.message))
        .collect()
}

fn avertissements(p: &Projet) -> Vec<String> {
    thbce_core::valider(p)
        .avertissements
        .iter()
        .map(|e| format!("{} : {}", e.chemin, e.message))
        .collect()
}

#[test]
fn validation_de_l_extension() {
    // Maison : la surface existante est sans effet, facultative.
    let p = cas("extension-maison-ademe");
    assert!(erreurs(&p).is_empty());
    // Logements collectifs : obligatoire.
    let mut p = cas("immeuble-collectif-ademe");
    p.batiments[0].exigences.as_mut().unwrap().extension = Some(ExtensionSaisie {
        surelevation: false,
        sref_existant_m2: None,
        sref_autres_extensions_m2: 0.0,
    });
    let e = erreurs(&p);
    assert!(
        e.iter()
            .any(|m| m.contains("extension.sref_existant_m2") && m.contains("obligatoire")),
        "{e:?}"
    );
    let x = p.batiments[0].exigences.as_mut().unwrap();
    x.extension = Some(ExtensionSaisie {
        surelevation: true,
        sref_existant_m2: Some(0.0),
        sref_autres_extensions_m2: -5.0,
    });
    let e = erreurs(&p);
    assert!(e.iter().any(|m| m.contains("sref_existant_m2")), "{e:?}");
    assert!(
        e.iter().any(|m| m.contains("sref_autres_extensions_m2")),
        "{e:?}"
    );
    p.batiments[0]
        .exigences
        .as_mut()
        .unwrap()
        .extension
        .as_mut()
        .unwrap()
        .sref_existant_m2 = Some(2000.0);
    p.batiments[0]
        .exigences
        .as_mut()
        .unwrap()
        .extension
        .as_mut()
        .unwrap()
        .sref_autres_extensions_m2 = 0.0;
    assert!(erreurs(&p).is_empty(), "{:?}", erreurs(&p));
    // Valeurs extrêmes refusées (elles rendraient la part de l'existant non finie).
    for (existant, autres) in [(1e308, 0.0), (1e-300, 0.0), (2000.0, 1e308)] {
        let x = p.batiments[0]
            .exigences
            .as_mut()
            .unwrap()
            .extension
            .as_mut()
            .unwrap();
        x.sref_existant_m2 = Some(existant);
        x.sref_autres_extensions_m2 = autres;
        assert!(!erreurs(&p).is_empty(), "{existant} {autres}");
    }
}

/// Fiches 4.2 et 4.3 (Q85) : avertissements seulement.
#[test]
fn controles_des_fiches_4_2_et_4_3() {
    // Les exemples n'en déclenchent aucun.
    for nom in [
        "immeuble-collectif-ademe",
        "maison-climat-ademe",
        "extension-maison-ademe",
    ] {
        assert!(avertissements(&cas(nom)).is_empty(), "{nom}");
    }
    // Immeuble de deux logements décrit en logements collectifs.
    let mut p = cas("immeuble-collectif-ademe");
    p.batiments[0].zones[0].nb_logements = Some(2);
    let a = avertissements(&p);
    assert!(
        a.iter()
            .any(|m| m.contains("strictement plus de deux logements")),
        "{a:?}"
    );
    // Sauf pour une extension (usage de l'existant).
    p.batiments[0].exigences.as_mut().unwrap().extension = Some(ExtensionSaisie {
        surelevation: false,
        sref_existant_m2: Some(1500.0),
        sref_autres_extensions_m2: 0.0,
    });
    assert!(avertissements(&p).is_empty(), "{:?}", avertissements(&p));

    // Maison et logements collectifs dans le même bâtiment.
    let mut p = cas("immeuble-collectif-ademe");
    let mut z = cas("maison-climat-ademe").batiments[0].zones[0].clone();
    z.id = "maison-accolee".into();
    p.batiments[0].zones.push(z.clone());
    let a = avertissements(&p);
    assert!(
        a.iter()
            .any(|m| m.contains("soit collectif, soit de maisons")),
        "{a:?}"
    );

    // Deux zones de maison individuelle.
    let mut p = cas("maison-climat-ademe");
    p.batiments[0].zones.push(z.clone());
    let a = avertissements(&p);
    assert!(
        a.iter().any(|m| m.contains("deuxième zone du même usage")),
        "{a:?}"
    );

    // Maison dans un bâtiment de bureaux plus grand (usage principal non résidentiel) ; vestiaires
    // seuls dans un établissement sportif. Les usages non calculés sont refusés (code 3), mais
    // la validation lève aussi ces avertissements.
    let mut p = cas("maison-climat-ademe");
    let mut bureaux = z.clone();
    bureaux.id = "bureaux".into();
    bureaux.usage = UsageZone::Bureaux;
    bureaux.groupes[0].su_m2 = Some(500.0);
    p.batiments[0].zones.push(bureaux.clone());
    let a = avertissements(&p);
    assert!(
        a.iter().any(|m| m.contains("logement de fonction")),
        "{a:?}"
    );
    let mut p = cas("maison-climat-ademe");
    let mut v = bureaux.clone();
    v.usage = UsageZone::Vestiaires;
    let mut s = bureaux;
    s.id = "salle".into();
    s.usage = UsageZone::SportPrive;
    p.batiments[0].zones = vec![v, s];
    let a = avertissements(&p);
    assert!(a.iter().any(|m| m.contains("vestiaires seuls")), "{a:?}");
}
