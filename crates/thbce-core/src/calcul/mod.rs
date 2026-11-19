// SPDX-License-Identifier: GPL-3.0-or-later
//! Point d'entrée du calcul.
//!
//! État : le calcul valide l'entrée et les données météo, détermine le climat du site (lot 3) et
//! les surfaces de référence, puis déroule deux années au pas horaire : en mode Th-B (lot 4),
//! pour les besoins de chauffage, de refroidissement et d'éclairage et le Bbio (§13.1) ; puis en
//! mode Th-D (lot 5), sur le jeu météo à séquence caniculaire, pour l'indicateur de confort
//! d'été DH (§13.5) ; enfin en mode Th-C (lot 6b), sur le jeu « Th-BC », pour les demandes des
//! systèmes de chauffage et de refroidissement (§8). Les calculs emploient le même enchaînement
//! horaire, paramétré par [`ModeCalcul`] ; les calculs Th-D et Th-C emploient la ventilation
//! (§6.2 à 6.5) et les émetteurs (§8.1) du projet. Les indicateurs réglementaires (Cep,
//! Cep,nr, Ic énergie, exigences et conformité, §13.2 à 13.4) sont calculés à partir des
//! consommations du calcul Th-C, relevées heure par heure pour l'autoconsommation (lot 7).

use crate::bati::assemblage_batiment::BatimentHoraire;
use crate::bati::assemblage_groupe::SortiesPasGroupe;
use crate::bati::classement_bruit::projet_avec_classes_deduites;
use crate::climat::{ClimatExterieur, ConfortAdaptatif, MeteoProjet};
use crate::conventions::{ModeCalcul, NB_HEURES_AN};
use crate::entree::{Batiment, Projet};
use crate::erreur::Erreur;
use crate::indicateurs::bilans_energies::{bilan, poste, ReleveElectrique};
use crate::indicateurs::sorties_th_b::{sorties, EnergiesMensuelles};
use crate::indicateurs::sorties_th_c::{calculer_indicateurs, EntreesIndicateurs};
use crate::indicateurs::sorties_zones::{
    calculer as calculer_sorties, EntreesSorties, ReleveZones,
};
use crate::pv::global::produire;
use crate::pv::ResultatsProductionPv;
use crate::resultats::{
    Consommations, PeriodeConfortAdaptatif, Resultats, ResultatsBatiment, ResultatsClimat,
    ResultatsEspaceTampon,
};
use crate::validation::valider;
use crate::{VERSION_METHODE, VERSION_MOTEUR};
use batiment::restituer_zone;
use ecs::{resultats_ecs_intergroupes, resultats_recuperateurs_collectifs};
use emission_distribution::{resultats_intergroupes, resultats_reseaux_cta};
use enveloppe::synthese_espace_tampon;
use generation::{avertissements_generations, resultats_generations};

// Restitution des résultats : un fichier par famille. Un lot ajoute la restitution de ses
// résultats dans le fichier de sa famille, et son appel dans `calculer` ou `restituer_zone`.
mod batiment;
mod brasseurs_air;
mod ecs;
mod emission_distribution;
mod enveloppe;
mod generation;
pub mod series;
mod solaire;
mod ventilation;
// Famille à venir : ACV (Icconstruction, annexe II) — `mod acv;`.

/// Options de calcul.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Arrondit les résultats à une décimale à la restitution (Th-BCE 2020 §1.3.1). Vrai par
    /// défaut ; à désactiver pour les tests et les comparaisons fines.
    pub arrondir: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { arrondir: true }
    }
}

/// Sorties facultatives demandées à [`calculer_avec`] ; aucune par défaut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Demandes {
    /// Séries horaires principales de chaque bâtiment (`series_horaires` ; Q118).
    pub series: bool,
    /// Sorties par zone et par groupe de la fiche 13.2 (`sorties_detaillees` ; Q116).
    pub sorties_detaillees: bool,
}

