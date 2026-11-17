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
    /// Distributions d'ECS intergroupes, calcul Th-C (§9.8) ; absent sans réseau.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub distributions_ecs_intergroupes: Vec<ResultatsEcsIntergroupe>,
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
}

/// Consommation électrique d'un poste, annuelle et par mois des scénarios, en kWh d'énergie
/// finale.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct PosteElectrique {
    /// Consommation annuelle, en kWh.
    pub annuel_kwh: f64,
    /// Consommation annuelle par m² de surface de référence du niveau (zone ou bâtiment), en
    /// kWh/m².
    pub annuel_kwh_m2: f64,
    /// Consommation des douze mois, en kWh.
    pub mensuel_kwh: Vec<f64>,
}

impl PosteElectrique {
    fn arrondir(&mut self) {
        self.annuel_kwh = arrondir_restitution(self.annuel_kwh);
        self.annuel_kwh_m2 = arrondir_restitution(self.annuel_kwh_m2);
        for x in &mut self.mensuel_kwh {
            *x = arrondir_restitution(*x);
        }
    }
}

/// Postes d'électricité propres au logement collectif et aux parkings, distincts pour que le
/// calcul des indicateurs les intègre selon le texte : éclairage (éq. 2510 : parties communes
/// et parkings), auxiliaires de ventilation (éq. 2511 : parkings), déplacements (éq. 2513 :
/// ascenseurs). Aucun ne produit d'apport interne dans le modèle thermique (`docs/questions.md`,
/// Q50).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct ConsommationsCollectif {
    /// Éclairage des circulations, hall et escalier (§7.2, éq. 792 à 794).
    pub eclairage_parties_communes: PosteElectrique,
    /// Ascenseurs (§10.1, éq. 2310).
    pub ascenseurs: PosteElectrique,
    /// Ventilation des parkings fermés (§10.3, éq. 2346 et §10.3.3.8).
    pub ventilation_parkings: PosteElectrique,
    /// Éclairage des parkings (§10.4, éq. 2353 et 2354).
    pub eclairage_parkings: PosteElectrique,
}

impl ConsommationsCollectif {
    /// Total des quatre postes, en kWh.
    pub fn total_kwh(&self) -> f64 {
        self.eclairage_parties_communes.annuel_kwh
            + self.ascenseurs.annuel_kwh
            + self.ventilation_parkings.annuel_kwh
            + self.eclairage_parkings.annuel_kwh
    }

    pub(crate) fn arrondir(&mut self) {
        self.eclairage_parties_communes.arrondir();
        self.ascenseurs.arrondir();
        self.ventilation_parkings.arrondir();
        self.eclairage_parkings.arrondir();
    }
}

/// Une cabine d'ascenseur (§10.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsAscenseur {
    /// Identifiant de la cabine.
    pub id: Id,
    /// « Ndem » : nombre de démarrages par an (éq. 2283).
    pub nb_demarrages: f64,
    /// « Tmobh » : durée moyenne de déplacement par jour, en h (éq. 2292).
    pub t_mob_h: f64,
    /// « Pti » : puissance de veille cabine immobile, en W.
    pub pti_w: f64,
    /// « Etm » : consommation annuelle en mobilité, en kWh (éq. 2302).
    pub e_tm_kwh: f64,
    /// « Eti » : consommation annuelle en immobilité, en kWh (éq. 2303).
    pub e_ti_kwh: f64,
    /// « Etot », en kWh (éq. 2304).
    pub e_tot_kwh: f64,
    /// Valeurs par défaut du texte appliquées.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// Un parking (§10.3 et §10.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsParking {
    /// Identifiant du parking.
    pub id: Id,
    /// « Event » : consommation annuelle des ventilateurs, en kWh (éq. 2346).
    pub e_vent_kwh: f64,
    /// « Eec » : consommation annuelle de l'éclairage, en kWh (éq. 2354).
    pub e_ecl_kwh: f64,
    /// Valeurs par défaut du texte appliquées.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

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

