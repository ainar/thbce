// SPDX-License-Identifier: GPL-3.0-or-later
//! Boucle solaire : fiche `C_GEN_Boucle solaire` (Th-BCE 2020 §9.15).
//!
//! La boucle transfère au ballon, par son échangeur en partie basse, l'énergie captée par les
//! capteurs solaires ; elle est vue par l'assemblage du ballon comme un générateur de base
//! (§9.10.3.2.2 : « dans le cas d'un générateur de base de type boucle solaire thermique, le
//! générateur est considéré comme enclenché dès qu'il peut réchauffer le ballon »).
//!
//! À chaque pas de temps : énergie que transférerait la boucle si sa pompe fonctionnait
//! (éq. 1963 à 1967), puis régulation sur l'écart de température entre la sortie des capteurs
//! et le ballon (éq. 1968 à 1970) ou sur l'ensoleillement (éq. 1971), consommation de la pompe
//! (éq. 1972) et pertes des tuyauteries vers le volume chauffé (éq. 1973). Le rayonnement
//! incident `Isr*` est celui de la fiche 3.2 sur le plan des capteurs, masques compris.
//!
//! Lectures : `docs/questions.md`, Q52.

use crate::bati::paroi_opaque::resoudre_masques;
use crate::climat::environnement_proche::{
    flux_atteignant_la_paroi, MasqueResolu, Soleil, SurfaceExposee,
};
use crate::climat::ClimatExterieur;
use crate::entree::{Catalogue, Exposition};

/// « (UA)hx » conventionnel : coefficient de l'échangeur, en W/K par m² de capteurs
/// (tableau 292 : `100·A`).
pub const UA_HX_CONV_W_K_M2: f64 = 100.0;
/// « ṁCp » conventionnel : débit calorifique de la boucle, en W/K par m² de capteurs
/// (tableau 292 : `58·A`).
pub const M_CP_CONV_W_K_M2: f64 = 58.0;
/// « αp » conventionnel : part de la puissance de la pompe transmise au fluide (tableau 292).
pub const ALPHA_P_CONV: f64 = 0.5;
/// « Isrmin » conventionnel, en W/m² (tableau 292).
pub const ISR_MIN_CONV_W_M2: f64 = 200.0;
/// « Δθreg » des installations desservant des maisons individuelles ou des logements
/// collectifs, en K (éq. 1968).
pub const DELTA_THETA_REG_HABITATION_K: f64 = 1.0;
/// « Δθreg » des installations desservant d'autres usages, en K (éq. 1968).
pub const DELTA_THETA_REG_AUTRES_K: f64 = 3.0;

/// Type de capteur pour les performances par défaut (tableau 293).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeCapteur {
    /// Capteur non vitré.
    NonVitre,
    /// Capteur plan vitré.
    Vitre,
    /// Capteur à tubes sous vide.
    Tubulaire,
}

/// « η0 », « a1 » (W/(m².K)) et « a2 » (W/(m².K²)) par défaut (tableau 293, relu sur l'image
/// de la p. 1167).
pub const fn performances_par_defaut(t: TypeCapteur) -> (f64, f64, f64) {
    match t {
        TypeCapteur::NonVitre => (0.6, 20.0, 0.0),
        TypeCapteur::Vitre => (0.6, 6.0, 0.0),
        TypeCapteur::Tubulaire => (0.6, 3.0, 0.0),
    }
}

/// « Typeregul_BS » : régulation de la boucle (§9.15.3.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegulationBoucle {
    /// 0 : sur l'écart de température entre la sortie des capteurs et le ballon, avec le
    /// réglage « Δθreg », en K (éq. 1969 et 1970).
    Temperature {
        /// « Δθreg », en K.
        delta_theta_reg_k: f64,
    },
    /// 1 : sur l'ensoleillement, avec le seuil « Isrmin », en W/m² (éq. 1971).
    Irradiance {
        /// « Isrmin », en W/m².
        isr_min_w_m2: f64,
    },
}

