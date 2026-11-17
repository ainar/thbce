// SPDX-License-Identifier: GPL-3.0-or-later
//! Chauffe-eau solaires individuels : assemblage « ballon base solaire » (Th-BCE 2020 §9.14,
//! avec la boucle solaire de la fiche 9.15 pour générateur de base) et saisie des kits CESI
//! (fiche `PR2_IdCESI`, §9.23).
//!
//! Le ballon a l'échangeur solaire en zone basse (éq. 1947) ; l'appoint est intégré au ballon
//! (« Type_prod_stockage = 1 »), séparé instantané (« = 3 », éq. 1800 à 1807) ou absent
//! (« = 0 »). Consigne conventionnelle de 55 °C (éq. 1948).
//!
//! Lectures : `docs/questions.md`, Q52.

use super::assemblage_ballon::ParametresSsc;
use super::assemblage_ballon::{AssemblageBallon, RegulationGenerateur};
use super::ballon_de_stockage::{ua_s_defaut_w_k, Ballon, NatureBallon, F_AUX_DEFAUT};
use super::boucle_solaire::{
    performances_par_defaut, BoucleSolaire, BoucleSolaireHoraire, CapteursExposes, CumulsBoucle,
    RegulationBoucle, TypeCapteur, ALPHA_P_CONV, DELTA_THETA_REG_HABITATION_K, ISR_MIN_CONV_W_M2,
    M_CP_CONV_W_K_M2, UA_HX_CONV_W_K_M2,
};
use super::cesci::{
    BallonCentral, CumulsCentral, LectureEntreeCescai, ProductionCollective, TypeProduction,
};
use super::gestion_regulation_ballon::{nb_iter_vp, Programmation, DELTA_THETA_DEFAUT_K};
use crate::entree::{
    AppointCesi, BallonSaisi, BoucleSolaireSaisie, CapteursSolaires, Catalogue, ChauffeEauSolaire,
    ChauffeEauSolaireCollectif, DescriptionCesi, NatureBallonSaisie, PerformancesCapteurs,
    PertesBallon, ProgrammationBallon, RaccordementSsc, RegulationBoucleSaisie,
    SystemeSolaireCombine, TransfertBoucle, TypeCapteurSolaire, TypePcad,
};
use crate::resultats::ValeurParDefaut;

/// « Δθreg » d'un kit CESI caractérisé selon la norme NF EN 12976, en K (éq. 2273).
pub const DELTA_THETA_REG_KIT_K: f64 = 2.0;

/// Paramètres Th-BCE d'un kit CESI (éq. 2263 à 2273) : `(boucle, Vtot, (UA)S)`.
pub fn parametres_kit(k: &crate::entree::KitCesi) -> (BoucleSolaire, f64, f64) {
    let a = k.surface_m2;
    // Éq. (2266) et (2267).
    let eta0 = k.ac_etoile_m2 / (0.8 * a);
    let boucle = BoucleSolaire {
        a_m2: a,
        eta0,
        a1_w_m2k: k.uc_etoile_w_m2k * eta0,
        // Éq. (2268) à (2270).
        a2_w_m2k2: 0.0,
        // « Kθ » : sortie de l'outil IdCESI (§9.23.3.2.3), requise (lecture Q52).
        k_theta: k.k_theta,
        ua_te_w_k: 0.0,
        ua_ti_w_k: 0.0,
        // « Les autres paramètres du modèle de chauffe-eau sont des valeurs conventionnelles ou
        // les valeurs par défaut » : échangeur, débit et αp du tableau 292.
        ua_hx_w_k: Some(UA_HX_CONV_W_K_M2 * a),
        p_np_w: k.p_np_w,
        alpha_p: ALPHA_P_CONV,
        m_cp_w_k: M_CP_CONV_W_K_M2 * a,
        // Éq. (2273).
        regulation: RegulationBoucle::Temperature {
            delta_theta_reg_k: DELTA_THETA_REG_KIT_K,
        },
    };
    // Éq. (2264) et (2271).
    (boucle, k.v_tot_l, k.us_w_k)
}

