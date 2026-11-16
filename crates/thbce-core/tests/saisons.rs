// SPDX-License-Identifier: GPL-3.0-or-later
//! Saisons propres d'un groupe (Th-BCE 2020 §4.6) : scénarios déroulés à la main.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::scenarios::saisons::{Journee, Saison, Saisons};

/// Groupe de 100 m², occupé en permanence (168 h par semaine type).
fn saisons(i_clim: bool) -> Saisons {
    Saisons::nouveau(100.0, 168.0, i_clim)
}

/// Déroule une année. `journee(d)` donne le cumul de la journée close à 9 h le jour `d`
/// (de 9 h le jour `d − 1` à 9 h le jour `d`). Renvoie les autorisations de chaque jour.
fn annee(
    s: &mut Saisons,
    journee: impl Fn(u16) -> Journee,
    confort_adaptatif: impl Fn(u16) -> bool,
) -> Vec<(bool, bool)> {
    (1..=365)
        .map(|j| {
            s.accumuler(&journee(j));
            s.nouveau_jour(j, confort_adaptatif(j));
            (s.aut_ch, s.aut_fr)
        })
        .collect()
}

fn occupe() -> Journee {
    Journee {
        nbh_occ: 24.0,
        ..Journee::default()
    }
}

#[test]
fn correspondance_des_autorisations_et_des_saisons() {
    // Tableau 11.
    assert_eq!(Saison::de(false, true), Saison::Refroidissement);
    assert_eq!(Saison::de(false, false), Saison::MiSaison);
    assert_eq!(Saison::de(true, false), Saison::Chauffage);
    assert_eq!(Saison::de(true, true), Saison::Mixte);
    assert_eq!(Saison::Refroidissement as u8, 1);
    assert_eq!(Saison::Mixte as u8, 4);
    // État initial : chauffage autorisé, refroidissement interdit.
    assert_eq!(saisons(true).saison(), Saison::Chauffage);
}

#[test]
fn annee_sans_besoin_ni_inconfort() {
    // Chauffage : autorisé les 56 premiers jours (éq. 76) ; au jour 57, la moyenne des besoins
    // est nulle et le chauffage fonctionne depuis plus de sept jours : arrêt (éq. 78). Jamais de
    // redémarrage faute d'inconfort froid.
    // Refroidissement : interdit jusqu'au jour 182 faute d'inconfort chaud, autorisé du jour 183
    // au jour 245 (éq. 89), arrêté au jour 246 puisque les besoins sont nuls (éq. 90).
    let mut s = saisons(true);
    let a = annee(&mut s, |_| occupe(), |_| false);
    for (i, &(ch, fr)) in a.iter().enumerate() {
        let j = i + 1;
        assert_eq!(ch, j <= 56, "chauffage, jour {j}");
        assert_eq!(fr, (183..=245).contains(&j), "refroidissement, jour {j}");
    }
    assert_eq!((s.j_fin_ch, s.j_debut_ch), (Some(57), None));
    assert_eq!((s.j_debut_fr, s.j_fin_fr), (Some(183), Some(246)));
    // Sans système de refroidissement : jamais autorisé (§4.6.3.3).
    let mut s = saisons(false);
    assert!(annee(&mut s, |_| occupe(), |_| false)
        .iter()
        .all(|&(_, fr)| !fr));
}

