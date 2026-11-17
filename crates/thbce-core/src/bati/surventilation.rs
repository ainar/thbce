// SPDX-License-Identifier: GPL-3.0-or-later
//! Surventilation naturelle par ouverture des baies : fiche `C_BAT_Surventilation naturelle par
//! ouverture des baies pour le confort thermique` (Th-BCE 2020 §5.13).
//!
//! Périmètre : gestion manuelle de l'ouverture (`Has_Gestion_Auto_Ouverture = 0`), y compris les
//! règles propres au calcul Th-B (§5.13.3.1.2), et calcul du débit du groupe (§5.13.3.2). La
//! gestion automatique de l'ouverture (§5.13.3.1.3) n'est pas prise en charge.
//!
//! Deux éléments de la gestion manuelle ne sont décrits que par un schéma ou pas du tout ; les
//! lectures retenues sont isolées dans [`r_ouv_theta_op_man_selon`] (`docs/questions.md`, Q32).

use libm::{fabs, sqrt};

use crate::conventions::ModeCalcul;
use crate::entree::{ExpoBr, TypeOuvrant};

/// « θei_seuil_bas » : température extérieure en dessous de laquelle les baies restent fermées,
/// en °C (§5.13.3.1.1, éq. 241).
pub const THETA_EI_SEUIL_BAS_C: f64 = 10.0;
/// « θei_seuil_haut » : température extérieure à partir de laquelle l'ouverture n'est plus
/// modérée, en °C (éq. 241).
pub const THETA_EI_SEUIL_HAUT_C: f64 = 18.0;
/// « Δθext_int » en gestion manuelle, en °C (éq. 241) : négatif, les occupants prolongeant
/// l'ouverture quand l'extérieur devient plus chaud que l'intérieur.
pub const DELTA_THETA_EXT_INT_MAN_C: f64 = -3.0;
/// « ρref » : masse volumique de référence de l'air, en kg/m³ (§5.13.2, constantes).
pub const RHO_REF_KG_M3: f64 = 1.2;
/// « θref » : température de référence, en °C (§5.13.2, constantes).
pub const THETA_REF_C: f64 = 19.0;
/// « Cw » : constante de prise en compte de la vitesse du vent (§5.13.2, constantes).
pub const C_W: f64 = 0.001;
/// « Cst » : constante de prise en compte du tirage thermique, en m/(°C.s²) (§5.13.2).
pub const C_ST: f64 = 0.0035;
/// « dCp » : écart des coefficients de pression entre façades au vent et sous le vent (§5.13.2).
pub const D_CP: f64 = 0.75;
/// « Cd_go » : coefficient de décharge des grandes ouvertures (§5.13.2).
pub const CD_GO: f64 = 0.6;
/// « Vventmax » : vitesse de vent maximale prise en compte en traversant, en m/s (§5.13.2).
pub const V_VENT_MAX_M_S: f64 = 3.0;
/// « Httf » dans le cas général, en m (§5.13.3.2.2).
pub const H_TTF_CAS_GENERAL_M: f64 = 1.5;
/// Inclinaison à partir de laquelle une baie est rattachée à une orientation, en degrés
/// (§5.13.3.2.4) ; en dessous, la baie de toiture est conventionnellement sous le vent.
pub const BETA_FACADE_DEG: f64 = 60.0;

/// « Rouvmax,b » par défaut d'un type d'ouvrant.
///
/// Th-BCE 2020 §5.13.3.2.3, tableau 44 : française et anglaise (ouverture à 90°), 80 % ;
/// italienne et soufflet (15°), 40 % ; soufflet d'oscillo-battant (7°), 20 % ; coulissant, 40 %.
pub fn r_ouv_max_par_defaut(type_ouvrant: TypeOuvrant) -> f64 {
    match type_ouvrant {
        TypeOuvrant::FrancaiseOuAnglaise => 0.80,
        TypeOuvrant::ItalienneOuSoufflet => 0.40,
        TypeOuvrant::SouffletOscilloBattant => 0.20,
        TypeOuvrant::Coulissant => 0.40,
    }
}

