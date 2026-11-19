// SPDX-License-Identifier: GPL-3.0-or-later
//! Restitution des résultats par zone et par groupe : besoins, saisons, confort d'été,
//! surfaces de référence.

use super::dh_le_plus_defavorable;
use super::emission_distribution::resultats_systemes;
use super::enveloppe::{defauts_groupe, enveloppe_groupe, resultats_baies};
use super::ventilation::{
    debits_bbio_groupe, puissance_horaire_w, resultats_mode, ventilation_groupe,
};
use crate::bati::assemblage_groupe::{GroupeHoraire, THETA_INITIALE_C};
use crate::bati::assemblage_zone::ZoneHoraire;
use crate::conventions::NB_HEURES_AN;
use crate::entree::{Groupe, Projet, UsageZone, Zone};
use crate::indicateurs::sorties_th_b::{sorties, EnergiesMensuelles};
use crate::resultats::{
    ResultatsConfort, ResultatsConfortEte, ResultatsGroupe, ResultatsSaisons,
    ResultatsVentilationZone, ResultatsZone,
};
use crate::ventilation::ventilation_projet::VentilationProjet;

/// Saisons propres d'un groupe à l'issue d'un calcul annuel (§4.6.3.6, éq. 92).
pub(super) fn resultats_saisons(groupe: &GroupeHoraire) -> ResultatsSaisons {
    let s = &groupe.etat.saisons;
    ResultatsSaisons {
        j_fin_ch: s.j_fin_ch,
        j_debut_ch: s.j_debut_ch,
        j_debut_fr: s.j_debut_fr,
        j_fin_fr: s.j_fin_fr,
    }
}

/// Confort d'été d'un groupe à l'issue du calcul Th-D (§13.5.3.7 et 13.5.3.8).
fn resultats_confort_ete(groupe: &GroupeHoraire) -> ResultatsConfortEte {
    let k = &groupe.cumuls.confort_ete;
    ResultatsConfortEte {
        dh_c_h: k.dh_c_h,
        nbh_inconf: k.nbh_inconf,
        nbh_inconf_1c: k.nbh_inconf_1c,
        nbh_inconf_2c: k.nbh_inconf_2c,
        nbh_occ_conf_adapt: k.nbh_occ_conf_adapt,
        theta_op_max_c: k.theta_op_max_c,
        theta_op_max_occ_c: k
            .theta_op_max_occ_c
            .is_finite()
            .then_some(k.theta_op_max_occ_c),
    }
}

