// SPDX-License-Identifier: GPL-3.0-or-later
//! Classement au bruit des baies (fiche 5.11, lot « bruit », Q200 à Q204) de bout en bout :
//! variante de la maison d'exemple `tests/cas/maison-climat-ademe` où la classe de bruit se
//! déduit de la saisie baie par baie, sur la météo ouverte ADEME.
//!
//! Valeurs lues à la main sur les tableaux de la partie I (p. 249 de l'annexe III) : catégorie 3
//! à 130 m, tranche « 100-160 m » : vue directe BR2, vue partielle BR2, vue arrière BR1,
//! vue masquée par des obstacles très protecteurs BR1.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::path::Path;

use serde_json::{json, Value};
use thbce_core::climat::MeteoProjet;
use thbce_core::entree::Projet;
use thbce_core::{calculer, valider, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn maison() -> Value {
    let t = std::fs::read_to_string(racine().join("tests/cas/maison-climat-ademe/entree.json"))
        .unwrap();
    serde_json::from_str(&t).unwrap()
}

fn projet(v: &Value) -> Projet {
    json::lire_projet(&v.to_string()).unwrap()
}

fn meteo_ademe(p: &Projet) -> MeteoProjet {
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    meteo::lire(
        &std::fs::read(chemin).unwrap(),
        p.localisation.zone_climatique_retenue(),
    )
    .unwrap()
}

fn resultats(v: &Value) -> Value {
    let p = projet(v);
    let r = calculer(&p, &meteo_ademe(&p), &Options::default()).unwrap();
    serde_json::to_value(r).unwrap()
}

fn baies(v: &mut Value) -> &mut Value {
    &mut v["batiments"][0]["zones"][0]["groupes"][0]["baies"]
}

fn messages(v: &Value) -> Vec<String> {
    valider(&projet(v))
        .erreurs
        .iter()
        .map(|e| format!("{} : {}", e.chemin, e.message))
        .collect()
}

fn infra(cat: &str, d: f64, vue: &str) -> Value {
    json!({ "categorie": cat, "distance_m": d, "vue": vue })
}

/// Saisie baie par baie sans classe de bruit du bâtiment : mêmes résultats que la saisie directe
/// de la classe lue sur les tableaux (BR2 pour les deux baies, donc zone BR2).
#[test]
fn classe_deduite_remplace_la_saisie() {
    let mut directe = maison();
    directe["batiments"][0]["exigences"]["zone_bruit"] = "br2".into();
    for b in baies(&mut directe).as_array_mut().unwrap() {
        b["expo_br"] = "br2".into();
    }

    let mut detaillee = maison();
    detaillee["batiments"][0]["exigences"]
        .as_object_mut()
        .unwrap()
        .remove("zone_bruit");
    let b = baies(&mut detaillee).as_array_mut().unwrap();
    b[0].as_object_mut().unwrap().remove("expo_br");
    b[1].as_object_mut().unwrap().remove("expo_br");
    b[0]["bruit"] = json!({ "infrastructures": [infra("categorie_3", 130.0, "directe")] });
    b[1]["bruit"] = json!({ "infrastructures": [infra("categorie_3", 130.0, "partielle")] });

    assert!(
        messages(&detaillee).is_empty(),
        "{:?}",
        messages(&detaillee)
    );
    assert_eq!(resultats(&detaillee), resultats(&directe));
    // La classe BR2 change bien le résultat par rapport à la maison d'origine (BR1).
    assert_ne!(resultats(&maison()), resultats(&directe));
}

/// Le classement détaillé en BR1 reproduit la maison d'origine.
#[test]
fn classe_br1_deduite_comme_la_maison() {
    let mut v = maison();
    v["batiments"][0]["exigences"]
        .as_object_mut()
        .unwrap()
        .remove("zone_bruit");
    for b in baies(&mut v).as_array_mut().unwrap() {
        b.as_object_mut().unwrap().remove("expo_br");
        // Catégorie 3 à 130 m, vue arrière : BR1.
        b["bruit"] = json!({ "infrastructures": [infra("categorie_3", 130.0, "arriere")] });
    }
    assert_eq!(resultats(&v), resultats(&maison()));
}

/// Règle de la façade : la baie sans détermination détaillée prend la classe la plus élevée des
/// baies de sa façade.
#[test]
fn regle_de_la_facade() {
    let mut v = maison();
    v["batiments"][0]["exigences"]
        .as_object_mut()
        .unwrap()
        .remove("zone_bruit");
    let b = baies(&mut v).as_array_mut().unwrap();
    b[0].as_object_mut().unwrap().remove("expo_br");
    b[1].as_object_mut().unwrap().remove("expo_br");
    // Sud : BR2 (vue directe) ; nord : BR1 par la façade ? Non : même façade « f » -> BR2.
    b[0]["bruit"] = json!({
        "infrastructures": [infra("categorie_3", 130.0, "directe")], "facade": "f" });
    b[1]["bruit"] = json!({ "facade": "f" });
    assert!(messages(&v).is_empty(), "{:?}", messages(&v));
    let mut directe = maison();
    directe["batiments"][0]["exigences"]["zone_bruit"] = "br2".into();
    for b in baies(&mut directe).as_array_mut().unwrap() {
        b["expo_br"] = "br2".into();
    }
    assert_eq!(resultats(&v), resultats(&directe));
    // Façade sans aucune baie déterminée : erreur.
    baies(&mut v)[0]["bruit"] = json!({ "facade": "f" });
    assert!(messages(&v)
        .iter()
        .any(|m| m.contains("aucune baie de cette façade")));
}

#[test]
fn incoherences_et_absences() {
    // Classe saisie pour le bâtiment (BR1) et déduite des baies (BR2) : incohérence.
    let mut v = maison();
    for b in baies(&mut v).as_array_mut().unwrap() {
        b.as_object_mut().unwrap().remove("expo_br");
        b["bruit"] = json!({ "infrastructures": [infra("categorie_3", 130.0, "directe")] });
    }
    let m = messages(&v);
    assert!(
        m.iter()
            .any(|m| m.contains("zone_bruit") && m.contains("incohérente")),
        "{m:?}"
    );
    // expo_br saisi (BR1) incohérent avec la détermination détaillée (BR2).
    let mut w = maison();
    baies(&mut w)[0]["bruit"] =
        json!({ "infrastructures": [infra("categorie_3", 130.0, "directe")] });
    let m = messages(&w);
    assert!(
        m.iter()
            .any(|m| m.contains("expo_br") && m.contains("incohérente")),
        "{m:?}"
    );
    // Sans `bruit` sur aucune baie, les `expo_br` seuls ne fixent pas la classe de la zone : ni
    // saisie, ni déduction possible.
    let mut x = maison();
    x["batiments"][0]["exigences"]
        .as_object_mut()
        .unwrap()
        .remove("zone_bruit");
    let m = messages(&x);
    assert!(
        m.iter()
            .any(|m| m.contains("classe d'exposition au bruit de la zone")),
        "{m:?}"
    );
    // Baie sans aucune donnée de bruit.
    let mut y = maison();
    baies(&mut y)[0]["bruit"] = json!({});
    assert!(messages(&y)
        .iter()
        .any(|m| m.contains("renseigner au moins une infrastructure")));
}
