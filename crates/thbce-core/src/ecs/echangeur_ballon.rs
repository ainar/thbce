// SPDX-License-Identifier: GPL-3.0-or-later
//! Accumulateur d'ECS en eau technique, séparé du réseau de distribution par un échangeur :
//! fiches `C_STO_échangeur_ballon` (Th-BCE 2020 §9.11) et `C_STO_Accumulateur_ECS en eau
//! technique` (§9.12), échangeur externe (à plaques, avec circulateur primaire) ou interne
//! (serpentin ou double-peau).
//!
//! Périmètre : assemblages « base seule » et « base et appoint intégré » (`Type_Assemblage` 0
//! et 1), production d'ECS sans réseau bouclé. Le ballon est décrit par quatre zones de
//! volumes égaux (éq. 1874, 1875, 1921 et 1922) et sa consigne est celle du tableau 288. Hors
//! de cette version : réchauffage de boucle (éq. 1883 et 1909 à 1913 : le débit de la boucle
//! n'est pas restitué par la fiche 9.8, Q48 point 8), appoint dans un stockage séparé ou
//! instantané (`Type_Assemblage` 2 et 3, éq. 1914 à 1920 et 1941 à 1944), systèmes solaires
//! combinés, échangeurs de chauffage (stockage de chauffage non pris en charge).
//!
//! Lectures : `docs/questions/Q106.md`.

use super::ballon_de_stockage::{Ballon, NB_ZONES, RHO_CW_WH_L_K};
use super::gestion_regulation_ballon::Puisage;

/// Consigne d'un accumulateur individuel, en °C (tableau 288).
pub const THETA_CONS_INDIVIDUEL_C: f64 = 55.0;
/// Consigne d'un accumulateur collectif, avec ou sans bouclage, en °C (tableau 288).
pub const THETA_CONS_COLLECTIF_C: f64 = 60.0;
/// Volume en dessous duquel un accumulateur est individuel, en L (§9.12.3.2.2).
pub const V_INDIVIDUEL_MAX_L: f64 = 500.0;
/// « Modmin » par défaut : rapport du débit minimal du circulateur primaire à son débit
/// nominal (tableau 286, colonne « Def »).
pub const MOD_MIN_DEFAUT: f64 = 0.3;
/// Élévation de température du ballon, en K, que représente une itération de puisage d'un
/// échangeur interne (éq. 1931).
const PAS_ITERATION_K: f64 = 2.0;
/// Itérations de la boucle de puisage d'un échangeur interne : borne de sécurité.
const NB_ITER_INTERNE_MAX: usize = 4000;

/// « Type_Application » (tableau 288).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationAccumulateur {
    /// 0 : accumulateur individuel, consigne de 55 °C.
    Individuel,
    /// 1 : accumulateur collectif, avec ou sans bouclage, consigne de 60 °C.
    Collectif,
}

impl ApplicationAccumulateur {
    /// Application par défaut du texte : individuel sous 500 L (§9.12.3.2.2).
    pub fn selon_volume(v_tot_l: f64) -> Self {
        if v_tot_l < V_INDIVIDUEL_MAX_L {
            Self::Individuel
        } else {
            Self::Collectif
        }
    }

    /// « θcons » : température de consigne du ballon en eau technique, en °C (tableau 288).
    pub fn theta_cons_c(self) -> f64 {
        match self {
            Self::Individuel => THETA_CONS_INDIVIDUEL_C,
            Self::Collectif => THETA_CONS_COLLECTIF_C,
        }
    }
}

/// Circulateur de charge du préparateur ECS (échangeur externe).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CirculateurPrimaire {
    /// « Type_Circulateur_Prep_ECS » : vrai pour un circulateur à vitesse variable.
    pub vitesse_variable: bool,
    /// « Pw_circ_prim_nom » : puissance absorbée au débit primaire nominal, en W.
    pub p_nom_w: f64,
    /// « Modmin » : rapport du débit minimal au débit nominal (vitesse variable).
    pub mod_min: f64,
}

/// Nature de l'échangeur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NatureEchangeur {
    /// Échangeur immergé (serpentin, double-peau) sur toute la hauteur du ballon.
    Interne,
    /// Échangeur à plaques à l'extérieur du ballon : piquage départ en zone haute, retour en
    /// zone basse (tableau 287), débit primaire nominal et circulateur.
    Externe {
        /// « qv_prim_nom » : débit primaire nominal, en m³/h.
        q_v_prim_nom_m3_h: f64,
        /// Circulateur du primaire.
        circulateur: CirculateurPrimaire,
    },
}

