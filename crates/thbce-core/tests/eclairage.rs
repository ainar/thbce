// SPDX-License-Identifier: GPL-3.0-or-later
//! Éclairage en habitation (Th-BCE 2020 §7.1) : valeurs calculées à la main.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::eclairage::{
    c2_habitation, eclairage_habitation, einat_vn_lux, k_vn, ro_moyen_vn, AutonomieLumineuse,
};

fn proche(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * (1.0 + b.abs())
}

macro_rules! assert_proche {
    ($a:expr, $b:expr) => {
        assert!(proche($a, $b), "{} obtenu, {} attendu", $a, $b)
    };
}

#[test]
fn coefficients_de_l_eclairement_naturel() {
    // Romoyen = (0,2 + 2,5 × 0,5 + 0,7)/(2 + 2,5) = 2,15/4,5 = 0,477778.
    assert_proche!(ro_moyen_vn(), 2.15 / 4.5);
    // Interréflexions : I = Romoyen/(1 − Romoyen)/4,5 = 0,914894/4,5 = 0,203310.
    // K1vn = 0,2 × I = 0,040662 ; K2vn = 0,4 + I = 0,603310 ; K3vn = 0,7 × 0,8 + 0,7 × I = 0,702317.
    let [k1, k2, k3] = k_vn();
    assert_proche!(k1, 0.040_661_938_534_278_96);
    assert_proche!(k2, 0.603_309_692_671_394_8);
    assert_proche!(k3, 0.702_316_784_869_976_3);
}

#[test]
fn eclairement_naturel_d_un_volume_normal() {
    // Flux de la baie sud de 8 m² du test des baies (sans protection) : Flt1 = 332 553,755 lm,
    // Flt2 = 28 800 lm, Flt3 = 24 960 lm ; Aeclnat2 = 96,44 m².
    // Einat = (0,040662 × 332 553,755 + 0,603310 × 28 800 + 0,702317 × 24 960)/96,44 = 502,15 lux.
    let e = einat_vn_lux(332_553.755_053_224_45, 28_800.0, 24_960.0, 96.44);
    assert_proche!(e, 502.150_834_162_263_1);
    // Linéaire en flux ; nul sans surface ayant accès à la lumière naturelle.
    assert_proche!(
        einat_vn_lux(0.0, 28_800.0, 0.0, 96.44),
        0.603_309_692_671_394_8 * 28_800.0 / 96.44
    );
    assert_eq!(einat_vn_lux(1.0e5, 1.0e4, 1.0e4, 0.0), 0.0);
}

#[test]
fn coefficient_c2_en_habitation() {
    // Figure 86 : A (0 ; 1), B (100 ; 1), G (200 ; 0,05), J (2800 ; 0).
    assert_eq!(c2_habitation(0.0), 1.0);
    assert_eq!(c2_habitation(100.0), 1.0);
    assert_proche!(c2_habitation(150.0), 0.525);
    assert_proche!(c2_habitation(200.0), 0.05);
    assert_proche!(c2_habitation(1_500.0), 0.025);
    assert_eq!(c2_habitation(2_800.0), 0.0);
    assert_eq!(c2_habitation(10_000.0), 0.0);
}

#[test]
fn consommation_et_apports_d_un_groupe_d_habitation() {
    // 100 m², 150 lux : 1 W/m² × 0,9 × 0,525 × 100 m² = 47,25 W, soit 0,04725 kWh sur l'heure ;
    // apports récupérables : moitié convectif, moitié radiatif.
    let e = eclairage_habitation(true, 150.0, 100.0);
    assert_proche!(e.c_ecl_kwh, 0.047_25);
    assert_proche!(e.f_ecl_c_w, 23.625);
    assert_proche!(e.f_ecl_r_w, 23.625);
    // De nuit (Einat = 0) : 0,9 W/m², soit 0,09 kWh pour 100 m².
    assert_proche!(eclairage_habitation(true, 0.0, 100.0).c_ecl_kwh, 0.09);
    // Éclairage non autorisé par le scénario (IEcl = 0), ou plein jour au-delà de 2 800 lux.
    assert_eq!(eclairage_habitation(false, 0.0, 100.0).c_ecl_kwh, 0.0);
    assert_eq!(eclairage_habitation(true, 3_000.0, 100.0).c_ecl_kwh, 0.0);
}

