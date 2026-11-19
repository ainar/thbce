// SPDX-License-Identifier: GPL-3.0-or-later
//! Réseaux de distribution de CTA hydrauliques : fiches `C-Dist-Gestion/régulation des réseaux de
//! distribution des CTA` (Th-BCE 2020 §8.11, `idtype = 1`) et `C-Dist-Distributions intergroupes
//! des CTA` (§8.12).
//!
//! « Les processus de calculs sont exactement similaires à ceux de la fiche gestion/régulation de
//! la distribution du groupe » (§8.11.3.6) et « Voir chapitre identique de la fiche C_Dist_Réseau
//! de distribution du groupe » (§8.12.2) : un réseau de CTA hydraulique suit la procédure d'un
//! réseau du groupe (§8.7.3.3.2, §8.7.3.3.3, §8.8.3.2 et §8.8.3.3, éq. 854 à 887) sur la
//! demande des batteries de la centrale (éq. 939 et 940) ; comme il peut desservir plusieurs
//! groupes, ses pertes vers le volume chauffé sont calculées sur les températures d'ambiance
//! conventionnelles `θvc_ch` (20 °C) et `θvc_fr` (26 °C) (note du §8.12.2). Lectures :
//! `docs/questions/Q121.md`.

use crate::entree::{FonctionReseau, RegulationTemperature, ReseauHydrauliqueGroupe};

use super::distribution::{
    pas_hydraulique_groupe, SortiesDistributionGroupe, THETA_AMB_CH_C, THETA_AMB_FR_C,
};

/// Réseau de CTA hydraulique d'une zone, prêt pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct ReseauCtaResolu {
    /// Indice de la zone dont la centrale est desservie.
    pub zone: usize,
    /// « idfonction » : chauffage ou refroidissement.
    pub fonction: FonctionReseau,
    /// Indice de la génération qui alimente le réseau.
    pub generation: usize,
    /// Paramètres du réseau hydraulique.
    pub reseau: ReseauHydrauliqueGroupe,
    /// Indice de l'espace tampon traversé par la partie hors volume chauffé.
    pub espace_tampon: Option<usize>,
}

impl ReseauCtaResolu {
    /// Pas de temps : `q_em_wh` est la demande de la centrale sur le réseau (≥ 0 en chauffage,
    /// ≤ 0 en refroidissement ; somme des éq. 939 ou 940) ; `relance` : « irelance » (éq. 938).
    pub fn pas(
        &self,
        q_em_wh: f64,
        relance: bool,
        theta_ext_c: f64,
        theta_ext_base_c: f64,
        b_therm: f64,
    ) -> SortiesDistributionGroupe {
        let theta_vc_c = match self.fonction {
            FonctionReseau::Chauffage => THETA_AMB_CH_C,
            FonctionReseau::Refroidissement => THETA_AMB_FR_C,
        };
        pas_hydraulique_groupe(
            &self.reseau,
            self.fonction,
            q_em_wh,
            theta_vc_c,
            theta_ext_c,
            theta_ext_base_c,
            b_therm,
            relance,
        )
    }

    /// Température extrême vue par la génération dans les conditions nominales, en °C : `θmax_ch`
    /// (éq. 953) ou `θmin_fr` (éq. 956) ; pour une température de retour constante, le départ
    /// nominal vaut `θret_dim + Δθem_dim`, sinon `θdep_dim`.
    pub fn theta_extreme_nominale_c(&self) -> Option<f64> {
        let r = &self.reseau;
        match r.regulation_temperature {
            RegulationTemperature::RetourConstant => {
                r.theta_ret_dim_c.map(|t| t + r.delta_theta_em_dim_k)
            }
            _ => r.theta_dep_dim_c,
        }
    }
}

/// Cumuls annuels d'un réseau de CTA hydraulique.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CumulsReseauCta {
    /// Énergie transmise à la génération, pertes comprises, en Wh (valeur absolue).
    pub q_wh: f64,
    /// « Waux » : consommation du circulateur, en Wh.
    pub w_aux_wh: f64,
    /// Pertes vers le volume chauffé, en Wh.
    pub phi_pertes_vc_wh: f64,
    /// Pertes hors volume chauffé, en Wh.
    pub phi_pertes_hvc_wh: f64,
    /// Nombre d'heures où le réseau débite.
    pub nbh_fonct: u32,
}

impl CumulsReseauCta {
    /// Ajoute un pas de temps.
    pub fn ajouter(&mut self, s: &SortiesDistributionGroupe) {
        self.q_wh += s.q_sys_ch_wh.abs() + s.q_sys_fr_wh.abs();
        self.w_aux_wh += s.w_aux_wh;
        self.phi_pertes_vc_wh += s.phi_pertes_vc_wh;
        self.phi_pertes_hvc_wh += s.phi_pertes_hvc_wh;
        self.nbh_fonct += u32::from(s.fonct);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]
    use super::*;
    use crate::entree::{Circulateur, RegulationDebit};