/// Contrôles communs à tout calcul : validité de l'entrée, usages pris en charge, données
/// météo de la zone climatique du projet.
fn verifier(
    projet: &Projet,
    meteo: &MeteoProjet,
) -> Result<crate::validation::RapportValidation, Erreur> {
    let rapport = valider(projet);
    if !rapport.est_valide() {
        return Err(Erreur::Saisie(rapport));
    }
    for (ib, bat) in projet.batiments.iter().enumerate() {
        for (iz, zone) in bat.zones.iter().enumerate() {
            if !zone.usage.est_pris_en_charge() {
                return Err(Erreur::NonPrisEnCharge {
                    chemin: format!("batiments[{ib}].zones[{iz}].usage"),
                    element: format!(
                        "l'usage n° {} (Th-BCE 2020 §4.1.3, tableau 4)",
                        zone.usage.numero()
                    ),
                });
            }
        }
    }
    meteo.verifier(projet.localisation.zone_climatique_retenue())?;
    Ok(rapport)
}

/// Calcule les indicateurs d'un projet à partir de sa description et des données météo de sa
/// zone climatique.
///
/// Trois calculs annuels s'enchaînent pour chaque bâtiment : Th-B sur le jeu météo « Th-BC »,
/// Th-D sur le jeu « Th-D » (annexe II §1.2.1, tableau 2), puis Th-C sur le jeu « Th-BC ».
pub fn calculer(
    projet: &Projet,
    meteo: &MeteoProjet,
    options: &Options,
) -> Result<Resultats, Erreur> {
    calculer_selon(projet, meteo, options, &Demandes::default())
}

/// Comme [`calculer`], et restitue en plus les sorties facultatives de `demandes`. Les autres
/// résultats sont identiques à ceux de [`calculer`].
pub fn calculer_avec(
    projet: &Projet,
    meteo: &MeteoProjet,
    options: &Options,
    demandes: &Demandes,
) -> Result<Resultats, Erreur> {
    calculer_selon(projet, meteo, options, demandes)
}

/// Comme [`calculer`], et restitue en plus les séries horaires principales de chaque bâtiment
/// (`series_horaires` des résultats du bâtiment : température extérieure, besoins et
/// températures des groupes, électricité par poste, consommations par poste et par énergie ;
/// Q118). Les autres résultats sont identiques à ceux de [`calculer`].
pub fn calculer_avec_series(
    projet: &Projet,
    meteo: &MeteoProjet,
    options: &Options,
) -> Result<Resultats, Erreur> {
    calculer_selon(
        projet,
        meteo,
        options,
        &Demandes {
            series: true,
            ..Demandes::default()
        },
    )
}

