// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : générations et générateurs (Th-BCE 2020 §8.14 et §8.17).
//!
//! Un fichier par famille de générateurs. Un nouveau générateur ajoute sa variante à
//! [`NatureGenerateur`] et ses types dans un fichier de ce dossier, déclaré ci-dessous.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::*;

mod combustion;
mod effet_joule;
mod reseaux_chaleur;
mod thermodynamique;
mod thermodynamique_gaz;

pub use combustion::*;
pub use effet_joule::*;
pub use reseaux_chaleur::*;
pub use thermodynamique::*;
pub use thermodynamique_gaz::*;

/// Th-BCE 2020 §8.14, §8.17.2 et §8.4.2 — génération : ensemble de générateurs qui reçoit les
/// demandes d'un ou de plusieurs réseaux de distribution intergroupes de chauffage, de
/// refroidissement ou d'ECS, et les répartit entre ses générateurs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Generation {
    /// Identifiant de la génération, unique dans le bâtiment.
    pub id: Id,
    /// « idraccord_reseau » (§8.4.2) : type de raccordement de la génération aux réseaux de
    /// distribution. `permanent` : les groupes desservis suivent la saison de fonctionnement de
    /// la génération (éq. 840) ; `avec_isolement` : chaque groupe garde ses propres saisons
    /// (éq. 841).
    pub raccordement_reseaux: RaccordementReseaux,
    /// « idpos » (§8.17.2) : emplacement de la génération, en ou hors volume chauffé. Il fixe
    /// la température d'ambiance des générateurs et des ballons (§8.17.3.5.8) et le caractère
    /// récupérable de leurs pertes (éq. 1058, fiche 11.1).
    pub position: PositionGeneration,
    /// « idtype_priorite » (§8.17.1) : mode de gestion des priorités entre les générateurs.
    /// Requis dès que deux générateurs assurent le même poste ; sans objet sinon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priorite: Option<TypePriorite>,
    /// « idgestion_ch » et « θwm_ch » (§8.17.3.5.10.1) : gestion de la température de
    /// fonctionnement en chauffage. Requise si la génération alimente des réseaux hydrauliques
    /// de chauffage ; sans objet pour une génération sur air ambiant (réseaux fictifs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gestion_temperature_chauffage: Option<GestionTemperatureGeneration>,
    /// « idgestion_fr » et « θwm_fr » : de même en refroidissement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gestion_temperature_refroidissement: Option<GestionTemperatureGeneration>,
    /// Générateurs de la génération (tableau 111). La génération doit pouvoir assurer chacun
    /// des postes de ses réseaux (§8.17.3.1) ; elle compte au moins un générateur ou un
    /// chauffe-eau thermodynamique.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generateurs: Vec<GenerateurSaisi>,
    /// Assemblages de production d'ECS avec stockage (§9.2 et §9.14) : un ballon, un générateur
    /// de base de la génération et, le cas échéant, un appoint intégré. Chaque assemblage est
    /// le premier sollicité pour l'ECS (§8.17.3.6).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stockages: Vec<StockageSaisi>,
    /// Chauffe-eau thermodynamiques décrits par les résultats de leur essai selon la norme
    /// NF EN 16147 (§9.22) : le moteur en déduit le ballon et la pompe à chaleur d'un
    /// assemblage « base seule ».
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chauffe_eau_thermodynamiques: Vec<ChauffeEauThermodynamique>,
    /// Chauffe-eau solaires individuels (§9.14 et §9.15, kits CESI §9.23) : ballon dont le
    /// générateur de base est une boucle solaire, avec un appoint intégré, un appoint séparé
    /// instantané ou sans appoint. Comme les autres assemblages avec stockage, ils sont les
    /// premiers sollicités pour l'ECS (§8.17.3.6).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chauffe_eau_solaires: Vec<ChauffeEauSolaire>,
    /// Systèmes solaires combinés (fiches 9.18 à 9.20) : un ballon à quatre zones chauffé par
    /// une boucle solaire, qui assure l'ECS et une partie du chauffage par l'échangeur d'une
    /// boucle de chauffage en zone 2, avec un appoint intégré en zone 3. Premiers sollicités pour
    /// l'ECS et pour le chauffage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub systemes_solaires_combines: Vec<SystemeSolaireCombine>,
    /// Productions solaires collectives à appoints décentralisés du logement collectif
    /// (fiches 9.16 et 9.17) : CESCI (capteurs collectifs, ballons individuels à appoint
    /// intégré ou séparé) et CESCAI (capteurs et ballon solaire collectifs, ballons
    /// individuels à appoint intégré). Une génération en porte au plus une, qui assure toute son
    /// ECS.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chauffe_eau_solaires_collectifs: Vec<ChauffeEauSolaireCollectif>,
}

/// « idpos » (§8.17.2) : emplacement d'une génération.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PositionGeneration {
    /// 1 : en volume chauffé.
    EnVolumeChauffe,
    /// 0 : hors volume chauffé. `espace_tampon` désigne l'espace tampon qui abrite la
    /// génération, dont le coefficient `btherm(h)` entre dans la température d'ambiance
    /// (§8.17.3.5.8) ; absent, la génération est à l'extérieur (`btherm = 1`).
    HorsVolumeChauffe {
        /// Identifiant de l'espace tampon qui abrite la génération.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        espace_tampon: Option<Id>,
    },
}

