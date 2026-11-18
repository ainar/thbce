// SPDX-License-Identifier: GPL-3.0-or-later
//! Protections mobiles : fiches `C_BAT_Gestion_protections_mobiles` (Th-BCE 2020 §5.9) et
//! `C_BAT_Perméabilité_des_PM` (§5.12).
//!
//! Périmètre : première protection mobile de type volet, store enroulable ou store vénitien, en
//! gestion manuelle (motorisée ou non) ou automatique avec dérogation (fiche d'application FA01,
//! `docs/questions/Q100.md`) ; seconde protection mobile ; règles propres au calcul Th-D
//! (§5.9.3.6).
//!
//! En l'absence d'espace tampon, `becl = 1` et `Etp** = Etp*` (éq. 176).

use thbce_data::protections_mobiles::{
    LigneGpm, LigneInoccupation, LigneOccupation, MatriceManuelle, R_PROT2_OCC0, TOP_J1_LIM_MANU_C,
    V_VENT_LIM_MANU_SE_M_S,
};
use thbce_data::protections_mobiles_auto::{
    Derogation, Hysteresis, LigneAuto, LigneAutoSv, MatriceAuto, MatriceAutoSv, MatriceSvManuelle,
};

use crate::entree::{
    Automatisme, GestionPm, LigneAutoVolet, MatriceAutoVolet, ProtectionMobile, RatiosHysteresis,
    TypeHorloge, TypePm, TypoPm,
};
use crate::resultats::ValeurParDefaut;

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
    /// « θop,fin(h−1) » : température opérative à la fin du pas précédent, en °C (gestion
    /// automatique des volets et stores enroulables, éq. 186, 187, 190 et 191).
    pub theta_op_fin_c: f64,
    /// « Hleg » : heure légale de la journée, de 1 à 24 (horloge personnalisable, éq. 182 et
    /// 184).
    pub hleg: u8,
}

/// État d'une baie d'un pas à l'autre : position haute ou basse de l'hystérésis de la gestion
/// automatique des volets et stores enroulables (§5.9.3.2.8, figure 31). Initialement basse
/// (`docs/questions/Q100.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EtatGpm {
    /// Vrai tant que `θop,fin` n'a pas repassé la limite basse depuis qu'elle a franchi la
    /// limite haute.
    pub branche_haute: bool,
}

impl EtatGpm {
    /// Th-BCE 2020 §5.9.3.2.8, figure 31 : on passe à la branche haute quand
    /// `θop,fin(h−1) > Toph-1limh`, à la branche basse quand `θop,fin(h−1) < Toph-1limb` ; entre
    /// les deux limites, la branche est conservée.
    fn mettre_a_jour(&mut self, theta_c: f64, limite_haute_c: f64, limite_basse_c: f64) {
        if theta_c > limite_haute_c {
            self.branche_haute = true;
        } else if theta_c < limite_basse_c {
            self.branche_haute = false;
        }
    }
}

/// Distinction du jour et de la nuit en gestion automatique (éq. 182 et 184).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Horloge {
    /// Niveau 0 : « Jour » toujours vrai.
    Aucune,
    /// Niveau 1, horloge crépusculaire : « Jour » vrai si `Etp** ≠ 0`.
    Crepusculaire,
    /// Niveau 2, horloge personnalisable : « Jour » vrai si `Hj_debut < Hleg ≤ Hj_fin`, par
    /// saison (hiver, mi-saison, été), en `(début, fin)`.
    Personnalisable([(u8, u8); 3]),
}

impl Horloge {
    /// « Jour » en gestion automatique. Th-BCE 2020 §5.9.3.2.7, éq. (182) et (184).
    pub fn jour(&self, etp_lux: f64, saison: SaisonGpm, hleg: u8) -> bool {
        match self {
            Self::Aucune => true,
            Self::Crepusculaire => etp_lux != 0.0,
            Self::Personnalisable(h) => {
                let (debut, fin) = h[saison.indice()];
                debut < hleg && hleg <= fin
            }
        }
    }
}

