// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats du calcul.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thbce_data::ZoneClimatique;

use crate::climat::OrigineMeteo;
use crate::conventions::arrondir_restitution;
use crate::entree::Id;

// Un fichier par famille ; chaque type est réexporté ici, de sorte que le chemin
// `crate::resultats::Type` reste celui de l'API. Un lot ajoute ses types dans le fichier de sa
// famille (avec, le cas échéant, leur méthode `arrondir`), et l'appel de l'arrondi dans
// `Resultats::arrondir`.
pub mod acv;
mod batiment;
mod besoins_confort;
mod consommations;
mod deplacements;
mod ecs;
mod emission_distribution;
mod enveloppe;
mod generation;
mod solaire;
mod ventilation;

pub use batiment::*;
pub use besoins_confort::*;
pub use consommations::*;
pub use deplacements::*;
pub use ecs::*;
pub use emission_distribution::*;
pub use enveloppe::*;
pub use generation::*;
pub use solaire::*;
pub use ventilation::*;

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
    /// « Bbio_pts_annuel » : besoin bioclimatique du bâtiment, en points (§13.1.3, éq. 2385 et
    /// 2386). Même valeur que `bbio.annuel.bbio_pts`.
    pub bbio_pts: Option<f64>,
    /// Besoins mensuels et annuels et Bbio du bâtiment (§13.1, « Sortie_Batiment_B »).
    pub bbio: Option<ResultatsBbio>,
    /// « Cep » : consommation d'énergie primaire, en kWhep/(m².an) (§13.2).
    pub cep_kwhep_m2: Option<f64>,
    /// « Cep,nr » : consommation d'énergie primaire non renouvelable, en kWhep/(m².an).
    pub cep_nr_kwhep_m2: Option<f64>,
    /// « DH » : degrés-heures d'inconfort, en °C.h (§13.5.3, éq. 2551). L'indicateur est défini
    /// par groupe (§13.5.1) : c'est ici celui du groupe le plus défavorable du bâtiment ; le DH
    /// de chaque groupe est dans `zones[].groupes[].confort_ete`.
    pub dh_c_h: Option<f64>,
    /// « Ic énergie », en kg éq. CO2/m² (annexe II §4.3.2, éq. 74).
    pub ic_energie_kgco2_m2: Option<f64>,
    /// Consommation annuelle des auxiliaires de ventilation du bâtiment (ventilateurs), en kWh
    /// d'énergie finale électrique : somme de celles de ses zones. Absente sans système de
    /// ventilation du projet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub c_aux_vent_kwh: Option<f64>,
    /// Résultats par zone.
    pub zones: Vec<ResultatsZone>,
    /// Espaces tampons du bâtiment (§5.2 à 5.4) ; absent sans espace tampon.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub espaces_tampons: Vec<ResultatsEspaceTampon>,
    /// Réseaux de distribution intergroupes, issus du calcul des consommations (Th-C) : demandes
    /// transmises aux générations (§8.9 et §8.10) ; absent sans réseau.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub distributions_intergroupes: Vec<ResultatsIntergroupes>,
    /// Distributions d'ECS intergroupes, calcul Th-C (§9.8) ; absent sans réseau.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub distributions_ecs_intergroupes: Vec<ResultatsEcsIntergroupe>,
    /// Récupérateurs collectifs sur les eaux grises (§9.21.5 et §9.21.6) ; absent sans
    /// récupérateur collectif.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recuperateurs_eaux_grises: Vec<ResultatsRecuperateurCollectif>,
    /// Générations, issues du calcul Th-C (§8.14, §8.17 et §8.29) : demandes de leurs réseaux,
    /// énergies fournies et consommations de leurs générateurs. Absent sans génération.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generations: Vec<ResultatsGeneration>,
    /// Consommations en énergie finale des générations attribuées au bâtiment, par poste et
    /// par type d'énergie (§8.29, éq. 1484 à 1486) : somme de celles de ses zones. Absent sans
    /// génération.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consommations: Option<Consommations>,
    /// Production photovoltaïque du bâtiment (§12), sur le jeu météo « Th-BC » ; absente sans
    /// installation. La répartition autoconsommé/exporté relève du chapitre 13.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_pv: Option<crate::pv::ResultatsProductionPv>,
    /// Postes propres au logement collectif et aux parkings du bâtiment, en électricité (§7.2,
    /// §10.1, §10.3, §10.4) : somme de ceux de ses zones. Absent sans logement collectif ni
    /// parking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consommations_collectif: Option<ConsommationsCollectif>,
    /// Ascenseurs, une entrée par cabine (§10.1) ; absent sans ascenseur.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ascenseurs: Vec<ResultatsAscenseur>,
    /// Parkings (§10.3 et §10.4) ; absent sans parking.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parkings: Vec<ResultatsParking>,
    /// Indicateurs réglementaires du bâtiment (§13.2 à 13.4, annexe II §4.3.2) : consommations
    /// importées par poste et par énergie, autoconsommation, forfait de refroidissement, Cep,
    /// Cep,nr, Ic énergie, exigences et conformité. Absents sans génération.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indicateurs: Option<crate::indicateurs::resultats::ResultatsIndicateurs>,
    /// Analyse du cycle de vie (annexe II §5.3) : Icconstruction, Icbâtiment, StockC ; absente
    /// sans données d'ACV.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acv: Option<acv::ResultatsAcv>,
}

