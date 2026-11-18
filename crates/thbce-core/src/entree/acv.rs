// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : analyse du cycle de vie (ACV) d'un bâtiment, annexe II de l'arrêté du 4
//! août 2021, chapitre 4 (contributions « composants », « chantier », « eau », « parcelle ») et
//! §5.3 (Icconstruction, Icbâtiment, StockC, Icded).
//!
//! **Aucune donnée environnementale n'est fournie par le moteur** : les valeurs d'impact de
//! chaque donnée (FDES, PEP, donnée environnementale par défaut, donnée configurée) et les
//! données de service (eau, assainissement, transport, énergie du chantier) sont des entrées,
//! saisies avec leur identifiant et leur source. Le moteur n'apporte que ce que le texte donne
//! lui-même : formules, pondération dynamique (article 11 de l'arrêté), valeurs forfaitaires de
//! l'annexe XI, valeurs conventionnelles du chapitre 4 (eau, chantier simplifié).
//!
//! Seul l'indicateur de changement climatique est calculé (méthode statique et dynamique) ; les
//! autres indicateurs du tableau 12 ne le sont pas (`docs/couverture.md`).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Id;

/// Données de l'ACV d'un bâtiment (annexe II, chapitre 4). La période d'étude de référence est
/// de 50 ans (annexe II §1.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DonneesAcv {
    /// Composants du bâtiment (contribution « composants », §4.2.1), décrits par lot et
    /// sous-lot (tableau 5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub composants: Vec<ComposantAcv>,
    /// Lots ou sous-lots pris en compte par leur valeur forfaitaire (§4.2.2, valeurs de
    /// l'annexe XI selon l'usage de chaque zone) : `8.1`, `8.3`, `8.4`, `8.5`, `10` ou `11`,
    /// selon l'usage. Un lot forfaitaire ne peut pas avoir de composant décrit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lots_forfaitaires: Vec<String>,
    /// Fluides frigorigènes des équipements des lots traités forfaitairement (§4.2.2.2.2,
    /// cas 3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fluides_lots_forfaitaires: Vec<FluideLotForfaitaire>,
    /// Chantier de construction (§4.5).
    pub chantier: ChantierAcv,
    /// Consommations et rejets d'eau du bâtiment (§4.4, calcul simplifié §4.4.2).
    pub eau: EauAcv,
    /// Parcelle (§4.6) ; absente si la parcelle n'a ni composant, ni comblement, ni usage
    /// d'eau.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parcelle: Option<ParcelleAcv>,
    /// Nombre d'occupants « Nocc » (§5.2.3).
    pub occupants: OccupantsAcv,
    /// Données propres à l'exigence `Icconstruction_max` (annexe de l'article R. 172-4,
    /// chapitre III, III) ; les autres (année de dépôt, combles, HSP, réseau de chaleur) sont
    /// celles de `exigences`.
    #[serde(default)]
    pub exigences: ParametresExigencesAcv,
    /// Données environnementales de service (impact sur le changement climatique, en kg éq.
    /// CO2 par unité), saisies par l'utilisateur avec leur source ; seules celles qu'un calcul
    /// demande sont obligatoires.
    pub donnees_service: DonneesService,
}

/// Données de la modulation de `Icconstruction_max` propres à l'ACV (version post-Rivaton,
/// permis déposés à partir du 1er juillet 2026).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParametresExigencesAcv {
    /// « Sagrément_ext » : surface d'espaces extérieurs (balcons, loggias, terrasses), en m²
    /// (`Miagrément_ext` des logements collectifs) ; 0 si absente.
    #[serde(default)]
    pub s_agrement_ext_m2: f64,
    /// Immeuble de grande hauteur au sens de l'article R. 146-3 du CCH (`Misurf_tot` et
    /// `Icconstruction_maxmoyen` propres).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub immeuble_grande_hauteur: bool,
    /// Présence d'un système de climatisation (`Miclim_RCU`, avec un réseau de chaleur classé).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub climatisation: bool,
}

