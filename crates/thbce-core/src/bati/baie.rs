// SPDX-License-Identifier: GPL-3.0-or-later
//! Baies vitrées : fiches `C_BAT_Baie vitrée` (Th-BCE 2020 §5.10) et `C_BAT_ensemble_baies`
//! (§5.14).
//!
//! La position des protections mobiles ([`Protections`]) est une entrée de ces fiches : elle
//! est calculée par la fiche `C_BAT_Gestion_protections_mobiles` (§5.9). Les stores vénitiens
//! (§5.10.3.6.2) ne sont pas encore pris en charge ; les facteurs de la partie protégée sont
//! déjà distingués par composante incidente ([`FacteursProteges`]) pour les recevoir.
//!
//! En l'absence d'espace tampon (§5.2 à 5.4, non pris en charge), `btherm`, `bsolaire` et
//! `becl` valent 1 et `IdET = 0`.

use crate::climat::environnement_proche::{
    flux_atteignant_la_paroi, qer_w_m2, FluxParoi, MasqueResolu, SurfaceExposee,
};
use crate::conventions::ModeCalcul;
use crate::entree::{Baie, BaieType, Catalogue, FacteursBaie};

use super::paroi_opaque::resoudre_masques;
use super::{ConditionsExterieures, HE_W_M2K};

/// « β1 » : inclinaison en dessous de laquelle la baie est considérée horizontale, en degrés
/// (§5.10.2, constantes).
pub const BETA_1_DEG: f64 = 30.0;
/// « β2 » : inclinaison à partir de laquelle la baie est considérée verticale, en degrés
/// (§5.10.2, constantes).
pub const BETA_2_DEG: f64 = 60.0;
/// « flf,b » : facteur de perte solaire de la baie, supposé nul (§5.10.3.7).
pub const F_LF: f64 = 0.0;
/// « Tli2,b » : transmission lumineuse globale de la seconde protection mobile (§5.10.3.8,
/// éq. 234).
pub const TLI_2: f64 = 0.1;
/// « Tlid2,b » : transmission du flux direct sous forme diffuse par la seconde protection mobile
/// (éq. 234). Le direct n'est donc transmis que sous forme diffuse.
pub const TLID_2: f64 = 0.1;

/// Coefficient de transmission thermique d'une baie selon son inclinaison β, en W/(m².K).
///
/// Th-BCE 2020 §5.10.3.5, éq. (207) et (208) : `Uhoriz` si `0° ≤ β < β1` ; interpolation
/// linéaire `(Uvert − Uhoriz)/(β2 − β1)·β + (β2·Uhoriz − β1·Uvert)/(β2 − β1)` si `β1 ≤ β < β2` ;
/// `Uvert` si `β2 ≤ β ≤ 90°`.
///
/// Renvoie `None` si un coefficient nécessaire à cette inclinaison n'est pas saisi.
pub fn u_baie(u_vert: Option<f64>, u_horiz: Option<f64>, beta_deg: f64) -> Option<f64> {
    if beta_deg < BETA_1_DEG {
        u_horiz
    } else if beta_deg < BETA_2_DEG {
        let (v, h) = (u_vert?, u_horiz?);
        Some(
            (v - h) / (BETA_2_DEG - BETA_1_DEG) * beta_deg
                + (BETA_2_DEG * h - BETA_1_DEG * v) / (BETA_2_DEG - BETA_1_DEG),
        )
    } else {
        u_vert
    }
}

/// Mode de gestion de la première protection mobile, au sens du calcul de `Hges,b` (§5.10.3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestionProtection {
    /// `type-GPM = 2` ou `3`, ou `type-GPM-sv = 2` : éq. (210).
    Manuelle,
    /// `type-GPM = 1`, ou `type-GPM-sv = 1` ou `3` : éq. (211).
    Automatique,
}

/// Position des protections mobiles d'une baie à un pas de temps (sorties de la fiche §5.9).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Protections {
    /// « partGPM » : part de la surface des baies appartenant aux locaux réellement occupés.
    pub part_gpm: f64,
    /// « Rprot1partGPM,b » : position de la première protection dans la part `partGPM`
    /// (0 ouverte, 1 fermée).
    pub r_prot1_part_gpm: f64,
    /// « Rprot11-partGPM,b » : position de la première protection dans la part `1 − partGPM`.
    pub r_prot1_hors_part_gpm: f64,
    /// « Rprot2,b » : position de la seconde protection mobile ; 0 s'il n'y en a pas.
    pub r_prot2: f64,
    /// Mode de gestion de la première protection.
    pub gestion: GestionProtection,
}

