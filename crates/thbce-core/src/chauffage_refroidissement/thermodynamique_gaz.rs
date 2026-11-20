// SPDX-License-Identifier: GPL-3.0-or-later
//! Générateurs thermodynamiques à absorption au gaz : fiche `C_GEN_THERMODYNAMIQUE_GAZ`
//! (Th-BCE 2020 §8.24), type 504 du tableau 111 ; matrices de performances communes avec les
//! pompes à chaleur à moteur gaz (§8.25, `pac_moteur_gaz.rs`).
//!
//! Une machine assure un seul mode (« Idfougen ») : chauffage (GAHP), refroidissement
//! (refroidisseur de liquide) ou ECS (GAHP, générateur de base d'un ballon). Ses performances
//! à pleine charge — GUE, puissance gaz absorbée et puissance électrique des auxiliaires —
//! sont décrites par trois matrices en fonction des températures amont et aval, construites en
//! début de calcul autour d'une valeur pivot (§8.24.3, corrections de statut et coefficients
//! `Cnn` de chaque technologie), puis interpolées à chaque pas de temps. La charge partielle
//! (§8.24.3.6) distingue le brûleur modulant (éq. 1341 à 1343) des cycles marche-arrêt
//! (éq. 1344 à 1346).
//!
//! Technologies calculées : sources sur air extérieur — GAHP air/eau classique et haute
//! température en chauffage, GAHP air/eau en ECS, refroidisseur air/eau. Les machines sur eau
//! glycolée, sur eau ou sur eau de tour (sources amont des fiches 8.26 et 8.27) sont refusées
//! par la validation. Coefficients relus sur les figures 140 à 145, 155 à 157 et 161 à 163
//! (p. 831 à 842, 859 à 863 et 871 à 876 de l'annexe III). Lectures : `docs/questions/Q135.md`
//! à `Q139.md`.

use super::calculs_generation::{Energie, Poste};
use super::generation::{EntreesAppel, FonctionGenerateur, SortiesAppel};
use super::thermodynamique_elec::{encadrement, Limites};

/// Règle de complément de la colonne du pivot : la ligne `cible` reçoit, si sa case est vide,
/// la valeur de la ligne `source` multipliée par `coef` (« Cnnav »).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegleColonne {
    /// Indice de la ligne (température aval) à compléter.
    pub cible: usize,
    /// Indice de la ligne de départ.
    pub source: usize,
    /// Coefficient « Cnnav ».
    pub coef: f64,
}

/// Case de départ d'une règle de complément d'une ligne.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SourceLigne {
    /// Colonne d'indice donné de la même ligne.
    Colonne(usize),
    /// Case fixe (ligne, colonne) : valeur critique reportée sur toute une zone (§8.24.3.5.1.1.2).
    Case(usize, usize),
}

/// Règle de complément d'une ligne : la colonne `cible` reçoit, si sa case est vide, la valeur
/// de la case `source` multipliée par `coef` (« Cnnam »), pour les lignes `debut..fin`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegleLigne {
    /// Indice de la colonne (température amont) à compléter.
    pub cible: usize,
    /// Case de départ.
    pub source: SourceLigne,
    /// Coefficient « Cnnam ».
    pub coef: f64,
    /// Première ligne concernée.
    pub debut: usize,
    /// Ligne qui suit la dernière concernée.
    pub fin: usize,
}

/// Complément d'une matrice : colonne du pivot d'abord, puis chaque ligne.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Remplissage {
    /// Règles de la colonne du pivot, dans l'ordre du texte.
    pub colonne: &'static [RegleColonne],
    /// Règles des lignes, dans l'ordre du texte.
    pub lignes: &'static [RegleLigne],
}

const fn c(cible: usize, source: usize, coef: f64) -> RegleColonne {
    RegleColonne {
        cible,
        source,
        coef,
    }
}

/// Toutes les lignes.
const TOUTES: usize = usize::MAX;

const fn l(cible: usize, source: usize, coef: f64) -> RegleLigne {
    RegleLigne {
        cible,
        source: SourceLigne::Colonne(source),
        coef,
        debut: 0,
        fin: TOUTES,
    }
}

const fn lr(cible: usize, source: SourceLigne, coef: f64, debut: usize, fin: usize) -> RegleLigne {
    RegleLigne {
        cible,
        source,
        coef,
        debut,
        fin,
    }
}

/// Grille des performances au gaz d'une technologie dans un mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrilleGaz {
    /// « Valθaval » : températures aval des lignes, en °C.
    pub theta_aval_c: &'static [f64],
    /// « Valθamont » : températures amont des colonnes, en °C.
    pub theta_amont_c: &'static [f64],
    /// Indices (ligne, colonne) de la valeur pivot.
    pub pivot: (usize, usize),
    /// Lignes dans l'ordre de priorité de la saisie.
    pub ordre_aval: &'static [usize],
    /// Colonnes dans l'ordre de priorité de la saisie.
    pub ordre_amont: &'static [usize],
    /// Complément du GUE (ou de l'EER).
    pub gue: Remplissage,
    /// Complément de la puissance gaz absorbée.
    pub pabs: Remplissage,
    /// Complément de la puissance des auxiliaires.
    pub paux: Remplissage,
}

/// Puissance des auxiliaires au point pivot déclarée ou par défaut :
/// `max(1,2 × déclarée ; coef_declare × Pabs_pivot)` ou `max(plancher ; coef_defaut ×
/// Pabs_pivot)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegleAuxPivot {
    /// Coefficient minimal de la valeur déclarée, rapporté à la puissance gaz au pivot.
    pub coef_declare: f64,
    /// Plancher de la valeur par défaut, en W.
    pub plancher_defaut_w: f64,
    /// Coefficient de la valeur par défaut, rapporté à la puissance gaz au pivot.
    pub coef_defaut: f64,
}

/// §8.24.3.3.1.3.1 : GAHP en chauffage et en ECS.
pub const AUX_PIVOT_CH_ECS: RegleAuxPivot = RegleAuxPivot {
    coef_declare: 0.0416,
    plancher_defaut_w: 500.0,
    coef_defaut: 0.05,
};

/// §8.24.3.5.1.3.1 : refroidisseur air/eau.
pub const AUX_PIVOT_FR: RegleAuxPivot = RegleAuxPivot {
    coef_declare: 0.0416,
    plancher_defaut_w: 400.0,
    coef_defaut: 0.05,
};

const COL_CH_8_24: [RegleColonne; 3] = [c(1, 2, 1.15), c(0, 1, 1.0), c(3, 2, 0.95)];
const LIGNES_GUE_CH_8_24: [RegleLigne; 5] = [
    l(2, 3, 0.95),
    l(1, 2, 0.85),
    l(0, 1, 0.9),
    // Zone critique (θamont ≥ 7 °C, θaval ≤ 35 °C) : valeur reportée sans coefficient.
    lr(4, SourceLigne::Colonne(3), 1.0, 0, 2),
    l(4, 3, 1.15),
];
const COL_UN_8_24: [RegleColonne; 3] = [c(1, 2, 1.0), c(0, 1, 1.0), c(3, 2, 1.0)];
const LIGNES_PABS_8_24: [RegleLigne; 4] =
    [l(2, 3, 1.06), l(1, 2, 1.03), l(0, 1, 1.0), l(4, 3, 1.0)];
