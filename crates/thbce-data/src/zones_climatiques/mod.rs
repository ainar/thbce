// SPDX-License-Identifier: GPL-3.0-or-later
//! Zones climatiques et correspondance département → zone.
//!
//! Le découpage de la RE2020 est celui de la RT2012 : la table est l'annexe I de l'arrêté du
//! 26 octobre 2010 (`docs/reference/arrete/arrete_26_octobre_2010_rt2012.pdf`). Elle est
//! extraite par script (`cargo run -p xtask -- zones-climatiques`) dans `table.rs`, jamais
//! saisie à la main.

use core::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Departement;

mod table;

pub use table::ZONE_PAR_DEPARTEMENT;

/// Les huit zones climatiques réglementaires (annexe II §2.2.1.1).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum ZoneClimatique {
    /// H1a
    H1a,
    /// H1b
    H1b,
    /// H1c
    H1c,
    /// H2a
    H2a,
    /// H2b
    H2b,
    /// H2c
    H2c,
    /// H2d
    H2d,
    /// H3
    H3,
}

impl ZoneClimatique {
    /// Les huit zones, dans l'ordre réglementaire.
    pub const TOUTES: [Self; 8] = [
        Self::H1a,
        Self::H1b,
        Self::H1c,
        Self::H2a,
        Self::H2b,
        Self::H2c,
        Self::H2d,
        Self::H3,
    ];

    /// Nom réglementaire, par exemple `"H2b"`.
    pub fn nom(self) -> &'static str {
        match self {
            Self::H1a => "H1a",
            Self::H1b => "H1b",
            Self::H1c => "H1c",
            Self::H2a => "H2a",
            Self::H2b => "H2b",
            Self::H2c => "H2c",
            Self::H2d => "H2d",
            Self::H3 => "H3",
        }
    }

    /// Lit un nom réglementaire (`"H1a"` … `"H3"`).
    pub fn depuis_nom(nom: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|z| z.nom() == nom)
    }

    /// Zone climatique d'un département (arrêté du 26 octobre 2010, annexe I).
    pub fn du_departement(departement: Departement) -> Self {
        let code = departement.code();
        match ZONE_PAR_DEPARTEMENT.binary_search_by(|(c, _)| (*c).cmp(code)) {
            Ok(i) => ZONE_PAR_DEPARTEMENT[i].1,
            // Inatteignable : la table couvre les 96 départements valides (test ci-dessous).
            Err(_) => Self::H1a,
        }
    }
}

impl fmt::Display for ZoneClimatique {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.nom())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_table_couvre_les_96_departements_une_fois_chacun() {
        assert_eq!(ZONE_PAR_DEPARTEMENT.len(), 96);
        // Table triée par code, sans doublon : condition de la recherche dichotomique.
        for paire in ZONE_PAR_DEPARTEMENT.windows(2) {
            assert!(
                paire[0].0 < paire[1].0,
                "{} puis {}",
                paire[0].0,
                paire[1].0
            );
        }
        // Chaque code de la table est un département valide, et réciproquement.
        for (code, zone) in ZONE_PAR_DEPARTEMENT {
            let d = Departement::nouveau(code).unwrap();
            assert_eq!(ZoneClimatique::du_departement(d), zone);
        }
        for i in (1..=95u8).filter(|i| *i != 20) {
            let code = format!("{i:02}");
            assert!(
                ZONE_PAR_DEPARTEMENT.iter().any(|(c, _)| *c == code),
                "{code}"
            );
        }
        for code in ["2A", "2B"] {
            assert!(
                ZONE_PAR_DEPARTEMENT.iter().any(|(c, _)| *c == code),
                "{code}"
            );
        }
    }

    #[test]
    fn les_huit_zones_sont_representees() {
        for zone in ZoneClimatique::TOUTES {
            assert!(
                ZONE_PAR_DEPARTEMENT.iter().any(|(_, z)| *z == zone),
                "{zone}"
            );
            assert_eq!(ZoneClimatique::depuis_nom(zone.nom()), Some(zone));
        }
        assert_eq!(ZoneClimatique::depuis_nom("H4"), None);
    }
}
