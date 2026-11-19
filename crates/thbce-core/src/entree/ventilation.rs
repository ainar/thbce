// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : ventilation (Th-BCE 2020 §6) — systèmes, échangeurs, réseaux
//! aérauliques, entrées d'air, bouches et conduits.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::*;

/// Th-BCE 2020 §6.2 à 6.5 — système de ventilation mécanique d'une zone : nature du système
/// (« Type_ventilation_mecaniq », §6.3.3.1.1), ventilateurs et réseau de conduits. Les bouches
/// et conduits sont décrits par groupe (`bouches_conduits`) et rattachés à ce système.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Ventilation {
    /// Identifiant du système.
    pub id: Id,
    /// Nature du système et paramètres propres.
    pub systeme: SystemeVentilation,
    /// Réseau de conduits : étanchéité et parties hors volume chauffé (§6.2.3.4, §6.3.3.3.2,
    /// §6.5.3.4).
    pub reseau: ReseauAeraulique,
    /// Raccordement à un puits climatique ou hydraulique (§6.8 à 6.11). Le puits climatique
    /// (`climatique`) se décrit par `puits_climatique` ; le puits hydraulique n'est pas encore
    /// pris en charge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub puits: Option<Puits>,
    /// Description du puits climatique (fiches 6.8, 6.10 et 6.11), requise si `puits` vaut
    /// `climatique` et interdite sinon. Le système doit être un simple flux par insufflation,
    /// un double flux ou une centrale de traitement d'air (§6.11.3). Le puits ne dessert que
    /// ce système.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub puits_climatique: Option<PuitsClimatique>,
    /// Réseau de distribution de CTA hydraulique de chauffage (fiche 8.11, « idtype = 1 »,
    /// §8.11.3.6.2 et fiche 8.12) : relie la centrale à la génération qui fournit l'énergie de
    /// ses fonctions de chauffage (préchauffage, sécurité antigel, humidificateur à vapeur,
    /// batterie chaude du mode actif), qui désignent toutes cette génération. Une centrale
    /// n'accepte qu'un seul réseau par fonction (§8.11.3.1). Absent : réseau fictif, sans
    /// pertes ni circulateur (`idtype = 0`). Les paramètres sont ceux d'un réseau hydraulique
    /// du groupe (§8.7.2 et §8.8.2) ; la température d'ambiance des pertes est celle,
    /// conventionnelle, de 20 °C (`docs/questions/Q121.md`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reseau_cta_chaud: Option<ReseauHydrauliqueGroupe>,
    /// Réseau de distribution de CTA hydraulique de refroidissement (§8.11.3.6.3) : même
    /// règle pour le prérefroidissement et la batterie froide du mode actif ; température
    /// d'ambiance conventionnelle de 26 °C.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reseau_cta_froid: Option<ReseauHydrauliqueGroupe>,
}

/// Nature d'un système de ventilation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SystemeVentilation {
    /// Ventilation mécanique simple flux par extraction (§6.5, `Isouf = 0`), autoréglable ou
    /// hygroréglable selon les bouches.
    SimpleFluxExtraction(VentilateurSimpleFlux),
    /// Ventilation mécanique simple flux par insufflation (§6.5, `Isouf = 1`).
    SimpleFluxInsufflation(VentilateurSimpleFlux),
    /// Extraction mécanique basse pression, pour les logements collectifs à conduits individuels
    /// ou de type shunt (§6.4).
    MecaniqueBassePression(VentilateurSimpleFlux),
    /// Ventilation mécanique double flux (§6.3, `Type_ventilation_mecaniq = 1`) : seul système
    /// double flux compatible avec l'usage résidentiel (§6.3.3.1.3).
    DoubleFlux(DoubleFlux),
    /// Ventilation naturelle par conduits (§6.6) : non prise en charge dans cette version.
    NaturelleParConduits,
    /// Ventilation hybride (§6.6) : non prise en charge dans cette version.
    Hybride,
    /// Aération par ouverture des fenêtres (§6.7) : ventilation naturelle sans ventilateur ni
    /// conduit. Les bouches de soufflage de chaque groupe donnent les débits d'air neuf fournis
    /// par une fenêtre équivalente (éq. 162) ; les débits repris sont ceux qui en sortent
    /// (éq. 163), les bouches de reprise ne servent pas. Le coefficient de dépassement vaut
    /// `Cfenb = 1,7` (§6.2.3.3.1, tableau 58, note 1), `Crdbnr = 1` et les fuites du réseau sont
    /// nulles (tableau 59) : `cdep`, `crdbnr`, `a_cond_m2` et le réseau ne se saisissent pas.
    Aeration,
    /// Centrale de traitement d'air sans description : forme de la version 0.1, refusée ;
    /// décrire la centrale par la variante `cta`.
    CentraleTraitementAir,
    /// Centrale de traitement d'air (§6.3, `Type_ventilation_mecaniq` de 2 à 4) : réservée au
    /// non résidentiel (§6.2.3.1.2 : « La CTA est employée uniquement dans le non
    /// résidentiel »).
    Cta(Cta),
}

