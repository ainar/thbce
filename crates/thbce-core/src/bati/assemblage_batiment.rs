// SPDX-License-Identifier: GPL-3.0-or-later
//! Enchaînement horaire d'un bâtiment : espaces tampons puis zones.
//!
//! Un espace tampon est défini au niveau du bâtiment et peut toucher des groupes de plusieurs
//! zones (§5.4.1). À chaque pas de temps, dans l'ordre de la figure 15 (§5.2.2) : les espaces
//! tampons sont calculés à partir de l'état des groupes à la fin du pas précédent (`θi,fin(h−1)`,
//! `θop,fin(h−1)`, saisons) et de leur occupation au pas courant ; leurs coefficients
//! `btherm(h)`, `bsolaire(h)`, `becl(h)` et leur température `θet(h)` sont ensuite transmis aux
//! composants d'enveloppe et aux entrées d'air des zones (§5.5, §5.7), calculées comme avant.

use crate::chauffage_refroidissement::distribution::{
    ApportReseauGroupe, DistributionIntergroupesResolue, SortiesDistributionIntergroupes,
};
use crate::chauffage_refroidissement::saisons_fonctionnement::{
    autorisations_effectives, DesserteGroupe,
};
use crate::chauffage_refroidissement::systemes_groupe::Liens;
use crate::climat::calendrier::{date_scenario, jour_bascule_saisons};
use crate::climat::environnement_proche::ventc_m_s;
use crate::climat::{ClimatExterieur, ConfortAdaptatif};
use crate::conventions::{ModeCalcul, NB_HEURES_AN};
use crate::ecs::distribution_ecs::{pas_intergroupe, SortiesDistributionEcsIntergroupe};
use crate::entree::{Batiment, Catalogue};
use crate::entree::{FonctionReseau, RaccordementReseaux, ReseauEcsIntergroupe};
use crate::erreur::Erreur;
use crate::pertes::pertes_recuperables::{recuperation_wh, FluxVolumeChauffe};

use super::assemblage_groupe::Lectures;
use super::assemblage_zone::{conditions_exterieures, ZoneHoraire};
use super::espace_tampon::assemblage::{EspaceTamponHoraire, GroupeVuParEt, LienGroupe};
use super::espace_tampon::espace_tampon::CoefficientsEt;

/// Bâtiment prêt pour un calcul annuel au pas horaire, en mode Th-B ou Th-D.
#[derive(Debug, Clone, PartialEq)]
pub struct BatimentHoraire {
    /// Identifiant du bâtiment.
    pub id: String,
    /// Mode de calcul.
    pub mode: ModeCalcul,
    /// Zones du bâtiment.
    pub zones: Vec<ZoneHoraire>,
    /// Espaces tampons du bâtiment, dans l'ordre de la saisie.
    pub espaces_tampons: Vec<EspaceTamponHoraire>,
    /// Coefficients des espaces tampons au dernier pas de temps calculé.
    pub coefficients: Vec<CoefficientsEt>,
    vus: Vec<GroupeVuParEt>,
    /// Systèmes de chauffage et de refroidissement du bâtiment en aval des groupes : réseaux
    /// intergroupes et générations (§8.4, §8.6, §8.9 et §8.10) ; vides en Th-B.
    pub systemes: SystemesBatiment,
}

/// Réseaux intergroupes et générations d'un bâtiment, et leurs sorties.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SystemesBatiment {
    /// Identifiants des générations.
    pub generations: Vec<String>,
    /// « idraccord_reseau » de chaque génération : vrai si permanent.
    pub permanent: Vec<bool>,
    /// Réseaux intergroupes.
    pub intergroupes: Vec<DistributionIntergroupesResolue>,
    /// Desserte de chaque groupe, dans l'ordre des zones puis des groupes.
    pub dessertes: Vec<DesserteGroupe>,
    /// « Ratsurf^dp,gr » : pour chaque réseau intergroupes, part de chaque groupe (même ordre).
    pub rat_surf: Vec<Vec<f64>>,
    /// Sorties de chaque réseau intergroupes au dernier pas de temps.
    pub sorties: Vec<SortiesDistributionIntergroupes>,
    /// « Autch^gen » et « Autfr^gen » de chaque génération.
    pub aut_gen: Vec<(bool, bool)>,
    /// Cumuls de chaque réseau intergroupes.
    pub cumuls: Vec<CumulsIntergroupes>,
    /// Distributions d'ECS intergroupes (§9.8).
    pub ecs: Vec<EcsIntergroupeResolu>,
    /// « Ratsurfaces_prim_e » : pour chaque distribution d'ECS intergroupe, part de chaque
    /// groupe (éq. 1737).
    pub rat_surf_ecs: Vec<Vec<f64>>,
    /// Sorties de chaque distribution d'ECS intergroupe au dernier pas de temps.
    pub sorties_ecs: Vec<SortiesDistributionEcsIntergroupe>,
    /// Cumuls de chaque distribution d'ECS intergroupe.
    pub cumuls_ecs: Vec<CumulsEcsIntergroupe>,
    /// Récupération des pertes et consommations des systèmes par les groupes (fiche 11.1) ;
    /// vrai sauf pour mesurer son effet (lecture Q45).
    pub recuperation: bool,
}

