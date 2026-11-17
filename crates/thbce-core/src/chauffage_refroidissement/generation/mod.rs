// SPDX-License-Identifier: GPL-3.0-or-later
//! Assemblage et gestion/régulation de la génération : fiches `S1_Syst_Assemblage de la
//! génération` (Th-BCE 2020 §8.14) et `C_GEN_Gestion/régulation de la génération` (§8.17).
//!
//! Une génération reçoit, à chaque pas de temps, les demandes de ses réseaux intergroupes de
//! chauffage, de refroidissement et d'ECS (éq. 993 à 995), y ajoute les demandes reportées du
//! pas précédent (éq. 1014 à 1016), fixe la température de fonctionnement de ses générateurs
//! (éq. 1010 à 1013) et répartit la demande entre eux selon le mode de gestion des priorités
//! (§8.17.3.7) : sans priorité, au prorata des puissances nominales (éq. 1017 à 1028), ou en
//! cascade (éq. 1029 à 1034). Le mode en alternance (§8.17.3.7.2.3) est refusé par la
//! validation dans cette version.
//!
//! Chaque générateur répond au même contrat (figure 112) : [`EntreesAppel`] et
//! [`SortiesAppel`]. L'assemblage (§8.14, figure 103) produit la matrice des consommations de la
//! génération (éq. 1056 et 1057), ses pertes vers l'ambiance (éq. 1058) et les demandes
//! reportées au pas suivant (éq. 1028 et 1034). Lectures : `docs/questions/Q046.md`.

use super::calculs_generation::{
    classe_charge, CumulsCef, CumulsGenerateur, Energie, MatriceCef, Poste,
};
use super::sources_amont::SourceAmontGenerateur;
use crate::ecs::assemblage_ballon::{AssemblageBallon, EntreesAssemblage};
use crate::ecs::cesci::{EntreesCollectif, ProductionCollective};
use crate::resultats::ValeurParDefaut;

// Un fichier par étape : modèles des générateurs (un nouveau générateur ajoute sa variante
// dans `generateurs.rs`), résolution de la saisie (`resolution.rs`) et gestion horaire
// (`gestion.rs`).
mod generateurs;
mod gestion;
mod resolution;

pub use generateurs::*;

/// « θamb_ch » : température d'ambiance conventionnelle en chauffage, en °C (tableau 110).
pub const THETA_AMB_CH_C: f64 = 20.0;
/// « θamb_fr » : température d'ambiance conventionnelle en refroidissement, en °C (tableau 110).
pub const THETA_AMB_FR_C: f64 = 26.0;
/// Nombre d'heures successives de sous-dimensionnement au-delà duquel l'alerte « court terme »
/// est levée (éq. 1060 et 1063).
pub const SEUIL_SOUSDIM_COURT_H: u32 = 6;
/// Nombre d'heures successives de sous-dimensionnement au-delà duquel l'alerte « long terme »
/// est levée (éq. 1061 et 1064).
pub const SEUIL_SOUSDIM_LONG_H: u32 = 72;

/// « idfougen » : fonction d'un générateur (§8.17.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FonctionGenerateur {
    /// 1 : chauffage.
    Chauffage,
    /// 2 : refroidissement.
    Refroidissement,
    /// 3 : ECS.
    Ecs,
    /// 4 : chauffage et ECS.
    ChauffageEcs,
    /// 5 : chauffage et refroidissement en alternance.
    ChauffageRefroidissement,
}

impl FonctionGenerateur {
    /// Vrai si le générateur peut assurer le poste.
    pub const fn assure(self, poste: Poste) -> bool {
        matches!(
            (self, poste),
            (
                Self::Chauffage | Self::ChauffageEcs | Self::ChauffageRefroidissement,
                Poste::Chauffage
            ) | (
                Self::Refroidissement | Self::ChauffageRefroidissement,
                Poste::Refroidissement
            ) | (Self::Ecs | Self::ChauffageEcs, Poste::Ecs)
        )
    }

    /// Fonction d'un générateur n'assurant qu'un poste.
    pub const fn mono(poste: Poste) -> Self {
        match poste {
            Poste::Chauffage => Self::Chauffage,
            Poste::Refroidissement => Self::Refroidissement,
            Poste::Ecs => Self::Ecs,
        }
    }
}

