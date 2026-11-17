// SPDX-License-Identifier: GPL-3.0-or-later
//! Données conventionnelles de l'éclairage : fiche `C_ECL_éclairage` (Th-BCE 2020 §7.1).
//!
//! Source : annexe III de l'arrêté du 4 août 2021, tableau 78 (p. 488 à 491) : coefficient
//! `C1` selon le mode de commande de l'éclairage et éclairement intérieur de référence `Eiref`,
//! par usage de zone et par type de local. Le texte renvoie à ce tableau tantôt sous le numéro
//! 78, tantôt sous le numéro 79.
//!
//! Transcription : la page 488 (usages 3 à 8) a été relue sur l'image de la page ; les pages
//! 489 à 491 (usages 9 à 28) sont transcrites de la couche de texte du PDF, que l'image de la
//! page 488 reproduit à l'identique, et restent à relire sur image avant d'ouvrir ces usages.
//! Les valeurs sont reprises telles qu'imprimées, y compris celles qui surprennent (salle de
//! réunion des bureaux à 50 lux, salle de repos de l'enseignement primaire à 0,60 puis 0,75).
//! Deux cellules de l'usage 18 sont imprimées « 0?75 » et « 0?7 » : lues 0,75 et 0,70, comme
//! les lignes « Circulation Accueil » de même première et dernière valeur.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Type de local au sens de l'éclairage (§7.1.2, « type_local » ; tableau 78).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeLocal {
    /// Bureau standard.
    BureauStandard,
    /// Salle de réunion.
    SalleReunion,
    /// Circulation, accueil.
    CirculationAccueil,
    /// Sanitaires collectifs.
    SanitairesCollectifs,
    /// Salle de classe.
    SalleClasse,
    /// Salle de repos.
    SalleRepos,
    /// Sanitaires, vestiaires.
    SanitairesVestiaires,
    /// Salle d'enseignement informatique.
    SalleEnseignementInformatique,
    /// Salle de conférence : salle polyvalente (enseignement secondaire) ou amphithéâtre
    /// (bâtiments universitaires).
    SalleConference,
    /// Centre de documentation.
    CentreDocumentation,
    /// Salle des professeurs.
    SalleProfesseurs,
    /// Salle multi-fonctions.
    SalleMultiFonctions,
    /// Local de service.
    LocalService,
    /// Chambre sans cuisine avec salle de bains.
    ChambreSansCuisine,
    /// Salle des petits déjeuners.
    SallePetitsDejeuners,
    /// Bar.
    Bar,
    /// Salle de séminaires, réunion.
    SalleSeminairesReunion,
    /// Salle de jeux.
    SalleJeux,
    /// Salle de restaurant.
    SalleRestaurant,
    /// Cuisine.
    Cuisine,
    /// Aire de vente inférieure à 300 m² (petit magasin de vente).
    AireVenteInferieure300,
    /// Aire de vente supérieure à 300 m² : local de grand volume (§7.1.3.3).
    AireVenteSuperieure300,
    /// Douches collectives.
    DouchesCollectives,
    /// Salle commune.
    SalleCommune,
    /// Locaux de soins et offices.
    LocauxSoinsOffices,
    /// Salle d'attente et de consultation (urgences, en partie nuit).
    SalleAttenteConsultation,
    /// Aire de production : local de grand volume en industrie ou artisanat (§7.1.3.3).
    AireProduction,
    /// Espace voyageurs.
    EspaceVoyageurs,
    /// Commerces (aérogares).
    Commerces,
    /// Inspection, filtrage.
    InspectionFiltrage,
    /// Salle de sport : local de grand volume (§7.1.3.3).
    SalleSport,
}

/// Ligne du tableau 78 : un type de local d'un usage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalEclairage {
    /// Numéro d'usage de la zone (tableau 4).
    pub usage: u8,
    /// Type de local.
    pub local: TypeLocal,
    /// « C1 » pour `Gest_ecl` de 1 à 4 : interrupteur manuel ; interrupteur manuel et
    /// programmation horaire ; marche et arrêt automatiques ; marche manuelle et arrêt
    /// automatique.
    pub c1: [f64; 4],
    /// « Eiref » : éclairement intérieur de référence, en lux.
    pub e_ref_lux: f64,
}

const fn l(usage: u8, local: TypeLocal, c1: [f64; 4], e_ref_lux: f64) -> LocalEclairage {
    LocalEclairage {
        usage,
        local,
        c1,
        e_ref_lux,
    }
}

