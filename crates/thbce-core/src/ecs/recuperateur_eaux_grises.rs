// SPDX-License-Identifier: GPL-3.0-or-later
//! Récupérateur de chaleur sur les eaux grises : fiche `C_BAT_Récupérateur de chaleur sur les
//! eaux grises` (Th-BCE 2020 §9.21) : émetteur équivalent (§9.21.2), ajout à l'assemblage de
//! ballon (§9.21.3, `θentrant`), distributions d'évacuation du groupe et intergroupe (§9.21.4 et
//! §9.21.5), échangeur instantané (§9.21.6) et pompe de relevage (§9.21.7, ajouts COMETH).
//!
//! Lectures retenues (Q108) : `Rat` est la part des besoins dédiée aux douches (éq. 2161
//! corrigée), les volumes d'un émetteur se répartissent sur ses appareils pondérés par le nombre
//! d'échangeurs, la part hors douches garde l'eau froide non préchauffée en position 2, et
//! `θec = 55 °C`. Les variantes sont portées par [`Lecture`], jamais exposées dans le format.

use crate::entree::{
    EchangeurEauxGrises, OrigineEfficaciteEauxGrises, PositionRecuperateur, RecuperateurEauxGrises,
};

use super::besoins_ecs::{C_W_WH_KG_K, RHO_W_KG_L, THETA_UW_C};

/// « debitnominal,douches-bains » : débit nominal d'une douche, en L/h (tableau 299, 8 L/min).
pub const DEBIT_NOMINAL_DOUCHES_L_H: f64 = 480.0;
/// « Δtnominal,douches-bains » : durée nominale d'une douche, en h (tableau 299, 5 min).
pub const DUREE_NOMINALE_DOUCHES_H: f64 = 0.083;
/// « Δθutil,douches-bains » (éq. 2165) : déperdition entre l'émission et l'évacuation, en K.
pub const D_THETA_UTIL_DOUCHES_K: f64 = 3.0;
/// « θec » : température de l'eau chaude prélevée au ballon, fixée à 55 °C par défaut
/// (tableau 299).
pub const THETA_EC_C: f64 = 55.0;
/// « θamb » : température des locaux chauffés, en °C (tableaux 301 et 302).
pub const THETA_AMB_C: f64 = 20.0;
/// Efficacité par défaut `0,8 × 0,15` (§9.21.6.3.2.1) en l'absence d'information.
pub const EFFICACITE_DEFAUT: f64 = 0.8 * 0.15;
/// « n_max » de l'itération de la position 2 (§9.21.2.4.1).
pub const N_MAX_ITERATIONS: usize = 1000;
/// Tolérance de l'itération de la position 2 : 5 % (§9.21.2.4.1).
pub const TOLERANCE_DEBIT: f64 = 0.05;

/// Lectures ambiguës de la fiche (Q108). Le défaut est la lecture retenue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Lecture {
    /// Éq. (2161) à la lettre : `Rat = (100 − Ratdouches-bains)/100`, au lieu de la part des
    /// douches `Ratdouches-bains/100`.
    pub rat_litteral: bool,
    /// Dénominateur des éq. (2159) et (2160) à la lettre : somme des appareils sans le nombre
    /// d'échangeurs.
    pub denominateur_litteral: bool,
    /// Éq. (2187) à la lettre : en position 2, l'émetteur ne compte que le volume des douches
    /// (la part hors douches disparaît des besoins).
    pub sans_reste_hors_douches: bool,
    /// Débit d'eau froide de la position 2 : fraction froide `(θec − θuw)/(θec − θep)` au lieu
    /// de la fraction chaude imprimée dans l'itération de la p. 1252.
    pub debit_froid_position_2: bool,
    /// Durée de la pompe de relevage à la lettre de l'éq. (2221) : `debitnominal/Veg,recup`.
    pub pompe_litterale: bool,
}

/// Efficacité retenue d'un échangeur et vrai si elle vient de la valeur par défaut
/// (§9.21.6.3.2.1).
pub fn efficacite_fixe(e: &EchangeurEauxGrises) -> (f64, bool) {
    match (e.origine, e.efficacite, e.c_trans) {
        (Some(o), Some(eps), Some(ct)) => (
            match o {
                OrigineEfficaciteEauxGrises::Certifiee => ct * eps,
                OrigineEfficaciteEauxGrises::Justifiee => 0.9 * ct * eps,
                OrigineEfficaciteEauxGrises::Declaree => (0.8 * ct * eps).min(0.15),
            },
            false,
        ),
        _ => (EFFICACITE_DEFAUT, true),
    }
}

