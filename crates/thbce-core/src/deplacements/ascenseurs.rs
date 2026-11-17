// SPDX-License-Identifier: GPL-3.0-or-later
//! Consommation des ascenseurs (Th-BCE 2020 §10.1, `C_Bat_Ascenseurs`, p. 1295 à 1308).
//!
//! La fiche calcule, pour chaque cabine, une consommation annuelle en mobilité (`Etm`) et en
//! immobilité (`Eti`), puis la répartit au pas horaire avec le profil de mobilité des scénarios
//! conventionnels (`Uk`) et entre les zones desservies.
//!
//! Lectures retenues et variantes : `docs/questions.md`, Q50. Les variantes non exposées en
//! entrée sont portées par [`Lectures`].

use crate::entree::{Ascenseur, TechMac};
use crate::resultats::ValeurParDefaut;

/// Nombre de pas de temps de l'année.
const NB_H: usize = 8760;

/// Accélération terrestre « g », en m/s² (tableau 309).
const G: f64 = 9.81;
/// « Eporte » : énergie d'un cycle ouverture/fermeture de porte, en J.
const E_PORTE_J: f64 = 1188.0;
/// « deco » : ratio de masse pour la décoration.
const DECO: f64 = 1.2;
/// « Cf » : coefficient de frottement, en m/s².
const CF: f64 = 0.45;
/// « Cor_Ch » : facteur multiplicatif du coefficient de hauteur.
const COR_CH: f64 = 1.1;
/// « Cor_Emobcab » : facteur multiplicatif de l'énergie de mobilité.
const COR_EMOBCAB: f64 = 0.9;

/// Lectures du texte à variante (Q50). `Default` donne les lectures retenues.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lectures {
    /// « Mpass » en kg : 75 (§10.1.3.2, texte) ; le tableau 309 donne 120 (colonnes décalées).
    pub mpass_kg: f64,
    /// « Pveilleporte » en W : 13 (§10.1.3.2) ; le tableau 309 donne 75.
    pub p_veille_porte_w: f64,
    /// Termes de gravité de la phase d'accélération des éq. (2299) et (2300) tels qu'imprimés,
    /// en `(G + F(X))` (vrai) ; sinon en `(G − F(X))`, comme les autres phases.
    pub gravite_acceleration_imprimee: bool,
    /// « Timmob_n » (éq. 2296) moyenné par les poids `C(i,z)·Q/Qz` (vrai) ou somme brute du
    /// texte (faux ; identique pour une seule zone desservie).
    pub timmob_n_moyenne: bool,
}

impl Default for Lectures {
    fn default() -> Self {
        Self {
            mpass_kg: 75.0,
            p_veille_porte_w: 13.0,
            gravite_acceleration_imprimee: true,
            timmob_n_moyenne: true,
        }
    }
}

/// Zone desservie vue d'une cabine.
#[derive(Debug, Clone)]
pub struct ZoneDesservie<'a> {
    /// « Qz » : masse utile affectée à la zone, toutes cabines confondues (éq. 2279), en kg.
    pub q_z_kg: f64,
    /// « NBz » : nombre maximal d'occupants de la zone (éq. 2280).
    pub nb_z: f64,
    /// « BVz » : besoin de voyages par personne et par an (éq. 2281, tableau 310).
    pub bv_z: f64,
    /// « Uz(h) » : profil de mobilité de la zone, 8760 valeurs (éq. 31).
    pub u_z: &'a [f64],
    /// Zone d'habitation (éq. 2294) ou non (éq. 2295).
    pub habitation: bool,
}

/// « Bv(k) » du tableau 310 pour un usage de zone ; `None` pour l'usage 1, absent du tableau.
pub fn bv(usage_numero: u8) -> Option<f64> {
    match usage_numero {
        2 => Some(1582.4),
        _ => None,
    }
}

