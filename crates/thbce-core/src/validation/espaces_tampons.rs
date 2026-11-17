// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des espaces tampons.

use std::collections::BTreeSet;

use super::enveloppe::valider_exposition;
use super::{RapportValidation, References};
use crate::entree::{Batiment, EspaceTamponSolarise, NatureEspaceTampon};

/// Espaces tampons d'un bâtiment (Th-BCE 2020 §5.2 à 5.4) : plages des tableaux 14, 15 et 19,
/// références des composants qui donnent sur eux.
pub(super) fn valider_espaces_tampons(
    r: &mut RapportValidation,
    cb: &str,
    bat: &Batiment,
    refs: &References<'_>,
) {
    let c_et = format!("{cb}.espaces_tampons");
    let ids = r.ids_uniques(&c_et, bat.espaces_tampons.iter().map(|e| &e.id));
    let solarises: BTreeSet<&str> = bat
        .espaces_tampons
        .iter()
        .filter(|e| matches!(e.nature, NatureEspaceTampon::Solarise(_)))
        .map(|e| e.id.as_str())
        .collect();
    let mut associes: BTreeSet<&str> = BTreeSet::new();
    // Références depuis les composants des groupes : (chemin, identifiant, baie, enveloppe).
    let mut references: Vec<(String, &String, bool, bool)> = Vec::new();
    for (iz, zone) in bat.zones.iter().enumerate() {
        for (ig, g) in zone.groupes.iter().enumerate() {
            let cg = format!("{cb}.zones[{iz}].groupes[{ig}]");
            for (i, p) in g.parois_opaques.iter().enumerate() {
                if let Some(id) = &p.espace_tampon {
                    references.push((
                        format!("{cg}.parois_opaques[{i}].espace_tampon"),
                        id,
                        false,
                        true,
                    ));
                }
            }
            for (i, p) in g.ponts_thermiques.iter().enumerate() {
                if let Some(id) = &p.espace_tampon {
                    references.push((
                        format!("{cg}.ponts_thermiques[{i}].espace_tampon"),
                        id,
                        false,
                        true,
                    ));
                }
            }
            for (i, b) in g.baies.iter().enumerate() {
                if let Some(id) = &b.espace_tampon {
                    references.push((format!("{cg}.baies[{i}].espace_tampon"), id, true, true));
                }
            }
            // Les entrées d'air ne suffisent pas à associer l'espace tampon à l'enveloppe.
            if let Some(id) = g
                .entrees_air
                .as_ref()
                .and_then(|e| e.espace_tampon.as_ref())
            {
                references.push((format!("{cg}.entrees_air.espace_tampon"), id, false, false));
            }
        }
    }
    for (chemin, id, baie, enveloppe) in references {
        if !ids.contains(id.as_str()) {
            r.erreur(
                chemin,
                Some(id.clone()),
                "espace tampon inconnu dans ce bâtiment",
            );
            continue;
        }
        if enveloppe {
            associes.insert(id.as_str());
        }
        // Th-BCE 2020 §5.10.3 : « les baies vitrées ne peuvent donner que sur des espaces
        // tampons vitrés ».
        if baie && !solarises.contains(id.as_str()) {
            r.erreur(
                chemin,
                Some(id.clone()),
                "une baie ne peut donner que sur un espace tampon solarisé (Th-BCE 2020 §5.10.3, \
                 « Présence d'un espace tampon »)",
            );
        }
    }
    for (i, et) in bat.espaces_tampons.iter().enumerate() {
        let c = format!("{c_et}[{i}]");
        // §5.4.1 : un espace tampon « doit nécessairement être associé à au moins un composant
        // d'enveloppe (paroi opaque, baie) d'un groupe ».
        if !et.id.trim().is_empty() && !associes.contains(et.id.as_str()) {
            r.erreur(
                format!("{c}.id"),
                Some(et.id.clone()),
                "aucune paroi opaque, baie ni aucun pont thermique ne donne sur cet espace tampon \
                 (Th-BCE 2020 §5.4.1)",
            );
        }
        match &et.nature {
            NatureEspaceTampon::NonSolarise(n) => {
                // Tableau 19 : de −5 à 1.
                r.plage(
                    format!("{c}.nature.non_solarise.b_et_ns"),
                    n.b_et_ns,
                    -5.0,
                    1.0,
                );
            }
            NatureEspaceTampon::Solarise(s) => {
                valider_espace_solarise(r, &format!("{c}.nature.solarise"), s, refs);
            }
        }
    }
}

