// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests de la chaîne validation → calcul sur le cas de démonstration.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{Projet, UsageZone};
use thbce_core::{calculer, valider, Erreur, Options};
use thbce_data::ZoneClimatique;

const ENTREE: &str = include_str!("../../../tests/cas/squelette-maison/entree.json");

fn projet() -> Projet {
    serde_json::from_str(ENTREE).unwrap()
}

fn chemins(problemes: &[thbce_core::validation::Probleme]) -> Vec<&str> {
    problemes.iter().map(|p| p.chemin.as_str()).collect()
}

#[test]
fn le_cas_de_demonstration_est_valide() {
    let r = valider(&projet());
    assert!(r.est_valide(), "{:?}", r.erreurs);
    assert!(r.avertissements.is_empty(), "{:?}", r.avertissements);
}

#[test]
fn la_zone_est_deduite_du_departement() {
    let mut p = projet();
    assert_eq!(
        p.localisation.zone_climatique_retenue(),
        ZoneClimatique::H2b
    );
    p.localisation.zone_climatique = Some(ZoneClimatique::H3);
    assert_eq!(p.localisation.zone_climatique_retenue(), ZoneClimatique::H3);
    let r = valider(&p);
    assert!(r.est_valide());
    assert_eq!(chemins(&r.avertissements), ["localisation.zone_climatique"]);
}

#[test]
#[allow(clippy::float_cmp)]
fn calcule_les_surfaces_de_reference() {
    // Th-BCE 2020 §13.1.3, éq. (2375), (2377), (2378) : un seul groupe d'habitation,
    // SREF = SHAB = 96,44 m².
    let p = projet();
    let meteo = MeteoProjet::synthetique(ZoneClimatique::H2b);
    let brut = calculer(&p, &meteo, &Options { arrondir: false }).unwrap();
    assert_eq!(brut.batiments[0].sref_m2, 96.44);
    assert_eq!(brut.batiments[0].zones[0].groupes[0].sref_m2, 96.44);
    assert!(!brut.valeurs_arrondies);

    let arrondi = calculer(&p, &meteo, &Options::default()).unwrap();
    assert_eq!(arrondi.batiments[0].sref_m2, 96.4);
    assert!(arrondi.valeurs_arrondies);
    assert!(
        !arrondi.reglementaire,
        "un jeu météo synthétique n'est jamais réglementaire"
    );
    // Le Bbio (lot 4d2), le DH (lot 5), Cep et Cep,nr (lot 7) sont calculés.
    assert!(arrondi.batiments[0].bbio_pts.is_some_and(|b| b > 0.0));
    assert!(arrondi.batiments[0].cep_kwhep_m2.is_some_and(|c| c > 0.0));
    assert!(arrondi.batiments[0].dh_c_h.is_some_and(|dh| dh >= 0.0));
}

#[test]
#[allow(clippy::float_cmp)]
fn somme_les_groupes_et_les_zones() {
    let mut p = projet();
    let mut g2 = p.batiments[0].zones[0].groupes[0].clone();
    g2.id = "second".into();
    g2.shab_m2 = Some(10.0);
    p.batiments[0].zones[0].groupes.push(g2);
    let mut z2 = p.batiments[0].zones[0].clone();
    z2.id = "autre-logement".into();
    p.batiments[0].zones.push(z2);
    let r = calculer(
        &p,
        &MeteoProjet::synthetique(ZoneClimatique::H2b),
        &Options { arrondir: false },
    )
    .unwrap();
    assert_eq!(r.batiments[0].zones[0].sref_m2, 96.44 + 10.0);
    assert_eq!(r.batiments[0].sref_m2, 2.0 * (96.44 + 10.0));
}

