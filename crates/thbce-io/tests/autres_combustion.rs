// SPDX-License-Identifier: GPL-3.0-or-later
//! Autres générateurs à combustion (fiche 8.20, lot 2e) de bout en bout : exemples
//! `maison-radiateurs-gaz-ademe` et `gymnase-air-chaud-ademe` sur la météo ouverte ADEME (CI
//! publique) et sur la météo réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo.csv cargo test -p thbce-io --test autres_combustion -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les tests vérifient des propriétés calculées à la
//! main à partir du texte (consommation d'un radiateur proportionnelle à l'énergie fournie,
//! éq. 1134 à 1136 ; statut des rendements, éq. 1132 ; pertes à l'arrêt selon le raccordement,
//! éq. 1150 et 1153) et les refus de la validation.

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

fn calcul(p: &Projet, m: &MeteoProjet) -> Resultats {
    let r = thbce_core::valider(p);
    assert!(r.est_valide(), "{:?}", r.erreurs);
    calculer(p, m, &Options { arrondir: false }).unwrap()
}

/// `(énergie fournie en chauffage, gaz du chauffage, gaz de l'ECS)` d'un générateur, en kWh.
fn generateur(r: &Resultats, id: &str) -> (f64, f64, f64) {
    for g in &r.batiments[0].generations {
        for x in &g.generateurs {
            if x.id == id {
                return (
                    x.q_fou_ch_kwh,
                    x.consommations.chauffage.gaz_kwh,
                    x.consommations.ecs.gaz_kwh,
                );
            }
        }
    }
    panic!("générateur {id} absent");
}

fn nature_mut<'a>(v: &'a mut serde_json::Value, id: &str) -> &'a mut serde_json::Value {
    for g in v["batiments"][0]["generations"].as_array_mut().unwrap() {
        for x in g["generateurs"].as_array_mut().unwrap() {
            if x["id"] == id {
                return &mut x["nature"]["autre_generateur_a_combustion"];
            }
        }
    }
    panic!("générateur {id} absent");
}

#[test]
fn radiateur_gaz_consommation_a_la_main() {
    let v = json_du_cas("maison-radiateurs-gaz-ademe");
    let p = projet(&v);
    let r = calcul(&p, &meteo_ademe(&p));
    let (q_fou, gaz, _) = generateur(&r, "radiateur-gaz");
    assert!(q_fou > 0.0);
    // Radiateur sans ventilateur, micro-ventouse : fmaj = 1,04 ; Rpn = 80 % par défaut ;
    // PCSI = 1,11. Pas de pertes à charge nulle (éq. 1128) : chaque heure,
    // Qcons = (Qfou + QPx)/PCSI avec QPx = fmaj·(100 − Rpn/PCSI)·Qfou·PCSI/Rpn (éq. 1134 à 1136).
    let k = (1.0 + 1.04 * (100.0 - 80.0 / 1.11) * 1.11 / 80.0) / 1.11;
    assert!(
        (gaz - k * q_fou).abs() < 1e-6 * gaz,
        "{gaz} contre {}",
        k * q_fou
    );
    // Rendement certifié de 90 % : moins de gaz ; déclaré à 90 % (min(72 ; 86) = 72 %) : plus.
    let mut vc = v.clone();
    nature_mut(&mut vc, "radiateur-gaz")["rendements"] =
        serde_json::json!({"statut": "certifie", "r_pn_pct": 90.0});
    let pc = projet(&vc);
    let (_, gaz_c, _) = generateur(&calcul(&pc, &meteo_ademe(&pc)), "radiateur-gaz");
    nature_mut(&mut vc, "radiateur-gaz")["rendements"]["statut"] = "declare".into();
    let pd = projet(&vc);
    let (q_d, gaz_d, _) = generateur(&calcul(&pd, &meteo_ademe(&pd)), "radiateur-gaz");
    assert!(gaz_c < gaz && gaz < gaz_d, "{gaz_c} {gaz} {gaz_d}");
    let kd = (1.0 + 1.04 * (100.0 - 72.0 / 1.11) * 1.11 / 72.0) / 1.11;
    assert!((gaz_d - kd * q_d).abs() < 1e-6 * gaz_d);
}

