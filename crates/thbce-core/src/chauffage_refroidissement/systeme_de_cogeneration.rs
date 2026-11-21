// SPDX-License-Identifier: GPL-3.0-or-later
//! Système de cogénération : fiche `C_GEN_système de cogénération` (Th-BCE 2020 §8.21), type 700
//! du tableau 111 : micro-cogénérateurs (moteur à combustion interne, moteur Stirling,
//! micro-turbine) de débit calorifique inférieur à 70 kW, avec ou sans brûleur d'appoint
//! intégré.
//!
//! Au pas de temps, l'ECS est servie d'abord (éq. 1188 à 1190), puis le chauffage sur la part
//! restante du pas (éq. 1191 à 1194) : ces deux appels se font dans l'ordre de la gestion des
//! priorités (fiche 8.17). Le bilan du pas — température aval moyenne (éq. 1179), rendements
//! corrigés, pertes, électricité produite, auxiliaires et consommation (éq. 1180 à 1224) — ne se
//! fait qu'une fois les deux postes servis, par [`Cogenerateur::bilan_du_pas`], appelée par la
//! génération après tous les appels du pas. Lectures : `docs/questions/Q157.md` et
//! `docs/questions/Q158.md`.

use super::calculs_generation::{Energie, Poste};
use super::generation::{EntreesAppel, SortiesAppel};

/// « fpa » : part des pertes échangée avec l'ambiance (tableau 135 : 0,3).
pub const F_PA: f64 = 0.3;
/// Rendement thermique par défaut du brûleur d'appoint intégré, en % (§8.21.3.1.3).
pub const ETA_BRULEUR_APPOINT_PCT: f64 = 90.0;
/// « PWaux,veille » par défaut, en W (tableau 140).
pub const P_W_AUX_VEILLE_DEFAUT_W: f64 = 15.0;
/// Part de « Pth;chp100+sup0 » qui donne « Pth,chp_min » par défaut (tableau 140).
pub const PART_P_TH_MIN_DEFAUT: f64 = 0.6;

/// Lecture des facteurs d'interpolation des éq. (1207), (1211) et (1215) (Q158).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LectureInterpolation {
    /// Retenue : interpolation linéaire entre les points de fonctionnement connus (§8.21.1,
    /// figure 116), continue aux bornes des modes : `fx = τ/(Pmin/Pmax)` en tout ou rien,
    /// `(τ − Pmin/Pmax)/(Pmax,chp/Pmax − Pmin/Pmax)` en module seul,
    /// `(τ − Pmax,chp/Pmax)/(1 − Pmax,chp/Pmax)` avec l'appoint.
    #[default]
    Figure116,
    /// Variante : formules imprimées, `fx = Pmin/Pmax · τ`, `Pmax,chp/Pmax · τ` et `τ`.
    Litterale,
}

/// Valeurs utiles maximales sans appoint intégré (tableau 138) : `(ηth, ηel)` en % sur PCS.
pub const fn valeurs_utiles_max_sans_appoint(t: TechnologieCogeneration) -> (f64, f64) {
    match t {
        TechnologieCogeneration::Stirling => (80.0, 15.0),
        TechnologieCogeneration::MoteurCombustionInterne
        | TechnologieCogeneration::MicroTurbine => (66.0, 24.0),
    }
}

/// « PWaux » utile maximale, en kW (tableau 138 et éq. 1178) : `45/1000 · Pth^0,48`.
pub fn p_w_aux_utile_max_kw(p_th_kw: f64) -> f64 {
    45.0 / 1000.0 * libm::pow(p_th_kw.max(0.0), 0.48)
}

/// Valeurs utiles maximales avec appoint intégré (éq. 1175 à 1177) : `(ηth, ηel)` en %, à
/// partir de celles du module seul.
pub fn valeurs_utiles_max_avec_appoint(
    eta_th0_pct: f64,
    eta_el0_pct: f64,
    p_th0_kw: f64,
    p_th100_kw: f64,
) -> (f64, f64) {
    // Éq. (1175).
    let fx = if p_th100_kw > 0.0 {
        p_th0_kw / p_th100_kw
    } else {
        1.0
    };
    // Éq. (1176), rendements en fraction.
    let th = 1.0 / (fx / (eta_th0_pct / 100.0) + (1.0 - fx) / (ETA_BRULEUR_APPOINT_PCT / 100.0));
    // Éq. (1177).
    let el = fx * (eta_el0_pct / 100.0) * th / (eta_th0_pct / 100.0);
    (100.0 * th, 100.0 * el)
}

