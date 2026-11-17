// SPDX-License-Identifier: GPL-3.0-or-later
//! Assemblage d'un ballon de stockage et de ses générateurs : fiches
//! `S1_GEN_générateur_pour_ballon` (Th-BCE 2020 §9.13) et `S2_GEN_Assemblage constitué d'un ou
//! plusieurs ballon et d'un ou plusieurs générateurs` (§9.14), pour la production d'ECS avec
//! stockage (§9.2).
//!
//! À chaque pas de temps (§9.14.3.2.2) : puisage d'ECS dans le ballon, énergie demandée au
//! générateur de base par son thermostat et injectée dans le ballon, puis de même pour un
//! appoint intégré. L'assemblage est vu par la gestion de la génération comme le premier
//! générateur de la cascade en ECS (§8.17.3.6).
//!
//! Périmètre : assemblages « base seule » (ballon électrique, chauffe-eau thermodynamique) et
//! « base et appoint intégré » en ECS seule ; second ballon séparé, appoint séparé instantané
//! et stockage de chauffage sont refusés par la validation. Lectures : `docs/questions.md`,
//! Q48.

use super::ballon_de_stockage::{Ballon, NB_ZONES, RHO_CW_WH_L_K};
use super::boucle_solaire::{BoucleSolaireHoraire, EntreesBoucle, SortiesBoucle};
use super::gestion_regulation_ballon::{
    active, boucle_puisage, boucle_puisage_appoint_separe, f_p, q_req_sto_wh, Programmation,
    Puisage, THETA_C_ECS_C,
};
use crate::chauffage_refroidissement::calculs_generation::Poste;
use crate::resultats::ValeurParDefaut;

/// Nombre d'heures consécutives sans atteindre la consigne au-delà duquel le texte signale un
/// ballon incohérent avec le projet (§9.14.3.4, éq. 1959).
pub const SEUIL_TEMPERATURE_INSUFFISANTE_H: u32 = 168;

/// Nombre d'heures de report d'énergie à l'issue du puisage au-delà duquel le moteur affiche
/// une alerte (§9.9.3.2.3, éq. 1780).
pub const SEUIL_REPORT_PUISAGE_H: u32 = 24;

/// « zech_ch » : indice (à partir de 0) de la zone de l'échangeur de la boucle de chauffage
/// d'un système solaire combiné, la zone n° 2 du texte (éq. 2129 et 2143).
pub const Z_ECH_CH: usize = 1;

/// « UAhx^bcl_ch(Vtot) » : coefficient d'échange de l'échangeur de la boucle de chauffage d'un
/// système solaire combiné, en W/K (éq. 2118 et tableau 296) : interpolation linéaire entre
/// 1 200 W/K pour 200 L et 12 000 W/K pour 2 000 L.
pub fn ua_hx_bcl_ch_w_k(v_tot_l: f64) -> f64 {
    let (v1, u1, v2, u2) = (200.0, 1200.0, 2000.0, 12_000.0);
    (u2 - u1) / (v2 - v1) * v_tot_l + (u1 * v2 - u2 * v1) / (v2 - v1)
}

/// Boucle de chauffage d'un système solaire combiné (fiches 9.18 à 9.20).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParametresSsc {
    /// Vrai pour un appoint de chauffage raccordé à l'assemblage (« Typeprod_stockage = 5 »,
    /// fiche 9.20) ; faux pour un appoint de chauffage indépendant (« = 4 », fiche 9.19).
    pub raccorde: bool,
    /// « Δθreg^bcl_ch » : réglage de la régulation de la boucle de chauffage, en K (éq. 2120,
    /// appoint indépendant).
    pub delta_theta_reg_bcl_k: f64,
}

/// Régulation d'un générateur de ballon, base ou appoint (§9.10.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegulationGenerateur {
    /// Indice du générateur dans la génération.
    pub generateur: usize,
    /// « typegest ».
    pub programmation: Programmation,
    /// « Δθ » : hystérésis du thermostat, en K.
    pub delta_theta_k: f64,
    /// « zbase » ou « zap » : indice (à partir de 0) de la zone la plus basse de l'échangeur.
    pub z_gen: usize,
    /// « zreg » : indice de la zone de la sonde de régulation.
    pub z_reg: usize,
    /// « hrelech » : hauteur relative de l'échangeur.
    pub h_rel_ech: f64,
}

