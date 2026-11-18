// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des systèmes de chauffage et de refroidissement (Th-BCE 2020 §8.1 à 8.10) :
//! plages des nomenclatures, références entre émetteurs, réseaux et générations, cohérence
//! d'`iclim`, et règles de saisie des fiches d'application (§8.2, §8.13).

use crate::entree::{
    Batiment, Circulateur, ConfigurationRecuperateur, EchangeurEauxGrises, Emetteur, EmetteurEcs,
    FonctionReseau, Groupe, RegulationDebit, RegulationTemperature, ReseauEcsIntergroupe,
    ReseauGroupe, ReseauHydrauliqueGroupe, ReseauIntergroupes, UsageZone, VariationSpatiale,
    VariationSpatialeFroid, VariationTemporelle,
};

use std::collections::BTreeSet;

use super::RapportValidation;

/// Valide les générations et les réseaux intergroupes d'un bâtiment, puis les systèmes de
/// chacun de ses groupes d'habitation.
pub(super) fn valider_systemes_batiment(r: &mut RapportValidation, cb: &str, bat: &Batiment) {
    let gens = r.ids_uniques(
        &format!("{cb}.generations"),
        bat.generations.iter().map(|g| &g.id),
    );
    r.ids_uniques(
        &format!("{cb}.distributions_intergroupes"),
        bat.distributions_intergroupes.iter().map(|d| &d.id),
    );
    for (i, d) in bat.distributions_intergroupes.iter().enumerate() {
        if let ReseauIntergroupes::HydrauliqueCollectif(h)
        | ReseauIntergroupes::HydrauliqueIndividuel(h) = &d.reseau
        {
            let ch = format!("{cb}.distributions_intergroupes[{i}].reseau");
            for (nom, x) in [
                ("l_vc_m", h.l_vc_m),
                ("l_hvc_m", h.l_hvc_m),
                ("u_vc_w_mk", h.u_vc_w_mk),
                ("u_hvc_w_mk", h.u_hvc_w_mk),
                ("p_aux_w", h.p_aux_w),
            ] {
                r.positif(format!("{ch}.{nom}"), x);
            }
            espace_tampon_connu(r, &ch, h.espace_tampon.as_deref(), bat);
        }
        if !gens.contains(d.generation.as_str()) {
            r.erreur(
                format!("{cb}.distributions_intergroupes[{i}].generation"),
                Some(d.generation.clone()),
                "génération inconnue du bâtiment (un réseau intergroupes est relié à une et une \
                 seule génération, Th-BCE 2020 §8.10.1)",
            );
        }
    }
    r.ids_uniques(
        &format!("{cb}.recuperateurs_eaux_grises"),
        bat.recuperateurs_eaux_grises.iter().map(|c| &c.id),
    );
    for (i, c) in bat.recuperateurs_eaux_grises.iter().enumerate() {
        let cc = format!("{cb}.recuperateurs_eaux_grises[{i}]");
        valider_echangeur_eaux_grises(r, &format!("{cc}.echangeur"), &c.echangeur);
        r.positif(format!("{cc}.l_evac_vc_m"), c.l_evac_vc_m);
        r.positif(format!("{cc}.l_evac_vnc_m"), c.l_evac_vnc_m);
        verifier_conduit_eaux_grises(r, &cc, c.l_evac_vc_m, c.l_evac_vnc_m);
    }
    let ids_ecs = r.ids_uniques(
        &format!("{cb}.distributions_ecs_intergroupes"),
        bat.distributions_ecs_intergroupes.iter().map(|d| &d.id),
    );
    for (i, d) in bat.distributions_ecs_intergroupes.iter().enumerate() {
        let cd = format!("{cb}.distributions_ecs_intergroupes[{i}]");
        if !gens.contains(d.generation.as_str()) {
            r.erreur(
                format!("{cd}.generation"),
                Some(d.generation.clone()),
                "génération inconnue du bâtiment (Th-BCE 2020 §9.8.3.1)",
            );
        }
        let (l_vc, l_hvc, u, et, cr) = match &d.reseau {
            ReseauEcsIntergroupe::Aucun => continue,
            ReseauEcsIntergroupe::Boucle(b) => {
                r.positif(format!("{cd}.reseau.boucle.p_circ_w"), b.p_circ_w);
                valider_gestion_circulateurs(r, &cd, b, bat, &d.id);
                (
                    b.l_vc_m,
                    b.l_hvc_m,
                    b.u_w_mk,
                    b.espace_tampon.as_deref(),
                    "boucle",
                )
            }
            ReseauEcsIntergroupe::Trace(t) => (
                t.l_vc_m,
                t.l_hvc_m,
                t.u_w_mk,
                t.espace_tampon.as_deref(),
                "trace",
            ),
        };
        let cr = format!("{cd}.reseau.{cr}");
        r.positif(format!("{cr}.l_vc_m"), l_vc);
        r.positif(format!("{cr}.l_hvc_m"), l_hvc);
        r.positif(format!("{cr}.u_w_mk"), u);
        espace_tampon_connu(r, &cr, et, bat);
    }
    for (iz, zone) in bat.zones.iter().enumerate() {
        if !zone.usage.est_pris_en_charge() {
            continue;
        }
        for (ig, g) in zone.groupes.iter().enumerate() {
            valider_ecs_groupe(
                r,
                &format!("{cb}.zones[{iz}].groupes[{ig}]"),
                g,
                bat,
                &ids_ecs,
                zone.usage,
            );
        }
        for (ig, g) in zone.groupes.iter().enumerate() {
            valider_systemes_groupe(r, &format!("{cb}.zones[{iz}].groupes[{ig}]"), g, bat);
        }
    }
}

