// SPDX-License-Identifier: GPL-3.0-or-later
//! Système de ventilation du projet d'une zone, pour les calculs Th-C et Th-D :
//! assemblage des fiches `C_VEN_Bouche_conduit` (§6.2), `C_VEN_Mécanique_SF` (§6.5),
//! `C_VEN_Mécanique_Basse-Pression` (§6.4) et `C_VEN_CTA et Double_flux` (§6.3, double flux).
//!
//! « La modélisation du système de ventilation s'effectue au niveau d'une zone » (§6,
//! introduction). Avant la simulation, chaque bouche reçoit ses débits (dépassement et fuites)
//! et son conduit son coefficient de transfert hors volume chauffé ; les ventilateurs, leur
//! puissance moyenne. À chaque pas de temps, avant le bilan aéraulique de la zone (§5.6),
//! [`VentilationProjet::pas`] donne la température et l'humidité de l'air soufflé dans chaque
//! groupe, à partir des températures des groupes à la fin du pas précédent.
//!
//! En habitation, les débits et la puissance des ventilateurs sont constants. Dans les autres
//! usages (simple flux, §6.2.3.1.1.1 et §6.5.3.3), ils dépendent de l'indicateur de ventilation
//! de la zone, `Ivent` : débits et puissance d'occupation ou d'inoccupation.
//!
//! Le calcul du Bbio n'emploie pas ce système : il emploie le système conventionnel de la
//! fiche 6.1 (§6.1.3 ; décision Q36).

use crate::entree::{
    Isouf, RepresentationEchangeur, StatutEchangeur, SystemeVentilation, UsageZone, Zone,
};
use crate::resultats::ValeurParDefaut;

use super::bouche_conduit::{
    a_cond_defaut_collectif_m2, a_cond_defaut_maison_m2, debits_dep_habitation,
    debits_dep_non_residentiel, k_res, q_fuites_m3_h, Debits, DebitsBoucheProjet, PressionReseau,
    TypologieReseau, CDEP_DEFAUT, CRDBNR_DEFAUT, DUGD_DEFAUT_H, K_RES_DEFAUT,
};
use super::cta_et_double_flux::{
    antigel, delta_bypass, epsilon_detaille, epsilon_t, sorties_echangeur_c,
    THETA_S_ECH_REP_LIM_DEFAUT_C,
};
use super::mecanique_basse_pression;
use super::mecanique_sf::{
    dugd_equ_h, h_cond_ext_w_k, p_vent_groupe_w, p_vent_residentiel_w, qm_kg_s, rho_kg_m3,
    t_apres_conduit_c, t_apres_ventilateur_c, t_lnc_c, theta_i_cond_c, C_PA_J_KG_K,
};

/// Nature du système, résolue pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub enum NatureSysteme {
    /// Simple flux par extraction (§6.5).
    SimpleFluxExtraction,
    /// Simple flux par insufflation (§6.5).
    SimpleFluxInsufflation,
    /// Extraction mécanique basse pression (§6.4).
    BassePression,
    /// Double flux (§6.3) ; `None` : sans échangeur.
    DoubleFlux(Option<EchangeurResolu>),
}

/// Efficacité d'un échangeur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EfficaciteEchangeur {
    /// « εt » après prise en compte du statut (éq. 396 à 398).
    Simplifiee(f64),
    /// Coefficient d'échange global et écoulements (éq. 499 à 505).
    Detaillee {
        /// « UA », en W/K.
        ua_w_k: f64,
        /// « idtype_echangeur ».
        ecoulement: crate::entree::EcoulementEchangeur,
    },
}

/// Échangeur d'un double flux, prêt pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct EchangeurResolu {
    /// Efficacité.
    pub efficacite: EfficaciteEchangeur,
    /// Régulation du bypass ; `None` sans bypass.
    pub bypass: Option<crate::entree::Bypass>,
    /// « isantigel ».
    pub antigel: bool,
    /// « θs,ech,rep,LIM », en °C.
    pub theta_s_ech_rep_lim_c: f64,
}

