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
//! reportées au pas suivant (éq. 1028 et 1034). Lectures : `docs/questions.md`, Q46.

use super::calculs_generation::{
    classe_charge, CumulsCef, CumulsGenerateur, Energie, MatriceCef, Poste,
};
use super::generateur_direct_a_effet_joule::GenerateurEffetJoule;

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
        }
    }
}

/// Générateur idéal : fournit exactement la demande, sans limite de puissance, avec un
/// rendement unitaire, en consommant de l'électricité.
///
/// Générateur de test, absent du format d'entrée : il a tenu lieu de génération dans les lots
/// 6a et 6b, avant la description des générateurs, et sert aux tests de la gestion de la
/// génération. Le texte ne prévoit pas de générateur par défaut.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenerateurIdeal {
    /// Fonction du générateur.
    pub fonction: FonctionGenerateur,
}

impl GenerateurIdeal {
    fn appel(&self, e: &EntreesAppel) -> SortiesAppel {
        let q = e.q_req_wh.max(0.0);
        SortiesAppel {
            q_cef_wh: q,
            q_cons_wh: q,
            q_fou_wh: q,
            eta_eff: 1.0,
            tau_charge: 0.0,
            ..SortiesAppel::default()
        }
    }
}

/// Modèle d'un générateur (tableau 111).
#[derive(Debug, Clone, PartialEq)]
pub enum Generateur {
    /// Type 500 : générateur direct à effet Joule (§8.18).
    EffetJoule(GenerateurEffetJoule),
    /// Générateur idéal, pour les tests.
    Ideal(GenerateurIdeal),
}

impl Generateur {
    /// « idfougen ».
    pub fn fonction(&self) -> FonctionGenerateur {
        match self {
            Self::EffetJoule(g) => FonctionGenerateur::mono(g.fonction),
            Self::Ideal(g) => g.fonction,
        }
    }

    /// « Rdim × Pngen_[po.] » : puissance nominale installée pour un poste, en W (éq. 1017).
    pub fn pn_gen_tot_w(&self, poste: Poste) -> f64 {
        if !self.fonction().assure(poste) {
            return 0.0;
        }
        match self {
            Self::EffetJoule(g) => g.p_max_w(),
            // Sans puissance nominale : parts égales en mode sans priorité.
            Self::Ideal(_) => 1.0,
        }
    }