// Signature imposée par `skip_serializing_if`.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn est_nul(x: &f64) -> bool {
    *x == 0.0
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

    fn arrondir(&mut self) {
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

    fn arrondir(&mut self) {
        self.annuel.arrondir();
        for m in &mut self.mensuel {
            m.arrondir();
        }
    }
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

/// Génération, calcul Th-C (§8.14, §8.17 et §8.29).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsGeneration {
    /// Identifiant de la génération.
    pub id: Id,
    /// Énergie de chauffage demandée par les réseaux, sécurité antigel des doubles flux
    /// comprise, en kWh (éq. 993).
    pub q_ch_kwh: f64,
    /// Énergie de refroidissement demandée (valeur absolue), en kWh (éq. 994).
    pub q_fr_kwh: f64,
    /// Énergie d'ECS demandée, en kWh (éq. 995).
    pub q_ecs_kwh: f64,
    /// Énergie de chauffage fournie par les générateurs, en kWh.
    pub q_fou_ch_kwh: f64,
    /// Énergie de refroidissement fournie, en kWh.
    pub q_fou_fr_kwh: f64,
    /// Énergie d'ECS fournie, en kWh.
    pub q_fou_ecs_kwh: f64,
    /// Demande restant reportée à la fin de l'année, tous postes confondus, en kWh (éq. 1028
    /// et 1034) : nulle si la génération a tout fourni.
    pub q_rep_fin_kwh: f64,
    /// Consommations en énergie finale de la génération, par poste et par type d'énergie
    /// (éq. 1056 et 1057).
    pub consommations: Consommations,
    /// « φvc_tot » : pertes et consommations d'auxiliaires transmises au volume chauffé, en kWh
    /// (éq. 1058), avant application de la part récupérable (fiche 11.1).
    pub pertes_vc_kwh: f64,
    /// Plus longue suite d'heures où la demande de chauffage ou d'ECS n'a pas été couverte,
    /// hors relance (éq. 1059) ; le texte y voit un sous-dimensionnement au-delà de 6 heures
    /// et un sous-dimensionnement critique au-delà de 72 heures (éq. 1060 et 1061).
    pub nbh_sousdim_ch_ecs_max: u32,
    /// De même en refroidissement (éq. 1062 à 1064).
    pub nbh_sousdim_fr_max: u32,
    /// Générateurs de la génération ; la pompe à chaleur d'un chauffe-eau thermodynamique
    /// décrit par son essai y figure sous l'identifiant du chauffe-eau.
    pub generateurs: Vec<ResultatsGenerateur>,
    /// Assemblages avec ballon de stockage d'ECS (§9.14) ; absent sans ballon.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stockages: Vec<ResultatsStockage>,
    /// Productions solaires collectives à appoints décentralisés (CESCI, CESCAI) ; absent sans
    /// production.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub solaires_collectifs: Vec<ResultatsSolaireCollectif>,
}

/// Assemblage avec ballon de stockage d'ECS, calcul Th-C (§9.9, §9.10 et §9.14).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsStockage {
    /// Identifiant de l'assemblage ou du chauffe-eau thermodynamique.
    pub id: Id,
    /// « Qfou,ECS » : énergie fournie par les ballons à la distribution d'ECS, en kWh
    /// (éq. 1954).
    pub q_fou_ecs_kwh: f64,
    /// « Qfou_sto_base » : énergie fournie aux ballons par le générateur de base, en kWh.
    pub q_fou_sto_base_kwh: f64,
    /// « Qfou_sto_ap » : énergie fournie aux ballons par l'appoint intégré, en kWh.
    pub q_fou_sto_ap_kwh: f64,
    /// « Φpertes » : pertes thermiques des ballons, en kWh (éq. 1771).
    pub pertes_kwh: f64,
    /// « Vp » : volume d'eau puisé dans les ballons, en L.
    pub v_puise_l: f64,
    /// Plus basse température de la zone haute en fin de pas de temps, en °C.
    pub theta_haut_min_c: f64,
    /// Plus longue suite d'heures où la zone haute est restée sous la consigne (éq. 1959) ; le
    /// texte juge le ballon incohérent avec le projet au-delà de 168 heures.
    pub nbh_temp_insuff_max: u32,
    /// « nbhreport_e » : nombre d'heures de l'année où l'énergie demandée n'a pas pu être
    /// entièrement puisée dans le ballon (éq. 1780) ; le texte affiche une alerte au-delà de
    /// 24 heures.
    #[serde(default)]
    pub nbh_report_e: u32,
    /// « Vz » : volumes des quatre zones d'un ballon, de bas en haut, en L (un des assemblages
    /// identiques).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub v_zones_l: Vec<f64>,
    /// « θbz » : températures des zones au début du calcul, de bas en haut, en °C.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub theta_zones_debut_c: Vec<f64>,
    /// « θbz » : températures des zones à la fin du calcul, de bas en haut, en °C. Avec
    /// `v_zones_l` et `theta_zones_debut_c`, elles donnent la variation du stock de chaleur des
    /// ballons sur l'année, qui ferme leur bilan.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub theta_zones_fin_c: Vec<f64>,
    /// Boucle solaire d'un chauffe-eau solaire (§9.15), pour l'ensemble des assemblages
    /// identiques.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solaire: Option<ResultatsBoucleSolaire>,
    /// Boucle de chauffage d'un système solaire combiné (fiches 9.18 à 9.20).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssc: Option<ResultatsSsc>,
    /// Paramètres Th-BCE identifiés à partir de l'essai d'un chauffe-eau thermodynamique
    /// (§9.22).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idcet: Option<ResultatsIdCet>,
    /// Valeurs par défaut de la méthode appliquées à l'assemblage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// Production solaire collective à appoints décentralisés (CESCI ou CESCAI, fiches 9.16 et
