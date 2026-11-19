// SPDX-License-Identifier: GPL-3.0-or-later
//! Système de ventilation du projet (Th-BCE 2020 §6.2 à 6.5, lot 6a) : débits, fuites,
//! puissances et températures de l'air soufflé calculés à la main d'après les équations, et
//! règles de validation.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::entree::{
    BoucheConduit, Bypass, BypassPuits, ClasseEtancheite, DoubleFlux, Echangeur,
    EchangeurSimplifie, Isouf, Projet, Puits, PuitsClimatique, RepresentationEchangeur,
    StatutEchangeur, SystemeVentilation, TypeSolPuits, UsageZone, VentilateurSimpleFlux, Zone,
};
use thbce_core::valider;
use thbce_core::ventilation::mecanique_sf::{rho_kg_m3, C_PA_J_KG_K, P_EL};
use thbce_core::ventilation::ventilation_projet::{
    AirSouffle, EtatGroupeVentilation, VentilationProjet,
};

const ENTREE: &str = include_str!("../../../tests/cas/squelette-maison/entree.json");

fn projet() -> Projet {
    serde_json::from_str(ENTREE).unwrap()
}

fn zone(p: &Projet) -> &Zone {
    &p.batiments[0].zones[0]
}

fn zone_mut(p: &mut Projet) -> &mut Zone {
    &mut p.batiments[0].zones[0]
}

fn proche(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} au lieu de {b}");
}

/// `80^0,667` : différence de pression d'un réseau haute pression en maison (tableau 60).
fn dp_maison() -> f64 {
    libm::pow(80.0, 0.667)
}

#[test]
fn simple_flux_autoreglable_a_la_main() {
    let p = projet();
    let v = VentilationProjet::nouvelle(zone(&p), 18.0).unwrap();
    // Éq. (387) à (390) et (403) : (180 × 14 + 105 × 154)/168 = 111,25 m³/h ; éq. (412),
    // Cdep = 1,15 saisi : 127,9375 m³/h.
    let q_dep = 1.15 * 111.25;
    // Éq. (421) : 96,44 × 0,05 m² de conduit ; éq. (419), Kres par défaut.
    let a = 96.44 * 0.05;
    let fuites = 3600.0 * 0.0675e-3 * a * dp_maison();
    let d = v.debits_groupe(0);
    // Éq. (428), Ratfuitevc = 0,25 par défaut.
    proche(d.q_rep_m3_h, -(q_dep + 0.25 * fuites));
    assert_eq!(d.q_souffle_m3_h, 0.0);
    // Éq. (430).
    proche(v.debits_cond_groupe(0).q_rep_m3_h, -(q_dep + fuites));
    // Éq. (641) : (30 × 14 + 15 × 154)/168 = 16,25 W ; éq. (642) : tout au seul groupe.
    proche(v.p_vent_w(), 16.25);
    proche(v.p_vent_groupe_w(0), 16.25);
    // Valeurs par défaut signalées : Kres, Ratfuitevc, Dugd, surface de conduit.
    let champs: Vec<_> = v
        .valeurs_par_defaut
        .iter()
        .map(|d| d.champ.as_str())
        .collect();
    assert_eq!(
        champs,
        [
            "ventilation.reseau.classe_etancheite",
            "ventilation.reseau.ratfuitevc",
            "groupes[0].bouches_conduits[0].dugd_h",
            "groupes[0].bouches_conduits[0].a_cond_m2",
        ]
    );
}

#[test]
fn hygroreglable_debit_regule_egal_a_qvarepspec() {
    let mut p = projet();
    let b = &mut zone_mut(&mut p).groupes[0].bouches_conduits[0];
    b.q_spec_conv_base_m3_h = None;
    b.qvarep_spec_m3_h = Some(92.0);
    b.dugd_h = Some(7.0);
    b.cdep = Some(1.0);
    zone_mut(&mut p)
        .ventilation
        .as_mut()
        .unwrap()
        .reseau
        .ratfuitevc = Some(0.0);
    b_surface(&mut p, 0.0);
    let v = VentilationProjet::nouvelle(zone(&p), 18.0).unwrap();
    // Sans fuite (surface nulle) ni dépassement : le débit régulé est Qvarepspec, quel que
    // soit Dugd (§6.2.3.1.1.2, lecture Q36 point 3).
    proche(v.debits_groupe(0).q_rep_m3_h, -92.0);
    // Puissance moyenne avec Dugd = 7 h (éq. 640 et 641).
    proche(v.p_vent_w(), (30.0 * 7.0 + 15.0 * 161.0) / 168.0);
}

fn b_surface(p: &mut Projet, a: f64) {
    for g in &mut zone_mut(p).groupes {
        for b in &mut g.bouches_conduits {
            b.a_cond_m2 = Some(a);
        }
    }
}

#[test]
fn surfaces_saisies_et_classe_d_etancheite() {
    let mut p = projet();
    b_surface(&mut p, 3.0);
    let r = &mut zone_mut(&mut p).ventilation.as_mut().unwrap().reseau;
    r.classe_etancheite = Some(ClasseEtancheite::B);
    r.ratfuitevc = Some(0.5);
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);
    let v = VentilationProjet::nouvelle(zone(&p), 18.0).unwrap();
    let fuites = 3600.0 * 0.009e-3 * 3.0 * dp_maison();
    proche(
        v.debits_groupe(0).q_rep_m3_h,
        -(1.15 * 111.25 + 0.5 * fuites),
    );
    assert!(v
        .valeurs_par_defaut
        .iter()
        .all(|d| !d.champ.contains("reseau") && !d.champ.contains("a_cond")));
}

