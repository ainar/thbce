// SPDX-License-Identifier: GPL-3.0-or-later
//! Sorties par zone et par groupe de la fiche 13.2 (Q116) : restitution par défaut inchangée,
//! somme des zones égale au bâtiment (importations, consommations, production photovoltaïque,
//! Cef, Cep, Cep,nr, besoins), lectures et variantes. Météo ouverte ADEME.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::Projet;
use thbce_core::resultats::sorties_detaillees::{LigneSortie, SortiesDetaillees};
use thbce_core::{calculer, calculer_avec, Demandes, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cas(nom: &str) -> (Projet, MeteoProjet) {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    let p = json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap();
    let zone = p.localisation.zone_climatique_retenue();
    let m = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    let m = meteo::lire(&std::fs::read(m).unwrap(), zone).unwrap();
    (p, m)
}

const SANS_ARRONDI: Options = Options { arrondir: false };
const DEMANDE: Demandes = Demandes {
    series: false,
    sorties_detaillees: true,
};

fn annuel(lignes: &[LigneSortie], poste: &str, energie: &str) -> f64 {
    lignes
        .iter()
        .filter(|l| l.poste == poste && l.energie == energie)
        .map(|l| l.quantite.annuel_kwh)
        .sum()
}

fn proche(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 1e-9 * (1.0 + a.abs().max(b.abs())),
        "{a} ≠ {b}"
    );
}

fn sorties(s: &Option<SortiesDetaillees>) -> &SortiesDetaillees {
    s.as_ref().expect("sorties détaillées")
}

#[test]
fn restitution_par_defaut_inchangee() {
    let (p, m) = cas("hotel-pv-sorties-ademe");
    let defaut = calculer(&p, &m, &SANS_ARRONDI).unwrap();
    assert!(defaut.batiments[0].sorties_detaillees.is_none());
    let mut avec = calculer_avec(&p, &m, &SANS_ARRONDI, &DEMANDE).unwrap();
    assert!(avec.batiments[0].sorties_detaillees.is_some());
    // Hors sorties détaillées, les résultats sont identiques.
    avec.batiments[0].sorties_detaillees = None;
    assert_eq!(
        json::ecrire_resultats(&defaut),
        json::ecrire_resultats(&avec)
    );
}

