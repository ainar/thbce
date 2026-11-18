// SPDX-License-Identifier: GPL-3.0-or-later
//! Mode actif des centrales de traitement d'air à charge variable (lot 1-ven2, Th-BCE 2020
//! §6.3.3.5, réseau de CTA de la fiche 8.11, éq. 940 ; lecture `docs/questions/Q095.md`) :
//! bout en bout sur l'exemple `tests/cas/bureaux-cta-dav-ademe`, météo ouverte ADEME ;
//! consommation des ventilateurs recoupée à la main (éq. 614 et 618) ; centrale à débit
//! constant (type 3) avec batteries chaude et froide ; sens des effets ; erreurs de saisie.
//!
//! Valeurs phares sur la météo réglementaire :
//! THBCE_METEO=/chemin/vers/meteo.csv cargo t-regl mode_actif_sur_meteo_reglementaire

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{Cta, Projet, SystemeVentilation, TypeCta};
use thbce_core::resultats::{Resultats, ResultatsVentilationMode};
use thbce_core::{calculer, valider, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bureaux_dav() -> Projet {
    let chemin = racine().join("tests/cas/bureaux-cta-dav-ademe/entree.json");
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn meteo_ademe(p: &Projet) -> MeteoProjet {
    let zone = p.localisation.zone_climatique_retenue();
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

fn calcul(p: &Projet) -> Resultats {
    assert!(valider(p).est_valide(), "{:?}", valider(p).erreurs);
    calculer(p, &meteo_ademe(p), &Options { arrondir: false }).unwrap()
}

fn cta_mut(p: &mut Projet) -> &mut Cta {
    match &mut p.batiments[0].zones[0]
        .ventilation
        .as_mut()
        .unwrap()
        .systeme
    {
        SystemeVentilation::Cta(c) => c,
        _ => panic!("centrale attendue"),
    }
}

fn th_c(r: &Resultats) -> ResultatsVentilationMode {
    r.batiments[0].zones[0]
        .ventilation
        .as_ref()
        .unwrap()
        .th_c
        .unwrap()
}

fn c_aux_vent(r: &Resultats) -> f64 {
    r.batiments[0].zones[0]
        .ventilation
        .as_ref()
        .unwrap()
        .c_aux_vent_kwh
}

/// Demande reçue par la génération `id`, en kWh : chauffage et refroidissement.
fn generation(r: &Resultats, id: &str) -> (f64, f64) {
    let g = r.batiments[0]
        .generations
        .iter()
        .find(|g| g.id == id)
        .unwrap();
    (g.q_ch_kwh, g.q_fr_kwh)
}

/// La batterie chaude assure la demande de l'émetteur contenu dans les bouches : le réseau du
/// groupe ne reçoit rien, la chaudière reçoit l'énergie de la batterie (éq. 940).
#[test]
fn mode_actif_de_bout_en_bout() {
    let p = bureaux_dav();
    let r = calcul(&p);
    let m = th_c(&r);
    let q_chaud = m.q_chaud_kwh.unwrap();
    let g = &r.batiments[0].zones[0].groupes[0];
    let sy = g.systemes.as_ref().unwrap();
    let q_em = sy.emetteurs[0].q_ch_kwh;
    println!(
        "mode actif {} h ; Qchaud(rec) {q_chaud:.1} kWh ; émetteur {q_em:.1} kWh",
        m.nbh_mode_actif.unwrap()
    );
    assert!(m.nbh_mode_actif.unwrap() > 0);
    assert!(q_chaud > 0.0 && q_em > 0.0);
    assert_eq!(sy.distributions[0].q_kwh, 0.0);
    assert!(m.q_froid_kwh.is_none());
    // La chaudière reçoit au moins l'énergie de la batterie (plus le préchauffage antigel).
    let (q_ch, _) = generation(&r, "chaufferie");
    assert!(q_ch >= q_chaud - 1e-6, "{q_ch} {q_chaud}");
}

/// Éq. (614) et (618) : avec des puissances de ventilateurs égales dans tous les régimes,
/// `τ·P + (1 − τ)·P = P` à chaque heure : 1 000 W × 8 760 h, sans échangeur rotatif.
#[test]
fn consommation_des_ventilateurs_a_la_main() {
    let mut p = bureaux_dav();
    {
        let c = cta_mut(&mut p);
        c.p_vent_rep_ch_w = 400.0;
        c.p_vent_sou_ch_w = 600.0;
        c.p_vent_rep_zn_occ_w = 400.0;
        c.p_vent_sou_zn_occ_w = 600.0;
        c.p_vent_rep_zn_inocc_w = 400.0;
        c.p_vent_sou_zn_inocc_w = 600.0;
    }
    let r = calcul(&p);
    assert!(th_c(&r).nbh_mode_actif.unwrap() > 0);
    assert!((c_aux_vent(&r) - 8760.0).abs() < 1e-6, "{}", c_aux_vent(&r));
}

/// Sens des effets : une température nominale de soufflage plus haute réduit le taux de
/// charge, donc la part du débit et des ventilateurs du mode actif (plus puissants ici que
/// ceux de la zone neutre, éq. 614).
#[test]
fn sens_des_effets() {
    let p = bureaux_dav();
    let r = calcul(&p);
    let mut q = p.clone();
    cta_mut(&mut q).theta_sou_nom_chaud_c = Some(40.0);
    let rq = calcul(&q);
    assert!(c_aux_vent(&rq) < c_aux_vent(&r));
    // Sans batterie chaude ni émetteur dans les bouches (radiateurs), plus de mode actif.
    let mut s = p.clone();
    {
        let c = cta_mut(&mut s);
        c.batterie_chaude = None;
        c.generation_batterie_chaude = None;
        c.theta_sou_nom_chaud_c = None;
    }
    s.batiments[0].zones[0].groupes[0].emetteurs[0]
        .chauffage
        .as_mut()
        .unwrap()
        .centrale = None;
    let rs = calcul(&s);
    assert!(th_c(&rs).nbh_mode_actif.is_none());
    assert!(
        rs.batiments[0].zones[0].groupes[0]
            .systemes
            .as_ref()
            .unwrap()
            .distributions[0]
            .q_kwh
            > 0.0
    );
}

/// Centrale à débit constant et température variable (type 3, τ = 1, éq. 582 à 584) avec
/// batteries chaude et froide, le froid venant d'une génération de froid (éq. 940).
fn type_3_chaud_froid() -> Projet {
    let mut v = serde_json::to_value(bureaux_dav()).unwrap();
    let b = &mut v["batiments"][0];
    b["generations"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id": "groupe-froid", "raccordement_reseaux": "avec_isolement",
            "position": "en_volume_chauffe",
            "generateurs": [{"id": "pac", "rdim": 1, "nature": {"thermodynamique_electrique": {
                "compresseur": "puissance_variable", "source_amont": {"air_exterieur": {}},
                "refroidissement": {"technologie": "air_exterieur_air_recycle",
                    "performances": {"pivot_declare": {"performance": 3.5, "pabs_kw": 20.0}},
                    "typo_emetteur": "systemes_a_air"}}}}]
        }));
    b["distributions_intergroupes"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id": "intergroupes-froid", "fonction": "refroidissement",
            "generation": "groupe-froid", "reseau": "fictif"
        }));
    let g = &mut b["zones"][0]["groupes"][0];
    g["iclim"] = true.into();
    g["programmation"]["refroidissement"] = "horloge".into();
    g["distributions"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id": "d-fr", "fonction": "refroidissement",
            "distribution_intergroupe": "intergroupes-froid", "reseau": "fictif"
        }));
    g["emetteurs"][0]["refroidissement"] = serde_json::json!({
        "type_emetteur": "soufflage_air_froid",
        "variation_spatiale": {"classe": {"classe": "B", "hsp_m": 2.7}},
        "variation_temporelle": {"defaut": {"arret_total": true}},
        "rats": 1, "ratt": 1, "distribution": "d-fr", "centrale": true});
    let c = &mut b["zones"][0]["ventilation"]["systeme"]["cta"];
    c["type_cta"] = "debit_constant_temperature_variable".into();
    c.as_object_mut().unwrap().remove("theta_sou_nom_chaud_c");
    c["batterie_froide"] = true.into();
    c["generation_batterie_froide"] = "groupe-froid".into();
    c["theta_dim_fr_c"] = 7.into();
    serde_json::from_value(v).unwrap()
}