const LIGNES_UN: [RegleLigne; 4] = [l(2, 3, 1.0), l(1, 2, 1.0), l(0, 1, 1.0), l(4, 3, 1.0)];

/// §8.24.3.3.1 — GAHP air / eau classique en chauffage (figures 140 à 142, tableau 218).
pub const CH_AIR_EAU_CLASSIQUE: GrilleGaz = GrilleGaz {
    theta_aval_c: &[30.0, 35.0, 45.0, 50.0],
    theta_amont_c: &[-15.0, -7.0, 2.0, 7.0, 20.0],
    pivot: (2, 3),
    ordre_aval: &[2, 3, 1, 0],
    ordre_amont: &[3, 1, 2, 4, 0],
    gue: Remplissage {
        colonne: &COL_CH_8_24,
        lignes: &LIGNES_GUE_CH_8_24,
    },
    pabs: Remplissage {
        colonne: &COL_UN_8_24,
        lignes: &LIGNES_PABS_8_24,
    },
    paux: Remplissage {
        colonne: &COL_UN_8_24,
        lignes: &LIGNES_UN,
    },
};

/// §8.24.3.3.2 — GAHP air / eau haute température en chauffage (figures 143 à 145,
/// tableau 219).
pub const CH_AIR_EAU_HAUTE_TEMPERATURE: GrilleGaz = GrilleGaz {
    theta_aval_c: &[30.0, 35.0, 45.0, 50.0, 60.0],
    theta_amont_c: &[-15.0, -7.0, 2.0, 7.0, 20.0],
    pivot: (2, 3),
    ordre_aval: &[2, 4, 1, 0, 3],
    ordre_amont: &[3, 1, 2, 4, 0],
    gue: Remplissage {
        colonne: &[c(1, 2, 1.15), c(0, 1, 1.0), c(3, 2, 0.95), c(4, 3, 0.8)],
        lignes: &LIGNES_GUE_CH_8_24,
    },
    pabs: Remplissage {
        colonne: &[c(1, 2, 1.0), c(0, 1, 1.0), c(3, 2, 1.0), c(4, 3, 1.0)],
        lignes: &LIGNES_PABS_8_24,
    },
    paux: Remplissage {
        colonne: &[c(1, 2, 1.0), c(0, 1, 1.0), c(3, 2, 1.0), c(4, 3, 1.0)],
        lignes: &LIGNES_UN,
    },
};

const COL_UN_ECS: [RegleColonne; 4] = [c(3, 4, 1.0), c(2, 3, 1.0), c(1, 2, 1.0), c(0, 1, 1.0)];

/// §8.24.3.4.1 — GAHP air / eau en ECS (figures 155 à 157, tableau 223).
pub const ECS_AIR_EAU: GrilleGaz = GrilleGaz {
    theta_aval_c: &[30.0, 35.0, 45.0, 50.0, 60.0],
    theta_amont_c: &[-15.0, -7.0, 2.0, 7.0, 20.0],
    pivot: (4, 3),
    ordre_aval: &[4, 3, 2, 1, 0],
    ordre_amont: &[3, 1, 2, 4, 0],
    gue: Remplissage {
        colonne: &[c(3, 4, 1.15), c(2, 3, 1.05), c(1, 2, 1.05), c(0, 1, 1.0)],
        lignes: &[
            l(2, 3, 0.9),
            l(1, 2, 0.9),
            l(0, 1, 0.8),
            lr(4, SourceLigne::Colonne(3), 1.0, 0, 2),
            l(4, 3, 1.1),
        ],
    },
    pabs: Remplissage {
        colonne: &COL_UN_ECS,
        lignes: &LIGNES_PABS_8_24,
    },
    paux: Remplissage {
        colonne: &COL_UN_ECS,
        lignes: &LIGNES_UN,
    },
};

const COL_UN_FR: [RegleColonne; 4] = [c(0, 1, 1.0), c(2, 1, 1.0), c(3, 2, 1.0), c(4, 3, 1.0)];
/// Valeur critique du refroidisseur air/eau : θamont = 35 °C, θaval = 15 °C.
const CRITIQUE_FR: SourceLigne = SourceLigne::Case(2, 3);

/// §8.24.3.5.1 — refroidisseur air / eau (figures 161 à 163, tableau 225).
pub const FR_AIR_EAU: GrilleGaz = GrilleGaz {
    theta_aval_c: &[4.0, 9.5, 15.0, 20.5, 26.0],
    theta_amont_c: &[5.0, 15.0, 25.0, 35.0, 45.0],
    pivot: (1, 3),
    ordre_aval: &[1, 3, 2, 4, 0],
    ordre_amont: &[3, 4, 2, 1, 0],
    gue: Remplissage {
        colonne: &[c(0, 1, 0.9), c(2, 1, 1.1), c(3, 2, 1.0), c(4, 2, 1.0)],
        lignes: &[
            lr(2, CRITIQUE_FR, 1.0, 2, TOUTES),
            lr(1, CRITIQUE_FR, 1.0, 2, TOUTES),
            lr(0, CRITIQUE_FR, 1.0, 2, TOUTES),
            lr(2, SourceLigne::Colonne(3), 1.2, 0, 2),
            lr(1, SourceLigne::Colonne(2), 1.15, 0, 2),
            lr(0, SourceLigne::Colonne(1), 1.15, 0, 2),
            l(4, 3, 0.8),
        ],
    },
    pabs: Remplissage {
        colonne: &COL_UN_FR,
        lignes: &[l(2, 3, 1.03), l(1, 2, 1.03), l(0, 1, 1.03), l(4, 3, 0.97)],
    },
    paux: Remplissage {
        colonne: &COL_UN_FR,
        lignes: &[l(2, 3, 0.9), l(1, 2, 0.9), l(0, 1, 0.9), l(4, 3, 1.0)],
    },
};

/// Valeur mesurée, certifiée ou justifiée.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointMesureGaz {
    /// Température aval, en °C.
    pub theta_aval_c: f64,
    /// Température amont, en °C.
    pub theta_amont_c: f64,
    /// GUE ou EER mesuré.
    pub performance: f64,
    /// Puissance gaz absorbée, en W.
    pub pabs_w: f64,
    /// Puissance des auxiliaires, en W.
    pub paux_w: f64,
    /// « ValCOR = 2 » : valeur justifiée.
    pub justifiee: bool,
}

/// Performances saisies pour un mode, puissances en W.
#[derive(Debug, Clone, PartialEq)]
pub enum PerformancesSaisiesGaz {
    /// « Statut_données = 1 ».
    Mesurees(Vec<PointMesureGaz>),
    /// « Statut_données = 2 » : valeur pivot déclarée (`Some`) ou par défaut (`None`), puissance
    /// gaz au pivot et puissance des auxiliaires déclarée (`Some`) ou par défaut.
    Pivot {
        /// GUE déclaré ; `None` : valeur par défaut.
        performance: Option<f64>,
        /// Puissance gaz absorbée au pivot, en W.
        pabs_w: f64,
        /// Puissance des auxiliaires au pivot déclarée, en W ; `None` : valeur par défaut.
        paux_w: Option<f64>,
    },
}

