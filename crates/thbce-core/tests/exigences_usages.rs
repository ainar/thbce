// SPDX-License-Identifier: GPL-3.0-or-later
//! Exigences des usages tertiaires (annexe de l'article R. 172-4 du CCH, chapitres II et III,
//! lot 0b, Q65 à Q68) : chaque valeur attendue est calculée à la main, coefficient par
//! coefficient, à partir des tableaux lus sur l'image des pages citées, sans passer par les
//! tables du moteur.

#![allow(clippy::unwrap_used, clippy::float_cmp)]

use thbce_core::entree::{CategorieContraintes as C, ParametresExigences, ZoneBruit as B};
use thbce_core::indicateurs::exigences::{
    dh_max, exigences_partie, ExigencesPartie, PartieBatiment, SeuilDh,
};
use thbce_data::ZoneClimatique as Z;

fn parametres(annee: u16, categorie: C, bruit: B) -> ParametresExigences {
    ParametresExigences {
        categorie_contraintes_exterieures: categorie,
        zone_bruit: bruit,
        annee_depot_permis: annee,
        hsp_moy_m: None,
        s_combles_m2: 0.0,
        reseau_chaleur_classe: false,
        parcelle_raccordement_gaz: false,
        zones: Vec::new(),
        mois_depot_permis: None,
    }
}

fn partie(usage: u8, sref: f64, rcu: bool, categorie: C, bruit: B) -> PartieBatiment {
    PartieBatiment {
        usage,
        sref_m2: sref,
        nb_logements: 1,
        sref_usage_m2: sref,
        raccorde_rcu: rcu,
        categorie,
        zone_bruit: bruit,
    }
}

fn proche(a: f64, b: f64, quoi: &str) {
    assert!((a - b).abs() < 1e-6, "{quoi} : {a} ≠ {b}");
}

/// Vérifie Bbio_max, Cep_max, Cep,nr_max et Icénergie_max.
fn verifier(e: &ExigencesPartie, bbio: f64, cep: f64, cep_nr: f64, ic: f64) {
    proche(e.bbio_max, bbio, "Bbio_max");
    proche(e.cep_max, cep, "Cep_max");
    proche(e.cep_nr_max, cep_nr, "Cep,nr_max");
    proche(e.ic_energie_max, ic, "Icénergie_max");
}

fn dh(usage: u8, c: C, z: Z, clim: bool) -> Option<SeuilDh> {
    dh_max(usage, c, z, clim, 0.0)
}

/// Bureaux de 2 000 m², H1b, 500 m d'altitude, BR2, catégorie 1, permis de 2026.
///
/// Bbio_maxmoyen 95 (p. 6) ; Mbgéo « 400m-800m », H1b : 0,25 (p. 9) ; Mbsurf_tot, années 2025
/// à 2027, 500 < Sref ≤ 4 000 : (−4,9 − 0,0022 × 2 000)/95 = −9,3/95 (p. 10) ; Mbbruit BR2 : 0.
/// Bbio_max = 95 × 1,25 − 9,3 = 109,45.
///
/// Cep,nr_maxmoyen 75, Cep_maxmoyen 85 (p. 30) ; Icénergie_maxmoyen 2025-2027 autres cas 200
/// (p. 32) ; Mcgéo 0,25 (p. 37) ; Mcsurf_tot, Sref > 1 500 : −6/75 = −0,08 ; Mccat 0.
/// Facteur 1,17 : Cep_max 99,45, Cep,nr_max 87,75, Icénergie_max 234. DH_max 1 150 (p. 101).
#[test]
fn bureaux_categorie_1() {
    let p = parametres(2026, C::Categorie1, B::Br2);
    let z = partie(3, 2000.0, false, C::Categorie1, B::Br2);
    let e = exigences_partie(&z, &p, Z::H1b, 500.0).unwrap();
    proche(e.mb_geo, 0.25, "Mbgéo");
    proche(e.mb_surf_tot, -9.3 / 95.0, "Mbsurf_tot");
    proche(e.mc_surf_tot, -0.08, "Mcsurf_tot");
    assert_eq!((e.mb_bruit, e.mc_cat, e.mb_hsp, e.mc_hsp), (0.0, 0.0, 0.0, 0.0));
    verifier(&e, 109.45, 99.45, 87.75, 234.0);
    assert_eq!(
        dh(3, C::Categorie1, Z::H1b, true),
        Some(SeuilDh::Maximum(1150.0))
    );
}

