// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation de la description d'un projet, séparée du calcul.
//!
//! Les contraintes vérifiées sont les plages des nomenclatures de l'annexe III (colonnes
//! « Min » et « Max »), l'unicité des identifiants et l'existence des références au catalogue.

use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thbce_data::ZoneClimatique;

use crate::bati::assemblage_groupe::EnveloppeResolue;
use crate::bati::baie::{u_baie, EnsembleBaies, EtatBaie, REDUCTION_SW_SP_C};
use crate::bati::comportement_thermique::{inertie_quotidienne, GroupeThermique};
use crate::bati::inerties_longues::{cma_surf_kj_k_m2, cms_surf_kj_k_m2};
use crate::bati::paroi_opaque::EnsembleParois;
use crate::bati::surventilation::traversant_conventionnel;
use crate::entree::{
    Baie, BaieType, Batiment, BoucheConduit, Catalogue, EntreesAir, EspaceTamponSolarise,
    Exposition, FacteursBaie, GestionPm, Groupe, Inertie, InertieAnnuelle, InertieQuotidienne,
    InertieSequentielle, Isouf, Masque, NatureEspaceTampon, ParoiOpaque, ParoiType, PontThermique,
    Projet, RepresentationEchangeur, SystemeVentilation, TypePm, Ventilation, Zone,
};
use crate::ventilation::bouche_conduit::{
    debits_specifiques_bbio_groupe, q_spec_conv_base_m3_h, DUGD_BBIO_H,
};
use crate::ventilation::debits_air_cep::{trav_zone_conventionnel, EntreeAir};

#[path = "validation/generation.rs"]
mod generation;
#[path = "validation/systemes.rs"]
mod systemes;

/// Nombre maximal de masques protégeant une surface (Th-BCE 2020 §3.2.2, « Nmasque »).
pub const NB_MASQUES_MAX: usize = 7;

/// Nombre de tranches d'un masque lointain azimutal (Th-BCE 2020 §3.2.3).
pub const NB_TRANCHES_AZIMUTALES: usize = 36;

/// Un problème relevé dans l'entrée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Probleme {
    /// Chemin de l'élément dans l'entrée, par exemple
    /// `batiments[0].zones[1].groupes[0].baies[3].a_b_m2`.
    pub chemin: String,
    /// Valeur fautive, quand elle est simple à afficher.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valeur: Option<String>,
    /// Contrainte non respectée, ou nature de l'avertissement.
    pub message: String,
}

/// Rapport de validation d'une entrée.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RapportValidation {
    /// Erreurs : l'entrée ne peut pas être calculée.
    pub erreurs: Vec<Probleme>,
    /// Avertissements : l'entrée est acceptée, mais un point mérite l'attention.
    pub avertissements: Vec<Probleme>,
}

impl RapportValidation {
    /// Vrai si l'entrée ne comporte aucune erreur.
    pub fn est_valide(&self) -> bool {
        self.erreurs.is_empty()
    }

    fn erreur(
        &mut self,
        chemin: impl Into<String>,
        valeur: Option<String>,
        message: impl Into<String>,
    ) {
        self.erreurs.push(Probleme {
            chemin: chemin.into(),
            valeur,
            message: message.into(),
        });
    }

    fn avertissement(
        &mut self,
        chemin: impl Into<String>,
        valeur: Option<String>,
        message: impl Into<String>,
    ) {
        self.avertissements.push(Probleme {
            chemin: chemin.into(),
            valeur,
            message: message.into(),
        });
    }

    /// Vérifie `min <= v <= max` ; une borne infinie n'est pas contrôlée.
    fn plage(&mut self, chemin: String, v: f64, min: f64, max: f64) {
        if !v.is_finite() {
            self.erreur(chemin, Some(v.to_string()), "valeur non finie");
        } else if v < min || v > max {
            let contrainte = if max.is_infinite() {
                format!("doit être supérieur ou égal à {min}")
            } else {
                format!("doit être compris entre {min} et {max}")
            };
            self.erreur(chemin, Some(v.to_string()), contrainte);
        }
    }

    fn positif(&mut self, chemin: String, v: f64) {
        self.plage(chemin, v, 0.0, f64::INFINITY);
    }

    fn strictement_positif(&mut self, chemin: String, v: f64) {
        if v.is_finite() && v <= 0.0 {
            self.erreur(chemin, Some(v.to_string()), "doit être strictement positif");
        } else {
            self.positif(chemin, v);
        }
    }

    /// Vérifie l'unicité d'une famille d'identifiants et renvoie leur ensemble.
    fn ids_uniques<'a>(
        &mut self,
        chemin: &str,
        ids: impl Iterator<Item = &'a String>,
    ) -> BTreeSet<&'a str> {
        let mut vus = BTreeSet::new();
        for (i, id) in ids.enumerate() {
            if id.trim().is_empty() {
                self.erreur(format!("{chemin}[{i}].id"), None, "identifiant vide");
            } else if !vus.insert(id.as_str()) {
                self.erreur(
                    format!("{chemin}[{i}].id"),
                    Some(id.clone()),
                    "identifiant déjà utilisé par un autre élément de la même liste",
                );
            }
        }
        vus
    }
}

/// Valide la description d'un projet.
pub fn valider(projet: &Projet) -> RapportValidation {
    let mut r = RapportValidation::default();

    // Th-BCE 2020 §3.1.2 : Alt, Dmer, Durb, minimum 0.
    let loc = &projet.localisation;
    r.positif("localisation.alt_m".into(), loc.alt_m);
    r.positif("localisation.dmer_km".into(), loc.dmer_km);
    r.positif("localisation.durb_km".into(), loc.durb_km);
    let zone_departement = ZoneClimatique::du_departement(loc.departement);
    if let Some(zone) = loc.zone_climatique {
        if zone != zone_departement {
            r.avertissement(
                "localisation.zone_climatique",
                Some(zone.to_string()),
                format!(
                    "la zone saisie contredit celle du département {} ({zone_departement}, arrêté \
                     du 26 octobre 2010, annexe I) ; la zone saisie est conservée",
                    loc.departement
                ),
            );
        }
    }

    // Catalogue.
    let cat = &projet.catalogue;
    let types_parois = r.ids_uniques("catalogue.parois", cat.parois.iter().map(|p| &p.id));
    r.ids_uniques("catalogue.baies", cat.baies.iter().map(|b| &b.id));
    let types_baies: BTreeMap<&str, &BaieType> =
        cat.baies.iter().map(|b| (b.id.as_str(), b)).collect();
    r.ids_uniques("catalogue.masques", cat.masques.iter().map(|m| &m.id));
    let masques: BTreeMap<&str, &Masque> = cat
        .masques
        .iter()
        .map(|m| (m.id.as_str(), &m.masque))
        .collect();
    for (i, p) in cat.parois.iter().enumerate() {
        valider_paroi_type(&mut r, &format!("catalogue.parois[{i}]"), p);
    }
    for (i, b) in cat.baies.iter().enumerate() {
        valider_baie_type(&mut r, &format!("catalogue.baies[{i}]"), b);
    }
    for (i, m) in cat.masques.iter().enumerate() {
        valider_masque(&mut r, &format!("catalogue.masques[{i}].masque"), &m.masque);
    }

    // Hiérarchie bâtiment → zone → groupe.
    if projet.batiments.is_empty() {
        r.erreur(
            "batiments",
            None,
            "le projet doit contenir au moins un bâtiment",
        );
    }
    r.ids_uniques("batiments", projet.batiments.iter().map(|b| &b.id));
    let refs = References {
        types_parois: &types_parois,
        types_baies: &types_baies,
        masques: &masques,
    };
    for (ib, bat) in projet.batiments.iter().enumerate() {
        let cb = format!("batiments[{ib}]");
        if bat.zones.is_empty() {
            r.erreur(
                format!("{cb}.zones"),
                None,
                "le bâtiment doit contenir au moins une zone",
            );
        }
        r.ids_uniques(&format!("{cb}.zones"), bat.zones.iter().map(|z| &z.id));
        for (iz, zone) in bat.zones.iter().enumerate() {
            valider_zone(&mut r, &format!("{cb}.zones[{iz}]"), zone, &refs);
        }
        valider_espaces_tampons(&mut r, &cb, bat, &refs);
        systemes::valider_systemes_batiment(&mut r, &cb, bat);
        generation::valider_generations(&mut r, &cb, bat);
    }
    // Contrôles de cohérence entre composants, une fois chaque élément valide : ils demandent
    // de résoudre l'enveloppe des groupes.
    if r.est_valide() {
        for (ib, bat) in projet.batiments.iter().enumerate() {
            for (iz, zone) in bat.zones.iter().enumerate() {
                if zone.usage.est_pris_en_charge() {
                    let c = format!("batiments[{ib}].zones[{iz}]");
                    valider_coherence_zone(&mut r, &c, zone, cat);
                }
            }
        }
    }
    r
}

