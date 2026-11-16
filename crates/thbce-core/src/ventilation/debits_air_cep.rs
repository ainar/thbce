// SPDX-License-Identifier: GPL-3.0-or-later
//! Bilan aéraulique d'une zone : fiche `C_VEN_Débits_d'air_Cep` (Th-BCE 2020 §5.6).
//!
//! Malgré son nom, la fiche est le seul modèle aéraulique de la méthode : la figure 18
//! (assemblage d'une zone pour le calcul du Bbio, §5.5.3) l'appelle « calcul des débits d'air ».
//! Elle s'applique donc en Th-B comme en Th-C. Elle détermine la pression au niveau du plancher
//! de la zone, `Pib`, telle que la somme des débits massiques de la zone soit nulle, puis les
//! débits qui traversent l'enveloppe de chaque groupe par ses défauts d'étanchéité et ses
//! entrées d'air, et ceux que les groupes échangent par le hall.
//!
//! Convention de la fiche : un débit entrant dans un groupe est positif, un débit sortant
//! négatif ; `ΔP = Pext − Pint`.
//!
//! Périmètre : air neuf à la température extérieure, ou à celle d'un espace tampon pour les
//! entrées d'air qui y puisent (§5.6.3.1, éq. 134) ; hall
//! fictif (aucun groupe de circulation saisi). Les débits spécifiques sont ceux d'une ventilation
//! mécanique à simple ou à double flux (éq. 154 à 157), ce qui couvre le système conventionnel
//! du Bbio ; l'aération par les fenêtres, la ventilation naturelle par conduit et les centrales
//! à débit variable (éq. 158, 159, 162 à 166) viendront avec leurs systèmes (lot 6a).
//! Lectures : `docs/questions.md`, Q37.

use crate::bati::comportement_thermique::AirEntrant;
use crate::entree::EntreesAir;

/// « ρref » : masse volumique de référence de l'air, en kg/m³, à la température de référence.
pub const RHO_REF_KG_M3: f64 = 1.2;
/// « θref » : température de référence, en °C.
pub const THETA_REF_C: f64 = 19.0;
/// « ΔPref,1 » : différence de pression de référence des entrées d'air, en Pa.
pub const DELTA_P_REF_1_PA: f64 = 20.0;
/// « ΔPref,2 » : différence de pression de référence de la perméabilité, en Pa.
pub const DELTA_P_REF_2_PA: f64 = 4.0;
/// « Ma » : masse molaire de l'air, en g/mol.
pub const M_A_G_MOL: f64 = 28.976;
/// « Mw » : masse molaire de l'eau, en g/mol.
pub const M_W_G_MOL: f64 = 18.0;
/// « RH_EA » : rapport donnant la position de l'entrée d'air en l'absence de perméabilité
/// verticale.
pub const R_H_EA: f64 = 0.92;
/// « Cd » : coefficient de décharge.
pub const C_D: f64 = 0.68;
/// « g » : pesanteur, en m/s².
pub const G_M_S2: f64 = 9.81;
/// « Cpa » : chaleur massique de l'air, en J/(kg.K).
pub const C_PA_J_KG_K: f64 = 1006.0;

/// Masse volumique d'un air humide, en kg/m³.
///
/// Th-BCE 2020 §5.6.3.1.1, éq. (131) à (134), même expression pour l'air extérieur, l'air d'un
/// groupe, l'air soufflé et l'air neuf :
/// `ρ = ρref × (273 + θref)/(273 + θ) × (1 + ω)/(1 + ω × Ma/Mw)`.
pub fn rho_kg_m3(theta_c: f64, omega_kg_kg: f64) -> f64 {
    RHO_REF_KG_M3 * (273.0 + THETA_REF_C) / (273.0 + theta_c) * (1.0 + omega_kg_kg)
        / (1.0 + omega_kg_kg * (M_A_G_MOL / M_W_G_MOL))
}

/// « ΔPcomp » : différence de pression à laquelle est soumis un composant de façade, en Pa.
///
/// Th-BCE 2020 §5.6.3.1.2, éq. (135) à (137) : `ΔP = Pext,comp − Pint,comp`, avec
/// `Pext,comp = ½ × Cp × ρext × Vventc² − g × zcomp × ρext` et
/// `Pint,comp = Pib − ρint × g × zcomp`.
pub fn delta_p_comp_pa(
    c_p: f64,
    rho_ext_kg_m3: f64,
    v_ventc_m_s: f64,
    z_comp_m: f64,
    rho_int_kg_m3: f64,
    p_ib_pa: f64,
) -> f64 {
    let p_ext =
        0.5 * c_p * rho_ext_kg_m3 * v_ventc_m_s * v_ventc_m_s - G_M_S2 * z_comp_m * rho_ext_kg_m3;
    let p_int = p_ib_pa - rho_int_kg_m3 * G_M_S2 * z_comp_m;
    p_ext - p_int
}

