// SPDX-License-Identifier: GPL-3.0-or-later
//! Gestion et régulation des espaces tampons solarisés : fiche `Gestion/Régulation des espaces
//! tampons solarisés` (Th-BCE 2020 §5.3, A3 p. 108 à 117).
//!
//! À chaque pas de temps : saison au sens de l'espace tampon (éq. 96), indicateur
//! d'hébergement (éq. 97), température de référence de la gestion (éq. 98), position des
//! protections mobiles des parois vitrées (éq. 99 et 100, figure 16, tableau 16), autorisation
//! d'ouverture (éq. 102, tableau 17), période de sommeil (éq. 103), ratio d'ouverture requis
//! (éq. 104, figure 17, tableau 18) et effectif (éq. 105).
//!
//! Lectures du texte : `docs/questions/Q041.md`.

use thbce_data::protections_mobiles::V_VENT_LIM_MANU_SE_M_S;

/// « Saisonet(j) » : saison au sens de l'espace tampon (§5.3.3.1.1, éq. 96).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaisonEt {
    /// 1 : saison de chauffage.
    Chauffage,
    /// 2 : mi-saison.
    MiSaison,
    /// 3 : saison de refroidissement.
    Refroidissement,
    /// 4 : saison mixte.
    Mixte,
}

/// « Saisonet(j) » : saison de l'espace tampon d'après les autorisations propres des groupes
/// qui sont en contact avec lui.
///
/// Th-BCE 2020 §5.3.3.1.1, éq. (96) : saison mixte si `Σ Autch,pro > 0` et `Σ Autfr,pro > 0` ;
/// sinon chauffage si `Σ Autch,pro > 0` ; sinon refroidissement si `Σ Autfr,pro > 0` ; sinon
/// mi-saison. `autorisations` donne, pour chaque groupe, `(Autch,pro(j), Autfr,pro(j))`.
pub fn saison_et(autorisations: impl IntoIterator<Item = (bool, bool)>) -> SaisonEt {
    let (mut ch, mut fr) = (false, false);
    for (a_ch, a_fr) in autorisations {
        ch |= a_ch;
        fr |= a_fr;
    }
    match (ch, fr) {
        (true, true) => SaisonEt::Mixte,
        (true, false) => SaisonEt::Chauffage,
        (false, true) => SaisonEt::Refroidissement,
        (false, false) => SaisonEt::MiSaison,
    }
}

/// « ihebergement_et » : vrai si tous les groupes en contact avec l'espace tampon sont
/// résidentiels ou d'hébergement.
///
/// Th-BCE 2020 §5.3.3.1.2, éq. (97) : `ihebergement_et = MIN(ihebergement^gr)`. Sans groupe, faux.
pub fn i_hebergement_et(groupes: impl IntoIterator<Item = bool>) -> bool {
    let mut n = 0;
    let mut tous = true;
    for g in groupes {
        n += 1;
        tous &= g;
    }
    n > 0 && tous
}

/// « θet_regul(h) » : température de référence de la gestion de l'espace tampon, en °C.
///
/// Th-BCE 2020 §5.3.3.2, éq. (98) :
/// `θet_regul(h) = MAX(θet(h−1) ; MAX(iocc^gr(h)·θop,fin^gr(h−1)))`, sur les groupes liés à
/// l'espace tampon. `groupes` donne `(iocc^gr(h), θop,fin^gr(h−1))`.
pub fn theta_et_regul_c(theta_et_h1_c: f64, groupes: impl IntoIterator<Item = (f64, f64)>) -> f64 {
    groupes
        .into_iter()
        .map(|(i_occ, theta_op)| i_occ * theta_op)
        .fold(theta_et_h1_c, f64::max)
}

/// Consignes de la gestion des protections mobiles, en °C (§5.3.3.3, tableau 16).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConsignesProtections {
    /// « θet,i_pm_limb » : en dessous, protections relevées.
    pub limb_c: f64,
    /// « θet,i_pm_limh » : au-dessus, protections baissées.
    pub limh_c: f64,
}

