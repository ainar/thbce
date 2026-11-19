// SPDX-License-Identifier: GPL-3.0-or-later
//! Ventilation mécanique double flux et centrales de traitement d'air : fiche `C_VEN_CTA et
//! Double_flux` (Th-BCE 2020 §6.3), en mode « zone neutre » (§6.3.3.4).
//!
//! Périmètre :
//! - préprocesseur : puissance moyenne des ventilateurs en habitation (éq. 390 à 394),
//!   coefficients de transfert des conduits (éq. 395), statut de l'efficacité (éq. 396 à 398) ;
//! - au pas horaire : débits massiques soufflés et repris (éq. 420 à 425, 429 à 431, 443 à 448,
//!   451 à 453), air neuf, recyclage et free cooling des centrales de traitement d'air (éq. 433
//!   à 441), air rejeté (éq. 454 à 456), ventilateurs (éq. 458 à 465, 474 et 519), bypass
//!   (éq. 492, 494 et 495), efficacité de l'échangeur (éq. 496 à 505) et consommation d'un
//!   échangeur rotatif (éq. 497), sécurité antigel (éq. 506 à 511, tableau 62), sorties de
//!   l'échangeur (éq. 512 à 514), boîte de mélange (éq. 515 à 518), batteries de préchauffage
//!   (éq. 522 à 525) et de prérefroidissement (éq. 539 à 548), air soufflé (éq. 560 à 564),
//!   consommations (éq. 565 et 570).
//!
//! - mode actif des centrales à charge variable (types 3 et 4 avec batteries chaude ou froide
//!   régulées sur la charge, §6.3.3.5) : déperditions en zone neutre (éq. 571), température
//!   aux bouches (éq. 582 à 594), sortie de la centrale (éq. 589, 595), énergies de chauffage
//!   et de refroidissement, latente comprise (éq. 596 à 612), ventilateurs (éq. 614) ;
//!   lectures `docs/questions/Q095.md`.
//!
//! Humidification d'hiver et rafraîchissement évaporatif : [`super::humidification`].
//! Hors de cette version, refusés par la validation : surventilation mécanique, puits
//! climatique ou hydraulique ; l'air neuf est donc l'air extérieur (éq. 413).
//!
//! Les numéros d'équation sont ceux de la fiche 6.3, sauf mention « fiche 6.2 » (les deux
//! fiches emploient les mêmes numéros, voir `docs/reference/index/anomalies.md`).

use crate::entree::{Bypass, EcoulementEchangeur, RegulationAirNeuf, StatutEchangeur, TypeCta};

use super::mecanique_sf::C_PA_J_KG_K;

/// « εutile_max » : efficacité maximale de l'échangeur en l'absence de valeur certifiée ou
/// justifiée (tableau 61).
pub const EPSILON_UTILE_MAX: f64 = 0.5;
/// « θs,ech,rep,LIM » par défaut, en °C : « autres cas » du tableau 62.
pub const THETA_S_ECH_REP_LIM_DEFAUT_C: f64 = 5.0;
/// « θs,ech,rep,LIM » par défaut d'un échangeur rotatif en bâtiment non résidentiel, en °C
/// (tableau 62).
pub const THETA_S_ECH_REP_LIM_ROTATIF_NR_C: f64 = -5.0;
/// « θs,ech,rep,LIM » par défaut d'un échangeur à plaques en bâtiment non résidentiel, en °C
/// (tableau 62).
pub const THETA_S_ECH_REP_LIM_PLAQUES_NR_C: f64 = 0.0;
/// « Lv » : chaleur de vaporisation de l'eau, en J/kg (tableau 61).
pub const L_V_J_KG: f64 = 2_501_000.0;
/// « A », « B », « C » : constantes de l'humidité spécifique à saturation (tableau 61, éq. 477).
pub const A_SAT: f64 = 18.8161;
/// Voir [`A_SAT`].
pub const B_SAT: f64 = 4110.34;
/// Voir [`A_SAT`].
pub const C_SAT: f64 = 235.0;
/// Écart entre la température du fluide de la batterie froide et celle de sa surface, en K
/// (éq. 539).
pub const ECART_BATTERIE_FROIDE_K: f64 = 2.0;

/// « θs,ech,rep,LIM » par défaut selon le type d'échangeur et l'usage.
///
/// Th-BCE 2020 §6.3.3.4.6.4, tableau 62 : échangeurs rotatifs en bâtiment non résidentiel,
/// −5 °C ; échangeurs à plaques en bâtiment non résidentiel, 0 °C ; autres cas, 5 °C. Le
/// bâtiment non résidentiel est lu comme l'usage de la zone autre que l'habitation.
pub fn theta_s_ech_rep_lim_defaut_c(rotatif: bool, habitation: bool) -> f64 {
    match (habitation, rotatif) {
        (true, _) => THETA_S_ECH_REP_LIM_DEFAUT_C,
        (false, true) => THETA_S_ECH_REP_LIM_ROTATIF_NR_C,
        (false, false) => THETA_S_ECH_REP_LIM_PLAQUES_NR_C,
    }
}

/// Vrai si la centrale fonctionne au débit nominal (mode actif) au pas de temps : en
/// occupation, en relance, ou en inoccupation après un besoin de chauffage ou de
/// refroidissement d'un groupe au pas précédent.
///
/// Th-BCE 2020 §6.3.3.4.3.2, conditions des éq. (421), (423), (444), (446), (460), (462) et
/// (464) : `isvent(h) = 1` ou `irelance(h) = 1` ou (`isvent(h−1) = 0` et `Isbesoin_ch(h−1)`)
/// ou (`isvent(h−1) = 0` et `Isbesoin_fr(h−1)`). Lecture de `irelance(h)` :
/// `docs/questions/Q090.md`.
pub fn conditions_debit_nominal(
    i_vent: bool,
    relance: bool,
    i_vent_h1: bool,
    besoin_ch_h1: bool,
    besoin_fr_h1: bool,
) -> bool {
    i_vent || relance || (!i_vent_h1 && (besoin_ch_h1 || besoin_fr_h1))
}

