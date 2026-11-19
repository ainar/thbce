// SPDX-License-Identifier: GPL-3.0-or-later
//! Consommation des escalators et tapis roulants (Th-BCE 2020 §10.2, `C_Bat_Escalators`,
//! p. 1309 à 1317, éq. 2311 à 2325).
//!
//! Contrairement aux ascenseurs, la fiche modélise **directement l'ensemble des escalators** du
//! bâtiment au pas horaire : masse totale transportée sur l'année (éq. 2315 à 2319), répartie
//! par le profil de mobilité des zones desservies (éq. 2320), puissance en occupation
//! (éq. 2311 à 2314 et 2321), en mise en veille (éq. 2322 et 2323), puis partition entre les
//! zones (éq. 2324). Un tapis roulant est un escalator de hauteur nulle (éq. 2325).
//!
//! Lectures retenues et variantes : `docs/questions/Q115.md`.

use crate::entree::{Escalator, EtatFermeture, SensEscalator};
use crate::resultats::ValeurParDefaut;

/// Nombre de pas de temps de l'année.
const NB_H: usize = 8760;
/// « g » : accélération terrestre, en m/s² (tableau 318 et §10.2.3.2).
const G: f64 = 9.81;
/// « Mpass » : masse conventionnelle d'un passager, en kg (§10.2.3.2).
const M_PASS_KG: f64 = 75.0;
/// « R_m » : rendement global marginal à la charge en montée (tableau 319).
const R_M: f64 = 0.8;
/// « R_d » : rendement global marginal à la charge en descente (tableau 319).
const R_D: f64 = 0.8;
/// « Pstandby_defaut » : puissance à l'arrêt par défaut d'une machine, en W (éq. 2323).
const P_STANDBY_DEFAUT_W: f64 = 88.0;

/// Lectures du texte à variante (Q115). `Default` donne les lectures retenues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Lectures {
    /// Masse transportée pendant l'heure `h` : `M · T(h)` (faux, retenu : `T(h)` est normé, sa
    /// somme annuelle vaut 1, et distribue `M`) ou `M_h · T(h)` avec `M_h = M/8760` tel
    /// qu'imprimé (vrai : la masse annuelle transportée serait alors `M/8760`).
    pub masse_horaire_imprimee: bool,
    /// Puissance à l'arrêt (éq. 2323) : 88 W par machine (faux, retenu) ou 88 W pour
    /// l'ensemble des escalators du bâtiment (vrai).
    pub standby_defaut_pour_l_ensemble: bool,
}

/// Zone desservie par au moins un escalator (`C(z) = 1`).
#[derive(Debug, Clone)]
pub struct ZoneEscalators<'a> {
    /// « NBz » : nombre maximal d'occupants conventionnels de la zone (éq. 2315).
    pub nb_z: f64,
    /// « BVz » : besoin de voyages par personne et par an (éq. 2316, tableau 310).
    pub bv_z: f64,
    /// « Uz(h) » : profil de mobilité de la zone, 8760 valeurs (éq. 31).
    pub u_z: &'a [f64],
}

/// « Bv(k) » du tableau 310 pour l'usage de zone `k` (le texte de §10.2.3.2 renvoie à un
/// « tableau 311 », qui est celui des rendements des ascenseurs : le besoin de voyages est celui
/// du tableau 310) ; `None` pour l'usage 1, absent du tableau.
pub fn bv(usage_numero: u8) -> Option<f64> {
    match usage_numero {
        2 => Some(1582.4),
        3 => Some(1731.9),
        4 => Some(804.0),
        5 => Some(1206.0),
        6 | 7 => Some(1608.0),
        8..=11 => Some(2920.0),
        12 => Some(924.0),
        13 => Some(2190.0),
        14 => Some(510.0),
        15 => Some(1460.0),
        16 => Some(1248.0),
        17 => Some(4158.0),
        18 => Some(402.0),
        19 => Some(2190.0),
        20 => Some(4380.0),
        21 => Some(2920.0),
        22 => Some(7665.0),
        23 => Some(2190.0),
        24 => Some(1040.0),
        25 => Some(2680.0),
        26 => Some(402.0),
        27 => Some(1206.0),
        28 => Some(2680.0),
        _ => None,
    }
}

