// SPDX-License-Identifier: GPL-3.0-or-later
//! Assemblage du `CHANGELOG.md` à partir des fragments de `changelog.d/`.
//!
//! Chaque lot écrit son propre fragment `changelog.d/<lot>.md`, rédigé en sections
//! Keep a Changelog (`### Ajouté`, `### Modifié`, `### Corrigé`…). Au moment de publier,
//! `xtask changelog <version>` les réunit sous une nouvelle section datée de `CHANGELOG.md`
//! puis les supprime ; `xtask changelog --apercu` affiche l'assemblage sans rien écrire.

use std::path::PathBuf;

use crate::racine;

const DOSSIER: &str = "changelog.d";
const FICHIER: &str = "CHANGELOG.md";
const TITRE_NON_PUBLIE: &str = "## [Non publié]";

/// Ordre des sections de Keep a Changelog ; les autres suivent, par ordre alphabétique.
const ORDRE: [&str; 6] = [
    "Ajouté",
    "Modifié",
    "Obsolète",
    "Retiré",
    "Corrigé",
    "Sécurité",
];

/// Fragment : nom du lot et sections (titre, contenu).
struct Fragment {
    lot: String,
    sections: Vec<(String, String)>,
}

fn dossier() -> PathBuf {
    racine().join(DOSSIER)
}

fn analyser(lot: &str, contenu: &str) -> Result<Fragment, String> {
    let contenu = contenu.replace("\r\n", "\n");
    let mut sections: Vec<(String, String)> = Vec::new();
    for ligne in contenu.lines() {
        if let Some(titre) = ligne.strip_prefix("### ") {
            let titre = titre.trim();
            if titre.is_empty() {
                return Err(format!("{DOSSIER}/{lot}.md : titre de section vide"));
            }
            sections.push((titre.to_owned(), String::new()));
        } else if let Some((_, texte)) = sections.last_mut() {
            texte.push_str(ligne);
            texte.push('\n');
        } else if !ligne.trim().is_empty() && !ligne.starts_with("<!--") {
            return Err(format!(
                "{DOSSIER}/{lot}.md : le fragment doit commencer par un titre `### Ajouté`, `### Modifié`, `### Corrigé`…"
            ));
        }
    }
    for (titre, texte) in &mut sections {
        *texte = texte.trim_matches('\n').to_owned();
        if texte.is_empty() {
            return Err(format!(
                "{DOSSIER}/{lot}.md : la section « {titre} » est vide"
            ));
        }
    }
    if sections.is_empty() {
        return Err(format!("{DOSSIER}/{lot}.md : aucune section"));
    }
    Ok(Fragment {
        lot: lot.to_owned(),
        sections,
    })
}

/// Fragments triés par nom de lot.
fn lire_fragments() -> Result<Vec<Fragment>, String> {
    let chemin = dossier();
    let mut fragments = Vec::new();
    let entrees = match std::fs::read_dir(&chemin) {
        Ok(entrees) => entrees,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(fragments),
        Err(e) => return Err(format!("{} : {e}", chemin.display())),
    };
    for entree in entrees {
        let entree = entree.map_err(|e| format!("{} : {e}", chemin.display()))?;
        let nom = entree.file_name().to_string_lossy().into_owned();
        if nom == "README.md" || nom == ".gitkeep" {
            continue;
        }
        let Some(lot) = nom.strip_suffix(".md") else {
            return Err(format!("{DOSSIER}/{nom} : fichier inattendu (<lot>.md)"));
        };
        if lot.is_empty()
            || !lot
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(format!(
                "{DOSSIER}/{nom} : nom de lot invalide (minuscules, chiffres et tirets)"
            ));
        }
        let contenu =
            std::fs::read_to_string(entree.path()).map_err(|e| format!("{DOSSIER}/{nom} : {e}"))?;
        fragments.push(analyser(lot, &contenu)?);
    }
    fragments.sort_by(|a, b| a.lot.cmp(&b.lot));
    Ok(fragments)
}

/// Sections réunies, dans l'ordre de Keep a Changelog.
fn assembler(fragments: &[Fragment]) -> String {
    let mut titres: Vec<&str> = Vec::new();
    for f in fragments {
        for (titre, _) in &f.sections {
            if !titres.contains(&titre.as_str()) {
                titres.push(titre);
            }
        }
    }
    let rang = |t: &str| ORDRE.iter().position(|o| *o == t).unwrap_or(ORDRE.len());
    titres.sort_by(|a, b| (rang(a), *a).cmp(&(rang(b), *b)));
    let mut sortie = String::new();
    for titre in titres {
        sortie.push_str(&format!("### {titre}\n\n"));
        for f in fragments {
            for (t, texte) in &f.sections {
                if t == titre {
                    // Pas de ligne vide entre les puces de fragments différents.
                    sortie.push_str(texte);
                    sortie.push('\n');
                }
            }
        }
        sortie.push('\n');
    }
    sortie.trim_end().to_owned()
}

/// Date du jour (UTC) au format `AAAA-MM-JJ`.
fn date_du_jour() -> Result<String, String> {
    let secondes = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("horloge : {e}"))?
        .as_secs();
    let (a, m, j) = date_civile(i64::try_from(secondes / 86_400).map_err(|e| e.to_string())?);
    Ok(format!("{a:04}-{m:02}-{j:02}"))
}

