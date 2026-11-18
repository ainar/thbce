// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des baies, de leurs types et de leurs facteurs solaires.

use super::enveloppe::valider_exposition;
use super::{RapportValidation, References};
use crate::bati::baie::{u_baie, EtatBaie, REDUCTION_SW_SP_C};
use crate::entree::{
    Automatisme, Baie, BaieType, FacteursBaie, FacteursLames, GestionPm, GestionSv,
    ProtectionMobile, TypeHorloge, TypePm,
};

pub(super) fn valider_baie(r: &mut RapportValidation, c: &str, b: &Baie, refs: &References<'_>) {
    let beta = b.exposition.inclinaison_deg;
    match refs.types_baies.get(b.type_baie.as_str()) {
        None => r.erreur(
            format!("{c}.type_baie"),
            Some(b.type_baie.clone()),
            "type de baie absent du catalogue",
        ),
        // Th-BCE 2020 §5.10.3.5, éq. (207) : le coefficient U sans protection mobile doit être
        // connu à l'inclinaison de la baie.
        Some(t) => {
            // Un type sans aucun coefficient est déjà signalé dans le catalogue.
            let un_u_saisi = t.u_sp_vert_w_m2k.is_some() || t.u_sp_horiz_w_m2k.is_some();
            if un_u_saisi
                && (0.0..=90.0).contains(&beta)
                && u_baie(t.u_sp_vert_w_m2k, t.u_sp_horiz_w_m2k, beta).is_none()
            {
                r.erreur(
                    format!("{c}.type_baie"),
                    Some(b.type_baie.clone()),
                    format!(
                        "le type de baie ne donne pas le coefficient U nécessaire à une \
                         inclinaison de {beta}° : u_sp_horiz_w_m2k en dessous de 60°, \
                         u_sp_vert_w_m2k à partir de 30° (les deux entre 30 et 60°)"
                    ),
                );
            }
            // §5.10.3.6.2 : la baie qui accueille un store vénitien est verticale.
            if t.protection_mobile
                .as_ref()
                .is_some_and(|pm| pm.type_pm == TypePm::StoreVenitien)
                && (beta - 90.0).abs() > f64::EPSILON
            {
                r.erreur(
                    format!("{c}.exposition.inclinaison_deg"),
                    Some(beta.to_string()),
                    "une baie équipée d'un store vénitien est verticale (90°, §5.10.3.6.2)",
                );
            }
            // Éq. (208) : de même pour le coefficient avec la protection mobile en place.
            let un_u_ap_saisi = t.u_ap_vert_w_m2k.is_some() || t.u_ap_horiz_w_m2k.is_some();
            if t.protection_mobile.is_some()
                && un_u_ap_saisi
                && (0.0..=90.0).contains(&beta)
                && u_baie(t.u_ap_vert_w_m2k, t.u_ap_horiz_w_m2k, beta).is_none()
            {
                r.erreur(
                    format!("{c}.type_baie"),
                    Some(b.type_baie.clone()),
                    format!(
                        "le type de baie ne donne pas le coefficient U avec protection mobile \
                         nécessaire à une inclinaison de {beta}° : u_ap_horiz_w_m2k en dessous \
                         de 60°, u_ap_vert_w_m2k à partir de 30°"
                    ),
                );
            }
        }
    }
    // Th-BCE 2020 §5.13.2 : classe d'exposition au bruit d'une baie ouvrable.
    if b.expo_br.is_none()
        && b.bruit.is_none()
        && refs
            .types_baies
            .get(b.type_baie.as_str())
            .is_some_and(|t| t.ouvrant.is_some())
    {
        r.erreur(
            format!("{c}.expo_br"),
            None,
            "classe d'exposition au bruit requise pour une baie ouvrable (expo_br, ou bruit pour la \
             déduire)",
        );
    }
    // Th-BCE 2020 §5.10.2 : inclinaison d'une baie entre 0° (horizontale) et 90° (verticale).
    if beta > 90.0 {
        r.erreur(
            format!("{c}.exposition.inclinaison_deg"),
            Some(beta.to_string()),
            "l'inclinaison d'une baie est comprise entre 0 et 90°",
        );
    }
    r.positif(format!("{c}.a_b_m2"), b.a_b_m2);
    valider_exposition(r, &format!("{c}.exposition"), &b.exposition, refs);
}

