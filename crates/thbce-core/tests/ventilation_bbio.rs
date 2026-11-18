// SPDX-License-Identifier: GPL-3.0-or-later
//! Débits d'air spécifiques du calcul du Bbio en habitation (Th-BCE 2020 §6.1 et §6.2) :
//! valeurs calculées à la main d'après les équations.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{BoucheConduit, Isouf, Projet};
use thbce_core::valider;
use thbce_core::ventilation::bouche_conduit::{
    debits_avec_depassement, debits_max_habitation, debits_min_habitation,
    debits_regules_habitation, debits_specifiques_bbio_groupe, debits_specifiques_bbio_habitation,
    q_spec_conv_base_hygroreglable_m3_h, Debits, CDEP_BBIO, CRDBNR_BBIO, DUGD_BBIO_H, K_RES_BBIO,
};
use thbce_core::ventilation::ven_bbio::{debits_systeme_conventionnel, LectureDebitsBbio};
use thbce_core::{calculer, Options};
use thbce_data::ZoneClimatique;

const ENTREE: &str = include_str!("../../../tests/cas/squelette-maison/entree.json");

fn projet() -> Projet {
    serde_json::from_str(ENTREE).unwrap()
}

fn bouche(isouf: Isouf, pointe: f64, base: Option<f64>, qvarep: Option<f64>) -> BoucheConduit {
    BoucheConduit {
        id: "b".into(),
        isouf,
        q_spec_conv_pointe_m3_h: pointe,
        q_spec_conv_base_m3_h: base,
        qvarep_spec_m3_h: qvarep,
        dugd_h: None,
        cdep: None,
        a_cond_m2: None,
        q_spec_occ_m3_h: None,
        q_spec_inocc_m3_h: None,
        q_spec_ch_m3_h: None,
        crdbnr: None,
    }
}

fn proche(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} au lieu de {b}");
}

#[test]
fn conventions_du_bbio() {
    // §6.2.3.2.1 : Crdbnr = 1 ; §6.2.3.2.2.2 : Dugd = 14 h par semaine ; éq. (414) : Cdep = 1 ;
    // tableau 59 : Kres = 0.
    assert_eq!(CRDBNR_BBIO, 1.0);
    assert_eq!(DUGD_BBIO_H, 14.0);
    assert_eq!(CDEP_BBIO, 1.0);
    assert_eq!(K_RES_BBIO, 0.0);
}

#[test]
fn debits_maximal_et_minimal_d_une_bouche() {
    // Éq. (387) et (389), Isouf = 0 : le débit repris est négatif, le débit soufflé nul.
    let max = debits_max_habitation(Isouf::Reprise, 180.0);
    let min = debits_min_habitation(Isouf::Reprise, 105.0);
    assert_eq!((max.q_rep_m3_h, max.q_souffle_m3_h), (-180.0, 0.0));
    assert_eq!((min.q_rep_m3_h, min.q_souffle_m3_h), (-105.0, 0.0));
    // Éq. (388) et (390), Isouf = 1.
    let max = debits_max_habitation(Isouf::Soufflage, 150.0);
    let min = debits_min_habitation(Isouf::Soufflage, 90.0);
    assert_eq!((max.q_rep_m3_h, max.q_souffle_m3_h), (0.0, 150.0));
    assert_eq!((min.q_rep_m3_h, min.q_souffle_m3_h), (0.0, 90.0));
}

