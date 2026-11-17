// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des générations (Th-BCE 2020 §8.14, §8.17 et §8.18) : générateurs, gestion des
//! priorités, emplacement, cohérence du montage (§8.17.3.1) et génération associée à la
//! sécurité antigel d'un double flux.

use crate::chauffage_refroidissement::thermodynamique_elec::{
    fonctionnement_continu, grille_chauffage, grille_ecs, grille_refroidissement,
    performances_saisies, taux_auxiliaires, FonctionnementCompresseur, Grille, PerformancesSaisies,
};
use crate::ecs::gestion_regulation_ballon::THETA_C_ECS_C;
use crate::ecs::idcet;
use crate::entree::{
    Batiment, ChaudiereSaisie, FonctionChaleur, FonctionEffetJoule, FonctionReseau,
    FonctionnementCompresseurSaisi, FonctionnementContinu, Generation,
    GestionTemperatureGeneration, NatureGenerateur, PerformancesPac, PoeleSaisi,
    PositionGeneration, ReseauChaleurFroidSaisi, ReseauIntergroupes, SourceAmontSaisie,
    SystemeVentilation, TechnologieChauffagePac, TechnologieRefroidissementPac,
    ThermodynamiqueElectriqueSaisi, TypePriorite,
};
use crate::entree::{PertesBallon, TechnologieEcsPac};

use super::RapportValidation;

const HORS_VERSION: &str = "non pris en charge dans cette version du moteur (docs/couverture.md)";

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
    if g.generateurs.is_empty() && g.chauffe_eau_thermodynamiques.is_empty() {
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
        })
    };
    let avec_stockage = !g.stockages.is_empty() || !g.chauffe_eau_thermodynamiques.is_empty();
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
        let assures = postes_assures(&x.nature);
        for (p, a) in assures.into_iter().enumerate() {
            nb[p] += usize::from(a && !(p == 2 && pilote(&x.id)));
        }
        if assures[2]
            && matches!(x.nature, NatureGenerateur::ThermodynamiqueElectrique(_))
            && !g.stockages.iter().any(|s| s.base.generateur == x.id)
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
    let assure = |p: usize| nb[p] > 0 || (p == 2 && avec_stockage);
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

