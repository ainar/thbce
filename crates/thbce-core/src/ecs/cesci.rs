// SPDX-License-Identifier: GPL-3.0-or-later
//! Productions solaires collectives à appoints décentralisés du logement collectif :
//! fiches `S1_GEN_CESCI` (Th-BCE 2020 §9.16) et `S2_GEN_CESCAI` (§9.17).
//!
//! - **CESCI** (« Type_PCAD » = 0 ou 2) : un champ de capteurs et une boucle solaire collectifs
//!   chauffent, par leurs échangeurs en zone basse, des ballons individuels groupés en
//!   ensembles identiques ; chaque ballon a son appoint intégré (0) ou un appoint séparé
//!   instantané (2). La boucle est régulée sur l'ensoleillement (éq. 1981), voit la
//!   température moyenne des zones basses de tous les ballons (éq. 2007) et répartit son
//!   énergie au prorata des volumes de ces zones (éq. 2008 et 2009).
//! - **CESCAI** (« Type_PCAD » = 1) : la boucle chauffe un ballon central ; le réseau d'ECS
//!   intergroupe bouclé en distribue l'énergie, diminuée de ses pertes, aux ballons
//!   individuels à appoint intégré (éq. 2075 à 2080).
//!
//! La production est un assemblage de la génération (§8.17.3.6), premier sollicité pour l'ECS :
//! les appoints sont des générateurs de la génération (`rdim` = nombre de ballons), appelés par
//! les ballons ; un appoint mixte assure en outre le chauffage, en cascade, comme tout
//! générateur de la génération (éq. 1991 et 2012 à 2015).
//!
//! Lectures : `docs/questions/Q053.md`.

use super::assemblage_ballon::{AssemblageBallon, BaseDuPas, EntreesAssemblage};
use super::ballon_de_stockage::{Ballon, NB_ZONES, RHO_CW_WH_L_K};
use super::boucle_solaire::{BoucleSolaireHoraire, EntreesBoucle};
use crate::chauffage_refroidissement::calculs_generation::Poste;

/// Type de production (« Type_PCAD »).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeProduction {
    /// CESCI à appoint intégré (0).
    Cesci,
    /// CESCAI (1).
    Cescai,
    /// CESCI à appoint séparé instantané (2).
    CesciAppointSepare,
}

/// Température de l'eau entrant dans les ballons individuels d'un CESCAI (Q53).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LectureEntreeCescai {
    /// Retenue : eau froide. L'énergie du ballon central n'entre dans les ballons individuels
    /// que par l'injection en zone basse (éq. 2078 et 2079), ce qui conserve l'énergie.
    #[default]
    EauFroide,
    /// Lecture littérale de l'éq. (2073) : `θentrant = θmoy_prim-e`. L'eau puisée est alors
    /// remplacée par de l'eau à la température du réseau intergroupe que personne ne chauffe,
    /// en plus de l'énergie injectée par l'éq. (2079). Conservée, non exposée.
    MoyennePrimaire,
}

/// Ballon central d'un CESCAI et ses cumuls.
#[derive(Debug, Clone, PartialEq)]
pub struct BallonCentral {
    /// Ballon (« ballon base seule », éq. 2065).
    pub ballon: Ballon,
    /// Cumuls annuels.
    pub cumuls: CumulsCentral,
}

/// Cumuls annuels du ballon central d'un CESCAI, en Wh.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CumulsCentral {
    /// Énergie fournie aux ballons individuels par le réseau intergroupe (Σ « Qfou_ens^B »).
    pub q_fou_reseau_wh: f64,
    /// Pertes du réseau intergroupe déduites du ballon central (éq. 2080).
    pub pertes_reseau_deduites_wh: f64,
    /// Pertes du ballon central.
    pub phi_pertes_wh: f64,
    /// Températures des zones au début du calcul, en °C.
    pub theta_debut_c: Option<[f64; NB_ZONES]>,
    /// Plus basse température de la zone haute en fin de pas de temps, en °C.
    pub theta_haut_min_c: Option<f64>,
}

