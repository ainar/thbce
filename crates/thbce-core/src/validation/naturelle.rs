// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation de la ventilation naturelle par conduits et de la ventilation hybride (Th-BCE 2020
//! §6.6, tableau 65).

use super::RapportValidation;
use crate::entree::{
    AssistanceMecanique, BoucheNaturelle, ExtracteurStatique, Isouf, NaturelleParConduits,
    Ventilation, Zone,
};

/// Plage des vecteurs de régulation de l'assistance (§6.6.3.3 : `kmax` et `lmax` au plus 10).
const NB_REGULATION_MAX: usize = 10;

/// Contrôle le système naturel ou hybride de la zone : usage, bouches, conduit, bouche
/// d'extraction, extracteur et assistance mécanique.
pub(super) fn valider_naturelle(r: &mut RapportValidation, c: &str, v: &Ventilation, zone: &Zone) {
    let cs = format!("{c}.systeme");
    let Some(n) = v.systeme.conduit_naturel() else {
        return;
    };
    let variante = if v.systeme.assistance().is_some() {
        "hybride"
    } else {
        "naturelle_par_conduits"
    };
    let cn = if variante == "hybride" {
        format!("{cs}.hybride.naturelle")
    } else {
        format!("{cs}.naturelle_par_conduits")
    };
    if !zone.usage.est_habitation() {
        r.erreur(
            cs.clone(),
            None,
            "la ventilation naturelle par conduits et la ventilation hybride ne s'appliquent qu'aux \
             usages de maison individuelle ou accolée et de logements collectifs (Th-BCE 2020 \
             §6.6.1)",
        );
        return;
    }
    let bouches = || zone.groupes.iter().flat_map(|g| &g.bouches_conduits);
    if bouches().any(|b| b.isouf == Isouf::Soufflage) {
        r.erreur(
            cs.clone(),
            None,
            "un système naturel ou hybride n'a que des bouches de reprise (isouf = reprise) \
             (Th-BCE 2020 §6.6)",
        );
    }
    if bouches().next().is_none() {
        r.erreur(cs.clone(), None, "au moins une bouche de reprise requise");
    }
    // Un conduit équivalent dessert un seul groupe : le texte ne répartit pas le débit du
    // conduit entre les groupes (Q234 point 4).
    let nb_groupes_desservis = zone
        .groupes
        .iter()
        .filter(|g| !g.bouches_conduits.is_empty())
        .count();
    if nb_groupes_desservis > 1 {
        r.hors_perimetre(
            cs.clone(),
            None,
            "ventilation naturelle ou hybride desservant plusieurs groupes d'une zone (Th-BCE 2020 \
             §6.6.3)",
        );
    }
    valider_conduit(r, &cn, n);
    if let Some(a) = v.systeme.assistance() {
        valider_assistance(r, &format!("{cs}.hybride.assistance"), a);
    }
}

fn valider_conduit(r: &mut RapportValidation, c: &str, n: &NaturelleParConduits) {
    if n.nb_conduits_identiques == 0 {
        r.erreur(
            format!("{c}.nb_conduits_identiques"),
            Some("0".into()),
            "au moins un conduit (Th-BCE 2020 §6.6.2, tableau 65 : « Nid » au moins 1)",
        );
    }
    r.strictement_positif(format!("{c}.per_cond_m"), n.per_cond_m);
    r.strictement_positif(format!("{c}.h_cond_m"), n.h_cond_m);
    if let Some(h) = n.h_mot_m {
        r.positif(format!("{c}.h_mot_m"), h);
    }
    r.strictement_positif(format!("{c}.a_cond_m2"), n.a_cond_m2);
    if let Some(x) = n.lambda_cond {
        r.positif(format!("{c}.lambda_cond"), x);
    }
    if let Some(x) = n.chi_coude {
        r.positif(format!("{c}.chi_coude"), x);
    }
    if let Some(x) = n.d_pref_pa {
        r.strictement_positif(format!("{c}.d_pref_pa"), x);
    }
    match &n.bouche {
        BoucheNaturelle::Fixe {
            a_sect_base_cm2,
            a_sect_pointe_cm2,
        } => {
            r.positif(format!("{c}.bouche.a_sect_base_cm2"), *a_sect_base_cm2);
            r.positif(format!("{c}.bouche.a_sect_pointe_cm2"), *a_sect_pointe_cm2);
            if a_sect_base_cm2.max(*a_sect_pointe_cm2) <= 0.0 {
                r.erreur(
                    format!("{c}.bouche"),
                    None,
                    "au moins une des sections de la bouche fixe est strictement positive \
                     (Th-BCE 2020 §6.6.3.1.1, éq. 674)",
                );
            }
        }
        BoucheNaturelle::Autoreglable { d_p_pa, q_v_m3_h } => {
            for j in 0..6 {
                r.strictement_positif(format!("{c}.bouche.d_p_pa[{j}]"), d_p_pa[j]);
                r.strictement_positif(format!("{c}.bouche.q_v_m3_h[{j}]"), q_v_m3_h[j]);
            }
            let croissant = |t: &[f64; 6], strict: bool| {
                t.windows(2)
                    .all(|w| if strict { w[1] > w[0] } else { w[1] >= w[0] })
            };
            if !croissant(d_p_pa, true) {
                r.erreur(
                    format!("{c}.bouche.d_p_pa"),
                    None,
                    "les différences de pression de la courbe de la bouche autoréglable sont \
                     strictement croissantes (Th-BCE 2020 §6.6.3.1.2, tableau 66)",
                );
            }
            if !croissant(q_v_m3_h, false) {
                r.erreur(
                    format!("{c}.bouche.q_v_m3_h"),
                    None,
                    "les débits de la courbe de la bouche autoréglable sont croissants \
                     (Th-BCE 2020 §6.6.3.1.2, tableau 66)",
                );
            }
        }
    }
    valider_extracteur(r, &format!("{c}.extracteur"), &n.extracteur);
}