#[test]
fn somme_des_zones_egale_le_batiment() {
    let (p, m) = cas("hotel-pv-sorties-ademe");
    let r = calculer_avec(&p, &m, &SANS_ARRONDI, &DEMANDE).unwrap();
    let b = &r.batiments[0];
    let s = sorties(&b.sorties_detaillees);
    let ind = b.indicateurs.as_ref().unwrap();
    assert_eq!(s.zones.len(), 2);
    let sref: f64 = s.zones.iter().map(|z| z.sref_m2).sum();
    proche(sref, b.sref_m2);

    // Électricité importée et consommée par poste (forfait de refroidissement compris).
    let postes = [
        (
            "chauffage",
            ind.electricite_importee.chauffage_kwh,
            ind.electricite_consommee.chauffage_kwh,
        ),
        (
            "refroidissement",
            ind.electricite_importee.refroidissement_kwh,
            ind.electricite_consommee.refroidissement_kwh,
        ),
        (
            "ecs",
            ind.electricite_importee.ecs_kwh,
            ind.electricite_consommee.ecs_kwh,
        ),
        (
            "eclairage",
            ind.electricite_importee.eclairage_kwh,
            ind.electricite_consommee.eclairage_kwh,
        ),
        (
            "auxiliaires_ventilation",
            ind.electricite_importee.auxiliaires_ventilation_kwh,
            ind.electricite_consommee.auxiliaires_ventilation_kwh,
        ),
        (
            "auxiliaires_distribution",
            ind.electricite_importee.auxiliaires_distribution_kwh,
            ind.electricite_consommee.auxiliaires_distribution_kwh,
        ),
        (
            "deplacements",
            ind.electricite_importee.deplacements_kwh,
            ind.electricite_consommee.deplacements_kwh,
        ),
        (
            "usages_mobiliers",
            ind.electricite_importee.mobilier_kwh,
            ind.electricite_consommee.mobilier_kwh,
        ),
    ];
    for (poste, imp, cons) in postes {
        let zi: f64 = s
            .zones
            .iter()
            .map(|z| annuel(&z.importees, poste, "electricite"))
            .sum();
        let zc: f64 = s
            .zones
            .iter()
            .map(|z| annuel(&z.electricite_consommee, poste, "electricite"))
            .sum();
        proche(zi, imp);
        proche(zc, cons);
    }
    // Mois : électricité importée hors mobilier et hors forfait (annuel seulement).
    for mois in 0..12 {
        let z: f64 = s
            .zones
            .iter()
            .flat_map(|z| &z.importees)
            .filter(|l| l.energie == "electricite" && l.poste != "usages_mobiliers")
            .map(|l| l.quantite.mensuel_kwh[mois])
            .sum();
        proche(z, ind.electricite_importee_mensuel_kwh[mois]);
    }

    // Cef, Cep et Cep,nr : sommes pondérées par la surface de référence.
    let somme = |f: fn(&thbce_core::resultats::sorties_detaillees::SortiesZone) -> f64| {
        s.zones.iter().map(|z| f(z) * z.sref_m2).sum::<f64>() / sref
    };
    proche(somme(|z| z.cef_kwh_m2), ind.cef_kwhef_m2);
    proche(somme(|z| z.cep_kwhep_m2), ind.cep_kwhep_m2);
    proche(somme(|z| z.cep_nr_kwhep_m2), ind.cep_nr_kwhep_m2);

    // Production photovoltaïque : les deux lectures redonnent le bâtiment.
    let pv = ind.production_pv.as_ref().unwrap();
    for variante in [false, true] {
        let p = |z: &thbce_core::resultats::sorties_detaillees::SortiesZone| {
            if variante {
                z.production_pv_variante_sref.clone().unwrap()
            } else {
                z.production_pv.clone().unwrap()
            }
        };
        let prod: f64 = s.zones.iter().map(|z| p(z).produite.annuel_kwh).sum();
        let ac: f64 = s.zones.iter().map(|z| p(z).autoconsommee.annuel_kwh).sum();
        let exp: f64 = s.zones.iter().map(|z| p(z).exportee.annuel_kwh).sum();
        proche(prod, pv.e_prod_pv_kwh);
        proche(ac, pv.e_prod_pv_ac_kwh);
        proche(exp, pv.e_exportee_kwh);
        for (mois, attendu) in pv.e_prod_pv_mensuel_kwh.iter().enumerate() {
            let pm: f64 = s
                .zones
                .iter()
                .map(|z| p(z).produite.mensuel_kwh[mois])
                .sum();
            proche(pm, *attendu);
        }
    }

    // Besoins : zone = somme de ses groupes ; chauffage du bâtiment = somme des zones.
    for z in &s.zones {
        let g: f64 = z
            .groupes
            .iter()
            .map(|g| g.besoins.chauffage.annuel_kwh)
            .sum();
        proche(z.besoins.chauffage.annuel_kwh, g);
        let g: f64 = z.groupes.iter().map(|g| g.besoins.ecs.annuel_kwh).sum();
        proche(z.besoins.ecs.annuel_kwh, g);
    }
}

#[test]
fn groupes_et_lectures() {
    let (p, m) = cas("hotel-pv-sorties-ademe");
    let r = calculer_avec(&p, &m, &SANS_ARRONDI, &DEMANDE).unwrap();
    let b = &r.batiments[0];
    let s = sorties(&b.sorties_detaillees);
    for (z, rz) in s.zones.iter().zip(&b.zones) {
        assert_eq!(z.groupes.len(), rz.groupes.len());
        let c = rz.consommations.as_ref().unwrap();
        for g in &z.groupes {
            // Un groupe n'a ni déplacement ni usages mobiliers.
            assert!(g
                .consommations
                .iter()
                .all(|l| l.poste != "deplacements" && l.poste != "usages_mobiliers"));
            // Éq. (2416) et (2421) : chauffage et ECS consommés du groupe (une seule zone, un
            // seul groupe par zone dans cet exemple : groupe = zone).
            proche(
                annuel(&g.consommations, "chauffage", "electricite"),
                c.annuel.chauffage.electricite_kwh,
            );
            proche(
                annuel(&g.consommations, "ecs", "electricite"),
                c.annuel.ecs.electricite_kwh,
            );
            assert!(g.cef_kwh_m2 > 0.0);
        }
        // Bâtiment tout électrique : CoefEP = CoefEPnr = 2,3 (tableau 333).
        proche(z.cep_kwhep_m2, 2.3 * z.cef_kwh_m2);
        proche(z.cep_nr_kwhep_m2, 2.3 * z.cef_kwh_m2);
        // Lecture des besoins : aux bornes des émetteurs (retenue) au-dessus du Qsys
        // thermique du groupe, qui ignore le rendement et les pertes au dos des émetteurs.
        assert!(z.besoins.chauffage.annuel_kwh > z.besoins_qsys_thermique.chauffage.annuel_kwh);
        assert_eq!(z.besoins.ecs, z.besoins_qsys_thermique.ecs);
        // Lecture du photovoltaïque : au prorata des consommations ou des surfaces.
        let a = z.production_pv.as_ref().unwrap();
        let v = z.production_pv_variante_sref.as_ref().unwrap();
        assert!(a.produite.annuel_kwh != v.produite.annuel_kwh);
        assert!(a.tac_pct > 0.0 && a.tac_pct <= 100.0 + 1e-9);
    }
    // Forfait de refroidissement de la zone de jour, seul refroidissement importé :
    // 0,009 × (1 300 − 350) × 1,0 / 2,3 × 300 = 1 115,2 kWh (éq. 2493, tableau 335).
    let jour = s.zones.iter().find(|z| z.id == "jour").unwrap();
    assert!((annuel(&jour.importees, "refroidissement", "electricite") - 1115.2).abs() < 0.05);
    // Part de la zone de jour (gros usages mobiliers) plus forte au prorata des consommations
    // qu'au prorata des surfaces (25 % de la surface de référence).
    let part = jour.production_pv.as_ref().unwrap().produite.annuel_kwh
        / b.indicateurs
            .as_ref()
            .unwrap()
            .production_pv
            .as_ref()
            .unwrap()
            .e_prod_pv_kwh;
    assert!(part > 0.5, "{part}");
    let part_sref = jour
        .production_pv_variante_sref
        .as_ref()
        .unwrap()
        .produite
        .annuel_kwh
        / b.indicateurs
            .as_ref()
            .unwrap()
            .production_pv
            .as_ref()
            .unwrap()
            .e_prod_pv_kwh;
    proche(part_sref, 0.25);
}

