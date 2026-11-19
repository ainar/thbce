// SPDX-License-Identifier: GPL-3.0-or-later
//! `thbce` : exécutable en ligne de commande du moteur Th-BCE 2020.
//!
//! Codes de retour : 0 succès ; 1 erreur d'usage ou de fichier ; 2 entrée invalide (projet ou
//! données météo) ; 3 entrée correcte mais hors du périmètre de cette version (élément ou usage
//! non pris en charge, à la validation comme au calcul) ou calcul impossible.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{ArgAction, CommandFactory, Parser, Subcommand};
use thbce_core::{
    calculer_avec, valider, Demandes, Erreur, Options, VERSION_METHODE, VERSION_MOTEUR,
};
use thbce_io::horaire as horaire_io;
use thbce_io::{json, meteo, rapport};

const CODE_USAGE: u8 = 1;
const CODE_ENTREE_INVALIDE: u8 = 2;
/// Entrée correcte mais hors du périmètre de cette version, ou calcul impossible.
const CODE_HORS_PERIMETRE: u8 = 3;

/// Gabarit de l'aide, en français.
const GABARIT_AIDE: &str = "{about-with-newline}\nUtilisation : {usage}\n\n{all-args}{after-help}";

/// Moteur de calcul Th-BCE 2020 (RE2020). Ce n'est pas le moteur réglementaire officiel et ses
/// résultats ne valent pas attestation de conformité.
#[derive(Parser)]
#[command(
    name = "thbce",
    disable_version_flag = true,
    disable_help_flag = true,
    disable_help_subcommand = true,
    help_template = GABARIT_AIDE,
    subcommand_help_heading = "Commandes",
    subcommand_value_name = "COMMANDE",
    after_help = "Codes de retour : 0 succès ; 1 erreur d'usage ou de fichier ; 2 entrée invalide ; \
                  3 entrée hors du périmètre de cette version ou calcul impossible."
)]
struct Cli {
    #[command(subcommand)]
    commande: Option<Commande>,
    /// Affiche la version du moteur et celle de la méthode implémentée.
    #[arg(short = 'V', long = "version", action = ArgAction::SetTrue)]
    version: bool,
    /// Affiche l'aide.
    #[arg(short = 'h', long = "help", action = ArgAction::Help, global = true)]
    aide: Option<bool>,
}

#[derive(Subcommand)]
enum Commande {
    /// Calcule les indicateurs d'un projet.
    #[command(help_template = GABARIT_AIDE)]
    Calcul {
        /// Description du projet, au format JSON natif.
        entree: PathBuf,
        /// Fichier météo : classeur réglementaire (.xlsx), CSV dérivé, ou fichier ouvert ADEME /
        /// Météo-France (non réglementaire). Le moteur n'embarque aucune donnée météo.
        #[arg(long, value_name = "FICHIER")]
        meteo: PathBuf,
        /// Fichier de sortie JSON ; sortie standard par défaut.
        #[arg(short = 'o', long, value_name = "FICHIER")]
        sortie: Option<PathBuf>,
        /// Restitue les valeurs en pleine précision, sans l'arrondi à une décimale du §1.3.1.
        #[arg(long)]
        sans_arrondi: bool,
        /// Écrit en plus les séries horaires principales (température extérieure, besoins et
        /// températures des groupes, électricité par poste, consommations par poste et par
        /// énergie) dans ce fichier : JSON si son nom finit par `.json`, CSV sinon. La sortie
        /// des résultats n'est pas modifiée.
        #[arg(long, value_name = "FICHIER")]
        horaire: Option<PathBuf>,
        /// Ajoute aux résultats, pour chaque bâtiment qui a des générations, les sorties par
        /// zone et par groupe de la fiche 13.2 (`batiments[].sorties_detaillees` : énergie
        /// importée et consommée par poste, production photovoltaïque attribuée, besoins, Cef,
        /// Cep, Cep,nr).
        #[arg(long)]
        sorties_detaillees: bool,
    },
    /// Valide la description d'un projet sans la calculer.
    #[command(help_template = GABARIT_AIDE)]
    Valider {
        /// Description du projet, au format JSON natif.
        entree: PathBuf,
    },
    /// Affiche le JSON Schema du format d'entrée.
    #[command(help_template = GABARIT_AIDE)]
    Schema {
        /// Affiche le schéma des résultats au lieu de celui de l'entrée.
        #[arg(long)]
        resultats: bool,
    },
    /// Outils sur les fichiers météo.
    #[command(
        help_template = GABARIT_AIDE,
        subcommand_help_heading = "Actions",
        subcommand_value_name = "ACTION"
    )]
    Meteo {
        #[command(subcommand)]
        action: ActionMeteo,
    },
    /// Affiche la version du moteur et celle de la méthode implémentée.
    #[command(help_template = GABARIT_AIDE)]
    Version,
}

