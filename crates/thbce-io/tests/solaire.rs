// SPDX-License-Identifier: GPL-3.0-or-later
//! Chauffe-eau solaires individuels (lot 7b, §9.14, §9.15 et §9.23) de bout en bout sur la
//! maison d'exemple équipée d'un CESI (`tests/cas/maison-cesi-ademe`), sur la météo ouverte
//! ADEME (CI publique) et sur la météo réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test solaire -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les tests vérifient des propriétés (bilan du
//! ballon, consommations, effet des masques et de l'orientation, Bbio indépendant de l'ECS).

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
use thbce_core::resultats::{Resultats, ResultatsGeneration, ResultatsStockage};
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

/// Modifie le chauffe-eau solaire de la maison d'exemple, décrit en JSON.
fn variante(base: &Projet, f: impl Fn(&mut serde_json::Value)) -> Projet {
    let mut p = base.clone();
    let c = &mut p.batiments[0].generations[1].chauffe_eau_solaires[0];
    let mut v = serde_json::to_value(&*c).unwrap();
    f(&mut v);
    *c = serde_json::from_value(v).unwrap();
    p
}

/// Génération d'ECS, ses consommations et son chauffe-eau solaire : `(Esol, appoint, pompe,
/// électricité d'ECS)`, en kWh.
fn bilan(g: &ResultatsGeneration, titre: &str) -> (f64, f64, f64, f64) {
    let st = &g.stockages[0];
    let sol = st.solaire.as_ref().unwrap();
    let elec = g.consommations.annuel.ecs.electricite_kwh;
    // Énergie solaire comprise dans l'énergie injectée par la base ; la somme des mois est
    // l'année.
    assert!(
        (sol.e_sol_kwh - st.q_fou_sto_base_kwh).abs() < 1e-6,
        "{titre}"
    );
    let mensuel: f64 = sol.e_sol_mensuel_kwh.iter().sum();
    assert!((mensuel - sol.e_sol_kwh).abs() < 1e-6, "{titre}");
    // Bilan du ballon : injections − pertes − puisage − écrêtement à la température maximale
    // (éq. 1779) = variation du stock entre le début et la fin du calcul.
    let ecart = st.q_fou_sto_base_kwh + st.q_fou_sto_ap_kwh
        - st.pertes_kwh
        - st.q_fou_ecs_kwh
        - sol.ecretement_kwh;
    let d_stock = variation_stock_kwh(st);
    assert!(
        (ecart - d_stock).abs() < 1e-6,
        "{titre} : {ecart} contre {d_stock}"
    );
    // Électricité d'ECS : la pompe de la boucle (éq. 1972) et les générateurs électriques à
    // rendement unitaire.
    let joule: f64 = g.generateurs.iter().map(|x| x.q_fou_ecs_kwh).sum();
    assert!((elec - joule - sol.w_pompe_kwh).abs() < 1e-6, "{titre}");
    (sol.e_sol_kwh, st.q_fou_sto_ap_kwh, sol.w_pompe_kwh, elec)
}

/// Variation du stock de chaleur d'un assemblage (un seul ballon) sur le calcul, en kWh :
/// `ρw·cw·Σ Vz·(θbz,fin − θbz,début)` (tableau 281 : ρw·cw = 1,163 Wh/(L.K)).
fn variation_stock_kwh(st: &ResultatsStockage) -> f64 {
    assert_eq!(st.v_zones_l.len(), 4);
    st.v_zones_l
        .iter()
        .zip(&st.theta_zones_debut_c)
        .zip(&st.theta_zones_fin_c)
        .map(|((v, d), f)| 1.163 * v * (f - d))
        .sum::<f64>()
        / 1000.0
}

