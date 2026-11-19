// SPDX-License-Identifier: GPL-3.0-or-later
//! Fiche `C_CALC_Sorties du mode Th-C` (Th-BCE 2020 §13.2.3.3 et §13.2.3.4) : sorties par groupe
//! et par zone, facultatives (`docs/questions/Q116.md`).
//!
//! Le relevé horaire par groupe et par zone ([`ReleveZones`]) se fait pendant le calcul Th-C,
//! comme celui du bâtiment ([`super::bilans_energies::ReleveElectrique`]) : différence des cumuls
//! d'un pas de temps à l'autre. Les postes qui n'appartiennent pas à un groupe (ventilateurs des
//! centrales, parkings, ascenseurs, escalators, usages mobiliers) s'ajoutent à leur zone ; le
//! reste du bâtiment (circulateurs des réseaux intergroupes, des centrales de traitement d'air et
//! de l'ECS intergroupe) est réparti au prorata des surfaces de référence : le texte ne dit pas
//! à quelle zone ou à quel groupe l'attribuer. La somme des zones redonne ainsi exactement le
//! bilan horaire du bâtiment (§13.4).

// Les boucles sur l'indice de poste parcourent plusieurs tableaux parallèles (`poste::*`).
#![allow(clippy::needless_range_loop)]

use crate::bati::assemblage_batiment::BatimentHoraire;
use crate::chauffage_refroidissement::calculs_generation::{Energie, Poste};
use crate::climat::calendrier::date_scenario;
use crate::conventions::NB_HEURES_AN;
use crate::entree::Batiment;
use crate::resultats::sorties_detaillees::{
    BesoinsSortie, LigneSortie, ProductionPvZone, SerieMensuelle, SortiesDetaillees, SortiesGroupe,
    SortiesZone,
};
use crate::resultats::{
    ConsommationsParEnergie, ConsommationsPostes, ResultatsGeneration, ResultatsZone,
};

use super::bilans_energies::{poste, NB_POSTES};
use super::resultats::ResultatsIndicateurs;
use super::sorties_th_c::{coefficients_electricite, coefficients_moyens};

/// Noms des postes, dans l'ordre de [`poste`].
const NOMS_POSTES: [&str; NB_POSTES] = [
    "chauffage",
    "refroidissement",
    "ecs",
    "eclairage",
    "auxiliaires_ventilation",
    "auxiliaires_distribution",
    "deplacements",
    "usages_mobiliers",
];

/// Énergies non électriques de la matrice des consommations, dans l'ordre de
/// `sorties_th_c::Colonne::TOUTES`.
const NOMS_ENERGIES: [&str; 5] = ["gaz", "fioul", "charbon", "bois", "reseau"];

/// Postes qu'un groupe porte (§13.2.3.3 : ni déplacement des occupants ni usages mobiliers).
const POSTES_GROUPE: [usize; 6] = [
    poste::CH,
    poste::FR,
    poste::ECS,
    poste::ECL,
    poste::AUX_VENT,
    poste::AUX_DIST,
];

/// Cumuls d'un groupe : électricité par poste, et `Qw,brut` en dernière position (Wh).
fn cumuls_groupe(
    b: &BatimentHoraire,
    i: usize,
    g: &crate::bati::assemblage_groupe::GroupeHoraire,
) -> [f64; NB_POSTES + 1] {
    let mut c = [0.0; NB_POSTES + 1];
    if let Some(m) = b.systemes.cumuls_cef_groupes.get(i) {
        let a = m.annuel();
        c[poste::CH] += a.valeur(Poste::Chauffage, Energie::Electricite);
        c[poste::FR] += a.valeur(Poste::Refroidissement, Energie::Electricite);
        c[poste::ECS] += a.valeur(Poste::Ecs, Energie::Electricite);
    }
    c[poste::ECL] += g.cumuls.c_ecl_kwh.iter().sum::<f64>() * 1000.0;
    if let Some(br) = &g.brasseurs {
        c[poste::AUX_VENT] += br.cumuls.w_wh;
    }
    if let Some(sy) = &g.systemes {
        c[poste::AUX_VENT] += sy.cumuls.w_vent_loc_wh;
        c[poste::AUX_DIST] += sy
            .cumuls
            .distributions
            .iter()
            .map(|d| d.w_aux_wh)
            .sum::<f64>();
        c[poste::AUX_DIST] += sy.ecs.iter().map(|e| e.cumuls.w_relevage_wh).sum::<f64>();
        c[NB_POSTES] += sy.ecs.iter().map(|e| e.cumuls.q_w_bruts_wh).sum::<f64>();
    }
    c
}

