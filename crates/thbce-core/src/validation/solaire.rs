// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des chauffe-eau solaires individuels (Th-BCE 2020 §9.14, §9.15 et §9.23).

use crate::ecs::gestion_regulation_ballon::THETA_C_ECS_C;
use crate::entree::{
    AppointCesi, BallonSaisi, Batiment, BoucleSolaireSaisie, CapteursSolaires, DescriptionCesi,
    FonctionEffetJoule, FonctionReseau, Generation, NatureGenerateur, PerformancesCapteurs,
    PertesBallon, RaccordementSsc, RegulationBoucleSaisie, ReseauEcsIntergroupe, ReseauGroupe,
    TransfertBoucle, TypePcad,
};

use super::generation::postes_assures;
use super::{valider_exposition, RapportValidation, References};

pub(super) fn valider_chauffe_eau_solaires(
    r: &mut RapportValidation,
    cb: &str,
    bat: &Batiment,
    refs: &References<'_>,
) {
    for (i, g) in bat.generations.iter().enumerate() {
        valider_generation(r, &format!("{cb}.generations[{i}]"), g, bat, refs);
    }
}

fn valider_generation(
    r: &mut RapportValidation,
    c: &str,
    g: &Generation,
    bat: &Batiment,
    refs: &References<'_>,
) {
    // Générateurs déjà pilotés par un ballon.
    let mut pilotes: Vec<&str> = g
        .stockages
        .iter()
        .flat_map(|s| {
            std::iter::once(s.base.generateur.as_str())
                .chain(s.appoint.as_ref().map(|a| a.generateur.as_str()))
        })
        .collect();
    for (k, x) in g.chauffe_eau_solaires.iter().enumerate() {
        let cc = format!("{c}.chauffe_eau_solaires[{k}]");
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
                "au moins un chauffe-eau (Th-BCE 2020 §9.14.2)",
            );
        }
        // Tableau 292 : β de 0 à 90°.
        valider_exposition(
            r,
            &format!("{cc}.exposition_capteurs"),
            &x.exposition_capteurs,
            refs,
        );
        r.plage(
            format!("{cc}.exposition_capteurs.inclinaison_deg"),
            x.exposition_capteurs.inclinaison_deg,
            0.0,
            90.0,
        );
        let mut f_aux_kit = None;
        match &x.description {
            DescriptionCesi::Composants {
                capteurs,
                boucle,
                ballon,
            } => {
                let cd = format!("{cc}.description.composants");
                valider_composants(r, &cd, capteurs, boucle, ballon);
            }
            DescriptionCesi::Kit(k) => {
                let ck = format!("{cc}.description.kit");
                r.strictement_positif(format!("{ck}.surface_m2"), k.surface_m2);
                r.strictement_positif(format!("{ck}.v_tot_l"), k.v_tot_l);
                r.positif(format!("{ck}.p_np_w"), k.p_np_w);
                r.strictement_positif(format!("{ck}.ac_etoile_m2"), k.ac_etoile_m2);
                r.positif(format!("{ck}.uc_etoile_w_m2k"), k.uc_etoile_w_m2k);
                r.strictement_positif(format!("{ck}.us_w_k"), k.us_w_k);
                r.plage(
                    format!("{ck}.theta_max_c"),
                    k.theta_max_c,
                    THETA_C_ECS_C,
                    100.0,
                );
                // « Kθ » : sortie de l'outil IdCESI (§9.23.3.2.3), sans valeur dans le texte.
                r.strictement_positif(format!("{ck}.k_theta"), k.k_theta);
                r.plage(format!("{ck}.k_theta"), k.k_theta, 0.0, 1.0);
                // §9.23.3.2.2 : « l'assemblage va être strictement identique au ballon base
                // seule solaire avec appoint intégré ».
                if !matches!(x.appoint, AppointCesi::Integre(_)) {
                    r.erreur(
                        format!("{cc}.appoint"),
                        Some(
                            match x.appoint {
                                AppointCesi::Aucun => "aucun",
                                _ => "separe_instantane",
                            }
                            .into(),
                        ),
                        "un kit CESI est un ballon base seule solaire avec appoint intégré \
                         (Th-BCE 2020 §9.23.3.2.2) : appoint `integre` requis ; un chauffe-eau \
                         sans appoint ou à appoint séparé se décrit par ses composants",
                    );
                }
                // Éq. (2266) : η0 = AC*/(0,8·A) ne peut dépasser 1.
                if k.surface_m2 > 0.0 && k.ac_etoile_m2 > 0.8 * k.surface_m2 {
                    r.erreur(
                        format!("{ck}.ac_etoile_m2"),
                        Some(k.ac_etoile_m2.to_string()),
                        "AC* supérieure à 0,8·A : le rendement optique déduit, AC*/(0,8·A), \
                         dépasserait 1 (Th-BCE 2020 §9.23.3.2.2, éq. 2266)",
                    );
                }
                match (&x.appoint, k.f_aux) {
                    (AppointCesi::Integre(_), None) => r.erreur(
                        format!("{ck}.f_aux"),
                        None,
                        "« faux » requise pour un kit à appoint intégré (Th-BCE 2020 §9.23.3.2.2, \
                         éq. 2272)",
                    ),
                    (AppointCesi::Integre(_), Some(f)) => f_aux_kit = Some(f),
                    // Appoint autre qu'intégré : refusé ci-dessus.
                    (_, _) => {}
                }
            }
        }
        match &x.appoint {
            AppointCesi::Aucun => {}
            AppointCesi::SepareInstantane => {
                // L'appoint séparé instantané est un générateur instantané d'ECS de la
                // génération (§9.14.1, « Type_prod_stockage = 3 »).
                let instantane = g
                    .generateurs
                    .iter()
                    .any(|y| postes_assures(&y.nature)[2] && !pilotes.contains(&y.id.as_str()));
                if !instantane {
                    r.erreur(
                        format!("{cc}.appoint"),
                        Some("separe_instantane".into()),
                        "un appoint séparé instantané demande un générateur instantané d'ECS dans \
                         la génération (non piloté par un ballon)",
                    );
                }
            }
            AppointCesi::Integre(a) => {
                let ca = format!("{cc}.appoint.integre");
                for (champ, z) in [("z_ap", a.z_ap), ("z_reg", a.z_reg)] {
                    if !(1..=4).contains(&z) {
                        r.erreur(
                            format!("{ca}.{champ}"),
                            Some(z.to_string()),
                            "numéro de zone du ballon de 1 (bas) à 4 (haut) (Th-BCE 2020 §9.9.2)",
                        );
                    }
                }
                if let Some(f) = a.f_aux {
                    if f_aux_kit.is_some() {
                        r.erreur(
                            format!("{ca}.f_aux"),
                            Some(f.to_string()),
                            "« faux » d'un kit CESI saisie dans le kit (`description.kit.f_aux`)",
                        );
                    }
                }
                if let Some(f) = a.f_aux.or(f_aux_kit) {
                    if !(f > 0.0 && f < 1.0) {
                        r.erreur(
                            format!("{ca}.f_aux"),
                            Some(f.to_string()),
                            "« faux » strictement compris entre 0 et 1 (Th-BCE 2020 §9.9.3.1)",
                        );
                    }
                }
                if let Some(d) = a.delta_theta_k {
                    r.plage(format!("{ca}.delta_theta_k"), d, 0.0, 20.0);
                }
                let ok = g.generateurs.iter().any(|y| {
                    y.id == a.generateur
                        && matches!(&y.nature, NatureGenerateur::EffetJoule(j)
                            if j.fonction == FonctionEffetJoule::Ecs)
                });
                if !ok || pilotes.contains(&a.generateur.as_str()) {
                    r.erreur(
                        format!("{ca}.generateur"),
                        Some(a.generateur.clone()),
                        "l'appoint intégré d'un chauffe-eau solaire est un générateur à effet \
                         Joule de fonction `ecs` de la génération, propre à l'assemblage",
                    );
                }
                pilotes.push(&a.generateur);
            }
        }
    }
    let chauffe = bat
        .distributions_intergroupes
        .iter()
        .any(|d| d.generation == g.id && d.fonction == FonctionReseau::Chauffage);
    // Éq. (2117) : « θch_max^gen », température maximale des réseaux de chauffage de la
    // génération, est la température de départ de dimensionnement la plus haute de ses réseaux
    // hydrauliques de chauffage (lecture Q46 point 5 et Q52 point 12) ; elle doit exister.
    let theta_ch_max_saisie = bat.zones.iter().flat_map(|z| &z.groupes).any(|gr| {
        gr.distributions.iter().any(|d| {
            d.fonction == FonctionReseau::Chauffage
                && matches!(&d.reseau, ReseauGroupe::Hydraulique(h) if h.theta_dep_dim_c.is_some())
                && bat.distributions_intergroupes.iter().any(|di| {
                    di.id == d.distribution_intergroupes
                        && di.generation == g.id
                        && di.fonction == FonctionReseau::Chauffage
                })
        })
    });
    for (k, x) in g.systemes_solaires_combines.iter().enumerate() {
        let cc = format!("{c}.systemes_solaires_combines[{k}]");
        if chauffe && !theta_ch_max_saisie {
            r.erreur(
                cc.clone(),
                None,
                "un système solaire combiné demande un réseau hydraulique de chauffage du groupe, \
                 alimenté par la génération, dont la température de départ de dimensionnement \
                 `theta_dep_dim_c` est saisie : c'est « θch_max^gen » (Th-BCE 2020 §9.18.3, \
                 éq. 2117 ; lecture Q52)",
            );
        }
        if g.generateurs.iter().any(|y| y.id == x.id) {
            r.erreur(
                format!("{cc}.id"),
                Some(x.id.clone()),
                "identifiant déjà porté par un générateur de la génération",
            );
        }
        if !chauffe {
            r.erreur(
                cc.clone(),
                None,
                "un système solaire combiné assure le chauffage : la génération doit alimenter \
                 un réseau de chauffage (Th-BCE 2020 §9.19.3, éq. 2127)",
            );
        }
        valider_exposition(
            r,
            &format!("{cc}.exposition_capteurs"),
            &x.exposition_capteurs,
            refs,
        );
        r.plage(
            format!("{cc}.exposition_capteurs.inclinaison_deg"),
            x.exposition_capteurs.inclinaison_deg,
            0.0,
            90.0,
        );
        valider_composants(r, &cc, &x.capteurs, &x.boucle, &x.ballon);
        let a = &x.appoint;
        if let Some(f) = a.f_aux {
            if !(f > 0.0 && f < 1.0) {
                r.erreur(
                    format!("{cc}.appoint.f_aux"),
                    Some(f.to_string()),
                    "« faux » strictement compris entre 0 et 1 (Th-BCE 2020 §9.9.3.1)",
                );
            }
        }
        if let Some(d) = a.delta_theta_k {
            r.plage(format!("{cc}.appoint.delta_theta_k"), d, 0.0, 20.0);
        }
        let generateur = g.generateurs.iter().find(|y| y.id == a.generateur);
        let (ok, attendu) = match x.raccordement {
            RaccordementSsc::Independant {
                delta_theta_reg_boucle_chauffage_k,
            } => {
                r.plage(
                    format!("{cc}.raccordement.independant.delta_theta_reg_boucle_chauffage_k"),
                    delta_theta_reg_boucle_chauffage_k,
                    0.0,
                    50.0,
                );
                (
                    generateur.is_some_and(|y| {
                        matches!(&y.nature, NatureGenerateur::EffetJoule(j)
                            if j.fonction == FonctionEffetJoule::Ecs)
                    }),
                    "générateur à effet Joule de fonction `ecs` (fiche 9.19, éq. 2127 : appoint \
                     d'ECS seule)",
                )
            }
            RaccordementSsc::Raccorde => (
                generateur.is_some_and(|y| {
                    let p = postes_assures(&y.nature);
                    p[0] && p[2]
                }),
                "générateur assurant le chauffage et l'ECS (fiche 9.20, éq. 2141)",
            ),
        };
        if !ok || pilotes.contains(&a.generateur.as_str()) {
            r.erreur(
                format!("{cc}.appoint.generateur"),
                Some(a.generateur.clone()),
                format!(
                    "l'appoint d'un système solaire combiné est un {attendu} de la génération, \
                     propre à l'assemblage"
                ),
            );
        }
        pilotes.push(&a.generateur);
    }
    let mut pilotes: Vec<String> = pilotes.iter().map(|x| (*x).to_owned()).collect();
    valider_collectifs(r, c, g, bat, refs, &mut pilotes);
}

