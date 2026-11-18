// SPDX-License-Identifier: GPL-3.0-or-later
//! Baies vitrées (Th-BCE 2020 §5.10 et §5.14) : valeurs calculées à la main.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::bati::baie::{
    self, u_baie, BaieResolue, EnsembleBaies, GestionProtection, Protections, TypeBaieEclairage,
};
use thbce_core::bati::ConditionsExterieures;
use thbce_core::climat::environnement_proche::Soleil;
use thbce_core::conventions::ModeCalcul;
use thbce_core::entree::{Baie, BaieType, Catalogue, Exposition, FacteursBaie};

fn proche(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * (1.0 + b.abs())
}

macro_rules! assert_proche {
    ($a:expr, $b:expr) => {
        assert!(proche($a, $b), "{} obtenu, {} attendu", $a, $b)
    };
}

fn exposition(azimut_deg: f64, inclinaison_deg: f64) -> Exposition {
    Exposition {
        azimut_deg,
        inclinaison_deg,
        lpb_m: None,
        hpb_m: None,
        masques: vec![],
    }
}

/// Fenêtre : Usp = 1,3 (verticale) et 1,5 (horizontale) ; Uap = 1,1 et 1,2.
fn fenetre() -> BaieType {
    BaieType {
        id: "f".into(),
        u_sp_vert_w_m2k: Some(1.3),
        u_sp_horiz_w_m2k: Some(1.5),
        u_ap_vert_w_m2k: Some(1.1),
        u_ap_horiz_w_m2k: Some(1.2),
        sans_protection: FacteursBaie {
            sw1_c: Some(0.4),
            sw2_c: Some(0.05),
            sw3_c: Some(0.0),
            sw1_e: 0.45,
            sw2_e: 0.05,
            sw3_e: 0.0,
            tli: 0.6,
            tlid: Some(0.0),
        },
        avec_protection: Some(FacteursBaie {
            sw1_c: Some(0.02),
            sw2_c: Some(0.03),
            sw3_c: Some(0.01),
            sw1_e: 0.02,
            sw2_e: 0.03,
            sw3_e: 0.01,
            tli: 0.1,
            tlid: Some(0.04),
        }),
        protection_mobile: None,
        ouvrant: None,
    }
}

fn baie(a: f64, e: Exposition) -> BaieResolue {
    let b = Baie {
        id: "b".into(),
        type_baie: "f".into(),
        a_b_m2: a,
        exposition: e,
        expo_br: None,
        bruit: None,
        espace_tampon: None,
    };
    BaieResolue::nouvelle(&b, &fenetre(), &Catalogue::default()).unwrap()
}

/// Soleil plein sud à 30°, IDn = 800 W/m², Idi = 100 W/m², EDn = 80 000 lux, Edi = 12 000 lux,
/// ciel à −5 °C, air à 10 °C.
///
/// Sur une verticale sud : Drp = 800·cos 30° = 692,8203230 ; Dfp = 50 ; Rrp = (400 + 100)·0,2·0,5
/// = 50 ; Isr = 792,8203230 W/m². Erp = 80 000·cos 30° = 69 282,03230 ; Efp = 6 000 ;
/// Errp = (40 000 + 12 000)·0,2·0,5 = 5 200 lux.
fn conditions(mode: ModeCalcul) -> ConditionsExterieures {
    ConditionsExterieures {
        soleil: Soleil {
            gamma_rad: 30f64.to_radians(),
            psi_rad: 0.0,
        },
        idn_w_m2: 800.0,
        idi_w_m2: 100.0,
        edn_lux: 80_000.0,
        edi_lux: 12_000.0,
        te_c: 10.0,
        teciel_c: -5.0,
        h: 0,
        mode,
        is_conf_adapt: false,
    }
}

const TH_B: ModeCalcul = ModeCalcul::ThB;

fn manuelle(r: f64) -> Protections {
    Protections {
        part_gpm: 1.0,
        r_prot1_part_gpm: r,
        ..Protections::OUVERTES
    }
}