#[test]
fn debit_regule_en_habitation() {
    // Éq. (403) : q_regul = Crdbnr × (q_max × Dugd + q_min × (168 − Dugd))/168.
    // Reprise 180/105 m³/h, Dugd = 14 : −(180 × 14 + 105 × 154)/168 = −18 690/168 = −111,25.
    let r = debits_regules_habitation(
        1.0,
        debits_max_habitation(Isouf::Reprise, 180.0),
        debits_min_habitation(Isouf::Reprise, 105.0),
        14.0,
    );
    proche(r.q_rep_m3_h, -111.25);
    assert_eq!(r.q_souffle_m3_h, 0.0);
    // Soufflage 150/90 m³/h : (150 × 14 + 90 × 154)/168 = 15 960/168 = 95.
    let r = debits_regules_habitation(
        1.0,
        debits_max_habitation(Isouf::Soufflage, 150.0),
        debits_min_habitation(Isouf::Soufflage, 90.0),
        14.0,
    );
    proche(r.q_souffle_m3_h, 95.0);
    assert_eq!(r.q_rep_m3_h, 0.0);
    // Hors Bbio (lot 6a) : temporisation, Dugd = 7 : (180 × 7 + 105 × 161)/168 = 108,125 ;
    // Crdbnr = 0,9 : 97,3125 ; Cdep = 1,15 (éq. 412) : 111,909375.
    let max = debits_max_habitation(Isouf::Reprise, 180.0);
    let min = debits_min_habitation(Isouf::Reprise, 105.0);
    proche(
        debits_regules_habitation(1.0, max, min, 7.0).q_rep_m3_h,
        -108.125,
    );
    let r = debits_regules_habitation(0.9, max, min, 7.0);
    proche(r.q_rep_m3_h, -97.3125);
    proche(debits_avec_depassement(1.15, r).q_rep_m3_h, -111.909_375);
    // Débits de pointe et de base égaux : le débit régulé est ce débit.
    let d = debits_max_habitation(Isouf::Reprise, 90.0);
    proche(debits_regules_habitation(1.0, d, d, 14.0).q_rep_m3_h, -90.0);
}

#[test]
fn debit_de_base_d_un_systeme_hygroreglable() {
    // §6.2.3.1.1.2 : q_base = (168 × Qvarepspec − Dugd × q_pointe)/(168 − Dugd).
    // Qvarepspec = 60, pointe = 135, Dugd = 14 : (10 080 − 1 890)/154 = 8 190/154 = 53,1818…
    let base = q_spec_conv_base_hygroreglable_m3_h(60.0, 135.0, 14.0);
    proche(base, 8190.0 / 154.0);
    proche(base, 53.181_818_181_818);
    // Dugd = 7 : (10 080 − 945)/161 = 56,7391…
    proche(
        q_spec_conv_base_hygroreglable_m3_h(60.0, 135.0, 7.0),
        9135.0 / 161.0,
    );
    // Le débit régulé du Bbio est alors Qvarepspec : (135 × 14 + 53,1818 × 154)/168 = 60.
    let d = debits_specifiques_bbio_habitation(&bouche(Isouf::Reprise, 135.0, None, Some(60.0)))
        .unwrap();
    proche(d.q_rep_m3_h, -60.0);
    assert_eq!(d.q_souffle_m3_h, 0.0);
}

#[test]
fn debits_specifiques_du_bbio_d_un_groupe() {
    // Une bouche : éq. (403) puis (412) avec Cdep = 1 et (428) avec Kres = 0.
    let d = debits_specifiques_bbio_habitation(&bouche(Isouf::Reprise, 180.0, Some(105.0), None))
        .unwrap();
    proche(d.q_rep_m3_h, -111.25);
    // Deux circuits de reprise et un de soufflage : −(111,25 + 60) = −171,25 ; +95.
    let bouches = [
        bouche(Isouf::Reprise, 180.0, Some(105.0), None),
        bouche(Isouf::Reprise, 135.0, None, Some(60.0)),
        bouche(Isouf::Soufflage, 150.0, Some(90.0), None),
    ];
    let d = debits_specifiques_bbio_groupe(&bouches).unwrap();
    proche(d.q_rep_m3_h, -171.25);
    proche(d.q_souffle_m3_h, 95.0);
    // Sans débit de base ni Qvarepspec : pas de débit (la validation refuse cette saisie).
    assert_eq!(
        debits_specifiques_bbio_habitation(&bouche(Isouf::Reprise, 180.0, None, None)),
        None
    );
}

