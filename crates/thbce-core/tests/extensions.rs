// SPDX-License-Identifier: GPL-3.0-or-later
//! Exigences des extensions calculées à la main (lot 0d ; Q86 à Q88).
//!
//! Chaque coefficient est relu sur l'image de sa page de
//! `docs/reference/arrete/chapitre1a3_annexe_r172-4_post-rivaton-i.pdf` : `Bbio_maxmoyen`
//! (p. 6), maisons (`Mbgéo`, `Mbsurf_moy` p. 7 ; `Mcsurf_moy` p. 35), logements collectifs
//! (`Mbgéo`, `Mbsurf_moy`, `Mbsurf_tot` p. 8 ; `Mcgéo`, `Mcsurf_moy`, `Mcsurf_tot` p. 36),
//! bureaux (`Mbgéo` p. 9, `Mbsurf_tot` p. 10, `Mcgéo`, `Mcsurf_tot` p. 37), `Cep,nr_maxmoyen` et
//! `Cep_maxmoyen` (p. 30), `Icénergie_maxmoyen` (p. 31 et 32). L'extension est calculée seule :
//! `Sref` et nombre de logements de l'extension (Q87).

#![allow(clippy::unwrap_used)]

use thbce_core::entree::{
    CategorieContraintes as C, ExtensionSaisie, ParametresExigences, ZoneBruit as B,
};
use thbce_core::indicateurs::exigences::{dh_max, exigences_partie, PartieBatiment, SeuilDh};
use thbce_core::indicateurs::extensions::{dh_max_intermediaire, famille, regime};
use thbce_core::indicateurs::resultats::{NatureTravaux as N, RegimeExigences as R};
use thbce_data::ZoneClimatique as Z;

fn parametres(annee: u16, mois: Option<u8>, existant: Option<f64>) -> ParametresExigences {
    ParametresExigences {
        categorie_contraintes_exterieures: C::Categorie1,
        zone_bruit: B::Br1,
        annee_depot_permis: annee,
        mois_depot_permis: mois,
        hsp_moy_m: Some(2.5),
        s_combles_m2: 0.0,
        reseau_chaleur_classe: false,
        parcelle_raccordement_gaz: false,
        zones: Vec::new(),
        extension: Some(ExtensionSaisie {
            surelevation: false,
            sref_existant_m2: existant,
            sref_autres_extensions_m2: 0.0,
        }),
    }
}

fn partie(usage: u8, sref: f64, nl: u32) -> PartieBatiment {
    PartieBatiment {
        usage,
        sref_m2: sref,
        nb_logements: nl,
        sref_usage_m2: sref,
        raccorde_rcu: false,
        categorie: C::Categorie1,
        zone_bruit: B::Br1,
    }
}

fn proche(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} ≠ {b}");
}

/// Extension de maison de 60 m² en H2b, 100 m, permis de 2025 (l'exemple
/// `tests/cas/extension-maison-ademe`) : `Smoylgt` = 60 m² ;
/// `Mbsurf_moy` = (49 − 0,49 × 60)/63 = 19,6/63 ; `Mcsurf_moy` = (49,5 − 0,55 × 60)/55 = 0,3 ;
/// `Mbgéo` = `Mcgéo` = 0.
#[test]
fn extension_de_maison_de_60_m2() {
    let p = parametres(2025, None, None);
    let e = exigences_partie(&partie(1, 60.0, 1), &p, Z::H2b, 100.0).unwrap();
    proche(e.bbio_max, 82.6);
    proche(e.cep_max, 97.5);
    proche(e.cep_nr_max, 71.5);
    proche(e.ic_energie_max, 208.0);
    let dh = dh_max(1, C::Categorie1, Z::H2b, false, 60.0).unwrap();
    assert_eq!(dh, SeuilDh::Maximum(1250.0));
    // Régime : strictement entre 50 et 80 m² → article 50-3, II.
    let f = famille(1);
    assert_eq!(regime(N::Extension, f, 60.0, None, &p), R::Intermediaire);
    // 45 m² → III ; 90 m² → complet.
    assert_eq!(
        regime(N::Extension, f, 45.0, None, &p),
        R::ExigencesDeMoyens
    );
    assert_eq!(regime(N::Extension, f, 90.0, None, &p), R::Complet);
}

/// Extension de maison de 90 m² (régime complet) : `Mbsurf_moy` = (49 − 44,1)/63 ;
/// `Mcsurf_moy` = (49,5 − 49,5)/55 = 0.
#[test]
fn extension_de_maison_de_90_m2() {
    let p = parametres(2025, None, None);
    let e = exigences_partie(&partie(1, 90.0, 1), &p, Z::H2b, 100.0).unwrap();
    proche(e.bbio_max, 67.9);
    proche(e.cep_max, 75.0);
    proche(e.cep_nr_max, 55.0);
    proche(e.ic_energie_max, 160.0);
}