    /// Fonction d'appel du générateur (figure 112).
    pub fn appel(&mut self, e: &EntreesAppel) -> SortiesAppel {
        match self {
            Self::EffetJoule(g) => g.appel(e),
            Self::Ideal(g) => g.appel(e),
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
    /// « θi,aval,eq(h) » : température d'air vue par un réseau fictif, en °C.
    pub theta_i_aval_eq_c: f64,
    /// « idrelance(h) ».
    pub relance: bool,
    /// « Ratsurf_dess » : part de la surface desservie par la génération pour ce poste (éq. 990
    /// à 992).
    pub rat_surf_dess: f64,
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
    /// Conditions de source amont de chaque générateur, dans l'ordre des générateurs ; vide si
    /// aucun n'en dépend.
    pub sources: &'a [ConditionsSource],
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

    /// Résout une génération saisie et validée.
    pub fn resoudre(g: &crate::entree::Generation) -> Self {
        use crate::entree::{
            FonctionEffetJoule, GestionTemperatureGeneration, NatureGenerateur, PositionGeneration,
            TypePriorite,
        };
        let generateurs = g
            .generateurs
            .iter()
            .filter_map(|s| {
                let rdim = f64::from(s.rdim.max(1));
                let modele = match &s.nature {
                    NatureGenerateur::EffetJoule(j) => {
                        Generateur::EffetJoule(GenerateurEffetJoule {
                            fonction: match j.fonction {
                                FonctionEffetJoule::Chauffage => Poste::Chauffage,
                                FonctionEffetJoule::Ecs => Poste::Ecs,
                            },
                            pn_gen_w: j.pn_gen_w,
                            rdim,
                        })
                    }
                    // Natures refusées par la validation.
                    _ => return None,
                };
                Some(GenerateurHoraire {
                    id: s.id.clone(),
                    priorite: [
                        s.priorite_chauffage.unwrap_or(1),
                        s.priorite_refroidissement.unwrap_or(1),
                        s.priorite_ecs.unwrap_or(1),
                    ],
                    modele,
                    jeu: JeuDonnees::default(),
                    cumuls: CumulsGenerateur::default(),
                })
            })
            .collect();
        let gestion = |x: Option<GestionTemperatureGeneration>| match x {
            Some(GestionTemperatureGeneration::Constante { theta_wm_c }) => {
                GestionTemperature::Constante(theta_wm_c)
            }
            Some(GestionTemperatureGeneration::Reseaux) | None => GestionTemperature::Reseaux,
        };
        Self {
            en_volume_chauffe: g.position == PositionGeneration::EnVolumeChauffe,
            priorite: match g.priorite {
                Some(TypePriorite::Cascade) => ModePriorite::Cascade,
                _ => ModePriorite::SansPriorite,
            },
            gestion_ch: gestion(g.gestion_temperature_chauffage),
            gestion_fr: gestion(g.gestion_temperature_refroidissement),
            ..Self::nouvelle(g.id.clone(), generateurs)
        }
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
            b_therm * theta_ext_c + (1.0 - b_therm) * conv
        }
    }

    /// Indices des générateurs assurant un poste, dans l'ordre de leurs priorités (ordre de la
    /// saisie à priorité égale).
    fn generateurs_du_poste(&self, poste: Poste) -> Vec<usize> {
        let mut v: Vec<usize> = (0..self.generateurs.len())
            .filter(|&i| self.generateurs[i].modele.fonction().assure(poste))
            .collect();
        if self.priorite == ModePriorite::Cascade {
            v.sort_by_key(|&i| self.generateurs[i].priorite[poste.indice()]);
        }
        v
    }

    /// Calcule un pas de temps de la génération ; `mois` est l'indice du mois des scénarios, de
    /// 0 à 11, pour les cumuls.
    pub fn pas(&mut self, e: &EntreesGeneration<'_>, mois: usize) -> &SortiesGeneration {
        // §8.17.3.5.1 : jeux de données remis à zéro.
        for (i, g) in self.generateurs.iter_mut().enumerate() {
            g.jeu = JeuDonnees::default();
            if let Some(s) = e.sources.get(i) {
                g.jeu.theta_amont_c = s.theta_amont_c;
            }
        }
        // Éq. (993) à (995) : demandes totales sans report.
        let mut q_sr = [0.0; 3];
        for r in e.reseaux {
            q_sr[r.poste.indice()] += r.q_sys_wh;
        }
        // Éq. (996) à (998) : part de chaque réseau dans la demande de son poste ; à défaut de
        // demande, sa part de la surface desservie.
        let rat_bes_dp: Vec<f64> = e
            .reseaux
            .iter()
            .map(|r| {
                let total = q_sr[r.poste.indice()];
                if total > 0.0 {
                    r.q_sys_wh / total
                } else {
                    r.rat_surf_dess
                }
            })
            .collect();
        // Éq. (1005) et (1006).
        let relance = [Poste::Chauffage, Poste::Refroidissement]
            .map(|p| e.reseaux.iter().any(|r| r.poste == p && r.relance));
        let theta_amb = self.theta_amb_c(e.aut_ch, e.aut_fr, e.theta_ext_c, e.b_therm);
        // Éq. (1014) à (1016) : demandes avec report.
        let q_ar = [0, 1, 2].map(|p| q_sr[p] + self.etat.q_rep_wh[p]);
        // Éq. (1009) à (1013) : températures de fonctionnement.
        let theta_aval = [
            self.theta_aval(
                Poste::Chauffage,
                e,
                &rat_bes_dp,
                relance[0],
                q_sr[0],
                q_ar[0],
            ),
            self.theta_aval(
                Poste::Refroidissement,
                e,
                &rat_bes_dp,
                relance[1],
                q_sr[1],
                q_ar[1],
            ),
            // Éq. (1009) : en ECS instantanée, les générateurs de la première version (effet
            // Joule) ne dépendent pas de la température de fonctionnement ; celle du départ le
            // plus chaud des réseaux est transmise.
            e.reseaux
                .iter()
                .filter(|r| r.poste == Poste::Ecs)
                .map(|r| r.theta_moy_c)
                .fold(0.0, f64::max),
        ];

        // §8.17.3.7 : algorithme principal pour les générateurs instantanés, dans l'ordre des
        // étapes du texte : ECS, refroidissement, chauffage (lecture Q46).
        let mut q_rep = [0.0; 3];
        for poste in [Poste::Ecs, Poste::Refroidissement, Poste::Chauffage] {
            let p = poste.indice();
            let actif = match poste {
                Poste::Ecs => e.id_encl,
                Poste::Refroidissement => e.aut_fr,
                Poste::Chauffage => e.aut_ch,
            };
            let ctx = ContexteAppel {
                poste,
                theta_aval_c: theta_aval[p],
                theta_amb_c: theta_amb,
                aut_ch: e.aut_ch,
                relance_ch: relance[0],
            };
            q_rep[p] = if actif {
                self.appeler_generateurs(&ctx, q_ar[p], e.sources)
            } else {
                // Hors période d'activation, les générateurs sont désactivés pour le poste : la
                // demande éventuelle reste reportée (éq. 1028).
                q_ar[p]
            };
        }

        // §8.17.3.9, éq. (1056) à (1058) : matrice et pertes de la génération.
        let mut q_cef = MatriceCef::NULLE;
        let mut phi_vc = 0.0;
        let mut q_fou = [0.0; 3];
        for g in &self.generateurs {
            q_cef.ajouter_matrice(&g.jeu.q_cef);
            phi_vc += g.jeu.phi_vc_wh;
            for (x, y) in q_fou.iter_mut().zip(g.jeu.q_fou_wh) {
                *x += y;
            }
        }
        q_cef.ajouter(Poste::Ecs, Energie::Electricite, e.w_rechauf_wh);
        if !self.en_volume_chauffe {
            phi_vc = 0.0;
        }
        // Éq. (1059) à (1064) : sous-dimensionnement.
        let n = &mut self.etat.nb_sousdim;
        n[0] = if (q_rep[0] > 0.0 || q_rep[2] > 0.0) && !relance[0] {
            n[0] + 1
        } else {
            0
        };
        n[1] = if q_rep[1] > 0.0 { n[1] + 1 } else { 0 };
        for (m, x) in self.cumuls.nb_sousdim_max.iter_mut().zip(n.iter()) {
            *m = (*m).max(*x);
        }
        self.etat.q_rep_wh = q_rep;
        self.etat.theta_aval_prec_c = [theta_aval[0], theta_aval[1]];

        // Fiche 8.29 : cumuls de la génération et des générateurs.
        self.cumuls.cef.ajouter(mois, &q_cef);
        self.cumuls.phi_vc_tot_wh += phi_vc;
        for p in 0..3 {
            self.cumuls.q_req_mensuel_wh[p][mois] += q_sr[p];
            self.cumuls.q_fou_wh[p] += q_fou[p];
        }
        let actifs = [e.aut_ch, e.aut_fr, e.id_encl];
        for g in &mut self.generateurs {
            g.cumuls.cef.ajouter(mois, &g.jeu.q_cef);
            g.cumuls.w_aux_wh += g.jeu.w_aux_wh;
            g.cumuls.phi_vc_wh += g.jeu.phi_vc_wh;
            g.cumuls.phi_rejet_wh += g.jeu.phi_rejet_wh;
            let f = g.modele.fonction();
            for poste in Poste::TOUS {
                let p = poste.indice();
                g.cumuls.q_fou_wh[p] += g.jeu.q_fou_wh[p];
                if !f.assure(poste) {
                    continue;
                }
                // Éq. (1522) à (1524).
                if actifs[p] {
                    g.cumuls.nbh_charge[p][classe_charge(g.jeu.tau_charge_poste[p])] += 1;
                } else {
                    g.cumuls.nbh_hors_fonctionnement[p] += 1;
                }
            }
        }
        self.sorties = SortiesGeneration {
            q_cef,
            phi_vc_tot_wh: phi_vc,
            q_req_tot_sr_wh: q_sr,
            q_req_tot_ar_wh: q_ar,
            q_fou_wh: q_fou,
            q_rep_wh: q_rep,
            rat_bes_dp,
            theta_aval_c: theta_aval,
            theta_amb_c: theta_amb,
            relance,
        };
        &self.sorties
    }

    /// Température de fonctionnement en chauffage ou en refroidissement (§8.17.3.5.10).
    fn theta_aval(
        &self,
        poste: Poste,
        e: &EntreesGeneration<'_>,
        rat_bes_dp: &[f64],
        relance: bool,
        q_sr: f64,
        q_ar: f64,
    ) -> f64 {
        let froid = poste == Poste::Refroidissement;
        let du_poste = || {
            e.reseaux
                .iter()
                .zip(rat_bes_dp)
                .filter(move |(r, _)| r.poste == poste)
        };
        if du_poste().any(|(r, _)| r.hydraulique) {
            // Éq. (1010) et (1011) : réseaux hydrauliques.
            let (gestion, extreme, prec) = if froid {
                (
                    self.gestion_fr,
                    self.theta_dist_fr_min_c,
                    self.etat.theta_aval_prec_c[1],
                )
            } else {
                (
                    self.gestion_ch,
                    self.theta_dist_ch_max_c,
                    self.etat.theta_aval_prec_c[0],
                )
            };
            let des_reseaux = || {
                let t = du_poste()
                    .filter(|(r, _)| r.hydraulique)
                    .map(|(r, _)| r.theta_moy_c);
                // Lecture Q46 : le réseau le plus exigeant, le plus chaud en chauffage et le
                // plus froid en refroidissement.
                if froid {
                    t.reduce(f64::min)
                } else {
                    t.reduce(f64::max)
                }
                .unwrap_or(prec)
            };
            match gestion {
                GestionTemperature::Constante(t) => t,
                GestionTemperature::Reseaux => {
                    if relance {
                        // Relance : fonctionnement à la température extrême des réseaux.
                        extreme.unwrap_or_else(des_reseaux)
                    } else if q_sr == 0.0 && q_ar > 0.0 {
                        // Demande reportée seule : température du pas précédent.
                        prec
                    } else {
                        des_reseaux()
                    }
                }
            }
        } else {
            // Éq. (1012) et (1013) : génération sur air ambiant.
            du_poste().map(|(r, rat)| rat * r.theta_i_aval_eq_c).sum()
        }
    }

    /// Appelle les générateurs d'un poste selon le mode de gestion des priorités et renvoie
    /// l'énergie reportée « Qrep » (éq. 1028 et 1034).
    fn appeler_generateurs(
        &mut self,
        ctx: &ContexteAppel,
        q_req_tot_ar_wh: f64,
        sources: &[ConditionsSource],
    ) -> f64 {
        let poste = ctx.poste;
        let mut indices = self.generateurs_du_poste(poste);
        if indices.is_empty() {
            return q_req_tot_ar_wh;
        }
        // §8.17.3.5.6 : en cascade, pendant une relance de chauffage, le générateur de rang le
        // plus élevé n'est pas pris en compte (lecture Q46 : s'il y en a au moins deux).
        if poste == Poste::Chauffage
            && self.priorite == ModePriorite::Cascade
            && ctx.relance_ch
            && indices.len() > 1
        {
            indices.pop();
        }
        match self.priorite {
            ModePriorite::SansPriorite => {
                // Éq. (1017) et (1018) : clés de répartition ; éq. (1020), (1023) et (1026) :
                // les générateurs qui ont refroidi sont écartés du chauffage et de l'ECS.
                let cle = |g: &GenerateurHoraire| {
                    let dispo = poste == Poste::Refroidissement || !g.jeu.ia_refroidi;
                    if dispo {
                        g.modele.pn_gen_tot_w(poste)
                    } else {
                        0.0
                    }
                };
                let total: f64 = indices.iter().map(|&i| cle(&self.generateurs[i])).sum();
                let mut fourni = 0.0;
                for &i in &indices {
                    let q_req = if total > 0.0 {
                        cle(&self.generateurs[i]) / total * q_req_tot_ar_wh
                    } else {
                        0.0
                    };
                    if let Some(s) = self.appeler(i, ctx, q_req, sources) {
                        fourni += s.q_fou_wh;
                    }
                }
                // Éq. (1028).
                q_req_tot_ar_wh - fourni
            }
            ModePriorite::Cascade => {
                // Éq. (1029) à (1034).
                let mut q_req = q_req_tot_ar_wh;
                for &i in &indices {
                    if let Some(s) = self.appeler(i, ctx, q_req, sources) {
                        q_req = s.q_rest_wh;
                    }
                }
                q_req
            }
        }
    }

    /// Appelle le générateur d'indice `i` pour un poste, si les conditions du texte le
    /// permettent, et met à jour son jeu de données.
    fn appeler(
        &mut self,
        i: usize,
        ctx: &ContexteAppel,
        q_req_wh: f64,
        sources: &[ConditionsSource],
    ) -> Option<SortiesAppel> {
        let g = &mut self.generateurs[i];
        let fonction = g.modele.fonction();
        let source = sources.get(i).copied().unwrap_or_default();
        let (r_pui_dispo, ecs_seule) = match ctx.poste {
            Poste::Ecs => {
                // Le générateur n'est disponible que s'il n'a pas refroidi.
                if g.jeu.ia_refroidi {
                    return None;
                }
                // « iecs_seule » : générateur d'ECS seule, ou hors saison de chauffage.
                let ecs_seule = fonction == FonctionGenerateur::Ecs || !ctx.aut_ch;
                if !(q_req_wh > 0.0 || ecs_seule) {
                    return None;
                }
                (1.0, ecs_seule)
            }
            Poste::Refroidissement => {
                // La demande est non nulle, ou la consommation à charge nulle du générateur est
                // comptée en refroidissement (hors saison de chauffage, ou générateur non
                // réversible).
                let appel = q_req_wh > 0.0
                    || fonction != FonctionGenerateur::ChauffageRefroidissement
                    || !ctx.aut_ch;
                if !appel {
                    return None;
                }
                if q_req_wh > 0.0 {
                    g.jeu.ia_refroidi = true;
                }
                (1.0 - g.jeu.r_fonct_ecs, false)
            }
            Poste::Chauffage => {
                if g.jeu.ia_refroidi {
                    return None;
                }
                (1.0 - g.jeu.r_fonct_ecs, false)
            }
        };
        if r_pui_dispo <= 0.0 {
            return None;
        }
        let s = g.modele.appel(&EntreesAppel {
            fonction: ctx.poste,
            q_req_wh,
            theta_amont_c: source.theta_amont_c,
            theta_aval_c: ctx.theta_aval_c,
            r_pui_dispo,
            ecs_seule,
            theta_amb_c: ctx.theta_amb_c,
            air_extrait_w_k: source.air_extrait_w_k,
        });
        g.jeu.ajouter(ctx.poste, r_pui_dispo, &s);
        Some(s)
    }

    /// « idsousdim_court » et « idsousdim_long » en chauffage et ECS (indice 0) ou en
    /// refroidissement (indice 1) : alertes de sous-dimensionnement (éq. 1060, 1061, 1063 et
    /// 1064).
    pub fn sous_dimensionnement(&self, k: usize) -> (bool, bool) {
        let n = self.cumuls.nb_sousdim_max[k];
        (n > SEUIL_SOUSDIM_COURT_H, n > SEUIL_SOUSDIM_LONG_H)
    }
}

/// Contexte commun aux appels des générateurs d'un poste à un pas de temps.
#[derive(Debug, Clone, Copy)]
struct ContexteAppel {
    poste: Poste,
    theta_aval_c: f64,
    theta_amb_c: f64,
    aut_ch: bool,
    relance_ch: bool,
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn joule(id: &str, poste: Poste, pn: f64) -> GenerateurHoraire {
        GenerateurHoraire::nouveau(
            id,
            Generateur::EffetJoule(GenerateurEffetJoule {
                fonction: poste,
                pn_gen_w: pn,
                rdim: 1.0,
            }),
        )
    }

