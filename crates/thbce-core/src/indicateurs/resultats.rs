// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats du calcul des indicateurs réglementaires d'un bâtiment (Th-BCE 2020 §13.2 à 13.4,
//! annexe II §4.3.2, annexe de l'article R. 172-4 du CCH).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::conventions::arrondir_restitution;
use crate::entree::{CategorieContraintes, Id, ZoneBruit};

/// Électricité par poste, en kWh d'énergie finale.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct PostesElectricite {
    /// Chauffage (auxiliaires des générateurs compris).
    pub chauffage_kwh: f64,
    /// Refroidissement (forfait de refroidissement compris, §13.3).
    pub refroidissement_kwh: f64,
    /// Eau chaude sanitaire.
    pub ecs_kwh: f64,
    /// Éclairage : groupes, parties communes et parkings (éq. 2510).
    pub eclairage_kwh: f64,
    /// Auxiliaires de ventilation : centrales, ventilateurs locaux, parkings (éq. 2504, 2511).
    pub auxiliaires_ventilation_kwh: f64,
    /// Auxiliaires de distribution (éq. 2505).
    pub auxiliaires_distribution_kwh: f64,
    /// Déplacement des occupants : ascenseurs (éq. 2513).
    pub deplacements_kwh: f64,
    /// Usages mobiliers (fiche 4.7) : hors du périmètre RE2020, ils ne comptent que dans
    /// l'autoconsommation.
    pub mobilier_kwh: f64,
}

impl PostesElectricite {
    /// Total hors usages mobiliers (périmètre RE2020, éq. 2514).
    pub fn hors_mobilier_kwh(&self) -> f64 {
        self.chauffage_kwh
            + self.refroidissement_kwh
            + self.ecs_kwh
            + self.eclairage_kwh
            + self.auxiliaires_ventilation_kwh
            + self.auxiliaires_distribution_kwh
            + self.deplacements_kwh
    }

    fn champs(&mut self) -> [&mut f64; 8] {
        [
            &mut self.chauffage_kwh,
            &mut self.refroidissement_kwh,
            &mut self.ecs_kwh,
            &mut self.eclairage_kwh,
            &mut self.auxiliaires_ventilation_kwh,
            &mut self.auxiliaires_distribution_kwh,
            &mut self.deplacements_kwh,
            &mut self.mobilier_kwh,
        ]
    }

    /// Valeurs dans l'ordre des postes de [`super::bilans_energies::poste`], en kWh.
    pub fn depuis_wh(w: &[f64; 8]) -> Self {
        let mut p = Self::default();
        for (x, w) in p.champs().into_iter().zip(w) {
            *x = w / 1000.0;
        }
        p
    }

    fn arrondir(&mut self) {
        for x in self.champs() {
            *x = arrondir_restitution(*x);
        }
    }
}

/// Bilan de la production photovoltaïque du bâtiment (§13.4.3.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BilanProductionPv {
    /// « O_Eef_Prod_PV_annuel » : production, en kWh.
    pub e_prod_pv_kwh: f64,
    /// Production autoconsommée par le bâtiment, en kWh (éq. 2527 et 2530).
    pub e_prod_pv_ac_kwh: f64,
    /// « O_Eef_Elec_Exportee_annuel » : électricité exportée, en kWh (éq. 2483) ; sans effet
    /// sur Cep, Cep,nr et Ic énergie (article 9 de l'arrêté).
    pub e_exportee_kwh: f64,
    /// « O_TAC_elec_PV_annuel » : taux d'autoconsommation, en % (éq. 2485).
    pub tac_pct: f64,
    /// Production, par mois des scénarios, en kWh.
    pub e_prod_pv_mensuel_kwh: Vec<f64>,
    /// Production autoconsommée, par mois, en kWh.
    pub e_prod_pv_ac_mensuel_kwh: Vec<f64>,
}

