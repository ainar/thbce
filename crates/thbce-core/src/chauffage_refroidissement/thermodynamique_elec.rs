// SPDX-License-Identifier: GPL-3.0-or-later
//! Générateurs thermodynamiques à compression électrique : fiche `C_GEN_THERMODYNAMIQUE_Elec`
//! (Th-BCE 2020 §8.23), type 503 du tableau 111.
//!
//! Pour chaque mode qu'il assure (chauffage, refroidissement, ECS), un générateur est décrit
//! par une matrice de performances à pleine charge — COP ou EER et puissance absorbée — en
//! fonction des températures amont et aval (§8.23.3.1). La matrice est construite en début de
//! calcul autour d'une valeur pivot : valeurs certifiées ou justifiées saisies, valeur pivot
//! déclarée ou par défaut, et complément par les coefficients `Cnn` de la technologie
//! (§8.23.3.3 à 8.23.3.5). À chaque pas de temps, les performances à pleine charge sont
//! interpolées aux températures de source (éq. 1283 à 1288), puis corrigées de la charge
//! partielle (éq. 1298 à 1326) selon la régulation du compresseur.
//!
//! Technologies de la première version : pompes à chaleur air extérieur/eau et air
//! extérieur/air recyclé en chauffage, refroidisseurs air extérieur/eau et air extérieur/air
//! recyclé, pompes à chaleur air extérieur/eau, air extrait/eau et air ambiant/eau en ECS. Les
//! tableaux de coefficients ont été relus sur les pages 762 à 767, 782 à 787 et 794 à 797 de
//! l'annexe III. Lectures : `docs/questions/Q047.md`.

use super::calculs_generation::{Energie, Poste};
use super::generation::{EntreesAppel, FonctionGenerateur, SortiesAppel};

/// « Deq » : durée équivalente liée aux irréversibilités, en minutes (§8.23.3.6).
pub const DEQ_MIN: f64 = 0.5;
/// « Dfou0 » en mode ECS, en minutes (tableau 216).
pub const DFOU0_ECS_MIN: f64 = 26.0;
/// « Taux » par défaut en chauffage ou en ECS (éq. 1278).
pub const TAUX_DEFAUT_CH_ECS: f64 = 0.02;
/// « Taux » par défaut en refroidissement (éq. 1279).
pub const TAUX_DEFAUT_FR: f64 = 0.01;
/// « LRcontmin » par défaut (éq. 1280).
pub const LR_CONTMIN_DEFAUT: f64 = 0.4;
/// « CcpLRcontmin » par défaut (éq. 1281).
pub const CCP_LR_CONTMIN_DEFAUT: f64 = 1.0;
/// Puissance nominale au-delà de laquelle le coefficient `Cnnam_COP(−7, 7)` des pompes à chaleur
/// sur air extérieur passe de 0,50 à 0,60, en W (tableaux 151 et 154).
pub const SEUIL_GRANDE_PUISSANCE_W: f64 = 100_000.0;

/// Règle de complément d'une matrice : la case d'indice `cible` reçoit, si elle est vide, la
/// valeur de la case d'indice `source` multipliée par `coef` (coefficients « Cnn »).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Regle {
    /// Indice de la ligne ou de la colonne à compléter.
    pub cible: usize,
    /// Indice de la ligne ou de la colonne de départ.
    pub source: usize,
    /// Coefficient « Cnn ».
    pub coef: f64,
}

const fn r(cible: usize, source: usize, coef: f64) -> Regle {
    Regle {
        cible,
        source,
        coef,
    }
}

/// Grille des performances d'une technologie dans un mode : températures, valeur pivot, ordre
/// de saisie et coefficients de complément (§8.23.3.3 à 8.23.3.5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grille {
    /// « Val_util_max » : valeur utile maximale du COP ou de l'EER.
    pub val_util_max: f64,
    /// Températures aval des lignes, en °C.
    pub theta_aval_c: &'static [f64],
    /// Températures amont des colonnes, en °C.
    pub theta_amont_c: &'static [f64],
    /// Indices (ligne, colonne) de la valeur pivot.
    pub pivot: (usize, usize),
    /// Lignes dans l'ordre de priorité de la saisie (« M_θ_Aval »).
    pub ordre_aval: &'static [usize],
    /// Colonnes dans l'ordre de priorité de la saisie (« M_θ_Amont »).
    pub ordre_amont: &'static [usize],
    /// Complément du COP ou de l'EER sur la colonne du pivot (températures aval).
    pub aval_perf: &'static [Regle],
    /// Complément du COP ou de l'EER sur chaque ligne (températures amont).
    pub amont_perf: &'static [Regle],
    /// Complément de la puissance absorbée sur la colonne du pivot.
    pub aval_pabs: &'static [Regle],
    /// Complément de la puissance absorbée sur chaque ligne.
    pub amont_pabs: &'static [Regle],
    /// Coefficient qui remplace celui de la première règle de `amont_perf` quand la puissance
    /// nominale atteint 100 kW (tableaux 151 et 154).
    pub amont_perf_grande_puissance: Option<f64>,
}

/// §8.23.3.3.1 — PAC air extérieur / eau en chauffage (figure 117, tableaux 150 à 152,
/// éq. 1239 et 1240).
pub const CH_AIR_EXTERIEUR_EAU: Grille = Grille {
    val_util_max: 3.5,
    theta_aval_c: &[23.5, 32.5, 42.5, 51.0, 60.0],
    theta_amont_c: &[-15.0, -7.0, 2.0, 7.0, 20.0],
    pivot: (1, 3),
    ordre_aval: &[1, 2, 3, 0, 4],
    ordre_amont: &[3, 1, 2, 4, 0],
    aval_perf: &[r(0, 1, 1.10), r(2, 1, 0.8), r(3, 2, 0.8), r(4, 3, 0.8)],
    amont_perf: &[r(1, 3, 0.50), r(2, 3, 0.80), r(4, 3, 1.25), r(0, 1, 0.80)],
    aval_pabs: &[r(0, 1, 1.09), r(2, 1, 0.9), r(3, 2, 0.915), r(4, 3, 0.91)],
    amont_pabs: &[r(1, 3, 0.86), r(2, 3, 0.95), r(4, 3, 1.13), r(0, 1, 0.92)],
    amont_perf_grande_puissance: Some(0.60),
};

/// §8.23.3.3.2 — PAC air extérieur / air recyclé en chauffage (figure 118, tableaux 153 à 155,
/// éq. 1241 et 1242).
pub const CH_AIR_EXTERIEUR_AIR_RECYCLE: Grille = Grille {
    val_util_max: 3.5,
    theta_aval_c: &[5.0, 10.0, 15.0, 20.0, 25.0],
    theta_amont_c: &[-15.0, -7.0, 2.0, 7.0, 20.0],
    pivot: (3, 3),
    ordre_aval: &[3, 2, 4, 1, 0],
    ordre_amont: &[3, 1, 2, 4, 0],
    aval_perf: &[r(0, 3, 1.3), r(1, 3, 1.20), r(2, 3, 1.10), r(4, 3, 0.9)],
    amont_perf: &[r(1, 3, 0.50), r(2, 3, 0.80), r(4, 3, 1.25), r(0, 1, 0.80)],
    aval_pabs: &[r(0, 3, 1.15), r(1, 3, 1.10), r(2, 3, 1.05), r(4, 3, 0.95)],
    amont_pabs: &[r(1, 3, 0.86), r(2, 3, 0.95), r(4, 3, 1.13), r(0, 1, 0.92)],
    amont_perf_grande_puissance: Some(0.6),
};

/// Températures aval des matrices en mode ECS, en °C (figures 126 à 128).
const THETA_AVAL_ECS_C: &[f64] = &[5.0, 15.0, 25.0, 35.0, 45.0, 55.0, 65.0];
/// Ordre de saisie des températures aval en ECS (tableaux 177, 180 et 183).
const ORDRE_AVAL_ECS: &[usize] = &[4, 3, 2, 5, 1, 6, 0];
/// Complément du COP sur la colonne du pivot en ECS (éq. 1256 ; tableaux 178, 181 et 184).
const AVAL_COP_ECS: &[Regle] = &[
    r(0, 4, 1.8),
    r(1, 4, 1.6),
    r(2, 4, 1.4),
    r(3, 4, 1.2),
    r(5, 4, 0.8),
    r(6, 4, 0.6),
];
/// Complément de la puissance absorbée sur la colonne du pivot en ECS (tableaux 179, 182 et
/// 185).
const AVAL_PABS_ECS: &[Regle] = &[
    r(0, 4, 1.40),
    r(1, 4, 1.30),
    r(2, 4, 1.20),
    r(3, 4, 1.10),
    r(5, 4, 0.90),
    r(6, 4, 0.80),
];

/// §8.23.3.4.1 — PAC air extérieur / eau en ECS (figure 126, tableaux 177 à 179, éq. 1256 et
/// 1257).
pub const ECS_AIR_EXTERIEUR_EAU: Grille = Grille {
    val_util_max: 2.7,
    theta_aval_c: THETA_AVAL_ECS_C,
    theta_amont_c: &[-7.0, 2.0, 7.0, 20.0, 35.0],
    pivot: (4, 2),
    ordre_aval: ORDRE_AVAL_ECS,
    ordre_amont: &[2, 1, 3, 0, 4],
    aval_perf: AVAL_COP_ECS,
    amont_perf: &[r(0, 2, 0.50), r(1, 2, 0.80), r(3, 2, 1.25), r(4, 2, 1.50)],
    aval_pabs: AVAL_PABS_ECS,
    amont_pabs: &[r(0, 2, 0.86), r(1, 2, 0.95), r(3, 2, 1.13), r(4, 2, 1.28)],
    amont_perf_grande_puissance: None,
};

/// §8.23.3.4.2 — PAC air extrait / eau en ECS (figure 127, tableaux 180 à 182, éq. 1258).
pub const ECS_AIR_EXTRAIT_EAU: Grille = Grille {
    val_util_max: 3.2,
    theta_aval_c: THETA_AVAL_ECS_C,
    theta_amont_c: &[5.0, 10.0, 15.0, 20.0, 25.0, 30.0],
    pivot: (4, 3),
    ordre_aval: ORDRE_AVAL_ECS,
    ordre_amont: &[3, 2, 4, 1, 5, 0],
    aval_perf: AVAL_COP_ECS,
    amont_perf: &[
        r(0, 3, 0.7),
        r(1, 3, 0.8),
        r(2, 3, 0.9),
        r(4, 3, 1.1),
        r(5, 3, 1.2),
    ],
    aval_pabs: AVAL_PABS_ECS,
    amont_pabs: &[
        r(0, 3, 0.85),
        r(1, 3, 0.90),
        r(2, 3, 0.95),
        r(4, 3, 1.05),
        r(5, 3, 1.10),
    ],
    amont_perf_grande_puissance: None,
};

/// §8.23.3.4.3 — PAC air ambiant / eau en ECS (figure 128, tableaux 183 à 185, éq. 1259).
///
/// La valeur pivot est à 15 °C d'air ambiant, comme le montrent la figure 128, l'ordre de
/// saisie du tableau 183, les indices (5, 3) du §8.23.3.4.3.1.1 et les coefficients
/// `Cnnam(θ, 15)` ; la phrase « la valeur pivot […] est la valeur pour θamont = 20 » et le
/// dernier terme de l'éq. (1259), `Cnnam_COP(20, 15)` pour la colonne à 30 °C, sont lus comme
/// des coquilles (Q47).
pub const ECS_AIR_AMBIANT_EAU: Grille = Grille {
    val_util_max: 3.1,
    theta_aval_c: THETA_AVAL_ECS_C,
    theta_amont_c: &[5.0, 10.0, 15.0, 20.0, 25.0, 30.0],
    pivot: (4, 2),
    ordre_aval: ORDRE_AVAL_ECS,
    ordre_amont: &[2, 3, 1, 4, 5, 0],
    aval_perf: AVAL_COP_ECS,
    amont_perf: &[
        r(0, 2, 0.8),
        r(1, 2, 0.9),
        r(3, 2, 1.1),
        r(4, 2, 1.2),
        r(5, 2, 1.3),
    ],
    aval_pabs: AVAL_PABS_ECS,
    amont_pabs: &[
        r(0, 2, 0.90),
        r(1, 2, 0.95),
        r(3, 2, 1.05),
        r(4, 2, 1.10),
        r(5, 2, 1.15),
    ],
    amont_perf_grande_puissance: None,
};

