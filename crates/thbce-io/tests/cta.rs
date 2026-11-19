// SPDX-License-Identifier: GPL-3.0-or-later
//! Double flux et centrales de traitement d'air hors habitation (lot 1-ven, fiche 6.3 en zone
//! neutre, réseaux de CTA de la fiche 8.11) : bout en bout sur l'exemple
//! `tests/cas/bureaux-cta-ademe`, météo ouverte ADEME ; consommations recoupées à la main ;
//! sens des effets ; refus de ce qui reste hors périmètre. Les valeurs de l'exemple sont
//! vérifiées par son `attendu.json`.
//!
//! Valeurs phares sur la météo réglementaire :
//! THBCE_METEO=/chemin/vers/meteo.csv cargo t-regl cta_sur_meteo_reglementaire

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::bati::assemblage_batiment::BatimentHoraire;
use thbce_core::bati::assemblage_groupe::{LectureAntigelCta, LectureRelanceCta, Lectures};
use thbce_core::climat::{ClimatExterieur, ConfortAdaptatif, MeteoProjet};
use thbce_core::conventions::ModeCalcul;
use thbce_core::entree::{Cta, DoubleFlux, Projet, SystemeVentilation, TypeCta, UsageZone};
use thbce_core::resultats::Resultats;
use thbce_core::{calculer, valider, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bureaux_cta() -> Projet {
    let chemin = racine().join("tests/cas/bureaux-cta-ademe/entree.json");
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn meteo_ademe(p: &Projet) -> MeteoProjet {
    let zone = p.localisation.zone_climatique_retenue();
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

fn calcul(p: &Projet) -> Resultats {
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

/// Consommation annuelle des auxiliaires de ventilation de la zone, en kWh.
fn c_aux_vent(r: &Resultats) -> f64 {
    r.batiments[0].zones[0]
        .ventilation
        .as_ref()
        .unwrap()
        .c_aux_vent_kwh
}

fn phares(r: &Resultats) -> [f64; 5] {
    let b = &r.batiments[0];
    [
        b.bbio_pts.unwrap(),
        b.dh_c_h.unwrap(),
        b.cep_kwhep_m2.unwrap(),
        b.cep_nr_kwhep_m2.unwrap(),
        b.ic_energie_kgco2_m2.unwrap(),
    ]
}

#[test]
fn la_cta_se_calcule_de_bout_en_bout() {
    let p = bureaux_cta();
    let rapport = valider(&p);
    assert!(rapport.est_valide(), "{:?}", rapport.erreurs);
    let r = calcul(&p);
    let z = &r.batiments[0].zones[0];
    let v = z.ventilation.as_ref().unwrap();
    let th_c = v.th_c.unwrap();
    // Préchauffage actif en saison de chauffage, demandé à la chaudière (éq. 523 et 939).
    let q_pre = th_c.q_prechaud_kwh.unwrap();
    assert!(q_pre > 0.0);
    assert!(th_c.q_prefroid_kwh.is_none());
    let g = &r.batiments[0].generations[0];
    assert!(g.q_ch_kwh >= q_pre + th_c.q_ag_kwh);
    // Consommation relevée heure par heure : la somme des groupes est celle de la zone, et la
    // somme des mois celle du groupe (éq. 565 et 570).
    let vg = z.groupes[0].ventilation.as_ref().unwrap();
    assert!((vg.c_aux_vent_kwh - v.c_aux_vent_kwh).abs() < 1e-6);
    let mensuel: f64 = vg.c_aux_vent_mensuel_kwh.iter().sum();
    assert!((mensuel - vg.c_aux_vent_kwh).abs() < 1e-6);
    // Débits restitués : ceux du mode actif (5 000 × 1,15 m³/h plus les fuites), vus par le
    // ventilateur au moins égaux aux débits reçus par le groupe.
    assert!(vg.q_souffle_cond_m3_h >= vg.q_spec_souffle_m3_h);
    assert!(vg.q_souffle_cond_m3_h > 5750.0);
    // Entre la puissance de zone neutre en inoccupation (260 W) toute l'année et celle du mode
    // actif (3 000 W) toute l'année.
    let c = c_aux_vent(&r);
    assert!(c > 260.0 * 8.76 && c < 3000.0 * 8.76, "{c}");
}

/// Consommations des ventilateurs et de l'échangeur rotatif recoupées à la main : avec des
/// puissances égales dans tous les régimes, la consommation ne dépend plus des régimes
/// (éq. 460 à 465 et 565) ; un échangeur rotatif sans bypass ajoute sa puissance à chaque
/// heure (éq. 497).
#[test]
fn consommations_a_la_main() {
    let mut p = bureaux_cta();
    {
        let c = cta_mut(&mut p);
        c.p_vent_rep_ch_w = 400.0;
        c.p_vent_sou_ch_w = 600.0;
        c.p_vent_rep_zn_occ_w = 400.0;
        c.p_vent_sou_zn_occ_w = 600.0;
        c.p_vent_rep_zn_inocc_w = 400.0;
        c.p_vent_sou_zn_inocc_w = 600.0;
    }
    assert!((c_aux_vent(&calcul(&p)) - 8760.0).abs() < 1e-6);
    {
        let e = cta_mut(&mut p).echangeur.as_mut().unwrap();
        e.bypass = None;
        e.rotatif = Some(true);
        e.p_ech_w = Some(100.0);
    }
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);
    let r = calcul(&p);
    assert!((c_aux_vent(&r) - 8760.0 * 1.1).abs() < 1e-6);
    // Tableau 62 : −5 °C pour un échangeur rotatif en bâtiment non résidentiel.
    let v = r.batiments[0].zones[0].ventilation.as_ref().unwrap();
    let d = v
        .valeurs_par_defaut
        .iter()
        .find(|d| d.champ.ends_with("theta_s_ech_rep_lim_c"))
        .unwrap();
    assert_eq!(d.valeur, -5.0);
}

/// Sens des effets.
#[test]
fn sens_des_effets() {
    let p = bureaux_cta();
    let r = calcul(&p);
    let [bbio, dh, ..] = phares(&r);
    // Débit variable (type 4) : débits de zone neutre en toute circonstance, ventilateurs de
    // zone neutre ; moins de consommation des ventilateurs ; Bbio inchangé (conventionnel).
    let mut q = p.clone();
    {
        let c = cta_mut(&mut q);
        c.type_cta = TypeCta::DebitVariable;
        c.batterie_prechauffage = None;
    }
    assert!(valider(&q).est_valide(), "{:?}", valider(&q).erreurs);
    let rq = calcul(&q);
    assert!(c_aux_vent(&rq) < c_aux_vent(&r));
    assert_eq!(phares(&rq)[0], bbio);
    // Régulation standard du taux d'air neuf : moins de free cooling, confort d'été dégradé.
    let mut q = p.clone();
    cta_mut(&mut q).regulation_air_neuf = Some(thbce_core::entree::RegulationAirNeuf::Standard);
    assert!(phares(&calcul(&q))[1] >= dh);
    // Sans préchauffage, la génération reçoit moins de demande de chauffage.
    let mut q = p.clone();
    cta_mut(&mut q).batterie_prechauffage = None;
    let rq = calcul(&q);
    assert!(rq.batiments[0].generations[0].q_ch_kwh < r.batiments[0].generations[0].q_ch_kwh);
    assert!(rq.batiments[0].zones[0]
        .ventilation
        .as_ref()
        .unwrap()
        .th_c
        .unwrap()
        .q_prechaud_kwh
        .is_none());
}

/// Batterie de prérefroidissement d'une centrale reliée à un groupe climatisé : énergie demandée
/// à une génération de froid par un réseau de CTA fictif (éq. 539 à 548 et 939, lecture Q91).
#[test]
fn prerefroidissement_vers_une_generation_de_froid() {
    let mut v = serde_json::to_value(bureaux_cta()).unwrap();
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
    g["emetteurs"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id": "ventilo-convecteurs", "refroidissement": {
                "type_emetteur": "soufflage_air_froid",
                "variation_spatiale": {"classe": {"classe": "B", "hsp_m": 2.7}},
                "variation_temporelle": {"defaut": {"arret_total": true}},
                "rats": 1, "ratt": 1, "distribution": "d-fr"}
        }));
    b["zones"][0]["ventilation"]["systeme"]["cta"]["batterie_prerefroidissement"] = serde_json::json!({"theta_cons_c": 19, "theta_ex_c": 24, "theta_dim_fr_c": 7,
            "generation": "groupe-froid"});
    let p: Projet = serde_json::from_value(v).unwrap();
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);
    let r = calcul(&p);
    let th_c = r.batiments[0].zones[0]
        .ventilation
        .as_ref()
        .unwrap()
        .th_c
        .unwrap();
    let q_fr = th_c.q_prefroid_kwh.unwrap();
    println!("prérefroidissement {q_fr:.1} kWh");
    assert!(q_fr < 0.0);
    let gf = r.batiments[0]
        .generations
        .iter()
        .find(|g| g.id == "groupe-froid")
        .unwrap();
    assert!(gf.q_fr_kwh >= -q_fr - 1e-9, "{} {q_fr}", gf.q_fr_kwh);
    // Sans prérefroidissement, la génération de froid reçoit moins de demande.
    let mut q = p.clone();
    cta_mut(&mut q).batterie_prerefroidissement = None;
    let rq = calcul(&q);
    let gq = rq.batiments[0]
        .generations
        .iter()
        .find(|g| g.id == "groupe-froid")
        .unwrap();
    assert!(gq.q_fr_kwh < gf.q_fr_kwh);
}

