// SPDX-License-Identifier: GPL-3.0-or-later
//! Restitution des résultats de l'ECS : stockages, émetteurs et distributions intergroupes.

use super::solaire::resultats_boucle;
use crate::bati::assemblage_batiment::BatimentHoraire;
use crate::ecs::assemblage_ballon::AssemblageBallon;
use crate::ecs::distribution_ecs::{l_vc_2nd_e_defaut_m, EmetteurEcsResolu};
use crate::entree::UsageZone;
use crate::resultats::{
    ResultatsEchangeurEcs, ResultatsEcsEmetteur, ResultatsEcsIntergroupe, ResultatsIdCet, ResultatsSsc, ResultatsStockage,
    ValeurParDefaut,
};

/// Résultats d'un assemblage avec ballon de stockage d'ECS (§9.14).
pub(super) fn resultats_stockage(s: &AssemblageBallon) -> ResultatsStockage {
    let k = &s.cumuls;
    ResultatsStockage {
        id: s.id.clone(),
        q_fou_ecs_kwh: k.q_fou_ecs_wh / 1000.0,
        q_fou_sto_base_kwh: k.q_fou_sto_wh[0] / 1000.0,
        q_fou_sto_ap_kwh: k.q_fou_sto_wh[1] / 1000.0,
        pertes_kwh: k.phi_pertes_wh / 1000.0,
        v_puise_l: k.v_p_l,
        theta_haut_min_c: k.theta_haut_min_c,
        nbh_temp_insuff_max: k.nbh_temp_insuff_max,
        nbh_report_e: k.nbh_report_e,
        v_zones_l: s.ballon.v_l.to_vec(),
        theta_zones_debut_c: k.theta_debut_c.unwrap_or(s.ballon.theta_c).to_vec(),
        theta_zones_fin_c: s.ballon.theta_c.to_vec(),
        ssc: s.ssc.map(|_| ResultatsSsc {
            q_puise_ch_kwh: k.q_puise_ch_wh / 1000.0,
            q_fou_ch_appoint_kwh: k.q_fou_ch_ap_wh / 1000.0,
        }),
        solaire: s
            .solaire
            .as_ref()
            .map(|b| resultats_boucle(b, s.nb, s.ballon.ecretement_wh * s.nb)),
        idcet: s.identification.map(|i| ResultatsIdCet {
            ua_s_w_k: i.triplet.ua_s_w_k,
            cop_pivot: i.triplet.cop_pivot,
            pabs_pivot_kw: i.triplet.pabs_pivot_w / 1000.0,
            epsilon: i.epsilon,
        }),
        echangeur: s.echangeur.map(|e| ResultatsEchangeurEcs {
            ua_ech_w_k: e.ua_ech_w_k,
            theta_cons_c: e.theta_cons_c,
            w_circ_prim_kwh: k.w_circ_prim_wh / 1000.0,
        }),
        valeurs_par_defaut: s.valeurs_par_defaut.clone(),
    }
}

/// Émetteur d'ECS et sa distribution à l'issue du calcul Th-C (§9.5 à 9.7).
pub(super) fn resultats_ecs(e: &EmetteurEcsResolu) -> ResultatsEcsEmetteur {
    let b = &e.besoins;
    let c = &e.cumuls;
    let mut valeurs_par_defaut = Vec::new();
    if e.theta_2nd_defaut {
        valeurs_par_defaut.push(ValeurParDefaut {
            champ: "distribution.theta_2nd_e_c".to_owned(),
            valeur: e.distribution.theta_2nd_e_c,
            regle: "Th-BCE 2020 §9.7.2, tableau 278 : température conventionnelle de la \
                    distribution d'ECS du groupe (lecture Q45)"
                .to_owned(),
        });
    }
    if e.l_vc_defaut {
        valeurs_par_defaut.push(ValeurParDefaut {
            champ: "distribution.l_vc_m".to_owned(),
            valeur: l_vc_2nd_e_defaut_m(UsageZone::MaisonIndividuelle, b.a_m2),
            regle: "Th-BCE 2020 §9.7.3, éq. (1729) : en habitation, Lvc_2nd-e = 6 × A/80"
                .to_owned(),
        });
    }
    ResultatsEcsEmetteur {
        id: e.id.clone(),
        a_m2: b.a_m2,
        nadeq: b.nadeq,
        a_l: b.a_l,
        corr: b.corr,
        v_uw_hebdo_corr_l: b.v_uw_hebdo_corr_l,
        q_w_kwh: c.q_w_wh / 1000.0,
        q_w_bruts_kwh: c.q_w_bruts_wh / 1000.0,
        q_w_2nd_kwh: c.q_w_2nd_wh / 1000.0,
        pertes_vc_kwh: c.phi_pertes_vc_wh / 1000.0,
        pertes_hvc_kwh: c.phi_pertes_hvc_wh / 1000.0,
        valeurs_par_defaut,
    }
}

/// Distributions d'ECS intergroupes d'un bâtiment à l'issue du calcul Th-C (§9.8).
pub(super) fn resultats_ecs_intergroupes(b: &BatimentHoraire) -> Vec<ResultatsEcsIntergroupe> {
    let s = &b.systemes;
    s.ecs
        .iter()
        .zip(&s.cumuls_ecs)
        .map(|(d, c)| ResultatsEcsIntergroupe {
            id: d.id.clone(),
            generation: s.generations.get(d.generation).cloned().unwrap_or_default(),
            q_w_prim_kwh: c.q_w_prim_mensuel_wh.iter().sum::<f64>() / 1000.0,
            q_w_prim_mensuel_kwh: c.q_w_prim_mensuel_wh.iter().map(|x| x / 1000.0).collect(),
            pertes_vc_kwh: c.phi_pertes_vc_wh / 1000.0,
            pertes_hvc_kwh: c.phi_pertes_hvc_wh / 1000.0,
            w_aux_kwh: c.w_aux_wh / 1000.0,
            w_rechauf_kwh: c.w_rechauf_wh / 1000.0,
            theta_depart_c: c.theta_depart_max_c,
        })
        .collect()
}