/// Résout un chauffe-eau solaire saisi et validé ; `appoint(id)` donne l'indice du générateur
/// d'appoint intégré dans la génération.
pub fn resoudre(
    c: &ChauffeEauSolaire,
    catalogue: &Catalogue,
    appoint: impl Fn(&str) -> Option<usize>,
) -> Option<AssemblageBallon> {
    let mut defauts = Vec::new();
    let defaut = |champ: &str, valeur: f64, regle: &str| ValeurParDefaut {
        champ: champ.to_owned(),
        valeur,
        regle: regle.to_owned(),
    };
    let (boucle, v_tot, ua_s, theta_max, h_rel_ech, f_aux_kit) = match &c.description {
        DescriptionCesi::Composants {
            capteurs,
            boucle,
            ballon,
        } => {
            let (b, v, ua, tmax, h) = resoudre_composants(capteurs, boucle, ballon, &mut defauts);
            (b, v, ua, tmax, h, None)
        }
        DescriptionCesi::Kit(k) => {
            let (boucle, v, ua) = parametres_kit(k);
            let regle_kit = "Th-BCE 2020 §9.23.3.2.2, éq. (2266) à (2273) : paramètres du kit \
                             CESI ; échangeur, débit et αp conventionnels du tableau 292";
            defauts.push(defaut("kit.eta0", boucle.eta0, regle_kit));
            defauts.push(defaut("kit.a1_w_m2k", boucle.a1_w_m2k, regle_kit));
            (boucle, v, ua, k.theta_max_c, 0.0, k.f_aux)
        }
    };
    let (appoint, appoint_separe) = match &c.appoint {
        AppointCesi::Aucun => (None, false),
        AppointCesi::Separe => (None, true),
        AppointCesi::Integre(a) => {
            let f = a.f_aux.or(f_aux_kit).unwrap_or_else(|| {
                defauts.push(defaut(
                    "appoint.f_aux",
                    F_AUX_DEFAUT,
                    "Th-BCE 2020 §9.9.3.1 : fraction du ballon chauffée par l'appoint par \
                     défaut",
                ));
                F_AUX_DEFAUT
            });
            if a.delta_theta_k.is_none() {
                defauts.push(defaut(
                    "appoint.delta_theta_k",
                    DELTA_THETA_DEFAUT_K,
                    "Th-BCE 2020 §9.10.3.2.2 : hystérésis du thermostat par défaut, 2 K",
                ));
            }
            let zone = |z: u8| usize::from(z.clamp(1, 4)) - 1;
            (
                Some((
                    RegulationGenerateur {
                        generateur: appoint(&a.generateur)?,
                        programmation: match a.programmation {
                            ProgrammationBallon::Permanent => Programmation::Permanent,
                            ProgrammationBallon::Nuit => Programmation::Nuit,
                            ProgrammationBallon::Jour => Programmation::Jour,
                        },
                        delta_theta_k: a.delta_theta_k.unwrap_or(DELTA_THETA_DEFAUT_K),
                        z_gen: zone(a.z_ap),
                        z_reg: zone(a.z_reg),
                        h_rel_ech: 0.0,
                    },
                    f,
                )),
                false,
            )
        }
    };
    let f_aux = appoint.map(|(_, f)| f);
    let mut a = AssemblageBallon::nouveau(
        c.id.clone(),
        f64::from(c.nb_assemblages.max(1)),
        Ballon::nouveau(v_tot, ua_s, f_aux, theta_max),
        RegulationGenerateur {
            // La boucle solaire n'est pas un générateur de la génération.
            generateur: usize::MAX,
            // §9.10.3.2.2 : la boucle est enclenchée dès qu'elle peut réchauffer le ballon.
            programmation: Programmation::Permanent,
            delta_theta_k: DELTA_THETA_DEFAUT_K,
            // Éq. (1947) et (1980) : échangeur et sonde en zone basse.
            z_gen: 0,
            z_reg: 0,
            h_rel_ech,
        },
        appoint.map(|(r, _)| r),
        // Éq. (1950) : 4 itérations sans appoint intégré.
        nb_iter_vp(f_aux),
    );
    a.valeurs_par_defaut = defauts;
    a.appoint_separe = appoint_separe;
    a.solaire = Some(Box::new(BoucleSolaireHoraire {
        boucle,
        capteurs: CapteursExposes::nouveaux(&c.exposition_capteurs, catalogue),
        cumuls: CumulsBoucle::default(),
    }));
    Some(a)
}