struct References<'a> {
    types_parois: &'a BTreeSet<&'a str>,
    types_baies: &'a BTreeMap<&'a str, &'a BaieType>,
    masques: &'a BTreeMap<&'a str, &'a Masque>,
}

fn valider_zone(r: &mut RapportValidation, c: &str, zone: &Zone, refs: &References<'_>) {
    if !zone.usage.est_pris_en_charge() {
        r.avertissement(
            format!("{c}.usage"),
            Some(format!("{:?}", zone.usage)),
            format!(
                "usage n° {} reconnu mais non calculé par cette version du moteur (seuls les \
                 usages 1 et 2 le sont)",
                zone.usage.numero()
            ),
        );
    }
    // Th-BCE 2020 §4.1.3, éq. (34) : le nombre de logements est nécessaire en logement collectif.
    // En maison individuelle, il vaut 1 s'il n'est pas saisi (maisons accolées : à saisir).
    match (zone.usage, zone.nb_lgt) {
        (_, Some(0)) => r.erreur(
            format!("{c}.nb_lgt"),
            Some("0".into()),
            "doit être supérieur ou égal à 1",
        ),
        (crate::entree::UsageZone::LogementCollectif, None) => r.erreur(
            format!("{c}.nb_lgt"),
            None,
            "nombre de logements requis en logement collectif (Th-BCE 2020 §4.1.3, éq. 34)",
        ),
        _ => {}
    }
    valider_aeraulique_zone(r, c, zone);
    if zone.usage.est_habitation() {
        match &zone.ventilation {
            None => r.erreur(
                format!("{c}.ventilation"),
                None,
                "système de ventilation requis en habitation : il sert aux calculs du confort \
                 d'été et des consommations (Th-BCE 2020 §6.2 à 6.5)",
            ),
            Some(v) => valider_ventilation(r, &format!("{c}.ventilation"), v, zone),
        }
    }
    if zone.groupes.is_empty() {
        r.erreur(
            format!("{c}.groupes"),
            None,
            "la zone doit contenir au moins un groupe",
        );
    }
    r.ids_uniques(&format!("{c}.groupes"), zone.groupes.iter().map(|g| &g.id));
    for (ig, groupe) in zone.groupes.iter().enumerate() {
        valider_groupe(r, &format!("{c}.groupes[{ig}]"), groupe, zone, refs);
    }
}

fn valider_groupe(
    r: &mut RapportValidation,
    c: &str,
    g: &Groupe,
    zone: &Zone,
    refs: &References<'_>,
) {
    // Th-BCE 2020 §4.1.2 : la surface utile du groupe, Agr, sert à tous les usages (apports
    // internes, répartition entre groupes). §13.1.3, éq. (2375) et (2376) : la surface de
    // référence est SHAB en habitation, SU sinon.
    match g.su_m2 {
        Some(s) => r.strictement_positif(format!("{c}.su_m2"), s),
        None => r.erreur(
            format!("{c}.su_m2"),
            None,
            "surface utile du groupe requise (Agr, Th-BCE 2020 §4.1.2)",
        ),
    }
    match (zone.usage.est_habitation(), g.shab_m2) {
        (_, Some(s)) => r.strictement_positif(format!("{c}.shab_m2"), s),
        (true, None) => r.erreur(
            format!("{c}.shab_m2"),
            None,
            "surface habitable requise en habitation : c'est la surface de référence (Th-BCE 2020 §13.1.3)",
        ),
        (false, None) => {}
    }
    // Th-BCE 2020 §8.3.2 : volume du groupe, paramètre d'intégration du bilan hydrique.
    match g.v_m3 {
        Some(v) => r.strictement_positif(format!("{c}.v_m3"), v),
        None if zone.usage.est_pris_en_charge() => r.erreur(
            format!("{c}.v_m3"),
            None,
            "volume du groupe requis (V, Th-BCE 2020 §8.3.2)",
        ),
        None => {}
    }
    valider_inertie(r, &format!("{c}.inertie"), &g.inertie);

    r.ids_uniques(
        &format!("{c}.parois_opaques"),
        g.parois_opaques.iter().map(|p| &p.id),
    );
    for (i, p) in g.parois_opaques.iter().enumerate() {
        valider_paroi(r, &format!("{c}.parois_opaques[{i}]"), p, refs);
    }
    r.ids_uniques(&format!("{c}.baies"), g.baies.iter().map(|b| &b.id));
    for (i, b) in g.baies.iter().enumerate() {
        valider_baie(r, &format!("{c}.baies[{i}]"), b, refs);
    }
    // Th-BCE 2020 §5.13.3.2.1, tableau 43, et §5.13.3.2.2.
    let a_des_ouvrants = g.baies.iter().any(|b| {
        refs.types_baies
            .get(b.type_baie.as_str())
            .is_some_and(|t| t.ouvrant.is_some())
    });
    match (traversant_conventionnel(zone.usage.numero()), g.traversant) {
        (Some(conv), Some(saisi)) if conv != saisi => r.erreur(
            format!("{c}.traversant"),
            Some(saisi.to_string()),
            "le caractère traversant est conventionnel pour cet usage (tableau 43) : ne pas le \
             saisir",
        ),
        (None, None) if a_des_ouvrants && zone.usage.est_pris_en_charge() => r.erreur(
            format!("{c}.traversant"),
            None,
            "caractère traversant du groupe requis en logement collectif quand des baies sont \
             ouvrables (Th-BCE 2020 §5.13.3.2.1)",
        ),
        _ => {}
    }
    if let Some(h) = g.h_ttf_m {
        if !(h > 4.0 && h <= 15.0) {
            r.erreur(
                format!("{c}.h_ttf_m"),
                Some(h.to_string()),
                "la hauteur de tirage thermique ne se saisit que si elle dépasse 4 m, dans la \
                 limite de 15 m ; sinon elle vaut 1,5 m (Th-BCE 2020 §5.13.3.2.2)",
            );
        }
    }
    r.ids_uniques(
        &format!("{c}.ponts_thermiques"),
        g.ponts_thermiques.iter().map(|p| &p.id),
    );
    for (i, p) in g.ponts_thermiques.iter().enumerate() {
        valider_pont_thermique(r, &format!("{c}.ponts_thermiques[{i}]"), p, refs);
    }
    // Th-BCE 2020 §6.2.3.1.3 : en habitation, les débits du Bbio sont les débits d'hygiène des
    // bouches du groupe.
    if zone.usage.est_habitation() && g.bouches_conduits.is_empty() {
        r.erreur(
            format!("{c}.bouches_conduits"),
            None,
            "au moins une bouche de ventilation requise en habitation : ses débits d'hygiène de \
             pointe et de base donnent les débits du Bbio (Th-BCE 2020 §6.2.3.1.1.2)",
        );
    }
    r.ids_uniques(
        &format!("{c}.bouches_conduits"),
        g.bouches_conduits.iter().map(|b| &b.id),
    );
    for (i, b) in g.bouches_conduits.iter().enumerate() {
        valider_bouche_conduit(r, &format!("{c}.bouches_conduits[{i}]"), b);
    }
    if let Some(e) = &g.entrees_air {
        valider_entrees_air(r, &format!("{c}.entrees_air"), e);
    }
}

