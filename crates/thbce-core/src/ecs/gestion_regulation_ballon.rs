// SPDX-License-Identifier: GPL-3.0-or-later
//! Gestion/régulation du ballon de stockage : fiche `C_STO_gestion_régulation_ballon`
//! (Th-BCE 2020 §9.10).
//!
//! La fiche calcule le volume puisé dans le ballon par une boucle itérative (éq. 1791 à 1799),
//! l'autorisation de fonctionnement des générateurs selon leur programmation (éq. 1810 à
//! 1812), leur enclenchement par le thermostat (éq. 1813 et 1814) et l'énergie qu'ils doivent
//! fournir au ballon (éq. 1815).
//!
//! La gestion optimisée de l'appoint (§9.10.3.3, éq. 1816 à 1824), l'appoint séparé instantané
//! (éq. 1800 à 1807) et les échangeurs côté distribution (éq. 1808 et 1809) sont hors de cette
//! version. Lectures : `docs/questions.md`, Q48.

use super::ballon_de_stockage::{Ballon, NB_ZONES, RHO_CW_WH_L_K};

/// « θc_base », « θc_ap » : consigne conventionnelle d'un ballon d'ECS, en °C (éq. 1948).
pub const THETA_C_ECS_C: f64 = 55.0;
/// « Δθbase », « Δθap » par défaut, en K (§9.10.3.2.2, « statut_delta_theta = 2 »).
pub const DELTA_THETA_DEFAUT_K: f64 = 2.0;

/// « typegest » : programmation du thermostat d'un générateur de ballon (§9.10.3.2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Programmation {
    /// 0 : fonctionnement permanent (éq. 1810).
    Permanent,
    /// 1 : de nuit seulement (éq. 1811).
    Nuit,
    /// 2 : de jour seulement (éq. 1812).
    Jour,
}

/// « fp(h) » : autorisation de fonctionnement selon la programmation et l'heure légale du pas
/// de temps (éq. 1810 à 1812) : la nuit, `hleg > 23 h` ou `hleg < 5 h` ; le jour,
/// `10 h < hleg < 17 h`.
pub fn f_p(programmation: Programmation, hleg: u8) -> bool {
    match programmation {
        Programmation::Permanent => true,
        Programmation::Nuit => !(5..=23).contains(&hleg),
        Programmation::Jour => hleg > 10 && hleg < 17,
    }
}

/// « Nbiter_vp » : nombre maximal d'itérations de la boucle de puisage (éq. 1950) : 4 pour un
/// ballon à base seule, `arrondi.inf(2 / min(faux ; 1 − faux))` avec un appoint intégré.
pub fn nb_iter_vp(f_aux: Option<f64>) -> usize {
    match f_aux {
        None => NB_ZONES,
        Some(f) => (2.0 / f.min(1.0 - f)).floor().max(1.0) as usize,
    }
}

/// Résultat de la boucle de puisage.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Puisage {
    /// « Vp(h) » : volume puisé au pas de temps, en L (éq. 1799).
    pub v_p_l: f64,
    /// « Qw_sto_unit_report » : énergie qui n'a pas pu être puisée, en Wh.
    pub q_report_wh: f64,
}

/// Boucle itérative de puisage d'ECS (§9.10.3.1.1, éq. 1791 à 1799).
///
/// À chaque itération, le volume puisé est calculé avec la température de la zone supérieure
/// et ne peut excéder le volume de la plus petite zone ; l'eau n'est puisée que si la zone
/// supérieure est plus chaude que la température de départ attendue `theta_cons_sortie_c`.
pub fn boucle_puisage(
    ballon: &mut Ballon,
    q_w_sto_unit_wh: f64,
    theta_entrant_c: f64,
    theta_cons_sortie_c: f64,
    nb_iter: usize,
) -> Puisage {
    let mut reste = q_w_sto_unit_wh;
    let mut v_total = 0.0;
    for _ in 0..nb_iter {
        if reste <= 0.0 {
            break;
        }
        let theta_p = ballon.theta_c[NB_ZONES - 1];
        if theta_p <= theta_cons_sortie_c || theta_p <= theta_entrant_c {
            // Éq. (1795) et (1798) : pas de puisage, l'énergie est reportée.
            break;
        }
        // Éq. (1794) et (1797).
        let par_litre = RHO_CW_WH_L_K * (theta_p - theta_entrant_c);
        let v_p = (reste / par_litre).min(ballon.v_z_min_l());
        reste -= par_litre * v_p;
        if reste < 1e-9 {
            reste = 0.0;
        }
        v_total += v_p;
        ballon.puiser(v_p, theta_entrant_c);
    }
    Puisage {
        v_p_l: v_total,
        q_report_wh: reste,
    }
}

