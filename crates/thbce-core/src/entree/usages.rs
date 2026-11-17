// SPDX-License-Identifier: GPL-3.0-or-later
//! Types d'entrée : usages des zones (Th-BCE 2020 §5.1.2).
//!
//! Un nouvel usage s'ajoute ici, comme variante de [`UsageZone`].

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Th-BCE 2020 §4.1.3, tableau 4 — usages d'une zone.
///
/// Les 28 usages sont reconnus à la lecture. La première version du moteur ne calcule que les
/// usages 1 et 2 (décision Q4) ; les autres sont signalés « non pris en charge ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UsageZone {
    /// 1. Maisons individuelles ou accolées.
    MaisonIndividuelle,
    /// 2. Logements collectifs.
    LogementCollectif,
    /// 3. Bureaux.
    Bureaux,
    /// 4. Enseignement primaire.
    EnseignementPrimaire,
    /// 5. Enseignement secondaire.
    EnseignementSecondaire,
    /// 6. Médiathèques et bibliothèques.
    MediathequeBibliotheque,
    /// 7. Bâtiments universitaires d'enseignement et de recherche, enseignements atypiques.
    Universitaire,
    /// 8. Hôtels 0, 1 et 2 étoiles (partie nuit).
    Hotel012Nuit,
    /// 9. Hôtels 3, 4 et 5 étoiles (partie nuit).
    Hotel345Nuit,
    /// 10. Hôtels 0, 1 et 2 étoiles (partie jour).
    Hotel012Jour,
    /// 11. Hôtels 3, 4 et 5 étoiles (partie jour).
    Hotel345Jour,
    /// 12. Établissements d'accueil de la petite enfance.
    PetiteEnfance,
    /// 13. Restaurants en continu, 18 heures par jour, 7 jours sur 7.
    RestaurantContinu,
    /// 14. Restaurants, 1 repas par jour, 5 jours sur 7.
    Restaurant1Repas5Jours,
    /// 15. Restaurants, 2 repas par jour, 7 jours sur 7.
    Restaurant2Repas7Jours,
    /// 16. Restaurants, 2 repas par jour, 6 jours sur 7.
    Restaurant2Repas6Jours,
    /// 17. Commerces.
    Commerces,
    /// 18. Vestiaires seuls.
    Vestiaires,
    /// 19. Établissements sanitaires avec hébergement.
    SanitaireAvecHebergement,
    /// 20. Établissements de santé (partie nuit).
    SanteNuit,
    /// 21. Établissements de santé (partie jour).
    SanteJour,
    /// 22. Aérogares.
    Aerogare,
    /// 23. Industries ou artisanats 3x8h.
    Industrie3x8,
    /// 24. Industries ou artisanats 8h à 18h.
    Industrie8h18h,
    /// 25. Établissements sportifs municipaux ou scolaires.
    SportMunicipalScolaire,
    /// 26. Restaurants scolaires, 1 repas par jour, 5 jours sur 7.
    RestaurantScolaire1Repas,
    /// 27. Restaurants scolaires, 3 repas par jour, 5 jours sur 7.
    RestaurantScolaire3Repas,
    /// 28. Établissements sportifs privés.
    SportPrive,
}

impl UsageZone {
    /// Numéro d'usage du tableau 4 (§4.1.3).
    pub fn numero(self) -> u8 {
        self as u8 + 1
    }

    /// Usages d'habitation au sens de la surface de référence (§13.1.3 : « Usage = 1 ou 2 »).
    pub fn est_habitation(self) -> bool {
        matches!(self, Self::MaisonIndividuelle | Self::LogementCollectif)
    }

    /// Usage pour lequel une partie de bâtiment peut être de catégorie de contraintes
    /// extérieures 3 (annexe de l'article R. 172-4, chapitre V, partie II ; Q66) : bureaux,
    /// hôtels, restaurants (scolaires compris), commerces, petite enfance, bâtiments
    /// universitaires et d'enseignements atypiques, médiathèques et bibliothèques, santé
    /// (partie jour), aérogares. Ce sont aussi les usages dont le tableau `DH_maxcat` a une
    /// colonne « Catégorie 3 ».
    pub fn admet_categorie_3(self) -> bool {
        matches!(self.numero(), 3 | 6..=17 | 21 | 22 | 26 | 27)
    }

    /// Année et mois à partir desquels une demande de permis de construire est soumise à la
    /// RE2020 pour cet usage (article R. 172-1 du CCH, Q68) : 1er janvier 2022 pour
    /// l'habitation, 1er juillet 2022 pour les bureaux et l'enseignement, 1er mai 2026 pour les
    /// autres usages (décret n° 2026-16 du 15 janvier 2026 ; colonne « Années 2022 à 2024 »
    /// marquée « - » dans le tableau `Icénergie_maxmoyen`).
    pub fn debut_re2020(self) -> (u16, u8) {
        match self.numero() {
            1 | 2 => (2022, 1),
            3..=5 => (2022, 7),
            _ => (2026, 5),
        }
    }

    /// Usage calculé par la version courante du moteur (décision Q4).
    pub fn est_pris_en_charge(self) -> bool {
        self.est_habitation()
    }

    /// « ihebergement » : usage d'habitation ou d'hébergement (§4.1.3, tableau 4) — usages 1 et
    /// 2, hôtels en partie nuit (8 et 9), établissements sanitaires avec hébergement (19) et
    /// établissements de santé en partie nuit (20).
    pub fn i_hebergement(self) -> bool {
        matches!(
            self,
            Self::MaisonIndividuelle
                | Self::LogementCollectif
                | Self::Hotel012Nuit
                | Self::Hotel345Nuit
                | Self::SanitaireAvecHebergement
                | Self::SanteNuit
        )
    }

    /// « ienseignement » : usage d'enseignement ou affilié (§4.1.3, tableau 4) — usages 4, 5,
    /// 7, 26 et 27.
    pub fn i_enseignement(self) -> bool {
        matches!(
            self,
            Self::EnseignementPrimaire
                | Self::EnseignementSecondaire
                | Self::Universitaire
                | Self::RestaurantScolaire1Repas
                | Self::RestaurantScolaire3Repas
        )
    }

    /// Usages « de type habitation » au sens du confort d'été (§13.5.3.3, éq. 2544) : maison
    /// individuelle, logement collectif, établissement sanitaire avec hébergement ; la
    /// température d'inconfort chaud y est ramenée à la consigne pendant la période de sommeil.
    pub fn sommeil_confort_ete(self) -> bool {
        matches!(
            self,
            Self::MaisonIndividuelle | Self::LogementCollectif | Self::SanitaireAvecHebergement
        )
    }
}
