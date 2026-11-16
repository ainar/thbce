// SPDX-License-Identifier: GPL-3.0-or-later
//! Calculs de la génération : fiche `C_GEN_Calculs génération` (Th-BCE 2020 §8.29).
//!
//! La fiche tient la comptabilité des générations : matrice horaire des consommations en
//! énergie finale par poste et par type d'énergie (tableau 252), répartition entre les groupes
//! au prorata de leurs demandes (éq. 1484 à 1486), cumuls mensuels et annuels par générateur,
//! par génération et par groupe (éq. 1494 à 1514), énergies fournies et efficacités annuelles
//! des générateurs (éq. 1521, 1525 et 1526), consommations d'auxiliaires (éq. 1527 et 1528),
//! consommations par type d'énergie (éq. 1529 à 1558) et nombres d'heures par taux de charge
//! (éq. 1522 à 1524).
//!
//! Les conversions en énergie primaire (tableau 251) relèvent du lot 7 : seules les
//! consommations en énergie finale sont cumulées ici. La production d'électricité des
//! cogénérations (éq. 1492, 1493, 1515 à 1519), les générateurs sur boucle d'eau (éq. 1490 et
//! 1491) et le bilan des boucles solaires (éq. 1520) sont hors de la première version ou relèvent
//! du lot 7b.

/// Poste de consommation d'une génération : lignes de la matrice des consommations
/// (§8.29.3.1, tableau 252).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Poste {
    /// 1 : chauffage.
    Chauffage,
    /// 2 : refroidissement.
    Refroidissement,
    /// 3 : eau chaude sanitaire.
    Ecs,
}

impl Poste {
    /// Les trois postes, dans l'ordre des lignes de la matrice.
    pub const TOUS: [Self; 3] = [Self::Chauffage, Self::Refroidissement, Self::Ecs];

    /// Indice de ligne, de 0 à 2.
    pub const fn indice(self) -> usize {
        match self {
            Self::Chauffage => 0,
            Self::Refroidissement => 1,
            Self::Ecs => 2,
        }
    }
}

/// Type d'énergie : colonnes de la matrice des consommations (§8.29.3.1, tableau 252 ;
/// « idengen », de 10 à 60).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Energie {
    /// 10 : gaz.
    Gaz,
    /// 20 : fioul.
    Fioul,
    /// 30 : charbon.
    Charbon,
    /// 40 : bois.
    Bois,
    /// 50 : électricité.
    Electricite,
    /// 60 : réseau de chaleur ou de froid.
    Reseau,
}

impl Energie {
    /// Les six types d'énergie, dans l'ordre des colonnes de la matrice.
    pub const TOUTES: [Self; 6] = [
        Self::Gaz,
        Self::Fioul,
        Self::Charbon,
        Self::Bois,
        Self::Electricite,
        Self::Reseau,
    ];

    /// Indice de colonne, de 0 à 5.
    pub const fn indice(self) -> usize {
        match self {
            Self::Gaz => 0,
            Self::Fioul => 1,
            Self::Charbon => 2,
            Self::Bois => 3,
            Self::Electricite => 4,
            Self::Reseau => 5,
        }
    }

    /// Code « idengen » du texte.
    pub const fn code(self) -> u8 {
        match self {
            Self::Gaz => 10,
            Self::Fioul => 20,
            Self::Charbon => 30,
            Self::Bois => 40,
            Self::Electricite => 50,
            Self::Reseau => 60,
        }
    }
}

/// « {Qcef(poste;énergie)} » : matrice 3 × 6 des consommations en énergie finale, en Wh
/// (tableaux 113 et 252).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MatriceCef(pub [[f64; 6]; 3]);

impl MatriceCef {
    /// Matrice nulle.
    pub const NULLE: Self = Self([[0.0; 6]; 3]);

    /// Valeur d'une case.
    pub fn valeur(&self, poste: Poste, energie: Energie) -> f64 {
        self.0[poste.indice()][energie.indice()]
    }

    /// Ajoute `q_wh` à la case `(poste ; énergie)`.
    pub fn ajouter(&mut self, poste: Poste, energie: Energie, q_wh: f64) {
        self.0[poste.indice()][energie.indice()] += q_wh;
    }

    /// Ajoute une autre matrice, case par case (éq. 1056).
    pub fn ajouter_matrice(&mut self, autre: &Self) {
        self.ajouter_matrice_ponderee(autre, [1.0; 3]);
    }

    /// Ajoute une autre matrice dont chaque ligne est multipliée par le ratio de son poste :
    /// répartition entre les groupes (éq. 1484 à 1486).
    pub fn ajouter_matrice_ponderee(&mut self, autre: &Self, ratios: [f64; 3]) {
        for (p, (ligne, src)) in self.0.iter_mut().zip(&autre.0).enumerate() {
            for (x, y) in ligne.iter_mut().zip(src) {
                *x += ratios[p] * y;
            }
        }
    }

    /// Total d'un poste, toutes énergies confondues (éq. 1494, 1501 et 1508).
    pub fn total_poste(&self, poste: Poste) -> f64 {
        self.0[poste.indice()].iter().sum()
    }

    /// Total d'un type d'énergie, tous postes confondus (éq. 1529 à 1534).
    pub fn total_energie(&self, energie: Energie) -> f64 {
        self.0.iter().map(|l| l[energie.indice()]).sum()
    }

    /// Total de la matrice.
    pub fn total(&self) -> f64 {
        self.0.iter().flatten().sum()
    }
}

/// Cumuls mensuels d'une matrice de consommations, par mois des scénarios, en Wh.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CumulsCef {
    /// Matrice cumulée de chaque mois.
    pub mensuel: [MatriceCef; 12],
}