impl SystemeVentilation {
    /// Nom JSON de la variante d'une centrale double flux (`double_flux` ou `cta`), `None` pour
    /// les autres systèmes.
    pub fn nom_centrale(&self) -> Option<&'static str> {
        match self {
            Self::DoubleFlux(_) => Some("double_flux"),
            Self::Cta(_) => Some("cta"),
            _ => None,
        }
    }

    /// Échangeur d'une centrale double flux ou d'une CTA.
    pub fn echangeur(&self) -> Option<&Echangeur> {
        match self {
            Self::DoubleFlux(d) => d.echangeur.as_ref(),
            Self::Cta(c) => c.echangeur.as_ref(),
            _ => None,
        }
    }

    /// Batterie de préchauffage d'une centrale double flux ou d'une CTA (§6.3.3.4.9.1).
    pub fn batterie_prechauffage(&self) -> Option<&BatteriePrechauffage> {
        match self {
            Self::DoubleFlux(d) => d.batterie_prechauffage.as_ref(),
            Self::Cta(c) => c.batterie_prechauffage.as_ref(),
            _ => None,
        }
    }

    /// Batterie de prérefroidissement d'une centrale double flux ou d'une CTA (§6.3.3.4.10).
    pub fn batterie_prerefroidissement(&self) -> Option<&BatteriePrerefroidissement> {
        match self {
            Self::DoubleFlux(d) => d.batterie_prerefroidissement.as_ref(),
            Self::Cta(c) => c.batterie_prerefroidissement.as_ref(),
            _ => None,
        }
    }

    /// Humidificateur d'hiver d'une centrale double flux ou d'une CTA (§6.3.3.4.9.2).
    pub fn humidificateur(&self) -> Option<&HumidificateurHiver> {
        match self {
            Self::DoubleFlux(d) => d.humidificateur.as_ref(),
            Self::Cta(c) => c.humidificateur.as_ref(),
            _ => None,
        }
    }

    /// Rafraîchissement par surventilation mécanique du système (§6.12) : simple flux,
    /// basse pression, double flux ou centrale de traitement d'air.
    pub fn surventilation(&self) -> Option<&SurventilationMecanique> {
        match self {
            Self::SimpleFluxExtraction(v)
            | Self::SimpleFluxInsufflation(v)
            | Self::MecaniqueBassePression(v) => v.rafraichissement_surventilation.as_ref(),
            Self::DoubleFlux(d) => d.rafraichissement_surventilation.as_ref(),
            Self::Cta(c) => c.rafraichissement_surventilation.as_ref(),
            _ => None,
        }
    }

    /// Rafraîchissement évaporatif d'une centrale double flux ou d'une CTA (§6.3.3.4.2).
    pub fn rafraichissement_adiabatique(&self) -> Option<&RafraichissementEvaporatif> {
        match self {
            Self::DoubleFlux(d) => d.rafraichissement_adiabatique.as_ref(),
            Self::Cta(c) => c.rafraichissement_adiabatique.as_ref(),
            _ => None,
        }
    }
}

/// Th-BCE 2020 §6.3.3.1 — type de centrale de traitement d'air (« Type_ventilation_mecaniq »).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeCta {
    /// 2 : centrale de prétraitement d'air à débit constant.
    DebitConstant,
    /// 3 : centrale de traitement d'air à débit constant et température variable.
    DebitConstantTemperatureVariable,
    /// 4 : centrale de traitement d'air à débit variable.
    DebitVariable,
}

/// « Type_régul_AN » : régulation du taux d'air neuf hors de la plage de fonctionnement en tout
/// air neuf (§6.3.3.4.3.3, éq. 437 à 439).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RegulationAirNeuf {
    /// 0 : régulation standard (par défaut) : débit d'air neuf minimal (éq. 437).
    Standard,
    /// 1 : régulation optimisée : tout air neuf si l'air neuf est plus froid que l'air repris
    /// (éq. 438 et 439).
    Optimisee,
}

