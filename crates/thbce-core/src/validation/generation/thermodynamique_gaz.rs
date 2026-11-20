// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des générateurs thermodynamiques au gaz : machines à absorption (Th-BCE 2020
//! §8.24) et pompes à chaleur à moteur gaz (§8.25).

use crate::chauffage_refroidissement::calculs_generation::Poste;
use crate::chauffage_refroidissement::pac_moteur_gaz;
use crate::chauffage_refroidissement::thermodynamique_gaz::{
    GrilleGaz, PerformancesSaisiesGaz, CH_AIR_EAU_CLASSIQUE, CH_AIR_EAU_HAUTE_TEMPERATURE,
    ECS_AIR_EAU, FR_AIR_EAU,
};
use crate::entree::{
    AbsorptionGazSaisie, FonctionnementBruleur, FonctionnementContinu, FonctionnementMoteurGaz,
    Generation, LimitesPac, ModeAbsorptionGaz, PacMoteurGazSaisie, PerformancesGaz,
    TechnologieChauffageGahp, TechnologieEcsGahp, TechnologiePacMoteurGaz,
    TechnologieRefroidissementGahp, ThermodynamiqueGazSaisi,
};
use crate::validation::RapportValidation;

/// Postes `[chauffage, refroidissement, ECS]` d'un générateur au gaz.
pub(super) fn postes_gaz(t: &ThermodynamiqueGazSaisi) -> [bool; 3] {
    match t {
        ThermodynamiqueGazSaisi::Absorption(a) => match a.mode {
            ModeAbsorptionGaz::Chauffage { .. } => [true, false, false],
            ModeAbsorptionGaz::Refroidissement { .. } => [false, true, false],
            ModeAbsorptionGaz::Ecs { .. } => [false, false, true],
        },
        ThermodynamiqueGazSaisi::MoteurGaz(m) => {
            [m.chauffage.is_some(), m.refroidissement.is_some(), false]
        }
    }
}

/// Fluide aval de chaque mode `[chauffage, refroidissement]` : `Some(true)` sur eau,
/// `Some(false)` sur air (« IdFluide_aval », tableaux 217 et 229).
pub(super) fn fluide_aval_gaz(t: &ThermodynamiqueGazSaisi) -> [Option<bool>; 2] {
    match t {
        ThermodynamiqueGazSaisi::Absorption(a) => match a.mode {
            ModeAbsorptionGaz::Chauffage { .. } => [Some(true), None],
            ModeAbsorptionGaz::Refroidissement { .. } => [None, Some(true)],
            ModeAbsorptionGaz::Ecs { .. } => [None, None],
        },
        ThermodynamiqueGazSaisi::MoteurGaz(m) => {
            let eau = |x: &Option<crate::entree::ModePacMoteurGaz>| {
                x.as_ref()
                    .map(|x| x.technologie == TechnologiePacMoteurGaz::AirExterieurEau)
            };
            [eau(&m.chauffage), eau(&m.refroidissement)]
        }
    }
}

/// Th-BCE 2020 §8.24 et §8.25 : générateur thermodynamique au gaz ; `id` : identifiant du
/// générateur dans la génération `g`.
pub(super) fn valider_thermodynamique_gaz(
    r: &mut RapportValidation,
    c: &str,
    t: &ThermodynamiqueGazSaisi,
    id: &str,
    g: &Generation,
) {
    match t {
        ThermodynamiqueGazSaisi::Absorption(a) => {
            valider_absorption(r, &format!("{c}.absorption"), a, id, g);
        }
        ThermodynamiqueGazSaisi::MoteurGaz(m) => {
            valider_moteur_gaz(r, &format!("{c}.moteur_gaz"), m);
        }
    }
}

fn performances(r: &mut RapportValidation, c: &str, grille: &GrilleGaz, p: &PerformancesGaz) {
    match PerformancesSaisiesGaz::depuis(p) {
        PerformancesSaisiesGaz::Mesurees(points) => {
            if let Err(message) = grille.verifier_mesures(&points) {
                r.erreur(format!("{c}.mesurees"), None, message);
            }
            // La vérification de l'ordre de priorité impose la valeur pivot.
        }
        PerformancesSaisiesGaz::Pivot {
            performance,
            pabs_w,
            paux_w,
        } => {
            let nom = if performance.is_some() {
                "pivot_declare"
            } else {
                "pivot_defaut"
            };
            if let Some(v) = performance {
                r.strictement_positif(format!("{c}.{nom}.performance"), v);
            }
            r.strictement_positif(format!("{c}.{nom}.pabs_kw"), pabs_w);
            if let Some(v) = paux_w {
                r.strictement_positif(format!("{c}.{nom}.paux_w"), v);
            }
        }
    }
}

fn continu(r: &mut RapportValidation, c: &str, f: FonctionnementContinu, sans_objet: bool) {
    if sans_objet {
        r.erreur(
            c.to_owned(),
            None,
            "sans objet en fonctionnement par cycles marche-arrêt (`LRcontmin = 1`)",
        );
    }
    r.plage(format!("{c}.lr_contmin"), f.lr_contmin, 0.0, 1.0);
    if f.lr_contmin <= 0.0 {
        r.erreur(
            format!("{c}.lr_contmin"),
            Some(f.lr_contmin.to_string()),
            "doit être strictement positif",
        );
    }
    r.strictement_positif(format!("{c}.ccp_lr_contmin"), f.ccp_lr_contmin);
}

