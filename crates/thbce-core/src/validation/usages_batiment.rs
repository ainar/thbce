// SPDX-License-Identifier: GPL-3.0-or-later
//! Cohérence entre les usages saisis et la description du bâtiment : fiches 4.2
//! (« FA_Maison_individuelle_Logement_Collectif ») et 4.3 (« FA_Comment identifier l'usage
//! d'un bâtiment et l'exigence associée ») de l'annexe III, définitions de l'annexe I de
//! l'arrêté du 4 août 2021, fiche d'application FA05 v2 (Q85).
//!
//! L'entrée ne décrit ni les logements un à un (superposition, porte d'entrée commune) ni les
//! locaux : l'usage reste une règle de saisie (doc-commentaire de `usage`), et ces contrôles
//! ne lèvent que des avertissements, les configurations signalées restant possibles
//! (bâtiments accolés, calcul par partie de bâtiment, extension).

use super::RapportValidation;
use crate::entree::{Batiment, UsageZone};

/// `Sref` d'une zone d'après la saisie : SHAB en habitation, SU sinon (§13.1.3, éq. 2375 et
/// 2376) ; une surface absente compte pour 0 (la validation de la zone la signale).
fn sref_zone(z: &crate::entree::Zone) -> f64 {
    z.groupes
        .iter()
        .map(|g| {
            if z.usage.est_habitation() {
                g.shab_m2
            } else {
                g.su_m2
            }
            .unwrap_or(0.0)
        })
        .sum()
}

/// Contrôles des fiches 4.2 et 4.3 pour le bâtiment `bat` (chemin `cb`).
pub(super) fn valider_usages_batiment(r: &mut RapportValidation, cb: &str, bat: &Batiment) {
    let a = |u: UsageZone| bat.zones.iter().any(|z| z.usage == u);
    let extension = bat
        .exigences
        .as_ref()
        .is_some_and(|e| e.extension.is_some());
    let maisons = a(UsageZone::MaisonIndividuelle);
    let collectifs = a(UsageZone::LogementCollectif);

    // Fiche 4.2.3 : « un bâtiment à usage collectif d'habitation ne peut pas contenir des
    // parties à usage de maison individuelle groupée voire de maison individuelle » ; annexe I :
    // des bâtiments accolés forment un bâtiment unique sans changer l'usage de chaque partie.
    if maisons && collectifs {
        r.avertissement(
            format!("{cb}.zones"),
            None,
            "zones de maison individuelle et de logements collectifs dans le même bâtiment : un \
             bâtiment d'habitation est soit collectif, soit de maisons (annexe III, fiche 4.2) ; \
             ce n'est possible que pour des bâtiments accolés, dont chaque partie garde son \
             usage (annexe I de l'arrêté du 4 août 2021)",
        );
    }

    // Annexe I : un bâtiment d'habitation collectif regroupe « strictement plus de deux
    // logements partiellement ou totalement superposés » ; un bâtiment qui n'en a pas plus de
    // deux est une maison individuelle (figure 11). Un logement de fonction dans un bâtiment
    // d'un autre usage peut former une zone de logements collectifs (figure 11, renvoi *) :
    // contrôle limité aux bâtiments d'habitation seuls, hors extensions (l'usage d'une
    // extension est celui du bâtiment existant).
    let habitation_seule = bat.zones.iter().all(|z| z.usage.est_habitation());
    if collectifs && habitation_seule && !extension {
        let nl: u32 = bat
            .zones
            .iter()
            .map(|z| match z.usage {
                UsageZone::LogementCollectif => z.nb_logements.unwrap_or(0),
                _ => z.nb_logements.unwrap_or(1),
            })
            .sum();
        if nl <= 2 {
            r.avertissement(
                format!("{cb}.zones"),
                Some(nl.to_string()),
                "bâtiment d'habitation de deux logements au plus décrit en logements collectifs : \
                 un bâtiment d'habitation collectif regroupe strictement plus de deux logements \
                 superposés (annexe I de l'arrêté du 4 août 2021 ; annexe III, fiche 4.2, figure \
                 11) ; sinon c'est une maison individuelle ou un bâtiment de maisons accolées, \
                 sauf calcul d'une partie d'un bâtiment collectif",
            );
        }
    }

    // Figure 11, renvoi (*) : un logement dans un bâtiment à usage autre que d'habitation
    // forme une zone de logements collectifs ou prend l'usage du bâtiment (article 2) ; FA05 v2 :
    // « Logements de fonction dans un bâtiment autre que d'habitation : Logements collectifs ».
    if maisons {
        let (mut s_hab, mut s_autres) = (0.0, 0.0);
        for z in &bat.zones {
            if z.usage.est_habitation() {
                s_hab += sref_zone(z);
            } else {
                s_autres += sref_zone(z);
            }
        }
        if s_autres > s_hab {
            for (iz, z) in bat.zones.iter().enumerate() {
                if z.usage == UsageZone::MaisonIndividuelle {
                    r.avertissement(
                        format!("{cb}.zones[{iz}].usage"),
                        Some("maison_individuelle".into()),
                        "zone de maison individuelle dans un bâtiment d'usage principal autre que \
                         d'habitation : un logement de fonction y forme une zone de logements \
                         collectifs ou prend l'usage principal (annexe III, fiche 4.2, figure \
                         11 ; article 2 de l'arrêté du 4 août 2021 ; FA05 v2), sauf maison \
                         accolée à un bâtiment d'un autre usage (annexe I)",
                    );
                }
            }
        }
    }

    // FA05 v2 : les vestiaires des établissements sportifs sont compris dans les scénarios des
    // usages 25 et 28 ; « il ne doit donc pas être saisi un bâtiment multi-usages avec une zone
    // vestiaires seuls pour ces établissements sportifs ».
    if a(UsageZone::Vestiaires)
        && (a(UsageZone::SportMunicipalScolaire) || a(UsageZone::SportPrive))
    {
        r.avertissement(
            format!("{cb}.zones"),
            None,
            "zone de vestiaires seuls (usage 18) dans un établissement sportif (usages 25 ou 28) : \
             ces vestiaires sont compris dans les scénarios des établissements sportifs et ne se \
             saisissent pas à part (FA05 v2)",
        );
    }

    // Fiche 4.3.3, 5) : une zone regroupe tous les groupes d'un même usage (et, pour certains
    // usages, d'un même fonctionnement, que les numéros d'usage distinguent déjà) ; FA05 v2
    // admet deux zones de logements collectifs, traversante et non traversante.
    for (iz, z) in bat.zones.iter().enumerate() {
        if z.usage != UsageZone::LogementCollectif
            && bat.zones[..iz].iter().any(|x| x.usage == z.usage)
        {
            r.avertissement(
                format!("{cb}.zones[{iz}].usage"),
                Some(crate::validation::nom_json(&z.usage)),
                "deuxième zone du même usage dans le bâtiment : une zone regroupe tous les \
                 groupes d'un même usage (annexe III, fiche 4.3.3 ; FA05 v2), les parties \
                 climatisées ou d'expositions au bruit différentes formant des groupes",
            );
        }
    }
}