    fn reseau(poste: Poste, q: f64) -> ReseauVu {
        ReseauVu {
            poste,
            hydraulique: false,
            q_sys_wh: q,
            theta_moy_c: 0.0,
            theta_i_aval_eq_c: 19.0,
            relance: false,
            rat_surf_dess: 1.0,
        }
    }

    fn entrees(reseaux: &[ReseauVu]) -> EntreesGeneration<'_> {
        EntreesGeneration {
            reseaux,
            aut_ch: true,
            aut_fr: false,
            id_encl: true,
            theta_ext_c: 5.0,
            b_therm: 1.0,
            w_rechauf_wh: 0.0,
            sources: &[],
        }
    }

    #[test]
    fn sans_priorite_au_prorata_des_puissances() {
        // Deux générateurs de 1 et 3 kW, demande de 2 kWh : 500 et 1 500 Wh (éq. 1018 et 1026).
        let mut g = GenerationHoraire::nouvelle(
            "g",
            vec![
                joule("a", Poste::Chauffage, 1000.0),
                joule("b", Poste::Chauffage, 3000.0),
            ],
        );
        let r = [reseau(Poste::Chauffage, 2000.0)];
        let s = g.pas(&entrees(&r), 0).clone();
        assert!((g.generateurs[0].jeu.q_fou_wh[0] - 500.0).abs() < 1e-9);
        assert!((g.generateurs[1].jeu.q_fou_wh[0] - 1500.0).abs() < 1e-9);
        assert!((s.q_cef.valeur(Poste::Chauffage, Energie::Electricite) - 2000.0).abs() < 1e-9);
        assert_eq!(s.q_rep_wh, [0.0; 3]);
        assert_eq!(s.rat_bes_dp, vec![1.0]);
        // Génération sur air : température de fonctionnement, celle de l'air (éq. 1012).
        assert_eq!(s.theta_aval_c[0], 19.0);
        assert_eq!(s.theta_amb_c, 20.0);
    }

