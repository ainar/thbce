// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats des parties communes et des déplacements : consommations électriques par poste
//! du logement collectif, ascenseurs et parcs de stationnement.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ValeurParDefaut;
use crate::conventions::arrondir_restitution;
use crate::entree::Id;

/// Consommation électrique d'un poste, annuelle et par mois des scénarios, en kWh d'énergie
/// finale.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct PosteElectrique {
    /// Consommation annuelle, en kWh.
    pub annuel_kwh: f64,
    /// Consommation annuelle par m² de surface de référence du niveau (zone ou bâtiment), en
    /// kWh/m².
    pub annuel_kwh_m2: f64,
    /// Consommation des douze mois, en kWh.
    pub mensuel_kwh: Vec<f64>,
}

impl PosteElectrique {
    pub(super) fn arrondir(&mut self) {
        self.annuel_kwh = arrondir_restitution(self.annuel_kwh);
        self.annuel_kwh_m2 = arrondir_restitution(self.annuel_kwh_m2);
        for x in &mut self.mensuel_kwh {
            *x = arrondir_restitution(*x);
        }
    }
}

/// Postes d'électricité propres au logement collectif et aux parkings, distincts pour que le
/// calcul des indicateurs les intègre selon le texte : éclairage (éq. 2510 : parties communes
/// et parkings), auxiliaires de ventilation (éq. 2511 : parkings), déplacements (éq. 2513 :
/// ascenseurs). Aucun ne produit d'apport interne dans le modèle thermique (`docs/questions/Q050.md`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct ConsommationsCollectif {
    /// Éclairage des circulations, hall et escalier (§7.2, éq. 792 à 794).
    pub eclairage_parties_communes: PosteElectrique,
    /// Ascenseurs (§10.1, éq. 2310).
    pub ascenseurs: PosteElectrique,
    /// Ventilation des parkings fermés (§10.3, éq. 2346 et §10.3.3.8).
    pub ventilation_parkings: PosteElectrique,
    /// Éclairage des parkings (§10.4, éq. 2353 et 2354).
    pub eclairage_parkings: PosteElectrique,
}

impl ConsommationsCollectif {
    /// Total des quatre postes, en kWh.
    pub fn total_kwh(&self) -> f64 {
        self.eclairage_parties_communes.annuel_kwh
            + self.ascenseurs.annuel_kwh
            + self.ventilation_parkings.annuel_kwh
            + self.eclairage_parkings.annuel_kwh
    }

    pub(crate) fn arrondir(&mut self) {
        self.eclairage_parties_communes.arrondir();
        self.ascenseurs.arrondir();
        self.ventilation_parkings.arrondir();
        self.eclairage_parkings.arrondir();
    }
}

/// Une cabine d'ascenseur (§10.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsAscenseur {
    /// Identifiant de la cabine.
    pub id: Id,
    /// « Ndem » : nombre de démarrages par an (éq. 2283).
    pub nb_demarrages: f64,
    /// « Tmobh » : durée moyenne de déplacement par jour, en h (éq. 2292).
    pub t_mob_h: f64,
    /// « Pti » : puissance de veille cabine immobile, en W.
    pub pti_w: f64,
    /// « Etm » : consommation annuelle en mobilité, en kWh (éq. 2302).
    pub e_tm_kwh: f64,
    /// « Eti » : consommation annuelle en immobilité, en kWh (éq. 2303).
    pub e_ti_kwh: f64,
    /// « Etot », en kWh (éq. 2304).
    pub e_tot_kwh: f64,
    /// Valeurs par défaut du texte appliquées.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// Un parking (§10.3 et §10.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsParking {
    /// Identifiant du parking.
    pub id: Id,
    /// « Event » : consommation annuelle des ventilateurs, en kWh (éq. 2346).
    pub e_vent_kwh: f64,
    /// « Eec » : consommation annuelle de l'éclairage, en kWh (éq. 2354).
    pub e_ecl_kwh: f64,
    /// Valeurs par défaut du texte appliquées.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}
