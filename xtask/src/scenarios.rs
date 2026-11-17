// SPDX-License-Identifier: GPL-3.0-or-later
//! Conversion du classeur des scénarios conventionnels (Th-BCE 2020 §15) en données versionnées.
//!
//! Chaque onglet décrit un usage : consignes de la zone, sept scénarios de zone (mobilité,
//! occupation, chauffage, refroidissement, ventilation, éclairage, besoins d'ECS), puis, pour
//! chaque local, trois scénarios (occupants, apports de chaleur, apports d'humidité). Un
//! scénario est une grille hebdomadaire (7 jours × 24 heures) et une grille annuelle
//! (5 semaines × 12 mois).
//!
//! Les grilles sont repérées par leur structure (ligne d'en-tête 1 à 24, ou 1 à 12), les
//! libellés variant d'un onglet à l'autre. Les grilles identiques ne sont écrites qu'une fois.

// Les comparaisons exactes de flottants sont voulues : valeurs lues telles quelles du classeur.
#![allow(clippy::float_cmp)]

use std::fmt::Write as _;

use calamine::{Data, Reader, Xlsx};

/// Classeur officiel, relatif à la racine du dépôt.
pub const SOURCE: &str = "docs/reference/scenarios/2026-04-29_scenarios_conventionnels.xlsx";
/// Fichier généré, relatif à la racine du dépôt.
pub const SORTIE: &str = "crates/thbce-data/src/scenarios/table.rs";

/// Nombre de semaines de chaque mois (Th-BCE 2020 §4.1.3).
const SEMAINES_PAR_MOIS: [usize; 12] = [4, 4, 5, 4, 5, 4, 4, 5, 4, 4, 5, 4];
const SCENARIOS_DE_ZONE: [&str; 7] = [
    "mobilité",
    "occupation",
    "chauffage",
    "refroidissement",
    "ventilation",
    "eclairage",
    "besoins d'ecs",
];

/// Corrections explicites de cellules du classeur officiel : (onglet, ligne à partir de 1,
/// colonne à partir de 0, contenu brut, valeur retenue).
///
/// La conversion échoue sur toute cellule non numérique qui n'est pas listée ici, et sur toute
/// correction dont le contenu brut ne correspond plus au classeur. Chaque entrée est une
/// hypothèse de lecture consignée dans `docs/questions/`.
pub const CORRECTIONS: [(&str, usize, usize, &str, f64); 3] = [
    // Q26 : « 0;5 » saisi en texte, semaine 3 du mois d'août, dans le facteur correctif de la
    // semaine d'un local ; la semaine 4 du même mois vaut 0,5. Même saisie dans trois onglets.
    ("VES", 242, 9, "0;5", 0.5),
    ("GYM_MUN", 394, 9, "0;5", 0.5),
    ("GYM_PRI", 394, 9, "0;5", 0.5),
];

type Hebdo = [[f64; 24]; 7];
type Annuel = [[f64; 12]; 5];

#[derive(Clone, Copy)]
struct Scenario {
    hebdo: usize,
    annuel: usize,
}

struct Local {
    nom: String,
    rat_l: f64,
    occupants_nominal: Option<f64>,
    occupants_unite: String,
    occupants_chaleur_w: f64,
    occupants_humidite_kg_h: f64,
    occupants: Scenario,
    chaleur_unite: String,
    chaleur_w: f64,
    chaleur: Scenario,
    humidite_unite: String,
    humidite_kg_h: f64,
    humidite: Scenario,
}

struct Usage {
    numero: u8,
    onglet: String,
    nom: String,
    consignes: [[f64; 2]; 3],
    zone: [Scenario; 7],
    ecs_unite: String,
    ecs_volume: Option<f64>,
    locaux: Vec<Local>,
}

/// Résultat de la conversion : usages et grilles dédoublonnées.
pub struct Scenarios {
    usages: Vec<Usage>,
    hebdos: Vec<Hebdo>,
    annuels: Vec<Annuel>,
}

fn texte(c: &Data) -> String {
    match c {
        Data::String(s) => s.trim().to_owned(),
        Data::Float(v) => v.to_string(),
        Data::Int(v) => v.to_string(),
        _ => String::new(),
    }
}

