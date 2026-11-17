// SPDX-License-Identifier: GPL-3.0-or-later
//! Sous-stations de réseaux de chaleur et de froid : fiche `C_gen_Réseau de chaleur et de
//! froid` (Th-BCE 2020 §8.28), types 600 et 601 du tableau 111. Seules les pertes de la
//! sous-station sont prises en compte (éq. 1467 à 1483). Lectures : `docs/questions.md`, Q49.

use super::calculs_generation::{Energie, Poste};
use super::generation::{EntreesAppel, SortiesAppel};

/// Type de réseau de chaleur (tableaux 247 et 248).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeReseauChaleur {
    /// Eau chaude basse température.
    EauChaudeBasseTemperature,
    /// Eau chaude haute température.
    EauChaudeHauteTemperature,
    /// Vapeur basse pression.
    VapeurBassePression,
    /// Vapeur haute pression.
    VapeurHautePression,
}

impl TypeReseauChaleur {
    /// « θprs » (°C) et « Dss » (tableau 248).
    pub const fn theta_prs_et_dss(self) -> (f64, f64) {
        match self {
            Self::EauChaudeBasseTemperature => (105.0, 0.6),
            Self::EauChaudeHauteTemperature => (150.0, 0.4),
            Self::VapeurBassePression => (110.0, 0.5),
            Self::VapeurHautePression => (180.0, 0.4),
        }
    }

    /// « Bss » (tableau 247) pour la classe d'isolation du réseau secondaire, de 1 à 4.
    pub fn b_ss(self, classe_secondaire: u8) -> f64 {
        let ligne = match self {
            Self::EauChaudeBasseTemperature => [3.5, 4.0, 4.4, 4.9],
            Self::EauChaudeHauteTemperature => [3.1, 3.5, 3.9, 4.3],
            Self::VapeurBassePression => [2.8, 3.2, 3.5, 3.9],
            Self::VapeurHautePression => [2.6, 3.0, 3.3, 3.7],
        };
        // Le tableau est ordonné de la classe 4 (la plus isolée) à la classe 1.
        ligne[usize::from(4 - classe_secondaire.clamp(1, 4))]
    }
}

/// Lecture de l'éq. (1473).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LectureHss {
    /// `PEss` en kW divisé par 1000, comme l'écrit le texte (retenue).
    Litterale,
    /// `PEss` converti en W puis divisé par 1000 (variante conservée, non exposée).
    PuissanceEnWatts,
}

/// Sous-station d'un réseau de chaleur ou de froid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReseauChaleurFroid {
    /// Réseau de froid (type 601) ; faux pour un réseau de chaleur (type 600).
    pub froid: bool,
    /// « idfougen » : assure le chauffage (réseau de chaleur).
    pub chauffage: bool,
    /// « idfougen » : assure l'ECS (réseau de chaleur).
    pub ecs: bool,
    /// « Rdim » : nombre de sous-stations identiques.
    pub rdim: f64,
    /// « idpos » : vrai en volume chauffé (sans effet : éq. 1480).
    pub en_volume_chauffe: bool,
    /// « PEss » : puissance d'échange d'une sous-station, en kW.
    pub p_ess_kw: f64,
    /// « Bss ».
    pub b_ss: f64,
    /// « Dss ».
    pub d_ss: f64,
    /// « θprs » : température primaire, en °C.
    pub theta_prs_c: f64,
    /// Lecture de l'éq. (1473).
    pub lecture_hss: LectureHss,
}

impl ReseauChaleurFroid {
    /// « Rdim × PEss », en W.
    pub fn pn_gen_tot_w(&self) -> f64 {
        self.rdim * 1000.0 * self.p_ess_kw
    }

    /// « Hss », en W/K (éq. 1473).
    pub fn h_ss_w_k(&self) -> f64 {
        let p = match self.lecture_hss {
            LectureHss::Litterale => self.p_ess_kw,
            LectureHss::PuissanceEnWatts => 1000.0 * self.p_ess_kw,
        };
        self.b_ss * libm::cbrt(p / 1000.0)
    }

