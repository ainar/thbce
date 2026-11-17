// SPDX-License-Identifier: GPL-3.0-or-later
//! Données conventionnelles de la gestion des protections mobiles.
//!
//! Source : annexe III de l'arrêté du 4 août 2021, fiche `C_BAT_Gestion_protections_mobiles`
//! (§5.9), `docs/reference/` :
//! - nomenclature (§5.9.2, p. 181) : `Topj-1lim_manu_v`, `Topj-1lim_manu_se`,
//!   `Vventlim_manu_se` ;
//! - figure 34 (p. 195 et 196), tableau en texte : `Pocc` et `Pderog` par usage ;
//! - figures 35 et 38 (p. 197 et 200) : matrices conventionnelles de la gestion manuelle pour
//!   la famille « habitation, hôtellerie et hébergement » ;
//! - tableau 34 (p. 218), tableau en texte : valeurs propres au calcul Th-D.
//!
//! Les figures 35 et 38 sont des images dans le texte. La transcription de
//! [`VOLET_HABITATION`] et de [`STORE_ENROULABLE_HABITATION`] a pour source les figures publiées
//! au Journal officiel du 15 août 2021, versionnées dans le dépôt :
//! - figure 35 (volets), page 357 du JO, 986 × 653 pixels, image sans perte :
//!   `docs/reference/arrete/figures/a3_figure35_volets_habitation_jo.png` ;
//! - figure 38 (stores enroulables), page 359 du JO, 780 × 509 pixels :
//!   `docs/reference/arrete/figures/a3_figure38_stores_enroulables_habitation_jo.png`.
//!
//! La transcription a été vérifiée par deux lectures indépendantes de ces images, cellule par
//! cellule, pour les lignes de gestion manuelle (16 novembre 2026, `docs/questions/Q031.md`,
//! décision Q31). Les tests de ce
//! module vérifient en outre la cohérence interne des matrices.
//!
//! Les figures 36 et 39 (famille « bureau », p. 197 et 200) sont transcrites des images du PDF
//! de l'annexe III (623 × 413 pixels, agrandies trois fois), le Journal officiel n'étant pas
//! versionné pour ces figures : une seule lecture, à confirmer (`docs/questions.md`, Q61).
//!
//! Sont transcrites les lignes de gestion manuelle des familles « habitation, hôtellerie et
//! hébergement » et « bureau ». Les matrices des autres familles (figures 37 à 37-3 et 40 à
//! 40-3) ne le sont pas.

/// « Topj-1lim_manu_v\[z\] » et « Topj-1lim_manu_se\[z\] » : limite haute de la température
/// opérative maximale du jour précédent, en gestion manuelle, en °C (§5.9.2, valeur
/// conventionnelle ; rappelée au §5.9.3.6).
pub const TOP_J1_LIM_MANU_C: f64 = 26.5;

/// « Vventlim_manu_se » : vitesse de vent à partir de laquelle un store enroulable extérieur est
/// remonté, en m/s (§5.9.2, valeur conventionnelle).
pub const V_VENT_LIM_MANU_SE_M_S: f64 = 10.0;

/// « Rprot2_occ0 » : ratio de fermeture de la seconde protection mobile pour un éclairement nul
/// (§5.9.3.5, éq. 198 : « = 0 (conventionnellement) »).
pub const R_PROT2_OCC0: f64 = 0.0;

/// « Pocc\[z\] » et « Pderog\[z\] » d'un usage (figure 34).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartsOccupation {
    /// « Pocc » : part des baies en groupe occupé sur lesquelles un occupant peut agir.
    pub p_occ: f64,
    /// « Pderog » : part des baies des locaux occupés en dérogation manuelle.
    pub p_derog: f64,
}

/// Famille d'usages au sens de la gestion des protections mobiles (figure 34).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilleGpm {
    /// « Habitation, hôtellerie et hébergement ».
    Habitation,
    /// « Bureaux ».
    Bureaux,
    /// « Enseignement ».
    Enseignement,
    /// « Restauration ».
    Restauration,
    /// « Hôpitaux (24h/24) ».
    Hopitaux,
    /// « Autre ».
    Autre,
}