/// Composant (produit de construction ou équipement) associé à une donnée environnementale
/// (§4.2.1.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ComposantAcv {
    /// Identifiant du composant.
    pub id: Id,
    /// Désignation libre.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub designation: Option<String>,
    /// Sous-lot du tableau 5 (`1.1` à `11.3`), ou lot `12` ou `13` (sans sous-lot). Pour un
    /// composant de la parcelle : désignation du tableau 6 (texte libre).
    pub sous_lot: String,
    /// « Qp » : quantité mise en œuvre, en multiple de l'unité fonctionnelle de la donnée.
    pub quantite: f64,
    /// Unité fonctionnelle (texte libre, pour la traçabilité).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unite: Option<String>,
    /// Donnée environnementale associée.
    pub donnee: DonneeEnvironnementale,
    /// « DVEp » : durée de vie estimée, en années ; par défaut la durée de vie de référence
    /// de la donnée (§4.2.1.1.1). Elle ne peut qu'être inférieure à la DVR.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dve_ans: Option<f64>,
    /// Fluide frigorigène contenu dans l'équipement (§4.2.1.1.6 et §4.2.1.2.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fluide: Option<FluideFrigorigene>,
}

/// Type de donnée environnementale (§2.4.2, §2.5.2, §2.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeDonneeEnvironnementale {
    /// FDES individuelle.
    FdesIndividuelle,
    /// FDES collective.
    FdesCollective,
    /// PEP individuel.
    PepIndividuel,
    /// PEP collectif.
    PepCollectif,
    /// Donnée issue d'un configurateur reconnu (donnée configurée).
    DonneeConfiguree,
    /// Donnée environnementale par défaut (DED) : comptée dans `Icded` (§5.3.4).
    DonneeParDefaut,
}

impl TypeDonneeEnvironnementale {
    /// Donnée par défaut au sens de §5.3.4.
    pub fn est_par_defaut(self) -> bool {
        self == Self::DonneeParDefaut
    }
}

/// Donnée environnementale d'un composant, réduite à l'indicateur de changement climatique,
/// par unité fonctionnelle et par phase du cycle de vie (§4.2.1.1.2, éq. 7 et 8). Valeurs
/// saisies telles que publiées (aucune donnée INIES n'est fournie par le moteur).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DonneeEnvironnementale {
    /// Type de donnée.
    #[serde(rename = "type")]
    pub type_donnee: TypeDonneeEnvironnementale,
    /// Identifiant de la donnée dans sa base (numéro INIES, référence du configurateur…).
    pub identifiant: String,
    /// Source de la donnée (base, configurateur, date de consultation).
    pub source: String,
    /// « DVR » : durée de vie de référence, en années.
    pub dvr_ans: f64,
    /// « DE Production » : modules A1 à A3, en kg éq. CO2 par unité fonctionnelle.
    pub production_kgco2: f64,
    /// « DE Edification » : modules A4 et A5.
    pub edification_kgco2: f64,
    /// « DE Exploitation » : modules B1 à B4 (B1 à B7 non séparés : 0, voir
    /// `exploitation_non_separee`).
    pub exploitation_kgco2: f64,
    /// « DE Fin de vie » : modules C1 à C4.
    pub fin_de_vie_kgco2: f64,
    /// « DE Module D ».
    pub module_d_kgco2: f64,
    /// Modules B1 à B4 non séparables de B6 et B7 : la phase d'exploitation du composant est
    /// exclue (`DE Exploitation = 0`, §4.2.1.1.2) ; un fluide frigorigène est alors compté par
    /// le calcul propre aux fluides (cas 1).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exploitation_non_separee: bool,
    /// Module B1 seul, si le module B est détaillé (cas 2 des fluides frigorigènes,
    /// §4.2.1.2.2, éq. 54 à 60).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub b1_kgco2: Option<f64>,
    /// Module A1 seul, s'il est dissocié (StockC, §5.3.2.1, cas 2b).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a1_kgco2: Option<f64>,
    /// « StockCp » : carbone biogénique stocké, en kg C par unité fonctionnelle, s'il est
    /// renseigné dans la donnée (§5.3.2.1, cas 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stock_c_kgc: Option<f64>,
}

/// Fluide frigorigène d'un composant (§4.2.1.1.6, §4.2.1.2.2). Cas 1 si la donnée n'a pas de
/// module B1 séparé (`b1_kgco2` absent), cas 2 sinon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FluideFrigorigene {
    /// Désignation du fluide (R32, R410A…).
    pub nom: String,
    /// « Chi » : charge initiale par unité fonctionnelle, en kg (charge supplémentaire de
    /// mise en œuvre comprise).
    pub charge_kg: f64,
    /// « PRGi » : potentiel de réchauffement global à 100 ans, en kg éq. CO2/kg.
    pub prg_kgco2_kg: f64,
    /// « DEi » : donnée environnementale conventionnelle du fluide (impacts directs et de
    /// production), en kg éq. CO2/kg, pour le calcul statique (éq. 26, cas 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub de_kgco2_kg: Option<f64>,
}

