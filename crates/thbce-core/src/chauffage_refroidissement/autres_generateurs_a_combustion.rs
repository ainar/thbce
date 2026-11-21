// SPDX-License-Identifier: GPL-3.0-or-later
//! Autres générateurs à combustion : fiche `C_GEN_AUTRES_GENERATEURS_A_COMBUSTION`
//! (Th-BCE 2020 §8.20), types 103 à 109 du tableau 111 : radiateurs, poêles et inserts au gaz,
//! chauffe-eau et accumulateurs au gaz, générateurs d'air chaud, tubes et panneaux radiants.
//!
//! Les rendements sont saisis sur PCI (« Rpn », « Rpint », éq. 1120). En chauffage, les
//! générateurs d'air chaud suivent le calcul des chaudières (§8.20.3.6.1.3 et 8.20.3.6.1.4, renvoi
//! à la fiche 8.19 : cycles tout ou rien puis modulation, avec les rendements ramenés au PCS) ;
//! les radiateurs et les appareils radiants ont des pertes proportionnelles à l'énergie fournie
//! (éq. 1134). En ECS (chauffe-eau, accumulateurs), le générateur fonctionne à pleine charge
//! pendant le temps `Rfonctecs` (éq. 1138 à 1146). À l'arrêt, les pertes dépendent du
//! raccordement du générateur (« idraccord_gnr », éq. 1150 à 1163). Lectures :
//! `docs/questions/Q155.md` et `docs/questions/Q156.md`.

use super::calculs_generation::{Energie, Poste};
use super::chaudieres_gaz_fioul_bois::pcsi;
use super::generation::{EntreesAppel, SortiesAppel};

/// « θfonct_min » par défaut d'un générateur sur eau, en °C (§8.20.3.2.2).
pub const THETA_FONCT_MIN_DEFAUT_C: f64 = 20.0;
/// « Rpn utile max » d'une valeur déclarée, en % (éq. 1129 et 1132).
pub const R_PN_UTILE_MAX_PCT: f64 = 86.0;
/// « Rpint utile max » d'une valeur déclarée, en % (éq. 1130).
pub const R_PINT_UTILE_MAX_PCT: f64 = 81.0;
/// Diviseur de la puissance de la veilleuse ajoutée à la consommation de veille (§8.20.3.4).
pub const DIVISEUR_VEILLEUSE: f64 = 2.58;
/// « p_Qp.g_fonct » : part des pertes par les parois en fonctionnement (tableau 132, « tout
/// autre type de générateur »).
pub const P_QP_FONCT: f64 = 0.30;
/// Puissance au-delà de laquelle les formules par défaut d'un générateur d'air chaud sont
/// évaluées à 300 kW (§8.20.3.4).
pub const PN_MAX_FORMULES_AIR_CHAUD_KW: f64 = 300.0;

/// Famille d'un autre générateur à combustion (tableau 124).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Famille {
    /// Type 103 : radiateur, poêle ou insert au gaz.
    Radiateur,
    /// Type 104 : chauffe-eau au gaz.
    ChauffeEau,
    /// Type 105 : accumulateur au gaz.
    Accumulateur,
    /// Types 106 et 107 : générateur d'air chaud.
    AirChaud,
    /// Type 108 : tube radiant.
    TubeRadiant,
    /// Type 109 : panneau radiant.
    PanneauRadiant,
}

impl Famille {
    /// « idfougen » du tableau 124 : vrai pour l'ECS (types 104 et 105), faux pour le
    /// chauffage.
    pub const fn ecs(self) -> bool {
        matches!(self, Self::ChauffeEau | Self::Accumulateur)
    }

    /// Vrai si le générateur a des pertes à charge nulle (§8.20.3.5.1 ; nulles pour les types
    /// 103, 108 et 109, éq. 1128).
    pub const fn pertes_a_charge_nulle(self) -> bool {
        matches!(self, Self::ChauffeEau | Self::Accumulateur | Self::AirChaud)
    }
}

