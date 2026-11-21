// SPDX-License-Identifier: GPL-3.0-or-later
//! Gestion et régulation horaire de la génération (Th-BCE 2020 §8.17) : demandes des réseaux,
//! températures de fonctionnement, répartition entre générateurs et stockages, assemblage
//! (§8.14).

use super::*;

impl GenerationHoraire {
    /// Calcule un pas de temps de la génération ; `mois` est l'indice du mois des scénarios, de
    /// 0 à 11, pour les cumuls.
    pub fn pas(&mut self, e: &EntreesGeneration<'_>, mois: usize) -> &SortiesGeneration {
        // §8.17.3.5.1 : jeux de données remis à zéro.
        for (i, g) in self.generateurs.iter_mut().enumerate() {
            g.jeu = JeuDonnees::default();
            if let Some(s) = e.sources.get(i) {
                g.jeu.theta_amont_c = s.theta_amont_c;
            }
        }
        // Éq. (993) à (995) : demandes totales sans report.
        let mut q_sr = [0.0; 3];
        for r in e.reseaux {
            q_sr[r.poste.indice()] += r.q_sys_wh;
        }
        // Éq. (996) à (998) : part de chaque réseau dans la demande de son poste ; à défaut de
        // demande, sa part de la surface desservie.
        let rat_bes_dp: Vec<f64> = e
            .reseaux
            .iter()
            .map(|r| {
                let total = q_sr[r.poste.indice()];
                if total > 0.0 {
                    r.q_sys_wh / total
                } else {
                    r.rat_surf_dess
                }
            })
            .collect();
        // Éq. (1005) et (1006).
        let relance = [Poste::Chauffage, Poste::Refroidissement]
            .map(|p| e.reseaux.iter().any(|r| r.poste == p && r.relance));
        let theta_amb = self.theta_amb_c(e.aut_ch, e.aut_fr, e.theta_ext_c, e.b_therm);
        // Éq. (1014) à (1016) : demandes avec report.
        let q_ar = [0, 1, 2].map(|p| q_sr[p] + self.etat.q_rep_wh[p]);
        // Éq. (1009) à (1013) : températures de fonctionnement.
        let theta_aval = [
            self.theta_aval(
                Poste::Chauffage,
                e,
                &rat_bes_dp,
                relance[0],
                q_sr[0],
                q_ar[0],
            ),
            self.theta_aval(
                Poste::Refroidissement,
                e,
                &rat_bes_dp,
                relance[1],
                q_sr[1],
                q_ar[1],
            ),
            // Éq. (1009) : en ECS instantanée, les générateurs de la première version (effet
            // Joule) ne dépendent pas de la température de fonctionnement ; celle du départ le
            // plus chaud des réseaux est transmise.
            e.reseaux
                .iter()
                .filter(|r| r.poste == Poste::Ecs)
                .map(|r| r.theta_moy_c)
                .fold(0.0, f64::max),
        ];

        // §8.17.3.7 : algorithme principal pour les générateurs instantanés, dans l'ordre des
        // étapes du texte : ECS, refroidissement, chauffage (lecture Q46).
        let mut q_rep = [0.0; 3];
        let mut q_ar = q_ar;
        let q_ar_ecs = q_ar[2];
        // §8.17.3.6 et §9.14 : les assemblages avec ballon de stockage sont les premiers
        // sollicités pour l'ECS ; ce qu'ils ne fournissent pas passe aux générateurs
        // instantanés, ou est reporté.
        let mut q_fou_sto = 0.0;
        let mut phi_pertes_sto = 0.0;
        let mut w_pompes_solaires = 0.0;
        let mut w_pompes_solaires_ch = 0.0;
        let mut phi_vc_boucles = 0.0;
        let mut q_puise_ch = 0.0;
        if e.id_encl && !self.stockages.is_empty() {
            let mut stockages = std::mem::take(&mut self.stockages);
            let theta_depart = e
                .reseaux
                .iter()
                .filter(|r| r.poste == Poste::Ecs)
                .map(|r| r.theta_dep_c)
                .fold(0.0, f64::max);
            for st in &mut stockages {
                // Fiche 3.2 : ensoleillement du plan des capteurs d'une boucle solaire.
                let isr = match (&st.solaire, e.climat) {
                    (Some(sol), Some(climat)) => sol.capteurs.isr_w_m2(climat, e.h),
                    _ => 0.0,
                };
                // Systèmes solaires combinés : premiers sollicités aussi pour le chauffage, en
                // saison de chauffage (fiches 9.19 et 9.20).
                let ssc = st.ssc.is_some();
                // Éq. (2117) : « θch_max^gen » est la température de départ de dimensionnement
                // la plus haute des réseaux hydrauliques de chauffage de la génération, requise
                // par la validation pour un système solaire combiné (lecture Q52) ; sans elle,
                // le ballon ne contribue pas au chauffage.
                let theta_ch_max = self.theta_dist_ch_max_c;
                let q_req_ch = match theta_ch_max {
                    Some(_) if ssc && e.aut_ch => q_ar[0],
                    _ => 0.0,
                };
                let s = st.pas(
                    &EntreesAssemblage {
                        q_req_ecs_wh: q_ar[2],
                        q_ecs_horaire_wh: q_sr[2],
                        theta_entrant_c: e.theta_cw_c,
                        theta_depart_c: theta_depart,
                        theta_amb_c: theta_amb,
                        hleg: e.hleg,
                        theta_ext_c: e.theta_ext_c,
                        isr_w_m2: isr,
                        mois,
                        q_req_ch_wh: q_req_ch,
                        theta_ch_max_c: theta_ch_max.unwrap_or(f64::NAN),
                        theta_aval_ch_c: theta_aval[0],
                    },
                    &mut |i, poste, q_req, theta_aval| {
                        self.appeler_pour_ballon(i, poste, q_req, theta_aval, theta_amb, e)
                    },
                );
                q_ar[2] = s.q_rest_ecs_wh;
                if ssc && e.aut_ch && theta_ch_max.is_some() {
                    q_ar[0] = s.q_rest_ch_wh;
                    q_puise_ch += s.q_puise_ch_wh;
                }
                w_pompes_solaires_ch += s.w_pompe_ch_wh;
                q_fou_sto += s.q_fou_ecs_wh;
                phi_pertes_sto += s.phi_pertes_wh;
                w_pompes_solaires += s.w_pompe_wh;
                phi_vc_boucles += s.phi_vc_bs_wh;
            }
            self.stockages = stockages;
        }
        // Fiches 9.16 et 9.17 : productions solaires collectives à appoints décentralisés,
        // premières sollicitées pour l'ECS ; un appoint séparé instantané reçoit le reste
        // (éq. 2010) comme générateur instantané d'ECS de la génération.
        let mut phi_pertes_dec_vc = 0.0;
        if e.id_encl && !self.collectifs.is_empty() {
            let mut collectifs = std::mem::take(&mut self.collectifs);
            let ecs = || e.reseaux.iter().filter(|r| r.poste == Poste::Ecs);
            let theta_depart = ecs().map(|r| r.theta_dep_c).fold(0.0, f64::max);
            let theta_moy_prim = ecs().map(|r| r.theta_moy_c).fold(0.0, f64::max);
            let phi_pertes_prim: f64 = ecs().map(|r| r.phi_pertes_prim_wh).sum();
            for c in &mut collectifs {
                // Éq. (2074) : ambiance d'un CESCAI (ballon central, boucle, réseau) à 20 °C en
                // volume chauffé, extérieure hors volume chauffé ; ses ballons individuels et
                // leurs appoints sont en volume chauffé, à 20 °C. CESCI : ambiance de la
                // génération (éq. 1978, lecture Q53).
                let cescai = c.central.is_some();
                let amb = match (cescai, self.en_volume_chauffe) {
                    (false, _) => theta_amb,
                    (true, true) => THETA_AMB_CH_C,
                    (true, false) => e.theta_ext_c,
                };
                let amb_appoints = if cescai { THETA_AMB_CH_C } else { theta_amb };
                let isr = e
                    .climat
                    .map_or(0.0, |climat| c.boucle.capteurs.isr_w_m2(climat, e.h));
                let s = c.pas(
                    &EntreesCollectif {
                        q_req_ecs_wh: q_ar[2],
                        phi_pertes_prim_wh: phi_pertes_prim,
                        theta_moy_prim_c: theta_moy_prim,
                        theta_cw_c: e.theta_cw_c,
                        theta_depart_c: theta_depart,
                        theta_amb_c: amb,
                        theta_amb_dec_c: THETA_AMB_CH_C,
                        hleg: e.hleg,
                        theta_ext_c: e.theta_ext_c,
                        isr_w_m2: isr,
                        mois,
                    },
                    &mut |i, poste, q_req, theta_aval| {
                        self.appeler_pour_ballon(i, poste, q_req, theta_aval, amb_appoints, e)
                    },
                );
                q_ar[2] = s.q_rest_ecs_wh;
                q_fou_sto += s.q_fou_ecs_wh;
                phi_pertes_sto += s.phi_pertes_wh;
                phi_pertes_dec_vc += s.phi_pertes_dec_vc_wh;
                w_pompes_solaires += s.w_pompe_wh;
                phi_vc_boucles += s.phi_vc_bs_wh;
            }
            self.collectifs = collectifs;
        }
        let ctx = |poste: Poste| ContexteAppel {
            poste,
            theta_aval_c: theta_aval[poste.indice()],
            theta_amb_c: theta_amb,
            aut_ch: e.aut_ch,
            relance_ch: relance[0],
        };
        // §8.17.3.7.2.3.4 : alternance « chauffage et ECS », hors du cas général.
        let biposte = self.alternance_biposte();
        if biposte {
            let [ch, ecs] = self.alternance_chauffage_ecs(
                &ctx(Poste::Chauffage),
                &ctx(Poste::Ecs),
                q_ar,
                e.id_encl,
                e.sources,
            );
            q_rep[0] = ch;
            q_rep[2] = ecs;
        }
        for poste in [Poste::Ecs, Poste::Refroidissement, Poste::Chauffage] {
            let p = poste.indice();
            if biposte && poste != Poste::Refroidissement {
                continue;
            }
            let actif = match poste {
                Poste::Ecs => e.id_encl,
                Poste::Refroidissement => e.aut_fr,
                Poste::Chauffage => e.aut_ch,
            };
            let ctx = ctx(poste);
            q_rep[p] = if actif {
                self.appeler_generateurs(&ctx, q_ar[p], e.sources)
            } else {
                // Hors période d'activation, les générateurs sont désactivés pour le poste : la
                // demande éventuelle reste reportée (éq. 1028).
                q_ar[p]
            };
        }

        // §8.17.3.8, éq. (1053) à (1055) : auxiliaires amont des générateurs thermodynamiques,
        // ajoutés à leurs auxiliaires et à leur matrice au prorata des charges des postes.
        for g in &mut self.generateurs {
            let Some(source) = g.source else { continue };
            // Pompes des sources sur eau en marche permanente pendant la saison du générateur
            // (éq. 1444 et 1446, lecture Q151).
            let en_saison = super::super::sources_amont::generateur_en_saison(
                g.modele.fonction(),
                e.aut_ch,
                e.aut_fr,
            );
            let w = source.w_aux_am_saison_wh(g.jeu.tau_charge, en_saison);
            if w == 0.0 {
                continue;
            }
            g.jeu.w_aux_wh += w;
            let e_el = Energie::Electricite;
            // Lecture Q151 : à charge nulle, les pompes d'un générateur qui ne fait que du froid,
            // ou d'un générateur réversible en saison de refroidissement seule, vont au poste
            // refroidissement.
            let froid_a_l_arret = g.jeu.tau_charge <= 0.0
                && g.modele.fonction().assure(Poste::Refroidissement)
                && (!g.modele.fonction().assure(Poste::Chauffage) || (e.aut_fr && !e.aut_ch));
            if g.jeu.ia_refroidi || froid_a_l_arret {
                g.jeu.q_cef.ajouter(Poste::Refroidissement, e_el, w);
            } else if g.jeu.tau_charge > 0.0 {
                // Lecture Q47 : la case « (2;50) » de la branche sans refroidissement de
                // l'éq. (1055) est lue « (1;50) », chauffage.
                let part_ecs = (g.jeu.r_fonct_ecs / g.jeu.tau_charge).clamp(0.0, 1.0);
                g.jeu
                    .q_cef
                    .ajouter(Poste::Chauffage, e_el, (1.0 - part_ecs) * w);
                g.jeu.q_cef.ajouter(Poste::Ecs, e_el, part_ecs * w);
            } else if g.modele.fonction() == FonctionGenerateur::Ecs || !e.aut_ch {
                g.jeu.q_cef.ajouter(Poste::Ecs, e_el, w);
            } else {
                g.jeu.q_cef.ajouter(Poste::Chauffage, e_el, w);
            }
        }

        // §8.17.3.9, éq. (1056) à (1058) : matrice et pertes de la génération.
        let mut q_cef = MatriceCef::NULLE;
        let mut phi_vc = 0.0;
        // Énergie fournie aux réseaux : celle des générateurs instantanés et, en ECS, celle
        // des ballons (l'énergie des générateurs de ballon va au stockage).
        let mut q_fou = [q_puise_ch, 0.0, q_fou_sto];
        let instantanes_ecs = self.generateurs_du_poste(Poste::Ecs);
        for (i, g) in self.generateurs.iter().enumerate() {
            q_cef.ajouter_matrice(&g.jeu.q_cef);
            phi_vc += g.jeu.phi_vc_wh;
            q_fou[0] += g.jeu.q_fou_wh[0];
            q_fou[1] += g.jeu.q_fou_wh[1];
            if instantanes_ecs.contains(&i) {
                q_fou[2] += g.jeu.q_fou_wh[2];
            }
        }
        // Éq. (1058) et (1961) : pertes des ballons, récupérables en volume chauffé.
        phi_vc += phi_pertes_sto;
        self.cumuls.phi_pertes_sto_wh += phi_pertes_sto;
        let q_ar = [q_ar[0], q_ar[1], q_ar_ecs];
        q_cef.ajouter(Poste::Ecs, Energie::Electricite, e.w_rechauf_wh);
        // Éq. (1972), (2135) et (2151) : pompes des boucles solaires, au poste ECS ou réparties
        // entre le chauffage et l'ECS pour un système solaire combiné.
        q_cef.ajouter(
            Poste::Ecs,
            Energie::Electricite,
            w_pompes_solaires - w_pompes_solaires_ch,
        );
        q_cef.ajouter(Poste::Chauffage, Energie::Electricite, w_pompes_solaires_ch);
        if !self.en_volume_chauffe {
            phi_vc = 0.0;
        }
        // Éq. (1973) : pertes des tuyauteries des boucles solaires « vers l'intérieur du
        // bâtiment », récupérables quel que soit l'emplacement du ballon (lecture Q52).
        phi_vc += phi_vc_boucles;
        // Éq. (2114) : ballons individuels d'un CESCAI, toujours en volume chauffé.
        phi_vc += phi_pertes_dec_vc;
        self.cumuls.phi_pertes_sto_wh += phi_pertes_dec_vc;
        // Éq. (1059) à (1064) : sous-dimensionnement. Les indicateurs « ne concernent que les
        // générateurs (le sous-dimensionnement des ballons de stockage est évalué séparément) » :
        // en ECS, seul compte ce que les générateurs instantanés n'ont pas fourni ; le reste dû
        // aux ballons (éq. 1953) relève de l'alerte du ballon (éq. 1780).
        let q_rep_ecs_generateurs = if instantanes_ecs.is_empty() {
            0.0
        } else {
            q_rep[2]
        };
        let n = &mut self.etat.nb_sousdim;
        n[0] = if (q_rep[0] > 0.0 || q_rep_ecs_generateurs > 0.0) && !relance[0] {
            n[0] + 1
        } else {
            0
        };
        n[1] = if q_rep[1] > 0.0 { n[1] + 1 } else { 0 };
        for (m, x) in self.cumuls.nb_sousdim_max.iter_mut().zip(n.iter()) {
            *m = (*m).max(*x);
        }
        self.etat.q_rep_wh = q_rep;
        self.etat.theta_aval_prec_c = [theta_aval[0], theta_aval[1]];

        // Fiche 8.29 : cumuls de la génération et des générateurs.
        self.cumuls.cef.ajouter(mois, &q_cef);
        self.cumuls.phi_vc_tot_wh += phi_vc;
        for p in 0..3 {
            self.cumuls.q_req_mensuel_wh[p][mois] += q_sr[p];
            self.cumuls.q_fou_wh[p] += q_fou[p];
        }
        let actifs = [e.aut_ch, e.aut_fr, e.id_encl];
        for g in &mut self.generateurs {
            g.cumuls.cef.ajouter(mois, &g.jeu.q_cef);
            g.cumuls.w_aux_wh += g.jeu.w_aux_wh;
            g.cumuls.phi_vc_wh += g.jeu.phi_vc_wh;
            g.cumuls.phi_rejet_wh += g.jeu.phi_rejet_wh;
            let f = g.modele.fonction();
            for poste in Poste::TOUS {
                let p = poste.indice();
                g.cumuls.q_fou_wh[p] += g.jeu.q_fou_wh[p];
                if !f.assure(poste) {
                    continue;
                }
                // Éq. (1522) à (1524).
                if actifs[p] {
                    g.cumuls.nbh_charge[p][classe_charge(g.jeu.tau_charge_poste[p])] += 1;
                } else {
                    g.cumuls.nbh_hors_fonctionnement[p] += 1;
                }
            }
        }
        for g in &mut self.generateurs {
            g.modele.fin_de_pas();
        }
        self.sorties = SortiesGeneration {
            q_cef,
            phi_vc_tot_wh: phi_vc,
            q_req_tot_sr_wh: q_sr,
            q_req_tot_ar_wh: q_ar,
            q_fou_wh: q_fou,
            q_rep_wh: q_rep,
            rat_bes_dp,
            theta_aval_c: theta_aval,
            theta_amb_c: theta_amb,
            relance,
        };
        &self.sorties
    }

