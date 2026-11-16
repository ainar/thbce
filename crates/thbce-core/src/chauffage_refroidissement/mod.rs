// SPDX-License-Identifier: GPL-3.0-or-later
//! Chauffage et refroidissement (Th-BCE 2020 §8).
//!
//! - [`emission`] : fiche `C-Emi-Systèmes Emissions du groupe en chaud et en froid` (§8.1) ;
//!   seule l'émission équivalente conventionnelle du calcul du Bbio est implémentée ;
//! - [`bilan_hydrique`] : fiche `C_Emi_Bilan hydrique` (§8.3) ; seul le bilan sans
//!   déshumidification du calcul du Bbio est implémenté.
//!
//! - [`emetteurs`] : émetteurs du projet, émetteurs équivalents et ventilateurs locaux (§8.1,
//!   calculs Th-C et Th-D) ;
//! - [`saisons_fonctionnement`] : fiche `C_Ein_Détermination des saisons de fonctionnement des
//!   systèmes de chaud et de froid` (§8.4) ;
//! - [`relances`] : fiche `C_Ein_Programmation des relances des systèmes` (§8.5) ;
//! - [`distribution`] : distributions du groupe et intergroupes (§8.6 à 8.10).
//!
//! La génération relève des lots 6c et 6d : chaque génération est traitée comme idéale.

pub mod bilan_hydrique;
pub mod distribution;
pub mod emetteurs;
pub mod emission;
pub mod relances;
pub mod saisons_fonctionnement;
pub mod systemes_groupe;