/// Th-BCE 2020 §6.3.2, tableau 61 — centrale de traitement d'air (types 2 à 4), en usage
/// autre que l'habitation.
///
/// Les débits des bouches se saisissent par bouche (`bouches_conduits` des groupes) : débit
/// en mode actif `q_spec_ch_m3_h` (« qsoufflé,CH »), débits de zone neutre en occupation et en
/// inoccupation `q_spec_occ_m3_h` et `q_spec_inocc_m3_h` (« qsoufflé,ZN,occ »,
/// « qsoufflé,ZN,inocc », §6.2.3.1.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Cta {
    /// « Type_ventilation_mecaniq » de 2 à 4.
    pub type_cta: TypeCta,
    /// « Pvent,rep,CH » : puissance du ventilateur de reprise en mode actif (nominal), en W.
    pub p_vent_rep_ch_w: f64,
    /// « Pvent,sou,CH » : puissance du ventilateur de soufflage en mode actif, en W.
    pub p_vent_sou_ch_w: f64,
    /// « Pvent,rep,ZN occ » : puissance du ventilateur de reprise en zone neutre en
    /// occupation, en W.
    pub p_vent_rep_zn_occ_w: f64,
    /// « Pvent,sou,ZN occ » : la même pour le ventilateur de soufflage, en W.
    pub p_vent_sou_zn_occ_w: f64,
    /// « Pvent,rep,ZN inocc » : puissance du ventilateur de reprise en zone neutre en
    /// inoccupation, en W.
    pub p_vent_rep_zn_inocc_w: f64,
    /// « Pvent,sou,ZN inocc » : la même pour le ventilateur de soufflage, en W.
    pub p_vent_sou_zn_inocc_w: f64,
    /// « θENC » : borne basse de la plage de température d'air neuf de fonctionnement en tout
    /// air neuf, en °C (éq. 435 et 436).
    pub theta_enc_c: f64,
    /// « θENF » : borne haute de cette plage, en °C, supérieure à `theta_enc_c`.
    pub theta_enf_c: f64,
    /// « Type_régul_AN ». Absent : régulation standard, « par défaut » selon le texte, signalé
    /// dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regulation_air_neuf: Option<RegulationAirNeuf>,
    /// Échangeur air extrait / air neuf. Absent : pas d'échangeur (éq. 496).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echangeur: Option<Echangeur>,
    /// Batterie de préchauffage régulée sur la température d'air neuf (« Is_prechaud ») :
    /// centrale à débit constant (`debit_constant`) seulement (§6.3.3.4.9).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batterie_prechauffage: Option<BatteriePrechauffage>,
    /// Batterie de prérefroidissement (« Is_prefroid ») : centrale à débit constant seulement
    /// (§6.3.3.4.10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batterie_prerefroidissement: Option<BatteriePrerefroidissement>,
    /// Batterie de chauffage régulée selon la charge thermique (« Is_chaud », mode actif du
    /// §6.3.3.5) : centrales de types 3 et 4 seulement. Elle assure la demande des émetteurs
    /// de chauffage contenus dans les bouches de soufflage (`centrale` des émetteurs) ;
    /// requiert `generation_batterie_chaude` et, en type 4, `theta_sou_nom_chaud_c`
    /// (`docs/questions/Q095.md`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batterie_chaude: Option<bool>,
    /// Génération qui fournit l'énergie de la batterie chaude (« Qchaud(rec) », éq. 598), par un
    /// réseau de distribution de CTA fictif (fiche 8.11, éq. 940) : identifiant d'une
    /// génération du bâtiment pourvue d'un générateur de chauffage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_batterie_chaude: Option<Id>,
    /// « θsou_nom_chaud » : température de soufflage nominale aux bouches en chauffage d'une
    /// centrale à débit variable (type 4), en °C (éq. 585). Sans objet en type 3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theta_sou_nom_chaud_c: Option<f64>,
    /// Batterie de refroidissement régulée selon la charge (« Is_froid », mode actif du
    /// §6.3.3.5) : centrales de types 3 et 4 seulement ; requiert
    /// `generation_batterie_froide`, `theta_dim_fr_c` et, en type 4, `theta_sou_nom_froid_c`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batterie_froide: Option<bool>,
    /// Génération qui fournit le froid de la batterie froide (« Qfroid(rec) », éq. 611), par un
    /// réseau de CTA fictif (fiche 8.11, éq. 940) : identifiant d'une génération du bâtiment
    /// pourvue d'un générateur de refroidissement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_batterie_froide: Option<Id>,
    /// « θsou,nom,froid » : température de soufflage nominale aux bouches en refroidissement
    /// d'une centrale à débit variable (type 4), en °C (éq. 591). Sans objet en type 3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theta_sou_nom_froid_c: Option<f64>,
    /// « θdim,fr » : température nominale du fluide dans la batterie froide ou l'évaporateur, en
    /// °C (éq. 604).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theta_dim_fr_c: Option<f64>,
    /// Humidificateur d'hiver (« Is_ChaudHR ») : `true` exige sa description
    /// `humidificateur` ; forme conservée pour la compatibilité (sans effet propre).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub humidification: Option<bool>,
    /// Description de l'humidificateur d'hiver (lot 1-ven3) ; absent : pas d'humidificateur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub humidificateur: Option<HumidificateurHiver>,
    /// Rafraîchissement évaporatif (« Type_humidification_Eté ») : `true` exige sa
    /// description `rafraichissement_adiabatique` ; forme conservée pour la compatibilité.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rafraichissement_evaporatif: Option<bool>,
    /// Description du rafraîchissement évaporatif (lot 1-ven3) ; absent : pas de
    /// rafraîchissement évaporatif.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rafraichissement_adiabatique: Option<RafraichissementEvaporatif>,
    /// Rafraîchissement par surventilation mécanique (§6.12) : `true` exige sa description
    /// `rafraichissement_surventilation` ; forme conservée pour la compatibilité.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surventilation_mecanique: Option<bool>,
    /// Description du rafraîchissement par surventilation mécanique (§6.12, lot 1-ven6) ;
    /// absent : pas de surventilation mécanique (« israfnoc = 0 »).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rafraichissement_surventilation: Option<SurventilationMecanique>,
}

/// Th-BCE 2020 §6.3.3.4.9.1 — batterie de préchauffage régulée sur la température extérieure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BatteriePrechauffage {
    /// « θcons_prechaud » : température de consigne de soufflage du préchauffage, en °C (éq.
    /// 522).
    pub theta_cons_c: f64,
    /// « θex_prechaud » : température extérieure au-dessus de laquelle le préchauffage n'est
    /// pas autorisé, en °C.
    pub theta_ex_c: f64,
    /// Génération qui fournit l'énergie du préchauffage (« Qprechaud », éq. 523), par un
    /// réseau de distribution de CTA fictif (fiche 8.11, éq. 939) : identifiant d'une
    /// génération du bâtiment pourvue d'un générateur de chauffage.
    pub generation: Id,
}

/// Th-BCE 2020 §6.3.3.4.10 — batterie de prérefroidissement régulée sur la température
/// extérieure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BatteriePrerefroidissement {
    /// « θcons_prefroid » : température de consigne de soufflage du prérefroidissement, en °C
    /// (éq. 540 et 544).
    pub theta_cons_c: f64,
    /// « θex_prefroid » : température extérieure au-dessous de laquelle le prérefroidissement
    /// n'est pas autorisé, en °C.
    pub theta_ex_c: f64,
    /// « θdim,fr » : température nominale moyenne du fluide dans la batterie, en °C (éq. 539).
    pub theta_dim_fr_c: f64,
    /// Génération qui fournit le froid (« Qprefroid », éq. 545), par un réseau de distribution
    /// de CTA fictif (fiche 8.11, éq. 939) : identifiant d'une génération du bâtiment pourvue
    /// d'un générateur de refroidissement.
    pub generation: Id,
}

/// Th-BCE 2020 §6.3.2 et §6.3.3.4.9.2 — type d'humidificateur d'hiver (« TypeHumidification »).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeHumidificationHiver {
    /// Humidificateur à vapeur : transformation isotherme, énergie demandée à la génération
    /// (éq. 526 à 529).
    Vapeur,
    /// Humidificateur à pulvérisation, ruissellement ou évaporation : transformation
    /// adiabatique, pompe (éq. 530 à 534 et 568).
    Pulverisation,
}

