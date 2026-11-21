// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests de la gestion de la génération.

use super::*;
use crate::chauffage_refroidissement::generateur_direct_a_effet_joule::GenerateurEffetJoule;

fn joule(id: &str, poste: Poste, pn: f64) -> GenerateurHoraire {
    GenerateurHoraire::nouveau(
        id,
        Generateur::EffetJoule(GenerateurEffetJoule {
            fonction: poste,
            pn_gen_w: pn,
            rdim: 1.0,
        }),
    )
}

fn reseau(poste: Poste, q: f64) -> ReseauVu {
    ReseauVu {
        poste,
        hydraulique: false,
        q_sys_wh: q,
        theta_moy_c: 0.0,
        theta_dep_c: 0.0,
        theta_i_aval_eq_c: 19.0,
        relance: false,
        rat_surf_dess: 1.0,
        phi_pertes_prim_wh: 0.0,
    }
}

fn entrees(reseaux: &[ReseauVu]) -> EntreesGeneration<'_> {
    EntreesGeneration {
        reseaux,
        aut_ch: true,
        aut_fr: false,
        id_encl: true,
        theta_ext_c: 5.0,
        b_therm: 1.0,
        w_rechauf_wh: 0.0,
        theta_cw_c: 12.0,
        hleg: 12,
        sources: &[],
        climat: None,
        h: 0,
        w_ext_kg_kg: 0.005,
        demandes_groupes: &[],
    }
}

#[test]
fn sans_priorite_au_prorata_des_puissances() {
    // Deux générateurs de 1 et 3 kW, demande de 2 kWh : 500 et 1 500 Wh (éq. 1018 et 1026).
    let mut g = GenerationHoraire::nouvelle(
        "g",
        vec![
            joule("a", Poste::Chauffage, 1000.0),
            joule("b", Poste::Chauffage, 3000.0),
        ],
    );
    let r = [reseau(Poste::Chauffage, 2000.0)];
    let s = g.pas(&entrees(&r), 0).clone();
    assert!((g.generateurs[0].jeu.q_fou_wh[0] - 500.0).abs() < 1e-9);
    assert!((g.generateurs[1].jeu.q_fou_wh[0] - 1500.0).abs() < 1e-9);
    assert!((s.q_cef.valeur(Poste::Chauffage, Energie::Electricite) - 2000.0).abs() < 1e-9);
    assert_eq!(s.q_rep_wh, [0.0; 3]);
    assert_eq!(s.rat_bes_dp, vec![1.0]);
    // Génération sur air : température de fonctionnement, celle de l'air (éq. 1012).
    assert_eq!(s.theta_aval_c[0], 19.0);
    assert_eq!(s.theta_amb_c, 20.0);
}

#[test]
fn report_de_la_demande_non_assuree() {
    // 4 kW installés, demande de 5 kWh : 1 kWh reporté (éq. 1028), fourni au pas suivant
    // (éq. 1014).
    let mut g = GenerationHoraire::nouvelle(
        "g",
        vec![
            joule("a", Poste::Chauffage, 1000.0),
            joule("b", Poste::Chauffage, 3000.0),
        ],
    );
    let r = [reseau(Poste::Chauffage, 5000.0)];
    let s = g.pas(&entrees(&r), 0).clone();
    assert!((s.q_rep_wh[0] - 1000.0).abs() < 1e-9);
    assert!((s.q_fou_wh[0] - 4000.0).abs() < 1e-9);
    let r = [reseau(Poste::Chauffage, 500.0)];
    let s = g.pas(&entrees(&r), 0).clone();
    assert!((s.q_req_tot_ar_wh[0] - 1500.0).abs() < 1e-9);
    assert!((s.q_fou_wh[0] - 1500.0).abs() < 1e-9);
    assert_eq!(s.q_rep_wh[0], 0.0);
    // Bilan sur les deux pas : tout ce qui a été demandé a été fourni et consommé.
    let c = g.cumuls.cef.annuel();
    assert!((c.total() - 5500.0).abs() < 1e-9);
    assert_eq!(g.sous_dimensionnement(0), (false, false));
}