#[test]
fn debits_du_systeme_conventionnel_du_bbio() {
    // §6.1.3 : VMC à débit soufflé et extrait constant. Ventilation installée à simple flux par
    // extraction : le système conventionnel souffle autant qu'il extrait (Q36 point 1).
    let extraction = Debits {
        q_rep_m3_h: -111.25,
        q_souffle_m3_h: 0.0,
    };
    let d = debits_systeme_conventionnel(extraction, LectureDebitsBbio::DoubleFluxConventionnelle);
    assert_eq!((d.q_rep_m3_h, d.q_souffle_m3_h), (-111.25, 111.25));
    // Simple flux par insufflation : il extrait autant qu'il souffle.
    let insufflation = Debits {
        q_rep_m3_h: 0.0,
        q_souffle_m3_h: 95.0,
    };
    let d =
        debits_systeme_conventionnel(insufflation, LectureDebitsBbio::DoubleFluxConventionnelle);
    assert_eq!((d.q_rep_m3_h, d.q_souffle_m3_h), (-95.0, 95.0));
    // Les deux sens décrits : pris tels quels.
    let double_flux = Debits {
        q_rep_m3_h: -100.0,
        q_souffle_m3_h: 95.0,
    };
    assert_eq!(
        debits_systeme_conventionnel(double_flux, LectureDebitsBbio::DoubleFluxConventionnelle),
        double_flux
    );
    // Lecture littérale, conservée : les débits des bouches, sans débit soufflé.
    assert_eq!(
        debits_systeme_conventionnel(extraction, LectureDebitsBbio::Litterale),
        extraction
    );
    assert_eq!(
        LectureDebitsBbio::default(),
        LectureDebitsBbio::DoubleFluxConventionnelle
    );
}

#[test]
fn les_resultats_donnent_les_debits_du_bbio() {
    // Maison de démonstration : extraction 180/105 m³/h, soit 111,25 m³/h repris et autant
    // soufflé ; 111,3 après l'arrondi de restitution.
    let meteo = MeteoProjet::synthetique(ZoneClimatique::H2b);
    let brut = calculer(&projet(), &meteo, &Options { arrondir: false }).unwrap();
    let d = brut.batiments[0].zones[0].groupes[0]
        .debits_bbio
        .clone()
        .unwrap();
    proche(d.q_spec_repris_m3_h, -111.25);
    proche(d.q_spec_souffle_m3_h, 111.25);
    let arrondi = calculer(&projet(), &meteo, &Options::default()).unwrap();
    let d = arrondi.batiments[0].zones[0].groupes[0]
        .debits_bbio
        .clone()
        .unwrap();
    assert_eq!(
        (d.q_spec_repris_m3_h, d.q_spec_souffle_m3_h),
        (-111.3, 111.3)
    );
}

#[test]
fn regles_de_saisie_des_bouches() {
    let chemin = "batiments[0].zones[0].groupes[0].bouches_conduits";
    let erreurs = |modifier: &dyn Fn(&mut Vec<BoucheConduit>)| -> Vec<String> {
        let mut p = projet();
        modifier(&mut p.batiments[0].zones[0].groupes[0].bouches_conduits);
        valider(&p).erreurs.into_iter().map(|e| e.chemin).collect()
    };
    assert!(erreurs(&|_| {}).is_empty());
    // En habitation, au moins une bouche.
    assert_eq!(erreurs(&|b| b.clear()), [chemin]);
    // Le débit de pointe est au moins égal au débit de base ; l'égalité est admise.
    assert_eq!(
        erreurs(&|b| b[0].q_spec_conv_base_m3_h = Some(181.0)),
        [format!("{chemin}[0].q_spec_conv_base_m3_h")]
    );
    assert!(erreurs(&|b| b[0].q_spec_conv_base_m3_h = Some(180.0)).is_empty());
    // Débits positifs.
    assert_eq!(
        erreurs(&|b| b[0].q_spec_conv_pointe_m3_h = -1.0),
        [
            format!("{chemin}[0].q_spec_conv_pointe_m3_h"),
            format!("{chemin}[0].q_spec_conv_base_m3_h")
        ]
    );
    // Débit de base ou Qvarepspec : l'un des deux, pas les deux.
    assert_eq!(
        erreurs(&|b| b[0].q_spec_conv_base_m3_h = None),
        [format!("{chemin}[0].q_spec_conv_base_m3_h")]
    );
    assert_eq!(
        erreurs(&|b| b[0].qvarep_spec_m3_h = Some(60.0)),
        [format!("{chemin}[0].qvarep_spec_m3_h")]
    );
    // Hygroréglable : Qvarepspec = 60 pour 180 en pointe : base = (10 080 − 2 520)/154 = 49,1.
    let hygro = |q: f64| {
        move |b: &mut Vec<BoucheConduit>| {
            b[0].q_spec_conv_base_m3_h = None;
            b[0].qvarep_spec_m3_h = Some(q);
        }
    };
    assert!(erreurs(&hygro(60.0)).is_empty());
    // Qvarepspec = 10 : base = (1 680 − 2 520)/154 < 0 ; Qvarepspec = 200 : base > pointe.
    assert_eq!(
        erreurs(&hygro(10.0)),
        [format!("{chemin}[0].qvarep_spec_m3_h")]
    );
    assert_eq!(
        erreurs(&hygro(200.0)),
        [format!("{chemin}[0].qvarep_spec_m3_h")]
    );
    // Identifiants uniques dans le groupe.
    assert_eq!(
        erreurs(&|b| {
            let copie = b[0].clone();
            b.push(copie);
        }),
        [format!("{chemin}[1].id")]
    );
}

