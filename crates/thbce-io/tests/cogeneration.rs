// SPDX-License-Identifier: GPL-3.0-or-later
//! Système de cogénération (fiche 8.21, lot 2e) de bout en bout : exemples
//! `maison-cogeneration-ademe` et `hotel-cogeneration-ademe` sur la météo ouverte ADEME (CI
//! publique) et sur la météo réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo.csv cargo test -p thbce-io --test cogeneration -- --ignored --nocapture
//! ```
//!
//! Aucun cas de référence n'est disponible : les tests vérifient le bilan électrique du
//! bâtiment avec l'électricité produite (§13.4, éq. 2520 à 2537), le taux d'autoconsommation
//! global (éq. 2455), son emploi pour le sous-lot 8.2 de l'ACV (annexe II, éq. 16), le sens des
//! effets et les refus de la validation.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::Projet;
use thbce_core::resultats::Resultats;
use thbce_core::{calculer, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json_du_cas(nom: &str) -> serde_json::Value {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    serde_json::from_str(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn projet(v: &serde_json::Value) -> Projet {
    json::lire_projet(&v.to_string()).unwrap()
}

fn meteo_ademe(p: &Projet) -> MeteoProjet {
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    meteo::lire(
        &std::fs::read(chemin).unwrap(),
        p.localisation.zone_climatique_retenue(),
    )
    .unwrap()
}

fn calcul(v: &serde_json::Value) -> Resultats {
    let p = projet(v);
    let r = thbce_core::valider(&p);
    assert!(r.est_valide(), "{:?}", r.erreurs);
    calculer(&p, &meteo_ademe(&p), &Options { arrondir: false }).unwrap()
}

fn cogenerateur<'a>(v: &'a mut serde_json::Value, id: &str) -> &'a mut serde_json::Value {
    for g in v["batiments"][0]["generations"].as_array_mut().unwrap() {
        for x in g["generateurs"].as_array_mut().unwrap() {
            if x["id"] == id {
                return &mut x["nature"]["cogeneration"];
            }
        }
    }
    panic!("générateur {id} absent");
}

/// Électricité produite par les générateurs du bâtiment, en kWh (éq. 1518).
fn prelec_kwh(r: &Resultats) -> f64 {
    r.batiments[0]
        .generations
        .iter()
        .flat_map(|g| &g.generateurs)
        .map(|x| x.q_prelec_kwh)
        .sum()
}

#[test]
fn bilan_electrique_avec_cogeneration() {
    let v = json_du_cas("hotel-cogeneration-ademe");
    let r = calcul(&v);
    let b = &r.batiments[0];
    let i = b.indicateurs.as_ref().unwrap();
    let c = i.production_cogeneration.as_ref().unwrap();
    // La production du bâtiment est celle de ses cogénérateurs (éq. 1493, 2520 à 2522).
    assert!(c.e_prod_coge_kwh > 1000.0);
    assert!((c.e_prod_coge_kwh - prelec_kwh(&r)).abs() < 1e-6 * c.e_prod_coge_kwh);
    // Éq. (2483) et (2486).
    assert!((c.e_exportee_kwh - (c.e_prod_coge_kwh - c.e_prod_coge_ac_kwh)).abs() < 1e-9);
    assert!((c.tac_pct - 100.0 * c.e_prod_coge_ac_kwh / c.e_prod_coge_kwh).abs() < 1e-9);
    // Éq. (2455) : la chaleur est autoconsommée en totalité.
    let g = 100.0 * (c.cef_tot_comb_kwh + c.e_prod_coge_ac_kwh)
        / (c.cef_tot_comb_kwh + c.e_prod_coge_kwh);
    assert!((c.tac_global_pct - g).abs() < 1e-9 && c.tac_global_pct >= c.tac_pct);
    // Éq. (2527) à (2535) : électricité consommée = importée + autoconsommée.
    let cons = &i.electricite_consommee;
    let imp = &i.electricite_importee;
    let somme = |p: &thbce_core::indicateurs::resultats::PostesElectricite| {
        p.chauffage_kwh
            + p.refroidissement_kwh
            + p.ecs_kwh
            + p.eclairage_kwh
            + p.auxiliaires_ventilation_kwh
            + p.auxiliaires_distribution_kwh
            + p.deplacements_kwh
            + p.mobilier_kwh
    };
    assert!(
        (somme(cons) - somme(imp) - c.e_prod_coge_ac_kwh).abs() < 1e-6 * somme(cons),
        "{} {} {}",
        somme(cons),
        somme(imp),
        c.e_prod_coge_ac_kwh
    );
    // Sans production électrique (rendement électrique certifié nul), le Cep augmente.
    let mut v0 = v.clone();
    let m = &mut cogenerateur(&mut v0, "cogenerateur-mci")["module_seul"];
    m["statut"] = "certifiee".into();
    m["eta_th_pct"] = 49.6.into();
    m["eta_el_pct"] = 0.0.into();
    let r0 = calcul(&v0);
    let i0 = r0.batiments[0].indicateurs.as_ref().unwrap();
    // Le bilan existe dès qu'un cogénérateur existe : sans électricité produite, l'éq. (5) de
    // l'annexe II donne (Cef + 0)/(Cef + 0) = 100 %, non 0 %.
    let c0 = i0.production_cogeneration.as_ref().unwrap();
    assert_eq!((c0.e_prod_coge_kwh, c0.e_prod_coge_ac_kwh), (0.0, 0.0));
    assert!(c0.cef_tot_comb_kwh > 0.0);
    assert_eq!(c0.tac_global_pct, 100.0);
    assert!(
        i0.cep_kwhep_m2 > i.cep_kwhep_m2,
        "{} {}",
        i0.cep_kwhep_m2,
        i.cep_kwhep_m2
    );
}

#[test]
fn raccordement_permanent_compte_les_pertes_a_l_arret() {
    let v = json_du_cas("hotel-cogeneration-ademe");
    let r = calcul(&v);
    let mut vi = v.clone();
    cogenerateur(&mut vi, "cogenerateur-mci")["raccordement"] = "avec_isolement".into();
    let ri = calcul(&vi);
    let gaz = |r: &Resultats| {
        r.batiments[0].generations[0].generateurs[0]
            .consommations
            .chauffage
            .gaz_kwh
    };
    // Éq. (1220) : raccordé en permanence, le cogénérateur consomme « φthstab » à l'arrêt.
    assert!(gaz(&r) > gaz(&ri), "{} {}", gaz(&r), gaz(&ri));
}

#[test]
fn acv_sous_lot_8_2_au_prorata_du_tac_global() {
    // Maison à micro-cogénération, avec les données ACV de la maison d'exemple (lot 13 compris)
    // et un composant du sous-lot 8.2.
    let mut v = json_du_cas("maison-cogeneration-ademe");
    let acv = json_du_cas("maison-acv-ademe");
    v["catalogue"] = acv["catalogue"].clone();
    v["batiments"][0]["installations_pv"] = acv["batiments"][0]["installations_pv"].clone();
    let mut donnees = acv["batiments"][0]["acv"].clone();
    let mut composant = donnees["composants"][0].clone();
    composant["id"] = "micro-cogenerateur".into();
    composant["designation"] = "micro-cogénérateur".into();
    composant["sous_lot"] = "8.2".into();
    composant["quantite"] = 1.0.into();
    composant["unite"] = "u".into();
    donnees["composants"]
        .as_array_mut()
        .unwrap()
        .push(composant);
    v["batiments"][0]["acv"] = donnees;
    let r = calcul(&v);
    let b = &r.batiments[0];
    let i = b.indicateurs.as_ref().unwrap();
    let c = i.production_cogeneration.as_ref().unwrap();
    let pv = i.production_pv.as_ref().unwrap();
    // Éq. (2528), (2530) et (2531) : même taux d'autoconsommation horaire pour les deux
    // productions ; l'exportation totale est leur somme.
    assert!(pv.e_prod_pv_kwh > 0.0 && c.e_prod_coge_kwh > 0.0);
    let a = b.acv.as_ref().unwrap();
    assert!((a.tac_coge_global_pct.unwrap() - c.tac_global_pct).abs() < 1e-9);
    // Sans le composant 8.2, Icconstruction baisse d'au plus sa contribution entière.
    let mut sans = v.clone();
    sans["batiments"][0]["acv"]["composants"]
        .as_array_mut()
        .unwrap()
        .pop();
    let r_sans = calcul(&sans);
    let ic = |r: &Resultats| {
        r.batiments[0]
            .acv
            .as_ref()
            .unwrap()
            .ic_construction_kgco2_m2
    };
    assert!(ic(&r) > ic(&r_sans));
    // Un composant du sous-lot 8.2 sans cogénérateur est refusé.
    let mut sans_cogen = json_du_cas("maison-acv-ademe");
    sans_cogen["batiments"][0]["acv"] = v["batiments"][0]["acv"].clone();
    let e = thbce_core::valider(&projet(&sans_cogen));
    assert!(e.erreurs.iter().any(|x| x.message.contains("TAC_cogé")));
    assert!(!e.seulement_hors_perimetre());
}

#[test]
fn validation_de_la_cogeneration() {
    let v = json_du_cas("maison-cogeneration-ademe");
    let messages = |v: &serde_json::Value| -> Vec<String> {
        thbce_core::valider(&projet(v))
            .erreurs
            .iter()
            .map(|e| format!("{} : {}", e.chemin, e.message))
            .collect()
    };
    assert!(messages(&v).is_empty());
    // Statut « par défaut » : pas de valeur saisie.
    let mut x = v.clone();
    cogenerateur(&mut x, "micro-cogenerateur")["avec_appoint"]["eta_th_pct"] = 80.0.into();
    assert!(messages(&x).iter().any(|m| m.contains("par défaut")));
    // Statut certifié : valeurs requises.
    let mut x = v.clone();
    cogenerateur(&mut x, "micro-cogenerateur")["module_seul"]
        .as_object_mut()
        .unwrap()
        .remove("p_w_aux_kw");
    assert!(messages(&x).iter().any(|m| m.contains("p_w_aux_kw")));
    // Appoint intégré moins puissant que le module.
    let mut x = v.clone();
    cogenerateur(&mut x, "micro-cogenerateur")["avec_appoint"]["p_th_kw"] = 4.0.into();
    assert!(messages(&x).iter().any(|m| m.contains("§8.21.3.2.6")));
    // Débit au-delà du champ de la fiche.
    let mut x = v.clone();
    cogenerateur(&mut x, "micro-cogenerateur")["avec_appoint"]["p_th_kw"] = 80.0.into();
    assert!(!messages(&x).is_empty());
    // Raccordement sans valeur par défaut : champ obligatoire.
    let mut x = v.clone();
    cogenerateur(&mut x, "micro-cogenerateur")
        .as_object_mut()
        .unwrap()
        .remove("raccordement");
    assert!(json::lire_projet(&x.to_string()).is_err());
}

#[test]
#[ignore = "météo réglementaire absente du dépôt (THBCE_METEO)"]
fn cogeneration_sur_meteo_reglementaire() {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO");
    let octets = std::fs::read(chemin).unwrap();
    for nom in ["maison-cogeneration-ademe", "hotel-cogeneration-ademe"] {
        let p = projet(&json_du_cas(nom));
        let m = meteo::lire(&octets, p.localisation.zone_climatique_retenue()).unwrap();
        let r = calculer(&p, &m, &Options { arrondir: false }).unwrap();
        let b = &r.batiments[0];
        let i = b.indicateurs.as_ref().unwrap();
        let c = i.production_cogeneration.as_ref().unwrap();
        println!(
            "{nom} : Bbio {:.1}, DH {:.1}, Cep {:.1}, Cep,nr {:.1}, Ic énergie {:.1}, \
             électricité produite {:.1} kWh, TAC {:.1} %",
            b.bbio_pts.unwrap(),
            b.dh_c_h.unwrap(),
            i.cep_kwhep_m2,
            i.cep_nr_kwhep_m2,
            i.ic_energie_kgco2_m2.unwrap(),
            c.e_prod_coge_kwh,
            c.tac_pct
        );
        // Valeurs phares au lot 2e (arrondies à une décimale) : Bbio, DH, Cep, Ic énergie,
        // électricité produite.
        let phares = match nom {
            "maison-cogeneration-ademe" => (59.7, 1093.9, 85.0, 601.0, 35.6),
            _ => (103.6, 1253.6, 302.5, 708.5, 13515.5),
        };
        let d = |x: f64| (x * 10.0).round() / 10.0;
        assert_eq!(
            (
                d(b.bbio_pts.unwrap()),
                d(b.dh_c_h.unwrap()),
                d(i.cep_kwhep_m2),
                d(i.ic_energie_kgco2_m2.unwrap()),
                d(c.e_prod_coge_kwh)
            ),
            phares,
            "{nom}"
        );
    }
}

/// Cogénérateur sans électricité produite (ηel = 0) et sous-lot 8.2 de l'ACV : le taux global
/// d'autoconsommation vaut 100 % (annexe II, éq. 5) et le composant n'est pas perdu.
#[test]
fn acv_sous_lot_8_2_sans_production_electrique() {
    let mut v = json_du_cas("maison-cogeneration-ademe");
    let acv = json_du_cas("maison-acv-ademe");
    v["catalogue"] = acv["catalogue"].clone();
    v["batiments"][0]["installations_pv"] = acv["batiments"][0]["installations_pv"].clone();
    let mut donnees = acv["batiments"][0]["acv"].clone();
    let mut composant = donnees["composants"][0].clone();
    composant["id"] = "micro-cogenerateur".into();
    composant["designation"] = "micro-cogénérateur".into();
    composant["sous_lot"] = "8.2".into();
    composant["quantite"] = 1.0.into();
    composant["unite"] = "u".into();
    donnees["composants"]
        .as_array_mut()
        .unwrap()
        .push(composant);
    v["batiments"][0]["acv"] = donnees;
    let ic = |r: &Resultats| {
        r.batiments[0]
            .acv
            .as_ref()
            .unwrap()
            .ic_construction_kgco2_m2
    };
    let avec_ref = calcul(&v);
    let mut sans = v.clone();
    sans["batiments"][0]["acv"]["composants"]
        .as_array_mut()
        .unwrap()
        .pop();
    let gain_ref = ic(&avec_ref) - ic(&calcul(&sans));
    // ηel = 0 pour le module seul et pour le module avec l'appoint.
    for x in [&mut v, &mut sans] {
        let c = cogenerateur(x, "micro-cogenerateur");
        c["module_seul"]["eta_el_pct"] = 0.0.into();
        c["avec_appoint"] = serde_json::json!({
            "p_th_kw": 19.0, "statut": "certifiee",
            "eta_th_pct": 80.0, "eta_el_pct": 0.0, "p_w_aux_kw": 0.1
        });
    }
    let r = calcul(&v);
    let i = r.batiments[0].indicateurs.as_ref().unwrap();
    let c = i.production_cogeneration.as_ref().unwrap();
    assert_eq!(c.e_prod_coge_kwh, 0.0);
    assert_eq!(c.tac_global_pct, 100.0);
    let a = r.batiments[0].acv.as_ref().unwrap();
    assert_eq!(a.tac_coge_global_pct, Some(100.0));
    // Le composant compte en entier : son apport dépasse celui du cas à production électrique,
    // où le taux global est inférieur à 100 %.
    let gain = ic(&r) - ic(&calcul(&sans));
    assert!(gain > 0.0 && gain >= gain_ref - 1e-9, "{gain} {gain_ref}");
}

/// Type × combustible × fonction : chaque combinaison est valide, donne des résultats finis,
/// consomme le vecteur énergétique du tableau 136 sur le ou les postes assurés et produit de
/// l'électricité dès que le chauffage est assuré.
#[test]
fn types_combustibles_et_fonctions() {
    let base = json_du_cas("maison-cogeneration-ademe");
    let types = ["moteur_combustion_interne", "stirling", "micro_turbine"];
    // (combustible saisi, vecteur restitué).
    let combustibles = [
        ("gaz_naturel", "gaz"),
        ("gpl", "gaz"),
        ("fioul", "fioul"),
        ("bois_buches", "bois"),
    ];
    for fonction in ["chauffage", "ecs", "chauffage_ecs"] {
        for t in types {
            for (comb, vecteur) in combustibles {
                let mut v = base.clone();
                {
                    let g = &mut v["batiments"][0]["generations"][0];
                    let coge = g["generateurs"][0].clone();
                    let mut coge = coge;
                    coge["nature"]["cogeneration"]["fonction"] = fonction.into();
                    coge["nature"]["cogeneration"]["type_cogenerateur"] = t.into();
                    coge["nature"]["cogeneration"]["combustible"] = comb.into();
                    let mut gens = vec![coge];
                    match fonction {
                        // ECS assurée par une résistance qui devient la base du ballon.
                        "chauffage" => {
                            gens[0].as_object_mut().unwrap().remove("priorite_ecs");
                            gens.push(serde_json::json!({
                                "id": "resistance", "rdim": 1, "priorite_ecs": 1,
                                "nature": {"effet_joule": {"fonction": "ecs", "pn_gen_w": 3000}}
                            }));
                            g["stockages"][0]["base"]["generateur"] = "resistance".into();
                        }
                        // Chauffage assuré par une résistance.
                        "ecs" => {
                            gens[0]
                                .as_object_mut()
                                .unwrap()
                                .remove("priorite_chauffage");
                            gens.push(serde_json::json!({
                                "id": "appoint", "rdim": 1, "priorite_chauffage": 1,
                                "nature": {"effet_joule": {"fonction": "chauffage", "pn_gen_w": 15000}}
                            }));
                        }
                        _ => {}
                    }
                    g["generateurs"] = gens.into();
                }
                let r = calcul(&v);
                let i = r.batiments[0].indicateurs.as_ref().unwrap();
                let etiq = format!("{fonction} {t} {comb}");
                assert!(i.cep_kwhep_m2.is_finite() && i.cep_kwhep_m2 > 0.0, "{etiq}");
                let coge = &r.batiments[0].generations[0].generateurs[0];
                let c = &coge.consommations;
                let (ch, ecs) = match vecteur {
                    "gaz" => (c.chauffage.gaz_kwh, c.ecs.gaz_kwh),
                    "fioul" => (c.chauffage.fioul_kwh, c.ecs.fioul_kwh),
                    _ => (c.chauffage.bois_kwh, c.ecs.bois_kwh),
                };
                assert!(ch.is_finite() && ecs.is_finite(), "{etiq}");
                assert_eq!(ch > 0.0, fonction != "ecs", "{etiq} : chauffage {ch}");
                assert_eq!(ecs > 0.0, fonction != "chauffage", "{etiq} : ECS {ecs}");
                // Aucun autre combustible.
                let total = c.chauffage.total_kwh() - c.chauffage.electricite_kwh
                    + c.ecs.total_kwh()
                    - c.ecs.electricite_kwh;
                assert!((total - ch - ecs).abs() < 1e-9 * total.max(1.0), "{etiq}");
                let p = i.production_cogeneration.as_ref().unwrap();
                // ECS seule : la faible charge du ballon reste sous « Pth,chp_min » (brûleur seul,
                // sans électricité, éq. 1205) ; le taux global reste défini (100 % sans
                // production, éq. 5 de l'annexe II).
                assert!(
                    p.e_prod_coge_kwh >= 0.0 && p.tac_global_pct.is_finite(),
                    "{etiq}"
                );
                assert!(
                    fonction == "ecs" || p.e_prod_coge_kwh > 0.0,
                    "{etiq} : pas d'électricité produite"
                );
                if p.e_prod_coge_kwh == 0.0 {
                    assert_eq!(p.tac_global_pct, 100.0, "{etiq}");
                }
                assert!(coge.q_prelec_kwh.is_finite(), "{etiq}");
            }
        }
    }
}

/// Un cogénérateur d'ECS sans ballon (production instantanée) est refusé (fiche 8.21,
/// p. 739 : l'ECS est accumulée).
#[test]
fn ecs_instantanee_par_cogenerateur_refusee() {
    let mut v = json_du_cas("maison-cogeneration-ademe");
    v["batiments"][0]["generations"][0]["stockages"] = serde_json::json!([]);
    let e = thbce_core::valider(&projet(&v));
    assert!(
        e.erreurs
            .iter()
            .any(|x| x.message.contains("ballon de stockage")),
        "{:?}",
        e.erreurs
    );
}
