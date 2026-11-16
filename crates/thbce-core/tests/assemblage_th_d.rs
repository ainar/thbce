// SPDX-License-Identifier: GPL-3.0-or-later
//! Enchaînement horaire du calcul Th-D (Th-BCE 2020 §5.5, §5.7, §5.21.3.10 et §13.5) sur un
//! jeu météo synthétique : règles propres au mode, indicateur DH recalculé à la main, bilan
//! d'énergie avec les inerties séquentielle et annuelle. Les tests sur une météo réaliste sont
//! dans `thbce-io` (`tests/confort_ete.rs`).
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::float_cmp
)]

use thbce_core::bati::assemblage_groupe::{GroupeHoraire, Lectures, SortiesPasGroupe};
use thbce_core::bati::assemblage_zone::ZoneHoraire;
use thbce_core::calcul::AVERTISSEMENT_DH;
use thbce_core::climat::calendrier::{hleg, jour};
use thbce_core::climat::{ClimatExterieur, ConfortAdaptatif, MeteoProjet};
use thbce_core::conventions::ModeCalcul;
use thbce_core::entree::{InertieAnnuelle, InertieQuotidienne, InertieSequentielle, Projet};
use thbce_core::scenarios::saisons::{LectureSaisonFroidThD, Saison};
use thbce_core::{calculer, series_horaires, valider, Options};
use thbce_data::ZoneClimatique;

const ENTREE: &str = include_str!("../../../tests/cas/squelette-maison/entree.json");

/// Premier et dernier jour (indices à partir de 0) de la séquence chaude du jeu synthétique.
const JOURS_CHAUDS: std::ops::Range<usize> = 150..250;

fn projet() -> Projet {
    serde_json::from_str(ENTREE).expect("le cas de démonstration doit être lisible")
}

/// Jeu synthétique sans soleil : 10 °C toute l'année (9,5 °C à l'altitude conventionnelle du
/// site, 100 m), sauf, dans le jeu « Th-D », 30 °C (29,5 °C sur le site) du jour 150 au jour
/// 249.
///
/// Moyenne glissante de la température extérieure (éq. 1) : elle part de 9,5 °C et vaut
/// `29,5 − 20 × 0,8ⁿ` le n-ième jour suivant le premier jour chaud ; elle atteint 16 °C pour
/// n = 2 (16,7 °C). Après la séquence, elle vaut `9,5 + 20 × 0,8ⁿ` (à très peu près) et repasse
/// sous 16 °C pour n = 6 (14,7 °C). La période de confort adaptatif va donc du jour d'indice
/// 152 au jour d'indice 255.
fn meteo() -> MeteoProjet {
    let mut m = MeteoProjet::synthetique(ZoneClimatique::H2b);
    m.th_d.te0_c[JOURS_CHAUDS.start * 24..JOURS_CHAUDS.end * 24].fill(30.0);
    m
}

fn climat_th_d(p: &Projet) -> (ClimatExterieur, ConfortAdaptatif) {
    let loc = &p.localisation;
    let climat = ClimatExterieur::nouveau(&meteo().th_d, loc.departement, loc.alt_m);
    let confort = ConfortAdaptatif::nouveau(&climat.te_c);
    (climat, confort)
}

/// Calcul annuel du premier groupe du projet, avec la série de ses sorties horaires.
fn simuler(
    p: &Projet,
    mode: ModeCalcul,
    lectures: Lectures,
) -> (GroupeHoraire, Vec<SortiesPasGroupe>) {
    let (climat, confort) = climat_th_d(p);
    let mut zone = ZoneHoraire::nouvelle_selon(
        &p.batiments[0].zones[0],
        &p.catalogue,
        mode,
        lectures,
        p.localisation.alt_m,
        climat.we_kg_kg[0],
    )
    .unwrap();
    let mut serie = Vec::with_capacity(8760);
    zone.simuler_avec(&climat, &confort, |_, z| serie.push(z.sorties[0].unwrap()))
        .unwrap();
    (zone.groupes.remove(0), serie)
}

