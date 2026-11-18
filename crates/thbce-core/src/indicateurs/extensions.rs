// SPDX-License-Identifier: GPL-3.0-or-later
//! Régime d'exigences des extensions et des constructions de petite surface : fiche 4.4 de
//! l'annexe III (« FA_Extension_de_bâtiment ») ; arrêté du 4 août 2021, article 3, II (permis
//! de 2022), articles 50-2 et 50-3 (exigences alternatives de l'article R. 172-3 du CCH ;
//! rédaction de l'arrêté du 18 mars 2026 pour les permis déposés à partir du 1er juillet 2026) ;
//! article R. 172-1 du CCH, III (usages 6 à 28) ; fiche d'application FA03 v2
//! (`docs/reference/fiches/re2020-fa03_extension-construction-inf-50_v2.pdf`, tableaux 2 à 4).
//!
//! L'extension se calcule seule, comme un bâtiment, avec les exigences de l'article R. 172-4
//! calculées sur sa propre `Sref` (Q87) ; ce module dit quel régime ouvre sa surface (cumulée
//! avec les autres extensions du même bâtiment) et vérifie les exigences de résultat du régime
//! intermédiaire (Bbio et DH, article 50-3, II ; Q88). Les bornes sont lues à la lettre de
//! l'arrêté (« inférieure à » stricte), non des tableaux de la FA03 (Q86).

use thbce_data::ZoneClimatique;

use super::resultats::{
    FamilleUsage, NatureTravaux, RegimeAlternatif, RegimeExigences, ResultatsExigences,
};
use crate::entree::{Batiment, CategorieContraintes, ParametresExigences};

/// Seuil des constructions et extensions de petite surface (article 50-3, I ; article R. 172-1,
/// III, 3°), en m².
pub const SEUIL_PETITE_SURFACE_M2: f64 = 50.0;

/// `DH_max` d'une surélévation climatisée de maison individuelle en H2d ou H3 sous le régime
/// intermédiaire (article 50-3, II, 1), en °C.h.
pub const DH_MAX_SURELEVATION_MAISON_C_H: f64 = 1400.0;

/// Famille d'usage d'un numéro d'usage (article R. 172-1, I et III).
pub fn famille(usage: u8) -> FamilleUsage {
    match usage {
        1 => FamilleUsage::MaisonIndividuelle,
        2..=5 => FamilleUsage::CollectifBureauxEnseignement,
        _ => FamilleUsage::TertiaireSpecifiqueOuIndustriel,
    }
}

/// Usage principal d'une partie de bâtiment : l'usage de plus grande surface (annexe I,
/// « Usage principal d'un bâtiment », lu en `Sref` ; à égalité, le plus petit numéro). Q86.
pub fn usage_principal(zones: &[(u8, f64)]) -> Option<u8> {
    let mut sommes: Vec<(u8, f64)> = Vec::new();
    for &(u, s) in zones {
        match sommes.iter_mut().find(|(v, _)| *v == u) {
            Some((_, x)) => *x += s,
            None => sommes.push((u, s)),
        }
    }
    sommes.sort_by_key(|&(u, _)| u);
    let mut meilleur: Option<(u8, f64)> = None;
    for (u, s) in sommes {
        if meilleur.is_none_or(|(_, m)| s > m) {
            meilleur = Some((u, s));
        }
    }
    meilleur.map(|(u, _)| u)
}

/// Surfaces qui fixent le régime d'exigences, en m².
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Surfaces {
    /// `Sref` de la partie décrite (extension ou construction).
    pub propre_m2: f64,
    /// Cumul des surfaces des extensions du même bâtiment : « la surface considérée pour
    /// l'application du présent I » de l'article 50-3 ; égal à `propre_m2` sans autre
    /// extension.
    pub cumul_m2: f64,
    /// `Sref` des locaux existants ; `None` : inconnue (aucun seuil de 30 % n'est alors
    /// franchi, ce qui retient le régime complet ; la validation l'exige quand elle sert).
    pub existant_m2: Option<f64>,
}