/// Th-BCE 2020 §9.21.6.3.2, éq. (2215) à (2219) : échangeur instantané. Renvoie
/// `(θep, θeg,evac)` pour une efficacité fixe, les températures d'entrée et les débits en L/h.
pub fn echangeur_instantane(
    eps_fixe: f64,
    theta_ef_c: f64,
    theta_eg_c: f64,
    debit_ef_l_h: f64,
    debit_eg_l_h: f64,
) -> (f64, f64) {
    // Éq. (2216) et (2217).
    let c_eg = RHO_W_KG_L * C_W_WH_KG_K * debit_eg_l_h;
    let c_ef = RHO_W_KG_L * C_W_WH_KG_K * debit_ef_l_h;
    // Éq. (2215).
    if !(c_eg > 0.0 && c_ef > 0.0) {
        return (theta_ef_c, theta_eg_c);
    }
    let min = c_eg.min(c_ef);
    // Éq. (2218) et (2219).
    (
        theta_ef_c + eps_fixe * (theta_eg_c - theta_ef_c) * min / c_ef,
        theta_eg_c + eps_fixe * (theta_ef_c - theta_eg_c) * min / c_eg,
    )
}

/// Th-BCE 2020 §9.21.4.3.2 (éq. 2196 à 2200) et §9.21.5.4.2 (éq. 2204 à 2208) : température des
/// eaux grises en sortie d'une distribution d'évacuation de longueurs `l_vc_m` (volume chauffé)
/// et `l_vnc_m` (volume non chauffé).
pub fn distribution_evacuation(
    theta_eg_c: f64,
    l_vc_m: f64,
    l_vnc_m: f64,
    theta_ext_c: f64,
) -> f64 {
    let somme = l_vc_m + l_vnc_m;
    // Éq. (2196) ; sans conduit (somme nulle, refusée par la validation), aucune perte.
    let b = if somme > 0.0 { l_vnc_m / somme } else { 0.0 };
    // Éq. (2197).
    let theta_air = (1.0 - b) * THETA_AMB_C + b * theta_ext_c;
    // Éq. (2199).
    let f_am = 1.0 - 0.16 / 100.0 * l_vc_m - 0.30 / 100.0 * l_vnc_m;
    // Éq. (2198) et (2200).
    theta_eg_c - (1.0 - f_am) * (theta_eg_c - theta_air)
}

/// Volume équivalent à 40 °C d'un puisage de douche, en L (éq. 2168).
pub fn v_equivalent_l() -> f64 {
    DEBIT_NOMINAL_DOUCHES_L_H * DUREE_NOMINALE_DOUCHES_H
}

/// Nombre de puisages équivalents (éq. 2167 et 2169) : arrondi supérieur de `V / Vuw-equiv`.
pub fn n_puisages_equivalents(v_l: f64) -> f64 {
    if v_l > 0.0 {
        (v_l / v_equivalent_l()).ceil()
    } else {
        0.0
    }
}

/// Débit d'eaux grises de `v_l` litres d'un pas de temps, relié ou non (éq. 2170 à 2177) : le
/// débit effectif d'un puisage, multiplié par le nombre de puisages et, au-delà d'un puisage,
/// par le facteur de concomitance `0,8/√(N − 1)`.
pub fn debit_eaux_grises_l_h(v_l: f64) -> f64 {
    let n = n_puisages_equivalents(v_l);
    if n < 1.0 {
        return 0.0;
    }
    // Éq. (2170) et (2171).
    let effectif = v_l / (n * DUREE_NOMINALE_DOUCHES_H);
    if n <= 1.0 {
        // Éq. (2172) et (2175).
        n * effectif
    } else {
        // Éq. (2173), (2174), (2176) et (2177).
        n * effectif * 0.8 / libm::sqrt(n - 1.0)
    }
}

/// « Csimultaneite » (éq. 2223).
pub fn c_simultaneite(nb_relies: f64) -> f64 {
    if nb_relies >= 5.0 {
        0.8 / libm::sqrt(nb_relies - 1.0)
    } else {
        1.0
    }
}

/// Chaleur déjà contenue dans l'eau froide du ballon préchauffée (lecture Q108 point 6) :
/// `ρw·cw·Vec·(θentrant − θcw)` (Wh), avec le volume d'eau chaude `Vec` réellement appelé au
/// ballon (réduit en positions 0 et 2) et `θentrant` (éq. 2191 et 2192 : `θep` en positions 0 et
/// 1, `θef = θcw` en position 2, donc chaleur nulle). Th-BCE 2020 §9.21.3, éq. (2190) à (2192) ;
/// déduite de `Qw` (éq. 2186, 2181, 2189, calculées à partir de `θcw`) pour que la génération
/// fournisse `ρw·cw·Vec·(θec − θentrant)`, ce que le ballon puise réellement (éq. 1794).
pub fn chaleur_preechauffage_wh(v_ec_l: f64, theta_entrant_c: f64, theta_cw_c: f64) -> f64 {
    RHO_W_KG_L * C_W_WH_KG_K * v_ec_l * (theta_entrant_c - theta_cw_c)
}

/// Configuration unitaire résolue.
#[derive(Debug, Clone, PartialEq)]
pub struct UniteResolue {
    /// « Nbechangeurs ».
    pub nb_ech: f64,
    /// « Nbrelies,douches-bains ».
    pub nb_rel: f64,
    /// « Nbnon-relies,douches-bains ».
    pub nb_non: f64,
    /// « εech_eg_fixe » retenue.
    pub eps: f64,
    /// Vrai si l'efficacité est la valeur par défaut.
    pub eps_defaut: bool,
    /// « Prelevage,nominal », en W.
    pub p_relevage_w: Option<f64>,
}

