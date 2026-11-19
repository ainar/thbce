// SPDX-License-Identifier: GPL-3.0-or-later
//! Puits climatique ou canadien (Th-BCE 2020 §6.8, §6.10 et §6.11).
//!
//! Le puits préchauffe ou prérafraîchit l'air neuf d'un simple flux par insufflation, d'un
//! double flux ou d'une centrale de traitement d'air. À chaque pas, séquentiellement (§6.8.1) :
//! température du sol au niveau des conduits (éq. 719 à 721), efficacité de l'échange sol-air
//! (éq. 723 à 727), température de sortie si le puits n'est pas contourné (éq. 728), puis
//! gestion du bypass (§6.10, éq. 750 à 752) ; l'air neuf prend la température de sortie
//! (éq. 731 : `θext` si le puits est contourné). Débit et humidité sont inchangés (§6.8.3.3.1).
//! Lectures : `docs/questions/Q231.md`.

use std::f64::consts::PI;

use crate::climat::calendrier::date_scenario;
use crate::entree::{BypassPuits, PuitsClimatique, TypeSolPuits};

/// « ρair » : masse volumique de l'air du puits, en kg/m³ (tableau 69).
pub const RHO_AIR_KG_M3: f64 = 1.22;
/// « Cpair » : capacité calorifique massique de l'air du puits, en J/(kg.K) (tableau 69).
pub const CP_AIR_J_KG_K: f64 = 1008.0;

/// « gm » : coefficient de correction de la température du sol selon sa nature
/// (§6.8.3.2.1, tableau 70).
pub fn gm(type_sol: TypeSolPuits) -> f64 {
    match type_sol {
        TypeSolPuits::SolHumide => 1.00,
        TypeSolPuits::SableSec => 0.90,
        TypeSolPuits::SableHumide => 0.98,
        TypeSolPuits::ArgileHumide => 1.04,
        TypeSolPuits::ArgileMouillee => 1.05,
    }
}

/// « AH » : facteur de correction de l'amplitude de la température du sol à la profondeur `Z`
/// (m).
///
/// Th-BCE 2020 §6.8.3.2.1, éq. (719).
pub fn facteur_ah(z_m: f64) -> f64 {
    (1.0 - 0.1993 * z_m + 0.01381 * z_m * z_m - 0.000335 * z_m * z_m * z_m).max(0.0)
}

/// « VS » : déphasage de la température du sol, en heures.
///
/// Th-BCE 2020 §6.8.3.2.1, éq. (720).
pub fn dephasage_vs_h(z_m: f64) -> f64 {
    24.0 * (0.1786 + 10.298 * z_m - 1.0156 * z_m * z_m + 0.3385 * z_m * z_m * z_m
        - 0.0195 * z_m * z_m * z_m * z_m)
}

/// « θAM » (moyenne des moyennes mensuelles) et « ΔθA » (demi-écart entre la moyenne mensuelle
/// maximale et la minimale) de la température extérieure horaire `te_c`, mois au sens des
/// scénarios (§6.8.3.2.1).
pub fn moyennes_mensuelles(te_c: &[f64]) -> (f64, f64) {
    let mut somme = [0.0_f64; 12];
    let mut nb = [0.0_f64; 12];
    for (h, t) in te_c.iter().enumerate() {
        let m = usize::from(date_scenario(h).mois).clamp(1, 12) - 1;
        somme[m] += t;
        nb[m] += 1.0;
    }
    let moyennes: Vec<f64> = somme
        .iter()
        .zip(&nb)
        .filter(|(_, n)| **n > 0.0)
        .map(|(s, n)| s / n)
        .collect();
    if moyennes.is_empty() {
        return (0.0, 0.0);
    }
    let theta_am = moyennes.iter().sum::<f64>() / moyennes.len() as f64;
    let max = moyennes.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min = moyennes.iter().copied().fold(f64::INFINITY, f64::min);
    (theta_am, (max - min) / 2.0)
}

/// Sol d'un puits : courbe sinusoïdale de la température à la profondeur des conduits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sol {
    gm: f64,
    ah: f64,
    vs_h: f64,
    theta_am_c: f64,
    delta_theta_a_k: f64,
}

