// SPDX-License-Identifier: GPL-3.0-or-later
//! Débits aux bouches d'extraction et de soufflage : fiche `C_VEN_Bouche_conduit`
//! (Th-BCE 2020 §6.2).
//!
//! Périmètre : ce qui sert au calcul du Bbio en habitation (maison individuelle ou accolée,
//! logement collectif) :
//! - débits maximal et minimal d'une bouche (§6.2.3.1.1.2, éq. 387 à 390, auxquelles renvoie le
//!   §6.2.3.1.3, « cas du Bbio ») ;
//! - débit de base d'un système hygroréglable (§6.2.3.1.1.2, formule non numérotée) ;
//! - débit régulé (§6.2.3.2.2.2, éq. 403), avec `Crdbnr = 1` (§6.2.3.2.1) et `Dugd = 14` h par
//!   semaine (tableau de la durée d'utilisation du grand débit, ligne « mode BBIO ») ;
//! - coefficient de dépassement (§6.2.3.3.1, éq. 412 à 414) : `Cdep = 1` en Bbio ;
//! - fuites des réseaux (§6.2.3.4, éq. 419, 420, 428 à 431) : `Kres = 0` en Bbio (tableau 59).
//!
//! Les autres usages, les centrales de traitement d'air, les valeurs de `Crdbnr`, de `Cdep` et
//! de `Kres` du calcul des consommations et les surfaces de conduits viendront au lot 6a.
//!
//! Convention de signe de la fiche : un débit repris est négatif, un débit soufflé positif.

use crate::entree::{BoucheConduit, Isouf};

/// Nombre d'heures d'une semaine (éq. 403).
pub const NB_HEURES_SEMAINE: f64 = 168.0;
/// « Dugd » : durée d'utilisation du grand débit en mode Bbio, en h par semaine (§6.2.3.2.2.2,
/// tableau de la durée d'utilisation du grand débit ; note 1 : « Pour le calcul du BBIO,
/// Dugd = 14 »).
pub const DUGD_BBIO_H: f64 = 14.0;
/// « Crdbnr » : coefficient de réduction des débits dans le cas du Bbio (§6.2.3.2.1 : « Crdbnr
/// = 1 quelle que soit l'occupation et l'usage du groupe »).
pub const CRDBNR_BBIO: f64 = 1.0;
/// « CdepBBio » : coefficient de dépassement du calcul du Bbio (§6.2.3.3.1, éq. 414).
pub const CDEP_BBIO: f64 = 1.0;
/// « Kres » : coefficient de fuite du réseau, en m³/(s.m²) sous 1 Pa, dans le « cas de
/// l'aération et du BBio » (§6.2.3.4, tableau 59).
pub const K_RES_BBIO: f64 = 0.0;

/// Débits repris et soufflé d'une bouche ou d'un groupe, en m³/h. Le débit repris est négatif
/// ou nul, le débit soufflé positif ou nul.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Debits {
    /// Débit repris, en m³/h (négatif ou nul).
    pub q_rep_m3_h: f64,
    /// Débit soufflé, en m³/h (positif ou nul).
    pub q_souffle_m3_h: f64,
}

impl Debits {
    fn de(isouf: Isouf, q_m3_h: f64) -> Self {
        match isouf {
            Isouf::Reprise => Self {
                q_rep_m3_h: -q_m3_h,
                q_souffle_m3_h: 0.0,
            },
            Isouf::Soufflage => Self {
                q_rep_m3_h: 0.0,
                q_souffle_m3_h: q_m3_h,
            },
        }
    }

    fn fois(self, k: f64) -> Self {
        Self {
            q_rep_m3_h: k * self.q_rep_m3_h,
            q_souffle_m3_h: k * self.q_souffle_m3_h,
        }
    }

    fn plus(self, autre: Self) -> Self {
        Self {
            q_rep_m3_h: self.q_rep_m3_h + autre.q_rep_m3_h,
            q_souffle_m3_h: self.q_souffle_m3_h + autre.q_souffle_m3_h,
        }
    }
}

/// « qrep,max » et « qsoufflé,max » : débit maximal de la bouche, en habitation.
///
/// Th-BCE 2020 §6.2.3.1.1.2, éq. (387) et (388) : si `Isouf = 0`,
/// `qrep,max = −qspec,rep,conv_pointe` et `qsoufflé,max = 0` ; si `Isouf = 1`,
/// `qsoufflé,max = qspec,soufflé,conv_pointe` et `qrep,max = 0`.
pub fn debits_max_habitation(isouf: Isouf, q_spec_conv_pointe_m3_h: f64) -> Debits {
    Debits::de(isouf, q_spec_conv_pointe_m3_h)
}

/// « qrep,min » et « qsoufflé,min » : débit minimal de la bouche, en habitation.
///
/// Th-BCE 2020 §6.2.3.1.1.2, éq. (389) et (390) : si `Isouf = 0`,
/// `qrep,min = −qspec,rep,conv_base` et `qsoufflé,min = 0` ; si `Isouf = 1`,
/// `qsoufflé,min = qspec,soufflé,conv_base` et `qrep,min = 0`.
pub fn debits_min_habitation(isouf: Isouf, q_spec_conv_base_m3_h: f64) -> Debits {
    Debits::de(isouf, q_spec_conv_base_m3_h)
}

