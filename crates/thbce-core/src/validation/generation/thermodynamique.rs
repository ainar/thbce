// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des générateurs thermodynamiques à compression électrique (Th-BCE 2020 §8.23)
//! et de leurs sources amont (§8.26).

use crate::chauffage_refroidissement::thermodynamique_elec::{
    fonctionnement_continu, grille_chauffage, grille_ecs, grille_refroidissement,
    performances_saisies, taux_auxiliaires, FonctionnementCompresseur, Grille, PerformancesSaisies,
};
use crate::entree::{
    Batiment, FonctionnementCompresseurSaisi, FonctionnementContinu, PerformancesPac,
    PompesCaptage, SourceAmontSaisie, TechnologieChauffagePac, TechnologieEcsPac,
    TechnologieRefroidissementPac, TemperatureSinusoidale, ThermodynamiqueElectriqueSaisi,
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
    let source = valider_source_amont(r, c, &t.source_amont, bat);
    if source == "tour_de_refroidissement" && (t.chauffage.is_some() || t.ecs.is_some()) {
        r.erreur(
            format!("{c}.source_amont"),
            Some(source.to_owned()),
            "les tours de refroidissement sont utilisées en production de froid seul (Th-BCE \
             2020 §8.26.3.4.3)",
        );
    }
    valider_combinaison(r, c, t);
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
                    sources_admises: &[&str],
                    performances: &PerformancesPac,
                    continu: Option<FonctionnementContinu>| {
        let cm = format!("{c}.{nom}");
        let Some(grille) = grille else {
            r.hors_perimetre(
                format!("{cm}.technologie"),
                None,
                "technologie de pompe à chaleur air extrait / air neuf (Th-BCE 2020 §8.23.3.2, \
                 tableau 145, et §8.26.3.3.1 : non prise en charge par cette version)",
            );
            return;
        };
        // La technologie fixe la nature de la source amont (lecture Q145).
        if !sources_admises.contains(&source) {
            r.erreur(
                format!("{c}.source_amont"),
                Some(source.to_owned()),
                format!(
                    "source amont incohérente avec la technologie du mode {nom}, qui admet : {} \
                     (Th-BCE 2020 §8.23.3.2, §8.26.3.3 et §8.27 ; lecture Q145)",
                    sources_admises.join(", ")
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
            sources_chauffage(m.technologie),
            &m.performances,
            m.fonctionnement_continu,
        );
    }
    if let Some(m) = &t.refroidissement {
        mode(
            "refroidissement",
            grille_refroidissement(m.technologie),
            sources_refroidissement(m.technologie),
            &m.performances,
            m.fonctionnement_continu,
        );
    }
    if let Some(m) = &t.ecs {
        mode(
            "ecs",
            grille_ecs(m.technologie),
            sources_ecs(m.technologie),
            &m.performances,
            None,
        );
    }
}

/// Sources amont admises par une technologie de chauffage (lecture Q145).
fn sources_chauffage(t: TechnologieChauffagePac) -> &'static [&'static str] {
    use TechnologieChauffagePac as T;
    match t {
        T::AirExterieurEau | T::AirExterieurAirRecycle => &["air_exterieur"],
        T::AirExtraitAirNeuf => &["air_extrait"],
        T::EauDeNappeEau | T::EauDeNappeAirRecycle => &["nappe"],
        T::EauGlycoleeEau => &["captage"],
        T::EauDeBoucleAirRecycle => &["boucle_d_eau"],
        T::SolEau | T::SolSol => &["sol"],
    }
}

/// Sources amont admises par une technologie de refroidissement (lecture Q145).
fn sources_refroidissement(t: TechnologieRefroidissementPac) -> &'static [&'static str] {
    use TechnologieRefroidissementPac as T;
    match t {
        T::AirExterieurEau | T::AirExterieurAirRecycle => &["air_exterieur"],
        T::AirExtraitAirNeuf => &["air_extrait"],
        T::EauEau | T::EauAir => &["captage", "nappe", "tour_de_refroidissement", "boucle_d_eau"],
        T::EauDeNappeAirRecycle | T::EauDeNappeEau => &["nappe", "captage"],
    }
}

/// Sources amont admises par une technologie d'ECS (lecture Q145).
fn sources_ecs(t: TechnologieEcsPac) -> &'static [&'static str] {
    use TechnologieEcsPac as T;
    match t {
        T::AirExterieurEau => &["air_exterieur"],
        T::AirExtraitEau => &["air_extrait"],
        T::AirAmbiantEau => &["air_ambiant"],
        T::EauDeNappeEau => &["nappe"],
        T::SolEau => &["sol"],
        T::EauGlycoleeEau => &["captage"],
    }
}