/// Régime des débits d'une centrale de traitement d'air en zone neutre : `true` pour les
/// débits nominaux du mode actif (indice CH), `false` pour ceux de la zone neutre (ZN).
///
/// Th-BCE 2020 §6.3.3.4.3.2 : types 2 et 3, débit nominal dans les conditions de
/// [`conditions_debit_nominal`], sinon zone neutre (éq. 421 à 424, 444 à 447) ; type 4, débit
/// de zone neutre en toute circonstance hors rafraîchissement adiabatique (éq. 425 et 448).
pub fn debits_nominaux(type_cta: TypeCta, conditions: bool) -> bool {
    match type_cta {
        TypeCta::DebitConstant | TypeCta::DebitConstantTemperatureVariable => conditions,
        TypeCta::DebitVariable => false,
    }
}

/// Puissances des ventilateurs de reprise et de soufflage d'une centrale de traitement d'air
/// au pas de temps, en W.
///
/// Th-BCE 2020 §6.3.3.4.4 : types 2 et 3, puissances du mode actif dans les conditions de
/// [`conditions_debit_nominal`], de zone neutre en inoccupation sinon (éq. 460 à 463) ; type 4,
/// puissances de zone neutre en occupation dans ces conditions, en inoccupation sinon (éq. 464
/// et 465). `ch`, `zn_occ` et `zn_inocc` : couples (reprise, soufflage).
pub fn p_vent_cta_w(
    type_cta: TypeCta,
    conditions: bool,
    ch: (f64, f64),
    zn_occ: (f64, f64),
    zn_inocc: (f64, f64),
) -> (f64, f64) {
    match (type_cta, conditions) {
        (_, false) => zn_inocc,
        (TypeCta::DebitVariable, true) => zn_occ,
        (_, true) => ch,
    }
}

/// « TauxAN_min » d'une bouche de soufflage : rapport du débit de zone neutre au débit du mode
/// actif.
///
/// Th-BCE 2020 fiche 6.2, éq. (393) en occupation et (397) en inoccupation. 1 si le débit du
/// mode actif est nul.
pub fn taux_an_min(q_zn_m3_h: f64, q_ch_m3_h: f64) -> f64 {
    if q_ch_m3_h > 0.0 {
        q_zn_m3_h / q_ch_m3_h
    } else {
        1.0
    }
}

/// « qm,air_neuf(h) » : débit d'air neuf d'une centrale de traitement d'air, en kg/s.
///
/// Th-BCE 2020 §6.3.3.4.3.3, éq. (435) à (439) : débit minimal (hygiénique) si l'air neuf est
/// plus froid que `θENC` ; interpolation jusqu'au débit soufflé entre `θENC` et `θENF` ;
/// au-delà, débit minimal en régulation standard, tout air neuf en régulation optimisée si
/// l'air neuf est plus froid que l'air repris (`θi,moy`, moyenne barycentrique des débits
/// repris).
#[allow(clippy::too_many_arguments)]
pub fn qm_air_neuf_kg_s(
    qm_air_neuf_min_kg_s: f64,
    qm_sou_cond_kg_s: f64,
    theta_an_c: f64,
    theta_enc_c: f64,
    theta_enf_c: f64,
    regulation: RegulationAirNeuf,
    theta_i_moy_c: f64,
) -> f64 {
    if theta_an_c <= theta_enc_c {
        // Éq. (435).
        qm_air_neuf_min_kg_s
    } else if theta_an_c < theta_enf_c {
        // Éq. (436).
        qm_air_neuf_min_kg_s
            + (qm_sou_cond_kg_s - qm_air_neuf_min_kg_s) * (theta_an_c - theta_enc_c)
                / (theta_enf_c - theta_enc_c)
    } else {
        match regulation {
            // Éq. (437).
            RegulationAirNeuf::Standard => qm_air_neuf_min_kg_s,
            // Éq. (438) et (439).
            RegulationAirNeuf::Optimisee if theta_an_c < theta_i_moy_c => qm_sou_cond_kg_s,
            RegulationAirNeuf::Optimisee => qm_air_neuf_min_kg_s,
        }
    }
}

/// « θsouf,mel(h) » ou « ωsouf,mel(h) » : grandeur de l'air soufflé après la boîte de mélange.
///
/// Th-BCE 2020 §6.3.3.4.7, éq. (515) et (516) : moyenne de l'air repris recyclé (après le
/// ventilateur d'extraction, `θextr2`, `ωextr2`) et de l'air neuf après l'échangeur (`θsouf1`,
/// `ωsouf1`), pondérée par les débits recyclé et d'air neuf.
pub fn melange(qm_sou_cond_kg_s: f64, qm_air_neuf_kg_s: f64, x_extr2: f64, x_souf1: f64) -> f64 {
    ((qm_sou_cond_kg_s - qm_air_neuf_kg_s) * x_extr2 + qm_air_neuf_kg_s * x_souf1)
        / qm_sou_cond_kg_s
}

/// Sortie de la batterie de préchauffage : `(θsouf,prechaud, Qprechaud)` en °C et en Wh.
///
/// Th-BCE 2020 §6.3.3.4.9.1, éq. (522) à (525) : si `θext < θex,prechaud`, l'air est porté à
/// la consigne `θcons,prechaud` s'il est plus froid ; sinon la batterie est sans effet.
pub fn prechauffage(
    theta_souf2_c: f64,
    theta_ext_c: f64,
    theta_cons_c: f64,
    theta_ex_c: f64,
    qm_sou_cond_kg_s: f64,
) -> (f64, f64) {
    if theta_ext_c < theta_ex_c {
        let theta = theta_cons_c.max(theta_souf2_c);
        (
            theta,
            C_PA_J_KG_K * qm_sou_cond_kg_s * (theta - theta_souf2_c).max(0.0),
        )
    } else {
        (theta_souf2_c, 0.0)
    }
}

