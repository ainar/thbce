// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des systèmes de chauffage et de refroidissement (Th-BCE 2020 §8.1 à 8.10) :
//! plages des nomenclatures, références entre émetteurs, réseaux et générations, cohérence
//! d'`iclim`, et règles de saisie des fiches d'application (§8.2, §8.13).

use crate::entree::{
    Batiment, Emetteur, FonctionReseau, Groupe, ReseauGroupe, ReseauIntergroupes,
    VariationSpatiale, VariationSpatialeFroid, VariationTemporelle,
};

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
        if !gens.contains(d.generation.as_str()) {
            r.erreur(
                format!("{cb}.distributions_intergroupes[{i}].generation"),
                Some(d.generation.clone()),
                "génération inconnue du bâtiment (un réseau intergroupes est relié à une et une \
                 seule génération, Th-BCE 2020 §8.10.1)",
            );
        }
    }
    for (iz, zone) in bat.zones.iter().enumerate() {
        if !zone.usage.est_habitation() {
            continue;
        }
        for (ig, g) in zone.groupes.iter().enumerate() {
            valider_systemes_groupe(r, &format!("{cb}.zones[{iz}].groupes[{ig}]"), g, bat);
        }
    }
}

fn valider_systemes_groupe(r: &mut RapportValidation, c: &str, g: &Groupe, bat: &Batiment) {
    let Some(iclim) = g.iclim else {
        r.erreur(
            format!("{c}.iclim"),
            None,
            "« iclim » requis en habitation : vrai si le groupe est climatisé (Th-BCE 2020 \
             §8.1.2)",
        );
        return;
    };
    match &g.programmation {
        None => r.erreur(
            format!("{c}.programmation"),
            None,
            "programmation des relances requise en habitation (Th-BCE 2020 §8.5)",
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
            "au moins un émetteur de chauffage requis en habitation (Th-BCE 2020 §8.1)",
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
            .find(|dp| dp.id == d.distribution_intergroupes)
        {
            None => r.erreur(
                format!("{cd}.distribution_intergroupes"),
                Some(d.distribution_intergroupes.clone()),
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
            VariationSpatiale::PoeleInsert { niveaux } => {
                if !(1..=2).contains(&niveaux) {
                    r.erreur(
                        format!("{cc}.variation_spatiale.poele_insert.niveaux"),
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
            VariationTemporelle::Certifiee { delta_theta_k }
            | VariationTemporelle::Justifiee { delta_theta_k } => r.plage(
                format!("{cc}.variation_temporelle"),
                delta_theta_k,
                min,
                f64::INFINITY,
            ),
            VariationTemporelle::Defaut { .. } | VariationTemporelle::PoeleInsert { .. } => {}
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
            VariationTemporelle::Certifiee { delta_theta_k }
            | VariationTemporelle::Justifiee { delta_theta_k } => r.plage(
                format!("{cf}.variation_temporelle"),
                delta_theta_k,
                f64::NEG_INFINITY,
                -0.4,
            ),
            VariationTemporelle::PoeleInsert { .. } => r.erreur(
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
