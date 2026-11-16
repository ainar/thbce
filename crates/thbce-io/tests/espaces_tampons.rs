// SPDX-License-Identifier: GPL-3.0-or-later
//! Espaces tampons (Th-BCE 2020 §5.2 à 5.4) de bout en bout, sur la variante de la maison
//! d'exemple `tests/cas/maison-veranda-ademe/entree.json` : une véranda au sud (mur sud et baie
//! sud) et un garage à l'ouest (mur ouest).
//!
//! Propriétés vérifiées, sur la météo ouverte ADEME (CI publique) et sur la météo
//! réglementaire (test ignoré par défaut) :
//! - bilans d'énergie fermés de la véranda et du groupe ;
//! - Bbio du garage entre celui du mur donnant sur l'extérieur et celui du mur adiabatique, et
//!   décroissant avec `bet_ns` ;
//! - besoin de chauffage de la véranda entre celui des parois donnant sur l'extérieur et celui
//!   des parois adiabatiques ; sens de variation avec l'isolation de la véranda, ses ouvertures
//!   et le passage de l'air neuf par elle.
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test --release -p thbce-io --test espaces_tampons -- --include-ignored --nocapture
//! ```

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::bati::assemblage_batiment::BatimentHoraire;
use thbce_core::bati::assemblage_groupe::Lectures;
use thbce_core::bati::espace_tampon::gestion_solarise::LectureFermetureEt;
use thbce_core::climat::{ClimatExterieur, ConfortAdaptatif, MeteoProjet};
use thbce_core::conventions::ModeCalcul;
use thbce_core::entree::{NatureEspaceTampon, Projet};
use thbce_core::resultats::Resultats;
use thbce_core::{calculer, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn variante() -> Projet {
    let chemin = racine().join("tests/cas/maison-veranda-ademe/entree.json");
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn meteo_ademe(projet: &Projet) -> MeteoProjet {
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    let zone = projet.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

fn meteo_reglementaire(projet: &Projet) -> MeteoProjet {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO pour lancer ce test");
    let zone = projet.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

fn brut(projet: &Projet, meteo: &MeteoProjet) -> Resultats {
    calculer(projet, meteo, &Options { arrondir: false }).unwrap()
}

/// `(Bbio, B_Ch, B_Fr, DH)` du bâtiment.
fn indicateurs(projet: &Projet, meteo: &MeteoProjet) -> (f64, f64, f64, f64) {
    let r = brut(projet, meteo);
    let b = &r.batiments[0];
    let a = &b.bbio.as_ref().unwrap().annuel;
    (a.bbio_pts, a.b_ch_kwh_m2, a.b_fr_kwh_m2, b.dh_c_h.unwrap())
}

/// Ne garde que les espaces tampons `gardes` : les composants qui donnaient sur un autre
/// espace tampon donnent sur l'extérieur.
fn garder(projet: &Projet, gardes: &[&str]) -> Projet {
    let mut p = projet.clone();
    let bat = &mut p.batiments[0];
    bat.espaces_tampons
        .retain(|e| gardes.contains(&e.id.as_str()));
    let g = &mut bat.zones[0].groupes[0];
    let garde = |id: &mut Option<String>| {
        if id.as_ref().is_some_and(|i| !gardes.contains(&i.as_str())) {
            *id = None;
        }
    };
    g.parois_opaques
        .iter_mut()
        .for_each(|c| garde(&mut c.espace_tampon));
    g.baies.iter_mut().for_each(|c| garde(&mut c.espace_tampon));
    p
}

/// Retire l'espace tampon `et` et les composants qui donnent sur lui : ils deviennent
/// adiabatiques.
fn adiabatique(projet: &Projet, et: &str) -> Projet {
    let mut p = projet.clone();
    let bat = &mut p.batiments[0];
    bat.espaces_tampons.retain(|e| e.id != et);
    let g = &mut bat.zones[0].groupes[0];
    let sur = |id: &Option<String>| id.as_deref() == Some(et);
    g.parois_opaques.retain(|c| !sur(&c.espace_tampon));
    g.baies.retain(|c| !sur(&c.espace_tampon));
    p
}

fn avec_b_garage(projet: &Projet, b: f64) -> Projet {
    let mut p = projet.clone();
    for e in &mut p.batiments[0].espaces_tampons {
        if let NatureEspaceTampon::NonSolarise(n) = &mut e.nature {
            n.b_et_ns = b;
        }
    }
    p
}

fn modifier_veranda(
    projet: &Projet,
    f: impl Fn(&mut thbce_core::entree::EspaceTamponSolarise),
) -> Projet {
    let mut p = projet.clone();
    for e in &mut p.batiments[0].espaces_tampons {
        if let NatureEspaceTampon::Solarise(s) = &mut e.nature {
            f(s);
        }
    }
    p
}

/// Bilans d'énergie de la véranda et du groupe, dans les deux modes de calcul.
fn verifier_les_bilans(projet: &Projet, meteo: &MeteoProjet, titre: &str) {
    for (mode, annee) in [
        (ModeCalcul::ThB, &meteo.th_bc),
        (ModeCalcul::ThD, &meteo.th_d),
    ] {
        let loc = &projet.localisation;
        let climat = ClimatExterieur::nouveau(annee, loc.departement, loc.alt_m);
        let confort = ConfortAdaptatif::nouveau(&climat.te_c);
        let mut bat = BatimentHoraire::nouveau(
            &projet.batiments[0],
            &projet.catalogue,
            mode,
            loc.alt_m,
            climat.we_kg_kg[0],
        )
        .unwrap();
        let (mut b_min, mut b_max) = (f64::INFINITY, f64::NEG_INFINITY);
        bat.simuler_avec(&climat, &confort, |_, b| {
            for c in &b.coefficients {
                // §5.4.3.3.1.6, éq. (126) : btherm entre −15 et 1 ; bsolaire et becl entre 0
                // et 1 (éq. 112, 113, 127 et 129).
                assert!((-15.0..=1.0).contains(&c.b_therm));
                assert!((0.0..=1.0).contains(&c.b_solaire));
                assert!((0.0..=1.0).contains(&c.b_ecl));
                assert!(c.theta_et_c.is_finite());
            }
            b_min = b_min.min(b.coefficients[0].b_therm);
            b_max = b_max.max(b.coefficients[0].b_therm);
        })
        .unwrap();
        let v = &bat.espaces_tampons[0];
        let k = &v.cumuls;
        let echelle = k.apports_solaires_wh.abs() + k.pertes_exterieur_wh.abs();
        println!(
            "{titre}, {mode:?} : véranda {:.1} °C en moyenne ({:.1} à {:.1} °C), btherm moyen \
             {:.3} ({b_min:.2} à {b_max:.2}), ouverte {} h ; bilan (kWh) : solaire {:.0}, reçu \
             du volume chauffé {:.0}, pertes vers l'extérieur {:.0}, stockage {:.2} ; résidu \
             {:.2e} Wh",
            k.somme_theta_et_c / f64::from(k.nbh),
            k.theta_et_min_c,
            k.theta_et_max_c,
            k.somme_b_therm / f64::from(k.nbh),
            k.nbh_surventilation,
            k.apports_solaires_wh / 1e3,
            k.recu_du_volume_chauffe_wh / 1e3,
            k.pertes_exterieur_wh / 1e3,
            k.stockage_wh / 1e3,
            k.residu_wh(),
        );
        assert!(
            k.residu_wh().abs() < 1e-9 * echelle,
            "résidu {}",
            k.residu_wh()
        );
        // Le groupe : le modèle à trois nœuds conserve l'énergie, btherm variable compris.
        let g = &bat.zones[0].groupes[0];
        let b = &g.cumuls.bilan;
        let echelle_g = b.entrees_wh();
        assert!(
            b.residu_wh().abs() < 1e-9 * echelle_g,
            "résidu du groupe {}",
            b.residu_wh()
        );
        // La véranda est plus tempérée que l'extérieur et prend de la chaleur au groupe sur
        // l'année.
        assert!(k.recu_du_volume_chauffe_wh > 0.0);
        assert!(k.nbh_surventilation > 0);
        // Le garage n'est pas ouvert, et son btherm est constant (sans générateur d'ECS).
        let garage = &bat.espaces_tampons[1].cumuls;
        assert_eq!(garage.nbh_surventilation, 0);
        assert!((garage.somme_b_therm / f64::from(garage.nbh) - 0.95).abs() < 1e-12);
    }
}

/// Garage : Bbio entre ceux du mur extérieur et du mur adiabatique ; monotone en `bet_ns`.
fn verifier_le_garage(projet: &Projet, meteo: &MeteoProjet, titre: &str) {
    let garage = garder(projet, &["garage"]);
    let exterieur = indicateurs(&garder(projet, &[]), meteo);
    let adiab = indicateurs(&adiabatique(&garage, "garage"), meteo);
    let par_b: Vec<(f64, (f64, f64, f64, f64))> = [1.0, 0.95, 0.5, 0.0]
        .into_iter()
        .map(|b| (b, indicateurs(&avec_b_garage(&garage, b), meteo)))
        .collect();
    println!("{titre}, garage (Bbio, B_Ch, B_Fr, DH) :");
    println!("  mur extérieur : {exterieur:.2?}");
    for (b, i) in &par_b {
        println!("  bet_ns = {b} : {i:.2?}");
    }
    println!("  mur adiabatique : {adiab:.2?}");
    // Décroissance du Bbio et du besoin de chauffage avec bet_ns (éq. 111).
    for w in par_b.windows(2) {
        assert!(w[1].1 .0 < w[0].1 .0, "Bbio non décroissant : {w:?}");
        assert!(w[1].1 .1 < w[0].1 .1, "B_Ch non décroissant : {w:?}");
    }
    // Entre le mur extérieur et le mur adiabatique.
    let (b_ext, b_adiab) = (exterieur.0, adiab.0);
    let b_moitie = par_b[2].1 .0;
    assert!(b_adiab < b_moitie && b_moitie < b_ext);
    // bet_ns = 0 : le mur n'échange plus rien ; seul reste son effet sur la perméabilité de
    // l'enveloppe (surface de façade, §5.6.3.1.5), d'où un Bbio un peu au-dessus de celui du mur
    // adiabatique.
    let b_nul = par_b[3].1 .0;
    assert!(b_adiab < b_nul);
}

/// Véranda : besoin de chauffage entre ceux des parois extérieures et adiabatiques ; sens de
/// variation.
fn verifier_la_veranda(projet: &Projet, meteo: &MeteoProjet, titre: &str) {
    let veranda = garder(projet, &["veranda"]);
    let exterieur = indicateurs(&garder(projet, &[]), meteo);
    let adiab = indicateurs(&adiabatique(&veranda, "veranda"), meteo);
    let reference = indicateurs(&veranda, meteo);
    let par_la_veranda = {
        let mut p = veranda.clone();
        p.batiments[0].zones[0].groupes[0]
            .entrees_air
            .as_mut()
            .unwrap()
            .espace_tampon = Some("veranda".into());
        indicateurs(&p, meteo)
    };
    let simple_vitrage = indicateurs(
        &modifier_veranda(&veranda, |s| {
            for p in &mut s.parois {
                p.u_ue_w_m2k = 5.8;
            }
        }),
        meteo,
    );
    let fermee = indicateurs(
        &modifier_veranda(&veranda, |s| {
            for p in &mut s.parois {
                p.r_ouv_max = 0.0;
            }
        }),
        meteo,
    );
    println!("{titre}, véranda (Bbio, B_Ch, B_Fr, DH) :");
    println!("  parois extérieures : {exterieur:.2?}");
    println!("  véranda : {reference:.2?}");
    println!("  véranda, entrées d'air par la véranda : {par_la_veranda:.2?}");
    println!("  véranda en simple vitrage : {simple_vitrage:.2?}");
    println!("  véranda sans ouvrant : {fermee:.2?}");
    println!("  parois adiabatiques : {adiab:.2?}");
    // La véranda réduit les déperditions du mur et de la baie sans les annuler.
    assert!(adiab.1 < reference.1 && reference.1 < exterieur.1);
    // Air neuf préchauffé par la véranda : moins de chauffage.
    assert!(par_la_veranda.1 < reference.1);
    // Véranda moins isolée : plus de chauffage.
    assert!(simple_vitrage.1 > reference.1);
    // Véranda sans ouvrant : plus chaude l'été, DH plus élevé ; plus chaude l'hiver aussi.
    assert!(fermee.3 > reference.3);
    assert!(fermee.1 < reference.1);
    // Lecture Q41 point 4 : variante de la branche de fermeture de l'éq. (104).
    let loc = &projet.localisation;
    let mesurer = |lecture| {
        let climat = ClimatExterieur::nouveau(&meteo.th_d, loc.departement, loc.alt_m);
        let confort = ConfortAdaptatif::nouveau(&climat.te_c);
        let mut b = BatimentHoraire::nouveau_selon(
            &veranda.batiments[0],
            &veranda.catalogue,
            ModeCalcul::ThD,
            Lectures {
                fermeture_et: lecture,
                ..Lectures::default()
            },
            loc.alt_m,
            climat.we_kg_kg[0],
        )
        .unwrap();
        b.simuler(&climat, &confort).unwrap();
        b.zones[0].groupes[0].cumuls.confort_ete.dh_c_h
    };
    let dh_retenue = mesurer(LectureFermetureEt::Symetrique);
    let dh_variante = mesurer(LectureFermetureEt::DemiOuverture);
    println!(
        "  Q41 point 4 : DH {dh_retenue:.1} °C.h (lecture retenue), {dh_variante:.1} °C.h \
         (ratio 0,5 au-dessus de θet_fer1)"
    );
    assert_eq!(dh_retenue, reference.3);
}

#[test]
fn espaces_tampons_sur_la_meteo_ademe() {
    let p = variante();
    let m = meteo_ademe(&p);
    let titre = "météo ADEME (+2,7 °C, vague de chaleur)";
    let (bbio, b_ch, b_fr, dh) = indicateurs(&p, &m);
    println!("{titre}, véranda et garage : Bbio = {bbio:.2} ; B_Ch = {b_ch:.2}, B_Fr = {b_fr:.2} ; DH = {dh:.1}");
    verifier_les_bilans(&p, &m, titre);
    verifier_le_garage(&p, &m, titre);
    verifier_la_veranda(&p, &m, titre);
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn espaces_tampons_sur_la_meteo_reglementaire() {
    let p = variante();
    let m = meteo_reglementaire(&p);
    assert!(m.est_reglementaire());
    let titre = "météo réglementaire RE2020, H2b";
    let (bbio, b_ch, b_fr, dh) = indicateurs(&p, &m);
    println!("{titre}, véranda et garage : Bbio = {bbio:.2} ; B_Ch = {b_ch:.2}, B_Fr = {b_fr:.2} ; DH = {dh:.1}");
    verifier_les_bilans(&p, &m, titre);
    verifier_le_garage(&p, &m, titre);
    verifier_la_veranda(&p, &m, titre);
}
