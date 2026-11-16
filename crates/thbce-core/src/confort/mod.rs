// SPDX-License-Identifier: GPL-3.0-or-later
//! Confort thermique des occupants.
//!
//! - [`indicateurs_confort`] : fiche `C_Ein_Indicateurs de confort` (Th-BCE 2020 §4.5) ;
//! - [`confort_ete`] : fiche `C_BAT_Confort_d'été` (§13.5), indicateur DH du calcul Th-D.

pub mod confort_ete;
pub mod indicateurs_confort;