#[test]
fn coefficient_u_selon_l_inclinaison() {
    // Éq. (207) : horizontale en dessous de 30°, verticale à partir de 60°, interpolation entre.
    let u = |beta| u_baie(Some(1.3), Some(1.5), beta).unwrap();
    assert_proche!(u(0.0), 1.5);
    assert_proche!(u(29.9), 1.5);
    assert_proche!(u(30.0), 1.5);
    assert_proche!(u(45.0), 1.4);
    assert_proche!(u(60.0), 1.3);
    assert_proche!(u(90.0), 1.3);
    // Un seul coefficient saisi : suffisant ou non selon l'inclinaison.
    assert_eq!(u_baie(Some(1.3), None, 90.0), Some(1.3));
    assert_eq!(u_baie(Some(1.3), None, 45.0), None);
    assert_eq!(u_baie(Some(1.3), None, 10.0), None);
    assert_eq!(u_baie(None, Some(1.5), 10.0), Some(1.5));
    assert_eq!(u_baie(None, Some(1.5), 60.0), None);
}

#[test]
fn baie_sans_protection_en_place() {
    // Éq. (219) à (221) avec Rprot = 0 : Fs1 = 8 × 0,4 × 792,8203230 = 2 537,025034 W ;
    // Fs2 = 8 × 0,05 × 792,8203230 = 317,1281292 W ; Fs3 = 0. Éq. (210) : Hges = 8 × 1,3.
    let b = baie(8.0, exposition(0.0, 90.0));
    let f = b.flux(&conditions(TH_B), &Protections::OUVERTES);
    assert_proche!(f.h_ges_w_k, 10.4);
    assert_proche!(f.fs1_w, 2_537.025_033_690_788);
    assert_proche!(f.fs2_w, 317.128_129_211_348_5);
    assert_proche!(f.fs3_w, 0.0);
    // Baie verticale : pas d'échange avec la voûte céleste (éq. 17 et 222).
    assert!(f.ftvc_w.abs() < 1e-9);
    // Éq. (231) à (233), baie non diffusante (Tlidsp = 0, Tliisp = 0,6), volume normal (type 2) :
    // Flt1 = 8 × 0,6 × 69 282,03230 ; Flt2 = 8 × 0,6 × 6 000 ; Flt3 = 8 × 0,6 × 5 200.
    assert_eq!(b.type_eclairage, TypeBaieEclairage::VolumeNormal);
    assert_proche!(f.flt1_lm[2], 332_553.755_053_224_6);
    assert_proche!(f.flt2_lm[2], 28_800.0);
    assert_proche!(f.flt3_lm[2], 24_960.0);
    for n in 0..2 {
        assert_eq!((f.flt1_lm[n], f.flt2_lm[n], f.flt3_lm[n]), (0.0, 0.0, 0.0));
    }
    // Éq. (224) : en Th-D, facteurs « -e » : Fs1 = 8 × 0,45 × 792,8203230.
    let f = b.flux(&conditions(ModeCalcul::ThD), &Protections::OUVERTES);
    assert_proche!(f.fs1_w, 2_854.153_162_902_137);
}

#[test]
fn baie_avec_protection_en_place() {
    // Rprot = 1 : Fs1 = 8 × 0,02 × 792,8203230 ; Fs2 = 8 × 0,03 × … ; Fs3 = 8 × 0,01 × …
    let b = baie(8.0, exposition(0.0, 90.0));
    let f = b.flux(&conditions(TH_B), &manuelle(1.0));
    assert_proche!(f.h_ges_w_k, 8.8);
    assert_proche!(f.fs1_w, 126.851_251_684_539_4);
    assert_proche!(f.fs2_w, 190.276_877_526_809_1);
    assert_proche!(f.fs3_w, 63.425_625_842_269_7);
    // Tliap = 0,1 dont 0,04 en diffus : Tliiap = 0,06.
    // Flt1 = 8 × 0,06 × 69 282,03230 ; Flt2 = 8 × (0,04 × 69 282,03230 + 0,1 × 6 000 + 0,04 × 5 200) ;
    // Flt3 = 8 × 0,06 × 5 200.
    assert_proche!(f.flt1_lm[2], 33_255.375_505_322_46);
    assert_proche!(f.flt2_lm[2], 28_634.250_336_881_64);
    assert_proche!(f.flt3_lm[2], 2_496.0);

    // Rprot = 0,5 en gestion manuelle : moyenne des deux états (éq. 210 et 219).
    let f = b.flux(&conditions(TH_B), &manuelle(0.5));
    assert_proche!(f.h_ges_w_k, 9.6);
    assert_proche!(
        f.fs1_w,
        (2_537.025_033_690_788 + 126.851_251_684_539_4) / 2.0
    );
}

