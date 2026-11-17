// SPDX-License-Identifier: GPL-3.0-or-later
//! Exigences maximales et conformité : annexe de l'article R. 172-4 du code de la construction
//! et de l'habitation, chapitres II et III (`docs/reference/arrete/chapitre1a3_annexe_r172-4_post-rivaton-i.pdf`,
//! version documentaire du site RT-RE bâtiment pour les permis déposés à partir du 1er juillet
//! 2026 ; tableaux relus sur les images des p. 6 à 9, 30 à 37 et 100).
//!
//! Seuls les usages de la première version sont transcrits : maisons individuelles ou accolées
//! (usage 1) et logements collectifs (usage 2). Les catégories de contraintes extérieures et les
//! zones de bruit (chapitre V, `docs/reference/texte/annexe-r172-4-chapitre-5.txt`) sont
//! saisies ; la validation refuse les catégories que la partie II du chapitre V exclut.

use thbce_data::ZoneClimatique;

use crate::entree::{CategorieContraintes, ParametresExigences, ZoneBruit};

/// Classe d'altitude des tableaux `Mbgéo` et `Mcgéo`, de 0 à 2, à la lettre de l'annexe :
/// « < 400m », « 400m-800m », « >800m » ; 400 m pile relève de la deuxième classe, 800 m pile
/// aussi (Q54 ; le tableau 336 du forfait de refroidissement a ses propres bornes).
pub fn classe_altitude(alt_m: f64) -> usize {
    if alt_m < 400.0 {
        0
    } else if alt_m <= 800.0 {
        1
    } else {
        2
    }
}

/// Description d'une zone (partie de bâtiment d'un usage) pour la fixation des exigences.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartieBatiment {
    /// Numéro d'usage (1 ou 2).
    pub usage: u8,
    /// « Sref » de la zone, en m².
    pub sref_m2: f64,
    /// « NL » : nombre de logements de la zone.
    pub nb_logements: u32,
    /// Somme des « Sref » des zones du bâtiment de même usage, en m² (`Mbsurf_tot`,
    /// `Mcsurf_tot` : chapitre II, I et II).
    pub sref_usage_m2: f64,
    /// Bâtiment raccordé à un réseau de chaleur urbain.
    pub raccorde_rcu: bool,
}

/// Coefficients et valeurs des exigences d'une zone.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ExigencesPartie {
    /// « Smoylgt » : surface moyenne des logements, en m² (chapitre I, §XI).
    pub smoy_lgt_m2: f64,
    /// « Bbio_maxmoyen », en points.
    pub bbio_max_moyen: f64,
    /// « Mbgéo ».
    pub mb_geo: f64,
    /// « Mbcombles ».
    pub mb_combles: f64,
    /// « Mbsurf_moy ».
    pub mb_surf_moy: f64,
    /// « Mbsurf_tot ».
    pub mb_surf_tot: f64,
    /// « Mbbruit ».
    pub mb_bruit: f64,
    /// « MbHSP ».
    pub mb_hsp: f64,
    /// « Bbio_max », en points.
    pub bbio_max: f64,
    /// « Cep,nr_maxmoyen », en kWhep/(m².an).
    pub cep_nr_max_moyen: f64,
    /// « Cep_maxmoyen », en kWhep/(m².an).
    pub cep_max_moyen: f64,
    /// « Icénergie_maxmoyen », en kg éq. CO2/m².
    pub ic_energie_max_moyen: f64,
    /// « Mcgéo ».
    pub mc_geo: f64,
    /// « Mccombles ».
    pub mc_combles: f64,
    /// « Mcsurf_moy ».
    pub mc_surf_moy: f64,
    /// « Mcsurf_tot ».
    pub mc_surf_tot: f64,
    /// « Mccat ».
    pub mc_cat: f64,
    /// « McHSP ».
    pub mc_hsp: f64,
    /// « Cep,nr_max », en kWhep/(m².an).
    pub cep_nr_max: f64,
    /// « Cep_max », en kWhep/(m².an).
    pub cep_max: f64,
    /// « Icénergie_max », en kg éq. CO2/m².
    pub ic_energie_max: f64,
}

