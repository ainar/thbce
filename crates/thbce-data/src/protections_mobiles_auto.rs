// SPDX-License-Identifier: GPL-3.0-or-later
//! Données conventionnelles complémentaires de la gestion des protections mobiles : gestion
//! automatique avec dérogation et stores vénitiens (lot 1-env, `docs/questions/Q100.md`).
//!
//! Source : annexe III de l'arrêté du 4 août 2021, fiche `C_BAT_Gestion_protections_mobiles`
//! (§5.9), `docs/reference/` :
//! - figures 35 à 40-3 (p. 197 à 202), lignes « gestion automatique, occupation, dérogation
//!   manuelle sans ou avec détecteur de présence » (`type-GPM-manu` = 1 et 2) ;
//! - figures 41 à 43-3 (p. 203 et 204), stores vénitiens en gestion manuelle ; les lignes de
//!   dérogation de la gestion automatique y sont « idem gestion manuelle » ;
//! - figures 44 à 47 (p. 205 et 206), matrices conventionnelles de gestion automatique ;
//! - tableau 34 (p. 218), lignes des stores à lames.
//!
//! Les figures sont des images du PDF de l'annexe III, lues à 250 dpi une première fois le
//! 18 novembre 2026 (une seule lecture, à confirmer ; les valeurs des figures 41 à 47 sont
//! nettes, celles des figures 35 à 40-3 sont des images de basse définition).

use crate::protections_mobiles::{
    inocc, occ, FamilleGpm, LigneGpm, LigneInoccupation, LigneOccupation,
};

/// « Eclim_manu_sv » : seuil d'éclairement incident de la gestion manuelle des stores vénitiens,
/// en lux (§5.9.2, tableau 33, valeur conventionnelle).
pub const ECL_LIM_MANU_SV_LUX: f64 = 15_000.0;

/// « Vventlim_manu_sv » : vitesse de vent à partir de laquelle un store vénitien extérieur est
/// remonté en gestion manuelle, en m/s (§5.9.2, tableau 33, valeur conventionnelle).
pub const V_VENT_LIM_MANU_SV_M_S: f64 = 10.0;

/// Tableau 34, lignes des stores à lames : en calcul Th-D, en période de confort adaptatif et
/// pour `Etp** ≥ Eclim_manu_sv`, `Rprot1_manu_occ_sv` vaut 90 % (toutes les familles).
pub const TH_D_R_PROT_OCC_SV: f64 = 0.90;
/// Tableau 34, lignes des stores à lames : idem pour `Rprot1_manu_inocc_sv`, 70 % (toutes les
/// familles, y compris « Autres », recontrôlée le 18 novembre 2026).
pub const TH_D_R_PROT_INOCC_SV: f64 = 0.70;

/// Lignes « dérogation manuelle » de la gestion automatique d'un type de protection pour une
/// famille : sans puis avec détecteur de présence. Indexées par [`LigneGpm`]. Le seuil `Ecl2`
/// de la seconde protection est celui de la ligne sans détecteur (« idem avec ou sans détecteur
/// de présence »).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Derogation {
    /// `type-GPM-manu = 2` : gestion automatique sans détecteur de présence.
    pub sans_detecteur: [LigneOccupation; 4],
    /// `type-GPM-manu = 1` : gestion automatique avec détecteur de présence.
    pub avec_detecteur: [LigneOccupation; 4],
}

/// Lignes de dérogation d'une famille : volets, puis stores enroulables.
pub fn derogation(famille: FamilleGpm) -> (&'static Derogation, &'static Derogation) {
    match famille {
        FamilleGpm::Habitation => (
            &DEROGATION_VOLET_HABITATION,
            &DEROGATION_STORE_ENROULABLE_HABITATION,
        ),
        FamilleGpm::Bureaux => (
            &DEROGATION_VOLET_BUREAUX,
            &DEROGATION_STORE_ENROULABLE_BUREAUX,
        ),
        FamilleGpm::Enseignement => (
            &DEROGATION_VOLET_ENSEIGNEMENT,
            &DEROGATION_STORE_ENROULABLE_ENSEIGNEMENT,
        ),
        FamilleGpm::Restauration => (
            &DEROGATION_VOLET_RESTAURATION,
            &DEROGATION_STORE_ENROULABLE_RESTAURATION,
        ),
        FamilleGpm::Hopitaux => (
            &DEROGATION_VOLET_HOPITAUX,
            &DEROGATION_STORE_ENROULABLE_HOPITAUX,
        ),
        FamilleGpm::Autre => (
            &DEROGATION_VOLET_AUTRES,
            &DEROGATION_STORE_ENROULABLE_AUTRES,
        ),
    }
}