#[test]
fn cascade_et_relance() {
    // Cascade : le générateur prioritaire est sollicité jusqu'à sa puissance, le suivant
    // reçoit le reste (éq. 1029 à 1034).
    let mut a = joule("base", Poste::Chauffage, 2000.0);
    a.priorite = [1; 3];
    let mut b = joule("appoint", Poste::Chauffage, 3000.0);
    b.priorite = [2; 3];
    // Saisis dans l'ordre inverse des priorités.
    let mut g = GenerationHoraire::nouvelle("g", vec![b, a]);
    g.priorite = ModePriorite::Cascade;
    let r = [reseau(Poste::Chauffage, 2600.0)];
    let s = g.pas(&entrees(&r), 0).clone();
    assert!((g.generateurs[1].jeu.q_fou_wh[0] - 2000.0).abs() < 1e-9);
    assert!((g.generateurs[0].jeu.q_fou_wh[0] - 600.0).abs() < 1e-9);
    assert_eq!(s.q_rep_wh[0], 0.0);
    // En relance, le générateur de rang le plus élevé n'est pas pris en compte
    // (§8.17.3.5.6) : 600 Wh reportés.
    let mut r = [reseau(Poste::Chauffage, 2600.0)];
    r[0].relance = true;
    let s = g.pas(&entrees(&r), 0).clone();
    assert_eq!(g.generateurs[0].jeu.q_fou_wh[0], 0.0);
    assert!((s.q_rep_wh[0] - 600.0).abs() < 1e-9);
    assert_eq!(s.relance, [true, false]);
}

#[test]
fn postes_et_saisons() {
    // Chauffage hors saison : générateurs désactivés, demande reportée ; ECS toute l'année ;
    // réchauffeur de boucle ajouté à la case (ECS ; électricité) (éq. 1057).
    let mut g = GenerationHoraire::nouvelle(
        "g",
        vec![
            joule("ch", Poste::Chauffage, 5000.0),
            joule("ecs", Poste::Ecs, 3000.0),
        ],
    );
    let r = [reseau(Poste::Chauffage, 100.0), reseau(Poste::Ecs, 600.0)];
    let mut e = entrees(&r);
    e.aut_ch = false;
    e.w_rechauf_wh = 50.0;
    let s = g.pas(&e, 6).clone();
    assert_eq!(s.q_fou_wh, [0.0, 0.0, 600.0]);
    assert_eq!(s.q_rep_wh, [100.0, 0.0, 0.0]);
    assert!((s.q_cef.valeur(Poste::Ecs, Energie::Electricite) - 650.0).abs() < 1e-9);
    assert_eq!(s.q_cef.total_poste(Poste::Chauffage), 0.0);
    assert!((g.generateurs[1].jeu.r_fonct_ecs - 0.2).abs() < 1e-12);
    // Ni saison de chauffage ni de refroidissement : ambiance à 23 °C.
    assert_eq!(s.theta_amb_c, 23.0);
    assert_eq!(g.cumuls.q_req_mensuel_wh[2][6], 600.0);
    assert_eq!(g.generateurs[0].cumuls.nbh_hors_fonctionnement[0], 1);
    // τ = 20 % : tranche « 20 % ≤ τ < 30 % » (éq. 1524).
    assert_eq!(g.generateurs[1].cumuls.nbh_charge[2][3], 1);
}

