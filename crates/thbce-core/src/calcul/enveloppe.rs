// SPDX-License-Identifier: GPL-3.0-or-later
//! Restitution des résultats de l'enveloppe : synthèse par groupe, baies, valeurs par défaut
//! et espaces tampons.

use crate::bati::assemblage_groupe::EnveloppeResolue;
use crate::bati::baie::{BaieResolue, EnsembleBaies, Protections};
use crate::bati::espace_tampon::assemblage::EspaceTamponHoraire;
use crate::bati::paroi_opaque::EnsembleParois;
use crate::bati::pont_thermique::EnsemblePonts;
use crate::bati::surventilation::H_TTF_CAS_GENERAL_M;
use crate::entree::{Catalogue, Groupe, UsageZone};
use crate::resultats::{
    ResultatsBaie, ResultatsEnveloppe, ResultatsEspaceTamponMode, ValeurParDefaut,
};

/// Synthèse annuelle d'un espace tampon.
pub(super) fn synthese_espace_tampon(et: &EspaceTamponHoraire) -> ResultatsEspaceTamponMode {
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

/// Valeurs par défaut appliquées au groupe. Th-BCE 2020 §5.13.3.2.2 : sans saisie, la hauteur
/// de tirage thermique vaut 1,5 m ; elle n'intervient que si le groupe a des baies ouvrables.
pub(super) fn defauts_groupe(groupe: &Groupe, catalogue: &Catalogue) -> Vec<ValeurParDefaut> {
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
pub(super) fn resultats_baies(groupe: &Groupe, catalogue: &Catalogue) -> Vec<ResultatsBaie> {
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
pub(super) fn enveloppe_groupe(
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
