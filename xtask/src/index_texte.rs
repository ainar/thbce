// SPDX-License-Identifier: GPL-3.0-or-later
//! Index du texte réglementaire : équations, tableaux, figures et fiches de l'annexe III
//! (méthode Th-BCE 2020) et de l'annexe II (règles générales) de l'arrêté du 4 août 2021.
//!
//! La commande lit l'extraction texte (`pdftotext`, une page par segment séparé par un saut de
//! page) et écrit, dans `docs/reference/index/`, des tables CSV et `anomalies.md`. Elle ne
//! rappelle jamais `pdftotext` : l'extraction versionnée suffit, la commande est donc
//! reproductible. La pagination est celle du PDF (page `n` = `n`-ième page ; pour l'annexe III,
//! elle coïncide avec la mention « n/1854 » en pied de page).
//!
//! ## Méthode
//!
//! - **Équations.** Les numéros sont globaux et croissants. Un candidat est une ligne réduite à
//!   `(n)`, ou un numéro coupé en fin de ligne (`(13` puis `4)`). Les candidats qui forment la
//!   plus longue suite strictement croissante sont retenus (la définition). Les autres sont
//!   signalés : doublon, ou hors séquence. Un numéro situé en fin de ligne, après la formule
//!   (`… = a + b (977)`), n'est retenu que s'il comble un trou de la suite. Les renvois dans le
//!   texte (« voir éq. (12) ») ne sont jamais des candidats : ils ne sont ni seuls sur leur
//!   ligne ni en fin de formule. Un numéro qui ouvre la page
//!   (colonne `tete_de_page` = 1) signale une équation qui peut commencer sur la page
//!   précédente : la commande `page --eq` produit alors les deux images, à lire pour
//!   trancher (l'extraction ne permet pas de le décider).
//! - **Tableaux et figures.** Une légende est une ligne qui commence par « Tableau » ou
//!   « Figure », suivie d'un numéro (suffixes admis : `56-1`) puis d'un séparateur (`:`, `–`,
//!   `-`) ou d'un titre en majuscule initiale. Les renvois (« voir Tableau 17), « Tableau 138 et
//!   Tableau 139 ») sont écartés.
//! - **Fiches.** Les titres de chapitre (`5.`) et de fiche (`5.13`) se reconnaissent par leur
//!   numéro, attendu dans l'ordre. Le titre complet est recoupé avec la table des matières
//!   (pages 1 à 14), dont les numéros de page ne servent qu'à l'appariement.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use crate::racine;

/// Dossier des tables générées, relatif à la racine du dépôt.
pub const DOSSIER: &str = "docs/reference/index";
/// Fichier des anomalies, relatif à la racine du dépôt.
pub const ANOMALIES: &str = "docs/reference/index/anomalies.md";
/// Fiches de `docs/couverture.md`, relatif à la racine du dépôt.
const COUVERTURE: &str = "docs/couverture.md";

/// Annexe de l'arrêté indexée.
#[derive(Debug, Clone, Copy)]
pub struct Annexe {
    /// Identifiant en ligne de commande (`iii`, `ii`).
    pub id: &'static str,
    /// Nom lisible.
    pub nom: &'static str,
    /// Extraction texte, relative à la racine du dépôt.
    pub source: &'static str,
    /// PDF d'origine, relatif à la racine du dépôt.
    pub pdf: &'static str,
    /// Première page du corps du texte (les pages précédentes sont la table des matières).
    /// Utilisée pour les fiches seulement.
    pub premiere_page_corps: usize,
    /// L'annexe est découpée en fiches (annexe III).
    pub avec_fiches: bool,
}

/// Annexe III : méthode de calcul détaillée Th-BCE 2020.
pub const ANNEXE_III: Annexe = Annexe {
    id: "iii",
    nom: "annexe III",
    source: "docs/reference/texte/annexe-iii-th-bce-2020.txt",
    pdf: "docs/reference/arrete/annexeiii_arrete_4_aout_2021_complet_compresse.pdf",
    premiere_page_corps: 15,
    avec_fiches: true,
};

/// Annexe II : règles générales.
pub const ANNEXE_II: Annexe = Annexe {
    id: "ii",
    nom: "annexe II",
    source: "docs/reference/texte/annexe-ii-regles-generales.txt",
    pdf: "docs/reference/arrete/annexeii_arrete_4_aout_2021.pdf",
    premiere_page_corps: 1,
    avec_fiches: false,
};

/// Annexes indexées.
pub const ANNEXES: [Annexe; 2] = [ANNEXE_III, ANNEXE_II];

/// Retrouve une annexe par son identifiant.
pub fn annexe_par_id(id: &str) -> Option<Annexe> {
    ANNEXES.iter().copied().find(|a| a.id == id)
}

/// Position dans le texte : (page, ligne dans la page).
type Pos = (usize, usize);

/// Texte d'une annexe : pages numérotées à partir de 1, lignes sans blancs de bord.
#[derive(Debug)]
pub struct Texte {
    pages: Vec<Vec<String>>,
}

impl Texte {
    /// Découpe l'extraction `pdftotext` en pages (sauts de page).
    pub fn lire(contenu: &str) -> Self {
        let mut pages: Vec<Vec<String>> = contenu
            .split('\u{c}')
            .map(|p| p.lines().map(|l| l.trim().to_owned()).collect())
            .collect();
        if pages.last().is_some_and(|p| p.iter().all(|l| l.is_empty())) {
            pages.pop();
        }
        Self { pages }
    }

    /// Nombre de pages.
    pub fn nombre_pages(&self) -> usize {
        self.pages.len()
    }

