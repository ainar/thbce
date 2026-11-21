// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation de l'échangeur de géocooling (Th-BCE 2020 §8.30).

use super::thermodynamique::valider_source_amont;
use crate::entree::{Batiment, Generation, GeocoolingSaisi, NatureGenerateur, TypePriorite};
use crate::validation::RapportValidation;

/// Th-BCE 2020 §8.30.1 et §8.30.2 (tableau 253) : plages, source géothermique sur eau,
/// génération en cascade dont l'échangeur est le premier générateur de froid (lecture Q252,
/// point 2).
pub(super) fn valider_geocooling(
    r: &mut RapportValidation,
    c: &str,
    geo: &GeocoolingSaisi,
    id: &str,
    g: &Generation,
    bat: &Batiment,
) {
    r.strictement_positif(format!("{c}.qv_aval_nom_m3_h"), geo.qv_aval_nom_m3_h);
    r.strictement_positif(format!("{c}.qv_amont_nom_m3_h"), geo.qv_amont_nom_m3_h);
    // Δθaval divise la demande (éq. 1574) : strictement positif.
    r.strictement_positif(format!("{c}.delta_theta_aval_k"), geo.delta_theta_aval_k);
    r.plage(
        format!("{c}.delta_theta_aval_k"),
        geo.delta_theta_aval_k,
        0.0,
        8.0,
    );
    r.plage(
        format!("{c}.delta_theta_amont_k"),
        geo.delta_theta_amont_k,
        0.0,
        8.0,
    );
    r.plage(
        format!("{c}.theta_r_source_lim_c"),
        geo.theta_r_source_lim_c,
        0.0,
        25.0,
    );
    r.positif(format!("{c}.p_cprim_nom_w"), geo.p_cprim_nom_w);
    if let Some(e) = geo.echangeur_intermediaire {
        r.strictement_positif(
            format!("{c}.echangeur_intermediaire.ua_geo_nom_w_k"),
            e.ua_geo_nom_w_k,
        );
    }
    // « idfluide_amont = 1 : eau » : source géothermique (sondes ou nappe, fiche 8.26).
    let source = valider_source_amont(r, c, &geo.source_amont, bat);
    if !matches!(source, "captage" | "nappe") {
        r.erreur(
            format!("{c}.source_amont"),
            Some(source.to_owned()),
            "la source amont d'un échangeur de géocooling est géothermique sur eau : captage par \
             sondes ou nappe (Th-BCE 2020 §8.30.2, « idfluide_amont = 1 : eau »)",
        );
    }
    // §8.30.1 : génération « en cascade », géocooling « systématiquement prioritaire par
    // rapport aux autres générateurs de froid ».
    if g.priorite != Some(TypePriorite::Cascade) {
        r.erreur(
            c.to_owned(),
            Some(id.to_owned()),
            "une génération pourvue d'un échangeur de géocooling est régulée en cascade (Th-BCE \
             2020 §8.30.1)",
        );
        return;
    }
    let rang = |x: &crate::entree::GenerateurSaisi| x.priorite_refroidissement.unwrap_or(1);
    let Some(moi) = g.generateurs.iter().find(|x| x.id == id) else {
        return;
    };
    let devance = g.generateurs.iter().any(|x| {
        x.id != id
            && super::postes_assures(&x.nature)[1]
            && (rang(x) <= rang(moi) || matches!(x.nature, NatureGenerateur::Geocooling(_)))
    });
    if devance {
        r.erreur(
            c.to_owned(),
            Some(id.to_owned()),
            "l'échangeur de géocooling est le seul générateur de froid de premier rang de sa \
             génération (`priorite_refroidissement` strictement inférieure à celle des autres ; \
             Th-BCE 2020 §8.30.1 : « systématiquement prioritaire »)",
        );
    }
}
