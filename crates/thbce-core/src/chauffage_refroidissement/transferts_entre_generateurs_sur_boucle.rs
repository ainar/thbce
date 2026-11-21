// SPDX-License-Identifier: GPL-3.0-or-later
//! Transferts entre générateurs sur boucle d'eau : fiche `C_Gen_Transferts Entre Generateurs
//! Sur Boucle D'eau` (Th-BCE 2020 §8.15).
//!
//! Une génération « boucle d'eau » comprend des pompes à chaleur sur boucle d'eau (type 507),
//! chacune associée à un groupe, une source amont de type boucle d'eau (température
//! sinusoïdale `θbe(j)`, éq. 1437 et 1438) et un générateur de chauffage et un générateur de
//! froid (ou une tour de refroidissement) qui maintiennent la boucle en température
//! (§8.15.3.1). À chaque pas de temps (figure 105) :
//!
//! 1. les pompes à chaleur de chaque groupe sont appelées en chauffage ou en refroidissement
//!    selon la demande et les saisons du groupe, au prorata de leurs puissances nominales
//!    (éq. 957 à 964) ;
//! 2. le bilan de la boucle compense leurs rejets chauds et froids (éq. 965 à 968) ;
//! 3. le reste est demandé au générateur de chauffage (éq. 969 à 971) et au générateur de
//!    froid ou à la tour (éq. 972 à 977) de la boucle.
//!
//! Ce module porte les fonctions pures de la fiche et l'état de la boucle ; l'algorithme est
//! appliqué par `GenerationHoraire::pas` (`generation/boucle.rs`). Lectures :
//! `docs/questions/Q240.md` à `Q243.md`.

use super::calculs_generation::{Energie, Poste};
use super::generation::{EntreesAppel, SortiesAppel};
use super::sources_amont::{Sinusoide, Tour, C_PE_J_KG_K, RHO_EAU_KG_M3};

/// « Reff_trBE » : ratio d'efficacité du transfert d'énergie sur la boucle, conventionnel
/// (tableau 108, éq. 967).
pub const R_EFF_TR_BE: f64 = 0.8;

/// « Φrejet_totBE_ch » et « Φrejet_totBE_fr » : rejets totaux des pompes à chaleur de la boucle
/// en mode chauffage (froid rejeté, compté positivement) et en mode refroidissement (chaleur
/// rejetée), en Wh, à partir des rejets « Φrejet^gnr(h) » de chaque pompe à chaleur (négatifs
/// en chauffage, positifs en refroidissement).
///
/// L'éq. (965) écrit `Σ MIN(0 ; Φrejet)`, une somme négative, que l'éq. (967) compare à la
/// somme positive de l'éq. (966) : elle est lue en valeur absolue (« rejet de froid total »,
/// tableau 108), lecture Q241 point 1.
// Th-BCE 2020 §8.15.3.4, éq. (965) et (966).
pub fn rejets_totaux_wh(rejets_wh: impl Iterator<Item = f64>) -> [f64; 2] {
    let mut r = [0.0; 2];
    for phi in rejets_wh {
        r[0] -= phi.min(0.0);
        r[1] += phi.max(0.0);
    }
    r
}

/// « Qtransfert_BE(h) », « Qsys_BE_ch(h) » et « Qsys_BE_fr(h) » : énergie transférée entre les
/// pompes à chaleur de la boucle et énergies à fournir par les générateurs de la boucle, en Wh,
/// à partir des rejets totaux `[ch, fr]` de [`rejets_totaux_wh`].
// Th-BCE 2020 §8.15.3.4, éq. (967) et (968).
pub fn bilan_boucle_wh(rejets: [f64; 2]) -> (f64, [f64; 2]) {
    let transfert = rejets[0].min(rejets[1]) * R_EFF_TR_BE;
    (transfert, [rejets[0] - transfert, rejets[1] - transfert])
}

/// « θaval_chaudBE(h) » : température de fonctionnement du générateur de chauffage de la
/// boucle, en °C, proportionnelle à sa charge `(Qsys_BE_ch + Qrep_ch(h − 1)) / Pngen` entre
/// `θbe(j)` et `θmax_rech_BE`. `Pngen` est la puissance nominale installée du générateur, en W,
/// sur le pas d'une heure. Sans puissance, la charge est nulle.
// Th-BCE 2020 §8.15.3.6, éq. (969).
pub fn theta_aval_chaud_c(theta_be_c: f64, theta_max_c: f64, q_wh: f64, pn_w: f64) -> f64 {
    let charge = if pn_w > 0.0 { q_wh / pn_w } else { 0.0 };
    theta_max_c.min(theta_be_c + charge * (theta_max_c - theta_be_c))
}

