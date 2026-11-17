// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation de la ventilation (Th-BCE 2020 §6) : systèmes, entrées d'air, bouches et
//! conduits, cohérence aéraulique de la zone.

use super::{nom_json, RapportValidation};
use crate::entree::{
    BoucheConduit, EntreesAir, Isouf, RepresentationEchangeur, SystemeVentilation, Ventilation,
    Zone,
};
use crate::ventilation::bouche_conduit::{q_spec_conv_base_m3_h, DUGD_BBIO_H};
use crate::ventilation::debits_air_cep::{trav_zone_conventionnel, EntreeAir};

/// Th-BCE 2020 §5.6.2 : paramètres du bilan aéraulique de la zone.
pub(super) fn valider_aeraulique_zone(r: &mut RapportValidation, c: &str, zone: &Zone) {
    if zone.usage.est_pris_en_charge() {
        let requis = [
            ("h_zone_m", zone.h_zone_m, "hauteur de la zone"),
            (
                "h_plancher_m",
                zone.h_plancher_m,
                "hauteur du point le plus bas de la zone par rapport au sol extérieur",
            ),
            (
                "q4pa_surf_m3_h_m2",
                zone.q4pa_surf_m3_h_m2,
                "perméabilité à l'air de l'enveloppe sous 4 Pa",
            ),
        ];
        for (champ, valeur, nom) in requis {
            if valeur.is_none() {
                r.erreur(
                    format!("{c}.{champ}"),
                    None,
                    format!("{nom} requise (Th-BCE 2020 §5.6.2)"),
                );
            }
        }
    }
    if let Some(h) = zone.h_zone_m {
        r.strictement_positif(format!("{c}.h_zone_m"), h);
    }
    if let Some(h) = zone.h_plancher_m {
        if !h.is_finite() {
            r.erreur(
                format!("{c}.h_plancher_m"),
                Some(h.to_string()),
                "doit être un nombre fini",
            );
        }
    }
    if let Some(q) = zone.q4pa_surf_m3_h_m2 {
        r.positif(format!("{c}.q4pa_surf_m3_h_m2"), q);
    }
    // Th-BCE 2020 §5.6.3.1.3, tableau 22.
    match (trav_zone_conventionnel(zone.usage.numero()), zone.trav_zone) {
        (Some(_), Some(saisi)) => r.erreur(
            format!("{c}.trav_zone"),
            Some(saisi.to_string()),
            "le caractère traversant de la zone est conventionnel pour cet usage (tableau 22) : \
             ne pas le saisir",
        ),
        (None, None) => r.erreur(
            format!("{c}.trav_zone"),
            None,
            "caractère traversant de la zone au sens des conditions d'hiver requis en logement \
             collectif (Th-BCE 2020 §5.6.3.1.3)",
        ),
        _ => {}
    }
    // Th-BCE 2020 §5.6.3.2.2 : ΔP1 et ΔP2 sont les mêmes pour tous les groupes de la zone.
    let mut pressions: Option<(f64, f64)> = None;
    for (ig, g) in zone.groupes.iter().enumerate() {
        let Some(a) = g.entrees_air.as_ref().and_then(|e| e.autoreglable.as_ref()) else {
            continue;
        };
        let p = (a.delta_p1_pa, a.delta_p2_pa);
        match pressions {
            None => pressions = Some(p),
            Some(reference) if reference != p => r.erreur(
                format!("{c}.groupes[{ig}].entrees_air.autoreglable"),
                None,
                "les pressions ΔP1 et ΔP2 des entrées d'air autoréglables doivent être les \
                 mêmes pour tous les groupes d'une zone (Th-BCE 2020 §5.6.3.2.2)",
            ),
            Some(_) => {}
        }
    }
}