/// « qspec,rep,conv_base » d'un système hygroréglable, en m³/h.
///
/// Th-BCE 2020 §6.2.3.1.1.2, « cas des systèmes hygroréglables » (formule non numérotée) :
/// `qspec,rep,conv_base = (168 × Qvarepspec − Dugd × qspec,rep,conv_pointe)/(168 − Dugd)`,
/// `Qvarepspec` étant la valeur de l'avis technique et `Dugd` la durée du §6.2.3.2.2.2. « Le
/// raisonnement est le même pour les circuits de soufflage. »
pub fn q_spec_conv_base_hygroreglable_m3_h(
    qvarep_spec_m3_h: f64,
    q_spec_conv_pointe_m3_h: f64,
    dugd_h: f64,
) -> f64 {
    (NB_HEURES_SEMAINE * qvarep_spec_m3_h - dugd_h * q_spec_conv_pointe_m3_h)
        / (NB_HEURES_SEMAINE - dugd_h)
}

/// « qrep,regul » et « qsoufflé,regul » : débits régulés, en habitation.
///
/// Th-BCE 2020 §6.2.3.2.2.2, éq. (403), quelle que soit l'occupation :
/// `q_regul = Crdbnr × (q_max × Dugd + q_min × (168 − Dugd))/168`.
pub fn debits_regules_habitation(crdbnr: f64, max: Debits, min: Debits, dugd_h: f64) -> Debits {
    max.fois(dugd_h)
        .plus(min.fois(NB_HEURES_SEMAINE - dugd_h))
        .fois(crdbnr / NB_HEURES_SEMAINE)
}

/// « qrep,dep » et « qsoufflé,dep » : débits après application du coefficient de dépassement.
///
/// Th-BCE 2020 §6.2.3.3.1, éq. (412) et (413) : `q_dep = Cdep × q_regul`.
pub fn debits_avec_depassement(cdep: f64, regul: Debits) -> Debits {
    regul.fois(cdep)
}

/// « qspec,rep,conv_base » ou « qspec,soufflé,conv_base » de la bouche, en m³/h : la valeur
/// saisie ou, pour un système hygroréglable, celle que donne la formule du §6.2.3.1.1.2 avec la
/// durée d'utilisation du grand débit du calcul en cours.
///
/// Lecture (`docs/questions.md`, Q36 point 3) : le débit de base d'un système hygroréglable est
/// déduit avec le même `Dugd` que celui de l'éq. (403), de sorte que le débit régulé est
/// exactement `Qvarepspec`, le « débit énergétique équivalent » de la note 4 du §6.2.3.2.2.2.
/// `None` si la bouche ne donne ni débit de base ni `Qvarepspec` (refusé par la validation).
pub fn q_spec_conv_base_m3_h(bouche: &BoucheConduit, dugd_h: f64) -> Option<f64> {
    bouche.q_spec_conv_base_m3_h.or_else(|| {
        bouche
            .qvarep_spec_m3_h
            .map(|q| q_spec_conv_base_hygroreglable_m3_h(q, bouche.q_spec_conv_pointe_m3_h, dugd_h))
    })
}

/// « qspec_repris » et « qspec_soufflé » d'une bouche pour le calcul du Bbio en habitation, en
/// m³/h.
///
/// Th-BCE 2020 §6.2.3.1.3 (débits maximal et minimal, éq. 387 à 390), §6.2.3.2.1 et
/// §6.2.3.2.2.2 (éq. 403, `Crdbnr = 1`, `Dugd = 14`), §6.2.3.3.1 (éq. 412 à 414, `Cdep = 1`),
/// §6.2.3.4 (éq. 428 et 429 avec `Kres = 0` : les fuites des réseaux sont nulles, et
/// `q_cond = q_dep`, éq. 430 et 431).
pub fn debits_specifiques_bbio_habitation(bouche: &BoucheConduit) -> Option<Debits> {
    let base = q_spec_conv_base_m3_h(bouche, DUGD_BBIO_H)?;
    let max = debits_max_habitation(bouche.isouf, bouche.q_spec_conv_pointe_m3_h);
    let min = debits_min_habitation(bouche.isouf, base);
    let regul = debits_regules_habitation(CRDBNR_BBIO, max, min, DUGD_BBIO_H);
    Some(debits_avec_depassement(CDEP_BBIO, regul))
}

/// Somme, sur les bouches d'un groupe, des débits spécifiques du calcul du Bbio en habitation.
///
/// Th-BCE 2020 §6.2.3 : « Les débits volumiques sont définis au niveau du groupe. »
pub fn debits_specifiques_bbio_groupe(bouches: &[BoucheConduit]) -> Option<Debits> {
    bouches.iter().try_fold(Debits::default(), |somme, b| {
        debits_specifiques_bbio_habitation(b).map(|d| somme.plus(d))
    })
}
