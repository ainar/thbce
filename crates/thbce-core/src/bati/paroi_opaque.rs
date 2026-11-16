// SPDX-License-Identifier: GPL-3.0-or-later
//! Parois opaques : fiches `C_BAT_paroi opaque` (Th-BCE 2020 §5.17) et
//! `C_BAT_ensemble_parois opaques` (§5.16), assemblées selon le §5.15.

use crate::climat::environnement_proche::{
    flux_atteignant_la_paroi, qer_w_m2, MasqueResolu, SurfaceExposee, T_SIMUL_DEB_VEG,
    T_SIMUL_FIN_VEG,
};
use crate::entree::{Catalogue, ParoiOpaque, ParoiType, PositionParoi};

use super::{fce, ConditionsExterieures, FluxOpaques, HE_W_M2K};

/// « β1 » : inclinaison en dessous de laquelle la paroi est horizontale à flux ascendant, en °.
pub const BETA_1_DEG: f64 = 30.0;
/// « β2 » : inclinaison au-dessus de laquelle la paroi est verticale, en °.
pub const BETA_2_DEG: f64 = 60.0;
/// « β3 » : inclinaison en dessous de laquelle la paroi est verticale, en °.
pub const BETA_3_DEG: f64 = 120.0;
/// « β4 » : inclinaison au-dessus de laquelle la paroi est horizontale à flux descendant, en °.
pub const BETA_4_DEG: f64 = 150.0;

/// Résistances superficielles cumulées des trois positions, en m².K/W (§5.17.3).
const RS_VERTICALE: f64 = 0.17;
const RS_HORIZONTALE_ASCENDANT: f64 = 0.14;
const RS_HORIZONTALE_DESCENDANT: f64 = 0.21;

/// « Uk_β » : coefficient de transmission thermique de la paroi dans sa position réelle.
///
/// Th-BCE 2020 §5.17.3, « Calcul du coefficient de transmission thermique global ». La
/// résistance propre de la paroi se déduit de `Uk` et de sa position de référence :
/// `Rth = 1/Uk − 0,17` (verticale), `− 0,14` (horizontale, flux ascendant) ou `− 0,21`
/// (horizontale, flux descendant). On en tire `Uvert = 1/(Rth + 0,17)`,
/// `Uhoriz,asc = 1/(Rth + 0,14)` et `Uhoriz,des = 1/(Rth + 0,21)`, puis, selon l'inclinaison β
/// (éq. 294) : `Uhoriz,asc` jusqu'à 30° ; interpolation linéaire de 30 à 60° ; `Uvert` de 60 à
/// 120° ; interpolation linéaire de 120 à 150° ([`u_entre_beta_3_et_beta_4`], décision Q29) ;
/// `Uhoriz,des` au-delà.
pub fn u_k_beta(u_k_w_m2k: f64, incl_k: PositionParoi, beta_deg: f64) -> f64 {
    let rs_reference = match incl_k {
        PositionParoi::Verticale => RS_VERTICALE,
        PositionParoi::HorizontaleFluxAscendant => RS_HORIZONTALE_ASCENDANT,
        PositionParoi::HorizontaleFluxDescendant => RS_HORIZONTALE_DESCENDANT,
    };
    let rth = 1.0 / u_k_w_m2k - rs_reference;
    let u_vert = 1.0 / (rth + RS_VERTICALE);
    let u_horiz_asc = 1.0 / (rth + RS_HORIZONTALE_ASCENDANT);
    let u_horiz_des = 1.0 / (rth + RS_HORIZONTALE_DESCENDANT);
    if beta_deg <= BETA_1_DEG {
        u_horiz_asc
    } else if beta_deg < BETA_2_DEG {
        (u_vert - u_horiz_asc) / (BETA_2_DEG - BETA_1_DEG) * beta_deg
            + (BETA_2_DEG * u_horiz_asc - BETA_1_DEG * u_vert) / (BETA_2_DEG - BETA_1_DEG)
    } else if beta_deg <= BETA_3_DEG {
        u_vert
    } else if beta_deg < BETA_4_DEG {
        u_entre_beta_3_et_beta_4(u_vert, u_horiz_des, beta_deg)
    } else {
        u_horiz_des
    }
}

