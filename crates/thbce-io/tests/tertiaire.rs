// SPDX-License-Identifier: GPL-3.0-or-later
//! Usages non résidentiels (lot 0a) : bureaux (usage 3) de bout en bout sur l'exemple
//! `tests/cas/bureaux-simple-flux-ademe`, météo ouverte ADEME ; sens des effets ; refus de ce
//! qui reste hors périmètre. Les valeurs de l'exemple sont vérifiées par son `attendu.json`.
//!
//! Valeurs phares sur la météo réglementaire :
//! THBCE_METEO=/chemin/vers/meteo.csv cargo t-regl bureaux_sur_meteo_reglementaire

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{GestEcl, GradEcl, Projet, SystemeVentilation};
use thbce_core::resultats::Resultats;
use thbce_core::{calculer, valider, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bureaux() -> Projet {
    let chemin = racine().join("tests/cas/bureaux-simple-flux-ademe/entree.json");
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn meteo_ademe(p: &Projet) -> MeteoProjet {
    let zone = p.localisation.zone_climatique_retenue();
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

fn calcul(p: &Projet) -> Resultats {
    calculer(p, &meteo_ademe(p), &Options { arrondir: false }).unwrap()
}

/// Indicateurs d'un calcul : Bbio, DH, Cep, Cep,nr, Ic énergie.
fn phares(r: &Resultats) -> [f64; 5] {
    let b = &r.batiments[0];
    [
        b.bbio_pts.unwrap(),
        b.dh_c_h.unwrap(),
        b.cep_kwhep_m2.unwrap(),
        b.cep_nr_kwhep_m2.unwrap(),
        b.ic_energie_kgco2_m2.unwrap(),
    ]
}

#[test]
fn les_bureaux_se_calculent_de_bout_en_bout() {
    let p = bureaux();
    let rapport = valider(&p);
    assert!(rapport.est_valide(), "{:?}", rapport.erreurs);
    let r = calcul(&p);
    let b = &r.batiments[0];
    let [bbio, dh, cep, cep_nr, ic] = phares(&r);
    println!(
        "ADEME : Bbio {bbio:.1} ; DH {dh:.1} ; Cep {cep:.1} ; Cep,nr {cep_nr:.1} ; Ic {ic:.1}"
    );
    assert!(bbio > 0.0 && dh >= 0.0 && cep > 0.0 && ic > 0.0);
    // Chaudière au gaz naturel et électricité : pas d'énergie renouvelable, Cep = Cep,nr.
    assert!((cep - cep_nr).abs() < 1e-9);
    // §13.1.3, éq. (2376) : hors habitation, la surface de référence est la surface utile.
    assert_eq!(b.sref_m2, 1000.0);
    let z = &b.zones[0];
    assert_eq!(z.nadeq, None);
    // §6.2.3.1.3, tableau 56-1 : débit conventionnel du Bbio, 4 m³/h par m² en occupation.
    let d = z.groupes[0].debits_bbio.as_ref().unwrap();
    assert_eq!(
        (d.q_spec_repris_m3_h, d.q_spec_souffle_m3_h),
        (-4000.0, 4000.0)
    );
    // §6.5.3.3, éq. (636) et (637) : 900 W en occupation (50 h par semaine, 52 semaines moins
    // les jours inoccupés), 90 W sinon ; entre 90 W et 900 W en moyenne, et bien en dessous
    // d'une marche permanente à 900 W.
    let v = z.ventilation.as_ref().unwrap();
    assert_eq!(v.p_vent_w, 900.0);
    assert!(v.c_aux_vent_kwh > 0.09 * 8760.0 && v.c_aux_vent_kwh < 0.4 * 0.9 * 8760.0);
    let i = b.indicateurs.as_ref().unwrap();
    assert!((i.electricite_consommee.auxiliaires_ventilation_kwh - v.c_aux_vent_kwh).abs() < 1e-6);
    // Article 10 : éclairage « dans un bâtiment tertiaire », 0,064 kg éq. CO2/kWh.
    let ecl: Vec<_> = i.lignes.iter().filter(|l| l.poste == "eclairage").collect();
    assert_eq!(ecl.len(), 1);
    assert_eq!(ecl[0].facteur_emission_kg_kwh, Some(0.064));
    // Exigences de l'usage 3 (lot 0b) : maximums et conformité évalués.
    let ex = i.exigences.as_ref().unwrap();
    assert!(ex.bbio_max_pts > 0.0 && ex.cep_max_kwhep_m2 > 0.0);
    assert_eq!(ex.conforme, ex.verifications.iter().all(|v| v.respectee));
    for v in &ex.verifications {
        println!("{v:?}");
    }
    println!(
        "Bbio_max {:.2} ; Cep_max {:.2} ; Cep,nr_max {:.2} ; Ic_max {:.2} ; conforme {}",
        ex.bbio_max_pts,
        ex.cep_max_kwhep_m2,
        ex.cep_nr_max_kwhep_m2,
        ex.ic_energie_max_kgco2_m2,
        ex.conforme
    );
    // ECS : tableau 277, 1,25 L par m² et par semaine sur 1 000 m² ; corr-e = (1 − 0,05) ×
    // (1 − 0,5 × 0,05) (mitigeurs économes, douches, Ratdouches-bains = 50 % en bureaux).
    let e = &z.groupes[0].systemes.as_ref().unwrap().ecs[0];
    assert!((e.a_l - 1.25).abs() < 1e-12 && (e.nadeq - 1000.0).abs() < 1e-9);
    assert!((e.corr - 0.95 * 0.975).abs() < 1e-12);
}

/// Sens des effets sur l'exemple.
#[test]
fn sens_des_effets() {
    let base = bureaux();
    let ref_ = calcul(&base);
    let [bbio, dh, cep, _, ic] = phares(&ref_);
    let eclairage = |r: &Resultats| {
        r.batiments[0]
            .indicateurs
            .as_ref()
            .unwrap()
            .electricite_consommee
            .eclairage_kwh
    };

    // 1. Plus d'isolant (U des murs et de la toiture divisés par deux) : le Bbio baisse, le
    //    Cep aussi.
    let mut p = base.clone();
    for t in &mut p.catalogue.parois {
        t.u_k_w_m2k /= 2.0;
    }
    let r = calcul(&p);
    let [bbio_isole, _, cep_isole, _, ic_isole] = phares(&r);
    println!("isolant : Bbio {bbio:.1} → {bbio_isole:.1} ; Cep {cep:.1} → {cep_isole:.1} ; Ic {ic:.1} → {ic_isole:.1}");
    assert!(bbio_isole < bbio && cep_isole < cep && ic_isole < ic);

    // 2. Puissance d'éclairage immobilier des bureaux plus forte (6 → 9 W/m²) : le Cep monte ;
    //    le Bbio, qui emploie une puissance conventionnelle (éq. 782), ne bouge pas.
    let mut p = base.clone();
    p.batiments[0].zones[0].groupes[0].locaux[0]
        .eclairage
        .p_ecl_tot_w_m2 = 9.0;
    let r = calcul(&p);
    let [bbio_ecl, _, cep_ecl, _, _] = phares(&r);
    println!(
        "éclairage 9 W/m² : Cep {cep:.1} → {cep_ecl:.1} ; éclairage {:.0} → {:.0} kWh",
        eclairage(&ref_),
        eclairage(&r)
    );
    assert_eq!(bbio_ecl, bbio);
    assert!(cep_ecl > cep && eclairage(&r) > eclairage(&ref_));

    // 3. Gestion de l'éclairage : sans gradation ni détection dans les bureaux, davantage de
    //    consommation.
    let mut p = base.clone();
    let e = &mut p.batiments[0].zones[0].groupes[0].locaux[0].eclairage;
    e.gest_ecl = GestEcl::InterrupteurManuel;
    e.grad_ecl = GradEcl::Aucune;
    e.p_ecl_aux_w_m2 = None;
    let r = calcul(&p);
    println!(
        "sans gestion : éclairage {:.0} → {:.0} kWh",
        eclairage(&ref_),
        eclairage(&r)
    );
    assert!(eclairage(&r) > eclairage(&ref_));

    // 4. Ventilation : sans régulation des débits (Crdbnr = 1 au lieu de 0,9), plus d'air neuf
    //    en occupation, donc plus de chauffage ; le Bbio (débits conventionnels) ne bouge pas.
    let mut p = base.clone();
    p.batiments[0].zones[0].groupes[0].bouches_conduits[0].crdbnr = Some(1.0);
    let r = calcul(&p);
    let [bbio_v, _, cep_v, _, _] = phares(&r);
    println!("Crdbnr = 1 : Cep {cep:.1} → {cep_v:.1}");
    assert_eq!(bbio_v, bbio);
    assert!(cep_v > cep);

    // 5. Ventilateur arrêté en inoccupation : moins d'auxiliaires.
    let mut p = base.clone();
    if let SystemeVentilation::SimpleFluxExtraction(v) = &mut p.batiments[0].zones[0]
        .ventilation
        .as_mut()
        .unwrap()
        .systeme
    {
        v.p_vent_inocc_w = Some(0.0);
    }
    p.batiments[0].zones[0].groupes[0].bouches_conduits[0].q_spec_inocc_m3_h = Some(0.0);
    let r = calcul(&p);
    let aux = |r: &Resultats| {
        r.batiments[0].zones[0]
            .ventilation
            .as_ref()
            .unwrap()
            .c_aux_vent_kwh
    };
    println!(
        "arrêt en inoccupation : ventilateurs {:.0} → {:.0} kWh",
        aux(&ref_),
        aux(&r)
    );
    assert!(aux(&r) < aux(&ref_));

    // 6. Éclairage conventionnel du Bbio saisi comme système du projet (ordre de grandeur de
    //    la variante de Q55 : éclairage conventionnel en Th-D) : effet sur le DH.
    let mut p = base.clone();
    for (l, e_ref) in p.batiments[0].zones[0].groupes[0]
        .locaux
        .iter_mut()
        .zip([500.0, 50.0, 100.0, 200.0])
    {
        l.eclairage.p_ecl_tot_w_m2 = 2.0 * e_ref / 100.0;
        l.eclairage.gest_ecl = GestEcl::InterrupteurEtHorloge;
        l.eclairage.grad_ecl = GradEcl::Manuelle;
        l.eclairage.fractionne = false;
        l.eclairage.p_ecl_aux_w_m2 = None;
        l.eclairage.eff_ecl_immo_projet_w_m2_100lux = None;
        l.eclairage.ei_proj_lux = None;
    }
    let r = calcul(&p);
    let [_, dh_conv, _, _, _] = phares(&r);
    println!("éclairage conventionnel en Th-D (Q55) : DH {dh:.1} → {dh_conv:.1}");
}

/// Ce qui reste refusé en non résidentiel : hors périmètre (code 3) ou erreur de saisie.
#[test]
fn refus() {
    let chemins = |p: &Projet| -> (Vec<String>, bool) {
        let r = valider(p);
        (
            r.erreurs.iter().map(|e| e.chemin.clone()).collect(),
            r.seulement_hors_perimetre(),
        )
    };
    let z = "batiments[0].zones[0]";

    // Les 28 usages sont calculés (lot 0c4), le double flux et la centrale de traitement d'air
    // en zone neutre aussi (lot 1-ven) : le refus d'un élément hors périmètre se teste sur un
    // puits climatique.
    let mut p = bureaux();
    p.batiments[0].zones[0].ventilation.as_mut().unwrap().puits =
        Some(thbce_core::entree::Puits::Climatique);
    let (c, hors) = chemins(&p);
    assert_eq!(c, [format!("{z}.ventilation.puits")]);
    assert!(hors);
    assert!(matches!(
        calculer(&p, &meteo_ademe(&p), &Options::default()),
        Err(thbce_core::Erreur::Saisie(_))
    ));
    // Double flux de l'habitation en non résidentiel : erreur de saisie (puissances en débit
    // de base et de pointe au lieu de l'occupation et de l'inoccupation).
    let mut p = bureaux();
    let maison = json::lire_projet(
        &std::fs::read_to_string(racine().join("tests/cas/maison-double-flux-ademe/entree.json"))
            .unwrap(),
    )
    .unwrap();
    p.batiments[0].zones[0]
        .ventilation
        .as_mut()
        .unwrap()
        .systeme = maison.batiments[0].zones[0]
        .ventilation
        .as_ref()
        .unwrap()
        .systeme
        .clone();
    let (_, hors) = chemins(&p);
    assert!(!hors);

    // Parking d'un bâtiment de bureaux : hors périmètre.
    let immeuble = json::lire_projet(
        &std::fs::read_to_string(racine().join("tests/cas/immeuble-collectif-ademe/entree.json"))
            .unwrap(),
    )
    .unwrap();
    let mut p = bureaux();
    p.batiments[0].parkings = immeuble.batiments[0].parkings.clone();
    let (c, hors) = chemins(&p);
    assert_eq!(c, ["batiments[0].parkings"]);
    assert!(hors);
    // Ascenseur desservant la zone de bureaux : hors périmètre.
    let mut p = bureaux();
    p.batiments[0].ascenseurs = immeuble.batiments[0].ascenseurs.clone();
    p.batiments[0].ascenseurs[0].zones_desservies = vec!["bureaux".into()];
    let (c, hors) = chemins(&p);
    assert_eq!(c, ["batiments[0].ascenseurs[0].zones_desservies[0]"]);
    assert!(hors);

    // Erreurs de saisie : locaux d'éclairage absents ; type de local d'un autre usage ; ratios
    // qui ne somment pas à 1 ; champs de l'habitation sur une bouche ; puissances de base et
    // de pointe ; périphériques de gestion sans dispositif.
    let g = format!("{z}.groupes[0]");
    let mut p = bureaux();
    p.batiments[0].zones[0].groupes[0].locaux.clear();
    assert_eq!(chemins(&p), (vec![format!("{g}.locaux")], false));
    let mut p = bureaux();
    p.batiments[0].zones[0].groupes[0].locaux[1].type_local =
        thbce_data::eclairage::TypeLocal::SalleClasse;
    assert_eq!(chemins(&p).0, [format!("{g}.locaux[1].type_local")]);
    let mut p = bureaux();
    p.batiments[0].zones[0].groupes[0].locaux[0].ratio_local = 0.5;
    assert_eq!(chemins(&p).0, [format!("{g}.locaux")]);
    let mut p = bureaux();
    p.batiments[0].zones[0].groupes[0].bouches_conduits[0].q_spec_conv_pointe_m3_h = 100.0;
    p.batiments[0].zones[0].groupes[0].bouches_conduits[0].q_spec_occ_m3_h = None;
    assert_eq!(
        chemins(&p).0,
        [
            format!("{g}.bouches_conduits[0].q_spec_conv_pointe_m3_h"),
            format!("{g}.bouches_conduits[0].q_spec_occ_m3_h")
        ]
    );
    let mut p = bureaux();
    if let SystemeVentilation::SimpleFluxExtraction(v) = &mut p.batiments[0].zones[0]
        .ventilation
        .as_mut()
        .unwrap()
        .systeme
    {
        v.p_vent_base_w = Some(100.0);
        v.p_vent_occ_w = None;
    }
    let s = format!("{z}.ventilation.systeme.simple_flux_extraction");
    assert_eq!(
        chemins(&p).0,
        [format!("{s}.p_vent_occ_w"), format!("{s}.p_vent_base_w")]
    );
    let mut p = bureaux();
    p.batiments[0].zones[0].groupes[0].locaux[1]
        .eclairage
        .p_ecl_aux_w_m2 = Some(0.2);
    assert_eq!(
        chemins(&p).0,
        [format!("{g}.locaux[1].eclairage.p_ecl_aux_w_m2")]
    );
    // Bureau standard à moins de 10 W/m² sans éclairement requis : erreur ; à 12 W/m², les
    // deux champs de l'éclairage mobilier sont sans objet.
    let mut p = bureaux();
    p.batiments[0].zones[0].groupes[0].locaux[0]
        .eclairage
        .ei_proj_lux = None;
    assert_eq!(
        chemins(&p).0,
        [format!("{g}.locaux[0].eclairage.ei_proj_lux")]
    );
    // En habitation, les champs du non résidentiel sont refusés.
    let mut p = maison;
    p.batiments[0].zones[0].groupes[0].locaux =
        bureaux().batiments[0].zones[0].groupes[0].locaux.clone();
    assert_eq!(chemins(&p).0, ["batiments[0].zones[0].groupes[0].locaux"]);
}

/// Calculs à la main du relecteur du lot sur l'exemple, indépendants du code.
#[test]
fn calculs_a_la_main_du_relecteur() {
    // Ventilateurs : 900 W pendant les heures d'occupation de l'année conventionnelle, 90 W
    // le reste du temps (éq. 636 et 637) : 2 902,5 kWh, pour la zone comme pour son groupe.
    let r = calcul(&bureaux());
    let z = &r.batiments[0].zones[0];
    let v = z.ventilation.as_ref().unwrap();
    assert!(
        (v.c_aux_vent_kwh - 2902.5).abs() < 1e-6,
        "{}",
        v.c_aux_vent_kwh
    );
    let g = z.groupes[0].ventilation.as_ref().unwrap();
    assert!(
        (g.c_aux_vent_kwh - 2902.5).abs() < 1e-6,
        "{}",
        g.c_aux_vent_kwh
    );
    // Débits spécifiques du projet vus du groupe : 4 191,89 m³/h en occupation (restitués).
    assert!(
        (g.q_spec_repris_m3_h + 4191.89).abs() < 5e-3,
        "{}",
        g.q_spec_repris_m3_h
    );

    // Débits d'inoccupation tous nuls mais puissance d'inoccupation saisie : la somme des
    // groupes reste celle de la zone (Q60, point 6).
    let mut p = bureaux();
    for b in &mut p.batiments[0].zones[0].groupes[0].bouches_conduits {
        b.q_spec_inocc_m3_h = Some(0.0);
    }
    let r = calcul(&p);
    let z = &r.batiments[0].zones[0];
    let zone = z.ventilation.as_ref().unwrap().c_aux_vent_kwh;
    let groupes: f64 = z
        .groupes
        .iter()
        .filter_map(|g| g.ventilation.as_ref())
        .map(|g| g.c_aux_vent_kwh)
        .sum();
    assert!(
        (zone - 2902.5).abs() < 1e-6 && (groupes - zone).abs() < 1e-6,
        "{zone} {groupes}"
    );

    // Aucun local n'a accès à la lumière naturelle : l'éclairage ne dépend plus du climat.
    // Th-C : 12 333,42 kWh par an ; Bbio : 14,770 kWh/m².
    let mut p = bureaux();
    for l in &mut p.batiments[0].zones[0].groupes[0].locaux {
        l.ratio_ecl_nat = 0.0;
    }
    let r = calcul(&p);
    let b = &r.batiments[0];
    let i = b.indicateurs.as_ref().unwrap();
    assert!(
        (i.electricite_consommee.eclairage_kwh - 12333.42).abs() < 5e-3,
        "{}",
        i.electricite_consommee.eclairage_kwh
    );
    let b_ecl = b.bbio.as_ref().unwrap().annuel.b_ecl_kwh_m2;
    assert!((b_ecl - 14.770).abs() < 5e-4, "{b_ecl}");
}

fn meteo_reglementaire(p: &Projet) -> MeteoProjet {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO pour lancer ce test");
    let zone = p.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

/// Valeurs phares de l'exemple de bureaux sur la météo réglementaire (H2b) : nombres annuels
/// seulement.
#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn bureaux_sur_meteo_reglementaire() {
    let p = bureaux();
    let r = calculer(&p, &meteo_reglementaire(&p), &Options { arrondir: false }).unwrap();
    assert!(r.reglementaire);
    let [bbio, dh, cep, cep_nr, ic] = phares(&r);
    println!(
        "réglementaire : Bbio {bbio:.1} ; DH {dh:.1} ; Cep {cep:.1} ; Cep,nr {cep_nr:.1} ; Ic {ic:.1}"
    );
    let proche = |a: f64, b: f64| (a - b).abs() <= 0.05 + 1e-9;
    assert!(proche(bbio, BBIO), "Bbio {bbio}");
    assert!(proche(dh, DH), "DH {dh}");
    assert!(proche(cep, CEP), "Cep {cep}");
    assert!(proche(cep_nr, CEP), "Cep,nr {cep_nr}");
    assert!(proche(ic, IC), "Ic énergie {ic}");
    // Exigences (permis de 2025, catégorie 1, BR1) : seul l'Ic énergie dépasse son maximum.
    let i = r.batiments[0].indicateurs.as_ref().unwrap();
    let ex = i.exigences.as_ref().unwrap();
    let non_respectees: Vec<&str> = ex
        .verifications
        .iter()
        .filter(|v| !v.respectee)
        .map(|v| v.indicateur.as_str())
        .collect();
    println!("réglementaire : non respectées {non_respectees:?}");
    assert_eq!(non_respectees, ["ic_energie"]);
    assert!(!ex.conforme);
}

/// Valeurs phares attendues sur la météo réglementaire (lot 0a, 17 novembre 2026).
const BBIO: f64 = 82.9;
const DH: f64 = 688.7;
const CEP: f64 = 66.8;
const IC: f64 = 349.8;
