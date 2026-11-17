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
//! Calcul des consommations et du confort d'été (lot 6a), en habitation, pour les systèmes
//! mécaniques (simple flux, basse pression et double flux) :
//! - débit régulé avec la durée d'utilisation du grand débit de la bouche (éq. 403, tableau 57) ;
//! - coefficient de dépassement de la bouche (éq. 412 et 413, tableau 58) ;
//! - fuites des réseaux (éq. 419 et 420, tableau 59), surfaces de conduits saisies ou par défaut
//!   (éq. 421 à 427, tableau 60) et débits spécifiques et vus par le ventilateur (éq. 428 à 431).
//!
//! Les autres usages et les centrales de traitement d'air (éq. 383 à 386, 391 à 402, 404 à 411,
//! 415 à 418, 432 à 439) restent hors de la première version.
//!
//! Convention de signe de la fiche : un débit repris est négatif, un débit soufflé positif.

use crate::entree::{BoucheConduit, ClasseEtancheite, Isouf};

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

/// « Dugd » par défaut d'une bouche de ventilation mécanique, en h par semaine : dispositif à
/// gestion manuelle, « par défaut » (§6.2.3.2.2.2, tableau 57).
pub const DUGD_DEFAUT_H: f64 = 14.0;
/// « Crdbnr » en habitation, quelle que soit l'occupation (§6.2.3.2.2.2).
pub const CRDBNR_HABITATION: f64 = 1.0;
/// « Cdep » par défaut (§6.2.3.3.1, tableau 58).
pub const CDEP_DEFAUT: f64 = 1.30;
/// « Kres » de la classe d'étanchéité par défaut, en m³/(s.m²) sous 1 Pa (tableau 59, « DEF »).
pub const K_RES_DEFAUT: f64 = 0.0675e-3;
/// Exposant de la différence de pression dans les éq. (419) et (420).
pub const EXPOSANT_FUITES: f64 = 0.667;

/// « Kres » : coefficient de fuite du réseau, en m³/(s.m²) sous 1 Pa (§6.2.3.4, tableau 59).
/// `None` : valeur par défaut « DEF ».
pub fn k_res(classe: Option<ClasseEtancheite>) -> f64 {
    match classe {
        Some(ClasseEtancheite::A) => 0.027e-3,
        Some(ClasseEtancheite::B) => 0.009e-3,
        Some(ClasseEtancheite::C) => 0.003e-3,
        None => K_RES_DEFAUT,
    }
}

/// Typologie du bâtiment au sens du tableau 60.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypologieReseau {
    /// Maison individuelle (ou accolée).
    MaisonIndividuelle,
    /// Bâtiment collectif (logement collectif).
    BatimentCollectif,
    /// Bâtiment non résidentiel.
    NonResidentiel,
}

/// Pression de fonctionnement d'un système au sens du tableau 60.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressionReseau {
    /// « Haute pression » : systèmes mécaniques simple flux, double flux.
    Haute,
    /// « Basse pression ».
    Basse,
}

impl TypologieReseau {
    /// « Ratfuitevc » par défaut (tableau 60).
    pub fn ratfuitevc_defaut(self) -> f64 {
        match self {
            Self::MaisonIndividuelle => 0.25,
            Self::BatimentCollectif => 0.5,
            Self::NonResidentiel => 0.75,
        }
    }

    /// « dP » conventionnelle du réseau, en Pa (tableau 60).
    pub fn d_p_pa(self, pression: PressionReseau) -> f64 {
        match (self, pression) {
            (Self::MaisonIndividuelle, PressionReseau::Haute) => 80.0,
            (Self::BatimentCollectif, PressionReseau::Haute) => 160.0,
            (Self::NonResidentiel, PressionReseau::Haute) => 250.0,
            // Le tableau 60 ne donne pas de pression en basse pression au bâtiment non
            // résidentiel ; les systèmes à basse pression n'y sont pas pris en charge.
            (_, PressionReseau::Basse) => 20.0,
        }
    }
}

/// « Ratsurfcond » : surface de conduits par m² de surface habitable en maison individuelle
/// (tableau 60).
pub const RAT_SURF_COND: f64 = 0.05;
/// « Ratdebcond » : surface de conduits par m³/h de débit maximal en bâtiment collectif, en
/// m².h/m³ (tableau 60).
pub const RAT_DEB_COND: f64 = 0.05;

/// « Acond » par défaut d'un groupe en maison individuelle, en m².
///
/// Th-BCE 2020 §6.2.3.4, éq. (421) : `Acond = SHAB × Ratsurfcond`. En double flux, la même
/// surface s'applique au circuit de reprise et au circuit de soufflage (éq. 422 et 423).
pub fn a_cond_defaut_maison_m2(shab_m2: f64) -> f64 {
    shab_m2 * RAT_SURF_COND
}

