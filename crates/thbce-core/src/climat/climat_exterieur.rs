// SPDX-License-Identifier: GPL-3.0-or-later
//! Fiche `C_EEX_Climat extérieur` (Th-BCE 2020 §3.1).
//!
//! Attribue à chaque pas de temps les données météorologiques du projet, en fonction du
//! département et de l'altitude. « Les données de sortie de même nom que les données d'entrée
//! ont la même valeur » (§3.1.3) : rayonnements, température du ciel, vent et position du
//! soleil sont repris tels quels.

use thbce_data::temperature_base::base_ext_0_c;
use thbce_data::Departement;

use super::donnees::AnneeMeteo;

/// Altitude conventionnelle du site, `altcorr`, en m.
///
/// Th-BCE 2020 §3.1.3, « Correction d'altitude » : altitude inférieure ou égale à 400 m, site
/// considéré à 100 m ; entre 400 m et 800 m inclus, 500 m ; au-delà de 800 m, 900 m.
pub fn altcorr_m(alt_m: f64) -> f64 {
    if alt_m <= 400.0 {
        100.0
    } else if alt_m <= 800.0 {
        500.0
    } else {
        900.0
    }
}

/// `θbase_ext = θbase_ext_0 − 0,005 × altcorr` (§3.1.3), en °C.
pub fn base_ext_c(departement: Departement, altcorr_m: f64) -> f64 {
    base_ext_0_c(departement) - 0.005 * altcorr_m
}

/// `Te = Te0 − 0,005 × altcorr` (§3.1.3), en °C.
pub fn te_c(te0_c: f64, altcorr_m: f64) -> f64 {
    te0_c - 0.005 * altcorr_m
}

/// `we = (we0 − 0,0025 × altcorr) / 1000` (§3.1.3) : `we0` en g/kg d'air sec, `we` en kg/kg.
pub fn we_kg_kg(we0_g_kg: f64, altcorr_m: f64) -> f64 {
    (we0_g_kg - 0.0025 * altcorr_m) / 1000.0
}

/// `Teau = Teau0 − 0,005 × altcorr` (§3.1.3), en °C.
pub fn teau_c(teau0_c: f64, altcorr_m: f64) -> f64 {
    teau0_c - 0.005 * altcorr_m
}

/// Coefficients du polynôme d'efficacité lumineuse du rayonnement direct, du degré 6 au
/// degré 0 (§3.1.3, « Calcul des données EDn et Edi »).
const COEFFICIENTS_EDN: [f64; 7] = [
    -1.037_532_10E-08,
    2.903_122_57E-06,
    -3.318_044_23E-04,
    1.992_831_62E-02,
    -6.721_710_72E-01,
    1.246_504_45E+01,
    2.389_548_89E+00,
];

/// « EDn » : éclairement normal au rayonnement, en lux.
///
/// Th-BCE 2020 §3.1.3 : `EDn = IDn × (−1,03753210E-08·γ⁶ + 2,90312257E-06·γ⁵ − 3,31804423E-04·γ⁴
/// + 1,99283162E-02·γ³ − 6,72171072E-01·γ² + 1,24650445E+01·γ + 2,38954889E+00)`.
///
/// Le texte ne donne pas l'unité de γ dans ce polynôme. Hypothèse (question Q20) : γ en degrés,
/// seule lecture qui donne une efficacité lumineuse plausible (environ 100 lm/W au-delà de 20°).
pub fn edn_lux(idn_w_m2: f64, gamma_rad: f64) -> f64 {
    let gamma_deg = gamma_rad.to_degrees();
    // Schéma de Horner : mêmes opérations, dans le même ordre, sur toutes les plateformes.
    let efficacite = COEFFICIENTS_EDN
        .iter()
        .fold(0.0, |acc, c| acc * gamma_deg + c);
    idn_w_m2 * efficacite
}

/// « Edi » : éclairement diffus horizontal, en lux.
///
/// Th-BCE 2020 §3.1.3 : si `IDn < 1 W/m²`, `Edi = 124·Idi` ; si `IDn > 120 W/m²`,
/// `Edi = 128·Idi` ; sinon `Edi = 116·Idi`.
pub fn edi_lux(idn_w_m2: f64, idi_w_m2: f64) -> f64 {
    if idn_w_m2 < 1.0 {
        124.0 * idi_w_m2
    } else if idn_w_m2 > 120.0 {
        128.0 * idi_w_m2
    } else {
        116.0 * idi_w_m2
    }
}

/// Sorties de la fiche pour une année : séries de 8 760 valeurs, calculées une fois avant la
/// boucle horaire.
#[derive(Debug, Clone, PartialEq)]
pub struct ClimatExterieur {
    /// `altcorr`, en m.
    pub altcorr_m: f64,
    /// « θbase_ext » : température extérieure de base corrigée de l'altitude, en °C.
    pub base_ext_c: f64,
    /// « Te » : température extérieure d'air sec pour le site, en °C.
    pub te_c: Vec<f64>,
    /// « we » : poids d'eau pour le site, en kg/kg d'air sec.
    pub we_kg_kg: Vec<f64>,
    /// « Teau » : température d'alimentation en eau froide, en °C.
    pub teau_c: Vec<f64>,
    /// « IDn » : rayonnement direct normal, en W/m².
    pub idn_w_m2: Vec<f64>,
    /// « Idi » : rayonnement diffus horizontal, en W/m².
    pub idi_w_m2: Vec<f64>,
    /// « EDn » : éclairement normal au rayonnement, en lux.
    pub edn_lux: Vec<f64>,
    /// « Edi » : éclairement diffus horizontal, en lux.
    pub edi_lux: Vec<f64>,
    /// « Teciel » : température du ciel, en °C.
    pub teciel_c: Vec<f64>,
    /// « Vent » : vitesse du vent à 10 m de hauteur, en m/s.
    pub vent_m_s: Vec<f64>,
    /// « γ » : hauteur du soleil, en radians.
    pub gamma_rad: Vec<f64>,
    /// « ψ » : azimut du soleil par rapport au sud, négatif le matin, en radians.
    pub psi_rad: Vec<f64>,
}