/// Gestion automatique d'un volet ou d'un store enroulable, résolue.
#[derive(Debug, Clone, Copy, PartialEq)]
struct AutoVolet {
    derogation: &'static Derogation,
    detecteur: bool,
    p_derog: f64,
    ecl_lim_lux: f64,
    top_lim_h_c: f64,
    top_lim_b_c: f64,
    /// `Vventlim_auto` (store enroulable extérieur).
    vent_lim_m_s: Option<f64>,
    horloge: Horloge,
    matrice: MatriceAuto,
}

/// Gestion d'un store vénitien, résolue.
#[derive(Debug, Clone, Copy, PartialEq)]
struct GestionSvResolue {
    manuelle: &'static MatriceSvManuelle,
    /// Gestion automatique : paramètres de l'automatisme.
    auto: Option<AutoSv>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct AutoSv {
    p_derog: f64,
    ecl_lim_lux: f64,
    vent_lim_m_s: Option<f64>,
    horloge: Horloge,
    matrice: MatriceAutoSv,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ModeGestion {
    /// Volet ou store enroulable en gestion manuelle.
    Manuelle {
        motorisee: bool,
        matrice: &'static MatriceManuelle,
    },
    /// Volet ou store enroulable en gestion automatique avec dérogation.
    Automatique(AutoVolet),
    /// Store vénitien.
    Venitien(GestionSvResolue),
}

/// Gestion d'une baie, prête pour le calcul horaire.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GestionResolue {
    type_pm: TypePm,
    exterieure: bool,
    pm2: bool,
    p_occ: f64,
    mode: ModeGestion,
    th_d: thbce_data::protections_mobiles::ValeursThD,
}

fn horloge_resolue(a: &Automatisme) -> Option<Horloge> {
    Some(match a.horloge {
        TypeHorloge::Aucune => Horloge::Aucune,
        TypeHorloge::Crepusculaire => Horloge::Crepusculaire,
        TypeHorloge::Personnalisable => {
            let h = a.horaires_jour?;
            Horloge::Personnalisable([
                (h[0].debut_h, h[0].fin_h),
                (h[1].debut_h, h[1].fin_h),
                (h[2].debut_h, h[2].fin_h),
            ])
        }
    })
}

fn ratios(h: RatiosHysteresis) -> Hysteresis {
    Hysteresis {
        haute: h.haute,
        basse: h.basse,
    }
}

fn ligne_auto(l: &LigneAutoVolet) -> LigneAuto {
    LigneAuto {
        jour_sous_seuil: ratios(l.jour_sous_seuil),
        jour_au_dessus: ratios(l.jour_au_dessus),
        nuit: ratios(l.nuit),
    }
}

fn matrice_auto_saisie(m: &MatriceAutoVolet) -> MatriceAuto {
    MatriceAuto {
        occupation: m.occupation.each_ref().map(ligne_auto),
        inoccupation: m.inoccupation.each_ref().map(ligne_auto),
    }
}

fn matrice_auto_sv_saisie(m: &crate::entree::MatriceAutoSv) -> MatriceAutoSv {
    let l = |x: &crate::entree::LigneAutoSv| LigneAutoSv {
        jour_sous_seuil: x.jour_sous_seuil,
        jour_au_dessus: x.jour_au_dessus,
        nuit: x.nuit,
    };
    MatriceAutoSv {
        occupation: m.occupation.each_ref().map(l),
        inoccupation: m.inoccupation.each_ref().map(l),
    }
}

impl SaisonGpm {
    /// Indice de saison des matrices à trois lignes : hiver 0, mi-saison 1, été 2.
    pub fn indice(self) -> usize {
        match self {
            Self::Hiver => 0,
            Self::MiSaison => 1,
            Self::Ete => 2,
        }
    }
}

impl GestionResolue {
    /// Résout la gestion d'une protection mobile pour un usage, sans signaler les valeurs par
    /// défaut. Renvoie `None` si la saisie est incomplète (cas écartés par la validation).
    pub fn nouvelle(pm: &ProtectionMobile, usage: u8) -> Option<Self> {
        Self::nouvelle_signalee(pm, usage, &mut Vec::new())
    }