/// Résultats d'une cabine.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultatCabine {
    /// « Ndem » : démarrages par an (éq. 2283).
    pub nb_demarrages: f64,
    /// « Tmob » en s (éq. 2291).
    pub t_mob_s: f64,
    /// « Tmobh » en h/jour (éq. 2292).
    pub t_mob_h: f64,
    /// « Emoy » en J : aller-retour moyen (éq. 2301).
    pub e_moy_j: f64,
    /// « Pti » en W (donnée ou éq. 2274).
    pub pti_w: f64,
    /// « Etm » en Wh (éq. 2302).
    pub etm_wh: f64,
    /// « Eti » en Wh (éq. 2303).
    pub eti_wh: f64,
    /// « Etot » en Wh (éq. 2304).
    pub etot_wh: f64,
    /// « Pcab(h) » : 8760 valeurs en Wh (éq. 2308).
    pub pcab_wh: Vec<f64>,
    /// Valeurs par défaut appliquées.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// « Rg(TechMac) », tableau 311.
fn rg(t: TechMac) -> f64 {
    match t {
        TechMac::TractionAvecReduction | TechMac::Autre => 0.6,
        TechMac::TractionSansReduction => 0.8,
        TechMac::Hydraulique => 0.29,
    }
}

/// « Ac(V) », tableau 312 (mêmes valeurs en électrique et en hydraulique), en m/s².
fn ac(v: f64) -> f64 {
    if v <= 1.0 {
        0.5
    } else if v <= 2.0 {
        0.8
    } else {
        1.2
    }
}

/// « α(TechMac) », tableau 313.
fn alpha(t: TechMac) -> f64 {
    match t {
        TechMac::TractionAvecReduction | TechMac::Autre => 6.0,
        TechMac::TractionSansReduction | TechMac::Hydraulique => 0.0,
    }
}

/// « Cp(TechMac) », tableau 314.
pub fn cp_defaut(t: TechMac) -> f64 {
    match t {
        TechMac::Hydraulique => -1.2,
        _ => 0.5,
    }
}

/// « Pec(Q) », tableau 315, en W.
fn pec(q: f64) -> f64 {
    if q <= 630.0 {
        140.0
    } else if q <= 1275.0 {
        210.0
    } else {
        280.0
    }
}

/// « S(direction, X) », tableau 317, pour X = 0 ; 0,25 ; 0,5 ; 0,75 ; 1 (mêmes valeurs en haut
/// et en bas).
const SPECTRE: [(f64, f64); 5] = [(0.0, 0.5), (0.25, 0.3), (0.5, 0.1), (0.75, 0.1), (1.0, 0.0)];

/// Éq. (2299) et (2300) : énergie d'un trajet de distance `z` avec la charge `x`, en J.
#[allow(clippy::too_many_arguments)]
fn e_trajet(
    monte: bool,
    f: f64,
    g_cp: f64,
    mi: f64,
    v: f64,
    a: f64,
    z: f64,
    gravite_imprimee: bool,
) -> f64 {
    let m = g_cp + f; // G + F(X)
    let d = g_cp - f; // G − F(X)
    let d_acc = v * v / (2.0 * a);
    let cin = (m + mi) * v * v / 2.0;
    let sens = if monte { -1.0 } else { 1.0 };
    // Gravité de la phase d'accélération : (G + F) tel qu'imprimé, (G − F) en variante.
    let grav_acc = if gravite_imprimee { m } else { d };
    let t1 = m * CF * d_acc + sens * grav_acc * G * d_acc + cin;
    let t2 = m * CF * (z - d_acc) + sens * d * G * (z - d_acc);
    let t3 = m * CF * d_acc + d * G * d_acc - cin;
    t1.max(0.0) + t2.max(0.0) + t3.max(0.0)
}

