// SPDX-License-Identifier: GPL-3.0-or-later
//! Ventilation mécanique simple flux par extraction ou par insufflation : fiche
//! `C_VEN_Mécanique_SF` (Th-BCE 2020 §6.5), en résidentiel.
//!
//! - puissance moyenne du ventilateur (éq. 640, 641 et 643) et répartition par groupe (éq. 642
//!   et 644) ; consommation (éq. 645 et 646) ;
//! - températures à l'extraction, après les pertes du conduit et le ventilateur (éq. 647 à 654) ;
//! - températures à l'insufflation, après le ventilateur et les pertes du conduit (éq. 663 à
//!   669) ; masses volumiques (éq. 656 et 670).
//!
//! Les fonctions de ce module servent aussi aux fiches 6.4 (basse pression) et 6.3 (double
//! flux), qui écrivent les mêmes relations. Le puits climatique et le puits hydraulique
//! (éq. 628 à 635, 657 à 661) sont hors de la première version : l'air neuf est l'air
//! extérieur (éq. 657).

/// « Cpa » : chaleur massique de l'air, en J/(kg.K) (tableaux 63 et 64).
pub const C_PA_J_KG_K: f64 = 1006.0;
/// « pel » (« ηel,vent ») : taux de récupération de la puissance des ventilateurs sous forme de
/// chaleur dans l'air (tableaux 61, 63 et 64).
pub const P_EL: f64 = 0.8;
/// « ρref » : masse volumique de l'air de référence, en kg/m³ (tableau 64).
pub const RHO_REF_KG_M3: f64 = 1.2;
/// « θref » : température de l'air de référence, en °C (tableau 61).
pub const THETA_REF_C: f64 = 20.0;
/// Résistance superficielle ajoutée à celle du conduit, en m².K/W (éq. 395, 625, 650 et 666).
pub const R_SUPERFICIELLE_COND_M2K_W: f64 = 0.02;
/// Débit massique en deçà duquel un ventilateur n'échauffe pas l'air, en kg/s (§6.3.3.4.8).
pub const QM_NUL_KG_S: f64 = 1e-6;
/// Nombre d'heures d'une semaine.
const NB_HEURES_SEMAINE: f64 = 168.0;

/// « Dugd_equ » : durée d'utilisation du grand débit retenue pour le ventilateur, en h par
/// semaine : la plus grande des bouches du système (éq. 619, 640 et, en double flux, 390).
pub fn dugd_equ_h(dugd_h: impl IntoIterator<Item = f64>) -> f64 {
    dugd_h.into_iter().fold(0.0, f64::max)
}

/// « Pvent » : puissance moyenne d'un ventilateur en résidentiel, en W, quel que soit `Ivent`.
///
/// Th-BCE 2020 §6.5.3.2.2, éq. (641) et (643) (et §6.4.4, éq. 620 ; §6.3.3.3.1, éq. 391 et
/// 393) : `(Pventpointe × Dugd_equ + Pventbase × (168 − Dugd_equ))/168`.
pub fn p_vent_residentiel_w(p_vent_base_w: f64, p_vent_pointe_w: f64, dugd_equ_h: f64) -> f64 {
    (p_vent_pointe_w * dugd_equ_h + p_vent_base_w * (NB_HEURES_SEMAINE - dugd_equ_h))
        / NB_HEURES_SEMAINE
}

/// « Pvent^g » : part de la puissance d'un ventilateur attribuée à un groupe, en W.
///
/// Th-BCE 2020 §6.5.3.2.2, éq. (642) et (644) : au prorata des débits spécifiques repris (ou
/// soufflés) du groupe. Nul si le système n'a aucun débit.
pub fn p_vent_groupe_w(p_vent_w: f64, q_spec_groupe_m3_h: f64, q_spec_total_m3_h: f64) -> f64 {
    if q_spec_total_m3_h == 0.0 {
        0.0
    } else {
        p_vent_w * q_spec_groupe_m3_h / q_spec_total_m3_h
    }
}

/// Masse volumique de l'air à une température donnée, en kg/m³.
///
/// Th-BCE 2020 §6.5.3.4.2, éq. (656), (662) et (670) (et §6.3, éq. 412, 418 et 564) :
/// `ρref·(273 + θref)/(273 + θ)`.
pub fn rho_kg_m3(theta_c: f64) -> f64 {
    RHO_REF_KG_M3 * (273.0 + THETA_REF_C) / (273.0 + theta_c)
}

/// « Hcond,rep,ext » ou « Hcond,souf,ext » : coefficient de transfert thermique de la partie
/// d'un conduit située hors volume chauffé, en W/K.
///
/// Th-BCE 2020 éq. (650) et (666) (et 395, 625) : `Acond,ext/(R + 0,02)`.
pub fn h_cond_ext_w_k(a_cond_ext_m2: f64, r_m2k_w: f64) -> f64 {
    a_cond_ext_m2 / (r_m2k_w + R_SUPERFICIELLE_COND_M2K_W)
}

/// « TLnc » : température du local non chauffé que traverse un conduit, en °C.
///
/// Th-BCE 2020 éq. (649) et (665) (et 471, 624) : `(1 − b) × θi,g,fin + b × θext`.
pub fn t_lnc_c(b: f64, theta_i_fin_c: f64, theta_ext_c: f64) -> f64 {
    (1.0 - b) * theta_i_fin_c + b * theta_ext_c
}

