// SPDX-License-Identifier: GPL-3.0-or-later
//! Protections mobiles : fiches `C_BAT_Gestion_protections_mobiles` (Th-BCE 2020 §5.9) et
//! `C_BAT_Perméabilité_des_PM` (§5.12).
//!
//! Périmètre : première protection mobile de type volet ou store enroulable, en gestion
//! manuelle (motorisée ou non), pour les usages de la famille « habitation, hôtellerie et
//! hébergement » pris en charge ; seconde protection mobile ; règles propres au calcul Th-D
//! (§5.9.3.6). La gestion automatique et les stores vénitiens ne sont pas pris en charge : la
//! validation les refuse (`docs/questions/`, Q30 et Q31).
//!
//! En l'absence d'espace tampon, `becl = 1` et `Etp** = Etp*` (éq. 176).

use thbce_data::protections_mobiles::{
    LigneGpm, LigneInoccupation, LigneOccupation, MatriceManuelle, R_PROT2_OCC0, TOP_J1_LIM_MANU_C,
    V_VENT_LIM_MANU_SE_M_S,
};

use crate::entree::{GestionPm, ProtectionMobile, TypePm, TypoPm};

use super::baie::{GestionProtection, Protections};

/// Températures opératives maximales du jour et de la veille, par groupe.
///
/// Th-BCE 2020 §5.9.3.2.1, éq. (175).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TemperaturesMaximales {
    /// « Topjmax » : température opérative maximale du jour en cours, en °C.
    pub top_j_max_c: f64,
    /// « Topj-1max » : température opérative maximale du jour précédent, en °C. Nulle le premier
    /// jour (note de l'éq. 175).
    pub top_j1_max_c: f64,
}

impl TemperaturesMaximales {
    /// Met à jour les maximums au début d'un pas de temps.
    ///
    /// Éq. (175), dans l'ordre du texte : `Topjmax = max(Topjmax ; θop,fin(h−1))` ; puis, si
    /// `Jour_année` vient d'être incrémenté, `Topj-1max = Topjmax` et `Topjmax = 0`.
    pub fn avancer(&mut self, theta_op_fin_precedent_c: f64, nouveau_jour: bool) {
        self.top_j_max_c = self.top_j_max_c.max(theta_op_fin_precedent_c);
        if nouveau_jour {
            self.top_j1_max_c = self.top_j_max_c;
            self.top_j_max_c = 0.0;
        }
    }
}

/// « saison_GPM » : saison au sens de la gestion des protections mobiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaisonGpm {
    /// 1 : hiver (chauffage).
    Hiver,
    /// 2 : mi-saison.
    MiSaison,
    /// 3 : été (refroidissement).
    Ete,
}

impl SaisonGpm {
    /// Th-BCE 2020 §5.9.3.2.6, éq. (181), à partir de la saison effective du groupe
    /// (1 : refroidissement ; 2 : mi-saison ; 3 : chauffe avec interdiction de refroidir ;
    /// 4 : chauffe avec autorisation de refroidir) : 3 donne l'hiver, 2 et 4 la mi-saison, 1
    /// l'été. Le texte écrit « si Saison = 2 et Saison = 4 » : lu « ou » (question Q31).
    /// Renvoie `None` hors de 1 à 4.
    pub fn de_saison_effective(saison_eff: u8) -> Option<Self> {
        match saison_eff {
            1 => Some(Self::Ete),
            2 | 4 => Some(Self::MiSaison),
            3 => Some(Self::Hiver),
            _ => None,
        }
    }
}

/// « Jour » en gestion manuelle.
///
/// Th-BCE 2020 §5.9.3.2.7.1, éq. (183) : si `iocc_gpm = 1`, il fait jour quand `Etp** ≠ 0` ; si
/// `iocc_gpm = 0` (inoccupation de jour), toujours ; si `iocc_gpm = −1` (inoccupation de nuit ou
/// de vacances), jamais.
pub fn jour_gestion_manuelle(i_occ_gpm: i8, etp_lux: f64) -> bool {
    match i_occ_gpm {
        1 => etp_lux != 0.0,
        0 => true,
        _ => false,
    }
}

/// Ratio de fermeture en fonction de l'éclairement.
///
/// Th-BCE 2020 §5.9.3.2.3, éq. (177) : `Rprot = 1` si `Etp** ≥ Eclman` ; sinon
/// `Rprot = Rprot0 + (1 − Rprot0)·Etp**/Eclman`.
pub fn r_prot_selon_eclairement(etp_lux: f64, r_prot0: f64, ecl_man_lux: f64) -> f64 {
    if etp_lux >= ecl_man_lux {
        1.0
    } else {
        r_prot0 + (1.0 - r_prot0) * etp_lux / ecl_man_lux
    }
}