/// Th-BCE 2020 §6.3.3.4.9.2 — humidificateur d'hiver (« Is_ChaudHR = 1 ») d'un double flux ou
/// d'une centrale à débit constant (types 1 et 2), en saison de chauffage, après la batterie
/// de préchauffage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HumidificateurHiver {
    /// « TypeHumidification ».
    pub type_humidification: TypeHumidificationHiver,
    /// « ωcons » : humidité spécifique de consigne de l'air soufflé, en kg/kg d'air sec (0 à 1).
    pub omega_cons_kg_kg: f64,
    /// « ηhum » : efficacité d'un humidificateur à pulvérisation (0 à 1) ; 0,8 par défaut
    /// (tableau 61 et éq. 530), valeur signalée ; refusée pour un humidificateur à vapeur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta_hum: Option<f64>,
    /// Génération qui fournit l'énergie d'un humidificateur à vapeur (« QchaudHR », éq. 529),
    /// par un réseau de distribution de CTA fictif (fiche 8.11, « idchaudHR », éq. 939) :
    /// identifiant d'une génération du bâtiment pourvue d'un générateur de chauffage. Requise
    /// pour un humidificateur à vapeur, refusée sinon (« QHR = 0 », éq. 533).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation: Option<Id>,
}

/// Th-BCE 2020 §6.3.2 — type de rafraîchissement évaporatif (« Type_humidification_Eté »).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeRafraichissementEvaporatif {
    /// 1 : humidificateur direct sur l'air soufflé (§6.3.3.4.11, éq. 549 à 559).
    Direct,
    /// 2 : humidificateur indirect sur l'air extrait, avant l'échangeur (§6.3.3.4.5.3, éq. 476
    /// à 486).
    Indirect,
}

/// Th-BCE 2020 §6.3.2 et §6.3.3.4.2 — rafraîchissement évaporatif (adiabatique) d'un double
/// flux ou d'une centrale de traitement d'air.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RafraichissementEvaporatif {
    /// « Type_humidification_Eté » : direct ou indirect.
    pub type_rafraichissement: TypeRafraichissementEvaporatif,
    /// « ηhum_dir » ou « ηhum_indir » : rendement de l'humidificateur (0 à 1) ; 0,8 par défaut
    /// (tableau 61), valeur signalée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta_hum: Option<f64>,
    /// « θi_base » : température intérieure au-dessus de laquelle le rafraîchissement est
    /// autorisé (§6.3.3.4.2), en °C ; au moins 22 °C.
    pub theta_i_base_c: f64,
    /// « Δθi1 » : paramètre du profil à hystérésis des débits d'une centrale à débit variable
    /// (éq. 426), en K, strictement positif ; requis pour une centrale à débit variable,
    /// refusé sinon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_theta_i1_k: Option<f64>,
    /// « Δθi2 » : voir `delta_theta_i1_k`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_theta_i2_k: Option<f64>,
}

/// « israfnoc » (§6.12.2, tableau 75) : nature du rafraîchissement par surventilation
/// mécanique d'un système de ventilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeSurventilation {
    /// 1 : rafraîchissement nocturne en période d'été seulement.
    NocturneEte,
    /// 2 : rafraîchissement nocturne toute l'année.
    NocturneAnnee,
    /// 3 : rafraîchissement par surventilation mécanique (freecooling) en période d'été
    /// seulement.
    FreecoolingEte,
    /// 4 : rafraîchissement par surventilation mécanique (freecooling) toute l'année.
    FreecoolingAnnee,
}

impl TypeSurventilation {
    /// Rafraîchissement nocturne (types 1 et 2) : activé seulement dans la période nocturne
    /// saisie (éq. 768 et 769).
    pub fn est_nocturne(self) -> bool {
        matches!(self, Self::NocturneEte | Self::NocturneAnnee)
    }

    /// Fonctionnement toute l'année (types 2 et 4), avec un jeu de consignes de mi-saison
    /// (éq. 764 et 765).
    pub fn toute_l_annee(self) -> bool {
        matches!(self, Self::NocturneAnnee | Self::FreecoolingAnnee)
    }
}

/// Consignes du rafraîchissement par surventilation mécanique pour une saison (§6.12.2,
/// tableau 75 : indices « été » et « mis »).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConsignesSurventilation {
    /// « hleg,deb_rafnoc » : heure légale (début de pas de temps) de début de la période de
    /// rafraîchissement nocturne, de 21 à 23 ou de 0 à 8 (éq. 767). Requise pour un
    /// rafraîchissement nocturne, sans objet en freecooling (note de l'éq. 770).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h_leg_deb_h: Option<u8>,
    /// « hleg,fin_rafnoc » : heure légale (début de pas de temps) de fin de la période de
    /// rafraîchissement nocturne, de 21 à 23 ou de 0 à 8, ultérieure à l'heure de début sur la
    /// période nuit (éq. 767).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h_leg_fin_h: Option<u8>,
    /// « θsd_dec_rafnoc » : température intérieure (sonde) de déclenchement, en °C (−20 à
    /// 50) ; −20 °C modélise l'absence de mesure de température intérieure (§6.12.3.3, notes).
    pub theta_sd_dec_c: f64,
    /// « θsd_arr_rafnoc » : température intérieure (sonde) d'arrêt, en °C (−20 à 50).
    pub theta_sd_arr_c: f64,
    /// « Δθlim_rafnoc » : écart limite entre les températures intérieure et extérieure pour
    /// autoriser le rafraîchissement, en K (−20 à 50).
    pub delta_theta_lim_k: f64,
    /// « θext_rafnoc » : température extérieure limite (basse) pour autoriser le
    /// rafraîchissement, en °C (−20 à 50) ; −20 °C modélise l'absence de sécurité sur la
    /// température extérieure.
    pub theta_ext_c: f64,
}

