// SPDX-License-Identifier: GPL-3.0-or-later
//! Séries horaires principales restituées sur demande (lot 1-sor, Q118) : température
//! extérieure, besoins et températures de chaque groupe, consommations par poste et par énergie
//! du bâtiment. Aucune de ces grandeurs n'est un indicateur du texte : ce sont les grandeurs
//! intermédiaires du calcul Th-C, relevées pas de temps par pas de temps.
//!
//! Le relevé n'a lieu que si l'appelant le demande ([`crate::calculer_avec_series`]) : la
//! restitution par défaut est inchangée.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::bati::assemblage_batiment::BatimentHoraire;
use crate::chauffage_refroidissement::calculs_generation::{Energie, Poste};
use crate::conventions::{arrondir_restitution, NB_HEURES_AN};
use crate::indicateurs::bilans_energies::{poste, ReleveElectrique, NB_POSTES};

/// Une série horaire : 8 760 valeurs, dans l'ordre des pas de temps de l'année.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ColonneHoraire {
    /// Nom de la série (voir `docs/modele-donnees.md`).
    pub nom: String,
    /// Unité : `C` (degrés Celsius) ou `Wh` (par pas de temps d'une heure).
    pub unite: String,
    /// Valeurs, une par heure de l'année (8 760).
    pub valeurs: Vec<f64>,
}

/// Séries horaires d'un bâtiment, calculées sur le jeu météo « Th-BC » (calcul Th-C).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SeriesHorairesBatiment {
    /// Séries, dans un ordre fixe : `te`, séries par groupe (`<zone>.<groupe>.…`), séries
    /// d'électricité par poste (`elec.…`) puis consommations de finale par poste et par énergie
    /// (`cef.<poste>.<énergie>`, colonnes nulles omises).
    pub colonnes: Vec<ColonneHoraire>,
}

impl SeriesHorairesBatiment {
    /// Arrondi de restitution (§1.3.1) de toutes les valeurs.
    pub fn arrondir(&mut self) {
        for c in &mut self.colonnes {
            for v in &mut c.valeurs {
                *v = arrondir_restitution(*v);
            }
        }
    }
}

const POSTES_ELEC: [(&str, usize); NB_POSTES] = [
    ("ch", poste::CH),
    ("fr", poste::FR),
    ("ecs", poste::ECS),
    ("ecl", poste::ECL),
    ("aux_vent", poste::AUX_VENT),
    ("aux_dist", poste::AUX_DIST),
    ("depl", poste::DEPL),
    ("mob", poste::MOB),
];

const NOMS_POSTES: [&str; 3] = ["ch", "fr", "ecs"];
const NOMS_ENERGIES: [&str; 6] = ["gaz", "fioul", "charbon", "bois", "electricite", "reseau"];

/// Relevé pendant le calcul Th-C.
pub struct Collecteur {
    /// Identifiant `<zone>.<groupe>` de chaque groupe, puis `q_sys_ch`, `q_sys_fr`,
    /// `theta_air`, `theta_op`.
    groupes: Vec<(String, [Vec<f64>; 4])>,
    /// Consommation du pas de temps par poste du modèle thermique et par énergie, en Wh.
    cef: Vec<[[f64; 6]; 3]>,
    precedent: [[f64; 6]; 3],
}

fn cumul_cef(b: &BatimentHoraire) -> [[f64; 6]; 3] {
    let mut c = [[0.0; 6]; 3];
    for m in &b.systemes.cumuls_cef_groupes {
        let a = m.annuel();
        for p in Poste::TOUS {
            for e in Energie::TOUTES {
                c[p.indice()][e.indice()] += a.valeur(p, e);
            }
        }
    }
    c
}

impl Collecteur {
    /// Prépare le relevé des groupes de `b`.
    pub fn nouveau(b: &BatimentHoraire) -> Self {
        let groupes = b
            .zones
            .iter()
            .flat_map(|z| {
                z.groupes.iter().map(move |g| {
                    (
                        format!("{}.{}", z.id, g.id),
                        std::array::from_fn(|_| Vec::with_capacity(NB_HEURES_AN)),
                    )
                })
            })
            .collect();
        Self {
            groupes,
            cef: vec![[[0.0; 6]; 3]; NB_HEURES_AN],
            precedent: [[0.0; 6]; 3],
        }
    }

    /// Relève le pas de temps `h` qui vient d'être calculé.
    pub fn relever(&mut self, h: usize, b: &BatimentHoraire) {
        let sorties = b.zones.iter().flat_map(|z| z.sorties.iter());
        for ((_, v), s) in self.groupes.iter_mut().zip(sorties) {
            let valeurs = s.as_ref().map_or([0.0; 4], |s| {
                [
                    s.q_sys_ch_wh,
                    s.q_sys_fr_wh,
                    s.moy.theta_i_c,
                    s.moy.theta_op_c,
                ]
            });
            for (colonne, x) in v.iter_mut().zip(valeurs) {
                colonne.push(x);
            }
        }
        let c = cumul_cef(b);
        if let Some(x) = self.cef.get_mut(h) {
            for p in 0..3 {
                for e in 0..6 {
                    x[p][e] = c[p][e] - self.precedent[p][e];
                }
            }
        }
        self.precedent = c;
    }

    /// Assemble les séries : `te_c` est la température extérieure du jeu « Th-BC », `releve`
    /// le relevé électrique complet (postes hors modèle thermique compris).
    pub fn finir(self, te_c: &[f64], releve: &ReleveElectrique) -> SeriesHorairesBatiment {
        let col = |nom: String, unite: &str, valeurs: Vec<f64>| ColonneHoraire {
            nom,
            unite: unite.into(),
            valeurs,
        };
        let mut colonnes = vec![col(
            "te".into(),
            "C",
            te_c.iter().take(NB_HEURES_AN).copied().collect(),
        )];
        for (id, [ch, fr, air, op]) in self.groupes {
            colonnes.push(col(format!("{id}.q_sys_ch"), "Wh", ch));
            colonnes.push(col(format!("{id}.q_sys_fr"), "Wh", fr));
            colonnes.push(col(format!("{id}.theta_air"), "C", air));
            colonnes.push(col(format!("{id}.theta_op"), "C", op));
        }
        let mut total = vec![0.0; NB_HEURES_AN];
        for (nom, p) in POSTES_ELEC {
            let v: Vec<f64> = releve.horaire.iter().map(|x| x[p]).collect();
            for (t, x) in total.iter_mut().zip(&v) {
                *t += x;
            }
            colonnes.push(col(format!("elec.{nom}"), "Wh", v));
        }
        colonnes.push(col("elec.total".into(), "Wh", total));
        for (p, np) in NOMS_POSTES.iter().enumerate() {
            for (e, ne) in NOMS_ENERGIES.iter().enumerate() {
                let v: Vec<f64> = self.cef.iter().map(|x| x[p][e]).collect();
                if v.iter().any(|x| *x != 0.0) {
                    colonnes.push(col(format!("cef.{np}.{ne}"), "Wh", v));
                }
            }
        }
        SeriesHorairesBatiment { colonnes }
    }
}