/// Récupérateur d'un émetteur d'ECS équivalent, prêt pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct RecuperateurResolu {
    /// « Tposition » (imposée à « en amont du ballon » en récupération collective).
    pub position: PositionRecuperateur,
    /// Identifiant du récupérateur collectif si la récupération est collective.
    pub collectif: Option<String>,
    /// Configurations unitaires.
    pub unites: Vec<UniteResolue>,
    /// « Levac,vc^2nd-e », en m.
    pub l_vc_m: f64,
    /// « Levac,vnc^2nd-e », en m.
    pub l_vnc_m: f64,
    /// « Rat » : part des besoins dédiée aux douches (éq. 2161), entre 0 et 1.
    pub rat: f64,
    /// Lecture des ambiguïtés.
    pub lecture: Lecture,
}

/// Flux d'un émetteur vers le récupérateur collectif (éq. 2201 à 2203), à l'échelle de
/// l'émetteur.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ApportCollectif {
    /// « debiteg^2nd-e » : débit d'eaux grises récupérables, en L/h.
    pub debit_eg_l_h: f64,
    /// « debitef^2nd-e » : débit d'eau froide appelé, en L/h.
    pub debit_ef_l_h: f64,
    /// « θeg^2nd-e » : température en sortie de la distribution du groupe, en °C.
    pub theta_eg_c: f64,
}

/// Résultat d'un pas de temps d'un émetteur.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SortiesRecuperateur {
    /// « Qw » de l'émetteur (éq. 2183, 2186 ou 2189), en Wh.
    pub q_w_wh: f64,
    /// « Qw » sans récupérateur (éq. 1693), en Wh.
    pub q_w_sans_wh: f64,
    /// Volume d'eau chaude appelé au ballon `Vec`, en L (éq. 2182, 2185, 2188).
    pub v_ec_l: f64,
    /// « θentrant » pondéré par le volume d'eau chaude, en °C (éq. 2190 à 2192) ; en
    /// récupération collective, `θcw` : le récupérateur collectif la remplace.
    pub theta_entrant_c: f64,
    /// « Qrelevage » (éq. 2224), en Wh.
    pub w_relevage_wh: f64,
    /// Flux vers le récupérateur collectif (récupération collective seulement).
    pub apport: ApportCollectif,
}

impl RecuperateurResolu {
    /// Résout un récupérateur saisi ; `rat_douches_bains` est « Ratdouches-bains » de
    /// l'usage, en fraction (§9.5.3.4.2).
    pub fn nouveau(r: &RecuperateurEauxGrises, rat_douches_bains: f64) -> Self {
        Self::nouveau_selon(r, rat_douches_bains, Lecture::default())
    }

    /// Comme [`Self::nouveau`], avec une lecture donnée (tests des variantes).
    pub fn nouveau_selon(
        r: &RecuperateurEauxGrises,
        rat_douches_bains: f64,
        lecture: Lecture,
    ) -> Self {
        let collectif = match r.configuration {
            crate::entree::ConfigurationRecuperateur::Collective => {
                Some(r.recuperateur_collectif.clone().unwrap_or_default())
            }
            crate::entree::ConfigurationRecuperateur::Individuelle => None,
        };
        let unites = r
            .unites
            .iter()
            .map(|u| {
                let (eps, eps_defaut) = efficacite_fixe(&u.echangeur.clone().unwrap_or_default());
                UniteResolue {
                    nb_ech: f64::from(u.nb_echangeurs),
                    nb_rel: f64::from(u.nb_douches_relies),
                    nb_non: f64::from(u.nb_douches_non_relies.unwrap_or(0)),
                    eps,
                    eps_defaut,
                    p_relevage_w: u.p_relevage_nominale_w,
                }
            })
            .collect();
        Self {
            // Figure 205 : « Typerecup = 0 implique Tposition = 1 ».
            position: if collectif.is_some() {
                PositionRecuperateur::AmontBallon
            } else {
                r.position.unwrap_or(PositionRecuperateur::AmontBallon)
            },
            collectif,
            unites,
            l_vc_m: r.l_evac_vc_m,
            l_vnc_m: r.l_evac_vnc_m,
            // Éq. (2161) : voir Q108 pour la lecture retenue.
            rat: if lecture.rat_litteral {
                1.0 - rat_douches_bains
            } else {
                rat_douches_bains
            },
            lecture,
        }
    }

    /// Nombre total d'appareils sanitaires pris en compte dans les éq. (2159) et (2160).
    fn n_appareils(&self) -> f64 {
        self.unites
            .iter()
            .map(|u| {
                let p = if self.lecture.denominateur_litteral {
                    1.0
                } else {
                    u.nb_ech
                };
                p * (u.nb_rel + u.nb_non)
            })
            .sum()
    }