/// « Typegest_circ-e » (§9.8.2 et §9.8.3.3.4, éq. 1747 et 1748) : « pour les usages
/// d'enseignement, nous supposerons que les circulateurs peuvent être arrêtés durant les
/// vacances. Pour les autres usages, le fonctionnement des circulateurs est permanent ». Le
/// paramètre d'intégration n'a pas de valeur conventionnelle : requis pour une boucle qui
/// dessert une zone d'usage affilié à l'enseignement (`ienseignement`, tableau 4), refusé à vrai
/// sinon (lecture Q71).
fn valider_gestion_circulateurs(
    r: &mut RapportValidation,
    cd: &str,
    b: &crate::entree::BoucleEcs,
    bat: &Batiment,
    id: &str,
) {
    let enseignement = bat.zones.iter().any(|z| {
        z.usage.i_enseignement()
            && z.groupes.iter().any(|g| {
                g.emetteurs_ecs
                    .iter()
                    .any(|e| e.distribution_intergroupe == id)
            })
    });
    let ch = format!("{cd}.reseau.boucle.arret_circulateurs_vacances");
    match (enseignement, b.arret_circulateurs_vacances) {
        (true, None) => r.erreur(
            ch,
            None,
            "gestion des circulateurs requise pour une boucle d'ECS qui dessert une zone \
             d'enseignement : vrai s'ils sont arrêtés pendant les vacances, faux s'ils \
             fonctionnent en permanence (« Typegest_circ-e », Th-BCE 2020 §9.8.2, éq. 1747)",
        ),
        (false, Some(true)) => r.erreur(
            ch,
            Some("true".into()),
            "l'arrêt des circulateurs pendant les vacances n'est prévu que pour les usages \
             d'enseignement ; ailleurs leur fonctionnement est permanent (Th-BCE 2020 \
             §9.8.3.3.4)",
        ),
        _ => {}
    }
}

