// SPDX-License-Identifier: GPL-3.0-or-later
//! Calcul annuel du confort d'été (mode Th-D, indicateur DH) de bout en bout sur la maison
//! d'exemple (`tests/cas/maison-climat-ademe/entree.json`, Indre-et-Loire, zone H2b).
//!
//! Comme pour le Bbio (`tests/bbio.rs`), le scénario est joué sur la météo ouverte ADEME du
//! dépôt (résultat non réglementaire, CI publique) et sur la météo réglementaire, qui ne peut
//! pas être versionnée (tests ignorés par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test confort_ete -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les tests vérifient des propriétés physiques, pas
//! des valeurs.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::bati::assemblage_batiment::BatimentHoraire;
use thbce_core::bati::assemblage_groupe::{Lectures, VentilationThD};
use thbce_core::bati::assemblage_zone::ZoneHoraire;
use thbce_core::bati::surventilation::LectureHysteresis;
use thbce_core::climat::{AnneeMeteo, ClimatExterieur, ConfortAdaptatif, MeteoProjet};
use thbce_core::conventions::ModeCalcul;
use thbce_core::entree::{
    InertieAnnuelle, InertieQuotidienne, InertieSequentielle, Projet, TypeOuvrant,
};
use thbce_core::resultats::{Resultats, ResultatsConfortEte};
use thbce_core::scenarios::saisons::{LectureSaisonFroidThD, Saison};
use thbce_core::{calculer, series_horaires, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn maison() -> Projet {
    let chemin = racine().join("tests/cas/maison-climat-ademe/entree.json");
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

fn confort_ete(r: &Resultats) -> &ResultatsConfortEte {
    r.batiments[0].zones[0].groupes[0]
        .confort_ete
        .as_ref()
        .unwrap()
}

/// Même maison, sans protection mobile sur les baies.
fn sans_protections(projet: &Projet) -> Projet {
    let mut p = projet.clone();
    for baie in &mut p.catalogue.baies {
        baie.protection_mobile = None;
        baie.avec_protection = None;
        baie.u_ap_vert_w_m2k = None;
        baie.u_ap_horiz_w_m2k = None;
    }
    p
}

/// Même maison, baies trois fois plus grandes (les parois opaques sont inchangées).
fn baies_triplees(projet: &Projet) -> Projet {
    let mut p = projet.clone();
    for baie in &mut p.batiments[0].zones[0].groupes[0].baies {
        baie.a_b_m2 *= 3.0;
        if let Some(l) = &mut baie.exposition.lpb_m {
            *l *= 3.0;
        }
    }
    p
}

/// Même maison, baies qui ne s'ouvrent pas : pas de surventilation naturelle.
fn sans_ouvrants(projet: &Projet) -> Projet {
    let mut p = projet.clone();
    for baie in &mut p.catalogue.baies {
        baie.ouvrant = None;
    }
    p
}

fn afficher(titre: &str, r: &Resultats) {
    let c = confort_ete(r);
    let g = &r.batiments[0].zones[0].groupes[0];
    println!(
        "{titre} : DH = {:.1} °C.h ; {} h d'inconfort (dont {} h à +1 °C et {} h à +2 °C) sur \
         {} h occupées de la période de confort adaptatif ; θop max = {:.2} °C (année), \
         {:.2} °C (en occupation, dans la période)",
        c.dh_c_h,
        c.nbh_inconf,
        c.nbh_inconf_1c,
        c.nbh_inconf_2c,
        c.nbh_occ_conf_adapt,
        c.theta_op_max_c,
        c.theta_op_max_occ_c.unwrap_or(f64::NAN),
    );
    println!(
        "  période de confort adaptatif (Th-D) : {:?} ; saisons Th-D : {:?}",
        r.climat.confort_adaptatif_th_d,
        g.saisons_th_d.as_ref().unwrap()
    );
}

/// Propriétés attendues du confort d'été de la maison d'exemple, quelle que soit la météo.
fn verifier_le_scenario(projet: &Projet, meteo: &MeteoProjet, titre: &str) -> Resultats {
    let r = brut(projet, meteo);
    afficher(titre, &r);
    let c = confort_ete(&r);
    // Le DH est une somme de dépassements : positif ou nul, et borné par le nombre d'heures.
    assert!(c.dh_c_h >= 0.0);
    assert!(c.nbh_inconf >= c.nbh_inconf_1c && c.nbh_inconf_1c >= c.nbh_inconf_2c);
    assert!(c.nbh_inconf <= c.nbh_occ_conf_adapt && c.nbh_occ_conf_adapt > 0);
    // Chaque heure comptée à +1 °C pèse au moins 1 °C.h, chaque heure à +2 °C au moins 2.
    assert!(c.dh_c_h >= f64::from(c.nbh_inconf_1c) + f64::from(c.nbh_inconf_2c));
    // Températures plausibles pour un logement non refroidi en période caniculaire.
    assert!((24.0..45.0).contains(&c.theta_op_max_c), "{c:?}");
    assert!(c.theta_op_max_occ_c.unwrap() <= c.theta_op_max_c);
    // Le DH du bâtiment et de la zone est celui du groupe le plus défavorable : ici le seul.
    assert_eq!(r.batiments[0].dh_c_h, Some(c.dh_c_h));
    assert_eq!(r.batiments[0].zones[0].dh_c_h, Some(c.dh_c_h));
    // Le Bbio n'est pas affecté par le second calcul annuel.
    assert!(r.batiments[0].bbio_pts.unwrap() > 0.0);
    // Depuis le lot 6b, le DH est calculé avec les systèmes du projet : plus d'avertissement.
    assert!(!r.avertissements.iter().any(|a| a.starts_with("DH calculé")));
    assert!(!r
        .avertissements
        .iter()
        .any(|a| a.contains("DH") && a.contains("pas encore calculés")));

    // Sans protection mobile, plus d'apports solaires : DH plus élevé. Avec (la maison
    // d'exemple a des volets motorisés), DH plus bas.
    let nu = brut(&sans_protections(projet), meteo);
    afficher("  sans protection mobile", &nu);
    assert!(confort_ete(&nu).dh_c_h > c.dh_c_h);
    assert!(confort_ete(&nu).theta_op_max_c > c.theta_op_max_c);
    // Des baies trois fois plus grandes : DH plus élevé encore sans protection.
    let vitre = brut(&baies_triplees(projet), meteo);
    afficher("  baies triplées, avec protections", &vitre);
    let vitre_nu = brut(&sans_protections(&baies_triplees(projet)), meteo);
    afficher("  baies triplées, sans protection", &vitre_nu);
    assert!(confort_ete(&vitre_nu).dh_c_h > confort_ete(&vitre).dh_c_h);
    assert!(confort_ete(&vitre_nu).dh_c_h > confort_ete(&nu).dh_c_h);
    // Sans ouverture des baies, pas de surventilation naturelle : DH plus élevé.
    let ferme = brut(&sans_ouvrants(projet), meteo);
    afficher("  baies non ouvrantes", &ferme);
    assert!(confort_ete(&ferme).dh_c_h > c.dh_c_h);
    r
}

/// Enchaînement horaire du calcul Th-D : bilan d'énergie, règles du mode, série horaire.
fn verifier_l_enchainement(projet: &Projet, meteo: &MeteoProjet) {
    let series = series_horaires(projet, meteo, ModeCalcul::ThD).unwrap();
    assert_eq!(series.len(), 1);
    let serie = &series[0];
    assert_eq!((serie.pas.len(), serie.te_c.len()), (8760, 8760));
    let loc = &projet.localisation;
    let climat = ClimatExterieur::nouveau(&meteo.th_d, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    // Le bâtiment entier : les pertes récupérables des systèmes y sont calculées (lot 6b).
    let mut bat = BatimentHoraire::nouveau(
        &projet.batiments[0],
        &projet.catalogue,
        ModeCalcul::ThD,
        loc.alt_m,
        climat.we_kg_kg[0],
    )
    .unwrap();
    let mut dh = 0.0;
    let mut heures_ete = 0_u32;
    bat.simuler_avec(&climat, &confort, |h, b| {
        let s = b.zones[0].sorties[0].unwrap();
        // La série horaire de l'API est celle du calcul.
        assert_eq!(serie.pas[h], s);
        // Aucun refroidissement en Th-D (§13.5.3.1).
        assert_eq!(s.q_sys_fr_wh, 0.0);
        let p = s.confort_ete.unwrap();
        assert_eq!(p.is_conf_adapt, confort.is_conf_adapt(h));
        assert_eq!(p.theta_op_c, s.fin.theta_op_c);
        // Température d'inconfort chaud entre la consigne (26 °C) et la consigne + 2 °C.
        assert!((26.0..=28.0).contains(&p.theta_op_conf_ch_corr_c));
        if p.is_conf_adapt {
            // Pas de chauffage en période de confort adaptatif (éq. 76, 77 et 80).
            assert_eq!(s.q_sys_ch_wh, 0.0, "pas {h}");
        } else {
            assert_eq!(p.dh_c_h, 0.0);
        }
        dh += p.dh_c_h;
        heures_ete += u32::from(s.saison == Saison::Refroidissement);
    })
    .unwrap();
    let g = &bat.zones[0].groupes[0];
    // Le DH est la somme des contributions horaires (éq. 2551).
    assert_eq!(dh, g.cumuls.confort_ete.dh_c_h);
    let par_calculer = brut(projet, meteo);
    assert_eq!(confort_ete(&par_calculer).dh_c_h, dh);
    // La période de confort adaptatif met le groupe en saison de refroidissement au sens des
    // occupants (éq. 88, lecture Q40), sans qu'aucun système ne refroidisse.
    assert!(heures_ete > 24 * 30, "{heures_ete} h");
    assert!(g.cumuls.nbh_surventilation > 0);
    assert_eq!(g.cumuls.q_sys_fr_wh, [0.0; 12]);
    // Bilan d'énergie fermé sur l'année, à 1e-9 des flux entrants (même tolérance et même
    // justification que pour le calcul Th-B, `tests/bbio.rs`).
    let b = &g.cumuls.bilan;
    println!(
        "  bilan Th-D (kWh) : solaire des baies {:.0}, parois opaques {:.0}, apports internes \
         {:.0}, chauffage {:.0} ; pertes des baies {:.0}, des parois opaques {:.0}, par l'air \
         {:.0}, stockage {:.1}, inerties longues {:.1} ; résidu {:.2e} Wh ; baies ouvertes {} h",
        b.apports_solaires_baies_wh / 1e3,
        b.apports_opaques_wh / 1e3,
        b.apports_internes_wh / 1e3,
        b.chauffage_wh / 1e3,
        b.pertes_baies_wh / 1e3,
        b.pertes_opaques_wh / 1e3,
        b.pertes_air_wh / 1e3,
        b.stockage_wh / 1e3,
        b.stockage_inerties_longues_wh / 1e3,
        b.residu_wh(),
        g.cumuls.nbh_surventilation
    );
    assert!(b.entrees_wh() > 1.0e6);
    assert_eq!(b.refroidissement_wh, 0.0);
    assert!(b.residu_wh().abs() <= 1e-9 * b.entrees_wh(), "{b:?}");
}

/// Maison d'exemple avec une inertie séquentielle « lourde » et une inertie annuelle saisie,
/// dont les surfaces d'échange sont inférieures à celle de l'inertie quotidienne : les deux
/// masses de longue période échangent alors avec la masse quotidienne (§5.21.3.4).
fn avec_inerties_longues(projet: &Projet) -> Projet {
    let mut p = projet.clone();
    let i = &mut p.batiments[0].zones[0].groupes[0].inertie;
    i.quotidienne = InertieQuotidienne::Moyenne;
    i.sequentielle = InertieSequentielle::Lourde;
    i.ams_surf_m2_m2 = 1.5;
    i.annuelle = InertieAnnuelle::Personnalisee {
        cma_surf_kj_k_m2: 2000.0,
    };
    i.ama_surf_m2_m2 = 1.0;
    p
}

/// Les inerties séquentielle et annuelle ferment le bilan d'énergie et lissent la température.
fn verifier_les_inerties_longues(projet: &Projet, meteo: &MeteoProjet) {
    let lourd = avec_inerties_longues(projet);
    let loc = &lourd.localisation;
    let climat = ClimatExterieur::nouveau(&meteo.th_d, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    let simuler = |p: &Projet, lectures: Lectures| {
        let mut zone = ZoneHoraire::nouvelle_selon(
            &p.batiments[0].zones[0],
            &p.catalogue,
            ModeCalcul::ThD,
            lectures,
            loc.alt_m,
            climat.we_kg_kg[0],
        )
        .unwrap();
        zone.simuler(&climat, &confort).unwrap();
        zone.groupes.remove(0)
    };
    let avec = simuler(&lourd, Lectures::default());
    let sans = simuler(
        &lourd,
        Lectures {
            inerties_longues: false,
            ..Lectures::default()
        },
    );
    let defaut = simuler(projet, Lectures::default());
    let b = &avec.cumuls.bilan;
    println!(
        "  inerties longues : DH {:.1} °C.h avec, {:.1} sans ; θop max {:.2} et {:.2} °C ; \
         stockage {:.1} kWh ; θms finale {:.2} °C, θma finale {:.2} °C ; résidu {:.2e} Wh",
        avec.cumuls.confort_ete.dh_c_h,
        sans.cumuls.confort_ete.dh_c_h,
        avec.cumuls.confort_ete.theta_op_max_c,
        sans.cumuls.confort_ete.theta_op_max_c,
        b.stockage_inerties_longues_wh / 1e3,
        avec.etat.inerties.theta_ms_c,
        avec.etat.inerties.theta_ma_c,
        b.residu_wh()
    );
    // Les masses de longue période ont échangé de la chaleur, et le bilan reste fermé.
    assert!(b.stockage_inerties_longues_wh != 0.0);
    assert!(b.residu_wh().abs() <= 1e-9 * b.entrees_wh(), "{b:?}");
    assert_eq!(sans.cumuls.bilan.stockage_inerties_longues_wh, 0.0);
    // Elles écrêtent la pointe de température de la séquence caniculaire.
    assert!(avec.cumuls.confort_ete.theta_op_max_c < sans.cumuls.confort_ete.theta_op_max_c);
    // Températures de masse plausibles en fin d'année.
    assert!((10.0..35.0).contains(&avec.etat.inerties.theta_ms_c));
    assert!((10.0..35.0).contains(&avec.etat.inerties.theta_ma_c));
    // Classes « par défaut » (Cms = Cmq, Cma = Cms) : aucune capacité supplémentaire, aucun
    // flux (tableau 54 et éq. 316).
    assert!(defaut.inerties.sans_effet());
    assert_eq!(defaut.cumuls.bilan.stockage_inerties_longues_wh, 0.0);
}

#[test]
fn dh_de_la_maison_d_exemple_sur_la_meteo_ademe() {
    let p = maison();
    let m = meteo_ademe(&p);
    let r = verifier_le_scenario(&p, &m, "météo ADEME (+2,7 °C, vague de chaleur)");
    // Météo ouverte : le résultat est marqué non réglementaire.
    assert!(!r.reglementaire);
    verifier_l_enchainement(&p, &m);
    verifier_les_inerties_longues(&p, &m);
    // Restitution : arrondi à une décimale (décision Q8).
    let arrondi = calculer(&p, &m, &Options::default()).unwrap();
    let attendu = (confort_ete(&r).dh_c_h * 10.0).round() / 10.0;
    assert_eq!(arrondi.batiments[0].dh_c_h, Some(attendu));
    assert_eq!(confort_ete(&arrondi).dh_c_h, attendu);
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn dh_de_la_maison_d_exemple_sur_la_meteo_reglementaire() {
    let p = maison();
    let m = meteo_reglementaire(&p);
    let r = verifier_le_scenario(&p, &m, "météo réglementaire RE2020, H2b, jeu Th-D");
    assert!(r.reglementaire);
    verifier_l_enchainement(&p, &m);
    verifier_les_inerties_longues(&p, &m);
}

/// Décale d'une ligne la lecture d'un jeu météo : la ligne `h + 1` alimente l'itération `h`
/// (variante de la décision Q25).
fn decaler_d_une_ligne(a: &AnneeMeteo) -> AnneeMeteo {
    let mut b = a.clone();
    for v in [
        &mut b.te0_c,
        &mut b.we0_g_kg,
        &mut b.idn_w_m2,
        &mut b.idi_w_m2,
        &mut b.teciel_c,
        &mut b.vent_m_s,
        &mut b.teau0_c,
        &mut b.gamma_rad,
        &mut b.psi_rad,
    ] {
        v.rotate_left(1);
    }
    b
}

/// DH de la maison d'exemple en Th-D pour un jeu de lectures.
fn dh_selon(projet: &Projet, annee: &AnneeMeteo, lectures: Lectures) -> (f64, f64) {
    let loc = &projet.localisation;
    let climat = ClimatExterieur::nouveau(annee, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    let mut zone = ZoneHoraire::nouvelle_selon(
        &projet.batiments[0].zones[0],
        &projet.catalogue,
        ModeCalcul::ThD,
        lectures,
        loc.alt_m,
        climat.we_kg_kg[0],
    )
    .unwrap();
    zone.simuler(&climat, &confort).unwrap();
    let c = &zone.groupes[0].cumuls.confort_ete;
    (c.dh_c_h, c.theta_op_max_c)
}

/// Sensibilité du DH aux lectures du texte qui peuvent être basculées sans modifier le code :
/// hystérésis de l'ouverture des baies (Q32), lecture du fichier météo (Q25), saison de
/// refroidissement (Q40) et ventilation du calcul Th-D (Q40 et Q42). Mesure, sans assertion de
/// valeur.
fn mesurer_les_sensibilites(projet: &Projet, meteo: &MeteoProjet) {
    let d = Lectures::default();
    let variantes: [(&str, Projet, AnneeMeteo, Lectures); 7] = [
        ("lectures retenues", projet.clone(), meteo.th_d.clone(), d),
        (
            "Q32 point 1, hystérésis en bande à mémoire",
            projet.clone(),
            meteo.th_d.clone(),
            Lectures {
                hysteresis: LectureHysteresis::BandeAMemoire,
                ..d
            },
        ),
        (
            "Q25, fichier météo décalé d'une ligne",
            projet.clone(),
            decaler_d_une_ligne(&meteo.th_d),
            d,
        ),
        (
            "Q40, Autfr,pro nulle sans système (pas de saison d'été)",
            projet.clone(),
            meteo.th_d.clone(),
            Lectures {
                saison_froid_th_d: LectureSaisonFroidThD::NulleSansSysteme,
                ..d
            },
        ),
        (
            "Q40 point 1 et Q42 point 1, assemblage provisoire du lot 5 (ventilation \
             conventionnelle du Bbio)",
            projet.clone(),
            meteo.th_d.clone(),
            Lectures {
                ventilation_th_d: VentilationThD::ConventionnelleBbio,
                ..d
            },
        ),
        (
            "inerties longues (séquentielle lourde, annuelle saisie)",
            avec_inerties_longues(projet),
            meteo.th_d.clone(),
            d,
        ),
        (
            "ouvrants à soufflet d'oscillo-battant (Rouvmax = 20 %)",
            {
                let mut p = projet.clone();
                for baie in &mut p.catalogue.baies {
                    if let Some(o) = &mut baie.ouvrant {
                        o.type_ouvrant = Some(TypeOuvrant::SouffletOscilloBattant);
                    }
                }
                p
            },
            meteo.th_d.clone(),
            d,
        ),
    ];
    for cas in [projet.clone(), baies_triplees(projet)] {
        let triple = cas != *projet;
        println!(
            "sensibilités du DH, {} :",
            if triple {
                "baies triplées"
            } else {
                "maison d'exemple"
            }
        );
        for (nom, p, annee, lectures) in &variantes {
            let p = if triple { baies_triplees(p) } else { p.clone() };
            let (dh, theta_max) = dh_selon(&p, annee, *lectures);
            println!("  {nom} : DH = {dh:.1} °C.h ; θop max = {theta_max:.2} °C");
            assert!(dh >= 0.0);
        }
    }
}

#[test]
#[ignore = "mesure de sensibilité, à lancer avec --nocapture"]
fn sensibilites_du_dh_sur_la_meteo_ademe() {
    let p = maison();
    mesurer_les_sensibilites(&p, &meteo_ademe(&p));
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn sensibilites_du_dh_sur_la_meteo_reglementaire() {
    let p = maison();
    mesurer_les_sensibilites(&p, &meteo_reglementaire(&p));
}

/// Temps d'un calcul complet de la maison d'exemple : validation, climat, deux années au pas
/// horaire (Th-B puis Th-D), restitution. À lancer en mode optimisé :
///
/// ```text
/// cargo test --release -p thbce-io --test confort_ete -- --ignored --nocapture temps_de_calcul
/// ```
#[test]
#[ignore = "mesure de performance, à lancer avec --release --nocapture"]
fn temps_de_calcul_th_b_et_th_d() {
    let p = maison();
    let m = meteo_ademe(&p);
    let n = 10;
    let debut = std::time::Instant::now();
    for _ in 0..n {
        std::hint::black_box(calculer(&p, &m, &Options::default()).unwrap());
    }
    let complet = debut.elapsed() / n;
    let debut = std::time::Instant::now();
    for _ in 0..n {
        std::hint::black_box(series_horaires(&p, &m, ModeCalcul::ThD).unwrap());
    }
    let th_d = debut.elapsed() / n;
    println!(
        "maison d'exemple : calcul complet Th-B + Th-D {complet:?} ; calcul Th-D seul avec \
         séries horaires {th_d:?} (moyennes de {n} calculs)"
    );
}
