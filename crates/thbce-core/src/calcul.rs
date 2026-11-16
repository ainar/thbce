// SPDX-License-Identifier: GPL-3.0-or-later
//! Point d'entrée du calcul.
//!
//! État : le calcul valide l'entrée et les données météo, détermine le climat du site (lot 3) et
//! les surfaces de référence, et renvoie la structure des résultats. Les indicateurs
//! réglementaires restent à `None` jusqu'à l'implémentation des lots 4 à 7
//! (`docs/couverture.md`).

use crate::bati::baie::{BaieResolue, EnsembleBaies, Protections};
use crate::bati::paroi_opaque::{EnsembleParois, ParoiResolue};
use crate::bati::pont_thermique::{EnsemblePonts, PontResolu};
use crate::climat::{ClimatExterieur, ConfortAdaptatif, MeteoProjet};
use crate::entree::{Catalogue, Groupe, Projet, UsageZone};
use crate::erreur::Erreur;
use crate::resultats::{
    PeriodeConfortAdaptatif, Resultats, ResultatsBaie, ResultatsBatiment, ResultatsClimat,
    ResultatsEnveloppe, ResultatsGroupe, ResultatsZone,
};
use crate::scenarios::ScenariosZone;
use crate::validation::valider;
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
    avertissements.push(
        "squelette du moteur : Bbio, Cep, Cep,nr, DH et Ic énergie ne sont pas encore calculés"
            .to_owned(),
    );

    let batiments = projet
        .batiments
        .iter()
        .map(|bat| {
            let zones: Vec<ResultatsZone> = bat
                .zones
                .iter()
                .map(|zone| {
                    let groupes: Vec<ResultatsGroupe> = zone
                        .groupes
                        .iter()
                        .map(|g| ResultatsGroupe {
                            id: g.id.clone(),
                            sref_m2: sref_groupe(g, zone.usage),
                            enveloppe: enveloppe_groupe(g, &projet.catalogue),
                            baies: resultats_baies(g, &projet.catalogue),
                        })
                        .collect();
                    // Th-BCE 2020 §13.1.3, éq. (2377) : SREF_zn = somme des SREF_gr de la zone.
                    let sref_m2 = groupes.iter().map(|g| g.sref_m2).sum();
                    // Th-BCE 2020 §4.1 : scénarios de la zone, préparés avant la boucle horaire.
                    let a_gr: Vec<f64> = zone
                        .groupes
                        .iter()
                        .map(|g| g.su_m2.unwrap_or(0.0))
                        .collect();
                    let scenarios =
                        ScenariosZone::nouveau(zone.usage, &a_gr, zone.nb_lgt.unwrap_or(1)).ok();
                    ResultatsZone {
                        id: zone.id.clone(),
                        sref_m2,
                        nadeq: scenarios.map(|s| s.nadeq),
                        groupes,
                    }
                })
                .collect();
            // Th-BCE 2020 §13.1.3, éq. (2378) : SREF_bat = somme des SREF_zn du bâtiment.
            let sref_m2 = zones.iter().map(|z| z.sref_m2).sum();
            ResultatsBatiment {
                id: bat.id.clone(),
                sref_m2,
                bbio_pts: None,
                cep_kwhep_m2: None,
                cep_nr_kwhep_m2: None,
                dh_c_h: None,
                ic_energie_kgco2_m2: None,
                zones,
            }
        })
        .collect();

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

/// Surface de référence d'un groupe.
///
/// Th-BCE 2020 §13.1.3, éq. (2375) et (2376) : `SREF_gr = SHAB_gr` si l'usage vaut 1 ou 2,
/// `SREF_gr = SU_gr` sinon. La validation garantit la présence de la surface requise.
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
fn enveloppe_groupe(groupe: &Groupe, catalogue: &Catalogue) -> ResultatsEnveloppe {
    let parois: Vec<ParoiResolue> = groupe
        .parois_opaques
        .iter()
        .filter_map(|p| {
            let type_paroi = catalogue.parois.iter().find(|t| t.id == p.type_paroi)?;
            Some(ParoiResolue::nouvelle(p, type_paroi, catalogue))
        })
        .collect();
    let mut ponts = Vec::new();
    for pont in &groupe.ponts_thermiques {
        PontResolu::resoudre(pont, catalogue, &mut ponts);
    }
    let baies: Vec<BaieResolue> = groupe
        .baies
        .iter()
        .filter_map(|b| {
            let type_baie = catalogue.baies.iter().find(|t| t.id == b.type_baie)?;
            BaieResolue::nouvelle(b, type_baie, catalogue)
        })
        .collect();
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

fn sref_groupe(groupe: &Groupe, usage: UsageZone) -> f64 {
    let surface = if usage.est_habitation() {
        groupe.shab_m2
    } else {
        groupe.su_m2
    };
    surface.unwrap_or(0.0)
}
