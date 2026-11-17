// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des données de la fixation des exigences (annexe de l'article R. 172-4 du CCH).

use thbce_data::ZoneClimatique;

use super::RapportValidation;

/// Annexe de l'article R. 172-4 du CCH, chapitre V, partie II (Q54, point 14) : catégorie de
/// contraintes extérieures déclarée pour un bâtiment d'habitation.
fn valider_categorie_contraintes(
    r: &mut RapportValidation,
    c: &str,
    bat: &crate::entree::Batiment,
    e: &crate::entree::ParametresExigences,
    loc: &crate::entree::Localisation,
) {
    use crate::entree::{CategorieContraintes, ZoneBruit};
    let cc = format!("{c}.categorie_contraintes_exterieures");
    let cb = c.strip_suffix(".exigences").unwrap_or(c);
    match e.categorie_contraintes_exterieures {
        CategorieContraintes::Categorie1 => {}
        CategorieContraintes::Categorie3 => r.erreur(
            cc,
            Some("categorie_3".into()),
            "la catégorie 3 est réservée aux locaux climatisés de bureaux, hôtels, restaurants, \
             commerces et autres usages non résidentiels (annexe de l'article R. 172-4, chapitre \
             V, partie II) : impossible pour un bâtiment d'habitation",
        ),
        CategorieContraintes::Categorie2 => {
            // Un local de catégorie 2 est, cumulativement, climatisé, exposé au bruit BR2 ou BR3,
            // dans un bâtiment construit en zone H2d ou H3 à moins de 400 m d'altitude ; une
            // zone est de catégorie 2 si tous ses locaux autres qu'à occupation passagère le
            // sont : chaque groupe d'habitation doit donc être climatisé.
            let regle = "(annexe de l'article R. 172-4, chapitre V, partie II : un local de \
                         catégorie 2 est climatisé, ses baies sont exposées au bruit BR2 ou BR3, \
                         et le bâtiment est construit en zone H2d ou H3 à moins de 400 m)";
            for (iz, z) in bat.zones.iter().enumerate() {
                for (ig, g) in z.groupes.iter().enumerate() {
                    if z.usage.est_habitation() && g.iclim != Some(true) {
                        r.erreur(
                            cc.clone(),
                            Some("categorie_2".into()),
                            format!(
                                "catégorie 2 impossible : le groupe {cb}.zones[{iz}].groupes[{ig}] \
                                 ({}) n'est pas climatisé {regle}",
                                g.id
                            ),
                        );
                    }
                }
            }
            let zone = loc.zone_climatique_retenue();
            if !matches!(zone, ZoneClimatique::H2d | ZoneClimatique::H3) {
                r.erreur(
                    cc.clone(),
                    Some("categorie_2".into()),
                    format!("catégorie 2 impossible en zone climatique {zone} {regle}"),
                );
            }
            if loc.alt_m >= 400.0 {
                r.erreur(
                    cc.clone(),
                    Some("categorie_2".into()),
                    format!(
                        "catégorie 2 impossible à {} m d'altitude (400 m ou plus) {regle}",
                        loc.alt_m
                    ),
                );
            }
            if e.zone_bruit == ZoneBruit::Br1 {
                r.erreur(
                    cc,
                    Some("categorie_2".into()),
                    format!("catégorie 2 impossible en zone de bruit BR1 {regle}"),
                );
            }
        }
    }
}

/// Données de la fixation des exigences (annexe de l'article R. 172-4 du CCH).
pub(super) fn valider_exigences(
    r: &mut RapportValidation,
    cb: &str,
    bat: &crate::entree::Batiment,
    loc: &crate::entree::Localisation,
) {
    let Some(e) = &bat.exigences else {
        return;
    };
    let c = format!("{cb}.exigences");
    valider_categorie_contraintes(r, &c, bat, e, loc);
    if e.annee_depot_permis < 2022 {
        r.erreur(
            format!("{c}.annee_depot_permis"),
            Some(e.annee_depot_permis.to_string()),
            "la RE2020 s'applique aux permis de construire déposés à partir de 2022",
        );
    }
    r.plage(format!("{c}.hsp_moy_m"), e.hsp_moy_m, 1.8, 10.0);
    r.plage(
        format!("{c}.s_combles_m2"),
        e.s_combles_m2,
        0.0,
        f64::INFINITY,
    );
    // Les exigences ne sont transcrites que pour les usages 1 et 2 (Q54) : les autres usages
    // sont refusés par la validation de la zone (« non pris en charge par cette version »).
    let maisons = bat
        .zones
        .iter()
        .any(|z| z.usage == crate::entree::UsageZone::MaisonIndividuelle);
    if e.s_combles_m2 > 0.0 && !maisons {
        r.avertissement(
            format!("{c}.s_combles_m2"),
            Some(e.s_combles_m2.to_string()),
            "la modulation selon les combles aménagés ne vaut que pour les maisons individuelles \
             ou accolées : sans effet",
        );
    }
}
