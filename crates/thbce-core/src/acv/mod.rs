// SPDX-License-Identifier: GPL-3.0-or-later
//! Analyse du cycle de vie d'un bâtiment (annexe II de l'arrêté du 4 août 2021, chapitre 4 et
//! §5.3) : contributions « composants » (détaillée et forfaitaire), « chantier », « eau »,
//! « parcelle », indicateurs Icconstruction, Icbâtiment, Icprojet, StockC, Icded. Seul
//! l'indicateur de changement climatique est calculé.
//!
//! Le module est pur : il reçoit les données de l'ACV (validées), la surface de référence et
//! l'usage des zones, Ic énergie et le taux d'autoconsommation photovoltaïque du bâtiment.

pub mod composants;
pub mod donnees;
pub mod exigences;

use std::collections::BTreeMap;

use crate::entree::{
    Assainissement, ChantierAcv, ComposantAcv, DonneesAcv, GestionEauxPluviales, OccupantsAcv,
};
use crate::resultats::acv::{
    ContributionAcv, PhasesAcv, QuantitesEauAcv, ResultatsAcv, ResultatsLotAcv, ResultatsSousLotAcv,
};
use crate::resultats::ValeurParDefaut;
use composants::{
    composant_chantier, composant_dynamique, composant_statique, fluide_cas1, fluide_cas2,
    fluide_cas3, fluide_statique, somme_f_co2, stock_c, Phases, DVE_LOT_FORFAITAIRE_ANS, PER_ANS,
};
use donnees::{forfait, lot_de, q_eau_potable_conv_m3};

/// Distance par défaut des terres importées et évacuées du chantier simplifié, en km (§4.5.2).
pub const DISTANCE_DEFAUT_KM: f64 = 30.0;
/// « μterres » : masse volumique des terres, en t/m³ (éq. 101).
pub const MU_TERRES_T_M3: f64 = 1.45;
/// « ε1 » par défaut, en L/m³ (éq. 103).
pub const EPSILON1_L_M3: f64 = 1.0;
/// « ε2 » par défaut, en L/t (éq. 106).
pub const EPSILON2_L_T: f64 = 0.7;

/// Entrées du calcul de l'ACV d'un bâtiment.
#[derive(Debug, Clone, Copy)]
pub struct EntreesAcv<'a> {
    /// Données saisies (validées).
    pub donnees: &'a DonneesAcv,
    /// Surface de référence du bâtiment, en m².
    pub sref_m2: f64,
    /// Zones du bâtiment : numéro d'usage et `Sref`.
    pub zones: &'a [(u8, f64)],
    /// Ic énergie du bâtiment, en kg éq. CO2/m².
    pub ic_energie_kgco2_m2: Option<f64>,
    /// Taux d'autoconsommation de la production photovoltaïque, en %.
    pub tac_pv_pct: Option<f64>,
}

/// Cumul d'un sous-lot, en kg éq. CO2 (absolu) puis rapporté à la Sref.
#[derive(Debug, Clone, Copy, Default)]
struct Cumul {
    statique: Phases,
    dynamique: Phases,
    ded: f64,
    stock_c: f64,
    forfaitaire: bool,
}

fn ajouter(a: &mut Phases, b: Phases, k: f64) {
    for (x, y) in a.iter_mut().zip(b) {
        *x += k * y;
    }
}

fn par_m2(p: Phases, sref: f64) -> PhasesAcv {
    PhasesAcv::depuis(p.map(|x| x / sref))
}

/// « DVEp » d'un composant : saisie ou, par défaut, DVR de la donnée (§4.2.1.1.1).
fn dve(c: &ComposantAcv, chemin: &str, defauts: &mut Vec<ValeurParDefaut>) -> f64 {
    c.dve_ans.unwrap_or_else(|| {
        defauts.push(ValeurParDefaut {
            champ: format!("{chemin}.dve_ans"),
            valeur: c.donnee.dvr_ans,
            regle: "annexe II §4.2.1.1.1 : DVE = DVR de la donnée environnementale".into(),
        });
        c.donnee.dvr_ans
    })
}

