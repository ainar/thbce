// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des zones et des groupes, et cohérence des composants d'une zone.

use super::baies::valider_baie;
use super::enveloppe::{valider_inertie, valider_paroi, valider_pont_thermique};
use super::ventilation::{
    valider_aeraulique_zone, valider_bouche_conduit, valider_entrees_air, valider_ventilation,
};
use super::{nom_json, RapportValidation, References};
use crate::bati::assemblage_groupe::EnveloppeResolue;
use crate::bati::baie::EnsembleBaies;
use crate::bati::comportement_thermique::GroupeThermique;
use crate::bati::paroi_opaque::EnsembleParois;
use crate::bati::surventilation::traversant_conventionnel;
use crate::entree::{Catalogue, Groupe, Zone};
use crate::ventilation::bouche_conduit::debits_specifiques_bbio_groupe;

pub(super) fn valider_zone(r: &mut RapportValidation, c: &str, zone: &Zone, refs: &References<'_>) {
    if !zone.usage.est_pris_en_charge() {
        r.hors_perimetre(
            format!("{c}.usage"),
            Some(nom_json(&zone.usage)),
            format!(
                "usage n° {} (seuls la maison individuelle ou accolée et le logement collectif \
                 sont calculés)",
                zone.usage.numero()
            ),
        );
    }
    // Th-BCE 2020 §4.1.3, éq. (34) : le nombre de logements est nécessaire en logement collectif.
    // En maison individuelle, il vaut 1 s'il n'est pas saisi (maisons accolées : à saisir).
    match (zone.usage, zone.nb_logements) {
        (_, Some(0)) => r.erreur(
            format!("{c}.nb_logements"),
            Some("0".into()),
            "doit être supérieur ou égal à 1",
        ),
        (crate::entree::UsageZone::LogementCollectif, None) => r.erreur(
            format!("{c}.nb_logements"),
            None,
            "nombre de logements requis en logement collectif (Th-BCE 2020 §4.1.3, éq. 34)",
        ),
        _ => {}
    }
    valider_aeraulique_zone(r, c, zone);
    if zone.usage.est_habitation() {
        match &zone.ventilation {
            None => r.erreur(
                format!("{c}.ventilation"),
                None,
                "système de ventilation requis en habitation : il sert aux calculs du confort \
                 d'été et des consommations (Th-BCE 2020 §6.2 à 6.5)",
            ),
            Some(v) => valider_ventilation(r, &format!("{c}.ventilation"), v, zone),
        }
    }
    if zone.groupes.is_empty() {
        r.erreur(
            format!("{c}.groupes"),
            None,
            "la zone doit contenir au moins un groupe",
        );
    }
    r.ids_uniques(&format!("{c}.groupes"), zone.groupes.iter().map(|g| &g.id));
    for (ig, groupe) in zone.groupes.iter().enumerate() {
        valider_groupe(r, &format!("{c}.groupes[{ig}]"), groupe, zone, refs);
    }
}

