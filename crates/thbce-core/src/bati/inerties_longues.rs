// SPDX-License-Identifier: GPL-3.0-or-later
//! Inerties séquentielle et annuelle d'un groupe : fiche `C_BAT_comportement thermique d'un
//! groupe` (Th-BCE 2020 §5.21.3.3, §5.21.3.4 et §5.21.3.10).
//!
//! Par superposition des états, deux capacités s'ajoutent à la capacité quotidienne `Cmq` du
//! modèle à trois nœuds : la masse séquentielle `θms`, sollicitée par la moyenne sur 24 h de la
//! température de masse quotidienne, et la masse annuelle `θma`, sollicitée par sa moyenne sur
//! une séquence de 14 jours. Chacune rend au nœud `θmq` un flux, `φmq_seq` et `φmq_ann`, qui
//! entre au second membre de l'éq. (365).
//!
//! « En mode Th-B et Th-C, les inerties séquentielle et annuelle ne sont pas prises en compte »
//! (§5.21.3.2) : ce module ne sert qu'au calcul Th-D.
//!
//! Lectures (`docs/questions.md`, Q40) :
//! - la figure 66 attache `θmq,NBj,seq` à la branche séquentielle et `θmq,24h` à la branche
//!   annuelle, à l'inverse des éq. (354) et (358) : les équations sont retenues ;
//! - les sommes des éq. (353) et (357) vont de `h−1−24` à `h−1`, soit 25 termes pour un
//!   diviseur de 24 (337 pour 336) : lues comme la moyenne des 24 (336) dernières valeurs ;
//! - les notes initialisent `θmq,24h` à 18 °C « pour les 24 premières heures » et `θmq,NBjseq`
//!   à 19 °C « pour les NBjseq premiers jours » : la moyenne est remplacée par cette valeur
//!   tant que l'historique n'est pas complet ;
//! - `ΔHg` est nul quand `ΔR` est nul (notes des éq. 329 et 330) ; la même valeur est retenue
//!   quand `ΔR` est négatif, une conductance négative n'ayant pas de sens.

use std::collections::VecDeque;

use crate::entree::{Inertie, InertieAnnuelle, InertieSequentielle};

use super::comportement_thermique::{inertie_quotidienne, H_IS_W_M2K};

/// « NBj,seq » : nombre de jours d'une séquence, en jours (§5.21.3.2, éq. 315).
pub const NB_J_SEQ: usize = 14;
/// Nombre de pas de temps de la moyenne sur 24 h (éq. 353).
pub const NB_H_24H: usize = 24;
/// Nombre de pas de temps de la moyenne sur la séquence (éq. 357).
pub const NB_H_SEQ: usize = 24 * NB_J_SEQ;
/// Valeur de `θmq,24h` pendant les 24 premières heures de simulation, en °C (note de l'éq. 353).
pub const THETA_MQ_24H_INITIALE_C: f64 = 18.0;
/// Valeur de `θmq,NBjseq` pendant les `NBj,seq` premiers jours, en °C (note de l'éq. 357).
pub const THETA_MQ_NBJSEQ_INITIALE_C: f64 = 19.0;
/// Valeur initiale de `θms`, en °C (note de l'éq. 354).
pub const THETA_MS_INITIALE_C: f64 = 18.0;
/// Valeur initiale de `θma`, en °C (note de l'éq. 358).
pub const THETA_MA_INITIALE_C: f64 = 18.0;

/// « Cms_surf » : capacité thermique séquentielle rapportée à la surface utile, en kJ/(K.m²).
///
/// Th-BCE 2020 §5.21.3.3, tableau 54 : par défaut et très légère, `Cmq_surf` ; légère, 250 ;
/// moyenne, 500 ; lourde, 850 ; ou valeur saisie.
pub fn cms_surf_kj_k_m2(classe: &InertieSequentielle, cmq_surf_kj_k_m2: f64) -> f64 {
    match *classe {
        InertieSequentielle::Personnalisee { cms_surf_kj_k_m2 } => cms_surf_kj_k_m2,
        InertieSequentielle::ParDefaut | InertieSequentielle::TresLegere => cmq_surf_kj_k_m2,
        InertieSequentielle::Legere => 250.0,
        InertieSequentielle::Moyenne => 500.0,
        InertieSequentielle::Lourde => 850.0,
    }
}