/// Contributions statique et dynamique d'un composant, fluide frigorigène compris : `(statique,
/// dynamique, fluide statique, fluide dynamique)`, en kg éq. CO2.
fn composant(q_dve: (f64, f64), c: &ComposantAcv) -> (Phases, Phases, Phases, Phases) {
    let (q, dve) = q_dve;
    let mut st = composant_statique(q, &c.donnee, dve);
    let mut dy = composant_dynamique(q, &c.donnee, dve);
    let mut fs = [0.0; 5];
    let mut fd = [0.0; 5];
    if let Some(f) = &c.fluide {
        let (e, fdv) = match c.donnee.b1_kgco2 {
            // Cas 2 : fuites dans B1, correctif seul (éq. 54 à 60).
            Some(b1) => fluide_cas2(b1, dve),
            // Cas 1 : B non détaillé (éq. 26, 45 à 53).
            None => {
                fs[2] = q * fluide_statique(f.charge_kg, f.de_kgco2_kg.unwrap_or(0.0));
                fluide_cas1(f.charge_kg, f.prg_kgco2_kg, dve)
            }
        };
        fd[2] = q * e; // éq. (50), (59)
        fd[3] = q * fdv; // éq. (53), (62)
        ajouter(&mut st, fs, 1.0);
        ajouter(&mut dy, fd, 1.0);
    }
    (st, dy, fs, fd)
}

/// « Nocc » (§5.2.3, éq. 131).
pub fn nocc(o: &OccupantsAcv) -> f64 {
    match o {
        OccupantsAcv::Logements(t) => t
            .iter()
            .map(|x| f64::from(x.type_t) * f64::from(x.nombre))
            .sum(),
        OccupantsAcv::Nombre(n) => *n,
    }
}

/// Contribution « composants » : cumuls par sous-lot (§4.2.1 et §4.2.2), en kg éq. CO2.
fn cumuls_composants(
    e: &EntreesAcv,
    defauts: &mut Vec<ValeurParDefaut>,
    fluides: &mut (Phases, Phases),
) -> BTreeMap<String, Cumul> {
    let d = e.donnees;
    let mut m: BTreeMap<String, Cumul> = BTreeMap::new();
    for (i, c) in d.composants.iter().enumerate() {
        let dve_p = dve(c, &format!("composants[{i}]"), defauts);
        let (st, dy, fs, fd) = composant((c.quantite, dve_p), c);
        ajouter(&mut fluides.0, fs, 1.0);
        ajouter(&mut fluides.1, fd, 1.0);
        let x = m.entry(c.sous_lot.clone()).or_default();
        ajouter(&mut x.statique, st, 1.0);
        ajouter(&mut x.dynamique, dy, 1.0);
        if c.donnee.type_donnee.est_par_defaut() {
            x.ded += dy.iter().sum::<f64>();
        }
        // Éq. (140) : Rp = 1.
        x.stock_c += c.quantite * stock_c(&c.donnee);
    }
    // §4.2.2 : valeurs forfaitaires (kg/m²) de l'usage de chaque zone, × Sref de la zone
    // (Q208).
    for k in &d.lots_forfaitaires {
        let x = m.entry(k.clone()).or_default();
        x.forfaitaire = true;
        for &(usage, sref) in e.zones {
            if let Some(v) = forfait(usage, k) {
                ajouter(&mut x.statique, v, sref);
                ajouter(&mut x.dynamique, v, sref);
            }
        }
        x.ded += x.dynamique.iter().sum::<f64>();
    }
    // Cas 3 des fluides : ajoutés au lot forfaitaire (§4.2.2.1, §4.2.2.2.2).
    for (i, f) in d.fluides_lots_forfaitaires.iter().enumerate() {
        let dve_p = f.dve_ans.unwrap_or_else(|| {
            defauts.push(ValeurParDefaut {
                champ: format!("fluides_lots_forfaitaires[{i}].dve_ans"),
                valeur: DVE_LOT_FORFAITAIRE_ANS,
                regle: "annexe II §4.2.2.2.2 : DVEp = 17 ans".into(),
            });
            DVE_LOT_FORFAITAIRE_ANS
        });
        let (ex, fdv) = fluide_cas3(f.charge_kg, f.prg_kgco2_kg, dve_p);
        let s = fluide_statique(f.charge_kg, f.de_kgco2_kg);
        let x = m.entry(f.sous_lot.clone()).or_default();
        x.statique[2] += s;
        x.dynamique[2] += ex;
        x.dynamique[3] += fdv;
        x.ded += ex + fdv;
        fluides.0[2] += s;
        fluides.1[2] += ex;
        fluides.1[3] += fdv;
    }
    m
}

