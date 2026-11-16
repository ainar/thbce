// SPDX-License-Identifier: GPL-3.0-or-later
//! Émission équivalente du Bbio (Th-BCE 2020 §8.1) et bilan hydrique du Bbio (§8.3) : valeurs
//! calculées à la main et propriétés.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::bati::comportement_thermique::{
    AirEntrant, AirEquivalent, GroupeThermique, Reponse, Sollicitations, Temperatures,
};
use thbce_core::chauffage_refroidissement::bilan_hydrique::{
    calcul_humidite_spe_fin, omega_fin_bbio_kg_kg, EchangesHydriques,
};
use thbce_core::chauffage_refroidissement::emission::{
    emission, emission_bbio, puissance_matrice_w, Consignes, EmetteurEquivalent, Sollicitation,
};
use thbce_core::entree::InertieQuotidienne;

fn proche(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * (1.0 + b.abs())
}

macro_rules! assert_proche {
    ($a:expr, $b:expr) => {
        assert!(proche($a, $b, 1e-9), "{} obtenu, {} attendu", $a, $b)
    };
}

/// Températures d'un cas de la matrice : seules `θi` et `θrm` servent à la sonde ; les autres
/// reçoivent des valeurs distinctes pour suivre l'interpolation.
fn t(theta_i: f64, theta_rm: f64) -> Temperatures {
    Temperatures {
        theta_i_c: theta_i,
        theta_s_c: theta_rm + 0.1,
        theta_mq_c: theta_rm - 0.2,
        theta_rm_c: theta_rm,
        theta_op_c: 0.5 * (theta_i + theta_rm),
    }
}

/// Matrice fabriquée : évolution libre à θi = 17 °C et θrm = 16 °C ; 10 W/m² convectifs :
/// 19 et 17 °C ; 10 W/m² radiatifs : 18 et 18 °C. Les températures finales sont décalées de
/// 0,5 K.
fn matrice(decalage: f64) -> [Reponse; 3] {
    let cas = |i: f64, rm: f64| Reponse {
        moy: t(i + decalage, rm + decalage),
        fin: t(i + decalage + 0.5, rm + decalage + 0.5),
    };
    [cas(17.0, 16.0), cas(19.0, 17.0), cas(18.0, 18.0)]
}

fn consignes() -> Consignes {
    Consignes {
        theta_ii_ch_c: 19.0,
        theta_ii_fr_c: 26.0,
        aut_ch: true,
        aut_fr: true,
        i_clim: true,
    }
}

#[test]
fn emission_conventionnelle_du_bbio() {
    // §8.1.3.1.
    let e = EmetteurEquivalent::BBIO;
    assert_eq!(
        (
            e.p_sd,
            e.p_em_conv,
            e.delta_theta_vs_k,
            e.delta_theta_vt_k
        ),
        (0.5, 0.5, 0.0, 0.0)
    );
    // Éq. (801) : θsd = 0,5 × 17 + 0,5 × 16 = 16,5 °C.
    assert_proche!(e.theta_sd_c(&t(17.0, 16.0)), 16.5);
    // Puissance des cas de la matrice : 10 W/m² × 100 m² = 1 000 W.
    assert_eq!(puissance_matrice_w(100.0), 1000.0);
}