#[test]
fn centrale_de_type_3_chaud_et_froid() {
    let p = type_3_chaud_froid();
    let r = calcul(&p);
    let m = th_c(&r);
    let (q_chaud, q_froid) = (m.q_chaud_kwh.unwrap(), m.q_froid_kwh.unwrap());
    println!(
        "type 3 : mode actif {} h ; Qchaud {q_chaud:.1} kWh ; Qfroid {q_froid:.1} kWh",
        m.nbh_mode_actif.unwrap()
    );
    assert!(q_chaud > 0.0);
    assert!(q_froid < 0.0);
    let sy = r.batiments[0].zones[0].groupes[0]
        .systemes
        .as_ref()
        .unwrap();
    println!(
        "émetteur : chaud {:.1} kWh, froid {:.1} kWh",
        sy.emetteurs[0].q_ch_kwh, sy.emetteurs[0].q_fr_kwh
    );
    assert!(sy.distributions.iter().all(|d| d.q_kwh == 0.0));
    let (_, q_fr) = generation(&r, "groupe-froid");
    assert!(q_fr >= -q_froid - 1e-6, "{q_fr} {q_froid}");
}

#[test]
fn erreurs_de_saisie() {
    let chemins = |p: &Projet| -> (Vec<String>, bool) {
        let r = valider(p);
        (
            r.erreurs.iter().map(|e| e.chemin.clone()).collect(),
            r.seulement_hors_perimetre(),
        )
    };
    let c = "batiments[0].zones[0].ventilation.systeme.cta";
    let em = "batiments[0].zones[0].groupes[0].emetteurs[0]";
    // Centrale de prétraitement (type 2) : pas de mode actif.
    let mut p = bureaux_dav();
    {
        let x = cta_mut(&mut p);
        x.type_cta = TypeCta::DebitConstant;
        x.theta_sou_nom_chaud_c = None;
    }
    assert_eq!(chemins(&p), (vec![format!("{c}.type_cta")], false));
    // Température nominale requise en type 4, sans objet en type 3.
    let mut p = bureaux_dav();
    cta_mut(&mut p).theta_sou_nom_chaud_c = None;
    assert_eq!(chemins(&p).0, [format!("{c}.theta_sou_nom_chaud_c")]);
    let mut p = bureaux_dav();
    cta_mut(&mut p).type_cta = TypeCta::DebitConstantTemperatureVariable;
    assert_eq!(chemins(&p).0, [format!("{c}.theta_sou_nom_chaud_c")]);
    // Génération manquante.
    let mut p = bureaux_dav();
    cta_mut(&mut p).generation_batterie_chaude = None;
    assert_eq!(chemins(&p).0, [format!("{c}.generation_batterie_chaude")]);
    // Émetteur dans les bouches sans batterie, batterie sans émetteur.
    let mut p = bureaux_dav();
    {
        let x = cta_mut(&mut p);
        x.batterie_chaude = None;
        x.generation_batterie_chaude = None;
        x.theta_sou_nom_chaud_c = None;
    }
    assert_eq!(chemins(&p).0, [format!("{em}.chauffage.centrale")]);
    let mut p = bureaux_dav();
    p.batiments[0].zones[0].groupes[0].emetteurs[0]
        .chauffage
        .as_mut()
        .unwrap()
        .centrale = None;
    assert_eq!(chemins(&p).0, [format!("{c}.batterie_chaude")]);
    // L'humidification reste hors périmètre (code 3).
    let mut p = bureaux_dav();
    cta_mut(&mut p).humidification = Some(true);
    assert_eq!(chemins(&p), (vec![format!("{c}.humidification")], true));
}

