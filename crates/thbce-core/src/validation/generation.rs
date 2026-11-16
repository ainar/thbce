// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des générations (Th-BCE 2020 §8.14, §8.17 et §8.18) : générateurs, gestion des
//! priorités, emplacement, cohérence du montage (§8.17.3.1) et génération associée à la
//! sécurité antigel d'un double flux.

use crate::chauffage_refroidissement::thermodynamique_elec::{
    fonctionnement_continu, grille_chauffage, grille_refroidissement, performances_saisies,
    taux_auxiliaires, FonctionnementCompresseur, Grille, PerformancesSaisies,
};
use crate::entree::{
    Batiment, FonctionEffetJoule, FonctionReseau, FonctionnementCompresseurSaisi,
    FonctionnementContinu, Generation, GestionTemperatureGeneration, NatureGenerateur,
    PerformancesPac, PositionGeneration, ReseauIntergroupes, SourceAmontSaisie, SystemeVentilation,
    TechnologieChauffagePac, TechnologieRefroidissementPac, ThermodynamiqueElectriqueSaisi,
    TypePriorite,
};

use super::RapportValidation;

const HORS_VERSION: &str = "non pris en charge dans cette version du moteur (docs/couverture.md)";

/// Postes qu'un générateur saisi peut assurer : `[chauffage, refroidissement, ECS]`.
pub(super) fn postes_assures(n: &NatureGenerateur) -> [bool; 3] {
    match n {
        NatureGenerateur::EffetJoule(j) => match j.fonction {
            FonctionEffetJoule::Chauffage => [true, false, false],
            FonctionEffetJoule::Ecs => [false, false, true],
        },
        NatureGenerateur::ThermodynamiqueElectrique(t) => {
            [t.chauffage.is_some(), t.refroidissement.is_some(), false]
        }
        _ => [false; 3],
    }
}

