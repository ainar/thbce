// SPDX-License-Identifier: GPL-3.0-or-later
//! Bindings JavaScript du moteur Th-BCE 2020.
//!
//! ```js
//! import init, { calculer, valider, versionMoteur } from "./pkg/thbce_wasm.js";
//! await init();
//! const meteo = new Uint8Array(await fichierMeteo.arrayBuffer());
//! const resultats = calculer(projet, meteo, { arrondir: true });
//! ```
//!
//! Le module n'embarque aucune donnée météo : l'appelant passe le contenu du fichier. Les
//! fonctions sont synchrones et sans état ; pour ne pas bloquer l'interface, les appeler depuis
//! un Web Worker.

use serde::{Deserialize, Serialize};
use thbce_core::entree::Projet;
use thbce_core::{Erreur, Options, VERSION_METHODE, VERSION_MOTEUR};
use thbce_io::{json, meteo};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &'static str = r#"
/** Options de `calculer`. */
export interface OptionsCalcul {
  /** Arrondit les résultats à une décimale (Th-BCE 2020 §1.3.1). Vrai par défaut. */
  arrondir?: boolean;
}
/** Problème relevé dans l'entrée. */
export interface Probleme {
  chemin: string;
  valeur?: string;
  message: string;
}
/** Rapport de validation d'une entrée. */
export interface RapportValidation {
  valide: boolean;
  erreurs: Probleme[];
  avertissements: Probleme[];
}
/** Provenance des données météo. */
export type OrigineMeteo =
  | { type: "reglementaire_re2020" }
  | { type: "ademe_prospectif"; scenario: string }
  | { type: "autre"; description: string };