/// Même maison, avec des masses séquentielle et annuelle qui échangent avec la masse
/// quotidienne : capacités supérieures (tableau 54 ; éq. 331 et 332) et surfaces d'échange
/// inférieures (éq. 329 et 330).
fn avec_inerties_longues(p: &Projet) -> Projet {
    let mut p = p.clone();
    let i = &mut p.batiments[0].zones[0].groupes[0].inertie;
    i.quotidienne = InertieQuotidienne::Moyenne;
    i.sequentielle = InertieSequentielle::Lourde;
    i.ams_surf_m2_m2 = 1.5;
    i.annuelle = InertieAnnuelle::Personnalisee {
        cma_surf_kj_k_m2: 2000.0,
    };
    i.ama_surf_m2_m2 = 1.0;
    p
}

#[test]
fn periode_de_confort_adaptatif_du_jeu_synthetique() {
    let (_, confort) = climat_th_d(&projet());
    assert_eq!(
        (confort.premier_jour, confort.dernier_jour),
        (Some(152), Some(255))
    );
    // θrm le deuxième jour suivant le premier jour chaud : 29,5 − 20 × 0,64 = 16,7 °C.
    assert!((confort.theta_rm_c[152] - 16.7).abs() < 1e-9);
}

#[test]
fn le_dh_est_la_somme_des_depassements_recalcules_a_la_main() {
    let p = projet();
    let (_, confort) = climat_th_d(&p);
    let (g, serie) = simuler(&p, ModeCalcul::ThD, Lectures::default());
    let k = &g.cumuls.confort_ete;
    // Recalcul indépendant de l'éq. (2551) à partir de la série horaire. Maison individuelle :
    // consigne de refroidissement en occupation de 26 °C ; seuil de jour (7 h à 22 h légales)
    // min(28 ; max(26 ; 0,33·θrm + 20,8)), soit 0,33·θrm + 20,8 borné entre 26 et 28 °C ;
    // seuil de nuit 26 °C.
    let mut dh = 0.0;
    let (mut nbh, mut nbh_1c, mut nbh_2c, mut nbh_occ) = (0_u32, 0_u32, 0_u32, 0_u32);
    for (h, s) in serie.iter().enumerate() {
        let c = s
            .confort_ete
            .expect("le calcul Th-D restitue le confort d'été");
        assert_eq!(c.theta_op_c, s.fin.theta_op_c);
        let periode = (152..=255).contains(&(h / 24));
        assert_eq!(c.is_conf_adapt, periode, "pas {h}");
        let theta_rm = confort.theta_rm_c[jour(h)];
        let seuil = if (7..=22).contains(&hleg(h)) {
            (0.33 * theta_rm + 20.8).clamp(26.0, 28.0)
        } else {
            26.0
        };
        assert!((c.theta_op_conf_ch_corr_c - seuil).abs() < 1e-12, "pas {h}");
        if !(periode && c.i_occ_zone) {
            assert_eq!(c.dh_c_h, 0.0);
            continue;
        }
        nbh_occ += 1;
        let ecart = s.fin.theta_op_c - seuil;
        dh += ecart.max(0.0);
        nbh += u32::from(ecart >= 0.0);
        nbh_1c += u32::from(ecart >= 1.0);
        nbh_2c += u32::from(ecart >= 2.0);
    }
    assert!((k.dh_c_h - dh).abs() <= 1e-9 * dh, "{} et {dh}", k.dh_c_h);
    assert_eq!(
        (k.nbh_inconf, k.nbh_inconf_1c, k.nbh_inconf_2c),
        (nbh, nbh_1c, nbh_2c)
    );
    assert_eq!(k.nbh_occ_conf_adapt, nbh_occ);
    // À 29,5 °C dehors nuit et jour, la maison ne descend pas sous la température extérieure :
    // toutes les nuits occupées de la séquence sont au-dessus de 26 °C. Le DH dépasse donc
    // largement 3,5 °C × 8 h × 90 nuits ≈ 2 500 °C.h.
    assert!(dh > 2500.0, "{dh}");
    assert!(k.theta_op_max_c > 29.5 && k.theta_op_max_c < 40.0);
    assert_eq!(k.theta_op_max_occ_c, k.theta_op_max_c);
}