/// Famille, `Pocc` et `Pderog` d'un usage (figure 34, p. 195 et 196, tableau en texte). `None`
/// hors des 28 usages.
pub fn famille_et_parts(usage: u8) -> Option<(FamilleGpm, PartsOccupation)> {
    use FamilleGpm as F;
    let (famille, p_occ, p_derog) = match usage {
        1 => (F::Habitation, 0.5, 0.25),
        2 => (F::Habitation, 0.7, 0.25),
        3 => (F::Bureaux, 0.5, 0.25),
        4 | 5 | 7 | 12 => (F::Enseignement, 0.7, 0.4),
        6 => (F::Bureaux, 0.7, 0.4),
        8 | 9 | 19 => (F::Habitation, 0.8, 0.25),
        10 | 11 => (F::Bureaux, 0.8, 0.25),
        13..=16 | 26 | 27 => (F::Restauration, 0.9, 0.1),
        17 | 23 | 24 => (F::Autre, 0.7, 0.1),
        18 | 22 | 25 | 28 => (F::Autre, 0.9, 0.0),
        20 => (F::Hopitaux, 0.8, 0.25),
        21 => (F::Bureaux, 0.5, 0.25),
        _ => return None,
    };
    Some((famille, PartsOccupation { p_occ, p_derog }))
}

/// `Pocc` et `Pderog` d'un usage (figure 34).
pub fn parts_occupation(usage: u8) -> Option<PartsOccupation> {
    famille_et_parts(usage).map(|(_, p)| p)
}

/// Matrices conventionnelles de gestion manuelle d'une famille : volets, puis stores
/// enroulables. `None` pour les familles non transcrites.
pub fn matrices(
    famille: FamilleGpm,
) -> Option<(&'static MatriceManuelle, &'static MatriceManuelle)> {
    match famille {
        FamilleGpm::Habitation => Some((&VOLET_HABITATION, &STORE_ENROULABLE_HABITATION)),
        FamilleGpm::Bureaux => Some((&VOLET_BUREAUX, &STORE_ENROULABLE_BUREAUX)),
        _ => None,
    }
}

/// Valeurs propres au calcul Th-D d'une famille (tableau 34, p. 218), appliquées en période de
/// confort adaptatif quand `Topj-1max ≥ Topj-1lim_manu`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValeursThD {
    /// Plancher du ratio de jour en gestion manuelle motorisée (`Type_GPM = 3`).
    pub r_prot_jour_min_motorisee: f64,
    /// Plancher du ratio de jour pour les autres gestions.
    pub r_prot_jour_min_autre: f64,
    /// Plafond du ratio de nuit des volets (« jour = nuit » : `min(valeur ; valeur Th-BC)`).
    pub r_prot_nuit_max_volet: f64,
    /// Ratio de nuit des stores enroulables (« jour = nuit »).
    pub r_prot_nuit_store_enroulable: f64,
}

/// Tableau 34 pour une famille : colonnes « Résidentiel » et « Bureaux ». `None` pour les
/// colonnes non transcrites.
pub fn valeurs_th_d(famille: FamilleGpm) -> Option<ValeursThD> {
    match famille {
        FamilleGpm::Habitation => Some(ValeursThD {
            r_prot_jour_min_motorisee: TH_D_R_PROT_JOUR_MIN_MOTORISEE,
            r_prot_jour_min_autre: TH_D_R_PROT_JOUR_MIN_AUTRE,
            r_prot_nuit_max_volet: TH_D_R_PROT_NUIT_MAX_VOLET,
            r_prot_nuit_store_enroulable: TH_D_R_PROT_NUIT_STORE_ENROULABLE,
        }),
        // Colonne « Bureaux » : max(80 % ; Th-BC) en gestion motorisée, max(70 % ; Th-BC)
        // sinon ; de nuit, min(50 % ; Th-BC) pour les volets, 50 % pour les stores enroulables.
        FamilleGpm::Bureaux => Some(ValeursThD {
            r_prot_jour_min_motorisee: 0.80,
            r_prot_jour_min_autre: 0.70,
            r_prot_nuit_max_volet: 0.50,
            r_prot_nuit_store_enroulable: 0.50,
        }),
        _ => None,
    }
}