/// Th-BCE 2020 §8.23.3.2, tableaux 146 et 148 : technologies d'un générateur réversible ou
/// double service sur eau ou sur sol. Les combinaisons de technologies sur air ne sont pas
/// contrôlées (comportement antérieur conservé).
fn valider_combinaison(r: &mut RapportValidation, c: &str, t: &ThermodynamiqueElectriqueSaisi) {
    use TechnologieChauffagePac as C;
    use TechnologieEcsPac as E;
    use TechnologieRefroidissementPac as F;
    let sur_air_ch = |x: C| matches!(x, C::AirExterieurEau | C::AirExterieurAirRecycle);
    if let (Some(ch), Some(fr)) = (&t.chauffage, &t.refroidissement) {
        let admis = matches!(
            (ch.technologie, fr.technologie),
            (C::EauGlycoleeEau, F::EauEau)
                | (C::EauDeBoucleAirRecycle, F::EauAir)
                | (C::EauDeNappeAirRecycle, F::EauDeNappeAirRecycle)
                | (C::EauDeNappeEau, F::EauDeNappeEau)
        );
        let sur_air = sur_air_ch(ch.technologie)
            && matches!(
                fr.technologie,
                F::AirExterieurEau | F::AirExterieurAirRecycle
            );
        if !admis && !sur_air {
            r.erreur(
                format!("{c}.refroidissement.technologie"),
                None,
                "combinaison de technologies absente du tableau 146 (machines réversibles sur \
                 eau : eau glycolée / eau, eau de boucle / air, eau de nappe / air, eau de nappe \
                 / eau ; Th-BCE 2020 §8.23.3.2)",
            );
        }
    }
    if let (Some(ch), Some(ecs)) = (&t.chauffage, &t.ecs) {
        let admis = matches!(
            (ch.technologie, ecs.technologie),
            (C::EauDeNappeEau, E::EauDeNappeEau)
                | (C::EauGlycoleeEau, E::EauGlycoleeEau)
                | (C::SolEau, E::SolEau)
        );
        let sur_air = sur_air_ch(ch.technologie) && ecs.technologie == E::AirExterieurEau;
        if !admis && !sur_air {
            r.erreur(
                format!("{c}.ecs.technologie"),
                None,
                "combinaison de technologies absente du tableau 148 (double service sur eau ou \
                 sur sol : eau de nappe / eau, eau glycolée / eau, sol / eau ; Th-BCE 2020 \
                 §8.23.3.2)",
            );
        }
    }
}

