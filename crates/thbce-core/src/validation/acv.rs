// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des données de l'ACV d'un bâtiment (annexe II de l'arrêté du 4 août 2021,
//! chapitre 4) : nomenclature des lots (tableau 5), données environnementales saisies,
//! valeurs forfaitaires disponibles pour l'usage (annexe XI), données de service exigées par
//! chaque calcul.

use super::RapportValidation;
use crate::acv::donnees::{forfait, lot_de, lot_forfait, releve_de};
use crate::acv::q_conv_batiment;
use crate::entree::{
    Assainissement, Batiment, ChantierAcv, ComposantAcv, DonneesAcv, DonneesService,
    GestionEauxPluviales, OccupantsAcv, TerresEvacuees, TerresImportees, UsageZone,
};

fn exiger(r: &mut RapportValidation, c: &str, v: Option<f64>, nom: &str, pourquoi: &str) {
    match v {
        None => r.erreur(
            format!("{c}.donnees_service.{nom}"),
            None,
            format!("donnée de service obligatoire pour {pourquoi}"),
        ),
        Some(x) => r.plage(format!("{c}.donnees_service.{nom}"), x, -1e9, 1e9),
    }
}

fn valider_composant(
    r: &mut RapportValidation,
    c: &str,
    x: &ComposantAcv,
    parcelle: bool,
    avec_cogeneration: bool,
) {
    if !parcelle && lot_de(&x.sous_lot).is_none() {
        r.erreur(
            format!("{c}.sous_lot"),
            Some(x.sous_lot.clone()),
            "sous-lot inconnu (annexe II, tableau 5 : 1.1 à 11.3, 12, 13)",
        );
    }
    // Annexe II, éq. (16) : composants du sous-lot 8.2 affectés au prorata du taux
    // d'autoconsommation global de la cogénération, qui demande un cogénérateur (fiche 8.21).
    if !parcelle && x.sous_lot == "8.2" && !avec_cogeneration {
        r.erreur(
            format!("{c}.sous_lot"),
            Some(x.sous_lot.clone()),
            "sous-lot 8.2 (installation de cogénération) sans cogénérateur dans les générations \
             du bâtiment : le taux d'autoconsommation global TAC_cogé (annexe II, éq. 16) n'est \
             pas défini",
        );
    }
    r.positif(format!("{c}.quantite"), x.quantite);
    let d = &x.donnee;
    if d.identifiant.trim().is_empty() || d.source.trim().is_empty() {
        r.erreur(
            format!("{c}.donnee"),
            None,
            "identifiant et source de la donnée environnementale obligatoires",
        );
    }
    r.plage(format!("{c}.donnee.dvr_ans"), d.dvr_ans, 1.0, 1000.0);
    for (n, v) in [
        ("production_kgco2", d.production_kgco2),
        ("edification_kgco2", d.edification_kgco2),
        ("exploitation_kgco2", d.exploitation_kgco2),
        ("fin_de_vie_kgco2", d.fin_de_vie_kgco2),
        ("module_d_kgco2", d.module_d_kgco2),
    ] {
        r.plage(format!("{c}.donnee.{n}"), v, -1e9, 1e9);
    }
    if let Some(dve) = x.dve_ans {
        // §4.2.1.1.1 : la durée de vie ne peut pas être augmentée.
        r.plage(format!("{c}.dve_ans"), dve, 1.0, d.dvr_ans.max(1.0));
    }
    if let Some(f) = &x.fluide {
        r.positif(format!("{c}.fluide.charge_kg"), f.charge_kg);
        r.positif(format!("{c}.fluide.prg_kgco2_kg"), f.prg_kgco2_kg);
        if d.b1_kgco2.is_none() && f.de_kgco2_kg.is_none() {
            r.erreur(
                format!("{c}.fluide.de_kgco2_kg"),
                None,
                "obligatoire sans module B1 séparé (cas 1, éq. 26)",
            );
        }
    }
}

fn valider_terres(
    r: &mut RapportValidation,
    c: &str,
    imp: &[TerresImportees],
    evac: &[TerresEvacuees],
    distance_obligatoire: bool,
) -> bool {
    for (i, t) in imp.iter().enumerate() {
        r.positif(
            format!("{c}.terres_importees[{i}].quantite_t"),
            t.quantite_t,
        );
        match t.distance_km {
            Some(dk) => r.positif(format!("{c}.terres_importees[{i}].distance_km"), dk),
            None if distance_obligatoire => r.erreur(
                format!("{c}.terres_importees[{i}].distance_km"),
                None,
                "distance obligatoire en calcul détaillé (donnée réelle du projet)",
            ),
            None => {}
        }
    }
    for (i, t) in evac.iter().enumerate() {
        r.positif(format!("{c}.terres_evacuees[{i}].quantite_t"), t.quantite_t);
        r.positif(
            format!("{c}.terres_evacuees[{i}].distance_km"),
            t.distance_km,
        );
    }
    !imp.is_empty() || !evac.is_empty()
}