    /// Appel du générateur (éq. 1467 à 1483).
    pub fn appel(&self, e: &EntreesAppel) -> SortiesAppel {
        let ecs = e.fonction == Poste::Ecs;
        let r_puis = if ecs { 1.0 } else { e.r_pui_dispo };
        let p_nom = 1000.0 * self.p_ess_kw;
        // Éq. (1467) et (1468).
        let q_req_act = e.q_req_wh.max(0.0) / self.rdim;
        let q_fou_act = q_req_act.min(r_puis * p_nom);
        // Éq. (1470).
        let tau = if r_puis * p_nom > 0.0 {
            q_fou_act / (r_puis * p_nom)
        } else {
            0.0
        };
        // Éq. (1474) et (1475).
        let theta_ss = self.d_ss * self.theta_prs_c + (1.0 - self.d_ss) * e.theta_aval_c;
        let phi_ss = self.h_ss_w_k() * (theta_ss - e.theta_amb_c);
        // Éq. (1471), (1481) à (1483).
        let q_ss_act = if self.froid {
            0.0
        } else if ecs {
            if e.ecs_seule {
                phi_ss
            } else {
                tau * phi_ss
            }
        } else {
            r_puis * phi_ss
        };
        // Éq. (1476).
        let q_cons_act = q_fou_act + q_ss_act;
        let rdim = self.rdim;
        SortiesAppel {
            energie: Energie::Reseau,
            q_cef_wh: rdim * q_cons_act,
            q_cons_wh: rdim * q_cons_act,
            // Éq. (1469) et (1478).
            q_fou_wh: rdim * q_fou_act,
            q_rest_wh: e.q_req_wh.max(0.0) - rdim * q_fou_act,
            tau_charge: tau,
            // Éq. (1479) et (1480).
            phi_vc_wh: 0.0,
            w_aux_pro_wh: 0.0,
            // Éq. (1477).
            eta_eff: if q_cons_act > 0.0 {
                q_fou_act / q_cons_act
            } else {
                0.0
            },
            // Éq. (1481).
            r_fonct_ecs: if ecs { tau } else { 0.0 },
            ..SortiesAppel::default()
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn reseau(froid: bool) -> ReseauChaleurFroid {
        let t = TypeReseauChaleur::EauChaudeBasseTemperature;
        let (theta_prs_c, d_ss) = t.theta_prs_et_dss();
        ReseauChaleurFroid {
            froid,
            chauffage: !froid,
            ecs: false,
            rdim: 1.0,
            en_volume_chauffe: true,
            p_ess_kw: 27.0,
            b_ss: t.b_ss(4),
            d_ss,
            theta_prs_c,
            lecture_hss: LectureHss::Litterale,
        }
    }

    fn entree(poste: Poste, q: f64, theta: f64, r_pui: f64, ecs_seule: bool) -> EntreesAppel {
        EntreesAppel {
            fonction: poste,
            q_req_wh: q,
            theta_amont_c: 0.0,
            theta_aval_c: theta,
            r_pui_dispo: r_pui,
            ecs_seule,
            theta_amb_c: 20.0,
            air_extrait_w_k: None,
        }
    }

    #[test]
    fn tableaux_247_et_248() {
        use TypeReseauChaleur::*;
        assert_eq!(EauChaudeBasseTemperature.b_ss(4), 3.5);
        assert_eq!(EauChaudeBasseTemperature.b_ss(1), 4.9);
        assert_eq!(VapeurHautePression.b_ss(2), 3.3);
        assert_eq!(VapeurBassePression.theta_prs_et_dss(), (110.0, 0.5));
    }

    #[test]
    fn pertes_de_sous_station_a_la_main() {
        let r = reseau(false);
        // Hss = 3,5 × (27/1000)^(1/3) = 3,5 × 0,3 = 1,05 W/K.
        assert!((r.h_ss_w_k() - 1.05).abs() < 1e-12);
        // θss = 0,6 × 105 + 0,4 × 50 = 83 °C ; pertes 1,05 × 63 = 66,15 Wh.
        let s = r.appel(&entree(Poste::Chauffage, 5000.0, 50.0, 1.0, false));
        assert_eq!(s.q_fou_wh, 5000.0);
        assert!((s.q_cons_wh - 5066.15).abs() < 1e-9);
        assert_eq!(s.energie, Energie::Reseau);
        assert_eq!(s.phi_vc_wh, 0.0);
        // Sans demande, les pertes subsistent ; réduites par la puissance disponible.
        let s = r.appel(&entree(Poste::Chauffage, 0.0, 50.0, 0.5, false));
        assert!((s.q_cons_wh - 33.075).abs() < 1e-9);
        // Plafond à 27 kW.
        let s = r.appel(&entree(Poste::Chauffage, 1e6, 50.0, 1.0, false));
        assert_eq!(s.q_fou_wh, 27000.0);
        assert_eq!(s.tau_charge, 1.0);
    }

    #[test]
    fn ecs_et_froid() {
        let r = reseau(false);
        // ECS seule : pertes entières ; sinon au prorata du temps de fonctionnement.
        let a = r.appel(&entree(Poste::Ecs, 2700.0, 60.0, 1.0, true));
        let b = r.appel(&entree(Poste::Ecs, 2700.0, 60.0, 1.0, false));
        let h = r.h_ss_w_k() * (0.6 * 105.0 + 0.4 * 60.0 - 20.0);
        assert!((a.q_cons_wh - 2700.0 - h).abs() < 1e-9);
        assert!((b.q_cons_wh - 2700.0 - 0.1 * h).abs() < 1e-9);
        assert!((b.r_fonct_ecs - 0.1).abs() < 1e-12);
        // Froid : pas de pertes.
        let f = reseau(true).appel(&entree(Poste::Refroidissement, 3000.0, 7.0, 1.0, false));
        assert_eq!(f.q_cons_wh, 3000.0);
        assert_eq!(f.eta_eff, 1.0);
    }
}