impl Protections {
    /// Baie sans protection mobile, ou protections toutes ouvertes (éq. 209, `type-PM = 0`).
    pub const OUVERTES: Self = Self {
        part_gpm: 0.0,
        r_prot1_part_gpm: 0.0,
        r_prot1_hors_part_gpm: 0.0,
        r_prot2: 0.0,
        gestion: GestionProtection::Manuelle,
    };

    /// « Rprot1,b » : ratio de fermeture de la baie par la première protection mobile.
    ///
    /// Th-BCE 2020 §5.10.3.5, éq. (209) :
    /// `Rprot1,b = partGPM·Rprot1partGPM,b + (1 − partGPM)·Rprot11-partGPM,b`. C'est le
    /// « Rprot,b » des éq. (210) et (219) à (233) : les éq. (235) à (237) s'y ramènent quand
    /// `Rprot2,b = 0`.
    pub fn r_prot(&self) -> f64 {
        self.part_gpm * self.r_prot1_part_gpm + (1.0 - self.part_gpm) * self.r_prot1_hors_part_gpm
    }

    /// Part de la baie derrière la seule seconde protection mobile (éq. 235 à 237) :
    /// `partGPM·(1 − Rprot1partGPM)·Rprot2,b`.
    fn part_seconde_protection(&self) -> f64 {
        self.part_gpm * (1.0 - self.r_prot1_part_gpm) * self.r_prot2
    }

    /// Part de la baie sans aucune protection en place (éq. 235 à 237) :
    /// `(1 − partGPM)·(1 − Rprot11-partGPM) + partGPM·(1 − Rprot1partGPM)·(1 − Rprot2,b)`.
    fn part_nue(&self) -> f64 {
        (1.0 - self.part_gpm) * (1.0 - self.r_prot1_hors_part_gpm)
            + self.part_gpm * (1.0 - self.r_prot1_part_gpm) * (1.0 - self.r_prot2)
    }
}

/// Facteurs solaires `Sw1`, `Sw2`, `Sw3` d'un état de la baie, pour un mode de calcul.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FacteursSolaires {
    /// Composante courte longueur d'onde.
    pub sw1: f64,
    /// Composante grande longueur d'onde et convective.
    pub sw2: f64,
    /// Composante liée à la lame d'air intérieure ventilée.
    pub sw3: f64,
}

impl FacteursSolaires {
    /// Th-BCE 2020 §5.10.3.7, éq. (223) et (224) : facteurs « -c » en Th-B et Th-C, « -e » en
    /// Th-D.
    fn de(f: &FacteursBaie, mode: ModeCalcul) -> Self {
        match mode {
            ModeCalcul::ThB | ModeCalcul::ThC => Self {
                sw1: f.sw1_c,
                sw2: f.sw2_c,
                sw3: f.sw3_c,
            },
            ModeCalcul::ThD => Self {
                sw1: f.sw1_e,
                sw2: f.sw2_e,
                sw3: f.sw3_e,
            },
        }
    }
}

/// Facteurs de la partie de la baie avec la première protection mobile en place, par composante
/// du flux incident (§5.10.3.6, tableaux 36 à 38).
///
/// Pour un volet ou un store enroulable, les trois composantes sont égales (éq. 214).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FacteursProteges {
    /// Facteurs solaires appliqués au rayonnement direct.
    pub dir: FacteursSolaires,
    /// Facteurs solaires appliqués au rayonnement diffus.
    pub dif: FacteursSolaires,
    /// Facteurs solaires appliqués au rayonnement réfléchi par le sol.
    pub refl: FacteursSolaires,
    /// « Tliap_dir » : transmission lumineuse globale du flux direct.
    pub tli_dir: f64,
    /// « Tlidap_dir » : part du flux direct transmise sous forme diffuse.
    pub tlid_dir: f64,
    /// « Tldap_dif » : transmission lumineuse globale du flux diffus.
    pub tld_dif: f64,
    /// « Tldap_ref » : transmission lumineuse globale du flux réfléchi.
    pub tld_ref: f64,
    /// « Tlidap_ref » : part du flux réfléchi transmise sous forme diffuse.
    pub tlid_ref: f64,
}

