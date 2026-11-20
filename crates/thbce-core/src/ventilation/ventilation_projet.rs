// SPDX-License-Identifier: GPL-3.0-or-later
//! Système de ventilation du projet d'une zone, pour les calculs Th-C et Th-D :
//! assemblage des fiches `C_VEN_Bouche_conduit` (§6.2), `C_VEN_Mécanique_SF` (§6.5),
//! `C_VEN_Mécanique_Basse-Pression` (§6.4) et `C_VEN_CTA et Double_flux` (§6.3, double flux et
//! centrales de traitement d'air en zone neutre).
//!
//! « La modélisation du système de ventilation s'effectue au niveau d'une zone » (§6,
//! introduction). Avant la simulation, chaque bouche reçoit ses débits (dépassement et fuites)
//! et son conduit son coefficient de transfert hors volume chauffé ; les ventilateurs, leur
//! puissance moyenne. À chaque pas de temps, avant le bilan aéraulique de la zone (§5.6),
//! [`VentilationProjet::pas`] donne la température et l'humidité de l'air soufflé dans chaque
//! groupe, à partir des températures des groupes à la fin du pas précédent.
//!
//! En habitation, les débits et la puissance des ventilateurs sont constants. Dans les autres
//! usages (simple flux, §6.2.3.1.1.1 et §6.5.3.3), ils dépendent de l'indicateur de ventilation
//! de la zone, `Ivent` : débits et puissance d'occupation ou d'inoccupation.
//!
//! Une centrale de traitement d'air (types 2 à 4) choisit en outre, avant chaque pas de temps,
//! entre ses débits nominaux (mode actif) et ses débits de zone neutre
//! ([`VentilationProjet::preparer_pas`], §6.3.3.4.3.2) ; hors habitation, la consommation des
//! ventilateurs des centrales double flux est relevée heure par heure pendant la simulation
//! ([`VentilationProjet::enregistrer`]).
//!
//! Le calcul du Bbio n'emploie pas ce système : il emploie le système conventionnel de la
//! fiche 6.1 (§6.1.3 ; décision Q36).

use crate::climat::calendrier::date_scenario;
use crate::conventions::NB_HEURES_AN;
use crate::entree::{
    BatteriePrechauffage, BatteriePrerefroidissement, Echangeur, Isouf, RegulationAirNeuf,
    RepresentationEchangeur, StatutEchangeur, SystemeVentilation, TypeCta, TypeHumidificationHiver,
    TypeRafraichissementEvaporatif, UsageZone, Zone,
};
use crate::resultats::ValeurParDefaut;

use super::bouche_conduit::{
    a_cond_defaut_collectif_m2, a_cond_defaut_maison_m2, debits_dep_habitation,
    debits_dep_non_residentiel, k_res, q_fuites_m3_h, Debits, DebitsBoucheProjet, PressionReseau,
    TypologieReseau, CDEP_DEFAUT, CFENB, CRDBNR_BBIO, CRDBNR_DEFAUT, DUGD_DEFAUT_H, K_RES_BBIO,
    K_RES_DEFAUT,
};
use super::cta_et_double_flux::{
    antigel, conditions_debit_nominal, debits_nominaux, delta_bypass, epsilon_detaille, epsilon_t,
    melange, p_vent_cta_w, prechauffage, prerefroidissement, qm_air_neuf_kg_s, sorties_echangeur_c,
    taux_an_min, theta_s_ech_rep_lim_defaut_c,
};
use super::cta_et_double_flux::{
    charge_debit_variable, cp_air_humide_j_kg_k, exp_h, froid_rec, q_chaud_rec_wh,
    theta_sou_bouches_charge_c, w_vent_rec_wh, ChargeCentrale, THETA_SOUF_MAX_C,
};
use super::debits_air_cep::{permea_verticale, rho_kg_m3 as rho_humide_kg_m3};
use super::humidification::{
    aut_adiab, consignes_hysteresis_c, groupe_assez_sec, humidificateur_adiabatique,
    humidification_hiver, p_vent_adiab_w, position_hysteresis, w_aux_pompe_wh, TypeHumidificateur,
    ETA_HUM_CONV,
};
use super::mecanique_basse_pression;
use super::mecanique_sf::{
    dugd_equ_h, h_cond_ext_w_k, p_vent_groupe_w, p_vent_residentiel_w, qm_kg_s, rho_kg_m3,
    t_apres_conduit_c, t_apres_ventilateur_c, t_lnc_c, theta_i_cond_c, C_PA_J_KG_K, RHO_REF_KG_M3,
};
use super::naturelle_hybride::{
    a_bouche_eq_m2, h_mot_defaut_m, ConditionsConduit, ConditionsPas, ConduitResolu, EtatNaturelle,
    CHI_COUDE_DEFAUT, D_PREF_DEFAUT_PA, LAMBDA_COND_DEFAUT, V_VENT_REF_DEFAUT_M_S,
};
use super::puits_climatique::EtatPuits;
use super::puits_hydraulique::{
    efficacite_batterie_ch, efficacite_batterie_fr, EtatPuitsHydraulique, CP_AIR_PH_J_KG_K,
    CP_EAU_GL_J_KG_K, EPS_BATT_DEFAUT, RHO_EAU_GL_KG_M3,
};
use super::surventilation_mecanique::{a_cond_rafnoc_m2, SurventilationResolue};

/// Nature du système, résolue pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub enum NatureSysteme {
    /// Simple flux par extraction (§6.5).
    SimpleFluxExtraction,
    /// Simple flux par insufflation (§6.5).
    SimpleFluxInsufflation,
    /// Extraction mécanique basse pression (§6.4).
    BassePression,
    /// Double flux (§6.3) ; `None` : sans échangeur.
    DoubleFlux(Option<EchangeurResolu>),
    /// Aération par ouverture des fenêtres (§6.7) : aucun ventilateur ni conduit.
    Aeration,
    /// Ventilation naturelle par conduits ou hybride (§6.6) ; l'état est
    /// [`VentilationProjet::naturelle`].
    Naturelle,
}

/// Efficacité d'un échangeur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EfficaciteEchangeur {
    /// « εt » après prise en compte du statut (éq. 396 à 398).
    Simplifiee(f64),
    /// Coefficient d'échange global et écoulements (éq. 499 à 505).
    Detaillee {
        /// « UA », en W/K.
        ua_w_k: f64,
        /// « idtype_echangeur ».
        ecoulement: crate::entree::EcoulementEchangeur,
    },
}

/// Échangeur d'un double flux, prêt pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct EchangeurResolu {
    /// Efficacité.
    pub efficacite: EfficaciteEchangeur,
    /// Régulation du bypass ; `None` sans bypass.
    pub bypass: Option<crate::entree::Bypass>,
    /// « isantigel ».
    pub antigel: bool,
    /// « θs,ech,rep,LIM », en °C.
    pub theta_s_ech_rep_lim_c: f64,
    /// « Pech » : puissance d'un échangeur rotatif, en W (éq. 497) ; 0 pour un échangeur à
    /// plaques.
    pub p_ech_w: f64,
}

/// Paramètres propres à une centrale de traitement d'air (types 2 à 4), prêts pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct CtaResolue {
    /// « Type_ventilation_mecaniq ».
    pub type_cta: TypeCta,
    /// « Pvent,rep,CH » et « Pvent,sou,CH », en W.
    pub p_vent_ch_w: (f64, f64),
    /// « Pvent,rep,ZN occ » et « Pvent,sou,ZN occ », en W.
    pub p_vent_zn_occ_w: (f64, f64),
    /// « Pvent,rep,ZN inocc » et « Pvent,sou,ZN inocc », en W.
    pub p_vent_zn_inocc_w: (f64, f64),
    /// « θENC » et « θENF », en °C.
    pub theta_enc_enf_c: (f64, f64),
    /// « Type_régul_AN ».
    pub regulation_air_neuf: RegulationAirNeuf,
    /// Batteries régulées sur la charge (mode actif, §6.3.3.5) ; `None` sans batterie chaude ni
    /// froide.
    pub batteries_charge: Option<BatteriesCharge>,
}

/// Batteries d'une centrale à charge variable régulées sur la charge (« Is_chaud »,
/// « Is_froid », §6.3.3.5).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BatteriesCharge {
    /// « Is_chaud ».
    pub chaud: bool,
    /// « θsou_nom_chaud » (type 4), en °C (éq. 585).
    pub theta_sou_nom_chaud_c: Option<f64>,
    /// Batterie froide (« Is_froid »), `None` sans elle.
    pub froid: Option<BatterieFroideCharge>,
}

/// Batterie froide régulée sur la charge.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BatterieFroideCharge {
    /// « θsou,nom,froid » (type 4), en °C (éq. 591).
    pub theta_sou_nom_c: Option<f64>,
    /// « θdim,fr », en °C (éq. 604).
    pub theta_dim_c: f64,
}

/// Rafraîchissement évaporatif d'une centrale, prêt pour le calcul (§6.3.3.4.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdiabatiqueResolu {
    /// Indirect (sur l'air extrait) ; direct sinon.
    pub indirect: bool,
    /// « ηhum_dir » ou « ηhum_indir ».
    pub eta: f64,
    /// « θi_base », en °C.
    pub theta_i_base_c: f64,
    /// Consignes du profil à hystérésis d'une centrale à débit variable (éq. 426).
    pub consignes: Option<[f64; 4]>,
}

/// Résultat du mode actif d'une centrale au pas de temps (§6.3.3.5).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ModeActif {
    /// La centrale a fonctionné en mode actif.
    pub actif: bool,
    /// « τCH/FR(h) » (éq. 582, 587 et 593).
    pub tau: f64,
    /// « Qchaud(rec)(h) », en Wh (éq. 599).
    pub q_chaud_wh: f64,
    /// « Qfroid(rec)(h) », latent compris, en Wh, négatif ou nul (éq. 611).
    pub q_froid_wh: f64,
    /// « Qsys_lat_fr(h) », en Wh, négatif ou nul (éq. 610).
    pub q_lat_wh: f64,
    /// Sous-dimensionnement de la batterie (éq. 597 et 605).
    pub sous_dimensionnee: bool,
    /// Le mode actif a chauffé l'air (sinon, s'il est actif, il l'a refroidi).
    pub chauffage: bool,
    /// « θsouf,mel(rec)(h) » : température après la boîte de mélange recalculée
    /// (§6.3.3.5.6), vue par la génération des batteries (éq. 951), en °C.
    pub theta_souf_mel_c: f64,
    /// « Wag(rec)(h) » pondérée par les durées de fonctionnement : `τ·Wag(rec) + (1 − τ)·Wag`
    /// (§6.3.3.5.5.4, lecture Q95 point 11), en Wh.
    pub w_ag_wh: f64,
    /// Demandes des émetteurs contenus dans les bouches que le mode actif n'assure pas
    /// (chauffage et refroidissement au même pas, centrale sans débit) : transmises telles
    /// quelles à la génération de la batterie de même fonction (lecture Q95 point 8), en Wh
    /// (chaud positif, froid négatif).
    pub q_direct_wh: (f64, f64),
    /// « Waux,adiab(rec)(h) » pondérée par les durées de fonctionnement (éq. 615 à 617), en Wh.
    pub w_aux_adiab_wh: f64,
    /// « Wélec^PH,S(rec)(h) » de la pompe du puits hydraulique, pondérée par les durées de
    /// fonctionnement : `τ·Wélec(rec) + (1 − τ)·Wélec` (§6.3.3.5.8.4.1), en Wh.
    pub w_puits_wh: f64,
}

/// Grandeurs de la première phase (zone neutre) gardées pour le mode actif.
#[derive(Debug, Clone, PartialEq, Default)]
struct ZoneNeutre {
    /// Débit spécifique massique et température soufflée de chaque bouche (0 pour une bouche
    /// de reprise), en kg/s et °C (éq. 571).
    souffle: Vec<(f64, f64)>,
    /// Humidité de l'air soufflé, en kg/kg.
    omega_souffle_kg_kg: f64,
    /// « Wvent(h) » de la zone neutre, en Wh.
    w_vent_wh: f64,
    /// « Wech(h) », en Wh.
    w_ech_wh: f64,
    /// « Wag(h) », en Wh (éq. 508).
    w_ag_wh: f64,
    /// « Waux,adiab(h) » de la zone neutre, en Wh (éq. 566).
    w_aux_adiab_wh: f64,
    /// « Wélec^PH,S(h) » de la zone neutre, en Wh (éq. 748 et 749).
    w_puits_wh: f64,
}

/// Circuits d'un système jusqu'au ventilateur de soufflage.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Centrale {
    /// Sorties partielles (air rejeté, échangeur, antigel, mélange, taux d'air neuf).
    sorties: PasVentilation,
    /// « qm,sou,cond(h) », en kg/s.
    qm_sou_kg_s: f64,
    /// « θsouf2(h) », en °C.
    theta_souf2_c: f64,
    /// « ωsouf2(h) », en kg/kg.
    omega_souf2_kg_kg: f64,
    /// « Wech(h) », en Wh.
    w_ech_wh: f64,
    /// « qm,eau_été^pompe(h) » du rafraîchissement indirect, en kg/s (éq. 486).
    qm_eau_ete_kg_s: f64,
    /// « qm,air_neuf(h) », en kg/s.
    qm_an_kg_s: f64,
}

/// Régime d'une centrale au pas de temps, fixé avant le calcul des groupes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Regime {
    /// « isvent(h) ».
    pub i_vent: bool,
    /// « isvent(h−1) ».
    pub i_vent_h1: bool,
    /// Conditions du débit nominal des éq. (421) et (460) remplies.
    pub conditions_nominales: bool,
    /// Débits nominaux (indice CH) employés ; débits de zone neutre sinon.
    pub nominal: bool,
    /// « Aut_adiab » : rafraîchissement évaporatif autorisé (§6.3.3.4.2).
    pub adiab: bool,
    /// Débits et ventilateurs d'une centrale à débit variable selon le profil à hystérésis
    /// (éq. 426 à 428, 449, 450, 466 et 467) : `Aut_adiab` et conditions du débit nominal.
    pub hysteresis: bool,
    /// « irafnoc_activé(h) » : rafraîchissement par surventilation mécanique en service
    /// (§6.12.3.3, éq. 766) ; ses débits et ses puissances remplacent ceux du régime.
    pub rafnoc: bool,
    /// Mode actif : `θan` du puits climatique recalculé avec les nouveaux débits d'air neuf
    /// (éq. 487 et 488).
    pub recalcul_puits: bool,
}

/// Relevé de la consommation des auxiliaires d'une centrale double flux hors habitation
/// pendant la simulation (éq. 565 et 570).
#[derive(Debug, Clone, PartialEq)]
pub struct ReleveVentilation {
    /// « Wvent(h) + Wech(h) » à chaque pas de temps, en Wh.
    pub serie_wh: Vec<f64>,
    /// « Waux,v^gr » de chaque groupe, cumulée par mois des scénarios, en kWh.
    pub mensuel_groupes_kwh: Vec<[f64; 12]>,
    /// Part de chaque groupe au dernier pas de temps, en Wh.
    dernier_groupes_wh: Vec<f64>,
    /// Consommation totale au dernier pas de temps, en Wh.
    dernier_wh: f64,
}

