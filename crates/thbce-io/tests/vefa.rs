// SPDX-License-Identifier: GPL-3.0-or-later
//! Chauffage par défaut des travaux réservés par l'acquéreur (VEFA, fiche 8.33, lot 2g), de
//! bout en bout sur l'exemple `immeuble-collectif-vefa-ademe`, sur la météo ouverte ADEME (CI
//! publique) et sur la météo réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test vefa -- --ignored --nocapture
//! ```
//!
//! La fiche ne fait que figer des composants existants : le test principal vérifie que la
//! saisie `chauffage_par_defaut_vefa` donne exactement le résultat des mêmes composants saisis
//! un par un selon les tableaux 261 à 266.

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

const VEFA: &str = "immeuble-collectif-vefa-ademe";

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

fn calcul(v: &serde_json::Value, meteo_de: &dyn Fn(&Projet) -> MeteoProjet) -> Resultats {
    let p = projet(v);
    let r = thbce_core::valider(&p);
    assert!(r.est_valide(), "{:?}", r.erreurs);
    calculer(&p, &meteo_de(&p), &Options { arrondir: false }).unwrap()
}

/// Les composants des tableaux 261 à 266, saisis à la main.
fn composants_saisis(hsp_m: f64) -> serde_json::Value {
    let mut v = json_du_cas(VEFA);
    let b = &mut v["batiments"][0];
    b.as_object_mut()
        .unwrap()
        .remove("chauffage_par_defaut_vefa");
    b["generations"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id": "vefa-defaut-generation",
            "raccordement_reseaux": "avec_isolement",
            "position": {"hors_volume_chauffe": {}},
            "priorite": "sans_priorite",
            "gestion_temperature_chauffage": "reseaux",
            "generateurs": [{"id": "vefa-defaut-generateur", "rdim": 1, "priorite_chauffage": 1,
                "nature": {"effet_joule": {"fonction": "chauffage", "pn_gen_w": 30000}}}]
        }));
    b["distributions_intergroupes"] = serde_json::json!([{
        "id": "vefa-defaut-intergroupes", "fonction": "chauffage",
        "generation": "vefa-defaut-generation", "reseau": "fictif"
    }]);
    let g = &mut b["zones"][0]["groupes"][0];
    g["distributions"] = serde_json::json!([{
        "id": "vefa-defaut-distribution", "fonction": "chauffage",
        "distribution_intergroupe": "vefa-defaut-intergroupes", "reseau": "fictif"
    }]);
    g["emetteurs"] = serde_json::json!([{
        "id": "vefa-defaut-emetteur",
        "chauffage": {
            "type_emetteur": "soufflage_air_chaud",
            "variation_spatiale": {"classe": {"classe": "C", "hsp_m": hsp_m}},
            "variation_temporelle": {"certifie": {"delta_theta_k": 1.8}},
            "effet_joule": true, "rats": 1.0, "ratt": 1.0,
            "distribution": "vefa-defaut-distribution"
        },
        "detection_presence": false
    }]);
    v
}

#[test]
fn identique_aux_composants_saisis() {
    let a = calcul(&json_du_cas(VEFA), &meteo_ademe);
    let b = calcul(&composants_saisis(2.5), &meteo_ademe);
    let (ba, bb) = (&a.batiments[0], &b.batiments[0]);
    assert_eq!(ba.cep_kwhep_m2, bb.cep_kwhep_m2);
    assert_eq!(ba.ic_energie_kgco2_m2, bb.ic_energie_kgco2_m2);
    assert_eq!(ba.dh_c_h, bb.dh_c_h);
    let g = ba
        .generations
        .iter()
        .find(|g| g.id == "vefa-defaut-generation")
        .unwrap();
    // Éq. (1667) à (1671) : générateur parfait, consommation égale à l'énergie fournie.
    let x = &g.generateurs[0];
    assert!(x.q_fou_ch_kwh > 0.0);
    assert!((x.consommations.chauffage.electricite_kwh - x.q_fou_ch_kwh).abs() < 1e-6);
}

/// Tableau 260 : les quatre colonnes de hauteur sous plafond (classe C : 0,4 / 1,2 / 2,0 /
/// 2,8 K), de bout en bout : chaque variante de la saisie donne exactement le résultat des
/// composants saisis à la main avec la même hauteur, et la consommation croît avec la
/// variation spatiale.
#[test]
fn quatre_hauteurs_sous_plafond() {
    let mut precedente = 0.0;
    for hsp in [2.5, 5.0, 7.0, 9.0] {
        let mut v = json_du_cas(VEFA);
        v["batiments"][0]["chauffage_par_defaut_vefa"]["groupes"][0]["hsp_m"] = hsp.into();
        let a = calcul(&v, &meteo_ademe);
        let b = calcul(&composants_saisis(hsp), &meteo_ademe);
        let (ba, bb) = (&a.batiments[0], &b.batiments[0]);
        assert_eq!(ba.cep_kwhep_m2, bb.cep_kwhep_m2, "hsp {hsp}");
        assert_eq!(ba.dh_c_h, bb.dh_c_h, "hsp {hsp}");
        let cep = ba.cep_kwhep_m2.unwrap();
        assert!(cep > precedente, "hsp {hsp} : {cep} <= {precedente}");
        precedente = cep;
    }
}