impl FacteursProteges {
    /// Volet ou store enroulable : Th-BCE 2020 §5.10.3.6.3, éq. (214).
    fn uniformes(f: &FacteursBaie, mode: ModeCalcul) -> Self {
        let s = FacteursSolaires::de(f, mode);
        Self {
            dir: s,
            dif: s,
            refl: s,
            tli_dir: f.tli,
            tlid_dir: f.tlid,
            tld_dif: f.tli,
            tld_ref: f.tli,
            tlid_ref: f.tlid,
        }
    }
}

/// Type d'une baie pour l'éclairement naturel (§5.10.3.2, tableau des types).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeBaieEclairage {
    /// Type 0, « GV » : baie d'un grand volume qui ne l'éclaire pas de façon uniforme.
    GrandVolume = 0,
    /// Type 1, « GVU » : baie horizontale éclairant un grand volume de façon uniforme.
    GrandVolumeUniforme = 1,
    /// Type 2, « VN » : baie d'un volume normal.
    VolumeNormal = 2,
}

/// Baie prête pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct BaieResolue {
    /// « Ab » : surface, en m².
    pub a_b_m2: f64,
    /// « αb » : orientation, en degrés.
    pub alpha_deg: f64,
    /// « βb » : inclinaison, en degrés.
    pub beta_deg: f64,
    /// « Usp,b » : U sans protection mobile, à l'inclinaison de la baie, en W/(m².K).
    pub u_sp_w_m2k: f64,
    /// « Uap,b » : U avec la première protection mobile, à l'inclinaison de la baie. Égal à
    /// `Usp,b` si le type de baie ne le donne pas (baie sans protection mobile).
    pub u_ap_w_m2k: f64,
    /// Facteurs sans protection mobile.
    pub sans_protection: FacteursBaie,
    /// Facteurs avec la première protection mobile ; ceux de la baie nue s'il n'y en a pas.
    pub avec_protection: FacteursBaie,
    /// Type de la baie pour l'éclairement naturel.
    pub type_eclairage: TypeBaieEclairage,
    /// Orientation et dimensions.
    pub surface: SurfaceExposee,
    /// Masques.
    pub masques: Vec<MasqueResolu>,
}

/// Flux transmis au groupe par une baie, ou par l'ensemble des baies, à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FluxBaies {
    /// « Hges » : coefficient de transfert thermique global, en W/K.
    pub h_ges_w_k: f64,
    /// « Fs1 » : flux de courte longueur d'onde, en W.
    pub fs1_w: f64,
    /// « Fs2 » : flux de grande longueur d'onde et convectif, en W.
    pub fs2_w: f64,
    /// « Fs3 » : flux à travers la lame d'air intérieure ventilée, en W.
    pub fs3_w: f64,
    /// « Ftvc » : flux dû au rayonnement froid vers la voûte céleste, en W.
    pub ftvc_w: f64,
    /// « Flt1(n) » : flux lumineux transmis sous forme directe, en lumens, par type de baie.
    pub flt1_lm: [f64; 3],
    /// « Flt2(n) » : flux lumineux transmis sous forme diffuse, en lumens, par type de baie.
    pub flt2_lm: [f64; 3],
    /// « Flt3(n) » : flux lumineux transmis sous forme réfléchie, en lumens, par type de baie.
    pub flt3_lm: [f64; 3],
}

