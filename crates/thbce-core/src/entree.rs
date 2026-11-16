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
    /// accolées. Requis en logement collectif ; 1 par défaut en maison individuelle.
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
    /// « Agr » : surface utile du groupe, en m² (§4.1.2). Requise pour tous les usages : elle
    /// répartit les apports internes entre groupes. Hors habitation, c'est aussi la surface de
    /// référence « SU » (§13.1.3, éq. 2376 ; décision Q28). En maison individuelle, on saisit en
    /// pratique `su_m2 = shab_m2`. En logement collectif, le calcul en déduit sa propre surface
    /// habitable, `Rat_hab · Az` (éq. 33), distincte de la SHAB saisie comme surface de référence.
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

/// Th-BCE 2020 §3.2 — masques solaires (identifiants du texte : `E_AZ`, `E_Az_cad`, `E_LV`,
/// `vd`, `vg`, `h`).
///
/// Les masques proches ne portent que sur les composants verticaux (§3.2.3) ; les masques
/// lointains portent sur toute inclinaison.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Masque {
    /// Masque lointain défini par tranches azimutales (`E_AZ`).
    LointainAzimutal {
        /// « γi » : hauteur de l'horizon vu du centre du composant, en degrés (0 à 90), pour
        /// chacune des 36 tranches de 10° (§3.2.3). Les tranches sont repérées par rapport à la
        /// normale au composant : la tranche 1 commence à l'opposé de la normale (Φ = −180°) et
        /// les tranches 18 et 19 encadrent la normale (figure 10).
        gamma_i_deg: Vec<f64>,
    },
    /// Masque lointain constitué d'arbres à feuilles caduques (`E_Az_cad`) : mêmes tranches,
    /// actif pendant la seule saison végétative conventionnelle.
    LointainArbresCaducs {
        /// « γi », comme pour le masque lointain azimutal.
        gamma_i_deg: Vec<f64>,
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

/// Th-BCE 2020 §5.17.2 — paramètres intrinsèques d'une paroi opaque.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParoiType {
    /// Identifiant du type de paroi.
    pub id: Id,
    /// « Uk » : coefficient de transmission thermique, en W/(m².K), dans la position `incl_k`.
    pub u_k_w_m2k: f64,
    /// « Inclk » : position pour laquelle `Uk` est donné. Obligatoire, sans valeur par défaut :
    /// une position supposée changerait U en silence. Le moteur en déduit le coefficient de la
    /// paroi dans son inclinaison réelle (§5.17.3, éq. 294). Correspondance avec les règles
    /// Th-Bat, d'après les résistances superficielles cumulées que le texte retire de `1/Uk` :
    /// `verticale` pour un U de paroi verticale (0,17 m².K/W) ; `horizontale_flux_ascendant` pour
    /// un U de plancher haut (0,14) ; `horizontale_flux_descendant` pour un U de plancher bas
    /// (0,21).
    pub incl_k: PositionParoi,
    /// « Sf-c,k » : facteur solaire pour le calcul du Cep (paroi non végétalisée).
    pub sf_c_k: f64,
    /// « Sf-e,k » : facteur solaire pour le calcul de DH (paroi non végétalisée).
    pub sf_e_k: f64,
    /// Toiture végétalisée (« Typevegetal » = 1) et ses facteurs solaires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vegetalisation: Option<Vegetalisation>,
}

/// « Inclk » (§5.17.2) : position de référence du coefficient `Uk` d'une paroi opaque.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PositionParoi {
    /// 1 : paroi verticale (Th-Bat : paroi verticale, flux horizontal).
    Verticale,
    /// 2 : paroi horizontale, flux ascendant (Th-Bat : plancher haut).
    HorizontaleFluxAscendant,
    /// 3 : paroi horizontale, flux descendant (Th-Bat : plancher bas).
    HorizontaleFluxDescendant,
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
    /// (indice « ap »). Requis si `protection_mobile` est renseignée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avec_protection: Option<FacteursBaie>,
    /// Première protection mobile de la baie et sa gestion (§5.9.2). Absente : baie sans
    /// protection mobile (`Type-PM = 0`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protection_mobile: Option<ProtectionMobile>,
}

/// Th-BCE 2020 §5.9.2 et §5.12.2 — protection mobile d'une baie et sa gestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProtectionMobile {
    /// « Type-PM » : type de la première protection mobile. Le store vénitien n'est pas pris en
    /// charge dans cette version : la validation le refuse.
    pub type_pm: TypePm,
    /// « Type-GPM » : gestion de la première protection mobile. La gestion automatique n'est pas
    /// prise en charge dans cette version : la validation la refuse.
    pub gestion: GestionPm,
    /// « Protext » : vrai si la protection est extérieure, faux si elle est intérieure.
    pub exterieure: bool,
    /// « PM2 » : présence d'une seconde protection mobile, intérieure, gérée manuellement, dont
    /// la seule fonction est d'éviter l'éblouissement.
    pub seconde_protection: bool,
    /// « Typo_PM » : typologie de la protection pour le passage de l'air quand elle est fermée
    /// (§5.12.3.1, tableau 40). Absente : typologie 0, « valeur par défaut » du texte (taux de
    /// passage de 0 %), signalée dans les résultats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typo_pm: Option<TypoPm>,
}

