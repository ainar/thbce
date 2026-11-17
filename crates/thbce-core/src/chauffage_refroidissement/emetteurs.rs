// SPDX-License-Identifier: GPL-3.0-or-later
//! Émetteurs du projet : fiche `C-Emi-Systèmes Emissions du groupe en chaud et en froid`
//! (Th-BCE 2020 §8.1.3.1, §8.1.3.2, §8.1.3.7 et §8.1.3.8), au-delà de l'émission équivalente
//! conventionnelle du calcul du Bbio ([`super::emission`]).
//!
//! - caractéristiques des émetteurs : variations spatiales (tableaux 82 à 84), parts
//!   convectives (tableaux 85 et 86), pertes au dos (éq. 795), variations temporelles
//!   (tableaux 87 et 88), mesure de la sonde (`Psd = 0,5`), contributions `Rat` (éq. 796) ;
//! - émetteurs équivalents de chaud et de froid (éq. 797 à 799) ;
//! - énergies requises par émetteur, pertes au dos comprises (éq. 810) ;
//! - ventilateurs locaux (éq. 811 à 816), en calcul des consommations seulement.
//!
//! Lectures : `docs/questions/Q043.md`.

use crate::entree::{
    ClasseEmetteurChaud, ClasseEmetteurFroid, Emetteur, GestionVentilateurs, RegulationBatterie,
    TypeEmetteurChaud, TypeEmetteurFroid, VariationSpatiale, VariationSpatialeFroid,
    VariationTemporelle, VentilateursLocaux,
};

use super::emission::EmetteurEquivalent;

/// « Psd » : ratio de la mesure de la sonde du régulateur, conventionnel (§8.1.3.1.7 : « La
/// valeur conventionnellement retenue est de 0.5, quelques soient les configurations de
/// régulateurs »).
pub const P_SD: f64 = 0.5;

/// « δθpresence_ch » : variation de la consigne de chauffage due à la détection de présence
/// pièce par pièce, en K (tableau 81, valeur conventionnelle).
pub const DELTA_THETA_PRESENCE_CH_K: f64 = -0.15;

/// « SeuilVCV_pvmv_ch » : seuil de besoins de chauffage du passage en moyenne vitesse, en
/// Wh/m² (tableau 81, valeur conventionnelle).
pub const SEUIL_VCV_PVMV_CH_WH_M2: f64 = 20.0;

/// « SeuilVCV_pvmv_fr » : seuil de besoins de froid du passage en moyenne vitesse, en Wh/m²
/// (tableau 81, valeur conventionnelle).
pub const SEUIL_VCV_PVMV_FR_WH_M2: f64 = -20.0;

/// Variation spatiale en chauffage d'une classe d'émetteur, en K (tableau 82).
///
/// Colonnes : moins de 4 m sous plafond ; de 4 à moins de 6 m ; de 6 à moins de 8 m ; « plus
/// de 8 m », lu comme 8 m et plus.
pub fn delta_theta_vs_ch_classe_k(classe: ClasseEmetteurChaud, hsp_m: f64) -> f64 {
    let ligne: [f64; 4] = match classe {
        ClasseEmetteurChaud::A => [0.0, 0.0, 0.0, 0.0],
        ClasseEmetteurChaud::B1 => [0.0, 0.0, 0.5, 1.0],
        ClasseEmetteurChaud::B2 => [0.0, 0.6, 1.7, 2.8],
        ClasseEmetteurChaud::B3 => [0.2, 0.8, 1.2, 1.6],
        ClasseEmetteurChaud::C => [0.4, 1.2, 2.0, 2.8],
    };
    ligne[colonne_hsp(hsp_m)]
}

/// Variation spatiale en refroidissement d'une classe d'émetteur, en K (tableau 84).
pub fn delta_theta_vs_fr_classe_k(classe: ClasseEmetteurFroid, hsp_m: f64) -> f64 {
    let ligne: [f64; 4] = match classe {
        ClasseEmetteurFroid::A => [0.0, 0.4, 0.8, 1.2],
        ClasseEmetteurFroid::B => [0.0, 0.2, 0.4, 0.6],
        ClasseEmetteurFroid::C => [0.0, 0.0, 0.0, 0.0],
    };
    ligne[colonne_hsp(hsp_m)]
}