#[test]
fn temperatures_de_fonctionnement_hydrauliques() {
    let mut g = GenerationHoraire::ideale("g");
    g.theta_dist_ch_max_c = Some(45.0);
    let mut r = [
        reseau(Poste::Chauffage, 1000.0),
        reseau(Poste::Chauffage, 500.0),
    ];
    for (x, t) in r.iter_mut().zip([32.0, 38.0]) {
        x.hydraulique = true;
        x.theta_moy_c = t;
        x.rat_surf_dess = 0.5;
    }
    // Température des réseaux : la plus haute (éq. 1010).
    let s = g.pas(&entrees(&r), 0).clone();
    assert_eq!(s.theta_aval_c[0], 38.0);
    assert!((s.rat_bes_dp[0] - 2.0 / 3.0).abs() < 1e-12);
    // Relance : température maximale des réseaux.
    r[0].relance = true;
    assert_eq!(g.pas(&entrees(&r), 0).theta_aval_c[0], 45.0);
    // Température constante.
    g.gestion_ch = GestionTemperature::Constante(50.0);
    assert_eq!(g.pas(&entrees(&r), 0).theta_aval_c[0], 50.0);
    // Hors volume chauffé, en saison de chauffage : θamb = b·θext + (1 − b)·20.
    g.en_volume_chauffe = false;
    let mut e = entrees(&r);
    e.b_therm = 0.6;
    let s = g.pas(&e, 0).clone();
    assert!((s.theta_amb_c - (0.6 * 5.0 + 0.4 * 20.0)).abs() < 1e-12);
}

#[test]
fn pompe_a_chaleur_gainee_et_appoint_en_cascade() {
    use crate::chauffage_refroidissement::sources_amont::{SourceAmont, SourceAmontGenerateur};
    use crate::chauffage_refroidissement::thermodynamique_elec::{
        FonctionnementCompresseur, GenerateurThermodynamique, LecturePasPartage, Limites, ModePac,
        PerformancesSaisies, CH_AIR_EXTERIEUR_EAU,
    };
    // PAC air/eau par défaut : 2 kW absorbés et COP 2,8 au point pivot, soit 5,6 kW ;
    // machine gainée, 50 W de ventilateurs de conduits à pleine charge.
    let pac = GenerateurThermodynamique {
        chauffage: Some(ModePac {
            matrice: CH_AIR_EXTERIEUR_EAU
                .construire(&PerformancesSaisies::PivotDefaut { pabs_w: 2000.0 }),
            limites: Limites::Aucune,
            dfou0_min: 32.0,
            lr_contmin: 1.0,
            ccp_lr_contmin: 1.0,
            t_air_lim_c: None,
        }),
        refroidissement: None,
        ecs: None,
        compresseur: FonctionnementCompresseur::ToutOuRien,
        w_aux_0_w: 40.0,
        rdim: 1.0,
        lecture_pas_partage: LecturePasPartage::Litterale,
    };
    let mut base = GenerateurHoraire::nouveau("pac", Generateur::Thermodynamique(Box::new(pac)));
    base.source = Some(SourceAmontGenerateur {
        source: SourceAmont::AirExterieur,
        p_vent_gaine_w: 50.0,
    });
    let mut appoint = joule("appoint", Poste::Chauffage, 3000.0);
    appoint.priorite = [2; 3];
    let mut g = GenerationHoraire::nouvelle("g", vec![base, appoint]);
    g.priorite = ModePriorite::Cascade;
    g.gestion_ch = GestionTemperature::Constante(32.5);
    let mut r = [reseau(Poste::Chauffage, 2800.0)];
    r[0].hydraulique = true;
    let sources = [ConditionsSource {
        theta_amont_c: 7.0,
        air_extrait_w_k: None,
    }; 2];
    let mut e = entrees(&r);
    e.sources = &sources;
    // Demande de 2 800 Wh à 7 °C : LR = 0,5 ; consommation 1 027,65625 Wh (voir le point de
    // fonctionnement de la fiche 8.23), plus les auxiliaires amont, 50 × 0,5 = 25 Wh, au
    // poste chauffage (éq. 1054 et 1055) ; l'appoint n'est pas sollicité.
    let s = g.pas(&e, 0).clone();
    assert!((s.q_fou_wh[0] - 2800.0).abs() < 1e-9);
    let attendu = 1027.65625 + 25.0;
    assert!((s.q_cef.valeur(Poste::Chauffage, Energie::Electricite) - attendu).abs() < 1e-9);
    assert!((g.generateurs[0].jeu.w_aux_wh - 65.0).abs() < 1e-9);
    assert_eq!(g.generateurs[1].jeu.q_fou_wh[0], 0.0);
    assert_eq!(g.generateurs[0].jeu.theta_amont_c, 7.0);
    // Demande de 7 000 Wh : la PAC plafonne à 5 600 Wh, l'appoint fournit 1 400 Wh.
    let mut r = [reseau(Poste::Chauffage, 7000.0)];
    r[0].hydraulique = true;
    let mut e = entrees(&r);
    e.sources = &sources;
    let s = g.pas(&e, 0).clone();
    assert!((g.generateurs[0].jeu.q_fou_wh[0] - 5600.0).abs() < 1e-9);
    assert!((g.generateurs[1].jeu.q_fou_wh[0] - 1400.0).abs() < 1e-9);
    assert_eq!(s.q_rep_wh[0], 0.0);
    // Pleine charge : 2 000 Wh absorbés, 50 Wh d'auxiliaires amont, 1 400 Wh d'appoint.
    assert!((s.q_cef.valeur(Poste::Chauffage, Energie::Electricite) - 3450.0).abs() < 1e-9);
    // COP annuel du générateur (éq. 1526) sur ces deux pas de temps.
    let c = &g.generateurs[0].cumuls;
    let cop = c.eta_eff_an(Poste::Chauffage).unwrap();
    assert!((cop - 8400.0 / (attendu + 2050.0)).abs() < 1e-9);
}

