// SPDX-License-Identifier: GPL-3.0-or-later
//! Température extérieure de base par département.
//!
//! Th-BCE 2020 §3.1.3, « Calcul de la température de base extérieure » : le tableau affecte une
//! température `θbase_ext_0` à chaque département (−9, −6 ou −3 °C). La table est extraite par
//! script (`cargo run -p xtask -- temperature-base`) dans `table.rs`.

use crate::Departement;

mod table;

pub use table::BASE_EXT_0_PAR_DEPARTEMENT;

/// `θbase_ext_0` : température de base du département à l'altitude 0 m, en °C.
pub fn base_ext_0_c(departement: Departement) -> f64 {
    let code = departement.code();
    match BASE_EXT_0_PAR_DEPARTEMENT.binary_search_by(|(c, _)| (*c).cmp(code)) {
        Ok(i) => BASE_EXT_0_PAR_DEPARTEMENT[i].1,
        // Inatteignable : la table couvre les 96 départements valides (test ci-dessous).
        Err(_) => f64::NAN,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn la_table_couvre_les_96_departements() {
        assert_eq!(BASE_EXT_0_PAR_DEPARTEMENT.len(), 96);
        for paire in BASE_EXT_0_PAR_DEPARTEMENT.windows(2) {
            assert!(paire[0].0 < paire[1].0);
        }
        for (code, v) in BASE_EXT_0_PAR_DEPARTEMENT {
            let d = Departement::nouveau(code).unwrap();
            assert_eq!(base_ext_0_c(d), v);
            assert!([-9.0, -6.0, -3.0].contains(&v), "{code} : {v}");
        }
    }

    #[test]
    fn effectifs_et_exemples_du_tableau() {
        // §3.1.3 : 51 départements à −9 °C, 36 à −6 °C, 9 à −3 °C.
        let effectif = |v: f64| {
            BASE_EXT_0_PAR_DEPARTEMENT
                .iter()
                .filter(|(_, x)| *x == v)
                .count()
        };
        assert_eq!(
            (effectif(-9.0), effectif(-6.0), effectif(-3.0)),
            (51, 36, 9)
        );
        let de = |code: &str| base_ext_0_c(Departement::nouveau(code).unwrap());
        assert_eq!(de("01"), -9.0);
        assert_eq!(de("95"), -9.0);
        assert_eq!(de("81"), -6.0);
        assert_eq!(de("86"), -6.0);
        assert_eq!(de("2A"), -3.0);
        assert_eq!(de("83"), -3.0);
    }
}
