// SPDX-License-Identifier: GPL-3.0-or-later
//! Consommations d'énergie finale par poste et par énergie.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::est_nul;
use crate::conventions::arrondir_restitution;

/// Consommations en énergie finale d'un poste par type d'énergie, en kWh (colonnes de la
/// matrice du tableau 252, §8.29.3.1). Les énergies sans consommation ne sont pas écrites.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct ConsommationsParEnergie {
    /// 10 : gaz, en kWh PCI.
    #[serde(default, skip_serializing_if = "est_nul")]
    pub gaz_kwh: f64,
    /// 20 : fioul.
    #[serde(default, skip_serializing_if = "est_nul")]
    pub fioul_kwh: f64,
    /// 30 : charbon.
    #[serde(default, skip_serializing_if = "est_nul")]
    pub charbon_kwh: f64,
    /// 40 : bois.
    #[serde(default, skip_serializing_if = "est_nul")]
    pub bois_kwh: f64,
    /// 50 : électricité.
    #[serde(default, skip_serializing_if = "est_nul")]
    pub electricite_kwh: f64,
    /// 60 : réseau de chaleur ou de froid.
    #[serde(default, skip_serializing_if = "est_nul")]
    pub reseau_kwh: f64,
}

impl ConsommationsParEnergie {
    /// Total du poste, toutes énergies confondues, en kWh.
    pub fn total_kwh(&self) -> f64 {
        self.gaz_kwh
            + self.fioul_kwh
            + self.charbon_kwh
            + self.bois_kwh
            + self.electricite_kwh
            + self.reseau_kwh
    }

    fn champs(&mut self) -> [&mut f64; 6] {
        [
            &mut self.gaz_kwh,
            &mut self.fioul_kwh,
            &mut self.charbon_kwh,
            &mut self.bois_kwh,
            &mut self.electricite_kwh,
            &mut self.reseau_kwh,
        ]
    }
}

/// Consommations en énergie finale des générations par poste (lignes de la matrice du tableau
/// 252). Les consommations des auxiliaires de génération — auxiliaires propres aux générateurs
/// et auxiliaires de leurs sources amont — sont comprises dans le poste qu'ils servent (éq.
/// 1054 et 1055), comme le réchauffeur d'une boucle d'ECS dans le poste ECS (éq. 1057).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct ConsommationsPostes {
    /// 1 : chauffage.
    pub chauffage: ConsommationsParEnergie,
    /// 2 : refroidissement.
    pub refroidissement: ConsommationsParEnergie,
    /// 3 : eau chaude sanitaire.
    pub ecs: ConsommationsParEnergie,
}

impl ConsommationsPostes {
    /// Consommations d'une matrice en Wh.
    pub fn depuis_matrice(
        m: &crate::chauffage_refroidissement::calculs_generation::MatriceCef,
    ) -> Self {
        let ligne = |l: &[f64; 6]| ConsommationsParEnergie {
            gaz_kwh: l[0] / 1000.0,
            fioul_kwh: l[1] / 1000.0,
            charbon_kwh: l[2] / 1000.0,
            bois_kwh: l[3] / 1000.0,
            electricite_kwh: l[4] / 1000.0,
            reseau_kwh: l[5] / 1000.0,
        };
        Self {
            chauffage: ligne(&m.0[0]),
            refroidissement: ligne(&m.0[1]),
            ecs: ligne(&m.0[2]),
        }
    }

    /// Total des trois postes, en kWh.
    pub fn total_kwh(&self) -> f64 {
        self.chauffage.total_kwh() + self.refroidissement.total_kwh() + self.ecs.total_kwh()
    }

    /// Total de l'électricité des trois postes, en kWh.
    pub fn electricite_kwh(&self) -> f64 {
        self.chauffage.electricite_kwh
            + self.refroidissement.electricite_kwh
            + self.ecs.electricite_kwh
    }

    fn ajouter(&mut self, autre: &Self) {
        let mut autre = *autre;
        for (a, b) in [
            (&mut self.chauffage, &mut autre.chauffage),
            (&mut self.refroidissement, &mut autre.refroidissement),
            (&mut self.ecs, &mut autre.ecs),
        ] {
            for (x, y) in a.champs().into_iter().zip(b.champs()) {
                *x += *y;
            }
        }
    }

    pub(super) fn arrondir(&mut self) {
        for p in [
            &mut self.chauffage,
            &mut self.refroidissement,
            &mut self.ecs,
        ] {
            for x in p.champs() {
                *x = arrondir_restitution(*x);
            }
        }
    }
}

/// Consommations en énergie finale, annuelles et mensuelles (mois des scénarios), des
/// générations d'un bâtiment attribuées à un groupe, à une zone ou au bâtiment, ou d'une
/// génération (§8.29, éq. 1497 à 1514).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Consommations {
    /// Consommations annuelles, en kWh.
    pub annuel: ConsommationsPostes,
    /// Consommations des douze mois, en kWh.
    pub mensuel: Vec<ConsommationsPostes>,
}

impl Consommations {
    /// Consommations à partir de cumuls mensuels.
    pub fn depuis_cumuls(
        c: &crate::chauffage_refroidissement::calculs_generation::CumulsCef,
    ) -> Self {
        Self {
            annuel: ConsommationsPostes::depuis_matrice(&c.annuel()),
            mensuel: c
                .mensuel
                .iter()
                .map(ConsommationsPostes::depuis_matrice)
                .collect(),
        }
    }

    /// Ajoute d'autres consommations, mois par mois.
    pub fn ajouter(&mut self, autre: &Self) {
        self.annuel.ajouter(&autre.annuel);
        if self.mensuel.len() < autre.mensuel.len() {
            self.mensuel
                .resize(autre.mensuel.len(), ConsommationsPostes::default());
        }
        for (a, b) in self.mensuel.iter_mut().zip(&autre.mensuel) {
            a.ajouter(b);
        }
    }

    pub(super) fn arrondir(&mut self) {
        self.annuel.arrondir();
        for m in &mut self.mensuel {
            m.arrondir();
        }
    }
}
