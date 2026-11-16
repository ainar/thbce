// SPDX-License-Identifier: GPL-3.0-or-later
//! Émission équivalente d'un groupe : fiche `C-Emi-Systèmes Emissions du groupe en chaud et en
//! froid` (Th-BCE 2020 §8.1).
//!
//! À partir de la matrice de comportement thermique du groupe (§5.21.3.14 : évolution libre,
//! puis 10 W/m² émis sous forme convective, puis 10 W/m² émis sous forme radiative), la fiche
//! détermine la puissance qu'un émetteur idéal doit fournir pour que la température vue par sa
//! sonde atteigne la consigne, puis les températures du groupe avec cette puissance.
//!
//! Périmètre : émission équivalente conventionnelle du calcul du Bbio (§8.1.3.1) : un émetteur
//! de chauffage et un émetteur de refroidissement, sans variation spatiale ni temporelle, sans
//! pertes au dos, sans ventilateur local, sans relance. Les émetteurs du projet (tableaux 82 à
//! 88, éq. 795 à 799), les ventilateurs locaux (éq. 811 à 823) et l'énergie latente (éq. 828)
//! relèvent du calcul des consommations (lot 6b).
//!
//! Lectures : `docs/questions.md`, Q38.

use crate::bati::comportement_thermique::{Reponse, Temperatures, P_MATRICE_W_M2};

/// Émetteur équivalent de chauffage ou de refroidissement d'un groupe (§8.1.3.2, éq. 798).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmetteurEquivalent {
    /// « Psd_eq » : part de la température de l'air dans la température vue par la sonde du
    /// régulateur, le reste revenant à la température radiante moyenne.
    pub p_sd: f64,
    /// « Pem_eq » : part convective de l'énergie transmise au groupe.
    pub p_em_conv: f64,
    /// « δθvs_eq » : variation spatiale, en K.
    pub delta_theta_vs_k: f64,
    /// « δθvt_eq » : variation temporelle, en K.
    pub delta_theta_vt_k: f64,
    /// « δθpresence_eq_ch » : effet de la détection de présence pièce par pièce sur la
    /// consigne de chauffage, en K (éq. 799) ; nul pour l'émetteur de froid.
    pub delta_theta_presence_k: f64,
}

impl EmetteurEquivalent {
    /// Émission équivalente conventionnelle du calcul du Bbio, en chauffage comme en
    /// refroidissement.
    ///
    /// Th-BCE 2020 §8.1.3.1 : variations spatiale et temporelle de 0 K ; part convective de
    /// 0,5 ; pertes au dos nulles ; ratio de la mesure de température par le régulateur de
    /// 0,5 ; pas de ventilateurs locaux.
    pub const BBIO: Self = Self {
        p_sd: 0.5,
        p_em_conv: 0.5,
        delta_theta_vs_k: 0.0,
        delta_theta_vt_k: 0.0,
        delta_theta_presence_k: 0.0,
    };

    /// Dérive de la température de consigne, `δθvs_eq + δθvt_eq`, en K (éq. 800).
    fn derive_k(&self) -> f64 {
        self.delta_theta_vs_k + self.delta_theta_vt_k
    }

    /// Température vue par la sonde du régulateur, en °C.
    ///
    /// Th-BCE 2020 §8.1.3.3.1, éq. (801), et §8.1.3.4, éq. (805) :
    /// `θsd = Psd·θi,moy + (1 − Psd)·θrm,moy`.
    pub fn theta_sd_c(&self, t: &Temperatures) -> f64 {
        self.p_sd * t.theta_i_c + (1.0 - self.p_sd) * t.theta_rm_c
    }
}

/// Émetteur équivalent sollicité à un pas de temps (§8.1.3.3.2, éq. 802).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sollicitation {
    /// Aucune émission n'est sollicitée.
    Aucune,
    /// L'émetteur équivalent de chauffage est sollicité.
    Chauffage,
    /// L'émetteur équivalent de refroidissement est sollicité.
    Refroidissement,
}