fn valider_groupe(
    r: &mut RapportValidation,
    c: &str,
    g: &Groupe,
    zone: &Zone,
    refs: &References<'_>,
) {
    // Th-BCE 2020 §4.1.2 : la surface utile du groupe, Agr, sert à tous les usages (apports
    // internes, répartition entre groupes). §13.1.3, éq. (2375) et (2376) : la surface de
    // référence est SHAB en habitation, SU sinon.
    match g.su_m2 {
        Some(s) => r.strictement_positif(format!("{c}.su_m2"), s),
        None => r.erreur(
            format!("{c}.su_m2"),
            None,
            "surface utile du groupe requise (Agr, Th-BCE 2020 §4.1.2)",
        ),
    }
    match (zone.usage.est_habitation(), g.shab_m2) {
        (_, Some(s)) => r.strictement_positif(format!("{c}.shab_m2"), s),
        (true, None) => r.erreur(
            format!("{c}.shab_m2"),
            None,
            "surface habitable requise en habitation : c'est la surface de référence (Th-BCE 2020 §13.1.3)",
        ),
        (false, None) => {}
    }
    // Th-BCE 2020 §8.3.2 : volume du groupe, paramètre d'intégration du bilan hydrique.
    match g.v_m3 {
        Some(v) => r.strictement_positif(format!("{c}.v_m3"), v),
        None if zone.usage.est_pris_en_charge() => r.erreur(
            format!("{c}.v_m3"),
            None,
            "volume du groupe requis (V, Th-BCE 2020 §8.3.2)",
        ),
        None => {}
    }
    valider_inertie(r, &format!("{c}.inertie"), &g.inertie);

    r.ids_uniques(
        &format!("{c}.parois_opaques"),
        g.parois_opaques.iter().map(|p| &p.id),
    );
    for (i, p) in g.parois_opaques.iter().enumerate() {
        valider_paroi(r, &format!("{c}.parois_opaques[{i}]"), p, refs);
    }
    r.ids_uniques(&format!("{c}.baies"), g.baies.iter().map(|b| &b.id));
    for (i, b) in g.baies.iter().enumerate() {
        valider_baie(r, &format!("{c}.baies[{i}]"), b, refs);
    }
    // Th-BCE 2020 §5.13.3.2.1, tableau 43, et §5.13.3.2.2.
    let a_des_ouvrants = g.baies.iter().any(|b| {
        refs.types_baies
            .get(b.type_baie.as_str())
            .is_some_and(|t| t.ouvrant.is_some())
    });
    match (traversant_conventionnel(zone.usage.numero()), g.traversant) {
        (Some(conv), Some(saisi)) if conv != saisi => r.erreur(
            format!("{c}.traversant"),
            Some(saisi.to_string()),
            "le caractère traversant est conventionnel pour cet usage (tableau 43) : ne pas le \
             saisir",
        ),
        (None, None) if a_des_ouvrants && zone.usage.est_pris_en_charge() => r.erreur(
            format!("{c}.traversant"),
            None,
            "caractère traversant du groupe requis en logement collectif quand des baies sont \
             ouvrables (Th-BCE 2020 §5.13.3.2.1)",
        ),
        _ => {}
    }
    if let Some(h) = g.h_ttf_m {
        if !(h > 4.0 && h <= 15.0) {
            r.erreur(
                format!("{c}.h_ttf_m"),
                Some(h.to_string()),
                "la hauteur de tirage thermique ne se saisit que si elle dépasse 4 m, dans la \
                 limite de 15 m ; sinon elle vaut 1,5 m (Th-BCE 2020 §5.13.3.2.2)",
            );
        }
    }
    r.ids_uniques(
        &format!("{c}.ponts_thermiques"),
        g.ponts_thermiques.iter().map(|p| &p.id),
    );
    for (i, p) in g.ponts_thermiques.iter().enumerate() {
        valider_pont_thermique(r, &format!("{c}.ponts_thermiques[{i}]"), p, refs);
    }
    // Th-BCE 2020 §6.2.3.1.3 : en habitation, les débits du Bbio sont les débits d'hygiène des
    // bouches du groupe.
    if zone.usage.est_habitation() && g.bouches_conduits.is_empty() {
        r.erreur(
            format!("{c}.bouches_conduits"),
            None,
            "au moins une bouche de ventilation requise en habitation : ses débits d'hygiène de \
             pointe et de base donnent les débits du Bbio (Th-BCE 2020 §6.2.3.1.1.2)",
        );
    }
    r.ids_uniques(
        &format!("{c}.bouches_conduits"),
        g.bouches_conduits.iter().map(|b| &b.id),
    );
    for (i, b) in g.bouches_conduits.iter().enumerate() {
        valider_bouche_conduit(r, &format!("{c}.bouches_conduits[{i}]"), b);
    }
    if let Some(e) = &g.entrees_air {
        valider_entrees_air(r, &format!("{c}.entrees_air"), e);
    }
}

