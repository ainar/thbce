// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : la description du projet telle que l'utilisateur la fournit.
//!
//! Chaque champ vient d'une nomenclature de l'annexe III, citée en commentaire, et en reprend
//! le nom en `snake_case` suffixé par l'unité. Aucun champ n'est ajouté s'il ne figure pas dans
//! le texte ; les fiches non encore lues en détail (systèmes, éclairage, ventilation, stores
//! vénitiens, espaces tampons) complèteront ces types au moment de leur implémentation.
//!
//! La méthode distingue les paramètres intrinsèques d'un composant de ses paramètres
//! d'intégration dans le projet (§1.2). Les premiers sont décrits une fois dans le
//! [`Catalogue`] ; les seconds sont portés par les instances, qui référencent le catalogue par
//! identifiant.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thbce_data::{Departement, ZoneClimatique};

/// Identifiant textuel choisi par l'utilisateur, unique parmi les objets de même nature.
pub type Id = String;

/// Niveau « projet » : données communes à tous les bâtiments (Th-BCE 2020 §5.1.1, note 1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Projet {
    /// Nom libre du projet.
    pub nom: String,
    /// Situation géographique.
    pub localisation: Localisation,
    /// Composants décrits par leurs paramètres intrinsèques.
    #[serde(default)]
    pub catalogue: Catalogue,
    /// Bâtiments du projet.
    pub batiments: Vec<Batiment>,
}

/// Th-BCE 2020 §3.1.2 et §5.1.2 — paramètres d'intégration du climat et du bâtiment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Localisation {
    /// « département ». Fixe la zone climatique et la température de base.
    pub departement: Departement,
    /// « Alt » : altitude du projet, en m.
    pub alt_m: f64,
    /// « Dmer » : distance à la mer, en km.
    pub dmer_km: f64,
    /// « Durb » : distance à l'îlot urbain, en km.
    pub durb_km: f64,
    /// Zone climatique saisie explicitement. Facultative : le moteur la déduit du département
    /// (arrêté du 26 octobre 2010, annexe I). Une valeur qui contredit le département est
    /// conservée et signalée par un avertissement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone_climatique: Option<ZoneClimatique>,
}

impl Localisation {
    /// Zone climatique retenue : celle saisie, sinon celle du département.
    pub fn zone_climatique_retenue(&self) -> ZoneClimatique {
        self.zone_climatique
            .unwrap_or_else(|| ZoneClimatique::du_departement(self.departement))
    }
}

/// Composants décrits une fois par leurs paramètres intrinsèques.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    /// Types de parois opaques.
    #[serde(default)]
    pub parois: Vec<ParoiType>,
    /// Types de baies vitrées.
    #[serde(default)]
    pub baies: Vec<BaieType>,
    /// Masques solaires.
    #[serde(default)]
    pub masques: Vec<MasqueDefinition>,
}

/// Niveau « bâtiment » (Th-BCE 2020 §5.1) : niveau où s'expriment Bbio, Cep et Cep,nr.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Batiment {
    /// Identifiant du bâtiment.
    pub id: Id,
    /// Zones : parties de bâtiment de même usage, donc de mêmes scénarios (§5.1.3).
    pub zones: Vec<Zone>,
}

/// Niveau « zone » (Th-BCE 2020 §5.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Zone {
    /// Identifiant de la zone.
    pub id: Id,
    /// « Usage zone » (§4.1.2).
    pub usage: UsageZone,
    /// « Nb_lgt » (§4.1.2) : nombre de logements, en logement collectif et pour les maisons
    /// accolées.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nb_lgt: Option<u32>,
    /// Groupes de la zone.
    pub groupes: Vec<Groupe>,
}