#[derive(Subcommand)]
enum ActionMeteo {
    /// Convertit le classeur météo réglementaire (.xlsx) en CSV, format lu par le module
    /// WebAssembly. Le fichier produit reste soumis aux conditions d'utilisation des données :
    /// usage limité au calcul réglementaire RE2020, diffusion interdite.
    #[command(help_template = GABARIT_AIDE)]
    Convertir {
        /// Classeur officiel `4_scenarios_meteorologiques_th-bc_th-d_re2020.xlsx`.
        classeur: PathBuf,
        /// Fichier CSV à écrire.
        #[arg(short = 'o', long, value_name = "FICHIER")]
        sortie: PathBuf,
    },
}

/// Erreur fatale : message et code de retour.
struct Echec(String, u8);

/// Cause d'une erreur d'entrée-sortie, en français.
fn cause_io(e: &std::io::Error) -> String {
    use std::io::ErrorKind as K;
    match e.kind() {
        K::NotFound => "fichier ou dossier introuvable".into(),
        K::PermissionDenied => "accès refusé".into(),
        K::IsADirectory => "c'est un dossier, pas un fichier".into(),
        K::AlreadyExists => "le fichier existe déjà".into(),
        K::StorageFull => "espace disque insuffisant".into(),
        _ => format!("lecture ou écriture impossible ({e})"),
    }
}

fn lire(chemin: &Path) -> Result<Vec<u8>, Echec> {
    std::fs::read(chemin).map_err(|e| {
        Echec(
            format!("{} : {}", chemin.display(), cause_io(&e)),
            CODE_USAGE,
        )
    })
}

fn ecrire(chemin: &Path, contenu: impl AsRef<[u8]>) -> Result<(), Echec> {
    std::fs::write(chemin, contenu).map_err(|e| {
        Echec(
            format!("{} : {}", chemin.display(), cause_io(&e)),
            CODE_USAGE,
        )
    })
}

/// Code de retour d'un rapport de validation qui comporte des erreurs : 3 si elles tiennent
/// toutes au périmètre de cette version, 2 sinon.
fn code_rapport(r: &thbce_core::validation::RapportValidation) -> u8 {
    if r.seulement_hors_perimetre() {
        CODE_HORS_PERIMETRE
    } else {
        CODE_ENTREE_INVALIDE
    }
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
            sans_arrondi,
            horaire,
            sorties_detaillees,
        } => {
            let projet = lire_projet(&entree)?;
            let rapport_validation = valider(&projet);
            if !rapport_validation.est_valide() {
                return Err(Echec(
                    rapport::en_texte(&rapport_validation),
                    code_rapport(&rapport_validation),
                ));
            }
            let zone = projet.localisation.zone_climatique_retenue();
            let donnees = meteo::lire(&lire(&chemin_meteo)?, zone).map_err(|e| {
                Echec(
                    format!("{} : {e}", chemin_meteo.display()),
                    CODE_ENTREE_INVALIDE,
                )
            })?;
            let options = Options {
                arrondir: !sans_arrondi,
            };
            let demandes = Demandes {
                series: horaire.is_some(),
                sorties_detaillees,
            };
            let mut resultats =
                calculer_avec(&projet, &donnees, &options, &demandes).map_err(|e| match e {
                    Erreur::Saisie(r) => Echec(rapport::en_texte(&r), code_rapport(&r)),
                    Erreur::Meteo(_) => Echec(e.to_string(), CODE_ENTREE_INVALIDE),
                    Erreur::NonPrisEnCharge { .. } | Erreur::Calcul(_) => {
                        Echec(e.to_string(), CODE_HORS_PERIMETRE)
                    }
                })?;
            for a in &resultats.avertissements {
                eprintln!("avertissement : {a}");
            }
            if !resultats.reglementaire {
                eprintln!(
                    "avertissement : données météo non réglementaires, résultat non réglementaire"
                );
            }
            // Synthèse des indicateurs et de la conformité, sur la sortie d'erreur pour ne pas
            // mêler le JSON des résultats.
            eprint!("{}", rapport::synthese_indicateurs(&resultats));
            // Séries horaires : écrites à part, les résultats gardent leur forme par défaut.
            let series = horaire.map(|chemin| {
                let document = if chemin.extension().is_some_and(|e| e == "json") {
                    horaire_io::en_json(&resultats)
                } else {
                    horaire_io::en_csv(&resultats)
                };
                (chemin, document)
            });
            for b in &mut resultats.batiments {
                b.series_horaires = None;
            }
            if let Some((chemin, document)) = series {
                ecrire(&chemin, document)?;
            }
            let document = json::ecrire_resultats(&resultats);
            match sortie {
                Some(chemin) => ecrire(&chemin, document)?,
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
                Err(Echec(texte, code_rapport(&r)))
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
        Commande::Meteo {
            action: ActionMeteo::Convertir { classeur, sortie },
        } => {
            let csv = meteo::classeur::convertir_en_csv(&lire(&classeur)?).map_err(|e| {
                Echec(
                    format!("{} : {e}", classeur.display()),
                    CODE_ENTREE_INVALIDE,
                )
            })?;
            ecrire(&sortie, csv)?;
            eprintln!(
                "{} écrit. Ces données restent réservées au calcul réglementaire RE2020 et ne doivent pas être diffusées.",
                sortie.display()
            );
            Ok(())
        }
        Commande::Version => {
            afficher_version();
            Ok(())
        }
    }
}

