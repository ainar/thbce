// SPDX-License-Identifier: GPL-3.0-or-later
//! Parois opaques et ponts thermiques (Th-BCE 2020 §5.15 à 5.20) : valeurs calculées à la main.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::bati::paroi_opaque::{self, u_k_beta, EnsembleParois, Inclinaison, ParoiResolue};
use thbce_core::bati::pont_thermique::{self, EnsemblePonts, PontResolu};
use thbce_core::bati::ConditionsExterieures;
use thbce_core::climat::environnement_proche::Soleil;
use thbce_core::conventions::ModeCalcul;
use thbce_core::entree::{
    Catalogue, Exposition, Masque, MasqueDefinition, ParoiOpaque, ParoiType, PontThermique,
    PositionParoi, Vegetalisation,
};

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

fn type_paroi(u: f64, incl_k: PositionParoi, sf_c: f64, sf_e: f64) -> ParoiType {
    ParoiType {
        id: "t".into(),
        u_k_w_m2k: u,
        incl_k,
        sf_c_k: sf_c,
        sf_e_k: sf_e,
        vegetalisation: None,
    }
}

fn paroi(a: f64, t: &ParoiType, e: Exposition) -> ParoiResolue {
    let p = ParoiOpaque {
        id: "p".into(),
        type_paroi: "t".into(),
        a_k_m2: a,
        exposition: e,
    };
    ParoiResolue::nouvelle(&p, t, &Catalogue::default())
}

/// Soleil plein sud à 30° de hauteur, IDn = 800 W/m², Idi = 100 W/m², ciel à −5 °C, air à 10 °C.
fn conditions(mode: ModeCalcul, is_conf_adapt: bool, h: usize) -> ConditionsExterieures {
    ConditionsExterieures {
        soleil: Soleil {
            gamma_rad: 30f64.to_radians(),
            psi_rad: 0.0,
        },
        idn_w_m2: 800.0,
        idi_w_m2: 100.0,
        edn_lux: 0.0,
        edi_lux: 0.0,
        te_c: 10.0,
        teciel_c: -5.0,
        h,
        mode,
        is_conf_adapt,
    }
}

const TH_B: ModeCalcul = ModeCalcul::ThB;
const VERT: PositionParoi = PositionParoi::Verticale;
const ASC: PositionParoi = PositionParoi::HorizontaleFluxAscendant;
const DES: PositionParoi = PositionParoi::HorizontaleFluxDescendant;

#[test]
fn coefficient_u_selon_l_inclinaison() {
    // U = 0,2 donné en position verticale : Rth = 5 − 0,17 = 4,83 m².K/W.
    // Uvert = 0,2 ; Uhoriz,asc = 1/4,97 = 0,2012072435 ; Uhoriz,des = 1/5,04 = 0,1984126984.
    let (u_asc, u_des) = (1.0 / 4.97, 1.0 / 5.04);
    assert_proche!(u_k_beta(0.2, VERT, 0.0), u_asc);
    assert_proche!(u_k_beta(0.2, VERT, 30.0), u_asc);
    // 45° : à mi-chemin entre l'horizontale et la verticale.
    assert_proche!(u_k_beta(0.2, VERT, 45.0), (u_asc + 0.2) / 2.0);
    assert_proche!(u_k_beta(0.2, VERT, 60.0), 0.2);
    assert_proche!(u_k_beta(0.2, VERT, 90.0), 0.2);
    assert_proche!(u_k_beta(0.2, VERT, 120.0), 0.2);
    // 135° : à mi-chemin entre la verticale et l'horizontale à flux descendant (question Q29).
    assert_proche!(u_k_beta(0.2, VERT, 135.0), (0.2 + u_des) / 2.0);
    assert_proche!(u_k_beta(0.2, VERT, 150.0), u_des);
    assert_proche!(u_k_beta(0.2, VERT, 180.0), u_des);
    // La position de référence est restituée telle quelle.
    assert_proche!(u_k_beta(0.12, ASC, 0.0), 0.12);
    assert_proche!(u_k_beta(0.15, DES, 180.0), 0.15);
    // Toiture donnée à flux ascendant, posée verticalement : Rth = 1/0,12 − 0,14 ; U = 1/(Rth + 0,17).
    assert_proche!(u_k_beta(0.12, ASC, 90.0), 1.0 / (1.0 / 0.12 - 0.14 + 0.17));
}

#[test]
fn le_coefficient_u_est_continu_et_monotone() {
    // De 0 à 180°, les résistances superficielles vont de 0,14 à 0,21 : U ne peut que décroître.
    let mut precedent = f64::INFINITY;
    let mut beta = 0.0;
    while beta <= 180.0 {
        let u = u_k_beta(0.3, VERT, beta);
        assert!(u <= precedent + 1e-12, "β = {beta}");
        // Continuité : pas de saut supérieur à celui d'un pas d'interpolation.
        if precedent.is_finite() {
            assert!((precedent - u).abs() < 1e-4, "saut à β = {beta}");
        }
        precedent = u;
        beta += 0.25;
    }
}

