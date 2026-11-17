// SPDX-License-Identifier: GPL-3.0-or-later
//! Production locale d'électricité : photovoltaïque (Th-BCE 2020 §12, lot 7a).

pub mod capteur;
pub mod entree;
pub mod global;
pub mod installation;
pub mod onduleur;
pub mod resultats;

pub use entree::InstallationPv;
pub use resultats::{ResultatsInstallationPv, ResultatsProductionPv};
