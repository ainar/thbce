// SPDX-License-Identifier: GPL-3.0-or-later
//! Réseaux de chaleur et de froid : facteur d'émission et taux d'énergie renouvelable et de
//! récupération (EnR&R).
//!
//! La table est l'annexe 7 de l'arrêté du 15 septembre 2006, dans sa version en vigueur au 25 mai
//! 2026 (`docs/reference/arrete/annexe7_arrete_15_septembre_2006_reseaux_chaleur.pdf`). Elle est
//! extraite par script (`cargo run -p xtask -- reseaux-chaleur`) dans `table.rs`. Seules les
//! données qui servent au calcul des indicateurs (lot 7) sont retenues : la 5e colonne
//! (contenu CO2 ACV, facteur d'émission de l'article 10 de l'arrêté du 4 août 2021) et la 6e
//! (taux d'EnR&R, article 9). Les lignes « autres réseaux » peuvent être employées quel que soit
//! le réseau (§IV de l'annexe).

mod table;

pub use table::{AUTRES_RESEAUX_DE_CHALEUR, AUTRES_RESEAUX_DE_FROID, RESEAUX};

/// Données d'un réseau de chaleur ou de froid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DonneesReseau {
    /// Contenu CO2 ACV, en kg éq. CO2/kWh.
    pub facteur_emission_kg_kwh: f64,
    /// Taux d'énergie renouvelable et de récupération, en % ; `None` pour un réseau de froid.
    pub taux_enrr_pct: Option<f64>,
}

/// Données du réseau d'identifiant `id` (quatre chiffres suivis de « C » ou « F »).
pub fn reseau(id: &str) -> Option<DonneesReseau> {
    RESEAUX
        .binary_search_by(|r| r.0.cmp(id))
        .ok()
        .map(|i| DonneesReseau {
            facteur_emission_kg_kwh: RESEAUX[i].1,
            taux_enrr_pct: RESEAUX[i].2,
        })
}

/// Données de la ligne « autres réseaux de chaleur » ou « autres réseaux de froid ».
pub fn autres_reseaux(froid: bool) -> DonneesReseau {
    if froid {
        DonneesReseau {
            facteur_emission_kg_kwh: AUTRES_RESEAUX_DE_FROID,
            taux_enrr_pct: None,
        }
    } else {
        DonneesReseau {
            facteur_emission_kg_kwh: AUTRES_RESEAUX_DE_CHALEUR.0,
            taux_enrr_pct: Some(AUTRES_RESEAUX_DE_CHALEUR.1),
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn table_de_l_annexe_7() {
        assert_eq!(RESEAUX.len(), 1086);
        assert!(RESEAUX.windows(2).all(|p| p[0].0 < p[1].0));
        // Réseaux de chaleur : taux renseigné ; réseaux de froid : aucun.
        assert!(RESEAUX.iter().all(|r| r.0.ends_with('C') == r.2.is_some()));
        // Lignes lues sur l'image des pages 3 et 82 du PDF.
        let r = reseau("0102C").unwrap();
        assert_eq!(r.facteur_emission_kg_kwh, 0.211);
        assert_eq!(r.taux_enrr_pct, Some(51.1));
        let r = reseau("9519C").unwrap();
        assert_eq!(r.facteur_emission_kg_kwh, 0.004);
        assert_eq!(r.taux_enrr_pct, Some(100.0));
        let r = reseau("9510F").unwrap();
        assert_eq!(r.facteur_emission_kg_kwh, 0.019);
        assert_eq!(r.taux_enrr_pct, None);
        assert!(reseau("0000C").is_none());
        assert_eq!(autres_reseaux(false).facteur_emission_kg_kwh, 0.385);
        assert_eq!(autres_reseaux(false).taux_enrr_pct, Some(0.0));
        assert_eq!(autres_reseaux(true).facteur_emission_kg_kwh, 0.120);
    }
}