/// Contrôles qui portent sur l'assemblage des composants d'une zone calculée.
///
/// 1. Th-BCE 2020 §5.21.3.6, éq. (339) : `Hgemq = 1/(1/HTH,k+pt − 1/Hgmqs)` n'a de sens que si
///    l'enveloppe opaque du groupe conduit moins que l'échange superficiel de ses parois
///    lourdes, `HTH,k+pt < Hgmqs = his × Amq` (éq. 326) ; sinon le modèle thermique n'a pas de
///    solution (décision Q34 point 5).
/// 2. Th-BCE 2020 §5.6.3.5, éq. (167) : la pression intérieure équilibre les débits massiques
///    de la zone par ses défauts d'étanchéité et ses entrées d'air. Une zone qui n'a ni les uns
///    ni les autres ne peut pas équilibrer des débits soufflés et repris de masses différentes
///    (décision Q37 point 11) ; or l'air soufflé et l'air repris n'ont pas la même masse
///    volumique, même à débits volumiques égaux.
fn valider_coherence_zone(r: &mut RapportValidation, c: &str, zone: &Zone, catalogue: &Catalogue) {
    let mut surface_permeable = 0.0;
    let mut module_entrees_air = 0.0;
    let mut debits_specifiques = 0.0;
    for (ig, g) in zone.groupes.iter().enumerate() {
        let enveloppe = EnveloppeResolue::de(g, catalogue, zone.usage);
        let op = EnsembleParois::de(&enveloppe.parois);
        let ba = EnsembleBaies::de(&enveloppe.baies);
        surface_permeable += op.a_opv_m2 + ba.a_baies_v_m2 + op.a_ophh_m2 + ba.a_baies_h_m2;
        module_entrees_air += g.entrees_air.as_ref().map_or(0.0, |e| e.m_m3_h);
        if let Some(d) = debits_specifiques_bbio_groupe(&g.bouches_conduits) {
            debits_specifiques += d.q_souffle_m3_h - d.q_rep_m3_h;
        }
        let Some(a_gr_m2) = g.su_m2 else { continue };
        let h_th_k_pt = enveloppe.h_th_k_pt_w_k();
        let thermique = GroupeThermique::nouveau(a_gr_m2, ba.a_baies_m2, &g.inertie.quotidienne);
        if thermique.hg_emq_w_k(h_th_k_pt).is_none() {
            r.erreur(
                format!("{c}.groupes[{ig}]"),
                Some(g.id.clone()),
                format!(
                    "groupe « {} » : l'enveloppe opaque conduit plus que l'échange superficiel \
                     des parois lourdes : HTH,k+pt = {h_th_k_pt:.1} W/K (parois opaques et ponts \
                     thermiques, éq. 340) pour Hgmqs = {:.1} W/K (his × Amq, éq. 326) ; le \
                     modèle thermique demande HTH,k+pt < Hgmqs (Th-BCE 2020 §5.21.3.6, \
                     éq. 339) ; vérifier les surfaces et les coefficients des parois et des \
                     ponts thermiques, la surface utile et l'inertie quotidienne",
                    g.id, thermique.hg_mqs_w_k
                ),
            );
        }
    }
    let permeabilite = surface_permeable * zone.q4pa_surf_m3_h_m2.unwrap_or(0.0);
    if permeabilite <= 0.0 && module_entrees_air <= 0.0 && debits_specifiques > 0.0 {
        r.erreur(
            format!("{c}.q4pa_surf_m3_h_m2"),
            zone.q4pa_surf_m3_h_m2.map(|q| q.to_string()),
            format!(
                "zone « {} » : sans perméabilité à l'air (perméabilité nulle ou aucune paroi \
                 déperditive hors plancher bas) ni entrée d'air, les débits massiques soufflés \
                 et repris par la ventilation ne peuvent pas s'équilibrer : le bilan aéraulique \
                 n'a pas de solution (Th-BCE 2020 §5.6.3.5, éq. 167) ; saisir une perméabilité \
                 non nulle ou des entrées d'air",
                zone.id
            ),
        );
    }
}

/// Th-BCE 2020 §5.6.2 : paramètres du bilan aéraulique de la zone.
fn valider_aeraulique_zone(r: &mut RapportValidation, c: &str, zone: &Zone) {
    if zone.usage.est_pris_en_charge() {
        let requis = [
            ("h_zone_m", zone.h_zone_m, "hauteur de la zone"),
            (
                "h_plancher_m",
                zone.h_plancher_m,
                "hauteur du point le plus bas de la zone par rapport au sol extérieur",
            ),
            (
                "q4pa_surf_m3_h_m2",
                zone.q4pa_surf_m3_h_m2,
                "perméabilité à l'air de l'enveloppe sous 4 Pa",
            ),
        ];
        for (champ, valeur, nom) in requis {
            if valeur.is_none() {
                r.erreur(
                    format!("{c}.{champ}"),
                    None,
                    format!("{nom} requise (Th-BCE 2020 §5.6.2)"),
                );
            }
        }
    }
    if let Some(h) = zone.h_zone_m {
        r.strictement_positif(format!("{c}.h_zone_m"), h);
    }
    if let Some(h) = zone.h_plancher_m {
        if !h.is_finite() {
            r.erreur(
                format!("{c}.h_plancher_m"),
                Some(h.to_string()),
                "doit être un nombre fini",
            );
        }
    }
    if let Some(q) = zone.q4pa_surf_m3_h_m2 {
        r.positif(format!("{c}.q4pa_surf_m3_h_m2"), q);
    }
    // Th-BCE 2020 §5.6.3.1.3, tableau 22.
    match (trav_zone_conventionnel(zone.usage.numero()), zone.trav_zone) {
        (Some(_), Some(saisi)) => r.erreur(
            format!("{c}.trav_zone"),
            Some(saisi.to_string()),
            "le caractère traversant de la zone est conventionnel pour cet usage (tableau 22) : \
             ne pas le saisir",
        ),
        (None, None) => r.erreur(
            format!("{c}.trav_zone"),
            None,
            "caractère traversant de la zone au sens des conditions d'hiver requis en logement \
             collectif (Th-BCE 2020 §5.6.3.1.3)",
        ),
        _ => {}
    }
    // Th-BCE 2020 §5.6.3.2.2 : ΔP1 et ΔP2 sont les mêmes pour tous les groupes de la zone.
    let mut pressions: Option<(f64, f64)> = None;
    for (ig, g) in zone.groupes.iter().enumerate() {
        let Some(a) = g.entrees_air.as_ref().and_then(|e| e.autoreglable.as_ref()) else {
            continue;
        };
        let p = (a.delta_p1_pa, a.delta_p2_pa);
        match pressions {
            None => pressions = Some(p),
            Some(reference) if reference != p => r.erreur(
                format!("{c}.groupes[{ig}].entrees_air.autoreglable"),
                None,
                "les pressions ΔP1 et ΔP2 des entrées d'air autoréglables doivent être les \
                 mêmes pour tous les groupes d'une zone (Th-BCE 2020 §5.6.3.2.2)",
            ),
            Some(_) => {}
        }
    }
}