/// Ligne d'une matrice conventionnelle : saison au sens de la gestion des protections mobiles,
/// l'été étant dédoublé selon la température opérative maximale de la veille.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LigneGpm {
    /// « hiver ».
    Hiver = 0,
    /// « mi-saison ».
    MiSaison = 1,
    /// « été », `Topj-1_max < Topj-1_lim_manu`.
    EteFrais = 2,
    /// « été », `Topj-1_lim_manu ≤ Topj-1_max`.
    EteChaud = 3,
}

/// Valeurs d'une ligne en occupation, gestion manuelle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LigneOccupation {
    /// « Rprot0 » : ratio de fermeture pour un éclairement nul, de jour, sans seconde protection.
    pub r_prot0: f64,
    /// « Ecl1 » : éclairement, en lux, à partir duquel la protection est entièrement fermée, de
    /// jour, sans seconde protection.
    pub ecl1_lux: f64,
    /// « Rprot » de la première protection, de jour, en présence d'une seconde protection.
    pub r_prot_avec_pm2: f64,
    /// « Ecl2 » : éclairement, en lux, à partir duquel la seconde protection est entièrement
    /// fermée.
    pub ecl2_lux: f64,
    /// « Rprot » de nuit.
    pub r_prot_nuit: f64,
}

/// Valeurs d'une ligne en inoccupation, gestion manuelle (« idem avec ou sans PM2 »).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LigneInoccupation {
    /// « Rprot » de jour.
    pub r_prot_jour: f64,
    /// « Rprot » de nuit.
    pub r_prot_nuit: f64,
}

/// Matrice conventionnelle de gestion manuelle d'un type de protection, pour une famille
/// d'usages. Les tableaux sont indexés par [`LigneGpm`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatriceManuelle {
    /// Occupation, gestion manuelle non motorisée.
    pub occupation_non_motorisee: [LigneOccupation; 4],
    /// Occupation, gestion manuelle motorisée.
    pub occupation_motorisee: [LigneOccupation; 4],
    /// Inoccupation, gestion manuelle non motorisée.
    pub inoccupation_non_motorisee: [LigneInoccupation; 4],
    /// Inoccupation, gestion manuelle motorisée.
    pub inoccupation_motorisee: [LigneInoccupation; 4],
}

const fn occ(r_prot0: f64, ecl1: f64, r_pm2: f64, ecl2: f64, nuit: f64) -> LigneOccupation {
    LigneOccupation {
        r_prot0,
        ecl1_lux: ecl1,
        r_prot_avec_pm2: r_pm2,
        ecl2_lux: ecl2,
        r_prot_nuit: nuit,
    }
}

const fn inocc(jour: f64, nuit: f64) -> LigneInoccupation {
    LigneInoccupation {
        r_prot_jour: jour,
        r_prot_nuit: nuit,
    }
}

