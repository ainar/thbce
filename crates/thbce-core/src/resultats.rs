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
    /// Générations, issues du calcul Th-C : énergie à fournir, sommée sur leurs réseaux. En
    /// attendant les générateurs (lots 6c et 6d), chaque génération est idéale : elle fournit
    /// exactement la demande. Absent sans génération.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generations: Vec<ResultatsGeneration>,
}

/// Réseau de distribution intergroupes, calcul Th-C.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsIntergroupes {
    /// Identifiant du réseau.
    pub id: Id,
    /// Identifiant de la génération qui l'alimente.
    pub generation: Id,
    /// Demande de chauffage transmise à la génération, pertes du réseau comprises, en kWh.
    pub q_ch_kwh: f64,
    /// Demande de refroidissement transmise à la génération (valeur absolue), en kWh.
    pub q_fr_kwh: f64,
    /// Demandes mensuelles de chauffage, en kWh (mois des scénarios).
    pub q_ch_mensuel_kwh: Vec<f64>,
    /// Demandes mensuelles de refroidissement, en kWh.
    pub q_fr_mensuel_kwh: Vec<f64>,
    /// Consommation des circulateurs du réseau, en kWh d'énergie finale électrique.
    pub w_aux_kwh: f64,
    /// Pertes du réseau vers le volume chauffé, en kWh.
    pub pertes_vc_kwh: f64,
    /// Pertes du réseau hors volume chauffé, en kWh.
    pub pertes_hvc_kwh: f64,
    /// Nombre d'heures où le réseau transmet une demande.
    pub nbh_fonct: u32,
    /// Température de départ extrême demandée à la génération (la plus haute en chauffage, la
    /// plus basse en refroidissement), en °C ; absente pour un réseau fictif.
    pub theta_dep_extreme_c: Option<f64>,
}

/// Génération, calcul Th-C.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsGeneration {
    /// Identifiant de la génération.
    pub id: Id,
    /// Énergie de chauffage à fournir, en kWh.
    pub q_ch_kwh: f64,
    /// Énergie de refroidissement à fournir (valeur absolue), en kWh.
    pub q_fr_kwh: f64,
}

/// Systèmes d'émission d'un groupe, calcul Th-C (§8.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsSystemesGroupe {
    /// « Qsys_ch » : énergie de chauffage requise par les émetteurs du groupe, pertes au dos
    /// comprises, en kWh (éq. 824).
    pub q_sys_ch_kwh: f64,
    /// « Qsys_fr » : énergie de refroidissement requise (valeur absolue), en kWh.
    pub q_sys_fr_kwh: f64,
    /// Valeurs mensuelles de `q_sys_ch_kwh`, en kWh.
    pub q_sys_ch_mensuel_kwh: Vec<f64>,
    /// Valeurs mensuelles de `q_sys_fr_kwh`, en kWh.
    pub q_sys_fr_mensuel_kwh: Vec<f64>,
    /// Énergies requises par émetteur (éq. 810).
    pub emetteurs: Vec<ResultatsEmetteur>,
    /// « Wvent_loc_tot » : consommation des ventilateurs locaux, en kWh d'énergie finale
    /// électrique (éq. 815).
    pub w_vent_loc_kwh: f64,
    /// Nombre d'heures de relance (éq. 848).
    pub nbh_relance: u32,
    /// Réseaux de distribution du groupe (§8.7 et §8.8).
    pub distributions: Vec<ResultatsDistributionGroupe>,
    /// Saisons propres du groupe en calcul Th-C (§4.6), dont les générations tirent les
    /// saisons effectives (§8.4).
    pub saisons_th_c: Option<ResultatsSaisons>,
}

/// Réseau de distribution du groupe, calcul Th-C (valeurs absolues en refroidissement).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsDistributionGroupe {
    /// Identifiant.
    pub id: Id,
    /// Demande transmise au réseau intergroupes, pertes comprises, en kWh.
    pub q_kwh: f64,
    /// Pertes vers le volume chauffé, en kWh (récupérables, §11.1).
    pub pertes_vc_kwh: f64,
    /// Pertes hors volume chauffé, en kWh.
    pub pertes_hvc_kwh: f64,
    /// Consommation du circulateur, en kWh d'énergie finale électrique.
    pub w_aux_kwh: f64,
}

