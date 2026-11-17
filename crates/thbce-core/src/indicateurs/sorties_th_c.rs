// SPDX-License-Identifier: GPL-3.0-or-later
//! Fiche `C_CALC_Sorties du mode Th-C` (Th-BCE 2020 §13.2) : quantités d'énergie importées par
//! poste et par énergie, Cep et Cep,nr du bâtiment ; Ic énergie (annexe II §4.3.2, éq. 74) ;
//! exigences et conformité (annexe de l'article R. 172-4 du CCH).
//!
//! Les indicateurs sont exprimés par bâtiment (§13.2.1).

use thbce_data::energies::{
    coef_ep, coef_ep_nr, facteur_emission_kg_kwh, CombustibleBois, Energie as Vecteur,
    UsageElectricite,
};
use thbce_data::ponderation_dynamique::{f_co2, ANNEE_MAX};
use thbce_data::reseaux_chaleur::{autres_reseaux, reseau};
use thbce_data::ZoneClimatique;

use crate::entree::{
    Batiment, CombustibleBoisSaisi, NatureGenerateur, ReseauChaleurFroidSaisi, UsageZone,
};
use crate::resultats::{
    ConsommationsParEnergie, ConsommationsPostes, ResultatsGeneration, ResultatsZone,
};

use super::bilans_energies::{poste, BilanElectrique, NB_POSTES};
use super::exigences::{dh_max, exigences_partie, PartieBatiment};
use super::forfait_refroidissement::{
    cef_fr_elec_kwh_m2, coef_kwh_fr_par_dh, coef_zone_clim_et_alt,
};
use super::resultats::{
    BilanProductionPv, ExigencesZone, ForfaitRefroidissementGroupe, LigneBilanEnergie,
    PostesElectricite, ResultatsExigences, ResultatsIndicateurs, VerificationExigence,
};

/// `Σ fCO2(a)` pour `a` de 1 à la période d'étude de référence (50 ans) : annexe II §4.3.2,
/// éq. (74) ; article 11 de l'arrêté.
pub fn somme_f_co2() -> f64 {
    (1..=ANNEE_MAX).filter_map(f_co2).sum()
}

/// Entrées du calcul des indicateurs d'un bâtiment.
#[derive(Debug, Clone, Copy)]
pub struct EntreesIndicateurs<'a> {
    /// Description du bâtiment.
    pub batiment: &'a Batiment,
    /// Zone climatique du projet.
    pub zone_climatique: ZoneClimatique,
    /// « Alt » : altitude du projet, en m.
    pub alt_m: f64,
    /// Résultats des zones (non arrondis) : surfaces de référence, DH des groupes.
    pub zones: &'a [ResultatsZone],
    /// « SREF_bat », en m².
    pub sref_m2: f64,
    /// « Bbio » du bâtiment, en points.
    pub bbio_pts: Option<f64>,
    /// Bilan horaire de l'électricité (§13.4).
    pub bilan: &'a BilanElectrique,
    /// Consommations annuelles des générations attribuées au bâtiment, en kWh.
    pub consommations: &'a ConsommationsPostes,
    /// Générations du bâtiment (consommations de chaque générateur), dans l'ordre de l'entrée.
    pub generations: &'a [ResultatsGeneration],
}

/// Sélection d'un poste dans les consommations des générations.
type ChoixPoste = fn(&ConsommationsPostes) -> &ConsommationsParEnergie;

/// Vecteur énergétique d'une part de consommation : libellé, coefficients et facteur.
#[derive(Debug, Clone, PartialEq)]
struct VecteurConnu {
    libelle: String,
    vecteur: Vecteur,
    /// Faux si le facteur d'émission n'est pas connu (combustible d'un poêle non saisi).
    facteur_connu: bool,
}

impl VecteurConnu {
    fn nouveau(libelle: &str, vecteur: Vecteur) -> Self {
        Self {
            libelle: libelle.to_owned(),
            vecteur,
            facteur_connu: true,
        }
    }
}