fn colonne_hsp(hsp_m: f64) -> usize {
    if hsp_m < 4.0 {
        0
    } else if hsp_m < 6.0 {
        1
    } else if hsp_m < 8.0 {
        2
    } else {
        3
    }
}

/// Variation spatiale des poêles et inserts, en K (tableau 83) : 0,9 pour un appareil
/// desservant un seul niveau, 1,4 pour deux niveaux.
pub fn delta_theta_vs_poele_k(niveaux: u8) -> f64 {
    if niveaux >= 2 {
        1.4
    } else {
        0.9
    }
}

/// « Pemconv_ch » (tableau 85).
pub fn p_em_conv_ch(t: TypeEmetteurChaud) -> f64 {
    match t {
        TypeEmetteurChaud::SoufflageAirChaud => 0.95,
        TypeEmetteurChaud::MurauxRayonnants => 0.70,
        TypeEmetteurChaud::PlancherChauffant => 0.50,
        TypeEmetteurChaud::MurChauffant => 0.35,
        TypeEmetteurChaud::PlafondChauffant => 0.20,
    }
}

/// « Pemconv_fr » (tableau 86).
pub fn p_em_conv_fr(t: TypeEmetteurFroid) -> f64 {
    match t {
        TypeEmetteurFroid::SoufflageAirFroid => 0.95,
        TypeEmetteurFroid::PoutreFroide => 0.80,
        TypeEmetteurFroid::PlafondRafraichissant => 0.50,
        TypeEmetteurFroid::MurRafraichissant => 0.35,
        TypeEmetteurFroid::PlancherRafraichissant => 0.20,
    }
}

/// « δθvt » d'un émetteur, en K (§8.1.3.1.5).
///
/// Valeur certifiée telle quelle ; valeur justifiée augmentée de 0,5 K en chaud (diminuée de
/// 0,5 K en froid) ; à défaut, tableau 87 (2,0 ou 1,8 K en chaud, −2,0 ou −1,8 K en froid,
/// selon que le couple régulateur/émetteur permet ou non un arrêt total) ; poêles et inserts,
/// tableau 88 (2 K avec thermostat d'ambiance, 2,5 K en régulation manuelle).
pub fn delta_theta_vt_k(v: &VariationTemporelle, chaud: bool) -> f64 {
    let signe = if chaud { 1.0 } else { -1.0 };
    match *v {
        VariationTemporelle::Certifie { delta_theta_k } => delta_theta_k,
        VariationTemporelle::Justifie { delta_theta_k } => delta_theta_k + signe * 0.5,
        VariationTemporelle::Defaut { arret_total } => signe * if arret_total { 1.8 } else { 2.0 },
        VariationTemporelle::PoeleOuInsert { thermostat } => {
            if thermostat {
                2.0
            } else {
                2.5
            }
        }
    }
}

/// Caractéristiques d'un émetteur pour une fonction (chaud ou froid).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FonctionEmetteur {
    /// « Pemconv » : part convective.
    pub p_em_conv: f64,
    /// « δθvs », en K.
    pub delta_theta_vs_k: f64,
    /// « δθvt », en K.
    pub delta_theta_vt_k: f64,
    /// « Rat » = `Rats × Ratt` (éq. 796).
    pub rat: f64,
    /// Indice du réseau de distribution du groupe qui alimente l'émetteur pour cette fonction.
    pub distribution: usize,
}

/// Ventilateurs locaux d'un émetteur, prêts pour le calcul (§8.1.3.8).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VentilateursResolus {
    /// « GestVCV ».
    pub gestion: GestionVentilateurs,
    /// « ispv ».
    pub spv: bool,
    /// Puissances en grande, moyenne, petite et super petite vitesse, en W.
    pub p_w: [f64; 4],
    /// Débits de recirculation en grande, moyenne et petite vitesse, en m³/h.
    pub q_recirc_m3_h: [f64; 3],
    /// « idregul_batt ».
    pub regulation_batterie: Option<RegulationBatterie>,
}