/// Contribution « chantier » (§4.5), en kg éq. CO2 (phase d'édification).
fn chantier(e: &EntreesAcv, defauts: &mut Vec<ValeurParDefaut>) -> f64 {
    let s = &e.donnees.donnees_service;
    let trans = s.transport_terres_kgco2_tkm.unwrap_or(0.0);
    let eau_pot = s.eau_potable_kgco2_m3.unwrap_or(0.0);
    let ac = s.assainissement_collectif_kgco2_m3.unwrap_or(0.0);
    let importees = |t: &[crate::entree::TerresImportees], defauts: &mut Vec<_>| -> f64 {
        t.iter()
            .enumerate()
            .map(|(i, x)| {
                let d = x.distance_km.unwrap_or_else(|| {
                    defauts.push(ValeurParDefaut {
                        champ: format!("chantier.terres_importees[{i}].distance_km"),
                        valeur: DISTANCE_DEFAUT_KM,
                        regle: "annexe II §4.5.2 : distance d'approvisionnement de 30 km".into(),
                    });
                    DISTANCE_DEFAUT_KM
                });
                x.quantite_t * d * trans
            })
            .sum()
    };
    let evacuees = |t: &[crate::entree::TerresEvacuees]| -> f64 {
        t.iter()
            .map(|x| x.quantite_t * (x.distance_km * trans + x.de_traitement_kgco2_t))
            .sum()
    };
    match &e.donnees.chantier {
        ChantierAcv::Detaille {
            energies,
            eau_potable_m3,
            eaux_usees_m3,
            terres_importees,
            terres_evacuees,
            composants,
        } => {
            // Éq. (93) à (97).
            let energie: f64 = energies.iter().map(|x| x.quantite * x.de_kgco2_unite).sum();
            let eau = eau_potable_m3 * eau_pot + eaux_usees_m3 * ac;
            let terres = importees(terres_importees, defauts) + evacuees(terres_evacuees);
            let comp: f64 = composants
                .iter()
                .map(|c| composant_chantier(c.quantite, &c.donnee))
                .sum();
            energie + eau + terres + comp
        }
        ChantierAcv::SimplifieMaison {
            s_parcelle_m2,
            s_emprise_sol_m2,
            terres_importees,
        } => {
            // Éq. (98) : énergie négligée ; éq. (99) : 10 m³.
            let eau = 10.0 * eau_pot + 10.0 * ac;
            // Éq. (100) et (101).
            let q_evac = if *s_parcelle_m2 > 500.0 {
                0.0
            } else {
                1.12 * MU_TERRES_T_M3 * s_emprise_sol_m2
            };
            let evac =
                q_evac * (DISTANCE_DEFAUT_KM * trans + s.traitement_inertes_kgco2_t.unwrap_or(0.0));
            eau + evac + importees(terres_importees, defauts)
        }
        ChantierAcv::SimplifieAutres {
            n_ete_grue,
            n_hiver_grue,
            n_ete_sans_grue,
            n_hiver_sans_grue,
            v_terres_excavees_m3,
            epsilon1_l_m3,
            terres_importees,
            terres_evacuees,
        } => {
            // Éq. (102).
            let q_elec = 10400.0 * n_ete_grue
                + 19500.0 * n_hiver_grue
                + 5200.0 * n_ete_sans_grue
                + 10400.0 * n_hiver_sans_grue;
            // Éq. (103).
            let eps = epsilon1_l_m3.unwrap_or_else(|| {
                defauts.push(ValeurParDefaut {
                    champ: "chantier.epsilon1_l_m3".into(),
                    valeur: EPSILON1_L_M3,
                    regle: "annexe II §4.5.2.1.2 : ε1 = 1 L/m³".into(),
                });
                EPSILON1_L_M3
            });
            let q_carbu = eps * v_terres_excavees_m3;
            let energie = q_elec * s.electricite_chantier_kgco2_kwh.unwrap_or(0.0)
                + q_carbu * s.carburant_kgco2_l.unwrap_or(0.0);
            // Éq. (104).
            let q_eau =
                60.0 * (n_ete_grue + n_hiver_grue) + 40.0 * (n_ete_sans_grue + n_hiver_sans_grue);
            let eau = q_eau * (eau_pot + ac);
            energie + eau + importees(terres_importees, defauts) + evacuees(terres_evacuees)
        }
    }
}