/// « δtrav_zone » conventionnel : caractère traversant de la zone au sens des conditions d'hiver.
///
/// Th-BCE 2020 §5.6.3.1.3, tableau 22 : 1 pour tous les usages sauf les hôtels, partie nuit
/// (usages 8 et 9), et les établissements sanitaires avec hébergement (usage 19), où il vaut 0 ;
/// « non conventionnel » en logement collectif (usage 2) : `None`, la valeur se saisit.
pub fn trav_zone_conventionnel(numero_usage: u8) -> Option<bool> {
    match numero_usage {
        2 => None,
        8 | 9 | 19 => Some(false),
        _ => Some(true),
    }
}

/// Coefficients de pression du vent d'une zone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoefficientsPression {
    /// « Cp,v » : façades au vent.
    pub c_p_v: f64,
    /// « Cp,s » : façades sous le vent.
    pub c_p_s: f64,
    /// « Cp,t » : toit.
    pub c_p_t: f64,
}

/// « hmoy » : hauteur moyenne de la zone, en m.
///
/// Th-BCE 2020 §5.6.3.1.3 : « hmoy = hzone + 0.5 hplafond ». `hplafond` n'est pas défini et la
/// somme n'est pas une hauteur moyenne. Lecture retenue (Q37 point 2) : hauteur du milieu de la
/// zone au-dessus du sol, `hplancher + 0,5 × hzone`.
pub fn h_moy_m(h_plancher_m: f64, h_zone_m: f64) -> f64 {
    h_plancher_m + 0.5 * h_zone_m
}

/// Coefficients de pression de la zone, pour l'écrantage « normal », conventionnel.
///
/// Th-BCE 2020 §5.6.3.1.3 : « L'écrantage de la zone est fixé conventionnellement à normal ».
/// Tableau 23, zone traversante, ligne « normal » : `Cp,v = +0,25` et `Cp,s = −0,50` si
/// `hmoy < 15 m` ; `+0,45` et `−0,50` entre 15 et 50 m ; toit conventionnellement incliné de
/// plus de 80° par rapport à la verticale (note du tableau) : `Cp,t = −0,60`. Au-delà de 50 m,
/// le tableau ne donne que le site ouvert (`+0,80`, `−0,70`, `−0,70`), repris ici faute de ligne
/// « normal » (Q37 point 2) ; les bornes 15 m et 50 m, que le tableau exclut des deux côtés,
/// sont rattachées à la tranche du milieu. Tableau 24, zone non traversante : `+0,05`, `−0,05`
/// et 0.
pub fn coefficients_pression(trav_zone: bool, h_moy_m: f64) -> CoefficientsPression {
    let (c_p_v, c_p_s, c_p_t) = if !trav_zone {
        (0.05, -0.05, 0.0)
    } else if h_moy_m < 15.0 {
        (0.25, -0.50, -0.60)
    } else if h_moy_m <= 50.0 {
        (0.45, -0.50, -0.60)
    } else {
        (0.80, -0.70, -0.70)
    };
    CoefficientsPression {
        c_p_v,
        c_p_s,
        c_p_t,
    }
}

/// « δpermea_verticale » : possibilité de transferts d'air verticaux entre niveaux de la zone.
///
/// Th-BCE 2020 §5.6.3.1.4, éq. (138) : 0 en usage d'habitation ou d'hébergement, ou si
/// `hzone < 3 m` ; 1 sinon.
pub fn permea_verticale(i_hebergement: bool, h_zone_m: f64) -> bool {
    !(i_hebergement || h_zone_m < 3.0)
}

/// Altitudes des composants aérauliques d'une zone, en m.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Altitudes {
    /// « zb » des défauts d'étanchéité.
    pub z_b_m: f64,
    /// « zh » des défauts d'étanchéité.
    pub z_h_m: f64,
    /// « zt » : altitude du toit.
    pub z_t_m: f64,
    /// Altitude basse des entrées d'air.
    pub z_ea_b_m: f64,
    /// Altitude haute des entrées d'air ; égale à l'altitude basse sans perméabilité verticale.
    pub z_ea_h_m: f64,
}