/// Complément de l'EER sur chaque ligne des refroidisseurs sur air extérieur (tableaux 196 et
/// 199, éq. 1264 et 1266).
const AMONT_EER_AIR_EXTERIEUR: &[Regle] = &[r(1, 3, 1.4), r(2, 3, 1.2), r(4, 3, 0.8), r(0, 3, 1.6)];
/// Complément de la puissance absorbée sur chaque ligne des refroidisseurs sur air extérieur
/// (tableaux 197 et 200).
const AMONT_PABS_FR_AIR_EXTERIEUR: &[Regle] =
    &[r(1, 3, 1.2), r(2, 3, 1.1), r(4, 3, 0.9), r(0, 3, 1.3)];

/// §8.23.3.5.1 — refroidisseur air extérieur / eau (figure 132, tableaux 195 à 197, éq. 1263
/// et 1264).
pub const FR_AIR_EXTERIEUR_EAU: Grille = Grille {
    val_util_max: 2.7,
    theta_aval_c: &[4.0, 9.5, 15.0, 20.5, 26.0],
    theta_amont_c: &[5.0, 15.0, 25.0, 35.0, 45.0],
    pivot: (1, 3),
    ordre_aval: &[1, 3, 2, 4, 0],
    ordre_amont: &[3, 2, 1, 0, 4],
    aval_perf: &[r(0, 1, 0.9), r(2, 1, 1.075), r(3, 1, 1.15), r(4, 1, 1.225)],
    amont_perf: AMONT_EER_AIR_EXTERIEUR,
    aval_pabs: &[
        r(0, 1, 0.945),
        r(2, 1, 1.055),
        r(3, 1, 1.11),
        r(4, 1, 1.165),
    ],
    amont_pabs: AMONT_PABS_FR_AIR_EXTERIEUR,
    amont_perf_grande_puissance: None,
};

/// §8.23.3.5.2 — refroidisseur air extérieur / air recyclé (figure 133, tableaux 198 à 200,
/// éq. 1265 et 1266). Les indices « (4,3) » de la deuxième ligne de l'éq. (1265) sont lus
/// « (3,4) » (Q47).
pub const FR_AIR_EXTERIEUR_AIR_RECYCLE: Grille = Grille {
    val_util_max: 2.7,
    theta_aval_c: &[22.0, 27.0, 32.0, 37.0],
    theta_amont_c: &[5.0, 15.0, 25.0, 35.0, 45.0],
    pivot: (1, 3),
    ordre_aval: &[1, 0, 2, 3],
    ordre_amont: &[3, 2, 1, 0, 4],
    aval_perf: &[r(0, 1, 0.9), r(2, 1, 1.075), r(3, 1, 1.15)],
    amont_perf: AMONT_EER_AIR_EXTERIEUR,
    aval_pabs: &[r(0, 1, 0.95), r(2, 1, 1.05), r(3, 1, 1.1)],
    amont_pabs: AMONT_PABS_FR_AIR_EXTERIEUR,
    amont_perf_grande_puissance: None,
};

/// Températures aval des PAC sur eau à eau chaude, en °C (figures 120, 121 et 124).
const THETA_AVAL_CH_EAU_C: &[f64] = &[23.5, 32.5, 42.5, 51.0, 60.0];
/// Ordre de saisie des températures aval de ces PAC (tableaux 159, 162 et 171).
const ORDRE_AVAL_CH_EAU: &[usize] = &[1, 2, 3, 0, 4];
/// Complément du COP sur la colonne du pivot de ces PAC (éq. 1245, 1247, 1253 ; tableaux 160,
/// 163 et 172).
const AVAL_COP_CH_EAU: &[Regle] = &[r(0, 1, 1.1), r(2, 1, 0.8), r(3, 2, 0.8), r(4, 3, 0.8)];
/// Complément de la puissance absorbée sur la colonne du pivot de ces PAC (tableaux 161, 164
/// et 173).
const AVAL_PABS_CH_EAU: &[Regle] = &[r(0, 1, 1.09), r(2, 1, 0.9), r(3, 2, 0.915), r(4, 3, 0.91)];
/// Températures amont des sources sur eau de nappe, en °C (moyenne de l'eau entrant et sortant
/// de l'évaporateur ; figures 120, 122 et 129).
const THETA_AMONT_NAPPE_C: &[f64] = &[3.5, 8.5, 13.5, 18.5];
/// Températures amont des sources sur eau glycolée, en °C (figures 121 et 131).
const THETA_AMONT_GLYCOLEE_C: &[f64] = &[-6.5, -1.5, 3.5, 8.5, 13.5];
/// Températures amont des sources sur sol (températures de bain d'essai), en °C (figures 124,
/// 125 et 130 ; tableau 245).
const THETA_AMONT_SOL_C: &[f64] = &[-4.0, 1.5, 4.0, 6.5];
/// Températures aval des PAC à air recyclé, en °C (figures 122 et 123).
const THETA_AVAL_AIR_RECYCLE_C: &[f64] = &[5.0, 10.0, 15.0, 20.0, 25.0];
/// Ordre de saisie des températures aval de ces PAC (tableaux 165 et 168).
const ORDRE_AVAL_AIR_RECYCLE: &[usize] = &[3, 2, 4, 1, 0];
/// Complément du COP sur la colonne du pivot de ces PAC (éq. 1249 et 1251 ; tableaux 166 et
/// 169). Les indices de l'éq. (1249), « (2,4) » pour la ligne du pivot et « (2,3) » pour celle à
/// 15 °C, sont lus « (4,2) » et « (3,2) » (lecture Q150).
const AVAL_COP_AIR_RECYCLE: &[Regle] = &[r(0, 3, 1.3), r(1, 3, 1.2), r(2, 3, 1.1), r(4, 3, 0.9)];
/// Complément de la puissance absorbée sur la colonne du pivot de ces PAC (tableaux 167 et
/// 170).
const AVAL_PABS_AIR_RECYCLE: &[Regle] =
    &[r(0, 3, 1.15), r(1, 3, 1.10), r(2, 3, 1.05), r(4, 3, 0.95)];

/// §8.23.3.3.4 — PAC eau de nappe / eau en chauffage (figure 120, tableaux 159 à 161,
/// éq. 1245 et 1246).
pub const CH_EAU_DE_NAPPE_EAU: Grille = Grille {
    val_util_max: 4.7,
    theta_aval_c: THETA_AVAL_CH_EAU_C,
    theta_amont_c: THETA_AMONT_NAPPE_C,
    pivot: (1, 1),
    ordre_aval: ORDRE_AVAL_CH_EAU,
    ordre_amont: &[1, 0, 2, 3],
    aval_perf: AVAL_COP_CH_EAU,
    amont_perf: &[r(0, 1, 0.9), r(2, 1, 1.1), r(3, 1, 1.2)],
    aval_pabs: AVAL_PABS_CH_EAU,
    amont_pabs: &[r(0, 1, 0.95), r(2, 1, 1.05), r(3, 1, 1.10)],
    amont_perf_grande_puissance: None,
};

/// §8.23.3.3.5 — PAC eau glycolée / eau en chauffage (figure 121, tableaux 162 à 164,
/// éq. 1247 et 1248).
pub const CH_EAU_GLYCOLEE_EAU: Grille = Grille {
    val_util_max: 3.7,
    theta_aval_c: THETA_AVAL_CH_EAU_C,
    theta_amont_c: THETA_AMONT_GLYCOLEE_C,
    pivot: (1, 1),
    ordre_aval: ORDRE_AVAL_CH_EAU,
    ordre_amont: &[1, 2, 3, 0, 4],
    aval_perf: AVAL_COP_CH_EAU,
    amont_perf: &[r(0, 1, 0.90), r(2, 1, 1.10), r(3, 1, 1.20), r(4, 1, 1.30)],
    aval_pabs: AVAL_PABS_CH_EAU,
    amont_pabs: &[r(0, 1, 0.95), r(2, 1, 1.05), r(3, 1, 1.10), r(4, 1, 1.15)],
    amont_perf_grande_puissance: None,
};

/// §8.23.3.3.6 — PAC eau de nappe / air recyclé en chauffage (figure 122, tableaux 165 à 167,
/// éq. 1249 et 1250).
pub const CH_EAU_DE_NAPPE_AIR_RECYCLE: Grille = Grille {
    val_util_max: 3.5,
    theta_aval_c: THETA_AVAL_AIR_RECYCLE_C,
    theta_amont_c: THETA_AMONT_NAPPE_C,
    pivot: (3, 1),
    ordre_aval: ORDRE_AVAL_AIR_RECYCLE,
    ordre_amont: &[1, 0, 2, 3],
    aval_perf: AVAL_COP_AIR_RECYCLE,
    amont_perf: &[r(0, 1, 0.9), r(2, 1, 1.1), r(3, 1, 1.2)],
    aval_pabs: AVAL_PABS_AIR_RECYCLE,
    amont_pabs: &[r(0, 1, 0.95), r(2, 1, 1.05), r(3, 1, 1.10)],
    amont_perf_grande_puissance: None,
};

/// §8.23.3.3.7 — PAC eau de boucle / air recyclé en chauffage (figure 123, tableaux 168 à
/// 170, éq. 1251 et 1252).
pub const CH_EAU_DE_BOUCLE_AIR_RECYCLE: Grille = Grille {
    val_util_max: 4.0,
    theta_aval_c: THETA_AVAL_AIR_RECYCLE_C,
    theta_amont_c: &[8.5, 13.5, 18.5, 23.5, 28.5],
    pivot: (3, 2),
    ordre_aval: ORDRE_AVAL_AIR_RECYCLE,
    ordre_amont: &[2, 1, 3, 0, 4],
    aval_perf: AVAL_COP_AIR_RECYCLE,
    amont_perf: &[r(0, 2, 0.8), r(1, 2, 0.9), r(3, 2, 1.1), r(4, 2, 1.2)],
    aval_pabs: AVAL_PABS_AIR_RECYCLE,
    amont_pabs: &[r(0, 2, 0.9), r(1, 2, 0.95), r(3, 2, 1.05), r(4, 2, 1.10)],
    amont_perf_grande_puissance: None,
};

/// §8.23.3.3.8 — PAC sol / eau en chauffage (figure 124, tableaux 171 à 173, éq. 1253 et
/// 1254). Le pivot est à 32,5 °C d'eau (figure 124 et indices « (2,3) ») ; « Θaval = 20 » de
/// la phrase du pivot, « −1;5 » des coefficients (lu 1,5 °C) et les priorités aval de la figure
/// (différentes du tableau 171, qui fait foi pour l'ordre de saisie) sont lus comme des
/// coquilles (lecture Q150).
pub const CH_SOL_EAU: Grille = Grille {
    val_util_max: 3.8,
    theta_aval_c: THETA_AVAL_CH_EAU_C,
    theta_amont_c: THETA_AMONT_SOL_C,
    pivot: (1, 2),
    ordre_aval: ORDRE_AVAL_CH_EAU,
    ordre_amont: &[2, 1, 0, 3],
    aval_perf: &[r(0, 1, 1.1), r(2, 1, 0.8), r(3, 2, 0.8), r(4, 3, 0.8)],
    amont_perf: &[r(0, 2, 0.8), r(1, 2, 0.95), r(3, 2, 1.07)],
    aval_pabs: AVAL_PABS_CH_EAU,
    amont_pabs: &[r(0, 2, 1.0), r(1, 2, 1.0), r(3, 2, 1.0)],
    amont_perf_grande_puissance: None,
};

/// §8.23.3.3.9 — PAC sol / sol en chauffage (figure 125, tableaux 174 à 176, éq. 1255) : une
/// seule température aval, 35 °C.
pub const CH_SOL_SOL: Grille = Grille {
    val_util_max: 3.8,
    theta_aval_c: &[35.0],
    theta_amont_c: THETA_AMONT_SOL_C,
    pivot: (0, 2),
    ordre_aval: &[0],
    ordre_amont: &[2, 1, 0, 3],
    aval_perf: &[],
    amont_perf: &[r(0, 2, 0.8), r(1, 2, 0.95), r(3, 2, 1.07)],
    aval_pabs: &[],
    amont_pabs: &[r(0, 2, 1.0), r(1, 2, 1.0), r(3, 2, 1.0)],
    amont_perf_grande_puissance: None,
};

