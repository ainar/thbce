// SPDX-License-Identifier: GPL-3.0-or-later
//! Rafraîchissement par surventilation mécanique (fiche 6.12, lot 1-ven6) de bout en bout :
//! maison double flux (`tests/cas/maison-surventilation-ademe`), centrale de traitement d'air
//! (`tests/cas/bureaux-cta-surventilation-ademe`), centrale à débit variable à batteries
//! régulées sur la charge et simple flux, sur la météo ouverte ADEME (CI publique) et sur la
//! météo réglementaire (test ignoré par défaut) :
//!
//! ```text
//! THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv cargo test -p thbce-io --test surventilation -- --ignored --nocapture
//! ```
//!
//! Les consommations des ventilateurs sont recalculées à la main (éq. 779 et 780) ; une
//! surventilation jamais déclenchée laisse les résultats inchangés à l'octet près ; les effets
//! sur la thermique (DH en baisse) sont des propriétés physiques, aucun cas de référence
//! n'étant disponible.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp
)]

use std::path::Path;

use thbce_core::climat::MeteoProjet;
use thbce_core::entree::{
    ConsignesSurventilation, Projet, SurventilationMecanique, SystemeVentilation,
    TypeSurventilation,
};
use thbce_core::resultats::Resultats;
use thbce_core::{calculer, valider, Options};
use thbce_io::{json, meteo};

fn racine() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cas(nom: &str) -> Projet {
    let chemin = racine().join(format!("tests/cas/{nom}/entree.json"));
    json::lire_projet(&std::fs::read_to_string(chemin).unwrap()).unwrap()
}

fn meteo_ademe(projet: &Projet) -> MeteoProjet {
    let chemin = racine().join("docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv");
    let zone = projet.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

fn meteo_reglementaire(projet: &Projet) -> MeteoProjet {
    let chemin = std::env::var_os("THBCE_METEO").expect("définir THBCE_METEO pour lancer ce test");
    let zone = projet.localisation.zone_climatique_retenue();
    meteo::lire(&std::fs::read(chemin).unwrap(), zone).unwrap()
}

fn brut(projet: &Projet, meteo: &MeteoProjet) -> Resultats {
    calculer(projet, meteo, &Options { arrondir: false }).unwrap()
}

fn bbio(r: &Resultats) -> f64 {
    r.batiments[0].bbio_pts.unwrap()
}

fn dh(r: &Resultats) -> f64 {
    r.batiments[0].dh_c_h.unwrap()
}

fn cep(r: &Resultats) -> f64 {
    r.batiments[0].cep_kwhep_m2.unwrap()
}

fn c_aux(r: &Resultats) -> f64 {
    r.batiments[0].zones[0]
        .ventilation
        .as_ref()
        .unwrap()
        .c_aux_vent_kwh
}

/// Résultats complets en JSON, sans le compteur d'heures de surventilation (`nbh_rafnoc`) ni les
/// noms et libellés qui distinguent les deux projets comparés.
fn json_hors_compteur(r: &Resultats) -> serde_json::Value {
    fn nettoyer(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                o.retain(|k, _| !matches!(k.as_str(), "nbh_rafnoc" | "nom" | "regle"));
                o.values_mut().for_each(nettoyer);
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(nettoyer),
            _ => {}
        }
    }
    let mut v = serde_json::from_str(&json::ecrire_resultats(r)).unwrap();
    nettoyer(&mut v);
    v
}

fn nbh_rafnoc(r: &Resultats) -> (u32, u32) {
    let v = r.batiments[0].zones[0].ventilation.as_ref().unwrap();
    (
        v.th_c.as_ref().unwrap().nbh_rafnoc.unwrap(),
        v.th_d.as_ref().unwrap().nbh_rafnoc.unwrap(),
    )
}

fn surventilation_mut(p: &mut Projet) -> &mut SurventilationMecanique {
    let v = p.batiments[0].zones[0].ventilation.as_mut().unwrap();
    match &mut v.systeme {
        SystemeVentilation::DoubleFlux(d) => d.rafraichissement_surventilation.as_mut(),
        SystemeVentilation::Cta(c) => c.rafraichissement_surventilation.as_mut(),
        SystemeVentilation::SimpleFluxExtraction(s) => s.rafraichissement_surventilation.as_mut(),
        _ => None,
    }
    .unwrap()
}

/// Consignes qui ne déclenchent jamais le rafraîchissement : 50 °C de température opérative.
fn jamais(c: &mut ConsignesSurventilation) {
    c.theta_sd_dec_c = 50.0;
    c.theta_sd_arr_c = 50.0;
}