/// Échangeur d'un accumulateur d'ECS en eau technique (un des assemblages identiques).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EchangeurEcs {
    /// Nature de l'échangeur.
    pub nature: NatureEchangeur,
    /// « UAech » : coefficient de transfert de l'échangeur, en W/K (préprocesseur).
    pub ua_ech_w_k: f64,
    /// « qv,ECS,nom » : débit d'ECS au point nominal, en m³/h (éq. 1876).
    pub q_ecs_nom_m3_h: f64,
    /// « θcons » : consigne du ballon, en °C.
    pub theta_cons_c: f64,
    /// Itérations de la boucle de puisage d'un échangeur externe (éq. 1950).
    pub nb_iter_vp: usize,
}

/// Efficacité d'un échangeur à contre-courant par la méthode du NUT (éq. 1827 et 1828, 1881,
/// 1890 et 1891).
pub fn efficacite_contre_courant(nut: f64, rd: f64) -> f64 {
    if (1.0 - rd).abs() < 1e-12 {
        nut / (nut + 1.0)
    } else {
        let e = libm::exp(-nut * (1.0 - rd));
        (1.0 - e) / (1.0 - rd * e)
    }
}

/// « qv,ECS,nom » : débit d'ECS au point nominal, en m³/h (éq. 1876 et 1923) :
/// `1000 × PechECS / (ρ × c × (θcons − 5 − 15))`.
pub fn debit_ecs_nominal_m3_h(p_ech_kw: f64, theta_cons_c: f64) -> f64 {
    1000.0 * p_ech_kw / (RHO_CW_WH_L_K * 1000.0 * (theta_cons_c - 5.0 - 15.0))
}

/// « UAech » d'un échangeur externe, en W/K (éq. 1876 à 1882) : inversion de la méthode du
/// NUT à contre-courant, pour l'efficacité nominale
/// `(θcons − 5 − 15)/(θcons − 15) × min(qECS,nom ; qprim,nom)/qECS,nom`. Le texte arrête la
/// résolution (Brent) quand `|Δε| < 0,01` ; la racine est ici cherchée par dichotomie à
/// 10⁻¹² près (lecture Q106).
pub fn ua_echangeur_externe_w_k(p_ech_kw: f64, theta_cons_c: f64, q_v_prim_nom_m3_h: f64) -> f64 {
    let q_ecs = debit_ecs_nominal_m3_h(p_ech_kw, theta_cons_c);
    let q_min = q_ecs.min(q_v_prim_nom_m3_h);
    let q_max = q_ecs.max(q_v_prim_nom_m3_h);
    // Éq. (1877) et (1878).
    let eps_nom =
        (theta_cons_c - 5.0 - 15.0) / (theta_cons_c - 15.0) * q_min / q_ecs;
    let rd = q_min / q_max;
    // Éq. (1880) et (1881) : l'efficacité croît avec UA.
    let eps = |ua: f64| {
        let nut = ua / (RHO_CW_WH_L_K * 1000.0 * q_min);
        efficacite_contre_courant(nut, rd)
    };
    let mut haut = 50.0 * p_ech_kw * 1000.0;
    let mut n = 0;
    while eps(haut) < eps_nom && n < 200 {
        haut *= 2.0;
        n += 1;
    }
    let mut bas = 0.0;
    for _ in 0..200 {
        let milieu = 0.5 * (bas + haut);
        if eps(milieu) < eps_nom {
            bas = milieu;
        } else {
            haut = milieu;
        }
    }
    0.5 * (bas + haut)
}

/// « UAech » d'un échangeur interne, en W/K (éq. 1923 à 1926), pour un débit primaire infini :
/// `UA = ρ·c·qv,ECS,nom × NUTnom`, `NUTnom = −ln(1 − εnom)`, `εnom = (θcons − 5 − 15)/(θcons −
/// 15)` (lecture Q106 : le texte imprime « 50 °C − 15 °C » au numérateur, valeur de
/// l'accumulateur individuel, alors que la prose pose l'ECS à `θcons − 5` pour les deux
/// applications).
pub fn ua_echangeur_interne_w_k(p_ech_kw: f64, theta_cons_c: f64) -> f64 {
    let q_ecs = debit_ecs_nominal_m3_h(p_ech_kw, theta_cons_c);
    let eps_nom = (theta_cons_c - 5.0 - 15.0) / (theta_cons_c - 15.0);
    let nut = -libm::log(1.0 - eps_nom);
    RHO_CW_WH_L_K * 1000.0 * q_ecs * nut
}

