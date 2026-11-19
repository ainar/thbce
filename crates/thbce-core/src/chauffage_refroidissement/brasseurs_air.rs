// SPDX-License-Identifier: GPL-3.0-or-later
//! Brasseurs d'air : fiche `C_BAT_Fonctionnement/gestion/régulation des brasseurs d'air`
//! (Th-BCE 2020 §8.32, éq. 1646 à 1666).
//!
//! Fonctions pures : valeurs corrigées d'un type de brasseur (§8.32.3.2.1), consignes de
//! l'hystérésis (§8.32.3.3.1), débit brassé à un pas de temps (§8.32.3.3.3), effet sur la
//! température opérative (§8.32.3.4) et consommation (§8.32.3.5). Le groupe porte l'état
//! (`Qv(h−1)` de chaque type) et appelle ces fonctions à chaque pas de temps où les brasseurs
//! sont autorisés. Lectures : `docs/questions/Q120.md`.

use crate::entree::{
    BrasseurAir, CaractereValeurBrasseur, GestionBrasseur, UsageBrasseur, UsageZone,
};

/// Tableau 258 : « Δθop_1_br_man » en usage jour et en usage nuit, en K.
pub const DELTA_THETA_OP_1_MAN_K: f64 = 2.0;
/// Tableau 258 : « Δθop_2_br_man », en K.
pub const DELTA_THETA_OP_2_MAN_K: f64 = 4.0;
/// Tableau 258 : « Δθop_3_br_man », en K.
pub const DELTA_THETA_OP_3_MAN_K: f64 = 1.0;

/// Tableau 257 : `RatUs_BA` par défaut, `(jour, nuit)`, selon le numéro d'usage de la zone (1 à
/// 28).
const RAT_US_DEFAUT: [(f64, f64); 28] = [
    (0.6, 0.4),
    (0.54, 0.36),
    (0.7, 0.0),
    (0.85, 0.0),
    (0.75, 0.0),
    (0.75, 0.0),
    (0.75, 0.0),
    (0.728, 0.728),
    (0.728, 0.728),
    (0.5179, 0.0),
    (0.79, 0.0),
    (0.728, 0.0),
    (0.7, 0.7),
    (0.7, 0.0),
    (0.7, 0.0),
    (0.7, 0.0),
    (0.93, 0.0),
    (0.75, 0.0),
    (0.6, 0.5),
    (0.75, 0.45),
    (0.65, 0.0),
    (1.0, 0.0),
    (0.7, 0.6),
    (0.7, 0.0),
    (0.75, 0.0),
    (0.7, 0.0),
    (0.7, 0.0),
    (0.75, 0.0),
];

/// Tableau 257 et remarque de §8.32.3.2.3 : `RatUs_BA` par défaut d'un type de brasseur.
/// 1 pour un usage « jour et nuit » ; la valeur « jour » ou « nuit » de la ligne de l'usage de
/// la zone sinon.
pub fn rat_us_defaut(usage_zone: UsageZone, usage: UsageBrasseur) -> f64 {
    let (jour, nuit) = RAT_US_DEFAUT[usize::from(usage_zone.numero() - 1)];
    match usage {
        UsageBrasseur::Jour => jour,
        UsageBrasseur::Nuit => nuit,
        UsageBrasseur::JourEtNuit => 1.0,
    }
}

/// Éq. de §8.32.3.2.1 : `Pelec_br_p_max_corr` (×1 certifiée, ×1,1 justifiée, ×1,2 déclarée).
pub fn p_elec_max_corr_w(p_w: f64, c: CaractereValeurBrasseur) -> f64 {
    p_w * match c {
        CaractereValeurBrasseur::Certifiee => 1.0,
        CaractereValeurBrasseur::Justifiee => 1.1,
        CaractereValeurBrasseur::Declaree => 1.2,
    }
}

/// Éq. de §8.32.3.2.1 : `Qvair_br_p_max_corr` (×1 certifié, ×0,9 justifié, ×0,8 déclaré).
pub fn q_v_max_corr_m3_h(q: f64, c: CaractereValeurBrasseur) -> f64 {
    q * match c {
        CaractereValeurBrasseur::Certifiee => 1.0,
        CaractereValeurBrasseur::Justifiee => 0.9,
        CaractereValeurBrasseur::Declaree => 0.8,
    }
}

