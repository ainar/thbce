// SPDX-License-Identifier: GPL-3.0-or-later
//! Résolution d'une génération saisie et validée en modèle horaire : générateurs (tableau 111),
//! assemblages avec ballon (§9.14), chauffe-eau thermodynamiques (§9.22) et solaires.
//!
//! Un nouveau générateur ajoute ici la branche de sa nature dans [`GenerationHoraire::resoudre`].

use super::*;
use crate::chauffage_refroidissement::autres_generateurs_a_combustion::AutreGenerateurCombustion;
use crate::chauffage_refroidissement::chaudieres_gaz_fioul_bois::Chaudiere;
use crate::chauffage_refroidissement::generateur_direct_a_effet_joule::GenerateurEffetJoule;
use crate::chauffage_refroidissement::pac_moteur_gaz::PacMoteurGaz;
use crate::chauffage_refroidissement::poeles_et_inserts::Poele;
use crate::chauffage_refroidissement::reseau_de_chaleur_et_de_froid::ReseauChaleurFroid;
use crate::chauffage_refroidissement::sources_amont::{SourceAmont, SourceAmontGenerateur};
use crate::chauffage_refroidissement::systeme_de_cogeneration::Cogenerateur;
use crate::chauffage_refroidissement::thermodynamique_elec::GenerateurThermodynamique;
use crate::chauffage_refroidissement::thermodynamique_gaz::MachineAbsorption;
use crate::ecs::echangeur_ballon::{
    ApplicationAccumulateur, CirculateurPrimaire, EchangeurEcs, MOD_MIN_DEFAUT,
};
use crate::entree::{ApplicationAccumulateurSaisie, NatureEchangeurSaisie};

impl GenerationHoraire {
    /// Résout une génération saisie et validée ; `ids_et` et `ids_zones` donnent les
    /// identifiants des espaces tampons et des zones du bâtiment, dans l'ordre de leurs indices.
    pub fn resoudre(
        g: &crate::entree::Generation,
        ids_et: &[String],
        ids_zones: &[String],
        catalogue: &crate::entree::Catalogue,
    ) -> Self {
        use crate::entree::{
            FonctionEffetJoule, GestionTemperatureGeneration, NatureGenerateur, PositionGeneration,
            ThermodynamiqueGazSaisi, TypePriorite,
        };
        let en_vc = g.position == PositionGeneration::EnVolumeChauffe;
        let avec_priorite = matches!(
            g.priorite,
            Some(TypePriorite::Cascade | TypePriorite::Alternance)
        );
        let mut generateurs: Vec<GenerateurHoraire> = g
            .generateurs
            .iter()
            .filter_map(|s| {
                let rdim = f64::from(s.rdim.max(1));
                let mut source = None;
                let mut valeurs_par_defaut = Vec::new();
                let modele = match &s.nature {
                    NatureGenerateur::ThermodynamiqueElectrique(t) => {
                        let (pac, src, defauts) = GenerateurThermodynamique::resoudre(
                            t,
                            rdim,
                            |id| ids_et.iter().position(|e| e == id),
                            |id| ids_zones.iter().position(|z| z == id),
                        )?;
                        source = Some(src);
                        valeurs_par_defaut = defauts;
                        Generateur::Thermodynamique(Box::new(pac))
                    }
                    NatureGenerateur::EffetJoule(j) => {
                        Generateur::EffetJoule(GenerateurEffetJoule {
                            fonction: match j.fonction {
                                FonctionEffetJoule::Chauffage => Poste::Chauffage,
                                FonctionEffetJoule::Ecs => Poste::Ecs,
                            },
                            pn_gen_w: j.pn_gen_w,
                            rdim,
                        })
                    }
                    NatureGenerateur::Chaudiere(c) => {
                        let (ch, defauts) = resoudre_chaudiere(c, rdim, en_vc)?;
                        valeurs_par_defaut = defauts;
                        Generateur::Chaudiere(Box::new(ch))
                    }
                    NatureGenerateur::PoeleOuInsert(p) => Generateur::Poele(Poele {
                        pn_gen_kw: p.pn_gen_kw,
                        eta_pct: p.eta_pct,
                        p_aux_vent_w: p.p_aux_vent_w,
                        rdim,
                    }),
                    NatureGenerateur::ReseauDeChaleurOuDeFroid(r) => {
                        Generateur::Reseau(resoudre_reseau(r, rdim, en_vc)?)
                    }
                    // Fiches 8.24 et 8.25 : source amont sur air extérieur, sans auxiliaires
                    // amont propres.
                    NatureGenerateur::ThermodynamiqueGaz(t) => {
                        source = Some(SourceAmontGenerateur {
                            source: SourceAmont::AirExterieur,
                            p_vent_gaine_w: 0.0,
                        });
                        match t {
                            ThermodynamiqueGazSaisi::Absorption(a) => {
                                let (m, defauts) = MachineAbsorption::resoudre(a, rdim)?;
                                valeurs_par_defaut = defauts;
                                Generateur::Absorption(Box::new(m))
                            }
                            ThermodynamiqueGazSaisi::MoteurGaz(a) => {
                                let (m, defauts) = PacMoteurGaz::resoudre(a, rdim);
                                valeurs_par_defaut = defauts;
                                Generateur::MoteurGaz(Box::new(m))
                            }
                        }
                    }
                    // Natures refusées par la validation (code 3) : points d'entrée des lots
                    // à venir, qui remplacent leur branche par l'aiguillage vers leur modèle
                    // (`docs/architecture.md`, §3.10).
                    NatureGenerateur::AutreGenerateurACombustion(c) => {
                        let (m, defauts) = resoudre_autre_combustion(c, rdim, en_vc, avec_priorite);
                        valeurs_par_defaut = defauts;
                        Generateur::AutreCombustion(Box::new(m))
                    }
                    NatureGenerateur::Cogeneration(c) => {
                        let (m, defauts) = resoudre_cogeneration(c, rdim, en_vc);
                        valeurs_par_defaut = defauts;
                        Generateur::Cogeneration(Box::new(m))
                    }
                    NatureGenerateur::BoucleDEauOuDrv | NatureGenerateur::PacHybride => {
                        return None
                    }
                };
                Some(GenerateurHoraire {
                    id: s.id.clone(),
                    priorite: [
                        s.priorite_chauffage.unwrap_or(1),
                        s.priorite_refroidissement.unwrap_or(1),
                        s.priorite_ecs.unwrap_or(1),
                    ],
                    modele,
                    source,
                    valeurs_par_defaut,
                    jeu: JeuDonnees::default(),
                    n_basculement: N_BASCULEMENT_INIT,
                    cumuls: CumulsGenerateur::default(),
                })
            })
            .collect();
        let mut stockages = Self::resoudre_stockages(g, &mut generateurs, ids_et, ids_zones);
        stockages.extend(g.chauffe_eau_solaires.iter().filter_map(|c| {
            crate::ecs::chauffe_eau_solaire::resoudre(c, catalogue, |id| {
                generateurs.iter().position(|x| x.id == id)
            })
        }));
        stockages.extend(g.systemes_solaires_combines.iter().filter_map(|c| {
            crate::ecs::chauffe_eau_solaire::resoudre_ssc(c, catalogue, |id| {
                generateurs.iter().position(|x| x.id == id)
            })
        }));
        let collectifs = g
            .chauffe_eau_solaires_collectifs
            .iter()
            .filter_map(|c| {
                crate::ecs::chauffe_eau_solaire::resoudre_collectif(c, catalogue, |id| {
                    generateurs.iter().position(|x| x.id == id)
                })
            })
            .collect();
        let gestion = |x: Option<GestionTemperatureGeneration>| match x {
            Some(GestionTemperatureGeneration::Constante { theta_wm_c }) => {
                GestionTemperature::Constante(theta_wm_c)
            }
            Some(GestionTemperatureGeneration::Reseaux) | None => GestionTemperature::Reseaux,
        };
        Self {
            en_volume_chauffe: g.position == PositionGeneration::EnVolumeChauffe,
            priorite: match g.priorite {
                Some(TypePriorite::Cascade) => ModePriorite::Cascade,
                Some(TypePriorite::Alternance) => ModePriorite::Alternance,
                Some(TypePriorite::SansPriorite) | None => ModePriorite::SansPriorite,
            },
            gestion_ch: gestion(g.gestion_temperature_chauffage),
            gestion_fr: gestion(g.gestion_temperature_refroidissement),
            stockages,
            collectifs,
            ..Self::nouvelle(g.id.clone(), generateurs)
        }
    }

