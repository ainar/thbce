// SPDX-License-Identifier: GPL-3.0-or-later
//! Restitution des résultats de la production solaire : boucles et productions collectives.

use super::ecs::resultats_stockage;
use crate::ecs::boucle_solaire::BoucleSolaireHoraire;
use crate::ecs::cesci::ProductionCollective;
use crate::resultats::{ResultatsBoucleSolaire, ResultatsSolaireCollectif, ResultatsStockage};

/// Résultats d'une boucle solaire pour `nb` assemblages identiques ; `ecretement_wh` :
/// énergie écrêtée à la température maximale des ballons qu'elle chauffe, en Wh.
pub(super) fn resultats_boucle(
    b: &BoucleSolaireHoraire,
    nb: f64,
    ecretement_wh: f64,
) -> ResultatsBoucleSolaire {
    let c = &b.cumuls;
    ResultatsBoucleSolaire {
        e_sol_kwh: c.e_sol_wh() * nb / 1000.0,
        e_sol_mensuel_kwh: c.e_sol_mensuel_wh.iter().map(|x| x * nb / 1000.0).collect(),
        w_pompe_kwh: c.e_aux_wh * nb / 1000.0,
        pertes_vc_kwh: c.phi_vc_wh * nb / 1000.0,
        isr_kwh_m2: c.isr_wh_m2 / 1000.0,
        a_capteurs_m2: b.boucle.a_m2,
        nbh_marche: c.nbh_marche,
        ecretement_kwh: ecretement_wh / 1000.0,
    }
}

/// Résultats d'une production solaire collective à appoints décentralisés (fiches 9.16 et
/// 9.17).
pub(super) fn resultats_collectif(c: &ProductionCollective) -> ResultatsSolaireCollectif {
    use crate::ecs::cesci::TypeProduction;
    let ecretement: f64 = match &c.central {
        Some(b) => b.ballon.ecretement_wh,
        None => c
            .ensembles
            .iter()
            .map(|a| a.ballon.ecretement_wh * a.nb)
            .sum(),
    };
    let central = c.central.as_ref().map(|b| {
        let k = &b.cumuls;
        ResultatsStockage {
            id: format!("{}/central", c.id),
            q_fou_ecs_kwh: (k.q_fou_reseau_wh + k.pertes_reseau_deduites_wh) / 1000.0,
            q_fou_sto_base_kwh: c.boucle.cumuls.e_sol_wh() / 1000.0,
            q_fou_sto_ap_kwh: 0.0,
            pertes_kwh: k.phi_pertes_wh / 1000.0,
            v_puise_l: 0.0,
            theta_haut_min_c: k.theta_haut_min_c.unwrap_or(b.ballon.theta_c[3]),
            nbh_temp_insuff_max: 0,
            nbh_report_e: 0,
            v_zones_l: b.ballon.v_l.to_vec(),
            theta_zones_debut_c: k.theta_debut_c.unwrap_or(b.ballon.theta_c).to_vec(),
            theta_zones_fin_c: b.ballon.theta_c.to_vec(),
            solaire: None,
            ssc: None,
            idcet: None,
            valeurs_par_defaut: Vec::new(),
        }
    });
    let k = c.central.as_ref().map(|b| b.cumuls).unwrap_or_default();
    ResultatsSolaireCollectif {
        id: c.id.clone(),
        type_pcad: match c.type_production {
            TypeProduction::Cesci => crate::entree::TypePcad::Cesci,
            TypeProduction::Cescai => crate::entree::TypePcad::Cescai,
            TypeProduction::CesciAppointSepare => crate::entree::TypePcad::CesciAppointSepare,
        },
        solaire: resultats_boucle(&c.boucle, 1.0, ecretement),
        ensembles: c.ensembles.iter().map(resultats_stockage).collect(),
        ballon_central: central,
        q_fou_reseau_kwh: k.q_fou_reseau_wh / 1000.0,
        pertes_reseau_deduites_kwh: k.pertes_reseau_deduites_wh / 1000.0,
    }
}