/// Éq. (2301) : `Emoy`, en J.
fn e_moy(a: &Ascenseur, ch: f64, lec: &Lectures, cp: f64) -> f64 {
    let p = DECO * a.q_kg; // éq. (2288)
    let g_cp = p + cp * a.q_kg; // éq. (2289)
    let mi = alpha(a.tech_mac) * (g_cp + p); // éq. (2290)
    let acc = ac(a.v_m_s);
    let z = ch * a.h_m;
    let mut somme = 0.0;
    for (x, s) in SPECTRE {
        let f = p + x * a.q_kg; // éq. (2298)
        for monte in [true, false] {
            somme += s * e_trajet(
                monte,
                f,
                g_cp,
                mi,
                a.v_m_s,
                acc,
                z,
                lec.gravite_acceleration_imprimee,
            );
        }
    }
    somme / rg(a.tech_mac)
}

/// Éq. (2274) : valeur par défaut de `Pti`, en W.
fn pti_defaut(a: &Ascenseur, lec: &Lectures) -> f64 {
    // Pman 75 + (netage + 1)·(Pboutonpal 0 + Pindpal 2) + Pfrein 0 + Pveilleporte
    // + Pincab 5 + Pec(Q) + Palarm 10 (tableau 316).
    75.0 + f64::from(a.netage + 1) * 2.0 + lec.p_veille_porte_w + 5.0 + pec(a.q_kg) + 10.0
}