impl PerformancesSaisiesGaz {
    /// Conversion de la saisie (puissances gaz en kW).
    pub fn depuis(p: &crate::entree::PerformancesGaz) -> Self {
        use crate::entree::{PerformancesGaz as P, StatutMesure};
        match p {
            P::Mesurees(points) => Self::Mesurees(
                points
                    .iter()
                    .map(|x| PointMesureGaz {
                        theta_aval_c: x.theta_aval_c,
                        theta_amont_c: x.theta_amont_c,
                        performance: x.performance,
                        pabs_w: 1000.0 * x.pabs_kw,
                        paux_w: x.paux_w,
                        justifiee: x.statut == StatutMesure::Justifie,
                    })
                    .collect(),
            ),
            P::PivotDeclare {
                performance,
                pabs_kw,
                paux_w,
            } => Self::Pivot {
                performance: Some(*performance),
                pabs_w: 1000.0 * pabs_kw,
                paux_w: *paux_w,
            },
            P::PivotDefaut { pabs_kw, paux_w } => Self::Pivot {
                performance: None,
                pabs_w: 1000.0 * pabs_kw,
                paux_w: *paux_w,
            },
        }
    }
}

/// Matrices d'un mode après complément.
#[derive(Debug, Clone, PartialEq)]
pub struct MatricesGaz {
    /// « Valθaval(i) », en °C.
    pub theta_aval_c: Vec<f64>,
    /// « Valθamont(j) », en °C.
    pub theta_amont_c: Vec<f64>,
    /// « ValGUE(i,j) » (ou EER).
    pub gue: Vec<Vec<f64>>,
    /// « ValPabs(i,j) », en W.
    pub pabs_w: Vec<Vec<f64>>,
    /// « ValPaux(i,j) », en W.
    pub paux_w: Vec<Vec<f64>>,
    /// Indices du pivot.
    pub pivot: (usize, usize),
}

fn indice(liste: &[f64], theta: f64) -> Option<usize> {
    liste.iter().position(|t| (t - theta).abs() < 1e-9)
}

fn completer(m: &mut [Vec<f64>], r: &Remplissage, pj: usize) {
    for regle in r.colonne {
        if m[regle.cible][pj] == 0.0 {
            m[regle.cible][pj] = m[regle.source][pj] * regle.coef;
        }
    }
    for i in 0..m.len() {
        for regle in r.lignes {
            if i < regle.debut || i >= regle.fin || m[i][regle.cible] != 0.0 {
                continue;
            }
            let v = match regle.source {
                SourceLigne::Colonne(j) => m[i][j],
                SourceLigne::Case(a, b) => m[a][b],
            };
            m[i][regle.cible] = v * regle.coef;
        }
    }
}

impl GrilleGaz {
    /// Vérifie que des valeurs mesurées couvrent exactement le croisement de deux listes de
    /// températures conformes aux ordres de priorité de la saisie, à commencer par le pivot
    /// (§8.24.3, « la fourniture des valeurs mesurées doit respecter l'ensemble des
    /// configurations amont/aval ») ; renvoie un message d'erreur sinon.
    pub fn verifier_mesures(&self, points: &[PointMesureGaz]) -> Result<(), String> {
        let (n, m) = (self.theta_aval_c.len(), self.theta_amont_c.len());
        let mut lignes = vec![false; n];
        let mut colonnes = vec![false; m];
        let mut cases = vec![vec![false; m]; n];
        for p in points {
            let (Some(i), Some(j)) = (
                indice(self.theta_aval_c, p.theta_aval_c),
                indice(self.theta_amont_c, p.theta_amont_c),
            ) else {
                return Err(format!(
                    "le couple de températures aval {} °C et amont {} °C n'appartient pas à la \
                     matrice de la technologie (aval : {:?} ; amont : {:?})",
                    p.theta_aval_c, p.theta_amont_c, self.theta_aval_c, self.theta_amont_c
                ));
            };
            if cases[i][j] {
                return Err(format!(
                    "valeur saisie deux fois pour les températures aval {} °C et amont {} °C",
                    p.theta_aval_c, p.theta_amont_c
                ));
            }
            if !(p.performance > 0.0 && p.pabs_w > 0.0 && p.paux_w > 0.0) {
                return Err(
                    "les performances, puissances gaz et puissances d'auxiliaires mesurées sont \
                     strictement positives"
                        .to_owned(),
                );
            }
            cases[i][j] = true;
            lignes[i] = true;
            colonnes[j] = true;
        }
        let prefixe = |presentes: &[bool], ordre: &[usize]| {
            let k = presentes.iter().filter(|x| **x).count();
            k > 0 && ordre.iter().take(k).all(|&o| presentes[o])
        };
        if !prefixe(&lignes, self.ordre_aval) || !prefixe(&colonnes, self.ordre_amont) {
            return Err(format!(
                "les températures des valeurs saisies ne respectent pas l'ordre de priorité de \
                 la technologie : températures aval dans l'ordre {:?}, températures amont dans \
                 l'ordre {:?}, à commencer par la valeur pivot",
                self.ordre_aval
                    .iter()
                    .map(|&k| self.theta_aval_c[k])
                    .collect::<Vec<_>>(),
                self.ordre_amont
                    .iter()
                    .map(|&k| self.theta_amont_c[k])
                    .collect::<Vec<_>>(),
            ));
        }
        for (i, li) in lignes.iter().enumerate() {
            for (j, cj) in colonnes.iter().enumerate() {
                if *li && *cj && !cases[i][j] {
                    return Err(format!(
                        "valeur manquante pour les températures aval {} °C et amont {} °C : les \
                         valeurs mesurées couvrent tous les couples des températures saisies",
                        self.theta_aval_c[i], self.theta_amont_c[j]
                    ));
                }
            }
        }
        Ok(())
    }

    /// Construit les matrices d'un mode : correction des valeurs saisies selon leur statut
    /// (§8.24.3.3.1.1.1 et 8.24.3.3.1.3.1 ; §8.25.3.1, tableau 230), puis complément par les
    /// coefficients de la technologie (§8.24.3.3.1.1.2, 8.24.3.3.1.2.1 et 8.24.3.3.1.3.2).
    ///
    /// `val_util_max` : valeur utile maximale du GUE pivot ; `aux` : règle des auxiliaires au
    /// pivot déclarés ou par défaut.
    pub fn construire(
        &self,
        saisie: &PerformancesSaisiesGaz,
        val_util_max: f64,
        aux: RegleAuxPivot,
    ) -> MatricesGaz {
        let (n, m) = (self.theta_aval_c.len(), self.theta_amont_c.len());
        let mut gue = vec![vec![0.0; m]; n];
        let mut pabs = vec![vec![0.0; m]; n];
        let mut paux = vec![vec![0.0; m]; n];
        let (pi, pj) = self.pivot;
        match saisie {
            PerformancesSaisiesGaz::Mesurees(points) => {
                for p in points {
                    if let (Some(i), Some(j)) = (
                        indice(self.theta_aval_c, p.theta_aval_c),
                        indice(self.theta_amont_c, p.theta_amont_c),
                    ) {
                        // GUE justifié × 0,9 ; auxiliaires justifiés × 1,1 ; puissance gaz
                        // mesurée sans modification.
                        let (k_gue, k_aux) = if p.justifiee { (0.9, 1.1) } else { (1.0, 1.0) };
                        gue[i][j] = k_gue * p.performance;
                        pabs[i][j] = p.pabs_w;
                        paux[i][j] = k_aux * p.paux_w;
                    }
                }
            }
            PerformancesSaisiesGaz::Pivot {
                performance,
                pabs_w,
                paux_w,
            } => {
                gue[pi][pj] = match performance {
                    Some(v) => (0.8 * v).min(val_util_max),
                    None => 0.8 * val_util_max,
                };
                pabs[pi][pj] = *pabs_w;
                paux[pi][pj] = match paux_w {
                    Some(v) => (1.2 * v).max(aux.coef_declare * pabs_w),
                    None => aux.plancher_defaut_w.max(aux.coef_defaut * pabs_w),
                };
            }
        }
        completer(&mut gue, &self.gue, pj);
        completer(&mut pabs, &self.pabs, pj);
        completer(&mut paux, &self.paux, pj);
        MatricesGaz {
            theta_aval_c: self.theta_aval_c.to_vec(),
            theta_amont_c: self.theta_amont_c.to_vec(),
            gue,
            pabs_w: pabs,
            paux_w: paux,
            pivot: self.pivot,
        }
    }
}

