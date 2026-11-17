// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : enveloppe hors baies — inertie, exposition et masques, parois opaques et
//! ponts thermiques.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::*;

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
        cmq_surf_kj_m2k: f64,
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
        cms_surf_kj_m2k: f64,
    },
    /// 1 : par défaut.
    Defaut,
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
        cma_surf_kj_m2k: f64,
    },
    /// 1 : par défaut, même valeur que l'inertie séquentielle (éq. 316).
    Defaut,
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
    /// Identifiant de l'espace tampon sur lequel donne la paroi (§5.17.3, « Prise en compte des
    /// espaces tampons »). Absent : la paroi donne sur l'extérieur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
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
    /// Identifiant de l'espace tampon sur lequel donne le pont thermique (§5.19.3). Absent : le
    /// pont thermique donne sur l'extérieur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub espace_tampon: Option<Id>,
}