#[test]
fn coefficient_hges_en_gestion_automatique() {
    // Éq. (211), partGPM = 0,4 et Rprot1partGPM = 0,5 :
    // part dérogée : 0,4 × (0,5 × 1,1 + 0,5 × 1,3) = 0,48 ;
    // part automatique fermée (Rprot11-partGPM = 1) : 0,6 × 1,1 = 0,66 ; Hges = 8 × 1,14 = 9,12 ;
    // part automatique non entièrement fermée (0,9) : 0,6 × 1,3 = 0,78 ; Hges = 8 × 1,26 = 10,08.
    let b = baie(8.0, exposition(0.0, 90.0));
    let p = |hors| Protections {
        part_gpm: 0.4,
        r_prot1_part_gpm: 0.5,
        r_prot1_hors_part_gpm: hors,
        r_prot2: 0.0,
        gestion: GestionProtection::Automatique,
    };
    assert_proche!(b.h_ges_w_k(&p(1.0)), 9.12);
    assert_proche!(b.h_ges_w_k(&p(0.9)), 10.08);
    // Éq. (209) : Rprot1,b = 0,4 × 0,5 + 0,6 × 0,9 = 0,74.
    assert_proche!(p(0.9).r_prot(), 0.74);
}

#[test]
fn seconde_protection_mobile() {
    // Éq. (235) à (237), partGPM = 0,5 ; Rprot1partGPM = 0,2 ; Rprot11-partGPM = 0,6 ; Rprot2 = 0,5.
    // Première protection : 0,5 × 0,2 + 0,5 × 0,6 = 0,4. Seconde seule : 0,5 × 0,8 × 0,5 = 0,2.
    // Sans protection : 0,5 × 0,4 + 0,5 × 0,8 × 0,5 = 0,4. Tli2 = Tlid2 = 0,1, donc Tlii2 = 0.
    // Flt1 = 8 × (0,4 × 0,06 + 0,2 × 0 + 0,4 × 0,6) × 69 282,03230 = 8 × 0,264 × 69 282,03230.
    // Flt2 = 8 × [(0,4 × 0,04 + 0,2 × 0,1) × 69 282,03230 + (0,4 × 0,1 + 0,2 × 0,1 + 0,4 × 0,6) × 6 000
    //        + (0,4 × 0,04 + 0,2 × 0,1) × 5 200] = 8 × (2 494,153163 + 1 800 + 187,2).
    // Flt3 = 8 × 0,264 × 5 200 = 10 982,4.
    let b = baie(8.0, exposition(0.0, 90.0));
    let p = Protections {
        part_gpm: 0.5,
        r_prot1_part_gpm: 0.2,
        r_prot1_hors_part_gpm: 0.6,
        r_prot2: 0.5,
        gestion: GestionProtection::Manuelle,
    };
    let f = b.flux(&conditions(TH_B), &p);
    assert_proche!(f.flt1_lm[2], 146_323.652_223_418_8);
    assert_proche!(f.flt2_lm[2], 35_850.825_303_193_48);
    assert_proche!(f.flt3_lm[2], 10_982.4);
    // La seconde protection ne change pas les flux solaires : Rprot = 0,4 (éq. 219).
    assert_proche!(
        f.fs1_w,
        0.6 * 2_537.025_033_690_788 + 0.4 * 126.851_251_684_539_4
    );
}

#[test]
fn sans_seconde_protection_les_eq_235_a_237_redonnent_les_eq_231_a_233() {
    // Avec Rprot2 = 0, les flux lumineux ne dépendent que de Rprot1,b (éq. 209) : deux
    // répartitions de même Rprot1,b = 0,5 donnent les mêmes flux.
    let b = baie(8.0, exposition(0.0, 90.0));
    let c = conditions(TH_B);
    let f1 = b.flux(&c, &manuelle(0.5));
    let f2 = b.flux(
        &c,
        &Protections {
            part_gpm: 0.25,
            r_prot1_part_gpm: 0.8,
            r_prot1_hors_part_gpm: 0.4,
            ..Protections::OUVERTES
        },
    );
    assert_proche!(f2.flt1_lm[2], f1.flt1_lm[2]);
    assert_proche!(f2.flt2_lm[2], f1.flt2_lm[2]);
    assert_proche!(f2.flt3_lm[2], f1.flt3_lm[2]);
}

