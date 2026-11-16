// SPDX-License-Identifier: GPL-3.0-or-later
//! Conventions générales de la méthode (Th-BCE 2020 §1).

/// Nombre de pas de temps horaires de l'année conventionnelle.
pub const NB_HEURES_AN: usize = 8760;

/// Mode de calcul.
///
/// Les codes numériques du texte diffèrent d'une fiche à l'autre (§3.1.2 : « 0-Th-B / 4-Th-D /
/// 5-Th-BCD » ; §5.21.2 : « 0-Th-B / 1-Th-C » ; §13.5.3 : « Mode = 4 ou 5 »). Décision Q9
/// (`docs/questions.md`) : le moteur n'expose aucun code ; chaque condition du texte portant sur
/// un code est traduite au cas par cas, avec sa référence en commentaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModeCalcul {
    /// Th-B : besoins bioclimatiques (Bbio).
    ThB,
    /// Th-C : consommations (Cep, Cep,nr).
    ThC,
    /// Th-D : confort d'été (DH).
    ThD,
}

/// Arrondi de restitution à un chiffre après la virgule.
///
/// Th-BCE 2020 §1.3.1 : « Les résultats de calculs, intermédiaires ou finaux, doivent être
/// fournis arrondis avec un chiffre après la virgule. » Décision Q8 (hypothèse du mainteneur, à
/// confirmer par un cas de validation) : le calcul interne reste en pleine précision `f64` ;
/// l'arrondi ne s'applique qu'à la restitution. Les demi-unités s'éloignent de zéro.
pub fn arrondir_restitution(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn arrondit_a_une_decimale() {
        assert_eq!(arrondir_restitution(12.34), 12.3);
        assert_eq!(arrondir_restitution(12.36), 12.4);
        assert_eq!(arrondir_restitution(-0.04), -0.0);
        assert_eq!(arrondir_restitution(0.25), 0.3);
        assert_eq!(arrondir_restitution(-0.25), -0.3);
        assert_eq!(arrondir_restitution(100.0), 100.0);
    }
}