    /// Pas de temps (§9.21.2.4) : `v_uw_em_l` est « Vuw^gr,em-e(h) » de l'émetteur, en L
    /// d'eau mitigée ; `theta_cw_c` est « θcw » et `theta_ext_c` « θext(h) ».
    pub fn pas(&self, v_uw_em_l: f64, theta_cw_c: f64, theta_ext_c: f64) -> SortiesRecuperateur {
        let n_app = self.n_appareils();
        let rho_c = RHO_W_KG_L * C_W_WH_KG_K;
        let fr_hot_cw = (THETA_UW_C - theta_cw_c) / (THETA_EC_C - theta_cw_c);
        let q_sans = rho_c * v_uw_em_l * (THETA_UW_C - theta_cw_c);
        let mut s = SortiesRecuperateur {
            q_w_sans_wh: q_sans,
            q_w_wh: q_sans,
            theta_entrant_c: theta_cw_c,
            ..SortiesRecuperateur::default()
        };
        if !(n_app > 0.0 && v_uw_em_l > 0.0) {
            return s;
        }
        // Éq. (2193) à (2200) : distribution d'évacuation du groupe (éq. 2166 pour θeg).
        let theta_eg_2nd = distribution_evacuation(
            THETA_UW_C - D_THETA_UTIL_DOUCHES_K,
            self.l_vc_m,
            self.l_vnc_m,
            theta_ext_c,
        );
        let mut v_ec_pre = 0.0;
        let mut somme_theta = 0.0;
        let mut q_w = 0.0;
        for u in &self.unites {
            // Éq. (2158) à (2160) : volumes de la configuration unitaire.
            let v_tot_u = v_uw_em_l * (u.nb_rel + u.nb_non) / n_app;
            let v_rel = self.rat * v_uw_em_l * u.nb_rel / n_app;
            let v_non = self.rat * v_uw_em_l * u.nb_non / n_app;
            // Éq. (2164), (2172) à (2177).
            let d_eg_rec = debit_eaux_grises_l_h(v_rel);
            let d_eg_non = debit_eaux_grises_l_h(v_non);
            let (theta_ep, v_ec_u) = match self.position {
                PositionRecuperateur::AmontMitigeursEtBallon => {
                    // Éq. (2178) : tout l'appel d'eau froide traverse l'échangeur.
                    let (ep, _) = echangeur_instantane(
                        u.eps,
                        theta_cw_c,
                        theta_eg_2nd,
                        d_eg_rec + d_eg_non,
                        d_eg_rec,
                    );
                    // Éq. (2184).
                    (ep, v_tot_u * (THETA_UW_C - ep) / (THETA_EC_C - ep))
                }
                PositionRecuperateur::AmontBallon => {
                    // Éq. (2180) (lecture Q108 : `θec` au dénominateur).
                    let d_ef = (d_eg_rec + d_eg_non) * fr_hot_cw;
                    let (ep, _) =
                        echangeur_instantane(u.eps, theta_cw_c, theta_eg_2nd, d_ef, d_eg_rec);
                    if self.collectif.is_some() {
                        s.apport.debit_eg_l_h += u.nb_ech * d_eg_rec;
                        s.apport.debit_ef_l_h += u.nb_ech * d_ef;
                    }
                    // Éq. (2181) : volume d'eau chaude inchangé ; préchauffage vu du ballon.
                    (ep, v_tot_u * fr_hot_cw)
                }
                PositionRecuperateur::AmontMitigeurs => {
                    let d_ef =
                        self.debit_froid_position_2(u.eps, theta_cw_c, theta_eg_2nd, d_eg_rec);
                    let (ep, _) =
                        echangeur_instantane(u.eps, theta_cw_c, theta_eg_2nd, d_ef, d_eg_rec);
                    // Éq. (2187) ; la part hors douches garde l'eau froide non préchauffée
                    // (lecture Q108).
                    let reste = if self.lecture.sans_reste_hors_douches {
                        0.0
                    } else {
                        (v_tot_u - v_rel - v_non).max(0.0)
                    };
                    (
                        ep,
                        v_rel * (THETA_UW_C - ep) / (THETA_EC_C - ep) + (v_non + reste) * fr_hot_cw,
                    )
                }
            };
            s.v_ec_l += u.nb_ech * v_ec_u;
            // Éq. (2191) et (2192) : θentrant = θep en positions 0 et 1, θef en position 2.
            if self.position != PositionRecuperateur::AmontMitigeurs {
                v_ec_pre += u.nb_ech * v_ec_u;
                somme_theta += u.nb_ech * v_ec_u * theta_ep;
            }
            // Éq. (2186) et (2189) : besoins de la position 0 ou 2, rapportés à `θec − θcw`.
            q_w += u.nb_ech * rho_c * v_ec_u * (THETA_EC_C - theta_cw_c);
            // Éq. (2220) à (2224) : pompe de relevage.
            if let Some(p) = u.p_relevage_w {
                if v_rel > 0.0 {
                    let debit_nominal =
                        c_simultaneite(u.nb_rel) * u.nb_rel * DEBIT_NOMINAL_DOUCHES_L_H;
                    // Éq. (2221) : lecture Q108 : `Veg,recup / debitnominal`.
                    let dt = if self.lecture.pompe_litterale {
                        debit_nominal / v_rel
                    } else {
                        v_rel / debit_nominal
                    };
                    s.w_relevage_wh += u.nb_ech * p * dt;
                }
            }
        }
        if self.position != PositionRecuperateur::AmontBallon {
            s.q_w_wh = q_w;
        }
        if v_ec_pre > 0.0 && self.collectif.is_none() {
            // Éq. (2191) : pondération par le volume d'eau chaude appelé ; en récupération
            // collective, le récupérateur collectif fixe `θentrant`.
            s.theta_entrant_c = somme_theta / v_ec_pre;
        }
        if self.collectif.is_some() {
            s.apport.theta_eg_c = theta_eg_2nd;
        }
        s
    }