/// Conditions d'un pas de temps pour la gestion des protections mobiles d'une baie.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionsGpm {
    /// « iocc_zone » : zone occupée.
    pub i_occ_zone: bool,
    /// « iocc_gpm » : 1 zone occupée ; 0 inoccupée le jour ; −1 inoccupée la nuit ou en vacances.
    pub i_occ_gpm: i8,
    /// « Etp* » : éclairement incident total sur la baie, en lux.
    pub etp_lux: f64,
    /// « Vvent_c » : vitesse du vent corrigée, en m/s.
    pub v_vent_c_m_s: f64,
    /// Saison au sens de la gestion des protections mobiles.
    pub saison: SaisonGpm,
    /// « Topj-1max » : température opérative maximale du jour précédent, en °C.
    pub top_j1_max_c: f64,
    /// Vrai en calcul Th-D pendant la période de confort adaptatif (`Isconf_adapt = 1`).
    pub confort_adaptatif_th_d: bool,
}

/// Gestion d'une baie, prête pour le calcul horaire.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GestionResolue {
    type_pm: TypePm,
    motorisee: bool,
    exterieure: bool,
    pm2: bool,
    p_occ: f64,
    matrice: &'static MatriceManuelle,
    th_d: thbce_data::protections_mobiles::ValeursThD,
}

impl GestionResolue {
    /// Résout la gestion d'une protection mobile pour un usage. Renvoie `None` pour ce que cette
    /// version ne prend pas en charge : store vénitien, gestion automatique, usage d'une famille
    /// dont les matrices ne sont pas transcrites (cas écartés par la validation).
    pub fn nouvelle(pm: &ProtectionMobile, usage: u8) -> Option<Self> {
        let (famille, parts) = thbce_data::protections_mobiles::famille_et_parts(usage)?;
        let (volet, store_enroulable) = thbce_data::protections_mobiles::matrices(famille)?;
        let th_d = thbce_data::protections_mobiles::valeurs_th_d(famille)?;
        let matrice = match pm.type_pm {
            TypePm::Volet => volet,
            TypePm::StoreEnroulable => store_enroulable,
            TypePm::StoreVenitien => return None,
        };
        let motorisee = match pm.gestion {
            GestionPm::ManuelleNonMotorisee => false,
            GestionPm::ManuelleMotorisee => true,
            GestionPm::AutomatiqueAvecDerogation => return None,
        };
        Some(Self {
            type_pm: pm.type_pm,
            motorisee,
            exterieure: pm.exterieure,
            pm2: pm.seconde_protection,
            p_occ: parts.p_occ,
            matrice,
            th_d,
        })
    }

    fn lignes(&self, c: &ConditionsGpm) -> (LigneOccupation, LigneInoccupation, bool) {
        let chaud = c.top_j1_max_c >= TOP_J1_LIM_MANU_C;
        let ligne = match c.saison {
            SaisonGpm::Hiver => LigneGpm::Hiver,
            SaisonGpm::MiSaison => LigneGpm::MiSaison,
            SaisonGpm::Ete if chaud => LigneGpm::EteChaud,
            SaisonGpm::Ete => LigneGpm::EteFrais,
        } as usize;
        let m = self.matrice;
        let (mut occ, mut inocc) = if self.motorisee {
            (
                m.occupation_motorisee[ligne],
                m.inoccupation_motorisee[ligne],
            )
        } else {
            (
                m.occupation_non_motorisee[ligne],
                m.inoccupation_non_motorisee[ligne],
            )
        };
        // Th-BCE 2020 §5.9.3.6, éq. (202) à (204) et tableau 34 (colonne de la famille d'usages) : en
        // Th-D, en période de confort adaptatif et quand Topj-1max ≥ Topj-1lim_manu, seuls les
        // ratios changent ; les seuils d'éclairement sont conservés.
        let th_d_chaud = c.confort_adaptatif_th_d && chaud;
        if th_d_chaud {
            let plancher = if self.motorisee {
                self.th_d.r_prot_jour_min_motorisee
            } else {
                self.th_d.r_prot_jour_min_autre
            };
            occ.r_prot0 = occ.r_prot0.max(plancher);
            // Colonne « Autres » : plancher propre à l'inoccupation (60 % contre 70 %).
            inocc.r_prot_jour = inocc
                .r_prot_jour
                .max(self.th_d.r_prot_inocc_jour_min.unwrap_or(plancher));
            match self.type_pm {
                TypePm::StoreEnroulable if self.th_d.nuit_store_plafonne => {
                    // Colonne « Hôpitaux nuit » : min(50 % ; valeur Th-BC).
                    occ.r_prot_nuit = occ.r_prot_nuit.min(self.th_d.r_prot_nuit_store_enroulable);
                    inocc.r_prot_nuit = inocc
                        .r_prot_nuit
                        .min(self.th_d.r_prot_nuit_store_enroulable);
                }
                TypePm::StoreEnroulable => {
                    occ.r_prot_nuit = self.th_d.r_prot_nuit_store_enroulable;
                    inocc.r_prot_nuit = self.th_d.r_prot_nuit_store_enroulable;
                }
                _ => {
                    occ.r_prot_nuit = occ.r_prot_nuit.min(self.th_d.r_prot_nuit_max_volet);
                    inocc.r_prot_nuit = inocc.r_prot_nuit.min(self.th_d.r_prot_nuit_max_volet);
                }
            }
        }
        (occ, inocc, th_d_chaud)
    }

