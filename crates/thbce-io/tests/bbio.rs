// SPDX-License-Identifier: GPL-3.0-or-later
//! Calcul annuel du Bbio de bout en bout sur la maison d'exemple
//! (`tests/cas/maison-climat-ademe/entree.json`, Indre-et-Loire, zone H2b).
//!
//! Le scénario est joué deux fois : sur la météo ouverte ADEME du dépôt, par la CI publique
//! (résultat non réglementaire), et sur la météo réglementaire, qui ne peut pas être versionnée
//! (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test bbio -- --ignored --nocapture
//! ```

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::bati::assemblage_zone::ZoneBbio;
use thbce_core::climat::calendrier::{hleg, nouveau_jour_legal};
use thbce_core::climat::{ClimatExterieur, ConfortAdaptatif, MeteoProjet};
use thbce_core::entree::Projet;
use thbce_core::resultats::{Resultats, ResultatsBbio};
use thbce_core::{calculer, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn maison() -> Projet {
    let chemin = racine().join("tests/cas/maison-climat-ademe/entree.json");
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

fn bbio(r: &Resultats) -> &ResultatsBbio {
    r.batiments[0].bbio.as_ref().unwrap()
}

/// Même maison, coefficients U des parois opaques divisés par deux.
fn mieux_isolee(projet: &Projet) -> Projet {
    let mut p = projet.clone();
    for paroi in &mut p.catalogue.parois {
        paroi.u_k_w_m2k /= 2.0;
    }
    p
}

fn afficher(titre: &str, r: &Resultats) {
    let b = bbio(r);
    println!("{titre} : réglementaire = {}", r.reglementaire);
    println!(
        "  Bbio = {:.2} points ; B_Ch = {:.2}, B_Fr = {:.2}, B_Ecl = {:.2} kWh/m²",
        b.annuel.bbio_pts, b.annuel.b_ch_kwh_m2, b.annuel.b_fr_kwh_m2, b.annuel.b_ecl_kwh_m2
    );
    println!("  mois   B_Ch   B_Fr  B_Ecl   Bbio");
    for (m, v) in b.mensuel.iter().enumerate() {
        println!(
            "  {:4} {:6.2} {:6.2} {:6.2} {:6.2}",
            m + 1,
            v.b_ch_kwh_m2,
            v.b_fr_kwh_m2,
            v.b_ecl_kwh_m2,
            v.bbio_pts
        );
    }
    let g = &r.batiments[0].zones[0].groupes[0];
    println!("  saisons : {:?}", g.saisons_bbio.as_ref().unwrap());
    println!("  confort : {:?}", g.confort_bbio.as_ref().unwrap());
}

/// Propriétés attendues du calcul annuel de la maison d'exemple, quelle que soit la météo.
fn verifier_le_scenario(projet: &Projet, meteo: &MeteoProjet, titre: &str) -> Resultats {
    let r = brut(projet, meteo);
    afficher(titre, &r);
    let b = bbio(&r);
    let a = &b.annuel;
    assert!(a.bbio_pts > 0.0);
    assert!(a.b_ch_kwh_m2 > 0.0 && a.b_ecl_kwh_m2 > 0.0);
    // Ordres de grandeur d'une maison isolée en zone H2b : ni nuls ni aberrants.
    assert!((5.0..60.0).contains(&a.b_ch_kwh_m2), "{}", a.b_ch_kwh_m2);
    assert!((0.5..5.0).contains(&a.b_ecl_kwh_m2), "{}", a.b_ecl_kwh_m2);
    assert!((20.0..200.0).contains(&a.bbio_pts), "{}", a.bbio_pts);

    // Pas de besoin de chauffage en juillet : le chauffage est interdit du jour 183 au jour 252
    // (§4.6.3.2, éq. 79), soit de juillet à début septembre au sens des scénarios.
    assert_eq!(b.mensuel[6].b_ch_kwh_m2, 0.0);
    assert_eq!(b.mensuel[7].b_ch_kwh_m2, 0.0);
    // Besoin de froid en été sans système de refroidissement : en Bbio, les autorisations
    // effectives sont les autorisations propres (§4.6.3.3, éq. 91 ; décision Q35 point 7).
    let froid_d_ete: f64 = b.mensuel[5..8].iter().map(|m| m.b_fr_kwh_m2).sum();
    assert!(froid_d_ete > 0.0);
    assert!(a.b_fr_kwh_m2 >= froid_d_ete);
    // Pas de besoin de froid en plein hiver.
    assert_eq!(b.mensuel[0].b_fr_kwh_m2, 0.0);
    assert_eq!(b.mensuel[11].b_fr_kwh_m2, 0.0);

    // Éq. (2386) : Bbio annuel = somme des Bbio mensuels ; éq. (2385), décision Q10.
    assert_eq!(b.mensuel.len(), 12);
    assert_eq!(
        a.bbio_pts,
        b.mensuel.iter().map(|m| m.bbio_pts).sum::<f64>()
    );
    let combine = 2.0 * a.b_ch_kwh_m2 + 2.0 * a.b_fr_kwh_m2 + 5.0 * a.b_ecl_kwh_m2;
    assert!((a.bbio_pts - combine).abs() <= 1e-9 * combine);
    assert_eq!(r.batiments[0].bbio_pts, Some(a.bbio_pts));

    // Une meilleure isolation (U des parois opaques divisé par deux) réduit le besoin de
    // chauffage, mois par mois, sans changer l'éclairage.
    let isole = brut(&mieux_isolee(projet), meteo);
    let bi = bbio(&isole);
    println!(
        "  U/2 : B_Ch = {:.2}, B_Fr = {:.2} kWh/m² ; Bbio = {:.2} points",
        bi.annuel.b_ch_kwh_m2, bi.annuel.b_fr_kwh_m2, bi.annuel.bbio_pts
    );
    assert!(bi.annuel.b_ch_kwh_m2 < 0.9 * a.b_ch_kwh_m2);
    for (m, (avant, apres)) in b.mensuel.iter().zip(&bi.mensuel).enumerate() {
        if matches!(m, 0 | 1 | 10 | 11) {
            assert!(apres.b_ch_kwh_m2 < avant.b_ch_kwh_m2, "mois {}", m + 1);
        }
    }
    r
}

/// Bilan d'énergie et enchaînement horaire de la maison d'exemple.
fn verifier_l_enchainement(projet: &Projet, meteo: &MeteoProjet) {
    let loc = &projet.localisation;
    let climat = ClimatExterieur::nouveau(&meteo.th_bc, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    let mut zone = ZoneBbio::nouvelle(
        &projet.batiments[0].zones[0],
        &projet.catalogue,
        loc.alt_m,
        climat.we_kg_kg[0],
    )
    .unwrap();
    let mut saison = None;
    let (mut theta_min, mut theta_max) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut top_j_max: f64 = 0.0;
    for h in 0..8760 {
        // Éq. (175) et décision Q39 : au premier pas de temps d'un jour légal, la température
        // opérative maximale de la veille est celle des pas de temps du jour légal précédent.
        let veille = zone.groupes[0].etat.fin.theta_op_c;
        zone.pas(h, &climat, &confort).unwrap();
        top_j_max = top_j_max.max(veille);
        if nouveau_jour_legal(h) {
            assert_eq!(
                zone.groupes[0].etat.t_max.top_j1_max_c, top_j_max,
                "pas {h}"
            );
            top_j_max = 0.0;
        }
        let s = zone.sorties[0].unwrap();
        // Les saisons ne basculent qu'après le pas de temps qui se termine à 9 h légales.
        if saison.is_some_and(|precedente| precedente != s.saison) {
            assert_eq!(hleg(h - 1), 9, "changement de saison au pas de temps {h}");
        }
        saison = Some(s.saison);
        // Un groupe ne chauffe pas et ne refroidit pas au même pas de temps (éq. 802).
        assert!(s.q_sys_ch_wh == 0.0 || s.q_sys_fr_wh == 0.0);
        assert!(s.q_sys_ch_wh >= 0.0 && s.q_sys_fr_wh <= 0.0 && s.c_ecl_kwh >= 0.0);
        theta_min = theta_min.min(s.fin.theta_op_c);
        theta_max = theta_max.max(s.fin.theta_op_c);
    }
    let g = &zone.groupes[0];
    let b = &g.cumuls.bilan;
    println!(
        "  température opérative de {theta_min:.1} à {theta_max:.1} °C ; baies ouvertes {} h",
        g.cumuls.nbh_surventilation
    );
    println!(
        "  bilan d'énergie (kWh) : solaire des baies {:.0}, parois opaques {:.0}, apports \
              internes {:.0}, chauffage {:.0} ; refroidissement {:.0}, pertes des baies {:.0}, \
              des parois opaques {:.0}, par l'air {:.0}, stockage {:.0} ; résidu {:.2e} Wh",
        b.apports_solaires_baies_wh / 1e3,
        b.apports_opaques_wh / 1e3,
        b.apports_internes_wh / 1e3,
        b.chauffage_wh / 1e3,
        b.refroidissement_wh / 1e3,
        b.pertes_baies_wh / 1e3,
        b.pertes_opaques_wh / 1e3,
        b.pertes_air_wh / 1e3,
        b.stockage_wh / 1e3,
        b.residu_wh()
    );
    // Bilan d'énergie fermé sur l'année. Le schéma de Crank-Nicolson (éq. 365) et
    // l'interpolation dans la matrice (éq. 825 et 826) sont exacts pour un modèle linéaire :
    // seules les erreurs d'arrondi, accumulées sur 8 760 pas de temps, écartent le résidu de
    // zéro. 1e-9 des flux entrants (moins de 0,01 Wh sur près de 10 MWh) laisse plusieurs
    // ordres de grandeur de marge et détecterait un watt oublié pendant une seule heure.
    assert!(b.entrees_wh() > 1.0e6);
    assert!(b.residu_wh().abs() <= 1e-9 * b.entrees_wh(), "{b:?}");
    // Le chauffage et le refroidissement du bilan sont les besoins de l'année.
    let q_ch: f64 = g.cumuls.q_sys_ch_wh.iter().sum();
    let q_fr: f64 = g.cumuls.q_sys_fr_wh.iter().sum();
    assert!((q_ch - b.chauffage_wh).abs() <= 1e-9 * q_ch);
    assert!((q_fr - b.refroidissement_wh).abs() <= 1e-9 * q_fr.max(1.0));
    // La surventilation naturelle par ouverture des baies joue en saison de refroidissement.
    assert!(g.cumuls.nbh_surventilation > 0);
    // Températures plausibles pour un logement chauffé à 19 °C et refroidi à 26 °C.
    assert!(theta_min > 10.0 && theta_max < 40.0);
}

#[test]
fn bbio_de_la_maison_d_exemple_sur_la_meteo_ademe() {
    let p = maison();
    let m = meteo_ademe(&p);
    let r = verifier_le_scenario(&p, &m, "météo ADEME (+2,7 °C, vague de chaleur)");
    // Météo ouverte : le résultat est marqué non réglementaire.
    assert!(!r.reglementaire);
    verifier_l_enchainement(&p, &m);
    // Restitution : arrondi à une décimale (décision Q8).
    let arrondi = calculer(&p, &m, &Options::default()).unwrap();
    assert!(arrondi.valeurs_arrondies && !arrondi.reglementaire);
    let attendu = (bbio(&r).annuel.bbio_pts * 10.0).round() / 10.0;
    assert_eq!(arrondi.batiments[0].bbio_pts, Some(attendu));
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn bbio_de_la_maison_d_exemple_sur_la_meteo_reglementaire() {
    let p = maison();
    let m = meteo_reglementaire(&p);
    let r = verifier_le_scenario(&p, &m, "météo réglementaire RE2020, H2b");
    assert!(r.reglementaire);
    verifier_l_enchainement(&p, &m);
}

/// Temps d'un calcul annuel complet de la maison d'exemple (validation, climat, 8 760 pas de
/// temps, restitution). À lancer en mode optimisé pour une mesure représentative :
///
/// ```text
/// cargo test --release -p thbce-io --test bbio -- --ignored --nocapture temps_de_calcul
/// ```
#[test]
#[ignore = "mesure de performance, à lancer avec --release --nocapture"]
fn temps_de_calcul_annuel() {
    let p = maison();
    let m = meteo_ademe(&p);
    let n = 10;
    let debut = std::time::Instant::now();
    for _ in 0..n {
        std::hint::black_box(calculer(&p, &m, &Options::default()).unwrap());
    }
    let par_calcul = debut.elapsed() / n;
    println!("calcul annuel de la maison d'exemple : {par_calcul:?} (moyenne de {n} calculs)");
}