/// Th-BCE 2020 §6.12 — rafraîchissement par surventilation mécanique d'un système de
/// ventilation (calculs Th-C et Th-D seulement). Les débits se saisissent par bouche
/// (`q_spec_rafnoc_m3_h` de chaque bouche du système).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurventilationMecanique {
    /// « israfnoc » de 1 à 4.
    pub type_surventilation: TypeSurventilation,
    /// Consignes en saison de refroidissement (indice « été »).
    pub ete: ConsignesSurventilation,
    /// Consignes hors saison de refroidissement (indice « mis ») : requises pour un
    /// fonctionnement toute l'année (types 2 et 4, éq. 765), sans objet sinon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mi_saison: Option<ConsignesSurventilation>,
    /// « Pvent_rafnoc_rep » : puissance électrique du ventilateur de reprise en
    /// rafraîchissement, en W (éq. 779).
    pub p_vent_rafnoc_rep_w: f64,
    /// « Pvent_rafnoc_souf » : puissance électrique du ventilateur de soufflage en
    /// rafraîchissement, en W (éq. 779).
    pub p_vent_rafnoc_souf_w: f64,
}

/// Th-BCE 2020 §6.4.3 et §6.5.2 — ventilateur d'un système simple flux : puissances en débit
/// de base et de pointe en résidentiel, en occupation et en inoccupation en non résidentiel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VentilateurSimpleFlux {
    /// « Pventbase » : puissance électrique du ventilateur en débit de base, en W. Requise en
    /// maison individuelle ou accolée et en logement collectif ; ne pas la saisir dans les
    /// autres usages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_base_w: Option<f64>,
    /// « Pventpointe » : puissance électrique du ventilateur en débit de pointe, en W. Requise
    /// en maison individuelle ou accolée et en logement collectif.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_pointe_w: Option<f64>,
    /// « Pventocc » : puissance électrique du ventilateur en occupation, en non résidentiel,
    /// en W (§6.5.3.3, éq. 636 et 638). Requise dans les usages autres que la maison
    /// individuelle ou accolée et le logement collectif ; ne pas la saisir en habitation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_occ_w: Option<f64>,
    /// « Pventinocc » : puissance électrique du ventilateur en inoccupation, en non
    /// résidentiel, en W (éq. 637 et 639). Requise comme `p_vent_occ_w` ; 0 si le ventilateur
    /// est arrêté en inoccupation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_inocc_w: Option<f64>,
    /// Rafraîchissement par surventilation mécanique (§6.12, lot 1-ven6) ; absent : pas de
    /// surventilation mécanique.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rafraichissement_surventilation: Option<SurventilationMecanique>,
}

/// Th-BCE 2020 §6.3.2, tableau 61 — groupe de ventilation double flux
/// (`Type_ventilation_mecaniq = 1`), en habitation et dans les autres usages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DoubleFlux {
    /// « Pvent,base,rep » : puissance électrique du ventilateur de reprise en débit de base, en W.
    /// Requise en maison individuelle ou accolée et en logement collectif ; ne pas la saisir
    /// dans les autres usages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_base_rep_w: Option<f64>,
    /// « Pvent,pointe,rep » : puissance électrique du ventilateur de reprise en débit de pointe,
    /// en W. Requise en habitation comme `p_vent_base_rep_w`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_pointe_rep_w: Option<f64>,
    /// « Pvent,base,sou » : puissance électrique du ventilateur de soufflage en débit de base, en
    /// W. Requise en habitation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_base_sou_w: Option<f64>,
    /// « Pvent,pointe,sou » : puissance électrique du ventilateur de soufflage en débit de
    /// pointe, en W. Requise en habitation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_pointe_sou_w: Option<f64>,
    /// « Pvent,rep,occ » : puissance du ventilateur de reprise en occupation, en usage autre
    /// que l'habitation, en W (éq. 458). Requise dans ces usages ; ne pas la saisir en
    /// habitation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_rep_occ_w: Option<f64>,
    /// « Pvent,sou,occ » : la même pour le ventilateur de soufflage, en W (éq. 458).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_sou_occ_w: Option<f64>,
    /// « Pvent,rep,inocc » : puissance du ventilateur de reprise en inoccupation, en usage
    /// autre que l'habitation, en W (éq. 459) ; 0 si le ventilateur est arrêté.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_rep_inocc_w: Option<f64>,
    /// « Pvent,inocc,sou » : la même pour le ventilateur de soufflage, en W (éq. 459).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_vent_sou_inocc_w: Option<f64>,
    /// Échangeur. Absent : pas d'échangeur (`type_échangeur = 0`, éq. 496).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echangeur: Option<Echangeur>,
    /// Forme de la version 0.1 : `true` est refusé ; décrire la batterie par
    /// `batterie_prechauffage`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prechauffage: Option<bool>,
    /// Batterie de préchauffage régulée sur la température d'air neuf (« Is_prechaud »,
    /// §6.3.3.4.9.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batterie_prechauffage: Option<BatteriePrechauffage>,
    /// Batterie de prérefroidissement (« Is_prefroid », §6.3.3.4.10) : la zone doit compter au
    /// moins un groupe climatisé (éq. 389).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batterie_prerefroidissement: Option<BatteriePrerefroidissement>,
    /// Humidificateur d'hiver (« Is_ChaudHR ») : `true` exige sa description
    /// `humidificateur` ; forme conservée pour la compatibilité (sans effet propre).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub humidification: Option<bool>,
    /// Description de l'humidificateur d'hiver (lot 1-ven3) ; absent : pas d'humidificateur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub humidificateur: Option<HumidificateurHiver>,
    /// Rafraîchissement évaporatif (« Type_humidification_Eté ») : `true` exige sa
    /// description `rafraichissement_adiabatique` ; forme conservée pour la compatibilité.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rafraichissement_evaporatif: Option<bool>,
    /// Description du rafraîchissement évaporatif (lot 1-ven3) ; absent : pas de
    /// rafraîchissement évaporatif.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rafraichissement_adiabatique: Option<RafraichissementEvaporatif>,
    /// Rafraîchissement par surventilation mécanique (§6.12) : `true` exige sa description
    /// `rafraichissement_surventilation` ; forme conservée pour la compatibilité.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surventilation_mecanique: Option<bool>,
    /// Description du rafraîchissement par surventilation mécanique (§6.12, lot 1-ven6) ;
    /// absent : pas de surventilation mécanique (« israfnoc = 0 »).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rafraichissement_surventilation: Option<SurventilationMecanique>,
}

