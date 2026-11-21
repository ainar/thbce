// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des DRV et des thermo-frigo-pompes (Th-BCE 2020 §8.16).

use super::thermodynamique::{
    aval_fictif_chauffage, aval_fictif_refroidissement, valider_thermodynamique,
};
use crate::entree::{Batiment, ThermodynamiqueElectriqueSaisi};
use crate::validation::RapportValidation;

/// DRV (type 509, `drv` vrai) ou thermo-frigo-pompe (type 508) : générateur thermodynamique
/// réversible (§8.16.3), machine air / air recyclé pour un DRV (« génération air / air »),
/// eau / eau pour une thermo-frigo-pompe (§8.16.1), sans ECS.
pub(super) fn valider_transfert_locaux(
    r: &mut RapportValidation,
    c: &str,
    t: &ThermodynamiqueElectriqueSaisi,
    bat: &Batiment,
    drv: bool,
) {
    valider_thermodynamique(r, c, t, bat);
    let quoi = if drv {
        "un générateur DRV"
    } else {
        "une thermo-frigo-pompe"
    };
    if t.chauffage.is_none() || t.refroidissement.is_none() {
        r.erreur(
            c.to_owned(),
            None,
            format!(
                "{quoi} est réversible : modes `chauffage` et `refroidissement` requis (Th-BCE \
                 2020 §8.16.3)"
            ),
        );
    }
    if t.ecs.is_some() {
        r.erreur(
            format!("{c}.ecs"),
            None,
            format!("{quoi} ne produit pas d'ECS (Th-BCE 2020 §8.16.3)"),
        );
    }
    let attendu = if drv {
        "air recyclé (« génération air / air »)"
    } else {
        "eau (« machine eau / eau »)"
    };
    let air = [
        t.chauffage
            .as_ref()
            .map(|m| ("chauffage", aval_fictif_chauffage(m.technologie))),
        t.refroidissement.as_ref().map(|m| {
            (
                "refroidissement",
                aval_fictif_refroidissement(m.technologie),
            )
        }),
    ];
    for (mode, sur_air) in air.into_iter().flatten() {
        if sur_air != drv {
            r.erreur(
                format!("{c}.{mode}.technologie"),
                None,
                format!(
                    "technologie incompatible : {quoi} a pour fluide aval l'{attendu} (Th-BCE \
                     2020 §8.16.1)"
                ),
            );
        }
    }
}
