// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation de la production d'ECS avec stockage d'une génération : assemblages avec
//! ballon (Th-BCE 2020 §9.14) et chauffe-eau thermodynamiques (§9.22).

use super::generation::postes_assures;
use super::RapportValidation;
use crate::ecs::echangeur_ballon::{ApplicationAccumulateur, THETA_CONS_COLLECTIF_C};
use crate::ecs::gestion_regulation_ballon::THETA_C_ECS_C;
use crate::ecs::idcet;
use crate::entree::{
    ApplicationAccumulateurSaisie, Batiment, FonctionEffetJoule, Generation, NatureEchangeurSaisie,
    NatureGenerateur, PertesBallon, ReseauEcsIntergroupe, SourceAmontSaisie,
};

/// Th-BCE 2020 §9.9, §9.10, §9.14 et §9.22 : assemblages avec ballon de stockage et
/// chauffe-eau thermodynamiques d'une génération.
pub(super) fn valider_stockages(
    r: &mut RapportValidation,
    c: &str,
    g: &Generation,
    bat: &Batiment,
) {
    r.ids_uniques(
        &format!("{c}.stockages"),
        g.stockages
            .iter()
            .map(|s| &s.id)
            .chain(g.chauffe_eau_thermodynamiques.iter().map(|x| &x.id))
            .chain(g.chauffe_eau_solaires.iter().map(|x| &x.id))
            .chain(g.systemes_solaires_combines.iter().map(|x| &x.id)),
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
        // Éq. (1948) : consigne conventionnelle de 55 °C, sous la température maximale ; celle
        // d'un accumulateur en eau technique est donnée par le tableau 288 (55 ou 60 °C).
        let consigne = s.echangeur.as_ref().map_or(THETA_C_ECS_C, |x| {
            match x.application.map(|a| match a {
                ApplicationAccumulateurSaisie::Individuel => ApplicationAccumulateur::Individuel,
                ApplicationAccumulateurSaisie::Collectif => ApplicationAccumulateur::Collectif,
            }) {
                Some(a) => a.theta_cons_c(),
                None => ApplicationAccumulateur::selon_volume(s.ballon.v_tot_l).theta_cons_c(),
            }
        });
        debug_assert!(consigne <= THETA_CONS_COLLECTIF_C);
        r.plage(
            format!("{cs}.ballon.theta_max_c"),
            s.ballon.theta_max_c,
            consigne,
            100.0,
        );
        if let Some(x) = &s.echangeur {
            valider_accumulateur(r, &cs, x, s.appoint.as_ref(), g, bat);
        }
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
                let thermo = matches!(
                    x.nature,
                    NatureGenerateur::ThermodynamiqueElectrique(_)
                        | NatureGenerateur::ThermodynamiqueGaz(_)
                );
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

/// Th-BCE 2020 §9.11 et §9.12 : accumulateur d'ECS en eau technique d'un assemblage.
fn valider_accumulateur(
    r: &mut RapportValidation,
    cs: &str,
    x: &crate::entree::EchangeurEcsSaisi,
    appoint: Option<&crate::entree::AppointBallon>,
    g: &Generation,
    bat: &Batiment,
) {
    r.strictement_positif(format!("{cs}.echangeur.p_ech_ecs_kw"), x.p_ech_ecs_kw);
    if let NatureEchangeurSaisie::Externe {
        q_v_prim_nom_m3_h,
        p_circ_prim_nom_w,
        mod_min,
        ..
    } = &x.nature
    {
        let ce = format!("{cs}.echangeur.nature.externe");
        r.strictement_positif(format!("{ce}.q_v_prim_nom_m3_h"), *q_v_prim_nom_m3_h);
        r.positif(format!("{ce}.p_circ_prim_nom_w"), *p_circ_prim_nom_w);
        if let Some(m) = mod_min {
            r.plage(format!("{ce}.mod_min"), *m, 0.0, 1.0);
        }
    }
    // Quatre zones de volumes égaux, quelle que soit la fraction de l'appoint (éq. 1874) :
    // « faux » est sans objet.
    if let Some(a) = appoint {
        if a.f_aux.is_some() {
            r.erreur(
                format!("{cs}.appoint.f_aux"),
                a.f_aux.map(|f| f.to_string()),
                "sans objet pour un accumulateur en eau technique : le ballon est décomposé en \
                 quatre zones de volumes égaux, quelle que soit la fraction chauffée par \
                 l'appoint (Th-BCE 2020 §9.12.3.2.2, éq. 1874 et 1875)",
            );
        }
    }
    // Réseau d'ECS bouclé : le réchauffage de boucle (éq. 1883 et 1909 à 1913) demande le
    // débit de la boucle, que la fiche 9.8 ne restitue pas (Q48 point 8, Q106).
    let bouclee = bat
        .distributions_ecs_intergroupes
        .iter()
        .any(|d| d.generation == g.id && matches!(d.reseau, ReseauEcsIntergroupe::Boucle(_)));
    if bouclee {
        r.hors_perimetre(
            format!("{cs}.echangeur"),
            None,
            "accumulateur d'ECS en eau technique desservant un réseau d'ECS bouclé \
             (réchauffage de boucle, Th-BCE 2020 §9.12.3.2.4.2)",
        );
    }
}
