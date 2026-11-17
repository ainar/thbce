// SPDX-License-Identifier: GPL-3.0-or-later
//! Questions de lecture : un fichier par question dans `docs/questions/`.
//!
//! Chaque fichier `Q<nnn>[a-z].md` commence par un en-tête
//!
//! ```text
//! ---
//! numero: Q55
//! sujet: "…"
//! statut: "…"
//! ---
//! ```
//!
//! dont les valeurs `sujet` et `statut` sont des chaînes JSON. Le tableau de bord
//! `docs/questions/ETAT.md` en est entièrement généré.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::racine;

const DOSSIER: &str = "docs/questions";
const ETAT: &str = "ETAT.md";

/// Question lue depuis l'en-tête d'un fichier.
#[derive(Debug, PartialEq, Eq)]
struct Question {
    /// Numéro principal (`1` pour `Q1a`).
    principal: u32,
    /// Suffixe de variante (`a` pour `Q1a`), éventuellement vide.
    suffixe: String,
    sujet: String,
    statut: String,
}

impl Question {
    fn numero(&self) -> String {
        format!("Q{}{}", self.principal, self.suffixe)
    }

    fn nom_fichier(&self) -> String {
        format!("Q{:03}{}.md", self.principal, self.suffixe)
    }
}

/// Décompose `Q12` ou `Q1a` en (12, "") ou (1, "a").
fn lire_numero(texte: &str) -> Option<(u32, String)> {
    let reste = texte.strip_prefix('Q')?;
    let fin = reste
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(reste.len());
    let (chiffres, suffixe) = reste.split_at(fin);
    if chiffres.is_empty() || chiffres.starts_with('0') {
        return None;
    }
    let valide = suffixe.is_empty()
        || (suffixe.len() == 1 && suffixe.chars().all(|c| c.is_ascii_lowercase()));
    if !valide {
        return None;
    }
    Some((chiffres.parse().ok()?, suffixe.to_owned()))
}

/// Lit l'en-tête et contrôle le début du corps d'un fichier de question.
fn lire_question(nom: &str, contenu: &str) -> Result<Question, String> {
    let contenu = contenu.replace("\r\n", "\n");
    let Some(reste) = contenu.strip_prefix("---\n") else {
        return Err(format!(
            "{nom} : en-tête absent (le fichier doit commencer par `---`)"
        ));
    };
    let Some((entete, corps)) = reste.split_once("\n---\n") else {
        return Err(format!("{nom} : en-tête non refermé par `---`"));
    };
    let (mut numero, mut sujet, mut statut) = (None, None, None);
    for ligne in entete.lines() {
        let Some((cle, valeur)) = ligne.split_once(": ") else {
            return Err(format!("{nom} : ligne d'en-tête illisible « {ligne} »"));
        };
        let place = match cle {
            "numero" => &mut numero,
            "sujet" => &mut sujet,
            "statut" => &mut statut,
            _ => return Err(format!("{nom} : clé d'en-tête inconnue « {cle} »")),
        };
        if place.is_some() {
            return Err(format!("{nom} : clé d'en-tête « {cle} » en double"));
        }
        let texte = if cle == "numero" {
            valeur.to_owned()
        } else {
            serde_json::from_str::<String>(valeur).map_err(|e| {
                format!("{nom} : « {cle} » doit être une chaîne entre guillemets doubles ({e})")
            })?
        };
        if texte.trim().is_empty() {
            return Err(format!("{nom} : « {cle} » est vide"));
        }
        *place = Some(texte);
    }
    let (Some(numero), Some(sujet), Some(statut)) = (numero, sujet, statut) else {
        return Err(format!(
            "{nom} : l'en-tête doit porter numero, sujet et statut"
        ));
    };
    let (principal, suffixe) =
        lire_numero(&numero).ok_or_else(|| format!("{nom} : numéro invalide « {numero} »"))?;
    let question = Question {
        principal,
        suffixe,
        sujet,
        statut,
    };
    let titre = format!("### {numero} — ");
    if !corps.trim_start().starts_with(&titre) {
        return Err(format!(
            "{nom} : le corps doit commencer par un titre « {titre}… »"
        ));
    }
    Ok(question)
}

fn dossier() -> PathBuf {
    racine().join(DOSSIER)
}