/// Entrées d'un assemblage à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntreesAssemblage {
    /// « Qreq,ECS(h) » : énergie demandée à l'assemblage, en Wh.
    pub q_req_ecs_wh: f64,
    /// « θentrant,ECS(h) » : température de l'eau entrant dans le ballon, en °C (eau froide ;
    /// éq. 1951 sans réseau bouclé).
    pub theta_entrant_c: f64,
    /// « θdepart,aval,ECS(h) » : température de départ attendue par la distribution, en °C.
    pub theta_depart_c: f64,
    /// « θamb(h) » : température d'ambiance de la génération, en °C.
    pub theta_amb_c: f64,
    /// « hleg » : heure légale du pas de temps.
    pub hleg: u8,
    /// « θe(h) » : température extérieure, en °C (boucle solaire).
    pub theta_ext_c: f64,
    /// « Isr* » : ensoleillement sur le plan des capteurs d'une boucle solaire, masques
    /// compris, en W/m² ; nul sans boucle.
    pub isr_w_m2: f64,
    /// Mois des scénarios, de 0 à 11, pour les cumuls.
    pub mois: usize,
    /// « Qreq_ch(h) » : demande de chauffage transmise à un système solaire combiné, en Wh
    /// (nulle hors saison de chauffage et pour les autres assemblages).
    pub q_req_ch_wh: f64,
    /// « θch_max^gen » : température maximale des réseaux de chauffage de la génération, en °C.
    pub theta_ch_max_c: f64,
    /// « θwm_ch^gen » : température de fonctionnement des générateurs instantanés de chauffage,
    /// en °C (appoint raccordé, éq. 2150).
    pub theta_aval_ch_c: f64,
}

/// Sorties d'un assemblage à un pas de temps, pour l'ensemble des assemblages identiques.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SortiesAssemblage {
    /// « Qfou,ECS(h) », en Wh (éq. 1954).
    pub q_fou_ecs_wh: f64,
    /// « Qrest,ECS(h) », en Wh (éq. 1953).
    pub q_rest_ecs_wh: f64,
    /// « Φpertes » des ballons, en Wh (éq. 1961, avant « Idpos_gen »).
    pub phi_pertes_wh: f64,
    /// « Vp(h) » : volume puisé, en L.
    pub v_p_l: f64,
    /// « Qreq_sto_base » et « Qreq_sto_ap », en Wh.
    pub q_req_sto_wh: [f64; 2],
    /// « Qfou_sto_base » et « Qfou_sto_ap », en Wh.
    pub q_fou_sto_wh: [f64; 2],
    /// « Qsol » : chaleur transférée par une boucle solaire de base, en Wh (comprise dans
    /// `q_fou_sto_wh[0]`).
    pub q_sol_wh: f64,
    /// « Pp » : consommation électrique de la pompe d'une boucle solaire, en Wh (éq. 1972).
    pub w_pompe_wh: f64,
    /// « ΦvcBS » : pertes d'une boucle solaire vers l'intérieur du bâtiment, en Wh (éq. 1973).
    pub phi_vc_bs_wh: f64,
    /// Part de la consommation de la pompe attribuée au chauffage, en Wh (comprise dans
    /// `w_pompe_wh` ; éq. 2135 et 2151).
    pub w_pompe_ch_wh: f64,
    /// « Qpuisé_ch_SSC » : chaleur puisée dans le ballon par la boucle de chauffage, en Wh.
    pub q_puise_ch_wh: f64,
    /// Chauffage fourni par l'appoint raccordé, en Wh (éq. 2150).
    pub q_fou_ch_ap_wh: f64,
    /// « Qrest_ch » : demande de chauffage que l'assemblage n'a pas couverte, en Wh.
    pub q_rest_ch_wh: f64,
}

/// Cumuls annuels d'un assemblage.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CumulsAssemblage {
    /// Énergie fournie à la distribution d'ECS, en Wh.
    pub q_fou_ecs_wh: f64,
    /// Pertes des ballons, en Wh.
    pub phi_pertes_wh: f64,
    /// Énergie fournie aux ballons par la base et par l'appoint, en Wh.
    pub q_fou_sto_wh: [f64; 2],
    /// Volume puisé, en L.
    pub v_p_l: f64,
    /// Plus longue suite d'heures où la zone haute est restée sous la consigne (éq. 1959).
    pub nbh_temp_insuff_max: u32,
    /// Plus basse température de la zone haute en fin de pas de temps, en °C.
    pub theta_haut_min_c: f64,
    /// « nbhreport_e » : nombre d'heures où une énergie est restée à reporter à l'issue de la
    /// boucle de puisage (éq. 1780). Le texte incrémente le compteur à chaque heure de report et
    /// le conserve sinon, sans le remettre à zéro : c'est un total annuel, et non une suite
    /// d'heures consécutives.
    pub nbh_report_e: u32,
    /// Chaleur puisée par la boucle de chauffage d'un système solaire combiné, en Wh.
    pub q_puise_ch_wh: f64,
    /// Chauffage fourni par l'appoint raccordé d'un système solaire combiné, en Wh.
    pub q_fou_ch_ap_wh: f64,
    /// Températures des zones au début du premier pas de temps, en °C.
    pub theta_debut_c: Option<[f64; NB_ZONES]>,
}

