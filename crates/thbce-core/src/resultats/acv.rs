// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats de l'analyse du cycle de vie d'un bâtiment (annexe II de l'arrêté du 4 août 2021,
//! chapitre 4 et §5.3) : contributions par phase du cycle de vie, par lot et par sous-lot,
//! indicateurs Icconstruction, Icbâtiment, Icprojet, StockC, Icded.
//!
//! Toutes les grandeurs d'impact sont rapportées à la surface de référence du bâtiment (kg éq.
//! CO2/m²) ; « i_… » : méthode statique, « ic_… » : méthode dynamique.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::conventions::arrondir_restitution;
use crate::resultats::ValeurParDefaut;

/// Impact sur le changement climatique par phase du cycle de vie du bâtiment, en kg éq.
/// CO2/m² (annexe II §1.4.2, tableau 4).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct PhasesAcv {
    /// Production (A1 à A3).
    pub production: f64,
    /// Édification (A4, A5).
    pub edification: f64,
    /// Exploitation (B1 à B7).
    pub exploitation: f64,
    /// Fin de vie (C1 à C4).
    pub fin_de_vie: f64,
    /// Bénéfices et charges au-delà du cycle de vie (module D).
    pub module_d: f64,
    /// Somme des quatre phases et du module D (éq. 30 et 34).
    pub total: f64,
}

impl PhasesAcv {
    /// Phases à partir des cinq termes ; `total` est leur somme (éq. 30).
    pub fn depuis(p: [f64; 5]) -> Self {
        Self {
            production: p[0],
            edification: p[1],
            exploitation: p[2],
            fin_de_vie: p[3],
            module_d: p[4],
            total: p[0] + p[1] + p[2] + p[3] + p[4],
        }
    }

    fn arrondir(&mut self) {
        for v in [
            &mut self.production,
            &mut self.edification,
            &mut self.exploitation,
            &mut self.fin_de_vie,
            &mut self.module_d,
            &mut self.total,
        ] {
            *v = arrondir_restitution(*v);
        }
    }
}

/// Contribution aux impacts : méthode statique et méthode dynamique.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct ContributionAcv {
    /// Méthode statique (PRG à 100 ans).
    pub i_statique: PhasesAcv,
    /// Méthode dynamique (pondération de l'article 11 de l'arrêté).
    pub ic: PhasesAcv,
}

impl ContributionAcv {
    fn arrondir(&mut self) {
        self.i_statique.arrondir();
        self.ic.arrondir();
    }
}

/// Contribution d'un sous-lot (ou d'un lot sans sous-lot) aux impacts des composants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsSousLotAcv {
    /// Sous-lot (`3.1`…) ou lot sans sous-lot (`10`, `12`, `13`…).
    pub sous_lot: String,
    /// Vrai si la valeur forfaitaire de l'annexe XI est utilisée (§4.2.2).
    pub forfaitaire: bool,
    /// Contribution (éq. 15, 21, 22, 63, 64), fluides frigorigènes compris.
    pub contribution: ContributionAcv,
    /// Part de `ic.total` due aux données par défaut et aux valeurs forfaitaires (« Icded,k »,
    /// §5.3.4), en kg éq. CO2/m².
    pub ic_ded_kgco2_m2: f64,
    /// « StockCsous lot k », en kg C/m² (éq. 140).
    pub stock_c_kgc_m2: f64,
}

/// Contribution d'un lot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsLotAcv {
    /// Numéro de lot (1 à 13).
    pub lot: u8,
    /// Contribution du lot (éq. 21 et 22).
    pub contribution: ContributionAcv,
    /// « Iclot,k » : impact dynamique du lot, en kg éq. CO2/m² (`contribution.ic.total`).
    pub ic_kgco2_m2: f64,
    /// « Icded,k » du lot, en kg éq. CO2/m².
    pub ic_ded_kgco2_m2: f64,
    /// « UDDk » (éq. 147), en % ; absent si l'impact du lot est nul.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub udd_pct: Option<f64>,
    /// « StockClot i », en kg C/m² (éq. 141).
    pub stock_c_kgc_m2: f64,
    /// Sous-lots du lot.
    pub sous_lots: Vec<ResultatsSousLotAcv>,
}

/// Quantités d'eau annuelles du bâtiment (§4.4.2) et de la parcelle (§4.6.2), en m³/an.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct QuantitesEauAcv {
    /// « Nocc » (§5.2.3).
    pub nocc: f64,
    /// « Qeau potable conv » du tableau 7, en m³/occupant/an.
    pub q_eau_potable_conv_m3: f64,
    /// « Qeau potable » (éq. 90).
    pub q_eau_potable_m3: f64,
    /// « Qeaux usées » (éq. 92).
    pub q_eaux_usees_m3: f64,
    /// « Qeaux pluviales » (éq. 85) ; nul hors réseau unitaire.
    pub q_eaux_pluviales_m3: f64,
    /// « Qparcelle,eau potable » (éq. 121).
    pub q_parcelle_eau_potable_m3: f64,
    /// « Qparcelle,eaux usées » (éq. 123).
    pub q_parcelle_eaux_usees_m3: f64,
}

