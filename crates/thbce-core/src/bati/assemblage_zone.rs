// SPDX-License-Identifier: GPL-3.0-or-later
//! Assemblage des composants d'une zone : fiche `S2_BAT-assemblage_zone` (Th-BCE 2020 §5.5,
//! figure 18), pour le calcul du Bbio (mode Th-B) et pour le calcul du confort d'été (mode
//! Th-D), qui emploie le même assemblage (`docs/questions/Q040.md`).
//!
//! À chaque pas de temps : climat et scénarios de la zone ; bilan aéraulique de la zone (§5.6)
//! avec les débits et l'air soufflé du système de ventilation — conventionnel en Th-B (§6.1 et
//! §6.2), celui du projet en Th-D (§6.2 à 6.5, lecture Q42) — et les températures des groupes au
//! pas précédent ; puis comportement thermique de chaque groupe (§5.7), dans
//! l'ordre de la figure 20 : les groupes qui cèdent de l'air au hall, le hall, puis les groupes
//! qui en reçoivent.
//!
//! Périmètre : calculs Th-B et Th-D des usages d'habitation ; hall fictif (aucun groupe de
//! circulation saisi). Les espaces tampons sont calculés au niveau du bâtiment, avant les zones
//! ([`super::assemblage_batiment`]) ; la zone reçoit leurs coefficients.

use crate::chauffage_refroidissement::relances::HORIZON_RELANCE;
use crate::chauffage_refroidissement::relances::{en_relance, ConditionsRelance};
use crate::climat::calendrier::{jour, nouveau_jour_legal};
use crate::climat::environnement_proche::{ventc_m_s, Soleil};
use crate::climat::{ClimatExterieur, ConfortAdaptatif};
use crate::conventions::{ModeCalcul, NB_HEURES_AN};
use crate::entree::{Catalogue, Zone};
use crate::erreur::Erreur;
use crate::scenarios::ScenariosZone;
use crate::ventilation::debits_air_cep::{
    flux_aeration, flux_entrants, hall_fictif, trav_zone_conventionnel, EtatGroupe, Exterieur,
    ZoneAeraulique,
};
use crate::ventilation::ven_bbio::{omega_air_souffle_kg_kg, t_air_souffle_c};
use crate::ventilation::ventilation_projet::{
    AirSouffle, EtatGroupeVentilation, ModeActif, PasVentilation, VentilationProjet,
};

use super::assemblage_groupe::{
    nbh_occ_ref, ConditionsPas, GroupeHoraire, LectureRelanceCta, Lectures, SortiesPasGroupe,
    VentilationThD, THETA_INITIALE_C,
};
use super::espace_tampon::espace_tampon::CoefficientsEt;
use super::ConditionsExterieures;

/// Zone d'habitation prête pour un calcul annuel au pas horaire, en mode Th-B ou Th-D.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneHoraire {
    /// Identifiant de la zone.
    pub id: String,
    /// Mode de calcul : Th-B (Bbio) ou Th-D (confort d'été).
    pub mode: ModeCalcul,
    /// Scénarios conventionnels de la zone (§4.1).
    pub scenarios: ScenariosZone,
    /// Modèle aéraulique de la zone (§5.6).
    pub aeraulique: ZoneAeraulique,
    /// Groupes de la zone.
    pub groupes: Vec<GroupeHoraire>,
    /// Sorties de chaque groupe au dernier pas de temps calculé.
    pub sorties: Vec<Option<SortiesPasGroupe>>,
    /// Système de ventilation du projet (§6.2 à 6.5), employé hors calcul du Bbio ; `None` en
    /// Th-B, où le système conventionnel de la fiche 6.1 s'applique.
    pub ventilation: Option<VentilationProjet>,
    /// Sorties du système de ventilation du projet au dernier pas de temps calculé.
    pub sorties_ventilation: PasVentilation,
    /// Mode actif de la centrale au dernier pas de temps calculé (§6.3.3.5).
    pub mode_actif: ModeActif,
    /// Cumuls annuels du système de ventilation du projet.
    pub cumuls_ventilation: CumulsVentilation,
    etats: Vec<EtatGroupe>,
    /// État des groupes au pas de temps en cours vu par le mode actif de la centrale.
    etats_mode_actif: Vec<EtatGroupeVentilation>,
    air_souffle: Vec<AirSouffle>,
    etats_ventilation: Vec<EtatGroupeVentilation>,
    /// « Pib(h−1) » : pression au niveau du plancher de la zone au pas précédent, en Pa (§6.6.3.1.5,
    /// éq. 698) ; 0 au premier pas.
    p_ib_prec_pa: f64,
}