/// Bouche et conduit du système, prêts pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct BoucheProjet {
    /// Indice du groupe de la bouche dans la zone.
    pub groupe: usize,
    /// Débits de la bouche et de son conduit : constants en habitation, ceux de l'occupation
    /// (`Ivent` vrai) dans les autres usages.
    pub debits: DebitsBoucheProjet,
    /// Débits en inoccupation (`Ivent` faux) dans les usages autres que l'habitation ; `None`
    /// en habitation.
    pub debits_inocc: Option<DebitsBoucheProjet>,
    /// « Dugd » de la bouche, en h par semaine.
    pub dugd_h: f64,
    /// « Hcond,rep,ext » ou « Hcond,souf,ext », en W/K.
    pub h_cond_ext_w_k: f64,
}

impl BoucheProjet {
    /// Débits de la bouche selon l'indicateur de ventilation de la zone.
    pub fn debits_selon(&self, i_vent: bool) -> DebitsBoucheProjet {
        match self.debits_inocc {
            Some(inocc) if !i_vent => inocc,
            _ => self.debits,
        }
    }
}

/// État d'un groupe vu par le système de ventilation, à la fin du pas de temps précédent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EtatGroupeVentilation {
    /// « θi,fin(h−1) », en °C.
    pub theta_i_fin_c: f64,
    /// Saison de chauffage du groupe (« Autch,eff(j) », §6.3.3.4.1, éq. 409).
    pub saison_de_chauffage: bool,
}

/// Air soufflé dans un groupe par le système au pas de temps.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AirSouffle {
    /// « Tair_soufflé », en °C (moyenne des bouches du groupe pondérée par leurs débits).
    pub t_air_souffle_c: f64,
    /// « ωair_soufflé », en kg/kg.
    pub omega_air_souffle_kg_kg: f64,
}

/// Sorties du système au pas de temps.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PasVentilation {
    /// « θair_extrait(h) » : température de l'air rejeté par le système, en °C (éq. 654 ;
    /// éq. 513 en double flux) ; préparée pour les générateurs thermodynamiques sur air extrait.
    pub theta_air_extrait_c: f64,
    /// « qm,air_extrait(h) » : débit d'air rejeté, en kg/s (éq. 654 et 456).
    pub qm_air_extrait_kg_s: f64,
    /// « ε(h) » : efficacité de l'échangeur au pas de temps (double flux).
    pub epsilon: f64,
    /// « δbypass(h) » (double flux).
    pub bypass: bool,
    /// « Wag(h) » : énergie fournie pour la sécurité antigel, en Wh (éq. 508).
    pub w_ag_wh: f64,
}

/// Système de ventilation du projet d'une zone d'habitation, prêt pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct VentilationProjet {
    /// Identifiant du système.
    pub id: String,
    /// Nature du système.
    pub nature: NatureSysteme,
    /// « b » du réseau (éq. 471, 649 et 665).
    pub b: f64,
    /// Bouches et conduits.
    pub bouches: Vec<BoucheProjet>,
    /// Puissance moyenne du ventilateur de reprise, en W (éq. 393, 620 et 641) ; hors
    /// habitation, puissance en occupation (éq. 636).
    pub p_vent_rep_w: f64,
    /// Puissance moyenne du ventilateur de soufflage, en W (éq. 391 et 643) ; hors habitation,
    /// puissance en occupation (éq. 638).
    pub p_vent_sou_w: f64,
    /// Puissances des ventilateurs de reprise et de soufflage en inoccupation, en W, hors
    /// habitation (éq. 637 et 639) ; `None` en habitation, où la puissance est constante.
    pub p_vent_inocc_w: Option<(f64, f64)>,
    /// Nombre de groupes de la zone.
    pub nb_groupes: usize,
    /// Valeurs par défaut du texte appliquées ; champ relatif à la zone.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
    /// « ρair_soufflé^g,s(h−1) » de chaque bouche, en kg/m³.
    rho_souffle: Vec<f64>,
}

fn defaut(champ: String, valeur: f64, regle: &str) -> ValeurParDefaut {
    ValeurParDefaut {
        champ,
        valeur,
        regle: regle.to_owned(),
    }
}

