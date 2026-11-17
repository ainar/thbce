// SPDX-License-Identifier: GPL-3.0-or-later
//! Indicateurs réglementaires (lot 7) de bout en bout sur tous les cas de `tests/cas/` : Cep,
//! Cep,nr, Ic énergie, exigences et conformité. Météo ouverte ADEME (CI publique) ; météo
//! réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test indicateurs -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence : bilans (somme des lignes, électricité importée et autoconsommée),
//! propriétés et ordres de grandeur.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::{Path, PathBuf};

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::Projet;
use thbce_core::resultats::Resultats;
use thbce_core::{calculer, Options};
use thbce_io::{json, meteo, rapport};

fn dossier_des_cas() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/cas")
}

fn cas() -> Vec<(String, Projet)> {
    let mut v: Vec<(String, Projet)> = std::fs::read_dir(dossier_des_cas())
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("entree.json").is_file())
        .map(|p| {
            let nom = p.file_name().unwrap().to_str().unwrap().to_owned();
            let projet =
                json::lire_projet(&std::fs::read_to_string(p.join("entree.json")).unwrap())
                    .unwrap();
            (nom, projet)
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

fn meteo_ademe(p: &Projet) -> MeteoProjet {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    meteo::lire(
        &std::fs::read(chemin).unwrap(),
        p.localisation.zone_climatique_retenue(),
    )
    .unwrap()
}

/// Bilans et propriétés communs à tous les cas.
fn verifier(nom: &str, r: &Resultats) {
    for b in &r.batiments {
        let i = b
            .indicateurs
            .as_ref()
            .unwrap_or_else(|| panic!("{nom} : indicateurs absents"));
        // Les lignes du bilan redonnent Cep, Cep,nr, Cef et Ic énergie.
        let somme = |f: fn(&thbce_core::indicateurs::resultats::LigneBilanEnergie) -> f64| {
            i.lignes.iter().map(f).sum::<f64>()
        };
        assert!(
            (somme(|l| l.cep_kwhep_m2) - i.cep_kwhep_m2).abs() < 1e-9,
            "{nom}"
        );
        assert!(
            (somme(|l| l.cep_nr_kwhep_m2) - i.cep_nr_kwhep_m2).abs() < 1e-9,
            "{nom}"
        );
        assert!(
            (somme(|l| l.cef_kwh) / i.sref_m2 - i.cef_kwhef_m2).abs() < 1e-9,
            "{nom}"
        );
        let ic = i
            .ic_energie_kgco2_m2
            .unwrap_or_else(|| panic!("{nom} : Ic"));
        assert!(
            (somme(|l| l.ic_energie_kgco2_m2.unwrap()) - ic).abs() < 1e-9,
            "{nom}"
        );
        assert_eq!(b.cep_kwhep_m2, Some(i.cep_kwhep_m2));
        // Coefficients d'énergie primaire non renouvelable au plus égaux aux totaux.
        assert!(i.cep_nr_kwhep_m2 <= i.cep_kwhep_m2 + 1e-9, "{nom}");
        // Électricité importée ≤ consommée, poste par poste.
        let (c, m) = (&i.electricite_consommee, &i.electricite_importee);
        for (x, y) in [
            (c.chauffage_kwh, m.chauffage_kwh),
            (c.ecs_kwh, m.ecs_kwh),
            (c.eclairage_kwh, m.eclairage_kwh),
            (c.auxiliaires_ventilation_kwh, m.auxiliaires_ventilation_kwh),
            (c.mobilier_kwh, m.mobilier_kwh),
        ] {
            assert!(y <= x + 1e-9, "{nom}");
        }
        match &i.production_pv {
            None => assert!((c.hors_mobilier_kwh() - m.hors_mobilier_kwh()).abs() < 1e-6),
            Some(pv) => {
                // L'énergie autoconsommée est l'électricité non importée, mobilier compris.
                let evitee =
                    c.hors_mobilier_kwh() + c.mobilier_kwh - m.hors_mobilier_kwh() - m.mobilier_kwh;
                assert!((evitee - pv.e_prod_pv_ac_kwh).abs() < 1e-6, "{nom}");
                assert!((pv.e_prod_pv_ac_kwh + pv.e_exportee_kwh - pv.e_prod_pv_kwh).abs() < 1e-6);
            }
        }
        let e = i
            .exigences
            .as_ref()
            .unwrap_or_else(|| panic!("{nom} : exigences"));
        assert!(e.cep_max_kwhep_m2 > 0.0 && e.ic_energie_max_kgco2_m2 > 0.0);
        assert_eq!(e.conforme, e.verifications.iter().all(|v| v.respectee));
    }
}

#[test]
fn indicateurs_de_tous_les_cas_meteo_ademe() {
    for (nom, p) in cas() {
        let r = calculer(&p, &meteo_ademe(&p), &Options { arrondir: false }).unwrap();
        verifier(&nom, &r);
    }
}

#[test]
fn le_poele_sans_combustible_est_refuse() {
    let chemin = dossier_des_cas().join("maison-poele-granules-ademe/entree.json");
    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(chemin).unwrap()).unwrap();
    let mut retire = false;
    for g in v["batiments"][0]["generations"].as_array_mut().unwrap() {
        for x in g["generateurs"].as_array_mut().unwrap() {
            if let Some(p) = x["nature"]["poele_ou_insert"].as_object_mut() {
                retire |= p.remove("combustible_bois").is_some();
            }
        }
    }
    assert!(retire);
    // `combustible_bois` est obligatoire sur un poêle ou un insert (Q54).
    assert!(json::lire_projet(&v.to_string()).is_err());
}

#[test]
#[ignore = "météo réglementaire requise : THBCE_METEO"]
fn indicateurs_de_tous_les_cas_meteo_reglementaire() {
    let chemin = std::env::var_os("THBCE_METEO").expect("THBCE_METEO");
    let octets = std::fs::read(chemin).unwrap();
    println!(
        "{:<36} {:>6} {:>6} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>8} {:>6}",
        "cas", "Bbio", "max", "Cep", "max", "Cep,nr", "max", "Ic", "max", "DH", "conf."
    );
    for (nom, p) in cas() {
        let m = meteo::lire(&octets, p.localisation.zone_climatique_retenue()).unwrap();
        let r = calculer(&p, &m, &Options { arrondir: false }).unwrap();
        verifier(&nom, &r);
        for b in &r.batiments {
            let i = b.indicateurs.as_ref().unwrap();
            let e = i.exigences.as_ref().unwrap();
            // Valeurs recalculées à la main par la relecture indépendante, à partir des
            // consommations par poste.
            let attendu = match nom.as_str() {
                "maison-climat-ademe" => Some((145.81, 182.28)),
                "maison-chaudiere-gaz-ademe" => Some((85.41, 591.07)),
                _ => None,
            };
            if let Some((cep, ic)) = attendu {
                assert!(
                    (i.cep_kwhep_m2 - cep).abs() < 0.01,
                    "{nom} : Cep {}",
                    i.cep_kwhep_m2
                );
                let obtenu = i.ic_energie_kgco2_m2.unwrap();
                assert!((obtenu - ic).abs() < 0.01, "{nom} : Ic {obtenu}");
            }
            println!(
                "{:<36} {:>6.1} {:>6.1} {:>7.1} {:>7.1} {:>7.1} {:>7.1} {:>7.1} {:>7.1} {:>8.1} {:>6}",
                nom,
                b.bbio_pts.unwrap(),
                e.bbio_max_pts,
                i.cep_kwhep_m2,
                e.cep_max_kwhep_m2,
                i.cep_nr_kwhep_m2,
                e.cep_nr_max_kwhep_m2,
                i.ic_energie_kgco2_m2.unwrap(),
                e.ic_energie_max_kgco2_m2,
                b.dh_c_h.unwrap(),
                if e.conforme { "oui" } else { "non" }
            );
            if std::env::var_os("THBCE_DETAIL").is_some() {
                print!("{}", rapport::synthese_indicateurs(&r));
                for l in &i.lignes {
                    println!(
                        "    {:<26} {:<28} {:>9.1} kWh  Cep {:>6.2}  Cep,nr {:>6.2}  Ic {:>7.2}",
                        l.poste,
                        l.energie,
                        l.cef_kwh,
                        l.cep_kwhep_m2,
                        l.cep_nr_kwhep_m2,
                        l.ic_energie_kgco2_m2.unwrap_or(f64::NAN)
                    );
                }
                println!(
                    "    électricité consommée {:?}\n    forfaits {:?}\n    PV {:?}",
                    i.electricite_consommee, i.forfaits_refroidissement, i.production_pv
                );
            }
            // Ordres de grandeur d'une maison RE2020 en H2b : Cep de 20 à 150 kWhep/(m².an),
            // Ic énergie de 20 à 600 kg éq. CO2/m².
            assert!((10.0..200.0).contains(&i.cep_kwhep_m2), "{nom}");
            assert!(
                (10.0..800.0).contains(&i.ic_energie_kgco2_m2.unwrap()),
                "{nom}"
            );
        }
    }
}
