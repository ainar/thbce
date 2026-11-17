// SPDX-License-Identifier: GPL-3.0-or-later
//! Fiche `C_PV_onduleurs_PV` (Th-BCE 2020 §12.4) : onduleur.
//!
//! Aucune fonction du module n'alloue après la construction : `pas` est appelée dans la boucle
//! horaire.

use super::entree::{OnduleurPv, RendementOnduleur};

/// « ηONDBASE » : rendement électrique par défaut (tableau 330).
pub const ETA_OND_BASE: f64 = 0.9;
/// Rapport `PPV / PAC NOM` au-delà duquel l'onduleur s'éteint (« LIM », §12.4.3).
pub const SEUIL_LIM: f64 = 1.15;
/// Rapport de la puissance nominale par défaut à `N × Pc` (§12.4.3).
pub const RATIO_P_AC_NOM_DEFAUT: f64 = 0.8;
/// Points de passage obligés de la courbe de rendement (cas n° 3, CEI 61683).
pub const POINTS_OBLIGES: [f64; 6] = [0.05, 0.1, 0.25, 0.5, 0.75, 1.0];

/// « PAC NOM » par défaut : `0,8 × N × Pc`, avec `Pc` la valeur de calcul d'un module, en W.
///
/// Th-BCE 2020 §12.4.3.
pub fn p_ac_nom_defaut_w(n: f64, pc_calcul_w: f64) -> f64 {
    RATIO_P_AC_NOM_DEFAUT * n * pc_calcul_w
}

/// Onduleur prêt pour le calcul horaire.
#[derive(Debug, Clone, PartialEq)]
pub struct OnduleurResolu {
    /// « PAC NOM », en W.
    pub p_ac_nom_w: f64,
    /// Vrai si `PAC NOM` est la valeur par défaut.
    pub p_ac_nom_par_defaut: bool,
    /// Points `(X, η)` de la courbe de rendement, par abscisse croissante, `(0 ; 0)` compris.
    courbe: Vec<(f64, f64)>,
}

/// Sorties d'un pas de temps de l'onduleur.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SortiesPasOnduleur {
    /// « ηOND » au point de fonctionnement.
    pub eta_ond: f64,
    /// Vrai si l'onduleur est éteint (« LIM = 0 »).
    pub eteint: bool,
    /// « Pond » : puissance délivrée au réseau, en W.
    pub p_ond_w: f64,
}

impl OnduleurResolu {
    /// Résout un onduleur. `n` et `pc_calcul_w` servent à la puissance nominale par défaut.
    pub fn nouveau(o: &OnduleurPv, n: f64, pc_calcul_w: f64) -> Self {
        let (p_ac_nom_w, defaut) = match o.p_ac_nom_w {
            Some(p) => (p, false),
            None => (p_ac_nom_defaut_w(n, pc_calcul_w), true),
        };
        // Cas n° 1 et n° 2 : deux segments de droite, `η(0) = 0`, `η(0,2) = η`, `η(1) = η`.
        let deux_segments = |eta: f64| vec![(0.0, 0.0), (0.2, eta), (1.0, eta)];
        let courbe = match &o.rendement {
            RendementOnduleur::Defaut => deux_segments(ETA_OND_BASE),
            RendementOnduleur::Europeen { eta_eu } => deux_segments(*eta_eu),
            // Cas n° 3 : `η(0) = 0` puis les points saisis.
            RendementOnduleur::Courbe { points } => {
                let mut c: Vec<(f64, f64)> = points.iter().map(|p| (p.x, p.eta)).collect();
                c.sort_by(|a, b| a.0.total_cmp(&b.0));
                c.insert(0, (0.0, 0.0));
                c
            }
        };
        Self {
            p_ac_nom_w,
            p_ac_nom_par_defaut: defaut,
            courbe,
        }
    }