fn valider_systemes_groupe(r: &mut RapportValidation, c: &str, g: &Groupe, bat: &Batiment) {
    let Some(iclim) = g.iclim else {
        r.erreur(
            format!("{c}.iclim"),
            None,
            "« iclim » requis : vrai si le groupe est climatisé (Th-BCE 2020 \
             §8.1.2)",
        );
        return;
    };
    match &g.programmation {
        None => r.erreur(
            format!("{c}.programmation"),
            None,
            "programmation des relances requise (Th-BCE 2020 §8.5)",
        ),
        Some(p) => {
            if iclim && p.refroidissement.is_none() {
                r.erreur(
                    format!("{c}.programmation.refroidissement"),
                    None,
                    "programmation de la relance du refroidissement requise pour un groupe \
                     climatisé (Th-BCE 2020 §8.5.3, tableau 93)",
                );
            }
        }
    }
    r.ids_uniques(&format!("{c}.emetteurs"), g.emetteurs.iter().map(|e| &e.id));
    r.ids_uniques(
        &format!("{c}.distributions"),
        g.distributions.iter().map(|d| &d.id),
    );
    if !g.emetteurs.iter().any(|e| e.chauffage.is_some()) {
        r.erreur(
            format!("{c}.emetteurs"),
            None,
            "au moins un émetteur de chauffage requis (Th-BCE 2020 §8.1)",
        );
    }
    let a_froid = g.emetteurs.iter().any(|e| e.refroidissement.is_some());
    if iclim != a_froid {
        r.erreur(
            format!("{c}.iclim"),
            Some(iclim.to_string()),
            "un groupe est climatisé (iclim) si et seulement si un de ses émetteurs refroidit \
             (Th-BCE 2020 §8.1.2 : « Booléen indiquant si le groupe est refroidi »)",
        );
    }
    for (i, d) in g.distributions.iter().enumerate() {
        let cd = format!("{c}.distributions[{i}]");
        match bat
            .distributions_intergroupes
            .iter()
            .find(|dp| dp.id == d.distribution_intergroupe)
        {
            None => r.erreur(
                format!("{cd}.distribution_intergroupe"),
                Some(d.distribution_intergroupe.clone()),
                "réseau intergroupes inconnu du bâtiment",
            ),
            Some(dp) => {
                // Éq. (850) et (851).
                if dp.fonction != d.fonction {
                    r.erreur(
                        format!("{cd}.fonction"),
                        None,
                        "le réseau du groupe et le réseau intergroupes ont la même fonction \
                         (Th-BCE 2020 §8.7.3.1, éq. 850)",
                    );
                }
                let types_compatibles = matches!(
                    (&d.reseau, &dp.reseau),
                    (ReseauGroupe::Fictif, ReseauIntergroupes::Fictif)
                        | (
                            ReseauGroupe::Hydraulique(_),
                            ReseauIntergroupes::HydrauliqueCollectif(_)
                                | ReseauIntergroupes::HydrauliqueIndividuel(_)
                        )
                );
                if !types_compatibles {
                    r.erreur(
                        format!("{cd}.reseau"),
                        None,
                        "le réseau du groupe et le réseau intergroupes sont du même type, fictif \
                         ou hydraulique (Th-BCE 2020 §8.7.3.1, éq. 851)",
                    );
                }
            }
        }
    }
    for (i, d) in g.distributions.iter().enumerate() {
        if let ReseauGroupe::Hydraulique(h) = &d.reseau {
            valider_hydraulique_groupe(
                r,
                &format!("{c}.distributions[{i}].reseau.hydraulique"),
                h,
                d.fonction,
                bat,
            );
        }
    }
    let (mut somme_ch, mut somme_fr) = (0.0, 0.0);
    for (i, e) in g.emetteurs.iter().enumerate() {
        let ce = format!("{c}.emetteurs[{i}]");
        valider_emetteur(r, &ce, e, g);
        if let Some(ch) = &e.chauffage {
            somme_ch += ch.rats * ch.ratt;
        }
        if let Some(fr) = &e.refroidissement {
            somme_fr += fr.rats * fr.ratt;
        }
    }
    // §8.1.3.2 : « La somme des Rat^em doit toujours être plus petite ou égale à 1. »
    for (nom, somme) in [("chauffage", somme_ch), ("refroidissement", somme_fr)] {
        if somme > 1.0 + 1e-9 {
            r.erreur(
                format!("{c}.emetteurs"),
                Some(format!("{somme}")),
                format!(
                    "la somme des Rats × Ratt des émetteurs en {nom} ne peut dépasser 1 \
                     (Th-BCE 2020 §8.1.3.2)"
                ),
            );
        }
    }
    if somme_ch == 0.0 && g.emetteurs.iter().any(|e| e.chauffage.is_some()) {
        r.erreur(
            format!("{c}.emetteurs"),
            None,
            "les émetteurs de chauffage desservent une part nulle du groupe (Rats × Ratt)",
        );
    }
}