    fn reseau(fonction: FonctionReseau) -> ReseauCtaResolu {
        ReseauCtaResolu {
            zone: 0,
            fonction,
            generation: 0,
            reseau: ReseauHydrauliqueGroupe {
                regulation_debit: RegulationDebit::DebitConstantContinu,
                regulation_temperature: RegulationTemperature::DepartConstant,
                theta_dep_dim_c: Some(if fonction == FonctionReseau::Chauffage {
                    60.0
                } else {
                    7.0
                }),
                theta_ret_dim_c: None,
                delta_theta_em_dim_k: if fonction == FonctionReseau::Chauffage {
                    10.0
                } else {
                    -5.0
                },
                q_nom_m3_h: 2.0,
                q_resid_m3_h: None,
                circulateur: Circulateur::VitesseConstante,
                p_aux_w: 100.0,
                l_vc_m: 10.0,
                l_hvc_m: 5.0,
                u_vc_w_mk: 0.2,
                u_hvc_w_mk: 0.3,
                espace_tampon: None,
            },
            espace_tampon: None,
        }
    }

    /// Chauffage, débit constant continu, départ constant à 60 °C, 10 000 Wh demandés :
    /// `Δθem = 10 000 / (998 × 1,163 × 2) = 4,3078 K`, `θret = 55,6922`, `θmoy = 57,8461` ;
    /// pertes en volume chauffé `0,2 × 10 × (57,8461 − 20) = 75,692` ; hors volume chauffé
    /// (extérieur à 0 °C, `btherm = 1`) `0,3 × 5 × 57,8461 = 86,769` ; circulateur 100 Wh dont
    /// la moitié est transmise à l'ambiance.
    #[test]
    fn chauffage_a_la_main() {
        let r = reseau(FonctionReseau::Chauffage);
        let s = r.pas(10_000.0, false, 0.0, -7.0, 1.0);
        let rho_cp = 998.0 * 1.163;
        let dt = 10_000.0 / (rho_cp * 2.0);
        let moy = 60.0 - dt / 2.0;
        assert!((dt - 4.3078_f64).abs() < 1e-3);
        assert!((s.theta_ret_c - (60.0 - dt)).abs() < 1e-9);
        assert!((s.phi_pertes_vc_wh - 0.2 * 10.0 * (moy - 20.0)).abs() < 1e-9);
        assert!((s.phi_pertes_vc_wh - 75.692).abs() < 1e-2);
        assert!((s.phi_pertes_hvc_wh - 0.3 * 5.0 * moy).abs() < 1e-9);
        assert!((s.phi_pertes_hvc_wh - 86.769).abs() < 1e-2);
        assert!(
            (s.q_sys_ch_wh - (10_000.0 + s.phi_pertes_vc_wh + s.phi_pertes_hvc_wh)).abs() < 1e-9
        );
        assert_eq!(s.q_sys_fr_wh, 0.0);
        assert!((s.w_aux_wh - 100.0).abs() < 1e-9);
        assert!((s.phi_aux_vc_wh - 50.0).abs() < 1e-9);
    }

    /// Refroidissement : demande de −5 000 Wh, départ constant à 7 °C ; `Δθem = −5 000 /
    /// (998 × 1,163 × 2) = −2,1539 K`, `θret = 9,1539`, `θmoy = 8,0770` ; pertes en volume
    /// refroidi `0,2 × 10 × MIN(0 ; 8,0770 − 26) = −35,846`.
    #[test]
    fn refroidissement_a_la_main() {
        let r = reseau(FonctionReseau::Refroidissement);
        let s = r.pas(-5_000.0, false, 30.0, -7.0, 1.0);
        let dt = -5_000.0 / (998.0 * 1.163 * 2.0);
        let moy = 7.0 - dt / 2.0;
        assert!((moy - 8.0770_f64).abs() < 1e-3);
        assert!((s.phi_pertes_vc_wh - 0.2 * 10.0 * (moy - 26.0)).abs() < 1e-9);
        assert!((s.phi_pertes_vc_wh + 35.846).abs() < 1e-2);
        assert!(s.q_sys_fr_wh < -5_000.0, "les pertes s'ajoutent au froid");
        assert_eq!(s.q_sys_ch_wh, 0.0);
    }

    #[test]
    fn sans_demande_pas_de_perte() {
        let r = reseau(FonctionReseau::Chauffage);
        let s = r.pas(0.0, false, 0.0, -7.0, 1.0);
        assert_eq!(s.q_sys_ch_wh, 0.0);
        assert_eq!(s.w_aux_wh, 0.0);
        assert!(!s.fonct);
    }

    #[test]
    fn temperature_extreme_nominale() {
        let mut r = reseau(FonctionReseau::Chauffage);
        assert_eq!(r.theta_extreme_nominale_c(), Some(60.0));
        // Éq. (953) : retour constant, θmax = θret_dim + Δθem_dim = 45 + 10.
        r.reseau.regulation_temperature = RegulationTemperature::RetourConstant;
        r.reseau.theta_ret_dim_c = Some(45.0);
        assert_eq!(r.theta_extreme_nominale_c(), Some(55.0));
    }
}