    /// Résout les assemblages avec ballon d'une génération (§9.14) et ses chauffe-eau
    /// thermodynamiques (§9.22), dont la pompe à chaleur est ajoutée aux générateurs.
    fn resoudre_stockages(
        g: &crate::entree::Generation,
        generateurs: &mut Vec<GenerateurHoraire>,
        ids_et: &[String],
        ids_zones: &[String],
    ) -> Vec<AssemblageBallon> {
        use crate::ecs::assemblage_ballon::RegulationGenerateur;
        use crate::ecs::ballon_de_stockage::{ua_s_defaut_w_k, Ballon, NatureBallon, F_AUX_DEFAUT};
        use crate::ecs::gestion_regulation_ballon::{
            nb_iter_vp, Programmation, DELTA_THETA_DEFAUT_K,
        };
        use crate::entree::{NatureBallonSaisie, PertesBallon, ProgrammationBallon};
        let programmation = |p: ProgrammationBallon| match p {
            ProgrammationBallon::Permanent => Programmation::Permanent,
            ProgrammationBallon::Nuit => Programmation::Nuit,
            ProgrammationBallon::Jour => Programmation::Jour,
        };
        let defaut = |champ: &str, valeur: f64, regle: &str| ValeurParDefaut {
            champ: champ.to_owned(),
            valeur,
            regle: regle.to_owned(),
        };
        let regle_delta = "Th-BCE 2020 §9.10.3.2.2 : hystérésis du thermostat par défaut, 2 K";
        let mut stockages = Vec::new();
        for s in &g.stockages {
            let Some(base) = generateurs.iter().position(|x| x.id == s.base.generateur) else {
                continue;
            };
            let mut defauts = Vec::new();
            let v = s.ballon.v_tot_l;
            let ua = match s.ballon.pertes {
                PertesBallon::Certifie { ua_s_w_k } => ua_s_w_k,
                PertesBallon::Justifie { ua_s_w_k } => 1.1 * ua_s_w_k,
                PertesBallon::Defaut { nature } => {
                    let ua = ua_s_defaut_w_k(
                        match nature {
                            NatureBallonSaisie::EffetJouleHorizontal => {
                                NatureBallon::EffetJouleHorizontal
                            }
                            NatureBallonSaisie::EffetJouleVertical => {
                                NatureBallon::EffetJouleVertical
                            }
                            NatureBallonSaisie::Autre => NatureBallon::Autre,
                        },
                        v,
                    );
                    defauts.push(defaut(
                        "ballon.pertes.defaut",
                        ua,
                        "Th-BCE 2020 §9.9.3.1, tableau 282 et éq. (1768) : coefficient de \
                         pertes par défaut du ballon, en W/K",
                    ));
                    ua
                }
            };
            if s.base.delta_theta_k.is_none() {
                defauts.push(defaut(
                    "base.delta_theta_k",
                    DELTA_THETA_DEFAUT_K,
                    regle_delta,
                ));
            }
            let zone = |z: u8| usize::from(z.clamp(1, 4)) - 1;
            let appoint = s.appoint.as_ref().and_then(|a| {
                let i = generateurs.iter().position(|x| x.id == a.generateur)?;
                if a.f_aux.is_none() && s.echangeur.is_none() {
                    defauts.push(defaut(
                        "appoint.f_aux",
                        F_AUX_DEFAUT,
                        "Th-BCE 2020 §9.9.3.1 : fraction du ballon chauffée par l'appoint par \
                         défaut",
                    ));
                }
                if a.delta_theta_k.is_none() {
                    defauts.push(defaut(
                        "appoint.delta_theta_k",
                        DELTA_THETA_DEFAUT_K,
                        regle_delta,
                    ));
                }
                Some((
                    RegulationGenerateur {
                        generateur: i,
                        programmation: programmation(a.programmation),
                        delta_theta_k: a.delta_theta_k.unwrap_or(DELTA_THETA_DEFAUT_K),
                        z_gen: zone(a.z_ap),
                        z_reg: zone(a.z_reg),
                        h_rel_ech: 0.0,
                    },
                    a.f_aux.unwrap_or(F_AUX_DEFAUT),
                ))
            });
            // Accumulateur en eau technique : quatre zones de volumes égaux (éq. 1874).
            let f_aux = appoint.filter(|_| s.echangeur.is_none()).map(|(_, f)| f);
            let mut a = AssemblageBallon::nouveau(
                s.id.clone(),
                f64::from(s.nb_assemblages.max(1)),
                Ballon::nouveau(v, ua, f_aux, s.ballon.theta_max_c),
                RegulationGenerateur {
                    generateur: base,
                    programmation: programmation(s.base.programmation),
                    delta_theta_k: s.base.delta_theta_k.unwrap_or(DELTA_THETA_DEFAUT_K),
                    // Éq. (1947) : échangeur de la base en zone inférieure.
                    z_gen: 0,
                    z_reg: zone(s.base.z_reg),
                    h_rel_ech: s.base.h_rel_ech.unwrap_or(0.0),
                },
                appoint.map(|(r, _)| r),
                nb_iter_vp(f_aux),
            );
            if let Some(x) = &s.echangeur {
                let application = x
                    .application
                    .map(|a| match a {
                        ApplicationAccumulateurSaisie::Individuel => {
                            ApplicationAccumulateur::Individuel
                        }
                        ApplicationAccumulateurSaisie::Collectif => {
                            ApplicationAccumulateur::Collectif
                        }
                    })
                    .unwrap_or_else(|| {
                        let a = ApplicationAccumulateur::selon_volume(v);
                        defauts.push(defaut(
                            "echangeur.application",
                            a.theta_cons_c(),
                            "Th-BCE 2020 §9.12.3.2.2 : accumulateur individuel sous 500 L, \
                             collectif au-delà ; valeur : consigne θcons du tableau 288, en °C",
                        ));
                        a
                    });
                let theta_cons = application.theta_cons_c();
                let nb_iter = a.nb_iter_vp;
                let ech = match &x.nature {
                    NatureEchangeurSaisie::Interne => {
                        EchangeurEcs::interne(x.p_ech_ecs_kw, theta_cons, nb_iter)
                    }
                    NatureEchangeurSaisie::Externe {
                        q_v_prim_nom_m3_h,
                        circulateur_vitesse_variable,
                        p_circ_prim_nom_w,
                        mod_min,
                    } => {
                        if mod_min.is_none() && *circulateur_vitesse_variable {
                            defauts.push(defaut(
                                "echangeur.externe.mod_min",
                                MOD_MIN_DEFAUT,
                                "Th-BCE 2020 §9.12.2, tableau 286 : rapport du débit minimal \
                                 du circulateur primaire au débit nominal par défaut",
                            ));
                        }
                        EchangeurEcs::externe(
                            x.p_ech_ecs_kw,
                            *q_v_prim_nom_m3_h,
                            CirculateurPrimaire {
                                vitesse_variable: *circulateur_vitesse_variable,
                                p_nom_w: *p_circ_prim_nom_w,
                                mod_min: mod_min.unwrap_or(MOD_MIN_DEFAUT),
                            },
                            theta_cons,
                            nb_iter,
                        )
                    }
                };
                a = a.avec_echangeur(ech);
            }
            a.valeurs_par_defaut = defauts;
            stockages.push(a);
        }
        for c in &g.chauffe_eau_thermodynamiques {
            let Some(r) = crate::ecs::idcet::resoudre(
                c,
                |id| ids_et.iter().position(|e| e == id),
                |id| ids_zones.iter().position(|z| z == id),
            ) else {
                continue;
            };
            let mut gnr = GenerateurHoraire::nouveau(
                c.id.clone(),
                Generateur::Thermodynamique(Box::new(r.pac)),
            );
            gnr.source = Some(r.source);
            if !c.essai.auxiliaires_compris {
                gnr.valeurs_par_defaut.push(defaut(
                    "essai.auxiliaires_compris",
                    crate::ecs::idcet::TAUX_SANS_AUXILIAIRES,
                    "Th-BCE 2020 §9.22.3.5.1, éq. (2250) : Taux = 0,02 quand les auxiliaires ne \
                     sont pas compris dans l'essai",
                ));
            }
            generateurs.push(gnr);
            let mut a = AssemblageBallon::nouveau(
                c.id.clone(),
                f64::from(c.nb_assemblages.max(1)),
                r.ballon,
                RegulationGenerateur {
                    generateur: generateurs.len() - 1,
                    programmation: programmation(c.programmation),
                    // Éq. (2262) : hystérésis par défaut ; §9.22.3.2.4 : échangeur et sonde en
                    // zone basse, hauteur relative nulle.
                    delta_theta_k: DELTA_THETA_DEFAUT_K,
                    z_gen: 0,
                    z_reg: 0,
                    h_rel_ech: 0.0,
                },
                None,
                nb_iter_vp(None),
            );
            a.identification = Some(r.identification);
            stockages.push(a);
        }
        stockages
    }
}