fn valider_composants(
    r: &mut RapportValidation,
    cd: &str,
    capteurs: &CapteursSolaires,
    boucle: &BoucleSolaireSaisie,
    ballon: &BallonSaisi,
) {
    valider_boucle(r, cd, capteurs, boucle);
    valider_ballon(r, &format!("{cd}.ballon"), ballon);
}

fn valider_boucle(
    r: &mut RapportValidation,
    cd: &str,
    capteurs: &CapteursSolaires,
    boucle: &BoucleSolaireSaisie,
) {
    r.strictement_positif(format!("{cd}.capteurs.surface_m2"), capteurs.surface_m2);
    if let PerformancesCapteurs::Certifiees {
        eta0,
        a1_w_m2k,
        a2_w_m2k2,
    } = capteurs.performances
    {
        let cp = format!("{cd}.capteurs.performances.certifiees");
        r.plage(format!("{cp}.eta0"), eta0, 0.0, 1.0);
        r.positif(format!("{cp}.a1_w_m2k"), a1_w_m2k);
        r.positif(format!("{cp}.a2_w_m2k2"), a2_w_m2k2);
    }
    r.strictement_positif(format!("{cd}.capteurs.k_theta"), capteurs.k_theta);
    r.plage(format!("{cd}.capteurs.k_theta"), capteurs.k_theta, 0.0, 1.0);
    r.positif(format!("{cd}.boucle.ua_te_w_k"), boucle.ua_te_w_k);
    r.positif(format!("{cd}.boucle.ua_ti_w_k"), boucle.ua_ti_w_k);
    r.positif(format!("{cd}.boucle.p_np_w"), boucle.p_np_w);
    if let TransfertBoucle::Echangeur {
        ua_hx_w_k: Some(ua),
    } = boucle.transfert
    {
        r.strictement_positif(format!("{cd}.boucle.transfert.echangeur.ua_hx_w_k"), ua);
    }
    if let RegulationBoucleSaisie::Irradiance {
        isr_min_w_m2: Some(v),
    } = boucle.regulation
    {
        r.plage(
            format!("{cd}.boucle.regulation.irradiance.isr_min_w_m2"),
            v,
            0.0,
            1500.0,
        );
    }
    if let Some(a) = boucle.alpha_p {
        r.plage(format!("{cd}.boucle.alpha_p"), a, 0.0, 1.0);
    }
    if let Some(m) = boucle.m_cp_w_k {
        r.strictement_positif(format!("{cd}.boucle.m_cp_w_k"), m);
    }
    if let Some(h) = boucle.h_rel_ech {
        r.plage(format!("{cd}.boucle.h_rel_ech"), h, 0.0, 1.0);
    }
}

