// SPDX-License-Identifier: GPL-3.0-or-later
//! Indicateurs par occupant (Th-BCE 2020 fiche 13.6, `C_CALC_Indicateurs pédagogiques en
//! fonction du nombre d'occupants`, p. 1404 à 1406 ; annexe II §5.2.3 et §5.3.1).
//!
//! Le nombre d'occupants d'une zone `Nocc_zn` est saisi (balise de zone) ou déduit : en
//! habitation, `Σ i·ni` d'après le nombre de pièces principales des logements (tableau 342 et
//! éq. 131 de l'annexe II) ; en tertiaire hors enseignement, nombre maximal d'occupants des
//! scénarios conventionnels que le texte permet d'employer ; en enseignement, nombre d'élèves
//! saisi. Lectures : `docs/questions/Q117.md`.

use super::resultats::{IndicateursOccupants, OccupantsZone};
use crate::entree::{Batiment, Zone};
use crate::resultats::ValeurParDefaut;

/// Nombre d'occupants d'une zone et origine, avant les scénarios.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoccZone {
    /// Déterminé par la saisie : valeur et origine.
    Connu(f64, &'static str),
    /// Tertiaire hors enseignement sans saisie : les scénarios peuvent le donner.
    DeLaScenarios,
    /// Non déterminé : saisie nécessaire.
    Inconnu,
}

/// Th-BCE 2020 §13.6.3.1, tableau 342 : 1 à 7 pièces principales ↔ autant d'occupants.
fn occupants_du_logement(pieces_principales: u8) -> f64 {
    f64::from(pieces_principales)
}

/// `Nocc_zn` d'après la saisie de la zone.
pub fn nocc_saisi(z: &Zone) -> NoccZone {
    if !z.types_logements.is_empty() {
        // Éq. (131) : Nocc = Σ i·ni.
        let n: f64 = z
            .types_logements
            .iter()
            .map(|t| occupants_du_logement(t.pieces_principales) * f64::from(t.nombre))
            .sum();
        return NoccZone::Connu(n, "logements");
    }
    if let Some(n) = z.nb_occupants {
        return NoccZone::Connu(n, "saisie");
    }
    if z.usage.est_habitation() || z.usage.i_enseignement() {
        NoccZone::Inconnu
    } else {
        NoccZone::DeLaScenarios
    }
}

