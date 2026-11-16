// SPDX-License-Identifier: GPL-3.0-or-later
//! Ventilation du projet (lot 6a) de bout en bout sur la maison d'exemple : simple flux
//! hygroréglable (`tests/cas/maison-climat-ademe`) et double flux
//! (`tests/cas/maison-double-flux-ademe`), sur la météo ouverte ADEME (CI publique) et sur la
//! météo réglementaire (tests ignorés par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test ventilation -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les tests vérifient des propriétés physiques
//! (bilans fermés, Bbio indépendant du système, DH dépendant du système, températures de l'air
//! soufflé) et la consommation des ventilateurs calculée à la main.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::bati::assemblage_batiment::BatimentHoraire;
use thbce_core::bati::assemblage_groupe::{Lectures, VentilationThD};
use thbce_core::bati::assemblage_zone::ZoneHoraire;
use thbce_core::climat::{ClimatExterieur, ConfortAdaptatif, MeteoProjet};
use thbce_core::conventions::{ModeCalcul, NB_HEURES_AN};
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

fn simple_flux() -> Projet {
    cas("maison-climat-ademe")
}

fn double_flux() -> Projet {
    cas("maison-double-flux-ademe")
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

/// La maison simple flux équipée d'un double flux aux mêmes débits d'hygiène : bouche de
/// soufflage hygroréglable symétrique de la bouche d'extraction, entrées d'air conservées.
/// Pour le calcul du Bbio, rien ne change : le système conventionnel souffle et extrait le
/// même débit (§6.1.3, décision Q36).
fn simple_flux_passe_en_double_flux(sf: &Projet, df: &Projet) -> Projet {
    let mut p = sf.clone();
    let zone = &mut p.batiments[0].zones[0];
    zone.ventilation = df.batiments[0].zones[0].ventilation.clone();
    let extraction = zone.groupes[0].bouches_conduits[0].clone();
    zone.groupes[0].bouches_conduits.push(BoucheConduit {
        id: "soufflage".into(),
        isouf: Isouf::Soufflage,
        ..extraction
    });
    p
}

/// Simulation Th-D de la zone de la maison, avec un contrôle à chaque pas de temps.
fn simuler_th_d(
    projet: &Projet,
    meteo: &MeteoProjet,
    lectures: Lectures,
    mut controle: impl FnMut(usize, f64, &ZoneHoraire),
) -> ZoneHoraire {
    let loc = &projet.localisation;
    let climat = ClimatExterieur::nouveau(&meteo.th_d, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    // Le bâtiment entier : les pertes récupérables des systèmes y sont calculées (lot 6b).
    let mut bat = BatimentHoraire::nouveau_selon(
        &projet.batiments[0],
        &projet.catalogue,
        ModeCalcul::ThD,
        lectures,
        loc.alt_m,
        climat.we_kg_kg[0],
    )
    .unwrap();
    bat.simuler_avec(&climat, &confort, |h, b| {
        controle(h, climat.te_c[h], &b.zones[0]);
    })
    .unwrap();
    bat.zones.remove(0)
}

/// Propriétés communes aux deux systèmes, sur une météo.
fn verifier(meteo_de: impl Fn(&Projet) -> MeteoProjet, titre: &str) {
    let sf = simple_flux();
    let df = double_flux();
    let m = meteo_de(&sf);
    let r_sf = brut(&sf, &m);
    let r_df = brut(&df, &m);
    let sf_df = simple_flux_passe_en_double_flux(&sf, &df);
    let r_sf_df = brut(&sf_df, &m);

    // Th-B : le Bbio ne dépend pas du système installé, à débits d'hygiène égaux.
    assert_eq!(bbio(&r_sf), bbio(&r_sf_df));
    // Th-D : le DH en dépend.
    assert!((dh(&r_sf) - dh(&r_sf_df)).abs() > 1.0);

    // DH avec l'assemblage provisoire du lot 5 (ventilation conventionnelle du Bbio).
    let provisoire = Lectures {
        ventilation_th_d: VentilationThD::ConventionnelleBbio,
        ..Lectures::default()
    };
    let dh_de = |p: &Projet, lectures: Lectures| {
        simuler_th_d(p, &m, lectures, |_, _, _| {}).groupes[0]
            .cumuls
            .confort_ete
            .dh_c_h
    };
    for (nom, p, r) in [
        ("simple flux hygroréglable", &sf, &r_sf),
        ("double flux", &df, &r_df),
        (
            "simple flux passé en double flux, entrées d'air conservées",
            &sf_df,
            &r_sf_df,
        ),
    ] {
        let z = &r.batiments[0].zones[0];
        let v = z.ventilation.as_ref().unwrap();
        let g = z.groupes[0].ventilation.as_ref().unwrap();
        let ancien = dh_de(p, provisoire);
        println!(
            "{titre}, {nom} : Bbio = {:.1} points ; DH = {:.1} °C.h (assemblage provisoire du \
             lot 5 : {ancien:.1}, écart {:+.1}) ; ventilateurs {:.2} W, {:.1} kWh/an, {:.2} \
             kWh/m² ; débits repris {:.1}, soufflé {:.1} m³/h ; bypass {} h, antigel {:.1} kWh",
            bbio(r),
            dh(r),
            dh(r) - ancien,
            v.p_vent_w,
            v.c_aux_vent_kwh,
            g.c_aux_vent_kwh_m2,
            g.q_spec_repris_m3_h,
            g.q_spec_souffle_m3_h,
            v.th_d.unwrap().nbh_bypass,
            v.th_d.unwrap().q_ag_kwh,
        );
        // La consommation annuelle est la puissance moyenne sommée sur 8 760 h, au niveau du
        // groupe, de la zone et du bâtiment ; mois au sens des scénarios.
        let annuel = v.p_vent_w * NB_HEURES_AN as f64 / 1000.0;
        assert!((v.c_aux_vent_kwh - annuel).abs() < 1e-9);
        assert!((g.c_aux_vent_kwh - annuel).abs() < 1e-9);
        assert_eq!(r.batiments[0].c_aux_vent_kwh, Some(v.c_aux_vent_kwh));
        let somme: f64 = g.c_aux_vent_mensuel_kwh.iter().sum();
        assert!((somme - annuel).abs() < 1e-9);
        // Ordre de grandeur : quelques kWh par m² et par an pour une maison.
        assert!((0.5..6.0).contains(&g.c_aux_vent_kwh_m2), "{g:?}");
        // DH dépendant de l'assemblage.
        assert!((dh(r) - ancien).abs() > 0.1);
    }

    // Consommation des ventilateurs à la main.
    // Simple flux : Dugd = 7 h (bouche temporisée), 14 W en base et 32 W en pointe (éq. 641).
    let p_sf = (32.0 * 7.0 + 14.0 * 161.0) / 168.0;
    let v_sf = r_sf.batiments[0].zones[0].ventilation.as_ref().unwrap();
    assert!((v_sf.p_vent_w - p_sf).abs() < 1e-12);
    // Double flux : Dugd = 14 h par défaut, 15 W en base et 45 W en pointe, sur chaque
    // ventilateur (éq. 391 et 393).
    let p_df = 2.0 * (45.0 * 14.0 + 15.0 * 154.0) / 168.0;
    let v_df = r_df.batiments[0].zones[0].ventilation.as_ref().unwrap();
    assert!((v_df.p_vent_w - p_df).abs() < 1e-12);

    // Enchaînement horaire du double flux en Th-D : air soufflé, bypass, bilan fermé.
    let epsilon = match &df.batiments[0].zones[0]
        .ventilation
        .as_ref()
        .unwrap()
        .systeme
    {
        SystemeVentilation::DoubleFlux(d) => d.echangeur.as_ref().unwrap(),
        _ => unreachable!(),
    };
    assert!(epsilon.bypass.is_some());
    let mut heures_recuperation = 0;
    let mut theta_i_prec = 18.0;
    let zone = simuler_th_d(&df, &m, Lectures::default(), |_, te, z| {
        let air = z.air_souffle()[0];
        let s = z.sorties_ventilation;
        if s.epsilon > 0.0 && te < theta_i_prec - 5.0 {
            // Échangeur actif : l'air soufflé est réchauffé, sans dépasser l'air repris
            // (ventilateurs et pertes des conduits compris, à 1 K près).
            assert!(air.t_air_souffle_c > te, "{air:?} {te}");
            assert!(
                air.t_air_souffle_c < theta_i_prec + 1.0,
                "{air:?} {theta_i_prec}"
            );
            heures_recuperation += 1;
        }
        if s.bypass {
            assert_eq!(s.epsilon, 0.0);
        }
        theta_i_prec = z.groupes[0].etat.fin.theta_i_c;
    });
    assert!(heures_recuperation > 1000, "{heures_recuperation}");
    assert!(zone.cumuls_ventilation.nbh_bypass > 0);
    let b = &zone.groupes[0].cumuls.bilan;
    assert!(b.residu_wh().abs() <= 1e-9 * b.entrees_wh(), "{b:?}");
    assert_eq!(zone.groupes[0].cumuls.confort_ete.dh_c_h, dh(&r_df));

    // Simple flux : bilan fermé, aucun débit soufflé.
    let zone = simuler_th_d(&sf, &m, Lectures::default(), |_, _, z| {
        assert_eq!(z.groupes[0].debits.q_souffle_m3_h, 0.0);
    });
    let b = &zone.groupes[0].cumuls.bilan;
    assert!(b.residu_wh().abs() <= 1e-9 * b.entrees_wh(), "{b:?}");
}

#[test]
fn ventilation_du_projet_sur_la_meteo_ademe() {
    verifier(meteo_ademe, "météo ADEME");
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn ventilation_du_projet_sur_la_meteo_reglementaire() {
    verifier(meteo_reglementaire, "météo réglementaire, H2b");
    // Variante de l'exemple avec véranda et garage.
    let v = cas("maison-veranda-ademe");
    let m = meteo_reglementaire(&v);
    let r = brut(&v, &m);
    let provisoire = Lectures {
        ventilation_th_d: VentilationThD::ConventionnelleBbio,
        ..Lectures::default()
    };
    let loc = &v.localisation;
    let climat = ClimatExterieur::nouveau(&m.th_d, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    let mut bat = BatimentHoraire::nouveau_selon(
        &v.batiments[0],
        &v.catalogue,
        ModeCalcul::ThD,
        provisoire,
        loc.alt_m,
        climat.we_kg_kg[0],
    )
    .unwrap();
    bat.simuler(&climat, &confort).unwrap();
    let ancien = bat.zones[0].groupes[0].cumuls.confort_ete.dh_c_h;
    let mut bat = BatimentHoraire::nouveau_selon(
        &v.batiments[0],
        &v.catalogue,
        ModeCalcul::ThD,
        Lectures::default(),
        loc.alt_m,
        climat.we_kg_kg[0],
    )
    .unwrap();
    bat.simuler(&climat, &confort).unwrap();
    assert_eq!(bat.zones[0].groupes[0].cumuls.confort_ete.dh_c_h, dh(&r));
    println!(
        "météo réglementaire, H2b, variante véranda et garage : Bbio = {:.1} points ; DH = \
         {:.1} °C.h (assemblage provisoire du lot 5 : {ancien:.1}, écart {:+.1})",
        bbio(&r),
        dh(&r),
        dh(&r) - ancien
    );
}
