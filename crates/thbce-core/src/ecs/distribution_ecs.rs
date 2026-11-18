// SPDX-License-Identifier: GPL-3.0-or-later
//! Distributions d'ECS du groupe et intergroupes : fiches `C_DIS_Distribution_ECS_du_groupe`
//! (Th-BCE 2020 §9.7) et `C_DIS_Distribution_ECS_intergroupe` (§9.8).

use crate::entree::{DistributionEcsGroupe, ReseauEcsIntergroupe, UsageZone};

use super::besoins_ecs::{C_W_WH_KG_K, RHO_W_KG_L, THETA_UW_C};
use super::recuperateur_eaux_grises::{RecuperateurResolu, SortiesRecuperateur};

/// « θ2nd-e » conventionnelle, en °C (tableau 278 de la fiche 9.7 ; le tableau 271 de la fiche
/// 9.5 écrit 48 °C : lecture Q45).
pub const THETA_2ND_E_CONV_C: f64 = 53.0;

/// « nbbouchons » en habitation (tableau 279 : 3 en maison et en logement collectif).
pub const NB_BOUCHONS_HABITATION: f64 = 3.0;

/// « nbbouchons » : nombre conventionnel de bouchons d'eau froide par heure de puisage d'une
/// distribution d'ECS du groupe, selon le numéro d'usage de la zone (§9.7.3, tableau 279) :
/// 3 pour les usages 1, 2, 5, 6, 7 et 16 ; 4 pour les hôtels en partie nuit (8 et 9) ; 2 pour
/// les autres.
pub fn nb_bouchons(numero_usage: u8) -> f64 {
    match numero_usage {
        1 | 2 | 5 | 6 | 7 | 16 => NB_BOUCHONS_HABITATION,
        8 | 9 => 4.0,
        _ => 2.0,
    }
}

/// « θamb » des pertes intergroupes, en °C (éq. 1745 et 1757).
pub const THETA_AMB_ECS_C: f64 = 20.0;

/// « Pcirc_vc-e » : part de la consommation des auxiliaires intergroupes transmise à
/// l'ambiance (tableau 280 et éq. 1751, 1760 : 0).
pub const P_CIRC_VC_ECS: f64 = 0.0;

/// Écart conventionnel entre départ et retour d'une boucle, en K (éq. 1742).
pub const DELTA_THETA_BOUCLE_K: f64 = 5.0;

/// « Lvc_2nd-e » par défaut (éq. 1729, « δlvc = 0 ») : `6 × A/80` en habitation,
/// `0,05 × A` dans les autres usages ; `A` est la surface desservie par l'émetteur d'ECS.
pub fn l_vc_2nd_e_defaut_m(usage: UsageZone, a_m2: f64) -> f64 {
    if usage.est_habitation() {
        6.0 * a_m2 / 80.0
    } else {
        0.05 * a_m2
    }
}

/// Sorties d'une distribution d'ECS du groupe à un pas de temps, en Wh.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SortiesDistributionEcsGroupe {
    /// « Qw » : besoins de l'émetteur d'ECS.
    pub q_w_wh: f64,
    /// « Qw_2nd-e » : besoins majorés des pertes de distribution (éq. 1734).
    pub q_w_2nd_wh: f64,
    /// « Φpertes_vc_2nd-e » (éq. 1732), récupérables.
    pub phi_pertes_vc_wh: f64,
    /// « Φpertes_hvc_2nd-e » (éq. 1733).
    pub phi_pertes_hvc_wh: f64,
}

/// Distribution d'ECS du groupe, prête pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct DistributionEcsGroupeResolue {
    /// « θ2nd-e », en °C.
    pub theta_2nd_e_c: f64,
    /// « Vvc_2nd-e », en L (éq. 1730).
    pub v_vc_l: f64,
    /// « Vhvc_2nd-e », en L (éq. 1730).
    pub v_hvc_l: f64,
    /// Multiplicateur des pertes en volume chauffé : `nbdist` si la longueur est saisie, 1 pour
    /// la valeur par défaut (éq. 1729 et lecture Q45).
    pub nb_vc: f64,
    /// « nbdist_2nd-e », multiplicateur des pertes hors volume chauffé.
    pub nb_hvc: f64,
    /// « nbbouchons ».
    pub nb_bouchons: f64,
    /// Indice de l'espace tampon traversé.
    pub espace_tampon: Option<usize>,
    /// « Qw(h − 1) », en Wh.
    pub q_w_precedent_wh: f64,
}

