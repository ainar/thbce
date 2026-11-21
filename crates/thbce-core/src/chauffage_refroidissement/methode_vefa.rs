// SPDX-License-Identifier: GPL-3.0-or-later
//! Chauffage par défaut à effet Joule des travaux réservés par l'acquéreur (VEFA, Th-BCE 2020
//! §8.33, fiche « Sn_Methode_VEFA_TCH_Chauffage_Effet_Joule »).
//!
//! La fiche ne décrit aucun calcul propre : elle fige les paramètres de composants existants
//! (générateur direct à effet Joule de la fiche 8.18, distributions fictives des §8.7 à 8.10,
//! émetteur de la fiche 8.1), « de la génération jusqu'à l'émetteur », dont seule la puissance
//! est saisie. [`projet_avec_chauffage_vefa`] ajoute ces composants au projet avant le calcul ;
//! les éq. (1667) à (1672) sont celles du générateur à effet Joule (§8.18, éq. 1101 à 1106).
//! Lecture de la variation spatiale de l'émetteur : `docs/questions/Q251.md`.

use crate::entree::{
    Batiment, ChauffageParDefautVefa, ClasseEmetteurChaud, DistributionGroupe,
    DistributionIntergroupes, EffetJouleSaisi, Emetteur, EmissionChaud, FonctionEffetJoule,
    FonctionReseau, GenerateurSaisi, Generation, GestionTemperatureGeneration, NatureGenerateur,
    PositionGeneration, Projet, RaccordementReseaux, ReseauGroupe, ReseauIntergroupes,
    TypeEmetteurChaud, TypePriorite, VariationSpatiale, VariationTemporelle,
};

/// Préfixe des identifiants des composants ajoutés ; aucun identifiant saisi ne peut le porter.
pub const PREFIXE_ID: &str = "vefa-defaut";

/// « θvt_chem » du tableau 265 : 1,8 K, statut « valeur certifiée » (hypothèse « le couple
/// régulateur/émetteur permet un arrêt total de l'émission », §8.33.4.2).
pub const DELTA_THETA_VT_K: f64 = 1.8;

/// Identifiant de la génération ajoutée.
pub fn id_generation() -> String {
    format!("{PREFIXE_ID}-generation")
}

/// Identifiant du générateur ajouté.
pub fn id_generateur() -> String {
    format!("{PREFIXE_ID}-generateur")
}

/// Identifiant du réseau intergroupes ajouté.
pub fn id_intergroupes() -> String {
    format!("{PREFIXE_ID}-intergroupes")
}

/// Identifiant de la distribution ajoutée au groupe.
pub fn id_distribution() -> String {
    format!("{PREFIXE_ID}-distribution")
}

/// Identifiant de l'émetteur ajouté au groupe.
pub fn id_emetteur() -> String {
    format!("{PREFIXE_ID}-emetteur")
}

/// Variation spatiale de l'émetteur par défaut : classe C et hauteur sous plafond (hypothèses du
/// §8.33.4.2, tableau 260 et paramètre « Classe_Variation_Spatiale_Chaud = 1 : Classe C » du
/// tableau 265), et non les valeurs 0 / 0,6 / 1,7 / 2,8 K que le tableau 265 inscrit à côté
/// (celles de la classe B2 du tableau 82). Lecture `docs/questions/Q251.md`.
pub fn variation_spatiale(hsp_m: f64) -> VariationSpatiale {
    VariationSpatiale::Classe {
        classe: ClasseEmetteurChaud::C,
        hsp_m,
    }
}

/// Th-BCE 2020 §8.33.4.4, tableaux 261 et 262 : génération du chauffage par défaut.
fn generation(c: &ChauffageParDefautVefa) -> Generation {
    Generation {
        id: id_generation(),
        // « idraccord_gnr = 0 : avec possibilité d'isolement ».
        raccordement_reseaux: RaccordementReseaux::AvecIsolement,
        // « idpos = 0 : hors volume chauffé » ; aucun espace tampon n'est désigné.
        position: PositionGeneration::HorsVolumeChauffe {
            espace_tampon: None,
        },
        // « idtype_priorite = 1 : sans priorité ».
        priorite: Some(TypePriorite::SansPriorite),
        // « idgestion_ch = 2 : fonctionnement à la température moyenne des réseaux ».
        gestion_temperature_chauffage: Some(GestionTemperatureGeneration::Reseaux),
        gestion_temperature_refroidissement: None,
        generateurs: vec![GenerateurSaisi {
            id: id_generateur(),
            // « Rdim = 1 ».
            rdim: 1,
            // « idpriorite_ch = 1 ».
            priorite_chauffage: Some(1),
            priorite_refroidissement: None,
            priorite_ecs: None,
            // Type 500, « Id_Fougen_Mod = 1 : chauffage », « Pngen » saisie (tableau 262).
            nature: NatureGenerateur::EffetJoule(EffetJouleSaisi {
                fonction: FonctionEffetJoule::Chauffage,
                pn_gen_w: c.pn_gen_w,
            }),
        }],
        stockages: Vec::new(),
        chauffe_eau_thermodynamiques: Vec::new(),
        chauffe_eau_solaires: Vec::new(),
        systemes_solaires_combines: Vec::new(),
        chauffe_eau_solaires_collectifs: Vec::new(),
    }
}

