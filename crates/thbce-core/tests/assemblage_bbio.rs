// SPDX-License-Identifier: GPL-3.0-or-later
//! Enchaînement horaire du calcul du Bbio (Th-BCE 2020 §5.5, §5.7 et §13.1) sur le jeu météo
//! synthétique : fermeture du bilan d'énergie, cohérence des sorties, validations de
//! l'assemblage. Les tests sur une météo réaliste sont dans `thbce-io` (`tests/bbio.rs`).
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::bati::assemblage_groupe::{nbh_occ_ref, BilanEnergie, THETA_INITIALE_C};
use thbce_core::bati::assemblage_zone::ZoneBbio;
use thbce_core::climat::calendrier::hleg;
use thbce_core::climat::{ClimatExterieur, ConfortAdaptatif, MeteoProjet};
use thbce_core::entree::{Projet, UsageZone};
use thbce_core::validation::Probleme;
use thbce_core::{calculer, valider, Erreur, Options};
use thbce_data::ZoneClimatique;

const ENTREE: &str = include_str!("../../../tests/cas/squelette-maison/entree.json");

fn projet() -> Projet {
    serde_json::from_str(ENTREE).expect("le cas de démonstration doit être lisible")
}

fn meteo() -> MeteoProjet {
    MeteoProjet::synthetique(ZoneClimatique::H2b)
}