/** Période de confort adaptatif : jours de 1 à 365, `null` si le seuil n'est jamais atteint. */
export interface PeriodeConfortAdaptatif { premier_jour: number | null; dernier_jour: number | null; }
export interface ResultatsClimat {
  zone_climatique: "H1a" | "H1b" | "H1c" | "H2a" | "H2b" | "H2c" | "H2d" | "H3";
  altcorr_m: number;
  base_ext_c: number;
  confort_adaptatif_th_bc: PeriodeConfortAdaptatif;
  confort_adaptatif_th_d: PeriodeConfortAdaptatif;
}
export interface ResultatsEnveloppe {
  a_op_m2: number;
  a_opv_m2: number;
  a_ophh_m2: number;
  a_ophb_m2: number;
  h_th_op_w_k: number;
  a_baies_m2: number;
  a_baies_v_m2: number;
  a_baies_h_m2: number;
  h_ges_sp_w_k: number;
  a_f_perm_m2: number;
  a_toit_perm_m2: number;
  l_pt_m: number;
  h_th_pt_w_k: number;
}
export interface ValeurParDefaut { champ: string; valeur: number; regle: string; }
export interface ResultatsBaie { id: string; valeurs_par_defaut: ValeurParDefaut[]; }
/** Besoins (kWh par m² de surface de référence) et Bbio (points) d'un mois ou de l'année. */
export interface BesoinsBbio { b_ch_kwh_m2: number; b_fr_kwh_m2: number; b_ecl_kwh_m2: number; bbio_pts: number; }
/** Sorties du mode Th-B : année et douze mois de l'année conventionnelle. */
export interface ResultatsBbio { annuel: BesoinsBbio; mensuel: BesoinsBbio[]; }
/** Débits du système de ventilation conventionnel du Bbio, en m³/h (repris négatif). */
export interface ResultatsDebitsBbio { q_spec_repris_m3_h: number; q_spec_souffle_m3_h: number; }
/** Saisons propres du groupe (calcul du Bbio ou calcul Th-D) : jours de 1 à 365, `null` si l'événement n'a pas eu lieu. */
export interface ResultatsSaisons {
  j_fin_ch: number | null;
  j_debut_ch: number | null;
  j_debut_fr: number | null;
  j_fin_fr: number | null;
}
/** Indicateurs de confort du groupe en évolution libre : heures et degrés-heures d'inconfort. */
export interface ResultatsConfort {
  nbh_inc_cons_ch: number;
  dh_inc_cons_ch_k_h: number;
  nbh_inc_cons_fr: number;
  dh_inc_cons_fr_k_h: number;
  nbh_inc_c_fr: [number, number, number];
  dh_inc_c_fr_k_h: [number, number, number];
}
/** Confort d'été du groupe, issu du calcul Th-D : DH en °C.h, heures d'inconfort, températures opératives maximales en °C. */
export interface ResultatsConfortEte {
  dh_c_h: number;
  nbh_inconf: number;
  nbh_inconf_1c: number;
  nbh_inconf_2c: number;
  nbh_occ_conf_adapt: number;
  theta_op_max_c: number;
  theta_op_max_occ_c: number | null;
}
export interface ResultatsGroupe {
  id: string;
  sref_m2: number;
  enveloppe: ResultatsEnveloppe;
  debits_bbio: ResultatsDebitsBbio | null;
  bbio: ResultatsBbio | null;
  saisons_bbio: ResultatsSaisons | null;
  confort_bbio: ResultatsConfort | null;
  confort_ete: ResultatsConfortEte | null;
  saisons_th_d: ResultatsSaisons | null;
  baies: ResultatsBaie[];
  valeurs_par_defaut: ValeurParDefaut[];
}
export interface ResultatsZone {
  id: string;
  sref_m2: number;
  nadeq: number | null;
  bbio: ResultatsBbio | null;
  /** DH du groupe le plus défavorable de la zone, en °C.h. */
  dh_c_h: number | null;
  groupes: ResultatsGroupe[];
}
export interface ResultatsBatiment {
  id: string;
  sref_m2: number;
  bbio_pts: number | null;
  bbio: ResultatsBbio | null;
  cep_kwhep_m2: number | null;
  cep_nr_kwhep_m2: number | null;
  dh_c_h: number | null;
  ic_energie_kgco2_m2: number | null;
  zones: ResultatsZone[];
  /** Espaces tampons du bâtiment ; absent sans espace tampon. */
  espaces_tampons?: ResultatsEspaceTampon[];
  /** Production photovoltaïque (§12) ; absente sans installation. */
  production_pv?: ResultatsProductionPv;
  /** Indicateurs réglementaires (§13.2 à 13.4, Ic énergie, exigences) ; absents sans génération. */
  indicateurs?: ResultatsIndicateurs;
}
export interface PostesElectricite {
  chauffage_kwh: number;
  refroidissement_kwh: number;
  ecs_kwh: number;
  eclairage_kwh: number;
  auxiliaires_ventilation_kwh: number;
  auxiliaires_distribution_kwh: number;
  deplacements_kwh: number;
  mobilier_kwh: number;
}
export interface LigneBilanEnergie {
  poste: string;
  energie: string;
  cef_kwh: number;
  coef_ep: number;
  coef_ep_nr: number;
  facteur_emission_kg_kwh: number | null;
  cep_kwhep_m2: number;
  cep_nr_kwhep_m2: number;
  ic_energie_kgco2_m2: number | null;
}
export interface VerificationExigence {
  indicateur: "bbio" | "cep" | "cep_nr" | "ic_energie" | "dh";
  groupe?: string;
  valeur: number;
  maximum: number;
  marge: number;
  marge_pct: number;
  respectee: boolean;
}
export interface ResultatsExigences {
  bbio_max_pts: number;
  cep_max_kwhep_m2: number;
  cep_nr_max_kwhep_m2: number;
  ic_energie_max_kgco2_m2: number;
  zones: ExigencesZone[];
  verifications: VerificationExigence[];
  /** Toujours faux si un indicateur exigé manque. */
  conforme: boolean;
  indicateurs_manquants?: string[];
}
/** Exigences d'une zone et coefficients de modulation (annexe de l'article R. 172-4). */
export interface ExigencesZone {
  id: string;
  usage: number;
  sref_m2: number;
  smoy_lgt_m2: number;
  bbio_max_moyen: number;
  mb_geo: number;
  mb_combles: number;
  mb_surf_moy: number;
  mb_surf_tot: number;
  mb_bruit: number;
  mb_hsp: number;
  bbio_max_pts: number;
  cep_max_moyen: number;
  cep_nr_max_moyen: number;
  ic_energie_max_moyen: number;
  mc_geo: number;
  mc_combles: number;
  mc_surf_moy: number;
  mc_surf_tot: number;
  mc_cat: number;
  mc_hsp: number;
  cep_max_kwhep_m2: number;
  cep_nr_max_kwhep_m2: number;
  ic_energie_max_kgco2_m2: number;
}
export interface ResultatsIndicateurs {
  sref_m2: number;
  electricite_consommee: PostesElectricite;
  electricite_importee: PostesElectricite;
  electricite_importee_mensuel_kwh: number[];
  production_pv?: {
    e_prod_pv_kwh: number;
    e_prod_pv_ac_kwh: number;
    e_exportee_kwh: number;
    tac_pct: number;
    e_prod_pv_mensuel_kwh: number[];
    e_prod_pv_ac_mensuel_kwh: number[];
  };
  forfaits_refroidissement?: {
    zone: string;
    groupe: string;
    dh_c_h: number;
    seuil_haut_c_h: number | null;
    cef_kwh_m2: number;
    cef_kwh: number;
  }[];
  energie_solaire_captee_kwh?: number;
  lignes: LigneBilanEnergie[];
  cef_kwhef_m2: number;
  cep_kwhep_m2: number;
  cep_nr_kwhep_m2: number;
  ic_energie_kgco2_m2: number | null;
  somme_f_co2: number;
  exigences?: ResultatsExigences;
}
export interface ResultatsInstallationPv {
  id: string;
  nb_modules: number;
  puissance_crete_calcul_kwc: number;
  eta_stc: number;
  u1_w_m2k: number;
  p_ac_nom_w: number;
  g_kwh_m2: number;
  e_pv_kwh: number;
  e_ond_kwh: number;
  e_ond_mensuel_kwh: number[];
  nbh_extinction: number;
  valeurs_par_defaut?: ValeurParDefaut[];
}
export interface ResultatsProductionPv {
  e_prod_pv_kwh: number;
  e_prod_pv_mensuel_kwh: number[];
  installations: ResultatsInstallationPv[];
}
export interface ResultatsEspaceTamponMode {
  b_therm_moyen: number;
  theta_et_moy_c: number;
  theta_et_min_c: number;
  theta_et_max_c: number;
  nbh_surventilation: number;
}
export interface ResultatsEspaceTampon {
  id: string;
  th_b: ResultatsEspaceTamponMode | null;
  th_d: ResultatsEspaceTamponMode | null;
  valeurs_par_defaut: ValeurParDefaut[];
}
/** Résultats d'un calcul. `reglementaire` est faux dès que la météo n'est pas celle de la RE2020. */
export interface Resultats {
  version_moteur: string;
  version_methode: string;
  reglementaire: boolean;
  origine_meteo_th_bc: OrigineMeteo;
  origine_meteo_th_d: OrigineMeteo;
  valeurs_arrondies: boolean;
  avertissements: string[];
  climat: ResultatsClimat;
  batiments: ResultatsBatiment[];
}
/** Erreur levée par `calculer` (propriété `cause` de l'exception). */
export interface ErreurCalcul {
  nature: "lecture" | "saisie" | "meteo" | "non_pris_en_charge" | "calcul";
  message: string;
  rapport?: RapportValidation;
}
"#;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct OptionsJs {
    arrondir: Option<bool>,
}

