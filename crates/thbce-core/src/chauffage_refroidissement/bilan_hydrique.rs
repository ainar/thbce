// SPDX-License-Identifier: GPL-3.0-or-later
//! Bilan hydrique d'un groupe : fiche `C_Emi_Bilan hydrique` (Th-BCE 2020 §8.3).
//!
//! L'humidité spécifique de l'air du groupe évolue sous l'effet de l'air entrant et des apports
//! internes d'humidité ; l'inertie hygroscopique des locaux est négligée et les débits d'air sec
//! entrants sont supposés égaux aux débits sortants (§8.3.3.1). L'humidité à la fin d'un pas de
//! temps alimente, au pas suivant, la masse volumique de l'air du groupe dans le bilan
//! aéraulique de la zone (§5.6) et le bilan hydrique lui-même.
//!
//! Périmètre : calcul du Bbio (§8.3.3.3, éq. 834), sans déshumidification. La
//! déshumidification sur les batteries froides (éq. 835 à 838) relève du calcul des
//! consommations (lot 6b).

use libm::exp;

/// Durée du pas de temps, en s (§8.3.3.3 : « T = 3600 s »).
pub const T_PAS_S: f64 = 3600.0;

/// Air entrant dans un local et apports d'humidité, pour le bilan hydrique.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EchangesHydriques {
    /// `Σ Qmaj(h)` : somme des débits d'air sec entrant dans le groupe, en kg/s.
    pub somme_q_maj_kg_s: f64,
    /// `Σ Qmaj(h) × ωmaj(h)` : débit d'eau apporté par l'air entrant, en kg/s.
    pub somme_q_omega_maj_kg_s: f64,
    /// « Aωint(h) » : apports internes d'humidité totaux du groupe, en kg/s (éq. 830 :
    /// occupants et hors occupants).
    pub a_omega_int_kg_s: f64,
    /// « V » : volume du groupe, en m³.
    pub v_m3: f64,
    /// « ρi,g(h−1) » : masse volumique de l'air du groupe à la fin du pas de temps précédent,
    /// en kg/m³.
    pub rho_i_kg_m3: f64,
}

/// « CalculHumiditeSpeFin » : humidité spécifique atteinte au bout d'un temps `t_s` à partir
/// d'une humidité initiale `omega_ini`, en kg/kg d'air sec.
///
/// Th-BCE 2020 §8.3.3.2, éq. (831) :
/// - `A = [Ratem_eff·(Σ(Qmaj·ωmaj) + Aωint) + Qm_recirc·ωsat]/[Ratem_eff·ΣQmaj + Qm_recirc] − ωini` ;
/// - `B = [Ratem_eff·ΣQmaj + Qm_recirc]/[Ratem_eff·V·ρi,g(h−1)]` ;
/// - `ωfin = ωini + A·(1 − e^(−B·T))`.
///
/// Sans débit d'air entrant ni recirculation, `A` n'est pas défini ; la limite de la formule
/// est employée : `ωfin = ωini + Aωint·T/(V·ρ)`, solution de l'éq. (829) sans renouvellement
/// d'air.
pub fn calcul_humidite_spe_fin(
    rat_em_eff: f64,
    q_m_recirc_kg_s: f64,
    omega_sat_kg_kg: f64,
    t_s: f64,
    omega_ini_kg_kg: f64,
    e: &EchangesHydriques,
) -> f64 {
    let debit = rat_em_eff * e.somme_q_maj_kg_s + q_m_recirc_kg_s;
    let masse_air = rat_em_eff * e.v_m3 * e.rho_i_kg_m3;
    let apports = rat_em_eff * (e.somme_q_omega_maj_kg_s + e.a_omega_int_kg_s)
        + q_m_recirc_kg_s * omega_sat_kg_kg;
    if debit <= 0.0 {
        return omega_ini_kg_kg + apports * t_s / masse_air;
    }
    let a = apports / debit - omega_ini_kg_kg;
    let b = debit / masse_air;
    omega_ini_kg_kg + a * (1.0 - exp(-b * t_s))
}

/// « ωi,g,fin(h) » : humidité spécifique du groupe à la fin du pas de temps, en calcul du Bbio.
///
/// Th-BCE 2020 §8.3.3.3, éq. (834) :
/// `ωi,g,fin(h) = CalculHumiditeSpeFin(1 ; 0 kg/s ; 0 kg/kg ; 3600 s ; ωi,g,fin(h−1))` : « la
/// déshumidification sur les batteries froides n'intervient pas ».
pub fn omega_fin_bbio_kg_kg(omega_fin_precedent_kg_kg: f64, e: &EchangesHydriques) -> f64 {
    calcul_humidite_spe_fin(1.0, 0.0, 0.0, T_PAS_S, omega_fin_precedent_kg_kg, e)
}