#[test]
fn le_coefficient_u_est_continu_aux_quatre_bornes() {
    // Éq. (294) : aux bornes β1 = 30°, β2 = 60°, β3 = 120° et β4 = 150°, les valeurs prises juste
    // avant, à la borne et juste après doivent se raccorder (décision Q29 pour β3 et β4).
    let (u_asc, u_des) = (1.0 / 4.97, 1.0 / 5.04);
    let eps = 1e-9;
    for (borne, attendu) in [(30.0, u_asc), (60.0, 0.2), (120.0, 0.2), (150.0, u_des)] {
        for beta in [borne - eps, borne, borne + eps] {
            let u = u_k_beta(0.2, VERT, beta);
            assert!(
                (u - attendu).abs() < 1e-10,
                "β = {beta} : {u} obtenu, {attendu} attendu"
            );
        }
    }
}

#[test]
fn en_th_d_le_facteur_solaire_bascule_avec_le_confort_adaptatif() {
    // Éq. (291) et (292) : en Th-D, fce = 1 − isconfort_adaptatif. Pendant la période de confort
    // adaptatif, Sf = Sf-e ; en dehors, Sf = Sf-c (décision Q29 : les équations, pas la
    // nomenclature). Toiture de 100 m², rayonnement horizontal de 500 W/m², Sf-c = 0,01 et
    // Sf-e = 0,04 : 500 W avec Sf-c, 2 000 W avec Sf-e.
    let toiture = paroi(
        100.0,
        &type_paroi(0.12, ASC, 0.01, 0.04),
        exposition(0.0, 0.0),
    );
    assert_proche!(
        toiture
            .flux(&conditions(ModeCalcul::ThD, true, 0))
            .phi_sh_s_w,
        2000.0
    );
    assert_proche!(
        toiture
            .flux(&conditions(ModeCalcul::ThD, false, 0))
            .phi_sh_s_w,
        500.0
    );
    // En Th-B et Th-C, Sf-c quelle que soit la période.
    assert_proche!(toiture.flux(&conditions(TH_B, true, 0)).phi_sh_s_w, 500.0);
    assert_proche!(
        toiture
            .flux(&conditions(ModeCalcul::ThC, true, 0))
            .phi_sh_s_w,
        500.0
    );

    // Même règle pour un pont thermique (éq. 302 et 303) : 40 m en toiture, Sf-c = 0,02 et
    // Sf-e = 0,05 m⁻¹, soit 40 × 0,05 × 500 = 1 000 W pendant la période et 400 W en dehors.
    let mut ponts = Vec::new();
    PontResolu::resoudre(
        &pont(Some(exposition(0.0, 0.0))),
        &Catalogue::default(),
        &mut ponts,
    );
    assert_proche!(
        ponts[0]
            .flux(&conditions(ModeCalcul::ThD, true, 0))
            .phi_sh_s_w,
        1000.0
    );
    assert_proche!(
        ponts[0]
            .flux(&conditions(ModeCalcul::ThD, false, 0))
            .phi_sh_s_w,
        400.0
    );
}

#[test]
fn mur_sud_et_toiture() {
    // Mur sud de 22 m², U = 0,2, Sf = 0,01. Rayonnement sur une verticale sud (voir les tests de
    // l'environnement proche) : 692,8203230 + 50 + 50 = 792,8203230 W/m².
    // Éq. (289) : Φsh_s = 22 × 0,01 × 792,8203230 = 174,4204711 W. Éq. (295) : HTH = 4,4 W/K.
    let mur = paroi(
        22.0,
        &type_paroi(0.2, VERT, 0.01, 0.03),
        exposition(0.0, 90.0),
    );
    assert_proche!(mur.h_th_w_k(), 4.4);
    let f = mur.flux(&conditions(TH_B, false, 0));
    assert_proche!(f.phi_sh_s_w, 174.420_471_066_061);
    // Paroi verticale : cos(β) = 0, pas d'échange avec la voûte céleste (éq. 17 et 297).
    assert!(f.phi_sh_vc_w.abs() < 1e-9);

    // Toiture de 100 m², U = 0,12 à flux ascendant, Sf = 0,01. Rayonnement horizontal :
    // 400 + 100 = 500 W/m² ; Φsh_s = 100 × 0,01 × 500 = 500 W.
    // Qer* = 5,5 × (−5 − 10) = −82,5 W/m² ; éq. (297) : Φsh_vc = 100 × (−82,5) × 0,12/25 = −39,6 W.
    let toiture = paroi(
        100.0,
        &type_paroi(0.12, ASC, 0.01, 0.01),
        exposition(0.0, 0.0),
    );
    let f = toiture.flux(&conditions(TH_B, false, 0));
    assert_proche!(f.phi_sh_s_w, 500.0);
    assert_proche!(f.phi_sh_vc_w, -39.6);
}