impl ClimatExterieur {
    /// Applique la fiche à une année météo, pour un département et une altitude.
    pub fn nouveau(annee: &AnneeMeteo, departement: Departement, alt_m: f64) -> Self {
        let altcorr = altcorr_m(alt_m);
        Self {
            altcorr_m: altcorr,
            base_ext_c: base_ext_c(departement, altcorr),
            te_c: annee.te0_c.iter().map(|t| te_c(*t, altcorr)).collect(),
            we_kg_kg: annee
                .we0_g_kg
                .iter()
                .map(|w| we_kg_kg(*w, altcorr))
                .collect(),
            teau_c: annee.teau0_c.iter().map(|t| teau_c(*t, altcorr)).collect(),
            idn_w_m2: annee.idn_w_m2.clone(),
            idi_w_m2: annee.idi_w_m2.clone(),
            edn_lux: annee
                .idn_w_m2
                .iter()
                .zip(&annee.gamma_rad)
                .map(|(i, g)| edn_lux(*i, *g))
                .collect(),
            edi_lux: annee
                .idn_w_m2
                .iter()
                .zip(&annee.idi_w_m2)
                .map(|(n, d)| edi_lux(*n, *d))
                .collect(),
            teciel_c: annee.teciel_c.clone(),
            vent_m_s: annee.vent_m_s.clone(),
            gamma_rad: annee.gamma_rad.clone(),
            psi_rad: annee.psi_rad.clone(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use thbce_data::ZoneClimatique;

    use super::super::donnees::JeuMeteo;
    use super::*;

    fn proche(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn classes_d_altitude() {
        for (alt, attendu) in [
            (0.0, 100.0),
            (400.0, 100.0),
            (400.1, 500.0),
            (800.0, 500.0),
            (800.1, 900.0),
            (2500.0, 900.0),
        ] {
            assert_eq!(altcorr_m(alt), attendu, "altitude {alt}");
        }
    }

    #[test]
    fn corrections_d_altitude_calculees_a_la_main() {
        // altcorr = 500 m : Te = 10 − 2,5 = 7,5 ; we = (6 − 1,25) / 1000 ; Teau = 12 − 2,5.
        assert_eq!(te_c(10.0, 500.0), 7.5);
        assert!(proche(we_kg_kg(6.0, 500.0), 0.004_75, 1e-15));
        assert_eq!(teau_c(12.0, 500.0), 9.5);
        // Ain (01) : −9 °C à 0 m ; à 900 m, −9 − 4,5 = −13,5 °C.
        let ain = Departement::nouveau("01").unwrap();
        assert_eq!(base_ext_c(ain, 900.0), -13.5);
        // Var (83) : −3 °C ; à 100 m, −3,5 °C.
        assert_eq!(base_ext_c(Departement::nouveau("83").unwrap(), 100.0), -3.5);
    }

    #[test]
    fn eclairement_direct() {
        // γ = 0 : le polynôme se réduit à son terme constant.
        assert!(proche(edn_lux(100.0, 0.0), 238.954_889, 1e-9));
        // γ = 10° : −0,0103753210 + 0,290312257 − 3,31804423 + 19,9283162 − 67,2171072
        //           + 124,650445 + 2,38954889 = 76,713095596 lm/W.
        assert!(proche(
            edn_lux(1.0, 10f64.to_radians()),
            76.713_095_596,
            1e-9
        ));
        assert_eq!(edn_lux(0.0, 0.5), 0.0);
    }

    #[test]
    fn eclairement_diffus_par_seuils() {
        assert_eq!(edi_lux(0.0, 10.0), 1240.0);
        assert_eq!(edi_lux(0.99, 10.0), 1240.0);
        assert_eq!(edi_lux(1.0, 10.0), 1160.0);
        assert_eq!(edi_lux(120.0, 10.0), 1160.0);
        assert_eq!(edi_lux(120.01, 10.0), 1280.0);
    }

    #[test]
    fn applique_la_fiche_a_une_annee() {
        let annee = AnneeMeteo::synthetique(ZoneClimatique::H1c, JeuMeteo::ThBC);
        let c = ClimatExterieur::nouveau(&annee, Departement::nouveau("73").unwrap(), 1200.0);
        assert_eq!(c.altcorr_m, 900.0);
        assert_eq!(c.base_ext_c, -13.5);
        assert_eq!(c.te_c.len(), 8760);
        assert_eq!(c.te_c[0], 10.0 - 4.5);
        assert_eq!(c.teau_c[8759], 10.0 - 4.5);
        assert!(proche(c.we_kg_kg[0], (5.0 - 2.25) / 1000.0, 1e-15));
        assert_eq!(c.vent_m_s, annee.vent_m_s);
        assert!(c.edn_lux.iter().all(|e| *e == 0.0));
    }
}
