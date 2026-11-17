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
/// Q22, `docs/questions/Q022.md`).
pub fn case_scenario(h: usize) -> u8 {
    (ihj(h) + ehlo(h) - 1) % 24 + 1
}

/// Heure légale à laquelle la fiche des saisons clôt la journée et décide des autorisations
/// (§4.6.3 : « chaque jour j de simulation, lorsque hleg = 9h »).
pub const HLEG_BASCULE_SAISONS: u8 = 9;

/// Indice du jour légal du pas de temps, à partir de 0.
///
/// Le jour légal commence à minuit en heure légale, c'est-à-dire au pas de temps dont la case
/// de scénario vaut 1 ; l'itération 0 (1 h à 2 h légales) appartient au jour 0. C'est **le**
/// jour du comportement des occupants : jour des scénarios (décision Q28), jour de l'éq. (175)
/// des protections mobiles (« incrémentation de Jour_année », décision Q39) et numéro du jour
/// de la fiche des saisons (« 57e jour de simulation (1er mars au sens des scénarios) »,
/// §4.6.3.1). Le jour de la moyenne glissante de la température extérieure reste celui du
/// fichier météo ([`jour`], « h tel que IHJ = 1 », §3.1.3 et §4.5.3.2.1).
///
/// Les deux changements d'heure ne créent ni ne suppriment de jour : la journée du passage à
/// l'heure d'été compte 23 pas de temps, celle du retour à l'heure d'hiver 25.
pub fn jour_legal(h: usize) -> usize {
    (h + usize::from(ehlo(h))) / 24
}

/// Vrai si le pas de temps `h` est le premier d'un jour légal (minuit légal vient de sonner).
/// Faux à l'itération 0, où la simulation commence.
pub fn nouveau_jour_legal(h: usize) -> bool {
    h > 0 && jour_legal(h) != jour_legal(h - 1)
}

/// Numéro du jour de la fiche des saisons, de 1 à 365, si le pas de temps `h` est celui où
/// elle clôt la journée : `hleg(h) = 9 h`, heure légale de fin du pas de temps.
///
/// Th-BCE 2020 §4.6.3 : les sommes vont « de hleg = 9h (jour j−1) à 9h (jour j) » ; la journée
/// close comprend donc le pas de temps qui se termine à 9 h légales, et la décision s'applique
/// à partir du pas suivant (décision Q39).
pub fn jour_bascule_saisons(h: usize) -> Option<u16> {
    (hleg(h) == HLEG_BASCULE_SAISONS).then(|| (jour_legal(h) + 1) as u16)
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
/// Lectures retenues (`docs/questions/Q028.md`) :
/// - l'heure est la case de scénario du tableau du §4.1.3, et le jour change quand cette case
///   revient à 1, c'est-à-dire à minuit en heure légale (décision Q28). Ce choix est isolé dans
///   cette fonction : c'est le deuxième paramètre à tester, après la lecture du fichier météo
///   (décision Q25), si un cas de référence diverge ;
/// - l'année conventionnelle compte 52 semaines, soit 364 jours ; au-delà, le pas de temps est
///   rattaché à un lundi ajouté à la dernière semaine de décembre (décision Q17).
pub fn date_scenario(h: usize) -> DateScenario {
    let heure = case_scenario(h);
    // Indice du jour légal : l'itération 0 est la case 2 du premier jour ; le jour suivant
    // commence à l'itération dont la case vaut 1.
    let jour_legal = jour_legal(h);
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
    fn jour_legal_et_bascule_des_saisons() {
        // Minuit légal : l'itération 23 (23 h–24 h UTC, 0 h–1 h légales) ouvre le jour 1.
        assert_eq!((jour_legal(0), jour_legal(22), jour_legal(23)), (0, 0, 1));
        assert!(!nouveau_jour_legal(0) && !nouveau_jour_legal(22) && nouveau_jour_legal(23));
        // Le jour légal change exactement quand la case de scénario revient à 1.
        for h in 1..8760 {
            assert_eq!(
                nouveau_jour_legal(h),
                case_scenario(h) == 1,
                "itération {h}"
            );
            assert_eq!(
                nouveau_jour_legal(h),
                date_scenario(h).heure == 1,
                "itération {h}"
            );
        }
        // 365 jours légaux, de 23 pas de temps au passage à l'heure d'été (jour d'indice 79),
        // de 25 au retour à l'heure d'hiver (jour d'indice 293), de 24 sinon ; le premier
        // commence à 1 h légale et n'en compte que 23, et une heure du jour 365 clôt l'année.
        let mut pas = [0_u32; 366];
        for h in 0..8760 {
            pas[jour_legal(h)] += 1;
        }
        assert_eq!((pas[0], pas[79], pas[293], pas[365]), (23, 23, 25, 1));
        assert!(pas
            .iter()
            .enumerate()
            .all(|(j, n)| *n == 24 || [0, 79, 293, 365].contains(&j)));
        // Bascule des saisons : pas de temps qui se termine à 9 h légales, une fois par jour.
        // En hiver, 7 h–8 h UTC (itération 7 le premier jour) ; en été, 6 h–7 h UTC.
        assert_eq!(jour_bascule_saisons(7), Some(1));
        assert_eq!(jour_bascule_saisons(6), None);
        assert_eq!(jour_bascule_saisons(24 + 7), Some(2));
        assert_eq!(jour_bascule_saisons(100 * 24 + 6), Some(101));
        assert_eq!(jour_bascule_saisons(100 * 24 + 7), None);
        let bascules: Vec<u16> = (0..8760).filter_map(jour_bascule_saisons).collect();
        assert_eq!(bascules, (1..=365).collect::<Vec<u16>>());
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
