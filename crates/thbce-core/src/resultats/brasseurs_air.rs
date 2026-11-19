// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats des brasseurs d'air d'un groupe (Th-BCE 2020 §8.32).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Brasseurs d'air d'un groupe : consommation du calcul Th-C et effet du calcul Th-D.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsBrasseurs {
    /// « Wabs_BA » : consommation électrique annuelle des brasseurs du groupe (calcul des
    /// consommations, saison de refroidissement propre), en kWh d'énergie finale (éq. 1665).
    /// Comptée dans les auxiliaires de ventilation (§13.4).
    pub w_abs_kwh: f64,
    /// Nombre d'heures de fonctionnement d'au moins un brasseur dans le calcul des consommations.
    pub nbh_fonctionnement: u32,
    /// Nombre d'heures de fonctionnement d'au moins un brasseur dans le calcul du confort d'été
    /// (Th-D, période de confort adaptatif).
    pub nbh_fonctionnement_th_d: u32,
    /// « Δθop^BA » maximal du calcul Th-D : plus fort déplacement de la limite d'inconfort chaud
    /// dû aux brasseurs (éq. 2547), en K.
    pub delta_theta_op_max_k: f64,
}

impl ResultatsBrasseurs {
    /// Arrondi de restitution.
    pub fn arrondir(&mut self) {
        let a = crate::conventions::arrondir_restitution;
        self.w_abs_kwh = a(self.w_abs_kwh);
        self.delta_theta_op_max_k = a(self.delta_theta_op_max_k);
    }
}