/// Boucle itérative de puisage d'un assemblage à appoint séparé instantané
/// (« Type_prod_stockage = 3 », §9.10.3.1.2, éq. 1800 à 1807, relues sur l'image de la
/// p. 1106).
///
/// Tant que la zone supérieure est au moins à la température de départ attendue, le puisage
/// est celui des autres assemblages. Sinon, si elle est plus chaude que l'eau entrante, l'eau
/// préchauffée est puisée avec l'écart de température constant `θcons,sortie − θentrant` : le
/// volume est celui qu'il faudrait à la température attendue et l'énergie puisée, plus faible,
/// laisse un reste fourni par l'appoint instantané ; la boucle s'arrête alors.
pub fn boucle_puisage_appoint_separe(
    ballon: &mut Ballon,
    q_w_sto_unit_wh: f64,
    theta_entrant_c: f64,
    theta_cons_sortie_c: f64,
    nb_iter: usize,
) -> Puisage {
    let mut reste = q_w_sto_unit_wh;
    let mut v_total = 0.0;
    for _ in 0..nb_iter {
        if reste <= 0.0 {
            break;
        }
        let theta_p = ballon.theta_c[NB_ZONES - 1];
        let (v_p, fin) = if theta_p >= theta_cons_sortie_c && theta_p > theta_entrant_c {
            // Éq. (1801) et (1805).
            let v = (reste / (RHO_CW_WH_L_K * (theta_p - theta_entrant_c))).min(ballon.v_z_min_l());
            (v, false)
        } else if theta_p > theta_entrant_c {
            // Éq. (1802) et (1806).
            let v = (reste / (RHO_CW_WH_L_K * (theta_cons_sortie_c - theta_entrant_c)))
                .min(ballon.v_z_min_l());
            (v, true)
        } else {
            // Éq. (1803) et (1807).
            break;
        };
        reste -= RHO_CW_WH_L_K * v_p * (theta_p - theta_entrant_c);
        if reste < 1e-9 {
            reste = 0.0;
        }
        v_total += v_p;
        ballon.puiser(v_p, theta_entrant_c);
        if fin {
            break;
        }
    }
    Puisage {
        v_p_l: v_total,
        q_report_wh: reste,
    }
}

/// « iactive » : enclenchement d'un générateur par son thermostat (éq. 1813 et 1814).
///
/// Le générateur s'enclenche s'il y a eu puisage, si la zone de la sonde est passée sous
/// `θc − Δθ`, ou si elle se trouve dans la bande d'hystérésis en cours de remontée. Pour le
/// générateur de base, la zone de la sonde doit en outre être sous la consigne majorée de
/// l'effet de ses pertes (éq. 1814 ; lecture Q48).
pub fn active(
    ballon: &Ballon,
    z_reg: usize,
    theta_c_c: f64,
    delta_theta_k: f64,
    puisage: bool,
    base: bool,
) -> bool {
    let t1 = ballon.theta_prec_c[z_reg];
    let t2 = ballon.theta_prec2_c[z_reg];
    let enclenche = puisage
        || t1 < theta_c_c - delta_theta_k
        || (t1 >= theta_c_c - delta_theta_k && t1 < theta_c_c && t2 < t1);
    if !base {
        return enclenche;
    }
    enclenche && t1 < theta_c_c + ballon.pertes_prec_wh[z_reg] / (RHO_CW_WH_L_K * ballon.v_l[z_reg])
}

