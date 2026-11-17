// SPDX-License-Identifier: GPL-3.0-or-later
//! Fiche `C_Sn_Methode_Th_C_Forfait de consommations de refroidissement` (Th-BCE 2020 §13.3).
//!
//! Un groupe non climatisé dont les degrés-heures d'inconfort dépassent le seuil bas reçoit un
//! forfait annuel de consommation d'électricité de refroidissement (éq. 2493), ajouté aux
//! sorties annuelles du mode Th-C du groupe, de la zone et du bâtiment (éq. 2494 à 2496).

use thbce_data::energies::COEF_EP_ELECTRICITE;
use thbce_data::ZoneClimatique;

use crate::confort::confort_ete::SEUIL_BAS_DH_C_H;

/// « Coef_kWh_fr_par_DH », en kWhep/(m².an.°C.h), selon le numéro d'usage de la zone (§13.3.3,
/// tableau 335) : 0,011 pour les usages 1 et 2, 0,016 pour l'enseignement primaire (4), 0,009
/// pour tous les autres.
pub fn coef_kwh_fr_par_dh(usage: u8) -> Option<f64> {
    match usage {
        1 | 2 => Some(0.011),
        4 => Some(0.016),
        3 | 5..=28 => Some(0.009),
        _ => None,
    }
}

/// « Coef_zone_clim_et_alt » (§13.3.3, tableau 336), selon la zone climatique et la classe
/// d'altitude du projet : [0 ; 400 m], ]400 ; 800 m], au-delà de 800 m.
pub fn coef_zone_clim_et_alt(zone: ZoneClimatique, alt_m: f64) -> f64 {
    use ZoneClimatique as Z;
    let ligne = match zone {
        Z::H1a => [0.8, 0.6, 0.4],
        Z::H1b | Z::H1c | Z::H2b => [1.0, 0.8, 0.6],
        Z::H2a => [0.7, 0.5, 0.3],
        Z::H2c => [1.1, 0.9, 0.7],
        Z::H2d | Z::H3 => [1.2, 1.0, 0.8],
    };
    ligne[classe_altitude(alt_m)]
}

/// Classe d'altitude, de 0 à 2 : jusqu'à 400 m, de 400 à 800 m, au-delà de 800 m (mêmes bornes
/// que la correction d'altitude du climat, §3.1.3).
pub fn classe_altitude(alt_m: f64) -> usize {
    if alt_m <= 400.0 {
        0
    } else if alt_m <= 800.0 {
        1
    } else {
        2
    }
}

/// « Cef_fr_elec_refroidissement,g » : forfait de consommation d'électricité de
/// refroidissement du groupe, en kWhef/(m².an) (éq. 2493).
///
/// `seuil_haut_c_h` : `DH_max` du groupe (annexe de l'article R. 172-4, chapitre III, §IV) ;
/// `None` s'il n'est pas connu (catégorie de contraintes extérieures non saisie), auquel cas le
/// DH n'est pas plafonné — ce qui ne change rien tant que le DH respecte l'exigence
/// (`docs/questions.md`, Q54). Nul pour un groupe climatisé (§13.3.3).
pub fn cef_fr_elec_kwh_m2(
    climatise: bool,
    dh_c_h: f64,
    seuil_haut_c_h: Option<f64>,
    coef_kwh_fr_par_dh: f64,
    coef_zone_clim_et_alt: f64,
) -> f64 {
    if climatise {
        return 0.0;
    }
    let dh = seuil_haut_c_h.map_or(dh_c_h, |s| dh_c_h.min(s));
    coef_kwh_fr_par_dh * (dh - SEUIL_BAS_DH_C_H).max(0.0) * coef_zone_clim_et_alt
        / COEF_EP_ELECTRICITE
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn forfait_de_l_eq_2493() {
        // Maison non climatisée en H2b à 100 m, DH = 1 000 °C.h, seuil haut 1 250 °C.h :
        // 0,011 × (1 000 − 350) × 1 / 2,3 = 3,1087 kWhef/m².
        let c = coef_zone_clim_et_alt(ZoneClimatique::H2b, 100.0);
        assert_eq!(c, 1.0);
        let f = cef_fr_elec_kwh_m2(false, 1000.0, Some(1250.0), 0.011, c);
        assert!((f - 0.011 * 650.0 / 2.3).abs() < 1e-12);
        // Plafonné au seuil haut, nul sous le seuil bas ou si le groupe est climatisé.
        let haut = cef_fr_elec_kwh_m2(false, 3000.0, Some(1250.0), 0.011, c);
        assert!((haut - 0.011 * 900.0 / 2.3).abs() < 1e-12);
        assert_eq!(cef_fr_elec_kwh_m2(false, 300.0, Some(1250.0), 0.011, c), 0.0);
        assert_eq!(cef_fr_elec_kwh_m2(true, 1000.0, Some(1250.0), 0.011, c), 0.0);
        // Altitude et zone (tableau 336).
        assert_eq!(coef_zone_clim_et_alt(ZoneClimatique::H3, 500.0), 1.0);
        assert_eq!(coef_zone_clim_et_alt(ZoneClimatique::H1a, 900.0), 0.4);
        assert_eq!(coef_kwh_fr_par_dh(1), Some(0.011));
        assert_eq!(coef_kwh_fr_par_dh(4), Some(0.016));
        assert_eq!(coef_kwh_fr_par_dh(17), Some(0.009));
    }
}
