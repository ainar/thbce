// SPDX-License-Identifier: GPL-3.0-or-later
//! Ventilation (Th-BCE 2020 §10.3, `C_VEN_Calcul des consommations de ventilation des parcs de
//! stationnement fermés`, p. 1318 à 1326) et éclairage (§10.4, `C_ECL_calcul des consommations
//! d'éclairage des parkings`, p. 1327 à 1332) des parkings.
//!
//! Les pas de temps sont les itérations `h` de 0 à 8759 ; « Hleg » du texte est l'heure légale
//! de fin du pas de temps (tableaux 321 et 323), c'est-à-dire la case de scénario de 1 à 24.
//! Lectures retenues : `docs/questions.md`, Q50.

use crate::climat::calendrier::{case_scenario, jour_legal};
use crate::entree::{Parking, PlageHoraire, TypeParking};
use crate::resultats::ValeurParDefaut;

/// Nombre de pas de temps de l'année.
const NB_H: usize = 8760;

/// « Typeusage » du parking (§10.3.2) : 0 bureau, 1 commerce, 2 habitat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeUsageParking {
    /// 0 : bureau.
    Bureau,
    /// 1 : commerce.
    Commerce,
    /// 2 : habitat.
    Habitat,
}

/// Jour de la semaine du pas de temps, de 1 (lundi) à 7 : l'année commence un lundi (§4.1.3).
fn jsem(h: usize) -> u8 {
    (jour_legal(h).min(364) % 7) as u8 + 1
}

/// Heure légale de fin du pas de temps, de 1 à 24.
fn hleg(h: usize) -> u8 {
    case_scenario(h)
}

/// Jour de l'année du pas de temps, de 1 à 365 (premier jour légal = 1).
fn jour_annee(h: usize) -> usize {
    jour_legal(h).min(364) + 1
}

/// Tableau 321 : « Rmvtpl » selon l'heure légale de fin de pas de temps (nul avant 8 h).
pub fn rmvtpl(usage: TypeUsageParking, hleg: u8) -> f64 {
    let h = hleg;
    match usage {
        TypeUsageParking::Bureau => match h {
            9..=11 => 0.308,
            12 | 14 | 15 => 0.154,
            17 | 18 => 0.308,
            _ => 0.0,
        },
        TypeUsageParking::Habitat => match h {
            8..=10 | 17..=19 => 0.19,
            11..=16 | 20..=22 => 0.095,
            _ => 0.0,
        },
        TypeUsageParking::Commerce => match h {
            8 | 21 | 22 => 0.278,
            9..=20 => 0.556,
            _ => 0.0,
        },
    }
}

/// Tableau 323 : « Fhext » selon l'heure légale de fin de pas de temps.
fn fh_ext(hleg: u8) -> f64 {
    match hleg {
        1..=6 | 23 | 24 => 1.0,
        7 => 0.79,
        8 => 0.48,
        9 => 0.15,
        10..=16 => 0.0,
        17 => 0.03,
        18 => 0.2,
        19 => 0.36,
        20 => 0.5,
        21 => 0.65,
        22 => 0.84,
        _ => 0.0,
    }
}

/// Plages d'ouverture par défaut (§10.4.3.2), en heures légales de début et de fin : habitat
/// 24 h/24 ; bureau 9 h à 19 h en semaine, jamais en week-end ; commerce 7 h à 22 h.
fn plages_ouverture(usage: TypeUsageParking) -> (PlageHoraire, PlageHoraire) {
    let p = |debut_h, fin_h| PlageHoraire { debut_h, fin_h };
    match usage {
        TypeUsageParking::Habitat => (p(0, 24), p(0, 24)),
        TypeUsageParking::Bureau => (p(9, 19), p(0, 0)),
        TypeUsageParking::Commerce => (p(7, 22), p(7, 22)),
    }
}

/// Vrai si le pas de temps commence dans la plage `[debut ; fin[`.
fn dans_plage(h: usize, p: PlageHoraire) -> bool {
    let debut_pas = hleg(h) - 1;
    p.debut_h <= debut_pas && debut_pas < p.fin_h
}

/// « NbjO(j) » : vecteur des jours ouverts, 365 valeurs (§10.4.3.2 : ouvert tous les jours).
fn jours_ouverts(p: &Parking) -> [bool; 365] {
    let mut v = [true; 365];
    for j in &p.jours_fermes {
        if let Some(x) = v.get_mut(usize::from(*j).wrapping_sub(1)) {
            *x = false;
        }
    }
    v
}