/// Cumuls annuels du système de ventilation du projet d'une zone.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CumulsVentilation {
    /// Nombre de pas de temps où l'échangeur est contourné par son bypass (éq. 494).
    pub nbh_bypass: u32,
    /// Énergie fournie pour la sécurité antigel, en Wh (éq. 508).
    pub w_ag_wh: f64,
    /// Énergie fournie par la batterie de préchauffage, en Wh (éq. 523).
    pub q_prechaud_wh: f64,
    /// Énergie fournie par la batterie de prérefroidissement, en Wh, négative (éq. 545).
    pub q_prefroid_wh: f64,
    /// Nombre de pas de temps où la centrale fonctionne en mode actif (§6.3.3.5).
    pub nbh_mode_actif: u32,
    /// Nombre de pas de temps où la batterie régulée sur la charge est sous-dimensionnée (éq.
    /// 597 et 605).
    pub nbh_sous_dimensionnement: u32,
    /// « Nbsousdim_DAV_Ch(h) » et « Nbsousdim_DAV_Cr(h) » : heures consécutives de
    /// sous-dimensionnement en chauffage et en refroidissement (éq. 597 et 605).
    pub nb_sousdim: [u32; 2],
    /// Plus longue série d'heures consécutives de sous-dimensionnement, en chauffage ou en
    /// refroidissement (éq. 597 et 605).
    pub nbh_sous_dimensionnement_consecutives: u32,
    /// Énergie fournie par la batterie chaude régulée sur la charge, en Wh (éq. 599).
    pub q_chaud_wh: f64,
    /// Énergie fournie par la batterie froide régulée sur la charge, latente comprise, en Wh,
    /// négative (éq. 611).
    pub q_froid_wh: f64,
    /// Énergie de l'humidificateur d'hiver à vapeur, en Wh (éq. 529).
    pub q_hr_wh: f64,
    /// Consommation de la pompe de l'humidificateur d'hiver, en Wh (éq. 568).
    pub w_aux_chaudhr_wh: f64,
    /// Consommation de la pompe du rafraîchissement évaporatif, en Wh (éq. 566 et 617).
    pub w_aux_adiab_wh: f64,
    /// Nombre de pas de temps où le rafraîchissement évaporatif est autorisé (§6.3.3.4.2).
    pub nbh_adiab: u32,
    /// Nombre de pas de temps où la surventilation mécanique est en service (éq. 766).
    pub nbh_rafnoc: u32,
    /// Consommation de la pompe du puits hydraulique, en Wh (éq. 748, 749 et 617).
    pub w_puits_wh: f64,
    /// Nombre de pas de temps où l'air neuf traverse le puits climatique (non contourné).
    pub nbh_puits: u32,
    /// Énergie échangée dans le puits climatique en préchauffage, en Wh (positive).
    pub q_puits_chaud_wh: f64,
    /// Énergie échangée dans le puits climatique en prérafraîchissement, en Wh (négative).
    pub q_puits_froid_wh: f64,
}

impl ZoneHoraire {
    /// Prépare une zone d'habitation validée, avec les lectures retenues du texte.
    ///
    /// `mode` : Th-B ou Th-D ; `alt_m` est l'altitude du projet ; `omega_initiale_kg_kg`,
    /// l'humidité de l'air des groupes avant le premier pas de temps (celle de l'air extérieur
    /// au premier pas de temps, décision Q39 point 4).
    pub fn nouvelle(
        zone: &Zone,
        catalogue: &Catalogue,
        mode: ModeCalcul,
        alt_m: f64,
        omega_initiale_kg_kg: f64,
    ) -> Result<Self, Erreur> {
        Self::nouvelle_selon(
            zone,
            catalogue,
            mode,
            Lectures::default(),
            alt_m,
            omega_initiale_kg_kg,
        )
    }

