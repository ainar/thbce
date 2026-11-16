// SPDX-License-Identifier: GPL-3.0-or-later
//! Générations du projet (lot 6c) de bout en bout sur la maison d'exemple et ses variantes, sur
//! la météo ouverte ADEME (CI publique) et sur la météo réglementaire (tests ignorés par
//! défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test generation -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les tests vérifient des propriétés (bilans fermés,
//! consommation de l'effet Joule égale à l'énergie fournie, répartition entre les groupes, Bbio
//! indépendant des générateurs) et non des valeurs.

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
use thbce_core::resultats::{Consommations, Resultats};
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

/// Bilans des consommations d'un bâtiment : le total annuel est la somme des mois ; les
/// consommations des groupes, des zones et du bâtiment sont celles des générations (éq. 1484 à
/// 1486).
fn verifier_bilans(r: &Resultats, titre: &str) {
    let b = &r.batiments[0];
    let somme_des_mois =
        |c: &Consommations| -> f64 { c.mensuel.iter().map(|m| m.total_kwh()).sum() };
    let total_generations: f64 = b
        .generations
        .iter()
        .map(|g| g.consommations.annuel.total_kwh())
        .sum();
    let cb = b.consommations.as_ref().unwrap();
    assert!(
        (cb.annuel.total_kwh() - total_generations).abs() < 1e-6,
        "{titre}"
    );
    assert!(
        (somme_des_mois(cb) - cb.annuel.total_kwh()).abs() < 1e-6,
        "{titre}"
    );
    assert_eq!(cb.mensuel.len(), 12);
    let mut total_groupes = 0.0;
    for z in &b.zones {
        let cz = z.consommations.as_ref().unwrap();
        let groupes: f64 = z
            .groupes
            .iter()
            .map(|g| g.consommations.as_ref().unwrap().annuel.total_kwh())
            .sum();
        assert!((cz.annuel.total_kwh() - groupes).abs() < 1e-6, "{titre}");
        total_groupes += groupes;
    }
    assert!((total_groupes - total_generations).abs() < 1e-6, "{titre}");
    for g in &b.generations {
        // La consommation de la génération est celle de ses générateurs (éq. 1056).
        let generateurs: f64 = g
            .generateurs
            .iter()
            .map(|x| x.consommations.total_kwh())
            .sum();
        assert!(
            (g.consommations.annuel.total_kwh() - generateurs).abs() < 1e-6,
            "{titre} : {}",
            g.id
        );
        // Ce qui a été demandé a été fourni, ou reste reporté à la fin de l'année (éq. 1028).
        let demande = g.q_ch_kwh + g.q_fr_kwh + g.q_ecs_kwh;
        let fourni = g.q_fou_ch_kwh + g.q_fou_fr_kwh + g.q_fou_ecs_kwh;
        assert!(
            (demande - fourni - g.q_rep_fin_kwh).abs() < 1e-6,
            "{titre} : {}",
            g.id
        );
    }
}