fn valider_ballon(r: &mut RapportValidation, cb: &str, ballon: &BallonSaisi) {
    r.strictement_positif(format!("{cb}.v_tot_l"), ballon.v_tot_l);
    if let PertesBallon::Certifie { ua_s_w_k } | PertesBallon::Justifie { ua_s_w_k } = ballon.pertes
    {
        r.strictement_positif(format!("{cb}.pertes.ua_s_w_k"), ua_s_w_k);
    }
    r.plage(
        format!("{cb}.theta_max_c"),
        ballon.theta_max_c,
        THETA_C_ECS_C,
        100.0,
    );
}

/// Productions solaires collectives à appoints décentralisés (fiches 9.16 et 9.17).
fn valider_collectifs(
    r: &mut RapportValidation,
    c: &str,
    g: &Generation,
    bat: &Batiment,
    refs: &References<'_>,
    pilotes: &mut Vec<String>,
) {
    if g.chauffe_eau_solaires_collectifs.is_empty() {
        return;
    }
    if g.chauffe_eau_solaires_collectifs.len() > 1 {
        r.erreur(
            format!("{c}.chauffe_eau_solaires_collectifs"),
            None,
            "une génération porte au plus une production solaire collective, qui assure toute son \
             ECS (Th-BCE 2020 §9.16.3 : le CESCI « a exactement le même rôle qu'un objet \
             génération »)",
        );
    }
    if !(g.stockages.is_empty()
        && g.chauffe_eau_thermodynamiques.is_empty()
        && g.chauffe_eau_solaires.is_empty()
        && g.systemes_solaires_combines.is_empty())
    {
        r.erreur(
            format!("{c}.chauffe_eau_solaires_collectifs"),
            None,
            "une génération pourvue d'une production solaire collective n'a pas d'autre \
             assemblage avec ballon : ses ballons individuels assurent toute l'ECS",
        );
    }
    let reseaux = |f: FonctionReseau| {
        bat.distributions_intergroupes
            .iter()
            .any(|d| d.generation == g.id && d.fonction == f)
    };
    let ecs: Vec<&ReseauEcsIntergroupe> = bat
        .distributions_ecs_intergroupes
        .iter()
        .filter(|d| d.generation == g.id)
        .map(|d| &d.reseau)
        .collect();
    for (k, x) in g.chauffe_eau_solaires_collectifs.iter().enumerate() {
        let cc = format!("{c}.chauffe_eau_solaires_collectifs[{k}]");
        if g.generateurs.iter().any(|y| y.id == x.id) {
            r.erreur(
                format!("{cc}.id"),
                Some(x.id.clone()),
                "identifiant déjà porté par un générateur de la génération",
            );
        }
        let cescai = x.type_pcad == TypePcad::Cescai;
        // Éq. (1975) : pas de refroidissement ; éq. (2063) et (2064) : un CESCAI n'assure que
        // l'ECS.
        if reseaux(FonctionReseau::Refroidissement)
            || (cescai && reseaux(FonctionReseau::Chauffage))
        {
            r.erreur(
                cc.clone(),
                None,
                if cescai {
                    "un CESCAI n'assure que l'ECS : sa génération n'alimente aucun réseau de \
                     chauffage ni de refroidissement (Th-BCE 2020 §9.17.3, éq. 2063 et 2064)"
                } else {
                    "les appoints d'un CESCI ne peuvent assurer le refroidissement : sa \
                     génération n'alimente aucun réseau de refroidissement (Th-BCE 2020 \
                     §9.16.3, éq. 1975)"
                },
            );
        }
        // Réseau d'ECS intergroupe : aucun pour un CESCI (les distributions du groupe sont
        // reliées à l'assemblage, §9.16.3) ; bouclé, sans réchauffeur, pour un CESCAI
        // (§9.17.3.2, deuxième étape).
        let ok_reseau = !ecs.is_empty()
            && ecs.iter().all(|d| match d {
                ReseauEcsIntergroupe::Aucun => !cescai,
                ReseauEcsIntergroupe::Boucle(b) => cescai && !b.rechauffeur,
                ReseauEcsIntergroupe::Trace(_) => false,
            });
        if !ok_reseau {
            r.erreur(
                cc.clone(),
                None,
                if cescai {
                    "un CESCAI distribue l'ECS par un réseau intergroupe bouclé (`boucle`, sans \
                     réchauffeur de boucle) (Th-BCE 2020 §9.17.3.2 ; lecture Q53)"
                } else {
                    "un CESCI dessert directement les distributions du groupe : réseau d'ECS \
                     intergroupe `aucun` (Th-BCE 2020 §9.16.3)"
                },
            );
        }
        valider_exposition(
            r,
            &format!("{cc}.exposition_capteurs"),
            &x.exposition_capteurs,
            refs,
        );
        r.plage(
            format!("{cc}.exposition_capteurs.inclinaison_deg"),
            x.exposition_capteurs.inclinaison_deg,
            0.0,
            90.0,
        );
        valider_boucle(r, &cc, &x.capteurs, &x.boucle);
        // Éq. (1981) : boucle d'un CESCI régulée sur l'ensoleillement.
        if !cescai
            && !matches!(
                x.boucle.regulation,
                RegulationBoucleSaisie::Irradiance { .. }
            )
        {
            r.erreur(
                format!("{cc}.boucle.regulation"),
                Some("temperature".into()),
                "la boucle solaire d'un CESCI est régulée sur l'ensoleillement (« Typeregul_BS = \
                 1 », Th-BCE 2020 §9.16.3, éq. 1981)",
            );
        }
        if x.boucle.h_rel_ech.is_some() {
            r.erreur(
                format!("{cc}.boucle.h_rel_ech"),
                None,
                "échangeurs solaires ponctuels en zone basse des ballons (éq. 1980 et 2066) : \
                 `h_rel_ech` sans objet",
            );
        }
        match (&x.ballon_central, cescai) {
            (Some(b), true) => valider_ballon(r, &format!("{cc}.ballon_central"), b),
            (None, true) => r.erreur(
                format!("{cc}.ballon_central"),
                None,
                "ballon central requis pour un CESCAI (Th-BCE 2020 §9.17.3, éq. 2065)",
            ),
            (Some(_), false) => r.erreur(
                format!("{cc}.ballon_central"),
                None,
                "ballon central sans objet pour un CESCI",
            ),
            (None, false) => {}
        }
        if x.ensembles.is_empty() {
            r.erreur(
                format!("{cc}.ensembles"),
                None,
                "au moins un ensemble de ballons individuels",
            );
        }
        r.ids_uniques(
            &format!("{cc}.ensembles"),
            x.ensembles.iter().map(|e| &e.id),
        );
        for (j, en) in x.ensembles.iter().enumerate() {
            let ce = format!("{cc}.ensembles[{j}]");
            if en.nb_ballons == 0 {
                r.erreur(
                    format!("{ce}.nb_ballons"),
                    Some("0".into()),
                    "au moins un ballon",
                );
            }
            valider_ballon(r, &format!("{ce}.ballon"), &en.ballon);
            let separe = x.type_pcad == TypePcad::CesciAppointSepare;
            match (&en.appoint, separe) {
                (Some(_), true) => r.erreur(
                    format!("{ce}.appoint"),
                    None,
                    "appoint intégré sans objet pour un CESCI à appoint séparé : les générateurs \
                     instantanés d'ECS de la génération assurent l'appoint (éq. 1979 et 2010)",
                ),
                (None, false) => r.erreur(
                    format!("{ce}.appoint"),
                    None,
                    "appoint intégré requis (éq. 1979 et 2065)",
                ),
                (None, true) => {}
                (Some(a), false) => {
                    let ca = format!("{ce}.appoint");
                    for (champ, z) in [("z_ap", a.z_ap), ("z_reg", a.z_reg)] {
                        if !(1..=4).contains(&z) {
                            r.erreur(
                                format!("{ca}.{champ}"),
                                Some(z.to_string()),
                                "numéro de zone du ballon de 1 (bas) à 4 (haut) (Th-BCE 2020 \
                                 §9.9.2)",
                            );
                        }
                    }
                    if let Some(f) = a.f_aux {
                        if !(f > 0.0 && f < 1.0) {
                            r.erreur(
                                format!("{ca}.f_aux"),
                                Some(f.to_string()),
                                "« faux » strictement compris entre 0 et 1 (Th-BCE 2020 \
                                 §9.9.3.1)",
                            );
                        }
                    }
                    if let Some(d) = a.delta_theta_k {
                        r.plage(format!("{ca}.delta_theta_k"), d, 0.0, 20.0);
                    }
                    let y = g.generateurs.iter().find(|y| y.id == a.generateur);
                    let nature_ok = y.is_some_and(|y| {
                        postes_assures(&y.nature)[2]
                            && matches!(
                                y.nature,
                                NatureGenerateur::EffetJoule(_) | NatureGenerateur::Chaudiere(_)
                            )
                    });
                    if !nature_ok || pilotes.contains(&a.generateur) {
                        r.erreur(
                            format!("{ca}.generateur"),
                            Some(a.generateur.clone()),
                            "l'appoint intégré d'un ballon individuel est un générateur de la \
                             génération assurant l'ECS, propre à l'ensemble : effet Joule de \
                             fonction `ecs` ou chaudière (les générateurs thermodynamiques \
                             d'appoint sont hors de cette version)",
                        );
                    } else if y.is_some_and(|y| y.rdim != en.nb_ballons) {
                        r.erreur(
                            format!("{ce}.nb_ballons"),
                            Some(en.nb_ballons.to_string()),
                            "le nombre de ballons de l'ensemble est le nombre de générateurs \
                             d'appoint identiques (`rdim` du générateur désigné, §9.16.3.4.2.1)",
                        );
                    }
                    pilotes.push(a.generateur.clone());
                }
            }
        }
        if x.type_pcad == TypePcad::CesciAppointSepare {
            let instantane = g.generateurs.iter().any(|y| postes_assures(&y.nature)[2]);
            if !instantane {
                r.erreur(
                    format!("{cc}.type_pcad"),
                    Some("cesci_appoint_separe".into()),
                    "un CESCI à appoint séparé instantané demande un générateur instantané d'ECS \
                     dans la génération (éq. 2010)",
                );
            }
        }
    }
}
