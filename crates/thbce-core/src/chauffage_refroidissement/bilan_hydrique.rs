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

/// « Lv_eau » : chaleur latente de vaporisation de l'eau à pression atmosphérique, en kJ/kg
/// (tableau 89).
pub const LV_EAU_KJ_KG: f64 = 2500.0;

/// « Ca » : chaleur massique de l'air sec, en J/(kg.K) (tableau 81).
pub const CA_J_KG_K: f64 = 1006.0;

/// « FBbatt » : facteur de by-pass conventionnel d'une batterie à débit d'eau régulé de façon
/// progressive (§8.1.3.8.5).
pub const FB_BATT: f64 = 0.8;

/// « θbatt_dim » d'un climatiseur à détente directe (réseau fictif), en °C (éq. 817).
pub const THETA_BATT_DIM_DETENTE_DIRECTE_C: f64 = 9.0;

/// `CalculHumiditeSpeMoy` (éq. 832) : humidité spécifique moyenne sur une durée `T`.
pub fn calcul_humidite_spe_moy(
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
    if debit <= 0.0 || t_s <= 0.0 {
        // Limite sans débit : évolution linéaire, moyenne à mi-parcours.
        return omega_ini_kg_kg + apports * t_s / (2.0 * masse_air);
    }
    let a = apports / debit - omega_ini_kg_kg;
    let bt = debit / masse_air * t_s;
    omega_ini_kg_kg + a * (1.0 - (1.0 - exp(-bt)) / bt)
}

/// `CalculTemps` (éq. 833) : durée nécessaire pour passer de `ω_ini` à `ω_fin`, en s ; `None`
/// si `ω_fin` n'est jamais atteinte.
pub fn calcul_temps(
    rat_em_eff: f64,
    q_m_recirc_kg_s: f64,
    omega_sat_kg_kg: f64,
    omega_ini_kg_kg: f64,
    omega_fin_kg_kg: f64,
    e: &EchangesHydriques,
) -> Option<f64> {
    let debit = rat_em_eff * e.somme_q_maj_kg_s + q_m_recirc_kg_s;
    let masse_air = rat_em_eff * e.v_m3 * e.rho_i_kg_m3;
    let apports = rat_em_eff * (e.somme_q_omega_maj_kg_s + e.a_omega_int_kg_s)
        + q_m_recirc_kg_s * omega_sat_kg_kg;
    if debit <= 0.0 {
        let pente = apports / masse_air;
        let t = (omega_fin_kg_kg - omega_ini_kg_kg) / pente;
        return (pente != 0.0 && t >= 0.0).then_some(t);
    }
    let a = apports / debit - omega_ini_kg_kg;
    let b = debit / masse_air;
    let c = 1.0 - (omega_fin_kg_kg - omega_ini_kg_kg) / a;
    (a != 0.0 && c > 0.0)
        .then(|| -libm::log(c) / b)
        .filter(|t| *t >= 0.0)
}

/// « ωsat » : humidité spécifique à saturation à la température de batterie, en kg/kg
/// (éq. 823, `HRsat = 100`).
pub fn omega_sat_kg_kg(theta_batt_c: f64) -> f64 {
    1e-3 * exp(18.8161 - 4110.34 / (theta_batt_c + 235.0))
}

/// Batterie froide d'un émetteur de froid à recyclage d'air sollicité au pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BatterieFroide {
    /// « Rateff_fr^em ».
    pub rat_eff: f64,
    /// « Qm_recirc_eff^em(h) », en kg/s (éq. 819 et 822).
    pub q_m_recirc_eff_kg_s: f64,
    /// « ωsat^em(h) », en kg/kg (éq. 823).
    pub omega_sat_kg_kg: f64,
}

impl BatterieFroide {
    /// Débit effectif et humidité à saturation d'une batterie (éq. 819 à 823).
    ///
    /// `q_m_recirc_kg_s` : débit de recirculation des ventilateurs (éq. 814) ;
    /// `q_sys_fr_wh` : énergie de froid requise par l'émetteur (≤ 0, éq. 810) ;
    /// `regulation_progressive` : `idregul_batt = 0`.
    pub fn nouvelle(
        rat_eff: f64,
        q_m_recirc_kg_s: f64,
        q_sys_fr_wh: f64,
        theta_batt_dim_c: f64,
        theta_i_moy_c: f64,
        regulation_progressive: bool,
    ) -> Self {
        let (q_eff, theta_batt) = if regulation_progressive {
            // Éq. (819) et (820) ; l'énergie du pas d'une heure est lue comme une puissance.
            let q = FB_BATT * q_m_recirc_kg_s;
            let t = if q > 0.0 {
                theta_batt_dim_c.max(theta_i_moy_c + q_sys_fr_wh / (CA_J_KG_K * q))
            } else {
                theta_batt_dim_c
            };
            (q, t)
        } else {
            // Éq. (821) et (822).
            let dt = theta_i_moy_c - theta_batt_dim_c;
            let q = if dt > 0.0 {
                -q_sys_fr_wh / (CA_J_KG_K * dt)
            } else {
                0.0
            };
            (q, theta_batt_dim_c)
        };
        Self {
            rat_eff,
            q_m_recirc_eff_kg_s: q_eff,
            omega_sat_kg_kg: omega_sat_kg_kg(theta_batt),
        }
    }
}