/// Double flux (type 1) hors habitation : débits et puissances d'occupation et
/// d'inoccupation (éq. 458 et 459).
#[test]
fn double_flux_non_residentiel() {
    let mut p = bureaux_cta();
    let v = p.batiments[0].zones[0].ventilation.as_mut().unwrap();
    let echangeur = match &v.systeme {
        SystemeVentilation::Cta(c) => c.echangeur.clone(),
        _ => None,
    };
    v.systeme = SystemeVentilation::DoubleFlux(DoubleFlux {
        p_vent_base_rep_w: None,
        p_vent_pointe_rep_w: None,
        p_vent_base_sou_w: None,
        p_vent_pointe_sou_w: None,
        p_vent_rep_occ_w: Some(500.0),
        p_vent_sou_occ_w: Some(500.0),
        p_vent_rep_inocc_w: Some(50.0),
        p_vent_sou_inocc_w: Some(50.0),
        echangeur,
        prechauffage: None,
        batterie_prechauffage: None,
        batterie_prerefroidissement: None,
        humidification: None,
        humidificateur: None,
        rafraichissement_evaporatif: None,
        rafraichissement_adiabatique: None,
        surventilation_mecanique: None,
        rafraichissement_surventilation: None,
    });
    for b in &mut p.batiments[0].zones[0].groupes[0].bouches_conduits {
        b.q_spec_ch_m3_h = None;
    }
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);
    let r = calcul(&p);
    // 1 000 W pendant les 2 610 h de ventilation en occupation, 100 W le reste de l'année
    // (comme l'exemple simple flux de bureaux).
    let attendu = (1000.0 * 2610.0 + 100.0 * (8760.0 - 2610.0)) / 1000.0;
    assert!(
        (c_aux_vent(&r) - attendu).abs() < 1e-6,
        "{}",
        c_aux_vent(&r)
    );
}

