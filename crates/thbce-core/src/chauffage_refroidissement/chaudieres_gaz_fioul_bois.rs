// SPDX-License-Identifier: GPL-3.0-or-later
//! Chaudières au gaz, au fioul et au bois : fiche `C_GEN_Chaudières_gaz_fioul_bois`
//! (Th-BCE 2020 §8.19), types 100 à 102, 200, 201 et 400 du tableau 111.
//!
//! Le générateur est décrit par ses rendements sur PCS à pleine charge et à 30 % de charge, ses
//! pertes à l'arrêt et ses auxiliaires (valeurs par défaut des éq. 1072 à 1077). Au pas de
//! temps, l'ECS est servie d'abord (éq. 1081 à 1098) puis le chauffage sur la puissance restante
//! (éq. 1099 à 1109). La consommation est exprimée en énergie finale sur PCI (éq. 1098), les
//! auxiliaires en électricité. Lectures : `docs/questions.md`, Q49.

use super::calculs_generation::{Energie, Poste};
use super::generation::{EntreesAppel, SortiesAppel};

/// Température maximale de fonctionnement conventionnelle, en °C (§8.19.3.1).
pub const THETA_FONCT_MAX_C: f64 = 100.0;
/// « Wveille » par défaut, en W (éq. 1075).
pub const W_VEILLE_DEFAUT_W: f64 = 20.0;
/// Part de la puissance nominale à laquelle est définie la puissance intermédiaire (« Pint :
/// puissance thermique utile à 30 % de charge »).
pub const TAUX_CHARGE_INTERMEDIAIRE: f64 = 0.3;

/// Combustible bois d'une chaudière (« TypeCombustibleBois »).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombustibleBois {
    /// 1 : bûches.
    Buches,
    /// 2 : granulés.
    Granules,
    /// 3 : plaquettes.
    Plaquettes,
}

/// « Alim_Chaudière_Bois » : mode de chargement et de tirage d'une chaudière bois (tableau 119).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlimentationBois {
    /// 0 : chargement manuel, tirage naturel.
    ManuelTirageNaturel,
    /// 1 : chargement manuel, air pulsé.
    ManuelAirPulse,
    /// 2 : chargement automatique, tirage naturel.
    AutomatiqueTirageNaturel,
    /// 3 : chargement automatique, air pulsé.
    AutomatiqueAirPulse,
}

/// Déclinaison d'une chaudière (tableau 115).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeChaudiere {
    /// Type 100 : chaudière standard au gaz.
    GazStandard,
    /// Type 101 : chaudière basse température au gaz.
    GazBasseTemperature,
    /// Type 102 : chaudière à condensation au gaz.
    GazCondensation,
    /// Type 200 : chaudière standard au fioul.
    FioulStandard,
    /// Type 201 : chaudière à condensation au fioul.
    FioulCondensation,
    /// Type 400 : chaudière au bois.
    Bois(CombustibleBois),
}

impl TypeChaudiere {
    /// « idengen » : énergie principale.
    pub const fn energie(self) -> Energie {
        match self {
            Self::GazStandard | Self::GazBasseTemperature | Self::GazCondensation => Energie::Gaz,
            Self::FioulStandard | Self::FioulCondensation => Energie::Fioul,
            Self::Bois(_) => Energie::Bois,
        }
    }

    const fn condensation(self) -> bool {
        matches!(self, Self::GazCondensation | Self::FioulCondensation)
    }

    const fn bois(self) -> bool {
        matches!(self, Self::Bois(_))
    }

    /// « θfonct_min » : température minimale de fonctionnement conventionnelle, en °C
    /// (§8.19.3.1).
    pub const fn theta_fonct_min_c(self) -> f64 {
        if self.bois() {
            70.0
        } else if self.condensation() {
            30.0
        } else {
            40.0
        }
    }

    /// Paramètres de la correction des rendements (tableau 120) : `(αth,nom, θav,ref,nom,
    /// αth,int, θav,ref,int)`.
    const fn correction(self) -> (f64, f64, f64, f64) {
        if self.bois() {
            (0.0, 70.0, 0.0, 70.0)
        } else if self.condensation() {
            (0.2, 70.0, 0.2, 33.0)
        } else {
            (0.04, 70.0, 0.05, 40.0)
        }
    }
}

