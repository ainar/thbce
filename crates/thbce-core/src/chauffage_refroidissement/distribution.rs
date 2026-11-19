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
//! Lectures : `docs/questions/Q043.md`.

use crate::entree::{
    Circulateur, DistributionGroupe, DistributionIntergroupes, FonctionReseau, RegulationDebit,
    RegulationTemperature, ReseauGroupe, ReseauHydrauliqueGroupe, ReseauIntergroupes,
};

/// « ρeau » : masse volumique de l'eau, en kg/m³ (tableau 95).
pub const RHO_EAU_KG_M3: f64 = 998.0;
/// « Cp_eau » : capacité thermique massique de l'eau, en Wh/(kg.K) (tableau 95).
pub const CP_EAU_WH_KG_K: f64 = 1.163;
/// « θext_lim_ch » : température extérieure au-delà de laquelle la température de départ d'une
/// loi d'eau prend sa valeur minimale, en °C (tableau 95, valeur conventionnelle).
pub const THETA_EXT_LIM_CH_C: f64 = 15.0;
/// « θdep_ch_min » : température de départ minimale d'une loi d'eau, en °C (tableau 95, valeur
/// conventionnelle).
pub const THETA_DEP_CH_MIN_C: f64 = 20.0;
/// « Pcirc_vc » d'un réseau du groupe : part de la consommation du circulateur transmise au
/// volume chauffé (tableau 97, éq. 879 et 886 : 50 %) ; même valeur pour un réseau de CTA
/// hydraulique (colonne « Conv. » de la nomenclature de la fiche 8.12, p. 620 : `Pcirc_vc` 0,5,
/// `θvc_ch` 20 °C, `θvc_fr` 26 °C).
pub const P_CIRC_VC_GROUPE: f64 = 0.5;
/// « Pcirc_vc » d'un réseau intergroupes (tableau 101, éq. 927 et 935 : 0 %).
pub const P_CIRC_VC_INTERGROUPES: f64 = 0.0;
/// « θamb_ch » et « θvc_ch » : température d'ambiance conventionnelle d'un réseau intergroupes
/// en chauffage, en °C (tableaux 99 et 101).
pub const THETA_AMB_CH_C: f64 = 20.0;
/// « θamb_fr » et « θvc_fr » : la même en refroidissement, en °C.
pub const THETA_AMB_FR_C: f64 = 26.0;

/// « Modcirc » d'un circulateur (éq. 862, 870, 908, 917) : 0 sans circulateur ; `Modpertes` à
/// vitesse constante ; `Modpertes·(qeff/qnom)^(2/3)` à pression différentielle constante ;
/// `Modpertes·(0,5·qeff/qnom + 0,5·(qeff/qnom)²)^(2/3)` à pression différentielle variable.
pub fn mod_circ(c: Circulateur, mod_pertes: f64, q_eff_m3_h: f64, q_nom_m3_h: f64) -> f64 {
    let x = if q_nom_m3_h > 0.0 {
        q_eff_m3_h / q_nom_m3_h
    } else {
        1.0
    };
    match c {
        Circulateur::Aucun => 0.0,
        Circulateur::VitesseConstante => mod_pertes,
        Circulateur::PressionConstante => mod_pertes * libm::pow(x, 2.0 / 3.0),
        Circulateur::PressionVariable => mod_pertes * libm::pow(0.5 * x + 0.5 * x * x, 2.0 / 3.0),
    }
}

/// `MAX(0 ; x)` en chauffage, `MIN(0 ; x)` en refroidissement (éq. 875 et 882, 877 et 884, 923
/// et 931, 925 et 933).
fn ecart(fonction: FonctionReseau, x: f64) -> f64 {
    match fonction {
        FonctionReseau::Chauffage => x.max(0.0),
        FonctionReseau::Refroidissement => x.min(0.0),
    }
}

