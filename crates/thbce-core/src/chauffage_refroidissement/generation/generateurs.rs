// SPDX-License-Identifier: GPL-3.0-or-later
//! Modèles des générateurs (Th-BCE 2020 §8.17.3.2, tableau 111) et répartition de leurs appels.
//!
//! Un nouveau générateur ajoute sa variante à [`Generateur`] et ses branches aux méthodes de
//! [`Generateur`] ; son modèle reste dans son propre module de `chauffage_refroidissement`.

use super::{EntreesAppel, FonctionGenerateur, SortiesAppel};
use crate::chauffage_refroidissement::calculs_generation::Poste;
use crate::chauffage_refroidissement::chaudieres_gaz_fioul_bois::Chaudiere;
use crate::chauffage_refroidissement::generateur_direct_a_effet_joule::GenerateurEffetJoule;
use crate::chauffage_refroidissement::pac_moteur_gaz::PacMoteurGaz;
use crate::chauffage_refroidissement::poeles_et_inserts::Poele;
use crate::chauffage_refroidissement::reseau_de_chaleur_et_de_froid::ReseauChaleurFroid;
use crate::chauffage_refroidissement::thermodynamique_elec::GenerateurThermodynamique;
use crate::chauffage_refroidissement::thermodynamique_gaz::MachineAbsorption;

/// Générateur idéal : fournit exactement la demande, sans limite de puissance, avec un
/// rendement unitaire, en consommant de l'électricité.
///
/// Générateur de test, absent du format d'entrée : il a tenu lieu de génération dans les lots
/// 6a et 6b, avant la description des générateurs, et sert aux tests de la gestion de la
/// génération. Le texte ne prévoit pas de générateur par défaut.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenerateurIdeal {
    /// Fonction du générateur.
    pub fonction: FonctionGenerateur,
}

impl GenerateurIdeal {
    fn appel(&self, e: &EntreesAppel) -> SortiesAppel {
        let q = e.q_req_wh.max(0.0);
        SortiesAppel {
            q_cef_wh: q,
            q_cons_wh: q,
            q_fou_wh: q,
            eta_eff: 1.0,
            tau_charge: 0.0,
            ..SortiesAppel::default()
        }
    }
}

/// Modèle d'un générateur (tableau 111).
#[derive(Debug, Clone, PartialEq)]
pub enum Generateur {
    /// Type 500 : générateur direct à effet Joule (§8.18).
    EffetJoule(GenerateurEffetJoule),
    /// Type 503 : générateur thermodynamique à compression électrique (§8.23).
    Thermodynamique(Box<GenerateurThermodynamique>),
    /// Types 100 à 102, 200, 201 et 400 : chaudière au gaz, au fioul ou au bois (§8.19).
    Chaudiere(Box<Chaudiere>),
    /// Types 403 et 404 : poêle ou insert à bois (§8.22).
    Poele(Poele),
    /// Types 600 et 601 : sous-station de réseau de chaleur ou de froid (§8.28).
    Reseau(ReseauChaleurFroid),
    /// Type 504 : machine à absorption au gaz (§8.24).
    Absorption(Box<MachineAbsorption>),
    /// Type 504 : pompe à chaleur à moteur gaz (§8.25).
    MoteurGaz(Box<PacMoteurGaz>),
    /// Générateur idéal, pour les tests.
    Ideal(GenerateurIdeal),
}

impl Generateur {
    /// « idfougen ».
    pub fn fonction(&self) -> FonctionGenerateur {
        match self {
            Self::EffetJoule(g) => FonctionGenerateur::mono(g.fonction),
            Self::Thermodynamique(g) => g.fonction(),
            Self::Chaudiere(g) => match (g.chauffage, g.ecs) {
                (true, true) => FonctionGenerateur::ChauffageEcs,
                (false, true) => FonctionGenerateur::Ecs,
                _ => FonctionGenerateur::Chauffage,
            },
            Self::Poele(_) => FonctionGenerateur::Chauffage,
            Self::Reseau(g) => match (g.froid, g.chauffage, g.ecs) {
                (true, _, _) => FonctionGenerateur::Refroidissement,
                (_, true, true) => FonctionGenerateur::ChauffageEcs,
                (_, false, true) => FonctionGenerateur::Ecs,
                _ => FonctionGenerateur::Chauffage,
            },
            Self::Absorption(g) => g.fonction(),
            Self::MoteurGaz(g) => g.fonction(),
            Self::Ideal(g) => g.fonction,
        }
    }

    /// « Rdim × Pngen_[po.] » : puissance nominale installée pour un poste, en W (éq. 1017).
    pub fn pn_gen_tot_w(&self, poste: Poste) -> f64 {
        if !self.fonction().assure(poste) {
            return 0.0;
        }
        match self {
            Self::EffetJoule(g) => g.p_max_w(),
            Self::Thermodynamique(g) => g.pn_gen_tot_w(poste),
            Self::Chaudiere(g) => g.pn_gen_tot_w(),
            Self::Poele(g) => g.pn_gen_tot_w(),
            Self::Reseau(g) => g.pn_gen_tot_w(),
            Self::Absorption(g) => g.pn_gen_tot_w(),
            Self::MoteurGaz(g) => g.pn_gen_tot_w(poste),
            // Sans puissance nominale : parts égales en mode sans priorité.
            Self::Ideal(_) => 1.0,
        }
    }

    /// Fonction d'appel du générateur (figure 112).
    pub fn appel(&mut self, e: &EntreesAppel) -> SortiesAppel {
        match self {
            Self::EffetJoule(g) => g.appel(e),
            Self::Thermodynamique(g) => g.appel(e),
            Self::Chaudiere(g) => g.appel(e),
            Self::Poele(g) => g.appel(e),
            Self::Reseau(g) => g.appel(e),
            Self::Absorption(g) => g.appel(e),
            Self::MoteurGaz(g) => g.appel(e),
            Self::Ideal(g) => g.appel(e),
        }
    }

    /// Fin du pas de temps : les générateurs à mémoire d'un pas de temps (chaudières,
    /// éq. 1089, 1105 et 1106) conservent l'état du pas qui s'achève.
    pub fn fin_de_pas(&mut self) {
        if let Self::Chaudiere(g) = self {
            g.fin_de_pas();
        }
    }
}
