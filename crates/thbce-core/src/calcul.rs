// SPDX-License-Identifier: GPL-3.0-or-later
//! Point d'entrée du calcul.
//!
//! État : le calcul valide l'entrée et les données météo, détermine le climat du site (lot 3) et
//! les surfaces de référence, puis déroule deux années au pas horaire : en mode Th-B (lot 4),
//! pour les besoins de chauffage, de refroidissement et d'éclairage et le Bbio (§13.1) ; puis en
//! mode Th-D (lot 5), sur le jeu météo à séquence caniculaire, pour l'indicateur de confort
//! d'été DH (§13.5). Les deux calculs emploient le même enchaînement horaire, paramétré par
//! [`ModeCalcul`] ; le calcul Th-D emploie la ventilation du projet (§6.2 à 6.5). La
//! consommation des ventilateurs est restituée pour le Cep ; Cep, Cep,nr et Ic énergie restent
//! à `None` jusqu'à l'implémentation des lots 6 et 7 (`docs/couverture.md`).

use crate::bati::assemblage_batiment::BatimentHoraire;
use crate::bati::assemblage_groupe::THETA_INITIALE_C;
use crate::bati::assemblage_groupe::{EnveloppeResolue, GroupeHoraire, SortiesPasGroupe};
use crate::bati::assemblage_zone::ZoneHoraire;
use crate::bati::baie::{BaieResolue, EnsembleBaies, Protections};
use crate::bati::espace_tampon::assemblage::EspaceTamponHoraire;
use crate::bati::paroi_opaque::EnsembleParois;
use crate::bati::pont_thermique::EnsemblePonts;
use crate::bati::surventilation::H_TTF_CAS_GENERAL_M;
use crate::climat::calendrier::date_scenario;
use crate::climat::{ClimatExterieur, ConfortAdaptatif, MeteoProjet};
use crate::conventions::{ModeCalcul, NB_HEURES_AN};
use crate::entree::{Batiment, Catalogue, Groupe, Projet, UsageZone, Zone};
use crate::erreur::Erreur;
use crate::indicateurs::sorties_th_b::{sorties, EnergiesMensuelles};
use crate::resultats::{
    PeriodeConfortAdaptatif, Resultats, ResultatsBaie, ResultatsBatiment, ResultatsClimat,
    ResultatsConfort, ResultatsConfortEte, ResultatsDebitsBbio, ResultatsEnveloppe,
    ResultatsEspaceTampon, ResultatsEspaceTamponMode, ResultatsGroupe, ResultatsSaisons,
    ResultatsVentilationGroupe, ResultatsVentilationMode, ResultatsVentilationZone, ResultatsZone,
    ValeurParDefaut,
};
use crate::validation::valider;
use crate::ventilation::bouche_conduit::debits_specifiques_bbio_groupe;
use crate::ventilation::ven_bbio::{debits_systeme_conventionnel, LectureDebitsBbio};
use crate::ventilation::ventilation_projet::VentilationProjet;
use crate::{VERSION_METHODE, VERSION_MOTEUR};

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

