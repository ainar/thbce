// SPDX-License-Identifier: GPL-3.0-or-later
//! Classement au bruit des baies, des groupes et des zones : fiche d'application 5.11
//! (`FA_Classement_au_bruit_d'une_baie`, annexe III, p. 245 à 250) et chapitre V, partie I, de
//! l'annexe de l'article R. 172-4 du code de la construction et de l'habitation (points 3 et 4).
//!
//! Fonctions pures. La catégorie des infrastructures, leur distance, la vue et la zone du plan
//! d'exposition au bruit sont des données saisies (Q200).

use crate::entree::{
    Baie, Batiment, BruitBaie, CategorieInfrastructure, ExigencesZoneSaisie, ExpoBr, Groupe,
    InfrastructureBruit, Projet, VueInfrastructure, Zone, ZoneBruit, ZonePeb,
};

use ExpoBr::{Br1, Br2, Br3};

/// Distance maximale de prise en compte d'une infrastructure, en m (configuration 1 : au-delà,
/// toutes les baies sont BR1).
const fn distance_maximale_m(c: CategorieInfrastructure) -> f64 {
    match c {
        CategorieInfrastructure::Categorie1 => 700.0,
        CategorieInfrastructure::Categorie2 => 500.0,
        CategorieInfrastructure::Categorie3 => 250.0,
        CategorieInfrastructure::Categorie4 => 100.0,
        CategorieInfrastructure::Categorie5 => 30.0,
    }
}

/// Une ligne de tableau : borne supérieure de la tranche de distance (m), puis la classe pour
/// les vues directe, partielle, masquée par obstacles peu protecteurs, masquée par obstacles très
/// protecteurs, arrière. Au-delà de la dernière borne : BR1 pour toutes les vues.
type Ligne = (f64, [ExpoBr; 5]);

/// Tableaux de la partie I, configuration 2, « infrastructures de transports terrestres »,
/// relus sur l'image (fiche 5.11.7, p. 249 et 250 ; annexe de l'article R. 172-4, p. 135 à 140).
fn tableau(c: CategorieInfrastructure) -> &'static [Ligne] {
    match c {
        CategorieInfrastructure::Categorie1 => &[
            (65.0, [Br3, Br3, Br3, Br3, Br3]),
            (125.0, [Br3, Br3, Br3, Br3, Br2]),
            (250.0, [Br3, Br3, Br3, Br2, Br2]),
            (400.0, [Br3, Br2, Br2, Br2, Br1]),
            (550.0, [Br2, Br2, Br2, Br1, Br1]),
            (700.0, [Br2, Br1, Br1, Br1, Br1]),
        ],
        CategorieInfrastructure::Categorie2 => &[
            (30.0, [Br3, Br3, Br3, Br3, Br3]),
            (65.0, [Br3, Br3, Br3, Br3, Br2]),
            (125.0, [Br3, Br3, Br3, Br2, Br2]),
            (250.0, [Br3, Br2, Br2, Br2, Br1]),
            (370.0, [Br2, Br2, Br2, Br1, Br1]),
            (500.0, [Br2, Br1, Br1, Br1, Br1]),
        ],
        CategorieInfrastructure::Categorie3 => &[
            (25.0, [Br3, Br3, Br3, Br3, Br2]),
            (50.0, [Br3, Br3, Br3, Br2, Br2]),
            (100.0, [Br3, Br2, Br2, Br2, Br1]),
            (160.0, [Br2, Br2, Br2, Br1, Br1]),
            (250.0, [Br2, Br1, Br1, Br1, Br1]),
        ],
        CategorieInfrastructure::Categorie4 => &[
            (15.0, [Br3, Br3, Br3, Br2, Br2]),
            (30.0, [Br3, Br2, Br2, Br2, Br1]),
            (60.0, [Br2, Br2, Br2, Br1, Br1]),
            (100.0, [Br2, Br1, Br1, Br1, Br1]),
        ],
        CategorieInfrastructure::Categorie5 => &[
            (10.0, [Br3, Br2, Br2, Br2, Br1]),
            (20.0, [Br2, Br2, Br2, Br1, Br1]),
            (30.0, [Br2, Br1, Br1, Br1, Br1]),
        ],
    }
}