impl BaieResolue {
    /// Résout une baie à partir de son type et de son exposition. Renvoie `None` si le type ne
    /// donne pas le coefficient U nécessaire à l'inclinaison de la baie (cas écarté par la
    /// validation).
    pub fn nouvelle(baie: &Baie, type_baie: &BaieType, catalogue: &Catalogue) -> Option<Self> {
        let beta_deg = baie.exposition.inclinaison_deg;
        let u_sp = u_baie(
            type_baie.u_sp_vert_w_m2k,
            type_baie.u_sp_horiz_w_m2k,
            beta_deg,
        )?;
        let u_ap = u_baie(
            type_baie.u_ap_vert_w_m2k,
            type_baie.u_ap_horiz_w_m2k,
            beta_deg,
        );
        Some(Self {
            a_b_m2: baie.a_b_m2,
            alpha_deg: baie.exposition.azimut_deg,
            beta_deg,
            u_sp_w_m2k: u_sp,
            u_ap_w_m2k: u_ap.unwrap_or(u_sp),
            sans_protection: type_baie.sans_protection.clone(),
            avec_protection: type_baie
                .avec_protection
                .clone()
                .unwrap_or_else(|| type_baie.sans_protection.clone()),
            // §5.10.3.2 : les grands volumes n'existent que dans des usages hors du périmètre
            // actuel (industrie, établissements sportifs, commerces).
            type_eclairage: TypeBaieEclairage::VolumeNormal,
            surface: SurfaceExposee::depuis_exposition(&baie.exposition),
            masques: resoudre_masques(&baie.exposition.masques, catalogue),
        })
    }

    /// « Hges,b » : coefficient de transfert thermique global de la baie, en W/K.
    ///
    /// Th-BCE 2020 §5.10.3.5 :
    /// - gestion manuelle, éq. (210) : `Ab·btherm·((1 − Rprot,b)·Usp,b + Rprot,b·Uap,b)` ;
    /// - gestion automatique, éq. (211) :
    ///   `Ab·btherm·[partGPM·(Rprot1partGPM·Uap + (1 − Rprot1partGPM)·Usp) + (1 − partGPM)·U]`,
    ///   où `U = Uap,b` si `Rprot11-partGPM,b = 1` et `Usp,b` sinon.
    pub fn h_ges_w_k(&self, p: &Protections) -> f64 {
        let (u_sp, u_ap) = (self.u_sp_w_m2k, self.u_ap_w_m2k);
        let u = match p.gestion {
            GestionProtection::Manuelle => {
                let r = p.r_prot();
                (1.0 - r) * u_sp + r * u_ap
            }
            GestionProtection::Automatique => {
                let u_hors_part = if p.r_prot1_hors_part_gpm >= 1.0 {
                    u_ap
                } else {
                    u_sp
                };
                p.part_gpm * (p.r_prot1_part_gpm * u_ap + (1.0 - p.r_prot1_part_gpm) * u_sp)
                    + (1.0 - p.part_gpm) * u_hors_part
            }
        };
        self.a_b_m2 * u
    }

    /// Rayonnement et éclairement atteignant la baie, masques compris (§3.2.3).
    pub fn incident(&self, c: &ConditionsExterieures) -> FluxParoi {
        flux_atteignant_la_paroi(
            &self.surface,
            &self.masques,
            &c.soleil,
            c.idn_w_m2,
            c.idi_w_m2,
            c.edn_lux,
            c.edi_lux,
            c.h,
        )
    }

    /// Flux transmis au groupe par la baie.
    pub fn flux(&self, c: &ConditionsExterieures, p: &Protections) -> FluxBaies {
        let incident = self.incident(c);
        let qer = qer_w_m2(&self.surface, c.teciel_c, c.te_c);
        let ap = FacteursProteges::uniformes(&self.avec_protection, c.mode);
        self.flux_transmis(&incident, qer, p, c.mode, &ap)
    }

