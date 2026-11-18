// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : eau chaude sanitaire — stockages et ballons (§9.14), chauffe-eau
//! thermodynamiques (§9.22), émission et distributions (§9.5 à 9.8).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::*;

/// Th-BCE 2020 §9.14.3.1, tableau 291 — assemblage d'un ballon de stockage d'ECS et de ses
/// générateurs : « base seule » sans `appoint`, « base et appoint intégré » avec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StockageSaisi {
    /// Identifiant de l'assemblage, unique dans la génération.
    pub id: Id,
    /// « Nbassemblage » : nombre d'assemblages identiques (au moins 1) ; le générateur de base
    /// désigné représente alors autant de générateurs identiques (`rdim`).
    pub nb_assemblages: u32,
    /// Ballon de stockage (§9.9).
    pub ballon: BallonSaisi,
    /// Générateur de base, dont l'échangeur ou la résistance est en partie basse du ballon
    /// (« zbase = 1 », éq. 1947).
    pub base: GenerateurBallon,
    /// Générateur d'appoint intégré au ballon (« Type_prod_stockage = 1 ») ; absent : base
    /// seule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appoint: Option<AppointBallon>,
}

/// Th-BCE 2020 §9.9.2, tableau 281 — ballon de stockage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BallonSaisi {
    /// « Vtot » : volume du ballon, en L.
    pub v_tot_l: f64,
    /// « (UA)s » et « Statut_UA » : coefficient de pertes thermiques du ballon.
    pub pertes: PertesBallon,
    /// « θmax » : température maximale du ballon, en °C (de 0 à 100).
    pub theta_max_c: f64,
}

/// « (UA)s » et « Statut_UA » (§9.9.3.1, éq. 1766 à 1768).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PertesBallon {
    /// Valeur certifiée, employée telle quelle (éq. 1766).
    Certifie {
        /// « UAs », en W/K.
        ua_s_w_k: f64,
    },
    /// Valeur justifiée par un essai, augmentée de 10 % (éq. 1767).
    Justifie {
        /// « UAs », en W/K.
        ua_s_w_k: f64,
    },
    /// Valeur par défaut selon la nature du ballon (tableau 282 et éq. 1768), signalée dans les
    /// résultats.
    Defaut {
        /// Nature du ballon.
        nature: NatureBallonSaisie,
    },
}

/// Nature d'un ballon pour ses pertes par défaut (tableau 282).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NatureBallonSaisie {
    /// Ballon d'ECS à effet Joule horizontal.
    EffetJouleHorizontal,
    /// Ballon d'ECS à effet Joule vertical.
    EffetJouleVertical,
    /// Autres ballons.
    Autre,
}

/// « typegest » (§9.10.3.2.1) : programmation du thermostat d'un générateur de ballon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProgrammationBallon {
    /// 0 : fonctionnement permanent.
    Permanent,
    /// 1 : de nuit seulement (heure légale inférieure à 5 h, éq. 1811).
    Nuit,
    /// 2 : de jour seulement (heure légale entre 10 h et 17 h, éq. 1812).
    Jour,
}

/// Générateur de base d'un ballon et sa régulation (§9.10.2 et §9.14.3.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateurBallon {
    /// Identifiant d'un générateur de la génération assurant l'ECS : générateur à effet Joule
    /// (résistance) ou générateur thermodynamique pourvu d'un mode `ecs`.
    pub generateur: Id,
    /// « typegest » : programmation du thermostat.
    pub programmation: ProgrammationBallon,
    /// « zreg » : numéro de la zone du ballon, de 1 (bas) à 4 (haut), qui porte la sonde de
    /// régulation.
    pub z_reg: u8,
    /// « Δθ » : hystérésis du thermostat, en K. Absente : 2 K, valeur par défaut du
    /// §9.10.3.2.2, signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_theta_k: Option<f64>,
    /// « hrelech » : hauteur relative de l'échangeur à partir du bas de sa zone, de 0 à 1.
    /// Requise pour un générateur thermodynamique (elle fixe la température vue par son
    /// condenseur, éq. 1781 à 1785) ; sans objet pour une résistance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h_rel_ech: Option<f64>,
}

/// Générateur d'appoint intégré à un ballon (§9.14.3.1, « Type_prod_stockage = 1 »).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AppointBallon {
    /// Identifiant d'un générateur à effet Joule de la génération assurant l'ECS.
    pub generateur: Id,
    /// « typegest_ap ».
    pub programmation: ProgrammationBallon,
    /// « zap » : numéro de la zone la plus basse chauffée par l'appoint, de 1 à 4 (3 pour un
    /// appoint à mi-hauteur).
    pub z_ap: u8,
    /// « zreg_ap » : numéro de la zone de la sonde de régulation de l'appoint.
    pub z_reg: u8,
    /// « faux » : fraction du volume du ballon chauffée par l'appoint, strictement entre 0 et
    /// 1. Absente : 0,5, valeur par défaut du §9.9.3.1, signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub f_aux: Option<f64>,
    /// « Δθap » : hystérésis, en K. Absente : 2 K.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_theta_k: Option<f64>,
}