/// Bouche et conduit du système, prêts pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct BoucheProjet {
    /// Indice du groupe de la bouche dans la zone.
    pub groupe: usize,
    /// Débits de la bouche et de son conduit : constants en habitation, ceux de l'occupation
    /// (`Ivent` vrai) dans les autres usages.
    pub debits: DebitsBoucheProjet,
    /// Débits en inoccupation (`Ivent` faux) dans les usages autres que l'habitation ; `None`
    /// en habitation.
    pub debits_inocc: Option<DebitsBoucheProjet>,
    /// « Dugd » de la bouche, en h par semaine.
    pub dugd_h: f64,
    /// « Cdep » de la bouche.
    pub cdep: f64,
    /// « Hcond,rep,ext » ou « Hcond,souf,ext », en W/K.
    pub h_cond_ext_w_k: f64,
    /// Débits du mode actif d'une bouche reliée à une centrale de traitement d'air, en
    /// occupation et en inoccupation (fiche 6.2, éq. 406, 410, 415 et 416) ; `None` sinon.
    pub debits_ch: Option<(DebitsBoucheProjet, DebitsBoucheProjet)>,
    /// « TauxAN_min,occ » et « TauxAN_min,inocc » d'une bouche de soufflage reliée à une
    /// centrale de traitement d'air (fiche 6.2, éq. 393, 397, 408 et 411) ; 1 sinon.
    pub taux_an_min: (f64, f64),
    /// Position du débit dans le profil à hystérésis du rafraîchissement évaporatif d'une
    /// centrale à débit variable, de 0 (zone neutre) à 1 (mode actif) (éq. 427 et 449).
    pub x_adiab: f64,
    /// Débits en rafraîchissement par surventilation mécanique (§6.12.3.4, éq. 758, 759 et
    /// 775 : ni `Cdep` ni `Crdbnr`, fuites comprises) ; `None` sans surventilation.
    pub debits_rafnoc: Option<DebitsBoucheProjet>,
}

/// « Autfr(j) » d'une centrale (§6.3.3.4.1) : saison effective de refroidissement si la
/// centrale dessert au moins un groupe climatisé (éq. 410), sinon la plus large des saisons
/// propres de refroidissement des groupes desservis (éq. 411).
fn aut_fr_centrale(groupes: &[EtatGroupeVentilation], dessert: impl Fn(usize) -> bool) -> bool {
    let desservis = || {
        groupes
            .iter()
            .enumerate()
            .filter(|(g, _)| dessert(*g))
            .map(|(_, x)| x)
    };
    if desservis().any(|g| g.clim) {
        desservis().any(|g| g.saison_de_refroidissement)
    } else {
        desservis().any(|g| g.aut_fr_propre)
    }
}

impl BoucheProjet {
    /// Débits de la bouche selon l'indicateur de ventilation de la zone.
    pub fn debits_selon(&self, i_vent: bool) -> DebitsBoucheProjet {
        match self.debits_inocc {
            Some(inocc) if !i_vent => inocc,
            _ => self.debits,
        }
    }

    /// Débits de la bouche dans un régime de centrale : débits nominaux (mode actif) ou de
    /// zone neutre, selon l'occupation (fiche 6.2, éq. 406 à 418).
    pub fn debits_regime(&self, regime: Regime) -> DebitsBoucheProjet {
        // Éq. (775) : débits du rafraîchissement par surventilation mécanique.
        if let Some(d) = self.debits_rafnoc.filter(|_| regime.rafnoc) {
            return d;
        }
        match self.debits_ch {
            Some((occ, inocc)) if regime.nominal => {
                if regime.i_vent {
                    occ
                } else {
                    inocc
                }
            }
            Some((occ, inocc)) if regime.hysteresis && self.x_adiab > 0.0 => {
                // Éq. (427) et (449) : entre le débit de zone neutre et celui du mode actif.
                // Borne basse : débit de zone neutre en occupation « ZN,occ », y compris en
                // relance (éq. 426 à 428, figure 73 ; Q96 point 4, lot 1-ven4).
                let zn = self.debits;
                let ch = if regime.i_vent { occ } else { inocc };
                let x = self.x_adiab;
                let l = |a: f64, b: f64| a + x * (b - a);
                DebitsBoucheProjet {
                    q_max_m3_h: l(zn.q_max_m3_h, ch.q_max_m3_h),
                    q_base_m3_h: l(zn.q_base_m3_h, ch.q_base_m3_h),
                    q_dep_m3_h: l(zn.q_dep_m3_h, ch.q_dep_m3_h),
                    q_fuites_m3_h: l(zn.q_fuites_m3_h, ch.q_fuites_m3_h),
                    ..zn
                }
            }
            _ => self.debits_selon(regime.i_vent),
        }
    }
}

/// État d'un groupe vu par le système de ventilation, à la fin du pas de temps précédent.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EtatGroupeVentilation {
    /// « θi,fin(h−1) », en °C.
    pub theta_i_fin_c: f64,
    /// « ωi,fin(h−1) », en kg/kg (éq. 473).
    pub omega_i_fin_kg_kg: f64,
    /// Saison de chauffage du groupe (« Autch,eff(j) », §6.3.3.4.1, éq. 409).
    pub saison_de_chauffage: bool,
    /// Saison de refroidissement effective d'un groupe climatisé (« Autfr,eff(j) », éq. 410) ;
    /// faux pour un groupe non climatisé et en calcul du confort d'été (§13.5.3.1).
    pub saison_de_refroidissement: bool,
    /// « irelance^gr » du groupe au pas précédent (`docs/questions/Q090.md`).
    pub relance: bool,
    /// « Isbesoin_ch^gr(h−1) » : besoin de chauffage du groupe au pas précédent.
    pub besoin_ch: bool,
    /// « Isbesoin_fr^gr(h−1) » : besoin de refroidissement du groupe au pas précédent.
    pub besoin_fr: bool,
    /// Groupe climatisé (« Is_climatisé^gr », éq. 410).
    pub clim: bool,
    /// Saison propre de refroidissement du groupe (« Autfr,pro », éq. 411).
    pub aut_fr_propre: bool,
    /// « θop,fin^gr(h−1) », en °C, pour la surventilation mécanique (éq. 771 et 772).
    pub theta_op_fin_c: f64,
}

/// Air soufflé dans un groupe par le système au pas de temps.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AirSouffle {
    /// « Tair_soufflé », en °C (moyenne des bouches du groupe pondérée par leurs débits).
    pub t_air_souffle_c: f64,
    /// « ωair_soufflé », en kg/kg.
    pub omega_air_souffle_kg_kg: f64,
}

/// Sorties du système au pas de temps.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PasVentilation {
    /// « θair_extrait(h) » : température de l'air rejeté par le système, en °C (éq. 654 ;
    /// éq. 513 en double flux) ; préparée pour les générateurs thermodynamiques sur air extrait.
    pub theta_air_extrait_c: f64,
    /// « qm,air_extrait(h) » : débit d'air rejeté, en kg/s (éq. 654 et 456).
    pub qm_air_extrait_kg_s: f64,
    /// « ε(h) » : efficacité de l'échangeur au pas de temps (double flux).
    pub epsilon: f64,
    /// « δbypass(h) » (double flux).
    pub bypass: bool,
    /// « Wag(h) » : énergie fournie pour la sécurité antigel, en Wh (éq. 508).
    pub w_ag_wh: f64,
    /// « Qprechaud(h) » : énergie fournie par la batterie de préchauffage, en Wh (éq. 523).
    pub q_prechaud_wh: f64,
    /// « Qprefroid(h) » : énergie fournie par la batterie de prérefroidissement, en Wh, négative
    /// ou nulle (éq. 545, lecture Q91).
    pub q_prefroid_wh: f64,
    /// « θsouf,mel(h) » : température après la boîte de mélange, en °C (éq. 515 et 517), vue par
    /// les batteries (fiche 8.11, éq. 951).
    pub theta_souf_mel_c: f64,
    /// « TauxAN(h) » (éq. 441).
    pub taux_an: f64,
    /// « QHR(h) » : énergie de l'humidificateur d'hiver à vapeur, en Wh (éq. 529).
    pub q_hr_wh: f64,
    /// « Waux,chaudHR(h) » : pompe de l'humidificateur d'hiver à pulvérisation, en Wh (éq.
    /// 568).
    pub w_aux_chaudhr_wh: f64,
    /// « Waux,adiab(h) » : pompe du rafraîchissement évaporatif, en Wh (éq. 566).
    pub w_aux_adiab_wh: f64,
    /// « Aut_adiab » du pas de temps (§6.3.3.4.2).
    pub adiab: bool,
    /// « irafnoc_activé(h) » du pas de temps (§6.12.3.3, éq. 766).
    pub rafnoc: bool,
    /// « θan(h) » : température de l'air neuf, en °C, avec un puits climatique ; `None` sans
    /// puits (`θext`).
    pub theta_an_c: Option<f64>,
    /// Le puits climatique est traversé (non contourné) et l'air neuf circule.
    pub puits_actif: bool,
    /// Énergie échangée dans le puits climatique, en Wh : `Cpa × qm,air_neuf × (θan − θext)`,
    /// positive en préchauffage, négative en prérafraîchissement.
    pub q_puits_wh: f64,
    /// « Wélec^PH,S(h) » : consommation de la pompe du puits hydraulique attribuée au système,
    /// en Wh (éq. 748 et 749), comptée dans les auxiliaires de ventilation.
    pub w_puits_wh: f64,
}

/// Système de ventilation du projet d'une zone d'habitation, prêt pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct VentilationProjet {
    /// Identifiant du système.
    pub id: String,
    /// Nature du système.
    pub nature: NatureSysteme,
    /// « b » du réseau (éq. 471, 649 et 665).
    pub b: f64,
    /// Bouches et conduits.
    pub bouches: Vec<BoucheProjet>,
    /// Puissance moyenne du ventilateur de reprise, en W (éq. 393, 620 et 641) ; hors
    /// habitation, puissance en occupation (éq. 636).
    pub p_vent_rep_w: f64,
    /// Puissance moyenne du ventilateur de soufflage, en W (éq. 391 et 643) ; hors habitation,
    /// puissance en occupation (éq. 638).
    pub p_vent_sou_w: f64,
    /// Puissances des ventilateurs de reprise et de soufflage en inoccupation, en W, hors
    /// habitation (éq. 637 et 639) ; `None` en habitation, où la puissance est constante.
    pub p_vent_inocc_w: Option<(f64, f64)>,
    /// Nombre de groupes de la zone.
    pub nb_groupes: usize,
    /// Valeurs par défaut du texte appliquées ; champ relatif à la zone.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
    /// Paramètres d'une centrale de traitement d'air ; `None` pour les autres systèmes.
    pub cta: Option<CtaResolue>,
    /// Batterie de préchauffage (double flux et centrale à débit constant).
    pub prechauffage: Option<BatteriePrechauffage>,
    /// Batterie de prérefroidissement (double flux et centrale à débit constant).
    pub prerefroidissement: Option<BatteriePrerefroidissement>,
    /// Relevé de la consommation des ventilateurs pendant la simulation : centrales double
    /// flux hors habitation ; `None` sinon (consommation déduite des puissances).
    pub releve: Option<ReleveVentilation>,
    /// « isreport_ch^g(h−1) » et « isreport_fr^g(h−1) » : la génération de la batterie chaude
    /// (froide) régulée sur la charge était en situation de report d'énergie au pas précédent
    /// (§6.3.3.1.4, éq. 583, 584, 588 et 594 ; défini p. 1721 ; lecture Q95 point 7) ; écrit
    /// par le bâtiment avant chaque pas de temps.
    pub isreport: [bool; 2],
    /// Humidificateur d'hiver (§6.3.3.4.9.2) ; `None` sans humidificateur.
    pub humidificateur: Option<TypeHumidificateur>,
    /// « ωcons » de l'humidificateur d'hiver, en kg/kg.
    pub omega_cons_kg_kg: f64,
    /// Rafraîchissement évaporatif (§6.3.3.4.2) ; `None` sans.
    pub adiabatique: Option<AdiabatiqueResolu>,
    /// Rafraîchissement par surventilation mécanique (§6.12) et état de sa régulation ;
    /// `None` sans.
    pub surventilation: Option<SurventilationResolue>,
    /// Puits climatique de l'air neuf (§6.8, 6.10 et 6.11) et son état ; `None` sans.
    pub puits: Option<EtatPuits>,
    /// Puits hydraulique de l'air neuf (§6.9) et son état ; `None` sans.
    pub puits_h: Option<EtatPuitsHydraulique>,
    /// Erreur de saisie du puits hydraulique : puissance de batterie supérieure à ce que le débit
    /// de référence admet (efficacité > 1, §6.3.3.3.4). Contrôlée par la validation.
    pub erreur_puits_h: Option<String>,
    /// Avertissement non bloquant du puits hydraulique : débit calorifique de l'air neuf
    /// supérieur à celui de l'eau de la boucle, l'énergie cédée à l'air (éq. 415) pouvant alors
    /// dépasser ce que l'eau transporte (Q233 point 2). Contrôlé par la validation.
    pub avertissement_puits_h: Option<String>,
    /// Ventilation naturelle par conduits ou hybride (§6.6) et son état ; `None` sinon.
    pub naturelle: Option<EtatNaturelle>,
    /// « θi,fin(h−2) » de chaque groupe, pour le sens de variation du profil à hystérésis.
    theta_prec: Vec<f64>,
    /// Régime du pas de temps en cours.
    regime: Regime,
    /// « ρair_soufflé^g,s(h−1) » de chaque bouche, en kg/m³.
    rho_souffle: Vec<f64>,
    /// La même au pas précédent celui en cours, pour les fuites du mode actif (éq. 574).
    rho_souffle_h1: Vec<f64>,
    /// Première phase du pas en cours, pour le mode actif.
    zn: ZoneNeutre,
}

/// Batteries régulées sur la charge d'une centrale saisie (§6.3.3.5, `docs/questions/Q095.md`).
fn batteries_charge(c: &crate::entree::Cta) -> Option<BatteriesCharge> {
    let chaud = c.batterie_chaude == Some(true);
    let froid = (c.batterie_froide == Some(true)).then(|| BatterieFroideCharge {
        theta_sou_nom_c: c.theta_sou_nom_froid_c,
        theta_dim_c: c.theta_dim_fr_c.unwrap_or(0.0),
    });
    (chaud || froid.is_some()).then_some(BatteriesCharge {
        chaud,
        theta_sou_nom_chaud_c: c.theta_sou_nom_chaud_c,
        froid,
    })
}

fn defaut(champ: String, valeur: f64, regle: &str) -> ValeurParDefaut {
    ValeurParDefaut {
        champ,
        valeur,
        regle: regle.to_owned(),
    }
}

