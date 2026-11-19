// SPDX-License-Identifier: GPL-3.0-or-later
//! Aération par ouverture des fenêtres (fiche 6.7, lot 1-ven5) de bout en bout : maison
//! (`tests/cas/maison-aeration-ademe`) et bureaux, sur la météo ouverte ADEME (CI publique) et
//! sur la météo réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test aeration -- --ignored --nocapture
//! ```
//!
//! Les débits et la consommation des ventilateurs sont calculés à la main ; les effets du
//! système sur la thermique (Bbio indépendant du système, DH, espace tampon) sont des propriétés
//! physiques, aucun cas de référence n'étant disponible.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{BoucheConduit, Isouf, Projet, SystemeVentilation};
use thbce_core::resultats::Resultats;
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

/// Même zone, avec une aération dont le débit d'hygiène à fournir est multiplié par `k`.
fn avec_debit(mut p: Projet, k: f64) -> Projet {
    for b in &mut p.batiments[0].zones[0].groupes[0].bouches_conduits {
        b.q_spec_conv_pointe_m3_h *= k;
        b.q_spec_conv_base_m3_h = b.q_spec_conv_base_m3_h.map(|q| q * k);
    }
    p
}

fn verifier_maison(m: &MeteoProjet, titre: &str) {
    let aer = cas("maison-aeration-ademe");
    let df = cas("maison-double-flux-ademe");
    let r = brut(&aer, m);
    let r_df = brut(&df, m);
    // Le Bbio n'emploie pas le système du projet : celui de la maison en double flux, aux mêmes
    // bouches (§6.1, décision Q36).
    assert_eq!(bbio(&r), bbio(&r_df), "{titre}");
    // Débits (calculés à la main, fiche 6.2) : (180 × 14 + 90 × 154)/168 = 97,5 m³/h ;
    // Cdep = Cfenb = 1,7 : 165,75 m³/h ; Kres = 0 : le débit vu par le ventilateur est le même ;
    // débit repris opposé au débit soufflé (éq. 163).
    let z = &r.batiments[0].zones[0];
    let v = z.ventilation.as_ref().unwrap();
    let g = z.groupes[0].ventilation.as_ref().unwrap();
    assert!((g.q_spec_souffle_m3_h - 165.75).abs() < 1e-9, "{titre}");
    assert!((g.q_spec_repris_m3_h + 165.75).abs() < 1e-9, "{titre}");
    assert!((g.q_souffle_cond_m3_h - 165.75).abs() < 1e-9, "{titre}");
    // Aucun ventilateur : puissance et consommation nulles.
    assert_eq!(
        (v.p_vent_w, v.c_aux_vent_kwh, g.c_aux_vent_kwh),
        (0.0, 0.0, 0.0)
    );
    // Seule la durée du grand débit est une valeur par défaut, une fois par bouche.
    let champs: Vec<_> = v
        .valeurs_par_defaut
        .iter()
        .map(|d| d.champ.as_str())
        .collect();
    assert_eq!(
        champs,
        [
            "groupes[0].bouches_conduits[0].dugd_h",
            "groupes[0].bouches_conduits[1].dugd_h"
        ],
        "{titre}"
    );
    // Sans récupération de chaleur sur l'air repris, ni les 85 % de l'échangeur, l'air neuf
    // coûte plus de chauffage et de confort d'été que celui du double flux.
    assert!(
        dh(&r) > dh(&r_df),
        "{titre} : DH {} / {}",
        dh(&r),
        dh(&r_df)
    );
    assert!(cep(&r) > cep(&r_df), "{titre}");
    // Plus l'air neuf fourni est abondant, plus le logement est froid l'hiver ; le DH, lui,
    // dépend de la température de l'air neuf, jamais plus chaude que le logement en pointe.
    let r_x2 = brut(&avec_debit(aer.clone(), 2.0), m);
    assert!(cep(&r_x2) > cep(&r), "{titre}");
    println!(
        "{titre} : Bbio {:.1} ; Cep {:.1} ({:.1} en double flux) ; DH {:.1} ({:.1} en double flux)",
        bbio(&r),
        cep(&r),
        cep(&r_df),
        dh(&r),
        dh(&r_df)
    );
}