/// Consignes et autorisations d'un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Consignes {
    /// « θiich(h) » : température de consigne de chauffage, en °C.
    pub theta_ii_ch_c: f64,
    /// « θiifr(h) » : température de consigne de refroidissement, en °C.
    pub theta_ii_fr_c: f64,
    /// « Autch,eff(j) » : autorisation effective de chauffer.
    pub aut_ch: bool,
    /// « Autfr,eff(j) » : autorisation effective de refroidir.
    pub aut_fr: bool,
    /// « iclim » de l'éq. (807) : vrai si le groupe peut être refroidi. En calcul du Bbio, vrai
    /// pour tout groupe (décision Q35 point 7 ; Q38 point 3) :
    /// [`crate::scenarios::saisons::refroidissement_possible`].
    pub i_clim: bool,
}

/// Sorties de l'émission pour un groupe à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Emission {
    /// Émetteur équivalent sollicité (éq. 802).
    pub sollicitation: Sollicitation,
    /// « φcrois(h) » : puissance requise par le groupe, en W (éq. 807) ; positive en chauffage,
    /// négative en refroidissement.
    pub phi_crois_w: f64,
    /// « φutil(h) » : puissance effective compte tenu des saisons, en W (éq. 809).
    pub phi_util_w: f64,
    /// « Qsys_ch(h) » : besoin de chauffage du groupe sur le pas de temps, en Wh, positif ou
    /// nul (éq. 810 et 824).
    pub q_sys_ch_wh: f64,
    /// « Qsys_fr(h) » : besoin de refroidissement du groupe sur le pas de temps, en Wh, négatif
    /// ou nul (éq. 810 et 824).
    pub q_sys_fr_wh: f64,
    /// Températures moyennes du groupe sur le pas de temps, émission comprise (éq. 826).
    pub moy: Temperatures,
    /// Températures du groupe à la fin du pas de temps, émission comprise (éq. 825).
    pub fin: Temperatures,
}

/// « φutil−conv » et « φutil−rad », rapportés à la puissance des cas de la matrice.
///
/// Th-BCE 2020 §8.1.3.9, éq. (825) à (827) : chaque température est
/// `θ(0,0) + φutil−conv/10·(θ(10,0) − θ(0,0)) + φutil−rad/10·(θ(0,10) − θ(0,0))`, avec
/// `φutil−conv = Pemconv·φutil` et `φutil−rad = (1 − Pemconv)·φutil`. Le « 10 » est la puissance
/// des cas de la matrice ([`puissance_matrice_w`]).
fn interpoler(
    libre: &Temperatures,
    conv: &Temperatures,
    rad: &Temperatures,
    k_conv: f64,
    k_rad: f64,
    derive_op_k: f64,
) -> Temperatures {
    let f = |l: f64, c: f64, r: f64| l + k_conv * (c - l) + k_rad * (r - l);
    Temperatures {
        theta_i_c: f(libre.theta_i_c, conv.theta_i_c, rad.theta_i_c),
        theta_s_c: f(libre.theta_s_c, conv.theta_s_c, rad.theta_s_c),
        theta_mq_c: f(libre.theta_mq_c, conv.theta_mq_c, rad.theta_mq_c),
        theta_rm_c: f(libre.theta_rm_c, conv.theta_rm_c, rad.theta_rm_c),
        theta_op_c: f(libre.theta_op_c, conv.theta_op_c, rad.theta_op_c) - derive_op_k,
    }
}

/// Puissance des cas 2 et 3 de la matrice de comportement, en W : `10 W/m² × Agr`.
///
/// Th-BCE 2020 §5.21.3.14 et §8.1.3.4 : « φconv = Pemconv × 10 × Agr et
/// φrad = (1 − Pemconv) × 10 × Agr ». Les éq. (806) et (825) écrivent « 10 » sans surface, ce
/// qui donnerait une puissance surfacique, alors que les figures 87 et 88 graduent `φsys` en W,
/// que l'éq. (810) donne `Qsys` en Wh sans multiplier par `Agr`, et que les éq. (71) et (2379)
/// divisent `Qsys` par une surface. Lecture retenue (Q38 point 2) : `φcrois` et `φutil` sont
/// des puissances du groupe, en W ; le « 10 » des éq. (806), (825) et (826) est `10 × Agr`.
pub fn puissance_matrice_w(a_gr_m2: f64) -> f64 {
    P_MATRICE_W_M2 * a_gr_m2
}

