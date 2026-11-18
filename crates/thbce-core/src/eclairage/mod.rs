// SPDX-License-Identifier: GPL-3.0-or-later
//! Éclairage : fiche `C_ECL_éclairage` (Th-BCE 2020 §7.1).
//!
//! Périmètre :
//! - usages d'habitation (maison individuelle ou accolée, logement collectif), pour lesquels le
//!   système d'éclairage est entièrement conventionnel (§7.1.3.6.2) : seules les
//!   caractéristiques des baies agissent sur la consommation ;
//! - autres usages : locaux du groupe (tableau 78), puissance conventionnelle du calcul du Bbio
//!   (éq. 782) ou puissances du projet, modes de commande (`C1`) et de gestion selon la lumière
//!   naturelle (`C2`, tableau 79), fractionnement, éclairage mobilier des bureaux
//!   (§7.1.3.2.3), consommation des périphériques de gestion (éq. 787).
//!
//! L'éclairement naturel intérieur est calculé pour les locaux de volume normal (baies de
//! type 2). Les grands volumes (baies de types 0 et 1 ; salles de sport, aires de production,
//! aires de vente de plus de 300 m²) et l'éclairage d'accentuation des commerces (§7.1.3.2.2)
//! ne sont pas pris en charge.

use thbce_data::eclairage::{est_grand_volume, local as local_tableau_78, TypeLocal};

use crate::entree::{GestEcl, GradEcl, LocalEclairage};

/// « RgrA,AT » : ratio de la surface totale des parois du groupe à sa surface utile (§7.1.2,
/// constantes).
pub const R_GR_A_AT: f64 = 4.5;
pub mod parties_communes;

/// « Roplafond_pv » : coefficient de réflexion lumineuse du plafond, locaux de volume normal.
pub const RO_PLAFOND_PV: f64 = 0.7;
/// « Romurs_pv » : coefficient de réflexion lumineuse des murs, locaux de volume normal.
pub const RO_MURS_PV: f64 = 0.5;
/// « Rosol_pv » : coefficient de réflexion lumineuse du sol, locaux de volume normal.
pub const RO_SOL_PV: f64 = 0.2;
/// « Rmursol_pv » : ratio de la surface des murs à celle du sol, locaux de volume normal.
pub const R_MUR_SOL_PV: f64 = 2.5;
/// « FFbpu_h » : facteur de forme entre une baie et le plan utile (baies de type 0 et 2).
pub const FF_BPU_H: f64 = 0.4;
/// « FFplpu_h » : facteur de forme entre le plafond et le plan utile (baies de type 0 et 2).
pub const FF_PLPU_H: f64 = 0.8;
/// « Seuil_auto_lumi » : seuil d'autonomie lumineuse du groupe, en lux (§7.1.2, conventionnel).
pub const SEUIL_AUTO_LUMI_LUX: f64 = 300.0;
/// « Crec_ecl_conv » : part des apports d'éclairage récupérée sous forme convective (§7.1.3.7).
pub const C_REC_ECL_CONV: f64 = 0.5;
/// « Crec_ecl_rad » : part des apports d'éclairage récupérée sous forme radiative (§7.1.3.7).
pub const C_REC_ECL_RAD: f64 = 0.5;

/// « Pecl_tot » conventionnelle en habitation, en W/m² (§7.1.3.1, éq. 781 ; §7.1.3.6.2 : une
/// puissance installée de 10 W/m² et un facteur de non-simultanéité).
pub const P_ECL_TOT_HABITATION_W_M2: f64 = 1.0;
/// « C1 » conventionnel en habitation : interrupteur seul (§7.1.3.6.2).
pub const C1_HABITATION: f64 = 0.9;

/// Points de référence de `C2` en habitation, `(Einat en lux ; C2)` : A, B, G et J
/// (§7.1.3.6.2, tableau de la p. 501 et figure 86).
pub const POINTS_C2_HABITATION: [(f64, f64); 4] =
    [(0.0, 1.0), (100.0, 1.0), (200.0, 0.05), (2800.0, 0.0)];