/// Bilan de l'électricité produite par les cogénérateurs du bâtiment (fiche 8.21, §13.2.3.5 et
/// §13.4.3.5, lot 2e).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BilanProductionCogeneration {
    /// « O_Eef_Prod_Coge_annuel » : électricité produite, en kWh (éq. 2480).
    pub e_prod_coge_kwh: f64,
    /// « O_Eef_prod_coge_AC_annuel » : électricité produite autoconsommée, en kWh (éq. 2482).
    pub e_prod_coge_ac_kwh: f64,
    /// Électricité produite exportée, en kWh (part de la cogénération de l'éq. 2483) ; sans
    /// effet sur Cep, Cep,nr et Ic énergie.
    pub e_exportee_kwh: f64,
    /// « O_TAC_elec_Coge_annuel » : taux d'autoconsommation de l'électricité produite, en %
    /// (éq. 2486).
    pub tac_pct: f64,
    /// « O_TAC_elec_annuel » : taux d'autoconsommation de toute l'électricité produite
    /// (photovoltaïque et cogénération), en % (éq. 2484).
    pub tac_elec_pct: f64,
    /// « Cef_tot_comb^cogé,bat » : consommation d'énergie finale du combustible des
    /// cogénérateurs pour le chauffage et l'ECS, auxiliaires exclus, en kWh (§13.2.2).
    pub cef_tot_comb_kwh: f64,
    /// « O_TAC_Coge_global_annuel » : taux d'autoconsommation global de la chaleur et de
    /// l'électricité des cogénérateurs, en % (éq. 2455 ; annexe II §3.7.2, éq. 5).
    pub tac_global_pct: f64,
    /// Électricité produite, par mois des scénarios, en kWh (éq. 2466).
    pub e_prod_coge_mensuel_kwh: Vec<f64>,
    /// Électricité produite autoconsommée, par mois, en kWh (éq. 2468).
    pub e_prod_coge_ac_mensuel_kwh: Vec<f64>,
}

/// Forfait de consommation de refroidissement d'un groupe (§13.3, éq. 2493).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ForfaitRefroidissementGroupe {
    /// Identifiant de la zone.
    pub zone: Id,
    /// Identifiant du groupe.
    pub groupe: Id,
    /// « DHg » du groupe, en °C.h.
    pub dh_c_h: f64,
    /// Seuil haut retenu (`DH_max` du groupe), en °C.h ; absent sans catégorie de contraintes
    /// extérieures saisie (DH non plafonné, Q54).
    pub seuil_haut_c_h: Option<f64>,
    /// « Cef_fr_elec_refroidissement,g », en kWhef/(m².an) de surface de référence du groupe.
    pub cef_kwh_m2: f64,
    /// Le même, multiplié par la surface de référence du groupe, en kWh.
    pub cef_kwh: f64,
}

/// Une ligne du bilan : consommation importée d'un poste en une énergie, et sa conversion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LigneBilanEnergie {
    /// Poste : `chauffage`, `refroidissement`, `ecs`, `eclairage`,
    /// `auxiliaires_ventilation`, `auxiliaires_distribution` ou `deplacements`.
    pub poste: String,
    /// Énergie : `electricite`, `gaz_naturel`, `gpl`, `fioul`, `charbon`, `bois_buches`,
    /// `bois_granules`, `bois_plaquettes`, `reseau_chaleur` ou `reseau_froid`, suivi pour un
    /// réseau de son identifiant de l'annexe 7 (`reseau_chaleur:0102C`).
    pub energie: String,
    /// Quantité importée, en kWh d'énergie finale (PCI pour les combustibles).
    pub cef_kwh: f64,
    /// « CoefEP ».
    pub coef_ep: f64,
    /// « CoefEPnr ».
    pub coef_ep_nr: f64,
    /// Facteur d'émission, en kg éq. CO2/kWh (article 10) ; absent quand il n'est pas connu
    /// (combustible d'un poêle non saisi).
    pub facteur_emission_kg_kwh: Option<f64>,
    /// Contribution au Cep, en kWhep/(m².an).
    pub cep_kwhep_m2: f64,
    /// Contribution au Cep,nr, en kWhep/(m².an).
    pub cep_nr_kwhep_m2: f64,
    /// Contribution à l'Ic énergie, en kg éq. CO2/m².
    pub ic_energie_kgco2_m2: Option<f64>,
}