/// Les mêmes bureaux en H3 à 50 m, catégorie 3, permis de 2030 : Mbgéo « < 400m », H3 : 0,25 ;
/// Mbsurf_tot « à partir de 2028 » : (−3,8 − 0,0044 × 2 000)/95 = −12,6/95 ; Mbbruit
/// « Cat 3 » : 0,4 (p. 10). Bbio_max = 95 × 1,65 − 12,6 = 144,15. Mcgéo 0,15 ; Mcsurf_tot
/// −0,08 ; Mccat 0 : facteur 1,07, Cep_max 90,95, Cep,nr_max 80,25, Icénergie_max 214.
/// DH : « Pas de seuil ».
#[test]
fn bureaux_categorie_3() {
    let p = parametres(2030, C::Categorie3, B::Br1);
    let z = partie(3, 2000.0, false, C::Categorie3, B::Br1);
    let e = exigences_partie(&z, &p, Z::H3, 50.0).unwrap();
    proche(e.mb_bruit, 0.4, "Mbbruit");
    verifier(&e, 144.15, 90.95, 80.25, 214.0);
    assert_eq!(dh(3, C::Categorie3, Z::H3, true), Some(SeuilDh::PasDeSeuil));
    // Catégorie 1 climatisée en H3 : 2 400 ; catégorie 2 : 2 600.
    assert_eq!(
        dh(3, C::Categorie1, Z::H3, true),
        Some(SeuilDh::Maximum(2400.0))
    );
    assert_eq!(
        dh(3, C::Categorie2, Z::H3, true),
        Some(SeuilDh::Maximum(2600.0))
    );
}

/// Bureaux raccordés à un réseau de chaleur urbain classé, permis de 2026 : Icénergie_maxmoyen
/// des raccordés de 2022 à 2024, 280 (p. 32 et 34) ; × 1,17 = 327,6. Réseau non classé : 200.
#[test]
fn bureaux_reseau_de_chaleur() {
    let mut p = parametres(2026, C::Categorie1, B::Br1);
    p.reseau_chaleur_classe = true;
    let z = partie(3, 2000.0, true, C::Categorie1, B::Br1);
    let e = exigences_partie(&z, &p, Z::H1b, 500.0).unwrap();
    proche(e.ic_energie_max_moyen, 280.0, "Icénergie_maxmoyen");
    proche(e.ic_energie_max, 327.6, "Icénergie_max");
    p.reseau_chaleur_classe = false;
    let e = exigences_partie(&z, &p, Z::H1b, 500.0).unwrap();
    proche(e.ic_energie_max_moyen, 200.0, "Icénergie_maxmoyen");
}

/// Enseignement primaire de 800 m², H3, 100 m, catégorie 2, BR2, permis de 2026.
///
/// Bbio_maxmoyen 68 ; Mbgéo 0,50 (p. 10) ; Mbsurf_tot primaire, 500 < Sref ≤ 1 000 :
/// (20 − 0,02 × 800)/68 = 4/68 (p. 11) ; Mbbruit 0. Bbio_max = 68 × 1,5 + 4 = 106.
/// Cep,nr_maxmoyen 65, Cep_maxmoyen 72 ; Icénergie 140 (p. 32) ; Mcgéo 0,30 ; Mcsurf_tot,
/// Sref > 500 : 0 ; Mccat « Cat 2 » : 0,05 (p. 38). Facteur 1,35 : Cep_max 97,2, Cep,nr_max
/// 87,75, Icénergie_max 189. DH_max 2 200 ; catégorie 1 climatisée en H3 : 1 800 (p. 101).
#[test]
fn enseignement_primaire() {
    let p = parametres(2026, C::Categorie2, B::Br2);
    let z = partie(4, 800.0, false, C::Categorie2, B::Br2);
    let e = exigences_partie(&z, &p, Z::H3, 100.0).unwrap();
    proche(e.mc_cat, 0.05, "Mccat");
    verifier(&e, 106.0, 97.2, 87.75, 189.0);
    assert_eq!(
        dh(4, C::Categorie2, Z::H3, true),
        Some(SeuilDh::Maximum(2200.0))
    );
    assert_eq!(
        dh(4, C::Categorie1, Z::H3, true),
        Some(SeuilDh::Maximum(1800.0))
    );
    assert_eq!(
        dh(4, C::Categorie1, Z::H2b, true),
        Some(SeuilDh::Maximum(900.0))
    );
    // Pas de catégorie 3 pour l'enseignement.
    assert_eq!(dh(4, C::Categorie3, Z::H3, true), None);
}