    fn lignes(&self, page: usize) -> &[String] {
        page.checked_sub(1)
            .and_then(|i| self.pages.get(i))
            .map_or(&[][..], Vec::as_slice)
    }

    /// Première ligne non vide à partir de `ligne` (exclue), parmi les `fenetre` suivantes.
    fn suivante(&self, page: usize, ligne: usize, fenetre: usize) -> Option<&str> {
        self.lignes(page)
            .iter()
            .skip(ligne + 1)
            .take(fenetre)
            .map(String::as_str)
            .find(|l| !l.is_empty())
    }
}

// ---------------------------------------------------------------------------------------------
// Outils de lecture
// ---------------------------------------------------------------------------------------------

/// Chiffres initiaux de `s` : (valeur, reste), de 1 à `max` chiffres, sans zéro initial.
fn entier_initial(s: &str, max: usize) -> Option<(u32, &str)> {
    let fin = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    if fin == 0 || fin > max || s.starts_with('0') {
        return None;
    }
    let valeur = s[..fin].parse().ok()?;
    Some((valeur, &s[fin..]))
}

/// Ligne réduite à `(n)`.
fn etiquette_seule(s: &str) -> Option<u32> {
    let interieur = s.strip_prefix('(')?.strip_suffix(')')?;
    match entier_initial(interieur.trim(), 4)? {
        (n, "") => Some(n),
        _ => None,
    }
}

/// Numéro coupé en deux lignes : `(13` puis `4)`.
fn etiquette_coupee(s: &str, suivante: Option<&str>) -> Option<u32> {
    let (debut, reste) = entier_initial(s.strip_prefix('(')?, 3)?;
    if !reste.is_empty() {
        return None;
    }
    let (fin, reste) = entier_initial(suivante?.trim_end_matches(')'), 3)?;
    let suivante = suivante?;
    if !suivante.ends_with(')') || !reste.is_empty() {
        return None;
    }
    let chiffres = format!("{debut}{fin}");
    chiffres.parse().ok().filter(|_| chiffres.len() <= 4)
}

/// Numéro dont la parenthèse fermante manque : `(1611`.
fn etiquette_ouverte(s: &str) -> Option<u32> {
    match entier_initial(s.strip_prefix('(')?, 4)? {
        (n, "") => Some(n),
        _ => None,
    }
}

/// Numéro en fin de ligne, après la formule : `… + b (977)`.
fn etiquette_fin(s: &str) -> Option<u32> {
    if s.chars().count() < 7 {
        return None;
    }
    let sans = s.strip_suffix(')')?;
    let debut = sans.rfind('(')?;
    let (n, reste) = entier_initial(sans[debut + 1..].trim(), 4)?;
    if !reste.is_empty() {
        return None;
    }
    // Un appel de fonction (`Flt1(0)`, `θ(0)`) n'est pas un numéro d'équation.
    let avant = sans[..debut].chars().next_back()?;
    if avant.is_alphanumeric() {
        return None;
    }
    Some(n)
}

// ---------------------------------------------------------------------------------------------
// Équations
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Genre {
    /// Numéro seul sur sa ligne.
    Seule,
    /// Numéro coupé sur deux lignes.
    Coupee,
    /// Numéro en fin de ligne de formule.
    Fin,
}

#[derive(Debug, Clone, Copy)]
struct Candidat {
    n: u32,
    pos: Pos,
    genre: Genre,
    /// Le numéro est la première ligne non vide de la page.
    en_tete: bool,
}

/// Équation retenue.
#[derive(Debug, Clone)]
pub struct Equation {
    /// Numéro de l'équation.
    pub n: u32,
    /// Page du numéro (fin de l'équation).
    pub page: usize,
    /// Page où l'équation commence (précédente si le numéro ouvre la page).
    pub page_debut: usize,
    /// Numéro déjà rencontré ailleurs : seconde occurrence (ou suivante) du même numéro.
    pub doublon: bool,
    pos: Pos,
}

fn candidats_equations(texte: &Texte) -> Vec<Candidat> {
    let mut sortie = Vec::new();
    for (i, lignes) in texte.pages.iter().enumerate() {
        let page = i + 1;
        let premiere = lignes.iter().position(|l| !l.is_empty());
        for (j, s) in lignes.iter().enumerate() {
            let suivante = texte.suivante(page, j, 3);
            let (n, genre) = if let Some(n) = etiquette_seule(s) {
                (n, Genre::Seule)
            } else if let Some(n) = etiquette_coupee(s, suivante) {
                (n, Genre::Coupee)
            } else if let Some(n) = etiquette_fin(s).or_else(|| etiquette_ouverte(s)) {
                (n, Genre::Fin)
            } else {
                continue;
            };
            sortie.push(Candidat {
                n,
                pos: (page, j),
                genre,
                en_tete: premiere == Some(j),
            });
        }
    }
    sortie
}

/// Plus longue suite strictement croissante (indices dans `valeurs`). En cas d'égalité, la
/// suite qui reste la plus proche de la valeur précédente est préférée.
fn plus_longue_croissante(valeurs: &[u32]) -> Vec<usize> {
    let n = valeurs.len();
    let mut longueur = vec![1_usize; n];
    let mut pred: Vec<Option<usize>> = vec![None; n];
    for i in 0..n {
        for j in 0..i {
            if valeurs[j] < valeurs[i] {
                let meilleur = pred[i].map_or(0, |p| valeurs[p]);
                if longueur[j] + 1 > longueur[i]
                    || (longueur[j] + 1 == longueur[i] && valeurs[j] > meilleur)
                {
                    longueur[i] = longueur[j] + 1;
                    pred[i] = Some(j);
                }
            }
        }
    }
    let Some(mut fin) = (0..n).max_by_key(|&i| (longueur[i], std::cmp::Reverse(i))) else {
        return Vec::new();
    };
    let mut suite = vec![fin];
    while let Some(p) = pred[fin] {
        suite.push(p);
        fin = p;
    }
    suite.reverse();
    suite
}