/// Zone prête au calcul, avec le climat du jeu synthétique (10 °C à 0 m, sans soleil).
fn zone_et_climat(p: &Projet) -> (ZoneBbio, ClimatExterieur, ConfortAdaptatif) {
    let loc = &p.localisation;
    let climat = ClimatExterieur::nouveau(&meteo().th_bc, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    let zone = ZoneBbio::nouvelle(
        &p.batiments[0].zones[0],
        &p.catalogue,
        loc.alt_m,
        climat.we_kg_kg[0],
    )
    .unwrap();
    (zone, climat, confort)
}

/// Tolérance de fermeture du bilan d'énergie, relative à la somme des flux entrants.
///
/// Le schéma de Crank-Nicolson (éq. 365) et l'interpolation dans la matrice (éq. 825 et 826)
/// sont exacts pour un modèle linéaire : le bilan ne s'écarte de zéro que par les erreurs
/// d'arrondi, de l'ordre de 1e-16 par opération et accumulées sur 8 760 pas de temps. 1e-9
/// laisse plusieurs ordres de grandeur de marge tout en détectant la perte d'un flux, même
/// faible (un watt oublié pendant une heure pèse 1e-7 du bilan annuel d'une maison).
const TOLERANCE_BILAN: f64 = 1e-9;

fn assert_bilan_ferme(b: &BilanEnergie) {
    assert!(b.entrees_wh() > 1.0e6, "bilan trop faible : {b:?}");
    assert!(
        b.residu_wh().abs() <= TOLERANCE_BILAN * b.entrees_wh(),
        "résidu de {} Wh pour {} Wh entrants : {b:?}",
        b.residu_wh(),
        b.entrees_wh()
    );
}

#[test]
fn heures_d_occupation_de_reference() {
    // Éq. (66) : somme du tableau d'occupation hebdomadaire.
    let mut grille = [[0.0; 24]; 7];
    grille[0][..8].fill(1.0);
    grille[6].fill(1.0);
    assert_eq!(nbh_occ_ref(&grille), 32.0);
}

#[test]
fn le_bilan_d_energie_se_ferme_sur_l_annee() {
    let p = projet();
    let (mut zone, climat, confort) = zone_et_climat(&p);
    zone.simuler(&climat, &confort).unwrap();
    let g = &zone.groupes[0];
    let b = &g.cumuls.bilan;
    assert_bilan_ferme(b);
    // À 9,5 °C constants et sans soleil : pas de flux solaire par les baies, un besoin de
    // chauffage, aucun besoin de froid ; l'enveloppe et l'air évacuent toute la chaleur.
    assert_eq!(b.apports_solaires_baies_wh, 0.0);
    assert!(b.chauffage_wh > 0.0);
    assert_eq!(b.refroidissement_wh, 0.0);
    assert!(b.pertes_opaques_wh > 0.0 && b.pertes_air_wh > 0.0 && b.pertes_baies_wh > 0.0);
    // Le chauffage du bilan est la somme des besoins mensuels (Wh).
    let q_ch: f64 = g.cumuls.q_sys_ch_wh.iter().sum();
    assert!((q_ch - b.chauffage_wh).abs() <= 1e-9 * q_ch);
    assert_eq!(g.cumuls.q_sys_fr_wh, [0.0; 12]);
    // La masse a quitté son état initial : la variation de stock est comptée.
    assert!(g.etat.fin.theta_mq_c != THETA_INITIALE_C);
    assert!(b.stockage_wh != 0.0);
}

#[test]
fn ordre_de_grandeur_du_besoin_de_chauffage_a_temperature_constante() {
    // Contrôle indépendant du modèle horaire, à ±25 %. Dehors, 9,5 °C constants (10 °C à 0 m,
    // site à 100 m), sans soleil. Déperditions de la maison de démonstration : parois et ponts
    // thermiques 49 W/K ; baies entre 11 W/K (volets fermés) et 13 W/K ; ventilation
    // conventionnelle, 111,25 m³/h avec un échangeur à 50 % : 0,34 × 111,25 × 0,5 = 19 W/K ;
    // défauts d'étanchéité et entrées d'air sous un tirage de l'ordre de 0,3 Pa : une
    // vingtaine de m³/h, soit 7 W/K ; au total environ 88 W/K.
    // Consigne de 19 °C en occupation (16 h par jour ouvré, 24 h le week-end : 76 % du temps)
    // et de 16 °C sinon, que la température n'atteint pas en quelques heures : environ 18,5 °C
    // en moyenne. Apports internes moyens : environ 440 W.
    // Besoin ≈ 88 × (18,5 − 9,5) − 440 ≈ 350 W. Le chauffage est interdit du jour 183 au jour
    // 252 (éq. 79) : 350 W × 295 jours × 24 h ≈ 2 480 kWh.
    let r = calculer(&projet(), &meteo(), &Options { arrondir: false }).unwrap();
    let bbio = r.batiments[0].bbio.as_ref().unwrap();
    let b_ch_kwh = bbio.annuel.b_ch_kwh_m2 * 96.44;
    assert!(
        (0.75 * 2_480.0..1.25 * 2_480.0).contains(&b_ch_kwh),
        "besoin de chauffage de {b_ch_kwh} kWh"
    );
    assert_eq!(bbio.annuel.b_fr_kwh_m2, 0.0);
    // Éclairage : 0,9 W/m² au plus pendant les heures d'éclairage autorisé, sans lumière
    // naturelle ; entre 1 et 4 kWh/m².
    assert!(
        (1.0..4.0).contains(&bbio.annuel.b_ecl_kwh_m2),
        "{}",
        bbio.annuel.b_ecl_kwh_m2
    );
}

#[test]
fn sorties_du_mode_th_b_aux_trois_niveaux() {
    let r = calculer(&projet(), &meteo(), &Options { arrondir: false }).unwrap();
    let bat = &r.batiments[0];
    let bbio = bat.bbio.as_ref().unwrap();
    // Éq. (2386) : le Bbio annuel est la somme des douze Bbio mensuels.
    assert_eq!(bbio.mensuel.len(), 12);
    assert_eq!(
        bbio.annuel.bbio_pts,
        bbio.mensuel.iter().map(|m| m.bbio_pts).sum::<f64>()
    );
    assert_eq!(bat.bbio_pts, Some(bbio.annuel.bbio_pts));
    // Éq. (2385), décision Q10 : chaque Bbio mensuel combine les besoins du mois.
    for m in &bbio.mensuel {
        let attendu = 2.0 * m.b_ch_kwh_m2 + 2.0 * m.b_fr_kwh_m2 + 5.0 * m.b_ecl_kwh_m2;
        assert!((m.bbio_pts - attendu).abs() <= 1e-12 * attendu.max(1.0));
    }
    // Un seul groupe : mêmes sorties au niveau du groupe, de la zone et du bâtiment.
    let zone = &bat.zones[0];
    assert_eq!(zone.bbio.as_ref(), Some(bbio));
    assert_eq!(zone.groupes[0].bbio.as_ref(), Some(bbio));
    // Arrondi de restitution (décision Q8) : une décimale, valeurs brutes en option.
    let arrondi = calculer(&projet(), &meteo(), &Options::default()).unwrap();
    let a = arrondi.batiments[0].bbio.as_ref().unwrap();
    assert_eq!(
        a.annuel.bbio_pts,
        (bbio.annuel.bbio_pts * 10.0).round() / 10.0
    );
    assert_eq!(arrondi.batiments[0].bbio_pts, Some(a.annuel.bbio_pts));
    assert_eq!(
        a.mensuel[0].b_ch_kwh_m2,
        (bbio.mensuel[0].b_ch_kwh_m2 * 10.0).round() / 10.0
    );
    // Saisons et indicateurs de confort du groupe.
    let g = &zone.groupes[0];
    let confort = g.confort_bbio.as_ref().unwrap();
    assert!(confort.nbh_inc_cons_ch > 0 && confort.dh_inc_cons_ch_k_h > 0.0);
    assert_eq!(confort.nbh_inc_cons_fr, 0);
    // À 9,5 °C constants, le besoin de chauffage ne passe jamais sous le seuil d'arrêt : la
    // saison de chauffage s'arrête au jour 183, où le chauffage est interdit (éq. 79) et le
    // refroidissement autorisé (éq. 89), et reprend peu après le jour 253 (éq. 81).
    let saisons = g.saisons_bbio.as_ref().unwrap();
    assert_eq!(
        (saisons.j_fin_ch, saisons.j_debut_fr),
        (Some(183), Some(183))
    );
    assert!(saisons.j_debut_ch.is_some_and(|j| (253..260).contains(&j)));
    assert!(!arrondi.reglementaire);
}

#[test]
fn les_saisons_basculent_apres_le_pas_de_temps_de_9_h_legales() {
    // §4.6.3 et décision Q39 : la saison d'un groupe ne change qu'entre le pas de temps qui se
    // termine à 9 h légales et le suivant. À 9,5 °C constants, le chauffage est interdit au
    // jour 183 (éq. 79) puis redémarre après le jour 253 (éq. 81) : deux changements au moins.
    let p = projet();
    let (mut zone, climat, confort) = zone_et_climat(&p);
    let mut precedente = None;
    let mut changements = Vec::new();
    for h in 0..8760 {
        zone.pas(h, &climat, &confort).unwrap();
        let saison = zone.sorties[0].unwrap().saison;
        if precedente.is_some_and(|s| s != saison) {
            assert_eq!(hleg(h - 1), 9, "changement de saison au pas de temps {h}");
            changements.push(h);
        }
        precedente = Some(saison);
    }
    assert!(changements.len() >= 2, "{changements:?}");
    // Jour 183 : la décision est prise à la fin du pas qui se termine à 9 h légales (6 h–7 h
    // UTC en été, itération 182 × 24 + 6) et vaut à partir du pas suivant.
    assert!(changements.contains(&(182 * 24 + 7)), "{changements:?}");
}

#[test]
fn deux_groupes_dans_une_zone_de_logement_collectif() {
    // Zone de logement collectif à deux groupes, l'un deux fois plus petit et sans baie : le
    // bilan aéraulique les relie par le hall fictif (§5.5.3). Chaque bilan d'énergie se ferme,
    // et la zone rapporte la somme des énergies à la somme des surfaces (§13.1.3).
    let mut p = projet();
    let zone = &mut p.batiments[0].zones[0];
    zone.usage = UsageZone::LogementCollectif;
    zone.nb_lgt = Some(2);
    zone.trav_zone = Some(true);
    zone.groupes[0].traversant = Some(true);
    let mut g2 = zone.groupes[0].clone();
    g2.id = "second".into();
    g2.shab_m2 = Some(48.22);
    g2.su_m2 = Some(48.22);
    g2.v_m3 = Some(120.55);
    g2.baies.clear();
    g2.entrees_air = None;
    for paroi in &mut g2.parois_opaques {
        paroi.a_k_m2 /= 2.0;
    }
    zone.groupes.push(g2);
    assert!(valider(&p).est_valide(), "{:?}", valider(&p).erreurs);

    let (mut calcul, climat, confort) = zone_et_climat(&p);
    calcul.simuler(&climat, &confort).unwrap();
    for g in &calcul.groupes {
        assert_bilan_ferme(&g.cumuls.bilan);
    }
    let r = calculer(&p, &meteo(), &Options { arrondir: false }).unwrap();
    let z = &r.batiments[0].zones[0];
    let (g1, g2) = (&z.groupes[0], &z.groupes[1]);
    let (b1, b2) = (g1.bbio.as_ref().unwrap(), g2.bbio.as_ref().unwrap());
    let bz = z.bbio.as_ref().unwrap();
    assert_eq!(z.sref_m2, 96.44 + 48.22);
    let pondere = |a: f64, b: f64| (a * g1.sref_m2 + b * g2.sref_m2) / z.sref_m2;
    for (zone, a, b) in [
        (
            bz.annuel.b_ch_kwh_m2,
            b1.annuel.b_ch_kwh_m2,
            b2.annuel.b_ch_kwh_m2,
        ),
        (
            bz.annuel.b_ecl_kwh_m2,
            b1.annuel.b_ecl_kwh_m2,
            b2.annuel.b_ecl_kwh_m2,
        ),
        (bz.annuel.bbio_pts, b1.annuel.bbio_pts, b2.annuel.bbio_pts),
    ] {
        assert!(
            (zone - pondere(a, b)).abs() <= 1e-9 * zone,
            "{zone} {a} {b}"
        );
    }
    assert!(b1.annuel.b_ch_kwh_m2 > 0.0 && b2.annuel.b_ch_kwh_m2 > 0.0);
    assert_eq!(r.batiments[0].bbio.as_ref(), Some(bz));
}

fn erreurs(p: &Projet) -> Vec<Probleme> {
    valider(p).erreurs
}

#[test]
fn un_groupe_dont_l_enveloppe_conduit_plus_que_l_echange_superficiel_est_refuse() {
    // Décision Q34 point 5 : l'éq. (339) demande HTH,k+pt < Hgmqs. Inertie moyenne :
    // Hgmqs = his × Amq = (2,5 + 1,2 × 5,5) × 2,5 × 96,44 = 9,1 × 241,1 = 2 194,0 W/K.
    // Des murs à U = 30 W/(m².K) : 90 × 30 + 100 × 0,12 + 100 × 0,15 + 4 = 2 731,0 W/K.
    let mut p = projet();
    p.catalogue.parois[0].u_k_w_m2k = 30.0;
    let e = erreurs(&p);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].chemin, "batiments[0].zones[0].groupes[0]");
    let m = &e[0].message;
    assert!(m.contains("« volume-chauffe »"), "{m}");
    assert!(m.contains("HTH,k+pt = 2731.0 W/K"), "{m}");
    assert!(m.contains("Hgmqs = 2194.0 W/K"), "{m}");
    assert!(m.contains("éq. 339"), "{m}");
    assert!(matches!(
        calculer(&p, &meteo(), &Options::default()),
        Err(Erreur::Saisie(_))
    ));
    // Juste en dessous de la limite, l'entrée est acceptée.
    p.catalogue.parois[0].u_k_w_m2k = 20.0;
    assert!(erreurs(&p).is_empty());
}

