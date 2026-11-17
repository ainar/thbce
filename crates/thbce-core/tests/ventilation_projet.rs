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
    BoucheConduit, Bypass, ClasseEtancheite, DoubleFlux, Echangeur, EchangeurSimplifie, Isouf,
    Projet, RepresentationEchangeur, StatutEchangeur, SystemeVentilation, UsageZone,
    VentilateurSimpleFlux, Zone,
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
        crdbnr: None,
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
        p_vent_base_rep_w: 20.0,
        p_vent_pointe_rep_w: 20.0,
        p_vent_base_sou_w: 20.0,
        p_vent_pointe_sou_w: 20.0,
        echangeur: Some(Echangeur {
            representation: RepresentationEchangeur::Simplifiee(EchangeurSimplifie {
                epsilon_saisi: 0.8,
                statut: Some(StatutEchangeur::Certifie),
            }),
            rotatif: None,
            bypass,
            antigel: Some(antigel),
            generation_antigel: antigel.then(|| "generation-effet-joule".into()),
            theta_s_ech_rep_lim_c: None,
        }),
        prechauffage: None,
        humidification: None,
        rafraichissement_evaporatif: None,
        surventilation_mecanique: None,
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
            saison_de_chauffage: chauffage,
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
        SystemeVentilation::Aeration,
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
    let mut p = projet();
    zone_mut(&mut p).ventilation.as_mut().unwrap().puits =
        Some(thbce_core::entree::Puits::Climatique);
    assert_eq!(chemins(&p), ["batiments[0].zones[0].ventilation.puits"]);
    // Sens des bouches : un double flux souffle et extrait.
    let mut p = double_flux(None, false);
    zone_mut(&mut p).groupes[0].bouches_conduits.pop();
    assert_eq!(chemins(&p), ["batiments[0].zones[0].ventilation.systeme"]);
    // Fonctions hors périmètre du double flux.
    let mut p = double_flux(None, false);
    if let SystemeVentilation::DoubleFlux(d) =
        &mut zone_mut(&mut p).ventilation.as_mut().unwrap().systeme
    {
        d.surventilation_mecanique = Some(true);
        d.echangeur.as_mut().unwrap().rotatif = Some(true);
    }
    assert_eq!(
        chemins(&p),
        [
            "batiments[0].zones[0].ventilation.systeme.double_flux.surventilation_mecanique",
            "batiments[0].zones[0].ventilation.systeme.double_flux.echangeur.rotatif",
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