/// Résultat de l'extraction des équations.
#[derive(Debug, Default)]
pub struct Equations {
    /// Équations retenues, dans l'ordre du texte.
    pub retenues: Vec<Equation>,
    anomalies: Vec<String>,
}

/// Cherche les définitions d'équations d'une annexe.
pub fn extraire_equations(texte: &Texte) -> Equations {
    let candidats = candidats_equations(texte);
    let (stricts, fins): (Vec<Candidat>, Vec<Candidat>) =
        candidats.into_iter().partition(|c| c.genre != Genre::Fin);
    let valeurs: Vec<u32> = stricts.iter().map(|c| c.n).collect();
    let suite = plus_longue_croissante(&valeurs);
    let retenus_idx: BTreeSet<usize> = suite.iter().copied().collect();
    let mut retenus: Vec<Candidat> = suite.iter().map(|&i| stricts[i]).collect();

    let mut anomalies = Vec::new();
    let mut doublons: Vec<Candidat> = Vec::new();
    // Candidats non retenus : doublons, permutations locales, bruit.
    for (i, c) in stricts.iter().enumerate() {
        if retenus_idx.contains(&i) {
            continue;
        }
        let page = c.pos.0;
        if let Some(r) = retenus.iter().find(|r| r.n == c.n) {
            anomalies.push(format!(
                "éq. ({}) : numéro en double, page {page} (retenu : page {})",
                c.n, r.pos.0
            ));
            doublons.push(*c);
            continue;
        }
        // Numéro hors séquence mais voisin de ses voisins de position (deux équations
        // permutées dans le texte) : l'équation existe, elle est retenue et signalée.
        let place = retenus.partition_point(|r| r.pos < c.pos);
        let proche = |k: Option<usize>| {
            k.and_then(|k| retenus.get(k))
                .is_some_and(|r| r.n.abs_diff(c.n) <= 5)
        };
        if proche(place.checked_sub(1)) || proche(Some(place)) {
            retenus.insert(place, *c);
            anomalies.push(format!(
                "éq. ({}) : hors séquence, page {page} (retenue à sa place dans le texte)",
                c.n
            ));
        }
    }

    // Numéros en fin de formule : seulement pour combler un trou.
    for c in fins {
        let place = retenus.partition_point(|r| r.pos < c.pos);
        let avant = place.checked_sub(1).map(|k| retenus[k].n);
        let apres = retenus.get(place).map(|r| r.n);
        let croissant = avant.is_none_or(|a| a < c.n) && apres.is_none_or(|b| c.n < b);
        if croissant && !retenus.iter().any(|r| r.n == c.n) {
            retenus.insert(place, c);
        }
    }

    // Trous.
    let presents: BTreeSet<u32> = retenus.iter().map(|c| c.n).collect();
    let max = presents.last().copied().unwrap_or(0);
    let mut n = 1;
    while n <= max {
        if presents.contains(&n) {
            n += 1;
            continue;
        }
        let debut = n;
        while n <= max && !presents.contains(&n) {
            n += 1;
        }
        let fin = n - 1;
        let page_avant = retenus.iter().rfind(|c| c.n < debut).map(|c| c.pos.0);
        let page_apres = retenus.iter().find(|c| c.n > fin).map(|c| c.pos.0);
        let plage = if debut == fin {
            format!("éq. ({debut})")
        } else {
            format!("éq. ({debut}) à ({fin})")
        };
        let entre = match (page_avant, page_apres) {
            (Some(a), Some(b)) if a == b => format!("page {a}"),
            (Some(a), Some(b)) => format!("pages {a} à {b}"),
            _ => "?".to_owned(),
        };
        anomalies.push(format!(
            "{plage} : introuvable dans le texte extrait ({entre})"
        ));
    }

    let mut equations: Vec<Equation> = retenus
        .iter()
        .map(|c| (c, false))
        .chain(doublons.iter().map(|c| (c, true)))
        .map(|(c, doublon)| Equation {
            n: c.n,
            page: c.pos.0,
            page_debut: if c.en_tete && c.pos.0 > 1 {
                c.pos.0 - 1
            } else {
                c.pos.0
            },
            doublon,
            pos: c.pos,
        })
        .collect();
    equations.sort_by_key(|e| (e.n, e.pos));
    // Ordre des anomalies : par numéro d'équation.
    anomalies.sort_by_key(|a| {
        a.trim_start_matches("éq. (")
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    });
    Equations {
        retenues: equations,
        anomalies,
    }
}

// ---------------------------------------------------------------------------------------------
// Tableaux et figures
// ---------------------------------------------------------------------------------------------

/// Légende de tableau ou de figure.
#[derive(Debug, Clone)]
pub struct Legende {
    /// Numéro, avec son suffixe éventuel (`56-1`).
    pub numero: String,
    /// Page de la légende.
    pub page: usize,
    /// Titre.
    pub titre: String,
    pos: Pos,
}

/// Clé de tri d'un numéro `n` ou `n-s`.
fn cle_numero(numero: &str) -> (u32, u32) {
    let (base, suffixe) = numero.split_once('-').unwrap_or((numero, "0"));
    (base.parse().unwrap_or(0), suffixe.parse().unwrap_or(0))
}