    /// Résout la gestion d'une protection mobile pour un usage et signale dans `defauts` les
    /// matrices conventionnelles prises faute de saisie (figures 44 à 47).
    pub fn nouvelle_signalee(
        pm: &ProtectionMobile,
        usage: u8,
        defauts: &mut Vec<ValeurParDefaut>,
    ) -> Option<Self> {
        let (famille, parts) = thbce_data::protections_mobiles::famille_et_parts(usage)?;
        let th_d = thbce_data::protections_mobiles::valeurs_th_d(famille)?;
        let mut signaler = |champ: &str, figure: &str| {
            defauts.push(ValeurParDefaut {
                champ: champ.to_owned(),
                valeur: 0.0,
                regle: format!(
                    "Th-BCE 2020 §5.9.3.3.2, {figure} : matrice conventionnelle de gestion \
                     automatique de l'usage ; la valeur sans objet désigne la matrice entière \
                     (question Q100)"
                ),
            });
        };
        let mode = match pm.type_pm {
            TypePm::Volet | TypePm::StoreEnroulable => {
                let (volet, store_enroulable) = thbce_data::protections_mobiles::matrices(famille)?;
                let (derog_v, derog_se) = thbce_data::protections_mobiles_auto::derogation(famille);
                let (matrice, derogation) = if pm.type_pm == TypePm::Volet {
                    (volet, derog_v)
                } else {
                    (store_enroulable, derog_se)
                };
                match pm.gestion? {
                    GestionPm::ManuelleNonMotorisee => ModeGestion::Manuelle {
                        motorisee: false,
                        matrice,
                    },
                    GestionPm::ManuelleMotorisee => ModeGestion::Manuelle {
                        motorisee: true,
                        matrice,
                    },
                    GestionPm::AutomatiqueAvecDerogation => {
                        let a = pm.automatisme.as_ref()?;
                        let matrice = match &a.matrice {
                            Some(m) => matrice_auto_saisie(m),
                            None => {
                                signaler(
                                    "protection_mobile.automatisme.matrice",
                                    "figures 44 et 45",
                                );
                                *thbce_data::protections_mobiles_auto::auto_conventionnelle(famille)
                            }
                        };
                        ModeGestion::Automatique(AutoVolet {
                            derogation,
                            detecteur: pm.detecteur_presence?,
                            p_derog: parts.p_derog,
                            ecl_lim_lux: a.eclairement_limite_lux,
                            top_lim_h_c: a.temperature_limite_haute_c?,
                            top_lim_b_c: a.temperature_limite_basse_c?,
                            vent_lim_m_s: a.vent_limite_m_s,
                            horloge: horloge_resolue(a)?,
                            matrice,
                        })
                    }
                }
            }
            TypePm::StoreVenitien => {
                let manuelle = thbce_data::protections_mobiles_auto::matrice_sv(famille);
                let auto = if pm.gestion_sv?.est_automatique() {
                    let a = pm.automatisme.as_ref()?;
                    let matrice = match &a.matrice_sv {
                        Some(m) => matrice_auto_sv_saisie(m),
                        None => {
                            signaler(
                                "protection_mobile.automatisme.matrice_sv",
                                "figures 46 et 47",
                            );
                            *thbce_data::protections_mobiles_auto::auto_sv_conventionnelle(famille)
                        }
                    };
                    Some(AutoSv {
                        p_derog: parts.p_derog,
                        ecl_lim_lux: a.eclairement_limite_lux,
                        vent_lim_m_s: a.vent_limite_m_s,
                        horloge: horloge_resolue(a)?,
                        matrice,
                    })
                } else {
                    None
                };
                ModeGestion::Venitien(GestionSvResolue { manuelle, auto })
            }
        };
        Some(Self {
            type_pm: pm.type_pm,
            exterieure: pm.exterieure,
            pm2: pm.seconde_protection,
            p_occ: parts.p_occ,
            mode,
            th_d,
        })
    }

