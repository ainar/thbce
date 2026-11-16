// SPDX-License-Identifier: GPL-3.0-or-later
//! Lecture et écriture autour du moteur Th-BCE 2020.
//!
//! Toutes les fonctions travaillent sur des octets ou des chaînes en mémoire, jamais sur des
//! chemins : elles servent aussi bien l'exécutable que le module WebAssembly.
//!
//! - [`json`] : format d'entrée JSON natif et son JSON Schema ;
//! - [`meteo`] : lecture des fichiers météo fournis par l'utilisateur ;
//! - [`rapport`] : mise en forme lisible des rapports de validation.
//!
//! Le lecteur RSEE XML n'existe pas encore : le schéma XSD n'est pas public et l'annexe VI de
//! l'arrêté ne nomme aucun élément XML. Il sera écrit à partir de fichiers d'exemple
//! (décision Q3, `docs/questions.md`).

pub mod json;
pub mod meteo;
pub mod rapport;
