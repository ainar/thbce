// SPDX-License-Identifier: GPL-3.0-or-later
//! Données météorologiques horaires fournies par l'appelant.
//!
//! Le moteur n'embarque aucune donnée météo : les fichiers réglementaires ne peuvent être ni
//! versionnés ni diffusés (décision Q1, `docs/questions/Q001.md`). Les lecteurs de fichiers sont
//! dans `thbce-io` ; tous produisent la structure unique définie ici.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thbce_data::ZoneClimatique;

use crate::conventions::NB_HEURES_AN;
use crate::erreur::Erreur;

/// Jeu de données météo d'une zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum JeuMeteo {
    /// « Th-BC » : besoins et consommations.
    #[serde(rename = "Th-BC")]
    ThBC,
    /// « Th-D » : confort d'été, avec séquence caniculaire.
    #[serde(rename = "Th-D")]
    ThD,
}

impl JeuMeteo {
    /// Nom du jeu dans les fichiers réglementaires.
    pub fn nom(self) -> &'static str {
        match self {
            Self::ThBC => "Th-BC",
            Self::ThD => "Th-D",
        }
    }
}

/// Provenance des données météo, reportée telle quelle dans les résultats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum OrigineMeteo {
    /// Fichiers météorologiques conventionnels de la RE2020.
    ReglementaireRe2020,
    /// Fichiers climatiques prospectifs ADEME / Météo-France (non réglementaires).
    AdemeProspectif {
        /// Scénario climatique, par exemple « +2,7 °C avec vague de chaleur ».
        scenario: String,
    },
    /// Toute autre source (non réglementaire).
    Autre {
        /// Description libre de la source.
        description: String,
    },
}

impl OrigineMeteo {
    /// Vrai pour les seuls fichiers conventionnels de la RE2020.
    pub fn est_reglementaire(&self) -> bool {
        matches!(self, Self::ReglementaireRe2020)
    }
}

/// Une année-type pour une zone et un jeu : 8 760 pas de temps, à l'altitude zéro.
///
/// Colonnes du fichier réglementaire ; correspondance avec la nomenclature du §3.1.2 selon la
/// décision Q1b. L'indice 0 est l'itération 0 du §4.1.3 (0 h à 1 h UTC).
#[derive(Debug, Clone, PartialEq)]
pub struct AnneeMeteo {
    /// Zone climatique.
    pub zone: ZoneClimatique,
    /// Jeu de données.
    pub jeu: JeuMeteo,
    /// Provenance.
    pub origine: OrigineMeteo,
    /// « Te0 » : température extérieure d'air sec au niveau de la mer, en °C (`te0`).
    pub te0_c: Vec<f64>,
    /// « we0 » : poids d'eau au niveau de la mer, en g/kg d'air sec (`we0`).
    pub we0_g_kg: Vec<f64>,
    /// « IDn » : rayonnement direct normal, en W/m² (`dirN_W_m2`).
    pub idn_w_m2: Vec<f64>,
    /// « Idi » : rayonnement diffus horizontal, en W/m² (`diff_W_m2`).
    pub idi_w_m2: Vec<f64>,
    /// « Teciel » : température du ciel, en °C (`Teciel_C`).
    pub teciel_c: Vec<f64>,
    /// « Vent » : vitesse du vent à 10 m de hauteur, en m/s (`Vent_m_s`).
    pub vent_m_s: Vec<f64>,
    /// « Teau0 » : température d'alimentation en eau froide à l'altitude zéro, en °C (`Teau_C`).
    pub teau0_c: Vec<f64>,
    /// « γ » : hauteur du soleil, en radians (`Gamma`, fourni en degrés).
    pub gamma_rad: Vec<f64>,
    /// « ψ » : azimut du soleil par rapport au sud, négatif le matin, en radians (`Psi`, fourni
    /// en degrés).
    pub psi_rad: Vec<f64>,
}