/// Consignes de l'hystérésis `(θarr, θV1, θV2)`, en °C.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Consignes {
    /// « θarr » : en dessous, les brasseurs sont arrêtés.
    pub arret_c: f64,
    /// « θV1 » : au-dessus, vitesse au moins intermédiaire.
    pub v1_c: f64,
    /// « θV2 » : au-dessus, vitesse maximale.
    pub v2_c: f64,
}

/// Type de brasseur résolu pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeBrasseur {
    /// Identifiant.
    pub id: String,
    /// « Nbr_p ».
    pub nombre: f64,
    /// « Ratbr_p » après limitation à `15·Nbr_p/Agr`.
    pub rat_br: f64,
    /// « Usp ».
    pub usage: UsageBrasseur,
    /// « RatUs_BA_p ».
    pub rat_us: f64,
    /// « Pelec_br_p_max_corr », en W.
    pub p_max_corr_w: f64,
    /// « Pelec_br_p_int » : 60 % de la puissance maximale corrigée, en W.
    pub p_int_w: f64,
    /// « Qvair_br_p_max_corr », en m³/h.
    pub q_max_corr_m3_h: f64,
    /// « Qvair_br_p_int » : 60 % du débit maximal corrigé, en m³/h.
    pub q_int_m3_h: f64,
    /// Gestion de la vitesse.
    pub gestion: GestionBrasseur,
}

impl TypeBrasseur {
    /// Résout un type saisi ; `a_gr_m2` : surface utile du groupe ; `rat_us` : valeur saisie ou
    /// par défaut.
    pub fn nouveau(b: &BrasseurAir, a_gr_m2: f64, rat_us: f64) -> Self {
        let n = f64::from(b.nombre);
        // §8.32.3.2.3 : surface unitaire impactée au plus 15 m².
        let rat_br = if a_gr_m2 > 0.0 {
            (15.0 * n / a_gr_m2).min(b.ratio_surface_brassee)
        } else {
            b.ratio_surface_brassee
        };
        let p_max = p_elec_max_corr_w(b.p_elec_max_w, b.p_elec_max_caractere);
        let q_max = q_v_max_corr_m3_h(b.q_v_max_m3_h, b.q_v_max_caractere);
        Self {
            id: b.id.clone(),
            nombre: n,
            rat_br,
            usage: b.usage,
            rat_us,
            p_max_corr_w: p_max,
            p_int_w: 0.6 * p_max,
            q_max_corr_m3_h: q_max,
            q_int_m3_h: 0.6 * q_max,
            gestion: b.gestion.clone(),
        }
    }

    /// Consignes `(θarr, θV1, θV2)` de l'hystérésis (éq. 1646 à 1650). `theta_ii_fr_plus_c` :
    /// consigne de refroidissement en occupation ; `delta_conf_adapt_c` : « Δθconf_adapt ».
    pub fn consignes(&self, theta_ii_fr_plus_c: f64, delta_conf_adapt_c: f64) -> Consignes {
        let ref_c = theta_ii_fr_plus_c + delta_conf_adapt_c;
        match &self.gestion {
            // Éq. (1646) et (1648).
            GestionBrasseur::Manuelle => {
                let dec = ref_c + DELTA_THETA_OP_3_MAN_K;
                Consignes {
                    arret_c: dec - DELTA_THETA_OP_1_MAN_K,
                    v1_c: dec,
                    v2_c: dec + DELTA_THETA_OP_2_MAN_K,
                }
            }
            // Éq. (1647) et (1649).
            GestionBrasseur::AutomatiqueThermostat(p) => {
                let dec = ref_c + p.delta_theta_op_3_k;
                Consignes {
                    arret_c: dec - p.delta_theta_op_1_k,
                    v1_c: dec,
                    v2_c: dec + p.delta_theta_op_2_k,
                }
            }
            // Éq. (1650).
            GestionBrasseur::Automatique(p) => {
                let d3 = p.delta_theta_op_3_k.unwrap_or(0.0);
                Consignes {
                    arret_c: p.theta_op_dec_c + d3 - p.delta_theta_op_1_k,
                    v1_c: p.theta_op_dec_c + d3,
                    v2_c: p.theta_op_dec_c + d3 + p.delta_theta_op_2_k,
                }
            }
        }
    }

