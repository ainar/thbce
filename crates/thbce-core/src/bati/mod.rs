// SPDX-License-Identifier: GPL-3.0-or-later
//! Bâti (Th-BCE 2020 §5).
//!
//! - [`baie`] : fiches `C_BAT_Baie vitrée` (§5.10) et `C_BAT_ensemble_baies` (§5.14) ;
//! - [`paroi_opaque`] : fiches `C_BAT_paroi opaque` (§5.17) et `C_BAT_ensemble_parois opaques`
//!   (§5.16), assemblées selon le §5.15 ;
//! - [`pont_thermique`] : fiches `C_BAT_pont thermique` (§5.19) et
//!   `C_BAT_ensemble_ponts_thermiques` (§5.20), assemblées selon le §5.18.
//!
//! Les espaces tampons (lot 4b du plan, §5.2 à 5.4) ne sont pas encore pris en charge : les
//! coefficients `btherm`, `bsolaire` et `bvoute` valent 1.

pub mod baie;
pub mod paroi_opaque;
pub mod pont_thermique;

use crate::conventions::ModeCalcul;

/// « he » : coefficient d'échange global (convectif et radiatif) extérieur, en W/(m².K).
///
/// Th-BCE 2020 : constante « he = 25,0 W/m²K » du tableau des constantes de la fiche
/// `C_BAT_paroi opaque` (§5.17.2, p. 290), employée dans l'éq. (297), et du tableau des
/// constantes de la fiche `C_BAT_pont thermique` (§5.19.2, p. 300), employée dans l'éq. (306).
/// Le texte donne donc la valeur pour les deux fiches.
pub const HE_W_M2K: f64 = 25.0;

/// Coefficient d'interpolation `fce(h)` entre les facteurs solaires en conditions « c » et « e ».
///
/// Th-BCE 2020 §5.17.3, éq. (290) et (291), et §5.19.3, éq. (301) et (302) : `fce = 1` en
/// calculs Th-B et Th-C ; `fce = 1 − isconfort_adaptatif(j)` en calcul Th-D. Le facteur solaire
/// vaut ensuite `fce·Sf-c + (1 − fce)·Sf-e` (éq. 292 et 303).
///
/// Coquille du texte : la nomenclature dit l'inverse (« 0 : conditions c ; 1 : conditions e »,
/// soit 1 en période de confort d'été). Les équations sont retenues (décision Q29).
pub fn fce(mode: ModeCalcul, is_conf_adapt: bool) -> f64 {
    match mode {
        ModeCalcul::ThB | ModeCalcul::ThC => 1.0,
        ModeCalcul::ThD => {
            if is_conf_adapt {
                0.0
            } else {
                1.0
            }
        }
    }
}

/// Flux transmis à un groupe par un ensemble de composants opaques, à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FluxOpaques {
    /// « Φsh_s » : flux dû au rayonnement solaire absorbé, en W.
    pub phi_sh_s_w: f64,
    /// « Φsh_vc » : flux dû au rayonnement froid de la voûte céleste, en W.
    pub phi_sh_vc_w: f64,
}

/// Conditions extérieures d'un pas de temps, communes à tous les composants d'un groupe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionsExterieures {
    /// Position du soleil.
    pub soleil: crate::climat::environnement_proche::Soleil,
    /// « IDn » : rayonnement direct normal, en W/m².
    pub idn_w_m2: f64,
    /// « Idi » : rayonnement diffus horizontal, en W/m².
    pub idi_w_m2: f64,
    /// « EDn » : éclairement direct normal, en lux.
    pub edn_lux: f64,
    /// « Edi » : éclairement diffus horizontal, en lux.
    pub edi_lux: f64,
    /// « Te » : température extérieure, en °C.
    pub te_c: f64,
    /// « Teciel » : température du ciel, en °C.
    pub teciel_c: f64,
    /// Pas de temps (itération à partir de 0).
    pub h: usize,
    /// Mode de calcul.
    pub mode: ModeCalcul,
    /// « isconfort_adaptatif(j) » : vrai en période de confort adaptatif.
    pub is_conf_adapt: bool,
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn coefficient_d_interpolation() {
        // Éq. (290) : conditions « c » en Th-B et Th-C, quelle que soit la période.
        assert_eq!(fce(ModeCalcul::ThB, true), 1.0);
        assert_eq!(fce(ModeCalcul::ThC, false), 1.0);
        // Éq. (291) : en Th-D, conditions « e » pendant la période de confort adaptatif.
        assert_eq!(fce(ModeCalcul::ThD, true), 0.0);
        assert_eq!(fce(ModeCalcul::ThD, false), 1.0);
    }
}