impl MatricesGaz {
    /// `(GUE_pc, Pgazcons_pc, Paux_pc)` aux températures du pas de temps, par interpolation
    /// linéaire (§8.24.3.3.1.1.3, 8.24.3.3.1.2.2 et 8.24.3.3.1.3.3), en W pour les puissances.
    pub fn pleine_charge(&self, theta_amont_c: f64, theta_aval_c: f64) -> (f64, f64, f64) {
        let (j1, j2, c_am) = encadrement(&self.theta_amont_c, theta_amont_c);
        let (i1, i2, c_av) = encadrement(&self.theta_aval_c, theta_aval_c);
        let interp = |m: &[Vec<f64>]| {
            (1.0 - c_am) * (1.0 - c_av) * m[i1][j1]
                + c_am * (1.0 - c_av) * m[i1][j2]
                + c_av * (1.0 - c_am) * m[i2][j1]
                + c_am * c_av * m[i2][j2]
        };
        (
            interp(&self.gue),
            interp(&self.pabs_w),
            interp(&self.paux_w),
        )
    }

    /// GUE au point pivot.
    pub fn gue_pivot(&self) -> f64 {
        self.gue[self.pivot.0][self.pivot.1]
    }

    /// Puissance gaz absorbée au point pivot, en W.
    pub fn pabs_pivot_w(&self) -> f64 {
        self.pabs_w[self.pivot.0][self.pivot.1]
    }

    /// Puissance fournie au point pivot, en W.
    pub fn pfou_pivot_w(&self) -> f64 {
        self.pabs_pivot_w() * self.gue_pivot()
    }
}

/// « Deq » des machines air/eau (tableau 227), en minutes : chauffage, refroidissement, ECS.
pub fn deq_air_eau_min(poste: Poste) -> f64 {
    match poste {
        Poste::Chauffage | Poste::Refroidissement => 0.5,
        Poste::Ecs => 2.1,
    }
}

/// Lecture de la formule par défaut des pertes à l'arrêt en chauffage (éq. 1365, Q136).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LecturePertes {
    /// Lecture retenue : interpolation linéaire entre 20 et 400 kW,
    /// `900 + (5500 − 900)/(400 − 20) × (Pnom − 20)`, continue aux bornes.
    #[default]
    Continue,
    /// Variante littérale : `… × (Pnom − 10)`, discontinue à 20 kW.
    Litterale,
}

/// Valeurs par défaut des caractéristiques de charge partielle (§8.24.3.6.4.1).
pub mod defauts {
    use super::{LecturePertes, Poste};

    /// « LRcontmin » par défaut (éq. 1362).
    pub fn lr_contmin(poste: Poste) -> f64 {
        if poste == Poste::Ecs {
            0.65
        } else {
            0.70
        }
    }

    /// « CcpLRcontmin » par défaut (éq. 1363).
    pub fn ccp_lr_contmin(poste: Poste) -> f64 {
        if poste == Poste::Ecs {
            0.75
        } else {
            0.90
        }
    }

    /// « Paux0 » par défaut, en W, pour la puissance nominale `pnom_kw` (éq. 1364).
    pub fn p_aux_0_w(poste: Poste, pnom_kw: f64) -> f64 {
        let (pmax, vmax) = if poste == Poste::Refroidissement {
            (200.0, 600.0)
        } else {
            (400.0, 300.0)
        };
        if pnom_kw <= 20.0 {
            30.0
        } else if pnom_kw >= pmax {
            vmax
        } else {
            30.0 + (vmax - 30.0) / (pmax - 20.0) * (pnom_kw - 20.0)
        }
    }

    /// « Pertes40°C » par défaut, en W (éq. 1365) ; en ECS, celles du chauffage × 1,1.
    pub fn pertes_40c_w(poste: Poste, pnom_kw: f64, lecture: LecturePertes) -> f64 {
        let chauffage = |p: f64| {
            if p <= 20.0 {
                900.0
            } else if p >= 400.0 {
                5500.0
            } else {
                let origine = match lecture {
                    LecturePertes::Continue => 20.0,
                    LecturePertes::Litterale => 10.0,
                };
                900.0 + (5500.0 - 900.0) / (400.0 - 20.0) * (p - origine)
            }
        };
        match poste {
            Poste::Chauffage => chauffage(pnom_kw),
            Poste::Ecs => 1.1 * chauffage(pnom_kw),
            Poste::Refroidissement => {
                if pnom_kw <= 10.0 {
                    900.0
                } else if pnom_kw >= 200.0 {
                    5500.0
                } else {
                    900.0 + (5500.0 - 900.0) / (200.0 - 10.0) * (pnom_kw - 10.0)
                }
            }
        }
    }

    /// « Rdtcomb » par défaut (éq. 1366) ; `theta_aval_max_c` : « Valθaval(Nθaval) ».
    pub fn rdt_comb(poste: Poste, echangeur: bool, theta_aval_max_c: f64) -> f64 {
        match (poste, echangeur) {
            (Poste::Refroidissement, _) | (_, false) => 0.80,
            (Poste::Chauffage, true) => 1.10 - 0.003 * (theta_aval_max_c + 5.0),
            (Poste::Ecs, true) => 0.92,
        }
    }

    /// « Rdtcomb » d'une valeur pivot justifiée (éq. 1361).
    pub fn rdt_comb_justifie(poste: Poste, echangeur: bool, mesure: f64) -> f64 {
        match (poste, echangeur) {
            (Poste::Refroidissement, _) | (Poste::Chauffage, false) | (Poste::Ecs, true) => {
                0.95 * mesure
            }
            (Poste::Chauffage, true) => 1.1 * mesure - 0.12,
            (Poste::Ecs, false) => 0.90 * mesure,
        }
    }
}

