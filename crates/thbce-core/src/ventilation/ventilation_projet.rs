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
    RepresentationEchangeur, StatutEchangeur, SystemeVentilation, TypeCta, UsageZone, Zone,
};
use crate::resultats::ValeurParDefaut;

use super::bouche_conduit::{
    a_cond_defaut_collectif_m2, a_cond_defaut_maison_m2, debits_dep_habitation,
    debits_dep_non_residentiel, k_res, q_fuites_m3_h, Debits, DebitsBoucheProjet, PressionReseau,
    TypologieReseau, CDEP_DEFAUT, CRDBNR_DEFAUT, DUGD_DEFAUT_H, K_RES_DEFAUT,
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
use super::mecanique_basse_pression;
use super::mecanique_sf::{
    dugd_equ_h, h_cond_ext_w_k, p_vent_groupe_w, p_vent_residentiel_w, qm_kg_s, rho_kg_m3,
    t_apres_conduit_c, t_apres_ventilateur_c, t_lnc_c, theta_i_cond_c, C_PA_J_KG_K,
};

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
    /// « Hcond,rep,ext » ou « Hcond,souf,ext », en W/K.
    pub h_cond_ext_w_k: f64,
    /// Débits du mode actif d'une bouche reliée à une centrale de traitement d'air, en
    /// occupation et en inoccupation (fiche 6.2, éq. 406, 410, 415 et 416) ; `None` sinon.
    pub debits_ch: Option<(DebitsBoucheProjet, DebitsBoucheProjet)>,
    /// « TauxAN_min,occ » et « TauxAN_min,inocc » d'une bouche de soufflage reliée à une
    /// centrale de traitement d'air (fiche 6.2, éq. 393, 397, 408 et 411) ; 1 sinon.
    pub taux_an_min: (f64, f64),
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
        match self.debits_ch {
            Some((occ, inocc)) if regime.nominal => {
                if regime.i_vent {
                    occ
                } else {
                    inocc
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
            _ => return None,
        };

        // §6.2.3.4, tableau 59 : coefficient de fuite.
        let k_res = k_res(v.reseau.classe_etancheite);
        if v.reseau.classe_etancheite.is_none() {
            valeurs_par_defaut.push(defaut(
                "ventilation.reseau.classe_etancheite".to_owned(),
                K_RES_DEFAUT,
                "Th-BCE 2020 §6.2.3.4, tableau 59 : Kres de la valeur par défaut « DEF », en \
                 m³/(s.m²) sous 1 Pa",
            ));
        }
        let d_p_pa = typologie.d_p_pa(pression);
        // Surfaces saisies (Is_surf_cond_def = 0) si toutes les bouches en portent une.
        let surfaces_saisies = zone
            .groupes
            .iter()
            .flat_map(|g| &g.bouches_conduits)
            .all(|b| b.a_cond_m2.is_some());
        let ratfuitevc = match v.reseau.ratfuitevc {
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
                let cdep = b.cdep.unwrap_or_else(|| {
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
                    let crdbnr = if recyclage {
                        1.0
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
                        // Éq. (424) à (427).
                        TypologieReseau::BatimentCollectif | TypologieReseau::NonResidentiel => {
                            a_cond_defaut_collectif_m2(q_max)
                        }
                    };
                    valeurs_par_defaut.push(defaut(
                        format!("groupes[{ig}].bouches_conduits[{ib}].a_cond_m2"),
                        a,
                        "Th-BCE 2020 §6.2.3.4, éq. (421) à (427), tableau 60 : surface de \
                         conduit par défaut",
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
                bouches.push(BoucheProjet {
                    groupe: ig,
                    debits,
                    debits_inocc,
                    dugd_h,
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
        let releve =
            (non_residentiel && v.systeme.nom_centrale().is_some()).then(|| ReleveVentilation {
                serie_wh: vec![0.0; NB_HEURES_AN],
                mensuel_groupes_kwh: vec![[0.0; 12]; nb_groupes],
                dernier_groupes_wh: vec![0.0; nb_groupes],
                dernier_wh: 0.0,
            });
        let rho_initiale = rho_kg_m3(theta_initiale_c);
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
            regime: Regime {
                i_vent: true,
                i_vent_h1: true,
                conditions_nominales: true,
                nominal: true,
            },
        })
    }

    /// Fixe le régime du pas de temps avant le calcul des débits des groupes : indicateur de
    /// ventilation de la zone et, pour une centrale de traitement d'air, débits nominaux ou de
    /// zone neutre (§6.3.3.4.3.2), selon l'état des groupes desservis au pas précédent.
    pub fn preparer_pas(&mut self, i_vent: bool, groupes: &[EtatGroupeVentilation]) {
        let i_vent_h1 = self.regime.i_vent;
        // Note 1 du §6.3.3.1.4.1 : une relance ou un besoin d'un des groupes reliés suffit.
        let conditions = conditions_debit_nominal(
            i_vent,
            groupes.iter().any(|g| g.relance),
            i_vent_h1,
            groupes.iter().any(|g| g.besoin_ch),
            groupes.iter().any(|g| g.besoin_fr),
        );
        self.regime = Regime {
            i_vent,
            i_vent_h1,
            conditions_nominales: conditions,
            nominal: self
                .cta
                .as_ref()
                .is_some_and(|c| debits_nominaux(c.type_cta, conditions)),
        };
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
        }
    }

    /// Puissances des ventilateurs de reprise et de soufflage dans un régime, en W : éq. (460)
    /// à (465) pour une centrale de traitement d'air, (458) et (459) ou puissances constantes
    /// sinon.
    fn p_vent_regime(&self, regime: Regime) -> (f64, f64) {
        match &self.cta {
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

    /// Débits spécifiques d'un groupe au pas de temps en cours (régime fixé par
    /// [`VentilationProjet::preparer_pas`]).
    pub fn debits_groupe_pas(&self, g: usize) -> Debits {
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
            })
    }

    /// Débits d'une bouche au pas de temps : selon le régime de la centrale de traitement d'air,
    /// selon `Ivent` pour les autres systèmes.
    fn debits_pas(&self, b: &BoucheProjet, i_vent: bool) -> DebitsBoucheProjet {
        if self.cta.is_some() {
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
            })
    }

    /// Débits vus par les ventilateurs pour un groupe, en m³/h (éq. 430 et 431). Hors
    /// habitation, débits de l'occupation.
    pub fn debits_cond_groupe(&self, g: usize) -> Debits {
        self.debits_cond_groupe_selon(g, true)
    }

    /// Débits vus par les ventilateurs pour un groupe selon `Ivent`.
    pub fn debits_cond_groupe_selon(&self, g: usize, i_vent: bool) -> Debits {
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
            })
    }

    /// Puissance moyenne totale des ventilateurs du système, en W. Hors habitation, puissance
    /// en occupation.
    pub fn p_vent_w(&self) -> f64 {
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
            if est_cta {
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
        // Air neuf : air extérieur (éq. 413 et 657, sans puits).
        let theta_an = theta_ext_c;
        // Éq. (432) et (435) à (439).
        let qm_an = match &self.cta {
            Some(c) => qm_air_neuf_kg_s(
                qm_an_min,
                qm_sou,
                theta_an,
                c.theta_enc_enf_c.0,
                c.theta_enc_enf_c.1,
                c.regulation_air_neuf,
                theta_i_moy,
            ),
            None => qm_sou,
        };
        // Éq. (454) à (456) : débit recyclé, débit repris et débit rejeté rééquilibrés.
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
        let mut c = Centrale {
            sorties: PasVentilation {
                theta_air_extrait_c: t_extr2,
                qm_air_extrait_kg_s: qm_extrait,
                theta_souf_mel_c: theta_an,
                taux_an: if qm_sou > 0.0 { qm_an / qm_sou } else { 1.0 },
                ..PasVentilation::default()
            },
            qm_sou_kg_s: qm_sou,
            theta_souf2_c: theta_an,
            omega_souf2_kg_kg: omega_ext_kg_kg,
            w_ech_wh: 0.0,
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
                        let bypass = e.bypass.as_ref().is_some_and(|by| {
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
                        // Éq. (506) à (511) ; θextr3 = θextr2 sans humidification
                        // indirecte (éq. 484 et 485).
                        let a = antigel(
                            epsilon,
                            e.antigel,
                            saison_de_chauffage,
                            theta_an,
                            t_extr2,
                            e.theta_s_ech_rep_lim_c,
                            qm_an,
                        );
                        sorties.w_ag_wh = a.w_ag_wh;
                        (a.theta_souf_ag_c, a.epsilon)
                    }
                };
                // Éq. (512) et (513).
                let (theta_souf1, theta_rejete) =
                    sorties_echangeur_c(theta_souf_ag, t_extr2, epsilon);
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
            if est_cta {
                b.debits_regime(regime)
            } else {
                b.debits_selon(i_vent)
            }
        };
        let (p_vent_rep_w, p_vent_sou_w) = if est_cta {
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
        let mut sorties = centrale.sorties;
        let w_ech_wh = centrale.w_ech_wh;
        let qm_sou = centrale.qm_sou_kg_s;
        self.zn.w_vent_wh = p_vent_rep_w + p_vent_sou_w;
        self.zn.w_ech_wh = w_ech_wh;
        self.zn.omega_souffle_kg_kg = omega_ext_kg_kg;
        self.zn.souffle.fill((0.0, 0.0));
        self.rho_souffle_h1.copy_from_slice(&self.rho_souffle);

        if qm_sou > 0.0 {
            let theta_souf2 = centrale.theta_souf2_c;
            let omega_mel = centrale.omega_souf2_kg_kg;
            // Éq. (521) à (525) : préchauffage en saison de chauffage.
            let saison_de_chauffage = groupes.iter().any(|g| g.saison_de_chauffage);
            let theta_pre = match &self.prechauffage {
                Some(p) if saison_de_chauffage => {
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
            // Éq. (538) à (548) : prérefroidissement en saison de refroidissement (éq. 410).
            let (theta_souf3, omega_souf3) = match &self.prerefroidissement {
                Some(p) if groupes.iter().any(|g| g.saison_de_refroidissement) => {
                    let r = prerefroidissement(
                        theta_pre,
                        omega_mel,
                        theta_ext_c,
                        p.theta_cons_c,
                        p.theta_ex_c,
                        p.theta_dim_fr_c,
                        qm_sou,
                    );
                    sorties.q_prefroid_wh = r.q_prefroid_wh;
                    (r.theta_souf3_c, r.omega_souf3_kg_kg)
                }
                _ => (theta_pre, omega_mel),
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

        // Éq. (565) et (570) : consommation des ventilateurs et de l'échangeur, répartie entre
        // les groupes au prorata des débits soufflés vus par le ventilateur (lecture Q42), ceux
        // de l'occupation si aucun débit n'est soufflé (lecture Q60).
        if self.releve.is_some() {
            let w = p_vent_rep_w + p_vent_sou_w + w_ech_wh;
            let r_pas = if est_cta {
                regime
            } else {
                Regime {
                    nominal: false,
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
        let parts = |r: Regime| -> Vec<f64> {
            (0..self.nb_groupes)
                .map(|g| {
                    self.bouches
                        .iter()
                        .filter(|b| b.groupe == g)
                        .map(|b| b.debits_regime(r))
                        .filter(|d| d.isouf == Isouf::Soufflage)
                        .map(|d| d.q_cond_m3_h())
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
        let Some((cta, batteries)) = self
            .cta
            .as_ref()
            .and_then(|c| c.batteries_charge.map(|b| (c.clone(), b)))
        else {
            return ModeActif::default();
        };
        // §6.3.3.1.5, figure 72 : saison effective de la centrale (éq. 409 et 410 : un groupe
        // suffit) et demande des émetteurs.
        let chauffe =
            batteries.chaud && zn.iter().any(|g| g.saison_de_chauffage) && q_sys_ch_wh > 0.0;
        let refroidit = !chauffe
            && batteries.froid.is_some()
            && zn.iter().any(|g| g.saison_de_refroidissement)
            && q_sys_fr_wh < 0.0;
        if !chauffe && !refroidit {
            return ModeActif::default();
        }
        // Éq. (573), (577) et (581) : débits et puissances du mode actif ; §6.3.3.5.4 à
        // 6.3.3.5.7 : circuit recalculé avec les températures des groupes au pas en cours.
        let regime = Regime {
            nominal: true,
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
        let q_sys = if chauffe { q_sys_ch_wh } else { q_sys_fr_wh };
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
                charge_debit_variable(q_sys, theta_nom, theta_i_moy, p_zn, cp, qm_spec)
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
        let mut resultat = ModeActif {
            actif: true,
            tau: charge.tau,
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
        // Éq. (614) et (618) : ventilateurs, échangeur de la zone neutre.
        if self.releve.is_some() {
            let w =
                w_vent_rec_wh(charge.tau, cta.p_vent_ch_w, self.zn.w_vent_wh) + self.zn.w_ech_wh;
            self.repartir_releve(w, regime);
        }
        resultat
    }
}
