// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des générations à boucle d'eau et des pompes à chaleur sur boucle d'eau (Th-BCE
//! 2020 §8.15).

use super::thermodynamique::valider_thermodynamique;
use super::{fluide_aval_hydraulique, postes_assures};
use crate::entree::{
    Batiment, FonctionReseau, Generation, NatureGenerateur, PacSurBoucleDEauSaisie,
    SourceAmontSaisie, TechnologieChauffagePac, TechnologieRefroidissementPac,
};
use crate::validation::RapportValidation;

/// Pompe à chaleur sur boucle d'eau (type 507) : groupe frigorifique eau / air sur la source
/// amont « boucle d'eau » (§8.15.1 et §8.15.3.1).
pub(super) fn valider_pac_boucle(
    r: &mut RapportValidation,
    c: &str,
    p: &PacSurBoucleDEauSaisie,
    g: &Generation,
    bat: &Batiment,
) {
    let cp = format!("{c}.pac");
    valider_thermodynamique(r, &cp, &p.pac, bat);
    if !bat
        .zones
        .iter()
        .flat_map(|z| z.groupes.iter())
        .any(|x| x.id == p.groupe)
    {
        r.erreur(
            format!("{c}.groupe"),
            Some(p.groupe.clone()),
            "groupe inconnu du bâtiment",
        );
    }
    if !matches!(p.pac.source_amont, SourceAmontSaisie::BoucleDEau { .. }) {
        r.erreur(
            format!("{cp}.source_amont"),
            None,
            "une pompe à chaleur sur boucle d'eau a pour source amont la boucle (`boucle_d_eau`, \
             « idamont-eau-type = 3 », Th-BCE 2020 §8.15.3.1)",
        );
    }
    if p.pac.ecs.is_some() {
        r.erreur(
            format!("{cp}.ecs"),
            None,
            "une pompe à chaleur sur boucle d'eau assure le chauffage et le refroidissement des \
             locaux d'un groupe, pas l'ECS (Th-BCE 2020 §8.15.1)",
        );
    }
    if p.pac.chauffage.is_none() && p.pac.refroidissement.is_none() {
        r.erreur(
            cp.clone(),
            None,
            "mode `chauffage` ou `refroidissement` requis (Th-BCE 2020 §8.15.3.3)",
        );
    }
    // §8.15.1 : « traitées comme des groupes frigorifiques eau / air ».
    if let Some(m) = &p.pac.chauffage {
        if m.technologie != TechnologieChauffagePac::EauDeBoucleAirRecycle {
            r.erreur(
                format!("{cp}.chauffage.technologie"),
                None,
                "technologie `eau_de_boucle_air_recycle` requise pour une pompe à chaleur sur \
                 boucle d'eau (Th-BCE 2020 §8.15.1 et tableau 145)",
            );
        }
    }
    if let Some(m) = &p.pac.refroidissement {
        if m.technologie != TechnologieRefroidissementPac::EauAir {
            r.erreur(
                format!("{cp}.refroidissement.technologie"),
                None,
                "technologie `eau_air` requise pour une pompe à chaleur sur boucle d'eau (Th-BCE \
                 2020 §8.15.1 et tableau 145)",
            );
        }
    }
    if g.boucle_d_eau.is_none() {
        r.erreur(
            c.to_owned(),
            None,
            "une pompe à chaleur sur boucle d'eau appartient à une génération qui décrit la \
             boucle (`boucle_d_eau`, Th-BCE 2020 §8.15.3.1)",
        );
    }
}

