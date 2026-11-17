// SPDX-License-Identifier: GPL-3.0-or-later
//! Fichiers climatiques prospectifs ADEME / Météo-France (Licence Ouverte 2.0), versionnés dans
//! `docs/reference/meteo/ademe/`.
//!
//! **Ces fichiers ne sont pas réglementaires.** Ils servent à exercer la chaîne de calcul dans
//! la CI publique ; tout résultat obtenu avec eux porte `reglementaire = false`.
//!
//! ## Règles de conversion (décision Q16, `docs/questions/Q016.md`)
//!
//! Colonnes, d'après la note technique détaillée ADEME / Météo-France (§3.1 et §3.2.2) :
//! `T2m` température de l'air à 2 m (°C), `T-1m` température du sol à 1 m (°C), `Hus` humidité
//! spécifique (kg d'eau par kg d'air), `FF` vent à 10 m (m/s), `Tc` température d'émittance de
//! l'atmosphère (°C), `Hs` hauteur du soleil (°), `Os_2` azimut du soleil par rapport au sud de
//! −180 à 180° (négatif le matin, comme `Psi`), `Rdn` direct normal et `Rdi` diffus horizontal
//! (W/m²).
//!
//! | Grandeur interne | Règle |
//! |---|---|
//! | `te0` | `T2m` |
//! | `we0` | `Hus / (1 − Hus) × 1000`, en g par kg d'air sec |
//! | `Teau0` | approchée par `T-1m` (ce n'est pas une température d'eau) |
//! | `Teciel` | `Tc` |
//! | `Vent` | `FF` |
//! | `Gamma`, `Psi` | `Hs` (bornée à 0), `Os_2` |
//! | `dirN`, `diff` | `Rdn`, `Rdi` |
//!
//! Les températures sont celles de la station, à son altitude : elles ne sont pas ramenées au
//! niveau de la mer.
//!
//! ## Convention horaire
//!
//! La note technique ne dit pas à quoi correspond l'horodatage. Constat sur les données : dans
//! les fichiers ADEME, `Hs` et `Os_2` sont des valeurs instantanées à l'heure UTC indiquée (le
//! midi solaire tombe à `12 − longitude/15`), et les rayonnements sont des moyennes sur l'heure
//! qui suit. Dans le fichier réglementaire, la ligne `h` porte la position du soleil de
//! l'instant `h − 0,5` et des rayonnements centrés sur ce même instant. Pour s'aligner sur le
//! fichier réglementaire, la ligne `h` produite ici vaut donc :
//!
//! - pour les rayonnements : la ligne ADEME `h − 1` ;
//! - pour les autres grandeurs, instantanées : la moyenne des lignes ADEME `h − 1` et `h`.
//!
//! Pour `h = 0`, la ligne `h − 1` est la dernière de l'année.

use thbce_core::climat::{AnneeMeteo, JeuMeteo, MeteoProjet, OrigineMeteo};
use thbce_core::conventions::NB_HEURES_AN;
use thbce_data::ZoneClimatique;

use super::ErreurMeteo;

/// Début de l'en-tête des fichiers ADEME, qui sert à reconnaître le format.
pub const DEBUT_EN_TETE: &str = "STATION;";

/// Scénario climatique du fichier versionné dans le dépôt.
pub const SCENARIO: &str = "TRACC France +2,7 °C avec vague de chaleur (ADEME / Météo-France)";

/// Station de référence de chaque zone climatique : villes du tableau 73 des règles Th-Bat 2020,
/// la station de H3 étant Marignane depuis la RE2020 (`docs/reference/README.md`).
pub fn station(zone: ZoneClimatique) -> &'static str {
    match zone {
        ZoneClimatique::H1a => "TRAPPES",
        ZoneClimatique::H1b => "NANCY",
        ZoneClimatique::H1c => "MACON",
        ZoneClimatique::H2a => "RENNES",
        ZoneClimatique::H2b => "TOURS",
        ZoneClimatique::H2c => "AGEN",
        ZoneClimatique::H2d => "CARPENTRAS",
        ZoneClimatique::H3 => "MARIGNANE",
    }
}

/// Colonnes lues, dans l'ordre : T2m, T-1m, Hus, FF, Tc, Hs, Os_2, Rdn, Rdi.
const COLONNES_LUES: [&str; 9] = ["T2m", "T-1m", "Hus", "FF", "Tc", "Hs", "Os_2", "Rdn", "Rdi"];
const T2M: usize = 0;
const TSOL: usize = 1;
const HUS: usize = 2;
const FF: usize = 3;
const TC: usize = 4;
const HS: usize = 5;
const OS2: usize = 6;
const RDN: usize = 7;
const RDI: usize = 8;

/// Moyenne de deux azimuts en degrés, en tenant compte du passage par ±180°.
fn moyenne_azimuts(a: f64, b: f64) -> f64 {
    let ecart = b - a;
    let ecart = if ecart > 180.0 {
        ecart - 360.0
    } else if ecart < -180.0 {
        ecart + 360.0
    } else {
        ecart
    };
    let m = a + ecart / 2.0;
    if m >= 180.0 {
        m - 360.0
    } else if m < -180.0 {
        m + 360.0
    } else {
        m
    }
}

