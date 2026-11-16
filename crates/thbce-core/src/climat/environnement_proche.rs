// SPDX-License-Identifier: GPL-3.0-or-later
//! Fiche `C_EEX_environnement_proche` (Th-BCE 2020 §3.2).
//!
//! Calcule le rayonnement solaire et l'éclairement naturel sur une paroi, compte tenu des
//! masques proches et lointains, ainsi que la correction locale du vent et le rayonnement froid
//! vers la voûte céleste. « Dans tous les calculs les angles sont en radians » (§3.2.1).
//!
//! Aucune fonction de ce module n'alloue : elles sont appelées dans la boucle horaire.

use libm::{acos, atan, cos, sin, tan};

use crate::entree::{Exposition, Masque};

/// « alb » : albédo du sol pour le rayonnement solaire, valeur conventionnelle (§3.2.2).
pub const ALB: f64 = 0.2;
/// « albecl » : albédo du sol pour l'éclairement naturel, valeur conventionnelle (§3.2.2).
pub const ALBECL: f64 = 0.2;
/// « Cvent » : correction locale de la vitesse du vent, valeur conventionnelle (§3.2.2).
pub const CVENT: f64 = 0.9;
/// « Hre » : coefficient d'échange radiatif extérieur, en W/(m².K) (§3.2.2, constantes).
pub const HRE_W_M2K: f64 = 5.5;
/// Hauteur du soleil, en radians, à partir de laquelle il fait jour (éq. 18).
pub const SEUIL_JOUR_ASTRO_RAD: f64 = 0.0001;
/// Valeur de `cos(Φ)` à partir de laquelle le soleil entre dans le plan du composant
/// (éq. 19, 20 et 22).
pub const SEUIL_COS_PHI: f64 = 1e-5;
/// Nombre de tranches azimutales d'un masque lointain (§3.2.3 : 36 tranches de 10°).
pub const NB_TRANCHES: usize = 36;
/// « T_simul_déb_veg » : début conventionnel de la saison végétative, en heure de simulation.
pub const T_SIMUL_DEB_VEG: usize = 2161;
/// « T_simul_fin_veg » : fin conventionnelle de la saison végétative, en heure de simulation.
pub const T_SIMUL_FIN_VEG: usize = 6553;

const PI: f64 = core::f64::consts::PI;
const DEMI_PI: f64 = core::f64::consts::FRAC_PI_2;

/// Position du soleil à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Soleil {
    /// « γ » : hauteur du soleil, en radians.
    pub gamma_rad: f64,
    /// « ψ » : azimut du soleil par rapport au sud, négatif au lever et positif au coucher, en
    /// radians. Le texte emploie aussi « ψ' », même angle compté de 0 à 360° ; les deux ne
    /// diffèrent que d'un tour et interviennent toujours par `ψ − α`.
    pub psi_rad: f64,
}

/// Paramètres d'intégration d'une surface exposée (§3.2.2), convertis en radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceExposee {
    /// « α » : azimut (0 : sud ; π/2 : ouest ; π : nord ; 3π/2 : est).
    pub alpha_rad: f64,
    /// « β » : inclinaison (0 : horizontale vers le haut ; π/2 : verticale ; π : vers le bas).
    pub beta_rad: f64,
    /// Vrai si le composant est vertical (« β = π/2 »), condition des masques proches. Déterminé
    /// sur la valeur saisie en degrés, pour ne pas dépendre d'un arrondi de conversion.
    pub verticale: bool,
    /// « lp » : largeur du composant, en m (masques proches verticaux, éq. 19).
    pub lp_m: f64,
    /// « hp » : hauteur du composant, en m (masque proche horizontal, éq. 20 et 21).
    pub hp_m: f64,
}

impl SurfaceExposee {
    /// Convertit les paramètres saisis (§3.2.3, « Conversion des angles en radians »).
    pub fn depuis_exposition(e: &Exposition) -> Self {
        #[allow(clippy::float_cmp)]
        let verticale = e.inclinaison_deg == 90.0;
        Self {
            alpha_rad: e.azimut_deg.to_radians(),
            beta_rad: if verticale {
                DEMI_PI
            } else {
                e.inclinaison_deg.to_radians()
            },
            verticale,
            lp_m: e.lpb_m.unwrap_or(0.0),
            hp_m: e.hpb_m.unwrap_or(0.0),
        }
    }
}