impl Surfaces {
    /// Une seule extension (ou une construction) de `s_m2`.
    pub fn seule(s_m2: f64, existant_m2: Option<f64>) -> Self {
        Self {
            propre_m2: s_m2,
            cumul_m2: s_m2,
            existant_m2,
        }
    }
}

/// Régime d'exigences (Q86).
///
/// Le cumul des extensions ne sert qu'à l'article 50-3, I, comme sa clause le dit (« pour
/// l'application du présent I ») ; les exclusions de l'article R. 172-1, III et la date
/// d'application de l'article 3, II portent sur la surface de la partie décrite. Bornes à la
/// lettre (« inférieure à » stricte).
pub fn regime(
    nature: NatureTravaux,
    famille: FamilleUsage,
    s: Surfaces,
    p: &ParametresExigences,
) -> RegimeExigences {
    use FamilleUsage as F;
    use RegimeExigences as R;
    let extension = nature != NatureTravaux::Construction;
    let part_inf_30 = |x: f64| s.existant_m2.is_some_and(|e| e > 0.0 && x / e < 0.3);
    let part_sup_30 = |x: f64| s.existant_m2.is_some_and(|e| e > 0.0 && x / e > 0.3);
    // Surface propre : article 3, II et article R. 172-1, III.
    let sp = s.propre_m2;
    // Surface cumulée : article 50-3, I.
    let sc = s.cumul_m2;
    let entre = |x: f64, a: f64, b: f64| x > a && x < b;
    // Arrêté du 4 août 2021, article 3, II : jusqu'au 31 décembre 2022, seule la RT 2012
    // s'applique aux bâtiments et extensions de moins de 50 m², aux extensions de maisons de
    // 50 à 100 m² (strictement) et aux autres extensions de moins de 150 m² et de 30 % de
    // l'existant. Les exigences alternatives de l'article 50-3 datent du 1er janvier 2023.
    if p.annee_depot_permis <= 2022 {
        let rt2012 = sp < SEUIL_PETITE_SURFACE_M2
            || (extension
                && match famille {
                    F::MaisonIndividuelle => entre(sp, 50.0, 100.0),
                    _ => sp < 150.0 && part_inf_30(sp),
                });
        return if rt2012 { R::Rt2012 } else { R::Complet };
    }
    // Article R. 172-1, III, 3° et 4° (usages 6 à 28) : hors du champ ; « cumulativement »
    // y signifie que les deux conditions (150 m² et 30 %) sont réunies.
    if famille == F::TertiaireSpecifiqueOuIndustriel
        && (sp < SEUIL_PETITE_SURFACE_M2 || (extension && sp < 150.0 && part_inf_30(sp)))
    {
        return R::HorsChamp;
    }
    let petite = sc < SEUIL_PETITE_SURFACE_M2;
    match famille {
        // Article 50-3, I : III pour moins de 50 m², II pour une extension de maison de 50 à
        // 80 m² (strictement), surélévations comprises.
        F::MaisonIndividuelle => {
            if petite {
                R::ExigencesDeMoyens
            } else if extension && entre(sc, 50.0, 80.0) {
                R::Intermediaire
            } else {
                R::Complet
            }
        }
        _ if petite => R::ExigencesDeMoyens,
        _ if !extension => R::Complet,
        _ => {
            // Article 50-3, I (rédaction de l'arrêté du 18 mars 2026, permis déposés à partir
            // du 1er juillet 2026) : surélévations à usage autre que de maison.
            let surelevation_2026 =
                nature == NatureTravaux::Surelevation && p.depot_a_partir_de(2026, 7);
            if entre(sc, 50.0, 150.0) && part_inf_30(sc) {
                R::ExigencesDeMoyens
            } else if surelevation_2026
                && ((entre(sc, 50.0, 150.0) && part_sup_30(sc)) || (sc > 150.0 && part_inf_30(sc)))
            {
                R::Intermediaire
            } else {
                R::Complet
            }
        }
    }
}

