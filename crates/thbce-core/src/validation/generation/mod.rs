// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des générations (Th-BCE 2020 §8.14, §8.17 et §8.18) : générateurs, gestion des
//! priorités, emplacement, cohérence du montage (§8.17.3.1) et génération associée à la
//! sécurité antigel d'un double flux.

use super::ecs::valider_stockages;
use super::RapportValidation;
use crate::entree::{
    Batiment, FonctionChaleur, FonctionEffetJoule, FonctionReseau, Generation,
    GestionTemperatureGeneration, NatureGenerateur, PositionGeneration, ReseauChaleurFroidSaisi,
    ReseauIntergroupes, SystemeVentilation, TechnologieChauffagePac, TechnologieRefroidissementPac,
    TypePriorite,
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
        // Natures refusées (code 3) : le lot qui les ouvre donne ici leurs postes, tirés de
        // leur saisie (`docs/architecture.md`, §3.10).
        NatureGenerateur::AutreGenerateurACombustion
        | NatureGenerateur::ThermodynamiqueGaz
        | NatureGenerateur::BoucleDEauOuDrv
        | NatureGenerateur::Cogeneration
        | NatureGenerateur::PacHybride => [false; 3],
    }
}

/// Natures de générateurs reconnues mais non calculées (code 3) : nom de la variante et
/// fiche du texte. Point d'entrée unique des lots qui les ouvrent : retirer la ligne, puis
/// suivre la recette « ajouter un générateur » (`docs/architecture.md`, §3.1 et §3.10).
pub(super) fn nature_hors_perimetre(n: &NatureGenerateur) -> Option<(&'static str, &'static str)> {
    match n {
        NatureGenerateur::EffetJoule(_)
        | NatureGenerateur::ThermodynamiqueElectrique(_)
        | NatureGenerateur::Chaudiere(_)
        | NatureGenerateur::PoeleOuInsert(_)
        | NatureGenerateur::ReseauDeChaleurOuDeFroid(_) => None,
        NatureGenerateur::AutreGenerateurACombustion => Some((
            "autre_generateur_a_combustion",
            "autre générateur à combustion (Th-BCE 2020 §8.20, types 103 à 109)",
        )),
        NatureGenerateur::ThermodynamiqueGaz => Some((
            "thermodynamique_gaz",
            "générateur thermodynamique au gaz, à absorption ou à moteur (Th-BCE 2020 §8.24 et \
             §8.25, type 504)",
        )),
        NatureGenerateur::BoucleDEauOuDrv => Some((
            "boucle_d_eau_ou_drv",
            "pompe à chaleur sur boucle d'eau, thermo-frigo-pompe ou générateur DRV (Th-BCE 2020 \
             §8.15 et §8.16, types 507 à 509)",
        )),
        NatureGenerateur::Cogeneration => Some((
            "cogeneration",
            "système de cogénération (Th-BCE 2020 §8.21, type 700)",
        )),
        // Q126 : la PAC hybride n'est pas un type du tableau 111 ; la fiche d'application du
        // §16.4 la décrit par ses composants (générateur thermodynamique, chaudière à
        // condensation, ballon le cas échéant) dans une même génération.
        NatureGenerateur::PacHybride => Some((
            "pac_hybride",
            "pompe à chaleur hybride en un seul générateur : la fiche d'application du Th-BCE \
             2020 §16.4 la décrit par ses composants, une pompe à chaleur \
             (`thermodynamique_electrique`) et une chaudière à condensation (`chaudiere`) dans \
             une même génération en cascade ; la saisie dédiée et la régulation en énergie \
             primaire du §16.4.3 ne sont pas prises en charge",
        )),
    }
}