fn valider_chantier(
    r: &mut RapportValidation,
    c: &str,
    ch: &ChantierAcv,
    s: &DonneesService,
    maison: bool,
) {
    let cc = format!("{c}.chantier");
    let eau = |r: &mut RapportValidation| {
        exiger(
            r,
            c,
            s.eau_potable_kgco2_m3,
            "eau_potable_kgco2_m3",
            "l'eau du chantier",
        );
        exiger(
            r,
            c,
            s.assainissement_collectif_kgco2_m3,
            "assainissement_collectif_kgco2_m3",
            "les eaux usées du chantier",
        );
    };
    match ch {
        ChantierAcv::Detaille {
            energies,
            eau_potable_m3,
            eaux_usees_m3,
            terres_importees,
            terres_evacuees,
            composants,
        } => {
            for (i, e) in energies.iter().enumerate() {
                r.positif(format!("{cc}.energies[{i}].quantite"), e.quantite);
                r.plage(
                    format!("{cc}.energies[{i}].de_kgco2_unite"),
                    e.de_kgco2_unite,
                    -1e9,
                    1e9,
                );
            }
            r.positif(format!("{cc}.eau_potable_m3"), *eau_potable_m3);
            r.positif(format!("{cc}.eaux_usees_m3"), *eaux_usees_m3);
            if *eau_potable_m3 > 0.0 || *eaux_usees_m3 > 0.0 {
                eau(r);
            }
            if valider_terres(r, &cc, terres_importees, terres_evacuees, true) {
                exiger(
                    r,
                    c,
                    s.transport_terres_kgco2_tkm,
                    "transport_terres_kgco2_tkm",
                    "les terres",
                );
            }
            for (i, x) in composants.iter().enumerate() {
                valider_composant(r, &format!("{cc}.composants[{i}]"), x, true, false);
            }
        }
        ChantierAcv::SimplifieMaison {
            s_parcelle_m2,
            s_emprise_sol_m2,
            terres_importees,
        } => {
            if !maison {
                r.erreur(
                    cc.clone(),
                    None,
                    "calcul simplifié des maisons (§4.5.2.1.1) réservé aux bâtiments dont \
                     toutes les zones sont des maisons individuelles ou accolées",
                );
            }
            r.strictement_positif(format!("{cc}.s_parcelle_m2"), *s_parcelle_m2);
            r.positif(format!("{cc}.s_emprise_sol_m2"), *s_emprise_sol_m2);
            eau(r);
            let imp = valider_terres(r, &cc, terres_importees, &[], false);
            if *s_parcelle_m2 <= 500.0 {
                exiger(
                    r,
                    c,
                    s.traitement_inertes_kgco2_t,
                    "traitement_inertes_kgco2_t",
                    "les terres évacuées (parcelle de 500 m² au plus, éq. 101)",
                );
            }
            if imp || *s_parcelle_m2 <= 500.0 {
                exiger(
                    r,
                    c,
                    s.transport_terres_kgco2_tkm,
                    "transport_terres_kgco2_tkm",
                    "les terres",
                );
            }
        }
        ChantierAcv::SimplifieAutres {
            n_ete_grue,
            n_hiver_grue,
            n_ete_sans_grue,
            n_hiver_sans_grue,
            v_terres_excavees_m3,
            epsilon1_l_m3,
            terres_importees,
            terres_evacuees,
        } => {
            for (n, v) in [
                ("n_ete_grue", n_ete_grue),
                ("n_hiver_grue", n_hiver_grue),
                ("n_ete_sans_grue", n_ete_sans_grue),
                ("n_hiver_sans_grue", n_hiver_sans_grue),
                ("v_terres_excavees_m3", v_terres_excavees_m3),
            ] {
                r.positif(format!("{cc}.{n}"), *v);
            }
            if let Some(e) = epsilon1_l_m3 {
                r.positif(format!("{cc}.epsilon1_l_m3"), *e);
            }
            eau(r);
            exiger(
                r,
                c,
                s.electricite_chantier_kgco2_kwh,
                "electricite_chantier_kgco2_kwh",
                "l'électricité du chantier (éq. 102)",
            );
            if *v_terres_excavees_m3 > 0.0 {
                exiger(
                    r,
                    c,
                    s.carburant_kgco2_l,
                    "carburant_kgco2_l",
                    "le carburant (éq. 103)",
                );
            }
            if valider_terres(r, &cc, terres_importees, terres_evacuees, false) {
                exiger(
                    r,
                    c,
                    s.transport_terres_kgco2_tkm,
                    "transport_terres_kgco2_tkm",
                    "les terres",
                );
            }
        }
    }
}

