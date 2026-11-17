// SPDX-License-Identifier: GPL-3.0-or-later
//! Système de ventilation conventionnel du calcul du Bbio : fiche `C_VEN_BBIO`
//! (Th-BCE 2020 §6.1).
//!
//! « Pour le calcul du Bbio, le système de ventilation est une VMC à débit soufflé et extrait
//! constant avec efficacité d'échangeur de 50 % » (§6.1.3). Les ventilateurs sont de puissance
//! nulle (éq. 374 et 375) et les pertes des conduits sont nulles (§6.1.3.2).
//!
//! Les débits soufflés et repris du groupe sont des entrées de la fiche ; ils viennent de la
//! fiche `C_VEN_Bouche_conduit` (§6.2, [`super::bouche_conduit`]).

use super::bouche_conduit::Debits;

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

/// Lecture des débits du système de ventilation conventionnel du Bbio (`docs/questions/Q036.md`,
/// point 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LectureDebitsBbio {
    /// Le système conventionnel souffle et extrait : quand les bouches du groupe ne décrivent
    /// qu'un sens (ventilation installée à simple flux), l'autre sens reçoit le même débit.
    #[default]
    DoubleFluxConventionnelle,
    /// Les débits sont ceux des bouches, tels quels : un groupe sans bouche de soufflage n'a pas
    /// de débit soufflé, et l'échangeur conventionnel est sans effet.
    Litterale,
}

/// « qspec_repris » et « qspec_soufflé » du système de ventilation conventionnel du Bbio, en
/// m³/h, à partir des débits des bouches du groupe (fiche `C_VEN_Bouche_conduit`).
///
/// Th-BCE 2020 §6.1.3 : « Pour le calcul du BBIO, le système de ventilation est une VMC à débit
/// soufflé et extrait constant avec efficacité d'échangeur de 50 % » ; annexe II, tableau des
/// modes de calcul : « débits hygiéniques réglementaires et système de ventilation prédéfini ».
/// Les éq. (387) à (390) ne donnent pourtant qu'un débit repris ou qu'un débit soufflé par
/// bouche, selon `Isouf`. Lecture retenue : le système prédéfini souffle et extrait le débit
/// d'hygiène du groupe ; si un seul sens est décrit, l'autre lui est égal en valeur absolue ; si
/// les deux le sont, ils sont pris tels quels.
pub fn debits_systeme_conventionnel(bouches: Debits, lecture: LectureDebitsBbio) -> Debits {
    match lecture {
        LectureDebitsBbio::Litterale => bouches,
        LectureDebitsBbio::DoubleFluxConventionnelle => {
            if bouches.q_souffle_m3_h == 0.0 {
                Debits {
                    q_souffle_m3_h: -bouches.q_rep_m3_h,
                    ..bouches
                }
            } else if bouches.q_rep_m3_h == 0.0 {
                Debits {
                    q_rep_m3_h: -bouches.q_souffle_m3_h,
                    ..bouches
                }
            } else {
                bouches
            }
        }
    }
}