impl Sol {
    /// Sol d'un puits décrit, pour la température extérieure horaire `te_c` du site.
    pub fn nouveau(p: &PuitsClimatique, te_c: &[f64]) -> Self {
        let (theta_am_c, delta_theta_a_k) = moyennes_mensuelles(te_c);
        Self {
            gm: gm(p.type_sol),
            ah: facteur_ah(p.profondeur_m),
            vs_h: dephasage_vs_h(p.profondeur_m),
            theta_am_c,
            delta_theta_a_k,
        }
    }

    /// « θsol(h) » : température du sol au pas de temps `h` (0 à 8 759), `JH(h) = h + 1`.
    ///
    /// Th-BCE 2020 §6.8.3.2.1, éq. (721).
    pub fn theta_c(&self, h: usize) -> f64 {
        let jh = (h + 1) as f64;
        self.gm
            * (self.theta_am_c
                - self.ah
                    * self.delta_theta_a_k
                    * libm::sin(2.0 * PI / 8760.0 * (jh - self.vs_h + 24.0 * 25.0)))
    }
}

/// « εsol-air » : efficacité de l'échange entre le sol et l'air neuf, au débit massique total
/// `qm_kg_s` (kg/s) de l'air neuf et à la température extérieure `theta_ext_c`.
///
/// Th-BCE 2020 §6.8.3.2.3, éq. (723) à (727). Le débit volumique de l'éq. (726) est en m³/s,
/// comme le dit la nomenclature (la vitesse `v0` est en m/s) : le facteur 3 600 que le texte
/// y écrit est écarté (`docs/questions/Q231.md` point 1). Un débit nul donne la limite, 1.
pub fn efficacite_sol_air(p: &PuitsClimatique, theta_ext_c: f64, qm_kg_s: f64) -> f64 {
    efficacite_avec_facteur(p, theta_ext_c, qm_kg_s, 1.0)
}

/// Variante non exposée de [`efficacite_sol_air`] : éq. (726) à la lettre, avec le facteur
/// 3 600 (vitesse en m/h dans l'éq. 725).
pub fn efficacite_sol_air_litterale(p: &PuitsClimatique, theta_ext_c: f64, qm_kg_s: f64) -> f64 {
    efficacite_avec_facteur(p, theta_ext_c, qm_kg_s, 3600.0)
}

fn efficacite_avec_facteur(
    p: &PuitsClimatique,
    theta_ext_c: f64,
    qm_kg_s: f64,
    facteur_qv: f64,
) -> f64 {
    if qm_kg_s <= 0.0 {
        return 1.0;
    }
    let nd = f64::from(p.nd.max(1));
    let qm_conduit = qm_kg_s / nd;
    // Éq. (726) et (727).
    let qv = qm_conduit / RHO_AIR_KG_M3 * facteur_qv;
    let v0 = qv / (PI * (p.di_m / 2.0) * (p.di_m / 2.0));
    // Éq. (725).
    let t = theta_ext_c / 100.0;
    let h_i = (4.13 + 0.23 * t - 0.0077 * t * t) * libm::pow(v0, 0.75) / libm::pow(p.di_m, 0.25);
    // Éq. (724).
    let u_g = 1.0
        / (libm::log((p.di_m + 2.0 * p.ep_m) / p.di_m) / (2.0 * PI * p.lambda_tube_w_m_k)
            + 1.0 / (h_i * PI * p.di_m));
    // Éq. (723).
    1.0 - libm::exp(-u_g * p.l_m / (qm_conduit * CP_AIR_J_KG_K))
}

/// « θair-out-actif » : température de l'air en sortie du puits s'il n'est pas contourné.
///
/// Th-BCE 2020 §6.8.3.2.3, éq. (728).
pub fn theta_sortie_active_c(theta_ext_c: f64, epsilon: f64, theta_sol_c: f64) -> f64 {
    theta_ext_c + epsilon * (theta_sol_c - theta_ext_c)
}