/// Valeurs par défaut du §8.20.3.4 : coefficients `A` à `H` des éq. (1121) à (1124), lus dans
/// les tableaux 126 à 128 ; `None` quand le tableau n'en donne pas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoefficientsDefaut {
    /// `(A, B)` de « Rpn ».
    pub ab: (f64, f64),
    /// `(C, D)` de « Rpint » (générateurs d'air chaud seulement).
    pub cd: Option<(f64, f64)>,
    /// `(E, F)` des pertes à charge nulle.
    pub ef: Option<(f64, f64)>,
    /// `(G, H)` des auxiliaires.
    pub gh: Option<(f64, f64)>,
}

/// « Rpn » par défaut, en % (éq. 1121) : `A + B·log Pn`.
pub fn r_pn_defaut_pct(c: &CoefficientsDefaut, pn_kw: f64) -> f64 {
    c.ab.0 + c.ab.1 * libm::log10(pn_kw.max(f64::MIN_POSITIVE))
}

/// « Rpint » par défaut, en % (éq. 1122) : `C + D·log Pn`.
pub fn r_pint_defaut_pct(c: &CoefficientsDefaut, pn_kw: f64) -> Option<f64> {
    c.cd.map(|(cc, d)| cc + d * libm::log10(pn_kw.max(f64::MIN_POSITIVE)))
}

/// « Qp0 » par défaut, en W (éq. 1123, `Pn·(E + F·log Pn)/100` en kW) ; lue comme « Qpo30 »,
/// pertes à l'arrêt pour un écart de 30 K (nomenclature, tableau 123 : « pertes à l'arrêt
/// mesurées ou par défaut ») : lecture Q155, point 3.
pub fn q_po_30_defaut_w(c: &CoefficientsDefaut, pn_kw: f64) -> Option<f64> {
    c.ef.map(|(e, f)| 1000.0 * pn_kw * (e + f * libm::log10(pn_kw.max(f64::MIN_POSITIVE))) / 100.0)
}

/// « Paux » par défaut, en W (éq. 1124) : `G + H·Pn`.
pub fn p_aux_defaut_w(c: &CoefficientsDefaut, pn_kw: f64) -> Option<f64> {
    c.gh.map(|(g, h)| g + h * pn_kw)
}

/// État d'un pas de temps (par générateur identique).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct EtatPas {
    /// « ida_fonctionne » : 0, 1 (chauffage) ou 3 (ECS).
    id_fonctionne: u8,
    /// « Rfonctecs ».
    r_fonct_ecs: f64,
}

/// Autre générateur à combustion résolu.
#[derive(Debug, Clone, PartialEq)]
pub struct AutreGenerateurCombustion {
    /// Famille du générateur.
    pub famille: Famille,
    /// Vrai pour le GPL (PCSI de 1,09).
    pub gpl: bool,
    /// « Rdim ».
    pub rdim: f64,
    /// « idpos_gen » : vrai en volume chauffé.
    pub en_volume_chauffe: bool,
    /// « Pngen », en kW.
    pub pn_gen_kw: f64,
    /// « Pint », en kW (générateurs d'air chaud).
    pub p_int_kw: f64,
    /// « Rpn » sur PCI, en %, après prise en compte du statut.
    pub r_pn_pct: f64,
    /// « Rpint » sur PCI, en % (générateurs d'air chaud).
    pub r_pint_pct: f64,
    /// « Waux,nom », en W.
    pub w_aux_nom_w: f64,
    /// « Wveille », en W, veilleuse comprise.
    pub w_veille_w: f64,
    /// « Qpo30 », en W.
    pub q_po_30_w: f64,
    /// « θfonct_min », en °C (générateurs sur eau).
    pub theta_fonct_min_c: f64,
    /// « fmaj » (tableaux 129 et 130) ; 1 pour les autres types.
    pub f_maj: f64,
    /// « p_Qp.g_arret » (tableau 131).
    pub p_qp_arret: f64,
    /// « idraccord_gnr » = 0 : raccordé en permanence (lecture Q156).
    pub raccord_permanent: bool,
    /// « QP0prev », en W : dernière valeur des pertes à l'arrêt calculée en fonctionnement
    /// (éq. 1165), conservée d'un pas à l'autre.
    q_p0_prev_w: f64,
    prec: EtatPas,
    cour: EtatPas,
}

