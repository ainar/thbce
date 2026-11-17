// SPDX-License-Identifier: GPL-3.0-or-later
//! Fiche `S2_PV_global_PV` (Th-BCE 2020 §12.2) : production photovoltaïque d'une installation.
//!
//! Assemble l'environnement proche (§3.2 : rayonnement sur le plan des modules, masques) et
//! l'installation (§12.1, figure 213), puis déroule l'année au pas horaire. La répartition de la
//! production entre part autoconsommée et part exportée relève du chapitre 13 (§13.2.3,
//! éq. 2527 à 2530, qui demande la consommation électrique tous usages) : ce module ne livre que
//! la série horaire de production.

use crate::bati::paroi_opaque::resoudre_masques;
use crate::climat::environnement_proche::{
    flux_atteignant_la_paroi, MasqueResolu, Soleil, SurfaceExposee,
};
use crate::climat::{calendrier::date_scenario, ClimatExterieur};
use crate::conventions::NB_HEURES_AN;
use crate::entree::Catalogue;

use super::entree::InstallationPv;
use super::installation::InstallationResolue;

/// Production annuelle d'une installation, au pas horaire.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductionInstallationPv {
    /// Identifiant de l'installation.
    pub id: String,
    /// Installation résolue.
    pub installation: InstallationResolue,
    /// « Pond » : puissance délivrée au réseau à chaque heure, en W (donc en Wh par pas).
    pub p_ond_w: Vec<f64>,
    /// « PPV » : puissance MPPT des modules, en W.
    pub p_pv_w: Vec<f64>,
    /// Rayonnement efficace `G` sur le plan des modules, en Wh/m² par pas.
    pub g_w_m2: Vec<f64>,
    /// Nombre d'heures d'extinction de l'onduleur (`LIM = 0`).
    pub nbh_extinction: u32,
}

impl ProductionInstallationPv {
    /// Production annuelle à la sortie de l'onduleur, en kWh.
    pub fn e_ond_kwh(&self) -> f64 {
        self.p_ond_w.iter().sum::<f64>() / 1000.0
    }

    /// Énergie annuelle MPPT à l'entrée de l'onduleur, en kWh.
    pub fn e_pv_kwh(&self) -> f64 {
        self.p_pv_w.iter().sum::<f64>() / 1000.0
    }

    /// Rayonnement efficace annuel dans le plan des modules, en kWh/m².
    pub fn g_kwh_m2(&self) -> f64 {
        self.g_w_m2.iter().sum::<f64>() / 1000.0
    }

    /// Production mensuelle à la sortie de l'onduleur, en kWh (mois des scénarios).
    pub fn e_ond_mensuel_kwh(&self) -> Vec<f64> {
        let mut m = vec![0.0; 12];
        for (h, p) in self.p_ond_w.iter().enumerate() {
            m[usize::from(date_scenario(h).mois) - 1] += p / 1000.0;
        }
        m
    }
}

/// Calcule la production d'une installation sur l'année du climat donné.
///
/// Les masques de l'installation sont ceux du catalogue (§3.2) ; la validation garantit leur
/// présence. `Vent` (corrigé) est une entrée du texte mais n'intervient dans aucune équation
/// de §12.3 : il n'est pas utilisé.
pub fn produire(
    i: &InstallationPv,
    catalogue: &Catalogue,
    climat: &ClimatExterieur,
) -> ProductionInstallationPv {
    let installation = InstallationResolue::nouvelle(i);
    let surface = SurfaceExposee::depuis_exposition(&i.exposition);
    let masques: Vec<MasqueResolu> = resoudre_masques(&i.exposition.masques, catalogue);
    let mut p_ond_w = Vec::with_capacity(NB_HEURES_AN);
    let mut p_pv_w = Vec::with_capacity(NB_HEURES_AN);
    let mut g_w_m2 = Vec::with_capacity(NB_HEURES_AN);
    let mut nbh_extinction = 0;
    for h in 0..NB_HEURES_AN {
        let soleil = Soleil {
            gamma_rad: climat.gamma_rad[h],
            psi_rad: climat.psi_rad[h],
        };
        let f = flux_atteignant_la_paroi(
            &surface,
            &masques,
            &soleil,
            climat.idn_w_m2[h],
            climat.idi_w_m2[h],
            0.0,
            0.0,
            h,
        );
        let s = installation.pas(
            f.theta_rad.to_degrees(),
            f.drp_w_m2,
            f.dfp_w_m2,
            f.rrp_w_m2,
            climat.te_c[h],
            climat.teciel_c[h],
        );
        nbh_extinction += u32::from(s.onduleur.eteint);
        p_ond_w.push(s.onduleur.p_ond_w);
        p_pv_w.push(s.capteur.p_pv_w);
        g_w_m2.push(s.capteur.g_w_m2);
    }
    ProductionInstallationPv {
        id: i.id.clone(),
        installation,
        p_ond_w,
        p_pv_w,
        g_w_m2,
        nbh_extinction,
    }
}
