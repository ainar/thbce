// SPDX-License-Identifier: GPL-3.0-or-later
//! Restitution des résultats de la ventilation.

use crate::climat::calendrier::date_scenario;
use crate::conventions::NB_HEURES_AN;
use crate::entree::{Groupe, UsageZone};
use crate::resultats::{ResultatsDebitsBbio, ResultatsVentilationGroupe};
use crate::ventilation::bouche_conduit::debits_specifiques_bbio_groupe;
use crate::ventilation::ven_bbio::{debits_systeme_conventionnel, LectureDebitsBbio};
use crate::ventilation::ventilation_projet::VentilationProjet;

/// Ventilation du projet vue d'un groupe : débits et consommation des ventilateurs.
///
/// Th-BCE 2020 §6.2.3.4 (éq. 428 à 431) ; §6.3.3.4.13 (éq. 565 et 570) et §6.5.3.3 (éq. 645 et
/// 646) : la consommation horaire, en Wh, est la puissance attribuée au groupe, constante en
/// habitation ; elle est sommée sur les mois des scénarios (§4.1.3).
pub(super) fn ventilation_groupe(
    v: &VentilationProjet,
    g: usize,
    sref_m2: f64,
) -> ResultatsVentilationGroupe {
    let p_vent_w = v.p_vent_groupe_w(g);
    let mut mensuel = vec![0.0; 12];
    for h in 0..NB_HEURES_AN {
        let m = usize::from(date_scenario(h).mois) - 1;
        if let Some(x) = mensuel.get_mut(m) {
            *x += p_vent_w / 1000.0;
        }
    }
    let c_aux_vent_kwh: f64 = mensuel.iter().sum();
    let d = v.debits_groupe(g);
    let cond = v.debits_cond_groupe(g);
    ResultatsVentilationGroupe {
        q_spec_repris_m3_h: d.q_rep_m3_h,
        q_spec_souffle_m3_h: d.q_souffle_m3_h,
        q_repris_cond_m3_h: cond.q_rep_m3_h,
        q_souffle_cond_m3_h: cond.q_souffle_m3_h,
        p_vent_w,
        c_aux_vent_kwh,
        c_aux_vent_kwh_m2: if sref_m2 > 0.0 {
            c_aux_vent_kwh / sref_m2
        } else {
            0.0
        },
        c_aux_vent_mensuel_kwh: mensuel,
    }
}

/// Débits du système de ventilation conventionnel du Bbio pour un groupe d'habitation.
///
/// Th-BCE 2020 §6.2.3.1.3 et §6.2.3.2.1 (débits des bouches en Bbio), §6.1.3 (système
/// conventionnel à débit soufflé et extrait constant). Les autres usages (débits conventionnels
/// du tableau 56-1) ne sont pas pris en charge.
pub(super) fn debits_bbio_groupe(groupe: &Groupe, usage: UsageZone) -> Option<ResultatsDebitsBbio> {
    if !usage.est_habitation() || groupe.bouches_conduits.is_empty() {
        return None;
    }
    let bouches = debits_specifiques_bbio_groupe(&groupe.bouches_conduits)?;
    let d = debits_systeme_conventionnel(bouches, LectureDebitsBbio::default());
    Some(ResultatsDebitsBbio {
        q_spec_repris_m3_h: d.q_rep_m3_h,
        q_spec_souffle_m3_h: d.q_souffle_m3_h,
    })
}
