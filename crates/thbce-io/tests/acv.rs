// SPDX-License-Identifier: GPL-3.0-or-later
//! ACV (lot ACV, annexe II chapitre 4) de l'exemple `tests/cas/maison-acv-ademe` sur la météo
//! réglementaire : valeurs phares (nombres annuels agrégés seulement). Les impacts des
//! composants sont fictifs (voir `cas.toml`) ; la météo n'agit que sur Ic énergie, Icbâtiment et
//! le taux d'autoconsommation du lot 13.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::{calculer, Options};
use thbce_io::{json, meteo};

#[test]
#[ignore = "météo réglementaire requise : THBCE_METEO"]
fn acv_maison_meteo_reglementaire() {
    let chemin = std::env::var_os("THBCE_METEO").expect("THBCE_METEO");
    let octets = std::fs::read(chemin).unwrap();
    let entree =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/cas/maison-acv-ademe/entree.json");
    let p = json::lire_projet(&std::fs::read_to_string(entree).unwrap()).unwrap();
    let m = meteo::lire(&octets, p.localisation.zone_climatique_retenue()).unwrap();
    let r = calculer(&p, &m, &Options { arrondir: true }).unwrap();
    let b = &r.batiments[0];
    let a = b.acv.as_ref().unwrap();
    let e = a.exigences.as_ref().unwrap();
    println!(
        "Ic énergie {:?}, TAC {:?}, Iccomposants {}, Icconstruction {} (max {}), Icbâtiment {:?}, \
         Icprojet {:?}, StockC {}",
        a.ic_energie_kgco2_m2,
        a.tac_pv_pct,
        a.ic_composants_kgco2_m2,
        a.ic_construction_kgco2_m2,
        e.ic_construction_max_kgco2_m2,
        a.ic_batiment_kgco2_m2,
        a.ic_projet_kgco2_m2,
        a.stock_c_kgc_m2
    );
    // Valeurs phares à l'ajout du lot ACV (arrondies à une décimale).
    assert_eq!(a.ic_energie_kgco2_m2, Some(167.1));
    assert_eq!(a.tac_pv_pct, Some(46.9));
    assert_eq!(a.ic_composants_kgco2_m2, 526.9);
    assert_eq!(a.ic_construction_kgco2_m2, 537.1);
    assert_eq!(e.ic_construction_max_kgco2_m2, 537.0);
    assert!(!e.respectee);
    assert_eq!(a.ic_batiment_kgco2_m2, Some(783.0));
    assert_eq!(a.stock_c_kgc_m2, 11.2);
}
