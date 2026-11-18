// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des générations (Th-BCE 2020 §8.14, §8.17 et §8.18) : générateurs, gestion des
//! priorités, emplacement, cohérence du montage (§8.17.3.1) et génération associée à la
//! sécurité antigel d'un double flux.

use super::ecs::valider_stockages;
use super::RapportValidation;
use crate::entree::{
    Batiment, FonctionChaleur, FonctionEffetJoule, FonctionReseau, Generation,
    GestionTemperatureGeneration, NatureGenerateur, PositionGeneration, ReseauChaleurFroidSaisi,
    ReseauIntergroupes, TechnologieChauffagePac, TechnologieRefroidissementPac, TypePriorite,
};
use combustion::{valider_chaudiere, valider_poele};
use reseaux_chaleur::valider_reseau;
use thermodynamique::valider_thermodynamique;

// Un fichier par famille de générateurs. Un nouveau générateur ajoute sa nature dans
// `postes_assures` et `valider_generation`, et ses contrôles dans un fichier déclaré ci-dessous.
mod combustion;
mod reseaux_chaleur;
mod thermodynamique;

/// Postes qu'un générateur saisi peut assurer : `[chauffage, refroidissement, ECS]`.
pub(super) fn postes_assures(n: &NatureGenerateur) -> [bool; 3] {
    match n {
        NatureGenerateur::EffetJoule(j) => match j.fonction {
            FonctionEffetJoule::Chauffage => [true, false, false],
            FonctionEffetJoule::Ecs => [false, false, true],
        },
        NatureGenerateur::ThermodynamiqueElectrique(t) => [
            t.chauffage.is_some(),
            t.refroidissement.is_some(),
            t.ecs.is_some(),
        ],
        NatureGenerateur::Chaudiere(c) => [
            c.fonction != FonctionChaleur::Ecs,
            false,
            c.fonction != FonctionChaleur::Chauffage,
        ],
        NatureGenerateur::PoeleOuInsert(_) => [true, false, false],
        NatureGenerateur::ReseauDeChaleurOuDeFroid(ReseauChaleurFroidSaisi::Chaleur {
            fonction,
            ..
        }) => [
            *fonction != FonctionChaleur::Ecs,
            false,
            *fonction != FonctionChaleur::Chauffage,
        ],
        NatureGenerateur::ReseauDeChaleurOuDeFroid(ReseauChaleurFroidSaisi::Froid { .. }) => {
            [false, true, false]
        }
        _ => [false; 3],
    }
}

/// Valide les générations d'un bâtiment.
pub(super) fn valider_generations(r: &mut RapportValidation, cb: &str, bat: &Batiment) {
    for (i, g) in bat.generations.iter().enumerate() {
        valider_generation(r, &format!("{cb}.generations[{i}]"), g, bat);
    }
    // Réseaux de CTA fictifs (fiche 8.11) : génération désignée par la sécurité antigel et par
    // les batteries de préchauffage et de prérefroidissement des centrales double flux
    // (§8.11.3, éq. 937 et 939).
    for (iz, z) in bat.zones.iter().enumerate() {
        let Some(v) = z.ventilation.as_ref() else {
            continue;
        };
        let Some(centrale) = v.systeme.nom_centrale() else {
            continue;
        };
        let cs = format!("{cb}.zones[{iz}].ventilation.systeme.{centrale}");
        let designee =
            |r: &mut RapportValidation, c: String, id: &str, poste: usize, nom: &str| match bat
                .generations
                .iter()
                .find(|g| g.id == id)
            {
                None => r.erreur(c, Some(id.to_owned()), "génération inconnue du bâtiment"),
                Some(g) => {
                    if !g
                        .generateurs
                        .iter()
                        .any(|x| postes_assures(&x.nature)[poste])
                    {
                        r.erreur(
                            c,
                            Some(id.to_owned()),
                            format!(
                                "la génération associée {nom} n'a pas de générateur de {} \
                                 (Th-BCE 2020 §8.11.3.1 et §8.17.3.1)",
                                if poste == 0 {
                                    "chauffage"
                                } else {
                                    "refroidissement"
                                }
                            ),
                        );
                    }
                }
            };
        if let Some(b) = v.systeme.batterie_prechauffage() {
            designee(
                r,
                format!("{cs}.batterie_prechauffage.generation"),
                &b.generation,
                0,
                "au préchauffage",
            );
        }
        if let Some(b) = v.systeme.batterie_prerefroidissement() {
            designee(
                r,
                format!("{cs}.batterie_prerefroidissement.generation"),
                &b.generation,
                1,
                "au prérefroidissement",
            );
        }
        let Some(e) = v.systeme.echangeur() else {
            continue;
        };
        let c = format!("{cs}.echangeur.generation_antigel");
        match (&e.generation_antigel, e.antigel == Some(true)) {
            (None, true) => r.erreur(
                c,
                None,
                "génération associée à la sécurité antigel requise : l'énergie de préchauffage \
                 de l'air neuf (Th-BCE 2020 §6.3, éq. 508) est demandée à une génération du \
                 bâtiment (§8.11.3, éq. 939)",
            ),
            (Some(id), false) => r.erreur(
                c,
                Some(id.clone()),
                "sans objet : l'échangeur n'a pas de sécurité antigel (`antigel`)",
            ),
            (Some(id), true) => designee(r, c, id, 0, "à la sécurité antigel"),
            (None, false) => {}
        }
    }
}