/// Assemblage d'un ballon, d'un générateur de base et d'un éventuel appoint intégré.
#[derive(Debug, Clone, PartialEq)]
pub struct AssemblageBallon {
    /// Identifiant.
    pub id: String,
    /// « Nbassemblage » : nombre d'assemblages identiques.
    pub nb: f64,
    /// Ballon (un des assemblages identiques).
    pub ballon: Ballon,
    /// « θc_base » et « θc_ap » : consigne, en °C (éq. 1948).
    pub theta_c_c: f64,
    /// Générateur de base.
    pub base: RegulationGenerateur,
    /// Générateur d'appoint intégré (« Type_prod_stockage = 1 »).
    pub appoint: Option<RegulationGenerateur>,
    /// « Nbiter_vp » (éq. 1950).
    pub nb_iter_vp: usize,
    /// « nbhtemp_sto_insuff(h) » (éq. 1959).
    pub nbh_temp_insuff: u32,
    /// Cumuls annuels.
    pub cumuls: CumulsAssemblage,
    /// Valeurs par défaut de la méthode appliquées.
    pub valeurs_par_defaut: Vec<ValeurParDefaut>,
    /// Identification d'un chauffe-eau thermodynamique décrit par son essai (§9.22).
    pub identification: Option<super::idcet::Identification>,
    /// Boucle solaire tenant lieu de générateur de base (« Issolaire_base », §9.10.2 et
    /// §9.15) ; le champ `generateur` de `base` est alors sans objet.
    pub solaire: Option<Box<BoucleSolaireHoraire>>,
    /// Vrai pour un assemblage à appoint séparé instantané (« Type_prod_stockage = 3 ») : le
    /// puisage suit les éq. (1800) à (1807) et le reste est fourni par les générateurs
    /// instantanés d'ECS de la génération.
    pub appoint_separe: bool,
    /// Boucle de chauffage d'un système solaire combiné (fiches 9.18 à 9.20).
    pub ssc: Option<ParametresSsc>,
}

impl AssemblageBallon {
    /// Assemblage dont la consigne est la consigne conventionnelle d'un ballon d'ECS.
    pub fn nouveau(
        id: impl Into<String>,
        nb: f64,
        ballon: Ballon,
        base: RegulationGenerateur,
        appoint: Option<RegulationGenerateur>,
        nb_iter_vp: usize,
    ) -> Self {
        Self {
            id: id.into(),
            nb,
            ballon,
            theta_c_c: THETA_C_ECS_C,
            base,
            appoint,
            nb_iter_vp,
            nbh_temp_insuff: 0,
            cumuls: CumulsAssemblage {
                theta_haut_min_c: f64::INFINITY,
                ..CumulsAssemblage::default()
            },
            valeurs_par_defaut: Vec::new(),
            identification: None,
            solaire: None,
            appoint_separe: false,
            ssc: None,
        }
    }

    /// Calcule un pas de temps (§9.14.3.2.2). `appel(generateur, poste, Qreq, θaval)` appelle
    /// un générateur de la génération pour l'ensemble des assemblages identiques et renvoie
    /// l'énergie qu'il fournit, en Wh ; il est appelé même sans demande, la fiche du générateur
    /// décidant de sa consommation à charge nulle. Le poste est l'ECS (chauffage du ballon),
    /// ou le chauffage pour l'appoint raccordé d'un système solaire combiné (éq. 2150).
    pub fn pas(
        &mut self,
        e: &EntreesAssemblage,
        appel: &mut dyn FnMut(usize, Poste, f64, f64) -> f64,
    ) -> SortiesAssemblage {
        let p = self.puiser_pas(e);
        let mut boucle = None;
        let base = if let Some(sol) = &mut self.solaire {
            // Boucle solaire de base (§9.10.3.2.2) : enclenchée dès qu'elle peut réchauffer le
            // ballon, selon sa propre régulation (§9.15.3.2) ; son énergie est injectée en zone
            // basse, sans égard pour la consigne (la température maximale du ballon, éq. 1779,
            // borne la stratification).
            let s = sol.boucle.pas(&EntreesBoucle {
                isr_w_m2: e.isr_w_m2,
                theta_e_c: e.theta_ext_c,
                theta_amb_c: e.theta_amb_c,
                theta_b_moy_ech_c: p.theta_aval_base_c,
            });
            sol.cumuls.ajouter(&s, e.isr_w_m2, e.mois);
            boucle = Some(s);
            BaseDuPas::Injection(s.q_sol_wh)
        } else {
            BaseDuPas::Generateur
        };
        self.terminer_pas(e, &p, base, boucle, appel)
    }