fn verifier(meteo_de: impl Fn(&Projet) -> MeteoProjet, titre: &str) {
    let maison = cas("maison-climat-ademe");
    let m = meteo_de(&maison);
    let r = brut(&maison, &m);
    let b = &r.batiments[0];
    verifier_bilans(&r, titre);
    assert!(
        !r.avertissements
            .iter()
            .any(|a| a.contains("générations idéales")),
        "{titre}"
    );

    // Effet Joule (§8.18) : rendement unitaire, la consommation d'électricité est l'énergie
    // fournie, c'est-à-dire la demande des réseaux, sans pertes ni auxiliaires.
    let chauffage = &b.generations[0];
    let c = &chauffage.consommations.annuel;
    assert!(chauffage.q_ch_kwh > 0.0);
    assert!((c.chauffage.electricite_kwh - chauffage.q_fou_ch_kwh).abs() < 1e-6);
    assert!((chauffage.q_fou_ch_kwh - chauffage.q_ch_kwh + chauffage.q_rep_fin_kwh).abs() < 1e-6);
    assert!(chauffage.q_rep_fin_kwh < 1e-6, "{titre} : {chauffage:?}");
    assert_eq!(c.total_kwh(), c.chauffage.electricite_kwh);
    assert_eq!(chauffage.pertes_vc_kwh, 0.0);
    let g = &chauffage.generateurs[0];
    assert!((g.eta_eff_ch_an.unwrap() - 1.0).abs() < 1e-12);
    assert_eq!(g.w_aux_kwh, 0.0);
    // La demande de la génération est celle de son réseau intergroupes.
    assert!((chauffage.q_ch_kwh - b.distributions_intergroupes[0].q_ch_kwh).abs() < 1e-9);
    // Production d'ECS : consommation égale à la demande de la distribution.
    let ecs = &b.generations[1];
    assert!((ecs.q_ecs_kwh - b.distributions_ecs_intergroupes[0].q_w_prim_kwh).abs() < 1e-9);
    assert!((ecs.consommations.annuel.ecs.electricite_kwh - ecs.q_fou_ecs_kwh).abs() < 1e-6);
    assert!(
        (ecs.q_fou_ecs_kwh - ecs.q_ecs_kwh).abs() < 1e-6,
        "{titre} : {ecs:?}"
    );
    // Un seul groupe : il porte toutes les consommations, par poste.
    let cg = b.zones[0].groupes[0].consommations.as_ref().unwrap();
    assert!((cg.annuel.chauffage.electricite_kwh - c.chauffage.electricite_kwh).abs() < 1e-6);
    assert!((cg.annuel.ecs.electricite_kwh - ecs.q_fou_ecs_kwh).abs() < 1e-6);
    assert_eq!(cg.annuel.refroidissement.total_kwh(), 0.0);
    // Chauffage en hiver, pas en été ; ECS toute l'année.
    assert!(cg.mensuel[0].chauffage.electricite_kwh > 0.0);
    assert_eq!(cg.mensuel[6].chauffage.electricite_kwh, 0.0);
    assert!(cg.mensuel.iter().all(|m| m.ecs.electricite_kwh > 0.0));
    println!(
        "{titre} : maison d'exemple, Bbio = {:.1} points, DH = {:.1} °C.h ; chauffage {:.1} kWh \
         d'électricité (demande {:.1} kWh), ECS {:.1} kWh (demande {:.1} kWh), sous-dimensionnement \
         {} h au plus",
        b.bbio_pts.unwrap(),
        b.dh_c_h.unwrap(),
        c.chauffage.electricite_kwh,
        chauffage.q_ch_kwh,
        ecs.consommations.annuel.ecs.electricite_kwh,
        ecs.q_ecs_kwh,
        chauffage.nbh_sousdim_ch_ecs_max,
    );

    // Générateurs sous-dimensionnés : la demande non couverte est reportée d'heure en heure
    // (éq. 1014 et 1028) et le calcul le signale (éq. 1059 à 1061) ; Bbio et DH, qui ne
    // dépendent pas des générateurs à effet Joule, sont inchangés.
    let mut petite = maison.clone();
    petite.batiments[0].generations[0].generateurs[0].rdim = 2;
    let rp = brut(&petite, &m);
    verifier_bilans(&rp, titre);
    let bp = &rp.batiments[0];
    assert_eq!(bp.bbio_pts, b.bbio_pts);
    assert_eq!(bp.dh_c_h, b.dh_c_h);
    assert!((bp.generations[0].q_ch_kwh - chauffage.q_ch_kwh).abs() < 1e-9);
    assert!(bp.generations[0].nbh_sousdim_ch_ecs_max > 6);
    assert!(rp
        .avertissements
        .iter()
        .any(|a| a.contains("sous-dimensionnement")));

    // Deux générateurs en cascade : le premier fournit l'essentiel, l'appoint le reste, et la
    // consommation totale est inchangée (rendements unitaires).
    let mut cascade = maison.clone();
    {
        let g = &mut cascade.batiments[0].generations[0];
        g.priorite = serde_json::from_str(r#""cascade""#).unwrap();
        g.generateurs[0].rdim = 2;
        g.generateurs[0].priorite_chauffage = Some(1);
        let mut appoint = g.generateurs[0].clone();
        appoint.id = "appoint".into();
        appoint.rdim = 6;
        appoint.priorite_chauffage = Some(2);
        g.generateurs.push(appoint);
    }
    let rk = brut(&cascade, &m);
    verifier_bilans(&rk, titre);
    let gk = &rk.batiments[0].generations[0];
    assert!(gk.generateurs[0].q_fou_ch_kwh > gk.generateurs[1].q_fou_ch_kwh);
    assert!(gk.generateurs[1].q_fou_ch_kwh > 0.0);
    // En relance, l'appoint n'est pas sollicité (§8.17.3.5.6) : une part de la demande est
    // reportée, sans changer le total fourni sur l'année.
    assert!(
        (gk.q_fou_ch_kwh + gk.q_rep_fin_kwh - chauffage.q_ch_kwh).abs() < 1e-6,
        "{titre}"
    );

    // Double flux avec sécurité antigel : son énergie (éq. 508) est demandée à la génération
    // désignée (§8.11.3, éq. 939), en plus de la demande du réseau de chauffage.
    let mut df = cas("maison-double-flux-ademe");
    let v = serde_json::to_value(&df.batiments[0].zones[0].ventilation).unwrap();
    let mut v2 = v;
    v2["systeme"]["double_flux"]["echangeur"]["antigel"] = true.into();
    v2["systeme"]["double_flux"]["echangeur"]["generation_antigel"] =
        "generation-effet-joule".into();
    df.batiments[0].zones[0].ventilation = serde_json::from_value(v2).unwrap();
    let rd = brut(&df, &m);
    verifier_bilans(&rd, titre);
    let bd = &rd.batiments[0];
    let q_ag = bd.zones[0]
        .ventilation
        .as_ref()
        .unwrap()
        .th_c
        .unwrap()
        .q_ag_kwh;
    let gd = &bd.generations[0];
    assert!(
        (gd.q_ch_kwh - bd.distributions_intergroupes[0].q_ch_kwh - q_ag).abs() < 1e-6,
        "{titre}"
    );
    println!(
        "{titre} : double flux avec antigel, {q_ag:.1} kWh d'antigel sur {:.1} kWh de chauffage",
        gd.q_ch_kwh
    );

    // Variante à plancher chauffant à eau : pompe à chaleur air extérieur / eau à la température
    // de ses réseaux, puis appoint électrique, en cascade (§8.23 et §8.17.3.7.2.2).
    let plancher = cas("maison-plancher-ademe");
    let rp = brut(&plancher, &m);
    verifier_bilans(&rp, titre);
    let bp = &rp.batiments[0];
    let gp = &bp.generations[0];
    assert_eq!(bp.bbio_pts, b.bbio_pts);
    let pac = &gp.generateurs[0];
    let appoint = &gp.generateurs[1];
    // COP saisonnier (éq. 1526) : énergie fournie sur électricité consommée, auxiliaires
    // compris. Les COP certifiés vont de 2,9 (−7 °C) à 4,5 (7 °C) à 32,5 °C d'eau ; la loi d'eau
    // du plancher (35 °C au plus au départ) et la charge partielle d'un compresseur à vitesse
    // variable laissent attendre un COP saisonnier entre 3 et 4,5.
    let cop = pac.eta_eff_ch_an.unwrap();
    assert!((3.0..4.6).contains(&cop), "{titre} : COP saisonnier {cop}");
    assert!((pac.q_fou_ch_kwh / pac.consommations.chauffage.electricite_kwh - cop).abs() < 1e-9);
    // La chaleur fournie est l'électricité absorbée par le compresseur et la chaleur puisée dans
    // l'air (éq. 1332) : φrejet est négatif et inférieur en valeur absolue à l'énergie fournie.
    assert!(pac.phi_rejet_kwh < 0.0 && -pac.phi_rejet_kwh < pac.q_fou_ch_kwh);
    assert!(pac.w_aux_kwh > 0.0);
    // L'appoint ne fournit qu'une faible part ; l'ensemble consomme bien moins que l'effet Joule.
    assert!(
        appoint.q_fou_ch_kwh < 0.1 * pac.q_fou_ch_kwh,
        "{titre} : {appoint:?}"
    );
    assert!(gp.consommations.annuel.chauffage.electricite_kwh < 0.4 * gp.q_ch_kwh);
    assert!(pac
        .valeurs_par_defaut
        .iter()
        .any(|v| v.champ == "taux_auxiliaires" && v.valeur == 0.02));
    println!(
        "{titre} : plancher et PAC air/eau, demande {:.1} kWh, PAC {:.1} kWh fournis pour {:.1} kWh \
         d'électricité (COP saisonnier {cop:.2}, auxiliaires {:.1} kWh), appoint {:.1} kWh, \
         report en fin d'année {:.1} kWh, sous-dimensionnement {} h ; DH = {:.1} °C.h",
        gp.q_ch_kwh,
        pac.q_fou_ch_kwh,
        pac.consommations.chauffage.electricite_kwh,
        pac.w_aux_kwh,
        appoint.q_fou_ch_kwh,
        gp.q_rep_fin_kwh,
        gp.nbh_sousdim_ch_ecs_max,
        bp.dh_c_h.unwrap(),
    );

    // Variante à pompe à chaleur air/air réversible : chauffage et refroidissement par le même
    // générateur, sur l'air ambiant (réseaux fictifs).
    let clim = cas("maison-pac-air-air-ademe");
    let rc = brut(&clim, &m);
    verifier_bilans(&rc, titre);
    let bc = &rc.batiments[0];
    assert_eq!(bc.bbio_pts, b.bbio_pts);
    let gc = &bc.generations[0];
    let rev = &gc.generateurs[0];
    let (cop, eer) = (rev.eta_eff_ch_an.unwrap(), rev.eta_eff_fr_an.unwrap());
    // Valeurs pivot utiles : COP 3,2 à 7 °C, EER 2,7 à 35 °C ; par défaut, le COP tombe à 1,6 à
    // −7 °C et l'EER monte à 3,2 à 25 °C.
    assert!((2.0..3.6).contains(&cop), "{titre} : COP saisonnier {cop}");
    assert!((2.0..4.4).contains(&eer), "{titre} : EER saisonnier {eer}");
    assert!(rev.phi_rejet_kwh != 0.0);
    let cc = &bc.consommations.as_ref().unwrap().annuel;
    assert!(cc.refroidissement.electricite_kwh > 0.0);
    assert!(
        (cc.refroidissement.electricite_kwh - gc.consommations.annuel.refroidissement.total_kwh())
            .abs()
            < 1e-6
    );
    // Refroidissement en été seulement.
    let mensuel = &gc.consommations.mensuel;
    assert_eq!(mensuel[0].refroidissement.total_kwh(), 0.0);
    assert!(mensuel[6].refroidissement.total_kwh() > 0.0);
    println!(
        "{titre} : PAC air/air réversible, chauffage {:.1} kWh fournis pour {:.1} kWh d'électricité \
         (COP saisonnier {cop:.2}), froid {:.1} kWh pour {:.1} kWh (EER saisonnier {eer:.2}), \
         auxiliaires {:.1} kWh ; Bbio = {:.1} points, DH = {:.1} °C.h",
        rev.q_fou_ch_kwh,
        rev.consommations.chauffage.electricite_kwh,
        rev.q_fou_fr_kwh,
        rev.consommations.refroidissement.electricite_kwh,
        rev.w_aux_kwh,
        bc.bbio_pts.unwrap(),
        bc.dh_c_h.unwrap(),
    );
}

#[test]
fn generations_sur_la_meteo_ademe() {
    verifier(meteo_ademe, "ADEME");
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn generations_sur_la_meteo_reglementaire() {
    verifier(meteo_reglementaire, "réglementaire");
}
