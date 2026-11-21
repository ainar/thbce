// SPDX-License-Identifier: GPL-3.0-or-later
//! Transferts entre locaux par les générateurs DRV ou thermo-frigo-pompes : fiche `C_GEN_Transferts
//! entre locaux par les générateurs DRV ou thermofrigopompes` (Th-BCE 2020 §8.16, figure 106).
//!
//! Le générateur est d'abord appelé en refroidissement, comme tout générateur de la génération
//! (§8.17.3.7.2) ; à son appel en chauffage, le rejet de chaleur exploitable (éq. 978) assure
//! tout ou partie de la demande (éq. 979). Lecture : `docs/questions/Q243.md`.

use super::gestion::ContexteAppel;
use super::*;

/// « Retgnr » : rendement de récupération du rejet en chaud des DRV et des thermo-frigo-pompes,
/// conventionnel (tableau 109).
pub const RET_RECUPERATION: f64 = 0.6;

/// « Φrejet_expCH^gnr(h) » : rejet de chaleur exploitable en chauffage, en Wh, à partir de
/// l'énergie fournie `Qfou` et de l'énergie consommée hors auxiliaires `Qcons` de l'appel en
/// refroidissement du pas de temps.
// Th-BCE 2020 §8.16.3.1, éq. (978).
pub fn rejet_exploitable_wh(q_fou_fr_wh: f64, q_cons_fr_wh: f64) -> f64 {
    q_fou_fr_wh + RET_RECUPERATION * q_cons_fr_wh
}

/// Vrai si le rejet exploitable ne suffit pas à la demande de chauffage : le générateur passe
/// en chauffage et le froid est assuré par son rejet froid (éq. 979, première branche).
// Th-BCE 2020 §8.16.3.2, éq. (979).
pub fn rejet_insuffisant(q_req_wh: f64, phi_exp_wh: f64) -> bool {
    q_req_wh > phi_exp_wh || (q_req_wh == 0.0 && phi_exp_wh == 0.0)
}

impl GenerationHoraire {
    /// Appel en chauffage d'un DRV ou d'une thermo-frigo-pompe d'indice `i` (§8.16.3.2), qui
    /// remplace l'appel générique (§8.17.3.7.2) ; renvoie les sorties de l'appel, dont l'énergie
    /// restant à fournir.
    pub(super) fn appeler_transfert_locaux(
        &mut self,
        i: usize,
        ctx: &ContexteAppel,
        q_req_wh: f64,
        sources: &[ConditionsSource],
    ) -> SortiesAppel {
        let source = sources.get(i).copied().unwrap_or_default();
        let g = &mut self.generateurs[i];
        let j = &mut g.jeu;
        // Éq. (978) : rejet exploitable de l'appel en refroidissement du pas de temps ;
        // `Qcons` est l'énergie consommée hors auxiliaires du générateur, seul l'appel en
        // refroidissement ayant eu lieu.
        let q_fou_fr = j.q_fou_wh[Poste::Refroidissement.indice()];
        let phi_exp = if j.appele[Poste::Refroidissement.indice()] {
            rejet_exploitable_wh(q_fou_fr, j.q_cons_wh)
        } else {
            0.0
        };
        if rejet_insuffisant(q_req_wh, phi_exp) {
            // Remise à zéro des sorties de l'appel en froid ; l'énergie fournie en froid est
            // conservée, assurée par le rejet froid du générateur. L'indicateur de
            // refroidissement est levé avec elles (lecture Q243 point 3).
            j.q_cef = MatriceCef::NULLE;
            j.phi_rejet_wh = 0.0;
            j.phi_vc_wh = 0.0;
            j.q_cons_wh = 0.0;
            j.w_aux_pro_wh = 0.0;
            j.w_aux_wh = 0.0;
            j.q_prelec_wh = 0.0;
            j.tau_charge = 0.0;
            j.ia_refroidi = false;
            // `Rpui_dispo = 1`, appel en chauffage de toute la demande.
            let s = g.modele.appel(&EntreesAppel {
                fonction: Poste::Chauffage,
                q_req_wh,
                theta_amont_c: source.theta_amont_c,
                theta_aval_c: ctx.theta_aval_c,
                r_pui_dispo: 1.0,
                ecs_seule: false,
                theta_amb_c: ctx.theta_amb_c,
                air_extrait_w_k: source.air_extrait_w_k,
            });
            g.jeu.ajouter(Poste::Chauffage, 1.0, &s);
            // Rejet corrigé par l'énergie fournie en froid, puisée dans les locaux refroidis
            // (lecture Q243 point 2 : `+ Qfou_fr`, le texte écrivant `− Qfou_fr`).
            g.jeu.phi_rejet_wh = s.phi_rejet_wh + q_fou_fr;
            s
        } else {
            // Le rejet suffit : la demande est fournie sans consommation supplémentaire ; le
            // rejet du générateur est diminué d'autant.
            j.q_fou_wh[Poste::Chauffage.indice()] = q_req_wh;
            j.q_fou_ch_recup_wh += q_req_wh;
            j.phi_rejet_wh -= q_req_wh;
            j.appele[Poste::Chauffage.indice()] = true;
            SortiesAppel {
                q_fou_wh: q_req_wh,
                q_rest_wh: 0.0,
                ..SortiesAppel::default()
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn rejet_exploitable_a_la_main() {
        // Éq. (978) : 6 000 Wh de froid fournis pour 2 000 Wh consommés : 6 000 + 0,6 × 2 000.
        assert_eq!(rejet_exploitable_wh(6000.0, 2000.0), 7200.0);
        // Éq. (979).
        assert!(rejet_insuffisant(8000.0, 7200.0));
        assert!(!rejet_insuffisant(7200.0, 7200.0));
        assert!(!rejet_insuffisant(0.0, 7200.0));
        assert!(rejet_insuffisant(0.0, 0.0));
    }
}
