// SPDX-License-Identifier: GPL-3.0-or-later
//! Assemblage des espaces tampons : fiche `S1-Bat-Assemblage des espaces tampons`
//! (Th-BCE 2020 §5.2, figure 15, A3 p. 104 à 107).
//!
//! À chaque pas de temps, avant les zones (figure 15 : entrées du pas `h−1` des groupes, sorties
//! `btherm(h)`, `bsolaire(h)`, `becl(h)` et `θet(h)` vers les composants d'enveloppe) :
//! environnement proche des parois de l'espace tampon (§3.2) ; gestion et régulation de
//! l'espace tampon solarisé (§5.3) ; espace tampon (§5.4). L'espace non solarisé n'emploie pas
//! la gestion (§5.2.2).

use crate::climat::calendrier::hleg;
use crate::climat::environnement_proche::{flux_atteignant_la_paroi, MasqueResolu, SurfaceExposee};
use crate::entree::{Catalogue, EspaceTampon, NatureEspaceTampon};
use crate::resultats::ValeurParDefaut;

use super::super::paroi_opaque::resoudre_masques;
use super::super::ConditionsExterieures;
use super::espace_tampon::{
    a_ouv_et_m2, b_ecl, b_solaire, b_therm_solarise, bilan_solarise, c_m_et_wh_k, fs_et_w,
    h_et_th_ext_w_k, non_solarise, qv_surv_ouv_m3_h, theta_i_eq_c, CoefficientsEt, ParoiEt,
    CM_ET_SURF_CONV_KJ_M2K, QV_ET_BASE_DEFAUT_M3_H_M2, THETA_ET_INITIALE_C,
    THETA_I_FIN_INITIALE_ET_C,
};
use super::gestion_solarise::{
    aut_ouv_et, i_hebergement_et, i_per_sommeil, r_ouv_et_eff, r_ouv_et_req_selon, r_prot_et,
    saison_et, theta_et_regul_c, ConditionsProtectionsEt, LectureFermetureEt,
};

/// Espace tampon solarisé prêt pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct SolariseResolu {
    /// Parois donnant sur l'extérieur.
    pub parois: Vec<ParoiEt>,
    /// Orientation et dimensions de chaque paroi.
    pub surfaces: Vec<SurfaceExposee>,
    /// Masques de chaque paroi.
    pub masques: Vec<Vec<MasqueResolu>>,
    /// « Het_th_ext », en W/K (éq. 115).
    pub h_et_th_ext_w_k: f64,
    /// « Cm_et », en Wh/K (éq. 116).
    pub c_m_et_wh_k: f64,
    /// « Qv,et,base », en m³/h.
    pub qv_et_base_m3_h: f64,
    /// « Httfet », en m.
    pub h_ttf_et_m: f64,
    /// « idprot ».
    pub id_prot: bool,
    /// « Protet_ext ».
    pub prot_ext: bool,
}

/// Nature d'un espace tampon prêt pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub enum ModeleEt {
    /// « bet_ns ».
    NonSolarise(f64),
    /// Modèle dynamique de l'espace solarisé.
    Solarise(Box<SolariseResolu>),
}

/// Groupe en contact avec un espace tampon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LienGroupe {
    /// Indice de la zone dans le bâtiment.
    pub zone: usize,
    /// Indice du groupe dans la zone.
    pub groupe: usize,
    /// « Het_th_int^gr », en W/K (éq. 106).
    pub h_et_th_int_w_k: f64,
}

/// État d'un groupe vu par l'espace tampon à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupeVuParEt {
    /// « θi,fin^gr(h−1) », en °C.
    pub theta_i_fin_c: f64,
    /// « θop,fin^gr(h−1) », en °C.
    pub theta_op_fin_c: f64,
    /// « iocc^gr(h) ».
    pub i_occ: f64,
    /// « Autch,pro^gr(j) ».
    pub aut_ch: bool,
    /// « Autfr,pro^gr(j) ».
    pub aut_fr: bool,
    /// « ihebergement^gr ».
    pub hebergement: bool,
}

/// État d'un espace tampon d'un pas de temps au suivant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EtatEt {
    /// « θet(h−1) », en °C.
    pub theta_et_c: f64,
    /// « Rprotet(h−1) ».
    pub r_prot: f64,
    /// « Rouvet_req(h−1) ».
    pub r_ouv_req: f64,
    /// « θet_regul(h−1) », en °C.
    pub theta_regul_c: f64,
    /// Vrai avant le premier pas de temps.
    pub initial: bool,
}

