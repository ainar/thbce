// SPDX-License-Identifier: GPL-3.0-or-later
//! Valeurs des exigences des usages 3 à 28 : annexe de l'article R. 172-4 du code de la
//! construction et de l'habitation, chapitre III
//! (`docs/reference/arrete/chapitre1a3_annexe_r172-4_post-rivaton-i.pdf`, version documentaire
//! du site RT-RE bâtiment pour les permis déposés à partir du 1er juillet 2026).
//!
//! Chaque tableau a été transcrit depuis l'image de sa page (rendue à 100 dpi) et recoupé avec
//! le texte de repérage `docs/reference/texte/annexe-r172-4-chapitres-1-a-3.txt` (Q65) :
//!
//! - I. `Bbio_maxmoyen` (p. 6) et coefficients `Mbgéo`, `Mbsurf_tot`, `Mbbruit` (p. 9 à 30) ;
//! - II. `Cep,nr_maxmoyen`, `Cep_maxmoyen` (p. 30 et 31), `Icénergie_maxmoyen` (p. 31 à 34) et
//!   coefficients `Mcgéo`, `Mcsurf_tot`, `Mccat` (p. 37 à 59) ;
//! - IV. `DH_maxcat` (p. 100 à 106).
//!
//! `Mbcombles`, `Mbsurf_moy`, `MbHSP`, `Mccombles`, `Mcsurf_moy` et `McHSP` valent 0 pour tous
//! ces usages. Les usages 1 et 2 restent dans [`super::exigences`].

/// Tableau d'un coefficient selon l'altitude (lignes « < 400m », « 400m-800m », « >800m ») ou
/// selon une classe (lignes de `Mbbruit` et `Mccat`), et la zone climatique (colonnes H1a, H1b,
/// H1c, H2a, H2b, H2c, H2d, H3).
pub(crate) type Grille = [[f64; 8]; 3];

/// Tableau nul.
const NUL: Grille = [[0.0; 8]; 3];

/// Tableau dont seule la dernière ligne (catégorie 3) est non nulle et vaut `x` partout.
const fn cat3(x: f64) -> Grille {
    [[0.0; 8], [0.0; 8], [x; 8]]
}

/// Tableau dont seule la dernière ligne (catégorie 3) est non nulle.
const fn cat3_par_zone(l: [f64; 8]) -> Grille {
    [[0.0; 8], [0.0; 8], l]
}

/// Morceau d'une modulation selon la surface : `(a + b × Sref) / valeur moyenne` pour
/// `Sref ≤ s_max` (bornes supérieures incluses, comme dans tous les tableaux de l'annexe).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Morceau {
    /// Borne supérieure de `Sref`, en m² (`f64::INFINITY` pour le dernier morceau).
    pub s_max: f64,
    /// Terme constant du numérateur.
    pub a: f64,
    /// Coefficient de `Sref` au numérateur.
    pub b: f64,
}

const fn m(s_max: f64, a: f64, b: f64) -> Morceau {
    Morceau { s_max, a, b }
}

const INF: f64 = f64::INFINITY;

