// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation du puits climatique (Th-BCE 2020 §6.8, §6.10 et §6.11) : cohérence de la
//! déclaration, systèmes qui peuvent le recevoir, plages du tableau 69.

use super::{nom_json, RapportValidation};
use crate::entree::{Puits, PuitsClimatique, SystemeVentilation, Ventilation};

/// Contrôle `puits` et `puits_climatique` d'un système de ventilation de chemin `c`.
pub(super) fn valider_puits(r: &mut RapportValidation, c: &str, v: &Ventilation) {
    match (v.puits, &v.puits_climatique) {
        (None, None) => return,
        (Some(Puits::Hydraulique), _) => {
            // Fiche 6.9 : pas encore prise en charge (éq. 386 : jamais avec un puits climatique).
            r.hors_perimetre(
                format!("{c}.puits"),
                Some(nom_json(&Puits::Hydraulique)),
                "puits hydraulique",
            );
            if v.puits_climatique.is_some() {
                r.erreur(
                    format!("{c}.puits_climatique"),
                    None,
                    "une centrale ne peut pas être reliée à la fois à un puits climatique et à un \
                     puits hydraulique (Th-BCE 2020 §6.3.3.1, éq. 386)",
                );
            }
            return;
        }
        (Some(Puits::Climatique), None) => {
            r.erreur(
                format!("{c}.puits_climatique"),
                None,
                "décrire le puits climatique (Th-BCE 2020 §6.8.2, tableau 69)",
            );
            return;
        }
        (None, Some(_)) => {
            r.erreur(
                format!("{c}.puits"),
                None,
                "un puits climatique décrit se déclare par `puits = climatique`",
            );
            return;
        }
        (Some(Puits::Climatique), Some(_)) => {}
    }
    // §6.8.1 et §6.11.3 : l'air neuf d'un simple flux par insufflation, d'un double flux ou
    // d'une centrale de traitement d'air ; les autres systèmes n'ont pas d'air neuf soufflé.
    if !matches!(
        v.systeme,
        SystemeVentilation::SimpleFluxInsufflation(_)
            | SystemeVentilation::DoubleFlux(_)
            | SystemeVentilation::Cta(_)
    ) {
        r.erreur(
            format!("{c}.puits"),
            Some(nom_json(&Puits::Climatique)),
            "le puits climatique ne se raccorde qu'à un simple flux par insufflation, un double \
             flux ou une centrale de traitement d'air (Th-BCE 2020 §6.8.1 et §6.11.3)",
        );
    }
    if let Some(p) = &v.puits_climatique {
        valider_description(r, &format!("{c}.puits_climatique"), p);
    }
}

/// Plages du tableau 69 et des consignes du tableau 73.
fn valider_description(r: &mut RapportValidation, c: &str, p: &PuitsClimatique) {
    if p.nd < 1 {
        r.erreur(
            format!("{c}.nd"),
            Some(p.nd.to_string()),
            "doit être supérieur ou égal à 1",
        );
    }
    r.strictement_positif(format!("{c}.l_m"), p.l_m);
    r.strictement_positif(format!("{c}.di_m"), p.di_m);
    r.positif(format!("{c}.ep_m"), p.ep_m);
    r.strictement_positif(format!("{c}.lambda_tube_w_m_k"), p.lambda_tube_w_m_k);
    r.positif(format!("{c}.profondeur_m"), p.profondeur_m);
    if let Some(b) = &p.bypass {
        for (champ, x) in [
            ("theta_int_hiver_c", b.theta_int_hiver_c),
            ("theta_ext_hiver_c", b.theta_ext_hiver_c),
            ("theta_int_ete_c", b.theta_int_ete_c),
            ("theta_ext_ete_c", b.theta_ext_ete_c),
        ] {
            r.plage(format!("{c}.bypass.{champ}"), x, -273.0, f64::INFINITY);
        }
    }
}