impl VentilationProjet {
    /// Prépare le système de ventilation d'une zone d'habitation validée.
    ///
    /// `theta_initiale_c` est la température des groupes avant le premier pas de temps : l'air
    /// soufflé au premier pas de temps est réputé à cette température pour le calcul des débits
    /// massiques (éq. 420, `ρair_soufflé(h−1)`). Renvoie `None` si la zone n'a pas de système
    /// ou si son système n'est pas pris en charge (cas écartés par la validation).
    pub fn nouvelle(zone: &Zone, theta_initiale_c: f64) -> Option<Self> {
        let v = zone.ventilation.as_ref()?;
        let typologie = match zone.usage {
            UsageZone::MaisonIndividuelle => TypologieReseau::MaisonIndividuelle,
            UsageZone::LogementCollectif => TypologieReseau::BatimentCollectif,
            _ => TypologieReseau::NonResidentiel,
        };
        let non_residentiel = typologie == TypologieReseau::NonResidentiel;
        let mut valeurs_par_defaut = Vec::new();
        let (nature, pression) = match &v.systeme {
            SystemeVentilation::SimpleFluxExtraction(_) => {
                (NatureSysteme::SimpleFluxExtraction, PressionReseau::Haute)
            }
            SystemeVentilation::SimpleFluxInsufflation(_) => {
                (NatureSysteme::SimpleFluxInsufflation, PressionReseau::Haute)
            }
            SystemeVentilation::MecaniqueBassePression(_) => (
                NatureSysteme::BassePression,
                mecanique_basse_pression::PRESSION_RESEAU,
            ),
            SystemeVentilation::DoubleFlux(df) => {
                let echangeur = df.echangeur.as_ref().map(|e| {
                    let efficacite = match &e.representation {
                        RepresentationEchangeur::Simplifiee(s) => {
                            let statut = s.statut.unwrap_or_else(|| {
                                valeurs_par_defaut.push(defaut(
                                    "ventilation.systeme.double_flux.echangeur.representation.\
                                     simplifiee.statut"
                                        .to_owned(),
                                    0.0,
                                    "Th-BCE 2020 §6.3.2, tableau 61 : « idstatut_echangeur », \
                                     0 = valeur déclarée par le fabricant (par défaut)",
                                ));
                                StatutEchangeur::Declare
                            });
                            EfficaciteEchangeur::Simplifiee(epsilon_t(s.epsilon_saisi, statut))
                        }
                        RepresentationEchangeur::Detaillee(d) => EfficaciteEchangeur::Detaillee {
                            ua_w_k: d.ua_w_k,
                            ecoulement: d.ecoulement,
                        },
                    };
                    let theta_s_ech_rep_lim_c = e.theta_s_ech_rep_lim_c.unwrap_or_else(|| {
                        valeurs_par_defaut.push(defaut(
                            "ventilation.systeme.double_flux.echangeur.theta_s_ech_rep_lim_c"
                                .to_owned(),
                            THETA_S_ECH_REP_LIM_DEFAUT_C,
                            "Th-BCE 2020 §6.3.3.4.6.4, tableau 62 : « autres cas »",
                        ));
                        THETA_S_ECH_REP_LIM_DEFAUT_C
                    });
                    EchangeurResolu {
                        efficacite,
                        bypass: e.bypass.clone(),
                        antigel: e.antigel.unwrap_or(false),
                        theta_s_ech_rep_lim_c,
                    }
                });
                (NatureSysteme::DoubleFlux(echangeur), PressionReseau::Haute)
            }
            _ => return None,
        };

        // §6.2.3.4, tableau 59 : coefficient de fuite.
        let k_res = k_res(v.reseau.classe_etancheite);
        if v.reseau.classe_etancheite.is_none() {
            valeurs_par_defaut.push(defaut(
                "ventilation.reseau.classe_etancheite".to_owned(),
                K_RES_DEFAUT,
                "Th-BCE 2020 §6.2.3.4, tableau 59 : Kres de la valeur par défaut « DEF », en \
                 m³/(s.m²) sous 1 Pa",
            ));
        }
        let d_p_pa = typologie.d_p_pa(pression);
        // Surfaces saisies (Is_surf_cond_def = 0) si toutes les bouches en portent une.
        let surfaces_saisies = zone
            .groupes
            .iter()
            .flat_map(|g| &g.bouches_conduits)
            .all(|b| b.a_cond_m2.is_some());
        let ratfuitevc = match v.reseau.ratfuitevc {
            Some(r) => r,
            None => {
                let r = typologie.ratfuitevc_defaut();
                valeurs_par_defaut.push(defaut(
                    "ventilation.reseau.ratfuitevc".to_owned(),
                    r,
                    "Th-BCE 2020 §6.2.3.4, tableau 60 : valeur par défaut",
                ));
                r
            }
        };

        let mut bouches = Vec::new();
        for (ig, g) in zone.groupes.iter().enumerate() {
            // Premier passage : débits de base de chaque bouche, pour la répartition de la
            // surface par défaut en maison individuelle (note de l'éq. 423).
            let mut provisoires = Vec::with_capacity(g.bouches_conduits.len());
            for (ib, b) in g.bouches_conduits.iter().enumerate() {
                let chemin = format!("groupes[{ig}].bouches_conduits[{ib}]");
                let dugd_h = b.dugd_h.unwrap_or_else(|| {
                    valeurs_par_defaut.push(defaut(
                        format!("{chemin}.dugd_h"),
                        DUGD_DEFAUT_H,
                        "Th-BCE 2020 §6.2.3.2.2.2, tableau 57 : dispositif à gestion manuelle \
                         (par défaut)",
                    ));
                    DUGD_DEFAUT_H
                });
                let cdep = b.cdep.unwrap_or_else(|| {
                    valeurs_par_defaut.push(defaut(
                        format!("{chemin}.cdep"),
                        CDEP_DEFAUT,
                        "Th-BCE 2020 §6.2.3.3.1, tableau 58 : valeur par défaut",
                    ));
                    CDEP_DEFAUT
                });
                let (q_max, q_base, q_dep, q_dep_inocc) = if non_residentiel {
                    let crdbnr = b.crdbnr.unwrap_or_else(|| {
                        valeurs_par_defaut.push(defaut(
                            format!("{chemin}.crdbnr"),
                            CRDBNR_DEFAUT,
                            "Th-BCE 2020 §6.2.3.2.1, tableau 57 : aucune régulation des débits",
                        ));
                        CRDBNR_DEFAUT
                    });
                    let q_inocc = b.q_spec_inocc_m3_h?;
                    let (q_max, q_dep, q_dep_inocc) = debits_dep_non_residentiel(
                        b.isouf,
                        b.q_spec_occ_m3_h?,
                        q_inocc,
                        crdbnr,
                        cdep,
                    );
                    (q_max, q_inocc, q_dep, Some(q_dep_inocc))
                } else {
                    let (q_max, q_base, q_dep) = debits_dep_habitation(b, dugd_h, cdep)?;
                    (q_max, q_base, q_dep, None)
                };
                provisoires.push((b, dugd_h, q_max, q_base, q_dep, q_dep_inocc));
            }
            let base_totale = |isouf: Isouf| {
                provisoires
                    .iter()
                    .filter(|p| p.0.isouf == isouf)
                    .map(|p| p.3)
                    .sum::<f64>()
            };
            let nb_circuit =
                |isouf: Isouf| provisoires.iter().filter(|p| p.0.isouf == isouf).count();
            for (ib, (b, dugd_h, q_max, q_base, q_dep, q_dep_inocc)) in
                provisoires.iter().copied().enumerate()
            {
                let a_cond_m2 = if surfaces_saisies {
                    b.a_cond_m2.unwrap_or(0.0)
                } else {
                    let a = match typologie {
                        // Éq. (421) à (423), surface répartie au prorata des débits de base.
                        TypologieReseau::MaisonIndividuelle => {
                            let total = base_totale(b.isouf);
                            let part = if total > 0.0 {
                                q_base / total
                            } else {
                                1.0 / nb_circuit(b.isouf) as f64
                            };
                            a_cond_defaut_maison_m2(g.shab_m2.unwrap_or(0.0)) * part
                        }
                        // Éq. (424) à (427).
                        TypologieReseau::BatimentCollectif | TypologieReseau::NonResidentiel => {
                            a_cond_defaut_collectif_m2(q_max)
                        }
                    };
                    valeurs_par_defaut.push(defaut(
                        format!("groupes[{ig}].bouches_conduits[{ib}].a_cond_m2"),
                        a,
                        "Th-BCE 2020 §6.2.3.4, éq. (421) à (427), tableau 60 : surface de \
                         conduit par défaut",
                    ));
                    a
                };
                let debits = DebitsBoucheProjet {
                    isouf: b.isouf,
                    q_max_m3_h: q_max,
                    q_base_m3_h: q_base,
                    q_dep_m3_h: q_dep,
                    q_fuites_m3_h: q_fuites_m3_h(b.isouf, q_dep, k_res, a_cond_m2, d_p_pa),
                    ratfuitevc,
                    a_cond_m2,
                };
                let r = match b.isouf {
                    Isouf::Reprise => v.reseau.r_rep_m2k_w,
                    Isouf::Soufflage => v.reseau.r_souf_m2k_w,
                }
                .unwrap_or(0.0);
                // Hors habitation, débits de l'inoccupation : mêmes conduits, fuites nulles si le
                // débit est nul (éq. 419 et 420).
                let debits_inocc = q_dep_inocc.map(|q_dep_inocc| DebitsBoucheProjet {
                    q_dep_m3_h: q_dep_inocc,
                    q_fuites_m3_h: q_fuites_m3_h(b.isouf, q_dep_inocc, k_res, a_cond_m2, d_p_pa),
                    ..debits
                });
                bouches.push(BoucheProjet {
                    groupe: ig,
                    debits,
                    debits_inocc,
                    dugd_h,
                    // Éq. (395), (650), (651), (666) et (667).
                    h_cond_ext_w_k: h_cond_ext_w_k(debits.a_cond_ext_m2(), r),
                });
            }
        }

        // Puissances moyennes des ventilateurs : éq. (390) à (394), (619), (620), (640), (641)
        // et (643) en habitation ; puissances en occupation et en inoccupation dans les autres
        // usages (éq. 636 à 639).
        let dugd = dugd_equ_h(bouches.iter().map(|b| b.dugd_h));
        let residentiel = |p: &crate::entree::VentilateurSimpleFlux| {
            p_vent_residentiel_w(
                p.p_vent_base_w.unwrap_or(0.0),
                p.p_vent_pointe_w.unwrap_or(0.0),
                dugd,
            )
        };
        let (p_vent_rep_w, p_vent_sou_w, p_vent_inocc_w) = match &v.systeme {
            SystemeVentilation::SimpleFluxExtraction(p) if non_residentiel => {
                (p.p_vent_occ_w?, 0.0, Some((p.p_vent_inocc_w?, 0.0)))
            }
            SystemeVentilation::SimpleFluxInsufflation(p) if non_residentiel => {
                (0.0, p.p_vent_occ_w?, Some((0.0, p.p_vent_inocc_w?)))
            }
            // Basse pression et double flux : habitation seulement dans cette version.
            _ if non_residentiel => return None,
            SystemeVentilation::SimpleFluxExtraction(p)
            | SystemeVentilation::MecaniqueBassePression(p) => (residentiel(p), 0.0, None),
            SystemeVentilation::SimpleFluxInsufflation(p) => (0.0, residentiel(p), None),
            SystemeVentilation::DoubleFlux(df) => (
                p_vent_residentiel_w(df.p_vent_base_rep_w, df.p_vent_pointe_rep_w, dugd),
                p_vent_residentiel_w(df.p_vent_base_sou_w, df.p_vent_pointe_sou_w, dugd),
                None,
            ),
            _ => return None,
        };
        let rho_initiale = rho_kg_m3(theta_initiale_c);
        Some(Self {
            id: v.id.clone(),
            nature,
            b: v.reseau.b,
            rho_souffle: vec![rho_initiale; bouches.len()],
            bouches,
            p_vent_rep_w,
            p_vent_sou_w,
            p_vent_inocc_w,
            nb_groupes: zone.groupes.len(),
            valeurs_par_defaut,
        })
    }