/// Chaudière résolue à partir de sa saisie : statuts des rendements (§8.19.3.1) et valeurs par
/// défaut (§8.19.3.1.4), ces dernières signalées.
fn resoudre_chaudiere(
    c: &crate::entree::ChaudiereSaisie,
    rdim: f64,
    en_volume_chauffe: bool,
) -> Option<(Chaudiere, Vec<ValeurParDefaut>)> {
    use crate::chauffage_refroidissement::chaudieres_gaz_fioul_bois::{
        valeurs_par_defaut, AlimentationBois, CombustibleBois, TypeChaudiere,
        TAUX_CHARGE_INTERMEDIAIRE, W_VEILLE_DEFAUT_W,
    };
    use crate::entree::{
        AlimentationBoisSaisie as A, CombustibleBoisSaisi as B, FonctionChaleur,
        PertesParoisChaudiere, StatutRendementChaudiere, TypeChaudiereSaisi as T,
    };
    let type_chaudiere = match c.type_chaudiere {
        T::GazStandard => TypeChaudiere::GazStandard,
        T::GazBasseTemperature => TypeChaudiere::GazBasseTemperature,
        T::GazCondensation => TypeChaudiere::GazCondensation,
        T::FioulStandard => TypeChaudiere::FioulStandard,
        T::FioulCondensation => TypeChaudiere::FioulCondensation,
        T::Bois => TypeChaudiere::Bois(match c.combustible_bois? {
            B::Buches => CombustibleBois::Buches,
            B::Granules => CombustibleBois::Granules,
            B::Plaquettes => CombustibleBois::Plaquettes,
        }),
    };
    let alim = c.alimentation_bois.map(|a| match a {
        A::ManuelTirageNaturel => AlimentationBois::ManuelTirageNaturel,
        A::ManuelAirPulse => AlimentationBois::ManuelAirPulse,
        A::AutomatiqueTirageNaturel => AlimentationBois::AutomatiqueTirageNaturel,
        A::AutomatiqueAirPulse => AlimentationBois::AutomatiqueAirPulse,
    });
    let mut defauts = Vec::new();
    let mut defaut = |champ: &str, valeur: f64, regle: &str| {
        defauts.push(ValeurParDefaut {
            champ: champ.to_owned(),
            valeur,
            regle: regle.to_owned(),
        });
    };
    let pn = c.pn_gen_kw;
    let p_int = c.rendements.and_then(|r| r.p_int_kw).unwrap_or_else(|| {
        let p = TAUX_CHARGE_INTERMEDIAIRE * pn;
        defaut(
            "rendements.p_int_kw",
            p,
            "Th-BCE 2020 §8.19.2 : « Pint » est la puissance utile à 30 % de charge ; \
                 30 % de Pn_gen (lecture Q49)",
        );
        p
    });
    let v = valeurs_par_defaut(type_chaudiere, alim, pn, p_int, c.gpl);
    let rendements = match c.rendements {
        Some(r) => {
            let k = match r.statut {
                StatutRendementChaudiere::Certifie => 1.0,
                StatutRendementChaudiere::Justifie => 0.9,
                StatutRendementChaudiere::Declare => 0.8,
            };
            (k * r.r_pn_pct, k * r.r_pint_pct)
        }
        None => {
            defaut(
                "rendements.r_pn_pct",
                v.r_pn_pct,
                "Th-BCE 2020 §8.19.3.1.4, éq. (1072) : rendement sur PCS à pleine charge par \
                 défaut",
            );
            defaut(
                "rendements.r_pint_pct",
                v.r_pint_pct,
                "Th-BCE 2020 §8.19.3.1.4, éq. (1073) : rendement sur PCS à 30 % de charge par \
                 défaut",
            );
            (v.r_pn_pct, v.r_pint_pct)
        }
    };
    let mut aux = |valeur: Option<f64>, champ: &str, defaut_v: f64, regle: &str| {
        valeur.unwrap_or_else(|| {
            defaut(champ, defaut_v, regle);
            defaut_v
        })
    };
    let w_nom = aux(
        c.auxiliaires.w_aux_nom_w,
        "auxiliaires.w_aux_nom_w",
        v.w_aux_nom_w,
        "Th-BCE 2020 §8.19.3.1.4, éq. (1076), tableau 119",
    );
    let w_int = aux(
        c.auxiliaires.w_aux_int_w,
        "auxiliaires.w_aux_int_w",
        v.w_aux_int_w,
        "Th-BCE 2020 §8.19.3.1.4, éq. (1077), tableau 119",
    );
    let w_veille = aux(
        c.auxiliaires.w_veille_w,
        "auxiliaires.w_veille_w",
        W_VEILLE_DEFAUT_W,
        "Th-BCE 2020 §8.19.3.1.4, éq. (1075)",
    );
    let q_po_30 = aux(
        c.q_po_30_w,
        "q_po_30_w",
        v.q_po_30_w,
        "Th-BCE 2020 §8.19.3.1.4, éq. (1074), tableau 118",
    );
    let p_qp_arret = match c.pertes_parois {
        PertesParoisChaudiere::SansVentilateur => 0.50,
        PertesParoisChaudiere::AvecVentilateur => 0.75,
        PertesParoisChaudiere::AvecClapet => 1.0,
    };
    let (chauffage, ecs) = match c.fonction {
        FonctionChaleur::Chauffage => (true, false),
        FonctionChaleur::Ecs => (false, true),
        FonctionChaleur::ChauffageEcs => (true, true),
    };
    Some((
        Chaudiere::nouvelle(
            type_chaudiere,
            c.gpl,
            chauffage,
            ecs,
            rdim,
            en_volume_chauffe,
            pn,
            p_int,
            rendements,
            (w_nom, w_int, w_veille),
            q_po_30,
            p_qp_arret,
        ),
        defauts,
    ))
}

