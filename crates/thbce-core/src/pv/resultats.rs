// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats de la production photovoltaïque (Th-BCE 2020 §12).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::global::ProductionInstallationPv;
use crate::conventions::arrondir_restitution;
use crate::entree::Id;
use crate::resultats::ValeurParDefaut;

/// Production d'une installation photovoltaïque.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsInstallationPv {
    /// Identifiant de l'installation.
    pub id: Id,
    /// « N » : nombre de modules.
    pub n_modules: u32,
    /// Puissance crête de calcul du champ, `N × Pc` de calcul, en kWc (§12.3.3.1).
    pub puissance_crete_calcul_kwc: f64,
    /// « ηSTC » : rendement des modules en conditions STC (éq. 2366).
    pub eta_stc: f64,
    /// « U1 » : coefficient d'échange avec l'ambiance, en W/(m².K) (éq. 2367).
    pub u1_w_m2k: f64,
    /// « PAC NOM » de l'onduleur, en W.
    pub p_ac_nom_w: f64,
    /// Rayonnement efficace annuel dans le plan des modules, en kWh/m².
    pub g_kwh_m2: f64,
    /// Énergie annuelle MPPT des modules, à l'entrée de l'onduleur, en kWh.
    pub e_pv_kwh: f64,
    /// « Pond » : production annuelle délivrée au réseau (sortie de l'onduleur), en énergie
    /// finale, en kWh.
    pub e_ond_kwh: f64,
    /// Production de chacun des douze mois, en kWh (mois des scénarios).
    pub e_ond_mensuel_kwh: Vec<f64>,
    /// Nombre d'heures d'extinction de l'onduleur (« LIM = 0 »).
    pub nbh_extinction: u32,
    /// Valeurs par défaut du texte appliquées.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// Production photovoltaïque d'un bâtiment, somme de ses installations (§13.2.3, éq. 2524).
///
/// La part autoconsommée et la part exportée ne sont pas calculées ici : elles relèvent du
/// chapitre 13 (éq. 2527 à 2530) et demandent la consommation électrique horaire tous usages.
/// La série horaire (`Eelec_prod_PV(h)`) est disponible par `thbce_core::series_pv`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsProductionPv {
    /// Production annuelle en énergie finale, `O_Eef_Prod_PV_annuel`, en kWh.
    pub e_prod_pv_kwh: f64,
    /// Production mensuelle, `O_Eef_Prod_PV_mois`, en kWh.
    pub e_prod_pv_mensuel_kwh: Vec<f64>,
    /// Installations.
    pub installations: Vec<ResultatsInstallationPv>,
}

impl ResultatsInstallationPv {
    /// Restitue une installation à partir de sa production horaire.
    pub fn depuis(p: &ProductionInstallationPv) -> Self {
        let c = &p.installation.capteur;
        Self {
            id: p.id.clone(),
            n_modules: c.n as u32,
            puissance_crete_calcul_kwc: c.n * c.performances.pc_w / 1000.0,
            eta_stc: c.eta_stc,
            u1_w_m2k: c.u1_w_m2k,
            p_ac_nom_w: p.installation.onduleur.p_ac_nom_w,
            g_kwh_m2: p.g_kwh_m2(),
            e_pv_kwh: p.e_pv_kwh(),
            e_ond_kwh: p.e_ond_kwh(),
            e_ond_mensuel_kwh: p.e_ond_mensuel_kwh(),
            nbh_extinction: p.nbh_extinction,
            valeurs_par_defaut: {
                let mut v = c.performances.valeurs_par_defaut.clone();
                if p.installation.onduleur.p_ac_nom_par_defaut {
                    v.push(ValeurParDefaut {
                        champ: "onduleur.p_ac_nom_w".to_owned(),
                        valeur: p.installation.onduleur.p_ac_nom_w,
                        regle: "Th-BCE 2020 §12.4.3 : PAC NOM = 0,8 × N × Pc".to_owned(),
                    });
                }
                v
            },
        }
    }

    pub(crate) fn arrondir(&mut self) {
        let a = arrondir_restitution;
        for x in [
            &mut self.puissance_crete_calcul_kwc,
            &mut self.p_ac_nom_w,
            &mut self.g_kwh_m2,
            &mut self.e_pv_kwh,
            &mut self.e_ond_kwh,
        ] {
            *x = a(*x);
        }
        // Rendement et coefficient d'échange : deux et trois décimales.
        self.eta_stc = (self.eta_stc * 1e3).round() / 1e3;
        self.u1_w_m2k = (self.u1_w_m2k * 1e3).round() / 1e3;
        for x in &mut self.e_ond_mensuel_kwh {
            *x = a(*x);
        }
    }
}

impl ResultatsProductionPv {
    /// Somme des installations d'un bâtiment.
    pub fn depuis(productions: &[ProductionInstallationPv]) -> Self {
        let mut mensuel = vec![0.0; 12];
        for p in productions {
            for (m, e) in mensuel.iter_mut().zip(p.e_ond_mensuel_kwh()) {
                *m += e;
            }
        }
        Self {
            e_prod_pv_kwh: productions.iter().map(|p| p.e_ond_kwh()).sum(),
            e_prod_pv_mensuel_kwh: mensuel,
            installations: productions
                .iter()
                .map(ResultatsInstallationPv::depuis)
                .collect(),
        }
    }

    pub(crate) fn arrondir(&mut self) {
        self.e_prod_pv_kwh = arrondir_restitution(self.e_prod_pv_kwh);
        for x in &mut self.e_prod_pv_mensuel_kwh {
            *x = arrondir_restitution(*x);
        }
        for i in &mut self.installations {
            i.arrondir();
        }
    }
}