    /// Vrai si les débits et la puissance dépendent de l'indicateur de ventilation de la zone
    /// (usages autres que l'habitation).
    pub fn selon_occupation(&self) -> bool {
        self.p_vent_inocc_w.is_some()
    }

    /// « qspec_repris » et « qspec_soufflé » d'un groupe, en m³/h : sommes des débits
    /// spécifiques de ses bouches, fuites en volume chauffé comprises (éq. 428 et 429). Hors
    /// habitation, débits de l'occupation.
    pub fn debits_groupe(&self, g: usize) -> Debits {
        self.debits_groupe_selon(g, true)
    }

    /// Débits spécifiques d'un groupe selon l'indicateur de ventilation `Ivent` de la zone
    /// (sans effet en habitation).
    pub fn debits_groupe_selon(&self, g: usize, i_vent: bool) -> Debits {
        self.bouches
            .iter()
            .filter(|b| b.groupe == g)
            .fold(Debits::default(), |mut d, b| {
                let debits = b.debits_selon(i_vent);
                match debits.isouf {
                    Isouf::Reprise => d.q_rep_m3_h += debits.q_spec_m3_h(),
                    Isouf::Soufflage => d.q_souffle_m3_h += debits.q_spec_m3_h(),
                }
                d
            })
    }

    /// Débits vus par les ventilateurs pour un groupe, en m³/h (éq. 430 et 431). Hors
    /// habitation, débits de l'occupation.
    pub fn debits_cond_groupe(&self, g: usize) -> Debits {
        self.debits_cond_groupe_selon(g, true)
    }