/// Résout un système solaire combiné saisi et validé (fiches 9.18 à 9.20) ; `appoint(id)`
/// donne l'indice du générateur d'appoint dans la génération.
pub fn resoudre_ssc(
    c: &SystemeSolaireCombine,
    catalogue: &Catalogue,
    appoint: impl Fn(&str) -> Option<usize>,
) -> Option<AssemblageBallon> {
    let mut defauts = Vec::new();
    let (boucle, v_tot, ua_s, theta_max, h_rel_ech) =
        resoudre_composants(&c.capteurs, &c.boucle, &c.ballon, &mut defauts);
    let a = &c.appoint;
    let f_aux = a.f_aux.unwrap_or_else(|| {
        defauts.push(ValeurParDefaut {
            champ: "appoint.f_aux".to_owned(),
            valeur: F_AUX_DEFAUT,
            regle: "Th-BCE 2020 §9.9.3.1 : fraction du ballon chauffée par l'appoint par défaut"
                .to_owned(),
        });
        F_AUX_DEFAUT
    });
    if a.delta_theta_k.is_none() {
        defauts.push(ValeurParDefaut {
            champ: "appoint.delta_theta_k".to_owned(),
            valeur: DELTA_THETA_DEFAUT_K,
            regle: "Th-BCE 2020 §9.10.3.2.2 : hystérésis du thermostat par défaut, 2 K".to_owned(),
        });
    }
    let mut assemblage = AssemblageBallon::nouveau(
        c.id.clone(),
        // Éq. (2133) et (2147) : un seul assemblage.
        1.0,
        Ballon::nouveau(v_tot, ua_s, Some(f_aux), theta_max),
        RegulationGenerateur {
            generateur: usize::MAX,
            programmation: Programmation::Permanent,
            delta_theta_k: DELTA_THETA_DEFAUT_K,
            // Éq. (2128) et (2142) : échangeur solaire en zone 1.
            z_gen: 0,
            z_reg: 0,
            h_rel_ech,
        },
        Some(RegulationGenerateur {
            generateur: appoint(&a.generateur)?,
            programmation: match a.programmation {
                ProgrammationBallon::Permanent => Programmation::Permanent,
                ProgrammationBallon::Nuit => Programmation::Nuit,
                ProgrammationBallon::Jour => Programmation::Jour,
            },
            delta_theta_k: a.delta_theta_k.unwrap_or(DELTA_THETA_DEFAUT_K),
            // Éq. (2130) et (2144) : échangeur et sonde de l'appoint en zone 3.
            z_gen: 2,
            z_reg: 2,
            h_rel_ech: 0.0,
        }),
        nb_iter_vp(Some(f_aux)),
    );
    assemblage.valeurs_par_defaut = defauts;
    assemblage.ssc = Some(match c.raccordement {
        RaccordementSsc::Independant {
            delta_theta_reg_boucle_chauffage_k,
        } => ParametresSsc {
            raccorde: false,
            delta_theta_reg_bcl_k: delta_theta_reg_boucle_chauffage_k,
        },
        RaccordementSsc::Raccorde => ParametresSsc {
            raccorde: true,
            delta_theta_reg_bcl_k: 0.0,
        },
    });
    assemblage.solaire = Some(Box::new(BoucleSolaireHoraire {
        boucle,
        capteurs: CapteursExposes::nouveaux(&c.exposition_capteurs, catalogue),
        cumuls: CumulsBoucle::default(),
    }));
    Some(assemblage)
}

