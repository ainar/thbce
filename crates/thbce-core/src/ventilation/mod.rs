// SPDX-License-Identifier: GPL-3.0-or-later
//! Ventilation (Th-BCE 2020 §6).
//!
//! - [`bouche_conduit`] : fiche `C_VEN_Bouche_conduit` (§6.2), débits spécifiques du groupe,
//!   dépassement et fuites des réseaux.
//! - [`debits_air_cep`] : fiche `C_VEN_Débits_d'air_Cep` (§5.6), bilan aéraulique de la zone
//!   (perméabilité de l'enveloppe, entrées d'air, échanges entre groupes).
//! - [`ven_bbio`] : fiche `C_VEN_BBIO` (§6.1), système de ventilation conventionnel du calcul
//!   du Bbio.
//! - [`mecanique_sf`] : fiche `C_VEN_Mécanique_SF` (§6.5), simple flux par extraction ou par
//!   insufflation.
//! - [`mecanique_basse_pression`] : fiche `C_VEN_Mécanique_Basse-Pression` (§6.4).
//! - [`cta_et_double_flux`] : fiche `C_VEN_CTA et Double_flux` (§6.3), double flux en
//!   habitation.
//! - [`humidification`] : fiche 6.3, humidificateur d'hiver et rafraîchissement évaporatif.
//! - [`puits_climatique`] : fiches 6.8, 6.10 et 6.11, température du sol, efficacité du
//!   puits, bypass.
//! - [`puits_hydraulique`] : fiche 6.9 et prétraitement du §6.3.3.3.4, batterie eau/air,
//!   capteur enterré à eau glycolée, pompe.
//! - [`surventilation_mecanique`] : fiche `C_VEN_Fonctions de rafraichissement par
//!   surventilation mécanique` (§6.12), régulation et surfaces de conduit.
//! - [`ventilation_projet`] : système de ventilation du projet d'une zone, pour les calculs
//!   Th-C et Th-D.

pub mod bouche_conduit;
pub mod cta_et_double_flux;
pub mod debits_air_cep;
pub mod humidification;
pub mod mecanique_basse_pression;
pub mod mecanique_sf;
pub mod puits_climatique;
pub mod puits_hydraulique;
pub mod surventilation_mecanique;
pub mod ven_bbio;
pub mod ventilation_projet;