/// Lit `N` ou `N-S` en tête de `s` : (numéro, reste).
fn numero_legende(s: &str) -> Option<(String, &str)> {
    let (base, reste) = entier_initial(s, 4)?;
    if let Some(apres) = reste.strip_prefix('-') {
        if let Some((suffixe, fin)) = entier_initial(apres, 2) {
            if !fin.starts_with(|c: char| c.is_alphanumeric()) {
                return Some((format!("{base}-{suffixe}"), fin));
            }
        }
    }
    Some((base.to_string(), reste))
}

/// Légendes commençant par `mot` (« Tableau » ou « Figure »).
pub fn extraire_legendes(texte: &Texte, mot: &str) -> Vec<Legende> {
    let mut sortie = Vec::new();
    for (i, lignes) in texte.pages.iter().enumerate() {
        let page = i + 1;
        for (j, s) in lignes.iter().enumerate() {
            let Some(apres) = s.strip_prefix(mot).and_then(|r| r.strip_prefix(' ')) else {
                continue;
            };
            let Some((numero, reste)) = numero_legende(apres.trim_start()) else {
                continue;
            };
            let reste = reste.trim();
            let titre = if reste.is_empty() {
                // Titre sur la ligne suivante : sinon c'est un renvoi coupé (« Tableau 121 »
                // puis « et Tableau 122 »).
                match texte.suivante(page, j, 3) {
                    Some(t)
                        if t.chars().next().is_some_and(char::is_uppercase)
                            && !t.starts_with("Figure")
                            && !t.starts_with("Tableau") =>
                    {
                        t.to_owned()
                    }
                    _ => continue,
                }
            } else if let Some(t) = reste
                .strip_prefix(':')
                .or_else(|| reste.strip_prefix('–'))
                .or_else(|| reste.strip_prefix('-'))
            {
                t.trim_start_matches([' ', ':', '–', '-']).trim().to_owned()
            } else if reste.chars().next().is_some_and(char::is_uppercase)
                && reste.chars().count() >= 15
            {
                reste.to_owned()
            } else {
                continue;
            };
            sortie.push(Legende {
                numero,
                page,
                titre,
                pos: (page, j),
            });
        }
    }
    sortie
}

/// Anomalies d'une série de légendes (doublons, trous, hors séquence).
fn anomalies_legendes(libelle: &str, legendes: &[Legende]) -> Vec<String> {
    let mut sortie = Vec::new();
    let mut vus: Vec<&Legende> = Vec::new();
    for l in legendes {
        if let Some(premier) = vus.iter().find(|v| v.numero == l.numero) {
            sortie.push(format!(
                "{libelle} {} : légende en double, pages {} et {}",
                l.numero, premier.page, l.page
            ));
        }
        vus.push(l);
    }
    // Hors séquence : un numéro inférieur au plus grand numéro déjà rencontré.
    let mut maxi = 0;
    for l in legendes {
        let base = cle_numero(&l.numero).0;
        if base < maxi {
            sortie.push(format!(
                "{libelle} {} : hors séquence, page {} (après la légende {maxi})",
                l.numero, l.page
            ));
        }
        maxi = maxi.max(base);
    }
    // Trous.
    let presents: BTreeSet<u32> = legendes.iter().map(|l| cle_numero(&l.numero).0).collect();
    let max = presents.last().copied().unwrap_or(0);
    let manquants: Vec<u32> = (1..=max).filter(|n| !presents.contains(n)).collect();
    for n in manquants {
        sortie.push(format!("{libelle} {n} : introuvable dans le texte extrait"));
    }
    sortie
}

// ---------------------------------------------------------------------------------------------
// Fiches
// ---------------------------------------------------------------------------------------------

/// Nature d'une subdivision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nature {
    /// Chapitre (`5.`).
    Chapitre,
    /// Fiche (`5.13`).
    Fiche,
}

/// Chapitre ou fiche de l'annexe III.
#[derive(Debug, Clone)]
pub struct Section {
    /// Numéro (`5` ou `5.13`).
    pub numero: String,
    /// Nature.
    pub nature: Nature,
    /// Titre.
    pub titre: String,
    /// Première page.
    pub page_debut: usize,
    /// Dernière page.
    pub page_fin: usize,
    pos: Pos,
}