/// Éq. (2351) et suivantes, « Ouv(h) » : le parking est ouvert (§10.4.3.4).
fn ouv(h: usize, usage: TypeUsageParking, nbjo: &[bool; 365]) -> bool {
    if !nbjo[jour_annee(h) - 1] {
        return false;
    }
    let (semaine, we) = plages_ouverture(usage);
    let js = jsem(h);
    if js <= 5 {
        dans_plage(h, semaine)
    } else if usage == TypeUsageParking::Commerce && js == 7 {
        false
    } else {
        dans_plage(h, we)
    }
}

/// Éq. (2351), « Det(h) » : l'éclairage est en mode détection de présence (§10.4.3.5).
fn det(h: usize, p: &Parking, nbjo: &[bool; 365]) -> bool {
    if !nbjo[jour_annee(h) - 1] {
        return false;
    }
    let plages = if jsem(h) <= 5 {
        &p.eclairage.plages_detection_semaine
    } else {
        &p.eclairage.plages_detection_week_end
    };
    plages.iter().any(|pl| dans_plage(h, *pl))
}

/// Éq. (2349) et (2350) : puissance d'éclairage installée, en W.
fn p_ecl_installee(p: &Parking, defauts: &mut Vec<ValeurParDefaut>) -> f64 {
    p.eclairage.p_installee_w.unwrap_or_else(|| {
        let (par_place, eq) = match p.type_parking {
            TypeParking::Interieur => (75.0, "(2349)"),
            TypeParking::Exterieur => (8.0, "(2350)"),
        };
        defauts.push(ValeurParDefaut {
            champ: "eclairage.p_installee_w".to_owned(),
            valeur: p.npl * par_place,
            regle: format!("Th-BCE 2020 §10.4.3.2, éq. {eq} : {par_place} W par place de parking"),
        });
        p.npl * par_place
    })
}

/// Éclairage d'un parking : « Pecapp(h) » en W, soit en Wh par pas d'une heure (éq. 2353).
/// Renvoie aussi les valeurs par défaut appliquées.
pub fn eclairage(p: &Parking, usage: TypeUsageParking) -> (Vec<f64>, Vec<ValeurParDefaut>) {
    let mut defauts = Vec::new();
    let pecins = p_ecl_installee(p, &mut defauts);
    let nbjo = jours_ouverts(p);
    // « Ex » : 0 si extinction à la fermeture, 1 sinon (marche forcée).
    let ex = if p.eclairage.extinction_en_fermeture.unwrap_or(true) {
        0.0
    } else {
        1.0
    };
    const TAU_DET: f64 = 0.2; // éq. (2348)
    let serie = (0..NB_H)
        .map(|h| {
            // Éq. (2347) et (2352) : besoin d'éclairage.
            let fh = match p.type_parking {
                TypeParking::Interieur => 1.0,
                TypeParking::Exterieur => fh_ext(hleg(h)),
            };
            let o = if ouv(h, usage, &nbjo) { 1.0 } else { 0.0 };
            let d = if det(h, p, &nbjo) { 1.0 } else { 0.0 };
            pecins * (o * fh * (1.0 - d + d * TAU_DET) + (1.0 - o) * ex)
        })
        .collect();
    (serie, defauts)
}

/// Constantes du tableau 320.
const EFF_VENT: f64 = 0.5;
const CCO_LIM: f64 = 5e-5;
const PROD_CO_VEH: f64 = 0.35;
const R_UTIL: f64 = 1.0;
const D_FIX_H: f64 = 0.01;
const V_MOY_M_H: f64 = 10_000.0;
const RL: f64 = 2.0;
const S_PL_M2: f64 = 12.0;

/// Éq. (2327) à (2332) : durée moyenne de fonctionnement du moteur d'un véhicule, en h.
pub fn d_mvmt_h(npl: f64, net: f64) -> f64 {
    let larg = (S_PL_M2 * npl / net / RL).sqrt(); // (2327)
    let long = RL * larg; // (2328)
    let peri = 2.0 * long + 2.0 * larg; // (2329)
    let l_moy_traj = peri * (net / 4.0 + 0.25); // (2330)
    D_FIX_H + l_moy_traj / V_MOY_M_H // (2331) et (2332)
}