/// Indice de la zone climatique dans les colonnes des tableaux (H1a à H3).
fn colonne(zone: ZoneClimatique) -> usize {
    use ZoneClimatique as Z;
    match zone {
        Z::H1a => 0,
        Z::H1b => 1,
        Z::H1c => 2,
        Z::H2a => 3,
        Z::H2b => 4,
        Z::H2c => 5,
        Z::H2d => 6,
        Z::H3 => 7,
    }
}

/// Zones H2d et H3.
fn zone_chaude(zone: ZoneClimatique) -> bool {
    matches!(zone, ZoneClimatique::H2d | ZoneClimatique::H3)
}

/// « Mbgéo » (chapitre III, I, tableaux des maisons et des logements collectifs).
fn mb_geo(usage: u8, zone: ZoneClimatique, alt_m: f64) -> f64 {
    const MAISONS: [[f64; 8]; 3] = [
        [0.15, 0.2, 0.2, -0.05, 0.0, -0.1, 0.05, -0.1],
        [0.4, 0.5, 0.45, 0.15, 0.3, 0.05, 0.1, -0.05],
        [0.75, 0.85, 0.75, 0.55, 0.65, 0.35, 0.25, 0.1],
    ];
    const COLLECTIFS: [[f64; 8]; 3] = [
        [0.1, 0.20, 0.15, -0.1, 0.0, -0.1, 0.0, -0.1],
        [0.4, 0.5, 0.45, 0.2, 0.3, 0.1, 0.2, -0.05],
        [0.8, 0.85, 0.75, 0.6, 0.65, 0.4, 0.4, 0.15],
    ];
    let t = if usage == 1 { &MAISONS } else { &COLLECTIFS };
    t[classe_altitude(alt_m)][colonne(zone)]
}

/// « Mcgéo » (chapitre III, II, 1 et 2).
fn mc_geo(usage: u8, zone: ZoneClimatique, alt_m: f64) -> f64 {
    const MAISONS: [[f64; 8]; 3] = [
        [0.1, 0.15, 0.1, -0.05, 0.0, -0.1, -0.10, -0.15],
        [0.4, 0.5, 0.4, 0.15, 0.3, 0.05, 0.0, -0.1],
        [0.75, 0.85, 0.75, 0.55, 0.6, 0.35, 0.25, 0.15],
    ];
    const COLLECTIFS: [[f64; 8]; 3] = [
        [0.05, 0.05, 0.05, -0.1, 0.0, -0.15, -0.1, -0.15],
        [0.35, 0.4, 0.35, 0.2, 0.2, 0.05, 0.05, -0.1],
        [0.55, 0.65, 0.55, 0.45, 0.5, 0.3, 0.3, 0.15],
    ];
    let t = if usage == 1 { &MAISONS } else { &COLLECTIFS };
    t[classe_altitude(alt_m)][colonne(zone)]
}

/// « Icénergie_maxmoyen », en kg éq. CO2/m² (chapitre III, II), selon l'année de dépôt du
/// permis de construire et le raccordement à un réseau de chaleur urbain.
pub fn ic_energie_max_moyen(usage: u8, p: &ParametresExigences, raccorde_rcu: bool) -> f64 {
    // Colonnes : 2022 à 2024, 2025 à 2027, à partir de 2028.
    let (rcu, autres) = match usage {
        1 => ([200.0, 200.0, 160.0], [160.0, 160.0, 160.0]),
        _ => ([560.0, 320.0, 260.0], [560.0, 260.0, 260.0]),
    };
    let periode = match p.annee_depot_permis {
        ..=2024 => 0,
        2025..=2027 => 1,
        _ => 2,
    };
    // Maisons dont la parcelle prévoyait un raccordement au gaz avant 2022 : 280 pour un
    // permis déposé avant le 31 décembre 2023.
    if usage == 1 && p.parcelle_raccordement_gaz && p.annee_depot_permis <= 2023 {
        return 280.0;
    }
    if raccorde_rcu {
        // Réseau classé : valeurs de 2022 à 2024 pour un permis déposé avant le 31 décembre
        // 2027.
        if p.reseau_chaleur_classe && p.annee_depot_permis <= 2027 {
            rcu[0]
        } else {
            rcu[periode]
        }
    } else {
        autres[periode]
    }
}