/// Machine à absorption au gaz (§8.24) : un mode, `Rdim` machines identiques.
#[derive(Debug, Clone, PartialEq)]
pub struct MachineAbsorption {
    /// « Idfougen » : poste unique assuré.
    pub poste: Poste,
    /// Matrices de performances.
    pub matrice: MatricesGaz,
    /// « Lim_Theta ».
    pub limites: Limites,
    /// « Fonc_brûl = 1 » : brûleur modulant.
    pub modulant: bool,
    /// « LRcontmin » corrigé (1 en tout ou rien).
    pub lr_contmin: f64,
    /// « CcpLRcontmin » corrigé (1 en tout ou rien).
    pub ccp_lr_contmin: f64,
    /// « Deq », en minutes (tableau 227).
    pub deq_min: f64,
    /// « Dfou0 », en minutes (figure 167 ; ECS : Q135).
    pub dfou0_min: f64,
    /// « Paux0 » d'une machine, en W.
    pub p_aux_0_w: f64,
    /// « Pertes40°C » d'une machine, en W.
    pub pertes_40c_w: f64,
    /// « Rdtcomb ».
    pub rdt_comb: f64,
    /// « Rdim ».
    pub rdim: f64,
}

/// Point de fonctionnement d'une machine à un pas de temps (W, ou Wh sur une heure).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PointGaz {
    /// « GUE_pc(h) ».
    pub gue_pc: f64,
    /// « Pgazcons_pc(h) ».
    pub pgaz_pc_w: f64,
    /// « Paux_pc(h) ».
    pub paux_pc_w: f64,
    /// « Pfou_pc_brut(h) ».
    pub pfou_pc_brut_w: f64,
    /// « Pfou_pc(h) » : limites de fonctionnement comprises.
    pub pfou_pc_w: f64,
    /// « Pfou_LR(h) ».
    pub pfou_lr_w: f64,
    /// « LR » (et « τcharge », éq. 1348).
    pub lr: f64,
    /// « Pgaz_LR ».
    pub pgaz_lr_w: f64,
    /// « Pgazma_LR ».
    pub pgazma_lr_w: f64,
    /// « Paux » à la charge du pas de temps.
    pub paux_w: f64,
    /// « Qrest_act ».
    pub q_rest_act_wh: f64,
}

impl PointGaz {
    /// « Pgazcons_LR » (éq. 1340).
    pub fn pgazcons_lr_w(&self) -> f64 {
        self.pgaz_lr_w + self.pgazma_lr_w
    }

    /// « GUE_LR » (éq. 1350 à 1352) ; nul sans consommation de gaz.
    pub fn gue_lr(&self) -> f64 {
        let p = self.pgazcons_lr_w();
        if p > 0.0 {
            self.pfou_lr_w / p
        } else {
            0.0
        }
    }
}

impl MachineAbsorption {
    /// « idfougen ».
    pub fn fonction(&self) -> FonctionGenerateur {
        FonctionGenerateur::mono(self.poste)
    }

    /// « Rdim × Pngen » : puissance fournie au point pivot, en W.
    pub fn pn_gen_tot_w(&self) -> f64 {
        self.rdim * self.matrice.pfou_pivot_w()
    }

    /// Point de fonctionnement d'une machine (§8.24.3.3 à 8.24.3.6).
    pub fn point(
        &self,
        q_req_act_wh: f64,
        theta_amont_c: f64,
        theta_aval_c: f64,
        theta_amb_c: f64,
    ) -> PointGaz {
        // Pleine charge en conditions non nominales (§8.24.3.3.1.4).
        let (gue_pc, pgaz_pc, paux_pc) = self.matrice.pleine_charge(theta_amont_c, theta_aval_c);
        let brut = pgaz_pc * gue_pc;
        let (pfou_pc, q_rest) = if self.limites.arret(self.poste, theta_amont_c, theta_aval_c) {
            (0.0, q_req_act_wh.max(0.0))
        } else {
            (brut, (q_req_act_wh - brut).max(0.0))
        };
        let pfou_lr = q_req_act_wh.min(pfou_pc).max(0.0);
        // Éq. (1348) : taux de charge rapporté à la puissance brute.
        let lr = if brut > 0.0 { pfou_lr / brut } else { 0.0 };
        let mut p = PointGaz {
            gue_pc,
            pgaz_pc_w: pgaz_pc,
            paux_pc_w: paux_pc,
            pfou_pc_brut_w: brut,
            pfou_pc_w: pfou_pc,
            pfou_lr_w: pfou_lr,
            lr,
            q_rest_act_wh: q_rest,
            ..PointGaz::default()
        };
        if pfou_lr <= 0.0 || gue_pc <= 0.0 {
            // §8.24.3.6.2.4 : charge nulle, auxiliaires à charge nulle seulement.
            p.paux_w = self.p_aux_0_w;
            return p;
        }
        let lr_c = self.lr_contmin;
        if self.modulant && lr >= lr_c && lr_c < 1.0 {
            // §8.24.3.6.2.1, éq. (1341) à (1343) : fonctionnement continu du brûleur.
            let a_cont = gue_pc * (self.ccp_lr_contmin - 1.0) / (1.0 - lr_c);
            let ccp_lr = 1.0 + a_cont / gue_pc * (1.0 - lr);
            p.pgaz_lr_w = pfou_lr / (ccp_lr * gue_pc);
        } else {
            // §8.24.3.6.2.2, éq. (1344) à (1346) : cycles marche-arrêt du brûleur ; la marche
            // correspond au taux LRcontmin.
            let pgazcons_contmin = brut * lr_c / (self.ccp_lr_contmin * gue_pc);
            let lr_rel = lr / lr_c;
            p.pgaz_lr_w = pgazcons_contmin * lr_rel;
            if lr_rel < 1.0 {
                let inv_tcyc = lr_rel * (1.0 - lr_rel) / self.dfou0_min;
                let pertes = self.pertes_40c_w * (60.0 - theta_amb_c) / 40.0;
                p.pgazma_lr_w = pgazcons_contmin * self.deq_min * inv_tcyc
                    + pertes / self.rdt_comb * (1.0 - lr_rel);
            }
        }
        // §8.24.3.6.2.3 : auxiliaires à charge partielle.
        let lr_rel = lr / lr_c;
        p.paux_w = paux_pc.min(self.p_aux_0_w + (paux_pc - self.p_aux_0_w) * lr_rel);
        p
    }

    /// Fonction d'appel (§8.24.3.6.4, éq. 1339 et 1347 à 1356).
    pub fn appel(&self, e: &EntreesAppel) -> SortiesAppel {
        let rdim = self.rdim;
        // Éq. (1339).
        let p = self.point(
            e.q_req_wh / rdim,
            e.theta_amont_c,
            e.theta_aval_c,
            e.theta_amb_c,
        );
        let gaz = p.pgazcons_lr_w();
        // Éq. (1355) et (1356).
        let phi_rejet = if self.poste == Poste::Refroidissement {
            gaz + p.pfou_lr_w
        } else {
            (gaz - p.pfou_lr_w).min(0.0)
        };
        SortiesAppel {
            energie: Energie::Gaz,
            // Éq. (1347).
            q_cef_wh: gaz * rdim,
            q_cons_wh: gaz * rdim,
            // Éq. (1349) : auxiliaires en électricité au poste servi.
            q_cef_aux_elec_wh: p.paux_w * rdim,
            w_aux_pro_wh: p.paux_w * rdim,
            // Éq. (1353) et (1354).
            q_fou_wh: p.pfou_lr_w * rdim,
            q_rest_wh: p.q_rest_act_wh * rdim,
            // Éq. (1348).
            tau_charge: p.lr,
            eta_eff: p.gue_lr(),
            phi_rejet_wh: phi_rejet * rdim,
            ..SortiesAppel::default()
        }
    }