/// « φth,stab,30 » par défaut, en W (tableau 140) : `1000 · 4,25 % · Pth / Pth^0,4`, avec la
/// puissance du module seul sans appoint, du module et de l'appoint sinon.
pub fn phi_th_stab_30_defaut_w(p_th_kw: f64) -> f64 {
    if p_th_kw <= 0.0 {
        return 0.0;
    }
    1000.0 * 0.0425 * p_th_kw / libm::pow(p_th_kw, 0.4)
}

/// Technologie du micro-cogénérateur (« type_chp »).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TechnologieCogeneration {
    /// Moteur à combustion interne.
    MoteurCombustionInterne,
    /// Moteur Stirling.
    Stirling,
    /// Micro-turbine.
    MicroTurbine,
}

/// Point de fonctionnement à pleine charge du module (avec l'appoint à 0 % ou à 100 %).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointPleineCharge {
    /// « Pth », en kW.
    pub p_th_kw: f64,
    /// « ηth,nom », en % sur PCS, après prise en compte du statut.
    pub eta_th_pct: f64,
    /// « ηel,nom », en % sur PCS.
    pub eta_el_pct: f64,
    /// « PWaux », en W.
    pub p_w_aux_w: f64,
    /// « θav,ref », en °C (tableau 137).
    pub theta_ref_c: f64,
}

/// État accumulé au cours d'un pas de temps, par cogénérateur identique.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct EtatPas {
    appele: bool,
    appele_ecs: bool,
    ecs_seule: bool,
    q_req_ecs_wh: f64,
    q_fou_ecs_wh: f64,
    theta_ecs_c: f64,
    r_fonct_ecs: f64,
    appele_ch: bool,
    q_req_ch_wh: f64,
    q_fou_ch_wh: f64,
    theta_ch_c: f64,
    r_puis_dispo: f64,
    tau_charge_ch: f64,
}

/// Bilan d'un pas de temps d'un ensemble de cogénérateurs identiques, en Wh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BilanCogeneration {
    /// Énergie du combustible principal.
    pub energie: Energie,
    /// « Qcef(ch;idengen) » et « Qcef(ecs;idengen) ».
    pub q_cef_combustible_wh: [f64; 2],
    /// « Qcef(ch;élec) » et « Qcef(ecs;élec) ».
    pub q_cef_elec_wh: [f64; 2],
    /// « Qcons ».
    pub q_cons_wh: f64,
    /// « Qprelec ».
    pub q_prelec_wh: f64,
    /// « Qpertes ».
    pub q_pertes_wh: f64,
    /// « Waux,pro ».
    pub w_aux_pro_wh: f64,
    /// « φvc ».
    pub phi_vc_wh: f64,
    /// « ηeff ».
    pub eta_eff: f64,
}

/// Micro-cogénérateur résolu.
#[derive(Debug, Clone, PartialEq)]
pub struct Cogenerateur {
    /// Le générateur assure le chauffage.
    pub chauffage: bool,
    /// Le générateur assure l'ECS.
    pub ecs: bool,
    /// Énergie du combustible.
    pub energie: Energie,
    /// « PCSI » (tableau 136).
    pub pcsi: f64,
    /// « Rdim ».
    pub rdim: f64,
    /// « idpos_gen » : vrai en volume chauffé.
    pub en_volume_chauffe: bool,
    /// « idraccord_gnr » : raccordé en permanence (lecture Q157).
    pub raccord_permanent: bool,
    /// « αth » (tableau 137).
    pub alpha_th: f64,
    /// « αel » (tableau 137).
    pub alpha_el: f64,
    /// Module seul à pleine charge (« chp100+sup0 »).
    pub module_seul: PointPleineCharge,
    /// Module et appoint intégré à pleine charge (« chp100+sup100 »).
    pub avec_appoint: Option<PointPleineCharge>,
    /// « Pth,chp_min », en kW.
    pub p_th_min_kw: f64,
    /// « φth,stab,30 », en W.
    pub phi_th_stab_30_w: f64,
    /// « PWaux,veille », en W.
    pub p_w_aux_veille_w: f64,
    /// Lecture des facteurs d'interpolation (Q158).
    pub lecture: LectureInterpolation,
    cour: EtatPas,
}

