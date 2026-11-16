// SPDX-License-Identifier: GPL-3.0-or-later
//! Ballon de stockage : fiche `C_STO_Ballon_de_stockage` (Th-BCE 2020 §9.9).
//!
//! Le ballon est décrit par quatre zones superposées (§9.9.3.1). À chaque pas de temps, ses
//! pertes sont calculées avec les températures de la fin du pas précédent (éq. 1770), le puisage
//! fait monter l'eau par effet piston (éq. 1773 et 1776), les générateurs injectent leur énergie
//! dans la zone de leur échangeur (éq. 1786) et les zones se mélangent en cas d'inversion de
//! température (éq. 1778 et 1779).
//!
//! Périmètre : ballon d'ECS à puisage direct, générateurs raccordés par un échangeur ou une
//! résistance en partie basse (`TypeRaccordement_Base_Ballon = 0`). Les piquages haut et bas
//! (éq. 1787 à 1789), l'échangeur interne côté distribution (éq. 1777) et le puisage pour le
//! chauffage sont hors de cette version. Lectures : `docs/questions.md`, Q48.

/// « ρw × cw » : capacité thermique volumique de l'eau, en Wh/(L.K) (tableau 281).
pub const RHO_CW_WH_L_K: f64 = 1.163;
/// « Nzone » : nombre de zones du ballon (tableau 281).
pub const NB_ZONES: usize = 4;
/// Température initiale des zones, en °C (§9.9.3.2.1, note de l'éq. 1771).
pub const THETA_INITIALE_BALLON_C: f64 = 50.0;
/// « faux » par défaut (§9.9.3.1, « statut_faux = 2 »).
pub const F_AUX_DEFAUT: f64 = 0.5;

/// Nature d'un ballon pour ses pertes par défaut (tableau 282).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatureBallon {
    /// Ballon d'ECS à effet Joule horizontal.
    EffetJouleHorizontal,
    /// Ballon d'ECS à effet Joule vertical.
    EffetJouleVertical,
    /// Autres ballons.
    Autre,
}

/// « Qpr » : pertes par défaut d'un ballon de volume `v_tot_l`, en kWh par jour (tableau 282,
/// relu sur l'image de la p. 1091).
pub fn q_pr_defaut_kwh_j(nature: NatureBallon, v_tot_l: f64) -> f64 {
    match nature {
        NatureBallon::EffetJouleHorizontal => 0.939 + 0.0104 * v_tot_l,
        NatureBallon::EffetJouleVertical if v_tot_l >= 75.0 => {
            0.224 + 0.0663 * libm::pow(v_tot_l, 2.0 / 3.0)
        }
        NatureBallon::EffetJouleVertical => 0.1474 + 0.0719 * libm::pow(v_tot_l, 2.0 / 3.0),
        NatureBallon::Autre => 0.189 * libm::pow(v_tot_l, 0.55),
    }
}

/// « UAs_util » par défaut, en W/K (éq. 1768) : `Qpr × 1000 / (45 × 24)`.
pub fn ua_s_defaut_w_k(nature: NatureBallon, v_tot_l: f64) -> f64 {
    q_pr_defaut_kwh_j(nature, v_tot_l) * 1000.0 / (45.0 * 24.0)
}

/// Ballon de stockage à quatre zones ; la zone d'indice 0 est la plus basse (zone n° 1 du
/// texte).
#[derive(Debug, Clone, PartialEq)]
pub struct Ballon {
    /// « Vz » : volume de chaque zone, en L.
    pub v_l: [f64; NB_ZONES],
    /// « Uz » : coefficient de pertes de chaque zone, en W/K (éq. 1769).
    pub u_w_k: [f64; NB_ZONES],
    /// « θmax » : température maximale du ballon, en °C.
    pub theta_max_c: f64,
    /// « θbz » : températures courantes des zones, en °C.
    pub theta_c: [f64; NB_ZONES],
    /// « θbz(h − 1) ».
    pub theta_prec_c: [f64; NB_ZONES],
    /// « θbz(h − 2) ».
    pub theta_prec2_c: [f64; NB_ZONES],
    /// « Φpertes,z(h) » : pertes du pas de temps, en Wh (éq. 1770).
    pub pertes_wh: [f64; NB_ZONES],
    /// « Φpertes,z(h − 1) ».
    pub pertes_prec_wh: [f64; NB_ZONES],
}

