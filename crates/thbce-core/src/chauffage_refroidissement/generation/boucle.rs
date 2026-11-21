// SPDX-License-Identifier: GPL-3.0-or-later
//! Pas de temps d'une génération à boucle d'eau (Th-BCE 2020 §8.15.3, figure 105) : pompes à
//! chaleur de la boucle par groupe, bilan de la boucle, générateurs de chauffage et de froid
//! de la boucle. Fonctions pures et état : `transferts_entre_generateurs_sur_boucle.rs`.

use super::gestion::ContexteAppel;
use super::*;
use crate::chauffage_refroidissement::transferts_entre_generateurs_sur_boucle::{
    bilan_boucle_wh, mode_groupe, rejets_totaux_wh, theta_aval_chaud_c, theta_aval_froid_c,
};

impl GenerationHoraire {
    /// Remplace l'appel des générateurs instantanés de chauffage et de refroidissement d'une
    /// génération à boucle d'eau ; renvoie l'énergie restant à fournir aux groupes au pas de
    /// temps, `[chauffage, refroidissement]` (somme des « Qrep_BE^gr(h) » des postes appelés ;
    /// le report d'un poste non appelé reste dans l'état de la boucle), en Wh. `theta_aval` est la
    /// température de fonctionnement de la génération en chauffage et en refroidissement
    /// (éq. 1012 et 1013), vue par les pompes à chaleur de la boucle.
    pub(super) fn pas_boucle(
        &mut self,
        e: &EntreesGeneration<'_>,
        theta_aval: [f64; 2],
        theta_amb_c: f64,
        relance_ch: bool,
        mois: usize,
    ) -> [f64; 2] {
        let Some(mut b) = self.boucle.take() else {
            return [0.0; 2];
        };
        // §8.15.3.3 : pompes à chaleur de chaque groupe, selon sa demande et ses saisons.
        let mut q_rep_groupes = [0.0; 2];
        for (rang, rep) in b.q_rep_groupes_wh.iter_mut().enumerate() {
            let d = e.demandes_groupes.get(rang).copied().unwrap_or_default();
            let Some(poste) = mode_groupe(&d) else {
                continue;
            };
            let p = poste.indice();
            // Éq. (960) et (963) : demande avec le report du groupe (lecture Q241 point 2 : la
            // charge de l'éq. 964 est répartie sur `Qreq_BE_ch`, comme en refroidissement).
            let q_be = [d.q_ch_wh, d.q_fr_wh][p] + rep[p];
            rep[p] = 0.0;
            // Éq. (957), (958), (961) et (964) : au prorata des puissances nominales
            // installées (« Npbe × Pngen », `Rdim` de chaque générateur) des pompes à chaleur
            // du groupe qui assurent le poste (lecture Q241 point 2 pour `Ratpngen_ch`).
            let du_groupe: Vec<usize> = b
                .membres
                .iter()
                .filter(|m| m.rang_groupe == rang)
                .map(|m| m.generateur)
                .filter(|&i| self.generateurs[i].modele.fonction().assure(poste))
                .collect();
            let total: f64 = du_groupe
                .iter()
                .map(|&i| self.generateurs[i].modele.pn_gen_tot_w(poste))
                .sum();
            let ctx = ContexteAppel {
                poste,
                theta_aval_c: theta_aval[p],
                theta_amb_c,
                aut_ch: d.aut_ch,
                relance_ch,
            };
            if du_groupe.is_empty() {
                rep[p] = q_be;
                q_rep_groupes[p] += q_be;
                continue;
            }
            for &i in &du_groupe {
                let q_req = if total > 0.0 {
                    self.generateurs[i].modele.pn_gen_tot_w(poste) / total * q_be
                } else {
                    q_be / du_groupe.len() as f64
                };
                rep[p] += match self.appeler(i, &ctx, q_req, e.sources) {
                    Some(s) => s.q_rest_wh,
                    None => q_req,
                };
            }
            q_rep_groupes[p] += rep[p];
        }
        // §8.15.3.4, éq. (965) à (968) : bilan de la boucle.
        let rejets = rejets_totaux_wh(
            b.membres
                .iter()
                .map(|m| self.generateurs[m.generateur].jeu.phi_rejet_wh),
        );
        let (transfert, q_sys) = bilan_boucle_wh(rejets);
        let c = &mut b.cumuls;
        c.phi_rejet_tot_ch_wh += rejets[0];
        c.phi_rejet_tot_fr_wh += rejets[1];
        c.q_transfert_wh += transfert;
        c.q_sys_ch_wh += q_sys[0];
        c.q_sys_fr_wh += q_sys[1];
        // §8.15.3.6 et §8.15.3.7 : générateurs de la boucle, éq. (969) à (977). Hors saison, le
        // générateur est désactivé et la demande reste reportée, comme à l'éq. (1028) (lecture
        // Q242 point 2).
        let theta_be = b.theta_be.theta_c(u8::try_from(mois + 1).unwrap_or(1));
        for (poste, gen, actif) in [
            (Poste::Chauffage, b.chaud, e.aut_ch),
            (Poste::Refroidissement, b.froid, e.aut_fr),
        ] {
            let p = poste.indice();
            let q_req = q_sys[p] + b.q_rep_wh[p];
            let Some(i) = gen else {
                b.q_rep_wh[p] = 0.0;
                continue;
            };
            if !actif {
                b.q_rep_wh[p] = q_req;
                continue;
            }
            let source = e.sources.get(i).copied().unwrap_or_default();
            let g = &mut self.generateurs[i];
            let (theta_amont_c, theta_aval_c) = match &g.modele {
                // Éq. (972) : la tour reçoit `θes_tour` (éq. 1435) et `θbe(j)`.
                Generateur::TourBoucle(t) => (
                    t.tour.theta_es_tour_c(e.theta_ext_c, e.w_ext_kg_kg),
                    theta_be,
                ),
                modele => {
                    let pn = modele.pn_gen_tot_w(poste);
                    let theta = if poste == Poste::Chauffage {
                        theta_aval_chaud_c(theta_be, b.theta_max_rech_c, q_req, pn)
                    } else {
                        theta_aval_froid_c(theta_be, b.theta_min_fr_c, q_req, pn)
                    };
                    (source.theta_amont_c, theta)
                }
            };
            // Éq. (971) et (977) : `Rpui_dispo = 1`.
            let s = g.modele.appel(&EntreesAppel {
                fonction: poste,
                q_req_wh: q_req,
                theta_amont_c,
                theta_aval_c,
                r_pui_dispo: 1.0,
                ecs_seule: false,
                theta_amb_c,
                air_extrait_w_k: source.air_extrait_w_k,
            });
            if poste == Poste::Refroidissement && q_req > 0.0 {
                g.jeu.ia_refroidi = true;
            }
            g.jeu.ajouter(poste, 1.0, &s);
            b.q_rep_wh[p] = s.q_rest_wh;
        }
        self.boucle = Some(b);
        q_rep_groupes
    }
}
