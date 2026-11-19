// SPDX-License-Identifier: GPL-3.0-or-later
//! Bindings JavaScript du moteur Th-BCE 2020.
//!
//! ```js
//! import init, { calculer, valider, versionMoteur, schemaEntree, schemaResultats }
//!   from "./pkg/thbce_wasm.js";
//! await init();
//! const meteo = new Uint8Array(await fichierMeteo.arrayBuffer());
//! const resultats = calculer(projet, meteo, { arrondir: true });
//! ```
//!
//! Noms des fonctions : en camelCase, selon l'usage de JavaScript (`calculer`, `valider`,
//! `versionMoteur`, `schemaEntree`, `schemaResultats`) ; les noms des champs de l'entrée et des
//! résultats restent ceux du format JSON (snake_case). Les types `Projet` et `Resultats` du
//! fichier `.d.ts` sont générés depuis les JSON Schema.
//!
//! Le module n'embarque aucune donnée météo : l'appelant passe le contenu du fichier. Les
//! fonctions sont synchrones et sans état ; pour ne pas bloquer l'interface, les appeler depuis
//! un Web Worker.

use serde::{Deserialize, Serialize};
use thbce_core::entree::Projet;
use thbce_core::{Erreur, Options, VERSION_METHODE, VERSION_MOTEUR};
use thbce_io::{json, meteo};
use wasm_bindgen::prelude::*;

/// Types de l'entrée (`Projet`) et des résultats (`Resultats`), générés depuis les JSON Schema
/// par `cargo run -p xtask -- typescript` ; `xtask verifier` contrôle qu'ils sont à jour.
#[wasm_bindgen(typescript_custom_section)]
const TYPES_SCHEMAS: &'static str = include_str!("types.d.ts");

/// Types propres à l'interface JavaScript.
#[wasm_bindgen(typescript_custom_section)]
const TYPES: &'static str = r#"
/** Options de `calculer`. */
export interface OptionsCalcul {
  /** Arrondit les résultats à une décimale (Th-BCE 2020 §1.3.1). Vrai par défaut. */
  arrondir?: boolean;
  /**
   * Restitue en plus, pour chaque bâtiment, les séries horaires principales du calcul Th-C
   * (`batiments[].series_horaires` : température extérieure, besoins et températures des
   * groupes, électricité par poste, consommations par poste et par énergie ; 8 760 valeurs par
   * série). Faux par défaut : la restitution est alors inchangée.
   */
  series_horaires?: boolean;
}
/** Problème relevé dans l'entrée. */
export interface Probleme {
  chemin: string;
  valeur?: string;
  message: string;
  /** Vrai si l'élément ou l'usage est hors du périmètre de cette version (voir docs/couverture.md). */
  hors_perimetre?: boolean;
}
/**
 * Rapport de validation d'une entrée. `valide` est faux dès qu'une erreur est relevée, y
 * compris pour un élément hors du périmètre de cette version : l'entrée n'est alors pas
 * calculable.
 */
export interface RapportValidation {
  valide: boolean;
  /** Vrai si toutes les erreurs tiennent au périmètre de cette version (`hors_perimetre`). */
  hors_perimetre: boolean;
  erreurs: Probleme[];
  avertissements: Probleme[];
}
/** Erreur levée par `calculer` ou `valider` (propriété `cause` de l'exception). */
export interface ErreurCalcul {
  /**
   * `lecture` : JSON illisible ; `saisie` : entrée invalide ; `meteo` : données météo ;
   * `non_pris_en_charge` : entrée correcte mais hors du périmètre de cette version ;
   * `calcul` : calcul impossible.
   */
  nature: "lecture" | "saisie" | "meteo" | "non_pris_en_charge" | "calcul";
  message: string;
  rapport?: RapportValidation;
}
"#;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct OptionsJs {
    arrondir: Option<bool>,
    series_horaires: Option<bool>,
}

#[derive(Serialize)]
struct RapportJs<'a> {
    valide: bool,
    hors_perimetre: bool,
    erreurs: &'a [thbce_core::validation::Probleme],
    avertissements: &'a [thbce_core::validation::Probleme],
}

impl<'a> RapportJs<'a> {
    fn depuis(r: &'a thbce_core::validation::RapportValidation) -> Self {
        Self {
            valide: r.est_valide(),
            hors_perimetre: r.seulement_hors_perimetre(),
            erreurs: &r.erreurs,
            avertissements: &r.avertissements,
        }
    }
}

#[derive(Serialize)]
struct ErreurJs<'a> {
    nature: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    rapport: Option<RapportJs<'a>>,
}

