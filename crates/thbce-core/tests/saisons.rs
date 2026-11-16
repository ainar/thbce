// SPDX-License-Identifier: GPL-3.0-or-later
//! Saisons propres d'un groupe (Th-BCE 2020 §4.6) : scénarios déroulés à la main.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::conventions::ModeCalcul;
use thbce_core::scenarios::saisons::{refroidissement_possible, Journee, Saison, Saisons};

/// Groupe de 100 m², occupé en permanence (168 h par semaine type), en calcul des
/// consommations.
fn saisons(i_clim: bool) -> Saisons {
    Saisons::nouveau(100.0, 168.0, ModeCalcul::ThC, i_clim)
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

#[test]
fn en_bbio_le_froid_est_autorise_sans_systeme_de_refroidissement() {
    // §4.6.3.3, éq. (91) : en calcul du Bbio, les autorisations effectives sont les autorisations
    // propres, calculées par la procédure de la fiche sans égard pour iclim. Même scénario que
    // `demarrage_et_arret_du_refroidissement`, sans système de refroidissement : démarrage au
    // jour 150, arrêt au jour 283.
    assert!(refroidissement_possible(ModeCalcul::ThB, false));
    assert!(!refroidissement_possible(ModeCalcul::ThC, false));
    assert!(!refroidissement_possible(ModeCalcul::ThD, false));
    assert!(refroidissement_possible(ModeCalcul::ThC, true));
    let journee = |d: u16| Journee {
        dh_fr_c_h: if d == 150 { 50.0 } else { 0.0 },
        q_fr_wh: if d <= 260 { 24_000.0 } else { 0.0 },
        ..occupe()
    };
    let mut s = Saisons::nouveau(100.0, 168.0, ModeCalcul::ThB, false);
    for j in 1..=365_u16 {
        s.accumuler(&journee(j));
        s.nouveau_jour(j, false);
        // L'autorisation effective vaut l'autorisation propre, chaque jour.
        assert_eq!(
            s.autorisations_effectives_bbio(),
            Some((s.aut_ch, s.aut_fr))
        );
        assert_eq!(s.aut_fr, (150..=282).contains(&j), "jour {j}");
        // Plein été : le froid est autorisé (éq. 89), sans système de refroidissement.
        if (183..=245).contains(&j) {
            assert!(s.aut_fr);
        }
    }
    assert_eq!((s.j_debut_fr, s.j_fin_fr), (Some(150), Some(283)));
    // En calcul des consommations, sans système : jamais de froid, et les autorisations
    // effectives ne sont pas celles du groupe.
    let mut s = Saisons::nouveau(100.0, 168.0, ModeCalcul::ThC, false);
    for j in 1..=365_u16 {
        s.accumuler(&journee(j));
        s.nouveau_jour(j, false);
        assert!(!s.aut_fr);
    }
    assert_eq!(s.autorisations_effectives_bbio(), None);
}

#[test]
fn moyenne_des_besoins_apres_un_redemarrage_sur_168_heures() {
    // Éq. (73) : une semaine après le redémarrage, Nbh_total_somme_ch = 168 ; la somme porte sur
    // les sept journées écoulées, de 9 h (j − 7) à 9 h (j), et non sur la seule dernière journée
    // comme l'écrit la formule (coquille, Q35 point 3).
    // Arrêt au jour 123 et redémarrage au jour 130, comme dans
    // `arret_redemarrage_et_reprise_du_chauffage`. Journées 131 à 136 : 10 Wh/m² par heure
    // (24 000 Wh) ; journée 137 : rien. Au jour 137, la moyenne sur 168 h vaut
    // 6 × 24 000 / 168 / 100 = 8,57 Wh/m² : le chauffage continue. Avec la somme sur 24 h de la
    // formule, elle vaudrait 0 et le chauffage s'arrêterait.
    let journee = |d: u16| Journee {
        q_ch_wh: if d <= 100 || (131..=136).contains(&d) {
            24_000.0
        } else {
            0.0
        },
        dh_ch_c_h: if d == 130 { 30.0 } else { 0.0 },
        ..occupe()
    };
    let mut s = saisons(false);
    for j in 1..=137_u16 {
        s.accumuler(&journee(j));
        s.nouveau_jour(j, false);
        if j == 137 {
            let attendu = 6.0 * 24_000.0 / 168.0 / 100.0;
            assert!((s.mgb_ch_wh_m2(j) - attendu).abs() < 1e-12);
            assert!(s.aut_ch, "le chauffage continue au jour 137");
        }
    }
    // Ensuite la fenêtre s'étend d'un jour par jour (éq. 74) : au jour 138, 8 journées, 192 h.
    s.accumuler(&journee(138));
    s.nouveau_jour(138, false);
    assert!((s.mgb_ch_wh_m2(138) - 6.0 * 24_000.0 / 192.0 / 100.0).abs() < 1e-12);
    // La fenêtre atteint 28 jours au jour 158 (journées 131 à 158 : six journées chauffées,
    // 6 × 24 000/672/100 = 2,14 Wh/m², au-dessus du seuil). Au jour 159, elle couvre les
    // journées 132 à 159 : cinq journées chauffées, 1,79 Wh/m² : arrêt.
    let mut arret = None;
    for j in 139..=170_u16 {
        s.accumuler(&journee(j));
        s.nouveau_jour(j, false);
        if !s.aut_ch && arret.is_none() {
            arret = Some(j);
        }
    }
    assert_eq!(arret, Some(159));
}

#[test]
fn ordre_de_grandeur_de_la_moyenne_glissante_des_besoins() {
    // Q35 point 1 : Qsys_ch(h) est une puissance horaire en W sur 1 h, soit des Wh ; divisée par
    // Agr, la moyenne est en Wh/m², unité du seuil d'arrêt (2 Wh/m²).
    // Maison d'exemple de 100 m² dont les déperditions valent 100 W/K (enveloppe et
    // renouvellement d'air). En plein hiver, 15 K d'écart et 500 W d'apports gratuits :
    // 1 000 W de besoin, 24 000 Wh par jour, soit une moyenne de 10 Wh/m² par heure, du même
    // ordre que le seuil et au-dessus : la saison de chauffage se maintient. En kWh, la même
    // moyenne vaudrait 0,01 et toute maison arrêterait son chauffage au jour 57.
    let besoin_w = 100.0 * 15.0 - 500.0;
    let mut s = Saisons::nouveau(100.0, 168.0, ModeCalcul::ThB, false);
    for j in 1..=57_u16 {
        s.accumuler(&Journee {
            q_ch_wh: besoin_w * 24.0,
            ..occupe()
        });
        s.nouveau_jour(j, false);
    }
    let mgb = s.mgb_ch_wh_m2(57);
    assert!((mgb - 10.0).abs() < 1e-12);
    assert!((1.0..100.0).contains(&mgb), "ordre de grandeur du seuil");
    assert!(s.aut_ch);
    // À la mi-saison, 2 K d'écart utile : 200 W, soit 2 Wh/m² par heure, au seuil : arrêt
    // quand la fenêtre de 28 jours ne contient plus que cela.
    for j in 58..=90_u16 {
        s.accumuler(&Journee {
            q_ch_wh: 200.0 * 24.0,
            ..occupe()
        });
        s.nouveau_jour(j, false);
    }
    assert!((s.mgb_ch_wh_m2(90) - 2.0).abs() < 1e-12);
    assert!(!s.aut_ch);
    assert_eq!(s.j_fin_ch, Some(85));
}