/// Th-BCE 2020 §9.22 — chauffe-eau thermodynamique à compression électrique décrit par son
/// essai selon la norme NF EN 16147 (étapes C à F en fonctionnement thermodynamique pur).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChauffeEauThermodynamique {
    /// Identifiant, unique dans la génération.
    pub id: Id,
    /// « Nbassemblage » : nombre de chauffe-eau identiques (au moins 1).
    pub nb_assemblages: u32,
    /// « VtotIdCET » : volume du ballon, en L.
    pub v_tot_l: f64,
    /// « θmax » : température maximale du ballon, en °C.
    pub theta_max_c: f64,
    /// Résultats de l'essai.
    pub essai: EssaiEn16147,
    /// Source amont de la pompe à chaleur (« Typesource_chaleur », éq. 2256 à 2258) : air
    /// extérieur, air ambiant d'un espace tampon ou air extrait.
    pub source_amont: SourceAmontSaisie,
    /// « typegest_base » : programmation du thermostat.
    pub programmation: ProgrammationBallon,
    /// « Lim_Theta » de la pompe à chaleur en ECS. Absent : pas de limite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limites: Option<LimitesPac>,
}

/// Résultats d'un essai selon la norme NF EN 16147 (§9.22.2, tableau 304).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EssaiEn16147 {
    /// « Idcycle » : cycle de soutirage.
    pub cycle: CycleSoutirage,
    /// « th » : durée de mise en température, en h (au plus 48).
    pub t_h_h: f64,
    /// « Pes » : puissance absorbée en régime stabilisé, en W.
    pub p_es_w: f64,
    /// « COPDHW » : coefficient de performance sur le cycle de soutirage.
    pub cop_dhw: f64,
    /// « θ'WH » : température d'eau chaude de référence, en °C.
    pub theta_wh_c: f64,
    /// « Isaux » : vrai si les auxiliaires de la pompe à chaleur sont compris dans les
    /// résultats de l'essai ; sinon `Taux = 0,02` s'applique (éq. 2250 et 2251).
    pub auxiliaires_compris: bool,
    /// Statut des résultats : certifiés ou justifiés (éq. 2252 à 2255).
    pub statut: StatutMesure,
}

/// « Idcycle » (§9.22.3.2.3, tableau 305).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CycleSoutirage {
    /// Cycle S.
    S,
    /// Cycle M.
    M,
    /// Cycle L.
    L,
    /// Cycle XL.
    Xl,
    /// Cycle XXL.
    Xxl,
}

/// Th-BCE 2020 §9.5 et §9.6 — émetteur d'ECS équivalent d'un groupe : émetteurs et appareils
/// sanitaires d'un même groupe reliés à une même génération (§9.5.3.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmetteurEcs {
    /// Identifiant, unique dans le groupe.
    pub id: Id,
    /// « Ratem » : part de la surface utile du groupe desservie ; la somme sur les émetteurs
    /// d'ECS du groupe vaut 1 (éq. 1673).
    pub rat_em: f64,
    /// « Nbmaison » (maison individuelle ou accolée : nombre de maisons desservies, plus de 1
    /// pour des maisons accolées) ou « Nblgt-e » (logement collectif : nombre de logements
    /// desservis), tableau 272. Requis, au moins 1, en maison individuelle ou accolée et en
    /// logement collectif ; sans objet dans les autres usages.
    #[serde(default, skip_serializing_if = "est_zero")]
    pub nb_logements: u32,
    /// « Nu » : nombre d'unités caractéristiques desservies, pour les usages autres que la
    /// maison individuelle ou accolée et le logement collectif (§9.6.3.2.2.1, tableau 277 : en
    /// m² de surface utile pour tous ces usages). Absent : la surface utile que l'émetteur
    /// dessert, `rat_em × su_m2` (éq. 1682). Ne pas saisir en habitation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nu: Option<f64>,
    /// « partmélangeur » : part des besoins passant par des mélangeurs, mitigeurs mécaniques et
    /// autres émetteurs (gain 0 %, tableau 273).
    pub part_melangeur: f64,
    /// « partmitigeur » : part passant par des mitigeurs thermostatiques ou mécaniques économes
    /// (classe C3 ou CH3 ; gain 5 %).
    pub part_mitigeur: f64,
    /// « parttemporisateur » : part passant par des robinets électroniques ou des temporisateurs
    /// (gain 7 %).
    pub part_temporisateur: f64,
    /// « App_ecs » : appareils sanitaires (tableau 275) ; s'il y en a de natures différentes,
    /// le plus défavorable.
    pub appareils: AppareilsEcs,
    /// « Irecup » : récupérateur de chaleur sur les eaux grises (fiche 9.21) : non pris en
    /// charge dans cette version. Absent : faux.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recuperateur_eaux_grises: Option<bool>,
    /// Réseau de distribution d'ECS du groupe, propre à l'émetteur (§9.7).
    pub distribution: DistributionEcsGroupe,
    /// Identifiant du réseau de distribution d'ECS intergroupe du bâtiment (§9.8).
    pub distribution_intergroupe: Id,
}

