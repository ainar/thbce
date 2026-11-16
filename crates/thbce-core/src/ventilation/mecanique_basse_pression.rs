// SPDX-License-Identifier: GPL-3.0-or-later
//! Extraction mécanique basse pression : fiche `C_VEN_Mécanique_Basse-Pression` (Th-BCE 2020
//! §6.4), conçue pour les logements collectifs à conduits individuels ou de type shunt.
//!
//! La fiche écrit les mêmes relations que l'extraction simple flux de la fiche 6.5 :
//! - puissance du ventilateur (éq. 619 et 620) : [`super::mecanique_sf::dugd_equ_h`] et
//!   [`super::mecanique_sf::p_vent_residentiel_w`] ;
//! - consommation (éq. 621) : la puissance, sommée heure par heure ;
//! - températures après les pertes du conduit et le ventilateur (éq. 622 à 627) :
//!   [`super::mecanique_sf::t_apres_conduit_c`], [`super::mecanique_sf::theta_i_cond_c`],
//!   [`super::mecanique_sf::t_lnc_c`], [`super::mecanique_sf::h_cond_ext_w_k`],
//!   [`super::mecanique_sf::t_apres_ventilateur_c`].
//!
//! Elle ne dit pas comment répartir la consommation « de la zone » (`Cvent`) entre ses groupes :
//! la répartition au prorata des débits repris de la fiche 6.5 (éq. 642) est retenue, et la
//! différence de pression des fuites est celle des systèmes « basse pression » du tableau 60
//! (`docs/questions.md`, Q42).

use super::bouche_conduit::PressionReseau;

/// Pression de fonctionnement retenue pour les fuites des conduits d'un système mécanique
/// basse pression (tableau 60, colonne « dP en basse pression » ; lecture Q42).
pub const PRESSION_RESEAU: PressionReseau = PressionReseau::Basse;
