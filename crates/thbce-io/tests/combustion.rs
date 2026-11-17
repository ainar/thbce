// SPDX-License-Identifier: GPL-3.0-or-later
//! Chaudières, poêles et inserts, sous-stations de réseaux (lot 6d) de bout en bout sur les
//! variantes de la maison d'exemple, sur la météo ouverte ADEME (CI publique) et sur la météo
//! réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test combustion -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les tests vérifient des propriétés (bilans fermés,
//! consommation d'un poêle égale à l'énergie fournie divisée par son rendement, énergies de la
//! matrice, Bbio indépendant des générateurs) et non des valeurs.

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

fn calcul(nom: &str, meteo_de: &dyn Fn(&Projet) -> MeteoProjet) -> Resultats {
    let p = cas(nom);
    let r = thbce_core::valider(&p);
    assert!(r.est_valide(), "{nom} : {:?}", r.erreurs);
    calculer(&p, &meteo_de(&p), &Options { arrondir: false }).unwrap()
}

/// Bilans : la génération consomme ce que ses générateurs consomment ; la demande est fournie ou
/// reportée ; les consommations du bâtiment sont celles de ses générations.
fn verifier_bilans(r: &Resultats, titre: &str) {
    let b = &r.batiments[0];
    let total: f64 = b
        .generations
        .iter()
        .map(|g| g.consommations.annuel.total_kwh())
        .sum();
    let cb = b.consommations.as_ref().unwrap();
    assert!((cb.annuel.total_kwh() - total).abs() < 1e-6, "{titre}");
    for g in &b.generations {
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
        let demande = g.q_ch_kwh + g.q_fr_kwh + g.q_ecs_kwh;
        let fourni = g.q_fou_ch_kwh + g.q_fou_fr_kwh + g.q_fou_ecs_kwh;
        assert!(
            (demande - fourni - g.q_rep_fin_kwh).abs() < 1e-6,
            "{titre} : {}",
            g.id
        );
    }
}

fn verifier(meteo_de: &dyn Fn(&Projet) -> MeteoProjet, titre: &str) {
    let reference = calcul("maison-plancher-ademe", meteo_de);
    let bbio_ref = reference.batiments[0].bbio_pts;

    // Chaudière gaz à condensation et ballon (§8.19).
    let r = calcul("maison-chaudiere-gaz-ademe", meteo_de);
    verifier_bilans(&r, titre);
    let b = &r.batiments[0];
    assert_eq!(b.bbio_pts, bbio_ref, "{titre} : le Bbio ne dépend pas des systèmes");
    let g = &b.generations[0];
    let c = &g.consommations.annuel;
    assert!(c.chauffage.gaz_kwh > g.q_fou_ch_kwh * 0.9, "{titre}");
    assert!(c.ecs.gaz_kwh > 0.0 && c.chauffage.electricite_kwh > 0.0, "{titre}");
    // Seules les cases gaz et électricité sont remplies.
    for p in [&c.chauffage, &c.ecs] {
        assert_eq!(p.fioul_kwh + p.bois_kwh + p.reseau_kwh + p.charbon_kwh, 0.0);
    }
    assert_eq!(c.refroidissement.total_kwh(), 0.0);
    let ch = &g.generateurs[0];
    // Électricité = auxiliaires de la chaudière.
    assert!(
        (c.chauffage.electricite_kwh + c.ecs.electricite_kwh - ch.w_aux_kwh).abs() < 1e-6,
        "{titre}"
    );
    // L'énergie fournie au ballon par la chaudière dépasse celle qu'il livre (pertes).
    let sto = &g.stockages[0];
    assert!(sto.q_fou_sto_base_kwh > sto.q_fou_ecs_kwh, "{titre}");
    assert!((ch.q_fou_ecs_kwh - sto.q_fou_sto_base_kwh).abs() < 1e-6, "{titre}");
    let eta = ch.eta_eff_ch_an.unwrap();
    assert!((0.7..1.15).contains(&eta), "{titre} : η = {eta}");
    println!(
        "{titre} : chaudière gaz : chauffage {:.1} kWh gaz + {:.1} kWh élec pour {:.1} kWh, η = {:.3} ; ECS {:.1} kWh gaz + {:.1} kWh élec pour {:.1} kWh au ballon, η = {:.3} ; pertes du ballon {:.1} kWh ; DH {:.1}",
        c.chauffage.gaz_kwh,
        c.chauffage.electricite_kwh,
        ch.q_fou_ch_kwh,
        eta,
        c.ecs.gaz_kwh,
        c.ecs.electricite_kwh,
        ch.q_fou_ecs_kwh,
        ch.eta_eff_ecs_an.unwrap(),
        sto.pertes_kwh,
        b.dh_c_h.unwrap_or(f64::NAN)
    );

    // Poêle à granulés et appoint électrique (§8.22).
    let r = calcul("maison-poele-granules-ademe", meteo_de);
    verifier_bilans(&r, titre);
    let b = &r.batiments[0];
    let g = &b.generations[0];
    let poele = &g.generateurs[0];
    let appoint = &g.generateurs[1];
    let c = &poele.consommations.chauffage;
    // Qcons = Qfou / η : les pertes sont proportionnelles à la charge (éq. 1228 à 1231).
    assert!((c.bois_kwh - poele.q_fou_ch_kwh / 0.88).abs() < 1e-6, "{titre}");
    // Auxiliaire : Σ τ × 35 W, avec τ = Qfou / 3 kW.
    assert!(
        (c.electricite_kwh - poele.q_fou_ch_kwh / 3.0 * 0.035).abs() < 1e-6,
        "{titre}"
    );
    assert!(appoint.q_fou_ch_kwh > 0.0, "{titre} : l'appoint intervient");
    assert!(
        (appoint.consommations.chauffage.electricite_kwh - appoint.q_fou_ch_kwh).abs() < 1e-6,
        "{titre}"
    );
    assert!(poele.q_fou_ch_kwh > appoint.q_fou_ch_kwh, "{titre}");
    println!(
        "{titre} : poêle : {:.1} kWh de bois (PCI) + {:.1} kWh élec pour {:.1} kWh, η = {:.3} ; appoint {:.1} kWh ; DH {:.1}",
        c.bois_kwh,
        c.electricite_kwh,
        poele.q_fou_ch_kwh,
        poele.eta_eff_ch_an.unwrap(),
        appoint.q_fou_ch_kwh,
        b.dh_c_h.unwrap_or(f64::NAN)
    );

    // Réseau de chaleur (§8.28).
    let r = calcul("maison-reseau-chaleur-ademe", meteo_de);
    verifier_bilans(&r, titre);
    let b = &r.batiments[0];
    let g = &b.generations[0];
    let c = &g.consommations.annuel;
    // Seules les pertes de sous-station s'ajoutent à l'énergie fournie ; ni auxiliaire ni perte
    // récupérable (éq. 1476, 1479 et 1480).
    assert!(c.chauffage.reseau_kwh > g.q_fou_ch_kwh, "{titre}");
    assert!(c.ecs.reseau_kwh > g.q_fou_ecs_kwh, "{titre}");
    assert_eq!(c.chauffage.electricite_kwh + c.ecs.electricite_kwh, 0.0);
    assert_eq!(c.chauffage.gaz_kwh + c.ecs.gaz_kwh, 0.0);
    assert_eq!(g.pertes_vc_kwh, 0.0, "{titre}");
    let x = &g.generateurs[0];
    println!(
        "{titre} : réseau de chaleur : chauffage {:.1} kWh pour {:.1} kWh, η = {:.3} ; ECS {:.1} kWh pour {:.1} kWh, η = {:.3} ; DH {:.1}",
        c.chauffage.reseau_kwh,
        g.q_fou_ch_kwh,
        x.eta_eff_ch_an.unwrap(),
        c.ecs.reseau_kwh,
        g.q_fou_ecs_kwh,
        x.eta_eff_ecs_an.unwrap(),
        b.dh_c_h.unwrap_or(f64::NAN)
    );
}

