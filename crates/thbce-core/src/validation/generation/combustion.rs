// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des chaudières (Th-BCE 2020 §8.19), des autres générateurs à combustion (§8.20),
//! des poêles et des inserts (§8.22).

use crate::entree::{
    AutreGenerateurCombustionSaisi, ChaudiereSaisie, PoeleSaisi, RaccordementGenerateur,
    TypePriorite,
};
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

/// Vrai pour un chauffe-eau ou un accumulateur au gaz (ECS sur eau, tableau 124).
pub(super) fn autre_combustion_ecs(x: &AutreGenerateurCombustionSaisi) -> bool {
    use crate::entree::TypeAutreGenerateurCombustion as T;
    matches!(
        x.type_generateur,
        T::ChauffeEauGaz | T::AccumulateurGaz { .. }
    )
}

/// Th-BCE 2020 §8.20 : autre générateur à combustion (types 103 à 109).
pub(super) fn valider_autre_combustion(
    r: &mut RapportValidation,
    c: &str,
    x: &AutreGenerateurCombustionSaisi,
    priorite: Option<TypePriorite>,
) {
    use crate::chauffage_refroidissement::calculs_generation::Energie;
    use crate::chauffage_refroidissement::chaudieres_gaz_fioul_bois::pcsi;
    use crate::entree::{StatutRendementCombustion as S, TypeAutreGenerateurCombustion as T};
    r.strictement_positif(format!("{c}.pn_gen_kw"), x.pn_gen_kw);
    r.plage(format!("{c}.pn_gen_kw"), x.pn_gen_kw, 0.0, f64::MAX);
    r.plage(format!("{c}.w_veille_w"), x.w_veille_w, 0.0, f64::MAX);
    for (nom, v) in [
        ("w_aux_nom_w", x.w_aux_nom_w),
        ("p_veilleuse_w", x.p_veilleuse_w),
        ("q_po_30_w", x.q_po_30_w),
    ] {
        if let Some(v) = v {
            r.plage(format!("{c}.{nom}"), v, 0.0, f64::MAX);
        }
    }
    let ecs = autre_combustion_ecs(x);
    let air_chaud = matches!(x.type_generateur, T::AirChaud { .. });
    if let Some(t) = x.theta_fonct_min_c {
        if ecs {
            r.plage(format!("{c}.theta_fonct_min_c"), t, 20.0, 100.0);
        } else {
            r.erreur(
                format!("{c}.theta_fonct_min_c"),
                None,
                "sans objet : seuls les générateurs sur eau (chauffe-eau, accumulateurs) sont \
                 limités par une température minimale de fonctionnement (Th-BCE 2020 §8.20.3.2.2)",
            );
        }
    }
    if let Some(p) = x.p_int_kw {
        if air_chaud {
            r.strictement_positif(format!("{c}.p_int_kw"), p);
            if p >= x.pn_gen_kw {
                r.erreur(
                    format!("{c}.p_int_kw"),
                    Some(p.to_string()),
                    "« Pint » doit être inférieure à « Pngen » (Th-BCE 2020 §8.20.2)",
                );
            }
        } else {
            r.erreur(
                format!("{c}.p_int_kw"),
                None,
                "sans objet : « Pint » ne concerne que les générateurs d'air chaud (Th-BCE 2020 \
                 §8.20.3.6.1)",
            );
        }
    }
    if let T::RadiateurGaz {
        ventilateur_combustion,
        evacuation_fumees,
        ..
    } = x.type_generateur
    {
        if !ventilateur_combustion && evacuation_fumees.is_none() {
            r.erreur(
                format!("{c}.type_generateur.radiateur_gaz.evacuation_fumees"),
                None,
                "évacuation des fumées requise sans ventilateur côté combustion (Th-BCE 2020 \
                 §8.20.3.6.2.4, tableau 129)",
            );
        }
    }
    // Tableau 128 : pas de valeur par défaut des auxiliaires d'un panneau radiant, ni d'un tube
    // radiant sans ventilateur côté émission.
    let sans_defaut_aux = matches!(
        x.type_generateur,
        T::PanneauRadiant
            | T::TubeRadiant {
                ventilateur_emission: false
            }
    );
    if sans_defaut_aux && x.w_aux_nom_w.is_none() {
        r.erreur(
            format!("{c}.w_aux_nom_w"),
            None,
            "« Waux,nom » requis : le tableau 128 ne donne pas de puissance d'auxiliaires par \
             défaut pour ce générateur (Th-BCE 2020 §8.20.3.4)",
        );
    }
    let pcsi = pcsi(Energie::Gaz, x.gpl);
    if let Some(rd) = &x.rendements {
        if ecs && rd.statut != S::Certifie {
            r.erreur(
                format!("{c}.rendements.statut"),
                None,
                "pour un chauffe-eau ou un accumulateur au gaz, le rendement est certifié ou par \
                 défaut (Th-BCE 2020 §8.20.3.7.1)",
            );
        }
        if rd.r_pint_pct.is_some() && !air_chaud {
            r.erreur(
                format!("{c}.rendements.r_pint_pct"),
                None,
                "sans objet : le rendement à charge intermédiaire ne concerne que les générateurs \
                 d'air chaud (Th-BCE 2020 §8.20.3.6.2.1)",
            );
        }
        let mut max = rd.r_pn_pct;
        for (nom, v) in [
            ("r_pn_pct", Some(rd.r_pn_pct)),
            ("r_pint_pct", rd.r_pint_pct),
        ] {
            let Some(v) = v else { continue };
            max = max.max(v);
            if !(v > 0.0 && v <= 150.0) {
                r.erreur(
                    format!("{c}.rendements.{nom}"),
                    Some(v.to_string()),
                    "rendement sur PCI strictement positif et au plus 150 % (Th-BCE 2020 \
                     §8.20.2, tableau 123)",
                );
            }
        }
        // Éq. (1120).
        if max / pcsi >= 100.0 {
            r.erreur(
                format!("{c}.rendements"),
                Some(max.to_string()),
                "La saisie des performances du générateur à combustion conduit à des rendements \
                 sur PCS égaux ou supérieurs à l'unité (Th-BCE 2020 §8.20.3.3, éq. 1120)",
            );
        }
    }
    // Lecture Q156 : « idraccord_gnr » n'intervient que pour une gestion avec priorité.
    if x.raccordement == Some(RaccordementGenerateur::Permanent)
        && !matches!(
            priorite,
            Some(TypePriorite::Cascade | TypePriorite::Alternance)
        )
    {
        r.erreur(
            format!("{c}.raccordement"),
            None,
            "un raccordement permanent (« idraccord_gnr » = 0) ne vaut que pour une gestion avec \
             priorité, en cascade ou en alternance (Th-BCE 2020 §8.20.3.8.1 ; lecture Q156)",
        );
    }
}