/// Relevé horaire de l'électricité par poste pour chaque zone, mensuel pour chaque groupe, en Wh.
#[derive(Debug, Clone, PartialEq)]
pub struct ReleveZones {
    /// Consommation de chaque pas de temps, par zone puis par poste.
    pub zones_horaire: Vec<Vec<[f64; NB_POSTES]>>,
    groupes_mensuel: Vec<[[f64; NB_POSTES]; 12]>,
    qw_brut_mensuel: Vec<[f64; 12]>,
    precedent: Vec<[f64; NB_POSTES + 1]>,
}

impl ReleveZones {
    /// Relevé vide pour la structure du bâtiment `b`.
    pub fn nouveau(b: &BatimentHoraire) -> Self {
        let nb_groupes: usize = b.zones.iter().map(|z| z.groupes.len()).sum();
        Self {
            zones_horaire: vec![vec![[0.0; NB_POSTES]; NB_HEURES_AN]; b.zones.len()],
            groupes_mensuel: vec![[[0.0; NB_POSTES]; 12]; nb_groupes],
            qw_brut_mensuel: vec![[0.0; 12]; nb_groupes],
            precedent: vec![[0.0; NB_POSTES + 1]; nb_groupes],
        }
    }

    /// Relève le pas de temps `h` qui vient d'être calculé.
    pub fn relever(&mut self, h: usize, b: &BatimentHoraire) {
        let mois = usize::from(date_scenario(h).mois) - 1;
        let mut i = 0;
        for (iz, z) in b.zones.iter().enumerate() {
            for g in &z.groupes {
                let c = cumuls_groupe(b, i, g);
                for p in 0..NB_POSTES {
                    let d = c[p] - self.precedent[i][p];
                    self.zones_horaire[iz][h][p] += d;
                    self.groupes_mensuel[i][mois][p] += d;
                }
                self.qw_brut_mensuel[i][mois] += c[NB_POSTES] - self.precedent[i][NB_POSTES];
                self.precedent[i] = c;
                i += 1;
            }
        }
    }

    /// Ajoute une série horaire (en Wh) au poste `p` de la zone `z`.
    pub fn ajouter_serie(&mut self, z: usize, p: usize, serie_wh: &[f64]) {
        if let Some(zone) = self.zones_horaire.get_mut(z) {
            for (x, w) in zone.iter_mut().zip(serie_wh) {
                x[p] += w;
            }
        }
    }

    /// Ajoute une consommation constante (en Wh par pas de temps) au poste `p` de la zone `z`.
    pub fn ajouter_constante(&mut self, z: usize, p: usize, w_wh: f64) {
        if let Some(zone) = self.zones_horaire.get_mut(z) {
            for x in zone {
                x[p] += w_wh;
            }
        }
    }
}

/// Entrées du calcul des sorties détaillées d'un bâtiment.
#[derive(Clone, Copy)]
pub struct EntreesSorties<'a> {
    /// Description du bâtiment.
    pub batiment: &'a Batiment,
    /// Bâtiment après le calcul Th-C (cumuls des groupes).
    pub th_c: &'a BatimentHoraire,
    /// Résultats des zones (non arrondis).
    pub zones: &'a [ResultatsZone],
    /// Consommation horaire du bâtiment par poste, usages mobiliers compris (§13.4).
    pub total_horaire: &'a [[f64; NB_POSTES]],
    /// Production photovoltaïque horaire du bâtiment, en Wh ; vide sans installation.
    pub prod_pv_wh: &'a [f64],
    /// Indicateurs du bâtiment (forfaits de refroidissement).
    pub indicateurs: &'a ResultatsIndicateurs,
    /// Générations du bâtiment.
    pub generations: &'a [ResultatsGeneration],
    /// Consommations annuelles des générations du bâtiment.
    pub consommations_bat: &'a ConsommationsPostes,
}

fn en_kwh(m: &[f64; 12]) -> [f64; 12] {
    std::array::from_fn(|k| m[k] / 1000.0)
}

fn valeur_colonne(c: &ConsommationsParEnergie, k: usize) -> f64 {
    [
        c.gaz_kwh,
        c.fioul_kwh,
        c.charbon_kwh,
        c.bois_kwh,
        c.reseau_kwh,
    ][k]
}

fn poste_de(c: &ConsommationsPostes, p: usize) -> &ConsommationsParEnergie {
    match p {
        0 => &c.chauffage,
        1 => &c.refroidissement,
        _ => &c.ecs,
    }
}