#[test]
fn combustion_sur_meteo_ademe() {
    verifier(&meteo_ademe, "ADEME");
}

#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn combustion_sur_meteo_reglementaire() {
    verifier(&meteo_reglementaire, "H2b réglementaire");
}

fn messages(p: &Projet) -> Vec<String> {
    thbce_core::valider(p)
        .erreurs
        .iter()
        .map(|e| e.message.clone())
        .collect()
}

#[test]
fn validation_des_generateurs_a_combustion() {
    let modifie = |cas_nom: &str, f: &dyn Fn(&mut serde_json::Value)| {
        let mut p = cas(cas_nom);
        let mut v = serde_json::to_value(&p.batiments[0].generations[0].generateurs[0].nature)
            .unwrap();
        f(&mut v);
        p.batiments[0].generations[0].generateurs[0].nature = serde_json::from_value(v).unwrap();
        messages(&p)
    };
    let m = modifie("maison-chaudiere-gaz-ademe", &|v| {
        v["chaudiere"]["type_chaudiere"] = "bois".into();
    });
    assert!(m.iter().any(|m| m.contains("TypeCombustibleBois")), "{m:?}");
    assert!(m.iter().any(|m| m.contains("Alim_Chaudière_Bois")), "{m:?}");
    let m = modifie("maison-chaudiere-gaz-ademe", &|v| {
        v["chaudiere"]["rendements"] =
            serde_json::json!({"statut": "certifie", "r_pn_pct": 101.0, "r_pint_pct": 95.0});
    });
    assert!(m.iter().any(|m| m.contains("égaux ou supérieurs à l'unité")), "{m:?}");
    let m = modifie("maison-reseau-chaleur-ademe", &|v| {
        v["reseau_de_chaleur_ou_de_froid"]["chaleur"]["identifiant_reseau"] = "0000C".into();
    });
    assert!(m.iter().any(|m| m.contains("absent de l'annexe 7")), "{m:?}");
    let m = modifie("maison-reseau-chaleur-ademe", &|v| {
        v["reseau_de_chaleur_ou_de_froid"]["chaleur"]["identifiant_reseau"] = "7501F".into();
    });
    assert!(m.iter().any(|m| m.contains("quatre chiffres")), "{m:?}");
    let m = modifie("maison-reseau-chaleur-ademe", &|v| {
        v["reseau_de_chaleur_ou_de_froid"]["chaleur"]["classe_isolation_secondaire"] = 5.into();
    });
    assert!(m.iter().any(|m| m.contains("classe d'isolation")), "{m:?}");
    // Un poêle alimente des réseaux fictifs, une chaudière des réseaux hydrauliques.
    let m = modifie("maison-plancher-ademe", &|v| {
        *v = serde_json::json!({"poele_ou_insert": {"pn_gen_kw": 5.0, "eta_pct": 80.0, "p_aux_vent_w": 0.0}});
    });
    assert!(m.iter().any(|m| m.contains("réseaux fictifs")), "{m:?}");
    let m = modifie("maison-climat-ademe", &|v| {
        *v = serde_json::json!({"chaudiere": {"type_chaudiere": "gaz_standard", "fonction": "chauffage", "pn_gen_kw": 8.0, "pertes_parois": "sans_ventilateur"}});
    });
    assert!(m.iter().any(|m| m.contains("réseaux hydrauliques")), "{m:?}");
}