/// Entrées de la fonction d'appel d'un générateur (§8.17.3.7.1, figure 112).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntreesAppel {
    /// « idfonction » : poste pour lequel le générateur est appelé.
    pub fonction: Poste,
    /// « Qreq » : demande d'énergie, en Wh (positive pour les trois postes).
    pub q_req_wh: f64,
    /// « θamont(h) » : température de la source amont, en °C (générateurs thermodynamiques).
    pub theta_amont_c: f64,
    /// « θaval(h) » : température de fonctionnement côté aval, en °C.
    pub theta_aval_c: f64,
    /// « Rpui_dispo » : part du pas de temps disponible pour le poste.
    pub r_pui_dispo: f64,
    /// « iecs_seule » : le générateur ne produit que de l'ECS au pas de temps (générateur d'ECS
    /// seule, ou hors saison de chauffage).
    pub ecs_seule: bool,
    /// « θamb(h) » : température d'ambiance de la génération, en °C.
    pub theta_amb_c: f64,
    /// Puissance maximale échangeable avec une source amont sur air extrait, en W, par degré
    /// d'écart entre l'air extrait et sa température limite (`Qmair_extrait × Cpa`, éq. 1454) ;
    /// `None` pour les autres sources.
    pub air_extrait_w_k: Option<f64>,
}

impl EntreesAppel {
    /// Demande simple pour un poste, sans condition de température (générateurs qui n'en
    /// dépendent pas).
    pub fn demande(fonction: Poste, q_req_wh: f64) -> Self {
        Self {
            fonction,
            q_req_wh,
            theta_amont_c: 0.0,
            theta_aval_c: 0.0,
            r_pui_dispo: 1.0,
            ecs_seule: fonction == Poste::Ecs,
            theta_amb_c: THETA_AMB_CH_C,
            air_extrait_w_k: None,
        }
    }
}

/// Sorties de la fonction d'appel d'un générateur (§8.17.3.7.1, figure 112), en Wh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SortiesAppel {
    /// « idengen » : énergie de la case de la matrice qui reçoit la consommation.
    pub energie: Energie,
    /// « Qcef(poste;énergie) » : consommation en énergie finale pour le poste de l'appel.
    pub q_cef_wh: f64,
    /// « Qcons » : énergie consommée par le générateur.
    pub q_cons_wh: f64,
    /// « Qfou » : énergie fournie.
    pub q_fou_wh: f64,
    /// « Qrest » : énergie restant à fournir.
    pub q_rest_wh: f64,
    /// « τcharge » : taux de charge.
    pub tau_charge: f64,
    /// « Φvc » : pertes et auxiliaires transmis à l'ambiance.
    pub phi_vc_wh: f64,
    /// « Waux,pro » : consommation des auxiliaires propres au générateur.
    pub w_aux_pro_wh: f64,
    /// « ηeff » : rendement, COP ou EER du pas de temps.
    pub eta_eff: f64,
    /// « φrejet » : énergie échangée avec la source amont (négative en chauffage et en ECS,
    /// positive en refroidissement).
    pub phi_rejet_wh: f64,
    /// « Qprelec » : électricité produite (cogénération, hors première version).
    pub q_prelec_wh: f64,
    /// « Rfonct_ecs » : part du pas de temps employée à pleine charge pour l'ECS.
    pub r_fonct_ecs: f64,
    /// Électricité des auxiliaires propres à ajouter à la case électricité du poste de
    /// l'appel (générateurs à combustion : `Waux,pro`, éq. 1117) ; nulle pour un générateur
    /// dont la consommation principale est déjà de l'électricité.
    pub q_cef_aux_elec_wh: f64,
}

impl Default for SortiesAppel {
    fn default() -> Self {
        Self {
            energie: Energie::Electricite,
            q_cef_wh: 0.0,
            q_cons_wh: 0.0,
            q_fou_wh: 0.0,
            q_rest_wh: 0.0,
            tau_charge: 0.0,
            phi_vc_wh: 0.0,
            w_aux_pro_wh: 0.0,
            eta_eff: 0.0,
            phi_rejet_wh: 0.0,
            q_prelec_wh: 0.0,
            r_fonct_ecs: 0.0,
            q_cef_aux_elec_wh: 0.0,
        }
    }
}