/// Th-BCE 2020 §6.3.2, tableau 61, et §6.3.3.4.6 — échangeur d'un double flux.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Echangeur {
    /// Représentation de l'échangeur (« idniveau_detail_echangeur »).
    pub representation: RepresentationEchangeur,
    /// Échangeur rotatif (échangeur à roue). Absent : échangeur à plaques (statique).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotatif: Option<bool>,
    /// « Pech » : puissance de l'échangeur rotatif, en W (éq. 497) ; requise pour un échangeur
    /// rotatif, sans objet sinon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_ech_w: Option<f64>,
    /// Bypass de l'échangeur (« isbypass = 1 ») et sa régulation. Absent : pas de bypass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bypass: Option<Bypass>,
    /// « isantigel » : sécurité antigel par préchauffage de l'air neuf. Absent ou faux :
    /// l'échangeur est contourné quand un risque de givrage est identifié (éq. 509).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub antigel: Option<bool>,
    /// Génération qui fournit l'énergie de la sécurité antigel (« Wag », éq. 508) : identifiant
    /// d'une génération du bâtiment pourvue d'un générateur de chauffage. La demande lui parvient
    /// comme celle d'un réseau de distribution associé à l'antigel (§8.11.3, éq. 939). Requise
    /// avec `antigel`, sans objet sinon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_antigel: Option<Id>,
    /// « θs,ech,rep,LIM » : température limite basse de sortie côté rejet, en °C (de −10 à
    /// 10). Absente : valeur par défaut du tableau 62, signalée dans les résultats : dans les
    /// usages autres que l'habitation, −5 °C pour un échangeur rotatif et 0 °C pour un
    /// échangeur à plaques ; 5 °C en habitation (« autres cas »).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theta_s_ech_rep_lim_c: Option<f64>,
}

/// « idniveau_detail_echangeur » : représentation simplifiée (efficacité) ou détaillée
/// (coefficient d'échange global).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RepresentationEchangeur {
    /// Représentation simplifiée : efficacité saisie, corrigée selon son statut (§6.3.3.3.3).
    Simplifiee(EchangeurSimplifie),
    /// Représentation détaillée : efficacité calculée à chaque pas de temps par la méthode NUT
    /// (éq. 499 à 505).
    Detaillee(EchangeurDetaille),
}

/// Échangeur en représentation simplifiée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EchangeurSimplifie {
    /// « εsaisi » : efficacité de l'échangeur en ventilation seule, mesurée aux débits de la
    /// zone neutre en occupation (de 0 à 1).
    pub epsilon_saisi: f64,
    /// « idstatut_echangeur ». Absent : valeur déclarée par le fabricant, « par défaut » selon
    /// le tableau 61, signalé dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statut: Option<StatutEchangeur>,
}

/// « idstatut_echangeur » (§6.3.3.3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatutEchangeur {
    /// 0 : valeur déclarée par le fabricant (éq. 398).
    Declare,
    /// 1 : valeur justifiée par un essai d'un laboratoire accrédité (éq. 397).
    Justifie,
    /// 2 : valeur certifiée (éq. 396).
    Certifie,
}

/// Échangeur en représentation détaillée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EchangeurDetaille {
    /// « UA_saisi » : coefficient d'échange global, en W/K.
    pub ua_w_k: f64,
    /// « idtype_echangeur » : nature des écoulements.
    pub ecoulement: EcoulementEchangeur,
}

/// « idtype_echangeur » (tableau 61).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EcoulementEchangeur {
    /// 1 : écoulements à contre-courant.
    ContreCourant,
    /// 2 : écoulements à courants parallèles.
    CourantsParalleles,
    /// 3 : écoulements à courants croisés, les deux fluides brassés.
    CourantsCroisesBrasses,
}

/// Régulation du bypass de l'échangeur (§6.3.3.4.6.2, éq. 494).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Bypass {
    /// « θext,bypass,hiver » : température d'air neuf au-dessus de laquelle l'échangeur peut
    /// être contourné en saison de chauffage, en °C.
    pub theta_ext_bypass_hiver_c: f64,
    /// « θext,bypass,été » : la même hors saison de chauffage, en °C.
    pub theta_ext_bypass_ete_c: f64,
    /// « θint,bypass,hiver » : température intérieure au-dessus de laquelle l'échangeur peut
    /// être contourné en saison de chauffage, en °C.
    pub theta_int_bypass_hiver_c: f64,
    /// « θint,bypass,été » : la même hors saison de chauffage, en °C.
    pub theta_int_bypass_ete_c: f64,
}

/// Th-BCE 2020 §6.2.2, §6.3.2 et §6.5.2 — réseau de conduits d'un système de ventilation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReseauAeraulique {
    /// « Cletres » : classe d'étanchéité du réseau. Absente : valeur par défaut du tableau 59
    /// (`Kres = 0,0675·10⁻³ m³/(s.m²)` sous 1 Pa), signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classe_etancheite: Option<ClasseEtancheite>,
    /// « Ratfuitevc » : part des fuites en volume chauffé (de 0 à 1). Requise quand les
    /// surfaces des conduits sont saisies (`a_cond_m2` des bouches, `Is_surf_cond_def = 0`) ;
    /// sinon valeur par défaut du tableau 60 (0,25 en maison individuelle, 0,5 en bâtiment
    /// collectif), signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ratfuitevc: Option<f64>,
    /// « Rrep » : résistance thermique de la partie des conduits de reprise située hors volume
    /// chauffé, en m².K/W. Requise si le système a des bouches de reprise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r_rep_m2k_w: Option<f64>,
    /// « Rsouf » : résistance thermique de la partie des conduits de soufflage située hors
    /// volume chauffé, en m².K/W. Requise si le système a des bouches de soufflage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r_souf_m2k_w: Option<f64>,
    /// « b » (« bet ») : ratio de prise en compte des locaux non chauffés que traversent les
    /// conduits (de 0 à 1 ; 1 pour des conduits à l'extérieur, éq. 649 et 471).
    pub b: f64,
}

