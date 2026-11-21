// SPDX-License-Identifier: GPL-3.0-or-later
//! Restitution des résultats des générations et de leurs avertissements.

use super::ecs::resultats_stockage;
use super::solaire::resultats_collectif;
use crate::bati::assemblage_batiment::BatimentHoraire;
use crate::chauffage_refroidissement::calculs_generation::Poste;
use crate::resultats::{
    Consommations, ConsommationsPostes, ResultatsBoucleDEau, ResultatsGenerateur,
    ResultatsGeneration,
};

/// Générations d'un bâtiment à l'issue du calcul Th-C : demandes de leurs réseaux, énergies
/// fournies et consommations de leurs générateurs (§8.29).
pub(super) fn resultats_generations(b: &BatimentHoraire) -> Vec<ResultatsGeneration> {
    b.systemes
        .generations_horaires
        .iter()
        .map(|g| {
            let c = &g.cumuls;
            let demande = |p: Poste| c.q_req_mensuel_wh[p.indice()].iter().sum::<f64>() / 1000.0;
            ResultatsGeneration {
                id: g.id.clone(),
                q_ch_kwh: demande(Poste::Chauffage),
                q_fr_kwh: demande(Poste::Refroidissement),
                q_ecs_kwh: demande(Poste::Ecs),
                q_fou_ch_kwh: c.q_fou_wh[0] / 1000.0,
                q_fou_fr_kwh: c.q_fou_wh[1] / 1000.0,
                q_fou_ecs_kwh: c.q_fou_wh[2] / 1000.0,
                q_rep_fin_kwh: g.etat.q_rep_wh.iter().sum::<f64>() / 1000.0,
                consommations: Consommations::depuis_cumuls(&c.cef),
                pertes_vc_kwh: c.phi_vc_tot_wh / 1000.0,
                nbh_sousdim_ch_ecs_max: c.nb_sousdim_max[0],
                nbh_sousdim_fr_max: c.nb_sousdim_max[1],
                stockages: g.stockages.iter().map(resultats_stockage).collect(),
                solaires_collectifs: g.collectifs.iter().map(resultats_collectif).collect(),
                boucle_d_eau: g.boucle.as_ref().map(|b| {
                    let c = &b.cumuls;
                    ResultatsBoucleDEau {
                        phi_rejet_tot_ch_kwh: c.phi_rejet_tot_ch_wh / 1000.0,
                        phi_rejet_tot_fr_kwh: c.phi_rejet_tot_fr_wh / 1000.0,
                        q_transfert_kwh: c.q_transfert_wh / 1000.0,
                        q_sys_ch_kwh: c.q_sys_ch_wh / 1000.0,
                        q_sys_fr_kwh: c.q_sys_fr_wh / 1000.0,
                        q_rep_fin_kwh: (b.q_rep_wh[0] + b.q_rep_wh[1]) / 1000.0,
                    }
                }),
                generateurs: g
                    .generateurs
                    .iter()
                    .map(|x| {
                        let k = &x.cumuls;
                        ResultatsGenerateur {
                            id: x.id.clone(),
                            q_fou_ch_kwh: k.q_fou_wh[0] / 1000.0,
                            q_fou_fr_kwh: k.q_fou_wh[1] / 1000.0,
                            q_fou_ecs_kwh: k.q_fou_wh[2] / 1000.0,
                            consommations: ConsommationsPostes::depuis_matrice(&k.cef.annuel()),
                            eta_eff_ch_an: k.eta_eff_an(Poste::Chauffage),
                            eta_eff_fr_an: k.eta_eff_an(Poste::Refroidissement),
                            eta_eff_ecs_an: k.eta_eff_an(Poste::Ecs),
                            w_aux_kwh: k.w_aux_wh / 1000.0,
                            phi_rejet_kwh: k.phi_rejet_wh / 1000.0,
                            q_prelec_kwh: k.q_prelec_wh / 1000.0,
                            valeurs_par_defaut: x.valeurs_par_defaut.clone(),
                        }
                    })
                    .collect(),
            }
        })
        .collect()
}

/// Alertes de sous-dimensionnement des générations (§8.17.3.9, éq. 1059 à 1064).
pub(super) fn avertissements_generations(id_bat: &str, b: &BatimentHoraire) -> Vec<String> {
    let mut v = Vec::new();
    for g in &b.systemes.generations_horaires {
        for (k, poste) in ["le chauffage ou l'ECS", "le refroidissement"]
            .into_iter()
            .enumerate()
        {
            let (court, long) = g.sous_dimensionnement(k);
            if k == 0 {
                for s in &g.stockages {
                    let n = s.cumuls.nbh_report_e;
                    if n > crate::ecs::assemblage_ballon::SEUIL_REPORT_PUISAGE_H {
                        v.push(format!(
                            "bâtiment « {id_bat} », génération « {} », ballon « {} » : pendant \
                             {n} heures de l'année, l'énergie demandée n'a pas pu être \
                             entièrement puisée dans le ballon (Th-BCE 2020 §9.9.3.2.3, éq. 1780)",
                            g.id, s.id
                        ));
                    }
                    let n = s.cumuls.nbh_temp_insuff_max;
                    if n > crate::ecs::assemblage_ballon::SEUIL_TEMPERATURE_INSUFFISANTE_H {
                        v.push(format!(
                            "bâtiment « {id_bat} », génération « {} », ballon « {} » : sur {n} \
                             heures consécutives, le ballon n'a jamais atteint sa température de \
                             consigne (Th-BCE 2020 §9.14.3.4, éq. 1959)",
                            g.id, s.id
                        ));
                    }
                }
            }
            if court {
                v.push(format!(
                    "bâtiment « {id_bat} », génération « {} » : sous-dimensionnement{} pour {poste} \
                     ({} heures successives de demande non couverte ; Th-BCE 2020 §8.17.3.9, éq. \
                     1059 à 1064)",
                    g.id,
                    if long { " critique" } else { "" },
                    g.cumuls.nb_sousdim_max[k]
                ));
            }
        }
    }
    v
}