/// Jours depuis le 1er janvier 1970 vers (année, mois, jour) (algorithme de H. Hinnant).
fn date_civile(jours: i64) -> (i64, i64, i64) {
    let z = jours + 719_468;
    let ere = z.div_euclid(146_097);
    let jour_ere = z.rem_euclid(146_097);
    let annee_ere = (jour_ere - jour_ere / 1_460 + jour_ere / 36_524 - jour_ere / 146_096) / 365;
    let jour_annee = jour_ere - (365 * annee_ere + annee_ere / 4 - annee_ere / 100);
    let mp = (5 * jour_annee + 2) / 153;
    let jour = jour_annee - (153 * mp + 2) / 5 + 1;
    let mois = if mp < 10 { mp + 3 } else { mp - 9 };
    let annee = annee_ere + ere * 400 + i64::from(mois <= 2);
    (annee, mois, jour)
}

fn version_valide(version: &str) -> bool {
    let parties: Vec<&str> = version.split('.').collect();
    parties.len() == 3
        && parties
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

fn date_valide(date: &str) -> bool {
    let b = date.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}

/// Insère la nouvelle section sous « Non publié ».
fn inserer(changelog: &str, section: &str) -> Result<String, String> {
    let debut = changelog
        .find(TITRE_NON_PUBLIE)
        .ok_or_else(|| format!("{FICHIER} : section « {TITRE_NON_PUBLIE} » absente"))?;
    let apres = debut + TITRE_NON_PUBLIE.len();
    let suivante = changelog[apres..]
        .find("\n## [")
        .map_or(changelog.len(), |p| apres + p + 1);
    // Le contenu éventuel de « Non publié » reste sous son titre.
    let non_publie = changelog[apres..suivante].trim_matches('\n');
    let mut sortie = String::new();
    sortie.push_str(&changelog[..apres]);
    sortie.push_str("\n\n");
    if !non_publie.is_empty() {
        sortie.push_str(non_publie);
        sortie.push_str("\n\n");
    }
    sortie.push_str(section);
    sortie.push_str("\n\n");
    sortie.push_str(&changelog[suivante..]);
    Ok(sortie)
}

/// `xtask changelog <version> [date]` assemble et supprime les fragments ;
/// sans version (`--apercu`), affiche l'assemblage sans rien écrire.
pub fn commande(version: Option<&str>, date: Option<&str>) -> Result<(), String> {
    let fragments = lire_fragments()?;
    if fragments.is_empty() {
        return Err(format!("aucun fragment dans {DOSSIER}/"));
    }
    let assemble = assembler(&fragments);
    let Some(version) = version else {
        println!("{assemble}");
        return Ok(());
    };
    if !version_valide(version) {
        return Err(format!("version invalide « {version} » (attendu : x.y.z)"));
    }
    let date = match date {
        Some(d) if date_valide(d) => d.to_owned(),
        Some(d) => return Err(format!("date invalide « {d} » (attendu : AAAA-MM-JJ)")),
        None => date_du_jour()?,
    };
    let chemin = racine().join(FICHIER);
    let actuel = std::fs::read_to_string(&chemin)
        .map_err(|e| format!("{} : {e}", chemin.display()))?
        .replace("\r\n", "\n");
    if actuel.contains(&format!("## [{version}]")) {
        return Err(format!("{FICHIER} contient déjà une section [{version}]"));
    }
    let section = format!("## [{version}] - {date}\n\n{assemble}");
    let nouveau = inserer(&actuel, &section)?;
    std::fs::write(&chemin, nouveau).map_err(|e| format!("{} : {e}", chemin.display()))?;
    for f in &fragments {
        let fichier = dossier().join(format!("{}.md", f.lot));
        std::fs::remove_file(&fichier).map_err(|e| format!("{} : {e}", fichier.display()))?;
    }
    println!(
        "{FICHIER} : section [{version}] écrite ({} fragments supprimés)",
        fragments.len()
    );
    Ok(())
}

/// Contrôle de `xtask verifier` : chaque fragment est lisible.
pub fn verifier() -> Result<(), String> {
    let n = lire_fragments()?.len();
    println!("{DOSSIER} est valide ({n} fragments)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(date_civile(0), (1970, 1, 1));
        assert_eq!(date_civile(20_453), (2025, 12, 31));
        assert_eq!(date_civile(20_543), (2026, 3, 31));
    }

    #[test]
    fn fragments_invalides() {
        assert!(analyser("a", "texte sans titre").is_err());
        assert!(analyser("a", "### Ajouté\n").is_err());
        assert!(analyser("a", "").is_err());
        assert!(analyser("a", "### Ajouté\n- un\n").is_ok());
    }

    #[test]
    fn assemblage_ordonne() {
        let a = analyser("a", "### Corrigé\n- c1\n\n### Ajouté\n- a1\n").expect("a");
        let b = analyser("b", "### Ajouté\n- b1\n### Divers\n- d\n").expect("b");
        assert_eq!(
            assembler(&[a, b]),
            "### Ajouté\n\n- a1\n- b1\n\n### Corrigé\n\n- c1\n\n### Divers\n\n- d"
        );
    }

    #[test]
    fn insertion_sous_non_publie() {
        let c = "# T\n\n## [Non publié]\n\n## [0.1.0] - 2026-11-17\n\ntexte\n";
        let s = inserer(c, "## [0.2.0] - 2026-12-01\n\n### Ajouté\n\n- x").expect("insertion");
        assert_eq!(
            s,
            "# T\n\n## [Non publié]\n\n## [0.2.0] - 2026-12-01\n\n### Ajouté\n\n- x\n\n## [0.1.0] - 2026-11-17\n\ntexte\n"
        );
    }
}
