// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des générateurs thermodynamiques à compression électrique (Th-BCE 2020 §8.23)
//! et de leurs sources amont (§8.26).

use crate::chauffage_refroidissement::thermodynamique_elec::{
    fonctionnement_continu, grille_chauffage, grille_ecs, grille_refroidissement,
    performances_saisies, taux_auxiliaires, FonctionnementCompresseur, Grille, PerformancesSaisies,
};
use crate::entree::{
    Batiment, FonctionnementCompresseurSaisi, FonctionnementContinu, PerformancesPac,
    SourceAmontSaisie, TechnologieEcsPac, ThermodynamiqueElectriqueSaisi,
};
use crate::validation::RapportValidation;

/// Th-BCE 2020 §8.23 et §8.26 : générateur thermodynamique à compression électrique.
pub(super) fn valider_thermodynamique(
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
        r.hors_perimetre(
            format!("{c}.ecs"),
            None,
            "générateur produisant du froid et de l'ECS (triple service, Th-BCE 2020 §8.23.3.2, \
             tableau 149)",
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
            r.hors_perimetre(
                format!("{cm}.technologie"),
                None,
                "technologie de pompe à chaleur autre que sur air (seules les machines sur air \
                 extérieur en chauffage et en refroidissement, sur air extérieur, air extrait ou \
                 air ambiant en ECS, sont calculées ; Th-BCE 2020 §8.23.3.2, tableaux 145 et 147)",
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
            PerformancesSaisies::PivotDefaut { pabs_w } => {
                r.strictement_positif(format!("{cm}.performances.pivot_defaut.pabs_kw"), pabs_w);
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