/// « Romoyen » : coefficient de réflexion lumineuse moyen des parois, locaux de volume normal.
///
/// Th-BCE 2020 §7.1.3.4 :
/// `Romoyen = (Rosol_pv + Rmursol_pv·Romurs_pv + Roplafond_pv)/(2 + Rmursol_pv)`.
pub fn ro_moyen_vn() -> f64 {
    (RO_SOL_PV + R_MUR_SOL_PV * RO_MURS_PV + RO_PLAFOND_PV) / (2.0 + R_MUR_SOL_PV)
}

/// « K1vn », « K2vn », « K3vn » : coefficients de l'éclairement naturel d'un volume normal.
///
/// Th-BCE 2020 §7.1.3.4, avec `I = Romoyen/(1 − Romoyen)/RgrA,At` (interréflexions) :
/// `K1vn = Rosol_pv·I` ; `K2vn = FFbpu_h + I` ; `K3vn = Roplafond_pv·FFplpu_h + Roplafond_pv·I`.
pub fn k_vn() -> [f64; 3] {
    let ro = ro_moyen_vn();
    let inter = ro / (1.0 - ro) / R_GR_A_AT;
    [
        RO_SOL_PV * inter,
        FF_BPU_H + inter,
        RO_PLAFOND_PV * FF_PLPU_H + RO_PLAFOND_PV * inter,
    ]
}

/// « Einat(2) » : éclairement naturel du plan utile par les baies de type 2, en lux.
///
/// Th-BCE 2020 §7.1.3.4 :
/// `Einat(2) = [K1vn·Flt1(2) + K2vn·Flt2(2) + K3vn·Flt3(2)]/Aeclnat2`, somme de l'éclairement
/// diffus des baies, de celui du plafond éclairé par le flux réfléchi du sol, et des
/// interréflexions. Note du §7.1.3.4 : `Einat = 0` si `Aeclnat` est nul.
///
/// Le texte écrit aussi « Einat(2) = 1/Aeclnat2·(Einatbaiedif + Einatplafref + Einatinter) »
/// alors que chacun des trois termes est déjà divisé par `Aeclnat2` : la forme en `K`, qui ne
/// divise qu'une fois, est retenue (question Q33).
pub fn einat_vn_lux(flt1_lm: f64, flt2_lm: f64, flt3_lm: f64, a_eclnat2_m2: f64) -> f64 {
    if a_eclnat2_m2 <= 0.0 {
        return 0.0;
    }
    let [k1, k2, k3] = k_vn();
    (k1 * flt1_lm + k2 * flt2_lm + k3 * flt3_lm) / a_eclnat2_m2
}

/// « C2 » en habitation : dépendance de l'éclairage artificiel à la lumière naturelle.
///
/// Th-BCE 2020 §7.1.3.6.2, figure 86 : interpolation linéaire entre les points A (0 ; 1),
/// B (100 ; 1), G (200 ; 0,05) et J (2800 ; 0) ; nul au-delà de 2800 lux.
pub fn c2_habitation(einat_lux: f64) -> f64 {
    let points = &POINTS_C2_HABITATION;
    if einat_lux <= points[0].0 {
        return points[0].1;
    }
    for paire in points.windows(2) {
        let ((x0, y0), (x1, y1)) = (paire[0], paire[1]);
        if einat_lux <= x1 {
            return y0 + (y1 - y0) * (einat_lux - x0) / (x1 - x0);
        }
    }
    0.0
}

/// Éclairage d'un groupe d'habitation sur un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EclairageGroupe {
    /// « CECL_GR » : consommation d'énergie finale d'éclairage du groupe sur l'heure, en kWh.
    pub c_ecl_kwh: f64,
    /// « FeclC » : flux convectif récupérable, en W (puissance moyenne sur l'heure).
    pub f_ecl_c_w: f64,
    /// « FeclR » : flux radiatif récupérable, en W (puissance moyenne sur l'heure).
    pub f_ecl_r_w: f64,
}