#[test]
fn besoin_de_chauffage_calcule_a_la_main() {
    // Agr = 100 m², consigne de 19 °C.
    // Éq. (801) : θsd_eq_ch = 16,5 °C < 19 °C : chauffage sollicité (éq. 802).
    // Éq. (804) : θi,moy_10 = 0,5 × 19 + 0,5 × 18 = 18,5 ; θrm,moy_10 = 0,5 × 17 + 0,5 × 18 = 17,5.
    // Éq. (805) : θsd_10 = 18 °C. Éq. (806) : a1 = 1000/(18 − 16,5) = 666,67 W/K.
    // Éq. (807) : φcrois = 666,67 × (19 − 16,5) = 1 666,67 W.
    let e = emission_bbio(&matrice(0.0), 100.0, &consignes());
    assert_eq!(e.sollicitation, Sollicitation::Chauffage);
    assert_proche!(e.phi_crois_w, 5000.0 / 3.0);
    assert_proche!(e.phi_util_w, 5000.0 / 3.0);
    assert_proche!(e.q_sys_ch_wh, 5000.0 / 3.0);
    assert_eq!(e.q_sys_fr_wh, 0.0);
    // Éq. (826) et (827) : φutil−conv/P = φutil−rad/P = 0,5 × 1,66667 = 0,83333.
    // θi,moy = 17 + 0,83333 × (19 − 17) + 0,83333 × (18 − 17) = 19,5 °C ;
    // θrm,moy = 16 + 0,83333 × 1 + 0,83333 × 2 = 18,5 °C ; la sonde voit bien 19 °C.
    assert_proche!(e.moy.theta_i_c, 19.5);
    assert_proche!(e.moy.theta_rm_c, 18.5);
    assert_proche!(EmetteurEquivalent::BBIO.theta_sd_c(&e.moy), 19.0);
    assert_proche!(e.moy.theta_op_c, 19.0);
    // Éq. (825) : mêmes coefficients sur les températures finales, décalées de 0,5 K.
    assert_proche!(e.fin.theta_i_c, 20.0);
    assert_proche!(e.fin.theta_rm_c, 19.0);
    assert_proche!(e.fin.theta_s_c, 19.1);
    assert_proche!(e.fin.theta_mq_c, 18.8);
}

#[test]
fn besoin_de_refroidissement_calcule_a_la_main() {
    // Même matrice décalée de 12 K : θsd_0 = 28,5 °C > 26 °C, refroidissement sollicité.
    // φcrois = 666,67 × (26 − 28,5) = −1 666,67 W.
    let e = emission_bbio(&matrice(12.0), 100.0, &consignes());
    assert_eq!(e.sollicitation, Sollicitation::Refroidissement);
    assert_proche!(e.phi_crois_w, -5000.0 / 3.0);
    assert_eq!(e.q_sys_ch_wh, 0.0);
    assert_proche!(e.q_sys_fr_wh, -5000.0 / 3.0);
    assert_proche!(EmetteurEquivalent::BBIO.theta_sd_c(&e.moy), 26.0);
    // Éq. (807) : un groupe qui ne peut pas être refroidi n'a pas de puissance négative.
    let sans_clim = emission_bbio(
        &matrice(12.0),
        100.0,
        &Consignes {
            i_clim: false,
            ..consignes()
        },
    );
    assert_eq!(sans_clim.phi_crois_w, 0.0);
    assert_eq!(sans_clim.q_sys_fr_wh, 0.0);
    assert_eq!(sans_clim.moy, matrice(12.0)[0].moy);
}

#[test]
fn sans_besoin_les_temperatures_sont_celles_de_l_evolution_libre() {
    // Matrice décalée de 5 K : θsd_0 = 21,5 °C, entre les consignes.
    let m = matrice(5.0);
    let e = emission_bbio(&m, 100.0, &consignes());
    assert_eq!(e.sollicitation, Sollicitation::Aucune);
    assert_eq!((e.phi_crois_w, e.phi_util_w), (0.0, 0.0));
    assert_eq!((e.q_sys_ch_wh, e.q_sys_fr_wh), (0.0, 0.0));
    assert_eq!((e.moy, e.fin), (m[0].moy, m[0].fin));
    // À la consigne exactement, pas de besoin (inégalités strictes de l'éq. 802).
    let e = emission_bbio(&matrice(2.5), 100.0, &consignes());
    assert_eq!(e.sollicitation, Sollicitation::Aucune);
}

#[test]
fn effet_des_saisons() {
    // Éq. (809) : hors saison de chauffage, la puissance requise n'est pas effective et les
    // températures restent celles de l'évolution libre.
    let m = matrice(0.0);
    let e = emission_bbio(
        &m,
        100.0,
        &Consignes {
            aut_ch: false,
            ..consignes()
        },
    );
    assert_eq!(e.sollicitation, Sollicitation::Chauffage);
    assert_proche!(e.phi_crois_w, 5000.0 / 3.0);
    assert_eq!((e.phi_util_w, e.q_sys_ch_wh), (0.0, 0.0));
    assert_eq!((e.moy, e.fin), (m[0].moy, m[0].fin));
    // De même pour le refroidissement hors saison de refroidissement.
    let e = emission_bbio(
        &matrice(12.0),
        100.0,
        &Consignes {
            aut_fr: false,
            ..consignes()
        },
    );
    assert_eq!((e.phi_util_w, e.q_sys_fr_wh), (0.0, 0.0));
}