/// Th-BCE 2020 §4.1.3, tableau 4 — usages d'une zone.
///
/// Les 28 usages sont reconnus à la lecture. La première version du moteur ne calcule que les
/// usages 1 et 2 (décision Q4) ; les autres sont signalés « non pris en charge ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UsageZone {
    /// 1. Maisons individuelles ou accolées.
    MaisonIndividuelle,
    /// 2. Logements collectifs.
    LogementCollectif,
    /// 3. Bureaux.
    Bureaux,
    /// 4. Enseignement primaire.
    EnseignementPrimaire,
    /// 5. Enseignement secondaire.
    EnseignementSecondaire,
    /// 6. Médiathèques et bibliothèques.
    MediathequeBibliotheque,
    /// 7. Bâtiments universitaires d'enseignement et de recherche, enseignements atypiques.
    Universitaire,
    /// 8. Hôtels 0, 1 et 2 étoiles (partie nuit).
    Hotel012Nuit,
    /// 9. Hôtels 3, 4 et 5 étoiles (partie nuit).
    Hotel345Nuit,
    /// 10. Hôtels 0, 1 et 2 étoiles (partie jour).
    Hotel012Jour,
    /// 11. Hôtels 3, 4 et 5 étoiles (partie jour).
    Hotel345Jour,
    /// 12. Établissements d'accueil de la petite enfance.
    PetiteEnfance,
    /// 13. Restaurants en continu, 18 heures par jour, 7 jours sur 7.
    RestaurantContinu,
    /// 14. Restaurants, 1 repas par jour, 5 jours sur 7.
    Restaurant1Repas5Jours,
    /// 15. Restaurants, 2 repas par jour, 7 jours sur 7.
    Restaurant2Repas7Jours,
    /// 16. Restaurants, 2 repas par jour, 6 jours sur 7.
    Restaurant2Repas6Jours,
    /// 17. Commerces.
    Commerces,
    /// 18. Vestiaires seuls.
    Vestiaires,
    /// 19. Établissements sanitaires avec hébergement.
    SanitaireAvecHebergement,
    /// 20. Établissements de santé (partie nuit).
    SanteNuit,
    /// 21. Établissements de santé (partie jour).
    SanteJour,
    /// 22. Aérogares.
    Aerogare,
    /// 23. Industries ou artisanats 3x8h.
    Industrie3x8,
    /// 24. Industries ou artisanats 8h à 18h.
    Industrie8h18h,
    /// 25. Établissements sportifs municipaux ou scolaires.
    SportMunicipalScolaire,
    /// 26. Restaurants scolaires, 1 repas par jour, 5 jours sur 7.
    RestaurantScolaire1Repas,
    /// 27. Restaurants scolaires, 3 repas par jour, 5 jours sur 7.
    RestaurantScolaire3Repas,
    /// 28. Établissements sportifs privés.
    SportPrive,
}

impl UsageZone {
    /// Numéro d'usage du tableau 4 (§4.1.3).
    pub fn numero(self) -> u8 {
        self as u8 + 1
    }

    /// Usages d'habitation au sens de la surface de référence (§13.1.3 : « Usage = 1 ou 2 »).
    pub fn est_habitation(self) -> bool {
        matches!(self, Self::MaisonIndividuelle | Self::LogementCollectif)
    }

    /// Usage calculé par la version courante du moteur (décision Q4).
    pub fn est_pris_en_charge(self) -> bool {
        self.est_habitation()
    }
}

/// Niveau « groupe » (Th-BCE 2020 §5.7) : local ou ensemble de locaux dont l'air est supposé
/// homogène en température et en humidité (§5.21.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Groupe {
    /// Identifiant du groupe.
    pub id: Id,
    /// « SHAB » : surface habitable, en m². Requise pour les usages d'habitation, dont elle est
    /// la surface de référence (§13.1.3, éq. 2375).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shab_m2: Option<f64>,
    /// « SU » : surface utile réglementaire, en m². Requise pour les autres usages (éq. 2376).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub su_m2: Option<f64>,
    /// « Categorie_CE1_CE2 » (§13.1.2).
    pub categorie_ce1_ce2: CategorieCe,
    /// Inertie thermique du groupe (§5.21.2).
    pub inertie: Inertie,
    /// Parois opaques (§5.15 à 5.17).
    #[serde(default)]
    pub parois_opaques: Vec<ParoiOpaque>,
    /// Baies vitrées (§5.8, 5.10).
    #[serde(default)]
    pub baies: Vec<Baie>,
    /// Ponts thermiques (§5.18 à 5.20).
    #[serde(default)]
    pub ponts_thermiques: Vec<PontThermique>,
}

/// Catégorie de contrainte extérieure du groupe (§13.1.2 : « 1 = CE1 / 2 = CE2 »).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum CategorieCe {
    /// CE1.
    #[serde(rename = "CE1")]
    Ce1,
    /// CE2.
    #[serde(rename = "CE2")]
    Ce2,
}

/// Th-BCE 2020 §5.21.2 et §5.21.3 — inertie du groupe sur trois horizons.
///
/// `Ams_surf` et `Ama_surf` « sont saisies dans tous les cas » (§5.21.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Inertie {
    /// Inertie quotidienne (« Typeinertie_quo », tableau 53).
    pub quotidienne: InertieQuotidienne,
    /// Inertie séquentielle (« Typeinertie_seq », tableau 54).
    pub sequentielle: InertieSequentielle,
    /// « Ams_surf » : surface d'échange équivalente au sens de l'inertie séquentielle, en m²/m².
    pub ams_surf_m2_m2: f64,
    /// Inertie annuelle (« Typeinertie_ann »).
    pub annuelle: InertieAnnuelle,
    /// « Ama_surf » : surface d'échange équivalente au sens de l'inertie annuelle, en m²/m².
    pub ama_surf_m2_m2: f64,
}