/// Th-BCE 2020 §5.6.2 et §5.6.3.2.1 : entrées d'air d'un groupe.
pub(super) fn valider_entrees_air(r: &mut RapportValidation, c: &str, e: &EntreesAir) {
    r.positif(format!("{c}.m_m3_h"), e.m_m3_h);
    if let Some(v) = e.r {
        r.plage(format!("{c}.r"), v, 0.0, 1.0);
    }
    if let Some(a) = &e.autoreglable {
        r.strictement_positif(format!("{c}.autoreglable.delta_p1_pa"), a.delta_p1_pa);
        r.positif(format!("{c}.autoreglable.m_prime_m3_h"), a.m_prime_m3_h);
        if a.delta_p2_pa.is_nan() || a.delta_p2_pa <= a.delta_p1_pa {
            r.erreur(
                format!("{c}.autoreglable.delta_p2_pa"),
                Some(a.delta_p2_pa.to_string()),
                "ΔP2 doit être supérieure à ΔP1 (Th-BCE 2020 §5.6.3.2.1)",
            );
        } else if !EntreeAir::de_saisie(e).est_monotone() {
            r.erreur(
                format!("{c}.autoreglable.m_prime_m3_h"),
                Some(a.m_prime_m3_h.to_string()),
                "la caractéristique de l'entrée d'air doit être croissante : \
                 M′ × ΔP2^0,5 ≥ M × ΔP1^0,5 (Th-BCE 2020 §5.6.3.2.1)",
            );
        }
    }
}

/// Th-BCE 2020 §6.2.2 : les débits ont pour minimum 0 ; §6.2.3.1.1.2 : le débit de pointe est le
/// débit maximal de la bouche, le débit de base son débit minimal.
pub(super) fn valider_bouche_conduit(r: &mut RapportValidation, c: &str, b: &BoucheConduit) {
    r.positif(
        format!("{c}.q_spec_conv_pointe_m3_h"),
        b.q_spec_conv_pointe_m3_h,
    );
    // §6.2.2 et §6.3.2 : Dugd de 0 à 168 h par semaine ; la formule du débit de base d'un
    // système hygroréglable divise par 168 − Dugd.
    if let Some(d) = b.dugd_h {
        if d.is_finite() && d >= 168.0 {
            r.erreur(
                format!("{c}.dugd_h"),
                Some(d.to_string()),
                "doit être inférieur à 168 h par semaine (Th-BCE 2020 §6.2.3.2.2.2)",
            );
        } else {
            r.positif(format!("{c}.dugd_h"), d);
        }
    }
    if let Some(cdep) = b.cdep {
        r.strictement_positif(format!("{c}.cdep"), cdep);
    }
    if let Some(a) = b.a_cond_m2 {
        r.positif(format!("{c}.a_cond_m2"), a);
    }
    if let (Some(d), Some(base)) = (b.dugd_h, q_spec_conv_base_m3_h(b, b.dugd_h.unwrap_or(0.0))) {
        if b.qvarep_spec_m3_h.is_some() && d.is_finite() && d < 168.0 && base < 0.0 {
            r.erreur(
                format!("{c}.qvarep_spec_m3_h"),
                b.qvarep_spec_m3_h.map(|q| q.to_string()),
                "le débit de base déduit de Qvarepspec avec la durée d'utilisation du grand débit \
                 de la bouche est négatif (Th-BCE 2020 §6.2.3.1.1.2)",
            );
        }
    }
    let (champ, valeur) = match (b.q_spec_conv_base_m3_h, b.qvarep_spec_m3_h) {
        (Some(base), None) => ("q_spec_conv_base_m3_h", base),
        (None, Some(q)) => ("qvarep_spec_m3_h", q),
        (Some(_), Some(_)) => {
            r.erreur(
                format!("{c}.qvarep_spec_m3_h"),
                None,
                "saisir le débit de base ou, pour un système hygroréglable, le débit Qvarepspec \
                 de l'avis technique, mais pas les deux (Th-BCE 2020 §6.2.3.1.1.2)",
            );
            return;
        }
        (None, None) => {
            r.erreur(
                format!("{c}.q_spec_conv_base_m3_h"),
                None,
                "débit de base requis, ou débit Qvarepspec pour un système hygroréglable \
                 (Th-BCE 2020 §6.2.3.1.1.2)",
            );
            return;
        }
    };
    r.positif(format!("{c}.{champ}"), valeur);
    // Le débit de base d'un système hygroréglable se déduit de Qvarepspec avec la durée
    // d'utilisation du grand débit ; celle du Bbio sert ici.
    if let Some(base) = q_spec_conv_base_m3_h(b, DUGD_BBIO_H) {
        if base.is_finite() && b.q_spec_conv_pointe_m3_h.is_finite() {
            if base > b.q_spec_conv_pointe_m3_h {
                r.erreur(
                    format!("{c}.{champ}"),
                    Some(valeur.to_string()),
                    "le débit de pointe doit être au moins égal au débit de base (Th-BCE 2020 \
                     §6.2.3.1.1.2)",
                );
            } else if base < 0.0 {
                r.erreur(
                    format!("{c}.{champ}"),
                    Some(valeur.to_string()),
                    "le débit de base déduit de Qvarepspec est négatif : Qvarepspec est trop \
                     faible devant le débit de pointe (Th-BCE 2020 §6.2.3.1.1.2)",
                );
            }
        }
    }
}