impl Default for EtatEt {
    fn default() -> Self {
        Self {
            theta_et_c: THETA_ET_INITIALE_C,
            r_prot: 0.0,
            r_ouv_req: 0.0,
            theta_regul_c: THETA_ET_INITIALE_C,
            initial: true,
        }
    }
}

/// Cumuls annuels d'un espace tampon.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CumulsEt {
    /// Nombre de pas de temps.
    pub nbh: u32,
    /// Somme des `btherm(h)`.
    pub somme_b_therm: f64,
    /// Somme des `θet(h)`, en °C.
    pub somme_theta_et_c: f64,
    /// Plus haute `θet(h)`, en °C.
    pub theta_et_max_c: f64,
    /// Plus basse `θet(h)`, en °C.
    pub theta_et_min_c: f64,
    /// Nombre de pas de temps où les parois de l'espace tampon sont ouvertes.
    pub nbh_surventilation: u32,
    /// Bilan d'énergie d'un espace solarisé, en Wh : apports solaires `Fs_et`.
    pub apports_solaires_wh: f64,
    /// Chaleur reçue du volume chauffé, `Het_th_int_eq·(θi_eq − θet)`, en Wh.
    pub recu_du_volume_chauffe_wh: f64,
    /// Chaleur perdue vers l'extérieur par les parois et l'air, en Wh.
    pub pertes_exterieur_wh: f64,
    /// Variation de l'énergie stockée, `Cm_et·Δθet`, en Wh.
    pub stockage_wh: f64,
}

impl CumulsEt {
    /// Défaut de fermeture du bilan d'énergie d'un espace solarisé, en Wh.
    pub fn residu_wh(&self) -> f64 {
        self.apports_solaires_wh + self.recu_du_volume_chauffe_wh
            - self.pertes_exterieur_wh
            - self.stockage_wh
    }
}

/// Espace tampon prêt pour un calcul annuel au pas horaire.
#[derive(Debug, Clone, PartialEq)]
pub struct EspaceTamponHoraire {
    /// Identifiant de l'espace tampon.
    pub id: String,
    /// Modèle.
    pub modele: ModeleEt,
    /// Groupes en contact.
    pub liens: Vec<LienGroupe>,
    /// « Het_th_int_eq », en W/K (éq. 107).
    pub h_et_th_int_eq_w_k: f64,
    /// Lecture de la branche de fermeture de l'éq. (104).
    pub lecture_fermeture: LectureFermetureEt,
    /// Valeurs par défaut du texte appliquées.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
    /// État.
    pub etat: EtatEt,
    /// Cumuls annuels.
    pub cumuls: CumulsEt,
    /// Coefficients du dernier pas de temps calculé.
    pub coefficients: CoefficientsEt,
    /// « Φrejet(h) » : somme des puissances des évaporateurs des générateurs thermodynamiques
    /// d'ECS qui puisent dans l'espace tampon, au pas précédent, en W (éq. 109) ; fixée par le
    /// bâtiment avant chaque pas de temps, nulle sans générateur.
    pub phi_rejet_w: f64,
}