/// Émetteur du projet, prêt pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct EmetteurResolu {
    /// Identifiant.
    pub id: String,
    /// Fonction de chauffage ; `None` si `idem_chaud = 0`.
    pub chaud: Option<FonctionEmetteur>,
    /// Fonction de refroidissement ; `None` si `idem_froid = 0`.
    pub froid: Option<FonctionEmetteur>,
    /// « Pper » : ratio des pertes au dos.
    pub p_per: f64,
    /// « Iddetection_presence ».
    pub detection_presence: bool,
    /// Ventilateurs locaux ; `None` si `GestVCV = 0`.
    pub ventilateurs: Option<VentilateursResolus>,
}

impl EmetteurResolu {
    /// Résout un émetteur saisi ; `distribution` donne l'indice d'un réseau du groupe d'après
    /// son identifiant (références vérifiées par la validation).
    pub fn nouveau(e: &Emetteur, distribution: impl Fn(&str) -> Option<usize>) -> Self {
        let chaud = e.chauffage.as_ref().map(|c| FonctionEmetteur {
            p_em_conv: p_em_conv_ch(c.type_emetteur),
            delta_theta_vs_k: match c.variation_spatiale {
                VariationSpatiale::Classe { classe, hsp_m } => {
                    delta_theta_vs_ch_classe_k(classe, hsp_m)
                }
                VariationSpatiale::PoeleOuInsert { niveaux } => delta_theta_vs_poele_k(niveaux),
                VariationSpatiale::Valeur { delta_theta_k } => delta_theta_k,
            },
            delta_theta_vt_k: delta_theta_vt_k(&c.variation_temporelle, true),
            rat: c.rats * c.ratt,
            distribution: distribution(&c.distribution).unwrap_or(0),
        });
        let froid = e.refroidissement.as_ref().map(|f| FonctionEmetteur {
            p_em_conv: p_em_conv_fr(f.type_emetteur),
            delta_theta_vs_k: match f.variation_spatiale {
                VariationSpatialeFroid::Classe { classe, hsp_m } => {
                    delta_theta_vs_fr_classe_k(classe, hsp_m)
                }
                VariationSpatialeFroid::Valeur { delta_theta_k } => delta_theta_k,
            },
            delta_theta_vt_k: delta_theta_vt_k(&f.variation_temporelle, false),
            rat: f.rats * f.ratt,
            distribution: distribution(&f.distribution).unwrap_or(0),
        });
        Self {
            id: e.id.clone(),
            chaud,
            froid,
            // Éq. (795) : Pper = Ri·Ue.
            p_per: e
                .pertes_au_dos
                .as_ref()
                .map_or(0.0, |p| p.r_i_m2k_w * p.u_e_w_m2k),
            detection_presence: e.detection_presence.unwrap_or(false),
            ventilateurs: e.ventilateurs.as_ref().map(ventilateurs),
        }
    }
}

fn ventilateurs(v: &VentilateursLocaux) -> VentilateursResolus {
    VentilateursResolus {
        gestion: v.gestion,
        spv: v.super_petite_vitesse,
        p_w: [v.p_gv_w, v.p_mv_w, v.p_pv_w, v.p_spv_w.unwrap_or(0.0)],
        q_recirc_m3_h: [v.q_recirc_gv_m3_h, v.q_recirc_mv_m3_h, v.q_recirc_pv_m3_h],
        regulation_batterie: v.regulation_batterie,
    }
}

/// Émission équivalente d'un groupe : émetteurs équivalents de chaud et de froid et
/// contributions effectives de chaque émetteur (§8.1.3.2).
#[derive(Debug, Clone, PartialEq)]
pub struct EmissionEquivalente {
    /// Émetteur équivalent de chaud.
    pub chaud: EmetteurEquivalent,
    /// Émetteur équivalent de froid.
    pub froid: EmetteurEquivalent,
    /// « Rateff_ch^em » de chaque émetteur (éq. 797).
    pub rat_eff_ch: Vec<f64>,
    /// « Rateff_fr^em » de chaque émetteur (éq. 797).
    pub rat_eff_fr: Vec<f64>,
    /// « Ratem_eq_ch » = `Σ Ratch^em` (éq. 798).
    pub rat_em_eq_ch: f64,
    /// « Ratem_eq_fr » = `Σ Ratfr^em` (éq. 798).
    pub rat_em_eq_fr: f64,
}