#[test]
fn facteur_solaire_selon_le_mode_et_la_vegetalisation() {
    // Éq. (290) à (292) : Sf-c en Th-B et Th-C ; en Th-D, Sf-e pendant la période de confort
    // adaptatif, Sf-c en dehors.
    let mur = paroi(
        22.0,
        &type_paroi(0.2, VERT, 0.01, 0.03),
        exposition(0.0, 90.0),
    );
    let en_c = 174.420_471_066_061;
    assert_proche!(
        mur.flux(&conditions(ModeCalcul::ThC, true, 0)).phi_sh_s_w,
        en_c
    );
    assert_proche!(
        mur.flux(&conditions(ModeCalcul::ThD, false, 0)).phi_sh_s_w,
        en_c
    );
    assert_proche!(
        mur.flux(&conditions(ModeCalcul::ThD, true, 0)).phi_sh_s_w,
        3.0 * en_c
    );

    // Éq. (293) : toiture végétalisée, facteurs propres entre les heures 2161 et 6553 exclues.
    let mut t = type_paroi(0.12, ASC, 0.01, 0.01);
    t.vegetalisation = Some(Vegetalisation {
        sf_c_v_k: 0.004,
        sf_e_v_k: 0.002,
    });
    let toiture = paroi(100.0, &t, exposition(0.0, 0.0));
    for (h, attendu) in [
        (0, 500.0),
        (2161, 500.0),
        (2162, 200.0),
        (6552, 200.0),
        (6553, 500.0),
    ] {
        assert_proche!(
            toiture.flux(&conditions(TH_B, false, h)).phi_sh_s_w,
            attendu
        );
    }
    assert_proche!(
        toiture
            .flux(&conditions(ModeCalcul::ThD, true, 3000))
            .phi_sh_s_w,
        100.0
    );
}

#[test]
fn un_masque_reduit_le_flux_solaire_du_mur() {
    // Masque lointain azimutal à 45° partout : soleil à 30°, direct supprimé ; restent le diffus
    // et le réfléchi, 50 + 50 = 100 W/m², soit 22 × 0,01 × 100 = 22 W.
    let catalogue = Catalogue {
        masques: vec![MasqueDefinition {
            id: "relief".into(),
            masque: Masque::LointainAzimutal {
                gamma_i_deg: vec![45.0; 36],
            },
        }],
        ..Catalogue::default()
    };
    let p = ParoiOpaque {
        id: "p".into(),
        type_paroi: "t".into(),
        a_k_m2: 22.0,
        exposition: Exposition {
            masques: vec!["relief".into()],
            ..exposition(0.0, 90.0)
        },
    };
    let mur = ParoiResolue::nouvelle(&p, &type_paroi(0.2, VERT, 0.01, 0.01), &catalogue);
    assert_proche!(mur.flux(&conditions(TH_B, false, 0)).phi_sh_s_w, 22.0);
}

#[test]
fn ensemble_des_parois_d_un_groupe() {
    // Maison d'exemple : 90 m² de murs à 0,2 ; 100 m² de toiture à 0,12 ; 100 m² de plancher à 0,15.
    let mur = type_paroi(0.2, VERT, 0.01, 0.01);
    let parois = vec![
        paroi(22.0, &mur, exposition(0.0, 90.0)),
        paroi(20.0, &mur, exposition(90.0, 90.0)),
        paroi(28.0, &mur, exposition(180.0, 90.0)),
        paroi(20.0, &mur, exposition(270.0, 90.0)),
        paroi(
            100.0,
            &type_paroi(0.12, ASC, 0.01, 0.01),
            exposition(0.0, 0.0),
        ),
        paroi(
            100.0,
            &type_paroi(0.15, DES, 0.0, 0.0),
            exposition(0.0, 180.0),
        ),
    ];
    let e = EnsembleParois::de(&parois);
    // Éq. (283) : 18 + 12 + 15 = 45 W/K. Éq. (286) et (287).
    assert_proche!(e.h_th_w_k, 45.0);
    assert_proche!(e.a_op_m2, 290.0);
    assert_proche!(e.a_opv_m2, 90.0);
    assert_proche!(e.a_ophh_m2, 100.0);
    assert_proche!(e.a_ophb_m2, 100.0);
    assert_proche!(e.h_th_opv_w_k, 18.0);
    assert_proche!(e.h_th_ophh_w_k, 12.0);
    assert_proche!(e.h_th_ophb_w_k, 15.0);
    // Éq. (284) et (285) : somme des flux des parois.
    let c = conditions(TH_B, false, 0);
    let total = paroi_opaque::flux_ensemble(&parois, &c);
    let somme: f64 = parois.iter().map(|p| p.flux(&c).phi_sh_s_w).sum();
    assert_proche!(total.phi_sh_s_w, somme);
    // Seule la toiture échange avec la voûte céleste : −39,6 W.
    assert_proche!(total.phi_sh_vc_w, -39.6);
    // Classes d'inclinaison (éq. 287) : verticale de 60 à 120° inclus.
    assert_eq!(Inclinaison::de(59.9), Inclinaison::VersLeHaut);
    assert_eq!(Inclinaison::de(60.0), Inclinaison::Verticale);
    assert_eq!(Inclinaison::de(120.0), Inclinaison::Verticale);
    assert_eq!(Inclinaison::de(120.1), Inclinaison::VersLeBas);
}