/// Jeu de données décrivant le fonctionnement d'un générateur à un pas de temps (figure 111) ;
/// remis à zéro au début de chaque pas de temps.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct JeuDonnees {
    /// « Qcef(po;en)^gnr(h) », en Wh.
    pub q_cef: MatriceCef,
    /// « θamont^gnr(h) », en °C.
    pub theta_amont_c: f64,
    /// « ηeff_ch », « ηeff_fr », « ηeff_ecs ».
    pub eta_eff: [f64; 3],
    /// « Qcons^gnr(h) », en Wh.
    pub q_cons_wh: f64,
    /// « φrejet^gnr(h) », en Wh.
    pub phi_rejet_wh: f64,
    /// « Qfou_ch », « Qfou_fr », « Qfou_ecs », en Wh.
    pub q_fou_wh: [f64; 3],
    /// « φvc^gnr(h) », en Wh.
    pub phi_vc_wh: f64,
    /// « Waux,pro^gnr(h) », en Wh.
    pub w_aux_pro_wh: f64,
    /// « Waux^gnr(h) » : auxiliaires propres et amont, en Wh.
    pub w_aux_wh: f64,
    /// « ia_refroidi^gnr(h) ».
    pub ia_refroidi: bool,
    /// « Rfonct_ecs^gnr(h) ».
    pub r_fonct_ecs: f64,
    /// « τcharge^gnr(h) ».
    pub tau_charge: f64,
    /// « Qprelec^gnr(h) », en Wh.
    pub q_prelec_wh: f64,
    /// Taux de charge du dernier appel pour chaque poste (sorties complémentaires de la fiche
    /// 8.29, éq. 1522 à 1524).
    pub tau_charge_poste: [f64; 3],
    /// Vrai si le générateur a été appelé pour le poste.
    pub appele: [bool; 3],
}

impl JeuDonnees {
    /// Mise à jour du jeu de données après un appel (§8.17.3.7.2, « += »).
    pub fn ajouter(&mut self, poste: Poste, r_pui_dispo: f64, s: &SortiesAppel) {
        let p = poste.indice();
        self.q_cef.ajouter(poste, s.energie, s.q_cef_wh);
        if s.q_cef_aux_elec_wh != 0.0 {
            self.q_cef
                .ajouter(poste, Energie::Electricite, s.q_cef_aux_elec_wh);
        }
        self.q_cons_wh += s.q_cons_wh;
        self.q_fou_wh[p] += s.q_fou_wh;
        self.q_prelec_wh += s.q_prelec_wh;
        self.tau_charge += r_pui_dispo * s.tau_charge;
        self.phi_rejet_wh += s.phi_rejet_wh;
        self.phi_vc_wh += s.phi_vc_wh;
        self.w_aux_pro_wh += s.w_aux_pro_wh;
        self.w_aux_wh += s.w_aux_pro_wh;
        self.eta_eff[p] += s.eta_eff;
        self.r_fonct_ecs += s.r_fonct_ecs;
        self.tau_charge_poste[p] = s.tau_charge;
        self.appele[p] = true;
    }
}

/// Générateur d'une génération, avec ses paramètres d'intégration et ses cumuls.
#[derive(Debug, Clone, PartialEq)]
pub struct GenerateurHoraire {
    /// Identifiant.
    pub id: String,
    /// « idpriorite_ch », « idpriorite_fr », « idpriorite_ecs » : rang dans la cascade.
    pub priorite: [u32; 3],
    /// Modèle du générateur.
    pub modele: Generateur,
    /// Source amont d'un générateur thermodynamique (fiche 8.26).
    pub source: Option<SourceAmontGenerateur>,
    /// Valeurs par défaut de la méthode appliquées au générateur.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
    /// Jeu de données du pas de temps.
    pub jeu: JeuDonnees,
    /// Cumuls annuels.
    pub cumuls: CumulsGenerateur,
}

impl GenerateurHoraire {
    /// Générateur de priorité 1 pour tous les postes.
    pub fn nouveau(id: impl Into<String>, modele: Generateur) -> Self {
        Self {
            id: id.into(),
            priorite: [1; 3],
            modele,
            source: None,
            valeurs_par_defaut: Vec::new(),
            jeu: JeuDonnees::default(),
            cumuls: CumulsGenerateur::default(),
        }
    }
}

/// « idtype_priorite » : mode de gestion des priorités des générateurs instantanés (§8.17.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModePriorite {
    /// 1 : sans priorité, demande répartie au prorata des puissances nominales.
    SansPriorite,
    /// 2 : en cascade, dans l'ordre des priorités.
    Cascade,
}

