// SPDX-License-Identifier: GPL-3.0-or-later
//! ACV (annexe II, chapitre 4) : recalculs à la main sur `tests/cas/maison-acv-ademe` et ACV d'usages
//! non résidentiels (bureaux, hôtel) construits à partir des exemples `bureaux-simple-flux-ademe`
//! et `hotel-pac-ademe` auxquels on ajoute des données d'ACV FICTIVES (aucun lien avec une FDES, un
//! PEP, une DED ou une donnée de service publiée). Météo ouverte ADEME ; sans valeur
//! réglementaire.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::float_cmp)]

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use thbce_core::resultats::Resultats;
use thbce_core::{calculer, Options};
use thbce_io::{json, meteo};

fn racine() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn entree(cas: &str) -> Value {
    let chemin = racine().join("tests/cas").join(cas).join("entree.json");
    serde_json::from_str(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn calcul(v: &Value) -> Result<Resultats, String> {
    let p = json::lire_projet(&v.to_string()).unwrap();
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    let m = meteo::lire(
        &std::fs::read(chemin).unwrap(),
        p.localisation.zone_climatique_retenue(),
    )
    .unwrap();
    calculer(&p, &m, &Options { arrondir: false }).map_err(|e| match e {
        thbce_core::Erreur::Saisie(r) => r
            .erreurs
            .iter()
            .map(|x| format!("{} : {}", x.chemin, x.message))
            .collect::<Vec<_>>()
            .join(" ; "),
        autre => autre.to_string(),
    })
}

fn proche(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() < tol, "{a} ≠ {b}");
}

/// Recalculs à la main du relecteur sur la maison de l'exemple (valeurs non arrondies ; la
/// météo ADEME n'agit que sur le taux d'autoconsommation du lot 13).
#[test]
fn maison_acv_recalculs_du_relecteur() {
    let r = calcul(&entree("maison-acv-ademe")).unwrap();
    let a = r.batiments[0].acv.as_ref().unwrap();
    proche(a.ic_chantier_kgco2_m2, 10.2074, 5e-5);
    proche(a.ic_eau_kgco2_m2, 78.7252, 5e-5);
    proche(a.stock_c_kgc_m2, 11.2401, 5e-5);
    proche(a.ic_ded_kgco2_m2, 335.2028, 5e-5);
    proche(a.udd_batiment_pct.unwrap(), 70.387, 5e-4);
    proche(
        a.exigences.as_ref().unwrap().ic_construction_max_kgco2_m2,
        536.79248,
        5e-6,
    );
    let lot13 = a.lots.iter().find(|l| l.lot == 13).unwrap();
    proche(lot13.ic_kgco2_m2, 18.7897, 5e-5);
    let l6 = a.lots.iter().find(|l| l.lot == 6).unwrap();
    let fenetres = l6.sous_lots.iter().find(|s| s.sous_lot == "6.2").unwrap();
    proche(fenetres.contribution.ic.total, 27.0710, 5e-5);
    // Permis de 2025 : avertissement sur la version de l'exigence (Q213).
    assert!(r.avertissements.iter().any(|m| m.contains("post-GTM2")));
}

fn donnee(id: &str, dvr: u32, prod: f64, edif: f64, fdv: f64, d: f64) -> Value {
    json!({
        "type": "fdes_collective", "identifiant": id,
        "source": "valeur fictive (test acv_exemples), sans lien avec une donnée publiée",
        "dvr_ans": dvr, "production_kgco2": prod, "edification_kgco2": edif,
        "exploitation_kgco2": 0.0, "fin_de_vie_kgco2": fdv, "module_d_kgco2": d
    })
}

fn acv_fictif(occupants: Value, eau_extra: Value) -> Value {
    let mut eau = json!({"assainissement": "collectif", "eaux_pluviales": "separatif"});
    for (k, v) in eau_extra.as_object().unwrap() {
        eau[k] = v.clone();
    }
    json!({
        "composants": [
            {"id": "structure", "designation": "structure béton", "sous_lot": "3.2",
             "quantite": 2000.0, "unite": "m2", "donnee": donnee("FDES-FICTIVE-A", 100, 60.0, 3.0, 5.0, -2.0)},
            {"id": "baies", "designation": "baies", "sous_lot": "6.2",
             "quantite": 300.0, "unite": "m2", "donnee": donnee("FDES-FICTIVE-B", 30, 90.0, 4.0, 6.0, -10.0)}
        ],
        "lots_forfaitaires": ["10", "11"],
        "chantier": {"methode": "simplifie_autres", "n_ete_grue": 6.0, "n_hiver_grue": 6.0,
                     "n_ete_sans_grue": 0.0, "n_hiver_sans_grue": 0.0, "v_terres_excavees_m3": 0.0},
        "eau": eau,
        "occupants": occupants,
        "donnees_service": {"eau_potable_kgco2_m3": 0.4, "assainissement_collectif_kgco2_m3": 0.6,
                            "electricite_chantier_kgco2_kwh": 0.06, "carburant_kgco2_l": 3.0,
                            "source": "valeurs fictives pour le test, sans lien avec les données publiées"}
    })
}

/// Σ fCO2(1..50) de l'article 11 de l'arrêté du 4 août 2021, sommée à la main : 39,543.
const SOMME_F_CO2: f64 = 39.543;

#[test]
fn bureaux_acv_fictive() {
    let mut v = entree("bureaux-simple-flux-ademe");
    v["batiments"][0]["acv"] = acv_fictif(json!({"nombre": 120.0}), json!({}));
    let r = calcul(&v).unwrap();
    let b = &r.batiments[0];
    let a = b.acv.as_ref().unwrap();
    // Tableau 7 : 5,59 m³ par employé et par an (éq. 91) ; eaux usées = eau potable (éq. 92).
    proche(a.eau_quantites.q_eau_potable_conv_m3, 5.59, 1e-12);
    proche(a.eau_quantites.q_eau_potable_m3, 5.59 * 120.0, 1e-9);
    let iceau = 5.59 * 120.0 * (0.4 + 0.6) * SOMME_F_CO2 / b.sref_m2;
    proche(a.ic_eau_kgco2_m2, iceau, 1e-9);
    // Valeurs forfaitaires des lots 10 et 11 des bureaux (annexe XI) : 57+54+4 = 115 et
    // 8+7 = 15 kg/m², statiques ; l'indicateur dynamique pondère l'exploitation.
    assert!(a.lots.iter().any(|l| l.lot == 10) && a.lots.iter().any(|l| l.lot == 11));
    proche(
        a.ic_construction_kgco2_m2,
        a.ic_composants_kgco2_m2 + a.ic_chantier_kgco2_m2,
        1e-9,
    );
    // Exigence des bureaux, permis de 2025 : 810 × (1 + Misurf_tot) + Migéo (50 en H2d et H3).
    let e = a.exigences.as_ref().unwrap();
    let z = &e.zones[0];
    assert_eq!(z.usage, 3);
    let s = b.sref_m2;
    proche(z.mi_surf_tot, 0.034 - 0.86 * s / 10000.0, 1e-12);
    proche(
        e.ic_construction_max_kgco2_m2,
        810.0 * (1.0 + z.mi_surf_tot) + z.mi_geo + z.mi_infra + z.mi_vrd + z.mi_pv,
        1e-9,
    );
    assert!(z.mi_geo == 0.0 || z.mi_geo == 50.0);
}

#[test]
fn hotel_acv_eau_non_communiquee() {
    // Hôtel 0, 1 et 2 étoiles : partie nuit (usage 8, 48 m³/occupant) et partie jour
    // (usage 10, « N/C » au tableau 7).
    let mut v = entree("hotel-pac-ademe");
    v["batiments"][0]["acv"] = acv_fictif(json!({"nombre": 80.0}), json!({}));
    // Sans consommation saisie : erreur de validation (et non refus d'un périmètre).
    let e = calcul(&v).unwrap_err();
    assert!(e.contains("q_eau_potable_saisie_m3"), "{e}");
    // Avec la consommation saisie : l'ACV est calculée.
    v["batiments"][0]["acv"]["eau"]["q_eau_potable_saisie_m3"] = json!(2500.0);
    let r = calcul(&v).unwrap();
    assert!(r.avertissements.iter().any(|m| m.contains("Q218")));
    let b = &r.batiments[0];
    let a = b.acv.as_ref().unwrap();
    assert_eq!(a.eau_quantites.q_eau_potable_conv_m3, 0.0);
    proche(a.eau_quantites.q_eau_potable_m3, 2500.0, 1e-12);
    let iceau = 2500.0 * (0.4 + 0.6) * SOMME_F_CO2 / b.sref_m2;
    proche(a.ic_eau_kgco2_m2, iceau, 1e-9);
    // Exigence : usages 8 et 10, Icconstruction_maxmoyen = 820 (permis de septembre 2026),
    // Misurf_tot nul, Mided nul en 2025-2027 ; pas de Mipv ni Miinfra ni Mivrd sans lots 1, 2, 13.
    let e = a.exigences.as_ref().unwrap();
    assert_eq!(e.zones.len(), 2);
    for z in &e.zones {
        assert!(z.mi_geo == 0.0 || z.mi_geo == 20.0);
        proche(z.ic_construction_max_kgco2_m2, 820.0 + z.mi_geo, 1e-9);
    }
    // Avec l'année de dépôt 2028 : 680 et Mided = −0,3 × (Icded − 300) si Icded > 300.
    let mut v2 = v.clone();
    v2["batiments"][0]["exigences"]["annee_depot_permis"] = json!(2028);
    v2["batiments"][0]["exigences"]["mois_depot_permis"] = json!(1);
    let r2 = calcul(&v2).unwrap();
    let a2 = r2.batiments[0].acv.as_ref().unwrap();
    let z = &a2.exigences.as_ref().unwrap().zones[0];
    let ded = a2.ic_ded_kgco2_m2;
    let mided = if ded > 300.0 {
        -0.3 * (ded - 300.0)
    } else {
        0.0
    };
    proche(z.mi_ded, mided, 1e-9);
    proche(
        z.ic_construction_max_kgco2_m2,
        680.0 + z.mi_geo + mided,
        1e-9,
    );
}