#[test]
fn sous_dimensionnement_compte_les_heures_successives() {
    let mut g = GenerationHoraire::nouvelle("g", vec![joule("a", Poste::Chauffage, 1000.0)]);
    let r = [reseau(Poste::Chauffage, 1500.0)];
    for _ in 0..7 {
        g.pas(&entrees(&r), 0);
    }
    assert_eq!(g.cumuls.nb_sousdim_max[0], 7);
    assert_eq!(g.sous_dimensionnement(0), (true, false));
    // Une heure sans report remet le compteur à zéro.
    let r = [reseau(Poste::Chauffage, 0.0)];
    for _ in 0..8 {
        g.pas(&entrees(&r), 0);
    }
    assert_eq!(g.etat.nb_sousdim[0], 0);
}

#[test]
fn alternance_monoposte_chauffage() {
    // §8.17.3.7.2.3.3, éq. (1043) à (1046), calcul à la main. Générateurs de 2 et 6 kW saisis
    // dans cet ordre : le plus puissant (6 kW) est appelé en premier.
    let mut g = GenerationHoraire::nouvelle(
        "g",
        vec![
            joule("petit", Poste::Chauffage, 2000.0),
            joule("grand", Poste::Chauffage, 6000.0),
        ],
    );
    g.priorite = ModePriorite::Alternance;
    let fourni = |g: &GenerationHoraire| {
        [
            g.generateurs[0].jeu.q_fou_wh[0],
            g.generateurs[1].jeu.q_fou_wh[0],
        ]
    };
    // 1 kWh demandé, qui tiendrait dans le petit générateur : le grand est surdimensionné,
    // son compteur passe de 20 à 19, 18… ; il fournit tant que le compteur est positif, soit
    // les heures 1 à 19.
    let r = [reseau(Poste::Chauffage, 1000.0)];
    for h in 1..=19 {
        let s = g.pas(&entrees(&r), 0).clone();
        assert_eq!(fourni(&g), [0.0, 1000.0], "heure {h}");
        assert_eq!(g.generateurs[1].n_basculement, 20 - h);
        assert_eq!(s.q_rep_wh[0], 0.0);
    }
    // Heure 20 : compteur du grand à 0, il est coupé ; le petit reçoit 1 kWh > 0 W (pas de
    // générateur après lui) : compteur réinitialisé à 20, il fournit tout.
    g.pas(&entrees(&r), 0);
    assert_eq!(g.generateurs[1].n_basculement, 0);
    assert_eq!(g.generateurs[0].n_basculement, N_BASCULEMENT_INIT);
    assert_eq!(fourni(&g), [1000.0, 0.0]);
    // 5 kWh : plus que le petit (2 kW) ; le grand est réinitialisé et fournit tout, le petit
    // ne reçoit rien (compteur 19).
    let r = [reseau(Poste::Chauffage, 5000.0)];
    g.pas(&entrees(&r), 0);
    assert_eq!(fourni(&g), [0.0, 5000.0]);
    assert_eq!(g.generateurs[0].n_basculement, 19);
    // 9 kWh : 6 kWh par le grand, 2 kWh par le petit (réinitialisé), 1 kWh reporté (éq. 1046).
    let r = [reseau(Poste::Chauffage, 9000.0)];
    let s = g.pas(&entrees(&r), 0).clone();
    assert_eq!(fourni(&g), [2000.0, 6000.0]);
    assert!((s.q_rep_wh[0] - 1000.0).abs() < 1e-9);
    assert_eq!(g.generateurs[0].n_basculement, N_BASCULEMENT_INIT);
}

