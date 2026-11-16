// SPDX-License-Identifier: GPL-3.0-or-later
//! Sorties du mode Th-B : fiche `C_CALC_Sorties du mode Th-B` (Th-BCE 2020 §13.1).
//!
//! Besoins mensuels et annuels de chauffage, de refroidissement et d'éclairage, rapportés à la
//! surface de référence, et besoin bioclimatique Bbio, au niveau du groupe, de la zone et du
//! bâtiment. Les surfaces de référence (éq. 2375 à 2378) sont calculées dans
//! [`crate::calcul`].

use crate::resultats::{BesoinsBbio, ResultatsBbio};

/// « α1 » : constante associée aux besoins de chaud (§13.1.2, tableau 331).
pub const ALPHA_1: f64 = 2.0;

/// « α2 » : constante associée aux besoins de froid (tableau 331).
pub const ALPHA_2: f64 = 2.0;

/// « β » : constante associée aux besoins d'éclairage (tableau 331).
pub const BETA: f64 = 5.0;

/// Nombre de mois de l'année conventionnelle.
pub const NB_MOIS: usize = 12;

/// Énergies mensuelles d'un groupe, ou sommées sur une zone ou un bâtiment : numérateurs des
/// éq. (2379) à (2381).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EnergiesMensuelles {
    /// Somme sur chaque mois des `Qsys_ch(h)`, en Wh.
    pub q_sys_ch_wh: [f64; NB_MOIS],
    /// Somme sur chaque mois des `Qsys_fr(h)`, en Wh, comptés positivement.
    pub q_sys_fr_wh: [f64; NB_MOIS],
    /// Somme sur chaque mois des `CECL(h)`, en kWh.
    pub c_ecl_kwh: [f64; NB_MOIS],
}

impl EnergiesMensuelles {
    /// Ajoute les énergies d'un autre groupe ou d'une autre zone.
    ///
    /// Th-BCE 2020 §13.1.3 : pour chaque zone et pour chaque bâtiment, « les besoins mensuels,
    /// annuels et le besoin bioclimatique s'obtiennent par sommation sur l'ensemble des
    /// groupes ». Des besoins exprimés par m² de surface de référence ne s'additionnent pas :
    /// ce sont les énergies qui sont sommées, puis rapportées à la surface de référence de la
    /// zone ou du bâtiment (éq. 2377 et 2378) (décision Q39 point 7).
    pub fn ajouter(&mut self, autre: &Self) {
        for m in 0..NB_MOIS {
            self.q_sys_ch_wh[m] += autre.q_sys_ch_wh[m];
            self.q_sys_fr_wh[m] += autre.q_sys_fr_wh[m];
            self.c_ecl_kwh[m] += autre.c_ecl_kwh[m];
        }
    }
}

/// « Bbio_pts_mois » : besoin bioclimatique d'un mois, en points.
///
/// Th-BCE 2020 §13.1.3, éq. (2385), lue selon la décision Q10 :
/// `Bbio_pts_mois = α1·B_Ch_mois + α2·B_Fr_mois + β·B_Ecl_mois` (le texte imprime les besoins
/// annuels au second membre, ce qui multiplierait le Bbio par 12 dans l'éq. 2386).
pub fn bbio_pts(b_ch_kwh_m2: f64, b_fr_kwh_m2: f64, b_ecl_kwh_m2: f64) -> f64 {
    ALPHA_1 * b_ch_kwh_m2 + ALPHA_2 * b_fr_kwh_m2 + BETA * b_ecl_kwh_m2
}