/// Débit d'ECS moyen et durée équivalente de puisage d'un pas de temps (§9.12.3.1,
/// éq. 1866 à 1873). `v_ef_m3` : volume d'eau chaude non mitigée puisé, en m³ ; `theta_ef_c` :
/// température d'eau froide ; `theta_depart_c` : départ de la distribution. Renvoie
/// `(qv_ef_ecs en m³/h, t_puis en h)`, ou `None` sans puisage.
///
/// Lecture Q106 : `ENTIER` de l'éq. (1868) est la partie entière (la prose dit « entier
/// directement supérieur ») ; les équations (1869) à (1873) emploient 6 l/min et 0,13, la
/// prose 8 l/min.
pub fn debit_ecs_moyen(v_ef_m3: f64, theta_ef_c: f64, theta_depart_c: f64) -> Option<(f64, f64)> {
    if !(v_ef_m3 > 0.0 && theta_depart_c > theta_ef_c) {
        return None;
    }
    // Éq. (1866) et (1867) : litres.
    let r_temp = (40.0 - theta_ef_c) / (theta_depart_c - theta_ef_c);
    let v_eq_l = 8.0 * 6.0 * r_temp;
    // Éq. (1868).
    let n = libm::floor(v_ef_m3 * 1000.0 / v_eq_l).max(1.0);
    if n <= 1.0 {
        // Éq. (1869) et (1872).
        Some((0.06 * n * 6.0 * r_temp, 2.0 * 0.13))
    } else {
        // Éq. (1871), (1870) et (1873).
        let y = 0.8 / libm::sqrt(n - 1.0);
        Some((
            0.06 * n * 6.0 * r_temp * y,
            (n * y * 0.13).min(1.0),
        ))
    }
}

impl EchangeurEcs {
    /// Échangeur externe d'un accumulateur : préprocesseur (éq. 1876 à 1882).
    pub fn externe(
        p_ech_kw: f64,
        q_v_prim_nom_m3_h: f64,
        circulateur: CirculateurPrimaire,
        theta_cons_c: f64,
        nb_iter_vp: usize,
    ) -> Self {
        Self {
            nature: NatureEchangeur::Externe {
                q_v_prim_nom_m3_h,
                circulateur,
            },
            ua_ech_w_k: ua_echangeur_externe_w_k(p_ech_kw, theta_cons_c, q_v_prim_nom_m3_h),
            q_ecs_nom_m3_h: debit_ecs_nominal_m3_h(p_ech_kw, theta_cons_c),
            theta_cons_c,
            nb_iter_vp,
        }
    }

    /// Échangeur interne d'un accumulateur : préprocesseur (éq. 1923 à 1927).
    pub fn interne(p_ech_kw: f64, theta_cons_c: f64, nb_iter_vp: usize) -> Self {
        Self {
            nature: NatureEchangeur::Interne,
            ua_ech_w_k: ua_echangeur_interne_w_k(p_ech_kw, theta_cons_c),
            q_ecs_nom_m3_h: debit_ecs_nominal_m3_h(p_ech_kw, theta_cons_c),
            theta_cons_c,
            nb_iter_vp,
        }
    }