/// Lignes non électriques (chauffage, refroidissement, ECS) d'un niveau, avec leurs quantités
/// mensuelles : `(poste, colonne, mensuel kWh)`.
fn lignes_non_electriques(
    conso: Option<&crate::resultats::Consommations>,
) -> Vec<(usize, usize, [f64; 12])> {
    let Some(c) = conso else {
        return Vec::new();
    };
    let mut v = Vec::new();
    for p in 0..3 {
        for k in 0..NOMS_ENERGIES.len() {
            if valeur_colonne(poste_de(&c.annuel, p), k) <= 0.0 {
                continue;
            }
            let mensuel = std::array::from_fn(|m| {
                c.mensuel
                    .get(m)
                    .map_or(0.0, |x| valeur_colonne(poste_de(x, p), k))
            });
            v.push((p, k, mensuel));
        }
    }
    v
}

/// Cef, Cep et Cep,nr en kWh d'un ensemble de lignes (hors usages mobiliers, éq. 2446).
#[derive(Default)]
struct Totaux {
    cef: f64,
    cep: f64,
    cep_nr: f64,
}

/// Sorties détaillées par zone et par groupe d'un bâtiment (Th-BCE 2020 §13.2.3.3 et §13.2.3.4,
/// éq. 2416 à 2461 ; lectures Q116).
pub fn calculer(e: &EntreesSorties<'_>, mut releve: ReleveZones) -> SortiesDetaillees {
    let nz = e.zones.len();
    let sref_zone: Vec<f64> = e.zones.iter().map(|z| z.sref_m2).collect();
    let sref_bat: f64 = sref_zone.iter().sum();
    let part_sref = |s: f64| if sref_bat > 0.0 { s / sref_bat } else { 0.0 };
    let mois_de: Vec<usize> = (0..NB_HEURES_AN)
        .map(|h| usize::from(date_scenario(h).mois) - 1)
        .collect();

    // Reste du bâtiment : consommations qui ne sont ni d'un groupe ni d'une zone (circulateurs
    // intergroupes, réseaux de CTA, ECS intergroupe), réparties au prorata des surfaces.
    let mut reste_mensuel = [[0.0; NB_POSTES]; 12];
    for (h, tot) in e.total_horaire.iter().enumerate().take(NB_HEURES_AN) {
        let m = mois_de[h];
        for p in 0..NB_POSTES {
            let somme: f64 = releve.zones_horaire.iter().map(|z| z[h][p]).sum();
            let r = tot[p] - somme;
            reste_mensuel[m][p] += r;
            for (z, s) in releve.zones_horaire.iter_mut().zip(&sref_zone) {
                z[h][p] += r * part_sref(*s);
            }
        }
    }

    // Importations (éq. 2536) et production attribuée à chaque zone, heure par heure.
    let avec_pv = !e.prod_pv_wh.is_empty();
    let mut cons_m = vec![[[0.0; NB_POSTES]; 12]; nz];
    let mut imp_m = vec![[[0.0; NB_POSTES]; 12]; nz];
    // [zone][variante : 0 = consommations, 1 = SREF][produit, autoconsommé][mois]
    let mut pv = vec![[[[0.0; 12]; 2]; 2]; nz];
    for h in 0..NB_HEURES_AN {
        let m = mois_de[h];
        let tous_bat: f64 = releve
            .zones_horaire
            .iter()
            .map(|z| z[h].iter().sum::<f64>())
            .sum();
        let prod = e.prod_pv_wh.get(h).copied().unwrap_or(0.0).max(0.0);
        // Éq. (2527) à (2530), (2536).
        let ac = prod.min(tous_bat.max(0.0));
        let tap = if tous_bat > 0.0 { ac / tous_bat } else { 0.0 };
        let tac = if prod > 0.0 { ac / prod } else { 0.0 };
        for z in 0..nz {
            let w = &releve.zones_horaire[z][h];
            for p in 0..NB_POSTES {
                cons_m[z][m][p] += w[p];
                imp_m[z][m][p] += w[p] * (1.0 - tap);
            }
            if avec_pv {
                let tous_z: f64 = w.iter().sum();
                let parts = [
                    if tous_bat > 0.0 {
                        tous_z / tous_bat
                    } else {
                        part_sref(sref_zone[z])
                    },
                    part_sref(sref_zone[z]),
                ];
                for (v, part) in parts.into_iter().enumerate() {
                    pv[z][v][0][m] += prod * part;
                    // Éq. (2532) à (2534) : taux d'autoconsommation du bâtiment appliqué à la
                    // production attribuée.
                    pv[z][v][1][m] += tac * prod * part;
                }
            }
        }
    }

    // Coefficients de conversion : moyens par (poste, colonne) sur le bâtiment.
    let coefs = coefficients_moyens(e.batiment, e.generations, e.consommations_bat);
    let (ep_elec, nr_elec) = coefficients_electricite();
    let forfaits = &e.indicateurs.forfaits_refroidissement;

    let mut zones_sorties = Vec::with_capacity(nz);
    let mut i_groupe = 0;
    for (iz, rz) in e.zones.iter().enumerate() {
        let sref_z = rz.sref_m2;
        let forfait_zone: f64 = forfaits
            .iter()
            .filter(|f| f.zone == rz.id)
            .map(|f| f.cef_kwh)
            .sum();

        // Groupes de la zone.
        let mut groupes = Vec::with_capacity(rz.groupes.len());
        let mut besoins_z = [[[0.0; 12]; 3]; 2];
        for (ig, rg) in rz.groupes.iter().enumerate() {
            let sref_g = rg.sref_m2;
            let forfait_g: f64 = forfaits
                .iter()
                .filter(|f| f.zone == rz.id && f.groupe == rg.id)
                .map(|f| f.cef_kwh)
                .sum();
            let gm = &releve.groupes_mensuel[i_groupe];
            let mut lignes = Vec::new();
            let mut t = Totaux::default();
            for p in POSTES_GROUPE {
                let mensuel: [f64; 12] = std::array::from_fn(|m| {
                    (gm[m][p] + reste_mensuel[m][p] * part_sref(sref_g)) / 1000.0
                });
                let supp = if p == poste::FR { forfait_g } else { 0.0 };
                let q = SerieMensuelle::nouvelle(mensuel, supp, sref_g);
                if q.annuel_kwh > 0.0 {
                    t.cef += q.annuel_kwh;
                    t.cep += q.annuel_kwh * ep_elec;
                    t.cep_nr += q.annuel_kwh * nr_elec;
                    lignes.push(LigneSortie {
                        poste: NOMS_POSTES[p].to_owned(),
                        energie: "electricite".to_owned(),
                        quantite: q,
                    });
                }
            }
            for (p, k, mensuel) in lignes_non_electriques(rg.consommations.as_ref()) {
                let q = SerieMensuelle::nouvelle(mensuel, 0.0, sref_g);
                t.cef += q.annuel_kwh;
                t.cep += q.annuel_kwh * coefs[p][k].0;
                t.cep_nr += q.annuel_kwh * coefs[p][k].1;
                lignes.push(LigneSortie {
                    poste: NOMS_POSTES[p].to_owned(),
                    energie: NOMS_ENERGIES[k].to_owned(),
                    quantite: q,
                });
            }
            // Besoins : éq. (2498) aux bornes des émetteurs (lecture retenue) et Qsys thermique.
            let gh = &e.th_c.zones[iz].groupes[ig];
            let (ch, fr, qs_ch, qs_fr) = match &gh.systemes {
                Some(sy) => (
                    en_kwh(&sy.cumuls.q_sys_ch_mensuel_wh),
                    en_kwh(&sy.cumuls.q_sys_fr_mensuel_wh),
                    en_kwh(&gh.cumuls.q_sys_ch_wh),
                    en_kwh(&gh.cumuls.q_sys_fr_wh),
                ),
                None => (
                    [0.0; 12],
                    [0.0; 12],
                    en_kwh(&gh.cumuls.q_sys_ch_wh),
                    en_kwh(&gh.cumuls.q_sys_fr_wh),
                ),
            };
            let ecs = en_kwh(&releve.qw_brut_mensuel[i_groupe]);
            for m in 0..12 {
                for (v, b) in [[ch, fr, ecs], [qs_ch, qs_fr, ecs]].iter().enumerate() {
                    for (k, serie) in b.iter().enumerate() {
                        besoins_z[v][k][m] += serie[m];
                    }
                }
            }
            let besoins = |s: [[f64; 12]; 3]| BesoinsSortie {
                chauffage: SerieMensuelle::nouvelle(s[0], 0.0, sref_g),
                refroidissement: SerieMensuelle::nouvelle(s[1], 0.0, sref_g),
                ecs: SerieMensuelle::nouvelle(s[2], 0.0, sref_g),
            };
            let par_m2 = |x: f64| if sref_g > 0.0 { x / sref_g } else { 0.0 };
            groupes.push(SortiesGroupe {
                id: rg.id.clone(),
                sref_m2: sref_g,
                consommations: lignes,
                cef_kwh_m2: par_m2(t.cef),
                cep_kwhep_m2: par_m2(t.cep),
                cep_nr_kwhep_m2: par_m2(t.cep_nr),
                besoins: besoins([ch, fr, ecs]),
                besoins_qsys_thermique: besoins([qs_ch, qs_fr, ecs]),
            });
            i_groupe += 1;
        }

        // Zone : quantités importées et consommées.
        let mut importees = Vec::new();
        let mut consommee = Vec::new();
        let mut t = Totaux::default();
        for p in 0..NB_POSTES {
            let supp = if p == poste::FR { forfait_zone } else { 0.0 };
            let imp = SerieMensuelle::nouvelle(
                std::array::from_fn(|m| imp_m[iz][m][p] / 1000.0),
                supp,
                sref_z,
            );
            let cons = SerieMensuelle::nouvelle(
                std::array::from_fn(|m| cons_m[iz][m][p] / 1000.0),
                supp,
                sref_z,
            );
            if p != poste::MOB {
                t.cef += imp.annuel_kwh;
                t.cep += imp.annuel_kwh * ep_elec;
                t.cep_nr += imp.annuel_kwh * nr_elec;
            }
            if imp.annuel_kwh > 0.0 {
                importees.push(LigneSortie {
                    poste: NOMS_POSTES[p].to_owned(),
                    energie: "electricite".to_owned(),
                    quantite: imp,
                });
            }
            if cons.annuel_kwh > 0.0 {
                consommee.push(LigneSortie {
                    poste: NOMS_POSTES[p].to_owned(),
                    energie: "electricite".to_owned(),
                    quantite: cons,
                });
            }
        }
        for (p, k, mensuel) in lignes_non_electriques(rz.consommations.as_ref()) {
            let q = SerieMensuelle::nouvelle(mensuel, 0.0, sref_z);
            t.cef += q.annuel_kwh;
            t.cep += q.annuel_kwh * coefs[p][k].0;
            t.cep_nr += q.annuel_kwh * coefs[p][k].1;
            importees.push(LigneSortie {
                poste: NOMS_POSTES[p].to_owned(),
                energie: NOMS_ENERGIES[k].to_owned(),
                quantite: q,
            });
        }
        let production = |v: usize| {
            avec_pv.then(|| {
                let produite = SerieMensuelle::nouvelle(
                    std::array::from_fn(|m| pv[iz][v][0][m] / 1000.0),
                    0.0,
                    sref_z,
                );
                let auto = SerieMensuelle::nouvelle(
                    std::array::from_fn(|m| pv[iz][v][1][m] / 1000.0),
                    0.0,
                    sref_z,
                );
                // Éq. (2437), sans cogénération.
                let exportee = SerieMensuelle::nouvelle(
                    std::array::from_fn(|m| (pv[iz][v][0][m] - pv[iz][v][1][m]) / 1000.0),
                    0.0,
                    sref_z,
                );
                let tac_pct = if produite.annuel_kwh > 0.0 {
                    100.0 * auto.annuel_kwh / produite.annuel_kwh
                } else {
                    0.0
                };
                ProductionPvZone {
                    produite,
                    autoconsommee: auto,
                    exportee,
                    tac_pct,
                }
            })
        };
        let par_m2 = |x: f64| if sref_z > 0.0 { x / sref_z } else { 0.0 };
        let besoins = |s: [[f64; 12]; 3]| BesoinsSortie {
            chauffage: SerieMensuelle::nouvelle(s[0], 0.0, sref_z),
            refroidissement: SerieMensuelle::nouvelle(s[1], 0.0, sref_z),
            ecs: SerieMensuelle::nouvelle(s[2], 0.0, sref_z),
        };
        zones_sorties.push(SortiesZone {
            id: rz.id.clone(),
            sref_m2: sref_z,
            importees,
            electricite_consommee: consommee,
            production_pv: production(0),
            production_pv_variante_sref: production(1),
            cef_kwh_m2: par_m2(t.cef),
            cep_kwhep_m2: par_m2(t.cep),
            cep_nr_kwhep_m2: par_m2(t.cep_nr),
            besoins: besoins(besoins_z[0]),
            besoins_qsys_thermique: besoins(besoins_z[1]),
            groupes,
        });
    }
    SortiesDetaillees {
        zones: zones_sorties,
    }
}