#[test]
fn logement_collectif_surface_par_debit_maximal() {
    let mut p = projet();
    let z = zone_mut(&mut p);
    z.usage = UsageZone::LogementCollectif;
    let v = VentilationProjet::nouvelle(zone(&p), 18.0).unwrap();
    // Éq. (424) : 180 × 0,05 = 9 m² ; tableau 60 : 160 Pa, Ratfuitevc = 0,5.
    let fuites = 3600.0 * 0.0675e-3 * 9.0 * libm::pow(160.0, 0.667);
    proche(
        v.debits_groupe(0).q_rep_m3_h,
        -(1.15 * 111.25 + 0.5 * fuites),
    );
    // Basse pression : 20 Pa (tableau 60, lecture Q42).
    zone_mut(&mut p).ventilation.as_mut().unwrap().systeme =
        SystemeVentilation::MecaniqueBassePression(VentilateurSimpleFlux {
            p_vent_base_w: Some(15.0),
            p_vent_pointe_w: Some(30.0),
            p_vent_occ_w: None,
            p_vent_inocc_w: None,
            rafraichissement_surventilation: None,
        });
    let v = VentilationProjet::nouvelle(zone(&p), 18.0).unwrap();
    let fuites = 3600.0 * 0.0675e-3 * 9.0 * libm::pow(20.0, 0.667);
    proche(
        v.debits_groupe(0).q_rep_m3_h,
        -(1.15 * 111.25 + 0.5 * fuites),
    );
}