/// Figure 35 : lignes « dérogation manuelle » de la gestion automatique, volets, famille « habitation ».
#[rustfmt::skip]
pub const DEROGATION_VOLET_HABITATION: Derogation = Derogation {
    sans_detecteur: [
        occ(0.10, 100_000.0, 0.10, 100_000.0, 0.80),
        occ(0.10, 100_000.0, 0.10, 60_000.0, 0.80),
        occ(0.20, 60_000.0, 0.20, 30_000.0, 0.80),
        occ(0.30, 40_000.0, 0.50, 30_000.0, 0.80),
    ],
    avec_detecteur: [
        occ(0.10, 100_000.0, 0.10, 100_000.0, 0.90),
        occ(0.10, 100_000.0, 0.10, 60_000.0, 0.90),
        occ(0.20, 50_000.0, 0.20, 30_000.0, 0.90),
        occ(0.30, 30_000.0, 0.50, 30_000.0, 0.90),
    ],
};
/// Figure 36 : lignes « dérogation manuelle » de la gestion automatique, volets, famille « bureaux ».
#[rustfmt::skip]
pub const DEROGATION_VOLET_BUREAUX: Derogation = Derogation {
    sans_detecteur: [
        occ(0.05, 80_000.0, 0.05, 80_000.0, 0.90),
        occ(0.05, 100_000.0, 0.05, 80_000.0, 0.90),
        occ(0.15, 80_000.0, 0.30, 20_000.0, 0.90),
        occ(0.20, 60_000.0, 0.30, 20_000.0, 0.90),
    ],
    avec_detecteur: [
        occ(0.05, 80_000.0, 0.05, 80_000.0, 0.90),
        occ(0.05, 100_000.0, 0.05, 80_000.0, 0.90),
        occ(0.15, 70_000.0, 0.30, 20_000.0, 0.90),
        occ(0.20, 50_000.0, 0.30, 20_000.0, 0.90),
    ],
};
/// Figure 37 : lignes « dérogation manuelle » de la gestion automatique, volets, famille « enseignement ».
#[rustfmt::skip]
pub const DEROGATION_VOLET_ENSEIGNEMENT: Derogation = Derogation {
    sans_detecteur: [
        occ(0.05, 80_000.0, 0.05, 80_000.0, 0.20),
        occ(0.05, 100_000.0, 0.05, 80_000.0, 0.20),
        occ(0.15, 80_000.0, 0.30, 20_000.0, 0.20),
        occ(0.20, 60_000.0, 0.30, 20_000.0, 0.20),
    ],
    avec_detecteur: [
        occ(0.05, 80_000.0, 0.05, 80_000.0, 0.20),
        occ(0.05, 100_000.0, 0.05, 80_000.0, 0.20),
        occ(0.15, 70_000.0, 0.30, 20_000.0, 0.20),
        occ(0.20, 50_000.0, 0.30, 20_000.0, 0.20),
    ],
};
/// Figure 37-1 : lignes « dérogation manuelle » de la gestion automatique, volets, famille « restauration ».
#[rustfmt::skip]
pub const DEROGATION_VOLET_RESTAURATION: Derogation = Derogation {
    sans_detecteur: [
        occ(0.05, 150_000.0, 0.05, 80_000.0, 0.10),
        occ(0.05, 150_000.0, 0.05, 80_000.0, 0.10),
        occ(0.15, 150_000.0, 0.30, 20_000.0, 0.10),
        occ(0.20, 150_000.0, 0.30, 20_000.0, 0.10),
    ],
    avec_detecteur: [
        occ(0.05, 150_000.0, 0.05, 80_000.0, 0.10),
        occ(0.05, 150_000.0, 0.05, 80_000.0, 0.10),
        occ(0.15, 150_000.0, 0.30, 20_000.0, 0.10),
        occ(0.20, 150_000.0, 0.30, 20_000.0, 0.10),
    ],
};
/// Figure 37-2 : lignes « dérogation manuelle » de la gestion automatique, volets, famille « hopitaux ».
#[rustfmt::skip]
pub const DEROGATION_VOLET_HOPITAUX: Derogation = Derogation {
    sans_detecteur: [
        occ(0.10, 60_000.0, 0.10, 80_000.0, 0.80),
        occ(0.10, 80_000.0, 0.10, 80_000.0, 0.80),
        occ(0.20, 30_000.0, 0.20, 20_000.0, 0.80),
        occ(0.30, 30_000.0, 0.50, 20_000.0, 0.80),
    ],
    avec_detecteur: [
        occ(0.10, 60_000.0, 0.10, 80_000.0, 0.90),
        occ(0.10, 80_000.0, 0.10, 80_000.0, 0.90),
        occ(0.20, 20_000.0, 0.20, 20_000.0, 0.90),
        occ(0.30, 20_000.0, 0.50, 20_000.0, 0.90),
    ],
};
/// Figure 37-3 : lignes « dérogation manuelle » de la gestion automatique, volets, famille « autres ».
#[rustfmt::skip]
pub const DEROGATION_VOLET_AUTRES: Derogation = Derogation {
    sans_detecteur: [
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.80),
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.80),
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.80),
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.80),
    ],
    avec_detecteur: [
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.80),
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.80),
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.80),
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.80),
    ],
};
/// Figure 38 : lignes « dérogation manuelle » de la gestion automatique, stores enroulables, famille « habitation ».
#[rustfmt::skip]
pub const DEROGATION_STORE_ENROULABLE_HABITATION: Derogation = Derogation {
    sans_detecteur: [
        occ(0.10, 100_000.0, 0.10, 100_000.0, 0.10),
        occ(0.10, 60_000.0, 0.10, 60_000.0, 0.10),
        occ(0.20, 50_000.0, 0.20, 30_000.0, 0.10),
        occ(0.30, 30_000.0, 0.50, 30_000.0, 0.10),
    ],
    avec_detecteur: [
        occ(0.10, 60_000.0, 0.10, 100_000.0, 0.10),
        occ(0.10, 60_000.0, 0.10, 60_000.0, 0.10),
        occ(0.20, 40_000.0, 0.20, 30_000.0, 0.10),
        occ(0.30, 30_000.0, 0.50, 30_000.0, 0.10),
    ],
};
/// Figure 39 : lignes « dérogation manuelle » de la gestion automatique, stores enroulables, famille « bureaux ».
#[rustfmt::skip]
pub const DEROGATION_STORE_ENROULABLE_BUREAUX: Derogation = Derogation {
    sans_detecteur: [
        occ(0.10, 60_000.0, 0.10, 80_000.0, 0.15),
        occ(0.10, 80_000.0, 0.20, 80_000.0, 0.15),
        occ(0.25, 30_000.0, 0.45, 20_000.0, 0.20),
        occ(0.30, 30_000.0, 0.50, 20_000.0, 0.20),
    ],
    avec_detecteur: [
        occ(0.10, 60_000.0, 0.10, 80_000.0, 0.10),
        occ(0.10, 80_000.0, 0.20, 80_000.0, 0.10),
        occ(0.25, 20_000.0, 0.45, 20_000.0, 0.15),
        occ(0.30, 20_000.0, 0.50, 20_000.0, 0.15),
    ],
};
/// Figure 40 : lignes « dérogation manuelle » de la gestion automatique, stores enroulables, famille « enseignement ».
#[rustfmt::skip]
pub const DEROGATION_STORE_ENROULABLE_ENSEIGNEMENT: Derogation = Derogation {
    sans_detecteur: [
        occ(0.10, 60_000.0, 0.10, 80_000.0, 0.15),
        occ(0.10, 80_000.0, 0.20, 80_000.0, 0.15),
        occ(0.25, 30_000.0, 0.45, 20_000.0, 0.15),
        occ(0.30, 30_000.0, 0.50, 20_000.0, 0.15),
    ],
    avec_detecteur: [
        occ(0.10, 60_000.0, 0.10, 80_000.0, 0.10),
        occ(0.10, 80_000.0, 0.20, 80_000.0, 0.10),
        occ(0.25, 20_000.0, 0.45, 20_000.0, 0.10),
        occ(0.30, 20_000.0, 0.50, 20_000.0, 0.10),
    ],
};
/// Figure 40-1 : lignes « dérogation manuelle » de la gestion automatique, stores enroulables, famille « restauration ».
#[rustfmt::skip]
pub const DEROGATION_STORE_ENROULABLE_RESTAURATION: Derogation = Derogation {
    sans_detecteur: [
        occ(0.10, 60_000.0, 0.10, 80_000.0, 0.15),
        occ(0.10, 80_000.0, 0.20, 80_000.0, 0.15),
        occ(0.25, 30_000.0, 0.45, 20_000.0, 0.20),
        occ(0.30, 30_000.0, 0.50, 20_000.0, 0.20),
    ],
    avec_detecteur: [
        occ(0.10, 60_000.0, 0.10, 80_000.0, 0.10),
        occ(0.10, 80_000.0, 0.20, 80_000.0, 0.10),
        occ(0.25, 20_000.0, 0.45, 20_000.0, 0.15),
        occ(0.30, 20_000.0, 0.50, 20_000.0, 0.15),
    ],
};
/// Figure 40-2 : lignes « dérogation manuelle » de la gestion automatique, stores enroulables, famille « hopitaux ».
#[rustfmt::skip]
pub const DEROGATION_STORE_ENROULABLE_HOPITAUX: Derogation = Derogation {
    sans_detecteur: [
        occ(0.10, 60_000.0, 0.10, 80_000.0, 0.15),
        occ(0.10, 80_000.0, 0.20, 80_000.0, 0.15),
        occ(0.25, 30_000.0, 0.45, 20_000.0, 0.20),
        occ(0.30, 30_000.0, 0.50, 20_000.0, 0.20),
    ],
    avec_detecteur: [
        occ(0.10, 60_000.0, 0.10, 80_000.0, 0.10),
        occ(0.10, 80_000.0, 0.20, 80_000.0, 0.10),
        occ(0.25, 20_000.0, 0.45, 20_000.0, 0.15),
        occ(0.30, 20_000.0, 0.50, 20_000.0, 0.15),
    ],
};
/// Figure 40-3 : lignes « dérogation manuelle » de la gestion automatique, stores enroulables, famille « autres ».
#[rustfmt::skip]
pub const DEROGATION_STORE_ENROULABLE_AUTRES: Derogation = Derogation {
    sans_detecteur: [
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.20),
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.20),
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.20),
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.20),
    ],
    avec_detecteur: [
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.20),
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.20),
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.20),
        occ(0.20, 200_000.0, 0.20, 200_000.0, 0.20),
    ],
};