    /// Débit brassé d'un brasseur du type au pas de temps `h`, en m³/h (§8.32.3.3.3, éq. 1651
    /// à 1654).
    ///
    /// `hleg` : heure légale de 0 à 23 ; `i_occ_zone` : zone occupée ; `theta_op_fin_h1_c` :
    /// « θop,fin(h−1) » ; `qv_prec` : « Qvair_br_p(h−1) » ; `hebergement` : « ihébergement ».
    pub fn debit_m3_h(
        &self,
        hleg: u8,
        i_occ_zone: bool,
        consignes: Consignes,
        theta_op_fin_h1_c: f64,
        qv_prec_m3_h: f64,
        hebergement: bool,
    ) -> f64 {
        // Éq. (1651) à (1653).
        let de_jour = hleg > 6 && hleg <= 22;
        let hors_plage = match self.usage {
            UsageBrasseur::Jour => !de_jour,
            UsageBrasseur::Nuit => de_jour,
            UsageBrasseur::JourEtNuit => false,
        };
        if !i_occ_zone || hors_plage {
            return 0.0;
        }
        // Éq. (1654).
        let mut qv = if theta_op_fin_h1_c < consignes.arret_c {
            0.0
        } else if theta_op_fin_h1_c < consignes.v1_c {
            qv_prec_m3_h
        } else if theta_op_fin_h1_c < consignes.v2_c {
            self.q_int_m3_h.max(qv_prec_m3_h)
        } else {
            self.q_max_corr_m3_h
        };
        // Période « nuit » en résidentiel et en hébergement (lecture Q120 point 3) : vitesse
        // maximale interdite (acoustique) et, en gestion manuelle, débit bloqué à celui de 23 h.
        let nuit = hleg > 22 || hleg <= 6;
        if self.usage != UsageBrasseur::Jour && hebergement && nuit {
            qv = qv.min(self.q_int_m3_h);
            if matches!(self.gestion, GestionBrasseur::Manuelle) && hleg <= 6 {
                qv = qv_prec_m3_h;
            }
        }
        qv
    }

    /// Puissance électrique d'un brasseur du type pour le débit `qv`, en W (éq. 1662).
    pub fn puissance_w(&self, qv_m3_h: f64) -> f64 {
        if qv_m3_h <= 0.0 {
            0.0
        } else if (qv_m3_h - self.q_int_m3_h).abs() < 1e-9 {
            self.p_int_w
        } else {
            self.p_max_corr_w
        }
    }

    /// « Δθop^BA_p(h) » (éq. 1655 à 1659), en K : effet de l'ensemble des brasseurs du type, de
    /// débit individuel `qv`, dans un groupe de volume `vol_m3`, avec `θrm(h−1) − θi(h−1)`.
    pub fn delta_theta_op_k(&self, qv_m3_h: f64, vol_m3: f64, ecart_rm_i_h1_k: f64) -> f64 {
        if qv_m3_h <= 0.0 || vol_m3 * self.rat_br <= 0.0 {
            return 0.0;
        }
        // Éq. (1656) à (1658).
        let tau = qv_m3_h * self.nombre / (vol_m3 * self.rat_br);
        let v = 0.0032 * tau;
        if v <= 0.2 {
            return 0.0;
        }
        // Éq. (1659).
        (1.8322 * libm::exp(0.0361 * ecart_rm_i_h1_k)) * libm::log(v)
            + 3.0498 * libm::exp(0.0368 * ecart_rm_i_h1_k)
    }
}

/// « Δθop^BA(h) » global du groupe (éq. 1660 et 1661), en K.
///
/// `deltas[p]` : « Δθop^BA_p(h) » de chaque type ; `hleg` : heure légale ; la moyenne des
/// types d'un même usage est pondérée par `Ratbr_p` et ramenée au plus petit `RatUs_BA` des
/// types de cet usage.
pub fn delta_theta_op_global_k(types: &[TypeBrasseur], deltas: &[f64], hleg: u8) -> f64 {
    let classe = |u: UsageBrasseur| -> f64 {
        let mut somme = 0.0;
        let mut rat_us_min = f64::INFINITY;
        for (t, d) in types.iter().zip(deltas) {
            if t.usage == u {
                somme += t.rat_br * d;
                rat_us_min = rat_us_min.min(t.rat_us);
            }
        }
        if rat_us_min.is_finite() && rat_us_min > 0.0 {
            somme / rat_us_min
        } else {
            0.0
        }
    };
    let jour = classe(UsageBrasseur::Jour) + classe(UsageBrasseur::JourEtNuit);
    let nuit = classe(UsageBrasseur::Nuit) + classe(UsageBrasseur::JourEtNuit);
    // Éq. (1661).
    if hleg > 6 && hleg <= 22 {
        jour
    } else {
        nuit
    }
}