    /// Débits vus par les ventilateurs pour un groupe selon `Ivent`.
    pub fn debits_cond_groupe_selon(&self, g: usize, i_vent: bool) -> Debits {
        self.bouches
            .iter()
            .filter(|b| b.groupe == g)
            .fold(Debits::default(), |mut d, b| {
                let debits = b.debits_selon(i_vent);
                match debits.isouf {
                    Isouf::Reprise => d.q_rep_m3_h += debits.q_cond_m3_h(),
                    Isouf::Soufflage => d.q_souffle_m3_h += debits.q_cond_m3_h(),
                }
                d
            })
    }

    /// Puissance moyenne totale des ventilateurs du système, en W. Hors habitation, puissance
    /// en occupation.
    pub fn p_vent_w(&self) -> f64 {
        self.p_vent_rep_w + self.p_vent_sou_w
    }

    /// Puissances des ventilateurs de reprise et de soufflage selon `Ivent`, en W (éq. 636 à
    /// 639 hors habitation ; constantes en habitation).
    pub fn p_vent_selon(&self, i_vent: bool) -> (f64, f64) {
        match self.p_vent_inocc_w {
            Some(inocc) if !i_vent => inocc,
            _ => (self.p_vent_rep_w, self.p_vent_sou_w),
        }
    }

