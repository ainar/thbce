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

/// La variante à plancher chauffant dont la pompe à chaleur air/eau, double service, réchauffe
/// aussi un ballon d'ECS de 200 L pourvu d'un appoint électrique intégré programmé de nuit.
fn double_service() -> Projet {
    let mut p = cas("maison-plancher-ademe");
    let b = &mut p.batiments[0];
    let mut v = serde_json::to_value(&b.generations[0]).unwrap();
    v["generateurs"][0]["nature"]["thermodynamique_electrique"]["ecs"] = serde_json::json!({
        "technologie": "air_exterieur_eau",
        "performances": {"pivot_declare": {"performance": 3.2, "pabs_kw": 1.6}}
    });
    v["generateurs"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id": "appoint-ballon", "rdim": 1,
            "nature": {"effet_joule": {"fonction": "ecs", "pn_gen_w": 1500}}
        }));
    v["stockages"] = serde_json::json!([{
        "id": "ballon", "nb_assemblages": 1,
        "ballon": {"v_tot_l": 200, "pertes": {"certifie": {"ua_s_w_k": 1.6}}, "theta_max_c": 70},
        "base": {"generateur": "pac-air-eau", "programmation": "permanent", "z_reg": 1,
                 "h_rel_ech": 0.4},
        "appoint": {"generateur": "appoint-ballon", "programmation": "nuit", "z_ap": 3,
                    "z_reg": 3}
    }]);
    b.generations[0] = serde_json::from_value(v).unwrap();
    b.generations.truncate(1);
    b.distributions_ecs_intergroupes[0].generation = "generation-eau-chaude".into();
    let r = thbce_core::valider(&p);
    assert!(r.est_valide(), "{:?}", r.erreurs);
    p
}

