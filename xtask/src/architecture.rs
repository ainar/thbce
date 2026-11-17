// SPDX-License-Identifier: GPL-3.0-or-later
//! Contrôle du guide d'architecture `docs/architecture.md` : chaque chemin qu'il cite existe, et
//! chaque élément de code qu'il désigne par `fichier.rs::Element` y est défini.
//!
//! Sont contrôlés les fragments de code en ligne (entre accents graves, hors blocs de code) qui
//! ressemblent à un chemin — sans espace, avec une barre oblique — et
//! les cibles des liens Markdown relatifs. Un fragment contenant `<`, `*`, `{` ou `…` est un
//! gabarit et n'est pas contrôlé. Un chemin se résout depuis la racine du dépôt, puis depuis
//! `crates/thbce-core/src/` (forme courte des modules du moteur). Pour `chemin.rs::A::b`, le
//! fichier doit définir `b` (`fn`, `struct`, `enum`, `const`, `static`, `type`, `trait`, `mod`
//! ou variante d'énumération en début de ligne).

use std::path::{Path, PathBuf};

use crate::racine;

const GUIDE: &str = "docs/architecture.md";

/// Fragments de code en ligne hors des blocs de code, et cibles des liens relatifs.
fn references(texte: &str) -> Vec<(usize, String, bool)> {
    let mut refs = Vec::new();
    let mut dans_bloc = false;
    for (n, ligne) in texte.lines().enumerate() {
        if ligne.trim_start().starts_with("```") {
            dans_bloc = !dans_bloc;
            continue;
        }
        if dans_bloc {
            continue;
        }
        let morceaux: Vec<&str> = ligne.split('`').collect();
        for (i, morceau) in morceaux.iter().enumerate() {
            // Fragment de code d'un texte de lien (`[`x`](x)`) : la cible est contrôlée.
            let texte_de_lien = morceaux.get(i + 1).is_some_and(|m| m.starts_with("]("));
            if i % 2 == 1 && !texte_de_lien {
                refs.push((n + 1, (*morceau).to_owned(), false));
            }
        }
        let mut reste = ligne;
        while let Some(p) = reste.find("](") {
            let apres = &reste[p + 2..];
            let Some(f) = apres.find(')') else { break };
            refs.push((n + 1, apres[..f].to_owned(), true));
            reste = &apres[f..];
        }
    }
    refs
}

fn est_chemin(s: &str) -> bool {
    !s.is_empty()
        && !s.contains(char::is_whitespace)
        && !s.contains(['<', '*', '{', '…', '"', '=', '$'])
        && !s.starts_with('-')
        && !s.starts_with("http")
        && !s.starts_with("//")
        && s.contains('/')
}

fn resoudre(base: &Path, chemin: &str) -> Option<PathBuf> {
    [base.to_path_buf(), base.join("crates/thbce-core/src")]
        .into_iter()
        .map(|b| b.join(chemin))
        .find(|p| p.exists())
}

/// Le fichier source définit-il `nom` ?
fn definit(source: &str, nom: &str) -> bool {
    let mots = [
        "fn ", "struct ", "enum ", "const ", "static ", "type ", "trait ", "mod ",
    ];
    source.lines().any(|l| {
        let t = l.trim_start();
        let t = t
            .strip_prefix("pub(crate) ")
            .or_else(|| t.strip_prefix("pub(super) "))
            .or_else(|| t.strip_prefix("pub "))
            .unwrap_or(t);
        let t = t
            .strip_prefix("const ")
            .filter(|r| r.starts_with("fn "))
            .unwrap_or(t);
        mots.iter().any(|m| {
            t.strip_prefix(m).is_some_and(|r| {
                r.starts_with(nom)
                    && !r[nom.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
            })
        }) || (t.starts_with(nom)
            && t[nom.len()..].starts_with(['(', ' ', ',', '{'])
            && t.chars().next().is_some_and(char::is_uppercase))
    })
}

fn verifier_reference(base: &Path, docs: &Path, r: &str, lien: bool) -> Result<(), String> {
    if lien {
        let cible = r.split('#').next().unwrap_or("");
        if cible.is_empty() || r.starts_with("http") {
            return Ok(());
        }
        return if docs.join(cible).exists() {
            Ok(())
        } else {
            Err(format!(
                "lien vers « {cible} » introuvable (relatif à docs/)"
            ))
        };
    }
    let (chemin, element) = match r.split_once("::") {
        Some((c, e)) if c.ends_with(".rs") => (c, Some(e)),
        _ => (r, None),
    };
    let chemin = chemin.trim_end_matches(':');
    let Some(fichier) = resoudre(base, chemin.trim_end_matches('/')) else {
        return Err(format!("« {chemin} » introuvable"));
    };
    if let Some(e) = element {
        let nom = e.rsplit("::").next().unwrap_or(e).trim_end_matches("()");
        let source = std::fs::read_to_string(&fichier)
            .map_err(|err| format!("{} : {err}", fichier.display()))?;
        if !definit(&source, nom) {
            return Err(format!("« {nom} » n'est pas défini dans {chemin}"));
        }
    }
    Ok(())
}

/// `xtask architecture` : contrôle le guide ; échoue au premier passage s'il cite un chemin ou
/// un élément disparu.
pub fn commande() -> Result<(), String> {
    let base = racine();
    let texte = std::fs::read_to_string(base.join(GUIDE)).map_err(|e| format!("{GUIDE} : {e}"))?;
    let docs = base.join("docs");
    let mut erreurs = Vec::new();
    let mut n = 0;
    for (ligne, r, lien) in references(&texte) {
        if !lien && !est_chemin(&r) {
            continue;
        }
        n += 1;
        if let Err(e) = verifier_reference(&base, &docs, &r, lien) {
            erreurs.push(format!("{GUIDE}:{ligne} : {e}"));
        }
    }
    if erreurs.is_empty() {
        println!("{GUIDE} : {n} chemins et éléments cités, tous présents");
        Ok(())
    } else {
        Err(format!(
            "{} référence(s) du guide d'architecture à corriger :\n{}",
            erreurs.len(),
            erreurs.join("\n")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnait_les_chemins_et_les_gabarits() {
        assert!(est_chemin("calcul/mod.rs"));
        assert!(est_chemin("calcul/mod.rs::calculer"));
        assert!(est_chemin(".cargo/config.toml"));
        assert!(!est_chemin("//!"));
        assert!(!est_chemin("changelog.d/<lot>.md"));
        assert!(!est_chemin("cargo run -p xtask -- typescript"));
        assert!(!est_chemin("THBCE_METEO=/chemin"));
        assert!(!est_chemin("snake_case"));
    }

    #[test]
    fn trouve_les_definitions() {
        let s = "pub fn calculer(x: u8) {}\n    pub(super) fn restituer_zone(\nenum Generateur {\n    Poele(Poele),\n";
        assert!(definit(s, "calculer"));
        assert!(definit(s, "restituer_zone"));
        assert!(definit(s, "Generateur"));
        assert!(definit(s, "Poele"));
        assert!(!definit(s, "calcul"));
        assert!(!definit(s, "Chaudiere"));
    }
}