/// Th-BCE 2020 §5.3.3.3, tableau 16 (relu sur l'image de la p. 113) : 28 et 30 °C en saison de
/// chauffage ; 26 et 28 °C en saison mixte ou en mi-saison ; 24 et 26 °C en saison de
/// refroidissement.
pub fn consignes_protections(saison: SaisonEt) -> ConsignesProtections {
    let (limb_c, limh_c) = match saison {
        SaisonEt::Chauffage => (28.0, 30.0),
        SaisonEt::MiSaison | SaisonEt::Mixte => (26.0, 28.0),
        SaisonEt::Refroidissement => (24.0, 26.0),
    };
    ConsignesProtections { limb_c, limh_c }
}

/// « Ventc_lim » : vitesse de vent au-delà de laquelle les protections extérieures de l'espace
/// tampon sont remontées, en m/s.
///
/// Le tableau 15 (§5.3.2) nomme ce paramètre sans lui donner de valeur. Les protections de
/// l'espace tampon sont « conventionnellement de type stores enroulables » en gestion manuelle
/// (§5.3.3.3) : retenu (décision Q41 point 3), le seuil des stores enroulables extérieurs en
/// gestion manuelle de la fiche 5.9, `Vventlim_manu_se` = 10 m/s.
pub const V_VENTC_LIM_ET_M_S: f64 = V_VENT_LIM_MANU_SE_M_S;

/// Données d'un pas de temps pour la position des protections mobiles de l'espace tampon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionsProtectionsEt {
    /// « idprot » : présence de protections mobiles.
    pub id_prot: bool,
    /// « Protet_ext » : protections extérieures.
    pub prot_ext: bool,
    /// « Σ Etp*n » : éclairement total des parois de l'espace tampon, en lux.
    pub somme_etp_lux: f64,
    /// « Ventc(h) » : vitesse du vent, en m/s.
    pub v_ventc_m_s: f64,
    /// « θet_regul(h) », en °C.
    pub theta_regul_c: f64,
    /// Saison au sens de l'espace tampon.
    pub saison: SaisonEt,
}

/// « Rprotet(h) » : position des protections mobiles de l'espace tampon (0 relevées, 1
/// baissées).
///
/// Th-BCE 2020 §5.3.3.3, éq. (99) : nulle sans protection. Éq. (100), figure 16 : de jour
/// (`Σ Etp*n > 0`), si les protections sont intérieures ou si `Ventc ≤ Ventc_lim` : 0 si
/// `θet_regul ≤ θet,i_pm_limb`, 1 si `θet_regul ≥ θet,i_pm_limh`, la position précédente entre
/// les deux ; 0 pour des protections extérieures par vent fort ; 0 de nuit.
pub fn r_prot_et(c: &ConditionsProtectionsEt, r_prot_h1: f64) -> f64 {
    if !c.id_prot || c.somme_etp_lux <= 0.0 {
        return 0.0;
    }
    if c.prot_ext && c.v_ventc_m_s > V_VENTC_LIM_ET_M_S {
        return 0.0;
    }
    let k = consignes_protections(c.saison);
    if c.theta_regul_c <= k.limb_c {
        0.0
    } else if c.theta_regul_c >= k.limh_c {
        1.0
    } else {
        r_prot_h1
    }
}

/// « idsurv » : l'espace tampon solarisé peut être surventilé (§5.3.3.4, éq. 101).
pub const ID_SURV: bool = true;

/// Paramètres de l'autorisation d'ouverture, en °C (§5.3.3.4.1, tableau 17).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParametresAutorisation {
    /// « dθext/et_ouv » : écart de température avec l'extérieur.
    pub d_theta_ext_et_ouv_c: f64,
    /// « θext_seuil_ouv » : température extérieure minimale.
    pub theta_ext_seuil_ouv_c: f64,
}

/// Th-BCE 2020 §5.3.3.4.1, tableau 17 (relu sur l'image de la p. 114) : `dθext/et_ouv` = −6 °C
/// en toute saison ; `θext_seuil_ouv` = 12 °C en saison de chauffage, 10 °C en saison mixte ou
/// en mi-saison, 8 °C en saison de refroidissement.
pub fn parametres_autorisation(saison: SaisonEt) -> ParametresAutorisation {
    ParametresAutorisation {
        d_theta_ext_et_ouv_c: -6.0,
        theta_ext_seuil_ouv_c: match saison {
            SaisonEt::Chauffage => 12.0,
            SaisonEt::MiSaison | SaisonEt::Mixte => 10.0,
            SaisonEt::Refroidissement => 8.0,
        },
    }
}