fn normaliser(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Entrée de la table des matières (titre, page imprimée).
fn table_des_matieres(texte: &Texte, premiere_page_corps: usize) -> Vec<(String, usize)> {
    let mut sortie = Vec::new();
    for lignes in texte
        .pages
        .iter()
        .take(premiere_page_corps.saturating_sub(1))
    {
        for s in lignes {
            let Some(i) = s.find("....") else { continue };
            let titre = s[..i].trim();
            let page = s[i..].trim_start_matches('.').trim();
            let Ok(page) = page.parse::<usize>() else {
                continue;
            };
            let minuscule = titre.to_lowercase();
            let sous_section = titre.split_whitespace().next().is_some_and(|m| {
                m.contains('.') && m.chars().all(|c| c.is_ascii_digit() || c == '.')
            });
            let generique = ["introduction", "nomenclature", "description math"]
                .iter()
                .any(|g| minuscule.starts_with(g));
            if titre.is_empty() || sous_section || generique {
                continue;
            }
            sortie.push((titre.to_owned(), page));
        }
    }
    sortie
}

/// Reconnaît `a.b` en tête de ligne (fiche) : (a, b, reste).
fn entete_fiche(s: &str) -> Option<(u32, u32, &str)> {
    let (a, reste) = entier_initial(s, 2)?;
    let (b, reste) = entier_initial(reste.strip_prefix('.')?, 2)?;
    if reste.is_empty() || reste.starts_with(char::is_whitespace) {
        Some((a, b, reste.trim()))
    } else {
        None
    }
}

/// Reconnaît `a.` en tête de ligne (chapitre) : (a, reste).
fn entete_chapitre(s: &str) -> Option<(u32, &str)> {
    let (a, reste) = entier_initial(s, 2)?;
    let reste = reste.strip_prefix('.')?;
    if reste.is_empty() || reste.starts_with(char::is_whitespace) {
        Some((a, reste.trim()))
    } else {
        None
    }
}

/// Extrait chapitres et fiches ; renvoie aussi les anomalies de titre.
pub fn extraire_sections(texte: &Texte, premiere_page_corps: usize) -> (Vec<Section>, Vec<String>) {
    let toc = table_des_matieres(texte, premiere_page_corps);
    let mut utilises: BTreeSet<usize> = BTreeSet::new();
    let mut sections: Vec<Section> = Vec::new();
    let mut anomalies = Vec::new();
    let (mut chapitre, mut fiche) = (0_u32, 0_u32);

    for page in premiere_page_corps..=texte.nombre_pages() {
        let lignes = texte.lignes(page);
        let non_vides: Vec<(usize, &str)> = lignes
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.is_empty())
            .map(|(j, l)| (j, l.as_str()))
            .collect();
        for (k, &(j, s)) in non_vides.iter().enumerate() {
            let suivante = non_vides.get(k + 1).map_or("", |&(_, l)| l);
            let (numero, nature, reste) = if let Some((a, b, reste)) =
                entete_fiche(s).filter(|&(a, b, _)| a == chapitre && b == fiche + 1)
            {
                fiche = b;
                (format!("{a}.{b}"), Nature::Fiche, reste)
            } else if let Some((a, reste)) = entete_chapitre(s).filter(|&(a, _)| a == chapitre + 1)
            {
                let titre = if reste.is_empty() { suivante } else { reste };
                let majuscules = titre.chars().count() >= 3
                    && titre.to_uppercase() == titre
                    && titre.chars().any(char::is_alphabetic);
                let prefixe = format!("{a}.1");
                let annonce = non_vides.iter().skip(k + 1).take(4).any(|&(_, l)| {
                    l == prefixe || l.strip_prefix(&prefixe).is_some_and(|r| r.starts_with(' '))
                });
                if !(annonce || majuscules) {
                    continue;
                }
                chapitre = a;
                fiche = 0;
                (a.to_string(), Nature::Chapitre, reste)
            } else {
                continue;
            };
            let entete = if reste.is_empty() { suivante } else { reste };
            let titre = resoudre_titre(&numero, entete, page, &toc, &mut utilises, &mut anomalies);
            sections.push(Section {
                numero,
                nature,
                titre,
                page_debut: page,
                page_fin: page,
                pos: (page, j),
            });
        }
    }

    // Pages de fin : veille de la subdivision suivante ; un chapitre s'arrête avant le chapitre
    // suivant.
    let total = texte.nombre_pages();
    for i in 0..sections.len() {
        let suivante = match sections[i].nature {
            Nature::Fiche => sections.get(i + 1).map(|s| s.page_debut),
            Nature::Chapitre => sections[i + 1..]
                .iter()
                .find(|s| s.nature == Nature::Chapitre)
                .map(|s| s.page_debut),
        };
        let fin = suivante.map_or(total, |p| p.saturating_sub(1));
        sections[i].page_fin = fin.max(sections[i].page_debut);
    }
    (sections, anomalies)
}

/// Titre complet d'une subdivision, d'après la table des matières.
fn resoudre_titre(
    numero: &str,
    entete: &str,
    page: usize,
    toc: &[(String, usize)],
    utilises: &mut BTreeSet<usize>,
    anomalies: &mut Vec<String>,
) -> String {
    let nom = normaliser(entete);
    let exploitable = nom.chars().count() >= 6 && !entete.starts_with(|c: char| c.is_ascii_digit());
    let ecart = |p: usize| p.abs_diff(page);
    let libres = (0..toc.len()).filter(|i| !utilises.contains(i) && ecart(toc[*i].1) <= 12);
    let choix = if exploitable {
        libres
            .filter(|&i| normaliser(&toc[i].0).starts_with(&nom))
            .min_by_key(|&i| (ecart(toc[i].1), i))
    } else {
        libres.filter(|&i| toc[i].1 == page).min()
    };
    if let Some(i) = choix {
        utilises.insert(i);
        return toc[i].0.clone();
    }
    let generique = ["introduction", "nomenclature"]
        .iter()
        .any(|g| entete.eq_ignore_ascii_case(g));
    if !generique && !numero.is_empty() && numero.contains('.') {
        anomalies.push(format!(
            "fiche {numero} (page {page}) : titre « {entete} » non confirmé par la table des matières"
        ));
    }
    entete.to_owned()
}

/// Fiches de `docs/couverture.md` : (numéro, page de début, page de fin).
fn fiches_couverture(contenu: &str) -> Vec<(String, usize, usize)> {
    let mut sortie = Vec::new();
    for ligne in contenu.lines() {
        let colonnes: Vec<&str> = ligne.split('|').map(str::trim).collect();
        let [_, numero, _titre, pages, ..] = colonnes.as_slice() else {
            continue;
        };
        if entete_fiche(numero).is_none() {
            continue;
        }
        let Some((debut, fin)) = pages.split_once('–') else {
            continue;
        };
        if let (Ok(d), Ok(f)) = (debut.trim().parse(), fin.trim().parse()) {
            sortie.push(((*numero).to_owned(), d, f));
        }
    }
    sortie
}