/// Th-BCE 2020 §5.6.2 et §5.6.3.2.1 : entrées d'air d'un groupe.
fn valider_entrees_air(r: &mut RapportValidation, c: &str, e: &EntreesAir) {
    r.positif(format!("{c}.m_m3_h"), e.m_m3_h);
    if let Some(v) = e.r {
        r.plage(format!("{c}.r"), v, 0.0, 1.0);
    }
    if let Some(a) = &e.autoreglable {
        r.strictement_positif(format!("{c}.autoreglable.delta_p1_pa"), a.delta_p1_pa);
        r.positif(format!("{c}.autoreglable.m_prime_m3_h"), a.m_prime_m3_h);
        if a.delta_p2_pa.is_nan() || a.delta_p2_pa <= a.delta_p1_pa {
            r.erreur(
                format!("{c}.autoreglable.delta_p2_pa"),
                Some(a.delta_p2_pa.to_string()),
                "ΔP2 doit être supérieure à ΔP1 (Th-BCE 2020 §5.6.3.2.1)",
            );
        } else if !EntreeAir::de_saisie(e).est_monotone() {
            r.erreur(
                format!("{c}.autoreglable.m_prime_m3_h"),
                Some(a.m_prime_m3_h.to_string()),
                "la caractéristique de l'entrée d'air doit être croissante : \
                 M′ × ΔP2^0,5 ≥ M × ΔP1^0,5 (Th-BCE 2020 §5.6.3.2.1)",
            );
        }
    }
}

/// Th-BCE 2020 §6.2.2 : les débits ont pour minimum 0 ; §6.2.3.1.1.2 : le débit de pointe est le
/// débit maximal de la bouche, le débit de base son débit minimal.
fn valider_bouche_conduit(r: &mut RapportValidation, c: &str, b: &BoucheConduit) {
    r.positif(
        format!("{c}.q_spec_conv_pointe_m3_h"),
        b.q_spec_conv_pointe_m3_h,
    );
    // §6.2.2 et §6.3.2 : Dugd de 0 à 168 h par semaine ; la formule du débit de base d'un
    // système hygroréglable divise par 168 − Dugd.
    if let Some(d) = b.dugd_h {
        if d.is_finite() && d >= 168.0 {
            r.erreur(
                format!("{c}.dugd_h"),
                Some(d.to_string()),
                "doit être inférieur à 168 h par semaine (Th-BCE 2020 §6.2.3.2.2.2)",
            );
        } else {
            r.positif(format!("{c}.dugd_h"), d);
        }
    }
    if let Some(cdep) = b.cdep {
        r.strictement_positif(format!("{c}.cdep"), cdep);
    }
    if let Some(a) = b.a_cond_m2 {
        r.positif(format!("{c}.a_cond_m2"), a);
    }
    if let (Some(d), Some(base)) = (b.dugd_h, q_spec_conv_base_m3_h(b, b.dugd_h.unwrap_or(0.0))) {
        if b.qvarep_spec_m3_h.is_some() && d.is_finite() && d < 168.0 && base < 0.0 {
            r.erreur(
                format!("{c}.qvarep_spec_m3_h"),
                b.qvarep_spec_m3_h.map(|q| q.to_string()),
                "le débit de base déduit de Qvarepspec avec la durée d'utilisation du grand débit \
                 de la bouche est négatif (Th-BCE 2020 §6.2.3.1.1.2)",
            );
        }
    }
    let (champ, valeur) = match (b.q_spec_conv_base_m3_h, b.qvarep_spec_m3_h) {
        (Some(base), None) => ("q_spec_conv_base_m3_h", base),
        (None, Some(q)) => ("qvarep_spec_m3_h", q),
        (Some(_), Some(_)) => {
            r.erreur(
                format!("{c}.qvarep_spec_m3_h"),
                None,
                "saisir le débit de base ou, pour un système hygroréglable, le débit Qvarepspec \
                 de l'avis technique, mais pas les deux (Th-BCE 2020 §6.2.3.1.1.2)",
            );
            return;
        }
        (None, None) => {
            r.erreur(
                format!("{c}.q_spec_conv_base_m3_h"),
                None,
                "débit de base requis, ou débit Qvarepspec pour un système hygroréglable \
                 (Th-BCE 2020 §6.2.3.1.1.2)",
            );
            return;
        }
    };
    r.positif(format!("{c}.{champ}"), valeur);
    // Le débit de base d'un système hygroréglable se déduit de Qvarepspec avec la durée
    // d'utilisation du grand débit ; celle du Bbio sert ici.
    if let Some(base) = q_spec_conv_base_m3_h(b, DUGD_BBIO_H) {
        if base.is_finite() && b.q_spec_conv_pointe_m3_h.is_finite() {
            if base > b.q_spec_conv_pointe_m3_h {
                r.erreur(
                    format!("{c}.{champ}"),
                    Some(valeur.to_string()),
                    "le débit de pointe doit être au moins égal au débit de base (Th-BCE 2020 \
                     §6.2.3.1.1.2)",
                );
            } else if base < 0.0 {
                r.erreur(
                    format!("{c}.{champ}"),
                    Some(valeur.to_string()),
                    "le débit de base déduit de Qvarepspec est négatif : Qvarepspec est trop \
                     faible devant le débit de pointe (Th-BCE 2020 §6.2.3.1.1.2)",
                );
            }
        }
    }
}

/// Th-BCE 2020 §5.21.2 : toutes les grandeurs d'inertie ont pour minimum 0.
fn valider_inertie(r: &mut RapportValidation, c: &str, i: &Inertie) {
    if let InertieQuotidienne::Personnalisee {
        amq_surf_m2_m2,
        cmq_surf_kj_k_m2,
    } = i.quotidienne
    {
        r.positif(format!("{c}.quotidienne.amq_surf_m2_m2"), amq_surf_m2_m2);
        r.positif(
            format!("{c}.quotidienne.cmq_surf_kj_k_m2"),
            cmq_surf_kj_k_m2,
        );
    }
    if let InertieSequentielle::Personnalisee { cms_surf_kj_k_m2 } = i.sequentielle {
        r.positif(
            format!("{c}.sequentielle.cms_surf_kj_k_m2"),
            cms_surf_kj_k_m2,
        );
    }
    if let InertieAnnuelle::Personnalisee { cma_surf_kj_k_m2 } = i.annuelle {
        r.positif(format!("{c}.annuelle.cma_surf_kj_k_m2"), cma_surf_kj_k_m2);
    }
    r.positif(format!("{c}.ams_surf_m2_m2"), i.ams_surf_m2_m2);
    r.positif(format!("{c}.ama_surf_m2_m2"), i.ama_surf_m2_m2);

    // Th-BCE 2020 §5.21.3.4 : « Par définition, ΔCm,s−q et ΔCm,a−s sont positives ou nulles »
    // (éq. 331 et 332) ; `ΔHg` est nul pour `ΔR` nul (éq. 329 et 330), et un `ΔR` négatif n'a
    // pas de sens. Une saisie qui y contrevient est signalée ; le calcul Th-D prend alors la
    // capacité ou la conductance nulle (`bati::inerties_longues`).
    let (amq_surf, cmq_surf) = inertie_quotidienne(&i.quotidienne);
    let cms_surf = cms_surf_kj_k_m2(&i.sequentielle, cmq_surf);
    let cma_surf = cma_surf_kj_k_m2(&i.annuelle, cms_surf);
    if cms_surf < cmq_surf {
        r.avertissement(
            format!("{c}.sequentielle"),
            Some(cms_surf.to_string()),
            format!(
                "la capacité séquentielle Cms_surf ({cms_surf} kJ/(K.m²)) est inférieure à la \
                 capacité quotidienne Cmq_surf ({cmq_surf}) : ΔCm,s−q, positive ou nulle par \
                 définition, est prise nulle et l'inertie séquentielle est sans effet \
                 (Th-BCE 2020 §5.21.3.4, éq. 331)"
            ),
        );
    }
    if cma_surf < cms_surf {
        r.avertissement(
            format!("{c}.annuelle"),
            Some(cma_surf.to_string()),
            format!(
                "la capacité annuelle Cma_surf ({cma_surf} kJ/(K.m²)) est inférieure à la \
                 capacité séquentielle Cms_surf ({cms_surf}) : ΔCm,a−s, positive ou nulle par \
                 définition, est prise nulle et l'inertie annuelle est sans effet \
                 (Th-BCE 2020 §5.21.3.4, éq. 332)"
            ),
        );
    }
    for (champ, a_surf, nom, equation) in [
        ("ams_surf_m2_m2", i.ams_surf_m2_m2, "séquentielle", 329),
        ("ama_surf_m2_m2", i.ama_surf_m2_m2, "annuelle", 330),
    ] {
        if a_surf > amq_surf {
            r.avertissement(
                format!("{c}.{champ}"),
                Some(a_surf.to_string()),
                format!(
                    "la surface d'échange de l'inertie {nom} ({a_surf} m²/m²) dépasse celle de \
                     l'inertie quotidienne Amq_surf ({amq_surf}) : la résistance ΔR serait \
                     négative ; la conductance est prise nulle et l'inertie {nom} est sans \
                     effet (Th-BCE 2020 §5.21.3.4, éq. {equation})"
                ),
            );
        }
    }
}