#[test]
fn sans_production_pas_de_pv_attribue() {
    let (p, m) = cas("hotel-pac-ademe");
    let r = calculer_avec(&p, &m, &SANS_ARRONDI, &DEMANDE).unwrap();
    let s = sorties(&r.batiments[0].sorties_detaillees);
    assert!(s.zones.iter().all(|z| z.production_pv.is_none()));
    // Sans photovoltaïque, tout est importé.
    for z in &s.zones {
        for l in &z.importees {
            if l.energie == "electricite" {
                proche(
                    l.quantite.annuel_kwh,
                    annuel(&z.electricite_consommee, &l.poste, "electricite"),
                );
            }
        }
    }
}

/// Valeurs phares de l'hôtel avec photovoltaïque (H2b) sur la météo réglementaire : nombres
/// annuels seulement.
///
/// THBCE_METEO=/chemin/vers/meteo.csv cargo t-regl sorties_detaillees_sur_meteo_reglementaire
#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn sorties_detaillees_sur_meteo_reglementaire() {
    let (p, _) = cas("hotel-pv-sorties-ademe");
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO pour lancer ce test");
    let m = meteo::lire(
        &std::fs::read(chemin).unwrap(),
        p.localisation.zone_climatique_retenue(),
    )
    .unwrap();
    let r = calculer_avec(&p, &m, &SANS_ARRONDI, &DEMANDE).unwrap();
    assert!(r.reglementaire);
    let b = &r.batiments[0];
    let s = sorties(&b.sorties_detaillees);
    let ind = b.indicateurs.as_ref().unwrap();
    let pv = ind.production_pv.as_ref().unwrap();
    let sref: f64 = s.zones.iter().map(|z| z.sref_m2).sum();
    proche(
        s.zones
            .iter()
            .map(|z| z.cep_kwhep_m2 * z.sref_m2)
            .sum::<f64>()
            / sref,
        ind.cep_kwhep_m2,
    );
    proche(
        s.zones
            .iter()
            .map(|z| z.production_pv.as_ref().unwrap().produite.annuel_kwh)
            .sum::<f64>(),
        pv.e_prod_pv_kwh,
    );
    let jour = s.zones.iter().find(|z| z.id == "jour").unwrap();
    let nuit = s.zones.iter().find(|z| z.id == "nuit").unwrap();
    let a = |z: &thbce_core::resultats::sorties_detaillees::SortiesZone| {
        z.production_pv.as_ref().unwrap().produite.annuel_kwh
    };
    // Valeurs du moteur au lot 1-sor3, à détecter en cas de régression (arrondies à 0,1).
    for (obtenu, attendu) in [
        (ind.cep_kwhep_m2, 170.8),
        (pv.e_prod_pv_kwh, 21169.8),
        (nuit.cep_kwhep_m2, 179.8),
        (jour.cep_kwhep_m2, 143.6),
        (a(nuit), 5543.8),
        (a(jour), 15626.0),
        (nuit.besoins.chauffage.annuel_kwh, 84936.2),
        (nuit.besoins_qsys_thermique.chauffage.annuel_kwh, 83025.2),
    ] {
        assert!((obtenu - attendu).abs() < 0.06, "{obtenu} ≠ {attendu}");
    }
}