impl DistributionEcsGroupeResolue {
    /// Th-BCE 2020 §9.7.3, éq. (1729) et (1730). `a_m2` : surface desservie par l'émetteur.
    pub fn nouvelle(
        d: &DistributionEcsGroupe,
        usage: UsageZone,
        a_m2: f64,
        espace_tampon: Option<usize>,
    ) -> Self {
        let section = core::f64::consts::PI * d.d_int_m * d.d_int_m / 4.0;
        let nb = f64::from(d.nb_dist.max(1));
        let l_vc = d.l_vc_m.unwrap_or_else(|| l_vc_2nd_e_defaut_m(usage, a_m2));
        Self {
            theta_2nd_e_c: d.theta_2nd_e_c.unwrap_or(THETA_2ND_E_CONV_C),
            v_vc_l: l_vc * section * 1000.0,
            v_hvc_l: d.l_hvc_m * section * 1000.0,
            nb_vc: if d.l_vc_m.is_some() { nb } else { 1.0 },
            nb_hvc: nb,
            nb_bouchons: nb_bouchons(usage.numero()),
            espace_tampon,
            q_w_precedent_wh: 0.0,
        }
    }

    /// « Issuccessif(h) » (éq. 1731) : 1 au premier pas de puisage, `(nbbouchons − 1)/nbbouchons`
    /// aux pas de puisage suivants, 0 sans puisage.
    pub fn is_successif(&self, q_w_wh: f64) -> f64 {
        if q_w_wh == 0.0 {
            0.0
        } else if self.q_w_precedent_wh == 0.0 {
            1.0
        } else {
            (self.nb_bouchons - 1.0) / self.nb_bouchons
        }
    }

    /// Pas de temps (éq. 1731 à 1734) ; l'état avance d'un pas.
    pub fn pas(
        &mut self,
        q_w_wh: f64,
        theta_i_c: f64,
        theta_ext_c: f64,
        b_therm: f64,
    ) -> SortiesDistributionEcsGroupe {
        let is = self.is_successif(q_w_wh);
        let rc = RHO_W_KG_L * C_W_WH_KG_K;
        let pertes_vc = rc
            * self.v_vc_l
            * (self.theta_2nd_e_c - theta_i_c)
            * self.nb_bouchons
            * is
            * self.nb_vc;
        let theta_hvc = b_therm * theta_ext_c + (1.0 - b_therm) * theta_i_c;
        let pertes_hvc = rc
            * self.v_hvc_l
            * (self.theta_2nd_e_c - theta_hvc).max(0.0)
            * self.nb_bouchons
            * is
            * self.nb_hvc;
        self.q_w_precedent_wh = q_w_wh;
        SortiesDistributionEcsGroupe {
            q_w_wh,
            q_w_2nd_wh: q_w_wh + pertes_vc + pertes_hvc,
            phi_pertes_vc_wh: pertes_vc,
            phi_pertes_hvc_wh: pertes_hvc,
        }
    }
}

/// Alerte de la fiche 9.7 : température de distribution inférieure à celle de l'eau mitigée.
pub fn alerte_temperature(theta_2nd_e_c: f64) -> bool {
    theta_2nd_e_c < THETA_UW_C
}

/// « idencl-e(j) » (§9.8.3.2, éq. 1736) : le réseau intergroupe fonctionne dès que l'une des
/// distributions du groupe qui lui sont reliées est en fonctionnement, c'est-à-dire dès que
/// l'indicateur `iecs(j)` (éq. 47) de la zone de l'une d'elles vaut 1.
pub fn id_encl(i_ecs_aval: impl IntoIterator<Item = bool>) -> bool {
    i_ecs_aval.into_iter().any(|i| i)
}

/// « Wcirc_prim-e » (§9.8.3.3.4, éq. 1747 et 1748), en Wh pour un pas d'une heure : `Pcirc-e`,
/// sauf si les circulateurs sont arrêtés pendant les vacances (« Typegest_circ-e = 1 ») et que
/// le réseau est arrêté (`idencl-e(j) = 0`). Sans saisie, fonctionnement permanent
/// (« Typegest_circ-e = 0 » ; la validation exige la saisie pour une boucle qui dessert une
/// zone d'enseignement).
pub fn w_circ_prim_wh(b: &crate::entree::BoucleEcs, id_encl: bool) -> f64 {
    if b.arret_circulateurs_vacances == Some(true) && !id_encl {
        0.0
    } else {
        b.p_circ_w
    }
}