    /// Prépare une zone d'habitation validée pour des lectures données du texte (mesure des
    /// sensibilités).
    pub fn nouvelle_selon(
        zone: &Zone,
        catalogue: &Catalogue,
        mode: ModeCalcul,
        lectures: Lectures,
        alt_m: f64,
        omega_initiale_kg_kg: f64,
    ) -> Result<Self, Erreur> {
        Self::nouvelle_dans_batiment(
            zone,
            catalogue,
            mode,
            lectures,
            alt_m,
            omega_initiale_kg_kg,
            &[],
        )
    }

    /// Prépare une zone d'un bâtiment dont `ids_espaces_tampons` donne les identifiants des
    /// espaces tampons, dans l'ordre (voir [`super::assemblage_batiment`]).
    pub fn nouvelle_dans_batiment(
        zone: &Zone,
        catalogue: &Catalogue,
        mode: ModeCalcul,
        lectures: Lectures,
        alt_m: f64,
        omega_initiale_kg_kg: f64,
        ids_espaces_tampons: &[String],
    ) -> Result<Self, Erreur> {
        let a_gr: Vec<f64> = zone
            .groupes
            .iter()
            .map(|g| g.su_m2.unwrap_or(0.0))
            .collect();
        let scenarios = ScenariosZone::nouveau(zone.usage, &a_gr, zone.nb_logements.unwrap_or(1))?;
        let nbh_occ_ref = nbh_occ_ref(scenarios.usage().occupation.grille_hebdo());
        let q4pa = zone.q4pa_surf_m3_h_m2.unwrap_or(0.0);
        let mut groupes: Vec<GroupeHoraire> = zone
            .groupes
            .iter()
            .map(|g| {
                GroupeHoraire::nouveau_dans_batiment(
                    g,
                    catalogue,
                    zone.usage,
                    mode,
                    lectures,
                    q4pa,
                    nbh_occ_ref,
                    omega_initiale_kg_kg,
                    ids_espaces_tampons,
                )
            })
            .collect();
        // Ventilation : système conventionnel de la fiche 6.1 en Th-B (§6.1.3, décision Q36) ;
        // système du projet en Th-C et en Th-D (lecture Q42). La variante
        // `VentilationThD::ConventionnelleBbio` ne touche que le Th-D : en Th-C, le système du
        // projet reste employé (sans lui, une centrale double flux hors habitation n'aurait pas
        // de relevé de consommation et ses ventilateurs compteraient 0 kWh).
        let ventilation = (mode == ModeCalcul::ThC
            || (mode == ModeCalcul::ThD && lectures.ventilation_th_d == VentilationThD::Projet))
            .then(|| VentilationProjet::nouvelle(zone, THETA_INITIALE_C))
            .flatten();
        if let Some(v) = &ventilation {
            for (ig, g) in groupes.iter_mut().enumerate() {
                g.debits = v.debits_groupe(ig);
            }
        }
        let aerauliques: Vec<_> = groupes.iter().map(|g| g.aeraulique).collect();
        let aeraulique = ZoneAeraulique::nouvelle(
            &aerauliques,
            // Th-BCE 2020 §5.6.3.1.3, tableau 22 : conventionnel, sauf en logement collectif.
            trav_zone_conventionnel(zone.usage.numero())
                .or(zone.trav_zone)
                .unwrap_or(true),
            // Éq. (138) : pas de perméabilité verticale en usage d'habitation ou d'hébergement.
            zone.usage.i_hebergement(),
            alt_m,
            zone.h_plancher_m.unwrap_or(0.0),
            zone.h_zone_m.unwrap_or(0.0),
        );
        let nb = groupes.len();
        Ok(Self {
            id: zone.id.clone(),
            mode,
            scenarios,
            aeraulique,
            groupes,
            sorties: vec![None; nb],
            ventilation,
            sorties_ventilation: PasVentilation::default(),
            mode_actif: ModeActif::default(),
            etats_mode_actif: Vec::new(),
            cumuls_ventilation: CumulsVentilation::default(),
            etats: Vec::with_capacity(nb),
            air_souffle: vec![AirSouffle::default(); nb],
            etats_ventilation: Vec::with_capacity(nb),
            p_ib_prec_pa: 0.0,
        })
    }