#[test]
fn regles_propres_au_calcul_th_d() {
    let p = projet();
    let (_, confort) = climat_th_d(&p);
    let (g, serie) = simuler(&p, ModeCalcul::ThD, Lectures::default());
    let mut froid_sans_chauffage = 0_u32;
    for (h, s) in serie.iter().enumerate() {
        // §13.5.3.1 : aucun refroidissement.
        assert_eq!(s.q_sys_fr_wh, 0.0);
        if confort.is_conf_adapt(h) && h > 152 * 24 + 12 {
            // Éq. (76), (77) et (80) : pas de chauffage en période de confort adaptatif, à
            // partir de la première bascule de 9 h légales qui suit son début.
            assert_eq!(s.q_sys_ch_wh, 0.0, "pas {h}");
            // Éq. (88) et (90), lecture retenue (Q40) : saison de refroidissement au sens des
            // occupants, sans système.
            assert_eq!(s.saison, Saison::Refroidissement, "pas {h}");
            froid_sans_chauffage += u32::from(s.fin.theta_op_c < 16.0);
        }
    }
    // Après la séquence chaude, la période de confort adaptatif dure encore six jours à
    // 9,5 °C dehors : la maison passe sous sa consigne réduite de 16 °C sans être chauffée.
    assert!(froid_sans_chauffage > 0);
    // Hors période, le chauffage fonctionne : la maison est chauffée en janvier.
    assert!(serie[..24 * 30].iter().any(|s| s.q_sys_ch_wh > 0.0));
    assert!(g.cumuls.q_sys_ch_wh[0] > 0.0);
    assert_eq!(g.cumuls.q_sys_fr_wh, [0.0; 12]);
    // La saison de refroidissement commence le premier jour de bascule de la période (jour 153
    // de 1 à 365) et s'arrête au premier jour de bascule qui la suit (éq. 90 : sans besoin de
    // froid, l'arrêt est décrété dès que la période est finie).
    assert_eq!(g.etat.saisons.j_debut_fr, Some(153));
    assert_eq!(g.etat.saisons.j_fin_fr, Some(257));
    // Les baies s'ouvrent pendant la séquence chaude (§5.13.3.1.1).
    assert!(g.cumuls.nbh_surventilation > 0);

    // Variante littérale de Q40 : jamais de saison de refroidissement sans système.
    let (g, serie) = simuler(
        &p,
        ModeCalcul::ThD,
        Lectures {
            saison_froid_th_d: LectureSaisonFroidThD::NulleSansSysteme,
            ..Lectures::default()
        },
    );
    assert!(serie.iter().all(|s| s.saison != Saison::Refroidissement));
    assert_eq!(g.etat.saisons.j_debut_fr, None);

    // Le même jeu météo en mode Th-B : un besoin de froid pendant la séquence chaude, pas de
    // confort d'été, pas d'inertie de longue période.
    let (g, serie) = simuler(&p, ModeCalcul::ThB, Lectures::default());
    assert!(g.cumuls.q_sys_fr_wh.iter().sum::<f64>() > 0.0);
    assert!(serie.iter().all(|s| s.confort_ete.is_none()));
    assert!(serie.iter().all(|s| s.inerties.phi_mq_seq_ann_w() == 0.0));
    assert_eq!(g.cumuls.confort_ete.dh_c_h, 0.0);
    assert_eq!(g.cumuls.bilan.stockage_inerties_longues_wh, 0.0);
}