/// Fluide frigorigène d'un équipement d'un lot traité forfaitairement (§4.2.2.2.2, cas 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FluideLotForfaitaire {
    /// Lot ou sous-lot forfaitaire de l'équipement (`8.1`…).
    pub sous_lot: String,
    /// Désignation du fluide.
    pub nom: String,
    /// « Chi » : charge totale de l'équipement, en kg.
    pub charge_kg: f64,
    /// « PRGi », en kg éq. CO2/kg.
    pub prg_kgco2_kg: f64,
    /// « DEi » : donnée conventionnelle du fluide, en kg éq. CO2/kg (calcul statique, éq. 26).
    pub de_kgco2_kg: f64,
    /// « DVEp » de l'équipement, en années ; 17 ans par défaut (§4.2.2.2.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dve_ans: Option<f64>,
}

/// Chantier de construction (§4.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "methode", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChantierAcv {
    /// Calcul détaillé (§4.5.1) sur les données réelles du projet.
    Detaille {
        /// Énergies consommées (éq. 93).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        energies: Vec<EnergieChantier>,
        /// « Qchantier,eau potable » : eau potable consommée, en m³ (éq. 94).
        eau_potable_m3: f64,
        /// « Qchantier,eaux usées » : eaux usées rejetées, en m³ (éq. 94).
        eaux_usees_m3: f64,
        /// Terres importées (éq. 95).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        terres_importees: Vec<TerresImportees>,
        /// Terres évacuées (éq. 95).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        terres_evacuees: Vec<TerresEvacuees>,
        /// Composants spécifiques au chantier (§4.5.1.1.4).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        composants: Vec<ComposantAcv>,
    },
    /// Calcul simplifié d'une maison individuelle ou accolée (§4.5.2.1.1).
    SimplifieMaison {
        /// Surface de la parcelle, en m² (éq. 100 et 101).
        s_parcelle_m2: f64,
        /// « Semprise au sol » : emprise au sol du bâtiment, en m².
        s_emprise_sol_m2: f64,
        /// Terres importées (distance par défaut 30 km, §4.5.2).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        terres_importees: Vec<TerresImportees>,
    },
    /// Calcul simplifié des autres typologies (§4.5.2.1.2).
    SimplifieAutres {
        /// « nété,grue » : mois d'été (avril à septembre) de chantier avec grue.
        n_ete_grue: f64,
        /// « nhiver,grue » : mois d'hiver (octobre à mars) avec grue.
        n_hiver_grue: f64,
        /// « nété,sans grue ».
        n_ete_sans_grue: f64,
        /// « nhiver,sans grue ».
        n_hiver_sans_grue: f64,
        /// « Qterres excavées » : volume de terres excavées, en m³ (éq. 103).
        v_terres_excavees_m3: f64,
        /// « ε1 » : carburant par m³ excavé, en L/m³ ; 1 par défaut.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        epsilon1_l_m3: Option<f64>,
        /// Terres importées (distance par défaut 30 km, §4.5.2).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        terres_importees: Vec<TerresImportees>,
        /// Terres évacuées (éq. 95, données réelles).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        terres_evacuees: Vec<TerresEvacuees>,
    },
}

/// Énergie consommée sur le chantier (éq. 93).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnergieChantier {
    /// Désignation de l'énergie.
    pub designation: String,
    /// « Qénergie,i » : quantité consommée (kWh ou L).
    pub quantite: f64,
    /// « DEénergie,i » : donnée de service, en kg éq. CO2 par unité.
    pub de_kgco2_unite: f64,
    /// Source de la donnée de service.
    pub source: String,
}

/// Terres importées sur le chantier (éq. 95).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerresImportees {
    /// « Qterres imp,i », en tonnes.
    pub quantite_t: f64,
    /// « di », en km ; 30 km par défaut en calcul simplifié, obligatoire en calcul détaillé.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance_km: Option<f64>,
}

/// Terres évacuées du chantier (éq. 95).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerresEvacuees {
    /// « Qterres évac,j », en tonnes.
    pub quantite_t: f64,
    /// « dj », en km.
    pub distance_km: f64,
    /// « DEtrait,j » : donnée de service du traitement, en kg éq. CO2/t.
    pub de_traitement_kgco2_t: f64,
    /// Source de la donnée de traitement.
    pub source: String,
}

