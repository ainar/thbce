// SPDX-License-Identifier: GPL-3.0-or-later
//! Calcul des indicateurs (Th-BCE 2020 §13).
//!
//! - [`sorties_th_b`] : fiche `C_CALC_Sorties du mode Th-B` (§13.1), besoins et Bbio ;
//! - [`sorties_th_c`] : fiche `C_CALC_Sorties du mode Th-C` (§13.2), Cep et Cep,nr, avec l'Ic
//!   énergie (annexe II §4.3.2) et les exigences ([`exigences`]) ;
//! - [`forfait_refroidissement`] : fiche du forfait de consommations de refroidissement
//!   (§13.3) ;
//! - [`bilans_energies`] : fiche `C_Calc_Bilans_Energies_Consommees_et_importées` (§13.4),
//!   autoconsommation de la production photovoltaïque ;
//! - [`equipements_mobiliers`] : consommations des équipements mobiliers (§4.7).

pub mod bilans_energies;
pub mod equipements_mobiliers;
pub mod exigences;
pub mod forfait_refroidissement;
pub mod resultats;
pub mod sorties_th_b;
pub mod sorties_th_c;