/// « Autouv_et(h) » : autorisation d'ouvrir les parois de l'espace tampon.
///
/// Th-BCE 2020 §5.3.3.4.1, éq. (102) : 1 si `θext_seuil_ouv ≤ θext(h)` et
/// `θext(h) ≤ θet_regul(h) − dθext/et_ouv`, 0 sinon. Avec `dθext/et_ouv = −6 °C`, l'ouverture
/// reste permise tant que l'extérieur n'est pas plus de 6 °C au-dessus de la température de
/// référence : même convention que `Δθext_int = −3 °C` pour les baies des groupes (§5.13.3.1.1,
/// éq. 241).
pub fn aut_ouv_et(theta_ext_c: f64, theta_regul_c: f64, saison: SaisonEt) -> bool {
    let p = parametres_autorisation(saison);
    p.theta_ext_seuil_ouv_c <= theta_ext_c && theta_ext_c <= theta_regul_c - p.d_theta_ext_et_ouv_c
}

/// Consignes de la surventilation naturelle de l'espace tampon, en °C (§5.3.3.4.4,
/// tableau 18).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConsignesOuverture {
    /// « θet_ouv1 ».
    pub ouv1_c: f64,
    /// « θet_ouv2 ».
    pub ouv2_c: f64,
    /// « θet_fer1 ».
    pub fer1_c: f64,
    /// « θet_fer2 ».
    pub fer2_c: f64,
}

/// Th-BCE 2020 §5.3.3.4.4, tableau 18 (relu sur l'image de la p. 115) : en saison de chauffage
/// 29, 30, 27 et 26 °C ; en mi-saison ou saison mixte 27, 28, 25 et 24 °C ; en saison de
/// refroidissement 25, 26, 23 et 22 °C (`θet_ouv1`, `θet_ouv2`, `θet_fer1`, `θet_fer2`).
pub fn consignes_ouverture(saison: SaisonEt) -> ConsignesOuverture {
    let (ouv1_c, ouv2_c, fer1_c, fer2_c) = match saison {
        SaisonEt::Chauffage => (29.0, 30.0, 27.0, 26.0),
        SaisonEt::MiSaison | SaisonEt::Mixte => (27.0, 28.0, 25.0, 24.0),
        SaisonEt::Refroidissement => (25.0, 26.0, 23.0, 22.0),
    };
    ConsignesOuverture {
        ouv1_c,
        ouv2_c,
        fer1_c,
        fer2_c,
    }
}

/// « iper_sommeil(h) » : période de sommeil des occupants.
///
/// Th-BCE 2020 §5.3.3.4.3, éq. (103) : vrai si `ihebergement_et = 1` et
/// (`hleg(h) > 22` ou `hleg(h) < 7`), `hleg` étant l'heure légale de fin du pas de temps.
pub fn i_per_sommeil(i_hebergement_et: bool, hleg: u8) -> bool {
    i_hebergement_et && !(7..=22).contains(&hleg)
}

/// Lecture de la branche « diminution de température » de l'éq. (104), mal imprimée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LectureFermetureEt {
    /// Lecture retenue (décision Q41 point 4) : symétrique de la branche d'ouverture et
    /// conforme à la figure 17 ; au-dessus de `θet_fer1`, le ratio précédent est conservé ;
    /// entre `θet_fer2` et `θet_fer1`, `min((θet_regul − θet_fer2)/(θet_fer1 − θet_fer2) ;
    /// Rouvet_req(h−1))`.
    Symetrique,
    /// Variante, conservée non exposée : au-dessus de `θet_fer1`, le ratio vaut 0,5, valeur
    /// imprimée au début de la branche.
    DemiOuverture,
}

/// Lecture retenue de la branche de fermeture de l'éq. (104).
pub const LECTURE_FERMETURE_ET: LectureFermetureEt = LectureFermetureEt::Symetrique;