/// Gestion/régulation et pertes d'un réseau hydraulique du groupe à un pas de temps
/// (§8.7.3.3.2, §8.7.3.3.3, §8.8.3.2 et §8.8.3.3, éq. 854 à 871 et 874 à 887).
///
/// `q_em_wh` : demande des émetteurs (≥ 0 en chauffage, ≤ 0 en refroidissement) ;
/// `b_therm` : coefficient de l'espace tampon traversé par la partie hors volume chauffé, 1 à
/// l'extérieur.
#[allow(clippy::too_many_arguments)]
pub fn pas_hydraulique_groupe(
    r: &ReseauHydrauliqueGroupe,
    fonction: FonctionReseau,
    q_em_wh: f64,
    theta_i_moy_c: f64,
    theta_ext_c: f64,
    theta_ext_base_c: f64,
    b_therm: f64,
    relance: bool,
) -> SortiesDistributionGroupe {
    let rho_cp = RHO_EAU_KG_M3 * CP_EAU_WH_KG_K;
    // Éq. (854) et (863).
    let fonct = match fonction {
        FonctionReseau::Chauffage => q_em_wh > 0.0,
        FonctionReseau::Refroidissement => q_em_wh < 0.0,
    };
    // Éq. (855) à (858) et (864) à (867). L'énergie du pas d'une heure (Wh) est lue comme une
    // puissance (W) dans les bilans `ρ·Cp·q·Δθ`.
    let (q_eff, delta_theta_em, mod_pertes) = if fonct {
        match r.regulation_debit {
            RegulationDebit::DebitVariable => {
                let q_req = q_em_wh / (rho_cp * r.delta_theta_em_dim_k);
                let q_eff = q_req.max(r.q_resid_m3_h.unwrap_or(0.0));
                let dt = if q_eff > 0.0 {
                    q_em_wh / (rho_cp * q_eff)
                } else {
                    0.0
                };
                (q_eff, dt, 1.0)
            }
            RegulationDebit::DebitConstantIntermittent => {
                let dt = r.delta_theta_em_dim_k;
                let m = if r.q_nom_m3_h * dt == 0.0 {
                    1.0
                } else {
                    (q_em_wh / (rho_cp * r.q_nom_m3_h * dt)).min(1.0)
                };
                (r.q_nom_m3_h, dt, m)
            }
            RegulationDebit::DebitConstantContinu => {
                let dt = if r.q_nom_m3_h > 0.0 {
                    q_em_wh / (rho_cp * r.q_nom_m3_h)
                } else {
                    0.0
                };
                (r.q_nom_m3_h, dt, 1.0)
            }
        }
    } else {
        (0.0, 0.0, 0.0)
    };
    // Éq. (859) à (861), (868) et (869) : température de départ ; à l'arrêt, celle de l'air.
    let theta_dep = if fonct {
        let dim = r.theta_dep_dim_c.unwrap_or(theta_i_moy_c);
        match r.regulation_temperature {
            RegulationTemperature::DepartConstant => dim,
            RegulationTemperature::RetourConstant => {
                r.theta_ret_dim_c.unwrap_or(theta_i_moy_c) + delta_theta_em
            }
            RegulationTemperature::LoiDEau => {
                let minimum = theta_i_moy_c + delta_theta_em;
                if theta_ext_c >= THETA_EXT_LIM_CH_C {
                    minimum.max(THETA_DEP_CH_MIN_C)
                } else if theta_ext_c <= theta_ext_base_c {
                    dim
                } else {
                    minimum.max(
                        dim + (THETA_DEP_CH_MIN_C - dim) / (THETA_EXT_LIM_CH_C - theta_ext_base_c)
                            * (theta_ext_c - theta_ext_base_c),
                    )
                }
            }
        }
    } else {
        theta_i_moy_c
    };
    // Éq. (871).
    let theta_ret = if fonct {
        theta_dep - delta_theta_em
    } else {
        theta_dep
    };
    // Éq. (874) à (877) et (881) à (884).
    let theta_moy = (theta_dep + theta_ret) / 2.0;
    let pertes_vc =
        mod_pertes * r.u_vc_w_mk * r.l_vc_m * ecart(fonction, theta_moy - theta_i_moy_c);
    let theta_hvc = b_therm * theta_ext_c + (1.0 - b_therm) * theta_i_moy_c;
    let pertes_hvc = mod_pertes * r.u_hvc_w_mk * r.l_hvc_m * ecart(fonction, theta_moy - theta_hvc);
    // Éq. (862), (870), (878), (879), (885) et (886).
    let w_aux = mod_circ(r.circulateur, mod_pertes, q_eff, r.q_nom_m3_h) * r.p_aux_w;
    // Éq. (880) et (887).
    let q = q_em_wh + pertes_vc + pertes_hvc;
    let (ch, fr) = match fonction {
        FonctionReseau::Chauffage => (q, 0.0),
        FonctionReseau::Refroidissement => (0.0, q),
    };
    SortiesDistributionGroupe {
        q_sys_ch_wh: ch,
        q_sys_fr_wh: fr,
        theta_dep_c: theta_dep,
        theta_ret_c: theta_ret,
        q_eff_m3_h: q_eff,
        fonct,
        mod_pertes,
        relance,
        w_aux_wh: w_aux,
        phi_aux_vc_wh: P_CIRC_VC_GROUPE * w_aux,
        phi_pertes_vc_wh: pertes_vc,
        phi_pertes_hvc_wh: pertes_hvc,
    }
}

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
    /// Indice de l'espace tampon traversé par la partie hors volume chauffé.
    pub espace_tampon: Option<usize>,
}