/// « δtrav_surv » conventionnel d'un usage : caractère traversant du groupe au sens de la
/// surventilation.
///
/// Th-BCE 2020 §5.13.3.2.1, tableau 43 (p. 265 et 266) : traversant pour les maisons
/// individuelles ou accolées (1), les commerces (17), les vestiaires (18), les aérogares (22),
/// les industries (23 et 24) et les établissements sportifs (25 et 28) ; non traversant pour
/// les autres usages, dont les bureaux (3) ; non conventionnel en logement collectif (2), selon
/// le groupe : renvoie `None`, la valeur étant saisie.
pub fn traversant_conventionnel(usage: u8) -> Option<bool> {
    match usage {
        1 | 17 | 18 | 22..=25 | 28 => Some(true),
        3..=16 | 19..=21 | 26 | 27 => Some(false),
        _ => None,
    }
}

/// « Cpr(h) » : modération de l'ouverture selon la plage horaire et l'exposition au bruit.
///
/// Th-BCE 2020 §5.13.3.1.1, tableau 42. En inoccupation, 0. En saison de chauffage (« chauffage
/// sans refroidissement »), 0,3. Hors saison de chauffage : en journée (`hlég` de 7 à 21), 1 en
/// BR1 et 0,7 en BR2 ou BR3 ; la nuit (`hlég` de 22 à 6), 1 en BR1 et 0,3 en BR2 ou BR3.
pub fn c_pr(expo: ExpoBr, i_occ_zone: bool, saison_de_chauffage: bool, hleg: u8) -> f64 {
    if !i_occ_zone {
        return 0.0;
    }
    if saison_de_chauffage {
        return 0.3;
    }
    let journee = (7..=21).contains(&hleg);
    match (expo, journee) {
        (ExpoBr::Br1, _) => 1.0,
        (_, true) => 0.7,
        (_, false) => 0.3,
    }
}

/// « Mod_ext(h) » : modération de l'ouverture selon la température extérieure.
///
/// Th-BCE 2020 §5.13.3.1.1, figure 61 : nul en dessous de `θei_seuil_bas` ; croissant
/// linéairement jusqu'à 1 à `θei_seuil_haut` ; égal à 1 jusqu'à `limite_haute`
/// (`θop(h−1) − Δθext_int`) ; nul au-delà. La figure ne dit pas si les bornes sont incluses :
/// la borne haute est incluse (question Q32).
pub fn mod_ext(theta_ext_c: f64, seuil_bas_c: f64, seuil_haut_c: f64, limite_haute_c: f64) -> f64 {
    if theta_ext_c > limite_haute_c || theta_ext_c <= seuil_bas_c {
        0.0
    } else if theta_ext_c >= seuil_haut_c {
        1.0
    } else {
        (theta_ext_c - seuil_bas_c) / (seuil_haut_c - seuil_bas_c)
    }
}

/// Conditions d'un pas de temps pour la gestion manuelle de l'ouverture des baies d'un groupe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionsOuverture {
    /// Mode de calcul.
    pub mode: ModeCalcul,
    /// « iocc_zone(h) ».
    pub i_occ_zone: bool,
    /// « hlég(h) » : heure légale du pas de temps, de 0 à 23 (`climat::calendrier::hleg`,
    /// §4.1.3).
    pub hleg: u8,
    /// Vrai si la saison du groupe est « chauffage sans refroidissement ».
    pub saison_de_chauffage: bool,
    /// « Autfr,pro(j) » : vrai en période d'autorisation de refroidissement (utilisé en Th-B).
    pub refroidissement_autorise: bool,
    /// « θext(h) » : température extérieure, en °C.
    pub theta_ext_c: f64,
    /// « θop,fin(h−1) », en °C.
    pub theta_op_h1_c: f64,
    /// « θop,fin(h−2) », en °C.
    pub theta_op_h2_c: f64,
    /// « θiifr_+ » : consigne de refroidissement en occupation normale, en °C.
    pub theta_iifr_c: f64,
    /// « Δθconf_adapt(h) » : écart entre la température d'inconfort chaud et la consigne de
    /// refroidissement, en °C.
    pub delta_theta_conf_adapt_c: f64,
}