/// Puissances et débits des deux vitesses des ventilateurs d'un parking hors habitation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VitessesVentilation {
    /// « Dvent1 », m³/h.
    pub d_vent1: f64,
    /// « Dvent2 », m³/h.
    pub d_vent2: f64,
    /// « Pvent1 », W.
    pub p_vent1: f64,
    /// « Pvent2 », W.
    pub p_vent2: f64,
}

impl VitessesVentilation {
    /// Valeurs par défaut de l'éq. (2326).
    pub fn defaut(npl: f64) -> Self {
        Self {
            d_vent1: 450.0 * npl,
            d_vent2: 900.0 * npl,
            p_vent1: 5.0 * npl,
            p_vent2: 40.0 * npl,
        }
    }
}

/// Ventilation d'un parking hors habitation, fermé et à ventilation forcée : « Pvent(h) » en W
/// (éq. 2335 à 2342). La fonction n'est atteignable par l'entrée qu'avec des zones d'usage
/// hors habitation, que la version courante ne calcule pas ; elle est testée isolément.
pub fn ventilation_hors_habitation(
    p: &Parking,
    usage: TypeUsageParking,
    v: VitessesVentilation,
) -> Vec<f64> {
    let nbjo = jours_ouverts(p);
    let dmvmt = d_mvmt_h(p.npl, f64::from(p.net));
    (0..NB_H)
        .map(|h| {
            let dreq = if nbjo[jour_annee(h) - 1] {
                let nveh = p.npl * R_UTIL * rmvtpl(usage, hleg(h)); // (2335)
                let prod_co = PROD_CO_VEH * nveh * dmvmt; // (2336)
                prod_co / (CCO_LIM * EFF_VENT) // (2337)
            } else {
                0.0 // (2338)
            };
            if dreq == 0.0 {
                0.0
            } else if v.d_vent1 > dreq {
                dreq * v.p_vent1 / v.d_vent1
            } else {
                dreq * v.p_vent2 / v.d_vent2
            }
        })
        .collect()
}