/// « ωsat(θ) » : humidité spécifique à saturation, en kg/kg (éq. 541 et 477).
pub fn omega_sat_kg_kg(theta_c: f64) -> f64 {
    0.001 * libm::exp(A_SAT - B_SAT / (theta_c + C_SAT))
}

/// Sortie de la batterie de prérefroidissement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prerefroidissement {
    /// « θsouf,3(h) », en °C.
    pub theta_souf3_c: f64,
    /// « ωsouf,3(h) », en kg/kg.
    pub omega_souf3_kg_kg: f64,
    /// « Qprefroid(h) », en Wh, négatif ou nul.
    pub q_prefroid_wh: f64,
}

/// Batterie de prérefroidissement régulée sur la température extérieure.
///
/// Th-BCE 2020 §6.3.3.4.10, éq. (539) à (548) : active si `θext > θex,prefroid` ; surface de
/// la batterie à `θdim,fr + 2 °C`, facteur de bypass `BP`, déshumidification `Δωfr`, sortie à
/// la consigne `θcons,prefroid` au plus. L'énergie de l'éq. (545) est lue avec
/// `MIN(θsouf3 − θsouf2 ; 0)` (`docs/questions/Q091.md`) : négative, comme toute demande de
/// froid (éq. 603), sensible et latente.
pub fn prerefroidissement(
    theta_souf2_c: f64,
    omega_souf2_kg_kg: f64,
    theta_ext_c: f64,
    theta_cons_c: f64,
    theta_ex_c: f64,
    theta_dim_fr_c: f64,
    qm_sou_cond_kg_s: f64,
) -> Prerefroidissement {
    if theta_ext_c <= theta_ex_c || theta_souf2_c <= theta_cons_c {
        // Éq. (546) à (548) ; sans écart à la consigne, BP = 1 (éq. 540) et rien ne change.
        return Prerefroidissement {
            theta_souf3_c: theta_souf2_c,
            omega_souf3_kg_kg: omega_souf2_kg_kg,
            q_prefroid_wh: 0.0,
        };
    }
    // Éq. (539) et (540).
    let theta_batt = theta_dim_fr_c + ECART_BATTERIE_FROIDE_K;
    let bp = if theta_souf2_c > theta_batt {
        ((theta_cons_c - theta_batt) / (theta_souf2_c - theta_batt)).min(1.0)
    } else {
        1.0
    };
    // Éq. (541) à (543).
    let d_omega = (omega_sat_kg_kg(theta_batt) - omega_souf2_kg_kg).min(0.0) * (1.0 - bp);
    let omega_souf3 = omega_souf2_kg_kg + d_omega;
    // Éq. (544).
    let theta_souf3 = theta_souf2_c - (theta_souf2_c - theta_cons_c).max(0.0);
    // Éq. (545), lecture Q91.
    let q = qm_sou_cond_kg_s
        * (C_PA_J_KG_K * (theta_souf3 - theta_souf2_c).min(0.0) + L_V_J_KG * d_omega);
    Prerefroidissement {
        theta_souf3_c: theta_souf3,
        omega_souf3_kg_kg: omega_souf3,
        q_prefroid_wh: q,
    }
}

/// « Cpv » : chaleur massique de la vapeur d'eau, en J/(kg.K) (tableau 61).
pub const C_PV_J_KG_K: f64 = 1830.0;
/// Température maximale de soufflage possible d'une batterie chaude, en °C (éq. 597).
pub const THETA_SOUF_MAX_C: f64 = 60.0;

/// « Cpa + Cpv·ω » : chaleur massique de l'air humide, en J/(kg.K) (éq. 571, 583 et 598).
pub fn cp_air_humide_j_kg_k(omega_kg_kg: f64) -> f64 {
    C_PA_J_KG_K + C_PV_J_KG_K * omega_kg_kg
}

/// « θsou,bouches(h) » qui assure la charge des groupes, en °C.
///
/// Th-BCE 2020 §6.3.3.5.8.1, éq. (583) et (584) : `θi,moy + (ΣQsys + PZN)/(cp·qm,sou,spec)`,
/// `ΣQsys` en Wh sur un pas d'une heure (W), positif en chauffage et négatif en
/// refroidissement ; `PZN` de l'éq. (571), en W.
pub fn theta_sou_bouches_charge_c(
    theta_i_moy_c: f64,
    somme_q_sys_wh: f64,
    p_zn_w: f64,
    cp_j_kg_k: f64,
    qm_sou_spec_kg_s: f64,
) -> f64 {
    theta_i_moy_c + (somme_q_sys_wh + p_zn_w) / (cp_j_kg_k * qm_sou_spec_kg_s)
}

/// Régime d'une centrale en mode actif : taux de charge et température aux bouches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChargeCentrale {
    /// « τCH/FR(h) ».
    pub tau: f64,
    /// « θsou,bouches(h) », en °C.
    pub theta_sou_bouches_c: f64,
}