/// Double flux de 100 m³/h équilibré, sans fuite ni perte de conduit (surfaces nulles,
/// `Cdep = 1`), ventilateurs de 20 W, échangeur certifié à 0,8.
fn double_flux(bypass: Option<Bypass>, antigel: bool) -> Projet {
    let mut p = projet();
    let z = zone_mut(&mut p);
    let reprise = BoucheConduit {
        id: "reprise".into(),
        isouf: Isouf::Reprise,
        q_spec_conv_pointe_m3_h: 100.0,
        q_spec_conv_base_m3_h: Some(100.0),
        qvarep_spec_m3_h: None,
        dugd_h: None,
        cdep: Some(1.0),
        a_cond_m2: Some(0.0),
        q_spec_occ_m3_h: None,
        q_spec_inocc_m3_h: None,
        q_spec_ch_m3_h: None,
        crdbnr: None,
        q_spec_rafnoc_m3_h: None,
    };
    z.groupes[0].bouches_conduits = vec![
        reprise.clone(),
        BoucheConduit {
            id: "soufflage".into(),
            isouf: Isouf::Soufflage,
            ..reprise
        },
    ];
    z.groupes[0].entrees_air = None;
    let v = z.ventilation.as_mut().unwrap();
    v.systeme = SystemeVentilation::DoubleFlux(DoubleFlux {
        p_vent_base_rep_w: Some(20.0),
        p_vent_pointe_rep_w: Some(20.0),
        p_vent_base_sou_w: Some(20.0),
        p_vent_pointe_sou_w: Some(20.0),
        p_vent_rep_occ_w: None,
        p_vent_sou_occ_w: None,
        p_vent_rep_inocc_w: None,
        p_vent_sou_inocc_w: None,
        echangeur: Some(Echangeur {
            representation: RepresentationEchangeur::Simplifiee(EchangeurSimplifie {
                epsilon_saisi: 0.8,
                statut: Some(StatutEchangeur::Certifie),
            }),
            rotatif: None,
            p_ech_w: None,
            bypass,
            antigel: Some(antigel),
            generation_antigel: antigel.then(|| "generation-effet-joule".into()),
            theta_s_ech_rep_lim_c: None,
        }),
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
    v.reseau.ratfuitevc = Some(1.0);
    v.reseau.r_souf_m2k_w = Some(0.6);
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);
    p
}

fn un_pas(p: &Projet, theta_ext: f64, theta_i: f64, chauffage: bool) -> (AirSouffle, f64, f64) {
    let mut v = VentilationProjet::nouvelle(zone(p), 18.0).unwrap();
    let mut air = [AirSouffle::default()];
    let s = v.pas(
        theta_ext,
        0.004,
        &[EtatGroupeVentilation {
            theta_i_fin_c: theta_i,
            omega_i_fin_kg_kg: 0.008,
            saison_de_chauffage: chauffage,
            ..EtatGroupeVentilation::default()
        }],
        &mut air,
        true,
    );
    assert_eq!(air[0].omega_air_souffle_kg_kg, 0.004);
    (air[0], s.epsilon, s.w_ag_wh)
}

#[test]
fn double_flux_un_pas_a_la_main() {
    let p = double_flux(None, false);
    // Air repris à 20 °C : 100 m³/h à ρ(20 °C) = 1,2 kg/m³ (éq. 412 et 443).
    let qm_rep = 1.2 * 100.0 / 3600.0;
    // Ventilateur d'extraction (éq. 474) : +0,8 × 20/(1006 × qm).
    let t_extr2 = 20.0 + P_EL * 20.0 / (C_PA_J_KG_K * qm_rep);
    // Air soufflé au premier pas : masse volumique à 18 °C (éq. 420, état initial).
    let qm_sou = rho_kg_m3(18.0) * 100.0 / 3600.0;
    // Air neuf à 5 °C : au-dessus du seuil de givrage (éq. 506), ε = 0,8 (éq. 396 et 498).
    let seuil = t_extr2 + (5.0 - t_extr2) / 0.8;
    assert!(seuil < 5.0);
    let (air, eps, w_ag) = un_pas(&p, 5.0, 20.0, true);
    assert_eq!((eps, w_ag), (0.8, 0.0));
    // Éq. (512), puis ventilateur de soufflage (éq. 519).
    let t_souf1 = 5.0 + 0.8 * (t_extr2 - 5.0);
    proche(
        air.t_air_souffle_c,
        t_souf1 + P_EL * 20.0 / (C_PA_J_KG_K * qm_sou),
    );

    // Air neuf à 0 °C, sous le seuil de givrage, sans antigel : échangeur contourné (éq. 509).
    let (air, eps, _) = un_pas(&p, 0.0, 20.0, true);
    assert_eq!(eps, 0.0);
    proche(air.t_air_souffle_c, P_EL * 20.0 / (C_PA_J_KG_K * qm_sou));
    // Avec antigel, en saison de chauffage : air neuf préchauffé au seuil (éq. 507 et 508).
    let p = double_flux(None, true);
    let (air, eps, w_ag) = un_pas(&p, 0.0, 20.0, true);
    assert_eq!(eps, 0.8);
    proche(w_ag, C_PA_J_KG_K * qm_sou * seuil);
    proche(
        air.t_air_souffle_c,
        seuil + 0.8 * (t_extr2 - seuil) + P_EL * 20.0 / (C_PA_J_KG_K * qm_sou),
    );
    // Hors saison de chauffage, l'antigel n'agit pas : échangeur contourné.
    let (_, eps, w_ag) = un_pas(&p, 0.0, 20.0, false);
    assert_eq!((eps, w_ag), (0.0, 0.0));
}

#[test]
fn double_flux_bypass_d_ete() {
    let by = Bypass {
        theta_ext_bypass_hiver_c: 15.0,
        theta_ext_bypass_ete_c: 12.0,
        theta_int_bypass_hiver_c: 24.0,
        theta_int_bypass_ete_c: 22.0,
    };
    let p = double_flux(Some(by), false);
    // Nuit d'été : air neuf à 16 °C, intérieur à 26 °C : échangeur contourné (éq. 494, 496).
    let (air, eps, _) = un_pas(&p, 16.0, 26.0, false);
    assert_eq!(eps, 0.0);
    let qm_sou = rho_kg_m3(18.0) * 100.0 / 3600.0;
    proche(
        air.t_air_souffle_c,
        16.0 + P_EL * 20.0 / (C_PA_J_KG_K * qm_sou),
    );
    // Même nuit en saison de chauffage : consignes d'hiver, intérieur sous 24 °C, échangeur
    // actif.
    let (_, eps, _) = un_pas(&p, 16.0, 23.0, true);
    assert_eq!(eps, 0.8);
}

#[test]
fn pertes_des_conduits_de_soufflage() {
    let mut p = double_flux(None, false);
    // Conduit de soufflage de 4 m² entièrement hors volume chauffé (Ratfuitevc = 0), conduit
    // de reprise sans surface.
    for b in &mut zone_mut(&mut p).groupes[0].bouches_conduits {
        b.a_cond_m2 = Some(if b.isouf == Isouf::Soufflage {
            4.0
        } else {
            0.0
        });
    }
    let r = &mut zone_mut(&mut p).ventilation.as_mut().unwrap().reseau;
    r.ratfuitevc = Some(0.0);
    r.classe_etancheite = Some(ClasseEtancheite::C);
    let (air, _, _) = un_pas(&p, 5.0, 20.0, true);
    let (sans_pertes, _, _) = un_pas(&double_flux(None, false), 5.0, 20.0, true);
    // Combles à la température extérieure (b = 1) : l'air soufflé se refroidit (éq. 562), et
    // les fuites hors volume chauffé réduisent le débit reçu par le groupe (éq. 429).
    assert!(air.t_air_souffle_c < sans_pertes.t_air_souffle_c);
    assert!(air.t_air_souffle_c > 5.0);
    let v = VentilationProjet::nouvelle(zone(&p), 18.0).unwrap();
    let d = v.debits_groupe(0);
    proche(d.q_souffle_m3_h, 100.0);
    assert!(v.debits_cond_groupe(0).q_souffle_m3_h > 100.0);
}

fn chemins(p: &Projet) -> Vec<String> {
    valider(p).erreurs.into_iter().map(|e| e.chemin).collect()
}

#[test]
fn validation_du_systeme_de_ventilation() {
    // Requis en habitation.
    let mut p = projet();
    zone_mut(&mut p).ventilation = None;
    assert_eq!(chemins(&p), ["batiments[0].zones[0].ventilation"]);
    // Systèmes hors de la première version : refusés avec un message explicite.
    for s in [
        SystemeVentilation::NaturelleParConduits,
        SystemeVentilation::Hybride,
        SystemeVentilation::CentraleTraitementAir,
    ] {
        let mut p = projet();
        zone_mut(&mut p).ventilation.as_mut().unwrap().systeme = s;
        let e = valider(&p).erreurs;
        assert_eq!(e.len(), 1, "{e:?}");
        assert_eq!(e[0].chemin, "batiments[0].zones[0].ventilation.systeme");
        assert!(
            e[0].message.contains("non pris en charge") || e[0].message.contains("non résidentiel")
        );
    }
    // Puits hydraulique (fiche 6.9), pas encore pris en charge : refusé.
    let mut p = projet();
    zone_mut(&mut p).ventilation.as_mut().unwrap().puits = Some(Puits::Hydraulique);
    assert_eq!(chemins(&p), ["batiments[0].zones[0].ventilation.puits"]);
    assert!(valider(&p).erreurs[0].hors_perimetre);
    // Sens des bouches : un double flux souffle et extrait.
    let mut p = double_flux(None, false);
    zone_mut(&mut p).groupes[0].bouches_conduits.pop();
    assert_eq!(chemins(&p), ["batiments[0].zones[0].ventilation.systeme"]);
    // Surventilation mécanique annoncée sans sa description (§6.12, lot 1-ven6) : erreur de
    // saisie.
    let mut p = double_flux(None, false);
    if let SystemeVentilation::DoubleFlux(d) =
        &mut zone_mut(&mut p).ventilation.as_mut().unwrap().systeme
    {
        d.surventilation_mecanique = Some(true);
        d.echangeur.as_mut().unwrap().rotatif = Some(true);
    }
    // Échangeur rotatif : sa puissance est requise (éq. 497).
    assert_eq!(
        chemins(&p),
        [
            "batiments[0].zones[0].ventilation.systeme.double_flux.surventilation_mecanique",
            "batiments[0].zones[0].ventilation.systeme.double_flux.echangeur.p_ech_w",
        ]
    );
    // Surfaces de conduits pour toutes les bouches ou aucune, avec Ratfuitevc.
    let mut p = projet();
    zone_mut(&mut p).groupes[0].bouches_conduits[0].a_cond_m2 = Some(2.0);
    assert_eq!(
        chemins(&p),
        ["batiments[0].zones[0].ventilation.reseau.ratfuitevc"]
    );
    // Résistance des conduits de soufflage requise pour une insufflation.
    let mut p = projet();
    zone_mut(&mut p).groupes[0].bouches_conduits[0].isouf = Isouf::Soufflage;
    zone_mut(&mut p).ventilation.as_mut().unwrap().systeme =
        SystemeVentilation::SimpleFluxInsufflation(VentilateurSimpleFlux {
            p_vent_base_w: Some(15.0),
            p_vent_pointe_w: Some(30.0),
            p_vent_occ_w: None,
            p_vent_inocc_w: None,
            rafraichissement_surventilation: None,
        });
    assert_eq!(
        chemins(&p),
        ["batiments[0].zones[0].ventilation.reseau.r_souf_m2k_w"]
    );
    // Dugd et Cdep.
    let mut p = projet();
    let b = &mut zone_mut(&mut p).groupes[0].bouches_conduits[0];
    b.dugd_h = Some(168.0);
    b.cdep = Some(0.0);
    assert_eq!(
        chemins(&p),
        [
            "batiments[0].zones[0].groupes[0].bouches_conduits[0].dugd_h",
            "batiments[0].zones[0].groupes[0].bouches_conduits[0].cdep",
        ]
    );
}

#[test]
fn simple_flux_par_insufflation() {
    let mut p = projet();
    let z = zone_mut(&mut p);
    let b = &mut z.groupes[0].bouches_conduits[0];
    b.isouf = Isouf::Soufflage;
    b.cdep = Some(1.0);
    b.a_cond_m2 = Some(0.0);
    let v = z.ventilation.as_mut().unwrap();
    v.systeme = SystemeVentilation::SimpleFluxInsufflation(VentilateurSimpleFlux {
        p_vent_base_w: Some(20.0),
        p_vent_pointe_w: Some(20.0),
        p_vent_occ_w: None,
        p_vent_inocc_w: None,
        rafraichissement_surventilation: None,
    });
    v.reseau.r_souf_m2k_w = Some(0.6);
    v.reseau.ratfuitevc = Some(1.0);
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);
    // Éq. (663) : air neuf réchauffé par le ventilateur ; pas de conduit hors volume chauffé.
    let (air, eps, _) = un_pas(&p, 5.0, 20.0, true);
    assert_eq!(eps, 0.0);
    let qm = rho_kg_m3(18.0) * 111.25 / 3600.0;
    proche(air.t_air_souffle_c, 5.0 + P_EL * 20.0 / (C_PA_J_KG_K * qm));
}