    /// Flux transmis au groupe, à partir des flux incidents et des facteurs de la partie
    /// protégée.
    ///
    /// Th-BCE 2020 §5.10.3.7 :
    /// - éq. (219) : `Fs1,b = Ab·bsolaire·(1 − flf)·[(1 − Rprot)·Sw1sp·Isr* + Rprot·(Sw1ap_dir·Drp*
    ///   + Sw1ap_dif·Dfp* + Sw1ap_ref·Rrp*)]` ;
    /// - éq. (220) et (221) : de même pour `Fs2,b` et `Fs3,b`, sans le facteur `(1 − flf)` ;
    /// - éq. (222) : `Ftvc,b = Ab·((1 − Rprot)·Usp/he + Rprot·Uap/he)·Qer*`.
    ///
    /// §5.10.3.8, éq. (235) à (237), qui se réduisent aux éq. (231) à (233) sans seconde
    /// protection mobile. Avec `P1 = Rprot1,b`, `P2 = partGPM·(1 − Rprot1partGPM)·Rprot2,b` et
    /// `P0` la part sans protection :
    /// - `Flt1 = Ab·becl·(P1·Tliiap_dir + P2·Tlii2 + P0·Tliisp)·Erp*` ;
    /// - `Flt2 = Ab·becl·[(P1·Tlidap_dir + P2·Tlid2 + P0·Tlidsp)·Erp*
    ///   + (P1·Tldap_dif + P2·Tli2 + P0·Tldsp)·Efp* + (P1·Tlidap_ref + P2·Tlid2 + P0·Tlidsp)·Errp*]` ;
    /// - `Flt3 = Ab·becl·(P1·Tliiap_ref + P2·Tlii2 + P0·Tliisp)·Errp*`.
    ///
    /// Les transmissions « direct en direct » se déduisent des éq. (225) à (230) :
    /// `Tliisp = Tlisp − Tlidsp`, `Tldsp = Tlisp`, `Tliiap_dir = Tliap_dir − Tlidap_dir`,
    /// `Tliiap_ref = Tldap_ref − Tlidap_ref`, `Tlii2 = Tli2 − Tlid2`.
    pub fn flux_transmis(
        &self,
        incident: &FluxParoi,
        qer_w_m2: f64,
        p: &Protections,
        mode: ModeCalcul,
        ap: &FacteursProteges,
    ) -> FluxBaies {
        let a = self.a_b_m2;
        let r = p.r_prot();
        let sp = FacteursSolaires::de(&self.sans_protection, mode);
        let (drp, dfp, rrp) = (incident.drp_w_m2, incident.dfp_w_m2, incident.rrp_w_m2);
        let isr = incident.isr_w_m2();
        let composante = |sw_sp: f64, dir: f64, dif: f64, refl: f64| {
            a * ((1.0 - r) * sw_sp * isr + r * (dir * drp + dif * dfp + refl * rrp))
        };

        // Transmissions lumineuses, éq. (225) à (230).
        let (tli_sp, tlid_sp) = (self.sans_protection.tli, self.sans_protection.tlid);
        let tlii_sp = tli_sp - tlid_sp;
        let tld_sp = tli_sp;
        let tlii_ap_dir = ap.tli_dir - ap.tlid_dir;
        let tlii_ap_ref = ap.tld_ref - ap.tlid_ref;
        let tlii_2 = TLI_2 - TLID_2;
        let (p1, p2, p0) = (r, p.part_seconde_protection(), p.part_nue());
        let (erp, efp, errp) = (incident.erp_lux, incident.efp_lux, incident.errp_lux);
        let flt1 = a * (p1 * tlii_ap_dir + p2 * tlii_2 + p0 * tlii_sp) * erp;
        let flt2 = a
            * ((p1 * ap.tlid_dir + p2 * TLID_2 + p0 * tlid_sp) * erp
                + (p1 * ap.tld_dif + p2 * TLI_2 + p0 * tld_sp) * efp
                + (p1 * ap.tlid_ref + p2 * TLID_2 + p0 * tlid_sp) * errp);
        let flt3 = a * (p1 * tlii_ap_ref + p2 * tlii_2 + p0 * tlii_sp) * errp;
        let n = self.type_eclairage as usize;
        let par_type = |v: f64| {
            let mut t = [0.0; 3];
            t[n] = v;
            t
        };

        FluxBaies {
            h_ges_w_k: self.h_ges_w_k(p),
            fs1_w: (1.0 - F_LF) * composante(sp.sw1, ap.dir.sw1, ap.dif.sw1, ap.refl.sw1),
            fs2_w: composante(sp.sw2, ap.dir.sw2, ap.dif.sw2, ap.refl.sw2),
            fs3_w: composante(sp.sw3, ap.dir.sw3, ap.dif.sw3, ap.refl.sw3),
            ftvc_w: a
                * ((1.0 - r) * self.u_sp_w_m2k / HE_W_M2K + r * self.u_ap_w_m2k / HE_W_M2K)
                * qer_w_m2,
            flt1_lm: par_type(flt1),
            flt2_lm: par_type(flt2),
            flt3_lm: par_type(flt3),
        }
    }
}