fn est_zero(n: &u32) -> bool {
    *n == 0
}

/// « App_ecs » (§9.5.3, tableau 275).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AppareilsEcs {
    /// Douches seules : gain de 5 %.
    DoucheSeule,
    /// Baignoire sabot (125 L au plus) : 2,5 %.
    BaignoireSabot,
    /// Baignoire standard (plus de 125 L et 175 L au plus) et autre : 0 %.
    BaignoireStandard,
    /// Grande baignoire (plus de 175 L) : −2,5 %.
    GrandeBaignoire,
}

/// Th-BCE 2020 §9.7.2 — distribution d'ECS du groupe : réseau sans circulateur ni isolant,
/// supposé à température constante.
///
/// Classement des longueurs en ou hors volume chauffé : fiche d'application 8.13 (en gaine
/// logement, en volume chauffé).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DistributionEcsGroupe {
    /// « θ2nd-e » : température de la distribution, en °C, au moins égale à celle de l'eau
    /// mitigée (40 °C). Absent : 53 °C, valeur conventionnelle du tableau 278 (lecture Q45),
    /// signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theta_2nd_e_c: Option<f64>,
    /// « Dint_2nd-e » : diamètre intérieur, en m.
    pub d_int_m: f64,
    /// « Lvc_2nd-e » : longueur en volume chauffé d'une distribution, en m. Absent : valeur par
    /// défaut de l'éq. (1729), `6 × A/80` en habitation pour la surface desservie (« δlvc = 0 »),
    /// signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l_vc_m: Option<f64>,
    /// « Lhvc_2nd-e » : longueur hors volume chauffé d'une distribution, en m.
    pub l_hvc_m: f64,
    /// « nbdist_2nd-e » : nombre de distributions identiques reliées à l'émetteur.
    pub nb_dist: u32,
    /// Espace tampon traversé par la partie hors volume chauffé (« btherm », éq. 1733). Absent :
    /// à l'extérieur (`btherm = 1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
}

/// Th-BCE 2020 §9.8 — distribution d'ECS intergroupe, reliée à une génération.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DistributionEcsIntergroupe {
    /// Identifiant, unique dans le bâtiment.
    pub id: Id,
    /// Identifiant de la génération qui produit l'ECS.
    pub generation: Id,
    /// « Typeréseauintergroupe-e ».
    pub reseau: ReseauEcsIntergroupe,
}

/// « Typeréseauintergroupe-e » (§9.8.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ReseauEcsIntergroupe {
    /// 0 : pas de réseau intergroupe (production individuelle) : ni pertes ni auxiliaires
    /// (éq. 1762 à 1765).
    Aucun,
    /// 1 : réseau bouclé (§9.8.3.3).
    Boucle(BoucleEcs),
    /// 2 : réseau tracé, à ruban chauffant (§9.8.3.4).
    Trace(ReseauEcsTrace),
}

/// Réseau d'ECS intergroupe bouclé (§9.8.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoucleEcs {
    /// « Lvc_prim_bcl-e » : longueur en volume chauffé, en m.
    pub l_vc_m: f64,
    /// « Lhvc_prim_bcl-e » : longueur hors volume chauffé, en m.
    pub l_hvc_m: f64,
    /// « Uprim-e » : coefficient linéique, en W/(m.K).
    pub u_w_mk: f64,
    /// « Isrechauf_bcl-e » : réchauffeur de boucle électrique.
    pub rechauffeur: bool,
    /// « Pcirc-e » : puissance des circulateurs, en W (fonctionnement permanent en habitation).
    pub p_circ_w: f64,
    /// « Typegest_circ-e » (§9.8.2, éq. 1747 et 1748) : vrai si les circulateurs sont arrêtés
    /// pendant les vacances des zones d'enseignement desservies (1), faux s'ils fonctionnent en
    /// permanence (0). Requis quand la boucle dessert une zone d'usage affilié à l'enseignement
    /// (usages 4, 5, 7, 26 et 27) ; ne se saisit pas sinon (fonctionnement permanent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arret_circulateurs_vacances: Option<bool>,
    /// Espace tampon traversé par la partie hors volume chauffé (« btherm », éq. 1744).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
}

/// Réseau d'ECS intergroupe tracé (§9.8.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReseauEcsTrace {
    /// « Lvc_prim_trac-e » : longueur en volume chauffé, en m.
    pub l_vc_m: f64,
    /// « Lhvc_prim_trac-e » : longueur hors volume chauffé, en m.
    pub l_hvc_m: f64,
    /// « Uprim-e » : coefficient linéique, en W/(m.K).
    pub u_w_mk: f64,
    /// Espace tampon traversé par la partie hors volume chauffé (« btherm », éq. 1756).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
}
