// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des chaudières (Th-BCE 2020 §8.19), des poêles et des inserts (§8.22).

use crate::entree::{ChaudiereSaisie, PoeleSaisi};
use crate::validation::RapportValidation;

/// Th-BCE 2020 §8.19 : chaudière au gaz, au fioul ou au bois.
pub(super) fn valider_chaudiere(r: &mut RapportValidation, c: &str, x: &ChaudiereSaisie) {
    use crate::entree::TypeChaudiereSaisi as T;
    r.plage(format!("{c}.pn_gen_kw"), x.pn_gen_kw, 0.0, f64::MAX);
    r.strictement_positif(format!("{c}.pn_gen_kw"), x.pn_gen_kw);
    let bois = x.type_chaudiere == T::Bois;
    if bois && x.combustible_bois.is_none() {
        r.erreur(
            format!("{c}.combustible_bois"),
            None,
            "« TypeCombustibleBois » requis pour une chaudière au bois (Th-BCE 2020 §8.19.2)",
        );
    }
    if !bois && x.combustible_bois.is_some() {
        r.erreur(
            format!("{c}.combustible_bois"),
            None,
            "sans objet : le combustible bois ne concerne que les chaudières au bois",
        );
    }
    if !bois && x.gpl && matches!(x.type_chaudiere, T::FioulStandard | T::FioulCondensation) {
        r.erreur(
            format!("{c}.gpl"),
            None,
            "sans objet : le gaz de pétrole liquéfié ne concerne que les chaudières au gaz",
        );
    }
    if bois && x.gpl {
        r.erreur(
            format!("{c}.gpl"),
            None,
            "sans objet : le gaz de pétrole liquéfié ne concerne que les chaudières au gaz",
        );
    }
    let aux = &x.auxiliaires;
    if bois
        && x.alimentation_bois.is_none()
        && (aux.w_aux_nom_w.is_none() || aux.w_aux_int_w.is_none())
    {
        r.erreur(
            format!("{c}.alimentation_bois"),
            None,
            "« Alim_Chaudière_Bois » requis pour une chaudière au bois dont les auxiliaires sont \
             par défaut (Th-BCE 2020 §8.19.3.1.4, tableau 119)",
        );
    }
    if let Some(rd) = &x.rendements {
        for (nom, v) in [("r_pn_pct", rd.r_pn_pct), ("r_pint_pct", rd.r_pint_pct)] {
            if !(v > 0.0 && v < 100.0) {
                r.erreur(
                    format!("{c}.rendements.{nom}"),
                    Some(v.to_string()),
                    "Générateur à combustion : la saisie des performances du générateur à \
                     combustion conduit à des rendements sur PCS égaux ou supérieurs à l'unité \
                     (Th-BCE 2020 §8.19.3.1) ; rendement strictement positif",
                );
            }
        }
        if let Some(p) = rd.p_int_kw {
            r.strictement_positif(format!("{c}.rendements.p_int_kw"), p);
        }
    }
    for (nom, v) in [
        ("auxiliaires.w_aux_nom_w", aux.w_aux_nom_w),
        ("auxiliaires.w_aux_int_w", aux.w_aux_int_w),
        ("auxiliaires.w_veille_w", aux.w_veille_w),
        ("q_po_30_w", x.q_po_30_w),
    ] {
        if let Some(v) = v {
            r.plage(format!("{c}.{nom}"), v, 0.0, f64::MAX);
        }
    }
}

/// Th-BCE 2020 §8.22 : poêle ou insert.
pub(super) fn valider_poele(r: &mut RapportValidation, c: &str, x: &PoeleSaisi) {
    r.strictement_positif(format!("{c}.pn_gen_kw"), x.pn_gen_kw);
    r.plage(format!("{c}.pn_gen_kw"), x.pn_gen_kw, 0.0, 100.0);
    r.strictement_positif(format!("{c}.eta_pct"), x.eta_pct);
    r.plage(format!("{c}.eta_pct"), x.eta_pct, 0.0, 100.0);
    r.plage(format!("{c}.p_aux_vent_w"), x.p_aux_vent_w, 0.0, f64::MAX);
}
