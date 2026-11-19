// SPDX-License-Identifier: GPL-3.0-or-later
//! Sorties détaillées par zone et par groupe de la fiche 13.2 (Th-BCE 2020 §13.2.3.3 et
//! §13.2.3.4, éq. 2416 à 2461), facultatives : `batiments[].sorties_detaillees`, demandées par
//! l'appelant (`calculer_avec`, option `--sorties-detaillees`, option `sorties_detaillees` du
//! module WebAssembly). Lectures : `docs/questions/Q116.md`.
//!
//! Les quantités sont en kWh (additives : la somme des zones redonne le bâtiment) ; les valeurs
//! « par m² » du texte sont rapportées à la surface de référence du niveau (zone ou groupe).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::conventions::arrondir_restitution;
use crate::entree::Id;

/// Quantité mensuelle (douze mois des scénarios) et annuelle, en kWh.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct SerieMensuelle {
    /// Valeur de chacun des douze mois, en kWh.
    pub mensuel_kwh: Vec<f64>,
    /// Valeur annuelle, en kWh : somme des mois, plus le forfait de refroidissement (§13.3,
    /// annuel seulement) pour l'électricité du refroidissement.
    pub annuel_kwh: f64,
    /// Valeur annuelle par m² de surface de référence du niveau, en kWh/m².
    pub annuel_kwh_m2: f64,
}

impl SerieMensuelle {
    /// Série dont l'annuel est la somme des mois, plus `supplement_annuel_kwh`.
    pub(crate) fn nouvelle(
        mensuel_kwh: [f64; 12],
        supplement_annuel_kwh: f64,
        sref_m2: f64,
    ) -> Self {
        let annuel_kwh = mensuel_kwh.iter().sum::<f64>() + supplement_annuel_kwh;
        Self {
            mensuel_kwh: mensuel_kwh.to_vec(),
            annuel_kwh,
            annuel_kwh_m2: if sref_m2 > 0.0 {
                annuel_kwh / sref_m2
            } else {
                0.0
            },
        }
    }

    fn arrondir(&mut self) {
        for x in &mut self.mensuel_kwh {
            *x = arrondir_restitution(*x);
        }
        self.annuel_kwh = arrondir_restitution(self.annuel_kwh);
        self.annuel_kwh_m2 = arrondir_restitution(self.annuel_kwh_m2);
    }
}

/// Énergie finale d'un poste pour une énergie : importée (zone) ou consommée (groupe, ou
/// électricité consommée de la zone).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LigneSortie {
    /// Poste : `chauffage`, `refroidissement`, `ecs`, `eclairage`, `auxiliaires_ventilation`,
    /// `auxiliaires_distribution`, `deplacements` ou `usages_mobiliers` (zone seulement).
    pub poste: String,
    /// Énergie : `electricite`, `gaz`, `fioul`, `charbon`, `bois` ou `reseau`.
    pub energie: String,
    /// Quantités mensuelles et annuelle.
    #[serde(flatten)]
    pub quantite: SerieMensuelle,
}

/// Besoins en énergie d'un niveau (éq. 2418 à 2420 et 2438 à 2440), en kWh.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct BesoinsSortie {
    /// `O_B_Ch` : chauffage.
    pub chauffage: SerieMensuelle,
    /// `O_B_Fr` : refroidissement (valeur absolue).
    pub refroidissement: SerieMensuelle,
    /// `O_B_Ecs` : ECS (`Qw,brut`).
    pub ecs: SerieMensuelle,
}

impl BesoinsSortie {
    fn arrondir(&mut self) {
        self.chauffage.arrondir();
        self.refroidissement.arrondir();
        self.ecs.arrondir();
    }
}

/// Sorties d'un groupe (§13.2.3.3) : quantités consommées, sans production ni importation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SortiesGroupe {
    /// Identifiant du groupe.
    pub id: Id,
    /// `SREF^gr`, en m² (éq. 2412 et 2413).
    pub sref_m2: f64,
    /// Consommations par poste et par énergie (éq. 2416, 2417, 2421, 2422) ; ni déplacement des
    /// occupants ni usages mobiliers au niveau du groupe. Les auxiliaires des réseaux
    /// intergroupes et des centrales de traitement d'air, qui ne dépendent pas d'un seul groupe,
    /// sont répartis au prorata des surfaces de référence (Q116).
    pub consommations: Vec<LigneSortie>,
    /// `O_Cef_annuel` (éq. 2425), en kWh/m².
    pub cef_kwh_m2: f64,
    /// `O_Cep_annuel` (éq. 2426), en kWhep/m².
    pub cep_kwhep_m2: f64,
    /// Consommation d'énergie primaire non renouvelable, en kWhep/m² (même forme que l'éq. 2458).
    pub cep_nr_kwhep_m2: f64,
    /// Besoins `O_B_*` : énergie requise aux bornes des émetteurs, éq. (2498) (lecture retenue,
    /// Q116) ; `Qw,brut` pour l'ECS.
    pub besoins: BesoinsSortie,
    /// Variante de la lecture des besoins : `Qsys` thermique du groupe (éq. 2379 et 2380, le
    /// besoin du Bbio), sans le rendement ni les pertes au dos des émetteurs. L'ECS est inchangée.
    pub besoins_qsys_thermique: BesoinsSortie,
}