/// Enseignement secondaire de 400 m², mêmes paramètres : Mbsurf_tot secondaire
/// (45 − 0,045 × 400)/68 = 27/68, Bbio_max = 102 + 27 = 129 ; Mcsurf_tot
/// (12,5 − 0,025 × 400)/63 = 2,5/63 (Cep,nr_maxmoyen 63) : Cep,nr_max = 63 × 1,35 + 2,5 =
/// 87,55 ; Cep_max = 72 × (1,35 + 2,5/63) = 100,057 142 9 ; Icénergie_max = 140 × (1,35 +
/// 2,5/63) = 194,555 555 6.
#[test]
fn enseignement_secondaire() {
    let p = parametres(2026, C::Categorie2, B::Br2);
    let z = partie(5, 400.0, false, C::Categorie2, B::Br2);
    let e = exigences_partie(&z, &p, Z::H3, 100.0).unwrap();
    verifier(&e, 129.0, 100.057_142_857, 87.55, 194.555_555_556);
}

/// Hôtel 3 à 5 étoiles, partie nuit, de 600 m², H1c, 900 m, catégorie 3, BR1, permis de
/// 2027, raccordé à un réseau de chaleur non classé.
///
/// Bbio_maxmoyen 76 ; Mbgéo « >800m », H1c : 0,65 (p. 14) ; Mbbruit « Cat 3 » : 0,05 (p. 14) ;
/// Bbio_max = 76 × 1,70 = 129,2. Cep,nr_maxmoyen 118, Cep_maxmoyen 138 (p. 31) ; Icénergie
/// raccordé 2025-2027 : 350 (p. 32) ; Mcgéo 0,35 (p. 42) ; Mcsurf_tot (81 − 0,081 × 600)/118 =
/// 32,4/118 (p. 42) ; Mccat 0. Cep,nr_max = 118 × 1,35 + 32,4 = 191,7 ; Cep_max = 138 ×
/// (1,35 + 32,4/118) = 224,191 525 4 ; Icénergie_max = 350 × (1,35 + 32,4/118) = 568,601 695.
#[test]
fn hotel_partie_nuit() {
    let p = parametres(2027, C::Categorie3, B::Br1);
    let z = partie(9, 600.0, true, C::Categorie3, B::Br1);
    let e = exigences_partie(&z, &p, Z::H1c, 900.0).unwrap();
    proche(e.mb_bruit, 0.05, "Mbbruit");
    verifier(&e, 129.2, 224.191_525_424, 191.7, 568.601_694_915);
    assert_eq!(dh(9, C::Categorie3, Z::H1c, true), Some(SeuilDh::PasDeSeuil));
    assert_eq!(
        dh(9, C::Categorie1, Z::H1c, true),
        Some(SeuilDh::Maximum(300.0))
    );
    // Partie jour (usage 10), catégorie 1 climatisée en H2d : 3 300 (p. 102).
    assert_eq!(
        dh(10, C::Categorie1, Z::H2d, true),
        Some(SeuilDh::Maximum(3300.0))
    );
}

/// Commerce de 300 m², H2d, 50 m, catégorie 2, BR3, permis de 2026.
///
/// Bbio_maxmoyen 170 ; Mbgéo 0,3 (p. 21) ; Mbsurf_tot (47,5 − 0,095 × 300)/170 = 19/170
/// (p. 21) ; Mbbruit BR2/3 : 0 (p. 22). Bbio_max = 170 × 1,3 + 19 = 240. Cep,nr_maxmoyen 163,
/// Cep_maxmoyen 182 ; Icénergie 315 (p. 33) ; Mcgéo 0,2 ; Mcsurf_tot (113 − 0,226 × 300)/163 =
/// 45,2/163 ; Mccat catégorie 2 : 0 (p. 49). Cep,nr_max = 163 × 1,2 + 45,2 = 240,8 ;
/// Cep_max = 182 × (1,2 + 45,2/163) = 268,868 711 7 ; Icénergie_max = 315 × (1,2 + 45,2/163) =
/// 465,349 693 3. DH_max catégorie 2 : 9 500 (p. 103).
#[test]
fn commerce() {
    let p = parametres(2026, C::Categorie2, B::Br3);
    let z = partie(17, 300.0, false, C::Categorie2, B::Br3);
    let e = exigences_partie(&z, &p, Z::H2d, 50.0).unwrap();
    verifier(&e, 240.0, 268.868_711_656, 240.8, 465.349_693_252);
    assert_eq!(
        dh(17, C::Categorie2, Z::H2d, true),
        Some(SeuilDh::Maximum(9500.0))
    );
    // Catégorie 3 : Mbbruit 0,2 et Mccat 0,05.
    let z = partie(17, 300.0, false, C::Categorie3, B::Br3);
    let e = exigences_partie(&z, &p, Z::H2d, 50.0).unwrap();
    proche(e.mb_bruit, 0.2, "Mbbruit");
    proche(e.mc_cat, 0.05, "Mccat");
    proche(e.bbio_max, 170.0 * 1.5 + 19.0, "Bbio_max");
}

