// SPDX-License-Identifier: GPL-3.0-or-later
//! Eau chaude sanitaire (Th-BCE 2020 §9).
//!
//! - [`besoins_ecs`] : fiches `C_EMI_Emission_ECS` (§9.5) et `C_EIN_besoins_ECS` (§9.6), sans
//!   récupération de chaleur sur les eaux grises (fiche 9.21, après la première version) ;
//! - [`distribution_ecs`] : fiches `C_DIS_Distribution_ECS_du_groupe` (§9.7) et
//!   `C_DIS_Distribution_ECS_intergroupe` (§9.8) ;
//! - [`ballon_de_stockage`] : fiche `C_STO_Ballon_de_stockage` (§9.9) ;
//! - [`gestion_regulation_ballon`] : fiche `C_STO_gestion_régulation_ballon` (§9.10) ;
//! - [`assemblage_ballon`] : fiches `S1_GEN_générateur_pour_ballon` (§9.13) et assemblage d'un
//!   ballon et de ses générateurs (§9.14) ;
//! - [`idcet`] : saisie des chauffe-eau thermodynamiques à compression électrique (§9.22).
//!
//! La production d'ECS, instantanée (§9.1) ou avec stockage (§9.2), passe par la génération
//! (`chauffage_refroidissement::generation`). Le ballon solaire relève du lot 7b.
//!
//! Lectures : `docs/questions.md`, Q45 et Q48.

pub mod assemblage_ballon;
pub mod ballon_de_stockage;
pub mod besoins_ecs;
pub mod distribution_ecs;
pub mod gestion_regulation_ballon;
pub mod idcet;