/// Production photovoltaïque attribuée à une zone, éq. (2433), (2435), (2437), (2447), (2449),
/// (2451) et (2453).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProductionPvZone {
    /// `O_Eef_prod_PV` : production attribuée à la zone.
    pub produite: SerieMensuelle,
    /// `O_Eef_prod_PV_AC` : part autoconsommée (éq. 2532 à 2534 : taux d'autoconsommation du
    /// bâtiment à chaque pas, appliqué à la production attribuée).
    pub autoconsommee: SerieMensuelle,
    /// `O_Eef_Elec_Exportee` (éq. 2437).
    pub exportee: SerieMensuelle,
    /// `O_TAC_elec_PV_annuel` (éq. 2453), en % ; nul sans production.
    pub tac_pct: f64,
}

impl ProductionPvZone {
    fn arrondir(&mut self) {
        self.produite.arrondir();
        self.autoconsommee.arrondir();
        self.exportee.arrondir();
        self.tac_pct = arrondir_restitution(self.tac_pct);
    }
}

/// Sorties d'une zone (§13.2.3.4) : quantités importées, et électricité consommée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SortiesZone {
    /// Identifiant de la zone.
    pub id: Id,
    /// `SREF^zn`, en m² (éq. 2414).
    pub sref_m2: f64,
    /// Énergie finale importée par poste et par énergie (éq. 2430, 2431, 2441, 2442).
    pub importees: Vec<LigneSortie>,
    /// Électricité consommée (part importée et part autoproduite) par poste (éq. 2432, 2443).
    pub electricite_consommee: Vec<LigneSortie>,
    /// Production photovoltaïque attribuée à la zone, au prorata des consommations horaires
    /// d'électricité de la zone (lecture retenue, Q116) ; absente sans production.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_pv: Option<ProductionPvZone>,
    /// Variante : production attribuée au prorata des surfaces de référence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_pv_variante_sref: Option<ProductionPvZone>,
    /// `O_Cef_annuel` (éq. 2456), en kWh/m² ; sans usages mobiliers (éq. 2446).
    pub cef_kwh_m2: f64,
    /// `O_Cep_annuel` (éq. 2457), en kWhep/m².
    pub cep_kwhep_m2: f64,
    /// `O_Cepnr_annuel` (éq. 2458), en kWhep/m².
    pub cep_nr_kwhep_m2: f64,
    /// Besoins `O_B_*` de la zone (éq. 2438 à 2440 : somme des groupes pondérée par `SREF^gr`).
    pub besoins: BesoinsSortie,
    /// Variante de la lecture des besoins (`Qsys` thermique).
    pub besoins_qsys_thermique: BesoinsSortie,
    /// Sorties de chaque groupe de la zone.
    pub groupes: Vec<SortiesGroupe>,
}

/// Sorties détaillées d'un bâtiment, dans l'ordre des zones.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SortiesDetaillees {
    /// Zones du bâtiment.
    pub zones: Vec<SortiesZone>,
}

impl SortiesDetaillees {
    pub(crate) fn arrondir(&mut self) {
        let a = arrondir_restitution;
        for z in &mut self.zones {
            z.sref_m2 = a(z.sref_m2);
            for l in z.importees.iter_mut().chain(&mut z.electricite_consommee) {
                l.quantite.arrondir();
            }
            for p in [&mut z.production_pv, &mut z.production_pv_variante_sref]
                .into_iter()
                .flatten()
            {
                p.arrondir();
            }
            z.cef_kwh_m2 = a(z.cef_kwh_m2);
            z.cep_kwhep_m2 = a(z.cep_kwhep_m2);
            z.cep_nr_kwhep_m2 = a(z.cep_nr_kwhep_m2);
            z.besoins.arrondir();
            z.besoins_qsys_thermique.arrondir();
            for g in &mut z.groupes {
                g.sref_m2 = a(g.sref_m2);
                for l in &mut g.consommations {
                    l.quantite.arrondir();
                }
                g.cef_kwh_m2 = a(g.cef_kwh_m2);
                g.cep_kwhep_m2 = a(g.cep_kwhep_m2);
                g.cep_nr_kwhep_m2 = a(g.cep_nr_kwhep_m2);
                g.besoins.arrondir();
                g.besoins_qsys_thermique.arrondir();
            }
        }
    }
}
