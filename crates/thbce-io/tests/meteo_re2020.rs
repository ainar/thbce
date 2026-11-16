// SPDX-License-Identifier: GPL-3.0-or-later
//! Contrôles du fichier météo réglementaire.
//!
//! Ce fichier ne peut pas être versionné (conditions d'utilisation). Ces tests sont ignorés par
//! défaut et se lancent en local :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io -- --ignored
//! ```

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use thbce_core::climat::{AnneeMeteo, MeteoProjet};
use thbce_core::conventions::NB_HEURES_AN;
use thbce_data::ZoneClimatique;
use thbce_io::meteo::lire_csv_re2020;

fn charger() -> Vec<MeteoProjet> {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO pour lancer ce test");
    let octets = std::fs::read(chemin).expect("lecture du fichier THBCE_METEO");
    ZoneClimatique::TOUTES
        .into_iter()
        .map(|zone| lire_csv_re2020(&octets, zone).unwrap_or_else(|e| panic!("{zone} : {e}")))
        .collect()
}

fn annees(jeux: &[MeteoProjet]) -> impl Iterator<Item = &AnneeMeteo> {
    jeux.iter().flat_map(|m| [&m.th_bc, &m.th_d])
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn les_huit_zones_et_les_deux_jeux_sont_complets() {
    let jeux = charger();
    assert_eq!(jeux.len(), 8);
    for (m, zone) in jeux.iter().zip(ZoneClimatique::TOUTES) {
        m.verifier(zone).unwrap();
        assert!(m.est_reglementaire());
    }
}

/// Décision Q1b : `h = 0` est l'itération 0 (0 h à 1 h UTC) ; `Gamma` vaut zéro la nuit.
#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
#[allow(clippy::float_cmp)]
fn gamma_est_nul_la_nuit() {
    for a in annees(&charger()) {
        for h in 0..NB_HEURES_AN {
            // Heures 23 h – 1 h UTC : nuit en toute saison en France métropolitaine.
            if h % 24 == 0 || h % 24 == 23 {
                assert_eq!(a.gamma_rad[h], 0.0, "{} {} h = {h}", a.zone, a.jeu.nom());
            }
            assert!(a.gamma_rad[h] >= 0.0);
        }
    }
}

/// Décision Q1b : `Psi` est l'azimut du soleil par rapport au sud, négatif à l'est (le matin),
/// positif à l'ouest (le soir), et croît au fil de la journée.
#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn psi_est_negatif_le_matin_et_croit_dans_la_journee() {
    for a in annees(&charger()) {
        for jour in 0..NB_HEURES_AN / 24 {
            let heures_de_jour: Vec<usize> = (jour * 24..(jour + 1) * 24)
                .filter(|h| a.gamma_rad[*h] > 0.0)
                .collect();
            let contexte = format!("{} {} jour {jour}", a.zone, a.jeu.nom());
            let (Some(premiere), Some(derniere)) = (heures_de_jour.first(), heures_de_jour.last())
            else {
                panic!("{contexte} : aucune heure de jour");
            };
            assert!(
                a.psi_rad[*premiere] < 0.0,
                "{contexte} : Psi positif au lever"
            );
            assert!(
                a.psi_rad[*derniere] > 0.0,
                "{contexte} : Psi négatif au coucher"
            );
            for paire in heures_de_jour.windows(2) {
                assert!(
                    a.psi_rad[paire[1]] > a.psi_rad[paire[0]],
                    "{contexte} : Psi non croissant à h = {}",
                    paire[1]
                );
            }
        }
        // À minuit UTC, le soleil est au nord : |Psi| proche de 180°.
        assert!(
            a.psi_rad[0].abs() > 150f64.to_radians(),
            "{} {}",
            a.zone,
            a.jeu.nom()
        );
    }
}