/// Débit de surventilation de chaque bouche.
fn avec_debit_rafnoc(p: &mut Projet, q: impl Fn(f64) -> f64) {
    for g in &mut p.batiments[0].zones[0].groupes {
        for b in &mut g.bouches_conduits {
            let base = b.q_spec_occ_m3_h.unwrap_or(b.q_spec_conv_pointe_m3_h);
            b.q_spec_rafnoc_m3_h = Some(q(base));
        }
    }
}

fn verifier_maison(m: &MeteoProjet, titre: &str) {
    let p = cas("maison-surventilation-ademe");
    let r = brut(&p, m);
    let reference = brut(&cas("maison-double-flux-ademe"), m);
    // Th-B : système conventionnel, sans surventilation (§6.12.1 : Cep et Th-D seulement).
    assert_eq!(bbio(&r), bbio(&reference), "{titre}");
    // Éq. (779) et (780) : 35 W hors surventilation (éq. 390 à 394), 80 + 80 W pendant.
    let (nc, nd) = nbh_rafnoc(&r);
    assert!(nc > 0 && nd > 0, "{titre}");
    let attendu = (35.0 * (8760 - nc) as f64 + 160.0 * nc as f64) / 1000.0;
    assert!(
        (c_aux(&r) - attendu).abs() < 1e-6,
        "{titre} : {} ≠ {attendu}",
        c_aux(&r)
    );
    // Air neuf frais la nuit : moins d'inconfort d'été.
    assert!(dh(&r) < dh(&reference), "{titre}");
    println!(
        "{titre} : maison, DH {:.1} (sans surventilation {:.1}), Cep {:.1} ({:.1}), {nc} h Th-C, {nd} h Th-D",
        dh(&r),
        dh(&reference),
        cep(&r),
        cep(&reference)
    );
    // Jamais déclenchée : résultats identiques à l'octet près (consommation relevée heure par
    // heure égale à la consommation déduite des puissances constantes).
    let mut p = cas("maison-surventilation-ademe");
    jamais(&mut surventilation_mut(&mut p).ete);
    let r = brut(&p, m);
    assert_eq!(nbh_rafnoc(&r), (0, 0), "{titre}");
    assert_eq!(
        json_hors_compteur(&r),
        json_hors_compteur(&reference),
        "{titre}"
    );
    // Période d'été seulement : aucune heure hors saison de refroidissement propre, donc
    // moins d'heures qu'en fonctionnement toute l'année avec les mêmes consignes.
    let mut p = cas("maison-surventilation-ademe");
    let s = surventilation_mut(&mut p);
    s.type_surventilation = TypeSurventilation::NocturneAnnee;
    s.mi_saison = Some(s.ete.clone());
    let annee = brut(&p, m);
    assert!(nbh_rafnoc(&annee).1 >= nd, "{titre}");
}

fn verifier_cta(m: &MeteoProjet, titre: &str) {
    let p = cas("bureaux-cta-surventilation-ademe");
    let r = brut(&p, m);
    let reference = brut(&cas("bureaux-cta-ademe"), m);
    assert_eq!(bbio(&r), bbio(&reference), "{titre}");
    assert!(dh(&r) < dh(&reference), "{titre}");
    // Éq. (760) et (761) : surface par défaut MAX(3 000 ; 6 000) × 0,05 = 300 m².
    let v = r.batiments[0].zones[0].ventilation.as_ref().unwrap();
    let a: Vec<f64> = v
        .valeurs_par_defaut
        .iter()
        .filter(|d| d.champ.ends_with("a_cond_m2"))
        .map(|d| d.valeur)
        .collect();
    assert_eq!(a, [300.0, 300.0], "{titre}");
    println!(
        "{titre} : bureaux CTA, DH {:.1} (sans {:.1}), Cep {:.1} ({:.1}), heures {:?}",
        dh(&r),
        dh(&reference),
        cep(&r),
        cep(&reference),
        nbh_rafnoc(&r)
    );
    // Débits de surventilation au plus égaux au débit maximal (surface inchangée) et
    // régulation jamais déclenchée : résultats identiques à l'octet près.
    let mut p = cas("bureaux-cta-surventilation-ademe");
    avec_debit_rafnoc(&mut p, |q| q);
    let s = surventilation_mut(&mut p);
    jamais(&mut s.ete);
    jamais(s.mi_saison.as_mut().unwrap());
    let r = brut(&p, m);
    assert_eq!(nbh_rafnoc(&r), (0, 0), "{titre}");
    assert_eq!(
        json_hors_compteur(&r),
        json_hors_compteur(&reference),
        "{titre}"
    );
}