#[test]
fn chauffe_eau_gaz_pertes_a_charge_nulle() {
    let v = json_du_cas("maison-radiateurs-gaz-ademe");
    let p = projet(&v);
    let m = meteo_ademe(&p);
    let r = calcul(&p, &m);
    let (_, _, gaz) = generateur(&r, "chauffe-eau-gaz");
    let q_ecs: f64 = r.batiments[0]
        .generations
        .iter()
        .flat_map(|g| &g.generateurs)
        .filter(|x| x.id == "chauffe-eau-gaz")
        .map(|x| x.q_fou_ecs_kwh)
        .sum();
    // Sans pertes à charge nulle, Qcons = Qfou/Rpn (éq. 1145) ; avec, davantage.
    assert!(gaz > q_ecs / 0.84 * (1.0 - 1e-9), "{gaz} {q_ecs}");
    let mut v0 = v.clone();
    nature_mut(&mut v0, "chauffe-eau-gaz")["q_po_30_w"] = 0.0.into();
    let p0 = projet(&v0);
    let r0 = calcul(&p0, &m);
    let (_, _, gaz0) = generateur(&r0, "chauffe-eau-gaz");
    let q_ecs: f64 = r0.batiments[0]
        .generations
        .iter()
        .flat_map(|g| &g.generateurs)
        .filter(|x| x.id == "chauffe-eau-gaz")
        .map(|x| x.q_fou_ecs_kwh)
        .sum();
    assert!(
        (gaz0 - q_ecs / 0.84).abs() < 1e-6 * gaz0,
        "{gaz0} {}",
        q_ecs / 0.84
    );
    assert!(gaz0 < gaz);
}

/// Gymnase en cascade avec l'air chaud en premier rang ; `hors_vc` place la génération hors
/// volume chauffé (à l'extérieur : btherm = 1). Retourne (gaz raccordé en permanence, gaz avec
/// isolement) de l'air chaud, en kWh.
fn air_chaud_perm_et_iso(hors_vc: bool) -> (f64, f64) {
    let mut v = json_du_cas("gymnase-air-chaud-ademe");
    for x in v["batiments"][0]["generations"][0]["generateurs"]
        .as_array_mut()
        .unwrap()
    {
        x["priorite_chauffage"] = if x["id"] == "air-chaud" { 1 } else { 2 }.into();
    }
    if hors_vc {
        v["batiments"][0]["generations"][0]["position"] =
            serde_json::json!({"hors_volume_chauffe": {}});
    }
    let p = projet(&v);
    let m = meteo_ademe(&p);
    let mut iso = v.clone();
    nature_mut(&mut iso, "air-chaud")
        .as_object_mut()
        .unwrap()
        .remove("raccordement");
    let pi = projet(&iso);
    let (q_perm, g_perm, _) = generateur(&calcul(&p, &m), "air-chaud");
    let (q_iso, g_iso, _) = generateur(&calcul(&pi, &m), "air-chaud");
    // Les pertes récupérées modifient un peu les besoins (fiche 11.1).
    assert!(q_perm > 0.0 && (q_perm - q_iso).abs() < 1e-2 * q_perm);
    (g_perm, g_iso)
}

#[test]
fn air_chaud_raccordement_permanent_et_isolement() {
    // Hors volume chauffé, θamb = θext et θaval = θi,aval,eq : QP0 > 0 (éq. 1127). Raccordé en
    // permanence, le générateur a des pertes à l'arrêt à chaque pas de la saison de chauffage
    // (éq. 1150) ; isolé, seulement après un pas de fonctionnement (éq. 1153).
    let (g_perm, g_iso) = air_chaud_perm_et_iso(true);
    assert!(g_perm > g_iso * 1.001, "{g_perm} {g_iso}");
}

#[test]
fn air_chaud_en_volume_chauffe_pertes_a_larret_negligeables() {
    // Lecture Q155 : en volume chauffé, θaval = θi,aval,eq ≈ θamb, donc QP0 ≈ 0 (éq. 1127) et
    // le raccordement n'a presque pas d'effet (variante chiffrée dans Q155).
    let (g_perm, g_iso) = air_chaud_perm_et_iso(false);
    assert!(
        g_perm >= g_iso && (g_perm - g_iso) < 1e-3 * g_iso,
        "{g_perm} {g_iso}"
    );
}