/// « Cletres » : classe d'étanchéité d'un réseau aéraulique (tableau 59).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ClasseEtancheite {
    /// Classe A.
    A,
    /// Classe B.
    B,
    /// Classe C.
    C,
}

/// Puits climatique ou hydraulique (§6.8 à 6.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Puits {
    /// Puits climatique (§6.8).
    Climatique,
    /// Puits hydraulique (§6.9).
    Hydraulique,
}

/// Th-BCE 2020 §6.8.2, tableau 69 — puits climatique (ou canadien) : conduits enterrés dans
/// lesquels circule l'air neuf, et gestion de son bypass (§6.10.2, tableau 73).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PuitsClimatique {
    /// « nd » : nombre de conduits (au moins 1).
    pub nd: u32,
    /// « L » : longueur d'un conduit, en m (strictement positive).
    pub l_m: f64,
    /// « di » : diamètre intérieur du conduit, en m (strictement positif).
    pub di_m: f64,
    /// « ep » : épaisseur du conduit, en m (positive ou nulle).
    pub ep_m: f64,
    /// « λtube » : conductivité thermique du conduit, en W/(m.K) (strictement positive).
    pub lambda_tube_w_m_k: f64,
    /// « Z » : profondeur d'enfouissement des conduits, en m (positive ou nulle).
    pub profondeur_m: f64,
    /// « Type_solPC » : nature du sol (tableau 70).
    pub type_sol: TypeSolPuits,
    /// Gestion du bypass en fonction des températures (« typegest_PC » = 1, §6.10.2) ; absent :
    /// pas de bypass (« typegest_PC » = 0), le puits est toujours traversé.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bypass: Option<BypassPuits>,
}

/// « Type_solPC » : nature du sol d'un puits climatique (§6.8.3.2.1, tableau 70).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeSolPuits {
    /// 0 - sol humide (`gm` = 1,00).
    SolHumide,
    /// 1 - sable sec (`gm` = 0,90).
    SableSec,
    /// 2 - sable humide (`gm` = 0,98).
    SableHumide,
    /// 3 - argile humide (`gm` = 1,04).
    ArgileHumide,
    /// 4 - argile mouillée (`gm` = 1,05).
    ArgileMouillee,
}

/// Consignes du bypass d'un puits climatique (§6.10.2, tableau 73, éq. 751 et 752) : le puits
/// est contourné lorsque l'air extérieur est plus frais que l'air intérieur, plus chaud que la
/// consigne extérieure et que l'air intérieur dépasse la consigne intérieure.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BypassPuits {
    /// « θint,bypass_PC^hiver » : température intérieure de consigne au-dessus de laquelle le
    /// puits peut être contourné en période de chauffage, en °C.
    pub theta_int_hiver_c: f64,
    /// « θext,bypass_PC^hiver » : température extérieure de consigne au-dessus de laquelle le
    /// puits peut être contourné en période de chauffage, en °C.
    pub theta_ext_hiver_c: f64,
    /// « θint,bypass_PC^été » : la même hors période de chauffage, en °C.
    pub theta_int_ete_c: f64,
    /// « θext,bypass_PC^été » : la même hors période de chauffage, en °C.
    pub theta_ext_ete_c: f64,
}

/// Th-BCE 2020 §5.6.2 et §5.6.3.2 — entrées d'air d'un groupe.
///
/// La méthode forme une entrée d'air équivalente pour la zone, somme de celles des groupes, puis
/// la répartit entre les groupes au prorata de leurs modules (éq. 147 à 149) et, dans chaque
/// groupe, par quarts entre le haut et le bas des façades au vent et sous le vent (tableau 26).
/// Le groupe se décrit donc par la somme des modules de ses entrées d'air.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EntreesAir {
    /// « M » : somme des modules des entrées d'air du groupe, en m³/h (débit sous 20 Pa).
    pub m_m3_h: f64,
    /// Comportement autoréglable ; absent pour des entrées d'air fixes. Les pressions `ΔP1` et
    /// `ΔP2` sont les mêmes pour tous les groupes d'une zone (§5.6.3.2.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoreglable: Option<AutoreglageEntreesAir>,
    /// « r » : atténuation du débit par un dispositif anti-retour, de 0 à 1 (éq. 146). Absent :
    /// pas de dispositif anti-retour (`r = 1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r: Option<f64>,
    /// Identifiant de l'espace tampon par lequel transite l'air neuf des entrées d'air
    /// (§5.6.3.1, éq. 134 : l'air neuf entre alors à la température de l'espace tampon). Absent :
    /// l'air neuf vient directement de l'extérieur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
}

/// Th-BCE 2020 §5.6.3.2.1, figure 21 — comportement autoréglable d'une entrée d'air.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AutoreglageEntreesAir {
    /// « ΔP1 » : pression du premier changement de comportement, en Pa.
    pub delta_p1_pa: f64,
    /// « ΔP2 » : pression du second changement de comportement, en Pa.
    pub delta_p2_pa: f64,
    /// « M′ » : somme des modules du défaut d'autorégulabilité, en m³/h.
    pub m_prime_m3_h: f64,
}