impl Ballon {
    /// Ballon de volume `v_tot_l` et de coefficient de pertes `ua_s_w_k` (après correction de
    /// statut). `f_aux` : fraction du volume chauffée par un appoint intégré, `None` pour un
    /// ballon à une seule puissance injectée (§9.9.3.1).
    pub fn nouveau(v_tot_l: f64, ua_s_w_k: f64, f_aux: Option<f64>, theta_max_c: f64) -> Self {
        let v_l = match f_aux {
            None => [v_tot_l / NB_ZONES as f64; NB_ZONES],
            Some(f) => {
                let bas = (1.0 - f) * v_tot_l / 2.0;
                let haut = f * v_tot_l / 2.0;
                [bas, bas, haut, haut]
            }
        };
        Self {
            v_l,
            // Éq. (1769).
            u_w_k: v_l.map(|v| ua_s_w_k * v / v_tot_l),
            theta_max_c,
            theta_c: [THETA_INITIALE_BALLON_C; NB_ZONES],
            theta_prec_c: [THETA_INITIALE_BALLON_C; NB_ZONES],
            theta_prec2_c: [THETA_INITIALE_BALLON_C; NB_ZONES],
            pertes_wh: [0.0; NB_ZONES],
            pertes_prec_wh: [0.0; NB_ZONES],
        }
    }

    /// Fixe la température de toutes les zones, et leur historique.
    pub fn fixer_temperature(&mut self, theta_c: f64) {
        self.theta_c = [theta_c; NB_ZONES];
        self.theta_prec_c = self.theta_c;
        self.theta_prec2_c = self.theta_c;
    }

    /// « Vtot », en L.
    pub fn v_tot_l(&self) -> f64 {
        self.v_l.iter().sum()
    }

    /// « Vz,min » : volume de la plus petite zone, en L.
    pub fn v_z_min_l(&self) -> f64 {
        self.v_l.iter().copied().fold(f64::INFINITY, f64::min)
    }

    /// Début d'un pas de temps : mémorise les températures des pas précédents et calcule les
    /// pertes des zones vers l'ambiance (éq. 1770).
    pub fn debut_pas(&mut self, theta_amb_c: f64) {
        self.theta_prec2_c = self.theta_prec_c;
        self.theta_prec_c = self.theta_c;
        self.pertes_prec_wh = self.pertes_wh;
        for z in 0..NB_ZONES {
            self.pertes_wh[z] = self.u_w_k[z] * (self.theta_prec_c[z] - theta_amb_c);
        }
    }

    /// « Φpertes(h) » : pertes du ballon au pas de temps, en Wh (éq. 1771).
    pub fn pertes_totales_wh(&self) -> f64 {
        self.pertes_wh.iter().sum()
    }

    /// Puisage d'ECS de `v_p_l` litres par effet piston : prélèvement dans la zone supérieure,
    /// entrée d'eau à `theta_entrant_c` dans la zone basse (éq. 1772, 1773 et 1776), puis
    /// mélange des zones (§9.9.3.2.2.2).
    pub fn puiser(&mut self, v_p_l: f64, theta_entrant_c: f64) {
        let avant = self.theta_c;
        self.theta_c[0] =
            (avant[0] * (self.v_l[0] - v_p_l) + theta_entrant_c * v_p_l) / self.v_l[0];
        for z in 1..NB_ZONES {
            self.theta_c[z] =
                (avant[z] * (self.v_l[z] - v_p_l) + avant[z - 1] * v_p_l) / self.v_l[z];
        }
        self.melanger();
    }