#[test]
fn le_bilan_d_energie_se_ferme_en_th_d_avec_les_inerties_longues() {
    let p = avec_inerties_longues(&projet());
    assert!(valider(&p).avertissements.is_empty());
    let (avec, serie) = simuler(&p, ModeCalcul::ThD, Lectures::default());
    assert!(!avec.inerties.sans_effet());
    let b = &avec.cumuls.bilan;
    // Même tolérance que pour le calcul Th-B (`assemblage_bbio.rs`) : les schémas de
    // Crank-Nicolson des éq. (354), (358) et (365) conservent l'énergie aux arrondis près.
    assert!(b.entrees_wh() > 1.0e6, "{b:?}");
    assert!(
        b.residu_wh().abs() <= 1e-9 * b.entrees_wh(),
        "résidu de {} Wh pour {} Wh entrants : {b:?}",
        b.residu_wh(),
        b.entrees_wh()
    );
    // L'énergie stockée dans les deux masses est l'opposé de la somme des flux rendus à la
    // masse quotidienne (éq. 355 et 359, en W sur des pas d'une heure).
    let flux: f64 = serie.iter().map(|s| s.inerties.phi_mq_seq_ann_w()).sum();
    assert!(flux != 0.0);
    assert!((b.stockage_inerties_longues_wh + flux).abs() <= 1e-9 * flux.abs());
    // Notes des éq. (353) et (357) : 18 °C pendant 24 h, 19 °C pendant 14 jours.
    assert!(serie[..24]
        .iter()
        .all(|s| s.inerties.theta_mq_24h_c == 18.0));
    assert!(serie[24].inerties.theta_mq_24h_c != 18.0);
    assert!(serie[..336]
        .iter()
        .all(|s| s.inerties.theta_mq_nbjseq_c == 19.0));
    assert!(serie[336].inerties.theta_mq_nbjseq_c != 19.0);
    // Éq. (353) : θmq,24h(h) est la moyenne des 24 températures de masse précédentes.
    let h = 5000;
    let moyenne = serie[h - 24..h]
        .iter()
        .map(|s| s.fin.theta_mq_c)
        .sum::<f64>()
        / 24.0;
    assert!((serie[h].inerties.theta_mq_24h_c - moyenne).abs() < 1e-12);

    // Sans les inerties de longue période, la maison s'échauffe plus vite au début de la
    // séquence chaude : sa température opérative est plus élevée trois jours après.
    let (sans, serie_sans) = simuler(
        &p,
        ModeCalcul::ThD,
        Lectures {
            inerties_longues: false,
            ..Lectures::default()
        },
    );
    assert_eq!(sans.cumuls.bilan.stockage_inerties_longues_wh, 0.0);
    let h = (JOURS_CHAUDS.start + 3) * 24;
    assert!(serie_sans[h].fin.theta_op_c > serie[h].fin.theta_op_c + 0.2);

    // Classes « par défaut » : Cms = Cmq et Cma = Cms (tableau 54, éq. 316), donc aucun effet,
    // au bit près.
    let defaut = projet();
    let (a, _) = simuler(&defaut, ModeCalcul::ThD, Lectures::default());
    let (b, _) = simuler(
        &defaut,
        ModeCalcul::ThD,
        Lectures {
            inerties_longues: false,
            ..Lectures::default()
        },
    );
    assert!(a.inerties.sans_effet());
    assert_eq!(a.cumuls.confort_ete, b.cumuls.confort_ete);
    assert_eq!(a.etat.fin, b.etat.fin);
}

#[test]
fn dh_restitue_aux_trois_niveaux() {
    // Deux groupes dans la zone : le second, d'inertie très légère, a un autre DH.
    let mut p = projet();
    let mut g2 = p.batiments[0].zones[0].groupes[0].clone();
    g2.id = "leger".into();
    g2.inertie.quotidienne = InertieQuotidienne::TresLegere;
    p.batiments[0].zones[0].groupes.push(g2);
    let brut = calculer(&p, &meteo(), &Options { arrondir: false }).unwrap();
    let zone = &brut.batiments[0].zones[0];
    let dh: Vec<f64> = zone
        .groupes
        .iter()
        .map(|g| g.confort_ete.as_ref().unwrap().dh_c_h)
        .collect();
    assert!(dh[0] > 0.0 && dh[1] > 0.0 && dh[0] != dh[1], "{dh:?}");
    // §13.5.1 : le DH est exprimé par groupe ; la zone et le bâtiment restituent celui du
    // groupe le plus défavorable (Q40).
    let pire = dh[0].max(dh[1]);
    assert_eq!(zone.dh_c_h, Some(pire));
    assert_eq!(brut.batiments[0].dh_c_h, Some(pire));
    // Saisons du calcul Th-D et avertissements.
    let g = &zone.groupes[0];
    assert_eq!(g.saisons_th_d.as_ref().unwrap().j_debut_fr, Some(153));
    let c = g.confort_ete.as_ref().unwrap();
    assert!(c.theta_op_max_occ_c.is_some_and(|t| t <= c.theta_op_max_c));
    assert!(brut.avertissements.iter().any(|a| a == AVERTISSEMENT_DH));
    assert!(brut
        .avertissements
        .iter()
        .any(|a| a == "Cep, Cep,nr et Ic énergie ne sont pas encore calculés par le moteur"));
    // Arrondi de restitution à une décimale (décision Q8).
    let arrondi = calculer(&p, &meteo(), &Options::default()).unwrap();
    assert_eq!(
        arrondi.batiments[0].dh_c_h,
        Some((pire * 10.0).round() / 10.0)
    );

    // Sans période de confort adaptatif (jeu « Th-D » à 9,5 °C constants) : DH nul, aucune
    // heure comptée, pas de température maximale en période.
    let froid = calculer(
        &projet(),
        &MeteoProjet::synthetique(ZoneClimatique::H2b),
        &Options { arrondir: false },
    )
    .unwrap();
    let c = froid.batiments[0].zones[0].groupes[0]
        .confort_ete
        .as_ref()
        .unwrap();
    assert_eq!((c.dh_c_h, c.nbh_inconf, c.nbh_occ_conf_adapt), (0.0, 0, 0));
    assert_eq!(c.theta_op_max_occ_c, None);
    assert_eq!(froid.batiments[0].dh_c_h, Some(0.0));
    assert_eq!(froid.climat.confort_adaptatif_th_d.premier_jour, None);
}