/// Sorties des brasseurs d'un groupe à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SortiesBrasseurs {
    /// « Wabs_BA^gr(h) » : consommation électrique des brasseurs du groupe, en Wh (éq. 1665).
    pub w_wh: f64,
    /// « Φth_BA_vc^gr(h) » : flux de chaleur vers l'ambiance associé, en Wh (éq. 1666).
    pub phi_vc_wh: f64,
    /// « Δθop^BA(h) » : impact sur la température opérative, en K (éq. 1661).
    pub delta_theta_op_k: f64,
    /// Vrai si au moins un brasseur tourne.
    pub en_marche: bool,
}

/// Cumuls annuels des brasseurs d'un groupe.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CumulsBrasseurs {
    /// Consommation électrique, en Wh.
    pub w_wh: f64,
    /// Nombre d'heures où au moins un brasseur tourne.
    pub nbh_marche: u32,
    /// Effet maximal sur la température opérative, en K.
    pub delta_theta_op_max_k: f64,
}

/// Brasseurs d'un groupe : types résolus, débits du pas précédent et cumuls.
#[derive(Debug, Clone, PartialEq)]
pub struct BrasseursGroupe {
    /// Types de brasseurs.
    pub types: Vec<TypeBrasseur>,
    /// « Qvair_br_p(h−1) » de chaque type, en m³/h.
    pub qv_precedent_m3_h: Vec<f64>,
    /// Sorties du dernier pas calculé.
    pub derniere: SortiesBrasseurs,
    /// Cumuls annuels.
    pub cumuls: CumulsBrasseurs,
}

/// Conditions d'un pas de temps pour les brasseurs d'un groupe.
#[derive(Debug, Clone, Copy)]
pub struct ConditionsBrasseurs {
    /// Heure légale, de 0 à 23.
    pub hleg: u8,
    /// Brasseurs autorisés : période de confort adaptatif en Th-D, saison de refroidissement
    /// propre (« Aut_fr_pro ») en Th-C (§8.32.3.1).
    pub autorises: bool,
    /// « iocc_zone(h) ».
    pub i_occ_zone: bool,
    /// « θiifr+(h) », en °C.
    pub theta_ii_fr_plus_c: f64,
    /// « Δθconf_adapt(h) », en °C.
    pub delta_conf_adapt_c: f64,
    /// « θop,fin(h−1) », en °C.
    pub theta_op_fin_h1_c: f64,
    /// « θi(h−1) », en °C.
    pub theta_i_h1_c: f64,
    /// « θrm(h−1) », en °C.
    pub theta_rm_h1_c: f64,
    /// « Vol » : volume du groupe, en m³.
    pub vol_m3: f64,
    /// « ihébergement ».
    pub hebergement: bool,
}

impl BrasseursGroupe {
    /// Brasseurs d'un groupe ; `rat_us` donne `RatUs_BA` de chaque type saisi (valeur saisie ou
    /// par défaut).
    pub fn nouveau(
        saisis: &[BrasseurAir],
        a_gr_m2: f64,
        rat_us: impl Fn(&BrasseurAir) -> f64,
    ) -> Self {
        let types: Vec<TypeBrasseur> = saisis
            .iter()
            .map(|b| TypeBrasseur::nouveau(b, a_gr_m2, rat_us(b)))
            .collect();
        Self {
            qv_precedent_m3_h: vec![0.0; types.len()],
            types,
            derniere: SortiesBrasseurs::default(),
            cumuls: CumulsBrasseurs::default(),
        }
    }