fn calculer_selon(
    projet: &Projet,
    meteo: &MeteoProjet,
    options: &Options,
    demandes: &Demandes,
) -> Result<Resultats, Erreur> {
    let avec_series = demandes.series;
    let rapport = verifier(projet, meteo)?;
    // Fiche 5.11 : la classe d'exposition au bruit déduite des baies prend la place de la saisie
    // absente (Q202).
    let resolu = projet_avec_classes_deduites(projet);
    let projet = resolu.as_ref().unwrap_or(projet);
    let loc = &projet.localisation;
    let zone = loc.zone_climatique_retenue();

    // Th-BCE 2020 §3.1 : climat du site pour chacun des deux jeux météo, calculé une fois avant
    // la boucle horaire.
    let climat_th_bc = ClimatExterieur::nouveau(&meteo.th_bc, loc.departement, loc.alt_m);
    let climat_th_d = ClimatExterieur::nouveau(&meteo.th_d, loc.departement, loc.alt_m);
    // Th-BCE 2020 §3.1.3, éq. (1) et (2), et §4.5.3.2.1, éq. (57) : moyenne glissante de la
    // température extérieure et période de confort adaptatif de chaque jeu météo.
    let confort_th_bc = ConfortAdaptatif::nouveau(&climat_th_bc.te_c);
    let confort_th_d = ConfortAdaptatif::nouveau(&climat_th_d.te_c);
    let periode = |c: &ConfortAdaptatif| {
        // Jours restitués de 1 à 365 (« Jour_année », §3.1.2).
        let numero = |j: Option<usize>| j.and_then(|j| u16::try_from(j + 1).ok());
        PeriodeConfortAdaptatif {
            premier_jour: numero(c.premier_jour),
            dernier_jour: numero(c.dernier_jour),
        }
    };
    let climat = ResultatsClimat {
        zone_climatique: zone,
        altcorr_m: climat_th_bc.altcorr_m,
        base_ext_c: climat_th_bc.base_ext_c,
        confort_adaptatif_th_bc: periode(&confort_th_bc),
        confort_adaptatif_th_d: periode(&confort_th_d),
    };

    let mut avertissements: Vec<String> = rapport
        .avertissements
        .iter()
        .map(|p| format!("{} : {}", p.chemin, p.message))
        .collect();

    let mut batiments = Vec::with_capacity(projet.batiments.len());
    for bat in &projet.batiments {
        let mut zones = Vec::with_capacity(bat.zones.len());
        let mut energies_bat = EnergiesMensuelles::default();
        // Mode Th-B, puis mode Th-D : même enchaînement horaire, climat et règles du mode. Le
        // bâtiment est simulé d'un bloc : ses espaces tampons sont calculés avant ses zones à
        // chaque pas de temps (§5.2).
        let th_b = simuler_batiment(bat, projet, ModeCalcul::ThB, &climat_th_bc, &confort_th_bc)?;
        let th_d = simuler_batiment(bat, projet, ModeCalcul::ThD, &climat_th_d, &confort_th_d)?;
        // Mode Th-C (calcul des consommations), sur le jeu météo « Th-BC » : systèmes du projet,
        // saisons effectives, relances et générations (consommations en énergie finale).
        // L'électricité consommée par poste est relevée à chaque pas de temps pour les bilans
        // horaires du calcul des indicateurs (§13.4).
        let mut releve = ReleveElectrique::default();
        let mut collecteur: Option<series::Collecteur> = None;
        // Fiche 13.2 : relevé par zone et par groupe, à la demande seulement.
        let mut releve_zones: Option<ReleveZones> = None;
        let th_c = simuler_batiment_avec(
            bat,
            projet,
            ModeCalcul::ThC,
            &climat_th_bc,
            &confort_th_bc,
            |h, b| {
                releve.relever(h, b);
                if demandes.sorties_detaillees {
                    releve_zones
                        .get_or_insert_with(|| ReleveZones::nouveau(b))
                        .relever(h, b);
                }
                if avec_series {
                    collecteur
                        .get_or_insert_with(|| series::Collecteur::nouveau(b))
                        .relever(h, b);
                }
            },
        )?;
        // §8.29, éq. (1499), (1506) et (1513) : consommations des générations attribuées aux
        // groupes, sommées par zone puis pour le bâtiment.
        let mut cef_groupes = th_c.systemes.cumuls_cef_groupes.iter();
        let avec_generations = !th_c.systemes.generations_horaires.is_empty();
        let mut consommations_bat = Consommations::default();
        for (zone, ((zb, zd), zc)) in bat
            .zones
            .iter()
            .zip(th_b.zones.iter().zip(&th_d.zones).zip(&th_c.zones))
        {
            let (mut resultats_zone, energies_zone) = restituer_zone(zone, projet, zb, zd, zc);
            energies_bat.ajouter(&energies_zone);
            if avec_generations {
                let mut consommations_zone = Consommations::default();
                for g in &mut resultats_zone.groupes {
                    let c = cef_groupes
                        .next()
                        .map(Consommations::depuis_cumuls)
                        .unwrap_or_default();
                    consommations_zone.ajouter(&c);
                    g.consommations = Some(c);
                }
                consommations_bat.ajouter(&consommations_zone);
                resultats_zone.consommations = Some(consommations_zone);
            }
            zones.push(resultats_zone);
        }
        avertissements.extend(avertissements_generations(&bat.id, &th_c));
        avertissements.extend(ventilation::avertissements_ventilation(&bat.id, &th_c));
        // Lot 6e : postes propres au logement collectif et aux parkings (§7.2, §10.1, §10.3 et
        // §10.4), calculés d'après les scénarios et la saisie.
        let sref_zones: Vec<f64> = zones.iter().map(|z| z.sref_m2).collect();
        let collectif = crate::deplacements::calculer(
            bat,
            &sref_zones,
            &crate::deplacements::ascenseurs::Lectures::default(),
        )?;
        let (conso_collectif, ascenseurs, parkings, escalators) = match collectif {
            Some(c) => {
                for (z, cz) in zones.iter_mut().zip(c.zones) {
                    z.consommations_collectif = Some(cz);
                }
                // Éq. (2510), (2511) et (2513) : éclairage des parties communes et des
                // parkings, ventilation des parkings, ascenseurs.
                let [ecl_pc, asc, vent_park, ecl_park] = &c.series_batiment_wh;
                releve.ajouter_serie(poste::ECL, ecl_pc);
                releve.ajouter_serie(poste::DEPL, asc);
                releve.ajouter_serie(poste::AUX_VENT, vent_park);
                releve.ajouter_serie(poste::ECL, ecl_park);
                // Éq. (2513) : les escalators sont comptés avec les ascenseurs.
                releve.ajouter_serie(poste::DEPL, &c.serie_escalators_wh);
                if let Some(rz) = &mut releve_zones {
                    for (iz, s) in c.series_zones_wh.iter().enumerate() {
                        rz.ajouter_serie(iz, poste::ECL, &s[0]);
                        rz.ajouter_serie(iz, poste::DEPL, &s[1]);
                        rz.ajouter_serie(iz, poste::AUX_VENT, &s[2]);
                        rz.ajouter_serie(iz, poste::ECL, &s[3]);
                    }
                    for (iz, s) in c.series_escalators_zones_wh.iter().enumerate() {
                        rz.ajouter_serie(iz, poste::DEPL, s);
                    }
                }
                (Some(c.batiment), c.ascenseurs, c.parkings, c.escalators)
            }
            None => (None, Vec::new(), Vec::new(), None),
        };
        // Éq. (2504) : ventilateurs des centrales, de puissance constante en habitation ; hors
        // habitation, puissance d'occupation ou d'inoccupation à chaque pas (éq. 636 à 639).
        for (iz, (z, zc)) in zones.iter().zip(&th_c.zones).enumerate() {
            if let Some(v) = &z.ventilation {
                let serie = zc
                    .ventilation
                    .as_ref()
                    .and_then(|vp| ventilation::puissance_horaire_w(vp, &zc.scenarios));
                match serie {
                    Some(serie) => {
                        releve.ajouter_serie(poste::AUX_VENT, &serie);
                        if let Some(rz) = &mut releve_zones {
                            rz.ajouter_serie(iz, poste::AUX_VENT, &serie);
                        }
                    }
                    None => {
                        releve.ajouter_constante(poste::AUX_VENT, v.p_vent_w);
                        if let Some(rz) = &mut releve_zones {
                            rz.ajouter_constante(iz, poste::AUX_VENT, v.p_vent_w);
                        }
                    }
                }
            }
        }
        // Fiche 4.7 : usages mobiliers, pour la consommation « tous usages » (éq. 2515).
        for (iz, z) in bat.zones.iter().enumerate() {
            let serie = crate::indicateurs::equipements_mobiliers::consommation_horaire_wh(z)?;
            releve.ajouter_serie(poste::MOB, &serie);
            if let Some(rz) = &mut releve_zones {
                rz.ajouter_serie(iz, poste::MOB, &serie);
            }
        }
        // §12 : production sur le jeu météo « Th-BC » (comme les consommations).
        let productions: Vec<_> = bat
            .installations_pv
            .iter()
            .map(|i| produire(i, &projet.catalogue, &climat_th_bc))
            .collect();
        let mut prod_pv_wh = vec![
            0.0;
            if productions.is_empty() {
                0
            } else {
                NB_HEURES_AN
            }
        ];
        for p in &productions {
            for (t, x) in prod_pv_wh.iter_mut().zip(&p.p_ond_w) {
                *t += x;
            }
        }
        let c_aux_vent: Vec<f64> = zones
            .iter()
            .filter_map(|z| z.ventilation.as_ref().map(|v| v.c_aux_vent_kwh))
            .collect();
        // Th-BCE 2020 §13.1.3, éq. (2378) : SREF_bat = somme des SREF_zn du bâtiment.
        let sref_m2: f64 = zones.iter().map(|z| z.sref_m2).sum();
        let bbio = sorties(&energies_bat, sref_m2);
        let series_horaires = collecteur.map(|c| c.finir(&climat_th_bc.te_c, &releve));
        let generations = resultats_generations(&th_c);
        // §13.2 à 13.4 : indicateurs du bâtiment, dès qu'il a des générations.
        let c_ecl_zones_kwh: Vec<f64> = th_c
            .zones
            .iter()
            .map(|z| {
                z.groupes
                    .iter()
                    .map(|g| g.cumuls.c_ecl_kwh.iter().sum::<f64>())
                    .sum()
            })
            .collect();
        let c_ecl_collectif_kwh = conso_collectif.as_ref().map_or(0.0, |c| {
            c.eclairage_parties_communes.annuel_kwh + c.eclairage_parkings.annuel_kwh
        });
        let indicateurs = avec_generations.then(|| {
            let bilan = bilan(&releve.horaire, &prod_pv_wh);
            let (mut i, a) = calculer_indicateurs(&EntreesIndicateurs {
                batiment: bat,
                zone_climatique: zone,
                alt_m: loc.alt_m,
                zones: &zones,
                sref_m2,
                bbio_pts: Some(bbio.annuel.bbio_pts),
                bilan: &bilan,
                consommations: &consommations_bat.annuel,
                generations: &generations,
                c_ecl_zones_kwh: &c_ecl_zones_kwh,
                c_ecl_collectif_kwh,
            });
            avertissements.extend(a);
            // Fiche 13.6 : indicateurs par occupant.
            let (occupants, a) = crate::indicateurs::occupants::calculer(
                bat,
                sref_m2,
                i.cep_kwhep_m2,
                i.ic_energie_kgco2_m2,
                bat.acv.as_ref().map(|d| crate::acv::nocc(&d.occupants)),
                |z| crate::deplacements::nb_max_occupants(z).ok(),
            );
            i.occupants = occupants;
            avertissements.extend(a);
            i
        });
        if !avec_generations {
            avertissements.push(format!(
                "bâtiment {} : sans génération, Cep, Cep,nr et Ic énergie ne sont pas calculés",
                bat.id
            ));
        }
        // Fiche 13.2 : sorties par zone et par groupe, si elles sont demandées.
        let sorties_detaillees = match (releve_zones, indicateurs.as_ref()) {
            (Some(rz), Some(i)) => Some(calculer_sorties(
                &EntreesSorties {
                    batiment: bat,
                    th_c: &th_c,
                    zones: &zones,
                    total_horaire: &releve.horaire,
                    prod_pv_wh: &prod_pv_wh,
                    indicateurs: i,
                    generations: &generations,
                    consommations_bat: &consommations_bat.annuel,
                },
                rz,
            )),
            _ => {
                if demandes.sorties_detaillees {
                    avertissements.push(format!(
                        "bâtiment {} : sans génération, les sorties détaillées par zone et par \
                         groupe ne sont pas calculées",
                        bat.id
                    ));
                }
                None
            }
        };
        let ic_energie_kgco2_m2 = indicateurs.as_ref().and_then(|i| i.ic_energie_kgco2_m2);
        let mut indicateurs = indicateurs;
        let acv = bat.acv.as_ref().map(|d| {
            let usages: Vec<(u8, f64)> = bat
                .zones
                .iter()
                .zip(&zones)
                .map(|(z, r)| (z.usage.numero(), r.sref_m2))
                .collect();
            crate::acv::calculer_acv(&crate::acv::EntreesAcv {
                donnees: d,
                sref_m2,
                zones: &usages,
                ic_energie_kgco2_m2,
                tac_pv_pct: indicateurs
                    .as_ref()
                    .and_then(|i| i.production_pv.as_ref())
                    .map(|p| p.tac_pct),
            })
        });
        let acv = acv.map(|mut a| {
            crate::acv::verifier_exigence(
                &mut a,
                bat,
                &zones,
                zone,
                loc.alt_m,
                indicateurs.as_mut().and_then(|i| i.exigences.as_mut()),
            );
            a
        });
        batiments.push(ResultatsBatiment {
            id: bat.id.clone(),
            sref_m2,
            bbio_pts: Some(bbio.annuel.bbio_pts),
            bbio: Some(bbio),
            cep_kwhep_m2: indicateurs.as_ref().map(|i| i.cep_kwhep_m2),
            cep_nr_kwhep_m2: indicateurs.as_ref().map(|i| i.cep_nr_kwhep_m2),
            dh_c_h: dh_le_plus_defavorable(zones.iter().filter_map(|z| z.dh_c_h)),
            ic_energie_kgco2_m2,
            c_aux_vent_kwh: (!c_aux_vent.is_empty()).then(|| c_aux_vent.iter().sum()),
            zones,
            distributions_intergroupes: resultats_intergroupes(&th_c),
            reseaux_cta: resultats_reseaux_cta(&th_c),
            distributions_ecs_intergroupes: resultats_ecs_intergroupes(&th_c),
            recuperateurs_eaux_grises: resultats_recuperateurs_collectifs(&th_c),
            generations,
            consommations: avec_generations.then_some(consommations_bat),
            production_pv: (!productions.is_empty())
                .then(|| ResultatsProductionPv::depuis(&productions)),
            indicateurs,
            acv,
            series_horaires,
            sorties_detaillees,
            consommations_collectif: conso_collectif,
            ascenseurs,
            escalators,
            parkings,
            espaces_tampons: th_b
                .espaces_tampons
                .iter()
                .zip(&th_d.espaces_tampons)
                .map(|(b, d)| ResultatsEspaceTampon {
                    id: b.id.clone(),
                    th_b: Some(synthese_espace_tampon(b)),
                    th_d: Some(synthese_espace_tampon(d)),
                    valeurs_par_defaut: b.valeurs_par_defaut.clone(),
                })
                .collect(),
        });
    }

    let mut resultats = Resultats {
        version_moteur: VERSION_MOTEUR.to_owned(),
        version_methode: VERSION_METHODE.to_owned(),
        reglementaire: meteo.est_reglementaire(),
        origine_meteo_th_bc: meteo.th_bc.origine.clone(),
        origine_meteo_th_d: meteo.th_d.origine.clone(),
        valeurs_arrondies: false,
        avertissements,
        climat,
        batiments,
    };
    if options.arrondir {
        resultats.arrondir();
    }
    Ok(resultats)
}