/// Masque prêt pour le calcul : angles en radians, tranches en tableau fixe.
#[derive(Debug, Clone, PartialEq)]
pub enum MasqueResolu {
    /// `vg` : masque proche vertical gauche.
    ProcheVerticalGauche {
        /// « dvg » : profondeur, en m.
        dvg_m: f64,
        /// « dpg » : distance à la paroi, en m.
        dpg_m: f64,
    },
    /// `vd` : masque proche vertical droit.
    ProcheVerticalDroit {
        /// « dvd » : profondeur, en m.
        dvd_m: f64,
        /// « dpd » : distance à la paroi, en m.
        dpd_m: f64,
    },
    /// `h` : masque proche horizontal.
    ProcheHorizontal {
        /// « dhm » : profondeur, en m.
        dhm_m: f64,
        /// « dhp » : distance à la paroi, en m.
        dhp_m: f64,
    },
    /// `E_LV` : masque lointain défini par un plan vertical.
    LointainPlanVertical {
        /// « dE » : distance au masque, en m.
        de_m: f64,
        /// « hpE » : hauteur du masque par rapport au centre de la paroi, en m.
        hpe_m: f64,
    },
    /// `E_AZ` : masque lointain par tranches azimutales.
    LointainAzimutal {
        /// « γi » par tranche, en radians.
        gamma_i_rad: [f64; NB_TRANCHES],
    },
    /// `E_Az_cad` : masque lointain d'arbres à feuilles caduques.
    LointainArbresCaducs {
        /// « γi » par tranche, en radians.
        gamma_i_rad: [f64; NB_TRANCHES],
    },
}

impl MasqueResolu {
    /// Convertit un masque saisi. La validation garantit 36 tranches ; une liste plus courte
    /// est complétée par des tranches sans masque.
    pub fn depuis_masque(masque: &Masque) -> Self {
        let tranches = |gamma_i_deg: &[f64]| {
            let mut t = [0.0; NB_TRANCHES];
            for (cible, g) in t.iter_mut().zip(gamma_i_deg) {
                *cible = g.to_radians();
            }
            t
        };
        match masque {
            Masque::LointainAzimutal { gamma_i_deg } => Self::LointainAzimutal {
                gamma_i_rad: tranches(gamma_i_deg),
            },
            Masque::LointainArbresCaducs { gamma_i_deg } => Self::LointainArbresCaducs {
                gamma_i_rad: tranches(gamma_i_deg),
            },
            Masque::LointainPlanVertical { de_m, hpe_m } => Self::LointainPlanVertical {
                de_m: *de_m,
                hpe_m: *hpe_m,
            },
            Masque::ProcheVerticalDroit { dvd_m, dpd_m } => Self::ProcheVerticalDroit {
                dvd_m: *dvd_m,
                dpd_m: *dpd_m,
            },
            Masque::ProcheVerticalGauche { dvg_m, dpg_m } => Self::ProcheVerticalGauche {
                dvg_m: *dvg_m,
                dpg_m: *dpg_m,
            },
            Masque::ProcheHorizontal { dhm_m, dhp_m } => Self::ProcheHorizontal {
                dhm_m: *dhm_m,
                dhp_m: *dhp_m,
            },
        }
    }
}

/// Ramène un angle dans [−π ; π[.
fn ramener(angle_rad: f64) -> f64 {
    let tour = 2.0 * PI;
    let a = (angle_rad + PI) % tour;
    (if a < 0.0 { a + tour } else { a }) - PI
}

/// `Φ = ψ − α` : azimut du soleil par rapport à la normale au composant, ramené dans [−π ; π[
/// (éq. 3 et 5 ; α et ψ « ramenés dans [−π ; π] »).
pub fn phi_rad(surface: &SurfaceExposee, soleil: &Soleil) -> f64 {
    ramener(soleil.psi_rad - surface.alpha_rad)
}

/// Hauteur de soleil corrigée (angle de profil) d'un composant d'enveloppe, `γ'k`, en degrés.
///
/// Th-BCE 2020 §3.2.3, éq. (4) : si `|Φk| ≥ π/2`, `γ'k = 90` ; sinon
/// `γ'k = (180/π)·arctan(tan(γ) / cos(Φk))`.
pub fn hauteur_soleil_corrigee_composant_deg(surface: &SurfaceExposee, soleil: &Soleil) -> f64 {
    let phi = phi_rad(surface, soleil);
    if phi.abs() >= DEMI_PI {
        90.0
    } else {
        atan(tan(soleil.gamma_rad) / cos(phi)).to_degrees()
    }
}