/// Exigence `Icconstruction_max` d'une zone et ses coefficients de modulation (annexe de
/// l'article R. 172-4, chapitre III, III), en kg éq. CO2/m² sauf les coefficients sans unité
/// `mi_combles`, `mi_surf_moy`, `mi_surf_tot`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExigencesIcConstructionZone {
    /// Zone.
    pub zone: String,
    /// Numéro d'usage.
    pub usage: u8,
    /// `Sref` de la zone, en m².
    pub sref_m2: f64,
    /// « Icconstruction_maxmoyen ».
    pub ic_construction_max_moyen: f64,
    /// « Micombles ».
    pub mi_combles: f64,
    /// « Misurf_moy ».
    pub mi_surf_moy: f64,
    /// « Misurf_tot ».
    pub mi_surf_tot: f64,
    /// « Migéo ».
    pub mi_geo: f64,
    /// « Miinfra ».
    pub mi_infra: f64,
    /// « Mivrd ».
    pub mi_vrd: f64,
    /// « Mipv ».
    pub mi_pv: f64,
    /// « Mided ».
    pub mi_ded: f64,
    /// « Miagrément_ext ».
    pub mi_agrement_ext: f64,
    /// « MiHSP ».
    pub mi_hsp: f64,
    /// « Miclim_RCU ».
    pub mi_clim_rcu: f64,
    /// « Icconstruction_max » de la zone.
    pub ic_construction_max_kgco2_m2: f64,
}

/// Exigence `Icconstruction_max` du bâtiment et sa vérification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExigencesIcConstruction {
    /// « Icconstruction_max » du bâtiment : moyenne des zones pondérée par leur `Sref`
    /// (chapitre II, V).
    pub ic_construction_max_kgco2_m2: f64,
    /// Exigences de chaque zone.
    pub zones: Vec<ExigencesIcConstructionZone>,
    /// Vrai si `Icconstruction ≤ Icconstruction_max`.
    pub respectee: bool,
}

/// Résultats de l'ACV d'un bâtiment (annexe II §5.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsAcv {
    /// « PER » : période d'étude de référence, en années.
    pub per_ans: u32,
    /// Contribution « composants » (éq. 27 à 30, fluides frigorigènes compris).
    pub composants: ContributionAcv,
    /// Part de la contribution « composants » due aux fluides frigorigènes (calcul propre aux
    /// fluides, §4.2.1.1.6, §4.2.1.2.2, §4.2.2.2.2).
    pub fluides_frigorigenes: ContributionAcv,
    /// Contribution « chantier » (éq. 97) : phase d'édification seule.
    pub chantier: ContributionAcv,
    /// Contribution « eau » (éq. 86 et 90) : phase d'exploitation seule.
    pub eau: ContributionAcv,
    /// Contribution « parcelle » (éq. 118) ; absente sans parcelle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parcelle: Option<ContributionAcv>,
    /// Lots 1 à 13 (seuls ceux qui ont un composant ou une valeur forfaitaire).
    pub lots: Vec<ResultatsLotAcv>,
    /// « Iccomposants », en kg éq. CO2/m².
    pub ic_composants_kgco2_m2: f64,
    /// « Icchantier », en kg éq. CO2/m².
    pub ic_chantier_kgco2_m2: f64,
    /// « Iceau », en kg éq. CO2/m².
    pub ic_eau_kgco2_m2: f64,
    /// « Icénergie », en kg éq. CO2/m² (repris des indicateurs, §4.3.2) ; absent sans
    /// génération.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ic_energie_kgco2_m2: Option<f64>,
    /// « Icconstruction » = Iccomposants + Icchantier (§5.3.1.1, éq. 132).
    pub ic_construction_kgco2_m2: f64,
    /// « Icbâtiment » = Icconstruction + Icénergie + Iceau (éq. 134) ; absent sans Icénergie.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ic_batiment_kgco2_m2: Option<f64>,
    /// « Icparcelle », en kg éq. CO2/m² (éq. 118 en dynamique) ; absent sans parcelle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ic_parcelle_kgco2_m2: Option<f64>,
    /// « Icprojet » = Icbâtiment + Icparcelle (éq. 136).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ic_projet_kgco2_m2: Option<f64>,
    /// « Icconstruction,occ », en kg éq. CO2/occupant (éq. 133) ; absent si Nocc est nul.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ic_construction_occ_kgco2: Option<f64>,
    /// « Icbâtiment,occ » (éq. 135).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ic_batiment_occ_kgco2: Option<f64>,
    /// « Icprojet,occ » (éq. 137).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ic_projet_occ_kgco2: Option<f64>,
    /// « Icded » de l'annexe de l'article R. 172-4 (chapitre I, IX) : données par défaut et
    /// valeurs forfaitaires, hors lots 1, 2 et 13 ; en kg éq. CO2/m².
    pub ic_ded_kgco2_m2: f64,
    /// « Icded_total » (§5.3.4) : tous lots, en kg éq. CO2/m².
    pub ic_ded_total_kgco2_m2: f64,
    /// « UDDbâtiment » = Icded_total / Iccomposants (éq. 146), en % ; absent si Iccomposants
    /// est nul.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub udd_batiment_pct: Option<f64>,
    /// « StockCbâtiment », en kg C/m² (éq. 142).
    pub stock_c_kgc_m2: f64,
    /// « StockCparcelle », en kg C/m² (éq. 143) ; absent sans parcelle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stock_c_parcelle_kgc_m2: Option<f64>,
    /// Quantités d'eau.
    pub eau_quantites: QuantitesEauAcv,
    /// « TAC_PVbat elec » appliqué au lot 13 (éq. 22), en % ; absent sans lot 13.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tac_pv_pct: Option<f64>,
    /// « TAC_cogé global » appliqué au sous-lot 8.2 (annexe II, éq. 16), en % ; absent sans
    /// sous-lot 8.2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tac_coge_global_pct: Option<f64>,
    /// Exigence `Icconstruction_max` ; absente sans `exigences` du bâtiment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exigences: Option<ExigencesIcConstruction>,
    /// Valeurs par défaut du texte appliquées (DVE = DVR, distances de 30 km, ε1…).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