/// §8.23.3.4.4 — PAC eau de nappe / eau en ECS (figure 129, tableaux 186 à 188, éq. 1256 et
/// 1260). « Cnnam_Pabs(18,5 ; 8,5) = 0,90 » est retenu tel qu'imprimé (lecture Q150).
pub const ECS_EAU_DE_NAPPE_EAU: Grille = Grille {
    val_util_max: 3.7,
    theta_aval_c: THETA_AVAL_ECS_C,
    theta_amont_c: THETA_AMONT_NAPPE_C,
    pivot: (4, 1),
    ordre_aval: ORDRE_AVAL_ECS,
    ordre_amont: &[1, 0, 2, 3],
    aval_perf: AVAL_COP_ECS,
    amont_perf: &[r(0, 1, 0.9), r(2, 1, 1.1), r(3, 1, 1.2)],
    aval_pabs: AVAL_PABS_ECS,
    amont_pabs: &[r(0, 1, 0.95), r(2, 1, 1.05), r(3, 1, 0.90)],
    amont_perf_grande_puissance: None,
};

/// §8.23.3.4.5 — PAC sol / eau en ECS (figure 130, tableaux 189 à 191, éq. 1256 et 1261). Les
/// indices du pivot « (5,2) » sont lus « (5,3) », colonne à 4 °C (lecture Q150).
pub const ECS_SOL_EAU: Grille = Grille {
    val_util_max: 3.0,
    theta_aval_c: THETA_AVAL_ECS_C,
    theta_amont_c: THETA_AMONT_SOL_C,
    pivot: (4, 2),
    ordre_aval: ORDRE_AVAL_ECS,
    ordre_amont: &[2, 1, 0, 3],
    aval_perf: AVAL_COP_ECS,
    amont_perf: &[r(0, 2, 0.8), r(1, 2, 0.95), r(3, 2, 1.07)],
    aval_pabs: AVAL_PABS_ECS,
    amont_pabs: &[r(0, 2, 1.0), r(1, 2, 1.0), r(3, 2, 1.0)],
    amont_perf_grande_puissance: None,
};

/// §8.23.3.4.6 — PAC eau glycolée / eau en ECS (figure 131, tableaux 192 à 194, éq. 1256 et
/// 1262).
pub const ECS_EAU_GLYCOLEE_EAU: Grille = Grille {
    val_util_max: 3.7,
    theta_aval_c: THETA_AVAL_ECS_C,
    theta_amont_c: THETA_AMONT_GLYCOLEE_C,
    pivot: (4, 1),
    ordre_aval: ORDRE_AVAL_ECS,
    ordre_amont: &[1, 2, 3, 0, 4],
    aval_perf: AVAL_COP_ECS,
    amont_perf: &[r(0, 1, 0.90), r(2, 1, 1.10), r(3, 1, 1.20), r(4, 1, 1.30)],
    aval_pabs: AVAL_PABS_ECS,
    amont_pabs: &[r(0, 1, 0.95), r(2, 1, 1.05), r(3, 1, 1.10), r(4, 1, 1.15)],
    amont_perf_grande_puissance: None,
};

/// Températures aval des refroidisseurs à eau glacée, en °C (figures 135 et 138).
const THETA_AVAL_FR_EAU_C: &[f64] = &[4.0, 9.5, 15.0, 20.5, 26.0];
/// Températures aval des refroidisseurs à air recyclé, en °C (figures 136 et 137).
const THETA_AVAL_FR_AIR_C: &[f64] = &[22.0, 27.0, 32.0, 37.0];
/// Températures amont des refroidisseurs sur eau, en °C (moyenne de l'eau entrant et sortant
/// du condenseur ; figures 135 et 136).
const THETA_AMONT_FR_EAU_C: &[f64] = &[2.5, 12.5, 22.5, 32.5, 42.5];
/// Températures amont des refroidisseurs sur eau de nappe (ou captage), en °C (figures 137 et
/// 138).
const THETA_AMONT_FR_NAPPE_C: &[f64] = &[7.5, 12.5, 17.5, 22.5];
/// Complément de l'EER sur la colonne du pivot des refroidisseurs à eau glacée (éq. 1269 et
/// 1275 ; tableaux 205 et 214).
const AVAL_EER_FR_EAU: &[Regle] = &[r(0, 1, 0.9), r(2, 1, 1.075), r(3, 1, 1.15), r(4, 1, 1.225)];
/// Complément de la puissance absorbée sur cette colonne (tableaux 206 et 215).
const AVAL_PABS_FR_EAU: &[Regle] = &[
    r(0, 1, 0.945),
    r(2, 1, 1.055),
    r(3, 1, 1.11),
    r(4, 1, 1.165),
];
/// Complément de l'EER sur la colonne du pivot des refroidisseurs à air recyclé (éq. 1271 et
/// 1273 ; tableaux 208 et 211). Les indices « (4,3) » de l'éq. (1271) et « (1,2) », « (2,3) »,
/// « (2,4) » de l'éq. (1273) sont lus comme des lignes de la colonne du pivot (lecture Q150).
const AVAL_EER_FR_AIR: &[Regle] = &[r(0, 1, 0.9), r(2, 1, 1.075), r(3, 1, 1.15)];
/// Complément de la puissance absorbée sur cette colonne (tableaux 209 et 212).
const AVAL_PABS_FR_AIR: &[Regle] = &[r(0, 1, 0.95), r(2, 1, 1.05), r(3, 1, 1.1)];
/// Complément de l'EER sur chaque ligne des refroidisseurs sur eau (tableaux 205 et 208).
const AMONT_EER_FR_EAU: &[Regle] = &[r(0, 3, 1.6), r(1, 3, 1.4), r(2, 3, 1.2), r(4, 3, 0.8)];
/// Complément de la puissance absorbée sur chaque ligne de ces refroidisseurs (tableaux 206 et
/// 209).
const AMONT_PABS_FR_EAU: &[Regle] = &[r(0, 3, 1.30), r(1, 3, 1.20), r(2, 3, 1.10), r(4, 3, 0.90)];
/// Complément de l'EER sur chaque ligne des refroidisseurs sur eau de nappe (tableaux 211 et
/// 214).
const AMONT_EER_FR_NAPPE: &[Regle] = &[r(0, 1, 1.10), r(2, 1, 0.90), r(3, 1, 0.80)];
/// Complément de la puissance absorbée sur chaque ligne de ces refroidisseurs (tableaux 212 et
/// 215).
const AMONT_PABS_FR_NAPPE: &[Regle] = &[r(0, 1, 1.05), r(2, 1, 0.95), r(3, 1, 1.10)];

/// §8.23.3.5.4 — refroidisseur eau / eau ou eau glycolée / eau (figure 135, tableaux 204 à
/// 206, éq. 1269 et 1270).
pub const FR_EAU_EAU: Grille = Grille {
    val_util_max: 3.7,
    theta_aval_c: THETA_AVAL_FR_EAU_C,
    theta_amont_c: THETA_AMONT_FR_EAU_C,
    pivot: (1, 3),
    ordre_aval: &[1, 3, 2, 4, 0],
    ordre_amont: &[3, 2, 1, 0, 4],
    aval_perf: AVAL_EER_FR_EAU,
    amont_perf: AMONT_EER_FR_EAU,
    aval_pabs: AVAL_PABS_FR_EAU,
    amont_pabs: AMONT_PABS_FR_EAU,
    amont_perf_grande_puissance: None,
};

/// §8.23.3.5.5 — refroidisseur eau / air ou eau glycolée / air (figure 136, tableaux 207 à
/// 209, éq. 1271 et 1272). Les mentions « IdFluide_aval = 1 : eau » et « IdFluide_amont = 1 :
/// air » sont lues inversées (lecture Q150).
pub const FR_EAU_AIR: Grille = Grille {
    val_util_max: 2.7,
    theta_aval_c: THETA_AVAL_FR_AIR_C,
    theta_amont_c: THETA_AMONT_FR_EAU_C,
    pivot: (1, 3),
    ordre_aval: &[1, 0, 2, 3],
    ordre_amont: &[3, 2, 1, 0, 4],
    aval_perf: AVAL_EER_FR_AIR,
    amont_perf: AMONT_EER_FR_EAU,
    aval_pabs: AVAL_PABS_FR_AIR,
    amont_pabs: AMONT_PABS_FR_EAU,
    amont_perf_grande_puissance: None,
};

/// §8.23.3.5.6 — refroidisseur eau de nappe (ou captage) / air (figure 137, tableaux 210 à 212,
/// éq. 1273 et 1274).
pub const FR_EAU_DE_NAPPE_AIR_RECYCLE: Grille = Grille {
    val_util_max: 3.7,
    theta_aval_c: THETA_AVAL_FR_AIR_C,
    theta_amont_c: THETA_AMONT_FR_NAPPE_C,
    pivot: (1, 1),
    ordre_aval: &[1, 0, 2, 3],
    ordre_amont: &[1, 2, 0, 3],
    aval_perf: AVAL_EER_FR_AIR,
    amont_perf: AMONT_EER_FR_NAPPE,
    aval_pabs: AVAL_PABS_FR_AIR,
    amont_pabs: AMONT_PABS_FR_NAPPE,
    amont_perf_grande_puissance: None,
};

/// §8.23.3.5.7 — refroidisseur eau de nappe (ou captage) / eau (figure 138, tableaux 213 à 215,
/// éq. 1275 et 1276). Les indices « (2,4) » du pivot de l'éq. (1275) sont lus « (2,2) », comme
/// au §8.23.3.5.7.2 (lecture Q150).
pub const FR_EAU_DE_NAPPE_EAU: Grille = Grille {
    val_util_max: 3.7,
    theta_aval_c: THETA_AVAL_FR_EAU_C,
    theta_amont_c: THETA_AMONT_FR_NAPPE_C,
    pivot: (1, 1),
    ordre_aval: &[1, 3, 2, 4, 0],
    ordre_amont: &[1, 2, 0, 3],
    aval_perf: AVAL_EER_FR_EAU,
    amont_perf: AMONT_EER_FR_NAPPE,
    aval_pabs: AVAL_PABS_FR_EAU,
    amont_pabs: AMONT_PABS_FR_NAPPE,
    amont_perf_grande_puissance: None,
};

/// Valeur de performance mesurée, certifiée ou justifiée (« Statut_données_PC = 1 »).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointMesure {
    /// Température aval de la mesure, en °C : une des lignes de la grille.
    pub theta_aval_c: f64,
    /// Température amont de la mesure, en °C : une des colonnes de la grille.
    pub theta_amont_c: f64,
    /// COP ou EER mesuré.
    pub performance: f64,
    /// Puissance absorbée mesurée, en W.
    pub pabs_w: f64,
    /// « COR(i,j) = 2 » : valeur justifiée, réduite de 10 % (éq. 1238) ; sinon certifiée.
    pub justifiee: bool,
}

/// Performances à pleine charge saisies pour un mode (§8.23.3.1).
#[derive(Debug, Clone, PartialEq)]
pub enum PerformancesSaisies {
    /// « Statut_données_PC = 1 » : valeurs certifiées ou justifiées, dont la valeur pivot.
    Mesurees(Vec<PointMesure>),
    /// « Statut_données_PC = 2 » et « Statut_val_pivot = 1 » : valeur pivot déclarée.
    PivotDeclare {
        /// « Val_COP » ou « Val_EER » : valeur déclarée, avant correction.
        performance: f64,
        /// « Val_Pabs » : puissance absorbée au point pivot, en W.
        pabs_w: f64,
    },
    /// « Statut_données_PC = 2 » et « Statut_val_pivot = 2 » : valeur pivot par défaut.
    PivotDefaut {
        /// « Val_Pabs » : puissance absorbée au point pivot, en W.
        pabs_w: f64,
    },
}

/// Matrices de performances d'un mode après complément (§8.23.3.3 à 8.23.3.5).
#[derive(Debug, Clone, PartialEq)]
pub struct MatricePerformance {
    /// « Valθav(i) », en °C.
    pub theta_aval_c: Vec<f64>,
    /// « Valθam(j) », en °C.
    pub theta_amont_c: Vec<f64>,
    /// « COP_util(i,j) » ou « EER_util(i,j) ».
    pub perf: Vec<Vec<f64>>,
    /// « Pabs(i,j) », en W.
    pub pabs_w: Vec<Vec<f64>>,
    /// Indices du pivot.
    pub pivot: (usize, usize),
}