impl AutreGenerateurCombustion {
    /// Générateur décrit par ses grandeurs résolues ; `w` regroupe `(Waux,nom, Wveille)` en W.
    #[allow(clippy::too_many_arguments)]
    pub fn nouveau(
        famille: Famille,
        gpl: bool,
        rdim: f64,
        en_volume_chauffe: bool,
        puissances_kw: (f64, f64),
        rendements_pct: (f64, f64),
        w: (f64, f64),
        q_po_30_w: f64,
        theta_fonct_min_c: f64,
        f_maj: f64,
        p_qp_arret: f64,
        raccord_permanent: bool,
    ) -> Self {
        Self {
            famille,
            gpl,
            rdim,
            en_volume_chauffe,
            pn_gen_kw: puissances_kw.0,
            p_int_kw: puissances_kw.1,
            r_pn_pct: rendements_pct.0,
            r_pint_pct: rendements_pct.1,
            w_aux_nom_w: w.0,
            w_veille_w: w.1,
            q_po_30_w,
            theta_fonct_min_c,
            f_maj,
            p_qp_arret,
            raccord_permanent,
            q_p0_prev_w: 0.0,
            prec: EtatPas::default(),
            cour: EtatPas::default(),
        }
    }

    /// « Rdim × Pngen », en W (clé de répartition de l'éq. 1017).
    pub fn pn_gen_tot_w(&self) -> f64 {
        self.rdim * 1000.0 * self.pn_gen_kw
    }

    /// « PCSI » (tableau 125).
    pub fn pcsi(&self) -> f64 {
        pcsi(Energie::Gaz, self.gpl)
    }

    /// « QP0 », en W : pertes à charge nulle ramenées à l'écart de température effectif
    /// (éq. 1127), nulles pour les types 103, 108 et 109 (éq. 1128).
    pub fn q_p0_w(&self, theta_aval_corr_c: f64, theta_amb_c: f64) -> f64 {
        if !self.famille.pertes_a_charge_nulle() || self.r_pn_pct <= 0.0 {
            return 0.0;
        }
        let x = ((theta_aval_corr_c - theta_amb_c) / 30.0).max(0.0);
        100.0 * self.q_po_30_w / self.r_pn_pct * libm::pow(x, 1.25)
    }

    /// Fin du pas de temps : l'indicateur de fonctionnement et `Rfonctecs` du pas deviennent
    /// ceux du pas précédent ; l'indicateur repart de zéro (éq. 1125).
    pub fn fin_de_pas(&mut self) {
        self.prec = self.cour;
        self.cour = EtatPas::default();
    }