/// Besoins mensuels et annuels et Bbio d'un groupe, d'une zone ou d'un bâtiment.
///
/// Th-BCE 2020 §13.1.3 :
/// - éq. (2379) à (2381) : `B_Ch_mois = Σ Qsys_ch(h)/SREF`, de même pour le refroidissement et
///   l'éclairage, en kWh par m² de surface de référence ; `Qsys` étant en Wh (§13.1.2), la
///   somme est divisée par 1000 ; `CECL` est déjà en kWh (décision Q33) ; le besoin de
///   refroidissement est compté positivement, `Qsys_fr` étant négatif (§8.1.3.7, éq. 810) ;
/// - éq. (2382) à (2384) : besoins annuels, somme des besoins mensuels ;
/// - éq. (2385) : Bbio mensuel ([`bbio_pts`]) ; éq. (2386) : Bbio annuel, somme des douze Bbio
///   mensuels.
///
/// Une surface de référence nulle donne des besoins nuls.
pub fn sorties(energies: &EnergiesMensuelles, sref_m2: f64) -> ResultatsBbio {
    let par_m2 = |x: f64| if sref_m2 > 0.0 { x / sref_m2 } else { 0.0 };
    let mut annuel = BesoinsBbio::default();
    let mensuel: Vec<BesoinsBbio> = (0..NB_MOIS)
        .map(|m| {
            let b_ch_kwh_m2 = par_m2(energies.q_sys_ch_wh[m] / 1000.0);
            let b_fr_kwh_m2 = par_m2(energies.q_sys_fr_wh[m] / 1000.0);
            let b_ecl_kwh_m2 = par_m2(energies.c_ecl_kwh[m]);
            let mois = BesoinsBbio {
                b_ch_kwh_m2,
                b_fr_kwh_m2,
                b_ecl_kwh_m2,
                bbio_pts: bbio_pts(b_ch_kwh_m2, b_fr_kwh_m2, b_ecl_kwh_m2),
            };
            annuel.b_ch_kwh_m2 += mois.b_ch_kwh_m2;
            annuel.b_fr_kwh_m2 += mois.b_fr_kwh_m2;
            annuel.b_ecl_kwh_m2 += mois.b_ecl_kwh_m2;
            annuel.bbio_pts += mois.bbio_pts;
            mois
        })
        .collect();
    ResultatsBbio { annuel, mensuel }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn constantes_du_tableau_331() {
        assert_eq!((ALPHA_1, ALPHA_2, BETA), (2.0, 2.0, 5.0));
        // Éq. (2385) : 2 × 10 + 2 × 3 + 5 × 1,5 = 33,5 points.
        assert_eq!(bbio_pts(10.0, 3.0, 1.5), 33.5);
    }

    #[test]
    fn besoins_et_bbio_calcules_a_la_main() {
        // Groupe de 100 m². Janvier : 500 kWh de chauffage, 20 kWh d'éclairage ; juillet :
        // 150 kWh de froid, 10 kWh d'éclairage.
        let mut e = EnergiesMensuelles::default();
        e.q_sys_ch_wh[0] = 500_000.0;
        e.c_ecl_kwh[0] = 20.0;
        e.q_sys_fr_wh[6] = 150_000.0;
        e.c_ecl_kwh[6] = 10.0;
        let s = sorties(&e, 100.0);
        assert_eq!(s.mensuel.len(), 12);
        // Janvier : B_Ch = 5 kWh/m², B_Ecl = 0,2 ; Bbio = 2 × 5 + 5 × 0,2 = 11 points.
        assert_eq!(s.mensuel[0].b_ch_kwh_m2, 5.0);
        assert_eq!(s.mensuel[0].b_ecl_kwh_m2, 0.2);
        assert_eq!(s.mensuel[0].bbio_pts, 11.0);
        // Juillet : B_Fr = 1,5 ; B_Ecl = 0,1 ; Bbio = 3 + 0,5 = 3,5 points.
        assert_eq!(s.mensuel[6].b_fr_kwh_m2, 1.5);
        assert_eq!(s.mensuel[6].bbio_pts, 3.5);
        assert_eq!(s.mensuel[3], BesoinsBbio::default());
        // Année : 5 ; 1,5 ; 0,3 ; Bbio = 14,5 points, somme des Bbio mensuels (éq. 2386) et
        // non douze fois la combinaison des besoins annuels (décision Q10).
        assert_eq!(s.annuel.b_ch_kwh_m2, 5.0);
        assert_eq!(s.annuel.b_fr_kwh_m2, 1.5);
        assert!((s.annuel.b_ecl_kwh_m2 - 0.3).abs() < 1e-15);
        assert_eq!(s.annuel.bbio_pts, 14.5);
        assert_eq!(
            s.annuel.bbio_pts,
            s.mensuel.iter().map(|m| m.bbio_pts).sum::<f64>()
        );
    }

    #[test]
    fn une_zone_somme_les_energies_et_non_les_besoins_par_m2() {
        // Deux groupes : 100 m² à 5 kWh/m² et 300 m² à 1 kWh/m² de chauffage en janvier.
        // Zone : (500 + 300) kWh / 400 m² = 2 kWh/m², et non 5 + 1.
        let mut a = EnergiesMensuelles::default();
        a.q_sys_ch_wh[0] = 500_000.0;
        let mut b = EnergiesMensuelles::default();
        b.q_sys_ch_wh[0] = 300_000.0;
        let mut zone = a;
        zone.ajouter(&b);
        assert_eq!(sorties(&zone, 400.0).annuel.b_ch_kwh_m2, 2.0);
        assert_eq!(sorties(&zone, 400.0).annuel.bbio_pts, 4.0);
        // Surface de référence nulle : besoins nuls, pas de division par zéro.
        assert_eq!(sorties(&zone, 0.0).annuel, BesoinsBbio::default());
    }
}
