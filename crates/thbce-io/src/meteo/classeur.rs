// SPDX-License-Identifier: GPL-3.0-or-later
//! Classeur météo réglementaire `4_scenarios_meteorologiques_th-bc_th-d_re2020.xlsx`.
//!
//! Structure décrite dans `docs/reference/README.md` : 16 onglets `<zone>_<mode>` (zones H1a à
//! H3, modes `Th-BC` et `Th-D`), 8 760 lignes, colonnes `h, te0, we0, dirN (W/m2), diff (W/m2),
//! Teciel(C), Vent(m/s), Teau(C), Gamma (), Psi ()`.
//!
//! Le lecteur cherche, dans l'onglet, la ligne d'en-tête dont la première cellule est `h`, puis
//! repère chaque colonne par le début de son nom ; il tolère donc des lignes de notice
//! au-dessus du tableau et des variantes d'espacement dans les unités.
//!
//! Ce lecteur n'a pu être essayé que sur un classeur synthétique de même structure : le
//! classeur officiel ne peut pas être versionné. Le test `classeur_reglementaire`
//! (`tests/meteo_re2020.rs`) le vérifie en local contre le CSV dérivé.

use std::io::Cursor;

use calamine::{Data, Reader, Xlsx};
use thbce_core::climat::{JeuMeteo, MeteoProjet, OrigineMeteo};
use thbce_core::conventions::NB_HEURES_AN;
use thbce_data::ZoneClimatique;

use super::csv_re2020::Series;
use super::ErreurMeteo;

/// Début du nom de chaque colonne de données, dans l'ordre de la structure interne.
const DEBUTS_DE_COLONNES: [&str; 9] = [
    "te0", "we0", "dirn", "diff", "teciel", "vent", "teau", "gamma", "psi",
];

fn nombre(cellule: &Data) -> Option<f64> {
    match cellule {
        Data::Float(v) => Some(*v),
        Data::Int(v) => Some(*v as f64),
        Data::String(s) => s.trim().replace(',', ".").parse().ok(),
        _ => None,
    }
    .filter(|v: &f64| v.is_finite())
}

fn texte(cellule: &Data) -> String {
    match cellule {
        Data::String(s) => s.trim().to_lowercase(),
        _ => String::new(),
    }
}

fn lire_onglet<R: std::io::Read + std::io::Seek>(
    classeur: &mut Xlsx<R>,
    zone: ZoneClimatique,
    jeu: JeuMeteo,
) -> Result<Series, ErreurMeteo> {
    let nom = format!("{}_{}", zone.nom(), jeu.nom());
    let plage = classeur
        .worksheet_range(&nom)
        .map_err(|e| ErreurMeteo::Classeur(format!("onglet « {nom} » : {e}")))?;
    let mut lignes = plage.rows().enumerate();

    // Ligne d'en-tête : première cellule « h ».
    let indices = loop {
        let Some((_, ligne)) = lignes.next() else {
            return Err(ErreurMeteo::Classeur(format!(
                "onglet « {nom} » : ligne d'en-tête « h, te0, … » introuvable"
            )));
        };
        if ligne.first().map(texte).as_deref() != Some("h") {
            continue;
        }
        let noms: Vec<String> = ligne.iter().map(texte).collect();
        let mut indices = [0usize; 9];
        for (cible, debut) in indices.iter_mut().zip(DEBUTS_DE_COLONNES) {
            *cible = noms
                .iter()
                .position(|n| n.starts_with(debut))
                .ok_or_else(|| {
                    ErreurMeteo::Classeur(format!(
                        "onglet « {nom} » : colonne « {debut} » introuvable"
                    ))
                })?;
        }
        break indices;
    };

    let mut series = Series::nouvelles();
    for (i, ligne) in lignes {
        let erreur = |message: String| ErreurMeteo::Ligne {
            ligne: i + 1,
            message: format!("onglet « {nom} » : {message}"),
        };
        let Some(h) = ligne.first().and_then(nombre) else {
            // Fin du tableau : ligne vide ou note de bas d'onglet.
            break;
        };
        let attendu = series.colonnes[0].len();
        #[allow(clippy::float_cmp)]
        if h != attendu as f64 {
            return Err(erreur(format!(
                "h = {h} alors que h = {attendu} était attendu"
            )));
        }
        for ((colonne, indice), debut) in series
            .colonnes
            .iter_mut()
            .zip(indices)
            .zip(DEBUTS_DE_COLONNES)
        {
            let v = ligne.get(indice).and_then(nombre).ok_or_else(|| {
                erreur(format!(
                    "valeur absente ou non numérique (colonne « {debut} »)"
                ))
            })?;
            colonne.push(v);
        }
    }
    let lues = series.colonnes[0].len();
    if lues != NB_HEURES_AN {
        return Err(ErreurMeteo::Incomplet {
            zone,
            jeu: jeu.nom(),
            lues,
        });
    }
    Ok(series)
}

/// Lit, dans le classeur réglementaire, les deux jeux (`Th-BC` et `Th-D`) d'une zone.
pub fn lire_classeur_re2020(
    octets: &[u8],
    zone: ZoneClimatique,
) -> Result<MeteoProjet, ErreurMeteo> {
    let mut classeur: Xlsx<_> = Xlsx::new(Cursor::new(octets))
        .map_err(|e| ErreurMeteo::Classeur(format!("fichier .xlsx illisible : {e}")))?;
    let th_bc = lire_onglet(&mut classeur, zone, JeuMeteo::ThBC)?;
    let th_d = lire_onglet(&mut classeur, zone, JeuMeteo::ThD)?;
    Ok(MeteoProjet {
        th_bc: th_bc.en_annee(zone, JeuMeteo::ThBC, OrigineMeteo::ReglementaireRe2020),
        th_d: th_d.en_annee(zone, JeuMeteo::ThD, OrigineMeteo::ReglementaireRe2020),
    })
}