    /// Chauffage d'un générateur d'air chaud (§8.20.3.6.1) : énergie fournie et pertes comme
    /// pour une chaudière (éq. 1099 et 1087 à 1097 de la fiche 8.19), rendements ramenés au PCS
    /// (`R/PCSI`, éq. 1120) et non corrigés selon la température aval ; « QP0 » tient lieu de
    /// « φthstab » (lecture Q155, point 1). Renvoie `(Qpertes, Qconsact)` en Wh.
    fn chauffage_air_chaud(&self, q_fou: f64, q_p0: f64) -> (f64, f64) {
        let pcsi = self.pcsi();
        let p_nom = 1000.0 * self.pn_gen_kw;
        let p_int = 1000.0 * self.p_int_kw;
        // Éq. (1087) et (1088), rendements sur PCS.
        let phi_int = (100.0 * pcsi / self.r_pint_pct - 1.0) * p_int;
        let phi_nom = (100.0 * pcsi / self.r_pn_pct - 1.0) * p_nom;
        let q_pertes = if q_fou <= p_int {
            // Éq. (1092) et (1094) : cycles tout ou rien.
            let fx = if p_int > 0.0 { q_fou / p_int } else { 1.0 };
            fx * phi_int + (1.0 - fx) * q_p0
        } else {
            // Éq. (1095) et (1097) : modulation de l'allure du brûleur.
            let fx = (q_fou - p_int) / (p_nom - p_int);
            fx * phi_nom + (1.0 - fx) * phi_int
        };
        // Éq. (1098).
        (q_pertes, (q_fou + q_pertes) / pcsi)
    }

    /// Chauffage d'un radiateur, d'un tube ou d'un panneau radiant (§8.20.3.6.2) : renvoie
    /// `(QPx, Qconsact)` en Wh.
    fn chauffage_radiant(&self, q_fou: f64) -> (f64, f64) {
        let pcsi = self.pcsi();
        // Éq. (1134).
        let q_px = self.f_maj * (100.0 - self.r_pn_pct / pcsi) * q_fou * pcsi / self.r_pn_pct;
        // Éq. (1135) et (1136).
        let eta = if q_fou + q_px > 0.0 {
            q_fou / (q_fou + q_px) * 100.0 * pcsi
        } else {
            0.0
        };
        let q_cons = if eta > 0.0 { q_fou / eta * 100.0 } else { 0.0 };
        (q_px, q_cons)
    }