/// Valide les générations d'un bâtiment.
pub(super) fn valider_generations(r: &mut RapportValidation, cb: &str, bat: &Batiment) {
    for (i, g) in bat.generations.iter().enumerate() {
        valider_generation(r, &format!("{cb}.generations[{i}]"), g, bat);
    }
    // Sécurité antigel des doubles flux : génération désignée (§8.11.3, éq. 939).
    for (iz, z) in bat.zones.iter().enumerate() {
        let Some(SystemeVentilation::DoubleFlux(d)) = z.ventilation.as_ref().map(|v| &v.systeme)
        else {
            continue;
        };
        let Some(e) = &d.echangeur else { continue };
        let c = format!(
            "{cb}.zones[{iz}].ventilation.systeme.double_flux.echangeur.generation_antigel"
        );
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
            (Some(id), true) => match bat.generations.iter().find(|g| g.id == *id) {
                None => r.erreur(c, Some(id.clone()), "génération inconnue du bâtiment"),
                Some(g) => {
                    if !g.generateurs.iter().any(|x| postes_assures(&x.nature)[0]) {
                        r.erreur(
                            c,
                            Some(id.clone()),
                            "la génération associée à la sécurité antigel n'a pas de générateur \
                             de chauffage (Th-BCE 2020 §8.17.3.1)",
                        );
                    }
                }
            },
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
        r.erreur(
            format!("{c}.priorite"),
            Some("alternance".into()),
            format!(
                "gestion des générateurs en alternance (Th-BCE 2020 §8.17.3.7.2.3) {HORS_VERSION}"
            ),
        );
    }
    if g.generateurs.is_empty() {
        r.erreur(
            format!("{c}.generateurs"),
            None,
            "au moins un générateur requis : une génération est un ensemble de générateurs \
             (Th-BCE 2020 §8.14.1)",
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
            autre => {
                let (nom, quoi) = match autre {
                    NatureGenerateur::Chaudiere => (
                        "chaudiere",
                        "chaudière au gaz, au fioul ou au bois (§8.19, lot 6d)",
                    ),
                    NatureGenerateur::PoeleOuInsert => {
                        ("poele_ou_insert", "poêle ou insert (§8.22, lot 6d)")
                    }
                    NatureGenerateur::ReseauDeChaleurOuDeFroid => (
                        "reseau_de_chaleur_ou_de_froid",
                        "réseau de chaleur ou de froid (§8.28, lot 6d)",
                    ),
                    NatureGenerateur::AutreGenerateurACombustion => (
                        "autre_generateur_a_combustion",
                        "autre générateur à combustion (§8.20, après v1)",
                    ),
                    NatureGenerateur::ThermodynamiqueGaz => (
                        "thermodynamique_gaz",
                        "générateur thermodynamique au gaz, à absorption ou à moteur (§8.24 et \
                         §8.25, après v1)",
                    ),
                    NatureGenerateur::BoucleDEauOuDrv => (
                        "boucle_d_eau_ou_drv",
                        "pompe à chaleur sur boucle d'eau, thermo-frigo-pompe ou générateur DRV \
                         (§8.15 et §8.16, après v1)",
                    ),
                    NatureGenerateur::Cogeneration => {
                        ("cogeneration", "système de cogénération (§8.21, après v1)")
                    }
                    _ => ("pac_hybride", "pompe à chaleur hybride (après v1)"),
                };
                r.erreur(
                    format!("{cg}.nature"),
                    Some(nom.into()),
                    format!("{quoi} : {HORS_VERSION}"),
                );
            }
        }
        for (p, a) in postes_assures(&x.nature).into_iter().enumerate() {
            nb[p] += usize::from(a);
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
    let assure = |p: usize| nb[p] > 0;
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
        && !g.generateurs.is_empty()
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

/// Th-BCE 2020 §8.23 et §8.26 : générateur thermodynamique à compression électrique.
fn valider_thermodynamique(
    r: &mut RapportValidation,
    c: &str,
    t: &ThermodynamiqueElectriqueSaisi,
    bat: &Batiment,
) {
    if t.chauffage.is_none() && t.refroidissement.is_none() {
        r.erreur(
            c.to_owned(),
            None,
            "au moins un mode requis : `chauffage` ou `refroidissement` (Th-BCE 2020 §8.23.3.2)",
        );
    }
    let (source, p_vent) = match &t.source_amont {
        SourceAmontSaisie::AirExterieur { p_vent_gaine_w } => ("air_exterieur", *p_vent_gaine_w),
        SourceAmontSaisie::AirAmbiant {
            espace_tampon,
            p_vent_gaine_w,
        } => {
            if !bat.espaces_tampons.iter().any(|e| e.id == *espace_tampon) {
                r.erreur(
                    format!("{c}.source_amont.air_ambiant.espace_tampon"),
                    Some(espace_tampon.clone()),
                    "espace tampon inconnu du bâtiment",
                );
            }
            ("air_ambiant", *p_vent_gaine_w)
        }
        SourceAmontSaisie::AirExtrait {
            zone,
            p_vent_gaine_w,
            ..
        } => {
            if !bat.zones.iter().any(|z| z.id == *zone) {
                r.erreur(
                    format!("{c}.source_amont.air_extrait.zone"),
                    Some(zone.clone()),
                    "zone inconnue du bâtiment",
                );
            }
            ("air_extrait", *p_vent_gaine_w)
        }
    };
    if let Some(p) = p_vent {
        r.positif(format!("{c}.source_amont.{source}.p_vent_gaine_w"), p);
    }
    if let Some(taux) = t.taux_auxiliaires {
        r.plage(
            format!("{c}.taux_auxiliaires.valeur"),
            taux.valeur,
            0.0,
            1.0,
        );
    }
    let compresseur = match t.compresseur {
        FonctionnementCompresseurSaisi::PuissanceVariable => {
            FonctionnementCompresseur::PuissanceVariable
        }
        FonctionnementCompresseurSaisi::ToutOuRien => FonctionnementCompresseur::ToutOuRien,
    };
    let (taux, _) = taux_auxiliaires(t.taux_auxiliaires, t.chauffage.is_some());
    let mut mode = |nom: &str,
                    grille: Option<&'static Grille>,
                    performances: &PerformancesPac,
                    continu: Option<FonctionnementContinu>| {
        let cm = format!("{c}.{nom}");
        let Some(grille) = grille else {
            r.erreur(
                format!("{cm}.technologie"),
                None,
                format!(
                    "technologie {HORS_VERSION} : seules les machines sur air extérieur, à eau \
                     ou à air recyclé, sont prises en charge (Th-BCE 2020 §8.23.3.2, tableau 145)"
                ),
            );
            return;
        };
        // Les technologies prises en charge en chauffage et en refroidissement puisent dans
        // l'air extérieur.
        if source != "air_exterieur" {
            r.erreur(
                format!("{c}.source_amont"),
                Some(source.to_owned()),
                format!(
                    "source amont incohérente avec la technologie du mode {nom} : une machine \
                     sur air extérieur a pour source `air_exterieur` (Th-BCE 2020 §8.26.3.3.1)"
                ),
            );
        }
        match performances_saisies(performances) {
            PerformancesSaisies::Mesurees(points) => {
                if let Err(message) = grille.verifier_mesures(&points) {
                    r.erreur(format!("{cm}.performances.mesurees"), None, message);
                } else if !points.iter().any(|p| {
                    (p.theta_aval_c - grille.theta_aval_c[grille.pivot.0]).abs() < 1e-9
                        && (p.theta_amont_c - grille.theta_amont_c[grille.pivot.1]).abs() < 1e-9
                }) {
                    r.erreur(
                        format!("{cm}.performances.mesurees"),
                        None,
                        "la valeur pivot de la technologie est requise (Th-BCE 2020 §8.23.3.3)",
                    );
                }
            }
            PerformancesSaisies::PivotDeclare {
                performance,
                pabs_w,
            } => {
                r.strictement_positif(
                    format!("{cm}.performances.pivot_declare.performance"),
                    performance,
                );
                r.strictement_positif(format!("{cm}.performances.pivot_declare.pabs_kw"), pabs_w);
            }
            PerformancesSaisies::PivotParDefaut { pabs_w } => {
                r.strictement_positif(
                    format!("{cm}.performances.pivot_par_defaut.pabs_kw"),
                    pabs_w,
                );
            }
        }
        if let Some(f) = continu {
            if compresseur == FonctionnementCompresseur::ToutOuRien {
                r.erreur(
                    format!("{cm}.fonctionnement_continu"),
                    None,
                    "sans objet pour un compresseur tout ou rien (Th-BCE 2020 §8.23.3.6)",
                );
            }
            r.plage(
                format!("{cm}.fonctionnement_continu.lr_contmin"),
                f.lr_contmin,
                0.0,
                1.0,
            );
            r.strictement_positif(
                format!("{cm}.fonctionnement_continu.ccp_lr_contmin"),
                f.ccp_lr_contmin,
            );
        }
        // Éq. (1282) : cohérence des données de charge partielle.
        let (lr_contmin, ccp, _) = fonctionnement_continu(compresseur, continu);
        if compresseur == FonctionnementCompresseur::PuissanceVariable
            && (lr_contmin * 0.3 < ccp * taux || lr_contmin >= 1.0 || lr_contmin <= 0.0)
        {
            r.erreur(
                format!("{cm}.fonctionnement_continu"),
                None,
                "Générateur thermodynamique : les données caractérisant la charge partielle du \
                 générateur thermodynamique sont en dehors des limites tolérées. Assurez-vous \
                 de la cohérence des paramètres LRcontmin, CCP_LRcontmin, Pabs et Taux (Th-BCE \
                 2020 §8.23.3.6, éq. 1282 ; après correction de statut, LRcontmin est \
                 strictement compris entre 0 et 1)",
            );
        }
    };
    if let Some(m) = &t.chauffage {
        mode(
            "chauffage",
            grille_chauffage(m.technologie),
            &m.performances,
            m.fonctionnement_continu,
        );
    }
    if let Some(m) = &t.refroidissement {
        mode(
            "refroidissement",
            grille_refroidissement(m.technologie),
            &m.performances,
            m.fonctionnement_continu,
        );
    }
}