/// Paramètres d'une boucle solaire (tableau 292), pour un assemblage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoucleSolaire {
    /// « A » : superficie d'entrée des capteurs, en m².
    pub a_m2: f64,
    /// « η0 » : rendement optique.
    pub eta0: f64,
    /// « a1 », en W/(m².K).
    pub a1_w_m2k: f64,
    /// « a2 », en W/(m².K²).
    pub a2_w_m2k2: f64,
    /// « Kθ » : facteur d'angle d'incidence.
    pub k_theta: f64,
    /// « (UA)te » : pertes des tuyauteries vers l'extérieur, en W/K.
    pub ua_te_w_k: f64,
    /// « (UA)ti » : pertes des tuyauteries vers l'intérieur du bâtiment, en W/K.
    pub ua_ti_w_k: f64,
    /// « (UA)hx » : coefficient de l'échangeur, en W/K ; `None` sans échangeur (§9.15.3.1.2).
    pub ua_hx_w_k: Option<f64>,
    /// « Pnp » : puissance nominale de la pompe, en W.
    pub p_np_w: f64,
    /// « αp ».
    pub alpha_p: f64,
    /// « ṁCp » : débit calorifique de la boucle, en W/K.
    pub m_cp_w_k: f64,
    /// Régulation.
    pub regulation: RegulationBoucle,
}

/// Conditions d'un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntreesBoucle {
    /// « Isr* » : ensoleillement sur le plan des capteurs, masques compris, en W/m².
    pub isr_w_m2: f64,
    /// « θe(h) » : température extérieure, en °C.
    pub theta_e_c: f64,
    /// « θamb(h) » : température ambiante, en °C.
    pub theta_amb_c: f64,
    /// « θb_moy_ech(h) » : température du ballon vue par l'échangeur solaire, en °C.
    pub theta_b_moy_ech_c: f64,
}

/// Sorties d'un pas de temps, pour un assemblage.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SortiesBoucle {
    /// « Qsol » : chaleur transférée au ballon, en Wh.
    pub q_sol_wh: f64,
    /// « Pp » : consommation de la pompe sur le pas d'une heure, en Wh.
    pub p_p_wh: f64,
    /// « ΦvcBS » : pertes vers l'ambiance, en Wh (éq. 1973).
    pub phi_vc_wh: f64,
    /// Vrai si la pompe fonctionne.
    pub en_marche: bool,
    /// « Qsol^Int » : énergie qui serait transférée pompe en marche, en Wh.
    pub q_sol_int_wh: f64,
    /// « θm » : température moyenne de la boucle (ou du ballon sans échangeur), en °C.
    pub theta_comp_c: f64,
}