fn valider_emetteur(r: &mut RapportValidation, ce: &str, e: &Emetteur, g: &Groupe) {
    if e.chauffage.is_none() && e.refroidissement.is_none() {
        r.erreur(
            ce.to_owned(),
            None,
            "un émetteur a au moins une fonction, chauffage ou refroidissement",
        );
    }
    let distribution =
        |r: &mut RapportValidation, chemin: String, id: &str, f: FonctionReseau| match g
            .distributions
            .iter()
            .find(|d| d.id == id)
        {
            None => r.erreur(
                chemin,
                Some(id.to_owned()),
                "réseau de distribution inconnu du groupe",
            ),
            Some(d) if d.fonction != f => r.erreur(
                chemin,
                Some(id.to_owned()),
                "le réseau de distribution n'a pas la fonction de l'émetteur (Th-BCE 2020 §8.7.1)",
            ),
            Some(_) => {}
        };
    if let Some(ch) = &e.chauffage {
        let cc = format!("{ce}.chauffage");
        r.plage(format!("{cc}.rats"), ch.rats, 0.0, 1.0);
        r.plage(format!("{cc}.ratt"), ch.ratt, 0.0, 1.0);
        distribution(
            r,
            format!("{cc}.distribution"),
            &ch.distribution,
            FonctionReseau::Chauffage,
        );
        match ch.variation_spatiale {
            VariationSpatiale::Classe { hsp_m, .. } => {
                r.strictement_positif(format!("{cc}.variation_spatiale.classe.hsp_m"), hsp_m);
            }
            VariationSpatiale::PoeleOuInsert { niveaux } => {
                if !(1..=2).contains(&niveaux) {
                    r.erreur(
                        format!("{cc}.variation_spatiale.poele_ou_insert.niveaux"),
                        Some(niveaux.to_string()),
                        "1 ou 2 niveaux desservis (Th-BCE 2020 §8.1.3.1.2, tableau 83)",
                    );
                }
            }
            VariationSpatiale::Valeur { delta_theta_k } => r.plage(
                format!("{cc}.variation_spatiale.valeur.delta_theta_k"),
                delta_theta_k,
                f64::NEG_INFINITY,
                f64::INFINITY,
            ),
        }
        // §8.1.3.1.5 : une valeur saisie ne peut être inférieure à 0,2 K pour un émetteur à
        // effet Joule, à 0,4 K pour les autres émetteurs de chauffage.
        let min = if ch.effet_joule { 0.2 } else { 0.4 };
        match ch.variation_temporelle {
            VariationTemporelle::Certifie { delta_theta_k }
            | VariationTemporelle::Justifie { delta_theta_k } => r.plage(
                format!("{cc}.variation_temporelle"),
                delta_theta_k,
                min,
                f64::INFINITY,
            ),
            VariationTemporelle::Defaut { .. } | VariationTemporelle::PoeleOuInsert { .. } => {}
        }
    }
    if let Some(fr) = &e.refroidissement {
        let cf = format!("{ce}.refroidissement");
        r.plage(format!("{cf}.rats"), fr.rats, 0.0, 1.0);
        r.plage(format!("{cf}.ratt"), fr.ratt, 0.0, 1.0);
        distribution(
            r,
            format!("{cf}.distribution"),
            &fr.distribution,
            FonctionReseau::Refroidissement,
        );
        if let VariationSpatialeFroid::Classe { hsp_m, .. } = fr.variation_spatiale {
            r.strictement_positif(format!("{cf}.variation_spatiale.classe.hsp_m"), hsp_m);
        }
        // §8.1.3.1.5 : « Elle ne peut être supérieure à (−0,4 K) pour les émetteurs de
        // refroidissement. »
        match fr.variation_temporelle {
            VariationTemporelle::Certifie { delta_theta_k }
            | VariationTemporelle::Justifie { delta_theta_k } => r.plage(
                format!("{cf}.variation_temporelle"),
                delta_theta_k,
                f64::NEG_INFINITY,
                -0.4,
            ),
            VariationTemporelle::PoeleOuInsert { .. } => r.erreur(
                format!("{cf}.variation_temporelle"),
                None,
                "la valeur des poêles et inserts (tableau 88) ne vaut qu'en chauffage",
            ),
            VariationTemporelle::Defaut { .. } => {}
        }
    }
    if let Some(p) = &e.pertes_au_dos {
        r.positif(format!("{ce}.pertes_au_dos.r_i_m2k_w"), p.r_i_m2k_w);
        r.positif(format!("{ce}.pertes_au_dos.u_e_w_m2k"), p.u_e_w_m2k);
        // Éq. (795) et tableau 81 : Pper entre 0 et 1 ; à 1, l'éq. (810) divise par zéro.
        let p_per = p.r_i_m2k_w * p.u_e_w_m2k;
        if p_per >= 1.0 {
            r.erreur(
                format!("{ce}.pertes_au_dos"),
                Some(p_per.to_string()),
                "Pper = Ri·Ue doit rester inférieur à 1 (Th-BCE 2020 §8.1.3.1.4, éq. 795)",
            );
        }
    }
    if let Some(v) = &e.ventilateurs {
        let cv = format!("{ce}.ventilateurs");
        for (nom, x) in [
            ("p_gv_w", v.p_gv_w),
            ("p_mv_w", v.p_mv_w),
            ("p_pv_w", v.p_pv_w),
            ("q_recirc_gv_m3_h", v.q_recirc_gv_m3_h),
            ("q_recirc_mv_m3_h", v.q_recirc_mv_m3_h),
            ("q_recirc_pv_m3_h", v.q_recirc_pv_m3_h),
        ] {
            r.positif(format!("{cv}.{nom}"), x);
        }
        match (v.super_petite_vitesse, v.p_spv_w) {
            (true, None) => r.erreur(
                format!("{cv}.p_spv_w"),
                None,
                "puissance en super petite vitesse requise avec un régime de super petite \
                 vitesse (Th-BCE 2020 §8.1.2)",
            ),
            (_, Some(p)) => r.positif(format!("{cv}.p_spv_w"), p),
            _ => {}
        }
        if e.refroidissement.is_some() && v.regulation_batterie.is_none() {
            r.erreur(
                format!("{cv}.regulation_batterie"),
                None,
                "régulation de la batterie froide requise pour un émetteur de froid à \
                 ventilateurs (« idregul_batt », Th-BCE 2020 §8.1.2)",
            );
        }
    }
}

