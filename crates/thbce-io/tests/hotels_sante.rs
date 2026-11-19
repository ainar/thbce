// SPDX-License-Identifier: GPL-3.0-or-later
//! Hôtels (usages 8 à 11, parties nuit et jour), établissements sanitaires avec hébergement (19)
//! et établissements de santé (20 et 21) : lot 0c2. Exemples `tests/cas/hotel-pac-ademe` (usages
//! 8 et 10 en deux zones) et `tests/cas/ehpad-pac-ademe` (usage 19) sur la météo ouverte ADEME
//! (leurs valeurs sont vérifiées par leur `attendu.json`) ; calculs à la main ; sens des effets ;
//! autres usages du lot par substitution ; refus.
//!
//! Valeurs phares sur la météo réglementaire :
//! THBCE_METEO=/chemin/vers/meteo.csv cargo t-regl hotels_sante_sur_meteo_reglementaire

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{GestionPm, Projet, UsageZone};
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

fn hotel() -> Projet {
    exemple("hotel-pac-ademe")
}

fn ehpad() -> Projet {
    exemple("ehpad-pac-ademe")
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

/// Valeurs recalculées à la main, indépendamment du code des postes concernés : surfaces,
/// débits conventionnels du Bbio (tableau 56-1), forfait de refroidissement (éq. 2493,
/// tableau 335), familles des protections mobiles (figure 34).
#[test]
fn hotel_et_ehpad_a_la_main() {
    let (ph, pe) = (hotel(), ehpad());
    for p in [&ph, &pe] {
        let v = valider(p);
        assert!(v.est_valide(), "{:?}", v.erreurs);
        assert!(v.avertissements.is_empty(), "{:?}", v.avertissements);
    }
    let (rh, re) = (calcul(&ph), calcul(&pe));
    for r in [&rh, &re] {
        let [bbio, dh, cep, cep_nr, ic] = phares(r);
        println!(
            "ADEME : Bbio {bbio:.1} ; DH {dh:.1} ; Cep {cep:.1} ; Cep,nr {cep_nr:.1} ; Ic {ic:.1}"
        );
        // Électricité seule : pas d'énergie renouvelable comptée, Cep = Cep,nr.
        assert!((cep - cep_nr).abs() < 1e-9);
    }
    let (bh, be) = (&rh.batiments[0], &re.batiments[0]);
    // §13.1.3, éq. (2376) : SREF = SU hors habitation.
    assert_eq!((bh.sref_m2, be.sref_m2), (1200.0, 1500.0));

    // Tableau 56-1 (p. 334) : hôtels 0 à 2 étoiles 3,0 (nuit) et 8,0 (jour) m³/(h.m²) ;
    // établissements sanitaires avec hébergement 4,5. En occupation : 3,0 × 900 = 2 700,
    // 8,0 × 300 = 2 400 et 4,5 × 1 500 = 6 750 m³/h.
    let debit = |b: &thbce_core::resultats::ResultatsBatiment, z: usize| {
        let d = b.zones[z].groupes[0].debits_bbio.as_ref().unwrap();
        (d.q_spec_repris_m3_h, d.q_spec_souffle_m3_h)
    };
    assert_eq!(debit(bh, 0), (-2700.0, 2700.0));
    assert_eq!(debit(bh, 1), (-2400.0, 2400.0));
    assert_eq!(debit(be, 0), (-6750.0, 6750.0));

    // Figure 34 (p. 195 et 196) : familles des protections mobiles.
    for (u, f) in [
        (8, FamilleGpm::Habitation),
        (9, FamilleGpm::Habitation),
        (10, FamilleGpm::Bureaux),
        (11, FamilleGpm::Bureaux),
        (19, FamilleGpm::Habitation),
        (20, FamilleGpm::Hopitaux),
        (21, FamilleGpm::Bureaux),
    ] {
        assert_eq!(famille_et_parts(u).unwrap().0, f, "usage {u}");
    }

    // Éq. (2493) : Cef = Coef_kWh_fr_par_DH × (min(DH ; seuil haut) − seuil bas)
    // × Coef_zone_clim_et_alt / CoefEP[électricité], par m² de SREF. Seuil bas 350 °C.h ;
    // seuil haut : DH_max de la catégorie 1, 1 300 °C.h pour l'hôtel (partie jour, usage 10) et
    // 900 °C.h pour l'établissement sanitaire (annexe de l'article R. 172-4, chapitre III,
    // tableaux DH_maxcat) ; ces DH dépassent le seuil. Pour la partie nuit de l'hôtel, le seuil
    // haut (300 °C.h) est inférieur au seuil bas : pas de forfait. Coefficient 0,009 (tableau
    // 335, p. 1384) ; 1,0 en H2b à 80 m (tableau 336) ; CoefEP de l'électricité 2,3.
    assert!(bh.zones[1].dh_c_h.unwrap() > 1300.0 && be.dh_c_h.unwrap() > 900.0);
    let attendu_h = 0.009 * (1300.0 - 350.0) * 1.0 / 2.3 * 300.0;
    let attendu_e = 0.009 * (900.0 - 350.0) * 1.0 / 2.3 * 1500.0;
    assert!((elec(&rh).refroidissement_kwh - attendu_h).abs() < 1e-6);
    assert!((elec(&re).refroidissement_kwh - attendu_e).abs() < 1e-6);
}

/// Variantes de sens (météo ADEME).
#[test]
fn sens_des_effets() {
    let base = hotel();
    let ref_ = calcul(&base);
    let [bbio, dh, cep, _, ic] = phares(&ref_);

    // 1. Plus d'isolant (U des parois divisés par deux) : Bbio, Cep et Ic énergie baissent.
    let mut p = base.clone();
    for t in &mut p.catalogue.parois {
        t.u_k_w_m2k /= 2.0;
    }
    let [bbio2, _, cep2, _, ic2] = phares(&calcul(&p));
    println!(
        "isolant : Bbio {bbio:.1} → {bbio2:.1} ; Cep {cep:.1} → {cep2:.1} ; Ic {ic:.1} → {ic2:.1}"
    );
    assert!(bbio2 < bbio && cep2 < cep && ic2 < ic);

    // 2. Volets manuels non motorisés au lieu de motorisés (figures 35 et 36 : familles
    //    « habitation » pour la partie nuit et « bureau » pour la partie jour) : le DH change.
    let mut p = base.clone();
    p.catalogue.baies[0]
        .protection_mobile
        .as_mut()
        .unwrap()
        .gestion = Some(GestionPm::ManuelleNonMotorisee);
    let dh2 = phares(&calcul(&p))[1];
    println!("volets manuels : DH {dh:.1} → {dh2:.1}");
    assert_ne!(dh2, dh);

    // 3. Partie nuit en hôtel 3 à 5 étoiles (usage 9, mêmes locaux : sanitaires collectifs
    //    plus éclairés au tableau 78) : le calcul aboutit, la ventilation du Bbio passe à
    //    2,0 m³/(h.m²) en occupation (tableau 56-1).
    let mut p = base.clone();
    p.batiments[0].zones[0].usage = UsageZone::Hotel345Nuit;
    p.batiments[0].zones[1].usage = UsageZone::Hotel345Jour;
    p.batiments[0].zones[1].groupes[0].locaux[0]
        .eclairage
        .p_ecl_tot_w_m2 = 6.0;
    let v = valider(&p);
    assert!(v.est_valide(), "{:?}", v.erreurs);
    let r = calcul(&p);
    let d = r.batiments[0].zones[0].groupes[0]
        .debits_bbio
        .as_ref()
        .unwrap();
    assert_eq!(d.q_spec_souffle_m3_h, 2.0 * 900.0);
    let d = r.batiments[0].zones[1].groupes[0]
        .debits_bbio
        .as_ref()
        .unwrap();
    assert_eq!(d.q_spec_souffle_m3_h, 7.0 * 300.0);
    assert_ne!(phares(&r)[0], bbio);

    // 4. Éclairage des chambres sans gestion (interrupteur) au lieu d'un détecteur : sans
    //    effet sur le Bbio (gestion conventionnelle, éq. 782), plus de consommation sinon.
    let mut p = base.clone();
    let e = &mut p.batiments[0].zones[0].groupes[0].locaux[0].eclairage;
    e.p_ecl_tot_w_m2 *= 2.0;
    let r = calcul(&p);
    println!(
        "éclairage ×2 : {:.0} → {:.0} kWh",
        elec(&ref_).eclairage_kwh,
        elec(&r).eclairage_kwh
    );
    assert!(elec(&r).eclairage_kwh > elec(&ref_).eclairage_kwh);
}

/// Usages 19 (EHPAD), 20 et 21 : l'EHPAD en établissement de santé partie nuit (20) puis jour
/// (21), avec les locaux du tableau 78 de chaque usage.
#[test]
fn usages_20_et_21() {
    let base = ehpad();
    let r19 = calcul(&base);

    // Usage 20 : les locaux du tableau 78 ; l'éclairage suit la grille de l'onglet `SAN_N`
    // (0,35 de nuit, Q74).
    let mut p = base.clone();
    p.batiments[0].zones[0].usage = UsageZone::SanteNuit;
    let locaux = &mut p.batiments[0].zones[0].groupes[0].locaux;
    for l in locaux.iter_mut() {
        l.type_local = match l.type_local {
            TypeLocal::LocalService => TypeLocal::LocauxSoinsOffices,
            TypeLocal::SalleCommune => TypeLocal::SalleAttenteConsultation,
            t => t,
        };
    }
    let v = valider(&p);
    assert!(v.est_valide(), "{:?}", v.erreurs);
    let r20 = calcul(&p);
    let [bbio, dh, cep, ..] = phares(&r20);
    println!(
        "usage 20 : Bbio {bbio:.1} ; DH {dh:.1} ; Cep {cep:.1} ; éclairage {:.0} kWh",
        elec(&r20).eclairage_kwh
    );
    // Valeurs de Q74 (lecture retenue C1 × IEcl ; météo ADEME), à 0,1 près.
    let proche = |a: f64, b: f64| assert!((a - b).abs() <= 0.1, "{a} ≠ {b}");
    proche(bbio, 211.4);
    proche(dh, 2574.8);
    proche(cep, 176.0);
    assert!((elec(&r20).eclairage_kwh - 40_248.0).abs() <= 1.0);
    // Grille d'éclairage de l'usage 20 : 0 ou 0,35 ou 1, avec des heures à 0,35.
    let sc = ScenariosZone::nouveau(UsageZone::SanteNuit, &[1500.0], 1).unwrap();
    let valeurs: Vec<f64> = (0..8760).map(|h| sc.evaluer(h).i_ecl).collect();
    assert!(valeurs.iter().all(|&x| x == 0.0 || x == 0.35 || x == 1.0));
    assert!(valeurs.contains(&0.35) && valeurs.contains(&1.0));
    // Consommation d'éclairage positive dans les deux usages (le calcul à la main de la
    // fraction est dans `crates/thbce-core/tests/eclairage.rs`).
    assert!(elec(&r20).eclairage_kwh > 0.0 && elec(&r19).eclairage_kwh > 0.0);

    // Usage 21 : partie jour des établissements de santé (famille « bureau » des protections
    // mobiles, pas de ventilation nocturne : Param_inoccupation = 0, tableau 56-1).
    let mut p = base.clone();
    p.batiments[0].zones[0].usage = UsageZone::SanteJour;
    let locaux = &mut p.batiments[0].zones[0].groupes[0].locaux;
    for l in locaux.iter_mut() {
        l.type_local = match l.type_local {
            TypeLocal::ChambreSansCuisine => TypeLocal::AireProduction,
            TypeLocal::LocalService => TypeLocal::SalleReunion,
            TypeLocal::SalleCommune => TypeLocal::SalleAttenteConsultation,
            t => t,
        };
    }
    let v = valider(&p);
    assert!(v.est_valide(), "{:?}", v.erreurs);
    let r21 = calcul(&p);
    assert!(phares(&r21)[2] > 0.0);
    let d = r21.batiments[0].zones[0].groupes[0]
        .debits_bbio
        .as_ref()
        .unwrap();
    assert_eq!(d.q_spec_souffle_m3_h, 4.5 * 1500.0);
}

#[test]
fn refus() {
    let chemins = |p: &Projet| -> Vec<String> {
        valider(p)
            .erreurs
            .iter()
            .map(|e| e.chemin.clone())
            .collect()
    };
    // Permis de 2025 : usage non soumis à la RE2020 avant le 1er mai 2026 (Q68).
    for mut p in [hotel(), ehpad()] {
        let x = p.batiments[0].exigences.as_mut().unwrap();
        x.annee_depot_permis = 2025;
        x.mois_depot_permis = None;
        assert!(!valider(&p).est_valide());
    }
    // Permis de 2026 sans mois : erreur de saisie (décision de l'utilisateur du 17 novembre).
    let mut p = ehpad();
    p.batiments[0].exigences.as_mut().unwrap().mois_depot_permis = None;
    let c = chemins(&p);
    assert!(c.iter().any(|c| c.ends_with("mois_depot_permis")), "{c:?}");
    // Usage 20 : le « local de service » n'existe pas dans le tableau 78.
    let mut p = ehpad();
    p.batiments[0].zones[0].usage = UsageZone::SanteNuit;
    let v = valider(&p);
    assert!(
        v.erreurs
            .iter()
            .any(|e| e.chemin.contains("locaux") && e.chemin.ends_with("type_local")),
        "{:?}",
        v.erreurs
    );
    // Hôtel : pas de salle de classe.
    let mut p = hotel();
    p.batiments[0].zones[0].groupes[0].locaux[0].type_local = TypeLocal::SalleClasse;
    assert!(!valider(&p).est_valide());
    // Les usages d'autres familles (commerces, aérogares…) restent refusés : voir
    // `squelette.rs`.
}

fn meteo_reglementaire(p: &Projet) -> MeteoProjet {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO pour lancer ce test");
    let zone = p.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

/// Valeurs phares de l'hôtel et de l'EHPAD (H2b) sur la météo réglementaire : nombres annuels
/// seulement.
#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn hotels_sante_sur_meteo_reglementaire() {
    // Exigences non respectées sur la météo réglementaire.
    let hotel_nr: &[&str] = &["bbio", "cep", "cep_nr", "dh"];
    let ehpad_nr: &[&str] = &["dh"];
    for (p, attendu, non_resp) in [(hotel(), HOTEL, hotel_nr), (ehpad(), EHPAD, ehpad_nr)] {
        let r = calculer(&p, &meteo_reglementaire(&p), &Options { arrondir: false }).unwrap();
        assert!(r.reglementaire);
        let v = phares(&r);
        let [bbio, dh, cep, cep_nr, ic] = v;
        println!(
            "{} réglementaire : Bbio {bbio:.1} ; DH {dh:.1} ; Cep {cep:.1} ; Cep,nr {cep_nr:.1} ; Ic {ic:.1}",
            p.batiments[0].id
        );
        for (x, a) in v.iter().zip(attendu) {
            assert!((x - a).abs() <= 0.05 + 1e-9, "{v:?} ≠ {attendu:?}");
        }
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
        assert_eq!(nr, non_resp);
        assert!(!ex.conforme);
    }
}

/// Hôtel, météo réglementaire H2b (lot 0c2) : Bbio, DH, Cep, Cep,nr, Ic énergie.
const HOTEL: [f64; 5] = [103.6, 1253.6, 193.4, 193.4, 229.6];
/// EHPAD, météo réglementaire H2b.
const EHPAD: [f64; 5] = [160.2, 1310.1, 188.8, 188.8, 230.0];