/// Th-BCE 2020 §8.33.4.4, tableau 265 : convecteur à soufflage d'air chaud.
fn emetteur(hsp_m: f64) -> Emetteur {
    Emetteur {
        id: id_emetteur(),
        chauffage: Some(EmissionChaud {
            // « Typologie_Emetteur_Chaud = 1 : soufflage d'air chaud » : Pemconv = 0,95.
            type_emetteur: TypeEmetteurChaud::SoufflageAirChaud,
            variation_spatiale: variation_spatiale(hsp_m),
            // « Statut_Variation_Temporelle_Chaud = 0 : valeur certifiée », 1,8 K.
            variation_temporelle: VariationTemporelle::Certifie {
                delta_theta_k: DELTA_THETA_VT_K,
            },
            effet_joule: true,
            // « Rat_s_ch = 1 », « Rat_t_ch = 1 ».
            rats: 1.0,
            ratt: 1.0,
            distribution: id_distribution(),
            centrale: None,
        }),
        refroidissement: None,
        // Pertes au dos nulles (§8.33.4.2).
        pertes_au_dos: None,
        // « detection_presence = 0 : autres cas ».
        detection_presence: Some(false),
        // « Gest_vcv = 0 : pas de ventilateurs ».
        ventilateurs: None,
    }
}

/// Ajoute à une copie du bâtiment les composants du chauffage par défaut (§8.33.4.4,
/// tableaux 261 à 266) et retire la saisie `chauffage_par_defaut_vefa`. Les groupes absents
/// sont ignorés (la validation les signale).
pub fn batiment_avec_chauffage_vefa(bat: &Batiment) -> Batiment {
    let mut b = bat.clone();
    let Some(c) = b.chauffage_par_defaut_vefa.take() else {
        return b;
    };
    b.generations.push(generation(&c));
    // Tableau 264 : « Type_Prim = 0 : réseau de distribution fictif sans pertes ».
    b.distributions_intergroupes.push(DistributionIntergroupes {
        id: id_intergroupes(),
        fonction: FonctionReseau::Chauffage,
        generation: id_generation(),
        reseau: ReseauIntergroupes::Fictif,
    });
    for gv in &c.groupes {
        for z in &mut b.zones {
            for g in z.groupes.iter_mut().filter(|g| g.id == gv.groupe) {
                // Tableau 266 : « Type_2nd = 0 : réseau de distribution fictif sans pertes ».
                g.distributions.push(DistributionGroupe {
                    id: id_distribution(),
                    fonction: FonctionReseau::Chauffage,
                    distribution_intergroupe: id_intergroupes(),
                    reseau: ReseauGroupe::Fictif,
                });
                g.emetteurs.push(emetteur(gv.hsp_m));
            }
        }
    }
    b
}

/// Copie du projet où chaque chauffage par défaut est remplacé par ses composants ; `None` si
/// aucun bâtiment n'en a.
pub fn projet_avec_chauffage_vefa(projet: &Projet) -> Option<Projet> {
    if projet
        .batiments
        .iter()
        .all(|b| b.chauffage_par_defaut_vefa.is_none())
    {
        return None;
    }
    let mut p = projet.clone();
    for b in &mut p.batiments {
        *b = batiment_avec_chauffage_vefa(b);
    }
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chauffage_refroidissement::emetteurs::delta_theta_vs_ch_classe_k;

    #[test]
    fn variation_spatiale_classe_c() {
        // Tableau 260 : classe C, 0,4 / 1,2 / 2,0 / 2,8 K selon la hauteur sous plafond.
        for (hsp, attendu) in [(2.5, 0.4), (5.0, 1.2), (7.0, 2.0), (9.0, 2.8)] {
            let VariationSpatiale::Classe { classe, hsp_m } = variation_spatiale(hsp) else {
                unreachable!()
            };
            assert!((delta_theta_vs_ch_classe_k(classe, hsp_m) - attendu).abs() < 1e-12);
        }
    }
}
