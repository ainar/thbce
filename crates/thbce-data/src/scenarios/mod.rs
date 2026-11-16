// SPDX-License-Identifier: GPL-3.0-or-later
//! Scénarios conventionnels d'occupation et d'usage (Th-BCE 2020 §2.1, §4.1 et §15).
//!
//! Source : classeur officiel du 29 avril 2026
//! (`docs/reference/scenarios/2026-04-29_scenarios_conventionnels.xlsx`), converti par
//! `cargo run -p xtask -- scenarios` dans `table.rs`. Rien n'est saisi à la main.
//!
//! Un scénario combine une grille hebdomadaire `p(j, h)` (7 jours × 24 heures, en heure légale)
//! et une grille annuelle `p(m, s)` (12 mois × 5 semaines) ; la façon de les combiner est
//! décrite par la fiche `C_EIN_Scénarios conventionnels` (§4.1.3), implémentée dans
//! `thbce-core`.

mod table;

pub use table::{GRILLES_ANNUELLES, GRILLES_HEBDO, USAGES};

/// Grille hebdomadaire : `[jour − 1][heure − 1]`, jour 1 = lundi, heures de 1 à 24.
pub type GrilleHebdo = [[f64; 24]; 7];
/// Grille annuelle : `[semaine − 1][mois − 1]`, semaines de 1 à 5, mois de 1 à 12. La valeur est
/// 0 pour une semaine que le mois ne compte pas.
pub type GrilleAnnuelle = [[f64; 12]; 5];

/// Nombre de semaines de chaque mois de l'année conventionnelle (§4.1.3).
pub const SEMAINES_PAR_MOIS: [u8; 12] = [4, 4, 5, 4, 5, 4, 4, 5, 4, 4, 5, 4];

/// Un scénario : indices de ses deux grilles dans les tables dédoublonnées.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scenario {
    /// Indice dans [`GRILLES_HEBDO`].
    pub hebdo: u16,
    /// Indice dans [`GRILLES_ANNUELLES`].
    pub annuel: u16,
}

impl Scenario {
    /// Grille hebdomadaire `p(j, h)`.
    pub fn grille_hebdo(&self) -> &'static GrilleHebdo {
        &GRILLES_HEBDO[self.hebdo as usize]
    }

    /// Grille annuelle `p(m, s)`.
    pub fn grille_annuelle(&self) -> &'static GrilleAnnuelle {
        &GRILLES_ANNUELLES[self.annuel as usize]
    }
}

/// Températures de consigne de la zone, en °C : occupation normale, arrêt de moins de 48 h,
/// arrêt de plus de 48 h, en chauffage (`ch`) et en refroidissement (`fr`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Consignes {
    /// « θii_ch+ » : chauffage, occupation normale.
    pub ch_normal_c: f64,
    /// « θii_fr+ » : refroidissement, occupation normale.
    pub fr_normal_c: f64,
    /// Chauffage, réduit de moins de 48 h.
    pub ch_arret_moins_48h_c: f64,
    /// Refroidissement, réduit de moins de 48 h.
    pub fr_arret_moins_48h_c: f64,
    /// Chauffage, réduit de plus de 48 h.
    pub ch_arret_plus_48h_c: f64,
    /// Refroidissement, réduit de plus de 48 h.
    pub fr_arret_plus_48h_c: f64,
}

/// Besoins d'eau chaude sanitaire de la zone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BesoinsEcs {
    /// Unité à laquelle se rapporte le volume, telle qu'écrite dans le classeur (« m² Sref »,
    /// « Nadeq* »…).
    pub unite: &'static str,
    /// Volume d'eau à 40 °C puisé par semaine et par unité, en litres. `None` quand le classeur
    /// renvoie à la fiche `C_EIN_Besoins_ECS` (usages d'habitation).
    pub volume_l_semaine_unite: Option<f64>,
    /// Clé de répartition horaire et facteur correctif de la semaine.
    pub scenario: Scenario,
}

