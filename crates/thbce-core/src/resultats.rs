// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats du calcul.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::climat::OrigineMeteo;
use crate::conventions::arrondir_restitution;
use crate::entree::Id;

/// Résultats d'un calcul sur un projet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Resultats {
    /// Version du moteur.
    pub version_moteur: String,
    /// Version de la méthode implémentée.
    pub version_methode: String,
    /// Faux dès qu'une hypothèse non réglementaire intervient : données météo autres que les
    /// fichiers conventionnels de la RE2020, ou règle de substitution documentée comme telle.
    pub reglementaire: bool,
    /// Provenance du jeu météo « Th-BC ».
    pub origine_meteo_th_bc: OrigineMeteo,
    /// Provenance du jeu météo « Th-D ».
    pub origine_meteo_th_d: OrigineMeteo,
    /// Vrai si les valeurs sont arrondies à une décimale (Th-BCE 2020 §1.3.1).
    pub valeurs_arrondies: bool,
    /// Avertissements émis par la validation et par le calcul.
    pub avertissements: Vec<String>,
    /// Résultats par bâtiment.
    pub batiments: Vec<ResultatsBatiment>,
}

/// Résultats d'un bâtiment. Un indicateur vaut `null` tant que la partie correspondante de la
/// méthode n'est pas implémentée (voir `docs/couverture.md`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsBatiment {
    /// Identifiant du bâtiment.
    pub id: Id,
    /// « SREF_bat » : surface de référence, en m² (Th-BCE 2020 §13.1.3, éq. 2378).
    pub sref_m2: f64,
    /// « Bbio » : besoin bioclimatique, en points (§13.1.3, éq. 2385 et 2386).
    pub bbio_pts: Option<f64>,
    /// « Cep » : consommation d'énergie primaire, en kWhep/(m².an) (§13.2).
    pub cep_kwhep_m2: Option<f64>,
    /// « Cep,nr » : consommation d'énergie primaire non renouvelable, en kWhep/(m².an).
    pub cep_nr_kwhep_m2: Option<f64>,
    /// « DH » : degrés-heures d'inconfort, en °C.h (§13.5.3, éq. 2551).
    pub dh_c_h: Option<f64>,
    /// « Ic énergie », en kg éq. CO2/m² (annexe II §4.3.2, éq. 74).
    pub ic_energie_kgco2_m2: Option<f64>,
    /// Résultats par zone.
    pub zones: Vec<ResultatsZone>,
}

/// Résultats d'une zone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsZone {
    /// Identifiant de la zone.
    pub id: Id,
    /// « SREF_zn », en m² (§13.1.3, éq. 2377).
    pub sref_m2: f64,
    /// Résultats par groupe.
    pub groupes: Vec<ResultatsGroupe>,
}

/// Résultats d'un groupe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsGroupe {
    /// Identifiant du groupe.
    pub id: Id,
    /// « SREF_gr », en m² (§13.1.3, éq. 2375 et 2376).
    pub sref_m2: f64,
}

impl Resultats {
    /// Applique l'arrondi de restitution à toutes les grandeurs (§1.3.1, décision Q8).
    pub fn arrondir(&mut self) {
        let a = arrondir_restitution;
        for b in &mut self.batiments {
            b.sref_m2 = a(b.sref_m2);
            b.bbio_pts = b.bbio_pts.map(a);
            b.cep_kwhep_m2 = b.cep_kwhep_m2.map(a);
            b.cep_nr_kwhep_m2 = b.cep_nr_kwhep_m2.map(a);
            b.dh_c_h = b.dh_c_h.map(a);
            b.ic_energie_kgco2_m2 = b.ic_energie_kgco2_m2.map(a);
            for z in &mut b.zones {
                z.sref_m2 = a(z.sref_m2);
                for g in &mut z.groupes {
                    g.sref_m2 = a(g.sref_m2);
                }
            }
        }
        self.valeurs_arrondies = true;
    }
}