/// Besoins et températures d'un groupe pour un émetteur équivalent de chauffage et un émetteur
/// équivalent de refroidissement.
///
/// Th-BCE 2020 §8.1.3.3 à 8.1.3.9, sans relance, sans détection de présence, sans brasseur
/// d'air et sans pertes au dos (`Ratem_eff = 1`, `Pper = 0`) :
/// - éq. (800) : `θi_eq_ch = θiich + δθvs_eq_ch + δθvt_eq_ch`, de même en refroidissement ;
/// - éq. (801) : températures vues par les sondes des deux émetteurs, en évolution libre ;
/// - éq. (802) : chauffage sollicité si `θsd_eq_ch < θi_eq_ch` ; sinon refroidissement
///   sollicité si `θsd_eq_fr > θi_eq_fr` ; le chauffage est prioritaire ;
/// - éq. (804) et (805) : `θsd_0` en évolution libre et `θsd_10` pour 10 W/m² émis avec la part
///   convective de l'émetteur sollicité ;
/// - éq. (806) : droite du groupe, `a1 = P/(θsd_10 − θsd_0)` et `a0 = −a1·θsd_0`, avec
///   `P = 10 × Agr` ([`puissance_matrice_w`]) ;
/// - éq. (807) : `φcrois = a0 + a1·θic` ; si le groupe ne peut pas être refroidi,
///   `max(φcrois ; 0)` ;
/// - éq. (809) : `φutil = φcrois` si l'émetteur sollicité est autorisé par la saison, 0 sinon ;
/// - éq. (810) et (824) : `Qsys_ch = max(0 ; φutil)` et `Qsys_fr = min(0 ; φutil)`, en Wh sur
///   le pas de temps d'une heure ;
/// - éq. (825) à (827) : températures finales et moyennes par interpolation dans la matrice ;
///   la température opérative est diminuée de la dérive de l'émetteur en fonction.
///
/// `matrice` est la sortie de
/// [`crate::bati::comportement_thermique::GroupeThermique::matrice`].
pub fn emission(
    matrice: &[Reponse; 3],
    a_gr_m2: f64,
    chaud: &EmetteurEquivalent,
    froid: &EmetteurEquivalent,
    c: &Consignes,
) -> Emission {
    emission_avec_relance(
        matrice,
        a_gr_m2,
        chaud,
        froid,
        c,
        &ConsignesRelance::SANS_RELANCE,
    )
}

/// Consignes de relance d'un pas de temps (§8.5, éq. 845 à 847).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConsignesRelance {
    /// « θiich_relance(h) », en °C ; 0 °C hors relance.
    pub theta_ii_ch_relance_c: f64,
    /// « θiifr_relance(h) », en °C ; 100 °C hors relance.
    pub theta_ii_fr_relance_c: f64,
}

impl ConsignesRelance {
    /// Hors relance (valeurs du §8.5.3, éq. 845) : sans effet sur les consignes de l'éq. (800).
    pub const SANS_RELANCE: Self = Self {
        theta_ii_ch_relance_c: 0.0,
        theta_ii_fr_relance_c: 100.0,
    };
}

