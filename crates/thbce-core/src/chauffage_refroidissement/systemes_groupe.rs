// SPDX-License-Identifier: GPL-3.0-or-later
//! Systèmes de chauffage et de refroidissement d'un groupe, en aval de l'émission : énergies
//! requises par émetteur, ventilateurs locaux, réseaux de distribution du groupe (Th-BCE 2020
//! §8.1.3.7, §8.1.3.8, §8.6 à 8.8), et leurs cumuls annuels.

use crate::entree::{FonctionReseau, Groupe};

use super::distribution::{DistributionGroupeResolue, SortiesDistributionGroupe};
use super::emetteurs::{
    besoins_emetteur_wh, regime_ventilateurs, ConditionsVentilateurs, EmetteurResolu,
    EmissionEquivalente, RegimeVentilateurs,
};
use super::relances::Relance;

/// Systèmes d'un groupe, prêts pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemesGroupe {
    /// Émetteurs.
    pub emetteurs: Vec<EmetteurResolu>,
    /// Émission équivalente (éq. 797 à 799).
    pub equivalente: EmissionEquivalente,
    /// « iclim ».
    pub i_clim: bool,
    /// Programmation des relances et son état.
    pub relance: Relance,
    /// Réseaux de distribution du groupe.
    pub distributions: Vec<DistributionGroupeResolue>,
    /// Régime des ventilateurs locaux de chaque émetteur au pas précédent.
    pub regimes: Vec<RegimeVentilateurs>,
    /// Sorties du dernier pas de temps calculé.
    pub sorties: SortiesSystemesGroupe,
    /// Cumuls annuels.
    pub cumuls: CumulsSystemesGroupe,
}

/// Sorties des systèmes d'un groupe à un pas de temps.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SortiesSystemesGroupe {
    /// `(Qsys_ch^em, Qsys_fr^em)` de chaque émetteur, en Wh (éq. 810).
    pub q_em_wh: Vec<(f64, f64)>,
    /// « Qsys_ch(h) » du groupe, somme sur les émetteurs, en Wh (éq. 824).
    pub q_sys_ch_wh: f64,
    /// « Qsys_fr(h) » du groupe, en Wh, négatif ou nul (éq. 824).
    pub q_sys_fr_wh: f64,
    /// « Wvent_loc_tot(h) », en Wh (éq. 815).
    pub w_vent_loc_wh: f64,
    /// « Φvent_loc_vc(h) », en Wh (éq. 816).
    pub phi_vent_loc_vc_wh: f64,
    /// Débit massique de recirculation de chaque émetteur, en kg/s (éq. 814).
    pub qm_recirc_kg_s: Vec<f64>,
    /// « irelance^gr(h) » (éq. 848).
    pub relance: bool,
    /// « θi,moy^gr(h) » : température moyenne de l'air après croisement, en °C.
    pub theta_i_moy_c: f64,
    /// Sorties de chaque réseau de distribution du groupe.
    pub distributions: Vec<SortiesDistributionGroupe>,
}

/// Cumuls annuels des systèmes d'un groupe.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CumulsSystemesGroupe {
    /// Énergie de chauffage requise par chaque émetteur, en Wh.
    pub q_em_ch_wh: Vec<f64>,
    /// Énergie de refroidissement requise par chaque émetteur, en Wh (valeur absolue).
    pub q_em_fr_wh: Vec<f64>,
    /// Besoins mensuels du groupe en chauffage, émission comprise (éq. 824), en Wh.
    pub q_sys_ch_mensuel_wh: [f64; 12],
    /// Besoins mensuels du groupe en refroidissement (valeur absolue), en Wh.
    pub q_sys_fr_mensuel_wh: [f64; 12],
    /// Consommation des ventilateurs locaux, en Wh.
    pub w_vent_loc_wh: f64,
    /// Nombre de pas de temps en relance.
    pub nbh_relance: u32,
}

impl SystemesGroupe {
    /// Résout les systèmes d'un groupe ; `intergroupes` donne l'indice d'un réseau intergroupes
    /// du bâtiment. `None` si le groupe ne décrit pas d'émetteur.
    pub fn nouveaux(g: &Groupe, intergroupes: impl Fn(&str) -> Option<usize>) -> Option<Self> {
        if g.emetteurs.is_empty() {
            return None;
        }
        let distributions: Vec<DistributionGroupeResolue> = g
            .distributions
            .iter()
            .map(|d| DistributionGroupeResolue::nouvelle(d, &intergroupes))
            .collect();
        let indice = |id: &str| distributions.iter().position(|d| d.id == id);
        let emetteurs: Vec<EmetteurResolu> = g
            .emetteurs
            .iter()
            .map(|e| EmetteurResolu::nouveau(e, indice))
            .collect();
        let equivalente = EmissionEquivalente::nouvelle(&emetteurs);
        let p = g.programmation.as_ref();
        let n = emetteurs.len();
        let nd = distributions.len();
        Some(Self {
            equivalente,
            i_clim: g.iclim.unwrap_or(false),
            relance: Relance::nouvelle(
                p.map_or(crate::entree::ProgrammationChauffage::Horloge, |p| {
                    p.chauffage
                }),
                p.and_then(|p| p.refroidissement),
            ),
            distributions,
            regimes: vec![RegimeVentilateurs::default(); n],
            sorties: SortiesSystemesGroupe {
                q_em_wh: vec![(0.0, 0.0); n],
                qm_recirc_kg_s: vec![0.0; n],
                distributions: vec![SortiesDistributionGroupe::default(); nd],
                ..SortiesSystemesGroupe::default()
            },
            cumuls: CumulsSystemesGroupe {
                q_em_ch_wh: vec![0.0; n],
                q_em_fr_wh: vec![0.0; n],
                ..CumulsSystemesGroupe::default()
            },
            emetteurs,
        })
    }

