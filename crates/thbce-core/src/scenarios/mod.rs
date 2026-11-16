// SPDX-License-Identifier: GPL-3.0-or-later
//! Scénarios et environnement intérieur (Th-BCE 2020 §4).
//!
//! - [`scenarios_conventionnels`] : fiche `C_EIN_Scénarios conventionnels` (§4.1).

pub mod scenarios_conventionnels;

pub use scenarios_conventionnels::{nadeq, ScenariosZone, SortiesZone};