/// Échangeur d'une centrale double flux ou d'une CTA, prêt pour le calcul : efficacité selon
/// son statut (éq. 396 à 398), `θs,ech,rep,LIM` saisie ou par défaut (tableau 62), `Pech`.
fn echangeur_resolu(
    e: &Echangeur,
    centrale: &str,
    habitation: bool,
    valeurs_par_defaut: &mut Vec<ValeurParDefaut>,
) -> EchangeurResolu {
    let efficacite = match &e.representation {
        RepresentationEchangeur::Simplifiee(s) => {
            let statut = s.statut.unwrap_or_else(|| {
                valeurs_par_defaut.push(defaut(
                    format!(
                        "ventilation.systeme.{centrale}.echangeur.representation.simplifiee.statut"
                    ),
                    0.0,
                    "Th-BCE 2020 §6.3.2, tableau 61 : « idstatut_echangeur », 0 = valeur \
                     déclarée par le fabricant (par défaut)",
                ));
                StatutEchangeur::Declare
            });
            EfficaciteEchangeur::Simplifiee(epsilon_t(s.epsilon_saisi, statut))
        }
        RepresentationEchangeur::Detaillee(d) => EfficaciteEchangeur::Detaillee {
            ua_w_k: d.ua_w_k,
            ecoulement: d.ecoulement,
        },
    };
    let rotatif = e.rotatif == Some(true);
    let theta_s_ech_rep_lim_c = e.theta_s_ech_rep_lim_c.unwrap_or_else(|| {
        let t = theta_s_ech_rep_lim_defaut_c(rotatif, habitation);
        valeurs_par_defaut.push(defaut(
            format!("ventilation.systeme.{centrale}.echangeur.theta_s_ech_rep_lim_c"),
            t,
            match (habitation, rotatif) {
                (true, _) => "Th-BCE 2020 §6.3.3.4.6.4, tableau 62 : « autres cas »",
                (false, true) => {
                    "Th-BCE 2020 §6.3.3.4.6.4, tableau 62 : échangeur rotatif en bâtiment non \
                     résidentiel"
                }
                (false, false) => {
                    "Th-BCE 2020 §6.3.3.4.6.4, tableau 62 : échangeur à plaques en bâtiment non \
                     résidentiel"
                }
            },
        ));
        t
    });
    EchangeurResolu {
        efficacite,
        bypass: e.bypass.clone(),
        antigel: e.antigel.unwrap_or(false),
        theta_s_ech_rep_lim_c,
        p_ech_w: if rotatif {
            e.p_ech_w.unwrap_or(0.0)
        } else {
            0.0
        },
    }
}

