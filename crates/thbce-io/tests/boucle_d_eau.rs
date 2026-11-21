// SPDX-License-Identifier: GPL-3.0-or-later
//! Boucles d'eau (fiche 8.15, lot 2f) de bout en bout : exemple `hotel-boucle-d-eau-ademe`
//! (pompes à chaleur sur boucle d'eau dans les deux zones d'un hôtel, pompes à chaleur air / eau
//! et tour de refroidissement de la boucle) sur la météo ouverte ADEME (CI publique) et sur la météo
//! réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo.csv cargo test -p thbce-io --test boucle_d_eau -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les tests vérifient un pas de temps à la main
//! (transfert entre un groupe en chauffage et un groupe en refroidissement, éq. 957 à 977), les
//! bilans annuels de la boucle, le sens des effets et les refus de la validation.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::chauffage_refroidissement::calculs_generation::Poste;
use thbce_core::chauffage_refroidissement::generation::{
    ConditionsSource, EntreesGeneration, GenerationHoraire, ReseauVu,
};
use thbce_core::chauffage_refroidissement::transferts_entre_generateurs_sur_boucle::{
    DemandeGroupe, R_EFF_TR_BE,
};
use thbce_core::climat::MeteoProjet;
use thbce_core::entree::Projet;
use thbce_core::resultats::{Resultats, ResultatsGeneration};
use thbce_core::{calculer, Options};
use thbce_io::{json, meteo};