    /// Température de fonctionnement en chauffage ou en refroidissement (§8.17.3.5.10).
    fn theta_aval(
        &self,
        poste: Poste,
        e: &EntreesGeneration<'_>,
        rat_bes_dp: &[f64],
        relance: bool,
        q_sr: f64,
        q_ar: f64,
    ) -> f64 {
        let froid = poste == Poste::Refroidissement;
        let du_poste = || {
            e.reseaux
                .iter()
                .zip(rat_bes_dp)
                .filter(move |(r, _)| r.poste == poste)
        };
        if du_poste().any(|(r, _)| r.hydraulique) {
            // Éq. (1010) et (1011) : réseaux hydrauliques.
            let (gestion, extreme, prec) = if froid {
                (
                    self.gestion_fr,
                    self.theta_dist_fr_min_c,
                    self.etat.theta_aval_prec_c[1],
                )
            } else {
                (
                    self.gestion_ch,
                    self.theta_dist_ch_max_c,
                    self.etat.theta_aval_prec_c[0],
                )
            };
            let des_reseaux = || {
                let t = du_poste()
                    .filter(|(r, _)| r.hydraulique)
                    .map(|(r, _)| r.theta_moy_c);
                // Lecture Q46 : le réseau le plus exigeant, le plus chaud en chauffage et le
                // plus froid en refroidissement.
                if froid {
                    t.reduce(f64::min)
                } else {
                    t.reduce(f64::max)
                }
                .unwrap_or(prec)
            };
            match gestion {
                GestionTemperature::Constante(t) => t,
                GestionTemperature::Reseaux => {
                    if relance {
                        // Relance : fonctionnement à la température extrême des réseaux.
                        extreme.unwrap_or_else(des_reseaux)
                    } else if q_sr == 0.0 && q_ar > 0.0 {
                        // Demande reportée seule : température du pas précédent.
                        prec
                    } else {
                        des_reseaux()
                    }
                }
            }
        } else {
            // Éq. (1012) et (1013) : génération sur air ambiant.
            du_poste().map(|(r, rat)| rat * r.theta_i_aval_eq_c).sum()
        }
    }

