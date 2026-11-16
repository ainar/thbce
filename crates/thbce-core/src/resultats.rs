// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats du calcul.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::climat::OrigineMeteo;
use crate::conventions::arrondir_restitution;
use crate::entree::Id;
use thbce_data::ZoneClimatique;

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
    /// Climat du projet (Th-BCE 2020 §3.1).
    pub climat: ResultatsClimat,
    /// Résultats par bâtiment.
    pub batiments: Vec<ResultatsBatiment>,
}

/// Grandeurs du climat extérieur propres au projet (Th-BCE 2020 §3.1.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsClimat {
    /// Zone climatique retenue.
    pub zone_climatique: ZoneClimatique,
    /// « altcorr » : altitude conventionnelle du site, en m.
    pub altcorr_m: f64,
    /// « θbase_ext » : température extérieure de base corrigée de l'altitude, en °C.
    pub base_ext_c: f64,
    /// Période de confort adaptatif déterminée sur le jeu météo « Th-BC ».
    pub confort_adaptatif_th_bc: PeriodeConfortAdaptatif,
    /// Période de confort adaptatif déterminée sur le jeu météo « Th-D ».
    pub confort_adaptatif_th_d: PeriodeConfortAdaptatif,
}

/// Période de confort adaptatif (§3.1.3, éq. 1 et 2) : jours de l'année numérotés de 1 à 365.
/// Les deux bornes valent `null` si la moyenne glissante n'atteint jamais 16 °C.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PeriodeConfortAdaptatif {
    /// « Premier_jour ».
    pub premier_jour: Option<u16>,
    /// « Dernier_jour ».
    pub dernier_jour: Option<u16>,
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
    /// « Nadeq » : nombre d'adultes équivalents de la zone, en habitation (§4.1.3, éq. 32 et 36).
    pub nadeq: Option<f64>,
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
    /// Grandeurs d'enveloppe du groupe.
    pub enveloppe: ResultatsEnveloppe,
}

/// Grandeurs d'enveloppe d'un groupe (Th-BCE 2020 §5.16.3 et §5.20.3). Les coefficients de
/// transmission sont ceux des éléments à leur inclinaison réelle (§5.17.3, éq. 294).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsEnveloppe {
    /// « Aop » : surface des parois opaques, en m² (éq. 286).
    pub a_op_m2: f64,
    /// « Aopv » : surface des parois opaques verticales, en m² (éq. 287).
    pub a_opv_m2: f64,
    /// « Aophh » : surface des parois opaques horizontales tournées vers le haut, en m² (éq. 287).
    pub a_ophh_m2: f64,
    /// « Aophb » : surface des parois opaques horizontales tournées vers le bas, en m² (éq. 287).
    pub a_ophb_m2: f64,
    /// « HTh » des parois opaques, en W/K (éq. 283).
    pub h_th_op_w_k: f64,
    /// « Ll » : longueur des ponts thermiques, en m (éq. 310).
    pub l_pt_m: f64,
    /// « HTh » des ponts thermiques, en W/K (éq. 307).
    pub h_th_pt_w_k: f64,
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
                z.nadeq = z.nadeq.map(a);
                for g in &mut z.groupes {
                    g.sref_m2 = a(g.sref_m2);
                    let e = &mut g.enveloppe;
                    for v in [
                        &mut e.a_op_m2,
                        &mut e.a_opv_m2,
                        &mut e.a_ophh_m2,
                        &mut e.a_ophb_m2,
                        &mut e.h_th_op_w_k,
                        &mut e.l_pt_m,
                        &mut e.h_th_pt_w_k,
                    ] {
                        *v = a(*v);
                    }
                }
            }
        }
        self.valeurs_arrondies = true;
    }
}