#[test]
fn arret_redemarrage_et_reprise_du_chauffage() {
    // Besoins de 10 Wh/m² par heure (24 000 Wh par jour pour 100 m²) jusqu'à la journée 100, nuls
    // ensuite. La moyenne sur 28 jours passe sous 2 Wh/m² quand il reste au plus 5 journées
    // chauffées dans la fenêtre : 10 × 5/28 = 1,79 au jour 123 (2,14 au jour 122). Arrêt au jour 123.
    // Inconfort froid de 30 °C.h le jour 130. Le calcul de MGdh_ch commence le deuxième jour
    // après l'arrêt (jour 125) : au jour 130, 6 jours et 144 h d'occupation ; le seuil est
    // 40 × max(0,5 ; 144/672) = 20 °C.h ; 30 ≥ 20 : redémarrage au jour 130.
    // Besoins nuls : nouvel arrêt après sept jours de fonctionnement, au jour 137 ; définitif,
    // un seul redémarrage étant admis (éq. 67), malgré l'inconfort du jour 150.
    // À l'automne, 30 °C.h par jour à partir du jour 280 : fenêtre de 28 jours occupés, seuil de
    // 40 °C.h ; 30 au jour 280, 60 au jour 281 : démarrage au jour 281, jusqu'à la fin de l'année.
    let journee = |d: u16| Journee {
        q_ch_wh: if d <= 100 { 24_000.0 } else { 0.0 },
        dh_ch_c_h: if d == 130 || d == 150 || d >= 280 {
            30.0
        } else {
            0.0
        },
        ..occupe()
    };
    let mut s = saisons(false);
    let a = annee(&mut s, journee, |_| false);
    for (i, &(ch, _)) in a.iter().enumerate() {
        let j = i + 1;
        let attendu = j <= 122 || (130..=136).contains(&j) || j >= 281;
        assert_eq!(ch, attendu, "jour {j}");
    }
    assert_eq!((s.j_fin_ch, s.j_debut_ch), (Some(137), Some(281)));
}

#[test]
fn demarrage_et_arret_du_refroidissement() {
    // Inconfort chaud de 50 °C.h le jour 150 : fenêtre de 28 jours occupés, seuil de 40 °C.h ;
    // démarrage au jour 150 (éq. 88), maintenu ensuite.
    // Besoins de froid de 10 Wh/m² par heure jusqu'à la journée 260 : la moyenne sur 28 jours
    // passe sous 2 Wh/m² au jour 283 (5 journées restantes dans la fenêtre). Arrêt au jour 283.
    let journee = |d: u16| Journee {
        dh_fr_c_h: if d == 150 { 50.0 } else { 0.0 },
        q_fr_wh: if d <= 260 { 24_000.0 } else { 0.0 },
        ..occupe()
    };
    let mut s = saisons(true);
    let a = annee(&mut s, journee, |_| false);
    for (i, &(_, fr)) in a.iter().enumerate() {
        let j = i + 1;
        assert_eq!(fr, (150..=282).contains(&j), "jour {j}");
    }
    assert_eq!((s.j_debut_fr, s.j_fin_fr), (Some(150), Some(283)));
    // Un inconfort insuffisant ne déclenche rien avant le jour 183 : 39 °C.h < 40 °C.h.
    let mut s = saisons(true);
    let a = annee(
        &mut s,
        |d| Journee {
            dh_fr_c_h: if d == 150 { 39.0 } else { 0.0 },
            ..occupe()
        },
        |_| false,
    );
    assert!(!a[149].1 && !a[181].1 && a[182].1);
    // Local peu occupé : le seuil est modulé, sans descendre sous la moitié. 21 °C.h au jour 150,
    // aucune occupation : seuil de 40 × 0,5 = 20 °C.h, atteint.
    let mut s = saisons(true);
    let a = annee(
        &mut s,
        |d| Journee {
            dh_fr_c_h: if d == 150 { 21.0 } else { 0.0 },
            ..Journee::default()
        },
        |_| false,
    );
    assert!(!a[148].1 && a[149].1);
}

#[test]
fn calcul_th_d_en_periode_de_confort_adaptatif() {
    // Période de confort adaptatif du jour 120 au jour 270. Chauffage : jamais autorisé pendant
    // la période (éq. 77 et 80). Refroidissement : démarre au premier jour de la période
    // (éq. 88) et ne s'arrête pas tant qu'elle dure (éq. 90) ; besoins nuls, arrêt au jour 271.
    let periode = |j: u16| (120..=270).contains(&j);
    let journee = |_: u16| Journee {
        q_ch_wh: 24_000.0,
        ..occupe()
    };
    let mut s = saisons(true);
    let a = annee(&mut s, journee, periode);
    for (i, &(ch, fr)) in a.iter().enumerate() {
        let j = i + 1;
        if (120..=270).contains(&j) {
            assert!(!ch, "chauffage, jour {j}");
        }
        assert_eq!(fr, (120..=270).contains(&j), "refroidissement, jour {j}");
    }
    // Avant la période, les besoins de chauffage maintiennent la saison de chauffage.
    assert!(a[118].0);
}