/// Somme des flux de plusieurs baies (§5.14.3, éq. 257 à 270).
pub fn somme(flux: impl IntoIterator<Item = FluxBaies>) -> FluxBaies {
    flux.into_iter().fold(FluxBaies::default(), |mut acc, f| {
        acc.h_ges_w_k += f.h_ges_w_k;
        acc.fs1_w += f.fs1_w;
        acc.fs2_w += f.fs2_w;
        acc.fs3_w += f.fs3_w;
        acc.ftvc_w += f.ftvc_w;
        for n in 0..3 {
            acc.flt1_lm[n] += f.flt1_lm[n];
            acc.flt2_lm[n] += f.flt2_lm[n];
            acc.flt3_lm[n] += f.flt3_lm[n];
        }
        acc
    })
}

/// Grandeurs fixes de l'ensemble des baies d'un groupe (§5.14.3, « Calculs divers »).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EnsembleBaies {
    /// « Abaies » : surface totale des baies, en m² (éq. 282).
    pub a_baies_m2: f64,
    /// « Abaies-v » : surface des baies verticales, `60° ≤ β ≤ 90°`, en m² (éq. 279).
    pub a_baies_v_m2: f64,
    /// « Abaies-h » : surface des baies horizontales, `β < 60°`, en m² (éq. 281).
    pub a_baies_h_m2: f64,
    /// « Partbaies_sud » : part des baies verticales orientées au sud (éq. 271).
    pub part_sud: f64,
    /// « Partbaies_nord » (éq. 273).
    pub part_nord: f64,
    /// « Partbaies_ouest » (éq. 275).
    pub part_ouest: f64,
    /// « Partbaies_est » (éq. 277).
    pub part_est: f64,
    /// « Partbaies_horiz » : part des baies d'inclinaison nulle dans la surface totale (éq. 280).
    pub part_horiz: f64,
}

impl EnsembleBaies {
    /// Cumule les baies d'un groupe.
    ///
    /// Th-BCE 2020 §5.14.3 : une baie verticale (`60° ≤ β ≤ 90°`) est au sud si
    /// `−45° ≤ α ≤ 45°` (éq. 271), au nord si `135° ≤ α ≤ 225°` (éq. 273), à l'ouest si
    /// `45° < α < 135°` (éq. 275), à l'est si `225° < α < 315°` (éq. 277). Les parts
    /// d'orientation sont rapportées à la surface des baies verticales ; `Partbaies_horiz` est la
    /// part des baies d'inclinaison nulle dans la surface totale (éq. 280).
    pub fn de(baies: &[BaieResolue]) -> Self {
        let mut e = Self::default();
        let (mut sud, mut nord, mut ouest, mut est, mut plat) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for b in baies {
            if b.beta_deg >= BETA_2_DEG {
                e.a_baies_v_m2 += b.a_b_m2;
                // α est saisi entre 0 et 360° : le secteur sud, de −45 à 45°, y est en deux parts.
                let alpha = b.alpha_deg;
                if alpha <= 45.0 || alpha >= 315.0 {
                    sud += b.a_b_m2;
                } else if alpha < 135.0 {
                    ouest += b.a_b_m2;
                } else if alpha <= 225.0 {
                    nord += b.a_b_m2;
                } else {
                    est += b.a_b_m2;
                }
            } else {
                e.a_baies_h_m2 += b.a_b_m2;
            }
            #[allow(clippy::float_cmp)]
            if b.beta_deg == 0.0 {
                plat += b.a_b_m2;
            }
        }
        e.a_baies_m2 = e.a_baies_v_m2 + e.a_baies_h_m2;
        if e.a_baies_v_m2 > 0.0 {
            e.part_sud = sud / e.a_baies_v_m2;
            e.part_nord = nord / e.a_baies_v_m2;
            e.part_ouest = ouest / e.a_baies_v_m2;
            e.part_est = est / e.a_baies_v_m2;
        }
        if e.a_baies_m2 > 0.0 {
            e.part_horiz = plat / e.a_baies_m2;
        }
        e
    }

    /// « Abaies-sud » : surface de baies orientées au sud, en m².
    ///
    /// Th-BCE 2020 §5.14.3, éq. (272) : `Abaies-sud = Partbaies-sud × Abaies`. Le texte multiplie
    /// une part rapportée aux baies verticales par la surface de toutes les baies (question
    /// Q30) ; il en va de même pour le nord, l'ouest et l'est (éq. 274, 276 et 278).
    pub fn a_baies_sud_m2(&self) -> f64 {
        self.part_sud * self.a_baies_m2
    }
}