/// Th-BCE 2020 §6.2.2 — ensemble « bouche et conduit » d'un groupe, en maison individuelle ou
/// accolée et en logement collectif : un circuit de reprise ou un circuit de soufflage.
///
/// Les débits sont les débits d'hygiène du circuit pour le groupe, en pointe (grand débit) et en
/// base. La méthode ne donne pas les débits réglementaires par nombre de pièces : ils se
/// saisissent. Pour un système hygroréglable, le débit de pointe vient des débits maximaux de
/// l'avis technique en cuisine et en WC et des débits d'hygiène des autres pièces ; le débit de
/// base se déduit de `qvarep_spec_m3_h` (§6.2.3.1.1.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoucheConduit {
    /// Identifiant, unique dans le groupe.
    pub id: Id,
    /// « Isouf » : sens du circuit.
    pub isouf: Isouf,
    /// « qspec,rep,conv_pointe » ou « qspec,souffle,conv_pointe » : débit volumique spécifique
    /// conventionnel en pointe, en m³/h, positif. Requis en maison individuelle ou accolée et
    /// en logement collectif ; ne pas le saisir dans les autres usages, qui se décrivent par
    /// `q_spec_occ_m3_h` et `q_spec_inocc_m3_h`.
    #[serde(default, skip_serializing_if = "est_nul")]
    pub q_spec_conv_pointe_m3_h: f64,
    /// « qspec,rep,occ » ou « qspec,soufflé,occ » : débit volumique total repris ou soufflé en
    /// occupation, en m³/h, positif, pour les usages autres que la maison individuelle ou
    /// accolée et le logement collectif (§6.2.3.1.1.1, éq. 383 et 384). Requis dans ces usages.
    /// Le calcul du Bbio ne l'emploie pas : ses débits sont conventionnels (§6.2.3.1.3,
    /// tableau 56-1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_spec_occ_m3_h: Option<f64>,
    /// « qspec,rep,inocc » ou « qspec,soufflé,inocc » : débit volumique total repris ou soufflé
    /// en inoccupation, en m³/h, positif ou nul (éq. 385 et 386). Requis comme
    /// `q_spec_occ_m3_h`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_spec_inocc_m3_h: Option<f64>,
    /// « qsoufflé,CH » : débit de la bouche en mode actif (hors zone neutre), en m³/h, positif,
    /// pour une bouche reliée à une centrale de traitement d'air (§6.2.3.1.2, éq. 391 et 394) ;
    /// requis dans ce cas, sans objet sinon. Les débits de zone neutre sont alors
    /// `q_spec_occ_m3_h` (« qsoufflé,ZN,occ ») et `q_spec_inocc_m3_h` (« qsoufflé,ZN,inocc »),
    /// au plus égaux à celui-ci ; les taux d'air neuf minimaux en découlent (éq. 393 et 397).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_spec_ch_m3_h: Option<f64>,
    /// « Crdbnr » : coefficient de réduction des débits en occupation, en non résidentiel
    /// (§6.2.3.2.1, tableau 57) : 1 sans régulation des débits, 0,9 avec un dispositif de
    /// détection d'utilisation du local, 0,8 avec un comptage d'occupants ou des sondes de CO2 ;
    /// une autre valeur si elle vient d'un Avis Technique. Absent : 1, aucune régulation,
    /// signalé dans les résultats. Sans objet en habitation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crdbnr: Option<f64>,
    /// « qspec,rep,conv_base » ou « qspec,souffle,conv_base » : débit volumique spécifique
    /// conventionnel en base, en m³/h, positif et au plus égal au débit de pointe. À saisir sauf
    /// pour un système hygroréglable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_spec_conv_base_m3_h: Option<f64>,
    /// « Qvarepspec » : débit de l'avis technique d'un système hygroréglable, en m³/h
    /// (§6.2.3.1.1.2). Le débit de base en est déduit ; ne pas saisir alors
    /// `q_spec_conv_base_m3_h`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qvarep_spec_m3_h: Option<f64>,
    /// « Dugd » : durée d'utilisation du grand débit, en h par semaine (§6.2.3.2.2.2, tableau
    /// 57) : 14 pour un dispositif à gestion manuelle, 7 avec temporisation ; davantage pour un
    /// passage en grand débit commandé depuis plusieurs pièces (note 3). Absente : 14, gestion
    /// manuelle « par défaut », signalée dans les résultats. Le calcul du Bbio emploie toujours
    /// 14.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dugd_h: Option<f64>,
    /// « Cdep » : coefficient de dépassement (§6.2.3.3.1, tableau 58) : 1,15 pour des
    /// composants autoréglables certifiés, la valeur de l'évaluation pour des composants
    /// hygroréglables. Absent : 1,30, valeur par défaut, signalée dans les résultats. Le calcul
    /// du Bbio emploie toujours 1 (éq. 414).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdep: Option<f64>,
    /// « Acond » : surface externe du conduit, en m², saisie par l'utilisateur
    /// (`Is_surf_cond_def = 0`, §6.2.3.4) ; en maison individuelle, cette possibilité est
    /// conditionnée à un test d'étanchéité à la réception (note 6). Absente : surface par défaut
    /// (éq. 421 à 427). Toutes les bouches d'un système la portent, ou aucune.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a_cond_m2: Option<f64>,
    /// « qspec,rep,rafnoc » ou « qspec,souf,rafnoc » (selon `isouf`) : débit volumique
    /// spécifique de la bouche en rafraîchissement par surventilation mécanique, en m³/h,
    /// positif (§6.12.2, éq. 758 et 759). Requis pour chaque bouche d'un système qui porte
    /// `rafraichissement_surventilation`, sans objet sinon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_spec_rafnoc_m3_h: Option<f64>,
}

/// « Isouf » (§6.2.2) : sens d'un circuit de ventilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Isouf {
    /// 0 : extraction (bouches et conduits de reprise).
    Reprise,
    /// 1 : insufflation (bouches et conduits de soufflage).
    Soufflage,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn est_nul(x: &f64) -> bool {
    *x == 0.0
}