/// Ligne d'occupation d'un store vénitien en gestion manuelle (figures 41 à 43-3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LigneOccupationSv {
    /// `Rprot` de jour pour `Etp** < Eclim_manu_sv`.
    pub r_prot_sous_seuil: f64,
    /// `Rprot` de jour pour `Etp** ≥ Eclim_manu_sv`.
    pub r_prot_au_dessus: f64,
    /// `Rprot` de nuit.
    pub r_prot_nuit: f64,
    /// « Ecl2 » : éclairement, en lux, à partir duquel la seconde protection est fermée.
    pub ecl2_lux: f64,
}

/// Matrice conventionnelle d'un store vénitien en gestion manuelle, indexée par saison au sens
/// de la gestion des protections mobiles : hiver, mi-saison, été (pas de dédoublement de l'été).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatriceSvManuelle {
    /// Occupation.
    pub occupation: [LigneOccupationSv; 3],
    /// Inoccupation (`jour` ne dépend pas de l'éclairement).
    pub inoccupation: [LigneInoccupation; 3],
}

const fn osv(sous: f64, dessus: f64, nuit: f64, ecl2: f64) -> LigneOccupationSv {
    LigneOccupationSv {
        r_prot_sous_seuil: sous,
        r_prot_au_dessus: dessus,
        r_prot_nuit: nuit,
        ecl2_lux: ecl2,
    }
}