fn espace_tampon_connu(r: &mut RapportValidation, c: &str, et: Option<&str>, bat: &Batiment) {
    if let Some(id) = et {
        if !bat.espaces_tampons.iter().any(|e| e.id == id) {
            r.erreur(
                format!("{c}.espace_tampon"),
                Some(id.to_owned()),
                "espace tampon inconnu du bâtiment",
            );
        }
    }
}

/// Paramètres d'un réseau hydraulique du groupe (§8.7.2 et §8.8.2).
fn valider_hydraulique_groupe(
    r: &mut RapportValidation,
    c: &str,
    h: &ReseauHydrauliqueGroupe,
    fonction: FonctionReseau,
    bat: &Batiment,
) {
    for (nom, x) in [
        ("l_vc_m", h.l_vc_m),
        ("l_hvc_m", h.l_hvc_m),
        ("u_vc_w_mk", h.u_vc_w_mk),
        ("u_hvc_w_mk", h.u_hvc_w_mk),
        ("p_aux_w", h.p_aux_w),
        ("q_nom_m3_h", h.q_nom_m3_h),
    ] {
        r.positif(format!("{c}.{nom}"), x);
    }
    espace_tampon_connu(r, c, h.espace_tampon.as_deref(), bat);
    // L'écart nominal est positif en chauffage, négatif en refroidissement (note du §8.9.3.5.3).
    let signe_ok = match fonction {
        FonctionReseau::Chauffage => h.delta_theta_em_dim_k > 0.0,
        FonctionReseau::Refroidissement => h.delta_theta_em_dim_k < 0.0,
    };
    if !signe_ok {
        r.erreur(
            format!("{c}.delta_theta_em_dim_k"),
            Some(h.delta_theta_em_dim_k.to_string()),
            "écart nominal départ-retour strictement positif en chauffage, strictement négatif \
             en refroidissement (Th-BCE 2020 §8.7.2 et §8.9.3.5.3)",
        );
    }
    match h.regulation_temperature {
        RegulationTemperature::LoiDEau if fonction == FonctionReseau::Refroidissement => r.erreur(
            format!("{c}.regulation_temperature"),
            None,
            "la loi d'eau n'existe qu'en chauffage (Th-BCE 2020 §8.7.1)",
        ),
        RegulationTemperature::DepartConstant | RegulationTemperature::LoiDEau
            if h.theta_dep_dim_c.is_none() =>
        {
            r.erreur(
                format!("{c}.theta_dep_dim_c"),
                None,
                "température de départ de dimensionnement requise (Th-BCE 2020 §8.7.2)",
            );
        }
        RegulationTemperature::RetourConstant if h.theta_ret_dim_c.is_none() => r.erreur(
            format!("{c}.theta_ret_dim_c"),
            None,
            "température de retour de dimensionnement requise (Th-BCE 2020 §8.7.2)",
        ),
        _ => {}
    }
    let variable = h.regulation_debit == RegulationDebit::DebitVariable;
    if variable && h.q_resid_m3_h.is_none() {
        r.erreur(
            format!("{c}.q_resid_m3_h"),
            None,
            "débit résiduel requis pour un réseau à débit variable (Th-BCE 2020 §8.7.2)",
        );
    }
    if let Some(q) = h.q_resid_m3_h {
        r.positif(format!("{c}.q_resid_m3_h"), q);
    }
    if !variable && h.q_nom_m3_h <= 0.0 {
        r.erreur(
            format!("{c}.q_nom_m3_h"),
            Some(h.q_nom_m3_h.to_string()),
            "débit nominal strictement positif requis pour un réseau à débit constant (éq. 856 \
             et 857)",
        );
    }
    // §8.7.3.3.2.7 : « Les circulateurs à vitesse variable ne sont pris en compte que dans le
    // cadre d'un réseau à débit variable. »
    if !variable
        && matches!(
            h.circulateur,
            Circulateur::PressionConstante | Circulateur::PressionVariable
        )
    {
        r.erreur(
            format!("{c}.circulateur"),
            None,
            "circulateur à vitesse variable réservé aux réseaux à débit variable (Th-BCE 2020 \
             §8.7.3.3.2.7)",
        );
    }
}

