// SPDX-License-Identifier: GPL-3.0-or-later
//! Système de ventilation conventionnel du calcul du Bbio : fiche `C_VEN_BBIO`
//! (Th-BCE 2020 §6.1).
//!
//! « Pour le calcul du Bbio, le système de ventilation est une VMC à débit soufflé et extrait
//! constant avec efficacité d'échangeur de 50 % » (§6.1.3). Les ventilateurs sont de puissance
//! nulle (éq. 374 et 375) et les pertes des conduits sont nulles (§6.1.3.2).
//!
//! Les débits soufflés et repris du groupe sont des entrées de la fiche ; ils viennent des
//! fiches de débits d'air, qui restent à lire.

/// « ε » : efficacité de l'échangeur pour le calcul du Bbio (§6.1.2, valeur conventionnelle).
pub const EPSILON_BBIO: f64 = 0.5;
/// « Cpa » : chaleur massique de l'air, en J/(kg.K) (§6.1.2, constantes).
pub const C_PA_J_KG_K: f64 = 1006.0;

/// « TAir_soufflé » : température de l'air soufflé dans le groupe, en °C.
///
/// Th-BCE 2020 §6.1.3 : l'air repris est à la température du groupe à la fin du pas de temps
/// précédent ; les pertes des conduits et la puissance des ventilateurs étant nulles,
/// `Textr2 = Textr1 = θi,fin(h−1)` (éq. 377). Éq. (378) à (380) :
/// `TAir_soufflé = θext + ε·(Textr2 − θext)`.
pub fn t_air_souffle_c(theta_ext_c: f64, theta_i_fin_precedent_c: f64) -> f64 {
    theta_ext_c + EPSILON_BBIO * (theta_i_fin_precedent_c - theta_ext_c)
}

/// « ωair_soufflé » : humidité de l'air soufflé, en kg/kg.
///
/// Th-BCE 2020 §6.1.3.3, éq. (381) : celle de l'air extérieur.
pub fn omega_air_souffle_kg_kg(omega_ext_kg_kg: f64) -> f64 {
    omega_ext_kg_kg
}

/// « Hvent » : déperditions par la ventilation, en W/K.
///
/// Th-BCE 2020 §6.1.3.4, éq. (382) : `Hvent = qsoufflé,cond × Cpa × (1 − ε)`. La nomenclature
/// donne `qsoufflé,cond` en m³/h, ce qui ne donne pas des W/K ; le débit est pris ici en kg/s
/// (question Q34). Cette sortie n'entre pas dans le bilan du groupe, qui reçoit le débit et la
/// température de l'air soufflé.
pub fn h_vent_w_k(qm_souffle_kg_s: f64) -> f64 {
    qm_souffle_kg_s * C_PA_J_KG_K * (1.0 - EPSILON_BBIO)
}