/// Modulation `Mbsurf_tot` ou `Mcsurf_tot`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum SurfTot {
    /// « = 0 ».
    Nulle,
    /// Une seule fonction de `Sref`.
    Morceaux(&'static [Morceau]),
    /// Selon l'année de dépôt du permis : 2022 à 2024, 2025 à 2027, à partir de 2028.
    ParPeriode([&'static [Morceau]; 3]),
    /// Industries ou artisanats de 8 h à 18 h (`Mcsurf_tot`, p. 56) : réseau de chaleur urbain
    /// classé, permis déposé jusqu'en 2027 (Q65) ; autres cas.
    SelonReseauClasse {
        /// Raccordé à un réseau de chaleur urbain classé, permis déposé jusqu'en 2027.
        classe_jusqu_en_2027: &'static [Morceau],
        /// Réseau classé à partir de 2028, réseau non classé, autres cas.
        autres: &'static [Morceau],
    },
}

/// Valeurs des exigences d'un usage (3 à 28).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DonneesUsage {
    /// Numéro d'usage.
    pub usage: u8,
    /// « Bbio_maxmoyen », en points.
    pub bbio_max_moyen: f64,
    /// « Mbgéo ».
    pub mb_geo: Grille,
    /// « Mbsurf_tot ».
    pub mb_surf_tot: SurfTot,
    /// « Mbbruit » : lignes BR1, BR2/BR3, catégorie 3 (Q66 : la catégorie 3 l'emporte).
    pub mb_bruit: Grille,
    /// « Cep,nr_maxmoyen », en kWhep/(m².an).
    pub cep_nr_max_moyen: f64,
    /// « Cep_maxmoyen », en kWhep/(m².an).
    pub cep_max_moyen: f64,
    /// « Icénergie_maxmoyen » des parties raccordées à un réseau de chaleur urbain, en kg éq.
    /// CO2/m², par période (2022 à 2024, 2025 à 2027, à partir de 2028) ; `None` pour les
    /// industries, sans ligne « raccordés ».
    pub ic_rcu: Option<[f64; 3]>,
    /// « Icénergie_maxmoyen » des autres cas ; `None` pour « - » (usage non soumis à la RE2020
    /// pour les permis de cette période, Q68).
    pub ic_autres: [Option<f64>; 3],
    /// « Mcgéo ».
    pub mc_geo: Grille,
    /// « Mcsurf_tot ».
    pub mc_surf_tot: SurfTot,
    /// « Mccat » : lignes catégorie 1, 2, 3.
    pub mc_cat: Grille,
    /// « DH_maxcat », en °C.h : catégorie 1 sauf parties climatisées en H2d et H3, catégorie 1
    /// climatisée en H2d et H3, catégorie 2.
    pub dh_max: [f64; 3],
    /// Le tableau `DH_maxcat` a une colonne « Catégorie 3 » (« Pas de seuil ») : usage cité
    /// par le chapitre V, partie II, pour les locaux de catégorie 3.
    pub categorie_3: bool,
}

const fn autres(x: f64) -> [Option<f64>; 3] {
    [None, Some(x), Some(x)]
}

/// Morceaux d'une modulation nulle au-delà de `s` et égale à `(a + b·Sref)/moyen` en deçà.
macro_rules! jusqu_a {
    ($s:expr, $a:expr, $b:expr) => {
        &[m($s, $a, $b), m(INF, 0.0, 0.0)]
    };
}

// Bbio des usages 4 et 5 (p. 10 et 11) : même Mbgéo, Mbsurf_tot propre à chacun.
const MB_GEO_ENSEIGNEMENT: Grille = [
    [0.10, 0.20, 0.25, -0.10, 0.0, 0.05, 0.50, 0.50],
    [0.25, 0.30, 0.25, 0.05, 0.10, 0.0, 0.35, 0.25],
    [0.45, 0.45, 0.40, 0.30, 0.35, 0.20, 0.30, 0.20],
];
const MC_GEO_ENSEIGNEMENT: Grille = [
    [0.05, 0.15, 0.10, -0.05, 0.0, -0.05, 0.40, 0.30],
    [0.30, 0.30, 0.30, 0.15, 0.20, 0.10, 0.30, 0.10],
    [0.60, 0.60, 0.60, 0.45, 0.50, 0.35, 0.35, 0.15],
];
// Mccat de l'enseignement (p. 38) : « Cat 1 : 0 ; Cat 2 : 0,05 ».
const MC_CAT_ENSEIGNEMENT: Grille = [[0.0; 8], [0.05; 8], [0.0; 8]];
// Usages 25 et 28 (p. 28) : même Bbio.
const MB_GEO_SPORT: Grille = [
    [0.0, 0.1, 0.25, -0.15, 0.0, 0.1, 0.55, 0.55],
    [0.0, 0.05, 0.15, -0.15, -0.05, -0.05, 0.4, 0.3],
    [0.05, 0.1, 0.15, -0.05, 0.0, -0.05, 0.25, 0.15],
];

/// Données des usages 3 à 28, dans l'ordre des numéros.
pub(crate) static USAGES: [DonneesUsage; 26] = [
    // 3. Bureaux (Bbio p. 9 et 10 ; Cep p. 37 ; DH p. 101).
    DonneesUsage {
        usage: 3,
        bbio_max_moyen: 95.0,
        mb_geo: [
            [0.05, 0.10, 0.20, -0.05, 0.0, 0.10, 0.30, 0.25],
            [0.25, 0.25, 0.20, 0.20, 0.20, 0.10, 0.10, -0.05],
            [0.45, 0.45, 0.40, 0.40, 0.35, 0.25, 0.30, 0.10],
        ],
        mb_surf_tot: SurfTot::ParPeriode([
            &[
                m(500.0, 24.0, -0.06),
                m(4000.0, -5.55, -0.0009),
                m(10000.0, -5.55, -0.0009),
                m(INF, -14.55, 0.0),
            ],
            &[
                m(500.0, 24.0, -0.06),
                m(4000.0, -4.9, -0.0022),
                m(10000.0, -9.7, -0.001),
                m(INF, -19.7, 0.0),
            ],
            &[
                m(500.0, 24.0, -0.06),
                m(4000.0, -3.8, -0.0044),
                m(10000.0, -21.4, 0.0),
                m(INF, -21.4, 0.0),
            ],
        ]),
        mb_bruit: cat3(0.4),
        cep_nr_max_moyen: 75.0,
        cep_max_moyen: 85.0,
        ic_rcu: Some([280.0, 200.0, 200.0]),
        ic_autres: [Some(200.0), Some(200.0), Some(200.0)],
        mc_geo: [
            [0.05, 0.10, 0.10, 0.0, 0.0, 0.0, 0.15, 0.15],
            [0.20, 0.25, 0.20, 0.15, 0.15, 0.05, 0.10, -0.05],
            [0.35, 0.40, 0.35, 0.35, 0.30, 0.20, 0.25, 0.10],
        ],
        mc_surf_tot: SurfTot::Morceaux(&[
            m(500.0, 18.0, -0.032),
            m(1500.0, 6.0, -0.008),
            m(INF, -6.0, 0.0),
        ]),
        mc_cat: NUL,
        dh_max: [1150.0, 2400.0, 2600.0],
        categorie_3: true,
    },
    // 4. Enseignement primaire (Bbio p. 10 et 11 ; Cep p. 38 ; DH p. 101).
    DonneesUsage {
        usage: 4,
        bbio_max_moyen: 68.0,
        mb_geo: MB_GEO_ENSEIGNEMENT,
        mb_surf_tot: SurfTot::Morceaux(&[
            m(500.0, 35.0, -0.05),
            m(1000.0, 20.0, -0.02),
            m(INF, 0.0, 0.0),
        ]),
        mb_bruit: NUL,
        cep_nr_max_moyen: 65.0,
        cep_max_moyen: 72.0,
        ic_rcu: Some([240.0, 200.0, 140.0]),
        ic_autres: [Some(240.0), Some(140.0), Some(140.0)],
        mc_geo: MC_GEO_ENSEIGNEMENT,
        mc_surf_tot: SurfTot::Morceaux(jusqu_a!(500.0, 12.5, -0.025)),
        mc_cat: MC_CAT_ENSEIGNEMENT,
        dh_max: [900.0, 1800.0, 2200.0],
        categorie_3: false,
    },
    // 5. Enseignement secondaire (Bbio p. 10 et 11 ; Cep p. 38 ; DH p. 101).
    DonneesUsage {
        usage: 5,
        bbio_max_moyen: 68.0,
        mb_geo: MB_GEO_ENSEIGNEMENT,
        mb_surf_tot: SurfTot::Morceaux(&[
            m(500.0, 45.0, -0.045),
            m(1000.0, 45.0, -0.045),
            m(INF, 0.0, 0.0),
        ]),
        mb_bruit: NUL,
        cep_nr_max_moyen: 63.0,
        cep_max_moyen: 72.0,
        ic_rcu: Some([240.0, 200.0, 140.0]),
        ic_autres: [Some(240.0), Some(140.0), Some(140.0)],
        mc_geo: MC_GEO_ENSEIGNEMENT,
        mc_surf_tot: SurfTot::Morceaux(jusqu_a!(500.0, 12.5, -0.025)),
        mc_cat: MC_CAT_ENSEIGNEMENT,
        dh_max: [900.0, 1800.0, 2200.0],
        categorie_3: false,
    },
    // 6. Médiathèques et bibliothèques (Bbio p. 11 et 12 ; Cep p. 39 ; DH p. 101).
    DonneesUsage {
        usage: 6,
        bbio_max_moyen: 117.0,
        mb_geo: [
            [0.05, 0.2, 0.25, -0.1, 0.0, 0.0, 0.3, 0.2],
            [0.2, 0.3, 0.3, 0.0, 0.1, 0.0, 0.25, 0.15],
            [0.4, 0.5, 0.40, 0.15, 0.3, 0.1, 0.25, 0.15],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: cat3_par_zone([0.15, 0.1, 0.1, 0.15, 0.15, 0.2, 0.1, 0.15]),
        cep_nr_max_moyen: 93.0,
        cep_max_moyen: 105.0,
        ic_rcu: Some([360.0, 285.0, 285.0]),
        ic_autres: autres(285.0),
        mc_geo: [
            [0.1, 0.15, 0.1, -0.05, 0.0, -0.05, 0.15, 0.05],
            [0.25, 0.3, 0.2, 0.1, 0.15, 0.05, 0.1, 0.0],
            [0.5, 0.45, 0.4, 0.35, 0.35, 0.2, 0.2, 0.05],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: cat3_par_zone([0.05, 0.05, 0.05, 0.05, 0.05, 0.1, 0.2, 0.3]),
        dh_max: [900.0, 2200.0, 2400.0],
        categorie_3: true,
    },
    // 7. Bâtiments universitaires d'enseignement et de recherche, enseignements atypiques
    // (Bbio p. 12 et 13 ; Cep p. 40 ; DH p. 101).
    DonneesUsage {
        usage: 7,
        bbio_max_moyen: 122.0,
        mb_geo: [
            [0.1, 0.2, 0.2, -0.05, 0.0, 0.0, 0.2, 0.2],
            [0.3, 0.35, 0.35, 0.1, 0.2, 0.05, 0.25, 0.15],
            [0.5, 0.6, 0.5, 0.3, 0.4, 0.2, 0.25, 0.15],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: cat3_par_zone([0.1, 0.05, 0.1, 0.1, 0.15, 0.2, 0.1, 0.15]),
        cep_nr_max_moyen: 102.0,
        cep_max_moyen: 112.0,
        ic_rcu: Some([225.0, 190.0, 190.0]),
        ic_autres: autres(190.0),
        mc_geo: [
            [0.05, 0.10, 0.10, -0.05, 0.0, 0.0, 0.2, 0.15],
            [0.1, 0.15, 0.15, 0.0, 0.05, 0.0, 0.1, 0.05],
            [0.2, 0.2, 0.2, 0.1, 0.1, 0.1, 0.1, 0.0],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: cat3_par_zone([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.05, 0.05]),
        dh_max: [900.0, 2200.0, 2400.0],
        categorie_3: true,
    },
    // 8. Hôtels 0, 1 et 2 étoiles, partie nuit (Bbio p. 13 et 14 ; Cep p. 41 ; DH p. 101 et
    // 102).
    DonneesUsage {
        usage: 8,
        bbio_max_moyen: 76.0,
        mb_geo: [
            [0.15, 0.2, 0.2, 0.0, 0.0, -0.1, 0.0, -0.15],
            [0.45, 0.45, 0.4, 0.25, 0.3, 0.15, 0.15, -0.05],
            [0.75, 0.8, 0.7, 0.6, 0.6, 0.4, 0.4, 0.15],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: cat3(0.05),
        cep_nr_max_moyen: 121.0,
        cep_max_moyen: 144.0,
        ic_rcu: Some([490.0, 390.0, 390.0]),
        ic_autres: autres(390.0),
        mc_geo: [
            [0.1, 0.1, 0.1, 0.0, 0.0, 0.0, 0.05, 0.0],
            [0.2, 0.25, 0.2, 0.1, 0.15, 0.1, 0.15, 0.05],
            [0.35, 0.35, 0.35, 0.25, 0.25, 0.2, 0.25, 0.1],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: NUL,
        dh_max: [300.0, 700.0, 1000.0],
        categorie_3: true,
    },
    // 9. Hôtels 3, 4 et 5 étoiles, partie nuit (Bbio p. 14 ; Cep p. 41 et 42 ; DH p. 101 et
    // 102).
    DonneesUsage {
        usage: 9,
        bbio_max_moyen: 76.0,
        mb_geo: [
            [0.15, 0.2, 0.2, 0.0, 0.0, -0.1, 0.0, -0.15],
            [0.45, 0.45, 0.4, 0.25, 0.3, 0.15, 0.15, -0.05],
            [0.75, 0.75, 0.65, 0.6, 0.6, 0.4, 0.4, 0.15],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: cat3(0.05),
        cep_nr_max_moyen: 118.0,
        cep_max_moyen: 138.0,
        ic_rcu: Some([485.0, 350.0, 350.0]),
        ic_autres: autres(350.0),
        mc_geo: [
            [0.1, 0.1, 0.1, 0.0, 0.0, 0.0, 0.05, 0.0],
            [0.2, 0.25, 0.2, 0.1, 0.15, 0.1, 0.15, 0.05],
            [0.35, 0.35, 0.35, 0.25, 0.25, 0.2, 0.25, 0.1],
        ],
        mc_surf_tot: SurfTot::Morceaux(jusqu_a!(1000.0, 81.0, -0.081)),
        mc_cat: NUL,
        dh_max: [300.0, 700.0, 1000.0],
        categorie_3: true,
    },
    // 10. Hôtels 0, 1 et 2 étoiles, partie jour (Bbio p. 15 ; Cep p. 42 et 43 ; DH p. 102).
    DonneesUsage {
        usage: 10,
        bbio_max_moyen: 134.0,
        mb_geo: [
            [0.1, 0.15, 0.25, -0.1, 0.0, 0.0, 0.35, 0.25],
            [0.25, 0.3, 0.35, 0.05, 0.15, 0.05, 0.3, 0.15],
            [0.45, 0.55, 0.5, 0.3, 0.35, 0.2, 0.35, 0.15],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: cat3(0.3),
        cep_nr_max_moyen: 235.0,
        cep_max_moyen: 252.0,
        ic_rcu: Some([595.0, 495.0, 495.0]),
        ic_autres: autres(495.0),
        mc_geo: [
            [0.05, 0.1, 0.1, -0.05, 0.0, 0.05, 0.15, 0.1],
            [0.1, 0.15, 0.15, 0.0, 0.05, 0.05, 0.15, 0.1],
            [0.2, 0.2, 0.2, 0.1, 0.15, 0.1, 0.15, 0.05],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: NUL,
        dh_max: [1300.0, 3300.0, 3400.0],
        categorie_3: true,
    },
    // 11. Hôtels 3, 4 et 5 étoiles, partie jour (Bbio p. 15 et 16 ; Cep p. 43 ; DH p. 102).
    DonneesUsage {
        usage: 11,
        bbio_max_moyen: 163.0,
        mb_geo: [
            [0.05, 0.15, 0.2, -0.1, 0.0, -0.05, 0.25, 0.1],
            [0.25, 0.3, 0.3, 0.1, 0.15, 0.05, 0.25, 0.1],
            [0.45, 0.5, 0.45, 0.3, 0.35, 0.20, 0.3, 0.15],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: cat3(0.3),
        cep_nr_max_moyen: 234.0,
        cep_max_moyen: 281.0,
        ic_rcu: Some([630.0, 520.0, 520.0]),
        ic_autres: autres(520.0),
        mc_geo: [
            [0.05, 0.1, 0.1, 0.0, 0.0, 0.0, 0.1, 0.1],
            [0.1, 0.15, 0.15, 0.05, 0.05, 0.05, 0.1, 0.05],
            [0.2, 0.25, 0.2, 0.15, 0.15, 0.1, 0.15, 0.05],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: NUL,
        dh_max: [1300.0, 3300.0, 3400.0],
        categorie_3: true,
    },
    // 12. Établissements d'accueil de la petite enfance (Bbio p. 16 et 17 ; Cep p. 44 et 45 ;
    // DH p. 102). Mbgéo H2b < 400 m imprimé « 0, » : lu 0 (Q65).
    DonneesUsage {
        usage: 12,
        bbio_max_moyen: 139.0,
        mb_geo: [
            [0.1, 0.15, 0.1, 0.0, 0.0, -0.1, 0.1, 0.0],
            [0.25, 0.3, 0.25, 0.15, 0.2, 0.05, 0.1, 0.05],
            [0.45, 0.5, 0.4, 0.4, 0.4, 0.25, 0.25, 0.15],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: cat3_par_zone([0.05, 0.1, 0.15, 0.1, 0.15, 0.2, 0.25, 0.3]),
        cep_nr_max_moyen: 150.0,
        cep_max_moyen: 182.0,
        ic_rcu: Some([895.0, 680.0, 680.0]),
        ic_autres: autres(680.0),
        mc_geo: [
            [0.1, 0.1, 0.1, 0.0, 0.0, -0.05, 0.05, -0.05],
            [0.25, 0.25, 0.25, 0.2, 0.20, 0.1, 0.1, 0.0],
            [0.45, 0.4, 0.4, 0.35, 0.35, 0.25, 0.25, 0.15],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: cat3_par_zone([0.0, 0.05, 0.05, 0.0, 0.05, 0.05, 0.2, 0.25]),
        dh_max: [550.0, 1600.0, 1600.0],
        categorie_3: true,
    },
    // 13. Restaurants en continu, 18 h par jour, 7 jours sur 7 (Bbio p. 17 et 18 ; Cep p. 45
    // et 46 ; DH p. 102).
    DonneesUsage {
        usage: 13,
        bbio_max_moyen: 245.0,
        mb_geo: [
            [0.05, 0.1, 0.2, -0.05, 0.0, 0.05, 0.25, 0.15],
            [0.15, 0.2, 0.25, 0.05, 0.1, 0.05, 0.25, 0.15],
            [0.3, 0.35, 0.35, 0.2, 0.25, 0.15, 0.30, 0.15],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: NUL,
        cep_nr_max_moyen: 282.0,
        cep_max_moyen: 578.0,
        ic_rcu: Some([670.0, 570.0, 570.0]),
        ic_autres: autres(570.0),
        mc_geo: [
            [0.0, 0.05, 0.1, -0.05, 0.0, 0.05, 0.2, 0.15],
            [0.1, 0.1, 0.1, 0.0, 0.05, 0.05, 0.15, 0.05],
            [0.2, 0.2, 0.15, 0.10, 0.15, 0.10, 0.15, 0.05],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: NUL,
        dh_max: [2500.0, 5000.0, 5000.0],
        categorie_3: true,
    },
    // 14. Restaurants, 1 repas par jour, 5 jours sur 7 (Bbio p. 18 et 19 ; Cep p. 46 ; DH
    // p. 102 et 103).
    DonneesUsage {
        usage: 14,
        bbio_max_moyen: 100.0,
        mb_geo: [
            [0.1, 0.15, 0.2, 0.0, 0.0, -0.05, 0.2, 0.1],
            [0.3, 0.35, 0.3, 0.2, 0.2, 0.05, 0.2, 0.15],
            [0.55, 0.55, 0.5, 0.45, 0.45, 0.25, 0.3, 0.2],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: NUL,
        cep_nr_max_moyen: 132.0,
        cep_max_moyen: 275.0,
        ic_rcu: Some([605.0, 470.0, 470.0]),
        ic_autres: autres(470.0),
        mc_geo: [
            [0.1, 0.1, 0.05, 0.0, 0.0, -0.05, 0.05, -0.05],
            [0.25, 0.25, 0.2, 0.15, 0.2, 0.05, 0.1, 0.0],
            [0.4, 0.4, 0.35, 0.35, 0.35, 0.2, 0.2, 0.05],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: NUL,
        dh_max: [250.0, 650.0, 650.0],
        categorie_3: true,
    },
    // 15. Restaurants, 2 repas par jour, 7 jours sur 7 (Bbio p. 19 et 20 ; Cep p. 47 ; DH
    // p. 103).
    DonneesUsage {
        usage: 15,
        bbio_max_moyen: 206.0,
        mb_geo: [
            [0.05, 0.1, 0.15, -0.05, 0.0, 0.0, 0.15, 0.1],
            [0.20, 0.25, 0.25, 0.1, 0.15, 0.05, 0.2, 0.1],
            [0.35, 0.4, 0.35, 0.25, 0.3, 0.15, 0.25, 0.15],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: NUL,
        cep_nr_max_moyen: 219.0,
        cep_max_moyen: 446.0,
        ic_rcu: Some([705.0, 570.0, 570.0]),
        ic_autres: autres(570.0),
        mc_geo: [
            [0.05, 0.1, 0.05, -0.05, 0.0, 0.0, 0.15, 0.1],
            [0.1, 0.1, 0.1, 0.05, 0.05, 0.05, 0.1, 0.05],
            [0.15, 0.2, 0.15, 0.1, 0.15, 0.05, 0.1, 0.05],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: NUL,
        dh_max: [1600.0, 3500.0, 3500.0],
        categorie_3: true,
    },
    // 16. Restaurants, 2 repas par jour, 6 jours sur 7 (Bbio p. 20 et 21 ; Cep p. 48 ; DH
    // p. 103).
    DonneesUsage {
        usage: 16,
        bbio_max_moyen: 177.0,
        mb_geo: [
            [0.05, 0.15, 0.15, -0.05, 0.0, 0.0, 0.2, 0.1],
            [0.2, 0.25, 0.25, 0.05, 0.15, 0.05, 0.2, 0.15],
            [0.35, 0.4, 0.35, 0.25, 0.3, 0.15, 0.3, 0.2],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: NUL,
        cep_nr_max_moyen: 214.0,
        cep_max_moyen: 412.0,
        ic_rcu: Some([675.0, 545.0, 545.0]),
        ic_autres: autres(545.0),
        mc_geo: [
            [0.05, 0.1, 0.1, -0.05, 0.0, 0.0, 0.1, 0.05],
            [0.15, 0.2, 0.15, 0.1, 0.1, 0.05, 0.1, 0.0],
            [0.3, 0.3, 0.25, 0.2, 0.25, 0.15, 0.15, 0.05],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: NUL,
        dh_max: [1250.0, 2500.0, 2500.0],
        categorie_3: true,
    },
    // 17. Commerces (Bbio p. 21 et 22 ; Cep p. 49 ; DH p. 103). Mccat : en-têtes « BR1 »,
    // « BR2/BR3 », « Cat 3 » lus catégories 1, 2 et 3 (Q65).
    DonneesUsage {
        usage: 17,
        bbio_max_moyen: 170.0,
        mb_geo: [
            [0.05, 0.1, 0.15, -0.05, 0.0, 0.05, 0.3, 0.25],
            [0.1, 0.2, 0.2, 0.0, 0.1, 0.0, 0.25, 0.2],
            [0.2, 0.3, 0.25, 0.15, 0.2, 0.1, 0.25, 0.2],
        ],
        mb_surf_tot: SurfTot::Morceaux(jusqu_a!(500.0, 47.5, -0.095)),
        mb_bruit: cat3(0.2),
        cep_nr_max_moyen: 163.0,
        cep_max_moyen: 182.0,
        ic_rcu: Some([315.0, 315.0, 315.0]),
        ic_autres: autres(315.0),
        mc_geo: [
            [0.0, 0.05, 0.1, -0.05, 0.0, 0.05, 0.2, 0.2],
            [0.0, 0.1, 0.1, -0.05, 0.05, 0.05, 0.15, 0.15],
            [0.05, 0.1, 0.1, 0.0, 0.05, 0.05, 0.1, 0.1],
        ],
        mc_surf_tot: SurfTot::Morceaux(jusqu_a!(500.0, 113.0, -0.226)),
        mc_cat: cat3(0.05),
        dh_max: [3300.0, 8000.0, 9500.0],
        categorie_3: true,
    },
    // 18. Vestiaires seuls (Bbio p. 22 ; Cep p. 50 ; DH p. 103).
    DonneesUsage {
        usage: 18,
        bbio_max_moyen: 225.0,
        mb_geo: [
            [0.05, 0.1, 0.1, -0.05, 0.0, -0.1, 0.0, -0.1],
            [0.3, 0.3, 0.25, 0.2, 0.25, 0.05, 0.1, 0.0],
            [0.55, 0.55, 0.5, 0.45, 0.5, 0.3, 0.25, 0.15],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: NUL,
        cep_nr_max_moyen: 242.0,
        cep_max_moyen: 306.0,
        ic_rcu: Some([1460.0, 1120.0, 1120.0]),
        ic_autres: autres(585.0),
        mc_geo: [
            [0.05, 0.1, 0.05, 0.0, 0.0, -0.05, 0.0, -0.05],
            [0.15, 0.15, 0.15, 0.1, 0.15, 0.05, 0.05, -0.05],
            [0.3, 0.3, 0.3, 0.25, 0.25, 0.15, 0.15, 0.05],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: NUL,
        dh_max: [1000.0, 2200.0, 2200.0],
        categorie_3: false,
    },
    // 19. Établissements sanitaires avec hébergement (Bbio p. 23 ; Cep p. 51 ; DH p. 104).
    DonneesUsage {
        usage: 19,
        bbio_max_moyen: 174.0,
        mb_geo: [
            [0.1, 0.15, 0.15, -0.05, 0.0, -0.05, 0.05, -0.05],
            [0.25, 0.3, 0.25, 0.15, 0.15, 0.1, 0.1, -0.05],
            [0.45, 0.5, 0.45, 0.3, 0.35, 0.25, 0.2, 0.05],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: NUL,
        cep_nr_max_moyen: 190.0,
        cep_max_moyen: 252.0,
        ic_rcu: Some([1155.0, 890.0, 890.0]),
        ic_autres: autres(330.0),
        mc_geo: [
            [0.1, 0.1, 0.05, 0.0, 0.0, -0.05, 0.0, -0.05],
            [0.25, 0.25, 0.2, 0.15, 0.15, 0.1, 0.1, -0.05],
            [0.4, 0.4, 0.35, 0.3, 0.35, 0.2, 0.25, 0.1],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: NUL,
        dh_max: [900.0, 2400.0, 2600.0],
        categorie_3: false,
    },
    // 20. Établissements de santé, partie nuit (Bbio p. 23 et 24 ; Cep p. 51 et 52 ; DH
    // p. 104).
    DonneesUsage {
        usage: 20,
        bbio_max_moyen: 164.0,
        mb_geo: [
            [0.05, 0.15, 0.2, -0.05, 0.0, 0.0, 0.2, 0.1],
            [0.25, 0.3, 0.3, 0.15, 0.2, 0.1, 0.25, 0.1],
            [0.45, 0.5, 0.45, 0.35, 0.4, 0.25, 0.35, 0.2],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: NUL,
        cep_nr_max_moyen: 274.0,
        cep_max_moyen: 302.0,
        ic_rcu: Some([575.0, 490.0, 490.0]),
        ic_autres: autres(320.0),
        mc_geo: [
            [0.05, 0.05, 0.1, 0.0, 0.0, 0.05, 0.1, 0.1],
            [0.05, 0.1, 0.1, 0.05, 0.05, 0.05, 0.1, 0.05],
            [0.1, 0.15, 0.15, 0.1, 0.1, 0.1, 0.1, 0.05],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: NUL,
        dh_max: [900.0, 2400.0, 2600.0],
        categorie_3: false,
    },
    // 21. Établissements de santé, partie jour (Bbio p. 24 et 25 ; Cep p. 52 et 53 ; DH
    // p. 104). Mbsurf_tot et Mcsurf_tot discontinus à 2 000 et 5 000 m², transcrits tels
    // quels (Q65).
    DonneesUsage {
        usage: 21,
        bbio_max_moyen: 133.0,
        mb_geo: [
            [0.05, 0.15, 0.2, -0.05, 0.0, 0.0, 0.25, 0.2],
            [0.15, 0.2, 0.2, 0.05, 0.05, 0.0, 0.2, 0.1],
            [0.25, 0.3, 0.25, 0.15, 0.2, 0.1, 0.15, 0.1],
        ],
        mb_surf_tot: SurfTot::Morceaux(jusqu_a!(2000.0, 22.0, -0.008)),
        mb_bruit: NUL,
        cep_nr_max_moyen: 165.0,
        cep_max_moyen: 180.0,
        ic_rcu: Some([615.0, 365.0, 365.0]),
        ic_autres: autres(230.0),
        mc_geo: [
            [0.05, 0.1, 0.1, -0.05, 0.0, -0.05, 0.1, 0.1],
            [0.15, 0.15, 0.15, 0.1, 0.1, 0.05, 0.15, 0.1],
            [0.3, 0.35, 0.3, 0.25, 0.25, 0.15, 0.2, 0.2],
        ],
        mc_surf_tot: SurfTot::Morceaux(&[
            m(2000.0, 0.0, 0.0),
            m(5000.0, 49.0, -0.026),
            m(INF, -78.0, 0.0),
        ]),
        mc_cat: NUL,
        dh_max: [1250.0, 3000.0, 3300.0],
        categorie_3: true,
    },
    // 22. Aérogares (Bbio p. 25 et 26 ; Cep p. 53 et 54 ; DH p. 104).
    DonneesUsage {
        usage: 22,
        bbio_max_moyen: 248.0,
        mb_geo: [
            [0.05, 0.1, 0.15, -0.05, 0.0, 0.05, 0.2, 0.2],
            [0.1, 0.15, 0.2, 0.0, 0.1, 0.05, 0.2, 0.15],
            [0.05, 0.15, -0.05, 0.0, 0.1, 0.25, 0.25, 0.05],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: NUL,
        cep_nr_max_moyen: 191.0,
        cep_max_moyen: 253.0,
        ic_rcu: Some([290.0, 260.0, 260.0]),
        ic_autres: autres(260.0),
        mc_geo: [
            [-0.05, 0.0, 0.05, -0.05, 0.0, 0.0, 0.1, 0.1],
            [-0.05, 0.0, 0.05, -0.1, 0.1, 0.2, 0.25, 0.0],
            [0.05, 0.1, -0.05, 0.0, 0.05, 0.15, 0.15, 0.0],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: NUL,
        dh_max: [12100.0, 21500.0, 21500.0],
        categorie_3: true,
    },
    // 23. Industries ou artisanats 3x8h (Bbio p. 26 et 27 ; Cep p. 54 et 55 ; DH p. 105).
    DonneesUsage {
        usage: 23,
        bbio_max_moyen: 257.0,
        mb_geo: [
            [0.05, 0.05, 0.1, -0.05, 0.0, 0.05, 0.25, 0.25],
            [0.05, 0.1, 0.1, 0.0, 0.05, 0.05, 0.2, 0.15],
            [0.1, 0.15, 0.15, 0.05, 0.1, 0.05, 0.2, 0.1],
        ],
        mb_surf_tot: SurfTot::Morceaux(jusqu_a!(5000.0, 50.0, -0.01)),
        mb_bruit: NUL,
        cep_nr_max_moyen: 290.0,
        cep_max_moyen: 365.0,
        ic_rcu: None,
        ic_autres: autres(315.0),
        mc_geo: [
            [0.05, 0.05, 0.1, -0.05, 0.0, 0.05, 0.15, 0.15],
            [0.05, 0.05, 0.1, 0.0, 0.05, 0.05, 0.1, 0.1],
            [0.1, 0.15, 0.15, 0.05, 0.1, 0.05, 0.1, 0.05],
        ],
        mc_surf_tot: SurfTot::Morceaux(jusqu_a!(5000.0, 15.0, -0.003)),
        mc_cat: NUL,
        dh_max: [3200.0, 8000.0, 8000.0],
        categorie_3: false,
    },
    // 24. Industries ou artisanats 8 h à 18 h (Bbio p. 27 ; Cep p. 55 et 56 ; DH p. 105).
    DonneesUsage {
        usage: 24,
        bbio_max_moyen: 69.0,
        mb_geo: [
            [0.1, 0.15, 0.25, -0.05, 0.0, 0.05, 0.4, 0.35],
            [0.2, 0.25, 0.3, 0.05, 0.1, 0.05, 0.4, 0.3],
            [0.35, 0.4, 0.45, 0.2, 0.25, 0.15, 0.35, 0.25],
        ],
        mb_surf_tot: SurfTot::Morceaux(jusqu_a!(5000.0, 65.0, -0.013)),
        mb_bruit: NUL,
        cep_nr_max_moyen: 94.0,
        cep_max_moyen: 116.0,
        ic_rcu: None,
        ic_autres: autres(110.0),
        mc_geo: [
            [0.05, 0.1, 0.1, -0.05, 0.0, 0.0, 0.2, 0.15],
            [0.15, 0.15, 0.2, 0.05, 0.1, 0.05, 0.15, 0.1],
            [0.25, 0.3, 0.3, 0.2, 0.2, 0.1, 0.2, 0.1],
        ],
        mc_surf_tot: SurfTot::SelonReseauClasse {
            classe_jusqu_en_2027: jusqu_a!(5000.0, 365.0, -0.073),
            autres: jusqu_a!(5000.0, 265.0, -0.053),
        },
        mc_cat: NUL,
        dh_max: [900.0, 2200.0, 2200.0],
        categorie_3: false,
    },
    // 25. Établissements sportifs municipaux ou scolaires (Bbio p. 28 ; Cep p. 56 et 57 ; DH
    // p. 105).
    DonneesUsage {
        usage: 25,
        bbio_max_moyen: 94.0,
        mb_geo: MB_GEO_SPORT,
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: NUL,
        cep_nr_max_moyen: 94.0,
        cep_max_moyen: 116.0,
        ic_rcu: Some([330.0, 265.0, 265.0]),
        ic_autres: autres(150.0),
        mc_geo: [
            [0.0, 0.1, 0.1, -0.1, 0.0, 0.0, 0.4, 0.25],
            [0.05, 0.1, 0.05, -0.05, 0.0, -0.05, 0.2, 0.05],
            [0.1, 0.15, 0.1, 0.05, 0.1, 0.0, 0.1, 0.0],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: NUL,
        dh_max: [2000.0, 4600.0, 5000.0],
        categorie_3: false,
    },
    // 26. Restaurants scolaires, 1 repas par jour, 5 jours sur 7 (Bbio p. 29 ; Cep p. 57 et
    // 58 ; DH p. 105).
    DonneesUsage {
        usage: 26,
        bbio_max_moyen: 76.0,
        mb_geo: [
            [0.15, 0.2, 0.15, -0.05, 0.0, -0.05, 0.1, 0.1],
            [0.35, 0.4, 0.35, 0.2, 0.25, 0.1, 0.2, 0.1],
            [0.65, 0.65, 0.6, 0.5, 0.55, 0.35, 0.35, 0.25],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: NUL,
        cep_nr_max_moyen: 119.0,
        cep_max_moyen: 251.0,
        ic_rcu: Some([570.0, 445.0, 445.0]),
        ic_autres: autres(445.0),
        mc_geo: [
            [0.1, 0.1, 0.05, 0.0, 0.0, -0.05, -0.05, -0.15],
            [0.25, 0.25, 0.2, 0.2, 0.2, 0.1, 0.05, -0.05],
            [0.45, 0.45, 0.4, 0.35, 0.4, 0.25, 0.20, 0.1],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: NUL,
        dh_max: [40.0, 170.0, 170.0],
        categorie_3: true,
    },
    // 27. Restaurants scolaires, 3 repas par jour, 5 jours sur 7 (Bbio p. 29 et 30 ; Cep
    // p. 58 ; DH p. 106).
    DonneesUsage {
        usage: 27,
        bbio_max_moyen: 116.0,
        mb_geo: [
            [0.1, 0.15, 0.15, -0.05, 0.0, -0.05, 0.1, 0.1],
            [0.3, 0.35, 0.3, 0.15, 0.2, 0.1, 0.2, 0.1],
            [0.55, 0.55, 0.5, 0.45, 0.45, 0.3, 0.35, 0.2],
        ],
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: NUL,
        cep_nr_max_moyen: 153.0,
        cep_max_moyen: 329.0,
        ic_rcu: Some([615.0, 485.0, 485.0]),
        ic_autres: autres(485.0),
        mc_geo: [
            [0.05, 0.1, 0.05, 0.0, 0.0, 0.0, 0.0, -0.05],
            [0.2, 0.2, 0.15, 0.1, 0.15, 0.05, 0.05, 0.0],
            [0.3, 0.35, 0.3, 0.25, 0.25, 0.2, 0.15, 0.1],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: NUL,
        dh_max: [260.0, 650.0, 650.0],
        categorie_3: true,
    },
    // 28. Établissements sportifs privés (Bbio p. 28 ; Cep p. 59 ; DH p. 106).
    DonneesUsage {
        usage: 28,
        bbio_max_moyen: 94.0,
        mb_geo: MB_GEO_SPORT,
        mb_surf_tot: SurfTot::Nulle,
        mb_bruit: NUL,
        cep_nr_max_moyen: 112.0,
        cep_max_moyen: 148.0,
        ic_rcu: Some([535.0, 420.0, 420.0]),
        ic_autres: autres(190.0),
        mc_geo: [
            [0.0, 0.1, 0.05, -0.1, 0.0, 0.05, 0.35, 0.25],
            [0.0, 0.05, 0.05, -0.05, 0.0, -0.05, 0.15, 0.05],
            [0.05, 0.1, 0.05, 0.0, 0.05, 0.0, 0.1, -0.05],
        ],
        mc_surf_tot: SurfTot::Nulle,
        mc_cat: NUL,
        dh_max: [2000.0, 4600.0, 5000.0],
        categorie_3: false,
    },
];

/// Données d'un usage de 3 à 28 ; `None` pour un autre numéro.
pub(crate) fn donnees_usage(usage: u8) -> Option<&'static DonneesUsage> {
    USAGES.iter().find(|d| d.usage == usage)
}

/// Période de l'année de dépôt du permis : 0 pour 2022 à 2024, 1 pour 2025 à 2027, 2 à partir
/// de 2028 (colonnes des tableaux `Icénergie_maxmoyen` et `Mbsurf_tot` des bureaux).
pub(crate) fn periode(annee_depot_permis: u16) -> usize {
    match annee_depot_permis {
        ..=2024 => 0,
        2025..=2027 => 1,
        _ => 2,
    }
}

/// Valeur d'une modulation par morceaux : `(a + b·Sref)/moyen` du premier morceau dont la
/// borne supérieure n'est pas dépassée.
pub(crate) fn morceaux(t: &[Morceau], sref_m2: f64, moyen: f64) -> f64 {
    t.iter()
        .find(|x| sref_m2 <= x.s_max)
        .map_or(0.0, |x| (x.a + x.b * sref_m2) / moyen)
}

impl SurfTot {
    /// Valeur de la modulation pour la somme `sref_usage_m2` des `Sref` des parties de bâtiment
    /// de l'usage (chapitre II, I et II), rapportée à la valeur moyenne de l'exigence.
    pub(crate) fn valeur(
        &self,
        sref_usage_m2: f64,
        moyen: f64,
        annee_depot_permis: u16,
        raccorde_reseau_classe: bool,
    ) -> f64 {
        match self {
            Self::Nulle => 0.0,
            Self::Morceaux(t) => morceaux(t, sref_usage_m2, moyen),
            Self::ParPeriode(p) => morceaux(p[periode(annee_depot_permis)], sref_usage_m2, moyen),
            Self::SelonReseauClasse {
                classe_jusqu_en_2027,
                autres,
            } => {
                let t = if raccorde_reseau_classe && annee_depot_permis <= 2027 {
                    classe_jusqu_en_2027
                } else {
                    autres
                };
                morceaux(t, sref_usage_m2, moyen)
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Indices des colonnes, pour lire les tests comme les tableaux.
    const H1A: usize = 0;
    const H1B: usize = 1;
    const H1C: usize = 2;
    const H2A: usize = 3;
    const H2B: usize = 4;
    const H2C: usize = 5;
    const H2D: usize = 6;
    const H3: usize = 7;

    fn u(n: u8) -> &'static DonneesUsage {
        donnees_usage(n).unwrap()
    }

    #[test]
    fn vingt_six_usages_dans_l_ordre() {
        for (i, d) in USAGES.iter().enumerate() {
            assert_eq!(usize::from(d.usage), i + 3);
        }
        assert!(donnees_usage(2).is_none());
        assert!(donnees_usage(29).is_none());
    }

    /// Colonne « Catégorie 3 » des tableaux `DH_maxcat` (p. 100 à 106) et liste des usages
    /// du chapitre V, partie II (bureaux, médiathèques, universitaires, hôtels, petite enfance,
    /// restaurants, commerces, santé partie jour, aérogares, restaurants scolaires) : mêmes
    /// usages.
    #[test]
    fn categorie_3_du_chapitre_v() {
        for d in &USAGES {
            assert_eq!(
                d.categorie_3,
                matches!(
                    d.usage,
                    3 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 21 | 22 | 26 | 27
                ),
                "usage {}",
                d.usage
            );
        }
    }

    /// Valeurs moyennes relues sur l'image des p. 6, 30 et 31 (relecture indépendante de la
    /// transcription : tableau recopié ligne à ligne depuis l'image).
    #[test]
    fn valeurs_moyennes_relues_sur_l_image() {
        let bbio = [
            95., 68., 68., 117., 122., 76., 76., 134., 163., 139., 245., 100., 206., 177., 170.,
            225., 174., 164., 133., 248., 257., 69., 94., 76., 116., 94.,
        ];
        let cep_nr = [
            75., 65., 63., 93., 102., 121., 118., 235., 234., 150., 282., 132., 219., 214., 163.,
            242., 190., 274., 165., 191., 290., 94., 94., 119., 153., 112.,
        ];
        let cep = [
            85., 72., 72., 105., 112., 144., 138., 252., 281., 182., 578., 275., 446., 412., 182.,
            306., 252., 302., 180., 253., 365., 116., 116., 251., 329., 148.,
        ];
        for (i, d) in USAGES.iter().enumerate() {
            assert_eq!(d.bbio_max_moyen, bbio[i], "usage {}", d.usage);
            assert_eq!(d.cep_nr_max_moyen, cep_nr[i], "usage {}", d.usage);
            assert_eq!(d.cep_max_moyen, cep[i], "usage {}", d.usage);
        }
    }

    /// `Icénergie_maxmoyen` relu sur l'image des p. 31 à 34 : (usage, raccordé 2022-2024,
    /// raccordé 2025-2027, autres 2025-2027).
    #[test]
    fn ic_energie_relu_sur_l_image() {
        let t: [(u8, f64, f64, f64); 24] = [
            (3, 280., 200., 200.),
            (4, 240., 200., 140.),
            (6, 360., 285., 285.),
            (7, 225., 190., 190.),
            (8, 490., 390., 390.),
            (9, 485., 350., 350.),
            (10, 595., 495., 495.),
            (11, 630., 520., 520.),
            (12, 895., 680., 680.),
            (13, 670., 570., 570.),
            (14, 605., 470., 470.),
            (15, 705., 570., 570.),
            (16, 675., 545., 545.),
            (17, 315., 315., 315.),
            (18, 1460., 1120., 585.),
            (19, 1155., 890., 330.),
            (20, 575., 490., 320.),
            (21, 615., 365., 230.),
            (22, 290., 260., 260.),
            (25, 330., 265., 150.),
            (26, 570., 445., 445.),
            (27, 615., 485., 485.),
            (28, 535., 420., 190.),
            (5, 240., 200., 140.),
        ];
        for (n, r0, r1, a1) in t {
            let d = u(n);
            let rcu = d.ic_rcu.unwrap();
            assert_eq!(
                (rcu[0], rcu[1], d.ic_autres[1]),
                (r0, r1, Some(a1)),
                "usage {n}"
            );
            // À partir de 2028, mêmes valeurs qu'en 2025-2027 sauf l'enseignement.
            if !matches!(n, 4 | 5) {
                assert_eq!(rcu[2], rcu[1], "usage {n}");
                assert_eq!(d.ic_autres[2], d.ic_autres[1], "usage {n}");
            }
        }
        // « - » : usages 6 à 28 non soumis avant 2025.
        for d in &USAGES[3..] {
            assert!(d.ic_autres[0].is_none(), "usage {}", d.usage);
        }
        assert_eq!(u(4).ic_rcu.unwrap()[2], 140.0);
        assert_eq!(u(23).ic_autres, [None, Some(315.0), Some(315.0)]);
        assert_eq!(u(24).ic_autres, [None, Some(110.0), Some(110.0)]);
        assert!(u(23).ic_rcu.is_none() && u(24).ic_rcu.is_none());
    }

    /// Cellules de `Mbgéo` et `Mcgéo` relues sur l'image, une par usage au moins, de préférence
    /// celles que le texte de repérage aligne mal.
    #[test]
    fn cellules_geo_relues_sur_l_image() {
        let mb: [(u8, usize, usize, f64); 16] = [
            (3, 0, H2D, 0.30),   // p. 9
            (4, 2, H2A, 0.30),   // p. 10
            (6, 2, H1C, 0.40),   // p. 11
            (7, 2, H1B, 0.6),    // p. 12
            (9, 2, H1C, 0.65),   // p. 14
            (11, 0, H2C, -0.05), // p. 16
            (12, 0, H2B, 0.0),   // p. 17, imprimé « 0, »
            (13, 2, H2D, 0.30),  // p. 18
            (17, 1, H2A, 0.0),   // p. 21
            (18, 1, H3, 0.0),    // p. 22
            (19, 2, H2D, 0.2),   // p. 23
            (22, 2, H1C, -0.05), // p. 25
            (23, 1, H2D, 0.2),   // p. 26
            (25, 1, H2B, -0.05), // p. 28
            (26, 2, H1A, 0.65),  // p. 29
            (27, 2, H2D, 0.35),  // p. 30
        ];
        for (n, l, c, x) in mb {
            assert_eq!(u(n).mb_geo[l][c], x, "Mbgéo usage {n}");
        }
        let mc: [(u8, usize, usize, f64); 14] = [
            (3, 2, H2D, 0.25),   // p. 37, imprimé « 0 ,25 »
            (5, 0, H2D, 0.40),   // p. 38
            (6, 2, H1A, 0.5),    // p. 39
            (7, 2, H3, 0.0),     // p. 40
            (10, 0, H2C, 0.05),  // p. 43
            (12, 1, H2B, 0.20),  // p. 44
            (13, 2, H2A, 0.10),  // p. 45
            (16, 1, H3, 0.0),    // p. 48
            (17, 1, H2A, -0.05), // p. 49
            (20, 0, H2C, 0.05),  // p. 52
            (22, 1, H2C, 0.2),   // p. 53
            (24, 1, H1C, 0.2),   // p. 55
            (26, 0, H3, -0.15),  // p. 57
            (28, 2, H3, -0.05),  // p. 59
        ];
        for (n, l, c, x) in mc {
            assert_eq!(u(n).mc_geo[l][c], x, "Mcgéo usage {n}");
        }
    }

    #[test]
    fn bruit_categorie_et_dh_relus_sur_l_image() {
        // Mbbruit, ligne « Catégorie 3 » (p. 10, 12, 13, 14, 16, 17, 22).
        assert_eq!(u(3).mb_bruit[2], [0.4; 8]);
        assert_eq!(u(6).mb_bruit[2][H2C], 0.2);
        assert_eq!(u(7).mb_bruit[2][H1B], 0.05);
        assert_eq!(u(8).mb_bruit[2], [0.05; 8]);
        assert_eq!(u(11).mb_bruit[2], [0.3; 8]);
        assert_eq!(u(12).mb_bruit[2][H3], 0.3);
        assert_eq!(u(17).mb_bruit[2], [0.2; 8]);
        // Lignes BR1 et BR2/BR3 nulles pour tous les usages 3 à 28.
        for d in &USAGES {
            assert_eq!(d.mb_bruit[0], [0.0; 8]);
            assert_eq!(d.mb_bruit[1], [0.0; 8]);
        }
        // Mccat (p. 38, 39, 40, 45, 49).
        assert_eq!(u(4).mc_cat[1], [0.05; 8]);
        assert_eq!(u(6).mc_cat[2][H3], 0.3);
        assert_eq!(u(7).mc_cat[2][H2D], 0.05);
        assert_eq!(u(12).mc_cat[2][H1A], 0.0);
        assert_eq!(u(12).mc_cat[2][H2A], 0.0);
        assert_eq!(u(17).mc_cat[2], [0.05; 8]);
        // DH_maxcat (p. 100 à 106).
        let dh: [(u8, [f64; 3], bool); 26] = [
            (3, [1150., 2400., 2600.], true),
            (4, [900., 1800., 2200.], false),
            (5, [900., 1800., 2200.], false),
            (6, [900., 2200., 2400.], true),
            (7, [900., 2200., 2400.], true),
            (8, [300., 700., 1000.], true),
            (9, [300., 700., 1000.], true),
            (10, [1300., 3300., 3400.], true),
            (11, [1300., 3300., 3400.], true),
            (12, [550., 1600., 1600.], true),
            (13, [2500., 5000., 5000.], true),
            (14, [250., 650., 650.], true),
            (15, [1600., 3500., 3500.], true),
            (16, [1250., 2500., 2500.], true),
            (17, [3300., 8000., 9500.], true),
            (18, [1000., 2200., 2200.], false),
            (19, [900., 2400., 2600.], false),
            (20, [900., 2400., 2600.], false),
            (21, [1250., 3000., 3300.], true),
            (22, [12100., 21500., 21500.], true),
            (23, [3200., 8000., 8000.], false),
            (24, [900., 2200., 2200.], false),
            (25, [2000., 4600., 5000.], false),
            (26, [40., 170., 170.], true),
            (27, [260., 650., 650.], true),
            (28, [2000., 4600., 5000.], false),
        ];
        for (n, v, c3) in dh {
            assert_eq!((u(n).dh_max, u(n).categorie_3), (v, c3), "usage {n}");
        }
    }

    /// Continuité des modulations par morceaux aux bornes où le texte la prévoit (calculée à la
    /// main) ; les discontinuités de l'usage 21 sont celles du texte (Q65).
    #[test]
    fn modulations_selon_la_surface() {
        let v = |n: u8, s: f64, annee: u16| {
            let d = u(n);
            (
                d.mb_surf_tot.valeur(s, 1.0, annee, false),
                d.mc_surf_tot.valeur(s, 1.0, annee, false),
            )
        };
        // Bureaux, 2022 : 24 − 0,06 × 500 = −6 = −5,55 − 0,0009 × 500.
        assert!((v(3, 500.0, 2022).0 + 6.0).abs() < 1e-12);
        assert!((v(3, 500.1, 2022).0 + 6.0).abs() < 1e-3);
        // Bureaux 2026 à 4 000 m² : −4,9 − 8,8 = −13,7 = −9,7 − 4.
        assert!((v(3, 4000.0, 2026).0 + 13.7).abs() < 1e-12);
        assert!((v(3, 4000.1, 2026).0 + 13.7).abs() < 1e-3);
        // 2028 et au-delà de 4 000 m² : −21,4.
        assert_eq!(v(3, 20000.0, 2030).0, -21.4);
        // Cep bureaux : 18 − 16 = 2 = 6 − 4 à 500 m² ; −6 au-delà de 1 500 m².
        assert!((v(3, 500.0, 2026).1 - 2.0).abs() < 1e-12);
        assert_eq!(v(3, 3000.0, 2026).1, -6.0);
        // Enseignement primaire : 35 − 25 = 10 = 20 − 10 à 500 m² ; 0 à 1 000 m².
        assert!((v(4, 500.0, 2026).0 - 10.0).abs() < 1e-12);
        assert!(v(4, 1000.0, 2026).0.abs() < 1e-12);
        // Santé jour : 22 − 16 = 6 à 2 000 m², 0 au-delà ; Cep : 0 jusqu'à 2 000 m², puis
        // 49 − 52 = −3, et −78 au-delà de 5 000 m² (49 − 130 = −81 à 5 000 m²).
        assert!((v(21, 2000.0, 2026).0 - 6.0).abs() < 1e-12);
        assert_eq!(v(21, 2000.1, 2026).0, 0.0);
        assert_eq!(v(21, 2000.0, 2026).1, 0.0);
        assert!((v(21, 3000.0, 2026).1 + 29.0).abs() < 1e-12);
        assert!((v(21, 5000.0, 2026).1 + 81.0).abs() < 1e-12);
        assert_eq!(v(21, 6000.0, 2026).1, -78.0);
        // Industries 8 h-18 h : réseau classé jusqu'en 2027.
        let d = u(24);
        assert!((d.mc_surf_tot.valeur(1000.0, 1.0, 2027, true) - 292.0).abs() < 1e-9);
        assert!((d.mc_surf_tot.valeur(1000.0, 1.0, 2028, true) - 212.0).abs() < 1e-9);
        assert!((d.mc_surf_tot.valeur(1000.0, 1.0, 2026, false) - 212.0).abs() < 1e-9);
        assert_eq!(d.mc_surf_tot.valeur(5000.1, 1.0, 2026, true), 0.0);
    }
}