/// Altitudes des défauts d'étanchéité et des entrées d'air.
///
/// Th-BCE 2020 §5.6.3.3.1, « hauteur des défauts d'étanchéité » :
/// `zb = z + hplancher + 0,25 × min(hzone, L)`, `zh = z + hplancher + 0,75 × min(hzone, L)` et
/// `zt = z + hplancher + min(hzone, L)`, avec `L = 15 m` si `δpermea_verticale = 1` et `L = 3 m`
/// sinon. §5.6.3.2.2, « hauteur des entrées d'air » : les mêmes `zb` et `zh` avec perméabilité
/// verticale ; sinon une altitude unique, `z + hplancher + RH_EA × min(hzone, 3)`.
///
/// `z` est l'altitude du lieu. Le bilan de la zone ne s'en sert pas par défaut
/// ([`LectureAltitudes`], Q37 point 3).
pub fn altitudes(z_m: f64, h_plancher_m: f64, h_zone_m: f64, permea_verticale: bool) -> Altitudes {
    let base = z_m + h_plancher_m;
    let h = h_zone_m.min(if permea_verticale { 15.0 } else { 3.0 });
    let (z_b_m, z_h_m) = (base + 0.25 * h, base + 0.75 * h);
    let (z_ea_b_m, z_ea_h_m) = if permea_verticale {
        (z_b_m, z_h_m)
    } else {
        let z = base + R_H_EA * h_zone_m.min(3.0);
        (z, z)
    };
    Altitudes {
        z_b_m,
        z_h_m,
        z_t_m: base + h,
        z_ea_b_m,
        z_ea_h_m,
    }
}

/// Origine des altitudes des composants dans les éq. (136) et (137) (`docs/questions.md`, Q37
/// point 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LectureAltitudes {
    /// Altitudes comptées depuis le plancher de la zone, niveau où `Pib` est définie
    /// (« pression au niveau du plancher de la zone ») : `z + hplancher` est ramené à zéro.
    #[default]
    DepuisLePlancher,
    /// Formules du texte telles quelles : `zcomp = z + hplancher + …`, avec l'altitude du lieu.
    /// Sans effet sur une zone d'un seul groupe ; entre deux groupes de températures
    /// différentes, crée un écart de pression `g × (z + hplancher) × (ρint,1 − ρint,2)`.
    Litterale,
}

/// Partie autoréglable de la caractéristique d'une entrée d'air.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Autoreglage {
    /// « ΔP1 » : pression du premier changement de comportement, en Pa.
    pub delta_p1_pa: f64,
    /// « ΔP2 » : pression du second changement de comportement, en Pa.
    pub delta_p2_pa: f64,
    /// « C′ » : coefficient de débit associé au défaut d'autorégulabilité.
    pub c_prime: f64,
}

/// Caractéristique débit-pression d'une entrée d'air (§5.6.3.2.1, figure 21).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntreeAir {
    /// « C » : coefficient de débit, en m³/s sous 1 Pa.
    pub c: f64,
    /// Comportement autoréglable ; `None` pour une entrée d'air fixe (`ΔP1 = ∞`).
    pub autoreglage: Option<Autoreglage>,
    /// « r » : atténuation du débit par le dispositif anti-retour ; 1 sans dispositif.
    pub r: f64,
}

/// « C » ou « C′ » : coefficient de débit d'une entrée d'air de module `M` (m³/h), en m³/s sous
/// 1 Pa.
///
/// Th-BCE 2020 §5.6.3.2.1 : `A = 10⁻⁴ × M × (10/|ΔPref,1|)^0,5` et `C = Cd × (2/ρref)^0,5 × A`.
pub fn coefficient_entree_air(m_m3_h: f64) -> f64 {
    let a = 1e-4 * m_m3_h * (10.0 / DELTA_P_REF_1_PA).sqrt();
    C_D * (2.0 / RHO_REF_KG_M3).sqrt() * a
}

impl EntreeAir {
    /// Entrée d'air de module `M`, de module de défaut d'autorégulabilité `M′` le cas échéant.
    pub fn nouvelle(m_m3_h: f64, autoreglable: Option<(f64, f64, f64)>, r: f64) -> Self {
        Self {
            c: coefficient_entree_air(m_m3_h),
            autoreglage: autoreglable.map(|(delta_p1_pa, delta_p2_pa, m_prime_m3_h)| Autoreglage {
                delta_p1_pa,
                delta_p2_pa,
                c_prime: coefficient_entree_air(m_prime_m3_h),
            }),
            r,
        }
    }

    /// Entrée d'air équivalente d'un groupe, d'après sa description. Sans dispositif
    /// anti-retour, `r = 1` (§5.6.3.2.1).
    pub fn de_saisie(e: &EntreesAir) -> Self {
        Self::nouvelle(
            e.m_m3_h,
            e.autoreglable
                .as_ref()
                .map(|a| (a.delta_p1_pa, a.delta_p2_pa, a.m_prime_m3_h)),
            e.r.unwrap_or(1.0),
        )
    }

