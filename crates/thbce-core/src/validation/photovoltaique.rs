// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des installations photovoltaïques (Th-BCE 2020 §12.1 à 12.4).

use crate::entree::Batiment;
use crate::pv::entree::{PerformancesPv, RendementOnduleur};
use crate::pv::onduleur::POINTS_OBLIGES;

use super::{valider_exposition, RapportValidation, References};

pub(super) fn valider_installations_pv(
    r: &mut RapportValidation,
    cb: &str,
    bat: &Batiment,
    refs: &References<'_>,
) {
    r.ids_uniques(
        &format!("{cb}.installations_pv"),
        bat.installations_pv.iter().map(|i| &i.id),
    );
    for (k, i) in bat.installations_pv.iter().enumerate() {
        let c = format!("{cb}.installations_pv[{k}]");
        // §12.2.2 : α de 0 à 360°, β de 0 à 90°.
        valider_exposition(r, &format!("{c}.exposition"), &i.exposition, refs);
        r.plage(
            format!("{c}.exposition.inclinaison_deg"),
            i.exposition.inclinaison_deg,
            0.0,
            90.0,
        );
        if i.n_modules < 1 {
            r.erreur(
                format!("{c}.n_modules"),
                Some(i.n_modules.to_string()),
                "au moins un module (Th-BCE 2020 §12.3.2)",
            );
        }
        r.strictement_positif(format!("{c}.modules.s_m2"), i.modules.s_m2);
        let pc = i.modules.performances.pc_saisie_w();
        r.strictement_positif(format!("{c}.modules.performances.pc_w"), pc);
        match i.modules.performances {
            PerformancesPv::Certifie {
                mu_par_c, noct_c, ..
            }
            | PerformancesPv::Justifie {
                mu_par_c, noct_c, ..
            }
            | PerformancesPv::Declare {
                mu_par_c, noct_c, ..
            } => {
                // Tableau 327 : μ de 0 à 0,006 °C⁻¹, NOCT de 40 à 90 °C.
                r.plage(
                    format!("{c}.modules.performances.mu_par_c"),
                    mu_par_c,
                    0.0,
                    0.006,
                );
                r.plage(
                    format!("{c}.modules.performances.noct_c"),
                    noct_c,
                    40.0,
                    90.0,
                );
            }
            PerformancesPv::Defaut { .. } => {}
        }
        if let Some(p) = i.onduleur.p_ac_nom_w {
            r.strictement_positif(format!("{c}.onduleur.p_ac_nom_w"), p);
        }
        match &i.onduleur.rendement {
            RendementOnduleur::Defaut => {}
            RendementOnduleur::Europeen { eta_eu } => {
                r.plage(format!("{c}.onduleur.rendement.eta_eu"), *eta_eu, 0.0, 1.0);
            }
            RendementOnduleur::Courbe { points } => {
                for (j, p) in points.iter().enumerate() {
                    r.plage(
                        format!("{c}.onduleur.rendement.points[{j}].eta"),
                        p.eta,
                        0.0,
                        1.0,
                    );
                    r.positif(format!("{c}.onduleur.rendement.points[{j}].x"), p.x);
                }
                // §12.4.3, cas n° 3 : points de passage obligés (CEI 61683).
                for x in POINTS_OBLIGES {
                    if !points.iter().any(|p| (p.x - x).abs() < 1e-9) {
                        r.erreur(
                            format!("{c}.onduleur.rendement.points"),
                            None,
                            format!(
                                "point de passage obligé absent : X = {x} (Th-BCE 2020 §12.4.3, cas n° 3)"
                            ),
                        );
                    }
                }
            }
        }
    }
}