/// Tableau 78 (p. 488 à 491), usages 3 à 28, dans l'ordre du texte.
#[rustfmt::skip]
pub static TABLEAU_78: [LocalEclairage; 134] = [
    // Usage 3.
    l(3, TypeLocal::BureauStandard, [0.9, 0.85, 0.8, 0.7], 5e+02),
    l(3, TypeLocal::SalleReunion, [0.7, 0.65, 0.6, 0.5], 5e+01),
    l(3, TypeLocal::CirculationAccueil, [0.8, 0.75, 0.7, 0.6], 1e+02),
    l(3, TypeLocal::SanitairesCollectifs, [0.7, 0.65, 0.6, 0.5], 2e+02),
    // Usage 4.
    l(4, TypeLocal::SalleClasse, [0.95, 0.9, 0.85, 0.75], 3e+02),
    l(4, TypeLocal::BureauStandard, [0.9, 0.85, 0.8, 0.7], 5e+02),
    l(4, TypeLocal::SalleReunion, [0.8, 0.75, 0.7, 0.6], 3e+02),
    l(4, TypeLocal::SalleRepos, [0.6, 0.75, 0.7, 0.6], 3e+02),
    l(4, TypeLocal::CirculationAccueil, [0.6, 0.55, 0.5, 0.4], 1e+02),
    l(4, TypeLocal::SanitairesVestiaires, [0.7, 0.65, 0.6, 0.5], 2e+02),
    // Usage 5.
    l(5, TypeLocal::SalleClasse, [0.95, 0.9, 0.85, 0.75], 3e+02),
    l(5, TypeLocal::SalleReunion, [0.8, 0.75, 0.7, 0.6], 3e+02),
    l(5, TypeLocal::SalleEnseignementInformatique, [0.95, 0.9, 0.85, 0.75], 3e+02),
    l(5, TypeLocal::SalleConference, [0.8, 0.75, 0.7, 0.6], 3e+02),
    l(5, TypeLocal::BureauStandard, [0.9, 0.85, 0.8, 0.7], 5e+02),
    l(5, TypeLocal::CentreDocumentation, [0.8, 0.75, 0.7, 0.6], 5e+02),
    l(5, TypeLocal::SalleProfesseurs, [0.8, 0.75, 0.7, 0.6], 3e+02),
    l(5, TypeLocal::CirculationAccueil, [0.6, 0.55, 0.5, 0.4], 1e+02),
    l(5, TypeLocal::SanitairesCollectifs, [0.7, 0.65, 0.6, 0.5], 2e+02),
    // Usage 6.
    l(6, TypeLocal::SalleMultiFonctions, [0.8, 0.75, 0.7, 0.6], 3e+02),
    l(6, TypeLocal::CentreDocumentation, [0.8, 0.75, 0.7, 0.6], 5e+02),
    l(6, TypeLocal::LocalService, [0.06, 0.05, 0.04, 0.02], 2e+02),
    l(6, TypeLocal::BureauStandard, [0.9, 0.85, 0.8, 0.7], 5e+02),
    l(6, TypeLocal::SalleReunion, [0.8, 0.75, 0.7, 0.6], 3e+02),
    l(6, TypeLocal::CirculationAccueil, [0.8, 0.75, 0.7, 0.6], 1e+02),
    l(6, TypeLocal::SanitairesCollectifs, [0.7, 0.65, 0.6, 0.5], 2e+02),
    // Usage 7.
    l(7, TypeLocal::SalleClasse, [0.95, 0.9, 0.85, 0.75], 3e+02),
    l(7, TypeLocal::SalleConference, [0.8, 0.75, 0.7, 0.6], 3e+02),
    l(7, TypeLocal::SalleEnseignementInformatique, [0.95, 0.9, 0.85, 0.75], 3e+02),
    l(7, TypeLocal::CentreDocumentation, [0.8, 0.75, 0.7, 0.6], 5e+02),
    l(7, TypeLocal::BureauStandard, [0.9, 0.85, 0.8, 0.7], 5e+02),
    l(7, TypeLocal::SalleReunion, [0.8, 0.75, 0.7, 0.6], 3e+02),
    l(7, TypeLocal::CirculationAccueil, [0.6, 0.55, 0.5, 0.4], 1e+02),
    l(7, TypeLocal::SanitairesCollectifs, [0.7, 0.65, 0.6, 0.5], 2e+02),
    // Usage 8.
    l(8, TypeLocal::ChambreSansCuisine, [0.6, 0.55, 0.5, 0.4], 7e+01),
    l(8, TypeLocal::SanitairesCollectifs, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(8, TypeLocal::LocalService, [0.06, 0.05, 0.04, 0.02], 2e+02),
    l(8, TypeLocal::CirculationAccueil, [0.8, 0.75, 0.7, 0.6], 1e+02),
    // Usage 9.
    l(9, TypeLocal::ChambreSansCuisine, [0.6, 0.55, 0.5, 0.4], 7e+01),
    l(9, TypeLocal::SanitairesCollectifs, [1.0, 0.95, 0.9, 0.8], 2e+02),
    l(9, TypeLocal::LocalService, [0.06, 0.05, 0.04, 0.02], 2e+02),
    l(9, TypeLocal::CirculationAccueil, [0.8, 0.75, 0.7, 0.6], 1e+02),
    // Usage 10.
    l(10, TypeLocal::BureauStandard, [0.8, 0.75, 0.7, 0.6], 5e+02),
    l(10, TypeLocal::SanitairesCollectifs, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(10, TypeLocal::CirculationAccueil, [0.8, 0.75, 0.7, 0.6], 1e+02),
    l(10, TypeLocal::SallePetitsDejeuners, [1.0, 1.0, 1.0, 1.0], 2e+02),
    // Usage 11.
    l(11, TypeLocal::BureauStandard, [0.9, 0.85, 0.8, 0.7], 5e+02),
    l(11, TypeLocal::SanitairesCollectifs, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(11, TypeLocal::CirculationAccueil, [0.8, 0.75, 0.7, 0.6], 1e+02),
    l(11, TypeLocal::Bar, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(11, TypeLocal::SallePetitsDejeuners, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(11, TypeLocal::SalleSeminairesReunion, [0.8, 0.75, 0.7, 0.6], 5e+02),
    // Usage 12.
    l(12, TypeLocal::SalleJeux, [0.95, 0.9, 0.85, 0.75], 3e+02),
    l(12, TypeLocal::SalleRepos, [0.8, 0.75, 0.7, 0.6], 3e+02),
    l(12, TypeLocal::BureauStandard, [0.9, 0.85, 0.8, 0.7], 5e+02),
    l(12, TypeLocal::SalleReunion, [0.8, 0.75, 0.7, 0.6], 5e+02),
    l(12, TypeLocal::CirculationAccueil, [0.6, 0.55, 0.5, 0.4], 1e+02),
    l(12, TypeLocal::SanitairesVestiaires, [0.7, 0.65, 0.6, 0.5], 2e+02),
    // Usage 13.
    l(13, TypeLocal::SalleRestaurant, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(13, TypeLocal::Cuisine, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(13, TypeLocal::LocalService, [0.06, 0.05, 0.04, 0.02], 2e+02),
    // Usage 14.
    l(14, TypeLocal::SalleRestaurant, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(14, TypeLocal::Cuisine, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(14, TypeLocal::LocalService, [0.06, 0.05, 0.04, 0.02], 2e+02),
    // Usage 15.
    l(15, TypeLocal::SalleRestaurant, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(15, TypeLocal::Cuisine, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(15, TypeLocal::LocalService, [0.06, 0.05, 0.04, 0.02], 2e+02),
    // Usage 16.
    l(16, TypeLocal::SalleRestaurant, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(16, TypeLocal::Cuisine, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(16, TypeLocal::LocalService, [0.06, 0.05, 0.04, 0.02], 2e+02),
    // Usage 17.
    l(17, TypeLocal::AireVenteInferieure300, [1.0, 1.0, 1.0, 1.0], 3e+02),
    l(17, TypeLocal::AireVenteSuperieure300, [1.0, 1.0, 1.0, 1.0], 3e+02),
    l(17, TypeLocal::CirculationAccueil, [1.0, 1.0, 1.0, 1.0], 3e+02),
    l(17, TypeLocal::SanitairesCollectifs, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(17, TypeLocal::DouchesCollectives, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(17, TypeLocal::LocalService, [0.06, 0.05, 0.04, 0.02], 2e+02),
    // Usage 18.
    l(18, TypeLocal::CirculationAccueil, [0.8, 0.75, 0.7, 0.6], 1e+02),
    l(18, TypeLocal::SanitairesCollectifs, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(18, TypeLocal::DouchesCollectives, [0.7, 0.65, 0.6, 0.5], 2e+02),
    // Usage 19.
    l(19, TypeLocal::ChambreSansCuisine, [1.0, 1.0, 1.0, 1.0], 7e+01),
    l(19, TypeLocal::CirculationAccueil, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(19, TypeLocal::LocalService, [0.06, 0.05, 0.04, 0.02], 2e+02),
    l(19, TypeLocal::SanitairesCollectifs, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(19, TypeLocal::SalleCommune, [0.8, 0.75, 0.7, 0.6], 5e+02),
    l(19, TypeLocal::BureauStandard, [0.9, 0.85, 0.8, 0.7], 5e+02),
    // Usage 20.
    l(20, TypeLocal::ChambreSansCuisine, [1.0, 1.0, 1.0, 1.0], 7e+01),
    l(20, TypeLocal::DouchesCollectives, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(20, TypeLocal::SanitairesCollectifs, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(20, TypeLocal::CirculationAccueil, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(20, TypeLocal::LocauxSoinsOffices, [1.0, 1.0, 1.0, 1.0], 5e+02),
    l(20, TypeLocal::BureauStandard, [0.9, 0.85, 0.8, 0.7], 5e+02),
    l(20, TypeLocal::SalleAttenteConsultation, [1.0, 1.0, 1.0, 1.0], 5e+02),
    l(20, TypeLocal::AireProduction, [1.0, 0.95, 0.9, 0.8], 5e+02),
    // Usage 21.
    l(21, TypeLocal::AireProduction, [1.0, 0.95, 0.9, 0.8], 5e+02),
    l(21, TypeLocal::SanitairesCollectifs, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(21, TypeLocal::CirculationAccueil, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(21, TypeLocal::DouchesCollectives, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(21, TypeLocal::SalleAttenteConsultation, [1.0, 1.0, 1.0, 1.0], 5e+02),
    l(21, TypeLocal::BureauStandard, [0.9, 0.85, 0.8, 0.7], 5e+02),
    l(21, TypeLocal::SalleReunion, [0.7, 0.65, 0.6, 0.5], 5e+02),
    // Usage 22.
    l(22, TypeLocal::EspaceVoyageurs, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(22, TypeLocal::CirculationAccueil, [1.0, 0.75, 0.7, 0.6], 2e+02),
    l(22, TypeLocal::Commerces, [1.0, 1.0, 1.0, 1.0], 3e+02),
    l(22, TypeLocal::BureauStandard, [0.9, 0.85, 0.8, 0.7], 5e+02),
    l(22, TypeLocal::InspectionFiltrage, [1.0, 1.0, 1.0, 1.0], 5e+02),
    l(22, TypeLocal::SanitairesVestiaires, [0.7, 0.65, 0.6, 0.5], 2e+02),
    // Usage 23.
    l(23, TypeLocal::BureauStandard, [0.9, 0.85, 0.8, 0.7], 5e+02),
    l(23, TypeLocal::CirculationAccueil, [0.8, 0.75, 0.7, 0.6], 1e+02),
    l(23, TypeLocal::AireProduction, [1.0, 1.0, 1.0, 1.0], 3e+02),
    l(23, TypeLocal::SanitairesVestiaires, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(23, TypeLocal::DouchesCollectives, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(23, TypeLocal::LocalService, [0.06, 0.05, 0.04, 0.02], 2e+02),
    // Usage 24.
    l(24, TypeLocal::BureauStandard, [0.9, 0.85, 0.8, 0.7], 5e+02),
    l(24, TypeLocal::CirculationAccueil, [0.8, 0.75, 0.7, 0.6], 1e+02),
    l(24, TypeLocal::AireProduction, [1.0, 1.0, 1.0, 1.0], 3e+02),
    l(24, TypeLocal::SanitairesVestiaires, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(24, TypeLocal::DouchesCollectives, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(24, TypeLocal::LocalService, [0.06, 0.05, 0.04, 0.02], 2e+02),
    // Usage 25.
    l(25, TypeLocal::SalleSport, [0.9, 0.85, 0.8, 0.7], 3e+02),
    l(25, TypeLocal::CirculationAccueil, [0.8, 0.75, 0.7, 0.6], 1e+02),
    l(25, TypeLocal::SanitairesCollectifs, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(25, TypeLocal::DouchesCollectives, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(25, TypeLocal::LocalService, [0.06, 0.05, 0.04, 0.02], 2e+02),
    // Usage 26.
    l(26, TypeLocal::SalleRestaurant, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(26, TypeLocal::Cuisine, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(26, TypeLocal::LocalService, [0.06, 0.05, 0.04, 0.02], 2e+02),
    // Usage 27.
    l(27, TypeLocal::SalleRestaurant, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(27, TypeLocal::Cuisine, [1.0, 1.0, 1.0, 1.0], 2e+02),
    l(27, TypeLocal::LocalService, [0.06, 0.05, 0.04, 0.02], 2e+02),
    // Usage 28.
    l(28, TypeLocal::SalleSport, [0.9, 0.85, 0.8, 0.7], 3e+02),
    l(28, TypeLocal::CirculationAccueil, [0.8, 0.75, 0.7, 0.6], 1e+02),
    l(28, TypeLocal::SanitairesVestiaires, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(28, TypeLocal::DouchesCollectives, [0.7, 0.65, 0.6, 0.5], 2e+02),
    l(28, TypeLocal::LocalService, [0.06, 0.05, 0.04, 0.02], 2e+02),
];

/// Ligne du tableau 78 d'un type de local dans un usage ; `None` si l'usage n'a pas ce type de
/// local (ou pour les usages 1 et 2, dont l'éclairage est conventionnel, §7.1.3.6.2).
pub fn local(usage: u8, type_local: TypeLocal) -> Option<&'static LocalEclairage> {
    TABLEAU_78
        .iter()
        .find(|l| l.usage == usage && l.local == type_local)
}

/// Types de locaux d'un usage, dans l'ordre du tableau 78.
pub fn locaux(usage: u8) -> impl Iterator<Item = &'static LocalEclairage> {
    TABLEAU_78.iter().filter(move |l| l.usage == usage)
}

/// Local de grand volume par convention (§7.1.3.3) : salle de sport des établissements
/// sportifs (usages 25 et 28), aire de production des industries (23 et 24), aire de vente
/// de plus de 300 m² des commerces (17).
pub fn est_grand_volume(usage: u8, type_local: TypeLocal) -> bool {
    matches!(
        (usage, type_local),
        (25 | 28, TypeLocal::SalleSport)
            | (23 | 24, TypeLocal::AireProduction)
            | (17, TypeLocal::AireVenteSuperieure300)
    )
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn tableau_78_bureaux() {
        // Relu sur l'image de la page 488.
        let b = local(3, TypeLocal::BureauStandard).unwrap();
        assert_eq!((b.c1, b.e_ref_lux), ([0.90, 0.85, 0.80, 0.70], 500.0));
        let r = local(3, TypeLocal::SalleReunion).unwrap();
        assert_eq!((r.c1, r.e_ref_lux), ([0.70, 0.65, 0.60, 0.50], 50.0));
        let c = local(3, TypeLocal::CirculationAccueil).unwrap();
        assert_eq!((c.c1, c.e_ref_lux), ([0.80, 0.75, 0.70, 0.60], 100.0));
        let s = local(3, TypeLocal::SanitairesCollectifs).unwrap();
        assert_eq!((s.c1, s.e_ref_lux), ([0.70, 0.65, 0.60, 0.50], 200.0));
        assert_eq!(locaux(3).count(), 4);
        assert!(local(3, TypeLocal::SalleClasse).is_none());
        assert!(local(1, TypeLocal::BureauStandard).is_none());
    }

    #[test]
    fn tableau_78_coherent() {
        // 26 usages, de 3 à 28, chacun avec au moins trois types de locaux distincts.
        for usage in 3..=28 {
            let lignes: Vec<_> = locaux(usage).collect();
            assert!(lignes.len() >= 3, "usage {usage}");
            for (i, a) in lignes.iter().enumerate() {
                assert!(
                    lignes[i + 1..].iter().all(|b| b.local != a.local),
                    "usage {usage}"
                );
                assert!(a.c1.iter().all(|c| (0.0..=1.0).contains(c)));
                assert!(a.e_ref_lux > 0.0);
                // C1 ne croît pas avec le degré d'automatisme, hors la ligne « salle de repos »
                // de l'enseignement primaire, imprimée 0,60 ; 0,75 ; 0,70 ; 0,60.
                if (usage, a.local) != (4, TypeLocal::SalleRepos) {
                    assert!(a.c1.windows(2).all(|w| w[1] <= w[0]), "usage {usage}");
                }
            }
        }
        assert_eq!(TABLEAU_78.len(), 134);
        assert!(est_grand_volume(25, TypeLocal::SalleSport));
        assert!(!est_grand_volume(3, TypeLocal::BureauStandard));
        assert!(!est_grand_volume(17, TypeLocal::AireVenteInferieure300));
    }
}
