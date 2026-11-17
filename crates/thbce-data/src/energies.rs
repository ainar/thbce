// SPDX-License-Identifier: GPL-3.0-or-later
//! Coefficients de conversion des énergies : énergie primaire (Cep), énergie primaire non
//! renouvelable (Cep,nr) et facteurs d'émission (Ic énergie).
//!
//! Sources :
//! - arrêté du 4 août 2021, **article 9** (version consolidée, modifiée par l'arrêté du 6 avril
//!   2022 ; `docs/reference/texte/arrete-4-aout-2021-articles.txt`) : coefficients de
//!   transformation en énergie primaire non renouvelable (I) et en énergie primaire (II) ;
//!   recoupés avec le tableau 333 de l'annexe III (§13.2.3.1) et le tableau 11 de l'annexe II
//!   (§5.2.2) ;
//! - arrêté du 4 août 2021, **article 10** : facteurs d'émission, en kg éq. CO2 par kWh
//!   d'énergie finale PCI ; identiques à l'indicateur « Changement climatique – total » des
//!   fiches INIES « \[RE2020\] » (`docs/reference/inies/inies-energies-re2020.txt`, Q5) ;
//! - réseaux de chaleur et de froid : 5e et 6e colonnes de l'annexe 7 de l'arrêté du 15
//!   septembre 2006 ([`crate::reseaux_chaleur`]).
//!
//! Par convention (article 9, dernier alinéa), l'énergie exportée vers un réseau n'affecte ni le
//! Cep, ni le Cep,nr, ni l'Ic énergie.

/// Type d'énergie importée par le bâtiment, au sens des articles 9 et 10.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Energie {
    /// Gaz méthane (naturel) issu des réseaux.
    GazNaturel,
    /// Gaz butane ou propane (GPL) : « autres énergies » de l'article 9.
    Gpl,
    /// Fioul : « autres énergies » de l'article 9, « autres combustibles fossiles » de
    /// l'article 10.
    Fioul,
    /// Charbon : comme le fioul.
    Charbon,
    /// Bois, biomasse, selon le combustible.
    Bois(CombustibleBois),
    /// Électricité, selon l'usage (l'article 10 distingue les usages, pas l'article 9).
    Electricite(UsageElectricite),
    /// Réseau de chaleur urbain : facteur d'émission et taux d'EnR&R du réseau.
    ReseauChaleur {
        /// Facteur d'émission, en kg éq. CO2/kWh (5e colonne de l'annexe 7).
        facteur_emission_kg_kwh: f64,
        /// Taux d'énergie renouvelable et de récupération, de 0 à 1 (6e colonne).
        rat_enr: f64,
    },
    /// Réseau de froid urbain.
    ReseauFroid {
        /// Facteur d'émission, en kg éq. CO2/kWh (5e colonne de l'annexe 7).
        facteur_emission_kg_kwh: f64,
    },
}

/// Combustible bois (article 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombustibleBois {
    /// Plaquettes forestières.
    Plaquettes,
    /// Granulés (pellets) ou briquettes.
    Granules,
    /// Bûches.
    Buches,
}

/// Usage de l'électricité (article 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageElectricite {
    /// Chauffage.
    Chauffage,
    /// Refroidissement.
    Refroidissement,
    /// Eau chaude sanitaire.
    Ecs,
    /// Éclairage d'un bâtiment d'habitation.
    EclairageHabitation,
    /// Éclairage d'un bâtiment tertiaire.
    EclairageTertiaire,
    /// Autres usages (auxiliaires, déplacements…).
    AutresUsages,
}

/// Coefficient de l'électricité, en énergie primaire comme en énergie primaire non
/// renouvelable (article 9, I et II).
pub const COEF_EP_ELECTRICITE: f64 = 2.3;

/// « CoefEP » : coefficient de conversion de l'énergie finale en énergie primaire (article 9,
/// II ; annexe III, tableau 333).
pub fn coef_ep(e: Energie) -> f64 {
    match e {
        Energie::Electricite(_) => COEF_EP_ELECTRICITE,
        // Bois 1 ; réseaux urbains de chauffage et de froid 1 ; gaz naturel 1 ; autres
        // énergies 1.
        Energie::GazNaturel
        | Energie::Gpl
        | Energie::Fioul
        | Energie::Charbon
        | Energie::Bois(_)
        | Energie::ReseauChaleur { .. }
        | Energie::ReseauFroid { .. } => 1.0,
    }
}

/// Coefficient de l'énergie renouvelable captée sur le bâtiment ou la parcelle, en énergie
/// primaire comme en énergie primaire non renouvelable (article 9, I et II).
pub const COEF_EP_ENR_CAPTEE: f64 = 0.0;

/// « CoefEPnr » : coefficient de conversion de l'énergie finale en énergie primaire non
/// renouvelable (article 9, I ; annexe III, tableau 333).
pub fn coef_ep_nr(e: Energie) -> f64 {
    match e {
        Energie::Electricite(_) => COEF_EP_ELECTRICITE,
        Energie::Bois(_) => 0.0,
        // « 1 − Ratio d'énergie renouvelable ou de récupération du réseau ».
        Energie::ReseauChaleur { rat_enr, .. } => 1.0 - rat_enr,
        Energie::GazNaturel
        | Energie::Gpl
        | Energie::Fioul
        | Energie::Charbon
        | Energie::ReseauFroid { .. } => 1.0,
    }
}

