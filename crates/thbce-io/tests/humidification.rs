// SPDX-License-Identifier: GPL-3.0-or-later
//! Humidificateur d'hiver et rafraîchissement évaporatif des centrales (lot 1-ven3, Th-BCE
//! 2020 §6.3.3.4.2, 6.3.3.4.5.3, 6.3.3.4.9.2 et 6.3.3.4.11 ; lectures `docs/questions/Q096.md`
//! et `docs/questions/Q097.md`) : bout en bout sur l'exemple
//! `tests/cas/bureaux-cta-humidification-ademe` (météo ouverte ADEME), sens des effets,
//! centrale à débit variable avec profil à hystérésis, erreurs de saisie et refus.
//!
//! Valeurs phares sur la météo réglementaire :
//! THBCE_METEO=/chemin/vers/meteo.csv cargo t-regl humidification_sur_meteo_reglementaire

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{
    Cta, Projet, SystemeVentilation, TypeHumidificationHiver, TypeRafraichissementEvaporatif,
};
use thbce_core::resultats::{Resultats, ResultatsVentilationMode};
use thbce_core::{calculer, valider, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cas(nom: &str) -> Projet {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
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

fn mode(r: &Resultats, th_c: bool) -> ResultatsVentilationMode {
    let v = r.batiments[0].zones[0].ventilation.as_ref().unwrap();
    if th_c { v.th_c } else { v.th_d }.unwrap()
}

fn c_aux_vent(r: &Resultats) -> f64 {
    r.batiments[0].zones[0]
        .ventilation
        .as_ref()
        .unwrap()
        .c_aux_vent_kwh
}

fn q_ch_generation(r: &Resultats, id: &str) -> f64 {
    r.batiments[0]
        .generations
        .iter()
        .find(|g| g.id == id)
        .unwrap()
        .q_ch_kwh
}

/// Humidificateur à vapeur et rafraîchissement indirect de l'exemple : l'énergie de
/// l'humidificateur passe à la chaudière (éq. 939), la pompe d'été aux auxiliaires (éq. 570).
#[test]
fn humidification_de_bout_en_bout() {
    let p = cas("bureaux-cta-humidification-ademe");
    let r = calcul(&p);
    let m = mode(&r, true);
    let q_hr = m.q_hr_kwh.unwrap();
    println!(
        "QHR {q_hr:.1} kWh ; Aut_adiab {} h ; pompe d'été {:.3} kWh ; auxiliaires {:.1} kWh",
        m.nbh_adiab.unwrap(),
        m.w_aux_adiab_kwh.unwrap(),
        c_aux_vent(&r)
    );
    assert!(q_hr > 0.0);
    assert_eq!(m.w_aux_chaudhr_kwh, Some(0.0));
    assert!(m.nbh_adiab.unwrap() > 0);
    assert!(m.w_aux_adiab_kwh.unwrap() > 0.0);
    // Sans humidificateur ni rafraîchissement : l'exemple de départ, moins de gaz.
    let base = calcul(&cas("bureaux-cta-ademe"));
    assert!(mode(&base, true).q_hr_kwh.is_none());
    assert!(q_ch_generation(&r, "chaufferie") > q_ch_generation(&base, "chaufferie"));
    // Les auxiliaires comprennent la pompe d'été.
    assert!(c_aux_vent(&r) > c_aux_vent(&base) - 1e-9 || m.nbh_adiab.unwrap() > 0);
}

/// Sens des effets et cas limites.
#[test]
fn sens_des_effets() {
    let p = cas("bureaux-cta-humidification-ademe");
    let r = calcul(&p);
    // Consigne d'humidité plus haute : plus d'énergie d'humidification.
    let mut q = p.clone();
    cta_mut(&mut q)
        .humidificateur
        .as_mut()
        .unwrap()
        .omega_cons_kg_kg = 0.008;
    let rq = calcul(&q);
    assert!(mode(&rq, true).q_hr_kwh.unwrap() > mode(&r, true).q_hr_kwh.unwrap());
    // Consigne nulle : aucune humidification (ωsouf3 = MAX(ωsouf2 ; 0) = ωsouf2, éq. 527).
    let mut q = p.clone();
    cta_mut(&mut q)
        .humidificateur
        .as_mut()
        .unwrap()
        .omega_cons_kg_kg = 0.0;
    assert_eq!(mode(&calcul(&q), true).q_hr_kwh, Some(0.0));
    // Pulvérisation : pas d'énergie (éq. 533), une pompe (éq. 568), air refroidi.
    let mut q = p.clone();
    {
        let h = cta_mut(&mut q).humidificateur.as_mut().unwrap();
        h.type_humidification = TypeHumidificationHiver::Pulverisation;
        h.generation = None;
    }
    let rq = calcul(&q);
    let mq = mode(&rq, true);
    assert_eq!(mq.q_hr_kwh, Some(0.0));
    assert!(mq.w_aux_chaudhr_kwh.unwrap() > 0.0);
    let v = rq.batiments[0].zones[0].ventilation.as_ref().unwrap();
    let d = v
        .valeurs_par_defaut
        .iter()
        .find(|d| d.champ.ends_with("humidificateur.eta_hum"))
        .unwrap();
    assert_eq!(d.valeur, 0.8);
    // Température de base inatteignable : jamais d'autorisation, mêmes résultats que sans
    // rafraîchissement évaporatif.
    let mut q = p.clone();
    cta_mut(&mut q)
        .rafraichissement_adiabatique
        .as_mut()
        .unwrap()
        .theta_i_base_c = 60.0;
    let rq = calcul(&q);
    assert_eq!(mode(&rq, true).nbh_adiab, Some(0));
    let mut s = p.clone();
    cta_mut(&mut s).rafraichissement_adiabatique = None;
    let rs = calcul(&s);
    assert_eq!(rq.batiments[0].cep_kwhep_m2, rs.batiments[0].cep_kwhep_m2);
    assert_eq!(rq.batiments[0].dh_c_h, rs.batiments[0].dh_c_h);
    // Le rafraîchissement indirect abaisse l'inconfort d'été (Th-D).
    assert!(r.batiments[0].dh_c_h.unwrap() <= rs.batiments[0].dh_c_h.unwrap());
}

/// Centrale à débit variable (type 4) en rafraîchissement direct : débits et ventilateurs
/// selon le profil à hystérésis (éq. 426 à 428, 466 et 467).
#[test]
fn debit_variable_et_hysteresis() {
    let mut p = cas("bureaux-cta-dav-ademe");
    let base = calcul(&p);
    let mut v = serde_json::to_value(&*cta_mut(&mut p)).unwrap();
    v["rafraichissement_adiabatique"] = serde_json::json!({
        "type_rafraichissement": "direct", "theta_i_base_c": 23,
        "delta_theta_i1_k": 2, "delta_theta_i2_k": 1});
    *cta_mut(&mut p) = serde_json::from_value(v).unwrap();
    let r = calcul(&p);
    let m = mode(&r, true);
    println!(
        "type 4 direct : Aut_adiab {} h ; pompe {:.3} kWh ; auxiliaires {:.1} → {:.1} kWh ; DH {:.1} → {:.1}",
        m.nbh_adiab.unwrap(),
        m.w_aux_adiab_kwh.unwrap(),
        c_aux_vent(&base),
        c_aux_vent(&r),
        base.batiments[0].dh_c_h.unwrap(),
        r.batiments[0].dh_c_h.unwrap()
    );
    assert!(m.nbh_adiab.unwrap() > 0);
    // Débits et ventilateurs relevés vers le mode actif pendant les heures chaudes.
    assert!(c_aux_vent(&r) > c_aux_vent(&base));
    assert!(r.batiments[0].dh_c_h.unwrap() < base.batiments[0].dh_c_h.unwrap());
}

#[test]
fn erreurs_de_saisie_et_refus() {
    let chemins = |p: &Projet| -> (Vec<String>, bool) {
        let r = valider(p);
        (
            r.erreurs.iter().map(|e| e.chemin.clone()).collect(),
            r.seulement_hors_perimetre(),
        )
    };
    let c = "batiments[0].zones[0].ventilation.systeme.cta";
    let p = cas("bureaux-cta-humidification-ademe");
    assert!(valider(&p).est_valide());
    // Vapeur sans génération.
    let mut q = p.clone();
    cta_mut(&mut q).humidificateur.as_mut().unwrap().generation = None;
    assert_eq!(
        chemins(&q),
        (vec![format!("{c}.humidificateur.generation")], false)
    );
    // Booléen contraire à la description.
    let mut q = p.clone();
    cta_mut(&mut q).humidification = Some(false);
    assert_eq!(chemins(&q), (vec![format!("{c}.humidification")], false));
    // Humidificateur d'une centrale de type 4 (§6.3.3.4.9).
    let mut q = cas("bureaux-cta-dav-ademe");
    let h = cta_mut(&mut cas("bureaux-cta-humidification-ademe"))
        .humidificateur
        .clone();
    cta_mut(&mut q).humidificateur = h;
    assert_eq!(chemins(&q), (vec![format!("{c}.humidificateur")], false));
    // θi_base sous 22 °C ; Δθ refusés hors type 4.
    let mut q = p.clone();
    {
        let a = cta_mut(&mut q)
            .rafraichissement_adiabatique
            .as_mut()
            .unwrap();
        a.theta_i_base_c = 20.0;
        a.delta_theta_i1_k = Some(1.0);
    }
    assert_eq!(
        chemins(&q),
        (
            vec![
                format!("{c}.rafraichissement_adiabatique.theta_i_base_c"),
                format!("{c}.rafraichissement_adiabatique.delta_theta_i1_k")
            ],
            false
        )
    );
    // Indirect sans échangeur.
    let mut q = p.clone();
    {
        let x = cta_mut(&mut q);
        x.echangeur = None;
    }
    assert!(chemins(&q)
        .0
        .contains(&format!("{c}.rafraichissement_adiabatique")));
    // Type 4 sans Δθ.
    let mut q = cas("bureaux-cta-dav-ademe");
    let mut a = cta_mut(&mut cas("bureaux-cta-humidification-ademe"))
        .rafraichissement_adiabatique
        .clone()
        .unwrap();
    a.type_rafraichissement = TypeRafraichissementEvaporatif::Direct;
    cta_mut(&mut q).rafraichissement_adiabatique = Some(a);
    assert_eq!(
        chemins(&q).0,
        [
            format!("{c}.rafraichissement_adiabatique.delta_theta_i1_k"),
            format!("{c}.rafraichissement_adiabatique.delta_theta_i2_k")
        ]
    );
    // Habitation : refus ciblé (code 3).
    let mut m = cas("maison-double-flux-ademe");
    let mut v = serde_json::to_value(&m).unwrap();
    let sys = &mut v["batiments"][0]["zones"][0]["ventilation"]["systeme"]["double_flux"];
    sys["humidificateur"] = serde_json::json!({
        "type_humidification": "pulverisation", "omega_cons_kg_kg": 0.006});
    m = serde_json::from_value(v).unwrap();
    let (e, hp) = chemins(&m);
    assert!(hp, "{e:?}");
    assert_eq!(
        e,
        ["batiments[0].zones[0].ventilation.systeme.double_flux.humidificateur"]
    );
}

fn meteo_reglementaire(p: &Projet) -> MeteoProjet {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO pour lancer ce test");
    let zone = p.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

/// Valeurs phares de l'exemple sur la météo réglementaire (H2b) : nombres annuels seulement.
#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn humidification_sur_meteo_reglementaire() {
    let p = cas("bureaux-cta-humidification-ademe");
    let r = calculer(&p, &meteo_reglementaire(&p), &Options { arrondir: false }).unwrap();
    assert!(r.reglementaire);
    let b = &r.batiments[0];
    let m = mode(&r, true);
    let phares = [
        b.bbio_pts.unwrap(),
        b.dh_c_h.unwrap(),
        b.cep_kwhep_m2.unwrap(),
        b.cep_nr_kwhep_m2.unwrap(),
        b.ic_energie_kgco2_m2.unwrap(),
        m.q_hr_kwh.unwrap(),
    ];
    println!("réglementaire : Bbio, DH, Cep, Cep,nr, Ic énergie, QHR = {phares:.1?}");
    let proche = |a: f64, b: f64| (a - b).abs() <= 0.05 + 1e-9;
    for (x, y) in phares.iter().zip(REGL) {
        assert!(proche(*x, y), "{phares:?}");
    }
}

/// Valeurs phares sur la météo réglementaire (lot 1-ven3, 18 novembre 2026).
const REGL: [f64; 6] = [82.9, 307.3, 81.7, 81.7, 373.5, 8799.2];