    /// « qv,EA(ΔP) » : débit volumique, en m³/s, positif s'il entre.
    ///
    /// Th-BCE 2020 §5.6.3.2.1 :
    /// - `0 ≤ ΔP ≤ ΔP1` : `C × |ΔP|^0,5` (éq. 141) ;
    /// - `ΔP1 < ΔP ≤ ΔP2` : `|a × ΔP + b|`, avec `qv,1 = C × |ΔP1|^0,5`,
    ///   `qv,2 = C′ × |ΔP2|^0,5`, `a = (qv,2 − qv,1)/(ΔP2 − ΔP1)` et
    ///   `b = −(qv,2 × ΔP1 − qv,1 × ΔP2)/(ΔP2 − ΔP1)` (éq. 143 à 145) ;
    /// - `ΔP2 < ΔP` : `C′ × |ΔP|^0,5` (éq. 142) ;
    /// - `ΔP < 0` : `−r × C × |ΔP|^0,5` (éq. 146).
    pub fn q_v_m3_s(&self, delta_p_pa: f64) -> f64 {
        if delta_p_pa < 0.0 {
            return -self.r * self.c * delta_p_pa.abs().sqrt();
        }
        match self.autoreglage {
            Some(ar) if delta_p_pa > ar.delta_p2_pa => ar.c_prime * delta_p_pa.sqrt(),
            Some(ar) if delta_p_pa > ar.delta_p1_pa => {
                let q_v1 = self.c * ar.delta_p1_pa.abs().sqrt();
                let q_v2 = ar.c_prime * ar.delta_p2_pa.abs().sqrt();
                let d = ar.delta_p2_pa - ar.delta_p1_pa;
                let a = (q_v2 - q_v1) / d;
                let b = -(q_v2 * ar.delta_p1_pa - q_v1 * ar.delta_p2_pa) / d;
                (a * delta_p_pa + b).abs()
            }
            _ => self.c * delta_p_pa.sqrt(),
        }
    }

    /// Condition de monotonie de la caractéristique, « nécessaire pour le solveur ».
    ///
    /// Th-BCE 2020 §5.6.3.2.1 : `C′ ≥ C × |ΔP1|^0,5/|ΔP2|^0,5`, ou de manière équivalente
    /// `a ≥ 0`.
    pub fn est_monotone(&self) -> bool {
        self.autoreglage.is_none_or(|ar| {
            ar.delta_p2_pa > ar.delta_p1_pa
                && ar.c_prime * ar.delta_p2_pa.abs().sqrt() >= self.c * ar.delta_p1_pa.abs().sqrt()
        })
    }
}

/// Contribution d'un groupe à « Cdef_façade » ou à « Cdef_toit », en m³/h sous 1 Pa.
///
/// Th-BCE 2020 §5.6.3.3.1, éq. (150) : `A × Q4Pa_surf/4^(2/3)`.
pub fn coefficient_permeabilite(a_m2: f64, q4pa_surf_m3_h_m2: f64) -> f64 {
    a_m2 * q4pa_surf_m3_h_m2 / libm::pow(DELTA_P_REF_2_PA, 2.0 / 3.0)
}

/// « qv,def(ΔP) » : débit volumique d'un défaut d'étanchéité, dans l'unité de `Cdef` (m³/h).
///
/// Th-BCE 2020 §5.6.3.3.2, éq. (151) : `sgn(ΔP) × Cdef × |ΔP|^(2/3)`.
pub fn q_v_def(c_def: f64, delta_p_pa: f64) -> f64 {
    if delta_p_pa == 0.0 {
        0.0
    } else {
        delta_p_pa.signum() * c_def * libm::pow(delta_p_pa.abs(), 2.0 / 3.0)
    }
}

/// Données aérauliques d'un groupe de la zone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupeAeraulique {
    /// « Af » : surface des façades du groupe, en m² (parois opaques et baies déperditives
    /// verticales, §5.6.3.1.5).
    pub a_f_m2: f64,
    /// « At » : surface des toits du groupe, en m² (parois opaques et baies déperditives
    /// horizontales, hors plancher bas).
    pub a_t_m2: f64,
    /// « Q4Pa_surf » : perméabilité à l'air, en m³/(h.m²) sous 4 Pa.
    pub q4pa_surf_m3_h_m2: f64,
    /// Entrée d'air équivalente du groupe (somme des modules de ses entrées d'air).
    pub entree_air: Option<EntreeAir>,
}