/// Électricité de chauffage de la première génération en Th-C, en kWh, selon la lecture des
/// pas de temps partagés avec l'ECS d'un générateur double service (Q47).
fn consommation_double_service(projet: &Projet, meteo: &MeteoProjet, ponderee: bool) -> f64 {
    use thbce_core::bati::assemblage_batiment::BatimentHoraire;
    use thbce_core::chauffage_refroidissement::calculs_generation::{Energie, Poste};
    use thbce_core::chauffage_refroidissement::generation::Generateur;
    use thbce_core::chauffage_refroidissement::thermodynamique_elec::LecturePasPartage;
    use thbce_core::climat::{ClimatExterieur, ConfortAdaptatif};
    use thbce_core::conventions::ModeCalcul;
    let loc = &projet.localisation;
    let climat = ClimatExterieur::nouveau(&meteo.th_bc, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    let mut b = BatimentHoraire::nouveau(
        &projet.batiments[0],
        &projet.catalogue,
        ModeCalcul::ThC,
        loc.alt_m,
        climat.we_kg_kg[0],
    )
    .unwrap();
    if ponderee {
        for g in &mut b.systemes.generations_horaires[0].generateurs {
            if let Generateur::Thermodynamique(pac) = &mut g.modele {
                pac.lecture_pas_partage = LecturePasPartage::PonderationTemporelle;
            }
        }
    }
    b.simuler(&climat, &confort).unwrap();
    b.systemes.generations_horaires[0]
        .cumuls
        .cef
        .annuel()
        .valeur(Poste::Chauffage, Energie::Electricite)
        / 1000.0
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
    // Ballon électrique (§9.9, §9.10 et §9.14) : la demande de la distribution est fournie par
    // le ballon ; la résistance, de rendement unitaire, consomme ce qu'elle injecte, c'est-à-dire
    // l'énergie puisée, les pertes du ballon et la variation de son stock.
    let ecs = &b.generations[1];
    assert!((ecs.q_ecs_kwh - b.distributions_ecs_intergroupes[0].q_w_prim_kwh).abs() < 1e-9);
    let ballon = &ecs.stockages[0];
    assert!((ballon.q_fou_ecs_kwh - ecs.q_fou_ecs_kwh).abs() < 1e-6);
    assert!(
        (ecs.q_fou_ecs_kwh + ecs.q_rep_fin_kwh - ecs.q_ecs_kwh).abs() < 1e-6,
        "{titre}"
    );
    let c_ecs = ecs.consommations.annuel.ecs.electricite_kwh;
    assert!((c_ecs - ballon.q_fou_sto_base_kwh).abs() < 1e-6);
    // Stock de 200 L : au plus 1,163 × 200 × 45 K, soit 10,5 kWh d'écart sur l'année.
    let ecart_stock = ballon.q_fou_sto_base_kwh - ballon.q_fou_ecs_kwh - ballon.pertes_kwh;
    assert!(ecart_stock.abs() < 10.5, "{titre} : {ecart_stock}");
    // Pertes : UA par défaut 2,31 W/K (tableau 282) sous 30 à 35 K d'écart, 600 à 700 kWh par an.
    assert!(
        (450.0..800.0).contains(&ballon.pertes_kwh),
        "{titre} : {ballon:?}"
    );
    assert!((ecs.pertes_vc_kwh - ballon.pertes_kwh).abs() < 1e-6);
    assert!(ballon.theta_haut_min_c > 45.0 && ballon.nbh_temp_insuff_max < 168);
    assert!(ballon
        .valeurs_par_defaut
        .iter()
        .any(|v| v.champ == "ballon.pertes.par_defaut"));
    // Un seul groupe : il porte toutes les consommations, par poste.
    let cg = b.zones[0].groupes[0].consommations.as_ref().unwrap();
    assert!((cg.annuel.chauffage.electricite_kwh - c.chauffage.electricite_kwh).abs() < 1e-6);
    assert!((cg.annuel.ecs.electricite_kwh - c_ecs).abs() < 1e-6);
    assert_eq!(cg.annuel.refroidissement.total_kwh(), 0.0);
    // Chauffage en hiver, pas en été ; ECS toute l'année.
    assert!(cg.mensuel[0].chauffage.electricite_kwh > 0.0);
    assert_eq!(cg.mensuel[6].chauffage.electricite_kwh, 0.0);
    assert!(cg.mensuel.iter().all(|m| m.ecs.electricite_kwh > 0.0));
    println!(
        "{titre} : maison d'exemple, Bbio = {:.1} points, DH = {:.1} °C.h ; chauffage {:.1} kWh \
         d'électricité (demande {:.1} kWh), ECS {:.1} kWh (demande {:.1} kWh, pertes du ballon {:.1} \
         kWh, zone haute au plus bas à {:.1} °C), sous-dimensionnement {} h au plus",
        b.bbio_pts.unwrap(),
        b.dh_c_h.unwrap(),
        c.chauffage.electricite_kwh,
        chauffage.q_ch_kwh,
        ecs.consommations.annuel.ecs.electricite_kwh,
        ecs.q_ecs_kwh,
        ballon.pertes_kwh,
        ballon.theta_haut_min_c,
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

    // Chauffe-eau thermodynamique de la variante (§9.22) : l'essai NF EN 16147 saisi est
    // reproduit par le modèle identifié ; la pompe à chaleur sur air extérieur consomme bien
    // moins qu'une résistance.
    let cet_gen = &bp.generations[1];
    let cet = &cet_gen.stockages[0];
    let id = cet.idcet.unwrap();
    assert!(id.epsilon < 1e-4, "{titre} : {id:?}");
    assert!(id.ua_s_w_k > 0.5 && id.ua_s_w_k < 4.0 && id.cop_pivot > 2.0 && id.cop_pivot < 4.0);
    let pac_ecs = &cet_gen.generateurs[0];
    let cop_ecs = pac_ecs.eta_eff_ecs_an.unwrap();
    assert!(
        (1.8..3.5).contains(&cop_ecs),
        "{titre} : COP du chauffe-eau {cop_ecs}"
    );
    let c_cet = cet_gen.consommations.annuel.ecs.electricite_kwh;
    assert!(c_cet < 0.6 * cet_gen.q_ecs_kwh, "{titre}");
    assert!((pac_ecs.q_fou_ecs_kwh - cet.q_fou_sto_base_kwh).abs() < 1e-6);
    println!(
        "{titre} : chauffe-eau thermodynamique, UAs = {:.3} W/K, COP pivot {:.3}, Pabs pivot {:.0} W \
         (ε = {:.1e}) ; ECS {:.1} kWh demandés, {:.1} kWh d'électricité, COP de la pompe à chaleur \
         {cop_ecs:.2}, pertes du ballon {:.1} kWh",
        id.ua_s_w_k,
        id.cop_pivot,
        1000.0 * id.pabs_pivot_kw,
        id.epsilon,
        cet_gen.q_ecs_kwh,
        c_cet,
        cet.pertes_kwh,
    );

    // Programmation de nuit du ballon électrique : la consigne conventionnelle de 55 °C et la
    // température de distribution de 53 °C ne laissent que 2 K de marge ; dès que la zone haute
    // passe sous 53 °C, les puisages sont reportés jusqu'à la nuit (lecture Q48).
    let mut nuit = maison.clone();
    nuit.batiments[0].generations[1].stockages[0]
        .base
        .programmation = serde_json::from_str(r#""nuit""#).unwrap();
    let rn = brut(&nuit, &m);
    verifier_bilans(&rn, titre);
    let gn = &rn.batiments[0].generations[1];
    assert!(gn.nbh_sousdim_ch_ecs_max > 6);
    assert!(gn.stockages[0].pertes_kwh < ballon.pertes_kwh);
    println!(
        "{titre} : ballon électrique programmé de nuit, ECS {:.1} kWh d'électricité, pertes {:.1} \
         kWh, demande reportée jusqu'à {} h de suite ; DH = {:.1} °C.h",
        gn.consommations.annuel.ecs.electricite_kwh,
        gn.stockages[0].pertes_kwh,
        gn.nbh_sousdim_ch_ecs_max,
        rn.batiments[0].dh_c_h.unwrap(),
    );

    // Pompe à chaleur double service : la pompe à chaleur air/eau de la variante à plancher
    // chauffant réchauffe aussi un ballon d'ECS, avec un appoint électrique intégré ; l'ECS est
    // prioritaire et le chauffage dispose du reste du pas de temps (éq. 1293).
    let double = double_service();
    let rd = brut(&double, &m);
    verifier_bilans(&rd, titre);
    let gd = &rd.batiments[0].generations[0];
    let pac_d = &gd.generateurs[0];
    assert!(pac_d.q_fou_ecs_kwh > 0.0 && pac_d.q_fou_ch_kwh > 0.0);
    assert!(gd.stockages[0].q_fou_sto_ap_kwh >= 0.0);
    let litterale = consommation_double_service(&double, &m, false);
    let ponderee = consommation_double_service(&double, &m, true);
    assert!((litterale - gd.consommations.annuel.chauffage.electricite_kwh).abs() < 1e-6);
    assert!(ponderee <= litterale);
    println!(
        "{titre} : PAC double service, chauffage {:.1} kWh fournis (COP {:.2}), ECS {:.1} kWh fournis \
         au ballon (COP {:.2}), appoint du ballon {:.1} kWh ; électricité de chauffage {litterale:.1} \
         kWh avec la lecture littérale des pas partagés, {ponderee:.1} kWh avec la pondération \
         temporelle",
        pac_d.q_fou_ch_kwh,
        pac_d.eta_eff_ch_an.unwrap(),
        pac_d.q_fou_ecs_kwh,
        pac_d.eta_eff_ecs_an.unwrap(),
        gd.stockages[0].q_fou_sto_ap_kwh,
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