/// Indice d'une température dans une liste de la grille.
fn indice(liste: &[f64], theta: f64) -> Option<usize> {
    liste.iter().position(|t| (t - theta).abs() < 1e-9)
}

impl Grille {
    /// Vérifie que des valeurs mesurées couvrent exactement le croisement de deux listes de
    /// températures conformes aux ordres de priorité de la saisie (« M_θ_Aval » et
    /// « M_θ_Amont », §8.23.3.3.1.2) ; renvoie un message d'erreur sinon.
    pub fn verifier_mesures(&self, points: &[PointMesure]) -> Result<(), String> {
        let mut lignes = vec![false; self.theta_aval_c.len()];
        let mut colonnes = vec![false; self.theta_amont_c.len()];
        let mut cases = vec![vec![false; colonnes.len()]; lignes.len()];
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
            cases[i][j] = true;
            lignes[i] = true;
            colonnes[j] = true;
            if !(p.performance > 0.0 && p.pabs_w > 0.0) {
                return Err(
                    "Générateur thermodynamique : certaines valeurs de la matrice parmi celles \
                     requises pour le calcul sont nulles"
                        .to_owned(),
                );
            }
        }
        let prefixe = |presentes: &[bool], ordre: &[usize]| {
            let n = presentes.iter().filter(|x| **x).count();
            n > 0 && ordre.iter().take(n).all(|&k| presentes[k])
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
        for (i, l) in lignes.iter().enumerate() {
            for (j, c) in colonnes.iter().enumerate() {
                if *l && *c && !cases[i][j] {
                    return Err(format!(
                        "Générateur thermodynamique : certaines valeurs de la matrice parmi \
                         celles requises pour le calcul sont nulles (températures aval {} °C et \
                         amont {} °C)",
                        self.theta_aval_c[i], self.theta_amont_c[j]
                    ));
                }
            }
        }
        Ok(())
    }

    /// Construit les matrices de performances d'un mode (§8.23.3.3.1.2 et 8.23.3.3.1.3) :
    /// correction des valeurs saisies selon leur statut (éq. 1238), puis complément par les
    /// coefficients de la technologie, d'abord sur la colonne du pivot, puis sur chaque ligne,
    /// en conservant les valeurs déjà définies.
    pub fn construire(&self, saisie: &PerformancesSaisies) -> MatricePerformance {
        let (n, m) = (self.theta_aval_c.len(), self.theta_amont_c.len());
        let mut perf = vec![vec![0.0; m]; n];
        let mut pabs = vec![vec![0.0; m]; n];
        let (pi, pj) = self.pivot;
        match saisie {
            PerformancesSaisies::Mesurees(points) => {
                for p in points {
                    if let (Some(i), Some(j)) = (
                        indice(self.theta_aval_c, p.theta_aval_c),
                        indice(self.theta_amont_c, p.theta_amont_c),
                    ) {
                        // Éq. (1238) : valeur certifiée telle quelle, valeur justifiée réduite
                        // de 10 % ; les puissances absorbées mesurées ne sont pas modifiées.
                        perf[i][j] = if p.justifiee {
                            0.9 * p.performance
                        } else {
                            p.performance
                        };
                        pabs[i][j] = p.pabs_w;
                    }
                }
            }
            PerformancesSaisies::PivotDeclare {
                performance,
                pabs_w,
            } => {
                // Valeur déclarée réduite de 20 % et plafonnée à la valeur utile maximale.
                perf[pi][pj] = (0.8 * performance).min(self.val_util_max);
                pabs[pi][pj] = *pabs_w;
            }
            PerformancesSaisies::PivotDefaut { pabs_w } => {
                perf[pi][pj] = 0.8 * self.val_util_max;
                pabs[pi][pj] = *pabs_w;
            }
        }
        // Tableaux 151 et 154 : coefficient amont selon la puissance nominale au point pivot.
        let grande = perf[pi][pj] * pabs[pi][pj] >= SEUIL_GRANDE_PUISSANCE_W;
        let completer =
            |mat: &mut Vec<Vec<f64>>, aval: &[Regle], amont: &[Regle], gp: Option<f64>| {
                for regle in aval {
                    if mat[regle.cible][pj] == 0.0 {
                        mat[regle.cible][pj] = mat[regle.source][pj] * regle.coef;
                    }
                }
                for ligne in mat.iter_mut() {
                    for (k, regle) in amont.iter().enumerate() {
                        if ligne[regle.cible] == 0.0 {
                            let coef = match gp {
                                Some(c) if k == 0 => c,
                                _ => regle.coef,
                            };
                            ligne[regle.cible] = ligne[regle.source] * coef;
                        }
                    }
                }
            };
        completer(
            &mut perf,
            self.aval_perf,
            self.amont_perf,
            self.amont_perf_grande_puissance.filter(|_| grande),
        );
        completer(&mut pabs, self.aval_pabs, self.amont_pabs, None);
        MatricePerformance {
            theta_aval_c: self.theta_aval_c.to_vec(),
            theta_amont_c: self.theta_amont_c.to_vec(),
            perf,
            pabs_w: pabs,
            pivot: self.pivot,
        }
    }
}

/// Indices et coefficient d'interpolation d'une température dans une liste croissante
/// (éq. 1283 à 1285) : hors de la liste, la valeur de la borne est conservée.
pub(crate) fn encadrement(valeurs: &[f64], theta: f64) -> (usize, usize, f64) {
    let n = valeurs.len();
    if n == 0 {
        return (0, 0, 0.0);
    }
    if theta < valeurs[0] {
        return (0, 0, 0.0);
    }
    if theta > valeurs[n - 1] {
        return (n - 1, n - 1, 1.0);
    }
    for k in 1..n {
        if theta <= valeurs[k] {
            let c = (theta - valeurs[k - 1]) / (valeurs[k] - valeurs[k - 1]);
            return (k - 1, k, c);
        }
    }
    (n - 1, n - 1, 1.0)
}

impl MatricePerformance {
    /// `(Pabs_pc, COP_pc ou EER_pc)` à pleine charge aux températures de source du pas de
    /// temps, par interpolation linéaire (éq. 1286 à 1288), en W et sans dimension.
    pub fn pleine_charge(&self, theta_amont_c: f64, theta_aval_c: f64) -> (f64, f64) {
        let (j1, j2, c_am) = encadrement(&self.theta_amont_c, theta_amont_c);
        let (i1, i2, c_av) = encadrement(&self.theta_aval_c, theta_aval_c);
        let interp = |m: &[Vec<f64>]| {
            (1.0 - c_am) * (1.0 - c_av) * m[i1][j1]
                + c_am * (1.0 - c_av) * m[i1][j2]
                + c_av * (1.0 - c_am) * m[i2][j1]
                + c_am * c_av * m[i2][j2]
        };
        (interp(&self.pabs_w), interp(&self.perf))
    }

    /// Puissance absorbée au point pivot, en W.
    pub fn pabs_pivot_w(&self) -> f64 {
        self.pabs_w[self.pivot.0][self.pivot.1]
    }

    /// Puissance fournie au point pivot, en W.
    pub fn pfou_pivot_w(&self) -> f64 {
        self.pabs_pivot_w() * self.perf[self.pivot.0][self.pivot.1]
    }
}

/// « Lim_Theta » : limites de fonctionnement de la machine dans un mode (§8.23.3.7.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Limites {
    /// 0 : pas de limite.
    Aucune,
    /// 1 : la machine s'arrête si l'une ou l'autre des limites est franchie.
    LUneOuLAutre {
        /// Limite sur la température aval, en °C (maximale en chauffage et en ECS, minimale en
        /// refroidissement).
        theta_aval_c: f64,
        /// Limite sur la température amont, en °C (minimale en chauffage et en ECS, maximale
        /// en refroidissement).
        theta_amont_c: f64,
    },
    /// 2 : la machine s'arrête si les deux limites sont franchies à la fois.
    LUneEtLAutre {
        /// Limite sur la température aval, en °C.
        theta_aval_c: f64,
        /// Limite sur la température amont, en °C.
        theta_amont_c: f64,
    },
}

impl Limites {
    /// Vrai si la machine ne peut pas fonctionner aux températures du pas de temps (éq. 1296
    /// et 1297).
    pub fn arret(&self, poste: Poste, theta_amont_c: f64, theta_aval_c: f64) -> bool {
        let franchies = |aval: f64, amont: f64| {
            if poste == Poste::Refroidissement {
                (theta_aval_c < aval, theta_amont_c > amont)
            } else {
                (theta_aval_c > aval, theta_amont_c < amont)
            }
        };
        match *self {
            Self::Aucune => false,
            Self::LUneOuLAutre {
                theta_aval_c: av,
                theta_amont_c: am,
            } => {
                let (a, b) = franchies(av, am);
                a || b
            }
            Self::LUneEtLAutre {
                theta_aval_c: av,
                theta_amont_c: am,
            } => {
                let (a, b) = franchies(av, am);
                a && b
            }
        }
    }
}

/// « Fonc_compr » : régulation de puissance du compresseur (§8.23.3.7.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FonctionnementCompresseur {
    /// 1 : puissance variable, puis cycles marche-arrêt sous le taux de charge minimal.
    PuissanceVariable,
    /// 2 : tout ou rien, cycles marche-arrêt.
    ToutOuRien,
}

/// Lecture de la puissance absorbée en chauffage ou en refroidissement d'un générateur double
/// service pendant un pas de temps partagé avec la production d'ECS (Q47).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LecturePasPartage {
    /// Lecture retenue, littérale : les éq. (1300) à (1318) sont appliquées avec la puissance
    /// disponible de l'éq. (1293), sans autre pondération.
    #[default]
    Litterale,
    /// Variante : la puissance du compresseur est pondérée par la part du pas de temps
    /// disponible, `1 − Rat_Fonct_ECS`.
    PonderationTemporelle,
}

/// Mode de fonctionnement d'un générateur thermodynamique : chauffage, refroidissement ou ECS.
#[derive(Debug, Clone, PartialEq)]
pub struct ModePac {
    /// Matrices de performances à pleine charge.
    pub matrice: MatricePerformance,
    /// « Lim_Theta » et ses températures.
    pub limites: Limites,
    /// « Dfou0 », en minutes (tableau 216).
    pub dfou0_min: f64,
    /// « LRcontmin », après correction de statut.
    pub lr_contmin: f64,
    /// « CcpLRcontmin », après correction de statut.
    pub ccp_lr_contmin: f64,
    /// « Tair_lim » : température limite de l'air rejeté d'une source sur air extrait, en °C
    /// (éq. 1454 et 1456) ; `None` pour les autres sources.
    pub t_air_lim_c: Option<f64>,
}

/// Générateur thermodynamique à compression électrique (§8.23).
#[derive(Debug, Clone, PartialEq)]
pub struct GenerateurThermodynamique {
    /// Mode chauffage.
    pub chauffage: Option<ModePac>,
    /// Mode refroidissement.
    pub refroidissement: Option<ModePac>,
    /// Mode ECS.
    pub ecs: Option<ModePac>,
    /// « Fonc_compr ».
    pub compresseur: FonctionnementCompresseur,
    /// « Waux,0 » : puissance des auxiliaires à charge nulle d'une machine, en W (éq. 1277).
    pub w_aux_0_w: f64,
    /// « Rdim » : nombre de machines identiques.
    pub rdim: f64,
    /// Lecture des pas de temps partagés avec l'ECS.
    pub lecture_pas_partage: LecturePasPartage,
}

/// Détail du calcul d'une machine pour un mode à un pas de temps (grandeurs d'une machine, en
/// W ou en Wh sur le pas d'une heure).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PointFonctionnement {
    /// « Pabs_pc(h) ».
    pub pabs_pc_w: f64,
    /// « COP_pc(h) » ou « EER_pc(h) ».
    pub perf_pc: f64,
    /// « Pfou_pc(h) » : puissance fournie à pleine charge, limites comprises.
    pub pfou_pc_w: f64,
    /// « Pfou_LR(h) ».
    pub pfou_lr_w: f64,
    /// « LR(h) ».
    pub lr: f64,
    /// « Pcomp_LR(h) ».
    pub pcomp_lr_w: f64,
    /// « Pcompma_LR(h) ».
    pub pcompma_lr_w: f64,
    /// Part de « Waux,0 » comptée dans la puissance absorbée.
    pub w_aux_w: f64,
    /// « Pabs_LR(h) ».
    pub pabs_lr_w: f64,
    /// « Qrest_act(h) ».
    pub q_rest_act_wh: f64,
}