#[test]
fn alternance_chauffage_et_ecs() {
    // §8.17.3.7.2.3.4, éq. (1047) à (1052) : deux générateurs idéaux « chauffage et ECS »
    // (puissance nominale conventionnelle de 1 W, ordre de la saisie à puissance égale).
    let ideal = |id: &str| {
        GenerateurHoraire::nouveau(
            id,
            Generateur::Ideal(GenerateurIdeal {
                fonction: FonctionGenerateur::ChauffageEcs,
            }),
        )
    };
    let mut g = GenerationHoraire::nouvelle("g", vec![ideal("x"), ideal("y")]);
    g.priorite = ModePriorite::Alternance;
    // 500 Wh de chauffage et 300 Wh d'ECS : la demande totale (800 Wh) dépasse le suivant
    // (1 W) ; x fournit l'ECS puis le chauffage ; y, sans demande, voit son compteur baisser.
    let r = [reseau(Poste::Chauffage, 500.0), reseau(Poste::Ecs, 300.0)];
    let s = g.pas(&entrees(&r), 0).clone();
    assert_eq!(g.generateurs[0].jeu.q_fou_wh, [500.0, 0.0, 300.0]);
    assert_eq!(g.generateurs[1].jeu.q_fou_wh, [0.0; 3]);
    assert_eq!(s.q_rep_wh, [0.0; 3]);
    assert_eq!(
        [
            g.generateurs[0].n_basculement,
            g.generateurs[1].n_basculement
        ],
        [20, 19]
    );
    // Hors saison de chauffage et hors période d'activation de l'ECS : désactivés, compteurs
    // inchangés, demande reportée.
    let mut e = entrees(&r);
    e.aut_ch = false;
    e.id_encl = false;
    let s = g.pas(&e, 0).clone();
    assert_eq!(s.q_rep_wh, [500.0, 0.0, 300.0]);
    assert_eq!(
        [
            g.generateurs[0].n_basculement,
            g.generateurs[1].n_basculement
        ],
        [20, 19]
    );
    // Une heure sans demande du réseau : les reports (800 Wh) sont servis par x, réinitialisé
    // à 20 ; y passe à 18. Puis vingt heures sans demande : les deux compteurs tombent à zéro.
    let vide: [ReseauVu; 0] = [];
    g.pas(&entrees(&vide), 0);
    assert_eq!(g.generateurs[0].jeu.q_fou_wh, [500.0, 0.0, 300.0]);
    assert_eq!(
        [
            g.generateurs[0].n_basculement,
            g.generateurs[1].n_basculement
        ],
        [20, 18]
    );
    for _ in 0..20 {
        g.pas(&entrees(&vide), 0);
    }
    assert_eq!(
        [
            g.generateurs[0].n_basculement,
            g.generateurs[1].n_basculement
        ],
        [0, 0]
    );
    // 0,5 Wh d'ECS : tient dans le suivant (1 W), x reste coupé ; y (sans suivant) est
    // réinitialisé et le fournit.
    let r = [reseau(Poste::Ecs, 0.5)];
    let s = g.pas(&entrees(&r), 0).clone();
    assert_eq!(g.generateurs[0].jeu.q_fou_wh, [0.0; 3]);
    assert_eq!(g.generateurs[1].jeu.q_fou_wh, [0.0, 0.0, 0.5]);
    assert_eq!(s.q_rep_wh, [0.0; 3]);
}
