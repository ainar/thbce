// SPDX-License-Identifier: GPL-3.0-or-later
//! Protections mobiles (Th-BCE 2020 §5.9 et §5.12) : valeurs calculées à la main à partir des
//! matrices conventionnelles des figures 35 et 38 (famille « habitation »).
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::bati::protection_mobile::{
    a_ouv_max_avec_pm_m2, jour_gestion_manuelle, r_prot_selon_eclairement, taux_passage_air,
    ConditionsGpm, GestionResolue, SaisonGpm, TemperaturesMaximales,
};
use thbce_core::entree::{GestionPm, ProtectionMobile, TypePm, TypoPm};

fn proche(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * (1.0 + b.abs())
}

macro_rules! assert_proche {
    ($a:expr, $b:expr) => {
        assert!(proche($a, $b), "{} obtenu, {} attendu", $a, $b)
    };
}

fn pm(type_pm: TypePm, gestion: GestionPm, exterieure: bool, pm2: bool) -> ProtectionMobile {
    ProtectionMobile {
        type_pm,
        gestion,
        exterieure,
        seconde_protection: pm2,
        typo_pm: None,
    }
}

/// Volet à gestion manuelle motorisée en maison individuelle (Pocc = 0,5).
fn volet_motorise(pm2: bool) -> GestionResolue {
    GestionResolue::nouvelle(
        &pm(TypePm::Volet, GestionPm::ManuelleMotorisee, true, pm2),
        1,
    )
    .unwrap()
}

/// Zone occupée, de jour, en hiver, sans vent, veille fraîche.
fn occupe(etp_lux: f64, saison: SaisonGpm) -> ConditionsGpm {
    ConditionsGpm {
        i_occ_zone: true,
        i_occ_gpm: 1,
        etp_lux,
        v_vent_c_m_s: 0.0,
        saison,
        top_j1_max_c: 20.0,
        confort_adaptatif_th_d: false,
    }
}

fn inoccupe(i_occ_gpm: i8, saison: SaisonGpm) -> ConditionsGpm {
    ConditionsGpm {
        i_occ_zone: false,
        i_occ_gpm,
        ..occupe(15_000.0, saison)
    }
}

#[test]
fn temperature_operative_maximale_de_la_veille() {
    // Éq. (175) : le maximum du jour est reporté sur la veille au changement de jour, puis remis
    // à zéro ; la veille du premier jour vaut zéro.
    let mut t = TemperaturesMaximales::default();
    for theta in [18.0, 22.0, 21.0] {
        t.avancer(theta, false);
    }
    assert_eq!((t.top_j_max_c, t.top_j1_max_c), (22.0, 0.0));
    // Premier pas du jour suivant : θop,fin(h−1) = 20 °C appartient encore à la veille.
    t.avancer(20.0, true);
    assert_eq!((t.top_j_max_c, t.top_j1_max_c), (0.0, 22.0));
    t.avancer(27.0, false);
    t.avancer(19.0, true);
    assert_eq!((t.top_j_max_c, t.top_j1_max_c), (0.0, 27.0));
}

#[test]
fn saison_jour_et_ratio_selon_l_eclairement() {
    // Éq. (181).
    assert_eq!(SaisonGpm::de_saison_effective(1), Some(SaisonGpm::Ete));
    assert_eq!(SaisonGpm::de_saison_effective(2), Some(SaisonGpm::MiSaison));
    assert_eq!(SaisonGpm::de_saison_effective(3), Some(SaisonGpm::Hiver));
    assert_eq!(SaisonGpm::de_saison_effective(4), Some(SaisonGpm::MiSaison));
    assert_eq!(SaisonGpm::de_saison_effective(0), None);
    // Éq. (183).
    assert!(jour_gestion_manuelle(1, 500.0));
    assert!(!jour_gestion_manuelle(1, 0.0));
    assert!(jour_gestion_manuelle(0, 0.0));
    assert!(!jour_gestion_manuelle(-1, 500.0));
    // Éq. (177) : Rprot0 = 0,1 ; Eclman = 100 000 lux.
    assert_proche!(r_prot_selon_eclairement(0.0, 0.1, 100_000.0), 0.1);
    assert_proche!(r_prot_selon_eclairement(20_000.0, 0.1, 100_000.0), 0.28);
    assert_proche!(r_prot_selon_eclairement(100_000.0, 0.1, 100_000.0), 1.0);
    assert_proche!(r_prot_selon_eclairement(150_000.0, 0.1, 100_000.0), 1.0);
}