/// Sous-station résolue à partir de sa saisie (tableaux 247 et 248).
fn resoudre_reseau(
    r: &crate::entree::ReseauChaleurFroidSaisi,
    rdim: f64,
    en_volume_chauffe: bool,
) -> Option<ReseauChaleurFroid> {
    use crate::chauffage_refroidissement::reseau_de_chaleur_et_de_froid::{
        LectureHss, TypeReseauChaleur,
    };
    use crate::entree::{FonctionChaleur, ReseauChaleurFroidSaisi as R, TypeReseauChaleurSaisi};
    Some(match r {
        R::Chaleur {
            fonction,
            type_reseau,
            classe_isolation_secondaire,
            p_ess_kw,
            ..
        } => {
            let t = match type_reseau {
                TypeReseauChaleurSaisi::EauChaudeBasseTemperature => {
                    TypeReseauChaleur::EauChaudeBasseTemperature
                }
                TypeReseauChaleurSaisi::EauChaudeHauteTemperature => {
                    TypeReseauChaleur::EauChaudeHauteTemperature
                }
                TypeReseauChaleurSaisi::VapeurBassePression => {
                    TypeReseauChaleur::VapeurBassePression
                }
                TypeReseauChaleurSaisi::VapeurHautePression => {
                    TypeReseauChaleur::VapeurHautePression
                }
            };
            let (theta_prs_c, d_ss) = t.theta_prs_et_dss();
            ReseauChaleurFroid {
                froid: false,
                chauffage: *fonction != FonctionChaleur::Ecs,
                ecs: *fonction != FonctionChaleur::Chauffage,
                rdim,
                en_volume_chauffe,
                p_ess_kw: *p_ess_kw,
                b_ss: t.b_ss(*classe_isolation_secondaire),
                d_ss,
                theta_prs_c,
                lecture_hss: LectureHss::Litterale,
            }
        }
        R::Froid { p_ess_kw, .. } => ReseauChaleurFroid {
            froid: true,
            chauffage: false,
            ecs: false,
            rdim,
            en_volume_chauffe,
            p_ess_kw: *p_ess_kw,
            b_ss: 0.0,
            d_ss: 0.0,
            theta_prs_c: 0.0,
            lecture_hss: LectureHss::Litterale,
        },
    })
}