/// Consommation d'éclairage et apports récupérables d'un groupe d'habitation.
///
/// Th-BCE 2020 §7.1.3.6.2 : `Pecl_tot = 1 W/m²`, `Pecl_aux = 0`, `Ratio_ecl_nat = 1`,
/// `Gest_ecl = 1` (`C1 = 0,9`), `Grad_ecl = 1`. Si `IEcl(h) = 0`, `CECL = 0` ; sinon, éq. (789)
/// sommée sur les locaux du groupe (éq. 786), qui partagent les mêmes valeurs :
/// `CECL_GR = Pecl_tot·C1·C2·Agr/1000`.
///
/// §7.1.3.7 : `FeclC = Crec_ecl_conv·CECL_GR` et `FeclR = Crec_ecl_rad·CECL_GR`.
///
/// Unités : la division par 1000 de l'éq. (789) donne des kWh sur l'heure, alors que la
/// nomenclature annonce des Wh ; les flux récupérables sont rendus en W, puissance moyenne sur
/// le pas de temps d'une heure (question Q33).
pub fn eclairage_habitation(i_ecl: bool, einat_lux: f64, a_gr_m2: f64) -> EclairageGroupe {
    if !i_ecl {
        return EclairageGroupe::default();
    }
    let puissance_w =
        P_ECL_TOT_HABITATION_W_M2 * C1_HABITATION * c2_habitation(einat_lux) * a_gr_m2;
    EclairageGroupe {
        c_ecl_kwh: puissance_w / 1000.0,
        f_ecl_c_w: C_REC_ECL_CONV * puissance_w,
        f_ecl_r_w: C_REC_ECL_RAD * puissance_w,
    }
}

/// Compteurs de l'autonomie en lumière naturelle d'un groupe (§7.1.3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AutonomieLumineuse {
    /// « Nbh_occ_Einat_sup » : heures de jour, éclairage autorisé, où `Einat` atteint le seuil.
    pub nbh_occ_einat_sup: u32,
    /// « Nbh_occ_Einat_inf » : heures de jour, éclairage autorisé, où `Einat` est sous le seuil.
    pub nbh_occ_einat_inf: u32,
    /// « Nbh_occ_nuit » : heures de nuit, éclairage autorisé.
    pub nbh_occ_nuit: u32,
    /// « Nbh_ecl_non_aut » : heures où l'éclairage n'est pas autorisé.
    pub nbh_ecl_non_aut: u32,
}

impl AutonomieLumineuse {
    /// Compte un pas de temps.
    ///
    /// Th-BCE 2020 §7.1.3.5, éq. (785) : si `IEcl = 1`, heure de nuit quand `Jour_astro = 0`,
    /// sinon heure de jour comptée selon que `Einat(0) + Einat(1) + Einat(2)` atteint ou non
    /// `Seuil_auto_lumi` ; si `IEcl = 0`, heure d'éclairage non autorisé.
    pub fn compter(&mut self, i_ecl: bool, jour_astro: bool, einat_lux: f64) {
        if !i_ecl {
            self.nbh_ecl_non_aut += 1;
        } else if !jour_astro {
            self.nbh_occ_nuit += 1;
        } else if einat_lux >= SEUIL_AUTO_LUMI_LUX {
            self.nbh_occ_einat_sup += 1;
        } else {
            self.nbh_occ_einat_inf += 1;
        }
    }

    /// « Taux_occ_einat_sup », en pour cent.
    ///
    /// Th-BCE 2020 §7.1.3.5 :
    /// `100·Nbh_occ_Einat_sup/(Nbh_occ_Einat_sup + Nbh_occ_Einat_inf)`. `None` si aucune heure
    /// de jour n'a été comptée.
    pub fn taux_occ_einat_sup_pct(&self) -> Option<f64> {
        let total = self.nbh_occ_einat_sup + self.nbh_occ_einat_inf;
        (total > 0).then(|| 100.0 * f64::from(self.nbh_occ_einat_sup) / f64::from(total))
    }
}