/// Contrôles qui portent sur l'assemblage des composants d'une zone calculée.
///
/// 1. Th-BCE 2020 §5.21.3.6, éq. (339) : `Hgemq = 1/(1/HTH,k+pt − 1/Hgmqs)` n'a de sens que si
///    l'enveloppe opaque du groupe conduit moins que l'échange superficiel de ses parois
///    lourdes, `HTH,k+pt < Hgmqs = his × Amq` (éq. 326) ; sinon le modèle thermique n'a pas de
///    solution (décision Q34 point 5).
/// 2. Th-BCE 2020 §5.6.3.5, éq. (167) : la pression intérieure équilibre les débits massiques
///    de la zone par ses défauts d'étanchéité et ses entrées d'air. Une zone qui n'a ni les uns
///    ni les autres ne peut pas équilibrer des débits soufflés et repris de masses différentes
///    (décision Q37 point 11) ; or l'air soufflé et l'air repris n'ont pas la même masse
///    volumique, même à débits volumiques égaux.
pub(super) fn valider_coherence_zone(
    r: &mut RapportValidation,
    c: &str,
    zone: &Zone,
    catalogue: &Catalogue,
) {
    let mut surface_permeable = 0.0;
    let mut module_entrees_air = 0.0;
    let mut debits_specifiques = 0.0;
    for (ig, g) in zone.groupes.iter().enumerate() {
        let enveloppe = EnveloppeResolue::de(g, catalogue, zone.usage);
        let op = EnsembleParois::de(&enveloppe.parois);
        let ba = EnsembleBaies::de(&enveloppe.baies);
        surface_permeable += op.a_opv_m2 + ba.a_baies_v_m2 + op.a_ophh_m2 + ba.a_baies_h_m2;
        module_entrees_air += g.entrees_air.as_ref().map_or(0.0, |e| e.m_m3_h);
        if let Some(d) = debits_specifiques_bbio_groupe(&g.bouches_conduits) {
            debits_specifiques += d.q_souffle_m3_h - d.q_rep_m3_h;
        }
        let Some(a_gr_m2) = g.su_m2 else { continue };
        let h_th_k_pt = enveloppe.h_th_k_pt_w_k();
        let thermique = GroupeThermique::nouveau(a_gr_m2, ba.a_baies_m2, &g.inertie.quotidienne);
        if thermique.hg_emq_w_k(h_th_k_pt).is_none() {
            r.erreur(
                format!("{c}.groupes[{ig}]"),
                Some(g.id.clone()),
                format!(
                    "groupe « {} » : l'enveloppe opaque conduit plus que l'échange superficiel \
                     des parois lourdes : HTH,k+pt = {h_th_k_pt:.1} W/K (parois opaques et ponts \
                     thermiques, éq. 340) pour Hgmqs = {:.1} W/K (his × Amq, éq. 326) ; le \
                     modèle thermique demande HTH,k+pt < Hgmqs (Th-BCE 2020 §5.21.3.6, \
                     éq. 339) ; vérifier les surfaces et les coefficients des parois et des \
                     ponts thermiques, la surface utile et l'inertie quotidienne",
                    g.id, thermique.hg_mqs_w_k
                ),
            );
        }
    }
    let permeabilite = surface_permeable * zone.q4pa_surf_m3_h_m2.unwrap_or(0.0);
    if permeabilite <= 0.0 && module_entrees_air <= 0.0 && debits_specifiques > 0.0 {
        r.erreur(
            format!("{c}.q4pa_surf_m3_h_m2"),
            zone.q4pa_surf_m3_h_m2.map(|q| q.to_string()),
            format!(
                "zone « {} » : sans perméabilité à l'air (perméabilité nulle ou aucune paroi \
                 déperditive hors plancher bas) ni entrée d'air, les débits massiques soufflés \
                 et repris par la ventilation ne peuvent pas s'équilibrer : le bilan aéraulique \
                 n'a pas de solution (Th-BCE 2020 §5.6.3.5, éq. 167) ; saisir une perméabilité \
                 non nulle ou des entrées d'air",
                zone.id
            ),
        );
    }
}