/// Figure 41 (p. 203), famille « habitation, hôtellerie et hébergement ».
pub const SV_HABITATION: MatriceSvManuelle = MatriceSvManuelle {
    occupation: [
        osv(0.20, 0.60, 0.80, 100_000.0),
        osv(0.10, 0.40, 0.80, 80_000.0),
        osv(0.20, 0.70, 0.80, 30_000.0),
    ],
    inoccupation: [inocc(0.40, 0.70), inocc(0.20, 0.70), inocc(0.40, 0.70)],
};
/// Figure 42 (p. 203), famille « bureau ».
pub const SV_BUREAUX: MatriceSvManuelle = MatriceSvManuelle {
    occupation: [
        osv(0.20, 0.60, 0.20, 60_000.0),
        osv(0.10, 0.40, 0.10, 80_000.0),
        osv(0.20, 0.70, 0.20, 40_000.0),
    ],
    inoccupation: [inocc(0.40, 0.40), inocc(0.20, 0.20), inocc(0.40, 0.40)],
};
/// Figure 43 (p. 203), famille « enseignement ».
pub const SV_ENSEIGNEMENT: MatriceSvManuelle = MatriceSvManuelle {
    occupation: SV_BUREAUX.occupation,
    inoccupation: [inocc(0.40, 0.90), inocc(0.20, 0.90), inocc(0.40, 0.90)],
};
/// Figure 43-1 (p. 204), famille « restauration ».
pub const SV_RESTAURATION: MatriceSvManuelle = MatriceSvManuelle {
    occupation: SV_BUREAUX.occupation,
    inoccupation: SV_BUREAUX.inoccupation,
};
/// Figure 43-2 (p. 204), famille « hôpitaux nuit ».
pub const SV_HOPITAUX: MatriceSvManuelle = MatriceSvManuelle {
    occupation: [
        osv(0.20, 0.60, 0.80, 60_000.0),
        osv(0.10, 0.40, 0.80, 80_000.0),
        osv(0.20, 0.70, 0.80, 40_000.0),
    ],
    inoccupation: [inocc(0.40, 0.70), inocc(0.20, 0.70), inocc(0.40, 0.70)],
};
/// Figure 43-3 (p. 204), famille « autres usages ».
pub const SV_AUTRES: MatriceSvManuelle = MatriceSvManuelle {
    occupation: [
        osv(0.20, 0.60, 0.20, 200_000.0),
        osv(0.10, 0.40, 0.10, 200_000.0),
        osv(0.20, 0.70, 0.20, 200_000.0),
    ],
    inoccupation: SV_BUREAUX.inoccupation,
};

