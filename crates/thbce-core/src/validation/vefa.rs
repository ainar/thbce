// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation du chauffage par défaut des travaux réservés par l'acquéreur (Th-BCE 2020 §8.33).
//! Les composants qu'il ajoute sont ensuite validés comme une saisie ordinaire.

use crate::chauffage_refroidissement::methode_vefa::PREFIXE_ID;
use crate::entree::{Batiment, ChauffageParDefautVefa};
use crate::validation::RapportValidation;

/// Th-BCE 2020 §8.33.1 et §8.33.4 : puissance, groupes desservis, identifiants réservés.
pub(super) fn valider_vefa(
    r: &mut RapportValidation,
    cb: &str,
    bat: &Batiment,
    c: &ChauffageParDefautVefa,
) {
    let cv = format!("{cb}.chauffage_par_defaut_vefa");
    r.strictement_positif(format!("{cv}.pn_gen_w"), c.pn_gen_w);
    if c.groupes.is_empty() {
        r.erreur(
            format!("{cv}.groupes"),
            None,
            "au moins un groupe desservi par le chauffage par défaut (Th-BCE 2020 §8.33)",
        );
    }
    r.ids_uniques(
        &format!("{cv}.groupes"),
        c.groupes.iter().map(|g| &g.groupe),
    );
    let reserve = |id: &str| id.starts_with(PREFIXE_ID);
    for (i, gen) in bat.generations.iter().enumerate() {
        if reserve(&gen.id) {
            r.erreur(
                format!("{cb}.generations[{i}].id"),
                Some(gen.id.clone()),
                format!("préfixe « {PREFIXE_ID} » réservé aux composants du chauffage par défaut (§8.33)"),
            );
        }
    }
    for (i, d) in bat.distributions_intergroupes.iter().enumerate() {
        if reserve(&d.id) {
            r.erreur(
                format!("{cb}.distributions_intergroupes[{i}].id"),
                Some(d.id.clone()),
                format!("préfixe « {PREFIXE_ID} » réservé aux composants du chauffage par défaut (§8.33)"),
            );
        }
    }
    for (i, gv) in c.groupes.iter().enumerate() {
        let cg = format!("{cv}.groupes[{i}]");
        r.strictement_positif(format!("{cg}.hsp_m"), gv.hsp_m);
        let trouve = bat.zones.iter().find_map(|z| {
            z.groupes
                .iter()
                .find(|g| g.id == gv.groupe)
                .map(|g| (z.usage, g))
        });
        let Some((usage, g)) = trouve else {
            r.erreur(
                format!("{cg}.groupe"),
                Some(gv.groupe.clone()),
                "groupe inconnu dans ce bâtiment",
            );
            continue;
        };
        // §8.33.1 : « système de chauffage par défaut en résidentiel ».
        if !usage.est_habitation() {
            r.erreur(
                format!("{cg}.groupe"),
                Some(gv.groupe.clone()),
                "le chauffage par défaut des travaux réservés par l'acquéreur ne vaut qu'en \
                 habitation (Th-BCE 2020 §8.33.1)",
            );
        }
        // §8.33.1 : « la saisie de ce système se réalise uniquement à la livraison du bâtiment
        // sans aucun générateur de chauffage ».
        if g.emetteurs.iter().any(|e| e.chauffage.is_some()) {
            r.erreur(
                format!("{cg}.groupe"),
                Some(gv.groupe.clone()),
                "le groupe a déjà un émetteur de chauffage : le chauffage par défaut ne se saisit \
                 que pour un logement livré sans chauffage (Th-BCE 2020 §8.33.1)",
            );
        }
        if g.emetteurs.iter().any(|e| reserve(&e.id))
            || g.distributions.iter().any(|d| reserve(&d.id))
        {
            r.erreur(
                format!("{cg}.groupe"),
                Some(gv.groupe.clone()),
                format!("préfixe « {PREFIXE_ID} » réservé aux composants du chauffage par défaut (§8.33)"),
            );
        }
    }
}
