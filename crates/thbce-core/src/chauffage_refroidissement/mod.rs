// SPDX-License-Identifier: GPL-3.0-or-later
//! Chauffage et refroidissement (Th-BCE 2020 §8).
//!
//! - [`emission`] : fiche `C-Emi-Systèmes Emissions du groupe en chaud et en froid` (§8.1) ;
//!   seule l'émission équivalente conventionnelle du calcul du Bbio est implémentée ;
//! - [`bilan_hydrique`] : fiche `C_Emi_Bilan hydrique` (§8.3) ; seul le bilan sans
//!   déshumidification du calcul du Bbio est implémenté.
//!
//! Les émetteurs du projet, les ventilateurs locaux, la déshumidification, la distribution et
//! la génération viendront au lot 6.

pub mod bilan_hydrique;
pub mod emission;