/// « PCSI » : rapport du PCS au PCI du combustible (tableau 116).
pub const fn pcsi(energie: Energie, gpl: bool) -> f64 {
    match energie {
        Energie::Gaz => {
            if gpl {
                1.09
            } else {
                1.11
            }
        }
        Energie::Fioul => 1.07,
        _ => 1.08,
    }
}

/// Valeurs par défaut calculées pour une chaudière (§8.19.3.1.4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValeursDefaut {
    /// « R_pn » sur PCS, en %.
    pub r_pn_pct: f64,
    /// « R_pint » sur PCS, en %.
    pub r_pint_pct: f64,
    /// « Q_po_30 », en W.
    pub q_po_30_w: f64,
    /// « Waux_nom », en W.
    pub w_aux_nom_w: f64,
    /// « Waux_int », en W.
    pub w_aux_int_w: f64,
}

/// Valeurs par défaut des rendements, des pertes à l'arrêt et des auxiliaires (éq. 1072 à 1077,
/// tableaux 118 et 119). `p_int_kw` est la puissance à 30 % de charge. Lecture Q49 : `log` est
/// le logarithme décimal ; les rendements restent en % (le « /100 » des éq. 1072 et 1073 est
/// omis) et `c5` est un pourcentage de la puissance nominale (« /100 » ajouté à l'éq. 1074).
pub fn valeurs_par_defaut(
    t: TypeChaudiere,
    alim: Option<AlimentationBois>,
    pn_kw: f64,
    p_int_kw: f64,
    gpl: bool,
) -> ValeursDefaut {
    let pcsi = pcsi(t.energie(), gpl);
    // Tableau 118.
    let (c1, c2, c3, c4, c5, c6) = match t {
        TypeChaudiere::Bois(CombustibleBois::Buches) => (89.0, 2.0, 84.0, 2.0, 14.0, -0.28),
        TypeChaudiere::Bois(_) => (91.0, 2.0, 88.0, 2.0, 14.0, -0.28),
        _ if pn_kw <= 400.0 => (94.0, 1.0, 103.0, 1.0, 4.0, -0.4),
        _ => (96.6, 0.0, 105.6, 0.0, 4.0, 0.0),
    };
    // Tableau 119.
    let (c7, c8, c9, c10, n) = if t.bois() {
        match alim.unwrap_or(AlimentationBois::ManuelTirageNaturel) {
            AlimentationBois::ManuelTirageNaturel => (0.0, 0.0, 0.0, 0.0, 1.0),
            AlimentationBois::ManuelAirPulse => (74.0, 0.5, 74.0, 0.5, 1.0),
            AlimentationBois::AutomatiqueTirageNaturel => (0.0, 10.0, 0.0, 10.0, 1.0),
            AlimentationBois::AutomatiqueAirPulse => (74.0, 10.5, 74.0, 10.5, 1.0),
        }
    } else {
        (0.0, 45.0, 0.0, 15.0, 0.48)
    };
    let log_pn = libm::log10(pn_kw.max(f64::MIN_POSITIVE));
    ValeursDefaut {
        // Éq. (1072) et (1073).
        r_pn_pct: (c1 + c2 * log_pn) / pcsi,
        r_pint_pct: (c3 + c4 * log_pn) / pcsi,
        // Éq. (1074).
        q_po_30_w: 1000.0 * (c5 / 100.0) * libm::pow(pn_kw, c6) * pn_kw,
        // Éq. (1076) et (1077).
        w_aux_nom_w: c7 + c8 * libm::pow(pn_kw, n),
        w_aux_int_w: c9 + c10 * libm::pow(p_int_kw, n),
    }
}

/// État d'un pas de temps, par chaudière identique.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct EtatPas {
    q_fou_ch_wh: f64,
    q_fou_ecs_wh: f64,
    r_fonct_ecs: f64,
    theta_cr_ch_c: f64,
    theta_cr_ecs_c: f64,
}