// ---------------------------------------------------------------------------------------------
// Usages autres que l'habitation (§7.1.3.1 à 7.1.3.6.1).
// ---------------------------------------------------------------------------------------------

/// « Eimin » : éclairement naturel en deçà duquel l'éclairage artificiel est indispensable, en
/// lux (§7.1.2, constantes ; point B du tableau 79).
pub const EI_MIN_LUX: f64 = 100.0;
/// « Part_resid_grad » : part de la puissance résiduelle d'un gradateur (tableau 79).
pub const PART_RESID_GRAD: f64 = 0.15;
/// « Eff_ecl_mob » : densité de puissance de l'éclairage mobilier des bureaux, en W/m² pour
/// 100 lux (§7.1.2 et §7.1.3.2.3).
pub const EFF_ECL_MOB_W_M2_100LUX: f64 = 1.0;
/// Puissance immobilière au-dessous de laquelle l'éclairage mobilier des bureaux est évalué, en
/// W/m² (§7.1.3.2.3).
pub const P_ECL_IMMO_SEUIL_BUREAUX_W_M2: f64 = 10.0;

/// Points de référence de la gestion manuelle (`Grad_ecl = 1`), `(Einat en lux ; C2)` : A, B, G
/// et J (tableau 79, figures 82 et 83).
pub const POINTS_C2_MANUEL: [(f64, f64); 4] =
    [(0.0, 1.0), (EI_MIN_LUX, 1.0), (700.0, 0.3), (2800.0, 0.0)];

fn interpoler(points: &[(f64, f64)], x: f64) -> f64 {
    if x <= points[0].0 {
        return points[0].1;
    }
    for paire in points.windows(2) {
        let ((x0, y0), (x1, y1)) = (paire[0], paire[1]);
        if x <= x1 {
            return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
        }
    }
    0.0
}

/// « Pecl_tot » conventionnelle du calcul du Bbio hors habitation, en W/m².
///
/// Th-BCE 2020 §7.1.3.1, éq. (782) : `Pecl_tot = 2 × Eiref/100`.
pub fn p_ecl_tot_bbio_w_m2(e_ref_lux: f64) -> f64 {
    2.0 * e_ref_lux / 100.0
}

/// « Pecl_tot,l » d'un local de bureaux d'une zone de bureaux, en W/m², éclairage mobilier
/// compris.
///
/// Th-BCE 2020 §7.1.3.2.3 : si la puissance immobilière `Pecl_immo_projet` est inférieure à
/// 10 W/m², l'éclairement qu'elle procure vaut `Ecl_immo_projet = 100 × Pecl_immo_projet /
/// Eff_ecl_immo_projet` ; s'il est inférieur à l'éclairement requis `Eiproj`, l'éclairage
/// mobilier apporte `Pecl_mob = (Eiproj − Ecl_immo_projet)/100 × Eff_ecl_mob`, avec
/// `Eff_ecl_mob = 1 W/m²` pour 100 lux. À partir de 10 W/m², `Pecl_tot = Pecl_immo_projet`.
pub fn p_ecl_tot_bureaux_w_m2(
    p_ecl_immo_projet_w_m2: f64,
    eff_ecl_immo_projet_w_m2_100lux: f64,
    ei_proj_lux: f64,
) -> f64 {
    if p_ecl_immo_projet_w_m2 >= P_ECL_IMMO_SEUIL_BUREAUX_W_M2 {
        return p_ecl_immo_projet_w_m2;
    }
    let ecl_immo_projet_lux = 100.0 * p_ecl_immo_projet_w_m2 / eff_ecl_immo_projet_w_m2_100lux;
    let p_ecl_mob = if ecl_immo_projet_lux < ei_proj_lux {
        (ei_proj_lux - ecl_immo_projet_lux) / 100.0 * EFF_ECL_MOB_W_M2_100LUX
    } else {
        0.0
    };
    p_ecl_immo_projet_w_m2 + p_ecl_mob
}