    /// Appelle les générateurs d'un poste selon le mode de gestion des priorités et renvoie
    /// l'énergie reportée « Qrep » (éq. 1028 et 1034).
    fn appeler_generateurs(
        &mut self,
        ctx: &ContexteAppel,
        q_req_tot_ar_wh: f64,
        sources: &[ConditionsSource],
    ) -> f64 {
        let poste = ctx.poste;
        let mut indices = self.generateurs_du_poste(poste);
        if indices.is_empty() {
            return q_req_tot_ar_wh;
        }
        // §8.17.3.5.6 : en cascade, pendant une relance de chauffage, le générateur de rang le
        // plus élevé n'est pas pris en compte (lecture Q46 : s'il y en a au moins deux).
        if poste == Poste::Chauffage
            && self.priorite == ModePriorite::Cascade
            && ctx.relance_ch
            && indices.len() > 1
        {
            indices.pop();
        }
        match self.priorite {
            ModePriorite::SansPriorite => {
                // Éq. (1017) et (1018) : clés de répartition ; éq. (1020), (1023) et (1026) :
                // les générateurs qui ont refroidi sont écartés du chauffage et de l'ECS.
                let cle = |g: &GenerateurHoraire| {
                    let dispo = poste == Poste::Refroidissement || !g.jeu.ia_refroidi;
                    if dispo {
                        g.modele.pn_gen_tot_w(poste)
                    } else {
                        0.0
                    }
                };
                let total: f64 = indices.iter().map(|&i| cle(&self.generateurs[i])).sum();
                let mut fourni = 0.0;
                for &i in &indices {
                    let q_req = if total > 0.0 {
                        cle(&self.generateurs[i]) / total * q_req_tot_ar_wh
                    } else {
                        0.0
                    };
                    if let Some(s) = self.appeler(i, ctx, q_req, sources) {
                        fourni += s.q_fou_wh;
                    }
                }
                // Éq. (1028).
                q_req_tot_ar_wh - fourni
            }
            ModePriorite::Cascade => {
                // Éq. (1029) à (1034).
                let mut q_req = q_req_tot_ar_wh;
                for &i in &indices {
                    if let Some(s) = self.appeler(i, ctx, q_req, sources) {
                        q_req = s.q_rest_wh;
                    }
                }
                q_req
            }
            ModePriorite::Alternance => {
                // §8.17.3.7.2.3.1 à 8.17.3.7.2.3.3, éq. (1035) à (1046) : génération monoposte,
                // générateurs par puissance nominale décroissante.
                let mut q_req = q_req_tot_ar_wh;
                for (k, &i) in indices.iter().enumerate() {
                    let p_suivant = puissance_suivante(&self.generateurs, &indices, k, poste);
                    let g = &mut self.generateurs[i];
                    g.n_basculement = compteur_basculement(g.n_basculement, q_req, p_suivant);
                    if g.n_basculement == 0 {
                        continue;
                    }
                    if let Some(s) = self.appeler(i, ctx, q_req, sources) {
                        q_req = s.q_rest_wh;
                    }
                }
                q_req
            }
        }
    }