/// Centrale à débit variable à batteries régulées sur la charge, en freecooling toute
/// l'année : le mode actif n'est pas recalculé pendant la surventilation (note du §6.12.3.4,
/// lecture Q230 point 6), le calcul aboutit et le DH baisse.
fn verifier_dav(m: &MeteoProjet, titre: &str) {
    let reference = brut(&cas("bureaux-cta-dav-ademe"), m);
    let mut p = cas("bureaux-cta-dav-ademe");
    let consignes = ConsignesSurventilation {
        h_leg_deb_h: None,
        h_leg_fin_h: None,
        theta_sd_dec_c: 24.0,
        theta_sd_arr_c: 21.0,
        delta_theta_lim_k: 2.0,
        theta_ext_c: 12.0,
    };
    if let SystemeVentilation::Cta(c) = &mut p.batiments[0].zones[0]
        .ventilation
        .as_mut()
        .unwrap()
        .systeme
    {
        c.rafraichissement_surventilation = Some(SurventilationMecanique {
            type_surventilation: TypeSurventilation::FreecoolingAnnee,
            ete: consignes.clone(),
            mi_saison: Some(consignes),
            p_vent_rafnoc_rep_w: 1500.0,
            p_vent_rafnoc_souf_w: 1700.0,
        });
    }
    avec_debit_rafnoc(&mut p, |q| q);
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);
    let r = brut(&p, m);
    let (nc, nd) = nbh_rafnoc(&r);
    assert!(nc > 0 && nd > 0, "{titre}");
    assert!(dh(&r) < dh(&reference), "{titre}");
    println!(
        "{titre} : bureaux CTA DAV, DH {:.1} (sans {:.1}), Cep {:.1} ({:.1}), heures ({nc}, {nd})",
        dh(&r),
        dh(&reference),
        cep(&r),
        cep(&reference)
    );
}

/// Simple flux par extraction (bureaux) : consommation répartie au prorata des débits repris
/// (éq. 642) dans le relevé horaire ; jamais déclenchée, elle égale celle de l'exemple.
fn verifier_simple_flux(m: &MeteoProjet, titre: &str) {
    let reference = brut(&cas("bureaux-simple-flux-ademe"), m);
    let mut p = cas("bureaux-simple-flux-ademe");
    let consignes = ConsignesSurventilation {
        h_leg_deb_h: None,
        h_leg_fin_h: None,
        theta_sd_dec_c: 24.0,
        theta_sd_arr_c: 21.0,
        delta_theta_lim_k: 2.0,
        theta_ext_c: 12.0,
    };
    if let SystemeVentilation::SimpleFluxExtraction(s) = &mut p.batiments[0].zones[0]
        .ventilation
        .as_mut()
        .unwrap()
        .systeme
    {
        s.rafraichissement_surventilation = Some(SurventilationMecanique {
            type_surventilation: TypeSurventilation::FreecoolingEte,
            ete: consignes,
            mi_saison: None,
            p_vent_rafnoc_rep_w: 1200.0,
            p_vent_rafnoc_souf_w: 0.0,
        });
    }
    avec_debit_rafnoc(&mut p, |q| 1.5 * q);
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);
    let r = brut(&p, m);
    assert!(nbh_rafnoc(&r).1 > 0 && dh(&r) < dh(&reference), "{titre}");
    let g = &r.batiments[0].zones[0].groupes[0];
    assert!(
        (g.ventilation.as_ref().unwrap().c_aux_vent_kwh - c_aux(&r)).abs() < 1e-6,
        "{titre}"
    );
    // Débit au plus égal au débit maximal (surface de conduit inchangée, éq. 760) et
    // régulation jamais déclenchée.
    avec_debit_rafnoc(&mut p, |q| q);
    jamais(&mut surventilation_mut(&mut p).ete);
    let r = brut(&p, m);
    assert_eq!(
        json_hors_compteur(&r),
        json_hors_compteur(&reference),
        "{titre}"
    );
}

#[test]
fn surventilation_sur_la_meteo_ademe() {
    let m = meteo_ademe(&cas("maison-surventilation-ademe"));
    verifier_maison(&m, "ADEME");
    verifier_cta(&m, "ADEME");
    verifier_dav(&m, "ADEME");
    verifier_simple_flux(&m, "ADEME");
}

#[test]
#[ignore = "nécessite le fichier météo réglementaire (THBCE_METEO)"]
fn surventilation_sur_la_meteo_reglementaire() {
    let m = meteo_reglementaire(&cas("maison-surventilation-ademe"));
    // Valeurs phares des deux exemples (nombres agrégés annuels, calculés par le moteur le
    // 19/11/2026, non comparés à un logiciel de référence).
    for (nom, attendus) in [
        ("maison-surventilation-ademe", [52.7, 135.4, 901.2]),
        ("bureaux-cta-surventilation-ademe", [82.9, 89.7, 329.0]),
    ] {
        let r = brut(&cas(nom), &m);
        for (obtenu, attendu) in [bbio(&r), cep(&r), dh(&r)].into_iter().zip(attendus) {
            assert!(
                (obtenu - attendu).abs() < 0.05,
                "{nom} : {obtenu} au lieu de {attendu}"
            );
        }
    }
    verifier_maison(&m, "réglementaire");
    verifier_cta(&m, "réglementaire");
    verifier_dav(&m, "réglementaire");
    verifier_simple_flux(&m, "réglementaire");
}