/// Zone de bureaux à simple flux par extraction, dérivée de la maison d'exemple : un groupe,
/// une bouche de reprise de 3 000 m³/h en occupation et 300 m³/h en inoccupation.
fn zone_de_bureaux(crdbnr: Option<f64>, q_inocc: f64) -> Projet {
    let mut p = projet();
    let z = zone_mut(&mut p);
    z.usage = UsageZone::Bureaux;
    z.groupes[0].bouches_conduits = vec![BoucheConduit {
        id: "extraction".into(),
        isouf: Isouf::Reprise,
        q_spec_conv_pointe_m3_h: 0.0,
        q_spec_conv_base_m3_h: None,
        qvarep_spec_m3_h: None,
        dugd_h: None,
        cdep: Some(1.15),
        a_cond_m2: None,
        q_spec_occ_m3_h: Some(3000.0),
        q_spec_inocc_m3_h: Some(q_inocc),
        q_spec_ch_m3_h: None,
        crdbnr,
        q_spec_rafnoc_m3_h: None,
    }];
    let v = z.ventilation.as_mut().unwrap();
    v.systeme = SystemeVentilation::SimpleFluxExtraction(VentilateurSimpleFlux {
        p_vent_base_w: None,
        p_vent_pointe_w: None,
        p_vent_occ_w: Some(900.0),
        p_vent_inocc_w: Some(90.0),
        rafraichissement_surventilation: None,
    });
    v.reseau.classe_etancheite = Some(ClasseEtancheite::B);
    v.reseau.ratfuitevc = None;
    p
}

