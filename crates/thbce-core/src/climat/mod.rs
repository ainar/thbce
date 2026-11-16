// SPDX-License-Identifier: GPL-3.0-or-later
//! Météo et environnement extérieur (Th-BCE 2020 §3).
//!
//! - [`donnees`] : données météo horaires fournies par l'appelant ;
//! - [`calendrier`] : repères horaires (heure du jour, heure légale) ;
//! - [`climat_exterieur`] : fiche `C_EEX_Climat extérieur` (§3.1) ;
//! - [`confort_adaptatif`] : période de confort adaptatif (§3.1.3) ;
//! - [`environnement_proche`] : fiche `C_EEX_environnement_proche` (§3.2).

pub mod calendrier;
pub mod climat_exterieur;
pub mod confort_adaptatif;
pub mod donnees;
pub mod environnement_proche;

pub use climat_exterieur::ClimatExterieur;
pub use confort_adaptatif::ConfortAdaptatif;
pub use donnees::{AnneeMeteo, JeuMeteo, MeteoProjet, OrigineMeteo};