/// Th-BCE 2020 §3.2.2 : α de 0 à 360°, β de 0 à 180°, dimensions positives, 0 à 7 masques.
fn valider_exposition(r: &mut RapportValidation, c: &str, e: &Exposition, refs: &References<'_>) {
    r.plage(format!("{c}.azimut_deg"), e.azimut_deg, 0.0, 360.0);
    r.plage(
        format!("{c}.inclinaison_deg"),
        e.inclinaison_deg,
        0.0,
        180.0,
    );
    if let Some(l) = e.lpb_m {
        r.positif(format!("{c}.lpb_m"), l);
    }
    if let Some(h) = e.hpb_m {
        r.positif(format!("{c}.hpb_m"), h);
    }
    if e.masques.len() > NB_MASQUES_MAX {
        r.erreur(
            format!("{c}.masques"),
            Some(e.masques.len().to_string()),
            format!("au plus {NB_MASQUES_MAX} masques par surface (Th-BCE 2020 §3.2.2)"),
        );
    }
    #[allow(clippy::float_cmp)]
    let verticale = e.inclinaison_deg == 90.0;
    for (i, id) in e.masques.iter().enumerate() {
        let Some(masque) = refs.masques.get(id.as_str()) else {
            r.erreur(
                format!("{c}.masques[{i}]"),
                Some(id.clone()),
                "masque absent du catalogue",
            );
            continue;
        };
        // Th-BCE 2020 §3.2.3, éq. (19) et (20) : les masques proches divisent par la largeur lp
        // (masques verticaux) ou la hauteur hp (masque horizontal) du composant.
        let (dimension, champ, valeur) = match masque {
            Masque::ProcheVerticalDroit { .. } | Masque::ProcheVerticalGauche { .. } => {
                ("largeur", "lpb_m", e.lpb_m)
            }
            Masque::ProcheHorizontal { .. } => ("hauteur", "hpb_m", e.hpb_m),
            _ => continue,
        };
        if !verticale {
            r.avertissement(
                format!("{c}.masques[{i}]"),
                Some(id.clone()),
                "masque proche sans effet : il ne s'applique qu'à un composant vertical (inclinaison de 90°, Th-BCE 2020 §3.2.3)",
            );
        } else if !valeur.is_some_and(|v| v > 0.0) {
            r.erreur(
                format!("{c}.{champ}"),
                valeur.map(|v| v.to_string()),
                format!("{dimension} strictement positive requise par le masque proche « {id} »"),
            );
        }
    }
}

fn valider_paroi(r: &mut RapportValidation, c: &str, p: &ParoiOpaque, refs: &References<'_>) {
    if !refs.types_parois.contains(p.type_paroi.as_str()) {
        r.erreur(
            format!("{c}.type_paroi"),
            Some(p.type_paroi.clone()),
            "type de paroi absent du catalogue",
        );
    }
    r.positif(format!("{c}.a_k_m2"), p.a_k_m2);
    valider_exposition(r, &format!("{c}.exposition"), &p.exposition, refs);
}

fn valider_baie(r: &mut RapportValidation, c: &str, b: &Baie, refs: &References<'_>) {
    let beta = b.exposition.inclinaison_deg;
    match refs.types_baies.get(b.type_baie.as_str()) {
        None => r.erreur(
            format!("{c}.type_baie"),
            Some(b.type_baie.clone()),
            "type de baie absent du catalogue",
        ),
        // Th-BCE 2020 §5.10.3.5, éq. (207) : le coefficient U sans protection mobile doit être
        // connu à l'inclinaison de la baie.
        Some(t) => {
            // Un type sans aucun coefficient est déjà signalé dans le catalogue.
            let un_u_saisi = t.u_sp_vert_w_m2k.is_some() || t.u_sp_horiz_w_m2k.is_some();
            if un_u_saisi
                && (0.0..=90.0).contains(&beta)
                && u_baie(t.u_sp_vert_w_m2k, t.u_sp_horiz_w_m2k, beta).is_none()
            {
                r.erreur(
                    format!("{c}.type_baie"),
                    Some(b.type_baie.clone()),
                    format!(
                        "le type de baie ne donne pas le coefficient U nécessaire à une \
                         inclinaison de {beta}° : u_sp_horiz_w_m2k en dessous de 60°, \
                         u_sp_vert_w_m2k à partir de 30° (les deux entre 30 et 60°)"
                    ),
                );
            }
            // Éq. (208) : de même pour le coefficient avec la protection mobile en place.
            let un_u_ap_saisi = t.u_ap_vert_w_m2k.is_some() || t.u_ap_horiz_w_m2k.is_some();
            if t.protection_mobile.is_some()
                && un_u_ap_saisi
                && (0.0..=90.0).contains(&beta)
                && u_baie(t.u_ap_vert_w_m2k, t.u_ap_horiz_w_m2k, beta).is_none()
            {
                r.erreur(
                    format!("{c}.type_baie"),
                    Some(b.type_baie.clone()),
                    format!(
                        "le type de baie ne donne pas le coefficient U avec protection mobile \
                         nécessaire à une inclinaison de {beta}° : u_ap_horiz_w_m2k en dessous \
                         de 60°, u_ap_vert_w_m2k à partir de 30°"
                    ),
                );
            }
        }
    }
    // Th-BCE 2020 §5.13.2 : classe d'exposition au bruit d'une baie ouvrable.
    if b.expo_br.is_none()
        && refs
            .types_baies
            .get(b.type_baie.as_str())
            .is_some_and(|t| t.ouvrant.is_some())
    {
        r.erreur(
            format!("{c}.expo_br"),
            None,
            "classe d'exposition au bruit requise pour une baie ouvrable",
        );
    }
    // Th-BCE 2020 §5.10.2 : inclinaison d'une baie entre 0° (horizontale) et 90° (verticale).
    if beta > 90.0 {
        r.erreur(
            format!("{c}.exposition.inclinaison_deg"),
            Some(beta.to_string()),
            "l'inclinaison d'une baie est comprise entre 0 et 90°",
        );
    }
    r.positif(format!("{c}.a_b_m2"), b.a_b_m2);
    valider_exposition(r, &format!("{c}.exposition"), &b.exposition, refs);
}

/// Th-BCE 2020 §5.19.2.
fn valider_pont_thermique(
    r: &mut RapportValidation,
    c: &str,
    p: &PontThermique,
    refs: &References<'_>,
) {
    r.positif(format!("{c}.psi_l_w_mk"), p.psi_l_w_mk);
    r.positif(format!("{c}.l_l_m"), p.l_l_m);
    r.plage(format!("{c}.sf_c_l"), p.sf_c_l, 0.0, 1.0);
    r.plage(format!("{c}.sf_e_l"), p.sf_e_l, 0.0, 1.0);
    if let Some(e) = &p.exposition {
        // Pour les masques proches, la largeur du pont thermique est conventionnelle (§3.2.3).
        let e = Exposition {
            lpb_m: Some(crate::bati::pont_thermique::LARGEUR_MASQUES_PROCHES_M),
            ..e.clone()
        };
        valider_exposition(r, &format!("{c}.exposition"), &e, refs);
    }
}