/// « Qreq_sto » : énergie à fournir au ballon par un générateur dont l'échangeur est dans la
/// zone d'indice `z_gen`, en Wh (éq. 1815) : remontée à la consigne des zones situées à partir
/// de celle de l'échangeur, augmentée de leurs pertes du pas de temps si `avec_pertes`.
pub fn q_req_sto_wh(ballon: &Ballon, z_gen: usize, theta_c_c: f64, avec_pertes: bool) -> f64 {
    let mut q = 0.0;
    for z in z_gen..NB_ZONES {
        q += RHO_CW_WH_L_K * ballon.v_l[z] * (theta_c_c - ballon.theta_c[z]);
        if avec_pertes {
            q += ballon.pertes_wh[z];
        }
    }
    q.max(0.0)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn programmation() {
        assert!(f_p(Programmation::Permanent, 12));
        // Nuit : hleg < 5 h.
        assert!(f_p(Programmation::Nuit, 0) && f_p(Programmation::Nuit, 4));
        assert!(!f_p(Programmation::Nuit, 5) && !f_p(Programmation::Nuit, 23));
        // Jour : 10 h < hleg < 17 h.
        assert!(f_p(Programmation::Jour, 11) && f_p(Programmation::Jour, 16));
        assert!(!f_p(Programmation::Jour, 10) && !f_p(Programmation::Jour, 17));
        assert_eq!(nb_iter_vp(None), 4);
        assert_eq!(nb_iter_vp(Some(0.5)), 4);
        assert_eq!(nb_iter_vp(Some(0.3)), 6);
    }

    #[test]
    fn puisage_iteratif() {
        // Ballon de 200 L à 55 °C, eau froide à 15 °C : 1 163 Wh demandent 25 L.
        let mut b = Ballon::nouveau(200.0, 0.0, None, 90.0);
        b.fixer_temperature(55.0);
        let p = boucle_puisage(&mut b, 1163.0, 15.0, 53.0, 4);
        assert!((p.v_p_l - 25.0).abs() < 1e-9);
        assert_eq!(p.q_report_wh, 0.0);
        assert!((b.theta_c[0] - 35.0).abs() < 1e-9);
        assert_eq!(b.theta_c[3], 55.0);
        // Demande de 6 000 Wh : 50 L par itération (2 326 Wh) ; à la troisième, la zone haute
        // ne contient plus que l'eau à 35 °C montée du bas, sous les 53 °C attendus : le reste
        // est reporté.
        let mut b = Ballon::nouveau(200.0, 0.0, None, 90.0);
        b.fixer_temperature(55.0);
        b.theta_c = [35.0, 35.0, 55.0, 55.0];
        let p = boucle_puisage(&mut b, 6000.0, 15.0, 53.0, 4);
        assert!((p.v_p_l - 100.0).abs() < 1e-9);
        assert!((p.q_report_wh - (6000.0 - 2.0 * 2326.0)).abs() < 1e-9);
        // Zone haute trop froide dès le départ : rien n'est puisé.
        let mut b = Ballon::nouveau(200.0, 0.0, None, 90.0);
        let p = boucle_puisage(&mut b, 500.0, 15.0, 53.0, 4);
        assert_eq!((p.v_p_l, p.q_report_wh), (0.0, 500.0));
    }

    #[test]
    fn puisage_avec_appoint_separe_instantane() {
        // Ballon à 55 °C : puisage ordinaire (éq. 1801).
        let mut b = Ballon::nouveau(200.0, 0.0, None, 90.0);
        b.fixer_temperature(55.0);
        let p = boucle_puisage_appoint_separe(&mut b, 1163.0, 15.0, 53.0, 4);
        assert!((p.v_p_l - 25.0).abs() < 1e-9);
        assert_eq!(p.q_report_wh, 0.0);
        // Ballon préchauffé à 35 °C, sous les 53 °C attendus (éq. 1802) : le volume est calculé
        // avec 53 − 15 = 38 K, 1 000/(1,163 × 38) = 22,63 L, et l'énergie puisée avec 20 K ;
        // le reste, 1 000 × (1 − 20/38) = 473,7 Wh, va à l'appoint instantané.
        let mut b = Ballon::nouveau(200.0, 0.0, None, 90.0);
        b.fixer_temperature(35.0);
        let p = boucle_puisage_appoint_separe(&mut b, 1000.0, 15.0, 53.0, 4);
        assert!((p.v_p_l - 1000.0 / (1.163 * 38.0)).abs() < 1e-9);
        assert!((p.q_report_wh - 1000.0 * (1.0 - 20.0 / 38.0)).abs() < 1e-9);
        // Eau du ballon à la température de l'eau froide : rien n'est puisé (éq. 1803).
        let mut b = Ballon::nouveau(200.0, 0.0, None, 90.0);
        b.fixer_temperature(15.0);
        let p = boucle_puisage_appoint_separe(&mut b, 500.0, 15.0, 53.0, 4);
        assert_eq!((p.v_p_l, p.q_report_wh), (0.0, 500.0));
    }

    #[test]
    fn enclenchement_et_energie_requise() {
        let mut b = Ballon::nouveau(200.0, 2.0, None, 90.0);
        b.fixer_temperature(54.0);
        // Dans la bande d'hystérésis, sans puisage ni remontée : pas d'enclenchement.
        assert!(!active(&b, 0, 55.0, 2.0, false, true));
        // Puisage : enclenchement.
        assert!(active(&b, 0, 55.0, 2.0, true, true));
        // Sous θc − Δθ.
        b.fixer_temperature(52.9);
        assert!(active(&b, 0, 55.0, 2.0, false, true));
        // En remontée dans la bande.
        b.fixer_temperature(54.0);
        b.theta_prec2_c = [53.0; 4];
        assert!(active(&b, 0, 55.0, 2.0, false, false));
        // Base : pas d'enclenchement au-dessus de la consigne, même avec un puisage (éq. 1814).
        b.fixer_temperature(60.0);
        assert!(!active(&b, 0, 55.0, 2.0, true, true));
        assert!(active(&b, 0, 55.0, 2.0, true, false));
        // Éq. (1815) : ballon à 50 °C dans une ambiance à 20 °C : 1,163 × 200 × 5 + 60 Wh.
        let mut b = Ballon::nouveau(200.0, 2.0, None, 90.0);
        b.debut_pas(20.0);
        assert!((q_req_sto_wh(&b, 0, 55.0, true) - (1163.0 + 60.0)).abs() < 1e-9);
        // Appoint en zone 3 : moitié haute seulement, sans les pertes.
        assert!((q_req_sto_wh(&b, 2, 55.0, false) - 581.5).abs() < 1e-9);
        b.fixer_temperature(56.0);
        assert_eq!(q_req_sto_wh(&b, 0, 55.0, false), 0.0);
    }
}