/// Exigences d'une zone (partie de bâtiment d'un usage), annexe de l'article R. 172-4.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExigencesZone {
    /// Identifiant de la zone.
    pub id: Id,
    /// Numéro d'usage.
    pub usage: u8,
    /// Catégorie de contraintes extérieures retenue pour la zone (celle de la zone si elle est
    /// saisie, celle du bâtiment sinon ; Q66).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categorie_contraintes_exterieures: Option<CategorieContraintes>,
    /// Classe d'exposition au bruit retenue pour la zone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone_bruit: Option<ZoneBruit>,
    /// « Sref » de la zone, en m².
    pub sref_m2: f64,
    /// « Smoylgt » : surface moyenne des logements, en m² ; 0 hors habitation.
    pub smoy_lgt_m2: f64,
    /// « Bbio_maxmoyen », en points.
    pub bbio_max_moyen: f64,
    /// « Mbgéo ».
    pub mb_geo: f64,
    /// « Mbcombles ».
    pub mb_combles: f64,
    /// « Mbsurf_moy ».
    pub mb_surf_moy: f64,
    /// « Mbsurf_tot ».
    pub mb_surf_tot: f64,
    /// « Mbbruit ».
    pub mb_bruit: f64,
    /// « MbHSP ».
    pub mb_hsp: f64,
    /// « Bbio_max », en points.
    pub bbio_max_pts: f64,
    /// « Cep_maxmoyen », en kWhep/(m².an).
    pub cep_max_moyen: f64,
    /// « Cep,nr_maxmoyen », en kWhep/(m².an).
    pub cep_nr_max_moyen: f64,
    /// « Icénergie_maxmoyen », en kg éq. CO2/m².
    pub ic_energie_max_moyen: f64,
    /// « Mcgéo ».
    pub mc_geo: f64,
    /// « Mccombles ».
    pub mc_combles: f64,
    /// « Mcsurf_moy ».
    pub mc_surf_moy: f64,
    /// « Mcsurf_tot ».
    pub mc_surf_tot: f64,
    /// « Mccat ».
    pub mc_cat: f64,
    /// « McHSP ».
    pub mc_hsp: f64,
    /// « Cep_max », en kWhep/(m².an).
    pub cep_max_kwhep_m2: f64,
    /// « Cep,nr_max », en kWhep/(m².an).
    pub cep_nr_max_kwhep_m2: f64,
    /// « Icénergie_max », en kg éq. CO2/m².
    pub ic_energie_max_kgco2_m2: f64,
}

/// Vérification d'une exigence : `valeur ≤ maximum`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct VerificationExigence {
    /// Indicateur : `bbio`, `cep`, `cep_nr`, `ic_energie` ou `dh`.
    pub indicateur: String,
    /// Groupe vérifié (DH : exigence par partie de bâtiment thermiquement homogène) ; absent
    /// pour un indicateur du bâtiment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groupe: Option<Id>,
    /// Valeur de l'indicateur.
    pub valeur: f64,
    /// Valeur maximale.
    pub maximum: f64,
    /// Marge : `maximum − valeur` (positive si l'exigence est respectée).
    pub marge: f64,
    /// Marge relative au maximum, en %.
    pub marge_pct: f64,
    /// Vrai si `valeur ≤ maximum`.
    pub respectee: bool,
}

