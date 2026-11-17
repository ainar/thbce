// SPDX-License-Identifier: GPL-3.0-or-later
//! Résultats du calcul des indicateurs réglementaires d'un bâtiment (Th-BCE 2020 §13.2 à 13.4,
//! annexe II §4.3.2, annexe de l'article R. 172-4 du CCH).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::conventions::arrondir_restitution;
use crate::entree::Id;

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
    /// « Sref » de la zone, en m².
    pub sref_m2: f64,
    /// « Smoylgt » : surface moyenne des logements, en m².
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
    /// Exigences de chaque zone.
    pub zones: Vec<ExigencesZone>,
    /// Vérification de chaque exigence ; le DH est vérifié pour chaque groupe.
    pub verifications: Vec<VerificationExigence>,
    /// Vrai si toutes les exigences vérifiées sont respectées. Seules les exigences sur Bbio,
    /// Cep, Cep,nr, Ic énergie et DH sont vérifiées : `Icconstruction`, les caractéristiques
    /// minimales et les autres articles de l'arrêté sont hors du moteur.
    pub conforme: bool,
}

/// Indicateurs réglementaires d'un bâtiment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultatsIndicateurs {
    /// « SREF_bat », en m².
    pub sref_m2: f64,
    /// Électricité consommée par poste (« O_Cef_cons_[poste]_elec_annuel »), en kWh.
    pub electricite_consommee: PostesElectricite,
    /// Électricité importée par poste (« O_Cef_imp_[poste]_elec_annuel »), en kWh : la part
    /// autoconsommée de la production locale est déduite heure par heure (éq. 2535).
    pub electricite_importee: PostesElectricite,
    /// Électricité importée hors usages mobiliers, par mois des scénarios, en kWh (le forfait
    /// de refroidissement, annuel, n'y est pas compris : §13.3.3.2).
    pub electricite_importee_mensuel_kwh: Vec<f64>,
    /// Bilan de la production photovoltaïque ; absent sans installation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_pv: Option<BilanProductionPv>,
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
        if let Some(e) = &mut self.exigences {
            e.bbio_max_pts = a(e.bbio_max_pts);
            e.cep_max_kwhep_m2 = a(e.cep_max_kwhep_m2);
            e.cep_nr_max_kwhep_m2 = a(e.cep_nr_max_kwhep_m2);
            e.ic_energie_max_kgco2_m2 = a(e.ic_energie_max_kgco2_m2);
            for z in &mut e.zones {
                z.sref_m2 = a(z.sref_m2);
                z.smoy_lgt_m2 = a(z.smoy_lgt_m2);
                z.bbio_max_pts = a(z.bbio_max_pts);
                z.cep_max_kwhep_m2 = a(z.cep_max_kwhep_m2);
                z.cep_nr_max_kwhep_m2 = a(z.cep_nr_max_kwhep_m2);
                z.ic_energie_max_kgco2_m2 = a(z.ic_energie_max_kgco2_m2);
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
                v.maximum = a(v.maximum);
                v.marge = a(v.marge);
                v.marge_pct = a(v.marge_pct);
            }
        }
    }
}
