// SPDX-License-Identifier: GPL-3.0-or-later
//! Productions solaires collectives à appoints décentralisés (lot 7c, fiches 9.16 et 9.17) de
//! bout en bout sur l'immeuble d'exemple équipé d'un CESCI (`tests/cas/immeuble-cesci-ademe`),
//! sur la météo ouverte ADEME (CI publique) et sur la météo réglementaire (test ignoré par
//! défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test solaire_collectif -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les tests vérifient des propriétés (bilans exacts
//! des ballons, consommations, ordres de grandeur) et comparent à l'immeuble d'exemple et à
//! son ballon collectif électrique de 1 000 L (`tests/cas/immeuble-collectif-ademe`).

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
use thbce_core::resultats::{
    Resultats, ResultatsGeneration, ResultatsSolaireCollectif, ResultatsStockage,
};
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

/// Indice de la génération d'ECS de l'immeuble.
const GEN: usize = 1;

/// Modifie la génération d'ECS de l'immeuble, décrite en JSON.
fn variante(base: &Projet, f: impl Fn(&mut serde_json::Value)) -> Projet {
    let mut p = base.clone();
    let mut v = serde_json::to_value(&p.batiments[0]).unwrap();
    f(&mut v);
    p.batiments[0] = serde_json::from_value(v).unwrap();
    p
}

/// Variation du stock de chaleur de `nb` ballons identiques, en kWh.
fn variation_stock_kwh(st: &ResultatsStockage, nb: f64) -> f64 {
    st.v_zones_l
        .iter()
        .zip(&st.theta_zones_debut_c)
        .zip(&st.theta_zones_fin_c)
        .map(|((v, d), f)| 1.163 * v * (f - d))
        .sum::<f64>()
        * nb
        / 1000.0
}

/// Bilans d'une production collective : `(Esol, appoints, pompe, électricité d'ECS)`, en kWh.
fn bilan(g: &ResultatsGeneration, nb_ballons: f64, titre: &str) -> (f64, f64, f64, f64) {
    let c: &ResultatsSolaireCollectif = &g.solaires_collectifs[0];
    let sol = &c.solaire;
    let mensuel: f64 = sol.e_sol_mensuel_kwh.iter().sum();
    assert!((mensuel - sol.e_sol_kwh).abs() < 1e-6, "{titre}");
    let ens = &c.ensembles[0];
    // Bilan exact des ballons individuels : énergie injectée en zone basse (boucle ou réseau)
    // + appoint − pertes − puisage − écrêtement = variation du stock.
    let ecretement_dec = if c.ballon_central.is_some() {
        0.0
    } else {
        sol.ecretement_kwh
    };
    let ecart = ens.q_fou_sto_base_kwh + ens.q_fou_sto_ap_kwh
        - ens.pertes_kwh
        - ens.q_fou_ecs_kwh
        - ecretement_dec;
    let d = variation_stock_kwh(ens, nb_ballons);
    assert!((ecart - d).abs() < 1e-6, "{titre} : {ecart} contre {d}");
    match &c.ballon_central {
        // CESCI : l'énergie solaire est celle injectée dans les ballons individuels.
        None => assert!(
            (sol.e_sol_kwh - ens.q_fou_sto_base_kwh).abs() < 1e-6,
            "{titre}"
        ),
        // CESCAI : bilan exact du ballon central ; les ballons individuels reçoivent l'énergie
        // du réseau intergroupe.
        Some(b) => {
            assert!((ens.q_fou_sto_base_kwh - c.q_fou_reseau_kwh).abs() < 1e-6);
            let ecart = sol.e_sol_kwh - b.pertes_kwh - b.q_fou_ecs_kwh - sol.ecretement_kwh;
            let d = variation_stock_kwh(b, 1.0);
            assert!((ecart - d).abs() < 1e-6, "{titre} : {ecart} contre {d}");
            assert!(
                (b.q_fou_ecs_kwh - c.q_fou_reseau_kwh - c.pertes_reseau_deduites_kwh).abs() < 1e-6
            );
        }
    }
    // Électricité d'ECS : appoints électriques à rendement unitaire et pompe de la boucle.
    let joule: f64 = g.generateurs.iter().map(|x| x.q_fou_ecs_kwh).sum();
    let elec = g.consommations.annuel.ecs.electricite_kwh;
    assert!((elec - joule - sol.w_pompe_kwh).abs() < 1e-6, "{titre}");
    // Toute la demande est fournie.
    assert!(
        (g.q_fou_ecs_kwh + g.q_rep_fin_kwh - g.q_ecs_kwh).abs() < 1e-6,
        "{titre}"
    );
    (sol.e_sol_kwh, joule, sol.w_pompe_kwh, elec)
}

