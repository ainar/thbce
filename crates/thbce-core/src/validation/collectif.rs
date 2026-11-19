// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation des postes propres au logement collectif et aux parkings (Th-BCE 2020 §10.1,
//! §10.3 et §10.4) : plages des nomenclatures et références aux zones.

use std::collections::BTreeMap;

use crate::entree::{Batiment, EtatFermeture, PlageHoraire, TechMac, UsageZone};

use super::RapportValidation;

/// Valide les ascenseurs et les parkings d'un bâtiment.
pub(super) fn valider_collectif(r: &mut RapportValidation, cb: &str, bat: &Batiment) {
    let zones: BTreeMap<&str, UsageZone> =
        bat.zones.iter().map(|z| (z.id.as_str(), z.usage)).collect();
    valider_escalators(r, cb, bat, &zones);
    r.ids_uniques(
        &format!("{cb}.ascenseurs"),
        bat.ascenseurs.iter().map(|a| &a.id),
    );
    for (i, a) in bat.ascenseurs.iter().enumerate() {
        let c = format!("{cb}.ascenseurs[{i}]");
        // Tableau 309 : Q ≥ 75 kg, 0 < V ≤ 2,5 m/s, H > 0, netage ≥ 2, Cp de −1,2 à 0,5.
        r.plage(format!("{c}.q_kg"), a.q_kg, 75.0, f64::INFINITY);
        r.plage(format!("{c}.v_m_s"), a.v_m_s, 0.0, 2.5);
        if a.v_m_s == 0.0 {
            r.erreur(
                format!("{c}.v_m_s"),
                Some("0".into()),
                "doit être strictement positif",
            );
        }
        r.strictement_positif(format!("{c}.h_m"), a.h_m);
        if a.netage < 2 {
            r.erreur(
                format!("{c}.netage"),
                Some(a.netage.to_string()),
                "doit être supérieur ou égal à 2 (tableau 309)",
            );
        }
        if let Some(cp) = a.cp {
            r.plage(format!("{c}.cp"), cp, -1.2, 0.5);
            // Tableau 314, note : −1,2 est obligatoire, et non demandé, pour un hydraulique.
            if a.tech_mac == TechMac::Hydraulique && (cp + 1.2).abs() > 1e-12 {
                r.erreur(
                    format!("{c}.cp"),
                    Some(cp.to_string()),
                    "vaut −1,2 pour un ascenseur hydraulique (tableau 314) : ne pas le saisir",
                );
            }
        }
        for (nom, x) in [
            ("pti_w", a.pti_w),
            ("dp1_w", a.dp1_w),
            ("t1_s", a.t1_s),
            ("dp2_w", a.dp2_w),
            ("t2_s", a.t2_s),
        ] {
            if let Some(x) = x {
                r.positif(format!("{c}.{nom}"), x);
            }
        }
        if !a.sc_veille
            && [a.dp1_w, a.t1_s, a.dp2_w, a.t2_s]
                .iter()
                .any(Option::is_some)
        {
            r.avertissement(
                format!("{c}.sc_veille"),
                Some("false".into()),
                "dp1_w, t1_s, dp2_w et t2_s ne sont employés que si sc_veille est vrai : ignorés",
            );
        }
        if let (Some(pti), Some(dp1), Some(dp2)) = (a.pti_w, a.dp1_w, a.dp2_w) {
            if a.sc_veille && dp1 + dp2 > pti {
                r.erreur(
                    format!("{c}.dp1_w"),
                    Some(dp1.to_string()),
                    "dP1 + dP2 ne peut pas dépasser Pti",
                );
            }
        }
        if a.zones_desservies.is_empty() {
            r.erreur(
                format!("{c}.zones_desservies"),
                None,
                "au moins une zone desservie (tableau 309 : C ≥ 1)",
            );
        }
        for (iz, id) in a.zones_desservies.iter().enumerate() {
            match zones.get(id.as_str()) {
                None => r.erreur(
                    format!("{c}.zones_desservies[{iz}]"),
                    Some(id.clone()),
                    "zone inconnue dans ce bâtiment",
                ),
                // Tableau 310 : le besoin de voyages Bv n'est défini que pour les usages 2 et
                // suivants ; les ascenseurs ne se calculent que dans les zones de logement
                // collectif.
                // Usage non résidentiel calculé : ascenseurs reconnus mais pas encore calculés
                // (besoin de voyages et nombre d'occupants de la zone hors habitation).
                Some(u) if !u.est_habitation() && u.est_pris_en_charge() => r.hors_perimetre(
                    format!("{c}.zones_desservies[{iz}]"),
                    Some(id.clone()),
                    "ascenseur desservant une zone d'usage non résidentiel (Th-BCE 2020 §10.1)",
                ),
                Some(u) if *u != UsageZone::LogementCollectif => r.erreur(
                    format!("{c}.zones_desservies[{iz}]"),
                    Some(id.clone()),
                    "les ascenseurs ne sont calculés que pour les zones de logement collectif \
                     (tableau 310 : pas de besoin de voyages pour la maison individuelle)",
                ),
                _ => {}
            }
        }
        let mut vues = std::collections::BTreeSet::new();
        for id in &a.zones_desservies {
            if !vues.insert(id) {
                r.erreur(
                    format!("{c}.zones_desservies"),
                    Some(id.clone()),
                    "zone listée deux fois",
                );
            }
        }
    }

    r.ids_uniques(
        &format!("{cb}.parkings"),
        bat.parkings.iter().map(|p| &p.id),
    );
    // §10.3.2 : « Typeusage » du parking (bureau, commerce, habitat). Seul l'habitat est
    // calculé : un parking d'un bâtiment qui a une zone non résidentielle calculée est refusé.
    if !bat.parkings.is_empty()
        && bat
            .zones
            .iter()
            .any(|z| !z.usage.est_habitation() && z.usage.est_pris_en_charge())
    {
        r.hors_perimetre(
            format!("{cb}.parkings"),
            None,
            "parking d'un bâtiment d'usage non résidentiel (Th-BCE 2020 §10.3 et §10.4)",
        );
    }
    for (i, p) in bat.parkings.iter().enumerate() {
        let c = format!("{cb}.parkings[{i}]");
        // §10.3.3 : Npl peut être un réel ; minimum 1 (tableaux 320 et 322).
        r.plage(format!("{c}.npl"), p.npl, 1.0, f64::INFINITY);
        if p.net < 1 {
            r.erreur(
                format!("{c}.net"),
                Some(p.net.to_string()),
                "doit être supérieur ou égal à 1",
            );
        }
        if let Some(v) = &p.ventilation {
            if let Some(x) = v.p_vent600_w_par_place {
                r.plage(
                    format!("{c}.ventilation.p_vent600_w_par_place"),
                    x,
                    1.0,
                    f64::INFINITY,
                );
            }
            if p.type_parking == crate::entree::TypeParking::Exterieur {
                r.avertissement(
                    format!("{c}.ventilation"),
                    None,
                    "un parking extérieur n'a pas de ventilation forcée (éq. 2333) : ignorée",
                );
            }
        }
        if let Some(x) = p.eclairage.p_installee_w {
            r.plage(
                format!("{c}.eclairage.p_installee_w"),
                x,
                1.0,
                f64::INFINITY,
            );
        }
        for (nom, plages) in [
            (
                "plages_detection_semaine",
                &p.eclairage.plages_detection_semaine,
            ),
            (
                "plages_detection_week_end",
                &p.eclairage.plages_detection_week_end,
            ),
        ] {
            if plages.len() > 3 {
                r.erreur(
                    format!("{c}.eclairage.{nom}"),
                    Some(plages.len().to_string()),
                    "au plus trois plages (tableau 322)",
                );
            }
            for (k, pl) in plages.iter().enumerate() {
                valider_plage(r, &format!("{c}.eclairage.{nom}[{k}]"), *pl);
            }
        }
        for j in &p.jours_fermes {
            if *j == 0 || *j > 365 {
                r.erreur(
                    format!("{c}.jours_fermes"),
                    Some(j.to_string()),
                    "numéro de jour de 1 à 365",
                );
            }
        }
        if !p.jours_fermes.is_empty() && p.eclairage.extinction_en_fermeture.is_none() {
            r.erreur(
                format!("{c}.eclairage.extinction_en_fermeture"),
                None,
                "requis quand le parking a des jours de fermeture (« Ex », tableau 322)",
            );
        }
    }
}