/// « θaval_froidBE(h) » : température de fonctionnement du générateur de froid de la boucle, en
/// °C, de `θbe(j)` à charge nulle jusqu'à `θmin_fr_BE` à pleine charge.
///
/// L'éq. (975) écrit `MIN[θmin_fr_BE ; …]`, qui vaudrait `θmin_fr_BE` à toute charge
/// inférieure à la pleine charge : la borne est lue `MAX`, symétrique de l'éq. (969)
/// (lecture Q241 point 3).
// Th-BCE 2020 §8.15.3.7.2, éq. (975).
pub fn theta_aval_froid_c(theta_be_c: f64, theta_min_c: f64, q_wh: f64, pn_w: f64) -> f64 {
    let charge = if pn_w > 0.0 { q_wh / pn_w } else { 0.0 };
    theta_min_c.max(theta_be_c + charge * (theta_min_c - theta_be_c))
}

/// Tour de refroidissement d'une boucle d'eau (§8.15.3.7.1), traitée comme un générateur de
/// froid de la génération : fonction de refroidissement, consommation électrique de ses pompes
/// et de ses ventilateurs au prorata de sa charge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TourBoucle {
    /// Nature, approche et consigne de la tour (éq. 1428 à 1435), puissances des pompes et des
    /// ventilateurs.
    pub tour: Tour,
    /// « qv,nom,tour » : débit nominal d'eau, en m³/h.
    pub qv_nom_m3_h: f64,
}

impl TourBoucle {
    /// « Pmax » : puissance maximale de froid de la tour dans les conditions du pas de temps, en
    /// W : l'eau entre dans la tour à la température de la boucle `θbe(j)` et en sort à
    /// `θes_tour` (lecture Q242 point 1), au débit nominal (m³/h converti en m³/s).
    // Th-BCE 2020 §8.15.3.7.1, éq. (972).
    pub fn p_max_w(&self, theta_be_c: f64, theta_es_tour_c: f64) -> f64 {
        self.qv_nom_m3_h / 3600.0
            * RHO_EAU_KG_M3
            * C_PE_J_KG_K
            * (theta_be_c - theta_es_tour_c).max(0.0)
    }

    /// Fonction d'appel de la tour (contrat de la figure 112) : `theta_aval_c` porte la
    /// température de la boucle `θbe(j)` et `theta_amont_c` la température d'eau en sortie de
    /// tour `θes_tour` (éq. 1435) ; le pas de temps dure une heure.
    // Th-BCE 2020 §8.15.3.7.1, éq. (972) à (974).
    pub fn appel(&self, e: &EntreesAppel) -> SortiesAppel {
        if e.fonction != Poste::Refroidissement {
            return SortiesAppel {
                q_rest_wh: e.q_req_wh,
                ..SortiesAppel::default()
            };
        }
        let q_req = e.q_req_wh.max(0.0);
        let p_max = self.p_max_w(e.theta_aval_c, e.theta_amont_c);
        let q_fou = q_req.min(p_max);
        let tau = if p_max > 0.0 { q_fou / p_max } else { 0.0 };
        // Éq. (973).
        let q_cons = (self.tour.p_pompes_w + self.tour.p_vent_w) * tau;
        SortiesAppel {
            energie: Energie::Electricite,
            q_cef_wh: q_cons,
            q_cons_wh: q_cons,
            q_fou_wh: q_fou,
            q_rest_wh: q_req - q_fou,
            tau_charge: tau,
            ..SortiesAppel::default()
        }
    }
}

/// Pompe à chaleur de la boucle (élément de « GBE ») et groupe qu'elle dessert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembreBoucle {
    /// Indice du générateur dans la génération.
    pub generateur: usize,
    /// Rang du groupe desservi dans [`BoucleDEauHoraire::groupes`].
    pub rang_groupe: usize,
}

/// Cumuls annuels du bilan de la boucle, en Wh.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CumulsBoucle {
    /// « Φrejet_totBE_ch » cumulé (froid rejeté par les pompes à chaleur en chauffage).
    pub phi_rejet_tot_ch_wh: f64,
    /// « Φrejet_totBE_fr » cumulé (chaleur rejetée par les pompes à chaleur en
    /// refroidissement).
    pub phi_rejet_tot_fr_wh: f64,
    /// « Qtransfert_BE » cumulé.
    pub q_transfert_wh: f64,
    /// « Qsys_BE_ch » cumulé.
    pub q_sys_ch_wh: f64,
    /// « Qsys_BE_fr » cumulé.
    pub q_sys_fr_wh: f64,
}