/// Simple flux en non résidentiel : §6.2.3.1.1.1, §6.2.3.2.2.1, §6.2.3.3.1, §6.2.3.4 et
/// §6.5.3.3, valeurs calculées à la main.
#[test]
fn simple_flux_non_residentiel_a_la_main() {
    let p = zone_de_bureaux(Some(0.9), 300.0);
    let v = VentilationProjet::nouvelle(zone(&p), 18.0).unwrap();
    assert!(v.selon_occupation());
    // Occupation : éq. (383) q_max = −3 000 ; éq. (400) × Crdbnr 0,9 ; éq. (412) × Cdep 1,15 :
    // −3 105 m³/h. Inoccupation : éq. (385) et (402), Crdbnr = 1 : −300 × 1,15 = −345 m³/h.
    // Conduit par défaut, éq. (424) : 3 000 × 0,05 = 150 m² ; fuites, éq. (419), classe B,
    // 250 Pa (tableau 60) : 3 600 × 0,009e-3 × 150 × 250^0,667.
    let fuites = 3600.0 * 0.009e-3 * 150.0 * libm::pow(250.0, 0.667);
    // Éq. (428), Ratfuitevc = 0,75 par défaut en non résidentiel ; éq. (430).
    let occ = v.debits_groupe_selon(0, true);
    proche(occ.q_rep_m3_h, -(3105.0 + 0.75 * fuites));
    assert_eq!(occ.q_souffle_m3_h, 0.0);
    proche(
        v.debits_cond_groupe_selon(0, true).q_rep_m3_h,
        -(3105.0 + fuites),
    );
    let inocc = v.debits_groupe_selon(0, false);
    proche(inocc.q_rep_m3_h, -(345.0 + 0.75 * fuites));
    proche(
        v.debits_cond_groupe_selon(0, false).q_rep_m3_h,
        -(345.0 + fuites),
    );
    // Les mêmes, en chiffres : fuites de 193,2245 m³/h.
    assert!((occ.q_rep_m3_h + 3249.918396).abs() < 1e-6);
    assert!((inocc.q_rep_m3_h + 489.918396).abs() < 1e-6);
    // `debits_groupe` restitue l'occupation.
    assert_eq!(v.debits_groupe(0), occ);
    // Éq. (636) et (637) : puissance d'occupation ou d'inoccupation ; éq. (642) : un seul
    // groupe la reçoit entière.
    assert_eq!(v.p_vent_selon(true), (900.0, 0.0));
    assert_eq!(v.p_vent_selon(false), (90.0, 0.0));
    proche(v.p_vent_groupe_selon_w(0, true), 900.0);
    proche(v.p_vent_groupe_selon_w(0, false), 90.0);
    assert_eq!(v.p_vent_w(), 900.0);
    // Valeur par défaut signalée : Ratfuitevc (0,75) et surface de conduit.
    let champs: Vec<&str> = v
        .valeurs_par_defaut
        .iter()
        .map(|d| d.champ.as_str())
        .collect();
    assert!(champs.contains(&"ventilation.reseau.ratfuitevc"));
    assert!(champs.iter().any(|c| c.ends_with("a_cond_m2")));
    assert!(!champs.iter().any(|c| c.ends_with("crdbnr")));

    // Sans `crdbnr` : 1, aucune régulation (tableau 57), signalé.
    let p = zone_de_bureaux(None, 0.0);
    let v = VentilationProjet::nouvelle(zone(&p), 18.0).unwrap();
    proche(
        v.debits_groupe_selon(0, true).q_rep_m3_h,
        -(3450.0 + 0.75 * fuites),
    );
    assert!(v
        .valeurs_par_defaut
        .iter()
        .any(|d| d.champ.ends_with("crdbnr") && d.valeur == 1.0));
    // Ventilation arrêtée en inoccupation : débit nul, donc dP = 0 et fuites nulles (éq. 419).
    assert_eq!(v.debits_groupe_selon(0, false).q_rep_m3_h, 0.0);
    assert_eq!(v.debits_cond_groupe_selon(0, false).q_rep_m3_h, 0.0);
    // La puissance d'inoccupation (90 W) reste attribuée au groupe, au prorata des débits de
    // l'occupation, faute de débit en inoccupation (Q60) : la somme des groupes est celle du
    // système.
    proche(v.p_vent_groupe_selon_w(0, false), 90.0);

    // Réseau de l'exemple `bureaux-simple-flux-ademe`, sans classe d'étanchéité (2,5 fois la
    // classe A, signalé) : valeurs du relecteur du lot, calculées à la main. Les fuites sont
    // celles de 250 Pa dans les deux états (tableau 60) : 1 086,89 m³/h vus du groupe.
    let mut p = zone_de_bureaux(Some(0.9), 300.0);
    zone_mut(&mut p)
        .ventilation
        .as_mut()
        .unwrap()
        .reseau
        .classe_etancheite = None;
    let v = VentilationProjet::nouvelle(zone(&p), 18.0).unwrap();
    assert!((v.debits_groupe_selon(0, true).q_rep_m3_h + 4191.89).abs() < 5e-3);
    assert!((v.debits_groupe_selon(0, false).q_rep_m3_h + 1431.89).abs() < 5e-3);
}

