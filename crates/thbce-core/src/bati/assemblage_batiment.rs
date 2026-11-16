// SPDX-License-Identifier: GPL-3.0-or-later
//! Enchaînement horaire d'un bâtiment : espaces tampons puis zones.
//!
//! Un espace tampon est défini au niveau du bâtiment et peut toucher des groupes de plusieurs
//! zones (§5.4.1). À chaque pas de temps, dans l'ordre de la figure 15 (§5.2.2) : les espaces
//! tampons sont calculés à partir de l'état des groupes à la fin du pas précédent (`θi,fin(h−1)`,
//! `θop,fin(h−1)`, saisons) et de leur occupation au pas courant ; leurs coefficients
//! `btherm(h)`, `bsolaire(h)`, `becl(h)` et leur température `θet(h)` sont ensuite transmis aux
//! composants d'enveloppe et aux entrées d'air des zones (§5.5, §5.7), calculées comme avant.

use crate::chauffage_refroidissement::calculs_generation::{CumulsCef, Poste};
use crate::chauffage_refroidissement::distribution::{
    ApportReseauGroupe, DistributionIntergroupesResolue, SortiesDistributionIntergroupes,
};
use crate::chauffage_refroidissement::generation::{
    ConditionsSource, EntreesGeneration, GenerationHoraire, ReseauVu,
};
use crate::chauffage_refroidissement::saisons_fonctionnement::{
    autorisations_effectives, DesserteGroupe,
};
use crate::chauffage_refroidissement::sources_amont::{SourceAmont, C_PA_J_KG_K};
use crate::chauffage_refroidissement::systemes_groupe::Liens;
use crate::climat::calendrier::{date_scenario, hleg, jour_bascule_saisons};
use crate::climat::environnement_proche::ventc_m_s;
use crate::climat::{ClimatExterieur, ConfortAdaptatif};
use crate::conventions::{ModeCalcul, NB_HEURES_AN};
use crate::ecs::distribution_ecs::{pas_intergroupe, SortiesDistributionEcsIntergroupe};
use crate::entree::{Batiment, Catalogue};
use crate::entree::{
    FonctionReseau, PositionGeneration, RaccordementReseaux, ReseauEcsIntergroupe, ReseauGroupe,
    ReseauIntergroupes, SystemeVentilation,
};
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
    /// Générations (§8.14 et §8.17), dans l'ordre de `generations`.
    pub generations_horaires: Vec<GenerationHoraire>,
    /// Espace tampon qui abrite chaque génération hors volume chauffé.
    pub et_generation: Vec<Option<usize>>,
    /// « Ratsurf^gen,gr » : pour chaque génération, part de chaque groupe desservi dans leur
    /// surface totale (éq. 988), pour la répartition des pertes vers l'ambiance.
    pub rat_surf_gen: Vec<Vec<f64>>,
    /// « Ratsurf_dess^dp » de chaque réseau intergroupes de chaud ou de froid (éq. 990 et 991).
    pub rat_surf_dess: Vec<f64>,
    /// « Ratsurf_dess_ecs^dp-e » de chaque distribution d'ECS intergroupe (éq. 992).
    pub rat_surf_dess_ecs: Vec<f64>,
    /// Pour chaque zone, génération qui fournit l'énergie de la sécurité antigel de son double
    /// flux (§8.11.3, éq. 939 ; lecture Q46).
    pub antigel: Vec<Option<usize>>,
    /// Surface de chaque groupe (ordre des zones puis des groupes) et indice de sa zone.
    pub groupes: Vec<(usize, f64)>,
    /// Consommations en énergie finale attribuées à chaque groupe (éq. 1484 à 1486, 1499, 1506
    /// et 1513).
    pub cumuls_cef_groupes: Vec<CumulsCef>,
    /// Énergie de la sécurité antigel transmise aux générations, par zone, en Wh.
    pub cumuls_antigel_wh: Vec<f64>,
    /// « Φrejet » : puissance puisée dans chaque espace tampon par les évaporateurs des
    /// générateurs thermodynamiques sur air ambiant au dernier pas de temps, en W (§5.4.3.2,
    /// éq. 109).
    pub phi_rejet_et_w: Vec<f64>,
}