impl DistributionGroupeResolue {
    /// Résout un réseau saisi ; `intergroupes` donne l'indice d'un réseau intergroupes d'après
    /// son identifiant (références vérifiées par la validation).
    pub fn nouvelle(
        d: &DistributionGroupe,
        intergroupes: impl Fn(&str) -> Option<usize>,
        espaces_tampons: impl Fn(&str) -> Option<usize>,
    ) -> Self {
        Self {
            id: d.id.clone(),
            fonction: d.fonction,
            intergroupes: intergroupes(&d.distribution_intergroupe).unwrap_or(0),
            espace_tampon: match &d.reseau {
                ReseauGroupe::Hydraulique(h) => {
                    h.espace_tampon.as_deref().and_then(espaces_tampons)
                }
                ReseauGroupe::Fictif => None,
            },
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
        theta_ext_c: f64,
        theta_ext_base_c: f64,
        b_therm: f64,
    ) -> SortiesDistributionGroupe {
        match &self.reseau {
            ReseauGroupe::Hydraulique(h) => pas_hydraulique_groupe(
                h,
                self.fonction,
                q_em_wh,
                theta_i_moy_c,
                theta_ext_c,
                theta_ext_base_c,
                b_therm,
                relance,
            ),
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
    /// Indice de l'espace tampon traversé par la partie hors volume chauffé.
    pub espace_tampon: Option<usize>,
    /// « qnom » : somme des débits nominaux des réseaux du groupe, en m³/h (éq. 902 et 911).
    pub q_nom_m3_h: f64,
    /// « qresid » : somme des débits résiduels des réseaux du groupe, en m³/h (éq. 903 et 912).
    pub q_resid_m3_h: f64,
}

impl DistributionIntergroupesResolue {
    /// Résout un réseau saisi.
    pub fn nouvelle(
        d: &DistributionIntergroupes,
        generation: impl Fn(&str) -> Option<usize>,
        espaces_tampons: impl Fn(&str) -> Option<usize>,
    ) -> Self {
        let espace_tampon = match &d.reseau {
            ReseauIntergroupes::HydrauliqueCollectif(h)
            | ReseauIntergroupes::HydrauliqueIndividuel(h) => {
                h.espace_tampon.as_deref().and_then(espaces_tampons)
            }
            ReseauIntergroupes::Fictif => None,
        };
        Self {
            id: d.id.clone(),
            fonction: d.fonction,
            generation: generation(&d.generation).unwrap_or(0),
            reseau: d.reseau.clone(),
            espace_tampon,
            q_nom_m3_h: 0.0,
            q_resid_m3_h: 0.0,
        }
    }

    /// Ajoute les débits nominal et résiduel d'un réseau du groupe connecté (éq. 902, 903, 911
    /// et 912).
    pub fn ajouter_reseau_groupe(&mut self, r: &ReseauGroupe) {
        if let ReseauGroupe::Hydraulique(h) = r {
            self.q_nom_m3_h += h.q_nom_m3_h;
            self.q_resid_m3_h += h.q_resid_m3_h.unwrap_or(0.0);
        }
    }

    /// Pas de temps du réseau à partir des réseaux du groupe qui lui sont connectés ;
    /// `theta_ext_c` et `b_therm` servent aux pertes hors volume chauffé (éq. 924 et 932).
    pub fn pas(
        &self,
        aval: &[ApportReseauGroupe],
        theta_ext_c: f64,
        b_therm: f64,
    ) -> SortiesDistributionIntergroupes {
        // Éq. (888) et (889).
        let relance = aval.iter().any(|a| a.sorties.relance);
        let q_ch: f64 = aval.iter().map(|a| a.sorties.q_sys_ch_wh).sum();
        let q_fr: f64 = aval.iter().map(|a| a.sorties.q_sys_fr_wh).sum();
        match &self.reseau {
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
            ReseauIntergroupes::HydrauliqueCollectif(h)
            | ReseauIntergroupes::HydrauliqueIndividuel(h) => {
                let collectif = matches!(self.reseau, ReseauIntergroupes::HydrauliqueCollectif(_));
                let (theta_amb, q_ds) = match self.fonction {
                    FonctionReseau::Chauffage => (THETA_AMB_CH_C, q_ch),
                    FonctionReseau::Refroidissement => (THETA_AMB_FR_C, q_fr),
                };
                // Éq. (904) à (907) et (913) à (916), lecture Q44 point 1 : départ le plus chaud
                // (le plus froid en refroidissement) des réseaux du groupe qui débitent ;
                // `Modpertes` le plus grand de ces réseaux pour un réseau collectif, nul pour des
                // réseaux individuels.
                let fonct = aval.iter().any(|a| a.sorties.fonct);
                let (theta_dep, q_tot, q_eff, mod_pertes) = if fonct {
                    let deps = aval
                        .iter()
                        .filter(|a| a.sorties.fonct)
                        .map(|a| a.sorties.theta_dep_c);
                    let theta_dep = match self.fonction {
                        FonctionReseau::Chauffage => deps.reduce(f64::max),
                        FonctionReseau::Refroidissement => deps.reduce(f64::min),
                    }
                    .unwrap_or(theta_amb);
                    let q_tot: f64 = aval.iter().map(|a| a.sorties.q_eff_m3_h).sum();
                    let m = if collectif {
                        aval.iter()
                            .map(|a| a.sorties.mod_pertes)
                            .fold(0.0, f64::max)
                    } else {
                        0.0
                    };
                    (theta_dep, q_tot, q_tot.max(self.q_resid_m3_h), m)
                } else {
                    (theta_amb, 0.0, 0.0, 0.0)
                };
                // Éq. (918).
                let theta_ret = if fonct && q_eff > 0.0 {
                    (aval
                        .iter()
                        .map(|a| a.sorties.q_eff_m3_h * a.sorties.theta_ret_c)
                        .sum::<f64>()
                        + (self.q_resid_m3_h - q_tot).max(0.0) * theta_dep)
                        / q_eff
                } else {
                    theta_dep
                };
                // Éq. (921) à (925) et (929) à (933).
                let theta_moy = (theta_dep + theta_ret) / 2.0;
                let pertes_vc = mod_pertes
                    * h.u_vc_w_mk
                    * h.l_vc_m
                    * ecart(self.fonction, theta_moy - theta_amb);
                let theta_hvc = b_therm * theta_ext_c + (1.0 - b_therm) * theta_amb;
                let pertes_hvc = mod_pertes
                    * h.u_hvc_w_mk
                    * h.l_hvc_m
                    * ecart(self.fonction, theta_moy - theta_hvc);
                // Éq. (908), (917), (926), (927), (934) et (935).
                let w_aux = mod_circ(h.circulateur, mod_pertes, q_eff, self.q_nom_m3_h) * h.p_aux_w;
                // Éq. (928) et (936).
                let q = q_ds + pertes_vc + pertes_hvc;
                let (ch, fr) = match self.fonction {
                    FonctionReseau::Chauffage => (q, 0.0),
                    FonctionReseau::Refroidissement => (0.0, q),
                };
                SortiesDistributionIntergroupes {
                    q_sys_ch_wh: ch,
                    q_sys_fr_wh: fr,
                    theta_moy_c: theta_moy,
                    theta_dep_c: theta_dep,
                    theta_ret_c: theta_ret,
                    relance,
                    w_aux_wh: w_aux,
                    phi_aux_vc_wh: P_CIRC_VC_INTERGROUPES * w_aux,
                    phi_pertes_vc_wh: pertes_vc,
                    phi_pertes_hvc_wh: pertes_hvc,
                    theta_i_aval_eq_c: aval.iter().map(|a| a.rat_surf * a.theta_i_moy_c).sum(),
                }
            }
        }
    }
}