/// Émetteurs d'ECS d'un groupe (§9.5 à 9.7). Hors habitation, un bâtiment sans production
/// d'ECS se décrit avec la production conventionnelle de la fiche d'application 9.24.
fn valider_ecs_groupe(
    r: &mut RapportValidation,
    c: &str,
    g: &Groupe,
    bat: &Batiment,
    ids_ecs: &BTreeSet<&str>,
    usage: UsageZone,
) {
    let habitation = usage.est_habitation();
    if g.emetteurs_ecs.is_empty() {
        r.erreur(
            format!("{c}.emetteurs_ecs"),
            None,
            "au moins un émetteur d'ECS requis (Th-BCE 2020 §9.5 et §9.6 ; pour un bâtiment tertiaire \
             sans production d'ECS, décrire la production conventionnelle du §9.24)",
        );
        return;
    }
    r.ids_uniques(
        &format!("{c}.emetteurs_ecs"),
        g.emetteurs_ecs.iter().map(|e| &e.id),
    );
    // Éq. (1673) : les surfaces desservies couvrent le groupe.
    let somme: f64 = g.emetteurs_ecs.iter().map(|e| e.rat_em).sum();
    if (somme - 1.0).abs() > 1e-6 {
        r.erreur(
            format!("{c}.emetteurs_ecs"),
            Some(somme.to_string()),
            "la somme des « Ratem » des émetteurs d'ECS du groupe vaut 1 (Th-BCE 2020 §9.5.3, \
             éq. 1673)",
        );
    }
    for (i, e) in g.emetteurs_ecs.iter().enumerate() {
        let ce = format!("{c}.emetteurs_ecs[{i}]");
        r.plage(format!("{ce}.rat_em"), e.rat_em, 0.0, 1.0);
        if habitation {
            if e.nb_logements == 0 {
                r.erreur(
                    format!("{ce}.nb_logements"),
                    Some("0".into()),
                    "doit être supérieur ou égal à 1 (Th-BCE 2020 §9.5.3, tableau 272)",
                );
            }
            if e.nu.is_some() {
                r.erreur(
                    format!("{ce}.nu"),
                    e.nu.map(|n| n.to_string()),
                    "le nombre d'unités ne se saisit pas en maison individuelle ou accolée ni en \
                     logement collectif : il se déduit des adultes équivalents (Th-BCE 2020 \
                     §9.6.3.2.2.2)",
                );
            }
        } else {
            if e.nb_logements != 0 {
                r.erreur(
                    format!("{ce}.nb_logements"),
                    Some(e.nb_logements.to_string()),
                    "le nombre de logements desservis ne se saisit qu'en maison individuelle ou \
                     accolée et en logement collectif (Th-BCE 2020 §9.5.2)",
                );
            }
            if let Some(nu) = e.nu {
                r.strictement_positif(format!("{ce}.nu"), nu);
            }
        }
        let parts = [
            ("part_melangeur", e.part_melangeur),
            ("part_mitigeur", e.part_mitigeur),
            ("part_temporisateur", e.part_temporisateur),
        ];
        for (nom, p) in parts {
            r.plage(format!("{ce}.{nom}"), p, 0.0, 1.0);
        }
        // Éq. (1675) : « La somme des trois pourcentages de M_partem doit être égale à 1. »
        let total: f64 = parts.iter().map(|(_, p)| p).sum();
        if (total - 1.0).abs() > 1e-6 {
            r.erreur(
                ce.clone(),
                Some(total.to_string()),
                "la somme des parts des trois catégories d'émetteurs d'ECS vaut 1 (Th-BCE 2020 \
                 §9.5.3, éq. 1675)",
            );
        }
        valider_recuperateur(r, &ce, e, bat, usage);
        let d = &e.distribution;
        let cd = format!("{ce}.distribution");
        r.strictement_positif(format!("{cd}.d_int_m"), d.d_int_m);
        r.positif(format!("{cd}.l_hvc_m"), d.l_hvc_m);
        if let Some(l) = d.l_vc_m {
            r.positif(format!("{cd}.l_vc_m"), l);
        }
        if d.nb_dist == 0 {
            r.erreur(
                format!("{cd}.nb_dist"),
                Some("0".into()),
                "doit être supérieur ou égal à 1",
            );
        }
        // §9.5.3 et §9.7.3 : message d'erreur si la distribution est plus froide que l'eau
        // mitigée.
        if let Some(t) = d.theta_2nd_e_c {
            if t < crate::ecs::besoins_ecs::THETA_UW_C {
                r.erreur(
                    format!("{cd}.theta_2nd_e_c"),
                    Some(t.to_string()),
                    "la température de la distribution d'ECS est au moins celle de l'eau \
                     mitigée, 40 °C (Th-BCE 2020 §9.5.3, éq. 1679, et §9.7.3)",
                );
            }
        }
        espace_tampon_connu(r, &cd, d.espace_tampon.as_deref(), bat);
        if !ids_ecs.contains(e.distribution_intergroupe.as_str()) {
            r.erreur(
                format!("{ce}.distribution_intergroupe"),
                Some(e.distribution_intergroupe.clone()),
                "distribution d'ECS intergroupe inconnue du bâtiment (Th-BCE 2020 §9.7.3)",
            );
        }
    }
}

