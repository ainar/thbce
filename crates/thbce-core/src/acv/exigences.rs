// SPDX-License-Identifier: GPL-3.0-or-later
//! Exigence `Icconstruction_max` et ses coefficients de modulation : annexe de l'article
//! R. 172-4 du CCH, chapitre II, III et chapitre III, III
//! (`docs/reference/arrete/chapitre1a3_annexe_r172-4_post-rivaton-i.pdf`, p. 60 à 81, relues
//! sur l'image ; version post-GTM2 pour les permis déposés avant le 1er juillet 2026 : sans
//! `Miagrément_ext`, `MiHSP`, `Miclim_RCU` ni les dispositions propres aux immeubles de grande
//! hauteur, `docs/questions/Q213.md`).
//!
//! `Icconstruction_max = Icconstruction_maxmoyen × (1 + Micombles + Misurf_moy + Misurf_tot) +
//! Migéo + Miinfra + Mivrd + Mipv + Mided + Miagrément_ext + MiHSP + Miclim_RCU`.
//!
//! Usages couverts : ceux que le moteur calcule (1 à 7 et 12).

use thbce_data::ZoneClimatique;

use crate::entree::{ParametresExigences, ParametresExigencesAcv};
use crate::resultats::acv::ExigencesIcConstructionZone;

/// Modulation `Misurf_moy`.
#[derive(Debug, Clone, Copy)]
enum SurfMoy {
    /// Maisons : `0,36 − 3,6 × Smoylgt/1000` jusqu'à 120 m², −0,072 au-delà.
    Maison,
    /// Logements collectifs : `(100 − 2,5 × Smoylgt)/Icconstruction_maxmoyen` jusqu'à 40 m².
    Collectif,
    /// Nulle.
    Nulle,
}

/// Données d'un usage.
#[derive(Debug, Clone, Copy)]
struct DonneesIc {
    /// `Icconstruction_maxmoyen` : 2022 à 2024, 2025 à 2027, 2028 à 2030, à partir de 2031 ;
    /// `None` pour une colonne « - ».
    moyen: [Option<f64>; 4],
    surf_moy: SurfMoy,
    /// `Misurf_tot` par morceaux : `(Sref_max, a, b)` → `a + b × Sref`, le premier morceau
    /// dont la borne n'est pas dépassée ; au-delà du dernier, la valeur du dernier à sa borne.
    surf_tot: &'static [(f64, f64, f64)],
    /// `Misurf_tot` des immeubles de grande hauteur (version post-Rivaton).
    surf_tot_igh: f64,
    /// `Miagrément_ext` des logements collectifs.
    agrement: bool,
    /// Coefficient de `MiHSP` (95 en habitation, 0 sinon).
    k_hsp: f64,
    /// `Migéo` en H2d et H3 sous 400 m, en kg éq. CO2/m².
    geo: f64,
    /// Seuils de `Miinfra`, `Mivrd`, `Mipv` et `Mided`, en kg éq. CO2/m².
    seuil_infra: f64,
    seuil_vrd: f64,
    seuil_ded: f64,
}