/// « Mod_ext_man(h) » en gestion manuelle.
///
/// Th-BCE 2020 §5.13.3.1.1, éq. (241) : seuils de 10 et 18 °C, `Δθext_int = −3 °C`.
/// §5.13.3.1.2, en Th-B : nul hors période d'autorisation de refroidissement (éq. 244) ; sinon
/// `θop(h−1) − Δθext_int` est remplacé par `min(θiifr − 1 ; θop(h−1) − Δθext_int)` et
/// `θei_seuil_haut` par `min(θiifr − 1 ; θei_seuil_haut)`.
pub fn mod_ext_man(c: &ConditionsOuverture) -> f64 {
    let limite = c.theta_op_h1_c - DELTA_THETA_EXT_INT_MAN_C;
    if c.mode == ModeCalcul::ThB {
        if !c.refroidissement_autorise {
            return 0.0;
        }
        let plafond = c.theta_iifr_c - 1.0;
        mod_ext(
            c.theta_ext_c,
            THETA_EI_SEUIL_BAS_C,
            THETA_EI_SEUIL_HAUT_C.min(plafond),
            limite.min(plafond),
        )
    } else {
        mod_ext(
            c.theta_ext_c,
            THETA_EI_SEUIL_BAS_C,
            THETA_EI_SEUIL_HAUT_C,
            limite,
        )
    }
}

/// Seuils de la gestion selon la température opérative.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeuilsOuverture {
    /// « θouv1 » : début de l'ouverture quand la température monte, en °C.
    pub ouv1_c: f64,
    /// « θouv2 » : ouverture complète quand la température monte, en °C.
    pub ouv2_c: f64,
    /// « θfer1 » : début de la fermeture quand la température descend, en °C.
    pub fer1_c: f64,
    /// « θfer2 » : fermeture complète quand la température descend, en °C.
    pub fer2_c: f64,
}

/// Th-BCE 2020 §5.13.3.1.1, éq. (242) et (243) : `θouv2 = θiifr_+ + Δθconf_adapt − 1 °C` hors
/// saison de chauffage, `+ 1 °C` en saison de chauffage ; `θouv1 = θfer1 = θouv2 − 2 °C` ;
/// `θfer2 = θouv2 − 3 °C`.
pub fn seuils_ouverture(c: &ConditionsOuverture) -> SeuilsOuverture {
    let decalage = if c.saison_de_chauffage { 1.0 } else { -1.0 };
    let ouv2_c = c.theta_iifr_c + c.delta_theta_conf_adapt_c + decalage;
    SeuilsOuverture {
        ouv1_c: ouv2_c - 2.0,
        ouv2_c,
        fer1_c: ouv2_c - 2.0,
        fer2_c: ouv2_c - 3.0,
    }
}

fn rampe(x: f64, debut: f64, fin: f64) -> f64 {
    ((x - debut) / (fin - debut)).clamp(0.0, 1.0)
}

/// Lecture de l'hystérésis de la figure 62, que le texte ne décrit que par un schéma.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LectureHysteresis {
    /// Lecture retenue (décision Q32) : la branche suivie dépend du sens de variation de la
    /// température opérative entre `h−2` et `h−1` ; à égalité, le ratio précédent est conservé.
    /// C'est la seule lecture qui emploie `θop,fin(h−2)`, entrée de la fiche.
    SensDeVariation,
    /// Variante : bande à mémoire,
    /// `Rouv = clamp(Rouv(h−1) ; branche d'ouverture(θop) ; branche de fermeture(θop))`. Le
    /// ratio précédent n'est modifié que s'il sort de l'intervalle délimité par les deux
    /// branches. **Première variante à essayer si un cas de référence diverge sur le DH.**
    BandeAMemoire,
}

/// Lecture de l'hystérésis employée par le moteur. Ce n'est pas une entrée du projet.
pub const LECTURE_HYSTERESIS: LectureHysteresis = LectureHysteresis::SensDeVariation;

/// « Rouv_θop_man(h) » : ratio d'ouverture selon la température opérative du pas précédent,
/// avec la lecture de l'hystérésis retenue ([`LECTURE_HYSTERESIS`]).
pub fn r_ouv_theta_op_man(precedent: f64, c: &ConditionsOuverture) -> f64 {
    r_ouv_theta_op_man_selon(LECTURE_HYSTERESIS, precedent, c)
}