fn afficher_version() {
    println!("thbce {VERSION_MOTEUR}");
    println!("méthode : {VERSION_METHODE}");
    println!("ce moteur n'est pas le moteur réglementaire officiel");
}

/// Message d'une erreur d'analyse des arguments, en français.
fn message_clap(e: &clap::Error) -> String {
    let contexte = |k: ContextKind| match e.get(k) {
        Some(ContextValue::String(s)) => s.clone(),
        Some(ContextValue::Strings(v)) => v.join(", "),
        Some(autre) => autre.to_string(),
        None => String::new(),
    };
    let message = match e.kind() {
        ErrorKind::UnknownArgument => {
            format!("argument inattendu : {}", contexte(ContextKind::InvalidArg))
        }
        ErrorKind::InvalidSubcommand => {
            format!(
                "commande inconnue : {}",
                contexte(ContextKind::InvalidSubcommand)
            )
        }
        ErrorKind::MissingRequiredArgument => {
            format!(
                "argument obligatoire manquant : {}",
                contexte(ContextKind::InvalidArg)
            )
        }
        ErrorKind::MissingSubcommand | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            "commande manquante".into()
        }
        ErrorKind::InvalidValue | ErrorKind::ValueValidation => format!(
            "valeur invalide « {} » pour {}",
            contexte(ContextKind::InvalidValue),
            contexte(ContextKind::InvalidArg)
        ),
        ErrorKind::ArgumentConflict => {
            format!(
                "arguments incompatibles : {}",
                contexte(ContextKind::InvalidArg)
            )
        }
        ErrorKind::TooManyValues | ErrorKind::WrongNumberOfValues => {
            format!(
                "nombre de valeurs incorrect pour {}",
                contexte(ContextKind::InvalidArg)
            )
        }
        _ => return e.render().to_string(),
    };
    let usage = contexte(ContextKind::Usage)
        .replace("Usage:", "Utilisation :")
        .trim()
        .to_owned();
    if usage.is_empty() {
        format!("erreur : {message}\n\nAide : thbce --help")
    } else {
        format!("erreur : {message}\n\n{usage}\n\nAide : thbce --help")
    }
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // `--help` n'est pas une erreur ; toute autre erreur d'analyse est une erreur d'usage.
            if e.use_stderr() {
                eprintln!("{}", message_clap(&e));
                return ExitCode::from(CODE_USAGE);
            }
            let _ = e.print();
            return ExitCode::SUCCESS;
        }
    };
    if cli.version {
        afficher_version();
        return ExitCode::SUCCESS;
    }
    let Some(commande) = cli.commande else {
        let _ = Cli::command().print_help();
        return ExitCode::from(CODE_USAGE);
    };
    match executer(commande) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Echec(message, code)) => {
            eprintln!("{}", message.trim_end());
            ExitCode::from(code)
        }
    }
}