    #[test]
    fn report_de_la_demande_non_assuree() {
        // 4 kW installés, demande de 5 kWh : 1 kWh reporté (éq. 1028), fourni au pas suivant
        // (éq. 1014).
        let mut g = GenerationHoraire::nouvelle(
            "g",
            vec![
                joule("a", Poste::Chauffage, 1000.0),
                joule("b", Poste::Chauffage, 3000.0),
            ],
        );
        let r = [reseau(Poste::Chauffage, 5000.0)];
        let s = g.pas(&entrees(&r), 0).clone();
        assert!((s.q_rep_wh[0] - 1000.0).abs() < 1e-9);
        assert!((s.q_fou_wh[0] - 4000.0).abs() < 1e-9);
        let r = [reseau(Poste::Chauffage, 500.0)];
        let s = g.pas(&entrees(&r), 0).clone();
        assert!((s.q_req_tot_ar_wh[0] - 1500.0).abs() < 1e-9);
        assert!((s.q_fou_wh[0] - 1500.0).abs() < 1e-9);
        assert_eq!(s.q_rep_wh[0], 0.0);
        // Bilan sur les deux pas : tout ce qui a été demandé a été fourni et consommé.
        let c = g.cumuls.cef.annuel();
        assert!((c.total() - 5500.0).abs() < 1e-9);
        assert_eq!(g.sous_dimensionnement(0), (false, false));
    }