/// Résultats de l'ensemble des escalators d'un bâtiment.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultatEscalators {
    /// « BVNB » : besoin de voyages annuel affecté à l'ensemble (éq. 2317).
    pub bvnb: f64,
    /// « M » : masse totale transportée sur l'année, en kg (éq. 2318).
    pub m_kg: f64,
    /// « Pnoload_m » : puissance à vide cumulée des machines de montée, en W.
    pub p_noload_m_w: f64,
    /// « Pnoload_d » : puissance à vide cumulée des machines de descente, en W.
    pub p_noload_d_w: f64,
    /// « H_m » : hauteur cumulée de montée, en m.
    pub h_m_m: f64,
    /// « H_d » : hauteur cumulée de descente, en m.
    pub h_d_m: f64,
    /// « Poff » : puissance cumulée à l'arrêt, en W (éq. 2322 et 2323).
    pub p_off_w: f64,
    /// Nombre d'heures d'occupation de l'année (`T(h) > 0`).
    pub nb_h_occupation: usize,
    /// « Pesc(h) » : 8760 valeurs en Wh (éq. 2321 à 2323 et 2325).
    pub pesc_wh: Vec<f64>,
    /// « Pescz(h) » : partition entre les zones desservies (éq. 2324), dans l'ordre de `zones`.
    pub pesc_zone_wh: Vec<Vec<f64>>,
    /// Valeurs par défaut appliquées.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
}

/// Puissance à l'arrêt d'une machine, en W : éq. (2322) si le scénario de veille est connu
/// (puissance de l'état choisi), éq. (2323) sinon.
fn p_off_machine_w(e: &Escalator) -> Option<f64> {
    if e.sc_veille {
        Some(
            match e.etat_fermeture.unwrap_or(EtatFermeture::ArretComplet) {
                EtatFermeture::ArretComplet => e.p_standby_w?,
                EtatFermeture::ArretSansPassager => e.p_autostart_w?,
                EtatFermeture::VitesseNominaleAVide => e.p_noload_w,
            },
        )
    } else {
        None
    }
}