fn vers_js<T: Serialize>(valeur: &T) -> Result<JsValue, JsValue> {
    // Objets JavaScript simples (et non `Map`), `null` pour les indicateurs absents.
    let s = serde_wasm_bindgen::Serializer::json_compatible();
    valeur
        .serialize(&s)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

fn erreur(nature: &'static str, message: String, rapport: Option<RapportJs<'_>>) -> JsValue {
    let cause = vers_js(&ErreurJs {
        nature,
        message: message.clone(),
        rapport,
    })
    .unwrap_or(JsValue::NULL);
    let e = js_sys_error(&message);
    let _ = js_set(&e, "cause", &cause);
    e
}

fn js_sys_error(message: &str) -> JsValue {
    JsError::new(message).into()
}

fn js_set(cible: &JsValue, cle: &str, valeur: &JsValue) -> Result<bool, JsValue> {
    reflect_set(cible, &JsValue::from_str(cle), valeur)
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Reflect, js_name = set, catch)]
    fn reflect_set(cible: &JsValue, cle: &JsValue, valeur: &JsValue) -> Result<bool, JsValue>;
    #[wasm_bindgen(js_namespace = JSON, js_name = stringify, catch)]
    fn json_stringify(valeur: &JsValue) -> Result<JsValue, JsValue>;
}

/// Lit l'entrée : objet JavaScript ou texte JSON. Un objet passe par son texte JSON, pour que
/// les erreurs de lecture portent le même chemin et le même message qu'en ligne de commande.
fn lire_entree(entree: JsValue) -> Result<Projet, JsValue> {
    let texte = match entree.as_string() {
        Some(texte) => texte,
        None => json_stringify(&entree)
            .ok()
            .and_then(|t| t.as_string())
            .ok_or_else(|| {
                erreur(
                    "lecture",
                    "l'entrée n'est ni un objet ni un texte JSON".into(),
                    None,
                )
            })?,
    };
    json::lire_projet(&texte).map_err(|e| erreur("lecture", e.to_string(), None))
}

/// Calcule les indicateurs d'un projet.
///
/// `entree` : description du projet (objet ou texte JSON). `meteo` : contenu du fichier météo,
/// soit le CSV dérivé du classeur réglementaire, soit un fichier ouvert ADEME / Météo-France
/// (non réglementaire). Le classeur `.xlsx` lui-même n'est pas lu par le module WebAssembly. Lève une `Error` dont la propriété `cause`
/// est un `ErreurCalcul`.
#[wasm_bindgen(unchecked_return_type = "Resultats")]
pub fn calculer(
    #[wasm_bindgen(unchecked_param_type = "Projet | string")] entree: JsValue,
    meteo: &[u8],
    #[wasm_bindgen(unchecked_param_type = "OptionsCalcul | undefined")] options: JsValue,
) -> Result<JsValue, JsValue> {
    let projet = lire_entree(entree)?;
    let options: OptionsJs = if options.is_undefined() || options.is_null() {
        OptionsJs::default()
    } else {
        serde_wasm_bindgen::from_value(options)
            .map_err(|e| erreur("lecture", format!("options : {e}"), None))?
    };
    let zone = projet.localisation.zone_climatique_retenue();
    let donnees = meteo::lire(meteo, zone).map_err(|e| erreur("meteo", e.to_string(), None))?;
    let series_horaires = options.series_horaires.unwrap_or(false);
    let options = Options {
        arrondir: options.arrondir.unwrap_or(true),
    };
    let calcul = if series_horaires {
        thbce_core::calculer_avec_series
    } else {
        thbce_core::calculer
    };
    match calcul(&projet, &donnees, &options) {
        Ok(resultats) => vers_js(&resultats),
        Err(e) => Err(match &e {
            Erreur::Saisie(r) if r.seulement_hors_perimetre() => erreur(
                "non_pris_en_charge",
                e.to_string(),
                Some(RapportJs::depuis(r)),
            ),
            Erreur::Saisie(r) => erreur("saisie", e.to_string(), Some(RapportJs::depuis(r))),
            Erreur::Meteo(_) => erreur("meteo", e.to_string(), None),
            Erreur::NonPrisEnCharge { .. } => erreur("non_pris_en_charge", e.to_string(), None),
            Erreur::Calcul(_) => erreur("calcul", e.to_string(), None),
        }),
    }
}

/// Valide la description d'un projet (objet ou texte JSON) sans la calculer.
#[wasm_bindgen(unchecked_return_type = "RapportValidation")]
pub fn valider(
    #[wasm_bindgen(unchecked_param_type = "Projet | string")] entree: JsValue,
) -> Result<JsValue, JsValue> {
    let projet = lire_entree(entree)?;
    vers_js(&RapportJs::depuis(&thbce_core::valider(&projet)))
}

/// Version du moteur et version de la méthode implémentée.
#[wasm_bindgen(js_name = versionMoteur)]
pub fn version_moteur() -> String {
    format!("thbce {VERSION_MOTEUR} — {VERSION_METHODE}")
}

/// JSON Schema du format d'entrée.
#[wasm_bindgen(js_name = schemaEntree)]
pub fn schema_entree() -> String {
    json::schema_entree()
}

/// JSON Schema des résultats.
#[wasm_bindgen(js_name = schemaResultats)]
pub fn schema_resultats() -> String {
    json::schema_resultats()
}