fn bois(c: Option<CombustibleBoisSaisi>) -> VecteurConnu {
    match c {
        Some(CombustibleBoisSaisi::Buches) => {
            VecteurConnu::nouveau("bois_buches", Vecteur::Bois(CombustibleBois::Buches))
        }
        Some(CombustibleBoisSaisi::Granules) => {
            VecteurConnu::nouveau("bois_granules", Vecteur::Bois(CombustibleBois::Granules))
        }
        Some(CombustibleBoisSaisi::Plaquettes) => VecteurConnu::nouveau(
            "bois_plaquettes",
            Vecteur::Bois(CombustibleBois::Plaquettes),
        ),
        // Coefficients d'énergie primaire du bois, indépendants du combustible ; facteur
        // d'émission inconnu.
        None => VecteurConnu {
            libelle: "bois".to_owned(),
            vecteur: Vecteur::Bois(CombustibleBois::Buches),
            facteur_connu: false,
        },
    }
}

fn reseau_chaleur(id: Option<&str>) -> VecteurConnu {
    let d = id.and_then(reseau).unwrap_or_else(|| autres_reseaux(false));
    VecteurConnu::nouveau(
        &format!("reseau_chaleur:{}", id.unwrap_or("autres")),
        Vecteur::ReseauChaleur {
            facteur_emission_kg_kwh: d.facteur_emission_kg_kwh,
            rat_enr: d.taux_enrr_pct.unwrap_or(0.0) / 100.0,
        },
    )
}

fn reseau_froid(id: Option<&str>) -> VecteurConnu {
    let d = id.and_then(reseau).unwrap_or_else(|| autres_reseaux(true));
    VecteurConnu::nouveau(
        &format!("reseau_froid:{}", id.unwrap_or("autres")),
        Vecteur::ReseauFroid {
            facteur_emission_kg_kwh: d.facteur_emission_kg_kwh,
        },
    )
}

/// Colonnes non électriques de la matrice des consommations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Colonne {
    Gaz,
    Fioul,
    Charbon,
    Bois,
    Reseau,
}

impl Colonne {
    const TOUTES: [Self; 5] = [
        Self::Gaz,
        Self::Fioul,
        Self::Charbon,
        Self::Bois,
        Self::Reseau,
    ];

    fn valeur(self, c: &ConsommationsParEnergie) -> f64 {
        match self {
            Self::Gaz => c.gaz_kwh,
            Self::Fioul => c.fioul_kwh,
            Self::Charbon => c.charbon_kwh,
            Self::Bois => c.bois_kwh,
            Self::Reseau => c.reseau_kwh,
        }
    }

    /// Vecteur d'une colonne quand aucun générateur ne le précise.
    fn par_defaut(self, froid: bool) -> VecteurConnu {
        match self {
            Self::Gaz => VecteurConnu::nouveau("gaz_naturel", Vecteur::GazNaturel),
            Self::Fioul => VecteurConnu::nouveau("fioul", Vecteur::Fioul),
            Self::Charbon => VecteurConnu::nouveau("charbon", Vecteur::Charbon),
            Self::Bois => bois(None),
            Self::Reseau if froid => reseau_froid(None),
            Self::Reseau => reseau_chaleur(None),
        }
    }
}

/// Vecteur de la colonne `col` consommée par un générateur, d'après sa description (§13.2.2 :
/// « Type_Combustible_Bois », « O_Type_Reseau », « O_RatENR_rdch ») ; le GPL est distingué du
/// gaz naturel (article 10).
fn vecteur_generateur(nature: &NatureGenerateur, col: Colonne, froid: bool) -> VecteurConnu {
    match (nature, col) {
        (NatureGenerateur::Chaudiere(c), Colonne::Gaz) if c.gpl => {
            VecteurConnu::nouveau("gpl", Vecteur::Gpl)
        }
        (NatureGenerateur::Chaudiere(c), Colonne::Bois) => bois(c.combustible_bois),
        (NatureGenerateur::PoeleOuInsert(p), Colonne::Bois) => bois(Some(p.combustible_bois)),
        (
            NatureGenerateur::ReseauDeChaleurOuDeFroid(ReseauChaleurFroidSaisi::Chaleur {
                identifiant_reseau,
                ..
            }),
            Colonne::Reseau,
        ) => reseau_chaleur(identifiant_reseau.as_deref()),
        (
            NatureGenerateur::ReseauDeChaleurOuDeFroid(ReseauChaleurFroidSaisi::Froid {
                identifiant_reseau,
                ..
            }),
            Colonne::Reseau,
        ) => reseau_froid(identifiant_reseau.as_deref()),
        _ => col.par_defaut(froid),
    }
}