impl PointFonctionnement {
    /// « COP_LR(h) » ou « EER_LR(h) » (éq. 1304, 1311, 1318 et 1323) ; nul sans consommation.
    pub fn perf_lr(&self) -> f64 {
        if self.pabs_lr_w > 0.0 {
            self.pfou_lr_w / self.pabs_lr_w
        } else {
            0.0
        }
    }
}

impl GenerateurThermodynamique {
    /// Mode du générateur pour un poste.
    pub fn mode(&self, poste: Poste) -> Option<&ModePac> {
        match poste {
            Poste::Chauffage => self.chauffage.as_ref(),
            Poste::Refroidissement => self.refroidissement.as_ref(),
            Poste::Ecs => self.ecs.as_ref(),
        }
    }

    /// « idfougen » : fonction du générateur d'après ses modes ; chauffage, refroidissement
    /// et ECS : triple service (tableau 149).
    pub fn fonction(&self) -> FonctionGenerateur {
        match (
            self.chauffage.is_some(),
            self.refroidissement.is_some(),
            self.ecs.is_some(),
        ) {
            (true, true, true) => FonctionGenerateur::ChauffageRefroidissementEcs,
            (true, true, false) => FonctionGenerateur::ChauffageRefroidissement,
            (true, false, true) => FonctionGenerateur::ChauffageEcs,
            (true, false, false) => FonctionGenerateur::Chauffage,
            (false, true, _) => FonctionGenerateur::Refroidissement,
            (false, false, _) => FonctionGenerateur::Ecs,
        }
    }

    /// Vrai pour un générateur double service (chauffage et ECS, §8.23.3.2) ou triple service
    /// (qui suit les mêmes règles pour l'ECS, §8.23.3.7.5 et 8.23.3.7.6).
    pub fn double_service(&self) -> bool {
        self.chauffage.is_some() && self.ecs.is_some()
    }

    /// « Rdim × Pngen » : puissance fournie au point pivot du mode, en W.
    pub fn pn_gen_tot_w(&self, poste: Poste) -> f64 {
        self.mode(poste)
            .map_or(0.0, |m| self.rdim * m.matrice.pfou_pivot_w())
    }

    /// Calcule le point de fonctionnement d'une machine pour un mode (§8.23.3.7).
    ///
    /// `q_req_act_wh` : énergie requise d'une machine (éq. 1295) ; `r_pui_dispo` : part du pas
    /// de temps disponible pour le mode (éq. 1293) ; `ecs_seule` : « iECS_seule(h) » ;
    /// `air_extrait_w_k` : `Qmair_extrait_act × Cpa` d'une source sur air extrait.
    #[allow(clippy::too_many_arguments)]
    pub fn point(
        &self,
        poste: Poste,
        q_req_act_wh: f64,
        theta_amont_c: f64,
        theta_aval_c: f64,
        r_pui_dispo: f64,
        ecs_seule: bool,
        air_extrait_w_k: Option<f64>,
    ) -> PointFonctionnement {
        let Some(m) = self.mode(poste) else {
            return PointFonctionnement {
                q_rest_act_wh: q_req_act_wh,
                ..PointFonctionnement::default()
            };
        };
        let froid = poste == Poste::Refroidissement;
        // Éq. (1286) à (1290) : performances à pleine charge.
        let (pabs_pc, perf_pc) = m.matrice.pleine_charge(theta_amont_c, theta_aval_c);
        let mut pfou_pc_brut = pabs_pc * perf_pc;
        // Éq. (1291), (1292) et (1454) à (1457) : source sur air extrait, puissance limitée par
        // la chaleur échangeable avec l'air.
        if let (Some(w_k), Some(t_lim)) = (air_extrait_w_k, m.t_air_lim_c) {
            let (ecart, denominateur) = if froid {
                (t_lim - theta_amont_c, pfou_pc_brut + pabs_pc)
            } else {
                (theta_amont_c - t_lim, pfou_pc_brut - pabs_pc)
            };
            let pech_max = w_k * ecart.max(0.0);
            if denominateur > 0.0 {
                pfou_pc_brut = pfou_pc_brut.min(pech_max * pfou_pc_brut / denominateur);
            }
        }
        // Éq. (1293) : part du pas de temps laissée par la production d'ECS.
        if poste != Poste::Ecs {
            pfou_pc_brut *= r_pui_dispo;
        }
        // Éq. (1296) et (1297) : limites de fonctionnement.
        let (pfou_pc, q_rest_act) = if m.limites.arret(poste, theta_amont_c, theta_aval_c) {
            (0.0, q_req_act_wh)
        } else {
            (pfou_pc_brut, (q_req_act_wh - pfou_pc_brut).max(0.0))
        };
        // Éq. (1298) et (1299).
        let pfou_lr = q_req_act_wh.min(pfou_pc).max(0.0);
        let lr = if pfou_pc > 0.0 {
            pfou_lr / pfou_pc
        } else {
            0.0
        };
        let mut p = PointFonctionnement {
            pabs_pc_w: pabs_pc,
            perf_pc,
            pfou_pc_w: pfou_pc,
            pfou_lr_w: pfou_lr,
            lr,
            q_rest_act_wh: q_rest_act,
            ..PointFonctionnement::default()
        };
        let w_aux_0 = self.w_aux_0_w;
        if poste == Poste::Ecs && self.double_service() {
            // §8.23.3.7.5, éq. (1319) à (1323) : en double service, l'ECS est produite à pleine
            // charge pendant une fraction du pas de temps, sans correction de charge partielle.
            // Lecture Q47 : « Pabs-LR = Pabs-PC » est lu « LR × Pabs-PC ».
            p.pcomp_lr_w = lr * pabs_pc;
            p.w_aux_w = if ecs_seule { (1.0 - lr) * w_aux_0 } else { 0.0 };
        } else if pfou_lr <= 0.0 {
            // §8.23.3.7.6, éq. (1324) à (1326) : charge nulle ou machine hors de ses limites ;
            // en double service, pendant la part du pas de temps laissée par l'ECS.
            p.w_aux_w = if self.double_service() {
                r_pui_dispo * w_aux_0
            } else {
                w_aux_0
            };
        } else {
            // Éq. (1300) et (1305).
            let pcomp_pc = pabs_pc - w_aux_0;
            let cycles =
                |pcomp_ref: f64, taux: f64| pcomp_ref * DEQ_MIN * taux * (1.0 - taux) / m.dfou0_min;
            match self.compresseur {
                FonctionnementCompresseur::ToutOuRien => {
                    // Éq. (1301) et (1302).
                    p.pcomp_lr_w = pcomp_pc * lr;
                    p.pcompma_lr_w = cycles(pcomp_pc, lr);
                }
                FonctionnementCompresseur::PuissanceVariable => {
                    // Éq. (1306) et (1307).
                    let cop_pc_net = pfou_pc_brut / pcomp_pc;
                    let ccp_net = m.lr_contmin * pcomp_pc * m.ccp_lr_contmin
                        / (m.lr_contmin * pabs_pc - m.ccp_lr_contmin * w_aux_0);
                    if lr >= m.lr_contmin {
                        // Éq. (1308) et (1309) : fonctionnement continu.
                        let cop_lr_net = cop_pc_net
                            * (1.0 + (ccp_net - 1.0) * (1.0 - lr) / (1.0 - m.lr_contmin));
                        p.pcomp_lr_w = pfou_lr / cop_lr_net;
                    } else {
                        // Éq. (1312) à (1316) : cycles marche-arrêt sous le taux minimal.
                        let pfou_contmin = pfou_pc_brut * m.lr_contmin;
                        let pcomp_contmin = pfou_contmin / (cop_pc_net * ccp_net);
                        p.pcomp_lr_w = pcomp_contmin * (1.0 - (m.lr_contmin - lr) / m.lr_contmin);
                        p.pcompma_lr_w = cycles(pcomp_contmin, lr / m.lr_contmin);
                    }
                }
            }
            if self.lecture_pas_partage == LecturePasPartage::PonderationTemporelle
                && poste != Poste::Ecs
            {
                p.pcomp_lr_w *= r_pui_dispo;
                p.pcompma_lr_w *= r_pui_dispo;
            }
            p.w_aux_w = w_aux_0;
        }
        // Éq. (1303), (1310), (1317), (1321) et (1322).
        p.pabs_lr_w = p.pcomp_lr_w + p.pcompma_lr_w + p.w_aux_w;
        p
    }

    /// Fonction d'appel du générateur (§8.23.3.7.7, éq. 1327 à 1338).
    pub fn appel(&self, e: &EntreesAppel) -> SortiesAppel {
        let rdim = self.rdim;
        let p = self.point(
            e.fonction,
            e.q_req_wh / rdim,
            e.theta_amont_c,
            e.theta_aval_c,
            e.r_pui_dispo,
            e.ecs_seule,
            e.air_extrait_w_k.map(|w| w / rdim),
        );
        // Éq. (1332) à (1334) : énergie échangée avec la source amont.
        let compresseur = p.pcomp_lr_w + p.pcompma_lr_w;
        let phi_rejet = if e.fonction == Poste::Refroidissement {
            compresseur + p.pfou_lr_w
        } else {
            (compresseur - p.pfou_lr_w).min(0.0)
        };
        SortiesAppel {
            energie: Energie::Electricite,
            // Éq. (1327).
            q_cef_wh: p.pabs_lr_w * rdim,
            q_cons_wh: p.pabs_lr_w * rdim,
            // Éq. (1329) et (1330).
            q_fou_wh: p.pfou_lr_w * rdim,
            q_rest_wh: p.q_rest_act_wh * rdim,
            // Éq. (1331).
            tau_charge: p.lr,
            phi_vc_wh: 0.0,
            // Éq. (1337), pour la part comptée dans la puissance absorbée du mode.
            w_aux_pro_wh: p.w_aux_w * rdim,
            // Éq. (1328).
            eta_eff: p.perf_lr(),
            phi_rejet_wh: phi_rejet * rdim,
            q_prelec_wh: 0.0,
            r_fonct_ecs: if e.fonction == Poste::Ecs { p.lr } else { 0.0 },
            q_cef_aux_elec_wh: 0.0,
        }
    }
}

/// « Dfou0 » d'un système d'émission, en minutes (tableau 216).
pub fn dfou0_min(typo: crate::entree::TypoEmetteur) -> f64 {
    use crate::entree::TypoEmetteur as T;
    match typo {
        T::PlancherOuPlafondInertieForte => 32.0,
        T::RadiateursOuPlafondInertieMoyenne => 19.0,
        T::VentiloConvecteursOuInertieFaible => 6.0,
        T::SystemesAAir => 2.0,
    }
}

/// Grille d'une technologie de chauffage ; `None` pour une technologie hors de la version (air
/// extrait / air neuf).
pub fn grille_chauffage(t: crate::entree::TechnologieChauffagePac) -> Option<&'static Grille> {
    use crate::entree::TechnologieChauffagePac as T;
    match t {
        T::AirExterieurEau => Some(&CH_AIR_EXTERIEUR_EAU),
        T::AirExterieurAirRecycle => Some(&CH_AIR_EXTERIEUR_AIR_RECYCLE),
        T::EauDeNappeEau => Some(&CH_EAU_DE_NAPPE_EAU),
        T::EauGlycoleeEau => Some(&CH_EAU_GLYCOLEE_EAU),
        T::EauDeNappeAirRecycle => Some(&CH_EAU_DE_NAPPE_AIR_RECYCLE),
        T::EauDeBoucleAirRecycle => Some(&CH_EAU_DE_BOUCLE_AIR_RECYCLE),
        T::SolEau => Some(&CH_SOL_EAU),
        T::SolSol => Some(&CH_SOL_SOL),
        T::AirExtraitAirNeuf => None,
    }
}

/// Grille d'une technologie de refroidissement ; `None` hors de la version.
pub fn grille_refroidissement(
    t: crate::entree::TechnologieRefroidissementPac,
) -> Option<&'static Grille> {
    use crate::entree::TechnologieRefroidissementPac as T;
    match t {
        T::AirExterieurEau => Some(&FR_AIR_EXTERIEUR_EAU),
        T::AirExterieurAirRecycle => Some(&FR_AIR_EXTERIEUR_AIR_RECYCLE),
        T::EauEau => Some(&FR_EAU_EAU),
        T::EauAir => Some(&FR_EAU_AIR),
        T::EauDeNappeAirRecycle => Some(&FR_EAU_DE_NAPPE_AIR_RECYCLE),
        T::EauDeNappeEau => Some(&FR_EAU_DE_NAPPE_EAU),
        T::AirExtraitAirNeuf => None,
    }
}