/// Surélévation climatisée de maison de 60 m² en H3, 50 m, catégorie 1 : `Mbgéo` = −0,1
/// (p. 7) : `Bbio_max` = 63 × (1 − 0,1 + 19,6/63) = 76,3 ; régime intermédiaire, `DH_max`
/// porté de 1 250 à 1 400 °C.h (article 50-3, II, 1).
#[test]
fn surelevation_climatisee_de_maison_en_h3() {
    let mut p = parametres(2025, None, None);
    if let Some(x) = &mut p.extension {
        x.surelevation = true;
    }
    let e = exigences_partie(&partie(1, 60.0, 1), &p, Z::H3, 50.0).unwrap();
    proche(e.bbio_max, 76.3);
    assert_eq!(
        regime(N::Surelevation, famille(1), 60.0, None, &p),
        R::Intermediaire
    );
    let dh = dh_max(1, C::Categorie1, Z::H3, true, 60.0)
        .unwrap()
        .maximum()
        .unwrap();
    assert_eq!(dh, 1250.0);
    assert_eq!(
        dh_max_intermediaire(dh, N::Surelevation, 1, true, Z::H3, C::Categorie1),
        1400.0
    );
}

/// Extension de logements collectifs de 120 m² (2 logements, `Smoylgt` = 60 m²), H2b, 100 m,
/// permis de 2025 : `Mbsurf_moy` = (−6 + 0,1 × 60)/65 = 0 ; `Mbsurf_tot` = (19,5 − 0,015 ×
/// 120)/65 = 17,7/65 ; `Mcsurf_moy` = (15 − 0,25 × 60)/70 = 0 ; `Mcsurf_tot` = (13 − 0,01 ×
/// 120)/70 = 11,8/70 ; `Icénergie_maxmoyen` = 260 (2025, autres cas).
#[test]
fn extension_de_logements_collectifs_de_120_m2() {
    let f = famille(2);
    // Existant de 1 000 m² : 12 % → article 50-3, III.
    let p = parametres(2025, None, Some(1000.0));
    assert_eq!(
        regime(N::Extension, f, 120.0, Some(0.12), &p),
        R::ExigencesDeMoyens
    );
    // Existant de 300 m² : 40 % → régime complet.
    let p = parametres(2025, None, Some(300.0));
    assert_eq!(regime(N::Extension, f, 120.0, Some(0.4), &p), R::Complet);
    let e = exigences_partie(&partie(2, 120.0, 2), &p, Z::H2b, 100.0).unwrap();
    proche(e.bbio_max, 82.7);
    proche(e.cep_nr_max, 81.8);
    proche(e.cep_max, 85.0 * 81.8 / 70.0);
    proche(e.ic_energie_max, 260.0 * 81.8 / 70.0);
    // La même en surélévation, permis de juillet 2026 : article 50-3, II.
    let p = parametres(2026, Some(7), Some(300.0));
    assert_eq!(
        regime(N::Surelevation, f, 120.0, Some(0.4), &p),
        R::Intermediaire
    );
}

/// Extension de bureaux de 200 m², H2b, 100 m, catégorie 1, permis de septembre 2026 :
/// `Mbsurf_tot` = (24 − 0,06 × 200)/95 = 12/95 : `Bbio_max` = 107 ; `Mcsurf_tot` = (18 −
/// 0,032 × 200)/75 = 11,6/75 : `Cep,nr_max` = 86,6, `Cep_max` = 85 × 86,6/75,
/// `Icénergie_max` = 200 × 86,6/75 ; `DH_max` = 1 150 (catégorie 1, non climatisé).
#[test]
fn extension_de_bureaux_de_200_m2() {
    let p = parametres(2026, Some(9), Some(500.0));
    assert_eq!(
        regime(N::Extension, famille(3), 200.0, Some(0.4), &p),
        R::Complet
    );
    // 100 m² sur 500 m² existants (20 %) → article 50-3, III.
    assert_eq!(
        regime(N::Extension, famille(3), 100.0, Some(0.2), &p),
        R::ExigencesDeMoyens
    );
    let e = exigences_partie(&partie(3, 200.0, 0), &p, Z::H2b, 100.0).unwrap();
    proche(e.bbio_max, 107.0);
    proche(e.cep_nr_max, 86.6);
    proche(e.cep_max, 85.0 * 86.6 / 75.0);
    proche(e.ic_energie_max, 200.0 * 86.6 / 75.0);
    assert_eq!(
        dh_max(3, C::Categorie1, Z::H2b, false, 0.0),
        Some(SeuilDh::Maximum(1150.0))
    );
}

/// Commerce (usage 17) : extension horizontale de 100 m² sur 500 m² existants hors du champ
/// (article R. 172-1, III, 4°) ; sur 200 m² existants (50 %), régime complet.
#[test]
fn extension_de_commerce() {
    let p = parametres(2026, Some(9), Some(500.0));
    let f = famille(17);
    assert_eq!(regime(N::Extension, f, 100.0, Some(0.2), &p), R::HorsChamp);
    assert_eq!(regime(N::Extension, f, 100.0, Some(0.5), &p), R::Complet);
    assert_eq!(regime(N::Construction, f, 40.0, None, &p), R::HorsChamp);
}