    /// Vrai si la génération est gérée en alternance avec des générateurs « chauffage et
    /// ECS » (« idfougen = 4 », §8.17.3.7.2.3.4) : la demande totale des deux postes décide des
    /// générateurs en service, en un seul passage.
    fn alternance_biposte(&self) -> bool {
        self.priorite == ModePriorite::Alternance
            && !self.generateurs.is_empty()
            && self
                .generateurs
                .iter()
                .all(|g| g.modele.fonction() == FonctionGenerateur::ChauffageEcs)
    }

    /// §8.17.3.7.2.3.4, éq. (1047) à (1052) : alternance d'une génération « chauffage et ECS ».
    /// Renvoie les énergies reportées `[Qrep_ch, Qrep_ecs]`.
    fn alternance_chauffage_ecs(
        &mut self,
        ctx_ch: &ContexteAppel,
        ctx_ecs: &ContexteAppel,
        q_ar: [f64; 3],
        id_encl: bool,
        sources: &[ConditionsSource],
    ) -> [f64; 2] {
        // Éq. (1047) à (1050).
        let mut q_req_ch = q_ar[0];
        let mut q_req_ecs = q_ar[2];
        // Hors saison de chauffage et hors période d'activation de l'ECS, les générateurs sont
        // désactivés ; les compteurs ne changent pas (lecture Q125, point 4).
        if !(ctx_ch.aut_ch || id_encl) {
            return [q_req_ch, q_req_ecs];
        }
        let indices = self.generateurs_du_poste(Poste::Chauffage);
        for (k, &i) in indices.iter().enumerate() {
            let p_suivant = puissance_suivante(&self.generateurs, &indices, k, Poste::Chauffage);
            let g = &mut self.generateurs[i];
            g.n_basculement =
                compteur_basculement(g.n_basculement, q_req_ch + q_req_ecs, p_suivant);
            if g.n_basculement == 0 {
                continue;
            }
            // Partie ECS : `Rpui_dispo = 1`, `iecs_seule = 1` hors saison de chauffage ; appel
            // dans la période d'activation de l'ECS, s'il y a une demande ou si le générateur
            // ne fait que de l'ECS au pas de temps (lecture Q125, point 6, comme Q46 point 6).
            if id_encl {
                if let Some(s) = self.appeler(i, ctx_ecs, q_req_ecs, sources) {
                    q_req_ecs = s.q_rest_wh;
                }
            }
            // Partie chauffage : `Rpui_dispo = 1 − Rfonct_ecs`, si `iecs_seule = 0` (en saison
            // de chauffage) ; `appeler` ne fait rien si la puissance disponible est nulle.
            if ctx_ch.aut_ch {
                if let Some(s) = self.appeler(i, ctx_ch, q_req_ch, sources) {
                    q_req_ch = s.q_rest_wh;
                }
            }
        }
        // Éq. (1052).
        [q_req_ch, q_req_ecs]
    }