/// « Cma_surf » : capacité thermique annuelle rapportée à la surface utile, en kJ/(K.m²).
///
/// Th-BCE 2020 §5.21.3.3, éq. (316) : par défaut, `Cma_surf = Cms_surf` ; ou valeur saisie.
pub fn cma_surf_kj_k_m2(classe: &InertieAnnuelle, cms_surf_kj_k_m2: f64) -> f64 {
    match *classe {
        InertieAnnuelle::Personnalisee { cma_surf_kj_k_m2 } => cma_surf_kj_k_m2,
        InertieAnnuelle::ParDefaut => cms_surf_kj_k_m2,
    }
}

/// « ΔHg » : conductance entre le nœud `θmq` et une masse de plus longue période, en W/K.
///
/// Th-BCE 2020 §5.21.3.4, éq. (326) à (330) : `Rmqs = 1/(his·Amq)`, `Rm·s = 1/(his·Am·)`,
/// `ΔR = Rm·s − Rmqs`, `ΔHg = 1/ΔR`, et `ΔHg = 0` pour `ΔR` nul. Nulle aussi pour une surface
/// d'échange nulle (résistance infinie) et pour `ΔR` négatif (voir l'en-tête du module).
pub fn delta_hg_w_k(a_m_longue_m2: f64, a_mq_m2: f64) -> f64 {
    if a_m_longue_m2 <= 0.0 || a_mq_m2 <= 0.0 {
        return 0.0;
    }
    let delta_r = 1.0 / (H_IS_W_M2K * a_m_longue_m2) - 1.0 / (H_IS_W_M2K * a_mq_m2);
    if delta_r > 0.0 {
        1.0 / delta_r
    } else {
        0.0
    }
}

/// Grandeurs fixes des inerties séquentielle et annuelle d'un groupe.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InertiesLongues {
    /// « ΔHgms,s−q », en W/K (éq. 329).
    pub delta_hg_ms_s_q_w_k: f64,
    /// « ΔHgms,a−q », en W/K (éq. 330).
    pub delta_hg_ms_a_q_w_k: f64,
    /// « ΔCm,s−q » : `Cms − Cmq`, en kJ/K (éq. 331).
    pub delta_c_m_s_q_kj_k: f64,
    /// « ΔCm,a−s » : `Cma − Cms`, en kJ/K (éq. 332).
    pub delta_c_m_a_s_kj_k: f64,
}

impl InertiesLongues {
    /// Th-BCE 2020 §5.21.3.3 et 5.21.3.4 : `Ams = Ams_surf·Agr` (éq. 318),
    /// `Ama = Ama_surf·Agr` (éq. 319), `Cms = Cms_surf·Agr` (éq. 321), `Cma = Cma_surf·Agr`
    /// (éq. 322), conductances des éq. (327) à (330), `ΔCm,s−q = Cms − Cmq` (éq. 331) et
    /// `ΔCm,a−s = Cma − Cms` (éq. 332).
    ///
    /// « Par définition, ΔCm,s−q et ΔCm,a−s sont positives ou nulles » : la validation refuse
    /// une saisie qui les rendrait négatives ; elles sont ici bornées à zéro.
    pub fn nouvelles(a_gr_m2: f64, inertie: &Inertie) -> Self {
        let (amq_surf, cmq_surf) = inertie_quotidienne(&inertie.quotidienne);
        let cms_surf = cms_surf_kj_k_m2(&inertie.sequentielle, cmq_surf);
        let cma_surf = cma_surf_kj_k_m2(&inertie.annuelle, cms_surf);
        let a_mq_m2 = amq_surf * a_gr_m2;
        Self {
            delta_hg_ms_s_q_w_k: delta_hg_w_k(inertie.ams_surf_m2_m2 * a_gr_m2, a_mq_m2),
            delta_hg_ms_a_q_w_k: delta_hg_w_k(inertie.ama_surf_m2_m2 * a_gr_m2, a_mq_m2),
            delta_c_m_s_q_kj_k: ((cms_surf - cmq_surf) * a_gr_m2).max(0.0),
            delta_c_m_a_s_kj_k: ((cma_surf - cms_surf) * a_gr_m2).max(0.0),
        }
    }