/// Apports internes dus aux occupants d'un local.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApportsOccupants {
    /// Occupation nominale par unité de surface utile. `None` quand le classeur renvoie au
    /// nombre d'adultes équivalents (usages d'habitation).
    pub nombre_nominal: Option<f64>,
    /// Unité de l'occupation nominale, telle qu'écrite dans le classeur (« Nadeq », « Noccnom »).
    pub unite: &'static str,
    /// Chaleur dégagée par occupant, en W.
    pub chaleur_w: f64,
    /// Humidité dégagée par occupant, en kg/h.
    pub humidite_kg_h: f64,
    /// Taux d'occupation et facteur correctif de la semaine.
    pub scenario: Scenario,
}

/// Apports internes hors occupants et éclairage d'un local (chaleur ou humidité).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApportsEquipements {
    /// Unité à laquelle se rapporte la valeur, telle qu'écrite dans le classeur (« m² »…).
    pub unite: &'static str,
    /// Apport nominal par unité : en W pour la chaleur, en kg/h pour l'humidité.
    pub valeur_par_unite: f64,
    /// Ratio des apports nominaux et ratio correctif de la semaine.
    pub scenario: Scenario,
}

/// Scénarios d'un type de local.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScenariosLocal {
    /// Nom du local.
    pub nom: &'static str,
    /// « Rat_l » : ratio par défaut de la surface utile du local sur celle du groupe.
    pub rat_l: f64,
    /// Occupants.
    pub occupants: ApportsOccupants,
    /// Apports de chaleur hors occupants et éclairage.
    pub chaleur: ApportsEquipements,
    /// Apports d'humidité hors occupants et éclairage.
    pub humidite: ApportsEquipements,
}

/// Scénarios conventionnels d'un usage de zone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScenariosUsage {
    /// Numéro d'usage, de 1 à 28 (§4.1.3, tableau 4).
    pub numero: u8,
    /// Nom de l'onglet du classeur.
    pub onglet: &'static str,
    /// Nom de la zone, tel qu'écrit dans le classeur.
    pub nom: &'static str,
    /// Températures de consigne.
    pub consignes: Consignes,
    /// Mobilité (1 : mobilité ; 0 : immobilité).
    pub mobilite: Scenario,
    /// Occupation (1 : occupation ; 0 : inoccupation).
    pub occupation: Scenario,
    /// Consigne de chauffage (1 : normale ; 0 : réduit de moins de 48 h ; −1 : réduit de plus de
    /// 48 h).
    pub chauffage: Scenario,
    /// Consigne de refroidissement, mêmes codes.
    pub refroidissement: Scenario,
    /// Ventilation (1 : fonctionnement ; 0 : arrêt ou valeur minimale).
    pub ventilation: Scenario,
    /// Éclairage (1 : fonctionnement ; 0 : arrêt ou valeur minimale).
    pub eclairage: Scenario,
    /// Besoins d'eau chaude sanitaire.
    pub ecs: BesoinsEcs,
    /// Locaux de la zone.
    pub locaux: &'static [ScenariosLocal],
}