/// Le pas de temps d'un simple flux par extraction suit l'indicateur de ventilation : débit
/// massique rejeté et échauffement par le ventilateur (éq. 652 à 654).
#[test]
fn simple_flux_non_residentiel_pas_de_temps() {
    let p = zone_de_bureaux(Some(1.0), 300.0);
    let mut v = VentilationProjet::nouvelle(zone(&p), 18.0).unwrap();
    let etat = [EtatGroupeVentilation {
        theta_i_fin_c: 20.0,
        saison_de_chauffage: true,
        ..EtatGroupeVentilation::default()
    }];
    let mut air = [AirSouffle::default()];
    let occ = v.pas(5.0, 0.004, &etat, &mut air, true);
    let inocc = v.pas(5.0, 0.004, &etat, &mut air, false);
    let fuites = 3600.0 * 0.009e-3 * 150.0 * libm::pow(250.0, 0.667);
    // Débit vu par le ventilateur, à la masse volumique de l'air du groupe (20 °C).
    let qm = |q_m3_h: f64| rho_kg_m3(20.0) * q_m3_h / 3600.0;
    proche(occ.qm_air_extrait_kg_s, qm(3450.0 + fuites));
    proche(inocc.qm_air_extrait_kg_s, qm(345.0 + fuites));
    // Le ventilateur échauffe moins l'air en occupation : 900 W pour un débit dix fois plus
    // fort qu'en inoccupation (90 W) — même ordre de grandeur, mais pas la même valeur.
    assert!(occ.theta_air_extrait_c > 5.0 && inocc.theta_air_extrait_c > 5.0);
    assert!((occ.theta_air_extrait_c - inocc.theta_air_extrait_c).abs() > 1e-6);
}

/// Maison en aération : une bouche de soufflage de 180 m³/h en pointe et 105 m³/h en base (le
/// débit d'hygiène à fournir par la fenêtre), sans ventilateur ni conduit.
fn aeration() -> Projet {
    let mut p = projet();
    let z = zone_mut(&mut p);
    z.groupes[0].bouches_conduits = vec![BoucheConduit {
        id: "fenetre".into(),
        isouf: Isouf::Soufflage,
        q_spec_conv_pointe_m3_h: 180.0,
        q_spec_conv_base_m3_h: Some(105.0),
        qvarep_spec_m3_h: None,
        dugd_h: None,
        cdep: None,
        a_cond_m2: None,
        q_spec_occ_m3_h: None,
        q_spec_inocc_m3_h: None,
        q_spec_ch_m3_h: None,
        crdbnr: None,
        q_spec_rafnoc_m3_h: None,
    }];
    z.groupes[0].entrees_air = None;
    let v = z.ventilation.as_mut().unwrap();
    v.systeme = SystemeVentilation::Aeration;
    v.reseau.r_rep_m2k_w = None;
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);
    p
}

#[test]
fn aeration_debits_a_la_main() {
    let p = aeration();
    let v = VentilationProjet::nouvelle(zone(&p), 18.0).unwrap();
    assert!(v.est_aeration());
    // Éq. (387) à (390) et (403) : (180 × 14 + 105 × 154)/168 = 111,25 m³/h ; §6.2.3.3.1,
    // note 1 : Cdep = Cfenb = 1,7, soit 189,125 m³/h ; tableau 59, aération : Kres = 0, aucune
    // fuite.
    let d = v.debits_groupe(0);
    proche(d.q_souffle_m3_h, 1.7 * 111.25);
    // Éq. (163) : le débit repris est l'opposé du débit soufflé, sans bouche de reprise.
    proche(d.q_rep_m3_h, -1.7 * 111.25);
    proche(v.debits_cond_groupe(0).q_souffle_m3_h, 1.7 * 111.25);
    // Aucun ventilateur.
    assert_eq!((v.p_vent_w(), v.p_vent_groupe_w(0)), (0.0, 0.0));
    // Seule la durée du grand débit est une valeur par défaut : ni Kres, ni Ratfuitevc, ni
    // surface de conduit.
    let champs: Vec<_> = v
        .valeurs_par_defaut
        .iter()
        .map(|d| d.champ.as_str())
        .collect();
    assert_eq!(champs, ["groupes[0].bouches_conduits[0].dugd_h"]);
}

#[test]
fn aeration_air_neuf_a_la_temperature_et_a_l_humidite_exterieures() {
    // §6.7.3, éq. (716) à (718) : Tair_soufflé = θext et ωair_soufflé = ωext.
    let p = aeration();
    let (air, eps, w_ag) = un_pas(&p, -3.0, 20.0, true);
    assert_eq!((air.t_air_souffle_c, eps, w_ag), (-3.0, 0.0, 0.0));
}