/// « δbypass_PC » : le puits est contourné (gestion du bypass en fonction des températures,
/// `typegest_PC = 1`). `chauffage` : « IsaisonPC = chauffage » ; `theta_i_fin_c` : « θi,fin_PC ».
///
/// Th-BCE 2020 §6.10.3, éq. (750) à (752) : conditions simultanées `θext < θi,fin`,
/// `θext >` consigne extérieure et `θi,fin >` consigne intérieure, de la période de chauffage ou
/// du reste de l'année.
pub fn delta_bypass(
    b: &BypassPuits,
    chauffage: bool,
    theta_ext_c: f64,
    theta_i_fin_c: f64,
) -> bool {
    let (ext, int) = if chauffage {
        (b.theta_ext_hiver_c, b.theta_int_hiver_c)
    } else {
        (b.theta_ext_ete_c, b.theta_int_ete_c)
    };
    theta_ext_c < theta_i_fin_c && theta_ext_c > ext && theta_i_fin_c > int
}

/// « Iactivation_PC » selon la régulation propre à la fiche 6.8 (variante non exposée, Q231
/// point 2) : en période de chauffage, le puits est activé si `θext < θext,arrêt_PC^hiver` ; hors
/// période de chauffage, s'il est activé si `θext > θext,arrêt_PC^été` (le texte écrit « hiver »
/// à l'éq. 730, coquille). Avec `typegest_PC = 1`, une condition sur la température de sortie
/// `θair-out` s'ajoute (lue sur la température de sortie du puits activé).
///
/// Th-BCE 2020 §6.8.3.3.2, éq. (729) et (730).
pub fn activation_fiche_6_8(
    chauffage: bool,
    gestion_sur_sortie: bool,
    theta_ext_c: f64,
    theta_arret_hiver_c: f64,
    theta_arret_ete_c: f64,
    theta_sortie_active_c: f64,
) -> bool {
    if chauffage {
        theta_ext_c < theta_arret_hiver_c
            && (!gestion_sur_sortie || theta_ext_c < theta_sortie_active_c)
    } else {
        theta_ext_c > theta_arret_ete_c
            && (!gestion_sur_sortie || theta_ext_c > theta_sortie_active_c)
    }
}

/// État d'un puits climatique au cours de la simulation d'un système.
#[derive(Debug, Clone, PartialEq)]
pub struct EtatPuits {
    /// Description saisie.
    pub description: PuitsClimatique,
    sol: Option<Sol>,
    theta_sol_c: f64,
    bypass: bool,
    /// « θan(h) » du pas, calculé avec le débit d'air neuf de la fin du pas précédent
    /// (éq. 414 et 658) ; `θext` tant qu'aucun débit n'est connu.
    theta_an_c: f64,
    /// Débit massique d'air neuf de la fin du pas précédent, en kg/s ; nul : inconnu.
    qm_prec_kg_s: f64,
}

impl EtatPuits {
    /// Puits décrit, avant le premier pas.
    pub fn nouveau(description: PuitsClimatique) -> Self {
        Self {
            description,
            sol: None,
            theta_sol_c: 0.0,
            bypass: false,
            theta_an_c: 0.0,
            qm_prec_kg_s: 0.0,
        }
    }

    /// Prépare le pas `h` : sol (calculé au premier pas sur `te_c`), bypass (éq. 750 à 752, avec
    /// la saison et la température intérieure des groupes desservis) et `θan(h)` avec le débit
    /// de la fin du pas précédent.
    pub fn preparer(
        &mut self,
        h: usize,
        te_c: &[f64],
        theta_ext_c: f64,
        chauffage: bool,
        theta_i_fin_c: f64,
    ) {
        let p = self.description;
        let sol = *self.sol.get_or_insert_with(|| Sol::nouveau(&p, te_c));
        self.theta_sol_c = sol.theta_c(h);
        // Éq. (750) : pas de bypass par défaut.
        self.bypass = p
            .bypass
            .is_some_and(|b| delta_bypass(&b, chauffage, theta_ext_c, theta_i_fin_c));
        self.theta_an_c = if self.qm_prec_kg_s > 0.0 {
            self.sortie_c(theta_ext_c, self.qm_prec_kg_s)
        } else {
            theta_ext_c
        };
    }

    /// Température de l'air en sortie du puits au débit `qm_kg_s` : `θext` si le puits est
    /// contourné (éq. 731), sinon éq. (728).
    pub fn sortie_c(&self, theta_ext_c: f64, qm_kg_s: f64) -> f64 {
        if self.bypass {
            theta_ext_c
        } else {
            let eps = efficacite_sol_air(&self.description, theta_ext_c, qm_kg_s);
            theta_sortie_active_c(theta_ext_c, eps, self.theta_sol_c)
        }
    }