impl EspaceTamponHoraire {
    /// Prépare un espace tampon validé. `liens` donne les groupes en contact et leurs
    /// coefficients `Het_th_int^gr`, calculés en début de simulation (§5.4.3.1).
    pub fn nouveau(
        et: &EspaceTampon,
        catalogue: &Catalogue,
        liens: Vec<LienGroupe>,
        lecture_fermeture: LectureFermetureEt,
    ) -> Self {
        let mut valeurs_par_defaut = Vec::new();
        let modele = match &et.nature {
            NatureEspaceTampon::NonSolarise(n) => ModeleEt::NonSolarise(n.b_et_ns),
            NatureEspaceTampon::Solarise(s) => {
                let parois: Vec<ParoiEt> = s
                    .parois
                    .iter()
                    .map(|p| ParoiEt {
                        a_ue_m2: p.a_ue_m2,
                        u_ue_w_m2k: p.u_ue_w_m2k,
                        swe_sp: p.swe_sp,
                        swe_ap: p.swe_ap.unwrap_or(p.swe_sp),
                        tle_sp: p.tle_sp,
                        tle_ap: p.tle_ap.unwrap_or(p.tle_sp),
                        r_ouv_max: p.r_ouv_max,
                    })
                    .collect();
                let cm_et_surf = s.cm_et_surf_kj_m2k.unwrap_or_else(|| {
                    valeurs_par_defaut.push(ValeurParDefaut {
                        champ: "nature.solarise.cm_et_surf_kj_m2k".to_owned(),
                        valeur: CM_ET_SURF_CONV_KJ_M2K,
                        regle: "Th-BCE 2020 §5.4.2, tableau 19 : capacité thermique \
                                quotidienne conventionnelle de l'espace tampon"
                            .to_owned(),
                    });
                    CM_ET_SURF_CONV_KJ_M2K
                });
                let qv_base = s.qv_et_base_m3_h.unwrap_or_else(|| {
                    let v = QV_ET_BASE_DEFAUT_M3_H_M2 * s.a_et_m2;
                    valeurs_par_defaut.push(ValeurParDefaut {
                        champ: "nature.solarise.qv_et_base_m3_h".to_owned(),
                        valeur: v,
                        regle: "Th-BCE 2020 §5.4.3.3.1.4 : par défaut, 1,0 m³/h par m² de \
                                surface au sol de l'espace tampon"
                            .to_owned(),
                    });
                    v
                });
                ModeleEt::Solarise(Box::new(SolariseResolu {
                    h_et_th_ext_w_k: h_et_th_ext_w_k(&parois, s.u_et_pb_w_m2k, s.a_et_m2),
                    c_m_et_wh_k: c_m_et_wh_k(cm_et_surf, s.a_et_m2),
                    qv_et_base_m3_h: qv_base,
                    h_ttf_et_m: s.h_ttf_et_m,
                    id_prot: s.protections_mobiles.is_some(),
                    prot_ext: s
                        .protections_mobiles
                        .as_ref()
                        .is_some_and(|p| p.exterieures),
                    surfaces: s
                        .parois
                        .iter()
                        .map(|p| SurfaceExposee::depuis_exposition(&p.exposition))
                        .collect(),
                    masques: s
                        .parois
                        .iter()
                        .map(|p| resoudre_masques(&p.exposition.masques, catalogue))
                        .collect(),
                    parois,
                }))
            }
        };
        let h_et_th_int_eq_w_k = liens.iter().map(|l| l.h_et_th_int_w_k).sum();
        Self {
            id: et.id.clone(),
            modele,
            liens,
            h_et_th_int_eq_w_k,
            lecture_fermeture,
            valeurs_par_defaut,
            etat: EtatEt::default(),
            cumuls: CumulsEt {
                theta_et_max_c: f64::NEG_INFINITY,
                theta_et_min_c: f64::INFINITY,
                ..CumulsEt::default()
            },
            coefficients: CoefficientsEt {
                b_therm: 1.0,
                b_solaire: 1.0,
                b_ecl: 1.0,
                theta_et_c: THETA_ET_INITIALE_C,
                a_ouv_et_m2: 0.0,
            },
            phi_rejet_w: 0.0,
        }
    }