/// Répartit la consommation `total_kwh` d'un poste dans une colonne entre les vecteurs des
/// générateurs qui la consomment, au prorata de leurs consommations (« quantités d'énergie
/// importées sous-décomposées », §13.2.3.4.2 et §13.2.3.5.2, éq. 2444 et 2476).
fn repartir(
    bat: &Batiment,
    generations: &[ResultatsGeneration],
    choisir: ChoixPoste,
    col: Colonne,
    froid: bool,
    total_kwh: f64,
) -> Vec<(VecteurConnu, f64)> {
    let mut parts: Vec<(VecteurConnu, f64)> = Vec::new();
    for rg in generations {
        let Some(g) = bat.generations.iter().find(|g| g.id == rg.id) else {
            continue;
        };
        for r in &rg.generateurs {
            let q = col.valeur(choisir(&r.consommations));
            if q <= 0.0 {
                continue;
            }
            let v = g.generateurs.iter().find(|x| x.id == r.id).map_or_else(
                || col.par_defaut(froid),
                |x| vecteur_generateur(&x.nature, col, froid),
            );
            match parts.iter_mut().find(|(w, _)| *w == v) {
                Some((_, x)) => *x += q,
                None => parts.push((v, q)),
            }
        }
    }
    let somme: f64 = parts.iter().map(|(_, q)| q).sum();
    if somme <= 0.0 {
        return vec![(col.par_defaut(froid), total_kwh)];
    }
    parts
        .into_iter()
        .map(|(v, q)| (v, total_kwh * q / somme))
        .collect()
}

/// Bâtiment raccordé à un réseau de chaleur urbain : une de ses générations comprend une
/// sous-station de réseau de chaleur.
pub fn raccorde_rcu(bat: &Batiment) -> bool {
    bat.generations
        .iter()
        .flat_map(|g| &g.generateurs)
        .any(|x| {
            matches!(
                x.nature,
                NatureGenerateur::ReseauDeChaleurOuDeFroid(ReseauChaleurFroidSaisi::Chaleur { .. })
            )
        })
}

