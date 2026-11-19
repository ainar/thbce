// SPDX-License-Identifier: GPL-3.0-or-later
//! Indicateurs par occupant (fiche 13.6, Q117) et Ic énergie annuel (annexe II éq. 75) de bout
//! en bout, sur l'immeuble d'exemple (météo ouverte ADEME). Valeurs vérifiées par les relations
//! du texte (éq. 131, 2552, 2553, 75), non par des constantes tirées du moteur.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{Projet, TypeLogement};
use thbce_core::{calculer, valider, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cas(nom: &str) -> Projet {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn calcul(p: &Projet) -> thbce_core::resultats::Resultats {
    let zone = p.localisation.zone_climatique_retenue();
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    let m: MeteoProjet = meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap();
    calculer(p, &m, &Options { arrondir: false }).unwrap()
}

fn chemins_erreurs(p: &Projet) -> Vec<String> {
    valider(p)
        .erreurs
        .iter()
        .map(|e| e.chemin.clone())
        .collect()
}

#[test]
fn ic_energie_annuel_somme_et_poids_eq_75() {
    let r = calcul(&cas("immeuble-collectif-ademe"));
    let i = r.batiments[0].indicateurs.as_ref().unwrap();
    let ic = i.ic_energie_kgco2_m2.unwrap();
    let a = &i.ic_energie_annuel_kgco2_m2;
    assert_eq!(a.len(), 50);
    // La somme des 50 valeurs annuelles redonne l'Ic énergie (éq. 74 et 75).
    assert!((a.iter().sum::<f64>() - ic).abs() < 1e-9 * ic);
    // Rapport des années 1 et 50 : fCO2(1)/fCO2(50) = 0,992/0,578 (article 11).
    assert!((a[0] / a[49] - 0.992 / 0.578).abs() < 1e-12);
    // Σ fCO2(a), a = 1..50 : 39,543 (Q54, point 7).
    assert!((i.somme_f_co2 - 39.543).abs() < 1e-9);
    assert!((a[0] - ic * 0.992 / 39.543).abs() < 1e-9);
}

#[test]
fn indicateurs_par_occupant_d_un_immeuble() {
    let mut p = cas("immeuble-collectif-ademe");
    // 12 logements : 4 T2, 6 T3 et 2 T5 → 8 + 18 + 10 = 36 occupants (éq. 131).
    let z = &mut p.batiments[0].zones[0];
    assert_eq!(z.nb_logements, Some(12));
    z.types_logements = vec![
        TypeLogement {
            pieces_principales: 2,
            nombre: 4,
        },
        TypeLogement {
            pieces_principales: 3,
            nombre: 6,
        },
        TypeLogement {
            pieces_principales: 5,
            nombre: 2,
        },
    ];
    assert!(valider(&p).erreurs.is_empty());
    let r = calcul(&p);
    let b = &r.batiments[0];
    let i = b.indicateurs.as_ref().unwrap();
    let o = i.occupants.as_ref().unwrap();
    assert_eq!(o.nocc_batiment, 36.0);
    assert_eq!(o.zones[0].origine, "logements");
    // Éq. (2553) : Cep × SREF / Nocc.
    let attendu = i.cep_kwhep_m2 * b.sref_m2 / 36.0;
    assert!((o.cep_par_occupant_kwhep - attendu).abs() < 1e-9);
    let ic = i.ic_energie_kgco2_m2.unwrap();
    assert!((o.ic_energie_par_occupant_kgco2.unwrap() - ic * b.sref_m2 / 36.0).abs() < 1e-9);
    assert!(o.valeurs_par_defaut.is_empty());
}

#[test]
fn sans_saisie_pas_d_indicateur_en_habitation_avec_avertissement() {
    let r = calcul(&cas("immeuble-collectif-ademe"));
    let b = &r.batiments[0];
    assert!(b.indicateurs.as_ref().unwrap().occupants.is_none());
    assert!(r
        .avertissements
        .iter()
        .any(|a| a.contains("indicateurs par occupant non calculés")));
}

#[test]
fn bureaux_sans_saisie_utilisent_les_scenarios_et_le_signalent() {
    let r = calcul(&cas("bureaux-simple-flux-ademe"));
    let i = r.batiments[0].indicateurs.as_ref().unwrap();
    let o = i.occupants.as_ref().unwrap();
    assert_eq!(o.zones[0].origine, "scenarios");
    assert!(o.nocc_batiment > 0.0);
    assert_eq!(o.valeurs_par_defaut.len(), 1);
    assert_eq!(o.valeurs_par_defaut[0].valeur, o.nocc_batiment);
    // Saisie du nombre d'occupants : elle l'emporte et n'est plus un défaut.
    let mut p = cas("bureaux-simple-flux-ademe");
    p.batiments[0].zones[0].nb_occupants = Some(50.0);
    let r = calcul(&p);
    let b = &r.batiments[0];
    let i = b.indicateurs.as_ref().unwrap();
    let o = i.occupants.as_ref().unwrap();
    assert_eq!(o.nocc_batiment, 50.0);
    assert!(o.valeurs_par_defaut.is_empty());
    assert!((o.cep_par_occupant_kwhep - i.cep_kwhep_m2 * b.sref_m2 / 50.0).abs() < 1e-9);
}

#[test]
fn enseignement_demande_le_nombre_d_eleves() {
    let r = calcul(&cas("ecole-primaire-ademe"));
    assert!(r.batiments[0]
        .indicateurs
        .as_ref()
        .unwrap()
        .occupants
        .is_none());
    let mut p = cas("ecole-primaire-ademe");
    p.batiments[0].zones[0].nb_occupants = Some(120.0);
    let r = calcul(&p);
    let o = r.batiments[0]
        .indicateurs
        .as_ref()
        .unwrap()
        .occupants
        .clone()
        .unwrap();
    assert_eq!(o.nocc_batiment, 120.0);
}

#[test]
fn refus_de_saisies_incoherentes() {
    let mut p = cas("immeuble-collectif-ademe");
    let z = &mut p.batiments[0].zones[0];
    z.types_logements = vec![TypeLogement {
        pieces_principales: 3,
        nombre: 11,
    }];
    assert_eq!(
        chemins_erreurs(&p),
        ["batiments[0].zones[0].types_logements"]
    );
    let z = &mut p.batiments[0].zones[0];
    z.types_logements = vec![TypeLogement {
        pieces_principales: 8,
        nombre: 12,
    }];
    assert_eq!(
        chemins_erreurs(&p),
        ["batiments[0].zones[0].types_logements[0].pieces_principales"]
    );
    let z = &mut p.batiments[0].zones[0];
    z.types_logements = vec![TypeLogement {
        pieces_principales: 3,
        nombre: 12,
    }];
    z.nb_occupants = Some(30.0);
    assert_eq!(chemins_erreurs(&p), ["batiments[0].zones[0].nb_occupants"]);
    // Pièces principales hors habitation.
    let mut p = cas("bureaux-simple-flux-ademe");
    p.batiments[0].zones[0].types_logements = vec![TypeLogement {
        pieces_principales: 3,
        nombre: 1,
    }];
    assert_eq!(
        chemins_erreurs(&p),
        ["batiments[0].zones[0].types_logements"]
    );
}