const EXEMPLE: &str = "hotel-boucle-d-eau-ademe";

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json_du_cas(nom: &str) -> serde_json::Value {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    serde_json::from_str(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn projet(v: &serde_json::Value) -> Projet {
    json::lire_projet(&v.to_string()).unwrap()
}

fn meteo_ademe(p: &Projet) -> MeteoProjet {
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    meteo::lire(
        &std::fs::read(chemin).unwrap(),
        p.localisation.zone_climatique_retenue(),
    )
    .unwrap()
}

fn calcul(v: &serde_json::Value) -> Resultats {
    let p = projet(v);
    let r = thbce_core::valider(&p);
    assert!(r.est_valide(), "{:?}", r.erreurs);
    calculer(&p, &meteo_ademe(&p), &Options { arrondir: false }).unwrap()
}

fn boucle(v: &mut serde_json::Value) -> &mut serde_json::Value {
    &mut v["batiments"][0]["generations"][0]
}

fn reseau(poste: Poste, q_sys_wh: f64, theta_c: f64) -> ReseauVu {
    ReseauVu {
        poste,
        hydraulique: false,
        q_sys_wh,
        theta_moy_c: theta_c,
        theta_dep_c: theta_c,
        theta_i_aval_eq_c: theta_c,
        relance: false,
        rat_surf_dess: 1.0,
        phi_pertes_prim_wh: 0.0,
    }
}

/// Un pas de temps d'avril (`θbe` = 24 + 6 sin(2π·4/12 + 7π/6) = 21 °C, éq. 1437 et 1438) : la
/// partie nuit demande 20 kWh de chauffage, la partie jour 8 kWh de refroidissement ; la tour,
/// passée en tour sèche, sort l'eau à max(10 + 5 ; 16) = 16 °C (éq. 1434 et 1435).
#[test]
fn transfert_entre_groupes_a_la_main() {
    let mut v = json_du_cas(EXEMPLE);
    boucle(&mut v)["boucle_d_eau"]["tour"]["nature"] = "seche".into();
    let p = projet(&v);
    let b = &p.batiments[0];
    let mut gen = GenerationHoraire::resoudre(&b.generations[0], &[], &[], &p.catalogue);
    let ids: Vec<&str> = gen.generateurs.iter().map(|g| g.id.as_str()).collect();
    assert_eq!(ids, ["pac-nuit", "pac-jour", "pac-air-eau-boucle", "tour"]);
    let reseaux = [
        reseau(Poste::Chauffage, 20_000.0, 20.0),
        reseau(Poste::Refroidissement, 8000.0, 26.0),
    ];
    let sources = [ConditionsSource {
        theta_amont_c: 21.0,
        air_extrait_w_k: None,
    }; 4];
    // Saisons ouvertes des deux côtés : la partie nuit, sans demande de froid, chauffe ; la
    // partie jour refroidit (§8.15.3.3).
    let demandes = [
        DemandeGroupe {
            q_ch_wh: 20_000.0,
            q_fr_wh: 0.0,
            aut_ch: true,
            aut_fr: true,
        },
        DemandeGroupe {
            q_ch_wh: 0.0,
            q_fr_wh: 8000.0,
            aut_ch: true,
            aut_fr: true,
        },
    ];
    let e = EntreesGeneration {
        reseaux: &reseaux,
        aut_ch: true,
        aut_fr: true,
        id_encl: true,
        theta_ext_c: 10.0,
        b_therm: 1.0,
        w_rechauf_wh: 0.0,
        theta_cw_c: 12.0,
        hleg: 12,
        sources: &sources,
        climat: None,
        h: 2400,
        w_ext_kg_kg: 0.005,
        demandes_groupes: &demandes,
    };
    let s = gen.pas(&e, 3).clone();
    let g = &gen.generateurs;
    // Éq. (961) et (964) : chaque groupe n'a qu'une PAC, qui reçoit toute la demande ; elle
    // la couvre (10 et 4 machines de 7,4 et 6,7 kW).
    assert!((g[0].jeu.q_fou_wh[0] - 20_000.0).abs() < 1e-6);
    assert!((g[1].jeu.q_fou_wh[1] - 8000.0).abs() < 1e-6);
    assert_eq!(g[0].jeu.q_fou_wh[1], 0.0);
    assert!(g[1].jeu.ia_refroidi && !g[0].jeu.ia_refroidi);
    // Énergie fournie aux réseaux : celle des PAC seules (la chaudière et la tour
    // fournissent la boucle).
    assert!((s.q_fou_wh[0] - 20_000.0).abs() < 1e-6);
    assert!((s.q_fou_wh[1] - 8000.0).abs() < 1e-6);
    assert_eq!(s.q_rep_wh, [0.0, 0.0, 0.0]);
    // Éq. (965) à (968) avec les rejets des PAC.
    let phi_ch = -g[0].jeu.phi_rejet_wh;
    let phi_fr = g[1].jeu.phi_rejet_wh;
    assert!(phi_ch > 0.0 && phi_fr > 0.0);
    let transfert = phi_ch.min(phi_fr) * R_EFF_TR_BE;
    let c = gen.boucle.as_ref().unwrap().cumuls;
    assert!((c.q_transfert_wh - transfert).abs() < 1e-9);
    assert!((c.q_sys_ch_wh - (phi_ch - transfert)).abs() < 1e-9);
    assert!((c.q_sys_fr_wh - (phi_fr - transfert)).abs() < 1e-9);
    // Le rejet de chaleur des PAC en refroidissement (8 000 Wh + consommation) est plus petit
    // que le froid rejeté en chauffage (20 000 Wh − consommation).
    assert!(phi_fr < phi_ch && c.q_sys_fr_wh > 0.0);
    // Éq. (969) à (971) : les PAC air / eau de la boucle fournissent tout `Qsys_BE_ch`.
    assert!((g[2].jeu.q_fou_wh[0] - c.q_sys_ch_wh).abs() < 1e-6);
    assert!(g[2].jeu.q_fou_wh[0] > 0.0);
    // Éq. (972) à (974) : Pmax = 30/3 600 × 1 000 × 4 180 × (21 − 16) = 174 166,7 W ;
    // τ = Qsys_BE_fr / Pmax ; consommation (400 + 600) × τ, en électricité, au refroidissement.
    let p_max = 30.0 / 3600.0 * 1000.0 * 4180.0 * 5.0;
    let tau = c.q_sys_fr_wh / p_max;
    assert!((g[3].jeu.q_fou_wh[1] - c.q_sys_fr_wh).abs() < 1e-9);
    assert!((g[3].jeu.tau_charge - tau).abs() < 1e-12);
    let elec_fr = g[3].jeu.q_cef.total_poste(Poste::Refroidissement);
    assert!((elec_fr - 1000.0 * tau).abs() < 1e-9, "{elec_fr}");
}

fn generation(r: &Resultats) -> &ResultatsGeneration {
    &r.batiments[0].generations[0]
}

#[test]
fn hotel_a_boucle_d_eau_bilans_annuels() {
    let v = json_du_cas(EXEMPLE);
    let r = calcul(&v);
    let g = generation(&r);
    let b = g.boucle_d_eau.as_ref().unwrap();
    let x = |id: &str| g.generateurs.iter().find(|x| x.id == id).unwrap();
    // Énergie fournie aux réseaux : celle des PAC seules.
    let pac_ch = x("pac-nuit").q_fou_ch_kwh + x("pac-jour").q_fou_ch_kwh;
    let pac_fr = x("pac-nuit").q_fou_fr_kwh + x("pac-jour").q_fou_fr_kwh;
    assert!((g.q_fou_ch_kwh - pac_ch).abs() < 1e-6);
    assert!((g.q_fou_fr_kwh - pac_fr).abs() < 1e-6);
    // Chaque heure, une PAC chauffe ou refroidit : la somme de ses rejets est la différence
    // des rejets totaux des éq. (965) et (966).
    let rejets = x("pac-nuit").phi_rejet_kwh + x("pac-jour").phi_rejet_kwh;
    assert!((rejets - (b.phi_rejet_tot_fr_kwh - b.phi_rejet_tot_ch_kwh)).abs() < 1e-6);
    // Éq. (968) : sans transfert (les deux parties changent de saison les mêmes jours),
    // `Qsys_BE` égale les rejets ; la chaudière et la tour les fournissent, au report près.
    assert_eq!(b.q_transfert_kwh, 0.0);
    assert!((b.q_sys_ch_kwh - b.phi_rejet_tot_ch_kwh).abs() < 1e-6);
    let fournie = x("pac-air-eau-boucle").q_fou_ch_kwh + x("tour").q_fou_fr_kwh;
    assert!((b.q_sys_ch_kwh + b.q_sys_fr_kwh - fournie - b.q_rep_fin_kwh).abs() < 1e-6);
    // La tour ne consomme que de l'électricité de refroidissement, au plus 1 kW.
    let tour = &x("tour").consommations;
    assert!(tour.refroidissement.electricite_kwh > 0.0);
    assert!(tour.refroidissement.electricite_kwh <= 8760.0);
    assert_eq!(tour.chauffage.electricite_kwh, 0.0);
    // Éq. (1490) et (1491) : les consommations d'une PAC sur boucle d'eau vont à son groupe
    // (la partie jour n'a qu'un groupe, desservi par `pac-jour`) ; celles de la tour, au
    // prorata des demandes de froid.
    let zone = |id: &str| {
        r.batiments[0]
            .zones
            .iter()
            .find(|z| z.id == id)
            .unwrap()
            .consommations
            .as_ref()
            .unwrap()
            .annuel
            .clone()
    };
    let (jour, nuit) = (zone("jour"), zone("nuit"));
    let pac_jour = &x("pac-jour").consommations;
    let tour_fr = x("tour").consommations.refroidissement.electricite_kwh;
    assert!(jour.refroidissement.electricite_kwh >= pac_jour.refroidissement.electricite_kwh);
    assert!(
        jour.refroidissement.electricite_kwh
            <= pac_jour.refroidissement.electricite_kwh + tour_fr + 1e-6
    );
    assert!(
        (jour.refroidissement.electricite_kwh + nuit.refroidissement.electricite_kwh
            - g.consommations.annuel.refroidissement.electricite_kwh)
            .abs()
            < 1e-6
    );
    // Les PAC air / eau de la boucle consomment de l'électricité de chauffage.
    let pac_boucle = x("pac-air-eau-boucle");
    assert!(pac_boucle.consommations.chauffage.electricite_kwh > 0.0);
    assert!(pac_boucle.consommations.chauffage.electricite_kwh < pac_boucle.q_fou_ch_kwh);

    // Sens des effets : une boucle plus chaude améliore le COP des PAC en chauffage et
    // ses PAC consomment moins.
    let mut chaude = v.clone();
    for k in 0..2 {
        chaude["batiments"][0]["generations"][0]["generateurs"][k]["nature"]
            ["pac_sur_boucle_d_eau"]["pac"]["source_amont"]["boucle_d_eau"]["boucle"]
            ["theta_min_c"] = 22.0.into();
    }
    let rc = calcul(&chaude);
    let gc = generation(&rc);
    let cons = |g: &ResultatsGeneration, id: &str| {
        g.generateurs
            .iter()
            .find(|x| x.id == id)
            .unwrap()
            .consommations
            .chauffage
            .electricite_kwh
    };
    assert!(cons(gc, "pac-nuit") < cons(g, "pac-nuit"));
    // Une tour plus petite reporte du froid d'une heure à l'autre.
    let mut petite = v.clone();
    boucle(&mut petite)["boucle_d_eau"]["tour"]["qv_nom_m3_h"] = 3.0.into();
    let rp = calcul(&petite);
    let tp = generation(&rp)
        .generateurs
        .iter()
        .find(|x| x.id == "tour")
        .unwrap();
    assert!(tp.q_fou_fr_kwh <= x("tour").q_fou_fr_kwh + 1e-6);
    assert!(generation(&rp).nbh_sousdim_fr_max >= g.nbh_sousdim_fr_max);
}

#[test]
fn validation_de_la_boucle_d_eau() {
    let v = json_du_cas(EXEMPLE);
    let rapport = |v: &serde_json::Value| thbce_core::valider(&projet(v));
    let messages = |v: &serde_json::Value| -> Vec<String> {
        rapport(v)
            .erreurs
            .iter()
            .map(|e| format!("{} : {}", e.chemin, e.message))
            .collect()
    };
    assert!(messages(&v).is_empty(), "{:?}", messages(&v));
    // Sans description de la boucle.
    let mut x = v.clone();
    boucle(&mut x).as_object_mut().unwrap().remove("boucle_d_eau");
    assert!(messages(&x).iter().any(|m| m.contains("§8.15.3.1")));
    // Sans générateur de chauffage de la boucle alors que les PAC chauffent.
    let mut x = v.clone();
    boucle(&mut x)["generateurs"].as_array_mut().unwrap().pop();
    assert!(messages(&x)
        .iter()
        .any(|m| m.contains("générateur de chauffage de la boucle requis")));
    // Sans tour ni générateur de froid alors que les PAC refroidissent.
    let mut x = v.clone();
    boucle(&mut x)["boucle_d_eau"]
        .as_object_mut()
        .unwrap()
        .remove("tour");
    assert!(messages(&x)
        .iter()
        .any(|m| m.contains("générateur de refroidissement de la boucle requis")));
    // θmax_rech_BE requise avec une chaudière de boucle.
    let mut x = v.clone();
    boucle(&mut x)["boucle_d_eau"]
        .as_object_mut()
        .unwrap()
        .remove("theta_max_rech_c");
    assert!(messages(&x).iter().any(|m| m.contains("θmax_rech_BE")));
    // PAC d'une autre technologie, ou sur une autre source.
    let mut x = v.clone();
    boucle(&mut x)["generateurs"][0]["nature"]["pac_sur_boucle_d_eau"]["pac"]["chauffage"]
        ["technologie"] = "air_exterieur_air_recycle".into();
    assert!(messages(&x)
        .iter()
        .any(|m| m.contains("eau_de_boucle_air_recycle")));
    // Deux températures de boucle différentes.
    let mut x = v.clone();
    boucle(&mut x)["generateurs"][1]["nature"]["pac_sur_boucle_d_eau"]["pac"]["source_amont"]
        ["boucle_d_eau"]["boucle"]["theta_max_c"] = 28.0.into();
    assert!(messages(&x).iter().any(|m| m.contains("même température")));
    // Groupe inconnu, ou groupe relié sans PAC.
    let mut x = v.clone();
    boucle(&mut x)["generateurs"][1]["nature"]["pac_sur_boucle_d_eau"]["groupe"] =
        "inconnu".into();
    let m = messages(&x);
    assert!(m.iter().any(|m| m.contains("groupe inconnu")), "{m:?}");
    assert!(m.iter().any(|m| m.contains("« jour »")), "{m:?}");
    // Chaudière de boucle d'ECS : le générateur de la boucle a une seule fonction.
    let mut x = v.clone();
    boucle(&mut x)["generateurs"][2] = serde_json::json!({
        "id": "chaudiere", "rdim": 1,
        "nature": {"chaudiere": {"type_chaudiere": "gaz_condensation", "fonction": "chauffage_ecs",
            "pn_gen_kw": 60.0, "pertes_parois": "avec_ventilateur"}}
    });
    assert!(messages(&x).iter().any(|m| m.contains("idfougen = 1")));
    // Générateur de boucle sur air.
    let mut x = v.clone();
    boucle(&mut x)["generateurs"][2] = serde_json::json!({
        "id": "convecteurs", "rdim": 1,
        "nature": {"effet_joule": {"fonction": "chauffage", "pn_gen_w": 10000}}
    });
    assert!(messages(&x)
        .iter()
        .any(|m| m.contains("l'eau pour fluide aval")));
    // Gestion des priorités sans objet.
    let mut x = v.clone();
    boucle(&mut x)["priorite"] = "cascade".into();
    assert!(messages(&x).iter().any(|m| m.contains("sans objet")));
    // PAC sur boucle d'eau hors d'une génération à boucle : erreur de saisie.
    let mut x = v.clone();
    let pac = boucle(&mut x)["generateurs"][0].clone();
    x["batiments"][0]["generations"][1]["generateurs"]
        .as_array_mut()
        .unwrap()
        .push(pac);
    assert!(messages(&x)
        .iter()
        .any(|m| m.contains("génération qui décrit la boucle")));
    // ECS sur la génération à boucle : hors périmètre (code 3, lecture Q240).
    let mut x = v.clone();
    let ballon = x["batiments"][0]["generations"][1]["stockages"].clone();
    boucle(&mut x)["stockages"] = ballon;
    let r = rapport(&x);
    assert!(r
        .erreurs
        .iter()
        .any(|e| e.hors_perimetre && e.message.contains("ECS")));
}

#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn boucle_d_eau_sur_meteo_reglementaire() {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO");
    let octets = std::fs::read(chemin).unwrap();
    let p = projet(&json_du_cas(EXEMPLE));
    let m = meteo::lire(&octets, p.localisation.zone_climatique_retenue()).unwrap();
    let r = calculer(&p, &m, &Options { arrondir: false }).unwrap();
    let b = &r.batiments[0];
    let i = b.indicateurs.as_ref().unwrap();
    let be = b.generations[0].boucle_d_eau.as_ref().unwrap();
    println!(
        "{EXEMPLE} : Bbio {:.1}, DH {:.1}, Cep {:.1}, Cep,nr {:.1}, Ic énergie {:.1}, Qsys_BE_ch \
         {:.1} kWh, Qsys_BE_fr {:.1} kWh, transfert {:.1} kWh",
        b.bbio_pts.unwrap(),
        b.dh_c_h.unwrap(),
        i.cep_kwhep_m2,
        i.cep_nr_kwhep_m2,
        i.ic_energie_kgco2_m2.unwrap(),
        be.q_sys_ch_kwh,
        be.q_sys_fr_kwh,
        be.q_transfert_kwh
    );
    let d = |x: f64| (x * 10.0).round() / 10.0;
    assert_eq!(
        (
            d(b.bbio_pts.unwrap()),
            d(b.dh_c_h.unwrap()),
            d(i.cep_kwhep_m2),
            d(i.ic_energie_kgco2_m2.unwrap()),
            d(be.q_sys_ch_kwh)
        ),
        // Valeurs phares au lot 2f (arrondies à une décimale).
        (103.6, 1253.6, 260.6, 317.7, 88834.8)
    );
}