    /// Vrai si aucune des deux inerties n'échange de chaleur avec la masse quotidienne : c'est
    /// le cas des classes « par défaut » (`Cms = Cmq` et `Cma = Cms`, tableau 54 et éq. 316).
    pub fn sans_effet(&self) -> bool {
        (self.delta_hg_ms_s_q_w_k <= 0.0 || self.delta_c_m_s_q_kj_k <= 0.0)
            && (self.delta_hg_ms_a_q_w_k <= 0.0 || self.delta_c_m_a_s_kj_k <= 0.0)
    }
}

/// Température d'une masse de longue période à la fin du pas de temps, en °C.
///
/// Th-BCE 2020 §5.21.3.10, éq. (354) et (358), schéma de Crank-Nicolson sur un pas d'une heure,
/// `ΔC` en kJ/K :
/// `θ(h) = [(ΔC/3,6 − 0,5·ΔHg)·θ(h−1) + ΔHg·θforçage(h)] / (ΔC/3,6 + 0,5·ΔHg)`.
///
/// Sans capacité (`ΔC = 0`), la formule fait osciller la température autour du forçage sans
/// qu'aucun flux ne soit échangé (`θmoy = θforçage`) : la masse suit alors le forçage. Sans
/// conductance, elle garde sa température.
pub fn theta_masse_longue_c(
    delta_c_kj_k: f64,
    delta_hg_w_k: f64,
    theta_precedente_c: f64,
    theta_forcage_c: f64,
) -> f64 {
    if delta_hg_w_k <= 0.0 {
        return theta_precedente_c;
    }
    if delta_c_kj_k <= 0.0 {
        return theta_forcage_c;
    }
    let c = delta_c_kj_k / 3.6;
    let demi = 0.5 * delta_hg_w_k;
    ((c - demi) * theta_precedente_c + delta_hg_w_k * theta_forcage_c) / (c + demi)
}

/// Flux rendus au nœud `θmq` par les inerties séquentielle et annuelle à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FluxInerties {
    /// « θmq,24h(h) », en °C (éq. 353).
    pub theta_mq_24h_c: f64,
    /// « θmq,NBjseq(h) », en °C (éq. 357).
    pub theta_mq_nbjseq_c: f64,
    /// « θms(h) », en °C (éq. 354).
    pub theta_ms_c: f64,
    /// « θma(h) », en °C (éq. 358).
    pub theta_ma_c: f64,
    /// « φmq_seq(h) », en W (éq. 355).
    pub phi_mq_seq_w: f64,
    /// « φmq_ann(h) », en W (éq. 359).
    pub phi_mq_ann_w: f64,
    /// Variation de l'énergie stockée dans les deux masses sur le pas de temps, en Wh :
    /// `ΔCm,s−q·Δθms + ΔCm,a−s·Δθma`. Égale à `−(φmq_seq + φmq_ann)` par construction du schéma
    /// de Crank-Nicolson ; elle sert au bilan d'énergie de contrôle.
    pub stockage_wh: f64,
}

impl FluxInerties {
    /// « φmq_seq(h) + φmq_ann(h) », en W : terme du second membre de l'éq. (365).
    pub fn phi_mq_seq_ann_w(&self) -> f64 {
        self.phi_mq_seq_w + self.phi_mq_ann_w
    }
}

/// État des inerties séquentielle et annuelle d'un groupe d'un pas de temps au suivant.
#[derive(Debug, Clone, PartialEq)]
pub struct EtatInerties {
    /// « θms(h−1) », en °C.
    pub theta_ms_c: f64,
    /// « θma(h−1) », en °C.
    pub theta_ma_c: f64,
    /// Températures `θmq` à la fin des pas de temps précédents, la plus récente en dernier ; au
    /// plus `24·NBj,seq` valeurs.
    historique: VecDeque<f64>,
}