    /// Appelle le générateur d'indice `i` pour le compte d'un ballon de stockage d'ECS
    /// (§9.13) et renvoie l'énergie fournie au ballon, en Wh. Comme en production instantanée
    /// (§8.17.3.7.2), le générateur est appelé s'il y a une demande ou s'il ne produit que de
    /// l'ECS au pas de temps, sa consommation à charge nulle étant alors comptée en ECS.
    fn appeler_pour_ballon(
        &mut self,
        i: usize,
        poste: Poste,
        q_req_wh: f64,
        theta_aval_c: f64,
        theta_amb_c: f64,
        e: &EntreesGeneration<'_>,
    ) -> f64 {
        let Some(g) = self.generateurs.get_mut(i) else {
            return 0.0;
        };
        if poste == Poste::Chauffage {
            // Appoint raccordé d'un système solaire combiné (éq. 2150) : appel de chauffage
            // d'un générateur instantané, après l'ECS (éq. 1293).
            if !e.aut_ch || g.jeu.ia_refroidi {
                return 0.0;
            }
            let r_pui_dispo = 1.0 - g.jeu.r_fonct_ecs;
            if r_pui_dispo <= 0.0 {
                return 0.0;
            }
            let source = e.sources.get(i).copied().unwrap_or_default();
            let s = g.modele.appel(&EntreesAppel {
                fonction: Poste::Chauffage,
                q_req_wh,
                theta_amont_c: source.theta_amont_c,
                theta_aval_c,
                r_pui_dispo,
                ecs_seule: false,
                theta_amb_c,
                air_extrait_w_k: source.air_extrait_w_k,
            });
            g.jeu.ajouter(Poste::Chauffage, r_pui_dispo, &s);
            return s.q_fou_wh;
        }
        let ecs_seule = g.modele.fonction() == FonctionGenerateur::Ecs || !e.aut_ch;
        if !(q_req_wh > 0.0 || ecs_seule) {
            return 0.0;
        }
        let source = e.sources.get(i).copied().unwrap_or_default();
        let s = g.modele.appel(&EntreesAppel {
            fonction: Poste::Ecs,
            q_req_wh,
            theta_amont_c: source.theta_amont_c,
            theta_aval_c,
            r_pui_dispo: 1.0,
            ecs_seule,
            theta_amb_c,
            air_extrait_w_k: source.air_extrait_w_k,
        });
        g.jeu.ajouter(Poste::Ecs, 1.0, &s);
        s.q_fou_wh
    }