    /// Calcule le pas de temps : coefficients `btherm(h)`, `bsolaire(h)`, `becl(h)` et `θet(h)`.
    ///
    /// `groupes` décrit les groupes de [`Self::liens`], dans le même ordre ; `v_ventc_m_s` est
    /// la vitesse du vent. Les générateurs thermodynamiques d'ECS (éq. 109) relèvent du lot 6 :
    /// `Φrejet = 0`.
    pub fn pas(
        &mut self,
        c: &ConditionsExterieures,
        v_ventc_m_s: f64,
        groupes: &[GroupeVuParEt],
    ) -> CoefficientsEt {
        let h = c.h;
        let theta_ext = c.te_c;
        // Éq. (108) : θi,fin(h−1) des groupes, 19 °C avant le premier pas de temps
        // (tableau 19).
        let initial = self.etat.initial;
        let theta_i_eq = theta_i_eq_c(self.liens.iter().zip(groupes).map(|(l, g)| {
            let theta = if initial {
                THETA_I_FIN_INITIALE_ET_C
            } else {
                g.theta_i_fin_c
            };
            (l.h_et_th_int_w_k, theta)
        }))
        .unwrap_or(THETA_I_FIN_INITIALE_ET_C);
        let h_int = self.h_et_th_int_eq_w_k;
        let coefficients = match &self.modele {
            ModeleEt::NonSolarise(b_et_ns) => {
                // §5.4.3.2, éq. (109) à (114).
                non_solarise(*b_et_ns, theta_ext, theta_i_eq, self.phi_rejet_w, h_int)
            }
            ModeleEt::Solarise(s) => {
                // Environnement proche des parois de l'espace tampon (§3.2), éq. (117).
                let mut isr = Vec::with_capacity(s.parois.len());
                let mut somme_etp = 0.0;
                for (surface, masques) in s.surfaces.iter().zip(&s.masques) {
                    let f = flux_atteignant_la_paroi(
                        surface, masques, &c.soleil, c.idn_w_m2, c.idi_w_m2, c.edn_lux, c.edi_lux,
                        h,
                    );
                    isr.push(f.isr_w_m2());
                    somme_etp += f.etp_lux();
                }
                // §5.3 : gestion et régulation.
                let saison = saison_et(groupes.iter().map(|g| (g.aut_ch, g.aut_fr)));
                let hebergement = i_hebergement_et(groupes.iter().map(|g| g.hebergement));
                let theta_regul = theta_et_regul_c(
                    self.etat.theta_et_c,
                    groupes.iter().map(|g| (g.i_occ, g.theta_op_fin_c)),
                );
                let theta_regul_h1 = if initial {
                    theta_regul
                } else {
                    self.etat.theta_regul_c
                };
                let r_prot = r_prot_et(
                    &ConditionsProtectionsEt {
                        id_prot: s.id_prot,
                        prot_ext: s.prot_ext,
                        somme_etp_lux: somme_etp,
                        v_ventc_m_s,
                        theta_regul_c: theta_regul,
                        saison,
                    },
                    self.etat.r_prot,
                );
                let aut = aut_ouv_et(theta_ext, theta_regul, saison);
                let r_req = r_ouv_et_req_selon(
                    self.lecture_fermeture,
                    theta_regul,
                    theta_regul_h1,
                    self.etat.r_ouv_req,
                    saison,
                );
                let sommeil = i_per_sommeil(hebergement, hleg(h));
                let r_eff = r_ouv_et_eff(sommeil, aut, r_req, self.etat.r_ouv_req);
                // §5.4.3.3 : bilan de l'espace tampon.
                let fs = fs_et_w(&s.parois, &isr, r_prot);
                let a_ouv = a_ouv_et_m2(&s.parois, r_eff);
                let qv_surv = qv_surv_ouv_m3_h(
                    a_ouv,
                    v_ventc_m_s,
                    s.h_ttf_et_m,
                    theta_ext,
                    self.etat.theta_et_c,
                );
                let qv_ent = s.qv_et_base_m3_h + qv_surv;
                let b = bilan_solarise(
                    fs,
                    s.h_et_th_ext_w_k,
                    qv_ent,
                    theta_ext,
                    h_int,
                    theta_i_eq,
                    s.c_m_et_wh_k,
                    self.etat.theta_et_c,
                );
                // Bilan d'énergie, pour vérification.
                let k = &mut self.cumuls;
                k.apports_solaires_wh += fs;
                k.recu_du_volume_chauffe_wh += h_int * (theta_i_eq - b.theta_et_moy_c);
                k.pertes_exterieur_wh += (s.h_et_th_ext_w_k
                    + super::espace_tampon::CV_AIR_WH_M3K * qv_ent)
                    * (b.theta_et_moy_c - theta_ext);
                k.stockage_wh += s.c_m_et_wh_k * (b.theta_et_c - self.etat.theta_et_c);
                self.etat.r_prot = r_prot;
                self.etat.r_ouv_req = r_req;
                self.etat.theta_regul_c = theta_regul;
                CoefficientsEt {
                    b_therm: b_therm_solarise(theta_i_eq, b.theta_et_c, theta_ext),
                    b_solaire: b_solaire(&s.parois, r_prot),
                    b_ecl: b_ecl(&s.parois, r_prot),
                    theta_et_c: b.theta_et_c,
                    a_ouv_et_m2: a_ouv,
                }
            }
        };
        self.etat.theta_et_c = coefficients.theta_et_c;
        self.etat.initial = false;
        let k = &mut self.cumuls;
        k.nbh += 1;
        k.somme_b_therm += coefficients.b_therm;
        k.somme_theta_et_c += coefficients.theta_et_c;
        k.theta_et_max_c = k.theta_et_max_c.max(coefficients.theta_et_c);
        k.theta_et_min_c = k.theta_et_min_c.min(coefficients.theta_et_c);
        k.nbh_surventilation += u32::from(coefficients.surventile());
        self.coefficients = coefficients;
        coefficients
    }
}