/// Bilan hydrique d'un groupe en calcul des consommations (§8.3.4, éq. 835 à 838).
///
/// `froid` : pour chaque émetteur de froid du groupe, sa contribution `Rateff_fr` et, s'il a
/// une batterie à recyclage d'air sollicitée au pas de temps, sa batterie ; `idb_fr` :
/// « idbfr(h) ». Renvoie l'humidité finale du groupe et l'énergie latente de chaque émetteur
/// de froid, en Wh, négative ou nulle (éq. 836). Sans émetteur de froid, le calcul est celui du
/// Bbio (éq. 834).
pub fn bilan_th_c(
    omega_ini_kg_kg: f64,
    froid: &[(f64, Option<BatterieFroide>)],
    idb_fr: bool,
    e: &EchangesHydriques,
) -> (f64, Vec<f64>) {
    let total: f64 = froid.iter().map(|(r, _)| r).sum();
    if total <= 0.0 {
        return (
            omega_fin_bbio_kg_kg(omega_ini_kg_kg, e),
            vec![0.0; froid.len()],
        );
    }
    let mut omega_fin = 0.0;
    let mut latent = Vec::with_capacity(froid.len());
    for &(rat, batterie) in froid {
        if rat <= 0.0 {
            latent.push(0.0);
            continue;
        }
        let (fin, q_lat) = match (idb_fr, batterie) {
            (true, Some(b)) if b.q_m_recirc_eff_kg_s > 0.0 => {
                local_deshumidifie(omega_ini_kg_kg, &b, e)
            }
            _ => (
                calcul_humidite_spe_fin(rat, 0.0, 0.0, T_PAS_S, omega_ini_kg_kg, e),
                0.0,
            ),
        };
        // Éq. (837).
        omega_fin += rat * fin;
        latent.push(q_lat);
    }
    // Les `Rateff_fr` somment à 1 (éq. 797) ; la division ne change rien dans ce cas.
    (omega_fin / total, latent)
}

/// Éq. (835) et (836) pour un local desservi par une batterie sollicitée.
fn local_deshumidifie(omega_ini: f64, b: &BatterieFroide, e: &EchangesHydriques) -> (f64, f64) {
    let (rat, q, sat) = (b.rat_eff, b.q_m_recirc_eff_kg_s, b.omega_sat_kg_kg);
    let (dt_deshu, omega_fin, omega_moy) = if omega_ini >= sat {
        // La déshumidification a lieu dès le début du pas de temps.
        let perm = calcul_humidite_spe_fin(rat, q, sat, T_PAS_S, omega_ini, e);
        if perm >= sat {
            (
                T_PAS_S,
                perm,
                calcul_humidite_spe_moy(rat, q, sat, T_PAS_S, omega_ini, e),
            )
        } else {
            // Arrêt au bout de dt_deshu ; le texte écrit « dtsec = 1 − dtdeshu » : lu
            // 3 600 − dtdeshu (lecture Q43).
            let dt = calcul_temps(rat, q, sat, omega_ini, sat, e)
                .unwrap_or(T_PAS_S)
                .min(T_PAS_S);
            let moy = calcul_humidite_spe_moy(rat, q, sat, dt, omega_ini, e);
            let fin = calcul_humidite_spe_fin(rat, 0.0, sat, T_PAS_S - dt, sat, e);
            (dt, fin, moy)
        }
    } else {
        let sans = calcul_humidite_spe_fin(rat, 0.0, sat, T_PAS_S, omega_ini, e);
        if sans <= sat {
            (0.0, sans, 0.0)
        } else {
            // La déshumidification commence au bout de dt_sec.
            let dt_sec = calcul_temps(rat, 0.0, sat, omega_ini, sat, e)
                .unwrap_or(T_PAS_S)
                .min(T_PAS_S);
            let dt = T_PAS_S - dt_sec;
            (
                dt,
                calcul_humidite_spe_fin(rat, q, sat, dt, sat, e),
                calcul_humidite_spe_moy(rat, q, sat, dt, sat, e),
            )
        }
    };
    // Éq. (836) : `Qsys_lat = −Lv_eau/3,6 × dt_deshu × Qm_recirc_eff × MAX(0 ; ωmoy − ωsat)`,
    // Lv en kJ/kg, dt en s et Qm en kg/s donnent des kJ ; divisés par 3,6, des Wh.
    let q_lat = -LV_EAU_KJ_KG / 3.6 * dt_deshu * q * (omega_moy - sat).max(0.0);
    (omega_fin, q_lat)
}