/// 9.17), calcul Th-C.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsSolaireCollectif {
    /// Identifiant de la production.
    pub id: Id,
    /// « Type_PCAD ».
    pub type_pcad: crate::entree::TypePcad,
    /// Boucle solaire collective (« Esol_tot^BS », « Eef_aux_tot^BS ») ; l'énergie solaire va
    /// aux ballons individuels (CESCI) ou au ballon central (CESCAI).
    pub solaire: ResultatsBoucleSolaire,
    /// Ensembles de ballons individuels, pour tous les ballons de chaque ensemble ; l'énergie
    /// injectée en zone basse (boucle solaire ou réseau intergroupe) est `q_fou_sto_base_kwh`,
    /// celle des appoints intégrés `q_fou_sto_ap_kwh`.
    pub ensembles: Vec<ResultatsStockage>,
    /// Ballon central d'un CESCAI : `q_fou_sto_base_kwh` est l'énergie solaire,
    /// `q_fou_ecs_kwh` l'énergie fournie au réseau intergroupe (ballons individuels et pertes
    /// du réseau déduites).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ballon_central: Option<ResultatsStockage>,
    /// CESCAI : énergie fournie aux ballons individuels par le réseau intergroupe, en kWh
    /// (Σ « Qfou_ens^B », éq. 2078).
    #[serde(default)]
    pub q_fou_reseau_kwh: f64,
    /// CESCAI : pertes du réseau intergroupe déduites du ballon central, en kWh (éq. 2080).
    #[serde(default)]
    pub pertes_reseau_deduites_kwh: f64,
}

impl ResultatsStockage {
    /// Arrondit les valeurs restituées (Q8).
    pub fn arrondir(&mut self) {
        let a = arrondir_restitution;
        let s = self;
        for x in [
            &mut s.q_fou_ecs_kwh,
            &mut s.q_fou_sto_base_kwh,
            &mut s.q_fou_sto_ap_kwh,
            &mut s.pertes_kwh,
            &mut s.v_puise_l,
            &mut s.theta_haut_min_c,
        ] {
            *x = a(*x);
        }
        if let Some(x) = &mut s.ssc {
            x.q_puise_ch_kwh = a(x.q_puise_ch_kwh);
            x.q_fou_ch_appoint_kwh = a(x.q_fou_ch_appoint_kwh);
        }
        if let Some(b) = &mut s.solaire {
            for x in [
                &mut b.e_sol_kwh,
                &mut b.w_pompe_kwh,
                &mut b.pertes_vc_kwh,
                &mut b.isr_kwh_m2,
                &mut b.ecretement_kwh,
            ] {
                *x = a(*x);
            }
            for x in &mut b.e_sol_mensuel_kwh {
                *x = a(*x);
            }
        }
        if let Some(i) = &mut s.idcet {
            // Paramètres identifiés restitués avec quatre décimales.
            for x in [&mut i.ua_s_w_k, &mut i.cop_pivot, &mut i.pabs_pivot_kw] {
                *x = (*x * 1e4).round() / 1e4;
            }
        }
    }
}

impl ResultatsSolaireCollectif {
    /// Arrondit les valeurs restituées (Q8).
    pub fn arrondir(&mut self) {
        let a = arrondir_restitution;
        for x in [
            &mut self.solaire.e_sol_kwh,
            &mut self.solaire.w_pompe_kwh,
            &mut self.solaire.pertes_vc_kwh,
            &mut self.solaire.isr_kwh_m2,
            &mut self.solaire.ecretement_kwh,
            &mut self.q_fou_reseau_kwh,
            &mut self.pertes_reseau_deduites_kwh,
        ] {
            *x = a(*x);
        }
        for x in &mut self.solaire.e_sol_mensuel_kwh {
            *x = a(*x);
        }
        for s in self
            .ensembles
            .iter_mut()
            .chain(self.ballon_central.iter_mut())
        {
            s.arrondir();
        }
    }
}