fn verifier(meteo_de: impl Fn(&Projet) -> MeteoProjet, titre: &str) {
    let immeuble = cas("immeuble-cesci-ademe");
    let m = meteo_de(&immeuble);
    let r = brut(&immeuble, &m);
    let b = &r.batiments[0];
    let g = &b.generations[GEN];
    let (esol, appoint, pompe, elec) = bilan(g, 12.0, titre);
    let c = &g.solaires_collectifs[0];
    let reference = brut(&cas("immeuble-collectif-ademe"), &m);
    let br = &reference.batiments[0];
    // Le Bbio ne dépend pas de la production d'ECS.
    assert_eq!(b.bbio_pts, br.bbio_pts);
    let gr = &br.generations[GEN];
    let elec_ref = gr.consommations.annuel.ecs.electricite_kwh;
    // Ordres de grandeur : 18 m² de capteurs vitrés aux valeurs par défaut (η0 = 0,6,
    // a1 = 6 W/(m².K)), 1,5 m² par logement, au sud à 45° ; un champ collectif chargé toute
    // l'année produit de l'ordre de 300 à 500 kWh/m² par an et couvre de 30 à 60 % des
    // besoins d'ECS.
    let productivite = esol / 18.0;
    let couverture = esol / g.q_ecs_kwh;
    assert!(
        (250.0..550.0).contains(&productivite),
        "{titre} : {productivite}"
    );
    assert!((0.3..0.6).contains(&couverture), "{titre} : {couverture}");
    assert!(elec < elec_ref, "{titre}");
    println!(
        "{titre} : CESCI 18 m² sud 45°, 12 ballons de 150 L ; ensoleillement du plan {:.0} \
         kWh/m², énergie solaire {esol:.1} kWh ({productivite:.0} kWh/m²), appoints \
         {appoint:.1} kWh, pompe {pompe:.1} kWh ({} h), pertes des ballons {:.1} kWh, pertes de \
         la boucle {:.1} kWh ; demande d'ECS {:.1} kWh, couverture solaire des besoins {:.1} % \
         (du ballon {:.1} %) ; électricité d'ECS {elec:.1} kWh contre {elec_ref:.1} kWh avec le \
         ballon collectif électrique de 1 000 L ({:.1} kWh de pertes) ; DH {:.1} °C.h \
         (exemple {:.1})",
        c.solaire.isr_kwh_m2,
        c.solaire.nbh_marche,
        c.ensembles[0].pertes_kwh,
        c.solaire.pertes_vc_kwh,
        g.q_ecs_kwh,
        100.0 * couverture,
        100.0 * esol / (esol + appoint),
        gr.stockages[0].pertes_kwh,
        b.dh_c_h.unwrap(),
        br.dh_c_h.unwrap(),
    );

    // CESCI à appoint séparé instantané (« Type_PCAD = 2 ») : les résistances, générateurs
    // instantanés d'ECS de la génération, complètent l'eau préchauffée.
    let separe = variante(&immeuble, |v| {
        let c = &mut v["generations"][GEN]["chauffe_eau_solaires_collectifs"][0];
        c["type_pcad"] = "cesci_appoint_separe".into();
        c["ensembles"][0].as_object_mut().unwrap().remove("appoint");
    });
    assert!(
        valider(&separe).erreurs.is_empty(),
        "{:?}",
        valider(&separe).erreurs
    );
    let rs = brut(&separe, &m);
    let gs = &rs.batiments[0].generations[GEN];
    let (esol_s, appoint_s, _, elec_s) = bilan(gs, 12.0, titre);
    assert_eq!(gs.solaires_collectifs[0].ensembles[0].q_fou_sto_ap_kwh, 0.0);

    // CESCAI : ballon solaire central de 900 L, réseau intergroupe bouclé de 40 m en volume
    // chauffé, ballons individuels de 150 L à appoint électrique intégré.
    let cescai = variante(&immeuble, |v| {
        let c = &mut v["generations"][GEN]["chauffe_eau_solaires_collectifs"][0];
        c["type_pcad"] = "cescai".into();
        c["boucle"]["regulation"] = "temperature".into();
        c["ballon_central"] = serde_json::json!({"v_tot_l": 900.0,
            "pertes": {"par_defaut": {"nature": "autre"}}, "theta_max_c": 90.0});
        v["distributions_ecs_intergroupes"][0]["reseau"] = serde_json::json!({"boucle": {
            "l_vc_m": 40.0, "l_hvc_m": 0.0, "u_w_mk": 0.25, "rechauffeur": false,
            "p_circ_w": 60.0}});
    });
    assert!(
        valider(&cescai).erreurs.is_empty(),
        "{:?}",
        valider(&cescai).erreurs
    );
    let rc = brut(&cescai, &m);
    let gc = &rc.batiments[0].generations[GEN];
    let (esol_c, appoint_c, _, elec_c) = bilan(gc, 12.0, titre);
    let cc = &gc.solaires_collectifs[0];
    assert!(cc.q_fou_reseau_kwh > 0.0 && cc.pertes_reseau_deduites_kwh > 0.0);
    println!(
        "{titre} : CESCI à appoint séparé : {esol_s:.1} kWh solaires, appoint instantané \
         {appoint_s:.1} kWh, électricité {elec_s:.1} kWh ; CESCAI (ballon central de 900 L, \
         boucle de 40 m) : {esol_c:.1} kWh solaires, {:.1} kWh fournis par le réseau, pertes du \
         réseau déduites {:.1} kWh et non couvertes {:.1} kWh, pertes du ballon central {:.1} \
         kWh, appoints {appoint_c:.1} kWh, électricité {elec_c:.1} kWh (circulateur compris \
         dans les consommations de la distribution)",
        cc.q_fou_reseau_kwh,
        cc.pertes_reseau_deduites_kwh,
        cc.pertes_reseau_non_couvertes_kwh,
        cc.ballon_central.as_ref().unwrap().pertes_kwh,
    );
}