/// Résout une production solaire collective à appoints décentralisés saisie et validée
/// (fiches 9.16 et 9.17) ; `appoint(id)` donne l'indice d'un générateur d'appoint dans la
/// génération.
pub fn resoudre_collectif(
    c: &ChauffeEauSolaireCollectif,
    catalogue: &Catalogue,
    appoint: impl Fn(&str) -> Option<usize>,
) -> Option<ProductionCollective> {
    let mut defauts = Vec::new();
    let boucle = resoudre_boucle(&c.capteurs, &c.boucle, &mut defauts);
    let type_production = match c.type_pcad {
        TypePcad::Cesci => TypeProduction::Cesci,
        TypePcad::Cescai => TypeProduction::Cescai,
        TypePcad::CesciAppointSepare => TypeProduction::CesciAppointSepare,
    };
    let mut ensembles = Vec::with_capacity(c.ensembles.len());
    for (k, en) in c.ensembles.iter().enumerate() {
        let mut d = Vec::new();
        let champ = format!("ensembles[{k}]");
        let ua_s = ua_s_ballon_w_k(&en.ballon, &format!("{champ}.ballon"), &mut d);
        let regulation = match &en.appoint {
            None => None,
            Some(a) => {
                let f = a.f_aux.unwrap_or_else(|| {
                    d.push(ValeurParDefaut {
                        champ: format!("{champ}.appoint.f_aux"),
                        valeur: F_AUX_DEFAUT,
                        regle: "Th-BCE 2020 §9.9.3.1 : fraction du ballon chauffée par \
                                l'appoint par défaut"
                            .to_owned(),
                    });
                    F_AUX_DEFAUT
                });
                if a.delta_theta_k.is_none() {
                    d.push(ValeurParDefaut {
                        champ: format!("{champ}.appoint.delta_theta_k"),
                        valeur: DELTA_THETA_DEFAUT_K,
                        regle: "Th-BCE 2020 §9.10.3.2.2 : hystérésis du thermostat par \
                                défaut, 2 K"
                            .to_owned(),
                    });
                }
                let zone = |z: u8| usize::from(z.clamp(1, 4)) - 1;
                Some((
                    RegulationGenerateur {
                        generateur: appoint(&a.generateur)?,
                        programmation: match a.programmation {
                            ProgrammationBallon::Permanent => Programmation::Permanent,
                            ProgrammationBallon::Nuit => Programmation::Nuit,
                            ProgrammationBallon::Jour => Programmation::Jour,
                        },
                        delta_theta_k: a.delta_theta_k.unwrap_or(DELTA_THETA_DEFAUT_K),
                        z_gen: zone(a.z_ap),
                        z_reg: zone(a.z_reg),
                        h_rel_ech: 0.0,
                    },
                    f,
                ))
            }
        };
        let f_aux = regulation.map(|(_, f)| f);
        let mut a = AssemblageBallon::nouveau(
            format!("{}/{}", c.id, en.id),
            f64::from(en.nb_ballons.max(1)),
            Ballon::nouveau(en.ballon.v_tot_l, ua_s, f_aux, en.ballon.theta_max_c),
            RegulationGenerateur {
                // L'énergie de la boucle ou du réseau est injectée : pas de générateur de base.
                generateur: usize::MAX,
                programmation: Programmation::Permanent,
                delta_theta_k: DELTA_THETA_DEFAUT_K,
                // Éq. (1980) et (2066) : échangeur et sonde en zone basse.
                z_gen: 0,
                z_reg: 0,
                h_rel_ech: 0.0,
            },
            regulation.map(|(r, _)| r),
            // Éq. (1950) : 4 itérations sans appoint intégré.
            nb_iter_vp(f_aux),
        );
        a.appoint_separe = type_production == TypeProduction::CesciAppointSepare;
        a.valeurs_par_defaut = d;
        ensembles.push(a);
    }
    let central = match (type_production, &c.ballon_central) {
        (TypeProduction::Cescai, Some(b)) => {
            let ua_s = ua_s_ballon_w_k(b, "ballon_central", &mut defauts);
            Some(BallonCentral {
                ballon: Ballon::nouveau(b.v_tot_l, ua_s, None, b.theta_max_c),
                cumuls: CumulsCentral::default(),
            })
        }
        _ => None,
    };
    if let Some(a) = ensembles.first_mut() {
        // Valeurs par défaut de la boucle et du ballon central, portées par le premier
        // ensemble.
        a.valeurs_par_defaut.splice(0..0, defauts);
    }
    Some(ProductionCollective {
        id: c.id.clone(),
        type_production,
        boucle: BoucleSolaireHoraire {
            boucle,
            capteurs: CapteursExposes::nouveaux(&c.exposition_capteurs, catalogue),
            cumuls: CumulsBoucle::default(),
        },
        ensembles,
        central,
        lecture_entree: LectureEntreeCescai::default(),
    })
}