/// Émetteur, calcul Th-C.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsEmetteur {
    /// Identifiant.
    pub id: Id,
    /// Énergie de chauffage requise, en kWh.
    pub q_ch_kwh: f64,
    /// Énergie de refroidissement requise (valeur absolue), en kWh.
    pub q_fr_kwh: f64,
}

impl ResultatsSystemesGroupe {
    fn arrondir(&mut self) {
        let a = arrondir_restitution;
        self.q_sys_ch_kwh = a(self.q_sys_ch_kwh);
        self.q_sys_fr_kwh = a(self.q_sys_fr_kwh);
        self.w_vent_loc_kwh = a(self.w_vent_loc_kwh);
        for x in self
            .q_sys_ch_mensuel_kwh
            .iter_mut()
            .chain(self.q_sys_fr_mensuel_kwh.iter_mut())
        {
            *x = a(*x);
        }
        for e in &mut self.emetteurs {
            e.q_ch_kwh = a(e.q_ch_kwh);
            e.q_fr_kwh = a(e.q_fr_kwh);
        }
        for d in &mut self.distributions {
            d.q_kwh = a(d.q_kwh);
            d.pertes_vc_kwh = a(d.pertes_vc_kwh);
            d.pertes_hvc_kwh = a(d.pertes_hvc_kwh);
            d.w_aux_kwh = a(d.w_aux_kwh);
        }
    }
}

impl ResultatsIntergroupes {
    fn arrondir(&mut self) {
        let a = arrondir_restitution;
        for x in [
            &mut self.q_ch_kwh,
            &mut self.q_fr_kwh,
            &mut self.w_aux_kwh,
            &mut self.pertes_vc_kwh,
            &mut self.pertes_hvc_kwh,
        ] {
            *x = a(*x);
        }
        for x in self
            .q_ch_mensuel_kwh
            .iter_mut()
            .chain(self.q_fr_mensuel_kwh.iter_mut())
        {
            *x = a(*x);
        }
        self.theta_dep_extreme_c = self.theta_dep_extreme_c.map(a);
    }
}

/// Résultats d'un espace tampon (Th-BCE 2020 §5.4) : synthèse annuelle de ses sorties
/// horaires dans chacun des deux calculs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsEspaceTampon {
    /// Identifiant de l'espace tampon.
    pub id: Id,
    /// Calcul du Bbio (mode Th-B).
    pub th_b: Option<ResultatsEspaceTamponMode>,
    /// Calcul du confort d'été (mode Th-D).
    pub th_d: Option<ResultatsEspaceTamponMode>,
    /// Valeurs par défaut du texte appliquées (`Cm_et_surf`, `Qv,et,base`).
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// Synthèse annuelle d'un espace tampon dans un mode de calcul.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsEspaceTamponMode {
    /// Moyenne annuelle de « btherm(h) ».
    pub b_therm_moyen: f64,
    /// Moyenne annuelle de « θet(h) », en °C.
    pub theta_et_moy_c: f64,
    /// Plus basse « θet(h) », en °C.
    pub theta_et_min_c: f64,
    /// Plus haute « θet(h) », en °C.
    pub theta_et_max_c: f64,
    /// Nombre d'heures où les parois de l'espace tampon sont ouvertes (surventilation).
    pub nbh_surventilation: u32,
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
    /// Besoins mensuels et annuels et Bbio de la zone (§13.1, « Sortie_Zone_B »).
    pub bbio: Option<ResultatsBbio>,
    /// « DH » du groupe le plus défavorable de la zone, en °C.h (§13.5.3, éq. 2551).
    pub dh_c_h: Option<f64>,
    /// Système de ventilation du projet (§6.2 à 6.5) ; absent hors habitation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ventilation: Option<ResultatsVentilationZone>,
    /// Résultats par groupe.
    pub groupes: Vec<ResultatsGroupe>,
}