/// Th-BCE 2020 §8.26 et §8.27 : références et plages d'une source amont ; renvoie le nom de sa
/// variante.
fn valider_source_amont(
    r: &mut RapportValidation,
    c: &str,
    s: &SourceAmontSaisie,
    bat: &Batiment,
) -> &'static str {
    let cs = format!("{c}.source_amont");
    let sinus = |r: &mut RapportValidation, ch: String, t: &TemperatureSinusoidale| {
        if !(1..=12).contains(&t.mois_minimum) {
            r.erreur(
                format!("{ch}.mois_minimum"),
                Some(t.mois_minimum.to_string()),
                "numéro de mois de 1 à 12 (Th-BCE 2020 §8.26.2, tableau 243)",
            );
        }
        if t.theta_max_c < t.theta_min_c {
            r.erreur(
                format!("{ch}.theta_max_c"),
                Some(t.theta_max_c.to_string()),
                "température maximale inférieure à la température minimale (Th-BCE 2020 \
                 §8.26.3.3.2.1)",
            );
        }
    };
    let pompes = |r: &mut RapportValidation, ch: String, p: &PompesCaptage| {
        r.positif(format!("{ch}.p_pompes_w"), p.p_pompes_w);
    };
    let ecarts = |r: &mut RapportValidation, ch: String, e: Option<crate::entree::EcartsSourceEau>| {
        if let Some(e) = e {
            if let Some(v) = e.delta_theta_ch_k {
                r.positif(format!("{ch}.ecarts.delta_theta_ch_k"), v);
            }
            if let Some(v) = e.delta_theta_fr_k {
                r.positif(format!("{ch}.ecarts.delta_theta_fr_k"), v);
            }
        }
    };
    match s {
        SourceAmontSaisie::AirExterieur { p_vent_gaine_w } => {
            if let Some(p) = p_vent_gaine_w {
                r.positif(format!("{cs}.air_exterieur.p_vent_gaine_w"), *p);
            }
            "air_exterieur"
        }
        SourceAmontSaisie::AirAmbiant {
            espace_tampon,
            p_vent_gaine_w,
        } => {
            if !bat.espaces_tampons.iter().any(|e| e.id == *espace_tampon) {
                r.erreur(
                    format!("{cs}.air_ambiant.espace_tampon"),
                    Some(espace_tampon.clone()),
                    "espace tampon inconnu du bâtiment",
                );
            }
            if let Some(p) = p_vent_gaine_w {
                r.positif(format!("{cs}.air_ambiant.p_vent_gaine_w"), *p);
            }
            "air_ambiant"
        }
        SourceAmontSaisie::AirExtrait {
            zone,
            p_vent_gaine_w,
            ..
        } => {
            if !bat.zones.iter().any(|z| z.id == *zone) {
                r.erreur(
                    format!("{cs}.air_extrait.zone"),
                    Some(zone.clone()),
                    "zone inconnue du bâtiment",
                );
            }
            if let Some(p) = p_vent_gaine_w {
                r.positif(format!("{cs}.air_extrait.p_vent_gaine_w"), *p);
            }
            "air_extrait"
        }
        SourceAmontSaisie::Captage {
            sol,
            rb_k_m_w,
            l_m,
            pompes: p,
            ecarts: e,
        } => {
            let ch = format!("{cs}.captage");
            sinus(r, format!("{ch}.sol"), sol);
            r.positif(format!("{ch}.rb_k_m_w"), *rb_k_m_w);
            r.strictement_positif(format!("{ch}.l_m"), *l_m);
            pompes(r, format!("{ch}.pompes"), p);
            ecarts(r, ch, *e);
            "captage"
        }
        SourceAmontSaisie::Nappe {
            nappe,
            pompes: p,
            echangeur,
            ecarts: e,
        } => {
            let ch = format!("{cs}.nappe");
            sinus(r, format!("{ch}.nappe"), nappe);
            pompes(r, format!("{ch}.pompes"), p);
            if let Some(x) = echangeur {
                let cx = format!("{ch}.echangeur");
                r.strictement_positif(format!("{cx}.qv_nappe_nom_m3_h"), x.qv_nappe_nom_m3_h);
                r.strictement_positif(format!("{cx}.qv_inter_nom_m3_h"), x.qv_inter_nom_m3_h);
                r.strictement_positif(format!("{cx}.rho_inter_kg_m3"), x.rho_inter_kg_m3);
                r.strictement_positif(format!("{cx}.cp_inter_j_kg_k"), x.cp_inter_j_kg_k);
                r.strictement_positif(format!("{cx}.ua_w_k"), x.ua_w_k);
                r.positif(format!("{cx}.p_pompes_inter_w"), x.p_pompes_inter_w);
            }
            ecarts(r, ch, *e);
            "nappe"
        }
        SourceAmontSaisie::TourDeRefroidissement {
            delta_theta_tour_k,
            p_pompes_tour_w,
            p_vent_tour_w,
            delta_theta_cond_fr_k,
            ..
        } => {
            let ch = format!("{cs}.tour_de_refroidissement");
            r.positif(format!("{ch}.delta_theta_tour_k"), *delta_theta_tour_k);
            r.positif(format!("{ch}.p_pompes_tour_w"), *p_pompes_tour_w);
            r.positif(format!("{ch}.p_vent_tour_w"), *p_vent_tour_w);
            if let Some(v) = delta_theta_cond_fr_k {
                r.positif(format!("{ch}.delta_theta_cond_fr_k"), *v);
            }
            "tour_de_refroidissement"
        }
        SourceAmontSaisie::BoucleDEau {
            boucle,
            pompes: p,
            ecarts: e,
        } => {
            let ch = format!("{cs}.boucle_d_eau");
            sinus(r, format!("{ch}.boucle"), boucle);
            pompes(r, format!("{ch}.pompes"), p);
            ecarts(r, ch, *e);
            "boucle_d_eau"
        }
        SourceAmontSaisie::Sol { sol, rb_k_m_w, l_m } => {
            let ch = format!("{cs}.sol");
            sinus(r, format!("{ch}.sol"), sol);
            r.positif(format!("{ch}.rb_k_m_w"), *rb_k_m_w);
            r.strictement_positif(format!("{ch}.l_m"), *l_m);
            "sol"
        }
    }
}
