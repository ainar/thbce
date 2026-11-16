// SPDX-License-Identifier: GPL-3.0-or-later
//! Erreurs du moteur. Aucune entrée utilisateur ne provoque de `panic!`.

use crate::validation::RapportValidation;

/// Erreur renvoyée par le moteur.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Erreur {
    /// La description du projet est invalide ; le rapport détaille chaque élément fautif.
    #[error("entrée invalide : {} erreur(s) de saisie", .0.erreurs.len())]
    Saisie(RapportValidation),
    /// L'entrée est valide mais fait appel à une partie de la méthode non encore implémentée.
    #[error("{chemin} : {element} n'est pas pris en charge par cette version du moteur")]
    NonPrisEnCharge {
        /// Chemin de l'élément dans l'entrée.
        chemin: String,
        /// Élément ou fiche de la méthode concerné.
        element: String,
    },
    /// Le calcul n'a pas abouti alors que l'entrée est valide : un cas que la validation aurait
    /// dû écarter. Le message nomme l'élément et la grandeur en cause.
    #[error("calcul impossible : {0}")]
    Calcul(String),
    /// Les données météo fournies sont incomplètes ou ne correspondent pas au projet.
    #[error("données météo : {0}")]
    Meteo(String),
}