/// « C1 » d'un local : taux d'utilisation de l'éclairage en l'absence de lumière naturelle.
///
/// Th-BCE 2020 §7.1.3.6.1 : 1 sans commande ni détection (`Gest_ecl = 0`) ; sinon la colonne du
/// tableau 78 du mode de commande.
pub fn c1(gest_ecl: GestEcl, c1_tableau_78: &[f64; 4]) -> f64 {
    match gest_ecl {
        GestEcl::Permanent => 1.0,
        GestEcl::InterrupteurManuel => c1_tableau_78[0],
        GestEcl::InterrupteurEtHorloge => c1_tableau_78[1],
        GestEcl::MarcheEtArretAutomatiques => c1_tableau_78[2],
        GestEcl::MarcheManuelleArretAutomatique => c1_tableau_78[3],
    }
}

/// « C2 » : dépendance de l'éclairage artificiel à la lumière naturelle, selon le mode de
/// gestion.
///
/// Th-BCE 2020 §7.1.3.6.1, tableau 79 et figures 82 à 85, interpolation linéaire entre les
/// points de référence :
/// - `Grad_ecl = 0` : `C2 = 1` ;
/// - 1, gestion manuelle : A (0 ; 1), B (100 ; 1), G (700 ; 0,3), J (2800 ; 0), nul au-delà ;
/// - 2, gradation automatique : A, B, E (`Eiref` ; 0,15), H (`2·Eiref` ; 0,15), I (`2·Eiref` ;
///   0), nul au-delà de `2·Eiref` ;
/// - 3, allumage et extinction automatiques : 1 jusqu'à `Eiref`, nul au-delà ;
/// - 4, extinction automatique : courbe de la gestion manuelle jusqu'à `Eiref` (le point D est
///   sur le segment BG si `Eiref < 700`, sur le segment GJ sinon), nul au-delà.
///
/// Quand `Eiref` ne dépasse pas `Eimin` (100 lux : circulations et salle de réunion des
/// bureaux, par exemple), lecture de `docs/questions/Q058.md` :
/// - gestions 3 et 4 : le tableau 79 dit « pour Einat > EiRef, C2 = 0 », sans condition ; le
///   basculement se fait à `Eiref`, même en dessous de 100 lux (la gestion 3 n'a pas de
///   point B) ;
/// - gestion 2 : le point E précède le point B ; `C2` reste égal à 1 jusqu'à `Eimin`,
///   « éclairement naturel minimum en deçà duquel l'éclairage artificiel est indispensable »,
///   puis suit les points situés au-delà.
pub fn c2(grad_ecl: GradEcl, einat_lux: f64, e_ref_lux: f64) -> f64 {
    match grad_ecl {
        GradEcl::Aucune => 1.0,
        GradEcl::Manuelle => interpoler(&POINTS_C2_MANUEL, einat_lux),
        GradEcl::GradationAutomatique => {
            if einat_lux <= EI_MIN_LUX {
                1.0
            } else if einat_lux <= e_ref_lux {
                1.0 + (PART_RESID_GRAD - 1.0) * (einat_lux - EI_MIN_LUX) / (e_ref_lux - EI_MIN_LUX)
            } else if einat_lux <= 2.0 * e_ref_lux {
                PART_RESID_GRAD
            } else {
                0.0
            }
        }
        GradEcl::AllumageEtExtinctionAutomatiques => {
            if einat_lux <= e_ref_lux {
                1.0
            } else {
                0.0
            }
        }
        GradEcl::ExtinctionAutomatique => {
            if einat_lux <= e_ref_lux {
                interpoler(&POINTS_C2_MANUEL, einat_lux)
            } else {
                0.0
            }
        }
    }
}