    /// Début d'un pas de temps (§9.14.3.2.2, première étape) : pertes du pas de temps, puisage
    /// d'ECS et, pour un système solaire combiné, chaleur puisée par la boucle de chauffage ;
    /// températures vues par les échangeurs de la base et de l'appoint (éq. 1785).
    pub fn puiser_pas(&mut self, e: &EntreesAssemblage) -> ApresPuisage {
        let nb = self.nb;
        if self.cumuls.theta_debut_c.is_none() {
            self.cumuls.theta_debut_c = Some(self.ballon.theta_c);
        }
        // Pertes du pas de temps, avec les températures de la fin du pas précédent (éq. 1770).
        self.ballon.debut_pas(e.theta_amb_c);
        // Première étape : volume puisé pour l'ECS (éq. 1952 à 1954).
        let puisage = if self.appoint_separe {
            boucle_puisage_appoint_separe(
                &mut self.ballon,
                e.q_req_ecs_wh / nb,
                e.theta_entrant_c,
                e.theta_depart_c,
                self.nb_iter_vp,
            )
        } else {
            boucle_puisage(
                &mut self.ballon,
                e.q_req_ecs_wh / nb,
                e.theta_entrant_c,
                e.theta_depart_c,
                self.nb_iter_vp,
            )
        };
        // Systèmes solaires combinés, deuxième et troisième étapes (fiche 9.18, éq. 2116 à
        // 2125) : chaleur puisée par la boucle de chauffage dans la partie basse du ballon.
        let (q_puise_ch, q_rest_ch) = match self.ssc {
            Some(p) if e.q_req_ch_wh > 0.0 => {
                let q_req = e.q_req_ch_wh;
                self.puiser_chauffage(&p, q_req, e.theta_ch_max_c, e.theta_amb_c)
            }
            _ => (0.0, e.q_req_ch_wh.max(0.0)),
        };
        // Éq. (1785) : la température vue par un échangeur est celle des zones après le
        // puisage (et, dans un système solaire combiné, après l'extraction de la chaleur de
        // chauffage), avant toute injection de puissance ; celle de l'appoint est donc calculée
        // avant l'injection de la base.
        let b = self.base;
        ApresPuisage {
            puisage,
            q_puise_ch_wh: q_puise_ch,
            q_rest_ch_wh: q_rest_ch,
            theta_aval_base_c: self.ballon.theta_b_moy_ech_c(b.z_gen, b.h_rel_ech),
            theta_aval_ap_c: self
                .appoint
                .map(|a| self.ballon.theta_b_moy_ech_c(a.z_gen, a.h_rel_ech)),
        }
    }

