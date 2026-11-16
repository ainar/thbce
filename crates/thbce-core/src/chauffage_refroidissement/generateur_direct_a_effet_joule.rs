// SPDX-License-Identifier: GPL-3.0-or-later
//! Générateur direct à effet Joule : fiche `C_GEN_Générateur direct à effet joule`
//! (Th-BCE 2020 §8.18), type 500 du tableau 111.
//!
//! Générateur parfait, de rendement unitaire, sans pertes ni auxiliaires (éq. 1065), limité par
//! sa puissance nominale (éq. 1066). Il assure le chauffage ou l'ECS, jamais les deux
//! (§8.18.3.1).

use super::calculs_generation::{Energie, Poste};
use super::generation::{EntreesAppel, SortiesAppel};

/// Générateur direct à effet Joule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenerateurEffetJoule {
    /// « idfougen » : chauffage (1) ou ECS (3).
    pub fonction: Poste,
    /// « Pngen » : puissance nominale d'un générateur, en W.
    pub pn_gen_w: f64,
    /// « Rdim » : nombre de générateurs identiques (§8.17.2).
    pub rdim: f64,
}

impl GenerateurEffetJoule {
    /// « Pmax » : puissance maximale disponible, en W (éq. 1066), pour l'ensemble des
    /// générateurs identiques (lecture Q46 : la fiche 8.18 ne cite pas `Rdim`, que la gestion de
    /// la génération applique aux puissances nominales, éq. 1017).
    pub fn p_max_w(&self) -> f64 {
        self.rdim * self.pn_gen_w
    }

    /// Appel du générateur pour une demande `Qreq`, en Wh (éq. 1065 à 1071).
    pub fn appel(&self, e: &EntreesAppel) -> SortiesAppel {
        let p_max = self.p_max_w();
        // Éq. (1067) : énergie fournie et consommée.
        let q_fou = e.q_req_wh.min(p_max).max(0.0);
        let q_cons = q_fou;
        // Éq. (1069).
        let tau_charge = if p_max > 0.0 { q_fou / p_max } else { 0.0 };
        SortiesAppel {
            energie: Energie::Electricite,
            q_cef_wh: q_cons,
            q_cons_wh: q_cons,
            q_fou_wh: q_fou,
            // Éq. (1068).
            q_rest_wh: e.q_req_wh - q_cons,
            tau_charge,
            // Éq. (1065) : rendement unitaire, ni auxiliaires ni pertes (éq. 1070).
            eta_eff: 1.0,
            // Éq. (1071).
            r_fonct_ecs: if self.fonction == Poste::Ecs {
                tau_charge
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

    #[test]
    fn effet_joule_a_la_main() {
        let g = GenerateurEffetJoule {
            fonction: Poste::Chauffage,
            pn_gen_w: 1000.0,
            rdim: 2.0,
        };
        // Demande de 500 Wh : fournie et consommée, charge 25 % des 2 kW installés.
        let s = g.appel(&EntreesAppel::demande(Poste::Chauffage, 500.0));
        assert_eq!(s.q_fou_wh, 500.0);
        assert_eq!(s.q_cons_wh, 500.0);
        assert_eq!(s.q_cef_wh, 500.0);
        assert_eq!(s.q_rest_wh, 0.0);
        assert_eq!(s.tau_charge, 0.25);
        assert_eq!(s.eta_eff, 1.0);
        assert_eq!(s.phi_vc_wh, 0.0);
        assert_eq!(s.w_aux_pro_wh, 0.0);
        assert_eq!(s.r_fonct_ecs, 0.0);
        // Demande de 2,6 kWh : plafonnée à 2 kWh, 600 Wh restent à fournir.
        let s = g.appel(&EntreesAppel::demande(Poste::Chauffage, 2600.0));
        assert_eq!(s.q_fou_wh, 2000.0);
        assert_eq!(s.q_rest_wh, 600.0);
        assert_eq!(s.tau_charge, 1.0);
    }

    #[test]
    fn effet_joule_en_ecs() {
        let g = GenerateurEffetJoule {
            fonction: Poste::Ecs,
            pn_gen_w: 3000.0,
            rdim: 1.0,
        };
        let s = g.appel(&EntreesAppel::demande(Poste::Ecs, 1500.0));
        assert_eq!(s.q_fou_wh, 1500.0);
        assert_eq!(s.r_fonct_ecs, 0.5);
    }
}