/// Lit l'année d'une station ADEME et la convertit dans la structure interne. La même année sert
/// aux deux jeux (`Th-BC` et `Th-D`).
pub fn lire_csv_ademe(octets: &[u8], zone: ZoneClimatique) -> Result<MeteoProjet, ErreurMeteo> {
    let texte = core::str::from_utf8(octets).map_err(|_| ErreurMeteo::Encodage)?;
    let texte = texte.strip_prefix('\u{feff}').unwrap_or(texte);
    let mut lignes = texte.lines().enumerate();
    let (_, en_tete) = lignes
        .next()
        .ok_or_else(|| ErreurMeteo::EnTete(String::new()))?;
    let noms: Vec<&str> = en_tete.trim().split(';').collect();
    let colonne = |nom: &str| {
        noms.iter()
            .position(|n| *n == nom)
            .ok_or_else(|| ErreurMeteo::EnTete(en_tete.to_owned()))
    };
    let i_station = colonne("STATION")?;
    let i_heure = colonne("HEURE")?;
    let mut indices = [0usize; 9];
    for (cible, nom) in indices.iter_mut().zip(COLONNES_LUES) {
        *cible = colonne(nom)?;
    }

    // Valeurs brutes de la station, telles qu'horodatées dans le fichier.
    let nom_station = station(zone);
    let mut brut: [Vec<f64>; 9] = std::array::from_fn(|_| Vec::with_capacity(NB_HEURES_AN));
    for (i, ligne) in lignes {
        let ligne = ligne.trim();
        if ligne.is_empty() {
            continue;
        }
        let champs: Vec<&str> = ligne.split(';').collect();
        if champs.get(i_station) != Some(&nom_station) {
            continue;
        }
        let erreur = |message: String| ErreurMeteo::Ligne {
            ligne: i + 1,
            message,
        };
        let attendu = brut[0].len() % 24;
        if champs.get(i_heure).and_then(|h| h.parse::<usize>().ok()) != Some(attendu) {
            return Err(erreur(format!(
                "HEURE = {attendu} attendue pour la station {nom_station}"
            )));
        }
        for ((serie, indice), nom) in brut.iter_mut().zip(indices).zip(COLONNES_LUES) {
            let v: f64 = champs
                .get(indice)
                .and_then(|c| c.trim().parse().ok())
                .filter(|v: &f64| v.is_finite())
                .ok_or_else(|| {
                    erreur(format!("valeur absente ou non numérique (colonne {nom})"))
                })?;
            serie.push(v);
        }
    }
    let lues = brut[0].len();
    if lues != NB_HEURES_AN {
        return Err(ErreurMeteo::Incomplet {
            zone,
            jeu: "ADEME",
            lues,
        });
    }

    // Alignement sur la convention horaire du fichier réglementaire (voir l'en-tête du module).
    let avant = |h: usize| if h == 0 { NB_HEURES_AN - 1 } else { h - 1 };
    let moyenne = |c: usize| -> Vec<f64> {
        (0..NB_HEURES_AN)
            .map(|h| (brut[c][avant(h)] + brut[c][h]) / 2.0)
            .collect()
    };
    let decale = |c: usize| -> Vec<f64> { (0..NB_HEURES_AN).map(|h| brut[c][avant(h)]).collect() };
    let hus = moyenne(HUS);
    let annee = |jeu: JeuMeteo| AnneeMeteo {
        zone,
        jeu,
        origine: OrigineMeteo::AdemeProspectif {
            scenario: SCENARIO.to_owned(),
        },
        te0_c: moyenne(T2M),
        // Humidité spécifique (kg/kg d'air humide) → poids d'eau (g/kg d'air sec).
        we0_g_kg: hus.iter().map(|q| q / (1.0 - q) * 1000.0).collect(),
        idn_w_m2: decale(RDN).into_iter().map(|v| v.max(0.0)).collect(),
        idi_w_m2: decale(RDI).into_iter().map(|v| v.max(0.0)).collect(),
        teciel_c: moyenne(TC),
        vent_m_s: moyenne(FF),
        teau0_c: moyenne(TSOL),
        gamma_rad: moyenne(HS)
            .into_iter()
            .map(|g| g.max(0.0).to_radians())
            .collect(),
        psi_rad: (0..NB_HEURES_AN)
            .map(|h| moyenne_azimuts(brut[OS2][avant(h)], brut[OS2][h]).to_radians())
            .collect(),
    };
    Ok(MeteoProjet {
        th_bc: annee(JeuMeteo::ThBC),
        th_d: annee(JeuMeteo::ThD),
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn moyenne_des_azimuts_autour_de_180() {
        assert_eq!(moyenne_azimuts(10.0, 30.0), 20.0);
        assert_eq!(moyenne_azimuts(-10.0, 10.0), 0.0);
        assert_eq!(moyenne_azimuts(170.0, -170.0), -180.0);
        assert_eq!(moyenne_azimuts(160.0, -170.0), 175.0);
        assert_eq!(moyenne_azimuts(-160.0, 170.0), -175.0);
    }

    #[test]
    fn refuse_un_fichier_sans_les_colonnes_attendues() {
        assert!(matches!(
            lire_csv_ademe(b"STATION;HEURE;T2m\n", ZoneClimatique::H1a),
            Err(ErreurMeteo::EnTete(_))
        ));
        assert_eq!(
            lire_csv_ademe(&[0xff], ZoneClimatique::H1a),
            Err(ErreurMeteo::Encodage)
        );
    }
}