/// Th-BCE 2020 §5.10.2.
pub(super) fn valider_baie_type(r: &mut RapportValidation, c: &str, b: &BaieType) {
    if b.u_sp_vert_w_m2k.is_none() && b.u_sp_horiz_w_m2k.is_none() {
        r.erreur(
            c.to_owned(),
            None,
            "au moins un des coefficients u_sp_vert_w_m2k et u_sp_horiz_w_m2k est requis",
        );
    }
    for (champ, u) in [
        ("u_sp_vert_w_m2k", b.u_sp_vert_w_m2k),
        ("u_sp_horiz_w_m2k", b.u_sp_horiz_w_m2k),
        ("u_ap_vert_w_m2k", b.u_ap_vert_w_m2k),
        ("u_ap_horiz_w_m2k", b.u_ap_horiz_w_m2k),
    ] {
        if let Some(u) = u {
            r.positif(format!("{c}.{champ}"), u);
        }
    }
    if let Some(o) = &b.ouvrant {
        // Th-BCE 2020 §5.13.3.2.3 : Rouvmax saisi, ou déduit du type d'ouvrant (tableau 44).
        match (o.r_ouv_max, o.type_ouvrant) {
            (Some(v), None) => r.plage(format!("{c}.ouvrant.r_ouv_max"), v, 0.0, 1.0),
            (None, Some(_)) => {}
            _ => r.erreur(
                format!("{c}.ouvrant"),
                None,
                "renseigner soit r_ouv_max, soit type_ouvrant",
            ),
        }
    }
    if let Some(pm) = &b.protection_mobile {
        valider_protection_mobile(r, c, b, pm);
        // Th-BCE 2020 §5.10.2 : facteurs et coefficient U de la baie avec la protection en place.
        if b.avec_protection.is_none() && pm.type_pm != TypePm::StoreVenitien {
            r.erreur(
                format!("{c}.avec_protection"),
                None,
                "requis quand une protection mobile est déclarée",
            );
        }
        if b.u_ap_vert_w_m2k.is_none() && b.u_ap_horiz_w_m2k.is_none() {
            r.erreur(
                c.to_owned(),
                None,
                "au moins un des coefficients u_ap_vert_w_m2k et u_ap_horiz_w_m2k est requis \
                 quand une protection mobile est déclarée",
            );
        }
    }
    valider_facteurs_baie(
        r,
        &format!("{c}.sans_protection"),
        &b.sans_protection,
        EtatBaie::SansProtection,
    );
    if let Some(f) = &b.avec_protection {
        valider_facteurs_baie(
            r,
            &format!("{c}.avec_protection"),
            f,
            EtatBaie::AvecProtection,
        );
    }
}

fn valider_facteurs_baie(r: &mut RapportValidation, c: &str, f: &FacteursBaie, etat: EtatBaie) {
    let composantes_c = [("sw1_c", f.sw1_c), ("sw2_c", f.sw2_c), ("sw3_c", f.sw3_c)];
    let saisies = composantes_c.iter().filter(|(_, v)| v.is_some()).count();
    if saisies != 0 && saisies != composantes_c.len() {
        r.erreur(
            c.to_owned(),
            None,
            "sw1_c, sw2_c et sw3_c se saisissent ensemble ou pas du tout (absents, la valeur par \
             défaut du §5.10.3.6.3 s'applique au facteur solaire global)",
        );
    }
    // Th-BCE 2020 §5.10.3.6.3, éq. (215) : la valeur par défaut `Swsp-c = Swsp-e − 0,1` n'a de
    // sens que si elle reste positive.
    if saisies == 0
        && etat == EtatBaie::SansProtection
        && f.sw1_e + f.sw2_e + f.sw3_e <= REDUCTION_SW_SP_C
    {
        r.erreur(
            c.to_owned(),
            None,
            "la valeur par défaut Swsp-c = Swsp-e − 0,1 (éq. 215) n'est pas positive : saisir \
             sw1_c, sw2_c et sw3_c",
        );
    }
    for (champ, v) in composantes_c.into_iter().chain([("tlid", f.tlid)]) {
        if let Some(v) = v {
            r.plage(format!("{c}.{champ}"), v, 0.0, 1.0);
        }
    }
    for (champ, v) in [
        ("sw1_e", f.sw1_e),
        ("sw2_e", f.sw2_e),
        ("sw3_e", f.sw3_e),
        ("tli", f.tli),
    ] {
        r.plage(format!("{c}.{champ}"), v, 0.0, 1.0);
    }
}

