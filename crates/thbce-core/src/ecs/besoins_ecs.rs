// SPDX-License-Identifier: GPL-3.0-or-later
//! Émission et besoins d'ECS : fiches `C_EMI_Emission_ECS` (Th-BCE 2020 §9.5) et
//! `C_EIN_besoins_ECS` (§9.6), pour les usages d'habitation.

use crate::entree::{AppareilsEcs, EmetteurEcs, UsageZone};
use crate::scenarios::scenarios_conventionnels::nadeq;

/// « ρw » : masse volumique de l'eau, en kg/L (tableau 276).
pub const RHO_W_KG_L: f64 = 1.0;
/// « cw » : capacité calorifique massique de l'eau, en Wh/(kg.K) (tableau 276).
pub const C_W_WH_KG_K: f64 = 1.163;
/// « θuw » : température de l'eau mitigée au puisage, en °C (tableaux 271 et 276).
pub const THETA_UW_C: f64 = 40.0;
/// « Ratdouches-bains » en maison individuelle ou accolée et en logement collectif
/// (tableau 274).
pub const RAT_DOUCHES_BAINS_HABITATION: f64 = 0.8;
/// Plafond des besoins unitaires hebdomadaires, en litres à 40 °C par adulte équivalent
/// (éq. 1686 et 1690).
pub const A_MAX_L: f64 = 392.0;
/// Besoins hebdomadaires par m² desservi rapportés à l'adulte équivalent, en litres à 40 °C
/// (éq. 1686 et 1690 : `40 × A/Nadeq`).
pub const A_PAR_M2_L: f64 = 40.0;

/// « gainem » des trois catégories d'émetteurs d'ECS (tableau 273) : mélangeurs 0 %, mitigeurs
/// thermostatiques ou économes 5 %, temporisateurs et robinets électroniques 7 %.
pub const GAINS_EMETTEURS: [f64; 3] = [0.0, 0.05, 0.07];

/// « gainapp » (tableau 275).
pub fn gain_appareils(a: AppareilsEcs) -> f64 {
    match a {
        AppareilsEcs::DoucheSeule => 0.05,
        AppareilsEcs::BaignoireSabot => 0.025,
        AppareilsEcs::BaignoireStandard => 0.0,
        AppareilsEcs::GrandeBaignoire => -0.025,
    }
}

/// « corr-e » : coefficient correctif des émetteurs et des appareils d'ECS (éq. 1676 à 1678) :
/// `(1 − Σ M_partem(i) × gainem(i)) × (1 − Ratdouches-bains × gainapp)`.
pub fn corr_e(e: &EmetteurEcs) -> f64 {
    let corr_em = 1.0
        - [e.part_melangeur, e.part_mitigeur, e.part_temporisateur]
            .iter()
            .zip(GAINS_EMETTEURS)
            .map(|(p, g)| p * g)
            .sum::<f64>();
    let corr_app = 1.0 - RAT_DOUCHES_BAINS_HABITATION * gain_appareils(e.appareils);
    corr_app * corr_em
}

/// Besoins hebdomadaires d'un émetteur d'ECS équivalent (§9.6.3.2 et §9.6.3.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BesoinsHebdomadaires {
    /// « Agr,em-e » : surface desservie, en m² (éq. 1674 et 1682).
    pub a_m2: f64,
    /// « Nadeq-e » = « Nu » : adultes équivalents (éq. 1685 et 1689).
    pub nadeq: f64,
    /// « a » : besoins unitaires, en litres à 40 °C par adulte équivalent et par semaine
    /// (éq. 1686 et 1690).
    pub a_l: f64,
    /// « Vuw_hebdo » : volume hebdomadaire à 40 °C, en L (éq. 1691).
    pub v_uw_hebdo_l: f64,
    /// « corr-e » (éq. 1678).
    pub corr: f64,
    /// « Vuw_hebdo_corr » : volume corrigé, en L (éq. 1692).
    pub v_uw_hebdo_corr_l: f64,
}

impl BesoinsHebdomadaires {
    /// Th-BCE 2020 §9.6.3.2.2.2 : `A = Ratem × Agr` ; surface moyenne d'une maison ou d'un
    /// logement `A/Nb` ; `Nmax` et `Nadeq-e` par les éq. (1684), (1685), (1688) et (1689),
    /// identiques à celles des scénarios (§4.1.3, éq. 34) ; `a = min(392 ; 40 × A/Nadeq-e)` ;
    /// `Vuw_hebdo = a × Nadeq-e` ; `Vuw_hebdo_corr = Vuw_hebdo × corr-e`. `None` hors habitation.
    pub fn nouveaux(e: &EmetteurEcs, usage: UsageZone, a_gr_m2: f64) -> Option<Self> {
        let a_m2 = e.rat_em * a_gr_m2;
        let nb = e.nb_logements.max(1);
        let nadeq = nadeq(usage, a_m2 / f64::from(nb), nb)?;
        let a_l = if nadeq > 0.0 {
            A_MAX_L.min(A_PAR_M2_L * a_m2 / nadeq)
        } else {
            0.0
        };
        let v = a_l * nadeq;
        let corr = corr_e(e);
        Some(Self {
            a_m2,
            nadeq,
            a_l,
            v_uw_hebdo_l: v,
            corr,
            v_uw_hebdo_corr_l: v * corr,
        })
    }

    /// « Qw » d'un pas de temps, en Wh (éq. 1693) : `ρw·cw·(Vuw_hebdo_corr × ah)·(θuw − θcw)`.
    pub fn q_w_wh(&self, ah: f64, theta_cw_c: f64) -> f64 {
        RHO_W_KG_L * C_W_WH_KG_K * self.v_uw_hebdo_corr_l * ah * (THETA_UW_C - theta_cw_c)
    }

    /// « Qw_bruts » d'un pas de temps, en Wh, sans correction des émetteurs (éq. 1694).
    pub fn q_w_bruts_wh(&self, ah: f64, theta_cw_c: f64) -> f64 {
        RHO_W_KG_L * C_W_WH_KG_K * self.v_uw_hebdo_l * ah * (THETA_UW_C - theta_cw_c)
    }
}