/// Production solaire collective à appoints décentralisés, prête pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductionCollective {
    /// Identifiant.
    pub id: String,
    /// « Type_PCAD ».
    pub type_production: TypeProduction,
    /// Boucle solaire collective et ses capteurs.
    pub boucle: BoucleSolaireHoraire,
    /// Ensembles de ballons individuels : un assemblage par ensemble, `nb` ballons identiques,
    /// sans générateur de base (l'énergie de la boucle ou du réseau y est injectée).
    pub ensembles: Vec<AssemblageBallon>,
    /// Ballon central d'un CESCAI.
    pub central: Option<BallonCentral>,
    /// Lecture de l'éq. (2073) (CESCAI).
    pub lecture_entree: LectureEntreeCescai,
}

/// Entrées d'un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntreesCollectif {
    /// Demande d'ECS transmise à la production, en Wh (besoins des distributions du groupe,
    /// pertes du réseau intergroupe d'un CESCAI et reports compris).
    pub q_req_ecs_wh: f64,
    /// « φpertes_vc_prim-e + φpertes_hvc_prim-e » : pertes du réseau d'ECS intergroupe, en Wh
    /// (CESCAI ; comprises dans `q_req_ecs_wh`).
    pub phi_pertes_prim_wh: f64,
    /// « θmoy_prim-e » : température moyenne du réseau d'ECS intergroupe, en °C (CESCAI).
    pub theta_moy_prim_c: f64,
    /// « θcw(h) » : température de l'eau froide, en °C.
    pub theta_cw_c: f64,
    /// « θecs_max^gen » : température de départ attendue par les distributions, en °C
    /// (éq. 1976 et 2071).
    pub theta_depart_c: f64,
    /// « θamb(h) » de la génération, en °C (éq. 1978 et 2074).
    pub theta_amb_c: f64,
    /// Ambiance des ballons individuels d'un CESCAI, toujours en volume chauffé, en °C
    /// (éq. 2074).
    pub theta_amb_dec_c: f64,
    /// « hleg ».
    pub hleg: u8,
    /// « θe(h) », en °C.
    pub theta_ext_c: f64,
    /// « Isr* » sur le plan des capteurs, en W/m².
    pub isr_w_m2: f64,
    /// Mois des scénarios, de 0 à 11.
    pub mois: usize,
}

/// Sorties d'un pas de temps, en Wh.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SortiesCollectif {
    /// Énergie fournie à la distribution d'ECS.
    pub q_fou_ecs_wh: f64,
    /// Reste : appoint séparé instantané (éq. 2010) ou report des ballons.
    pub q_rest_ecs_wh: f64,
    /// Pertes des ballons placés à l'emplacement de la génération (ballons individuels d'un
    /// CESCI, ballon central d'un CESCAI).
    pub phi_pertes_wh: f64,
    /// Pertes des ballons individuels d'un CESCAI, toujours en volume chauffé (éq. 2114).
    pub phi_pertes_dec_vc_wh: f64,
    /// « Pp » : consommation de la pompe de la boucle (éq. 1972).
    pub w_pompe_wh: f64,
    /// « ΦvcBS » : pertes de la boucle vers l'intérieur (éq. 1973).
    pub phi_vc_bs_wh: f64,
}

impl ProductionCollective {
    /// Nombre total de ballons individuels.
    pub fn nb_ballons(&self) -> f64 {
        self.ensembles.iter().map(|a| a.nb).sum()
    }