/// « idfluide-aval » (tableau 110) imposé par la nature d'un générateur : `Some(true)` pour
/// l'eau (réseaux hydrauliques), `Some(false)` pour l'air ambiant (réseaux fictifs), `None`
/// quand il dépend de la technologie saisie (générateurs thermodynamiques, contrôlés à part)
/// ou que les deux sont admis (effet Joule, Q46 point 8). Contrôle du §8.17.3.1 : un
/// générateur ouvert par un lot à venir y reçoit sa branche (`docs/architecture.md`, §3.10).
pub(super) fn fluide_aval_hydraulique(n: &NatureGenerateur) -> Option<bool> {
    match n {
        NatureGenerateur::Chaudiere(_) | NatureGenerateur::ReseauDeChaleurOuDeFroid(_) => {
            Some(true)
        }
        NatureGenerateur::PoeleOuInsert(_) => Some(false),
        NatureGenerateur::EffetJoule(_) | NatureGenerateur::ThermodynamiqueElectrique(_) => None,
        // Natures refusées (code 3) : à fixer par le lot qui les ouvre.
        NatureGenerateur::AutreGenerateurACombustion
        | NatureGenerateur::ThermodynamiqueGaz
        | NatureGenerateur::BoucleDEauOuDrv
        | NatureGenerateur::Cogeneration
        | NatureGenerateur::PacHybride => None,
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
        // Éq. (939) : humidificateur d'hiver à vapeur (« idchaudHR »).
        if let Some(id) = v
            .systeme
            .humidificateur()
            .and_then(|h| h.generation.as_ref())
        {
            designee(
                r,
                format!("{cs}.humidificateur.generation"),
                id,
                0,
                "à l'humidificateur d'hiver",
            );
        }
        // Éq. (940) : batteries régulées sur la charge des centrales à charge variable.
        if let SystemeVentilation::Cta(cta) = &v.systeme {
            for (present, champ, id, poste, role) in [
                (
                    cta.batterie_chaude == Some(true),
                    "generation_batterie_chaude",
                    &cta.generation_batterie_chaude,
                    0,
                    "à la batterie chaude",
                ),
                (
                    cta.batterie_froide == Some(true),
                    "generation_batterie_froide",
                    &cta.generation_batterie_froide,
                    1,
                    "à la batterie froide",
                ),
            ] {
                let c = format!("{cs}.{champ}");
                match (id, present) {
                    (None, true) => r.erreur(
                        c,
                        None,
                        "génération requise : l'énergie de la batterie régulée sur la charge \
                         (Th-BCE 2020 §6.3.3.5.8, éq. 598 et 611) est demandée à une génération \
                         du bâtiment par un réseau de CTA (§8.11.3, éq. 940)",
                    ),
                    (Some(id), false) => r.erreur(
                        c,
                        Some(id.clone()),
                        "sans objet sans batterie régulée sur la charge",
                    ),
                    (Some(id), true) => designee(r, c, id, poste, role),
                    (None, false) => {}
                }
            }
        }
        valider_reseaux_cta(r, bat, &format!("{cb}.zones[{iz}].ventilation"), v);
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
    if let (true, Some(p @ (TypePriorite::SansPriorite | TypePriorite::Alternance))) =
        (avec_stockage, g.priorite)
    {
        r.erreur(
            format!("{c}.priorite"),
            Some(
                if p == TypePriorite::Alternance {
                    "alternance"
                } else {
                    "sans_priorite"
                }
                .into(),
            ),
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
                if let Some((nom, quoi)) = nature_hors_perimetre(autre) {
                    r.hors_perimetre(format!("{cg}.nature"), Some(nom.into()), quoi);
                }
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
    // §8.17.3.7.2.3 : « vérification supplémentaire de cohérence du montage » en alternance,
    // tous les générateurs ont la même fonction, autre que chauffage et refroidissement
    // (lecture Q125, point 1).
    if g.priorite == Some(TypePriorite::Alternance) {
        valider_alternance(r, c, g);
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
        let Some(hydraulique) = fluide_aval_hydraulique(&x.nature) else {
            continue;
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

/// Gestion en alternance (§8.17.3.7.2.3) : « tous les générateurs doivent avoir le même
/// idfougen, et il ne peut pas s'agir de idfougen = 4 (chauffage et refroidissement) ». Le
/// texte numérote 4 « chauffage et ECS » au tableau 110 et traite ce cas au
/// §8.17.3.7.2.3.4 : la fonction exclue est lue « chauffage et refroidissement » (5, lecture
/// Q125, point 1 ; `docs/reference/index/anomalies-constatees.md`).
fn valider_alternance(r: &mut RapportValidation, c: &str, g: &Generation) {
    // Les natures hors périmètre (code 3) n'ont pas encore de fonction : écartées.
    let mut calcules = g
        .generateurs
        .iter()
        .enumerate()
        .filter(|(_, x)| nature_hors_perimetre(&x.nature).is_none());
    let Some((_, premier)) = calcules.next() else {
        return;
    };
    let fonction = postes_assures(&premier.nature);
    for (k, x) in calcules {
        if postes_assures(&x.nature) != fonction {
            r.erreur(
                format!("{c}.generateurs[{k}].nature"),
                None,
                format!(
                    "en alternance, tous les générateurs ont la même fonction (« idfougen ») que \
                     le premier, « {} » (Th-BCE 2020 §8.17.3.7.2.3)",
                    premier.id
                ),
            );
        }
    }
    // Le texte ne décrit que les générations monoposte (§8.17.3.7.2.3.1 à 8.17.3.7.2.3.3) et
    // « chauffage et ECS » (§8.17.3.7.2.3.4).
    let nb_postes = fonction.iter().filter(|p| **p).count();
    if nb_postes > 1 && fonction != [true, false, true] {
        r.erreur(
            format!("{c}.priorite"),
            Some("alternance".into()),
            "la gestion en alternance s'applique à des générateurs d'un seul poste ou de \
             chauffage et ECS, pas à des générateurs de chauffage et de refroidissement (Th-BCE \
             2020 §8.17.3.7.2.3)",
        );
    }
}

/// Chemin du bâtiment à partir de celui d'une de ses générations.
fn c_batiment(c: &str) -> &str {
    c.rfind(".generations[").map_or(c, |i| &c[..i])
}

/// Réseaux de distribution de CTA hydrauliques d'une zone (fiches 8.11 et 8.12) : « une CTA
/// n'accepte qu'un seul réseau pour chacune de ces fonctions, un réseau pouvant en regrouper
/// plusieurs » (§8.11.3.1) ; un réseau est relié à une seule génération, que désignent donc
/// toutes les fonctions qu'il regroupe.
fn valider_reseaux_cta(
    r: &mut RapportValidation,
    bat: &Batiment,
    c: &str,
    v: &crate::entree::Ventilation,
) {
    let b = |id: Option<&String>| id.cloned();
    let s = &v.systeme;
    let cta = match s {
        SystemeVentilation::Cta(cta) => Some(cta),
        _ => None,
    };
    let chaud: Vec<(&str, Option<String>)> = vec![
        (
            "batterie_prechauffage",
            b(s.batterie_prechauffage().map(|x| &x.generation)),
        ),
        (
            "humidificateur",
            b(s.humidificateur().and_then(|h| h.generation.as_ref())),
        ),
        (
            "echangeur.generation_antigel",
            b(s.echangeur()
                .filter(|e| e.antigel == Some(true))
                .and_then(|e| e.generation_antigel.as_ref())),
        ),
        (
            "generation_batterie_chaude",
            b(cta
                .filter(|x| x.batterie_chaude == Some(true))
                .and_then(|x| x.generation_batterie_chaude.as_ref())),
        ),
    ];
    let froid: Vec<(&str, Option<String>)> = vec![
        (
            "batterie_prerefroidissement",
            b(s.batterie_prerefroidissement().map(|x| &x.generation)),
        ),
        (
            "generation_batterie_froide",
            b(cta
                .filter(|x| x.batterie_froide == Some(true))
                .and_then(|x| x.generation_batterie_froide.as_ref())),
        ),
    ];
    for (nom, reseau, fonction, fonctions) in [
        (
            "reseau_cta_chaud",
            &v.reseau_cta_chaud,
            FonctionReseau::Chauffage,
            &chaud,
        ),
        (
            "reseau_cta_froid",
            &v.reseau_cta_froid,
            FonctionReseau::Refroidissement,
            &froid,
        ),
    ] {
        let Some(h) = reseau else { continue };
        let cr = format!("{c}.{nom}");
        let generations: Vec<&String> = fonctions.iter().filter_map(|(_, g)| g.as_ref()).collect();
        match generations.first() {
            None => r.erreur(
                cr.clone(),
                None,
                "réseau de CTA sans objet : aucune fonction de la centrale (préchauffage, \
                 sécurité antigel, humidificateur à vapeur, batterie chaude ; \
                 prérefroidissement, batterie froide) n'est reliée à une génération (Th-BCE \
                 2020 §8.11.3.1)",
            ),
            Some(premiere) => {
                if generations.iter().any(|g| g != premiere) {
                    r.erreur(
                        cr.clone(),
                        None,
                        "un réseau de CTA est relié à une seule génération : toutes les fonctions \
                         qu'il regroupe désignent la même (Th-BCE 2020 §8.11.3.1)",
                    );
                }
            }
        }
        super::systemes::valider_hydraulique_groupe(r, &cr, h, fonction, bat);
    }
}