    /// « ηOND(x) » : interpolation linéaire entre les points de la courbe, extrapolation linéaire
    /// au-delà du dernier (§12.4.3, « inter et extrapolation linéaire »). Bornée à `[0 ; 1]`
    /// (décision Q51 point 4).
    pub fn eta_ond(&self, x: f64) -> f64 {
        let c = &self.courbe;
        if c.len() < 2 {
            return 0.0;
        }
        // Segment qui encadre x, le dernier segment prolongé au-delà de son extrémité.
        let i = c
            .windows(2)
            .position(|s| x <= s[1].0)
            .unwrap_or(c.len() - 2);
        let ((x0, y0), (x1, y1)) = (c[i], c[i + 1]);
        let eta = if x1 > x0 {
            y0 + (y1 - y0) * (x - x0) / (x1 - x0)
        } else {
            y1
        };
        eta.clamp(0.0, 1.0)
    }

    /// Un pas de temps.
    ///
    /// Th-BCE 2020 §12.4.3, éq. (2374) : `Pond = PPV · ηOND(PPV/PAC NOM) · LIM(PPV/PAC NOM)` ;
    /// `LIM = 0` si `PPV > 1,15 · PAC NOM`, sinon 1.
    pub fn pas(&self, p_pv_w: f64) -> SortiesPasOnduleur {
        if p_pv_w <= 0.0 {
            return SortiesPasOnduleur::default();
        }
        let eteint = p_pv_w > SEUIL_LIM * self.p_ac_nom_w;
        let eta_ond = self.eta_ond(p_pv_w / self.p_ac_nom_w);
        SortiesPasOnduleur {
            eta_ond,
            eteint,
            p_ond_w: if eteint { 0.0 } else { p_pv_w * eta_ond },
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::pv::entree::PointRendementOnduleur;

    fn onduleur(r: RendementOnduleur) -> OnduleurResolu {
        OnduleurResolu::nouveau(
            &OnduleurPv {
                p_ac_nom_w: Some(1000.0),
                rendement: r,
            },
            1.0,
            1000.0,
        )
    }

    #[test]
    fn cas_1_deux_segments() {
        let o = onduleur(RendementOnduleur::Defaut);
        assert_eq!(o.eta_ond(0.0), 0.0);
        assert!((o.eta_ond(0.1) - 0.45).abs() < 1e-12);
        assert_eq!(o.eta_ond(0.2), 0.9);
        assert_eq!(o.eta_ond(1.0), 0.9);
        assert_eq!(o.eta_ond(1.1), 0.9);
    }

    #[test]
    fn cas_2_rendement_europeen() {
        let o = onduleur(RendementOnduleur::Europeen { eta_eu: 0.96 });
        assert_eq!(o.eta_ond(0.5), 0.96);
        assert!((o.pas(500.0).p_ond_w - 480.0).abs() < 1e-9);
    }

    #[test]
    fn cas_3_courbe_interpolee_et_extrapolee() {
        let pts = [
            (0.05, 0.80),
            (0.1, 0.90),
            (0.25, 0.95),
            (0.5, 0.97),
            (0.75, 0.97),
            (1.0, 0.96),
        ]
        .map(|(x, eta)| PointRendementOnduleur { x, eta });
        let o = onduleur(RendementOnduleur::Courbe {
            points: pts.to_vec(),
        });
        assert!((o.eta_ond(0.025) - 0.40).abs() < 1e-12);
        assert!((o.eta_ond(0.175) - 0.925).abs() < 1e-12);
        // Extrapolation du dernier segment (pente −0,04) : 0,96 − 0,04 × 0,1.
        assert!((o.eta_ond(1.1) - 0.956).abs() < 1e-12);
    }

    #[test]
    fn extinction_au_dela_de_115_pour_cent() {
        let o = onduleur(RendementOnduleur::Defaut);
        assert_eq!(o.pas(1150.0).p_ond_w, 1035.0);
        let s = o.pas(1150.1);
        assert!(s.eteint);
        assert_eq!(s.p_ond_w, 0.0);
    }

    #[test]
    fn puissance_nominale_par_defaut() {
        let o = OnduleurResolu::nouveau(&OnduleurPv::default(), 10.0, 300.0);
        assert!(o.p_ac_nom_par_defaut);
        assert_eq!(o.p_ac_nom_w, 2400.0);
    }
}