/// Matrice des stores vénitiens en gestion manuelle d'une famille (figures 41 à 43-3).
pub fn matrice_sv(famille: FamilleGpm) -> &'static MatriceSvManuelle {
    match famille {
        FamilleGpm::Habitation => &SV_HABITATION,
        FamilleGpm::Bureaux => &SV_BUREAUX,
        FamilleGpm::Enseignement => &SV_ENSEIGNEMENT,
        FamilleGpm::Restauration => &SV_RESTAURATION,
        FamilleGpm::Hopitaux => &SV_HOPITAUX,
        FamilleGpm::Autre => &SV_AUTRES,
    }
}

/// Valeurs d'une ligne d'une matrice de gestion automatique d'un volet ou d'un store enroulable
/// (`Rprot1_auto`, figures 44 et 45) : pour chaque colonne, ratio quand
/// `θop,fin(h−1) > Toph-1limh` (`haute`) et quand `θop,fin(h−1) < Toph-1limb` (`basse`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LigneAuto {
    /// Jour, `Etp** < Eclim_auto`.
    pub jour_sous_seuil: Hysteresis,
    /// Jour, `Etp** ≥ Eclim_auto`.
    pub jour_au_dessus: Hysteresis,
    /// Nuit.
    pub nuit: Hysteresis,
}

/// Les deux valeurs d'une cellule de la matrice `M_Rprot1_auto` (figure 31).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hysteresis {
    /// Valeur pour `Toph-1limh < θop,fin(h−1)`.
    pub haute: f64,
    /// Valeur pour `θop,fin(h−1) < Toph-1limb`.
    pub basse: f64,
}

const fn hy(haute: f64, basse: f64) -> Hysteresis {
    Hysteresis { haute, basse }
}

const fn la(a: Hysteresis, b: Hysteresis, c: Hysteresis) -> LigneAuto {
    LigneAuto {
        jour_sous_seuil: a,
        jour_au_dessus: b,
        nuit: c,
    }
}