const fn colonne(v: VueInfrastructure) -> usize {
    match v {
        VueInfrastructure::Directe => 0,
        VueInfrastructure::Partielle => 1,
        VueInfrastructure::MasqueePeuProtecteurs => 2,
        VueInfrastructure::MasqueeTresProtecteurs => 3,
        VueInfrastructure::Arriere => 4,
    }
}

/// Classe d'une baie exposée à une infrastructure de transports terrestres (chapitre V, partie
/// I, point 3, configurations 1 et 2).
///
/// Configuration 1 : au-delà de la distance maximale de prise en compte, BR1. Configuration 2 :
/// tableau de la catégorie, ligne de la distance, colonne de la vue. Une distance égale à la
/// borne d'une tranche appartient à la tranche qui s'arrête à cette borne (« 0-65 m » contient
/// 65 m, comme « distance supérieure à 700 m » exclut 700 m ; Q201).
// Th-BCE 2020 fiche 5.11 §5.11.3 et §5.11.7 ; annexe R. 172-4, chap. V, partie I, point 3.
pub fn classe_infrastructure(i: &InfrastructureBruit) -> ExpoBr {
    if i.distance_m > distance_maximale_m(i.categorie) {
        return Br1;
    }
    tableau(i.categorie)
        .iter()
        .find(|(borne, _)| i.distance_m <= *borne)
        .map_or(Br1, |(_, classes)| classes[colonne(i.vue)])
}

/// Classe d'une baie d'un bâtiment situé dans le plan d'exposition au bruit d'un aérodrome
/// (configuration 2, « toutes vues » ; fiche 5.11.3.1).
// Th-BCE 2020 fiche 5.11 §5.11.3.1 ; annexe R. 172-4, chap. V, partie I, point 3.2.
pub const fn classe_aerodrome(z: ZonePeb) -> ExpoBr {
    match z {
        ZonePeb::A | ZonePeb::B | ZonePeb::C => Br3,
        ZonePeb::D => Br2,
        ZonePeb::HorsZone => Br1,
    }
}

/// Classe d'une baie déterminée en détail : la plus défavorable de celles des infrastructures
/// et de l'aérodrome (« dans le cas de plusieurs infrastructures, on retiendra la classe
/// d'exposition au bruit la plus défavorable »). `None` si ni infrastructure ni zone du plan
/// d'exposition n'est saisie.
// Th-BCE 2020 fiche 5.11 §5.11.6.1 ; annexe R. 172-4, chap. V, partie I, point 3.2.
pub fn classe_baie_detaillee(b: &BruitBaie) -> Option<ExpoBr> {
    b.infrastructures
        .iter()
        .map(classe_infrastructure)
        .chain(b.zone_peb.map(classe_aerodrome))
        .max()
}

/// Toutes les baies d'un bâtiment.
fn baies_du_batiment(bat: &Batiment) -> impl Iterator<Item = &Baie> {
    bat.zones
        .iter()
        .flat_map(|z| z.groupes.iter())
        .flat_map(|g| g.baies.iter())
}

/// Classe d'une baie sans la règle de la façade : détermination détaillée, sinon saisie directe.
fn classe_propre(b: &Baie) -> Option<ExpoBr> {
    b.bruit
        .as_ref()
        .and_then(classe_baie_detaillee)
        .or(b.expo_br)
}

/// Classe d'exposition au bruit retenue pour la baie `b` du bâtiment `bat` : sa détermination
/// détaillée (`bruit.infrastructures`, `bruit.zone_peb`) si elle existe, sinon `expo_br` saisi ;
/// à défaut des deux, et si la baie désigne une façade, la classe la plus élevée des autres baies
/// de cette façade (« à défaut d'une détermination détaillée, la classe BR d'une baie d'une
/// façade est la classe la plus élevée des baies de cette façade »). `None` si rien ne la fixe.
// Th-BCE 2020 fiche 5.11 §5.11.4 ; annexe R. 172-4, chap. V, partie I, point 3.2 (dernier alinéa).
pub fn classe_baie(bat: &Batiment, b: &Baie) -> Option<ExpoBr> {
    if let Some(c) = classe_propre(b) {
        return Some(c);
    }
    let facade = b.bruit.as_ref()?.facade.as_ref()?;
    baies_du_batiment(bat)
        .filter(|a| a.bruit.as_ref().and_then(|x| x.facade.as_ref()) == Some(facade))
        .filter_map(classe_propre)
        .max()
}

