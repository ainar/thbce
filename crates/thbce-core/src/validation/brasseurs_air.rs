// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des brasseurs d'air d'un groupe (Th-BCE 2020 §8.32.3.1 et §8.32.3.2).

use crate::chauffage_refroidissement::brasseurs_air::rat_us_defaut;
use crate::entree::{BrasseurAir, GestionBrasseur, Groupe, UsageBrasseur, UsageZone};

use super::RapportValidation;

/// Contrôle les brasseurs d'air saisis dans un groupe : groupe non climatisé (§8.32.3.1),
/// plages, un seul « usage » lorsque l'usage « jour et nuit » est employé (§8.32.3.2.2) et
/// surface couverte au plus égale à la surface occupée de l'usage (§8.32.3.2.3).
pub(super) fn valider_brasseurs_groupe(
    r: &mut RapportValidation,
    c: &str,
    g: &Groupe,
    usage: UsageZone,
) {
    if g.brasseurs_air.is_empty() {
        return;
    }
    let cb = format!("{c}.brasseurs_air");
    if g.iclim == Some(true) {
        r.erreur(
            format!("{c}.iclim"),
            Some("true".into()),
            "les brasseurs d'air ne s'introduisent que dans des groupes non climatisés \
             (Th-BCE 2020 §8.32.3.1)",
        );
    }
    r.ids_uniques(&cb, g.brasseurs_air.iter().map(|b| &b.id));
    let a_gr = g.su_m2.unwrap_or(0.0);
    for (i, b) in g.brasseurs_air.iter().enumerate() {
        valider_type(r, &format!("{cb}[{i}]"), b);
    }
    // §8.32.3.2.2 : un type « jour et nuit » exclut les types des autres usages.
    let a_mixte = g
        .brasseurs_air
        .iter()
        .any(|b| b.usage == UsageBrasseur::JourEtNuit);
    if a_mixte
        && g.brasseurs_air
            .iter()
            .any(|b| b.usage != UsageBrasseur::JourEtNuit)
    {
        r.erreur(
            cb.clone(),
            None,
            "un type de brasseurs d'usage « jour et nuit » exclut les types d'un autre usage \
             dans le groupe (Th-BCE 2020 §8.32.3.2.2)",
        );
    }
    // §8.32.3.2.3 : surface couverte (ramenée à 15 m² par brasseur) et surface occupée.
    for u in [
        UsageBrasseur::Nuit,
        UsageBrasseur::Jour,
        UsageBrasseur::JourEtNuit,
    ] {
        let mut somme = 0.0;
        let mut rat_us_min = f64::INFINITY;
        for b in g.brasseurs_air.iter().filter(|b| b.usage == u) {
            let n = f64::from(b.nombre);
            somme += if a_gr > 0.0 {
                (15.0 * n / a_gr).min(b.ratio_surface_brassee)
            } else {
                b.ratio_surface_brassee
            };
            rat_us_min = rat_us_min.min(
                b.ratio_surface_occupee
                    .unwrap_or_else(|| rat_us_defaut(usage, u)),
            );
        }
        if rat_us_min.is_finite() && somme > rat_us_min + 1e-12 {
            r.erreur(
                cb.clone(),
                Some(somme.to_string()),
                format!(
                    "la surface couverte par les brasseurs d'un même usage ne peut dépasser la \
                     surface occupée par cet usage ({rat_us_min}, Th-BCE 2020 §8.32.3.2.3)"
                ),
            );
        }
    }
    // §8.32.1 : modèle valable pour des hauteurs sous plafond inférieures à 3 m.
    if let (Some(v), true) = (g.v_m3, a_gr > 0.0) {
        if v / a_gr >= 3.0 {
            r.avertissement(
                format!("{c}.v_m3"),
                Some((v / a_gr).to_string()),
                "le modèle des brasseurs d'air n'est valable que pour une hauteur sous plafond \
                 inférieure à 3 m (Th-BCE 2020 §8.32.1) ; hauteur moyenne estimée par V/Agr",
            );
        }
    }
}

fn valider_type(r: &mut RapportValidation, c: &str, b: &BrasseurAir) {
    if b.nombre == 0 {
        r.erreur(
            format!("{c}.nombre"),
            Some("0".into()),
            "au moins un brasseur par type (Th-BCE 2020 §8.32.2, Nbr_p)",
        );
    }
    r.plage(
        format!("{c}.ratio_surface_brassee"),
        b.ratio_surface_brassee,
        0.0,
        1.0,
    );
    if let Some(x) = b.ratio_surface_occupee {
        r.plage(format!("{c}.ratio_surface_occupee"), x, 0.0, 1.0);
    }
    r.strictement_positif(format!("{c}.p_elec_max_w"), b.p_elec_max_w);
    r.strictement_positif(format!("{c}.q_v_max_m3_h"), b.q_v_max_m3_h);
    match &b.gestion {
        GestionBrasseur::Manuelle => {}
        GestionBrasseur::AutomatiqueThermostat(p) => {
            for (n, x) in [
                ("delta_theta_op_1_k", p.delta_theta_op_1_k),
                ("delta_theta_op_2_k", p.delta_theta_op_2_k),
                ("delta_theta_op_3_k", p.delta_theta_op_3_k),
            ] {
                r.positif(format!("{c}.gestion.automatique_thermostat.{n}"), x);
            }
        }
        GestionBrasseur::Automatique(p) => {
            r.positif(
                format!("{c}.gestion.automatique.theta_op_dec_c"),
                p.theta_op_dec_c,
            );
            for (n, x) in [
                ("delta_theta_op_1_k", Some(p.delta_theta_op_1_k)),
                ("delta_theta_op_2_k", Some(p.delta_theta_op_2_k)),
                ("delta_theta_op_3_k", p.delta_theta_op_3_k),
            ] {
                if let Some(x) = x {
                    r.positif(format!("{c}.gestion.automatique.{n}"), x);
                }
            }
        }
    }
}
