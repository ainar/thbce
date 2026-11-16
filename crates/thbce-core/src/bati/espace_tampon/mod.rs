// SPDX-License-Identifier: GPL-3.0-or-later
//! Espaces tampons (Th-BCE 2020 §5.2 à 5.4) :
//!
//! - [`assemblage`] : fiche `S1-Bat-Assemblage des espaces tampons` (§5.2) ;
//! - [`gestion_solarise`] : fiche `Gestion/Régulation des espaces tampons solarisés` (§5.3) ;
//! - [`espace_tampon`] : fiche `Espaces tampons` (§5.4).
//!
//! Les coefficients `btherm`, `bsolaire` et `becl` qu'ils produisent sont appliqués aux
//! parois opaques (§5.17.3), ponts thermiques (§5.19.3) et baies (§5.10.3) qui séparent le
//! volume chauffé de l'espace tampon, par l'assemblage du groupe ; l'air neuf qui transite par
//! un espace tampon entre à sa température (§5.6.3.1, éq. 134). Lectures du texte :
//! `docs/questions.md`, Q41.

pub mod assemblage;
#[allow(clippy::module_inception)]
pub mod espace_tampon;
pub mod gestion_solarise;