/// Lit toutes les questions, triées par numéro, en contrôlant leur cohérence.
fn lire_toutes() -> Result<Vec<Question>, String> {
    let chemin = dossier();
    let mut trouvees: Vec<(String, Question)> = Vec::new();
    let entrees = std::fs::read_dir(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
    for entree in entrees {
        let entree = entree.map_err(|e| format!("{} : {e}", chemin.display()))?;
        let nom = entree.file_name().to_string_lossy().into_owned();
        if nom == "README.md" || nom == ETAT {
            continue;
        }
        if !(nom.starts_with('Q') && nom.ends_with(".md")) {
            return Err(format!(
                "{DOSSIER}/{nom} : fichier inattendu (seuls Q<nnn>.md et README.md)"
            ));
        }
        let contenu =
            std::fs::read_to_string(entree.path()).map_err(|e| format!("{DOSSIER}/{nom} : {e}"))?;
        trouvees.push((nom.clone(), lire_question(&nom, &contenu)?));
    }
    trouvees.sort_by(|a, b| a.0.cmp(&b.0));
    let mut utilises: BTreeMap<String, String> = BTreeMap::new();
    for (nom, q) in &trouvees {
        if let Some(autre) = utilises.get(&q.numero()) {
            return Err(format!(
                "{} déjà utilisé par {autre} (doublon dans {nom} : un numéro n'a qu'un seul fichier)",
                q.numero()
            ));
        }
        utilises.insert(q.numero(), nom.clone());
        if q.nom_fichier() != *nom {
            return Err(format!(
                "{nom} : le numéro {} impose le nom {}",
                q.numero(),
                q.nom_fichier()
            ));
        }
    }
    let mut questions: Vec<Question> = trouvees.into_iter().map(|(_, q)| q).collect();
    questions.sort_by(|a, b| (a.principal, &a.suffixe).cmp(&(b.principal, &b.suffixe)));
    Ok(questions)
}

/// Tableau de bord, une ligne par question.
fn tableau(questions: &[Question]) -> String {
    let echapper = |t: &str| t.replace('|', "\\|");
    let mut sortie = String::from("| N° | Sujet | Statut |\n|---|---|---|\n");
    for q in questions {
        sortie.push_str(&format!(
            "| {} | {} | {} |\n",
            q.numero(),
            echapper(&q.sujet),
            echapper(&q.statut)
        ));
    }
    sortie
}

/// Contenu complet de `ETAT.md`.
fn etat(questions: &[Question]) -> String {
    format!(
        "<!-- SPDX-License-Identifier: GPL-3.0-or-later -->\n# État des questions\n\n> Fichier généré par `cargo run -p xtask -- questions`, ne pas modifier à la main.\n> En cas de conflit lors d'une intégration de `main`, garder n'importe quelle version puis relancer la commande.\n\n{}",
        tableau(questions)
    )
}

/// `xtask questions` : régénère `ETAT.md` ; avec `verifier`, contrôle tout sans rien
/// écrire (doublons, en-têtes, noms de fichiers, titres, `ETAT.md` à jour).
pub fn commande(verifier: bool) -> Result<(), String> {
    let questions = lire_toutes()?;
    let chemin = dossier().join(ETAT);
    let genere = etat(&questions);
    if verifier {
        let actuel = std::fs::read_to_string(&chemin)
            .map_err(|e| format!("{} : {e}", chemin.display()))?
            .replace("\r\n", "\n");
        if genere != actuel {
            return Err(format!(
                "{DOSSIER}/{ETAT} n'est pas à jour : relancer `cargo run -p xtask -- questions`"
            ));
        }
        println!("{DOSSIER} est à jour ({} questions)", questions.len());
    } else {
        std::fs::write(&chemin, genere).map_err(|e| format!("{} : {e}", chemin.display()))?;
        println!("{DOSSIER}/{ETAT} écrit ({} questions)", questions.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fichier(entete: &str, corps: &str) -> String {
        format!("---\n{entete}\n---\n\n{corps}\n")
    }

    #[test]
    fn numeros() {
        assert_eq!(lire_numero("Q12"), Some((12, String::new())));
        assert_eq!(lire_numero("Q1a"), Some((1, "a".to_owned())));
        assert_eq!(lire_numero("Q012"), None);
        assert_eq!(lire_numero("Q1ab"), None);
        assert_eq!(lire_numero("12"), None);
    }

    #[test]
    fn lecture_valide() {
        let f = fichier(
            "numero: Q1a\nsujet: \"Un \\\"sujet\\\"\"\nstatut: \"tranchée\"",
            "### Q1a — Titre",
        );
        let q = lire_question("Q001a.md", &f).expect("valide");
        assert_eq!(q.numero(), "Q1a");
        assert_eq!(q.sujet, "Un \"sujet\"");
    }

    #[test]
    fn lectures_invalides() {
        let ok = "numero: Q2\nsujet: \"s\"\nstatut: \"t\"";
        assert!(lire_question("Q002.md", "### Q2 — x").is_err());
        assert!(lire_question("Q002.md", &fichier(ok, "texte")).is_err());
        let sans_statut = "numero: Q2\nsujet: \"s\"";
        assert!(lire_question("Q002.md", &fichier(sans_statut, "### Q2 — x")).is_err());
        let nu = "numero: Q2\nsujet: s\nstatut: \"t\"";
        assert!(lire_question("Q002.md", &fichier(nu, "### Q2 — x")).is_err());
        assert!(lire_question("Q002.md", &fichier(ok, "### Q2 — x")).is_ok());
    }

    #[test]
    fn etat_genere() {
        let q = Question {
            principal: 3,
            suffixe: String::new(),
            sujet: "a | b".to_owned(),
            statut: "t".to_owned(),
        };
        let e = etat(&[q]);
        assert!(e.contains("ne pas modifier à la main"));
        assert!(e.ends_with("| Q3 | a \\| b | t |\n"));
    }
}
