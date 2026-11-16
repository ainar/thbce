// SPDX-License-Identifier: GPL-3.0-or-later
//! Ponts thermiques : fiches `C_BAT_pont thermique` (Th-BCE 2020 §5.19) et
//! `C_BAT_ensemble_ponts_thermiques` (§5.20), assemblées selon le §5.18.

use crate::climat::environnement_proche::{
    flux_atteignant_la_paroi, qer_w_m2, MasqueResolu, SurfaceExposee,
};
use crate::entree::{Catalogue, Exposition, PontThermique};

use super::espace_tampon::espace_tampon::CoefficientsEt;
use super::paroi_opaque::resoudre_masques;
use super::{fce, ConditionsExterieures, FluxOpaques, HE_W_M2K};

/// Largeur conventionnelle, en m, de la paroi verticale qui représente un pont thermique pour
/// les masques proches (§3.2.3, « Cas particulier des ponts thermiques » : 30 cm).
pub const LARGEUR_MASQUES_PROCHES_M: f64 = 0.3;

/// Azimuts, en degrés, des quatre ponts thermiques de la description simplifiée (§5.18.3,
/// éq. 298) : `(α, β) = (0°, 90°), (90°, 90°), (180°, 90°), (270°, 90°)`.
pub const AZIMUTS_SIMPLIFIES_DEG: [f64; 4] = [0.0, 90.0, 180.0, 270.0];

/// Tronçon de pont thermique prêt pour le calcul : une orientation, une longueur.
#[derive(Debug, Clone, PartialEq)]
pub struct PontResolu {
    /// « ll » : longueur du tronçon, en m.
    pub l_l_m: f64,
    /// « Ψl » : coefficient de transmission thermique linéique, en W/(m.K).
    pub psi_l_w_mk: f64,
    /// « Sf-c,l » et « Sf-e,l », en m⁻¹.
    pub sf_c: f64,
    /// Voir `sf_c`.
    pub sf_e: f64,
    /// Orientation et dimensions.
    pub surface: SurfaceExposee,
    /// Masques.
    pub masques: Vec<MasqueResolu>,
    /// Indice, dans le bâtiment, de l'espace tampon sur lequel donne le pont thermique.
    pub espace_tampon: Option<usize>,
}

impl PontResolu {
    /// Résout un pont thermique en un tronçon (description détaillée) ou quatre (description
    /// simplifiée, §5.18.3).
    pub fn resoudre(pont: &PontThermique, catalogue: &Catalogue, sortie: &mut Vec<Self>) {
        let troncon = |l_l_m: f64, exposition: &Exposition, masques: Vec<MasqueResolu>| Self {
            l_l_m,
            psi_l_w_mk: pont.psi_l_w_mk,
            sf_c: pont.sf_c_l,
            sf_e: pont.sf_e_l,
            surface: SurfaceExposee::depuis_exposition(exposition),
            masques,
            espace_tampon: None,
        };
        match &pont.exposition {
            // Description détaillée. Pour les masques proches, le pont est une paroi verticale de
            // 30 cm de large (§3.2.3).
            Some(e) => {
                let e = Exposition {
                    lpb_m: Some(LARGEUR_MASQUES_PROCHES_M),
                    ..e.clone()
                };
                sortie.push(troncon(
                    pont.l_l_m,
                    &e,
                    resoudre_masques(&e.masques, catalogue),
                ));
            }
            // Description simplifiée : quatre ponts verticaux d'un quart de la longueur, sans
            // masque (éq. 298).
            None => {
                for azimut_deg in AZIMUTS_SIMPLIFIES_DEG {
                    let e = Exposition {
                        azimut_deg,
                        inclinaison_deg: 90.0,
                        lpb_m: None,
                        hpb_m: None,
                        masques: vec![],
                    };
                    sortie.push(troncon(pont.l_l_m / 4.0, &e, Vec::new()));
                }
            }
        }
    }

    /// « HTh,l » : coefficient de transmission thermique global, en W/K.
    ///
    /// Th-BCE 2020 §5.19.3, éq. (304) et (305) : `HTh,l = ll·btherm,l·Ψl`, avec `btherm = 1` en
    /// l'absence d'espace tampon.
    pub fn h_th_w_k(&self) -> f64 {
        self.l_l_m * self.psi_l_w_mk
    }