/// « Typeinertie_quo » (§5.21.3, tableau 53).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InertieQuotidienne {
    /// 0 : personnalisée, saisie directe de `Amq_surf` (m²/m²) et `Cmq_surf` (kJ/(K.m²)).
    Personnalisee {
        /// « Amq_surf ».
        amq_surf_m2_m2: f64,
        /// « Cmq_surf ».
        cmq_surf_kj_k_m2: f64,
    },
    /// 1 : très légère.
    TresLegere,
    /// 2 : légère.
    Legere,
    /// 3 : moyenne.
    Moyenne,
    /// 4 : lourde.
    Lourde,
    /// 5 : très lourde.
    TresLourde,
}

/// « Typeinertie_seq » (§5.21.3, tableau 54).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InertieSequentielle {
    /// 0 : personnalisée, saisie directe de `Cms_surf` (kJ/(K.m²)).
    Personnalisee {
        /// « Cms_surf ».
        cms_surf_kj_k_m2: f64,
    },
    /// 1 : par défaut.
    ParDefaut,
    /// 2 : très légère.
    TresLegere,
    /// 3 : légère.
    Legere,
    /// 4 : moyenne.
    Moyenne,
    /// 5 : lourde.
    Lourde,
}

/// « Typeinertie_ann » (§5.21.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InertieAnnuelle {
    /// 0 : personnalisée, saisie directe de `Cma_surf` (kJ/(K.m²)).
    Personnalisee {
        /// « Cma_surf ».
        cma_surf_kj_k_m2: f64,
    },
    /// 1 : par défaut, même valeur que l'inertie séquentielle (éq. 316).
    ParDefaut,
}

/// Th-BCE 2020 §3.2.2 — paramètres d'intégration d'une surface exposée au rayonnement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Exposition {
    /// « α » : azimut, en degrés (0 : sud ; 90 : ouest ; 180 : nord ; 270 : est).
    pub azimut_deg: f64,
    /// « β » : inclinaison, en degrés (0 : horizontale vers le haut ; 90 : verticale ;
    /// 180 : horizontale vers le bas).
    pub inclinaison_deg: f64,
    /// « lpb » : largeur de la paroi, en m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lpb_m: Option<f64>,
    /// « hpb » : hauteur de la paroi, en m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hpb_m: Option<f64>,
    /// « Id_masque » : identifiants des masques protégeant la surface (« Nmasque », 0 à 7).
    #[serde(default)]
    pub masques: Vec<Id>,
}

/// Masque du catalogue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MasqueDefinition {
    /// Identifiant du masque.
    pub id: Id,
    /// Géométrie du masque.
    pub masque: Masque,
}

/// Th-BCE 2020 §3.2 — masques solaires.
///
/// Le masque lointain constitué d'arbres à feuilles caduques (§3.2.1) sera ajouté à la lecture
/// détaillée de la fiche (lot 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Masque {
    /// Masque lointain défini par tranches azimutales.
    LointainAzimutal {
        /// Tranches azimutales.
        tranches: Vec<TrancheAzimutale>,
    },
    /// Masque lointain de type plan vertical éloigné.
    LointainPlanVertical {
        /// « dE » : distance entre le masque et la paroi, en m.
        de_m: f64,
        /// « hpE » : hauteur du masque par rapport au centre de la paroi, en m.
        hpe_m: f64,
    },
    /// Masque proche vertical droit.
    ProcheVerticalDroit {
        /// « dvd » : profondeur du masque, en m.
        dvd_m: f64,
        /// « dpd » : distance entre la paroi et le masque, en m.
        dpd_m: f64,
    },
    /// Masque proche vertical gauche.
    ProcheVerticalGauche {
        /// « dvg » : profondeur du masque, en m.
        dvg_m: f64,
        /// « dpg » : distance entre la paroi et le masque, en m.
        dpg_m: f64,
    },
    /// Masque proche horizontal.
    ProcheHorizontal {
        /// « dhm » : profondeur du masque, en m.
        dhm_m: f64,
        /// « dhp » : distance entre la paroi et le masque, en m.
        dhp_m: f64,
    },
}

/// Tranche d'un masque lointain azimutal (§3.2.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrancheAzimutale {
    /// « Az » : angle de la tranche azimutale, en degrés.
    pub az_deg: f64,
    /// « γi » : hauteur de l'horizon vu du centre de la paroi, en degrés (0 à 90).
    pub gamma_deg: f64,
}