/// Grille d'une technologie d'ECS ; `None` hors de la version.
pub fn grille_ecs(t: crate::entree::TechnologieEcsPac) -> Option<&'static Grille> {
    use crate::entree::TechnologieEcsPac as T;
    match t {
        T::AirExterieurEau => Some(&ECS_AIR_EXTERIEUR_EAU),
        T::AirExtraitEau => Some(&ECS_AIR_EXTRAIT_EAU),
        T::AirAmbiantEau => Some(&ECS_AIR_AMBIANT_EAU),
        T::EauDeNappeEau => Some(&ECS_EAU_DE_NAPPE_EAU),
        T::SolEau => Some(&ECS_SOL_EAU),
        T::EauGlycoleeEau => Some(&ECS_EAU_GLYCOLEE_EAU),
    }
}

/// Performances saisies, puissances converties en W.
pub fn performances_saisies(p: &crate::entree::PerformancesPac) -> PerformancesSaisies {
    use crate::entree::{PerformancesPac as P, StatutMesure};
    match p {
        P::Mesurees(points) => PerformancesSaisies::Mesurees(
            points
                .iter()
                .map(|x| PointMesure {
                    theta_aval_c: x.theta_aval_c,
                    theta_amont_c: x.theta_amont_c,
                    performance: x.performance,
                    pabs_w: 1000.0 * x.pabs_kw,
                    justifiee: x.statut == StatutMesure::Justifie,
                })
                .collect(),
        ),
        P::PivotDeclare {
            performance,
            pabs_kw,
        } => PerformancesSaisies::PivotDeclare {
            performance: *performance,
            pabs_w: 1000.0 * pabs_kw,
        },
        P::PivotDefaut { pabs_kw } => PerformancesSaisies::PivotDefaut {
            pabs_w: 1000.0 * pabs_kw,
        },
    }
}

/// « LRcontmin » et « CcpLRcontmin » après correction de statut (§8.23.3.6, éq. 1280 et
/// 1281) : `(LRcontmin, CcpLRcontmin, par défaut)`.
pub fn fonctionnement_continu(
    compresseur: FonctionnementCompresseur,
    saisie: Option<crate::entree::FonctionnementContinu>,
) -> (f64, f64, bool) {
    use crate::entree::StatutMesure;
    match (compresseur, saisie) {
        // Machine tout ou rien : « LRcontmin = 1 » (tableau 143), paramètres sans objet.
        (FonctionnementCompresseur::ToutOuRien, _) => (1.0, 1.0, false),
        (_, None) => (LR_CONTMIN_DEFAUT, CCP_LR_CONTMIN_DEFAUT, true),
        (_, Some(f)) => match f.statut {
            StatutMesure::Certifie => (f.lr_contmin, f.ccp_lr_contmin, false),
            StatutMesure::Justifie => (f.lr_contmin + 0.05, 0.9 * f.ccp_lr_contmin, false),
        },
    }
}

/// « Taux » après correction de statut (§8.23.3.6, éq. 1278 et 1279) : `(Taux, par défaut)` ;
/// `chauffe` est vrai si le générateur assure le chauffage ou l'ECS.
pub fn taux_auxiliaires(
    saisie: Option<crate::entree::TauxAuxiliaires>,
    chauffe: bool,
) -> (f64, bool) {
    use crate::entree::StatutMesure;
    match saisie {
        Some(t) => match t.statut {
            StatutMesure::Certifie => (t.valeur, false),
            StatutMesure::Justifie => (1.1 * t.valeur, false),
        },
        None if chauffe => (TAUX_DEFAUT_CH_ECS, true),
        None => (TAUX_DEFAUT_FR, true),
    }
}

impl GenerateurThermodynamique {
    /// Résout un générateur saisi et validé : matrices de performances, paramètres de charge
    /// partielle, auxiliaires, source amont, et valeurs par défaut appliquées.
    ///
    /// `espace_tampon` et `zone` donnent les indices des espaces tampons et des zones du
    /// bâtiment d'après leurs identifiants.
    pub fn resoudre(
        s: &crate::entree::ThermodynamiqueElectriqueSaisi,
        rdim: f64,
        espace_tampon: impl Fn(&str) -> Option<usize>,
        zone: impl Fn(&str) -> Option<usize>,
    ) -> Option<(
        Self,
        super::sources_amont::SourceAmontGenerateur,
        Vec<crate::resultats::ValeurParDefaut>,
    )> {
        use super::sources_amont::{SourceAmont, SourceAmontGenerateur};
        use crate::entree::{
            CombinaisonLimites, FonctionnementCompresseurSaisi, LimitesPac, PerformancesPac,
            SourceAmontSaisie,
        };
        use crate::resultats::ValeurParDefaut;
        let compresseur = match s.compresseur {
            FonctionnementCompresseurSaisi::PuissanceVariable => {
                FonctionnementCompresseur::PuissanceVariable
            }
            FonctionnementCompresseurSaisi::ToutOuRien => FonctionnementCompresseur::ToutOuRien,
        };
        let mut defauts = Vec::new();
        let (source, t_air_lim_c) = match &s.source_amont {
            SourceAmontSaisie::AirExterieur { p_vent_gaine_w } => (
                SourceAmontGenerateur {
                    source: SourceAmont::AirExterieur,
                    p_vent_gaine_w: p_vent_gaine_w.unwrap_or(0.0),
                },
                None,
            ),
            SourceAmontSaisie::AirAmbiant {
                espace_tampon: id,
                p_vent_gaine_w,
            } => (
                SourceAmontGenerateur {
                    source: SourceAmont::AirAmbiant {
                        espace_tampon: espace_tampon(id)?,
                    },
                    p_vent_gaine_w: p_vent_gaine_w.unwrap_or(0.0),
                },
                None,
            ),
            SourceAmontSaisie::AirExtrait {
                zone: id,
                t_air_lim_c,
                p_vent_gaine_w,
            } => (
                SourceAmontGenerateur {
                    source: SourceAmont::AirExtrait { zone: zone(id)? },
                    p_vent_gaine_w: p_vent_gaine_w.unwrap_or(0.0),
                },
                Some(*t_air_lim_c),
            ),
            // Sources sur eau et sur sol (§8.26.3.3.2, fiche 8.27).
            saisie => (
                SourceAmontGenerateur {
                    source: super::sources_amont::resoudre_eau_sol(saisie, &mut defauts)?,
                    p_vent_gaine_w: 0.0,
                },
                None,
            ),
        };
        let limites = |l: Option<LimitesPac>| match l {
            None => Limites::Aucune,
            Some(l) => match l.combinaison {
                CombinaisonLimites::LUneOuLAutre => Limites::LUneOuLAutre {
                    theta_aval_c: l.theta_aval_c,
                    theta_amont_c: l.theta_amont_c,
                },
                CombinaisonLimites::LUneEtLAutre => Limites::LUneEtLAutre {
                    theta_aval_c: l.theta_aval_c,
                    theta_amont_c: l.theta_amont_c,
                },
            },
        };
        let mut mode =
            |nom: &str,
             grille: &Grille,
             performances: &PerformancesPac,
             lim: Option<LimitesPac>,
             dfou0: f64,
             continu: Option<crate::entree::FonctionnementContinu>| {
                if matches!(performances, PerformancesPac::PivotDefaut { .. }) {
                    defauts.push(ValeurParDefaut {
                        champ: format!("{nom}.performances.pivot_defaut"),
                        valeur: 0.8 * grille.val_util_max,
                        regle: "Th-BCE 2020 §8.23.3.1 : valeur pivot par défaut, la valeur utile \
                            maximale de la technologie réduite de 20 %"
                            .to_owned(),
                    });
                }
                let (lr_contmin, ccp_lr_contmin, defaut) =
                    fonctionnement_continu(compresseur, continu);
                if defaut {
                    defauts.push(ValeurParDefaut {
                        champ: format!("{nom}.fonctionnement_continu.lr_contmin"),
                        valeur: lr_contmin,
                        regle: "Th-BCE 2020 §8.23.3.6, éq. (1280) et (1281) : LRcontmin = 0,4 et \
                            CcpLRcontmin = 1 par défaut"
                            .to_owned(),
                    });
                }
                ModePac {
                    matrice: grille.construire(&performances_saisies(performances)),
                    limites: limites(lim),
                    dfou0_min: dfou0,
                    lr_contmin,
                    ccp_lr_contmin,
                    t_air_lim_c,
                }
            };
        let chauffage = match &s.chauffage {
            Some(c) => Some(mode(
                "chauffage",
                grille_chauffage(c.technologie)?,
                &c.performances,
                c.limites,
                dfou0_min(c.typo_emetteur),
                c.fonctionnement_continu,
            )),
            None => None,
        };
        let refroidissement = match &s.refroidissement {
            Some(c) => Some(mode(
                "refroidissement",
                grille_refroidissement(c.technologie)?,
                &c.performances,
                c.limites,
                dfou0_min(c.typo_emetteur),
                c.fonctionnement_continu,
            )),
            None => None,
        };
        let ecs = match &s.ecs {
            Some(c) => Some(mode(
                "ecs",
                grille_ecs(c.technologie)?,
                &c.performances,
                c.limites,
                DFOU0_ECS_MIN,
                None,
            )),
            None => None,
        };
        // Éq. (1277) à (1279) : auxiliaires à charge nulle, rapportés à la puissance absorbée
        // au point pivot du mode chauffage s'il existe, du mode unique sinon.
        let (taux, taux_defaut) =
            taux_auxiliaires(s.taux_auxiliaires, chauffage.is_some() || ecs.is_some());
        if taux_defaut {
            defauts.push(ValeurParDefaut {
                champ: "taux_auxiliaires".to_owned(),
                valeur: taux,
                regle: "Th-BCE 2020 §8.23.3.6, éq. (1278) et (1279) : Taux par défaut, 0,02 en \
                        chauffage ou en ECS, 0,01 en refroidissement"
                    .to_owned(),
            });
        }
        let pabs_pivot = chauffage
            .as_ref()
            .or(ecs.as_ref())
            .or(refroidissement.as_ref())
            .map_or(0.0, |m| m.matrice.pabs_pivot_w());
        Some((
            Self {
                chauffage,
                refroidissement,
                ecs,
                compresseur,
                w_aux_0_w: taux * pabs_pivot,
                rdim,
                lecture_pas_partage: LecturePasPartage::default(),
            },
            source,
            defauts,
        ))
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn proche(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} ≠ {b}");
    }

    #[test]
    fn matrice_par_defaut_air_eau() {
        // Valeur pivot par défaut : 0,8 × 3,5 = 2,8 ; 2 kW absorbés au point pivot.
        let m =
            CH_AIR_EXTERIEUR_EAU.construire(&PerformancesSaisies::PivotDefaut { pabs_w: 2000.0 });
        assert_eq!(m.perf[1][3], 0.8 * 3.5);
        // Colonne du pivot (éq. 1239) : 23,5 °C : × 1,10 ; 42,5 : × 0,8 ; 51 : × 0,8² ; 60 : ×
        // 0,8³.
        proche(m.perf[0][3], 2.8 * 1.10);
        proche(m.perf[2][3], 2.8 * 0.8);
        proche(m.perf[3][3], 2.8 * 0.64);
        proche(m.perf[4][3], 2.8 * 0.512);
        // Lignes (éq. 1240) : −7 °C : × 0,5 ; 2 : × 0,8 ; 20 : × 1,25 ; −15 : × 0,5 × 0,8.
        proche(m.perf[1][1], 1.4);
        proche(m.perf[1][2], 2.24);
        proche(m.perf[1][4], 3.5);
        proche(m.perf[1][0], 1.12);
        proche(m.perf[3][0], 2.8 * 0.64 * 0.4);
        // Puissances absorbées (tableau 152).
        proche(m.pabs_w[1][3], 2000.0);
        proche(m.pabs_w[2][3], 1800.0);
        proche(m.pabs_w[3][3], 1800.0 * 0.915);
        proche(m.pabs_w[4][3], 1800.0 * 0.915 * 0.91);
        proche(m.pabs_w[0][3], 2180.0);
        proche(m.pabs_w[1][1], 1720.0);
        proche(m.pabs_w[1][0], 1720.0 * 0.92);
        proche(m.pabs_w[1][2], 1900.0);
        proche(m.pabs_w[1][4], 2260.0);
        proche(m.pfou_pivot_w(), 5600.0);
    }