#[test]
fn ordre_de_grandeur_et_unites_des_apports_d_eclairage() {
    // Décision Q33, point 1. Logement de 100 m², éclairage autorisé, sans lumière naturelle :
    // éq. (789), 1 W/m² × 0,9 × 1 × 100 m² / 1000 = 0,09 kWh sur l'heure. Les apports transmis au
    // modèle thermique sont en watts : 0,09 kWh sur une heure = 90 W, dont 45 W convectifs et
    // 45 W radiatifs (§7.1.3.7).
    let e = eclairage_habitation(true, 0.0, 100.0);
    assert_proche!(e.c_ecl_kwh, 0.09);
    assert_proche!(e.f_ecl_c_w, 45.0);
    assert_proche!(e.f_ecl_r_w, 45.0);
    assert_proche!(e.f_ecl_c_w + e.f_ecl_r_w, 90.0);
    // Cohérence des unités : la somme des apports, en W, vaut 1000 fois la consommation en kWh.
    assert_proche!(e.f_ecl_c_w + e.f_ecl_r_w, 1_000.0 * e.c_ecl_kwh);
}

#[test]
fn autonomie_en_lumiere_naturelle() {
    // Éq. (785), seuil de 300 lux.
    let mut a = AutonomieLumineuse::default();
    assert_eq!(a.taux_occ_einat_sup_pct(), None);
    a.compter(false, true, 1_000.0);
    a.compter(true, false, 0.0);
    a.compter(true, true, 299.9);
    a.compter(true, true, 300.0);
    a.compter(true, true, 800.0);
    a.compter(true, true, 50.0);
    assert_eq!(
        (
            a.nbh_ecl_non_aut,
            a.nbh_occ_nuit,
            a.nbh_occ_einat_inf,
            a.nbh_occ_einat_sup
        ),
        (1, 1, 2, 2)
    );
    assert_proche!(a.taux_occ_einat_sup_pct().unwrap(), 50.0);
}

/// Usages autres que l'habitation : valeurs calculées à la main d'après les §7.1.3.1 à
/// 7.1.3.6.1 et les tableaux 78 et 79.
mod tertiaire {
    use thbce_core::eclairage::{
        a_eclnat2_m2, c1, c2, ctrl_ecl_einat, eclairage_locaux, facteur_einat_sans_acces,
        p_ecl_tot_bbio_w_m2, p_ecl_tot_bureaux_w_m2, LocalResolu,
    };
    use thbce_core::entree::{GestEcl, GradEcl, LocalEclairage};

    fn proche(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12 * (1.0 + b.abs())
    }