/// DH d'une zone ou d'un bâtiment : celui du groupe le plus défavorable.
///
/// Th-BCE 2020 §13.5.1 : l'indicateur « est exprimé par groupe » ; §13.3.1 : les seuils
/// s'apprécient sur un DH « calculé au niveau groupe ». Le texte ne définit pas de DH de zone
/// ou de bâtiment ; le plus élevé des groupes est restitué à ces niveaux, puisque c'est lui qui
/// décide du respect d'un seuil (`docs/questions/Q040.md`). `None` sans groupe.
fn dh_le_plus_defavorable(dh: impl Iterator<Item = f64>) -> Option<f64> {
    dh.reduce(f64::max)
}

/// Calcul annuel d'une zone dans un mode de calcul.
///
/// Th-BCE 2020 §5.5 (assemblage de la zone, figure 18). `climat` et `confort` sont ceux du jeu
/// météo du mode.
fn simuler_batiment(
    bat: &Batiment,
    projet: &Projet,
    mode: ModeCalcul,
    climat: &ClimatExterieur,
    confort: &ConfortAdaptatif,
) -> Result<BatimentHoraire, Erreur> {
    simuler_batiment_avec(bat, projet, mode, climat, confort, |_, _| {})
}

/// Comme [`simuler_batiment`], en appelant `apres_le_pas(h, batiment)` à la fin de chaque pas
/// de temps.
fn simuler_batiment_avec(
    bat: &Batiment,
    projet: &Projet,
    mode: ModeCalcul,
    climat: &ClimatExterieur,
    confort: &ConfortAdaptatif,
    apres_le_pas: impl FnMut(usize, &BatimentHoraire),
) -> Result<BatimentHoraire, Erreur> {
    // Humidité initiale de l'air des groupes : celle de l'air extérieur au premier pas de temps
    // (décision Q39 point 4).
    let omega_initiale = climat.we_kg_kg.first().copied().unwrap_or(0.0);
    let mut calcul = BatimentHoraire::nouveau(
        bat,
        &projet.catalogue,
        mode,
        projet.localisation.alt_m,
        omega_initiale,
    )?;
    calcul.simuler_avec(climat, confort, apres_le_pas)?;
    Ok(calcul)
}