#[test]
fn aeration_validation() {
    // Cdep et la surface de conduit sont fixés par le texte (Crdbnr ne se saisit pas en
    // habitation).
    let mut p = aeration();
    zone_mut(&mut p).groupes[0].bouches_conduits[0].cdep = Some(1.0);
    assert_eq!(
        chemins(&p),
        ["batiments[0].zones[0].groupes[0].bouches_conduits[0].cdep"]
    );
    let mut p = aeration();
    zone_mut(&mut p).groupes[0].bouches_conduits[0].a_cond_m2 = Some(1.0);
    zone_mut(&mut p)
        .ventilation
        .as_mut()
        .unwrap()
        .reseau
        .ratfuitevc = Some(0.5);
    assert_eq!(
        chemins(&p),
        ["batiments[0].zones[0].groupes[0].bouches_conduits[0].a_cond_m2"]
    );
    // Sans bouche de soufflage : aucun débit d'air neuf.
    let mut p = aeration();
    zone_mut(&mut p).groupes[0].bouches_conduits[0].isouf = Isouf::Reprise;
    assert_eq!(chemins(&p), ["batiments[0].zones[0].ventilation.systeme"]);
    // Une bouche de reprise de plus n'est pas comptée : avertissement, pas d'erreur.
    let mut p = aeration();
    let mut reprise = zone(&p).groupes[0].bouches_conduits[0].clone();
    reprise.id = "reprise".into();
    reprise.isouf = Isouf::Reprise;
    zone_mut(&mut p).groupes[0].bouches_conduits.push(reprise);
    let r = valider(&p);
    assert!(r.est_valide(), "{:?}", r.erreurs);
    assert_eq!(
        r.avertissements
            .iter()
            .map(|a| a.chemin.as_str())
            .collect::<Vec<_>>(),
        ["batiments[0].zones[0].groupes[0].bouches_conduits[1].isouf"]
    );
    // Paramètres de réseau sans objet : avertissement.
    let mut p = aeration();
    zone_mut(&mut p)
        .ventilation
        .as_mut()
        .unwrap()
        .reseau
        .ratfuitevc = Some(0.5);
    let r = valider(&p);
    assert!(r.est_valide());
    assert_eq!(
        r.avertissements[0].chemin,
        "batiments[0].zones[0].ventilation.reseau"
    );
    // L'espace tampon de l'aération ne se saisit qu'avec ce système, et doit exister.
    let mut p = projet();
    zone_mut(&mut p).groupes[0].aeration_espace_tampon = Some("serre".into());
    assert!(chemins(&p).contains(&"batiments[0].zones[0].groupes[0].aeration_espace_tampon".into()));
    let mut p = aeration();
    zone_mut(&mut p).groupes[0].aeration_espace_tampon = Some("inconnu".into());
    assert_eq!(
        chemins(&p),
        ["batiments[0].zones[0].groupes[0].aeration_espace_tampon"]
    );
}

/// Puits climatique de 1 conduit de 30 m (di 0,2 m, ep 5 mm, λ 0,4 W/(m.K)) à 2 m, en sable
/// humide.
fn puits_climatique(bypass: Option<BypassPuits>) -> PuitsClimatique {
    PuitsClimatique {
        nd: 1,
        l_m: 30.0,
        di_m: 0.2,
        ep_m: 0.005,
        lambda_tube_w_m_k: 0.4,
        profondeur_m: 2.0,
        type_sol: TypeSolPuits::SableHumide,
        bypass,
    }
}

fn double_flux_avec_puits(bypass: Option<BypassPuits>) -> Projet {
    let mut p = double_flux(None, false);
    let v = zone_mut(&mut p).ventilation.as_mut().unwrap();
    v.puits = Some(Puits::Climatique);
    v.puits_climatique = Some(puits_climatique(bypass));
    p
}

#[test]
fn validation_du_puits_climatique() {
    let c = "batiments[0].zones[0].ventilation";
    // Double flux avec puits décrit : valide.
    let p = double_flux_avec_puits(None);
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);
    // Déclaré sans description, ou décrit sans déclaration.
    let mut p = double_flux_avec_puits(None);
    zone_mut(&mut p)
        .ventilation
        .as_mut()
        .unwrap()
        .puits_climatique = None;
    assert_eq!(chemins(&p), [format!("{c}.puits_climatique")]);
    let mut p = double_flux_avec_puits(None);
    zone_mut(&mut p).ventilation.as_mut().unwrap().puits = None;
    assert_eq!(chemins(&p), [format!("{c}.puits")]);
    // Plages du tableau 69.
    let mut p = double_flux_avec_puits(None);
    let d = zone_mut(&mut p)
        .ventilation
        .as_mut()
        .unwrap()
        .puits_climatique
        .as_mut()
        .unwrap();
    d.nd = 0;
    d.l_m = 0.0;
    d.profondeur_m = -1.0;
    assert_eq!(
        chemins(&p),
        [
            format!("{c}.puits_climatique.nd"),
            format!("{c}.puits_climatique.l_m"),
            format!("{c}.puits_climatique.profondeur_m"),
        ]
    );
    // Un simple flux par extraction n'a pas d'air neuf soufflé : refusé (§6.11.3) ; le simple
    // flux par insufflation est accepté.
    let mut p = projet();
    let v = zone_mut(&mut p).ventilation.as_mut().unwrap();
    v.puits = Some(Puits::Climatique);
    v.puits_climatique = Some(puits_climatique(None));
    assert_eq!(chemins(&p), [format!("{c}.puits")]);
    // Puits hydraulique avec un puits climatique décrit : exception de l'éq. (386).
    let mut p = double_flux_avec_puits(None);
    zone_mut(&mut p).ventilation.as_mut().unwrap().puits = Some(Puits::Hydraulique);
    let r = valider(&p);
    assert_eq!(r.erreurs.len(), 2, "{r:?}");
    assert!(r.erreurs[0].hors_perimetre);
    assert!(r.erreurs[1].message.contains("386"));
}

