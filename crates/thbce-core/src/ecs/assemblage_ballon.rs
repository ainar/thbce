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

use super::ballon_de_stockage::{Ballon, NB_ZONES};
use super::boucle_solaire::{BoucleSolaireHoraire, EntreesBoucle};
use super::gestion_regulation_ballon::{
    active, boucle_puisage, boucle_puisage_appoint_separe, f_p, q_req_sto_wh, Programmation,
    THETA_C_ECS_C,
};
use crate::resultats::ValeurParDefaut;

/// Nombre d'heures consécutives sans atteindre la consigne au-delà duquel le texte signale un
/// ballon incohérent avec le projet (§9.14.3.4, éq. 1959).
pub const SEUIL_TEMPERATURE_INSUFFISANTE_H: u32 = 168;

/// Nombre d'heures de report d'énergie à l'issue du puisage au-delà duquel le moteur affiche
/// une alerte (§9.9.3.2.3, éq. 1780).
pub const SEUIL_REPORT_PUISAGE_H: u32 = 24;

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
        }
    }

    /// Calcule un pas de temps (§9.14.3.2.2). `appel(generateur, Qreq, θaval)` appelle un
    /// générateur de la génération pour l'ensemble des assemblages identiques et renvoie
    /// l'énergie qu'il fournit, en Wh ; il est appelé même sans demande, la fiche du générateur
    /// décidant de sa consommation à charge nulle.
    pub fn pas(
        &mut self,
        e: &EntreesAssemblage,
        appel: &mut dyn FnMut(usize, f64, f64) -> f64,
    ) -> SortiesAssemblage {
        let nb = self.nb;
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
        let q_rest = puisage.q_report_wh * nb;
        let q_fou = e.q_req_ecs_wh - q_rest;
        let a_puise = puisage.v_p_l > 0.0;
        // Éq. (1785) : la température vue par un échangeur est celle des zones après le
        // puisage, avant toute injection de puissance ; celle de l'appoint est donc calculée
        // avant l'injection de la base.
        let b = self.base;
        let theta_aval_base = self.ballon.theta_b_moy_ech_c(b.z_gen, b.h_rel_ech);
        let theta_aval_ap = self
            .appoint
            .map(|a| self.ballon.theta_b_moy_ech_c(a.z_gen, a.h_rel_ech));
        // Troisième étape : générateur de base (§9.13 : la température aval du générateur est
        // la température du ballon vue par son échangeur ; lecture Q48). §9.10.3.2.2 : « si
        // fp(h) = 0, alors Qreq_sto_base(h) = 0 » ; §9.14.3.2.2 : « si le générateur de base
        // est autorisé à fonctionner, il est appelé » : hors de sa programmation, le générateur
        // n'est pas appelé et ne consomme rien, pas même ses auxiliaires à charge nulle.
        let mut q_req_sto = [0.0; 2];
        let mut q_fou_sto = [0.0; 2];
        let mut boucle = None;
        if let Some(sol) = &mut self.solaire {
            // Boucle solaire de base (§9.10.3.2.2) : enclenchée dès qu'elle peut réchauffer le
            // ballon, selon sa propre régulation (§9.15.3.2) ; son énergie est injectée en zone
            // basse, sans égard pour la consigne (la température maximale du ballon, éq. 1779,
            // borne la stratification).
            let s = sol.boucle.pas(&EntreesBoucle {
                isr_w_m2: e.isr_w_m2,
                theta_e_c: e.theta_ext_c,
                theta_amb_c: e.theta_amb_c,
                theta_b_moy_ech_c: theta_aval_base,
            });
            q_fou_sto[0] = s.q_sol_wh;
            let c = &mut sol.cumuls;
            c.e_sol_mensuel_wh[e.mois.min(11)] += s.q_sol_wh;
            c.e_aux_wh += s.p_p_wh;
            c.phi_vc_wh += s.phi_vc_wh;
            c.isr_wh_m2 += e.isr_w_m2;
            c.nbh_marche += u32::from(s.en_marche);
            boucle = Some(s);
        } else if f_p(b.programmation, e.hleg) {
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
            q_fou_sto[0] = appel(b.generateur, q_req_sto[0] * nb, theta_aval_base) / nb;
        }
        // Les pertes du pas de temps sont appliquées au ballon, que la base fonctionne ou non.
        self.ballon.injecter(b.z_gen, q_fou_sto[0], true);
        // Quatrième étape : appoint intégré, après l'injection de la base ; les pertes du pas
        // de temps, déjà appliquées, ne sont pas comptées une seconde fois (note de l'éq. 1786).
        if let (Some(a), Some(theta_aval_ap)) = (self.appoint, theta_aval_ap) {
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
                q_fou_sto[1] = appel(a.generateur, q_req_sto[1] * nb, theta_aval_ap) / nb;
                self.ballon.injecter(a.z_gen, q_fou_sto[1], false);
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
        };
        let c = &mut self.cumuls;
        c.q_fou_ecs_wh += s.q_fou_ecs_wh;
        c.phi_pertes_wh += s.phi_pertes_wh;
        c.v_p_l += s.v_p_l;
        for k in 0..2 {
            c.q_fou_sto_wh[k] += s.q_fou_sto_wh[k];
        }
        c.nbh_temp_insuff_max = c.nbh_temp_insuff_max.max(self.nbh_temp_insuff);
        c.theta_haut_min_c = c.theta_haut_min_c.min(theta_haut);
        // Éq. (1780).
        if puisage.q_report_wh != 0.0 {
            c.nbh_report_e += 1;
        }
        s
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
        let mut resistance = |_: usize, q: f64, _: f64| q.min(2000.0);
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
        let mut resistance = |_: usize, q: f64, _: f64| q.min(4000.0);
        // De jour, la résistance n'est pas autorisée : rien n'est injecté ; la zone haute à
        // 50 °C est sous les 53 °C attendus, la demande est reportée (éq. 1795 et 1953).
        let s = a.pas(&entrees(1000.0, 12), &mut resistance);
        assert_eq!(s.q_fou_sto_wh[0], 0.0);
        assert_eq!(s.q_rest_ecs_wh, 1000.0);
        assert_eq!(s.q_fou_ecs_wh, 0.0);
        // Le générateur n'est pas appelé hors de sa programmation (§9.10.3.2.2 et §9.14.3.2.2),
        // même à charge nulle ; les pertes du ballon sont appliquées.
        let mut appels = 0;
        let mut compteur = |_: usize, q: f64, _: f64| {
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
        let mut appel = |g: usize, q: f64, theta: f64| {
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
}