/// Th-BCE 2020 §5.17.2.
fn valider_paroi_type(r: &mut RapportValidation, c: &str, p: &ParoiType) {
    r.positif(format!("{c}.u_k_w_m2k"), p.u_k_w_m2k);
    r.plage(format!("{c}.sf_c_k"), p.sf_c_k, 0.0, 1.0);
    r.plage(format!("{c}.sf_e_k"), p.sf_e_k, 0.0, 1.0);
    if let Some(v) = &p.vegetalisation {
        r.plage(format!("{c}.vegetalisation.sf_c_v_k"), v.sf_c_v_k, 0.0, 1.0);
        r.plage(format!("{c}.vegetalisation.sf_e_v_k"), v.sf_e_v_k, 0.0, 1.0);
    }
}

/// Th-BCE 2020 §5.10.2.
fn valider_baie_type(r: &mut RapportValidation, c: &str, b: &BaieType) {
    if b.u_sp_vert_w_m2k.is_none() && b.u_sp_horiz_w_m2k.is_none() {
        r.erreur(
            c.to_owned(),
            None,
            "au moins un des coefficients u_sp_vert_w_m2k et u_sp_horiz_w_m2k est requis",
        );
    }
    for (champ, u) in [
        ("u_sp_vert_w_m2k", b.u_sp_vert_w_m2k),
        ("u_sp_horiz_w_m2k", b.u_sp_horiz_w_m2k),
        ("u_ap_vert_w_m2k", b.u_ap_vert_w_m2k),
        ("u_ap_horiz_w_m2k", b.u_ap_horiz_w_m2k),
    ] {
        if let Some(u) = u {
            r.positif(format!("{c}.{champ}"), u);
        }
    }
    if let Some(o) = &b.ouvrant {
        // Th-BCE 2020 §5.13.3.2.3 : Rouvmax saisi, ou déduit du type d'ouvrant (tableau 44).
        match (o.r_ouv_max, o.type_ouvrant) {
            (Some(v), None) => r.plage(format!("{c}.ouvrant.r_ouv_max"), v, 0.0, 1.0),
            (None, Some(_)) => {}
            _ => r.erreur(
                format!("{c}.ouvrant"),
                None,
                "renseigner soit r_ouv_max, soit type_ouvrant",
            ),
        }
    }
    if let Some(pm) = &b.protection_mobile {
        // Décision Q30, point 7 : pas de calcul approché pour ce que la version ne couvre pas.
        if pm.type_pm == TypePm::StoreVenitien {
            r.erreur(
                format!("{c}.protection_mobile.type_pm"),
                Some("store_venitien".to_owned()),
                "store vénitien non pris en charge dans cette version",
            );
        }
        if pm.gestion == GestionPm::AutomatiqueAvecDerogation {
            r.erreur(
                format!("{c}.protection_mobile.gestion"),
                Some("automatique_avec_derogation".to_owned()),
                "gestion automatique des protections mobiles non prise en charge dans cette \
                 version",
            );
        }
        // Th-BCE 2020 §5.10.2 : facteurs et coefficient U de la baie avec la protection en place.
        if b.avec_protection.is_none() {
            r.erreur(
                format!("{c}.avec_protection"),
                None,
                "requis quand une protection mobile est déclarée",
            );
        }
        if b.u_ap_vert_w_m2k.is_none() && b.u_ap_horiz_w_m2k.is_none() {
            r.erreur(
                c.to_owned(),
                None,
                "au moins un des coefficients u_ap_vert_w_m2k et u_ap_horiz_w_m2k est requis \
                 quand une protection mobile est déclarée",
            );
        }
    }
    valider_facteurs_baie(
        r,
        &format!("{c}.sans_protection"),
        &b.sans_protection,
        EtatBaie::SansProtection,
    );
    if let Some(f) = &b.avec_protection {
        valider_facteurs_baie(
            r,
            &format!("{c}.avec_protection"),
            f,
            EtatBaie::AvecProtection,
        );
    }
}

fn valider_facteurs_baie(r: &mut RapportValidation, c: &str, f: &FacteursBaie, etat: EtatBaie) {
    let composantes_c = [("sw1_c", f.sw1_c), ("sw2_c", f.sw2_c), ("sw3_c", f.sw3_c)];
    let saisies = composantes_c.iter().filter(|(_, v)| v.is_some()).count();
    if saisies != 0 && saisies != composantes_c.len() {
        r.erreur(
            c.to_owned(),
            None,
            "sw1_c, sw2_c et sw3_c se saisissent ensemble ou pas du tout (absents, la valeur par \
             défaut du §5.10.3.6.3 s'applique au facteur solaire global)",
        );
    }
    // Th-BCE 2020 §5.10.3.6.3, éq. (215) : la valeur par défaut `Swsp-c = Swsp-e − 0,1` n'a de
    // sens que si elle reste positive.
    if saisies == 0
        && etat == EtatBaie::SansProtection
        && f.sw1_e + f.sw2_e + f.sw3_e <= REDUCTION_SW_SP_C
    {
        r.erreur(
            c.to_owned(),
            None,
            "la valeur par défaut Swsp-c = Swsp-e − 0,1 (éq. 215) n'est pas positive : saisir \
             sw1_c, sw2_c et sw3_c",
        );
    }
    for (champ, v) in composantes_c.into_iter().chain([("tlid", f.tlid)]) {
        if let Some(v) = v {
            r.plage(format!("{c}.{champ}"), v, 0.0, 1.0);
        }
    }
    for (champ, v) in [
        ("sw1_e", f.sw1_e),
        ("sw2_e", f.sw2_e),
        ("sw3_e", f.sw3_e),
        ("tli", f.tli),
    ] {
        r.plage(format!("{c}.{champ}"), v, 0.0, 1.0);
    }
}

/// Th-BCE 2020 §3.2.2.
fn valider_masque(r: &mut RapportValidation, c: &str, m: &Masque) {
    match m {
        Masque::LointainAzimutal { gamma_i_deg } | Masque::LointainArbresCaducs { gamma_i_deg } => {
            if gamma_i_deg.len() != NB_TRANCHES_AZIMUTALES {
                r.erreur(
                    format!("{c}.gamma_i_deg"),
                    Some(gamma_i_deg.len().to_string()),
                    format!("{NB_TRANCHES_AZIMUTALES} tranches azimutales de 10° attendues (Th-BCE 2020 §3.2.3)"),
                );
            }
            for (i, g) in gamma_i_deg.iter().enumerate() {
                r.plage(format!("{c}.gamma_i_deg[{i}]"), *g, 0.0, 90.0);
            }
        }
        Masque::LointainPlanVertical { de_m, hpe_m } => {
            r.positif(format!("{c}.de_m"), *de_m);
            r.positif(format!("{c}.hpe_m"), *hpe_m);
        }
        Masque::ProcheVerticalDroit { dvd_m, dpd_m } => {
            r.positif(format!("{c}.dvd_m"), *dvd_m);
            r.positif(format!("{c}.dpd_m"), *dpd_m);
        }
        Masque::ProcheVerticalGauche { dvg_m, dpg_m } => {
            r.positif(format!("{c}.dvg_m"), *dvg_m);
            r.positif(format!("{c}.dpg_m"), *dpg_m);
        }
        Masque::ProcheHorizontal { dhm_m, dhp_m } => {
            r.positif(format!("{c}.dhm_m"), *dhm_m);
            r.positif(format!("{c}.dhp_m"), *dhp_m);
        }
    }
}