/// Figure 35 (p. 197) : première protection mobile de type volet, famille « habitation,
/// hôtellerie et hébergement », lignes de gestion manuelle
/// (`docs/reference/arrete/figures/a3_figure35_volets_habitation_jo.png`).
#[rustfmt::skip]
pub const VOLET_HABITATION: MatriceManuelle = MatriceManuelle {
    //                             Rprot0  Ecl1       Rprot(PM2)  Ecl2       nuit
    occupation_non_motorisee: [
        occ(0.15, 100_000.0, 0.15, 100_000.0, 0.80), // hiver
        occ(0.15, 100_000.0, 0.15,  80_000.0, 0.80), // mi-saison
        occ(0.20,  80_000.0, 0.20,  30_000.0, 0.80), // été, Topj-1_max < limite
        occ(0.25,  60_000.0, 0.40,  30_000.0, 0.80), // été, limite ≤ Topj-1_max
    ],
    occupation_motorisee: [
        occ(0.10, 100_000.0, 0.10, 100_000.0, 0.90),
        occ(0.10, 100_000.0, 0.10,  60_000.0, 0.90),
        occ(0.20,  60_000.0, 0.20,  20_000.0, 0.90),
        occ(0.30,  40_000.0, 0.40,  20_000.0, 0.90),
    ],
    //                                jour  nuit
    inoccupation_non_motorisee: [
        inocc(0.20, 0.70),
        inocc(0.20, 0.70),
        inocc(0.20, 0.70),
        inocc(0.40, 0.70),
    ],
    inoccupation_motorisee: [
        inocc(0.10, 0.80),
        inocc(0.10, 0.80),
        inocc(0.20, 0.80),
        inocc(0.50, 0.80),
    ],
};

/// Figure 38 (p. 200) : première protection mobile de type store enroulable, famille
/// « habitation, hôtellerie et hébergement », lignes de gestion manuelle
/// (`docs/reference/arrete/figures/a3_figure38_stores_enroulables_habitation_jo.png`).
#[rustfmt::skip]
pub const STORE_ENROULABLE_HABITATION: MatriceManuelle = MatriceManuelle {
    occupation_non_motorisee: [
        occ(0.10, 100_000.0, 0.10, 100_000.0, 0.10),
        occ(0.10,  80_000.0, 0.10,  80_000.0, 0.10),
        occ(0.15,  60_000.0, 0.15,  30_000.0, 0.10),
        occ(0.20,  40_000.0, 0.40,  30_000.0, 0.10),
    ],
    occupation_motorisee: [
        occ(0.10, 100_000.0, 0.10, 100_000.0, 0.10),
        occ(0.10,  60_000.0, 0.10,  60_000.0, 0.10),
        occ(0.20,  50_000.0, 0.20,  20_000.0, 0.10),
        occ(0.30,  30_000.0, 0.50,  20_000.0, 0.10),
    ],
    inoccupation_non_motorisee: [
        inocc(0.10, 0.20),
        inocc(0.20, 0.20),
        inocc(0.30, 0.20),
        inocc(0.40, 0.20),
    ],
    inoccupation_motorisee: [
        inocc(0.10, 0.10),
        inocc(0.20, 0.10),
        inocc(0.30, 0.10),
        inocc(0.60, 0.10),
    ],
};

/// Figure 36 (p. 197) : première protection mobile de type volet, famille « bureau », lignes
/// de gestion manuelle.
#[rustfmt::skip]
pub const VOLET_BUREAUX: MatriceManuelle = MatriceManuelle {
    //                             Rprot0  Ecl1       Rprot(PM2)  Ecl2       nuit
    occupation_non_motorisee: [
        occ(0.10,  80_000.0, 0.10,  60_000.0, 0.30), // hiver
        occ(0.10, 100_000.0, 0.10,  80_000.0, 0.30), // mi-saison
        occ(0.10,  80_000.0, 0.20,  40_000.0, 0.30), // été, Topj-1_max < limite
        occ(0.15,  60_000.0, 0.20,  40_000.0, 0.30), // été, limite ≤ Topj-1_max
    ],
    occupation_motorisee: [
        occ(0.05,  80_000.0, 0.05,  60_000.0, 0.70),
        occ(0.05, 100_000.0, 0.05,  80_000.0, 0.70),
        occ(0.15,  80_000.0, 0.30,  30_000.0, 0.70),
        occ(0.20,  60_000.0, 0.30,  30_000.0, 0.70),
    ],
    //                                jour  nuit
    inoccupation_non_motorisee: [
        inocc(0.10, 0.30),
        inocc(0.10, 0.30),
        inocc(0.20, 0.30),
        inocc(0.30, 0.30),
    ],
    inoccupation_motorisee: [
        inocc(0.10, 0.70),
        inocc(0.10, 0.70),
        inocc(0.30, 0.70),
        inocc(0.40, 0.70),
    ],
};

