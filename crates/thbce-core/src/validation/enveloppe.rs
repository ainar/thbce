// SPDX-License-Identifier: GPL-3.0-or-later
//! Validation de l'enveloppe hors baies : inertie, exposition et masques, parois opaques et
//! ponts thermiques.

use super::{RapportValidation, References, NB_MASQUES_MAX, NB_TRANCHES_AZIMUTALES};
use crate::bati::comportement_thermique::inertie_quotidienne;
use crate::bati::inerties_longues::{cma_surf_kj_m2k, cms_surf_kj_m2k};
use crate::entree::{
    Exposition, Inertie, InertieAnnuelle, InertieQuotidienne, InertieSequentielle, Masque,
    ParoiOpaque, ParoiType, PontThermique,
};

/// Th-BCE 2020 §5.21.2 : toutes les grandeurs d'inertie ont pour minimum 0.
pub(super) fn valider_inertie(r: &mut RapportValidation, c: &str, i: &Inertie) {
    if let InertieQuotidienne::Personnalisee {
        amq_surf_m2_m2,
        cmq_surf_kj_m2k,
    } = i.quotidienne
    {
        r.positif(format!("{c}.quotidienne.amq_surf_m2_m2"), amq_surf_m2_m2);
        r.positif(format!("{c}.quotidienne.cmq_surf_kj_m2k"), cmq_surf_kj_m2k);
    }
    if let InertieSequentielle::Personnalisee { cms_surf_kj_m2k } = i.sequentielle {
        r.positif(format!("{c}.sequentielle.cms_surf_kj_m2k"), cms_surf_kj_m2k);
    }
    if let InertieAnnuelle::Personnalisee { cma_surf_kj_m2k } = i.annuelle {
        r.positif(format!("{c}.annuelle.cma_surf_kj_m2k"), cma_surf_kj_m2k);
    }
    r.positif(format!("{c}.ams_surf_m2_m2"), i.ams_surf_m2_m2);
    r.positif(format!("{c}.ama_surf_m2_m2"), i.ama_surf_m2_m2);

    // Th-BCE 2020 §5.21.3.4 : « Par définition, ΔCm,s−q et ΔCm,a−s sont positives ou nulles »
    // (éq. 331 et 332) ; `ΔHg` est nul pour `ΔR` nul (éq. 329 et 330), et un `ΔR` négatif n'a
    // pas de sens. Une saisie qui y contrevient est signalée ; le calcul Th-D prend alors la
    // capacité ou la conductance nulle (`bati::inerties_longues`).
    let (amq_surf, cmq_surf) = inertie_quotidienne(&i.quotidienne);
    let cms_surf = cms_surf_kj_m2k(&i.sequentielle, cmq_surf);
    let cma_surf = cma_surf_kj_m2k(&i.annuelle, cms_surf);
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
pub(super) fn valider_exposition(
    r: &mut RapportValidation,
    c: &str,
    e: &Exposition,
    refs: &References<'_>,
) {
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

pub(super) fn valider_paroi(
    r: &mut RapportValidation,
    c: &str,
    p: &ParoiOpaque,
    refs: &References<'_>,
) {
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

/// Th-BCE 2020 §5.19.2.
pub(super) fn valider_pont_thermique(
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
pub(super) fn valider_paroi_type(r: &mut RapportValidation, c: &str, p: &ParoiType) {
    r.positif(format!("{c}.u_k_w_m2k"), p.u_k_w_m2k);
    r.plage(format!("{c}.sf_c_k"), p.sf_c_k, 0.0, 1.0);
    r.plage(format!("{c}.sf_e_k"), p.sf_e_k, 0.0, 1.0);
    if let Some(v) = &p.vegetalisation {
        r.plage(format!("{c}.vegetalisation.sf_c_v_k"), v.sf_c_v_k, 0.0, 1.0);
        r.plage(format!("{c}.vegetalisation.sf_e_v_k"), v.sf_e_v_k, 0.0, 1.0);
    }
}

/// Th-BCE 2020 §3.2.2.
pub(super) fn valider_masque(r: &mut RapportValidation, c: &str, m: &Masque) {
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