/// « idgestion_ch », « idgestion_fr » : gestion de la température de fonctionnement d'une
/// génération qui alimente des réseaux hydrauliques (§8.17.3.5.10.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GestionTemperature {
    /// 1 : température moyenne constante, « θwm », en °C.
    Constante(f64),
    /// 2 : température moyenne des réseaux de distribution.
    Reseaux,
}

/// Réseau intergroupes vu par la génération à un pas de temps (§8.14.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReseauVu {
    /// « idfonction » : chauffage, refroidissement ou ECS.
    pub poste: Poste,
    /// « idtype » : vrai pour un réseau hydraulique, faux pour un réseau fictif.
    pub hydraulique: bool,
    /// « Qsys(h) » : demande du réseau, en Wh, comptée positivement (éq. 993 à 995).
    pub q_sys_wh: f64,
    /// « θmoy(h) » : température moyenne du réseau, en °C.
    pub theta_moy_c: f64,
    /// « θdep(h) » : température de départ demandée par le réseau, en °C.
    pub theta_dep_c: f64,
    /// « θi,aval,eq(h) » : température d'air vue par un réseau fictif, en °C.
    pub theta_i_aval_eq_c: f64,
    /// « idrelance(h) ».
    pub relance: bool,
    /// « Ratsurf_dess » : part de la surface desservie par la génération pour ce poste (éq. 990
    /// à 992).
    pub rat_surf_dess: f64,
    /// « φpertes_vc_prim-e + φpertes_hvc_prim-e » : pertes d'un réseau d'ECS intergroupe
    /// comprises dans `q_sys_wh`, en Wh (nulles sinon) ; déduites du ballon central d'un
    /// CESCAI (éq. 2076).
    pub phi_pertes_prim_wh: f64,
}

/// Entrées d'une génération à un pas de temps.
#[derive(Debug, Clone, PartialEq)]
pub struct EntreesGeneration<'a> {
    /// Réseaux intergroupes reliés à la génération.
    pub reseaux: &'a [ReseauVu],
    /// « Autch^gen(j) » (fiche 8.4).
    pub aut_ch: bool,
    /// « Autfr^gen(j) ».
    pub aut_fr: bool,
    /// « idencl^gen(j) » : période de fonctionnement en ECS (éq. 1007).
    pub id_encl: bool,
    /// « θext(h) », en °C.
    pub theta_ext_c: f64,
    /// « btherm(h) » de l'emplacement d'une génération hors volume chauffé.
    pub b_therm: f64,
    /// « Σ Wrechauffe^dp-e(h) » : réchauffeurs des boucles d'ECS reliées, en Wh (éq. 1057).
    pub w_rechauf_wh: f64,
    /// « θcw(h) » : température de l'eau froide, en °C (ballons de stockage).
    pub theta_cw_c: f64,
    /// « hleg » : heure légale du pas de temps (programmation des ballons).
    pub hleg: u8,
    /// Conditions de source amont de chaque générateur, dans l'ordre des générateurs ; vide si
    /// aucun n'en dépend.
    pub sources: &'a [ConditionsSource],
    /// Climat du calcul, pour l'ensoleillement des capteurs des boucles solaires (fiche 3.2) ;
    /// `None` : ensoleillement nul.
    pub climat: Option<&'a crate::climat::ClimatExterieur>,
    /// Indice du pas de temps dans l'année, de 0 à 8 759.
    pub h: usize,
}

/// Conditions de la source amont d'un générateur thermodynamique à un pas de temps (fiche
/// 8.26).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ConditionsSource {
    /// « θamont(h) », en °C (éq. 1405 à 1407).
    pub theta_amont_c: f64,
    /// `Qmair_extrait × Cpa`, en W/K, pour une source sur air extrait (éq. 1454).
    pub air_extrait_w_k: Option<f64>,
}