impl Default for EtatInerties {
    fn default() -> Self {
        Self {
            theta_ms_c: THETA_MS_INITIALE_C,
            theta_ma_c: THETA_MA_INITIALE_C,
            historique: VecDeque::with_capacity(NB_H_SEQ),
        }
    }
}

impl EtatInerties {
    fn moyenne_des_dernieres(&self, n: usize) -> Option<f64> {
        (self.historique.len() >= n).then(|| {
            #[allow(clippy::cast_precision_loss)]
            let diviseur = n as f64;
            self.historique.iter().rev().take(n).sum::<f64>() / diviseur
        })
    }

    /// « θmq,24h(h) » : moyenne des `θmq` des 24 pas de temps précédents, en °C.
    ///
    /// Th-BCE 2020 §5.21.3.10.1, éq. (353) ; 18 °C pendant les 24 premières heures (note).
    pub fn theta_mq_24h_c(&self) -> f64 {
        self.moyenne_des_dernieres(NB_H_24H)
            .unwrap_or(THETA_MQ_24H_INITIALE_C)
    }

    /// « θmq,NBjseq(h) » : moyenne des `θmq` des `24·NBj,seq` pas de temps précédents, en °C.
    ///
    /// Th-BCE 2020 §5.21.3.10.2, éq. (357) ; 19 °C pendant les `NBj,seq` premiers jours (note).
    pub fn theta_mq_nbjseq_c(&self) -> f64 {
        self.moyenne_des_dernieres(NB_H_SEQ)
            .unwrap_or(THETA_MQ_NBJSEQ_INITIALE_C)
    }

    /// Calcule les températures des deux masses à la fin du pas de temps et les flux rendus au
    /// nœud `θmq`, puis avance l'état des masses.
    ///
    /// Th-BCE 2020 §5.21.3.10 :
    /// - éq. (354) : `θms(h)` à partir de `θms(h−1)` et de `θmq,24h(h)` ;
    /// - éq. (355) et (356) : `φmq_seq = ΔHgms,s−q·(θms,moy − θmq,24h)`, avec
    ///   `θms,moy = (θms(h) + θms(h−1))/2` ;
    /// - éq. (358) : `θma(h)` à partir de `θma(h−1)` et de `θmq,NBjseq(h)`, avec `ΔCm,a−s` et
    ///   `ΔHgms,a−q` ;
    /// - éq. (359) et (360) : `φmq_ann = ΔHgms,a−q·(θma,moy − θmq,NBjseq)`.
    ///
    /// Les deux moyennes ne dépendent que des pas de temps précédents : les flux sont connus
    /// avant la résolution du bilan du groupe.
    pub fn avancer(&mut self, p: &InertiesLongues) -> FluxInerties {
        let theta_mq_24h_c = self.theta_mq_24h_c();
        let theta_mq_nbjseq_c = self.theta_mq_nbjseq_c();
        let theta_ms_c = theta_masse_longue_c(
            p.delta_c_m_s_q_kj_k,
            p.delta_hg_ms_s_q_w_k,
            self.theta_ms_c,
            theta_mq_24h_c,
        );
        let theta_ma_c = theta_masse_longue_c(
            p.delta_c_m_a_s_kj_k,
            p.delta_hg_ms_a_q_w_k,
            self.theta_ma_c,
            theta_mq_nbjseq_c,
        );
        // Sans capacité, la masse suit le forçage et n'échange rien (voir
        // `theta_masse_longue_c`).
        let flux = |delta_c: f64, delta_hg: f64, fin: f64, debut: f64, forcage: f64| {
            if delta_c <= 0.0 {
                0.0
            } else {
                delta_hg * (0.5 * (fin + debut) - forcage)
            }
        };
        let sorties = FluxInerties {
            theta_mq_24h_c,
            theta_mq_nbjseq_c,
            theta_ms_c,
            theta_ma_c,
            phi_mq_seq_w: flux(
                p.delta_c_m_s_q_kj_k,
                p.delta_hg_ms_s_q_w_k,
                theta_ms_c,
                self.theta_ms_c,
                theta_mq_24h_c,
            ),
            phi_mq_ann_w: flux(
                p.delta_c_m_a_s_kj_k,
                p.delta_hg_ms_a_q_w_k,
                theta_ma_c,
                self.theta_ma_c,
                theta_mq_nbjseq_c,
            ),
            stockage_wh: p.delta_c_m_s_q_kj_k / 3.6 * (theta_ms_c - self.theta_ms_c)
                + p.delta_c_m_a_s_kj_k / 3.6 * (theta_ma_c - self.theta_ma_c),
        };
        self.theta_ms_c = theta_ms_c;
        self.theta_ma_c = theta_ma_c;
        sorties
    }