fn nombre(c: &Data) -> Option<f64> {
    match c {
        Data::Float(v) => Some(*v),
        Data::Int(v) => Some(*v as f64),
        Data::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn cellule(ligne: &[Data], i: usize) -> &Data {
    ligne.get(i).unwrap_or(&Data::Empty)
}

/// Vrai si les cellules de la ligne, à partir de la colonne 2, sont exactement 1, 2, …, n.
fn est_en_tete(ligne: &[Data], n: usize) -> bool {
    let valeurs: Vec<f64> = ligne.iter().skip(2).filter_map(nombre).collect();
    let non_vides = ligne
        .iter()
        .skip(2)
        .filter(|c| !texte(c).is_empty())
        .count();
    non_vides == n
        && valeurs.len() == n
        && valeurs
            .iter()
            .enumerate()
            .all(|(i, v)| *v == (i + 1) as f64)
}

fn indice<T: PartialEq + Copy>(table: &mut Vec<T>, grille: T) -> usize {
    table.iter().position(|g| *g == grille).unwrap_or_else(|| {
        table.push(grille);
        table.len() - 1
    })
}

struct Lecteur<'a> {
    onglet: &'a str,
    lignes: &'a [Vec<Data>],
    i: usize,
}

impl Lecteur<'_> {
    fn erreur(&self, message: &str) -> String {
        format!("onglet {}, ligne {} : {message}", self.onglet, self.i + 1)
    }

    /// Avance jusqu'à la ligne dont la colonne 1 commence par l'un des libellés (casse ignorée).
    fn chercher(&mut self, libelles: &[&str]) -> Result<&[Data], String> {
        while let Some(ligne) = self.lignes.get(self.i) {
            self.i += 1;
            let t = texte(cellule(ligne, 1)).to_lowercase();
            if libelles.iter().any(|l| t.starts_with(l)) {
                return Ok(ligne);
            }
        }
        Err(self.erreur(&format!("libellé {libelles:?} introuvable")))
    }

    /// Prochaine ligne dont la colonne 2 n'est pas vide.
    fn ligne_de_valeur(&mut self) -> Result<&[Data], String> {
        while let Some(ligne) = self.lignes.get(self.i) {
            self.i += 1;
            if !texte(cellule(ligne, 2)).is_empty() {
                return Ok(ligne);
            }
        }
        Err(self.erreur("valeur attendue"))
    }

    /// Valeur numérique d'une cellule de grille de la ligne courante, corrections comprises.
    fn valeur_de_grille(&self, ligne: &[Data], colonne: usize) -> Option<f64> {
        let c = cellule(ligne, colonne);
        nombre(c).or_else(|| {
            let brut = texte(c);
            CORRECTIONS
                .iter()
                .find(|(o, l, col, b, _)| {
                    *o == self.onglet && *l == self.i + 1 && *col == colonne && *b == brut
                })
                .map(|(_, _, _, _, v)| *v)
        })
    }

    /// Lit la prochaine grille hebdomadaire puis la prochaine grille annuelle.
    fn scenario(
        &mut self,
        hebdos: &mut Vec<Hebdo>,
        annuels: &mut Vec<Annuel>,
    ) -> Result<Scenario, String> {
        while !self.lignes.get(self.i).is_some_and(|l| est_en_tete(l, 24)) {
            if self.i >= self.lignes.len() {
                return Err(self.erreur("grille hebdomadaire introuvable"));
            }
            self.i += 1;
        }
        self.i += 1;
        let mut hebdo = [[0.0; 24]; 7];
        for (jour, cible) in hebdo.iter_mut().enumerate() {
            let ligne = self
                .lignes
                .get(self.i)
                .ok_or_else(|| self.erreur("grille hebdomadaire tronquée"))?;
            if nombre(cellule(ligne, 1)) != Some((jour + 1) as f64) {
                return Err(self.erreur(&format!("jour {} attendu", jour + 1)));
            }
            for (heure, v) in cible.iter_mut().enumerate() {
                *v = self.valeur_de_grille(ligne, heure + 2).ok_or_else(|| {
                    self.erreur(&format!(
                        "valeur absente, jour {}, heure {}",
                        jour + 1,
                        heure + 1
                    ))
                })?;
            }
            self.i += 1;
        }
        while !self.lignes.get(self.i).is_some_and(|l| est_en_tete(l, 12)) {
            if self.i >= self.lignes.len()
                || self.lignes.get(self.i).is_some_and(|l| est_en_tete(l, 24))
            {
                return Err(self.erreur("grille annuelle introuvable"));
            }
            self.i += 1;
        }
        self.i += 1;
        let mut annuel = [[0.0; 12]; 5];
        for (semaine, cible) in annuel.iter_mut().enumerate() {
            let ligne = self
                .lignes
                .get(self.i)
                .ok_or_else(|| self.erreur("grille annuelle tronquée"))?;
            if nombre(cellule(ligne, 1)) != Some((semaine + 1) as f64) {
                return Err(self.erreur(&format!("semaine {} attendue", semaine + 1)));
            }
            for (mois, v) in cible.iter_mut().enumerate() {
                let valeur = self.valeur_de_grille(ligne, mois + 2);
                // La semaine n'existe que si le mois compte assez de semaines (§4.1.3).
                let existe = semaine < SEMAINES_PAR_MOIS[mois];
                *v = match (valeur, existe) {
                    (Some(x), true) => x,
                    (None, false) => 0.0,
                    (None, true) => {
                        return Err(self.erreur(&format!(
                            "valeur absente, semaine {}, mois {}",
                            semaine + 1,
                            mois + 1
                        )))
                    }
                    (Some(_), false) => {
                        return Err(self.erreur(&format!(
                            "valeur présente pour la semaine {} du mois {}, qui n'en compte que {}",
                            semaine + 1,
                            mois + 1,
                            SEMAINES_PAR_MOIS[mois]
                        )))
                    }
                };
            }
            self.i += 1;
        }
        Ok(Scenario {
            hebdo: indice(hebdos, hebdo),
            annuel: indice(annuels, annuel),
        })
    }
}