#[test]
fn refus_complementaires() {
    let msg = |v: &serde_json::Value, texte: &str| {
        let r = thbce_core::valider(&projet(v));
        assert!(
            r.erreurs.iter().any(|e| e.message.contains(texte)),
            "{texte} : {:?}",
            r.erreurs
        );
    };
    // Aucun groupe desservi.
    let mut v = json_du_cas(VEFA);
    v["batiments"][0]["chauffage_par_defaut_vefa"]["groupes"] = serde_json::json!([]);
    msg(&v, "au moins un groupe");
    // Usage hors habitation.
    let mut v = json_du_cas(VEFA);
    v["batiments"][0]["zones"][0]["usage"] = "bureaux".into();
    msg(&v, "qu'en habitation");
    // Hauteur sous plafond nulle.
    let mut v = json_du_cas(VEFA);
    v["batiments"][0]["chauffage_par_defaut_vefa"]["groupes"][0]["hsp_m"] = 0.into();
    let r = thbce_core::valider(&projet(&v));
    assert!(r.erreurs.iter().any(|e| e.chemin.ends_with("hsp_m")));
    // Groupe cité deux fois.
    let mut v = json_du_cas(VEFA);
    let g = v["batiments"][0]["chauffage_par_defaut_vefa"]["groupes"][0].clone();
    v["batiments"][0]["chauffage_par_defaut_vefa"]["groupes"] = serde_json::json!([g, g]);
    let r = thbce_core::valider(&projet(&v));
    assert!(!r.est_valide());
}

#[test]
fn refus() {
    let erreurs = |v: &serde_json::Value| thbce_core::valider(&projet(v));
    let msg = |v: &serde_json::Value, texte: &str| {
        let r = erreurs(v);
        assert!(!r.est_valide());
        assert!(
            r.erreurs.iter().any(|e| e.message.contains(texte)),
            "{texte} : {:?}",
            r.erreurs
        );
    };
    // Groupe inconnu.
    let mut v = json_du_cas(VEFA);
    v["batiments"][0]["chauffage_par_defaut_vefa"]["groupes"][0]["groupe"] = "absent".into();
    msg(&v, "groupe inconnu");
    // Groupe déjà chauffé.
    let mut v = composants_saisis(2.5);
    v["batiments"][0]["chauffage_par_defaut_vefa"] =
        serde_json::json!({"pn_gen_w": 1000, "groupes": [{"groupe": "immeuble", "hsp_m": 2.5}]});
    msg(&v, "déjà un émetteur de chauffage");
    // Identifiant réservé (et puissance nulle).
    let mut v = json_du_cas(VEFA);
    v["batiments"][0]["chauffage_par_defaut_vefa"]["pn_gen_w"] = 0.into();
    v["batiments"][0]["generations"][0]["id"] = "vefa-defaut-ballon".into();
    msg(&v, "réservé");
    assert!(erreurs(&v)
        .erreurs
        .iter()
        .any(|e| e.chemin.ends_with("pn_gen_w")));
}

/// Valeurs phares sur la météo réglementaire (nombres annuels seulement).
#[test]
#[ignore = "météo réglementaire (THBCE_METEO)"]
fn vefa_sur_meteo_reglementaire() {
    let r = calcul(&json_du_cas(VEFA), &meteo_reglementaire);
    let b = &r.batiments[0];
    println!(
        "{VEFA} : Bbio {:.1}, DH {:.1}, Cep {:.1}, Cep,nr {:.1}, Ic énergie {:.1}",
        b.bbio_pts.unwrap(),
        b.dh_c_h.unwrap(),
        b.cep_kwhep_m2.unwrap(),
        b.cep_nr_kwhep_m2.unwrap(),
        b.ic_energie_kgco2_m2.unwrap()
    );
    // Valeurs phares (une décimale), relevées le 21 novembre 2026.
    let un = |x: f64| (x * 10.0).round() / 10.0;
    assert_eq!(un(b.bbio_pts.unwrap()), 31.8);
    assert_eq!(un(b.dh_c_h.unwrap()), 1039.6);
    assert_eq!(un(b.cep_kwhep_m2.unwrap()), 189.1);
    assert_eq!(un(b.cep_nr_kwhep_m2.unwrap()), 189.1);
    assert_eq!(un(b.ic_energie_kgco2_m2.unwrap()), 234.9);
}
