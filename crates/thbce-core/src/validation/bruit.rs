// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation de la détermination détaillée du classement au bruit des baies (fiche
//! d'application 5.11 ; annexe de l'article R. 172-4, chapitre V, partie I ; Q200 à Q204).

use super::RapportValidation;
use crate::bati::classement_bruit::{classe_baie, classe_baie_detaillee};
use crate::entree::Batiment;

/// Contrôle le champ `bruit` de chaque baie : distances, présence d'au moins une donnée,
/// façade déterminable, cohérence avec `expo_br` si les deux sont saisis.
pub(super) fn valider_bruit(r: &mut RapportValidation, cb: &str, bat: &Batiment) {
    for (iz, z) in bat.zones.iter().enumerate() {
        for (ig, g) in z.groupes.iter().enumerate() {
            for (ib, b) in g.baies.iter().enumerate() {
                let Some(bruit) = &b.bruit else { continue };
                let c = format!("{cb}.zones[{iz}].groupes[{ig}].baies[{ib}].bruit");
                for (i, inf) in bruit.infrastructures.iter().enumerate() {
                    // Une distance est une longueur finie, positive ou nulle.
                    r.positif(
                        format!("{c}.infrastructures[{i}].distance_m"),
                        inf.distance_m,
                    );
                }
                let detail = classe_baie_detaillee(bruit);
                if detail.is_none() && bruit.facade.is_none() {
                    r.erreur(
                        c.clone(),
                        None,
                        "renseigner au moins une infrastructure, la zone du plan d'exposition au \
                         bruit ou la façade de la baie",
                    );
                } else if detail.is_none() && classe_baie(bat, b).is_none() {
                    r.erreur(
                        format!("{c}.facade"),
                        bruit.facade.clone(),
                        "aucune baie de cette façade n'a de classe d'exposition au bruit \
                         (détermination détaillée ou expo_br) : la classe de la baie ne peut pas \
                         s'en déduire",
                    );
                }
                if let (Some(d), Some(s)) = (detail, b.expo_br) {
                    if d != s {
                        r.erreur(
                            format!("{cb}.zones[{iz}].groupes[{ig}].baies[{ib}].expo_br"),
                            Some(format!("{s:?}")),
                            format!(
                                "classe d'exposition au bruit saisie incohérente avec sa \
                                 détermination détaillée ({d:?}, tableaux de la partie I du \
                                 chapitre V)"
                            ),
                        );
                    }
                }
            }
        }
    }
}