impl VentilationProjet {
    /// Prépare le système de ventilation d'une zone d'habitation validée.
    ///
    /// `theta_initiale_c` est la température des groupes avant le premier pas de temps : l'air
    /// soufflé au premier pas de temps est réputé à cette température pour le calcul des débits
    /// massiques (éq. 420, `ρair_soufflé(h−1)`). Renvoie `None` si la zone n'a pas de système
    /// ou si son système n'est pas pris en charge (cas écartés par la validation).
    pub fn nouvelle(zone: &Zone, theta_initiale_c: f64) -> Option<Self> {
        let v = zone.ventilation.as_ref()?;
        let typologie = match zone.usage {
            UsageZone::MaisonIndividuelle => TypologieReseau::MaisonIndividuelle,
            UsageZone::LogementCollectif => TypologieReseau::BatimentCollectif,
            _ => TypologieReseau::NonResidentiel,
        };
        let non_residentiel = typologie == TypologieReseau::NonResidentiel;
        let mut valeurs_par_defaut = Vec::new();
        let (nature, pression) = match &v.systeme {
            SystemeVentilation::SimpleFluxExtraction(_) => {
                (NatureSysteme::SimpleFluxExtraction, PressionReseau::Haute)
            }
            SystemeVentilation::SimpleFluxInsufflation(_) => {
                (NatureSysteme::SimpleFluxInsufflation, PressionReseau::Haute)
            }
            SystemeVentilation::MecaniqueBassePression(_) => (
                NatureSysteme::BassePression,
                mecanique_basse_pression::PRESSION_RESEAU,
            ),
            SystemeVentilation::DoubleFlux(_) | SystemeVentilation::Cta(_) => {
                let centrale = v.systeme.nom_centrale().unwrap_or("double_flux");
                let echangeur = v.systeme.echangeur().map(|e| {
                    echangeur_resolu(e, centrale, !non_residentiel, &mut valeurs_par_defaut)
                });
                (NatureSysteme::DoubleFlux(echangeur), PressionReseau::Haute)
            }
            // Éq. (162) à (164) ; Kres = 0 (tableau 59, « cas de l'aération et du BBio »).
            SystemeVentilation::Aeration => (NatureSysteme::Aeration, PressionReseau::Haute),
            // §6.6 : Kres du tableau 59 ; les fuites sont celles du conduit équivalent.
            SystemeVentilation::NaturelleParConduits(_) | SystemeVentilation::Hybride(_) => {
                (NatureSysteme::Naturelle, PressionReseau::Haute)
            }
            _ => return None,
        };
        let aeration = nature == NatureSysteme::Aeration;
        // §6.12 : rafraîchissement par surventilation mécanique.
        let surventilation = v
            .systeme
            .surventilation()
            .map(SurventilationResolue::nouvelle);
        let surv = surventilation.is_some();
        // §6.11.3 : puits climatique sur l'air neuf (le système est un simple flux par
        // insufflation, un double flux ou une CTA, contrôlé par la validation).
        let puits = v
            .puits_climatique
            .filter(|_| v.puits == Some(crate::entree::Puits::Climatique))
            .map(EtatPuits::nouveau);

        // §6.2.3.4, tableau 59 : coefficient de fuite.
        let k_res = if aeration {
            K_RES_BBIO
        } else {
            k_res(v.reseau.classe_etancheite)
        };
        if v.reseau.classe_etancheite.is_none() && !aeration {
            valeurs_par_defaut.push(defaut(
                "ventilation.reseau.classe_etancheite".to_owned(),
                K_RES_DEFAUT,
                "Th-BCE 2020 §6.2.3.4, tableau 59 : Kres de la valeur par défaut « DEF », en \
                 m³/(s.m²) sous 1 Pa",
            ));
        }
        let d_p_pa = typologie.d_p_pa(pression);
        // Surfaces saisies (Is_surf_cond_def = 0) si toutes les bouches en portent une.
        let surfaces_saisies = aeration
            || zone
                .groupes
                .iter()
                .flat_map(|g| &g.bouches_conduits)
                .all(|b| b.a_cond_m2.is_some());
        let ratfuitevc = match v.reseau.ratfuitevc {
            _ if aeration => 0.0,
            Some(r) => r,
            None => {
                let r = typologie.ratfuitevc_defaut();
                valeurs_par_defaut.push(defaut(
                    "ventilation.reseau.ratfuitevc".to_owned(),
                    r,
                    "Th-BCE 2020 §6.2.3.4, tableau 60 : valeur par défaut",
                ));
                r
            }
        };

        let mut bouches = Vec::new();
        for (ig, g) in zone.groupes.iter().enumerate() {
            // Premier passage : débits de base de chaque bouche, pour la répartition de la
            // surface par défaut en maison individuelle (note de l'éq. 423).
            let mut provisoires = Vec::with_capacity(g.bouches_conduits.len());
            for (ib, b) in g.bouches_conduits.iter().enumerate() {
                let chemin = format!("groupes[{ig}].bouches_conduits[{ib}]");
                let dugd_h = b.dugd_h.unwrap_or_else(|| {
                    valeurs_par_defaut.push(defaut(
                        format!("{chemin}.dugd_h"),
                        DUGD_DEFAUT_H,
                        "Th-BCE 2020 §6.2.3.2.2.2, tableau 57 : dispositif à gestion manuelle \
                         (par défaut)",
                    ));
                    DUGD_DEFAUT_H
                });
                // §6.2.3.3.1, note 1 : « Dans le cas de l'aération, Cdep = Cfenb ».
                let cdep = if aeration { Some(CFENB) } else { b.cdep }.unwrap_or_else(|| {
                    valeurs_par_defaut.push(defaut(
                        format!("{chemin}.cdep"),
                        CDEP_DEFAUT,
                        "Th-BCE 2020 §6.2.3.3.1, tableau 58 : valeur par défaut",
                    ));
                    CDEP_DEFAUT
                });
                let (q_max, q_base, q_dep, q_dep_inocc) = if non_residentiel {
                    // Fiche 6.2, §6.2.3.2.2.3 : `Crdbnr = 1` en présence de recyclage (débit de
                    // zone neutre inférieur au débit du mode actif, lecture Q92).
                    let recyclage = b
                        .q_spec_ch_m3_h
                        .zip(b.q_spec_occ_m3_h)
                        .is_some_and(|(ch, occ)| occ < ch);
                    // §6.2.3.2.1 : « Dans le cas particulier de l'aération, Crdbnr = 1 quelle
                    // que soit l'occupation ».
                    let crdbnr = if recyclage || aeration {
                        CRDBNR_BBIO
                    } else {
                        b.crdbnr.unwrap_or_else(|| {
                            valeurs_par_defaut.push(defaut(
                                format!("{chemin}.crdbnr"),
                                CRDBNR_DEFAUT,
                                "Th-BCE 2020 §6.2.3.2.1, tableau 57 : aucune régulation des \
                                 débits",
                            ));
                            CRDBNR_DEFAUT
                        })
                    };
                    let q_inocc = b.q_spec_inocc_m3_h?;
                    let (q_max, q_dep, q_dep_inocc) = debits_dep_non_residentiel(
                        b.isouf,
                        b.q_spec_occ_m3_h?,
                        q_inocc,
                        crdbnr,
                        cdep,
                    );
                    (q_max, q_inocc, q_dep, Some((q_dep_inocc, crdbnr)))
                } else {
                    let (q_max, q_base, q_dep) = debits_dep_habitation(b, dugd_h, cdep)?;
                    (q_max, q_base, q_dep, None)
                };
                provisoires.push((b, dugd_h, q_max, q_base, q_dep, q_dep_inocc, cdep));
            }
            let base_totale = |isouf: Isouf| {
                provisoires
                    .iter()
                    .filter(|p| p.0.isouf == isouf)
                    .map(|p| p.3)
                    .sum::<f64>()
            };
            let nb_circuit =
                |isouf: Isouf| provisoires.iter().filter(|p| p.0.isouf == isouf).count();
            for (ib, (b, dugd_h, q_max, q_base, q_dep, q_dep_inocc, cdep)) in
                provisoires.iter().copied().enumerate()
            {
                let a_cond_m2 = if surfaces_saisies {
                    b.a_cond_m2.unwrap_or(0.0)
                } else {
                    let mut recalculee = false;
                    let a = match typologie {
                        // Éq. (421) à (423), surface répartie au prorata des débits de base.
                        TypologieReseau::MaisonIndividuelle => {
                            let total = base_totale(b.isouf);
                            let part = if total > 0.0 {
                                q_base / total
                            } else {
                                1.0 / nb_circuit(b.isouf) as f64
                            };
                            a_cond_defaut_maison_m2(g.shab_m2.unwrap_or(0.0)) * part
                        }
                        // Éq. (424) à (427) ; éq. (760) et (761) en surventilation mécanique
                        // (lecture Q230 point 3).
                        TypologieReseau::BatimentCollectif | TypologieReseau::NonResidentiel => {
                            match b.q_spec_rafnoc_m3_h.filter(|_| surv) {
                                Some(q) => {
                                    recalculee = true;
                                    a_cond_rafnoc_m2(q_max, q)
                                }
                                None => a_cond_defaut_collectif_m2(q_max),
                            }
                        }
                    };
                    valeurs_par_defaut.push(defaut(
                        format!("groupes[{ig}].bouches_conduits[{ib}].a_cond_m2"),
                        a,
                        if recalculee {
                            "Th-BCE 2020 §6.2.3.4, éq. (421) à (427), tableau 60, et §6.12.3.1, \
                             éq. (760), (761) : surface de conduit par défaut, recalculée pour \
                             le débit de surventilation"
                        } else {
                            "Th-BCE 2020 §6.2.3.4, éq. (421) à (427), tableau 60 : surface de \
                             conduit par défaut"
                        },
                    ));
                    a
                };
                let debits = DebitsBoucheProjet {
                    isouf: b.isouf,
                    q_max_m3_h: q_max,
                    q_base_m3_h: q_base,
                    q_dep_m3_h: q_dep,
                    q_fuites_m3_h: q_fuites_m3_h(b.isouf, q_dep, k_res, a_cond_m2, d_p_pa),
                    ratfuitevc,
                    a_cond_m2,
                };
                let r = match b.isouf {
                    Isouf::Reprise => v.reseau.r_rep_m2k_w,
                    Isouf::Soufflage => v.reseau.r_souf_m2k_w,
                }
                .unwrap_or(0.0);
                // Hors habitation, débits de l'inoccupation : mêmes conduits, fuites nulles si le
                // débit est nul (éq. 419 et 420).
                let avec_q = |q_dep: f64| DebitsBoucheProjet {
                    q_dep_m3_h: q_dep,
                    q_fuites_m3_h: q_fuites_m3_h(b.isouf, q_dep, k_res, a_cond_m2, d_p_pa),
                    ..debits
                };
                let debits_inocc = q_dep_inocc.map(|(q, _)| avec_q(q));
                // Centrale de traitement d'air : débits du mode actif (fiche 6.2, éq. 391, 394,
                // 406, 410, 415 et 416 ; `Crdbnr = 1` en inoccupation) et taux d'air neuf
                // minimaux (éq. 393, 397, 408 et 411).
                let (debits_ch, taux) = match (&v.systeme, b.q_spec_ch_m3_h, q_dep_inocc) {
                    (SystemeVentilation::Cta(_), Some(q_ch), Some((_, crdbnr))) => {
                        let signe = match b.isouf {
                            Isouf::Reprise => -1.0,
                            Isouf::Soufflage => 1.0,
                        };
                        let taux = match b.isouf {
                            Isouf::Soufflage => (
                                taux_an_min(b.q_spec_occ_m3_h.unwrap_or(0.0), q_ch),
                                taux_an_min(b.q_spec_inocc_m3_h.unwrap_or(0.0), q_ch),
                            ),
                            Isouf::Reprise => (1.0, 1.0),
                        };
                        (
                            Some((
                                avec_q(signe * cdep * crdbnr * q_ch),
                                avec_q(signe * cdep * q_ch),
                            )),
                            taux,
                        )
                    }
                    _ => (None, (1.0, 1.0)),
                };
                // Éq. (758), (759) et (775) : débit de surventilation sans Cdep ni Crdbnr,
                // fuites du conduit comprises.
                let debits_rafnoc = b.q_spec_rafnoc_m3_h.filter(|_| surv).map(|q| {
                    let signe = match b.isouf {
                        Isouf::Reprise => -1.0,
                        Isouf::Soufflage => 1.0,
                    };
                    avec_q(signe * q)
                });
                bouches.push(BoucheProjet {
                    debits_rafnoc,
                    x_adiab: 0.0,
                    groupe: ig,
                    debits,
                    debits_inocc,
                    dugd_h,
                    cdep,
                    // Éq. (395), (650), (651), (666) et (667).
                    h_cond_ext_w_k: h_cond_ext_w_k(debits.a_cond_ext_m2(), r),
                    debits_ch,
                    taux_an_min: taux,
                });
            }
        }

        // Puissances moyennes des ventilateurs : éq. (390) à (394), (619), (620), (640), (641)
        // et (643) en habitation ; puissances en occupation et en inoccupation dans les autres
        // usages (éq. 636 à 639).
        let dugd = dugd_equ_h(bouches.iter().map(|b| b.dugd_h));
        let residentiel = |p: &crate::entree::VentilateurSimpleFlux| {
            p_vent_residentiel_w(
                p.p_vent_base_w.unwrap_or(0.0),
                p.p_vent_pointe_w.unwrap_or(0.0),
                dugd,
            )
        };
        let (p_vent_rep_w, p_vent_sou_w, p_vent_inocc_w) = match &v.systeme {
            SystemeVentilation::SimpleFluxExtraction(p) if non_residentiel => {
                (p.p_vent_occ_w?, 0.0, Some((p.p_vent_inocc_w?, 0.0)))
            }
            SystemeVentilation::SimpleFluxInsufflation(p) if non_residentiel => {
                (0.0, p.p_vent_occ_w?, Some((0.0, p.p_vent_inocc_w?)))
            }
            // Éq. (458) et (459) : double flux hors habitation.
            SystemeVentilation::DoubleFlux(df) if non_residentiel => (
                df.p_vent_rep_occ_w?,
                df.p_vent_sou_occ_w?,
                Some((df.p_vent_rep_inocc_w?, df.p_vent_sou_inocc_w?)),
            ),
            // Éq. (460) à (465) : puissances du mode actif restituées comme celles de
            // l'occupation ; celles de zone neutre en inoccupation pour l'inoccupation.
            SystemeVentilation::Cta(c) if non_residentiel => (
                c.p_vent_rep_ch_w,
                c.p_vent_sou_ch_w,
                Some((c.p_vent_rep_zn_inocc_w, c.p_vent_sou_zn_inocc_w)),
            ),
            // Aération : aucun ventilateur.
            SystemeVentilation::Aeration if non_residentiel => (0.0, 0.0, Some((0.0, 0.0))),
            SystemeVentilation::Aeration => (0.0, 0.0, None),
            // Ventilation naturelle ou hybride, habitation seulement : la consommation de
            // l'assistance se relève pas à pas (éq. 708).
            SystemeVentilation::NaturelleParConduits(_) | SystemeVentilation::Hybride(_)
                if !non_residentiel =>
            {
                (0.0, 0.0, None)
            }
            // Basse pression : habitation seulement ; CTA : hors habitation seulement.
            _ if non_residentiel => return None,
            SystemeVentilation::Cta(_) => return None,
            SystemeVentilation::SimpleFluxExtraction(p)
            | SystemeVentilation::MecaniqueBassePression(p) => (residentiel(p), 0.0, None),
            SystemeVentilation::SimpleFluxInsufflation(p) => (0.0, residentiel(p), None),
            SystemeVentilation::DoubleFlux(df) => (
                p_vent_residentiel_w(df.p_vent_base_rep_w?, df.p_vent_pointe_rep_w?, dugd),
                p_vent_residentiel_w(df.p_vent_base_sou_w?, df.p_vent_pointe_sou_w?, dugd),
                None,
            ),
            _ => return None,
        };
        let cta = match &v.systeme {
            SystemeVentilation::Cta(c) => Some(CtaResolue {
                type_cta: c.type_cta,
                p_vent_ch_w: (c.p_vent_rep_ch_w, c.p_vent_sou_ch_w),
                p_vent_zn_occ_w: (c.p_vent_rep_zn_occ_w, c.p_vent_sou_zn_occ_w),
                p_vent_zn_inocc_w: (c.p_vent_rep_zn_inocc_w, c.p_vent_sou_zn_inocc_w),
                theta_enc_enf_c: (c.theta_enc_c, c.theta_enf_c),
                regulation_air_neuf: c.regulation_air_neuf.unwrap_or_else(|| {
                    valeurs_par_defaut.push(defaut(
                        "ventilation.systeme.cta.regulation_air_neuf".to_owned(),
                        0.0,
                        "Th-BCE 2020 §6.3.2, tableau 61 et §6.3.3.4.3.3 : « Type_régul_AN », 0 = \
                         régulation standard (par défaut)",
                    ));
                    RegulationAirNeuf::Standard
                }),
                batteries_charge: batteries_charge(c),
            }),
            _ => None,
        };
        let nb_groupes = zone.groupes.len();
        // §6.6 : ventilation naturelle par conduits ou hybride.
        let naturelle = match v.systeme.conduit_naturel() {
            Some(n) if nature == NatureSysteme::Naturelle => {
                let reprise = |b: &&BoucheProjet| b.debits.isouf == Isouf::Reprise;
                let premiere = bouches.iter().find(reprise);
                let (dugd_h, cdep) =
                    premiere.map_or((DUGD_DEFAUT_H, CDEP_DEFAUT), |b| (b.dugd_h, b.cdep));
                let groupe = premiere.map_or(0, |b| b.groupe);
                let mut d_ou = |champ: &str, valeur: Option<f64>, defaut_v: f64, regle: &str| {
                    valeur.unwrap_or_else(|| {
                        valeurs_par_defaut.push(defaut(
                            format!("ventilation.systeme.{champ}"),
                            defaut_v,
                            regle,
                        ));
                        defaut_v
                    })
                };
                let h_zone_m = zone.h_zone_m.unwrap_or(0.0);
                let permea = permea_verticale(zone.usage.i_hebergement(), h_zone_m);
                let h_mot_m = d_ou(
                    "h_mot_m",
                    n.h_mot_m,
                    h_mot_defaut_m(permea, n.h_cond_m, h_zone_m),
                    "Th-BCE 2020 §6.6.3.1.5, éq. (699) et (700) : hauteur de tirage thermique par \
                     défaut",
                );
                let lambda_cond = d_ou(
                    "lambda_cond",
                    n.lambda_cond,
                    LAMBDA_COND_DEFAUT,
                    "Th-BCE 2020 §6.6.2, tableau 65 : « λcond », valeur conventionnelle",
                );
                let chi_coude = d_ou(
                    "chi_coude",
                    n.chi_coude,
                    CHI_COUDE_DEFAUT,
                    "Th-BCE 2020 §6.6.2, tableau 65 : « χcoude », valeur conventionnelle",
                );
                let d_pref_pa = d_ou(
                    "d_pref_pa",
                    n.d_pref_pa,
                    D_PREF_DEFAUT_PA,
                    "Th-BCE 2020 §6.6.2, tableau 65 : « dPref », valeur conventionnelle",
                );
                let v_vent_ref_m_s = d_ou(
                    "extracteur.v_vent_ref_m_s",
                    n.extracteur.v_vent_ref_m_s,
                    V_VENT_REF_DEFAUT_M_S,
                    "Th-BCE 2020 §6.6.2, tableau 65 : « Vvent,réf », valeur conventionnelle",
                );
                let a_bouche_eq = match &n.bouche {
                    crate::entree::BoucheNaturelle::Fixe {
                        a_sect_base_cm2,
                        a_sect_pointe_cm2,
                    } => a_bouche_eq_m2(dugd_h, *a_sect_pointe_cm2, *a_sect_base_cm2),
                    crate::entree::BoucheNaturelle::Autoreglable { .. } => 0.0,
                };
                let conduit = ConduitResolu {
                    nid: f64::from(n.nb_conduits_identiques),
                    per_cond_m: n.per_cond_m,
                    h_cond_m: n.h_cond_m,
                    h_mot_m,
                    a_cond_m2: n.a_cond_m2,
                    lambda_cond,
                    chi_coude,
                    d_pref_pa,
                    bouche: n.bouche.clone(),
                    a_bouche_eq_m2: a_bouche_eq,
                    c_extr: n.extracteur.c_extr,
                    xi_extr: n.extracteur.xi_extr,
                    v_vent_ref_m_s,
                    k_res,
                    ratfuitevc,
                    r_rep_m2k_w: v.reseau.r_rep_m2k_w.unwrap_or(0.0),
                    b: v.reseau.b,
                    cdep,
                    dugd_h,
                };
                // La surface de conduit des bouches ne sert pas : celle du conduit équivalent est
                // saisie (`a_cond_m2`), le défaut des éq. (421) à (427) n'est pas employé.
                valeurs_par_defaut.retain(|d| !d.champ.ends_with(".a_cond_m2"));
                // Éq. (706) : débit repris des bouches comme d'un simple flux standard.
                let q_rep_dep_standard_m3_h = bouches
                    .iter()
                    .filter(reprise)
                    .map(|b| b.debits.q_dep_m3_h)
                    .sum();
                Some(EtatNaturelle::nouveau(
                    conduit,
                    v.systeme.assistance().cloned(),
                    groupe,
                    typologie == TypologieReseau::BatimentCollectif,
                    q_rep_dep_standard_m3_h,
                ))
            }
            _ => None,
        };
        // §6.9 et §6.3.3.3.4 : puits hydraulique ; efficacité de la batterie déduite des
        // puissances déclarées aux débits d'air neuf de référence (éq. 399 à 407, `ρref` de la
        // fiche 6.3) ou valeur par défaut (éq. 408).
        let mut erreur_puits_h = None;
        let mut avertissement_puits_h = None;
        let puits_h = match (v.puits, v.puits_hydraulique) {
            (Some(crate::entree::Puits::Hydraulique), Some(d)) => {
                let qm_ref = |debit: &dyn Fn(&BoucheProjet) -> Option<DebitsBoucheProjet>| -> f64 {
                    bouches
                        .iter()
                        .filter_map(debit)
                        .filter(|x| x.isouf == Isouf::Soufflage)
                        .map(|x| qm_kg_s(RHO_REF_KG_M3, x.q_cond_m3_h()))
                        .sum()
                };
                // Éq. (399) et (400) : débit nominal en occupation, fuites comprises ; éq.
                // (403) à (405) : débit de surventilation si elle est synchronisée.
                let qm_hiv = qm_ref(&|b| Some(b.debits_ch.map_or(b.debits, |c| c.0)));
                let qm_ete = if d.surventilation_synchronisee {
                    qm_ref(&|b| b.debits_rafnoc)
                } else {
                    qm_hiv
                };
                let eps = match d.batterie {
                    Some(b) => efficacite_batterie_ch(b.p_ch_kw, qm_hiv).and_then(|ch| {
                        efficacite_batterie_fr(b.p_fr_kw, qm_ete).map(|fr| (ch, fr))
                    }),
                    None => {
                        valeurs_par_defaut.push(defaut(
                            "ventilation.puits_hydraulique.batterie".to_owned(),
                            EPS_BATT_DEFAUT,
                            "Th-BCE 2020 §6.3.3.3.4, éq. (408) : « Type_eff_echangeur_PH » = 0, \
                             efficacité par défaut de la batterie eau/air",
                        ));
                        Ok((EPS_BATT_DEFAUT, EPS_BATT_DEFAUT))
                    }
                };
                let (ch, fr) = eps.unwrap_or_else(|e| {
                    erreur_puits_h = Some(e);
                    (EPS_BATT_DEFAUT, EPS_BATT_DEFAUT)
                });
                // Éq. (415) lue à la lettre : avertir quand `DCair` dépasse `DCeau` (éq. 736 à 738).
                let dc_air = qm_hiv.max(qm_ete) * CP_AIR_PH_J_KG_K;
                let dc_eau = d.q_max_m3_h * RHO_EAU_GL_KG_M3 * CP_EAU_GL_J_KG_K / 3600.0;
                if dc_air > dc_eau {
                    avertissement_puits_h = Some(format!(
                        "le débit calorifique de l'air neuf ({dc_air:.1} W/K) dépasse celui de \
                         l'eau de la boucle ({dc_eau:.1} W/K) : l'énergie cédée à l'air par la \
                         batterie (éq. 415) peut dépasser ce que l'eau transporte ; revoir le \
                         débit de la pompe (Th-BCE 2020 §6.9.3.2.2, éq. 736 à 738)"
                    ));
                }
                Some(EtatPuitsHydraulique::nouveau(d, ch, fr))
            }
            _ => None,
        };
        // Relevé horaire : centrales double flux hors habitation, tout système à
        // surventilation mécanique (puissance variable d'heure en heure, éq. 779 et 780) et tout
        // système à puits hydraulique (pompe, éq. 748 et 749).
        let releve = (non_residentiel && v.systeme.nom_centrale().is_some()
            || surv
            || puits_h.is_some()
            || naturelle.as_ref().is_some_and(|n| n.assistance.is_some()))
        .then(|| ReleveVentilation {
            serie_wh: vec![0.0; NB_HEURES_AN],
            mensuel_groupes_kwh: vec![[0.0; 12]; nb_groupes],
            dernier_groupes_wh: vec![0.0; nb_groupes],
            dernier_wh: 0.0,
        });
        let rho_initiale = rho_kg_m3(theta_initiale_c);
        // §6.3.3.4.9.2 : humidificateur d'hiver ; ηhum conventionnel de 0,8 (éq. 530).
        let humidificateur = v
            .systeme
            .humidificateur()
            .map(|h| match h.type_humidification {
                TypeHumidificationHiver::Vapeur => TypeHumidificateur::Vapeur,
                TypeHumidificationHiver::Pulverisation => TypeHumidificateur::Pulverisation {
                    eta: h.eta_hum.unwrap_or_else(|| {
                        valeurs_par_defaut.push(defaut(
                            "ventilation.systeme.humidificateur.eta_hum".to_owned(),
                            ETA_HUM_CONV,
                            "Th-BCE 2020 §6.3.2, tableau 61 et éq. (530) : « ηhum », efficacité \
                         fixée conventionnellement à 80 %",
                        ));
                        ETA_HUM_CONV
                    }),
                },
            });
        let omega_cons_kg_kg = v
            .systeme
            .humidificateur()
            .map_or(0.0, |h| h.omega_cons_kg_kg);
        // §6.3.3.4.2 : rafraîchissement évaporatif ; rendement conventionnel de 0,8.
        let adiabatique = v.systeme.rafraichissement_adiabatique().map(|a| {
            let variable = cta
                .as_ref()
                .is_some_and(|c| c.type_cta == TypeCta::DebitVariable);
            AdiabatiqueResolu {
                indirect: a.type_rafraichissement == TypeRafraichissementEvaporatif::Indirect,
                eta: a.eta_hum.unwrap_or_else(|| {
                    valeurs_par_defaut.push(defaut(
                        "ventilation.systeme.rafraichissement_adiabatique.eta_hum".to_owned(),
                        ETA_HUM_CONV,
                        "Th-BCE 2020 §6.3.2, tableau 61 : « ηhum_dir », « ηhum_indir » \
                         conventionnels de 0,8",
                    ));
                    ETA_HUM_CONV
                }),
                theta_i_base_c: a.theta_i_base_c,
                consignes: match (variable, a.delta_theta_i1_k, a.delta_theta_i2_k) {
                    (true, Some(d1), Some(d2)) => {
                        Some(consignes_hysteresis_c(a.theta_i_base_c, d1, d2))
                    }
                    _ => None,
                },
            }
        });
        Some(Self {
            id: v.id.clone(),
            nature,
            b: v.reseau.b,
            rho_souffle: vec![rho_initiale; bouches.len()],
            rho_souffle_h1: vec![rho_initiale; bouches.len()],
            zn: ZoneNeutre {
                souffle: vec![(0.0, 0.0); bouches.len()],
                ..ZoneNeutre::default()
            },
            bouches,
            p_vent_rep_w,
            p_vent_sou_w,
            p_vent_inocc_w,
            nb_groupes,
            valeurs_par_defaut,
            cta,
            prechauffage: v.systeme.batterie_prechauffage().cloned(),
            prerefroidissement: v.systeme.batterie_prerefroidissement().cloned(),
            releve,
            isreport: [false; 2],
            humidificateur,
            omega_cons_kg_kg,
            adiabatique,
            theta_prec: vec![theta_initiale_c; nb_groupes],
            regime: Regime {
                i_vent: true,
                i_vent_h1: true,
                conditions_nominales: true,
                nominal: true,
                adiab: false,
                hysteresis: false,
                rafnoc: false,
                recalcul_puits: false,
            },
            surventilation,
            puits,
            puits_h,
            erreur_puits_h,
            avertissement_puits_h,
            naturelle,
        })
    }

    /// Système de ventilation naturelle par conduits ou hybride (§6.6).
    pub fn est_naturelle(&self) -> bool {
        self.naturelle.is_some()
    }