fn valider_extracteur(r: &mut RapportValidation, c: &str, e: &ExtracteurStatique) {
    for (j, x) in e.c_extr.iter().enumerate() {
        if !x.is_finite() {
            r.erreur(
                format!("{c}.c_extr[{j}]"),
                Some(x.to_string()),
                "valeur non finie",
            );
        }
    }
    r.positif(format!("{c}.xi_extr"), e.xi_extr);
    if let Some(x) = e.v_vent_ref_m_s {
        r.strictement_positif(format!("{c}.v_vent_ref_m_s"), x);
    }
}

fn valider_assistance(r: &mut RapportValidation, c: &str, a: &AssistanceMecanique) {
    r.plage(
        format!("{c}.theta_ext_lim_c"),
        a.theta_ext_lim_c,
        -40.0,
        f64::INFINITY,
    );
    r.positif(format!("{c}.v_vent_lim_m_s"), a.v_vent_lim_m_s);
    r.positif(format!("{c}.p_vent_pointe_w"), a.p_vent_pointe_w);
    let k = a.v_vent_reg_m_s.len();
    let l = a.theta_ext_reg_c.len();
    if !(1..=NB_REGULATION_MAX).contains(&k) {
        r.erreur(
            format!("{c}.v_vent_reg_m_s"),
            Some(k.to_string()),
            "de 1 à 10 vitesses de vent de régulation (Th-BCE 2020 §6.6.3.3, tableau 65)",
        );
    }
    if !(1..=NB_REGULATION_MAX).contains(&l) {
        r.erreur(
            format!("{c}.theta_ext_reg_c"),
            Some(l.to_string()),
            "de 1 à 10 températures de régulation (Th-BCE 2020 §6.6.3.3, tableau 65)",
        );
    }
    // §6.6.3.3 : 0 ≤ vreg,k ≤ vreg,k−1 ≤ vreg,1 < vlim et θext,reg,l ≥ θext,reg,l−1 ≥
    // θext,reg,1 > θlim.
    if a.v_vent_reg_m_s.iter().any(|v| *v < 0.0)
        || a.v_vent_reg_m_s.windows(2).any(|w| w[1] > w[0])
        || a.v_vent_reg_m_s
            .first()
            .is_some_and(|v| *v >= a.v_vent_lim_m_s)
    {
        r.erreur(
            format!("{c}.v_vent_reg_m_s"),
            None,
            "vitesses de vent de régulation positives, décroissantes et inférieures au seuil \
             (Th-BCE 2020 §6.6.3.3)",
        );
    }
    if a.theta_ext_reg_c.windows(2).any(|w| w[1] < w[0])
        || a.theta_ext_reg_c
            .first()
            .is_some_and(|t| *t <= a.theta_ext_lim_c)
    {
        r.erreur(
            format!("{c}.theta_ext_reg_c"),
            None,
            "températures de régulation croissantes et supérieures au seuil (Th-BCE 2020 \
             §6.6.3.3)",
        );
    }
    if a.p_vent_w.len() != k || a.p_vent_w.iter().any(|ligne| ligne.len() != l) {
        r.erreur(
            format!("{c}.p_vent_w"),
            None,
            "matrice de puissances : une ligne par vitesse de vent de régulation, une colonne par \
             température de régulation (Th-BCE 2020 §6.6.3.3, figure 78)",
        );
    }
    for (i, ligne) in a.p_vent_w.iter().enumerate() {
        for (j, p) in ligne.iter().enumerate() {
            r.positif(format!("{c}.p_vent_w[{i}][{j}]"), *p);
        }
    }
}
