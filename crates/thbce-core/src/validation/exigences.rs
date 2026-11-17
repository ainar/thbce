// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des données de la fixation des exigences (annexe de l'article R. 172-4 du CCH).

use thbce_data::ZoneClimatique;

use super::RapportValidation;

/// Annexe de l'article R. 172-4 du CCH, chapitre V, partie II (Q54, point 14, et Q66) :
/// catégorie de contraintes extérieures retenue pour chaque zone (celle saisie pour la zone,
/// celle du bâtiment sinon).
///
/// Une zone est de catégorie 3 si tous ses locaux autres qu'à occupation passagère le sont : un
/// local de catégorie 3 est climatisé, situé dans une zone d'un usage cité par le texte
/// ([`crate::entree::UsageZone::admet_categorie_3`]), et ses baies ne peuvent être ouvertes ou
/// il est dans un immeuble de grande hauteur (ces deux faits ne sont pas saisis). Une zone est de
/// catégorie 2 si tous ses locaux autres qu'à occupation passagère sont de catégorie 2 ou 3 : un
/// local de catégorie 2 est climatisé, ses baies sont exposées au bruit BR2 ou BR3 et le
/// bâtiment est construit en zone H2d ou H3 à moins de 400 m.
///
/// En habitation, tous les locaux sont occupés de façon non passagère : un groupe non climatisé
/// exclut les catégories 2 et 3 (erreur). Ailleurs, un groupe non climatisé peut ne contenir que
/// des locaux à occupation passagère (circulations, sanitaires) : avertissement.
fn valider_categorie_contraintes(
    r: &mut RapportValidation,
    c: &str,
    bat: &crate::entree::Batiment,
    e: &crate::entree::ParametresExigences,
    loc: &crate::entree::Localisation,
) {
    use crate::entree::{CategorieContraintes, ZoneBruit};
    let cb = c.strip_suffix(".exigences").unwrap_or(c);
    let zone_clim = loc.zone_climatique_retenue();
    // Une erreur qui tient au bâtiment (zone climatique, altitude…) n'est signalée qu'une fois
    // par chemin de saisie.
    let mut signalees: Vec<(String, &'static str)> = Vec::new();
    let mut une_fois =
        |r: &mut RapportValidation, chemin: &str, cle: &'static str, valeur: &str, m: String| {
            if !signalees.iter().any(|(ch, k)| ch == chemin && *k == cle) {
                signalees.push((chemin.to_owned(), cle));
                r.erreur(chemin.to_owned(), Some(valeur.to_owned()), m);
            }
        };
    let regle2 = "(annexe de l'article R. 172-4, chapitre V, partie II : un local de catégorie 2 \
                  est climatisé, ses baies sont exposées au bruit BR2 ou BR3, et le bâtiment est \
                  construit en zone H2d ou H3 à moins de 400 m)";
    let regle3 = "(annexe de l'article R. 172-4, chapitre V, partie II : un local de catégorie 3 \
                  est climatisé, situé dans une zone de bureaux, hôtels, restaurants, commerces, \
                  établissements d'accueil de la petite enfance, bâtiments universitaires \
                  d'enseignement et de recherche, médiathèques et bibliothèques, bâtiments \
                  d'enseignements atypiques, établissements de santé (partie jour) ou \
                  aérogares)";
    for (iz, z) in bat.zones.iter().enumerate() {
        let iez = e.zones.iter().position(|x| x.zone == z.id);
        let chemin_zone = match iez {
            Some(i) if e.zones[i].categorie_contraintes_exterieures.is_some() => {
                format!("{c}.zones[{i}].categorie_contraintes_exterieures")
            }
            _ => format!("{c}.categorie_contraintes_exterieures"),
        };
        let cat_zone = e.categorie_zone(&z.id);
        let habitation = z.usage.est_habitation();
        let n = z.usage.numero();
        let bruit = e.zone_bruit_zone(&z.id);
        for (ig, g) in z.groupes.iter().enumerate() {
            let ieg = iez.and_then(|i| e.zones[i].groupes.iter().position(|x| x.groupe == g.id));
            let cc = match (iez, ieg) {
                (Some(i), Some(j)) => {
                    format!("{c}.zones[{i}].groupes[{j}].categorie_contraintes_exterieures")
                }
                _ => chemin_zone.clone(),
            };
            let categorie = e.categorie_groupe(&z.id, &g.id);
            let (code, libelle, regle) = match categorie {
                CategorieContraintes::Categorie1 => continue,
                CategorieContraintes::Categorie2 => ("categorie_2", "catégorie 2", regle2),
                CategorieContraintes::Categorie3 => ("categorie_3", "catégorie 3", regle3),
            };
            if categorie == CategorieContraintes::Categorie3 {
                if habitation {
                    une_fois(
                        r,
                        &cc,
                        "cat3_habitation",
                        code,
                        "la catégorie 3 est réservée aux locaux climatisés de bureaux, hôtels, \
                         restaurants, commerces et autres usages non résidentiels (annexe de \
                         l'article R. 172-4, chapitre V, partie II) : impossible pour un \
                         bâtiment d'habitation"
                            .to_owned(),
                    );
                    continue;
                }
                if !z.usage.admet_categorie_3() {
                    une_fois(
                        r,
                        &cc,
                        "cat3_usage",
                        code,
                        format!(
                            "catégorie 3 impossible pour la zone {cb}.zones[{iz}] ({}), d'usage \
                             {n} {regle}",
                            z.id
                        ),
                    );
                    continue;
                }
            }
            // Climatisation : en habitation, tous les locaux sont occupés de façon non passagère.
            if g.iclim != Some(true) {
                let groupe = format!("le groupe {cb}.zones[{iz}].groupes[{ig}] ({})", g.id);
                if habitation {
                    r.erreur(
                        cc.clone(),
                        Some(code.into()),
                        format!("{libelle} impossible : {groupe} n'est pas climatisé {regle}"),
                    );
                } else {
                    r.avertissement(
                        cc.clone(),
                        Some(code.into()),
                        format!(
                            "{libelle} : {groupe} n'est pas climatisé ; elle n'est possible que \
                             s'il ne comprend que des locaux à occupation passagère {regle}"
                        ),
                    );
                }
            }
            if categorie == CategorieContraintes::Categorie2 {
                if !matches!(zone_clim, ZoneClimatique::H2d | ZoneClimatique::H3) {
                    une_fois(
                        r,
                        &cc,
                        "cat2_zone",
                        code,
                        format!("catégorie 2 impossible en zone climatique {zone_clim} {regle}"),
                    );
                }
                if loc.alt_m >= 400.0 {
                    une_fois(
                        r,
                        &cc,
                        "cat2_alt",
                        code,
                        format!(
                            "catégorie 2 impossible à {} m d'altitude (400 m ou plus) {regle}",
                            loc.alt_m
                        ),
                    );
                }
                if bruit == ZoneBruit::Br1 {
                    une_fois(
                        r,
                        &cc,
                        "cat2_br1",
                        code,
                        format!("catégorie 2 impossible en zone de bruit BR1 {regle}"),
                    );
                }
            }
        }
        // Catégorie de la zone : la catégorie 3 suppose un usage cité par le chapitre V, même si
        // tous les groupes ont une catégorie propre ; un groupe de catégorie inférieure à celle
        // de sa zone ne peut contenir que des locaux à occupation passagère.
        if cat_zone == CategorieContraintes::Categorie3
            && !habitation
            && !z.usage.admet_categorie_3()
        {
            une_fois(
                r,
                &chemin_zone,
                "cat3_usage",
                "categorie_3",
                format!(
                    "catégorie 3 impossible pour la zone {cb}.zones[{iz}] ({}), d'usage {n} \
                     {regle3}",
                    z.id
                ),
            );
        }
        let rang = |k: CategorieContraintes| k as u8;
        for (ig, g) in z.groupes.iter().enumerate() {
            let k = e.categorie_groupe(&z.id, &g.id);
            if rang(k) < rang(cat_zone) {
                r.avertissement(
                    chemin_zone.clone(),
                    None,
                    format!(
                        "le groupe {cb}.zones[{iz}].groupes[{ig}] ({}) a une catégorie inférieure \
                         à celle de sa zone : la zone n'est de cette catégorie que si ce groupe \
                         ne comprend que des locaux à occupation passagère (annexe de l'article \
                         R. 172-4, chapitre V, partie II)",
                        g.id
                    ),
                );
            }
        }
    }
}