    /// Pas de temps : débits, effet sur la température opérative et consommation
    /// (§8.32.3.3 à 8.32.3.5, éq. 1651 à 1666).
    pub fn pas(&mut self, c: &ConditionsBrasseurs) -> SortiesBrasseurs {
        let mut s = SortiesBrasseurs::default();
        if !c.autorises {
            self.qv_precedent_m3_h.fill(0.0);
            self.derniere = s;
            return s;
        }
        let ecart = c.theta_rm_h1_c - c.theta_i_h1_c;
        let mut deltas = Vec::with_capacity(self.types.len());
        for (t, prec) in self.types.iter().zip(self.qv_precedent_m3_h.iter_mut()) {
            let consignes = t.consignes(c.theta_ii_fr_plus_c, c.delta_conf_adapt_c);
            let qv = t.debit_m3_h(
                c.hleg,
                c.i_occ_zone,
                consignes,
                c.theta_op_fin_h1_c,
                *prec,
                c.hebergement,
            );
            *prec = qv;
            deltas.push(t.delta_theta_op_k(qv, c.vol_m3, ecart));
            // Éq. (1662) à (1666), `Wabs_BA_p` lue pour l'ensemble des `Nbr_p` brasseurs (Q120
            // point 5).
            s.w_wh += t.puissance_w(qv) * t.nombre;
            s.en_marche |= qv > 0.0;
        }
        s.phi_vc_wh = s.w_wh;
        s.delta_theta_op_k = delta_theta_op_global_k(&self.types, &deltas, c.hleg);
        self.derniere = s;
        self.cumuls.w_wh += s.w_wh;
        self.cumuls.nbh_marche += u32::from(s.en_marche);
        self.cumuls.delta_theta_op_max_k = self.cumuls.delta_theta_op_max_k.max(s.delta_theta_op_k);
        s
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]
    use super::*;
    use crate::entree::{ParametresHysteresisBrasseur, UsageZone};

    fn brasseur(usage: UsageBrasseur, gestion: GestionBrasseur) -> BrasseurAir {
        BrasseurAir {
            id: "b".into(),
            nombre: 4,
            ratio_surface_brassee: 0.9,
            usage,
            ratio_surface_occupee: None,
            p_elec_max_w: 50.0,
            p_elec_max_caractere: CaractereValeurBrasseur::Declaree,
            q_v_max_m3_h: 5000.0,
            q_v_max_caractere: CaractereValeurBrasseur::Justifiee,
            gestion,
        }
    }

    #[test]
    fn valeurs_corrigees_a_la_main() {
        // 50 × 1,2 = 60 W ; 5 000 × 0,9 = 4 500 m³/h ; 60 % : 36 W et 2 700 m³/h.
        let t = TypeBrasseur::nouveau(
            &brasseur(UsageBrasseur::Jour, GestionBrasseur::Manuelle),
            100.0,
            0.7,
        );
        assert!((t.p_max_corr_w - 60.0).abs() < 1e-9);
        assert!((t.p_int_w - 36.0).abs() < 1e-9);
        assert!((t.q_max_corr_m3_h - 4500.0).abs() < 1e-9);
        assert!((t.q_int_m3_h - 2700.0).abs() < 1e-9);
        // 15 × 4 / 100 = 0,6 < 0,9 : surface brassée limitée.
        assert!((t.rat_br - 0.6).abs() < 1e-12);
    }

    #[test]
    fn rat_us_du_tableau_257() {
        assert_eq!(
            rat_us_defaut(UsageZone::MaisonIndividuelle, UsageBrasseur::Jour),
            0.6
        );
        assert_eq!(
            rat_us_defaut(UsageZone::MaisonIndividuelle, UsageBrasseur::Nuit),
            0.4
        );
        assert_eq!(
            rat_us_defaut(UsageZone::MaisonIndividuelle, UsageBrasseur::JourEtNuit),
            1.0
        );
        assert_eq!(
            rat_us_defaut(UsageZone::Hotel012Jour, UsageBrasseur::Jour),
            0.5179
        );
    }

    #[test]
    fn consignes_manuelles() {
        // θiifr+ = 26, Δθconf_adapt = 1 : déclenchement 26 + 1 + 1 = 28 ; V2 = 32 ; arrêt = 26.
        let t = TypeBrasseur::nouveau(
            &brasseur(UsageBrasseur::Jour, GestionBrasseur::Manuelle),
            100.0,
            0.7,
        );
        let c = t.consignes(26.0, 1.0);
        assert_eq!((c.arret_c, c.v1_c, c.v2_c), (26.0, 28.0, 32.0));
    }