    /// Calcule le pas de temps `h` de la zone dans son mode de calcul.
    ///
    /// `climat` est le climat du site pour le jeu météo du mode (« Th-BC » en Th-B, « Th-D » en
    /// Th-D) et `confort` la moyenne glissante de la température extérieure qui s'en déduit.
    pub fn pas(
        &mut self,
        h: usize,
        climat: &ClimatExterieur,
        confort: &ConfortAdaptatif,
    ) -> Result<(), Erreur> {
        self.pas_avec_espaces_tampons(h, climat, confort, &[])
    }

    /// Air soufflé dans chaque groupe au dernier pas de temps calculé (température et
    /// humidité), par le système conventionnel du Bbio en Th-B et par le système du projet
    /// sinon.
    pub fn air_souffle(&self) -> &[AirSouffle] {
        &self.air_souffle
    }

    /// « iocc_zone(h) » : indicateur d'occupation de la zone au pas de temps (§4.1.3).
    pub fn i_occ_zone(&self, h: usize) -> f64 {
        self.scenarios.evaluer(h).i_occ_zone
    }

    /// Calcule le pas de temps `h` de la zone, les coefficients des espaces tampons du
    /// bâtiment au même pas de temps étant connus (§5.2, figure 15 : les espaces tampons sont
    /// calculés avant les zones).
    pub fn pas_avec_espaces_tampons(
        &mut self,
        h: usize,
        climat: &ClimatExterieur,
        confort: &ConfortAdaptatif,
        espaces_tampons: &[CoefficientsEt],
    ) -> Result<(), Erreur> {
        let ext = conditions_exterieures(climat, confort, h, self.mode);
        let scenarios = self.scenarios.evaluer(h);
        let c = ConditionsPas {
            ext,
            omega_e_kg_kg: climat.we_kg_kg[h],
            v_vent_c_m_s: ventc_m_s(climat.vent_m_s[h]),
            // §4.5.3.2.1 : θrm est calculée au premier pas de temps du jour (IHJ = 1).
            theta_rm_c: confort.theta_rm_c.get(jour(h)).copied().unwrap_or(0.0),
            nouveau_jour: nouveau_jour_legal(h),
            scenarios,
            theta_ii_fr_plus_c: self.scenarios.usage().consignes.fr_normal_c,
            theta_ii_ch_plus_c: self.scenarios.usage().consignes.ch_normal_c,
            theta_ext_base_c: climat.base_ext_c,
            p_ch_avenir: [0.0; HORIZON_RELANCE],
            p_fr_avenir: [0.0; HORIZON_RELANCE],
            i_occ_zone_h1: h > 0 && self.scenarios.evaluer(h - 1).i_occ_zone > 0.0,
            theta_cw_c: climat.teau_c[h],
        };
        // §8.5 : indicateurs de consigne des pas de temps à venir, en calcul des consommations.
        // Au-delà de la dernière heure de l'année, le calendrier reprend au début (lecture Q43).
        let mut c = c;
        if self.mode == ModeCalcul::ThC {
            for t in 0..HORIZON_RELANCE {
                let s = if t == 0 {
                    scenarios
                } else {
                    self.scenarios.evaluer((h + t) % NB_HEURES_AN)
                };
                c.p_ch_avenir[t] = s.p_ch;
                c.p_fr_avenir[t] = s.p_fr;
            }
        }
        let exterieur = Exterieur {
            theta_ext_c: ext.te_c,
            omega_e_kg_kg: c.omega_e_kg_kg,
            v_ventc_m_s: c.v_vent_c_m_s,
        };

        // §5.5.3, étape 1 : bilan aéraulique de la zone, avec les températures et les humidités
        // des groupes à la fin du pas précédent. Air soufflé : en Th-B, par le système
        // conventionnel du Bbio, après l'échangeur (§6.1.3) ; sinon, par le système du projet
        // (§6.3.3.4 et §6.5.3.4), calculé avant les groupes (§6.3.3.3, « première phase »).
        // §6.2.3.2.2.1 : hors habitation, les débits des bouches suivent l'indicateur de
        // ventilation de la zone (éq. 400 à 402 ; débits conventionnels du §6.2.3.1.3 en Bbio).
        let i_vent = scenarios.i_vent > 0.0;
        // État des groupes au pas précédent vu par le système du projet ; une centrale de
        // traitement d'air en déduit son régime de débits (§6.3.3.4.3.2, lecture Q90).
        if let Some(v) = &mut self.ventilation {
            self.etats_ventilation.clear();
            let mode = self.mode;
            self.etats_ventilation
                .extend(self.groupes.iter().zip(&self.sorties).map(|(g, s)| {
                    let sy = g.systemes.as_ref().map(|sy| &sy.sorties);
                    EtatGroupeVentilation {
                        theta_i_fin_c: g.etat.fin.theta_i_c,
                        omega_i_fin_kg_kg: g.etat.omega_i_fin_kg_kg,
                        // Q42 point 9 : saison effective de chauffage (fiche 8.4) en calcul des
                        // consommations ; autorisation propre dans les autres modes.
                        saison_de_chauffage: if mode == ModeCalcul::ThC {
                            g.etat.aut_eff.0
                        } else {
                            g.etat.saisons.aut_ch
                        },
                        // Éq. (410) : saison effective d'un groupe climatisé, en calcul des
                        // consommations seulement (aucun refroidissement en Th-D, §13.5.3.1).
                        saison_de_refroidissement: mode == ModeCalcul::ThC
                            && g.i_clim
                            && g.etat.aut_eff.1,
                        relance: match (g.systemes.as_ref(), g.lectures.relance_cta) {
                            // Q90 point 1, variante écartée : relance du pas en cours anticipée.
                            (Some(x), LectureRelanceCta::Anticipee) if mode == ModeCalcul::ThC => {
                                let mut r = x.relance;
                                en_relance(&r.pas(&ConditionsRelance {
                                    aut_ch: g.etat.aut_eff.0,
                                    aut_fr: g.etat.aut_eff.1,
                                    p_ch: c.p_ch_avenir,
                                    p_fr: c.p_fr_avenir,
                                    theta_ext_c: ext.te_c,
                                    theta_ext_base_c: c.theta_ext_base_c,
                                    theta_ii_ch_plus_c: c.theta_ii_ch_plus_c,
                                    theta_ii_fr_plus_c: c.theta_ii_fr_plus_c,
                                }))
                            }
                            _ => sy.is_some_and(|x| x.relance),
                        },
                        besoin_ch: s.is_some_and(|x| x.q_sys_ch_wh > 0.0),
                        besoin_fr: s.is_some_and(|x| x.q_sys_fr_wh < 0.0),
                        clim: g.i_clim,
                        aut_fr_propre: g.etat.saisons.aut_fr_passif,
                        theta_op_fin_c: g.etat.fin.theta_op_c,
                    }
                }));
            // §6.8, 6.10 et 6.11 : puits climatique (sol, bypass, `θan` avec les débits du pas
            // précédent).
            v.preparer_puits(h, &climat.te_c, ext.te_c, &self.etats_ventilation);
            // §6.12 : surventilation mécanique, avec l'air neuf à `θan(h)` (la température
            // extérieure sans puits).
            let theta_an_c = v.theta_an_c(ext.te_c);
            v.reguler_surventilation(h, theta_an_c, &self.etats_ventilation);
            v.preparer_pas(i_vent, &self.etats_ventilation);
            // §6.6 : ventilation naturelle ou hybride, avec la pression `Pib` du pas précédent.
            v.pas_naturelle(
                h,
                ext.te_c,
                c.omega_e_kg_kg,
                c.v_vent_c_m_s,
                self.p_ib_prec_pa,
                &self.etats_ventilation,
            );
        }
        for (ig, g) in self.groupes.iter_mut().enumerate() {
            match (&self.ventilation, g.debits_bbio_non_residentiel) {
                (Some(v), _)
                    if v.selon_occupation() || v.surventilation.is_some() || v.est_naturelle() =>
                {
                    g.debits = v.debits_groupe_pas(ig);
                }
                (None, Some((occ, inocc))) => g.debits = if i_vent { occ } else { inocc },
                _ => {}
            }
        }
        if let Some(v) = &mut self.ventilation {
            self.sorties_ventilation = v.pas(
                ext.te_c,
                c.omega_e_kg_kg,
                &self.etats_ventilation,
                &mut self.air_souffle,
                i_vent,
            );
            self.cumuls_ventilation.nbh_bypass += u32::from(self.sorties_ventilation.bypass);
            self.cumuls_ventilation.w_ag_wh += self.sorties_ventilation.w_ag_wh;
            self.cumuls_ventilation.q_prechaud_wh += self.sorties_ventilation.q_prechaud_wh;
            self.cumuls_ventilation.q_prefroid_wh += self.sorties_ventilation.q_prefroid_wh;
            let sv = self.sorties_ventilation;
            let cv = &mut self.cumuls_ventilation;
            cv.q_hr_wh += sv.q_hr_wh;
            cv.w_aux_chaudhr_wh += sv.w_aux_chaudhr_wh;
            cv.w_aux_adiab_wh += sv.w_aux_adiab_wh;
            cv.nbh_adiab += u32::from(sv.adiab);
            cv.nbh_rafnoc += u32::from(sv.rafnoc);
            cv.nbh_puits += u32::from(sv.puits_actif);
            cv.w_puits_wh += sv.w_puits_wh;
            cv.q_puits_chaud_wh += sv.q_puits_wh.max(0.0);
            cv.q_puits_froid_wh += sv.q_puits_wh.min(0.0);
        } else {
            for (a, g) in self.air_souffle.iter_mut().zip(&self.groupes) {
                *a = AirSouffle {
                    t_air_souffle_c: t_air_souffle_c(ext.te_c, g.etat.fin.theta_i_c),
                    omega_air_souffle_kg_kg: omega_air_souffle_kg_kg(c.omega_e_kg_kg),
                };
            }
        }
        // §6.7.3.1, éq. (715) : en aération, l'air fourni par la fenêtre d'un groupe dont l'air
        // neuf transite par un espace tampon est à la température de sortie de cet espace ;
        // sinon à la température extérieure (éq. 716), déjà posée par le système.
        let est_aeration = self.ventilation.as_ref().is_some_and(|v| v.est_aeration());
        if est_aeration {
            for (g, a) in self.groupes.iter().zip(self.air_souffle.iter_mut()) {
                if let Some(e) = g
                    .aeration_espace_tampon
                    .and_then(|i| espaces_tampons.get(i))
                {
                    a.t_air_souffle_c = e.theta_et_c;
                }
            }
        }
        self.etats.clear();
        self.etats
            .extend(self.groupes.iter().zip(&self.air_souffle).map(|(g, a)| {
                EtatGroupe {
                    theta_i_fin_c: g.etat.fin.theta_i_c,
                    omega_i_fin_kg_kg: g.etat.omega_i_fin_kg_kg,
                    q_spec_repris_m3_h: g.debits.q_rep_m3_h,
                    q_spec_souffle_m3_h: g.debits.q_souffle_m3_h,
                    t_air_souffle_c: a.t_air_souffle_c,
                    omega_air_souffle_kg_kg: a.omega_air_souffle_kg_kg,
                    theta_an_c: g
                        .entree_air_espace_tampon
                        .and_then(|i| espaces_tampons.get(i))
                        .map(|e| e.theta_et_c),
                    aeration: est_aeration,
                }
            }));
        let bilan = self
            .aeraulique
            .bilan(&exterieur, &self.etats)
            .ok_or_else(|| {
                Erreur::Calcul(format!(
                    "zone « {} », pas de temps {h} : le bilan aéraulique n'a pas de solution \
                     (Th-BCE 2020 §5.6.3.5, éq. 167)",
                    self.id
                ))
            })?;
        // Retenue pour la ventilation naturelle ou hybride du pas suivant (éq. 698).
        self.p_ib_prec_pa = bilan.p_ib_pa;

        // §5.5.3, étape 2, figure 20 : d'abord les groupes qui cèdent de l'air au hall (ou qui
        // n'échangent rien avec lui), puis le hall fictif, enfin les groupes qui en reçoivent.
        // Si aucun groupe ne cède d'air, un débit reçu n'est qu'un résidu du solveur : le
        // groupe est traité à la première passe et ne reçoit rien du hall.
        let un_groupe_cede = bilan.groupes.iter().any(|b| b.q_m_hall_kg_s < 0.0);
        for passe_des_receveurs in [false, true] {
            // Éq. (130) : le hall fictif prend la température et l'humidité des débits que lui
            // cèdent les groupes d'entrée, calculées au pas de temps courant.
            let hall = if passe_des_receveurs {
                hall_fictif(&bilan, &self.etats)
            } else {
                None
            };
            for (g, groupe) in self.groupes.iter_mut().enumerate() {
                let b = &bilan.groupes[g];
                let receveur = un_groupe_cede && b.q_m_hall_kg_s > 0.0;
                if receveur != passe_des_receveurs {
                    continue;
                }
                let air = flux_entrants(b, &exterieur, &self.etats[g], hall);
                groupe.aeration = flux_aeration(b, &self.etats[g]);
                let apports = self.scenarios.apports_groupe(g, &scenarios);
                let sorties = groupe
                    .pas_avec_espaces_tampons(&c, &apports, &air, espaces_tampons)
                    .ok_or_else(|| {
                        Erreur::Calcul(format!(
                            "groupe « {} », pas de temps {h} : le modèle thermique n'a pas de \
                         solution (Th-BCE 2020 §5.21.3.6, éq. 339)",
                            groupe.id
                        ))
                    })?;
                self.sorties[g] = Some(sorties);
                // Le hall voit l'état des groupes d'entrée au pas de temps courant.
                self.etats[g].theta_i_fin_c = groupe.etat.fin.theta_i_c;
                self.etats[g].omega_i_fin_kg_kg = groupe.etat.omega_i_fin_kg_kg;
            }
        }
        // §6.3.3.1.5, seconde phase : mode actif d'une centrale à charge variable, après les
        // groupes, avec les demandes des émetteurs contenus dans ses bouches de soufflage.
        if let Some(v) = &mut self.ventilation {
            self.mode_actif = ModeActif::default();
            if v.cta.as_ref().is_some_and(|c| c.batteries_charge.is_some()) {
                let (mut q_ch, mut q_fr) = (0.0, 0.0);
                self.etats_mode_actif.clear();
                for (g, e) in self.groupes.iter().zip(&self.etats_ventilation) {
                    let sy = g.systemes.as_ref();
                    // Lecture Q95 : θi,moy^g(h) du groupe et son humidité à la fin du pas.
                    self.etats_mode_actif.push(EtatGroupeVentilation {
                        theta_i_fin_c: sy.map_or(g.etat.fin.theta_i_c, |s| s.sorties.theta_i_moy_c),
                        omega_i_fin_kg_kg: g.etat.omega_i_fin_kg_kg,
                        ..*e
                    });
                    for (em, q) in sy
                        .into_iter()
                        .flat_map(|s| s.emetteurs.iter().zip(&s.sorties.q_em_wh))
                    {
                        if em.chaud.is_some_and(|c| c.centrale) {
                            q_ch += q.0;
                        }
                        if em.froid.is_some_and(|f| f.centrale) {
                            q_fr += q.1;
                        }
                    }
                }
                self.mode_actif = v.mode_actif(
                    ext.te_c,
                    c.omega_e_kg_kg,
                    &self.etats_ventilation,
                    &self.etats_mode_actif,
                    i_vent,
                    q_ch,
                    q_fr,
                );
                let m = self.mode_actif;
                let cv = &mut self.cumuls_ventilation;
                cv.nbh_mode_actif += u32::from(m.actif);
                cv.nbh_sous_dimensionnement += u32::from(m.sous_dimensionnee);
                // Éq. (597) et (605) : compteurs d'heures consécutives, remis à zéro à la
                // première heure sans sous-dimensionnement.
                for (k, n) in cv.nb_sousdim.iter_mut().enumerate() {
                    *n = if m.sous_dimensionnee && m.chauffage == (k == 0) {
                        *n + 1
                    } else {
                        0
                    };
                    cv.nbh_sous_dimensionnement_consecutives =
                        cv.nbh_sous_dimensionnement_consecutives.max(*n);
                }
                if m.actif {
                    // §6.3.3.5.5.4 : sécurité antigel recalculée (lecture Q95 point 11).
                    cv.w_ag_wh += m.w_ag_wh - self.sorties_ventilation.w_ag_wh;
                    self.sorties_ventilation.w_ag_wh = m.w_ag_wh;
                    // Éq. (617) : pompe du rafraîchissement évaporatif recalculée.
                    cv.w_aux_adiab_wh += m.w_aux_adiab_wh - self.sorties_ventilation.w_aux_adiab_wh;
                    self.sorties_ventilation.w_aux_adiab_wh = m.w_aux_adiab_wh;
                    // §6.3.3.5.5.1 : pompe du puits hydraulique recalculée.
                    cv.w_puits_wh += m.w_puits_wh - self.sorties_ventilation.w_puits_wh;
                    self.sorties_ventilation.w_puits_wh = m.w_puits_wh;
                }
                cv.q_chaud_wh += m.q_chaud_wh;
                cv.q_froid_wh += m.q_froid_wh;
            }
            v.enregistrer(h);
        }
        Ok(())
    }

