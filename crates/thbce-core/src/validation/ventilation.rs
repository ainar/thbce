// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation de la ventilation (Th-BCE 2020 §6) : systèmes, entrées d'air, bouches et
//! conduits, cohérence aéraulique de la zone.

use super::{nom_json, RapportValidation};
use crate::entree::{
    BatteriePrechauffage, BatteriePrerefroidissement, BoucheConduit, Echangeur, EntreesAir,
    HumidificateurHiver, Isouf, RafraichissementEvaporatif, RepresentationEchangeur,
    SystemeVentilation, TypeCta, TypeHumidificationHiver, TypeRafraichissementEvaporatif,
    Ventilation, Zone,
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
/// Th-BCE 2020 §6.2.3.1.1.1 et §6.2.3.2.2.1 : bouche et conduit d'un groupe d'un usage autre
/// que la maison individuelle ou accolée et le logement collectif.
pub(super) fn valider_bouche_conduit_non_residentielle(
    r: &mut RapportValidation,
    c: &str,
    b: &BoucheConduit,
) {
    let regle = "ne se saisit qu'en maison individuelle ou accolée et en logement collectif ; \
                 dans les autres usages, saisir les débits en occupation et en inoccupation \
                 (Th-BCE 2020 §6.2.3.1.1.1)";
    if b.q_spec_conv_pointe_m3_h != 0.0 {
        r.erreur(
            format!("{c}.q_spec_conv_pointe_m3_h"),
            Some(b.q_spec_conv_pointe_m3_h.to_string()),
            regle,
        );
    }
    for (champ, valeur) in [
        ("q_spec_conv_base_m3_h", b.q_spec_conv_base_m3_h),
        ("qvarep_spec_m3_h", b.qvarep_spec_m3_h),
        ("dugd_h", b.dugd_h),
    ] {
        if let Some(v) = valeur {
            r.erreur(format!("{c}.{champ}"), Some(v.to_string()), regle);
        }
    }
    match b.q_spec_occ_m3_h {
        Some(q) => r.strictement_positif(format!("{c}.q_spec_occ_m3_h"), q),
        None => r.erreur(
            format!("{c}.q_spec_occ_m3_h"),
            None,
            "débit en occupation requis (Th-BCE 2020 §6.2.3.1.1.1, éq. 383 et 384)",
        ),
    }
    match b.q_spec_inocc_m3_h {
        Some(q) => r.positif(format!("{c}.q_spec_inocc_m3_h"), q),
        None => r.erreur(
            format!("{c}.q_spec_inocc_m3_h"),
            None,
            "débit en inoccupation requis, 0 si la ventilation est arrêtée (Th-BCE 2020 \
             §6.2.3.1.1.1, éq. 385 et 386)",
        ),
    }
    if let Some(k) = b.crdbnr {
        if !(k > 0.0 && k <= 1.0) {
            r.erreur(
                format!("{c}.crdbnr"),
                Some(k.to_string()),
                "coefficient de réduction des débits attendu entre 0 (exclu) et 1 (Th-BCE 2020 \
                 §6.2.3.2.1, tableau 57)",
            );
        }
    }
    if let Some(cdep) = b.cdep {
        r.strictement_positif(format!("{c}.cdep"), cdep);
    }
    if let Some(a) = b.a_cond_m2 {
        r.positif(format!("{c}.a_cond_m2"), a);
    }
}

pub(super) fn valider_bouche_conduit(r: &mut RapportValidation, c: &str, b: &BoucheConduit) {
    for (champ, valeur) in [
        ("q_spec_occ_m3_h", b.q_spec_occ_m3_h),
        ("q_spec_inocc_m3_h", b.q_spec_inocc_m3_h),
        ("q_spec_ch_m3_h", b.q_spec_ch_m3_h),
        ("crdbnr", b.crdbnr),
    ] {
        if let Some(v) = valeur {
            r.erreur(
                format!("{c}.{champ}"),
                Some(v.to_string()),
                "ne se saisit que dans les usages autres que la maison individuelle ou accolée \
                 et le logement collectif (Th-BCE 2020 §6.2.3.1.1.1 et §6.2.3.2.1)",
            );
        }
    }

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

/// Th-BCE 2020 §6.2 à 6.5 : système de ventilation d'une zone.
pub(super) fn valider_ventilation(
    r: &mut RapportValidation,
    c: &str,
    v: &Ventilation,
    zone: &Zone,
) {
    let bouches = || zone.groupes.iter().flat_map(|g| &g.bouches_conduits);
    // Hors habitation, les bouches ne servent pas au Bbio (débits conventionnels du tableau
    // 56-1) : chaque groupe n'en a pas forcément, mais le système de la zone en a au moins une.
    if !zone.usage.est_habitation() && bouches().next().is_none() && !zone.groupes.is_empty() {
        r.erreur(
            c.to_owned(),
            None,
            "au moins une bouche de ventilation requise dans un groupe de la zone : ses débits \
             en occupation et en inoccupation sont ceux du système (Th-BCE 2020 §6.2.3.1.1.1)",
        );
    }
    // Une zone sans groupe, ou sans bouche, est déjà signalée par ailleurs.
    let sans_bouche = bouches().next().is_none();
    let a_reprise = sans_bouche || bouches().any(|b| b.isouf == Isouf::Reprise);
    let a_soufflage = bouches().any(|b| b.isouf == Isouf::Soufflage);
    let cs = format!("{c}.systeme");
    let habitation = zone.usage.est_habitation();
    // §6.4.3 et §6.5.3.3 : puissances en débit de base et de pointe en habitation (éq. 641 et
    // 643), en occupation et en inoccupation dans les autres usages (éq. 636 à 639).
    let puissances =
        |r: &mut RapportValidation, nature: &str, p: &crate::entree::VentilateurSimpleFlux| {
            let (requis, exclus, regle_requis, regle_exclus) = if habitation {
                (
                [
                    ("p_vent_base_w", p.p_vent_base_w),
                    ("p_vent_pointe_w", p.p_vent_pointe_w),
                ],
                [
                    ("p_vent_occ_w", p.p_vent_occ_w),
                    ("p_vent_inocc_w", p.p_vent_inocc_w),
                ],
                "puissance requise en maison individuelle ou accolée et en logement collectif \
                 (Th-BCE 2020 §6.5.3.3, éq. 641 et 643)",
                "ne se saisit que dans les usages autres que la maison individuelle ou accolée \
                 et le logement collectif (Th-BCE 2020 §6.5.3.3, éq. 636 à 639)",
            )
            } else {
                (
                    [
                        ("p_vent_occ_w", p.p_vent_occ_w),
                        ("p_vent_inocc_w", p.p_vent_inocc_w),
                    ],
                    [
                        ("p_vent_base_w", p.p_vent_base_w),
                        ("p_vent_pointe_w", p.p_vent_pointe_w),
                    ],
                    "puissance requise dans les usages autres que la maison individuelle ou \
                 accolée et le logement collectif (Th-BCE 2020 §6.5.3.3, éq. 636 à 639)",
                    "ne se saisit qu'en maison individuelle ou accolée et en logement collectif \
                 (Th-BCE 2020 §6.5.3.3, éq. 641 et 643)",
                )
            };
            for (champ, valeur) in requis {
                match valeur {
                    Some(v) => r.positif(format!("{cs}.{nature}.{champ}"), v),
                    None => r.erreur(format!("{cs}.{nature}.{champ}"), None, regle_requis),
                }
            }
            for (champ, valeur) in exclus {
                if let Some(v) = valeur {
                    r.erreur(
                        format!("{cs}.{nature}.{champ}"),
                        Some(v.to_string()),
                        regle_exclus,
                    );
                }
            }
        };
    // §6.2.3.1.2 : débit du mode actif des bouches, propre aux centrales de traitement d'air.
    let est_cta = matches!(v.systeme, SystemeVentilation::Cta(_));
    for (ig, g) in zone.groupes.iter().enumerate() {
        for (ib, b) in g.bouches_conduits.iter().enumerate() {
            let cb = format!("{}.groupes[{ig}].bouches_conduits[{ib}]", zone_chemin(c));
            match (est_cta && !habitation, b.q_spec_ch_m3_h) {
                (true, None) => r.erreur(
                    format!("{cb}.q_spec_ch_m3_h"),
                    None,
                    "débit du mode actif requis pour une bouche reliée à une centrale de \
                     traitement d'air (Th-BCE 2020 §6.2.3.1.2, éq. 391 et 394)",
                ),
                (true, Some(q)) => {
                    r.strictement_positif(format!("{cb}.q_spec_ch_m3_h"), q);
                    for (champ, zn) in [
                        ("q_spec_occ_m3_h", b.q_spec_occ_m3_h),
                        ("q_spec_inocc_m3_h", b.q_spec_inocc_m3_h),
                    ] {
                        if zn.is_some_and(|zn| zn > q) {
                            r.erreur(
                                format!("{cb}.{champ}"),
                                zn.map(|x| x.to_string()),
                                "le débit de zone neutre ne peut dépasser le débit du mode actif \
                                 (Th-BCE 2020 §6.2.3.1.2, taux d'air neuf minimal des éq. 393 \
                                 et 397 au plus égal à 1)",
                            );
                        }
                    }
                }
                (false, Some(q)) if !habitation => r.erreur(
                    format!("{cb}.q_spec_ch_m3_h"),
                    Some(q.to_string()),
                    "ne se saisit que pour une bouche reliée à une centrale de traitement d'air \
                     (Th-BCE 2020 §6.2.3.1.2)",
                ),
                _ => {}
            }
        }
    }
    match &v.systeme {
        SystemeVentilation::MecaniqueBassePression(_) if !habitation => r.erreur(
            cs.clone(),
            Some(nom_json(&v.systeme)),
            "l'extraction mécanique basse pression est propre aux logements collectifs à \
             conduits individuels ou de type shunt (Th-BCE 2020 §6.4)",
        ),
        SystemeVentilation::SimpleFluxExtraction(p)
        | SystemeVentilation::MecaniqueBassePression(p) => {
            let nature = if matches!(v.systeme, SystemeVentilation::SimpleFluxExtraction(_)) {
                "simple_flux_extraction"
            } else {
                "mecanique_basse_pression"
            };
            puissances(r, nature, p);
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
            puissances(r, "simple_flux_insufflation", p);
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
            // Éq. (390) à (394) en habitation ; (458) et (459) dans les autres usages.
            let hab = [
                ("p_vent_base_rep_w", df.p_vent_base_rep_w),
                ("p_vent_pointe_rep_w", df.p_vent_pointe_rep_w),
                ("p_vent_base_sou_w", df.p_vent_base_sou_w),
                ("p_vent_pointe_sou_w", df.p_vent_pointe_sou_w),
            ];
            let nr = [
                ("p_vent_rep_occ_w", df.p_vent_rep_occ_w),
                ("p_vent_sou_occ_w", df.p_vent_sou_occ_w),
                ("p_vent_rep_inocc_w", df.p_vent_rep_inocc_w),
                ("p_vent_sou_inocc_w", df.p_vent_sou_inocc_w),
            ];
            let (requis, exclus, regle_requis, regle_exclus) = if habitation {
                (
                    hab,
                    nr,
                    "puissance requise en maison individuelle ou accolée et en logement \
                     collectif (Th-BCE 2020 §6.3.3.3.1, éq. 391 à 394)",
                    "ne se saisit que dans les usages autres que la maison individuelle ou \
                     accolée et le logement collectif (Th-BCE 2020 §6.3.3.4.4, éq. 458 et 459)",
                )
            } else {
                (
                    nr,
                    hab,
                    "puissance requise dans les usages autres que la maison individuelle ou \
                     accolée et le logement collectif (Th-BCE 2020 §6.3.3.4.4, éq. 458 et 459)",
                    "ne se saisit qu'en maison individuelle ou accolée et en logement collectif \
                     (Th-BCE 2020 §6.3.3.3.1, éq. 391 à 394)",
                )
            };
            for (champ, valeur) in requis {
                match valeur {
                    Some(x) => r.positif(format!("{cd}.{champ}"), x),
                    None => r.erreur(format!("{cd}.{champ}"), None, regle_requis),
                }
            }
            for (champ, valeur) in exclus {
                if let Some(x) = valeur {
                    r.erreur(format!("{cd}.{champ}"), Some(x.to_string()), regle_exclus);
                }
            }
            if !(a_reprise && a_soufflage) && !sans_bouche {
                r.erreur(
                    cs.clone(),
                    None,
                    "un double flux a des bouches de reprise et des bouches de soufflage \
                     (Th-BCE 2020 §6.3)",
                );
            }
            if df.prechauffage == Some(true) && df.batterie_prechauffage.is_none() {
                r.erreur(
                    format!("{cd}.prechauffage"),
                    Some("true".into()),
                    "décrire la batterie de préchauffage par `batterie_prechauffage` (Th-BCE \
                     2020 §6.3.3.4.9.1)",
                );
            }
            if df.surventilation_mecanique == Some(true) && df.rafraichissement_surventilation.is_none()
            {
                r.erreur(
                    format!("{cd}.surventilation_mecanique"),
                    Some("true".into()),
                    "décrire le rafraîchissement par `rafraichissement_surventilation` (Th-BCE \
                     2020 §6.12.2, tableau 75)",
                );
            }
            valider_humidification(
                r,
                &cd,
                habitation,
                None,
                &Humidifications {
                    hiver: (df.humidification, df.humidificateur.as_ref()),
                    ete: (
                        df.rafraichissement_evaporatif,
                        df.rafraichissement_adiabatique.as_ref(),
                    ),
                    echangeur: df.echangeur.is_some(),
                    prerefroidissement: df.batterie_prerefroidissement.is_some(),
                },
            );
            valider_batteries(
                r,
                &cd,
                zone,
                df.batterie_prechauffage.as_ref(),
                df.batterie_prerefroidissement.as_ref(),
            );
            if let Some(e) = &df.echangeur {
                valider_echangeur(r, &format!("{cd}.echangeur"), e);
            }
        }
        SystemeVentilation::Cta(cta) if !habitation => {
            let cc = format!("{cs}.cta");
            for (champ, p) in [
                ("p_vent_rep_ch_w", cta.p_vent_rep_ch_w),
                ("p_vent_sou_ch_w", cta.p_vent_sou_ch_w),
                ("p_vent_rep_zn_occ_w", cta.p_vent_rep_zn_occ_w),
                ("p_vent_sou_zn_occ_w", cta.p_vent_sou_zn_occ_w),
                ("p_vent_rep_zn_inocc_w", cta.p_vent_rep_zn_inocc_w),
                ("p_vent_sou_zn_inocc_w", cta.p_vent_sou_zn_inocc_w),
            ] {
                r.positif(format!("{cc}.{champ}"), p);
            }
            r.plage(
                format!("{cc}.theta_enc_c"),
                cta.theta_enc_c,
                f64::NEG_INFINITY,
                f64::INFINITY,
            );
            if cta.theta_enf_c.is_nan() || cta.theta_enf_c <= cta.theta_enc_c {
                r.erreur(
                    format!("{cc}.theta_enf_c"),
                    Some(cta.theta_enf_c.to_string()),
                    "la borne haute de la plage de fonctionnement en tout air neuf doit dépasser \
                     la borne basse (Th-BCE 2020 §6.3.3.4.3.3, éq. 436)",
                );
            }
            if !(a_reprise && a_soufflage) && !sans_bouche {
                r.erreur(
                    cs.clone(),
                    None,
                    "une centrale de traitement d'air a des bouches de reprise et des bouches de \
                     soufflage (Th-BCE 2020 §6.3)",
                );
            }
            // §6.3.3.4.9 et §6.3.3.4.10 : prétraitement des types 1 et 2 seulement.
            if cta.type_cta != TypeCta::DebitConstant {
                for (champ, present) in [
                    ("batterie_prechauffage", cta.batterie_prechauffage.is_some()),
                    (
                        "batterie_prerefroidissement",
                        cta.batterie_prerefroidissement.is_some(),
                    ),
                ] {
                    if present {
                        r.erreur(
                            format!("{cc}.{champ}"),
                            None,
                            "le prétraitement de l'air ne s'applique qu'aux doubles flux et aux \
                             centrales à débit constant (Th-BCE 2020 §6.3.3.4.9 et 6.3.3.4.10)",
                        );
                    }
                }
            }
            valider_mode_actif(r, &cc, zone, cta);
            if cta.surventilation_mecanique == Some(true)
                && cta.rafraichissement_surventilation.is_none()
            {
                r.erreur(
                    format!("{cc}.surventilation_mecanique"),
                    Some("true".into()),
                    "décrire le rafraîchissement par `rafraichissement_surventilation` (Th-BCE \
                     2020 §6.12.2, tableau 75)",
                );
            }
            valider_humidification(
                r,
                &cc,
                false,
                Some(cta.type_cta),
                &Humidifications {
                    hiver: (cta.humidification, cta.humidificateur.as_ref()),
                    ete: (
                        cta.rafraichissement_evaporatif,
                        cta.rafraichissement_adiabatique.as_ref(),
                    ),
                    echangeur: cta.echangeur.is_some(),
                    prerefroidissement: cta.batterie_prerefroidissement.is_some(),
                },
            );
            valider_batteries(
                r,
                &cc,
                zone,
                cta.batterie_prechauffage.as_ref(),
                cta.batterie_prerefroidissement.as_ref(),
            );
            if let Some(e) = &cta.echangeur {
                valider_echangeur(r, &format!("{cc}.echangeur"), e);
            }
        }
        SystemeVentilation::NaturelleParConduits | SystemeVentilation::Hybride => r.hors_perimetre(
            cs.clone(),
            Some(nom_json(&v.systeme)),
            "ventilation naturelle par conduits ou hybride (Th-BCE 2020 §6.6)",
        ),
        SystemeVentilation::Aeration => valider_aeration(r, c, v, zone),
        SystemeVentilation::CentraleTraitementAir if !habitation => r.erreur(
            cs.clone(),
            Some(nom_json(&v.systeme)),
            "décrire la centrale de traitement d'air par la variante `cta` (Th-BCE 2020 §6.3.2, \
             tableau 61)",
        ),
        SystemeVentilation::CentraleTraitementAir | SystemeVentilation::Cta(_) => r.erreur(
            cs.clone(),
            None,
            "centrale de traitement d'air : réservée au non résidentiel (Th-BCE 2020 §6.2.3.1.2 \
             et §6.3.3.1.2)",
        ),
    }
    valider_surventilation(r, c, v, zone);
    if !matches!(v.systeme, SystemeVentilation::Aeration) {
        for (ig, g) in zone.groupes.iter().enumerate() {
            if let Some(id) = &g.aeration_espace_tampon {
                r.erreur(
                    format!("{}.groupes[{ig}].aeration_espace_tampon", zone_chemin(c)),
                    Some(id.clone()),
                    "ne se saisit que pour un système d'aération (Th-BCE 2020 §6.7.3.1)",
                );
            }
        }
    }
    if let Some(p) = v.puits {
        r.hors_perimetre(
            format!("{c}.puits"),
            Some(nom_json(&p)),
            "puits climatique ou hydraulique",
        );
    }

    // Réseau ; sans conduit en aération (§6.7).
    let aeration = matches!(v.systeme, SystemeVentilation::Aeration);
    let cr = format!("{c}.reseau");
    r.plage(format!("{cr}.b"), v.reseau.b, 0.0, 1.0);
    if let Some(x) = v.reseau.ratfuitevc {
        r.plage(format!("{cr}.ratfuitevc"), x, 0.0, 1.0);
    }
    for (champ, requis, valeur) in [
        ("r_rep_m2k_w", a_reprise && !aeration, v.reseau.r_rep_m2k_w),
        (
            "r_souf_m2k_w",
            a_soufflage && !aeration,
            v.reseau.r_souf_m2k_w,
        ),
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

/// Th-BCE 2020 §6.3.3.5 : batteries chaude et froide régulées sur la charge d'une centrale à
/// charge variable (mode actif, lecture `docs/questions/Q095.md`).
fn valider_mode_actif(r: &mut RapportValidation, cc: &str, zone: &Zone, cta: &crate::entree::Cta) {
    let chaude = cta.batterie_chaude == Some(true);
    let froide = cta.batterie_froide == Some(true);
    let type_4 = cta.type_cta == TypeCta::DebitVariable;
    if (chaude || froide) && cta.type_cta == TypeCta::DebitConstant {
        r.erreur(
            format!("{cc}.type_cta"),
            Some("debit_constant".into()),
            "le mode actif (batteries régulées sur la charge) n'est autorisé que pour les \
             centrales à conditions variables en fonction de la charge, types 3 et 4 (Th-BCE \
             2020 §6.3.3.5)",
        );
    }
    let emetteur = |froid: bool| {
        zone.groupes.iter().flat_map(|g| &g.emetteurs).any(|e| {
            if froid {
                e.refroidissement
                    .as_ref()
                    .is_some_and(|f| f.centrale == Some(true))
            } else {
                e.chauffage
                    .as_ref()
                    .is_some_and(|c| c.centrale == Some(true))
            }
        })
    };
    // Champs propres à chaque batterie : (présente, champ, valeur, requis).
    let champs = [
        (
            chaude,
            "theta_sou_nom_chaud_c",
            cta.theta_sou_nom_chaud_c,
            type_4,
        ),
        (
            froide,
            "theta_sou_nom_froid_c",
            cta.theta_sou_nom_froid_c,
            type_4,
        ),
        (froide, "theta_dim_fr_c", cta.theta_dim_fr_c, true),
    ];
    for (presente, champ, valeur, requis) in champs {
        let c = format!("{cc}.{champ}");
        match (valeur, presente && requis) {
            (None, true) => r.erreur(
                c,
                None,
                "requis pour une batterie régulée sur la charge (Th-BCE 2020 §6.3.3.5.8, éq. \
                 585, 591 et 604 ; températures nominales en type 4 seulement)",
            ),
            (Some(x), false) => r.erreur(
                c,
                Some(x.to_string()),
                "sans objet : pas de batterie régulée sur la charge correspondante, ou centrale \
                 à débit constant (type 3, éq. 583 et 584)",
            ),
            (Some(x), true) => r.plage(c, x, -50.0, 100.0),
            (None, false) => {}
        }
    }
    for (presente, froid, champ) in [
        (chaude, false, "batterie_chaude"),
        (froide, true, "batterie_froide"),
    ] {
        if presente && !emetteur(froid) {
            r.erreur(
                format!("{cc}.{champ}"),
                Some("true".into()),
                "la batterie régulée sur la charge assure la demande des émetteurs contenus dans \
                 les bouches de soufflage (Th-BCE 2020 §6.3.2, tableau 61, « Qsys_ch^g,s ») : \
                 au moins un émetteur d'un groupe de la zone doit le déclarer (`centrale`)",
            );
        }
    }
}

/// Th-BCE 2020 §6.3.2, tableau 61 : émetteur contenu dans les bouches de soufflage de la
/// centrale de la zone (`centrale`, lecture `docs/questions/Q095.md`). `c` est le chemin de
/// l'émetteur, `g` son groupe et `zone` la zone du groupe.
pub fn valider_emetteur_centrale(
    r: &mut RapportValidation,
    c: &str,
    e: &crate::entree::Emetteur,
    g: &crate::entree::Groupe,
    zone: &Zone,
) {
    let cta = match zone.ventilation.as_ref().map(|v| &v.systeme) {
        Some(SystemeVentilation::Cta(cta)) => Some(cta),
        _ => None,
    };
    for (present, fonction, batterie) in [
        (
            e.chauffage
                .as_ref()
                .is_some_and(|x| x.centrale == Some(true)),
            "chauffage",
            cta.is_some_and(|x| x.batterie_chaude == Some(true)),
        ),
        (
            e.refroidissement
                .as_ref()
                .is_some_and(|x| x.centrale == Some(true)),
            "refroidissement",
            cta.is_some_and(|x| x.batterie_froide == Some(true)),
        ),
    ] {
        if !present {
            continue;
        }
        let cf = format!("{c}.{fonction}.centrale");
        if !batterie {
            r.erreur(
                cf.clone(),
                Some("true".into()),
                "un émetteur contenu dans les bouches de soufflage suppose une centrale de \
                 traitement d'air de la zone pourvue de la batterie régulée sur la charge de \
                 même fonction (Th-BCE 2020 §6.3.3.5)",
            );
        }
        if e.ventilateurs.is_some() {
            r.erreur(
                format!("{c}.ventilateurs"),
                None,
                "un émetteur contenu dans les bouches de soufflage d'une centrale n'a pas de \
                 ventilateurs locaux (Th-BCE 2020 §6.3.3.5 et §8.1.3.8)",
            );
        }
        if !g
            .bouches_conduits
            .iter()
            .any(|b| b.isouf == Isouf::Soufflage)
        {
            r.erreur(
                cf,
                Some("true".into()),
                "le groupe n'a pas de bouche de soufflage de la centrale (Th-BCE 2020 §6.3.2)",
            );
        }
    }
}

/// Chemin de la zone à partir de celui de son système de ventilation.
fn zone_chemin(c: &str) -> &str {
    c.strip_suffix(".ventilation").unwrap_or(c)
}

/// Th-BCE 2020 §6.3.2, tableau 61, et §6.3.3.4.6 : échangeur d'une centrale double flux.
fn valider_echangeur(r: &mut RapportValidation, ce: &str, e: &Echangeur) {
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
    match (e.rotatif == Some(true), e.p_ech_w) {
        (true, Some(p)) => r.positif(format!("{ce}.p_ech_w"), p),
        (true, None) => r.erreur(
            format!("{ce}.p_ech_w"),
            None,
            "puissance de l'échangeur rotatif requise (Th-BCE 2020 §6.3.3.4.6.3, éq. 497)",
        ),
        (false, Some(p)) => r.erreur(
            format!("{ce}.p_ech_w"),
            Some(p.to_string()),
            "ne se saisit que pour un échangeur rotatif (« Pech », échangeur à roue)",
        ),
        (false, None) => {}
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

/// Th-BCE 2020 §6.3.3.2.2 et §6.3.3.2.4 : batteries de préchauffage et de prérefroidissement.
/// Humidificateur d'hiver et rafraîchissement évaporatif saisis d'une centrale.
struct Humidifications<'a> {
    hiver: (Option<bool>, Option<&'a HumidificateurHiver>),
    ete: (Option<bool>, Option<&'a RafraichissementEvaporatif>),
    echangeur: bool,
    prerefroidissement: bool,
}

/// Th-BCE 2020 §6.3.2 (tableau 61), §6.3.3.4.2, §6.3.3.4.9.2, §6.3.3.4.10 et §6.3.3.4.11 :
/// humidificateur d'hiver (types 1 et 2) et rafraîchissement évaporatif, calculés hors
/// habitation (lot 1-ven3, Q96).
fn valider_humidification(
    r: &mut RapportValidation,
    c: &str,
    habitation: bool,
    type_cta: Option<TypeCta>,
    h: &Humidifications<'_>,
) {
    let (is_hr, hum) = h.hiver;
    let (is_ete, ete) = h.ete;
    for (champ, bool_saisi, decrit, nom) in [
        ("humidification", is_hr, hum.is_some(), "humidificateur"),
        (
            "rafraichissement_evaporatif",
            is_ete,
            ete.is_some(),
            "rafraichissement_adiabatique",
        ),
    ] {
        match (bool_saisi, decrit) {
            // En habitation, le booléen seul reste un refus ciblé (code 3), qu'il soit décrit
            // ou non (lot 1-ven4, reprise de relecture : pas de code 2 puis code 3).
            (Some(true), false) if habitation => r.hors_perimetre(
                format!("{c}.{champ}"),
                Some("true".into()),
                "humidification et rafraîchissement évaporatif d'un double flux en \
                 habitation (Th-BCE 2020 §6.3.3.4.9.2 et 6.3.3.4.11)",
            ),
            (Some(true), false) => r.erreur(
                format!("{c}.{champ}"),
                Some("true".into()),
                format!("décrire l'équipement par `{nom}` (Th-BCE 2020 §6.3.2, tableau 61)"),
            ),
            (Some(false), true) => r.erreur(
                format!("{c}.{champ}"),
                Some("false".into()),
                format!("incohérent avec la description `{nom}`"),
            ),
            _ => {}
        }
    }
    if habitation {
        for (champ, present) in [
            ("humidificateur", hum.is_some()),
            ("rafraichissement_adiabatique", ete.is_some()),
        ] {
            if present {
                r.hors_perimetre(
                    format!("{c}.{champ}"),
                    None,
                    "humidification et rafraîchissement évaporatif d'un double flux en \
                     habitation (Th-BCE 2020 §6.3.3.4.9.2 et 6.3.3.4.11)",
                );
            }
        }
        return;
    }
    if let Some(x) = hum {
        let ch = format!("{c}.humidificateur");
        // §6.3.3.4.9 : prétraitement des types 1 et 2 seulement.
        if matches!(
            type_cta,
            Some(TypeCta::DebitConstantTemperatureVariable | TypeCta::DebitVariable)
        ) {
            r.erreur(
                ch.clone(),
                None,
                "l'humidification d'hiver ne s'applique qu'aux doubles flux et aux centrales à \
                 débit constant (Th-BCE 2020 §6.3.3.4.9)",
            );
        }
        r.plage(
            format!("{ch}.omega_cons_kg_kg"),
            x.omega_cons_kg_kg,
            0.0,
            1.0,
        );
        if let Some(e) = x.eta_hum {
            r.plage(format!("{ch}.eta_hum"), e, 0.0, 1.0);
        }
        match (x.type_humidification, x.generation.is_some(), x.eta_hum) {
            (TypeHumidificationHiver::Vapeur, false, _) => r.erreur(
                format!("{ch}.generation"),
                None,
                "génération requise : l'énergie d'un humidificateur à vapeur (éq. 529) est \
                 demandée à une génération par un réseau de CTA (Th-BCE 2020 §8.11.3, éq. 939)",
            ),
            (TypeHumidificationHiver::Vapeur, _, Some(e)) => r.erreur(
                format!("{ch}.eta_hum"),
                Some(e.to_string()),
                "sans objet pour un humidificateur à vapeur (Th-BCE 2020 éq. 526 à 529)",
            ),
            (TypeHumidificationHiver::Pulverisation, true, _) => r.erreur(
                format!("{ch}.generation"),
                None,
                "sans objet : un humidificateur à pulvérisation ne demande pas d'énergie \
                 (Th-BCE 2020 éq. 533)",
            ),
            _ => {}
        }
    }
    if let Some(x) = ete {
        let ce = format!("{c}.rafraichissement_adiabatique");
        if let Some(e) = x.eta_hum {
            r.plage(format!("{ce}.eta_hum"), e, 0.0, 1.0);
        }
        // §6.3.3.4.2 : valeur minimale de θi_base.
        r.plage(
            format!("{ce}.theta_i_base_c"),
            x.theta_i_base_c,
            crate::ventilation::humidification::THETA_I_BASE_MIN_C,
            f64::INFINITY,
        );
        let variable = type_cta == Some(TypeCta::DebitVariable);
        for (champ, d) in [
            ("delta_theta_i1_k", x.delta_theta_i1_k),
            ("delta_theta_i2_k", x.delta_theta_i2_k),
        ] {
            match (d, variable) {
                (None, true) => r.erreur(
                    format!("{ce}.{champ}"),
                    None,
                    "requis pour une centrale à débit variable (profil à hystérésis, Th-BCE \
                     2020 éq. 426)",
                ),
                (Some(v), true) => {
                    if v.is_nan() || v <= 0.0 {
                        r.erreur(
                            format!("{ce}.{champ}"),
                            Some(v.to_string()),
                            "strictement positif (Th-BCE 2020 éq. 426, figure 73)",
                        );
                    }
                }
                (Some(v), false) => r.erreur(
                    format!("{ce}.{champ}"),
                    Some(v.to_string()),
                    "sans objet hors centrale à débit variable (Th-BCE 2020 §6.3.3.1.4.2)",
                ),
                (None, false) => {}
            }
        }
        match x.type_rafraichissement {
            // §6.3.3.4.10 : direct incompatible avec la batterie de prérefroidissement.
            TypeRafraichissementEvaporatif::Direct if h.prerefroidissement => r.erreur(
                ce.clone(),
                None,
                "le rafraîchissement évaporatif direct est incompatible avec une batterie de \
                 prérefroidissement (Th-BCE 2020 §6.3.3.4.10)",
            ),
            // §6.3.3.4.5.3 : l'air extrait refroidi ne sert que par l'échangeur.
            TypeRafraichissementEvaporatif::Indirect if !h.echangeur => r.erreur(
                ce.clone(),
                None,
                "le rafraîchissement évaporatif indirect refroidit l'air neuf par l'échangeur \
                 air extrait / air neuf : échangeur requis (Th-BCE 2020 §6.3.3.4.5.3)",
            ),
            _ => {}
        }
    }
}

fn valider_batteries(
    r: &mut RapportValidation,
    c: &str,
    zone: &Zone,
    pre: Option<&BatteriePrechauffage>,
    fr: Option<&BatteriePrerefroidissement>,
) {
    let fini = |r: &mut RapportValidation, chemin: String, t: f64| {
        r.plage(chemin, t, f64::NEG_INFINITY, f64::INFINITY);
    };
    if let Some(p) = pre {
        fini(
            r,
            format!("{c}.batterie_prechauffage.theta_cons_c"),
            p.theta_cons_c,
        );
        fini(
            r,
            format!("{c}.batterie_prechauffage.theta_ex_c"),
            p.theta_ex_c,
        );
    }
    if let Some(f) = fr {
        let cf = format!("{c}.batterie_prerefroidissement");
        fini(r, format!("{cf}.theta_cons_c"), f.theta_cons_c);
        fini(r, format!("{cf}.theta_ex_c"), f.theta_ex_c);
        fini(r, format!("{cf}.theta_dim_fr_c"), f.theta_dim_fr_c);
        // Éq. (389).
        if !zone.groupes.iter().any(|g| g.iclim == Some(true)) {
            r.erreur(
                cf.clone(),
                None,
                "une centrale ne peut contenir de batterie de prérefroidissement que si elle est \
                 reliée à au moins un groupe climatisé (Th-BCE 2020 §6.3.3.2.4, éq. 389)",
            );
        }
        // Éq. (387).
        if let Some(p) = pre {
            if p.theta_ex_c >= f.theta_ex_c {
                r.erreur(
                    format!("{c}.batterie_prechauffage.theta_ex_c"),
                    Some(p.theta_ex_c.to_string()),
                    "la température extérieure d'autorisation du préchauffage doit être \
                     inférieure à celle du prérefroidissement (Th-BCE 2020 §6.3.3.2.2, éq. 387)",
                );
            }
        }
    }
}

/// Th-BCE 2020 §6.12 : rafraîchissement par surventilation mécanique (tableau 75, éq. 767).
fn valider_surventilation(r: &mut RapportValidation, c: &str, v: &Ventilation, zone: &Zone) {
    use crate::ventilation::surventilation_mecanique::periode_incoherente;
    let surv = v.systeme.surventilation();
    if let Some(s) = surv {
        let variante = match &v.systeme {
            SystemeVentilation::SimpleFluxExtraction(_) => "simple_flux_extraction",
            SystemeVentilation::SimpleFluxInsufflation(_) => "simple_flux_insufflation",
            SystemeVentilation::MecaniqueBassePression(_) => "mecanique_basse_pression",
            SystemeVentilation::DoubleFlux(_) => "double_flux",
            _ => "cta",
        };
        let cs = format!("{c}.systeme.{variante}.rafraichissement_surventilation");
        r.positif(format!("{cs}.p_vent_rafnoc_rep_w"), s.p_vent_rafnoc_rep_w);
        r.positif(format!("{cs}.p_vent_rafnoc_souf_w"), s.p_vent_rafnoc_souf_w);
        let nocturne = s.type_surventilation.est_nocturne();
        let annee = s.type_surventilation.toute_l_annee();
        if annee && s.mi_saison.is_none() {
            r.erreur(
                format!("{cs}.mi_saison"),
                None,
                "consignes hors saison de refroidissement requises pour un fonctionnement toute \
                 l'année (Th-BCE 2020 §6.12.3.2, éq. 765)",
            );
        }
        if !annee && s.mi_saison.is_some() {
            r.avertissement(
                format!("{cs}.mi_saison"),
                None,
                "sans objet : le rafraîchissement ne fonctionne qu'en saison de refroidissement \
                 (Th-BCE 2020 §6.12.3.2, éq. 762 et 763)",
            );
        }
        for (saison, consignes) in [("ete", Some(&s.ete)), ("mi_saison", s.mi_saison.as_ref())] {
            let Some(k) = consignes else { continue };
            let ck = format!("{cs}.{saison}");
            for (champ, x) in [
                ("theta_sd_dec_c", k.theta_sd_dec_c),
                ("theta_sd_arr_c", k.theta_sd_arr_c),
                ("delta_theta_lim_k", k.delta_theta_lim_k),
                ("theta_ext_c", k.theta_ext_c),
            ] {
                r.plage(format!("{ck}.{champ}"), x, -20.0, 50.0);
            }
            match (nocturne, k.h_leg_deb_h, k.h_leg_fin_h) {
                (true, Some(deb), Some(fin)) => {
                    if deb > 23 || fin > 23 {
                        r.erreur(
                            format!("{ck}.h_leg_deb_h"),
                            Some(format!("{deb} ; {fin}")),
                            "heures légales de 0 à 23 (Th-BCE 2020 §6.12.2, tableau 75)",
                        );
                    } else if let Some(m) = periode_incoherente(deb, fin) {
                        r.erreur(
                            format!("{ck}.h_leg_deb_h"),
                            Some(format!("{deb} ; {fin}")),
                            format!("{m} (Th-BCE 2020 §6.12.3.3, éq. 767)"),
                        );
                    }
                }
                (true, _, _) => r.erreur(
                    format!("{ck}.h_leg_deb_h"),
                    None,
                    "heures légales de début et de fin de la période nocturne requises pour un \
                     rafraîchissement nocturne (Th-BCE 2020 §6.12.3.3, éq. 768 et 769)",
                ),
                (false, None, None) => {}
                (false, _, _) => r.avertissement(
                    format!("{ck}.h_leg_deb_h"),
                    None,
                    "sans objet en freecooling (Th-BCE 2020 §6.12.3.3, note de l'éq. 770)",
                ),
            }
        }
    }
    for (ig, g) in zone.groupes.iter().enumerate() {
        for (ib, b) in g.bouches_conduits.iter().enumerate() {
            let chemin = format!(
                "{}.groupes[{ig}].bouches_conduits[{ib}].q_spec_rafnoc_m3_h",
                zone_chemin(c)
            );
            match (surv.is_some(), b.q_spec_rafnoc_m3_h) {
                (true, Some(q)) => r.positif(chemin, q),
                (true, None) => r.erreur(
                    chemin,
                    None,
                    "débit de rafraîchissement par surventilation mécanique requis pour chaque \
                     bouche du système (Th-BCE 2020 §6.12.3.1, éq. 758 et 759)",
                ),
                (false, Some(_)) => r.avertissement(
                    chemin,
                    None,
                    "sans objet : le système n'a pas de rafraîchissement par surventilation \
                     mécanique (Th-BCE 2020 §6.12)",
                ),
                (false, None) => {}
            }
        }
    }
}

/// Th-BCE 2020 §6.7 : aération par ouverture des fenêtres.
///
/// Les débits d'air neuf d'un groupe sont ceux de ses bouches de soufflage (éq. 162) ; le débit
/// repris en est déduit (éq. 163). Les paramètres propres aux systèmes mécaniques sont fixés par
/// le texte : `Cdep = Cfenb` (§6.2.3.3.1, note 1), `Crdbnr = 1` (§6.2.3.2.1), `Kres = 0`
/// (tableau 59, « cas de l'aération et du BBio »). Les saisir est une erreur ; les paramètres
/// du réseau, sans objet, sont signalés. Lecture : `docs/questions/Q098.md`.
fn valider_aeration(r: &mut RapportValidation, c: &str, v: &Ventilation, zone: &Zone) {
    let cs = format!("{c}.systeme");
    let a_soufflage = zone
        .groupes
        .iter()
        .flat_map(|g| &g.bouches_conduits)
        .any(|b| b.isouf == Isouf::Soufflage);
    // Une zone sans aucune bouche est déjà refusée en code 2 par la règle commune à tous les
    // systèmes : hors habitation par `valider_ventilation` (§6.2.3.1.1.1), en habitation pour
    // chaque groupe par `valider_groupe` (§6.2.3.1.1.2). Le message propre à l'aération ne
    // s'ajoute qu'en présence de bouches, toutes de reprise (Q98, complément du 19/11/2026).
    let sans_bouche = zone.groupes.iter().all(|g| g.bouches_conduits.is_empty());
    if !a_soufflage && !sans_bouche {
        r.erreur(
            cs,
            None,
            "au moins une bouche de soufflage requise : en aération, le débit d'air neuf d'un \
             groupe est la somme de ses débits spécifiques soufflés (Th-BCE 2020 §5.6.3.4.5, \
             éq. 162)",
        );
    }
    let reseau = &v.reseau;
    if reseau.classe_etancheite.is_some()
        || reseau.ratfuitevc.is_some()
        || reseau.r_rep_m2k_w.is_some()
        || reseau.r_souf_m2k_w.is_some()
    {
        r.avertissement(
            format!("{c}.reseau"),
            None,
            "sans objet en aération : pas de conduit, fuites nulles (Th-BCE 2020 §6.2.3.4, \
             tableau 59, « cas de l'aération et du BBio »)",
        );
    }
    for (ig, g) in zone.groupes.iter().enumerate() {
        let cg = format!("{}.groupes[{ig}]", zone_chemin(c));
        for (ib, b) in g.bouches_conduits.iter().enumerate() {
            let cb = format!("{cg}.bouches_conduits[{ib}]");
            if b.isouf == Isouf::Reprise {
                r.avertissement(
                    format!("{cb}.isouf"),
                    Some("reprise".to_owned()),
                    "en aération, le débit repris d'un groupe est l'opposé de son débit \
                     soufflé (Th-BCE 2020 §5.6.3.4.5, éq. 163) : cette bouche de reprise n'entre \
                     pas dans le calcul des consommations ni du DH (le calcul du Bbio la lit \
                     comme toute bouche, §6.1 et décision Q36) ; l'air neuf fourni par les \
                     fenêtres se décrit par les bouches de soufflage",
                );
            }
            for (champ, valeur, regle) in [
                (
                    "cdep",
                    b.cdep,
                    "en aération, Cdep = Cfenb quelle que soit l'occupation (Th-BCE 2020 \
                     §6.2.3.3.1, note 1)",
                ),
                (
                    "crdbnr",
                    b.crdbnr,
                    "en aération, Crdbnr = 1 quelle que soit l'occupation (Th-BCE 2020 \
                     §6.2.3.2.1, tableau 57)",
                ),
                (
                    "a_cond_m2",
                    b.a_cond_m2,
                    "sans objet en aération : pas de conduit (Th-BCE 2020 §6.2.3.4, tableau 59)",
                ),
            ] {
                if let Some(x) = valeur {
                    r.erreur(format!("{cb}.{champ}"), Some(x.to_string()), regle);
                }
            }
        }
    }
}
