// SPDX-License-Identifier: GPL-3.0-or-later
//! Pertes et consommations électriques récupérables : fiche `C_PER_Pertes et consommations
//! électriques récupérables` (Th-BCE 2020 §11.1).
//!
//! Les pertes et consommations des systèmes transmises au volume chauffé à un pas de temps sont
//! récupérées en partie par les groupes, et injectées dans leur modèle thermique au pas de
//! temps suivant, en parts convective et radiative (éq. 2355 à 2365).

/// « Partrecup_vent_loc » (tableau 324).
pub const PART_RECUP_VENT_LOC: f64 = 1.0;
/// « Partrecup_circ_chfr ».
pub const PART_RECUP_CIRC_CHFR: f64 = 0.6;
/// « Partrecup_circ_ECS ».
pub const PART_RECUP_CIRC_ECS: f64 = 0.6;
/// « Partrecup_dgr_chfr ».
pub const PART_RECUP_DGR_CHFR: f64 = 1.0;
/// « Partrecup_dintgr_chfr ».
pub const PART_RECUP_DINTGR_CHFR: f64 = 0.6;
/// « Partrecup_dist_ECS ».
pub const PART_RECUP_DIST_ECS: f64 = 0.6;
/// « Partrecup_gen ».
pub const PART_RECUP_GEN: f64 = 0.6;
/// « Partconv_vent_loc ».
pub const PART_CONV_VENT_LOC: f64 = 1.0;
/// « Partconv_autres ».
pub const PART_CONV_AUTRES: f64 = 0.5;

/// Flux d'un groupe transmis au volume chauffé à un pas de temps, en Wh, avant application des
/// parts récupérables. Les flux non localisés (réseaux intergroupes, générations) sont déjà
/// répartis entre les groupes au prorata de leurs surfaces (éq. 2360 à 2362).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FluxVolumeChauffe {
    /// « Φvent_loc_vc + Φth-BA-vc » : ventilateurs locaux (brasseurs d'air hors v1).
    pub vent_loc_wh: f64,
    /// « Σ Φaux_vc^ds » : circulateurs des réseaux du groupe.
    pub aux_dgr_wh: f64,
    /// « Σ Φpertes_vc^ds » : pertes des réseaux de chaud et de froid du groupe.
    pub pertes_dgr_wh: f64,
    /// « Σ Φpertes_vc_2nd-e » : pertes des distributions d'ECS du groupe.
    pub pertes_dgr_ecs_wh: f64,
    /// « Σ Ratsurf × Φaux_vc^dp » : circulateurs des réseaux intergroupes de chaud et de froid.
    pub aux_dintgr_wh: f64,
    /// « Σ Ratsurf × Φpertes_vc^dp » : pertes des réseaux intergroupes de chaud et de froid.
    pub pertes_dintgr_wh: f64,
    /// « Σ Ratsurface × Φaux_vc^dp-e » : auxiliaires des réseaux d'ECS intergroupes.
    pub aux_dintgr_ecs_wh: f64,
    /// « Σ Ratsurface × Φpertes_vc_prim-e » : pertes des réseaux d'ECS intergroupes.
    pub pertes_dintgr_ecs_wh: f64,
    /// « Σ Ratsurf × Φvc_tot^gen » : générations (nul pour une génération idéale).
    pub generation_wh: f64,
}

/// Flux récupérés par un groupe, en Wh : `(Φrecup_conv, Φrecup_rad)` (éq. 2355 à 2365).
///
/// Les pertes des réseaux de froid sont négatives (éq. 882 et 931) : elles sont récupérées de
/// même, comme des apports négatifs.
pub fn recuperation_wh(f: &FluxVolumeChauffe) -> (f64, f64) {
    // Éq. (2355) à (2362).
    let vent_loc = PART_RECUP_VENT_LOC * f.vent_loc_wh;
    let autres = PART_RECUP_DGR_CHFR * f.pertes_dgr_wh
        + PART_RECUP_DIST_ECS * f.pertes_dgr_ecs_wh
        + PART_RECUP_CIRC_CHFR * f.aux_dgr_wh
        + PART_RECUP_CIRC_CHFR * f.aux_dintgr_wh
        + PART_RECUP_DINTGR_CHFR * f.pertes_dintgr_wh
        + PART_RECUP_CIRC_ECS * f.aux_dintgr_ecs_wh
        + PART_RECUP_DIST_ECS * f.pertes_dintgr_ecs_wh
        + PART_RECUP_GEN * f.generation_wh;
    // Éq. (2363) à (2365).
    let total = vent_loc + autres;
    let conv = PART_CONV_VENT_LOC * vent_loc + PART_CONV_AUTRES * autres;
    (conv, total - conv)
}
