// SPDX-License-Identifier: GPL-3.0-or-later
//! Derniers usages ouverts au lot 0c4 : commerces (17), vestiaires seuls (18), aérogares (22),
//! industries ou artisanats (23 et 24) et établissements sportifs (25 et 28). Exemples
//! `tests/cas/commerce-pac-ademe` (usage 17, grand volume éclairé par des éclairants de toiture
//! uniformément répartis et des baies verticales) et `tests/cas/gymnase-pac-ademe` (usage 25,
//! salle de sport éclairée par des baies verticales) sur la météo ouverte ADEME (leurs valeurs
//! sont vérifiées par leur `attendu.json`) ; calculs à la main ; sens des effets ; autres usages
//! par substitution des locaux du tableau 78 ; refus de saisie des grands volumes.
//!
//! Valeurs phares sur la météo réglementaire :
//! THBCE_METEO=/chemin/vers/meteo.csv cargo t-regl commerces_sports_sur_meteo_reglementaire

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{AffectationEclairage, Projet, UsageZone};
use thbce_core::resultats::Resultats;
use thbce_core::scenarios::scenarios_conventionnels::ScenariosZone;
use thbce_core::{calculer, valider, Options};
use thbce_data::eclairage::TypeLocal;
use thbce_data::protections_mobiles::{famille_et_parts, FamilleGpm};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn exemple(nom: &str) -> Projet {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn commerce() -> Projet {
    exemple("commerce-pac-ademe")
}

fn gymnase() -> Projet {
    exemple("gymnase-pac-ademe")
}

fn meteo_ademe(p: &Projet) -> MeteoProjet {
    let zone = p.localisation.zone_climatique_retenue();
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

fn calcul(p: &Projet) -> Resultats {
    calculer(p, &meteo_ademe(p), &Options { arrondir: false }).unwrap()
}

/// Indicateurs d'un calcul : Bbio, DH, Cep, Cep,nr, Ic énergie.
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

fn elec(r: &Resultats) -> &thbce_core::indicateurs::resultats::PostesElectricite {
    &r.batiments[0]
        .indicateurs
        .as_ref()
        .unwrap()
        .electricite_consommee
}

fn eclairage_kwh(r: &Resultats) -> f64 {
    elec(r).eclairage_kwh
}

/// Surfaces, débits conventionnels du Bbio (tableau 56-1), forfait de refroidissement
/// (éq. 2493, tableau 335) et familles des protections mobiles (figure 34), recalculés à la main.
#[test]
fn commerce_et_gymnase_a_la_main() {
    let (pc, pg) = (commerce(), gymnase());
    for p in [&pc, &pg] {
        let v = valider(p);
        assert!(v.est_valide(), "{:?}", v.erreurs);
        assert!(v.avertissements.is_empty(), "{:?}", v.avertissements);
    }
    let (rc, rg) = (calcul(&pc), calcul(&pg));
    for r in [&rc, &rg] {
        let [bbio, dh, cep, cep_nr, ic] = phares(r);
        println!(
            "ADEME : Bbio {bbio:.1} ; DH {dh:.1} ; Cep {cep:.1} ; Cep,nr {cep_nr:.1} ; Ic {ic:.1}"
        );
        assert!((cep - cep_nr).abs() < 1e-9);
    }
    let (bc, bg) = (&rc.batiments[0], &rg.batiments[0]);
    // §13.1.3, éq. (2376) : SREF = SU hors habitation.
    assert_eq!((bc.sref_m2, bg.sref_m2), (800.0, 900.0));

    // Tableau 56-1 : commerces 3,7 m³/(h.m²) ; établissements sportifs 3,0. En occupation :
    // 3,7 × 800 = 2 960 et 3,0 × 900 = 2 700 m³/h.
    for (b, q) in [(bc, 2960.0), (bg, 2700.0)] {
        let d = b.zones[0].groupes[0].debits_bbio.as_ref().unwrap();
        assert_eq!((d.q_spec_repris_m3_h, d.q_spec_souffle_m3_h), (-q, q));
    }

    // Éq. (2493) : Cef = 0,009 × (min(DH ; seuil haut) − 350) × Coef_zone_clim_et_alt / 2,3
    // par m² de SREF ; coefficient 1,0 en H2b à 80 m et en H1b à 210 m (tableau 336). Seuil
    // haut : DH_max, 3 300 °C.h pour les commerces et 2 000 pour le gymnase (exigences).
    let dh_c = bc.dh_c_h.unwrap();
    assert!(dh_c > 350.0 && dh_c < 3300.0);
    let attendu = 0.009 * (dh_c - 350.0) / 2.3 * 800.0;
    assert!(
        (elec(&rc).refroidissement_kwh - attendu).abs() < 1e-3,
        "{} {attendu}",
        elec(&rc).refroidissement_kwh
    );
    let dh_g = bg.dh_c_h.unwrap();
    assert!(dh_g > 2000.0);
    let attendu = 0.009 * (2000.0 - 350.0) / 2.3 * 900.0;
    assert!(
        (elec(&rg).refroidissement_kwh - attendu).abs() < 1e-3,
        "{}",
        elec(&rg).refroidissement_kwh
    );

    // Figure 34 : famille « autres », Pocc = 0,7 et Pderog = 0,1 (17, 23, 24) ou 0,9 et 0
    // (18, 22, 25, 28).
    for u in [17, 23, 24] {
        let (f, p) = famille_et_parts(u).unwrap();
        assert_eq!((f, p.p_occ, p.p_derog), (FamilleGpm::Autre, 0.7, 0.1));
    }
    for u in [18, 22, 25, 28] {
        let (f, p) = famille_et_parts(u).unwrap();
        assert_eq!((f, p.p_occ, p.p_derog), (FamilleGpm::Autre, 0.9, 0.0));
    }
}

/// Éclairage d'accentuation (§7.1.3.2.2) : `max {4,5 ; Pecl_tot,l}` pour le petit magasin de
/// vente ; grands volumes : effets de sens sur l'éclairage (météo ADEME).
#[test]
fn sens_des_effets() {
    let base = commerce();
    let ref_ = calcul(&base);
    let [bbio, _, cep, _, _] = phares(&ref_);
    let petit = |p: &mut Projet, w: f64| {
        p.batiments[0].zones[0].groupes[0].locaux[1]
            .eclairage
            .p_ecl_tot_w_m2 = w;
    };

    // 1. Accentuation : 3,0 W/m² saisis (relevés à 4,5) et 4,5 donnent le même calcul ; 6,0 en
    //    consomme davantage ; le Bbio (éq. 782) n'en dépend pas.
    let mut p = base.clone();
    petit(&mut p, 4.5);
    let r45 = calcul(&p);
    assert_eq!(eclairage_kwh(&r45), eclairage_kwh(&ref_));
    assert_eq!(phares(&r45), phares(&ref_));
    petit(&mut p, 6.0);
    let r60 = calcul(&p);
    println!(
        "accentuation : éclairage {:.0} (3,0 et 4,5) → {:.0} kWh (6,0)",
        eclairage_kwh(&ref_),
        eclairage_kwh(&r60)
    );
    assert!(eclairage_kwh(&r60) > eclairage_kwh(&ref_));
    assert_eq!(phares(&r60)[0], bbio);

    // 2. Sans éclairants de toiture : moins de lumière naturelle dans le grand volume, donc plus
    //    d'éclairage artificiel. Le local de grand volume n'a plus que les vitrines (type 0) ; sa
    //    part ayant accès à la lumière naturelle redevient libre (la validation ne l'impose plus).
    let mut p = base.clone();
    p.batiments[0].zones[0].groupes[0]
        .baies
        .retain(|b| b.id != "eclairants");
    p.batiments[0].zones[0].groupes[0].locaux[0].ratio_ecl_nat = 0.5;
    assert!(valider(&p).est_valide());
    let r = calcul(&p);
    println!(
        "sans éclairants : éclairage {:.0} → {:.0} kWh ; Cep {cep:.1} → {:.1}",
        eclairage_kwh(&ref_),
        eclairage_kwh(&r),
        phares(&r)[2]
    );
    assert!(eclairage_kwh(&r) > eclairage_kwh(&ref_));

    // 3. Vitrines affectées à un volume normal au lieu du grand volume : l'éclairement du grand
    //    volume ne vient plus que des éclairants de toiture, celui du petit magasin augmente.
    let mut p = base.clone();
    p.batiments[0].zones[0].groupes[0].baies[0].affectation_eclairage = None;
    let r = calcul(&p);
    println!(
        "vitrines en volume normal : éclairage {:.0} → {:.0} kWh",
        eclairage_kwh(&ref_),
        eclairage_kwh(&r)
    );
    assert_ne!(eclairage_kwh(&r), eclairage_kwh(&ref_));

    // 4. Gymnase : salle de sport sans accès à la lumière naturelle (Ratioécl_nat = 0) : plus
    //    d'éclairage ; gestion fractionnée : jamais plus que non fractionnée.
    let base = gymnase();
    let ref_ = calcul(&base);
    let mut p = base.clone();
    p.batiments[0].zones[0].groupes[0].locaux[0].ratio_ecl_nat = 0.0;
    let sombre = calcul(&p);
    let mut p = base.clone();
    p.batiments[0].zones[0].groupes[0].locaux[0]
        .eclairage
        .fractionne = false;
    let non_fractionne = calcul(&p);
    println!(
        "gymnase : éclairage {:.0} ; sans lumière naturelle {:.0} ; non fractionné {:.0} kWh",
        eclairage_kwh(&ref_),
        eclairage_kwh(&sombre),
        eclairage_kwh(&non_fractionne)
    );
    assert!(eclairage_kwh(&sombre) > eclairage_kwh(&ref_));
    assert!(eclairage_kwh(&non_fractionne) >= eclairage_kwh(&ref_));
    assert_ne!(eclairage_kwh(&non_fractionne), eclairage_kwh(&ref_));

    // 5. Stores enroulables motorisés de la famille « autres » (figure 40-3, tableau 34) : le
    //    DH change.
    let dh = phares(&ref_)[1];
    let mut p = base.clone();
    if let Some(pm) = p.catalogue.baies[0].protection_mobile.as_mut() {
        pm.type_pm = thbce_core::entree::TypePm::StoreEnroulable;
    }
    let r = calcul(&p);
    println!("stores enroulables : DH {dh:.1} → {:.1}", phares(&r)[1]);
    assert_ne!(phares(&r)[1], dh);
}

/// Les autres usages du lot se calculent avec les locaux du tableau 78 de chaque usage : le
/// gymnase en sport privé (28), en vestiaires seuls (18), en aérogare (22) et en industries
/// (23 et 24), et le commerce avec ses locaux d'origine.
#[test]
fn usages_18_22_23_24_et_28() {
    let cas: [(UsageZone, &[(TypeLocal, f64)]); 5] = [
        (
            UsageZone::SportPrive,
            &[
                (TypeLocal::SalleSport, 0.65),
                (TypeLocal::CirculationAccueil, 0.1),
                (TypeLocal::SanitairesVestiaires, 0.15),
                (TypeLocal::DouchesCollectives, 0.05),
                (TypeLocal::LocalService, 0.05),
            ],
        ),
        (
            UsageZone::Vestiaires,
            &[
                (TypeLocal::CirculationAccueil, 0.2),
                (TypeLocal::SanitairesCollectifs, 0.5),
                (TypeLocal::DouchesCollectives, 0.3),
            ],
        ),
        // Les ratios des locaux du classeur des scénarios somment à 0,999 (Q82) ; l'éclairage
        // demande une somme égale à 1.
        (
            UsageZone::Aerogare,
            &[
                (TypeLocal::EspaceVoyageurs, 0.42),
                (TypeLocal::CirculationAccueil, 0.18),
                (TypeLocal::Commerces, 0.11),
                (TypeLocal::BureauStandard, 0.143),
                (TypeLocal::SanitairesVestiaires, 0.104),
                (TypeLocal::InspectionFiltrage, 0.043),
            ],
        ),
        (
            UsageZone::Industrie3x8,
            &[
                (TypeLocal::AireProduction, 0.6),
                (TypeLocal::BureauStandard, 0.1),
                (TypeLocal::CirculationAccueil, 0.1),
                (TypeLocal::SanitairesVestiaires, 0.05),
                (TypeLocal::DouchesCollectives, 0.05),
                (TypeLocal::LocalService, 0.1),
            ],
        ),
        (
            UsageZone::Industrie8h18h,
            &[
                (TypeLocal::AireProduction, 0.6),
                (TypeLocal::BureauStandard, 0.1),
                (TypeLocal::CirculationAccueil, 0.1),
                (TypeLocal::SanitairesVestiaires, 0.05),
                (TypeLocal::DouchesCollectives, 0.05),
                (TypeLocal::LocalService, 0.1),
            ],
        ),
    ];
    for (usage, locaux) in cas {
        let mut p = gymnase();
        p.batiments[0].zones[0].usage = usage;
        let g = &mut p.batiments[0].zones[0].groupes[0];
        let modele = g.locaux[1].clone();
        let grand_volume = locaux
            .iter()
            .any(|(t, _)| *t == TypeLocal::AireProduction || *t == TypeLocal::SalleSport);
        g.locaux = locaux
            .iter()
            .enumerate()
            .map(|(i, (t, ratio))| {
                let mut l = modele.clone();
                l.id = format!("l{i}");
                l.type_local = *t;
                l.ratio_local = *ratio;
                l.ratio_ecl_nat = if matches!(t, TypeLocal::AireProduction | TypeLocal::SalleSport)
                {
                    0.8
                } else {
                    0.0
                };
                l
            })
            .collect();
        if !grand_volume {
            for b in &mut g.baies {
                b.affectation_eclairage = None;
            }
        }
        let v = valider(&p);
        assert!(v.est_valide(), "{usage:?} : {:?}", v.erreurs);
        let r = calcul(&p);
        let [bbio, dh, cep, _, _] = phares(&r);
        println!(
            "usage {} : Bbio {bbio:.1} ; DH {dh:.1} ; Cep {cep:.1}",
            usage.numero()
        );
        assert!(bbio > 0.0 && cep > 0.0);
    }
}

/// Indicateurs fractionnaires des scénarios (Q27, Q63, Q83) : 0,5 en éclairage et ventilation
/// des commerces, une semaine de décembre à 0,5 pour les industries.
#[test]
fn indicateurs_fractionnaires() {
    let compte =
        |usage: UsageZone,
         f: &dyn Fn(&thbce_core::scenarios::scenarios_conventionnels::SortiesZone) -> f64| {
            let sc = ScenariosZone::nouveau(usage, &[100.0], 1).unwrap();
            (0..8760).filter(|&h| f(&sc.evaluer(h)) == 0.5).count()
        };
    // Commerces : 6 cases hebdomadaires à 0,5 en ventilation et en éclairage, sur 52 semaines
    // moins les semaines d'inoccupation annuelle éventuelles.
    let (nv, ne) = (
        compte(UsageZone::Commerces, &|s| s.i_vent),
        compte(UsageZone::Commerces, &|s| s.i_ecl),
    );
    println!("commerces : {nv} h de ventilation et {ne} h d'éclairage à 0,5");
    assert_eq!((nv, ne), (313, 313));
    // Industries : grille annuelle à 0,5 la semaine 4 de décembre, occupation, ventilation et
    // éclairage ; elle ne concerne que les heures où la grille hebdomadaire vaut 1.
    for u in [UsageZone::Industrie3x8, UsageZone::Industrie8h18h] {
        let (no, nv, ne) = (
            compte(u, &|s| s.i_occ_zone),
            compte(u, &|s| s.i_vent),
            compte(u, &|s| s.i_ecl),
        );
        println!("{u:?} : {no} h d'occupation, {nv} de ventilation, {ne} d'éclairage à 0,5");
        let attendu = if u == UsageZone::Industrie3x8 {
            193
        } else {
            60
        };
        assert_eq!((no, nv, ne), (attendu, attendu, attendu));
    }
    // Usage 20 (Q74) : 0,35 de nuit seulement.
    assert!(compte(UsageZone::SanteNuit, &|s| s.i_ecl) == 0);
}

/// Saisie des grands volumes : refus de la validation.
#[test]
fn saisie_des_grands_volumes() {
    let chemins = |p: &Projet| -> Vec<String> {
        valider(p)
            .erreurs
            .iter()
            .map(|e| e.chemin.clone())
            .collect()
    };
    let g = "batiments[0].zones[0].groupes[0]";

    // Baie de grand volume sans local de grand volume (le petit magasin n'en est pas un).
    let mut p = commerce();
    p.batiments[0].zones[0].groupes[0].locaux[0].type_local = TypeLocal::AireVenteInferieure300;
    p.batiments[0].zones[0].groupes[0].locaux[1].type_local = TypeLocal::LocalService;
    let c = chemins(&p);
    assert!(
        c.contains(&format!("{g}.baies[0].affectation_eclairage"))
            && c.contains(&format!("{g}.baies[2].affectation_eclairage")),
        "{c:?}"
    );

    // Éclairant de toiture réparti uniformément : baie horizontale seulement.
    let mut p = commerce();
    p.batiments[0].zones[0].groupes[0].baies[2]
        .exposition
        .inclinaison_deg = 30.0;
    let c = chemins(&p);
    assert!(
        c.contains(&format!("{g}.baies[2].affectation_eclairage")),
        "{c:?}"
    );

    // Avec des éclairants de toiture uniformément répartis, le local de grand volume est éclairé
    // de façon homogène : Ratioécl_nat = 1.
    let mut p = commerce();
    p.batiments[0].zones[0].groupes[0].locaux[0].ratio_ecl_nat = 0.7;
    let c = chemins(&p);
    assert_eq!(c, [format!("{g}.locaux[0].ratio_ecl_nat")], "{c:?}");

    // L'habitation n'a pas de grand volume.
    let mut p = exemple("maison-plancher-ademe");
    p.batiments[0].zones[0].groupes[0].baies[0].affectation_eclairage =
        Some(AffectationEclairage::GrandVolume);
    let c = chemins(&p);
    assert!(
        c.contains(&format!("{g}.baies[0].affectation_eclairage")),
        "{c:?}"
    );

    // Les 28 usages sont calculés : aucun refus « hors périmètre » pour l'usage.
    for u in [
        UsageZone::Commerces,
        UsageZone::Vestiaires,
        UsageZone::Aerogare,
        UsageZone::Industrie3x8,
        UsageZone::Industrie8h18h,
        UsageZone::SportMunicipalScolaire,
        UsageZone::SportPrive,
    ] {
        assert!(u.est_pris_en_charge(), "{u:?}");
    }

    // Un système hors périmètre reste refusé : puits climatique (le double flux et la centrale
    // de traitement d'air en zone neutre sont calculés depuis le lot 1-ven).
    let mut p = commerce();
    p.batiments[0].zones[0].ventilation.as_mut().unwrap().puits =
        Some(thbce_core::entree::Puits::Climatique);
    assert!(valider(&p).seulement_hors_perimetre());
}

fn meteo_reglementaire(p: &Projet) -> MeteoProjet {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO pour lancer ce test");
    let zone = p.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

/// Valeurs phares du commerce (H2b) et du gymnase (H1b) sur la météo réglementaire : nombres
/// annuels seulement.
#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn commerces_sports_sur_meteo_reglementaire() {
    let commerce_nr: &[&str] = &[];
    let gymnase_nr: &[&str] = &["cep", "cep_nr", "dh"];
    for (p, attendu, non_resp) in [
        (commerce(), COMMERCE, commerce_nr),
        (gymnase(), GYMNASE, gymnase_nr),
    ] {
        let r = calculer(&p, &meteo_reglementaire(&p), &Options { arrondir: false }).unwrap();
        assert!(r.reglementaire);
        let v = phares(&r);
        let [bbio, dh, cep, cep_nr, ic] = v;
        println!(
            "{} réglementaire : Bbio {bbio:.1} ; DH {dh:.1} ; Cep {cep:.1} ; Cep,nr {cep_nr:.1} ; Ic {ic:.1}",
            p.batiments[0].id
        );
        let ex = r.batiments[0]
            .indicateurs
            .as_ref()
            .unwrap()
            .exigences
            .as_ref()
            .unwrap();
        let nr: Vec<&str> = ex
            .verifications
            .iter()
            .filter(|v| !v.respectee)
            .map(|v| v.indicateur.as_str())
            .collect();
        println!("non respectées : {nr:?}");
        for (x, a) in v.iter().zip(attendu) {
            assert!((x - a).abs() <= 0.05 + 1e-9, "{v:?} ≠ {attendu:?}");
        }
        assert_eq!(nr, non_resp);
        assert_eq!(ex.conforme, non_resp.is_empty());
    }
}

/// Commerce, météo réglementaire H2b (lot 0c4) : Bbio, DH, Cep, Cep,nr, Ic énergie.
const COMMERCE: [f64; 5] = [148.6, 1938.5, 122.0, 122.0, 144.1];
/// Gymnase, météo réglementaire H1b.
const GYMNASE: [f64; 5] = [100.4, 2262.7, 143.0, 143.0, 161.7];
