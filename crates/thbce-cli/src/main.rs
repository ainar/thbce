// SPDX-License-Identifier: GPL-3.0-or-later
//! `thbce` : exécutable en ligne de commande du moteur Th-BCE 2020.
//!
//! Codes de retour : 0 succès ; 1 erreur d'usage ou de fichier ; 2 entrée invalide (projet ou
//! données météo) ; 3 erreur de calcul.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use thbce_core::{calculer, valider, Erreur, Options, VERSION_METHODE, VERSION_MOTEUR};
use thbce_io::{json, meteo, rapport};

const CODE_USAGE: u8 = 1;
const CODE_ENTREE_INVALIDE: u8 = 2;
const CODE_ERREUR_CALCUL: u8 = 3;

/// Moteur de calcul Th-BCE 2020 (RE2020). Ce n'est pas le moteur réglementaire officiel.
#[derive(Parser)]
#[command(name = "thbce", version, disable_version_flag = true)]
struct Cli {
    #[command(subcommand)]
    commande: Commande,
}

#[derive(Subcommand)]
enum Commande {
    /// Calcule les indicateurs d'un projet.
    Calcul {
        /// Description du projet, au format JSON natif.
        entree: PathBuf,
        /// Fichier météo (CSV long au format du classeur réglementaire). Le moteur n'embarque
        /// aucune donnée météo.
        #[arg(long, value_name = "FICHIER")]
        meteo: PathBuf,
        /// Fichier de sortie JSON ; sortie standard par défaut.
        #[arg(short = 'o', long, value_name = "FICHIER")]
        sortie: Option<PathBuf>,
        /// Ajoute les séries horaires aux résultats (pas encore disponible).
        #[arg(long)]
        horaire: bool,
        /// Restitue les valeurs en pleine précision, sans l'arrondi à une décimale du §1.3.1.
        #[arg(long)]
        sans_arrondi: bool,
    },
    /// Valide la description d'un projet sans la calculer.
    Valider {
        /// Description du projet, au format JSON natif.
        entree: PathBuf,
    },
    /// Affiche le JSON Schema du format d'entrée.
    Schema {
        /// Affiche le schéma des résultats au lieu de celui de l'entrée.
        #[arg(long)]
        resultats: bool,
    },
    /// Affiche la version du moteur et celle de la méthode implémentée.
    Version,
}

/// Erreur fatale : message et code de retour.
struct Echec(String, u8);

fn lire(chemin: &Path) -> Result<Vec<u8>, Echec> {
    std::fs::read(chemin).map_err(|e| Echec(format!("{} : {e}", chemin.display()), CODE_USAGE))
}

fn lire_texte(chemin: &Path) -> Result<String, Echec> {
    String::from_utf8(lire(chemin)?).map_err(|_| {
        Echec(
            format!("{} : le fichier n'est pas un texte UTF-8", chemin.display()),
            CODE_ENTREE_INVALIDE,
        )
    })
}

fn lire_projet(chemin: &Path) -> Result<thbce_core::entree::Projet, Echec> {
    let texte = lire_texte(chemin)?;
    json::lire_projet(&texte)
        .map_err(|e| Echec(format!("{} : {e}", chemin.display()), CODE_ENTREE_INVALIDE))
}

fn executer(commande: Commande) -> Result<(), Echec> {
    match commande {
        Commande::Calcul {
            entree,
            meteo: chemin_meteo,
            sortie,
            horaire,
            sans_arrondi,
        } => {
            if horaire {
                return Err(Echec(
                    "--horaire : les séries horaires ne sont pas encore disponibles (squelette du moteur)".into(),
                    CODE_USAGE,
                ));
            }
            let projet = lire_projet(&entree)?;
            let rapport_validation = valider(&projet);
            if !rapport_validation.est_valide() {
                return Err(Echec(
                    rapport::en_texte(&rapport_validation),
                    CODE_ENTREE_INVALIDE,
                ));
            }
            let zone = projet.localisation.zone_climatique_retenue();
            let donnees = meteo::lire_csv_re2020(&lire(&chemin_meteo)?, zone).map_err(|e| {
                Echec(
                    format!("{} : {e}", chemin_meteo.display()),
                    CODE_ENTREE_INVALIDE,
                )
            })?;
            let options = Options {
                arrondir: !sans_arrondi,
            };
            let resultats = calculer(&projet, &donnees, &options).map_err(|e| match e {
                Erreur::Saisie(r) => Echec(rapport::en_texte(&r), CODE_ENTREE_INVALIDE),
                Erreur::Meteo(_) => Echec(e.to_string(), CODE_ENTREE_INVALIDE),
                Erreur::NonPrisEnCharge { .. } => Echec(e.to_string(), CODE_ERREUR_CALCUL),
            })?;
            for a in &resultats.avertissements {
                eprintln!("avertissement : {a}");
            }
            if !resultats.reglementaire {
                eprintln!(
                    "avertissement : données météo non réglementaires, résultat non réglementaire"
                );
            }
            let document = json::ecrire_resultats(&resultats);
            match sortie {
                Some(chemin) => std::fs::write(&chemin, document)
                    .map_err(|e| Echec(format!("{} : {e}", chemin.display()), CODE_USAGE))?,
                None => print!("{document}"),
            }
            Ok(())
        }
        Commande::Valider { entree } => {
            let projet = lire_projet(&entree)?;
            let r = valider(&projet);
            let texte = rapport::en_texte(&r);
            if r.est_valide() {
                print!("{texte}");
                Ok(())
            } else {
                Err(Echec(texte, CODE_ENTREE_INVALIDE))
            }
        }
        Commande::Schema { resultats } => {
            print!(
                "{}",
                if resultats {
                    json::schema_resultats()
                } else {
                    json::schema_entree()
                }
            );
            Ok(())
        }
        Commande::Version => {
            println!("thbce {VERSION_MOTEUR}");
            println!("méthode : {VERSION_METHODE}");
            println!("ce moteur n'est pas le moteur réglementaire officiel");
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // `--help` n'est pas une erreur ; toute autre erreur d'analyse est une erreur d'usage.
            let _ = e.print();
            return if e.use_stderr() {
                ExitCode::from(CODE_USAGE)
            } else {
                ExitCode::SUCCESS
            };
        }
    };
    match executer(cli.commande) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Echec(message, code)) => {
            eprintln!("{}", message.trim_end());
            ExitCode::from(code)
        }
    }
}