impl Cogenerateur {
    /// Cogénérateur décrit par ses grandeurs résolues ; `postes` : (chauffage, ECS) ; `alpha` :
    /// (αth, αel) ; `pertes` : (Pth,chp_min en kW, φth,stab,30 en W, PWaux,veille en W).
    #[allow(clippy::too_many_arguments)]
    pub fn nouveau(
        postes: (bool, bool),
        energie: Energie,
        pcsi: f64,
        rdim: f64,
        en_volume_chauffe: bool,
        raccord_permanent: bool,
        alpha: (f64, f64),
        module_seul: PointPleineCharge,
        avec_appoint: Option<PointPleineCharge>,
        pertes: (f64, f64, f64),
    ) -> Self {
        Self {
            chauffage: postes.0,
            ecs: postes.1,
            energie,
            pcsi,
            rdim,
            en_volume_chauffe,
            raccord_permanent,
            alpha_th: alpha.0,
            alpha_el: alpha.1,
            module_seul,
            avec_appoint,
            p_th_min_kw: pertes.0,
            phi_th_stab_30_w: pertes.1,
            p_w_aux_veille_w: pertes.2,
            lecture: LectureInterpolation::default(),
            cour: EtatPas::default(),
        }
    }

    /// « Pngen » : puissance utile nominale, appoint intégré compris (éq. 1173 et 1174), en kW.
    pub fn pn_gen_kw(&self) -> f64 {
        self.avec_appoint
            .map_or(self.module_seul.p_th_kw, |a| a.p_th_kw)
    }

    /// « Rdim × Pngen », en W (clé de répartition de l'éq. 1017).
    pub fn pn_gen_tot_w(&self) -> f64 {
        self.rdim * 1000.0 * self.pn_gen_kw()
    }

    /// Rendements thermique et électrique d'un point, corrigés à la température aval, en %
    /// (éq. 1180 à 1183).
    fn rendements(&self, p: &PointPleineCharge, theta_aval_c: f64) -> (f64, f64) {
        // Bornés à zéro : avec « αth » élevé (condensation) et une température aval très
        // supérieure à « θav,ref », la correction pourrait annuler le rendement thermique ;
        // le module ne fournit alors plus rien, au lieu de produire des valeurs infinies
        // (lecture Q157).
        (
            (p.eta_th_pct + self.alpha_th * (p.theta_ref_c - theta_aval_c)).max(0.0),
            (p.eta_el_pct + self.alpha_el * (p.theta_ref_c - theta_aval_c)).max(0.0),
        )
    }

    /// « Pmax,chp(h) » (éq. 1186) et « Pmax(h) » (éq. 1185), en W.
    fn p_max_w(&self, theta_aval_c: f64) -> (f64, f64) {
        let m = &self.module_seul;
        let (th0, _) = self.rendements(m, theta_aval_c);
        let p_chp = 1000.0 * m.p_th_kw * th0 / m.eta_th_pct;
        let p = self.avec_appoint.map_or(p_chp, |a| {
            let (th, _) = self.rendements(&a, theta_aval_c);
            1000.0 * a.p_th_kw * th / a.eta_th_pct
        });
        (p_chp, p)
    }

    /// Fin du pas de temps : l'état accumulé est remis à zéro.
    pub fn fin_de_pas(&mut self) {
        self.cour = EtatPas::default();
    }