/// Paramètres d'une boucle solaire et de son ballon décrits composant par composant :
/// `(boucle, Vtot, (UA)S, θmax, hrelech)` ; les valeurs par défaut appliquées sont ajoutées à
/// `defauts`.
fn resoudre_composants(
    capteurs: &CapteursSolaires,
    boucle: &BoucleSolaireSaisie,
    ballon: &BallonSaisi,
    defauts: &mut Vec<ValeurParDefaut>,
) -> (BoucleSolaire, f64, f64, f64, f64) {
    let b = resoudre_boucle(capteurs, boucle, defauts);
    let ua_s = ua_s_ballon_w_k(ballon, "ballon", defauts);
    (
        b,
        ballon.v_tot_l,
        ua_s,
        ballon.theta_max_c,
        boucle.h_rel_ech.unwrap_or(0.0),
    )
}

/// « (UA)S » d'un ballon saisi, en W/K, après correction de statut (éq. 1766 à 1768) ; la
/// valeur par défaut éventuelle est ajoutée à `defauts` sous le champ `{champ}.pertes`.
pub(crate) fn ua_s_ballon_w_k(
    ballon: &BallonSaisi,
    champ: &str,
    defauts: &mut Vec<ValeurParDefaut>,
) -> f64 {
    match ballon.pertes {
        PertesBallon::Certifie { ua_s_w_k } => ua_s_w_k,
        PertesBallon::Justifie { ua_s_w_k } => 1.1 * ua_s_w_k,
        PertesBallon::Defaut { nature } => {
            let ua = ua_s_defaut_w_k(
                match nature {
                    NatureBallonSaisie::EffetJouleHorizontal => NatureBallon::EffetJouleHorizontal,
                    NatureBallonSaisie::EffetJouleVertical => NatureBallon::EffetJouleVertical,
                    NatureBallonSaisie::Autre => NatureBallon::Autre,
                },
                ballon.v_tot_l,
            );
            defauts.push(ValeurParDefaut {
                champ: format!("{champ}.pertes.defaut"),
                valeur: ua,
                regle: "Th-BCE 2020 §9.9.3.1, tableau 282 et éq. (1768) : coefficient de \
                        pertes par défaut du ballon, en W/K"
                    .to_owned(),
            });
            ua
        }
    }
}