fn valider_espace_solarise(
    r: &mut RapportValidation,
    c: &str,
    s: &EspaceTamponSolarise,
    refs: &References<'_>,
) {
    r.strictement_positif(format!("{c}.a_et_m2"), s.a_et_m2);
    r.positif(format!("{c}.u_et_pb_w_m2k"), s.u_et_pb_w_m2k);
    r.positif(format!("{c}.h_ttf_et_m"), s.h_ttf_et_m);
    if let Some(v) = s.cm_et_surf_kj_m2k {
        r.positif(format!("{c}.cm_et_surf_kj_m2k"), v);
    }
    if let Some(v) = s.qv_et_base_m3_h {
        r.positif(format!("{c}.qv_et_base_m3_h"), v);
    }
    if s.parois.is_empty() {
        r.erreur(
            format!("{c}.parois"),
            None,
            "un espace tampon solarisé a au moins une paroi donnant sur l'extérieur",
        );
    }
    r.ids_uniques(&format!("{c}.parois"), s.parois.iter().map(|p| &p.id));
    let protections = s.protections_mobiles.is_some();
    let mut h_ext = s.u_et_pb_w_m2k * s.a_et_m2;
    for (i, p) in s.parois.iter().enumerate() {
        let cp = format!("{c}.parois[{i}]");
        r.strictement_positif(format!("{cp}.a_ue_m2"), p.a_ue_m2);
        r.positif(format!("{cp}.u_ue_w_m2k"), p.u_ue_w_m2k);
        h_ext += p.a_ue_m2 * p.u_ue_w_m2k;
        valider_exposition(r, &format!("{cp}.exposition"), &p.exposition, refs);
        r.plage(format!("{cp}.swe_sp"), p.swe_sp, 0.0, 1.0);
        r.plage(format!("{cp}.tle_sp"), p.tle_sp, 0.0, 1.0);
        r.plage(format!("{cp}.r_ouv_max"), p.r_ouv_max, 0.0, 1.0);
        for (nom, v) in [("swe_ap", p.swe_ap), ("tle_ap", p.tle_ap)] {
            match (v, protections) {
                (Some(v), _) => r.plage(format!("{cp}.{nom}"), v, 0.0, 1.0),
                (None, true) => r.erreur(
                    format!("{cp}.{nom}"),
                    None,
                    "requis quand l'espace tampon a des protections mobiles (Th-BCE 2020 \
                     §5.4.3.3.1.3, éq. 118 et 128)",
                ),
                (None, false) => {}
            }
        }
    }
    // Éq. (124) et (125) : Het doit être strictement positif ; les échanges avec le volume
    // chauffé y contribuent toujours, mais l'espace tampon doit aussi échanger avec l'extérieur.
    let qv_base = s.qv_et_base_m3_h.unwrap_or(
        crate::bati::espace_tampon::espace_tampon::QV_ET_BASE_DEFAUT_M3_H_M2 * s.a_et_m2,
    );
    if h_ext <= 0.0 && qv_base <= 0.0 && r.est_valide() {
        r.erreur(
            format!("{c}.parois"),
            None,
            "l'espace tampon n'échange ni chaleur ni air avec l'extérieur (Th-BCE 2020 §5.4.3.3.1.5, \
             éq. 124)",
        );
    }
}