    /// Ligne d'été chaud : `Topj-1max ≥ Topj-1lim_manu` (26,5 °C, éq. 201 à 204).
    fn ligne(c: &ConditionsGpm) -> (LigneGpm, bool) {
        let chaud = c.top_j1_max_c >= TOP_J1_LIM_MANU_C;
        let ligne = match c.saison {
            SaisonGpm::Hiver => LigneGpm::Hiver,
            SaisonGpm::MiSaison => LigneGpm::MiSaison,
            SaisonGpm::Ete if chaud => LigneGpm::EteChaud,
            SaisonGpm::Ete => LigneGpm::EteFrais,
        };
        (ligne, chaud)
    }

    /// Lignes de la matrice de gestion manuelle (volet ou store enroulable), valeurs de Th-D
    /// comprises.
    fn lignes(&self, c: &ConditionsGpm) -> (LigneOccupation, LigneInoccupation, bool) {
        let (ligne, chaud) = Self::ligne(c);
        let i = ligne as usize;
        let (occ, inocc, motorisee) = match &self.mode {
            ModeGestion::Manuelle { motorisee, matrice } => {
                let m = matrice;
                if *motorisee {
                    (m.occupation_motorisee[i], m.inoccupation_motorisee[i], true)
                } else {
                    (
                        m.occupation_non_motorisee[i],
                        m.inoccupation_non_motorisee[i],
                        false,
                    )
                }
            }
            // Dérogation de la gestion automatique : lignes `type-GPM-manu` = 1 (avec détecteur
            // de présence) ou 2 (sans), éq. (180). Seule l'occupation sert (`partGPM = 0` en
            // inoccupation) ; Th-D traite la dérogation comme une gestion non motorisée
            // (`Type_GPM ≠ 3`) et sans effet du détecteur (tableau 34 et §5.9.3.6).
            ModeGestion::Automatique(a) => {
                let d = if a.detecteur {
                    a.derogation.avec_detecteur[i]
                } else {
                    a.derogation.sans_detecteur[i]
                };
                (
                    d,
                    LigneInoccupation {
                        r_prot_jour: 0.0,
                        r_prot_nuit: 0.0,
                    },
                    false,
                )
            }
            ModeGestion::Venitien(_) => unreachable_ligne(),
        };
        let mut occ = occ;
        let mut inocc = inocc;
        // Th-BCE 2020 §5.9.3.6, éq. (202) à (204) et tableau 34 (colonne de la famille d'usages) : en
        // Th-D, en période de confort adaptatif et quand Topj-1max ≥ Topj-1lim_manu, seuls les
        // ratios changent ; les seuils d'éclairement sont conservés.
        let th_d_chaud = c.confort_adaptatif_th_d && chaud;
        if th_d_chaud {
            let plancher = if motorisee {
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

    /// Position des protections mobiles de la baie au pas de temps, sans mémoire d'un pas à
    /// l'autre : équivaut à [`Self::protections_avec_etat`] avec l'état initial. Pour une gestion
    /// automatique de volet ou de store enroulable, l'hystérésis part de la branche basse.
    pub fn protections(&self, c: &ConditionsGpm) -> Protections {
        self.protections_avec_etat(c, &mut EtatGpm::default())
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
    ///
    /// Gestion automatique (éq. 179, 186, 187, 190, 191) : `partGPM = Pocc·Pderog` en occupation
    /// reçoit la gestion manuelle par dérogation (lignes `type-GPM-manu` 1 et 2, « Jour » de
    /// l'horloge), le reste de la baie la matrice automatique avec hystérésis. Stores vénitiens :
    /// éq. (194) à (197).
    pub fn protections_avec_etat(&self, c: &ConditionsGpm, etat: &mut EtatGpm) -> Protections {
        match &self.mode {
            ModeGestion::Venitien(sv) => self.protections_sv(sv, c),
            ModeGestion::Automatique(a) => self.protections_auto(a, c, etat),
            ModeGestion::Manuelle { .. } => self.protections_manuelles(c),
        }
    }

    fn protections_manuelles(&self, c: &ConditionsGpm) -> Protections {
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

    /// Volet ou store enroulable en gestion automatique avec dérogation.
    ///
    /// Th-BCE 2020 §5.9.3.2.4.2, éq. (179) : `partGPM = Pocc·Pderog` en occupation, 0 sinon ;
    /// §5.9.3.4.1.1, éq. (186) et (187) (volets), §5.9.3.4.2.1, éq. (190) et (191) (stores
    /// enroulables).
    fn protections_auto(
        &self,
        a: &AutoVolet,
        c: &ConditionsGpm,
        etat: &mut EtatGpm,
    ) -> Protections {
        let (occ, _, th_d_chaud) = self.lignes(c);
        let pm2_active = self.pm2 && !th_d_chaud;
        // « Jour » : horloge de l'automatisme (éq. 182).
        let jour = a.horloge.jour(c.etp_lux, c.saison, c.hleg);
        // Hystérésis de la figure 31 sur θop,fin(h−1).
        etat.mettre_a_jour(c.theta_op_fin_c, a.top_lim_h_c, a.top_lim_b_c);
        let vent_se = self.type_pm == TypePm::StoreEnroulable && self.exterieure;
        let r_hors_part = if vent_se && a.vent_lim_m_s.is_some_and(|v| c.v_vent_c_m_s >= v) {
            0.0
        } else {
            let ligne = if c.i_occ_zone {
                &a.matrice.occupation[c.saison.indice()]
            } else {
                &a.matrice.inoccupation[c.saison.indice()]
            };
            let cellule = if !jour {
                ligne.nuit
            } else if c.etp_lux >= a.ecl_lim_lux {
                ligne.jour_au_dessus
            } else {
                ligne.jour_sous_seuil
            };
            if etat.branche_haute {
                cellule.haute
            } else {
                cellule.basse
            }
        };
        let automatique = |part_gpm, r_part, r_prot2| Protections {
            part_gpm,
            r_prot1_part_gpm: r_part,
            r_prot1_hors_part_gpm: r_hors_part,
            r_prot2,
            gestion: GestionProtection::Automatique,
        };
        if !c.i_occ_zone {
            return automatique(0.0, 0.0, 0.0);
        }
        let r_prot2 = if pm2_active {
            r_prot_selon_eclairement(c.etp_lux, R_PROT2_OCC0, occ.ecl2_lux)
        } else {
            0.0
        };
        let part = self.p_occ * a.p_derog;
        if vent_se && c.v_vent_c_m_s >= V_VENT_LIM_MANU_SE_M_S {
            return automatique(part, 0.0, r_prot2);
        }
        let dans_part = if !jour {
            occ.r_prot_nuit
        } else if pm2_active {
            occ.r_prot_avec_pm2
        } else {
            r_prot_selon_eclairement(c.etp_lux, occ.r_prot0, occ.ecl1_lux)
        };
        automatique(part, dans_part, r_prot2)
    }

    /// Store vénitien (gestion manuelle ou automatique).
    ///
    /// Th-BCE 2020 §5.9.3.4.3 : gestion automatique, éq. (194) et (195) ; gestion manuelle,
    /// éq. (196) et (197) ; seconde protection, éq. (198) ; Th-D, éq. (201) et (205), tableau 34.
    fn protections_sv(&self, sv: &GestionSvResolue, c: &ConditionsGpm) -> Protections {
        use thbce_data::protections_mobiles_auto::{
            ECL_LIM_MANU_SV_LUX, TH_D_R_PROT_INOCC_SV, TH_D_R_PROT_OCC_SV, V_VENT_LIM_MANU_SV_M_S,
        };
        let s = c.saison.indice();
        let (_, chaud) = Self::ligne(c);
        let th_d_conf = c.confort_adaptatif_th_d;
        let pm2_active = self.pm2 && !(th_d_conf && chaud);
        // « Jour » : horloge en gestion automatique (éq. 184), éclairement et occupation en
        // gestion manuelle (éq. 185).
        let jour = match &sv.auto {
            Some(a) => a.horloge.jour(c.etp_lux, c.saison, c.hleg),
            None => jour_gestion_manuelle(c.i_occ_gpm, c.etp_lux),
        };
        let au_dessus = c.etp_lux >= ECL_LIM_MANU_SV_LUX;
        let m = &sv.manuelle;
        // Ratio de la gestion manuelle en occupation (partie `partGPM`), tableau 34 en Th-D.
        let occ_manuel = {
            let l = &m.occupation[s];
            if !jour {
                l.r_prot_nuit
            } else if au_dessus {
                if th_d_conf {
                    TH_D_R_PROT_OCC_SV
                } else {
                    l.r_prot_au_dessus
                }
            } else {
                l.r_prot_sous_seuil
            }
        };
        // Ratio de la gestion manuelle en inoccupation.
        let inocc_manuel = {
            let l = &m.inoccupation[s];
            if !jour {
                l.r_prot_nuit
            } else if th_d_conf && au_dessus {
                TH_D_R_PROT_INOCC_SV
            } else {
                l.r_prot_jour
            }
        };
        let r_prot2 = if c.i_occ_zone && pm2_active {
            r_prot_selon_eclairement(c.etp_lux, R_PROT2_OCC0, m.occupation[s].ecl2_lux)
        } else {
            0.0
        };
        let vent_manu = self.exterieure && c.v_vent_c_m_s >= V_VENT_LIM_MANU_SV_M_S;
        match &sv.auto {
            None => {
                if !c.i_occ_zone {
                    return Protections {
                        part_gpm: 0.0,
                        r_prot1_part_gpm: 0.0,
                        r_prot1_hors_part_gpm: inocc_manuel,
                        r_prot2: 0.0,
                        gestion: GestionProtection::Manuelle,
                    };
                }
                let (dans, hors) = if vent_manu {
                    (0.0, 0.0)
                } else {
                    (occ_manuel, inocc_manuel)
                };
                Protections {
                    part_gpm: self.p_occ,
                    r_prot1_part_gpm: dans,
                    r_prot1_hors_part_gpm: hors,
                    r_prot2,
                    gestion: GestionProtection::Manuelle,
                }
            }
            Some(a) => {
                let ligne = if c.i_occ_zone {
                    &a.matrice.occupation[s]
                } else {
                    &a.matrice.inoccupation[s]
                };
                let hors = if self.exterieure && a.vent_lim_m_s.is_some_and(|v| c.v_vent_c_m_s >= v)
                {
                    0.0
                } else if !jour {
                    ligne.nuit
                } else if c.etp_lux >= a.ecl_lim_lux {
                    ligne.jour_au_dessus
                } else {
                    ligne.jour_sous_seuil
                };
                if !c.i_occ_zone {
                    return Protections {
                        part_gpm: 0.0,
                        r_prot1_part_gpm: 0.0,
                        r_prot1_hors_part_gpm: hors,
                        r_prot2: 0.0,
                        gestion: GestionProtection::Automatique,
                    };
                }
                Protections {
                    part_gpm: self.p_occ * a.p_derog,
                    r_prot1_part_gpm: if vent_manu { 0.0 } else { occ_manuel },
                    r_prot1_hors_part_gpm: hors,
                    r_prot2,
                    gestion: GestionProtection::Automatique,
                }
            }
        }
    }
}

/// Une gestion de store vénitien n'a pas de ligne de matrice de volet.
fn unreachable_ligne() -> (LigneOccupation, LigneInoccupation, bool) {
    (
        LigneOccupation {
            r_prot0: 0.0,
            ecl1_lux: 1.0,
            r_prot_avec_pm2: 0.0,
            ecl2_lux: 1.0,
            r_prot_nuit: 0.0,
        },
        LigneInoccupation {
            r_prot_jour: 0.0,
            r_prot_nuit: 0.0,
        },
        false,
    )
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
