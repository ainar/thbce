// SPDX-License-Identifier: GPL-3.0-or-later
//! Distributions de chauffage et de refroidissement : fiches `S1_Dist_Assemblage des systèmes
//! de chauffage et de refroidissement` (Th-BCE 2020 §8.6), `C-DIST-Gestion/régulation des
//! distributions … du groupe` (§8.7), `C-Dist-Distribution … du groupe` (§8.8),
//! `C-DIST-Gestion/régulation des distributions intergroupes` (§8.9) et `C-Dist-Distribution
//! intergroupes` (§8.10).
//!
//! Chaque émetteur est rattaché, pour chacune de ses fonctions, à un réseau de distribution du
//! groupe ; chaque réseau du groupe à un réseau intergroupes de même fonction et de même type
//! (éq. 850 et 851) ; chaque réseau intergroupes à une génération. Les réseaux transmettent la
//! demande des émetteurs, augmentée de leurs pertes, à la génération (éq. 880, 887, 928, 936).
//!
//! Lectures : `docs/questions.md`, Q43.

use crate::entree::{
    DistributionGroupe, DistributionIntergroupes, FonctionReseau, ReseauGroupe, ReseauIntergroupes,
};

/// Sorties d'un réseau de distribution du groupe à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SortiesDistributionGroupe {
    /// « Qsys_ch(h) » : besoin de chauffage augmenté des pertes du réseau, en Wh (≥ 0).
    pub q_sys_ch_wh: f64,
    /// « Qsys_fr(h) » : besoin de refroidissement augmenté des pertes du réseau, en Wh (≤ 0).
    pub q_sys_fr_wh: f64,
    /// « θdep(h) », en °C.
    pub theta_dep_c: f64,
    /// « θret(h) », en °C.
    pub theta_ret_c: f64,
    /// « qeff(h) », en m³/h.
    pub q_eff_m3_h: f64,
    /// « δfonct(h) ».
    pub fonct: bool,
    /// « Modpertes(h) ».
    pub mod_pertes: f64,
    /// « irelance(h) », transmis au réseau intergroupes (éq. 852).
    pub relance: bool,
    /// « Waux(h) » : consommation du circulateur, en Wh.
    pub w_aux_wh: f64,
    /// « Φaux_vc(h) » : part de cette consommation transmise au volume chauffé, en Wh.
    pub phi_aux_vc_wh: f64,
    /// « Φpertes_vc(h) » : pertes vers le volume chauffé, en Wh (récupérables).
    pub phi_pertes_vc_wh: f64,
    /// « Φpertes_hvc(h) » : pertes hors volume chauffé, en Wh.
    pub phi_pertes_hvc_wh: f64,
}

/// Réseau de distribution du groupe, prêt pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct DistributionGroupeResolue {
    /// Identifiant.
    pub id: String,
    /// « idfonction ».
    pub fonction: FonctionReseau,
    /// Indice du réseau intergroupes du bâtiment.
    pub intergroupes: usize,
    /// Nature du réseau.
    pub reseau: ReseauGroupe,
}

impl DistributionGroupeResolue {
    /// Résout un réseau saisi ; `intergroupes` donne l'indice d'un réseau intergroupes d'après
    /// son identifiant (références vérifiées par la validation).
    pub fn nouvelle(d: &DistributionGroupe, intergroupes: impl Fn(&str) -> Option<usize>) -> Self {
        Self {
            id: d.id.clone(),
            fonction: d.fonction,
            intergroupes: intergroupes(&d.distribution_intergroupes).unwrap_or(0),
            reseau: d.reseau.clone(),
        }
    }

    /// Pas de temps du réseau.
    ///
    /// `q_em_wh` : somme des énergies requises par les émetteurs rattachés au réseau pour sa
    /// fonction (`Qsys_ch^em` ≥ 0 ou `Qsys_fr^em` ≤ 0, éq. 810) ; `theta_i_moy_c` : température
    /// moyenne de l'air du groupe après croisement ; `relance` : « irelance^gr(h) ».
    pub fn pas(
        &self,
        q_em_wh: f64,
        theta_i_moy_c: f64,
        relance: bool,
    ) -> SortiesDistributionGroupe {
        let _ = theta_i_moy_c;
        match self.reseau {
            // §8.7.3.3.1, éq. (853), et §8.8.3.1, éq. (872) et (873) : réseau fictif, demande
            // transmise telle quelle, sans pertes ni auxiliaires.
            ReseauGroupe::Fictif => {
                let (ch, fr) = match self.fonction {
                    FonctionReseau::Chauffage => (q_em_wh.max(0.0), 0.0),
                    FonctionReseau::Refroidissement => (0.0, q_em_wh.min(0.0)),
                };
                SortiesDistributionGroupe {
                    q_sys_ch_wh: ch,
                    q_sys_fr_wh: fr,
                    relance,
                    ..SortiesDistributionGroupe::default()
                }
            }
        }
    }
}