/// Usage des zones pour l'eau : la consommation conventionnelle commune à toutes les zones
/// (Q210) ; `None` si elles diffèrent ou si le tableau 7 n'en donne pas.
pub fn q_conv_batiment(zones: &[(u8, f64)]) -> Option<f64> {
    let mut v = zones.iter().map(|(u, _)| q_eau_potable_conv_m3(*u));
    let premier = v.next()??;
    v.all(|x| x == Some(premier)).then_some(premier)
}

/// Calcule l'ACV d'un bâtiment.
pub fn calculer_acv(e: &EntreesAcv) -> ResultatsAcv {
    let d = e.donnees;
    let sref = e.sref_m2;
    let s = &d.donnees_service;
    let mut defauts = Vec::new();
    let mut fluides = ([0.0; 5], [0.0; 5]);
    let somme_f = somme_f_co2();

    // Composants, par sous-lot puis par lot (éq. 15, 21, 22, 27 à 34, 140 à 142).
    let cumuls = cumuls_composants(e, &mut defauts, &mut fluides);
    let tac = e.tac_pv_pct.unwrap_or(0.0) / 100.0;
    let avec_lot13 = cumuls.contains_key("13");
    let mut par_lot: BTreeMap<u8, Vec<(String, Cumul)>> = BTreeMap::new();
    for (k, mut c) in cumuls {
        let lot = lot_de(&k).or_else(|| donnees::lot_forfait(&k)).unwrap_or(0);
        if lot == 13 {
            // Éq. (22) : prorata de l'autoconsommation ; même affectation pour StockC
            // (§5.3.2.2) et Icded.
            c.statique = c.statique.map(|x| x * tac);
            c.dynamique = c.dynamique.map(|x| x * tac);
            c.ded *= tac;
            c.stock_c *= tac;
        }
        par_lot.entry(lot).or_default().push((k, c));
    }
    let mut lots = Vec::new();
    let (mut st_tot, mut dy_tot) = ([0.0; 5], [0.0; 5]);
    let (mut ded_total, mut ded_reglementaire, mut stock_tot) = (0.0, 0.0, 0.0);
    for (lot, sl) in par_lot {
        let (mut st, mut dy, mut ded, mut stock) = ([0.0; 5], [0.0; 5], 0.0, 0.0);
        let mut sous_lots = Vec::new();
        for (k, c) in sl {
            ajouter(&mut st, c.statique, 1.0);
            ajouter(&mut dy, c.dynamique, 1.0);
            ded += c.ded;
            stock += c.stock_c;
            sous_lots.push(ResultatsSousLotAcv {
                sous_lot: k,
                forfaitaire: c.forfaitaire,
                contribution: ContributionAcv {
                    i_statique: par_m2(c.statique, sref),
                    ic: par_m2(c.dynamique, sref),
                },
                ic_ded_kgco2_m2: c.ded / sref,
                stock_c_kgc_m2: c.stock_c / sref,
            });
        }
        ajouter(&mut st_tot, st, 1.0);
        ajouter(&mut dy_tot, dy, 1.0);
        ded_total += ded;
        if !matches!(lot, 1 | 2 | 13) {
            ded_reglementaire += ded;
        }
        stock_tot += stock;
        let ic = par_m2(dy, sref);
        lots.push(ResultatsLotAcv {
            lot,
            contribution: ContributionAcv {
                i_statique: par_m2(st, sref),
                ic,
            },
            ic_kgco2_m2: ic.total,
            ic_ded_kgco2_m2: ded / sref,
            udd_pct: (ic.total != 0.0).then(|| 100.0 * ded / sref / ic.total),
            stock_c_kgc_m2: stock / sref,
            sous_lots,
        });
    }
    let composants = ContributionAcv {
        i_statique: par_m2(st_tot, sref),
        ic: par_m2(dy_tot, sref),
    };

    // Chantier (éq. 97) : édification, statique = dynamique.
    let ch = chantier(e, &mut defauts) / sref;
    let chantier = ContributionAcv {
        i_statique: PhasesAcv::depuis([0.0, ch, 0.0, 0.0, 0.0]),
        ic: PhasesAcv::depuis([0.0, ch, 0.0, 0.0, 0.0]),
    };

    // Eau (§4.4.2 : calcul simplifié ; éq. 80, 81, 84, 87 à 90).
    let nocc = nocc(&d.occupants);
    let q_conv = q_conv_batiment(e.zones).unwrap_or(0.0);
    let q_pot = q_conv * nocc; // éq. (91)
    let q_usees = q_pot; // éq. (92)
    let de_pot = s.eau_potable_kgco2_m3.unwrap_or(0.0);
    let de_ac = s.assainissement_collectif_kgco2_m3.unwrap_or(0.0);
    let de_ass = match d.eau.assainissement {
        Assainissement::Collectif => de_ac,
        Assainissement::NonCollectif => s.assainissement_non_collectif_kgco2_m3.unwrap_or(0.0),
    };
    let q_pluv = match d.eau.eaux_pluviales {
        // Éq. (85).
        GestionEauxPluviales::Unitaire => {
            d.eau.pluviometrie_mm.unwrap_or(0.0) / 1000.0
                * (d.eau.s_toitures_m2.unwrap_or(0.0)
                    + d.eau.s_parkings_impermeabilises_m2.unwrap_or(0.0))
        }
        GestionEauxPluviales::Infiltration | GestionEauxPluviales::Separatif => 0.0,
    };
    let annuel_eau = q_pot * de_pot + q_usees * de_ass + q_pluv * de_ac;
    let eau = ContributionAcv {
        i_statique: PhasesAcv::depuis([0.0, 0.0, annuel_eau * PER_ANS / sref, 0.0, 0.0]),
        ic: PhasesAcv::depuis([0.0, 0.0, annuel_eau * somme_f / sref, 0.0, 0.0]),
    };

    // Parcelle (§4.6, eau en calcul simplifié §4.6.2).
    let mut q_parc = (0.0, 0.0);
    let mut stock_parcelle = None;
    let parcelle = d.parcelle.as_ref().map(|p| {
        let (mut st, mut dy, mut stock) = ([0.0; 5], [0.0; 5], 0.0);
        for (i, c) in p.composants.iter().enumerate() {
            let dve_p = dve(c, &format!("parcelle.composants[{i}]"), &mut defauts);
            let (a, b, _, _) = composant((c.quantite, dve_p), c);
            ajouter(&mut st, a, 1.0);
            ajouter(&mut dy, b, 1.0);
            stock += c.quantite * stock_c(&c.donnee);
        }
        stock_parcelle = Some(stock / sref);
        // Éq. (106).
        let trans = s.transport_terres_kgco2_tkm.unwrap_or(0.0);
        let carbu = s.carburant_kgco2_l.unwrap_or(0.0);
        if !p.comblements.is_empty() {
            defauts.push(ValeurParDefaut {
                champ: "parcelle.comblements".into(),
                valeur: EPSILON2_L_T,
                regle: "annexe II §4.6.1.1.2 : ε2 = 0,7 L/t".into(),
            });
        }
        let comb: f64 = p
            .comblements
            .iter()
            .map(|x| x.quantite_t * (x.distance_km.unwrap_or(0.0) * trans + EPSILON2_L_T * carbu))
            .sum();
        st[1] += comb;
        dy[1] += comb;
        // Éq. (108), (109), (121) à (123), (112).
        let nettoyage = p.s_nettoyee_m2 * 0.001 * 4.0;
        let q_pot_p = p.s_vegetalisee_m2 * 0.0033 * 20.0 + nettoyage;
        let q_usees_p =
            d.eau.pluviometrie_mm.unwrap_or(0.0) / 1000.0 * p.s_impermeabilisees_m2 + nettoyage;
        q_parc = (q_pot_p, q_usees_p);
        let annuel = q_pot_p * de_pot + q_usees_p * de_ac;
        st[2] += annuel * PER_ANS; // éq. (110), (113)
        dy[2] += annuel * somme_f; // éq. (119), (120)
        ContributionAcv {
            i_statique: par_m2(st, sref),
            ic: par_m2(dy, sref),
        }
    });

    // §5.3.1.
    let ic_composants = composants.ic.total;
    let ic_construction = ic_composants + ch; // éq. (132)
    let ic_eau = eau.ic.total;
    let ic_batiment = e.ic_energie_kgco2_m2.map(|x| ic_construction + x + ic_eau); // éq. (134)
                                                                                   // Éq. (118) : sans module D.
    let ic_parcelle = parcelle.map(|p| p.ic.total - p.ic.module_d);
    let ic_projet = ic_batiment.zip(ic_parcelle).map(|(b, p)| b + p); // éq. (136)
    let occ = |x: f64| if nocc > 0.0 { x * sref / nocc } else { 0.0 };
    ResultatsAcv {
        per_ans: 50,
        composants,
        fluides_frigorigenes: ContributionAcv {
            i_statique: par_m2(fluides.0, sref),
            ic: par_m2(fluides.1, sref),
        },
        chantier,
        eau,
        parcelle,
        lots,
        ic_composants_kgco2_m2: ic_composants,
        ic_chantier_kgco2_m2: ch,
        ic_eau_kgco2_m2: ic_eau,
        ic_energie_kgco2_m2: e.ic_energie_kgco2_m2,
        ic_construction_kgco2_m2: ic_construction,
        ic_batiment_kgco2_m2: ic_batiment,
        ic_parcelle_kgco2_m2: ic_parcelle,
        ic_projet_kgco2_m2: ic_projet,
        ic_construction_occ_kgco2: occ(ic_construction), // éq. (133)
        ic_batiment_occ_kgco2: ic_batiment.map(occ),     // éq. (135)
        ic_projet_occ_kgco2: ic_projet.map(occ),         // éq. (137)
        ic_ded_kgco2_m2: ded_reglementaire / sref,
        ic_ded_total_kgco2_m2: ded_total / sref,
        udd_batiment_pct: (ic_composants != 0.0).then(|| 100.0 * ded_total / sref / ic_composants), // éq. (146)
        stock_c_kgc_m2: stock_tot / sref,
        stock_c_parcelle_kgc_m2: stock_parcelle,
        eau_quantites: QuantitesEauAcv {
            nocc,
            q_eau_potable_conv_m3: q_conv,
            q_eau_potable_m3: q_pot,
            q_eaux_usees_m3: q_usees,
            q_eaux_pluviales_m3: q_pluv,
            q_parcelle_eau_potable_m3: q_parc.0,
            q_parcelle_eaux_usees_m3: q_parc.1,
        },
        tac_pv_pct: avec_lot13.then_some(tac * 100.0),
        exigences: None,
        valeurs_par_defaut: defauts,
    }
}