/// Distribution d'ECS intergroupe, prête pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct EcsIntergroupeResolu {
    /// Identifiant.
    pub id: String,
    /// Indice de la génération.
    pub generation: usize,
    /// Nature du réseau.
    pub reseau: ReseauEcsIntergroupe,
    /// Indice de l'espace tampon traversé.
    pub espace_tampon: Option<usize>,
}

/// Cumuls annuels d'une distribution d'ECS intergroupe, en Wh.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CumulsEcsIntergroupe {
    /// « Qw_prim-e » par mois des scénarios : besoins transmis à la génération.
    pub q_w_prim_mensuel_wh: [f64; 12],
    /// Pertes en volume chauffé.
    pub phi_pertes_vc_wh: f64,
    /// Pertes hors volume chauffé.
    pub phi_pertes_hvc_wh: f64,
    /// Circulateurs ou traceurs.
    pub w_aux_wh: f64,
    /// Réchauffeur de boucle.
    pub w_rechauf_wh: f64,
    /// Plus haute température de départ demandée, en °C.
    pub theta_depart_max_c: Option<f64>,
}

/// Cumuls annuels d'un réseau intergroupes : demande transmise à la génération.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CumulsIntergroupes {
    /// Demande de chauffage, par mois des scénarios, en Wh.
    pub q_ch_mensuel_wh: [f64; 12],
    /// Demande de refroidissement (valeur absolue), par mois, en Wh.
    pub q_fr_mensuel_wh: [f64; 12],
    /// Consommation des circulateurs, en Wh.
    pub w_aux_wh: f64,
    /// Pertes vers le volume chauffé, en Wh.
    pub phi_pertes_vc_wh: f64,
    /// Pertes hors volume chauffé, en Wh.
    pub phi_pertes_hvc_wh: f64,
    /// Nombre de pas de temps où le réseau débite ou transmet une demande.
    pub nbh_fonct: u32,
    /// Plus haute température de départ demandée (chauffage) ou plus basse (froid), en °C ;
    /// `None` pour un réseau fictif ou sans demande.
    pub theta_dep_extreme_c: Option<f64>,
}

impl BatimentHoraire {
    /// Prépare un bâtiment validé, avec les lectures retenues du texte.
    pub fn nouveau(
        bat: &Batiment,
        catalogue: &Catalogue,
        mode: ModeCalcul,
        alt_m: f64,
        omega_initiale_kg_kg: f64,
    ) -> Result<Self, Erreur> {
        Self::nouveau_selon(
            bat,
            catalogue,
            mode,
            Lectures::default(),
            alt_m,
            omega_initiale_kg_kg,
        )
    }