/// Angle entre le soleil et la normale à la paroi, `θ`, en radians.
///
/// Th-BCE 2020 §3.2.3, éq. (7) et (12) :
/// `θ = min[π/2 ; arccos(cos(γ)·sin(β)·cos(ψ' − α) + sin(γ)·cos(β))]`.
pub fn angle_incidence_rad(surface: &SurfaceExposee, soleil: &Soleil) -> f64 {
    let (gamma, beta) = (soleil.gamma_rad, surface.beta_rad);
    let cos_theta =
        cos(gamma) * sin(beta) * cos(soleil.psi_rad - surface.alpha_rad) + sin(gamma) * cos(beta);
    // Garde numérique : un arrondi peut porter l'argument hors de [−1 ; 1].
    acos(cos_theta.clamp(-1.0, 1.0)).min(DEMI_PI)
}

/// Rayonnements et éclairements sur le plan d'une paroi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluxParoi {
    /// « θ » : angle entre le soleil et la normale à la paroi, en radians.
    pub theta_rad: f64,
    /// « Drp » : rayonnement direct, en W/m².
    pub drp_w_m2: f64,
    /// « Dfp » : rayonnement diffus, en W/m².
    pub dfp_w_m2: f64,
    /// « Rrp » : rayonnement réfléchi par le sol, en W/m².
    pub rrp_w_m2: f64,
    /// « Erp » : éclairement direct, en lux.
    pub erp_lux: f64,
    /// « Efp » : éclairement diffus, en lux.
    pub efp_lux: f64,
    /// « Errp » : éclairement réfléchi par le sol, en lux.
    pub errp_lux: f64,
}

impl FluxParoi {
    /// Rayonnement total, en W/m² (somme des trois composantes, comme dans l'éq. 26).
    pub fn isr_w_m2(&self) -> f64 {
        self.drp_w_m2 + self.dfp_w_m2 + self.rrp_w_m2
    }

    /// « ETp* » : éclairement total, en lux (§3.2.3, « Éclairement atteignant une paroi »).
    pub fn etp_lux(&self) -> f64 {
        self.erp_lux + self.efp_lux + self.errp_lux
    }
}

/// Rayonnement et éclairement incidents, sans masque.
///
/// Th-BCE 2020 §3.2.3 :
/// - éq. (8) `Drp = cos(θ)·IDn` ; éq. (13) `Erp = cos(θ)·EDn` ;
/// - éq. (9) `Dfp = Idi·0,5·(1 + cos(β))` ; éq. (14) `Efp = Edi·0,5·(1 + cos(β))` ;
/// - éq. (10) `Rrp = (IDh + Idi)·alb·0,5·(1 − |cos(β)|)` avec éq. (11) `IDh = IDn·sin(γ)` ;
/// - éq. (15) `Errp = (EDh + Edi)·albecl·0,5·(1 − |cos(β)|)` avec éq. (16) `EDh = EDn·sin(γ)`.
///
/// Les formules s'appliquent telles quelles quand la hauteur du soleil est nulle et le
/// rayonnement direct non nul (lever et coucher, question Q19) : le texte ne prévoit pas de
/// traitement particulier.
pub fn flux_sans_masque(
    surface: &SurfaceExposee,
    soleil: &Soleil,
    idn_w_m2: f64,
    idi_w_m2: f64,
    edn_lux: f64,
    edi_lux: f64,
) -> FluxParoi {
    let theta_rad = angle_incidence_rad(surface, soleil);
    let cos_theta = cos(theta_rad);
    let cos_beta = cos(surface.beta_rad);
    let sin_gamma = sin(soleil.gamma_rad);
    let vue_du_ciel = 0.5 * (1.0 + cos_beta);
    let vue_du_sol = 0.5 * (1.0 - cos_beta.abs());
    FluxParoi {
        theta_rad,
        drp_w_m2: cos_theta * idn_w_m2,
        dfp_w_m2: idi_w_m2 * vue_du_ciel,
        rrp_w_m2: (idn_w_m2 * sin_gamma + idi_w_m2) * ALB * vue_du_sol,
        erp_lux: cos_theta * edn_lux,
        efp_lux: edi_lux * vue_du_ciel,
        errp_lux: (edn_lux * sin_gamma + edi_lux) * ALBECL * vue_du_sol,
    }
}

/// Facteurs d'affaiblissement d'un masque : rayonnement direct et rayonnement diffus. Ils valent
/// aussi pour l'éclairement naturel (§3.2.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affaiblissement {
    /// Facteur sur le rayonnement et l'éclairement directs.
    pub dir: f64,
    /// Facteur sur le rayonnement et l'éclairement diffus.
    pub dif: f64,
}

impl Affaiblissement {
    /// Aucun affaiblissement : « avant le calcul des masques tous les facteurs d'affaiblissement
    /// sont forcés à 1 » (§3.2.3).
    pub const SANS_EFFET: Self = Self { dir: 1.0, dif: 1.0 };
}