impl EmissionEquivalente {
    /// Th-BCE 2020 §8.1.3.2, éq. (797) à (799).
    ///
    /// `Rateff^em = Rat^em/Σ Rat^em` (nul si la somme est nulle) ; les caractéristiques des
    /// émetteurs équivalents sont les moyennes des caractéristiques des émetteurs pondérées par
    /// leurs `Rateff` ; `δθpresence_eq_ch = Σ Rateff_ch × δθpresence_ch × Iddetection_presence`.
    /// Un émetteur équivalent sans émetteur (aucun émetteur de froid, par exemple) reçoit les
    /// caractéristiques conventionnelles du Bbio, sans effet puisqu'il n'est jamais sollicité
    /// avec une contribution non nulle.
    pub fn nouvelle(emetteurs: &[EmetteurResolu]) -> Self {
        let rats_ch: Vec<f64> = emetteurs
            .iter()
            .map(|e| e.chaud.map_or(0.0, |c| c.rat))
            .collect();
        let rats_fr: Vec<f64> = emetteurs
            .iter()
            .map(|e| e.froid.map_or(0.0, |c| c.rat))
            .collect();
        let normer = |rats: &[f64]| -> (f64, Vec<f64>) {
            let somme: f64 = rats.iter().sum();
            let eff = rats
                .iter()
                .map(|r| if somme > 0.0 { r / somme } else { 0.0 })
                .collect();
            (somme, eff)
        };
        let (rat_em_eq_ch, rat_eff_ch) = normer(&rats_ch);
        let (rat_em_eq_fr, rat_eff_fr) = normer(&rats_fr);
        let equivalent = |eff: &[f64], f: &dyn Fn(&EmetteurResolu) -> Option<FonctionEmetteur>| {
            if eff.iter().all(|r| *r == 0.0) {
                return EmetteurEquivalent::BBIO;
            }
            let mut eq = EmetteurEquivalent {
                p_sd: 0.0,
                p_em_conv: 0.0,
                delta_theta_vs_k: 0.0,
                delta_theta_vt_k: 0.0,
                delta_theta_presence_k: 0.0,
            };
            for (e, r) in emetteurs.iter().zip(eff) {
                if let Some(c) = f(e) {
                    eq.p_sd += r * P_SD;
                    eq.p_em_conv += r * c.p_em_conv;
                    eq.delta_theta_vs_k += r * c.delta_theta_vs_k;
                    eq.delta_theta_vt_k += r * c.delta_theta_vt_k;
                }
            }
            eq
        };
        let mut chaud = equivalent(&rat_eff_ch, &|e| e.chaud);
        // Éq. (799).
        chaud.delta_theta_presence_k = emetteurs
            .iter()
            .zip(&rat_eff_ch)
            .filter(|(e, _)| e.detection_presence)
            .map(|(_, r)| r * DELTA_THETA_PRESENCE_CH_K)
            .sum();
        let froid = equivalent(&rat_eff_fr, &|e| e.froid);
        Self {
            chaud,
            froid,
            rat_eff_ch,
            rat_eff_fr,
            rat_em_eq_ch,
            rat_em_eq_fr,
        }
    }
}

/// Énergies requises par émetteur au pas de temps, en Wh (éq. 810) : `(Qsys_ch^em,
/// Qsys_fr^em)`, avec `Qsys_ch^em = idem_chaud × Rateff_ch × MAX(0 ; φutil)/(1 − Pper)` et
/// `Qsys_fr^em = idem_froid × Rateff_fr × MIN(0 ; φutil)/(1 − Pper)`.
pub fn besoins_emetteur_wh(
    e: &EmetteurResolu,
    rat_eff_ch: f64,
    rat_eff_fr: f64,
    phi_util_w: f64,
) -> (f64, f64) {
    let pertes = 1.0 - e.p_per;
    let ch = if e.chaud.is_some() {
        rat_eff_ch * phi_util_w.max(0.0) / pertes
    } else {
        0.0
    };
    let fr = if e.froid.is_some() {
        rat_eff_fr * phi_util_w.min(0.0) / pertes
    } else {
        0.0
    };
    (ch, fr)
}

