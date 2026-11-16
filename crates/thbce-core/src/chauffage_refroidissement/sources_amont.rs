// SPDX-License-Identifier: GPL-3.0-or-later
//! Sources amont des générateurs thermodynamiques : fiche `C_Gen_Sources amont des générateurs
//! thermodynamiques` (Th-BCE 2020 §8.26), pour les sources sur air.
//!
//! La fiche donne la température amont des générateurs (air extérieur, éq. 1405 ; air ambiant
//! d'un volume non chauffé, éq. 1406 ; air extrait, éq. 1407), la consommation de leurs
//! auxiliaires amont (éq. 1441 à 1443 et 1452) et la puissance maximale échangeable avec une
//! source sur air extrait (éq. 1453 à 1457, appliquée par la fiche 8.23, éq. 1291 et 1292).
//!
//! Dans cette version, chaque générateur a sa propre source amont : les ratios de répartition
//! entre générateurs d'une même source (éq. 1402 à 1404 et 1452) valent 1. Les sources sur eau
//! (captage, nappe, tours de refroidissement, boucles d'eau ; éq. 1408 à 1440 et 1444 à 1451)
//! sont hors de la première version.

/// « Cpa » : capacité thermique massique de l'air, en J/(kg.K).
pub const C_PA_J_KG_K: f64 = crate::ventilation::mecanique_sf::C_PA_J_KG_K;

/// « idamont-air-type » : source amont sur air d'un générateur thermodynamique (§8.26.3.3.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SourceAmont {
    /// 1 : air extérieur (éq. 1405).
    AirExterieur,
    /// 2 : air ambiant d'un volume non chauffé, espace tampon d'indice donné (éq. 1406).
    AirAmbiant {
        /// Indice de l'espace tampon dans le bâtiment.
        espace_tampon: usize,
    },
    /// 3 : air extrait par le système de ventilation d'une zone (éq. 1407).
    AirExtrait {
        /// Indice de la zone dont le système de ventilation irrigue la source.
        zone: usize,
    },
}

/// Source amont d'un générateur et ses auxiliaires.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceAmontGenerateur {
    /// Nature de la source.
    pub source: SourceAmont,
    /// « Pvent_gainé » : puissance des ventilateurs nécessaire pour vaincre les pertes de charge
    /// des conduits d'une machine gainée, en W ; nulle pour une machine non gainée (éq. 1442).
    pub p_vent_gaine_w: f64,
}

impl SourceAmontGenerateur {
    /// « Waux,am » : consommation des auxiliaires amont au pas de temps, en Wh, pour le taux de
    /// charge du générateur (éq. 1441 à 1443).
    pub fn w_aux_am_wh(&self, tau_charge: f64) -> f64 {
        self.p_vent_gaine_w * tau_charge
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn auxiliaires_amont() {
        // Machine non gainée : auxiliaires compris dans la performance (éq. 1442).
        let s = SourceAmontGenerateur {
            source: SourceAmont::AirExterieur,
            p_vent_gaine_w: 0.0,
        };
        assert_eq!(s.w_aux_am_wh(0.8), 0.0);
        // Machine gainée : 30 W à pleine charge, au prorata de la charge (éq. 1443).
        let s = SourceAmontGenerateur {
            source: SourceAmont::AirExterieur,
            p_vent_gaine_w: 30.0,
        };
        assert_eq!(s.w_aux_am_wh(0.5), 15.0);
    }
}