    /// Mélange des zones en cas d'inversion de température (éq. 1778), puis limitation à la
    /// température maximale du ballon (éq. 1779).
    pub fn melanger(&mut self) {
        let mut a = 0;
        while a + 1 < NB_ZONES {
            if self.theta_c[a] > self.theta_c[a + 1] {
                // Mélange de la zone `a` avec les zones supérieures, tant que le mélange reste
                // plus chaud que la zone suivante.
                let mut fin = a + 1;
                let moyenne = |fin: usize, t: &[f64; NB_ZONES], v: &[f64; NB_ZONES]| {
                    let vol: f64 = v[a..=fin].iter().sum();
                    (a..=fin).map(|j| v[j] * t[j]).sum::<f64>() / vol
                };
                let mut m = moyenne(fin, &self.theta_c, &self.v_l);
                while fin + 1 < NB_ZONES && m > self.theta_c[fin + 1] {
                    fin += 1;
                    m = moyenne(fin, &self.theta_c, &self.v_l);
                }
                self.theta_c[a..=fin].fill(m);
                a = 0;
            } else {
                a += 1;
            }
        }
        for t in &mut self.theta_c {
            *t = t.min(self.theta_max_c);
        }
    }

    /// « θb_moy_ech(h) » : température moyenne du ballon vue par l'échangeur d'un générateur
    /// placé à partir du bas de la zone d'indice `z_ech`, sur la hauteur relative `h_rel_ech`
    /// (éq. 1781 à 1785). Les températures sont les moyennes entre la fin du pas précédent et
    /// l'état après puisage (éq. 1785).
    pub fn theta_b_moy_ech_c(&self, z_ech: usize, h_rel_ech: f64) -> f64 {
        let moy = |z: usize| (self.theta_prec_c[z] + self.theta_c[z]) / 2.0;
        if h_rel_ech <= 0.0 {
            // Éq. (1783).
            return moy(z_ech);
        }
        // Éq. (1782) et (1784) : zones traversées, pondérées par leurs hauteurs relatives.
        let v_tot = self.v_tot_l();
        let mut reste = h_rel_ech;
        let mut somme = 0.0;
        for (z, v) in self.v_l.iter().enumerate().skip(z_ech) {
            let h = (v / v_tot).min(reste);
            somme += h * moy(z);
            reste -= h;
            if reste <= 0.0 {
                break;
            }
        }
        somme / (h_rel_ech - reste.max(0.0))
    }

    /// Injection de `q_inj_wh` dans la zone d'indice `z` par un échangeur intégré, avec ou sans
    /// les pertes du pas de temps (éq. 1786 et sa note), puis mélange des zones.
    pub fn injecter(&mut self, z: usize, q_inj_wh: f64, avec_pertes: bool) {
        for k in 0..NB_ZONES {
            let q = if k == z { q_inj_wh } else { 0.0 }
                - if avec_pertes { self.pertes_wh[k] } else { 0.0 };
            self.theta_c[k] += q / (RHO_CW_WH_L_K * self.v_l[k]);
        }
        self.melanger();
    }

    /// Énergie contenue dans le ballon par rapport à une température de référence, en Wh.
    pub fn energie_wh(&self, theta_ref_c: f64) -> f64 {
        (0..NB_ZONES)
            .map(|z| RHO_CW_WH_L_K * self.v_l[z] * (self.theta_c[z] - theta_ref_c))
            .sum()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn proche(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} ≠ {b}");
    }

    #[test]
    fn pertes_par_defaut() {
        // Tableau 282 et éq. (1768) : ballon vertical de 200 L,
        // Qpr = 0,224 + 0,0663 × 200^(2/3) = 2,4914 kWh/j ; UA = 2,307 W/K.
        let q = q_pr_defaut_kwh_j(NatureBallon::EffetJouleVertical, 200.0);
        proche(q, 0.224 + 0.0663 * libm::pow(200.0, 2.0 / 3.0));
        assert!((q - 2.4914).abs() < 1e-3);
        proche(
            ua_s_defaut_w_k(NatureBallon::EffetJouleVertical, 200.0),
            q * 1000.0 / 1080.0,
        );
        proche(
            q_pr_defaut_kwh_j(NatureBallon::EffetJouleVertical, 50.0),
            0.1474 + 0.0719 * libm::pow(50.0, 2.0 / 3.0),
        );
        proche(
            q_pr_defaut_kwh_j(NatureBallon::EffetJouleHorizontal, 200.0),
            3.019,
        );
        proche(
            q_pr_defaut_kwh_j(NatureBallon::Autre, 300.0),
            0.189 * libm::pow(300.0, 0.55),
        );
    }