    /// Puissance des ventilateurs attribuée à un groupe, en W ; c'est aussi sa consommation
    /// horaire, en Wh, la puissance étant constante en habitation (éq. 392, 394, 641 et 643).
    /// Hors habitation, puissance en occupation.
    ///
    /// Simple flux par extraction et basse pression : au prorata des débits spécifiques repris
    /// (éq. 642) ; par insufflation : des débits spécifiques soufflés (éq. 644) ; double flux :
    /// des débits soufflés vus par le ventilateur (éq. 570, débits massiques lus en débits
    /// volumiques, `docs/questions/Q042.md`).
    pub fn p_vent_groupe_w(&self, g: usize) -> f64 {
        self.p_vent_groupe_selon_w(g, true)
    }

    /// Puissance des ventilateurs attribuée à un groupe selon `Ivent`, en W (éq. 642 et 644 :
    /// « en résidentiel et en non résidentiel », au prorata des débits spécifiques du pas de
    /// temps).
    pub fn p_vent_groupe_selon_w(&self, g: usize, i_vent: bool) -> f64 {
        let (p_rep, p_sou) = self.p_vent_selon(i_vent);
        let p_vent_w = p_rep + p_sou;
        let somme = |f: &dyn Fn(&Debits) -> f64, cond: bool| {
            let de = |i: usize| {
                if cond {
                    self.debits_cond_groupe_selon(i, i_vent)
                } else {
                    self.debits_groupe_selon(i, i_vent)
                }
            };
            let total: f64 = (0..self.nb_groupes).map(|i| f(&de(i))).sum();
            (f(&de(g)), total)
        };
        match self.nature {
            NatureSysteme::SimpleFluxExtraction | NatureSysteme::BassePression => {
                let (q, total) = somme(&|d| d.q_rep_m3_h, false);
                p_vent_groupe_w(p_vent_w, q, total)
            }
            NatureSysteme::SimpleFluxInsufflation => {
                let (q, total) = somme(&|d| d.q_souffle_m3_h, false);
                p_vent_groupe_w(p_vent_w, q, total)
            }
            NatureSysteme::DoubleFlux(_) => {
                let (q, total) = somme(&|d| d.q_souffle_m3_h, true);
                p_vent_groupe_w(p_vent_w, q, total)
            }
        }
    }

