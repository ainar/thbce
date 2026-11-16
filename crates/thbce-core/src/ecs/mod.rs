// SPDX-License-Identifier: GPL-3.0-or-later
//! Eau chaude sanitaire (Th-BCE 2020 §9).
//!
//! - [`besoins_ecs`] : fiches `C_EMI_Emission_ECS` (§9.5) et `C_EIN_besoins_ECS` (§9.6), sans
//!   récupération de chaleur sur les eaux grises (fiche 9.21, après la première version) ;
//! - [`distribution_ecs`] : fiches `C_DIS_Distribution_ECS_du_groupe` (§9.7) et
//!   `C_DIS_Distribution_ECS_intergroupe` (§9.8).
//!
//! La production instantanée d'ECS (§9.1) passe par les générateurs de la génération
//! (`chauffage_refroidissement::generation`) ; le stockage (§9.2, 9.9 et suivantes) relève de la
//! suite du lot 6c et des lots 6d et 7b.
//!
//! Lectures : `docs/questions.md`, Q45.

pub mod besoins_ecs;
pub mod distribution_ecs;