    /// Appelle le générateur d'indice `i` pour un poste, si les conditions du texte le
    /// permettent, et met à jour son jeu de données.
    fn appeler(
        &mut self,
        i: usize,
        ctx: &ContexteAppel,
        q_req_wh: f64,
        sources: &[ConditionsSource],
    ) -> Option<SortiesAppel> {
        let g = &mut self.generateurs[i];
        let fonction = g.modele.fonction();
        let source = sources.get(i).copied().unwrap_or_default();
        let (r_pui_dispo, ecs_seule) = match ctx.poste {
            Poste::Ecs => {
                // Le générateur n'est disponible que s'il n'a pas refroidi.
                if g.jeu.ia_refroidi {
                    return None;
                }
                // « iecs_seule » : générateur d'ECS seule, ou hors saison de chauffage.
                let ecs_seule = fonction == FonctionGenerateur::Ecs || !ctx.aut_ch;
                if !(q_req_wh > 0.0 || ecs_seule) {
                    return None;
                }
                (1.0, ecs_seule)
            }
            Poste::Refroidissement => {
                // La demande est non nulle, ou la consommation à charge nulle du générateur est
                // comptée en refroidissement (hors saison de chauffage, ou générateur non
                // réversible).
                let appel = q_req_wh > 0.0
                    || fonction != FonctionGenerateur::ChauffageRefroidissement
                    || !ctx.aut_ch;
                if !appel {
                    return None;
                }
                if q_req_wh > 0.0 {
                    g.jeu.ia_refroidi = true;
                }
                (1.0 - g.jeu.r_fonct_ecs, false)
            }
            Poste::Chauffage => {
                if g.jeu.ia_refroidi {
                    return None;
                }
                (1.0 - g.jeu.r_fonct_ecs, false)
            }
        };
        if r_pui_dispo <= 0.0 {
            return None;
        }
        let s = g.modele.appel(&EntreesAppel {
            fonction: ctx.poste,
            q_req_wh,
            theta_amont_c: source.theta_amont_c,
            theta_aval_c: ctx.theta_aval_c,
            r_pui_dispo,
            ecs_seule,
            theta_amb_c: ctx.theta_amb_c,
            air_extrait_w_k: source.air_extrait_w_k,
        });
        g.jeu.ajouter(ctx.poste, r_pui_dispo, &s);
        Some(s)
    }
}