/// Maison d'exemple : le radiateur devient un panneau radiant au GPL et le chauffe-eau un
/// accumulateur à condensation de plus de 200 L.
fn variante_panneau_accumulateur() -> serde_json::Value {
    let mut v = json_du_cas("maison-radiateurs-gaz-ademe");
    let r = nature_mut(&mut v, "radiateur-gaz");
    r["type_generateur"] = "panneau_radiant".into();
    r["gpl"] = true.into();
    r["pn_gen_kw"] = 3.0.into();
    r["w_aux_nom_w"] = 10.0.into();
    r["w_veille_w"] = 0.0.into();
    let a = nature_mut(&mut v, "chauffe-eau-gaz");
    a["type_generateur"] = serde_json::json!(
        {"accumulateur_gaz": {"condensation": true, "plus_de_200_l_montee_rapide": true}}
    );
    v
}

fn ecs_fou(r: &Resultats, id: &str) -> f64 {
    r.batiments[0]
        .generations
        .iter()
        .flat_map(|g| &g.generateurs)
        .filter(|x| x.id == id)
        .map(|x| x.q_fou_ecs_kwh)
        .sum()
}

#[test]
fn panneau_radiant_gpl_et_accumulateur_a_la_main() {
    let v = variante_panneau_accumulateur();
    let p = projet(&v);
    let m = meteo_ademe(&p);
    let r = calcul(&p, &m);
    // Panneau radiant (type 109, tableau 126 : Rpn = 90 % ; tableau 130 : fmaj = 1,00 ; PCSI du
    // GPL = 1,09) : chaque heure Qcons = (Qfou + QPx)/PCSI, QPx = fmaj·(100 − Rpn/PCSI)·Qfou·
    // PCSI/Rpn (éq. 1134 à 1136), soit Qcons = Qfou/0,9.
    let (q_fou, gaz, _) = generateur(&r, "radiateur-gaz");
    assert!(q_fou > 0.0);
    let k = (1.0 + 1.0 * (100.0 - 90.0 / 1.09) * 1.09 / 90.0) / 1.09;
    assert!((k - 1.0_f64 / 0.9).abs() < 1e-12);
    assert!((gaz - k * q_fou).abs() < 1e-6 * gaz, "{gaz} {}", k * q_fou);
    // Accumulateur à condensation (tableau 126 : Rpn = 98 %), sans pertes à charge nulle :
    // Qcons = Qfou/0,98 (éq. 1145) ; avec le Qpo30 par défaut (tableau 127 : E = 1,7 au-delà de
    // 200 L, soit 1000·18·1,7/100 = 306 W) : davantage.
    let mut v0 = v.clone();
    nature_mut(&mut v0, "chauffe-eau-gaz")["q_po_30_w"] = 0.0.into();
    let p0 = projet(&v0);
    let r0 = calcul(&p0, &m);
    let (_, _, gaz0) = generateur(&r0, "chauffe-eau-gaz");
    let q0 = ecs_fou(&r0, "chauffe-eau-gaz");
    assert!(q0 > 0.0);
    assert!(
        (gaz0 - q0 / 0.98).abs() < 1e-6 * gaz0,
        "{gaz0} {}",
        q0 / 0.98
    );
    let (_, _, gaz_defaut) = generateur(&r, "chauffe-eau-gaz");
    assert!(gaz_defaut > gaz0);
    // La veilleuse (Pveilleuse/2,58 ajouté à Wveille) augmente la consommation électrique.
    let elec = |r: &Resultats| -> f64 {
        r.batiments[0]
            .generations
            .iter()
            .flat_map(|g| &g.generateurs)
            .filter(|x| x.id == "chauffe-eau-gaz")
            .map(|x| x.consommations.ecs.electricite_kwh)
            .sum()
    };
    let mut vv = v.clone();
    nature_mut(&mut vv, "chauffe-eau-gaz")["p_veilleuse_w"] = 258.0.into();
    let pv = projet(&vv);
    let rv = calcul(&pv, &m);
    assert!(elec(&rv) > elec(&r), "{} {}", elec(&rv), elec(&r));
}