fn verifier(meteo_de: impl Fn(&Projet) -> MeteoProjet, titre: &str) {
    let maison = cas("maison-cesi-ademe");
    let m = meteo_de(&maison);
    let r = brut(&maison, &m);
    let b = &r.batiments[0];
    let g = &b.generations[1];
    let (esol, ap, pompe, elec) = bilan(g, titre);
    let st = &g.stockages[0];
    let sol = st.solaire.as_ref().unwrap();
    // Le Bbio ne dépend pas de la production d'ECS.
    let reference = brut(&cas("maison-climat-ademe"), &m);
    let br = &reference.batiments[0];
    assert_eq!(b.bbio_pts, br.bbio_pts);
    let elec_ref = br.generations[1].consommations.annuel.ecs.electricite_kwh;
    // Ordres de grandeur : 4 m² de capteurs vitrés à valeurs par défaut au sud à 45°, de
    // l'ordre de 300 kWh/m² par an ; couverture solaire du ballon de 50 à 70 %.
    let couverture = esol / (esol + ap);
    assert!((150.0..450.0).contains(&(esol / 4.0)), "{titre} : {esol}");
    assert!((0.45..0.75).contains(&couverture), "{titre} : {couverture}");
    assert!(elec < 0.5 * elec_ref, "{titre}");
    assert!(sol.nbh_marche > 1000 && sol.nbh_marche < 3500);
    // Pas de report : l'appoint intégré, en fonctionnement permanent, maintient le haut du
    // ballon à la consigne.
    assert!(g.q_rep_fin_kwh < 1e-6 && st.nbh_report_e == 0, "{titre}");
    println!(
        "{titre} : CESI 4 m² sud 45°, ballon 300 L ; ensoleillement du plan {:.0} kWh/m², énergie \
         solaire {esol:.1} kWh ({:.0} kWh/m²), appoint {ap:.1} kWh, pompe {pompe:.1} kWh ({} h), \
         pertes du ballon {:.1} kWh, pertes de la boucle {:.1} kWh, écrêtement {:.1} kWh ; \
         couverture {:.1} % ; \
         électricité d'ECS {elec:.1} kWh contre {elec_ref:.1} kWh avec le ballon électrique de \
         l'exemple ; demande d'ECS {:.1} kWh ; DH {:.1} °C.h (exemple {:.1})",
        sol.isr_kwh_m2,
        esol / 4.0,
        sol.nbh_marche,
        st.pertes_kwh,
        sol.pertes_vc_kwh,
        sol.ecretement_kwh,
        100.0 * couverture,
        g.q_ecs_kwh,
        b.dh_c_h.unwrap(),
        br.dh_c_h.unwrap(),
    );

    // Orientation et masques (fiche 3.2) : capteurs au nord, ou derrière un relief.
    let nord = variante(&maison, |v| {
        v["exposition_capteurs"]["azimut_deg"] = 180.0.into()
    });
    let rn = brut(&nord, &m);
    let (esol_nord, ..) = bilan(&rn.batiments[0].generations[1], titre);
    assert!(esol_nord < 0.7 * esol, "{titre} : {esol_nord}");
    let mut masque = variante(&maison, |v| {
        v["exposition_capteurs"]["masques"] = serde_json::json!(["horizon-25"]);
    });
    masque.catalogue.masques.push(
        serde_json::from_value(serde_json::json!({"id": "horizon-25",
            "masque": {"lointain_azimutal": {"gamma_i_deg": vec![25.0; 36]}}}))
        .unwrap(),
    );
    let rm = brut(&masque, &m);
    let (esol_masque, ..) = bilan(&rm.batiments[0].generations[1], titre);
    assert!(esol_masque < esol, "{titre} : {esol_masque} {esol}");

    // Capteurs certifiés plus performants.
    let certifies = variante(&maison, |v| {
        v["description"]["composants"]["capteurs"]["performances"] =
            serde_json::json!({"certifiees": {"eta0": 0.78, "a1_w_m2k": 3.6, "a2_w_m2k2": 0.015}});
    });
    let rc = brut(&certifies, &m);
    let gc = &rc.batiments[0].generations[1];
    let (esol_c, ap_c, _, elec_c) = bilan(gc, titre);
    let ecretement_c = gc.stockages[0].solaire.as_ref().unwrap().ecretement_kwh;
    assert!(esol_c > esol && elec_c < elec);

    // Kit CESI (§9.23). « Kθ », sortie de l'outil IdCESI (§9.23.3.2.3), est requis : faute
    // de sortie IdCESI, 0,92, la valeur des capteurs plans vitrés de l'exemple (facteur
    // d'angle d'incidence courant d'un capteur plan vitré), saisie (lecture Q52).
    let kit = variante(&maison, |v| {
        v["description"] = serde_json::json!({"kit": {"surface_m2": 4.0, "v_tot_l": 300.0,
            "p_np_w": 40.0, "ac_etoile_m2": 2.4, "uc_etoile_w_m2k": 4.5, "us_w_k": 1.9,
            "f_aux": 0.4, "theta_max_c": 90.0, "k_theta": 0.92}});
        v["appoint"]["integre"]
            .as_object_mut()
            .unwrap()
            .remove("f_aux");
    });
    let rk = brut(&kit, &m);
    let gk = &rk.batiments[0].generations[1];
    let (esol_k, ap_k, ..) = bilan(gk, titre);
    assert!(!gk.stockages[0]
        .valeurs_par_defaut
        .iter()
        .any(|x| x.champ == "kit.k_theta"));

    // Appoint séparé instantané (« Type_prod_stockage = 3 ») : la résistance de la génération
    // complète l'eau préchauffée.
    let separe = variante(&maison, |v| v["appoint"] = "separe_instantane".into());
    let rs = brut(&separe, &m);
    let gs = &rs.batiments[0].generations[1];
    let (esol_s, ap_s, pompe_s, elec_s) = bilan(gs, titre);
    assert_eq!(ap_s, 0.0);
    assert!(gs.generateurs[0].q_fou_ecs_kwh > 0.0);
    assert!((gs.q_fou_ecs_kwh - gs.q_ecs_kwh + gs.q_rep_fin_kwh).abs() < 1e-6);
    println!(
        "{titre} : capteurs certifiés (η0 0,78, a1 3,6, a2 0,015) : {esol_c:.1} kWh solaires, \
         appoint {ap_c:.1} kWh, couverture {:.1} %, électricité {elec_c:.1} kWh, écrêtement \
         {ecretement_c:.1} kWh ; kit CESI (AC* \
         2,4 m², UC* 4,5, US 1,9, Kθ 0,92 saisi) : {esol_k:.1} kWh, appoint {ap_k:.1} kWh, \
         couverture {:.1} % ; appoint séparé instantané : {esol_s:.1} kWh solaires, appoint \
         instantané {:.1} kWh, pompe {pompe_s:.1} kWh, électricité {elec_s:.1} kWh ; capteurs \
         au nord {esol_nord:.1} kWh, masqués {esol_masque:.1} kWh",
        100.0 * esol_c / (esol_c + ap_c),
        100.0 * esol_k / (esol_k + ap_k),
        gs.generateurs[0].q_fou_ecs_kwh,
    );
}