/// Sorties d'un réseau intergroupes à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SortiesDistributionIntergroupes {
    /// « Qsys_ch(h) » : demande de chauffage transmise à la génération, en Wh (éq. 919 et 928).
    pub q_sys_ch_wh: f64,
    /// « Qsys_fr(h) » : demande de refroidissement transmise à la génération, en Wh (≤ 0).
    pub q_sys_fr_wh: f64,
    /// « θmoy(h) » : température moyenne du réseau, en °C (0 pour un réseau fictif, éq. 920).
    pub theta_moy_c: f64,
    /// « θdep(h) » : température de départ demandée à la génération, en °C.
    pub theta_dep_c: f64,
    /// « θret(h) » : température de retour, en °C.
    pub theta_ret_c: f64,
    /// « irelance(h) » (éq. 888).
    pub relance: bool,
    /// « Waux(h) », en Wh.
    pub w_aux_wh: f64,
    /// « Φaux_vc(h) », en Wh.
    pub phi_aux_vc_wh: f64,
    /// « Φpertes_vc(h) », en Wh.
    pub phi_pertes_vc_wh: f64,
    /// « Φpertes_hvc(h) », en Wh.
    pub phi_pertes_hvc_wh: f64,
    /// « θi,aval,eq(h) » : température d'air vue par les émetteurs sur air ambiant d'un réseau
    /// fictif, moyenne des groupes pondérée par leurs surfaces desservies (éq. 898), en °C.
    pub theta_i_aval_eq_c: f64,
}

/// Contribution d'un réseau du groupe à un réseau intergroupes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApportReseauGroupe {
    /// Sorties du réseau du groupe.
    pub sorties: SortiesDistributionGroupe,
    /// « Ratsurf^dp,gr » du groupe pour ce réseau (éq. 891 et 892).
    pub rat_surf: f64,
    /// « θi,moy^gr(h) », en °C.
    pub theta_i_moy_c: f64,
}

/// Réseau intergroupes, prêt pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct DistributionIntergroupesResolue {
    /// Identifiant.
    pub id: String,
    /// « idfonction ».
    pub fonction: FonctionReseau,
    /// Indice de la génération.
    pub generation: usize,
    /// Nature du réseau.
    pub reseau: ReseauIntergroupes,
}

impl DistributionIntergroupesResolue {
    /// Résout un réseau saisi.
    pub fn nouvelle(
        d: &DistributionIntergroupes,
        generation: impl Fn(&str) -> Option<usize>,
    ) -> Self {
        Self {
            id: d.id.clone(),
            fonction: d.fonction,
            generation: generation(&d.generation).unwrap_or(0),
            reseau: d.reseau.clone(),
        }
    }

    /// Pas de temps du réseau à partir des réseaux du groupe qui lui sont connectés.
    pub fn pas(&self, aval: &[ApportReseauGroupe]) -> SortiesDistributionIntergroupes {
        // Éq. (888) et (889).
        let relance = aval.iter().any(|a| a.sorties.relance);
        let q_ch: f64 = aval.iter().map(|a| a.sorties.q_sys_ch_wh).sum();
        let q_fr: f64 = aval.iter().map(|a| a.sorties.q_sys_fr_wh).sum();
        match self.reseau {
            // §8.9.3.5.1, éq. (897) et (898) ; §8.10.3.1, éq. (919) et (920).
            ReseauIntergroupes::Fictif => SortiesDistributionIntergroupes {
                q_sys_ch_wh: if self.fonction == FonctionReseau::Chauffage {
                    q_ch
                } else {
                    0.0
                },
                q_sys_fr_wh: if self.fonction == FonctionReseau::Refroidissement {
                    q_fr
                } else {
                    0.0
                },
                relance,
                theta_i_aval_eq_c: aval.iter().map(|a| a.rat_surf * a.theta_i_moy_c).sum(),
                ..SortiesDistributionIntergroupes::default()
            },
        }
    }
}