/// Figure 39 (p. 200) : première protection mobile de type store enroulable, famille
/// « bureau », lignes de gestion manuelle.
#[rustfmt::skip]
pub const STORE_ENROULABLE_BUREAUX: MatriceManuelle = MatriceManuelle {
    occupation_non_motorisee: [
        occ(0.15,  60_000.0, 0.10,  60_000.0, 0.10),
        occ(0.15,  80_000.0, 0.15,  80_000.0, 0.10),
        occ(0.20,  40_000.0, 0.35,  40_000.0, 0.10),
        occ(0.25,  40_000.0, 0.40,  40_000.0, 0.10),
    ],
    occupation_motorisee: [
        occ(0.10,  60_000.0, 0.10,  60_000.0, 0.10),
        occ(0.10,  80_000.0, 0.20,  80_000.0, 0.10),
        occ(0.25,  30_000.0, 0.45,  30_000.0, 0.10),
        occ(0.30,  30_000.0, 0.50,  30_000.0, 0.10),
    ],
    inoccupation_non_motorisee: [
        inocc(0.15, 0.15),
        inocc(0.15, 0.15),
        inocc(0.25, 0.20),
        inocc(0.30, 0.20),
    ],
    inoccupation_motorisee: [
        inocc(0.10, 0.10),
        inocc(0.10, 0.10),
        inocc(0.30, 0.15),
        inocc(0.40, 0.15),
    ],
};