/// Th-BCE 2020 §10.2.3.4 et §10.2.3.5, éq. (2311) à (2325). `zones` : zones desservies par au
/// moins un escalator (`C(z) = 1`).
pub fn calculer(
    escalators: &[Escalator],
    zones: &[ZoneEscalators<'_>],
    lec: &Lectures,
) -> ResultatEscalators {
    let mut defauts = Vec::new();
    // Éq. (2317) à (2319).
    let nbv: Vec<f64> = zones.iter().map(|z| z.bv_z * z.nb_z).collect();
    let bvnb: f64 = nbv.iter().sum();
    let m_kg = M_PASS_KG * bvnb;
    let m_h_kg = m_kg / NB_H as f64;

    // Étape 2 : caractéristiques cumulées des machines.
    let (mut p_noload_m, mut p_noload_d, mut h_m, mut h_d) = (0.0, 0.0, 0.0, 0.0);
    let mut p_off = 0.0;
    let mut nb_sans_veille = 0_usize;
    for e in escalators {
        // Éq. (2312) et (2314) : avec régulation basse vitesse, Pslowspeed tient lieu de
        // Pnoload.
        let p_vide = if e.slowspeed {
            e.p_slowspeed_w.unwrap_or(e.p_noload_w)
        } else {
            e.p_noload_w
        };
        match e.sens {
            SensEscalator::Montee => {
                p_noload_m += p_vide;
                h_m += e.h_m;
            }
            SensEscalator::Descente => {
                p_noload_d += p_vide;
                h_d += e.h_m;
            }
        }
        match p_off_machine_w(e) {
            Some(p) => p_off += p,
            None => {
                nb_sans_veille += 1;
            }
        }
    }
    if nb_sans_veille > 0 {
        // Éq. (2323).
        let n = if lec.standby_defaut_pour_l_ensemble {
            1.0
        } else {
            nb_sans_veille as f64
        };
        p_off += n * P_STANDBY_DEFAUT_W;
        defauts.push(ValeurParDefaut {
            champ: "escalators[].p_standby_w".to_owned(),
            valeur: P_STANDBY_DEFAUT_W,
            regle: "Th-BCE 2020 §10.2.3.4.3, éq. (2323) : puissance à l'arrêt par défaut \
                    quand le scénario de mise en veille n'est pas connu (ScVeille = 0)"
                .to_owned(),
        });
    }

    // Étape 3, éq. (2320) : T(h).
    let mut t = vec![0.0; NB_H];
    let mut somme = 0.0;
    for (z, w) in zones.iter().zip(&nbv) {
        for (x, u) in t.iter_mut().zip(z.u_z) {
            *x += w * u;
        }
    }
    for x in &t {
        somme += x;
    }
    if somme > 0.0 {
        for x in &mut t {
            *x /= somme;
        }
    }

    // Étape 4 : demande horaire.
    // Éq. (2321) : Pesc_m(M·T, H_m/r_m) + Pesc_d(M·T, H_d·r_d), avec M·T en kg par heure,
    // soit M·T/3600 en kg/s : la puissance de levage vaut ṁ·g·H (éq. 2311 et 2313, où
    // M·g·sinθ·Vnom est la même grandeur).
    let mut pesc_wh = vec![0.0; NB_H];
    let mut nb_h_occ = 0;
    for (h, p) in pesc_wh.iter_mut().enumerate() {
        if t[h] > 0.0 {
            nb_h_occ += 1;
            let masse_h = if lec.masse_horaire_imprimee {
                m_h_kg * t[h]
            } else {
                m_kg * t[h]
            };
            let flux = masse_h / 3600.0;
            let monte = p_noload_m + flux * G * (h_m / R_M);
            let descend = (p_noload_d - flux * G * (h_d * R_D)).max(0.0);
            *p = monte + descend;
        } else {
            *p = p_off;
        }
    }

    // §10.2.3.5, éq. (2324).
    let pesc_zone_wh: Vec<Vec<f64>> = nbv
        .iter()
        .map(|w| {
            let part = if bvnb > 0.0 { w / bvnb } else { 0.0 };
            pesc_wh.iter().map(|p| p * part).collect()
        })
        .collect();

    ResultatEscalators {
        bvnb,
        m_kg,
        p_noload_m_w: p_noload_m,
        p_noload_d_w: p_noload_d,
        h_m_m: h_m,
        h_d_m: h_d,
        p_off_w: p_off,
        nb_h_occupation: nb_h_occ,
        pesc_wh,
        pesc_zone_wh,
        valeurs_par_defaut: defauts,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn machine(sens: SensEscalator, h_m: f64, p_noload_w: f64) -> Escalator {
        Escalator {
            id: "e".into(),
            sens,
            v_nom_m_s: None,
            h_m,
            p_noload_w,
            slowspeed: false,
            p_slowspeed_w: None,
            sc_veille: false,
            etat_fermeture: None,
            p_standby_w: None,
            p_autostart_w: None,
            zones_desservies: vec!["z".into()],
        }
    }

    /// Profil de 10 heures par jour (de 9 h à 18 h), 3 650 heures dans l'année.
    fn profil() -> Vec<f64> {
        (0..NB_H)
            .map(|h| {
                if (9..19).contains(&(h % 24)) {
                    1.0
                } else {
                    0.0
                }
            })
            .collect()
    }

    #[test]
    fn tapis_roulant_eq_2325() {
        // Tapis (H = 0) de 1 000 W à vide : 1 000 Wh pendant les 3 650 heures d'occupation,
        // 88 Wh (éq. 2323) les 5 110 autres.
        let u = profil();
        let z = ZoneEscalators {
            nb_z: 100.0,
            bv_z: 1731.9,
            u_z: &u,
        };
        let r = calculer(
            &[machine(SensEscalator::Montee, 0.0, 1000.0)],
            &[z],
            &Lectures::default(),
        );
        assert_eq!(r.nb_h_occupation, 3650);
        let attendu = 3650.0 * 1000.0 + 5110.0 * 88.0;
        assert!((r.pesc_wh.iter().sum::<f64>() - attendu).abs() < 1e-6);
        assert_eq!(r.valeurs_par_defaut.len(), 1);
    }

    #[test]
    fn escalator_montant_et_descendant_eq_2318_2321() {
        let u = profil();
        let z = ZoneEscalators {
            nb_z: 100.0,
            bv_z: 1731.9,
            u_z: &u,
        };
        let r = calculer(
            &[
                machine(SensEscalator::Montee, 4.0, 2000.0),
                machine(SensEscalator::Descente, 4.0, 1500.0),
            ],
            &[z],
            &Lectures::default(),
        );
        // BVNB = 100 × 1731,9 ; M = 75 × BVNB ; T(h) = 1/3650 aux heures d'occupation.
        let m = 75.0 * 100.0 * 1731.9;
        assert!((r.m_kg - m).abs() < 1e-6);
        let flux = m / 3650.0 / 3600.0;
        // Occupation : (2000 + ṁ·g·4/0,8) + max(0, 1500 − ṁ·g·4·0,8).
        let occ = 2000.0 + flux * 9.81 * 5.0 + (1500.0 - flux * 9.81 * 3.2);
        let h = 10; // 10 h : occupé.
        assert!((r.pesc_wh[h] - occ).abs() < 1e-9);
        // Fermeture : 2 × 88 W.
        assert_eq!(r.pesc_wh[3], 176.0);
    }

    #[test]
    fn regulation_basse_vitesse_et_veille_connue() {
        let u = profil();
        let z = ZoneEscalators {
            nb_z: 100.0,
            bv_z: 1731.9,
            u_z: &u,
        };
        let mut e = machine(SensEscalator::Montee, 0.0, 1000.0);
        e.slowspeed = true;
        e.p_slowspeed_w = Some(400.0);
        e.sc_veille = true;
        e.p_standby_w = Some(30.0);
        let r = calculer(&[e], &[z], &Lectures::default());
        // Éq. (2312) : Pslowspeed à la place de Pnoload ; éq. (2322) : Pstandby.
        assert_eq!(r.pesc_wh[10], 400.0);
        assert_eq!(r.pesc_wh[3], 30.0);
        assert!(r.valeurs_par_defaut.is_empty());
    }

    #[test]
    fn partition_entre_zones_eq_2324() {
        let u = profil();
        let z1 = ZoneEscalators {
            nb_z: 100.0,
            bv_z: 1000.0,
            u_z: &u,
        };
        let z2 = ZoneEscalators {
            nb_z: 50.0,
            bv_z: 3000.0,
            u_z: &u,
        };
        let r = calculer(
            &[machine(SensEscalator::Montee, 0.0, 1000.0)],
            &[z1, z2],
            &Lectures::default(),
        );
        // Poids 100 000 et 150 000 : 40 % et 60 %.
        assert!((r.pesc_zone_wh[0][10] - 400.0).abs() < 1e-9);
        assert!((r.pesc_zone_wh[1][10] - 600.0).abs() < 1e-9);
    }

    #[test]
    fn masse_horaire_imprimee_donne_une_charge_8760_fois_plus_faible() {
        let u = profil();
        let z = || ZoneEscalators {
            nb_z: 100.0,
            bv_z: 1731.9,
            u_z: &u,
        };
        let e = [machine(SensEscalator::Montee, 4.0, 2000.0)];
        let a = calculer(&e, &[z()], &Lectures::default());
        let b = calculer(
            &e,
            &[z()],
            &Lectures {
                masse_horaire_imprimee: true,
                ..Lectures::default()
            },
        );
        let (ca, cb) = (a.pesc_wh[10] - 2000.0, b.pesc_wh[10] - 2000.0);
        assert!((ca / cb - 8760.0).abs() < 1e-6);
    }

    #[test]
    fn bv_tableau_310() {
        assert_eq!(bv(17), Some(4158.0));
        assert_eq!(bv(22), Some(7665.0));
        assert_eq!(bv(1), None);
    }
}