#[derive(Serialize)]
struct RapportJs<'a> {
    valide: bool,
    erreurs: &'a [thbce_core::validation::Probleme],
    avertissements: &'a [thbce_core::validation::Probleme],
}

impl<'a> RapportJs<'a> {
    fn depuis(r: &'a thbce_core::validation::RapportValidation) -> Self {
        Self {
            valide: r.est_valide(),
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
}

/// Lit l'entrée : objet JavaScript ou texte JSON.
fn lire_entree(entree: JsValue) -> Result<Projet, JsValue> {
    if let Some(texte) = entree.as_string() {
        json::lire_projet(&texte).map_err(|e| erreur("lecture", e.to_string(), None))
    } else {
        serde_wasm_bindgen::from_value(entree).map_err(|e| erreur("lecture", e.to_string(), None))
    }
}

/// Calcule les indicateurs d'un projet.
///
/// `entree` : description du projet (objet ou texte JSON). `meteo` : contenu du fichier météo,
/// soit le CSV dérivé du classeur réglementaire, soit un fichier ouvert ADEME / Météo-France
/// (non réglementaire). Le classeur `.xlsx` lui-même n'est pas lu par le module WebAssembly. Lève une `Error` dont la propriété `cause`
/// est un `ErreurCalcul`.
#[wasm_bindgen(unchecked_return_type = "Resultats")]
pub fn calculer(
    #[wasm_bindgen(unchecked_param_type = "object | string")] entree: JsValue,
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
    let options = Options {
        arrondir: options.arrondir.unwrap_or(true),
    };
    match thbce_core::calculer(&projet, &donnees, &options) {
        Ok(resultats) => vers_js(&resultats),
        Err(e) => Err(match &e {
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
    #[wasm_bindgen(unchecked_param_type = "object | string")] entree: JsValue,
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