/// Ce qui reste refusé, et les erreurs de saisie propres aux centrales.
#[test]
fn refus_et_erreurs() {
    let chemins = |p: &Projet| -> (Vec<String>, bool) {
        let r = valider(p);
        (
            r.erreurs.iter().map(|e| e.chemin.clone()).collect(),
            r.seulement_hors_perimetre(),
        )
    };
    let c = "batiments[0].zones[0].ventilation.systeme.cta";
    // Mode actif des centrales à charge variable (lot 1-ven2) : génération et émetteur contenu
    // dans les bouches requis (erreurs de saisie ; voir `cta_mode_actif.rs`).
    let mut p = bureaux_cta();
    {
        let x = cta_mut(&mut p);
        x.type_cta = TypeCta::DebitConstantTemperatureVariable;
        x.batterie_prechauffage = None;
        x.batterie_chaude = Some(true);
    }
    assert_eq!(
        chemins(&p),
        (
            vec![
                format!("{c}.batterie_chaude"),
                format!("{c}.generation_batterie_chaude")
            ],
            false
        )
    );
    // Humidification sans description (lot 1-ven3) : erreur de saisie.
    let mut p = bureaux_cta();
    cta_mut(&mut p).humidification = Some(true);
    assert_eq!(chemins(&p), (vec![format!("{c}.humidification")], false));
    // Préchauffage d'une centrale à température variable : erreur (§6.3.3.4.9).
    let mut p = bureaux_cta();
    cta_mut(&mut p).type_cta = TypeCta::DebitConstantTemperatureVariable;
    assert_eq!(
        chemins(&p),
        (vec![format!("{c}.batterie_prechauffage")], false)
    );
    // Prérefroidissement sans groupe climatisé (éq. 389).
    let mut p = bureaux_cta();
    cta_mut(&mut p).batterie_prerefroidissement =
        Some(thbce_core::entree::BatteriePrerefroidissement {
            theta_cons_c: 18.0,
            theta_ex_c: 24.0,
            theta_dim_fr_c: 7.0,
            generation: "chaufferie".into(),
        });
    let (e, hors) = chemins(&p);
    assert!(!hors);
    assert!(
        e.contains(&format!("{c}.batterie_prerefroidissement")),
        "{e:?}"
    );
    // La chaudière n'assure pas le refroidissement.
    assert!(
        e.contains(&format!("{c}.batterie_prerefroidissement.generation")),
        "{e:?}"
    );
    // Débit du mode actif manquant ; débit de zone neutre supérieur.
    let mut p = bureaux_cta();
    p.batiments[0].zones[0].groupes[0].bouches_conduits[0].q_spec_ch_m3_h = None;
    let g = "batiments[0].zones[0].groupes[0].bouches_conduits[0]";
    assert_eq!(chemins(&p).0, [format!("{g}.q_spec_ch_m3_h")]);
    let mut p = bureaux_cta();
    p.batiments[0].zones[0].groupes[0].bouches_conduits[0].q_spec_occ_m3_h = Some(6000.0);
    assert_eq!(chemins(&p).0, [format!("{g}.q_spec_occ_m3_h")]);
    // Centrale en habitation : erreur.
    let mut p = bureaux_cta();
    p.batiments[0].zones[0].usage = UsageZone::LogementCollectif;
    assert!(chemins(&p)
        .0
        .contains(&"batiments[0].zones[0].ventilation.systeme".to_owned()));
    // Forme de la version 0.1 : erreur de saisie, plus un refus.
    let mut p = bureaux_cta();
    p.batiments[0].zones[0]
        .ventilation
        .as_mut()
        .unwrap()
        .systeme = SystemeVentilation::CentraleTraitementAir;
    let (e, hors) = chemins(&p);
    assert!(!hors);
    assert!(e.contains(&"batiments[0].zones[0].ventilation.systeme".to_owned()));
}

