// SPDX-License-Identifier: GPL-3.0-or-later
//! Repères horaires du pas de temps (Th-BCE 2020 §3.1.2, §3.1.3 et §4.1.3).
//!
//! Le pas de temps `h` est l'itération « à partir de 0 » du tableau du §4.1.3 : l'itération 0
//! va de 0 h à 1 h UTC. [`date_scenario`] donne les repères de mois, de semaine et de jour des scénarios.

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

/// Nombre de semaines de chaque mois de l'année conventionnelle (§4.1.3).
pub const SEMAINES_PAR_MOIS: [u8; 12] = thbce_data::scenarios::SEMAINES_PAR_MOIS;

/// Date d'un pas de temps au sens des scénarios d'occupation (§4.1.3, « Calendrier »).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateScenario {
    /// Mois de l'année, de 1 à 12.
    pub mois: u8,
    /// Semaine du mois, de 1 à 4 ou 5 selon le mois.
    pub semaine: u8,
    /// Jour de la semaine, de 1 (lundi) à 7.
    pub jour: u8,
    /// Heure du jour, de 1 à 24 : numéro de la case de scénario, en heure légale.
    pub heure: u8,
}

/// Date d'un pas de temps au sens des scénarios.
///
/// Th-BCE 2020 §4.1.3 : le mois, la semaine du mois, le jour de la semaine et l'heure du jour
/// « se déduisent de l'heure de la simulation par incrémentations successives » ; « on suppose
/// que l'année commence un lundi » ; les scénarios « sont décrits en heure légale ».
///
/// Lectures retenues (`docs/questions.md`) :
/// - l'heure est la case de scénario du tableau du §4.1.3, et le jour change quand cette case
///   revient à 1, c'est-à-dire à minuit en heure légale (question Q28) ;
/// - l'année conventionnelle compte 52 semaines, soit 364 jours ; au-delà, le pas de temps est
///   rattaché à un lundi ajouté à la dernière semaine de décembre (décision Q17).
pub fn date_scenario(h: usize) -> DateScenario {
    let heure = case_scenario(h);
    // Indice du jour légal : l'itération 0 est la case 2 du premier jour ; le jour suivant
    // commence à l'itération dont la case vaut 1.
    let jour_legal = (h + usize::from(ehlo(h))) / 24;
    if jour_legal >= 364 {
        return DateScenario {
            mois: 12,
            semaine: SEMAINES_PAR_MOIS[11],
            jour: 1,
            heure,
        };
    }
    let mut semaine = jour_legal / 7;
    let jour = (jour_legal % 7) as u8 + 1;
    for (mois, semaines) in SEMAINES_PAR_MOIS.iter().enumerate() {
        let semaines = usize::from(*semaines);
        if semaine < semaines {
            return DateScenario {
                mois: mois as u8 + 1,
                semaine: semaine as u8 + 1,
                jour,
                heure,
            };
        }
        semaine -= semaines;
    }
    // Inatteignable : 52 semaines couvrent les jours 0 à 363.
    DateScenario {
        mois: 12,
        semaine: SEMAINES_PAR_MOIS[11],
        jour,
        heure,
    }
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
    fn date_des_scenarios() {
        let d = |h| {
            let d = date_scenario(h);
            (d.mois, d.semaine, d.jour, d.heure)
        };
        // L'année commence un lundi ; l'itération 0 est la case 2 du lundi de la semaine 1.
        assert_eq!(d(0), (1, 1, 1, 2));
        assert_eq!(d(22), (1, 1, 1, 24));
        // Minuit légal : l'itération 23 ouvre le mardi.
        assert_eq!(d(23), (1, 1, 2, 1));
        // Janvier compte 4 semaines (28 jours) : le 29e jour légal ouvre février.
        assert_eq!(d(28 * 24 - 2), (1, 4, 7, 24));
        assert_eq!(d(28 * 24 - 1), (2, 1, 1, 1));
        // Mars compte 5 semaines : jours 56 à 90.
        assert_eq!(d(56 * 24), (3, 1, 1, 2));
        // À partir de l'itération 1896, l'heure d'été décale la case d'une heure.
        assert_eq!(d(90 * 24), (3, 5, 7, 3));
        assert_eq!(d(91 * 24), (4, 1, 1, 3));
        // Passage à l'heure d'été : la case 2 est sautée, le jour ne change pas deux fois.
        assert_eq!((d(1895).3, d(1896).3), (1, 3));
        assert_eq!((d(1895).2, d(1896).2), (d(1894).2 % 7 + 1, d(1895).2));
        // Retour à l'heure d'hiver : la case 2 est jouée deux fois le même jour.
        assert_eq!(d(7031), d(7032));
        // Décision Q17 : au-delà de 364 jours, lundi de la dernière semaine de décembre.
        assert_eq!(d(363 * 24 + 22), (12, 4, 7, 24));
        assert_eq!(d(363 * 24 + 23), (12, 4, 1, 1));
        assert_eq!(d(8758), (12, 4, 1, 24));
        assert_eq!(d(8759), (12, 4, 1, 1));
        // Chaque mois reçoit le nombre de semaines du tableau du §4.1.3.
        for h in 0..8760 {
            let date = date_scenario(h);
            assert!(
                date.semaine >= 1 && date.semaine <= SEMAINES_PAR_MOIS[usize::from(date.mois) - 1]
            );
            assert!((1..=7).contains(&date.jour) && (1..=24).contains(&date.heure));
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