/// Matrice `M_Rprot1_auto` : occupation puis inoccupation, indexées hiver, mi-saison, été.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatriceAuto {
    /// Groupe en occupation.
    pub occupation: [LigneAuto; 3],
    /// Groupe en inoccupation.
    pub inoccupation: [LigneAuto; 3],
}

/// Figure 44 (p. 205), « habitation, hôtellerie et hébergement ».
pub const AUTO_RESIDENTIEL: MatriceAuto = MatriceAuto {
    occupation: [
        la(hy(0.0, 0.0), hy(0.0, 0.0), hy(1.0, 1.0)),
        la(hy(0.0, 0.0), hy(0.5, 0.0), hy(1.0, 1.0)),
        la(hy(0.5, 0.0), hy(1.0, 0.75), hy(0.5, 0.9)),
    ],
    inoccupation: [
        la(hy(0.0, 0.0), hy(0.0, 0.0), hy(1.0, 1.0)),
        la(hy(0.0, 0.0), hy(0.5, 0.0), hy(1.0, 1.0)),
        la(hy(1.0, 0.75), hy(1.0, 0.75), hy(0.5, 0.5)),
    ],
};
/// Figure 45 (p. 205), tertiaire.
pub const AUTO_TERTIAIRE: MatriceAuto = MatriceAuto {
    occupation: [
        la(hy(0.0, 0.0), hy(0.0, 0.0), hy(1.0, 1.0)),
        la(hy(0.0, 0.0), hy(0.5, 0.0), hy(1.0, 1.0)),
        la(hy(0.5, 0.0), hy(1.0, 0.75), hy(0.0, 0.0)),
    ],
    inoccupation: [
        la(hy(0.0, 0.0), hy(0.0, 0.0), hy(1.0, 1.0)),
        la(hy(0.0, 0.0), hy(0.5, 0.0), hy(1.0, 1.0)),
        la(hy(1.0, 0.75), hy(1.0, 0.75), hy(0.0, 0.0)),
    ],
};

/// Ligne d'une matrice de gestion automatique d'un store vénitien (`Rprot1_auto_sv`, figures 46
/// et 47) : jour sous le seuil, jour au-dessus, nuit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LigneAutoSv {
    /// Jour, `Etp** < Eclim_auto_sv`.
    pub jour_sous_seuil: f64,
    /// Jour, `Etp** ≥ Eclim_auto_sv`.
    pub jour_au_dessus: f64,
    /// Nuit.
    pub nuit: f64,
}

/// Matrice `M_Rprot1_auto_sv`, indexée hiver, mi-saison, été.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatriceAutoSv {
    /// Groupe en occupation.
    pub occupation: [LigneAutoSv; 3],
    /// Groupe en inoccupation.
    pub inoccupation: [LigneAutoSv; 3],
}

const fn las(a: f64, b: f64, c: f64) -> LigneAutoSv {
    LigneAutoSv {
        jour_sous_seuil: a,
        jour_au_dessus: b,
        nuit: c,
    }
}

/// Figure 46 (p. 205), « habitation, hôtellerie et hébergement ».
pub const AUTO_SV_RESIDENTIEL: MatriceAutoSv = MatriceAutoSv {
    occupation: [las(0.0, 0.0, 1.0), las(0.0, 0.5, 1.0), las(1.0, 1.0, 0.5)],
    inoccupation: [las(0.0, 0.0, 1.0), las(0.0, 0.5, 1.0), las(1.0, 1.0, 0.5)],
};
/// Figure 47 (p. 206), tertiaire.
pub const AUTO_SV_TERTIAIRE: MatriceAutoSv = MatriceAutoSv {
    occupation: [las(0.0, 0.0, 1.0), las(0.0, 0.5, 1.0), las(1.0, 1.0, 0.0)],
    inoccupation: [las(0.0, 0.0, 1.0), las(0.0, 0.5, 1.0), las(1.0, 1.0, 0.0)],
};

/// Matrice conventionnelle de gestion automatique des volets et stores enroulables d'un usage
/// (figures 44 et 45) : « habitation, hôtellerie et hébergement » ou « tertiaire ».
pub fn auto_conventionnelle(famille: FamilleGpm) -> &'static MatriceAuto {
    if famille == FamilleGpm::Habitation {
        &AUTO_RESIDENTIEL
    } else {
        &AUTO_TERTIAIRE
    }
}