/// Origine d'une demande reçue par une génération.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origine {
    /// Réseau intergroupes de chaud ou de froid.
    Reseau(usize),
    /// Distribution d'ECS intergroupe.
    Ecs(usize),
    /// Sécurité antigel du double flux d'une zone.
    Antigel(usize),
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
                et.phi_rejet_w = self.systemes.phi_rejet_et_w.get(i).copied().unwrap_or(0.0);
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
        let ids_zones: Vec<String> = bat.zones.iter().map(|z| z.id.clone()).collect();
        let mut generations_horaires: Vec<GenerationHoraire> = bat
            .generations
            .iter()
            .map(|g| GenerationHoraire::resoudre(g, &ids_et, &ids_zones))
            .collect();
        let et_generation = bat
            .generations
            .iter()
            .map(|g| match &g.position {
                PositionGeneration::HorsVolumeChauffe {
                    espace_tampon: Some(id),
                } => ids_et.iter().position(|e| e == id),
                _ => None,
            })
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
                            // Éq. (980) et (981) : températures extrêmes des réseaux
                            // hydrauliques, celles de dimensionnement de leurs départs.
                            if let ReseauGroupe::Hydraulique(h) = &d.reseau {
                                if let (Some(t), Some(gen)) = (
                                    h.theta_dep_dim_c,
                                    generations_horaires.get_mut(dp.generation),
                                ) {
                                    match d.fonction {
                                        FonctionReseau::Chauffage => {
                                            gen.theta_dist_ch_max_c = Some(
                                                gen.theta_dist_ch_max_c.map_or(t, |x| x.max(t)),
                                            );
                                        }
                                        FonctionReseau::Refroidissement => {
                                            gen.theta_dist_fr_min_c = Some(
                                                gen.theta_dist_fr_min_c.map_or(t, |x| x.min(t)),
                                            );
                                        }
                                    }
                                }
                            }
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
        // Éq. (990) à (992) : part de chaque réseau dans la surface desservie par sa génération
        // pour son poste.
        let parts = |a: &[Vec<f64>], cle: &dyn Fn(usize) -> (usize, u8)| -> Vec<f64> {
            let totaux: Vec<f64> = a.iter().map(|x| x.iter().sum()).collect();
            (0..a.len())
                .map(|k| {
                    let somme: f64 = (0..a.len())
                        .filter(|&j| cle(j) == cle(k))
                        .map(|j| totaux[j])
                        .sum();
                    if somme > 0.0 {
                        totaux[k] / somme
                    } else {
                        0.0
                    }
                })
                .collect()
        };
        let rat_surf_dess = parts(&a_desservie, &|k| {
            (
                intergroupes[k].generation,
                u8::from(intergroupes[k].fonction == FonctionReseau::Chauffage),
            )
        });
        let rat_surf_dess_ecs = parts(&a_ecs, &|k| (ecs[k].generation, 0));
        // Sécurité antigel des doubles flux : génération désignée par la saisie.
        let antigel: Vec<Option<usize>> = bat
            .zones
            .iter()
            .map(|z| match z.ventilation.as_ref().map(|v| &v.systeme) {
                Some(SystemeVentilation::DoubleFlux(d)) => d
                    .echangeur
                    .as_ref()
                    .filter(|e| e.antigel == Some(true))
                    .and_then(|e| e.generation_antigel.as_ref())
                    .and_then(|id| generations.iter().position(|g| g == id)),
                _ => None,
            })
            .collect();
        let groupes: Vec<(usize, f64)> = zones
            .iter()
            .enumerate()
            .flat_map(|(iz, z)| z.groupes.iter().map(move |g| (iz, g.a_gr_m2)))
            .collect();
        // Éq. (988) et (989) : surfaces des groupes desservis par chaque génération.
        let rat_surf_gen: Vec<Vec<f64>> = (0..generations.len())
            .map(|ig| {
                let a: Vec<f64> = groupes
                    .iter()
                    .enumerate()
                    .map(|(igr, (iz, a_gr))| {
                        let desservi = (0..n)
                            .any(|k| intergroupes[k].generation == ig && a_desservie[k][igr] > 0.0)
                            || (0..n_ecs).any(|k| ecs[k].generation == ig && a_ecs[k][igr] > 0.0)
                            || antigel[*iz] == Some(ig);
                        if desservi {
                            *a_gr
                        } else {
                            0.0
                        }
                    })
                    .collect();
                let total: f64 = a.iter().sum();
                a.iter()
                    .map(|x| if total > 0.0 { x / total } else { 0.0 })
                    .collect()
            })
            .collect();
        let nb_groupes = groupes.len();
        let nb_zones = zones.len();
        let mut s = Self {
            generations_horaires,
            et_generation,
            rat_surf_gen,
            rat_surf_dess,
            rat_surf_dess_ecs,
            antigel,
            groupes,
            cumuls_cef_groupes: vec![CumulsCef::default(); nb_groupes],
            cumuls_antigel_wh: vec![0.0; nb_zones],
            phi_rejet_et_w: vec![0.0; ids_et.len()],
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
        // §9.8 : distributions d'ECS intergroupes ; génération d'ECS en fonctionnement toute
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
        // §8.14 et §8.17 : générations, après leurs réseaux. Chacune reçoit les demandes de ses
        // réseaux de chaud, de froid et d'ECS, et l'énergie de la sécurité antigel des doubles
        // flux qui la désignent (§8.11.3, éq. 939 et 952 : réseau associé à l'antigel, vu comme
        // un réseau sur air à la température de l'air neuf ; lecture Q46).
        let mut vus: Vec<ReseauVu> = Vec::new();
        let mut origines: Vec<Origine> = Vec::new();
        let mut sources: Vec<ConditionsSource> = Vec::new();
        let mut phi_vc_gen = vec![0.0; groupes.len()];
        for x in &mut self.phi_rejet_et_w {
            *x = 0.0;
        }
        for ig in 0..self.generations_horaires.len() {
            vus.clear();
            origines.clear();
            for (k, dp) in self.intergroupes.iter().enumerate() {
                if dp.generation != ig {
                    continue;
                }
                let s = &self.sorties[k];
                let (poste, q) = match dp.fonction {
                    FonctionReseau::Chauffage => (Poste::Chauffage, s.q_sys_ch_wh),
                    FonctionReseau::Refroidissement => (Poste::Refroidissement, -s.q_sys_fr_wh),
                };
                vus.push(ReseauVu {
                    poste,
                    hydraulique: !matches!(dp.reseau, ReseauIntergroupes::Fictif),
                    q_sys_wh: q,
                    theta_moy_c: s.theta_moy_c,
                    theta_dep_c: s.theta_dep_c,
                    theta_i_aval_eq_c: s.theta_i_aval_eq_c,
                    relance: s.relance,
                    rat_surf_dess: self.rat_surf_dess[k],
                });
                origines.push(Origine::Reseau(k));
            }
            let mut w_rechauf = 0.0;
            for (k, d) in self.ecs.iter().enumerate() {
                if d.generation != ig {
                    continue;
                }
                let s = &self.sorties_ecs[k];
                w_rechauf += s.w_rechauf_wh;
                vus.push(ReseauVu {
                    poste: Poste::Ecs,
                    hydraulique: true,
                    q_sys_wh: s.q_w_prim_wh,
                    theta_moy_c: s.theta_moy_c,
                    theta_dep_c: s.theta_depart_c,
                    theta_i_aval_eq_c: 0.0,
                    relance: false,
                    rat_surf_dess: self.rat_surf_dess_ecs[k],
                });
                origines.push(Origine::Ecs(k));
            }
            for (iz, z) in zones.iter().enumerate() {
                if self.antigel[iz] != Some(ig) {
                    continue;
                }
                let w_ag = z.sorties_ventilation.w_ag_wh;
                self.cumuls_antigel_wh[iz] += w_ag;
                vus.push(ReseauVu {
                    poste: Poste::Chauffage,
                    hydraulique: false,
                    q_sys_wh: w_ag,
                    theta_moy_c: 0.0,
                    theta_dep_c: 0.0,
                    theta_i_aval_eq_c: theta_ext_c,
                    relance: false,
                    // Sans demande, la sécurité antigel ne pèse pas dans les répartitions.
                    rat_surf_dess: 0.0,
                });
                origines.push(Origine::Antigel(iz));
            }
            let b_therm = self.et_generation[ig]
                .and_then(|i| espaces_tampons.get(i))
                .map_or(1.0, |e| e.b_therm);
            let (aut_ch, aut_fr) = self.aut_gen.get(ig).copied().unwrap_or((false, false));
            let gen = &mut self.generations_horaires[ig];
            // Fiche 8.26, éq. (1405) à (1407) et (1453) : conditions des sources amont.
            sources.clear();
            sources.extend(gen.generateurs.iter().map(|g| {
                match g.source.map(|s| s.source) {
                    None | Some(SourceAmont::AirExterieur) => ConditionsSource {
                        theta_amont_c: theta_ext_c,
                        air_extrait_w_k: None,
                    },
                    Some(SourceAmont::AirAmbiant { espace_tampon }) => ConditionsSource {
                        theta_amont_c: espaces_tampons
                            .get(espace_tampon)
                            .map_or(theta_ext_c, |e| e.theta_et_c),
                        air_extrait_w_k: None,
                    },
                    Some(SourceAmont::AirExtrait { zone }) => {
                        let v = zones.get(zone).map(|z| z.sorties_ventilation);
                        ConditionsSource {
                            theta_amont_c: v.map_or(theta_ext_c, |v| v.theta_air_extrait_c),
                            air_extrait_w_k: Some(
                                v.map_or(0.0, |v| v.qm_air_extrait_kg_s) * C_PA_J_KG_K,
                            ),
                        }
                    }
                }
            }));
            gen.pas(
                &EntreesGeneration {
                    reseaux: &vus,
                    aut_ch,
                    aut_fr,
                    // Éq. (1007) : génération d'ECS en fonctionnement toute l'année (Q45).
                    id_encl: true,
                    theta_ext_c,
                    b_therm,
                    w_rechauf_wh: w_rechauf,
                    theta_cw_c,
                    hleg: hleg(h),
                    sources: &sources,
                },
                mois,
            );
            // §5.4.3.2, éq. (109) : chaleur puisée dans les espaces tampons par les générateurs
            // sur air ambiant (« φrejet » est négatif en production d'ECS).
            for g in &gen.generateurs {
                if let Some(SourceAmont::AirAmbiant { espace_tampon }) = g.source.map(|s| s.source)
                {
                    if let Some(x) = self.phi_rejet_et_w.get_mut(espace_tampon) {
                        *x -= g.jeu.phi_rejet_wh;
                    }
                }
            }
            let s = &gen.sorties;
            // Éq. (999) à (1001) : part de chaque groupe dans la demande de la génération, par
            // poste ; éq. (1484) à (1486) : consommations attribuées aux groupes.
            for (igr, g) in groupes.iter().enumerate() {
                let mut rat = [0.0; 3];
                for ((r, o), rat_dp) in vus.iter().zip(&origines).zip(&s.rat_bes_dp) {
                    let part = match *o {
                        // Éq. (893) et (894).
                        Origine::Reseau(k) => {
                            let de = |x: &crate::chauffage_refroidissement::distribution::SortiesDistributionGroupe| {
                                if r.poste == Poste::Chauffage {
                                    x.q_sys_ch_wh
                                } else {
                                    -x.q_sys_fr_wh
                                }
                            };
                            let somme = |gr: &super::assemblage_groupe::GroupeHoraire| -> f64 {
                                gr.systemes.as_ref().map_or(0.0, |sy| {
                                    sy.distributions
                                        .iter()
                                        .zip(&sy.sorties.distributions)
                                        .filter(|(d, _)| d.intergroupes == k)
                                        .map(|(_, x)| de(x))
                                        .sum()
                                })
                            };
                            let total: f64 = groupes.iter().map(|gr| somme(gr)).sum();
                            if total > 0.0 {
                                somme(g) / total
                            } else {
                                self.rat_surf[k].get(igr).copied().unwrap_or(0.0)
                            }
                        }
                        // Éq. (1739).
                        Origine::Ecs(k) => {
                            let somme = |gr: &super::assemblage_groupe::GroupeHoraire| -> f64 {
                                gr.systemes.as_ref().map_or(0.0, |sy| {
                                    sy.ecs
                                        .iter()
                                        .filter(|e| e.intergroupe == k)
                                        .map(|e| e.sorties.q_w_2nd_wh)
                                        .sum()
                                })
                            };
                            let total: f64 = groupes.iter().map(|gr| somme(gr)).sum();
                            if total > 0.0 {
                                somme(g) / total
                            } else {
                                self.rat_surf_ecs[k].get(igr).copied().unwrap_or(0.0)
                            }
                        }
                        // Groupes de la zone ventilée, au prorata de leurs surfaces.
                        Origine::Antigel(iz) => {
                            let total: f64 = self
                                .groupes
                                .iter()
                                .filter(|(z, _)| *z == iz)
                                .map(|(_, a)| a)
                                .sum();
                            match self.groupes.get(igr) {
                                Some((z, a)) if *z == iz && total > 0.0 => a / total,
                                _ => 0.0,
                            }
                        }
                    };
                    rat[r.poste.indice()] += rat_dp * part;
                }
                self.cumuls_cef_groupes[igr].ajouter_pondere(mois, &s.q_cef, rat);
                // Éq. (2362) : pertes de la génération vers l'ambiance, au prorata des surfaces.
                phi_vc_gen[igr] += self.rat_surf_gen[ig][igr] * s.phi_vc_tot_wh;
            }
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
                    generation_wh: phi_vc_gen[ig],
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