/// Conditions extérieures d'un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Exterieur {
    /// « θext » : température extérieure, en °C.
    pub theta_ext_c: f64,
    /// « we » : humidité spécifique du site, en kg/kg.
    pub omega_e_kg_kg: f64,
    /// « Vventc » : vitesse du vent, en m/s.
    pub v_ventc_m_s: f64,
}

/// État d'un groupe et débits spécifiques qu'il reçoit, à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EtatGroupe {
    /// « θi,fin » : température de l'air du groupe à la fin du pas de temps précédent, en °C.
    pub theta_i_fin_c: f64,
    /// « ωi,fin » : humidité spécifique du groupe à la fin du pas de temps précédent, en kg/kg.
    pub omega_i_fin_kg_kg: f64,
    /// « qspec_repris » : somme des débits volumiques spécifiques repris, en m³/h (négative).
    pub q_spec_repris_m3_h: f64,
    /// « qspec_soufflé » : somme des débits volumiques spécifiques soufflés, en m³/h.
    pub q_spec_souffle_m3_h: f64,
    /// « Tair_soufflé » : température de l'air soufflé, en °C.
    pub t_air_souffle_c: f64,
    /// « ωair_soufflé » : humidité spécifique de l'air soufflé, en kg/kg.
    pub omega_air_souffle_kg_kg: f64,
    /// « TAN » des entrées d'air, en °C, quand l'air neuf transite d'abord par un espace
    /// tampon (« Tair_out », §5.6.3.1, éq. 134) ; `None` : l'air neuf est à la température extérieure.
    pub theta_an_c: Option<f64>,
}

/// Débits d'un groupe à l'équilibre aéraulique de la zone.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BilanGroupe {
    /// Débits massiques par les défauts d'étanchéité, en kg/s : façade au vent en haut, sous le
    /// vent en haut, au vent en bas, sous le vent en bas, puis toit.
    pub q_m_def_kg_s: [f64; 5],
    /// Débits massiques par les entrées d'air, en kg/s, dans le même ordre (sans le toit).
    pub q_m_ea_kg_s: [f64; 4],
    /// « qm,spec_repris » : débit massique repris par la ventilation spécifique, en kg/s.
    pub q_m_spec_repris_kg_s: f64,
    /// « qm,spec_soufflé » : débit massique soufflé par la ventilation spécifique, en kg/s.
    pub q_m_spec_souffle_kg_s: f64,
    /// « qm,hall » : débit massique échangé avec le hall, en kg/s (éq. 168 et 170) ; positif
    /// quand le groupe reçoit de l'air du hall.
    pub q_m_hall_kg_s: f64,
    /// « qm,neuf » : débit d'air neuf entrant par l'enveloppe, en kg/s (éq. 171).
    pub q_m_neuf_kg_s: f64,
    /// « qm,traversant » : débit sortant par l'enveloppe, en kg/s, négatif (éq. 172).
    pub q_m_traversant_kg_s: f64,
    /// « Qv,def,entr » : débit volumique entrant par les défauts d'étanchéité, en m³/h.
    pub q_v_def_entr_m3_h: f64,
    /// « Qv,def,sort » : débit volumique sortant par les défauts d'étanchéité, en m³/h.
    pub q_v_def_sort_m3_h: f64,
    /// « Qv,EA,entr » : débit volumique entrant par les entrées d'air, en m³/h.
    pub q_v_ea_entr_m3_h: f64,
    /// « Qv,EA,sort » : débit volumique sortant par les entrées d'air, en m³/h.
    pub q_v_ea_sort_m3_h: f64,
}

impl BilanGroupe {
    /// Somme des débits massiques du groupe hors échange avec le hall, en kg/s.
    fn somme_kg_s(&self) -> f64 {
        self.q_m_def_kg_s.iter().sum::<f64>()
            + self.q_m_ea_kg_s.iter().sum::<f64>()
            + self.q_m_spec_repris_kg_s
            + self.q_m_spec_souffle_kg_s
    }
}

/// Équilibre aéraulique d'une zone à un pas de temps.
#[derive(Debug, Clone, PartialEq)]
pub struct BilanZone {
    /// « Pib » : pression au niveau du plancher de la zone, en Pa.
    pub p_ib_pa: f64,
    /// Débits de chaque groupe.
    pub groupes: Vec<BilanGroupe>,
    /// « Hv_déf,z » : déperditions par les défauts d'étanchéité de la zone, en W/K (éq. 34 du
    /// §5.6.3.4.4), comptées sur l'air entrant (Q37 point 6).
    pub h_v_def_w_k: f64,
}

