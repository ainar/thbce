// SPDX-License-Identifier: GPL-3.0-or-later
//! Restitution des résultats des systèmes du groupe et des distributions intergroupes.

use super::batiment::resultats_saisons;
use super::ecs::resultats_ecs;
use crate::bati::assemblage_batiment::BatimentHoraire;
use crate::bati::assemblage_groupe::GroupeHoraire;
use crate::resultats::{
    ResultatsDistributionGroupe, ResultatsEmetteur, ResultatsIntergroupes, ResultatsSystemesGroupe,
};

/// Systèmes d'émission d'un groupe à l'issue du calcul Th-C (§8.1).
pub(super) fn resultats_systemes(g: &GroupeHoraire) -> Option<ResultatsSystemesGroupe> {
    let sy = g.systemes.as_ref()?;
    let c = &sy.cumuls;
    let kwh = |v: &[f64; 12]| v.iter().map(|x| x / 1000.0).collect::<Vec<_>>();
    Some(ResultatsSystemesGroupe {
        q_sys_ch_kwh: c.q_sys_ch_mensuel_wh.iter().sum::<f64>() / 1000.0,
        q_sys_fr_kwh: c.q_sys_fr_mensuel_wh.iter().sum::<f64>() / 1000.0,
        q_sys_ch_mensuel_kwh: kwh(&c.q_sys_ch_mensuel_wh),
        q_sys_fr_mensuel_kwh: kwh(&c.q_sys_fr_mensuel_wh),
        emetteurs: sy
            .emetteurs
            .iter()
            .enumerate()
            .map(|(i, e)| ResultatsEmetteur {
                id: e.id.clone(),
                q_ch_kwh: c.q_em_ch_wh[i] / 1000.0,
                q_fr_kwh: c.q_em_fr_wh[i] / 1000.0,
            })
            .collect(),
        w_vent_loc_kwh: c.w_vent_loc_wh / 1000.0,
        nbh_relance: c.nbh_relance,
        recup_kwh: c.recup_wh / 1000.0,
        ecs: sy.ecs.iter().map(resultats_ecs).collect(),
        distributions: sy
            .distributions
            .iter()
            .zip(&c.distributions)
            .map(|(d, k)| ResultatsDistributionGroupe {
                id: d.id.clone(),
                q_kwh: k.q_wh / 1000.0,
                pertes_vc_kwh: k.phi_pertes_vc_wh / 1000.0,
                pertes_hvc_kwh: k.phi_pertes_hvc_wh / 1000.0,
                w_aux_kwh: k.w_aux_wh / 1000.0,
            })
            .collect(),
        saisons_th_c: Some(resultats_saisons(g)),
    })
}

/// Réseaux intergroupes d'un bâtiment à l'issue du calcul Th-C (§8.9 et §8.10).
pub(super) fn resultats_intergroupes(b: &BatimentHoraire) -> Vec<ResultatsIntergroupes> {
    let s = &b.systemes;
    s.intergroupes
        .iter()
        .zip(&s.cumuls)
        .map(|(d, c)| ResultatsIntergroupes {
            id: d.id.clone(),
            generation: s.generations.get(d.generation).cloned().unwrap_or_default(),
            q_ch_kwh: c.q_ch_mensuel_wh.iter().sum::<f64>() / 1000.0,
            q_fr_kwh: c.q_fr_mensuel_wh.iter().sum::<f64>() / 1000.0,
            q_ch_mensuel_kwh: c.q_ch_mensuel_wh.iter().map(|x| x / 1000.0).collect(),
            q_fr_mensuel_kwh: c.q_fr_mensuel_wh.iter().map(|x| x / 1000.0).collect(),
            w_aux_kwh: c.w_aux_wh / 1000.0,
            pertes_vc_kwh: c.phi_pertes_vc_wh / 1000.0,
            pertes_hvc_kwh: c.phi_pertes_hvc_wh / 1000.0,
            nbh_fonct: c.nbh_fonct,
            theta_dep_extreme_c: c.theta_dep_extreme_c,
        })
        .collect()
}
