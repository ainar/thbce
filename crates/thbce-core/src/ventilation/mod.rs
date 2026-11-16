// SPDX-License-Identifier: GPL-3.0-or-later
//! Ventilation (Th-BCE 2020 §6).
//!
//! - [`bouche_conduit`] : fiche `C_VEN_Bouche_conduit` (§6.2), débits spécifiques du groupe ;
//!   seule la partie utile au calcul du Bbio en habitation est implémentée.
//! - [`ven_bbio`] : fiche `C_VEN_BBIO` (§6.1), système de ventilation conventionnel du calcul
//!   du Bbio.

pub mod bouche_conduit;
pub mod ven_bbio;
