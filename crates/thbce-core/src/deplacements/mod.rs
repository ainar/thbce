// SPDX-License-Identifier: GPL-3.0-or-later
//! Postes propres au logement collectif et aux parkings (lot 6e) : éclairage des parties
//! communes (§7.2), ascenseurs (§10.1), ventilation (§10.3) et éclairage (§10.4) des parkings.
//!
//! Ces consommations, électriques, ne dépendent ni du modèle thermique ni des systèmes : elles
//! se calculent à partir des scénarios conventionnels (« Uk ») et de la description saisie.
//! Elles sont restituées par zone et par bâtiment sous quatre postes distincts
//! ([`ConsommationsCollectif`]) pour le calcul des indicateurs (éq. 2510, 2511 et 2513).
//! Aucune n'entraîne d'apport interne : voir `docs/questions.md`, Q50.

pub mod ascenseurs;
pub mod parkings;

use std::collections::BTreeMap;

use crate::climat::calendrier::date_scenario;
use crate::conventions::NB_HEURES_AN;
use crate::eclairage::parties_communes;
use crate::entree::{Batiment, UsageZone};
use crate::erreur::Erreur;
use crate::resultats::{
    ConsommationsCollectif, PosteElectrique, ResultatsAscenseur, ResultatsParking,
};
use crate::scenarios::scenarios_conventionnels::ScenariosZone;

use ascenseurs::{Lectures, ZoneDesservie};
use parkings::TypeUsageParking;

/// Résultats du lot 6e pour un bâtiment.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultatCollectif {
    /// Postes de chaque zone, dans l'ordre des zones.
    pub zones: Vec<ConsommationsCollectif>,
    /// Postes du bâtiment : somme des zones.
    pub batiment: ConsommationsCollectif,
    /// Cabines d'ascenseur.
    pub ascenseurs: Vec<ResultatsAscenseur>,
    /// Parkings.
    pub parkings: Vec<ResultatsParking>,
}

/// Cumule une série horaire en Wh en un poste en kWh (annuel et douze mois des scénarios).
fn poste(serie_wh: &[f64], sref_m2: f64) -> PosteElectrique {
    let mut mensuel = vec![0.0; 12];
    for (h, w) in serie_wh.iter().enumerate() {
        mensuel[usize::from(date_scenario(h).mois) - 1] += w / 1000.0;
    }
    let annuel: f64 = mensuel.iter().sum();
    PosteElectrique {
        annuel_kwh: annuel,
        annuel_kwh_m2: if sref_m2 > 0.0 { annuel / sref_m2 } else { 0.0 },
        mensuel_kwh: mensuel,
    }
}

