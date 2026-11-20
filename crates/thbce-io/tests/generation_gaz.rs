// SPDX-License-Identifier: GPL-3.0-or-later
//! Générateurs thermodynamiques au gaz (fiches 8.24 et 8.25, lot 2c) de bout en bout : exemples
//! `maison-gahp-ademe`, `maison-pac-moteur-gaz-air-air-ademe` et `commerce-pac-moteur-gaz-ademe`,
//! sens des effets des lectures Q137 à Q143, refus (code 3) et erreurs de saisie ; valeurs
//! phares sur la météo réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo.csv cargo test -p thbce-io --test generation_gaz -- --ignored --nocapture
//! ```

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::chauffage_refroidissement::calculs_generation::{Energie, Poste};
use thbce_core::chauffage_refroidissement::generation::Generateur;
use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{NatureGenerateur, Projet, ThermodynamiqueGazSaisi};
use thbce_core::resultats::Resultats;
use thbce_core::{calculer, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cas(nom: &str) -> Projet {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn meteo_ademe(projet: &Projet) -> MeteoProjet {
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    meteo::lire(
        &std::fs::read(chemin).unwrap(),
        projet.localisation.zone_climatique_retenue(),
    )
    .unwrap()
}

fn brut(projet: &Projet, meteo: &MeteoProjet) -> Resultats {
    calculer(projet, meteo, &Options { arrondir: false }).unwrap()
}

/// Gaz consommé par les générations du premier bâtiment en Th-C, en kWh, après modification
/// des modèles résolus (variantes de lecture).
fn gaz_th_c(projet: &Projet, meteo: &MeteoProjet, modifier: impl Fn(&mut Generateur)) -> f64 {
    use thbce_core::bati::assemblage_batiment::BatimentHoraire;
    use thbce_core::climat::{ClimatExterieur, ConfortAdaptatif};
    use thbce_core::conventions::ModeCalcul;
    let loc = &projet.localisation;
    let climat = ClimatExterieur::nouveau(&meteo.th_bc, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    let mut b = BatimentHoraire::nouveau(
        &projet.batiments[0],
        &projet.catalogue,
        ModeCalcul::ThC,
        loc.alt_m,
        climat.we_kg_kg[0],
    )
    .unwrap();
    for g in &mut b.systemes.generations_horaires {
        for x in &mut g.generateurs {
            modifier(&mut x.modele);
        }
    }
    b.simuler(&climat, &confort).unwrap();
    b.systemes
        .generations_horaires
        .iter()
        .map(|g| {
            let a = g.cumuls.cef.annuel();
            [Poste::Chauffage, Poste::Refroidissement, Poste::Ecs]
                .into_iter()
                .map(|p| a.valeur(p, Energie::Gaz))
                .sum::<f64>()
        })
        .sum::<f64>()
        / 1000.0
}

/// Générateurs au gaz d'un projet, modifiables.
fn gaz_mut(p: &mut Projet) -> Vec<&mut ThermodynamiqueGazSaisi> {
    p.batiments[0]
        .generations
        .iter_mut()
        .flat_map(|g| g.generateurs.iter_mut())
        .filter_map(|x| match &mut x.nature {
            NatureGenerateur::ThermodynamiqueGaz(t) => Some(t),
            _ => None,
        })
        .collect()
}

#[test]
fn generateurs_au_gaz_sur_la_meteo_ademe() {
    // Maison à GAHP : gaz en chauffage et en ECS, électricité des auxiliaires ; Bbio de la
    // maison à chaudière (le Bbio ne dépend pas des systèmes).
    let gahp = cas("maison-gahp-ademe");
    let m = meteo_ademe(&gahp);
    let r = brut(&gahp, &m);
    let b = &r.batiments[0];
    let base = brut(&cas("maison-chaudiere-gaz-ademe"), &m);
    assert_eq!(b.bbio_pts, base.batiments[0].bbio_pts);
    let g = &b.generations[0];
    let c = &g.consommations.annuel;
    assert!(c.chauffage.gaz_kwh > 0.0 && c.ecs.gaz_kwh > 0.0);
    assert!(c.chauffage.electricite_kwh > 0.0 && c.ecs.electricite_kwh > 0.0);
    // Énergie fournie : toute la demande (pas de report en fin d'année).
    assert!((g.q_fou_ch_kwh - g.q_ch_kwh).abs() < 1e-6);
    // Rendement saisonnier sur gaz inférieur au GUE pivot (1,2) : charge partielle et pertes.
    let eta = g.q_fou_ch_kwh / c.chauffage.gaz_kwh;
    assert!(eta > 0.3 && eta < 1.2, "{eta}");
    for v in [
        "bruleur",
        "echangeur_fumees",
        "p_aux_0",
        "pertes_40c",
        "rdt_comb_pivot",
    ] {
        assert!(
            g.generateurs[0]
                .valeurs_par_defaut
                .iter()
                .any(|d| d.champ.ends_with(v)),
            "{v} non signalé"
        );
    }

    // Q139 : sans les pertes à l'arrêt de l'éq. (1345), la consommation baisse.
    let gaz = gaz_th_c(&gahp, &m, |_| {});
    let sans_pertes = gaz_th_c(&gahp, &m, |x| {
        if let Generateur::Absorption(a) = x {
            a.pertes_40c_w = 0.0;
        }
    });
    // Q137 : brûleur modulant au lieu de tout ou rien.
    let modulant = gaz_th_c(&gahp, &m, |x| {
        if let Generateur::Absorption(a) = x {
            a.modulant = true;
            a.lr_contmin = if a.poste == Poste::Ecs { 0.65 } else { 0.70 };
            a.ccp_lr_contmin = if a.poste == Poste::Ecs { 0.75 } else { 0.90 };
        }
    });
    println!(
        "maison-gahp : gaz {gaz:.1} kWh ; sans pertes {sans_pertes:.1} ; modulant {modulant:.1}"
    );
    assert!(sans_pertes < gaz);
    assert!(modulant.is_finite() && modulant > 0.0);

    // PAC à moteur gaz air/air réversible : gaz dans les deux modes.
    let mg = cas("maison-pac-moteur-gaz-air-air-ademe");
    let r = brut(&mg, &m);
    let c = &r.batiments[0].generations[0].consommations.annuel;
    assert!(c.chauffage.gaz_kwh > 0.0 && c.refroidissement.gaz_kwh > 0.0);
    assert!(c.chauffage.electricite_kwh > 0.0);
    // Q143 : consommation sans les irréversibilités (variante littérale), plus faible.
    use thbce_core::chauffage_refroidissement::pac_moteur_gaz::LectureConsommation;
    let avec = gaz_th_c(&mg, &m, |_| {});
    let litterale = gaz_th_c(&mg, &m, |x| {
        if let Generateur::MoteurGaz(p) = x {
            p.lecture = LectureConsommation::Litterale;
        }
    });
    // Q141 : sans le seuil θam,ms.
    let sans_seuil = gaz_th_c(&mg, &m, |x| {
        if let Generateur::MoteurGaz(p) = x {
            if let Some(ch) = &mut p.chauffage {
                ch.theta_am_ms_c = None;
            }
        }
    });
    println!(
        "maison-pac-mg : gaz {avec:.1} kWh ; littérale {litterale:.1} ; sans θam,ms {sans_seuil:.1}"
    );
    assert!(litterale < avec);

    // Commerce : PAC MG air/eau en cascade avec un appoint électrique.
    let co = cas("commerce-pac-moteur-gaz-ademe");
    let r = brut(&co, &meteo_ademe(&co));
    let c = &r.batiments[0].generations[0].consommations.annuel;
    assert!(c.chauffage.gaz_kwh > 0.0);
}

#[test]
fn refus_et_erreurs() {
    use thbce_core::entree::{ModeAbsorptionGaz, TechnologieChauffageGahp};
    // GAHP sur eau glycolée : hors périmètre (code 3), seul motif.
    let mut p = cas("maison-gahp-ademe");
    if let ThermodynamiqueGazSaisi::Absorption(a) = gaz_mut(&mut p).remove(0) {
        if let ModeAbsorptionGaz::Chauffage { technologie, .. } = &mut a.mode {
            *technologie = TechnologieChauffageGahp::EauGlycoleeEauClassique;
        }
    }
    let r = thbce_core::valider(&p);
    assert!(r.seulement_hors_perimetre(), "{:?}", r.erreurs);

    // Production indirecte d'ECS d'une PAC à moteur gaz : hors périmètre.
    let mut p = cas("maison-pac-moteur-gaz-air-air-ademe");
    if let ThermodynamiqueGazSaisi::MoteurGaz(m) = gaz_mut(&mut p).remove(0) {
        m.production_ecs_indirecte = Some(thbce_core::entree::ProductionEcsIndirecteSaisie {
            eta_th_ecs: None,
            p_pompe_ecs_w: 50.0,
        });
    }
    let r = thbce_core::valider(&p);
    assert!(r.seulement_hors_perimetre(), "{:?}", r.erreurs);

    // GAHP d'ECS sans ballon : erreur.
    let mut p = cas("maison-gahp-ademe");
    p.batiments[0].generations[0].stockages.clear();
    let r = thbce_core::valider(&p);
    assert!(!r.est_valide() && !r.seulement_hors_perimetre());
    assert!(r.erreurs.iter().any(|e| e.chemin.ends_with("mode.ecs")));

    // PAC à moteur gaz air/eau sur des réseaux fictifs : erreur de cohérence (§8.17.3.1).
    let mut p = cas("maison-pac-moteur-gaz-air-air-ademe");
    if let ThermodynamiqueGazSaisi::MoteurGaz(m) = gaz_mut(&mut p).remove(0) {
        m.chauffage.as_mut().unwrap().technologie =
            thbce_core::entree::TechnologiePacMoteurGaz::AirExterieurEau;
    }
    let r = thbce_core::valider(&p);
    assert!(
        r.erreurs
            .iter()
            .any(|e| e.message.contains("technologie incohérente")),
        "{:?}",
        r.erreurs
    );
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn generateurs_au_gaz_sur_la_meteo_reglementaire() {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO");
    let octets = std::fs::read(chemin).unwrap();
    let un = |x: f64| (x * 10.0).round() / 10.0;
    // Valeurs phares (arrondies à une décimale) : Cep, Cep,nr, Ic énergie, DH.
    let mut ecarts = Vec::new();
    for (nom, phares) in [
        ("maison-gahp-ademe", PHARES_GAHP),
        ("maison-pac-moteur-gaz-air-air-ademe", PHARES_MG_AIR_AIR),
        ("commerce-pac-moteur-gaz-ademe", PHARES_MG_COMMERCE),
    ] {
        let p = cas(nom);
        let m = meteo::lire(&octets, p.localisation.zone_climatique_retenue()).unwrap();
        let r = brut(&p, &m);
        let b = &r.batiments[0];
        let i = b.indicateurs.as_ref().unwrap();
        let obtenu = [
            un(i.cep_kwhep_m2),
            un(i.cep_nr_kwhep_m2),
            un(i.ic_energie_kgco2_m2.unwrap()),
            un(b.dh_c_h.unwrap()),
        ];
        println!("{nom} : Cep, Cep,nr, Ic énergie, DH = {obtenu:?}");
        if obtenu != phares {
            ecarts.push(nom);
        }
    }
    assert!(ecarts.is_empty(), "{ecarts:?}");
}

const PHARES_GAHP: [f64; 4] = [152.7, 152.7, 1118.7, 1064.8];
const PHARES_MG_AIR_AIR: [f64; 4] = [133.7, 133.7, 523.7, 1042.9];
const PHARES_MG_COMMERCE: [f64; 4] = [141.9, 141.9, 491.6, 1938.5];