#[test]
fn fenetre_de_toit_et_voute_celeste() {
    // Baie horizontale de 2 m² : Usp = 1,5 ; Qer* = 5,5 × (−5 − 10) = −82,5 W/m².
    // Éq. (222) : Ftvc = 2 × 1,5/25 × (−82,5) = −9,9 W ; protection fermée : 2 × 1,2/25 × (−82,5).
    let b = baie(2.0, exposition(0.0, 0.0));
    let c = conditions(TH_B);
    assert_proche!(b.flux(&c, &Protections::OUVERTES).ftvc_w, -9.9);
    assert_proche!(b.flux(&c, &manuelle(1.0)).ftvc_w, -7.92);
    // Rayonnement horizontal : 400 + 100 = 500 W/m² ; Fs1 = 2 × 0,4 × 500.
    assert_proche!(b.flux(&c, &Protections::OUVERTES).fs1_w, 400.0);
}

#[test]
fn ensemble_des_baies_d_un_groupe() {
    let baies = vec![
        baie(8.0, exposition(0.0, 90.0)),
        baie(2.0, exposition(180.0, 90.0)),
        baie(3.0, exposition(90.0, 90.0)),
        baie(1.0, exposition(270.0, 90.0)),
        // Bornes du secteur sud : 45° et 315° (soit −45°) en font partie (éq. 271).
        baie(1.0, exposition(45.0, 90.0)),
        baie(1.0, exposition(315.0, 90.0)),
        baie(4.0, exposition(0.0, 0.0)),
        baie(1.0, exposition(0.0, 45.0)),
    ];
    let e = EnsembleBaies::de(&baies);
    // Éq. (279), (281), (282) : 16 m² verticaux, 5 m² horizontaux (β < 60°), 21 m² en tout.
    assert_proche!(e.a_baies_v_m2, 16.0);
    assert_proche!(e.a_baies_h_m2, 5.0);
    assert_proche!(e.a_baies_m2, 21.0);
    // Parts rapportées aux baies verticales : 10/16, 2/16, 3/16, 1/16.
    assert_proche!(e.part_sud, 0.625);
    assert_proche!(e.part_nord, 0.125);
    assert_proche!(e.part_ouest, 0.1875);
    assert_proche!(e.part_est, 0.0625);
    // Éq. (280) : seules les baies d'inclinaison nulle, 4/21. Éq. (272) : 0,625 × 21.
    assert_proche!(e.part_horiz, 4.0 / 21.0);
    assert_proche!(e.a_baies_sud_m2(), 13.125);

    // Éq. (257) à (270) : sommes sur les baies.
    let c = conditions(TH_B);
    let p = Protections::OUVERTES;
    let total = baie::somme(baies.iter().map(|b| b.flux(&c, &p)));
    let fs1: f64 = baies.iter().map(|b| b.flux(&c, &p).fs1_w).sum();
    assert_proche!(total.fs1_w, fs1);
    // Hges = 16 × 1,3 + 4 × 1,5 + 1 × 1,4 (baie à 45°) = 28,2 W/K.
    assert_proche!(total.h_ges_w_k, 28.2);
}