/// Facteur appliqué à l'éclairement naturel de la partie d'un local de volume normal qui n'a
/// pas d'accès effectif à la lumière naturelle.
///
/// Th-BCE 2020 §7.1.3.6.1 : `2,5·Ratioecl_nat − 1,5` si `Ratioecl_nat > 0,7` ;
/// `0,5·Ratioecl_nat − 0,1` si `0,2 < Ratioecl_nat ≤ 0,7` ; 0 sinon.
pub fn facteur_einat_sans_acces(ratio_ecl_nat: f64) -> f64 {
    if ratio_ecl_nat > 0.7 {
        2.5 * ratio_ecl_nat - 1.5
    } else if ratio_ecl_nat > 0.2 {
        0.5 * ratio_ecl_nat - 0.1
    } else {
        0.0
    }
}

/// « CTRLecl_einat,l » : commande de l'éclairage artificiel d'un local de volume normal selon
/// l'éclairement naturel, en occupation.
///
/// Th-BCE 2020 §7.1.3.6.1 : `CTRLecl_einat = Ratioecl_nat·C2ae + (1 − Ratioecl_nat)·C2pae`. Si
/// le local n'a aucun accès à la lumière naturelle, `C2ae = C2pae = 1`. Sinon `C2pae` se lit
/// avec `Einat(2)` réduit par [`facteur_einat_sans_acces`] ; `C2ae` avec `Einat(2)` si la
/// gestion est fractionnée, et vaut `C2pae` si elle ne l'est pas : « c'est la partie sans accès
/// à l'éclairement naturel qui impose le fonctionnement de l'éclairage artificiel pour
/// l'ensemble du local ».
pub fn ctrl_ecl_einat(
    grad_ecl: GradEcl,
    fractionne: bool,
    ratio_ecl_nat: f64,
    einat2_lux: f64,
    e_ref_lux: f64,
) -> f64 {
    if ratio_ecl_nat <= 0.0 {
        return 1.0;
    }
    let c2_pae = c2(
        grad_ecl,
        einat2_lux * facteur_einat_sans_acces(ratio_ecl_nat),
        e_ref_lux,
    );
    let c2_ae = if fractionne {
        c2(grad_ecl, einat2_lux, e_ref_lux)
    } else {
        c2_pae
    };
    ratio_ecl_nat * c2_ae + (1.0 - ratio_ecl_nat) * c2_pae
}

/// Local d'un groupe, prêt pour le calcul de l'éclairage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalResolu {
    /// « Alocal,l » : surface du local, en m² (`Agr × Ratiolocal,l`).
    pub a_local_m2: f64,
    /// « Ratioécl_nat,l ».
    pub ratio_ecl_nat: f64,
    /// « Pecl_tot,l », en W/m².
    pub p_ecl_tot_w_m2: f64,
    /// « Pecl_aux,l », en W/m².
    pub p_ecl_aux_w_m2: f64,
    /// « C1 ».
    pub c1: f64,
    /// « Grad_ecl,l ».
    pub grad_ecl: GradEcl,
    /// Gestion fractionnée (`Fr_Grad_ecl,l = 2`).
    pub fractionne: bool,
    /// « Eiref », en lux.
    pub e_ref_lux: f64,
}

/// Vrai si le système d'un local a des périphériques de gestion (§7.1.3.6 : sans détection de
/// présence ni gestion automatique selon la lumière naturelle, `Pecl_aux = 0`).
pub fn a_des_peripheriques_de_gestion(gest_ecl: GestEcl, grad_ecl: GradEcl) -> bool {
    !(matches!(
        gest_ecl,
        GestEcl::Permanent | GestEcl::InterrupteurManuel | GestEcl::InterrupteurEtHorloge
    ) && matches!(grad_ecl, GradEcl::Aucune | GradEcl::Manuelle))
}

