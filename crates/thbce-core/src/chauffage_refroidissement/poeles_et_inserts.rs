// SPDX-License-Identifier: GPL-3.0-or-later
//! Poêles et inserts à bois : fiche `C_GEN_poêles et inserts` (Th-BCE 2020 §8.22), types 403
//! et 404 du tableau 111. Générateur de chauffage seul, fonctionnant à puissance nominale
//! pendant une fraction du pas de temps égale au taux de charge (éq. 1225 à 1237). Lectures :
//! `docs/questions/Q049.md`.

use super::calculs_generation::Energie;
use super::generation::{EntreesAppel, SortiesAppel};

/// Poêle ou insert.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Poele {
    /// « Pngen » : puissance nominale d'un appareil, en kW.
    pub pn_gen_kw: f64,
    /// « ηH,sys,n » : rendement moyen, en % (de 0 exclu à 100 exclu).
    pub eta_pct: f64,
    /// « Paux,vent » : puissance du ventilateur, en W.
    pub p_aux_vent_w: f64,
    /// « Rdim » : nombre d'appareils identiques.
    pub rdim: f64,
}

impl Poele {
    /// « Rdim × Pngen », en W.
    pub fn pn_gen_tot_w(&self) -> f64 {
        self.rdim * 1000.0 * self.pn_gen_kw
    }

    /// « QH,sys,ls,100 » : pertes à pleine charge pendant un pas de temps d'une heure, en Wh
    /// (éq. 1228 ; lecture Q49 : `Pngen` est converti de kW en W comme aux éq. 1226 et 1227).
    pub fn pertes_pleine_charge_wh(&self) -> f64 {
        (100.0 - self.eta_pct) / self.eta_pct * 1000.0 * self.pn_gen_kw
    }

    /// Appel du générateur (éq. 1225 à 1237).
    pub fn appel(&self, e: &EntreesAppel) -> SortiesAppel {
        let p_nom = 1000.0 * self.pn_gen_kw;
        // Éq. (1225) et (1226).
        let q_req_act = e.q_req_wh.max(0.0) / self.rdim;
        let q_fou_act = q_req_act.min(p_nom);
        // Éq. (1227).
        let tau = if p_nom > 0.0 { q_fou_act / p_nom } else { 0.0 };
        // Éq. (1229) et (1230).
        let phi_th = tau * self.pertes_pleine_charge_wh();
        // Éq. (1231).
        let q_cons_act = q_fou_act + phi_th;
        let rdim = self.rdim;
        // Éq. (1234).
        let w_aux = rdim * tau * self.p_aux_vent_w;
        SortiesAppel {
            energie: Energie::Bois,
            q_cef_wh: rdim * q_cons_act,
            q_cef_aux_elec_wh: w_aux,
            q_cons_wh: rdim * q_cons_act,
            // Éq. (1232) et (1237).
            q_fou_wh: rdim * q_fou_act,
            q_rest_wh: e.q_req_wh.max(0.0) - rdim * q_fou_act,
            tau_charge: tau,
            // Éq. (1235) : pertes intégralement perdues, hors ventilateur.
            phi_vc_wh: 0.0,
            w_aux_pro_wh: w_aux,
            // Éq. (1236).
            eta_eff: if q_cons_act > 0.0 {
                q_fou_act / q_cons_act
            } else {
                0.0
            },
            ..SortiesAppel::default()
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::chauffage_refroidissement::calculs_generation::Poste;

    #[test]
    fn poele_a_la_main() {
        let p = Poele {
            pn_gen_kw: 8.0,
            eta_pct: 80.0,
            p_aux_vent_w: 40.0,
            rdim: 1.0,
        };
        // Charge de 25 % : 2 kWh fournis, pertes 0,25 × (20/80) × 8 kWh = 0,5 kWh.
        let s = p.appel(&EntreesAppel::demande(Poste::Chauffage, 2000.0));
        assert_eq!(s.q_fou_wh, 2000.0);
        assert!((s.q_cons_wh - 2500.0).abs() < 1e-9);
        assert!((s.eta_eff - 0.8).abs() < 1e-12);
        assert!((s.w_aux_pro_wh - 10.0).abs() < 1e-12);
        assert_eq!(s.phi_vc_wh, 0.0);
        // Plafonné à la puissance nominale.
        let s = p.appel(&EntreesAppel::demande(Poste::Chauffage, 20000.0));
        assert_eq!(s.q_fou_wh, 8000.0);
        assert_eq!(s.q_rest_wh, 12000.0);
        assert!((s.q_cons_wh - 10000.0).abs() < 1e-9);
        // Pas de consommation hors fonctionnement.
        let s = p.appel(&EntreesAppel::demande(Poste::Chauffage, 0.0));
        assert_eq!(s.q_cons_wh, 0.0);
        assert_eq!(s.w_aux_pro_wh, 0.0);
    }
}
