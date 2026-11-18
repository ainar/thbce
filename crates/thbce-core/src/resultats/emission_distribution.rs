// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats des systèmes du groupe (émission, distribution du groupe) et des distributions
//! intergroupes de chauffage et de refroidissement.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ResultatsEcsEmetteur, ResultatsSaisons};
use crate::conventions::arrondir_restitution;
use crate::entree::Id;

/// Réseau de distribution intergroupes, calcul Th-C.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsIntergroupes {
    /// Identifiant du réseau.
    pub id: Id,
    /// Identifiant de la génération qui l'alimente.
    pub generation: Id,
    /// Demande de chauffage transmise à la génération, pertes du réseau comprises, en kWh.
    pub q_ch_kwh: f64,
    /// Demande de refroidissement transmise à la génération (valeur absolue), en kWh.
    pub q_fr_kwh: f64,
    /// Demandes mensuelles de chauffage, en kWh (mois des scénarios).
    pub q_ch_mensuel_kwh: Vec<f64>,
    /// Demandes mensuelles de refroidissement, en kWh.
    pub q_fr_mensuel_kwh: Vec<f64>,
    /// Consommation des circulateurs du réseau, en kWh d'énergie finale électrique.
    pub w_aux_kwh: f64,
    /// Pertes du réseau vers le volume chauffé, en kWh.
    pub pertes_vc_kwh: f64,
    /// Pertes du réseau hors volume chauffé, en kWh.
    pub pertes_hvc_kwh: f64,
    /// Nombre d'heures où le réseau transmet une demande.
    pub nbh_fonct: u32,
    /// Température de départ extrême demandée à la génération (la plus haute en chauffage, la
    /// plus basse en refroidissement), en °C ; absente pour un réseau fictif.
    pub theta_dep_extreme_c: Option<f64>,
}

/// Systèmes d'émission d'un groupe, calcul Th-C (§8.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsSystemesGroupe {
    /// « Qsys_ch » : énergie de chauffage requise par les émetteurs du groupe, pertes au dos
    /// comprises, en kWh (éq. 824).
    pub q_sys_ch_kwh: f64,
    /// « Qsys_fr » : énergie de refroidissement requise (valeur absolue), en kWh.
    pub q_sys_fr_kwh: f64,
    /// Valeurs mensuelles de `q_sys_ch_kwh`, en kWh.
    pub q_sys_ch_mensuel_kwh: Vec<f64>,
    /// Valeurs mensuelles de `q_sys_fr_kwh`, en kWh.
    pub q_sys_fr_mensuel_kwh: Vec<f64>,
    /// Énergies requises par émetteur (éq. 810).
    pub emetteurs: Vec<ResultatsEmetteur>,
    /// « Wvent_loc_tot » : consommation des ventilateurs locaux, en kWh d'énergie finale
    /// électrique (éq. 815).
    pub w_vent_loc_kwh: f64,
    /// Nombre d'heures de relance (éq. 848).
    pub nbh_relance: u32,
    /// Réseaux de distribution du groupe (§8.7 et §8.8).
    pub distributions: Vec<ResultatsDistributionGroupe>,
    /// Pertes et consommations des systèmes récupérées par le groupe (§11.1), en kWh.
    pub recup_kwh: f64,
    /// Émetteurs d'ECS équivalents du groupe et leurs distributions (§9.5 à 9.7).
    pub ecs: Vec<ResultatsEcsEmetteur>,
    /// Saisons propres du groupe en calcul Th-C (§4.6), dont les générations tirent les
    /// saisons effectives (§8.4).
    pub saisons_th_c: Option<ResultatsSaisons>,
}

/// Réseau de distribution du groupe, calcul Th-C (valeurs absolues en refroidissement).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsDistributionGroupe {
    /// Identifiant.
    pub id: Id,
    /// Demande transmise au réseau intergroupes, pertes comprises, en kWh.
    pub q_kwh: f64,
    /// Pertes vers le volume chauffé, en kWh (récupérables, §11.1).
    pub pertes_vc_kwh: f64,
    /// Pertes hors volume chauffé, en kWh.
    pub pertes_hvc_kwh: f64,
    /// Consommation du circulateur, en kWh d'énergie finale électrique.
    pub w_aux_kwh: f64,
}

/// Émetteur, calcul Th-C.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsEmetteur {
    /// Identifiant.
    pub id: Id,
    /// Énergie de chauffage requise, en kWh.
    pub q_ch_kwh: f64,
    /// Énergie de refroidissement requise (valeur absolue), en kWh.
    pub q_fr_kwh: f64,
}

impl ResultatsSystemesGroupe {
    pub(super) fn arrondir(&mut self) {
        let a = arrondir_restitution;
        self.q_sys_ch_kwh = a(self.q_sys_ch_kwh);
        self.q_sys_fr_kwh = a(self.q_sys_fr_kwh);
        self.w_vent_loc_kwh = a(self.w_vent_loc_kwh);
        for x in self
            .q_sys_ch_mensuel_kwh
            .iter_mut()
            .chain(self.q_sys_fr_mensuel_kwh.iter_mut())
        {
            *x = a(*x);
        }
        for e in &mut self.emetteurs {
            e.q_ch_kwh = a(e.q_ch_kwh);
            e.q_fr_kwh = a(e.q_fr_kwh);
        }
        self.recup_kwh = a(self.recup_kwh);
        for e in &mut self.ecs {
            for x in [
                &mut e.a_m2,
                &mut e.nadeq,
                &mut e.a_l,
                &mut e.v_uw_hebdo_corr_l,
                &mut e.q_w_kwh,
                &mut e.q_w_bruts_kwh,
                &mut e.q_w_2nd_kwh,
                &mut e.pertes_vc_kwh,
                &mut e.pertes_hvc_kwh,
            ] {
                *x = a(*x);
            }
            e.corr = (e.corr * 1e4).round() / 1e4;
            if let Some(r) = &mut e.recuperateur {
                r.q_w_sans_recuperateur_kwh = a(r.q_w_sans_recuperateur_kwh);
                r.w_relevage_kwh = a(r.w_relevage_kwh);
                r.theta_entrant_moyen_c = r.theta_entrant_moyen_c.map(a);
            }
        }
        for d in &mut self.distributions {
            d.q_kwh = a(d.q_kwh);
            d.pertes_vc_kwh = a(d.pertes_vc_kwh);
            d.pertes_hvc_kwh = a(d.pertes_hvc_kwh);
            d.w_aux_kwh = a(d.w_aux_kwh);
        }
    }
}

impl ResultatsIntergroupes {
    pub(super) fn arrondir(&mut self) {
        let a = arrondir_restitution;
        for x in [
            &mut self.q_ch_kwh,
            &mut self.q_fr_kwh,
            &mut self.w_aux_kwh,
            &mut self.pertes_vc_kwh,
            &mut self.pertes_hvc_kwh,
        ] {
            *x = a(*x);
        }
        for x in self
            .q_ch_mensuel_kwh
            .iter_mut()
            .chain(self.q_fr_mensuel_kwh.iter_mut())
        {
            *x = a(*x);
        }
        self.theta_dep_extreme_c = self.theta_dep_extreme_c.map(a);
    }
}