fn lire_onglet(
    onglet: &str,
    lignes: &[Vec<Data>],
    hebdos: &mut Vec<Hebdo>,
    annuels: &mut Vec<Annuel>,
) -> Result<Usage, String> {
    let mut l = Lecteur {
        onglet,
        lignes,
        i: 0,
    };
    let nom = texte(cellule(l.chercher(&["nom de la zone"])?, 3));
    let numero: u8 = nom
        .split('.')
        .next()
        .and_then(|n| n.trim().parse().ok())
        .ok_or_else(|| l.erreur(&format!("numéro d'usage introuvable dans « {nom} »")))?;
    let mut consignes = [[0.0; 2]; 3];
    for (cible, libelle) in
        consignes
            .iter_mut()
            .zip(["normal", "arrêt moins de 48", "arrêt plus de 48"])
    {
        let ligne = l.chercher(&[libelle])?;
        let (ch, fr) = (nombre(cellule(ligne, 3)), nombre(cellule(ligne, 4)));
        *cible = [
            ch.ok_or("consigne de chauffage absente")?,
            fr.ok_or("consigne de refroidissement absente")?,
        ];
    }

    let mut zone = [Scenario {
        hebdo: 0,
        annuel: 0,
    }; 7];
    let (mut ecs_unite, mut ecs_volume) = (String::new(), None);
    for (cible, libelle) in zone.iter_mut().zip(SCENARIOS_DE_ZONE) {
        if libelle == "besoins d'ecs" {
            // Deux lignes précèdent le libellé : l'unité, puis le volume hebdomadaire par unité
            // (« ** » en habitation : voir la fiche C_EIN_Besoins_ECS).
            ecs_unite = texte(cellule(l.ligne_de_valeur()?, 2));
            ecs_volume = nombre(cellule(l.ligne_de_valeur()?, 2));
        }
        l.chercher(&[libelle])?;
        *cible = l.scenario(hebdos, annuels)?;
    }

    let mut locaux = Vec::new();
    while l.chercher(&["local n°"]).is_ok() {
        let nom_local = texte(cellule(l.chercher(&["nom du local"])?, 2));
        let rat_l = nombre(cellule(l.chercher(&["rat_l", "ratel"])?, 2))
            .ok_or_else(|| l.erreur("ratio de surface absent"))?;
        let occupant = l.chercher(&["occupant"])?;
        let (occupants_nominal, occupants_unite) =
            (nombre(cellule(occupant, 2)), texte(cellule(occupant, 3)));
        let occupants_chaleur_w =
            nombre(cellule(l.ligne_de_valeur()?, 2)).ok_or("chaleur par occupant absente")?;
        let occupants_humidite_kg_h =
            nombre(cellule(l.ligne_de_valeur()?, 2)).ok_or("humidité par occupant absente")?;
        let occupants = l.scenario(hebdos, annuels)?;
        l.chercher(&["apports de chaleur"])?;
        let chaleur_unite = texte(cellule(l.ligne_de_valeur()?, 2));
        let chaleur_w =
            nombre(cellule(l.ligne_de_valeur()?, 2)).ok_or("apport de chaleur absent")?;
        let chaleur = l.scenario(hebdos, annuels)?;
        l.chercher(&["apports d'humidité"])?;
        let humidite_unite = texte(cellule(l.ligne_de_valeur()?, 2));
        let humidite_kg_h =
            nombre(cellule(l.ligne_de_valeur()?, 2)).ok_or("apport d'humidité absent")?;
        let humidite = l.scenario(hebdos, annuels)?;
        locaux.push(Local {
            nom: nom_local,
            rat_l,
            occupants_nominal,
            occupants_unite,
            occupants_chaleur_w,
            occupants_humidite_kg_h,
            occupants,
            chaleur_unite,
            chaleur_w,
            chaleur,
            humidite_unite,
            humidite_kg_h,
            humidite,
        });
    }
    if locaux.is_empty() {
        return Err(format!("onglet {onglet} : aucun local"));
    }
    Ok(Usage {
        numero,
        onglet: onglet.to_owned(),
        nom,
        consignes,
        zone,
        ecs_unite,
        ecs_volume,
        locaux,
    })
}