impl LocalResolu {
    /// Local dans le calcul du Bbio.
    ///
    /// Th-BCE 2020 §7.1.3.1, éq. (782) : `Pecl_tot = 2 × Eiref/100`, `Pecl_aux = 0`,
    /// `Gest_ecl = 2`, `Grad_ecl = 1`, gestion non fractionnée. `None` si le tableau 78 n'a pas
    /// ce type de local pour l'usage, ou pour un local de grand volume (non pris en charge).
    pub fn bbio(usage: u8, l: &LocalEclairage, a_gr_m2: f64) -> Option<Self> {
        let t = local_tableau_78(usage, l.type_local)?;
        if est_grand_volume(usage, l.type_local) {
            return None;
        }
        Some(Self {
            a_local_m2: a_gr_m2 * l.ratio_local,
            ratio_ecl_nat: l.ratio_ecl_nat,
            p_ecl_tot_w_m2: p_ecl_tot_bbio_w_m2(t.e_ref_lux),
            p_ecl_aux_w_m2: 0.0,
            c1: c1(GestEcl::InterrupteurEtHorloge, &t.c1),
            grad_ecl: GradEcl::Manuelle,
            fractionne: false,
            e_ref_lux: t.e_ref_lux,
        })
    }

    /// Local avec le système d'éclairage du projet (§7.1.3.2), pour le calcul des
    /// consommations. `None` si le tableau 78 n'a pas ce type de local pour l'usage, pour un
    /// local de grand volume, ou si une donnée requise manque (cas écartés par la validation).
    pub fn projet(usage: u8, l: &LocalEclairage, a_gr_m2: f64) -> Option<Self> {
        let t = local_tableau_78(usage, l.type_local)?;
        if est_grand_volume(usage, l.type_local) {
            return None;
        }
        let e = &l.eclairage;
        // §7.1.3.2.3 : éclairage mobilier des locaux de bureaux d'une zone de bureaux.
        let p_ecl_tot_w_m2 = if est_local_de_bureaux(usage, l.type_local)
            && e.p_ecl_tot_w_m2 < P_ECL_IMMO_SEUIL_BUREAUX_W_M2
        {
            p_ecl_tot_bureaux_w_m2(
                e.p_ecl_tot_w_m2,
                e.eff_ecl_immo_projet_w_m2_100lux?,
                e.ei_proj_lux?,
            )
        } else {
            e.p_ecl_tot_w_m2
        };
        Some(Self {
            a_local_m2: a_gr_m2 * l.ratio_local,
            ratio_ecl_nat: l.ratio_ecl_nat,
            p_ecl_tot_w_m2,
            p_ecl_aux_w_m2: if a_des_peripheriques_de_gestion(e.gest_ecl, e.grad_ecl) {
                e.p_ecl_aux_w_m2.unwrap_or(0.0)
            } else {
                0.0
            },
            c1: c1(e.gest_ecl, &t.c1),
            grad_ecl: e.grad_ecl,
            fractionne: e.fractionne,
            e_ref_lux: t.e_ref_lux,
        })
    }

    /// « CECL_local,l » : consommation du local sur l'heure, en kWh.
    ///
    /// Th-BCE 2020 §7.1.3.6, éq. (787) :
    /// `Max(Pecl_tot·CTRLecl_occ·CTRLecl_einat ; Pecl_aux)·Alocal/1000`, avec `CTRLecl_occ = 0`
    /// si `IEcl = 0` et `C1` si `IEcl = 1` (éq. 788).
    pub fn c_ecl_local_kwh(&self, i_ecl: bool, einat2_lux: f64) -> f64 {
        self.c_ecl_local_fraction_kwh(if i_ecl { 1.0 } else { 0.0 }, einat2_lux)
    }