    /// Fin d'un pas de temps (§9.14.3.2.2, troisième et quatrième étapes) : énergie du
    /// générateur de base (appelé, ou injectée par une boucle solaire ou un réseau de
    /// distribution), appoint intégré, appoint raccordé d'un système solaire combiné, cumuls.
    /// `boucle` : sorties de la boucle solaire propre à l'assemblage, le cas échéant.
    pub fn terminer_pas(
        &mut self,
        e: &EntreesAssemblage,
        p: &ApresPuisage,
        base: BaseDuPas,
        boucle: Option<SortiesBoucle>,
        appel: &mut dyn FnMut(usize, Poste, f64, f64) -> f64,
    ) -> SortiesAssemblage {
        let nb = self.nb;
        let puisage = p.puisage;
        let q_rest = puisage.q_report_wh * nb;
        let q_fou = e.q_req_ecs_wh - q_rest;
        let a_puise = puisage.v_p_l > 0.0;
        let mut q_rest_ch = p.q_rest_ch_wh;
        // Troisième étape : générateur de base (§9.13 : la température aval du générateur est
        // la température du ballon vue par son échangeur ; lecture Q48). §9.10.3.2.2 : « si
        // fp(h) = 0, alors Qreq_sto_base(h) = 0 » ; §9.14.3.2.2 : « si le générateur de base
        // est autorisé à fonctionner, il est appelé » : hors de sa programmation, le générateur
        // n'est pas appelé et ne consomme rien, pas même ses auxiliaires à charge nulle.
        let b = self.base;
        let mut q_req_sto = [0.0; 2];
        let mut q_fou_sto = [0.0; 2];
        match base {
            BaseDuPas::Injection(q) => q_fou_sto[0] = q,
            BaseDuPas::Generateur => {
                if f_p(b.programmation, e.hleg) {
                    if active(
                        &self.ballon,
                        b.z_reg,
                        self.theta_c_c,
                        b.delta_theta_k,
                        a_puise,
                        true,
                    ) {
                        q_req_sto[0] = q_req_sto_wh(&self.ballon, b.z_gen, self.theta_c_c, true);
                    }
                    q_fou_sto[0] = appel(
                        b.generateur,
                        Poste::Ecs,
                        q_req_sto[0] * nb,
                        p.theta_aval_base_c,
                    ) / nb;
                }
            }
        }
        // Les pertes du pas de temps sont appliquées au ballon, que la base fonctionne ou non.
        self.ballon.injecter(b.z_gen, q_fou_sto[0], true);
        // Quatrième étape : appoint intégré, après l'injection de la base ; les pertes du pas
        // de temps, déjà appliquées, ne sont pas comptées une seconde fois (note de l'éq. 1786).
        if let (Some(a), Some(theta_aval_ap)) = (self.appoint, p.theta_aval_ap_c) {
            if f_p(a.programmation, e.hleg) {
                if active(
                    &self.ballon,
                    a.z_reg,
                    self.theta_c_c,
                    a.delta_theta_k,
                    a_puise,
                    false,
                ) {
                    q_req_sto[1] = q_req_sto_wh(&self.ballon, a.z_gen, self.theta_c_c, false);
                }
                q_fou_sto[1] =
                    appel(a.generateur, Poste::Ecs, q_req_sto[1] * nb, theta_aval_ap) / nb;
                self.ballon.injecter(a.z_gen, q_fou_sto[1], false);
            }
        }
        // Système solaire combiné à appoint raccordé, neuvième étape (éq. 2150) : l'appoint est
        // appelé une seconde fois pour le chauffage restant, à la température de fonctionnement
        // des générateurs instantanés de chauffage ; il fonctionne alors comme un générateur
        // instantané, sans la programmation du thermostat du ballon (lecture Q52).
        let mut q_fou_ch_ap = 0.0;
        if let (Some(ps), Some(a)) = (self.ssc, self.appoint) {
            if ps.raccorde && q_rest_ch > 0.0 {
                q_fou_ch_ap = appel(a.generateur, Poste::Chauffage, q_rest_ch, e.theta_aval_ch_c);
                q_rest_ch = (q_rest_ch - q_fou_ch_ap).max(0.0);
            }
        }
        // Éq. (1959) : heures consécutives sans atteindre la consigne en zone haute.
        let theta_haut = self.ballon.theta_c[NB_ZONES - 1];
        self.nbh_temp_insuff = if theta_haut < self.theta_c_c - 1e-6 {
            self.nbh_temp_insuff + 1
        } else {
            0
        };
        let s = SortiesAssemblage {
            q_fou_ecs_wh: q_fou,
            q_rest_ecs_wh: q_rest,
            phi_pertes_wh: self.ballon.pertes_totales_wh() * nb,
            v_p_l: puisage.v_p_l * nb,
            q_req_sto_wh: q_req_sto.map(|q| q * nb),
            q_fou_sto_wh: q_fou_sto.map(|q| q * nb),
            q_sol_wh: boucle.map_or(0.0, |b| b.q_sol_wh * nb),
            w_pompe_wh: boucle.map_or(0.0, |b| b.p_p_wh * nb),
            phi_vc_bs_wh: boucle.map_or(0.0, |b| b.phi_vc_wh * nb),
            // Éq. (2135) et (2151) : la pompe d'un système solaire combiné est répartie entre le
            // chauffage et l'ECS au prorata des demandes, et va à l'ECS sans demande.
            w_pompe_ch_wh: match (self.ssc, boucle) {
                (Some(_), Some(b)) if e.q_req_ch_wh + e.q_req_ecs_wh > 0.0 => {
                    b.p_p_wh * nb * e.q_req_ch_wh / (e.q_req_ch_wh + e.q_req_ecs_wh)
                }
                _ => 0.0,
            },
            q_puise_ch_wh: p.q_puise_ch_wh,
            q_fou_ch_ap_wh: q_fou_ch_ap,
            q_rest_ch_wh: q_rest_ch,
        };
        let c = &mut self.cumuls;
        c.q_fou_ecs_wh += s.q_fou_ecs_wh;
        c.phi_pertes_wh += s.phi_pertes_wh;
        c.v_p_l += s.v_p_l;
        for k in 0..2 {
            c.q_fou_sto_wh[k] += s.q_fou_sto_wh[k];
        }
        c.nbh_temp_insuff_max = c.nbh_temp_insuff_max.max(self.nbh_temp_insuff);
        c.q_puise_ch_wh += s.q_puise_ch_wh;
        c.q_fou_ch_ap_wh += s.q_fou_ch_ap_wh;
        c.theta_haut_min_c = c.theta_haut_min_c.min(theta_haut);
        // Éq. (1780).
        if puisage.q_report_wh != 0.0 {
            c.nbh_report_e += 1;
        }
        s
    }
}