/// Boucle d'eau d'une génération, prête pour le calcul horaire.
#[derive(Debug, Clone, PartialEq)]
pub struct BoucleDEauHoraire {
    /// « θbe(j) » : température de l'eau de la boucle (éq. 1437 et 1438).
    pub theta_be: Sinusoide,
    /// « θmax_rech_BE », en °C (éq. 969).
    pub theta_max_rech_c: f64,
    /// « θmin_fr_BE », en °C (éq. 975).
    pub theta_min_fr_c: f64,
    /// Pompes à chaleur de la boucle (« GBE »).
    pub membres: Vec<MembreBoucle>,
    /// Identifiants des groupes desservis, dans l'ordre de leur premier membre.
    pub groupes: Vec<String>,
    /// Indice du générateur de chauffage de la boucle.
    pub chaud: Option<usize>,
    /// Indice du générateur de froid de la boucle ou de la tour.
    pub froid: Option<usize>,
    /// Vrai si le générateur de froid de la boucle est sa tour de refroidissement.
    pub froid_par_tour: bool,
    /// « Qrep_BE_ch^gr(h − 1) » et « Qrep_BE_fr^gr(h − 1) » de chaque groupe, en Wh.
    pub q_rep_groupes_wh: Vec<[f64; 2]>,
    /// « Qrep_ch(h − 1) » et « Qrep_fr(h − 1) » des générateurs de la boucle, en Wh.
    pub q_rep_wh: [f64; 2],
    /// Cumuls annuels.
    pub cumuls: CumulsBoucle,
}

impl BoucleDEauHoraire {
    /// Vrai si le générateur d'indice `i` maintient la boucle en température (générateur de
    /// chauffage, de froid ou tour) : son énergie fournie va à la boucle, pas aux réseaux.
    pub fn maintient(&self, i: usize) -> bool {
        self.chaud == Some(i) || self.froid == Some(i)
    }
}

/// Demande de la génération pour un groupe desservi par la boucle à un pas de temps
/// (éq. 1002 et 1003) et saisons du groupe (§8.15.3.3).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DemandeGroupe {
    /// « Qreq,ch^gen,gr(h) », en Wh.
    pub q_ch_wh: f64,
    /// « Qreq,fr^gen,gr(h) », en Wh (positive).
    pub q_fr_wh: f64,
    /// « Autch(j) » du groupe.
    pub aut_ch: bool,
    /// « Autfr(j) » du groupe.
    pub aut_fr: bool,
}