/// Th-BCE 2020 §5.17.2 — paramètres intrinsèques d'une paroi opaque.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParoiType {
    /// Identifiant du type de paroi.
    pub id: Id,
    /// « Uk » : coefficient de transmission thermique, en W/(m².K).
    pub u_k_w_m2k: f64,
    /// « Sf-c,k » : facteur solaire pour le calcul du Cep (paroi non végétalisée).
    pub sf_c_k: f64,
    /// « Sf-e,k » : facteur solaire pour le calcul de DH (paroi non végétalisée).
    pub sf_e_k: f64,
    /// Toiture végétalisée (« Typevegetal » = 1) et ses facteurs solaires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vegetalisation: Option<Vegetalisation>,
}

/// Facteurs solaires d'une toiture végétalisée (§5.17.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Vegetalisation {
    /// « Sf-c,v,k » : facteur solaire pour le calcul du Cep.
    pub sf_c_v_k: f64,
    /// « Sf-e,v,k » : facteur solaire pour le calcul de DH.
    pub sf_e_v_k: f64,
}

/// Paroi opaque d'un groupe : paramètres d'intégration (§5.17.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParoiOpaque {
    /// Identifiant de la paroi.
    pub id: Id,
    /// Référence à un [`ParoiType`] du catalogue.
    pub type_paroi: Id,
    /// « Ak » : surface de la paroi, en m².
    pub a_k_m2: f64,
    /// Orientation, inclinaison et masques.
    pub exposition: Exposition,
}

/// Th-BCE 2020 §5.10.2 — paramètres intrinsèques d'une baie vitrée.
///
/// Les matrices de facteurs des stores vénitiens (`M_Sw_BC`, `M_Sw_E`…) seront ajoutées avec la
/// fiche 5.10 (lot 4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BaieType {
    /// Identifiant du type de baie.
    pub id: Id,
    /// « Usp,vert,b » : U de la baie verticale sans protection mobile en place, en W/(m².K).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub u_sp_vert_w_m2k: Option<f64>,
    /// « Usp,horiz,b » : U de la baie horizontale sans protection mobile en place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub u_sp_horiz_w_m2k: Option<f64>,
    /// « Uap,vert,b » : U de la baie verticale avec protection mobile en place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub u_ap_vert_w_m2k: Option<f64>,
    /// « Uap,horiz,b » : U de la baie horizontale avec protection mobile en place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub u_ap_horiz_w_m2k: Option<f64>,
    /// Facteurs de la baie sans protection mobile en place (indice « sp »).
    pub sans_protection: FacteursBaie,
    /// Facteurs de la baie avec protection mobile de type volet ou store enroulable en place
    /// (indice « ap »).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avec_protection: Option<FacteursBaie>,
}

/// Facteurs solaires et lumineux d'une baie dans un état de protection (§5.10.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FacteursBaie {
    /// « Sw1-c » : composante courte longueur d'onde (directe), calcul du Cep.
    pub sw1_c: f64,
    /// « Sw2-c » : composante grande longueur d'onde et convective, calcul du Cep.
    pub sw2_c: f64,
    /// « Sw3-c » : composante liée à la lame d'air intérieure ventilée, calcul du Cep.
    pub sw3_c: f64,
    /// « Sw1-e » : composante courte longueur d'onde (directe), calcul de DH.
    pub sw1_e: f64,
    /// « Sw2-e » : composante grande longueur d'onde et convective, calcul de DH.
    pub sw2_e: f64,
    /// « Sw3-e » : composante liée à la lame d'air intérieure ventilée, calcul de DH.
    pub sw3_e: f64,
    /// « Tli » : facteur de transmission lumineuse global.
    pub tli: f64,
    /// « Tlid » : transmission lumineuse du flux direct transmis sous forme diffuse.
    pub tlid: f64,
}

/// Baie vitrée d'un groupe : paramètres d'intégration (§5.10.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Baie {
    /// Identifiant de la baie.
    pub id: Id,
    /// Référence à un [`BaieType`] du catalogue.
    pub type_baie: Id,
    /// « Ab » : surface de la baie, en m².
    pub a_b_m2: f64,
    /// Orientation, inclinaison et masques.
    pub exposition: Exposition,
}

/// Th-BCE 2020 §5.19.2 — pont thermique.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PontThermique {
    /// Identifiant du pont thermique.
    pub id: Id,
    /// « Ψl » : coefficient de transmission thermique linéique, en W/(m.K).
    pub psi_l_w_mk: f64,
    /// « ll » : longueur, en m.
    pub l_l_m: f64,
    /// « Sf-c,l » : facteur solaire pour le calcul du Cep, en m⁻¹.
    pub sf_c_l: f64,
    /// « Sf-e,l » : facteur solaire pour le calcul de DH, en m⁻¹ (le texte dit « Tic » :
    /// décision Q14).
    pub sf_e_l: f64,
}