/// Récupérateur de chaleur sur les eaux grises d'un émetteur d'ECS (§9.21.2 à 9.21.7).
fn valider_recuperateur(
    r: &mut RapportValidation,
    ce: &str,
    e: &EmetteurEcs,
    bat: &Batiment,
    usage: UsageZone,
) {
    let cr = format!("{ce}.recuperateur");
    match (e.recuperateur_eaux_grises, &e.recuperateur) {
        (Some(true), None) => r.erreur(
            format!("{ce}.recuperateur_eaux_grises"),
            Some("true".into()),
            "« Irecup » = 1 exige la description du récupérateur (`recuperateur`, Th-BCE 2020 \
             §9.21.2)",
        ),
        (Some(false), Some(_)) => r.erreur(
            cr.clone(),
            None,
            "un récupérateur est décrit alors que « Irecup » vaut 0 (Th-BCE 2020 §9.21.2)",
        ),
        _ => {}
    }
    let Some(rec) = &e.recuperateur else { return };
    let individuelle = rec.configuration == ConfigurationRecuperateur::Individuelle;
    match (individuelle, rec.position, &rec.recuperateur_collectif) {
        (true, None, _) => r.erreur(
            format!("{cr}.position"),
            None,
            "position du récupérateur requise en récupération individuelle (Th-BCE 2020 \
             §9.21.2.1, « Tposition »)",
        ),
        (true, _, Some(id)) => r.erreur(
            format!("{cr}.recuperateur_collectif"),
            Some(id.clone()),
            "un récupérateur collectif ne se désigne qu'en récupération collective (Th-BCE 2020 \
             §9.21.2.1)",
        ),
        (false, Some(_), _) => r.erreur(
            format!("{cr}.position"),
            None,
            "en récupération collective la position est imposée, en amont du ballon d'ECS \
             uniquement (figure 205 : « Typerecup = 0 implique Tposition = 1 »)",
        ),
        (false, None, None) => r.erreur(
            format!("{cr}.recuperateur_collectif"),
            None,
            "identifiant du récupérateur collectif requis en récupération collective (Th-BCE \
             2020 §9.21.2.1)",
        ),
        (false, None, Some(id)) if !bat.recuperateurs_eaux_grises.iter().any(|c| &c.id == id) => {
            r.erreur(
                format!("{cr}.recuperateur_collectif"),
                Some(id.clone()),
                "récupérateur collectif inconnu du bâtiment (`recuperateurs_eaux_grises`, \
                 Th-BCE 2020 §9.21.5)",
            );
        }
        _ => {}
    }
    if rec.unites.is_empty() {
        r.erreur(
            format!("{cr}.unites"),
            None,
            "au moins une configuration unitaire requise (Th-BCE 2020 §9.21.2.1)",
        );
    }
    for (i, u) in rec.unites.iter().enumerate() {
        let cu = format!("{cr}.unites[{i}]");
        if u.nb_echangeurs == 0 {
            r.erreur(
                format!("{cu}.nb_echangeurs"),
                Some("0".into()),
                "au moins un échangeur par configuration unitaire (Th-BCE 2020 §9.21.2.3)",
            );
        }
        if u.nb_douches_relies == 0 {
            r.erreur(
                format!("{cu}.nb_douches_relies"),
                Some("0".into()),
                "au moins une douche ou baignoire reliée à l'échangeur (Th-BCE 2020 §9.21.2.3)",
            );
        }
        match (&u.echangeur, individuelle) {
            (None, true) => {}
            (Some(x), true) => valider_echangeur_eaux_grises(r, &format!("{cu}.echangeur"), x),
            (Some(_), false) => r.erreur(
                format!("{cu}.echangeur"),
                None,
                "en récupération collective l'échangeur est celui du récupérateur collectif \
                 (Th-BCE 2020 §9.21.5 et §9.21.6.3.1)",
            ),
            (None, false) => {}
        }
        if let Some(p) = u.p_relevage_nominale_w {
            r.strictement_positif(format!("{cu}.p_relevage_nominale_w"), p);
        }
    }
    r.positif(format!("{cr}.l_evac_vc_m"), rec.l_evac_vc_m);
    r.positif(format!("{cr}.l_evac_vnc_m"), rec.l_evac_vnc_m);
    verifier_conduit_eaux_grises(r, &cr, rec.l_evac_vc_m, rec.l_evac_vnc_m);
    if crate::ecs::besoins_ecs::rat_douches_bains(usage) == 0.0 {
        r.avertissement(
            cr,
            None,
            "aucun besoin d'ECS n'est dédié aux douches ou aux bains dans cet usage (« Ratdouches-\
             bains » = 0, tableau 274) : le récupérateur est sans effet (Th-BCE 2020 §9.21.2.4.1)",
        );
    }
}