#[test]
fn chaque_erreur_de_saisie_designe_l_element_fautif() {
    let mut p = projet();
    p.localisation.alt_m = -5.0;
    p.catalogue.parois[0].sf_c_k = 1.5;
    p.catalogue.baies[0].u_sp_vert_w_m2k = None;
    let g = &mut p.batiments[0].zones[0].groupes[0];
    g.shab_m2 = None;
    g.parois_opaques[1].type_paroi = "inconnu".into();
    g.parois_opaques[2].exposition.inclinaison_deg = 200.0;
    g.parois_opaques[3].id = "mur-sud".into();
    g.baies[0].exposition.masques = vec!["absent".into()];
    g.baies[1].a_b_m2 = f64::NAN;
    g.ponts_thermiques[0].l_l_m = -1.0;
    let r = valider(&p);
    let attendus = [
        "localisation.alt_m",
        "catalogue.parois[0].sf_c_k",
        "catalogue.baies[0]",
        "batiments[0].zones[0].groupes[0].shab_m2",
        "batiments[0].zones[0].groupes[0].parois_opaques[3].id",
        "batiments[0].zones[0].groupes[0].parois_opaques[1].type_paroi",
        "batiments[0].zones[0].groupes[0].parois_opaques[2].exposition.inclinaison_deg",
        "batiments[0].zones[0].groupes[0].baies[0].exposition.masques[0]",
        "batiments[0].zones[0].groupes[0].baies[1].a_b_m2",
        "batiments[0].zones[0].groupes[0].ponts_thermiques[0].l_l_m",
    ];
    assert_eq!(chemins(&r.erreurs), attendus);
    let meteo = MeteoProjet::synthetique(ZoneClimatique::H2b);
    assert!(matches!(
        calculer(&p, &meteo, &Options::default()),
        Err(Erreur::Saisie(_))
    ));
}

#[test]
fn le_type_de_baie_doit_donner_u_a_l_inclinaison_de_la_baie() {
    // Th-BCE 2020 §5.10.3.5, éq. (207) : le type d'exemple ne donne que le U vertical. Une baie à
    // 45° demande aussi le U horizontal ; une baie ne dépasse pas 90°.
    let mut p = projet();
    assert!(valider(&p).est_valide());
    p.batiments[0].zones[0].groupes[0].baies[1]
        .exposition
        .inclinaison_deg = 45.0;
    // Deux erreurs : U sans protection (éq. 207) et U avec protection mobile (éq. 208).
    assert_eq!(
        chemins(&valider(&p).erreurs),
        [
            "batiments[0].zones[0].groupes[0].baies[1].type_baie",
            "batiments[0].zones[0].groupes[0].baies[1].type_baie"
        ]
    );
    p.catalogue.baies[0].u_sp_horiz_w_m2k = Some(1.5);
    assert_eq!(valider(&p).erreurs.len(), 1);
    p.catalogue.baies[0].u_ap_horiz_w_m2k = Some(1.2);
    assert!(valider(&p).est_valide());
    p.batiments[0].zones[0].groupes[0].baies[1]
        .exposition
        .inclinaison_deg = 120.0;
    assert_eq!(
        chemins(&valider(&p).erreurs),
        ["batiments[0].zones[0].groupes[0].baies[1].exposition.inclinaison_deg"]
    );
}

#[test]
fn les_facteurs_solaires_c_se_saisissent_ensemble_et_leur_defaut_doit_etre_positif() {
    // Th-BCE 2020 §5.10.3.6.3, éq. (215), (216) et (218).
    let mut p = projet();
    let f = &mut p.catalogue.baies[0].sans_protection;
    f.sw2_c = None;
    assert_eq!(
        chemins(&valider(&p).erreurs),
        ["catalogue.baies[0].sans_protection"]
    );
    // Tout absent : valeurs par défaut, signalées par baie dans les résultats.
    let f = &mut p.catalogue.baies[0].sans_protection;
    (f.sw1_c, f.sw3_c, f.tlid) = (None, None, None);
    assert!(valider(&p).est_valide());
    let meteo = MeteoProjet::synthetique(ZoneClimatique::H2b);
    let r = calculer(&p, &meteo, &Options::default()).unwrap();
    let baies = &r.batiments[0].zones[0].groupes[0].baies;
    assert_eq!(baies.len(), 2);
    for b in baies {
        let champs: Vec<&str> = b
            .valeurs_par_defaut
            .iter()
            .map(|d| d.champ.as_str())
            .collect();
        assert_eq!(
            champs,
            [
                "sans_protection.sw1_c",
                "sans_protection.sw2_c",
                "sans_protection.sw3_c",
                "sans_protection.tlid"
            ]
        );
    }
    // Swsp-e ≤ 0,1 : la valeur par défaut de l'éq. (215) ne serait pas positive.
    let f = &mut p.catalogue.baies[0].sans_protection;
    (f.sw1_e, f.sw2_e, f.sw3_e) = (0.08, 0.02, 0.0);
    assert_eq!(
        chemins(&valider(&p).erreurs),
        ["catalogue.baies[0].sans_protection"]
    );
}