    /// Calcule les 8 760 pas de temps de l'année.
    pub fn simuler(
        &mut self,
        climat: &ClimatExterieur,
        confort: &ConfortAdaptatif,
    ) -> Result<(), Erreur> {
        self.simuler_avec(climat, confort, |_, _| {})
    }

    /// Calcule les 8 760 pas de temps de l'année et appelle `apres_le_pas(h, zone)` à la fin de
    /// chacun : les sorties horaires des groupes sont alors dans [`Self::sorties`]. C'est le
    /// point d'accès aux séries horaires.
    pub fn simuler_avec(
        &mut self,
        climat: &ClimatExterieur,
        confort: &ConfortAdaptatif,
        mut apres_le_pas: impl FnMut(usize, &Self),
    ) -> Result<(), Erreur> {
        for h in 0..NB_HEURES_AN {
            self.pas(h, climat, confort)?;
            apres_le_pas(h, self);
        }
        Ok(())
    }
}

/// Conditions extérieures du pas de temps `h` dans un mode de calcul.
pub fn conditions_exterieures(
    climat: &ClimatExterieur,
    confort: &ConfortAdaptatif,
    h: usize,
    mode: ModeCalcul,
) -> ConditionsExterieures {
    ConditionsExterieures {
        soleil: Soleil {
            gamma_rad: climat.gamma_rad[h],
            psi_rad: climat.psi_rad[h],
        },
        idn_w_m2: climat.idn_w_m2[h],
        idi_w_m2: climat.idi_w_m2[h],
        edn_lux: climat.edn_lux[h],
        edi_lux: climat.edi_lux[h],
        te_c: climat.te_c[h],
        teciel_c: climat.teciel_c[h],
        h,
        mode,
        is_conf_adapt: confort.is_conf_adapt(h),
    }
}