    /// Appel du générateur pour un poste (éq. 1119 à 1172).
    pub fn appel(&mut self, e: &EntreesAppel) -> SortiesAppel {
        let ecs = e.fonction == Poste::Ecs;
        // Éq. (1119) : seuls les générateurs sur eau sont limités par leur température
        // minimale de fonctionnement.
        let theta_corr = if self.famille.ecs() {
            e.theta_aval_c.max(self.theta_fonct_min_c)
        } else {
            e.theta_aval_c
        };
        let q_p0 = self.q_p0_w(theta_corr, e.theta_amb_c);
        let r_puis = if ecs { 1.0 } else { e.r_pui_dispo };
        let p_max = 1000.0 * self.pn_gen_kw;
        // Éq. (1140).
        let q_req_act = e.q_req_wh.max(0.0) / self.rdim;
        let (q_fou, tau, r_fonct, q_px, q_cons, w_aux) = if q_req_act > 0.0 {
            // Éq. (1126) et (1165).
            self.cour.id_fonctionne = if ecs { 3 } else { 1 };
            self.q_p0_prev_w = q_p0;
            if ecs {
                // Éq. (1139) à (1143).
                let q_fou = q_req_act.min(p_max);
                let tau = if p_max > 0.0 { q_fou / p_max } else { 0.0 };
                let r = tau;
                let pcsi = self.pcsi();
                let i_ecs = if e.ecs_seule { 1.0 } else { 0.0 };
                // Éq. (1138), (1144) et (1145) : rendement à pleine charge.
                let eta = self.r_pn_pct;
                let arret = i_ecs * (1.0 - r) * q_p0;
                let q_px = (100.0 - eta / pcsi) * q_fou * pcsi / eta + arret;
                let q_cons = q_fou / eta * 100.0 + arret;
                // Éq. (1146).
                let w = r * self.w_aux_nom_w + i_ecs * (1.0 - r) * self.w_veille_w;
                (q_fou, tau, r, q_px, q_cons, w)
            } else {
                // Éq. (1099), (1131) et (1133) ; éq. (1102).
                let dispo = r_puis * p_max;
                let q_fou = q_req_act.min(dispo).max(0.0);
                let tau = if dispo > 0.0 { q_fou / dispo } else { 0.0 };
                let (q_px, q_cons) = if self.famille == Famille::AirChaud {
                    self.chauffage_air_chaud(q_fou, q_p0)
                } else {
                    self.chauffage_radiant(q_fou)
                };
                // Éq. (1137).
                let w = r_puis * (tau * (self.w_aux_nom_w - self.w_veille_w) + self.w_veille_w);
                (q_fou, tau, 0.0, q_px, q_cons, w)
            }
        } else if ecs {
            // §8.20.3.8.2, éq. (1157) à (1163) : pertes et veille comptées en ECS seulement si
            // le générateur ne produit que de l'ECS au pas de temps.
            if e.ecs_seule {
                let q_px = if self.raccord_permanent {
                    if self.prec.id_fonctionne > 0 {
                        q_p0.max(self.q_p0_prev_w)
                    } else {
                        q_p0
                    }
                } else if self.prec.id_fonctionne > 0 {
                    self.prec.r_fonct_ecs * self.q_p0_prev_w
                } else {
                    0.0
                };
                (0.0, 0.0, 0.0, q_px, q_px, self.w_veille_w)
            } else {
                (0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
            }
        } else {
            // §8.20.3.8.1, éq. (1150) à (1156).
            let q_px = if self.raccord_permanent {
                if self.prec.id_fonctionne > 0 || r_puis < 1.0 {
                    r_puis * q_p0.max(self.q_p0_prev_w)
                } else {
                    r_puis * q_p0
                }
            } else if self.prec.id_fonctionne == 1 || r_puis < 1.0 {
                r_puis * self.q_p0_prev_w
            } else if self.prec.id_fonctionne == 3 {
                self.prec.r_fonct_ecs * self.q_p0_prev_w
            } else {
                0.0
            };
            (0.0, 0.0, 0.0, q_px, q_px, r_puis * self.w_veille_w)
        };
        if ecs {
            self.cour.r_fonct_ecs = r_fonct;
        }
        // Éq. (1164) : pertes par les parois et auxiliaires, en volume chauffé.
        let p_qp = if self.cour.id_fonctionne > 0 {
            P_QP_FONCT
        } else {
            self.p_qp_arret
        };
        let phi = if self.en_volume_chauffe {
            q_px * p_qp + w_aux
        } else {
            0.0
        };
        let rdim = self.rdim;
        // Éq. (1166) à (1171).
        let q_fou_tot = q_fou * rdim;
        SortiesAppel {
            energie: Energie::Gaz,
            q_cef_wh: q_cons * rdim,
            q_cef_aux_elec_wh: w_aux * rdim,
            q_cons_wh: q_cons * rdim,
            q_fou_wh: q_fou_tot,
            q_rest_wh: e.q_req_wh.max(0.0) - q_fou_tot,
            tau_charge: tau,
            phi_vc_wh: phi * rdim,
            w_aux_pro_wh: w_aux * rdim,
            // Éq. (1135), (1152) et (1172).
            eta_eff: if q_cons > 0.0 && q_fou > 0.0 {
                q_fou / q_cons
            } else {
                0.0
            },
            r_fonct_ecs: r_fonct,
            ..SortiesAppel::default()
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn appel(g: &mut AutreGenerateurCombustion, poste: Poste, q: f64, theta: f64) -> SortiesAppel {
        let mut e = EntreesAppel::demande(poste, q);
        e.theta_aval_c = theta;
        e.theta_amb_c = 20.0;
        g.appel(&e)
    }

    /// Radiateur gaz de 4 kW, rendement par défaut 80 % PCI, sans ventilateur, cheminée
    /// (`fmaj` = 1,06), gaz naturel (PCSI 1,11).
    #[test]
    fn radiateur_a_la_main() {
        let mut g = AutreGenerateurCombustion::nouveau(
            Famille::Radiateur,
            false,
            1.0,
            true,
            (4.0, 1.2),
            (80.0, 0.0),
            (0.0, 2.0),
            0.0,
            20.0,
            1.06,
            0.5,
            false,
        );
        let s = appel(&mut g, Poste::Chauffage, 2000.0, 20.0);
        // QPx = 1,06 × (100 − 80/1,11) × 2000 × 1,11 / 80.
        let q_px = 1.06 * (100.0 - 80.0 / 1.11) * 2000.0 * 1.11 / 80.0;
        let q_cons = (2000.0 + q_px) / 1.11;
        assert!((s.q_cons_wh - q_cons).abs() < 1e-9, "{}", s.q_cons_wh);
        assert_eq!(s.q_fou_wh, 2000.0);
        assert_eq!(s.tau_charge, 0.5);
        // Éq. (1137) : Waux = 0,5 × (0 − 2) + 2 = 1 Wh.
        assert!((s.w_aux_pro_wh - 1.0).abs() < 1e-12);
        // Éq. (1164) : 0,30 × QPx + Waux.
        assert!((s.phi_vc_wh - (0.3 * q_px + 1.0)).abs() < 1e-9);
        // Plafond à la puissance nominale.
        let s = appel(&mut g, Poste::Chauffage, 6000.0, 20.0);
        assert_eq!(s.q_fou_wh, 4000.0);
        assert_eq!(s.q_rest_wh, 2000.0);
        // À l'arrêt : pas de pertes à charge nulle (éq. 1128), veille seule.
        g.fin_de_pas();
        let s = appel(&mut g, Poste::Chauffage, 0.0, 20.0);
        assert_eq!(s.q_cons_wh, 0.0);
        assert_eq!(s.w_aux_pro_wh, 2.0);
    }

    /// Générateur d'air chaud standard de 20 kW : Rpn = 84 %, Rpint = 77 % sur PCI, Pint = 6 kW,
    /// Qpo30 = 20 × (1,75 − 0,55 log 20)/100 kW.
    #[test]
    fn air_chaud_a_la_main() {
        let q_po_30 = 1000.0 * 20.0 * (1.75 - 0.55 * libm::log10(20.0)) / 100.0;
        let mut g = AutreGenerateurCombustion::nouveau(
            Famille::AirChaud,
            false,
            1.0,
            true,
            (20.0, 6.0),
            (84.0, 77.0),
            (1080.0, 10.0),
            q_po_30,
            20.0,
            1.0,
            0.75,
            false,
        );
        let pcsi = 1.11;
        // θaval 35 °C, θamb 20 °C : QP0 = 100·Qpo30/84·(15/30)^1,25.
        let q_p0 = 100.0 * q_po_30 / 84.0 * libm::pow(0.5, 1.25);
        assert!((g.q_p0_w(35.0, 20.0) - q_p0).abs() < 1e-9);
        // Cycles tout ou rien à 3 kWh : fx = 0,5.
        let s = appel(&mut g, Poste::Chauffage, 3000.0, 35.0);
        let phi_int = (100.0 * pcsi / 77.0 - 1.0) * 6000.0;
        let q_pertes = 0.5 * phi_int + 0.5 * q_p0;
        assert!((s.q_cons_wh - (3000.0 + q_pertes) / pcsi).abs() < 1e-9);
        // Modulation à 13 kWh : fx = (13 − 6)/(20 − 6) = 0,5.
        g.fin_de_pas();
        let s = appel(&mut g, Poste::Chauffage, 13000.0, 35.0);
        let phi_nom = (100.0 * pcsi / 84.0 - 1.0) * 20000.0;
        let q_pertes = 0.5 * phi_nom + 0.5 * phi_int;
        assert!((s.q_cons_wh - (13000.0 + q_pertes) / pcsi).abs() < 1e-9);
        // Éq. (1137) : τ = 0,65.
        let w = 0.65 * (1080.0 - 10.0) + 10.0;
        assert!((s.w_aux_pro_wh - w).abs() < 1e-9);
        // Arrêt après fonctionnement, avec isolement : QPx = QP0prev (éq. 1153).
        g.fin_de_pas();
        let s = appel(&mut g, Poste::Chauffage, 0.0, 30.0);
        assert!((s.q_cons_wh - q_p0).abs() < 1e-9);
        // Arrêt suivant : plus de pertes.
        g.fin_de_pas();
        let s = appel(&mut g, Poste::Chauffage, 0.0, 30.0);
        assert_eq!(s.q_cons_wh, 0.0);
        assert_eq!(s.w_aux_pro_wh, 10.0);
        // Raccordé en permanence : pertes à l'arrêt à chaque pas (éq. 1150).
        g.raccord_permanent = true;
        g.fin_de_pas();
        let s = appel(&mut g, Poste::Chauffage, 0.0, 30.0);
        let q_p0_30 = 100.0 * q_po_30 / 84.0 * libm::pow(10.0 / 30.0, 1.25);
        assert!((s.q_cons_wh - q_p0_30).abs() < 1e-9);
    }

    /// Accumulateur gaz de 10 kW, 84 % PCI, Qpo30 = 150 W, θaval 55 °C, θamb 20 °C.
    #[test]
    fn accumulateur_ecs_a_la_main() {
        let mut g = AutreGenerateurCombustion::nouveau(
            Famille::Accumulateur,
            false,
            1.0,
            false,
            (10.0, 3.0),
            (84.0, 0.0),
            (0.0, 5.0),
            150.0,
            20.0,
            1.0,
            0.5,
            false,
        );
        let pcsi = 1.11;
        let q_p0 = 100.0 * 150.0 / 84.0 * libm::pow(35.0 / 30.0, 1.25);
        let mut e = EntreesAppel::demande(Poste::Ecs, 2500.0);
        e.theta_aval_c = 55.0;
        e.theta_amb_c = 20.0;
        let s = g.appel(&e);
        // R = 0,25 ; éq. (1145) : Qcons = 2500/84 × 100 + 0,75 × QP0.
        assert_eq!(s.r_fonct_ecs, 0.25);
        let q_cons = 2500.0 / 84.0 * 100.0 + 0.75 * q_p0;
        assert!((s.q_cons_wh - q_cons).abs() < 1e-9);
        // Éq. (1146).
        assert!((s.w_aux_pro_wh - 0.75 * 5.0).abs() < 1e-12);
        // Hors volume chauffé : rien de récupérable.
        assert_eq!(s.phi_vc_wh, 0.0);
        let _ = pcsi;
        // Arrêt après fonctionnement en ECS, avec isolement : Rfonctecs(h−1)·QP0prev (1160).
        g.fin_de_pas();
        let mut e = EntreesAppel::demande(Poste::Ecs, 0.0);
        e.theta_aval_c = 55.0;
        e.theta_amb_c = 20.0;
        let s = g.appel(&e);
        assert!((s.q_cons_wh - 0.25 * q_p0).abs() < 1e-9);
        assert_eq!(s.w_aux_pro_wh, 5.0);
    }

    #[test]
    fn valeurs_par_defaut_a_la_main() {
        let c = CoefficientsDefaut {
            ab: (84.0, 0.0),
            cd: Some((77.0, 0.0)),
            ef: Some((1.75, -0.55)),
            gh: Some((0.0, 54.0)),
        };
        assert_eq!(r_pn_defaut_pct(&c, 50.0), 84.0);
        assert_eq!(r_pint_defaut_pct(&c, 50.0), Some(77.0));
        let q = 1000.0 * 50.0 * (1.75 - 0.55 * libm::log10(50.0)) / 100.0;
        assert_eq!(q_po_30_defaut_w(&c, 50.0), Some(q));
        assert_eq!(p_aux_defaut_w(&c, 50.0), Some(2700.0));
    }
}