    #[test]
    fn statuts_des_valeurs_saisies() {
        // Valeur déclarée : réduite de 20 % et plafonnée à 3,5.
        let m = CH_AIR_EXTERIEUR_EAU.construire(&PerformancesSaisies::PivotDeclare {
            performance: 4.0,
            pabs_w: 1500.0,
        });
        proche(m.perf[1][3], 3.2);
        let m = CH_AIR_EXTERIEUR_EAU.construire(&PerformancesSaisies::PivotDeclare {
            performance: 5.0,
            pabs_w: 1500.0,
        });
        proche(m.perf[1][3], 3.5);
        // Valeurs mesurées : certifiée telle quelle (4,5, non plafonnée), justifiée × 0,9 ; les
        // cases non mesurées sont complétées à partir des valeurs mesurées.
        let point = |aval, amont, performance, pabs_w, justifiee| PointMesure {
            theta_aval_c: aval,
            theta_amont_c: amont,
            performance,
            pabs_w,
            justifiee,
        };
        let points = vec![
            point(32.5, 7.0, 4.5, 1500.0, false),
            point(32.5, -7.0, 3.0, 1400.0, true),
            point(42.5, 7.0, 3.6, 1700.0, false),
            point(42.5, -7.0, 2.4, 1600.0, false),
        ];
        assert_eq!(CH_AIR_EXTERIEUR_EAU.verifier_mesures(&points), Ok(()));
        let m = CH_AIR_EXTERIEUR_EAU.construire(&PerformancesSaisies::Mesurees(points.clone()));
        proche(m.perf[1][3], 4.5);
        proche(m.perf[1][1], 2.7);
        proche(m.perf[2][3], 3.6);
        proche(m.perf[2][1], 2.4);
        proche(m.pabs_w[2][1], 1600.0);
        // 51 °C à partir de 42,5 °C mesuré ; 2 °C à partir de 7 °C ; −15 °C à partir de −7 °C.
        proche(m.perf[3][3], 3.6 * 0.8);
        proche(m.perf[1][2], 4.5 * 0.8);
        proche(m.perf[1][0], 2.7 * 0.8);
        proche(m.pabs_w[3][3], 1700.0 * 0.915);
        // Ordre de priorité : 42,5 °C sans 32,5 °C, ou −7 °C sans 7 °C, est refusé ; une case
        // manquante dans le croisement aussi.
        assert!(CH_AIR_EXTERIEUR_EAU
            .verifier_mesures(&[point(42.5, 7.0, 3.6, 1700.0, false)])
            .is_err());
        assert!(CH_AIR_EXTERIEUR_EAU.verifier_mesures(&points[..3]).is_err());
        assert!(CH_AIR_EXTERIEUR_EAU
            .verifier_mesures(&[point(30.0, 7.0, 3.6, 1700.0, false)])
            .is_err());
    }

    #[test]
    fn grande_puissance() {
        // 150 kW au point pivot : Cnnam_COP(−7, 7) = 0,60 (tableau 151).
        let m = CH_AIR_EXTERIEUR_EAU.construire(&PerformancesSaisies::PivotDefaut {
            pabs_w: 150_000.0 / 2.8,
        });
        proche(m.perf[1][1], 2.8 * 0.6);
        proche(m.perf[1][0], 2.8 * 0.6 * 0.8);
    }

    #[test]
    fn autres_grilles() {
        let defaut = PerformancesSaisies::PivotDefaut { pabs_w: 1000.0 };
        // Air / air en chauffage : pivot à 20 °C intérieur et 7 °C extérieur.
        let m = CH_AIR_EXTERIEUR_AIR_RECYCLE.construire(&defaut);
        proche(m.perf[3][3], 2.8);
        proche(m.perf[0][3], 2.8 * 1.3);
        proche(m.perf[4][3], 2.8 * 0.9);
        proche(m.perf[2][1], 2.8 * 1.1 * 0.5);
        proche(m.pabs_w[1][4], 1000.0 * 1.10 * 1.13);
        // Refroidisseurs : pivot à 35 °C extérieur ; EER par défaut 0,8 × 2,7 = 2,16.
        let m = FR_AIR_EXTERIEUR_AIR_RECYCLE.construire(&defaut);
        proche(m.perf[1][3], 2.16);
        proche(m.perf[0][3], 2.16 * 0.9);
        proche(m.perf[2][3], 2.16 * 1.075);
        proche(m.perf[3][3], 2.16 * 1.15);
        proche(m.perf[1][0], 2.16 * 1.6);
        proche(m.perf[1][4], 2.16 * 0.8);
        proche(m.pabs_w[3][2], 1000.0 * 1.1 * 1.1);
        let m = FR_AIR_EXTERIEUR_EAU.construire(&defaut);
        proche(m.perf[4][1], 2.16 * 1.225 * 1.4);
        proche(m.pabs_w[0][4], 1000.0 * 0.945 * 0.9);
        // ECS : colonnes de la valeur pivot identiques pour les trois sources (éq. 1256).
        for g in [
            ECS_AIR_EXTERIEUR_EAU,
            ECS_AIR_EXTRAIT_EAU,
            ECS_AIR_AMBIANT_EAU,
        ] {
            let m = g.construire(&defaut);
            let j = g.pivot.1;
            let pivot = 0.8 * g.val_util_max;
            proche(m.perf[4][j], pivot);
            proche(m.perf[0][j], pivot * 1.8);
            proche(m.perf[6][j], pivot * 0.6);
            proche(m.pabs_w[0][j], 1400.0);
            proche(m.pabs_w[6][j], 800.0);
        }
        let m = ECS_AIR_AMBIANT_EAU.construire(&defaut);
        proche(m.perf[4][5], 0.8 * 3.1 * 1.3);
        proche(m.pabs_w[4][0], 900.0);
        let m = ECS_AIR_EXTRAIT_EAU.construire(&defaut);
        proche(m.perf[4][0], 0.8 * 3.2 * 0.7);
        let m = ECS_AIR_EXTERIEUR_EAU.construire(&defaut);
        proche(m.perf[4][4], 0.8 * 2.7 * 1.5);
        proche(m.pabs_w[4][0], 860.0);
    }

    #[test]
    fn interpolation_des_cop() {
        let m =
            CH_AIR_EXTERIEUR_EAU.construire(&PerformancesSaisies::PivotDefaut { pabs_w: 2000.0 });
        // Aux nœuds de la matrice.
        let (pabs, cop) = m.pleine_charge(7.0, 32.5);
        proche(pabs, 2000.0);
        proche(cop, 2.8);
        // Entre 2 °C (2,24) et 7 °C (2,8) à 32,5 °C : à 4 °C, Cθam = 0,4 → 2,464.
        let (pabs, cop) = m.pleine_charge(4.0, 32.5);
        proche(cop, 2.24 + 0.4 * (2.8 - 2.24));
        proche(pabs, 1900.0 + 0.4 * 100.0);
        // Interpolation sur les deux températures (éq. 1287) : 4 °C et 37,5 °C (Cθav = 0,5).
        let (_, cop) = m.pleine_charge(4.0, 37.5);
        let bas = 2.24 + 0.4 * (2.8 - 2.24);
        let haut = 0.8 * bas;
        proche(cop, 0.5 * bas + 0.5 * haut);
        // Hors de la matrice : valeurs des bornes (éq. 1283 et 1284).
        let (_, cop) = m.pleine_charge(-20.0, 32.5);
        proche(cop, m.perf[1][0]);
        let (_, cop) = m.pleine_charge(30.0, 70.0);
        proche(cop, m.perf[4][4]);
        let (_, cop) = m.pleine_charge(7.0, 20.0);
        proche(cop, m.perf[0][3]);
    }

    fn pac(compresseur: FonctionnementCompresseur) -> GenerateurThermodynamique {
        let matrice =
            CH_AIR_EXTERIEUR_EAU.construire(&PerformancesSaisies::PivotDefaut { pabs_w: 2000.0 });
        GenerateurThermodynamique {
            chauffage: Some(ModePac {
                matrice,
                limites: Limites::Aucune,
                dfou0_min: 32.0,
                lr_contmin: LR_CONTMIN_DEFAUT,
                ccp_lr_contmin: CCP_LR_CONTMIN_DEFAUT,
                t_air_lim_c: None,
            }),
            refroidissement: None,
            ecs: None,
            compresseur,
            // Taux = 0,02 : Waux,0 = 40 W.
            w_aux_0_w: 0.02 * 2000.0,
            rdim: 1.0,
            lecture_pas_partage: LecturePasPartage::Litterale,
        }
    }

    #[test]
    fn point_de_fonctionnement_tout_ou_rien() {
        // Au point pivot (7 °C, 32,5 °C) : Pabs_pc = 2 000 W, COP_pc = 2,8, Pfou_pc = 5 600 W.
        // Demande de 2 800 Wh : LR = 0,5.
        let g = pac(FonctionnementCompresseur::ToutOuRien);
        let p = g.point(Poste::Chauffage, 2800.0, 7.0, 32.5, 1.0, false, None);
        proche(p.pfou_pc_w, 5600.0);
        proche(p.lr, 0.5);
        // Éq. (1300) à (1303) : Pcomp_pc = 1 960 W ; Pcomp_LR = 980 W ;
        // Pcompma = 1 960 × 0,5 × 0,5 × 0,5 / 32 = 7,65625 W ; Pabs_LR = 1 027,65625 W.
        proche(p.pcomp_lr_w, 980.0);
        proche(p.pcompma_lr_w, 1960.0 * 0.5 * 0.25 / 32.0);
        proche(p.pabs_lr_w, 980.0 + 7.65625 + 40.0);
        proche(p.perf_lr(), 2800.0 / 1027.65625);
        proche(p.q_rest_act_wh, 0.0);
        let s = g.appel(&EntreesAppel {
            fonction: Poste::Chauffage,
            q_req_wh: 2800.0,
            theta_amont_c: 7.0,
            theta_aval_c: 32.5,
            ..EntreesAppel::demande(Poste::Chauffage, 2800.0)
        });
        proche(s.q_fou_wh, 2800.0);
        proche(s.q_cef_wh, 1027.65625);
        proche(s.tau_charge, 0.5);
        proche(s.w_aux_pro_wh, 40.0);
        // Éq. (1332) : chaleur puisée à la source, comptée négativement.
        proche(s.phi_rejet_wh, 980.0 + 7.65625 - 2800.0);
        // Pleine charge dépassée : 400 Wh restent à fournir ; pas de cycles à LR = 1.
        let p = g.point(Poste::Chauffage, 6000.0, 7.0, 32.5, 1.0, false, None);
        proche(p.q_rest_act_wh, 400.0);
        proche(p.lr, 1.0);
        proche(p.pabs_lr_w, 2000.0);
        proche(p.perf_lr(), 2.8);
        // Charge nulle : auxiliaires seuls (éq. 1324).
        let p = g.point(Poste::Chauffage, 0.0, 7.0, 32.5, 1.0, false, None);
        proche(p.pabs_lr_w, 40.0);
        proche(p.perf_lr(), 0.0);
    }