#[test]
fn chauffe_eau_solaire_sur_la_meteo_ademe() {
    verifier(meteo_ademe, "ADEME");
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn chauffe_eau_solaire_sur_la_meteo_reglementaire() {
    verifier(meteo_reglementaire, "réglementaire");
}

#[test]
fn validation_des_chauffe_eau_solaires() {
    let messages = |p: &Projet| -> Vec<String> {
        valider(p)
            .erreurs
            .iter()
            .map(|e| format!("{} : {}", e.chemin, e.message))
            .collect()
    };
    let maison = cas("maison-cesi-ademe");
    assert!(messages(&maison).is_empty(), "{:?}", messages(&maison));
    // Capteurs inclinés au-delà de 90° (tableau 292), Kθ hors de [0 ; 1].
    let p = variante(&maison, |v| {
        v["exposition_capteurs"]["inclinaison_deg"] = 120.0.into();
        v["description"]["composants"]["capteurs"]["k_theta"] = 1.2.into();
    });
    let m = messages(&p);
    assert!(m.iter().any(|x| x.contains("inclinaison_deg")), "{m:?}");
    assert!(m.iter().any(|x| x.contains("k_theta")), "{m:?}");
    // Kit dont AC* dépasse 0,8·A (η0 > 1, éq. 2266), sans faux avec un appoint intégré.
    let p = variante(&maison, |v| {
        v["description"] = serde_json::json!({"kit": {"surface_m2": 2.0, "v_tot_l": 200.0,
            "p_np_w": 30.0, "ac_etoile_m2": 1.8, "uc_etoile_w_m2k": 4.0, "us_w_k": 1.5,
            "theta_max_c": 90.0, "k_theta": 0.92}});
    });
    let m = messages(&p);
    assert!(m.iter().any(|x| x.contains("ac_etoile_m2")), "{m:?}");
    assert!(m.iter().any(|x| x.contains("f_aux")), "{m:?}");
    // « Kθ » d'un kit requis (Q52) : absent, l'entrée ne se lit pas.
    let mut v =
        serde_json::to_value(&maison.batiments[0].generations[1].chauffe_eau_solaires[0]).unwrap();
    v["description"] = serde_json::json!({"kit": {"surface_m2": 4.0, "v_tot_l": 300.0,
        "p_np_w": 40.0, "ac_etoile_m2": 2.4, "uc_etoile_w_m2k": 4.5, "us_w_k": 1.9,
        "f_aux": 0.4, "theta_max_c": 90.0}});
    let e = serde_json::from_value::<thbce_core::entree::ChauffeEauSolaire>(v).unwrap_err();
    assert!(e.to_string().contains("k_theta"), "{e}");
    // Kit à appoint séparé instantané ou sans appoint : refusé (§9.23.3.2.2, ballon base seule
    // solaire avec appoint intégré).
    for appoint in ["separe_instantane", "aucun"] {
        let p = variante(&maison, |v| {
            v["description"] = serde_json::json!({"kit": {"surface_m2": 4.0, "v_tot_l": 300.0,
                "p_np_w": 40.0, "ac_etoile_m2": 2.4, "uc_etoile_w_m2k": 4.5, "us_w_k": 1.9,
                "theta_max_c": 90.0, "k_theta": 0.92}});
            v["appoint"] = appoint.into();
        });
        let m = messages(&p);
        assert!(
            m.iter()
                .any(|x| x.contains("un kit CESI est un ballon base seule")),
            "{appoint} : {m:?}"
        );
    }
    // Appoint séparé instantané sans générateur instantané d'ECS dans la génération.
    let mut p = variante(&maison, |v| v["appoint"] = "separe_instantane".into());
    p.batiments[0].generations[1].generateurs.clear();
    let m = messages(&p);
    assert!(
        m.iter().any(|x| x.contains("appoint séparé instantané")),
        "{m:?}"
    );
    // Appoint intégré qui n'est pas une résistance de la génération.
    let p = variante(&maison, |v| {
        v["appoint"]["integre"]["generateur"] = "inconnu".into();
    });
    assert!(messages(&p)
        .iter()
        .any(|x| x.contains("appoint.integre.generateur")));
    // Système solaire combiné : valide tel quel ; un appoint raccordé doit assurer le
    // chauffage et l'ECS ; la génération doit alimenter un réseau de chauffage.
    let ssc = cas("maison-ssc-ademe");
    assert!(messages(&ssc).is_empty(), "{:?}", messages(&ssc));
    let mut p = ssc.clone();
    p.batiments[0].generations[0].systemes_solaires_combines[0].raccordement =
        serde_json::from_str(r#""raccorde""#).unwrap();
    assert!(messages(&p)
        .iter()
        .any(|x| x.contains("assurant le chauffage et l'ECS")));
    // « θch_max^gen » (éq. 2117) : température de départ de dimensionnement requise.
    let mut p = ssc.clone();
    for d in &mut p.batiments[0].zones[0].groupes[0].distributions {
        if let thbce_core::entree::ReseauGroupe::Hydraulique(h) = &mut d.reseau {
            h.theta_dep_dim_c = None;
        }
    }
    assert!(messages(&p)
        .iter()
        .any(|x| x.contains("température de départ de dimensionnement")));
    let mut p = ssc;
    p.batiments[0].distributions_intergroupes.clear();
    p.batiments[0].zones[0].groupes[0].distributions.clear();
    assert!(messages(&p)
        .iter()
        .any(|x| x.contains("un système solaire combiné assure le chauffage")));
    // Génération réduite au chauffe-eau solaire, sans appoint : admise.
    let mut p = variante(&maison, |v| v["appoint"] = "aucun".into());
    p.batiments[0].generations[1].generateurs.clear();
    assert!(messages(&p).is_empty(), "{:?}", messages(&p));
}

/// Système solaire combiné (fiches 9.18 à 9.20) : bilans et contribution au chauffage.
fn verifier_ssc(meteo_de: impl Fn(&Projet) -> MeteoProjet, titre: &str) {
    let maison = cas("maison-ssc-ademe");
    let m = meteo_de(&maison);
    let r = brut(&maison, &m);
    let b = &r.batiments[0];
    let g = &b.generations[0];
    let st = &g.stockages[0];
    let sol = st.solaire.as_ref().unwrap();
    let ssc = st.ssc.unwrap();
    // Bilan du ballon : solaire + appoint = ECS + chauffage puisé + pertes + écrêtement +
    // variation du stock entre le début et la fin du calcul.
    let ecart = st.q_fou_sto_base_kwh + st.q_fou_sto_ap_kwh
        - st.q_fou_ecs_kwh
        - ssc.q_puise_ch_kwh
        - st.pertes_kwh
        - sol.ecretement_kwh;
    let d_stock = variation_stock_kwh(st);
    assert!(
        (ecart - d_stock).abs() < 1e-6,
        "{titre} : {ecart} contre {d_stock}"
    );
    // Le ballon contribue au chauffage ; l'énergie fournie en chauffage par la génération est
    // la chaleur puisée et celle des générateurs.
    assert!(ssc.q_puise_ch_kwh > 0.0, "{titre}");
    let gen_ch: f64 = g.generateurs.iter().map(|x| x.q_fou_ch_kwh).sum();
    assert!(
        (g.q_fou_ch_kwh - ssc.q_puise_ch_kwh - gen_ch).abs() < 1e-6,
        "{titre}"
    );
    assert!(
        (g.q_fou_ch_kwh + g.q_rep_fin_kwh - g.q_ch_kwh).abs() < 1e-6,
        "{titre}"
    );
    // Appoint indépendant : toute la consommation de l'appoint du ballon va à l'ECS (note 2 de
    // l'éq. 2135) ; la pompe est répartie entre les deux postes.
    let c = &g.consommations.annuel;
    let appoint = g
        .generateurs
        .iter()
        .find(|x| x.id == "resistance-ssc")
        .unwrap();
    assert!(
        (c.ecs.electricite_kwh - appoint.q_fou_ecs_kwh - sol.w_pompe_kwh).abs() < sol.w_pompe_kwh,
        "{titre}"
    );
    assert!(
        (c.ecs.electricite_kwh + c.chauffage.electricite_kwh
            - g.generateurs
                .iter()
                .map(|x| x.consommations.total_kwh())
                .sum::<f64>()
            - sol.w_pompe_kwh)
            .abs()
            < 1e-6,
        "{titre}"
    );
    let reference = brut(&cas("maison-plancher-ademe"), &m);
    let br = &reference.batiments[0];
    assert_eq!(b.bbio_pts, br.bbio_pts);
    let total = |r: &Resultats| {
        r.batiments[0]
            .consommations
            .as_ref()
            .unwrap()
            .annuel
            .total_kwh()
    };
    println!(
        "{titre} : SSC 10 m², 500 L, appoint indépendant ; énergie solaire {:.1} kWh, chauffage \
         puisé {:.1} kWh sur {:.1} kWh de demande ({:.1} %), ECS {:.1} kWh dont appoint {:.1} \
         kWh, pompe {:.1} kWh, pertes du ballon {:.1} kWh ; consommations {:.1} kWh contre {:.1} \
         kWh pour la variante à PAC et chauffe-eau thermodynamique ; DH {:.1} °C.h (variante \
         {:.1})",
        sol.e_sol_kwh,
        ssc.q_puise_ch_kwh,
        g.q_ch_kwh,
        100.0 * ssc.q_puise_ch_kwh / g.q_ch_kwh,
        st.q_fou_ecs_kwh,
        st.q_fou_sto_ap_kwh,
        sol.w_pompe_kwh,
        st.pertes_kwh,
        total(&r),
        total(&reference),
        b.dh_c_h.unwrap(),
        br.dh_c_h.unwrap(),
    );

    // Appoint raccordé (fiche 9.20) : la chaudière à condensation de la variante au gaz,
    // appelée pour l'ECS du ballon puis pour le chauffage restant (éq. 2150).
    let mut gaz = cas("maison-chaudiere-gaz-ademe");
    let ssc_saisi = maison.batiments[0].generations[0].systemes_solaires_combines[0].clone();
    {
        let gg = &mut gaz.batiments[0].generations[0];
        gg.stockages.clear();
        let mut v = serde_json::to_value(&ssc_saisi).unwrap();
        v["appoint"]["generateur"] = "chaudiere-gaz".into();
        v["raccordement"] = "raccorde".into();
        gg.systemes_solaires_combines
            .push(serde_json::from_value(v).unwrap());
    }
    let rg = brut(&gaz, &m);
    let gg = &rg.batiments[0].generations[0];
    let sg = gg.stockages[0].ssc.unwrap();
    let chaudiere = &gg.generateurs[0];
    assert!((sg.q_fou_ch_appoint_kwh - chaudiere.q_fou_ch_kwh).abs() < 1e-6);
    assert!(sg.q_puise_ch_kwh > 0.0);
    assert!((gg.q_fou_ch_kwh + gg.q_rep_fin_kwh - gg.q_ch_kwh).abs() < 1e-6);
    let ref_gaz = brut(&cas("maison-chaudiere-gaz-ademe"), &m);
    let gr = &ref_gaz.batiments[0].generations[0].consommations.annuel;
    let cg = &gg.consommations.annuel;
    assert!(cg.ecs.gaz_kwh < 0.6 * gr.ecs.gaz_kwh, "{titre}");
    println!(
        "{titre} : SSC à appoint raccordé (chaudière gaz) : chauffage puisé {:.1} kWh, chaudière \
         {:.1} kWh en chauffage ; gaz d'ECS {:.1} kWh contre {:.1} kWh avec le ballon de 200 L, gaz \
         de chauffage {:.1} kWh contre {:.1} kWh",
        sg.q_puise_ch_kwh,
        sg.q_fou_ch_appoint_kwh,
        cg.ecs.gaz_kwh,
        gr.ecs.gaz_kwh,
        cg.chauffage.gaz_kwh,
        gr.chauffage.gaz_kwh,
    );
}

#[test]
fn systeme_solaire_combine_sur_la_meteo_ademe() {
    verifier_ssc(meteo_ademe, "ADEME");
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn systeme_solaire_combine_sur_la_meteo_reglementaire() {
    verifier_ssc(meteo_reglementaire, "réglementaire");
}