/// Scénarios de l'usage de numéro donné (1 à 28).
pub fn usage(numero: u8) -> Option<&'static ScenariosUsage> {
    USAGES.get(usize::from(numero).checked_sub(1)?)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn valeurs(s: Scenario) -> impl Iterator<Item = f64> {
        s.grille_hebdo()
            .iter()
            .flatten()
            .copied()
            .chain(s.grille_annuelle().iter().flatten().copied())
    }

    #[test]
    fn les_28_usages_sont_presents_et_numerotes() {
        assert_eq!(USAGES.len(), 28);
        for (i, u) in USAGES.iter().enumerate() {
            assert_eq!(usize::from(u.numero), i + 1);
            assert!(u.nom.starts_with(&format!("{}.", u.numero)), "{}", u.nom);
            assert_eq!(usage(u.numero), Some(u));
            assert!(!u.locaux.is_empty());
        }
        assert_eq!(usage(0), None);
        assert_eq!(usage(29), None);
    }

    #[test]
    fn les_grilles_respectent_leurs_plages() {
        for u in &USAGES {
            // Tous usages : indicateurs de zone entre −1 et 1, taux et ratios entre 0 et 1.
            for s in [
                u.occupation,
                u.ventilation,
                u.eclairage,
                u.chauffage,
                u.refroidissement,
                u.mobilite,
            ] {
                assert!(
                    valeurs(s).all(|v| (-1.0..=1.0).contains(&v)),
                    "{}",
                    u.onglet
                );
            }
            assert!(
                u.ecs
                    .scenario
                    .grille_hebdo()
                    .iter()
                    .flatten()
                    .all(|v| (0.0..=1.0).contains(v)),
                "{}",
                u.onglet
            );
            for l in u.locaux {
                for s in [
                    l.occupants.scenario,
                    l.chaleur.scenario,
                    l.humidite.scenario,
                ] {
                    assert!(
                        valeurs(s).all(|v| (0.0..=1.0).contains(&v)),
                        "{} / {}",
                        u.onglet,
                        l.nom
                    );
                }
                assert!((0.0..=1.0).contains(&l.rat_l));
            }
        }
        // Usages d'habitation (périmètre de la première version) : les indicateurs de zone sont
        // bien des booléens (§4.1.2), et les consignes valent 1 (normal), 0 (réduit de moins de
        // 48 h) ou −1 (réduit de plus de 48 h). D'autres usages portent des valeurs
        // fractionnaires (question Q27).
        for u in [usage(1).unwrap(), usage(2).unwrap()] {
            for s in [u.occupation, u.ventilation, u.eclairage] {
                assert!(valeurs(s).all(|v| v == 0.0 || v == 1.0), "{}", u.onglet);
            }
            for s in [u.chauffage, u.refroidissement] {
                assert!(
                    valeurs(s).all(|v| v == -1.0 || v == 0.0 || v == 1.0),
                    "{}",
                    u.onglet
                );
            }
        }
    }

    #[test]
    fn une_semaine_inexistante_vaut_zero() {
        for g in &GRILLES_ANNUELLES {
            for (mois, semaines) in SEMAINES_PAR_MOIS.iter().enumerate() {
                if *semaines == 4 {
                    assert_eq!(g[4][mois], 0.0);
                }
            }
        }
    }

    /// Recoupement avec le tableau de synthèse de l'annexe III (§2.1.1, p. 24).
    #[test]
    fn maison_individuelle() {
        let u = usage(1).unwrap();
        assert_eq!(u.onglet, "MI");
        let c = u.consignes;
        assert_eq!(
            (c.ch_normal_c, c.ch_arret_moins_48h_c, c.ch_arret_plus_48h_c),
            (19.0, 16.0, 16.0)
        );
        assert_eq!(
            (c.fr_normal_c, c.fr_arret_moins_48h_c, c.fr_arret_plus_48h_c),
            (26.0, 30.0, 30.0)
        );
        // Occupation : lundi 0 h–9 h et 17 h–0 h ; mercredi 0 h–9 h et 13 h–0 h ; week-end 24 h/24.
        // La case d'indice k couvre l'heure légale de k h à k + 1 h.
        let occ = u.occupation.grille_hebdo();
        let heures_occupees =
            |jour: usize| (0..24).filter(|h| occ[jour][*h] == 1.0).collect::<Vec<_>>();
        let semaine: Vec<usize> = (0..9).chain(17..24).collect();
        let mercredi: Vec<usize> = (0..9).chain(13..24).collect();
        assert_eq!(heures_occupees(0), semaine);
        assert_eq!(heures_occupees(1), semaine);
        assert_eq!(heures_occupees(2), mercredi);
        assert_eq!(heures_occupees(3), semaine);
        assert_eq!(heures_occupees(4), semaine);
        assert_eq!(heures_occupees(5), (0..24).collect::<Vec<_>>());
        assert_eq!(heures_occupees(6), (0..24).collect::<Vec<_>>());
        // « Inoccupé 1 semaine en décembre » : une seule semaine à 0, en décembre.
        let annuel = u.occupation.grille_annuelle();
        let inoccupees: Vec<(usize, usize)> = (0..12)
            .flat_map(|m| (0..usize::from(SEMAINES_PAR_MOIS[m])).map(move |s| (m, s)))
            .filter(|(m, s)| annuel[*s][*m] == 0.0)
            .collect();
        assert_eq!(inoccupees.len(), 1);
        assert_eq!(inoccupees[0].0, 11);
        // Éclairage : lundi 6 h–9 h et 17 h–22 h.
        let ecl = u.eclairage.grille_hebdo();
        let allume: Vec<usize> = (0..24).filter(|h| ecl[0][*h] == 1.0).collect();
        assert_eq!(allume, (6..9).chain(17..22).collect::<Vec<_>>());
        // Ventilation 24 h/24, 7 j/7, 52 semaines par an.
        assert!(u
            .ventilation
            .grille_hebdo()
            .iter()
            .flatten()
            .all(|v| *v == 1.0));
        // Local unique, 5,7 W/m² en occupation et 1,14 W/m² hors occupation (ratio 0,2).
        assert_eq!(u.locaux.len(), 1);
        let l = &u.locaux[0];
        assert_eq!((l.nom, l.rat_l), ("Maison individuelle", 1.0));
        assert_eq!(l.chaleur.valeur_par_unite, 5.7);
        let ratios = l.chaleur.scenario.grille_hebdo();
        assert!((l.chaleur.valeur_par_unite * ratios[0][12] - 1.14).abs() < 1e-12);
        assert_eq!(ratios[0][7], 1.0);
        // Occupants : 90 W et 0,055 kg/h par adulte équivalent ; nombre renvoyé à la fiche ECS.
        assert_eq!(
            (
                l.occupants.chaleur_w,
                l.occupants.humidite_kg_h,
                l.occupants.nombre_nominal
            ),
            (90.0, 0.055, None)
        );
        assert_eq!(l.humidite.valeur_par_unite, 0.0);
        assert_eq!(u.ecs.volume_l_semaine_unite, None);
    }

    /// Recoupement avec le tableau de synthèse de l'annexe III (§2.1.1, p. 24).
    #[test]
    fn logement_collectif_et_bureaux() {
        let lc = usage(2).unwrap();
        let locaux: Vec<(&str, f64)> = lc.locaux.iter().map(|l| (l.nom, l.rat_l)).collect();
        assert_eq!(locaux.len(), 2);
        assert_eq!((locaux[0].1, locaux[1].1), (0.9, 0.1));
        assert_eq!(lc.locaux[0].chaleur.valeur_par_unite, 5.7);
        assert_eq!(lc.locaux[1].chaleur.valeur_par_unite, 0.0);
        assert_eq!(lc.consignes, usage(1).unwrap().consignes);

        let bur = usage(3).unwrap();
        assert_eq!(bur.consignes.ch_arret_plus_48h_c, 7.0);
        assert_eq!(bur.ecs.volume_l_semaine_unite, Some(1.25));
        let b = &bur.locaux[0];
        assert_eq!((b.nom, b.rat_l), ("Bureau standard", 0.6047));
        assert_eq!(
            (b.occupants.nombre_nominal, b.chaleur.valeur_par_unite),
            (Some(0.1), 16.0)
        );
        // Occupation des bureaux : du lundi au vendredi, de 8 h à 18 h.
        let occ = bur.occupation.grille_hebdo();
        assert_eq!(
            (0..24).filter(|h| occ[0][*h] == 1.0).collect::<Vec<_>>(),
            (8..18).collect::<Vec<_>>()
        );
        assert!(occ[5].iter().chain(&occ[6]).all(|v| *v == 0.0));
    }

    #[test]
    fn les_ratios_de_surface_des_locaux_somment_a_un() {
        for u in &USAGES {
            let somme: f64 = u.locaux.iter().map(|l| l.rat_l).sum();
            // Somme exacte en habitation ; à 0,001 près ailleurs (aérogares : 0,999, question Q27).
            let tolerance = if u.numero <= 2 { 1e-12 } else { 1.5e-3 };
            assert!((somme - 1.0).abs() < tolerance, "{} : {somme}", u.onglet);
        }
    }
}