fn pont(exposition: Option<Exposition>) -> PontThermique {
    PontThermique {
        id: "pt".into(),
        psi_l_w_mk: 0.1,
        l_l_m: 40.0,
        sf_c_l: 0.02,
        sf_e_l: 0.05,
        exposition,
    }
}

#[test]
fn pont_thermique_en_description_simplifiee() {
    // §5.18.3, éq. (298) : quatre ponts verticaux de 10 m, au sud, à l'ouest, au nord et à l'est.
    let mut ponts = Vec::new();
    PontResolu::resoudre(&pont(None), &Catalogue::default(), &mut ponts);
    assert_eq!(ponts.len(), 4);
    assert!(ponts
        .iter()
        .all(|p| p.l_l_m == 10.0 && p.surface.verticale && p.masques.is_empty()));
    // Éq. (307) et (310) : 40 × 0,1 = 4 W/K, 40 m.
    let e = EnsemblePonts::de(&ponts);
    assert_proche!(e.h_th_w_k, 4.0);
    assert_proche!(e.l_m, 40.0);
    // Rayonnement : 792,8203230 W/m² au sud ; 100 W/m² (diffus et réfléchi) sur les trois autres,
    // le soleil étant dans le plan de l'est et de l'ouest et derrière le nord.
    // Éq. (300) : Φsh_s = 10 × 0,02 × (792,8203230 + 300) = 218,5640646 W.
    let f = pont_thermique::flux_ensemble(&ponts, &conditions(TH_B, false, 0));
    assert_proche!(f.phi_sh_s_w, 218.564_064_605_51);
    assert!(f.phi_sh_vc_w.abs() < 1e-9);
    // Éq. (302) et (303) : en Th-D, pendant la période de confort adaptatif, Sf-e = 0,05.
    let f = pont_thermique::flux_ensemble(&ponts, &conditions(ModeCalcul::ThD, true, 0));
    assert_proche!(f.phi_sh_s_w, 218.564_064_605_51 * 2.5);
}

#[test]
fn pont_thermique_en_description_detaillee() {
    // Pont thermique horizontal en toiture (β = 0), 40 m : rayonnement 500 W/m².
    // Éq. (300) : 40 × 0,02 × 500 = 400 W. Éq. (306) : 40 × (−82,5) × 0,1/25 = −13,2 W.
    let mut ponts = Vec::new();
    PontResolu::resoudre(
        &pont(Some(exposition(0.0, 0.0))),
        &Catalogue::default(),
        &mut ponts,
    );
    assert_eq!(ponts.len(), 1);
    let f = ponts[0].flux(&conditions(TH_B, false, 0));
    assert_proche!(f.phi_sh_s_w, 400.0);
    assert_proche!(f.phi_sh_vc_w, -13.2);
    assert_proche!(ponts[0].h_th_w_k(), 4.0);

    // Pont vertical plein sud avec une joue à gauche : pour les masques proches, le pont est une
    // paroi de 30 cm de large (§3.2.3). Soleil à Φ = 45° : dlg = 0,3 × tg(45°) = 0,3 ;
    // F = 1 − 0,3/0,3 = 0. Le direct est supprimé : restent 100 W/m², soit 40 × 0,02 × 100 = 80 W.
    let catalogue = Catalogue {
        masques: vec![MasqueDefinition {
            id: "joue".into(),
            masque: Masque::ProcheVerticalGauche {
                dvg_m: 0.3,
                dpg_m: 0.0,
            },
        }],
        ..Catalogue::default()
    };
    let e = Exposition {
        masques: vec!["joue".into()],
        ..exposition(0.0, 90.0)
    };
    let mut ponts = Vec::new();
    PontResolu::resoudre(&pont(Some(e)), &catalogue, &mut ponts);
    assert_proche!(ponts[0].surface.lp_m, 0.3);
    let mut c = conditions(TH_B, false, 0);
    c.soleil.psi_rad = 45f64.to_radians();
    assert_proche!(ponts[0].flux(&c).phi_sh_s_w, 80.0);
}