/// Boucle solaire d'un chauffe-eau solaire, calcul Th-C (§9.15 ; sorties « Esol_tot^BS » et
/// « Eef_aux_tot^BS » des fiches d'assemblage). L'énergie solaire transmise au ballon est
/// comprise dans `q_fou_sto_base_kwh` ; elle n'est pas une consommation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsBoucleSolaire {
    /// « Esol_tot^BS » : énergie solaire transmise aux ballons par les échangeurs, en kWh.
    pub e_sol_kwh: f64,
    /// La même, par mois des scénarios, en kWh.
    pub e_sol_mensuel_kwh: Vec<f64>,
    /// « Eef_aux_tot^BS » : consommation électrique des pompes, en kWh (comprise dans les
    /// consommations d'ECS de la génération, éq. 1972).
    pub w_pompe_kwh: f64,
    /// « ΦvcBS » : pertes des tuyauteries vers l'intérieur du bâtiment, en kWh (éq. 1973).
    pub pertes_vc_kwh: f64,
    /// Ensoleillement annuel sur le plan des capteurs, masques compris, en kWh/m².
    pub isr_kwh_m2: f64,
    /// Superficie d'entrée des capteurs d'un assemblage, en m².
    pub a_capteurs_m2: f64,
    /// Heures de fonctionnement de la pompe.
    pub nbh_marche: u32,
    /// Énergie retirée aux ballons par la limitation à leur température maximale (éq. 1779),
    /// en kWh : part de l'énergie solaire transmise que le ballon ne peut pas stocker (lecture
    /// Q52).
    pub ecretement_kwh: f64,
}

/// Contribution d'un système solaire combiné au chauffage, calcul Th-C.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsSsc {
    /// « Qpuisé_ch_SSC » : chaleur de chauffage puisée dans le ballon, en kWh ; elle est
    /// comprise dans l'énergie fournie en chauffage par la génération.
    pub q_puise_ch_kwh: f64,
    /// Chauffage fourni par l'appoint raccordé à l'assemblage, en kWh (éq. 2150).
    pub q_fou_ch_appoint_kwh: f64,
}

/// Identification d'un chauffe-eau thermodynamique (§9.22.3.4) : entrées du modèle Th-BCE
/// déduites de l'essai, avant correction de statut.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsIdCet {
    /// « UAs,util » : coefficient de pertes du ballon, en W/K.
    pub ua_s_w_k: f64,
    /// « ValCOPpivot » : valeur pivot du COP.
    pub cop_pivot: f64,
    /// « ValPabspivot » : valeur pivot de la puissance absorbée, en kW.
    pub pabs_pivot_kw: f64,
    /// « ε » : écart relatif entre l'essai reproduit par le modèle et l'essai saisi (éq. 2248).
    pub epsilon: f64,
}

/// Générateur, calcul Th-C (§8.29.3.8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsGenerateur {
    /// Identifiant du générateur.
    pub id: Id,
    /// « Eef_fou_ch » : énergie fournie en chauffage, en kWh (éq. 1525).
    pub q_fou_ch_kwh: f64,
    /// « Eef_fou_fr » : énergie fournie en refroidissement, en kWh.
    pub q_fou_fr_kwh: f64,
    /// « Eef_fou_ecs » : énergie fournie en ECS, en kWh.
    pub q_fou_ecs_kwh: f64,
    /// Consommations annuelles en énergie finale, par poste et par type d'énergie (éq. 1494 à
    /// 1509).
    pub consommations: ConsommationsPostes,
    /// « ηeff_ch_an » : efficacité annuelle en chauffage (éq. 1526) — COP annuel d'un
    /// générateur thermodynamique, rendement des autres ; absente sans consommation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta_eff_ch_an: Option<f64>,
    /// « ηeff_fr_an » : efficacité annuelle en refroidissement (EER annuel).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta_eff_fr_an: Option<f64>,
    /// « ηeff_ecs_an » : efficacité annuelle en ECS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta_eff_ecs_an: Option<f64>,
    /// « Cef_aux » : consommation des auxiliaires du générateur, propres et amont, en kWh
    /// d'énergie finale électrique (éq. 1527 et 1528) ; elle est comprise dans `consommations`.
    pub w_aux_kwh: f64,
    /// « φrejet » : énergie échangée par un générateur thermodynamique avec sa source amont, en
    /// kWh (négative : chaleur puisée ; positive : chaleur rejetée en refroidissement).
    #[serde(default, skip_serializing_if = "est_nul")]
    pub phi_rejet_kwh: f64,
    /// Valeurs par défaut de la méthode appliquées au générateur.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
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
    /// Pertes et consommations des systèmes récupérées par le groupe (§11.1), en kWh.
    pub recup_kwh: f64,
    /// Émetteurs d'ECS équivalents du groupe et leurs distributions (§9.5 à 9.7).
    pub ecs: Vec<ResultatsEcsEmetteur>,
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

