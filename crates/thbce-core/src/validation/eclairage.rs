// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation de l'éclairage (Th-BCE 2020 §7.1) : locaux d'un groupe et leur système
//! d'éclairage, hors habitation.

use super::{nom_json, RapportValidation};
use crate::entree::{Groupe, Zone};

/// Th-BCE 2020 §7.1 : locaux d'éclairage d'un groupe. Sans objet en habitation, dont
/// l'éclairage est conventionnel (§7.1.3.6.2) ; requis dans les autres usages calculés.
pub(super) fn valider_locaux_eclairage(
    r: &mut RapportValidation,
    c: &str,
    g: &Groupe,
    zone: &Zone,
) {
    use crate::eclairage::{a_des_peripheriques_de_gestion, est_local_de_bureaux};
    use thbce_data::eclairage::{est_grand_volume, local as local_tableau_78};
    let cl = format!("{c}.locaux");
    if zone.usage.est_habitation() {
        if !g.locaux.is_empty() {
            r.erreur(
                cl,
                None,
                "les locaux d'éclairage ne se saisissent pas en maison individuelle ou accolée ni \
                 en logement collectif : l'éclairage y est conventionnel (Th-BCE 2020 §7.1.3.6.2)",
            );
        }
        return;
    }
    if !zone.usage.est_pris_en_charge() {
        return;
    }
    if g.locaux.is_empty() {
        r.erreur(
            cl,
            None,
            "au moins un local d'éclairage requis : type de local, part de la surface du groupe, \
             accès à la lumière naturelle et système d'éclairage (Th-BCE 2020 §7.1.2)",
        );
        return;
    }
    r.ids_uniques(&cl, g.locaux.iter().map(|l| &l.id));
    let somme: f64 = g.locaux.iter().map(|l| l.ratio_local).sum();
    if (somme - 1.0).abs() > 1e-6 {
        r.erreur(
            cl.clone(),
            Some(somme.to_string()),
            "la somme des « Ratiolocal » des locaux du groupe vaut 1 (Th-BCE 2020 §7.1.2)",
        );
    }
    let usage = zone.usage.numero();
    for (i, l) in g.locaux.iter().enumerate() {
        let c = format!("{cl}[{i}]");
        r.plage(format!("{c}.ratio_local"), l.ratio_local, 0.0, 1.0);
        r.plage(format!("{c}.ratio_ecl_nat"), l.ratio_ecl_nat, 0.0, 1.0);
        if local_tableau_78(usage, l.type_local).is_none() {
            r.erreur(
                format!("{c}.type_local"),
                Some(nom_json(&l.type_local)),
                format!(
                    "ce type de local n'existe pas pour l'usage n° {usage} (Th-BCE 2020 \
                     §7.1.3.6.1, tableau 78)"
                ),
            );
        } else if est_grand_volume(usage, l.type_local) {
            r.hors_perimetre(
                format!("{c}.type_local"),
                Some(nom_json(&l.type_local)),
                "éclairage d'un local de grand volume (Th-BCE 2020 §7.1.3.3 et §7.1.3.4)",
            );
        }
        let e = &l.eclairage;
        let ce = format!("{c}.eclairage");
        r.positif(format!("{ce}.p_ecl_tot_w_m2"), e.p_ecl_tot_w_m2);
        match (
            a_des_peripheriques_de_gestion(e.gest_ecl, e.grad_ecl),
            e.p_ecl_aux_w_m2,
        ) {
            (true, Some(p)) => r.positif(format!("{ce}.p_ecl_aux_w_m2"), p),
            (true, None) => r.erreur(
                format!("{ce}.p_ecl_aux_w_m2"),
                None,
                "puissance des appareillages et périphériques de gestion requise avec une \
                 détection de présence ou une gestion automatique selon la lumière naturelle \
                 (Th-BCE 2020 §7.1.3.6)",
            ),
            (false, Some(p)) if p != 0.0 => r.erreur(
                format!("{ce}.p_ecl_aux_w_m2"),
                Some(p.to_string()),
                "sans détection de présence ni gestion automatique selon la lumière naturelle, \
                 la puissance des périphériques de gestion est nulle par convention : ne pas la \
                 saisir (Th-BCE 2020 §7.1.3.6)",
            ),
            (false, _) => {}
        }
        let mobilier = est_local_de_bureaux(usage, l.type_local)
            && e.p_ecl_tot_w_m2 < crate::eclairage::P_ECL_IMMO_SEUIL_BUREAUX_W_M2;
        for (champ, valeur) in [
            (
                "eff_ecl_immo_projet_w_m2_100lux",
                e.eff_ecl_immo_projet_w_m2_100lux,
            ),
            ("ei_proj_lux", e.ei_proj_lux),
        ] {
            match (mobilier, valeur) {
                (true, Some(v)) => r.strictement_positif(format!("{ce}.{champ}"), v),
                (true, None) => r.erreur(
                    format!("{ce}.{champ}"),
                    None,
                    "requis pour un local de bureaux dont la puissance d'éclairage immobilier \
                     est inférieure à 10 W/m² : l'éclairage mobilier complète l'éclairement \
                     requis (Th-BCE 2020 §7.1.3.2.3)",
                ),
                (false, Some(v)) => r.erreur(
                    format!("{ce}.{champ}"),
                    Some(v.to_string()),
                    "ne se saisit que pour un local de bureaux d'une zone de bureaux dont la \
                     puissance d'éclairage immobilier est inférieure à 10 W/m² (Th-BCE 2020 \
                     §7.1.3.2.3)",
                ),
                (false, None) => {}
            }
        }
    }
}