/// Système de ventilation du projet d'une zone (Th-BCE 2020 §6.2 à 6.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsVentilationZone {
    /// Identifiant du système.
    pub id: Id,
    /// Puissance moyenne des ventilateurs, en W (éq. 391 à 394, 620, 641 et 643), constante
    /// en habitation.
    pub p_vent_w: f64,
    /// Consommation annuelle des ventilateurs, en kWh d'énergie finale électrique : la
    /// puissance moyenne sommée heure par heure (éq. 565, 621 et 645).
    pub c_aux_vent_kwh: f64,
    /// Fonctionnement du système dans le calcul Th-D ; absent si le calcul Th-D n'emploie pas
    /// le système du projet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub th_d: Option<ResultatsVentilationMode>,
    /// Valeurs par défaut de la méthode appliquées au système et à ses bouches ; champ relatif
    /// à la zone.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// Fonctionnement annuel du système de ventilation du projet dans un calcul.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsVentilationMode {
    /// Nombre d'heures où l'échangeur d'un double flux est contourné par son bypass (éq. 494).
    pub nbh_bypass: u32,
    /// Énergie fournie pour la sécurité antigel de l'échangeur, en kWh (éq. 508).
    pub q_ag_kwh: f64,
}

/// Ventilation du projet vue d'un groupe (Th-BCE 2020 §6.2 à 6.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsVentilationGroupe {
    /// « qspec_repris » : débit spécifique repris au groupe, fuites en volume chauffé
    /// comprises, en m³/h, négatif (éq. 428).
    pub q_spec_repris_m3_h: f64,
    /// « qspec_soufflé » : débit spécifique soufflé dans le groupe, fuites en volume chauffé
    /// comprises, en m³/h (éq. 429).
    pub q_spec_souffle_m3_h: f64,
    /// « qrepris,cond » : débit repris vu par le ventilateur, en m³/h, négatif (éq. 430).
    pub q_repris_cond_m3_h: f64,
    /// « qsoufflé,cond » : débit soufflé vu par le ventilateur, en m³/h (éq. 431).
    pub q_souffle_cond_m3_h: f64,
    /// Puissance des ventilateurs attribuée au groupe, en W (éq. 570, 642 et 644).
    pub p_vent_w: f64,
    /// Consommation annuelle des auxiliaires de ventilation du groupe, en kWh d'énergie finale
    /// électrique (« Waux,v^gr », éq. 570 ; « Cvent^g », éq. 646).
    pub c_aux_vent_kwh: f64,
    /// La même, par m² de surface de référence du groupe, en kWh/m².
    pub c_aux_vent_kwh_m2: f64,
    /// Consommations mensuelles, en kWh, mois au sens des scénarios (§4.1.3).
    pub c_aux_vent_mensuel_kwh: Vec<f64>,
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
    /// Débits du système de ventilation conventionnel du Bbio ; `null` si le groupe ne décrit
    /// aucune bouche de ventilation ou hors habitation.
    pub debits_bbio: Option<ResultatsDebitsBbio>,
    /// Besoins mensuels et annuels et Bbio du groupe (§13.1, « Sortie_Groupe_B »).
    pub bbio: Option<ResultatsBbio>,
    /// Saisons propres du groupe en calcul du Bbio (§4.6).
    pub saisons_bbio: Option<ResultatsSaisons>,
    /// Indicateurs de confort du groupe en calcul du Bbio (§4.5), en évolution libre.
    pub confort_bbio: Option<ResultatsConfort>,
    /// Confort d'été du groupe, issu du calcul Th-D (§13.5).
    pub confort_ete: Option<ResultatsConfortEte>,
    /// Saisons propres du groupe en calcul Th-D (§4.6).
    pub saisons_th_d: Option<ResultatsSaisons>,
    /// Ventilation du projet vue du groupe : débits et consommation des ventilateurs ; absente
    /// hors habitation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ventilation: Option<ResultatsVentilationGroupe>,
    /// Systèmes d'émission du groupe, issus du calcul des consommations (Th-C) ; absents sans
    /// émetteur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub systemes: Option<ResultatsSystemesGroupe>,
    /// Résultats par baie.
    pub baies: Vec<ResultatsBaie>,
    /// Valeurs par défaut de la méthode appliquées aux paramètres non saisis du groupe ; liste
    /// vide si aucune.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// Besoins et besoin bioclimatique d'un mois ou de l'année (Th-BCE 2020 §13.1.2, sorties).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct BesoinsBbio {
    /// « B_Ch » : besoin de chauffage, en kWh par m² de surface de référence (éq. 2379 et
    /// 2382).
    pub b_ch_kwh_m2: f64,
    /// « B_Fr » : besoin de refroidissement, en kWh par m² de surface de référence, compté
    /// positivement (éq. 2380 et 2383).
    pub b_fr_kwh_m2: f64,
    /// « B_Ecl » : besoin d'éclairage, en kWh par m² de surface de référence (éq. 2381 et
    /// 2384).
    pub b_ecl_kwh_m2: f64,
    /// « Bbio_pts » : besoin bioclimatique, en points (éq. 2385 et 2386 ; décision Q10).
    pub bbio_pts: f64,
}