    /// Calcule un pas de temps du système : air soufflé dans chaque groupe (`air`, un élément
    /// par groupe) et sorties du système.
    ///
    /// `groupes` donne l'état de chaque groupe à la fin du pas de temps précédent. Un groupe
    /// sans bouche de soufflage reçoit un air soufflé à la température et à l'humidité
    /// extérieures, sans emploi puisque son débit soufflé est nul. `i_vent` est l'indicateur
    /// de ventilation de la zone (§4.1.3, éq. 45), sans effet en habitation.
    pub fn pas(
        &mut self,
        theta_ext_c: f64,
        omega_ext_kg_kg: f64,
        groupes: &[EtatGroupeVentilation],
        air: &mut [AirSouffle],
        i_vent: bool,
    ) -> PasVentilation {
        let (p_vent_rep_w, p_vent_sou_w) = self.p_vent_selon(i_vent);
        for a in air.iter_mut() {
            *a = AirSouffle {
                t_air_souffle_c: theta_ext_c,
                // Éq. (514), (563) et (669) : humidité de l'air extérieur.
                omega_air_souffle_kg_kg: omega_ext_kg_kg,
            };
        }
        let b = self.b;
        let t_lnc = |g: usize| t_lnc_c(b, groupes[g].theta_i_fin_c, theta_ext_c);

        // Circuit de reprise : pertes des conduits et ventilateur d'extraction (éq. 469 à 475,
        // 647 à 655). Débits massiques à la masse volumique du groupe au pas précédent (éq. 412,
        // 443, 656).
        let mut qm_rep = 0.0;
        let mut somme_t_rep = 0.0;
        let mut somme_theta_i_rep = 0.0;
        for bouche in self
            .bouches
            .iter()
            .filter(|b| b.debits_selon(i_vent).isouf == Isouf::Reprise)
        {
            let theta_i = groupes[bouche.groupe].theta_i_fin_c;
            let rho = rho_kg_m3(theta_i);
            let lnc = t_lnc(bouche.groupe);
            let q_cond = bouche.debits_selon(i_vent).q_cond_m3_h();
            let qm = qm_kg_s(rho, q_cond).abs();
            let theta_cond = theta_i_cond_c(
                bouche.debits_selon(i_vent).q_spec_m3_h(),
                q_cond,
                theta_i,
                lnc,
            );
            let t_extr1 = t_apres_conduit_c(theta_cond, lnc, bouche.h_cond_ext_w_k, qm);
            qm_rep += qm;
            somme_t_rep += qm * t_extr1;
            somme_theta_i_rep += qm * theta_i;
        }
        let (t_extr1, theta_i_moy) = if qm_rep > 0.0 {
            (somme_t_rep / qm_rep, somme_theta_i_rep / qm_rep)
        } else {
            (theta_ext_c, theta_ext_c)
        };
        let t_extr2 = t_apres_ventilateur_c(t_extr1, p_vent_rep_w, qm_rep);
        let mut sorties = PasVentilation {
            theta_air_extrait_c: t_extr2,
            qm_air_extrait_kg_s: qm_rep,
            ..PasVentilation::default()
        };

        // Circuit de soufflage.
        let qm_sou: f64 = self
            .bouches
            .iter()
            .zip(&self.rho_souffle)
            .filter(|(b, _)| b.debits_selon(i_vent).isouf == Isouf::Soufflage)
            .map(|(b, rho)| qm_kg_s(*rho, b.debits_selon(i_vent).q_cond_m3_h()))
            .sum();
        if qm_sou <= 0.0 {
            return sorties;
        }
        // Air neuf : air extérieur (éq. 413 et 657, sans puits).
        let theta_an = theta_ext_c;
        let theta_sortie_centrale = match &self.nature {
            NatureSysteme::DoubleFlux(echangeur) => {
                let saison_de_chauffage = groupes.iter().any(|g| g.saison_de_chauffage);
                let (theta_souf_ag, epsilon) = match echangeur {
                    None => (theta_an, 0.0),
                    Some(e) => {
                        // Éq. (492) à (495) : bypass, avec la température des groupes repris.
                        let bypass = e.bypass.as_ref().is_some_and(|by| {
                            delta_bypass(by, saison_de_chauffage, theta_an, theta_i_moy)
                        });
                        sorties.bypass = bypass;
                        // Éq. (496) à (505).
                        let epsilon = if bypass {
                            0.0
                        } else {
                            match e.efficacite {
                                EfficaciteEchangeur::Simplifiee(eps) => eps,
                                EfficaciteEchangeur::Detaillee { ua_w_k, ecoulement } => {
                                    epsilon_detaille(
                                        ua_w_k,
                                        ecoulement,
                                        C_PA_J_KG_K * qm_sou,
                                        C_PA_J_KG_K * qm_rep,
                                    )
                                }
                            }
                        };
                        // Éq. (506) à (511) ; θextr3 = θextr2 sans humidification indirecte
                        // (éq. 484 et 485).
                        let a = antigel(
                            epsilon,
                            e.antigel,
                            saison_de_chauffage,
                            theta_an,
                            t_extr2,
                            e.theta_s_ech_rep_lim_c,
                            qm_sou,
                        );
                        sorties.w_ag_wh = a.w_ag_wh;
                        (a.theta_souf_ag_c, a.epsilon)
                    }
                };
                sorties.epsilon = epsilon;
                // Éq. (512) et (513).
                let (theta_souf1, theta_rejete) =
                    sorties_echangeur_c(theta_souf_ag, t_extr2, epsilon);
                sorties.theta_air_extrait_c = theta_rejete;
                // Éq. (517) à (520) et (560) : ventilateur de soufflage, puis sortie de centrale.
                t_apres_ventilateur_c(theta_souf1, p_vent_sou_w, qm_sou)
            }
            // Éq. (663) : ventilateur d'insufflation.
            _ => t_apres_ventilateur_c(theta_an, p_vent_sou_w, qm_sou),
        };

        // Pertes des conduits de soufflage, bouche par bouche (éq. 562 et 664), température de
        // l'air soufflé dans chaque groupe (éq. 668), masse volumique pour le pas suivant
        // (éq. 564 et 670).
        let mut somme_q = vec![0.0; air.len()];
        let mut somme_qt = vec![0.0; air.len()];
        for (bouche, rho) in self.bouches.iter().zip(self.rho_souffle.iter_mut()) {
            if bouche.debits_selon(i_vent).isouf != Isouf::Soufflage {
                continue;
            }
            let qm = qm_kg_s(*rho, bouche.debits_selon(i_vent).q_cond_m3_h());
            let t = t_apres_conduit_c(
                theta_sortie_centrale,
                t_lnc(bouche.groupe),
                bouche.h_cond_ext_w_k,
                qm,
            );
            *rho = rho_kg_m3(t);
            let q = bouche.debits_selon(i_vent).q_spec_m3_h();
            if let (Some(sq), Some(sqt)) = (
                somme_q.get_mut(bouche.groupe),
                somme_qt.get_mut(bouche.groupe),
            ) {
                *sq += q;
                *sqt += q * t;
            }
        }
        for ((a, q), qt) in air.iter_mut().zip(&somme_q).zip(&somme_qt) {
            if *q > 0.0 {
                a.t_air_souffle_c = qt / q;
            }
        }
        sorties
    }
}