/// Contexte commun aux appels des générateurs d'un poste à un pas de temps.
#[derive(Debug, Clone, Copy)]
struct ContexteAppel {
    poste: Poste,
    theta_aval_c: f64,
    theta_amb_c: f64,
    aut_ch: bool,
    relance_ch: bool,
}

/// « Rdim × Pngen^(gnr+1) » : puissance installée du générateur qui suit le `k`-ième dans
/// l'ordre de l'alternance, en W ; nulle pour le dernier, qui n'a pas de suivant (lecture
/// Q125, points 2 et 3).
fn puissance_suivante(
    generateurs: &[GenerateurHoraire],
    indices: &[usize],
    k: usize,
    poste: Poste,
) -> f64 {
    indices
        .get(k + 1)
        .map_or(0.0, |&j| generateurs[j].modele.pn_gen_tot_w(poste))
}

/// Compteur de basculement marche/arrêt d'un générateur en alternance (§8.17.3.7.2.3) :
/// décrémenté (sans descendre sous zéro) quand la demande restante tient dans la puissance
/// du générateur suivant (« le générateur est surdimensionné par rapport à la demande »),
/// réinitialisé à `Nbasculement_init` sinon. Le générateur est activé si le compteur est
/// positif. Demande en Wh sur un pas d'une heure, puissance en W.
pub(super) fn compteur_basculement(n_prec: u32, q_req_wh: f64, p_suivant_w: f64) -> u32 {
    if q_req_wh <= p_suivant_w {
        n_prec.saturating_sub(1)
    } else {
        N_BASCULEMENT_INIT
    }
}
