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

/// Exemples sans `exigences` saisies.
fn sans_exigences(nom: &str) -> bool {
    nom.starts_with("bureaux-")
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
        // Les exemples d'habitation saisissent tous leurs `exigences` ; l'exemple de bureaux
        // n'en a pas (exigences de l'usage 3 : lot 0b).
        let Some(e) = i.exigences.as_ref() else {
            assert!(sans_exigences(nom), "{nom} : exigences");
            continue;
        };
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
            let Some(e) = i.exigences.as_ref() else {
                assert!(sans_exigences(&nom), "{nom} : exigences");
                println!(
                    "{:<36} {:>6.1} {:>6} {:>7.1} {:>7} {:>7.1} {:>7} {:>7.1} {:>7} {:>8.1} {:>6}",
                    nom,
                    b.bbio_pts.unwrap(),
                    "-",
                    i.cep_kwhep_m2,
                    "-",
                    i.cep_nr_kwhep_m2,
                    "-",
                    i.ic_energie_kgco2_m2.unwrap(),
                    "-",
                    b.dh_c_h.unwrap(),
                    "-"
                );
                continue;
            };
            // Valeurs recalculées à la main par la relecture indépendante, à partir des
            // consommations par poste.
            let attendu = match nom.as_str() {
                "maison-climat-ademe" => Some((145.81, 182.28)),
                "maison-chaudiere-gaz-ademe" => Some((85.41, 591.07)),
                _ => None,
            };
            // Valeurs phares de la version 0.1.0 (arrondies à une décimale), garde-fou contre une
            // régression : Bbio, DH, Cep et Ic énergie de la maison d'exemple, Bbio et DH de
            // l'immeuble d'exemple.
            let phares = match nom.as_str() {
                "maison-climat-ademe" => Some((59.7, 1042.9, Some((145.8, 182.3)))),
                "immeuble-collectif-ademe" => Some((31.8, 1039.6, None)),
                _ => None,
            };
            let a_une_decimale = |x: f64| (x * 10.0).round() / 10.0;
            if let Some((bbio, dh, cep_ic)) = phares {
                assert_eq!(a_une_decimale(b.bbio_pts.unwrap()), bbio, "{nom} : Bbio");
                assert_eq!(a_une_decimale(b.dh_c_h.unwrap()), dh, "{nom} : DH");
                if let Some((cep, ic)) = cep_ic {
                    assert_eq!(a_une_decimale(i.cep_kwhep_m2), cep, "{nom} : Cep");
                    assert_eq!(
                        a_une_decimale(i.ic_energie_kgco2_m2.unwrap()),
                        ic,
                        "{nom} : Ic énergie"
                    );
                }
            }
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

/// Annexe de l'article R. 172-4, chapitre V, partie II (Q54, point 14) : catégorie de
/// contraintes extérieures déclarée pour un bâtiment d'habitation.
mod categorie_contraintes {
    use super::*;

    fn variante(nom: &str, modifier: impl Fn(&mut serde_json::Value)) -> Vec<String> {
        let texte =
            std::fs::read_to_string(dossier_des_cas().join(nom).join("entree.json")).unwrap();
        let mut v: serde_json::Value = serde_json::from_str(&texte).unwrap();
        modifier(&mut v);
        let p = json::lire_projet(&v.to_string()).unwrap();
        thbce_core::valider(&p)
            .erreurs
            .iter()
            .map(|e| format!("{} : {}", e.chemin, e.message))
            .collect()
    }

    /// Maison climatisée (PAC air/air), dans le Var (H3), à 80 m, en BR2 : catégorie 2 admise.
    fn categorie_2(v: &mut serde_json::Value) {
        v["localisation"]["departement"] = "83".into();
        let e = &mut v["batiments"][0]["exigences"];
        e["categorie_contraintes_exterieures"] = "categorie_2".into();
        e["zone_bruit"] = "br2".into();
    }

    #[test]
    fn categorie_2_admise_quand_les_trois_conditions_sont_reunies() {
        let e = variante("maison-pac-air-air-ademe", categorie_2);
        assert!(e.is_empty(), "{e:?}");
    }

    #[test]
    fn categorie_2_refusee_sans_climatisation() {
        let e = variante("maison-pac-air-air-ademe", |v| {
            categorie_2(v);
            v["batiments"][0]["zones"][0]["groupes"][0]["iclim"] = false.into();
        });
        assert!(e.iter().any(|m| m.contains("n'est pas climatisé")), "{e:?}");
    }

    #[test]
    fn categorie_2_refusee_hors_h2d_et_h3() {
        let e = variante("maison-pac-air-air-ademe", |v| {
            categorie_2(v);
            v["localisation"]["departement"] = "37".into();
        });
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(e[0].contains("zone climatique H2b"), "{e:?}");
    }

    #[test]
    fn categorie_2_refusee_a_400_m_et_plus() {
        let e = variante("maison-pac-air-air-ademe", |v| {
            categorie_2(v);
            v["localisation"]["alt_m"] = 400.into();
        });
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(e[0].contains("400 m d'altitude"), "{e:?}");
    }

    #[test]
    fn categorie_2_refusee_en_br1() {
        let e = variante("maison-pac-air-air-ademe", |v| {
            categorie_2(v);
            v["batiments"][0]["exigences"]["zone_bruit"] = "br1".into();
        });
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(e[0].contains("zone de bruit BR1"), "{e:?}");
    }

    #[test]
    fn categorie_3_refusee_en_habitation() {
        let e = variante("maison-climat-ademe", |v| {
            v["batiments"][0]["exigences"]["categorie_contraintes_exterieures"] =
                "categorie_3".into();
        });
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(e[0].contains("réservée aux locaux climatisés"), "{e:?}");
    }
}