fn valider_protection_mobile(
    r: &mut RapportValidation,
    c: &str,
    b: &BaieType,
    pm: &ProtectionMobile,
) {
    let cp = format!("{c}.protection_mobile");
    let venitien = pm.type_pm == TypePm::StoreVenitien;
    let automatique = if venitien {
        if pm.gestion.is_some() {
            r.erreur(
                format!("{cp}.gestion"),
                None,
                "sans objet pour un store vénitien : utiliser gestion_sv",
            );
        }
        match pm.gestion_sv {
            None => {
                r.erreur(
                    format!("{cp}.gestion_sv"),
                    None,
                    "requis pour un store vénitien (Type-GPM-sv)",
                );
                false
            }
            Some(g) => g.est_automatique(),
        }
    } else {
        if pm.gestion_sv.is_some() {
            r.erreur(
                format!("{cp}.gestion_sv"),
                None,
                "réservé aux stores vénitiens : utiliser gestion",
            );
        }
        match pm.gestion {
            None => {
                r.erreur(
                    format!("{cp}.gestion"),
                    None,
                    "requis pour un volet ou un store enroulable (Type-GPM)",
                );
                false
            }
            Some(g) => g == GestionPm::AutomatiqueAvecDerogation,
        }
    };
    if automatique {
        // Th-BCE 2020 §5.9.2 : « Detec_pres » et paramètres de l'automatisme (pas de valeur
        // conventionnelle pour les seuils : question Q100).
        if !venitien && pm.detecteur_presence.is_none() {
            r.erreur(
                format!("{cp}.detecteur_presence"),
                None,
                "requis en gestion automatique d'un volet ou d'un store enroulable (Detec_pres)",
            );
        }
        match &pm.automatisme {
            None => r.erreur(
                format!("{cp}.automatisme"),
                None,
                "requis en gestion automatique",
            ),
            Some(a) => valider_automatisme(r, &format!("{cp}.automatisme"), a, pm, venitien),
        }
    } else {
        if pm.automatisme.is_some() {
            r.erreur(
                format!("{cp}.automatisme"),
                None,
                "sans objet en gestion manuelle",
            );
        }
        if pm.detecteur_presence.is_some() && !venitien {
            r.avertissement(
                format!("{cp}.detecteur_presence"),
                None,
                "sans effet en gestion manuelle",
            );
        }
    }
    if venitien {
        // Th-BCE 2020 §5.10.3.6.2 : facteurs du store vénitien, saisis dans `store_venitien`.
        if b.avec_protection.is_some() {
            r.erreur(
                format!("{c}.avec_protection"),
                None,
                "non utilisé pour un store vénitien : saisir store_venitien",
            );
        }
        match &b.store_venitien {
            None => r.erreur(
                format!("{c}.store_venitien"),
                None,
                "requis pour un store vénitien",
            ),
            Some(l) => {
                let suivi = pm.gestion_sv.is_some_and(GestionSv::suit_le_soleil);
                let n = if suivi { 6 } else { 1 };
                let cl = format!("{c}.store_venitien");
                for (champ, v, attendu) in [
                    ("th_bc.direct", &l.th_bc.direct, 6),
                    ("th_bc.diffus", &l.th_bc.diffus, n),
                    ("th_bc.reflechi", &l.th_bc.reflechi, n),
                ] {
                    if v.len() != attendu {
                        r.erreur(
                            format!("{cl}.{champ}"),
                            Some(v.len().to_string()),
                            format!(
                                "{attendu} élément(s) attendu(s) (hauteurs de soleil de 0 à 75° \
                                 par pas de 15°, ou un seul en position fixe pour le diffus et \
                                 le réfléchi)"
                            ),
                        );
                    }
                    for (k, f) in v.iter().enumerate() {
                        valider_facteurs_lames(r, &format!("{cl}.{champ}[{k}]"), f);
                    }
                }
                for (champ, f) in [
                    ("th_d.direct", &l.th_d.direct),
                    ("th_d.diffus", &l.th_d.diffus),
                    ("th_d.reflechi", &l.th_d.reflechi),
                ] {
                    valider_facteurs_lames(r, &format!("{cl}.{champ}"), f);
                }
            }
        }
    } else if b.store_venitien.is_some() {
        r.erreur(
            format!("{c}.store_venitien"),
            None,
            "réservé aux types de baie dont la protection est un store vénitien",
        );
    }
}