    /// Puisage d'un pas de temps pour `q_unit_wh` à fournir par ce ballon (éq. 1952, report
    /// compris), à la place de `boucle_puisage` ; `v_ef_m3` : volume d'eau chaude non mitigée
    /// puisé aux points de puisage, qui fixe les débits de l'échangeur : renvoie le volume puisé (pour un échangeur interne, le
    /// volume d'eau chaude équivalent à l'énergie prélevée), l'énergie restant à fournir et la
    /// consommation du circulateur primaire, en Wh.
    pub fn puiser(
        &self,
        ballon: &mut Ballon,
        q_unit_wh: f64,
        v_ef_m3: f64,
        theta_ef_c: f64,
        theta_depart_c: f64,
    ) -> (Puisage, f64) {
        let vide = Puisage {
            v_p_l: 0.0,
            q_report_wh: q_unit_wh.max(0.0),
        };
        if !(q_unit_wh > 0.0 && theta_depart_c > theta_ef_c) {
            return (vide, 0.0);
        }
        // « Vef,ecs » : volume d'eau chaude non mitigée puisé au pas de temps, pour un
        // assemblage (lecture Q106 : défini par le texte comme une entrée, sans équation).
        let Some((qv_ef, t_puis)) = debit_ecs_moyen(v_ef_m3, theta_ef_c, theta_depart_c) else {
            return (vide, 0.0);
        };
        match self.nature {
            NatureEchangeur::Externe {
                q_v_prim_nom_m3_h,
                circulateur,
            } => self.puiser_externe(
                ballon,
                q_unit_wh,
                theta_ef_c,
                theta_depart_c,
                qv_ef,
                t_puis,
                q_v_prim_nom_m3_h,
                circulateur,
            ),
            NatureEchangeur::Interne => {
                (self.puiser_interne(ballon, q_unit_wh, theta_ef_c, theta_depart_c, qv_ef), 0.0)
            }
        }
    }

    /// Th-BCE 2020 §9.12.3.2.4.1, éq. (1884) à (1908).
    #[allow(clippy::too_many_arguments)]
    fn puiser_externe(
        &self,
        ballon: &mut Ballon,
        q_unit_wh: f64,
        theta_ef_c: f64,
        theta_depart_c: f64,
        qv_ef: f64,
        t_puis: f64,
        q_v_prim_nom_m3_h: f64,
        circ: CirculateurPrimaire,
    ) -> (Puisage, f64) {
        // Éq. (1884) et (1885).
        let qv_aval = qv_ef;
        let theta_retour_aval = theta_ef_c;
        // Le débit primaire variable est proportionnel au rapport des débits d'ECS
        // (éq. 1887).
        let q_ecs_nom = self.q_ecs_nom_m3_h;
        // Éq. (1886) et (1887).
        let qv_prim = if circ.vitesse_variable {
            (qv_aval / q_ecs_nom).max(circ.mod_min) * q_v_prim_nom_m3_h
        } else {
            q_v_prim_nom_m3_h
        };
        // Éq. (1888) à (1891).
        let q_min = qv_aval.min(qv_prim);
        let nut = self.ua_ech_w_k / (RHO_CW_WH_L_K * 1000.0 * q_min);
        let rd = q_min / qv_aval.max(qv_prim);
        let eps = efficacite_contre_courant(nut, rd);
        // Éq. (1892).
        let theta_entree_ech = theta_retour_aval
            + qv_aval / (eps * q_min) * (theta_depart_c - theta_retour_aval);
        let mut reste = q_unit_wh;
        let mut v_total = 0.0;
        for _ in 0..self.nb_iter_vp {
            if reste <= 0.0 {
                break;
            }
            // Éq. (1893) : zone haute (piquage départ, tableau 287).
            let theta_sortie_ballon = ballon.theta_c[NB_ZONES - 1];
            // Éq. (1900) à (1904) : température du ballon insuffisante, arrêt du puisage.
            if theta_sortie_ballon < theta_entree_ech {
                break;
            }
            // Éq. (1894) et (1895).
            let ecart = if circ.vitesse_variable {
                theta_sortie_ballon - theta_retour_aval
            } else {
                theta_entree_ech - theta_retour_aval
            };
            let theta_sortie_ech = theta_sortie_ballon - eps * q_min / qv_aval * ecart;
            let delta = theta_sortie_ballon - theta_sortie_ech;
            if delta <= 0.0 {
                break;
            }
            // Éq. (1897) et (1898).
            let v_puisage = reste / (RHO_CW_WH_L_K * delta);
            let v_p = v_puisage.min(ballon.v_z_min_l());
            // Éq. (1899).
            reste -= v_p * RHO_CW_WH_L_K * delta;
            if reste < 1e-9 {
                reste = 0.0;
            }
            v_total += v_p;
            // Éq. (1896) : l'eau revient en zone basse à la sortie de l'échangeur.
            ballon.puiser(v_p, theta_sortie_ech);
        }
        // Éq. (1907) et (1908) : consommation du circulateur primaire, comptée que le puisage
        // ait pu être assuré ou non.
        let w_circ = if circ.vitesse_variable {
            libm::pow(qv_prim / q_v_prim_nom_m3_h, 2.0 / 3.0).min(1.0) * circ.p_nom_w * t_puis
        } else {
            circ.p_nom_w * t_puis
        };
        (
            Puisage {
                v_p_l: v_total,
                q_report_wh: reste,
            },
            w_circ,
        )
    }

