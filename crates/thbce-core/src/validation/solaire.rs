// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des chauffe-eau solaires individuels (Th-BCE 2020 §9.14, §9.15 et §9.23).

use crate::ecs::gestion_regulation_ballon::THETA_C_ECS_C;
use crate::entree::{
    AppointCesi, BallonSaisi, Batiment, BoucleSolaireSaisie, CapteursSolaires, DescriptionCesi,
    FonctionEffetJoule, FonctionReseau, Generation, NatureGenerateur, PerformancesCapteurs,
    PertesBallon, RaccordementSsc, RegulationBoucleSaisie, TransfertBoucle,
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
                if let Some(kt) = k.k_theta {
                    r.strictement_positif(format!("{ck}.k_theta"), kt);
                    r.plage(format!("{ck}.k_theta"), kt, 0.0, 1.0);
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
                    (_, Some(f)) => r.erreur(
                        format!("{ck}.f_aux"),
                        Some(f.to_string()),
                        "« faux » sans objet sans appoint intégré",
                    ),
                    (_, None) => {}
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
    for (k, x) in g.systemes_solaires_combines.iter().enumerate() {
        let cc = format!("{c}.systemes_solaires_combines[{k}]");
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
}

fn valider_composants(
    r: &mut RapportValidation,
    cd: &str,
    capteurs: &CapteursSolaires,
    boucle: &BoucleSolaireSaisie,
    ballon: &BallonSaisi,
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
    r.strictement_positif(format!("{cd}.ballon.v_tot_l"), ballon.v_tot_l);
    if let PertesBallon::Certifie { ua_s_w_k } | PertesBallon::Justifie { ua_s_w_k } = ballon.pertes
    {
        r.strictement_positif(format!("{cd}.ballon.pertes.ua_s_w_k"), ua_s_w_k);
    }
    r.plage(
        format!("{cd}.ballon.theta_max_c"),
        ballon.theta_max_c,
        THETA_C_ECS_C,
        100.0,
    );
}