/// Exigences et conformité du bâtiment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsExigences {
    /// « Bbio_max » du bâtiment, en points : moyenne des zones pondérée par leur `Sref`
    /// (chapitre II, V).
    pub bbio_max_pts: f64,
    /// « Cep_max » du bâtiment, en kWhep/(m².an).
    pub cep_max_kwhep_m2: f64,
    /// « Cep,nr_max » du bâtiment, en kWhep/(m².an).
    pub cep_nr_max_kwhep_m2: f64,
    /// « Icénergie_max » du bâtiment, en kg éq. CO2/m².
    pub ic_energie_max_kgco2_m2: f64,
    /// « Icconstruction_max » du bâtiment, en kg éq. CO2/m² (détail dans `acv.exigences`) ;
    /// absent sans données d'ACV.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ic_construction_max_kgco2_m2: Option<f64>,
    /// Exigences de chaque zone.
    pub zones: Vec<ExigencesZone>,
    /// Vérification de chaque exigence ; le DH est vérifié pour chaque groupe.
    pub verifications: Vec<VerificationExigence>,
    /// Vrai si toutes les exigences vérifiées sont respectées. Les exigences sur Bbio, Cep,
    /// Cep,nr, Ic énergie et DH sont vérifiées, et celle sur `Icconstruction` si le bâtiment a
    /// des données d'ACV (`acv`) ; les caractéristiques minimales et les autres articles de
    /// l'arrêté sont hors du moteur. Toujours faux si un
    /// indicateur exigé manque (`indicateurs_manquants`).
    pub conforme: bool,
    /// Indicateurs exigés qui n'ont pas pu être calculés ni donc vérifiés ; vide en général.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indicateurs_manquants: Vec<String>,
    /// Régime d'exigences d'une extension ou d'une construction de petite surface (arrêté du
    /// 4 août 2021, articles 3 et 50-3 ; article R. 172-1 du CCH ; Q86 à Q88) ; absent pour une
    /// construction neuve dont la `Sref` atteint 50 m². Les exigences ci-dessus (article
    /// R. 172-4) restent calculées : les exigences alternatives « peuvent être appliquées à la
    /// place » des résultats minimaux.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regime_alternatif: Option<RegimeAlternatif>,
}

/// Nature des travaux pour le choix du régime d'exigences (fiche 4.4 ; FA03 v2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NatureTravaux {
    /// Construction d'un bâtiment neuf.
    Construction,
    /// Extension horizontale d'un bâtiment existant.
    Extension,
    /// Extension verticale (surélévation).
    Surelevation,
}

/// Famille d'usage des tableaux de la FA03 v2 (article R. 172-1 du CCH, I et III ; article
/// 50-3, I) : usage principal de la partie calculée (Q86).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FamilleUsage {
    /// Maisons individuelles ou accolées (usage 1).
    MaisonIndividuelle,
    /// Logements collectifs, bureaux, enseignement primaire ou secondaire (usages 2 à 5 ;
    /// article R. 172-1, I).
    CollectifBureauxEnseignement,
    /// Bâtiments tertiaires spécifiques et industriels ou artisanaux (usages 6 à 28 ; article
    /// R. 172-1, III).
    TertiaireSpecifiqueOuIndustriel,
}

/// Régime d'exigences applicable (FA03 v2, « Que recouvrent les trois régimes des
/// extensions ? »).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RegimeExigences {
    /// Résultats minimaux de l'article R. 172-4 (« RE2020 complète » ; pour une extension, avec
    /// les adaptations de l'article 50-2, sans effet sur le calcul) : `exigences` ci-dessus.
    Complet,
    /// Exigences alternatives de l'article 50-3, II (« RE2020 intermédiaire ») : `Bbio_max`
    /// (1° de l'article R. 172-4), `Icconstruction_max` (4°, hors du moteur) et `DH_max` (5°),
    /// avec des exigences de moyens ciblées (articles 22 à 26 et 29, hors du moteur).
    Intermediaire,
    /// Exigences alternatives de l'article 50-3, III : exigences de moyens seulement (parois,
    /// ponts thermiques, émetteurs, éclairage…), aucune exigence de résultat ; hors du moteur.
    ExigencesDeMoyens,
    /// Hors du champ de la RE2020 : construction ou extension de moins de 50 m², extension de
    /// moins de 150 m² et de 30 % de l'existant, pour les usages de l'article R. 172-1, III
    /// (usages 6 à 28).
    HorsChamp,
    /// Permis déposé en 2022 : la RE2020 ne s'applique qu'à compter du 1er janvier 2023
    /// (arrêté du 4 août 2021, article 3, II) ; seule la RT 2012 s'applique.
    Rt2012,
}