    /// Résout une machine saisie et validée ; `None` pour une technologie hors de la version.
    pub fn resoudre(
        s: &crate::entree::AbsorptionGazSaisie,
        rdim: f64,
    ) -> Option<(Self, Vec<crate::resultats::ValeurParDefaut>)> {
        Self::resoudre_avec(s, rdim, LecturePertes::default())
    }

    /// Comme [`MachineAbsorption::resoudre`], avec une lecture de l'éq. (1365) imposée.
    pub fn resoudre_avec(
        s: &crate::entree::AbsorptionGazSaisie,
        rdim: f64,
        lecture: LecturePertes,
    ) -> Option<(Self, Vec<crate::resultats::ValeurParDefaut>)> {
        use crate::entree::{
            CombinaisonLimites, CycleAbsorption, FonctionnementBruleur, ModeAbsorptionGaz as M,
            StatutMesure, TechnologieChauffageGahp as Ch, TechnologieEcsGahp as Ecs,
            TechnologieRefroidissementGahp as Fr,
        };
        use crate::resultats::ValeurParDefaut;
        let mut defauts = Vec::new();
        let mut defaut = |champ: &str, valeur: f64, regle: &str| {
            defauts.push(ValeurParDefaut {
                champ: champ.to_owned(),
                valeur,
                regle: regle.to_owned(),
            });
        };
        // Valeurs utiles maximales (tableau 217, « Val_util_max »).
        let (poste, grille, val_util_max, aux, dfou0) = match s.mode {
            M::Chauffage {
                technologie,
                typo_emetteur,
            } => {
                let g = match technologie {
                    Ch::AirEauClassique => &CH_AIR_EAU_CLASSIQUE,
                    Ch::AirEauHauteTemperature => &CH_AIR_EAU_HAUTE_TEMPERATURE,
                    _ => return None,
                };
                (
                    Poste::Chauffage,
                    g,
                    1.51,
                    AUX_PIVOT_CH_ECS,
                    super::thermodynamique_elec::dfou0_min(typo_emetteur),
                )
            }
            M::Refroidissement {
                technologie,
                cycle,
                typo_emetteur,
            } => {
                if technologie != Fr::AirEau {
                    return None;
                }
                let v = match cycle {
                    CycleAbsorption::SimpleEffetOuGax => 0.7,
                    CycleAbsorption::DoubleEffet => 1.0,
                };
                (
                    Poste::Refroidissement,
                    &FR_AIR_EAU,
                    v,
                    AUX_PIVOT_FR,
                    super::thermodynamique_elec::dfou0_min(typo_emetteur),
                )
            }
            M::Ecs { technologie } => {
                if technologie != Ecs::AirEau {
                    return None;
                }
                // Q135 : « Dfou0 » du mode ECS repris du tableau 216 (fiche 8.23).
                (
                    Poste::Ecs,
                    &ECS_AIR_EAU,
                    1.13,
                    AUX_PIVOT_CH_ECS,
                    super::thermodynamique_elec::DFOU0_ECS_MIN,
                )
            }
        };
        let saisie = PerformancesSaisiesGaz::depuis(&s.performances);
        if let PerformancesSaisiesGaz::Pivot {
            performance,
            paux_w,
            ..
        } = &saisie
        {
            if performance.is_none() {
                defaut(
                    "performances.pivot_defaut",
                    0.8 * val_util_max,
                    "Th-BCE 2020 §8.24.3 : GUE pivot par défaut, 0,8 × Val_util_max de la \
                     technologie",
                );
            }
            if paux_w.is_none() {
                defaut(
                    "performances.paux_w",
                    aux.plancher_defaut_w,
                    "Th-BCE 2020 §8.24.3.3.1.3.1 et §8.24.3.5.1.3.1 : auxiliaires au pivot par \
                     défaut, max(plancher ; 0,05 × Pabs pivot) ; plancher indiqué",
                );
            }
        }
        let matrice = grille.construire(&saisie, val_util_max, aux);
        let modulant = match s.bruleur {
            Some(b) => b == FonctionnementBruleur::Modulant,
            None => {
                defaut(
                    "bruleur",
                    2.0,
                    "Th-BCE 2020 §8.24.3.6.4.1 : brûleur tout ou rien par défaut (lecture Q137)",
                );
                false
            }
        };
        let echangeur = s.echangeur_fumees.unwrap_or_else(|| {
            defaut(
                "echangeur_fumees",
                0.0,
                "Th-BCE 2020 §8.24.3.6.4.1 : pas d'échangeur eau/fumées par défaut",
            );
            false
        });
        let (lr_contmin, ccp_lr_contmin) = if !modulant {
            (1.0, 1.0)
        } else {
            match s.fonctionnement_continu {
                Some(f) => match f.statut {
                    StatutMesure::Certifie => (f.lr_contmin, f.ccp_lr_contmin),
                    // Éq. (1357) et (1358).
                    StatutMesure::Justifie => {
                        ((f.lr_contmin + 0.1).min(1.0), 0.9 * f.ccp_lr_contmin)
                    }
                },
                None => {
                    let v = (defauts::lr_contmin(poste), defauts::ccp_lr_contmin(poste));
                    defaut(
                        "fonctionnement_continu.lr_contmin",
                        v.0,
                        "Th-BCE 2020 §8.24.3.6.4.1, éq. (1362) et (1363) : LRcontmin et \
                         CcpLRcontmin par défaut (0,70 et 0,90 ; 0,65 et 0,75 en ECS)",
                    );
                    v
                }
            }
        };
        // Q136 : « Pnom » des valeurs par défaut, puissance fournie au point pivot, en kW.
        let pnom_kw = matrice.pfou_pivot_w() / 1000.0;
        let corrige = |v: Option<crate::entree::ValeurMesuree>, k: f64| {
            v.map(|x| match x.statut {
                StatutMesure::Certifie => x.valeur,
                StatutMesure::Justifie => k * x.valeur,
            })
        };
        // Éq. (1359) et (1364).
        let p_aux_0_w = corrige(s.p_aux_0, 1.1).unwrap_or_else(|| {
            let v = defauts::p_aux_0_w(poste, pnom_kw);
            defaut(
                "p_aux_0",
                v,
                "Th-BCE 2020 §8.24.3.6.4.1, éq. (1364) : Paux0 par défaut selon la puissance \
                 nominale (lecture Q136)",
            );
            v
        });
        // Éq. (1360) et (1365).
        let pertes_40c_w = corrige(s.pertes_40c, 1.1).unwrap_or_else(|| {
            let v = defauts::pertes_40c_w(poste, pnom_kw, lecture);
            defaut(
                "pertes_40c",
                v,
                "Th-BCE 2020 §8.24.3.6.4.1, éq. (1365) : Pertes40°C par défaut selon la \
                 puissance nominale (lecture Q136)",
            );
            v
        });
        // Éq. (1361) et (1366).
        let rdt_comb = match s.rdt_comb_pivot {
            Some(x) => match x.statut {
                StatutMesure::Certifie => x.valeur,
                StatutMesure::Justifie => defauts::rdt_comb_justifie(poste, echangeur, x.valeur),
            },
            None => {
                let theta_max = grille.theta_aval_c.last().copied().unwrap_or(0.0);
                let v = defauts::rdt_comb(poste, echangeur, theta_max);
                defaut(
                    "rdt_comb_pivot",
                    v,
                    "Th-BCE 2020 §8.24.3.6.4.1, éq. (1366) : rendement de combustion par défaut",
                );
                v
            }
        };
        let limites = match s.limites {
            None => Limites::Aucune,
            Some(x) => match x.combinaison {
                CombinaisonLimites::LUneOuLAutre => Limites::LUneOuLAutre {
                    theta_aval_c: x.theta_aval_c,
                    theta_amont_c: x.theta_amont_c,
                },
                CombinaisonLimites::LUneEtLAutre => Limites::LUneEtLAutre {
                    theta_aval_c: x.theta_aval_c,
                    theta_amont_c: x.theta_amont_c,
                },
            },
        };
        Some((
            Self {
                poste,
                matrice,
                limites,
                modulant,
                lr_contmin,
                ccp_lr_contmin,
                deq_min: deq_air_eau_min(poste),
                dfou0_min: dfou0,
                p_aux_0_w,
                pertes_40c_w,
                rdt_comb,
                rdim,
            },
            defauts,
        ))
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn proche(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9 * b.abs().max(1.0), "{a} ≠ {b}");
    }