/// `MIN(HSPmoy ; 2,9)/2,5 − 1` pour `HSPmoy` > 2,5 m, 0 sinon.
fn ecart_hsp(hsp_moy_m: f64) -> f64 {
    if hsp_moy_m > 2.5 {
        hsp_moy_m.min(2.9) / 2.5 - 1.0
    } else {
        0.0
    }
}

/// Exigences d'une zone d'usage 1 ou 2 ; `None` pour un autre usage.
pub fn exigences_partie(
    z: &PartieBatiment,
    p: &ParametresExigences,
    zone: ZoneClimatique,
    alt_m: f64,
) -> Option<ExigencesPartie> {
    if !matches!(z.usage, 1 | 2) {
        return None;
    }
    let maison = z.usage == 1;
    let sref = z.sref_m2;
    let s = if z.nb_logements > 0 {
        sref / f64::from(z.nb_logements)
    } else {
        sref
    };
    let col = colonne(zone);
    let cat2 = p.categorie_contraintes_exterieures == CategorieContraintes::Categorie2;
    let combles = if maison && sref > 0.0 {
        0.4 * p.s_combles_m2 / sref
    } else {
        0.0
    };

    // I. Bbio_max.
    let bbio_moy = if maison { 63.0 } else { 65.0 };
    let mb_surf_moy = if maison {
        if s <= 100.0 {
            (49.0 - 0.49 * s) / bbio_moy
        } else if s <= 150.0 {
            (18.0 - 0.18 * s) / bbio_moy
        } else {
            -9.0 / bbio_moy
        }
    } else if s <= 80.0 {
        (-6.0 + 0.1 * s) / bbio_moy
    } else if s <= 120.0 {
        (-2.0 + 0.05 * s) / bbio_moy
    } else {
        4.0 / bbio_moy
    };
    let mb_surf_tot = if !maison && z.sref_usage_m2 <= 1300.0 {
        (19.5 - 0.015 * z.sref_usage_m2) / bbio_moy
    } else {
        0.0
    };
    let mb_bruit = if p.zone_bruit == ZoneBruit::Br1 {
        0.0
    } else if maison {
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.1, 0.1][col]
    } else {
        [0.0, 0.0, 0.1, 0.0, 0.0, 0.1, 0.2, 0.2][col]
    };
    let e = ecart_hsp(p.hsp_moy_m);
    let mb_hsp = if maison {
        if sref <= 150.0 {
            e
        } else {
            e / 2.0
        }
    } else {
        e / 1.5
    };
    let mb_geo = mb_geo(z.usage, zone, alt_m);
    let bbio_max =
        bbio_moy * (1.0 + mb_geo + combles + mb_surf_moy + mb_surf_tot + mb_bruit + mb_hsp);

    // II. Cep,nr_max, Cep_max et Icénergie_max.
    let (cep_nr_moy, cep_moy) = if maison { (55.0, 75.0) } else { (70.0, 85.0) };
    let mc_surf_moy = if maison {
        if s <= 100.0 {
            (49.5 - 0.55 * s) / cep_nr_moy
        } else if s <= 150.0 {
            (14.5 - 0.2 * s) / cep_nr_moy
        } else {
            -15.5 / cep_nr_moy
        }
    } else if s <= 40.0 {
        (45.0 - s) / cep_nr_moy
    } else if s <= 80.0 {
        (15.0 - 0.25 * s) / cep_nr_moy
    } else if s <= 120.0 {
        (3.0 - 0.1 * s) / cep_nr_moy
    } else {
        -9.0 / cep_nr_moy
    };
    let mc_surf_tot = if !maison && z.sref_usage_m2 <= 1300.0 {
        (13.0 - 0.01 * z.sref_usage_m2) / cep_nr_moy
    } else {
        0.0
    };
    let mc_cat = if cat2 && zone_chaude(zone) { 0.1 } else { 0.0 };
    let mc_hsp = if maison && sref <= 150.0 {
        e / 2.0
    } else {
        e / 4.0
    };
    let mc_geo = mc_geo(z.usage, zone, alt_m);
    let m = 1.0 + mc_geo + combles + mc_surf_moy + mc_surf_tot + mc_cat + mc_hsp;
    let ic_moy = ic_energie_max_moyen(z.usage, p, z.raccorde_rcu);
    Some(ExigencesPartie {
        smoy_lgt_m2: s,
        bbio_max_moyen: bbio_moy,
        mb_geo,
        mb_combles: combles,
        mb_surf_moy,
        mb_surf_tot,
        mb_bruit,
        mb_hsp,
        bbio_max,
        cep_nr_max_moyen: cep_nr_moy,
        cep_max_moyen: cep_moy,
        ic_energie_max_moyen: ic_moy,
        mc_geo,
        mc_combles: combles,
        mc_surf_moy,
        mc_surf_tot,
        mc_cat,
        mc_hsp,
        cep_nr_max: cep_nr_moy * m,
        cep_max: cep_moy * m,
        ic_energie_max: ic_moy * m,
    })
}