fn valider_generation(r: &mut RapportValidation, c: &str, g: &Generation, bat: &Batiment) {
    if let PositionGeneration::HorsVolumeChauffe {
        espace_tampon: Some(id),
    } = &g.position
    {
        if !bat.espaces_tampons.iter().any(|e| e.id == *id) {
            r.erreur(
                format!("{c}.position.hors_volume_chauffe.espace_tampon"),
                Some(id.clone()),
                "espace tampon inconnu du bâtiment",
            );
        }
    }
    for (nom, x) in [
        (
            "gestion_temperature_chauffage",
            g.gestion_temperature_chauffage,
        ),
        (
            "gestion_temperature_refroidissement",
            g.gestion_temperature_refroidissement,
        ),
    ] {
        if let Some(GestionTemperatureGeneration::Constante { theta_wm_c }) = x {
            r.plage(
                format!("{c}.{nom}.constante.theta_wm_c"),
                theta_wm_c,
                0.0,
                100.0,
            );
        }
    }
    if g.priorite == Some(TypePriorite::Alternance) {
        r.hors_perimetre(
            format!("{c}.priorite"),
            Some("alternance".into()),
            "gestion des générateurs en alternance (Th-BCE 2020 §8.17.3.7.2.3)",
        );
    }
    if g.generateurs.is_empty()
        && g.chauffe_eau_thermodynamiques.is_empty()
        && g.chauffe_eau_solaires.is_empty()
        && g.systemes_solaires_combines.is_empty()
        && g.chauffe_eau_solaires_collectifs.is_empty()
    {
        r.erreur(
            format!("{c}.generateurs"),
            None,
            "au moins un générateur requis : une génération est un ensemble de générateurs \
             (Th-BCE 2020 §8.14.1)",
        );
    }
    valider_stockages(r, c, g, bat);
    // Générateurs pilotés par un ballon : ils ne sont pas des générateurs instantanés d'ECS.
    let pilote = |id: &str| {
        g.stockages.iter().any(|s| {
            s.base.generateur == id || s.appoint.as_ref().is_some_and(|a| a.generateur == id)
        }) || g.chauffe_eau_solaires.iter().any(
            |s| matches!(&s.appoint, crate::entree::AppointCesi::Integre(a) if a.generateur == id),
        ) || g
            .systemes_solaires_combines
            .iter()
            .any(|s| s.appoint.generateur == id)
            || g.chauffe_eau_solaires_collectifs.iter().any(|c| {
                c.ensembles
                    .iter()
                    .any(|e| e.appoint.as_ref().is_some_and(|a| a.generateur == id))
            })
    };
    let avec_stockage = !g.stockages.is_empty()
        || !g.chauffe_eau_thermodynamiques.is_empty()
        || !g.chauffe_eau_solaires.is_empty()
        || !g.systemes_solaires_combines.is_empty()
        || !g.chauffe_eau_solaires_collectifs.is_empty();
    if avec_stockage && g.priorite == Some(TypePriorite::SansPriorite) {
        r.erreur(
            format!("{c}.priorite"),
            Some("sans_priorite".into()),
            "une génération pourvue d'un ballon de stockage est régulée en cascade (Th-BCE \
             2020 §8.17.3.6)",
        );
    }
    r.ids_uniques(
        &format!("{c}.generateurs"),
        g.generateurs.iter().map(|x| &x.id),
    );
    let mut nb = [0usize; 3];
    for (k, x) in g.generateurs.iter().enumerate() {
        let cg = format!("{c}.generateurs[{k}]");
        if x.rdim == 0 {
            r.erreur(
                format!("{cg}.rdim"),
                Some("0".into()),
                "« Rdim » : nombre de générateurs identiques, au moins 1 (Th-BCE 2020 §8.17.2)",
            );
        }
        match &x.nature {
            NatureGenerateur::EffetJoule(j) => {
                r.strictement_positif(format!("{cg}.nature.effet_joule.pn_gen_w"), j.pn_gen_w);
            }
            NatureGenerateur::ThermodynamiqueElectrique(t) => {
                valider_thermodynamique(
                    r,
                    &format!("{cg}.nature.thermodynamique_electrique"),
                    t,
                    bat,
                );
            }
            NatureGenerateur::Chaudiere(c) => {
                valider_chaudiere(r, &format!("{cg}.nature.chaudiere"), c);
            }
            NatureGenerateur::PoeleOuInsert(p) => {
                valider_poele(r, &format!("{cg}.nature.poele_ou_insert"), p);
            }
            NatureGenerateur::ReseauDeChaleurOuDeFroid(n) => {
                valider_reseau(r, &format!("{cg}.nature.reseau_de_chaleur_ou_de_froid"), n);
            }
            autre => {
                let (nom, quoi) = match autre {
                    NatureGenerateur::AutreGenerateurACombustion => (
                        "autre_generateur_a_combustion",
                        "autre générateur à combustion (§8.20)",
                    ),
                    NatureGenerateur::ThermodynamiqueGaz => (
                        "thermodynamique_gaz",
                        "générateur thermodynamique au gaz, à absorption ou à moteur (§8.24 et \
                         §8.25)",
                    ),
                    NatureGenerateur::BoucleDEauOuDrv => (
                        "boucle_d_eau_ou_drv",
                        "pompe à chaleur sur boucle d'eau, thermo-frigo-pompe ou générateur DRV \
                         (§8.15 et §8.16)",
                    ),
                    NatureGenerateur::Cogeneration => {
                        ("cogeneration", "système de cogénération (§8.21)")
                    }
                    _ => ("pac_hybride", "pompe à chaleur hybride"),
                };
                r.hors_perimetre(format!("{cg}.nature"), Some(nom.into()), quoi);
            }
        }
        let assures = postes_assures(&x.nature);
        for (p, a) in assures.into_iter().enumerate() {
            nb[p] += usize::from(a && !(p == 2 && pilote(&x.id)));
        }
        if assures[2]
            && matches!(x.nature, NatureGenerateur::ThermodynamiqueElectrique(_))
            && !g.stockages.iter().any(|s| s.base.generateur == x.id)
            && !g
                .systemes_solaires_combines
                .iter()
                .any(|s| s.appoint.generateur == x.id)
        {
            r.erreur(
                format!("{cg}.nature.thermodynamique_electrique.ecs"),
                None,
                "un générateur thermodynamique d'ECS est le générateur de base d'un assemblage \
                 avec stockage (`stockages`, Th-BCE 2020 §8.23.3.2)",
            );
        }
    }
    // Priorités (§8.17.3.7) : mode requis dès que deux générateurs assurent un même poste ;
    // en cascade, rangs requis et distincts.
    let noms = ["chauffage", "refroidissement", "ecs"];
    for p in 0..3 {
        if nb[p] < 2 {
            continue;
        }
        match g.priorite {
            None => r.erreur(
                format!("{c}.priorite"),
                None,
                format!(
                    "« idtype_priorite » requis : plusieurs générateurs assurent le poste {} \
                     (Th-BCE 2020 §8.17.1)",
                    noms[p]
                ),
            ),
            Some(TypePriorite::Cascade) => {
                let mut vus = std::collections::BTreeSet::new();
                for (k, x) in g.generateurs.iter().enumerate() {
                    if !postes_assures(&x.nature)[p] {
                        continue;
                    }
                    let rang = [
                        x.priorite_chauffage,
                        x.priorite_refroidissement,
                        x.priorite_ecs,
                    ][p];
                    let champ = format!(
                        "{c}.generateurs[{k}].priorite_{}",
                        ["chauffage", "refroidissement", "ecs"][p]
                    );
                    match rang {
                        None | Some(0) => r.erreur(
                            champ,
                            rang.map(|x| x.to_string()),
                            "rang de priorité requis (à partir de 1) pour une gestion en cascade \
                             (Th-BCE 2020 §8.17.3.7.2.2)",
                        ),
                        Some(n) => {
                            if !vus.insert(n) {
                                r.erreur(
                                    champ,
                                    Some(n.to_string()),
                                    "rang de priorité déjà attribué à un autre générateur pour \
                                     ce poste",
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    // Cohérence du montage (§8.17.3.1).
    // Un système solaire combiné assure lui-même une partie du chauffage (fiches 9.19 et 9.20).
    let assure = |p: usize| {
        nb[p] > 0
            || (p == 2 && avec_stockage)
            || (p == 0 && !g.systemes_solaires_combines.is_empty())
    };
    let mut types: [Option<bool>; 2] = [None; 2];
    for (i, d) in bat.distributions_intergroupes.iter().enumerate() {
        if d.generation != g.id {
            continue;
        }
        let p = usize::from(d.fonction == FonctionReseau::Refroidissement);
        if !assure(p) && !g.generateurs.is_empty() {
            r.erreur(
                format!("{c}.generateurs"),
                None,
                format!(
                    "aucun générateur de la génération n'assure le poste {} du réseau « {} » \
                     (Th-BCE 2020 §8.17.3.1)",
                    noms[p], d.id
                ),
            );
        }
        let hydraulique = !matches!(d.reseau, ReseauIntergroupes::Fictif);
        match types[p] {
            None => types[p] = Some(hydraulique),
            Some(t) if t != hydraulique => r.erreur(
                format!("{}.distributions_intergroupes[{i}].reseau", c_batiment(c)),
                None,
                format!(
                    "les réseaux intergroupes de {} d'une même génération sont tous fictifs ou \
                     tous hydrauliques (Th-BCE 2020 §8.17.3.1)",
                    noms[p]
                ),
            ),
            _ => {}
        }
    }
    // Cohérence entre les réseaux et le fluide aval des générateurs thermodynamiques
    // (§8.17.3.1) : réseaux fictifs et générateurs sur air, réseaux hydrauliques et générateurs
    // sur eau.
    for (k, x) in g.generateurs.iter().enumerate() {
        let NatureGenerateur::ThermodynamiqueElectrique(t) = &x.nature else {
            continue;
        };
        let sur_air = [
            t.chauffage
                .as_ref()
                .map(|m| m.technologie == TechnologieChauffagePac::AirExterieurAirRecycle),
            t.refroidissement
                .as_ref()
                .map(|m| m.technologie == TechnologieRefroidissementPac::AirExterieurAirRecycle),
        ];
        for p in 0..2 {
            if let (Some(air), Some(hydraulique)) = (sur_air[p], types[p]) {
                if air == hydraulique {
                    r.erreur(
                        format!(
                            "{c}.generateurs[{k}].nature.thermodynamique_electrique.{}.technologie",
                            noms[p]
                        ),
                        None,
                        format!(
                            "technologie incohérente avec les réseaux de {} de la génération : \
                             un générateur sur air alimente des réseaux fictifs, un générateur \
                             sur eau des réseaux hydrauliques (Th-BCE 2020 §8.17.3.1)",
                            noms[p]
                        ),
                    );
                }
            }
        }
    }
    // Chaudières et sous-stations de réseau de chaleur : fluide aval eau ; poêles et inserts :
    // air ambiant (§8.19.2, §8.22 et §8.17.3.1).
    for (k, x) in g.generateurs.iter().enumerate() {
        let hydraulique = match &x.nature {
            NatureGenerateur::Chaudiere(_) | NatureGenerateur::ReseauDeChaleurOuDeFroid(_) => true,
            NatureGenerateur::PoeleOuInsert(_) => false,
            _ => continue,
        };
        for p in 0..2 {
            if postes_assures(&x.nature)[p] && types[p] == Some(!hydraulique) {
                r.erreur(
                    format!("{c}.generateurs[{k}].nature"),
                    None,
                    format!(
                        "générateur incohérent avec les réseaux de {} de la génération : {} \
                         (Th-BCE 2020 §8.17.3.1)",
                        noms[p],
                        if hydraulique {
                            "une chaudière ou une sous-station de réseau de chaleur alimente des \
                             réseaux hydrauliques"
                        } else {
                            "un poêle ou un insert alimente des réseaux fictifs (air)"
                        }
                    ),
                );
            }
        }
    }
    for (p, nom) in [
        "gestion_temperature_chauffage",
        "gestion_temperature_refroidissement",
    ]
    .into_iter()
    .enumerate()
    {
        let saisie = [
            g.gestion_temperature_chauffage,
            g.gestion_temperature_refroidissement,
        ][p];
        if types[p] == Some(true) && saisie.is_none() {
            r.erreur(
                format!("{c}.{nom}"),
                None,
                "gestion de la température de fonctionnement requise pour une génération qui \
                 alimente des réseaux hydrauliques : `constante` ou `reseaux` (Th-BCE 2020 \
                 §8.17.3.5.10.1)",
            );
        }
    }
    if bat
        .distributions_ecs_intergroupes
        .iter()
        .any(|d| d.generation == g.id)
        && !assure(2)
        && g.chauffe_eau_solaires.is_empty()
        && !(g.generateurs.is_empty() && g.chauffe_eau_thermodynamiques.is_empty())
    {
        r.erreur(
            format!("{c}.generateurs"),
            None,
            "aucun générateur de la génération n'assure la production d'ECS de ses \
             distributions (Th-BCE 2020 §8.17.3.1)",
        );
    }
}

/// Chemin du bâtiment à partir de celui d'une de ses générations.
fn c_batiment(c: &str) -> &str {
    c.rfind(".generations[").map_or(c, |i| &c[..i])
}