/// Restitution des résultats d'une zone à partir de ses deux calculs annuels.
///
/// Th-BCE 2020 §13.1 (sorties du mode Th-B) et §13.5 (confort d'été). Renvoie aussi les
/// énergies mensuelles de la zone en mode Th-B, que le bâtiment somme (§13.1.3).
pub(super) fn restituer_zone(
    zone: &Zone,
    projet: &Projet,
    th_b: &ZoneHoraire,
    th_d: &ZoneHoraire,
    th_c: &ZoneHoraire,
) -> (ResultatsZone, EnergiesMensuelles) {
    let mut energies_zone = EnergiesMensuelles::default();
    // Système de ventilation du projet, préparé hors de la boucle horaire : en habitation, la
    // puissance des ventilateurs est constante (éq. 392 et 394, 641 et 643), et la
    // consommation se somme heure par heure (éq. 565, 645).
    // Hors habitation, la centrale double flux relève sa consommation pendant le calcul des
    // consommations (éq. 565 et 570) : le système du calcul Th-C est restitué.
    let ventilation = th_c
        .ventilation
        .clone()
        .filter(|v| v.releve.is_some())
        .or_else(|| VentilationProjet::nouvelle(zone, THETA_INITIALE_C));
    let groupes: Vec<ResultatsGroupe> = zone
        .groupes
        .iter()
        .zip(th_b.groupes.iter().zip(&th_d.groupes).zip(&th_c.groupes))
        .enumerate()
        .map(|(ig, (g, ((b, d), gc)))| {
            let sref_m2 = sref_groupe(g, zone.usage);
            let energies = energies_groupe(b);
            energies_zone.ajouter(&energies);
            let k = &b.cumuls.confort;
            ResultatsGroupe {
                id: g.id.clone(),
                sref_m2,
                enveloppe: enveloppe_groupe(g, &projet.catalogue, zone.usage),
                debits_bbio: debits_bbio_groupe(g, zone.usage),
                bbio: Some(sorties(&energies, sref_m2)),
                saisons_bbio: Some(resultats_saisons(b)),
                confort_bbio: Some(ResultatsConfort {
                    nbh_inc_cons_ch: k.nbh_inc_cons_ch,
                    dh_inc_cons_ch_k_h: k.dh_inc_cons_ch_k_h,
                    nbh_inc_cons_fr: k.nbh_inc_cons_fr,
                    dh_inc_cons_fr_k_h: k.dh_inc_cons_fr_k_h,
                    nbh_inc_c_fr: k.nbh_inc_c_fr,
                    dh_inc_c_fr_k_h: k.dh_inc_c_fr_k_h,
                }),
                confort_ete: Some(resultats_confort_ete(d)),
                saisons_th_d: Some(resultats_saisons(d)),
                ventilation: ventilation
                    .as_ref()
                    .map(|v| ventilation_groupe(v, ig, sref_m2, &th_c.scenarios)),
                systemes: resultats_systemes(gc),
                brasseurs: super::brasseurs_air::resultats_brasseurs(d, gc),
                consommations: None,
                baies: resultats_baies(g, &projet.catalogue, zone.usage),
                valeurs_par_defaut: {
                    let mut v = defauts_groupe(g, &projet.catalogue);
                    v.extend(super::brasseurs_air::defauts_brasseurs(g, zone.usage));
                    v
                },
            }
        })
        .collect();
    // Th-BCE 2020 §13.1.3, éq. (2377) : SREF_zn = somme des SREF_gr de la zone.
    let sref_m2: f64 = groupes.iter().map(|g| g.sref_m2).sum();
    let dh_c_h = dh_le_plus_defavorable(
        groupes
            .iter()
            .filter_map(|g| g.confort_ete.as_ref().map(|c| c.dh_c_h)),
    );
    (
        ResultatsZone {
            id: zone.id.clone(),
            sref_m2,
            // Adultes équivalents : usages 1 et 2 seulement (§4.1.3.6.3.1).
            nadeq: zone.usage.est_habitation().then_some(th_b.scenarios.nadeq),
            bbio: Some(sorties(&energies_zone, sref_m2)),
            dh_c_h,
            ventilation: ventilation.as_ref().map(|v| ResultatsVentilationZone {
                id: v.id.clone(),
                p_vent_w: v.p_vent_w(),
                // Puissance constante en habitation ; hors habitation, celle de l'occupation
                // ou de l'inoccupation à chaque pas de temps (éq. 636 à 639).
                c_aux_vent_kwh: match puissance_horaire_w(v, &th_c.scenarios) {
                    Some(serie) => serie.iter().sum::<f64>() / 1000.0,
                    None => v.p_vent_w() * NB_HEURES_AN as f64 / 1000.0,
                },
                th_d: th_d
                    .ventilation
                    .as_ref()
                    .map(|_| resultats_mode(v, &th_d.cumuls_ventilation)),
                th_c: th_c
                    .ventilation
                    .as_ref()
                    .map(|_| resultats_mode(v, &th_c.cumuls_ventilation)),
                valeurs_par_defaut: v.valeurs_par_defaut.clone(),
            }),
            consommations: None,
            consommations_collectif: None,
            groupes,
        },
        energies_zone,
    )
}

/// Énergies mensuelles d'un groupe à l'issue du calcul annuel.
fn energies_groupe(groupe: &GroupeHoraire) -> EnergiesMensuelles {
    EnergiesMensuelles {
        q_sys_ch_wh: groupe.cumuls.q_sys_ch_wh,
        q_sys_fr_wh: groupe.cumuls.q_sys_fr_wh,
        c_ecl_kwh: groupe.cumuls.c_ecl_kwh,
    }
}

/// Surface de référence d'un groupe.
///
/// Th-BCE 2020 §13.1.3, éq. (2375) et (2376) : `SREF_gr = SHAB_gr` si l'usage vaut 1 ou 2,
/// `SREF_gr = SU_gr` sinon. La validation garantit la présence de la surface requise.
fn sref_groupe(groupe: &Groupe, usage: UsageZone) -> f64 {
    let surface = if usage.est_habitation() {
        groupe.shab_m2
    } else {
        groupe.su_m2
    };
    surface.unwrap_or(0.0)
}
