// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation du puits climatique (Th-BCE 2020 §6.8, §6.10 et §6.11) : cohérence de la
//! déclaration, systèmes qui peuvent le recevoir, plages du tableau 69.

use super::{nom_json, RapportValidation};
use crate::entree::{
    Puits, PuitsClimatique, PuitsHydraulique, SystemeVentilation, Ventilation, Zone,
};
use crate::ventilation::ventilation_projet::VentilationProjet;

/// Contrôle `puits`, `puits_climatique` et `puits_hydraulique` d'un système de ventilation de
/// chemin `c`.
pub(super) fn valider_puits(r: &mut RapportValidation, c: &str, v: &Ventilation, zone: &Zone) {
    if v.id_puits_climatique.is_some() {
        // Plusieurs centrales sur un même puits (`Id_Puits_Climatique`, §6.8.2, éq. 722, 753 et
        // 754 sommées sur les systèmes du puits) : docs/questions/Q232.md.
        r.hors_perimetre(
            format!("{c}.id_puits_climatique"),
            v.id_puits_climatique.clone(),
            "puits climatique partagé entre plusieurs centrales (Id_Puits_Climatique) ; un puits \
             ne dessert que la centrale dont il est la description",
        );
    }
    match (v.puits, &v.puits_climatique, &v.puits_hydraulique) {
        (None, None, None) => return,
        (Some(Puits::Hydraulique), pc, ph) => {
            // Éq. (386) : une centrale n'est jamais reliée aux deux puits.
            if pc.is_some() {
                r.erreur(
                    format!("{c}.puits_climatique"),
                    None,
                    "une centrale ne peut pas être reliée à la fois à un puits climatique et à un \
                     puits hydraulique (Th-BCE 2020 §6.3.3.1, éq. 386)",
                );
                return;
            }
            if ph.is_none() {
                r.erreur(
                    format!("{c}.puits_hydraulique"),
                    None,
                    "décrire le puits hydraulique (Th-BCE 2020 §6.9.2, tableau 71)",
                );
                return;
            }
        }
        (Some(Puits::Climatique), None, _) => {
            r.erreur(
                format!("{c}.puits_climatique"),
                None,
                "décrire le puits climatique (Th-BCE 2020 §6.8.2, tableau 69)",
            );
            return;
        }
        (Some(Puits::Climatique), Some(_), Some(_)) => {
            r.erreur(
                format!("{c}.puits_hydraulique"),
                None,
                "une centrale ne peut pas être reliée à la fois à un puits climatique et à un \
                 puits hydraulique (Th-BCE 2020 §6.3.3.1, éq. 386)",
            );
            return;
        }
        (Some(Puits::Climatique), Some(_), None) => {}
        (None, Some(_), _) => {
            r.erreur(
                format!("{c}.puits"),
                None,
                "un puits climatique décrit se déclare par `puits = climatique`",
            );
            return;
        }
        (None, None, Some(_)) => {
            r.erreur(
                format!("{c}.puits"),
                None,
                "un puits hydraulique décrit se déclare par `puits = hydraulique`",
            );
            return;
        }
    }
    // §6.8.1, §6.9.1 et §6.11.3 : l'air neuf d'un simple flux par insufflation, d'un double
    // flux ou d'une centrale de traitement d'air ; les autres systèmes n'ont pas d'air neuf
    // soufflé.
    if !matches!(
        v.systeme,
        SystemeVentilation::SimpleFluxInsufflation(_)
            | SystemeVentilation::DoubleFlux(_)
            | SystemeVentilation::Cta(_)
    ) {
        let (nature, fiche) = match v.puits {
            Some(Puits::Hydraulique) => (Puits::Hydraulique, "puits hydraulique, §6.9.1"),
            _ => (Puits::Climatique, "puits climatique, §6.8.1 et §6.11.3"),
        };
        r.erreur(
            format!("{c}.puits"),
            Some(nom_json(&nature)),
            format!(
                "le {fiche} : il ne se raccorde qu'à un simple flux par insufflation, un double \
                 flux ou une centrale de traitement d'air (Th-BCE 2020)"
            ),
        );
    }
    if let Some(p) = &v.puits_climatique {
        valider_description(r, &format!("{c}.puits_climatique"), p);
    }
    if let Some(p) = &v.puits_hydraulique {
        valider_description_hydraulique(r, &format!("{c}.puits_hydraulique"), p, v);
        // §6.3.3.3.4 : efficacité de la batterie > 1 (message d'erreur du texte), contrôlée sur le
        // système résolu quand le reste de l'entrée est valide.
        if r.est_valide() {
            if let Some(e) = VentilationProjet::nouvelle(zone, 18.0).and_then(|x| x.erreur_puits_h)
            {
                r.erreur(format!("{c}.puits_hydraulique.batterie"), None, e);
            }
        }
    }
}

/// Plages du tableau 71 et cohérence de la surventilation synchronisée.
fn valider_description_hydraulique(
    r: &mut RapportValidation,
    c: &str,
    p: &PuitsHydraulique,
    v: &Ventilation,
) {
    r.strictement_positif(format!("{c}.q_max_m3_h"), p.q_max_m3_h);
    r.positif(format!("{c}.p_max_w"), p.p_max_w);
    r.strictement_positif(format!("{c}.l_m"), p.l_m);
    r.strictement_positif(format!("{c}.lambda_tube_w_m_k"), p.lambda_tube_w_m_k);
    r.positif(format!("{c}.ep_m"), p.ep_m);
    r.strictement_positif(format!("{c}.di_m"), p.di_m);
    r.positif(format!("{c}.profondeur_m"), p.profondeur_m);
    r.positif(
        format!("{c}.theta_ext_arret_hiver_c"),
        p.theta_ext_arret_hiver_c,
    );
    r.positif(
        format!("{c}.theta_ext_arret_ete_c"),
        p.theta_ext_arret_ete_c,
    );
    if let Some(b) = &p.batterie {
        r.strictement_positif(format!("{c}.batterie.p_ch_kw"), b.p_ch_kw);
        r.strictement_positif(format!("{c}.batterie.p_fr_kw"), b.p_fr_kw);
    }
    if p.surventilation_synchronisee && v.systeme.surventilation().is_none() {
        r.erreur(
            format!("{c}.surventilation_synchronisee"),
            Some("true".to_owned()),
            "la surventilation d'été synchronisée avec le puits suppose une centrale pourvue de \
             `rafraichissement_surventilation` (Th-BCE 2020 §6.3.3.3.4, éq. 403)",
        );
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