/// Sorties d'une distribution d'ECS intergroupe à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SortiesDistributionEcsIntergroupe {
    /// « Qw_prim-e » : besoins transmis à la génération, en Wh (éq. 1752, 1761, 1764).
    pub q_w_prim_wh: f64,
    /// « θmoy_prim-e », en °C.
    pub theta_moy_c: f64,
    /// « θdépart_prim-e », en °C.
    pub theta_depart_c: f64,
    /// « θretour_prim-e », en °C.
    pub theta_retour_c: f64,
    /// « Waux_prim-e » : circulateurs ou traceurs, en Wh.
    pub w_aux_wh: f64,
    /// « Wrechauf_prim-e » : réchauffeur de boucle, en Wh.
    pub w_rechauf_wh: f64,
    /// « Φaux_vc », en Wh.
    pub phi_aux_vc_wh: f64,
    /// « Φpertes_vc_prim-e », en Wh.
    pub phi_pertes_vc_wh: f64,
    /// « Φpertes_hvc_prim-e », en Wh.
    pub phi_pertes_hvc_wh: f64,
    /// « idencl-e(j) » (éq. 1736) : réseau en fonctionnement ; transmis à la génération
    /// (éq. 1007).
    pub id_encl: bool,
}

/// Pas de temps d'une distribution d'ECS intergroupe (§9.8.3).
///
/// `aval` : `(Qw_2nd-e, θ2nd-e)` de chaque distribution du groupe reliée ; `id_encl` :
/// « idencl-e(j) » (éq. 1736), vrai pendant la période de fonctionnement de la génération ;
/// `theta_cw_c` : température de l'eau froide ; `b_therm` : espace tampon traversé.
pub fn pas_intergroupe(
    reseau: &ReseauEcsIntergroupe,
    aval: &[(f64, f64)],
    id_encl: bool,
    theta_cw_c: f64,
    theta_ext_c: f64,
    b_therm: f64,
) -> SortiesDistributionEcsIntergroupe {
    pas_intergroupe_selon_depart(
        reseau,
        aval,
        id_encl,
        theta_cw_c,
        theta_ext_c,
        b_therm,
        None,
    )
}

/// Comme [`pas_intergroupe`] ; `theta_depart_ballon_c` : température de départ imposée par le
/// ballon central d'un CESCAI (« θdépart_prim-e = θcentr_b4 », §9.17.3.2, deuxième étape),
/// qui fixe la température moyenne et les pertes d'un réseau bouclé. Les pertes sont alors
/// bornées à zéro : un ballon central plus froid que l'ambiance ne fait pas circuler de
/// chaleur (lecture Q53). La température de départ restituée reste celle attendue par les
/// distributions du groupe (éq. 2071).
pub fn pas_intergroupe_selon_depart(
    reseau: &ReseauEcsIntergroupe,
    aval: &[(f64, f64)],
    id_encl: bool,
    theta_cw_c: f64,
    theta_ext_c: f64,
    b_therm: f64,
    theta_depart_ballon_c: Option<f64>,
) -> SortiesDistributionEcsIntergroupe {
    let q_2nd: f64 = aval.iter().map(|(q, _)| q).sum();
    // Éq. (1740), (1753) et (1762).
    let theta_depart = aval
        .iter()
        .map(|(_, t)| *t)
        .reduce(f64::max)
        .unwrap_or(THETA_AMB_ECS_C);
    let encl = if id_encl { 1.0 } else { 0.0 };
    // Éq. (1744) et (1756).
    let pertes = |l_vc: f64, l_hvc: f64, u: f64, theta_moy: f64| {
        (
            u * l_vc * (theta_moy - THETA_AMB_ECS_C) * encl,
            u * l_hvc
                * encl
                * (theta_moy - (THETA_AMB_ECS_C + b_therm * (theta_ext_c - THETA_AMB_ECS_C))),
        )
    };
    match reseau {
        ReseauEcsIntergroupe::Aucun => SortiesDistributionEcsIntergroupe {
            id_encl,
            q_w_prim_wh: q_2nd,
            theta_moy_c: theta_depart,
            theta_depart_c: theta_depart,
            theta_retour_c: theta_cw_c,
            ..SortiesDistributionEcsIntergroupe::default()
        },
        ReseauEcsIntergroupe::Boucle(b) => {
            // Éq. (1742) et (1743).
            let depart = theta_depart_ballon_c.unwrap_or(theta_depart);
            let theta_retour = depart - DELTA_THETA_BOUCLE_K;
            let theta_moy = (depart + theta_retour) / 2.0;
            let (vc, hvc) = pertes(b.l_vc_m, b.l_hvc_m, b.u_w_mk, theta_moy);
            let (vc, hvc) = if theta_depart_ballon_c.is_some() {
                (vc.max(0.0), hvc.max(0.0))
            } else {
                (vc, hvc)
            };
            // Éq. (1747) à (1749) : circulateurs arrêtés quand la génération d'ECS est arrêtée
            // (vacances des zones d'enseignement) si « Typegest_circ-e = 1 », permanents sinon.
            let w_aux = w_circ_prim_wh(b, id_encl);
            let w_rechauf = if b.rechauffeur { vc + hvc } else { 0.0 };
            // Éq. (1752).
            let q = if b.rechauffeur {
                q_2nd
            } else {
                q_2nd + vc + hvc
            };
            SortiesDistributionEcsIntergroupe {
                id_encl,
                q_w_prim_wh: q,
                theta_moy_c: theta_moy,
                theta_depart_c: theta_depart,
                theta_retour_c: theta_retour,
                w_aux_wh: w_aux,
                w_rechauf_wh: w_rechauf,
                phi_aux_vc_wh: P_CIRC_VC_ECS * w_aux,
                phi_pertes_vc_wh: vc,
                phi_pertes_hvc_wh: hvc,
            }
        }
        ReseauEcsIntergroupe::Trace(t) => {
            // Éq. (1754) à (1761) : pertes compensées par le traceur, non ajoutées aux besoins.
            let (vc, hvc) = pertes(t.l_vc_m, t.l_hvc_m, t.u_w_mk, theta_depart);
            let w_aux = vc + hvc;
            SortiesDistributionEcsIntergroupe {
                id_encl,
                q_w_prim_wh: q_2nd,
                theta_moy_c: theta_depart,
                theta_depart_c: theta_depart,
                theta_retour_c: theta_cw_c,
                w_aux_wh: w_aux,
                w_rechauf_wh: 0.0,
                phi_aux_vc_wh: P_CIRC_VC_ECS * w_aux,
                phi_pertes_vc_wh: vc,
                phi_pertes_hvc_wh: hvc,
            }
        }
    }
}