impl ResultatsAcv {
    /// Arrondi de restitution (§1.3.1, Q8).
    pub fn arrondir(&mut self) {
        let a = arrondir_restitution;
        self.composants.arrondir();
        self.fluides_frigorigenes.arrondir();
        self.chantier.arrondir();
        self.eau.arrondir();
        if let Some(p) = &mut self.parcelle {
            p.arrondir();
        }
        for l in &mut self.lots {
            l.contribution.arrondir();
            l.ic_kgco2_m2 = a(l.ic_kgco2_m2);
            l.ic_ded_kgco2_m2 = a(l.ic_ded_kgco2_m2);
            l.stock_c_kgc_m2 = a(l.stock_c_kgc_m2);
            l.udd_pct = l.udd_pct.map(a);
            for s in &mut l.sous_lots {
                s.contribution.arrondir();
                s.ic_ded_kgco2_m2 = a(s.ic_ded_kgco2_m2);
                s.stock_c_kgc_m2 = a(s.stock_c_kgc_m2);
            }
        }
        for v in [
            &mut self.ic_composants_kgco2_m2,
            &mut self.ic_chantier_kgco2_m2,
            &mut self.ic_eau_kgco2_m2,
            &mut self.ic_construction_kgco2_m2,
            &mut self.ic_ded_kgco2_m2,
            &mut self.ic_ded_total_kgco2_m2,
            &mut self.stock_c_kgc_m2,
        ] {
            *v = a(*v);
        }
        for v in [
            &mut self.ic_energie_kgco2_m2,
            &mut self.ic_batiment_kgco2_m2,
            &mut self.ic_parcelle_kgco2_m2,
            &mut self.ic_projet_kgco2_m2,
            &mut self.ic_construction_occ_kgco2,
            &mut self.ic_batiment_occ_kgco2,
            &mut self.ic_projet_occ_kgco2,
            &mut self.udd_batiment_pct,
            &mut self.stock_c_parcelle_kgc_m2,
            &mut self.tac_pv_pct,
            &mut self.tac_coge_global_pct,
        ] {
            *v = v.map(a);
        }
        if let Some(x) = &mut self.exigences {
            x.ic_construction_max_kgco2_m2 = a(x.ic_construction_max_kgco2_m2);
            for z in &mut x.zones {
                for v in [
                    &mut z.sref_m2,
                    &mut z.ic_construction_max_moyen,
                    &mut z.mi_geo,
                    &mut z.mi_infra,
                    &mut z.mi_vrd,
                    &mut z.mi_pv,
                    &mut z.mi_ded,
                    &mut z.mi_agrement_ext,
                    &mut z.mi_hsp,
                    &mut z.mi_clim_rcu,
                    &mut z.ic_construction_max_kgco2_m2,
                ] {
                    *v = a(*v);
                }
                for v in [&mut z.mi_combles, &mut z.mi_surf_moy, &mut z.mi_surf_tot] {
                    *v = (*v * 10000.0).round() / 10000.0;
                }
            }
        }
        let q = &mut self.eau_quantites;
        for v in [
            &mut q.nocc,
            &mut q.q_eau_potable_conv_m3,
            &mut q.q_eau_potable_m3,
            &mut q.q_eaux_usees_m3,
            &mut q.q_eaux_pluviales_m3,
            &mut q.q_parcelle_eau_potable_m3,
            &mut q.q_parcelle_eaux_usees_m3,
        ] {
            *v = a(*v);
        }
    }
}