    /// Prépare un bâtiment validé pour des lectures données du texte.
    pub fn nouveau_selon(
        bat: &Batiment,
        catalogue: &Catalogue,
        mode: ModeCalcul,
        lectures: Lectures,
        alt_m: f64,
        omega_initiale_kg_kg: f64,
    ) -> Result<Self, Erreur> {
        let ids: Vec<String> = bat.espaces_tampons.iter().map(|e| e.id.clone()).collect();
        let mut zones = bat
            .zones
            .iter()
            .map(|z| {
                ZoneHoraire::nouvelle_dans_batiment(
                    z,
                    catalogue,
                    mode,
                    lectures,
                    alt_m,
                    omega_initiale_kg_kg,
                    &ids,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        // §5.4.3.1, éq. (106) et (107) : coefficients des groupes vers chaque espace tampon,
        // calculés en début de simulation.
        let mut totaux_ouverture = vec![0.0; ids.len()];
        let espaces_tampons = bat
            .espaces_tampons
            .iter()
            .enumerate()
            .map(|(i, et)| {
                let mut liens = Vec::new();
                for (iz, z) in zones.iter().enumerate() {
                    for (ig, g) in z.groupes.iter().enumerate() {
                        let h = g.enveloppe.h_et_th_int_w_k(i);
                        let touche = g
                            .enveloppe
                            .parois
                            .iter()
                            .any(|p| p.espace_tampon == Some(i))
                            || g.enveloppe.ponts.iter().any(|p| p.espace_tampon == Some(i))
                            || g.enveloppe.baies.iter().any(|b| b.espace_tampon == Some(i));
                        if touche {
                            liens.push(LienGroupe {
                                zone: iz,
                                groupe: ig,
                                h_et_th_int_w_k: h,
                            });
                        }
                        totaux_ouverture[i] += g
                            .enveloppe
                            .baies
                            .iter()
                            .filter(|b| b.espace_tampon == Some(i))
                            .map(|b| b.a_ouv_max_m2)
                            .sum::<f64>();
                    }
                }
                EspaceTamponHoraire::nouveau(et, catalogue, liens, lectures.fermeture_et)
            })
            .collect::<Vec<_>>();
        for z in &mut zones {
            for g in &mut z.groupes {
                g.fixer_parts_ouverture_espaces_tampons(&totaux_ouverture);
            }
        }
        let coefficients = espaces_tampons.iter().map(|e| e.coefficients).collect();
        let systemes = if mode == ModeCalcul::ThB {
            SystemesBatiment::default()
        } else {
            let mut s = SystemesBatiment::nouveaux(bat, &mut zones);
            s.recuperation = mode != ModeCalcul::ThD || lectures.recuperation_th_d;
            s
        };
        Ok(Self {
            systemes,
            id: bat.id.clone(),
            mode,
            zones,
            espaces_tampons,
            coefficients,
            vus: Vec::new(),
        })
    }

    /// Calcule le pas de temps `h` du bâtiment : espaces tampons, puis zones.
    pub fn pas(
        &mut self,
        h: usize,
        climat: &ClimatExterieur,
        confort: &ConfortAdaptatif,
    ) -> Result<(), Erreur> {
        if !self.espaces_tampons.is_empty() {
            let ext = conditions_exterieures(climat, confort, h, self.mode);
            let v_ventc = ventc_m_s(climat.vent_m_s[h]);
            let i_occ: Vec<f64> = self.zones.iter().map(|z| z.i_occ_zone(h)).collect();
            for (i, et) in self.espaces_tampons.iter_mut().enumerate() {
                self.vus.clear();
                self.vus.extend(et.liens.iter().map(|l| {
                    let g = &self.zones[l.zone].groupes[l.groupe];
                    GroupeVuParEt {
                        theta_i_fin_c: g.etat.fin.theta_i_c,
                        theta_op_fin_c: g.etat.fin.theta_op_c,
                        i_occ: i_occ[l.zone],
                        aut_ch: g.etat.saisons.aut_ch,
                        aut_fr: g.etat.saisons.aut_fr,
                        hebergement: g.habitation,
                    }
                }));
                self.coefficients[i] = et.pas(&ext, v_ventc, &self.vus);
            }
        }
        for z in &mut self.zones {
            z.pas_avec_espaces_tampons(h, climat, confort, &self.coefficients)?;
        }
        if self.mode != ModeCalcul::ThB {
            self.systemes.pas(
                h,
                &mut self.zones,
                climat.te_c[h],
                climat.teau_c[h],
                &self.coefficients,
            );
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

    /// Calcule les 8 760 pas de temps de l'année et appelle `apres_le_pas(h, batiment)` à la
    /// fin de chacun.
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

impl SystemesBatiment {
    /// Résout les réseaux intergroupes et les générations du bâtiment et lie les systèmes des
    /// groupes. Références vérifiées par la validation.
    fn nouveaux(bat: &crate::entree::Batiment, zones: &mut [ZoneHoraire]) -> Self {
        let generations: Vec<String> = bat.generations.iter().map(|g| g.id.clone()).collect();
        let ids_et: Vec<String> = bat.espaces_tampons.iter().map(|e| e.id.clone()).collect();
        let permanent = bat
            .generations
            .iter()
            .map(|g| g.raccordement_reseaux == RaccordementReseaux::Permanent)
            .collect();
        let mut intergroupes: Vec<DistributionIntergroupesResolue> = bat
            .distributions_intergroupes
            .iter()
            .map(|d| {
                DistributionIntergroupesResolue::nouvelle(
                    d,
                    |id| generations.iter().position(|g| g == id),
                    |id| ids_et.iter().position(|e| e == id),
                )
            })
            .collect();
        let ids_dp: Vec<String> = intergroupes.iter().map(|d| d.id.clone()).collect();
        let ids_ecs: Vec<String> = bat
            .distributions_ecs_intergroupes
            .iter()
            .map(|d| d.id.clone())
            .collect();
        let mut a_ecs = vec![Vec::new(); ids_ecs.len()];
        let mut dessertes = Vec::new();
        let mut a_desservie = vec![Vec::new(); intergroupes.len()];
        for (z, zone) in zones.iter_mut().zip(&bat.zones) {
            for (g, groupe) in z.groupes.iter_mut().zip(&zone.groupes) {
                g.lier_systemes(
                    groupe,
                    zone.usage,
                    &Liens {
                        intergroupes: &ids_dp,
                        intergroupes_ecs: &ids_ecs,
                        espaces_tampons: &ids_et,
                    },
                );
                let mut surf_ecs = vec![0.0; ids_ecs.len()];
                if let Some(sy) = &g.systemes {
                    for e in &sy.ecs {
                        if let Some(s) = surf_ecs.get_mut(e.intergroupe) {
                            // Éq. (1737) et (1738).
                            *s += e.besoins.a_m2;
                        }
                    }
                }
                for (a, s) in a_ecs.iter_mut().zip(surf_ecs) {
                    a.push(s);
                }
                if let Some(sy) = &g.systemes {
                    for d in &sy.distributions {
                        if let Some(dp) = intergroupes.get_mut(d.intergroupes) {
                            dp.ajouter_reseau_groupe(&d.reseau);
                        }
                    }
                }
                let mut d = DesserteGroupe {
                    i_clim: g.i_clim,
                    ..DesserteGroupe::default()
                };
                let mut surf = vec![0.0; intergroupes.len()];
                if let Some(sy) = &g.systemes {
                    for (k, e) in sy.emetteurs.iter().enumerate() {
                        for (f, rat, liste) in [
                            (e.chaud, sy.equivalente.rat_eff_ch[k], &mut d.chaud),
                            (e.froid, sy.equivalente.rat_eff_fr[k], &mut d.froid),
                        ] {
                            let Some(f) = f else { continue };
                            let Some(ds) = sy.distributions.get(f.distribution) else {
                                continue;
                            };
                            let dp = ds.intergroupes;
                            if let Some(dpr) = intergroupes.get(dp) {
                                if !liste.contains(&dpr.generation) {
                                    liste.push(dpr.generation);
                                }
                                // Éq. (890) : surface desservie, Σ Rateff × Agr.
                                surf[dp] += rat * g.a_gr_m2;
                            }
                        }
                    }
                }
                for (a, s) in a_desservie.iter_mut().zip(surf) {
                    a.push(s);
                }
                dessertes.push(d);
            }
        }
        let ratios = |a: &[Vec<f64>]| -> Vec<Vec<f64>> {
            a.iter()
                .map(|a| {
                    let total: f64 = a.iter().sum();
                    a.iter()
                        .map(|x| if total > 0.0 { x / total } else { 0.0 })
                        .collect()
                })
                .collect()
        };
        let ecs = bat
            .distributions_ecs_intergroupes
            .iter()
            .map(|d| EcsIntergroupeResolu {
                id: d.id.clone(),
                generation: generations
                    .iter()
                    .position(|g| *g == d.generation)
                    .unwrap_or(0),
                espace_tampon: match &d.reseau {
                    ReseauEcsIntergroupe::Boucle(b) => b.espace_tampon.as_deref(),
                    ReseauEcsIntergroupe::Trace(t) => t.espace_tampon.as_deref(),
                    ReseauEcsIntergroupe::Aucun => None,
                }
                .and_then(|id| ids_et.iter().position(|e| e == id)),
                reseau: d.reseau.clone(),
            })
            .collect::<Vec<_>>();
        let n_ecs = ecs.len();
        let rat_surf_ecs = ratios(&a_ecs);
        // Éq. (891) et (892).
        let rat_surf = a_desservie
            .iter()
            .map(|a| {
                let total: f64 = a.iter().sum();
                a.iter()
                    .map(|x| if total > 0.0 { x / total } else { 0.0 })
                    .collect()
            })
            .collect();
        let n = intergroupes.len();
        let mut s = Self {
            aut_gen: vec![(false, false); generations.len()],
            generations,
            permanent,
            intergroupes,
            dessertes,
            rat_surf,
            sorties: vec![SortiesDistributionIntergroupes::default(); n],
            cumuls: vec![CumulsIntergroupes::default(); n],
            ecs,
            rat_surf_ecs,
            sorties_ecs: vec![SortiesDistributionEcsIntergroupe::default(); n_ecs],
            cumuls_ecs: vec![CumulsEcsIntergroupe::default(); n_ecs],
            recuperation: true,
        };
        s.fixer_autorisations(zones);
        s
    }

    /// Autorisations effectives des groupes (fiche 8.4, éq. 839 à 841), à partir de leurs
    /// autorisations propres.
    fn fixer_autorisations(&mut self, zones: &mut [ZoneHoraire]) {
        let propres: Vec<(bool, bool)> = zones
            .iter()
            .flat_map(|z| z.groupes.iter())
            .map(|g| (g.etat.saisons.aut_ch, g.etat.saisons.aut_fr))
            .collect();
        let (eff, gen) = autorisations_effectives(&propres, &self.dessertes, &self.permanent);
        self.aut_gen = gen;
        for (g, a) in zones.iter_mut().flat_map(|z| z.groupes.iter_mut()).zip(eff) {
            g.etat.aut_eff = a;
        }
    }

    /// Pas de temps des réseaux intergroupes, après celui des groupes ; à la bascule des
    /// saisons (9 h légales, décision Q39), nouvelles autorisations effectives (lecture Q43).
    fn pas(
        &mut self,
        h: usize,
        zones: &mut [ZoneHoraire],
        theta_ext_c: f64,
        theta_cw_c: f64,
        espaces_tampons: &[CoefficientsEt],
    ) {
        let mois = usize::from(date_scenario(h).mois) - 1;
        let groupes: Vec<_> = zones.iter().flat_map(|z| z.groupes.iter()).collect();
        let mut aval = Vec::new();
        for (k, dp) in self.intergroupes.iter().enumerate() {
            aval.clear();
            for (ig, g) in groupes.iter().enumerate() {
                let Some(sy) = &g.systemes else { continue };
                for (d, s) in sy.distributions.iter().zip(&sy.sorties.distributions) {
                    if d.intergroupes == k {
                        aval.push(ApportReseauGroupe {
                            sorties: *s,
                            rat_surf: self.rat_surf[k].get(ig).copied().unwrap_or(0.0),
                            theta_i_moy_c: sy.sorties.theta_i_moy_c,
                        });
                    }
                }
            }
            // Éq. (924) et (932) : `btherm` de l'espace tampon traversé, 1 à l'extérieur.
            let b_therm = dp
                .espace_tampon
                .and_then(|i| espaces_tampons.get(i))
                .map_or(1.0, |e| e.b_therm);
            let s = dp.pas(&aval, theta_ext_c, b_therm);
            let c = &mut self.cumuls[k];
            c.q_ch_mensuel_wh[mois] += s.q_sys_ch_wh;
            c.q_fr_mensuel_wh[mois] -= s.q_sys_fr_wh;
            c.w_aux_wh += s.w_aux_wh;
            c.phi_pertes_vc_wh += s.phi_pertes_vc_wh;
            c.phi_pertes_hvc_wh += s.phi_pertes_hvc_wh;
            let demande = s.q_sys_ch_wh != 0.0 || s.q_sys_fr_wh != 0.0;
            c.nbh_fonct += u32::from(demande);
            if demande && s.theta_dep_c != 0.0 {
                let t = s.theta_dep_c;
                c.theta_dep_extreme_c = Some(match (c.theta_dep_extreme_c, dp.fonction) {
                    (None, _) => t,
                    (Some(x), FonctionReseau::Chauffage) => x.max(t),
                    (Some(x), FonctionReseau::Refroidissement) => x.min(t),
                });
            }
            self.sorties[k] = s;
        }
        // §9.8 : distributions d'ECS intergroupes ; génération idéale, en fonctionnement toute
        // l'année (« idencl-e(j) = 1 », éq. 1736 ; lecture Q45).
        for (k, d) in self.ecs.iter().enumerate() {
            let aval: Vec<(f64, f64)> = groupes
                .iter()
                .filter_map(|g| g.systemes.as_ref())
                .flat_map(|sy| sy.ecs.iter())
                .filter(|e| e.intergroupe == k)
                .map(|e| (e.sorties.q_w_2nd_wh, e.distribution.theta_2nd_e_c))
                .collect();
            let b_therm = d
                .espace_tampon
                .and_then(|i| espaces_tampons.get(i))
                .map_or(1.0, |e| e.b_therm);
            let s = pas_intergroupe(&d.reseau, &aval, true, theta_cw_c, theta_ext_c, b_therm);
            let c = &mut self.cumuls_ecs[k];
            c.q_w_prim_mensuel_wh[mois] += s.q_w_prim_wh;
            c.phi_pertes_vc_wh += s.phi_pertes_vc_wh;
            c.phi_pertes_hvc_wh += s.phi_pertes_hvc_wh;
            c.w_aux_wh += s.w_aux_wh;
            c.w_rechauf_wh += s.w_rechauf_wh;
            if !aval.is_empty() {
                c.theta_depart_max_c = Some(
                    c.theta_depart_max_c
                        .map_or(s.theta_depart_c, |t| t.max(s.theta_depart_c)),
                );
            }
            self.sorties_ecs[k] = s;
        }
        // §11.1 : pertes et consommations récupérables, injectées au pas suivant.
        let recup: Vec<(f64, f64)> = groupes
            .iter()
            .enumerate()
            .map(|(ig, g)| {
                let Some(sy) = &g.systemes else {
                    return (0.0, 0.0);
                };
                let s = &sy.sorties;
                let par_surface = |ratios: &[Vec<f64>], flux: &mut dyn Iterator<Item = f64>| {
                    ratios
                        .iter()
                        .zip(flux)
                        .map(|(r, x)| r.get(ig).copied().unwrap_or(0.0) * x)
                        .sum::<f64>()
                };
                let f = FluxVolumeChauffe {
                    vent_loc_wh: s.phi_vent_loc_vc_wh,
                    aux_dgr_wh: s.distributions.iter().map(|d| d.phi_aux_vc_wh).sum(),
                    pertes_dgr_wh: s.distributions.iter().map(|d| d.phi_pertes_vc_wh).sum(),
                    pertes_dgr_ecs_wh: sy.ecs.iter().map(|e| e.sorties.phi_pertes_vc_wh).sum(),
                    aux_dintgr_wh: par_surface(
                        &self.rat_surf,
                        &mut self.sorties.iter().map(|x| x.phi_aux_vc_wh),
                    ),
                    pertes_dintgr_wh: par_surface(
                        &self.rat_surf,
                        &mut self.sorties.iter().map(|x| x.phi_pertes_vc_wh),
                    ),
                    aux_dintgr_ecs_wh: par_surface(
                        &self.rat_surf_ecs,
                        &mut self.sorties_ecs.iter().map(|x| x.phi_aux_vc_wh),
                    ),
                    pertes_dintgr_ecs_wh: par_surface(
                        &self.rat_surf_ecs,
                        &mut self.sorties_ecs.iter().map(|x| x.phi_pertes_vc_wh),
                    ),
                    // Génération idéale : ni pertes ni auxiliaires.
                    generation_wh: 0.0,
                };
                recuperation_wh(&f)
            })
            .collect();
        drop(groupes);
        if self.recuperation {
            // Énergies du pas d'une heure, en Wh, restituées comme des puissances, en W.
            for (g, r) in zones
                .iter_mut()
                .flat_map(|z| z.groupes.iter_mut())
                .zip(recup)
            {
                g.recup_w = r;
                if let Some(sy) = &mut g.systemes {
                    sy.cumuls.recup_wh += r.0 + r.1;
                }
            }
        }
        if jour_bascule_saisons(h).is_some() {
            self.fixer_autorisations(zones);
        }
    }
}