fn valider_eau(r: &mut RapportValidation, c: &str, a: &DonneesAcv, bat: &Batiment) {
    let zones: Vec<(u8, f64)> = bat.zones.iter().map(|z| (z.usage.numero(), 0.0)).collect();
    let q_conv = q_conv_batiment(&zones);
    match (q_conv, a.eau.q_eau_potable_saisie_m3) {
        (None, None) => r.erreur(
            format!("{c}.eau.q_eau_potable_saisie_m3"),
            None,
            "consommation d'eau potable à saisir (m³/an) : le tableau 7 de l'annexe II est « N/C » \
             pour un usage des zones, ou les zones ont des consommations conventionnelles \
             différentes (Q218)",
        ),
        (None, Some(q)) => {
            r.positif(format!("{c}.eau.q_eau_potable_saisie_m3"), q);
            r.avertissement(
                format!("{c}.eau.q_eau_potable_saisie_m3"),
                Some(q.to_string()),
                "consommation d'eau potable saisie, hors consommation conventionnelle du \
                 tableau 7 (« N/C » ou zones de consommations différentes) : Q218",
            );
        }
        (Some(_), Some(q)) => r.avertissement(
            format!("{c}.eau.q_eau_potable_saisie_m3"),
            Some(q.to_string()),
            "ignorée : le tableau 7 donne une consommation conventionnelle commune aux zones",
        ),
        (Some(_), None) => {}
    }
    let s = &a.donnees_service;
    exiger(
        r,
        c,
        s.eau_potable_kgco2_m3,
        "eau_potable_kgco2_m3",
        "l'eau du bâtiment",
    );
    match a.eau.assainissement {
        Assainissement::Collectif => exiger(
            r,
            c,
            s.assainissement_collectif_kgco2_m3,
            "assainissement_collectif_kgco2_m3",
            "l'assainissement collectif",
        ),
        Assainissement::NonCollectif => exiger(
            r,
            c,
            s.assainissement_non_collectif_kgco2_m3,
            "assainissement_non_collectif_kgco2_m3",
            "l'assainissement non collectif",
        ),
    }
    if a.eau.eaux_pluviales == GestionEauxPluviales::Unitaire {
        for (n, v) in [
            ("pluviometrie_mm", a.eau.pluviometrie_mm),
            ("s_toitures_m2", a.eau.s_toitures_m2),
        ] {
            match v {
                Some(x) => r.positif(format!("{c}.eau.{n}"), x),
                None => r.erreur(
                    format!("{c}.eau.{n}"),
                    None,
                    "obligatoire pour un réseau unitaire (éq. 85)",
                ),
            }
        }
        exiger(
            r,
            c,
            s.assainissement_collectif_kgco2_m3,
            "assainissement_collectif_kgco2_m3",
            "les eaux pluviales en réseau unitaire",
        );
    }
    let residentiel = bat.zones.iter().all(|z| z.usage.est_habitation());
    match &a.occupants {
        OccupantsAcv::Logements(t) => {
            if !residentiel {
                r.erreur(
                    format!("{c}.occupants"),
                    None,
                    "Nocc par types de logements réservé à l'habitation (§5.2.3)",
                );
            }
            if t.iter().all(|x| x.nombre == 0) || t.iter().any(|x| x.type_t == 0) {
                r.erreur(
                    format!("{c}.occupants"),
                    None,
                    "au moins un logement, de type T1 ou plus",
                );
            }
        }
        OccupantsAcv::Nombre(n) => {
            if residentiel {
                r.erreur(
                    format!("{c}.occupants"),
                    None,
                    "en habitation, Nocc se déduit des types de logements (éq. 131)",
                );
            }
            r.strictement_positif(format!("{c}.occupants"), *n);
        }
    }
}