/// « DH_max = DH_maxcat » d'un groupe (chapitre II, IV, et chapitre III, IV), en °C.h.
///
/// `climatise` : groupe climatisé (`iclim`) ; `smoy_lgt_m2` : surface moyenne des logements de
/// sa zone. `None` pour un usage autre que 1 et 2.
pub fn dh_max(
    usage: u8,
    categorie: CategorieContraintes,
    zone: ZoneClimatique,
    climatise: bool,
    smoy_lgt_m2: f64,
) -> Option<f64> {
    let cat2 = categorie == CategorieContraintes::Categorie2;
    match usage {
        1 => Some(if cat2 { 1850.0 } else { 1250.0 }),
        2 => {
            let s = smoy_lgt_m2;
            Some(if cat2 {
                if s <= 20.0 {
                    2600.0
                } else if s <= 60.0 {
                    2850.0 - 12.5 * s
                } else {
                    2100.0
                }
            } else if climatise && zone_chaude(zone) {
                if s <= 20.0 {
                    1600.0
                } else if s <= 60.0 {
                    1700.0 - 5.0 * s
                } else {
                    1400.0
                }
            } else {
                1250.0
            })
        }
        _ => None,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn parametres() -> ParametresExigences {
        ParametresExigences {
            categorie_contraintes_exterieures: CategorieContraintes::Categorie1,
            zone_bruit: ZoneBruit::Br1,
            annee_depot_permis: 2022,
            hsp_moy_m: 2.5,
            s_combles_m2: 0.0,
            reseau_chaleur_classe: false,
            parcelle_raccordement_gaz: false,
        }
    }

    #[test]
    fn maison_moyenne_en_h2b() {
        // Maison de 100 m² en H2b, à moins de 400 m : Mbgéo = Mcgéo = 0 ;
        // Mbsurf_moy = (49 − 49)/63 = 0 ; Mcsurf_moy = (49,5 − 55)/55 = −0,1.
        let z = PartieBatiment {
            usage: 1,
            sref_m2: 100.0,
            nb_logements: 1,
            sref_usage_m2: 100.0,
            raccorde_rcu: false,
        };
        let e = exigences_partie(&z, &parametres(), ZoneClimatique::H2b, 100.0).unwrap();
        assert!((e.bbio_max - 63.0).abs() < 1e-9);
        assert!((e.mc_surf_moy + 0.1).abs() < 1e-12);
        assert!((e.cep_max - 75.0 * 0.9).abs() < 1e-9);
        assert!((e.cep_nr_max - 55.0 * 0.9).abs() < 1e-9);
        assert!((e.ic_energie_max - 160.0 * 0.9).abs() < 1e-9);
        // Maison de 80 m² : Mcsurf_moy = (49,5 − 44)/55 = 0,1 ; Cep_max = 82,5.
        let z = PartieBatiment {
            sref_m2: 80.0,
            sref_usage_m2: 80.0,
            ..z
        };
        let e = exigences_partie(&z, &parametres(), ZoneClimatique::H2b, 100.0).unwrap();
        assert!((e.cep_max - 82.5).abs() < 1e-9);
        assert!((e.bbio_max - 63.0 * (1.0 + (49.0 - 0.49 * 80.0) / 63.0)).abs() < 1e-9);
    }

    #[test]
    fn hauteur_sous_plafond_bruit_et_categorie() {
        let mut p = parametres();
        p.hsp_moy_m = 3.2;
        p.zone_bruit = ZoneBruit::Br2;
        p.categorie_contraintes_exterieures = CategorieContraintes::Categorie2;
        let z = PartieBatiment {
            usage: 1,
            sref_m2: 120.0,
            nb_logements: 1,
            sref_usage_m2: 120.0,
            raccorde_rcu: false,
        };
        let e = exigences_partie(&z, &p, ZoneClimatique::H3, 100.0).unwrap();
        // MIN(3,2 ; 2,9)/2,5 − 1 = 0,16.
        assert!((e.mb_hsp - 0.16).abs() < 1e-12);
        assert!((e.mc_hsp - 0.08).abs() < 1e-12);
        assert_eq!(e.mb_bruit, 0.1);
        assert_eq!(e.mc_cat, 0.1);
        assert_eq!(
            dh_max(
                1,
                p.categorie_contraintes_exterieures,
                ZoneClimatique::H3,
                false,
                120.0
            ),
            Some(1850.0)
        );
    }

    #[test]
    fn logements_collectifs() {
        let z = PartieBatiment {
            usage: 2,
            sref_m2: 780.0,
            nb_logements: 12,
            sref_usage_m2: 780.0,
            raccorde_rcu: false,
        };
        let e = exigences_partie(&z, &parametres(), ZoneClimatique::H2b, 100.0).unwrap();
        // Smoylgt = 65 m² : Mbsurf_moy = (−6 + 6,5)/65 ; Mbsurf_tot = (19,5 − 11,7)/65.
        assert!((e.smoy_lgt_m2 - 65.0).abs() < 1e-12);
        assert!((e.bbio_max - (65.0 + 0.5 + 7.8)).abs() < 1e-9);
        // Mcsurf_moy = (15 − 16,25)/70 ; Mcsurf_tot = (13 − 7,8)/70.
        let m = 1.0 + (15.0 - 16.25) / 70.0 + (13.0 - 7.8) / 70.0;
        assert!((e.cep_max - 85.0 * m).abs() < 1e-9);
        assert!((e.ic_energie_max - 560.0 * m).abs() < 1e-9);
        let c1 = CategorieContraintes::Categorie1;
        assert_eq!(dh_max(2, c1, ZoneClimatique::H2b, true, 65.0), Some(1250.0));
        assert_eq!(dh_max(2, c1, ZoneClimatique::H3, true, 40.0), Some(1500.0));
        let c2 = CategorieContraintes::Categorie2;
        assert_eq!(
            dh_max(2, c2, ZoneClimatique::H2b, false, 40.0),
            Some(2350.0)
        );
    }

    /// Valeurs recalculées à la main par la relecture indépendante du lot 7.
    #[test]
    fn valeurs_de_la_relecture_independante() {
        let proche = |a: f64, b: f64| assert!((a - b).abs() < 1e-3, "{a} ≠ {b}");
        let mut p = parametres();
        p.annee_depot_permis = 2025;
        let maison = |rcu: bool| PartieBatiment {
            usage: 1,
            sref_m2: 96.44,
            nb_logements: 1,
            sref_usage_m2: 96.44,
            raccorde_rcu: rcu,
        };
        let z = ZoneClimatique::H2b;
        let e = exigences_partie(&maison(false), &p, z, 100.0).unwrap();
        proche(e.bbio_max, 64.7444);
        proche(e.cep_max, 70.1700);
        proche(e.cep_nr_max, 51.4580);
        proche(e.ic_energie_max, 149.6960);
        proche(
            exigences_partie(&maison(true), &p, z, 100.0)
                .unwrap()
                .ic_energie_max,
            187.12,
        );
        for annee in [2028, 2031] {
            p.annee_depot_permis = annee;
            let e = exigences_partie(&maison(true), &p, z, 100.0).unwrap();
            proche(e.ic_energie_max, 149.696);
        }
        p.parcelle_raccordement_gaz = true;
        p.annee_depot_permis = 2023;
        proche(
            exigences_partie(&maison(false), &p, z, 100.0)
                .unwrap()
                .ic_energie_max,
            261.968,
        );
        p.annee_depot_permis = 2024;
        proche(
            exigences_partie(&maison(false), &p, z, 100.0)
                .unwrap()
                .ic_energie_max,
            149.696,
        );

        let mut p = parametres();
        let immeuble = |rcu: bool| PartieBatiment {
            usage: 2,
            sref_m2: 720.0,
            nb_logements: 12,
            sref_usage_m2: 720.0,
            raccorde_rcu: rcu,
        };
        p.annee_depot_permis = 2025;
        let e = exigences_partie(&immeuble(false), &p, z, 100.0).unwrap();
        proche(e.bbio_max, 73.7);
        proche(e.cep_max, 92.042857);
        proche(e.cep_nr_max, 75.8);
        proche(e.ic_energie_max, 281.542857);
        proche(
            exigences_partie(&immeuble(true), &p, z, 100.0)
                .unwrap()
                .ic_energie_max,
            346.514,
        );
        p.annee_depot_permis = 2022;
        proche(
            exigences_partie(&immeuble(false), &p, z, 100.0)
                .unwrap()
                .ic_energie_max,
            606.4,
        );
        p.reseau_chaleur_classe = true;
        p.annee_depot_permis = 2027;
        proche(
            exigences_partie(&immeuble(true), &p, z, 100.0)
                .unwrap()
                .ic_energie_max,
            606.4,
        );
        p.annee_depot_permis = 2028;
        // Valeur donnée à deux décimales par la relecture (281,542857 arrondi).
        let e = exigences_partie(&immeuble(true), &p, z, 100.0).unwrap();
        assert!(
            (e.ic_energie_max - 281.54).abs() < 5e-3,
            "{}",
            e.ic_energie_max
        );
    }

    #[test]
    fn classes_d_altitude_a_la_lettre_de_l_annexe() {
        assert_eq!(classe_altitude(399.9), 0);
        assert_eq!(classe_altitude(400.0), 1);
        assert_eq!(classe_altitude(800.0), 1);
        assert_eq!(classe_altitude(800.1), 2);
        // Le tableau 336 du forfait garde « [0 ; 400 m] ».
        assert_eq!(
            super::super::forfait_refroidissement::classe_altitude(400.0),
            0
        );
    }

    #[test]
    fn ic_energie_max_moyen_selon_l_annee_et_le_reseau() {
        let mut p = parametres();
        assert_eq!(ic_energie_max_moyen(1, &p, false), 160.0);
        assert_eq!(ic_energie_max_moyen(1, &p, true), 200.0);
        assert_eq!(ic_energie_max_moyen(2, &p, false), 560.0);
        p.annee_depot_permis = 2026;
        assert_eq!(ic_energie_max_moyen(2, &p, false), 260.0);
        assert_eq!(ic_energie_max_moyen(2, &p, true), 320.0);
        p.reseau_chaleur_classe = true;
        assert_eq!(ic_energie_max_moyen(2, &p, true), 560.0);
        p.annee_depot_permis = 2028;
        assert_eq!(ic_energie_max_moyen(1, &p, true), 160.0);
        assert_eq!(ic_energie_max_moyen(2, &p, true), 260.0);
        p.annee_depot_permis = 2023;
        p.parcelle_raccordement_gaz = true;
        assert_eq!(ic_energie_max_moyen(1, &p, false), 280.0);
    }
}