const MAISON: DonneesIc = DonneesIc {
    moyen: [Some(640.0), Some(530.0), Some(475.0), Some(415.0)],
    surf_moy: SurfMoy::Maison,
    surf_tot: &[],
    surf_tot_igh: 0.0,
    agrement: false,
    k_hsp: 95.0,
    geo: 30.0,
    seuil_infra: 40.0,
    seuil_vrd: 20.0,
    seuil_ded: 370.0,
};
const COLLECTIF: DonneesIc = DonneesIc {
    moyen: [Some(740.0), Some(650.0), Some(580.0), Some(490.0)],
    surf_moy: SurfMoy::Collectif,
    surf_tot: &[(1300.0, -0.104, 0.8e-4), (4000.0, 0.0455, -0.350e-4)],
    surf_tot_igh: 0.0,
    agrement: true,
    k_hsp: 95.0,
    geo: 30.0,
    seuil_infra: 40.0,
    seuil_vrd: 10.0,
    seuil_ded: 250.0,
};
const BUREAUX: DonneesIc = DonneesIc {
    moyen: [Some(980.0), Some(810.0), Some(710.0), Some(600.0)],
    surf_moy: SurfMoy::Nulle,
    surf_tot: &[(2500.0, 0.034, -0.86e-4)],
    surf_tot_igh: -0.05,
    agrement: false,
    k_hsp: 0.0,
    geo: 50.0,
    seuil_infra: 40.0,
    seuil_vrd: 10.0,
    seuil_ded: 275.0,
};
const ENSEIGNEMENT: DonneesIc = DonneesIc {
    moyen: [Some(900.0), Some(770.0), Some(680.0), Some(590.0)],
    surf_moy: SurfMoy::Nulle,
    surf_tot: &[(10000.0, 0.084, -0.21e-4)],
    surf_tot_igh: -0.05,
    agrement: false,
    k_hsp: 0.0,
    geo: 0.0,
    seuil_infra: 60.0,
    seuil_vrd: 20.0,
    seuil_ded: 300.0,
};
const MEDIATHEQUES: DonneesIc = DonneesIc {
    moyen: [None, Some(940.0), Some(785.0), Some(630.0)],
    surf_moy: SurfMoy::Nulle,
    surf_tot: &[],
    surf_tot_igh: 0.05,
    agrement: false,
    k_hsp: 0.0,
    geo: 30.0,
    seuil_infra: 60.0,
    seuil_vrd: 20.0,
    seuil_ded: 440.0,
};
const UNIVERSITAIRES: DonneesIc = DonneesIc {
    moyen: [None, Some(940.0), Some(790.0), Some(640.0)],
    seuil_ded: 320.0,
    ..MEDIATHEQUES
};
const PETITE_ENFANCE: DonneesIc = DonneesIc {
    moyen: [None, Some(950.0), Some(780.0), Some(630.0)],
    geo: 0.0,
    seuil_ded: 530.0,
    ..MEDIATHEQUES
};

fn donnees(usage: u8) -> Option<&'static DonneesIc> {
    match usage {
        1 => Some(&MAISON),
        2 => Some(&COLLECTIF),
        3 => Some(&BUREAUX),
        4 | 5 => Some(&ENSEIGNEMENT),
        6 => Some(&MEDIATHEQUES),
        7 => Some(&UNIVERSITAIRES),
        12 => Some(&PETITE_ENFANCE),
        _ => None,
    }
}

/// Indicateurs du bâtiment qui modulent l'exigence, en kg éq. CO2/m².
#[derive(Debug, Clone, Copy, Default)]
pub struct IndicateursModulation {
    /// « Iclot1 ».
    pub ic_lot1: f64,
    /// « Iclot2 ».
    pub ic_lot2: f64,
    /// « Iclot13 ».
    pub ic_lot13: f64,
    /// « Icded » (lots 1, 2 et 13 exclus).
    pub ic_ded: f64,
}

/// Zone pour la fixation de l'exigence.
#[derive(Debug, Clone, Copy)]
pub struct ZoneIc<'a> {
    /// Identifiant de la zone.
    pub id: &'a str,
    /// Numéro d'usage.
    pub usage: u8,
    /// `Sref` de la zone, en m².
    pub sref_m2: f64,
    /// Nombre de logements.
    pub nb_logements: u32,
    /// Somme des `Sref` des zones de même usage (`Misurf_tot`, comme `Mbsurf_tot` : Q67).
    pub sref_usage_m2: f64,
    /// Bâtiment raccordé à un réseau de chaleur urbain.
    pub raccorde_rcu: bool,
}

/// `Icconstruction_maxmoyen` de l'usage pour l'année de dépôt ; immeuble de grande hauteur
/// (version post-Rivaton) : 95 % de la troisième colonne en 2028 à 2030, 95 % de cette valeur à
/// partir de 2031 (`docs/questions/Q214.md`).
fn ic_moyen(d: &DonneesIc, p: &ParametresExigences, igh: bool) -> Option<f64> {
    let col = match p.annee_depot_permis {
        ..=2024 => 0,
        2025..=2027 => 1,
        2028..=2030 => 2,
        _ => 3,
    };
    if igh && col >= 2 {
        let v2028 = 0.95 * d.moyen[2]?;
        return Some(if col == 2 { v2028 } else { 0.95 * v2028 });
    }
    d.moyen[col]
}