/// Taux de charge d'une centrale à débit variable (type 4) soufflant à `θsou,bouches`.
///
/// Th-BCE 2020 §6.3.3.5.8.1, éq. (586), (587), (592) et (593) : puissance de la centrale à la
/// température `theta_sou_c` au-delà de la zone neutre, `Psys,max = cp·qm,sou,spec·(θsou −
/// θi,moy) − PZN` (signe de `PZN` de l'éq. 586 retenu aussi en refroidissement, où l'éq. 592
/// imprime `+ PZN` : `docs/questions/Q095.md`), puis `τ = ΣQsys / Psys,max`. Au-delà de 1, ou
/// si la centrale ne peut pas répondre à sa température nominale (puissance nulle ou de signe
/// opposé à la demande), la température est relevée (ou abaissée) selon l'éq. (583) ou (584)
/// et `τ = 1`. Sans report d'énergie des générations au pas précédent dans cette version, les
/// éq. (588) et (594) sont sans effet.
pub fn charge_debit_variable(
    somme_q_sys_wh: f64,
    theta_sou_c: f64,
    theta_i_moy_c: f64,
    p_zn_w: f64,
    cp_j_kg_k: f64,
    qm_sou_spec_kg_s: f64,
) -> ChargeCentrale {
    let p_max = p_sys_max_w(
        theta_sou_c,
        theta_i_moy_c,
        p_zn_w,
        cp_j_kg_k,
        qm_sou_spec_kg_s,
    );
    let tau = if somme_q_sys_wh == 0.0 {
        // Mode actif engagé par le seul report d'énergie (éq. 588 et 594) : aucune charge.
        0.0
    } else if p_max * somme_q_sys_wh > 0.0 {
        somme_q_sys_wh / p_max
    } else {
        f64::INFINITY
    };
    if tau > 1.0 {
        ChargeCentrale {
            tau: 1.0,
            theta_sou_bouches_c: theta_sou_bouches_charge_c(
                theta_i_moy_c,
                somme_q_sys_wh,
                p_zn_w,
                cp_j_kg_k,
                qm_sou_spec_kg_s,
            ),
        }
    } else {
        ChargeCentrale {
            tau,
            theta_sou_bouches_c: theta_sou_c,
        }
    }
}

/// « Psys_CH/FR_max(h) », en W (éq. 586 et 592, lecture Q95 sur le signe de `PZN`).
pub fn p_sys_max_w(
    theta_sou_c: f64,
    theta_i_moy_c: f64,
    p_zn_w: f64,
    cp_j_kg_k: f64,
    qm_sou_spec_kg_s: f64,
) -> f64 {
    cp_j_kg_k * qm_sou_spec_kg_s * (theta_sou_c - theta_i_moy_c) - p_zn_w
}

/// « expH^g,s(h) » d'un conduit de soufflage (éq. 589).
pub fn exp_h(h_cond_souf_ext_w_k: f64, qm_sou_cond_kg_s: f64) -> f64 {
    libm::exp(-h_cond_souf_ext_w_k / (C_PA_J_KG_K * qm_sou_cond_kg_s))
}

/// Énergie de la batterie chaude d'une centrale en mode actif, en Wh.
///
/// Th-BCE 2020 §6.3.3.5.8.3, éq. (596), (598) et (599) : `MAX(0 ; τ·cp·qm,sou,cond·(θsouf,CH −
/// θsouf2))`, `θsouf,CH` étant la température à la sortie de la centrale (éq. 595).
pub fn q_chaud_rec_wh(
    tau: f64,
    cp_j_kg_k: f64,
    qm_sou_cond_kg_s: f64,
    theta_souf_ch_c: f64,
    theta_souf2_c: f64,
) -> f64 {
    (tau * cp_j_kg_k * qm_sou_cond_kg_s * (theta_souf_ch_c - theta_souf2_c)).max(0.0)
}

/// Batterie froide d'une centrale en mode actif.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FroidCentrale {
    /// « Qfroid(rec)(h) » : énergie sensible et latente, en Wh, négative ou nulle (éq. 611).
    pub q_froid_wh: f64,
    /// « Qsys_lat_fr(h) » : part latente, en Wh, négative ou nulle (éq. 610).
    pub q_lat_wh: f64,
    /// « ωsouf_CH/FR(h) » : humidité à la sortie de la batterie, en kg/kg (éq. 609 et 612).
    pub omega_souf_kg_kg: f64,
    /// Sous-dimensionnement de la batterie (`θsouf,CH/FR < θbatt`, éq. 605).
    pub sous_dimensionnee: bool,
}

/// Énergie de la batterie froide d'une centrale en mode actif.
///
/// Th-BCE 2020 §6.3.3.5.8.4, éq. (601) à (612) : énergie sensible `MIN(0 ; τ·cp·qm,sou,cond·
/// (θsouf,CH/FR − θsouf2))` ; si elle n'est pas nulle, surface de la batterie à `θdim,fr +
/// 2 °C` (éq. 604), facteur de bypass `BP` (éq. 606), déshumidification `Δωfr` (éq. 607 et
/// 608) et énergie latente `τ·qm,sou,cond·Lv·Δωfr` (éq. 610) ajoutée (éq. 611).
pub fn froid_rec(
    tau: f64,
    cp_j_kg_k: f64,
    qm_sou_cond_kg_s: f64,
    theta_souf_fr_c: f64,
    theta_souf2_c: f64,
    omega_souf2_kg_kg: f64,
    theta_dim_fr_c: f64,
) -> FroidCentrale {
    // Éq. (602) et (603).
    let sensible =
        (tau * cp_j_kg_k * qm_sou_cond_kg_s * (theta_souf_fr_c - theta_souf2_c)).min(0.0);
    if sensible >= 0.0 {
        // Éq. (612).
        return FroidCentrale {
            q_froid_wh: 0.0,
            q_lat_wh: 0.0,
            omega_souf_kg_kg: omega_souf2_kg_kg,
            sous_dimensionnee: false,
        };
    }
    // Éq. (604) à (606) ; air entrant plus froid que la surface de la batterie : BP = 1, sans
    // déshumidification (comme pour le prérefroidissement).
    let theta_batt = theta_dim_fr_c + ECART_BATTERIE_FROIDE_K;
    let bp = if theta_souf2_c > theta_batt {
        ((theta_souf_fr_c - theta_batt) / (theta_souf2_c - theta_batt)).min(1.0)
    } else {
        1.0
    };
    // Éq. (607) à (609).
    let d_omega = (omega_sat_kg_kg(theta_batt) - omega_souf2_kg_kg).min(0.0) * (1.0 - bp);
    // Éq. (610) et (611).
    let lat = tau * qm_sou_cond_kg_s * L_V_J_KG * d_omega;
    FroidCentrale {
        q_froid_wh: sensible + lat,
        q_lat_wh: lat,
        omega_souf_kg_kg: omega_souf2_kg_kg + d_omega,
        sous_dimensionnee: theta_souf_fr_c < theta_batt,
    }
}