/// Émetteur d'ECS équivalent d'un groupe et sa distribution, prêts pour le calcul.
#[derive(Debug, Clone, PartialEq)]
pub struct EmetteurEcsResolu {
    /// Identifiant.
    pub id: String,
    /// Besoins hebdomadaires (§9.6.3.2 et §9.6.3.3).
    pub besoins: super::besoins_ecs::BesoinsHebdomadaires,
    /// Distribution du groupe.
    pub distribution: DistributionEcsGroupeResolue,
    /// Indice de la distribution d'ECS intergroupe du bâtiment.
    pub intergroupe: usize,
    /// Vrai si `Lvc_2nd-e` prend la valeur par défaut de l'éq. (1729).
    pub l_vc_defaut: bool,
    /// Vrai si `θ2nd-e` prend la valeur conventionnelle du tableau 278.
    pub theta_2nd_defaut: bool,
    /// Récupérateur de chaleur sur les eaux grises (fiche 9.21), si « Irecup » vaut 1.
    pub recuperateur: Option<RecuperateurResolu>,
    /// Sorties du récupérateur au dernier pas de temps.
    pub sorties_recuperateur: SortiesRecuperateur,
    /// Sorties du dernier pas de temps.
    pub sorties: SortiesDistributionEcsGroupe,
    /// « iecs(j) » de la zone au dernier pas de temps (éq. 47) : faux pendant les vacances
    /// d'une zone d'enseignement, où la distribution intergroupe qui la dessert est arrêtée
    /// (éq. 1736) si aucune autre distribution reliée ne fonctionne.
    pub i_ecs: bool,
    /// Cumuls annuels : `Qw`, `Qw_bruts`, `Qw_2nd-e`, pertes en et hors volume chauffé, en Wh.
    pub cumuls: CumulsEcs,
}