/// Émetteur d'ECS équivalent, calcul Th-C (§9.5 à 9.7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsEcsEmetteur {
    /// Identifiant.
    pub id: Id,
    /// « Agr,em-e » : surface desservie, en m² (éq. 1674).
    pub a_m2: f64,
    /// « Nadeq-e » : adultes équivalents (éq. 1685 et 1689).
    pub nadeq: f64,
    /// « a » : besoins unitaires, en litres à 40 °C par adulte équivalent et par semaine.
    pub a_l: f64,
    /// « corr-e » : coefficient correctif des émetteurs et des appareils (éq. 1678).
    pub corr: f64,
    /// « Vuw_hebdo_corr » : volume hebdomadaire corrigé, en litres à 40 °C (éq. 1692).
    pub v_uw_hebdo_corr_l: f64,
    /// « Qw » : besoins annuels, en kWh (éq. 1693).
    pub q_w_kwh: f64,
    /// « Qw_bruts » : besoins annuels sans correction des émetteurs, en kWh (éq. 1694).
    pub q_w_bruts_kwh: f64,
    /// « Qw_2nd-e » : besoins augmentés des pertes de la distribution du groupe, en kWh.
    pub q_w_2nd_kwh: f64,
    /// Pertes de la distribution du groupe en volume chauffé, en kWh (récupérables à 60 %).
    pub pertes_vc_kwh: f64,
    /// Pertes de la distribution du groupe hors volume chauffé, en kWh.
    pub pertes_hvc_kwh: f64,
    /// Valeurs par défaut de la méthode appliquées.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// Distribution d'ECS intergroupe, calcul Th-C (§9.8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsEcsIntergroupe {
    /// Identifiant.
    pub id: Id,
    /// Génération qui produit l'ECS.
    pub generation: Id,
    /// « Qw_prim-e » : besoins transmis à la génération, en kWh.
    pub q_w_prim_kwh: f64,
    /// Valeurs mensuelles de `q_w_prim_kwh`, en kWh.
    pub q_w_prim_mensuel_kwh: Vec<f64>,
    /// Pertes en volume chauffé, en kWh.
    pub pertes_vc_kwh: f64,
    /// Pertes hors volume chauffé, en kWh.
    pub pertes_hvc_kwh: f64,
    /// Circulateurs ou traceurs, en kWh d'énergie finale électrique.
    pub w_aux_kwh: f64,
    /// Réchauffeur de boucle, en kWh d'énergie finale électrique.
    pub w_rechauf_kwh: f64,
    /// Température de départ demandée à la génération, en °C.
    pub theta_depart_c: Option<f64>,
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
        self.recup_kwh = a(self.recup_kwh);
        for e in &mut self.ecs {
            for x in [
                &mut e.a_m2,
                &mut e.nadeq,
                &mut e.a_l,
                &mut e.v_uw_hebdo_corr_l,
                &mut e.q_w_kwh,
                &mut e.q_w_bruts_kwh,
                &mut e.q_w_2nd_kwh,
                &mut e.pertes_vc_kwh,
                &mut e.pertes_hvc_kwh,
            ] {
                *x = a(*x);
            }
            e.corr = (e.corr * 1e4).round() / 1e4;
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
    /// Consommations en énergie finale des générations attribuées à la zone : somme de celles
    /// de ses groupes. Absent sans génération.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consommations: Option<Consommations>,
    /// Postes propres au logement collectif et aux parkings attribués à la zone (§7.2, §10.1,
    /// §10.3, §10.4). Absent sans logement collectif ni parking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consommations_collectif: Option<ConsommationsCollectif>,
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
    /// Fonctionnement du système dans le calcul Th-C ; l'énergie de la sécurité antigel y est
    /// demandée à la génération désignée par `generation_antigel`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub th_c: Option<ResultatsVentilationMode>,
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
    /// Consommations en énergie finale des générations attribuées au groupe, par poste et par
    /// type d'énergie, au prorata de ses demandes (§8.29, éq. 1484 à 1486, 1499, 1506 et 1513) ;
    /// matrice « Qcef[poste ; énergie] » des sorties du mode Th-C (§13.2, éq. 2421). Absent sans
    /// génération.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consommations: Option<Consommations>,
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
            if let Some(pv) = &mut b.production_pv {
                pv.arrondir();
            }
            if let Some(c) = &mut b.consommations_collectif {
                c.arrondir();
            }
            if let Some(i) = &mut b.indicateurs {
                i.arrondir();
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