/// Espaces tampons d'un bâtiment (Th-BCE 2020 §5.2 à 5.4) : plages des tableaux 14, 15 et 19,
/// références des composants qui donnent sur eux.
fn valider_espaces_tampons(
    r: &mut RapportValidation,
    cb: &str,
    bat: &Batiment,
    refs: &References<'_>,
) {
    let c_et = format!("{cb}.espaces_tampons");
    let ids = r.ids_uniques(&c_et, bat.espaces_tampons.iter().map(|e| &e.id));
    let solarises: BTreeSet<&str> = bat
        .espaces_tampons
        .iter()
        .filter(|e| matches!(e.nature, NatureEspaceTampon::Solarise(_)))
        .map(|e| e.id.as_str())
        .collect();
    let mut associes: BTreeSet<&str> = BTreeSet::new();
    // Références depuis les composants des groupes : (chemin, identifiant, baie, enveloppe).
    let mut references: Vec<(String, &String, bool, bool)> = Vec::new();
    for (iz, zone) in bat.zones.iter().enumerate() {
        for (ig, g) in zone.groupes.iter().enumerate() {
            let cg = format!("{cb}.zones[{iz}].groupes[{ig}]");
            for (i, p) in g.parois_opaques.iter().enumerate() {
                if let Some(id) = &p.espace_tampon {
                    references.push((
                        format!("{cg}.parois_opaques[{i}].espace_tampon"),
                        id,
                        false,
                        true,
                    ));
                }
            }
            for (i, p) in g.ponts_thermiques.iter().enumerate() {
                if let Some(id) = &p.espace_tampon {
                    references.push((
                        format!("{cg}.ponts_thermiques[{i}].espace_tampon"),
                        id,
                        false,
                        true,
                    ));
                }
            }
            for (i, b) in g.baies.iter().enumerate() {
                if let Some(id) = &b.espace_tampon {
                    references.push((format!("{cg}.baies[{i}].espace_tampon"), id, true, true));
                }
            }
            // Les entrées d'air ne suffisent pas à associer l'espace tampon à l'enveloppe.
            if let Some(id) = g
                .entrees_air
                .as_ref()
                .and_then(|e| e.espace_tampon.as_ref())
            {
                references.push((format!("{cg}.entrees_air.espace_tampon"), id, false, false));
            }
        }
    }
    for (chemin, id, baie, enveloppe) in references {
        if !ids.contains(id.as_str()) {
            r.erreur(
                chemin,
                Some(id.clone()),
                "espace tampon inconnu dans ce bâtiment",
            );
            continue;
        }
        if enveloppe {
            associes.insert(id.as_str());
        }
        // Th-BCE 2020 §5.10.3 : « les baies vitrées ne peuvent donner que sur des espaces
        // tampons vitrés ».
        if baie && !solarises.contains(id.as_str()) {
            r.erreur(
                chemin,
                Some(id.clone()),
                "une baie ne peut donner que sur un espace tampon solarisé (Th-BCE 2020 §5.10.3, \
                 « Présence d'un espace tampon »)",
            );
        }
    }
    for (i, et) in bat.espaces_tampons.iter().enumerate() {
        let c = format!("{c_et}[{i}]");
        // §5.4.1 : un espace tampon « doit nécessairement être associé à au moins un composant
        // d'enveloppe (paroi opaque, baie) d'un groupe ».
        if !et.id.trim().is_empty() && !associes.contains(et.id.as_str()) {
            r.erreur(
                format!("{c}.id"),
                Some(et.id.clone()),
                "aucune paroi opaque, baie ni aucun pont thermique ne donne sur cet espace tampon \
                 (Th-BCE 2020 §5.4.1)",
            );
        }
        match &et.nature {
            NatureEspaceTampon::NonSolarise(n) => {
                // Tableau 19 : de −5 à 1.
                r.plage(
                    format!("{c}.nature.non_solarise.b_et_ns"),
                    n.b_et_ns,
                    -5.0,
                    1.0,
                );
            }
            NatureEspaceTampon::Solarise(s) => {
                valider_espace_solarise(r, &format!("{c}.nature.solarise"), s, refs);
            }
        }
    }
}

fn valider_espace_solarise(
    r: &mut RapportValidation,
    c: &str,
    s: &EspaceTamponSolarise,
    refs: &References<'_>,
) {
    r.strictement_positif(format!("{c}.a_et_m2"), s.a_et_m2);
    r.positif(format!("{c}.u_et_pb_w_m2k"), s.u_et_pb_w_m2k);
    r.positif(format!("{c}.h_ttf_et_m"), s.h_ttf_et_m);
    if let Some(v) = s.cm_et_surf_kj_m2k {
        r.positif(format!("{c}.cm_et_surf_kj_m2k"), v);
    }
    if let Some(v) = s.qv_et_base_m3_h {
        r.positif(format!("{c}.qv_et_base_m3_h"), v);
    }
    if s.parois.is_empty() {
        r.erreur(
            format!("{c}.parois"),
            None,
            "un espace tampon solarisé a au moins une paroi donnant sur l'extérieur",
        );
    }
    r.ids_uniques(&format!("{c}.parois"), s.parois.iter().map(|p| &p.id));
    let protections = s.protections_mobiles.is_some();
    let mut h_ext = s.u_et_pb_w_m2k * s.a_et_m2;
    for (i, p) in s.parois.iter().enumerate() {
        let cp = format!("{c}.parois[{i}]");
        r.strictement_positif(format!("{cp}.a_ue_m2"), p.a_ue_m2);
        r.positif(format!("{cp}.u_ue_w_m2k"), p.u_ue_w_m2k);
        h_ext += p.a_ue_m2 * p.u_ue_w_m2k;
        valider_exposition(r, &format!("{cp}.exposition"), &p.exposition, refs);
        r.plage(format!("{cp}.swe_sp"), p.swe_sp, 0.0, 1.0);
        r.plage(format!("{cp}.tle_sp"), p.tle_sp, 0.0, 1.0);
        r.plage(format!("{cp}.r_ouv_max"), p.r_ouv_max, 0.0, 1.0);
        for (nom, v) in [("swe_ap", p.swe_ap), ("tle_ap", p.tle_ap)] {
            match (v, protections) {
                (Some(v), _) => r.plage(format!("{cp}.{nom}"), v, 0.0, 1.0),
                (None, true) => r.erreur(
                    format!("{cp}.{nom}"),
                    None,
                    "requis quand l'espace tampon a des protections mobiles (Th-BCE 2020 \
                     §5.4.3.3.1.3, éq. 118 et 128)",
                ),
                (None, false) => {}
            }
        }
    }
    // Éq. (124) et (125) : Het doit être strictement positif ; les échanges avec le volume
    // chauffé y contribuent toujours, mais l'espace tampon doit aussi échanger avec l'extérieur.
    let qv_base = s.qv_et_base_m3_h.unwrap_or(
        crate::bati::espace_tampon::espace_tampon::QV_ET_BASE_DEFAUT_M3_H_M2 * s.a_et_m2,
    );
    if h_ext <= 0.0 && qv_base <= 0.0 && r.est_valide() {
        r.erreur(
            format!("{c}.parois"),
            None,
            "l'espace tampon n'échange ni chaleur ni air avec l'extérieur (Th-BCE 2020 §5.4.3.3.1.5, \
             éq. 124)",
        );
    }
}

/// Message des systèmes et fonctions de ventilation hors de la première version.
const HORS_PERIMETRE_VENTILATION: &str = "non pris en charge dans cette version du moteur \
     (après v1, docs/couverture.md)";