    /// Un pas de temps. `appel(generateur, poste, Qreq, θaval)` appelle un appoint de la
    /// génération pour l'ensemble des ballons identiques (comme pour un assemblage avec
    /// ballon).
    pub fn pas(
        &mut self,
        e: &EntreesCollectif,
        appel: &mut dyn FnMut(usize, Poste, f64, f64) -> f64,
    ) -> SortiesCollectif {
        let nb_tot = self.nb_ballons();
        // Éq. (1985) et (2070) : demande d'un ensemble, les besoins des distributions du
        // groupe qui lui sont reliées. Le moteur ne relie pas un ensemble à des groupes : la
        // demande est répartie au prorata du nombre de ballons (lecture Q53). Pour un CESCAI,
        // les pertes du réseau intergroupe ne sont pas puisées dans les ballons individuels :
        // elles sont déduites du ballon central (éq. 2076).
        let cescai = self.type_production == TypeProduction::Cescai;
        let q_ballons = if cescai {
            (e.q_req_ecs_wh - e.phi_pertes_prim_wh).max(0.0)
        } else {
            e.q_req_ecs_wh
        };
        let theta_entrant = match (cescai, self.lecture_entree) {
            (true, LectureEntreeCescai::MoyennePrimaire) => e.theta_moy_prim_c,
            // Éq. (1986).
            _ => e.theta_cw_c,
        };
        let entrees = |nb: f64| EntreesAssemblage {
            q_req_ecs_wh: q_ballons * nb / nb_tot,
            q_ecs_horaire_wh: q_ballons * nb / nb_tot,
            theta_entrant_c: theta_entrant,
            theta_depart_c: e.theta_depart_c,
            theta_amb_c: if cescai {
                e.theta_amb_dec_c
            } else {
                e.theta_amb_c
            },
            hleg: e.hleg,
            theta_ext_c: e.theta_ext_c,
            isr_w_m2: 0.0,
            mois: e.mois,
            q_req_ch_wh: 0.0,
            theta_ch_max_c: f64::NAN,
            theta_aval_ch_c: 0.0,
        };
        let mut s = SortiesCollectif::default();
        if let Some(c) = &mut self.central {
            // CESCAI.
            let b = &mut c.ballon;
            if c.cumuls.theta_debut_c.is_none() {
                c.cumuls.theta_debut_c = Some(b.theta_c);
            }
            // Éq. (2075) : énergie contenue au début du pas de temps (températures de la fin du
            // pas précédent ; la zone haute fixe aussi le départ du réseau intergroupe).
            let q_contenue = b.energie_wh(e.theta_cw_c);
            // Pertes du ballon central, avec les températures de la fin du pas précédent ;
            // appliquées à la douzième étape.
            b.debut_pas(e.theta_amb_c);
            // Éq. (2076) et (2077).
            let q_dispo = (q_contenue - e.phi_pertes_prim_wh).max(0.0);
            let v_base_tot: f64 = self.ensembles.iter().map(|a| a.ballon.v_l[0] * a.nb).sum();
            let mut q_fou_reseau = 0.0;
            for a in &mut self.ensembles {
                let ea = entrees(a.nb);
                let p = a.puiser_pas(&ea);
                // Éq. (2078) et (2079) : énergie reçue du réseau, limitée par l'écart entre la
                // température moyenne du réseau et celle des zones après puisage.
                let capacite = a.nb
                    * RHO_CW_WH_L_K
                    * (0..NB_ZONES)
                        .map(|z| a.ballon.v_l[z] * (e.theta_moy_prim_c - a.ballon.theta_c[z]))
                        .sum::<f64>()
                        .max(0.0);
                let q_dispo_ens = q_dispo * a.ballon.v_l[0] * a.nb / v_base_tot;
                let q_fou_ens = q_dispo_ens.min(capacite);
                q_fou_reseau += q_fou_ens;
                let sa =
                    a.terminer_pas(&ea, &p, BaseDuPas::Injection(q_fou_ens / a.nb), None, appel);
                s.q_fou_ecs_wh += sa.q_fou_ecs_wh;
                s.q_rest_ecs_wh += sa.q_rest_ecs_wh;
                s.phi_pertes_dec_vc_wh += sa.phi_pertes_wh;
            }
            // Éq. (2080) : énergie fournie au réseau, ballons individuels et pertes du réseau,
            // soutirée de la zone haute, sans les pertes du ballon. Les pertes du réseau,
            // calculées au départ de la zone haute du ballon central et bornées à zéro, sont
            // toutes payées par le ballon central (lecture Q53).
            let pertes_deduites = e.phi_pertes_prim_wh;
            b.injecter(NB_ZONES - 1, -(q_fou_reseau + pertes_deduites), false);
            // Onzième étape : la boucle voit la température de la zone basse du ballon central
            // (« θBSaval(h) = θcentr_b1(h) »).
            let sb = self.boucle.boucle.pas(&EntreesBoucle {
                isr_w_m2: e.isr_w_m2,
                theta_e_c: e.theta_ext_c,
                theta_amb_c: e.theta_amb_c,
                theta_b_moy_ech_c: b.theta_c[0],
            });
            self.boucle.cumuls.ajouter(&sb, e.isr_w_m2, e.mois);
            // Douzième étape : énergie solaire en zone basse, avec les pertes du ballon.
            b.injecter(0, sb.q_sol_wh, true);
            let cc = &mut c.cumuls;
            cc.q_fou_reseau_wh += q_fou_reseau;
            cc.pertes_reseau_deduites_wh += pertes_deduites;
            cc.phi_pertes_wh += b.pertes_totales_wh();
            let haut = b.theta_c[NB_ZONES - 1];
            cc.theta_haut_min_c = Some(cc.theta_haut_min_c.map_or(haut, |t| t.min(haut)));
            s.phi_pertes_wh += b.pertes_totales_wh();
            s.w_pompe_wh = sb.p_p_wh;
            s.phi_vc_bs_wh = sb.phi_vc_wh;
            // Les pertes du réseau, comprises dans la demande, sont fournies par le ballon
            // central (éq. 2080).
            s.q_fou_ecs_wh += e.q_req_ecs_wh - q_ballons;
        } else {
            // CESCI : première étape, puisage dans tous les ballons.
            let puisages: Vec<_> = self
                .ensembles
                .iter_mut()
                .map(|a| {
                    let ea = entrees(a.nb);
                    (ea, a.puiser_pas(&ea))
                })
                .collect();
            // Éq. (2007) : température moyenne vue par les échangeurs solaires, pondérée par
            // les volumes des zones de base de tous les ballons.
            let v_base_tot: f64 = self.ensembles.iter().map(|a| a.ballon.v_l[0] * a.nb).sum();
            let theta_moy = self
                .ensembles
                .iter()
                .zip(&puisages)
                .map(|(a, (_, p))| a.ballon.v_l[0] * a.nb * p.theta_aval_base_c)
                .sum::<f64>()
                / v_base_tot;
            // Deuxième étape : la boucle solaire collective.
            let sb = self.boucle.boucle.pas(&EntreesBoucle {
                isr_w_m2: e.isr_w_m2,
                theta_e_c: e.theta_ext_c,
                theta_amb_c: e.theta_amb_c,
                theta_b_moy_ech_c: theta_moy,
            });
            self.boucle.cumuls.ajouter(&sb, e.isr_w_m2, e.mois);
            for (a, (ea, p)) in self.ensembles.iter_mut().zip(&puisages) {
                // Éq. (2008) et (2009).
                let q_sol_unit = sb.q_sol_wh * a.ballon.v_l[0] / v_base_tot;
                let sa = a.terminer_pas(ea, p, BaseDuPas::Injection(q_sol_unit), None, appel);
                s.q_fou_ecs_wh += sa.q_fou_ecs_wh;
                s.q_rest_ecs_wh += sa.q_rest_ecs_wh;
                s.phi_pertes_wh += sa.phi_pertes_wh;
            }
            s.w_pompe_wh = sb.p_p_wh;
            s.phi_vc_bs_wh = sb.phi_vc_wh;
        }
        s
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::climat::environnement_proche::SurfaceExposee;
    use crate::ecs::assemblage_ballon::RegulationGenerateur;
    use crate::ecs::boucle_solaire::{
        BoucleSolaire, CapteursExposes, CumulsBoucle, RegulationBoucle,
    };
    use crate::ecs::gestion_regulation_ballon::{nb_iter_vp, Programmation};
    use crate::entree::Exposition;

    fn boucle() -> BoucleSolaireHoraire {
        BoucleSolaireHoraire {
            boucle: BoucleSolaire {
                a_m2: 10.0,
                eta0: 0.6,
                a1_w_m2k: 6.0,
                a2_w_m2k2: 0.0,
                k_theta: 0.9,
                ua_te_w_k: 2.0,
                ua_ti_w_k: 3.0,
                ua_hx_w_k: Some(1000.0),
                p_np_w: 80.0,
                alpha_p: 0.5,
                m_cp_w_k: 580.0,
                regulation: RegulationBoucle::Irradiance {
                    isr_min_w_m2: 200.0,
                },
            },
            capteurs: CapteursExposes {
                surface: SurfaceExposee::depuis_exposition(&Exposition {
                    azimut_deg: 0.0,
                    inclinaison_deg: 45.0,
                    lpb_m: None,
                    hpb_m: None,
                    masques: Vec::new(),
                }),
                masques: Vec::new(),
            },
            cumuls: CumulsBoucle::default(),
        }
    }

    fn ensemble(nb: f64, v: f64, appoint: Option<usize>) -> AssemblageBallon {
        let f_aux = appoint.map(|_| 0.4);
        let reg = |g: usize, z: usize| RegulationGenerateur {
            generateur: g,
            programmation: Programmation::Permanent,
            delta_theta_k: 2.0,
            z_gen: z,
            z_reg: z,
            h_rel_ech: 0.0,
        };
        let mut a = AssemblageBallon::nouveau(
            "b",
            nb,
            Ballon::nouveau(v, 1.0, f_aux, 90.0),
            reg(usize::MAX, 0),
            appoint.map(|g| reg(g, 2)),
            nb_iter_vp(f_aux),
        );
        a.appoint_separe = appoint.is_none();
        a
    }

    fn entrees(q: f64, isr: f64) -> EntreesCollectif {
        EntreesCollectif {
            q_req_ecs_wh: q,
            phi_pertes_prim_wh: 0.0,
            theta_moy_prim_c: 50.5,
            theta_cw_c: 12.0,
            theta_depart_c: 53.0,
            theta_amb_c: 20.0,
            theta_amb_dec_c: 20.0,
            hleg: 12,
            theta_ext_c: 15.0,
            isr_w_m2: isr,
            mois: 5,
        }
    }

    #[test]
    fn repartition_de_l_energie_solaire_d_un_cesci() {
        // Deux ensembles : 4 ballons de 200 L et 2 ballons de 100 L, sans appoint intégré
        // (appoint séparé), sans puisage. Éq. (2007) : les zones basses (50 L et 25 L) pondèrent
        // la température vue par la boucle ; éq. (2008) et (2009) : chaque ballon reçoit
        // Qsol·Vz/Σ(Vz·nb), soit 200/250 et 50/250 de l'énergie par ensemble.
        let mut p = ProductionCollective {
            id: "c".into(),
            type_production: TypeProduction::CesciAppointSepare,
            boucle: boucle(),
            ensembles: vec![ensemble(4.0, 200.0, None), ensemble(2.0, 100.0, None)],
            central: None,
            lecture_entree: LectureEntreeCescai::default(),
        };
        p.ensembles[1].ballon.fixer_temperature(30.0);
        // Température vue par la boucle : (200 × 50 + 50 × 30)/250 = 46 °C.
        let attendu = p.boucle.boucle.pas(&EntreesBoucle {
            isr_w_m2: 800.0,
            theta_e_c: 15.0,
            theta_amb_c: 20.0,
            theta_b_moy_ech_c: 46.0,
        });
        assert!(attendu.en_marche && attendu.q_sol_wh > 0.0);
        let mut appel = |_: usize, _: Poste, _: f64, _: f64| 0.0;
        let s = p.pas(&entrees(0.0, 800.0), &mut appel);
        let q = |k: usize| p.ensembles[k].cumuls.q_fou_sto_wh[0];
        assert!((q(0) - 0.8 * attendu.q_sol_wh).abs() < 1e-9);
        assert!((q(1) - 0.2 * attendu.q_sol_wh).abs() < 1e-9);
        assert_eq!(s.w_pompe_wh, 80.0);
        assert_eq!(s.q_fou_ecs_wh, 0.0);
    }

    #[test]
    fn demande_repartie_au_prorata_des_ballons() {
        // 3 + 1 ballons de 150 L chauds, 4 kWh demandés : 3 kWh et 1 kWh puisés, rien à
        // reporter ; nuit : la boucle est arrêtée.
        let mut p = ProductionCollective {
            id: "c".into(),
            type_production: TypeProduction::Cesci,
            boucle: boucle(),
            ensembles: vec![ensemble(3.0, 150.0, Some(0)), ensemble(1.0, 150.0, Some(1))],
            central: None,
            lecture_entree: LectureEntreeCescai::default(),
        };
        for a in &mut p.ensembles {
            a.ballon.fixer_temperature(55.0);
        }
        let mut appel = |_: usize, _: Poste, _: f64, _: f64| 0.0;
        let s = p.pas(&entrees(4000.0, 0.0), &mut appel);
        assert!((s.q_fou_ecs_wh - 4000.0).abs() < 1e-9);
        assert!((p.ensembles[0].cumuls.q_fou_ecs_wh - 3000.0).abs() < 1e-9);
        assert!((p.ensembles[1].cumuls.q_fou_ecs_wh - 1000.0).abs() < 1e-9);
        assert_eq!(s.w_pompe_wh, 0.0);
    }

    fn cescai(theta_central_c: f64, theta_dec_c: f64) -> ProductionCollective {
        let mut central = Ballon::nouveau(1000.0, 3.0, None, 90.0);
        central.fixer_temperature(theta_central_c);
        let mut p = ProductionCollective {
            id: "c".into(),
            type_production: TypeProduction::Cescai,
            boucle: boucle(),
            ensembles: vec![ensemble(2.0, 150.0, Some(0))],
            central: Some(BallonCentral {
                ballon: central,
                cumuls: CumulsCentral::default(),
            }),
            lecture_entree: LectureEntreeCescai::default(),
        };
        p.ensembles[0].ballon.fixer_temperature(theta_dec_c);
        p
    }

    /// Entrées d'un CESCAI la nuit : réseau bouclé au départ de la zone haute du ballon
    /// central (θmoy_prim-e = θcentr_b4 − 2,5 K) et pertes du réseau `pertes_wh`.
    fn entrees_cescai(theta_central_c: f64, pertes_wh: f64) -> EntreesCollectif {
        let mut e = entrees(pertes_wh, 0.0);
        e.phi_pertes_prim_wh = pertes_wh;
        e.theta_moy_prim_c = theta_central_c - 2.5;
        e
    }

    #[test]
    fn cescai_bilan_du_ballon_central() {
        // Ballon central de 1 000 L à 50 °C, eau froide à 12 °C : 44 194 Wh contenus
        // (éq. 2075). Réseau à 47,5 °C, pertes 500 Wh ; deux ballons individuels de 150 L à
        // 40 °C, sans puisage : chacun peut recevoir 1,163 × 150 × 7,5 = 1 308,4 Wh
        // (éq. 2078). Nuit : pas de solaire. Le ballon central cède 2 × 1 308,4 + 500 Wh
        // (éq. 2080) et perd 3 W/K × 30 K = 90 Wh.
        let mut p = cescai(50.0, 40.0);
        let avant = p.central.as_ref().unwrap().ballon.energie_wh(0.0);
        let mut appel = |_: usize, _: Poste, _: f64, _: f64| 0.0;
        let s = p.pas(&entrees_cescai(50.0, 500.0), &mut appel);
        let c = p.central.as_ref().unwrap();
        let recu = 2.0 * 1.163 * 150.0 * 7.5;
        assert!((c.cumuls.q_fou_reseau_wh - recu).abs() < 1e-6);
        assert!((c.cumuls.pertes_reseau_deduites_wh - 500.0).abs() < 1e-9);
        assert!((c.cumuls.phi_pertes_wh - 90.0).abs() < 1e-9);
        let apres = c.ballon.energie_wh(0.0);
        assert!((avant - apres - recu - 500.0 - 90.0).abs() < 1e-6);
        // Les pertes du réseau sont payées par le ballon central ; rien n'a été puisé.
        assert!((s.q_fou_ecs_wh - 500.0).abs() < 1e-9);
        assert!(s.phi_pertes_dec_vc_wh > 0.0 && s.phi_pertes_wh == 90.0);
        // Les ballons individuels ne dépassent pas la zone haute du ballon central d'avant le
        // transfert.
        assert!(p.ensembles[0].ballon.theta_c.iter().all(|t| *t <= 50.0));
    }

    #[test]
    fn cescai_ballon_central_plus_froid_que_les_ballons() {
        // Ballon central à 30 °C (réseau à 27,5 °C), ballons individuels à 40 °C : aucun
        // transfert (éq. 2078 : « l'eau des ballons doit être plus froide que l'eau de la
        // distribution intergroupe ») ; le ballon central paie les pertes du réseau (200 Wh)
        // et les siennes (3 × 10 = 30 Wh), au bilan exact.
        let mut p = cescai(30.0, 40.0);
        let avant = p.central.as_ref().unwrap().ballon.energie_wh(0.0);
        let dec_avant = p.ensembles[0].ballon.energie_wh(0.0);
        let mut appel = |_: usize, _: Poste, _: f64, _: f64| 0.0;
        p.pas(&entrees_cescai(30.0, 200.0), &mut appel);
        let c = p.central.as_ref().unwrap();
        assert_eq!(c.cumuls.q_fou_reseau_wh, 0.0);
        assert!((avant - c.ballon.energie_wh(0.0) - 200.0 - 30.0).abs() < 1e-6);
        assert_eq!(p.ensembles[0].cumuls.q_fou_sto_wh[0], 0.0);
        assert!(p.ensembles[0].ballon.energie_wh(0.0) < dec_avant);
    }
}
