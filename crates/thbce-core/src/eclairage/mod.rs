// SPDX-License-Identifier: GPL-3.0-or-later
//! Éclairage : fiche `C_ECL_éclairage` (Th-BCE 2020 §7.1).
//!
//! Périmètre : usages d'habitation (maison individuelle ou accolée, logement collectif), pour
//! lesquels le système d'éclairage est entièrement conventionnel (§7.1.3.6.2) : seules les
//! caractéristiques des baies agissent sur la consommation. L'éclairement naturel intérieur est
//! calculé pour les locaux de volume normal (baies de type 2). Les grands volumes, les modes de
//! gestion des autres usages (tableaux 78 et 79) et le fractionnement ne sont pas pris en charge.

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