/// Modèle aéraulique d'une zone : groupes, coefficients de pression et altitudes.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneAeraulique {
    groupes: Vec<GroupeAeraulique>,
    c_p: CoefficientsPression,
    alt: Altitudes,
    /// Coefficients de perméabilité de chaque groupe : un quart de façade (tableau 29), toit.
    c_def: Vec<(f64, f64)>,
}

impl ZoneAeraulique {
    /// Prépare le modèle d'une zone.
    ///
    /// Th-BCE 2020 §5.6.3.1.5, éq. (139) et (140) : surfaces de façade et de toit de la zone et
    /// ratios `Ratf` et `Ratt` de chaque groupe ; §5.6.3.3.1, éq. (150) et tableau 29 :
    /// `Cdef_façade` du groupe à chacune des quatre positions, `¼ × Ratf × Cdef_façade,zone`, et
    /// `Cdef_toit = Ratt × Cdef_toit,zone`.
    pub fn nouvelle(
        groupes: &[GroupeAeraulique],
        trav_zone: bool,
        i_hebergement: bool,
        z_m: f64,
        h_plancher_m: f64,
        h_zone_m: f64,
    ) -> Self {
        Self::nouvelle_selon(
            groupes,
            trav_zone,
            i_hebergement,
            z_m,
            h_plancher_m,
            h_zone_m,
            LectureAltitudes::default(),
        )
    }

    /// Comme [`Self::nouvelle`], en choisissant l'origine des altitudes.
    pub fn nouvelle_selon(
        groupes: &[GroupeAeraulique],
        trav_zone: bool,
        i_hebergement: bool,
        z_m: f64,
        h_plancher_m: f64,
        h_zone_m: f64,
        lecture: LectureAltitudes,
    ) -> Self {
        let a_f_zone: f64 = groupes.iter().map(|g| g.a_f_m2).sum();
        let a_t_zone: f64 = groupes.iter().map(|g| g.a_t_m2).sum();
        let c_facade_zone: f64 = groupes
            .iter()
            .map(|g| coefficient_permeabilite(g.a_f_m2, g.q4pa_surf_m3_h_m2))
            .sum();
        let c_toit_zone: f64 = groupes
            .iter()
            .map(|g| coefficient_permeabilite(g.a_t_m2, g.q4pa_surf_m3_h_m2))
            .sum();
        let ratio = |a: f64, total: f64| if total > 0.0 { a / total } else { 0.0 };
        let c_def = groupes
            .iter()
            .map(|g| {
                (
                    0.25 * ratio(g.a_f_m2, a_f_zone) * c_facade_zone,
                    ratio(g.a_t_m2, a_t_zone) * c_toit_zone,
                )
            })
            .collect();
        Self {
            groupes: groupes.to_vec(),
            c_p: coefficients_pression(trav_zone, h_moy_m(h_plancher_m, h_zone_m)),
            alt: {
                let pv = permea_verticale(i_hebergement, h_zone_m);
                match lecture {
                    LectureAltitudes::DepuisLePlancher => altitudes(0.0, 0.0, h_zone_m, pv),
                    LectureAltitudes::Litterale => altitudes(z_m, h_plancher_m, h_zone_m, pv),
                }
            },
            c_def,
        }
    }

    /// Coefficients de pression de la zone.
    pub fn coefficients_pression(&self) -> CoefficientsPression {
        self.c_p
    }

    /// Altitudes des composants de la zone.
    pub fn altitudes(&self) -> Altitudes {
        self.alt
    }