    #[test]
    fn hysteresis_du_debit() {
        let t = TypeBrasseur::nouveau(
            &brasseur(
                UsageBrasseur::Jour,
                GestionBrasseur::AutomatiqueThermostat(ParametresHysteresisBrasseur {
                    delta_theta_op_1_k: 2.0,
                    delta_theta_op_2_k: 3.0,
                    delta_theta_op_3_k: 0.0,
                }),
            ),
            100.0,
            0.7,
        );
        // θiifr+ = 26 : arrêt 24, V1 26, V2 29.
        let c = t.consignes(26.0, 0.0);
        assert_eq!((c.arret_c, c.v1_c, c.v2_c), (24.0, 26.0, 29.0));
        let d = |th, prec| t.debit_m3_h(12, true, c, th, prec, false);
        assert_eq!(d(23.0, 4500.0), 0.0);
        assert_eq!(d(25.0, 2700.0), 2700.0, "hystérésis : on garde le débit");
        assert_eq!(d(25.0, 0.0), 0.0);
        assert_eq!(d(27.0, 0.0), 2700.0, "vitesse intermédiaire");
        assert_eq!(d(27.0, 4500.0), 4500.0);
        assert_eq!(d(30.0, 0.0), 4500.0);
        // Inoccupation, ou hors de la plage « jour ».
        assert_eq!(t.debit_m3_h(12, false, c, 30.0, 0.0, false), 0.0);
        assert_eq!(t.debit_m3_h(23, true, c, 30.0, 0.0, false), 0.0);
        assert_eq!(t.debit_m3_h(6, true, c, 30.0, 0.0, false), 0.0);
        assert_eq!(t.debit_m3_h(7, true, c, 30.0, 0.0, false), 4500.0);
    }

    #[test]
    fn nuit_en_hebergement() {
        let mut b = brasseur(UsageBrasseur::JourEtNuit, GestionBrasseur::Manuelle);
        let t = TypeBrasseur::nouveau(&b, 100.0, 1.0);
        let c = t.consignes(26.0, 0.0);
        // Nuit en hébergement : plafonné à la vitesse intermédiaire.
        assert_eq!(t.debit_m3_h(23, true, c, 40.0, 0.0, true), 2700.0);
        // Gestion manuelle de 0 h à 6 h : débit de la veille conservé.
        assert_eq!(t.debit_m3_h(3, true, c, 40.0, 1000.0, true), 1000.0);
        // Hors hébergement : pas de plafond.
        assert_eq!(t.debit_m3_h(23, true, c, 40.0, 0.0, false), 4500.0);
        b.gestion = GestionBrasseur::Manuelle;
    }

    #[test]
    fn effet_sur_la_temperature_operative() {
        // 4 brasseurs de 2 700 m³/h dans 300 m³, Ratbr 0,6 : τ = 10 800 / (300·0,6) = 60 vol/h,
        // v = 0,0032 × 60 = 0,192 ≤ 0,2 : aucun effet.
        let t = TypeBrasseur::nouveau(
            &brasseur(UsageBrasseur::Jour, GestionBrasseur::Manuelle),
            100.0,
            0.7,
        );
        assert_eq!(t.delta_theta_op_k(2700.0, 300.0, 1.0), 0.0);
        // 4 500 m³/h : τ = 18 000 / 180 = 100, v = 0,32 ; θrm − θi = 1 :
        // 1,8322·e^0,0361·ln 0,32 + 3,0498·e^0,0368
        // = 1,8322 × 1,0367595 × (−1,1394343) + 3,0498 × 1,0374855 ≈ 0,9997
        let attendu = 1.8322 * 1.036_759_5 * -1.139_434_3 + 3.0498 * 1.037_485_5;
        let d = t.delta_theta_op_k(4500.0, 300.0, 1.0);
        assert!((d - attendu).abs() < 1e-4);
        assert!((d - 1.0).abs() < 0.01, "environ 1,0 K, d = {d}");
    }

    #[test]
    fn effet_global_jour_et_nuit() {
        let jour = TypeBrasseur::nouveau(
            &brasseur(UsageBrasseur::Jour, GestionBrasseur::Manuelle),
            100.0,
            0.6,
        );
        let nuit = TypeBrasseur::nouveau(
            &brasseur(UsageBrasseur::Nuit, GestionBrasseur::Manuelle),
            100.0,
            0.4,
        );
        let types = [jour, nuit];
        // Ratbr = 0,6 chacun : jour 0,6 × 2 / 0,6 = 2 ; nuit 0,6 × 3 / 0,4 = 4,5.
        let deltas = [2.0, 3.0];
        assert!((delta_theta_op_global_k(&types, &deltas, 12) - 2.0).abs() < 1e-12);
        assert!((delta_theta_op_global_k(&types, &deltas, 2) - 4.5).abs() < 1e-12);
    }
}