    /// Débit d'eau froide de la position 2 (§9.21.2.4.1) : itération `θep(n+1) ← échangeur`,
    /// `debitef(n+1) = debiteg,recup × (θuw − θep)/(θec − θep)`, arrêtée à moins de 5 %.
    fn debit_froid_position_2(
        &self,
        eps: f64,
        theta_cw_c: f64,
        theta_eg_c: f64,
        d_rec: f64,
    ) -> f64 {
        if d_rec <= 0.0 || d_rec.is_nan() {
            return 0.0;
        }
        let fraction = |ep: f64| {
            if self.lecture.debit_froid_position_2 {
                (THETA_EC_C - THETA_UW_C) / (THETA_EC_C - ep)
            } else {
                (THETA_UW_C - ep) / (THETA_EC_C - ep)
            }
        };
        // Initialisation : rendement initial de 50 %.
        let mut d = d_rec * fraction((theta_eg_c + theta_cw_c) / 2.0);
        for _ in 0..N_MAX_ITERATIONS {
            let (ep, _) = echangeur_instantane(eps, theta_cw_c, theta_eg_c, d, d_rec);
            let d_suivant = d_rec * fraction(ep);
            let ecart = (d_suivant - d).abs() / d_suivant;
            d = d_suivant;
            if ecart <= TOLERANCE_DEBIT {
                break;
            }
        }
        d
    }
}