#[test]
fn derives_de_consigne_d_un_emetteur_non_conventionnel() {
    // Éq. (800) : θi_eq_ch = 19 + 0,4 + 1,8 = 21,2 °C ; φcrois = 666,67 × (21,2 − 16,5) = 3 133,33 W.
    // Éq. (825) et (826) : la température opérative est diminuée de la dérive (2,2 K).
    let chaud = EmetteurEquivalent {
        delta_theta_vs_k: 0.4,
        delta_theta_vt_k: 1.8,
        ..EmetteurEquivalent::BBIO
    };
    let e = emission(
        &matrice(0.0),
        100.0,
        &chaud,
        &EmetteurEquivalent::BBIO,
        &consignes(),
    );
    assert_proche!(e.phi_crois_w, 4.7 * 2000.0 / 3.0);
    assert_proche!(chaud.theta_sd_c(&e.moy), 21.2);
    assert_proche!(e.moy.theta_op_c, 21.2 - 2.2);
}

/// Groupe et sollicitations réalistes : maison de 100 m², 10 m² de baies, inertie moyenne.
fn groupe_reel() -> (GroupeThermique, Sollicitations) {
    let g = GroupeThermique::nouveau(100.0, 10.0, &InertieQuotidienne::Moyenne);
    let s = Sollicitations {
        air: AirEquivalent::melange(&[AirEntrant {
            qm_kg_s: 0.04,
            theta_c: 5.0,
            omega_kg_kg: 0.004,
        }]),
        theta_e_c: 2.0,
        h_th_k_pt_w_k: 45.0,
        hg_es_w_k: 13.0,
        phi_sl_w: -20.0,
        phi_sh_w: -30.0,
        fs1_w: 200.0,
        fs3_w: 0.0,
        phi_conv_w: 150.0,
        phi_rad_w: 150.0,
    };
    (g, s)
}

#[test]
fn la_puissance_requise_tient_la_consigne_dans_le_modele_thermique() {
    // Le modèle thermique est linéaire en puissance émise : résoudre le pas de temps avec la
    // puissance trouvée par l'émission doit donner exactement la consigne à la sonde, et les
    // températures interpolées (éq. 825 et 826) doivent être celles de cette résolution.
    let (g, s) = groupe_reel();
    let m = g.matrice(&s, 17.0, 0.0).unwrap();
    let e = emission_bbio(&m, g.a_gr_m2, &consignes());
    assert_eq!(e.sollicitation, Sollicitation::Chauffage);
    assert!(e.q_sys_ch_wh > 500.0 && e.q_sys_ch_wh < 5000.0, "{}", e.q_sys_ch_wh);
    let direct = g
        .resoudre(&s, 17.0, 0.0, 0.5 * e.phi_util_w, 0.5 * e.phi_util_w)
        .unwrap();
    assert_proche!(EmetteurEquivalent::BBIO.theta_sd_c(&direct.moy), 19.0);
    for (a, b) in [(e.moy, direct.moy), (e.fin, direct.fin)] {
        assert_proche!(a.theta_i_c, b.theta_i_c);
        assert_proche!(a.theta_s_c, b.theta_s_c);
        assert_proche!(a.theta_mq_c, b.theta_mq_c);
        assert_proche!(a.theta_rm_c, b.theta_rm_c);
        assert_proche!(a.theta_op_c, b.theta_op_c);
    }
    // En été, avec une masse chaude : besoin de froid, consigne de 26 °C tenue.
    let s_ete = Sollicitations {
        theta_e_c: 32.0,
        air: AirEquivalent::melange(&[AirEntrant {
            qm_kg_s: 0.04,
            theta_c: 31.0,
            omega_kg_kg: 0.010,
        }]),
        fs1_w: 900.0,
        ..s
    };
    let m = g.matrice(&s_ete, 28.0, 0.0).unwrap();
    let e = emission_bbio(&m, g.a_gr_m2, &consignes());
    assert_eq!(e.sollicitation, Sollicitation::Refroidissement);
    assert!(e.q_sys_fr_wh < 0.0);
    let direct = g
        .resoudre(&s_ete, 28.0, 0.0, 0.5 * e.phi_util_w, 0.5 * e.phi_util_w)
        .unwrap();
    assert_proche!(EmetteurEquivalent::BBIO.theta_sd_c(&direct.moy), 26.0);
    assert_proche!(e.fin.theta_mq_c, direct.fin.theta_mq_c);
}

