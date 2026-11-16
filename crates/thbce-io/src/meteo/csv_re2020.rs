// SPDX-License-Identifier: GPL-3.0-or-later
//! CSV long dérivé du classeur réglementaire : colonnes
//! `zone, mode, h, te0, we0, dirN_W_m2, diff_W_m2, Teciel_C, Vent_m_s, Teau_C, Gamma, Psi`
//! (8 zones × 2 modes × 8 760 heures). Les lignes commençant par `#` sont des commentaires.

use std::fmt::Write as _;

use thbce_core::climat::{AnneeMeteo, JeuMeteo, MeteoProjet, OrigineMeteo};
use thbce_core::conventions::NB_HEURES_AN;
use thbce_data::ZoneClimatique;

use super::ErreurMeteo;

/// En-tête attendu du CSV long.
pub const COLONNES: [&str; 12] = [
    "zone",
    "mode",
    "h",
    "te0",
    "we0",
    "dirN_W_m2",
    "diff_W_m2",
    "Teciel_C",
    "Vent_m_s",
    "Teau_C",
    "Gamma",
    "Psi",
];

/// Préfixe de la ligne de commentaire qui déclare une provenance non réglementaire.
pub const MARQUE_ORIGINE: &str = "# thbce-origine:";

/// Les neuf séries d'une année, dans l'ordre des colonnes du format réglementaire.
pub(super) struct Series {
    pub(super) colonnes: [Vec<f64>; 9],
}

impl Series {
    pub(super) fn nouvelles() -> Self {
        Self {
            colonnes: std::array::from_fn(|_| Vec::with_capacity(NB_HEURES_AN)),
        }
    }

    pub(super) fn en_annee(
        self,
        zone: ZoneClimatique,
        jeu: JeuMeteo,
        origine: OrigineMeteo,
    ) -> AnneeMeteo {
        let [te0_c, we0_g_kg, idn_w_m2, idi_w_m2, teciel_c, vent_m_s, teau0_c, gamma_deg, psi_deg] =
            self.colonnes;
        // Décision Q1b : Gamma et Psi sont fournis en degrés ; le calcul travaille en radians
        // (Th-BCE 2020 §3.2.1).
        let en_radians = |v: Vec<f64>| v.into_iter().map(f64::to_radians).collect();
        AnneeMeteo {
            zone,
            jeu,
            origine,
            te0_c,
            we0_g_kg,
            idn_w_m2,
            idi_w_m2,
            teciel_c,
            vent_m_s,
            teau0_c,
            gamma_rad: en_radians(gamma_deg),
            psi_rad: en_radians(psi_deg),
        }
    }
}

/// Lit, dans un CSV long au format réglementaire, les deux jeux (`Th-BC` et `Th-D`) d'une zone.
pub fn lire_csv_re2020(octets: &[u8], zone: ZoneClimatique) -> Result<MeteoProjet, ErreurMeteo> {
    let texte = core::str::from_utf8(octets).map_err(|_| ErreurMeteo::Encodage)?;
    let texte = texte.strip_prefix('\u{feff}').unwrap_or(texte);

    let mut origine = OrigineMeteo::ReglementaireRe2020;
    let mut en_tete_lu = false;
    let mut th_bc = Series::nouvelles();
    let mut th_d = Series::nouvelles();
    let nom_zone = zone.nom();

    for (i, brute) in texte.lines().enumerate() {
        let ligne = brute.trim();
        if ligne.is_empty() {
            continue;
        }
        if let Some(description) = ligne.strip_prefix(MARQUE_ORIGINE) {
            origine = OrigineMeteo::Autre {
                description: description.trim().to_owned(),
            };
            continue;
        }
        if ligne.starts_with('#') {
            continue;
        }
        if !en_tete_lu {
            let colonnes: Vec<&str> = ligne.split(',').map(str::trim).collect();
            if colonnes != COLONNES {
                return Err(ErreurMeteo::EnTete(ligne.to_owned()));
            }
            en_tete_lu = true;
            continue;
        }
        let erreur = |message: String| ErreurMeteo::Ligne {
            ligne: i + 1,
            message,
        };
        let mut champs = ligne.split(',');
        let (Some(z), Some(mode), Some(h)) = (champs.next(), champs.next(), champs.next()) else {
            return Err(erreur("moins de trois champs".to_owned()));
        };
        if z != nom_zone {
            continue;
        }
        let series = match mode {
            "Th-BC" => &mut th_bc,
            "Th-D" => &mut th_d,
            autre => {
                return Err(erreur(format!(
                    "mode « {autre} » inconnu (Th-BC ou Th-D attendu)"
                )))
            }
        };
        let attendu = series.colonnes[0].len();
        if h.parse::<usize>().ok() != Some(attendu) {
            return Err(erreur(format!(
                "h = {h} alors que h = {attendu} était attendu pour {z} {mode}"
            )));
        }
        let mut n = 0;
        for (colonne, champ) in series.colonnes.iter_mut().zip(&mut champs) {
            let v: f64 = champ.trim().parse().map_err(|_| {
                erreur(format!(
                    "valeur « {champ} » non numérique (colonne {})",
                    COLONNES[n + 3]
                ))
            })?;
            if !v.is_finite() {
                return Err(erreur(format!(
                    "valeur non finie (colonne {})",
                    COLONNES[n + 3]
                )));
            }
            colonne.push(v);
            n += 1;
        }
        if n != 9 || champs.next().is_some() {
            return Err(erreur(format!("{} champs attendus", COLONNES.len())));
        }
    }
    if !en_tete_lu {
        return Err(ErreurMeteo::EnTete(String::new()));
    }
    for (series, jeu) in [(&th_bc, JeuMeteo::ThBC), (&th_d, JeuMeteo::ThD)] {
        let lues = series.colonnes[0].len();
        if lues != NB_HEURES_AN {
            return Err(ErreurMeteo::Incomplet {
                zone,
                jeu: jeu.nom(),
                lues,
            });
        }
    }
    Ok(MeteoProjet {
        th_bc: th_bc.en_annee(zone, JeuMeteo::ThBC, origine.clone()),
        th_d: th_d.en_annee(zone, JeuMeteo::ThD, origine),
    })
}