/// Matrice conventionnelle de gestion automatique des stores vénitiens d'un usage (figures 46 et
/// 47).
pub fn auto_sv_conventionnelle(famille: FamilleGpm) -> &'static MatriceAutoSv {
    if famille == FamilleGpm::Habitation {
        &AUTO_SV_RESIDENTIEL
    } else {
        &AUTO_SV_TERTIAIRE
    }
}

/// Indice de saison (hiver 0, mi-saison 1, été 2) d'une [`LigneGpm`] (été frais et chaud
/// confondus).
pub fn indice_saison(ligne: LigneGpm) -> usize {
    match ligne {
        LigneGpm::Hiver => 0,
        LigneGpm::MiSaison => 1,
        LigneGpm::EteFrais | LigneGpm::EteChaud => 2,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const FAMILLES: [FamilleGpm; 6] = [
        FamilleGpm::Habitation,
        FamilleGpm::Bureaux,
        FamilleGpm::Enseignement,
        FamilleGpm::Restauration,
        FamilleGpm::Hopitaux,
        FamilleGpm::Autre,
    ];

    #[test]
    fn les_lignes_de_derogation_sont_coherentes() {
        for f in FAMILLES {
            let (v, se) = derogation(f);
            for d in [v, se] {
                for (s, a) in d.sans_detecteur.iter().zip(&d.avec_detecteur) {
                    // « idem avec ou sans détecteur de présence » pour Ecl2.
                    assert_eq!(s.ecl2_lux, a.ecl2_lux);
                    // Le détecteur ne change ni Rprot0 ni Rprot de la seconde protection.
                    assert_eq!(s.r_prot0, a.r_prot0);
                    assert_eq!(s.r_prot_avec_pm2, a.r_prot_avec_pm2);
                    // Avec détecteur, on ferme au même éclairement ou plus tôt.
                    assert!(a.ecl1_lux <= s.ecl1_lux);
                }
            }
        }
    }

    #[test]
    fn valeurs_lues_sur_les_figures() {
        // Figure 35, volets, habitation, été chaud, sans détecteur : 30 %, 40 000 lux, 50 %.
        let l = DEROGATION_VOLET_HABITATION.sans_detecteur[3];
        assert_eq!(
            (l.r_prot0, l.ecl1_lux, l.r_prot_avec_pm2),
            (0.30, 40_000.0, 0.50)
        );
        assert_eq!(l.r_prot_nuit, 0.80);
        // Figure 39, stores enroulables, bureau, avec détecteur, été frais : nuit 15 %.
        let l = DEROGATION_STORE_ENROULABLE_BUREAUX.avec_detecteur[2];
        assert_eq!(
            (l.r_prot0, l.ecl1_lux, l.r_prot_nuit),
            (0.25, 20_000.0, 0.15)
        );
        assert_eq!(l.ecl2_lux, 20_000.0);
        // Figure 42, stores vénitiens, bureau : été, occupation 20 % / 70 % / 20 %, Ecl2 40 000.
        let l = SV_BUREAUX.occupation[2];
        assert_eq!(
            (l.r_prot_sous_seuil, l.r_prot_au_dessus, l.r_prot_nuit),
            (0.2, 0.7, 0.2)
        );
        assert_eq!(l.ecl2_lux, 40_000.0);
        // Figure 47, tertiaire, occupation, été : 100 % / 100 % / 0 % la nuit.
        assert_eq!(AUTO_SV_TERTIAIRE.occupation[2], las(1.0, 1.0, 0.0));
        // Figure 44, été en occupation : 50 % / 0 %, 100 % / 75 %, 50 % / 90 %.
        let l = AUTO_RESIDENTIEL.occupation[2];
        assert_eq!(l.nuit, hy(0.5, 0.9));
    }

    #[test]
    fn les_matrices_automatiques_ont_des_ratios_valides() {
        for m in [&AUTO_RESIDENTIEL, &AUTO_TERTIAIRE] {
            for l in m.occupation.iter().chain(&m.inoccupation) {
                for h in [l.jour_sous_seuil, l.jour_au_dessus, l.nuit] {
                    assert!((0.0..=1.0).contains(&h.haute) && (0.0..=1.0).contains(&h.basse));
                    // Il fait plus chaud quand θ est haute : on ne ferme pas moins.
                    assert!(h.haute >= h.basse || l.nuit == h);
                }
            }
        }
    }
}