/// Contrôles de saisie (tableau 75, éq. 765 et 767) : erreurs de saisie (code 2), jamais un
/// refus de périmètre.
#[test]
fn surventilation_validation() {
    let chemins = |p: &Projet| -> Vec<String> {
        let r = valider(p);
        assert!(r.erreurs.iter().all(|e| !e.hors_perimetre));
        r.erreurs.into_iter().map(|e| e.chemin).collect()
    };
    let base = cas("maison-surventilation-ademe");
    assert!(chemins(&base).is_empty());
    let cs =
        "batiments[0].zones[0].ventilation.systeme.double_flux.rafraichissement_surventilation";
    // Débit de bouche manquant.
    let mut p = base.clone();
    p.batiments[0].zones[0].groupes[0].bouches_conduits[0].q_spec_rafnoc_m3_h = None;
    assert_eq!(
        chemins(&p),
        ["batiments[0].zones[0].groupes[0].bouches_conduits[0].q_spec_rafnoc_m3_h"]
    );
    // Toute l'année sans consignes de mi-saison.
    let mut p = base.clone();
    surventilation_mut(&mut p).type_surventilation = TypeSurventilation::NocturneAnnee;
    assert_eq!(chemins(&p), [format!("{cs}.mi_saison")]);
    // Éq. (767) : début hors de la période nuit ; fin antérieure au début.
    for (deb, fin) in [(12, 6), (2, 23), (6, 2)] {
        let mut p = base.clone();
        let e = &mut surventilation_mut(&mut p).ete;
        e.h_leg_deb_h = Some(deb);
        e.h_leg_fin_h = Some(fin);
        assert_eq!(
            chemins(&p),
            [format!("{cs}.ete.h_leg_deb_h")],
            "{deb} {fin}"
        );
    }
    // Nocturne sans période.
    let mut p = base.clone();
    surventilation_mut(&mut p).ete.h_leg_fin_h = None;
    assert_eq!(chemins(&p), [format!("{cs}.ete.h_leg_deb_h")]);
    // Consigne hors plage (−20 à 50 °C).
    let mut p = base.clone();
    surventilation_mut(&mut p).ete.theta_ext_c = -30.0;
    assert_eq!(chemins(&p), [format!("{cs}.ete.theta_ext_c")]);
    // Forme de compatibilité sans description : erreur de saisie.
    let mut p = cas("maison-double-flux-ademe");
    if let SystemeVentilation::DoubleFlux(d) = &mut p.batiments[0].zones[0]
        .ventilation
        .as_mut()
        .unwrap()
        .systeme
    {
        d.surventilation_mecanique = Some(true);
    }
    assert_eq!(
        chemins(&p),
        ["batiments[0].zones[0].ventilation.systeme.double_flux.surventilation_mecanique"]
    );
    // Booléen faux avec une description présente : incohérent (comme le rafraîchissement
    // évaporatif d'un double flux).
    let mut p = cas("maison-surventilation-ademe");
    if let SystemeVentilation::DoubleFlux(d) = &mut p.batiments[0].zones[0]
        .ventilation
        .as_mut()
        .unwrap()
        .systeme
    {
        d.surventilation_mecanique = Some(false);
    }
    assert_eq!(
        chemins(&p),
        ["batiments[0].zones[0].ventilation.systeme.double_flux.surventilation_mecanique"]
    );
    let mut p = cas("bureaux-cta-surventilation-ademe");
    if let SystemeVentilation::Cta(c) = &mut p.batiments[0].zones[0]
        .ventilation
        .as_mut()
        .unwrap()
        .systeme
    {
        c.surventilation_mecanique = Some(false);
    }
    assert_eq!(
        chemins(&p),
        ["batiments[0].zones[0].ventilation.systeme.cta.surventilation_mecanique"]
    );
    // Débit saisi sans surventilation : avertissement.
    let mut p = cas("maison-double-flux-ademe");
    p.batiments[0].zones[0].groupes[0].bouches_conduits[0].q_spec_rafnoc_m3_h = Some(100.0);
    let r = valider(&p);
    assert!(r.est_valide());
    assert_eq!(r.avertissements.len(), 1);
}