/// Indice (à partir de 0) de la tranche azimutale où se trouve le soleil.
///
/// Th-BCE 2020 §3.2.3, figure 10 : 36 tranches de 10° repérées par rapport à la normale au
/// composant ; la tranche 1 commence à l'opposé de la normale, les tranches 18 et 19 encadrent
/// la normale, et un soleil à 20–30° du côté des Φ positifs est dans la tranche 21.
pub fn tranche_azimutale(phi_rad: f64) -> usize {
    let i = ((ramener(phi_rad).to_degrees() + 180.0) / 10.0).floor();
    (i.max(0.0) as usize).min(NB_TRANCHES - 1)
}

fn affaiblissement_tranches(
    gamma_i_rad: &[f64; NB_TRANCHES],
    phi: f64,
    gamma: f64,
) -> Affaiblissement {
    // §3.2.3 : « si γ est supérieur à l'angle γi de cette tranche, le composant est éclairé
    // (F = 1), sinon il est à l'ombre (F = 0) » ; le diffus n'est pas affaibli.
    let dir = if gamma > gamma_i_rad[tranche_azimutale(phi)] {
        1.0
    } else {
        0.0
    };
    Affaiblissement { dir, dif: 1.0 }
}

/// Facteurs d'affaiblissement d'un masque au pas de temps `h`.
///
/// Th-BCE 2020 §3.2.3 : masques proches verticaux (éq. 19), masque proche horizontal (éq. 20 et
/// 21), masque lointain par plan vertical (éq. 22), masques lointains par tranches azimutales
/// et arbres à feuilles caduques.
pub fn affaiblissement(
    masque: &MasqueResolu,
    surface: &SurfaceExposee,
    soleil: &Soleil,
    h: usize,
) -> Affaiblissement {
    let phi = phi_rad(surface, soleil);
    let cos_phi = cos(phi);
    let soleil_dans_le_plan = cos_phi >= SEUIL_COS_PHI;
    let borne = |f: f64| f.clamp(0.0, 1.0);
    match *masque {
        // Éq. (19) : dlg = max(0 ; dvg·tg(Φ)) ; Fvg,dir = min[max(0 ; 1 − (dlg − dpg)/lp) ; 1].
        // Le diffus n'est pas affaibli (Fvg,dif = 1).
        MasqueResolu::ProcheVerticalGauche { dvg_m, dpg_m } => {
            if surface.verticale && soleil_dans_le_plan && dvg_m > 0.0 {
                let dlg = (dvg_m * tan(phi)).max(0.0);
                Affaiblissement {
                    dir: borne(1.0 - (dlg - dpg_m) / surface.lp_m),
                    dif: 1.0,
                }
            } else {
                Affaiblissement::SANS_EFFET
            }
        }
        // Éq. (19) : dld = max(0 ; −dvd·tg(Φ)) ; Fvd,dir = min[max(0 ; 1 − (dld − dpd)/lp) ; 1].
        MasqueResolu::ProcheVerticalDroit { dvd_m, dpd_m } => {
            if surface.verticale && soleil_dans_le_plan && dvd_m > 0.0 {
                let dld = (-dvd_m * tan(phi)).max(0.0);
                Affaiblissement {
                    dir: borne(1.0 - (dld - dpd_m) / surface.lp_m),
                    dif: 1.0,
                }
            } else {
                Affaiblissement::SANS_EFFET
            }
        }
        MasqueResolu::ProcheHorizontal { dhm_m, dhp_m } => {
            if !(surface.verticale && dhm_m > 0.0) {
                return Affaiblissement::SANS_EFFET;
            }
            // Éq. (20) : dh = max(0 ; dhm·tg(γ)/cos(Φ)) ; Fh,dir = min[max(0 ; 1 − (dh − dhp)/hp) ; 1].
            // Le texte écrit « dh − dhg » : dhg n'est pas défini, dhp est retenu (question Q21).
            let dir = if soleil_dans_le_plan {
                let dh = (dhm_m * tan(soleil.gamma_rad) / cos_phi).max(0.0);
                borne(1.0 - (dh - dhp_m) / surface.hp_m)
            } else {
                1.0
            };
            // Éq. (21) : θg = (180/π)·arctan((dhp + hp/2)/dhm) ; Fh,dif = θg/90.
            let theta_g_deg = atan((dhp_m + surface.hp_m / 2.0) / dhm_m).to_degrees();
            Affaiblissement {
                dir,
                dif: theta_g_deg / 90.0,
            }
        }
        // Éq. (22), pour toute inclinaison : dhE = dE·tg(γ)/cos(Φ) ; dhE > hpE ⇒ 1 ; dhE < hpE ⇒ 0.
        // Le cas d'égalité n'est pas prévu par le texte : le composant est tenu pour éclairé.
        MasqueResolu::LointainPlanVertical { de_m, hpe_m } => {
            if soleil_dans_le_plan && hpe_m > 0.0 {
                let dhe = de_m * tan(soleil.gamma_rad) / cos_phi;
                Affaiblissement {
                    dir: if dhe < hpe_m { 0.0 } else { 1.0 },
                    dif: 1.0,
                }
            } else {
                Affaiblissement::SANS_EFFET
            }
        }
        MasqueResolu::LointainAzimutal { ref gamma_i_rad } => {
            affaiblissement_tranches(gamma_i_rad, phi, soleil.gamma_rad)
        }
        // « Si Tsimul_deb_veg < Tsimul < Tsimul_fin_veg », comme le masque azimutal ; sinon 1.
        // Tsimul est pris égal à l'itération h (question Q23).
        MasqueResolu::LointainArbresCaducs { ref gamma_i_rad } => {
            if T_SIMUL_DEB_VEG < h && h < T_SIMUL_FIN_VEG {
                affaiblissement_tranches(gamma_i_rad, phi, soleil.gamma_rad)
            } else {
                Affaiblissement::SANS_EFFET
            }
        }
    }
}