/// Assainissement des eaux usées du bâtiment (§4.4.1.1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Assainissement {
    /// Assainissement collectif (AC).
    Collectif,
    /// Assainissement non collectif (ANC).
    NonCollectif,
}

/// Gestion des eaux pluviales (§4.4.1.1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GestionEauxPluviales {
    /// Infiltration à la parcelle : `DEeaux pluviales = 0`.
    Infiltration,
    /// Réseau séparatif : `DEeaux pluviales = 0`.
    Separatif,
    /// Réseau unitaire : `DEeaux pluviales = DEassainissementAC`.
    Unitaire,
}

/// Consommations et rejets d'eau du bâtiment (§4.4, calcul simplifié §4.4.2 : consommation
/// conventionnelle du tableau 7, sans facteur d'équipement ni eau de pluie).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EauAcv {
    /// Assainissement des eaux usées.
    pub assainissement: Assainissement,
    /// Gestion des eaux pluviales.
    pub eaux_pluviales: GestionEauxPluviales,
    /// « Qpluviométrie » : pluviométrie moyenne annuelle du site, en mm/an (réseau unitaire,
    /// éq. 85 ; parcelle, éq. 112).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pluviometrie_mm: Option<f64>,
    /// « Stoitures » : toitures et couvertures (bâtiment, aires de stationnement couvertes), en
    /// m² (réseau unitaire, éq. 85).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s_toitures_m2: Option<f64>,
    /// « Sparkings imperméabilisés non couverts », en m² (réseau unitaire, éq. 85).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s_parkings_impermeabilises_m2: Option<f64>,
    /// « Qeau potable » du bâtiment saisie, en m³/an (usages internes et classiques, hors
    /// arrosage de la parcelle) : obligatoire quand le tableau 7 de l'annexe II donne « N/C »
    /// pour une zone ou que les zones ont des consommations conventionnelles différentes, et
    /// alors employée à la place de `Qeau potable conv × Nocc` (éq. 91) ; ignorée sinon
    /// (`docs/questions/Q218.md`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q_eau_potable_saisie_m3: Option<f64>,
}

/// Parcelle (§4.6, eau en calcul simplifié §4.6.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParcelleAcv {
    /// Composants de la parcelle (tableau 6).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub composants: Vec<ComposantAcv>,
    /// Matériaux de comblement de cavités (éq. 106).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comblements: Vec<TerresImportees>,
    /// « Sparcelle végétalisée », en m² (éq. 108).
    #[serde(default)]
    pub s_vegetalisee_m2: f64,
    /// « Snettoyée » : surfaces extérieures nettoyées hors stationnement, en m² (éq. 109).
    #[serde(default)]
    pub s_nettoyee_m2: f64,
    /// « SSurfaces imperméabilisées hors parkings », en m² (éq. 112).
    #[serde(default)]
    pub s_impermeabilisees_m2: f64,
}

/// Nombre d'occupants « Nocc » (§5.2.3, éq. 131).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum OccupantsAcv {
    /// Habitation : nombre de logements de chaque type `Ti` ; `Nocc = Σ i × ni`.
    Logements(Vec<LogementsType>),
    /// Autres usages : nombre d'occupants du programme (élèves en enseignement).
    Nombre(f64),
}

/// Logements d'un type `Ti`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LogementsType {
    /// « i » : nombre de pièces principales (T1 = 1…).
    pub type_t: u8,
    /// « ni » : nombre de logements de ce type.
    pub nombre: u32,
}

/// Données environnementales de service, en kg éq. CO2 par unité ; chacune avec sa source.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DonneesService {
    /// « DEeau potable », par m³.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eau_potable_kgco2_m3: Option<f64>,
    /// « DEassainissementAC », par m³.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assainissement_collectif_kgco2_m3: Option<f64>,
    /// « DEassainissementANC », par m³.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assainissement_non_collectif_kgco2_m3: Option<f64>,
    /// « DETrans » : transport de terres par camion, par t.km.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport_terres_kgco2_tkm: Option<f64>,
    /// « DEtrait » des déchets inertes, par t (chantier simplifié d'une maison).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traitement_inertes_kgco2_t: Option<f64>,
    /// Électricité du chantier, par kWh (chantier simplifié, éq. 102).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub electricite_chantier_kgco2_kwh: Option<f64>,
    /// « DEcarbu » : production et combustion d'un litre de carburant (éq. 103 et 106).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carburant_kgco2_l: Option<f64>,
    /// Source des données de service.
    pub source: String,
}
