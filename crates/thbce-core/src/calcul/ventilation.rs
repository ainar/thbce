// SPDX-License-Identifier: GPL-3.0-or-later
//! Restitution des résultats de la ventilation.

use crate::bati::assemblage_zone::CumulsVentilation;
use crate::climat::calendrier::date_scenario;
use crate::conventions::NB_HEURES_AN;
use crate::entree::{Groupe, UsageZone};
use crate::resultats::{ResultatsDebitsBbio, ResultatsVentilationGroupe, ResultatsVentilationMode};
use crate::scenarios::ScenariosZone;
use crate::ventilation::bouche_conduit::{
    debits_bbio_non_residentiel, debits_specifiques_bbio_groupe,
};
use crate::ventilation::ven_bbio::{debits_systeme_conventionnel, LectureDebitsBbio};
use crate::ventilation::ventilation_projet::VentilationProjet;

/// Ventilation du projet vue d'un groupe : débits et consommation des ventilateurs.
///
/// Th-BCE 2020 §6.2.3.4 (éq. 428 à 431) ; §6.3.3.4.13 (éq. 565 et 570) et §6.5.3.3 (éq. 645 et
/// 646) : la consommation horaire, en Wh, est la puissance attribuée au groupe, constante en
/// habitation ; elle est sommée sur les mois des scénarios (§4.1.3).
///
/// Hors habitation, la puissance est celle de l'occupation ou de l'inoccupation selon
/// l'indicateur de ventilation de la zone (éq. 636 à 639) ; `p_vent_w` restitue alors celle de
/// l'occupation.
/// Fonctionnement annuel du système de ventilation du projet dans un calcul (éq. 494, 508,
/// 523 et 545).
pub(super) fn resultats_mode(
    v: &VentilationProjet,
    c: &CumulsVentilation,
) -> ResultatsVentilationMode {
    // §6.3.3.5 : batteries régulées sur la charge.
    let batteries = v.cta.as_ref().and_then(|x| x.batteries_charge);
    ResultatsVentilationMode {
        nbh_bypass: c.nbh_bypass,
        q_ag_kwh: c.w_ag_wh / 1000.0,
        q_prechaud_kwh: v.prechauffage.as_ref().map(|_| c.q_prechaud_wh / 1000.0),
        q_prefroid_kwh: v
            .prerefroidissement
            .as_ref()
            .map(|_| c.q_prefroid_wh / 1000.0),
        nbh_mode_actif: batteries.map(|_| c.nbh_mode_actif),
        nbh_sous_dimensionnement: batteries.map(|_| c.nbh_sous_dimensionnement),
        q_chaud_kwh: batteries.filter(|b| b.chaud).map(|_| c.q_chaud_wh / 1000.0),
        q_froid_kwh: batteries
            .and_then(|b| b.froid)
            .map(|_| c.q_froid_wh / 1000.0),
    }
}

pub(super) fn ventilation_groupe(
    v: &VentilationProjet,
    g: usize,
    sref_m2: f64,
    scenarios: &ScenariosZone,
) -> ResultatsVentilationGroupe {
    let p_vent_w = v.p_vent_groupe_w(g);
    let p_vent_inocc_w = v.p_vent_groupe_selon_w(g, false);
    let selon_occupation = v.selon_occupation();
    let mut mensuel = vec![0.0; 12];
    // Hors habitation, consommation relevée pendant le calcul des consommations (éq. 570).
    if let Some(r) = v.releve.as_ref().and_then(|r| r.mensuel_groupes_kwh.get(g)) {
        mensuel.copy_from_slice(r);
    }
    for h in (0..NB_HEURES_AN).filter(|_| v.releve.is_none()) {
        let m = usize::from(date_scenario(h).mois) - 1;
        let p = if selon_occupation && scenarios.evaluer(h).i_vent <= 0.0 {
            p_vent_inocc_w
        } else {
            p_vent_w
        };
        if let Some(x) = mensuel.get_mut(m) {
            *x += p / 1000.0;
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

/// « Pvent(h) » d'un système dont la puissance dépend de l'indicateur de ventilation de la
/// zone (usages autres que l'habitation, §6.5.3.3, éq. 636 à 639), en W, c'est-à-dire en Wh par
/// pas de temps, pour les 8 760 pas de temps. `None` en habitation, où la puissance est
/// constante.
pub(super) fn puissance_horaire_w(
    v: &VentilationProjet,
    scenarios: &ScenariosZone,
) -> Option<Vec<f64>> {
    if let Some(r) = &v.releve {
        return Some(r.serie_wh.clone());
    }
    v.selon_occupation().then(|| {
        (0..NB_HEURES_AN)
            .map(|h| {
                let (rep, sou) = v.p_vent_selon(scenarios.evaluer(h).i_vent > 0.0);
                rep + sou
            })
            .collect()
    })
}

/// Débits du système de ventilation conventionnel du Bbio pour un groupe.
///
/// Th-BCE 2020 §6.2.3.1.3 et §6.2.3.2.1 (débits des bouches en Bbio), §6.1.3 (système
/// conventionnel à débit soufflé et extrait constant). Hors habitation, débits conventionnels
/// du tableau 56-1 : ceux de l'occupation sont restitués.
pub(super) fn debits_bbio_groupe(groupe: &Groupe, usage: UsageZone) -> Option<ResultatsDebitsBbio> {
    if !usage.est_habitation() {
        let (occ, _) = debits_bbio_non_residentiel(usage.numero(), groupe.su_m2?)?;
        return Some(ResultatsDebitsBbio {
            q_spec_repris_m3_h: occ.q_rep_m3_h,
            q_spec_souffle_m3_h: occ.q_souffle_m3_h,
        });
    }
    if groupe.bouches_conduits.is_empty() {
        return None;
    }
    let bouches = debits_specifiques_bbio_groupe(&groupe.bouches_conduits)?;
    let d = debits_systeme_conventionnel(bouches, LectureDebitsBbio::default());
    Some(ResultatsDebitsBbio {
        q_spec_repris_m3_h: d.q_rep_m3_h,
        q_spec_souffle_m3_h: d.q_souffle_m3_h,
    })
}