/// « idtype_priorite » (§8.17.1) : mode de gestion des priorités entre générateurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypePriorite {
    /// 1 : sans priorité ; la demande est répartie au prorata des puissances nominales.
    SansPriorite,
    /// 2 : en cascade ; le générateur prioritaire est sollicité jusqu'à sa puissance maximale,
    /// le reste passe au suivant (pompe à chaleur et appoint, par exemple).
    Cascade,
    /// 3 : en alternance (§8.17.3.7.2.3) : générateurs d'une même fonction (un seul poste, ou
    /// chauffage et ECS), sollicités par puissance nominale décroissante ; un générateur est
    /// fournit pendant 19 heures où le générateur suivant aurait suffi et est coupé à la 20ᵉ
    /// (« Nbasculement_init » = 20).
    /// Incompatible avec un ballon de stockage (§8.17.3.6).
    Alternance,
}

/// « idgestion » et « θwm » (§8.17.3.5.10.1) : température de fonctionnement d'une génération
/// reliée à des réseaux hydrauliques.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GestionTemperatureGeneration {
    /// 1 : fonctionnement à température moyenne constante.
    Constante {
        /// « θwm » : température moyenne de fonctionnement, en °C (de 0 à 100).
        theta_wm_c: f64,
    },
    /// 2 : fonctionnement à la température moyenne des réseaux de distribution.
    Reseaux,
}

/// Générateur d'une génération (§8.17.2, tableau 111).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateurSaisi {
    /// Identifiant du générateur, unique dans la génération.
    pub id: Id,
    /// « Rdim » (« Ratdim ») : nombre de générateurs identiques (au moins 1).
    pub rdim: u32,
    /// « idpriorite_ch » : rang du générateur en chauffage dans une gestion en cascade (1 : le
    /// premier sollicité). Requis en cascade si plusieurs générateurs chauffent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priorite_chauffage: Option<u32>,
    /// « idpriorite_fr » : de même en refroidissement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priorite_refroidissement: Option<u32>,
    /// « idpriorite_ecs » : de même en ECS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priorite_ecs: Option<u32>,
    /// « idtype » : nature du générateur et ses caractéristiques.
    pub nature: NatureGenerateur,
}

/// « idtype » (§8.17.3.2, tableau 111) : nature d'un générateur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum NatureGenerateur {
    /// Type 500 : générateur direct à effet Joule (convecteurs, panneaux rayonnants, résistances
    /// d'appoint… ; §8.18).
    EffetJoule(EffetJouleSaisi),
    /// Type 503 : générateur thermodynamique à compression électrique (pompe à chaleur,
    /// climatiseur, refroidisseur ; §8.23).
    ThermodynamiqueElectrique(Box<ThermodynamiqueElectriqueSaisi>),
    /// Types 100 à 102, 200, 201 et 400 : chaudière au gaz, au fioul ou au bois (§8.19).
    Chaudiere(Box<ChaudiereSaisie>),
    /// Types 403 et 404 : poêle ou insert à bois (§8.22).
    PoeleOuInsert(PoeleSaisi),
    /// Types 600 et 601 : sous-station d'un réseau de chaleur ou de froid (§8.28).
    ReseauDeChaleurOuDeFroid(ReseauChaleurFroidSaisi),
    /// Types 103 à 109 : radiateurs, chauffe-eau et accumulateurs au gaz, générateurs d'air
    /// chaud, tubes et panneaux radiants (§8.20).
    AutreGenerateurACombustion(Box<AutreGenerateurCombustionSaisi>),
    /// Type 504 : générateur thermodynamique au gaz, à absorption (§8.24) ou à moteur gaz
    /// (§8.25), sur air extérieur.
    ThermodynamiqueGaz(ThermodynamiqueGazSaisi),
    /// Types 507 à 509 : pompe à chaleur sur boucle d'eau, thermo-frigo-pompe, générateur DRV
    /// (§8.15 et §8.16) : hors de la première version.
    BoucleDEauOuDrv,
    /// Type 700 : système de cogénération (§8.21) : hors de la première version.
    Cogeneration,
    /// Pompe à chaleur hybride (association d'une pompe à chaleur et d'une chaudière dans un
    /// même produit) saisie comme un seul générateur : non prise en charge. Elle n'est pas un
    /// type du tableau 111 ; la fiche d'application du §16.4 la décrit par ses composants, à
    /// saisir comme une pompe à chaleur et une chaudière à condensation d'une même génération
    /// en cascade (lecture Q126).
    PacHybride,
}

/// Fonction d'une chaudière ou d'un réseau de chaleur (« idfougen » 1, 3 ou 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FonctionChaleur {
    /// 1 : chauffage.
    Chauffage,
    /// 3 : ECS.
    Ecs,
    /// 4 : chauffage et ECS.
    ChauffageEcs,
}

/// « idraccord_reseau » (§8.4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RaccordementReseaux {
    /// 0 : avec possibilité d'isolement.
    AvecIsolement,
    /// 1 : permanent.
    Permanent,
}