    #[test]
    fn point_de_fonctionnement_puissance_variable() {
        let mut g = pac(FonctionnementCompresseur::PuissanceVariable);
        if let Some(m) = &mut g.chauffage {
            m.ccp_lr_contmin = 1.1;
        }
        // COP net à pleine charge (éq. 1306) : 5 600 / 1 960.
        let cop_net = 5600.0 / 1960.0;
        // Éq. (1307) : CCPnet = 0,4 × 1 960 × 1,1 / (0,4 × 2 000 − 1,1 × 40).
        let ccp_net = 0.4 * 1960.0 * 1.1 / (0.4 * 2000.0 - 1.1 * 40.0);
        // LR = 0,7, en fonctionnement continu (éq. 1308 à 1310).
        let p = g.point(Poste::Chauffage, 3920.0, 7.0, 32.5, 1.0, false, None);
        proche(p.lr, 0.7);
        let cop_lr_net = cop_net * (1.0 + (ccp_net - 1.0) * 0.3 / 0.6);
        proche(p.pcomp_lr_w, 3920.0 / cop_lr_net);
        proche(p.pcompma_lr_w, 0.0);
        proche(p.pabs_lr_w, 3920.0 / cop_lr_net + 40.0);
        // À LR = LRcontmin, le COP brut est majoré de CcpLRcontmin : 1,1 × 2,8.
        let p = g.point(Poste::Chauffage, 0.4 * 5600.0, 7.0, 32.5, 1.0, false, None);
        proche(p.perf_lr(), 1.1 * 2.8);
        // LR = 0,2, sous le taux minimal (éq. 1312 à 1317) : cycles entre l'arrêt et LRcontmin.
        let p = g.point(Poste::Chauffage, 1120.0, 7.0, 32.5, 1.0, false, None);
        let pcomp_contmin = 0.4 * 5600.0 / (cop_net * ccp_net);
        proche(p.pcomp_lr_w, pcomp_contmin * 0.5);
        proche(p.pcompma_lr_w, pcomp_contmin * 0.5 * 0.5 * 0.5 / 32.0);
        proche(p.pabs_lr_w, p.pcomp_lr_w + p.pcompma_lr_w + 40.0);
    }

    #[test]
    fn limites_de_fonctionnement() {
        let mut g = pac(FonctionnementCompresseur::ToutOuRien);
        if let Some(m) = &mut g.chauffage {
            m.limites = Limites::LUneOuLAutre {
                theta_aval_c: 55.0,
                theta_amont_c: -7.0,
            };
        }
        // Sous −7 °C : la machine ne fonctionne pas, toute la demande reste à fournir et seuls
        // les auxiliaires consomment (éq. 1296 et 1324).
        let p = g.point(Poste::Chauffage, 3000.0, -10.0, 32.5, 1.0, false, None);
        proche(p.pfou_lr_w, 0.0);
        proche(p.q_rest_act_wh, 3000.0);
        proche(p.pabs_lr_w, 40.0);
        // L'une et l'autre : il faut les deux conditions.
        if let Some(m) = &mut g.chauffage {
            m.limites = Limites::LUneEtLAutre {
                theta_aval_c: 55.0,
                theta_amont_c: -7.0,
            };
        }
        let p = g.point(Poste::Chauffage, 3000.0, -10.0, 32.5, 1.0, false, None);
        assert!(p.pfou_lr_w > 0.0);
        // En refroidissement, les limites sont une température aval minimale et une température
        // amont maximale (éq. 1297).
        let l = Limites::LUneOuLAutre {
            theta_aval_c: 5.0,
            theta_amont_c: 40.0,
        };
        assert!(l.arret(Poste::Refroidissement, 42.0, 9.5));
        assert!(l.arret(Poste::Refroidissement, 35.0, 4.0));
        assert!(!l.arret(Poste::Refroidissement, 35.0, 9.5));
    }

    #[test]
    fn machines_identiques_et_refroidissement() {
        // Deux machines identiques : mêmes performances, puissances doublées (éq. 1295, 1327 et
        // 1329).
        let mut g = pac(FonctionnementCompresseur::ToutOuRien);
        g.rdim = 2.0;
        let e = EntreesAppel {
            theta_amont_c: 7.0,
            theta_aval_c: 32.5,
            ..EntreesAppel::demande(Poste::Chauffage, 5600.0)
        };
        let s = g.appel(&e);
        proche(s.tau_charge, 0.5);
        proche(s.q_cef_wh, 2.0 * 1027.65625);
        proche(g.pn_gen_tot_w(Poste::Chauffage), 11200.0);
        // Refroidissement : le rejet est la somme du froid produit et du travail du
        // compresseur (éq. 1334).
        let matrice = FR_AIR_EXTERIEUR_AIR_RECYCLE
            .construire(&PerformancesSaisies::PivotDefaut { pabs_w: 1000.0 });
        let clim = GenerateurThermodynamique {
            chauffage: None,
            refroidissement: Some(ModePac {
                matrice,
                limites: Limites::Aucune,
                dfou0_min: 2.0,
                lr_contmin: 1.0,
                ccp_lr_contmin: 1.0,
                t_air_lim_c: None,
            }),
            ecs: None,
            compresseur: FonctionnementCompresseur::ToutOuRien,
            w_aux_0_w: 10.0,
            rdim: 1.0,
            lecture_pas_partage: LecturePasPartage::Litterale,
        };
        assert_eq!(clim.fonction(), FonctionGenerateur::Refroidissement);
        // 35 °C dehors, 27 °C dedans : EER 2,16, 2 160 W ; demande de 1 080 Wh : LR = 0,5.
        let s = clim.appel(&EntreesAppel {
            theta_amont_c: 35.0,
            theta_aval_c: 27.0,
            ..EntreesAppel::demande(Poste::Refroidissement, 1080.0)
        });
        let pcomp = 990.0 * 0.5;
        let pcompma = 990.0 * 0.5 * 0.25 / 2.0;
        proche(s.q_cef_wh, pcomp + pcompma + 10.0);
        proche(s.phi_rejet_wh, pcomp + pcompma + 1080.0);
        proche(s.eta_eff, 1080.0 / (pcomp + pcompma + 10.0));
    }

    #[test]
    fn double_service() {
        // PAC double service : ECS prioritaire à pleine charge pendant une fraction du pas de
        // temps (éq. 1319 à 1323), chauffage sur le temps restant (éq. 1293).
        let mut g = pac(FonctionnementCompresseur::ToutOuRien);
        g.ecs = Some(ModePac {
            matrice: ECS_AIR_EXTERIEUR_EAU
                .construire(&PerformancesSaisies::PivotDefaut { pabs_w: 2000.0 }),
            limites: Limites::Aucune,
            dfou0_min: DFOU0_ECS_MIN,
            lr_contmin: 1.0,
            ccp_lr_contmin: 1.0,
            t_air_lim_c: None,
        });
        assert_eq!(g.fonction(), FonctionGenerateur::ChauffageEcs);
        // ECS à 7 °C et 45 °C : COP 2,16, 4 320 W ; demande de 1 080 Wh : LR = 0,25.
        let p = g.point(Poste::Ecs, 1080.0, 7.0, 45.0, 1.0, false, None);
        proche(p.lr, 0.25);
        proche(p.pabs_lr_w, 500.0);
        proche(p.perf_lr(), 2.16);
        // Hors saison de chauffage, les auxiliaires du reste du pas de temps s'ajoutent
        // (éq. 1322).
        let p = g.point(Poste::Ecs, 1080.0, 7.0, 45.0, 1.0, true, None);
        proche(p.pabs_lr_w, 500.0 + 0.75 * 40.0);
        // Chauffage sur les trois quarts restants : puissance disponible 0,75 × 5 600 W.
        let p = g.point(Poste::Chauffage, 2100.0, 7.0, 32.5, 0.75, false, None);
        proche(p.pfou_pc_w, 4200.0);
        proche(p.lr, 0.5);
        proche(p.pcomp_lr_w, 980.0);
        // Variante pondérée par le temps disponible.
        g.lecture_pas_partage = LecturePasPartage::PonderationTemporelle;
        let q = g.point(Poste::Chauffage, 2100.0, 7.0, 32.5, 0.75, false, None);
        proche(q.pcomp_lr_w, 0.75 * 980.0);
        // Sans demande de chauffage : auxiliaires sur la part du pas de temps restante
        // (éq. 1325).
        let p = g.point(Poste::Chauffage, 0.0, 7.0, 32.5, 0.75, false, None);
        proche(p.pabs_lr_w, 30.0);
    }

    #[test]
    fn source_sur_air_extrait() {
        // CET sur air extrait : 0,04 kg/s d'air à 20 °C, rejeté au plus froid à 5 °C :
        // Pech = 0,04 × 1 006 × 15 = 603,6 W.
        let g = GenerateurThermodynamique {
            chauffage: None,
            refroidissement: None,
            ecs: Some(ModePac {
                matrice: ECS_AIR_EXTRAIT_EAU
                    .construire(&PerformancesSaisies::PivotDefaut { pabs_w: 500.0 }),
                limites: Limites::Aucune,
                dfou0_min: DFOU0_ECS_MIN,
                lr_contmin: 1.0,
                ccp_lr_contmin: 1.0,
                t_air_lim_c: Some(5.0),
            }),
            compresseur: FonctionnementCompresseur::ToutOuRien,
            w_aux_0_w: 0.0,
            rdim: 1.0,
            lecture_pas_partage: LecturePasPartage::Litterale,
        };
        // Sans limite : COP 2,56 à 20 °C et 45 °C, 1 280 W ; éq. (1455) :
        // 603,6 × 1 280 / (1 280 − 500) = 990,52 W.
        let w_k = 0.04 * 1006.0;
        let p = g.point(Poste::Ecs, 5000.0, 20.0, 45.0, 1.0, true, Some(w_k));
        proche(p.pfou_pc_w, 603.6 * 1280.0 / 780.0);
        let p = g.point(Poste::Ecs, 5000.0, 20.0, 45.0, 1.0, true, None);
        proche(p.pfou_pc_w, 1280.0);
    }

    #[test]
    fn grilles_sur_eau_et_sur_sol_a_la_main() {
        // PAC eau de nappe / air recyclé (éq. 1249 et 1250) : pivot déclaré 5,0 réduit de 20 %
        // et plafonné à 3,5 ; ligne à 5 °C : 3,5 × 1,3 = 4,55 ; colonne à 18,5 °C : × 1,2.
        let m = CH_EAU_DE_NAPPE_AIR_RECYCLE.construire(&PerformancesSaisies::PivotDeclare {
            performance: 5.0,
            pabs_w: 2000.0,
        });
        proche(m.perf[3][1], 3.5);
        proche(m.perf[0][1], 4.55);
        proche(m.perf[0][3], 4.55 * 1.2);
        proche(m.pabs_w[0][1], 2300.0);
        proche(m.pabs_w[0][3], 2300.0 * 1.10);
        // PAC sol / sol (éq. 1255) : une ligne, pivot par défaut 0,8 × 3,8 = 3,04 à 4 °C.
        let m = CH_SOL_SOL.construire(&PerformancesSaisies::PivotDefaut { pabs_w: 1000.0 });
        proche(m.perf[0][2], 3.04);
        proche(m.perf[0][0], 3.04 * 0.8);
        proche(m.perf[0][1], 3.04 * 0.95);
        proche(m.perf[0][3], 3.04 * 1.07);
        proche(m.pabs_w[0][0], 1000.0);
        // À 2,75 °C (tableau 245 pour 2,5 °C de sol), à mi-chemin entre 1,5 et 4 °C ; la
        // température aval est sans effet.
        let (pabs, cop) = m.pleine_charge(2.75, 28.0);
        proche(cop, 0.5 * 3.04 * 0.95 + 0.5 * 3.04);
        proche(pabs, 1000.0);
        assert!(CH_SOL_SOL
            .verifier_mesures(&[PointMesure {
                theta_aval_c: 35.0,
                theta_amont_c: 4.0,
                performance: 4.2,
                pabs_w: 900.0,
                justifiee: false,
            }])
            .is_ok());
        // Refroidisseur eau de nappe / eau (éq. 1275 et 1276) : 0,8 × 5 = 4 plafonné à 3,7 ;
        // ligne à 26 °C : 3,7 × 1,225 ; colonne à 22,5 °C : × 0,8.
        let m = FR_EAU_DE_NAPPE_EAU.construire(&PerformancesSaisies::PivotDeclare {
            performance: 5.0,
            pabs_w: 1000.0,
        });
        proche(m.perf[1][1], 3.7);
        proche(m.perf[4][1], 3.7 * 1.225);
        proche(m.perf[4][3], 3.7 * 1.225 * 0.8);
        proche(m.pabs_w[0][0], 945.0 * 1.05);
        // PAC eau glycolée / eau en ECS (éq. 1256 et 1262) : pivot par défaut 0,8 × 3,7 à
        // −1,5 °C et 45 °C ; ligne à 55 °C : × 0,8 ; colonne à 13,5 °C : × 1,3.
        let m =
            ECS_EAU_GLYCOLEE_EAU.construire(&PerformancesSaisies::PivotDefaut { pabs_w: 1000.0 });
        proche(m.perf[4][1], 2.96);
        proche(m.perf[5][4], 2.96 * 0.8 * 1.3);
        proche(m.pabs_w[5][4], 900.0 * 1.15);
    }
}