/// Écarts entre l'index des fiches et les plages de `docs/couverture.md`.
pub fn ecarts_couverture(sections: &[Section], couverture: &str) -> Vec<String> {
    let mut ecarts = Vec::new();
    let reference = fiches_couverture(couverture);
    for (numero, debut, fin) in &reference {
        match sections
            .iter()
            .find(|s| s.nature == Nature::Fiche && s.numero == *numero)
        {
            None => ecarts.push(format!("fiche {numero} : absente de l'index")),
            Some(s) if (s.page_debut, s.page_fin) != (*debut, *fin) => ecarts.push(format!(
                "fiche {numero} : pages {}–{} dans l'index, {debut}–{fin} dans docs/couverture.md",
                s.page_debut, s.page_fin
            )),
            Some(_) => {}
        }
    }
    for s in sections.iter().filter(|s| s.nature == Nature::Fiche) {
        if !reference.iter().any(|(n, _, _)| *n == s.numero) {
            ecarts.push(format!(
                "fiche {} : absente de docs/couverture.md",
                s.numero
            ));
        }
    }
    ecarts
}

// ---------------------------------------------------------------------------------------------
// Index complet et écriture
// ---------------------------------------------------------------------------------------------

/// Index d'une annexe.
#[derive(Debug)]
pub struct Index {
    /// Annexe indexée.
    pub annexe: Annexe,
    /// Nombre de pages du texte.
    pub pages: usize,
    /// Chapitres et fiches (annexe III).
    pub sections: Vec<Section>,
    /// Équations.
    pub equations: Vec<Equation>,
    /// Tableaux.
    pub tableaux: Vec<Legende>,
    /// Figures.
    pub figures: Vec<Legende>,
    /// Anomalies, par rubrique.
    pub anomalies: Vec<(String, Vec<String>)>,
}

impl Index {
    /// Subdivision qui contient la position.
    fn section_de(&self, pos: Pos) -> &str {
        self.sections
            .iter()
            .rfind(|s| s.pos <= pos)
            .map_or("", |s| s.numero.as_str())
    }
}

/// Construit l'index d'une annexe depuis son extraction texte.
pub fn construire(annexe: Annexe, contenu: &str) -> Index {
    let texte = Texte::lire(contenu);
    let mut anomalies = Vec::new();
    let (sections, anomalies_sections) = if annexe.avec_fiches {
        extraire_sections(&texte, annexe.premiere_page_corps)
    } else {
        (Vec::new(), Vec::new())
    };
    let equations = extraire_equations(&texte);
    let corps = annexe.premiere_page_corps;
    let en_corps = |l: &Legende| l.page >= corps;
    let tableaux: Vec<Legende> = extraire_legendes(&texte, "Tableau")
        .into_iter()
        .filter(en_corps)
        .collect();
    let figures: Vec<Legende> = extraire_legendes(&texte, "Figure")
        .into_iter()
        .filter(en_corps)
        .collect();
    if annexe.avec_fiches {
        anomalies.push(("Fiches".to_owned(), anomalies_sections));
    }
    anomalies.push(("Équations".to_owned(), equations.anomalies));
    anomalies.push((
        "Tableaux".to_owned(),
        anomalies_legendes("tableau", &tableaux),
    ));
    anomalies.push(("Figures".to_owned(), anomalies_legendes("figure", &figures)));
    Index {
        annexe,
        pages: texte.nombre_pages(),
        sections,
        equations: equations.retenues,
        tableaux,
        figures,
        anomalies,
    }
}