/// Sorties d'une génération à un pas de temps.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SortiesGeneration {
    /// « Qcef(po;en)^gen(h) » : matrice des consommations, en Wh (éq. 1056 et 1057).
    pub q_cef: MatriceCef,
    /// « φvc_tot » : pertes et auxiliaires transmis à l'ambiance, en Wh (éq. 1058) ; nul pour
    /// une génération hors volume chauffé.
    pub phi_vc_tot_wh: f64,
    /// « Qreq_tot_sr » : demandes du pas de temps, sans report, par poste, en Wh.
    pub q_req_tot_sr_wh: [f64; 3],
    /// « Qreq_tot_ar » : demandes avec report, par poste, en Wh.
    pub q_req_tot_ar_wh: [f64; 3],
    /// « Qfou » : énergie fournie par poste, tous générateurs confondus, en Wh.
    pub q_fou_wh: [f64; 3],
    /// « Qrep(h) » : demandes reportées au pas suivant, par poste, en Wh.
    pub q_rep_wh: [f64; 3],
    /// « Ratbes^gen,dp(h) » de chaque réseau, dans l'ordre des entrées (éq. 996 à 998).
    pub rat_bes_dp: Vec<f64>,
    /// « θaval_ch », « θaval_fr », « θaval_ecs », en °C.
    pub theta_aval_c: [f64; 3],
    /// « θamb(h) », en °C.
    pub theta_amb_c: f64,
    /// « idrelance_ch(h) » et « idrelance_fr(h) » (éq. 1005 et 1006).
    pub relance: [bool; 2],
}

/// État d'une génération d'un pas de temps au suivant.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EtatGeneration {
    /// « Qrep(h − 1) » par poste, en Wh.
    pub q_rep_wh: [f64; 3],
    /// « θaval_ch(h − 1) » et « θaval_fr(h − 1) », en °C.
    pub theta_aval_prec_c: [f64; 2],
    /// « Nbsousdim_ch(h) » et « Nbsousdim_fr(h) ».
    pub nb_sousdim: [u32; 2],
}

/// Cumuls annuels d'une génération.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CumulsGeneration {
    /// Consommations en énergie finale, par mois (éq. 1497, 1504 et 1511).
    pub cef: CumulsCef,
    /// Demandes des réseaux, sans report, par poste et par mois, en Wh.
    pub q_req_mensuel_wh: [[f64; 12]; 3],
    /// Énergie fournie par poste, en Wh.
    pub q_fou_wh: [f64; 3],
    /// « φvc_tot », en Wh.
    pub phi_vc_tot_wh: f64,
    /// Pertes des ballons de stockage, en Wh.
    pub phi_pertes_sto_wh: f64,
    /// Plus longue suite d'heures de sous-dimensionnement, en chauffage ou ECS puis en
    /// refroidissement.
    pub nb_sousdim_max: [u32; 2],
}

/// Génération, prête pour le calcul horaire.
#[derive(Debug, Clone, PartialEq)]
pub struct GenerationHoraire {
    /// Identifiant.
    pub id: String,
    /// « idpos » : vrai si la génération est en volume chauffé.
    pub en_volume_chauffe: bool,
    /// « idtype_priorite ».
    pub priorite: ModePriorite,
    /// « idgestion_ch » et « θwm_ch ».
    pub gestion_ch: GestionTemperature,
    /// « idgestion_fr » et « θwm_fr ».
    pub gestion_fr: GestionTemperature,
    /// « θdist_ch_max » : température maximale des réseaux hydrauliques de chauffage, en °C
    /// (éq. 980).
    pub theta_dist_ch_max_c: Option<f64>,
    /// « θdist_fr_min » : température minimale des réseaux hydrauliques de refroidissement.
    pub theta_dist_fr_min_c: Option<f64>,
    /// Générateurs.
    pub generateurs: Vec<GenerateurHoraire>,
    /// Assemblages avec ballon de stockage d'ECS (§9.14), premiers sollicités en ECS.
    pub stockages: Vec<AssemblageBallon>,
    /// Productions solaires collectives à appoints décentralisés (CESCI et CESCAI, fiches
    /// 9.16 et 9.17), premières sollicitées en ECS.
    pub collectifs: Vec<ProductionCollective>,
    /// État.
    pub etat: EtatGeneration,
    /// Cumuls annuels.
    pub cumuls: CumulsGeneration,
    /// Sorties du dernier pas de temps.
    pub sorties: SortiesGeneration,
}

impl GenerationHoraire {
    /// Génération en volume chauffé, sans priorité, à la température de ses réseaux.
    pub fn nouvelle(id: impl Into<String>, generateurs: Vec<GenerateurHoraire>) -> Self {
        Self {
            id: id.into(),
            en_volume_chauffe: true,
            priorite: ModePriorite::SansPriorite,
            gestion_ch: GestionTemperature::Reseaux,
            gestion_fr: GestionTemperature::Reseaux,
            theta_dist_ch_max_c: None,
            theta_dist_fr_min_c: None,
            generateurs,
            stockages: Vec::new(),
            collectifs: Vec::new(),
            etat: EtatGeneration::default(),
            cumuls: CumulsGeneration::default(),
            sorties: SortiesGeneration::default(),
        }
    }

