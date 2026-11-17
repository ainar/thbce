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
    /// Raccordement à un puits climatique ou hydraulique (§6.8 à 6.11) : non pris en charge
    /// dans cette version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub puits: Option<Puits>,
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
    /// Aération par ouverture des fenêtres (§6.7) : non prise en charge dans cette version.
    Aeration,
    /// Centrale de traitement d'air (§6.3, `Type_ventilation_mecaniq` de 2 à 4) : réservée au
    /// non résidentiel (§6.2.3.1.2 : « La CTA est employée uniquement dans le non
    /// résidentiel »).
    CentraleTraitementAir,
}

/// Th-BCE 2020 §6.4.3 et §6.5.2 — ventilateur d'un système simple flux en résidentiel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VentilateurSimpleFlux {
    /// « Pventbase » : puissance électrique du ventilateur en débit de base, en W.
    pub p_vent_base_w: f64,
    /// « Pventpointe » : puissance électrique du ventilateur en débit de pointe, en W.
    pub p_vent_pointe_w: f64,
}

/// Th-BCE 2020 §6.3.2, tableau 61 — groupe de ventilation double flux en habitation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DoubleFlux {
    /// « Pvent,base,rep » : puissance électrique du ventilateur de reprise en débit de base, en W.
    pub p_vent_base_rep_w: f64,
    /// « Pvent,pointe,rep » : puissance électrique du ventilateur de reprise en débit de pointe,
    /// en W.
    pub p_vent_pointe_rep_w: f64,
    /// « Pvent,base,sou » : puissance électrique du ventilateur de soufflage en débit de base, en
    /// W.
    pub p_vent_base_sou_w: f64,
    /// « Pvent,pointe,sou » : puissance électrique du ventilateur de soufflage en débit de
    /// pointe, en W.
    pub p_vent_pointe_sou_w: f64,
    /// Échangeur statique. Absent : pas d'échangeur (`type_échangeur = 0`, éq. 496).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echangeur: Option<Echangeur>,
    /// Batterie de préchauffage (« Is_prechaud ») : non prise en charge dans cette version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prechauffage: Option<bool>,
    /// Humidificateur d'hiver (« Is_ChaudHR ») : non pris en charge dans cette version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub humidification: Option<bool>,
    /// Rafraîchissement évaporatif (« Type_humidification_Eté ») : non pris en charge dans
    /// cette version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rafraichissement_evaporatif: Option<bool>,
    /// Rafraîchissement par surventilation mécanique (§6.12) : non pris en charge dans cette
    /// version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surventilation_mecanique: Option<bool>,
}

/// Th-BCE 2020 §6.3.2, tableau 61, et §6.3.3.4.6 — échangeur d'un double flux.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Echangeur {
    /// Représentation de l'échangeur (« idniveau_detail_echangeur »).
    pub representation: RepresentationEchangeur,
    /// Échangeur rotatif (« Pech », échangeur à roue) : non pris en charge dans cette version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotatif: Option<bool>,
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
    /// 10). Absente : 5 °C, valeur par défaut du tableau 62 (« autres cas »), signalée dans les
    /// résultats.
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
    /// conventionnel en pointe, en m³/h, positif.
    pub q_spec_conv_pointe_m3_h: f64,
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
