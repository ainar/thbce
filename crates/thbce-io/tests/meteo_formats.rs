// SPDX-License-Identifier: GPL-3.0-or-later
//! Lecteurs météo : classeur au format réglementaire (sur un classeur synthétique) et fichiers
//! ouverts ADEME / Météo-France (sur le fichier versionné dans `docs/reference/meteo/ademe/`).
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::{
    AnneeMeteo, ClimatExterieur, ConfortAdaptatif, JeuMeteo, MeteoProjet, OrigineMeteo,
};
use thbce_core::conventions::NB_HEURES_AN;
use thbce_data::{Departement, ZoneClimatique};
use thbce_io::meteo::{self, FormatMeteo};

#[cfg(feature = "classeur")]
/// Année synthétique à valeurs variables, pour distinguer lignes, colonnes, zones et jeux.
fn annee_variable(zone: ZoneClimatique, jeu: JeuMeteo) -> AnneeMeteo {
    let mut a = AnneeMeteo::synthetique(zone, jeu);
    let base = zone as usize as f64 * 10.0 + if jeu == JeuMeteo::ThD { 5.0 } else { 0.0 };
    for h in 0..NB_HEURES_AN {
        let x = h as f64;
        a.te0_c[h] = base + x / 1000.0;
        a.we0_g_kg[h] = 4.0 + x / 10_000.0;
        a.idn_w_m2[h] = x % 900.0;
        a.idi_w_m2[h] = x % 300.0;
        a.teciel_c[h] = -10.0 + x / 2000.0;
        a.vent_m_s[h] = x % 7.0;
        a.teau0_c[h] = 8.0 + x / 5000.0;
        // Degrés entiers : la conversion en radians est la même à l'écriture et à la lecture.
        a.gamma_rad[h] = (x % 60.0).to_radians();
        a.psi_rad[h] = (x % 360.0 - 180.0).to_radians();
    }
    a
}

#[cfg(feature = "classeur")]
/// Classeur de même structure que le classeur réglementaire : un onglet de notice, puis 16
/// onglets `<zone>_<mode>` avec une ligne de titre au-dessus de l'en-tête.
fn classeur_synthetique() -> Vec<u8> {
    let mut classeur = rust_xlsxwriter::Workbook::new();
    classeur
        .add_worksheet()
        .set_name("Notice")
        .unwrap()
        .write(0, 0, "Classeur synthétique de test")
        .unwrap();
    let colonnes = [
        "h",
        "te0",
        "we0",
        "dirN (W/m2)",
        "diff (W/m2)",
        "Teciel(C)",
        "Vent(m/s)",
        "Teau(C)",
        "Gamma ()",
        "Psi ()",
    ];
    for zone in ZoneClimatique::TOUTES {
        for jeu in [JeuMeteo::ThBC, JeuMeteo::ThD] {
            let a = annee_variable(zone, jeu);
            let onglet = classeur.add_worksheet();
            onglet
                .set_name(format!("{}_{}", zone.nom(), jeu.nom()))
                .unwrap();
            onglet.write(0, 0, "Données de test").unwrap();
            for (c, nom) in colonnes.iter().enumerate() {
                onglet.write(1, c as u16, *nom).unwrap();
            }
            for h in 0..NB_HEURES_AN {
                let l = h as u32 + 2;
                let valeurs = [
                    h as f64,
                    a.te0_c[h],
                    a.we0_g_kg[h],
                    a.idn_w_m2[h],
                    a.idi_w_m2[h],
                    a.teciel_c[h],
                    a.vent_m_s[h],
                    a.teau0_c[h],
                    (h % 60) as f64,
                    (h % 360) as f64 - 180.0,
                ];
                for (c, v) in valeurs.iter().enumerate() {
                    onglet.write(l, c as u16, *v).unwrap();
                }
            }
        }
    }
    classeur.save_to_buffer().unwrap()
}

#[cfg(feature = "classeur")]
#[test]
fn lit_un_classeur_au_format_reglementaire() {
    let octets = classeur_synthetique();
    assert_eq!(meteo::detecter(&octets), FormatMeteo::ClasseurRe2020);
    for zone in [ZoneClimatique::H1a, ZoneClimatique::H2d, ZoneClimatique::H3] {
        let lu = meteo::lire(&octets, zone).unwrap();
        let mut attendu = MeteoProjet {
            th_bc: annee_variable(zone, JeuMeteo::ThBC),
            th_d: annee_variable(zone, JeuMeteo::ThD),
        };
        // Un classeur au format réglementaire est tenu pour réglementaire.
        attendu.th_bc.origine = OrigineMeteo::ReglementaireRe2020;
        attendu.th_d.origine = OrigineMeteo::ReglementaireRe2020;
        assert_eq!(lu, attendu, "{zone}");
        lu.verifier(zone).unwrap();
    }
}