    fn pivot(gue: Option<f64>, pabs_w: f64, paux_w: Option<f64>) -> PerformancesSaisiesGaz {
        PerformancesSaisiesGaz::Pivot {
            performance: gue,
            pabs_w,
            paux_w,
        }
    }

    #[test]
    fn matrice_chauffage_classique_par_defaut() {
        // Pivot déclaré 1,6 : min(0,8 × 1,6 ; 1,51) = 1,28 ; Pabs 20 kW ; auxiliaires par défaut
        // max(500 ; 0,05 × 20 000) = 1 000 W.
        let m = CH_AIR_EAU_CLASSIQUE.construire(
            &pivot(Some(1.6), 20_000.0, None),
            1.51,
            AUX_PIVOT_CH_ECS,
        );
        // Lignes : 30, 35, 45, 50 °C ; colonnes : −15, −7, 2, 7, 20 °C.
        proche(m.gue[2][3], 1.28);
        proche(m.gue[1][3], 1.28 * 1.15); // 35 °C
        proche(m.gue[0][3], 1.28 * 1.15); // 30 °C : zone critique
        proche(m.gue[3][3], 1.28 * 0.95); // 50 °C
                                          // Ligne 45 °C : 2 °C, −7 °C, −15 °C, 20 °C.
        proche(m.gue[2][2], 1.28 * 0.95);
        proche(m.gue[2][1], 1.28 * 0.95 * 0.85);
        proche(m.gue[2][0], 1.28 * 0.95 * 0.85 * 0.9);
        proche(m.gue[2][4], 1.28 * 1.15);
        // Zone critique à 20 °C pour 30 et 35 °C : sans coefficient.
        proche(m.gue[0][4], 1.28 * 1.15);
        proche(m.gue[1][4], 1.28 * 1.15);
        // Puissance gaz : 1,06 puis 1,03.
        proche(m.pabs_w[2][2], 20_000.0 * 1.06);
        proche(m.pabs_w[0][1], 20_000.0 * 1.06 * 1.03);
        proche(m.paux_w[0][0], 1000.0);
        // Interpolation : 4,5 °C (moitié de 2 et 7), 47,5 °C (moitié de 45 et 50).
        let (g, pg, pa) = m.pleine_charge(4.5, 47.5);
        let attendu = 0.25 * (1.28 * 0.95 + 1.28 + 1.28 * 0.95 * 0.95 + 1.28 * 0.95);
        proche(g, attendu);
        proche(pg, 0.25 * (20_000.0 * 1.06 * 2.0 + 20_000.0 * 2.0));
        proche(pa, 1000.0);
    }

    #[test]
    fn statuts_et_auxiliaires() {
        // Auxiliaires déclarés 600 W : max(720 ; 0,0416 × 20 000 = 832) = 832 W.
        let m = CH_AIR_EAU_CLASSIQUE.construire(
            &pivot(None, 20_000.0, Some(600.0)),
            1.51,
            AUX_PIVOT_CH_ECS,
        );
        proche(m.gue_pivot(), 0.8 * 1.51);
        proche(m.paux_w[2][3], 832.0);
        // Refroidisseur, défaut : max(400 ; 0,05 × 5 000) = 400 W.
        let m = FR_AIR_EAU.construire(&pivot(None, 5_000.0, None), 0.7, AUX_PIVOT_FR);
        proche(m.paux_w[1][3], 400.0);
        // Mesures : pivot certifié 1,5 et point justifié à 2 °C (1,4 × 0,9).
        let pts = vec![
            PointMesureGaz {
                theta_aval_c: 45.0,
                theta_amont_c: 7.0,
                performance: 1.5,
                pabs_w: 20_000.0,
                paux_w: 700.0,
                justifiee: false,
            },
            PointMesureGaz {
                theta_aval_c: 45.0,
                theta_amont_c: -7.0,
                performance: 1.4,
                pabs_w: 21_000.0,
                paux_w: 700.0,
                justifiee: true,
            },
        ];
        assert!(CH_AIR_EAU_CLASSIQUE.verifier_mesures(&pts).is_ok());
        let m = CH_AIR_EAU_CLASSIQUE.construire(
            &PerformancesSaisiesGaz::Mesurees(pts.clone()),
            1.51,
            AUX_PIVOT_CH_ECS,
        );
        proche(m.gue[2][1], 1.26);
        proche(m.paux_w[2][1], 770.0);
        proche(m.pabs_w[2][1], 21_000.0);
        // −15 °C déduit de −7 °C : 1,26 × 0,9 ; 2 °C déduit du pivot : 1,5 × 0,95.
        proche(m.gue[2][0], 1.26 * 0.9);
        proche(m.gue[2][2], 1.5 * 0.95);
        // Ordre de priorité : −7 °C sans 7 °C refusé.
        assert!(CH_AIR_EAU_CLASSIQUE.verifier_mesures(&pts[1..]).is_err());
    }

    #[test]
    fn matrice_refroidisseur_zone_critique() {
        let m = FR_AIR_EAU.construire(&pivot(Some(0.9), 5_000.0, None), 0.7, AUX_PIVOT_FR);
        // EER pivot min(0,72 ; 0,7) = 0,7. Colonne 35 °C : 4 °C × 0,9 ; 15 °C × 1,1 ; puis
        // 20,5 et 26 °C égaux à 15 °C.
        proche(m.gue[1][3], 0.7);
        proche(m.gue[0][3], 0.63);
        proche(m.gue[2][3], 0.77);
        proche(m.gue[4][3], 0.77);
        // Zone critique (θaval ≥ 15 °C, θamont ≤ 35 °C) : 0,77.
        proche(m.gue[3][0], 0.77);
        // Lignes 4 et 9,5 °C : 25 °C × 1,2 ; 15 °C × 1,15 ; 5 °C × 1,15 ; 45 °C × 0,8.
        proche(m.gue[1][2], 0.84);
        proche(m.gue[1][0], 0.84 * 1.15 * 1.15);
        proche(m.gue[1][4], 0.56);
        proche(m.gue[3][4], 0.77 * 0.8);
        proche(m.pabs_w[1][4], 5_000.0 * 0.97);
        proche(m.paux_w[1][0], 400.0 * 0.9 * 0.9 * 0.9);
    }