    /// « θan(h) » du pas avec le débit de la fin du pas précédent.
    pub fn theta_an_c(&self) -> f64 {
        self.theta_an_c
    }

    /// Aucun débit n'est connu : le premier pas se calcule avec son propre débit d'air neuf.
    pub fn sans_historique(&self) -> bool {
        self.qm_prec_kg_s <= 0.0
    }

    /// Le puits est contourné au pas de temps (« δbypass_PC »).
    pub fn contourne(&self) -> bool {
        self.bypass
    }

    /// Retient le débit massique d'air neuf de la fin du pas.
    pub fn retenir_debit(&mut self, qm_kg_s: f64) {
        self.qm_prec_kg_s = qm_kg_s.max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn puits() -> PuitsClimatique {
        PuitsClimatique {
            nd: 2,
            l_m: 30.0,
            di_m: 0.2,
            ep_m: 0.005,
            lambda_tube_w_m_k: 0.4,
            profondeur_m: 2.0,
            type_sol: TypeSolPuits::SableHumide,
            bypass: None,
        }
    }

    #[test]
    fn profondeur_a_la_main() {
        // Z = 2 : AH = 1 − 0,3986 + 0,05524 − 0,00268 = 0,65396 ;
        // VS = 24 × (0,1786 + 20,596 − 4,0624 + 2,708 − 0,312) = 24 × 19,1082 = 458,5968 h.
        assert!((facteur_ah(2.0) - 0.65396).abs() < 1e-9);
        assert!((dephasage_vs_h(2.0) - 458.5968).abs() < 1e-9);
        // Profondeur nulle : AH = 1 ; grande profondeur : AH borné à 0.
        assert_eq!(facteur_ah(0.0), 1.0);
        assert_eq!(facteur_ah(30.0), 0.0);
    }

    #[test]
    fn sol_a_la_main() {
        // Température extérieure : 10 °C toute l'année → ΔθA = 0, θsol = gm × 10 = 9,8 °C.
        let te = vec![10.0; 8760];
        let s = Sol::nouveau(&puits(), &te);
        assert!((s.theta_c(1000) - 9.8).abs() < 1e-9);
        // Amplitude : moitié de l'année à 0, moitié à 20 → moyennes mensuelles 0 ou 20 :
        // θAM = (6 × 0 + 6 × 20)/12 = 10, ΔθA = 10 (mois entiers : janvier à juin à 0).
        let mut te = vec![0.0; 8760];
        for (h, t) in te.iter_mut().enumerate() {
            if date_scenario(h).mois > 6 {
                *t = 20.0;
            }
        }
        let (am, d) = moyennes_mensuelles(&te);
        assert!((am - 10.0).abs() < 1e-9 && (d - 10.0).abs() < 1e-9);
        // éq. 721 au pas h = 4 000 (JH = 4 001) avec Z = 2, sable humide :
        // arg = 2π/8760 × (4001 − 458,5968 + 600) = 2π/8760 × 4142,4032 ; θsol = 0,98 × (10 −
        // 0,65396 × 10 × sin(arg)).
        let s = Sol::nouveau(&puits(), &te);
        let arg = 2.0 * PI / 8760.0 * (4001.0 - 458.5968 + 600.0);
        let attendu = 0.98 * (10.0 - 0.65396 * 10.0 * arg.sin());
        assert!((s.theta_c(4000) - attendu).abs() < 1e-9);
    }

    #[test]
    fn efficacite_a_la_main() {
        // 2 conduits, 0,0678 kg/s au total (≈ 200 m³/h), θext = 10 °C, di = 0,2 m.
        // Par conduit : qm = 0,0339 kg/s ; qv = 0,0339/1,22 = 0,027787 m³/s ;
        // v0 = 0,027787/(π × 0,01) = 0,88448 m/s ;
        // hi = (4,13 + 0,023 − 0,000077) × 0,88448^0,75 / 0,2^0,25 = 5,6639 W/(m².K) ;
        // Ug = 1/(ln(1,05)/(2π × 0,4) + 1/(5,6639 × π × 0,2)) = 1/(0,019413 + 0,280978)
        //    = 3,3288 ; NTU = 3,3288 × 30/(0,0339 × 1008) = 2,9224 ; ε = 1 − exp(−2,9224)
        //    = 0,9462.
        let e = efficacite_sol_air(&puits(), 10.0, 0.0678);
        assert!((e - 0.9462).abs() < 5e-4, "{e}");
        // Débit nul : limite 1.
        assert_eq!(efficacite_sol_air(&puits(), 10.0, 0.0), 1.0);
        // Éq. (728).
        assert!((theta_sortie_active_c(0.0, 0.9, 10.0) - 9.0).abs() < 1e-12);
        // La lecture littérale (×3600) donne un puits parfait.
        assert!(efficacite_sol_air_litterale(&puits(), 10.0, 0.0678) > 0.999_999);
    }

    #[test]
    fn bypass_a_la_main() {
        let b = BypassPuits {
            theta_int_hiver_c: 22.0,
            theta_ext_hiver_c: 8.0,
            theta_int_ete_c: 24.0,
            theta_ext_ete_c: 12.0,
        };
        // Chauffage : θext 10 < θi 23 ; 10 > 8 ; 23 > 22 : contourné (éq. 751).
        assert!(delta_bypass(&b, true, 10.0, 23.0));
        // Trop froid dehors (7 < 8) ou intérieur trop frais (21 < 22) : puits traversé.
        assert!(!delta_bypass(&b, true, 7.0, 23.0));
        assert!(!delta_bypass(&b, true, 10.0, 21.0));
        // Hors chauffage : consignes d'été (éq. 752) ; dehors plus chaud que dedans : traversé.
        assert!(delta_bypass(&b, false, 15.0, 25.0));
        assert!(!delta_bypass(&b, false, 26.0, 25.0));
        assert!(!delta_bypass(&b, false, 11.0, 25.0));
    }

    #[test]
    fn regulation_de_la_fiche_6_8() {
        // Hiver : actif si θext < 15 (éq. 729) ; avec gestion sur la sortie, aussi si
        // θext < θsortie.
        assert!(activation_fiche_6_8(true, false, 5.0, 15.0, 20.0, 0.0));
        assert!(!activation_fiche_6_8(true, false, 16.0, 15.0, 20.0, 0.0));
        assert!(!activation_fiche_6_8(true, true, 5.0, 15.0, 20.0, 3.0));
        // Hors chauffage : actif si θext > 20 (éq. 730, seuil d'été).
        assert!(activation_fiche_6_8(false, false, 25.0, 15.0, 20.0, 0.0));
        assert!(!activation_fiche_6_8(false, false, 18.0, 15.0, 20.0, 0.0));
        assert!(!activation_fiche_6_8(false, true, 25.0, 15.0, 20.0, 27.0));
    }

    #[test]
    fn etat_du_puits() {
        let mut e = EtatPuits::nouveau(puits());
        let te = vec![10.0; 8760];
        // Premier pas : pas de débit connu, θan = θext.
        e.preparer(0, &te, 10.0, true, 20.0);
        assert!(e.sans_historique());
        assert_eq!(e.theta_an_c(), 10.0);
        e.retenir_debit(0.0678);
        // Pas suivant : θan avec le débit précédent ; sol à 9,8 °C (θext constante).
        e.preparer(1, &te, 0.0, true, 20.0);
        let eps = efficacite_sol_air(&puits(), 0.0, 0.0678);
        assert!((e.theta_an_c() - eps * e.theta_sol_c).abs() < 1e-9);
        // Avec bypass : θext.
        let mut p = puits();
        p.bypass = Some(BypassPuits {
            theta_int_hiver_c: 15.0,
            theta_ext_hiver_c: -5.0,
            theta_int_ete_c: 24.0,
            theta_ext_ete_c: 12.0,
        });
        let mut e = EtatPuits::nouveau(p);
        e.retenir_debit(0.0678);
        e.preparer(0, &te, 3.0, true, 20.0);
        assert!(e.contourne());
        assert_eq!(e.theta_an_c(), 3.0);
    }
}