    #[test]
    fn cascade_et_relance() {
        // Cascade : le générateur prioritaire est sollicité jusqu'à sa puissance, le suivant
        // reçoit le reste (éq. 1029 à 1034).
        let mut a = joule("base", Poste::Chauffage, 2000.0);
        a.priorite = [1; 3];
        let mut b = joule("appoint", Poste::Chauffage, 3000.0);
        b.priorite = [2; 3];
        // Saisis dans l'ordre inverse des priorités.
        let mut g = GenerationHoraire::nouvelle("g", vec![b, a]);
        g.priorite = ModePriorite::Cascade;
        let r = [reseau(Poste::Chauffage, 2600.0)];
        let s = g.pas(&entrees(&r), 0).clone();
        assert!((g.generateurs[1].jeu.q_fou_wh[0] - 2000.0).abs() < 1e-9);
        assert!((g.generateurs[0].jeu.q_fou_wh[0] - 600.0).abs() < 1e-9);
        assert_eq!(s.q_rep_wh[0], 0.0);
        // En relance, le générateur de rang le plus élevé n'est pas pris en compte
        // (§8.17.3.5.6) : 600 Wh reportés.
        let mut r = [reseau(Poste::Chauffage, 2600.0)];
        r[0].relance = true;
        let s = g.pas(&entrees(&r), 0).clone();
        assert_eq!(g.generateurs[0].jeu.q_fou_wh[0], 0.0);
        assert!((s.q_rep_wh[0] - 600.0).abs() < 1e-9);
        assert_eq!(s.relance, [true, false]);
    }