#[test]
fn les_protections_mobiles_hors_perimetre_sont_refusees() {
    use thbce_core::entree::{GestionPm, TypePm};
    // Décision Q30, point 7 : erreur explicite, sans calcul approché.
    let mut p = projet();
    assert!(valider(&p).est_valide());
    p.catalogue.baies[0]
        .protection_mobile
        .as_mut()
        .unwrap()
        .type_pm = TypePm::StoreVenitien;
    let r = valider(&p);
    assert_eq!(
        chemins(&r.erreurs),
        ["catalogue.baies[0].protection_mobile.type_pm"]
    );
    assert_eq!(
        r.erreurs[0].message,
        "store vénitien : non pris en charge par cette version (voir docs/couverture.md)"
    );
    assert!(r.seulement_hors_perimetre());
    let pm = p.catalogue.baies[0].protection_mobile.as_mut().unwrap();
    pm.type_pm = TypePm::Volet;
    pm.gestion = GestionPm::AutomatiqueAvecDerogation;
    assert_eq!(
        chemins(&valider(&p).erreurs),
        ["catalogue.baies[0].protection_mobile.gestion"]
    );
    // Une protection mobile déclarée demande les facteurs et le U de la baie protégée.
    let mut p = projet();
    p.catalogue.baies[0].avec_protection = None;
    p.catalogue.baies[0].u_ap_vert_w_m2k = None;
    assert_eq!(
        chemins(&valider(&p).erreurs),
        ["catalogue.baies[0].avec_protection", "catalogue.baies[0]"]
    );
}

#[test]
fn regles_de_saisie_de_la_surventilation_naturelle() {
    use thbce_core::entree::TypeOuvrant;
    // Th-BCE 2020 §5.13 : une baie ouvrable demande sa classe d'exposition au bruit.
    let mut p = projet();
    p.batiments[0].zones[0].groupes[0].baies[0].expo_br = None;
    assert_eq!(
        chemins(&valider(&p).erreurs),
        ["batiments[0].zones[0].groupes[0].baies[0].expo_br"]
    );
    // Rouvmax saisi ou déduit du type d'ouvrant, pas les deux ni aucun.
    let mut p = projet();
    p.catalogue.baies[0].ouvrant.as_mut().unwrap().type_ouvrant = Some(TypeOuvrant::Coulissant);
    assert_eq!(
        chemins(&valider(&p).erreurs),
        ["catalogue.baies[0].ouvrant"]
    );
    p.catalogue.baies[0].ouvrant.as_mut().unwrap().r_ouv_max = None;
    assert!(valider(&p).est_valide());
    // Maison individuelle : traversante par convention (tableau 43).
    let mut p = projet();
    p.batiments[0].zones[0].groupes[0].traversant = Some(false);
    assert_eq!(
        chemins(&valider(&p).erreurs),
        ["batiments[0].zones[0].groupes[0].traversant"]
    );
    // Hauteur de tirage thermique : saisie seulement au-delà de 4 m, jusqu'à 15 m.
    let mut p = projet();
    for (h, valide) in [
        (3.0, false),
        (4.0, false),
        (6.0, true),
        (15.0, true),
        (15.5, false),
    ] {
        p.batiments[0].zones[0].groupes[0].h_ttf_m = Some(h);
        assert_eq!(valider(&p).est_valide(), valide, "Httf = {h}");
    }
    // Logement collectif : le caractère traversant se saisit.
    let mut p = projet();
    p.batiments[0].zones[0].usage = UsageZone::LogementCollectif;
    p.batiments[0].zones[0].nb_logements = Some(2);
    // Caractère traversant au sens des conditions d'hiver (§5.6.3.1.3) : hors du sujet.
    p.batiments[0].zones[0].trav_zone = Some(true);
    assert_eq!(
        chemins(&valider(&p).erreurs),
        ["batiments[0].zones[0].groupes[0].traversant"]
    );
    p.batiments[0].zones[0].groupes[0].traversant = Some(false);
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);
}

#[test]
fn un_projet_vide_est_refuse_sans_panique() {
    let mut p = projet();
    p.batiments.clear();
    assert_eq!(chemins(&valider(&p).erreurs), ["batiments"]);
    let mut p = projet();
    p.batiments[0].zones[0].groupes.clear();
    assert_eq!(
        chemins(&valider(&p).erreurs),
        ["batiments[0].zones[0].groupes"]
    );
}