/// « Rouvet_req(h) » : ratio d'ouverture requis des parois de l'espace tampon, avec la lecture
/// retenue de l'éq. (104).
pub fn r_ouv_et_req(
    theta_regul_c: f64,
    theta_regul_h1_c: f64,
    r_req_h1: f64,
    saison: SaisonEt,
) -> f64 {
    r_ouv_et_req_selon(
        LECTURE_FERMETURE_ET,
        theta_regul_c,
        theta_regul_h1_c,
        r_req_h1,
        saison,
    )
}

/// « Rouvet_req(h) » pour une lecture donnée de la branche de fermeture.
///
/// Th-BCE 2020 §5.3.3.4.4, éq. (104), figure 17, relue sur l'image de la p. 117 : 0 si
/// `θet_regul(h) ≤ θet_fer2` ; 1 si `θet_regul(h) ≥ θet_ouv2` ; sinon, si la température monte
/// (`θet_regul(h−1) ≤ θet_regul(h)`) : le ratio précédent jusqu'à `θet_ouv1`, puis
/// `max((θet_regul − θet_ouv1)/(θet_ouv2 − θet_ouv1) ; Rouvet_req(h−1))` ; si elle descend :
/// voir [`LectureFermetureEt`].
pub fn r_ouv_et_req_selon(
    lecture: LectureFermetureEt,
    theta_regul_c: f64,
    theta_regul_h1_c: f64,
    r_req_h1: f64,
    saison: SaisonEt,
) -> f64 {
    let k = consignes_ouverture(saison);
    let t = theta_regul_c;
    if t <= k.fer2_c {
        0.0
    } else if t >= k.ouv2_c {
        1.0
    } else if theta_regul_h1_c <= t {
        if t <= k.ouv1_c {
            r_req_h1
        } else {
            ((t - k.ouv1_c) / (k.ouv2_c - k.ouv1_c)).max(r_req_h1)
        }
    } else if t >= k.fer1_c {
        match lecture {
            LectureFermetureEt::Symetrique => r_req_h1,
            LectureFermetureEt::DemiOuverture => 0.5,
        }
    } else {
        ((t - k.fer2_c) / (k.fer1_c - k.fer2_c)).min(r_req_h1)
    }
}