/// Calcule une cabine. `zones` : zones desservies (`C(i,z) = 1`), avec `Qz` calculé sur toutes
/// les cabines. Éq. (2282) à (2308).
pub fn calculer_cabine(
    a: &Ascenseur,
    zones: &[ZoneDesservie<'_>],
    lec: &Lectures,
) -> ResultatCabine {
    let mut defauts = Vec::new();
    let mut defaut = |champ: &str, valeur: f64, regle: &str| {
        defauts.push(ValeurParDefaut {
            champ: champ.to_owned(),
            valeur,
            regle: regle.to_owned(),
        });
    };

    // Éq. (2282) : besoin de voyages annuel affecté à la cabine.
    let bvnb: f64 = zones
        .iter()
        .map(|z| a.q_kg / z.q_z_kg * z.bv_z * z.nb_z)
        .sum();
    // Éq. (2283) et (2284) : Σ X·S = 0,2.
    let nb_demarrages = bvnb * lec.mpass_kg / (a.q_kg * 0.2);
    let cp = a.cp.unwrap_or_else(|| {
        let c = cp_defaut(a.tech_mac);
        defaut(
            "cp",
            c,
            "Th-BCE 2020 §10.1.3.2, tableau 314 : coefficient d'équilibrage selon TechMac",
        );
        c
    });
    // Éq. (2286).
    let n = f64::from(a.netage);
    let ch = (n + 1.0) / (n + 2.0) * COR_CH;
    let acc = ac(a.v_m_s);
    // Éq. (2291) à (2293).
    let t_mob_s = (ch * a.h_m / a.v_m_s + a.v_m_s / acc) * nb_demarrages;
    let t_mob_h = t_mob_s / (365.0 * 3600.0);
    let t_immob_s = 24.0 * 365.0 * 3600.0 - t_mob_s;
    // Éq. (2294) à (2297).
    let tn = |habitation: bool| {
        if habitation {
            8.0 * 365.0 * 3600.0
        } else {
            (12.0 * 365.0 + 52.0 * 48.0 + 9.0 * 24.0) * 3600.0
        }
    };
    let (somme_tn, somme_poids) = zones.iter().fold((0.0, 0.0), |(s, p), z| {
        let w = a.q_kg / z.q_z_kg;
        (s + w * tn(z.habitation), p + w)
    });
    let t_immob_n_s = if lec.timmob_n_moyenne && somme_poids > 0.0 {
        somme_tn / somme_poids
    } else {
        somme_tn
    };
    let t_immob_j_s = (t_immob_s - t_immob_n_s).max(0.0);

    // §10.1.3.3 : valeurs par défaut des paramètres de veille.
    let regle_pti = "Th-BCE 2020 §10.1.3.3, éq. (2274) : somme des puissances auxiliaires";
    let pti_w = a.pti_w.unwrap_or_else(|| {
        let v = pti_defaut(a, lec);
        defaut("pti_w", v, regle_pti);
        v
    });
    let (mut dp1, mut t1, mut dp2, mut t2) = (0.0, 86_400.0, 0.0, 86_400.0);
    if a.sc_veille {
        let connu = |x: Option<f64>| x;
        let pti_connu = a.pti_w.is_some();
        let (c_dp1, c_t1, c_dp2, c_t2) =
            (connu(a.dp1_w), connu(a.t1_s), connu(a.dp2_w), connu(a.t2_s));
        let etage1 = pti_connu && c_dp1.is_some() && c_t1.is_some();
        let etage2 = etage1 && c_dp2.is_some() && c_t2.is_some();
        if etage1 {
            dp1 = c_dp1.unwrap_or(0.0);
            t1 = c_t1.unwrap_or(86_400.0);
        }
        if etage2 {
            dp2 = c_dp2.unwrap_or(0.0);
            t2 = c_t2.unwrap_or(86_400.0);
        }
    }

    // Éq. (2301) et (2302).
    let e_moy_j = e_moy(a, ch, lec, cp);
    let etm_wh =
        (e_moy_j * COR_EMOBCAB * nb_demarrages / 2.0 + E_PORTE_J * nb_demarrages + pti_w * t_mob_s)
            / 3600.0;
    // Éq. (2303), dont les termes en dP1 et dP2 portent la durée d'immobilité de la période
    // (Q50 point 4) : puissance Pti avant T1, Pti − dP1 jusqu'à T1 + T2, puis Pti − dP1 − dP2.
    let part = |duree_s: f64, nb_periodes: f64| {
        let longueur = if nb_periodes > 0.0 {
            duree_s / nb_periodes
        } else {
            f64::INFINITY
        };
        let f1 = (t1 / longueur).min(1.0);
        let f12 = ((t1 + t2) / longueur).min(1.0);
        (dp1 * f1 + dp2 * f12 + (pti_w - dp1 - dp2)) * duree_s / 3600.0
    };
    let eti_wh = part(t_immob_j_s, nb_demarrages) + part(t_immob_n_s, 365.0);
    let etot_wh = etm_wh + eti_wh;

    // Éq. (2305) à (2308).
    let mut u = vec![0.0; NB_H];
    for z in zones {
        let w = a.q_kg / z.q_z_kg;
        for (x, uz) in u.iter_mut().zip(z.u_z) {
            *x += w * uz;
        }
    }
    let somme_u: f64 = u.iter().sum();
    let denom = 24.0 / t_mob_h - 1.0;
    let pcab_wh = u
        .iter()
        .map(|uh| {
            let fmob = if somme_u > 0.0 { uh / somme_u } else { 0.0 };
            let fimob = if t_mob_h > 0.0 && denom > 0.0 {
                (1.0 / (365.0 * t_mob_h) - fmob) / denom
            } else {
                1.0 / NB_H as f64
            };
            fimob * eti_wh + fmob * etm_wh
        })
        .collect();

    ResultatCabine {
        nb_demarrages,
        t_mob_s,
        t_mob_h,
        e_moy_j,
        pti_w,
        etm_wh,
        eti_wh,
        etot_wh,
        pcab_wh,
        valeurs_par_defaut: defauts,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn cabine() -> Ascenseur {
        Ascenseur {
            id: "a".into(),
            tech_mac: TechMac::TractionSansReduction,
            q_kg: 630.0,
            v_m_s: 1.0,
            h_m: 12.0,
            netage: 4,
            cp: None,
            zones_desservies: vec!["z".into()],
            sc_veille: false,
            pti_w: None,
            dp1_w: None,
            t1_s: None,
            dp2_w: None,
            t2_s: None,
        }
    }

    #[test]
    fn pti_par_defaut_eq_2274() {
        // 75 + 5·2 + 13 + 5 + 140 + 10 = 253 W.
        assert!((pti_defaut(&cabine(), &Lectures::default()) - 253.0).abs() < 1e-12);
    }

    #[test]
    fn cabine_seule_a_la_somme_horaire_de_etot() {
        let u: Vec<f64> = (0..NB_H)
            .map(|h| if h % 24 == 9 { 0.3 } else { 0.0 })
            .collect();
        let z = ZoneDesservie {
            q_z_kg: 630.0,
            nb_z: 20.0,
            bv_z: 1582.4,
            u_z: &u,
            habitation: true,
        };
        let r = calculer_cabine(&cabine(), &[z], &Lectures::default());
        let somme: f64 = r.pcab_wh.iter().sum();
        assert!((somme - r.etot_wh).abs() < 1e-6 * r.etot_wh);
        // 20 × 1582,4 × 75 / (630 × 0,2) démarrages.
        assert!((r.nb_demarrages - 20.0 * 1582.4 * 75.0 / 126.0).abs() < 1e-6);
        // Sans veille : Eti = Pti × durée d'immobilité.
        let attendu = 253.0 * (24.0 * 365.0 * 3600.0 - r.t_mob_s) / 3600.0;
        assert!((r.eti_wh - attendu).abs() < 1e-6 * attendu);
        assert_eq!(r.valeurs_par_defaut.len(), 2);
    }

    #[test]
    fn energie_de_trajet_positive_et_montee_a_vide_plus_faible() {
        // Contrepoids de 0,5·Q : monter à vide consomme moins que monter chargé.
        let (f_vide, f_plein) = (756.0, 756.0 + 630.0);
        let g_cp = 756.0 + 0.5 * 630.0;
        let vide = e_trajet(true, f_vide, g_cp, 0.0, 1.0, 0.5, 13.2, true);
        let plein = e_trajet(true, f_plein, g_cp, 0.0, 1.0, 0.5, 13.2, true);
        assert!(vide >= 0.0 && plein > vide);
    }

    #[test]
    fn veille_reduit_l_energie_d_immobilite() {
        let u = vec![1.0; NB_H];
        let z = || ZoneDesservie {
            q_z_kg: 630.0,
            nb_z: 20.0,
            bv_z: 1582.4,
            u_z: &u,
            habitation: true,
        };
        let sans = calculer_cabine(&cabine(), &[z()], &Lectures::default());
        let mut a = cabine();
        a.sc_veille = true;
        a.pti_w = Some(253.0);
        a.dp1_w = Some(150.0);
        a.t1_s = Some(300.0);
        a.dp2_w = Some(50.0);
        a.t2_s = Some(600.0);
        let avec = calculer_cabine(&a, &[z()], &Lectures::default());
        assert!(avec.eti_wh < sans.eti_wh);
        assert!(avec.eti_wh > 0.0);
    }

    /// Effet des variantes de lecture (Q50) sur la cabine du cas d'exemple de l'immeuble :
    /// 21,9 occupants maximaux, 630 kg, 1 m/s, 10,2 m, 4 étages.
    #[test]
    fn effet_des_variantes_de_lecture() {
        let u = vec![1.0; NB_H];
        let z = || ZoneDesservie {
            q_z_kg: 630.0,
            nb_z: 21.88,
            bv_z: 1582.4,
            u_z: &u,
            habitation: true,
        };
        let a = cabine();
        let ref_ = calculer_cabine(&a, &[z()], &Lectures::default());
        let variante = |l: Lectures| calculer_cabine(&a, &[z()], &l).etot_wh / ref_.etot_wh - 1.0;
        let mpass = variante(Lectures {
            mpass_kg: 120.0,
            ..Lectures::default()
        });
        let veille = variante(Lectures {
            p_veille_porte_w: 75.0,
            ..Lectures::default()
        });
        let gravite = variante(Lectures {
            gravite_acceleration_imprimee: false,
            ..Lectures::default()
        });
        println!(
            "Q50 : Mpass 120 kg {:+.2} %, Pveilleporte 75 W {:+.2} %, gravité (G−F) {:+.4} %",
            mpass * 100.0,
            veille * 100.0,
            gravite * 100.0
        );
        assert!(mpass > 0.0 && veille > 0.0);
        assert!(gravite < 0.0 && gravite > -0.05);
    }
}