fn valider_plage(r: &mut RapportValidation, c: &str, p: PlageHoraire) {
    if p.debut_h > 24 || p.fin_h > 24 || p.debut_h > p.fin_h {
        r.erreur(
            c.to_owned(),
            Some(format!("{} h – {} h", p.debut_h, p.fin_h)),
            "heures légales de 0 à 24, début au plus égal à la fin",
        );
    }
}

/// Valide les escalators et tapis roulants (Th-BCE 2020 §10.2, tableau 318 ; Q115).
fn valider_escalators(
    r: &mut RapportValidation,
    cb: &str,
    bat: &Batiment,
    zones: &BTreeMap<&str, UsageZone>,
) {
    r.ids_uniques(
        &format!("{cb}.escalators"),
        bat.escalators.iter().map(|e| &e.id),
    );
    for (i, e) in bat.escalators.iter().enumerate() {
        let c = format!("{cb}.escalators[{i}]");
        if let Some(v) = e.v_nom_m_s {
            r.plage(format!("{c}.v_nom_m_s"), v, 0.0, 2.5);
            if v == 0.0 {
                r.erreur(
                    format!("{c}.v_nom_m_s"),
                    Some("0".into()),
                    "doit être strictement positif",
                );
            }
        }
        r.positif(format!("{c}.h_m"), e.h_m);
        r.positif(format!("{c}.p_noload_w"), e.p_noload_w);
        for (nom, x) in [
            ("p_slowspeed_w", e.p_slowspeed_w),
            ("p_standby_w", e.p_standby_w),
            ("p_autostart_w", e.p_autostart_w),
        ] {
            if let Some(x) = x {
                r.positif(format!("{c}.{nom}"), x);
            }
        }
        if e.slowspeed && e.p_slowspeed_w.is_none() {
            r.erreur(
                format!("{c}.p_slowspeed_w"),
                None,
                "requis avec une régulation basse vitesse (§10.2.3.3 : pas de valeur par défaut)",
            );
        }
        if !e.slowspeed && e.p_slowspeed_w.is_some() {
            r.avertissement(
                format!("{c}.p_slowspeed_w"),
                None,
                "n'est employé qu'avec `slowspeed` vrai : ignoré",
            );
        }
        if e.sc_veille {
            let (champ, valeur) = match e.etat_fermeture.unwrap_or(EtatFermeture::ArretComplet) {
                EtatFermeture::ArretComplet => ("p_standby_w", e.p_standby_w),
                EtatFermeture::ArretSansPassager => ("p_autostart_w", e.p_autostart_w),
                EtatFermeture::VitesseNominaleAVide => ("p_noload_w", Some(e.p_noload_w)),
            };
            if valeur.is_none() {
                r.erreur(
                    format!("{c}.{champ}"),
                    None,
                    "requis par l'état de fermeture choisi quand le scénario de veille est \
                     connu (§10.2.3.3 : pas de valeur par défaut)",
                );
            }
        } else if e.etat_fermeture.is_some() {
            r.avertissement(
                format!("{c}.etat_fermeture"),
                None,
                "n'est employé que si `sc_veille` est vrai : ignoré (88 W, éq. 2323)",
            );
        }
        if e.zones_desservies.is_empty() {
            r.erreur(
                format!("{c}.zones_desservies"),
                None,
                "au moins une zone desservie (tableau 318 : C ≥ 1)",
            );
        }
        let mut vues = std::collections::BTreeSet::new();
        for (iz, id) in e.zones_desservies.iter().enumerate() {
            match zones.get(id.as_str()) {
                None => r.erreur(
                    format!("{c}.zones_desservies[{iz}]"),
                    Some(id.clone()),
                    "zone inconnue dans ce bâtiment",
                ),
                Some(UsageZone::MaisonIndividuelle) => r.erreur(
                    format!("{c}.zones_desservies[{iz}]"),
                    Some(id.clone()),
                    "pas de besoin de voyages pour la maison individuelle (tableau 310)",
                ),
                _ => {}
            }
            if !vues.insert(id) {
                r.erreur(
                    format!("{c}.zones_desservies"),
                    Some(id.clone()),
                    "zone listée deux fois",
                );
            }
        }
    }
}