/// Th-BCE 2020 §8.23 et §8.26 : générateur thermodynamique à compression électrique.
fn valider_thermodynamique(
    r: &mut RapportValidation,
    c: &str,
    t: &ThermodynamiqueElectriqueSaisi,
    bat: &Batiment,
) {
    if t.chauffage.is_none() && t.refroidissement.is_none() && t.ecs.is_none() {
        r.erreur(
            c.to_owned(),
            None,
            "au moins un mode requis : `chauffage`, `refroidissement` ou `ecs` (Th-BCE 2020 \
             §8.23.3.2)",
        );
    }
    if t.refroidissement.is_some() && t.ecs.is_some() {
        r.erreur(
            format!("{c}.ecs"),
            None,
            format!(
                "générateur produisant du froid et de l'ECS (triple service, Th-BCE 2020 \
                 §8.23.3.2, tableau 149) {HORS_VERSION}"
            ),
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
    let (taux, _) = taux_auxiliaires(t.taux_auxiliaires, t.chauffage.is_some() || t.ecs.is_some());
    let mut mode = |nom: &str,
                    grille: Option<&'static Grille>,
                    source_attendue: &str,
                    performances: &PerformancesPac,
                    continu: Option<FonctionnementContinu>| {
        let cm = format!("{c}.{nom}");
        let Some(grille) = grille else {
            r.erreur(
                format!("{cm}.technologie"),
                None,
                format!(
                    "technologie {HORS_VERSION} : seules les machines sur air (air extérieur \
                     en chauffage et en refroidissement ; air extérieur, air extrait ou air \
                     ambiant en ECS) sont prises en charge (Th-BCE 2020 §8.23.3.2, tableaux \
                     145 et 147)"
                ),
            );
            return;
        };
        // La technologie fixe la nature de la source amont.
        if source != source_attendue {
            r.erreur(
                format!("{c}.source_amont"),
                Some(source.to_owned()),
                format!(
                    "source amont incohérente avec la technologie du mode {nom}, qui a pour \
                     source `{source_attendue}` (Th-BCE 2020 §8.26.3.3.1)"
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
            "air_exterieur",
            &m.performances,
            m.fonctionnement_continu,
        );
    }
    if let Some(m) = &t.refroidissement {
        mode(
            "refroidissement",
            grille_refroidissement(m.technologie),
            "air_exterieur",
            &m.performances,
            m.fonctionnement_continu,
        );
    }
    if let Some(m) = &t.ecs {
        mode(
            "ecs",
            grille_ecs(m.technologie),
            match m.technologie {
                TechnologieEcsPac::AirExtraitEau => "air_extrait",
                TechnologieEcsPac::AirAmbiantEau => "air_ambiant",
                _ => "air_exterieur",
            },
            &m.performances,
            None,
        );
    }
}

/// Th-BCE 2020 §9.9, §9.10, §9.14 et §9.22 : assemblages avec ballon de stockage et
/// chauffe-eau thermodynamiques d'une génération.
fn valider_stockages(r: &mut RapportValidation, c: &str, g: &Generation, bat: &Batiment) {
    r.ids_uniques(
        &format!("{c}.stockages"),
        g.stockages
            .iter()
            .map(|s| &s.id)
            .chain(g.chauffe_eau_thermodynamiques.iter().map(|x| &x.id)),
    );
    let zone_valide = |r: &mut RapportValidation, chemin: String, z: u8| {
        if !(1..=4).contains(&z) {
            r.erreur(
                chemin,
                Some(z.to_string()),
                "numéro de zone du ballon de 1 (bas) à 4 (haut) (Th-BCE 2020 §9.9.2)",
            );
        }
    };
    let mut bases: Vec<&str> = Vec::new();
    for (k, s) in g.stockages.iter().enumerate() {
        let cs = format!("{c}.stockages[{k}]");
        if s.nb_assemblages == 0 {
            r.erreur(
                format!("{cs}.nb_assemblages"),
                Some("0".into()),
                "au moins un assemblage (Th-BCE 2020 §9.14.2)",
            );
        }
        r.strictement_positif(format!("{cs}.ballon.v_tot_l"), s.ballon.v_tot_l);
        if let PertesBallon::Certifie { ua_s_w_k } | PertesBallon::Justifie { ua_s_w_k } =
            s.ballon.pertes
        {
            r.strictement_positif(format!("{cs}.ballon.pertes.ua_s_w_k"), ua_s_w_k);
        }
        // Éq. (1948) : consigne conventionnelle de 55 °C, sous la température maximale.
        r.plage(
            format!("{cs}.ballon.theta_max_c"),
            s.ballon.theta_max_c,
            THETA_C_ECS_C,
            100.0,
        );
        zone_valide(r, format!("{cs}.base.z_reg"), s.base.z_reg);
        if let Some(d) = s.base.delta_theta_k {
            r.plage(format!("{cs}.base.delta_theta_k"), d, 0.0, 20.0);
        }
        match g.generateurs.iter().find(|x| x.id == s.base.generateur) {
            None => r.erreur(
                format!("{cs}.base.generateur"),
                Some(s.base.generateur.clone()),
                "générateur inconnu de la génération",
            ),
            Some(x) => {
                if !postes_assures(&x.nature)[2] {
                    r.erreur(
                        format!("{cs}.base.generateur"),
                        Some(x.id.clone()),
                        "le générateur de base d'un ballon d'ECS assure l'ECS : générateur à \
                         effet Joule de fonction `ecs`, chaudière ou réseau de chaleur assurant \
                         l'ECS, ou générateur thermodynamique pourvu d'un mode `ecs`",
                    );
                }
                if x.rdim != s.nb_assemblages {
                    r.erreur(
                        format!("{cs}.nb_assemblages"),
                        Some(s.nb_assemblages.to_string()),
                        "le nombre d'assemblages identiques est le nombre de générateurs de \
                         base identiques (`rdim` du générateur désigné)",
                    );
                }
                let thermo = matches!(x.nature, NatureGenerateur::ThermodynamiqueElectrique(_));
                match s.base.h_rel_ech {
                    None if thermo => r.erreur(
                        format!("{cs}.base.h_rel_ech"),
                        None,
                        "hauteur relative de l'échangeur requise pour un générateur \
                         thermodynamique (Th-BCE 2020 §9.9.3.2.4, éq. 1781 à 1785)",
                    ),
                    Some(h) => r.plage(format!("{cs}.base.h_rel_ech"), h, 0.0, 1.0),
                    None => {}
                }
                if bases.contains(&x.id.as_str()) {
                    r.erreur(
                        format!("{cs}.base.generateur"),
                        Some(x.id.clone()),
                        "générateur déjà employé par un autre assemblage",
                    );
                }
                bases.push(&x.id);
            }
        }
        if let Some(a) = &s.appoint {
            zone_valide(r, format!("{cs}.appoint.z_ap"), a.z_ap);
            zone_valide(r, format!("{cs}.appoint.z_reg"), a.z_reg);
            if let Some(f) = a.f_aux {
                if !(f > 0.0 && f < 1.0) {
                    r.erreur(
                        format!("{cs}.appoint.f_aux"),
                        Some(f.to_string()),
                        "« faux » strictement compris entre 0 et 1 (Th-BCE 2020 §9.9.3.1)",
                    );
                }
            }
            if let Some(d) = a.delta_theta_k {
                r.plage(format!("{cs}.appoint.delta_theta_k"), d, 0.0, 20.0);
            }
            let ok = g.generateurs.iter().any(|x| {
                x.id == a.generateur
                    && x.id != s.base.generateur
                    && matches!(&x.nature, NatureGenerateur::EffetJoule(j)
                        if j.fonction == FonctionEffetJoule::Ecs)
            });
            if !ok || bases.contains(&a.generateur.as_str()) {
                r.erreur(
                    format!("{cs}.appoint.generateur"),
                    Some(a.generateur.clone()),
                    "l'appoint intégré d'un ballon est un générateur à effet Joule de fonction \
                     `ecs` de la génération, distinct du générateur de base et propre à \
                     l'assemblage",
                );
            }
            bases.push(&a.generateur);
        }
    }
    for (k, x) in g.chauffe_eau_thermodynamiques.iter().enumerate() {
        let cc = format!("{c}.chauffe_eau_thermodynamiques[{k}]");
        if g.generateurs.iter().any(|y| y.id == x.id) {
            r.erreur(
                format!("{cc}.id"),
                Some(x.id.clone()),
                "identifiant déjà porté par un générateur de la génération",
            );
        }
        if x.nb_assemblages == 0 {
            r.erreur(
                format!("{cc}.nb_assemblages"),
                Some("0".into()),
                "au moins un chauffe-eau",
            );
        }
        r.strictement_positif(format!("{cc}.v_tot_l"), x.v_tot_l);
        r.plage(
            format!("{cc}.theta_max_c"),
            x.theta_max_c,
            THETA_C_ECS_C,
            100.0,
        );
        let e = &x.essai;
        r.strictement_positif(format!("{cc}.essai.t_h_h"), e.t_h_h);
        r.plage(format!("{cc}.essai.t_h_h"), e.t_h_h, 0.0, 48.0);
        r.strictement_positif(format!("{cc}.essai.p_es_w"), e.p_es_w);
        r.strictement_positif(format!("{cc}.essai.cop_dhw"), e.cop_dhw);
        r.plage(format!("{cc}.essai.theta_wh_c"), e.theta_wh_c, 40.0, 90.0);
        let mut references = true;
        match &x.source_amont {
            SourceAmontSaisie::AirExterieur { .. } => {}
            SourceAmontSaisie::AirAmbiant { espace_tampon, .. } => {
                if !bat.espaces_tampons.iter().any(|t| t.id == *espace_tampon) {
                    references = false;
                    r.erreur(
                        format!("{cc}.source_amont.air_ambiant.espace_tampon"),
                        Some(espace_tampon.clone()),
                        "espace tampon inconnu du bâtiment",
                    );
                }
            }
            SourceAmontSaisie::AirExtrait { zone, .. } => {
                if !bat.zones.iter().any(|z| z.id == *zone) {
                    references = false;
                    r.erreur(
                        format!("{cc}.source_amont.air_extrait.zone"),
                        Some(zone.clone()),
                        "zone inconnue du bâtiment",
                    );
                }
            }
        }
        let valeurs = x.v_tot_l > 0.0
            && e.t_h_h > 0.0
            && e.t_h_h <= 48.0
            && e.p_es_w > 0.0
            && e.cop_dhw > 0.0
            && (40.0..=90.0).contains(&e.theta_wh_c);
        if !(references && valeurs) {
            continue;
        }
        // §9.22.3.4 : identification du triplet (UAs, COP pivot, Pabs pivot).
        match idcet::identifier(&idcet::essai_saisi(x)) {
            None => r.erreur(
                format!("{cc}.essai"),
                None,
                "aucun triplet (UAs, COP pivot, Pabs pivot) ne permet au modèle Th-BCE de \
                 reproduire cet essai (Th-BCE 2020 §9.22.3.4) : résultats d'essai incohérents",
            ),
            Some(i) if i.epsilon >= idcet::EPSILON_MAX => r.avertissement(
                format!("{cc}.essai"),
                Some(format!("ε = {:.4}", i.epsilon)),
                format!(
                    "l'essai n'est reproduit par le modèle Th-BCE qu'à ε = {:.4} près, \
                     au-dessus du seuil de 10⁻⁴ de l'éq. (2249) : th = {:.2} h, Pes = {:.1} W, \
                     COPDHW = {:.3} avec le triplet retenu (Th-BCE 2020 §9.22.3.4, lecture Q48)",
                    i.epsilon,
                    i.essai_reproduit.t_h_h,
                    i.essai_reproduit.p_es_w,
                    i.essai_reproduit.cop_dhw
                ),
            ),
            Some(_) => {}
        }
    }
}

/// Th-BCE 2020 §8.19 : chaudière au gaz, au fioul ou au bois.
fn valider_chaudiere(r: &mut RapportValidation, c: &str, x: &ChaudiereSaisie) {
    use crate::entree::TypeChaudiereSaisi as T;
    r.plage(format!("{c}.pn_gen_kw"), x.pn_gen_kw, 0.0, f64::MAX);
    r.strictement_positif(format!("{c}.pn_gen_kw"), x.pn_gen_kw);
    let bois = x.type_chaudiere == T::Bois;
    if bois && x.combustible_bois.is_none() {
        r.erreur(
            format!("{c}.combustible_bois"),
            None,
            "« TypeCombustibleBois » requis pour une chaudière au bois (Th-BCE 2020 §8.19.2)",
        );
    }
    if !bois && x.combustible_bois.is_some() {
        r.erreur(
            format!("{c}.combustible_bois"),
            None,
            "sans objet : le combustible bois ne concerne que les chaudières au bois",
        );
    }
    if !bois && x.gpl && matches!(x.type_chaudiere, T::FioulStandard | T::FioulCondensation) {
        r.erreur(
            format!("{c}.gpl"),
            None,
            "sans objet : le gaz de pétrole liquéfié ne concerne que les chaudières au gaz",
        );
    }
    if bois && x.gpl {
        r.erreur(
            format!("{c}.gpl"),
            None,
            "sans objet : le gaz de pétrole liquéfié ne concerne que les chaudières au gaz",
        );
    }
    let aux = &x.auxiliaires;
    if bois
        && x.alimentation_bois.is_none()
        && (aux.w_aux_nom_w.is_none() || aux.w_aux_int_w.is_none())
    {
        r.erreur(
            format!("{c}.alimentation_bois"),
            None,
            "« Alim_Chaudière_Bois » requis pour une chaudière au bois dont les auxiliaires sont \
             par défaut (Th-BCE 2020 §8.19.3.1.4, tableau 119)",
        );
    }
    if let Some(rd) = &x.rendements {
        for (nom, v) in [("r_pn_pct", rd.r_pn_pct), ("r_pint_pct", rd.r_pint_pct)] {
            if !(v > 0.0 && v < 100.0) {
                r.erreur(
                    format!("{c}.rendements.{nom}"),
                    Some(v.to_string()),
                    "Générateur à combustion : la saisie des performances du générateur à \
                     combustion conduit à des rendements sur PCS égaux ou supérieurs à l'unité \
                     (Th-BCE 2020 §8.19.3.1) ; rendement strictement positif",
                );
            }
        }
        if let Some(p) = rd.p_int_kw {
            r.strictement_positif(format!("{c}.rendements.p_int_kw"), p);
        }
    }
    for (nom, v) in [
        ("auxiliaires.w_aux_nom_w", aux.w_aux_nom_w),
        ("auxiliaires.w_aux_int_w", aux.w_aux_int_w),
        ("auxiliaires.w_veille_w", aux.w_veille_w),
        ("q_po_30_w", x.q_po_30_w),
    ] {
        if let Some(v) = v {
            r.plage(format!("{c}.{nom}"), v, 0.0, f64::MAX);
        }
    }
}

/// Th-BCE 2020 §8.22 : poêle ou insert.
fn valider_poele(r: &mut RapportValidation, c: &str, x: &PoeleSaisi) {
    r.strictement_positif(format!("{c}.pn_gen_kw"), x.pn_gen_kw);
    r.plage(format!("{c}.pn_gen_kw"), x.pn_gen_kw, 0.0, 100.0);
    r.strictement_positif(format!("{c}.eta_pct"), x.eta_pct);
    r.plage(format!("{c}.eta_pct"), x.eta_pct, 0.0, 100.0);
    r.plage(format!("{c}.p_aux_vent_w"), x.p_aux_vent_w, 0.0, f64::MAX);
}

/// Th-BCE 2020 §8.28 : sous-station d'un réseau de chaleur ou de froid.
fn valider_reseau(r: &mut RapportValidation, c: &str, x: &ReseauChaleurFroidSaisi) {
    let (id, p_ess, froid) = match x {
        ReseauChaleurFroidSaisi::Chaleur {
            identifiant_reseau,
            classe_isolation_secondaire,
            p_ess_kw,
            ..
        } => {
            if !(1..=4).contains(classe_isolation_secondaire) {
                r.erreur(
                    format!("{c}.chaleur.classe_isolation_secondaire"),
                    Some(classe_isolation_secondaire.to_string()),
                    "classe d'isolation du réseau secondaire de 1 à 4 (Th-BCE 2020 §8.28.3.1, \
                     tableau 247)",
                );
            }
            (identifiant_reseau, *p_ess_kw, false)
        }
        ReseauChaleurFroidSaisi::Froid {
            identifiant_reseau,
            p_ess_kw,
        } => (identifiant_reseau, *p_ess_kw, true),
    };
    r.strictement_positif(format!("{c}.p_ess_kw"), p_ess);
    if let Some(id) = id {
        let lettre = if froid { 'F' } else { 'C' };
        let bon =
            id.len() == 5 && id[..4].chars().all(|ch| ch.is_ascii_digit()) && id.ends_with(lettre);
        if bon && thbce_data::reseaux_chaleur::reseau(id).is_none() {
            r.erreur(
                format!("{c}.identifiant_reseau"),
                Some(id.clone()),
                "identifiant absent de l'annexe 7 de l'arrêté du 15 septembre 2006 : omettre le \
                 champ pour employer les lignes « autres réseaux »",
            );
        }
        if !bon {
            r.erreur(
                format!("{c}.identifiant_reseau"),
                Some(id.clone()),
                format!(
                    "identifiant de réseau de {} : quatre chiffres suivis de « {lettre} » \
                     (annexe 7 de l'arrêté du 15 septembre 2006)",
                    if froid { "froid" } else { "chaleur" }
                ),
            );
        }
    }
}