/// Sorties du mode Th-B d'un groupe, d'une zone ou d'un bâtiment (Th-BCE 2020 §13.1).
///
/// Sans arrondi de restitution, le Bbio annuel est la somme des douze Bbio mensuels (éq. 2386) ;
/// avec l'arrondi, chaque valeur est arrondie séparément à une décimale (décision Q8) et la
/// somme des valeurs mensuelles arrondies peut différer de la valeur annuelle arrondie.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsBbio {
    /// Besoins et Bbio de l'année.
    pub annuel: BesoinsBbio,
    /// Besoins et Bbio de chacun des douze mois de l'année conventionnelle (§4.1.3 : mois de 4
    /// ou 5 semaines, au sens des scénarios).
    pub mensuel: Vec<BesoinsBbio>,
}

/// Saisons propres d'un groupe (Th-BCE 2020 §4.6.3.6, éq. 92) : jours de l'année conventionnelle
/// de 1 à 365, au sens des scénarios (le jour 57 est le premier jour de mars). `null` si
/// l'événement n'a pas eu lieu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsSaisons {
    /// « jfin_ch » : dernier jour où la saison de chauffage s'est arrêtée.
    pub j_fin_ch: Option<u16>,
    /// « jdebut_ch » : dernier jour où la saison de chauffage a commencé (hors début d'année,
    /// où le chauffage est autorisé d'emblée).
    pub j_debut_ch: Option<u16>,
    /// « jdebut_fr » : dernier jour où la saison de refroidissement a commencé.
    pub j_debut_fr: Option<u16>,
    /// « jfin_fr » : dernier jour où la saison de refroidissement s'est arrêtée.
    pub j_fin_fr: Option<u16>,
}

/// Indicateurs de confort d'un groupe sur l'année (Th-BCE 2020 §4.5.3.4 et 4.5.3.5), calculés
/// en occupation sur la température opérative en évolution libre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsConfort {
    /// « Nbhinc_cons,ch » : heures d'inconfort froid par rapport à la consigne de chauffage.
    pub nbh_inc_cons_ch: u32,
    /// « DHinc_cons,ch » : degrés-heures d'inconfort froid, en K.h (éq. 64).
    pub dh_inc_cons_ch_k_h: f64,
    /// « Nbhinc_cons,fr » : heures d'inconfort chaud par rapport à la consigne de
    /// refroidissement.
    pub nbh_inc_cons_fr: u32,
    /// « DHinc_cons,fr » : degrés-heures d'inconfort chaud par rapport à la consigne de
    /// refroidissement, en K.h (éq. 65).
    pub dh_inc_cons_fr_k_h: f64,
    /// « Nbhinc_C1,fr » à « Nbhinc_C3,fr » : heures d'inconfort chaud par rapport au seuil des
    /// catégories d'ambiance 1 à 3.
    pub nbh_inc_c_fr: [u32; 3],
    /// « DHinc_C1,fr » à « DHinc_C3,fr », en K.h (éq. 65).
    pub dh_inc_c_fr_k_h: [f64; 3],
}

/// Confort d'été d'un groupe (Th-BCE 2020 §13.5), issu du calcul Th-D : jeu météo avec séquence
/// caniculaire, sans refroidissement. Les indicateurs portent sur les pas de temps occupés de
/// la période de confort adaptatif.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsConfortEte {
    /// « DH » : degrés-heures d'inconfort, en °C.h (éq. 2551).
    pub dh_c_h: f64,
    /// « Nbhinconf » : heures où la température opérative atteint la température d'inconfort
    /// chaud (éq. 2548).
    pub nbh_inconf: u32,
    /// « Nbhinconf+1C » : heures où elle la dépasse d'au moins 1 °C (éq. 2549).
    pub nbh_inconf_1c: u32,
    /// « Nbhinconf+2C » : heures où elle la dépasse d'au moins 2 °C (éq. 2550).
    pub nbh_inconf_2c: u32,
    /// Nombre d'heures occupées de la période de confort adaptatif, sur lesquelles portent les
    /// indicateurs.
    pub nbh_occ_conf_adapt: u32,
    /// Température opérative maximale du groupe sur l'année, en °C (fin de pas de temps).
    pub theta_op_max_c: f64,
    /// Température opérative maximale du groupe en occupation pendant la période de confort
    /// adaptatif, en °C ; `null` si la période est vide.
    pub theta_op_max_occ_c: Option<f64>,
}