fn limites(r: &mut RapportValidation, c: &str, l: Option<LimitesPac>) {
    if let Some(l) = l {
        r.plage(format!("{c}.theta_aval_c"), l.theta_aval_c, -50.0, 150.0);
        r.plage(format!("{c}.theta_amont_c"), l.theta_amont_c, -50.0, 150.0);
    }
}

fn valider_absorption(
    r: &mut RapportValidation,
    c: &str,
    a: &AbsorptionGazSaisie,
    id: &str,
    g: &Generation,
) {
    let hors = "machine à absorption sur eau glycolée, sur eau ou sur eau de tour (sources \
                amont des fiches 8.26 et 8.27) ; seules les machines sur air extérieur sont \
                calculées (Th-BCE 2020 §8.24.3, lecture Q135)";
    let grille = match a.mode {
        ModeAbsorptionGaz::Chauffage { technologie, .. } => match technologie {
            TechnologieChauffageGahp::AirEauClassique => Some(&CH_AIR_EAU_CLASSIQUE),
            TechnologieChauffageGahp::AirEauHauteTemperature => Some(&CH_AIR_EAU_HAUTE_TEMPERATURE),
            _ => None,
        },
        ModeAbsorptionGaz::Refroidissement { technologie, .. } => {
            (technologie == TechnologieRefroidissementGahp::AirEau).then_some(&FR_AIR_EAU)
        }
        ModeAbsorptionGaz::Ecs { technologie } => {
            (technologie == TechnologieEcsGahp::AirEau).then_some(&ECS_AIR_EAU)
        }
    };
    let Some(grille) = grille else {
        r.hors_perimetre(format!("{c}.mode"), None, hors);
        return;
    };
    performances(r, &format!("{c}.performances"), grille, &a.performances);
    limites(r, &format!("{c}.limites"), a.limites);
    if let Some(f) = a.fonctionnement_continu {
        continu(
            r,
            &format!("{c}.fonctionnement_continu"),
            f,
            a.bruleur != Some(FonctionnementBruleur::Modulant),
        );
    }
    for (nom, v) in [("p_aux_0", a.p_aux_0), ("pertes_40c", a.pertes_40c)] {
        if let Some(v) = v {
            r.positif(format!("{c}.{nom}.valeur"), v.valeur);
        }
    }
    if let Some(v) = a.rdt_comb_pivot {
        r.plage(format!("{c}.rdt_comb_pivot.valeur"), v.valeur, 0.0, 1.2);
        if v.valeur <= 0.0 {
            r.erreur(
                format!("{c}.rdt_comb_pivot.valeur"),
                Some(v.valeur.to_string()),
                "doit être strictement positif",
            );
        }
    }
    // §8.24.1 : en ECS, la machine comporte obligatoirement un ballon de stockage.
    if matches!(a.mode, ModeAbsorptionGaz::Ecs { .. })
        && !g.stockages.iter().any(|s| s.base.generateur == id)
    {
        r.erreur(
            format!("{c}.mode.ecs"),
            None,
            "une machine à absorption d'ECS est le générateur de base d'un assemblage avec \
             stockage (`stockages`, Th-BCE 2020 §8.24.1)",
        );
    }
}

fn valider_moteur_gaz(r: &mut RapportValidation, c: &str, m: &PacMoteurGazSaisie) {
    if m.chauffage.is_none() && m.refroidissement.is_none() {
        r.erreur(
            c.to_owned(),
            None,
            "au moins un mode requis : `chauffage` ou `refroidissement` (Th-BCE 2020 §8.25.2)",
        );
    }
    if m.production_ecs_indirecte.is_some() {
        r.hors_perimetre(
            format!("{c}.production_ecs_indirecte"),
            None,
            "production indirecte d'ECS par récupération de chaleur sur le moteur d'une pompe \
             à chaleur à moteur gaz (Th-BCE 2020 §8.25.3.4.5, lecture Q144)",
        );
    }
    for (nom, poste, x) in [
        ("chauffage", Poste::Chauffage, &m.chauffage),
        (
            "refroidissement",
            Poste::Refroidissement,
            &m.refroidissement,
        ),
    ] {
        let Some(x) = x else { continue };
        let (grille, _) = pac_moteur_gaz::grille(x.technologie, poste);
        performances(
            r,
            &format!("{c}.{nom}.performances"),
            grille,
            &x.performances,
        );
        if let Some(v) = x.p_aux_uint_w {
            r.positif(format!("{c}.{nom}.p_aux_uint_w"), v);
        }
    }
    if let Some(f) = m.fonctionnement_continu {
        continu(
            r,
            &format!("{c}.fonctionnement_continu"),
            f,
            m.moteur != Some(FonctionnementMoteurGaz::VitesseVariable),
        );
    }
    if let Some(v) = m.p_aux_0_w {
        r.positif(format!("{c}.p_aux_0_w"), v);
    }
}