/// Construit l'index d'une annexe depuis le dépôt.
pub fn charger(annexe: Annexe) -> Result<Index, String> {
    let chemin = racine().join(annexe.source);
    let contenu =
        std::fs::read_to_string(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
    Ok(construire(annexe, &contenu))
}

/// Champ CSV, guillemets si nécessaire.
fn champ(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

fn csv_equations(index: &Index) -> String {
    let mut s = String::new();
    if index.annexe.avec_fiches {
        s.push_str("equation,page,tete_de_page,doublon,fiche\n");
    } else {
        s.push_str("equation,page,tete_de_page,doublon\n");
    }
    for e in &index.equations {
        let _ = write!(
            s,
            "{},{},{},{}",
            e.n,
            e.page,
            u8::from(e.page_debut != e.page),
            u8::from(e.doublon)
        );
        if index.annexe.avec_fiches {
            let _ = write!(s, ",{}", index.section_de(e.pos));
        }
        s.push('\n');
    }
    s
}

fn csv_legendes(index: &Index, entete: &str, legendes: &[Legende]) -> String {
    let mut tries: Vec<&Legende> = legendes.iter().collect();
    tries.sort_by_key(|l| (cle_numero(&l.numero), l.pos));
    let mut s = String::new();
    if index.annexe.avec_fiches {
        let _ = writeln!(s, "{entete},page,fiche,titre");
    } else {
        let _ = writeln!(s, "{entete},page,titre");
    }
    for l in tries {
        let _ = write!(s, "{},{}", l.numero, l.page);
        if index.annexe.avec_fiches {
            let _ = write!(s, ",{}", index.section_de(l.pos));
        }
        let _ = writeln!(s, ",{}", champ(&l.titre));
    }
    s
}

fn csv_fiches(index: &Index) -> String {
    let mut s = String::from("numero,type,titre,page_debut,page_fin\n");
    for f in &index.sections {
        let nature = match f.nature {
            Nature::Chapitre => "chapitre",
            Nature::Fiche => "fiche",
        };
        let _ = writeln!(
            s,
            "{},{nature},{},{},{}",
            f.numero,
            champ(&f.titre),
            f.page_debut,
            f.page_fin
        );
    }
    s
}

fn markdown_anomalies(indexes: &[Index]) -> String {
    let mut s = String::from(
        "<!-- SPDX-License-Identifier: GPL-3.0-or-later -->\n\
         # Anomalies de l'index du texte réglementaire\n\n\
         Fichier généré par `cargo run -p xtask -- index-texte`, ne pas modifier à la main.\n\n\
         Les anomalies décrivent ce que l'**extraction texte** ne permet pas de retrouver : numéros \
         introuvables (équation affichée comme image, numéro coupé, légende absente), numéros en \
         double, numéros hors séquence. Aucun numéro n'est déduit ni inventé : en cas de doute, \
         lire l'image de la page (`cargo run -p xtask -- page`). Pour l'annexe II, la page est \
         la page du PDF (la pagination imprimée a une page de retard).\n",
    );
    for index in indexes {
        let _ = write!(s, "\n## {}\n", index.annexe.nom);
        for (rubrique, liste) in &index.anomalies {
            let _ = write!(s, "\n### {rubrique} ({})\n\n", liste.len());
            if liste.is_empty() {
                s.push_str("Aucune.\n");
            }
            for a in liste {
                let _ = writeln!(s, "- {a}");
            }
        }
    }
    s
}

/// Fichiers générés : (chemin relatif au dépôt, contenu).
pub fn generer(indexes: &[Index]) -> Vec<(String, String)> {
    let mut fichiers = Vec::new();
    for index in indexes {
        let id = index.annexe.id;
        let base = format!("{DOSSIER}/annexe-{id}");
        if index.annexe.avec_fiches {
            fichiers.push((format!("{base}-fiches.csv"), csv_fiches(index)));
        }
        fichiers.push((format!("{base}-equations.csv"), csv_equations(index)));
        fichiers.push((
            format!("{base}-tableaux.csv"),
            csv_legendes(index, "tableau", &index.tableaux),
        ));
        fichiers.push((
            format!("{base}-figures.csv"),
            csv_legendes(index, "figure", &index.figures),
        ));
    }
    fichiers.push((ANOMALIES.to_owned(), markdown_anomalies(indexes)));
    fichiers
}

/// Index des deux annexes depuis le dépôt, et écarts avec `docs/couverture.md`.
fn tout_charger() -> Result<(Vec<Index>, Vec<String>), String> {
    let indexes = ANNEXES
        .iter()
        .map(|a| charger(*a))
        .collect::<Result<Vec<_>, _>>()?;
    let chemin = racine().join(COUVERTURE);
    let couverture =
        std::fs::read_to_string(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
    let ecarts = indexes
        .iter()
        .filter(|i| i.annexe.avec_fiches)
        .flat_map(|i| ecarts_couverture(&i.sections, &couverture))
        .collect();
    Ok((indexes, ecarts))
}

/// Commande `index-texte [--verifier]`.
pub fn commande(verifier: bool) -> Result<(), String> {
    let (indexes, ecarts) = tout_charger()?;
    let fichiers = generer(&indexes);
    let mut erreurs = Vec::new();
    for (chemin, contenu) in &fichiers {
        let absolu = racine().join(chemin);
        if verifier {
            let actuel = std::fs::read_to_string(&absolu).unwrap_or_default();
            if actuel.replace("\r\n", "\n") != *contenu {
                erreurs.push(format!(
                    "{chemin} n'est pas à jour : relancer `cargo run -p xtask -- index-texte`"
                ));
            }
        } else {
            if let Some(parent) = absolu.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("{} : {e}", parent.display()))?;
            }
            std::fs::write(&absolu, contenu).map_err(|e| format!("{} : {e}", absolu.display()))?;
            println!("{chemin} écrit");
        }
    }
    if !verifier {
        for index in &indexes {
            println!(
                "{} : {} pages, {} chapitres et fiches, {} équations, {} tableaux, {} figures",
                index.annexe.nom,
                index.pages,
                index.sections.len(),
                index.equations.iter().filter(|e| !e.doublon).count(),
                index.tableaux.len(),
                index.figures.len()
            );
            for (rubrique, liste) in &index.anomalies {
                println!("  anomalies, {rubrique} : {}", liste.len());
            }
        }
    }
    for e in &ecarts {
        println!("écart avec docs/couverture.md : {e}");
    }
    if verifier && erreurs.is_empty() && ecarts.is_empty() {
        println!("l'index du texte réglementaire est à jour");
    }
    erreurs.extend(
        ecarts
            .into_iter()
            .map(|e| format!("index et docs/couverture.md divergent : {e}")),
    );
    if erreurs.is_empty() {
        Ok(())
    } else {
        Err(erreurs.join("\n"))
    }
}

/// Élément à retrouver dans l'index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Element<'a> {
    /// Équation (numéro).
    Equation(&'a str),
    /// Tableau (numéro, suffixe admis).
    Tableau(&'a str),
    /// Figure (numéro, suffixe admis).
    Figure(&'a str),
}

/// Pages (début, fin) d'un élément, d'après l'index.
pub fn pages_de(index: &Index, element: Element<'_>) -> Result<(usize, usize), String> {
    let nom = index.annexe.nom;
    let legende = |liste: &[Legende], numero: &str, genre: &str| {
        let trouvees: Vec<&Legende> = liste.iter().filter(|l| l.numero == numero).collect();
        match trouvees.as_slice() {
            [] => Err(format!(
                "{genre} {numero} introuvable dans l'index de l'{nom} (voir {ANOMALIES})"
            )),
            [seule] => Ok((seule.page, seule.page)),
            plusieurs => Err(format!(
                "{genre} {numero} en double dans l'{nom} (pages {}) : demander une page avec `page <N>`",
                plusieurs
                    .iter()
                    .map(|l| l.page.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    };
    match element {
        Element::Equation(numero) => {
            let n: u32 = numero
                .trim_matches(['(', ')'])
                .parse()
                .map_err(|_| format!("numéro d'équation invalide : {numero}"))?;
            let trouvees: Vec<&Equation> = index.equations.iter().filter(|e| e.n == n).collect();
            match trouvees.as_slice() {
                [] => Err(format!(
                    "équation {n} introuvable dans l'index de l'{nom} (voir {ANOMALIES})"
                )),
                [seule] => Ok((seule.page_debut, seule.page)),
                plusieurs => Err(format!(
                    "équation {n} en double dans l'{nom} (pages {}) : demander une page avec `page <N>`",
                    plusieurs
                        .iter()
                        .map(|e| e.page.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            }
        }
        Element::Tableau(numero) => legende(&index.tableaux, numero, "tableau"),
        Element::Figure(numero) => legende(&index.figures, numero, "figure"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etiquettes_de_ligne() {
        assert_eq!(etiquette_seule("(131)"), Some(131));
        assert_eq!(etiquette_seule("(378 )"), Some(378));
        assert_eq!(etiquette_seule("(0)"), None);
        assert_eq!(etiquette_seule("voir (12)"), None);
        assert_eq!(etiquette_coupee("(13", Some("4)")), Some(134));
        assert_eq!(etiquette_coupee("(13", Some("texte")), None);
        assert_eq!(etiquette_fin("𝑸 = 𝑨 + 𝑩 (977)"), Some(977));
        assert_eq!(etiquette_ouverte("(1611"), Some(1611));
        assert_eq!(etiquette_fin("Flt1(0)"), None);
        assert_eq!(etiquette_fin("a = 3 (voir éq. 12)"), None);
    }

    #[test]
    fn suite_croissante_ecarte_le_bruit() {
        let suite = plus_longue_croissante(&[1, 2, 5, 3, 4, 0, 5, 6]);
        let valeurs: Vec<u32> = suite.iter().map(|&i| [1, 2, 5, 3, 4, 0, 5, 6][i]).collect();
        assert_eq!(valeurs, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn equations_doublons_trous_et_pages() {
        let texte = "x = 1\n\n(1)\n\n(2)\n\u{c}\n(3)\n\n(3)\n\ny = 2 (5)\n\n(7)\n\u{c}";
        let t = Texte::lire(texte);
        let e = extraire_equations(&t);
        let numeros: Vec<u32> = e
            .retenues
            .iter()
            .filter(|x| !x.doublon)
            .map(|x| x.n)
            .collect();
        assert_eq!(numeros, vec![1, 2, 3, 5, 7]);
        // L'équation 3 ouvre la page 2 : début probable à la page 1.
        let trois = e.retenues.iter().find(|x| x.n == 3 && !x.doublon).unwrap();
        assert_eq!((trois.page_debut, trois.page), (1, 2));
        assert!(e
            .anomalies
            .iter()
            .any(|a| a.contains("(3) : numéro en double")));
        assert!(e
            .anomalies
            .iter()
            .any(|a| a.starts_with("éq. (4) : introuvable")));
        assert!(e
            .anomalies
            .iter()
            .any(|a| a.starts_with("éq. (6) : introuvable")));
    }

    #[test]
    fn legendes_et_renvois() {
        let texte = "Tableau 1 : Nomenclature\nvoir Tableau 17).\n\
                     Tableau 56-1 :Paramètres\nTableau 138 et Tableau 139 multipliées\n\
                     Tableau 121\net Tableau 122\nTableau 5 – Valeurs\nFigure 74 Modélisation d'un système\n\u{c}";
        let t = Texte::lire(texte);
        let tableaux = extraire_legendes(&t, "Tableau");
        let numeros: Vec<&str> = tableaux.iter().map(|l| l.numero.as_str()).collect();
        assert_eq!(numeros, vec!["1", "56-1", "5"]);
        assert_eq!(tableaux[1].titre, "Paramètres");
        let figures = extraire_legendes(&t, "Figure");
        assert_eq!(figures.len(), 1);
        assert_eq!(cle_numero("56-1"), (56, 1));
    }

    #[test]
    fn fiches_et_chapitres() {
        let mut texte = String::new();
        // Table des matières (page 1) puis corps (pages 2 et 3).
        texte.push_str("C_EEX_Climat exterieur ........ 2\nIntroduction ....... 2\n\u{c}");
        texte.push_str("1.\nGENERALITES\n1.1 Introduction\ntexte\n\u{c}");
        texte.push_str("1.2\nC_EEX_Climat\n1.2.1 Introduction\n\u{c}");
        let t = Texte::lire(&texte);
        let (sections, _) = extraire_sections(&t, 2);
        let numeros: Vec<&str> = sections.iter().map(|s| s.numero.as_str()).collect();
        assert_eq!(numeros, vec!["1", "1.1", "1.2"]);
        assert_eq!(sections[2].titre, "C_EEX_Climat exterieur");
        assert_eq!((sections[1].page_debut, sections[1].page_fin), (2, 2));
        assert_eq!((sections[2].page_debut, sections[2].page_fin), (3, 3));
    }
}
