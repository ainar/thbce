// SPDX-License-Identifier: GPL-3.0-or-later
//! Repères horaires du pas de temps (Th-BCE 2020 §3.1.2, §3.1.3 et §4.1.3).
//!
//! Le pas de temps `h` est l'itération « à partir de 0 » du tableau du §4.1.3 : l'itération 0
//! va de 0 h à 1 h UTC. Les repères de semaine et de mois, propres aux scénarios, viendront avec
//! la fiche 4.1 (lot 4).

/// Première itération en heure d'été (§4.1.3, tableau : « passage à l'heure d'été », 1896).
pub const ITERATION_PASSAGE_HEURE_ETE: usize = 1896;
/// Première itération revenue en heure d'hiver (§4.1.3, tableau : 7032).
pub const ITERATION_PASSAGE_HEURE_HIVER: usize = 7032;

/// « IHJ » : heure du jour en temps UTC, de 1 à 24 (§3.1.2).
pub fn ihj(h: usize) -> u8 {
    (h % 24) as u8 + 1
}

/// Indice du jour de l'année, à partir de 0 (« Jour_année » − 1).
pub fn jour(h: usize) -> usize {
    h / 24
}

/// « EHLO » : nombre d'heures de décalage entre l'heure légale et l'heure UTC, 1 en hiver et
/// 2 en été (§3.1.2 ; dates de bascule du §4.1.3).
pub fn ehlo(h: usize) -> u8 {
    if (ITERATION_PASSAGE_HEURE_ETE..ITERATION_PASSAGE_HEURE_HIVER).contains(&h) {
        2
    } else {
        1
    }
}

/// « Hleg » : heure légale de fin du pas de temps.
///
/// Th-BCE 2020 §3.1.3 : `Hleg = IHJ + EHLO`, « cette heure doit être strictement inférieure à
/// 24 h » ; la valeur est donc ramenée dans [0 ; 24[.
pub fn hleg(h: usize) -> u8 {
    (ihj(h) + ehlo(h)) % 24
}

/// Numéro de la case de scénario, de 1 à 24 (§4.1.3, dernière colonne du tableau).
///
/// La dernière ligne du tableau (itération 8759 : « Hleg 1H–2H, case 2 ») contredit la règle
/// d'hiver appliquée partout ailleurs, qui donne la case 1 ; la règle est retenue (question
/// Q22, `docs/questions.md`).
pub fn case_scenario(h: usize) -> u8 {
    (ihj(h) + ehlo(h) - 1) % 24 + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lignes du tableau du §4.1.3 : (itération, Hleg de fin, case de scénario).
    #[test]
    fn reproduit_le_tableau_du_4_1_3() {
        for (h, hleg_fin, case) in [
            (0, 2, 2),
            (1, 3, 3),
            (1895, 1, 1),
            (1896, 3, 3),
            (7031, 2, 2),
            (7032, 2, 2),
        ] {
            assert_eq!(hleg(h), hleg_fin, "itération {h}");
            assert_eq!(case_scenario(h), case, "itération {h}");
        }
    }

    #[test]
    fn bornes() {
        assert_eq!((ihj(0), ihj(23), ihj(24)), (1, 24, 1));
        assert_eq!((jour(0), jour(23), jour(24), jour(8759)), (0, 0, 1, 364));
        assert_eq!(
            (ehlo(1895), ehlo(1896), ehlo(7031), ehlo(7032)),
            (1, 2, 2, 1)
        );
        for h in 0..8760 {
            assert!(hleg(h) < 24);
            assert!((1..=24).contains(&case_scenario(h)));
        }
        // Minuit légal : 22 h–23 h UTC en hiver donne Hleg = 24, ramenée à 0, et la case 24.
        assert_eq!((hleg(22), case_scenario(22)), (0, 24));
    }
}