/// Branche `β3 < β < β4` (120 à 150°) de l'éq. (294) : interpolation linéaire entre la position
/// verticale et la position horizontale à flux descendant.
///
/// Formule imprimée (Th-BCE 2020 §5.17.3, p. 292, relue sur l'image de la page) :
///
/// ```text
/// Uk_β = (Uvert,k − Uhoriz,des,k)/(β4 − β3) · β + (β4·Uhoriz,des,k − β3·Uvert,k)/(β4 − β3)
/// ```
///
/// Elle vaut `Uhoriz,des` à β3 = 120° et `Uvert` à β4 = 150° : elle saute aux deux bornes, à
/// l'inverse de la branche de 30 à 60°, qui raccorde `Uhoriz,asc` à `Uvert`, et de la figure 64.
///
/// Lecture retenue (décision Q29, à confirmer par un cas de référence) : la même interpolation
/// avec `Uvert` et `Uhoriz,des` échangés, continue aux deux bornes :
///
/// ```text
/// Uk_β = (Uhoriz,des,k − Uvert,k)/(β4 − β3) · β + (β4·Uvert,k − β3·Uhoriz,des,k)/(β4 − β3)
/// ```
///
/// Ce choix est isolé ici : pour revenir à la formule imprimée, il suffit d'échanger les deux
/// premiers arguments.
fn u_entre_beta_3_et_beta_4(u_vert: f64, u_horiz_des: f64, beta_deg: f64) -> f64 {
    (u_horiz_des - u_vert) / (BETA_4_DEG - BETA_3_DEG) * beta_deg
        + (BETA_4_DEG * u_vert - BETA_3_DEG * u_horiz_des) / (BETA_4_DEG - BETA_3_DEG)
}

/// Classe d'inclinaison d'une paroi pour les cumuls de surface (§5.16.3, éq. 287).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inclinaison {
    /// `60° ≤ β ≤ 120°` : paroi verticale.
    Verticale,
    /// `β < 60°` : paroi horizontale tournée vers le haut.
    VersLeHaut,
    /// Sinon : paroi horizontale tournée vers le bas.
    VersLeBas,
}

impl Inclinaison {
    /// Classe d'une inclinaison en degrés (éq. 287).
    pub fn de(beta_deg: f64) -> Self {
        if (BETA_2_DEG..=BETA_3_DEG).contains(&beta_deg) {
            Self::Verticale
        } else if beta_deg < BETA_2_DEG {
            Self::VersLeHaut
        } else {
            Self::VersLeBas
        }
    }
}

/// Paroi opaque prête pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct ParoiResolue {
    /// « Ak » : surface, en m².
    pub a_k_m2: f64,
    /// « Uk_β », en W/(m².K).
    pub u_k_beta_w_m2k: f64,
    /// « Sf-c,k » et « Sf-e,k ».
    pub sf_c: f64,
    /// Voir `sf_c`.
    pub sf_e: f64,
    /// « Sf-c,v,k » et « Sf-e,v,k » d'une toiture végétalisée.
    pub sf_vegetalise: Option<(f64, f64)>,
    /// Classe d'inclinaison.
    pub inclinaison: Inclinaison,
    /// Orientation et dimensions.
    pub surface: SurfaceExposee,
    /// Masques.
    pub masques: Vec<MasqueResolu>,
}

/// Résout les masques d'une liste d'identifiants. La validation garantit leur présence.
pub(crate) fn resoudre_masques(ids: &[String], catalogue: &Catalogue) -> Vec<MasqueResolu> {
    ids.iter()
        .filter_map(|id| catalogue.masques.iter().find(|m| &m.id == id))
        .map(|m| MasqueResolu::depuis_masque(&m.masque))
        .collect()
}

impl ParoiResolue {
    /// Résout une paroi à partir de son type et de son exposition.
    pub fn nouvelle(paroi: &ParoiOpaque, type_paroi: &ParoiType, catalogue: &Catalogue) -> Self {
        let beta_deg = paroi.exposition.inclinaison_deg;
        Self {
            a_k_m2: paroi.a_k_m2,
            u_k_beta_w_m2k: u_k_beta(type_paroi.u_k_w_m2k, type_paroi.incl_k, beta_deg),
            sf_c: type_paroi.sf_c_k,
            sf_e: type_paroi.sf_e_k,
            sf_vegetalise: type_paroi
                .vegetalisation
                .as_ref()
                .map(|v| (v.sf_c_v_k, v.sf_e_v_k)),
            inclinaison: Inclinaison::de(beta_deg),
            surface: SurfaceExposee::depuis_exposition(&paroi.exposition),
            masques: resoudre_masques(&paroi.exposition.masques, catalogue),
        }
    }

    /// « HTH,k » : coefficient de transmission thermique global, en W/K.
    ///
    /// Th-BCE 2020 §5.17.3, éq. (295) et (296) : `HTH,k = Ak·btherm,k·Uk_β`, avec `btherm = 1`
    /// en l'absence d'espace tampon.
    pub fn h_th_w_k(&self) -> f64 {
        self.a_k_m2 * self.u_k_beta_w_m2k
    }