/// Séries horaires d'un groupe sur l'année de calcul d'un mode.
#[derive(Debug, Clone, PartialEq)]
pub struct SerieHoraireGroupe {
    /// Identifiant du bâtiment.
    pub batiment: String,
    /// Identifiant de la zone.
    pub zone: String,
    /// Identifiant du groupe.
    pub groupe: String,
    /// « Te(h) » : température extérieure du site, en °C, pour chaque pas de temps.
    pub te_c: Vec<f64>,
    /// Sorties du groupe à chaque pas de temps : températures, besoins, débit de
    /// surventilation, saison, inerties et, en Th-D, confort d'été.
    pub pas: Vec<SortiesPasGroupe>,
}

/// Calcule un projet dans un seul mode (Th-B ou Th-D) et renvoie les séries horaires de chaque
/// groupe : 8 760 pas de temps, dans l'ordre des bâtiments, des zones et des groupes.
///
/// C'est l'accès de l'API Rust aux températures opératives horaires et, en Th-D, aux
/// degrés-heures de chaque pas de temps ([`SortiesPasGroupe::confort_ete`]). Les valeurs ne
/// sont pas arrondies.
pub fn series_horaires(
    projet: &Projet,
    meteo: &MeteoProjet,
    mode: ModeCalcul,
) -> Result<Vec<SerieHoraireGroupe>, Erreur> {
    verifier(projet, meteo)?;
    let resolu = projet_avec_classes_deduites(projet);
    let projet = resolu.as_ref().unwrap_or(projet);
    let loc = &projet.localisation;
    let annee = match mode {
        ModeCalcul::ThD => &meteo.th_d,
        ModeCalcul::ThB | ModeCalcul::ThC => &meteo.th_bc,
    };
    let climat = ClimatExterieur::nouveau(annee, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    let omega_initiale = climat.we_kg_kg.first().copied().unwrap_or(0.0);
    let mut series = Vec::new();
    for bat in &projet.batiments {
        let mut calcul =
            BatimentHoraire::nouveau(bat, &projet.catalogue, mode, loc.alt_m, omega_initiale)?;
        let premier = series.len();
        for z in &calcul.zones {
            series.extend(z.groupes.iter().map(|g| SerieHoraireGroupe {
                batiment: bat.id.clone(),
                zone: z.id.clone(),
                groupe: g.id.clone(),
                te_c: climat.te_c.iter().take(NB_HEURES_AN).copied().collect(),
                pas: Vec::with_capacity(NB_HEURES_AN),
            }));
        }
        calcul.simuler_avec(&climat, &confort, |_, b| {
            let sorties = b.zones.iter().flat_map(|z| z.sorties.iter());
            for (serie, s) in series[premier..].iter_mut().zip(sorties) {
                serie.pas.extend(s.iter().copied());
            }
        })?;
    }
    Ok(series)
}

/// Série horaire de production photovoltaïque d'un bâtiment (§12 ; entrée du chapitre 13).
#[derive(Debug, Clone, PartialEq)]
pub struct SeriePvBatiment {
    /// Identifiant du bâtiment.
    pub batiment: String,
    /// « Eelec_prod_PV_integre(h) » (éq. 2524) : somme des `Pond` des installations, en Wh par
    /// pas de temps, 8 760 valeurs.
    pub e_prod_pv_wh: Vec<f64>,
    /// `Pond` de chaque installation (identifiant, 8 760 valeurs en W).
    pub installations: Vec<(String, Vec<f64>)>,
}

/// Production photovoltaïque horaire de chaque bâtiment doté d'installations, sur le jeu météo
/// « Th-BC ». Les valeurs ne sont pas arrondies. La répartition entre part autoconsommée et
/// part exportée (§13.2.3, éq. 2527 à 2530) est à la charge du lot 7.
pub fn series_pv(projet: &Projet, meteo: &MeteoProjet) -> Result<Vec<SeriePvBatiment>, Erreur> {
    verifier(projet, meteo)?;
    let loc = &projet.localisation;
    let climat = ClimatExterieur::nouveau(&meteo.th_bc, loc.departement, loc.alt_m);
    Ok(projet
        .batiments
        .iter()
        .filter(|b| !b.installations_pv.is_empty())
        .map(|b| {
            let prods: Vec<_> = b
                .installations_pv
                .iter()
                .map(|i| produire(i, &projet.catalogue, &climat))
                .collect();
            let mut total = vec![0.0; NB_HEURES_AN];
            for p in &prods {
                for (t, x) in total.iter_mut().zip(&p.p_ond_w) {
                    *t += x;
                }
            }
            SeriePvBatiment {
                batiment: b.id.clone(),
                e_prod_pv_wh: total,
                installations: prods.into_iter().map(|p| (p.id, p.p_ond_w)).collect(),
            }
        })
        .collect())
}