/// Composition d'une génération à boucle d'eau (§8.15.3.1) : au moins une pompe à chaleur sur
/// boucle d'eau, toutes sur la même boucle ; un générateur de chauffage et un générateur de
/// froid (ou la tour) autres que des pompes à chaleur sur boucle d'eau, sur eau, chacun d'une
/// seule fonction, selon les modes des pompes à chaleur ; pas d'ECS (lecture Q240).
pub(super) fn valider_boucle(r: &mut RapportValidation, c: &str, g: &Generation, bat: &Batiment) {
    let Some(b) = &g.boucle_d_eau else {
        return;
    };
    let cb = format!("{c}.boucle_d_eau");
    let pac: Vec<&PacSurBoucleDEauSaisie> = g
        .generateurs
        .iter()
        .filter_map(|x| match &x.nature {
            NatureGenerateur::PacSurBoucleDEau(p) => Some(p.as_ref()),
            _ => None,
        })
        .collect();
    if pac.is_empty() {
        r.erreur(
            format!("{c}.generateurs"),
            None,
            "une génération à boucle d'eau compte au moins une pompe à chaleur sur boucle d'eau \
             (`pac_sur_boucle_d_eau`, type 507, Th-BCE 2020 §8.15.3.1)",
        );
    }
    // Une seule boucle : même température de boucle pour toutes les pompes à chaleur (éq. 1437).
    let boucles: Vec<_> = pac
        .iter()
        .filter_map(|p| match &p.pac.source_amont {
            SourceAmontSaisie::BoucleDEau { boucle, .. } => Some(*boucle),
            _ => None,
        })
        .collect();
    if boucles.windows(2).any(|w| w[0] != w[1]) {
        r.erreur(
            format!("{c}.generateurs"),
            None,
            "les pompes à chaleur d'une même boucle d'eau ont la même température de boucle \
             (`source_amont.boucle_d_eau.boucle`, Th-BCE 2020 §8.15.2 et éq. 1437)",
        );
    }
    let chauffe = pac.iter().any(|p| p.pac.chauffage.is_some());
    let refroidit = pac.iter().any(|p| p.pac.refroidissement.is_some());
    // Générateurs de la boucle (§8.15.3.5).
    let mut nb = [0usize; 2];
    for (k, x) in g.generateurs.iter().enumerate() {
        if matches!(x.nature, NatureGenerateur::PacSurBoucleDEau(_)) {
            continue;
        }
        let cg = format!("{c}.generateurs[{k}].nature");
        let postes = postes_assures(&x.nature);
        if postes[2] || (postes[0] && postes[1]) || !(postes[0] || postes[1]) {
            r.erreur(
                cg.clone(),
                None,
                "le générateur d'une boucle d'eau en assure soit le chauffage (« idfougen = 1 »), \
                 soit le refroidissement (« idfougen = 2 ») (Th-BCE 2020 §8.15.3.1)",
            );
        }
        if !eau_aval(&x.nature) {
            r.erreur(
                cg,
                None,
                "le générateur d'une boucle d'eau a l'eau pour fluide aval (Th-BCE 2020 \
                 §8.15.3.5)",
            );
        }
        nb[0] += usize::from(postes[0]);
        nb[1] += usize::from(postes[1]);
    }
    if b.tour.is_some() {
        nb[1] += 1;
    }
    for (p, (nom, utile, nb)) in [
        ("chauffage", chauffe, nb[0]),
        ("refroidissement", refroidit, nb[1]),
    ]
    .into_iter()
    .enumerate()
    {
        if nb > 1 {
            r.erreur(
                format!("{c}.generateurs"),
                None,
                format!(
                    "une boucle d'eau a un seul générateur de {nom} (la tour de refroidissement \
                     comprise ; Th-BCE 2020 §8.15.3.6 et §8.15.3.7)"
                ),
            );
        }
        if utile && nb == 0 {
            r.erreur(
                format!("{c}.generateurs"),
                None,
                format!(
                    "générateur de {nom} de la boucle requis : des pompes à chaleur de la boucle \
                     assurent le {nom}, la boucle est reliée à une source de {} pour son \
                     maintien en température (Th-BCE 2020 §8.15.1 et §8.15.3.1)",
                    if p == 0 { "chaleur" } else { "froid" }
                ),
            );
        }
    }
    // Températures de fonctionnement des générateurs de la boucle (éq. 969 et 975).
    let generateur = |p: usize| {
        g.generateurs.iter().any(|x| {
            !matches!(x.nature, NatureGenerateur::PacSurBoucleDEau(_))
                && postes_assures(&x.nature)[p]
        })
    };
    match (generateur(0), b.theta_max_rech_c) {
        (true, None) => r.erreur(
            format!("{cb}.theta_max_rech_c"),
            None,
            "« θmax_rech_BE » requise avec un générateur de chauffage de la boucle (Th-BCE 2020 \
             §8.15.3.6, éq. 969)",
        ),
        (_, Some(t)) => r.plage(format!("{cb}.theta_max_rech_c"), t, 20.0, 100.0),
        _ => {}
    }
    match (generateur(1), b.theta_min_fr_c) {
        (true, None) => r.erreur(
            format!("{cb}.theta_min_fr_c"),
            None,
            "« θmin_fr_BE » requise avec un générateur de froid de la boucle (Th-BCE 2020 \
             §8.15.3.7.2, éq. 975)",
        ),
        (_, Some(t)) => r.plage(format!("{cb}.theta_min_fr_c"), t, 0.0, 100.0),
        _ => {}
    }
    if let Some(t) = &b.tour {
        let ct = format!("{cb}.tour");
        if g.generateurs.iter().any(|x| x.id == t.id) {
            r.erreur(
                format!("{ct}.id"),
                Some(t.id.clone()),
                "identifiant déjà employé par un générateur de la génération",
            );
        }
        r.positif(format!("{ct}.delta_theta_tour_k"), t.delta_theta_tour_k);
        r.strictement_positif(format!("{ct}.qv_nom_m3_h"), t.qv_nom_m3_h);
        r.positif(format!("{ct}.p_pompes_tour_w"), t.p_pompes_tour_w);
        r.positif(format!("{ct}.p_vent_tour_w"), t.p_vent_tour_w);
    }
    // Hors périmètre (lecture Q240) : ECS et stockages, centrales de traitement d'air.
    if !g.stockages.is_empty()
        || !g.chauffe_eau_thermodynamiques.is_empty()
        || !g.chauffe_eau_solaires.is_empty()
        || !g.systemes_solaires_combines.is_empty()
        || !g.chauffe_eau_solaires_collectifs.is_empty()
        || bat
            .distributions_ecs_intergroupes
            .iter()
            .any(|d| d.generation == g.id)
    {
        r.hors_perimetre(
            cb.clone(),
            None,
            "production d'ECS par une génération à boucle d'eau (Th-BCE 2020 §8.15 ; lecture \
             Q240)",
        );
    }
    if g.priorite.is_some() {
        r.erreur(
            format!("{c}.priorite"),
            None,
            "sans objet pour une génération à boucle d'eau : la fiche 8.15 répartit la demande \
             de chaque groupe entre ses pompes à chaleur au prorata de leurs puissances (éq. 961 \
             et 964)",
        );
    }
    // §8.15.1 : les pompes à chaleur de la boucle sont des machines eau / air ; leurs réseaux
    // sont fictifs (§8.17.3.1).
    for (i, d) in bat.distributions_intergroupes.iter().enumerate() {
        if d.generation == g.id && !matches!(d.reseau, crate::entree::ReseauIntergroupes::Fictif) {
            r.erreur(
                format!(
                    "{}.distributions_intergroupes[{i}].reseau",
                    super::c_batiment(c)
                ),
                None,
                "les réseaux d'une génération à boucle d'eau sont fictifs : ses pompes à chaleur \
                 sont des machines eau / air (Th-BCE 2020 §8.15.1 et §8.17.3.1)",
            );
        }
    }
    // Chaque groupe relié à la génération est desservi par des pompes à chaleur de la boucle
    // pour chacun de ses postes (lien d'interface, §8.15.3.1).
    for z in &bat.zones {
        for gr in &z.groupes {
            for d in &gr.distributions {
                let Some(di) = bat
                    .distributions_intergroupes
                    .iter()
                    .find(|x| x.id == d.distribution_intergroupe)
                else {
                    continue;
                };
                if di.generation != g.id {
                    continue;
                }
                let froid = di.fonction == FonctionReseau::Refroidissement;
                let desservi = pac.iter().any(|p| {
                    p.groupe == gr.id
                        && if froid {
                            p.pac.refroidissement.is_some()
                        } else {
                            p.pac.chauffage.is_some()
                        }
                });
                if !desservi {
                    r.erreur(
                        format!("{c}.generateurs"),
                        Some(gr.id.clone()),
                        format!(
                            "le groupe « {} » est relié à la génération à boucle d'eau pour le \
                             {} : une pompe à chaleur de la boucle doit lui être associée \
                             (`groupe`) et assurer ce poste (Th-BCE 2020 §8.15.3.1)",
                            gr.id,
                            if froid {
                                "refroidissement"
                            } else {
                                "chauffage"
                            }
                        ),
                    );
                }
            }
        }
    }
}

/// Vrai si le générateur a l'eau pour fluide aval (§8.15.3.5) : chaudières, réseaux de chaleur
/// et de froid, cogénérations, générateurs thermodynamiques sur eau.
fn eau_aval(n: &NatureGenerateur) -> bool {
    use super::thermodynamique::{aval_fictif_chauffage, aval_fictif_refroidissement};
    match n {
        NatureGenerateur::ThermodynamiqueElectrique(t) => {
            t.chauffage
                .as_ref()
                .is_none_or(|m| !aval_fictif_chauffage(m.technologie))
                && t.refroidissement
                    .as_ref()
                    .is_none_or(|m| !aval_fictif_refroidissement(m.technologie))
        }
        NatureGenerateur::ThermodynamiqueGaz(t) => super::thermodynamique_gaz::fluide_aval_gaz(t)
            .into_iter()
            .all(|x| x != Some(false)),
        autre => fluide_aval_hydraulique(autre) == Some(true),
    }
}
