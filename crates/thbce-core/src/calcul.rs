// SPDX-License-Identifier: GPL-3.0-or-later
//! Point d'entrée du calcul.
//!
//! État : le calcul valide l'entrée et les données météo, détermine le climat du site (lot 3) et
//! les surfaces de référence, puis déroule l'année au pas horaire en mode Th-B (lot 4) : besoins
//! de chauffage, de refroidissement et d'éclairage et Bbio (§13.1). Cep, Cep,nr, DH et
//! Ic énergie restent à `None` jusqu'à l'implémentation des lots 5 à 7 (`docs/couverture.md`).

use crate::bati::assemblage_groupe::{EnveloppeResolue, GroupeBbio};
use crate::bati::assemblage_zone::ZoneBbio;
use crate::bati::baie::{BaieResolue, EnsembleBaies, Protections};
use crate::bati::paroi_opaque::EnsembleParois;
use crate::bati::pont_thermique::EnsemblePonts;
use crate::bati::surventilation::H_TTF_CAS_GENERAL_M;
use crate::climat::{ClimatExterieur, ConfortAdaptatif, MeteoProjet};
use crate::entree::{Catalogue, Groupe, Projet, UsageZone, Zone};
use crate::erreur::Erreur;
use crate::indicateurs::sorties_th_b::{sorties, EnergiesMensuelles};
use crate::resultats::{
    PeriodeConfortAdaptatif, Resultats, ResultatsBaie, ResultatsBatiment, ResultatsClimat,
    ResultatsConfort, ResultatsDebitsBbio, ResultatsEnveloppe, ResultatsGroupe, ResultatsSaisons,
    ResultatsZone, ValeurParDefaut,
};
use crate::validation::valider;
use crate::ventilation::bouche_conduit::debits_specifiques_bbio_groupe;
use crate::ventilation::ven_bbio::{debits_systeme_conventionnel, LectureDebitsBbio};
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

/// Calcule les indicateurs d'un projet à partir de sa description et des données météo de sa
/// zone climatique.
pub fn calculer(
    projet: &Projet,
    meteo: &MeteoProjet,
    options: &Options,
) -> Result<Resultats, Erreur> {
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
    let loc = &projet.localisation;
    let zone = loc.zone_climatique_retenue();
    meteo.verifier(zone)?;

    // Th-BCE 2020 §3.1 : climat du site pour chacun des deux jeux météo, calculé une fois avant
    // la boucle horaire.
    let climat_th_bc = ClimatExterieur::nouveau(&meteo.th_bc, loc.departement, loc.alt_m);
    let climat_th_d = ClimatExterieur::nouveau(&meteo.th_d, loc.departement, loc.alt_m);
    let periode = |climat: &ClimatExterieur| {
        let c = ConfortAdaptatif::nouveau(&climat.te_c);
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
        confort_adaptatif_th_bc: periode(&climat_th_bc),
        confort_adaptatif_th_d: periode(&climat_th_d),
    };

    let mut avertissements: Vec<String> = rapport
        .avertissements
        .iter()
        .map(|p| format!("{} : {}", p.chemin, p.message))
        .collect();
    avertissements
        .push("Cep, Cep,nr, DH et Ic énergie ne sont pas encore calculés par le moteur".to_owned());

    // Th-BCE 2020 §4.5.3.2.1, éq. (57) : moyenne glissante de la température extérieure du jeu
    // météo « Th-BC », employée par les indicateurs de confort du calcul Th-B.
    let confort_th_bc = ConfortAdaptatif::nouveau(&climat_th_bc.te_c);
    let mut batiments = Vec::with_capacity(projet.batiments.len());
    for bat in &projet.batiments {
        let mut zones = Vec::with_capacity(bat.zones.len());
        let mut energies_bat = EnergiesMensuelles::default();
        for zone in &bat.zones {
            let (resultats_zone, energies_zone) =
                calculer_zone(zone, projet, &climat_th_bc, &confort_th_bc)?;
            energies_bat.ajouter(&energies_zone);
            zones.push(resultats_zone);
        }
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
            dh_c_h: None,
            ic_energie_kgco2_m2: None,
            zones,
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

/// Calcul annuel d'une zone en mode Th-B et restitution de ses résultats.
///
/// Th-BCE 2020 §5.5 (assemblage de la zone, figure 18) et §13.1 (sorties du mode Th-B). Renvoie
/// aussi les énergies mensuelles de la zone, que le bâtiment somme (§13.1.3).
fn calculer_zone(
    zone: &Zone,
    projet: &Projet,
    climat: &ClimatExterieur,
    confort: &ConfortAdaptatif,
) -> Result<(ResultatsZone, EnergiesMensuelles), Erreur> {
    // Humidité initiale de l'air des groupes : celle de l'air extérieur au premier pas de temps
    // (décision Q39 point 4).
    let omega_initiale = climat.we_kg_kg.first().copied().unwrap_or(0.0);
    let mut calcul = ZoneBbio::nouvelle(
        zone,
        &projet.catalogue,
        projet.localisation.alt_m,
        omega_initiale,
    )?;
    calcul.simuler(climat, confort)?;

    let mut energies_zone = EnergiesMensuelles::default();
    let groupes: Vec<ResultatsGroupe> = zone
        .groupes
        .iter()
        .zip(&calcul.groupes)
        .map(|(g, c)| {
            let sref_m2 = sref_groupe(g, zone.usage);
            let energies = energies_groupe(c);
            energies_zone.ajouter(&energies);
            let s = &c.etat.saisons;
            let k = &c.cumuls.confort;
            ResultatsGroupe {
                id: g.id.clone(),
                sref_m2,
                enveloppe: enveloppe_groupe(g, &projet.catalogue, zone.usage),
                debits_bbio: debits_bbio_groupe(g, zone.usage),
                bbio: Some(sorties(&energies, sref_m2)),
                saisons_bbio: Some(ResultatsSaisons {
                    j_fin_ch: s.j_fin_ch,
                    j_debut_ch: s.j_debut_ch,
                    j_debut_fr: s.j_debut_fr,
                    j_fin_fr: s.j_fin_fr,
                }),
                confort_bbio: Some(ResultatsConfort {
                    nbh_inc_cons_ch: k.nbh_inc_cons_ch,
                    dh_inc_cons_ch_k_h: k.dh_inc_cons_ch_k_h,
                    nbh_inc_cons_fr: k.nbh_inc_cons_fr,
                    dh_inc_cons_fr_k_h: k.dh_inc_cons_fr_k_h,
                    nbh_inc_c_fr: k.nbh_inc_c_fr,
                    dh_inc_c_fr_k_h: k.dh_inc_c_fr_k_h,
                }),
                baies: resultats_baies(g, &projet.catalogue),
                valeurs_par_defaut: defauts_groupe(g, &projet.catalogue),
            }
        })
        .collect();
    // Th-BCE 2020 §13.1.3, éq. (2377) : SREF_zn = somme des SREF_gr de la zone.
    let sref_m2: f64 = groupes.iter().map(|g| g.sref_m2).sum();
    Ok((
        ResultatsZone {
            id: zone.id.clone(),
            sref_m2,
            nadeq: Some(calcul.scenarios.nadeq),
            bbio: Some(sorties(&energies_zone, sref_m2)),
            groupes,
        },
        energies_zone,
    ))
}

/// Énergies mensuelles d'un groupe à l'issue du calcul annuel.
fn energies_groupe(groupe: &GroupeBbio) -> EnergiesMensuelles {
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