    #[test]
    fn postes_et_saisons() {
        // Chauffage hors saison : générateurs désactivés, demande reportée ; ECS toute l'année ;
        // réchauffeur de boucle ajouté à la case (ECS ; électricité) (éq. 1057).
        let mut g = GenerationHoraire::nouvelle(
            "g",
            vec![
                joule("ch", Poste::Chauffage, 5000.0),
                joule("ecs", Poste::Ecs, 3000.0),
            ],
        );
        let r = [reseau(Poste::Chauffage, 100.0), reseau(Poste::Ecs, 600.0)];
        let mut e = entrees(&r);
        e.aut_ch = false;
        e.w_rechauf_wh = 50.0;
        let s = g.pas(&e, 6).clone();
        assert_eq!(s.q_fou_wh, [0.0, 0.0, 600.0]);
        assert_eq!(s.q_rep_wh, [100.0, 0.0, 0.0]);
        assert!((s.q_cef.valeur(Poste::Ecs, Energie::Electricite) - 650.0).abs() < 1e-9);
        assert_eq!(s.q_cef.total_poste(Poste::Chauffage), 0.0);
        assert!((g.generateurs[1].jeu.r_fonct_ecs - 0.2).abs() < 1e-12);
        // Ni saison de chauffage ni de refroidissement : ambiance à 23 °C.
        assert_eq!(s.theta_amb_c, 23.0);
        assert_eq!(g.cumuls.q_req_mensuel_wh[2][6], 600.0);
        assert_eq!(g.generateurs[0].cumuls.nbh_hors_fonctionnement[0], 1);
        assert_eq!(g.generateurs[1].cumuls.nbh_charge[2][2], 1);
    }