/// Autre générateur à combustion résolu à partir de sa saisie (§8.20.3.1 à 8.20.3.4, tableaux
/// 124 à 131) ; `avec_priorite` : la génération est gérée en cascade ou en alternance (seul
/// cas où « idraccord_gnr » intervient, lecture Q156).
fn resoudre_autre_combustion(
    c: &crate::entree::AutreGenerateurCombustionSaisi,
    rdim: f64,
    en_volume_chauffe: bool,
    avec_priorite: bool,
) -> (AutreGenerateurCombustion, Vec<ValeurParDefaut>) {
    use crate::chauffage_refroidissement::autres_generateurs_a_combustion::{
        p_aux_defaut_w, q_po_30_defaut_w, r_pint_defaut_pct, r_pn_defaut_pct, CoefficientsDefaut,
        Famille, DIVISEUR_VEILLEUSE, PN_MAX_FORMULES_AIR_CHAUD_KW, R_PINT_UTILE_MAX_PCT,
        R_PN_UTILE_MAX_PCT, THETA_FONCT_MIN_DEFAUT_C,
    };
    use crate::entree::{
        EvacuationFumees, PertesParoisChaudiere, RaccordementGenerateur,
        StatutRendementCombustion as S, TypeAutreGenerateurCombustion as T,
    };
    let pn = c.pn_gen_kw;
    // Tableaux 126 à 128 ; tableaux 129 et 130 pour « fmaj ».
    let (famille, coef, f_maj) = match c.type_generateur {
        T::RadiateurGaz {
            ventilateur_combustion,
            ventilateur_emission,
            evacuation_fumees,
        } => {
            let a = if pn < 5.0 {
                80.0
            } else if ventilateur_combustion {
                84.0
            } else {
                82.0
            };
            let nb = f64::from(u8::from(ventilateur_combustion) + u8::from(ventilateur_emission));
            let f_maj = if ventilateur_combustion {
                1.02
            } else if evacuation_fumees == Some(EvacuationFumees::MicroVentouse) {
                1.04
            } else {
                1.06
            };
            (
                Famille::Radiateur,
                CoefficientsDefaut {
                    ab: (a, 0.0),
                    cd: None,
                    ef: None,
                    gh: Some((40.0 * nb, 0.0)),
                },
                f_maj,
            )
        }
        T::ChauffeEauGaz => (
            Famille::ChauffeEau,
            CoefficientsDefaut {
                ab: (if pn < 10.0 { 82.0 } else { 84.0 }, 0.0),
                cd: None,
                ef: Some((1.5, 0.0)),
                gh: Some((0.0, 0.0)),
            },
            1.0,
        ),
        T::AccumulateurGaz {
            condensation,
            plus_de_200_l_montee_rapide,
        } => (
            Famille::Accumulateur,
            CoefficientsDefaut {
                ab: (if condensation { 98.0 } else { 84.0 }, 0.0),
                cd: None,
                ef: Some((
                    if plus_de_200_l_montee_rapide {
                        1.7
                    } else {
                        1.5
                    },
                    0.0,
                )),
                gh: Some((0.0, 0.0)),
            },
            1.0,
        ),
        T::AirChaud {
            condensation,
            ventilateur_emission,
        } => (
            Famille::AirChaud,
            CoefficientsDefaut {
                ab: (if condensation { 90.0 } else { 84.0 }, 0.0),
                cd: Some((if condensation { 83.0 } else { 77.0 }, 0.0)),
                ef: Some((1.75, -0.55)),
                gh: Some((0.0, if ventilateur_emission { 54.0 } else { 4.0 })),
            },
            1.0,
        ),
        T::TubeRadiant {
            ventilateur_emission,
        } => (
            Famille::TubeRadiant,
            CoefficientsDefaut {
                ab: (85.0, 0.0),
                cd: None,
                ef: None,
                gh: ventilateur_emission.then_some((0.0, 54.0)),
            },
            1.06,
        ),
        T::PanneauRadiant => (
            Famille::PanneauRadiant,
            CoefficientsDefaut {
                ab: (90.0, 0.0),
                cd: None,
                ef: None,
                gh: None,
            },
            1.0,
        ),
    };
    // §8.20.3.4 : formules d'un générateur d'air chaud évaluées à 300 kW au plus.
    let pn_f = if famille == Famille::AirChaud {
        pn.min(PN_MAX_FORMULES_AIR_CHAUD_KW)
    } else {
        pn
    };
    let mut defauts = Vec::new();
    let mut defaut = |champ: &str, valeur: f64, regle: &str| {
        defauts.push(ValeurParDefaut {
            champ: champ.to_owned(),
            valeur,
            regle: regle.to_owned(),
        });
    };
    let air_chaud = famille == Famille::AirChaud;
    let p_int = if air_chaud {
        c.p_int_kw.unwrap_or_else(|| {
            let p = 0.3 * pn;
            defaut(
                "p_int_kw",
                p,
                "Th-BCE 2020 §8.20.2, tableau 123 : « Rpint » est défini à 30 % de charge ; \
                 30 % de Pngen (lecture Q155)",
            );
            p
        })
    } else {
        0.0
    };
    let r_pn_def = r_pn_defaut_pct(&coef, pn_f);
    let r_pint_def = r_pint_defaut_pct(&coef, pn_f).unwrap_or(0.0);
    let (r_pn, r_pint) = match c.rendements {
        Some(r) => {
            // Éq. (1129), (1130) et (1132) ; valeur justifiée diminuée de 10 %.
            let pn_eff = match r.statut {
                S::Certifie => r.r_pn_pct,
                S::Justifie => 0.9 * r.r_pn_pct,
                S::Declare => (0.8 * r.r_pn_pct).min(R_PN_UTILE_MAX_PCT),
            };
            let pint_eff = match (r.r_pint_pct, r.statut) {
                (Some(x), S::Certifie) => x,
                (Some(x), S::Justifie) => 0.9 * x,
                (Some(x), S::Declare) => (0.8 * x).min(R_PINT_UTILE_MAX_PCT),
                (None, _) => {
                    if air_chaud {
                        defaut(
                            "rendements.r_pint_pct",
                            r_pint_def,
                            "Th-BCE 2020 §8.20.3.4, éq. (1122), tableau 126",
                        );
                    }
                    r_pint_def
                }
            };
            (pn_eff, pint_eff)
        }
        None => {
            defaut(
                "rendements.r_pn_pct",
                r_pn_def,
                "Th-BCE 2020 §8.20.3.4, éq. (1121), tableau 126",
            );
            if air_chaud {
                defaut(
                    "rendements.r_pint_pct",
                    r_pint_def,
                    "Th-BCE 2020 §8.20.3.4, éq. (1122), tableau 126",
                );
            }
            (r_pn_def, r_pint_def)
        }
    };
    // La validation exige `w_aux_nom_w` quand le tableau 128 ne donne pas de valeur.
    let w_aux_nom = c.w_aux_nom_w.unwrap_or_else(|| {
        let w = p_aux_defaut_w(&coef, pn_f).unwrap_or(0.0);
        defaut(
            "w_aux_nom_w",
            w,
            "Th-BCE 2020 §8.20.3.4, éq. (1124), tableau 128",
        );
        w
    });
    let w_veille = c.w_veille_w + c.p_veilleuse_w.unwrap_or(0.0) / DIVISEUR_VEILLEUSE;
    let q_po_30 = if famille.pertes_a_charge_nulle() {
        c.q_po_30_w.unwrap_or_else(|| {
            let q = q_po_30_defaut_w(&coef, pn_f).unwrap_or(0.0);
            defaut(
                "q_po_30_w",
                q,
                "Th-BCE 2020 §8.20.3.4, éq. (1123), tableau 127 (lecture Q155)",
            );
            q
        })
    } else {
        0.0
    };
    let theta_min = if famille.ecs() {
        c.theta_fonct_min_c.unwrap_or_else(|| {
            defaut(
                "theta_fonct_min_c",
                THETA_FONCT_MIN_DEFAUT_C,
                "Th-BCE 2020 §8.20.3.2.2",
            );
            THETA_FONCT_MIN_DEFAUT_C
        })
    } else {
        THETA_FONCT_MIN_DEFAUT_C
    };
    let p_qp_arret = match c.pertes_parois {
        PertesParoisChaudiere::SansVentilateur => 0.50,
        PertesParoisChaudiere::AvecVentilateur => 0.75,
        PertesParoisChaudiere::AvecClapet => 1.0,
    };
    // Lecture Q156 : « idraccord_gnr » ne vaut que pour une gestion avec priorité ; avec
    // isolement par défaut (tableau 123).
    let permanent = avec_priorite && c.raccordement == Some(RaccordementGenerateur::Permanent);
    (
        AutreGenerateurCombustion::nouveau(
            famille,
            c.gpl,
            rdim,
            en_volume_chauffe,
            (pn, p_int),
            (r_pn, r_pint),
            (w_aux_nom, w_veille),
            q_po_30,
            theta_min,
            f_maj,
            p_qp_arret,
            permanent,
        ),
        defauts,
    )
}