/// « Rouv_θop_man(h) » pour une lecture donnée de l'hystérésis.
///
/// Th-BCE 2020 §5.13.3.1.1, figure 62 : profil à hystérésis. Sur la branche d'ouverture, le
/// ratio croît de 0 à `θouv1` à 1 à `θouv2` ; sur la branche de fermeture, il reste à 1 jusqu'à
/// `θfer1` puis décroît jusqu'à 0 à `θfer2`. §5.13.3.1.2, éq. (245) : en Th-B, nul en
/// inoccupation.
///
/// **Décisions Q32, à confirmer par un cas de référence**, isolées ici :
/// 1. passage d'une branche à l'autre : voir [`LectureHysteresis`]. Avec le calendrier du
///    fichier météo (Q25) et le changement de jour des scénarios (Q28), c'est l'un des
///    paramètres à tester en premier si le DH s'écarte d'un cas de référence ;
/// 2. le texte ne décrit le profil qu'« en occupation et en période journée (hlég > 7 h et
///    hlég < 23 h) », soit de 8 h à 22 h : en dehors de cette plage, le ratio précédent est
///    conservé. Des fenêtres ouvertes le soir le restent la nuit, modérées par `Cpr` ; en
///    inoccupation, `Cpr = 0` les ferme.
///
/// `precedent` est le ratio du pas de temps précédent.
pub fn r_ouv_theta_op_man_selon(
    lecture: LectureHysteresis,
    precedent: f64,
    c: &ConditionsOuverture,
) -> f64 {
    if c.mode == ModeCalcul::ThB && !c.i_occ_zone {
        return 0.0;
    }
    let journee = c.hleg > 7 && c.hleg < 23;
    if !(c.i_occ_zone && journee) {
        return precedent;
    }
    let s = seuils_ouverture(c);
    let ouverture = rampe(c.theta_op_h1_c, s.ouv1_c, s.ouv2_c);
    let fermeture = rampe(c.theta_op_h1_c, s.fer2_c, s.fer1_c);
    match lecture {
        LectureHysteresis::SensDeVariation => {
            if c.theta_op_h1_c > c.theta_op_h2_c {
                ouverture
            } else if c.theta_op_h1_c < c.theta_op_h2_c {
                fermeture
            } else {
                precedent
            }
        }
        // La branche d'ouverture est partout inférieure ou égale à la branche de fermeture.
        LectureHysteresis::BandeAMemoire => precedent.max(ouverture).min(fermeture),
    }
}

/// « Rouv(h) » : ratio d'ouverture d'une baie en gestion manuelle.
///
/// Th-BCE 2020 §5.13.3.1.1, éq. (240) :
/// `Rouv = iocc·Pocc·Cpr·Mod_ext_man·Rouv_θop_man`.
pub fn r_ouv_manuelle(
    c: &ConditionsOuverture,
    p_occ: f64,
    expo: ExpoBr,
    r_ouv_theta_op: f64,
) -> f64 {
    if !c.i_occ_zone {
        return 0.0;
    }
    p_occ
        * c_pr(expo, c.i_occ_zone, c.saison_de_chauffage, c.hleg)
        * mod_ext_man(c)
        * r_ouv_theta_op
}

/// « Aouv,b(h) » : surface d'ouverture équivalente d'une baie, en m².
///
/// Th-BCE 2020 §5.13.3.2.3 :
/// - éq. (248) : `Rouv_corrige = min(1 ; Rouv × Aouvmax / Aouvmax_ac_PM)`, et
///   `Rouv_corrige = Rouv` si `Aouvmax_ac_PM` est nul ;
/// - éq. (249) : `Aouv,b = Rouv_corrige × Aouvmax_ac_PM`.
pub fn a_ouv_baie_m2(r_ouv: f64, a_ouv_max_m2: f64, a_ouv_max_avec_pm_m2: f64) -> f64 {
    let r_ouv_corrige = if a_ouv_max_avec_pm_m2 > 0.0 {
        (r_ouv * a_ouv_max_m2 / a_ouv_max_avec_pm_m2).min(1.0)
    } else {
        r_ouv
    };
    r_ouv_corrige * a_ouv_max_avec_pm_m2
}

/// Ouverture d'une baie au pas de temps, pour le calcul du débit du groupe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OuvertureBaie {
    /// « Aouv,b(h) », en m².
    pub a_ouv_m2: f64,
    /// « αb » : orientation, en degrés (0 sud, 90 ouest, 180 nord, 270 est).
    pub alpha_deg: f64,
    /// « βb » : inclinaison, en degrés.
    pub beta_deg: f64,
}