    #[test]
    fn zones_et_pertes() {
        let b = Ballon::nouveau(200.0, 2.0, None, 90.0);
        assert_eq!(b.v_l, [50.0; 4]);
        assert_eq!(b.u_w_k, [0.5; 4]);
        // Appoint sur 40 % du volume : deux zones hautes de 40 L, deux zones basses de 60 L.
        let mut b = Ballon::nouveau(200.0, 2.0, Some(0.4), 90.0);
        assert_eq!(b.v_l, [60.0, 60.0, 40.0, 40.0]);
        proche(b.v_z_min_l(), 40.0);
        // Éq. (1770) : à 50 °C dans une ambiance à 20 °C, 2 × 30 = 60 W.
        b.debut_pas(20.0);
        proche(b.pertes_totales_wh(), 60.0);
        proche(b.pertes_wh[0], 0.6 * 30.0);
    }

    #[test]
    fn puisage_par_effet_piston() {
        let mut b = Ballon::nouveau(200.0, 0.0, None, 90.0);
        b.theta_c = [20.0, 40.0, 50.0, 60.0];
        // 25 L puisés, eau froide à 10 °C : chaque zone reçoit 25 L de la zone du dessous.
        b.puiser(25.0, 10.0);
        proche(b.theta_c[0], 15.0);
        proche(b.theta_c[1], 30.0);
        proche(b.theta_c[2], 45.0);
        proche(b.theta_c[3], 55.0);
    }

    #[test]
    fn injection_et_melange() {
        let mut b = Ballon::nouveau(200.0, 0.0, None, 90.0);
        b.theta_c = [10.0, 10.0, 50.0, 50.0];
        // 1 163 Wh en zone basse : + 20 K sur 50 L, soit 30 °C, plus chaud que la zone 2 :
        // mélange des deux zones à 20 °C, sous la zone 3.
        b.injecter(0, 1163.0, false);
        proche(b.theta_c[0], 20.0);
        proche(b.theta_c[1], 20.0);
        proche(b.theta_c[2], 50.0);
        // Assez pour dépasser les zones hautes : tout le ballon se mélange.
        b.theta_c = [10.0, 10.0, 50.0, 50.0];
        b.injecter(0, 1.163 * 50.0 * 100.0, false);
        for t in b.theta_c {
            proche(t, 55.0);
        }
        // Limitation à la température maximale (éq. 1779).
        b.injecter(0, 1.163 * 200.0 * 100.0, false);
        assert_eq!(b.theta_c, [90.0; 4]);
        // Pertes appliquées avec l'injection (éq. 1786).
        let mut b = Ballon::nouveau(200.0, 2.0, None, 90.0);
        b.debut_pas(20.0);
        b.injecter(0, 0.0, true);
        proche(b.theta_c[3], 50.0 - 15.0 / (1.163 * 50.0));
        proche(b.energie_wh(50.0), -60.0);
    }

    #[test]
    fn temperature_vue_par_l_echangeur() {
        let mut b = Ballon::nouveau(200.0, 0.0, None, 90.0);
        b.theta_prec_c = [20.0, 30.0, 40.0, 50.0];
        b.theta_c = [10.0, 30.0, 40.0, 50.0];
        // Échangeur ponctuel en zone basse : moyenne entre la fin du pas précédent et l'état
        // après puisage (éq. 1783 et 1785).
        proche(b.theta_b_moy_ech_c(0, 0.0), 15.0);
        // Échangeur sur 37,5 % de la hauteur : zone 1 entière (25 %) et la moitié de la zone 2.
        proche(
            b.theta_b_moy_ech_c(0, 0.375),
            (0.25 * 15.0 + 0.125 * 30.0) / 0.375,
        );
        // Appoint en zone 3 sur toute la partie haute.
        proche(b.theta_b_moy_ech_c(2, 0.5), 45.0);
    }
}