/// Th-BCE 2020 §6.2 à 6.5 : système de ventilation d'une zone d'habitation.
fn valider_ventilation(r: &mut RapportValidation, c: &str, v: &Ventilation, zone: &Zone) {
    let bouches = || zone.groupes.iter().flat_map(|g| &g.bouches_conduits);
    // Une zone sans groupe, ou sans bouche, est déjà signalée par ailleurs.
    let sans_bouche = bouches().next().is_none();
    let a_reprise = sans_bouche || bouches().any(|b| b.isouf == Isouf::Reprise);
    let a_soufflage = bouches().any(|b| b.isouf == Isouf::Soufflage);
    let cs = format!("{c}.systeme");
    match &v.systeme {
        SystemeVentilation::SimpleFluxExtraction(p)
        | SystemeVentilation::MecaniqueBassePression(p) => {
            let nature = if matches!(v.systeme, SystemeVentilation::SimpleFluxExtraction(_)) {
                "simple_flux_extraction"
            } else {
                "mecanique_basse_pression"
            };
            r.positif(format!("{cs}.{nature}.p_vent_base_w"), p.p_vent_base_w);
            r.positif(format!("{cs}.{nature}.p_vent_pointe_w"), p.p_vent_pointe_w);
            if a_soufflage {
                r.erreur(
                    cs.clone(),
                    None,
                    "un système par extraction n'a que des bouches de reprise (isouf = reprise) \
                     (Th-BCE 2020 §6.4 et §6.5)",
                );
            }
            if !a_reprise {
                r.erreur(cs.clone(), None, "au moins une bouche de reprise requise");
            }
        }
        SystemeVentilation::SimpleFluxInsufflation(p) => {
            r.positif(
                format!("{cs}.simple_flux_insufflation.p_vent_base_w"),
                p.p_vent_base_w,
            );
            r.positif(
                format!("{cs}.simple_flux_insufflation.p_vent_pointe_w"),
                p.p_vent_pointe_w,
            );
            if a_reprise && !sans_bouche {
                r.erreur(
                    cs.clone(),
                    None,
                    "un système par insufflation n'a que des bouches de soufflage (isouf = \
                     soufflage) (Th-BCE 2020 §6.5)",
                );
            }
            if !a_soufflage && !sans_bouche {
                r.erreur(cs.clone(), None, "au moins une bouche de soufflage requise");
            }
        }
        SystemeVentilation::DoubleFlux(df) => {
            let cd = format!("{cs}.double_flux");
            r.positif(format!("{cd}.p_vent_base_rep_w"), df.p_vent_base_rep_w);
            r.positif(format!("{cd}.p_vent_pointe_rep_w"), df.p_vent_pointe_rep_w);
            r.positif(format!("{cd}.p_vent_base_sou_w"), df.p_vent_base_sou_w);
            r.positif(format!("{cd}.p_vent_pointe_sou_w"), df.p_vent_pointe_sou_w);
            if !(a_reprise && a_soufflage) && !sans_bouche {
                r.erreur(
                    cs.clone(),
                    None,
                    "un double flux a des bouches de reprise et des bouches de soufflage \
                     (Th-BCE 2020 §6.3)",
                );
            }
            for (champ, present) in [
                ("prechauffage", df.prechauffage),
                ("humidification", df.humidification),
                (
                    "rafraichissement_evaporatif",
                    df.rafraichissement_evaporatif,
                ),
                ("surventilation_mecanique", df.surventilation_mecanique),
            ] {
                if present == Some(true) {
                    r.erreur(
                        format!("{cd}.{champ}"),
                        Some("true".into()),
                        HORS_PERIMETRE_VENTILATION,
                    );
                }
            }
            if let Some(e) = &df.echangeur {
                let ce = format!("{cd}.echangeur");
                match &e.representation {
                    RepresentationEchangeur::Simplifiee(s) => r.plage(
                        format!("{ce}.representation.simplifiee.epsilon_saisi"),
                        s.epsilon_saisi,
                        0.0,
                        1.0,
                    ),
                    RepresentationEchangeur::Detaillee(d) => {
                        r.positif(format!("{ce}.representation.detaillee.ua_w_k"), d.ua_w_k);
                    }
                }
                if e.rotatif == Some(true) {
                    r.erreur(
                        format!("{ce}.rotatif"),
                        Some("true".into()),
                        format!("échangeur rotatif {HORS_PERIMETRE_VENTILATION}"),
                    );
                }
                if let Some(t) = e.theta_s_ech_rep_lim_c {
                    r.plage(format!("{ce}.theta_s_ech_rep_lim_c"), t, -10.0, 10.0);
                }
                if let Some(b) = &e.bypass {
                    for (champ, t) in [
                        ("theta_ext_bypass_hiver_c", b.theta_ext_bypass_hiver_c),
                        ("theta_ext_bypass_ete_c", b.theta_ext_bypass_ete_c),
                        ("theta_int_bypass_hiver_c", b.theta_int_bypass_hiver_c),
                        ("theta_int_bypass_ete_c", b.theta_int_bypass_ete_c),
                    ] {
                        r.plage(
                            format!("{ce}.bypass.{champ}"),
                            t,
                            f64::NEG_INFINITY,
                            f64::INFINITY,
                        );
                    }
                }
            }
        }
        SystemeVentilation::NaturelleParConduits
        | SystemeVentilation::Hybride
        | SystemeVentilation::Aeration => r.erreur(
            cs.clone(),
            None,
            format!(
                "ventilation naturelle, hybride ou par ouverture des fenêtres (Th-BCE 2020 §6.6 \
                 et §6.7) {HORS_PERIMETRE_VENTILATION}"
            ),
        ),
        SystemeVentilation::CentraleTraitementAir => r.erreur(
            cs.clone(),
            None,
            "centrale de traitement d'air : réservée au non résidentiel (Th-BCE 2020 §6.2.3.1.2 \
             et §6.3.3.1.3)",
        ),
    }
    if let Some(p) = v.puits {
        r.erreur(
            format!("{c}.puits"),
            Some(format!("{p:?}")),
            format!("puits climatique ou hydraulique {HORS_PERIMETRE_VENTILATION}"),
        );
    }

    // Réseau.
    let cr = format!("{c}.reseau");
    r.plage(format!("{cr}.b"), v.reseau.b, 0.0, 1.0);
    if let Some(x) = v.reseau.ratfuitevc {
        r.plage(format!("{cr}.ratfuitevc"), x, 0.0, 1.0);
    }
    for (champ, requis, valeur) in [
        ("r_rep_m2k_w", a_reprise, v.reseau.r_rep_m2k_w),
        ("r_souf_m2k_w", a_soufflage, v.reseau.r_souf_m2k_w),
    ] {
        match valeur {
            Some(x) => r.positif(format!("{cr}.{champ}"), x),
            None if requis => r.erreur(
                format!("{cr}.{champ}"),
                None,
                "résistance thermique des conduits hors volume chauffé requise (Th-BCE 2020 \
                 §6.5.2 et §6.3.2)",
            ),
            None => {}
        }
    }
    // §6.2.3.4 : surfaces de conduits saisies pour toutes les bouches (Is_surf_cond_def = 0),
    // avec le ratio des fuites en volume chauffé, ou pour aucune.
    let nb = bouches().count();
    let nb_saisies = bouches().filter(|b| b.a_cond_m2.is_some()).count();
    if nb_saisies > 0 && nb_saisies < nb {
        r.erreur(
            format!("{c}.reseau"),
            None,
            "les surfaces de conduits (a_cond_m2) se saisissent pour toutes les bouches de la \
             zone, ou pour aucune (Th-BCE 2020 §6.2.3.4, Is_surf_cond_def)",
        );
    }
    if nb_saisies == nb && nb > 0 && v.reseau.ratfuitevc.is_none() {
        r.erreur(
            format!("{cr}.ratfuitevc"),
            None,
            "ratio des fuites en volume chauffé requis quand les surfaces de conduits sont \
             saisies (Th-BCE 2020 §6.2.3.4)",
        );
    }
}