/// Chaudière au gaz, au fioul ou au bois.
#[derive(Debug, Clone, PartialEq)]
pub struct Chaudiere {
    /// Déclinaison.
    pub type_chaudiere: TypeChaudiere,
    /// Vrai pour un gaz de pétrole liquéfié (PCSI de 1,09).
    pub gpl: bool,
    /// « idfougen » : le générateur assure le chauffage.
    pub chauffage: bool,
    /// « idfougen » : le générateur assure l'ECS.
    pub ecs: bool,
    /// « Rdim » : nombre de générateurs identiques.
    pub rdim: f64,
    /// « idpos_gen » : vrai en volume chauffé.
    pub en_volume_chauffe: bool,
    /// « Pn_gen » : puissance thermique utile nominale, en kW.
    pub pn_gen_kw: f64,
    /// « R_pn » sur PCS, en %, après correction de statut.
    pub r_pn_pct: f64,
    /// « Pint », en kW.
    pub p_int_kw: f64,
    /// « R_pint » sur PCS, en %, après correction de statut.
    pub r_pint_pct: f64,
    /// « Waux_nom », en W.
    pub w_aux_nom_w: f64,
    /// « Waux_int », en W.
    pub w_aux_int_w: f64,
    /// « Wveille », en W.
    pub w_veille_w: f64,
    /// « Q_po_30 », en W.
    pub q_po_30_w: f64,
    /// « p_Qp.g_arret ».
    pub p_qp_arret: f64,
    prec: EtatPas,
    cour: EtatPas,
}

impl Chaudiere {
    /// Chaudière décrite par ses grandeurs résolues.
    #[allow(clippy::too_many_arguments)]
    pub fn nouvelle(
        type_chaudiere: TypeChaudiere,
        gpl: bool,
        chauffage: bool,
        ecs: bool,
        rdim: f64,
        en_volume_chauffe: bool,
        pn_gen_kw: f64,
        p_int_kw: f64,
        rendements_pct: (f64, f64),
        auxiliaires_w: (f64, f64, f64),
        q_po_30_w: f64,
        p_qp_arret: f64,
    ) -> Self {
        Self {
            type_chaudiere,
            gpl,
            chauffage,
            ecs,
            rdim,
            en_volume_chauffe,
            pn_gen_kw,
            r_pn_pct: rendements_pct.0,
            p_int_kw,
            r_pint_pct: rendements_pct.1,
            w_aux_nom_w: auxiliaires_w.0,
            w_aux_int_w: auxiliaires_w.1,
            w_veille_w: auxiliaires_w.2,
            q_po_30_w,
            p_qp_arret,
            prec: EtatPas::default(),
            cour: EtatPas::default(),
        }
    }

    /// « Rdim × Pn_gen », en W (clé de répartition de l'éq. 1017).
    pub fn pn_gen_tot_w(&self) -> f64 {
        self.rdim * 1000.0 * self.pn_gen_kw
    }

    fn pcsi(&self) -> f64 {
        pcsi(self.type_chaudiere.energie(), self.gpl)
    }

    /// « R_pn(θaval) », en % (éq. 1078).
    fn r_pn_theta(&self, theta_c: f64) -> f64 {
        let (a, r, _, _) = self.type_chaudiere.correction();
        self.r_pn_pct + a * (r - theta_c)
    }

    /// « R_pint(θaval) », en % (éq. 1079).
    fn r_pint_theta(&self, theta_c: f64) -> f64 {
        let (_, _, a, r) = self.type_chaudiere.correction();
        self.r_pint_pct + a * (r - theta_c)
    }

    /// « φthstab(θaval) », en W (éq. 1080).
    fn phi_th_stab_w(&self, theta_c: f64, theta_amb_c: f64) -> f64 {
        let x = ((theta_c - theta_amb_c) / 30.0).max(0.0);
        self.q_po_30_w * libm::pow(x, 1.25)
    }

    /// Fin du pas de temps : les grandeurs du pas deviennent celles du pas précédent.
    pub fn fin_de_pas(&mut self) {
        self.prec = self.cour;
        self.cour = EtatPas::default();
    }