    /// Th-BCE 2020 §9.12.3.3.4.1, éq. (1928) à (1940) : énergie prélevée dans chacune des
    /// zones par l'échangeur interne, itération par itération.
    fn puiser_interne(
        &self,
        ballon: &mut Ballon,
        q_unit_wh: f64,
        theta_ef_c: f64,
        theta_depart_c: f64,
        qv_ef: f64,
    ) -> Puisage {
        let theta_entrant = theta_ef_c;
        let theta_retour_aval = theta_ef_c;
        let v_tot = ballon.v_tot_l();
        // Éq. (1927) : `UAech[z] = Vz/V × UAech`.
        let nut_z: [f64; NB_ZONES] = std::array::from_fn(|z| {
            // Éq. (1930).
            self.ua_ech_w_k * ballon.v_l[z] / v_tot / (qv_ef * RHO_CW_WH_L_K * 1000.0)
        });
        // Éq. (1931).
        let q_iter = v_tot * RHO_CW_WH_L_K * PAS_ITERATION_K;
        let mut reste = q_unit_wh;
        let mut q_total = 0.0;
        for _ in 0..NB_ITER_INTERNE_MAX {
            if reste <= 1e-9 {
                reste = reste.max(0.0);
                if reste < 1e-9 {
                    reste = 0.0;
                }
                break;
            }
            // Éq. (1932).
            let q_w = reste.min(q_iter);
            // Éq. (1933) à (1936) : l'indice 0 est la zone 1.
            let mut q_max = [0.0; NB_ZONES];
            let mut d_theta = [0.0; NB_ZONES];
            let mut somme = 0.0;
            for z in 0..NB_ZONES {
                let base = if z == 0 { theta_retour_aval } else { theta_entrant };
                q_max[z] = ballon.v_l[z] * RHO_CW_WH_L_K * (ballon.theta_c[z] - base).max(0.0);
                d_theta[z] = ((ballon.theta_c[z] - theta_retour_aval - somme)
                    / (1.0 / nut_z[z] + 0.5))
                    .max(0.0);
                somme += d_theta[z];
            }
            // Éq. (1937).
            let theta_sortie_ballon = theta_entrant + somme;
            // Éq. (1938) et (1939).
            let mut preleve = [0.0; NB_ZONES];
            if theta_sortie_ballon >= theta_depart_c && theta_sortie_ballon > theta_retour_aval {
                for z in 0..NB_ZONES {
                    preleve[z] = q_max[z]
                        .min(d_theta[z] / (theta_sortie_ballon - theta_retour_aval) * q_w);
                }
            }
            let q_pris: f64 = preleve.iter().sum();
            // Éq. (1940).
            reste -= q_pris;
            q_total += q_pris;
            if q_pris <= 1e-9 {
                break;
            }
            // Les températures des zones sont recalculées, sans pertes ni apports.
            for z in 0..NB_ZONES {
                ballon.theta_c[z] -= preleve[z] / (RHO_CW_WH_L_K * ballon.v_l[z]);
            }
            ballon.melanger();
        }
        let v_equivalent = if theta_depart_c > theta_ef_c {
            q_total / (RHO_CW_WH_L_K * (theta_depart_c - theta_ef_c))
        } else {
            0.0
        };
        Puisage {
            v_p_l: v_equivalent,
            q_report_wh: reste.max(0.0),
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn ballon_uniforme(v: f64, theta: f64) -> Ballon {
        let mut b = Ballon::nouveau(v, 0.0, None, 95.0);
        b.fixer_temperature(theta);
        b
    }

    #[test]
    fn consignes_du_tableau_288() {
        assert_eq!(ApplicationAccumulateur::selon_volume(300.0).theta_cons_c(), 55.0);
        assert_eq!(ApplicationAccumulateur::selon_volume(500.0).theta_cons_c(), 60.0);
    }

    /// Débits équilibrés (qprim,nom = qECS,nom) : `εnom = 35/40` donne `NUT = ε/(1 − ε) = 7`,
    /// d'où `UA = ρ c q × NUT = 7 × P/35 = 4 000 W/K` pour 20 kW (éq. 1876 à 1881).
    #[test]
    fn ua_externe_a_debits_equilibres() {
        let q_ecs: f64 = 1000.0 * 20.0 / (1163.0 * 35.0);
        let ua = ua_echangeur_externe_w_k(20.0, 55.0, q_ecs);
        assert!((ua - 4000.0).abs() < 1e-6, "{ua}");
    }

    /// `εnom = 35/40`, `NUT = −ln(1 − 0,875)`, `UA = 10 000/35 × NUT` (éq. 1923 à 1926).
    #[test]
    fn ua_interne() {
        let ua = ua_echangeur_interne_w_k(10.0, 55.0);
        assert!((ua - 10_000.0 / 35.0 * 2.0794415416798357).abs() < 1e-9, "{ua}");
    }

    /// 30 L d'eau chaude à 55 °C pour de l'eau froide à 15 °C : un seul puisage équivalent de
    /// 30 L (note de l'éq. 1867), `qv = 0,06 × 6 × 0,625 = 0,225 m³/h`, `tpuis = 0,26 h` ;
    /// 90 L : `N = 3`, `Y = 0,8/√2` (éq. 1868 à 1873).
    #[test]
    fn debit_equivalent() {
        let (q, t) = debit_ecs_moyen(0.030, 15.0, 55.0).unwrap();
        assert!((q - 0.225).abs() < 1e-12 && (t - 0.26).abs() < 1e-12);
        let (q, t) = debit_ecs_moyen(0.090, 15.0, 55.0).unwrap();
        let y = 0.8 / 2.0_f64.sqrt();
        assert!((q - 0.06 * 3.0 * 6.0 * 0.625 * y).abs() < 1e-12);
        assert!((t - 3.0 * y * 0.13).abs() < 1e-12);
        assert!(debit_ecs_moyen(0.0, 15.0, 55.0).is_none());
    }

    /// Échangeur externe, circulateur à vitesse constante, ballon uniforme à 55 °C de 300 L,
    /// demande de 2 000 Wh ; calcul à la main avec les éq. (1888) à (1899).
    #[test]
    fn puisage_externe_a_la_main() {
        let q_ecs_nom: f64 = 1000.0 * 20.0 / (1163.0 * 35.0);
        let circ = CirculateurPrimaire {
            vitesse_variable: false,
            p_nom_w: 60.0,
            mod_min: MOD_MIN_DEFAUT,
        };
        let e = EchangeurEcs::externe(20.0, q_ecs_nom, circ, 55.0, 4);
        let mut b = ballon_uniforme(300.0, 55.0);
        let (p, w) = e.puiser(&mut b, 2000.0, 2000.0 / (1.163 * 38.0) / 1000.0, 15.0, 53.0);
        // Calcul indépendant.
        let v_ef: f64 = 2000.0 / (1.163 * 38.0) / 1000.0;
        let r_temp: f64 = 25.0 / 38.0;
        let n = libm::floor(v_ef * 1000.0 / (48.0 * r_temp)).max(1.0);
        assert_eq!(n, 1.0);
        let qv: f64 = 0.36 * r_temp;
        let nut: f64 = 4000.0 / (1163.0 * qv.min(q_ecs_nom));
        let rd = qv.min(q_ecs_nom) / qv.max(q_ecs_nom);
        let ex = libm::exp(-nut * (1.0 - rd));
        let eps = (1.0 - ex) / (1.0 - rd * ex);
        let q_min = qv.min(q_ecs_nom);
        let theta_entree = 15.0 + qv / (eps * q_min) * 38.0;
        let theta_sortie_ech = 55.0 - eps * q_min / qv * (theta_entree - 15.0);
        let delta = 55.0 - theta_sortie_ech;
        let v_p: f64 = (2000.0 / (1.163 * delta)).min(75.0);
        assert!((p.v_p_l - v_p).abs() < 1e-9, "{} {v_p}", p.v_p_l);
        assert_eq!(p.q_report_wh, 0.0);
        assert!((w - 60.0 * 0.26).abs() < 1e-12);
        // Bilan : l'eau revenue en zone basse est à `theta_sortie_ech`.
        let attendu_bas = (55.0 * (75.0 - v_p) + theta_sortie_ech * v_p) / 75.0;
        assert!((b.theta_c[0] - attendu_bas).abs() < 1e-9);
    }

    /// Ballon trop froid pour l'échangeur : pas de puisage, énergie reportée, circulateur
    /// compté tout de même (éq. 1900 à 1904 et 1907).
    #[test]
    fn puisage_externe_ballon_froid() {
        let circ = CirculateurPrimaire {
            vitesse_variable: false,
            p_nom_w: 60.0,
            mod_min: MOD_MIN_DEFAUT,
        };
        let e = EchangeurEcs::externe(20.0, 0.5, circ, 55.0, 4);
        let mut b = ballon_uniforme(300.0, 30.0);
        let (p, w) = e.puiser(&mut b, 2000.0, 2000.0 / (1.163 * 38.0) / 1000.0, 15.0, 53.0);
        assert_eq!((p.v_p_l, p.q_report_wh), (0.0, 2000.0));
        assert!(w > 0.0);
        assert!(b.theta_c.iter().all(|&t| t == 30.0));
    }

    /// Échangeur interne, ballon uniforme à 55 °C de 300 L, 500 Wh demandés (une seule
    /// itération, `Qiter = 300 × 1,163 × 2 = 697,8 Wh`) : calcul à la main des éq. (1930) à
    /// (1938), puis bilan d'énergie du ballon.
    #[test]
    fn puisage_interne_a_la_main() {
        let e = EchangeurEcs::interne(10.0, 55.0, 4);
        let mut b = ballon_uniforme(300.0, 55.0);
        let avant = b.energie_wh(15.0);
        let (p, w) = e.puiser(&mut b, 500.0, 500.0 / (1.163 * 30.0) / 1000.0, 15.0, 45.0);
        assert_eq!(w, 0.0);
        assert!(p.q_report_wh < 1e-9);
        // Calcul indépendant.
        let v_ef: f64 = 500.0 / (1.163 * 30.0) / 1000.0;
        let r_temp: f64 = 25.0 / 30.0;
        let n = libm::floor(v_ef * 1000.0 / (48.0 * r_temp)).max(1.0);
        assert_eq!(n, 1.0);
        let qv: f64 = 0.36 * r_temp;
        let ua: f64 = 10_000.0 / 35.0 * 2.0794415416798357;
        let nut: f64 = ua / 4.0 / (qv * 1163.0);
        let mut d = [0.0_f64; 4];
        let mut somme = 0.0_f64;
        for z in 0..4 {
            d[z] = ((55.0 - 15.0 - somme) / (1.0 / nut + 0.5)).max(0.0);
            somme += d[z];
        }
        let theta_sortie = 15.0 + somme;
        assert!(theta_sortie >= 45.0);
        for z in 0..4 {
            let q_z: f64 = (d[z] / (theta_sortie - 15.0) * 500.0).min(75.0 * 1.163 * 40.0);
            let attendu = 55.0 - q_z / (1.163 * 75.0);
            assert!((b.theta_c[z] - attendu).abs() < 1e-9, "zone {z}");
        }
        assert!((avant - b.energie_wh(15.0) - 500.0).abs() < 1e-6);
    }

    /// Plusieurs itérations (1 500 Wh > Qiter) : le bilan du ballon se ferme.
    #[test]
    fn puisage_interne_plusieurs_iterations() {
        let e = EchangeurEcs::interne(10.0, 55.0, 4);
        let mut b = ballon_uniforme(300.0, 55.0);
        let avant = b.energie_wh(15.0);
        let (p, _) = e.puiser(&mut b, 1500.0, 1500.0 / (1.163 * 30.0) / 1000.0, 15.0, 45.0);
        let apres = b.energie_wh(15.0);
        assert!((avant - apres - (1500.0 - p.q_report_wh)).abs() < 1e-6);
        assert!(p.q_report_wh < 1500.0);
    }

    /// Ballon trop froid pour un échangeur interne : rien n'est prélevé (éq. 1939).
    #[test]
    fn puisage_interne_ballon_froid() {
        let e = EchangeurEcs::interne(10.0, 55.0, 4);
        let mut b = ballon_uniforme(300.0, 30.0);
        let (p, _) = e.puiser(&mut b, 1000.0, 1000.0 / (1.163 * 38.0) / 1000.0, 15.0, 53.0);
        assert_eq!((p.v_p_l, p.q_report_wh), (0.0, 1000.0));
    }
}