/// Débits conventionnels du Bbio hors habitation (§6.2.3.1.3, tableau 56-1), calculés à la
/// main.
#[test]
fn debits_conventionnels_non_residentiels() {
    use thbce_core::ventilation::bouche_conduit::{
        debits_bbio_non_residentiel, debits_dep_non_residentiel, param_debits_bbio,
    };
    // Bureaux, 1 000 m² : 4 m³/h par m² en occupation ; max(60 ; 0,42 × 1 000) = 420 m³/h en
    // inoccupation ; repris et soufflés par le système conventionnel du Bbio (Q36, Q60).
    let (occ, inocc) = debits_bbio_non_residentiel(3, 1000.0).unwrap();
    assert_eq!((occ.q_rep_m3_h, occ.q_souffle_m3_h), (-4000.0, 4000.0));
    assert_eq!((inocc.q_rep_m3_h, inocc.q_souffle_m3_h), (-420.0, 420.0));
    // Petit groupe de 100 m² : le plancher de 60 m³/h l'emporte sur 42 m³/h.
    let (occ, inocc) = debits_bbio_non_residentiel(3, 100.0).unwrap();
    assert_eq!((occ.q_souffle_m3_h, inocc.q_souffle_m3_h), (400.0, 60.0));
    // Commerces : Param_inoccupation = 0, le plancher s'applique.
    let (occ, inocc) = debits_bbio_non_residentiel(17, 500.0).unwrap();
    proche(occ.q_souffle_m3_h, 1850.0);
    assert_eq!(inocc.q_souffle_m3_h, 60.0);
    // Habitation : débits d'hygiène des bouches, pas de paramètres conventionnels.
    assert!(debits_bbio_non_residentiel(1, 100.0).is_none());
    assert!(param_debits_bbio(2).is_none() && param_debits_bbio(29).is_none());
    for usage in 3..=28 {
        let (o, i) = param_debits_bbio(usage).unwrap();
        assert!(o > 0.0 && i >= 0.0, "usage {usage}");
    }
    // Hôtels 3 à 5 étoiles, partie nuit : le tableau donne plus en inoccupation (2,2) qu'en
    // occupation (2).
    assert_eq!(param_debits_bbio(9), Some((2.0, 2.2)));

    // Débits du projet, éq. (383) à (386), (400) à (402), (412) et (413) : soufflage de
    // 2 000 m³/h en occupation et 500 en inoccupation, Crdbnr = 0,8, Cdep = 1,3.
    let (max, occ, inocc) = debits_dep_non_residentiel(Isouf::Soufflage, 2000.0, 500.0, 0.8, 1.3);
    proche(max, 2000.0);
    proche(occ, 2080.0);
    proche(inocc, 650.0);
    let (max, occ, inocc) = debits_dep_non_residentiel(Isouf::Reprise, 2000.0, 0.0, 1.0, 1.15);
    proche(max, -2000.0);
    proche(occ, -2300.0);
    assert_eq!(inocc, 0.0);
}