#[cfg(feature = "classeur")]
#[test]
fn signale_un_classeur_invalide() {
    assert!(matches!(
        meteo::lire(b"PK\x03\x04pas un classeur", ZoneClimatique::H1a),
        Err(meteo::ErreurMeteo::Classeur(_))
    ));
    let mut classeur = rust_xlsxwriter::Workbook::new();
    classeur
        .add_worksheet()
        .set_name("H1a_Th-BC")
        .unwrap()
        .write(0, 0, "h")
        .unwrap();
    let octets = classeur.save_to_buffer().unwrap();
    // En-tête incomplet dans le premier onglet.
    assert!(matches!(
        meteo::lire(&octets, ZoneClimatique::H1a),
        Err(meteo::ErreurMeteo::Classeur(_))
    ));
    // Onglet absent pour une autre zone.
    assert!(matches!(
        meteo::lire(&octets, ZoneClimatique::H3),
        Err(meteo::ErreurMeteo::Classeur(_))
    ));
}

fn fichier_ademe() -> Vec<u8> {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    std::fs::read(chemin).expect("fichier ADEME versionné")
}

/// Heure moyenne pondérée d'une série positive, moyennée sur les jours de l'année.
fn centroide(serie: &[f64]) -> f64 {
    let jours: Vec<f64> = serie
        .chunks(24)
        .filter(|jour| jour.iter().sum::<f64>() > 0.0)
        .map(|jour| {
            jour.iter()
                .enumerate()
                .map(|(h, v)| h as f64 * v)
                .sum::<f64>()
                / jour.iter().sum::<f64>()
        })
        .collect();
    jours.iter().sum::<f64>() / jours.len() as f64
}

#[test]
fn lit_les_huit_stations_ademe_sans_les_dire_reglementaires() {
    let octets = fichier_ademe();
    assert_eq!(meteo::detecter(&octets), FormatMeteo::CsvAdeme);
    for zone in ZoneClimatique::TOUTES {
        let m = meteo::lire(&octets, zone).unwrap_or_else(|e| panic!("{zone} : {e}"));
        m.verifier(zone).unwrap();
        assert!(!m.est_reglementaire(), "{zone}");
        assert!(matches!(
            m.th_bc.origine,
            OrigineMeteo::AdemeProspectif { .. }
        ));
        let a = &m.th_bc;
        for h in 0..NB_HEURES_AN {
            // Mêmes propriétés que le fichier réglementaire : nuit de 23 h à 1 h UTC, grandeurs
            // physiques positives.
            if h % 24 == 0 || h % 24 == 23 {
                assert_eq!(a.gamma_rad[h], 0.0, "{zone} h = {h}");
            }
            assert!(
                a.idn_w_m2[h] >= 0.0
                    && a.idi_w_m2[h] >= 0.0
                    && a.we0_g_kg[h] > 0.0
                    && a.vent_m_s[h] >= 0.0
            );
        }
        // Azimut négatif le matin, positif le soir, croissant dans la journée.
        for jour in 0..365 {
            let de_jour: Vec<usize> = (jour * 24..(jour + 1) * 24)
                .filter(|h| a.gamma_rad[*h] > 0.0)
                .collect();
            assert!(
                a.psi_rad[de_jour[0]] < 0.0 && a.psi_rad[*de_jour.last().unwrap()] > 0.0,
                "{zone} jour {jour}"
            );
            assert!(
                de_jour
                    .windows(2)
                    .all(|p| a.psi_rad[p[1]] > a.psi_rad[p[0]]),
                "{zone} jour {jour}"
            );
        }
        // Convention horaire : rayonnement diffus centré sur le même instant que la hauteur du
        // soleil, comme dans le fichier réglementaire. La nébulosité n'étant pas symétrique autour
        // de midi, l'écart toléré est de 0,3 h ; un décalage d'une heure en donnerait 1.
        let (c_gamma, c_diffus) = (centroide(&a.gamma_rad), centroide(&a.idi_w_m2));
        assert!(
            (c_gamma - c_diffus).abs() < 0.3,
            "{zone} : soleil {c_gamma:.2} h, diffus {c_diffus:.2} h"
        );
        // Le midi solaire tombe une demi-heure après 12 h − longitude/15, soit entre 12 h et 13 h.
        assert!((12.0..13.0).contains(&c_gamma), "{zone} : {c_gamma:.2}");
    }
}

#[test]
fn enchaine_le_climat_sur_une_station_ademe() {
    // Tours (H2b), Indre-et-Loire, 80 m : altcorr = 100 m, donc Te = T − 0,5 °C.
    let m = meteo::lire(&fichier_ademe(), ZoneClimatique::H2b).unwrap();
    let climat = ClimatExterieur::nouveau(&m.th_d, Departement::nouveau("37").unwrap(), 80.0);
    assert_eq!(climat.te_c[100], m.th_d.te0_c[100] - 0.5);
    assert!(climat.edn_lux.iter().all(|e| *e >= 0.0));
    // Un climat à +2,7 °C a une période de confort adaptatif, centrée sur l'été.
    let c = ConfortAdaptatif::nouveau(&climat.te_c);
    let (premier, dernier) = (c.premier_jour.unwrap(), c.dernier_jour.unwrap());
    assert!(premier < 182 && dernier > 182, "{premier}..{dernier}");
}