/// « Sors(h) » : surface équivalente du phénomène traversant, en m².
///
/// Th-BCE 2020 §5.13.3.2.4, éq. (252) et (253). Pour chacune des deux distributions cardinales
/// (`k = 1` : S-O-N-E ; `k = 2` : décalée de 45°), les baies de façade sont regroupées en quatre
/// secteurs de 90° centrés sur `αref = (k−1)·45° + (n−1)·90°` (`αbas ≤ α < αhaut`), et
/// `Sors,k = ¼·Σ 1/√(1/Aouvn² + 1/(Aouv − Aouvn)²)` sur les secteurs non vides ;
/// `Sors = min(Sors,1 ; Sors,2)`.
///
/// Lectures retenues (question Q32) : seules les baies d'inclinaison `β ≥ 60°` sont rattachées à
/// un secteur, comme le dit le texte, alors que l'éq. (253) écrit « β < 60° » ; les baies de
/// toiture comptent dans `Aouv`. Un secteur qui réunit toute la surface d'ouverture a un terme
/// nul (limite de la formule quand `Aouv − Aouvn` tend vers zéro).
pub fn s_ors_m2(baies: &[OuvertureBaie]) -> f64 {
    let a_ouv: f64 = baies.iter().map(|b| b.a_ouv_m2).sum();
    let mut s_ors = f64::INFINITY;
    for k in 0..2 {
        let mut secteurs = [0.0_f64; 4];
        for b in baies.iter().filter(|b| b.beta_deg >= BETA_FACADE_DEG) {
            // Écart à αbas du premier secteur, ramené dans [0 ; 360[.
            let ecart = (b.alpha_deg - (f64::from(k) * 45.0 - 45.0)).rem_euclid(360.0);
            // Troncature voulue : indice de secteur de 0 à 3.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let n = ((ecart / 90.0) as usize).min(3);
            secteurs[n] += b.a_ouv_m2;
        }
        let somme: f64 = secteurs
            .iter()
            .filter(|&&a| a > 0.0)
            .map(|&a| {
                let reste = a_ouv - a;
                if reste > 0.0 {
                    1.0 / sqrt(1.0 / (a * a) + 1.0 / (reste * reste))
                } else {
                    0.0
                }
            })
            .sum();
        s_ors = s_ors.min(0.25 * somme);
    }
    if s_ors.is_finite() {
        s_ors
    } else {
        0.0
    }
}

/// « Qv,surv,ouv(h) » : débit volumique de surventilation du groupe, en m³/h.
///
/// Th-BCE 2020 §5.13.3.2.4 :
/// - non traversant, éq. (251) :
///   `3600·Aouv/2·[max(Cw·Vvent_c² ; Cst·Httf·|θext − θi,fin(h−1)|)]^0,5` ;
/// - traversant, éq. (254) : `3600·max(Cd_go·Sors·min(Vvent_c ; Vventmax)·dCp^0,5 ;
///   Aouv/2·(Cst·Httf·|θext − θi,fin(h−1)|)^0,5)`.
///
/// `s_ors_m2` vaut `None` pour un groupe non traversant.
pub fn qv_surv_m3_h(
    a_ouv_m2: f64,
    s_ors_m2: Option<f64>,
    v_vent_c_m_s: f64,
    h_ttf_m: f64,
    theta_ext_c: f64,
    theta_i_fin_h1_c: f64,
) -> f64 {
    let tirage = C_ST * h_ttf_m * fabs(theta_ext_c - theta_i_fin_h1_c);
    match s_ors_m2 {
        None => 3600.0 * a_ouv_m2 / 2.0 * sqrt((C_W * v_vent_c_m_s * v_vent_c_m_s).max(tirage)),
        Some(s_ors) => {
            let vent = CD_GO * s_ors * v_vent_c_m_s.min(V_VENT_MAX_M_S) * sqrt(D_CP);
            3600.0 * vent.max(a_ouv_m2 / 2.0 * sqrt(tirage))
        }
    }
}

/// « ρext(h) » : masse volumique de l'air entrant, en kg/m³.
///
/// Th-BCE 2020 §5.13.3.3, éq. (255) : `ρext = ρref·(273 + θref)/(273 + θext)`.
pub fn rho_ext_kg_m3(theta_ext_c: f64) -> f64 {
    RHO_REF_KG_M3 * (273.0 + THETA_REF_C) / (273.0 + theta_ext_c)
}

/// « Qmaj(h) » : débit massique d'air entrant dans le groupe par les baies, en kg/s.
///
/// Th-BCE 2020 §5.13.3.3, éq. (256) :
/// `Qmaj = max(Qv,surv,ouv/3600 × ρext ; Σ qm,aération)`.
pub fn qm_aj_kg_s(qv_surv_m3_h: f64, theta_ext_c: f64, qm_aeration_kg_s: f64) -> f64 {
    (qv_surv_m3_h / 3600.0 * rho_ext_kg_m3(theta_ext_c)).max(qm_aeration_kg_s)
}