#[test]
fn series_horaires_et_modes_pris_en_charge() {
    let p = projet();
    let m = meteo();
    let series = series_horaires(&p, &m, ModeCalcul::ThD).unwrap();
    assert_eq!(series.len(), 1);
    let s = &series[0];
    assert_eq!(
        (s.batiment.as_str(), s.zone.as_str(), s.groupe.as_str()),
        ("maison", "logement", "volume-chauffe")
    );
    assert_eq!((s.pas.len(), s.te_c.len()), (8760, 8760));
    // Température extérieure du site : 29,5 °C pendant la séquence chaude.
    assert_eq!(s.te_c[JOURS_CHAUDS.start * 24], 29.5);
    assert_eq!(s.te_c[0], 9.5);
    // La série est celle du calcul complet.
    let (g, serie) = simuler(&p, ModeCalcul::ThD, Lectures::default());
    assert_eq!(s.pas, serie);
    let dh: f64 = s.pas.iter().map(|x| x.confort_ete.unwrap().dh_c_h).sum();
    assert_eq!(dh, g.cumuls.confort_ete.dh_c_h);
    // En Th-B, la série porte les besoins et pas de confort d'été ; le jeu « Th-BC » n'a pas de
    // séquence chaude.
    let th_b = series_horaires(&p, &m, ModeCalcul::ThB).unwrap();
    assert!(th_b[0].pas.iter().all(|x| x.confort_ete.is_none()));
    assert_eq!(th_b[0].te_c[JOURS_CHAUDS.start * 24], 9.5);
    // Calcul des consommations (lot 6b) : émetteurs du projet, saisons effectives et relances ;
    // le jeu « Th-BC » froid demande du chauffage.
    let th_c = series_horaires(&p, &m, ModeCalcul::ThC).unwrap();
    assert!(th_c[0].pas.iter().map(|x| x.q_sys_ch_wh).sum::<f64>() > 0.0);
    assert!(th_c[0].pas.iter().all(|x| x.confort_ete.is_none()));
}

#[test]
fn inerties_incoherentes_signalees_par_la_validation() {
    // §5.21.3.4 : ΔCm,s−q et ΔCm,a−s sont positives ou nulles par définition ; ΔR négatif.
    let mut p = projet();
    let i = &mut p.batiments[0].zones[0].groupes[0].inertie;
    i.quotidienne = InertieQuotidienne::TresLourde; // Cmq_surf = 370, Amq_surf = 3,5
    i.sequentielle = InertieSequentielle::Legere; // Cms_surf = 250 < 370
    i.annuelle = InertieAnnuelle::Personnalisee {
        cma_surf_kj_k_m2: 100.0, // < 250
    };
    i.ams_surf_m2_m2 = 4.0; // > 3,5
    i.ama_surf_m2_m2 = 3.5; // égal : ΔR nul, cas prévu par le texte
    let r = valider(&p);
    assert!(r.est_valide());
    let chemins: Vec<&str> = r.avertissements.iter().map(|a| a.chemin.as_str()).collect();
    let c = "batiments[0].zones[0].groupes[0].inertie";
    assert_eq!(
        chemins,
        [
            format!("{c}.sequentielle"),
            format!("{c}.annuelle"),
            format!("{c}.ams_surf_m2_m2")
        ]
    );
    // Le calcul se fait, les deux inerties étant sans effet.
    let (g, _) = simuler(&p, ModeCalcul::ThD, Lectures::default());
    assert!(g.inerties.sans_effet());
}