/// État d'un groupe à 20 °C en saison de chauffage.
fn groupe_en_chauffage(theta_i: f64) -> [EtatGroupeVentilation; 1] {
    [EtatGroupeVentilation {
        theta_i_fin_c: theta_i,
        omega_i_fin_kg_kg: 0.008,
        saison_de_chauffage: true,
        ..EtatGroupeVentilation::default()
    }]
}

#[test]
fn double_flux_avec_puits_a_la_main() {
    let p = double_flux_avec_puits(None);
    let mut v = VentilationProjet::nouvelle(zone(&p), 18.0).unwrap();
    // Température extérieure de 10 °C toute l'année : ΔθA = 0, θsol = gm × θAM = 0,98 × 10 =
    // 9,8 °C (éq. 721, sable humide).
    let te = vec![10.0; 8760];
    let etat = groupe_en_chauffage(20.0);
    let mut air = [AirSouffle::default()];
    // Premier pas, air neuf à 0 °C : aucun débit connu, le débit du pas (100 m³/h à 18 °C,
    // 0,033562 kg/s) sert à l'efficacité : ε = 0,94585 (éq. 723 à 727, calculé à la main :
    // v0 = 1,0654 m/s, hi = 6,6475 W/(m².K), Ug = 3,6124 W/(m.K), NTU = 2,9166).
    v.preparer_puits(0, &te, 0.0, &etat);
    let s = v.pas(0.0, 0.004, &etat, &mut air, true);
    let qm0 = rho_kg_m3(18.0) * 100.0 / 3600.0;
    let theta_an0 = 0.945_850_645 * 9.8;
    assert!(
        (s.theta_an_c.unwrap() - theta_an0).abs() < 1e-4,
        "{:?}",
        s.theta_an_c
    );
    assert!(s.puits_actif);
    // Énergie échangée (éq. 728) : Cpa × qm × (θan − θext).
    proche(s.q_puits_wh, C_PA_J_KG_K * qm0 * s.theta_an_c.unwrap());
    // Échangeur à 0,8 (éq. 512, 519) avec l'air neuf sorti du puits (éq. 414).
    let qm_rep = 1.2 * 100.0 / 3600.0;
    let t_extr2 = 20.0 + P_EL * 20.0 / (C_PA_J_KG_K * qm_rep);
    let t_souf1 = s.theta_an_c.unwrap() + 0.8 * (t_extr2 - s.theta_an_c.unwrap());
    proche(
        air[0].t_air_souffle_c,
        t_souf1 + P_EL * 20.0 / (C_PA_J_KG_K * qm0),
    );
    // Pas suivant : θan avec le débit d'air neuf de la fin du pas précédent (éq. 414), pas
    // celui du pas en cours (0,033481 kg/s, qui donnerait 9,27046).
    v.preparer_puits(1, &te, 0.0, &etat);
    let s = v.pas(0.0, 0.004, &etat, &mut air, true);
    assert!((s.theta_an_c.unwrap() - theta_an0).abs() < 1e-4);
    assert!((s.theta_an_c.unwrap() - 9.270_457_8).abs() > 5e-4);
}

#[test]
fn puits_contourne_par_le_bypass() {
    // Hiver : θext 10 < θi 23, θext > 8 et θi > 22 : puits contourné (éq. 751), l'air neuf est
    // à la température extérieure (éq. 731).
    let by = BypassPuits {
        theta_int_hiver_c: 22.0,
        theta_ext_hiver_c: 8.0,
        theta_int_ete_c: 24.0,
        theta_ext_ete_c: 12.0,
    };
    let p = double_flux_avec_puits(Some(by));
    let mut v = VentilationProjet::nouvelle(zone(&p), 18.0).unwrap();
    let te = vec![10.0; 8760];
    let etat = groupe_en_chauffage(23.0);
    let mut air = [AirSouffle::default()];
    v.preparer_puits(0, &te, 10.0, &etat);
    let s = v.pas(10.0, 0.004, &etat, &mut air, true);
    assert_eq!(s.theta_an_c, Some(10.0));
    assert!(!s.puits_actif);
    assert_eq!(s.q_puits_wh, 0.0);
    // Intérieur à 21 °C, sous la consigne de 22 °C : le puits est traversé.
    let etat = groupe_en_chauffage(21.0);
    v.preparer_puits(1, &te, 10.0, &etat);
    let s = v.pas(10.0, 0.004, &etat, &mut air, true);
    assert!(s.puits_actif);
}

#[test]
fn sans_puits_l_air_neuf_est_a_la_temperature_exterieure() {
    let p = double_flux(None, false);
    let mut v = VentilationProjet::nouvelle(zone(&p), 18.0).unwrap();
    let te = vec![10.0; 8760];
    let etat = groupe_en_chauffage(20.0);
    let mut air = [AirSouffle::default()];
    v.preparer_puits(0, &te, 3.0, &etat);
    let s = v.pas(3.0, 0.004, &etat, &mut air, true);
    assert_eq!(s.theta_an_c, None);
    assert!(!s.puits_actif);
}