/// « Acond,rep » ou « Acond,soufflé » par défaut d'une bouche en bâtiment collectif, en m².
///
/// Th-BCE 2020 §6.2.3.4, éq. (424) et (425) : `abs(qmax) × Ratdebcond`. Pour la VMC double
/// flux, les éq. (426) et (427) emploient le débit maximal « en mode actif » des CTA, qui n'a
/// pas d'objet en habitation : le débit maximal de la bouche le remplace (`docs/questions.md`,
/// Q42).
pub fn a_cond_defaut_collectif_m2(q_max_m3_h: f64) -> f64 {
    q_max_m3_h.abs() * RAT_DEB_COND
}

/// « qrepris,fuites » ou « qsoufflé,fuites » : fuites totales du conduit, en m³/h, de même signe
/// que le débit du circuit (négatif en reprise).
///
/// Th-BCE 2020 §6.2.3.4, éq. (419) et (420) : `∓3600 × Kres × Acond × dP^0,667`, avec `dP = 0`
/// quand le débit après dépassement est nul.
pub fn q_fuites_m3_h(
    isouf: Isouf,
    q_dep_m3_h: f64,
    k_res: f64,
    a_cond_m2: f64,
    d_p_pa: f64,
) -> f64 {
    if q_dep_m3_h == 0.0 {
        return 0.0;
    }
    let q = 3600.0 * k_res * a_cond_m2 * libm::pow(d_p_pa, EXPOSANT_FUITES);
    match isouf {
        Isouf::Reprise => -q,
        Isouf::Soufflage => q,
    }
}

/// Débits d'une bouche et de son conduit pour le calcul des consommations et du confort d'été,
/// en m³/h, signés comme dans la fiche (reprise négative, soufflage positif).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DebitsBoucheProjet {
    /// Sens du circuit.
    pub isouf: Isouf,
    /// « qrep,max » ou « qsoufflé,max » (éq. 387 et 388).
    pub q_max_m3_h: f64,
    /// « qspec,rep,conv_base » ou « qspec,soufflé,conv_base », valeur absolue (éq. 389 et 390,
    /// formule des systèmes hygroréglables).
    pub q_base_m3_h: f64,
    /// « qrep,dep » ou « qsoufflé,dep » (éq. 412 et 413).
    pub q_dep_m3_h: f64,
    /// « qrepris,fuites » ou « qsoufflé,fuites » (éq. 419 et 420).
    pub q_fuites_m3_h: f64,
    /// « Ratfuitevc ».
    pub ratfuitevc: f64,
    /// « Acond » : surface externe du conduit, en m².
    pub a_cond_m2: f64,
}

impl DebitsBoucheProjet {
    /// « qspec_repris » ou « qspec_soufflé » : débit spécifique reçu ou fourni par le groupe,
    /// fuites en volume chauffé comprises (éq. 428 et 429).
    pub fn q_spec_m3_h(&self) -> f64 {
        self.q_dep_m3_h + self.ratfuitevc * self.q_fuites_m3_h
    }

    /// « qrepris,cond » ou « qsoufflé,cond » : débit vu par le ventilateur (éq. 430 et 431).
    pub fn q_cond_m3_h(&self) -> f64 {
        self.q_dep_m3_h + self.q_fuites_m3_h
    }

    /// « qrep,fuite,hvc » ou « qsou,fuite,hvc » : fuites hors volume chauffé, en m³/h
    /// (nomenclature du §6.3.2) : la part des fuites que les éq. (428) et (429) n'affectent pas
    /// au groupe, `(1 − Ratfuitevc) × qfuites`.
    pub fn q_fuite_hvc_m3_h(&self) -> f64 {
        (1.0 - self.ratfuitevc) * self.q_fuites_m3_h
    }

    /// « Acond,rep,ext » ou « Acond,souf,ext » : surface du conduit hors volume chauffé, en m²
    /// (éq. 626, 651 et 667).
    pub fn a_cond_ext_m2(&self) -> f64 {
        (1.0 - self.ratfuitevc) * self.a_cond_m2
    }
}

// ---------------------------------------------------------------------------------------------
// Usages autres que la maison individuelle ou accolée et le logement collectif.
// ---------------------------------------------------------------------------------------------

/// « Crdbnr » sans régulation des débits (§6.2.3.2.1, tableau 57).
pub const CRDBNR_DEFAUT: f64 = 1.0;
/// Débit minimal conventionnel du Bbio en inoccupation, en m³/h (§6.2.3.1.3 :
/// `max(60 ; Param_inoccupation × SREF)`).
pub const Q_MIN_INOCCUPATION_BBIO_M3_H: f64 = 60.0;

/// « Param_occupation » et « Param_inoccupation » du calcul du Bbio, en m³/h par m² de surface
/// de référence, selon le numéro d'usage (§6.2.3.1.3, tableau 56-1, p. 334 et 335). `None`
/// pour les usages 1 et 2, dont les débits du Bbio sont les débits d'hygiène des bouches.
pub fn param_debits_bbio(numero_usage: u8) -> Option<(f64, f64)> {
    Some(match numero_usage {
        3 => (4.0, 0.42),
        4 => (7.2, 0.38),
        5 => (4.7, 0.38),
        6 => (5.6, 0.46),
        7 => (7.4, 0.6),
        8 => (3.0, 3.0),
        9 => (2.0, 2.2),
        10 => (8.0, 0.0),
        11 => (7.0, 0.0),
        12 => (4.7, 0.75),
        13..=16 => (10.0, 0.5),
        17 => (3.7, 0.0),
        18 => (6.7, 6.7),
        19 | 20 => (4.5, 4.5),
        21 => (4.5, 0.0),
        22 => (4.0, 0.3),
        23 | 24 => (3.1, 0.0),
        25 | 28 => (3.0, 0.0),
        26 | 27 => (8.0, 0.3),
        _ => return None,
    })
}