/// « Wvent(rec)(h) » : consommation des ventilateurs d'une centrale en mode actif, en Wh
/// (éq. 614) : `τ·(Pvent,rep(rec) + Pvent,sou(rec)) + (1 − τ)·Wvent(h)`.
pub fn w_vent_rec_wh(tau: f64, p_vent_ch_w: (f64, f64), w_vent_zn_wh: f64) -> f64 {
    tau * (p_vent_ch_w.0 + p_vent_ch_w.1) + (1.0 - tau) * w_vent_zn_wh
}

/// « εt » : efficacité de l'échangeur en représentation simplifiée, après prise en compte du
/// statut de la valeur saisie.
///
/// Th-BCE 2020 §6.3.3.3.3, éq. (396) à (398) : certifiée, `εsaisi` ; justifiée,
/// `0,9 × εsaisi` ; déclarée, `MIN(0,8 × εsaisi ; εutile_max)`.
pub fn epsilon_t(epsilon_saisi: f64, statut: StatutEchangeur) -> f64 {
    match statut {
        StatutEchangeur::Certifie => epsilon_saisi,
        StatutEchangeur::Justifie => 0.9 * epsilon_saisi,
        StatutEchangeur::Declare => (0.8 * epsilon_saisi).min(EPSILON_UTILE_MAX),
    }
}

/// « ε(h) » d'un échangeur en représentation détaillée, ramenée aux écarts de températures du
/// côté de l'air neuf.
///
/// Th-BCE 2020 §6.3.3.4.6.3, éq. (499) à (505), à partir des débits capacitifs
/// `Cair_neuf = Cpa × qm,air_neuf` et `Cair_rejeté = Cpa × qm,air_extrait` (W/K). Nulle si l'un
/// des deux débits est nul.
pub fn epsilon_detaille(
    ua_w_k: f64,
    ecoulement: EcoulementEchangeur,
    c_air_neuf_w_k: f64,
    c_air_rejete_w_k: f64,
) -> f64 {
    let c_min = c_air_neuf_w_k.min(c_air_rejete_w_k);
    let c_max = c_air_neuf_w_k.max(c_air_rejete_w_k);
    if c_min <= 0.0 {
        return 0.0;
    }
    // Éq. (499) et (500).
    let c = c_min / c_max;
    let nut = ua_w_k / c_min;
    let epsilon = match ecoulement {
        // Éq. (501) et (502).
        EcoulementEchangeur::ContreCourant => {
            if (1.0 - c).abs() < 1e-12 {
                nut / (nut + 1.0)
            } else {
                let e = libm::exp(-nut * (1.0 - c));
                (1.0 - e) / (1.0 - c * e)
            }
        }
        // Éq. (503).
        EcoulementEchangeur::CourantsParalleles => (1.0 - libm::exp(-nut * (1.0 + c))) / (1.0 + c),
        // Éq. (504).
        EcoulementEchangeur::CourantsCroisesBrasses => {
            if nut <= 0.0 {
                0.0
            } else {
                1.0 / (1.0 / (1.0 - libm::exp(-nut)) + c / (1.0 - libm::exp(-nut * c)) - 1.0 / nut)
            }
        }
    };
    // Éq. (505).
    c_min / c_air_neuf_w_k * epsilon
}

/// « δbypass(h) » : activation du bypass de l'échangeur.
///
/// Th-BCE 2020 §6.3.3.4.6.2, éq. (494) et (495) : trois conditions simultanées, avec les
/// consignes « hiver » en saison de chauffage et « été » le reste de l'année :
/// `θan < θi,fin(h−1)`, `θan > θext,bypass` et `θi,fin(h−1) > θint,bypass`.
pub fn delta_bypass(
    bypass: &Bypass,
    saison_de_chauffage: bool,
    theta_an_c: f64,
    theta_i_fin_c: f64,
) -> bool {
    let (theta_ext_bypass, theta_int_bypass) = if saison_de_chauffage {
        (
            bypass.theta_ext_bypass_hiver_c,
            bypass.theta_int_bypass_hiver_c,
        )
    } else {
        (bypass.theta_ext_bypass_ete_c, bypass.theta_int_bypass_ete_c)
    };
    theta_an_c < theta_i_fin_c && theta_an_c > theta_ext_bypass && theta_i_fin_c > theta_int_bypass
}

/// « θeech,min(h) » : température d'air neuf en deçà de laquelle l'échangeur givre, en °C.
///
/// Th-BCE 2020 §6.3.3.4.6.4, éq. (506) : `θextr3 + (θs,ech,rep,LIM − θextr3)/ε`, pour `ε > 0`.
pub fn theta_e_ech_min_c(theta_extr3_c: f64, theta_s_ech_rep_lim_c: f64, epsilon: f64) -> f64 {
    theta_extr3_c + (theta_s_ech_rep_lim_c - theta_extr3_c) / epsilon
}

