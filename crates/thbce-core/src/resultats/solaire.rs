// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats de la production solaire d'ECS et de chauffage : productions collectives,
//! boucles solaires et systèmes solaires combinés.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ResultatsStockage;
use crate::conventions::arrondir_restitution;
use crate::entree::Id;

/// Production solaire collective à appoints décentralisés (CESCI ou CESCAI, fiches 9.16 et
/// 9.17), calcul Th-C.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsSolaireCollectif {
    /// Identifiant de la production.
    pub id: Id,
    /// « Type_PCAD ».
    pub type_pcad: crate::entree::TypePcad,
    /// Boucle solaire collective (« Esol_tot^BS », « Eef_aux_tot^BS ») ; l'énergie solaire va
    /// aux ballons individuels (CESCI) ou au ballon central (CESCAI).
    pub solaire: ResultatsBoucleSolaire,
    /// Ensembles de ballons individuels, pour tous les ballons de chaque ensemble ; l'énergie
    /// injectée en zone basse (boucle solaire ou réseau intergroupe) est `q_fou_sto_base_kwh`,
    /// celle des appoints intégrés `q_fou_sto_ap_kwh`.
    pub ensembles: Vec<ResultatsStockage>,
    /// Ballon central d'un CESCAI : `q_fou_sto_base_kwh` est l'énergie solaire,
    /// `q_fou_ecs_kwh` l'énergie fournie au réseau intergroupe (ballons individuels et pertes
    /// du réseau déduites).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ballon_central: Option<ResultatsStockage>,
    /// CESCAI : énergie fournie aux ballons individuels par le réseau intergroupe, en kWh
    /// (Σ « Qfou_ens^B », éq. 2078).
    #[serde(default)]
    pub q_fou_reseau_kwh: f64,
    /// CESCAI : pertes du réseau intergroupe déduites du ballon central, en kWh (éq. 2080).
    #[serde(default)]
    pub pertes_reseau_deduites_kwh: f64,
}

impl ResultatsSolaireCollectif {
    /// Arrondit les valeurs restituées (Q8).
    pub fn arrondir(&mut self) {
        let a = arrondir_restitution;
        for x in [
            &mut self.solaire.e_sol_kwh,
            &mut self.solaire.w_pompe_kwh,
            &mut self.solaire.pertes_vc_kwh,
            &mut self.solaire.isr_kwh_m2,
            &mut self.solaire.ecretement_kwh,
            &mut self.q_fou_reseau_kwh,
            &mut self.pertes_reseau_deduites_kwh,
        ] {
            *x = a(*x);
        }
        for x in &mut self.solaire.e_sol_mensuel_kwh {
            *x = a(*x);
        }
        for s in self
            .ensembles
            .iter_mut()
            .chain(self.ballon_central.iter_mut())
        {
            s.arrondir();
        }
    }
}

/// Boucle solaire d'un chauffe-eau solaire, calcul Th-C (§9.15 ; sorties « Esol_tot^BS » et
/// « Eef_aux_tot^BS » des fiches d'assemblage). L'énergie solaire transmise au ballon est
/// comprise dans `q_fou_sto_base_kwh` ; elle n'est pas une consommation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsBoucleSolaire {
    /// « Esol_tot^BS » : énergie solaire transmise aux ballons par les échangeurs, en kWh.
    pub e_sol_kwh: f64,
    /// La même, par mois des scénarios, en kWh.
    pub e_sol_mensuel_kwh: Vec<f64>,
    /// « Eef_aux_tot^BS » : consommation électrique des pompes, en kWh (comprise dans les
    /// consommations d'ECS de la génération, éq. 1972).
    pub w_pompe_kwh: f64,
    /// « ΦvcBS » : pertes des tuyauteries vers l'intérieur du bâtiment, en kWh (éq. 1973).
    pub pertes_vc_kwh: f64,
    /// Ensoleillement annuel sur le plan des capteurs, masques compris, en kWh/m².
    pub isr_kwh_m2: f64,
    /// Superficie d'entrée des capteurs d'un assemblage, en m².
    pub a_capteurs_m2: f64,
    /// Heures de fonctionnement de la pompe.
    pub nbh_marche: u32,
    /// Énergie retirée aux ballons par la limitation à leur température maximale (éq. 1779),
    /// en kWh : part de l'énergie solaire transmise que le ballon ne peut pas stocker (lecture
    /// Q52).
    pub ecretement_kwh: f64,
}

/// Contribution d'un système solaire combiné au chauffage, calcul Th-C.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsSsc {
    /// « Qpuisé_ch_SSC » : chaleur de chauffage puisée dans le ballon, en kWh ; elle est
    /// comprise dans l'énergie fournie en chauffage par la génération.
    pub q_puise_ch_kwh: f64,
    /// Chauffage fourni par l'appoint raccordé à l'assemblage, en kWh (éq. 2150).
    pub q_fou_ch_appoint_kwh: f64,
}