    /// Génération idéale : un générateur idéal par poste (assemblage provisoire des lots 6a et
    /// 6b, conservé pour les tests).
    pub fn ideale(id: impl Into<String>) -> Self {
        let generateurs = Poste::TOUS
            .iter()
            .map(|p| {
                GenerateurHoraire::nouveau(
                    format!("ideal-{}", p.indice() + 1),
                    Generateur::Ideal(GenerateurIdeal {
                        fonction: FonctionGenerateur::mono(*p),
                    }),
                )
            })
            .collect();
        Self::nouvelle(id, generateurs)
    }

    /// « θcentr_b4(h − 1) » : température de la zone haute du ballon central d'un CESCAI, qui
    /// fixe le départ du réseau d'ECS intergroupe (§9.17.3.2, deuxième étape) ; `None` sans
    /// CESCAI.
    pub fn theta_depart_reseau_ecs_c(&self) -> Option<f64> {
        self.collectifs
            .iter()
            .find_map(|c| c.central.as_ref())
            .map(|b| b.ballon.theta_c[crate::ecs::ballon_de_stockage::NB_ZONES - 1])
    }

    /// « θamb(h) » : température d'ambiance de la génération (§8.17.3.5.8).
    pub fn theta_amb_c(&self, aut_ch: bool, aut_fr: bool, theta_ext_c: f64, b_therm: f64) -> f64 {
        let conv = match (aut_ch, aut_fr) {
            (true, false) => THETA_AMB_CH_C,
            (false, true) => THETA_AMB_FR_C,
            _ => (THETA_AMB_CH_C + THETA_AMB_FR_C) / 2.0,
        };
        if self.en_volume_chauffe {
            conv
        } else {
            crate::bati::espace_tampon::espace_tampon::theta_ambiance_hors_volume_chauffe_c(
                b_therm,
                theta_ext_c,
                conv,
            )
        }
    }

    /// Indices des générateurs assurant un poste, dans l'ordre de leurs priorités (ordre de la
    /// saisie à priorité égale).
    fn generateurs_du_poste(&self, poste: Poste) -> Vec<usize> {
        // En ECS, les générateurs de base et d'appoint d'un ballon sont pilotés par leur
        // assemblage (§8.17.3.4) : ils ne comptent pas parmi les générateurs instantanés.
        let pilote = |i: usize| {
            let collectif = poste == Poste::Ecs
                && self.collectifs.iter().any(|c| {
                    c.ensembles
                        .iter()
                        .any(|a| a.appoint.is_some_and(|x| x.generateur == i))
                });
            collectif
                || self.stockages.iter().any(|s| {
                    let du_ballon =
                        s.base.generateur == i || s.appoint.is_some_and(|a| a.generateur == i);
                    match poste {
                        Poste::Ecs => du_ballon,
                        // L'appoint raccordé d'un système solaire combiné assure aussi le chauffage
                        // pour le compte de l'assemblage (éq. 2150).
                        Poste::Chauffage => {
                            s.ssc.is_some_and(|p| p.raccorde)
                                && s.appoint.is_some_and(|a| a.generateur == i)
                        }
                        Poste::Refroidissement => false,
                    }
                })
        };
        let mut v: Vec<usize> = (0..self.generateurs.len())
            .filter(|&i| self.generateurs[i].modele.fonction().assure(poste) && !pilote(i))
            .collect();
        if self.priorite == ModePriorite::Cascade {
            v.sort_by_key(|&i| self.generateurs[i].priorite[poste.indice()]);
        }
        v
    }

    /// « idsousdim_court » et « idsousdim_long » en chauffage et ECS (indice 0) ou en
    /// refroidissement (indice 1) : alertes de sous-dimensionnement (éq. 1060, 1061, 1063 et
    /// 1064).
    pub fn sous_dimensionnement(&self, k: usize) -> (bool, bool) {
        let n = self.cumuls.nb_sousdim_max[k];
        (n > SEUIL_SOUSDIM_COURT_H, n > SEUIL_SOUSDIM_LONG_H)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests;
