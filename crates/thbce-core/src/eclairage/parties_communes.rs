// SPDX-License-Identifier: GPL-3.0-or-later
//! Éclairage des parties communes en logement collectif (Th-BCE 2020 §7.2,
//! `C_ECL_calcul des consommations d'éclairage des parties communes en logement collectif`,
//! p. 502 et 503).
//!
//! Le profil d'utilisation est celui de la mobilité des ascenseurs (« Uk », §4.1.3 éq. 31). La
//! consommation n'est calculée que pour l'usage 2 (logement collectif). Lectures : Q50.

use crate::entree::UsageZone;

/// « Pef_ecl_parties_communes » : puissance surfacique conventionnelle, en W/m² (éq. 791).
pub const P_ECL_PARTIES_COMMUNES_W_M2: f64 = 2.19;

/// Éq. (792) et (793) : consommation du pas de temps, en Wh, pour une zone de surface de
/// référence `sref_m2` et un indice de mobilité `uk` (produit `pamobi × psmobi`).
pub fn consommation_wh(usage: UsageZone, uk: f64, sref_m2: f64) -> f64 {
    if usage == UsageZone::LogementCollectif {
        P_ECL_PARTIES_COMMUNES_W_M2 * uk * sref_m2
    } else {
        0.0
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn seul_le_logement_collectif_consomme() {
        assert!((consommation_wh(UsageZone::LogementCollectif, 0.3, 100.0) - 65.7).abs() < 1e-12);
        assert_eq!(
            consommation_wh(UsageZone::MaisonIndividuelle, 0.3, 100.0),
            0.0
        );
    }
}