/// Tableau 34 (p. 218), colonne « Résidentiel » : en calcul Th-D, en période de confort
/// adaptatif et quand `Topj-1max ≥ Topj-1lim_manu`, le ratio de jour est porté au moins à cette
/// valeur en gestion manuelle motorisée (`Type_GPM = 3`).
pub const TH_D_R_PROT_JOUR_MIN_MOTORISEE: f64 = 0.80;
/// Tableau 34, colonne « Résidentiel » : idem pour les autres gestions (`Type_GPM ≠ 3`).
pub const TH_D_R_PROT_JOUR_MIN_AUTRE: f64 = 0.70;
/// Tableau 34, colonne « Résidentiel », lignes « jour = nuit » des volets : dans les mêmes
/// conditions, le ratio de nuit est ramené au plus à cette valeur (`min(70 % ; valeur Th-BC)`).
pub const TH_D_R_PROT_NUIT_MAX_VOLET: f64 = 0.70;
/// Tableau 34, colonne « Résidentiel », lignes « jour = nuit » des stores enroulables : dans les
/// mêmes conditions, le ratio de nuit prend cette valeur (50 %).
pub const TH_D_R_PROT_NUIT_STORE_ENROULABLE: f64 = 0.50;

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn lignes(m: &MatriceManuelle) -> impl Iterator<Item = &LigneOccupation> {
        m.occupation_non_motorisee
            .iter()
            .chain(&m.occupation_motorisee)
    }

    #[test]
    fn les_matrices_transcrites_sont_coherentes() {
        for m in [&VOLET_HABITATION, &STORE_ENROULABLE_HABITATION] {
            for l in lignes(m) {
                for r in [l.r_prot0, l.r_prot_avec_pm2, l.r_prot_nuit] {
                    assert!((0.0..=1.0).contains(&r));
                }
                assert!(l.ecl1_lux > 0.0 && l.ecl2_lux > 0.0);
            }
            for t in [&m.occupation_non_motorisee, &m.occupation_motorisee] {
                // Du plus froid au plus chaud : on ferme davantage et à un éclairement plus
                // faible ; les deux lignes d'été partagent le seuil de la seconde protection.
                for w in t.windows(2) {
                    assert!(w[1].r_prot0 >= w[0].r_prot0);
                    assert!(w[1].ecl1_lux <= w[0].ecl1_lux);
                    assert!(w[1].ecl2_lux <= w[0].ecl2_lux);
                }
                assert_eq!(t[2].ecl2_lux, t[3].ecl2_lux);
                // De nuit, la position ne dépend pas de la saison.
                assert!(t.iter().all(|l| l.r_prot_nuit == t[0].r_prot_nuit));
            }
            for t in [&m.inoccupation_non_motorisee, &m.inoccupation_motorisee] {
                for w in t.windows(2) {
                    assert!(w[1].r_prot_jour >= w[0].r_prot_jour);
                }
                assert!(t.iter().all(|l| l.r_prot_nuit == t[0].r_prot_nuit));
            }
        }
    }

    #[test]
    fn matrices_de_la_famille_bureau() {
        for m in [&VOLET_BUREAUX, &STORE_ENROULABLE_BUREAUX] {
            for l in lignes(m) {
                for r in [l.r_prot0, l.r_prot_avec_pm2, l.r_prot_nuit] {
                    assert!((0.0..=1.0).contains(&r));
                }
                assert!(l.ecl1_lux > 0.0 && l.ecl2_lux > 0.0);
            }
            for t in [&m.occupation_non_motorisee, &m.occupation_motorisee] {
                // On ferme davantage du plus froid au plus chaud ; les deux lignes d'été
                // partagent le seuil de la seconde protection ; la mi-saison a les seuils
                // d'éclairement les plus hauts.
                for w in t.windows(2) {
                    assert!(w[1].r_prot0 >= w[0].r_prot0);
                }
                assert_eq!(t[2].ecl2_lux, t[3].ecl2_lux);
                assert!(t.iter().all(|l| l.ecl1_lux <= t[1].ecl1_lux));
                assert!(t.iter().all(|l| l.r_prot_nuit == t[0].r_prot_nuit));
            }
            for t in [&m.inoccupation_non_motorisee, &m.inoccupation_motorisee] {
                for w in t.windows(2) {
                    assert!(w[1].r_prot_jour >= w[0].r_prot_jour);
                    assert!(w[1].r_prot_nuit >= w[0].r_prot_nuit);
                }
            }
        }
        // Quelques cellules, relues sur les figures 36 et 39.
        assert_eq!(
            VOLET_BUREAUX.occupation_non_motorisee[1].ecl1_lux,
            100_000.0
        );
        assert_eq!(VOLET_BUREAUX.occupation_motorisee[3].r_prot0, 0.20);
        assert_eq!(
            STORE_ENROULABLE_BUREAUX.occupation_motorisee[2].r_prot_avec_pm2,
            0.45
        );
        assert_eq!(
            STORE_ENROULABLE_BUREAUX.inoccupation_motorisee[3].r_prot_jour,
            0.40
        );
    }

    #[test]
    fn familles_et_parts_des_28_usages() {
        for usage in 1..=28 {
            let (_, p) = famille_et_parts(usage).unwrap();
            assert!((0.0..=1.0).contains(&p.p_occ) && (0.0..=1.0).contains(&p.p_derog));
        }
        assert!(famille_et_parts(0).is_none() && famille_et_parts(29).is_none());
        let (f, p) = famille_et_parts(3).unwrap();
        assert_eq!((f, p.p_occ, p.p_derog), (FamilleGpm::Bureaux, 0.5, 0.25));
        assert_eq!(famille_et_parts(6).unwrap().0, FamilleGpm::Bureaux);
        assert_eq!(famille_et_parts(20).unwrap().0, FamilleGpm::Hopitaux);
        assert!(matrices(FamilleGpm::Bureaux).is_some());
        assert!(matrices(FamilleGpm::Enseignement).is_none());
        let th_d = valeurs_th_d(FamilleGpm::Bureaux).unwrap();
        assert_eq!(
            (th_d.r_prot_jour_min_motorisee, th_d.r_prot_nuit_max_volet),
            (0.80, 0.50)
        );
    }

    #[test]
    fn parts_d_occupation_des_usages_d_habitation() {
        assert_eq!(parts_occupation(1).map(|p| p.p_occ), Some(0.5));
        assert_eq!(parts_occupation(2).map(|p| p.p_occ), Some(0.7));
        assert_eq!(parts_occupation(3), None);
    }
}