/// Effet de la sécurité antigel ou du risque de givrage sur l'échangeur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Antigel {
    /// « ε(h) » après prise en compte du risque de givrage.
    pub epsilon: f64,
    /// « θsouf,ag(h) » : température de l'air neuf à l'entrée de l'échangeur, en °C.
    pub theta_souf_ag_c: f64,
    /// « Wag(h) » : énergie fournie pour la fonction antigel, en Wh (éq. 508 : `Cpa × qm ×
    /// ΔT` sur un pas de temps d'une heure).
    pub w_ag_wh: f64,
}

/// Sécurité antigel de l'échangeur.
///
/// Th-BCE 2020 §6.3.3.4.6.4 : avec un échangeur d'efficacité non nulle, en saison de chauffage
/// effective et avec un antigel, l'air neuf est préchauffé jusqu'à `θeech,min` (éq. 507 et 508) ;
/// sans antigel ou hors saison de chauffage, l'échangeur est contourné quand l'air neuf est
/// plus froid que `θeech,min` (éq. 509 à 511). `qm_air_neuf_kg_s` : débit d'air neuf (éq. 432).
pub fn antigel(
    epsilon: f64,
    is_antigel: bool,
    saison_de_chauffage: bool,
    theta_an_c: f64,
    theta_extr3_c: f64,
    theta_s_ech_rep_lim_c: f64,
    qm_air_neuf_kg_s: f64,
) -> Antigel {
    let sans_effet = Antigel {
        epsilon,
        theta_souf_ag_c: theta_an_c,
        w_ag_wh: 0.0,
    };
    // « si ε est nul, l'algorithme correspondant à la sécurité antigel n'est pas appelé ».
    if epsilon <= 0.0 {
        return sans_effet;
    }
    let theta_min = theta_e_ech_min_c(theta_extr3_c, theta_s_ech_rep_lim_c, epsilon);
    if is_antigel && saison_de_chauffage {
        // Éq. (507) et (508).
        let theta_souf_ag_c = theta_an_c.max(theta_min);
        Antigel {
            epsilon,
            theta_souf_ag_c,
            w_ag_wh: C_PA_J_KG_K * qm_air_neuf_kg_s * (theta_souf_ag_c - theta_an_c).max(0.0),
        }
    } else if theta_an_c < theta_min {
        // Éq. (509) à (511).
        Antigel {
            epsilon: 0.0,
            ..sans_effet
        }
    } else {
        sans_effet
    }
}