/// « θi,cond » : température moyenne de l'air repris entre l'air du groupe et l'air infiltré
/// dans le conduit hors volume chauffé, en °C.
///
/// Th-BCE 2020 éq. (648) (et 470, 623) :
/// `(qspec_repris × θi,fin + (qrep,cond − qspec_repris) × TLnc)/qrep,cond`. Les débits sont de
/// même signe ; un débit vu par le ventilateur nul donne `θi,fin`.
pub fn theta_i_cond_c(
    q_spec_repris: f64,
    q_rep_cond: f64,
    theta_i_fin_c: f64,
    t_lnc_c: f64,
) -> f64 {
    if q_rep_cond == 0.0 {
        theta_i_fin_c
    } else {
        (q_spec_repris * theta_i_fin_c + (q_rep_cond - q_spec_repris) * t_lnc_c) / q_rep_cond
    }
}

/// Température de l'air à la sortie d'un conduit qui échange avec un local non chauffé, en °C.
///
/// Th-BCE 2020 éq. (647) et (664) (et 469, 562, 622) :
/// `T − (T − TLnc) × (1 − exp(−Hcond,ext/(Cpa × |qm|)))`, `qm` étant le débit massique dans le
/// conduit en kg/s (`ρ × q/3600` dans les éq. 647 et 664). Sans débit, l'air est à `TLnc` à la
/// limite ; la fonction renvoie alors `T`, la température n'ayant pas d'emploi.
pub fn t_apres_conduit_c(t_entree_c: f64, t_lnc_c: f64, h_cond_ext_w_k: f64, qm_kg_s: f64) -> f64 {
    let qm = qm_kg_s.abs();
    if qm < QM_NUL_KG_S {
        return t_entree_c;
    }
    t_entree_c - (t_entree_c - t_lnc_c) * (1.0 - libm::exp(-h_cond_ext_w_k / (C_PA_J_KG_K * qm)))
}

/// Température de l'air après un ventilateur, en °C.
///
/// Th-BCE 2020 éq. (653) et (663) (et 474, 519, 627) : `T + pel × Pvent/(Cpa × |qm|)`, `qm` en
/// kg/s (`ρ × q/3600` dans les éq. 653 et 663). Sans débit (moins de 10⁻⁶ kg/s, §6.3.3.4.8),
/// la température est inchangée.
pub fn t_apres_ventilateur_c(t_entree_c: f64, p_vent_w: f64, qm_kg_s: f64) -> f64 {
    let qm = qm_kg_s.abs();
    if qm < QM_NUL_KG_S {
        return t_entree_c;
    }
    t_entree_c + P_EL * p_vent_w / (C_PA_J_KG_K * qm)
}

/// Débit massique d'un débit volumique en m³/h, en kg/s.
pub fn qm_kg_s(rho_kg_m3: f64, q_m3_h: f64) -> f64 {
    rho_kg_m3 * q_m3_h / 3600.0
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn puissance_moyenne_et_repartition() {
        // Éq. (641) : 14 h de pointe à 30 W, 154 h de base à 15 W.
        let p = p_vent_residentiel_w(15.0, 30.0, 14.0);
        assert!((p - (30.0 * 14.0 + 15.0 * 154.0) / 168.0).abs() < 1e-12);
        assert!((p - 16.25).abs() < 1e-12);
        // Éq. (642) : deux groupes, 60 et 30 m³/h.
        assert!((p_vent_groupe_w(p, 60.0, 90.0) - 16.25 * 2.0 / 3.0).abs() < 1e-12);
        assert_eq!(p_vent_groupe_w(p, 0.0, 0.0), 0.0);
        assert_eq!(dugd_equ_h([14.0, 7.0, 28.0]), 28.0);
    }

    #[test]
    fn masse_volumique_de_reference() {
        assert!((rho_kg_m3(20.0) - 1.2).abs() < 1e-12);
        assert!((rho_kg_m3(0.0) - 1.2 * 293.0 / 273.0).abs() < 1e-12);
    }

    #[test]
    fn conduit_et_ventilateur_a_la_main() {
        // Conduit de 10 m² hors volume chauffé, R = 0,48 : H = 10/0,5 = 20 W/K.
        let h = h_cond_ext_w_k(10.0, 0.48);
        assert!((h - 20.0).abs() < 1e-12);
        // 100 m³/h à 1,2 kg/m³ : 1/30 kg/s ; NUT = 20/(1006/30) = 0,5964.
        let qm = qm_kg_s(1.2, 100.0);
        let t = t_apres_conduit_c(20.0, 0.0, h, qm);
        let attendu = 20.0 * libm::exp(-20.0 / (1006.0 / 30.0));
        assert!((t - attendu).abs() < 1e-12);
        assert!(t > 0.0 && t < 20.0);
        // Sans pertes (H = 0) : inchangée.
        assert_eq!(t_apres_conduit_c(20.0, 0.0, 0.0, qm), 20.0);
        // Ventilateur de 20 W sur 1/30 kg/s : +0,8 × 20 × 30/1006 = 0,4771 K.
        let t = t_apres_ventilateur_c(10.0, 20.0, qm);
        assert!((t - (10.0 + 0.8 * 20.0 * 30.0 / 1006.0)).abs() < 1e-12);
        assert_eq!(t_apres_ventilateur_c(10.0, 20.0, 0.0), 10.0);
    }

    #[test]
    fn local_non_chauffe_et_melange() {
        assert_eq!(t_lnc_c(1.0, 20.0, 0.0), 0.0);
        assert_eq!(t_lnc_c(0.0, 20.0, 0.0), 20.0);
        assert!((t_lnc_c(0.25, 20.0, 0.0) - 15.0).abs() < 1e-12);
        // 100 m³/h repris au groupe, 10 m³/h infiltrés hors volume chauffé à 5 °C.
        let t = theta_i_cond_c(-100.0, -110.0, 20.0, 5.0);
        assert!((t - (100.0 * 20.0 + 10.0 * 5.0) / 110.0).abs() < 1e-12);
    }
}
