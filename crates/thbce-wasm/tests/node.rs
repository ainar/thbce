// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests du module WebAssembly, exécutés sous Node par `wasm-pack test --node`.
#![cfg(target_arch = "wasm32")]

use thbce_core::climat::donnees::DESCRIPTION_SYNTHETIQUE;
use thbce_core::climat::MeteoProjet;
use thbce_data::ZoneClimatique;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;

const ENTREE: &str = include_str!("../../../tests/cas/squelette-maison/entree.json");

fn meteo() -> Vec<u8> {
    thbce_io::meteo::ecrire_csv(
        &[MeteoProjet::synthetique(ZoneClimatique::H2b)],
        DESCRIPTION_SYNTHETIQUE,
    )
    .into_bytes()
}

fn champ(objet: &JsValue, cle: &str) -> JsValue {
    js_sys::Reflect::get(objet, &JsValue::from_str(cle)).unwrap()
}

#[wasm_bindgen_test]
fn version() {
    assert!(thbce_wasm::version_moteur().starts_with("thbce "));
}

#[wasm_bindgen_test]
fn calcule_le_cas_de_demonstration() {
    let r = thbce_wasm::calculer(JsValue::from_str(ENTREE), &meteo(), JsValue::UNDEFINED).unwrap();
    assert_eq!(champ(&r, "reglementaire"), JsValue::FALSE);
    let batiments = js_sys::Array::from(&champ(&r, "batiments"));
    assert_eq!(batiments.length(), 1);
    assert_eq!(champ(&batiments.get(0), "sref_m2").as_f64(), Some(96.4));
    // Jeu synthétique à 10 °C sans soleil : le besoin de chauffage, donc le Bbio, est positif.
    let bbio = champ(&batiments.get(0), "bbio_pts").as_f64();
    assert!(bbio.is_some_and(|b| b > 0.0), "{bbio:?}");
    assert!(champ(&batiments.get(0), "cep_kwhep_m2").is_null());
    // Le jeu « Th-D » synthétique n'a pas de période de confort adaptatif : DH calculé, nul.
    assert_eq!(champ(&batiments.get(0), "dh_c_h").as_f64(), Some(0.0));
}

#[wasm_bindgen_test]
fn valide_et_signale_les_erreurs() {
    let r = thbce_wasm::valider(JsValue::from_str(ENTREE)).unwrap();
    assert_eq!(champ(&r, "valide"), JsValue::TRUE);
    let fautive = ENTREE.replace("\"shab_m2\": 96.44", "\"shab_m2\": -1");
    let e = thbce_wasm::calculer(JsValue::from_str(&fautive), &meteo(), JsValue::UNDEFINED)
        .unwrap_err();
    assert_eq!(
        champ(&champ(&e, "cause"), "nature").as_string().as_deref(),
        Some("saisie")
    );
}