    #[test]
    fn temperatures_de_fonctionnement_hydrauliques() {
        let mut g = GenerationHoraire::ideale("g");
        g.theta_dist_ch_max_c = Some(45.0);
        let mut r = [
            reseau(Poste::Chauffage, 1000.0),
            reseau(Poste::Chauffage, 500.0),
        ];
        for (x, t) in r.iter_mut().zip([32.0, 38.0]) {
            x.hydraulique = true;
            x.theta_moy_c = t;
            x.rat_surf_dess = 0.5;
        }
        // Température des réseaux : la plus haute (éq. 1010).
        let s = g.pas(&entrees(&r), 0).clone();
        assert_eq!(s.theta_aval_c[0], 38.0);
        assert!((s.rat_bes_dp[0] - 2.0 / 3.0).abs() < 1e-12);
        // Relance : température maximale des réseaux.
        r[0].relance = true;
        assert_eq!(g.pas(&entrees(&r), 0).theta_aval_c[0], 45.0);
        // Température constante.
        g.gestion_ch = GestionTemperature::Constante(50.0);
        assert_eq!(g.pas(&entrees(&r), 0).theta_aval_c[0], 50.0);
        // Hors volume chauffé, en saison de chauffage : θamb = b·θext + (1 − b)·20.
        g.en_volume_chauffe = false;
        let mut e = entrees(&r);
        e.b_therm = 0.6;
        let s = g.pas(&e, 0).clone();
        assert!((s.theta_amb_c - (0.6 * 5.0 + 0.4 * 20.0)).abs() < 1e-12);
    }

    #[test]
    fn sous_dimensionnement_compte_les_heures_successives() {
        let mut g = GenerationHoraire::nouvelle("g", vec![joule("a", Poste::Chauffage, 1000.0)]);
        let r = [reseau(Poste::Chauffage, 1500.0)];
        for _ in 0..7 {
            g.pas(&entrees(&r), 0);
        }
        assert_eq!(g.cumuls.nb_sousdim_max[0], 7);
        assert_eq!(g.sous_dimensionnement(0), (true, false));
        // Une heure sans report remet le compteur à zéro.
        let r = [reseau(Poste::Chauffage, 0.0)];
        for _ in 0..8 {
            g.pas(&entrees(&r), 0);
        }
        assert_eq!(g.etat.nb_sousdim[0], 0);
    }
}