/// Efficacité d'un échangeur sur les eaux grises (§9.21.6.3.2.1).
fn valider_echangeur_eaux_grises(r: &mut RapportValidation, c: &str, x: &EchangeurEauxGrises) {
    if let Some(e) = x.efficacite {
        r.plage(format!("{c}.efficacite"), e, 0.0, 1.0);
    }
    if let Some(t) = x.c_trans {
        r.plage(format!("{c}.c_trans"), t, 0.0, 1.0);
    }
    if x.origine.is_none() && (x.efficacite.is_some() || x.c_trans.is_some()) {
        r.erreur(
            c.to_owned(),
            None,
            "l'origine des valeurs (certifiée, justifiée ou déclarée) est requise avec \
             l'efficacité et `c_trans` (Th-BCE 2020 §9.21.6.3.2.1)",
        );
    }
    if x.origine.is_some() && (x.efficacite.is_none() || x.c_trans.is_none()) {
        r.avertissement(
            c.to_owned(),
            None,
            "efficacité ou `c_trans` absent : valeur par défaut εech_eg_fixe = 0,8 × 0,15 \
             (Th-BCE 2020 §9.21.6.3.2.1)",
        );
    }
}

/// Conduit d'évacuation des eaux grises : longueur non nulle et perte inférieure à 100 %
/// (éq. 2196 et 2199).
fn verifier_conduit_eaux_grises(r: &mut RapportValidation, c: &str, vc: f64, vnc: f64) {
    if vc + vnc <= 0.0 || (vc + vnc).is_nan() {
        r.erreur(
            c.to_owned(),
            None,
            "la longueur du conduit d'évacuation des eaux grises doit être non nulle (éq. 2196)",
        );
    } else if 0.16 / 100.0 * vc + 0.30 / 100.0 * vnc > 1.0 {
        r.erreur(
            c.to_owned(),
            None,
            "conduit d'évacuation des eaux grises trop long : coefficient de pertes Fam négatif \
             (éq. 2199)",
        );
    }
}