impl BoucleSolaire {
    /// Énergie transférée si la pompe fonctionne, `(Qsol^Int, θcomp)`, en Wh et en °C
    /// (éq. 1963 à 1967, relues sur l'image de la p. 1168).
    ///
    /// Avec échangeur, `θm` est la racine de l'équation du second degré (éq. 1966)
    /// `a2·θm² + [a1 + (UAte + UAti + UAhx)/A − 2·a2·θe]·θm + a2·θe² − a1·θe − η0·Isr*·Kθ
    /// − [αp·Pnp + UAte·θe + UAti·θamb + UAhx·θb]/A = 0`, puis `Qsol^Int = UAhx·(θm − θb)`
    /// (éq. 1965). Le premier membre croît avec `θm` au-delà de son sommet, situé sous `θe` :
    /// la racine retenue est la plus grande, seule à faire décroître les pertes du capteur
    /// quand `θm` baisse vers `θe` (lecture Q52). Sans échangeur, `θm = θb` (éq. 1967).
    pub fn q_sol_int_wh(&self, e: &EntreesBoucle) -> (f64, f64) {
        let a = self.a_m2;
        let (te, tb) = (e.theta_e_c, e.theta_b_moy_ech_c);
        let gains = self.eta0 * e.isr_w_m2 * self.k_theta;
        match self.ua_hx_w_k {
            Some(ua_hx) => {
                let a2 = self.a2_w_m2k2;
                let b =
                    self.a1_w_m2k + (self.ua_te_w_k + self.ua_ti_w_k + ua_hx) / a - 2.0 * a2 * te;
                let c = a2 * te * te
                    - self.a1_w_m2k * te
                    - gains
                    - (self.alpha_p * self.p_np_w
                        + self.ua_te_w_k * te
                        + self.ua_ti_w_k * e.theta_amb_c
                        + ua_hx * tb)
                        / a;
                let theta_m = if a2 > 0.0 {
                    (-b + libm::sqrt((b * b - 4.0 * a2 * c).max(0.0))) / (2.0 * a2)
                } else {
                    -c / b
                };
                (ua_hx * (theta_m - tb), theta_m)
            }
            None => {
                let ecart = tb - te;
                let q = gains * a
                    - a * (self.a1_w_m2k + self.a2_w_m2k2 * ecart) * ecart
                    - self.ua_te_w_k * ecart
                    - self.ua_ti_w_k * (tb - e.theta_amb_c)
                    + self.alpha_p * self.p_np_w;
                (q, tb)
            }
        }
    }

    /// Un pas de temps d'une heure (§9.15.3).
    pub fn pas(&self, e: &EntreesBoucle) -> SortiesBoucle {
        let (q_int, theta_comp) = self.q_sol_int_wh(e);
        let en_marche = match self.regulation {
            RegulationBoucle::Temperature { delta_theta_reg_k } => {
                // Éq. (1969) et (1970).
                let delta_theta_ce = (2.0 * q_int
                    + self.ua_te_w_k * (theta_comp - e.theta_e_c)
                    + self.ua_ti_w_k * (theta_comp - e.theta_amb_c)
                    - self.alpha_p * self.p_np_w)
                    / (2.0 * self.m_cp_w_k);
                delta_theta_ce >= delta_theta_reg_k
            }
            // Éq. (1971), qui remplace la condition (1970) (le texte écrit « la condition
            // (1972) remplace alors la condition (1971) »).
            RegulationBoucle::Irradiance { isr_min_w_m2 } => e.isr_w_m2 >= isr_min_w_m2,
        };
        SortiesBoucle {
            q_sol_wh: if en_marche { q_int } else { 0.0 },
            p_p_wh: if en_marche { self.p_np_w } else { 0.0 },
            // Éq. (1973) : pertes des tuyauteries vers l'intérieur. Comptées pompe en marche
            // seulement : pompe arrêtée, aucune énergie ne circule et `θcomp` n'est qu'une
            // température de calcul (lecture Q52).
            phi_vc_wh: if en_marche {
                (self.ua_ti_w_k * (theta_comp - e.theta_amb_c)).max(0.0)
            } else {
                0.0
            },
            en_marche,
            q_sol_int_wh: q_int,
            theta_comp_c: theta_comp,
        }
    }
}

/// Capteurs d'une boucle solaire exposés au rayonnement : surface et masques de la fiche 3.2.
#[derive(Debug, Clone, PartialEq)]
pub struct CapteursExposes {
    /// Orientation et inclinaison du plan des capteurs.
    pub surface: SurfaceExposee,
    /// Masques résolus.
    pub masques: Vec<MasqueResolu>,
}

impl CapteursExposes {
    /// Capteurs de l'exposition donnée, avec les masques du catalogue.
    pub fn nouveaux(exposition: &Exposition, catalogue: &Catalogue) -> Self {
        Self {
            surface: SurfaceExposee::depuis_exposition(exposition),
            masques: resoudre_masques(&exposition.masques, catalogue),
        }
    }