    /// Débits des groupes pour une pression `Pib` donnée.
    ///
    /// Th-BCE 2020 §5.6.3.4 : débits massiques spécifiques, `ρint × qspec_repris/3600` et
    /// `ρair_soufflé × qspec_soufflé/3600` (éq. 154 à 157) ; débits massiques par les entrées
    /// d'air et par les défauts d'étanchéité, `qm = ρ × qv` (éq. 160 et 161), avec la masse
    /// volumique de l'air du groupe si l'air sort (`ΔP < 0`) et celle de l'air neuf ou de l'air
    /// extérieur s'il entre (Q37 point 4).
    ///
    /// Renvoie un itérateur : le solveur de l'éq. (167) l'évalue des dizaines de fois par pas de
    /// temps, sans allocation.
    fn debits<'a>(
        &'a self,
        ext: &'a Exterieur,
        etats: &'a [EtatGroupe],
        p_ib_pa: f64,
    ) -> impl Iterator<Item = BilanGroupe> + 'a {
        let rho_ext = rho_kg_m3(ext.theta_ext_c, ext.omega_e_kg_kg);
        let c_p = self.c_p;
        let alt = self.alt;
        self.groupes
            .iter()
            .zip(etats)
            .zip(&self.c_def)
            .map(move |((g, e), &(c_facade, c_toit))| {
                let rho_int = rho_kg_m3(e.theta_i_fin_c, e.omega_i_fin_kg_kg);
                // §5.6.3.1, éq. (134) : masse volumique de l'air neuf des entrées d'air, à
                // « TAN = Tair_out » s'il transite par un espace tampon, à θext sinon.
                let rho_an = e
                    .theta_an_c
                    .map_or(rho_ext, |t| rho_kg_m3(t, ext.omega_e_kg_kg));
                let dp = |c_p: f64, z: f64| {
                    delta_p_comp_pa(c_p, rho_ext, ext.v_ventc_m_s, z, rho_int, p_ib_pa)
                };
                let mut b = BilanGroupe::default();
                let defauts = [
                    (c_p.c_p_v, alt.z_h_m, c_facade),
                    (c_p.c_p_s, alt.z_h_m, c_facade),
                    (c_p.c_p_v, alt.z_b_m, c_facade),
                    (c_p.c_p_s, alt.z_b_m, c_facade),
                    (c_p.c_p_t, alt.z_t_m, c_toit),
                ];
                for (i, &(cp, z, c)) in defauts.iter().enumerate() {
                    let q_v_m3_h = q_v_def(c, dp(cp, z));
                    let rho = if q_v_m3_h < 0.0 { rho_int } else { rho_ext };
                    b.q_m_def_kg_s[i] = rho * q_v_m3_h / 3600.0;
                    b.q_v_def_entr_m3_h += q_v_m3_h.max(0.0);
                    b.q_v_def_sort_m3_h += q_v_m3_h.min(0.0);
                }
                if let Some(ea) = &g.entree_air {
                    let entrees = [
                        (c_p.c_p_v, alt.z_ea_h_m),
                        (c_p.c_p_s, alt.z_ea_h_m),
                        (c_p.c_p_v, alt.z_ea_b_m),
                        (c_p.c_p_s, alt.z_ea_b_m),
                    ];
                    for (i, &(cp, z)) in entrees.iter().enumerate() {
                        // Tableau 26 : un quart de l'entrée d'air du groupe à chaque position.
                        let q_v_m3_s = 0.25 * ea.q_v_m3_s(dp(cp, z));
                        let rho = if q_v_m3_s < 0.0 { rho_int } else { rho_an };
                        b.q_m_ea_kg_s[i] = rho * q_v_m3_s;
                        b.q_v_ea_entr_m3_h += 3600.0 * q_v_m3_s.max(0.0);
                        b.q_v_ea_sort_m3_h += 3600.0 * q_v_m3_s.min(0.0);
                    }
                }
                b.q_m_spec_repris_kg_s = rho_int * e.q_spec_repris_m3_h / 3600.0;
                b.q_m_spec_souffle_kg_s = rho_kg_m3(e.t_air_souffle_c, e.omega_air_souffle_kg_kg)
                    * e.q_spec_souffle_m3_h
                    / 3600.0;
                b
            })
    }

    /// Équilibre aéraulique de la zone.
    ///
    /// Th-BCE 2020 §5.6.3.5, éq. (167) : `Pib` annule la somme des débits massiques de la zone
    /// (défauts d'étanchéité, entrées d'air, débits spécifiques soufflés et repris). La somme
    /// décroît quand `Pib` croît ; l'équation est résolue par encadrement puis dichotomie.
    /// §5.6.3.6 : débit échangé avec le hall (éq. 168 à 170), air neuf (éq. 171) et air
    /// traversant (éq. 172).
    ///
    /// `None` si la zone n'a ni perméabilité ni entrée d'air alors que ses débits spécifiques
    /// ne s'équilibrent pas : l'équation n'a pas de solution.
    pub fn bilan(&self, ext: &Exterieur, etats: &[EtatGroupe]) -> Option<BilanZone> {
        if etats.len() != self.groupes.len() {
            return None;
        }
        let residu = |p: f64| -> f64 { self.debits(ext, etats, p).map(|b| b.somme_kg_s()).sum() };
        // Encadrement : le résidu est positif pour une pression intérieure assez basse.
        let (mut bas, mut haut) = (-1.0, 1.0);
        let mut pas = 1.0;
        while residu(bas) < 0.0 {
            pas *= 2.0;
            bas -= pas;
            if pas > 1e12 {
                return None;
            }
        }
        pas = 1.0;
        while residu(haut) > 0.0 {
            pas *= 2.0;
            haut += pas;
            if pas > 1e12 {
                return None;
            }
        }
        for _ in 0..200 {
            let milieu = 0.5 * (bas + haut);
            if milieu <= bas || milieu >= haut {
                break;
            }
            if residu(milieu) > 0.0 {
                bas = milieu;
            } else {
                haut = milieu;
            }
        }
        let p_ib_pa = 0.5 * (bas + haut);
        let mut groupes: Vec<BilanGroupe> = self.debits(ext, etats, p_ib_pa).collect();
        let mut h_v_def_w_k = 0.0;
        for b in &mut groupes {
            // Éq. (168) et (170).
            b.q_m_hall_kg_s = -b.somme_kg_s();
            // Éq. (171) et (172).
            let enveloppe = b.q_m_def_kg_s.iter().chain(&b.q_m_ea_kg_s);
            b.q_m_neuf_kg_s = enveloppe.clone().map(|q| q.max(0.0)).sum();
            b.q_m_traversant_kg_s = enveloppe.map(|q| q.min(0.0)).sum();
            h_v_def_w_k += C_PA_J_KG_K * b.q_m_def_kg_s.iter().map(|q| q.max(0.0)).sum::<f64>();
        }
        Some(BilanZone {
            p_ib_pa,
            groupes,
            h_v_def_w_k,
        })
    }
}