/// Calcule les indicateurs d'un bâtiment. Renvoie aussi les avertissements du calcul.
pub fn calculer_indicateurs(e: &EntreesIndicateurs<'_>) -> (ResultatsIndicateurs, Vec<String>) {
    let bat = e.batiment;
    let sref = e.sref_m2;
    let par_m2 = |x: f64| if sref > 0.0 { x / sref } else { 0.0 };
    let mut avertissements = Vec::new();
    let params = bat.exigences;

    // Exigences des zones (nécessaires au seuil haut du forfait de refroidissement).
    let rcu = raccorde_rcu(bat);
    let sref_usage = |u: UsageZone| -> f64 {
        bat.zones
            .iter()
            .zip(e.zones)
            .filter(|(z, _)| z.usage == u)
            .map(|(_, r)| r.sref_m2)
            .sum()
    };
    let parties: Vec<(PartieBatiment, Option<super::exigences::ExigencesPartie>)> = bat
        .zones
        .iter()
        .zip(e.zones)
        .map(|(z, r)| {
            let p = PartieBatiment {
                usage: z.usage.numero(),
                sref_m2: r.sref_m2,
                nb_lgt: z.nb_lgt.unwrap_or(1),
                sref_usage_m2: sref_usage(z.usage),
                raccorde_rcu: rcu,
            };
            let ex = params
                .as_ref()
                .and_then(|pa| exigences_partie(&p, pa, e.zone_climatique, e.alt_m));
            (p, ex)
        })
        .collect();

    // §13.3 : forfait de refroidissement des groupes non climatisés.
    let coef_zone = coef_zone_clim_et_alt(e.zone_climatique, e.alt_m);
    let mut forfaits = Vec::new();
    let mut seuils_inconnus = false;
    for ((z, r), (p, ex)) in bat.zones.iter().zip(e.zones).zip(&parties) {
        let Some(coef) = coef_kwh_fr_par_dh(p.usage) else {
            continue;
        };
        for (g, rg) in z.groupes.iter().zip(&r.groupes) {
            let Some(dh) = rg.confort_ete.as_ref().map(|c| c.dh_c_h) else {
                continue;
            };
            let climatise = g.iclim == Some(true);
            let seuil = params.as_ref().and_then(|pa| {
                dh_max(
                    p.usage,
                    pa.categorie_contraintes_exterieures,
                    e.zone_climatique,
                    climatise,
                    ex.map_or(p.sref_m2, |x| x.smoy_lgt_m2),
                )
            });
            let cef_m2 = cef_fr_elec_kwh_m2(climatise, dh, seuil, coef, coef_zone);
            if cef_m2 > 0.0 {
                seuils_inconnus |= seuil.is_none();
                forfaits.push(ForfaitRefroidissementGroupe {
                    zone: z.id.clone(),
                    groupe: g.id.clone(),
                    dh_c_h: dh,
                    seuil_haut_c_h: seuil,
                    cef_kwh_m2: cef_m2,
                    // Éq. (2495) et (2496) : pondération par la surface du groupe.
                    cef_kwh: cef_m2 * rg.sref_m2,
                });
            }
        }
    }
    if seuils_inconnus {
        avertissements.push(format!(
            "bâtiment {} : forfait de refroidissement calculé sans seuil haut (catégorie de \
             contraintes extérieures non saisie dans `exigences`) : le DH n'est pas plafonné",
            bat.id
        ));
    }
    let forfait_kwh: f64 = forfaits.iter().map(|f| f.cef_kwh).sum();

    // Électricité consommée et importée par poste ; le forfait, annuel, s'ajoute aux deux
    // (§13.3.3.2, éq. 2494 à 2496).
    let b = e.bilan;
    let mut consommee = PostesElectricite::depuis_wh(&b.cons_wh);
    let mut importee = PostesElectricite::depuis_wh(&b.imp_wh);
    consommee.refroidissement_kwh += forfait_kwh;
    importee.refroidissement_kwh += forfait_kwh;
    let imp_mensuel: Vec<f64> = b
        .imp_mensuel_wh
        .iter()
        .map(|m| m[..poste::MOB].iter().sum::<f64>() / 1000.0)
        .collect();
    debug_assert_eq!(NB_POSTES, poste::MOB + 1);

    // Lignes du bilan : électricité par poste (usage de l'article 10), puis combustibles et
    // réseaux des postes chauffage, refroidissement et ECS.
    let mut parts: Vec<(&str, VecteurConnu, f64)> = Vec::new();
    let elec = |u: UsageElectricite| VecteurConnu::nouveau("electricite", Vecteur::Electricite(u));
    parts.push((
        "chauffage",
        elec(UsageElectricite::Chauffage),
        importee.chauffage_kwh,
    ));
    parts.push((
        "refroidissement",
        elec(UsageElectricite::Refroidissement),
        importee.refroidissement_kwh,
    ));
    parts.push(("ecs", elec(UsageElectricite::Ecs), importee.ecs_kwh));
    // Usages d'habitation seulement dans cette version : éclairage « habitation ».
    parts.push((
        "eclairage",
        elec(UsageElectricite::EclairageHabitation),
        importee.eclairage_kwh,
    ));
    parts.push((
        "auxiliaires_ventilation",
        elec(UsageElectricite::AutresUsages),
        importee.auxiliaires_ventilation_kwh,
    ));
    parts.push((
        "auxiliaires_distribution",
        elec(UsageElectricite::AutresUsages),
        importee.auxiliaires_distribution_kwh,
    ));
    parts.push((
        "deplacements",
        elec(UsageElectricite::AutresUsages),
        importee.deplacements_kwh,
    ));
    let postes: [(&str, ChoixPoste, bool); 3] = [
        ("chauffage", |c| &c.chauffage, false),
        ("refroidissement", |c| &c.refroidissement, true),
        ("ecs", |c| &c.ecs, false),
    ];
    for (nom, choisir, froid) in postes {
        for col in Colonne::TOUTES {
            let total = col.valeur(choisir(e.consommations));
            if total <= 0.0 {
                continue;
            }
            for (v, q) in repartir(bat, e.generations, choisir, col, froid, total) {
                parts.push((nom, v, q));
            }
        }
    }

    let somme = somme_f_co2();
    let mut lignes = Vec::new();
    let (mut cef, mut cep, mut cep_nr, mut ic) = (0.0, 0.0, 0.0, Some(0.0));
    let mut facteur_manquant = false;
    for (nom, v, q) in parts {
        if q <= 0.0 {
            continue;
        }
        let ce = coef_ep(v.vecteur);
        let cnr = coef_ep_nr(v.vecteur);
        let f = v.facteur_connu.then(|| facteur_emission_kg_kwh(v.vecteur));
        let l_ic = f.map(|f| par_m2(q * f * somme));
        cef += q;
        cep += q * ce;
        cep_nr += q * cnr;
        ic = match (ic, l_ic) {
            (Some(a), Some(x)) => Some(a + x),
            _ => {
                facteur_manquant = true;
                None
            }
        };
        lignes.push(LigneBilanEnergie {
            poste: nom.to_owned(),
            energie: v.libelle,
            cef_kwh: q,
            coef_ep: ce,
            coef_ep_nr: cnr,
            facteur_emission_kg_kwh: f,
            cep_kwhep_m2: par_m2(q * ce),
            cep_nr_kwhep_m2: par_m2(q * cnr),
            ic_energie_kgco2_m2: l_ic,
        });
    }
    if facteur_manquant {
        avertissements.push(format!(
            "bâtiment {} : Ic énergie non calculé, le combustible d'un générateur au bois \
             (`combustible_bois`) n'est pas saisi",
            bat.id
        ));
    }
    let cep_m2 = par_m2(cep);
    let cep_nr_m2 = par_m2(cep_nr);

    let production_pv = (b.prod_pv_wh > 0.0).then(|| BilanProductionPv {
        e_prod_pv_kwh: b.prod_pv_wh / 1000.0,
        e_prod_pv_ac_kwh: b.prod_pv_ac_wh / 1000.0,
        e_exportee_kwh: b.exportee_wh() / 1000.0,
        tac_pct: b.tac_pct().unwrap_or(0.0),
        e_prod_pv_mensuel_kwh: b.prod_pv_mensuel_wh.iter().map(|x| x / 1000.0).collect(),
        e_prod_pv_ac_mensuel_kwh: b.prod_pv_ac_mensuel_wh.iter().map(|x| x / 1000.0).collect(),
    });
    // Chauffe-eau solaires individuels et systèmes combinés, puis productions solaires
    // collectives (CESCI, CESCAI).
    let solaire: f64 = e
        .generations
        .iter()
        .flat_map(|g| &g.stockages)
        .filter_map(|s| s.solaire.as_ref().map(|b| b.e_sol_kwh))
        .sum::<f64>()
        + e.generations
            .iter()
            .flat_map(|g| &g.solaires_collectifs)
            .map(|c| c.solaire.e_sol_kwh)
            .sum::<f64>();

    let exigences = params.map(|pa| {
        let mut zones = Vec::new();
        let mut verifications = Vec::new();
        let mut dh_manquants = Vec::new();
        let (mut bbio_max, mut cep_max, mut cep_nr_max, mut ic_max, mut s_tot) =
            (0.0, 0.0, 0.0, 0.0, 0.0);
        for ((z, r), (p, ex)) in bat.zones.iter().zip(e.zones).zip(&parties) {
            let Some(x) = ex else { continue };
            // Chapitre II, V : prorata des surfaces de référence des zones.
            bbio_max += x.bbio_max * p.sref_m2;
            cep_max += x.cep_max * p.sref_m2;
            cep_nr_max += x.cep_nr_max * p.sref_m2;
            ic_max += x.ic_energie_max * p.sref_m2;
            s_tot += p.sref_m2;
            zones.push(ExigencesZone {
                id: z.id.clone(),
                usage: p.usage,
                sref_m2: p.sref_m2,
                smoy_lgt_m2: x.smoy_lgt_m2,
                bbio_max_moyen: x.bbio_max_moyen,
                mb_geo: x.mb_geo,
                mb_combles: x.mb_combles,
                mb_surf_moy: x.mb_surf_moy,
                mb_surf_tot: x.mb_surf_tot,
                mb_bruit: x.mb_bruit,
                mb_hsp: x.mb_hsp,
                bbio_max_pts: x.bbio_max,
                cep_max_moyen: x.cep_max_moyen,
                cep_nr_max_moyen: x.cep_nr_max_moyen,
                ic_energie_max_moyen: x.ic_energie_max_moyen,
                mc_geo: x.mc_geo,
                mc_combles: x.mc_combles,
                mc_surf_moy: x.mc_surf_moy,
                mc_surf_tot: x.mc_surf_tot,
                mc_cat: x.mc_cat,
                mc_hsp: x.mc_hsp,
                cep_max_kwhep_m2: x.cep_max,
                cep_nr_max_kwhep_m2: x.cep_nr_max,
                ic_energie_max_kgco2_m2: x.ic_energie_max,
            });
            // Chapitre II, IV : DH_max de chaque partie thermiquement homogène (le groupe).
            for (g, rg) in z.groupes.iter().zip(&r.groupes) {
                let max = dh_max(
                    p.usage,
                    pa.categorie_contraintes_exterieures,
                    e.zone_climatique,
                    g.iclim == Some(true),
                    x.smoy_lgt_m2,
                );
                match (rg.confort_ete.as_ref().map(|c| c.dh_c_h), max) {
                    (Some(dh), Some(max)) => {
                        verifications.push(verifier("dh", Some(g.id.clone()), dh, max));
                    }
                    (None, Some(_)) => dh_manquants.push(format!("dh ({})", g.id)),
                    _ => {}
                }
            }
        }
        let moy = |x: f64| if s_tot > 0.0 { x / s_tot } else { 0.0 };
        let (bbio_max, cep_max, cep_nr_max, ic_max) =
            (moy(bbio_max), moy(cep_max), moy(cep_nr_max), moy(ic_max));
        let mut tete = Vec::new();
        if let Some(bbio) = e.bbio_pts {
            tete.push(verifier("bbio", None, bbio, bbio_max));
        }
        tete.push(verifier("cep", None, cep_m2, cep_max));
        tete.push(verifier("cep_nr", None, cep_nr_m2, cep_nr_max));
        if let Some(ic) = ic {
            tete.push(verifier("ic_energie", None, ic, ic_max));
        }
        tete.extend(verifications);
        // Un indicateur exigé qui n'a pas pu être calculé interdit de conclure à la conformité.
        let mut manquants = Vec::new();
        if e.bbio_pts.is_none() {
            manquants.push("bbio".to_owned());
        }
        if ic.is_none() {
            manquants.push("ic_energie".to_owned());
        }
        manquants.extend(dh_manquants);
        let conforme = manquants.is_empty() && tete.iter().all(|v| v.respectee);
        ResultatsExigences {
            bbio_max_pts: bbio_max,
            cep_max_kwhep_m2: cep_max,
            cep_nr_max_kwhep_m2: cep_nr_max,
            ic_energie_max_kgco2_m2: ic_max,
            zones,
            verifications: tete,
            conforme,
            indicateurs_manquants: manquants,
        }
    });
    if params.is_none() {
        avertissements.push(format!(
            "bâtiment {} : `exigences` non saisi : exigences maximales et conformité non \
             calculées",
            bat.id
        ));
    }

    (
        ResultatsIndicateurs {
            sref_m2: sref,
            electricite_consommee: consommee,
            electricite_importee: importee,
            electricite_importee_mensuel_kwh: imp_mensuel,
            production_pv,
            forfaits_refroidissement: forfaits,
            energie_solaire_captee_kwh: (solaire > 0.0).then_some(solaire),
            lignes,
            cef_kwhef_m2: par_m2(cef),
            cep_kwhep_m2: cep_m2,
            cep_nr_kwhep_m2: cep_nr_m2,
            ic_energie_kgco2_m2: ic,
            somme_f_co2: somme,
            exigences,
        },
        avertissements,
    )
}

/// Vérifie `valeur ≤ maximum` et calcule la marge.
fn verifier(
    indicateur: &str,
    groupe: Option<String>,
    valeur: f64,
    maximum: f64,
) -> VerificationExigence {
    let marge = maximum - valeur;
    VerificationExigence {
        indicateur: indicateur.to_owned(),
        groupe,
        valeur,
        maximum,
        marge,
        marge_pct: if maximum > 0.0 {
            100.0 * marge / maximum
        } else {
            0.0
        },
        respectee: valeur <= maximum,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn somme_des_coefficients_de_ponderation() {
        // Σ des années 1 à 50 de l'article 11 (année 0 exclue, éq. 74).
        let s = somme_f_co2();
        assert!((s - 39.543).abs() < 1e-9, "{s}");
    }
}