/// Lit les 28 onglets du classeur.
pub fn extraire(octets: &[u8]) -> Result<Scenarios, String> {
    let mut classeur: Xlsx<_> =
        Xlsx::new(std::io::Cursor::new(octets)).map_err(|e| e.to_string())?;
    let (mut usages, mut hebdos, mut annuels) = (Vec::new(), Vec::new(), Vec::new());
    for onglet in classeur.sheet_names() {
        let plage = classeur
            .worksheet_range(&onglet)
            .map_err(|e| format!("onglet {onglet} : {e}"))?;
        // Les lignes sont complétées à gauche pour que la colonne A du classeur ait l'indice 0.
        let (_, premiere_colonne) = plage.start().unwrap_or((0, 0));
        let lignes: Vec<Vec<Data>> = plage
            .rows()
            .map(|l| {
                std::iter::repeat_n(Data::Empty, premiere_colonne as usize)
                    .chain(l.iter().cloned())
                    .collect()
            })
            .collect();
        if !lignes.iter().any(|l| {
            texte(cellule(l, 1))
                .to_lowercase()
                .starts_with("nom de la zone")
        }) {
            continue;
        }
        usages.push(lire_onglet(&onglet, &lignes, &mut hebdos, &mut annuels)?);
    }
    usages.sort_by_key(|u| u.numero);
    let numeros: Vec<u8> = usages.iter().map(|u| u.numero).collect();
    if numeros != (1..=28).collect::<Vec<u8>>() {
        return Err(format!(
            "usages lus : {numeros:?} ; les usages 1 à 28 étaient attendus"
        ));
    }
    Ok(Scenarios {
        usages,
        hebdos,
        annuels,
    })
}

fn chaine(s: &str) -> String {
    format!("{s:?}")
}

fn option(v: Option<f64>) -> String {
    v.map_or("None".to_owned(), |x| format!("Some({x:?})"))
}

fn scenario(s: Scenario) -> String {
    format!("Scenario {{ hebdo: {}, annuel: {} }}", s.hebdo, s.annuel)
}