/// « θi,fin,hall » et « ωfin,hall » : température et humidité du hall fictif.
///
/// Th-BCE 2020 §5.5.3, éq. (130), et §5.6.3.7 : moyenne des températures et des humidités des
/// débits d'air que les groupes cèdent au hall, pondérée par ces débits. `None` si aucun groupe
/// ne cède d'air au hall.
pub fn hall_fictif(bilan: &BilanZone, etats: &[EtatGroupe]) -> Option<(f64, f64)> {
    let (mut q, mut q_theta, mut q_omega) = (0.0, 0.0, 0.0);
    for (b, e) in bilan.groupes.iter().zip(etats) {
        let cede = (-b.q_m_hall_kg_s).max(0.0);
        q += cede;
        q_theta += cede * e.theta_i_fin_c;
        q_omega += cede * e.omega_i_fin_kg_kg;
    }
    (q > 0.0).then(|| (q_theta / q, q_omega / q))
}

/// « Qmaj », « Tmaj » et « ωmaj » : flux d'air entrant dans un groupe qui n'est pas le hall.
///
/// Th-BCE 2020 §5.6.3.8 et éq. (173) et (174) : un débit pour l'air neuf entrant par
/// l'enveloppe, à la température et à l'humidité extérieures ; un débit pour l'air soufflé par
/// la ventilation spécifique, à sa température et à son humidité ; si `qm,hall > 0`, ce débit, à
/// la température et à l'humidité du hall.
///
/// Quand l'air neuf des entrées d'air transite par un espace tampon (§5.6.3.1, éq. 134,
/// `TAN = Tair_out`), la part de `qm,neuf` entrée par les entrées d'air arrive à la température
/// de l'espace tampon, avec l'humidité extérieure ; celle des défauts d'étanchéité reste à la
/// température extérieure (décision Q41 point 8).
pub fn flux_entrants(
    bilan: &BilanGroupe,
    ext: &Exterieur,
    etat: &EtatGroupe,
    hall: Option<(f64, f64)>,
) -> Vec<AirEntrant> {
    let mut flux = Vec::with_capacity(4);
    match etat.theta_an_c {
        None => flux.push(AirEntrant {
            qm_kg_s: bilan.q_m_neuf_kg_s,
            theta_c: ext.theta_ext_c,
            omega_kg_kg: ext.omega_e_kg_kg,
        }),
        Some(theta_an_c) => {
            let q_ea: f64 = bilan.q_m_ea_kg_s.iter().map(|q| q.max(0.0)).sum();
            flux.push(AirEntrant {
                qm_kg_s: bilan.q_m_neuf_kg_s - q_ea,
                theta_c: ext.theta_ext_c,
                omega_kg_kg: ext.omega_e_kg_kg,
            });
            flux.push(AirEntrant {
                qm_kg_s: q_ea,
                theta_c: theta_an_c,
                omega_kg_kg: ext.omega_e_kg_kg,
            });
        }
    }
    flux.push(AirEntrant {
        qm_kg_s: bilan.q_m_spec_souffle_kg_s,
        theta_c: etat.t_air_souffle_c,
        omega_kg_kg: etat.omega_air_souffle_kg_kg,
    });
    if let (true, Some((theta_c, omega_kg_kg))) = (bilan.q_m_hall_kg_s > 0.0, hall) {
        flux.push(AirEntrant {
            qm_kg_s: bilan.q_m_hall_kg_s,
            theta_c,
            omega_kg_kg,
        });
    }
    flux
}