/// `DH_max` d'un groupe sous le régime intermédiaire : celui de l'article R. 172-4, porté à
/// 1 400 °C.h pour une surélévation climatisée de maison individuelle en H2d ou H3 (article
/// 50-3, II, 1), valeur de la colonne « catégorie 1, climatisé en H2d et H3 » des logements
/// collectifs. Q88 : en catégorie 1 seulement, et `max(DH_maxcat ; 1 400)` (« porter à »
/// n'abaisse pas l'exigence).
pub fn dh_max_intermediaire(
    dh_max: f64,
    nature: NatureTravaux,
    usage: u8,
    climatise: bool,
    zone: ZoneClimatique,
    categorie: CategorieContraintes,
) -> f64 {
    if nature == NatureTravaux::Surelevation
        && usage == 1
        && climatise
        && matches!(zone, ZoneClimatique::H2d | ZoneClimatique::H3)
        && categorie == CategorieContraintes::Categorie1
    {
        dh_max.max(DH_MAX_SURELEVATION_MAISON_C_H)
    } else {
        dh_max
    }
}

/// Régime d'exigences du bâtiment `bat` : présent pour une extension saisie
/// (`exigences.extension`) et pour une construction neuve de moins de 50 m² ; `zones` donne
/// l'usage et la `Sref` de chaque zone ; `ex` : exigences de l'article R. 172-4 déjà vérifiées
/// (Bbio et DH repris par le régime intermédiaire).
pub fn regime_alternatif(
    bat: &Batiment,
    pa: &ParametresExigences,
    sref_m2: f64,
    zones: &[(u8, f64)],
    zone: ZoneClimatique,
    ex: &ResultatsExigences,
) -> Option<RegimeAlternatif> {
    let (nature, surfaces) = match &pa.extension {
        Some(x) => {
            let nature = if x.surelevation {
                NatureTravaux::Surelevation
            } else {
                NatureTravaux::Extension
            };
            let surfaces = Surfaces {
                propre_m2: sref_m2,
                cumul_m2: sref_m2 + x.sref_autres_extensions_m2,
                existant_m2: x.sref_existant_m2.filter(|&e| e > 0.0),
            };
            (nature, surfaces)
        }
        None if sref_m2 < SEUIL_PETITE_SURFACE_M2 => {
            (NatureTravaux::Construction, Surfaces::seule(sref_m2, None))
        }
        None => return None,
    };
    let famille = famille(usage_principal(zones)?);
    let regime = regime(nature, famille, surfaces, pa);
    let s = surfaces.cumul_m2;
    let part = surfaces.existant_m2.map(|e| s / e);
    let mut verifications = Vec::new();
    let mut manquants = Vec::new();
    let conforme = match regime {
        RegimeExigences::Complet => Some(ex.conforme),
        RegimeExigences::Intermediaire => {
            let mut groupes = bat
                .zones
                .iter()
                .flat_map(|z| z.groupes.iter().map(move |g| (z, g)));
            for v in &ex.verifications {
                match v.indicateur.as_str() {
                    "bbio" => verifications.push(v.clone()),
                    "dh" => {
                        // Les vérifications du DH suivent l'ordre des zones et des groupes :
                        // chaque groupe est cherché dans sa zone (identifiants uniques par zone).
                        let groupe = v.groupe.as_deref().unwrap_or_default();
                        let trouve = groupes.find(|(_, g)| g.id == groupe);
                        let max = trouve.map_or(v.maximum, |(z, g)| {
                            dh_max_intermediaire(
                                v.maximum,
                                nature,
                                z.usage.numero(),
                                g.iclim == Some(true),
                                zone,
                                pa.categorie_groupe(&z.id, &g.id),
                            )
                        });
                        verifications.push(super::sorties_th_c::verifier(
                            "dh",
                            v.groupe.clone(),
                            v.valeur,
                            max,
                        ));
                    }
                    _ => {}
                }
            }
            // Bbio (1°) et DH (5°) exigés ; Ic énergie n'est pas exigé par ce régime.
            manquants.extend(
                ex.indicateurs_manquants
                    .iter()
                    .filter(|m| *m == "bbio" || m.starts_with("dh"))
                    .cloned(),
            );
            Some(manquants.is_empty() && verifications.iter().all(|v| v.respectee))
        }
        _ => None,
    };
    Some(RegimeAlternatif {
        nature,
        famille,
        sref_consideree_m2: s,
        part_existant_pct: part.map(|x| 100.0 * x),
        regime,
        verifications,
        conforme,
        indicateurs_manquants: manquants,
    })
}

