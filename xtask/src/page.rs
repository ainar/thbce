// SPDX-License-Identifier: GPL-3.0-or-later
//! Commande `page` : image PNG d'une page de l'annexe III ou II, pour lire une équation ou un
//! tableau sur le PDF d'origine (qui fait foi).
//!
//! ```text
//! cargo run -p xtask -- page <N> [--annexe iii|ii]
//! cargo run -p xtask -- page [--eq <n>] [--tableau <t>] [--figure <f>] [--annexe iii|ii]
//! ```
//!
//! L'image est produite par `pdftoppm` (paquet `poppler-utils`) à 150 dpi dans `target/pages/`,
//! jamais versionné. Avec `--eq`, une équation à cheval sur deux pages donne deux images.

use std::path::PathBuf;
use std::process::Command;

use crate::index_texte::{self, Annexe, Element, ANNEXE_III};
use crate::racine;

/// Résolution des images, en points par pouce.
const DPI: &str = "150";

const USAGE: &str = "usage : xtask page <N> [--annexe iii|ii] | page --eq <n> | --tableau <t> | --figure <f> [--annexe iii|ii]";

/// Page(s) demandée(s) et annexe.
struct Demande {
    annexe: Annexe,
    page: Option<usize>,
    element: Option<(String, String)>,
}

fn lire_arguments(args: &[&str]) -> Result<Demande, String> {
    let mut annexe = ANNEXE_III;
    let mut page = None;
    let mut element: Option<(String, String)> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            option @ ("--annexe" | "--eq" | "--tableau" | "--figure") => {
                let valeur = args
                    .get(i + 1)
                    .ok_or_else(|| format!("{option} attend une valeur. {USAGE}"))?;
                if option == "--annexe" {
                    annexe = index_texte::annexe_par_id(&valeur.to_lowercase())
                        .ok_or_else(|| format!("annexe inconnue : {valeur} (iii ou ii)"))?;
                } else if element.is_some() {
                    return Err(format!("un seul de --eq, --tableau, --figure. {USAGE}"));
                } else {
                    element = Some((option.to_owned(), (*valeur).to_owned()));
                }
                i += 2;
            }
            numero => {
                if page.is_some() {
                    return Err(format!("une seule page. {USAGE}"));
                }
                page = Some(
                    numero
                        .parse::<usize>()
                        .map_err(|_| format!("page invalide : {numero}. {USAGE}"))?,
                );
                i += 1;
            }
        }
    }
    if page.is_none() && element.is_none() {
        return Err(USAGE.to_owned());
    }
    Ok(Demande {
        annexe,
        page,
        element,
    })
}

/// Produit l'image d'une page ; renvoie son chemin.
fn image(annexe: Annexe, page: usize) -> Result<PathBuf, String> {
    let dossier = racine().join("target").join("pages");
    std::fs::create_dir_all(&dossier).map_err(|e| format!("{} : {e}", dossier.display()))?;
    let prefixe = dossier.join(format!("annexe-{}-p{page:04}", annexe.id));
    let pdf = racine().join(annexe.pdf);
    let sortie = Command::new("pdftoppm")
        .args(["-f", &page.to_string(), "-l", &page.to_string()])
        .args(["-r", DPI, "-png", "-singlefile"])
        .arg(&pdf)
        .arg(&prefixe)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "pdftoppm introuvable : installer poppler-utils (apt install poppler-utils, \
                 brew install poppler)"
                    .to_owned()
            } else {
                format!("pdftoppm : {e}")
            }
        })?;
    if !sortie.status.success() {
        return Err(format!(
            "pdftoppm a échoué sur la page {page} : {}",
            String::from_utf8_lossy(&sortie.stderr).trim()
        ));
    }
    let png = prefixe.with_extension("png");
    if !png.exists() {
        return Err(format!("image non produite : {}", png.display()));
    }
    Ok(png)
}

/// Commande `page`.
pub fn commande(args: &[&str]) -> Result<(), String> {
    let demande = lire_arguments(args)?;
    let annexe = demande.annexe;
    let (debut, fin) = if let Some(page) = demande.page {
        (page, page)
    } else if let Some((option, valeur)) = &demande.element {
        let index = index_texte::charger(annexe)?;
        let element = match option.as_str() {
            "--eq" => Element::Equation(valeur),
            "--tableau" => Element::Tableau(valeur),
            _ => Element::Figure(valeur),
        };
        index_texte::pages_de(&index, element)?
    } else {
        return Err(USAGE.to_owned());
    };
    let total = index_texte::charger(annexe)?.pages;
    if debut == 0 || fin > total {
        return Err(format!(
            "page hors de l'{} : 1 à {total} (pagination du PDF)",
            annexe.nom
        ));
    }
    for page in debut..=fin {
        println!("{}", image(annexe, page)?.display());
    }
    Ok(())
}