fn messages(v: &serde_json::Value) -> Vec<String> {
    thbce_core::valider(&projet(v))
        .erreurs
        .iter()
        .map(|e| format!("{} : {}", e.chemin, e.message))
        .collect()
}

#[test]
fn validation_des_autres_generateurs_a_combustion() {
    let v = json_du_cas("maison-radiateurs-gaz-ademe");
    assert!(messages(&v).is_empty());
    // Chauffe-eau : rendement justifié refusé (certifié ou par défaut, §8.20.3.7.1).
    let mut x = v.clone();
    nature_mut(&mut x, "chauffe-eau-gaz")["rendements"] =
        serde_json::json!({"statut": "justifie", "r_pn_pct": 90.0});
    assert!(messages(&x).iter().any(|m| m.contains("§8.20.3.7.1")));
    // Éq. (1120) : rendement sur PCS au moins égal à 1.
    let mut x = v.clone();
    nature_mut(&mut x, "radiateur-gaz")["rendements"] =
        serde_json::json!({"statut": "certifie", "r_pn_pct": 112.0});
    assert!(messages(&x).iter().any(|m| m.contains("éq. 1120")));
    // Radiateur sans ventilateur de combustion : évacuation des fumées requise.
    let mut x = v.clone();
    nature_mut(&mut x, "radiateur-gaz")["type_generateur"]["radiateur_gaz"]
        .as_object_mut()
        .unwrap()
        .remove("evacuation_fumees");
    assert!(messages(&x).iter().any(|m| m.contains("tableau 129")));
    // Panneau radiant : pas d'auxiliaires par défaut.
    let mut x = v.clone();
    nature_mut(&mut x, "radiateur-gaz")["type_generateur"] = "panneau_radiant".into();
    assert!(messages(&x).iter().any(|m| m.contains("tableau 128")));
    // Raccordement permanent hors gestion avec priorité (Q156).
    let mut x = v.clone();
    nature_mut(&mut x, "chauffe-eau-gaz")["raccordement"] = "permanent".into();
    assert!(messages(&x).iter().any(|m| m.contains("Q156")));
    // Un radiateur ne produit pas d'ECS : la génération d'ECS n'assure plus son poste.
    let mut x = v.clone();
    nature_mut(&mut x, "chauffe-eau-gaz")["type_generateur"] = serde_json::json!({
        "radiateur_gaz": {"ventilateur_combustion": true, "ventilateur_emission": false}
    });
    assert!(!messages(&x).is_empty());
}

#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn autres_combustion_sur_meteo_reglementaire() {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO");
    let octets = std::fs::read(chemin).unwrap();
    for nom in ["maison-radiateurs-gaz-ademe", "gymnase-air-chaud-ademe"] {
        let p = projet(&json_du_cas(nom));
        let m = meteo::lire(&octets, p.localisation.zone_climatique_retenue()).unwrap();
        let r = calcul(&p, &m);
        let b = &r.batiments[0];
        let i = b.indicateurs.as_ref().unwrap();
        println!(
            "{nom} : Bbio {:.1}, DH {:.1}, Cep {:.1}, Cep,nr {:.1}, Ic énergie {:.1}",
            b.bbio_pts.unwrap(),
            b.dh_c_h.unwrap(),
            i.cep_kwhep_m2,
            i.cep_nr_kwhep_m2,
            i.ic_energie_kgco2_m2.unwrap_or(f64::NAN)
        );
        // Valeurs phares au lot 2e (arrondies à une décimale) : Bbio, DH, Cep, Ic énergie.
        let phares = match nom {
            "maison-radiateurs-gaz-ademe" => (59.7, 1021.3, 83.9, 630.1),
            _ => (100.4, 2262.7, 147.6, 290.1),
        };
        let d = |x: f64| (x * 10.0).round() / 10.0;
        assert_eq!(
            (
                d(b.bbio_pts.unwrap()),
                d(b.dh_c_h.unwrap()),
                d(i.cep_kwhep_m2),
                d(i.ic_energie_kgco2_m2.unwrap())
            ),
            phares,
            "{nom}"
        );
    }
}