    /// Appel pour un poste : énergie fournie (éq. 1188 à 1194). La consommation est calculée
    /// par [`Self::bilan_du_pas`].
    pub fn appel(&mut self, e: &EntreesAppel) -> SortiesAppel {
        let q_req = e.q_req_wh.max(0.0) / self.rdim;
        self.cour.appele = true;
        // Un poste n'est appelé qu'une fois par pas : l'état du poste est remplacé, non
        // cumulé (lecture Q157).
        debug_assert!(
            !(if e.fonction == Poste::Ecs {
                self.cour.appele_ecs
            } else {
                self.cour.appele_ch
            }),
            "cogénérateur appelé deux fois pour le même poste dans un pas de temps"
        );
        let (q_fou, tau, r_fonct) = if e.fonction == Poste::Ecs {
            self.cour.appele_ecs = true;
            // Éq. (1188) et (1189), avec « Pmax,ecs(h) » à la température aval de l'ECS.
            let (_, p_max) = self.p_max_w(e.theta_aval_c);
            let q_fou = q_req.min(p_max).max(0.0);
            let r = if p_max > 0.0 { q_fou / p_max } else { 0.0 };
            self.cour.ecs_seule = e.ecs_seule;
            self.cour.q_req_ecs_wh = q_req;
            self.cour.q_fou_ecs_wh = q_fou;
            self.cour.theta_ecs_c = e.theta_aval_c;
            self.cour.r_fonct_ecs = r;
            (q_fou, r, r)
        } else {
            // Éq. (1191) à (1193).
            let (_, p_max) = self.p_max_w(e.theta_aval_c);
            let dispo = e.r_pui_dispo * p_max;
            let q_fou = q_req.min(dispo).max(0.0);
            let tau = if dispo > 0.0 { q_fou / dispo } else { 0.0 };
            self.cour.appele_ch = true;
            self.cour.q_req_ch_wh = q_req;
            self.cour.q_fou_ch_wh = q_fou;
            self.cour.theta_ch_c = e.theta_aval_c;
            self.cour.r_puis_dispo = e.r_pui_dispo;
            self.cour.tau_charge_ch = tau;
            (q_fou, tau, 0.0)
        };
        let q_fou_tot = q_fou * self.rdim;
        SortiesAppel {
            energie: self.energie,
            q_fou_wh: q_fou_tot,
            // Éq. (1190) et (1194).
            q_rest_wh: e.q_req_wh.max(0.0) - q_fou_tot,
            tau_charge: tau,
            r_fonct_ecs: r_fonct,
            ..SortiesAppel::default()
        }
    }

    /// Facteur d'interpolation d'un mode de fonctionnement (éq. 1207, 1211 et 1215, Q158) :
    /// `bas` et `haut` sont les bornes du mode en taux de charge, `k` le coefficient imprimé.
    fn fx(&self, tau: f64, bas: f64, haut: f64, k: f64) -> f64 {
        match self.lecture {
            LectureInterpolation::Figure116 => {
                if haut > bas {
                    ((tau - bas) / (haut - bas)).clamp(0.0, 1.0)
                } else {
                    1.0
                }
            }
            LectureInterpolation::Litterale => k * tau,
        }
    }