/// L'air neuf transitant par un espace tampon (§6.7.3.1, éq. 715) : sur la maison à véranda
/// solarisée, l'air soufflé est à la température de sortie de la véranda au lieu de celle de
/// l'extérieur.
fn verifier_espace_tampon(m: &MeteoProjet, titre: &str) {
    let mut p = cas("maison-veranda-ademe");
    let z = &mut p.batiments[0].zones[0];
    let extraction = z.groupes[0].bouches_conduits[0].clone();
    z.groupes[0].bouches_conduits = vec![BoucheConduit {
        id: "fenetre".into(),
        isouf: Isouf::Soufflage,
        qvarep_spec_m3_h: None,
        q_spec_conv_base_m3_h: Some(90.0),
        dugd_h: None,
        ..extraction
    }];
    let ventilation = z.ventilation.as_mut().unwrap();
    ventilation.systeme = SystemeVentilation::Aeration;
    ventilation.reseau.r_rep_m2k_w = None;
    let direct = brut(&p, m);
    p.batiments[0].zones[0].groupes[0].aeration_espace_tampon = Some("veranda".into());
    let tampon = brut(&p, m);
    // Même Bbio (Th-B n'emploie pas l'aération) ; l'air de la véranda, plus chaud que l'air
    // extérieur lors des pointes de chaleur, élève le DH.
    assert_eq!(bbio(&direct), bbio(&tampon), "{titre}");
    assert!(
        dh(&tampon) > dh(&direct),
        "{titre} : {} / {}",
        dh(&tampon),
        dh(&direct)
    );
    println!(
        "{titre} : véranda, DH {:.1} avec espace tampon, {:.1} sans",
        dh(&tampon),
        dh(&direct)
    );
}

/// Bureaux, hors habitation : débits d'occupation et d'inoccupation (éq. 400 à 402) avec
/// `Crdbnr = 1` et `Cdep = Cfenb`.
fn verifier_bureaux(m: &MeteoProjet, titre: &str) {
    let mut p = cas("bureaux-simple-flux-ademe");
    let z = &mut p.batiments[0].zones[0];
    let extraction = z.groupes[0].bouches_conduits[0].clone();
    z.groupes[0].bouches_conduits = vec![BoucheConduit {
        id: "fenetres".into(),
        isouf: Isouf::Soufflage,
        crdbnr: None,
        cdep: None,
        ..extraction
    }];
    let ventilation = z.ventilation.as_mut().unwrap();
    ventilation.systeme = SystemeVentilation::Aeration;
    ventilation.reseau.r_rep_m2k_w = None;
    let r = brut(&p, m);
    let g = r.batiments[0].zones[0].groupes[0]
        .ventilation
        .as_ref()
        .unwrap();
    // Hors habitation, le débit restitué est celui de l'occupation (§6.2.3.2.2.1) :
    // Cdep × Crdbnr × qocc = 1,7 × 1 × 3 000 = 5 100 m³/h.
    assert!((g.q_spec_souffle_m3_h - 5100.0).abs() < 1e-6, "{titre}");
    assert!((g.q_spec_repris_m3_h + 5100.0).abs() < 1e-6, "{titre}");
    assert_eq!(g.c_aux_vent_kwh, 0.0, "{titre}");
    let reference = brut(&cas("bureaux-simple-flux-ademe"), m);
    assert_eq!(bbio(&r), bbio(&reference), "{titre}");
    println!(
        "{titre} : bureaux, Cep {:.1} (extraction mécanique : {:.1})",
        cep(&r),
        cep(&reference)
    );
}

#[test]
fn aeration_sur_la_meteo_ademe() {
    let m = meteo_ademe(&cas("maison-aeration-ademe"));
    verifier_maison(&m, "ADEME");
    verifier_espace_tampon(&m, "ADEME");
    verifier_bureaux(&m, "ADEME");
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn aeration_sur_la_meteo_reglementaire() {
    let m = meteo_reglementaire(&cas("maison-aeration-ademe"));
    verifier_maison(&m, "réglementaire");
    // Valeurs phares de la maison (nombres agrégés annuels, calculés par le moteur le 19/11/2026,
    // non comparés à un logiciel de référence) : Bbio 52,7 points, Cep 185,9 kWhep/(m².an),
    // DH 1 156,8 °C.h.
    let r = brut(&cas("maison-aeration-ademe"), &m);
    for (obtenu, attendu) in [(bbio(&r), 52.7), (cep(&r), 185.9), (dh(&r), 1156.8)] {
        assert!(
            (obtenu - attendu).abs() < 0.05,
            "{obtenu} au lieu de {attendu}"
        );
    }
    verifier_espace_tampon(&m, "réglementaire");
    verifier_bureaux(&m, "réglementaire");
}

/// Q98 (complément du 19/11/2026) : une zone en aération sans aucune bouche est refusée en
/// code 2, en habitation comme hors habitation (aucun débit d'air neuf, éq. 162).
#[test]
fn aeration_sans_bouche_refusee() {
    for nom in ["maison-aeration-ademe", "bureaux-simple-flux-ademe"] {
        let mut p = cas(nom);
        let z = &mut p.batiments[0].zones[0];
        for g in &mut z.groupes {
            g.bouches_conduits.clear();
        }
        let v = z.ventilation.as_mut().unwrap();
        v.systeme = SystemeVentilation::Aeration;
        v.reseau.r_rep_m2k_w = None;
        let r = thbce_core::valider(&p);
        assert!(!r.est_valide(), "{nom}");
        assert!(r.erreurs.iter().all(|e| !e.hors_perimetre), "{nom}");
        assert!(
            r.erreurs.iter().any(|e| e.message.contains("bouche")),
            "{nom} : {:?}",
            r.erreurs
        );
    }
}