/// Cogénérateur résolu à partir de sa saisie (§8.21.3.1, tableaux 136 à 140).
fn resoudre_cogeneration(
    c: &crate::entree::CogenerationSaisie,
    rdim: f64,
    en_volume_chauffe: bool,
) -> (Cogenerateur, Vec<ValeurParDefaut>) {
    use crate::chauffage_refroidissement::systeme_de_cogeneration::{
        p_w_aux_utile_max_kw, phi_th_stab_30_defaut_w, valeurs_utiles_max_avec_appoint,
        valeurs_utiles_max_sans_appoint, PointPleineCharge, TechnologieCogeneration as Tc,
        PART_P_TH_MIN_DEFAUT, P_W_AUX_VEILLE_DEFAUT_W,
    };
    use crate::entree::{
        CombustibleCogeneration as C, FonctionChaleur, PerformancesCogeneration,
        RaccordementGenerateur, StatutPerformancesCogeneration as S, TypeCogenerateur as T,
    };
    // Tableau 136.
    let (energie, pcsi) = match c.combustible {
        C::GazNaturel => (Energie::Gaz, 1.11),
        C::Gpl => (Energie::Gaz, 1.09),
        C::Fioul => (Energie::Fioul, 1.07),
        C::BoisBuches | C::BoisGranules | C::BoisPlaquettes => (Energie::Bois, 1.08),
    };
    // Tableau 137.
    let (alpha_th, theta0, theta100) = if c.condensation {
        (0.2, 35.0, 50.0)
    } else {
        (0.04, 53.5, 70.0)
    };
    let techno = match c.type_cogenerateur {
        T::MoteurCombustionInterne => Tc::MoteurCombustionInterne,
        T::Stirling => Tc::Stirling,
        T::MicroTurbine => Tc::MicroTurbine,
    };
    let mut defauts = Vec::new();
    let mut defaut = |champ: &str, valeur: f64, regle: &str| {
        defauts.push(ValeurParDefaut {
            champ: champ.to_owned(),
            valeur,
            regle: regle.to_owned(),
        });
    };
    let p0 = c.module_seul.p_th_kw;
    // Tableau 138 (micro-cogénérateur standard ; repris pour la condensation, lecture Q157).
    let (th0_max, el0_max) = valeurs_utiles_max_sans_appoint(techno);
    // §8.21.3.1.3 : statut des données.
    let mut point = |nom: &str,
                     p: &PerformancesCogeneration,
                     max: (f64, f64, f64),
                     theta_ref: f64|
     -> PointPleineCharge {
        let (th_max, el_max, aux_max_kw) = max;
        let (th, el, aux_kw) = match p.statut {
            S::Defaut => {
                let v = (0.8 * th_max, 0.8 * el_max, 1.2 * aux_max_kw);
                let r = "Th-BCE 2020 §8.21.3.1.3 : statut « par défaut », 0,8 × valeur utile \
                         maximale des tableaux 138 et 139 (1,2 × pour les auxiliaires)";
                defaut(&format!("{nom}.eta_th_pct"), v.0, r);
                defaut(&format!("{nom}.eta_el_pct"), v.1, r);
                defaut(&format!("{nom}.p_w_aux_kw"), v.2, r);
                v
            }
            S::Declaree => (
                (0.8 * p.eta_th_pct.unwrap_or(0.0)).min(th_max),
                (0.8 * p.eta_el_pct.unwrap_or(0.0)).min(el_max),
                (1.2 * p.p_w_aux_kw.unwrap_or(0.0)).max(aux_max_kw),
            ),
            S::Justifiee => (
                0.9 * p.eta_th_pct.unwrap_or(0.0),
                0.9 * p.eta_el_pct.unwrap_or(0.0),
                1.1 * p.p_w_aux_kw.unwrap_or(0.0),
            ),
            S::Certifiee => (
                p.eta_th_pct.unwrap_or(0.0),
                p.eta_el_pct.unwrap_or(0.0),
                p.p_w_aux_kw.unwrap_or(0.0),
            ),
        };
        PointPleineCharge {
            p_th_kw: p.p_th_kw,
            eta_th_pct: th,
            eta_el_pct: el,
            p_w_aux_w: 1000.0 * aux_kw,
            theta_ref_c: theta_ref,
        }
    };
    let module_seul = point(
        "module_seul",
        &c.module_seul,
        (th0_max, el0_max, p_w_aux_utile_max_kw(p0)),
        theta0,
    );
    let avec_appoint = c.avec_appoint.as_ref().map(|a| {
        // Éq. (1175) à (1178).
        let (th, el) = valeurs_utiles_max_avec_appoint(th0_max, el0_max, p0, a.p_th_kw);
        point(
            "avec_appoint",
            a,
            (th, el, p_w_aux_utile_max_kw(a.p_th_kw)),
            theta100,
        )
    });
    let p_min = c.p_th_min_kw.unwrap_or_else(|| {
        let v = PART_P_TH_MIN_DEFAUT * p0;
        defaut(
            "p_th_min_kw",
            v,
            "Th-BCE 2020 §8.21.3.1.3, tableau 140 : 60 % de Pth;chp100+sup0",
        );
        v
    });
    let phi30 = c.phi_th_stab_30_w.unwrap_or_else(|| {
        let p = c.avec_appoint.as_ref().map_or(p0, |a| a.p_th_kw);
        let v = phi_th_stab_30_defaut_w(p);
        defaut(
            "phi_th_stab_30_w",
            v,
            "Th-BCE 2020 §8.21.3.1.3, tableau 140 : 1000 × 4,25 % × Pth / Pth^0,4",
        );
        v
    });
    let veille = c.p_w_aux_veille_w.unwrap_or_else(|| {
        defaut(
            "p_w_aux_veille_w",
            P_W_AUX_VEILLE_DEFAUT_W,
            "Th-BCE 2020 §8.21.3.1.3, tableau 140",
        );
        P_W_AUX_VEILLE_DEFAUT_W
    });
    let postes = match c.fonction {
        FonctionChaleur::Chauffage => (true, false),
        FonctionChaleur::Ecs => (false, true),
        FonctionChaleur::ChauffageEcs => (true, true),
    };
    (
        Cogenerateur::nouveau(
            postes,
            energie,
            pcsi,
            rdim,
            en_volume_chauffe,
            c.raccordement == RaccordementGenerateur::Permanent,
            (alpha_th, 0.0),
            module_seul,
            avec_appoint,
            (p_min, phi30, veille),
        ),
        defauts,
    )
}