/// Mode des pompes à chaleur d'un groupe au pas de temps (§8.15.3.3) : `None` en mi-saison
/// (générateurs désactivés), sinon le poste pour lequel elles sont appelées, à charge nulle ou
/// non. Les conditions du texte se recouvrent (saisons de chauffage et de refroidissement
/// ouvertes avec une demande de froid) : elles sont prises dans l'ordre du texte, le
/// refroidissement d'abord (lecture Q241 point 4).
// Th-BCE 2020 §8.15.3.3, éq. (959) et (962).
pub fn mode_groupe(d: &DemandeGroupe) -> Option<Poste> {
    if !d.aut_ch && !d.aut_fr {
        None
    } else if d.aut_fr && (d.q_fr_wh > 0.0 || !d.aut_ch) {
        Some(Poste::Refroidissement)
    } else if d.aut_ch {
        Some(Poste::Chauffage)
    } else {
        None
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::chauffage_refroidissement::sources_amont::Tour;
    use crate::entree::NatureTourRefroidissement;

    fn proche(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} ≠ {b}");
    }

    #[test]
    fn bilan_de_la_boucle_a_la_main() {
        // Trois PAC : deux en chauffage (rejets −1 500 et −500 Wh), une en refroidissement
        // (+1 200 Wh). Éq. (965) et (966) : 2 000 et 1 200 Wh ; éq. (967) :
        // min(2 000 ; 1 200) × 0,8 = 960 Wh ; éq. (968) : 1 040 et 240 Wh.
        let r = rejets_totaux_wh([-1500.0, 1200.0, -500.0].into_iter());
        assert_eq!(r, [2000.0, 1200.0]);
        let (t, sys) = bilan_boucle_wh(r);
        proche(t, 960.0);
        proche(sys[0], 1040.0);
        proche(sys[1], 240.0);
        // Rejets d'un seul signe : pas de transfert.
        let (t, sys) = bilan_boucle_wh(rejets_totaux_wh([-800.0, 0.0].into_iter()));
        assert_eq!((t, sys), (0.0, [800.0, 0.0]));
    }

    #[test]
    fn temperatures_de_fonctionnement_a_la_main() {
        // Éq. (969) : boucle à 18 °C, θmax_rech 35 °C, 3 000 Wh demandés à 10 kW :
        // 18 + 0,3 × 17 = 23,1 °C ; au-delà de la pleine charge, 35 °C.
        proche(theta_aval_chaud_c(18.0, 35.0, 3000.0, 10_000.0), 23.1);
        proche(theta_aval_chaud_c(18.0, 35.0, 12_000.0, 10_000.0), 35.0);
        proche(theta_aval_chaud_c(18.0, 35.0, 0.0, 10_000.0), 18.0);
        // Éq. (975), lue MAX : boucle à 25 °C, θmin_fr 10 °C, 4 000 Wh à 8 kW :
        // 25 − 0,5 × 15 = 17,5 °C ; au-delà, 10 °C.
        proche(theta_aval_froid_c(25.0, 10.0, 4000.0, 8000.0), 17.5);
        proche(theta_aval_froid_c(25.0, 10.0, 16_000.0, 8000.0), 10.0);
    }

    #[test]
    fn tour_de_la_boucle_a_la_main() {
        let t = TourBoucle {
            tour: Tour {
                nature: NatureTourRefroidissement::Seche,
                delta_tour_k: 5.0,
                consigne_c: 20.0,
                p_pompes_w: 300.0,
                p_vent_w: 500.0,
            delta_cond_fr_k: 0.0,
            },
            qv_nom_m3_h: 3.6,
        };
        // Éq. (972) : 3,6 m³/h = 0,001 m³/s ; 1 × 4 180 × (30 − 22) = 33 440 W.
        proche(t.p_max_w(30.0, 22.0), 33_440.0);
        let e = EntreesAppel {
            theta_amont_c: 22.0,
            theta_aval_c: 30.0,
            ..EntreesAppel::demande(Poste::Refroidissement, 16_720.0)
        };
        let s = t.appel(&e);
        // τ = 16 720 / 33 440 = 0,5 ; éq. (973) : (300 + 500) × 0,5 = 400 Wh d'électricité.
        proche(s.q_fou_wh, 16_720.0);
        proche(s.tau_charge, 0.5);
        proche(s.q_cons_wh, 400.0);
        proche(s.q_cef_wh, 400.0);
        assert_eq!(s.energie, Energie::Electricite);
        assert_eq!(s.q_rest_wh, 0.0);
        // Demande supérieure à Pmax : reste reporté.
        let s = t.appel(&EntreesAppel {
            q_req_wh: 40_000.0,
            ..e
        });
        proche(s.q_rest_wh, 40_000.0 - 33_440.0);
        proche(s.q_cons_wh, 800.0);
        // Eau de sortie de tour plus chaude que la boucle : la tour ne refroidit pas.
        let s = t.appel(&EntreesAppel {
            theta_amont_c: 31.0,
            ..e
        });
        assert_eq!((s.q_fou_wh, s.q_cons_wh, s.q_rest_wh), (0.0, 0.0, 16_720.0));
    }

    #[test]
    fn mode_des_pompes_a_chaleur_d_un_groupe() {
        let d = |q_ch, q_fr, aut_ch, aut_fr| DemandeGroupe {
            q_ch_wh: q_ch,
            q_fr_wh: q_fr,
            aut_ch,
            aut_fr,
        };
        assert_eq!(mode_groupe(&d(100.0, 0.0, false, false)), None);
        assert_eq!(
            mode_groupe(&d(0.0, 100.0, true, true)),
            Some(Poste::Refroidissement)
        );
        assert_eq!(
            mode_groupe(&d(0.0, 0.0, false, true)),
            Some(Poste::Refroidissement)
        );
        assert_eq!(mode_groupe(&d(0.0, 0.0, true, true)), Some(Poste::Chauffage));
        assert_eq!(mode_groupe(&d(50.0, 0.0, true, false)), Some(Poste::Chauffage));
        assert_eq!(mode_groupe(&d(0.0, 0.0, true, false)), Some(Poste::Chauffage));
    }
}
