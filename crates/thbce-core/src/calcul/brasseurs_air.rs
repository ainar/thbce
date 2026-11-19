// SPDX-License-Identifier: GPL-3.0-or-later
//! Restitution des brasseurs d'air d'un groupe (Th-BCE 2020 §8.32).

use crate::bati::assemblage_groupe::GroupeHoraire;
use crate::chauffage_refroidissement::brasseurs_air::rat_us_defaut;
use crate::entree::{GestionBrasseur, Groupe, UsageZone};
use crate::resultats::{ResultatsBrasseurs, ValeurParDefaut};

/// Brasseurs du groupe : consommation du calcul Th-C, effet du calcul Th-D ; `None` sans brasseur.
pub(super) fn resultats_brasseurs(
    th_d: &GroupeHoraire,
    th_c: &GroupeHoraire,
) -> Option<ResultatsBrasseurs> {
    let c = th_c.brasseurs.as_ref()?;
    let d = th_d.brasseurs.as_ref()?;
    Some(ResultatsBrasseurs {
        w_abs_kwh: c.cumuls.w_wh / 1000.0,
        nbh_fonctionnement: c.cumuls.nbh_marche,
        nbh_fonctionnement_th_d: d.cumuls.nbh_marche,
        delta_theta_op_max_k: d.cumuls.delta_theta_op_max_k,
    })
}

/// Valeurs par défaut des brasseurs d'un groupe : `RatUs_BA` du tableau 257 et `Δθop_3_br_p_aut`
/// nul (§8.32.3.2.4.3).
pub(super) fn defauts_brasseurs(groupe: &Groupe, usage: UsageZone) -> Vec<ValeurParDefaut> {
    let mut v = Vec::new();
    for (i, b) in groupe.brasseurs_air.iter().enumerate() {
        if b.ratio_surface_occupee.is_none() {
            v.push(ValeurParDefaut {
                champ: format!("brasseurs_air[{i}].ratio_surface_occupee"),
                valeur: rat_us_defaut(usage, b.usage),
                regle: "Th-BCE 2020 §8.32.3.2.3, tableau 257 : RatUs_BA par défaut selon l'usage \
                        de la zone"
                    .to_owned(),
            });
        }
        if let GestionBrasseur::Automatique(p) = &b.gestion {
            if p.delta_theta_op_3_k.is_none() {
                v.push(ValeurParDefaut {
                    champ: format!("brasseurs_air[{i}].gestion.automatique.delta_theta_op_3_k"),
                    valeur: 0.0,
                    regle: "Th-BCE 2020 §8.32.3.2.4.3 : Δθop_3_br_p_aut vaut 0 par défaut"
                        .to_owned(),
                });
            }
        }
    }
    v
}
