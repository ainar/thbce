// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation de la description d'un projet, séparée du calcul.
//!
//! Les contraintes vérifiées sont les plages des nomenclatures de l'annexe III (colonnes
//! « Min » et « Max »), l'unicité des identifiants et l'existence des références au catalogue.

use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thbce_data::ZoneClimatique;

use crate::entree::{BaieType, Masque, Projet};
use baies::valider_baie_type;
use batiment::{valider_coherence_zone, valider_zone};
use enveloppe::{valider_exposition, valider_masque, valider_paroi_type};
use espaces_tampons::valider_espaces_tampons;
use exigences::valider_exigences;

// Un fichier par famille. Un lot ajoute ses contrôles dans le fichier de sa famille, et une
// nouvelle famille par un `mod` ci-dessous et un appel dans `valider`.
mod acv;
mod baies;
mod batiment;
mod brasseurs_air;
mod bruit;
mod collectif;
mod eclairage;
mod ecs;
mod enveloppe;
mod espaces_tampons;
mod exigences;
mod generation;
mod photovoltaique;
mod solaire;
mod systemes;
mod usages_batiment;
mod ventilation;
// Famille à venir : ACV (Icconstruction, annexe II) — `mod acv;`.

/// Nombre maximal de masques protégeant une surface (Th-BCE 2020 §3.2.2, « Nmasque »).
pub const NB_MASQUES_MAX: usize = 7;

/// Nombre de tranches d'un masque lointain azimutal (Th-BCE 2020 §3.2.3).
pub const NB_TRANCHES_AZIMUTALES: usize = 36;

/// Un problème relevé dans l'entrée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Probleme {
    /// Chemin de l'élément dans l'entrée, par exemple
    /// `batiments[0].zones[1].groupes[0].baies[3].a_b_m2`.
    pub chemin: String,
    /// Valeur fautive, quand elle est simple à afficher.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valeur: Option<String>,
    /// Contrainte non respectée, ou nature de l'avertissement.
    pub message: String,
    /// Vrai si l'erreur tient à un élément ou un usage que cette version du moteur ne calcule
    /// pas (`docs/couverture.md`), et non à une saisie fautive.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hors_perimetre: bool,
}

/// Altitude maximale admise, en m.
pub(crate) const ALTITUDE_MAX_M: f64 = 4000.0;

/// Fin des messages d'erreur des éléments et usages hors du périmètre de cette version.
pub(crate) const HORS_VERSION: &str =
    "non pris en charge par cette version (voir docs/couverture.md)";

/// Nom d'une valeur d'énumération dans le format JSON (snake_case).
pub(crate) fn nom_json<T: Serialize>(v: &T) -> String {
    match serde_json::to_value(v) {
        Ok(serde_json::Value::String(s)) => s,
        Ok(autre) => autre.to_string(),
        Err(_) => String::new(),
    }
}

/// Rapport de validation d'une entrée.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RapportValidation {
    /// Erreurs : l'entrée ne peut pas être calculée.
    pub erreurs: Vec<Probleme>,
    /// Avertissements : l'entrée est acceptée, mais un point mérite l'attention.
    pub avertissements: Vec<Probleme>,
}

impl RapportValidation {
    /// Vrai si l'entrée ne comporte aucune erreur.
    pub fn est_valide(&self) -> bool {
        self.erreurs.is_empty()
    }

    /// Vrai si l'entrée comporte des erreurs et qu'elles tiennent toutes à des éléments ou des
    /// usages hors du périmètre de cette version : l'entrée est correcte mais non calculable.
    pub fn seulement_hors_perimetre(&self) -> bool {
        !self.erreurs.is_empty() && self.erreurs.iter().all(|e| e.hors_perimetre)
    }