/// Valide les données de l'ACV d'un bâtiment.
pub(super) fn valider_acv(r: &mut RapportValidation, cb: &str, bat: &Batiment) {
    let Some(a) = &bat.acv else {
        return;
    };
    let c = format!("{cb}.acv");
    r.ids_uniques(
        &format!("{c}.composants"),
        a.composants.iter().map(|x| &x.id),
    );
    let avec_cogeneration = bat.generations.iter().any(|g| {
        g.generateurs
            .iter()
            .any(|x| matches!(x.nature, crate::entree::NatureGenerateur::Cogeneration(_)))
    });
    for (i, x) in a.composants.iter().enumerate() {
        valider_composant(
            r,
            &format!("{c}.composants[{i}]"),
            x,
            false,
            avec_cogeneration,
        );
        if let Some(k) = a
            .lots_forfaitaires
            .iter()
            .find(|k| releve_de(&x.sous_lot, k))
        {
            r.erreur(
                format!("{c}.composants[{i}].sous_lot"),
                Some(x.sous_lot.clone()),
                format!("le lot {k} est pris en compte par sa valeur forfaitaire"),
            );
        }
    }
    if a.composants.iter().any(|x| x.sous_lot == "13")
        && (bat.installations_pv.is_empty() || bat.generations.is_empty())
    {
        r.erreur(
            format!("{c}.composants"),
            None,
            "lot 13 : le taux d'autoconsommation de la production photovoltaïque (éq. 22) \
             demande une installation photovoltaïque et des générations",
        );
    }
    for (i, k) in a.lots_forfaitaires.iter().enumerate() {
        let ck = format!("{c}.lots_forfaitaires[{i}]");
        if lot_forfait(k).is_none() {
            r.erreur(
                ck,
                Some(k.clone()),
                "lot forfaitaire : 8.1, 8.3, 8.4, 8.5, 10 ou 11",
            );
            continue;
        }
        if a.lots_forfaitaires[..i].contains(k) {
            r.erreur(ck.clone(), Some(k.clone()), "lot forfaitaire en double");
        }
        for z in &bat.zones {
            if forfait(z.usage.numero(), k).is_none() {
                r.erreur(
                    ck.clone(),
                    Some(k.clone()),
                    format!(
                        "l'annexe XI ne donne pas de valeur forfaitaire pour l'usage de la zone {}",
                        z.id
                    ),
                );
            }
        }
    }
    for (i, f) in a.fluides_lots_forfaitaires.iter().enumerate() {
        let cf = format!("{c}.fluides_lots_forfaitaires[{i}]");
        if !a.lots_forfaitaires.contains(&f.sous_lot) {
            r.erreur(
                format!("{cf}.sous_lot"),
                Some(f.sous_lot.clone()),
                "lot non déclaré forfaitaire",
            );
        }
        r.positif(format!("{cf}.charge_kg"), f.charge_kg);
        r.positif(format!("{cf}.prg_kgco2_kg"), f.prg_kgco2_kg);
        r.positif(format!("{cf}.de_kgco2_kg"), f.de_kgco2_kg);
        if let Some(d) = f.dve_ans {
            r.plage(format!("{cf}.dve_ans"), d, 1.0, 1000.0);
        }
    }
    let maison = bat
        .zones
        .iter()
        .all(|z| z.usage == UsageZone::MaisonIndividuelle);
    valider_chantier(r, &c, &a.chantier, &a.donnees_service, maison);
    valider_eau(r, &c, a, bat);
    if let Some(p) = &bat.exigences {
        if !p.depot_a_partir_de(2026, 5) {
            r.avertissement(
                "exigences.annee_depot_permis".to_owned(),
                Some(p.annee_depot_permis.to_string()),
                "permis déposé avant le 1er mai 2026 : Icconstruction_max est calculé avec la \
                 version post-GTM2 de l'annexe de l'article R. 172-4 (seule versionnée), non \
                 avec la rédaction en vigueur à la date du dépôt (Q213)",
            );
        }
    }
    if let Some(p) = &a.parcelle {
        let cp = format!("{c}.parcelle");
        for (i, x) in p.composants.iter().enumerate() {
            valider_composant(r, &format!("{cp}.composants[{i}]"), x, true, false);
        }
        if valider_terres(r, &format!("{cp}.comblements"), &p.comblements, &[], true) {
            let s = &a.donnees_service;
            exiger(
                r,
                &c,
                s.transport_terres_kgco2_tkm,
                "transport_terres_kgco2_tkm",
                "le comblement",
            );
            exiger(
                r,
                &c,
                s.carburant_kgco2_l,
                "carburant_kgco2_l",
                "le comblement (éq. 106)",
            );
        }
        for (n, v) in [
            ("s_vegetalisee_m2", p.s_vegetalisee_m2),
            ("s_nettoyee_m2", p.s_nettoyee_m2),
            ("s_impermeabilisees_m2", p.s_impermeabilisees_m2),
        ] {
            r.positif(format!("{cp}.{n}"), v);
        }
        if p.s_impermeabilisees_m2 > 0.0 && a.eau.pluviometrie_mm.is_none() {
            r.erreur(
                format!("{c}.eau.pluviometrie_mm"),
                None,
                "obligatoire pour les eaux pluviales de la parcelle (éq. 112)",
            );
        }
    }
}