    /// §6.6 : débits du conduit, assistance mécanique et température de l'air repris au pas de
    /// temps `h`, avec la température et l'humidité de l'extérieur, la vitesse du vent corrigée
    /// et l'état du groupe desservi et la pression `Pib` du pas précédent (éq. 673 à 714). À
    /// appeler avant [`VentilationProjet::debits_groupe_pas`] ; sans effet hors ventilation
    /// naturelle ou hybride.
    pub fn pas_naturelle(
        &mut self,
        h: usize,
        theta_ext_c: f64,
        omega_ext_kg_kg: f64,
        v_vent_c_m_s: f64,
        p_ib_prec_pa: f64,
        groupes: &[EtatGroupeVentilation],
    ) {
        let Some(n) = &mut self.naturelle else { return };
        let Some(g) = groupes.get(n.groupe) else {
            return;
        };
        n.pas(&ConditionsPas {
            conduit: ConditionsConduit {
                theta_ext_c,
                rho_ext_kg_m3: rho_humide_kg_m3(theta_ext_c, omega_ext_kg_kg),
                v_vent_c_m_s,
                theta_int_prec_c: g.theta_i_fin_c,
                rho_int_prec_kg_m3: rho_humide_kg_m3(g.theta_i_fin_c, g.omega_i_fin_kg_kg),
                p_ib_prec_pa,
            },
            // Case de scénario de 1 à 24, échelle de `hgd` (tableau 67, Q236 point 2).
            h_leg: date_scenario(h).heure,
        });
        // Consommation de l'assistance mécanique, relevée pour le groupe desservi.
        let (w, groupe) = (n.c_vent_wh, n.groupe);
        if let Some(r) = &mut self.releve {
            r.dernier_wh = w;
            r.dernier_groupes_wh.fill(0.0);
            if let Some(x) = r.dernier_groupes_wh.get_mut(groupe) {
                *x = w;
            }
        }
    }

    /// Fixe le régime du pas de temps avant le calcul des débits des groupes : indicateur de
    /// ventilation de la zone et, pour une centrale de traitement d'air, débits nominaux ou de
    /// zone neutre (§6.3.3.4.3.2), selon l'état des groupes desservis au pas précédent.
    pub fn preparer_pas(&mut self, i_vent: bool, groupes: &[EtatGroupeVentilation]) {
        let i_vent_h1 = self.regime.i_vent;
        // §6.12.3.3 : surventilation mécanique régulée par
        // [`VentilationProjet::reguler_surventilation`] avant ce régime.
        let rafnoc = self.surventilation.is_some_and(|s| s.actif);
        // Note 1 du §6.3.3.1.4.1 : une relance ou un besoin d'un des groupes reliés suffit.
        let conditions = conditions_debit_nominal(
            i_vent,
            groupes.iter().any(|g| g.relance),
            i_vent_h1,
            groupes.iter().any(|g| g.besoin_ch),
            groupes.iter().any(|g| g.besoin_fr),
        ) || self.report_type_3();
        // §6.3.3.4.2 : autorisation du rafraîchissement évaporatif (`irafinocc_activé(h) = 0`) ;
        // profil à hystérésis des centrales à débit
        // variable dans les conditions du débit nominal (éq. 426 à 428 et 449 à 450).
        let adiab = self.adiabatique.is_some_and(|a| {
            aut_adiab(
                self.theta_moy_desservis_c(groupes),
                a.theta_i_base_c,
                aut_fr_centrale(groupes, |g| self.dessert(g)),
                rafnoc,
            )
        });
        let consignes = self.adiabatique.and_then(|a| a.consignes);
        let hysteresis = adiab && conditions && consignes.is_some();
        for b in &mut self.bouches {
            b.x_adiab = match (hysteresis, consignes, groupes.get(b.groupe)) {
                (true, Some(c), Some(g)) => position_hysteresis(
                    g.theta_i_fin_c,
                    self.theta_prec
                        .get(b.groupe)
                        .copied()
                        .unwrap_or(g.theta_i_fin_c),
                    b.x_adiab,
                    c,
                ),
                // Éq. (425), (428), (448) et (450) : débit de zone neutre.
                _ => 0.0,
            };
        }
        for (t, g) in self.theta_prec.iter_mut().zip(groupes) {
            *t = g.theta_i_fin_c;
        }
        self.regime = Regime {
            i_vent,
            i_vent_h1,
            conditions_nominales: conditions,
            nominal: self
                .cta
                .as_ref()
                .is_some_and(|c| debits_nominaux(c.type_cta, conditions)),
            adiab,
            hysteresis,
            rafnoc,
            recalcul_puits: false,
        };
    }

    /// Régulation du rafraîchissement par surventilation mécanique au pas de temps `h`
    /// (§6.12.3.2 et 6.12.3.3, éq. 762 à 774), à appeler avant
    /// [`VentilationProjet::preparer_pas`] ; sans effet sans surventilation. `theta_an_c` :
    /// « θan(h) », air neuf à la température extérieure (sans puits) ; `groupes` : état des
    /// groupes au pas précédent, dont seuls ceux que le système dessert comptent (« gr ↔ s »).
    pub fn reguler_surventilation(
        &mut self,
        h: usize,
        theta_an_c: f64,
        groupes: &[EtatGroupeVentilation],
    ) {
        let desservis: Vec<bool> = (0..groupes.len()).map(|g| self.dessert(g)).collect();
        let Some(s) = &mut self.surventilation else {
            return;
        };
        let theta_op = groupes
            .iter()
            .zip(&desservis)
            .filter(|(_, d)| **d)
            .map(|(g, _)| g.theta_op_fin_c);
        // Éq. (762), (764) et (765) : « MIN Autfr_pro^gr(j) » des groupes desservis, saison
        // propre sans égard pour `iclim` (lecture Q230 point 2).
        let aut_fr_pro = groupes
            .iter()
            .zip(&desservis)
            .filter(|(_, d)| **d)
            .all(|(g, _)| g.aut_fr_propre);
        s.reguler(h, theta_an_c, theta_op, aut_fr_pro);
    }

    /// Prépare le puits climatique du pas de temps `h` (fiches 6.8, 6.10 et 6.11) : température
    /// du sol, bypass selon la saison de chauffage de l'un des groupes desservis (éq. 755) et
    /// leur température moyenne à la fin du pas précédent (éq. 754, lecture Q231 point 4), puis
    /// `θan(h)` avec les débits d'air neuf de la fin du pas précédent. `te_c` : température
    /// extérieure horaire de l'année, pour la température du sol. Sans effet sans puits.
    pub fn preparer_puits(
        &mut self,
        h: usize,
        te_c: &[f64],
        theta_ext_c: f64,
        groupes: &[EtatGroupeVentilation],
    ) {
        let desservis: Vec<bool> = (0..groupes.len()).map(|g| self.dessert(g)).collect();
        let theta_i = self.theta_moy_desservis_c(groupes);
        let chauffage = groupes
            .iter()
            .zip(&desservis)
            .any(|(g, d)| *d && g.saison_de_chauffage);
        if let Some(p) = &mut self.puits {
            p.preparer(h, te_c, theta_ext_c, chauffage, theta_i);
        }
        if let Some(p) = &mut self.puits_h {
            p.preparer(h, te_c, theta_ext_c, chauffage);
        }
    }

    /// Le système a un puits, climatique ou hydraulique, sur son air neuf.
    pub fn a_un_puits(&self) -> bool {
        self.puits.is_some() || self.puits_h.is_some()
    }

    /// « θan(h) » du pas avec les débits d'air neuf de la fin du pas précédent, et le puits en
    /// service (non contourné, ou pompe en marche) ; `None` sans puits.
    fn puits_precedent(&self) -> Option<(f64, bool)> {
        match (&self.puits, &self.puits_h) {
            (Some(p), _) => Some((p.theta_an_c(), !p.contourne())),
            (_, Some(p)) => {
                let s = p.sortie();
                Some((s.theta_an_c, s.en_marche))
            }
            _ => None,
        }
    }

    /// « θan(h) » recalculé au débit d'air neuf `qm_kg_s` (mode actif, éq. 487 à 491, ou premier
    /// pas), et puits en service ; `None` sans puits.
    fn puits_au_debit(&self, theta_ext_c: f64, qm_kg_s: f64) -> Option<(f64, bool)> {
        match (&self.puits, &self.puits_h) {
            (Some(p), _) => Some((p.sortie_c(theta_ext_c, qm_kg_s), !p.contourne())),
            (_, Some(p)) => {
                let s = p.sortie_pour(theta_ext_c, qm_kg_s);
                Some((s.theta_an_c, s.en_marche))
            }
            _ => None,
        }
    }

    /// Aucun débit n'est connu : le premier pas se calcule avec son propre débit d'air neuf.
    fn puits_sans_historique(&self) -> bool {
        match (&self.puits, &self.puits_h) {
            (Some(p), _) => p.sans_historique(),
            (_, Some(p)) => p.sans_historique(),
            _ => false,
        }
    }

    /// Retient le débit d'air neuf de la fin du pas pour `θan` du pas suivant (éq. 414, 415 et
    /// 658).
    fn retenir_debit_puits(&mut self, qm_kg_s: f64) {
        if let Some(p) = &mut self.puits {
            p.retenir_debit(qm_kg_s);
        }
        if let Some(p) = &mut self.puits_h {
            p.retenir_debit(qm_kg_s);
        }
    }

    /// « Wélec^PH,S(h) » : la pompe tourne et l'air neuf circule (éq. 748 ; éq. 749 : la totalité
    /// pour l'unique système du puits), en Wh sur le pas d'une heure.
    fn w_puits_wh(&self, en_marche: bool, qm_an_kg_s: f64) -> f64 {
        match &self.puits_h {
            // Écart au texte : la condition « qm_an > 0 » n'est pas écrite à l'éq. (748) ; elle
            // évite la division par zéro de l'éq. (749) (répartition de la consommation de la
            // pompe au prorata des débits d'air neuf, nuls quand l'air neuf ne circule pas).
            // Q233 point 5 ; sans effet quand l'air neuf circule.
            Some(p) if en_marche && qm_an_kg_s > 0.0 => p.description.p_max_w,
            _ => 0.0,
        }
    }

    /// « θan(h) » : température de l'air neuf du pas, avec le puits climatique (éq. 414), ou
    /// `theta_ext_c` sans puits.
    pub fn theta_an_c(&self, theta_ext_c: f64) -> f64 {
        self.puits_precedent().map_or(theta_ext_c, |(t, _)| t)
    }

    /// « θi,moy » des groupes desservis par la centrale au pas précédent, pour l'autorisation
    /// du rafraîchissement évaporatif (§6.3.3.4.2 : « température moyenne des groupes reliés
    /// à la centrale ») : moyenne arithmétique (lecture Q96 point 3).
    fn theta_moy_desservis_c(&self, groupes: &[EtatGroupeVentilation]) -> f64 {
        let (n, somme) = groupes
            .iter()
            .enumerate()
            .filter(|(g, _)| self.dessert(*g))
            .fold((0.0, 0.0), |(n, s), (_, x)| (n + 1.0, s + x.theta_i_fin_c));
        if n > 0.0 {
            somme / n
        } else {
            f64::NEG_INFINITY
        }
    }

    /// §6.3.3.1.4 (p. 355) : une centrale de type 3 passe aussi au débit nominal lorsque la
    /// génération de ses batteries était en report d'énergie au pas précédent
    /// (`isreport_ch^g(h−1) = 1` ou `isreport_fr^g(h−1) = 1`), condition que les éq. (423) et
    /// (446) n'écrivent pas (lecture Q95 point 7).
    fn report_type_3(&self) -> bool {
        self.cta.as_ref().is_some_and(|c| {
            matches!(c.type_cta, TypeCta::DebitConstantTemperatureVariable)
                && c.batteries_charge.is_some()
        }) && (self.isreport[0] || self.isreport[1])
    }

    /// Le système a une bouche dans le groupe `g` (§8.11.3.2, éq. 938 : groupes connectés à
    /// la centrale).
    pub fn dessert(&self, g: usize) -> bool {
        self.bouches.iter().any(|b| b.groupe == g)
    }

    /// Régime du pas de temps en cours.
    pub fn regime(&self) -> Regime {
        self.regime
    }

    /// Régime d'une centrale hors simulation, pour la restitution : débit nominal en
    /// occupation, zone neutre en inoccupation (sans relance ni besoin).
    fn regime_statique(&self, i_vent: bool) -> Regime {
        Regime {
            i_vent,
            i_vent_h1: i_vent,
            conditions_nominales: i_vent,
            nominal: self
                .cta
                .as_ref()
                .is_some_and(|c| debits_nominaux(c.type_cta, i_vent)),
            adiab: false,
            hysteresis: false,
            rafnoc: false,
            recalcul_puits: false,
        }
    }

    /// Puissances des ventilateurs de reprise et de soufflage dans un régime, en W : éq. (460)
    /// à (465) pour une centrale de traitement d'air, (458) et (459) ou puissances constantes
    /// sinon.
    fn p_vent_regime(&self, regime: Regime) -> (f64, f64) {
        // Éq. (779) : puissances du rafraîchissement par surventilation mécanique (lecture Q230
        // point 5 pour les centrales à débit variable).
        if let Some(s) = self.surventilation.filter(|_| regime.rafnoc) {
            return s.p_vent_w;
        }
        match &self.cta {
            // Éq. (466) et (467) : centrale à débit variable en rafraîchissement évaporatif.
            Some(c) if regime.hysteresis => {
                let r = |isouf: Isouf| {
                    let (mut ecart, mut plage) = (0.0, 0.0);
                    for b in self.bouches.iter() {
                        // Éq. (467) : écarts aux débits de zone neutre en occupation.
                        let zn = b.debits;
                        let Some((occ, inocc)) = b.debits_ch else {
                            continue;
                        };
                        if zn.isouf != isouf {
                            continue;
                        }
                        let ch = if regime.i_vent { occ } else { inocc };
                        let d = (ch.q_cond_m3_h() - zn.q_cond_m3_h()).abs();
                        ecart += b.x_adiab * d;
                        plage += d;
                    }
                    if plage > 0.0 {
                        ecart / plage
                    } else {
                        0.0
                    }
                };
                (
                    p_vent_adiab_w(c.p_vent_zn_occ_w.0, c.p_vent_ch_w.0, r(Isouf::Reprise)),
                    p_vent_adiab_w(c.p_vent_zn_occ_w.1, c.p_vent_ch_w.1, r(Isouf::Soufflage)),
                )
            }
            Some(c) => p_vent_cta_w(
                c.type_cta,
                regime.conditions_nominales,
                c.p_vent_ch_w,
                c.p_vent_zn_occ_w,
                c.p_vent_zn_inocc_w,
            ),
            None => match self.p_vent_inocc_w {
                Some(inocc) if !regime.i_vent => inocc,
                _ => (self.p_vent_rep_w, self.p_vent_sou_w),
            },
        }
    }

    /// Somme des débits soufflés maximaux d'un groupe, en m³/h : numérateur des éq. (941) et
    /// (942) (Th-BCE 2020 §8.11.3.4). Bouche reliée à une centrale de traitement d'air : débit
    /// de soufflage du mode actif en occupation (« qsoufflé,CH,max », éq. 942) ; autre bouche :
    /// le plus grand de ses débits d'occupation et d'inoccupation (« qsoufflé,max », éq. 941).
    pub fn q_souffle_max_groupe_m3_h(&self, g: usize) -> f64 {
        self.bouches
            .iter()
            .filter(|b| b.groupe == g)
            .filter_map(|b| {
                let d = match b.debits_ch {
                    Some((occ, _)) => occ,
                    None => {
                        let inocc = b.debits_inocc.unwrap_or(b.debits);
                        if inocc.q_spec_m3_h() > b.debits.q_spec_m3_h() {
                            inocc
                        } else {
                            b.debits
                        }
                    }
                };
                (d.isouf == Isouf::Soufflage).then(|| d.q_spec_m3_h())
            })
            .sum()
    }