fn meteo_reglementaire(p: &Projet) -> MeteoProjet {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO pour lancer ce test");
    let zone = p.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

/// Valeurs phares de l'exemple sur la météo réglementaire (H2b) : nombres annuels seulement.
#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn mode_actif_sur_meteo_reglementaire() {
    let p = bureaux_dav();
    let r = calculer(&p, &meteo_reglementaire(&p), &Options { arrondir: false }).unwrap();
    assert!(r.reglementaire);
    let b = &r.batiments[0];
    let m = th_c(&r);
    let phares = [
        b.bbio_pts.unwrap(),
        b.dh_c_h.unwrap(),
        b.cep_kwhep_m2.unwrap(),
        b.cep_nr_kwhep_m2.unwrap(),
        b.ic_energie_kgco2_m2.unwrap(),
        m.q_chaud_kwh.unwrap(),
    ];
    println!("réglementaire : Bbio, DH, Cep, Cep,nr, Ic énergie, Qchaud(rec) = {phares:.1?}");
    let proche = |a: f64, b: f64| (a - b).abs() <= 0.05 + 1e-9;
    for (x, y) in phares.iter().zip(REGL) {
        assert!(proche(*x, y), "{phares:?}");
    }
}

/// Valeurs phares sur la météo réglementaire (lot 1-ven2, 19 novembre 2026).
const REGL: [f64; 6] = [82.9, 619.5, 68.9, 68.9, 293.1, 27643.8];