/// Th-BCE 2020 §9.21.5.4 et §9.21.6.3.1, éq. (2201) à (2211) : récupérateur collectif.
/// `apports` : flux des émetteurs reliés ; renvoie `θep(h)` (`θcw` sans écoulement).
pub fn pas_collectif(
    apports: &[ApportCollectif],
    l_vc_m: f64,
    l_vnc_m: f64,
    eps: f64,
    theta_cw_c: f64,
    theta_ext_c: f64,
) -> f64 {
    // Éq. (2201) et (2202).
    let d_eg: f64 = apports.iter().map(|a| a.debit_eg_l_h).sum();
    let d_ef: f64 = apports.iter().map(|a| a.debit_ef_l_h).sum();
    if d_eg <= 0.0 || d_eg.is_nan() {
        return theta_cw_c;
    }
    // Éq. (2203).
    let theta_eg = apports
        .iter()
        .map(|a| a.debit_eg_l_h * a.theta_eg_c)
        .sum::<f64>()
        / d_eg;
    // Éq. (2204) à (2208), puis éq. (2209) à (2211) et (2215) à (2218).
    let theta_sortie = distribution_evacuation(theta_eg, l_vc_m, l_vnc_m, theta_ext_c);
    echangeur_instantane(eps, theta_cw_c, theta_sortie, d_ef, d_eg).0
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    //! Valeurs calculées à la main d'après les équations de la fiche 9.21 (recalcul indépendant
    //! du code, en arithmétique décimale) : émetteur de 100 L d'eau mitigée par heure, eau
    //! froide à 10 °C, extérieur à 0 °C, deux douches reliées à un échangeur d'efficacité 0,12
    //! (valeur par défaut), conduit de 2 m en volume chauffé, `Rat = 0,8`.

    use super::*;
    use crate::entree::{ConfigurationRecuperateur, RecuperateurEauxGrises, UniteRecuperateur};

    fn proche(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-6 * b.abs().max(1.0), "{a} != {b}");
    }

    fn saisie(
        position: PositionRecuperateur,
        nb_ech: u32,
        rel: u32,
        non: u32,
    ) -> RecuperateurEauxGrises {
        RecuperateurEauxGrises {
            configuration: ConfigurationRecuperateur::Individuelle,
            position: Some(position),
            recuperateur_collectif: None,
            unites: vec![UniteRecuperateur {
                nb_echangeurs: nb_ech,
                nb_douches_relies: rel,
                nb_douches_non_relies: Some(non),
                echangeur: None,
                p_relevage_nominale_w: None,
            }],
            l_evac_vc_m: 2.0,
            l_evac_vnc_m: 0.0,
        }
    }

    #[test]
    fn efficacite_selon_l_origine() {
        let e = |o, eps, ct| EchangeurEauxGrises {
            origine: o,
            efficacite: eps,
            c_trans: ct,
        };
        use OrigineEfficaciteEauxGrises::*;
        // 0,8 × 0,5 × 0,9 = 0,36 ; 0,9 × 0,8 × 0,5 = 0,36 ; min(0,8 × 0,9 × 0,5 ; 0,15) = 0,15.
        proche(
            efficacite_fixe(&e(Some(Certifiee), Some(0.8), Some(0.5))).0,
            0.4,
        );
        proche(
            efficacite_fixe(&e(Some(Justifiee), Some(0.8), Some(0.5))).0,
            0.36,
        );
        proche(
            efficacite_fixe(&e(Some(Declaree), Some(0.9), Some(0.5))).0,
            0.15,
        );
        proche(
            efficacite_fixe(&e(Some(Declaree), Some(0.1), Some(0.5))).0,
            0.04,
        );
        // Au moins une valeur absente : 0,8 × 0,15.
        assert_eq!(
            efficacite_fixe(&e(Some(Certifiee), None, Some(0.5))),
            (0.12, true)
        );
        assert_eq!(
            efficacite_fixe(&EchangeurEauxGrises::default()),
            (0.12, true)
        );
    }

    #[test]
    fn echangeur_instantane_a_la_main() {
        // Ceg = 2 Cef : min = Cef, θep = 10 + 0,12 × 27 × 1 = 13,24 ; θeg,evac = 37 − 0,12 × 27
        // × 0,5 = 35,38.
        let (ep, eg) = echangeur_instantane(0.12, 10.0, 37.0, 100.0, 200.0);
        proche(ep, 13.24);
        proche(eg, 35.38);
        // Cef = 2 Ceg : min = Ceg, θep = 10 + 0,12 × 27 × 0,5 = 11,62.
        let (ep, eg) = echangeur_instantane(0.12, 10.0, 37.0, 200.0, 100.0);
        proche(ep, 11.62);
        proche(eg, 33.76);
        // Éq. (2215) : sans débit, efficacité nulle.
        assert_eq!(
            echangeur_instantane(0.12, 10.0, 37.0, 0.0, 100.0),
            (10.0, 37.0)
        );
    }

    #[test]
    fn distribution_evacuation_a_la_main() {
        // Lvc = 2 m, Lvnc = 3 m, θext = 5 °C : b = 0,6, θair = 0,4 × 20 + 0,6 × 5 = 11,
        // Fam = 1 − 0,0032 − 0,009 = 0,9878, Δθ = 0,0122 × 26 = 0,3172.
        proche(distribution_evacuation(37.0, 2.0, 3.0, 5.0), 36.6828);
        // Lvc seul : Δθ = 0,0032 × 17 = 0,0544.
        proche(distribution_evacuation(37.0, 2.0, 0.0, 0.0), 36.9456);
    }

    #[test]
    fn debits_d_eaux_grises_a_la_main() {
        proche(v_equivalent_l(), 39.84);
        assert_eq!(n_puisages_equivalents(30.0), 1.0);
        assert_eq!(n_puisages_equivalents(80.0), 3.0);
        // Un puisage : 30/0,083. Deux : 2 × 60/(2 × 0,083) × 0,8/√1. Trois : 80/0,083 × 0,8/√2.
        proche(debit_eaux_grises_l_h(30.0), 361.4457831325301);
        proche(debit_eaux_grises_l_h(60.0), 578.3132530120482);
        proche(debit_eaux_grises_l_h(80.0), 545.2389638064944);
        assert_eq!(debit_eaux_grises_l_h(0.0), 0.0);
        // Éq. (2223).
        assert_eq!(c_simultaneite(4.0), 1.0);
        proche(c_simultaneite(5.0), 0.4);
    }

    #[test]
    fn position_1_en_amont_du_ballon() {
        // Veg,recup = 0,8 × 100 = 80 L (N = 3) : debit_eg = 545,2390 L/h, θeg = 36,9456 °C ;
        // debit_ef = 545,2390 × 30/45 = 363,4926 L/h < debit_eg : min = Cef, θep = 10 + 0,12 ×
        // 26,9456 = 13,233472 °C. Besoins inchangés (éq. 2183), 1,163 × 100 × 30 = 3 489 Wh ;
        // eau chaude 100 × 30/45 = 66,667 L.
        let r =
            RecuperateurResolu::nouveau(&saisie(PositionRecuperateur::AmontBallon, 1, 2, 0), 0.8);
        let s = r.pas(100.0, 10.0, 0.0);
        proche(s.q_w_wh, 3489.0);
        proche(s.q_w_sans_wh, 3489.0);
        proche(s.v_ec_l, 66.66666666666666);
        proche(s.theta_entrant_c, 13.233472);
        assert_eq!(s.w_relevage_wh, 0.0);
    }

    #[test]
    fn position_0_en_amont_des_mitigeurs_et_du_ballon() {
        // θep = 13,233472 °C (min = Cef = Ceg). Éq. (2184) : Vec = 100 × (40 − 13,233472)/(55 −
        // 13,233472) = 64,086074 L ; éq. (2186) : Qw = 1,163 × 64,086074 × 45 = 3 353,9447 Wh.
        let r = RecuperateurResolu::nouveau(
            &saisie(PositionRecuperateur::AmontMitigeursEtBallon, 1, 2, 0),
            0.8,
        );
        let s = r.pas(100.0, 10.0, 0.0);
        proche(s.theta_entrant_c, 13.233472);
        proche(s.v_ec_l, 64.08607390109132);
        proche(s.q_w_wh, 3353.9446776136147);
        assert!(s.q_w_wh < s.q_w_sans_wh);
    }

    #[test]
    fn position_2_en_amont_des_mitigeurs() {
        // Itération : θep(0) = (36,9456 + 10)/2, debit(0) = 285,8 ; θep(1) = 13,233472
        // (Cef < Ceg donc indépendant du débit), debit(1) = 349,4222 (écart de 18 %) ; debit(2)
        // identique, écart nul : arrêt. Éq. (2187) : Vec = 80 × 26,766528/41,766528 + 20 × 30/45
        // (part hors douches, lecture Q108) = 64,602192 L ; Qw = 1,163 × 64,602192 × 45.
        let r = RecuperateurResolu::nouveau(
            &saisie(PositionRecuperateur::AmontMitigeurs, 1, 2, 0),
            0.8,
        );
        let s = r.pas(100.0, 10.0, 0.0);
        proche(s.v_ec_l, 64.60219245420639);
        proche(s.q_w_wh, 3380.9557420908914);
        // Éq. (2192) : aucun effet sur l'entrée du ballon.
        assert_eq!(s.theta_entrant_c, 10.0);
    }

    /// Absence de double comptage de la déduction (Q108 point 6), à la main sur l'heure des tests
    /// précédents (100 L d'eau mitigée, θcw = 10 °C, θep = 13,233472 °C, ρc = 1,163 Wh/L/K).
    /// Ce que le ballon puise réellement est `ρc·Vec·(55 − θentrant)`, et ce que l'usager
    /// consomme est `ρc·100·(40 − θep)` en positions 0 et 1.
    #[test]
    fn deduction_sans_double_comptage() {
        let rec = |p| RecuperateurResolu::nouveau(&saisie(p, 1, 2, 0), 0.8);
        // Position 0 : Qw = 3 353,9447 Wh pour Vec = 64,086074 L (déjà réduit par le mitigeur).
        // Déduction 1,163 × 64,086074 × 3,233472 = 240,9975 Wh, soit 3 112,9472 Wh de demande :
        // 1,163 × 100 × (40 − 13,233472), l'énergie réellement nécessaire pour porter 100 L à
        // 40 °C depuis θep.
        let s = rec(PositionRecuperateur::AmontMitigeursEtBallon).pas(100.0, 10.0, 0.0);
        let d = chaleur_preechauffage_wh(s.v_ec_l, s.theta_entrant_c, 10.0);
        proche(d, 240.997471);
        proche(s.q_w_wh - d, 3112.947206);
        proche(s.q_w_wh - d, 1.163 * s.v_ec_l * (55.0 - s.theta_entrant_c));
        // Position 1 : Vec = 66,6667 L inchangé, Qw = 3 489 Wh ; déduction 1,163 × 66,6667 ×
        // 3,233472 = 250,7019 Wh ; demande 3 238,2981 Wh = 1,163 × 66,6667 × (55 − 13,233472).
        let s = rec(PositionRecuperateur::AmontBallon).pas(100.0, 10.0, 0.0);
        let d = chaleur_preechauffage_wh(s.v_ec_l, s.theta_entrant_c, 10.0);
        proche(d, 250.701862);
        proche(s.q_w_wh - d, 3238.298138);
        // Position 2 : θentrant = θef = 10 °C, déduction nulle ; Qw (éq. 2189, Vec réduit)
        // est déjà la chaleur à fournir.
        let s = rec(PositionRecuperateur::AmontMitigeurs).pas(100.0, 10.0, 0.0);
        assert_eq!(
            chaleur_preechauffage_wh(s.v_ec_l, s.theta_entrant_c, 10.0),
            0.0
        );
    }

    #[test]
    fn variantes_de_lecture_chiffrees() {
        let sai = saisie(PositionRecuperateur::AmontMitigeurs, 1, 2, 0);
        // Éq. (2187) à la lettre : la part hors douches (20 L) disparaît, Qw = 2 683,16 Wh au
        // lieu de 3 380,96 Wh (− 21 %).
        let l = Lecture {
            sans_reste_hors_douches: true,
            ..Lecture::default()
        };
        let s = RecuperateurResolu::nouveau_selon(&sai, 0.8, l).pas(100.0, 10.0, 0.0);
        proche(s.q_w_wh, 2683.1557420908916);
        // Éq. (2161) à la lettre : Rat = 0,2, soit 20 L de douches au lieu de 80 L.
        let l = Lecture {
            rat_litteral: true,
            ..Lecture::default()
        };
        let r = RecuperateurResolu::nouveau_selon(&sai, 0.8, l);
        proche(r.rat, 0.2);
        // Dénominateur sans le nombre d'échangeurs : 3 échangeurs d'une douche reliée et d'une
        // non reliée, 300 L : 40 L par appareil relié (retenu), 120 L à la lettre (360 L de
        // douches reliées au total pour 240 L réels).
        let sai3 = saisie(PositionRecuperateur::AmontBallon, 3, 1, 1);
        let retenu = RecuperateurResolu::nouveau(&sai3, 0.8);
        proche(retenu.rat * 300.0 * 1.0 / retenu.n_appareils(), 40.0);
        let litteral = RecuperateurResolu::nouveau_selon(
            &sai3,
            0.8,
            Lecture {
                denominateur_litteral: true,
                ..Lecture::default()
            },
        );
        proche(litteral.rat * 300.0 * 1.0 / litteral.n_appareils(), 120.0);
        // Débit froid de la position 2 : fraction froide (θec − θuw)/(θec − θep) = 15/41,77 au
        // lieu de la fraction chaude 26,77/41,77.
        let r = RecuperateurResolu::nouveau(&sai, 0.8);
        let chaud = r.debit_froid_position_2(0.12, 10.0, 36.9456, 545.2389638064944);
        let mut froid = r.clone();
        froid.lecture.debit_froid_position_2 = true;
        let froid = froid.debit_froid_position_2(0.12, 10.0, 36.9456, 545.2389638064944);
        proche(chaud, 545.2389638064944 * 26.766528 / 41.766528);
        proche(froid, 545.2389638064944 * 15.0 / 41.766528);
    }

    #[test]
    fn pompe_de_relevage_a_la_main() {
        // 80 L d'eaux grises récupérables, deux douches (Csimultaneite = 1) : débit nominal
        // 2 × 480 = 960 L/h, durée 80/960 h, 50 W : 4,1667 Wh. À la lettre de l'éq. (2221) :
        // 960/80 = 12 h, 600 Wh.
        let mut sai = saisie(PositionRecuperateur::AmontBallon, 1, 2, 0);
        sai.unites[0].p_relevage_nominale_w = Some(50.0);
        let r = RecuperateurResolu::nouveau(&sai, 0.8);
        proche(r.pas(100.0, 10.0, 0.0).w_relevage_wh, 4.166666666666667);
        let l = RecuperateurResolu::nouveau_selon(
            &sai,
            0.8,
            Lecture {
                pompe_litterale: true,
                ..Lecture::default()
            },
        );
        proche(l.pas(100.0, 10.0, 0.0).w_relevage_wh, 600.0);
        // Éq. (2224) : multiplié par le nombre d'échangeurs.
        sai.unites[0].nb_echangeurs = 3;
        sai.unites[0].nb_douches_relies = 2;
        let r = RecuperateurResolu::nouveau(&sai, 0.8);
        // Volume par échangeur : 0,8 × 300 × 2/6 = 80 L, soit 3 × 4,1667 Wh.
        proche(r.pas(300.0, 10.0, 0.0).w_relevage_wh, 12.5);
    }

    #[test]
    fn recuperateur_collectif_a_la_main() {
        // Deux émetteurs : (545,2390 L/h d'eaux grises, 363,4926 L/h d'eau froide, 36,9456 °C) et
        // (200 L/h, 133,333 L/h, 36 °C) : 745,2390 L/h, 496,8260 L/h, 36,691829 °C ; conduit
        // intergroupe de 1 m en volume chauffé : Δθ = 0,0016 × 16,691829, soit 36,665122 °C ;
        // efficacité 0,15, Cef < Ceg : θep = 10 + 0,15 × 26,665122 = 13,999768 °C.
        let a = [
            ApportCollectif {
                debit_eg_l_h: 545.2389638064944,
                debit_ef_l_h: 363.49264253766296,
                theta_eg_c: 36.9456,
            },
            ApportCollectif {
                debit_eg_l_h: 200.0,
                debit_ef_l_h: 200.0 * 30.0 / 45.0,
                theta_eg_c: 36.0,
            },
        ];
        proche(
            pas_collectif(&a, 1.0, 0.0, 0.15, 10.0, 0.0),
            13.999768318492269,
        );
        // Sans écoulement : eau froide non préchauffée.
        assert_eq!(pas_collectif(&[], 1.0, 0.0, 0.15, 10.0, 0.0), 10.0);
    }

    #[test]
    fn collectif_impose_la_position_1() {
        let mut sai = saisie(PositionRecuperateur::AmontMitigeurs, 1, 2, 0);
        sai.configuration = ConfigurationRecuperateur::Collective;
        sai.recuperateur_collectif = Some("rc".into());
        let r = RecuperateurResolu::nouveau(&sai, 0.8);
        assert_eq!(r.position, PositionRecuperateur::AmontBallon);
        let s = r.pas(100.0, 10.0, 0.0);
        proche(s.q_w_wh, 3489.0);
        // Flux transmis au récupérateur collectif (éq. 2201 et 2202).
        proche(s.apport.debit_eg_l_h, 545.2389638064944);
        proche(s.apport.debit_ef_l_h, 363.49264253766296);
        proche(s.apport.theta_eg_c, 36.9456);
        assert_eq!(s.theta_entrant_c, 10.0);
    }
}