/// Exigence `Icconstruction_max` d'une zone ; `None` pour un usage non couvert ou une année où
/// la RE2020 ne vise pas l'usage.
pub fn exigence_zone(
    z: &ZoneIc,
    p: &ParametresExigences,
    pa: &ParametresExigencesAcv,
    zone: ZoneClimatique,
    alt_m: f64,
    ind: IndicateursModulation,
) -> Option<ExigencesIcConstructionZone> {
    let d = donnees(z.usage)?;
    let rivaton = p.depot_a_partir_de(2026, 7);
    let igh = rivaton && pa.immeuble_grande_hauteur;
    let moyen = ic_moyen(d, p, igh)?;
    let sref = z.sref_m2;
    let smoy = if z.nb_logements > 0 {
        sref / f64::from(z.nb_logements)
    } else {
        sref
    };
    let mi_combles = if z.usage == 1 && sref > 0.0 {
        0.4 * p.s_combles_m2 / sref
    } else {
        0.0
    };
    let mi_surf_moy = match d.surf_moy {
        SurfMoy::Maison if smoy <= 120.0 => 0.36 - 3.6 * smoy / 1000.0,
        SurfMoy::Maison => -0.072,
        SurfMoy::Collectif if smoy <= 40.0 => (100.0 - 2.5 * smoy) / moyen,
        SurfMoy::Collectif | SurfMoy::Nulle => 0.0,
    };
    let mi_surf_tot = if igh {
        d.surf_tot_igh
    } else {
        let s = z.sref_usage_m2;
        match d.surf_tot.iter().find(|(max, _, _)| s <= *max) {
            Some((_, a, b)) => a + b * s,
            None => d.surf_tot.last().map_or(0.0, |(max, a, b)| a + b * max),
        }
    };
    let chaud = matches!(zone, ZoneClimatique::H2d | ZoneClimatique::H3);
    let mi_geo = if chaud && alt_m < 400.0 { d.geo } else { 0.0 };
    let au_dela = |x: f64, seuil: f64| if x > seuil { x - seuil } else { 0.0 };
    let mi_infra = au_dela(ind.ic_lot2, d.seuil_infra);
    let mi_vrd = au_dela(ind.ic_lot1, d.seuil_vrd);
    let mi_pv = au_dela(ind.ic_lot13, 20.0);
    let ecart_ded = au_dela(ind.ic_ded, d.seuil_ded);
    let mi_ded = match p.annee_depot_permis {
        ..=2024 => 0.3 * ecart_ded,
        2025..=2027 => 0.0,
        _ => -0.3 * ecart_ded,
    };
    let (mut mi_agrement_ext, mut mi_hsp, mut mi_clim_rcu) = (0.0, 0.0, 0.0);
    if rivaton {
        let r = if sref > 0.0 {
            pa.s_agrement_ext_m2 / sref
        } else {
            0.0
        };
        if d.agrement && r > 0.1 {
            mi_agrement_ext = (r.min(0.25) - 0.1) * 110.0;
        }
        if let Some(h) = p.hsp_moy_m {
            if h > 2.5 {
                mi_hsp = (h.min(2.9) / 2.5 - 1.0) * d.k_hsp;
            }
        }
        if z.raccorde_rcu && p.reseau_chaleur_classe && pa.climatisation {
            mi_clim_rcu = 25.0;
        }
    }
    let max = moyen * (1.0 + mi_combles + mi_surf_moy + mi_surf_tot)
        + mi_geo
        + mi_infra
        + mi_vrd
        + mi_pv
        + mi_ded
        + mi_agrement_ext
        + mi_hsp
        + mi_clim_rcu;
    Some(ExigencesIcConstructionZone {
        zone: z.id.to_owned(),
        usage: z.usage,
        sref_m2: sref,
        ic_construction_max_moyen: moyen,
        mi_combles,
        mi_surf_moy,
        mi_surf_tot,
        mi_geo,
        mi_infra,
        mi_vrd,
        mi_pv,
        mi_ded,
        mi_agrement_ext,
        mi_hsp,
        mi_clim_rcu,
        ic_construction_max_kgco2_m2: max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(annee: u16, mois: u8) -> ParametresExigences {
        serde_json::from_value(serde_json::json!({
            "categorie_contraintes_exterieures": "categorie_1",
            "zone_bruit": "br1",
            "annee_depot_permis": annee,
            "mois_depot_permis": mois,
            "hsp_moy_m": 2.7,
        }))
        .unwrap()
    }

    fn proche(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} ≠ {b}");
    }

    #[test]
    fn maison_2025() {
        let z = ZoneIc {
            id: "z",
            usage: 1,
            sref_m2: 100.0,
            nb_logements: 1,
            sref_usage_m2: 100.0,
            raccorde_rcu: false,
        };
        let ind = IndicateursModulation {
            ic_lot1: 25.0,
            ic_lot2: 50.0,
            ic_lot13: 10.0,
            ic_ded: 400.0,
        };
        let pa = ParametresExigencesAcv::default();
        // Permis de mars 2026 (version post-GTM2) : 530 × (1 + 0 + 0,36 − 0,36 + 0) + 30 (H3,
        // < 400 m) + 10 (Miinfra) + 5 (Mivrd) + 0 + 0 (Mided en 2025 à 2027) = 575.
        let x = exigence_zone(&z, &params(2026, 3), &pa, ZoneClimatique::H3, 100.0, ind).unwrap();
        proche(x.mi_surf_moy, 0.0);
        proche(x.ic_construction_max_kgco2_m2, 575.0);
        // Permis de juillet 2026 : MiHSP = (2,7/2,5 − 1) × 95 = 7,6.
        let x = exigence_zone(&z, &params(2026, 7), &pa, ZoneClimatique::H3, 100.0, ind).unwrap();
        proche(x.mi_hsp, 7.6);
        proche(x.ic_construction_max_kgco2_m2, 582.6);
        // 2028, H1a : 475 × 1 + 10 + 5 − 0,3 × 30 + 7,6 = 488,6.
        let x = exigence_zone(&z, &params(2028, 1), &pa, ZoneClimatique::H1a, 100.0, ind).unwrap();
        proche(x.ic_construction_max_kgco2_m2, 488.6);
    }

    #[test]
    fn collectif_et_bureaux() {
        let pa = ParametresExigencesAcv {
            s_agrement_ext_m2: 300.0,
            immeuble_grande_hauteur: false,
            climatisation: false,
        };
        let z = ZoneIc {
            id: "z",
            usage: 2,
            sref_m2: 1000.0,
            nb_logements: 30,
            sref_usage_m2: 1000.0,
            raccorde_rcu: false,
        };
        let ind = IndicateursModulation::default();
        // Smoy = 33,3 m² : Misurf_moy = (100 − 83,33)/650 ; Misurf_tot = −0,104 + 0,08 =
        // −0,024 ; Miagrément = (0,25 − 0,1) × 110 = 16,5 ; MiHSP = 7,6.
        let x = exigence_zone(&z, &params(2026, 9), &pa, ZoneClimatique::H1a, 100.0, ind).unwrap();
        let attendu = 650.0 * (1.0 + (100.0 - 2.5 * 1000.0 / 30.0) / 650.0 - 0.024) + 16.5 + 7.6;
        proche(x.ic_construction_max_kgco2_m2, attendu);
        // Bureaux de 3 000 m² en 2025 : Misurf_tot = −0,181 (au-delà de 2 500 m²).
        let z = ZoneIc {
            usage: 3,
            sref_m2: 3000.0,
            nb_logements: 0,
            sref_usage_m2: 3000.0,
            ..z
        };
        let x = exigence_zone(&z, &params(2025, 9), &pa, ZoneClimatique::H2d, 100.0, ind).unwrap();
        proche(x.mi_surf_tot, -0.181);
        proche(x.ic_construction_max_kgco2_m2, 810.0 * (1.0 - 0.181) + 50.0);
    }
}