/// Th-BCE 2020 §6.2 à 6.5 : système de ventilation d'une zone d'habitation.
pub(super) fn valider_ventilation(
    r: &mut RapportValidation,
    c: &str,
    v: &Ventilation,
    zone: &Zone,
) {
    let bouches = || zone.groupes.iter().flat_map(|g| &g.bouches_conduits);
    // Une zone sans groupe, ou sans bouche, est déjà signalée par ailleurs.
    let sans_bouche = bouches().next().is_none();
    let a_reprise = sans_bouche || bouches().any(|b| b.isouf == Isouf::Reprise);
    let a_soufflage = bouches().any(|b| b.isouf == Isouf::Soufflage);
    let cs = format!("{c}.systeme");
    match &v.systeme {
        SystemeVentilation::SimpleFluxExtraction(p)
        | SystemeVentilation::MecaniqueBassePression(p) => {
            let nature = if matches!(v.systeme, SystemeVentilation::SimpleFluxExtraction(_)) {
                "simple_flux_extraction"
            } else {
                "mecanique_basse_pression"
            };
            r.positif(format!("{cs}.{nature}.p_vent_base_w"), p.p_vent_base_w);
            r.positif(format!("{cs}.{nature}.p_vent_pointe_w"), p.p_vent_pointe_w);
            if a_soufflage {
                r.erreur(
                    cs.clone(),
                    None,
                    "un système par extraction n'a que des bouches de reprise (isouf = reprise) \
                     (Th-BCE 2020 §6.4 et §6.5)",
                );
            }
            if !a_reprise {
                r.erreur(cs.clone(), None, "au moins une bouche de reprise requise");
            }
        }
        SystemeVentilation::SimpleFluxInsufflation(p) => {
            r.positif(
                format!("{cs}.simple_flux_insufflation.p_vent_base_w"),
                p.p_vent_base_w,
            );
            r.positif(
                format!("{cs}.simple_flux_insufflation.p_vent_pointe_w"),
                p.p_vent_pointe_w,
            );
            if a_reprise && !sans_bouche {
                r.erreur(
                    cs.clone(),
                    None,
                    "un système par insufflation n'a que des bouches de soufflage (isouf = \
                     soufflage) (Th-BCE 2020 §6.5)",
                );
            }
            if !a_soufflage && !sans_bouche {
                r.erreur(cs.clone(), None, "au moins une bouche de soufflage requise");
            }
        }
        SystemeVentilation::DoubleFlux(df) => {
            let cd = format!("{cs}.double_flux");
            r.positif(format!("{cd}.p_vent_base_rep_w"), df.p_vent_base_rep_w);
            r.positif(format!("{cd}.p_vent_pointe_rep_w"), df.p_vent_pointe_rep_w);
            r.positif(format!("{cd}.p_vent_base_sou_w"), df.p_vent_base_sou_w);
            r.positif(format!("{cd}.p_vent_pointe_sou_w"), df.p_vent_pointe_sou_w);
            if !(a_reprise && a_soufflage) && !sans_bouche {
                r.erreur(
                    cs.clone(),
                    None,
                    "un double flux a des bouches de reprise et des bouches de soufflage \
                     (Th-BCE 2020 §6.3)",
                );
            }
            for (champ, present) in [
                ("prechauffage", df.prechauffage),
                ("humidification", df.humidification),
                (
                    "rafraichissement_evaporatif",
                    df.rafraichissement_evaporatif,
                ),
                ("surventilation_mecanique", df.surventilation_mecanique),
            ] {
                if present == Some(true) {
                    r.hors_perimetre(format!("{cd}.{champ}"), Some("true".into()), champ);
                }
            }
            if let Some(e) = &df.echangeur {
                let ce = format!("{cd}.echangeur");
                match &e.representation {
                    RepresentationEchangeur::Simplifiee(s) => r.plage(
                        format!("{ce}.representation.simplifiee.epsilon_saisi"),
                        s.epsilon_saisi,
                        0.0,
                        1.0,
                    ),
                    RepresentationEchangeur::Detaillee(d) => {
                        r.positif(format!("{ce}.representation.detaillee.ua_w_k"), d.ua_w_k);
                    }
                }
                if e.rotatif == Some(true) {
                    r.hors_perimetre(
                        format!("{ce}.rotatif"),
                        Some("true".into()),
                        "échangeur rotatif",
                    );
                }
                if let Some(t) = e.theta_s_ech_rep_lim_c {
                    r.plage(format!("{ce}.theta_s_ech_rep_lim_c"), t, -10.0, 10.0);
                }
                if let Some(b) = &e.bypass {
                    for (champ, t) in [
                        ("theta_ext_bypass_hiver_c", b.theta_ext_bypass_hiver_c),
                        ("theta_ext_bypass_ete_c", b.theta_ext_bypass_ete_c),
                        ("theta_int_bypass_hiver_c", b.theta_int_bypass_hiver_c),
                        ("theta_int_bypass_ete_c", b.theta_int_bypass_ete_c),
                    ] {
                        r.plage(
                            format!("{ce}.bypass.{champ}"),
                            t,
                            f64::NEG_INFINITY,
                            f64::INFINITY,
                        );
                    }
                }
            }
        }
        SystemeVentilation::NaturelleParConduits
        | SystemeVentilation::Hybride
        | SystemeVentilation::Aeration => r.hors_perimetre(
            cs.clone(),
            Some(nom_json(&v.systeme)),
            "ventilation naturelle, hybride ou par ouverture des fenêtres (Th-BCE 2020 §6.6 et \
             §6.7)",
        ),
        SystemeVentilation::CentraleTraitementAir => r.erreur(
            cs.clone(),
            None,
            "centrale de traitement d'air : réservée au non résidentiel (Th-BCE 2020 §6.2.3.1.2 \
             et §6.3.3.1.3)",
        ),
    }
    if let Some(p) = v.puits {
        r.hors_perimetre(
            format!("{c}.puits"),
            Some(nom_json(&p)),
            "puits climatique ou hydraulique",
        );
    }

    // Réseau.
    let cr = format!("{c}.reseau");
    r.plage(format!("{cr}.b"), v.reseau.b, 0.0, 1.0);
    if let Some(x) = v.reseau.ratfuitevc {
        r.plage(format!("{cr}.ratfuitevc"), x, 0.0, 1.0);
    }
    for (champ, requis, valeur) in [
        ("r_rep_m2k_w", a_reprise, v.reseau.r_rep_m2k_w),
        ("r_souf_m2k_w", a_soufflage, v.reseau.r_souf_m2k_w),
    ] {
        match valeur {
            Some(x) => r.positif(format!("{cr}.{champ}"), x),
            None if requis => r.erreur(
                format!("{cr}.{champ}"),
                None,
                "résistance thermique des conduits hors volume chauffé requise (Th-BCE 2020 \
                 §6.5.2 et §6.3.2)",
            ),
            None => {}
        }
    }
    // §6.2.3.4 : surfaces de conduits saisies pour toutes les bouches (Is_surf_cond_def = 0),
    // avec le ratio des fuites en volume chauffé, ou pour aucune.
    let nb = bouches().count();
    let nb_saisies = bouches().filter(|b| b.a_cond_m2.is_some()).count();
    if nb_saisies > 0 && nb_saisies < nb {
        r.erreur(
            format!("{c}.reseau"),
            None,
            "les surfaces de conduits (a_cond_m2) se saisissent pour toutes les bouches de la \
             zone, ou pour aucune (Th-BCE 2020 §6.2.3.4, Is_surf_cond_def)",
        );
    }
    if nb_saisies == nb && nb > 0 && v.reseau.ratfuitevc.is_none() {
        r.erreur(
            format!("{cr}.ratfuitevc"),
            None,
            "ratio des fuites en volume chauffé requis quand les surfaces de conduits sont \
             saisies (Th-BCE 2020 §6.2.3.4)",
        );
    }
}