#[test]
fn solaire_collectif_sur_la_meteo_ademe() {
    verifier(meteo_ademe, "ADEME");
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn solaire_collectif_sur_la_meteo_reglementaire() {
    verifier(meteo_reglementaire, "réglementaire");
}

#[test]
fn validation_des_productions_collectives() {
    let messages = |p: &Projet| -> Vec<String> {
        valider(p)
            .erreurs
            .iter()
            .map(|e| format!("{} : {}", e.chemin, e.message))
            .collect()
    };
    let immeuble = cas("immeuble-cesci-ademe");
    assert!(messages(&immeuble).is_empty(), "{:?}", messages(&immeuble));
    let c = "/generations/1/chauffe_eau_solaires_collectifs/0";
    let avec =
        |f: &dyn Fn(&mut serde_json::Value)| variante(&immeuble, |v| f(v.pointer_mut(c).unwrap()));
    // Éq. (1981) : boucle d'un CESCI régulée sur l'ensoleillement.
    let p = avec(&|x| x["boucle"]["regulation"] = "temperature".into());
    assert!(messages(&p).iter().any(|m| m.contains("ensoleillement")));
    // CESCAI sans ballon central, et réseau intergroupe non bouclé.
    let p = avec(&|x| x["type_pcad"] = "cescai".into());
    let m = messages(&p);
    assert!(
        m.iter().any(|x| x.contains("ballon central requis")),
        "{m:?}"
    );
    assert!(
        m.iter().any(|x| x.contains("réseau intergroupe bouclé")),
        "{m:?}"
    );
    // Nombre de ballons différent du `rdim` de l'appoint.
    let p = avec(&|x| x["ensembles"][0]["nb_ballons"] = 10.into());
    assert!(messages(&p).iter().any(|m| m.contains("rdim")));
    // Appoint séparé avec un appoint intégré saisi.
    let p = avec(&|x| x["type_pcad"] = "cesci_appoint_separe".into());
    assert!(messages(&p)
        .iter()
        .any(|m| m.contains("appoint intégré sans objet")));
    // Appoint qui n'assure pas l'ECS.
    let p = avec(&|x| x["ensembles"][0]["appoint"]["generateur"] = "inconnu".into());
    assert!(messages(&p)
        .iter()
        .any(|m| m.contains("ensembles[0].appoint.generateur")));
    // Un CESCI dessert directement les distributions du groupe.
    let p = variante(&immeuble, |v| {
        v["distributions_ecs_intergroupes"][0]["reseau"] = serde_json::json!({"boucle": {
            "l_vc_m": 40.0, "l_hvc_m": 0.0, "u_w_mk": 0.25, "rechauffeur": false,
            "p_circ_w": 60.0}});
    });
    assert!(messages(&p)
        .iter()
        .any(|m| m.contains("intergroupe `aucun`")));
}