/// Classe d'une partie de bâtiment, groupe ou zone, à partir des classes de ses baies (point
/// 4) : BR3 si toutes sont BR3 ; BR2 si toutes sont BR2 ou BR3 et l'une au moins BR2 ; BR1 dans
/// les autres cas. `None` si la partie n'a pas de baie ou si l'une d'elles n'a pas de classe
/// (la détermination est impossible : Q203).
// Th-BCE 2020 annexe R. 172-4, chap. V, partie I, point 4.
pub fn classe_partie<I: IntoIterator<Item = Option<ExpoBr>>>(classes: I) -> Option<ZoneBruit> {
    let mut n = 0_usize;
    let (mut n_br1, mut n_br2) = (0_usize, 0_usize);
    for c in classes {
        n += 1;
        match c? {
            Br1 => n_br1 += 1,
            Br2 => n_br2 += 1,
            Br3 => {}
        }
    }
    if n == 0 {
        None
    } else if n_br1 > 0 {
        Some(ZoneBruit::Br1)
    } else if n_br2 > 0 {
        Some(ZoneBruit::Br2)
    } else {
        Some(ZoneBruit::Br3)
    }
}

/// Classe d'un groupe déduite de ses baies (`None` si indéterminable).
pub fn classe_groupe(bat: &Batiment, g: &Groupe) -> Option<ZoneBruit> {
    classe_partie(g.baies.iter().map(|b| classe_baie(bat, b)))
}

/// Classe d'une zone déduite de ses baies (`None` si indéterminable).
pub fn classe_zone(bat: &Batiment, z: &Zone) -> Option<ZoneBruit> {
    classe_partie(
        z.groupes
            .iter()
            .flat_map(|g| g.baies.iter())
            .map(|b| classe_baie(bat, b)),
    )
}

/// Vrai si une baie du projet porte une détermination détaillée.
fn a_du_detail(projet: &Projet) -> bool {
    projet
        .batiments
        .iter()
        .any(|bat| baies_du_batiment(bat).any(|b| b.bruit.is_some()))
}