/// Régime d'exigences d'une extension ou d'une construction de petite surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RegimeAlternatif {
    /// Nature des travaux.
    pub nature: NatureTravaux,
    /// Famille d'usage retenue (usage principal de la partie calculée, Q86).
    pub famille: FamilleUsage,
    /// Surface considérée, en m² : `Sref` du bâtiment calculé, plus celle des autres extensions
    /// du même bâtiment (article 50-3, I).
    pub sref_consideree_m2: f64,
    /// Part de la surface considérée dans la `Sref` des locaux existants, en % ; absente pour
    /// une construction et sans `sref_existant_m2`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_existant_pct: Option<f64>,
    /// Régime applicable.
    pub regime: RegimeExigences,
    /// Vérifications du régime intermédiaire (article 50-3, II) : Bbio et DH de chaque groupe ;
    /// vide pour les autres régimes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verifications: Vec<VerificationExigence>,
    /// Conformité aux exigences de résultat du régime que le moteur évalue : régime complet,
    /// celle des `exigences` ; régime intermédiaire, Bbio et DH (`Icconstruction` et les
    /// exigences de moyens sont hors du moteur) ; absente pour les autres régimes, qui n'ont pas
    /// d'exigence de résultat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conforme: Option<bool>,
    /// Indicateurs exigés par le régime intermédiaire qui n'ont pas pu être calculés.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indicateurs_manquants: Vec<String>,
}

/// Nombre d'occupants d'une zone et son origine (fiche 13.6.3.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OccupantsZone {
    /// Identifiant de la zone.
    pub zone: String,
    /// « Nocc_zn » : nombre d'occupants de la zone.
    pub nocc: f64,
    /// Origine : `logements` (éq. 131 et tableau 342, `Σ i·ni`), `saisie` (`nb_occupants` de
    /// la zone) ou `scenarios` (nombre maximal d'occupants conventionnel, usages tertiaires
    /// hors enseignement sans saisie, signalé dans `valeurs_par_defaut`).
    pub origine: String,
}

/// Indicateurs par occupant d'un bâtiment (fiche 13.6, annexe II §5.2.3 et §5.3.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct IndicateursOccupants {
    /// « Nocc_bat » : nombre d'occupants du bâtiment, somme de ceux des zones (éq. 2552).
    pub nocc_batiment: f64,
    /// Origine : `zones` (somme de celles des zones) ou `acv` (`acv.occupants` du bâtiment,
    /// quand le nombre d'occupants d'une zone n'est pas déterminé ; `zones` est alors vide).
    pub origine: String,
    /// Nombre d'occupants de chaque zone.
    pub zones: Vec<OccupantsZone>,
    /// « Cep_annuel_par_occ » : `Cep × SREF / Nocc`, en kWhep/(occupant.an) (éq. 2553 et 130) ;
    /// 0 si `Nocc` est nul.
    pub cep_par_occupant_kwhep: f64,
    /// `Icénergie × SREF / Nocc`, en kg éq. CO2/occupant (annexe II §5.3.1 : les indicateurs
    /// d'impact « peuvent aussi être exprimés au nombre d'occupants ») ; absent si l'Ic énergie
    /// ne l'est pas, 0 si `Nocc` est nul.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ic_energie_par_occupant_kgco2: Option<f64>,
    /// Nombres d'occupants déduits des scénarios conventionnels faute de saisie (fiche
    /// 13.6.3.1 : « peuvent être utilisées »).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub valeurs_par_defaut: Vec<crate::resultats::ValeurParDefaut>,
}

impl IndicateursOccupants {
    pub(crate) fn arrondir(&mut self) {
        let a = crate::conventions::arrondir_restitution;
        self.nocc_batiment = a(self.nocc_batiment);
        for z in &mut self.zones {
            z.nocc = a(z.nocc);
        }
        self.cep_par_occupant_kwhep = a(self.cep_par_occupant_kwhep);
        self.ic_energie_par_occupant_kgco2 = self.ic_energie_par_occupant_kgco2.map(a);
    }
}

