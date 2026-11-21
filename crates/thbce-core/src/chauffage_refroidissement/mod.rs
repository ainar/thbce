// SPDX-License-Identifier: GPL-3.0-or-later
//! Chauffage et refroidissement (Th-BCE 2020 §8).
//!
//! - [`emission`] : fiche `C-Emi-Systèmes Emissions du groupe en chaud et en froid` (§8.1) ;
//!   seule l'émission équivalente conventionnelle du calcul du Bbio est implémentée ;
//! - [`bilan_hydrique`] : fiche `C_Emi_Bilan hydrique` (§8.3) ; seul le bilan sans
//!   déshumidification du calcul du Bbio est implémenté.
//!
//! - [`emetteurs`] : émetteurs du projet, émetteurs équivalents et ventilateurs locaux (§8.1,
//!   calculs Th-C et Th-D) ;
//! - [`saisons_fonctionnement`] : fiche `C_Ein_Détermination des saisons de fonctionnement des
//!   systèmes de chaud et de froid` (§8.4) ;
//! - [`relances`] : fiche `C_Ein_Programmation des relances des systèmes` (§8.5) ;
//! - [`distribution`] : distributions du groupe et intergroupes (§8.6 à 8.10).
//!
//! - [`generation`] : assemblage et gestion/régulation de la génération (§8.14 et §8.17) ;
//! - [`generateur_direct_a_effet_joule`] : générateur direct à effet Joule (§8.18) ;
//! - [`chaudieres_gaz_fioul_bois`], [`poeles_et_inserts`], [`reseau_de_chaleur_et_de_froid`] :
//!   générateurs à combustion et sous-stations (§8.19, §8.22 et §8.28) ;
//! - [`thermodynamique_elec`] : générateurs thermodynamiques à compression électrique (§8.23) ;
//! - [`sources_amont`] : sources amont sur air des générateurs thermodynamiques (§8.26) ;
//! - [`calculs_generation`] : matrices et cumuls des consommations (§8.29).

pub mod autres_generateurs_a_combustion;
pub mod bilan_hydrique;
pub mod brasseurs_air;
pub mod calculs_generation;
pub mod chaudieres_gaz_fioul_bois;
pub mod distribution;
pub mod emetteurs;
pub mod emission;
pub mod generateur_direct_a_effet_joule;
pub mod generation;
pub mod pac_moteur_gaz;
pub mod poeles_et_inserts;
pub mod relances;
pub mod reseau_de_chaleur_et_de_froid;
pub mod reseaux_cta;
pub mod saisons_fonctionnement;
pub mod sources_amont;
pub mod systeme_de_cogeneration;
pub mod systemes_groupe;
pub mod thermodynamique_elec;
pub mod thermodynamique_gaz;