/// Calcule les postes du lot 6e d'un bâtiment. `sref_zones_m2` : « SREF_zn » de chaque zone.
///
/// Renvoie `None` si le bâtiment n'a ni zone de logement collectif ni parking.
///
/// # Errors
///
/// Erreur de scénarios d'une zone (usage non pris en charge).
pub fn calculer(
    bat: &Batiment,
    sref_zones_m2: &[f64],
    lectures: &Lectures,
) -> Result<Option<ResultatCollectif>, Erreur> {
    let a_communs = bat
        .zones
        .iter()
        .any(|z| z.usage == UsageZone::LogementCollectif);
    if !a_communs && bat.parkings.is_empty() {
        return Ok(None);
    }
    let sref_bat: f64 = sref_zones_m2.iter().sum();
    let n_zones = bat.zones.len();

    // Scénarios de chaque zone : « Uk(h) » et nombre maximal d'occupants « NBz » (éq. 2280).
    let mut uk: Vec<Vec<f64>> = Vec::with_capacity(n_zones);
    let mut nb_max: Vec<f64> = Vec::with_capacity(n_zones);
    for z in &bat.zones {
        let a_gr: Vec<f64> = z.groupes.iter().map(|g| g.su_m2.unwrap_or(0.0)).collect();
        let sc = ScenariosZone::nouveau(z.usage, &a_gr, z.nb_lgt.unwrap_or(1))?;
        let sorties: Vec<_> = (0..NB_HEURES_AN).map(|h| sc.evaluer(h)).collect();
        uk.push(sorties.iter().map(|s| s.uk).collect());
        nb_max.push(sorties.iter().map(|s| s.n_occ).fold(0.0, f64::max));
    }

    let mut series_zone: Vec<[Vec<f64>; 4]> = (0..n_zones)
        .map(|_| std::array::from_fn(|_| vec![0.0; NB_HEURES_AN]))
        .collect();

    // §7.2, éq. (792) et (793).
    for (iz, z) in bat.zones.iter().enumerate() {
        for h in 0..NB_HEURES_AN {
            series_zone[iz][0][h] =
                parties_communes::consommation_wh(z.usage, uk[iz][h], sref_zones_m2[iz]);
        }
    }

    // §10.1 : ascenseurs.
    let indice_zone: BTreeMap<&str, usize> = bat
        .zones
        .iter()
        .enumerate()
        .map(|(i, z)| (z.id.as_str(), i))
        .collect();
    // Éq. (2279) : Qz.
    let mut q_z = vec![0.0; n_zones];
    for a in &bat.ascenseurs {
        for id in &a.zones_desservies {
            if let Some(&iz) = indice_zone.get(id.as_str()) {
                q_z[iz] += a.q_kg;
            }
        }
    }
    let mut resultats_ascenseurs = Vec::new();
    for a in &bat.ascenseurs {
        let desservies: Vec<usize> = a
            .zones_desservies
            .iter()
            .filter_map(|id| indice_zone.get(id.as_str()).copied())
            .collect();
        let zones: Vec<ZoneDesservie<'_>> = desservies
            .iter()
            .map(|&iz| ZoneDesservie {
                q_z_kg: q_z[iz],
                nb_z: nb_max[iz],
                bv_z: ascenseurs::bv(bat.zones[iz].usage.numero()).unwrap_or(0.0),
                u_z: &uk[iz],
                habitation: bat.zones[iz].usage.est_habitation(),
            })
            .collect();
        let r = ascenseurs::calculer_cabine(a, &zones, lectures);
        // Éq. (2309) et (2310) : répartition entre les zones desservies selon `Qz`.
        let somme_q: f64 = desservies.iter().map(|&iz| q_z[iz]).sum();
        for &iz in &desservies {
            let part = q_z[iz] / somme_q;
            for (x, p) in series_zone[iz][1].iter_mut().zip(&r.pcab_wh) {
                *x += p * part;
            }
        }
        resultats_ascenseurs.push(ResultatsAscenseur {
            id: a.id.clone(),
            n_dem: r.n_dem,
            t_mob_h: r.t_mob_h,
            pti_w: r.pti_w,
            e_tm_kwh: r.etm_wh / 1000.0,
            e_ti_kwh: r.eti_wh / 1000.0,
            e_tot_kwh: r.etot_wh / 1000.0,
            valeurs_par_defaut: r.valeurs_par_defaut,
        });
    }

    // §10.3 et §10.4 : parkings, répartis au prorata des surfaces de référence des zones.
    // Les zones calculées sont d'habitation : « Typeusage » = habitat (tableau de §10.3.2).
    let mut resultats_parkings = Vec::new();
    for p in &bat.parkings {
        let (vent, mut defauts) = parkings::ventilation_habitat(p);
        let (ecl, d) = parkings::eclairage(p, TypeUsageParking::Habitat);
        defauts.extend(d);
        for iz in 0..n_zones {
            let part = if sref_bat > 0.0 {
                sref_zones_m2[iz] / sref_bat
            } else {
                0.0
            };
            for h in 0..NB_HEURES_AN {
                series_zone[iz][2][h] += vent[h] * part;
                series_zone[iz][3][h] += ecl[h] * part;
            }
        }
        resultats_parkings.push(ResultatsParking {
            id: p.id.clone(),
            e_vent_kwh: vent.iter().sum::<f64>() / 1000.0,
            e_ecl_kwh: ecl.iter().sum::<f64>() / 1000.0,
            valeurs_par_defaut: defauts,
        });
    }

    let zones: Vec<ConsommationsCollectif> = series_zone
        .iter()
        .zip(sref_zones_m2)
        .map(|(s, &sref)| ConsommationsCollectif {
            eclairage_parties_communes: poste(&s[0], sref),
            ascenseurs: poste(&s[1], sref),
            ventilation_parkings: poste(&s[2], sref),
            eclairage_parkings: poste(&s[3], sref),
        })
        .collect();
    // Éq. (2517) : somme des zones, ramenée à la surface du bâtiment.
    let somme = |f: fn(&ConsommationsCollectif) -> &PosteElectrique| {
        let mut mensuel = vec![0.0; 12];
        for z in &zones {
            for (m, x) in mensuel.iter_mut().zip(&f(z).mensuel_kwh) {
                *m += x;
            }
        }
        let annuel: f64 = mensuel.iter().sum();
        PosteElectrique {
            annuel_kwh: annuel,
            annuel_kwh_m2: if sref_bat > 0.0 {
                annuel / sref_bat
            } else {
                0.0
            },
            mensuel_kwh: mensuel,
        }
    };
    let batiment = ConsommationsCollectif {
        eclairage_parties_communes: somme(|c| &c.eclairage_parties_communes),
        ascenseurs: somme(|c| &c.ascenseurs),
        ventilation_parkings: somme(|c| &c.ventilation_parkings),
        eclairage_parkings: somme(|c| &c.eclairage_parkings),
    };
    Ok(Some(ResultatCollectif {
        zones,
        batiment,
        ascenseurs: resultats_ascenseurs,
        parkings: resultats_parkings,
    }))
}