    /// Position des protections mobiles de la baie au pas de temps.
    ///
    /// Th-BCE 2020 §5.9.3, gestion manuelle :
    /// - éq. (178) : `partGPM = Pocc[z]` en occupation, 0 en inoccupation ;
    /// - volets, éq. (188) et (189) ; stores enroulables, éq. (192) et (193). En occupation, de
    ///   jour et sans seconde protection, `Rprot1partGPM` suit l'éq. (177) avec `Rprot0` et
    ///   `Ecl1` ; sinon il est lu dans la matrice (colonne « avec 2nde protection » de jour,
    ///   colonne « nuit » sinon). `Rprot11-partGPM` est le ratio d'inoccupation, de jour ou de
    ///   nuit. En inoccupation, `Rprot1partGPM = 0` ;
    /// - stores enroulables extérieurs, éq. (192) : en occupation, si
    ///   `Vvent_c ≥ Vventlim_manu_se`, les deux ratios sont nuls ;
    /// - seconde protection, éq. (198) à (200) : en occupation,
    ///   `Rprot2 = Rprot2_occ0 + (1 − Rprot2_occ0)·Etp**/Ecl2_occ1`, plafonné à 1 ; nul en
    ///   inoccupation ou sans seconde protection ;
    /// - Th-D, éq. (201) : en période de confort adaptatif, si `Topj-1max ≥ Topj-1lim_manu`, la
    ///   seconde protection est désactivée.
    pub fn protections(&self, c: &ConditionsGpm) -> Protections {
        let (occ, inocc, th_d_chaud) = self.lignes(c);
        let pm2_active = self.pm2 && !th_d_chaud;
        let jour = jour_gestion_manuelle(c.i_occ_gpm, c.etp_lux);
        let hors_part = if jour {
            inocc.r_prot_jour
        } else {
            inocc.r_prot_nuit
        };
        let manuelle = |part_gpm, r_prot1_part_gpm, r_prot1_hors_part_gpm, r_prot2| Protections {
            part_gpm,
            r_prot1_part_gpm,
            r_prot1_hors_part_gpm,
            r_prot2,
            gestion: GestionProtection::Manuelle,
        };
        if !c.i_occ_zone {
            return manuelle(0.0, 0.0, hors_part, 0.0);
        }
        let r_prot2 = if pm2_active {
            r_prot_selon_eclairement(c.etp_lux, R_PROT2_OCC0, occ.ecl2_lux)
        } else {
            0.0
        };
        if self.type_pm == TypePm::StoreEnroulable
            && self.exterieure
            && c.v_vent_c_m_s >= V_VENT_LIM_MANU_SE_M_S
        {
            return manuelle(self.p_occ, 0.0, 0.0, r_prot2);
        }
        let dans_part = if !jour {
            occ.r_prot_nuit
        } else if pm2_active {
            occ.r_prot_avec_pm2
        } else {
            r_prot_selon_eclairement(c.etp_lux, occ.r_prot0, occ.ecl1_lux)
        };
        manuelle(self.p_occ, dans_part, hors_part, r_prot2)
    }
}

/// « Tauxpassage_air_PM » : taux de passage de l'air à travers la protection mobile en place.
///
/// Th-BCE 2020 §5.12.3.1, tableau 40 : 0 % (typologie 0), 10 % (1), 25 % (2), 50 % (3), 75 % (4).
pub fn taux_passage_air(typo: TypoPm) -> f64 {
    match typo {
        TypoPm::Etanche => 0.0,
        TypoPm::FaiblementAjouree => 0.10,
        TypoPm::Ajouree => 0.25,
        TypoPm::FortementAjouree => 0.50,
        TypoPm::TresFortementAjouree => 0.75,
    }
}

/// « Aouvmax_ac_PM,b(h) » : surface d'ouverture maximale de la baie avec sa protection mobile en
/// place, en m².
///
/// Th-BCE 2020 §5.12.3.2, éq. (239) :
/// `Aouvmax_ac_PM = Aouvmax × (Rprot,b × Tauxpassage_air_PM + (1 − Rprot,b))`. Sans protection
/// mobile (typologie 5 du tableau 40, taux de 100 %), passer `taux_passage = 1`.
pub fn a_ouv_max_avec_pm_m2(a_ouv_max_m2: f64, r_prot: f64, taux_passage: f64) -> f64 {
    a_ouv_max_m2 * (r_prot * taux_passage + (1.0 - r_prot))
}