    /// Débits spécifiques d'un groupe au pas de temps en cours (régime fixé par
    /// [`VentilationProjet::preparer_pas`]).
    pub fn debits_groupe_pas(&self, g: usize) -> Debits {
        // §6.6 : débits du conduit résolus pour le pas (éq. 707).
        if let Some(n) = &self.naturelle {
            return if g == n.groupe {
                n.debits
            } else {
                Debits::default()
            };
        }
        let d =
            self.bouches
                .iter()
                .filter(|b| b.groupe == g)
                .fold(Debits::default(), |mut d, b| {
                    let debits = self.debits_pas(b, self.regime.i_vent);
                    match debits.isouf {
                        Isouf::Reprise => d.q_rep_m3_h += debits.q_spec_m3_h(),
                        Isouf::Soufflage => d.q_souffle_m3_h += debits.q_spec_m3_h(),
                    }
                    d
                });
        self.selon_nature(d)
    }

    /// Éq. (163) : en aération, le débit repris d'un groupe est l'opposé de son débit soufflé
    /// (les bouches de reprise ne sont pas comptées) ; sans effet sur les autres systèmes.
    fn selon_nature(&self, mut d: Debits) -> Debits {
        if self.nature == NatureSysteme::Aeration {
            d.q_rep_m3_h = -d.q_souffle_m3_h;
        }
        d
    }

    /// Débits d'une bouche au pas de temps : selon le régime de la centrale de traitement d'air,
    /// selon `Ivent` pour les autres systèmes.
    fn debits_pas(&self, b: &BoucheProjet, i_vent: bool) -> DebitsBoucheProjet {
        if self.cta.is_some() || self.regime.rafnoc {
            b.debits_regime(self.regime)
        } else {
            b.debits_selon(i_vent)
        }
    }

    /// Reporte la consommation du dernier pas de temps calculé dans le relevé (sans effet sans
    /// relevé). À appeler une fois par pas de temps `h`, après [`VentilationProjet::pas`].
    pub fn enregistrer(&mut self, h: usize) {
        let Some(r) = &mut self.releve else { return };
        let m = usize::from(date_scenario(h).mois).saturating_sub(1);
        if let Some(x) = r.serie_wh.get_mut(h) {
            *x = r.dernier_wh;
        }
        for (cumul, w) in r.mensuel_groupes_kwh.iter_mut().zip(&r.dernier_groupes_wh) {
            if let Some(x) = cumul.get_mut(m) {
                *x += w / 1000.0;
            }
        }
    }

    /// Vrai pour une aération par ouverture des fenêtres (§6.7) : les débits repris sont alors
    /// l'opposé des débits soufflés (éq. 163) et l'air soufflé est un air neuf entrant par les
    /// baies (éq. 256).
    pub fn est_aeration(&self) -> bool {
        self.nature == NatureSysteme::Aeration
    }

    /// Vrai si les débits et la puissance dépendent de l'indicateur de ventilation de la zone
    /// (usages autres que l'habitation).
    pub fn selon_occupation(&self) -> bool {
        self.p_vent_inocc_w.is_some()
    }

    /// « qspec_repris » et « qspec_soufflé » d'un groupe, en m³/h : sommes des débits
    /// spécifiques de ses bouches, fuites en volume chauffé comprises (éq. 428 et 429). Hors
    /// habitation, débits de l'occupation.
    pub fn debits_groupe(&self, g: usize) -> Debits {
        self.debits_groupe_selon(g, true)
    }

    /// Débits spécifiques d'un groupe selon l'indicateur de ventilation `Ivent` de la zone
    /// (sans effet en habitation).
    pub fn debits_groupe_selon(&self, g: usize, i_vent: bool) -> Debits {
        // §6.6 : débits moyens des pas calculés (restitution).
        if let Some(n) = &self.naturelle {
            return n.debits_moyens(g);
        }
        let d =
            self.bouches
                .iter()
                .filter(|b| b.groupe == g)
                .fold(Debits::default(), |mut d, b| {
                    let debits = b.debits_regime(self.regime_statique(i_vent));
                    match debits.isouf {
                        Isouf::Reprise => d.q_rep_m3_h += debits.q_spec_m3_h(),
                        Isouf::Soufflage => d.q_souffle_m3_h += debits.q_spec_m3_h(),
                    }
                    d
                });
        self.selon_nature(d)
    }

    /// Débits vus par les ventilateurs pour un groupe, en m³/h (éq. 430 et 431). Hors
    /// habitation, débits de l'occupation.
    pub fn debits_cond_groupe(&self, g: usize) -> Debits {
        self.debits_cond_groupe_selon(g, true)
    }

    /// Débits vus par les ventilateurs pour un groupe selon `Ivent`.
    pub fn debits_cond_groupe_selon(&self, g: usize, i_vent: bool) -> Debits {
        if let Some(n) = &self.naturelle {
            return n.debits_cond_moyens(g);
        }
        let d =
            self.bouches
                .iter()
                .filter(|b| b.groupe == g)
                .fold(Debits::default(), |mut d, b| {
                    let debits = b.debits_regime(self.regime_statique(i_vent));
                    match debits.isouf {
                        Isouf::Reprise => d.q_rep_m3_h += debits.q_cond_m3_h(),
                        Isouf::Soufflage => d.q_souffle_m3_h += debits.q_cond_m3_h(),
                    }
                    d
                });
        self.selon_nature(d)
    }

    /// Puissance moyenne totale des ventilateurs du système, en W. Hors habitation, puissance
    /// en occupation.
    pub fn p_vent_w(&self) -> f64 {
        if let Some(n) = &self.naturelle {
            return n.puissance_moyenne_w();
        }
        self.p_vent_rep_w + self.p_vent_sou_w
    }

    /// Puissances des ventilateurs de reprise et de soufflage selon `Ivent`, en W (éq. 636 à
    /// 639 hors habitation ; constantes en habitation).
    pub fn p_vent_selon(&self, i_vent: bool) -> (f64, f64) {
        if self.cta.is_some() {
            return self.p_vent_regime(self.regime_statique(i_vent));
        }
        match self.p_vent_inocc_w {
            Some(inocc) if !i_vent => inocc,
            _ => (self.p_vent_rep_w, self.p_vent_sou_w),
        }
    }

    /// Puissance des ventilateurs attribuée à un groupe, en W ; c'est aussi sa consommation
    /// horaire, en Wh, la puissance étant constante en habitation (éq. 392, 394, 641 et 643).
    /// Hors habitation, puissance en occupation.
    ///
    /// Simple flux par extraction et basse pression : au prorata des débits spécifiques repris
    /// (éq. 642) ; par insufflation : des débits spécifiques soufflés (éq. 644) ; double flux :
    /// des débits soufflés vus par le ventilateur (éq. 570, débits massiques lus en débits
    /// volumiques, `docs/questions/Q042.md`).
    pub fn p_vent_groupe_w(&self, g: usize) -> f64 {
        self.p_vent_groupe_selon_w(g, true)
    }

    /// Puissance des ventilateurs attribuée à un groupe selon `Ivent`, en W (éq. 642 et 644 :
    /// « en résidentiel et en non résidentiel », au prorata des débits spécifiques du pas de
    /// temps).
    ///
    /// Si tous les débits de l'inoccupation sont nuls alors que la puissance d'inoccupation ne
    /// l'est pas, le prorata n'est pas défini : la puissance est répartie au prorata des débits
    /// de l'occupation, pour que la somme des groupes reste la consommation du système
    /// (`docs/questions/Q060.md`).
    pub fn p_vent_groupe_selon_w(&self, g: usize, i_vent: bool) -> f64 {
        let (p_rep, p_sou) = self.p_vent_selon(i_vent);
        let p_vent_w = p_rep + p_sou;
        let somme = |f: &dyn Fn(&Debits) -> f64, cond: bool| {
            let pour = |etat: bool| {
                let de = |i: usize| {
                    if cond {
                        self.debits_cond_groupe_selon(i, etat)
                    } else {
                        self.debits_groupe_selon(i, etat)
                    }
                };
                let total: f64 = (0..self.nb_groupes).map(|i| f(&de(i))).sum();
                (f(&de(g)), total)
            };
            let (q, total) = pour(i_vent);
            if total == 0.0 && !i_vent {
                pour(true)
            } else {
                (q, total)
            }
        };
        match self.nature {
            NatureSysteme::SimpleFluxExtraction | NatureSysteme::BassePression => {
                let (q, total) = somme(&|d| d.q_rep_m3_h, false);
                p_vent_groupe_w(p_vent_w, q, total)
            }
            NatureSysteme::SimpleFluxInsufflation => {
                let (q, total) = somme(&|d| d.q_souffle_m3_h, false);
                p_vent_groupe_w(p_vent_w, q, total)
            }
            NatureSysteme::DoubleFlux(_) => {
                let (q, total) = somme(&|d| d.q_souffle_m3_h, true);
                p_vent_groupe_w(p_vent_w, q, total)
            }
            NatureSysteme::Aeration => 0.0,
            // §6.6 : puissance moyenne de l'assistance mécanique, pour le seul groupe desservi.
            NatureSysteme::Naturelle => self
                .naturelle
                .as_ref()
                .filter(|n| n.groupe == g)
                .map_or(0.0, |n| n.puissance_moyenne_w()),
        }
    }

    /// Circuits de reprise et de soufflage d'un système jusqu'au ventilateur de soufflage
    /// (éq. 412 à 520, 647 à 663) : employé par le pas de temps en zone neutre (première phase,
    /// [`VentilationProjet::pas`]) et par le recalcul du mode actif (§6.3.3.5.2 à 6.3.3.5.7,
    /// [`VentilationProjet::mode_actif`]), qui en change les températures des groupes, les
    /// débits et les puissances des ventilateurs.
    #[allow(clippy::too_many_arguments)]
    fn centrale(
        &self,
        theta_ext_c: f64,
        omega_ext_kg_kg: f64,
        groupes: &[EtatGroupeVentilation],
        regime: Regime,
        i_vent: bool,
        p_vent_w: (f64, f64),
        rho_souffle: &[f64],
    ) -> Centrale {
        let est_cta = self.cta.is_some();
        let deb = |b: &BoucheProjet| {
            if est_cta || regime.rafnoc {
                b.debits_regime(regime)
            } else {
                b.debits_selon(i_vent)
            }
        };
        let (p_vent_rep_w, p_vent_sou_w) = p_vent_w;
        let b = self.b;
        let t_lnc = |g: usize| t_lnc_c(b, groupes[g].theta_i_fin_c, theta_ext_c);

        // Circuit de reprise : pertes des conduits et ventilateur d'extraction (éq. 469 à 475,
        // 647 à 655). Débits massiques à la masse volumique du groupe au pas précédent (éq. 412,
        // 443, 656).
        let mut qm_rep = 0.0;
        let mut somme_t_rep = 0.0;
        let mut somme_theta_i_rep = 0.0;
        let mut somme_omega_rep = 0.0;
        for bouche in self
            .bouches
            .iter()
            .filter(|b| deb(b).isouf == Isouf::Reprise)
        {
            let theta_i = groupes[bouche.groupe].theta_i_fin_c;
            let rho = rho_kg_m3(theta_i);
            let lnc = t_lnc(bouche.groupe);
            let q_cond = deb(bouche).q_cond_m3_h();
            let qm = qm_kg_s(rho, q_cond).abs();
            let theta_cond = theta_i_cond_c(deb(bouche).q_spec_m3_h(), q_cond, theta_i, lnc);
            let t_extr1 = t_apres_conduit_c(theta_cond, lnc, bouche.h_cond_ext_w_k, qm);
            qm_rep += qm;
            somme_t_rep += qm * t_extr1;
            somme_theta_i_rep += qm * theta_i;
            somme_omega_rep += qm * groupes[bouche.groupe].omega_i_fin_kg_kg;
        }
        // Éq. (472) et (473).
        let (t_extr1, theta_i_moy, omega_extr) = if qm_rep > 0.0 {
            (
                somme_t_rep / qm_rep,
                somme_theta_i_rep / qm_rep,
                somme_omega_rep / qm_rep,
            )
        } else {
            (theta_ext_c, theta_ext_c, omega_ext_kg_kg)
        };

        // Circuit de soufflage : débits massiques (éq. 420 à 425) et, pour une centrale de
        // traitement d'air, air neuf minimal (éq. 433 et 434).
        let mut qm_sou = 0.0;
        let mut qm_an_min = 0.0;
        for (bouche, rho) in self.bouches.iter().zip(rho_souffle) {
            let d = deb(bouche);
            if d.isouf != Isouf::Soufflage {
                continue;
            }
            let qm = qm_kg_s(*rho, d.q_cond_m3_h());
            qm_sou += qm;
            qm_an_min += if regime.nominal {
                let (occ, inocc) = bouche.taux_an_min;
                qm * if regime.i_vent { occ } else { inocc }
            } else {
                qm
            };
        }
        // Air neuf : air extérieur sans puits (éq. 413 et 657) ; avec un puits climatique, sortie
        // du puits avec les débits d'air neuf de la fin du pas précédent (éq. 414 et 658).
        let precedent = self.puits_precedent();
        let theta_an_prec = precedent.map_or(theta_ext_c, |(t, _)| t);
        // §6.3.3.4.5.3 : rafraîchissement évaporatif indirect autorisé (« Aut_adiab = 1 » et
        // « Type_Humidification_Ete = 2 ») ; pas de recyclage (éq. 440), pas de bypass (éq. 493).
        let indirect = regime.adiab && self.adiabatique.is_some_and(|a| a.indirect);
        // Éq. (432) et (435) à (440).
        // Éq. (777) : tout air neuf en surventilation mécanique.
        let qm_an = match &self.cta {
            Some(_) if indirect || regime.rafnoc => qm_sou,
            Some(c) => qm_air_neuf_kg_s(
                qm_an_min,
                qm_sou,
                theta_an_prec,
                c.theta_enc_enf_c.0,
                c.theta_enc_enf_c.1,
                c.regulation_air_neuf,
                theta_i_moy,
            ),
            None => qm_sou,
        };
        // Éq. (488) : en mode actif, `θan` est recalculée avec les nouveaux débits d'air neuf ;
        // au tout premier pas, aucun débit n'est connu : le débit du pas sert (Q231 point 3).
        let (theta_an, puits_en_marche) = match precedent {
            Some(_) if regime.recalcul_puits || self.puits_sans_historique() => self
                .puits_au_debit(theta_ext_c, qm_an)
                .unwrap_or((theta_ext_c, false)),
            Some(x) => x,
            None => (theta_ext_c, false),
        };
        // Éq. (454) à (456) : débit recyclé, débit repris et débit rejeté rééquilibrés. Le texte
        // (p. 372) écrit les débits repris en valeurs signées, négatives : `qm,rep,cond =
        // MIN(qm,rep,cond ; qm,air_neuf − qm,sou,cond)` et `qm,air_extrait = qm,rep,cond −
        // (qm,air_neuf − qm,sou,cond)`, avec `qm,air_neuf − qm,sou,cond = −qm,recyclé ≤ 0`. Ici
        // les débits repris sont en valeurs absolues (`qm_rep ≥ 0`) : le MIN de deux négatifs
        // devient le MAX des valeurs absolues, `|qm,rep,cond| = MAX(|qm,rep,cond| ; qm,recyclé)`,
        // et `|qm,air_extrait| = |qm,rep,cond| − qm,recyclé` ; les deux formes sont équivalentes.
        let qm_recycle = if est_cta {
            (qm_sou - qm_an).max(0.0)
        } else {
            0.0
        };
        let (qm_rep_cond, qm_extrait) = if qm_recycle > 0.0 {
            let q = qm_rep.max(qm_recycle);
            (q, q - qm_recycle)
        } else {
            (qm_rep, qm_rep)
        };
        let t_extr2 = t_apres_ventilateur_c(t_extr1, p_vent_rep_w, qm_rep_cond);
        // Éq. (478) à (486) : humidification indirecte de l'air extrait ; sinon `θextr3 =
        // θextr2` (éq. 484 et 485).
        let (t_extr3, qm_eau_ete) = match self.adiabatique {
            Some(a) if indirect => {
                let x = humidificateur_adiabatique(t_extr2, omega_extr, a.eta);
                (
                    x.theta_c,
                    qm_rep_cond * (x.omega_kg_kg - omega_extr).max(0.0),
                )
            }
            _ => (t_extr2, 0.0),
        };
        let mut c = Centrale {
            sorties: PasVentilation {
                theta_air_extrait_c: t_extr2,
                qm_air_extrait_kg_s: qm_extrait,
                theta_souf_mel_c: theta_an,
                taux_an: if qm_sou > 0.0 { qm_an / qm_sou } else { 1.0 },
                theta_an_c: self.a_un_puits().then_some(theta_an),
                puits_actif: puits_en_marche && qm_an > 0.0,
                w_puits_wh: self.w_puits_wh(puits_en_marche, qm_an),
                q_puits_wh: if self.a_un_puits() {
                    C_PA_J_KG_K * qm_an * (theta_an - theta_ext_c)
                } else {
                    0.0
                },
                ..PasVentilation::default()
            },
            qm_sou_kg_s: qm_sou,
            qm_an_kg_s: qm_an,
            theta_souf2_c: theta_an,
            omega_souf2_kg_kg: omega_ext_kg_kg,
            w_ech_wh: 0.0,
            qm_eau_ete_kg_s: qm_eau_ete,
        };
        if qm_sou <= 0.0 {
            return c;
        }
        let sorties = &mut c.sorties;
        let (theta_souf1, epsilon) = match &self.nature {
            NatureSysteme::DoubleFlux(echangeur) => {
                let saison_de_chauffage = groupes.iter().any(|g| g.saison_de_chauffage);
                let (theta_souf_ag, epsilon) = match echangeur {
                    None => (theta_an, 0.0),
                    Some(e) => {
                        // Éq. (492) à (495) : bypass, avec la température des groupes repris.
                        // Éq. (776) : bypass forcé en surventilation mécanique, que l'échangeur
                        // en ait un ou non (lecture Q230 point 4).
                        let bypass = regime.rafnoc
                            || !indirect
                                && e.bypass.as_ref().is_some_and(|by| {
                                    delta_bypass(by, saison_de_chauffage, theta_an, theta_i_moy)
                                });
                        sorties.bypass = bypass;
                        // Éq. (496) à (505), avec les débits d'air neuf et d'air rejeté
                        // (éq. 499).
                        let epsilon = if bypass {
                            0.0
                        } else {
                            c.w_ech_wh = e.p_ech_w;
                            match e.efficacite {
                                EfficaciteEchangeur::Simplifiee(eps) => eps,
                                EfficaciteEchangeur::Detaillee { ua_w_k, ecoulement } => {
                                    epsilon_detaille(
                                        ua_w_k,
                                        ecoulement,
                                        C_PA_J_KG_K * qm_an,
                                        C_PA_J_KG_K * qm_extrait,
                                    )
                                }
                            }
                        };
                        // Éq. (778) : antigel coupé en surventilation mécanique (échangeur
                        // en bypass, ε = 0).
                        if regime.rafnoc {
                            (theta_an, epsilon)
                        } else {
                            // Éq. (506) à (511), avec θextr3.
                            let a = antigel(
                                epsilon,
                                e.antigel,
                                saison_de_chauffage,
                                theta_an,
                                t_extr3,
                                e.theta_s_ech_rep_lim_c,
                                qm_an,
                            );
                            sorties.w_ag_wh = a.w_ag_wh;
                            (a.theta_souf_ag_c, a.epsilon)
                        }
                    }
                };
                // Éq. (512) et (513).
                let (theta_souf1, theta_rejete) =
                    sorties_echangeur_c(theta_souf_ag, t_extr3, epsilon);
                sorties.theta_air_extrait_c = theta_rejete;
                (theta_souf1, epsilon)
            }
            _ => (theta_an, 0.0),
        };
        sorties.epsilon = epsilon;
        // Éq. (515) à (518) : boîte de mélange, sans effet sans recyclage.
        let (theta_mel, omega_mel) = if qm_recycle > 0.0 {
            (
                melange(qm_sou, qm_an, t_extr2, theta_souf1),
                melange(qm_sou, qm_an, omega_extr, omega_ext_kg_kg),
            )
        } else {
            (theta_souf1, omega_ext_kg_kg)
        };
        sorties.theta_souf_mel_c = theta_mel;
        // Éq. (519) et (520), (663) : ventilateur de soufflage.
        c.theta_souf2_c = t_apres_ventilateur_c(theta_mel, p_vent_sou_w, qm_sou);
        c.omega_souf2_kg_kg = omega_mel;
        c
    }