/// Débits conventionnels du calcul du Bbio d'un groupe hors habitation, en m³/h : en occupation
/// puis en inoccupation au sens de la ventilation (`Ivent`).
///
/// Th-BCE 2020 §6.2.3.1.3 : en occupation, `q_max = Param_occupation × SREF_gr` ; en
/// inoccupation, `q_min = max(60 ; Param_inoccupation × SREF_gr)` ; §6.2.3.2.1 et §6.2.3.2.2.1 :
/// `Crdbnr = 1`, le débit régulé vaut `q_max` en occupation et `q_min` en inoccupation ;
/// `Cdep = 1` (éq. 414) et fuites nulles (tableau 59).
///
/// Le texte écrit ces débits « pour chaque bouche », en reprise si `Isouf = 0` et en soufflage
/// si `Isouf = 1`. Ils ne dépendent pourtant que de la surface du groupe : ils sont comptés une
/// fois par groupe (lecture Q60), et le système conventionnel du Bbio, à débits soufflé et
/// extrait égaux (§6.1.3, décision Q36 point 1), reprend et souffle ce débit.
pub fn debits_bbio_non_residentiel(numero_usage: u8, sref_gr_m2: f64) -> Option<(Debits, Debits)> {
    let (param_occ, param_inocc) = param_debits_bbio(numero_usage)?;
    let symetrique = |q: f64| Debits {
        q_rep_m3_h: -q,
        q_souffle_m3_h: q,
    };
    let q_occ = CDEP_BBIO * CRDBNR_BBIO * param_occ * sref_gr_m2;
    let q_inocc = CDEP_BBIO * Q_MIN_INOCCUPATION_BBIO_M3_H.max(param_inocc * sref_gr_m2);
    Some((symetrique(q_occ), symetrique(q_inocc)))
}

/// Débits d'une bouche hors habitation pour le calcul des consommations et du confort d'été :
/// débit maximal, puis débits après dépassement en occupation et en inoccupation, en m³/h,
/// signés (reprise négative).
///
/// Th-BCE 2020 §6.2.3.1.1.1, éq. (383) à (386) : `q_max = ∓qspec,occ`, `q_min = ∓qspec,inocc` ;
/// §6.2.3.2.2.1, éq. (400) à (402) : en occupation (`Ivent` vrai), `q_regul = Crdbnr × q_max` ;
/// en inoccupation, `Crdbnr = 1` et `q_regul = q_min` ; §6.2.3.3.1, éq. (412) et (413) :
/// `q_dep = Cdep × q_regul`.
pub fn debits_dep_non_residentiel(
    isouf: Isouf,
    q_spec_occ_m3_h: f64,
    q_spec_inocc_m3_h: f64,
    crdbnr: f64,
    cdep: f64,
) -> (f64, f64, f64) {
    let signe = |d: Debits| d.q_rep_m3_h + d.q_souffle_m3_h;
    let max = Debits::de(isouf, q_spec_occ_m3_h);
    let min = Debits::de(isouf, q_spec_inocc_m3_h);
    (
        signe(max),
        signe(debits_avec_depassement(cdep, max.fois(crdbnr))),
        signe(debits_avec_depassement(cdep, min)),
    )
}

/// Débits maximal, de base et après dépassement d'une bouche en habitation, pour le calcul
/// des consommations et du confort d'été.
///
/// Th-BCE 2020 §6.2.3.1.1.2 (éq. 387 à 390 et formule des systèmes hygroréglables avec le `Dugd`
/// de la bouche, lecture Q36 point 3), §6.2.3.2.2.2 (éq. 403, `Crdbnr = 1`) et §6.2.3.3.1
/// (éq. 412 et 413). Renvoie `(qmax, qbase, qdep)` en valeurs signées, `None` si la bouche ne
/// donne ni débit de base ni `Qvarepspec`.
pub fn debits_dep_habitation(
    bouche: &BoucheConduit,
    dugd_h: f64,
    cdep: f64,
) -> Option<(f64, f64, f64)> {
    let base = q_spec_conv_base_m3_h(bouche, dugd_h)?;
    let max = debits_max_habitation(bouche.isouf, bouche.q_spec_conv_pointe_m3_h);
    let min = debits_min_habitation(bouche.isouf, base);
    let regul = debits_regules_habitation(CRDBNR_HABITATION, max, min, dugd_h);
    let dep = debits_avec_depassement(cdep, regul);
    let signe = |d: Debits| d.q_rep_m3_h + d.q_souffle_m3_h;
    Some((signe(max), base, signe(dep)))
}