/// Paramètres d'une boucle solaire décrite composant par composant (tableau 292) ; les
/// valeurs par défaut appliquées sont ajoutées à `defauts`.
pub(crate) fn resoudre_boucle(
    capteurs: &CapteursSolaires,
    boucle: &BoucleSolaireSaisie,
    defauts: &mut Vec<ValeurParDefaut>,
) -> BoucleSolaire {
    let defaut = |champ: &str, valeur: f64, regle: &str| ValeurParDefaut {
        champ: champ.to_owned(),
        valeur,
        regle: regle.to_owned(),
    };
    let a = capteurs.surface_m2;
    let (eta0, a1, a2) = match capteurs.performances {
        PerformancesCapteurs::Certifie {
            eta0,
            a1_w_m2k,
            a2_w_m2k2,
        } => (eta0, a1_w_m2k, a2_w_m2k2),
        PerformancesCapteurs::Defaut { type_capteur } => {
            let p = performances_par_defaut(match type_capteur {
                TypeCapteurSolaire::NonVitre => TypeCapteur::NonVitre,
                TypeCapteurSolaire::Vitre => TypeCapteur::Vitre,
                TypeCapteurSolaire::Tubulaire => TypeCapteur::Tubulaire,
            });
            let regle = "Th-BCE 2020 §9.15.3.1, tableau 293 : performances par défaut \
                         des capteurs solaires";
            defauts.push(defaut("capteurs.performances.eta0", p.0, regle));
            defauts.push(defaut("capteurs.performances.a1_w_m2k", p.1, regle));
            defauts.push(defaut("capteurs.performances.a2_w_m2k2", p.2, regle));
            p
        }
    };
    let ua_hx = match boucle.transfert {
        TransfertBoucle::Echangeur { ua_hx_w_k } => Some(ua_hx_w_k.unwrap_or_else(|| {
            let v = UA_HX_CONV_W_K_M2 * a;
            defauts.push(defaut(
                "boucle.transfert.echangeur.ua_hx_w_k",
                v,
                "Th-BCE 2020 §9.15.2, tableau 292 : (UA)hx conventionnel, 100·A",
            ));
            v
        })),
        TransfertBoucle::SansEchangeur => None,
    };
    let alpha_p = boucle.alpha_p.unwrap_or_else(|| {
        defauts.push(defaut(
            "boucle.alpha_p",
            ALPHA_P_CONV,
            "Th-BCE 2020 §9.15.2, tableau 292 : αp conventionnel",
        ));
        ALPHA_P_CONV
    });
    let m_cp = boucle.m_cp_w_k.unwrap_or_else(|| {
        let v = M_CP_CONV_W_K_M2 * a;
        defauts.push(defaut(
            "boucle.m_cp_w_k",
            v,
            "Th-BCE 2020 §9.15.2, tableau 292 : débit calorifique conventionnel, 58·A",
        ));
        v
    });
    let regulation = match boucle.regulation {
        // Éq. (1968) : maisons individuelles et logements collectifs.
        RegulationBoucleSaisie::Temperature => RegulationBoucle::Temperature {
            delta_theta_reg_k: DELTA_THETA_REG_HABITATION_K,
        },
        RegulationBoucleSaisie::Irradiance { isr_min_w_m2 } => RegulationBoucle::Irradiance {
            isr_min_w_m2: isr_min_w_m2.unwrap_or_else(|| {
                defauts.push(defaut(
                    "boucle.regulation.irradiance.isr_min_w_m2",
                    ISR_MIN_CONV_W_M2,
                    "Th-BCE 2020 §9.15.2, tableau 292 : seuil d'ensoleillement \
                         conventionnel",
                ));
                ISR_MIN_CONV_W_M2
            }),
        },
    };
    BoucleSolaire {
        a_m2: a,
        eta0,
        a1_w_m2k: a1,
        a2_w_m2k2: a2,
        k_theta: capteurs.k_theta,
        ua_te_w_k: boucle.ua_te_w_k,
        ua_ti_w_k: boucle.ua_ti_w_k,
        ua_hx_w_k: ua_hx,
        p_np_w: boucle.p_np_w,
        alpha_p,
        m_cp_w_k: m_cp,
        regulation,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::entree::KitCesi;

    #[test]
    fn parametres_d_un_kit() {
        // Éq. (2266) et (2267) : A = 4 m², AC* = 2,6 m², UC* = 5 W/(m².K) :
        // η0 = 2,6/3,2 = 0,8125 ; a1 = 5 × 0,8125 = 4,0625 W/(m².K).
        let k = KitCesi {
            surface_m2: 4.0,
            v_tot_l: 300.0,
            p_np_w: 35.0,
            ac_etoile_m2: 2.6,
            uc_etoile_w_m2k: 5.0,
            us_w_k: 1.8,
            f_aux: Some(0.4),
            theta_max_c: 90.0,
            k_theta: 0.92,
        };
        let (b, v, us) = parametres_kit(&k);
        assert!((b.eta0 - 0.8125).abs() < 1e-12);
        assert!((b.a1_w_m2k - 4.0625).abs() < 1e-12);
        assert_eq!((b.a2_w_m2k2, b.ua_te_w_k, b.ua_ti_w_k), (0.0, 0.0, 0.0));
        assert_eq!((v, us), (300.0, 1.8));
        assert_eq!(b.ua_hx_w_k, Some(400.0));
        assert_eq!(b.m_cp_w_k, 232.0);
        assert_eq!(
            b.regulation,
            RegulationBoucle::Temperature {
                delta_theta_reg_k: 2.0
            }
        );
        // Kθ saisi, repris tel quel ; éq. (2267) : a1·A = UC*·AC*/0,8, pertes du capteur
        // supérieures de 25 % à celles de l'essai (lecture Q52).
        assert_eq!(b.k_theta, 0.92);
        assert!((b.a1_w_m2k * b.a_m2 - 5.0 * 2.6 / 0.8).abs() < 1e-12);
    }
}