#[test]
fn volet_motorise_en_occupation() {
    let g = volet_motorise(false);
    // Hiver, de jour, 20 000 lux : Rprot0 = 10 %, Ecl1 = 100 000 → 0,1 + 0,9 × 0,2 = 0,28 dans la
    // part occupée (Pocc = 0,5, éq. 178) ; 10 % (inoccupation, jour) dans le reste.
    // Éq. (209) : Rprot1 = 0,5 × 0,28 + 0,5 × 0,1 = 0,19.
    let p = g.protections(&occupe(20_000.0, SaisonGpm::Hiver));
    assert_proche!(p.part_gpm, 0.5);
    assert_proche!(p.r_prot1_part_gpm, 0.28);
    assert_proche!(p.r_prot1_hors_part_gpm, 0.10);
    assert_proche!(p.r_prot(), 0.19);
    assert_eq!(p.r_prot2, 0.0);
    // Au-delà du seuil : fermé.
    assert_proche!(
        g.protections(&occupe(100_000.0, SaisonGpm::Hiver))
            .r_prot1_part_gpm,
        1.0
    );
    // De nuit (éclairement nul en occupation) : 90 % en occupation, 80 % en inoccupation.
    let p = g.protections(&occupe(0.0, SaisonGpm::Hiver));
    assert_proche!(p.r_prot1_part_gpm, 0.90);
    assert_proche!(p.r_prot1_hors_part_gpm, 0.80);
    // Été, 30 000 lux. Veille à 26,4 °C : Rprot0 = 20 %, Ecl1 = 60 000 → 0,2 + 0,8 × 0,5 = 0,6.
    // Veille à 26,5 °C (limite atteinte) : Rprot0 = 30 %, Ecl1 = 40 000 → 0,3 + 0,7 × 0,75 = 0,825 ;
    // inoccupation de jour : 50 %.
    let ete = |veille| ConditionsGpm {
        top_j1_max_c: veille,
        ..occupe(30_000.0, SaisonGpm::Ete)
    };
    assert_proche!(g.protections(&ete(26.4)).r_prot1_part_gpm, 0.6);
    let p = g.protections(&ete(26.5));
    assert_proche!(p.r_prot1_part_gpm, 0.825);
    assert_proche!(p.r_prot1_hors_part_gpm, 0.50);
    // La limite de température ne joue qu'en été.
    let mi = ConditionsGpm {
        top_j1_max_c: 30.0,
        ..occupe(30_000.0, SaisonGpm::MiSaison)
    };
    assert_proche!(g.protections(&mi).r_prot1_part_gpm, 0.1 + 0.9 * 0.3);
}

#[test]
fn volet_en_inoccupation() {
    // Éq. (189) : partGPM = 0 ; la baie entière suit le ratio d'inoccupation.
    let g = volet_motorise(false);
    let p = g.protections(&inoccupe(0, SaisonGpm::Hiver));
    assert_eq!((p.part_gpm, p.r_prot1_part_gpm), (0.0, 0.0));
    assert_proche!(p.r_prot(), 0.10);
    // Inoccupation de nuit ou de vacances : 80 %, quel que soit l'éclairement.
    assert_proche!(
        g.protections(&inoccupe(-1, SaisonGpm::Hiver)).r_prot(),
        0.80
    );
    // Gestion non motorisée : 20 % de jour, 70 % de nuit ; 40 % en été après une veille chaude.
    let g = GestionResolue::nouvelle(
        &pm(TypePm::Volet, GestionPm::ManuelleNonMotorisee, true, false),
        1,
    )
    .unwrap();
    assert_proche!(
        g.protections(&inoccupe(0, SaisonGpm::MiSaison)).r_prot(),
        0.20
    );
    assert_proche!(
        g.protections(&inoccupe(-1, SaisonGpm::MiSaison)).r_prot(),
        0.70
    );
    let chaud = ConditionsGpm {
        top_j1_max_c: 28.0,
        ..inoccupe(0, SaisonGpm::Ete)
    };
    assert_proche!(g.protections(&chaud).r_prot(), 0.40);
    // Logement collectif : Pocc = 0,7 (figure 34).
    let g = GestionResolue::nouvelle(
        &pm(TypePm::Volet, GestionPm::ManuelleMotorisee, true, false),
        2,
    )
    .unwrap();
    assert_proche!(
        g.protections(&occupe(20_000.0, SaisonGpm::Hiver)).part_gpm,
        0.7
    );
}