/// Cumuls annuels d'un émetteur d'ECS, en Wh.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CumulsEcs {
    /// « Qw » (éq. 1693).
    pub q_w_wh: f64,
    /// « Qw_bruts » (éq. 1694).
    pub q_w_bruts_wh: f64,
    /// « Qw_2nd-e » (éq. 1734).
    pub q_w_2nd_wh: f64,
    /// « Φpertes_vc_2nd-e ».
    pub phi_pertes_vc_wh: f64,
    /// « Φpertes_hvc_2nd-e ».
    pub phi_pertes_hvc_wh: f64,
    /// « Qw » sans récupérateur (éq. 1693), en Wh.
    pub q_w_sans_recup_wh: f64,
    /// « Qrelevage » (éq. 2224), en Wh.
    pub w_relevage_wh: f64,
    /// Volume d'eau chaude appelé au ballon, en L, et somme de ce volume pondéré par
    /// `θentrant`, en L·°C (moyenne de la température d'entrée du ballon).
    pub v_ec_l: f64,
    /// Voir `v_ec_l`.
    pub v_ec_theta_l_c: f64,
}

impl EmetteurEcsResolu {
    /// Résout un émetteur d'ECS ; `None` hors habitation.
    pub fn nouveau(
        e: &crate::entree::EmetteurEcs,
        usage: UsageZone,
        a_gr_m2: f64,
        intergroupe: usize,
        espace_tampon: Option<usize>,
    ) -> Option<Self> {
        let besoins = super::besoins_ecs::BesoinsHebdomadaires::nouveaux(e, usage, a_gr_m2)?;
        Some(Self {
            id: e.id.clone(),
            distribution: DistributionEcsGroupeResolue::nouvelle(
                &e.distribution,
                usage,
                besoins.a_m2,
                espace_tampon,
            ),
            besoins,
            intergroupe,
            l_vc_defaut: e.distribution.l_vc_m.is_none(),
            theta_2nd_defaut: e.distribution.theta_2nd_e_c.is_none(),
            recuperateur: e
                .recuperateur
                .as_ref()
                .filter(|_| e.recuperateur_eaux_grises != Some(false))
                .map(|r| {
                    RecuperateurResolu::nouveau(r, super::besoins_ecs::rat_douches_bains(usage))
                }),
            sorties_recuperateur: SortiesRecuperateur::default(),
            sorties: SortiesDistributionEcsGroupe::default(),
            i_ecs: true,
            cumuls: CumulsEcs::default(),
        })
    }

    /// Pas de temps : besoins (éq. 1693 et 1694) et distribution du groupe (éq. 1731 à 1734) ;
    /// `i_ecs` : « iecs(j) » de la zone (éq. 47), transmis à la distribution intergroupe
    /// (éq. 1736). Les besoins et la distribution du groupe n'en dépendent pas (lecture Q71).
    pub fn pas(
        &mut self,
        ah: f64,
        i_ecs: bool,
        theta_cw_c: f64,
        theta_i_c: f64,
        theta_ext_c: f64,
        b_therm: f64,
    ) {
        self.i_ecs = i_ecs;
        let q_w_sans = self.besoins.q_w_wh(ah, theta_cw_c);
        // Fiche 9.21 : le récupérateur modifie les besoins (positions 0 et 2) et la température
        // d'entrée du ballon (éq. 2183, 2186, 2189 et 2191).
        let rec = match &self.recuperateur {
            Some(r) => r.pas(self.besoins.v_uw_hebdo_corr_l * ah, theta_cw_c, theta_ext_c),
            None => SortiesRecuperateur {
                q_w_wh: q_w_sans,
                q_w_sans_wh: q_w_sans,
                v_ec_l: self.besoins.v_uw_hebdo_corr_l * ah * (THETA_UW_C - theta_cw_c)
                    / (super::recuperateur_eaux_grises::THETA_EC_C - theta_cw_c),
                theta_entrant_c: theta_cw_c,
                ..SortiesRecuperateur::default()
            },
        };
        self.sorties_recuperateur = rec;
        let q_w = rec.q_w_wh;
        self.sorties = self.distribution.pas(q_w, theta_i_c, theta_ext_c, b_therm);
        let c = &mut self.cumuls;
        c.q_w_wh += q_w;
        c.q_w_sans_recup_wh += rec.q_w_sans_wh;
        c.w_relevage_wh += rec.w_relevage_wh;
        c.v_ec_l += rec.v_ec_l;
        c.v_ec_theta_l_c += rec.v_ec_l * rec.theta_entrant_c;
        c.q_w_bruts_wh += self.besoins.q_w_bruts_wh(ah, theta_cw_c);
        c.q_w_2nd_wh += self.sorties.q_w_2nd_wh;
        c.phi_pertes_vc_wh += self.sorties.phi_pertes_vc_wh;
        c.phi_pertes_hvc_wh += self.sorties.phi_pertes_hvc_wh;
    }
}