    /// Élément ou usage hors du périmètre de cette version (erreur, `hors_perimetre`).
    fn hors_perimetre(
        &mut self,
        chemin: impl Into<String>,
        valeur: Option<String>,
        element: impl std::fmt::Display,
    ) {
        self.erreurs.push(Probleme {
            chemin: chemin.into(),
            valeur,
            message: format!("{element} : {HORS_VERSION}"),
            hors_perimetre: true,
        });
    }

    fn erreur(
        &mut self,
        chemin: impl Into<String>,
        valeur: Option<String>,
        message: impl Into<String>,
    ) {
        self.erreurs.push(Probleme {
            chemin: chemin.into(),
            valeur,
            message: message.into(),
            hors_perimetre: false,
        });
    }

    fn avertissement(
        &mut self,
        chemin: impl Into<String>,
        valeur: Option<String>,
        message: impl Into<String>,
    ) {
        self.avertissements.push(Probleme {
            chemin: chemin.into(),
            valeur,
            message: message.into(),
            hors_perimetre: false,
        });
    }

    /// Vérifie `min <= v <= max` ; une borne infinie n'est pas contrôlée.
    fn plage(&mut self, chemin: String, v: f64, min: f64, max: f64) {
        if !v.is_finite() {
            self.erreur(chemin, Some(v.to_string()), "valeur non finie");
        } else if v < min || v > max {
            let contrainte = if max.is_infinite() {
                format!("doit être supérieur ou égal à {min}")
            } else {
                format!("doit être compris entre {min} et {max}")
            };
            self.erreur(chemin, Some(v.to_string()), contrainte);
        }
    }

    fn positif(&mut self, chemin: String, v: f64) {
        self.plage(chemin, v, 0.0, f64::INFINITY);
    }

    fn strictement_positif(&mut self, chemin: String, v: f64) {
        if v.is_finite() && v <= 0.0 {
            self.erreur(chemin, Some(v.to_string()), "doit être strictement positif");
        } else {
            self.positif(chemin, v);
        }
    }

    /// Vérifie l'unicité d'une famille d'identifiants et renvoie leur ensemble.
    fn ids_uniques<'a>(
        &mut self,
        chemin: &str,
        ids: impl Iterator<Item = &'a String>,
    ) -> BTreeSet<&'a str> {
        let mut vus = BTreeSet::new();
        for (i, id) in ids.enumerate() {
            if id.trim().is_empty() {
                self.erreur(format!("{chemin}[{i}].id"), None, "identifiant vide");
            } else if !vus.insert(id.as_str()) {
                self.erreur(
                    format!("{chemin}[{i}].id"),
                    Some(id.clone()),
                    "identifiant déjà utilisé par un autre élément de la même liste",
                );
            }
        }
        vus
    }
}