fn valider_facteurs_lames(r: &mut RapportValidation, c: &str, f: &FacteursLames) {
    for (champ, v) in [
        ("sw1", f.sw1),
        ("sw2", f.sw2),
        ("sw3", f.sw3),
        ("tl_global", f.tl_global),
    ] {
        r.plage(format!("{c}.{champ}"), v, 0.0, 1.0);
    }
    if let Some(v) = f.tlid {
        r.plage(format!("{c}.tlid"), v, 0.0, 1.0);
    }
}

fn valider_automatisme(
    r: &mut RapportValidation,
    c: &str,
    a: &Automatisme,
    pm: &ProtectionMobile,
    venitien: bool,
) {
    r.strictement_positif(
        format!("{c}.eclairement_limite_lux"),
        a.eclairement_limite_lux,
    );
    if venitien {
        for (champ, v) in [
            ("temperature_limite_haute_c", a.temperature_limite_haute_c),
            ("temperature_limite_basse_c", a.temperature_limite_basse_c),
        ] {
            if v.is_some() {
                r.erreur(
                    format!("{c}.{champ}"),
                    None,
                    "sans objet pour un store vénitien",
                );
            }
        }
        if a.matrice.is_some() {
            r.erreur(
                format!("{c}.matrice"),
                None,
                "utiliser matrice_sv pour un store vénitien",
            );
        }
    } else {
        match (a.temperature_limite_haute_c, a.temperature_limite_basse_c) {
            (Some(h), Some(bas)) => {
                r.plage(format!("{c}.temperature_limite_haute_c"), h, -50.0, 100.0);
                r.plage(format!("{c}.temperature_limite_basse_c"), bas, -50.0, 100.0);
                if bas > h {
                    r.erreur(
                        format!("{c}.temperature_limite_basse_c"),
                        Some(bas.to_string()),
                        "la limite basse ne dépasse pas la limite haute (figure 31)",
                    );
                }
            }
            _ => r.erreur(
                c.to_owned(),
                None,
                "temperature_limite_haute_c et temperature_limite_basse_c sont requises",
            ),
        }
        if a.matrice_sv.is_some() {
            r.erreur(
                format!("{c}.matrice_sv"),
                None,
                "réservé aux stores vénitiens",
            );
        }
        if let Some(m) = &a.matrice {
            for (k, l) in m.occupation.iter().chain(&m.inoccupation).enumerate() {
                for (n, h) in [&l.jour_sous_seuil, &l.jour_au_dessus, &l.nuit]
                    .into_iter()
                    .enumerate()
                {
                    for (cote, v) in [("haute", h.haute), ("basse", h.basse)] {
                        r.plage(format!("{c}.matrice[{k}][{n}].{cote}"), v, 0.0, 1.0);
                    }
                }
            }
        }
    }
    if let Some(m) = &a.matrice_sv {
        for (k, l) in m.occupation.iter().chain(&m.inoccupation).enumerate() {
            for (n, v) in [l.jour_sous_seuil, l.jour_au_dessus, l.nuit]
                .into_iter()
                .enumerate()
            {
                r.plage(format!("{c}.matrice_sv[{k}][{n}]"), v, 0.0, 1.0);
            }
        }
    }
    // Vitesse limite du vent : protection extérieure autre qu'un volet (éq. 186, 190, 194).
    let exige_vent = pm.exterieure && pm.type_pm != TypePm::Volet;
    match a.vent_limite_m_s {
        Some(v) => r.strictement_positif(format!("{c}.vent_limite_m_s"), v),
        None if exige_vent => r.erreur(
            format!("{c}.vent_limite_m_s"),
            None,
            "requis pour une protection extérieure (store enroulable ou vénitien)",
        ),
        None => {}
    }
    match (a.horloge, a.horaires_jour) {
        (TypeHorloge::Personnalisable, None) => r.erreur(
            format!("{c}.horaires_jour"),
            None,
            "requis pour l'horloge personnalisable",
        ),
        (TypeHorloge::Personnalisable, Some(h)) => {
            for (k, x) in h.iter().enumerate() {
                if !(1..=24).contains(&x.debut_h)
                    || !(1..=24).contains(&x.fin_h)
                    || x.debut_h >= x.fin_h
                {
                    r.erreur(
                        format!("{c}.horaires_jour[{k}]"),
                        None,
                        "heures légales de 1 à 24, début strictement avant la fin",
                    );
                }
            }
        }
        (_, Some(_)) => r.avertissement(
            format!("{c}.horaires_jour"),
            None,
            "sans effet sans horloge personnalisable",
        ),
        _ => {}
    }
}