/// Facteurs d'affaiblissement globaux d'un composant.
///
/// Th-BCE 2020 §3.2.3, éq. (23) et (24) : produit des facteurs des masques en œuvre, pour le
/// direct et pour le diffus. Éq. (25) : le réfléchi n'est pas affaibli (`FfRp = 1`).
pub fn affaiblissement_global(
    masques: &[MasqueResolu],
    surface: &SurfaceExposee,
    soleil: &Soleil,
    h: usize,
) -> Affaiblissement {
    masques.iter().fold(Affaiblissement::SANS_EFFET, |acc, m| {
        let f = affaiblissement(m, surface, soleil, h);
        Affaiblissement {
            dir: acc.dir * f.dir,
            dif: acc.dif * f.dif,
        }
    })
}

/// Rayonnement et éclairement atteignant une paroi, masques compris.
///
/// Th-BCE 2020 §3.2.3 : `Drp* = Drp·FfDir`, `Dfp* = Dfp·FfDiff`, `Rrp* = Rrp·FfRp`, et de même
/// pour `Erp*`, `Efp*`, `Errp*`.
#[allow(clippy::too_many_arguments)]
pub fn flux_atteignant_la_paroi(
    surface: &SurfaceExposee,
    masques: &[MasqueResolu],
    soleil: &Soleil,
    idn_w_m2: f64,
    idi_w_m2: f64,
    edn_lux: f64,
    edi_lux: f64,
    h: usize,
) -> FluxParoi {
    let brut = flux_sans_masque(surface, soleil, idn_w_m2, idi_w_m2, edn_lux, edi_lux);
    let ff = affaiblissement_global(masques, surface, soleil, h);
    FluxParoi {
        theta_rad: brut.theta_rad,
        drp_w_m2: brut.drp_w_m2 * ff.dir,
        dfp_w_m2: brut.dfp_w_m2 * ff.dif,
        rrp_w_m2: brut.rrp_w_m2,
        erp_lux: brut.erp_lux * ff.dir,
        efp_lux: brut.efp_lux * ff.dif,
        errp_lux: brut.errp_lux,
    }
}

/// « Ventc » : vitesse du vent corrigée à 10 m de hauteur, `Ventc = Vent × Cvent` (§3.2.3).
pub fn ventc_m_s(vent_m_s: f64) -> f64 {
    vent_m_s * CVENT
}

/// « Qer* » : densité de flux de chaleur vers la voûte céleste, en W/m².
///
/// Th-BCE 2020 §3.2.3, éq. (17) : `Qer* = hre·(Teciel − Te)·max(cos(β) ; 0)`.
pub fn qer_w_m2(surface: &SurfaceExposee, teciel_c: f64, te_c: f64) -> f64 {
    HRE_W_M2K * (teciel_c - te_c) * cos(surface.beta_rad).max(0.0)
}

/// « Jour_astro » : vrai si le soleil est au-dessus de l'horizon (éq. 18, `γ ≥ 0,0001`).
pub fn jour_astro(soleil: &Soleil) -> bool {
    soleil.gamma_rad >= SEUIL_JOUR_ASTRO_RAD
}