    /// « Isr* » : ensoleillement sur le plan des capteurs au pas `h`, masques compris, en
    /// W/m² : somme des composantes directe, diffuse et réfléchie par le sol de la fiche 3.2
    /// (§3.2.3), comme pour les modules photovoltaïques (§12.2).
    pub fn isr_w_m2(&self, climat: &ClimatExterieur, h: usize) -> f64 {
        let soleil = Soleil {
            gamma_rad: climat.gamma_rad[h],
            psi_rad: climat.psi_rad[h],
        };
        let f = flux_atteignant_la_paroi(
            &self.surface,
            &self.masques,
            &soleil,
            climat.idn_w_m2[h],
            climat.idi_w_m2[h],
            0.0,
            0.0,
            h,
        );
        f.drp_w_m2 + f.dfp_w_m2 + f.rrp_w_m2
    }
}

/// Boucle solaire d'un assemblage et ses capteurs, avec ses cumuls annuels.
#[derive(Debug, Clone, PartialEq)]
pub struct BoucleSolaireHoraire {
    /// Paramètres de la boucle.
    pub boucle: BoucleSolaire,
    /// Capteurs exposés.
    pub capteurs: CapteursExposes,
    /// Cumuls annuels, pour un assemblage.
    pub cumuls: CumulsBoucle,
}

/// Cumuls annuels d'une boucle solaire, pour un assemblage.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CumulsBoucle {
    /// « Esol_tot^BS » : énergie transmise à l'échangeur, en Wh, par mois des scénarios.
    pub e_sol_mensuel_wh: [f64; 12],
    /// « Eef_aux_tot^BS » : consommation de la pompe, en Wh.
    pub e_aux_wh: f64,
    /// Pertes des tuyauteries vers l'intérieur, en Wh.
    pub phi_vc_wh: f64,
    /// Ensoleillement annuel sur le plan des capteurs, en Wh/m².
    pub isr_wh_m2: f64,
    /// Heures de fonctionnement de la pompe.
    pub nbh_marche: u32,
}