#[test]
fn seconde_protection_mobile() {
    // Avec une seconde protection, de jour en occupation, la première prend la valeur de la
    // colonne « avec 2nde protection » : 20 % en été (veille fraîche), quel que soit l'éclairement.
    // Éq. (198) : Ecl2 = 20 000 lux ; 5 000 lux → Rprot2 = 0,25 ; 30 000 lux → 1.
    let g = volet_motorise(true);
    let p = g.protections(&occupe(5_000.0, SaisonGpm::Ete));
    assert_proche!(p.r_prot1_part_gpm, 0.20);
    assert_proche!(p.r_prot2, 0.25);
    assert_proche!(
        g.protections(&occupe(30_000.0, SaisonGpm::Ete)).r_prot2,
        1.0
    );
    // Hiver : Ecl2 = 100 000 lux.
    assert_proche!(
        g.protections(&occupe(30_000.0, SaisonGpm::Hiver)).r_prot2,
        0.3
    );
    // Éq. (199) : jamais descendue en inoccupation.
    assert_eq!(g.protections(&inoccupe(0, SaisonGpm::Ete)).r_prot2, 0.0);
}

#[test]
fn regles_du_calcul_th_d() {
    // §5.9.3.6, tableau 34 (résidentiel) : en période de confort adaptatif, veille à 27 °C.
    let th_d = |base: ConditionsGpm| ConditionsGpm {
        top_j1_max_c: 27.0,
        confort_adaptatif_th_d: true,
        ..base
    };
    // Volet motorisé : Rprot0 = max(80 % ; 30 %) ; 10 000 lux, Ecl1 = 40 000 (seuil conservé) →
    // 0,8 + 0,2 × 0,25 = 0,85. Inoccupation de jour : max(80 % ; 50 %).
    let g = volet_motorise(true);
    let p = g.protections(&th_d(occupe(10_000.0, SaisonGpm::Ete)));
    assert_proche!(p.r_prot1_part_gpm, 0.85);
    assert_proche!(p.r_prot1_hors_part_gpm, 0.80);
    // Éq. (201) : la seconde protection est désactivée.
    assert_eq!(p.r_prot2, 0.0);
    // De nuit : min(70 % ; 90 %) en occupation, min(70 % ; 80 %) en inoccupation.
    let p = g.protections(&th_d(occupe(0.0, SaisonGpm::Ete)));
    assert_proche!(p.r_prot1_part_gpm, 0.70);
    assert_proche!(p.r_prot1_hors_part_gpm, 0.70);
    // Veille sous la limite : mêmes valeurs qu'en Th-BC (éq. 203).
    let frais = ConditionsGpm {
        top_j1_max_c: 26.0,
        ..th_d(occupe(5_000.0, SaisonGpm::Ete))
    };
    assert_eq!(
        g.protections(&frais),
        g.protections(&occupe(5_000.0, SaisonGpm::Ete))
    );
    // Gestion non motorisée : plancher de 70 %.
    let g = GestionResolue::nouvelle(
        &pm(TypePm::Volet, GestionPm::ManuelleNonMotorisee, true, false),
        1,
    )
    .unwrap();
    assert_proche!(
        g.protections(&th_d(occupe(0.0, SaisonGpm::Ete)))
            .r_prot1_part_gpm,
        0.70
    );
    assert_proche!(
        g.protections(&th_d(inoccupe(0, SaisonGpm::Ete))).r_prot(),
        0.70
    );
    // Store enroulable : 50 % de nuit.
    let g = GestionResolue::nouvelle(
        &pm(
            TypePm::StoreEnroulable,
            GestionPm::ManuelleMotorisee,
            true,
            false,
        ),
        1,
    )
    .unwrap();
    let p = g.protections(&th_d(occupe(0.0, SaisonGpm::Ete)));
    assert_proche!(p.r_prot1_part_gpm, 0.50);
    assert_proche!(p.r_prot1_hors_part_gpm, 0.50);
}

