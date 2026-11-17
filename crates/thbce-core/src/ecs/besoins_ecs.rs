// SPDX-License-Identifier: GPL-3.0-or-later
//! Émission et besoins d'ECS : fiches `C_EMI_Emission_ECS` (Th-BCE 2020 §9.5) et
//! `C_EIN_besoins_ECS` (§9.6), pour les 28 usages : adultes équivalents en maison individuelle
//! ou accolée et en logement collectif (§9.6.3.2.2.2), besoins unitaires du tableau 277 et
//! nombre d'unités de l'émetteur dans les autres usages (§9.6.3.2.2.1).

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
/// « Ratdouches-bains » en bureaux (tableau 274).
pub const RAT_DOUCHES_BAINS_BUREAUX: f64 = 0.5;

/// « Ratdouches-bains » : part conventionnelle des besoins d'ECS dédiée aux douches et aux
/// bains (§9.5.3.4, tableau 274) : 80 % en maison individuelle ou accolée et en logement
/// collectif, 50 % en bureaux, 0 % dans les autres usages.
pub fn rat_douches_bains(usage: UsageZone) -> f64 {
    match usage {
        UsageZone::MaisonIndividuelle | UsageZone::LogementCollectif => {
            RAT_DOUCHES_BAINS_HABITATION
        }
        UsageZone::Bureaux => RAT_DOUCHES_BAINS_BUREAUX,
        _ => 0.0,
    }
}
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
pub fn corr_e(e: &EmetteurEcs, usage: UsageZone) -> f64 {
    let corr_em = 1.0
        - [e.part_melangeur, e.part_mitigeur, e.part_temporisateur]
            .iter()
            .zip(GAINS_EMETTEURS)
            .map(|(p, g)| p * g)
            .sum::<f64>();
    let corr_app = 1.0 - rat_douches_bains(usage) * gain_appareils(e.appareils);
    corr_app * corr_em
}

/// « Nu » d'un émetteur d'ECS équivalent hors habitation, en unités du tableau 277.
///
/// Th-BCE 2020 §9.6.3.2.2.1 : « le nombre d'unités caractéristiques Nu est un paramètre
/// d'intégration ». Le tableau 277 donne la même unité à tous les usages, le m² de surface
/// utile : sans saisie de `nu`, l'émetteur dessert `Ratem × Agr` (éq. 1674 et 1682), surface
/// utile que la méthode lui attribue déjà (lecture Q56).
pub fn nu_tertiaire(e: &EmetteurEcs, a_gr_m2: f64) -> f64 {
    e.nu.unwrap_or(e.rat_em * a_gr_m2)
}

/// Besoins hebdomadaires d'un émetteur d'ECS équivalent (§9.6.3.2 et §9.6.3.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BesoinsHebdomadaires {
    /// « Agr,em-e » : surface desservie, en m² (éq. 1674 et 1682).
    pub a_m2: f64,
    /// « Nu » : nombre d'unités de l'émetteur — adultes équivalents « Nadeq-e » en maison
    /// individuelle ou accolée et en logement collectif (éq. 1685 et 1689), unités du
    /// tableau 277 (m² de surface utile) dans les autres usages.
    pub nadeq: f64,
    /// « a » : besoins unitaires, en litres à 40 °C par unité et par semaine (éq. 1686 et 1690 ;
    /// tableau 277 hors habitation).
    pub a_l: f64,
    /// « Vuw_hebdo » : volume hebdomadaire à 40 °C, en L (éq. 1691).
    pub v_uw_hebdo_l: f64,
    /// « corr-e » (éq. 1678).
    pub corr: f64,
    /// « Vuw_hebdo_corr » : volume corrigé, en L (éq. 1692).
    pub v_uw_hebdo_corr_l: f64,
}

impl BesoinsHebdomadaires {
    /// Th-BCE 2020 §9.6.3.2 : `A = Ratem × Agr` (éq. 1682).
    ///
    /// - Maison individuelle ou accolée, logement collectif (§9.6.3.2.2.2) : surface moyenne
    ///   d'une maison ou d'un logement `A/Nb` ; `Nmax` et `Nadeq-e` par les éq. (1684), (1685),
    ///   (1688) et (1689), identiques à celles des scénarios (§4.1.3, éq. 34) ;
    ///   `a = min(392 ; 40 × A/Nadeq-e)`.
    /// - Autres usages (§9.6.3.2.2.1) : `a` est le besoin unitaire du tableau 277, figé dans les
    ///   scénarios conventionnels ; `Nu`, paramètre d'intégration, est le nombre d'unités de
    ///   l'émetteur ([`nu_tertiaire`]).
    ///
    /// Dans tous les cas `Vuw_hebdo = a × Nu` (éq. 1691) et `Vuw_hebdo_corr = Vuw_hebdo × corr-e`
    /// (éq. 1692). `None` si l'usage n'a pas de besoin unitaire.
    pub fn nouveaux(e: &EmetteurEcs, usage: UsageZone, a_gr_m2: f64) -> Option<Self> {
        let a_m2 = e.rat_em * a_gr_m2;
        let (nu, a_l) = if usage.est_habitation() {
            let nb = e.nb_logements.max(1);
            let nadeq = nadeq(usage, a_m2 / f64::from(nb), nb)?;
            let a_l = if nadeq > 0.0 {
                A_MAX_L.min(A_PAR_M2_L * a_m2 / nadeq)
            } else {
                0.0
            };
            (nadeq, a_l)
        } else {
            let a_l = thbce_data::scenarios::usage(usage.numero())?
                .ecs
                .volume_l_semaine_unite?;
            (nu_tertiaire(e, a_gr_m2), a_l)
        };
        let v = a_l * nu;
        let corr = corr_e(e, usage);
        Some(Self {
            a_m2,
            nadeq: nu,
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