/// Calcul des consommations (Th-C) de l'exemple pour des lectures données.
fn th_c_selon(p: &Projet, lectures: Lectures) -> BatimentHoraire {
    let m = meteo_ademe(p);
    let loc = &p.localisation;
    let climat = ClimatExterieur::nouveau(&m.th_bc, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    let mut bat = BatimentHoraire::nouveau_selon(
        &p.batiments[0],
        &p.catalogue,
        ModeCalcul::ThC,
        lectures,
        loc.alt_m,
        climat.we_kg_kg[0],
    )
    .unwrap();
    bat.simuler(&climat, &confort).unwrap();
    bat
}

/// Variantes écartées, chiffrées sur l'exemple (calcul des consommations, météo ADEME) :
/// relance anticipée au pas en cours (Q90 point 1) et température de l'éq. (951) pour
/// l'antigel quand le préchauffage passe par la même génération (Q93 point 3).
#[test]
fn variantes_ecartees_q90_q93() {
    let p = bureaux_cta();
    let d = Lectures::default();
    let resume = |b: &BatimentHoraire| {
        let z = &b.zones[0];
        let w: f64 = z
            .ventilation
            .as_ref()
            .unwrap()
            .releve
            .as_ref()
            .unwrap()
            .serie_wh
            .iter()
            .sum();
        (w / 1000.0, z.cumuls_ventilation.q_prechaud_wh / 1000.0)
    };
    let base = th_c_selon(&p, d);
    let anticipee = th_c_selon(
        &p,
        Lectures {
            relance_cta: LectureRelanceCta::Anticipee,
            ..d
        },
    );
    let (w0, q0) = resume(&base);
    let (w1, q1) = resume(&anticipee);
    println!(
        "Q90 point 1 : ventilateurs {w0:.1} → {w1:.1} kWh ({:+.2} %) ; préchauffage {q0:.1} → \
         {q1:.1} kWh ({:+.2} %)",
        100.0 * (w1 / w0 - 1.0),
        100.0 * (q1 / q0 - 1.0)
    );
    let melange = th_c_selon(
        &p,
        Lectures {
            antigel_cta: LectureAntigelCta::MelangeSiPrechauffage,
            ..d
        },
    );
    // La chaudière dessert aussi un réseau hydraulique : ses températures de fonctionnement
    // sont celles des réseaux (éq. 1009 à 1011), la température d'air équivalente n'intervient
    // pas (éq. 1012 et 1013) ; effet nul.
    assert_eq!(
        format!("{:?}", base.systemes.cumuls_cef_groupes),
        format!("{:?}", melange.systemes.cumuls_cef_groupes)
    );
}

fn meteo_reglementaire(p: &Projet) -> MeteoProjet {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO pour lancer ce test");
    let zone = p.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

/// Valeurs phares de l'exemple de CTA sur la météo réglementaire (H2b) : nombres annuels
/// seulement.
#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn cta_sur_meteo_reglementaire() {
    let p = bureaux_cta();
    let r = calculer(&p, &meteo_reglementaire(&p), &Options { arrondir: false }).unwrap();
    assert!(r.reglementaire);
    let [bbio, dh, cep, cep_nr, ic] = phares(&r);
    let v = r.batiments[0].zones[0].ventilation.as_ref().unwrap();
    let pre = v.th_c.unwrap().q_prechaud_kwh.unwrap();
    println!(
        "réglementaire : Bbio {bbio:.1} ; DH {dh:.1} ; Cep {cep:.1} ; Cep,nr {cep_nr:.1} ; Ic \
         {ic:.1} ; ventilateurs {:.1} kWh ; préchauffage {pre:.1} kWh",
        v.c_aux_vent_kwh
    );
    let proche = |a: f64, b: f64| (a - b).abs() <= 0.05 + 1e-9;
    assert!(proche(bbio, REGL[0]), "Bbio {bbio}");
    assert!(proche(dh, REGL[1]), "DH {dh}");
    assert!(proche(cep, REGL[2]), "Cep {cep}");
    assert!(proche(cep_nr, REGL[3]), "Cep,nr {cep_nr}");
    assert!(proche(ic, REGL[4]), "Ic énergie {ic}");
}

/// Valeurs phares attendues sur la météo réglementaire (lot 1-ven, 18 novembre 2026) : Bbio,
/// DH, Cep, Cep,nr, Ic énergie.
const REGL: [f64; 5] = [82.9, 582.9, 75.3, 75.3, 300.3];