/// Exigence `Icconstruction_max` du bâtiment (annexe de l'article R. 172-4, chapitre II, III
/// et V ; chapitre III, III) et vérification, ajoutée aux exigences du bâtiment et à sa
/// conformité. Sans `exigences` du bâtiment, rien n'est calculé ; un usage non couvert laisse
/// l'exigence absente et l'indicateur est déclaré manquant.
pub fn verifier_exigence(
    a: &mut ResultatsAcv,
    bat: &crate::entree::Batiment,
    zones: &[crate::resultats::ResultatsZone],
    zone_climatique: thbce_data::ZoneClimatique,
    alt_m: f64,
    resultats_exigences: Option<&mut crate::indicateurs::resultats::ResultatsExigences>,
) {
    let (Some(p), Some(d)) = (&bat.exigences, &bat.acv) else {
        return;
    };
    let lot = |n: u8| {
        a.lots
            .iter()
            .find(|l| l.lot == n)
            .map_or(0.0, |l| l.ic_kgco2_m2)
    };
    let ind = exigences::IndicateursModulation {
        ic_lot1: lot(1),
        ic_lot2: lot(2),
        ic_lot13: lot(13),
        ic_ded: a.ic_ded_kgco2_m2,
    };
    let rcu = crate::indicateurs::sorties_th_c::raccorde_rcu(bat);
    let mut parties = Vec::new();
    for (z, r) in bat.zones.iter().zip(zones) {
        let sref_usage = bat
            .zones
            .iter()
            .zip(zones)
            .filter(|(x, _)| x.usage == z.usage)
            .map(|(_, y)| y.sref_m2)
            .sum();
        let zi = exigences::ZoneIc {
            id: &z.id,
            usage: z.usage.numero(),
            sref_m2: r.sref_m2,
            nb_logements: z.nb_logements.unwrap_or(1),
            sref_usage_m2: sref_usage,
            raccorde_rcu: rcu,
        };
        match exigences::exigence_zone(&zi, p, &d.exigences, zone_climatique, alt_m, ind) {
            Some(x) => parties.push(x),
            None => {
                if let Some(x) = resultats_exigences {
                    x.indicateurs_manquants.push("ic_construction".to_owned());
                    x.conforme = false;
                }
                return;
            }
        }
    }
    let s: f64 = parties.iter().map(|x| x.sref_m2).sum();
    let max = if s > 0.0 {
        parties
            .iter()
            .map(|x| x.ic_construction_max_kgco2_m2 * x.sref_m2)
            .sum::<f64>()
            / s
    } else {
        0.0
    };
    let v = crate::indicateurs::sorties_th_c::verifier(
        "ic_construction",
        None,
        a.ic_construction_kgco2_m2,
        max,
    );
    a.exigences = Some(crate::resultats::acv::ExigencesIcConstruction {
        ic_construction_max_kgco2_m2: max,
        zones: parties,
        respectee: v.respectee,
    });
    if let Some(x) = resultats_exigences {
        x.ic_construction_max_kgco2_m2 = Some(max);
        x.conforme = x.conforme && v.respectee;
        x.verifications.push(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entree::{DonneesService, EauAcv, LogementsType};

    fn proche(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} ≠ {b}");
    }

    fn donnees() -> DonneesAcv {
        DonneesAcv {
            composants: vec![],
            lots_forfaitaires: vec!["8.1".into()],
            fluides_lots_forfaitaires: vec![],
            chantier: ChantierAcv::SimplifieMaison {
                s_parcelle_m2: 450.0,
                s_emprise_sol_m2: 100.0,
                terres_importees: vec![],
            },
            eau: EauAcv {
                assainissement: Assainissement::Collectif,
                eaux_pluviales: GestionEauxPluviales::Unitaire,
                pluviometrie_mm: Some(600.0),
                s_toitures_m2: Some(100.0),
                s_parkings_impermeabilises_m2: None,
            },
            parcelle: None,
            occupants: OccupantsAcv::Logements(vec![
                LogementsType {
                    type_t: 4,
                    nombre: 1,
                },
                LogementsType {
                    type_t: 2,
                    nombre: 1,
                },
            ]),
            exigences: crate::entree::ParametresExigencesAcv::default(),
            donnees_service: DonneesService {
                eau_potable_kgco2_m3: Some(0.4),
                assainissement_collectif_kgco2_m3: Some(0.6),
                transport_terres_kgco2_tkm: Some(0.1),
                traitement_inertes_kgco2_t: Some(3.0),
                source: "valeurs fictives".into(),
                ..DonneesService::default()
            },
        }
    }

    #[test]
    fn maison_calculee_a_la_main() {
        let d = donnees();
        let r = calculer_acv(&EntreesAcv {
            donnees: &d,
            sref_m2: 100.0,
            zones: &[(1, 100.0)],
            ic_energie_kgco2_m2: Some(150.0),
            tac_pv_pct: None,
        });
        // Nocc = 4 + 2 = 6 (éq. 131) ; Qeau potable = 48 × 6 = 288 m³/an (éq. 91).
        proche(r.eau_quantites.nocc, 6.0);
        proche(r.eau_quantites.q_eau_potable_m3, 288.0);
        // Réseau unitaire : 600/1000 × 100 = 60 m³/an (éq. 85).
        proche(r.eau_quantites.q_eaux_pluviales_m3, 60.0);
        // 288 × 0,4 + 288 × 0,6 + 60 × 0,6 = 324 kg/an ; × 39,543 / 100 m².
        proche(r.ic_eau_kgco2_m2, 324.0 * 39.543 / 100.0);
        proche(r.eau.i_statique.total, 324.0 * 50.0 / 100.0);
        // Chantier : 10 m³ × (0,4 + 0,6) + 1,12 × 1,45 × 100 t × (30 × 0,1 + 3) = 984,4 kg.
        proche(r.ic_chantier_kgco2_m2, 9.844);
        // Forfait 8.1 de l'usage 1 : 61 + 0 + 106 + 3 + 0 = 170 kg/m², données par défaut.
        proche(r.ic_composants_kgco2_m2, 170.0);
        proche(r.ic_ded_kgco2_m2, 170.0);
        proche(r.udd_batiment_pct.unwrap_or(0.0), 100.0);
        proche(r.ic_construction_kgco2_m2, 179.844);
        proche(
            r.ic_batiment_kgco2_m2.unwrap_or(0.0),
            179.844 + 150.0 + 324.0 * 39.543 / 100.0,
        );
        // Icconstruction,occ = Ic × Sref / Nocc (éq. 133).
        proche(r.ic_construction_occ_kgco2, 179.844 * 100.0 / 6.0);
    }
}