impl CumulsBoucle {
    /// « Esol_tot^BS » annuelle, en Wh.
    pub fn e_sol_wh(&self) -> f64 {
        self.e_sol_mensuel_wh.iter().sum()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn boucle(ua_hx: Option<f64>, a2: f64) -> BoucleSolaire {
        BoucleSolaire {
            a_m2: 4.0,
            eta0: 0.8,
            a1_w_m2k: 4.0,
            a2_w_m2k2: a2,
            k_theta: 0.9,
            ua_te_w_k: 2.0,
            ua_ti_w_k: 1.0,
            ua_hx_w_k: ua_hx,
            p_np_w: 40.0,
            alpha_p: 0.5,
            m_cp_w_k: 232.0,
            regulation: RegulationBoucle::Temperature {
                delta_theta_reg_k: 1.0,
            },
        }
    }

    fn entrees(isr: f64, tb: f64) -> EntreesBoucle {
        EntreesBoucle {
            isr_w_m2: isr,
            theta_e_c: 10.0,
            theta_amb_c: 20.0,
            theta_b_moy_ech_c: tb,
        }
    }

    #[test]
    fn equilibre_avec_echangeur_lineaire() {
        // a2 = 0 : éq. (1966) linéaire. Bilan de la boucle (éq. 1964) :
        // 0,8 × 4 × 800 × 0,9 = 2 304 W captés ; pertes 4 × 4 × (θm − 10) + 2 × (θm − 10)
        // + 1 × (θm − 20) ; pompe 20 W ; transfert 400 × (θm − 30).
        // 2 304 + 20 + 180 + 20 + 12 000 = 419 θm, soit θm = 14 524/419.
        let b = boucle(Some(400.0), 0.0);
        let (q, tm) = b.q_sol_int_wh(&entrees(800.0, 30.0));
        let tm_main = 14_524.0 / 419.0;
        assert!((tm - tm_main).abs() < 1e-9, "{tm}");
        assert!((q - 400.0 * (tm_main - 30.0)).abs() < 1e-9);
        // Bilan de l'éq. (1964).
        let pertes = 16.0 * (tm - 10.0) + 2.0 * (tm - 10.0) + (tm - 20.0);
        assert!((2304.0 - pertes + 20.0 - q).abs() < 1e-9);
    }

    #[test]
    fn equilibre_avec_echangeur_du_second_degre() {
        // a2 > 0 : la racine retenue vérifie l'éq. (1964) avec Uc = a1 + a2·(θm − θe) + UAte/A
        // (éq. 1963).
        let b = boucle(Some(400.0), 0.015);
        let e = entrees(900.0, 45.0);
        let (q, tm) = b.q_sol_int_wh(&e);
        let uc = 4.0 + 0.015 * (tm - 10.0) + 2.0 / 4.0;
        let bilan = 0.8 * 4.0 * 900.0 * 0.9 - 4.0 * uc * (tm - 10.0) - (tm - 20.0) + 20.0;
        assert!((bilan - q).abs() < 1e-6, "{bilan} {q}");
        assert!(q > 0.0 && tm > 45.0);
    }

    #[test]
    fn sans_echangeur() {
        // Éq. (1967) : θm = θb = 30 °C ; 2 304 − 4 × (4 + 0) × 20 − 2 × 20 − 1 × 10 + 20.
        let b = boucle(None, 0.0);
        let (q, tm) = b.q_sol_int_wh(&entrees(800.0, 30.0));
        assert_eq!(tm, 30.0);
        assert!((q - (2304.0 - 320.0 - 40.0 - 10.0 + 20.0)).abs() < 1e-9);
    }

    #[test]
    fn regulation_en_temperature_et_en_irradiance() {
        let b = boucle(Some(400.0), 0.0);
        // Fort ensoleillement : la boucle tourne, la pompe consomme sa puissance nominale.
        let s = b.pas(&entrees(800.0, 30.0));
        assert!(s.en_marche && s.q_sol_wh > 0.0);
        assert_eq!(s.p_p_wh, 40.0);
        // Éq. (1969) : Δθce = [2·Qsol + 2·(θm − 10) + (θm − 20) − 20]/(2 × 232).
        let tm = s.theta_comp_c;
        let dtce = (2.0 * s.q_sol_wh + 2.0 * (tm - 10.0) + (tm - 20.0) - 20.0) / 464.0;
        assert!(dtce >= 1.0);
        // Éq. (1973).
        assert!((s.phi_vc_wh - (tm - 20.0)).abs() < 1e-9);
        // Ballon chaud et faible ensoleillement : Qsol^Int négatif, la pompe s'arrête.
        let s = b.pas(&entrees(100.0, 60.0));
        assert!(s.q_sol_int_wh < 0.0);
        assert!(!s.en_marche);
        assert_eq!((s.q_sol_wh, s.p_p_wh, s.phi_vc_wh), (0.0, 0.0, 0.0));
        // Régulation sur l'ensoleillement : seuil de 200 W/m².
        let bi = BoucleSolaire {
            regulation: RegulationBoucle::Irradiance {
                isr_min_w_m2: ISR_MIN_CONV_W_M2,
            },
            ..b
        };
        assert!(!bi.pas(&entrees(199.0, 30.0)).en_marche);
        assert!(bi.pas(&entrees(200.0, 30.0)).en_marche);
    }

    #[test]
    fn performances_par_defaut_du_tableau_293() {
        assert_eq!(
            performances_par_defaut(TypeCapteur::NonVitre),
            (0.6, 20.0, 0.0)
        );
        assert_eq!(performances_par_defaut(TypeCapteur::Vitre), (0.6, 6.0, 0.0));
        assert_eq!(
            performances_par_defaut(TypeCapteur::Tubulaire),
            (0.6, 3.0, 0.0)
        );
    }
}