    /// Bilan du pas de temps (éq. 1179 à 1224), une fois les postes servis ; `None` si le
    /// cogénérateur n'a été appelé pour aucun poste (hors période d'activation de ses postes).
    pub fn bilan_du_pas(&self, theta_amb_c: f64) -> Option<BilanCogeneration> {
        let c = &self.cour;
        if !c.appele {
            return None;
        }
        // Éq. (1179) ; sans demande, température du poste appelé (lecture Q157).
        let q_req = c.q_req_ecs_wh + c.q_req_ch_wh;
        let theta_aval = if q_req > 0.0 {
            (c.q_req_ecs_wh * c.theta_ecs_c + c.q_req_ch_wh * c.theta_ch_c) / q_req
        } else if c.appele_ch {
            c.theta_ch_c
        } else {
            c.theta_ecs_c
        };
        let m = &self.module_seul;
        let (th0, el0) = self.rendements(m, theta_aval);
        // Éq. (1184).
        let x = ((theta_aval - theta_amb_c) / 30.0).max(0.0);
        let phi_stab = (self.phi_th_stab_30_w * libm::pow(x, 1.25)).max(0.0);
        let (p_max_chp, p_max) = self.p_max_w(theta_aval);
        // Éq. (1187) : rapport des rendements du point « chp100+sup100 », celui du module seul
        // sans appoint intégré (lecture Q157).
        let rapport_min = self.avec_appoint.map_or(th0 / m.eta_th_pct, |a| {
            self.rendements(&a, theta_aval).0 / a.eta_th_pct
        });
        let p_min_chp = 1000.0 * self.p_th_min_kw * rapport_min;
        // Éq. (1195) à (1199).
        let pertes_relatives = |th: f64, el: f64| {
            if th + el > 0.0 {
                100.0 / (th + el) - 1.0
            } else {
                0.0
            }
        };
        let rapport_el_th = |th: f64, el: f64| if th > 0.0 { el / th } else { 0.0 };
        let phi_min = pertes_relatives(th0, el0) * p_min_chp;
        let prelec_min = rapport_el_th(th0, el0) * p_min_chp;
        let w_aux0 = m.p_w_aux_w;
        let w_aux_min = if p_max_chp > 0.0 {
            w_aux0 * p_min_chp / p_max_chp
        } else {
            0.0
        };
        let phi_0 = pertes_relatives(th0, el0) * p_max_chp;
        let prelec_0 = rapport_el_th(th0, el0) * p_max_chp;
        // Éq. (1202) et (1203).
        let q_fou = c.q_fou_ecs_wh + c.q_fou_ch_wh;
        let tau = c.r_fonct_ecs + c.r_puis_dispo * c.tau_charge_ch;
        let r_min = if p_max > 0.0 { p_min_chp / p_max } else { 0.0 };
        let r_chp = if p_max > 0.0 { p_max_chp / p_max } else { 0.0 };
        let (q_pertes, q_prelec, w_aux, q_cons) = if tau <= 0.0 || q_fou <= 0.0 {
            // §8.21.3.2.7, éq. (1217) à (1221).
            let permanent = if self.raccord_permanent { 1.0 } else { 0.0 };
            (0.0, 0.0, self.p_w_aux_veille_w, permanent * phi_stab)
        } else {
            let (q_pertes, q_prelec, w_aux) = if tau <= r_min {
                // Éq. (1204) à (1207) : brûleur seul en tout ou rien ; « Pveilleuse » n'est
                // pas défini par la fiche, pris nul (lecture Q158).
                let fx = self.fx(tau, 0.0, r_min, r_min);
                (
                    fx * phi_min + (1.0 - fx) * phi_stab,
                    0.0,
                    fx * w_aux_min + (1.0 - fx) * self.p_w_aux_veille_w,
                )
            } else if tau <= r_chp || self.avec_appoint.is_none() {
                // Éq. (1208) à (1211) : module seul, régulé.
                let fx = self.fx(tau, r_min, r_chp, r_chp);
                (
                    fx * phi_0 + (1.0 - fx) * phi_min,
                    fx * prelec_0 + (1.0 - fx) * prelec_min,
                    fx * w_aux0 + (1.0 - fx) * w_aux_min,
                )
            } else {
                // Éq. (1200), (1201) et (1212) à (1215) : module et appoint.
                let a = self.avec_appoint.unwrap_or(*m);
                let (th1, el1) = self.rendements(&a, theta_aval);
                let phi_1 = pertes_relatives(th1, el1) * p_max;
                let prelec_1 = rapport_el_th(th1, el1) * p_max;
                let fx = self.fx(tau, r_chp, 1.0, 1.0);
                (
                    fx * phi_1 + (1.0 - fx) * phi_0,
                    fx * prelec_1 + (1.0 - fx) * prelec_0,
                    fx * a.p_w_aux_w + (1.0 - fx) * w_aux0,
                )
            };
            // Éq. (1216).
            let q_cons = (q_fou + q_prelec + q_pertes) / self.pcsi;
            (q_pertes, q_prelec, w_aux, q_cons)
        };
        // Éq. (1222).
        let phi_vc = if self.en_volume_chauffe {
            F_PA * (q_pertes + w_aux)
        } else {
            0.0
        };
        // Éq. (1224) ; un cogénérateur qui n'assure qu'un poste y compte tout (lecture Q157).
        let part_ecs = if !self.chauffage {
            1.0
        } else if !self.ecs {
            0.0
        } else if c.ecs_seule || c.tau_charge_ch <= 0.0 {
            1.0
        } else {
            c.r_fonct_ecs
        };
        let r = self.rdim;
        Some(BilanCogeneration {
            energie: self.energie,
            q_cef_combustible_wh: [r * q_cons * (1.0 - part_ecs), r * q_cons * part_ecs],
            q_cef_elec_wh: [r * w_aux * (1.0 - part_ecs), r * w_aux * part_ecs],
            q_cons_wh: r * q_cons,
            q_prelec_wh: r * q_prelec,
            q_pertes_wh: r * q_pertes,
            w_aux_pro_wh: r * w_aux,
            phi_vc_wh: r * phi_vc,
            // Éq. (1223).
            eta_eff: if q_cons > 0.0 { q_fou / q_cons } else { 0.0 },
        })
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    /// Moteur à combustion interne au gaz naturel, standard, sans appoint : 10 kW thermiques,
    /// ηth = 0,8 × 66 = 52,8 %, ηel = 0,8 × 24 = 19,2 %, PWaux = 1,2 × 45/1000 × 10^0,48 kW,
    /// Pth,min = 6 kW, θav,ref = 53,5 °C, αth = 0,04.
    fn mci() -> Cogenerateur {
        let w_aux = 1000.0 * 1.2 * p_w_aux_utile_max_kw(10.0);
        Cogenerateur::nouveau(
            (true, true),
            Energie::Gaz,
            1.11,
            1.0,
            false,
            false,
            (0.04, 0.0),
            PointPleineCharge {
                p_th_kw: 10.0,
                eta_th_pct: 52.8,
                eta_el_pct: 19.2,
                p_w_aux_w: w_aux,
                theta_ref_c: 53.5,
            },
            None,
            (6.0, phi_th_stab_30_defaut_w(10.0), 15.0),
        )
    }

    fn appel(g: &mut Cogenerateur, poste: Poste, q: f64, theta: f64, r: f64) -> SortiesAppel {
        let mut e = EntreesAppel::demande(poste, q);
        e.theta_aval_c = theta;
        e.r_pui_dispo = r;
        e.ecs_seule = false;
        g.appel(&e)
    }

    #[test]
    fn valeurs_par_defaut_a_la_main() {
        // Tableau 140 : 1000 × 4,25 % × 10 / 10^0,4.
        let phi = 1000.0 * 0.0425 * 10.0 / libm::pow(10.0, 0.4);
        assert!((phi_th_stab_30_defaut_w(10.0) - phi).abs() < 1e-12);
        // Éq. (1175) à (1177) : module de 10 kW, appoint jusqu'à 20 kW, fx = 0,5.
        let (th, el) = valeurs_utiles_max_avec_appoint(66.0, 24.0, 10.0, 20.0);
        let th_a = 1.0 / (0.5 / 0.66 + 0.5 / 0.9);
        assert!((th - 100.0 * th_a).abs() < 1e-9);
        assert!((el - 100.0 * 0.5 * 0.24 * th_a / 0.66).abs() < 1e-9);
    }

    /// Module seul, régulé : chauffage seul à 8 kWh, θaval = 53,5 °C (pas de correction).
    #[test]
    fn module_seul_a_la_main() {
        let mut g = mci();
        let s = appel(&mut g, Poste::Chauffage, 8000.0, 53.5, 1.0);
        assert_eq!(s.q_fou_wh, 8000.0);
        let b = g.bilan_du_pas(20.0).unwrap();
        // τ = 0,8 ; Pmin/Pmax = 0,6 : fx = (0,8 − 0,6)/(1 − 0,6) = 0,5.
        let pertes = |p: f64| (100.0 / (52.8 + 19.2) - 1.0) * p;
        let prelec = |p: f64| 19.2 / 52.8 * p;
        let q_pertes = 0.5 * pertes(10_000.0) + 0.5 * pertes(6000.0);
        let q_prelec = 0.5 * prelec(10_000.0) + 0.5 * prelec(6000.0);
        assert!((b.q_pertes_wh - q_pertes).abs() < 1e-9);
        assert!((b.q_prelec_wh - q_prelec).abs() < 1e-9);
        // Éq. (1216).
        assert!((b.q_cons_wh - (8000.0 + q_prelec + q_pertes) / 1.11).abs() < 1e-9);
        // Tout au chauffage (pas d'ECS au pas de temps).
        assert_eq!(b.q_cef_combustible_wh[1], 0.0);
        let w0 = 1000.0 * 1.2 * p_w_aux_utile_max_kw(10.0);
        assert!((b.w_aux_pro_wh - (0.5 * w0 + 0.5 * 0.6 * w0)).abs() < 1e-9);
    }

    /// Tout ou rien sous la puissance minimale, ECS puis chauffage, et arrêt.
    #[test]
    fn tout_ou_rien_double_service_et_arret() {
        let mut g = mci();
        // θaval ECS 60 °C, chauffage 40 °C.
        let s = appel(&mut g, Poste::Ecs, 2000.0, 60.0, 1.0);
        let p_max_ecs = 1000.0 * 10.0 * (52.8 + 0.04 * (53.5 - 60.0)) / 52.8;
        assert!((s.r_fonct_ecs - 2000.0 / p_max_ecs).abs() < 1e-12);
        let r = s.r_fonct_ecs;
        let s = appel(&mut g, Poste::Chauffage, 1000.0, 40.0, 1.0 - r);
        assert_eq!(s.q_fou_wh, 1000.0);
        let b = g.bilan_du_pas(20.0).unwrap();
        // Éq. (1179) : θaval = (2000 × 60 + 1000 × 40)/3000.
        let theta = (2000.0 * 60.0 + 1000.0 * 40.0) / 3000.0;
        let th0 = 52.8 + 0.04 * (53.5 - theta);
        let p_max = 1000.0 * 10.0 * th0 / 52.8;
        let p_min = 1000.0 * 6.0 * th0 / 52.8;
        let p_max_ch = 1000.0 * 10.0 * (52.8 + 0.04 * (53.5 - 40.0)) / 52.8;
        let tau = r + (1.0 - r) * 1000.0 / ((1.0 - r) * p_max_ch);
        assert!(tau < p_min / p_max);
        let fx = tau / (p_min / p_max);
        let phi_stab = phi_th_stab_30_defaut_w(10.0) * libm::pow((theta - 20.0) / 30.0, 1.25);
        let phi_min = (100.0 / (th0 + 19.2) - 1.0) * p_min;
        let q_pertes = fx * phi_min + (1.0 - fx) * phi_stab;
        assert!((b.q_pertes_wh - q_pertes).abs() < 1e-9);
        assert_eq!(b.q_prelec_wh, 0.0);
        // Éq. (1224) : répartition selon Rfonct,ECS.
        assert!((b.q_cef_combustible_wh[1] - b.q_cons_wh * r).abs() < 1e-9);
        // Arrêt, avec isolement : veille seule ; raccordé en permanence : pertes à l'arrêt.
        g.fin_de_pas();
        appel(&mut g, Poste::Chauffage, 0.0, 50.0, 1.0);
        let b = g.bilan_du_pas(20.0).unwrap();
        assert_eq!((b.q_cons_wh, b.w_aux_pro_wh), (0.0, 15.0));
        g.raccord_permanent = true;
        let b = g.bilan_du_pas(20.0).unwrap();
        assert!((b.q_cons_wh - phi_th_stab_30_defaut_w(10.0)).abs() < 1e-9);
        // Non appelé : rien.
        g.fin_de_pas();
        assert!(g.bilan_du_pas(20.0).is_none());
    }

    /// Même cogénérateur avec brûleur d'appoint intégré : 20 kW à pleine charge, ηth = 70 %,
    /// ηel = 12 %, auxiliaires de 2 × ceux du module seul.
    fn mci_avec_appoint() -> Cogenerateur {
        let mut g = mci();
        let w0 = 1000.0 * 1.2 * p_w_aux_utile_max_kw(10.0);
        g.avec_appoint = Some(PointPleineCharge {
            p_th_kw: 20.0,
            eta_th_pct: 70.0,
            eta_el_pct: 12.0,
            p_w_aux_w: 2.0 * w0,
            theta_ref_c: 53.5,
        });
        g
    }

    /// Mode module + appoint (éq. 1200, 1201, 1212 à 1215), θaval = θav,ref (pas de
    /// correction) : Pmax,chp = 10 kW, Pmax = 20 kW, Pmax,chp/Pmax = 0,5 ; demande de 15 kWh,
    /// soit τ = 0,75.
    #[test]
    fn module_et_appoint_a_la_main() {
        let w0 = 1000.0 * 1.2 * p_w_aux_utile_max_kw(10.0);
        // Éq. (1195) à (1200) : pertes et production électrique de chaque point.
        let phi_0 = (100.0 / (52.8 + 19.2) - 1.0) * 10_000.0;
        let prelec_0 = 19.2 / 52.8 * 10_000.0;
        let phi_1 = (100.0 / (70.0 + 12.0) - 1.0) * 20_000.0;
        let prelec_1 = 12.0 / 70.0 * 20_000.0;
        for (lecture, fx) in [
            // Retenue : (τ − 0,5)/(1 − 0,5) ; variante imprimée : fx = τ.
            (LectureInterpolation::Figure116, 0.5),
            (LectureInterpolation::Litterale, 0.75),
        ] {
            let mut g = mci_avec_appoint();
            g.lecture = lecture;
            let s = appel(&mut g, Poste::Chauffage, 15_000.0, 53.5, 1.0);
            assert_eq!(s.q_fou_wh, 15_000.0);
            assert!((s.tau_charge - 0.75).abs() < 1e-12);
            let b = g.bilan_du_pas(20.0).unwrap();
            let q_pertes = fx * phi_1 + (1.0 - fx) * phi_0;
            let q_prelec = fx * prelec_1 + (1.0 - fx) * prelec_0;
            let w_aux = fx * 2.0 * w0 + (1.0 - fx) * w0;
            assert!((b.q_pertes_wh - q_pertes).abs() < 1e-9, "{lecture:?}");
            assert!((b.q_prelec_wh - q_prelec).abs() < 1e-9, "{lecture:?}");
            assert!((b.w_aux_pro_wh - w_aux).abs() < 1e-9, "{lecture:?}");
            // Éq. (1216).
            let cons = (15_000.0 + q_prelec + q_pertes) / 1.11;
            assert!((b.q_cons_wh - cons).abs() < 1e-9, "{lecture:?}");
            assert!((b.eta_eff - 15_000.0 / cons).abs() < 1e-12);
        }
        // À τ = 1 : fx = 1 dans la lecture retenue comme dans la variante.
        let mut g = mci_avec_appoint();
        appel(&mut g, Poste::Chauffage, 20_000.0, 53.5, 1.0);
        let b = g.bilan_du_pas(20.0).unwrap();
        assert!((b.q_prelec_wh - prelec_1).abs() < 1e-9);
        assert!((b.q_pertes_wh - phi_1).abs() < 1e-9);
    }

    /// Variante imprimée du module seul (éq. 1211) : fx = Pmax,chp/Pmax · τ.
    #[test]
    fn variante_litterale_du_module_seul() {
        let mut g = mci();
        g.lecture = LectureInterpolation::Litterale;
        appel(&mut g, Poste::Chauffage, 8000.0, 53.5, 1.0);
        let b = g.bilan_du_pas(20.0).unwrap();
        // Sans appoint, Pmax,chp = Pmax : fx = 1 × 0,8.
        let fx = 0.8;
        let phi = |p: f64| (100.0 / 72.0 - 1.0) * p;
        let q_pertes = fx * phi(10_000.0) + (1.0 - fx) * phi(6000.0);
        assert!((b.q_pertes_wh - q_pertes).abs() < 1e-9);
    }

    /// Rdim = 2 : chaque module reçoit la moitié de la demande, les sorties sont doublées.
    #[test]
    fn rdim_superieur_a_un() {
        let mut un = mci();
        appel(&mut un, Poste::Chauffage, 8000.0, 53.5, 1.0);
        let b1 = un.bilan_du_pas(20.0).unwrap();
        let mut deux = mci();
        deux.rdim = 2.0;
        let s = appel(&mut deux, Poste::Chauffage, 16_000.0, 53.5, 1.0);
        assert!((s.q_fou_wh - 16_000.0).abs() < 1e-9);
        assert!((s.q_rest_wh).abs() < 1e-9);
        let b2 = deux.bilan_du_pas(20.0).unwrap();
        assert!((b2.q_cons_wh - 2.0 * b1.q_cons_wh).abs() < 1e-9);
        assert!((b2.q_prelec_wh - 2.0 * b1.q_prelec_wh).abs() < 1e-9);
        assert!((b2.w_aux_pro_wh - 2.0 * b1.w_aux_pro_wh).abs() < 1e-9);
        assert!((deux.pn_gen_tot_w() - 20_000.0).abs() < 1e-9);
    }

    /// Rendement thermique corrigé nul ou négatif (condensation, « αth » = 0,2, température
    /// aval très élevée) : borné à zéro, le module ne fournit rien et tout reste fini (Q157).
    #[test]
    fn rendement_thermique_corrige_borne() {
        let mut g = mci();
        g.alpha_th = 0.2;
        let s = appel(
            &mut g,
            Poste::Chauffage,
            5000.0,
            53.5 + 52.8 / 0.2 + 10.0,
            1.0,
        );
        assert_eq!(s.q_fou_wh, 0.0);
        assert_eq!(s.q_rest_wh, 5000.0);
        let b = g.bilan_du_pas(20.0).unwrap();
        for x in [
            b.q_cons_wh,
            b.q_prelec_wh,
            b.q_pertes_wh,
            b.w_aux_pro_wh,
            b.phi_vc_wh,
            b.eta_eff,
        ] {
            assert!(x.is_finite(), "{b:?}");
        }
    }
}