/// Copie du projet où la classe déduite des baies prend la place de la saisie absente : `expo_br`
/// de chaque baie qui a une détermination détaillée (ou une façade), et `zone_bruit` des zones
/// sans classe saisie (ni pour la zone, ni pour le bâtiment) dont la classe se déduit de toutes
/// les baies. Le calcul (`Cpr`, `Mbbruit`, catégories) lit ensuite ces champs sans changement.
/// `None` si aucune baie n'a de détermination détaillée : le projet est utilisé tel quel.
///
/// À appeler sur un projet validé (la cohérence des saisies directes et déduites y est
/// vérifiée).
pub fn projet_avec_classes_deduites(projet: &Projet) -> Option<Projet> {
    if !a_du_detail(projet) {
        return None;
    }
    let mut p = projet.clone();
    for (ib, bat) in projet.batiments.iter().enumerate() {
        let cible = &mut p.batiments[ib];
        for (iz, z) in bat.zones.iter().enumerate() {
            for (ig, g) in z.groupes.iter().enumerate() {
                for (i, b) in g.baies.iter().enumerate() {
                    if b.bruit.is_some() {
                        cible.zones[iz].groupes[ig].baies[i].expo_br = classe_baie(bat, b);
                    }
                }
            }
        }
        let Some(ex) = cible.exigences.as_mut() else {
            continue;
        };
        if ex.zone_bruit.is_some() {
            // Une classe du bâtiment tient lieu de saisie pour les zones qui n'ont pas la leur.
            continue;
        }
        for z in &bat.zones {
            let saisie = ex
                .zones
                .iter()
                .find(|e| e.zone == z.id)
                .and_then(|e| e.zone_bruit);
            let Some(deduite) = classe_zone(bat, z).filter(|_| saisie.is_none()) else {
                continue;
            };
            match ex.zones.iter_mut().find(|e| e.zone == z.id) {
                Some(e) => e.zone_bruit = Some(deduite),
                None => ex.zones.push(ExigencesZoneSaisie {
                    zone: z.id.clone(),
                    categorie_contraintes_exterieures: None,
                    zone_bruit: Some(deduite),
                    groupes: Vec::new(),
                }),
            }
        }
    }
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn infra(c: CategorieInfrastructure, d: f64, v: VueInfrastructure) -> ExpoBr {
        classe_infrastructure(&InfrastructureBruit {
            categorie: c,
            distance_m: d,
            vue: v,
        })
    }

    use CategorieInfrastructure::{Categorie1, Categorie2, Categorie3, Categorie4, Categorie5};
    use VueInfrastructure::{
        Arriere, Directe, MasqueePeuProtecteurs, MasqueeTresProtecteurs, Partielle,
    };

    /// Valeurs relues sur l'image des tableaux (p. 249 et 250 de l'annexe III), une cellule par
    /// catégorie et par colonne de vue, choisies hors des bornes.
    #[test]
    fn tableaux_relus_sur_l_image() {
        // Catégorie 1.
        assert_eq!(infra(Categorie1, 30.0, Directe), Br3);
        assert_eq!(infra(Categorie1, 100.0, Arriere), Br2);
        assert_eq!(infra(Categorie1, 100.0, MasqueeTresProtecteurs), Br3);
        assert_eq!(infra(Categorie1, 200.0, MasqueeTresProtecteurs), Br2);
        assert_eq!(infra(Categorie1, 300.0, Partielle), Br2);
        assert_eq!(infra(Categorie1, 300.0, Directe), Br3);
        assert_eq!(infra(Categorie1, 300.0, Arriere), Br1);
        assert_eq!(infra(Categorie1, 500.0, MasqueeTresProtecteurs), Br1);
        assert_eq!(infra(Categorie1, 500.0, MasqueePeuProtecteurs), Br2);
        assert_eq!(infra(Categorie1, 600.0, Directe), Br2);
        assert_eq!(infra(Categorie1, 600.0, Partielle), Br1);
        // Catégorie 2.
        assert_eq!(infra(Categorie2, 20.0, Arriere), Br3);
        assert_eq!(infra(Categorie2, 50.0, Arriere), Br2);
        assert_eq!(infra(Categorie2, 100.0, MasqueeTresProtecteurs), Br2);
        assert_eq!(infra(Categorie2, 100.0, MasqueePeuProtecteurs), Br3);
        assert_eq!(infra(Categorie2, 200.0, Partielle), Br2);
        assert_eq!(infra(Categorie2, 200.0, Arriere), Br1);
        assert_eq!(infra(Categorie2, 300.0, Directe), Br2);
        assert_eq!(infra(Categorie2, 300.0, MasqueeTresProtecteurs), Br1);
        assert_eq!(infra(Categorie2, 400.0, Directe), Br2);
        assert_eq!(infra(Categorie2, 400.0, Partielle), Br1);
        // Catégorie 3.
        assert_eq!(infra(Categorie3, 10.0, Arriere), Br2);
        assert_eq!(infra(Categorie3, 10.0, Directe), Br3);
        assert_eq!(infra(Categorie3, 40.0, MasqueeTresProtecteurs), Br2);
        assert_eq!(infra(Categorie3, 40.0, Partielle), Br3);
        assert_eq!(infra(Categorie3, 70.0, Partielle), Br2);
        assert_eq!(infra(Categorie3, 70.0, Arriere), Br1);
        assert_eq!(infra(Categorie3, 130.0, Directe), Br2);
        assert_eq!(infra(Categorie3, 130.0, MasqueeTresProtecteurs), Br1);
        assert_eq!(infra(Categorie3, 200.0, Directe), Br2);
        assert_eq!(infra(Categorie3, 200.0, Partielle), Br1);
        // Catégorie 4.
        assert_eq!(infra(Categorie4, 10.0, MasqueePeuProtecteurs), Br3);
        assert_eq!(infra(Categorie4, 10.0, MasqueeTresProtecteurs), Br2);
        assert_eq!(infra(Categorie4, 20.0, Directe), Br3);
        assert_eq!(infra(Categorie4, 20.0, Arriere), Br1);
        assert_eq!(infra(Categorie4, 40.0, Directe), Br2);
        assert_eq!(infra(Categorie4, 40.0, MasqueeTresProtecteurs), Br1);
        assert_eq!(infra(Categorie4, 80.0, Directe), Br2);
        assert_eq!(infra(Categorie4, 80.0, Partielle), Br1);
        // Catégorie 5.
        assert_eq!(infra(Categorie5, 5.0, Directe), Br3);
        assert_eq!(infra(Categorie5, 5.0, Partielle), Br2);
        assert_eq!(infra(Categorie5, 5.0, Arriere), Br1);
        assert_eq!(infra(Categorie5, 15.0, MasqueeTresProtecteurs), Br1);
        assert_eq!(infra(Categorie5, 15.0, MasqueePeuProtecteurs), Br2);
        assert_eq!(infra(Categorie5, 25.0, Directe), Br2);
        assert_eq!(infra(Categorie5, 25.0, Partielle), Br1);
    }

    /// Configuration 1 : au-delà de la distance maximale de prise en compte, BR1 ; sur la borne
    /// (« supérieure à ») la ligne du tableau s'applique encore (Q201).
    #[test]
    fn configuration_1_et_bornes() {
        for (c, dmax) in [
            (Categorie1, 700.0),
            (Categorie2, 500.0),
            (Categorie3, 250.0),
            (Categorie4, 100.0),
            (Categorie5, 30.0),
        ] {
            assert_eq!(infra(c, dmax + 0.1, Directe), Br1);
            assert_eq!(infra(c, dmax, Directe), Br2, "borne {dmax}");
        }
        // 65 m : tranche « 0-65 m » de la catégorie 1 (arrière : BR3, contre BR2 au-delà).
        assert_eq!(infra(Categorie1, 65.0, Arriere), Br3);
        assert_eq!(infra(Categorie1, 65.1, Arriere), Br2);
        assert_eq!(infra(Categorie1, 0.0, Directe), Br3);
    }

    #[test]
    fn aerodrome_et_plusieurs_infrastructures() {
        assert_eq!(classe_aerodrome(ZonePeb::A), Br3);
        assert_eq!(classe_aerodrome(ZonePeb::B), Br3);
        assert_eq!(classe_aerodrome(ZonePeb::C), Br3);
        assert_eq!(classe_aerodrome(ZonePeb::D), Br2);
        assert_eq!(classe_aerodrome(ZonePeb::HorsZone), Br1);
        // Catégorie 3 à 130 m en vue directe : BR2 ; catégorie 5 à 25 m en vue partielle : BR1 ;
        // zone D : BR2 ; la plus défavorable l'emporte.
        let b = BruitBaie {
            infrastructures: vec![
                InfrastructureBruit {
                    categorie: Categorie3,
                    distance_m: 130.0,
                    vue: Directe,
                },
                InfrastructureBruit {
                    categorie: Categorie5,
                    distance_m: 25.0,
                    vue: Partielle,
                },
            ],
            zone_peb: Some(ZonePeb::D),
            facade: None,
        };
        assert_eq!(classe_baie_detaillee(&b), Some(Br2));
        let b = BruitBaie {
            zone_peb: Some(ZonePeb::C),
            ..b
        };
        assert_eq!(classe_baie_detaillee(&b), Some(Br3));
        assert_eq!(classe_baie_detaillee(&BruitBaie::default()), None);
    }

    #[test]
    fn classe_d_une_partie_de_batiment() {
        let c = |v: &[ExpoBr]| classe_partie(v.iter().map(|x| Some(*x)));
        assert_eq!(c(&[Br3, Br3]), Some(ZoneBruit::Br3));
        assert_eq!(c(&[Br3, Br2, Br3]), Some(ZoneBruit::Br2));
        assert_eq!(c(&[Br2, Br2]), Some(ZoneBruit::Br2));
        assert_eq!(c(&[Br3, Br1]), Some(ZoneBruit::Br1));
        assert_eq!(c(&[Br2, Br1, Br3]), Some(ZoneBruit::Br1));
        assert_eq!(c(&[Br1]), Some(ZoneBruit::Br1));
        assert_eq!(c(&[]), None);
        assert_eq!(classe_partie([Some(Br3), None]), None);
    }
}