impl AnneeMeteo {
    /// Les neuf séries, avec leur nom, pour les contrôles.
    fn series(&self) -> [(&'static str, &[f64]); 9] {
        [
            ("te0", &self.te0_c),
            ("we0", &self.we0_g_kg),
            ("dirN", &self.idn_w_m2),
            ("diff", &self.idi_w_m2),
            ("Teciel", &self.teciel_c),
            ("Vent", &self.vent_m_s),
            ("Teau", &self.teau0_c),
            ("Gamma", &self.gamma_rad),
            ("Psi", &self.psi_rad),
        ]
    }

    /// Vérifie que chaque série compte 8 760 valeurs finies.
    pub fn verifier(&self) -> Result<(), Erreur> {
        for (nom, serie) in self.series() {
            if serie.len() != NB_HEURES_AN {
                return Err(Erreur::Meteo(format!(
                    "{} {} : la série {nom} compte {} valeurs, {NB_HEURES_AN} attendues",
                    self.zone,
                    self.jeu.nom(),
                    serie.len()
                )));
            }
            if let Some(h) = serie.iter().position(|v| !v.is_finite()) {
                return Err(Erreur::Meteo(format!(
                    "{} {} : la série {nom} contient une valeur non finie à h = {h}",
                    self.zone,
                    self.jeu.nom()
                )));
            }
        }
        Ok(())
    }

    /// Année synthétique à valeurs constantes, sans soleil, pour les tests de la chaîne de
    /// calcul. Elle n'a aucune signification physique et n'est jamais réglementaire.
    pub fn synthetique(zone: ZoneClimatique, jeu: JeuMeteo) -> Self {
        let constante = |v: f64| vec![v; NB_HEURES_AN];
        Self {
            zone,
            jeu,
            origine: OrigineMeteo::Autre {
                description: DESCRIPTION_SYNTHETIQUE.to_owned(),
            },
            te0_c: constante(10.0),
            we0_g_kg: constante(5.0),
            idn_w_m2: constante(0.0),
            idi_w_m2: constante(0.0),
            teciel_c: constante(0.0),
            vent_m_s: constante(1.0),
            teau0_c: constante(10.0),
            gamma_rad: constante(0.0),
            psi_rad: constante(0.0),
        }
    }
}

/// Description portée par les années synthétiques.
pub const DESCRIPTION_SYNTHETIQUE: &str =
    "jeu synthétique à valeurs constantes, réservé aux tests de la chaîne de calcul";

/// Les deux jeux météo nécessaires à un calcul complet, pour la zone du projet.
#[derive(Debug, Clone, PartialEq)]
pub struct MeteoProjet {
    /// Jeu « Th-BC ».
    pub th_bc: AnneeMeteo,
    /// Jeu « Th-D ».
    pub th_d: AnneeMeteo,
}

impl MeteoProjet {
    /// Vérifie les deux jeux et leur cohérence avec la zone climatique attendue.
    pub fn verifier(&self, zone: ZoneClimatique) -> Result<(), Erreur> {
        for (annee, jeu) in [(&self.th_bc, JeuMeteo::ThBC), (&self.th_d, JeuMeteo::ThD)] {
            if annee.jeu != jeu {
                return Err(Erreur::Meteo(format!(
                    "jeu {} fourni à la place du jeu {}",
                    annee.jeu.nom(),
                    jeu.nom()
                )));
            }
            if annee.zone != zone {
                return Err(Erreur::Meteo(format!(
                    "données météo de la zone {} fournies pour un projet en zone {zone}",
                    annee.zone
                )));
            }
            annee.verifier()?;
        }
        Ok(())
    }

    /// Le calcul n'est réglementaire que si les deux jeux le sont.
    pub fn est_reglementaire(&self) -> bool {
        self.th_bc.origine.est_reglementaire() && self.th_d.origine.est_reglementaire()
    }

    /// Jeux synthétiques pour les tests (voir [`AnneeMeteo::synthetique`]).
    pub fn synthetique(zone: ZoneClimatique) -> Self {
        Self {
            th_bc: AnneeMeteo::synthetique(zone, JeuMeteo::ThBC),
            th_d: AnneeMeteo::synthetique(zone, JeuMeteo::ThD),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_jeu_synthetique_est_coherent_et_non_reglementaire() {
        let m = MeteoProjet::synthetique(ZoneClimatique::H2b);
        assert!(m.verifier(ZoneClimatique::H2b).is_ok());
        assert!(!m.est_reglementaire());
    }

    #[test]
    fn refuse_une_zone_ou_une_longueur_incoherente() {
        let mut m = MeteoProjet::synthetique(ZoneClimatique::H2b);
        assert!(matches!(
            m.verifier(ZoneClimatique::H3),
            Err(Erreur::Meteo(_))
        ));
        m.th_d.vent_m_s.pop();
        assert!(matches!(
            m.verifier(ZoneClimatique::H2b),
            Err(Erreur::Meteo(_))
        ));
        let mut m = MeteoProjet::synthetique(ZoneClimatique::H1a);
        m.th_bc.te0_c[12] = f64::NAN;
        assert!(matches!(
            m.verifier(ZoneClimatique::H1a),
            Err(Erreur::Meteo(_))
        ));
    }
}