/// Indicateurs réglementaires d'un bâtiment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsIndicateurs {
    /// « SREF_bat », en m².
    pub sref_m2: f64,
    /// Électricité consommée par poste (« O_Cef_cons_\[poste\]_elec_annuel »), en kWh.
    pub electricite_consommee: PostesElectricite,
    /// Électricité importée par poste (« O_Cef_imp_\[poste\]_elec_annuel »), en kWh : la part
    /// autoconsommée de la production locale est déduite heure par heure (éq. 2535).
    pub electricite_importee: PostesElectricite,
    /// Électricité importée hors usages mobiliers, par mois des scénarios, en kWh (le forfait
    /// de refroidissement, annuel, n'y est pas compris : §13.3.3.2).
    pub electricite_importee_mensuel_kwh: Vec<f64>,
    /// Bilan de la production photovoltaïque ; absent sans installation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_pv: Option<BilanProductionPv>,
    /// Bilan de l'électricité produite par les cogénérateurs ; absent sans cogénération.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_cogeneration: Option<BilanProductionCogeneration>,
    /// Forfaits de consommation de refroidissement des groupes non climatisés dont le DH
    /// dépasse le seuil bas (§13.3) ; absent sans forfait.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forfaits_refroidissement: Vec<ForfaitRefroidissementGroupe>,
    /// Énergie solaire thermique captée et transmise aux ballons, en kWh : énergie
    /// renouvelable captée sur le bâtiment, de coefficients nuls (article 9) ; pour information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub energie_solaire_captee_kwh: Option<f64>,
    /// Consommations importées par poste et par énergie, et leur conversion.
    pub lignes: Vec<LigneBilanEnergie>,
    /// « O_Cef_annuel » : énergie finale importée, hors usages mobiliers, en kWhef/(m².an)
    /// (éq. 2487).
    pub cef_kwhef_m2: f64,
    /// « O_Cep_annuel », en kWhep/(m².an) (éq. 2488).
    pub cep_kwhep_m2: f64,
    /// « O_Cep_nr_annuel », en kWhep/(m².an) (éq. 2489).
    pub cep_nr_kwhep_m2: f64,
    /// « Icénergie », en kg éq. CO2/m² (annexe II §4.3.2, éq. 74) ; absent si un facteur
    /// d'émission manque.
    pub ic_energie_kgco2_m2: Option<f64>,
    /// `Σ fCO2(a)` pour `a` de 1 à 50 (article 11) : poids de la consommation annuelle dans
    /// l'Ic énergie.
    pub somme_f_co2: f64,
    /// « Icénergie_annuel(a) » (annexe II §4.3.2, éq. 75), en kg éq. CO2/(m².an), pour les
    /// années `a` de 1 à 50 de la période d'étude (50 valeurs, `fCO2(a)` de l'article 11) : la
    /// somme des 50 valeurs redonne l'Ic énergie. Absent si l'Ic énergie n'est pas calculé
    /// (`docs/questions/Q117.md`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ic_energie_annuel_kgco2_m2: Vec<f64>,
    /// Indicateurs par occupant (fiche 13.6, éq. 2552 et 2553 ; annexe II §5.2.3, éq. 130) ;
    /// absents si le nombre d'occupants d'une zone n'est ni saisi ni déductible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occupants: Option<IndicateursOccupants>,
    /// Exigences et conformité ; absentes sans `exigences` dans l'entrée du bâtiment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exigences: Option<ResultatsExigences>,
}