/// « Rouvet_eff(h) » : ratio d'ouverture effectif.
///
/// Th-BCE 2020 §5.3.3.4.5, éq. (105) : `Rouvet_req(h−1)` en période de sommeil ;
/// `Autouv_et(h)·Rouvet_req(h)` sinon. La part `Partouv_et_inocc`, citée par le texte qui
/// précède, n'apparaît pas dans l'équation : elle n'est pas employée (décision Q41 point 5).
pub fn r_ouv_et_eff(sommeil: bool, aut_ouv: bool, r_req: f64, r_req_h1: f64) -> f64 {
    if sommeil {
        r_req_h1
    } else if aut_ouv {
        r_req
    } else {
        0.0
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn saison_de_l_espace_tampon() {
        // Éq. (96).
        assert_eq!(saison_et([(true, false), (false, true)]), SaisonEt::Mixte);
        assert_eq!(
            saison_et([(true, false), (false, false)]),
            SaisonEt::Chauffage
        );
        assert_eq!(saison_et([(false, true)]), SaisonEt::Refroidissement);
        assert_eq!(saison_et([(false, false)]), SaisonEt::MiSaison);
    }

    #[test]
    fn hebergement_et_reference() {
        assert!(i_hebergement_et([true, true]));
        assert!(!i_hebergement_et([true, false]));
        assert!(!i_hebergement_et(std::iter::empty()));
        // Éq. (98) : un groupe inoccupé compte pour 0 °C.
        assert_eq!(theta_et_regul_c(15.0, [(1.0, 24.0), (0.0, 30.0)]), 24.0);
        assert_eq!(theta_et_regul_c(26.0, [(1.0, 24.0)]), 26.0);
    }

    #[test]
    fn protections_mobiles_de_l_espace_tampon() {
        let c = ConditionsProtectionsEt {
            id_prot: true,
            prot_ext: true,
            somme_etp_lux: 1000.0,
            v_ventc_m_s: 3.0,
            theta_regul_c: 27.0,
            saison: SaisonEt::MiSaison,
        };
        // Entre 26 et 28 °C : position précédente.
        assert_eq!(r_prot_et(&c, 0.0), 0.0);
        assert_eq!(r_prot_et(&c, 1.0), 1.0);
        assert_eq!(
            r_prot_et(
                &ConditionsProtectionsEt {
                    theta_regul_c: 28.0,
                    ..c
                },
                0.0
            ),
            1.0
        );
        assert_eq!(
            r_prot_et(
                &ConditionsProtectionsEt {
                    theta_regul_c: 26.0,
                    ..c
                },
                1.0
            ),
            0.0
        );
        // Vent fort, protections extérieures : relevées ; intérieures : inchangées.
        let vent = ConditionsProtectionsEt {
            v_ventc_m_s: 12.0,
            theta_regul_c: 35.0,
            ..c
        };
        assert_eq!(r_prot_et(&vent, 1.0), 0.0);
        assert_eq!(
            r_prot_et(
                &ConditionsProtectionsEt {
                    prot_ext: false,
                    ..vent
                },
                0.0
            ),
            1.0
        );
        // Nuit et absence de protection, éq. (99).
        let nuit = ConditionsProtectionsEt {
            somme_etp_lux: 0.0,
            theta_regul_c: 35.0,
            ..c
        };
        assert_eq!(r_prot_et(&nuit, 1.0), 0.0);
        let sans = ConditionsProtectionsEt {
            id_prot: false,
            theta_regul_c: 35.0,
            ..c
        };
        assert_eq!(r_prot_et(&sans, 1.0), 0.0);
    }

    #[test]
    fn autorisation_d_ouverture() {
        // Mi-saison : de 10 °C à θregul + 6 °C.
        assert!(!aut_ouv_et(9.9, 25.0, SaisonEt::MiSaison));
        assert!(aut_ouv_et(10.0, 25.0, SaisonEt::MiSaison));
        assert!(aut_ouv_et(31.0, 25.0, SaisonEt::MiSaison));
        assert!(!aut_ouv_et(31.1, 25.0, SaisonEt::MiSaison));
        assert!(!aut_ouv_et(11.0, 25.0, SaisonEt::Chauffage));
        assert!(aut_ouv_et(8.0, 25.0, SaisonEt::Refroidissement));
    }

    #[test]
    fn periode_de_sommeil() {
        assert!(i_per_sommeil(true, 23));
        assert!(i_per_sommeil(true, 0));
        assert!(i_per_sommeil(true, 6));
        assert!(!i_per_sommeil(true, 7));
        assert!(!i_per_sommeil(true, 22));
        assert!(!i_per_sommeil(false, 2));
    }

    #[test]
    fn hysteresis_de_l_ouverture() {
        let s = SaisonEt::MiSaison; // 27, 28, 25, 24 °C.
                                    // Bornes.
        assert_eq!(r_ouv_et_req(24.0, 23.0, 0.7, s), 0.0);
        assert_eq!(r_ouv_et_req(28.0, 29.0, 0.2, s), 1.0);
        // Montée : ratio conservé sous θouv1, rampe au-delà.
        assert_eq!(r_ouv_et_req(26.0, 25.0, 0.3, s), 0.3);
        assert!((r_ouv_et_req(27.5, 27.0, 0.0, s) - 0.5).abs() < 1e-12);
        assert_eq!(r_ouv_et_req(27.5, 27.0, 0.8, s), 0.8);
        // Descente : ratio conservé au-dessus de θfer1, rampe en dessous.
        assert_eq!(r_ouv_et_req(26.0, 27.0, 1.0, s), 1.0);
        assert!((r_ouv_et_req(24.5, 25.0, 1.0, s) - 0.5).abs() < 1e-12);
        assert_eq!(r_ouv_et_req(24.5, 25.0, 0.2, s), 0.2);
        // Variante de l'éq. (104) imprimée.
        assert_eq!(
            r_ouv_et_req_selon(LectureFermetureEt::DemiOuverture, 26.0, 27.0, 1.0, s),
            0.5
        );
    }

    #[test]
    fn ratio_effectif() {
        // Éq. (105).
        assert_eq!(r_ouv_et_eff(true, false, 0.2, 0.6), 0.6);
        assert_eq!(r_ouv_et_eff(false, true, 0.2, 0.6), 0.2);
        assert_eq!(r_ouv_et_eff(false, false, 0.2, 0.6), 0.0);
    }
}