/// Ventilation d'un parking d'habitat fermé : « Pvent(h) » en W (éq. 2333, 2334, 2343 à 2345).
/// Renvoie aussi les valeurs par défaut appliquées.
pub fn ventilation_habitat(p: &Parking) -> (Vec<f64>, Vec<ValeurParDefaut>) {
    let mut defauts = Vec::new();
    let Some(v) = &p.ventilation else {
        return (vec![0.0; NB_H], defauts); // (2334)
    };
    if p.type_parking == TypeParking::Exterieur {
        return (vec![0.0; NB_H], defauts); // (2333)
    }
    let pvent600 = v.p_vent600_w_par_place.unwrap_or_else(|| {
        defauts.push(ValeurParDefaut {
            champ: "ventilation.p_vent600_w_par_place".to_owned(),
            valeur: 40.0,
            regle: "Th-BCE 2020 §10.3.3.1, éq. (2326) : Pvent600 = 40 W/place".to_owned(),
        });
        40.0
    });
    let nbjo = jours_ouverts(p);
    let serie = (0..NB_H)
        .map(|h| {
            if !v.regulee {
                pvent600 * p.npl * R_UTIL // (2345)
            } else if nbjo[jour_annee(h) - 1] {
                pvent600 * p.npl * R_UTIL * rmvtpl(TypeUsageParking::Habitat, hleg(h))
            // (2343)
            } else {
                0.0 // (2344)
            }
        })
        .collect();
    (serie, defauts)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::entree::{EclairageParking, VentilationParking};

    fn parking(type_parking: TypeParking) -> Parking {
        Parking {
            id: "p".into(),
            type_parking,
            npl: 20.0,
            net: 1,
            ventilation: None,
            eclairage: EclairageParking {
                p_installee_w: None,
                plages_detection_semaine: vec![],
                plages_detection_week_end: vec![],
                extinction_en_fermeture: None,
            },
            jours_fermes: vec![],
        }
    }

    #[test]
    fn rmvtpl_somme_a_deux_mouvements_par_place_et_par_jour() {
        let s = |u| (1..=24).map(|h| rmvtpl(u, h)).sum::<f64>();
        assert!((s(TypeUsageParking::Habitat) - 1.995).abs() < 1e-9);
        assert!((s(TypeUsageParking::Bureau) - 2.002).abs() < 1e-9);
        assert!((s(TypeUsageParking::Commerce) - 7.506).abs() < 1e-9);
    }

    #[test]
    fn eclairage_interieur_habitat_toute_l_annee() {
        // 75 W/place × 20 places en marche forcée 24 h/24 : 1500 W × 8760 h = 13 140 kWh.
        let (s, d) = eclairage(&parking(TypeParking::Interieur), TypeUsageParking::Habitat);
        let kwh: f64 = s.iter().sum::<f64>() / 1000.0;
        assert!((kwh - 13_140.0).abs() < 1e-6);
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn detection_abat_de_80_pour_cent() {
        let mut p = parking(TypeParking::Interieur);
        p.eclairage.plages_detection_semaine = vec![PlageHoraire {
            debut_h: 0,
            fin_h: 24,
        }];
        p.eclairage.plages_detection_week_end = vec![PlageHoraire {
            debut_h: 0,
            fin_h: 24,
        }];
        let (s, _) = eclairage(&p, TypeUsageParking::Habitat);
        let kwh: f64 = s.iter().sum::<f64>() / 1000.0;
        assert!((kwh - 0.2 * 13_140.0).abs() < 1e-6);
    }

    #[test]
    fn eclairage_exterieur_suit_le_tableau_323() {
        // Σ Fhext sur 24 h : 6 + 0,79 + 0,48 + 0,15 + 0,03 + 0,2 + 0,36 + 0,5 + 0,65 + 0,84 + 2 = 12.
        let somme: f64 = (1..=24).map(fh_ext).sum();
        assert!((somme - 12.0).abs() < 1e-9);
        let (s, _) = eclairage(&parking(TypeParking::Exterieur), TypeUsageParking::Habitat);
        let kwh: f64 = s.iter().sum::<f64>() / 1000.0;
        let attendu = 160.0 * 12.0 * 365.0 / 1000.0;
        assert!((kwh - attendu).abs() < 0.01 * attendu);
    }

    #[test]
    fn fermeture_eteint_ou_force() {
        let mut p = parking(TypeParking::Interieur);
        p.jours_fermes = vec![1, 2, 3];
        p.eclairage.extinction_en_fermeture = Some(true);
        let eteint: f64 = eclairage(&p, TypeUsageParking::Habitat).0.iter().sum();
        p.eclairage.extinction_en_fermeture = Some(false);
        let force: f64 = eclairage(&p, TypeUsageParking::Habitat).0.iter().sum();
        // Le premier jour légal compte 23 pas de temps.
        assert!((force - eteint - 3.0 * 24.0 * 1500.0).abs() < 2.0 * 1500.0);
    }

    #[test]
    fn ventilation_habitat_permanente_et_regulee() {
        let mut p = parking(TypeParking::Interieur);
        p.ventilation = Some(VentilationParking {
            regulee: false,
            p_vent600_w_par_place: None,
        });
        let (s, d) = ventilation_habitat(&p);
        // 40 W × 20 places × 8760 h.
        assert!((s.iter().sum::<f64>() - 40.0 * 20.0 * 8760.0).abs() < 1e-6);
        assert_eq!(d.len(), 1);
        p.ventilation = Some(VentilationParking {
            regulee: true,
            p_vent600_w_par_place: Some(40.0),
        });
        let (r, _) = ventilation_habitat(&p);
        let attendu = 40.0 * 20.0 * 1.995 * 365.0;
        assert!((r.iter().sum::<f64>() - attendu).abs() < 0.01 * attendu);
        p.type_parking = TypeParking::Exterieur;
        assert_eq!(ventilation_habitat(&p).0.iter().sum::<f64>(), 0.0);
    }

    #[test]
    fn ventilation_bureau_debit_requis() {
        // 100 places sur 1 étage : Larg = √(12·100/2) = 24,49 m, Long = 48,99 m, Peri = 146,97 m,
        // Lmoytraj = 146,97 × 0,5 = 73,5 m ; Dmvmt = 0,01 + 73,5/10000.
        let dm = d_mvmt_h(100.0, 1.0);
        assert!((dm - (0.01 + 73.485 / 10_000.0)).abs() < 1e-4);
        let mut p = parking(TypeParking::Interieur);
        p.npl = 100.0;
        let s = ventilation_hors_habitation(
            &p,
            TypeUsageParking::Bureau,
            VitessesVentilation::defaut(100.0),
        );
        assert!(s.iter().sum::<f64>() > 0.0);
        // Aucun mouvement avant 8 h légales.
        assert_eq!(s[0], 0.0);
    }
}