    /// Appel du générateur pour un poste (éq. 1081 à 1118).
    pub fn appel(&mut self, e: &EntreesAppel) -> SortiesAppel {
        let ecs = e.fonction == Poste::Ecs;
        let theta_cr = e.theta_aval_c.max(self.type_chaudiere.theta_fonct_min_c());
        // Éq. (1082) et (1100) : puissances aux températures du pas de temps, en W.
        let p_nom = 1000.0 * self.r_pn_theta(theta_cr) / self.r_pn_pct * self.pn_gen_kw;
        let p_int = 1000.0 * self.r_pint_theta(theta_cr) / self.r_pint_pct * self.p_int_kw;
        let r_puis = if ecs { 1.0 } else { e.r_pui_dispo };
        let q_req = e.q_req_wh.max(0.0) / self.rdim;
        // Éq. (1081) et (1099).
        let q_fou = (r_puis * p_nom).min(q_req).max(0.0);
        let (r_fonct, tau_poste) = if ecs {
            let r = if q_fou > 0.0 { q_fou / p_nom } else { 0.0 };
            (r, r)
        } else {
            let d = r_puis * p_nom;
            (0.0, if d > 0.0 { q_fou / d } else { 0.0 })
        };
        let (q_pertes, w_aux, fonct_hors_poste) = if q_fou > 0.0 {
            // Pertes à charge intermédiaire et nominale (éq. 1087 et 1088).
            let phi_int = (100.0 / self.r_pint_theta(theta_cr) - 1.0) * p_int;
            let phi_nom = (100.0 / self.r_pn_theta(theta_cr) - 1.0) * p_nom;
            let phi_stab = self.phi_th_stab_w(theta_cr, e.theta_amb_c);
            if q_fou <= p_int {
                // Cycles tout ou rien (éq. 1092 à 1094).
                let fx = q_fou / p_int;
                (
                    fx * phi_int + (1.0 - fx) * phi_stab,
                    fx * self.w_aux_int_w + (1.0 - fx) * self.w_veille_w,
                    false,
                )
            } else {
                // Modulation de l'allure du brûleur (éq. 1095 à 1097).
                let fx = (q_fou - p_int) / (p_nom - p_int);
                (
                    fx * phi_nom + (1.0 - fx) * phi_int,
                    fx * self.w_aux_nom_w + (1.0 - fx) * self.w_aux_int_w,
                    false,
                )
            }
        } else if ecs {
            // Éq. (1089) à (1091) : à l'arrêt, seulement si le générateur ne produit que de
            // l'ECS ; les pertes dépendent du dernier fonctionnement en ECS.
            if e.ecs_seule {
                (
                    self.prec.r_fonct_ecs
                        * self.phi_th_stab_w(self.prec.theta_cr_ecs_c, e.theta_amb_c),
                    self.w_veille_w,
                    false,
                )
            } else {
                (0.0, 0.0, false)
            }
        } else {
            // Éq. (1104) à (1109).
            let q = if r_puis < 1.0 {
                r_puis * self.phi_th_stab_w(self.cour.theta_cr_ecs_c, e.theta_amb_c)
            } else if self.prec.q_fou_ch_wh > 0.0 {
                r_puis * self.phi_th_stab_w(self.prec.theta_cr_ch_c, e.theta_amb_c)
            } else if self.prec.q_fou_ecs_wh > 0.0 {
                self.prec.r_fonct_ecs * self.phi_th_stab_w(self.prec.theta_cr_ecs_c, e.theta_amb_c)
            } else {
                0.0
            };
            (q, self.w_veille_w, self.cour.q_fou_ecs_wh > 0.0)
        };
        // Éq. (1098) ; à l'arrêt, éq. (1090) et (1108) : la consommation est la perte (Q49).
        let q_cons = if q_fou > 0.0 {
            (q_fou + q_pertes) / self.pcsi()
        } else {
            q_pertes
        };
        if ecs {
            self.cour.q_fou_ecs_wh = q_fou;
            self.cour.r_fonct_ecs = r_fonct;
            self.cour.theta_cr_ecs_c = theta_cr;
        } else {
            self.cour.q_fou_ch_wh = q_fou;
            self.cour.theta_cr_ch_c = theta_cr;
        }
        // Éq. (1118) : pertes par les parois et auxiliaires récupérables en volume chauffé.
        let fonctionne = q_fou > 0.0 || fonct_hors_poste;
        let p_qp = if fonctionne {
            if self.type_chaudiere.bois() {
                0.25
            } else {
                0.30
            }
        } else {
            self.p_qp_arret
        };
        let phi_vc = if self.en_volume_chauffe {
            p_qp * q_pertes + w_aux
        } else {
            0.0
        };
        let rdim = self.rdim;
        let q_fou_tot = q_fou * rdim;
        SortiesAppel {
            energie: self.type_chaudiere.energie(),
            // Éq. (1117) : combustible au poste servi ; auxiliaires en électricité.
            q_cef_wh: q_cons * rdim,
            q_cef_aux_elec_wh: w_aux * rdim,
            q_cons_wh: q_cons * rdim,
            q_fou_wh: q_fou_tot,
            // Éq. (1085) et (1103) ramenées à l'ensemble des chaudières identiques.
            q_rest_wh: e.q_req_wh.max(0.0) - q_fou_tot,
            tau_charge: tau_poste,
            phi_vc_wh: phi_vc * rdim,
            w_aux_pro_wh: w_aux * rdim,
            // Éq. (1116).
            eta_eff: if q_cons > 0.0 { q_fou / q_cons } else { 0.0 },
            r_fonct_ecs: r_fonct,
            ..SortiesAppel::default()
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn gaz_condensation() -> Chaudiere {
        // 20 kW, valeurs par défaut.
        let pn = 20.0;
        let pint = 0.3 * pn;
        let t = TypeChaudiere::GazCondensation;
        let v = valeurs_par_defaut(t, None, pn, pint, false);
        Chaudiere::nouvelle(
            t,
            false,
            true,
            true,
            1.0,
            true,
            pn,
            pint,
            (v.r_pn_pct, v.r_pint_pct),
            (v.w_aux_nom_w, v.w_aux_int_w, W_VEILLE_DEFAUT_W),
            v.q_po_30_w,
            0.5,
        )
    }

    fn appel(
        c: &mut Chaudiere,
        poste: Poste,
        q: f64,
        theta: f64,
        r_pui: f64,
        ecs_seule: bool,
    ) -> SortiesAppel {
        c.appel(&EntreesAppel {
            fonction: poste,
            q_req_wh: q,
            theta_amont_c: 0.0,
            theta_aval_c: theta,
            r_pui_dispo: r_pui,
            ecs_seule,
            theta_amb_c: 20.0,
            air_extrait_w_k: None,
        })
    }

    #[test]
    fn valeurs_par_defaut_a_la_main() {
        let v = valeurs_par_defaut(TypeChaudiere::GazCondensation, None, 20.0, 6.0, false);
        // (94 + log10 20) / 1,11.
        assert!((v.r_pn_pct - (94.0 + libm::log10(20.0)) / 1.11).abs() < 1e-12);
        assert!((v.r_pint_pct - (103.0 + libm::log10(20.0)) / 1.11).abs() < 1e-12);
        // 1000 × 0,04 × 20^−0,4 × 20.
        assert!((v.q_po_30_w - 40.0 * libm::pow(20.0, -0.4) * 20.0).abs() < 1e-9);
        assert!((v.w_aux_nom_w - 45.0 * libm::pow(20.0, 0.48)).abs() < 1e-9);
        assert!((v.w_aux_int_w - 15.0 * libm::pow(6.0, 0.48)).abs() < 1e-9);
        let b = valeurs_par_defaut(
            TypeChaudiere::Bois(CombustibleBois::Granules),
            Some(AlimentationBois::AutomatiqueAirPulse),
            20.0,
            6.0,
            false,
        );
        assert!((b.r_pn_pct - (91.0 + 2.0 * libm::log10(20.0)) / 1.08).abs() < 1e-12);
        assert!((b.w_aux_nom_w - (74.0 + 10.5 * 20.0)).abs() < 1e-9);
    }

    #[test]
    fn rendement_a_charge_nominale_a_la_main() {
        let mut c = gaz_condensation();
        // θaval = 50 °C : R_pn(50) = R_pn + 0,2 × (70 − 50).
        let r50 = c.r_pn_pct + 4.0;
        let p_nom = 1000.0 * r50 / c.r_pn_pct * 20.0;
        let s = appel(&mut c, Poste::Chauffage, 1e9, 50.0, 1.0, false);
        assert!((s.q_fou_wh - p_nom).abs() < 1e-6);
        assert!((s.tau_charge - 1.0).abs() < 1e-12);
        // Pertes à pleine charge : (100/R − 1) × P ; consommation sur PCI.
        let pertes = (100.0 / r50 - 1.0) * p_nom;
        // Plage de modulation, fx = 1 : pertes nominales.
        assert!((s.q_cons_wh - (p_nom + pertes) / 1.11).abs() < 1e-6);
        assert!((s.q_cef_aux_elec_wh - c.w_aux_nom_w).abs() < 1e-9);
        assert_eq!(s.energie, Energie::Gaz);
    }

    #[test]
    fn cycles_tout_ou_rien_et_arret() {
        let mut c = gaz_condensation();
        // Demande de 1 kWh à 40 °C : sous Pint, interpolation entre pertes intermédiaires et à
        // l'arrêt.
        let s = appel(&mut c, Poste::Chauffage, 1000.0, 40.0, 1.0, false);
        assert!((s.q_fou_wh - 1000.0).abs() < 1e-9);
        let r_int = c.r_pint_pct + 0.2 * (33.0 - 40.0);
        let r_nom = c.r_pn_pct + 0.2 * 30.0;
        let p_int = 1000.0 * r_int / c.r_pint_pct * 6.0;
        let fx = 1000.0 / p_int;
        let phi_int = (100.0 / r_int - 1.0) * p_int;
        let stab = c.q_po_30_w * libm::pow(20.0 / 30.0, 1.25);
        let pertes = fx * phi_int + (1.0 - fx) * stab;
        assert!((s.q_cons_wh - (1000.0 + pertes) / 1.11).abs() < 1e-6);
        let _ = r_nom;
        // Pas suivant sans demande : pertes à l'arrêt de la température du pas précédent.
        c.fin_de_pas();
        let s = appel(&mut c, Poste::Chauffage, 0.0, 40.0, 1.0, false);
        assert_eq!(s.q_fou_wh, 0.0);
        assert!((s.q_cons_wh - stab).abs() < 1e-9);
        assert!((s.w_aux_pro_wh - 20.0).abs() < 1e-12);
        // p_Qp.g_arret = 0,5 : moitié des pertes plus les auxiliaires.
        assert!((s.phi_vc_wh - (0.5 * stab + 20.0)).abs() < 1e-9);
        // Un pas de plus : plus de pertes.
        c.fin_de_pas();
        let s = appel(&mut c, Poste::Chauffage, 0.0, 40.0, 1.0, false);
        assert_eq!(s.q_cons_wh, 0.0);
    }

    #[test]
    fn ecs_prioritaire_et_puissance_restante() {
        let mut c = gaz_condensation();
        let s = appel(&mut c, Poste::Ecs, 5000.0, 60.0, 1.0, false);
        let p_nom_ecs = 1000.0 * (c.r_pn_pct + 0.2 * 10.0) / c.r_pn_pct * 20.0;
        assert!((s.r_fonct_ecs - 5000.0 / p_nom_ecs).abs() < 1e-12);
        let s2 = appel(
            &mut c,
            Poste::Chauffage,
            1e9,
            40.0,
            1.0 - s.r_fonct_ecs,
            false,
        );
        let p_nom_ch = 1000.0 * (c.r_pn_pct + 0.2 * 30.0) / c.r_pn_pct * 20.0;
        assert!((s2.q_fou_wh - (1.0 - s.r_fonct_ecs) * p_nom_ch).abs() < 1e-6);
        // Deux chaudières identiques : la demande est partagée, les sorties multipliées.
        let mut c2 = gaz_condensation();
        c2.rdim = 2.0;
        let mut c1 = gaz_condensation();
        let a = appel(&mut c1, Poste::Chauffage, 1000.0, 40.0, 1.0, false);
        let b = appel(&mut c2, Poste::Chauffage, 2000.0, 40.0, 1.0, false);
        assert!((b.q_cons_wh - 2.0 * a.q_cons_wh).abs() < 1e-9);
        assert!((b.q_fou_wh - 2000.0).abs() < 1e-9);
    }
}