#[test]
fn un_element_hors_perimetre_est_refuse_comme_hors_perimetre() {
    use thbce_core::entree::TypePm;
    // Les 28 usages sont calculés (lot 0c4) : le refus « hors périmètre » se teste sur un
    // élément non pris en charge, ici un store vénitien (décision Q30, point 7).
    let mut p = projet();
    p.catalogue.baies[0]
        .protection_mobile
        .as_mut()
        .unwrap()
        .type_pm = TypePm::StoreVenitien;
    let r = valider(&p);
    // Élément reconnu mais hors du périmètre de cette version : erreur marquée `hors_perimetre`.
    assert_eq!(
        chemins(&r.erreurs),
        ["catalogue.baies[0].protection_mobile.type_pm"]
    );
    assert!(r.seulement_hors_perimetre());
    assert_eq!(r.erreurs[0].valeur.as_deref(), Some("store_venitien"));
    assert!(r.avertissements.is_empty(), "{:?}", r.avertissements);
    let e = calculer(
        &p,
        &MeteoProjet::synthetique(ZoneClimatique::H2b),
        &Options::default(),
    )
    .unwrap_err();
    assert!(e.hors_perimetre(), "{e}");
    assert_eq!(UsageZone::SportPrive.numero(), 28);
    assert!(UsageZone::Commerces.est_pris_en_charge());
}

#[test]
fn la_meteo_d_une_autre_zone_est_refusee() {
    let e = calculer(
        &projet(),
        &MeteoProjet::synthetique(ZoneClimatique::H1a),
        &Options::default(),
    )
    .unwrap_err();
    assert!(matches!(e, Erreur::Meteo(_)), "{e}");
}

#[test]
fn la_surface_utile_et_le_nombre_de_logements_sont_exiges() {
    // Th-BCE 2020 §4.1.2 : Agr sert à tous les usages ; §4.1.3, éq. (34) : Nb_lgt en collectif.
    let mut p = projet();
    p.batiments[0].zones[0].groupes[0].su_m2 = None;
    assert_eq!(
        chemins(&valider(&p).erreurs),
        ["batiments[0].zones[0].groupes[0].su_m2"]
    );
    // Th-BCE 2020 §8.3.2 : le volume du groupe est requis par le bilan hydrique.
    let mut p = projet();
    p.batiments[0].zones[0].groupes[0].v_m3 = None;
    assert_eq!(
        chemins(&valider(&p).erreurs),
        ["batiments[0].zones[0].groupes[0].v_m3"]
    );
    p.batiments[0].zones[0].groupes[0].v_m3 = Some(0.0);
    assert_eq!(
        chemins(&valider(&p).erreurs),
        ["batiments[0].zones[0].groupes[0].v_m3"]
    );
    let mut p = projet();
    p.batiments[0].zones[0].usage = UsageZone::LogementCollectif;
    // Le caractère traversant se saisit en collectif (§5.13.3.2.1) : hors du sujet de ce test.
    p.batiments[0].zones[0].groupes[0].traversant = Some(true);
    p.batiments[0].zones[0].trav_zone = Some(false);
    assert_eq!(
        chemins(&valider(&p).erreurs),
        ["batiments[0].zones[0].nb_logements"]
    );
    p.batiments[0].zones[0].nb_logements = Some(0);
    assert_eq!(
        chemins(&valider(&p).erreurs),
        ["batiments[0].zones[0].nb_logements"]
    );
    p.batiments[0].zones[0].nb_logements = Some(2);
    assert!(valider(&p).est_valide());
}

#[test]
fn les_adultes_equivalents_figurent_dans_les_resultats() {
    // Éq. (32) : Algt = 96,44 m² ; Nmax = 0,025 × 96,44 = 2,411 ; Nadeq = 1,75 + 0,3 × 0,661.
    let meteo = MeteoProjet::synthetique(ZoneClimatique::H2b);
    let r = calculer(&projet(), &meteo, &Options { arrondir: false }).unwrap();
    let nadeq = r.batiments[0].zones[0].nadeq.unwrap();
    assert!((nadeq - 1.9483).abs() < 1e-12, "{nadeq}");
}

#[test]
fn schema_et_version_du_format_sont_facultatifs() {
    let mut v: serde_json::Value = serde_json::from_str(ENTREE).unwrap();
    v["$schema"] = "https://example.org/thbce-entree.schema.json".into();
    v["version_format"] = 1.into();
    let p: Projet = serde_json::from_value(v.clone()).unwrap();
    assert!(valider(&p).est_valide());
    v["version_format"] = 2.into();
    let p: Projet = serde_json::from_value(v).unwrap();
    assert_eq!(chemins(&valider(&p).erreurs), ["version_format"]);
}