/// Avertissement du calcul pour les régimes sans exigence de résultat évaluée par le moteur.
pub fn avertissement(batiment: &str, r: &RegimeAlternatif) -> Option<String> {
    let s = r.sref_consideree_m2;
    match r.regime {
        RegimeExigences::Complet | RegimeExigences::Intermediaire => None,
        RegimeExigences::ExigencesDeMoyens => Some(format!(
            "bâtiment {batiment} : surface considérée de {s:.1} m² : exigences alternatives de \
             l'article 50-3, III de l'arrêté du 4 août 2021 (exigences de moyens, hors du \
             moteur) ; les exigences de l'article R. 172-4 restent calculées"
        )),
        RegimeExigences::HorsChamp => Some(format!(
            "bâtiment {batiment} : surface considérée de {s:.1} m² : hors du champ de la RE2020 \
             (article R. 172-1, III du CCH) ; exigences calculées pour information"
        )),
        RegimeExigences::Rt2012 => Some(format!(
            "bâtiment {batiment} : surface considérée de {s:.1} m², permis déposé en 2022 : \
             seule la RT 2012 s'applique (arrêté du 4 août 2021, article 3, II) ; exigences \
             calculées pour information"
        )),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::entree::{ExtensionSaisie, ZoneBruit};
    use FamilleUsage as F;
    use NatureTravaux as N;
    use RegimeExigences as R;

    fn p(annee: u16, mois: Option<u8>) -> ParametresExigences {
        ParametresExigences {
            categorie_contraintes_exterieures: CategorieContraintes::Categorie1,
            zone_bruit: Some(ZoneBruit::Br1),
            annee_depot_permis: annee,
            mois_depot_permis: mois,
            hsp_moy_m: Some(2.5),
            s_combles_m2: 0.0,
            reseau_chaleur_classe: false,
            parcelle_raccordement_gaz: false,
            zones: Vec::new(),
            extension: Some(ExtensionSaisie {
                surelevation: false,
                sref_existant_m2: None,
                sref_autres_extensions_m2: 0.0,
            }),
        }
    }

    /// Une seule extension de `s` m² sur `e` m² existants.
    fn sf(s: f64, e: Option<f64>) -> Surfaces {
        Surfaces::seule(s, e)
    }

    /// Article 50-3, I, à la lettre (« inférieure à » stricte) ; cas aux frontières de la
    /// relecture de la PR #58.
    #[test]
    fn maisons_individuelles() {
        let q = p(2025, None);
        let r = |n, s| regime(n, F::MaisonIndividuelle, sf(s, None), &q);
        assert_eq!(r(N::Extension, 49.9), R::ExigencesDeMoyens);
        assert_eq!(r(N::Construction, 45.0), R::ExigencesDeMoyens);
        assert_eq!(r(N::Extension, 50.0), R::Complet);
        assert_eq!(r(N::Extension, 50.1), R::Intermediaire);
        assert_eq!(r(N::Extension, 60.0), R::Intermediaire);
        assert_eq!(r(N::Surelevation, 79.9), R::Intermediaire);
        assert_eq!(r(N::Extension, 79.9), R::Intermediaire);
        assert_eq!(r(N::Extension, 80.0), R::Complet);
        assert_eq!(r(N::Construction, 60.0), R::Complet);
        // Permis de 2022 (article 3, II) : RT 2012 sous 50 m² et strictement de 50 à 100 m².
        let q = p(2022, None);
        let r = |n, s| regime(n, F::MaisonIndividuelle, sf(s, None), &q);
        assert_eq!(r(N::Extension, 49.9), R::Rt2012);
        assert_eq!(r(N::Construction, 45.0), R::Rt2012);
        assert_eq!(r(N::Extension, 50.0), R::Complet);
        assert_eq!(r(N::Extension, 80.0), R::Rt2012);
        assert_eq!(r(N::Extension, 99.9), R::Rt2012);
        assert_eq!(r(N::Extension, 100.0), R::Complet);
        assert_eq!(r(N::Construction, 90.0), R::Complet);
    }

    /// Logements collectifs, bureaux, enseignement (FA03 v2, tableau 3).
    #[test]
    fn collectif_bureaux_enseignement() {
        let f = F::CollectifBureauxEnseignement;
        let q = p(2025, None);
        let r = |n, s, e: f64| regime(n, f, sf(s, Some(e)), &q);
        assert_eq!(r(N::Extension, 40.0, 80.0), R::ExigencesDeMoyens);
        // 120 m² : 29,9 % → III ; 30 % → complet.
        assert_eq!(r(N::Extension, 120.0, 120.0 / 0.299), R::ExigencesDeMoyens);
        assert_eq!(r(N::Extension, 120.0, 400.0), R::Complet);
        assert_eq!(r(N::Extension, 120.0, 300.0), R::Complet);
        // À 20 % : 149,9 m² → III ; 150 m² → complet.
        assert_eq!(r(N::Extension, 149.9, 749.5), R::ExigencesDeMoyens);
        assert_eq!(r(N::Extension, 150.0, 750.0), R::Complet);
        // Avant juillet 2026, une surélévation suit les règles des extensions.
        assert_eq!(r(N::Surelevation, 120.0, 300.0), R::Complet);
        assert_eq!(r(N::Surelevation, 200.0, 1000.0), R::Complet);
        // À partir du 1er juillet 2026 (arrêté du 18 mars 2026).
        let q = p(2026, Some(7));
        let r = |n, s, e: f64| regime(n, f, sf(s, Some(e)), &q);
        assert_eq!(r(N::Surelevation, 40.0, 400.0), R::ExigencesDeMoyens);
        assert_eq!(
            r(N::Surelevation, 120.0, 120.0 / 0.299),
            R::ExigencesDeMoyens
        );
        assert_eq!(r(N::Surelevation, 120.0, 400.0), R::Complet);
        assert_eq!(r(N::Surelevation, 120.0, 120.0 / 0.301), R::Intermediaire);
        assert_eq!(r(N::Surelevation, 150.1, 750.5), R::Intermediaire);
        assert_eq!(r(N::Surelevation, 200.0, 500.0), R::Complet);
        assert_eq!(r(N::Extension, 200.0, 1000.0), R::Complet);
        let q = p(2026, Some(6));
        assert_eq!(
            regime(N::Surelevation, f, sf(150.1, Some(750.5)), &q),
            R::Complet
        );
        // Permis de 2022 : RT 2012 sous 150 m² et 30 %.
        let q = p(2022, Some(9));
        let r = |n, s, e: f64| regime(n, f, sf(s, Some(e)), &q);
        assert_eq!(r(N::Extension, 120.0, 1000.0), R::Rt2012);
        assert_eq!(r(N::Extension, 120.0, 300.0), R::Complet);
        assert_eq!(regime(N::Construction, f, sf(45.0, None), &q), R::Rt2012);
    }

    /// Usages 6 à 28 (FA03 v2, tableau 4, et article R. 172-1, III).
    #[test]
    fn tertiaire_specifique() {
        let f = F::TertiaireSpecifiqueOuIndustriel;
        let q = p(2026, Some(9));
        let r = |n, s, e: f64| regime(n, f, sf(s, Some(e)), &q);
        assert_eq!(regime(N::Construction, f, sf(49.9, None), &q), R::HorsChamp);
        assert_eq!(r(N::Extension, 49.9, 99.8), R::HorsChamp);
        assert_eq!(r(N::Extension, 50.0, 100.0), R::Complet);
        assert_eq!(r(N::Extension, 149.9, 149.9 / 0.299), R::HorsChamp);
        assert_eq!(r(N::Extension, 150.0, 750.0), R::Complet);
        assert_eq!(r(N::Extension, 100.0, 200.0), R::Complet);
        // Surélévations : le texte (R. 172-1, III) les exclut comme les extensions ; la FA03 les
        // range dans les exigences de moyens (Q86).
        assert_eq!(r(N::Surelevation, 100.0, 500.0), R::HorsChamp);
        assert_eq!(r(N::Surelevation, 100.0, 200.0), R::Intermediaire);
        assert_eq!(r(N::Surelevation, 160.0, 800.0), R::Intermediaire);
        assert_eq!(r(N::Surelevation, 160.0, 320.0), R::Complet);
    }

    /// Cumul des extensions : article 50-3, I seulement (Q86, point 4b).
    #[test]
    fn cumul_des_extensions() {
        let q = p(2026, Some(9));
        // Commerce : 100 m² décrits sur 1 000 m² existants, 60 m² d'autres extensions. R. 172-1,
        // III porte sur la partie décrite : hors du champ.
        let s = Surfaces {
            propre_m2: 100.0,
            cumul_m2: 160.0,
            existant_m2: Some(1000.0),
        };
        let f = F::TertiaireSpecifiqueOuIndustriel;
        assert_eq!(regime(N::Extension, f, s, &q), R::HorsChamp);
        // Bureaux : 40 m² décrits, 30 m² d'autres extensions → 70 m² cumulés, sur 1 000 m² :
        // article 50-3, III (et non « moins de 50 m² »).
        let s = Surfaces {
            propre_m2: 40.0,
            cumul_m2: 70.0,
            existant_m2: Some(1000.0),
        };
        let f = F::CollectifBureauxEnseignement;
        assert_eq!(regime(N::Extension, f, s, &q), R::ExigencesDeMoyens);
        // Maison : 40 + 30 m² → 70 m² cumulés : article 50-3, II.
        let s = Surfaces {
            propre_m2: 40.0,
            cumul_m2: 70.0,
            existant_m2: None,
        };
        assert_eq!(
            regime(N::Extension, F::MaisonIndividuelle, s, &q),
            R::Intermediaire
        );
        // Permis de 2022 : l'article 3, II porte sur la partie décrite.
        let q = p(2022, None);
        assert_eq!(
            regime(N::Extension, F::MaisonIndividuelle, s, &q),
            R::Rt2012
        );
    }

    #[test]
    fn usage_principal_et_famille() {
        assert_eq!(usage_principal(&[(2, 60.0), (3, 200.0)]), Some(3));
        assert_eq!(
            usage_principal(&[(3, 100.0), (2, 60.0), (2, 60.0)]),
            Some(2)
        );
        assert_eq!(usage_principal(&[(4, 100.0), (3, 100.0)]), Some(3));
        assert_eq!(usage_principal(&[]), None);
        assert_eq!(famille(1), F::MaisonIndividuelle);
        assert_eq!(famille(5), F::CollectifBureauxEnseignement);
        assert_eq!(famille(6), F::TertiaireSpecifiqueOuIndustriel);
    }

    #[test]
    fn dh_des_surelevations_de_maisons_climatisees() {
        let c1 = CategorieContraintes::Categorie1;
        let z = ZoneClimatique::H3;
        assert_eq!(
            dh_max_intermediaire(1250.0, N::Surelevation, 1, true, z, c1),
            1400.0
        );
        assert_eq!(
            dh_max_intermediaire(1250.0, N::Extension, 1, true, z, c1),
            1250.0
        );
        assert_eq!(
            dh_max_intermediaire(1250.0, N::Surelevation, 1, false, z, c1),
            1250.0
        );
        assert_eq!(
            dh_max_intermediaire(1250.0, N::Surelevation, 1, true, ZoneClimatique::H2c, c1),
            1250.0
        );
        assert_eq!(
            dh_max_intermediaire(
                1850.0,
                N::Surelevation,
                1,
                true,
                z,
                CategorieContraintes::Categorie2
            ),
            1850.0
        );
    }
}