impl CumulsCef {
    /// Ajoute la matrice d'un pas de temps du mois d'indice `mois` (de 0 à 11).
    pub fn ajouter(&mut self, mois: usize, m: &MatriceCef) {
        self.mensuel[mois].ajouter_matrice(m);
    }

    /// Ajoute la matrice d'un pas de temps, chaque ligne étant pondérée par le ratio de son
    /// poste.
    pub fn ajouter_pondere(&mut self, mois: usize, m: &MatriceCef, ratios: [f64; 3]) {
        self.mensuel[mois].ajouter_matrice_ponderee(m, ratios);
    }

    /// Matrice annuelle : somme des matrices mensuelles (éq. 1495, 1498 et 1500).
    pub fn annuel(&self) -> MatriceCef {
        let mut a = MatriceCef::NULLE;
        for m in &self.mensuel {
            a.ajouter_matrice(m);
        }
        a
    }
}

/// Nombre de classes de taux de charge des éq. (1522) à (1524) : charge nulle, puis dix
/// tranches de 10 %.
pub const NB_CLASSES_CHARGE: usize = 11;

/// Classe du taux de charge `tau` (éq. 1522 à 1524) : 0 pour une charge nulle, puis `k` pour
/// `(k − 1) × 10 % < τ ≤ k × 10 %`, la dernière classe recueillant les charges au-delà de 90 %.
pub fn classe_charge(tau: f64) -> usize {
    if tau <= 0.0 {
        0
    } else {
        // Les bornes hautes appartiennent à la tranche : 10 % va dans « 0 à 10 % ».
        (((tau * 10.0) - 1e-12).floor() as usize + 1).min(NB_CLASSES_CHARGE - 1)
    }
}

/// Cumuls annuels d'un générateur (§8.29.3.8).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CumulsGenerateur {
    /// Consommations en énergie finale, par mois (éq. 1494, 1501 et 1508).
    pub cef: CumulsCef,
    /// « Eef_fou » : énergie fournie par poste, en Wh (éq. 1525).
    pub q_fou_wh: [f64; 3],
    /// « Cef_aux » : consommation des auxiliaires, propres et amont, en Wh (éq. 1527) ; elle
    /// est comprise dans la matrice des consommations.
    pub w_aux_wh: f64,
    /// Pertes vers l'ambiance, en Wh.
    pub phi_vc_wh: f64,
    /// « φrejet » : énergie échangée avec la source amont, en Wh (négative en chauffage et en
    /// ECS, positive en refroidissement).
    pub phi_rejet_wh: f64,
    /// Nombre d'heures hors période de fonctionnement, par poste (« nbhcharge_HF », éq. 1522 à
    /// 1524).
    pub nbh_hors_fonctionnement: [u32; 3],
    /// Nombre d'heures par classe de taux de charge et par poste (éq. 1522 à 1524).
    pub nbh_charge: [[u32; NB_CLASSES_CHARGE]; 3],
}

impl CumulsGenerateur {
    /// « ηeff_an » : efficacité annuelle du générateur pour un poste (éq. 1526) — COP ou EER
    /// annuel d'un générateur thermodynamique, rendement des autres ; `None` sans consommation.
    pub fn eta_eff_an(&self, poste: Poste) -> Option<f64> {
        let c = self.cef.annuel().total_poste(poste);
        (c > 0.0).then(|| self.q_fou_wh[poste.indice()] / c)
    }

    /// « Qfou_3postes » : énergie totale fournie sur l'année, en Wh (éq. 1521).
    pub fn q_fou_3postes_wh(&self) -> f64 {
        self.q_fou_wh.iter().sum()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn matrice_et_repartition() {
        let mut m = MatriceCef::NULLE;
        m.ajouter(Poste::Chauffage, Energie::Electricite, 100.0);
        m.ajouter(Poste::Ecs, Energie::Electricite, 40.0);
        m.ajouter(Poste::Ecs, Energie::Gaz, 10.0);
        assert_eq!(m.total_poste(Poste::Ecs), 50.0);
        assert_eq!(m.total_energie(Energie::Electricite), 140.0);
        assert_eq!(m.total(), 150.0);
        // Éq. (1484) à (1486) : deux groupes, 30 % et 70 % du chauffage, 50 % chacun de l'ECS.
        let mut g1 = MatriceCef::NULLE;
        let mut g2 = MatriceCef::NULLE;
        g1.ajouter_matrice_ponderee(&m, [0.3, 0.0, 0.5]);
        g2.ajouter_matrice_ponderee(&m, [0.7, 0.0, 0.5]);
        assert!((g1.valeur(Poste::Chauffage, Energie::Electricite) - 30.0).abs() < 1e-12);
        assert!((g2.valeur(Poste::Ecs, Energie::Gaz) - 5.0).abs() < 1e-12);
        assert!((g1.total() + g2.total() - m.total()).abs() < 1e-12);
    }

    #[test]
    fn classes_de_charge() {
        assert_eq!(classe_charge(0.0), 0);
        assert_eq!(classe_charge(0.05), 1);
        assert_eq!(classe_charge(0.1), 1);
        assert_eq!(classe_charge(0.100_001), 2);
        assert_eq!(classe_charge(0.95), 10);
        assert_eq!(classe_charge(1.0), 10);
    }

    #[test]
    fn efficacite_annuelle() {
        let mut c = CumulsGenerateur::default();
        assert_eq!(c.eta_eff_an(Poste::Chauffage), None);
        let mut m = MatriceCef::NULLE;
        m.ajouter(Poste::Chauffage, Energie::Electricite, 1000.0);
        c.cef.ajouter(0, &m);
        c.q_fou_wh[0] = 3000.0;
        assert_eq!(c.eta_eff_an(Poste::Chauffage), Some(3.0));
        assert_eq!(c.q_fou_3postes_wh(), 3000.0);
    }
}