/// Produit le source Rust des tables.
pub fn generer(s: &Scenarios) -> String {
    let mut o = String::new();
    o.push_str("// SPDX-License-Identifier: GPL-3.0-or-later\n");
    o.push_str(
        "// Fichier généré par `cargo run -p xtask -- scenarios` : ne pas modifier à la main.\n",
    );
    o.push_str("// Source : scénarios conventionnels RE2020, classeur du 29 avril 2026\n");
    let _ = writeln!(o, "// ({SOURCE}).");
    for (onglet, ligne, colonne, brut, valeur) in CORRECTIONS {
        let _ = writeln!(o, "// Correction appliquée : onglet {onglet}, ligne {ligne}, colonne {colonne} : « {brut} » lu {valeur:?}.");
    }
    o.push_str("#![allow(clippy::unreadable_literal, clippy::excessive_precision)]\n#![cfg_attr(rustfmt, rustfmt_skip)]\n\n");
    o.push_str("use super::{\n    ApportsEquipements, ApportsOccupants, BesoinsEcs, Consignes, GrilleAnnuelle, GrilleHebdo, Scenario,\n    ScenariosLocal, ScenariosUsage,\n};\n\n");
    let _ = writeln!(
        o,
        "/// Grilles hebdomadaires distinctes : `[jour − 1][heure − 1]`."
    );
    let _ = writeln!(
        o,
        "pub static GRILLES_HEBDO: [GrilleHebdo; {}] = [",
        s.hebdos.len()
    );
    for g in &s.hebdos {
        o.push_str("    [\n");
        for jour in g {
            let _ = writeln!(o, "        {jour:?},");
        }
        o.push_str("    ],\n");
    }
    o.push_str("];\n\n");
    let _ = writeln!(o, "/// Grilles annuelles distinctes : `[semaine − 1][mois − 1]`, 0 pour une semaine inexistante.");
    let _ = writeln!(
        o,
        "pub static GRILLES_ANNUELLES: [GrilleAnnuelle; {}] = [",
        s.annuels.len()
    );
    for g in &s.annuels {
        o.push_str("    [\n");
        for semaine in g {
            let _ = writeln!(o, "        {semaine:?},");
        }
        o.push_str("    ],\n");
    }
    o.push_str("];\n\n");
    let _ = writeln!(
        o,
        "/// Scénarios des 28 usages, dans l'ordre de leur numéro (Th-BCE 2020 §4.1.3, tableau 4)."
    );
    let _ = writeln!(
        o,
        "pub static USAGES: [ScenariosUsage; {}] = [",
        s.usages.len()
    );
    for u in &s.usages {
        o.push_str("    ScenariosUsage {\n");
        let _ = writeln!(o, "        numero: {},", u.numero);
        let _ = writeln!(o, "        onglet: {},", chaine(&u.onglet));
        let _ = writeln!(o, "        nom: {},", chaine(&u.nom));
        let c = &u.consignes;
        let _ = writeln!(
            o,
            "        consignes: Consignes {{ ch_normal_c: {:?}, fr_normal_c: {:?}, ch_arret_moins_48h_c: {:?}, fr_arret_moins_48h_c: {:?}, ch_arret_plus_48h_c: {:?}, fr_arret_plus_48h_c: {:?} }},",
            c[0][0], c[0][1], c[1][0], c[1][1], c[2][0], c[2][1]
        );
        for (nom, sc) in [
            "mobilite",
            "occupation",
            "chauffage",
            "refroidissement",
            "ventilation",
            "eclairage",
        ]
        .iter()
        .zip(u.zone)
        {
            let _ = writeln!(o, "        {nom}: {},", scenario(sc));
        }
        let _ = writeln!(
            o,
            "        ecs: BesoinsEcs {{ unite: {}, volume_l_semaine_unite: {}, scenario: {} }},",
            chaine(&u.ecs_unite),
            option(u.ecs_volume),
            scenario(u.zone[6])
        );
        o.push_str("        locaux: &[\n");
        for l in &u.locaux {
            o.push_str("            ScenariosLocal {\n");
            let _ = writeln!(o, "                nom: {},", chaine(&l.nom));
            let _ = writeln!(o, "                rat_l: {:?},", l.rat_l);
            let _ = writeln!(
                o,
                "                occupants: ApportsOccupants {{ nombre_nominal: {}, unite: {}, chaleur_w: {:?}, humidite_kg_h: {:?}, scenario: {} }},",
                option(l.occupants_nominal),
                chaine(&l.occupants_unite),
                l.occupants_chaleur_w,
                l.occupants_humidite_kg_h,
                scenario(l.occupants)
            );
            let _ = writeln!(
                o,
                "                chaleur: ApportsEquipements {{ unite: {}, valeur_par_unite: {:?}, scenario: {} }},",
                chaine(&l.chaleur_unite),
                l.chaleur_w,
                scenario(l.chaleur)
            );
            let _ = writeln!(
                o,
                "                humidite: ApportsEquipements {{ unite: {}, valeur_par_unite: {:?}, scenario: {} }},",
                chaine(&l.humidite_unite),
                l.humidite_kg_h,
                scenario(l.humidite)
            );
            o.push_str("            },\n");
        }
        o.push_str("        ],\n    },\n");
    }
    o.push_str("];\n");
    o
}
