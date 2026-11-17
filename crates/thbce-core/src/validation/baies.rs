// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des baies, de leurs types et de leurs facteurs solaires.

use super::enveloppe::valider_exposition;
use super::{RapportValidation, References};
use crate::bati::baie::{u_baie, EtatBaie, REDUCTION_SW_SP_C};
use crate::entree::{Baie, BaieType, FacteursBaie, GestionPm, TypePm};

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
        && refs
            .types_baies
            .get(b.type_baie.as_str())
            .is_some_and(|t| t.ouvrant.is_some())
    {
        r.erreur(
            format!("{c}.expo_br"),
            None,
            "classe d'exposition au bruit requise pour une baie ouvrable",
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
        // Décision Q30, point 7 : pas de calcul approché pour ce que la version ne couvre pas.
        if pm.type_pm == TypePm::StoreVenitien {
            r.hors_perimetre(
                format!("{c}.protection_mobile.type_pm"),
                Some("store_venitien".to_owned()),
                "store vénitien",
            );
        }
        if pm.gestion == GestionPm::AutomatiqueAvecDerogation {
            r.hors_perimetre(
                format!("{c}.protection_mobile.gestion"),
                Some("automatique_avec_derogation".to_owned()),
                "gestion automatique des protections mobiles",
            );
        }
        // Th-BCE 2020 §5.10.2 : facteurs et coefficient U de la baie avec la protection en place.
        if b.avec_protection.is_none() {
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