/// Établissement de santé, partie jour, de 3 000 m², H2a, 450 m, catégorie 1, BR1, permis de
/// 2028, raccordé à un réseau classé.
///
/// Bbio_maxmoyen 133 ; Mbgéo « 400m-800m », H2a : 0,05 (p. 24) ; Mbsurf_tot, Sref > 2 000 : 0
/// (p. 25) ; Bbio_max = 133 × 1,05 = 139,65. Cep,nr_maxmoyen 165, Cep_maxmoyen 180 ;
/// Icénergie raccordé, à partir de 2028 (la dérogation du réseau classé s'arrête en 2027) : 365
/// (p. 33) ; Mcgéo 0,1 (p. 52) ; Mcsurf_tot, 2 000 < Sref ≤ 5 000 : (49 − 0,026 × 3 000)/165 =
/// −29/165 (p. 53). Cep,nr_max = 165 × 1,1 − 29 = 152,5 ; Cep_max = 180 × (1,1 − 29/165) =
/// 166,363 636 4 ; Icénergie_max = 365 × (1,1 − 29/165) = 337,348 484 8. DH_max catégorie 1
/// climatisée hors H2d et H3 : 1 250 (p. 104).
#[test]
fn sante_partie_jour() {
    let mut p = parametres(2028, C::Categorie1, B::Br1);
    p.reseau_chaleur_classe = true;
    let z = partie(21, 3000.0, true, C::Categorie1, B::Br1);
    let e = exigences_partie(&z, &p, Z::H2a, 450.0).unwrap();
    verifier(&e, 139.65, 166.363_636_364, 152.5, 337.348_484_848);
    assert_eq!(
        dh(21, C::Categorie1, Z::H2a, true),
        Some(SeuilDh::Maximum(1250.0))
    );
    // Partie nuit (usage 20) : pas de catégorie 3 ; catégorie 2 : 2 600 (p. 104).
    assert_eq!(dh(20, C::Categorie3, Z::H3, true), None);
    assert_eq!(
        dh(20, C::Categorie2, Z::H3, true),
        Some(SeuilDh::Maximum(2600.0))
    );
}

/// Industrie de 8 h à 18 h de 1 000 m², H2b, 100 m, raccordée à un réseau classé, permis de
/// 2026 : Mcsurf_tot de la colonne « réseau classé » (365 − 0,073 × 1 000)/94 = 292/94
/// (p. 56) ; Mcgéo 0 (p. 55). Cep,nr_max = 94 + 292 = 386 ; Icénergie_maxmoyen 110 (pas de
/// ligne « raccordés », p. 33) : 110 × (1 + 292/94) = 451,702 127 7.
#[test]
fn industrie_reseau_classe() {
    let mut p = parametres(2026, C::Categorie1, B::Br1);
    p.reseau_chaleur_classe = true;
    let z = partie(24, 1000.0, true, C::Categorie1, B::Br1);
    let e = exigences_partie(&z, &p, Z::H2b, 100.0).unwrap();
    proche(e.cep_nr_max, 386.0, "Cep,nr_max");
    proche(e.ic_energie_max, 451.702_127_660, "Icénergie_max");
}

/// Usages 6 à 28 : pas d'exigence pour un permis déposé avant 2025 (colonne « - »), que la
/// validation refuse (Q68).
#[test]
fn usage_non_soumis_avant_2026() {
    let p = parametres(2024, C::Categorie1, B::Br1);
    let z = partie(6, 500.0, false, C::Categorie1, B::Br1);
    assert!(exigences_partie(&z, &p, Z::H2b, 100.0).is_none());
    // Les bureaux (usage 3) sont soumis depuis 2022.
    let z = partie(3, 500.0, false, C::Categorie1, B::Br1);
    assert!(exigences_partie(&z, &p, Z::H2b, 100.0).is_some());
}