/// Comme [`emission`], avec les consignes de relance du calcul des consommations.
///
/// Th-BCE 2020 §8.1.3.3.1, éq. (800) : `θi_eq_ch = MAX(θiich ; θiich_relance) + δθvs_eq_ch +
/// δθvt_eq_ch + δθpresence_eq_ch` et `θi_eq_fr = MIN(θiifr ; θiifr_relance) + δθvs_eq_fr +
/// δθvt_eq_fr` (brasseurs d'air hors de la première version : `δθconsfr_BA = 0`).
pub fn emission_avec_relance(
    matrice: &[Reponse; 3],
    a_gr_m2: f64,
    chaud: &EmetteurEquivalent,
    froid: &EmetteurEquivalent,
    c: &Consignes,
    relance: &ConsignesRelance,
) -> Emission {
    let [libre, conv, rad] = matrice;
    // Éq. (800) et (801).
    let theta_i_eq_ch = c.theta_ii_ch_c.max(relance.theta_ii_ch_relance_c)
        + chaud.derive_k()
        + chaud.delta_theta_presence_k;
    let theta_i_eq_fr = c.theta_ii_fr_c.min(relance.theta_ii_fr_relance_c) + froid.derive_k();
    let theta_sd_eq_ch = chaud.theta_sd_c(&libre.moy);
    let theta_sd_eq_fr = froid.theta_sd_c(&libre.moy);
    let besoin_ch = theta_sd_eq_ch < theta_i_eq_ch;
    let besoin_fr = theta_sd_eq_fr > theta_i_eq_fr;
    // Éq. (802).
    let (sollicitation, emetteur, theta_ic) = if besoin_ch {
        (Sollicitation::Chauffage, chaud, theta_i_eq_ch)
    } else if besoin_fr {
        (Sollicitation::Refroidissement, froid, theta_i_eq_fr)
    } else {
        return Emission {
            sollicitation: Sollicitation::Aucune,
            phi_crois_w: 0.0,
            phi_util_w: 0.0,
            q_sys_ch_wh: 0.0,
            q_sys_fr_wh: 0.0,
            moy: libre.moy,
            fin: libre.fin,
        };
    };
    // Éq. (804) et (805).
    let p_em = emetteur.p_em_conv;
    let melange = |a: f64, b: f64| p_em * a + (1.0 - p_em) * b;
    let theta_sd_0 = emetteur.theta_sd_c(&libre.moy);
    let theta_sd_10 = emetteur.p_sd * melange(conv.moy.theta_i_c, rad.moy.theta_i_c)
        + (1.0 - emetteur.p_sd) * melange(conv.moy.theta_rm_c, rad.moy.theta_rm_c);
    // Éq. (806) et (807).
    let p_w = puissance_matrice_w(a_gr_m2);
    let a1 = p_w / (theta_sd_10 - theta_sd_0);
    let a0 = -a1 * theta_sd_0;
    let mut phi_crois_w = a0 + a1 * theta_ic;
    if !c.i_clim {
        phi_crois_w = phi_crois_w.max(0.0);
    }
    // Éq. (809).
    let effective = (besoin_fr && c.aut_fr) || (besoin_ch && c.aut_ch);
    let phi_util_w = if effective { phi_crois_w } else { 0.0 };
    // Éq. (825) à (827). Les dérives de consigne sont celles de l'émetteur en fonction.
    let (k_conv, k_rad) = (p_em * phi_util_w / p_w, (1.0 - p_em) * phi_util_w / p_w);
    let derive_op_k = if phi_util_w == 0.0 {
        0.0
    } else {
        emetteur.derive_k()
    };
    Emission {
        sollicitation,
        phi_crois_w,
        phi_util_w,
        // Éq. (810) et (824).
        q_sys_ch_wh: phi_util_w.max(0.0),
        q_sys_fr_wh: phi_util_w.min(0.0),
        moy: interpoler(&libre.moy, &conv.moy, &rad.moy, k_conv, k_rad, derive_op_k),
        fin: interpoler(&libre.fin, &conv.fin, &rad.fin, k_conv, k_rad, derive_op_k),
    }
}

/// Besoins et températures d'un groupe avec l'émission équivalente conventionnelle du calcul du
/// Bbio (§8.1.3.1) : voir [`emission`] et [`EmetteurEquivalent::BBIO`].
pub fn emission_bbio(matrice: &[Reponse; 3], a_gr_m2: f64, c: &Consignes) -> Emission {
    emission(
        matrice,
        a_gr_m2,
        &EmetteurEquivalent::BBIO,
        &EmetteurEquivalent::BBIO,
        c,
    )
}