    #[test]
    fn matrice_ecs() {
        let m = ECS_AIR_EAU.construire(&pivot(None, 10_000.0, None), 1.13, AUX_PIVOT_CH_ECS);
        let p = 0.8 * 1.13;
        proche(m.gue[4][3], p);
        proche(m.gue[3][3], p * 1.15);
        proche(m.gue[1][3], p * 1.15 * 1.05 * 1.05);
        proche(m.gue[0][3], m.gue[1][3]);
        proche(m.gue[4][4], p * 1.1);
        proche(m.gue[0][4], m.gue[0][3]);
        proche(m.paux_w[4][3], 500.0);
    }

    #[test]
    fn valeurs_par_defaut_de_charge_partielle() {
        proche(defauts::p_aux_0_w(Poste::Chauffage, 20.0), 30.0);
        proche(
            defauts::p_aux_0_w(Poste::Chauffage, 210.0),
            30.0 + 270.0 / 380.0 * 190.0,
        );
        proche(
            defauts::p_aux_0_w(Poste::Refroidissement, 110.0),
            30.0 + 570.0 / 180.0 * 90.0,
        );
        proche(
            defauts::pertes_40c_w(Poste::Chauffage, 39.0, LecturePertes::Continue),
            1130.0,
        );
        proche(
            defauts::pertes_40c_w(Poste::Chauffage, 39.0, LecturePertes::Litterale),
            900.0 + 4600.0 / 380.0 * 29.0,
        );
        proche(
            defauts::pertes_40c_w(Poste::Ecs, 10.0, LecturePertes::Continue),
            990.0,
        );
        proche(
            defauts::pertes_40c_w(Poste::Refroidissement, 29.0, LecturePertes::Continue),
            1360.0,
        );
        proche(defauts::rdt_comb(Poste::Chauffage, true, 50.0), 0.935);
        proche(defauts::rdt_comb(Poste::Ecs, true, 60.0), 0.92);
        proche(
            defauts::rdt_comb_justifie(Poste::Chauffage, true, 0.95),
            0.925,
        );
    }

    fn machine(modulant: bool) -> MachineAbsorption {
        let m = CH_AIR_EAU_CLASSIQUE.construire(
            &pivot(Some(1.6), 20_000.0, Some(600.0)),
            1.51,
            AUX_PIVOT_CH_ECS,
        );
        MachineAbsorption {
            poste: Poste::Chauffage,
            matrice: m,
            limites: Limites::Aucune,
            modulant,
            lr_contmin: if modulant { 0.7 } else { 1.0 },
            ccp_lr_contmin: if modulant { 0.9 } else { 1.0 },
            deq_min: 0.5,
            dfou0_min: 19.0,
            p_aux_0_w: 40.0,
            pertes_40c_w: 1000.0,
            rdt_comb: 0.8,
            rdim: 2.0,
        }
    }

    #[test]
    fn charge_partielle_tout_ou_rien() {
        // Au pivot (7 °C, 45 °C) : GUE 1,28, Pgaz 20 kW, Pfou 25,6 kW, Paux 832 W.
        let m = machine(false);
        let p = m.point(12_800.0, 7.0, 45.0, 20.0);
        proche(p.lr, 0.5);
        // Éq. (1344) : Pgaz = 20 000 × 0,5 ; éq. (1345) : 20 000 × 0,5 × 0,5 × 0,5 / 19 +
        // 1 000 × (60 − 20)/40 / 0,8 × 0,5.
        proche(p.pgaz_lr_w, 10_000.0);
        proche(
            p.pgazma_lr_w,
            20_000.0 * 0.5 * 0.25 / 19.0 + 1000.0 / 0.8 * 0.5,
        );
        proche(p.paux_w, 40.0 + (832.0 - 40.0) * 0.5);
        // Deux machines : demande 25,6 kWh, sortie gaz doublée.
        let s = m.appel(&EntreesAppel {
            fonction: Poste::Chauffage,
            q_req_wh: 25_600.0,
            theta_amont_c: 7.0,
            theta_aval_c: 45.0,
            r_pui_dispo: 1.0,
            ecs_seule: false,
            theta_amb_c: 20.0,
            air_extrait_w_k: None,
        });
        assert_eq!(s.energie, Energie::Gaz);
        proche(s.q_cef_wh, 2.0 * p.pgazcons_lr_w());
        proche(s.q_cef_aux_elec_wh, 2.0 * p.paux_w);
        proche(s.q_fou_wh, 25_600.0);
        proche(s.phi_rejet_wh, 2.0 * (p.pgazcons_lr_w() - 12_800.0));
        // Charge nulle : auxiliaires à charge nulle.
        let p0 = m.point(0.0, 7.0, 45.0, 20.0);
        assert_eq!(p0.pgazcons_lr_w(), 0.0);
        proche(p0.paux_w, 40.0);
    }

    #[test]
    fn charge_partielle_modulant() {
        let m = machine(true);
        // LR = 0,8 ≥ 0,7 : CCP = 1 + (0,9 − 1)(1 − 0,8)/(1 − 0,7) = 1 − 0,2/3.
        let p = m.point(0.8 * 25_600.0, 7.0, 45.0, 20.0);
        let ccp = 1.0 - 0.2 / 3.0;
        proche(p.pgaz_lr_w, 0.8 * 25_600.0 / (ccp * 1.28));
        assert_eq!(p.pgazma_lr_w, 0.0);
        proche(p.paux_w, 832.0);
        // LR = 0,35 : cycles, LRrel = 0,5, Pgazcons_LRcontmin = 25 600 × 0,7/(0,9 × 1,28).
        let p = m.point(0.35 * 25_600.0, 7.0, 45.0, 20.0);
        let contmin = 25_600.0 * 0.7 / (0.9 * 1.28);
        proche(p.pgaz_lr_w, contmin * 0.5);
        proche(p.pgazma_lr_w, contmin * 0.5 * 0.25 / 19.0 + 1250.0 * 0.5);
        // Pleine charge et report.
        let p = m.point(30_000.0, 7.0, 45.0, 20.0);
        proche(p.q_rest_act_wh, 30_000.0 - 25_600.0);
        proche(p.pgaz_lr_w, 20_000.0);
    }

    #[test]
    fn limites_de_fonctionnement() {
        let mut m = machine(false);
        m.limites = Limites::LUneOuLAutre {
            theta_aval_c: 50.0,
            theta_amont_c: -7.0,
        };
        let p = m.point(10_000.0, -10.0, 45.0, 20.0);
        assert_eq!(p.pfou_lr_w, 0.0);
        proche(p.q_rest_act_wh, 10_000.0);
        proche(p.paux_w, 40.0);
    }
}