/// État d'un assemblage après le puisage d'un pas de temps (§9.14.3.2.2, première étape).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApresPuisage {
    /// Volume puisé et énergie restant à puiser, pour un assemblage.
    pub puisage: Puisage,
    /// « Qpuisé_ch_SSC » : chaleur puisée par la boucle de chauffage d'un système solaire
    /// combiné, en Wh.
    pub q_puise_ch_wh: f64,
    /// « Qrest_ch » : demande de chauffage non couverte par le ballon, en Wh.
    pub q_rest_ch_wh: f64,
    /// « θb_moy_ech » vue par l'échangeur de la base, en °C (éq. 1785).
    pub theta_aval_base_c: f64,
    /// La même pour l'appoint intégré, en °C.
    pub theta_aval_ap_c: Option<f64>,
}

/// Énergie du générateur de base d'un assemblage au pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BaseDuPas {
    /// Générateur de la génération, appelé selon son thermostat (§9.10.3.2.2).
    Generateur,
    /// Énergie injectée en zone de base, pour un assemblage, en Wh : boucle solaire (§9.15),
    /// part de la boucle collective d'un CESCI (éq. 2009) ou énergie reçue du réseau
    /// intergroupe d'un CESCAI (éq. 2079).
    Injection(f64),
}

impl AssemblageBallon {
    /// Gestion-régulation de la boucle de chauffage d'un système solaire combiné (fiche 9.18,
    /// éq. 2116 à 2125, relues sur l'image des p. 1217 et 1218) et extraction de la chaleur
    /// dans la zone de son échangeur, sans pertes (troisième étape des fiches 9.19 et 9.20) :
    /// renvoie `(Qpuisé_ch_SSC, Qrest_ch)`, en Wh.
    fn puiser_chauffage(
        &mut self,
        p: &ParametresSsc,
        q_req_ch_wh: f64,
        theta_ch_max_c: f64,
        theta_amb_c: f64,
    ) -> (f64, f64) {
        let b = &mut self.ballon;
        // Éq. (2117) et (2118).
        let theta_puisage = theta_ch_max_c + q_req_ch_wh / ua_hx_bcl_ch_w_k(b.v_tot_l());
        // Éq. (2116) : énergie disponible dans les zones situées sous l'échangeur, après le
        // puisage d'ECS.
        let dispo: f64 = (0..=Z_ECH_CH)
            .map(|z| RHO_CW_WH_L_K * b.v_l[z] * (b.theta_c[z] - theta_puisage).max(0.0))
            .sum();
        // Éq. (2119).
        let q_int = q_req_ch_wh.min(dispo);
        let q_puise = if p.raccorde {
            // Éq. (2123) à (2125) : la boucle ne fonctionne que si le ballon couvre toute la
            // demande (l'éq. 2123, mal imprimée, est lue `Qpuisé_ch_SSC_int < Qreq_ch` ;
            // lecture Q52).
            if q_int < q_req_ch_wh {
                0.0
            } else {
                q_req_ch_wh
            }
        } else if b.theta_prec_c[Z_ECH_CH] - theta_amb_c < p.delta_theta_reg_bcl_k || dispo <= 0.0 {
            // Éq. (2120) et (2121).
            0.0
        } else {
            // Éq. (2122).
            q_int
        };
        if q_puise > 0.0 {
            b.theta_c[Z_ECH_CH] -= q_puise / (RHO_CW_WH_L_K * b.v_l[Z_ECH_CH]);
            b.melanger();
        }
        (q_puise, q_req_ch_wh - q_puise)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ecs::ballon_de_stockage::RHO_CW_WH_L_K;

    fn regulation(programmation: Programmation) -> RegulationGenerateur {
        RegulationGenerateur {
            generateur: 0,
            programmation,
            delta_theta_k: 2.0,
            z_gen: 0,
            z_reg: 0,
            h_rel_ech: 0.0,
        }
    }

    fn entrees(q: f64, hleg: u8) -> EntreesAssemblage {
        EntreesAssemblage {
            q_req_ecs_wh: q,
            theta_entrant_c: 15.0,
            theta_depart_c: 53.0,
            theta_amb_c: 20.0,
            hleg,
            theta_ext_c: 10.0,
            isr_w_m2: 0.0,
            mois: 0,
            q_req_ch_wh: 0.0,
            theta_ch_max_c: 35.0,
            theta_aval_ch_c: 35.0,
        }
    }

    #[test]
    fn ballon_electrique_base_seule() {
        // Ballon de 200 L, UA = 2 W/K, résistance de 2 kW en fonctionnement permanent.
        let ballon = Ballon::nouveau(200.0, 2.0, None, 90.0);
        let mut a = AssemblageBallon::nouveau(
            "ballon",
            1.0,
            ballon,
            regulation(Programmation::Permanent),
            None,
            4,
        );
        let mut resistance = |_: usize, _: Poste, q: f64, _: f64| q.min(2000.0);
        // Premier pas : ballon à 50 °C, sous θc − Δθ : il faut 1 163 Wh pour remonter à 55 °C
        // et 60 Wh de pertes (éq. 1815) ; la résistance les fournit.
        let s = a.pas(&entrees(0.0, 12), &mut resistance);
        assert!((s.q_req_sto_wh[0] - 1223.0).abs() < 1e-9);
        assert!((s.q_fou_sto_wh[0] - 1223.0).abs() < 1e-9);
        assert!((s.phi_pertes_wh - 60.0).abs() < 1e-9);
        for t in a.ballon.theta_c {
            assert!((t - 55.0).abs() < 1e-9);
        }
        // À la consigne : pas d'enclenchement, le ballon se refroidit de ses pertes (70 Wh).
        let s = a.pas(&entrees(0.0, 13), &mut resistance);
        assert_eq!(s.q_req_sto_wh[0], 0.0);
        let chute = 70.0 / (RHO_CW_WH_L_K * 200.0);
        assert!((a.ballon.theta_c[0] - (55.0 - chute)).abs() < 1e-9);
        // Puisage de 930,4 Wh (20 L depuis la zone haute) : fourni, et la base s'enclenche pour
        // tout remonter à la consigne, pertes comprises.
        let avant = a.ballon.energie_wh(0.0);
        let s = a.pas(&entrees(930.4, 14), &mut resistance);
        assert_eq!(s.q_rest_ecs_wh, 0.0);
        assert!((s.q_fou_ecs_wh - 930.4).abs() < 1e-9);
        assert!(s.q_req_sto_wh[0] > 930.4);
        // Bilan du ballon sur le pas : variation = injection − pertes − puisage net de l'eau
        // froide entrée (le puisage retire ρc·Vp·(θb4 − θentrant)).
        let apres = a.ballon.energie_wh(0.0);
        assert!(
            (apres - avant - s.q_fou_sto_wh[0] + s.phi_pertes_wh + s.q_fou_ecs_wh).abs() < 1e-6
        );
    }

    #[test]
    fn programmation_de_nuit_et_report() {
        let ballon = Ballon::nouveau(200.0, 2.0, None, 90.0);
        let mut a = AssemblageBallon::nouveau(
            "ballon",
            2.0,
            ballon,
            regulation(Programmation::Nuit),
            None,
            4,
        );
        let mut resistance = |_: usize, _: Poste, q: f64, _: f64| q.min(4000.0);
        // De jour, la résistance n'est pas autorisée : rien n'est injecté ; la zone haute à
        // 50 °C est sous les 53 °C attendus, la demande est reportée (éq. 1795 et 1953).
        let s = a.pas(&entrees(1000.0, 12), &mut resistance);
        assert_eq!(s.q_fou_sto_wh[0], 0.0);
        assert_eq!(s.q_rest_ecs_wh, 1000.0);
        assert_eq!(s.q_fou_ecs_wh, 0.0);
        // Le générateur n'est pas appelé hors de sa programmation (§9.10.3.2.2 et §9.14.3.2.2),
        // même à charge nulle ; les pertes du ballon sont appliquées.
        let mut appels = 0;
        let mut compteur = |_: usize, _: Poste, q: f64, _: f64| {
            appels += 1;
            q
        };
        let avant = a.ballon.energie_wh(0.0);
        let s = a.pas(&entrees(0.0, 13), &mut compteur);
        assert_eq!(appels, 0);
        assert!((avant - a.ballon.energie_wh(0.0) - s.phi_pertes_wh / 2.0).abs() < 1e-9);
        assert_eq!(a.cumuls.nbh_report_e, 1);
        // De nuit, deux assemblages identiques : demande double au générateur.
        let s = a.pas(&entrees(0.0, 2), &mut resistance);
        assert!(s.q_req_sto_wh[0] > 2.0 * 1163.0);
        assert_eq!(a.cumuls.nbh_temp_insuff_max, 2);
        assert_eq!(a.nbh_temp_insuff, 0);
    }

    #[test]
    fn base_et_appoint_integre() {
        // Base limitée à 500 W, appoint électrique en zone 3 : l'appoint complète la moitié
        // haute du ballon.
        let ballon = Ballon::nouveau(200.0, 0.0, Some(0.5), 90.0);
        let base = regulation(Programmation::Permanent);
        let appoint = RegulationGenerateur {
            generateur: 1,
            z_gen: 2,
            z_reg: 2,
            ..base
        };
        let mut a = AssemblageBallon::nouveau("ballon", 1.0, ballon, base, Some(appoint), 4);
        let mut avals = Vec::new();
        let mut appel = |g: usize, _: Poste, q: f64, theta: f64| {
            avals.push(theta);
            if g == 0 {
                q.min(500.0)
            } else {
                q
            }
        };
        let s = a.pas(&entrees(0.0, 12), &mut appel);
        // Éq. (1785) : l'appoint voit le ballon avant l'injection de la base, à 50 °C, et non
        // le mélange à 52,15 °C qui la suit.
        assert_eq!(avals, [50.0, 50.0]);
        // La base injecte 500 Wh en zone basse : + 8,6 K sur 50 L, mélangés avec la zone 2.
        assert!((s.q_fou_sto_wh[0] - 500.0).abs() < 1e-9);
        // Tout le ballon se mélange à 50 + 500/232,6 = 52,15 °C ; l'appoint remonte les deux
        // zones hautes à 55 °C : 581,5 − 250 = 331,5 Wh.
        assert!((s.q_fou_sto_wh[1] - 331.5).abs() < 1e-9);
        assert!((a.ballon.theta_c[3] - 55.0).abs() < 1e-9);
        assert!(a.ballon.theta_c[0] < 55.0);
    }

    #[test]
    fn boucle_de_chauffage_d_un_systeme_solaire_combine() {
        // Éq. (2118) : 1 200 W/K à 200 L, 12 000 W/K à 2 000 L, linéaire.
        assert!((ua_hx_bcl_ch_w_k(200.0) - 1200.0).abs() < 1e-9);
        assert!((ua_hx_bcl_ch_w_k(500.0) - 3000.0).abs() < 1e-9);
        // Ballon de 200 L à 50 °C, sans pertes, appoint en zone 3 (faux = 0,5) ; réseau à
        // 35 °C, 1 000 Wh demandés : θpuisage = 35 + 1 000/1 200 (éq. 2117) ; énergie
        // disponible dans les zones 1 et 2, 1,163 × 100 × (50 − 35,83) = 1 647,6 Wh (éq. 2116).
        let nouveau = |raccorde: bool| {
            let base = regulation(Programmation::Permanent);
            let appoint = RegulationGenerateur {
                generateur: 1,
                z_gen: 2,
                z_reg: 2,
                ..base
            };
            let mut a = AssemblageBallon::nouveau(
                "ssc",
                1.0,
                Ballon::nouveau(200.0, 0.0, Some(0.5), 90.0),
                base,
                Some(appoint),
                4,
            );
            a.ssc = Some(ParametresSsc {
                raccorde,
                delta_theta_reg_bcl_k: 2.0,
            });
            a
        };
        let e = |q_ch: f64| EntreesAssemblage {
            q_req_ch_wh: q_ch,
            ..entrees(0.0, 12)
        };
        let appels = std::cell::RefCell::new(Vec::new());
        let mut appel = |g: usize, p: Poste, q: f64, _: f64| {
            appels.borrow_mut().push((g, p, q));
            if p == Poste::Chauffage {
                q
            } else {
                0.0
            }
        };
        // Appoint indépendant : la demande est couverte (éq. 2122), puisée en zone 2.
        let mut a = nouveau(false);
        let s = a.pas(&e(1000.0), &mut appel);
        assert!((s.q_puise_ch_wh - 1000.0).abs() < 1e-9);
        assert!(s.q_rest_ch_wh.abs() < 1e-9);
        // 2 000 Wh : seule l'énergie disponible est puisée, le reste va aux autres générateurs.
        let mut a = nouveau(false);
        let s = a.pas(&e(2000.0), &mut appel);
        let dispo = 1.163 * 100.0 * (50.0 - (35.0 + 2000.0 / 1200.0));
        assert!((s.q_puise_ch_wh - dispo).abs() < 1e-9);
        assert!((s.q_rest_ch_wh - (2000.0 - dispo)).abs() < 1e-9);
        // Appoint raccordé : rien n'est puisé si le ballon ne couvre pas toute la demande
        // (éq. 2123 et 2124) ; l'appoint est rappelé pour le chauffage (éq. 2150).
        appels.borrow_mut().clear();
        let mut a = nouveau(true);
        let s = a.pas(&e(2000.0), &mut appel);
        assert_eq!(s.q_puise_ch_wh, 0.0);
        assert_eq!(s.q_fou_ch_ap_wh, 2000.0);
        assert_eq!(s.q_rest_ch_wh, 0.0);
        assert!(appels.borrow().contains(&(1, Poste::Chauffage, 2000.0)));
    }
}