    fn local(json: &str) -> LocalEclairage {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn puissances() {
        // Éq. (782) : bureau standard, Eiref = 500 lux : 10 W/m².
        assert!(proche(p_ecl_tot_bbio_w_m2(500.0), 10.0));
        // §7.1.3.2.3 : 6 W/m² à 1,5 W/m² pour 100 lux : 400 lux ; requis 500 lux : il manque
        // 100 lux, soit 1 W/m² d'éclairage mobilier.
        assert!(proche(p_ecl_tot_bureaux_w_m2(6.0, 1.5, 500.0), 7.0));
        // Éclairement requis atteint : pas d'éclairage mobilier.
        assert!(proche(p_ecl_tot_bureaux_w_m2(6.0, 1.2, 500.0), 6.0));
        // À partir de 10 W/m², la puissance immobilière seule.
        assert!(proche(p_ecl_tot_bureaux_w_m2(10.0, 4.0, 500.0), 10.0));
    }

    #[test]
    fn coefficient_c1() {
        let ligne = [0.90, 0.85, 0.80, 0.70];
        assert_eq!(c1(GestEcl::Permanent, &ligne), 1.0);
        assert_eq!(c1(GestEcl::InterrupteurManuel, &ligne), 0.90);
        assert_eq!(c1(GestEcl::InterrupteurEtHorloge, &ligne), 0.85);
        assert_eq!(c1(GestEcl::MarcheEtArretAutomatiques, &ligne), 0.80);
        assert_eq!(c1(GestEcl::MarcheManuelleArretAutomatique, &ligne), 0.70);
    }

    #[test]
    fn coefficient_c2_tableau_79() {
        // Grad_ecl = 0.
        assert_eq!(c2(GradEcl::Aucune, 5000.0, 500.0), 1.0);
        // 1, gestion manuelle : B (100 ; 1), G (700 ; 0,3), J (2800 ; 0).
        assert_eq!(c2(GradEcl::Manuelle, 50.0, 500.0), 1.0);
        assert!(proche(c2(GradEcl::Manuelle, 400.0, 500.0), 0.65));
        assert!(proche(c2(GradEcl::Manuelle, 700.0, 500.0), 0.3));
        assert!(proche(c2(GradEcl::Manuelle, 1750.0, 500.0), 0.15));
        assert_eq!(c2(GradEcl::Manuelle, 3000.0, 500.0), 0.0);
        // 2, gradation : B (100 ; 1), E (500 ; 0,15), H (1000 ; 0,15), nul au-delà.
        assert_eq!(c2(GradEcl::GradationAutomatique, 100.0, 500.0), 1.0);
        assert!(proche(
            c2(GradEcl::GradationAutomatique, 300.0, 500.0),
            0.575
        ));
        assert!(proche(
            c2(GradEcl::GradationAutomatique, 500.0, 500.0),
            0.15
        ));
        assert!(proche(
            c2(GradEcl::GradationAutomatique, 1000.0, 500.0),
            0.15
        ));
        assert_eq!(c2(GradEcl::GradationAutomatique, 1000.1, 500.0), 0.0);
        // 3, allumage et extinction automatiques : 1 jusqu'à Eiref.
        assert_eq!(
            c2(GradEcl::AllumageEtExtinctionAutomatiques, 500.0, 500.0),
            1.0
        );
        assert_eq!(
            c2(GradEcl::AllumageEtExtinctionAutomatiques, 500.1, 500.0),
            0.0
        );
        // 4, extinction automatique, Eiref = 500 < 700 : point D (500 ; (6,7 − 3,5)/6).
        assert!(proche(
            c2(GradEcl::ExtinctionAutomatique, 500.0, 500.0),
            3.2 / 6.0
        ));
        assert!(proche(
            c2(GradEcl::ExtinctionAutomatique, 400.0, 500.0),
            0.65
        ));
        assert_eq!(c2(GradEcl::ExtinctionAutomatique, 501.0, 500.0), 0.0);
        // Eiref = 1400 ≥ 700 : point D (1400 ; 0,4 − 1400/7000 = 0,2).
        assert!(proche(
            c2(GradEcl::ExtinctionAutomatique, 1400.0, 1400.0),
            0.2
        ));
        // Eiref ≤ Eimin (circulation, 100 lux ; salle de réunion des bureaux, 50 lux), lecture
        // Q58. Gradation : 1 jusqu'à 100 lux, puis les points au-delà.
        assert_eq!(c2(GradEcl::GradationAutomatique, 80.0, 50.0), 1.0);
        assert_eq!(c2(GradEcl::GradationAutomatique, 101.0, 50.0), 0.0);
        assert!(proche(
            c2(GradEcl::GradationAutomatique, 150.0, 100.0),
            0.15
        ));
        // Gestions 3 et 4 : « pour Einat > EiRef, C2 = 0 », y compris sous 100 lux.
        assert_eq!(
            c2(GradEcl::AllumageEtExtinctionAutomatiques, 50.0, 50.0),
            1.0
        );
        assert_eq!(
            c2(GradEcl::AllumageEtExtinctionAutomatiques, 80.0, 50.0),
            0.0
        );
        assert_eq!(c2(GradEcl::ExtinctionAutomatique, 40.0, 50.0), 1.0);
        assert_eq!(c2(GradEcl::ExtinctionAutomatique, 80.0, 50.0), 0.0);
        assert_eq!(c2(GradEcl::ExtinctionAutomatique, 100.0, 100.0), 1.0);
        assert_eq!(c2(GradEcl::ExtinctionAutomatique, 120.0, 100.0), 0.0);
    }

    #[test]
    fn fractionnement() {
        assert!(proche(facteur_einat_sans_acces(0.9), 0.75));
        assert!(proche(facteur_einat_sans_acces(0.7), 0.25));
        assert!(proche(facteur_einat_sans_acces(0.5), 0.15));
        assert_eq!(facteur_einat_sans_acces(0.2), 0.0);
        // Local sans accès : C2ae = C2pae = 1.
        assert_eq!(
            ctrl_ecl_einat(GradEcl::Manuelle, true, 0.0, 2000.0, 500.0),
            1.0
        );
        // Ratio 0,5, Einat(2) = 2 000 lux, gestion manuelle. Partie sans accès : 300 lux,
        // C2pae = 1 − 0,7 × 200/600 ; partie avec accès : 2 000 lux,
        // C2ae = 0,3 × 800/2100.
        let c2_pae = 1.0 - 0.7 * 200.0 / 600.0;
        let c2_ae = 0.3 * 800.0 / 2100.0;
        assert!(proche(
            ctrl_ecl_einat(GradEcl::Manuelle, true, 0.5, 2000.0, 500.0),
            0.5 * c2_ae + 0.5 * c2_pae
        ));
        // Non fractionnée : la partie sans accès impose son fonctionnement à tout le local.
        assert!(proche(
            ctrl_ecl_einat(GradEcl::Manuelle, false, 0.5, 2000.0, 500.0),
            c2_pae
        ));
    }

    #[test]
    fn consommation_d_un_groupe_de_bureaux() {
        // Groupe de 1 000 m² : 60 % de bureaux standards (gradation, détection, fractionnée),
        // 40 % de circulations sans accès à la lumière naturelle (interrupteur).
        let bureaux = local(
            r#"{"id": "b", "type_local": "bureau_standard", "ratio_local": 0.6,
                "ratio_ecl_nat": 0.8,
                "eclairage": {"p_ecl_tot_w_m2": 6, "p_ecl_aux_w_m2": 0.3,
                              "gest_ecl": "marche_manuelle_arret_automatique",
                              "grad_ecl": "gradation_automatique", "fractionne": true,
                              "eff_ecl_immo_projet_w_m2_100lux": 1.5, "ei_proj_lux": 500}}"#,
        );
        let circulations = local(
            r#"{"id": "c", "type_local": "circulation_accueil", "ratio_local": 0.4,
                "ratio_ecl_nat": 0,
                "eclairage": {"p_ecl_tot_w_m2": 4, "gest_ecl": "interrupteur_manuel",
                              "grad_ecl": "manuelle", "fractionne": false}}"#,
        );
        // Projet : 6 W/m² immobiliers + 1 W/m² mobilier ; C1 = 0,70 ; circulations C1 = 0,80.
        let projet = [
            LocalResolu::projet(3, &bureaux, 1000.0).unwrap(),
            LocalResolu::projet(3, &circulations, 1000.0).unwrap(),
        ];
        assert!(proche(projet[0].p_ecl_tot_w_m2, 7.0));
        assert_eq!((projet[0].c1, projet[0].p_ecl_aux_w_m2), (0.70, 0.3));
        assert_eq!((projet[1].c1, projet[1].p_ecl_aux_w_m2), (0.80, 0.0));
        // Aeclnat2 = 600 × 0,8 = 480 m².
        assert!(proche(a_eclnat2_m2(&projet), 480.0));
        // De nuit (Einat = 0), en occupation : bureaux 7 × 0,7 × 600 = 2 940 W ; circulations
        // 4 × 0,8 × 400 = 1 280 W.
        let nuit = eclairage_locaux(&projet, true, 0.0);
        assert!(proche(nuit.c_ecl_kwh, 4.22));
        assert!(proche(nuit.f_ecl_c_w, 2110.0));
        assert!(proche(nuit.f_ecl_r_w, 2110.0));
        // Einat(2) = 1 200 lux : partie avec accès, au-delà de 2 × Eiref, C2ae = 0 ; partie
        // sans accès, 1 200 × (2,5 × 0,8 − 1,5) = 600 lux, C2pae = 0,15 ;
        // CTRLecl_einat = 0,2 × 0,15 = 0,03 ; 7 × 0,7 × 0,03 = 0,147 W/m², moins que les
        // périphériques de gestion (0,3 W/m²) : 0,3 × 600 = 180 W. Circulations : 1 280 W.
        let jour = eclairage_locaux(&projet, true, 1200.0);
        assert!(proche(jour.c_ecl_kwh, 0.18 + 1.28));
        // En inoccupation : seuls les périphériques de gestion consomment.
        let inocc = eclairage_locaux(&projet, false, 0.0);
        assert!(proche(inocc.c_ecl_kwh, 0.18));