/// « θsouf1(h) » et « θair,rejeté(h) » : températures de l'air soufflé et de l'air rejeté à la
/// sortie de l'échangeur, en °C.
///
/// Th-BCE 2020 §6.3.3.4.6.5, éq. (512) et (513).
pub fn sorties_echangeur_c(theta_souf_ag_c: f64, theta_extr3_c: f64, epsilon: f64) -> (f64, f64) {
    let ecart = theta_extr3_c - theta_souf_ag_c;
    (
        theta_souf_ag_c + epsilon * ecart,
        theta_extr3_c - epsilon * ecart,
    )
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn statut_de_l_efficacite() {
        assert_eq!(epsilon_t(0.85, StatutEchangeur::Certifie), 0.85);
        assert!((epsilon_t(0.85, StatutEchangeur::Justifie) - 0.765).abs() < 1e-12);
        // Déclarée : MIN(0,68 ; 0,5).
        assert_eq!(epsilon_t(0.85, StatutEchangeur::Declare), 0.5);
        assert!((epsilon_t(0.5, StatutEchangeur::Declare) - 0.4).abs() < 1e-12);
    }

    #[test]
    fn efficacite_detaillee_a_la_main() {
        // Débits équilibrés (C = 1), NUT = 3 à contre-courant : 3/4.
        let e = epsilon_detaille(150.0, EcoulementEchangeur::ContreCourant, 50.0, 50.0);
        assert!((e - 0.75).abs() < 1e-12);
        // Courants parallèles, C = 1, NUT = 3 : (1 − e⁻⁶)/2.
        let e = epsilon_detaille(150.0, EcoulementEchangeur::CourantsParalleles, 50.0, 50.0);
        assert!((e - (1.0 - libm::exp(-6.0)) / 2.0).abs() < 1e-12);
        // Contre-courant, C = 0,5 (air rejeté le plus faible), NUT = 2 : ε sur Cmin, puis
        // ramenée à l'air neuf (éq. 505) × 0,5.
        let e = epsilon_detaille(50.0, EcoulementEchangeur::ContreCourant, 50.0, 25.0);
        let x = libm::exp(-2.0 * 0.5);
        assert!((e - 0.5 * (1.0 - x) / (1.0 - 0.5 * x)).abs() < 1e-12);
        // Croisés brassés, C = 1, NUT = 1 : 1/(2/(1 − e⁻¹) − 1).
        let e = epsilon_detaille(
            50.0,
            EcoulementEchangeur::CourantsCroisesBrasses,
            50.0,
            50.0,
        );
        let y = 1.0 - libm::exp(-1.0);
        assert!((e - 1.0 / (2.0 / y - 1.0)).abs() < 1e-12);
        assert_eq!(
            epsilon_detaille(50.0, EcoulementEchangeur::ContreCourant, 0.0, 50.0),
            0.0
        );
    }

    #[test]
    fn bypass_selon_la_saison() {
        let b = Bypass {
            theta_ext_bypass_hiver_c: 16.0,
            theta_ext_bypass_ete_c: 12.0,
            theta_int_bypass_hiver_c: 24.0,
            theta_int_bypass_ete_c: 22.0,
        };
        // Été : air neuf à 15 °C, intérieur à 25 °C.
        assert!(delta_bypass(&b, false, 15.0, 25.0));
        // Hiver : air neuf sous la consigne de 16 °C.
        assert!(!delta_bypass(&b, true, 15.0, 25.0));
        // Air neuf plus chaud que l'intérieur.
        assert!(!delta_bypass(&b, false, 26.0, 25.0));
        // Intérieur sous la consigne d'été.
        assert!(!delta_bypass(&b, false, 15.0, 21.0));
    }

    #[test]
    fn valeurs_par_defaut_du_tableau_62() {
        assert_eq!(theta_s_ech_rep_lim_defaut_c(false, true), 5.0);
        assert_eq!(theta_s_ech_rep_lim_defaut_c(true, true), 5.0);
        assert_eq!(theta_s_ech_rep_lim_defaut_c(true, false), -5.0);
        assert_eq!(theta_s_ech_rep_lim_defaut_c(false, false), 0.0);
    }

    #[test]
    fn regime_et_ventilateurs_des_cta() {
        // Occupation, relance, besoin au pas précédent en inoccupation.
        assert!(conditions_debit_nominal(true, false, true, false, false));
        assert!(conditions_debit_nominal(false, true, false, false, false));
        assert!(conditions_debit_nominal(false, false, false, true, false));
        assert!(conditions_debit_nominal(false, false, false, false, true));
        // Besoin au pas précédent, mais en occupation au pas précédent : zone neutre.
        assert!(!conditions_debit_nominal(false, false, true, true, false));
        assert!(debits_nominaux(TypeCta::DebitConstant, true));
        assert!(debits_nominaux(
            TypeCta::DebitConstantTemperatureVariable,
            true
        ));
        assert!(!debits_nominaux(TypeCta::DebitVariable, true));
        assert!(!debits_nominaux(TypeCta::DebitConstant, false));
        let (ch, occ, inocc) = ((300.0, 310.0), (150.0, 160.0), (40.0, 50.0));
        assert_eq!(
            p_vent_cta_w(TypeCta::DebitConstant, true, ch, occ, inocc),
            ch
        );
        assert_eq!(
            p_vent_cta_w(TypeCta::DebitConstant, false, ch, occ, inocc),
            inocc
        );
        assert_eq!(
            p_vent_cta_w(TypeCta::DebitVariable, true, ch, occ, inocc),
            occ
        );
        assert_eq!(
            p_vent_cta_w(TypeCta::DebitVariable, false, ch, occ, inocc),
            inocc
        );
        assert_eq!(taux_an_min(1000.0, 4000.0), 0.25);
        assert_eq!(taux_an_min(1000.0, 0.0), 1.0);
    }

    #[test]
    fn air_neuf_et_melange_a_la_main() {
        // Débit minimal 0,4 kg/s, soufflé 1,6 kg/s, plage tout air neuf de 12 à 18 °C.
        let an = |t: f64, r: RegulationAirNeuf, ti: f64| {
            qm_air_neuf_kg_s(0.4, 1.6, t, 12.0, 18.0, r, ti)
        };
        assert_eq!(an(5.0, RegulationAirNeuf::Standard, 22.0), 0.4);
        // 15 °C : la moitié de la plage, 0,4 + 1,2 × 3/6 = 1,0 kg/s.
        assert!((an(15.0, RegulationAirNeuf::Standard, 22.0) - 1.0).abs() < 1e-12);
        assert_eq!(an(20.0, RegulationAirNeuf::Standard, 22.0), 0.4);
        assert_eq!(an(20.0, RegulationAirNeuf::Optimisee, 22.0), 1.6);
        assert_eq!(an(24.0, RegulationAirNeuf::Optimisee, 22.0), 0.4);
        // Mélange : 1,2 kg/s recyclés à 21 °C, 0,4 kg/s d'air neuf à 15 °C : 19,5 °C.
        assert!((melange(1.6, 0.4, 21.0, 15.0) - 19.5).abs() < 1e-12);
    }

    #[test]
    fn batteries_a_la_main() {
        // Préchauffage : air à 8 °C porté à 16 °C, 0,5 kg/s : 1006 × 0,5 × 8 = 4 024 Wh.
        let (t, q) = prechauffage(8.0, -2.0, 16.0, 10.0, 0.5);
        assert_eq!(t, 16.0);
        assert!((q - 4024.0).abs() < 1e-9);
        // Air extérieur au-dessus du seuil : sans effet.
        assert_eq!(prechauffage(8.0, 12.0, 16.0, 10.0, 0.5), (8.0, 0.0));
        // Air déjà plus chaud que la consigne : sans effet.
        assert_eq!(prechauffage(18.0, -2.0, 16.0, 10.0, 0.5), (18.0, 0.0));

        // Prérefroidissement : air à 30 °C et 12 g/kg, consigne 18 °C, fluide à 7 °C (surface
        // à 9 °C), 0,5 kg/s. BP = 9/21 ; ωsat(9 °C) = 10⁻³ exp(18,8161 − 4110,34/244).
        let r = prerefroidissement(30.0, 0.012, 32.0, 18.0, 25.0, 7.0, 0.5);
        let omega_sat = 0.001 * libm::exp(18.8161 - 4110.34 / 244.0);
        let d_omega = (omega_sat - 0.012) * (1.0 - 9.0 / 21.0);
        assert!(d_omega < 0.0);
        assert_eq!(r.theta_souf3_c, 18.0);
        assert!((r.omega_souf3_kg_kg - (0.012 + d_omega)).abs() < 1e-15);
        let q = 0.5 * (1006.0 * -12.0 + 2_501_000.0 * d_omega);
        assert!((r.q_prefroid_wh - q).abs() < 1e-9);
        // Air extérieur sous le seuil : sans effet.
        let r = prerefroidissement(30.0, 0.012, 20.0, 18.0, 25.0, 7.0, 0.5);
        assert_eq!(r.q_prefroid_wh, 0.0);
        assert_eq!(r.theta_souf3_c, 30.0);
    }

    #[test]
    fn antigel_a_la_main() {
        // Air repris à 20 °C, ε = 0,8, limite à 5 °C : θeech,min = 20 + (5 − 20)/0,8 = 1,25 °C.
        assert!((theta_e_ech_min_c(20.0, 5.0, 0.8) - 1.25).abs() < 1e-12);
        // Avec antigel en saison de chauffage, air neuf à −5 °C : préchauffé à 1,25 °C.
        let a = antigel(0.8, true, true, -5.0, 20.0, 5.0, 0.04);
        assert!((a.theta_souf_ag_c - 1.25).abs() < 1e-12);
        assert!((a.w_ag_wh - 1006.0 * 0.04 * 6.25).abs() < 1e-9);
        assert_eq!(a.epsilon, 0.8);
        // Sans antigel : échangeur contourné.
        let a = antigel(0.8, false, true, -5.0, 20.0, 5.0, 0.04);
        assert_eq!(a.epsilon, 0.0);
        assert_eq!(a.theta_souf_ag_c, -5.0);
        assert_eq!(a.w_ag_wh, 0.0);
        // Air neuf plus chaud que le seuil : rien.
        let a = antigel(0.8, false, true, 5.0, 20.0, 5.0, 0.04);
        assert_eq!(a.epsilon, 0.8);
        // Sorties de l'échangeur : 1,25 + 0,8 × 18,75 = 16,25 °C ; rejet 20 − 15 = 5 °C.
        let (s, r) = sorties_echangeur_c(1.25, 20.0, 0.8);
        assert!((s - 16.25).abs() < 1e-12);
        assert!((r - 5.0).abs() < 1e-12);
    }

    #[test]
    fn mode_actif_a_la_main() {
        let proche = |a: f64, b: f64| assert!((a - b).abs() < 1e-9, "{a} ≠ {b}");
        // Éq. (571) et (583) : cp = 1006 + 1830 × 0,008 = 1020,64 J/(kg.K).
        let cp = cp_air_humide_j_kg_k(0.008);
        proche(cp, 1020.64);
        proche(
            theta_sou_bouches_charge_c(20.0, 1000.0, -500.0, cp, 0.5),
            20.0 + 500.0 / 510.32,
        );
        // Éq. (586) et (587) : Psys,max = 1006 × 0,5 × (30 − 20) + 1000 = 6 030 W.
        let c = charge_debit_variable(2000.0, 30.0, 20.0, -1000.0, 1006.0, 0.5);
        proche(c.tau, 2000.0 / 6030.0);
        proche(c.theta_sou_bouches_c, 30.0);
        // τ > 1 : température relevée selon l'éq. (583), τ = 1.
        let c = charge_debit_variable(8000.0, 30.0, 20.0, -1000.0, 1006.0, 0.5);
        proche(c.tau, 1.0);
        proche(c.theta_sou_bouches_c, 20.0 + 7000.0 / 503.0);
        // Éq. (592) et (593), signe de PZN de l'éq. (586) (lecture Q95) : 503 × (14 − 24) − 500.
        let c = charge_debit_variable(-2000.0, 14.0, 24.0, 500.0, 1006.0, 0.5);
        proche(c.tau, 2000.0 / 5530.0);
        // Nominale inopérante (soufflage plus froid que l'ambiance en chauffage) : éq. (583).
        let c = charge_debit_variable(1000.0, 18.0, 20.0, 0.0, 1006.0, 0.5);
        proche(c.tau, 1.0);
        proche(c.theta_sou_bouches_c, 20.0 + 1000.0 / 503.0);
        // Mode actif engagé par le seul report (éq. 588) : aucune charge, τ = 0 à la
        // nominale ; la centrale porte ensuite τ à 1.
        let c = charge_debit_variable(0.0, 30.0, 20.0, -1000.0, 1006.0, 0.5);
        proche(c.tau, 0.0);
        proche(c.theta_sou_bouches_c, 30.0);
        // Éq. (589).
        proche(exp_h(10.0, 0.5), libm::exp(-10.0 / 503.0));
        // Éq. (598) et (599).
        proche(q_chaud_rec_wh(0.5, 1020.0, 0.6, 35.0, 18.0), 5202.0);
        proche(q_chaud_rec_wh(1.0, 1020.0, 0.6, 15.0, 18.0), 0.0);
        // Éq. (602) à (611) : sensible 503 × (14 − 26) = −6 036 Wh ; θbatt = 9 °C,
        // BP = 5/17, ωsat(9 °C) = 0,001·exp(18,8161 − 4110,34/244).
        let f = froid_rec(1.0, 1006.0, 0.5, 14.0, 26.0, 0.012, 7.0);
        let w_sat = 0.001 * libm::exp(18.8161 - 4110.34 / 244.0);
        let d_omega = (w_sat - 0.012) * (1.0 - 5.0 / 17.0);
        proche(f.q_lat_wh, 0.5 * 2_501_000.0 * d_omega);
        proche(f.q_froid_wh, -6036.0 + 0.5 * 2_501_000.0 * d_omega);
        proche(f.omega_souf_kg_kg, 0.012 + d_omega);
        assert!(!f.sous_dimensionnee);
        // Éq. (612) : pas de froid, humidité inchangée.
        let f = froid_rec(1.0, 1006.0, 0.5, 27.0, 26.0, 0.012, 7.0);
        assert_eq!((f.q_froid_wh, f.omega_souf_kg_kg), (0.0, 0.012));
        // Éq. (614) : 0,4 × 800 + 0,6 × 600.
        proche(w_vent_rec_wh(0.4, (300.0, 500.0), 600.0), 680.0);
    }
}