/// Avertissement porté par tout résultat tant que les émetteurs du projet ne sont pas décrits
/// (lot 6b) : le calcul Th-D emploie la ventilation du projet (lot 6a) mais l'émission
/// conventionnelle du calcul du Bbio (`docs/questions.md`, Q40 et Q42).
pub const AVERTISSEMENT_DH: &str =
    "DH calculé avec la ventilation du projet et l'émission conventionnelle du calcul du Bbio : \
     les émetteurs du projet ne sont pas encore pris en compte par le moteur";

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
/// Deux calculs annuels s'enchaînent pour chaque zone : Th-B sur le jeu météo « Th-BC », puis
/// Th-D sur le jeu « Th-D » (annexe II §1.2.1, tableau 2).
pub fn calculer(
    projet: &Projet,
    meteo: &MeteoProjet,
    options: &Options,
) -> Result<Resultats, Erreur> {
    let rapport = verifier(projet, meteo)?;
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
    avertissements
        .push("Cep, Cep,nr et Ic énergie ne sont pas encore calculés par le moteur".to_owned());
    avertissements.push(AVERTISSEMENT_DH.to_owned());

    let mut batiments = Vec::with_capacity(projet.batiments.len());
    for bat in &projet.batiments {
        let mut zones = Vec::with_capacity(bat.zones.len());
        let mut energies_bat = EnergiesMensuelles::default();
        // Mode Th-B, puis mode Th-D : même enchaînement horaire, climat et règles du mode. Le
        // bâtiment est simulé d'un bloc : ses espaces tampons sont calculés avant ses zones à
        // chaque pas de temps (§5.2).
        let th_b = simuler_batiment(bat, projet, ModeCalcul::ThB, &climat_th_bc, &confort_th_bc)?;
        let th_d = simuler_batiment(bat, projet, ModeCalcul::ThD, &climat_th_d, &confort_th_d)?;
        for (zone, (zb, zd)) in bat.zones.iter().zip(th_b.zones.iter().zip(&th_d.zones)) {
            let (resultats_zone, energies_zone) = restituer_zone(zone, projet, zb, zd);
            energies_bat.ajouter(&energies_zone);
            zones.push(resultats_zone);
        }
        let c_aux_vent: Vec<f64> = zones
            .iter()
            .filter_map(|z| z.ventilation.as_ref().map(|v| v.c_aux_vent_kwh))
            .collect();
        // Th-BCE 2020 §13.1.3, éq. (2378) : SREF_bat = somme des SREF_zn du bâtiment.
        let sref_m2: f64 = zones.iter().map(|z| z.sref_m2).sum();
        let bbio = sorties(&energies_bat, sref_m2);
        batiments.push(ResultatsBatiment {
            id: bat.id.clone(),
            sref_m2,
            bbio_pts: Some(bbio.annuel.bbio_pts),
            bbio: Some(bbio),
            cep_kwhep_m2: None,
            cep_nr_kwhep_m2: None,
            dh_c_h: dh_le_plus_defavorable(zones.iter().filter_map(|z| z.dh_c_h)),
            ic_energie_kgco2_m2: None,
            c_aux_vent_kwh: (!c_aux_vent.is_empty()).then(|| c_aux_vent.iter().sum()),
            zones,
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
/// décide du respect d'un seuil (`docs/questions.md`, Q40). `None` sans groupe.
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
    calcul.simuler(climat, confort)?;
    Ok(calcul)
}

/// Synthèse annuelle d'un espace tampon.
fn synthese_espace_tampon(et: &EspaceTamponHoraire) -> ResultatsEspaceTamponMode {
    let k = &et.cumuls;
    let n = f64::from(k.nbh.max(1));
    ResultatsEspaceTamponMode {
        b_therm_moyen: k.somme_b_therm / n,
        theta_et_moy_c: k.somme_theta_et_c / n,
        theta_et_min_c: k.theta_et_min_c,
        theta_et_max_c: k.theta_et_max_c,
        nbh_surventilation: k.nbh_surventilation,
    }
}

/// Saisons propres d'un groupe à l'issue d'un calcul annuel (§4.6.3.6, éq. 92).
fn resultats_saisons(groupe: &GroupeHoraire) -> ResultatsSaisons {
    let s = &groupe.etat.saisons;
    ResultatsSaisons {
        j_fin_ch: s.j_fin_ch,
        j_debut_ch: s.j_debut_ch,
        j_debut_fr: s.j_debut_fr,
        j_fin_fr: s.j_fin_fr,
    }
}

/// Confort d'été d'un groupe à l'issue du calcul Th-D (§13.5.3.7 et 13.5.3.8).
fn resultats_confort_ete(groupe: &GroupeHoraire) -> ResultatsConfortEte {
    let k = &groupe.cumuls.confort_ete;
    ResultatsConfortEte {
        dh_c_h: k.dh_c_h,
        nbh_inconf: k.nbh_inconf,
        nbh_inconf_1c: k.nbh_inconf_1c,
        nbh_inconf_2c: k.nbh_inconf_2c,
        nbh_occ_conf_adapt: k.nbh_occ_conf_adapt,
        theta_op_max_c: k.theta_op_max_c,
        theta_op_max_occ_c: k
            .theta_op_max_occ_c
            .is_finite()
            .then_some(k.theta_op_max_occ_c),
    }
}

/// Restitution des résultats d'une zone à partir de ses deux calculs annuels.
///
/// Th-BCE 2020 §13.1 (sorties du mode Th-B) et §13.5 (confort d'été). Renvoie aussi les
/// énergies mensuelles de la zone en mode Th-B, que le bâtiment somme (§13.1.3).
fn restituer_zone(
    zone: &Zone,
    projet: &Projet,
    th_b: &ZoneHoraire,
    th_d: &ZoneHoraire,
) -> (ResultatsZone, EnergiesMensuelles) {
    let mut energies_zone = EnergiesMensuelles::default();
    // Système de ventilation du projet, préparé hors de la boucle horaire : en habitation, la
    // puissance des ventilateurs est constante (éq. 392 et 394, 641 et 643), et la
    // consommation se somme heure par heure (éq. 565, 645).
    let ventilation = VentilationProjet::nouvelle(zone, THETA_INITIALE_C);
    let groupes: Vec<ResultatsGroupe> = zone
        .groupes
        .iter()
        .zip(th_b.groupes.iter().zip(&th_d.groupes))
        .enumerate()
        .map(|(ig, (g, (b, d)))| {
            let sref_m2 = sref_groupe(g, zone.usage);
            let energies = energies_groupe(b);
            energies_zone.ajouter(&energies);
            let k = &b.cumuls.confort;
            ResultatsGroupe {
                id: g.id.clone(),
                sref_m2,
                enveloppe: enveloppe_groupe(g, &projet.catalogue, zone.usage),
                debits_bbio: debits_bbio_groupe(g, zone.usage),
                bbio: Some(sorties(&energies, sref_m2)),
                saisons_bbio: Some(resultats_saisons(b)),
                confort_bbio: Some(ResultatsConfort {
                    nbh_inc_cons_ch: k.nbh_inc_cons_ch,
                    dh_inc_cons_ch_k_h: k.dh_inc_cons_ch_k_h,
                    nbh_inc_cons_fr: k.nbh_inc_cons_fr,
                    dh_inc_cons_fr_k_h: k.dh_inc_cons_fr_k_h,
                    nbh_inc_c_fr: k.nbh_inc_c_fr,
                    dh_inc_c_fr_k_h: k.dh_inc_c_fr_k_h,
                }),
                confort_ete: Some(resultats_confort_ete(d)),
                saisons_th_d: Some(resultats_saisons(d)),
                ventilation: ventilation
                    .as_ref()
                    .map(|v| ventilation_groupe(v, ig, sref_m2)),
                baies: resultats_baies(g, &projet.catalogue),
                valeurs_par_defaut: defauts_groupe(g, &projet.catalogue),
            }
        })
        .collect();
    // Th-BCE 2020 §13.1.3, éq. (2377) : SREF_zn = somme des SREF_gr de la zone.
    let sref_m2: f64 = groupes.iter().map(|g| g.sref_m2).sum();
    let dh_c_h = dh_le_plus_defavorable(
        groupes
            .iter()
            .filter_map(|g| g.confort_ete.as_ref().map(|c| c.dh_c_h)),
    );
    (
        ResultatsZone {
            id: zone.id.clone(),
            sref_m2,
            nadeq: Some(th_b.scenarios.nadeq),
            bbio: Some(sorties(&energies_zone, sref_m2)),
            dh_c_h,
            ventilation: ventilation.as_ref().map(|v| ResultatsVentilationZone {
                id: v.id.clone(),
                p_vent_w: v.p_vent_w(),
                c_aux_vent_kwh: v.p_vent_w() * NB_HEURES_AN as f64 / 1000.0,
                th_d: th_d.ventilation.as_ref().map(|_| ResultatsVentilationMode {
                    nbh_bypass: th_d.cumuls_ventilation.nbh_bypass,
                    q_ag_kwh: th_d.cumuls_ventilation.w_ag_wh / 1000.0,
                }),
                valeurs_par_defaut: v.valeurs_par_defaut.clone(),
            }),
            groupes,
        },
        energies_zone,
    )
}

/// Ventilation du projet vue d'un groupe : débits et consommation des ventilateurs.
///
/// Th-BCE 2020 §6.2.3.4 (éq. 428 à 431) ; §6.3.3.4.13 (éq. 565 et 570) et §6.5.3.3 (éq. 645 et
/// 646) : la consommation horaire, en Wh, est la puissance attribuée au groupe, constante en
/// habitation ; elle est sommée sur les mois des scénarios (§4.1.3).
fn ventilation_groupe(v: &VentilationProjet, g: usize, sref_m2: f64) -> ResultatsVentilationGroupe {
    let p_vent_w = v.p_vent_groupe_w(g);
    let mut mensuel = vec![0.0; 12];
    for h in 0..NB_HEURES_AN {
        let m = usize::from(date_scenario(h).mois) - 1;
        if let Some(x) = mensuel.get_mut(m) {
            *x += p_vent_w / 1000.0;
        }
    }
    let c_aux_vent_kwh: f64 = mensuel.iter().sum();
    let d = v.debits_groupe(g);
    let cond = v.debits_cond_groupe(g);
    ResultatsVentilationGroupe {
        q_spec_repris_m3_h: d.q_rep_m3_h,
        q_spec_souffle_m3_h: d.q_souffle_m3_h,
        q_repris_cond_m3_h: cond.q_rep_m3_h,
        q_souffle_cond_m3_h: cond.q_souffle_m3_h,
        p_vent_w,
        c_aux_vent_kwh,
        c_aux_vent_kwh_m2: if sref_m2 > 0.0 {
            c_aux_vent_kwh / sref_m2
        } else {
            0.0
        },
        c_aux_vent_mensuel_kwh: mensuel,
    }
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

/// Énergies mensuelles d'un groupe à l'issue du calcul annuel.
fn energies_groupe(groupe: &GroupeHoraire) -> EnergiesMensuelles {
    EnergiesMensuelles {
        q_sys_ch_wh: groupe.cumuls.q_sys_ch_wh,
        q_sys_fr_wh: groupe.cumuls.q_sys_fr_wh,
        c_ecl_kwh: groupe.cumuls.c_ecl_kwh,
    }
}

/// Débits du système de ventilation conventionnel du Bbio pour un groupe d'habitation.
///
/// Th-BCE 2020 §6.2.3.1.3 et §6.2.3.2.1 (débits des bouches en Bbio), §6.1.3 (système
/// conventionnel à débit soufflé et extrait constant). Les autres usages (débits conventionnels
/// du tableau 56-1) ne sont pas pris en charge.
fn debits_bbio_groupe(groupe: &Groupe, usage: UsageZone) -> Option<ResultatsDebitsBbio> {
    if !usage.est_habitation() || groupe.bouches_conduits.is_empty() {
        return None;
    }
    let bouches = debits_specifiques_bbio_groupe(&groupe.bouches_conduits)?;
    let d = debits_systeme_conventionnel(bouches, LectureDebitsBbio::default());
    Some(ResultatsDebitsBbio {
        q_spec_repris_m3_h: d.q_rep_m3_h,
        q_spec_souffle_m3_h: d.q_souffle_m3_h,
    })
}

/// Valeurs par défaut appliquées au groupe. Th-BCE 2020 §5.13.3.2.2 : sans saisie, la hauteur
/// de tirage thermique vaut 1,5 m ; elle n'intervient que si le groupe a des baies ouvrables.
fn defauts_groupe(groupe: &Groupe, catalogue: &Catalogue) -> Vec<ValeurParDefaut> {
    let a_des_ouvrants = groupe.baies.iter().any(|b| {
        catalogue
            .baies
            .iter()
            .any(|t| t.id == b.type_baie && t.ouvrant.is_some())
    });
    if a_des_ouvrants && groupe.h_ttf_m.is_none() {
        vec![ValeurParDefaut {
            champ: "h_ttf_m".to_owned(),
            valeur: H_TTF_CAS_GENERAL_M,
            regle: "Th-BCE 2020 §5.13.3.2.2 : dans le cas général, Httf est pris égal à 1,5 m"
                .to_owned(),
        }]
    } else {
        Vec::new()
    }
}

/// Valeurs par défaut appliquées à chaque baie d'un groupe (Th-BCE 2020 §5.10.3.6.3, éq. 215,
/// 216 et 218).
fn resultats_baies(groupe: &Groupe, catalogue: &Catalogue) -> Vec<ResultatsBaie> {
    groupe
        .baies
        .iter()
        .map(|b| ResultatsBaie {
            id: b.id.clone(),
            valeurs_par_defaut: catalogue
                .baies
                .iter()
                .find(|t| t.id == b.type_baie)
                .and_then(|t| BaieResolue::nouvelle(b, t, catalogue))
                .map(|r| r.valeurs_par_defaut)
                .unwrap_or_default(),
        })
        .collect()
}

/// Grandeurs d'enveloppe d'un groupe (Th-BCE 2020 §5.14.3, éq. 279, 281 et 282 ; §5.16.3,
/// éq. 283, 286 et 287 ; §5.20.3, éq. 307 et 310). Les références au catalogue ont été vérifiées par la validation.
fn enveloppe_groupe(
    groupe: &Groupe,
    catalogue: &Catalogue,
    usage: UsageZone,
) -> ResultatsEnveloppe {
    let EnveloppeResolue {
        parois,
        ponts,
        baies,
        ..
    } = EnveloppeResolue::de(groupe, catalogue, usage);
    let ba = EnsembleBaies::de(&baies);
    let op = EnsembleParois::de(&parois);
    let pt = EnsemblePonts::de(&ponts);
    ResultatsEnveloppe {
        a_op_m2: op.a_op_m2,
        a_opv_m2: op.a_opv_m2,
        a_ophh_m2: op.a_ophh_m2,
        a_ophb_m2: op.a_ophb_m2,
        h_th_op_w_k: op.h_th_w_k,
        a_baies_m2: ba.a_baies_m2,
        // Th-BCE 2020 §5.6.3.1.5 et §5.7.2 : façades et toits au sens de la perméabilité.
        a_f_perm_m2: op.a_opv_m2 + ba.a_baies_v_m2,
        a_toit_perm_m2: op.a_ophh_m2 + ba.a_baies_h_m2,
        a_baies_v_m2: ba.a_baies_v_m2,
        a_baies_h_m2: ba.a_baies_h_m2,
        h_ges_sp_w_k: baies
            .iter()
            .map(|b| b.h_ges_w_k(&Protections::OUVERTES))
            .sum(),
        l_pt_m: pt.l_m,
        h_th_pt_w_k: pt.h_th_w_k,
    }
}

/// Surface de référence d'un groupe.
///
/// Th-BCE 2020 §13.1.3, éq. (2375) et (2376) : `SREF_gr = SHAB_gr` si l'usage vaut 1 ou 2,
/// `SREF_gr = SU_gr` sinon. La validation garantit la présence de la surface requise.
fn sref_groupe(groupe: &Groupe, usage: UsageZone) -> f64 {
    let surface = if usage.est_habitation() {
        groupe.shab_m2
    } else {
        groupe.su_m2
    };
    surface.unwrap_or(0.0)
}