/// Écrit des jeux météo au format CSV long, avec leur marque de provenance.
///
/// Réservé aux jeux non réglementaires (jeux d'essai) : la marque `# thbce-origine:` est
/// toujours écrite.
pub fn ecrire_csv(jeux: &[MeteoProjet], description: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "{MARQUE_ORIGINE} {description}");
    let _ = writeln!(s, "{}", COLONNES.join(","));
    for projet in jeux {
        for annee in [&projet.th_bc, &projet.th_d] {
            for h in 0..annee.te0_c.len().min(NB_HEURES_AN) {
                let _ = writeln!(
                    s,
                    "{},{},{h},{},{},{},{},{},{},{},{},{}",
                    annee.zone,
                    annee.jeu.nom(),
                    annee.te0_c[h],
                    annee.we0_g_kg[h],
                    annee.idn_w_m2[h],
                    annee.idi_w_m2[h],
                    annee.teciel_c[h],
                    annee.vent_m_s[h],
                    annee.teau0_c[h],
                    annee.gamma_rad[h].to_degrees(),
                    annee.psi_rad[h].to_degrees(),
                );
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use thbce_core::climat::donnees::DESCRIPTION_SYNTHETIQUE;

    use super::*;

    fn csv_synthetique() -> String {
        let jeux = [
            MeteoProjet::synthetique(ZoneClimatique::H1a),
            MeteoProjet::synthetique(ZoneClimatique::H3),
        ];
        ecrire_csv(&jeux, DESCRIPTION_SYNTHETIQUE)
    }

    #[test]
    fn relit_un_jeu_synthetique_a_l_identique() {
        let csv = csv_synthetique();
        for zone in [ZoneClimatique::H1a, ZoneClimatique::H3] {
            let lu = lire_csv_re2020(csv.as_bytes(), zone).unwrap();
            assert_eq!(lu, MeteoProjet::synthetique(zone));
            assert!(!lu.est_reglementaire());
        }
    }

    #[test]
    fn sans_marque_le_fichier_est_tenu_pour_reglementaire() {
        let csv = csv_synthetique();
        let sans_marque: String = csv.lines().skip(1).map(|l| format!("{l}\n")).collect();
        let lu = lire_csv_re2020(sans_marque.as_bytes(), ZoneClimatique::H3).unwrap();
        assert!(lu.est_reglementaire());
    }

    #[test]
    fn signale_une_zone_absente_un_en_tete_faux_et_une_ligne_fautive() {
        let csv = csv_synthetique();
        assert!(matches!(
            lire_csv_re2020(csv.as_bytes(), ZoneClimatique::H2b),
            Err(ErreurMeteo::Incomplet { lues: 0, .. })
        ));
        assert!(matches!(
            lire_csv_re2020(b"a,b,c\n", ZoneClimatique::H1a),
            Err(ErreurMeteo::EnTete(_))
        ));
        let fautif = csv.replacen("H1a,Th-BC,3,10,", "H1a,Th-BC,3,dix,", 1);
        assert!(matches!(
            lire_csv_re2020(fautif.as_bytes(), ZoneClimatique::H1a),
            Err(ErreurMeteo::Ligne { ligne: 6, .. })
        ));
        let trou = csv.replacen("H1a,Th-BC,3,", "H1a,Th-BC,4,", 1);
        assert!(matches!(
            lire_csv_re2020(trou.as_bytes(), ZoneClimatique::H1a),
            Err(ErreurMeteo::Ligne { ligne: 6, .. })
        ));
        assert_eq!(
            lire_csv_re2020(&[0xff, 0xfe], ZoneClimatique::H1a),
            Err(ErreurMeteo::Encodage)
        );
    }
}