    /// Ajoute à l'historique la température de masse quotidienne `θmq(h)` à la fin du pas de
    /// temps qui vient d'être résolu.
    pub fn enregistrer(&mut self, theta_mq_fin_c: f64) {
        if self.historique.len() == NB_H_SEQ {
            self.historique.pop_front();
        }
        self.historique.push_back(theta_mq_fin_c);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::entree::InertieQuotidienne;

    fn inertie(
        sequentielle: InertieSequentielle,
        ams_surf_m2_m2: f64,
        annuelle: InertieAnnuelle,
        ama_surf_m2_m2: f64,
    ) -> Inertie {
        Inertie {
            quotidienne: InertieQuotidienne::Moyenne,
            sequentielle,
            ams_surf_m2_m2,
            annuelle,
            ama_surf_m2_m2,
        }
    }

    fn proche(a: f64, b: f64, tolerance: f64) -> bool {
        (a - b).abs() <= tolerance
    }

    #[test]
    fn classes_du_tableau_54_et_eq_316() {
        let cmq = 165.0;
        assert_eq!(
            cms_surf_kj_k_m2(&InertieSequentielle::ParDefaut, cmq),
            165.0
        );
        assert_eq!(
            cms_surf_kj_k_m2(&InertieSequentielle::TresLegere, cmq),
            165.0
        );
        assert_eq!(cms_surf_kj_k_m2(&InertieSequentielle::Legere, cmq), 250.0);
        assert_eq!(cms_surf_kj_k_m2(&InertieSequentielle::Moyenne, cmq), 500.0);
        assert_eq!(cms_surf_kj_k_m2(&InertieSequentielle::Lourde, cmq), 850.0);
        let saisie = InertieSequentielle::Personnalisee {
            cms_surf_kj_k_m2: 321.0,
        };
        assert_eq!(cms_surf_kj_k_m2(&saisie, cmq), 321.0);
        // Éq. (316) : par défaut, Cma_surf = Cms_surf.
        assert_eq!(cma_surf_kj_k_m2(&InertieAnnuelle::ParDefaut, 500.0), 500.0);
        let saisie = InertieAnnuelle::Personnalisee {
            cma_surf_kj_k_m2: 1200.0,
        };
        assert_eq!(cma_surf_kj_k_m2(&saisie, 500.0), 1200.0);
        assert_eq!((NB_J_SEQ, NB_H_SEQ), (14, 336));
    }

    #[test]
    fn conductances_et_capacites_calculees_a_la_main() {
        // Groupe de 100 m², inertie quotidienne moyenne : Amq = 250 m², Cmq = 16 500 kJ/K.
        // his = 2,5 + 1,2 × 5,5 = 9,1 W/(m².K) ; Hgmqs = 9,1 × 250 = 2 275 W/K.
        // Séquentielle moyenne, Ams_surf = 1 : Ams = 100 m², Hgmss = 910 W/K ;
        // ΔR = 1/910 − 1/2275 ; ΔHg = 910 × 2275/(2275 − 910) = 2 070 250/1 365 = 1 516,67 W/K.
        // Cms = 500 × 100 = 50 000 kJ/K ; ΔCm,s−q = 33 500 kJ/K.
        // Annuelle saisie à 1 000 kJ/(K.m²), Ama_surf = 0,5 : Hgmas = 455 W/K ;
        // ΔHg = 455 × 2275/(2275 − 455) = 1 035 125/1 820 = 568,75 W/K ; ΔCm,a−s = 50 000 kJ/K.
        let p = InertiesLongues::nouvelles(
            100.0,
            &inertie(
                InertieSequentielle::Moyenne,
                1.0,
                InertieAnnuelle::Personnalisee {
                    cma_surf_kj_k_m2: 1000.0,
                },
                0.5,
            ),
        );
        assert!(proche(p.delta_hg_ms_s_q_w_k, 2_070_250.0 / 1365.0, 1e-9));
        assert!(proche(p.delta_hg_ms_a_q_w_k, 568.75, 1e-9));
        assert_eq!(p.delta_c_m_s_q_kj_k, 33_500.0);
        assert_eq!(p.delta_c_m_a_s_kj_k, 50_000.0);
        assert!(!p.sans_effet());
    }

    #[test]
    fn classes_par_defaut_et_cas_degeneres_sans_effet() {
        // Par défaut : Cms = Cmq et Cma = Cms, aucune capacité supplémentaire.
        let defaut = InertiesLongues::nouvelles(
            100.0,
            &inertie(
                InertieSequentielle::ParDefaut,
                1.0,
                InertieAnnuelle::ParDefaut,
                1.0,
            ),
        );
        assert_eq!(
            (defaut.delta_c_m_s_q_kj_k, defaut.delta_c_m_a_s_kj_k),
            (0.0, 0.0)
        );
        assert!(defaut.sans_effet());
        // Notes des éq. (329) et (330) : ΔR nul (Ams = Amq), ΔHg = 0.
        assert_eq!(delta_hg_w_k(250.0, 250.0), 0.0);
        // ΔR négatif (Ams > Amq) ou surface nulle : conductance nulle.
        assert_eq!(delta_hg_w_k(300.0, 250.0), 0.0);
        assert_eq!(delta_hg_w_k(0.0, 250.0), 0.0);
        // Capacité séquentielle inférieure à la capacité quotidienne : ΔC bornée à zéro.
        let leger = Inertie {
            quotidienne: InertieQuotidienne::TresLourde,
            ..inertie(
                InertieSequentielle::Legere,
                1.0,
                InertieAnnuelle::ParDefaut,
                1.0,
            )
        };
        assert_eq!(
            InertiesLongues::nouvelles(100.0, &leger).delta_c_m_s_q_kj_k,
            0.0
        );
        // Sans conductance, la masse garde sa température ; sans capacité, elle suit le
        // forçage.
        assert_eq!(theta_masse_longue_c(1000.0, 0.0, 18.0, 25.0), 18.0);
        assert_eq!(theta_masse_longue_c(0.0, 100.0, 18.0, 25.0), 25.0);
    }

    #[test]
    fn initialisations_et_moyennes_glissantes() {
        let mut e = EtatInerties::default();
        assert_eq!((e.theta_ms_c, e.theta_ma_c), (18.0, 18.0));
        // Notes des éq. (353) et (357) : 18 °C pendant les 24 premières heures, 19 °C pendant
        // les 14 premiers jours.
        for h in 0..23 {
            e.enregistrer(20.0 + f64::from(h));
            assert_eq!(e.theta_mq_24h_c(), 18.0);
            assert_eq!(e.theta_mq_nbjseq_c(), 19.0);
        }
        // 24 valeurs disponibles (20, 21, …, 43) : moyenne 31,5 °C.
        e.enregistrer(43.0);
        assert_eq!(e.theta_mq_24h_c(), 31.5);
        assert_eq!(e.theta_mq_nbjseq_c(), 19.0);
        // La moyenne glisse : 21, …, 43 et 50, soit 31,5 + (50 − 20)/24 = 32,75 °C.
        e.enregistrer(50.0);
        assert_eq!(e.theta_mq_24h_c(), 32.75);
        // 336 valeurs : 25 déjà saisies, de somme 20 + … + 43 + 50 = 806, puis 311 à 10 °C.
        for _ in 0..(NB_H_SEQ - 26) {
            e.enregistrer(10.0);
            assert_eq!(e.theta_mq_nbjseq_c(), 19.0);
        }
        e.enregistrer(10.0);
        assert!(proche(
            e.theta_mq_nbjseq_c(),
            (806.0 + 3110.0) / 336.0,
            1e-12
        ));
        assert_eq!(e.theta_mq_24h_c(), 10.0);
        // La fenêtre ne garde que 336 valeurs : la première (20 °C) sort.
        e.enregistrer(10.0);
        assert!(proche(
            e.theta_mq_nbjseq_c(),
            (806.0 - 20.0 + 3120.0) / 336.0,
            1e-12
        ));
    }

    #[test]
    fn pas_de_temps_calcule_a_la_main() {
        // Mêmes grandeurs que `conductances_et_capacites_calculees_a_la_main`.
        let p = InertiesLongues {
            delta_hg_ms_s_q_w_k: 2_070_250.0 / 1365.0,
            delta_hg_ms_a_q_w_k: 568.75,
            delta_c_m_s_q_kj_k: 33_500.0,
            delta_c_m_a_s_kj_k: 50_000.0,
        };
        let mut e = EtatInerties::default();
        // Premier pas de temps : θmq,24h = 18 °C = θms ; θmq,NBjseq = 19 °C, θma = 18 °C.
        // Éq. (354) : θms reste à 18 °C, φmq_seq = 0.
        // Éq. (358) : ΔC/3,6 = 13 888,889 Wh/K ; 0,5·ΔHg = 284,375 ;
        // θma = (13 604,514 × 18 + 568,75 × 19)/14 173,264 = 18,040128… °C ;
        // éq. (359) : φmq_ann = 568,75 × ((18 + 18,040128)/2 − 19) = −557,338… W.
        let f = e.avancer(&p);
        assert_eq!((f.theta_mq_24h_c, f.theta_mq_nbjseq_c), (18.0, 19.0));
        assert!(proche(f.theta_ms_c, 18.0, 1e-12) && proche(f.phi_mq_seq_w, 0.0, 1e-9));
        let c_a = 50_000.0 / 3.6;
        let theta_ma = ((c_a - 284.375) * 18.0 + 568.75 * 19.0) / (c_a + 284.375);
        assert!(proche(theta_ma, 18.040_128, 1e-6));
        assert!(proche(f.theta_ma_c, theta_ma, 1e-12));
        assert!(proche(f.phi_mq_ann_w, -557.338, 1e-3));
        // Le flux rendu à la masse quotidienne est l'opposé de l'énergie stockée (W × 1 h).
        assert!(proche(f.stockage_wh, -f.phi_mq_seq_ann_w(), 1e-9));
        assert_eq!(e.theta_ma_c, f.theta_ma_c);

        // Après 24 pas de temps à θmq = 20 °C : θmq,24h = 20 °C.
        // Éq. (354) : ΔC/3,6 = 9 305,556 Wh/K ; 0,5·ΔHg = 758,333 ;
        // θms = (8 547,222 × 18 + 1 516,667 × 20)/10 063,889 = 18,301408… °C ;
        // éq. (355) : φmq_seq = 1 516,667 × ((18 + 18,301408)/2 − 20) = −2 804,766… W.
        let mut e = EtatInerties::default();
        for _ in 0..24 {
            e.enregistrer(20.0);
        }
        let f = e.avancer(&p);
        assert_eq!(f.theta_mq_24h_c, 20.0);
        assert!(proche(f.theta_ms_c, 18.301_408, 1e-6));
        assert!(proche(f.phi_mq_seq_w, -2804.766, 1e-3));
        assert!(proche(f.stockage_wh, -f.phi_mq_seq_ann_w(), 1e-9));
        // La masse séquentielle se rapproche de la moyenne quotidienne sans la dépasser.
        let suivant = e.avancer(&p);
        assert!(suivant.theta_ms_c > f.theta_ms_c && suivant.theta_ms_c < 20.0);
    }
}