    /// Flux transmis au groupe par le pont thermique.
    ///
    /// Th-BCE 2020 §5.19.3 : éq. (299) `Isr,l = Drp* + Dfp* + Rrp*` ; éq. (303)
    /// `Sf,l = fce·Sf-c,l + (1 − fce)·Sf-e,l` ; éq. (300) `Φsh_s,l = ll·Sf,l·Isr,l·bsolaire,l` ;
    /// éq. (306) `Φsh_vc,l = ll·bvoute,l·Qer*·Ψl/he`.
    pub fn flux(&self, c: &ConditionsExterieures) -> FluxOpaques {
        let incident = flux_atteignant_la_paroi(
            &self.surface,
            &self.masques,
            &c.soleil,
            c.idn_w_m2,
            c.idi_w_m2,
            0.0,
            0.0,
            c.h,
        );
        let f = fce(c.mode, c.is_conf_adapt);
        let sf = f * self.sf_c + (1.0 - f) * self.sf_e;
        let qer = qer_w_m2(&self.surface, c.teciel_c, c.te_c);
        FluxOpaques {
            phi_sh_s_w: self.l_l_m * sf * incident.isr_w_m2(),
            // Éq. (306) : Φsh_vc,l = ll · bvoute,l · (Qer*l · Ψl / he), avec he = 25 W/(m².K),
            // constante de la fiche (§5.19.2, p. 300), la même que pour les parois opaques
            // (§5.17.2, éq. 297).
            phi_sh_vc_w: self.l_l_m * qer * self.psi_l_w_mk / HE_W_M2K,
        }
    }
}

impl PontResolu {
    /// « HTh,l(h) » avec l'espace tampon : `ll·btherm,l(h)·Ψl` (§5.19.3, éq. 304).
    pub fn h_th_reduit_w_k(&self, et: Option<&CoefficientsEt>) -> f64 {
        self.h_th_w_k() * et.map_or(1.0, |e| e.b_therm)
    }

    /// Flux transmis au groupe par le pont thermique, avec l'espace tampon.
    ///
    /// Th-BCE 2020 §5.19.3 : avec un espace tampon, `bvoute,l = 0` ; éq. (300) : le flux
    /// solaire est multiplié par `bsolaire,l(h)`.
    pub fn flux_reduit(
        &self,
        c: &ConditionsExterieures,
        et: Option<&CoefficientsEt>,
    ) -> FluxOpaques {
        let f = self.flux(c);
        match et {
            None => f,
            Some(e) => FluxOpaques {
                phi_sh_s_w: f.phi_sh_s_w * e.b_solaire,
                phi_sh_vc_w: 0.0,
            },
        }
    }
}

/// Grandeurs fixes de l'ensemble des ponts thermiques d'un groupe (§5.20.3).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EnsemblePonts {
    /// « HTh » : coefficient de transmission thermique global, en W/K (éq. 307).
    pub h_th_w_k: f64,
    /// « Ll » : longueur totale, en m (éq. 310).
    pub l_m: f64,
}

impl EnsemblePonts {
    /// Cumule les ponts thermiques d'un groupe (éq. 307 et 310).
    pub fn de(ponts: &[PontResolu]) -> Self {
        ponts.iter().fold(Self::default(), |e, p| Self {
            h_th_w_k: e.h_th_w_k + p.h_th_w_k(),
            l_m: e.l_m + p.l_l_m,
        })
    }
}

/// Flux transmis au groupe par l'ensemble de ses ponts thermiques (§5.20.3, éq. 308 et 309).
pub fn flux_ensemble(ponts: &[PontResolu], c: &ConditionsExterieures) -> FluxOpaques {
    ponts.iter().fold(FluxOpaques::default(), |acc, p| {
        let f = p.flux(c);
        FluxOpaques {
            phi_sh_s_w: acc.phi_sh_s_w + f.phi_sh_s_w,
            phi_sh_vc_w: acc.phi_sh_vc_w + f.phi_sh_vc_w,
        }
    })
}