/// Valide la description d'un projet.
pub fn valider(projet: &Projet) -> RapportValidation {
    let mut r = RapportValidation::default();

    if let Some(v) = projet.version_format {
        if v != crate::entree::VERSION_FORMAT {
            r.erreur(
                "version_format",
                Some(v.to_string()),
                format!(
                    "version du format d'entrée inconnue : cette version du moteur lit la version {}",
                    crate::entree::VERSION_FORMAT
                ),
            );
        }
    }

    // Th-BCE 2020 §3.1.2 : Alt, Dmer, Durb, minimum 0.
    let loc = &projet.localisation;
    // Borne haute : aucun bâtiment d'habitation n'est construit en France au-delà de 4 000 m ;
    // une valeur plus forte est une faute de saisie (unité, par exemple).
    r.plage("localisation.alt_m".into(), loc.alt_m, 0.0, ALTITUDE_MAX_M);
    r.positif("localisation.dmer_km".into(), loc.dmer_km);
    r.positif("localisation.durb_km".into(), loc.durb_km);
    let zone_departement = ZoneClimatique::du_departement(loc.departement);
    if let Some(zone) = loc.zone_climatique {
        if zone != zone_departement {
            r.avertissement(
                "localisation.zone_climatique",
                Some(zone.to_string()),
                format!(
                    "la zone saisie contredit celle du département {} ({zone_departement}, arrêté \
                     du 26 octobre 2010, annexe I) ; la zone saisie est conservée",
                    loc.departement
                ),
            );
        }
    }

    // Catalogue.
    let cat = &projet.catalogue;
    let types_parois = r.ids_uniques("catalogue.parois", cat.parois.iter().map(|p| &p.id));
    r.ids_uniques("catalogue.baies", cat.baies.iter().map(|b| &b.id));
    let types_baies: BTreeMap<&str, &BaieType> =
        cat.baies.iter().map(|b| (b.id.as_str(), b)).collect();
    r.ids_uniques("catalogue.masques", cat.masques.iter().map(|m| &m.id));
    let masques: BTreeMap<&str, &Masque> = cat
        .masques
        .iter()
        .map(|m| (m.id.as_str(), &m.masque))
        .collect();
    for (i, p) in cat.parois.iter().enumerate() {
        valider_paroi_type(&mut r, &format!("catalogue.parois[{i}]"), p);
    }
    for (i, b) in cat.baies.iter().enumerate() {
        valider_baie_type(&mut r, &format!("catalogue.baies[{i}]"), b);
    }
    for (i, m) in cat.masques.iter().enumerate() {
        valider_masque(&mut r, &format!("catalogue.masques[{i}].masque"), &m.masque);
    }

    // Hiérarchie bâtiment → zone → groupe.
    if projet.batiments.is_empty() {
        r.erreur(
            "batiments",
            None,
            "le projet doit contenir au moins un bâtiment",
        );
    }
    r.ids_uniques("batiments", projet.batiments.iter().map(|b| &b.id));
    let refs = References {
        types_parois: &types_parois,
        types_baies: &types_baies,
        masques: &masques,
    };
    for (ib, bat) in projet.batiments.iter().enumerate() {
        let cb = format!("batiments[{ib}]");
        if bat.zones.is_empty() {
            r.erreur(
                format!("{cb}.zones"),
                None,
                "le bâtiment doit contenir au moins une zone",
            );
        }
        r.ids_uniques(&format!("{cb}.zones"), bat.zones.iter().map(|z| &z.id));
        for (iz, zone) in bat.zones.iter().enumerate() {
            valider_zone(&mut r, &format!("{cb}.zones[{iz}]"), zone, &refs);
        }
        valider_espaces_tampons(&mut r, &cb, bat, &refs);
        systemes::valider_systemes_batiment(&mut r, &cb, bat);
        generation::valider_generations(&mut r, &cb, bat);
        photovoltaique::valider_installations_pv(&mut r, &cb, bat, &refs);
        solaire::valider_chauffe_eau_solaires(&mut r, &cb, bat, &refs);
        collectif::valider_collectif(&mut r, &cb, bat);
        bruit::valider_bruit(&mut r, &cb, bat);
        valider_exigences(&mut r, &cb, bat, &projet.localisation);
        acv::valider_acv(&mut r, &cb, bat);
        usages_batiment::valider_usages_batiment(&mut r, &cb, bat);
    }
    // Contrôles de cohérence entre composants, une fois chaque élément valide : ils demandent
    // de résoudre l'enveloppe des groupes.
    if r.est_valide() {
        for (ib, bat) in projet.batiments.iter().enumerate() {
            for (iz, zone) in bat.zones.iter().enumerate() {
                if zone.usage.est_pris_en_charge() {
                    let c = format!("batiments[{ib}].zones[{iz}]");
                    valider_coherence_zone(&mut r, &c, zone, cat);
                }
            }
        }
    }
    r
}

struct References<'a> {
    types_parois: &'a BTreeSet<&'a str>,
    types_baies: &'a BTreeMap<&'a str, &'a BaieType>,
    masques: &'a BTreeMap<&'a str, &'a Masque>,
}