    /// Calcule un pas de temps du système : air soufflé dans chaque groupe (`air`, un élément
    /// par groupe) et sorties du système.
    ///
    /// `groupes` donne l'état de chaque groupe à la fin du pas de temps précédent. Un groupe
    /// sans bouche de soufflage reçoit un air soufflé à la température et à l'humidité
    /// extérieures, sans emploi puisque son débit soufflé est nul. `i_vent` est l'indicateur
    /// de ventilation de la zone (§4.1.3, éq. 45), sans effet en habitation ; une centrale de
    /// traitement d'air suit le régime fixé par [`VentilationProjet::preparer_pas`].
    pub fn pas(
        &mut self,
        theta_ext_c: f64,
        omega_ext_kg_kg: f64,
        groupes: &[EtatGroupeVentilation],
        air: &mut [AirSouffle],
        i_vent: bool,
    ) -> PasVentilation {
        let regime = self.regime;
        let est_cta = self.cta.is_some();
        let deb = |b: &BoucheProjet| {
            if est_cta || regime.rafnoc {
                b.debits_regime(regime)
            } else {
                b.debits_selon(i_vent)
            }
        };
        let (p_vent_rep_w, p_vent_sou_w) = if est_cta || regime.rafnoc {
            self.p_vent_regime(regime)
        } else {
            self.p_vent_selon(i_vent)
        };
        for a in air.iter_mut() {
            *a = AirSouffle {
                t_air_souffle_c: theta_ext_c,
                // Éq. (514), (563) et (669) : humidité de l'air extérieur.
                omega_air_souffle_kg_kg: omega_ext_kg_kg,
            };
        }
        // §6.7.3 : en aération, l'air neuf fourni par la fenêtre est à la température et à
        // l'humidité extérieures (éq. 716 et 718) ; la température de sortie d'un espace tampon
        // (éq. 715) est substituée par la zone, qui la connaît. Ni centrale ni ventilateur.
        // §6.6 : en refoulement l'air soufflé est à la température et à l'humidité extérieures
        // (éq. 713 et 714), déjà posées ; ni centrale ni échangeur.
        if matches!(
            self.nature,
            NatureSysteme::Aeration | NatureSysteme::Naturelle
        ) {
            return PasVentilation::default();
        }
        let b = self.b;
        let t_lnc = |g: usize| t_lnc_c(b, groupes[g].theta_i_fin_c, theta_ext_c);
        let centrale = self.centrale(
            theta_ext_c,
            omega_ext_kg_kg,
            groupes,
            regime,
            i_vent,
            (p_vent_rep_w, p_vent_sou_w),
            &self.rho_souffle,
        );
        // Débit d'air neuf de la fin du pas, pour `θan` du pas suivant (éq. 414, 415 et 658).
        self.retenir_debit_puits(centrale.qm_an_kg_s);
        let mut sorties = centrale.sorties;
        sorties.adiab = regime.adiab;
        sorties.rafnoc = regime.rafnoc;
        // Éq. (486) et (566) : pompe du rafraîchissement indirect.
        sorties.w_aux_adiab_wh = w_aux_pompe_wh(centrale.qm_eau_ete_kg_s);
        let w_ech_wh = centrale.w_ech_wh;
        let qm_sou = centrale.qm_sou_kg_s;
        self.zn.w_vent_wh = p_vent_rep_w + p_vent_sou_w;
        self.zn.w_ech_wh = w_ech_wh;
        self.zn.w_ag_wh = sorties.w_ag_wh;
        self.zn.w_puits_wh = sorties.w_puits_wh;
        self.zn.omega_souffle_kg_kg = omega_ext_kg_kg;
        self.zn.souffle.fill((0.0, 0.0));
        self.rho_souffle_h1.copy_from_slice(&self.rho_souffle);

        if qm_sou > 0.0 {
            let theta_souf2 = centrale.theta_souf2_c;
            let omega_mel = centrale.omega_souf2_kg_kg;
            // Éq. (521) à (525) : préchauffage en saison de chauffage.
            let saison_de_chauffage = groupes.iter().any(|g| g.saison_de_chauffage);
            // Éq. (778) : préchauffage, humidificateur d'hiver et prérefroidissement coupés en
            // surventilation mécanique (`Tair_soufflé = Tsouf2`).
            let traitement = !regime.rafnoc;
            let theta_pre = match &self.prechauffage {
                Some(p) if saison_de_chauffage && traitement => {
                    let (t, q) = prechauffage(
                        theta_souf2,
                        theta_ext_c,
                        p.theta_cons_c,
                        p.theta_ex_c,
                        qm_sou,
                    );
                    sorties.q_prechaud_wh = q;
                    t
                }
                _ => theta_souf2,
            };
            // Éq. (526) à (537) : humidificateur d'hiver en saison de chauffage.
            let (theta_hr, omega_hr) = match self.humidificateur {
                Some(t) if traitement => {
                    let x = humidification_hiver(
                        t,
                        saison_de_chauffage,
                        theta_pre,
                        omega_mel,
                        self.omega_cons_kg_kg,
                        qm_sou,
                    );
                    sorties.q_hr_wh = x.q_hr_wh;
                    // Éq. (568) et (569) : pompe d'un humidificateur à pulvérisation.
                    sorties.w_aux_chaudhr_wh = w_aux_pompe_wh(x.qm_eau_kg_s);
                    (x.theta_souf3_c, x.omega_souf3_kg_kg)
                }
                _ => (theta_pre, omega_mel),
            };
            // Éq. (538) à (548) : prérefroidissement en saison de refroidissement (éq. 410).
            let (theta_souf3, omega_souf3) = match &self.prerefroidissement {
                Some(p) if traitement && groupes.iter().any(|g| g.saison_de_refroidissement) => {
                    let r = prerefroidissement(
                        theta_hr,
                        omega_hr,
                        theta_ext_c,
                        p.theta_cons_c,
                        p.theta_ex_c,
                        p.theta_dim_fr_c,
                        qm_sou,
                    );
                    sorties.q_prefroid_wh = r.q_prefroid_wh;
                    (r.theta_souf3_c, r.omega_souf3_kg_kg)
                }
                _ => (theta_hr, omega_hr),
            };
            // Éq. (549) à (559) : rafraîchissement évaporatif direct, si l'air des groupes
            // desservis est assez sec (sur l'air de l'étape précédente, `θsouf2` sans
            // préchauffage ni humidificateur d'hiver ; lecture Q96 point 5).
            let (theta_souf3, omega_souf3) = match self.adiabatique {
                Some(a)
                    if regime.adiab
                        && !a.indirect
                        && groupes.iter().enumerate().all(|(g, x)| {
                            !self.dessert(g)
                                || groupe_assez_sec(x.theta_i_fin_c, x.omega_i_fin_kg_kg)
                        }) =>
                {
                    let x = humidificateur_adiabatique(theta_souf3, omega_souf3, a.eta);
                    // Éq. (559) et (566).
                    sorties.w_aux_adiab_wh =
                        w_aux_pompe_wh(qm_sou * (x.omega_kg_kg - omega_souf3).max(0.0));
                    (x.theta_c, x.omega_kg_kg)
                }
                _ => (theta_souf3, omega_souf3),
            };
            // Éq. (560) et (561) : sortie de la centrale.
            let theta_sortie_centrale = theta_souf3;
            self.zn.omega_souffle_kg_kg = omega_souf3;

            // Pertes des conduits de soufflage, bouche par bouche (éq. 562 et 664), température
            // de l'air soufflé dans chaque groupe (éq. 668), humidité (éq. 563), masse
            // volumique pour le pas suivant (éq. 564 et 670).
            let mut somme_q = vec![0.0; air.len()];
            let mut somme_qt = vec![0.0; air.len()];
            for ((bouche, rho), zn) in self
                .bouches
                .iter()
                .zip(self.rho_souffle.iter_mut())
                .zip(self.zn.souffle.iter_mut())
            {
                let d = deb(bouche);
                if d.isouf != Isouf::Soufflage {
                    continue;
                }
                let qm = qm_kg_s(*rho, d.q_cond_m3_h());
                let t = t_apres_conduit_c(
                    theta_sortie_centrale,
                    t_lnc(bouche.groupe),
                    bouche.h_cond_ext_w_k,
                    qm,
                );
                // Débit spécifique massique et température soufflée, pour les déperditions en
                // zone neutre du mode actif (éq. 571).
                *zn = (qm_kg_s(*rho, d.q_spec_m3_h()).max(0.0), t);
                *rho = rho_kg_m3(t);
                let q = d.q_spec_m3_h();
                if let (Some(sq), Some(sqt)) = (
                    somme_q.get_mut(bouche.groupe),
                    somme_qt.get_mut(bouche.groupe),
                ) {
                    *sq += q;
                    *sqt += q * t;
                }
            }
            for ((a, q), qt) in air.iter_mut().zip(&somme_q).zip(&somme_qt) {
                if *q > 0.0 {
                    a.t_air_souffle_c = qt / q;
                    a.omega_air_souffle_kg_kg = omega_souf3;
                }
            }
        }

        self.zn.w_aux_adiab_wh = sorties.w_aux_adiab_wh;
        // Éq. (565) et (570) : consommation des ventilateurs, de l'échangeur et des pompes
        // d'humidificateurs, répartie entre les groupes au prorata des débits soufflés vus par
        // le ventilateur (lecture Q42), ceux de l'occupation si aucun débit n'est soufflé
        // (lecture Q60).
        if self.releve.is_some() {
            let w = p_vent_rep_w
                + p_vent_sou_w
                + w_ech_wh
                + sorties.w_aux_adiab_wh
                + sorties.w_aux_chaudhr_wh
                + sorties.w_puits_wh;
            let r_pas = if est_cta {
                regime
            } else {
                Regime {
                    nominal: false,
                    rafnoc: regime.rafnoc,
                    ..self.regime_statique(i_vent)
                }
            };
            self.repartir_releve(w, r_pas);
        }
        sorties
    }

