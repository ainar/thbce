// SPDX-License-Identifier: GPL-3.0-or-later
//! Coefficients de pondération dynamique des émissions de gaz à effet de serre.
//!
//! Source : arrêté du 4 août 2021, article 11, tableau publié en image au Journal officiel
//! du 15 août 2021 (texte 23, page 59), `docs/reference/arrete/jo_15-08-2021_articles_10-11.pdf`.
//! L'article 11 n'a pas été modifié depuis son entrée en vigueur le 1er janvier 2022
//! (version consolidée : `docs/reference/texte/arrete-4-aout-2021-articles.txt`).
//!
//! Le tableau a été transcrit depuis l'image extraite du PDF (`pdfimages`, 396 ppi). Les tests
//! de ce module vérifient la transcription : valeurs des années 0 et 50, décroissance stricte.
//!
//! Usage : annexe II §4.2.1.2 et §4.3.2, éq. (74) — `f_CO2(a)` pondère une émission ayant lieu
//! l'année `a` de la période d'étude de référence.

/// Dernière année tabulée (la période d'étude de référence est de 50 ans, annexe II §1).
pub const ANNEE_MAX: usize = 50;

/// `f_CO2(a)`, « cas général (f_CO2 ou f) », pour `a` de 0 à 50.
pub const F_CO2: [f64; ANNEE_MAX + 1] = [
    1.000, 0.992, 0.984, 0.976, 0.969, 0.961, 0.953, 0.945, 0.937, 0.929, // 0 à 9
    0.921, 0.913, 0.905, 0.897, 0.889, 0.880, 0.872, 0.864, 0.856, 0.848, // 10 à 19
    0.840, 0.831, 0.823, 0.815, 0.806, 0.798, 0.790, 0.781, 0.773, 0.764, // 20 à 29
    0.756, 0.747, 0.739, 0.730, 0.721, 0.713, 0.704, 0.695, 0.686, 0.678, // 30 à 39
    0.669, 0.660, 0.651, 0.642, 0.633, 0.624, 0.615, 0.606, 0.597, 0.587, // 40 à 49
    0.578, // 50
];

/// `f_fluides_frigo(a)`, « fluides frigorigènes », pour `a` de 0 à 50.
pub const F_FLUIDES_FRIGO: [f64; ANNEE_MAX + 1] = [
    1.000, 0.999, 0.998, 0.997, 0.996, 0.995, 0.994, 0.993, 0.992, 0.991, // 0 à 9
    0.990, 0.988, 0.987, 0.986, 0.984, 0.983, 0.981, 0.980, 0.978, 0.976, // 10 à 19
    0.975, 0.973, 0.971, 0.969, 0.967, 0.965, 0.963, 0.961, 0.958, 0.956, // 20 à 29
    0.954, 0.951, 0.948, 0.946, 0.943, 0.940, 0.937, 0.934, 0.930, 0.927, // 30 à 39
    0.923, 0.920, 0.916, 0.912, 0.908, 0.904, 0.899, 0.895, 0.890, 0.885, // 40 à 49
    0.880, // 50
];

/// `f_CO2(a)` ; `None` au-delà de l'année 50.
pub fn f_co2(annee: usize) -> Option<f64> {
    F_CO2.get(annee).copied()
}

/// `f_fluides_frigo(a)` ; `None` au-delà de l'année 50.
pub fn f_fluides_frigo(annee: usize) -> Option<f64> {
    F_FLUIDES_FRIGO.get(annee).copied()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn annee_0_vaut_1_dans_les_deux_colonnes() {
        assert_eq!(F_CO2[0], 1.000);
        assert_eq!(F_FLUIDES_FRIGO[0], 1.000);
    }

    #[test]
    fn les_deux_colonnes_decroissent_strictement() {
        for colonne in [&F_CO2, &F_FLUIDES_FRIGO] {
            for a in 1..=ANNEE_MAX {
                assert!(colonne[a] < colonne[a - 1], "année {a}");
            }
        }
    }

    #[test]
    fn annee_50() {
        assert_eq!(F_CO2[50], 0.578);
        assert_eq!(F_FLUIDES_FRIGO[50], 0.880);
        assert_eq!(f_co2(51), None);
        assert_eq!(f_fluides_frigo(50), Some(0.880));
    }
}