/// Th-BCE 2020 §13.6.3.2, éq. (2552) et (2553) ; annexe II éq. (130). `nb_scenarios` donne le
/// nombre maximal d'occupants conventionnel d'une zone. Renvoie les indicateurs, ou `None` et
/// un avertissement si le nombre d'occupants d'une zone n'est pas déterminé.
pub fn calculer(
    bat: &Batiment,
    sref_m2: f64,
    cep_kwhep_m2: f64,
    ic_energie_kgco2_m2: Option<f64>,
    nocc_acv: Option<f64>,
    nb_scenarios: impl Fn(&Zone) -> Option<f64>,
) -> (Option<IndicateursOccupants>, Vec<String>) {
    let mut zones = Vec::new();
    let mut defauts = Vec::new();
    let mut manquantes = Vec::new();
    for (i, z) in bat.zones.iter().enumerate() {
        let (n, origine) = match nocc_saisi(z) {
            NoccZone::Connu(n, o) => (n, o),
            NoccZone::DeLaScenarios => match nb_scenarios(z) {
                Some(n) => {
                    defauts.push(ValeurParDefaut {
                        champ: format!("zones[{i}].nb_occupants"),
                        valeur: n,
                        regle: "Th-BCE 2020 §13.6.3.1 : valeur d'occupation des scénarios \
                                conventionnels (nombre maximal d'occupants de la zone)"
                            .to_owned(),
                    });
                    (n, "scenarios")
                }
                None => {
                    manquantes.push(z.id.clone());
                    continue;
                }
            },
            NoccZone::Inconnu => {
                manquantes.push(z.id.clone());
                continue;
            }
        };
        zones.push(OccupantsZone {
            zone: z.id.clone(),
            nocc: n,
            origine: origine.to_owned(),
        });
    }
    // Nocc du bâtiment : somme de celui des zones (éq. 2552) ; à défaut, celui de l'ACV
    // (annexe II §5.2.3, saisi pour le bâtiment : `acv.occupants`), qui est la même donnée.
    let (nocc, zones, origine) = if manquantes.is_empty() {
        let n: f64 = zones.iter().map(|z| z.nocc).sum();
        (n, zones, "zones")
    } else if let Some(n) = nocc_acv {
        (n, Vec::new(), "acv")
    } else {
        return (
            None,
            vec![format!(
                "bâtiment {} : indicateurs par occupant non calculés, nombre d'occupants non \
                 déterminé pour la zone {} (`types_logements` en habitation, `nb_occupants` en \
                 enseignement, fiche 13.6.3.1)",
                bat.id,
                manquantes.join(", ")
            )],
        );
    };
    // Éq. (2553) : 0 si Nocc = 0.
    let par_occ = |x: f64| if nocc > 0.0 { x * sref_m2 / nocc } else { 0.0 };
    (
        Some(IndicateursOccupants {
            nocc_batiment: nocc,
            origine: origine.to_owned(),
            zones,
            cep_par_occupant_kwhep: par_occ(cep_kwhep_m2),
            ic_energie_par_occupant_kgco2: ic_energie_kgco2_m2.map(par_occ),
            valeurs_par_defaut: defauts,
        }),
        Vec::new(),
    )
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::entree::{TypeLogement, UsageZone};

    fn zone(usage: UsageZone) -> Zone {
        serde_json::from_value(serde_json::json!({
            "id": "z", "usage": serde_json::to_value(usage).unwrap(), "groupes": []
        }))
        .unwrap()
    }

    #[test]
    fn pieces_principales_eq_131() {
        // 3 logements T2 et 2 logements T4 : 2×3 + 4×2 = 14 occupants.
        let mut z = zone(UsageZone::LogementCollectif);
        z.types_logements = vec![
            TypeLogement {
                pieces_principales: 2,
                nombre: 3,
            },
            TypeLogement {
                pieces_principales: 4,
                nombre: 2,
            },
        ];
        assert_eq!(nocc_saisi(&z), NoccZone::Connu(14.0, "logements"));
    }

    #[test]
    fn habitation_et_enseignement_sans_saisie_inconnus() {
        assert_eq!(
            nocc_saisi(&zone(UsageZone::MaisonIndividuelle)),
            NoccZone::Inconnu
        );
        assert_eq!(
            nocc_saisi(&zone(UsageZone::EnseignementPrimaire)),
            NoccZone::Inconnu
        );
        assert_eq!(
            nocc_saisi(&zone(UsageZone::Bureaux)),
            NoccZone::DeLaScenarios
        );
    }

    #[test]
    fn indicateur_par_occupant_eq_2553() {
        // Maison de 100 m², Cep = 80 kWhep/m², Ic = 200 kg/m², T4 : 4 occupants.
        let mut z = zone(UsageZone::MaisonIndividuelle);
        z.types_logements = vec![TypeLogement {
            pieces_principales: 4,
            nombre: 1,
        }];
        let bat: Batiment = serde_json::from_value(serde_json::json!({
            "id": "b", "zones": [serde_json::to_value(&z).unwrap()]
        }))
        .unwrap();
        let (r, a) = calculer(&bat, 100.0, 80.0, Some(200.0), None, |_| None);
        let r = r.unwrap();
        assert!(a.is_empty());
        assert_eq!(r.nocc_batiment, 4.0);
        // 80 × 100 / 4 = 2000 kWhep par occupant ; 200 × 100 / 4 = 5000 kg.
        assert_eq!(r.cep_par_occupant_kwhep, 2000.0);
        assert_eq!(r.ic_energie_par_occupant_kgco2, Some(5000.0));
    }

    #[test]
    fn nocc_de_l_acv_quand_une_zone_manque() {
        let bat: Batiment = serde_json::from_value(serde_json::json!({
            "id": "b", "zones": [serde_json::to_value(zone(UsageZone::MaisonIndividuelle)).unwrap()]
        }))
        .unwrap();
        // 90 kWhep/m² × 120 m² / 5 occupants = 2160.
        let (r, a) = calculer(&bat, 120.0, 90.0, None, Some(5.0), |_| None);
        let r = r.unwrap();
        assert!(a.is_empty());
        assert_eq!(r.origine, "acv");
        assert_eq!(r.cep_par_occupant_kwhep, 2160.0);
    }

    #[test]
    fn nocc_nul_donne_zero() {
        let mut z = zone(UsageZone::Bureaux);
        z.nb_occupants = Some(0.0);
        let bat: Batiment = serde_json::from_value(serde_json::json!({
            "id": "b", "zones": [serde_json::to_value(&z).unwrap()]
        }))
        .unwrap();
        let (r, _) = calculer(&bat, 100.0, 80.0, None, None, |_| None);
        assert_eq!(r.unwrap().cep_par_occupant_kwhep, 0.0);
    }

    #[test]
    fn scenarios_signales_par_defaut() {
        let bat: Batiment = serde_json::from_value(serde_json::json!({
            "id": "b", "zones": [serde_json::to_value(zone(UsageZone::Bureaux)).unwrap()]
        }))
        .unwrap();
        let (r, _) = calculer(&bat, 1000.0, 60.0, None, None, |_| Some(40.0));
        let r = r.unwrap();
        assert_eq!(r.valeurs_par_defaut.len(), 1);
        assert_eq!(r.zones[0].origine, "scenarios");
        // 60 × 1000 / 40 = 1500.
        assert_eq!(r.cep_par_occupant_kwhep, 1500.0);
    }
}