    /// Répartit la consommation `w_wh` du pas de temps entre les groupes au prorata des débits
    /// soufflés vus par le ventilateur dans le régime `r` (éq. 570 et 618), ceux de
    /// l'occupation si aucun débit n'est soufflé (lecture Q60).
    fn repartir_releve(&mut self, w_wh: f64, r: Regime) {
        // Simple flux par extraction et basse pression : au prorata des débits spécifiques
        // repris (éq. 642), par insufflation des débits spécifiques soufflés (éq. 644) ; double
        // flux : des débits soufflés vus par le ventilateur (éq. 570).
        let (circuit, cond) = match self.nature {
            NatureSysteme::SimpleFluxExtraction | NatureSysteme::BassePression => {
                (Isouf::Reprise, false)
            }
            NatureSysteme::SimpleFluxInsufflation => (Isouf::Soufflage, false),
            _ => (Isouf::Soufflage, true),
        };
        let parts = |r: Regime| -> Vec<f64> {
            (0..self.nb_groupes)
                .map(|g| {
                    self.bouches
                        .iter()
                        .filter(|b| b.groupe == g)
                        .map(|b| b.debits_regime(r))
                        .filter(|d| d.isouf == circuit)
                        .map(|d| {
                            if cond {
                                d.q_cond_m3_h()
                            } else {
                                d.q_spec_m3_h().abs()
                            }
                        })
                        .sum()
                })
                .collect()
        };
        let mut q = parts(r);
        if q.iter().sum::<f64>() <= 0.0 {
            q = parts(self.regime_statique(true));
        }
        let total: f64 = q.iter().sum();
        if let Some(rel) = &mut self.releve {
            rel.dernier_wh = w_wh;
            for (x, qg) in rel.dernier_groupes_wh.iter_mut().zip(&q) {
                *x = if total > 0.0 { w_wh * qg / total } else { 0.0 };
            }
        }
    }

    /// Seconde phase du pas de temps d'une centrale à charge variable (types 3 et 4) pourvue
    /// d'une batterie chaude ou froide régulée sur la charge : mode actif du §6.3.3.5, après le
    /// calcul des groupes (§6.3.3.1.5, figure 72).
    ///
    /// `zn` est l'état des groupes vu par la première phase (fin du pas précédent, saisons) ;
    /// `moy`, le même état au pas de temps en cours (`θi,moy^g(h)`, `ωi^g(h)`, lecture Q95) ;
    /// `q_sys_ch_wh` et `q_sys_fr_wh`, les sommes des demandes `Qsys_ch^g,s(h)` et
    /// `Qsys_fr^g,s(h)` des émetteurs contenus dans les bouches de soufflage (positive et
    /// négative ou nulle). Sans demande, ou hors saison, la centrale reste en zone neutre et
    /// rien n'est recalculé. Sinon, le circuit est recalculé aux débits et puissances du mode
    /// actif (éq. 572 à 581) avec les températures des groupes du pas en cours (§6.3.3.5.4 à
    /// 6.3.3.5.7), puis la température aux bouches (éq. 582 à 594), à la sortie de la centrale
    /// (éq. 595), l'énergie des batteries (éq. 596 à 612) et les ventilateurs (éq. 614), dont
    /// la consommation remplace celle de la zone neutre dans le relevé (éq. 618).
    ///
    /// La demande que le mode actif n'assure pas (chauffage et refroidissement au même pas
    /// dans des groupes différents, centrale sans débit soufflé) est rendue dans
    /// [`ModeActif::q_direct_wh`] pour être transmise à la génération de la batterie de même
    /// fonction (lecture Q95 point 8) : elle n'est jamais perdue.
    #[allow(clippy::too_many_arguments)]
    pub fn mode_actif(
        &mut self,
        theta_ext_c: f64,
        omega_ext_kg_kg: f64,
        zn: &[EtatGroupeVentilation],
        moy: &[EtatGroupeVentilation],
        i_vent: bool,
        q_sys_ch_wh: f64,
        q_sys_fr_wh: f64,
    ) -> ModeActif {
        let mut m = self.mode_actif_centrale(
            theta_ext_c,
            omega_ext_kg_kg,
            zn,
            moy,
            i_vent,
            q_sys_ch_wh,
            q_sys_fr_wh,
        );
        let ch_assure = m.actif && m.chauffage;
        let fr_assure = m.actif && !m.chauffage;
        m.q_direct_wh = (
            if ch_assure { 0.0 } else { q_sys_ch_wh.max(0.0) },
            if fr_assure { 0.0 } else { q_sys_fr_wh.min(0.0) },
        );
        m
    }

    #[allow(clippy::too_many_arguments)]
    fn mode_actif_centrale(
        &mut self,
        theta_ext_c: f64,
        omega_ext_kg_kg: f64,
        zn: &[EtatGroupeVentilation],
        moy: &[EtatGroupeVentilation],
        i_vent: bool,
        q_sys_ch_wh: f64,
        q_sys_fr_wh: f64,
    ) -> ModeActif {
        let Some((cta, batteries)) = self
            .cta
            .as_ref()
            .and_then(|c| c.batteries_charge.map(|b| (c.clone(), b)))
        else {
            return ModeActif::default();
        };
        // §6.12.3.4, note : en surventilation mécanique, les débits ne sont pas recalculés pour
        // un besoin de froid ; la centrale reste aux débits du rafraîchissement et la demande
        // des émetteurs passe directement à la génération (lecture Q230 point 6, Q95).
        if self.regime.rafnoc {
            return ModeActif::default();
        }
        // §6.3.3.1.5, figure 72 : saison effective de la centrale (éq. 409 et 410 : un groupe
        // suffit), demande des émetteurs ou report d'énergie de la génération de la batterie au
        // pas précédent (éq. 583, 584, 588 et 594 : `isreport^g(h−1) = 1`, lecture Q95 point 7).
        let [report_ch, report_fr] = self.isreport;
        let chauffe = batteries.chaud
            && zn.iter().any(|g| g.saison_de_chauffage)
            && (q_sys_ch_wh > 0.0 || report_ch);
        let refroidit = !chauffe
            && batteries.froid.is_some()
            && zn.iter().any(|g| g.saison_de_refroidissement)
            && (q_sys_fr_wh < 0.0 || report_fr);
        let report = if chauffe { report_ch } else { report_fr };
        if !chauffe && !refroidit {
            return ModeActif::default();
        }
        // Éq. (573), (577) et (581) : débits et puissances du mode actif ; §6.3.3.5.4 à
        // 6.3.3.5.7 : circuit recalculé avec les températures des groupes au pas en cours.
        let regime = Regime {
            nominal: true,
            recalcul_puits: true,
            ..self.regime
        };
        let centrale = self.centrale(
            theta_ext_c,
            omega_ext_kg_kg,
            moy,
            regime,
            i_vent,
            cta.p_vent_ch_w,
            &self.rho_souffle,
        );
        // Débits d'air neuf recalculés : ce sont ceux de la fin du pas (éq. 487 à 489).
        self.retenir_debit_puits(centrale.qm_an_kg_s);
        let qm_cond = centrale.qm_sou_kg_s;
        if qm_cond <= 0.0 {
            return ModeActif::default();
        }
        let cp = cp_air_humide_j_kg_k(centrale.omega_souf2_kg_kg);
        let theta_souf2 = centrale.theta_souf2_c;
        // Éq. (571) : déperditions ou apports de l'air soufflé en zone neutre (première phase).
        let cp_zn = cp_air_humide_j_kg_k(self.zn.omega_souffle_kg_kg);
        let mut p_zn = 0.0;
        for (bouche, (qm_spec, t)) in self.bouches.iter().zip(&self.zn.souffle) {
            p_zn += cp_zn * qm_spec * (t - moy[bouche.groupe].theta_i_fin_c);
        }
        // Éq. (574) et (576) : débits spécifiques recalculés ; θi,moy des groupes soufflés,
        // pondérée par ces débits (lecture Q95) ; éq. (589) et (590) pour la température aux
        // bouches sans refroidissement supplémentaire.
        let b = self.b;
        let mut qm_spec = 0.0;
        let mut somme_theta_i = 0.0;
        let mut somme_sans_froid = 0.0;
        for ((bouche, rho), rho_h1) in self
            .bouches
            .iter()
            .zip(&self.rho_souffle)
            .zip(&self.rho_souffle_h1)
        {
            let d = bouche.debits_regime(regime);
            if d.isouf != Isouf::Soufflage {
                continue;
            }
            let qm_c = qm_kg_s(*rho, d.q_cond_m3_h());
            let q_fuites = d.q_cond_m3_h() - d.q_spec_m3_h();
            let qm_s = (qm_c - qm_kg_s(*rho_h1, q_fuites)).max(0.0);
            qm_spec += qm_s;
            somme_theta_i += qm_s * moy[bouche.groupe].theta_i_fin_c;
            let lnc = t_lnc_c(b, moy[bouche.groupe].theta_i_fin_c, theta_ext_c);
            let e = exp_h(bouche.h_cond_ext_w_k, qm_c);
            somme_sans_froid += qm_c * (theta_souf2 - (theta_souf2 - lnc) * (1.0 - e));
        }
        if qm_spec <= 0.0 {
            return ModeActif::default();
        }
        let theta_i_moy = somme_theta_i / qm_spec;
        // Demande du mode engagé (sans demande, mode actif engagé par le seul report).
        let q_sys = if chauffe {
            q_sys_ch_wh.max(0.0)
        } else {
            q_sys_fr_wh.min(0.0)
        };
        // Éq. (582) à (594).
        let charge = match cta.type_cta {
            TypeCta::DebitVariable => {
                let theta_nom = if chauffe {
                    // Éq. (585).
                    batteries.theta_sou_nom_chaud_c.unwrap_or(theta_i_moy)
                } else {
                    // Éq. (590) et (591).
                    let nom = batteries
                        .froid
                        .and_then(|f| f.theta_sou_nom_c)
                        .unwrap_or(theta_i_moy);
                    (somme_sans_froid / qm_cond).min(nom)
                };
                let c = charge_debit_variable(q_sys, theta_nom, theta_i_moy, p_zn, cp, qm_spec);
                // Éq. (588) et (594) : en report, débit nominal pendant tout le pas de temps.
                if report && c.tau < 1.0 {
                    ChargeCentrale { tau: 1.0, ..c }
                } else {
                    c
                }
            }
            // Éq. (582) à (584) : débit constant, température variable, τ = 100 %.
            _ => ChargeCentrale {
                tau: 1.0,
                theta_sou_bouches_c: theta_sou_bouches_charge_c(
                    theta_i_moy,
                    q_sys,
                    p_zn,
                    cp,
                    qm_spec,
                ),
            },
        };
        // Éq. (595) : température à la sortie de la centrale.
        let mut somme_sortie = 0.0;
        for (bouche, rho) in self.bouches.iter().zip(&self.rho_souffle) {
            let d = bouche.debits_regime(regime);
            if d.isouf != Isouf::Soufflage {
                continue;
            }
            let qm_c = qm_kg_s(*rho, d.q_cond_m3_h());
            let lnc = t_lnc_c(b, moy[bouche.groupe].theta_i_fin_c, theta_ext_c);
            let e = exp_h(bouche.h_cond_ext_w_k, qm_c);
            somme_sortie += qm_c * (charge.theta_sou_bouches_c - lnc * (1.0 - e)) / e;
        }
        let theta_souf_rec = somme_sortie / qm_cond;
        // §6.3.3.5.5.4 : sécurité antigel recalculée avec les débits du mode actif, pondérée
        // par les durées de fonctionnement en mode actif et en zone neutre comme les autres
        // consommations du §6.3.3.5.8.4.1 (éq. 614 et 617 ; lecture Q95 point 11).
        let w_ag_wh = charge.tau * centrale.sorties.w_ag_wh + (1.0 - charge.tau) * self.zn.w_ag_wh;
        let mut resultat = ModeActif {
            actif: true,
            tau: charge.tau,
            chauffage: chauffe,
            // §6.3.3.5.6 : boîte de mélange recalculée, vue par la génération (éq. 951).
            theta_souf_mel_c: centrale.sorties.theta_souf_mel_c,
            w_ag_wh,
            ..ModeActif::default()
        };
        if chauffe {
            // Éq. (596) à (599).
            resultat.q_chaud_wh =
                q_chaud_rec_wh(charge.tau, cp, qm_cond, theta_souf_rec, theta_souf2);
            resultat.sous_dimensionnee = theta_souf_rec > THETA_SOUF_MAX_C;
        } else if let Some(f) = batteries.froid {
            // Éq. (601) à (612).
            let r = froid_rec(
                charge.tau,
                cp,
                qm_cond,
                theta_souf_rec,
                theta_souf2,
                centrale.omega_souf2_kg_kg,
                f.theta_dim_c,
            );
            resultat.q_froid_wh = r.q_froid_wh;
            resultat.q_lat_wh = r.q_lat_wh;
            resultat.sous_dimensionnee = r.sous_dimensionnee;
        }
        // Éq. (614) à (618) : ventilateurs, échangeur de la zone neutre, pompe du
        // rafraîchissement indirect recalculée (éq. 615 et 617 ; nulle pour le direct, éq. 616).
        let w_aux_adiab = charge.tau * w_aux_pompe_wh(centrale.qm_eau_ete_kg_s)
            + (1.0 - charge.tau) * self.zn.w_aux_adiab_wh;
        resultat.w_aux_adiab_wh = w_aux_adiab;
        // Pompe du puits hydraulique recalculée avec les débits du mode actif (§6.3.3.5.5.1).
        let w_puits =
            charge.tau * centrale.sorties.w_puits_wh + (1.0 - charge.tau) * self.zn.w_puits_wh;
        resultat.w_puits_wh = w_puits;
        if self.releve.is_some() {
            let w = w_vent_rec_wh(charge.tau, cta.p_vent_ch_w, self.zn.w_vent_wh)
                + self.zn.w_ech_wh
                + w_aux_adiab
                + w_puits;
            self.repartir_releve(w, regime);
        }
        resultat
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn debits(q: f64) -> DebitsBoucheProjet {
        DebitsBoucheProjet {
            isouf: Isouf::Soufflage,
            q_max_m3_h: q,
            q_base_m3_h: q,
            q_dep_m3_h: q,
            q_fuites_m3_h: 0.0,
            ratfuitevc: 1.0,
            a_cond_m2: 0.0,
        }
    }

    /// Éq. (427) en relance (inoccupation) : la borne basse du profil à hystérésis est le débit
    /// de zone neutre en occupation `Qv_sou_ZN,occ` (éq. 466, figure 73), non celui de
    /// l'inoccupation ; calcul à la main : 100 + 0,5 × (400 − 100) = 250 m³/h.
    #[test]
    fn hysteresis_bornee_a_la_zone_neutre_en_occupation() {
        let b = BoucheProjet {
            groupe: 0,
            debits: debits(100.0),
            debits_inocc: Some(debits(20.0)),
            dugd_h: 14.0,
            cdep: 1.3,
            h_cond_ext_w_k: 0.0,
            debits_ch: Some((debits(500.0), debits(400.0))),
            taux_an_min: (1.0, 1.0),
            x_adiab: 0.5,
            debits_rafnoc: None,
        };
        let regime = Regime {
            i_vent: false,
            hysteresis: true,
            ..Regime::default()
        };
        let d = b.debits_regime(regime);
        assert!((d.q_dep_m3_h - 250.0).abs() < 1e-12, "{}", d.q_dep_m3_h);
        // En occupation : 100 + 0,5 × (500 − 100) = 300 m³/h.
        let d = b.debits_regime(Regime {
            i_vent: true,
            ..regime
        });
        assert!((d.q_dep_m3_h - 300.0).abs() < 1e-12);
    }
}