    /// Calcule, après l'émission, les énergies requises par émetteur (éq. 810) et les
    /// ventilateurs locaux (éq. 811 à 816, en calcul des consommations seulement : `th_c`).
    /// Le pas se termine par [`SystemesGroupe::distribuer`], après le bilan hydrique.
    ///
    /// `rho_air_kg_m3` : masse volumique de l'air du groupe au pas précédent (éq. 814).
    #[allow(clippy::too_many_arguments)]
    pub fn emettre(
        &mut self,
        phi_util_w: f64,
        th_c: bool,
        cv: &ConditionsVentilateurs,
        relance: bool,
        theta_i_moy_c: f64,
        rho_air_kg_m3: f64,
    ) {
        let s = &mut self.sorties;
        s.relance = relance;
        s.theta_i_moy_c = theta_i_moy_c;
        s.q_sys_ch_wh = 0.0;
        s.q_sys_fr_wh = 0.0;
        s.w_vent_loc_wh = 0.0;
        for (i, e) in self.emetteurs.iter().enumerate() {
            let rat = (
                self.equivalente.rat_eff_ch[i],
                self.equivalente.rat_eff_fr[i],
            );
            let q = besoins_emetteur_wh(e, rat.0, rat.1, phi_util_w);
            s.q_em_wh[i] = q;
            s.q_sys_ch_wh += q.0;
            s.q_sys_fr_wh += q.1;
            let regime = match (&e.ventilateurs, th_c) {
                (Some(v), true) => regime_ventilateurs(e, v, rat, q, cv, self.regimes[i]),
                _ => RegimeVentilateurs::default(),
            };
            self.regimes[i] = regime;
            s.w_vent_loc_wh += regime.w_vent_loc_wh;
            s.qm_recirc_kg_s[i] = rho_air_kg_m3 * regime.q_v_recirc_m3_h / 3600.0;
        }
        // Éq. (813) et (816) : toute la consommation des ventilateurs est transmise à
        // l'ambiance.
        s.phi_vent_loc_vc_wh = s.w_vent_loc_wh;
    }

    /// « θbatt_dim^em » de l'émetteur d'indice `i`, en °C (éq. 817 et 818) : 9 °C sur un réseau
    /// fictif (climatiseur à détente directe).
    pub fn theta_batt_dim_c(&self, i: usize) -> f64 {
        let _ = i;
        super::bilan_hydrique::THETA_BATT_DIM_DETENTE_DIRECTE_C
    }

    /// Termine le pas de temps : ajoute l'énergie latente de déshumidification de chaque
    /// émetteur (éq. 828, `latent_wh` ≤ 0, vide hors calcul des consommations), puis calcule
    /// les réseaux de distribution du groupe (§8.7 et §8.8) et les cumuls.
    pub fn distribuer(&mut self, latent_wh: &[f64], mois: usize) {
        let s = &mut self.sorties;
        let (relance, theta_i_moy_c) = (s.relance, s.theta_i_moy_c);
        for (q, l) in s.q_em_wh.iter_mut().zip(latent_wh) {
            q.1 += l;
            s.q_sys_fr_wh += l;
        }
        for (i, q) in s.q_em_wh.iter().enumerate() {
            self.cumuls.q_em_ch_wh[i] += q.0;
            self.cumuls.q_em_fr_wh[i] -= q.1;
        }
        for (k, d) in self.distributions.iter().enumerate() {
            let q: f64 = self
                .emetteurs
                .iter()
                .zip(&s.q_em_wh)
                .map(|(e, q)| match d.fonction {
                    FonctionReseau::Chauffage => {
                        if e.chaud.is_some_and(|c| c.distribution == k) {
                            q.0
                        } else {
                            0.0
                        }
                    }
                    FonctionReseau::Refroidissement => {
                        if e.froid.is_some_and(|c| c.distribution == k) {
                            q.1
                        } else {
                            0.0
                        }
                    }
                })
                .sum();
            s.distributions[k] = d.pas(q, theta_i_moy_c, relance);
        }
        let c = &mut self.cumuls;
        if let (Some(ch), Some(fr)) = (
            c.q_sys_ch_mensuel_wh.get_mut(mois),
            c.q_sys_fr_mensuel_wh.get_mut(mois),
        ) {
            *ch += s.q_sys_ch_wh;
            *fr -= s.q_sys_fr_wh;
        }
        c.w_vent_loc_wh += s.w_vent_loc_wh;
        c.nbh_relance += u32::from(relance);
    }
}