fn echanges() -> EchangesHydriques {
    // 0,04 kg/s d'air à 4 g/kg ; 0,2 kg/h d'apports internes ; 250 m³ ; 1,2 kg/m³.
    EchangesHydriques {
        somme_q_maj_kg_s: 0.04,
        somme_q_omega_maj_kg_s: 0.04 * 0.004,
        a_omega_int_kg_s: 0.2 / 3600.0,
        v_m3: 250.0,
        rho_i_kg_m3: 1.2,
    }
}

#[test]
fn bilan_hydrique_calcule_a_la_main() {
    // Éq. (831), Ratem_eff = 1, sans recirculation, ωini = 6 g/kg, T = 3600 s :
    // A = (0,00016 + 0,2/3600)/0,04 − 0,006 = 0,005388889 − 0,006 = −0,000611111 ;
    // B = 0,04/(250 × 1,2) = 1,333333e-4 s⁻¹ ; B·T = 0,48 ; 1 − e^(−0,48) = 0,381216608 ;
    // ωfin = 0,006 − 0,000611111 × 0,381216608 = 0,005767034.
    let omega = omega_fin_bbio_kg_kg(0.006, &echanges());
    assert!(
        proche(omega, 0.006 - 0.000_611_111_111_111 * 0.381_216_608_109_13, 1e-12),
        "{omega}"
    );
    // Éq. (834) : c'est CalculHumiditeSpeFin(1 ; 0 ; 0 ; 3600 ; ωini).
    assert_eq!(
        omega,
        calcul_humidite_spe_fin(1.0, 0.0, 0.0, 3600.0, 0.006, &echanges())
    );
}

#[test]
fn proprietes_du_bilan_hydrique() {
    let e = echanges();
    // Régime permanent : ω∞ = (Σ Qω + Aωint)/Σ Q ; partant de cette valeur, rien ne change, et
    // sur une durée très longue toute valeur initiale y conduit.
    let omega_inf = (e.somme_q_omega_maj_kg_s + e.a_omega_int_kg_s) / e.somme_q_maj_kg_s;
    assert_proche!(omega_fin_bbio_kg_kg(omega_inf, &e), omega_inf);
    assert_proche!(
        calcul_humidite_spe_fin(1.0, 0.0, 0.0, 1.0e7, 0.012, &e),
        omega_inf
    );
    // Sur une durée nulle, l'humidité initiale est conservée.
    assert_eq!(calcul_humidite_spe_fin(1.0, 0.0, 0.0, 0.0, 0.006, &e), 0.006);
    // Sans débit d'air : limite de la formule, les apports internes s'accumulent,
    // ωfin = 0,006 + (0,2/3600) × 3600/(250 × 1,2) = 0,006 + 0,2/300.
    let ferme = EchangesHydriques {
        somme_q_maj_kg_s: 0.0,
        somme_q_omega_maj_kg_s: 0.0,
        ..e
    };
    assert_proche!(omega_fin_bbio_kg_kg(0.006, &ferme), 0.006 + 0.2 / 300.0);
    // Un débit infime redonne la même valeur : la limite est continue.
    let presque_ferme = EchangesHydriques {
        somme_q_maj_kg_s: 1e-9,
        somme_q_omega_maj_kg_s: 4e-12,
        ..e
    };
    assert!(proche(
        omega_fin_bbio_kg_kg(0.006, &presque_ferme),
        0.006 + 0.2 / 300.0,
        1e-6
    ));
}