        // Bbio : 2 × Eiref/100, soit 10 W/m² en bureaux (C1 = 0,85) et 2 W/m² en circulations
        // (C1 = 0,75), gestion manuelle non fractionnée, sans périphériques.
        let bbio = [
            LocalResolu::bbio(3, &bureaux, 1000.0).unwrap(),
            LocalResolu::bbio(3, &circulations, 1000.0).unwrap(),
        ];
        let nuit = eclairage_locaux(&bbio, true, 0.0);
        assert!(proche(
            nuit.c_ecl_kwh,
            (10.0 * 0.85 * 600.0 + 2.0 * 0.75 * 400.0) / 1000.0
        ));
        assert_eq!(eclairage_locaux(&bbio, false, 0.0).c_ecl_kwh, 0.0);
        // Einat(2) = 1 200 lux, non fractionnée : 600 lux partout dans les bureaux,
        // C2 = 1 − 0,7 × 500/600.
        let c2 = 1.0 - 0.7 * 500.0 / 600.0;
        let jour = eclairage_locaux(&bbio, true, 1200.0);
        assert!(proche(
            jour.c_ecl_kwh,
            (10.0 * 0.85 * c2 * 600.0 + 2.0 * 0.75 * 400.0) / 1000.0
        ));
        // Type de local absent du tableau 78 pour l'usage, grand volume : non résolus.
        let classe = local(
            r#"{"id": "x", "type_local": "salle_classe", "ratio_local": 1, "ratio_ecl_nat": 1,
                "eclairage": {"p_ecl_tot_w_m2": 6, "gest_ecl": "interrupteur_manuel",
                              "grad_ecl": "manuelle", "fractionne": false}}"#,
        );
        assert!(LocalResolu::projet(3, &classe, 100.0).is_none());
        let sport = local(
            r#"{"id": "x", "type_local": "salle_sport", "ratio_local": 1, "ratio_ecl_nat": 1,
                "eclairage": {"p_ecl_tot_w_m2": 6, "gest_ecl": "interrupteur_manuel",
                              "grad_ecl": "manuelle", "fractionne": false}}"#,
        );
        assert!(LocalResolu::bbio(25, &sport, 100.0).is_none());
    }

    /// Les quatre locaux de l'exemple `bureaux-simple-flux-ademe` (groupe de 1 000 m²), valeurs
    /// calculées à la main par le relecteur du lot, indépendamment du code.
    #[test]
    fn locaux_de_l_exemple_de_bureaux_a_la_main() {
        let locaux = [
            local(
                r#"{"id": "bureaux", "type_local": "bureau_standard", "ratio_local": 0.6,
                    "ratio_ecl_nat": 0.8,
                    "eclairage": {"p_ecl_tot_w_m2": 6, "p_ecl_aux_w_m2": 0.2,
                                  "gest_ecl": "marche_manuelle_arret_automatique",
                                  "grad_ecl": "gradation_automatique", "fractionne": true,
                                  "eff_ecl_immo_projet_w_m2_100lux": 1.5, "ei_proj_lux": 500}}"#,
            ),
            local(
                r#"{"id": "reunion", "type_local": "salle_reunion", "ratio_local": 0.1,
                    "ratio_ecl_nat": 0.8,
                    "eclairage": {"p_ecl_tot_w_m2": 8, "gest_ecl": "interrupteur_manuel",
                                  "grad_ecl": "manuelle", "fractionne": false}}"#,
            ),
            local(
                r#"{"id": "circulations", "type_local": "circulation_accueil",
                    "ratio_local": 0.26, "ratio_ecl_nat": 0.2,
                    "eclairage": {"p_ecl_tot_w_m2": 4, "p_ecl_aux_w_m2": 0.1,
                                  "gest_ecl": "marche_et_arret_automatiques",
                                  "grad_ecl": "manuelle", "fractionne": false}}"#,
            ),
            local(
                r#"{"id": "sanitaires", "type_local": "sanitaires_collectifs",
                    "ratio_local": 0.04, "ratio_ecl_nat": 0,
                    "eclairage": {"p_ecl_tot_w_m2": 6, "p_ecl_aux_w_m2": 0.1,
                                  "gest_ecl": "marche_et_arret_automatiques",
                                  "grad_ecl": "aucune", "fractionne": false}}"#,
            ),
        ];
        let projet: Vec<LocalResolu> = locaux
            .iter()
            .map(|l| LocalResolu::projet(3, l, 1000.0).unwrap())
            .collect();
        let bbio: Vec<LocalResolu> = locaux
            .iter()
            .map(|l| LocalResolu::bbio(3, l, 1000.0).unwrap())
            .collect();
        let pres = |a: f64, b: f64| (a - b).abs() < 5e-7;
        // Une heure d'occupation sans lumière naturelle, puis une heure d'inoccupation.
        let (c, b) = (
            eclairage_locaux(&projet, true, 0.0).c_ecl_kwh,
            eclairage_locaux(&bbio, true, 0.0).c_ecl_kwh,
        );
        assert!(pres(c, 4.372) && pres(b, 5.659), "{c} {b}");
        let (c, b) = (
            eclairage_locaux(&projet, false, 0.0).c_ecl_kwh,
            eclairage_locaux(&bbio, false, 0.0).c_ecl_kwh,
        );
        assert!(pres(c, 0.150) && b == 0.0, "{c} {b}");
        // Une heure d'occupation avec Einat(2) = 400 lux.
        let (c, b) = (
            eclairage_locaux(&projet, true, 400.0).c_ecl_kwh,
            eclairage_locaux(&bbio, true, 400.0).c_ecl_kwh,
        );
        assert!(pres(c, 2.682317) && pres(b, 5.056417), "{c} {b}");
    }
}