#[test]
fn store_enroulable_et_vent() {
    let store = |exterieur| {
        GestionResolue::nouvelle(
            &pm(
                TypePm::StoreEnroulable,
                GestionPm::ManuelleNonMotorisee,
                exterieur,
                false,
            ),
            1,
        )
        .unwrap()
    };
    // Mi-saison, 40 000 lux : Rprot0 = 10 %, Ecl1 = 80 000 → 0,1 + 0,9 × 0,5 = 0,55 ;
    // inoccupation de jour : 20 %.
    let calme = occupe(40_000.0, SaisonGpm::MiSaison);
    let p = store(true).protections(&calme);
    assert_proche!(p.r_prot1_part_gpm, 0.55);
    assert_proche!(p.r_prot1_hors_part_gpm, 0.20);
    // Éq. (192) : à partir de 10 m/s, le store extérieur est remonté sur toute la baie.
    let vent = |v| ConditionsGpm {
        v_vent_c_m_s: v,
        ..calme
    };
    assert_eq!(store(true).protections(&vent(9.9)), p);
    let p_vent = store(true).protections(&vent(10.0));
    assert_eq!(
        (p_vent.r_prot1_part_gpm, p_vent.r_prot1_hors_part_gpm),
        (0.0, 0.0)
    );
    // Un store intérieur ou un volet ne sont pas concernés.
    assert_eq!(
        store(false).protections(&vent(15.0)),
        store(false).protections(&calme)
    );
    let v = volet_motorise(false);
    assert_eq!(v.protections(&vent(15.0)), v.protections(&calme));
}

#[test]
fn ce_que_la_version_ne_prend_pas_en_charge() {
    let auto = pm(
        TypePm::Volet,
        GestionPm::AutomatiqueAvecDerogation,
        true,
        false,
    );
    assert!(GestionResolue::nouvelle(&auto, 1).is_none());
    let venitien = pm(
        TypePm::StoreVenitien,
        GestionPm::ManuelleMotorisee,
        true,
        false,
    );
    assert!(GestionResolue::nouvelle(&venitien, 1).is_none());
    // Usage hors de la famille « habitation » : matrices non transcrites.
    let volet = pm(TypePm::Volet, GestionPm::ManuelleMotorisee, true, false);
    assert!(GestionResolue::nouvelle(&volet, 3).is_none());
}

#[test]
fn permeabilite_des_protections_mobiles() {
    // Tableau 40.
    assert_eq!(taux_passage_air(TypoPm::Etanche), 0.0);
    assert_eq!(taux_passage_air(TypoPm::FaiblementAjouree), 0.10);
    assert_eq!(taux_passage_air(TypoPm::Ajouree), 0.25);
    assert_eq!(taux_passage_air(TypoPm::FortementAjouree), 0.50);
    assert_eq!(taux_passage_air(TypoPm::TresFortementAjouree), 0.75);
    // Éq. (239) : 2 m² ouvrables, protection à moitié fermée, taux de 10 % :
    // 2 × (0,5 × 0,1 + 0,5) = 1,1 m². Sans protection (taux de 100 %) : 2 m².
    assert_proche!(a_ouv_max_avec_pm_m2(2.0, 0.5, 0.10), 1.1);
    assert_proche!(a_ouv_max_avec_pm_m2(2.0, 0.5, 1.0), 2.0);
    assert_proche!(a_ouv_max_avec_pm_m2(2.0, 1.0, 0.0), 0.0);
}
