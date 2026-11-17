// SPDX-License-Identifier: GPL-3.0-or-later
//! Fiche `C_EIN_calcul des consommations électriques des équipements mobiliers` (Th-BCE 2020
//! §4.7).
//!
//! Par convention, la consommation électrique des équipements mobiliers est égale aux apports
//! internes de chaleur non dus aux occupants (§4.7.1). Elle n'entre pas dans le périmètre de
//! la RE2020 mais dans la consommation « tous usages » qui fixe la part autoconsommée de la
//! production locale d'électricité (§13.4, éq. 2515 et 2527).

use crate::conventions::NB_HEURES_AN;
use crate::entree::Zone;
use crate::erreur::Erreur;
use crate::scenarios::scenarios_conventionnels::ScenariosZone;

/// « Cef_us_mob^z(h) » : consommation horaire des usages mobiliers d'une zone, en Wh, pour les
/// 8 760 pas de temps (éq. 93 et 94) : `Σloc Aloc·Qmax_proc·tch(m,s)·tch(j,h)`, soit les apports
/// internes hors occupants et éclairage des scénarios (§4.1.3, éq. 41).
///
/// # Errors
///
/// Usage de la zone sans scénarios pris en charge.
pub fn consommation_horaire_wh(zone: &Zone) -> Result<Vec<f64>, Erreur> {
    let a_gr: Vec<f64> = zone.groupes.iter().map(|g| g.su_m2.unwrap_or(0.0)).collect();
    let sc = ScenariosZone::nouveau(zone.usage, &a_gr, zone.nb_lgt.unwrap_or(1))?;
    Ok((0..NB_HEURES_AN).map(|h| sc.evaluer(h).q_int_w).collect())
}