/// Débits spécifiques d'un groupe pour le calcul du Bbio (Th-BCE 2020 §6.1 et §6.2), constants
/// sur l'année en habitation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsDebitsBbio {
    /// « qspec_repris » : débit volumique spécifique repris, en m³/h, négatif (§6.2.3.2.2.2,
    /// éq. 403, avec `Crdbnr = 1`, `Dugd = 14` h par semaine et `Cdep = 1`).
    pub q_spec_repris_m3_h: f64,
    /// « qspec_soufflé » : débit volumique spécifique soufflé, en m³/h, positif.
    pub q_spec_souffle_m3_h: f64,
}

/// Résultats d'une baie.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsBaie {
    /// Identifiant de la baie.
    pub id: Id,
    /// Valeurs par défaut de la méthode appliquées aux paramètres non saisis de la baie ; liste
    /// vide si tout a été saisi.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
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
    /// « Abaies » : surface des baies, en m² (§5.14.3, éq. 282).
    pub a_baies_m2: f64,
    /// « Abaies-v » : surface des baies verticales, d'inclinaison au moins égale à 60°, en m²
    /// (éq. 279).
    pub a_baies_v_m2: f64,
    /// « Abaies-h » : surface des baies horizontales, d'inclinaison inférieure à 60°, en m²
    /// (éq. 281).
    pub a_baies_h_m2: f64,
    /// « Hges » des baies sans protection mobile en place, en W/K (§5.10.3.5, éq. 210 avec
    /// `Rprot = 0`). La valeur horaire dépend de la position des protections mobiles.
    pub h_ges_sp_w_k: f64,
    /// « Af,perm » : surface des parois déperditives verticales, opaques et vitrées, en m²,
    /// utile aux calculs de perméabilité (§5.7.2 ; « Af » du §5.6.3.1.5).
    pub a_f_perm_m2: f64,
    /// « Atoit,perm » : surface des parois déperditives horizontales hors plancher bas, opaques
    /// et vitrées, en m², utile aux calculs de perméabilité (§5.7.2 ; « At » du §5.6.3.1.5).
    pub a_toit_perm_m2: f64,
    /// « Ll » : longueur des ponts thermiques, en m (éq. 310).
    pub l_pt_m: f64,
    /// « HTh » des ponts thermiques, en W/K (éq. 307).
    pub h_th_pt_w_k: f64,
}

impl ResultatsBbio {
    /// Applique l'arrondi de restitution à chaque valeur (§1.3.1, décision Q8).
    fn arrondir(&mut self) {
        for b in std::iter::once(&mut self.annuel).chain(&mut self.mensuel) {
            b.b_ch_kwh_m2 = arrondir_restitution(b.b_ch_kwh_m2);
            b.b_fr_kwh_m2 = arrondir_restitution(b.b_fr_kwh_m2);
            b.b_ecl_kwh_m2 = arrondir_restitution(b.b_ecl_kwh_m2);
            b.bbio_pts = arrondir_restitution(b.bbio_pts);
        }
    }
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
            for d in &mut b.distributions_intergroupes {
                d.arrondir();
            }
            for g in &mut b.generations {
                g.q_ch_kwh = a(g.q_ch_kwh);
                g.q_fr_kwh = a(g.q_fr_kwh);
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
                    if let Some(m) = &mut v.th_d {
                        m.q_ag_kwh = a(m.q_ag_kwh);
                    }
                }
                if let Some(bbio) = &mut z.bbio {
                    bbio.arrondir();
                }
                for g in &mut z.groupes {
                    g.sref_m2 = a(g.sref_m2);
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