/// « Type-PM » (§5.9.2) : type de la première protection mobile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypePm {
    /// 1 : volet.
    Volet,
    /// 2 : store enroulable.
    StoreEnroulable,
    /// 3 : store vénitien (à lames orientables). Non pris en charge dans cette version.
    StoreVenitien,
}

/// « Type-GPM » (§5.9.2) : gestion d'un volet ou d'un store enroulable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GestionPm {
    /// 1 : automatique avec dérogation. Non prise en charge dans cette version.
    AutomatiqueAvecDerogation,
    /// 2 : manuelle non motorisée.
    ManuelleNonMotorisee,
    /// 3 : manuelle motorisée (y compris avec commande générale dérogeable, §5.9.3.1).
    ManuelleMotorisee,
}

/// « Typo_PM » (§5.12.3.1, tableau 40) : typologie d'une protection mobile selon son taux de
/// passage d'air en position fermée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypoPm {
    /// 0 : valeur par défaut, volets isolants ; 0 %.
    Etanche,
    /// 1 : tablier non ou faiblement ajouré (moins de 10 % de sa surface) ; 10 %. Exemples du
    /// texte : volets roulants, battants ou coulissants sans projection, stores enroulables
    /// opaques.
    FaiblementAjouree,
    /// 2 : tablier ajouré (10 à 30 %) ; 25 %. Exemples : volets roulants à agrafes ou à
    /// micro-lames ajourées.
    Ajouree,
    /// 3 : tablier fortement ajouré (30 à 50 %) ; 50 %. Exemples : volets battants persiennés
    /// sans projection, moustiquaires.
    FortementAjouree,
    /// 4 : tablier très fortement ajouré (plus de 50 %) ou protection à projection ; 75 %.
    TresFortementAjouree,
}

/// Facteurs solaires et lumineux d'une baie dans un état de protection (§5.10.2).
///
/// Trois grandeurs ont une valeur par défaut donnée par la méthode (§5.10.3.6.3) et sont
/// facultatives : les facteurs solaires « -c » (éq. 215 et 216) et `tlid` (éq. 218). Toute
/// valeur par défaut appliquée est listée dans les résultats (`valeurs_par_defaut`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FacteursBaie {
    /// « Sw1-c » : composante courte longueur d'onde (directe), calcul du Cep.
    ///
    /// Les trois composantes « -c » se saisissent ensemble ou pas du tout. Absentes, la valeur
    /// par défaut du texte s'applique au facteur solaire global `Sw-c = Sw1-c + Sw2-c + Sw3-c` :
    /// sans protection mobile, `Swsp-c = Swsp-e − 0,1` (éq. 215) ; avec protection mobile,
    /// `Swap-c = Swap-e` (éq. 216). Le texte renvoie aux règles Th-S pour la décomposition en
    /// trois composantes : le moteur garde les proportions des composantes « -e » (question
    /// Q30).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sw1_c: Option<f64>,
    /// « Sw2-c » : composante grande longueur d'onde et convective, calcul du Cep. Voir `sw1_c`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sw2_c: Option<f64>,
    /// « Sw3-c » : composante liée à la lame d'air intérieure ventilée, calcul du Cep. Voir
    /// `sw1_c`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sw3_c: Option<f64>,
    /// « Sw1-e » : composante courte longueur d'onde (directe), calcul de DH.
    pub sw1_e: f64,
    /// « Sw2-e » : composante grande longueur d'onde et convective, calcul de DH.
    pub sw2_e: f64,
    /// « Sw3-e » : composante liée à la lame d'air intérieure ventilée, calcul de DH.
    pub sw3_e: f64,
    /// « Tli » : facteur de transmission lumineuse global.
    pub tli: f64,
    /// « Tlid » : transmission lumineuse du flux direct transmis sous forme diffuse. Absente :
    /// 0, valeur par défaut du texte (éq. 218).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tlid: Option<f64>,
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
    /// Orientation, inclinaison et masques du pont thermique (§5.18.3). Champ facultatif.
    /// Présent : description détaillée. **Absent : la description simplifiée s'applique**, où le
    /// pont thermique est réparti en quatre ponts verticaux orientés au sud, à l'ouest, au nord
    /// et à l'est, d'un quart de la longueur chacun, sans masque (éq. 298).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exposition: Option<Exposition>,
}
