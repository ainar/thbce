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
    use crate::entree::AffectationEclairage as A;
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
        valider_affectations_baies(r, c, g, false);
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
    let a_un_grand_volume = g
        .locaux
        .iter()
        .any(|l| est_grand_volume(usage, l.type_local));
    valider_affectations_baies(r, c, g, a_un_grand_volume);
    // §7.1.3.3 : avec des éclairants de toiture uniformément répartis (baies de type 1), le local
    // de grand volume est éclairé de façon homogène et `Ratioecl_nat,l` vaut 1.
    let toiture_uniforme = g
        .baies
        .iter()
        .any(|b| b.affectation_eclairage == Some(A::GrandVolumeToitureUniforme));
    for (i, l) in g.locaux.iter().enumerate() {
        let c = format!("{cl}[{i}]");
        r.plage(format!("{c}.ratio_local"), l.ratio_local, 0.0, 1.0);
        r.plage(format!("{c}.ratio_ecl_nat"), l.ratio_ecl_nat, 0.0, 1.0);
        if local_tableau_78(usage, l.type_local).is_none() {
            // Usage 7 : le classeur des scénarios a un local de service (15 % de la surface),
            // le tableau 78 n'en a pas (lecture Q70).
            let precision = if usage == 7
                && l.type_local == thbce_data::eclairage::TypeLocal::LocalService
            {
                " ; les locaux de service d'un bâtiment universitaire se décrivent avec l'un des \
                 types de local de l'usage 7 (docs/questions/Q070.md)"
            } else {
                ""
            };
            r.erreur(
                format!("{c}.type_local"),
                Some(nom_json(&l.type_local)),
                format!(
                    "ce type de local n'existe pas pour l'usage n° {usage} (Th-BCE 2020 \
                     §7.1.3.6.1, tableau 78){precision}"
                ),
            );
        }
        if toiture_uniforme
            && est_grand_volume(usage, l.type_local)
            && (l.ratio_ecl_nat - 1.0).abs() > 1e-9
        {
            r.erreur(
                format!("{c}.ratio_ecl_nat"),
                Some(l.ratio_ecl_nat.to_string()),
                "le groupe a des éclairants de toiture uniformément répartis \
                 (affectation_eclairage : grand_volume_toiture_uniforme) : le local de grand volume \
                 est éclairé de façon homogène et sa part ayant accès à la lumière naturelle vaut 1 \
                 (Th-BCE 2020 §7.1.3.3)",
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

/// Th-BCE 2020 §5.10.3.2 : affectation des baies d'un groupe à un grand volume. Une baie de grand
/// volume exige un local de grand volume dans le groupe ; un éclairant de toiture uniformément
/// réparti est une baie horizontale (inclinaison nulle).
fn valider_affectations_baies(
    r: &mut RapportValidation,
    c: &str,
    g: &Groupe,
    a_un_grand_volume: bool,
) {
    use crate::entree::AffectationEclairage as A;
    for (i, b) in g.baies.iter().enumerate() {
        let champ = format!("{c}.baies[{i}].affectation_eclairage");
        match b.affectation_eclairage {
            None | Some(A::VolumeNormal) => {}
            Some(a) if !a_un_grand_volume => r.erreur(
                champ,
                Some(nom_json(&a)),
                "baie affectée à un grand volume dans un groupe qui n'a pas de local de grand \
                 volume (salle de sport des établissements sportifs, aire de production des \
                 industries, aire de vente de plus de 300 m² des commerces ; Th-BCE 2020 \
                 §7.1.3.3)",
            ),
            Some(A::GrandVolumeToitureUniforme) if b.exposition.inclinaison_deg != 0.0 => r.erreur(
                champ,
                Some(nom_json(&A::GrandVolumeToitureUniforme)),
                "un éclairant de toiture uniformément réparti est une baie horizontale \
                 (inclinaison nulle ; Th-BCE 2020 §5.10.3.2)",
            ),
            Some(_) => {}
        }
    }
}