/// Une entrée au plus par zone ou par groupe : l'erreur porte sur le champ de référence
/// (`zone`, `groupe`) et non sur un `id`.
fn references_uniques<'a>(
    r: &mut RapportValidation,
    chemin: &str,
    champ: &str,
    refs: impl Iterator<Item = &'a String>,
) {
    let mut vues: Vec<&str> = Vec::new();
    for (i, x) in refs.enumerate() {
        if vues.contains(&x.as_str()) {
            r.erreur(
                format!("{chemin}[{i}].{champ}"),
                Some(x.clone()),
                format!("{champ} déjà décrit(e) par une autre entrée de la même liste"),
            );
        } else {
            vues.push(x);
        }
    }
}

/// Données propres aux zones (`exigences.zones`) : zones existantes, une entrée par zone.
fn valider_exigences_zones(
    r: &mut RapportValidation,
    c: &str,
    bat: &crate::entree::Batiment,
    e: &crate::entree::ParametresExigences,
) {
    references_uniques(
        r,
        &format!("{c}.zones"),
        "zone",
        e.zones.iter().map(|z| &z.zone),
    );
    for (i, z) in e.zones.iter().enumerate() {
        let Some(zone) = bat.zones.iter().find(|x| x.id == z.zone) else {
            r.erreur(
                format!("{c}.zones[{i}].zone"),
                Some(z.zone.clone()),
                "zone inconnue dans ce bâtiment",
            );
            continue;
        };
        references_uniques(
            r,
            &format!("{c}.zones[{i}].groupes"),
            "groupe",
            z.groupes.iter().map(|g| &g.groupe),
        );
        for (j, g) in z.groupes.iter().enumerate() {
            if !zone.groupes.iter().any(|x| x.id == g.groupe) {
                r.erreur(
                    format!("{c}.zones[{i}].groupes[{j}].groupe"),
                    Some(g.groupe.clone()),
                    "groupe inconnu dans cette zone",
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
    valider_exigences_zones(r, &c, bat, e);
    valider_categorie_contraintes(r, &c, bat, e, loc);
    // Mois contrôlé d'abord : hors plage, les dates d'effet ne sont pas vérifiées (elles en
    // dépendent) pour ne pas ajouter une fausse erreur de date.
    let mois_valide = match e.mois_depot_permis {
        Some(m) if !(1..=12).contains(&m) => {
            r.erreur(
                format!("{c}.mois_depot_permis"),
                Some(m.to_string()),
                "mois de 1 à 12 attendu",
            );
            false
        }
        Some(_) => true,
        // Q68, décision du mainteneur du 17 novembre 2026 : le mois est obligatoire quand
        // l'année ne suffit pas à fixer la version du texte applicable.
        None => {
            let tertiaire_2022 = e.annee_depot_permis == 2022
                && bat
                    .zones
                    .iter()
                    .any(|z| (3..=5).contains(&z.usage.numero()));
            if e.annee_depot_permis == 2026 {
                r.erreur(
                    format!("{c}.mois_depot_permis"),
                    None,
                    "mois de dépôt du permis obligatoire pour un permis déposé en 2026 : trois \
                     versions de l'annexe de l'article R. 172-4 s'y succèdent (usages 6 à 28 \
                     soumis à compter du 1er mai 2026, modulations MbHSP et McHSP à compter du \
                     1er juillet 2026)",
                );
            } else if tertiaire_2022 {
                r.erreur(
                    format!("{c}.mois_depot_permis"),
                    None,
                    "mois de dépôt du permis obligatoire pour un permis déposé en 2022 avec une \
                     zone de bureaux ou d'enseignement : la RE2020 ne s'y applique qu'à compter \
                     du 1er juillet 2022",
                );
            }
            true
        }
    };
    if e.annee_depot_permis < 2022 {
        r.erreur(
            format!("{c}.annee_depot_permis"),
            Some(e.annee_depot_permis.to_string()),
            "la RE2020 s'applique aux permis de construire déposés à partir de 2022",
        );
    } else if mois_valide {
        // Q68 : usages 6 à 28 soumis à partir du 1er mai 2026 (colonne « Années 2022 à 2024 »
        // marquée « - » dans le tableau `Icénergie_maxmoyen`).
        const MOIS: [&str; 12] = [
            "janvier",
            "février",
            "mars",
            "avril",
            "mai",
            "juin",
            "juillet",
            "août",
            "septembre",
            "octobre",
            "novembre",
            "décembre",
        ];
        let mut usages: Vec<u8> = Vec::new();
        for z in &bat.zones {
            let n = z.usage.numero();
            let (annee, mois) = z.usage.debut_re2020();
            if !e.depot_a_partir_de(annee, mois) && !usages.contains(&n) {
                usages.push(n);
                r.erreur(
                    format!("{c}.annee_depot_permis"),
                    Some(e.annee_depot_permis.to_string()),
                    format!(
                        "la RE2020 ne s'applique à l'usage {n} que pour les permis de construire \
                         déposés à partir du 1er {} {annee} (article R. 172-1 du CCH)",
                        MOIS[usize::from(mois - 1)]
                    ),
                );
            }
        }
    }
    // Q69 : `MbHSP` et `McHSP` pour les permis déposés à partir du 1er juillet 2026.
    let habitation = bat.zones.iter().any(|z| z.usage.est_habitation());
    match e.hsp_moy_m {
        // Hors plage, c'est une erreur même quand la valeur est sans effet (permis antérieur
        // à juillet 2026, bâtiment sans habitation) : la saisie est incohérente.
        Some(h) => r.plage(format!("{c}.hsp_moy_m"), h, 1.8, 10.0),
        None if habitation && e.depot_a_partir_de(2026, 7) => r.erreur(
            format!("{c}.hsp_moy_m"),
            None,
            "hauteur sous plafond moyenne des locaux d'habitation obligatoire pour un bâtiment \
             comprenant une zone d'habitation (modulations MbHSP et McHSP)",
        ),
        None => {}
    }
    r.plage(
        format!("{c}.s_combles_m2"),
        e.s_combles_m2,
        0.0,
        f64::INFINITY,
    );
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