/// Facteur d'émission, en kg éq. CO2 par kWh d'énergie finale PCI (article 10).
pub fn facteur_emission_kg_kwh(e: Energie) -> f64 {
    match e {
        Energie::Bois(CombustibleBois::Plaquettes) => 0.024,
        Energie::Bois(CombustibleBois::Granules | CombustibleBois::Buches) => 0.03,
        Energie::Electricite(u) => match u {
            UsageElectricite::Chauffage => 0.079,
            UsageElectricite::Refroidissement => 0.064,
            UsageElectricite::Ecs => 0.065,
            UsageElectricite::EclairageTertiaire => 0.064,
            UsageElectricite::EclairageHabitation => 0.069,
            UsageElectricite::AutresUsages => 0.064,
        },
        Energie::GazNaturel => 0.227,
        // « Gaz butane » et « Gaz propane » : 0,272.
        Energie::Gpl => 0.272,
        // « Autres combustibles fossiles ».
        Energie::Fioul | Energie::Charbon => 0.324,
        Energie::ReseauChaleur {
            facteur_emission_kg_kwh,
            ..
        }
        | Energie::ReseauFroid {
            facteur_emission_kg_kwh,
        } => facteur_emission_kg_kwh,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    /// Lit l'indicateur « Changement climatique – total » de la fiche INIES « [RE2020] » dont
    /// le nom contient `motif`.
    fn inies(motif: &str) -> f64 {
        let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/reference/inies/inies-energies-re2020.txt");
        let texte = std::fs::read_to_string(chemin).expect("fiches INIES");
        let debut = texte
            .find(motif)
            .unwrap_or_else(|| panic!("fiche INIES « {motif} » introuvable"));
        let reste = &texte[debut..];
        let ligne = reste
            .lines()
            .find(|l| l.contains("Changement climatique - total"))
            .expect("indicateur");
        ligne
            .split_whitespace()
            .last()
            .and_then(|x| x.parse().ok())
            .expect("valeur")
    }

    #[test]
    fn facteurs_de_l_article_10_egaux_aux_fiches_inies_re2020() {
        use CombustibleBois as B;
        use UsageElectricite as U;
        let cas = [
            ("bois bûches en chaudière", Energie::Bois(B::Buches)),
            ("bois bûches en poêle", Energie::Bois(B::Buches)),
            ("bois granulés en chaudière", Energie::Bois(B::Granules)),
            ("bois granulés en poêle", Energie::Bois(B::Granules)),
            ("bois plaquettes en chaudière", Energie::Bois(B::Plaquettes)),
            ("bois plaquettes en poêle", Energie::Bois(B::Plaquettes)),
            ("fioul domestique en chaudière", Energie::Fioul),
            ("gaz naturel en chaudière", Energie::GazNaturel),
            ("propane ou de butane", Energie::Gpl),
            (
                "autres usages dans un bâtiment résidentiel",
                Energie::Electricite(U::AutresUsages),
            ),
            (
                "l’usage d'ECS dans un bâtiment résidentiel",
                Energie::Electricite(U::Ecs),
            ),
            (
                "l’usage de chauffage dans un bâtiment résidentiel",
                Energie::Electricite(U::Chauffage),
            ),
            (
                "l’usage de climatisation dans un bâtiment résidentiel",
                Energie::Electricite(U::Refroidissement),
            ),
            (
                "l’usage d'éclairage dans un bâtiment résidentiel",
                Energie::Electricite(U::EclairageHabitation),
            ),
            (
                "l’usage d'éclairage dans un bâtiment tertiaire",
                Energie::Electricite(U::EclairageTertiaire),
            ),
        ];
        for (motif, e) in cas {
            assert_eq!(facteur_emission_kg_kwh(e), inies(motif), "{motif}");
        }
    }

    #[test]
    fn coefficients_de_l_article_9() {
        let elec = Energie::Electricite(UsageElectricite::Chauffage);
        assert_eq!((coef_ep(elec), coef_ep_nr(elec)), (2.3, 2.3));
        let bois = Energie::Bois(CombustibleBois::Granules);
        assert_eq!((coef_ep(bois), coef_ep_nr(bois)), (1.0, 0.0));
        let rdc = Energie::ReseauChaleur {
            facteur_emission_kg_kwh: 0.1,
            rat_enr: 0.6,
        };
        assert_eq!(coef_ep(rdc), 1.0);
        assert!((coef_ep_nr(rdc) - 0.4).abs() < 1e-12);
        for e in [Energie::GazNaturel, Energie::Gpl, Energie::Fioul] {
            assert_eq!((coef_ep(e), coef_ep_nr(e)), (1.0, 1.0));
        }
        assert_eq!(COEF_EP_ENR_CAPTEE, 0.0);
    }
}