    /// « Sf,k(h) » : facteur solaire au pas de temps.
    ///
    /// Th-BCE 2020 §5.17.3, éq. (292) : `Sf,k = fce·Sf-c,k + (1 − fce)·Sf-e,k`. Éq. (293) : pour
    /// une paroi végétalisée, hors période de repos végétatif (`h > Tsimul_deb_veg` et
    /// `h < Tsimul_fin_veg`), les facteurs `Sf-c,v,k` et `Sf-e,v,k` les remplacent.
    pub fn sf(&self, fce: f64, h: usize) -> f64 {
        let (sf_c, sf_e) = match self.sf_vegetalise {
            Some(vegetalise) if h > T_SIMUL_DEB_VEG && h < T_SIMUL_FIN_VEG => vegetalise,
            _ => (self.sf_c, self.sf_e),
        };
        fce * sf_c + (1.0 - fce) * sf_e
    }

    /// Flux transmis au groupe par la paroi.
    ///
    /// Th-BCE 2020 §5.17.3 : éq. (288) `Isr,k = Drp* + Dfp* + Rrp*` ; éq. (289)
    /// `Φsh_s,k = Ak·Sf,k·Isr,k·bsolaire,k` ; éq. (297) `Φsh_vc,k = Ak·bvoute,k·Qer*·Uk_β/he`.
    /// Sans espace tampon, `bsolaire = 1` et `bvoute = 1`.
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
        let sf = self.sf(fce(c.mode, c.is_conf_adapt), c.h);
        let qer = qer_w_m2(&self.surface, c.teciel_c, c.te_c);
        FluxOpaques {
            phi_sh_s_w: self.a_k_m2 * sf * incident.isr_w_m2(),
            phi_sh_vc_w: self.a_k_m2 * qer * self.u_k_beta_w_m2k / HE_W_M2K,
        }
    }
}

/// Grandeurs fixes de l'ensemble des parois opaques d'un groupe (§5.16.3).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EnsembleParois {
    /// « HTh » : coefficient de transmission thermique global, en W/K (éq. 283).
    pub h_th_w_k: f64,
    /// « Aop » : surface totale, en m² (éq. 286).
    pub a_op_m2: f64,
    /// « Aopv » : surface des parois verticales, en m² (éq. 287).
    pub a_opv_m2: f64,
    /// « Aophh » : surface des parois horizontales tournées vers le haut, en m².
    pub a_ophh_m2: f64,
    /// « Aophb » : surface des parois horizontales tournées vers le bas, en m².
    pub a_ophb_m2: f64,
    /// « HTh_opv » : coefficient des parois verticales, en W/K.
    pub h_th_opv_w_k: f64,
    /// « HTh_ophh » : coefficient des parois tournées vers le haut, en W/K.
    pub h_th_ophh_w_k: f64,
    /// « HTh_ophb » : coefficient des parois tournées vers le bas, en W/K.
    pub h_th_ophb_w_k: f64,
}

impl EnsembleParois {
    /// Cumule les parois d'un groupe (§5.16.3, éq. 283, 286 et 287).
    pub fn de(parois: &[ParoiResolue]) -> Self {
        let mut e = Self::default();
        for p in parois {
            let h = p.h_th_w_k();
            e.h_th_w_k += h;
            e.a_op_m2 += p.a_k_m2;
            let (surface, coefficient) = match p.inclinaison {
                Inclinaison::Verticale => (&mut e.a_opv_m2, &mut e.h_th_opv_w_k),
                Inclinaison::VersLeHaut => (&mut e.a_ophh_m2, &mut e.h_th_ophh_w_k),
                Inclinaison::VersLeBas => (&mut e.a_ophb_m2, &mut e.h_th_ophb_w_k),
            };
            *surface += p.a_k_m2;
            *coefficient += h;
        }
        e
    }
}

/// Flux transmis au groupe par l'ensemble de ses parois opaques (§5.16.3, éq. 284 et 285).
pub fn flux_ensemble(parois: &[ParoiResolue], c: &ConditionsExterieures) -> FluxOpaques {
    parois.iter().fold(FluxOpaques::default(), |acc, p| {
        let f = p.flux(c);
        FluxOpaques {
            phi_sh_s_w: acc.phi_sh_s_w + f.phi_sh_s_w,
            phi_sh_vc_w: acc.phi_sh_vc_w + f.phi_sh_vc_w,
        }
    })
}