#[test]
fn une_zone_etanche_et_sans_entree_d_air_est_refusee() {
    // Décision Q37 point 11 : sans défaut d'étanchéité ni entrée d'air, l'éq. (167) n'a pas de
    // solution dès que la ventilation souffle et reprend de l'air.
    let mut p = projet();
    p.batiments[0].zones[0].q4pa_surf_m3_h_m2 = Some(0.0);
    p.batiments[0].zones[0].groupes[0].entrees_air = None;
    let e = erreurs(&p);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].chemin, "batiments[0].zones[0].q4pa_surf_m3_h_m2");
    assert!(e[0].message.contains("« logement »"), "{}", e[0].message);
    assert!(e[0].message.contains("éq. 167"), "{}", e[0].message);
    // Une entrée d'air suffit à rendre le bilan soluble, comme une perméabilité non nulle.
    let mut avec_entrees = p.clone();
    avec_entrees.batiments[0].zones[0].groupes[0].entrees_air = projet().batiments[0].zones[0]
        .groupes[0]
        .entrees_air
        .clone();
    assert!(erreurs(&avec_entrees).is_empty());
    assert!(calculer(&avec_entrees, &meteo(), &Options::default()).is_ok());
    let mut permeable = p.clone();
    permeable.batiments[0].zones[0].q4pa_surf_m3_h_m2 = Some(0.6);
    assert!(calculer(&permeable, &meteo(), &Options::default()).is_ok());
    // Sans aucun débit de ventilation, une zone étanche n'a rien à équilibrer.
    for b in &mut p.batiments[0].zones[0].groupes[0].bouches_conduits {
        b.q_spec_conv_pointe_m3_h = 0.0;
        b.q_spec_conv_base_m3_h = Some(0.0);
    }
    assert!(erreurs(&p).is_empty());
    assert!(calculer(&p, &meteo(), &Options::default()).is_ok());
}