impl ResultatsIndicateurs {
    /// Arrondit à une décimale les valeurs restituées (Th-BCE 2020 §1.3.1) ; coefficients de
    /// conversion et de modulation conservés.
    pub fn arrondir(&mut self) {
        let a = arrondir_restitution;
        self.sref_m2 = a(self.sref_m2);
        self.electricite_consommee.arrondir();
        self.electricite_importee.arrondir();
        for x in &mut self.electricite_importee_mensuel_kwh {
            *x = a(*x);
        }
        if let Some(p) = &mut self.production_pv {
            for x in [
                &mut p.e_prod_pv_kwh,
                &mut p.e_prod_pv_ac_kwh,
                &mut p.e_exportee_kwh,
                &mut p.tac_pct,
            ] {
                *x = a(*x);
            }
            for x in p
                .e_prod_pv_mensuel_kwh
                .iter_mut()
                .chain(p.e_prod_pv_ac_mensuel_kwh.iter_mut())
            {
                *x = a(*x);
            }
        }
        if let Some(p) = &mut self.production_cogeneration {
            for x in [
                &mut p.e_prod_coge_kwh,
                &mut p.e_prod_coge_ac_kwh,
                &mut p.e_exportee_kwh,
                &mut p.tac_pct,
                &mut p.tac_elec_pct,
                &mut p.cef_tot_comb_kwh,
                &mut p.tac_global_pct,
            ] {
                *x = a(*x);
            }
            for x in p
                .e_prod_coge_mensuel_kwh
                .iter_mut()
                .chain(p.e_prod_coge_ac_mensuel_kwh.iter_mut())
            {
                *x = a(*x);
            }
        }
        for f in &mut self.forfaits_refroidissement {
            f.dh_c_h = a(f.dh_c_h);
            f.cef_kwh_m2 = a(f.cef_kwh_m2);
            f.cef_kwh = a(f.cef_kwh);
        }
        self.energie_solaire_captee_kwh = self.energie_solaire_captee_kwh.map(a);
        for l in &mut self.lignes {
            l.cef_kwh = a(l.cef_kwh);
            l.cep_kwhep_m2 = a(l.cep_kwhep_m2);
            l.cep_nr_kwhep_m2 = a(l.cep_nr_kwhep_m2);
            l.ic_energie_kgco2_m2 = l.ic_energie_kgco2_m2.map(a);
        }
        self.cef_kwhef_m2 = a(self.cef_kwhef_m2);
        self.cep_kwhep_m2 = a(self.cep_kwhep_m2);
        self.cep_nr_kwhep_m2 = a(self.cep_nr_kwhep_m2);
        self.ic_energie_kgco2_m2 = self.ic_energie_kgco2_m2.map(a);
        for x in &mut self.ic_energie_annuel_kgco2_m2 {
            *x = a(*x);
        }
        if let Some(o) = &mut self.occupants {
            o.arrondir();
        }
        if let Some(e) = &mut self.exigences {
            // Valeurs maximales : produits de coefficients décimaux du texte, arrondies à 1e-9
            // près avant l'arrondi de restitution pour qu'une erreur de représentation ne fasse
            // pas tomber une demi-décimale du mauvais côté (55 × 1,15 = 63,24999999999999).
            let ax = |x: f64| a((x * 1e9).round() / 1e9);
            e.bbio_max_pts = ax(e.bbio_max_pts);
            e.cep_max_kwhep_m2 = ax(e.cep_max_kwhep_m2);
            e.cep_nr_max_kwhep_m2 = ax(e.cep_nr_max_kwhep_m2);
            e.ic_energie_max_kgco2_m2 = ax(e.ic_energie_max_kgco2_m2);
            e.ic_construction_max_kgco2_m2 = e.ic_construction_max_kgco2_m2.map(ax);
            for z in &mut e.zones {
                z.sref_m2 = a(z.sref_m2);
                z.smoy_lgt_m2 = a(z.smoy_lgt_m2);
                z.bbio_max_pts = ax(z.bbio_max_pts);
                z.cep_max_kwhep_m2 = ax(z.cep_max_kwhep_m2);
                z.cep_nr_max_kwhep_m2 = ax(z.cep_nr_max_kwhep_m2);
                z.ic_energie_max_kgco2_m2 = ax(z.ic_energie_max_kgco2_m2);
                for m in [
                    &mut z.mb_geo,
                    &mut z.mb_combles,
                    &mut z.mb_surf_moy,
                    &mut z.mb_surf_tot,
                    &mut z.mb_bruit,
                    &mut z.mb_hsp,
                    &mut z.mc_geo,
                    &mut z.mc_combles,
                    &mut z.mc_surf_moy,
                    &mut z.mc_surf_tot,
                    &mut z.mc_cat,
                    &mut z.mc_hsp,
                ] {
                    *m = (*m * 1e4).round() / 1e4;
                }
            }
            for v in &mut e.verifications {
                v.valeur = a(v.valeur);
                v.maximum = ax(v.maximum);
                v.marge = a(v.marge);
                v.marge_pct = a(v.marge_pct);
            }
            if let Some(r) = &mut e.regime_alternatif {
                r.sref_consideree_m2 = a(r.sref_consideree_m2);
                r.part_existant_pct = r.part_existant_pct.map(a);
                for v in &mut r.verifications {
                    v.valeur = a(v.valeur);
                    v.maximum = ax(v.maximum);
                    v.marge = a(v.marge);
                    v.marge_pct = a(v.marge_pct);
                }
            }
        }
    }
}
