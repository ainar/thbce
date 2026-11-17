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
//! dont les valeurs `sujet` et `statut` sont des chaînes JSON. Le tableau de bord de
//! `docs/questions/README.md`, entre deux marqueurs, en est généré.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::racine;

const DOSSIER: &str = "docs/questions";
const MARQUE_DEBUT: &str = "<!-- debut tableau genere -->";
const MARQUE_FIN: &str = "<!-- fin tableau genere -->";
const MARQUE_PLAGES: &str = "<!-- plages reservees:";

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
    if question.nom_fichier() != nom {
        return Err(format!(
            "{nom} : le numéro {numero} impose le nom {}",
            question.nom_fichier()
        ));
    }
    let titre = format!("### {numero} — ");
    if !corps.trim_start().starts_with(&titre) {
        return Err(format!(
            "{nom} : le corps doit commencer par un titre « {titre}… »"
        ));
    }
    Ok(question)
}

/// Plages réservées déclarées dans le README (`60-69, 80-89`).
fn plages_reservees(readme: &str) -> Result<BTreeSet<u32>, String> {
    let mut numeros = BTreeSet::new();
    let Some(debut) = readme.find(MARQUE_PLAGES) else {
        return Ok(numeros);
    };
    let reste = &readme[debut + MARQUE_PLAGES.len()..];
    let Some(fin) = reste.find("-->") else {
        return Err("README des questions : ligne « plages reservees » non refermée".to_owned());
    };
    for plage in reste[..fin]
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        let borne = |t: &str| {
            t.trim()
                .parse::<u32>()
                .map_err(|_| format!("README des questions : plage réservée illisible « {plage} »"))
        };
        let (a, b) = match plage.split_once('-') {
            Some((a, b)) => (borne(a)?, borne(b)?),
            None => (borne(plage)?, borne(plage)?),
        };
        numeros.extend(a..=b);
    }
    Ok(numeros)
}

fn dossier() -> PathBuf {
    racine().join(DOSSIER)
}

/// Lit toutes les questions, triées par numéro, en contrôlant leur cohérence.
fn lire_toutes() -> Result<Vec<Question>, String> {
    let chemin = dossier();
    let mut questions = Vec::new();
    let entrees = std::fs::read_dir(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
    for entree in entrees {
        let entree = entree.map_err(|e| format!("{} : {e}", chemin.display()))?;
        let nom = entree.file_name().to_string_lossy().into_owned();
        if nom == "README.md" {
            continue;
        }
        if !(nom.starts_with('Q') && nom.ends_with(".md")) {
            return Err(format!(
                "{DOSSIER}/{nom} : fichier inattendu (seuls Q<nnn>.md et README.md)"
            ));
        }
        let contenu =
            std::fs::read_to_string(entree.path()).map_err(|e| format!("{DOSSIER}/{nom} : {e}"))?;
        questions.push(lire_question(&nom, &contenu)?);
    }
    questions.sort_by(|a, b| (a.principal, &a.suffixe).cmp(&(b.principal, &b.suffixe)));
    for paire in questions.windows(2) {
        if paire[0].numero() == paire[1].numero() {
            return Err(format!("numéro {} en double", paire[0].numero()));
        }
    }
    Ok(questions)
}

/// Contrôle l'absence de trou dans les numéros principaux, hors plages réservées.
fn controler_numeros(questions: &[Question], reserves: &BTreeSet<u32>) -> Result<(), String> {
    let presents: BTreeSet<u32> = questions.iter().map(|q| q.principal).collect();
    let Some(&max) = presents.last() else {
        return Err(format!("{DOSSIER} ne contient aucune question"));
    };
    let manquants: Vec<String> = (1..=max)
        .filter(|n| !presents.contains(n) && !reserves.contains(n))
        .map(|n| format!("Q{n}"))
        .collect();
    if manquants.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "numéros manquants : {} (à créer, ou à déclarer dans la ligne « plages reservees » du README des questions)",
            manquants.join(", ")
        ))
    }
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

/// README avec le tableau de bord à jour entre les marqueurs.
fn readme_avec_tableau(readme: &str, tableau: &str) -> Result<String, String> {
    let debut = readme
        .find(MARQUE_DEBUT)
        .ok_or_else(|| format!("README des questions : marqueur « {MARQUE_DEBUT} » absent"))?;
    let fin = readme
        .find(MARQUE_FIN)
        .ok_or_else(|| format!("README des questions : marqueur « {MARQUE_FIN} » absent"))?;
    if fin < debut {
        return Err("README des questions : marqueurs dans le mauvais ordre".to_owned());
    }
    Ok(format!(
        "{}{MARQUE_DEBUT}\n\n{tableau}\n{}",
        &readme[..debut],
        &readme[fin..]
    ))
}

/// `xtask questions` : régénère le tableau de bord ; avec `verifier`, contrôle tout
/// sans rien écrire (numéros, en-têtes, tableau à jour).
pub fn commande(verifier: bool) -> Result<(), String> {
    let questions = lire_toutes()?;
    let chemin = dossier().join("README.md");
    let readme = std::fs::read_to_string(&chemin)
        .map_err(|e| format!("{} : {e}", chemin.display()))?
        .replace("\r\n", "\n");
    controler_numeros(&questions, &plages_reservees(&readme)?)?;
    let genere = readme_avec_tableau(&readme, &tableau(&questions))?;
    if verifier {
        if genere != readme {
            return Err(format!(
                "{DOSSIER}/README.md n'est pas à jour : relancer `cargo run -p xtask -- questions`"
            ));
        }
        println!("{DOSSIER} est à jour ({} questions)", questions.len());
    } else {
        std::fs::write(&chemin, genere).map_err(|e| format!("{} : {e}", chemin.display()))?;
        println!("{DOSSIER}/README.md écrit ({} questions)", questions.len());
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
        assert!(lire_question("Q003.md", &fichier(ok, "### Q2 — x")).is_err());
        assert!(lire_question("Q002.md", &fichier(ok, "texte")).is_err());
        let sans_statut = "numero: Q2\nsujet: \"s\"";
        assert!(lire_question("Q002.md", &fichier(sans_statut, "### Q2 — x")).is_err());
        let nu = "numero: Q2\nsujet: s\nstatut: \"t\"";
        assert!(lire_question("Q002.md", &fichier(nu, "### Q2 — x")).is_err());
        assert!(lire_question("Q002.md", &fichier(ok, "### Q2 — x")).is_ok());
    }

    #[test]
    fn trous_et_plages() {
        let q = |n| Question {
            principal: n,
            suffixe: String::new(),
            sujet: "s".to_owned(),
            statut: "t".to_owned(),
        };
        let questions = [q(1), q(2), q(5)];
        assert!(controler_numeros(&questions, &BTreeSet::new()).is_err());
        let reserves = plages_reservees("<!-- plages reservees: 3-4 -->").expect("plage");
        assert!(controler_numeros(&questions, &reserves).is_ok());
    }

    #[test]
    fn tableau_entre_marqueurs() {
        let readme = format!("avant\n{MARQUE_DEBUT}\nvieux\n{MARQUE_FIN}\naprès\n");
        let genere = readme_avec_tableau(&readme, "T\n").expect("marqueurs");
        assert_eq!(
            genere,
            format!("avant\n{MARQUE_DEBUT}\n\nT\n\n{MARQUE_FIN}\naprès\n")
        );
        assert_eq!(
            readme_avec_tableau(&genere, "T\n").expect("idempotent"),
            genere
        );
    }
}