#[test]
fn valeurs_par_defaut_des_facteurs_solaires() {
    // §5.10.3.6.3. Sans protection : Swsp-e = 0,45 + 0,05 = 0,5 ; éq. (215) : Swsp-c = 0,4 ;
    // composantes dans les proportions de Swsp-e : Sw1 = 0,45 × 0,8 = 0,36 ; Sw2 = 0,05 × 0,8 = 0,04.
    // Avec protection, éq. (216) : Swap-c = Swap-e, composantes égales. Éq. (218) : Tlid = 0.
    let mut t = fenetre();
    for f in [&mut t.sans_protection, t.avec_protection.as_mut().unwrap()] {
        (f.sw1_c, f.sw2_c, f.sw3_c, f.tlid) = (None, None, None, None);
    }
    let b = Baie {
        id: "b".into(),
        type_baie: "f".into(),
        a_b_m2: 8.0,
        exposition: exposition(0.0, 90.0),
        expo_br: None,
        bruit: None,
        espace_tampon: None,
    };
    let r = BaieResolue::nouvelle(&b, &t, &Catalogue::default()).unwrap();
    assert_proche!(r.sans_protection.sw_c.sw1, 0.36);
    assert_proche!(r.sans_protection.sw_c.sw2, 0.04);
    assert_proche!(r.sans_protection.sw_c.sw3, 0.0);
    assert_proche!(
        r.sans_protection.sw_c.global(),
        r.sans_protection.sw_e.global() - 0.1
    );
    assert_eq!(r.avec_protection.sw_c, r.avec_protection.sw_e);
    assert_eq!((r.sans_protection.tlid, r.avec_protection.tlid), (0.0, 0.0));
    // Chaque valeur par défaut appliquée est signalée, avec la règle du texte.
    let champs: Vec<&str> = r
        .valeurs_par_defaut
        .iter()
        .map(|d| d.champ.as_str())
        .collect();
    assert_eq!(
        champs,
        [
            "sans_protection.sw1_c",
            "sans_protection.sw2_c",
            "sans_protection.sw3_c",
            "sans_protection.tlid",
            "avec_protection.sw1_c",
            "avec_protection.sw2_c",
            "avec_protection.sw3_c",
            "avec_protection.tlid",
        ]
    );
    assert_proche!(r.valeurs_par_defaut[0].valeur, 0.36);
    assert!(r.valeurs_par_defaut[0].regle.contains("éq. (215)"));
    assert!(r.valeurs_par_defaut[4].regle.contains("éq. (216)"));
    assert!(r.valeurs_par_defaut[3].regle.contains("éq. (218)"));
    // Le flux Th-B utilise la valeur par défaut : Fs1 = 8 × 0,36 × 792,8203230.
    let f = r.flux(&conditions(TH_B), &Protections::OUVERTES);
    assert_proche!(f.fs1_w, 8.0 * 0.36 * 792.820_323_027_551);
    // Tout saisi : aucune valeur par défaut.
    assert!(baie(8.0, exposition(0.0, 90.0))
        .valeurs_par_defaut
        .is_empty());
    // Valeur par défaut non positive (Swsp-e ≤ 0,1) ou saisie partielle : pas de résolution.
    let mut t2 = t.clone();
    t2.sans_protection.sw1_e = 0.05;
    assert!(BaieResolue::nouvelle(&b, &t2, &Catalogue::default()).is_none());
    let mut t3 = t.clone();
    t3.sans_protection.sw1_c = Some(0.3);
    assert!(BaieResolue::nouvelle(&b, &t3, &Catalogue::default()).is_none());
}

/// À revoir quand ces grandeurs seront consommées (décision Q30, point 3).
///
/// Th-BCE 2020 §5.14.3 : les parts par orientation (éq. 271, 273, 275 et 277) sont rapportées à
/// la surface des baies verticales, mais les éq. (272), (274), (276) et (278) les multiplient par
/// la surface de toutes les baies. Pour un groupe qui a des baies horizontales, la surface
/// « orientée au sud » dépasse donc la surface des baies verticales réellement au sud. Le moteur
/// applique le texte tel quel ; ce test fixe l'écart sur un exemple : 8 m² de baies verticales,
/// toutes au sud, et 4 m² de fenêtres de toit donnent Abaies-sud = 1 × 12 = 12 m², et non 8 m².
///
/// Marqué `#[ignore]` parce qu'il décrit un écart et non une propriété voulue ; il passe avec
/// `--include-ignored` tant que le texte est appliqué tel quel.
#[test]
#[ignore = "écart documenté de l'éq. (272), à revoir quand ces grandeurs seront consommées (Q30)"]
fn ecart_de_l_eq_272_pour_un_groupe_avec_des_baies_horizontales() {
    let baies = vec![
        baie(8.0, exposition(0.0, 90.0)),
        baie(4.0, exposition(0.0, 0.0)),
    ];
    let e = EnsembleBaies::de(&baies);
    assert_proche!(e.part_sud, 1.0);
    assert_proche!(e.a_baies_v_m2, 8.0);
    // Texte tel quel : 12 m². Lecture « physique » attendue : 8 m².
    assert_proche!(e.a_baies_sud_m2(), 12.0);
    assert_proche!(e.a_baies_sud_m2() - e.a_baies_v_m2, 4.0);
}