// Signature imposée par `skip_serializing_if`.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn est_nul(x: &f64) -> bool {
    *x == 0.0
}

/// Valeur par défaut donnée par le texte de la méthode et appliquée à un paramètre non saisi.
/// Le moteur n'applique de valeur par défaut que lorsque le texte la donne, et la signale ici.
/// Ces valeurs sont des paramètres du calcul : l'arrondi de restitution ne leur est pas appliqué.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ValeurParDefaut {
    /// Champ de l'entrée auquel la valeur s'applique, relatif à l'élément (par exemple
    /// `sans_protection.sw1_c` pour le type d'une baie).
    pub champ: String,
    /// Valeur appliquée.
    pub valeur: f64,
    /// Règle du texte dont elle provient.
    pub regle: String,
}

impl Resultats {
    /// Applique l'arrondi de restitution à toutes les grandeurs (§1.3.1, décision Q8).
    pub fn arrondir(&mut self) {
        let a = arrondir_restitution;
        for b in &mut self.batiments {
            b.sref_m2 = a(b.sref_m2);
            b.bbio_pts = b.bbio_pts.map(a);
            if let Some(bbio) = &mut b.bbio {
                bbio.arrondir();
            }
            b.cep_kwhep_m2 = b.cep_kwhep_m2.map(a);
            b.cep_nr_kwhep_m2 = b.cep_nr_kwhep_m2.map(a);
            b.dh_c_h = b.dh_c_h.map(a);
            b.ic_energie_kgco2_m2 = b.ic_energie_kgco2_m2.map(a);
            b.c_aux_vent_kwh = b.c_aux_vent_kwh.map(a);
            if let Some(pv) = &mut b.production_pv {
                pv.arrondir();
            }
            if let Some(c) = &mut b.consommations_collectif {
                c.arrondir();
            }
            if let Some(i) = &mut b.indicateurs {
                i.arrondir();
            }
            if let Some(x) = &mut b.acv {
                x.arrondir();
            }
            for x in &mut b.ascenseurs {
                for v in [
                    &mut x.nb_demarrages,
                    &mut x.t_mob_h,
                    &mut x.pti_w,
                    &mut x.e_tm_kwh,
                    &mut x.e_ti_kwh,
                    &mut x.e_tot_kwh,
                ] {
                    *v = a(*v);
                }
            }
            for x in &mut b.parkings {
                x.e_vent_kwh = a(x.e_vent_kwh);
                x.e_ecl_kwh = a(x.e_ecl_kwh);
            }
            for d in &mut b.distributions_intergroupes {
                d.arrondir();
            }
            for g in &mut b.generations {
                for x in [
                    &mut g.q_ch_kwh,
                    &mut g.q_fr_kwh,
                    &mut g.q_ecs_kwh,
                    &mut g.q_fou_ch_kwh,
                    &mut g.q_fou_fr_kwh,
                    &mut g.q_fou_ecs_kwh,
                    &mut g.q_rep_fin_kwh,
                    &mut g.pertes_vc_kwh,
                ] {
                    *x = a(*x);
                }
                g.consommations.arrondir();
                for s in &mut g.stockages {
                    s.arrondir();
                }
                for c in &mut g.solaires_collectifs {
                    c.arrondir();
                }
                for x in &mut g.generateurs {
                    x.q_fou_ch_kwh = a(x.q_fou_ch_kwh);
                    x.q_fou_fr_kwh = a(x.q_fou_fr_kwh);
                    x.q_fou_ecs_kwh = a(x.q_fou_ecs_kwh);
                    x.w_aux_kwh = a(x.w_aux_kwh);
                    x.phi_rejet_kwh = a(x.phi_rejet_kwh);
                    x.consommations.arrondir();
                    // Efficacités restituées avec deux décimales.
                    for e in [
                        &mut x.eta_eff_ch_an,
                        &mut x.eta_eff_fr_an,
                        &mut x.eta_eff_ecs_an,
                    ] {
                        *e = e.map(|v| (v * 100.0).round() / 100.0);
                    }
                }
            }
            if let Some(c) = &mut b.consommations {
                c.arrondir();
            }
            for d in &mut b.distributions_ecs_intergroupes {
                for x in [
                    &mut d.q_w_prim_kwh,
                    &mut d.pertes_vc_kwh,
                    &mut d.pertes_hvc_kwh,
                    &mut d.w_aux_kwh,
                    &mut d.w_rechauf_kwh,
                ] {
                    *x = a(*x);
                }
                for x in &mut d.q_w_prim_mensuel_kwh {
                    *x = a(*x);
                }
                d.theta_depart_c = d.theta_depart_c.map(a);
                d.theta_entrant_moyen_c = d.theta_entrant_moyen_c.map(a);
            }
            for et in &mut b.espaces_tampons {
                for m in [&mut et.th_b, &mut et.th_d].into_iter().flatten() {
                    m.b_therm_moyen = a(m.b_therm_moyen);
                    m.theta_et_moy_c = a(m.theta_et_moy_c);
                    m.theta_et_min_c = a(m.theta_et_min_c);
                    m.theta_et_max_c = a(m.theta_et_max_c);
                }
            }
            for z in &mut b.zones {
                z.sref_m2 = a(z.sref_m2);
                z.nadeq = z.nadeq.map(a);
                z.dh_c_h = z.dh_c_h.map(a);
                if let Some(v) = &mut z.ventilation {
                    v.p_vent_w = a(v.p_vent_w);
                    v.c_aux_vent_kwh = a(v.c_aux_vent_kwh);
                    for m in [&mut v.th_d, &mut v.th_c].into_iter().flatten() {
                        m.q_ag_kwh = a(m.q_ag_kwh);
                        m.q_prechaud_kwh = m.q_prechaud_kwh.map(a);
                        m.q_prefroid_kwh = m.q_prefroid_kwh.map(a);
                        m.q_chaud_kwh = m.q_chaud_kwh.map(a);
                        m.q_froid_kwh = m.q_froid_kwh.map(a);
                    }
                }
                if let Some(c) = &mut z.consommations_collectif {
                    c.arrondir();
                }
                if let Some(c) = &mut z.consommations {
                    c.arrondir();
                }
                if let Some(bbio) = &mut z.bbio {
                    bbio.arrondir();
                }
                for g in &mut z.groupes {
                    g.sref_m2 = a(g.sref_m2);
                    if let Some(c) = &mut g.consommations {
                        c.arrondir();
                    }
                    if let Some(bbio) = &mut g.bbio {
                        bbio.arrondir();
                    }
                    if let Some(c) = &mut g.confort_bbio {
                        c.dh_inc_cons_ch_k_h = a(c.dh_inc_cons_ch_k_h);
                        c.dh_inc_cons_fr_k_h = a(c.dh_inc_cons_fr_k_h);
                        for v in &mut c.dh_inc_c_fr_k_h {
                            *v = a(*v);
                        }
                    }
                    if let Some(c) = &mut g.confort_ete {
                        c.dh_c_h = a(c.dh_c_h);
                        c.theta_op_max_c = a(c.theta_op_max_c);
                        c.theta_op_max_occ_c = c.theta_op_max_occ_c.map(a);
                    }
                    if let Some(v) = &mut g.ventilation {
                        for x in [
                            &mut v.q_spec_repris_m3_h,
                            &mut v.q_spec_souffle_m3_h,
                            &mut v.q_repris_cond_m3_h,
                            &mut v.q_souffle_cond_m3_h,
                            &mut v.p_vent_w,
                            &mut v.c_aux_vent_kwh,
                            &mut v.c_aux_vent_kwh_m2,
                        ] {
                            *x = a(*x);
                        }
                        for x in &mut v.c_aux_vent_mensuel_kwh {
                            *x = a(*x);
                        }
                    }
                    if let Some(s) = &mut g.systemes {
                        s.arrondir();
                    }
                    if let Some(d) = &mut g.debits_bbio {
                        d.q_spec_repris_m3_h = a(d.q_spec_repris_m3_h);
                        d.q_spec_souffle_m3_h = a(d.q_spec_souffle_m3_h);
                    }
                    let e = &mut g.enveloppe;
                    for v in [
                        &mut e.a_op_m2,
                        &mut e.a_opv_m2,
                        &mut e.a_ophh_m2,
                        &mut e.a_ophb_m2,
                        &mut e.h_th_op_w_k,
                        &mut e.a_baies_m2,
                        &mut e.a_baies_v_m2,
                        &mut e.a_baies_h_m2,
                        &mut e.h_ges_sp_w_k,
                        &mut e.a_f_perm_m2,
                        &mut e.a_toit_perm_m2,
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
