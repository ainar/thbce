// SPDX-License-Identifier: GPL-3.0-or-later
//! Données conventionnelles de la méthode Th-BCE 2020.
//!
//! Cette crate ne contient que des tables issues des textes réglementaires versionnés dans
//! `docs/reference/`, chacune avec sa source. Elle ne fait ni entrée-sortie ni calcul.
//!
//! Les fichiers météorologiques réglementaires n'en font pas partie : leurs conditions
//! d'utilisation interdisent de les diffuser. Ils sont fournis par l'appelant au moment du calcul.

pub mod departement;
pub mod ponderation_dynamique;
pub mod zones_climatiques;

pub use departement::{Departement, ErreurDepartement};
pub use zones_climatiques::ZoneClimatique;