/// Régime de fonctionnement des ventilateurs locaux d'un émetteur.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RegimeVentilateurs {
    /// « Qv_recirc^em(h) » : débit de recirculation, en m³/h.
    pub q_v_recirc_m3_h: f64,
    /// « Wvent_loc^em(h) » : consommation électrique, en Wh ; transmise en totalité à
    /// l'ambiance sous forme de chaleur (éq. 813).
    pub w_vent_loc_wh: f64,
}

/// Conditions d'un pas de temps pour les ventilateurs locaux.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionsVentilateurs {
    /// « Autch,eff(j) » et « Autfr,eff(j) ».
    pub aut_ch: bool,
    /// Voir `aut_ch`.
    pub aut_fr: bool,
    /// « irelance(h) ».
    pub relance: bool,
    /// « iocc_zone(h) ».
    pub i_occ: bool,
    /// « iocc_zone(h−1) ».
    pub i_occ_h1: bool,
    /// « idbch(h) » et « idbfr(h) » (éq. 809).
    pub idb_ch: bool,
    /// Voir `idb_ch`.
    pub idb_fr: bool,
    /// « Agr », en m².
    pub a_gr_m2: f64,
}

/// Régime des ventilateurs locaux d'un émetteur au pas de temps (éq. 811 et 812).
///
/// `qsys` = `(Qsys_ch^em, Qsys_fr^em)` du pas de temps (éq. 810) ; `precedent` : régime du pas
/// précédent, reconduit en gestion manuelle hors du premier pas d'occupation. Hors des
/// saisons de fonctionnement de l'émetteur, les ventilateurs sont arrêtés (éq. 811).
pub fn regime_ventilateurs(
    e: &EmetteurResolu,
    v: &VentilateursResolus,
    rat_eff: (f64, f64),
    qsys: (f64, f64),
    c: &ConditionsVentilateurs,
    precedent: RegimeVentilateurs,
) -> RegimeVentilateurs {
    // Éq. (811).
    let actif = (e.chaud.is_some() && c.aut_ch) || (e.froid.is_some() && c.aut_fr);
    if !actif {
        return RegimeVentilateurs::default();
    }
    let regime = |i: usize| RegimeVentilateurs {
        q_v_recirc_m3_h: v.q_recirc_m3_h.get(i).copied().unwrap_or(0.0),
        w_vent_loc_wh: v.p_w[i],
    };
    let (gv, mv, pv) = (regime(0), regime(1), regime(2));
    let mv_ou_pv = || {
        let seuil_ch = rat_eff.0 * c.a_gr_m2 * SEUIL_VCV_PVMV_CH_WH_M2;
        let seuil_fr = rat_eff.1 * c.a_gr_m2 * SEUIL_VCV_PVMV_FR_WH_M2;
        if qsys.0 > seuil_ch || qsys.1 < seuil_fr {
            mv
        } else {
            pv
        }
    };
    // Éq. (812).
    if c.relance {
        return gv;
    }
    match v.gestion {
        GestionVentilateurs::Manuelle => {
            if c.i_occ && !c.i_occ_h1 {
                mv_ou_pv()
            } else if precedent == RegimeVentilateurs::default() {
                // Régime initial, avant le premier pas d'occupation de la saison : fonctionnement
                // permanent en petite vitesse (lecture Q43).
                pv
            } else {
                precedent
            }
        }
        GestionVentilateurs::AutomatiquePermanente => {
            if v.spv && !c.idb_ch && !c.idb_fr {
                RegimeVentilateurs {
                    q_v_recirc_m3_h: 0.0,
                    w_vent_loc_wh: v.p_w[3],
                }
            } else {
                mv_ou_pv()
            }
        }
        GestionVentilateurs::AutomatiqueAvecArret => {
            if !c.idb_ch && !c.idb_fr {
                RegimeVentilateurs::default()
            } else {
                mv_ou_pv()
            }
        }
    }
}