    /// « CECL_local,l » avec un indicateur d'éclairage fractionnaire (usage 20 : 0,35 la nuit).
    /// Identique à [`Self::c_ecl_local_kwh`] pour `i_ecl` égal à 0 ou 1 (`Q074`).
    pub fn c_ecl_local_fraction_kwh(&self, i_ecl: f64, einat2_lux: f64) -> f64 {
        let ctrl_occ = ctrl_ecl_occ(self.c1, i_ecl);
        let ctrl_einat = ctrl_ecl_einat(
            self.grad_ecl,
            self.fractionne,
            self.ratio_ecl_nat,
            einat2_lux,
            self.e_ref_lux,
        );
        (self.p_ecl_tot_w_m2 * ctrl_occ * ctrl_einat).max(self.p_ecl_aux_w_m2) * self.a_local_m2
            / 1000.0
    }
}

/// « CTRLecl_occ » avec un indicateur `IEcl` éventuellement fractionnaire.
///
/// Th-BCE 2020 §7.1.3.6, éq. (788) : `CTRLecl_occ = 0` si `IEcl = 0`, `C1` si `IEcl = 1`. Le
/// classeur des scénarios donne 0,35 la nuit pour l'usage 20 (grille d'éclairage de l'onglet
/// `SAN_N`) : lecture retenue, l'éclairage de nuit à 35 % (`C1 × IEcl`) ; la variante « tout
/// indicateur strictement positif vaut 1 » n'est pas exposée (`docs/questions/Q074.md`).
pub fn ctrl_ecl_occ(c1: f64, i_ecl: f64) -> f64 {
    c1 * i_ecl
}

/// Local « de bureaux » d'une zone de bureaux, au sens du §7.1.3.2.3 : le bureau standard de
/// l'usage 3 (lecture Q59 : les salles de réunion, circulations et sanitaires sont des « autres
/// locaux », dont les caractéristiques sont des données d'entrée, §7.1.3.2.4).
pub fn est_local_de_bureaux(usage: u8, type_local: TypeLocal) -> bool {
    usage == 3 && type_local == TypeLocal::BureauStandard
}

/// « Aeclnat2 » : surface des locaux de volume normal du groupe ayant accès à la lumière
/// naturelle, en m² (§7.1.3.3 : `Σ Agr·Ratio_local,l·Ratioecl_nat,l`).
pub fn a_eclnat2_m2(locaux: &[LocalResolu]) -> f64 {
    locaux.iter().map(|l| l.a_local_m2 * l.ratio_ecl_nat).sum()
}

/// Consommation d'éclairage et apports récupérables d'un groupe hors habitation.
///
/// Th-BCE 2020 §7.1.3.6, éq. (786) : somme des consommations des locaux (éq. 787) ; §7.1.3.7 :
/// `FeclC = Crec_ecl_conv·CECL_GR` et `FeclR = Crec_ecl_rad·CECL_GR`, en W (puissance moyenne
/// sur l'heure, question Q33).
pub fn eclairage_locaux(locaux: &[LocalResolu], i_ecl: bool, einat2_lux: f64) -> EclairageGroupe {
    eclairage_locaux_fraction(locaux, if i_ecl { 1.0 } else { 0.0 }, einat2_lux)
}

/// Comme [`eclairage_locaux`], avec un indicateur d'éclairage fractionnaire (`Q074`).
pub fn eclairage_locaux_fraction(
    locaux: &[LocalResolu],
    i_ecl: f64,
    einat2_lux: f64,
) -> EclairageGroupe {
    let c_ecl_kwh: f64 = locaux
        .iter()
        .map(|l| l.c_ecl_local_fraction_kwh(i_ecl, einat2_lux))
        .sum();
    let puissance_w = c_ecl_kwh * 1000.0;
    EclairageGroupe {
        c_ecl_kwh,
        f_ecl_c_w: C_REC_ECL_CONV * puissance_w,
        f_ecl_r_w: C_REC_ECL_RAD * puissance_w,
    }
}
