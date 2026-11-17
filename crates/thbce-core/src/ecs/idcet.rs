// SPDX-License-Identifier: GPL-3.0-or-later
//! Saisie des chauffe-eau thermodynamiques à compression électrique : fiche `PR1_IdCET`
//! (Th-BCE 2020 §9.22).
//!
//! Les essais de la norme NF EN 16147 portent sur l'ensemble formé par la pompe à chaleur et
//! son ballon. La fiche identifie les trois paramètres du modèle Th-BCE — coefficient de pertes
//! du ballon `UAs`, valeur pivot du COP et valeur pivot de la puissance absorbée — pour
//! lesquels le modèle « ballon base seule » reproduit les trois résultats de l'essai : durée
//! de mise en température `th`, puissance absorbée en régime stabilisé `Pes` et `COPDHW` sur
//! le cycle de soutirage (§9.22.3.1). Les étapes C, D et E de la norme sont simulées au pas
//! horaire (§9.22.3.3) et le triplet est cherché par l'algorithme de Nelder-Mead (§9.22.3.4).
//!
//! Lectures : `docs/questions.md`, Q48.

use super::assemblage_ballon::{AssemblageBallon, EntreesAssemblage, RegulationGenerateur};
use super::ballon_de_stockage::{Ballon, RHO_CW_WH_L_K};
use super::gestion_regulation_ballon::{Programmation, DELTA_THETA_DEFAUT_K};
use crate::chauffage_refroidissement::calculs_generation::Poste;
use crate::chauffage_refroidissement::thermodynamique_elec::{
    FonctionnementCompresseur, GenerateurThermodynamique, LecturePasPartage, Limites, ModePac,
    PerformancesSaisies, PointMesure, DFOU0_ECS_MIN, ECS_AIR_EXTERIEUR_EAU,
};

/// Température d'eau froide de l'essai, en °C (éq. 2225).
pub const THETA_CW_ESSAI_C: f64 = 10.0;
/// Température maximale du ballon dans IdCET, en °C (§9.22.3.2.4).
pub const THETA_MAX_IDCET_C: f64 = 90.0;
/// Durée maximale de la mise en température, en h (§9.22.3.3.2).
pub const DUREE_MAX_ETAPE_C_H: usize = 48;
/// Seuil de la fonction d'écart en dessous duquel le triplet est accepté (éq. 2249).
pub const EPSILON_MAX: f64 = 1e-4;
/// Valeur de la fonction d'écart pour un triplet absurde (§9.22.3.4).
pub const EPSILON_ABSURDE: f64 = 100.0;
/// Température de l'eau mitigée des volumes de puisage de la norme, en °C (tableau 306).
pub const THETA_MITIGEE_ESSAI_C: f64 = 40.0;
/// « Taux » restitué quand les auxiliaires ne sont pas compris dans l'essai (éq. 2250).
pub const TAUX_SANS_AUXILIAIRES: f64 = 0.02;

/// « Idcycle » : cycle de soutirage de l'étape E (§9.22.3.2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cycle {
    /// 0 : cycle S.
    S,
    /// 1 : cycle M.
    M,
    /// 2 : cycle L.
    L,
    /// 3 : cycle XL.
    Xl,
    /// 4 : cycle XXL.
    Xxl,
}

impl Cycle {
    /// Énergies de soutirage des seize heures de 7 h à 23 h, en kWh (tableau 305 ; les autres
    /// heures sont nulles). Les sommes redonnent les énergies totales du tableau 306 : 2,100,
    /// 5,845, 11,655, 19,070 et 24,530 kWh.
    pub const fn profil_kwh(self) -> [f64; 16] {
        match self {
            Self::S => [
                0.210, 0.105, 0.105, 0.0, 0.210, 0.315, 0.0, 0.0, 0.0, 0.0, 0.0, 0.210, 0.0, 0.420,
                0.525, 0.0,
            ],
            Self::M => [
                1.610, 0.420, 0.210, 0.105, 0.210, 0.315, 0.0, 0.105, 0.105, 0.105, 0.0, 0.315,
                0.105, 0.735, 1.505, 0.0,
            ],
            Self::L => [
                1.715, 3.920, 0.210, 0.105, 0.210, 0.315, 0.0, 0.105, 0.105, 0.105, 0.0, 0.315,
                0.105, 0.735, 3.710, 0.0,
            ],
            Self::Xl => [
                6.450, 0.420, 0.210, 0.210, 0.315, 0.735, 0.0, 0.105, 0.210, 0.210, 0.105, 0.315,
                0.105, 5.155, 4.525, 0.0,
            ],
            Self::Xxl => [
                8.270, 0.420, 0.210, 0.210, 0.315, 0.735, 0.0, 0.105, 0.210, 0.210, 0.105, 0.315,
                0.105, 6.975, 6.345, 0.0,
            ],
        }
    }

    /// « V40°C » : volume quotidien d'eau à 40 °C, en L (tableau 306).
    pub const fn v40_l(self) -> f64 {
        match self {
            Self::S => 60.0,
            Self::M => 167.1,
            Self::L => 333.2,
            Self::Xl => 545.2,
            Self::Xxl => 701.3,
        }
    }

    /// Besoin d'ECS de l'heure d'indice `k` de l'étape E, en Wh (éq. 2227, 2230 et 2231) :
    /// `ρw·cw × V40°C × ah(k) × (40 − 10)`, la clé `ah` étant la part de l'heure dans l'énergie
    /// du cycle (éq. 2230).
    pub fn besoin_wh(self, k: usize) -> f64 {
        let profil = self.profil_kwh();
        let total: f64 = profil.iter().sum();
        let ah = profil.get(k).copied().unwrap_or(0.0) / total;
        RHO_CW_WH_L_K * self.v40_l() * ah * (THETA_MITIGEE_ESSAI_C - THETA_CW_ESSAI_C)
    }
}

/// Résultats d'un essai selon la norme NF EN 16147 et paramètres d'IdCET (tableau 304).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Essai {
    /// « Idcycle ».
    pub cycle: Cycle,
    /// « VtotIdCET » : volume du ballon, en L.
    pub v_tot_l: f64,
    /// « th » : durée de mise en température, en h.
    pub t_h_h: f64,
    /// « Pes » : puissance absorbée en régime stabilisé, en W.
    pub p_es_w: f64,
    /// « COPDHW ».
    pub cop_dhw: f64,
    /// « θ'WH » : température d'eau chaude de référence, en °C.
    pub theta_wh_c: f64,
    /// Vrai pour une pompe à chaleur sur air ambiant (« Typesource_chaleur = 2 ») : ambiance de
    /// l'essai à 15 °C, sinon 20 °C (éq. 2226).
    pub air_ambiant: bool,
}

impl Essai {
    /// « θamb » de l'essai, en °C (éq. 2226).
    pub fn theta_amb_c(&self) -> f64 {
        if self.air_ambiant {
            15.0
        } else {
            20.0
        }
    }
}

/// Triplet de paramètres du modèle Th-BCE (§9.22.3.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triplet {
    /// « UAs,util », en W/K.
    pub ua_s_w_k: f64,
    /// « ValCOPpivot ».
    pub cop_pivot: f64,
    /// « ValPabspivot », en W.
    pub pabs_pivot_w: f64,
}

/// Résultats de l'essai reproduits par le modèle : `(th, Pes, COPDHW)` (éq. 2246).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResultatsEssai {
    /// « th^Th-BCE », en h.
    pub t_h_h: f64,
    /// « Pes^Th-BCE », en W.
    pub p_es_w: f64,
    /// « COPDHW^Th-BCE ».
    pub cop_dhw: f64,
}

/// Pompe à chaleur d'ECS du modèle d'IdCET (§9.22.3.2.5) : compresseur en marche-arrêt,
/// auxiliaires nuls, seule la colonne du pivot de la matrice étant employée (éq. 2235).
fn pompe_a_chaleur(t: &Triplet) -> GenerateurThermodynamique {
    let g = &ECS_AIR_EXTERIEUR_EAU;
    let matrice = g.construire(&PerformancesSaisies::Mesurees(vec![PointMesure {
        theta_aval_c: g.theta_aval_c[g.pivot.0],
        theta_amont_c: g.theta_amont_c[g.pivot.1],
        performance: t.cop_pivot,
        pabs_w: t.pabs_pivot_w,
        justifiee: false,
    }]));
    GenerateurThermodynamique {
        chauffage: None,
        refroidissement: None,
        ecs: Some(ModePac {
            matrice,
            limites: Limites::Aucune,
            dfou0_min: DFOU0_ECS_MIN,
            lr_contmin: 1.0,
            ccp_lr_contmin: 1.0,
            t_air_lim_c: None,
        }),
        compresseur: FonctionnementCompresseur::ToutOuRien,
        w_aux_0_w: 0.0,
        rdim: 1.0,
        lecture_pas_partage: LecturePasPartage::Litterale,
    }
}

/// Simule les étapes C, D et E de l'essai avec le modèle Th-BCE pour un triplet de paramètres
/// (§9.22.3.3) ; `None` pour un triplet absurde (§9.22.3.4) : valeur négative, mise en
/// température de plus de 48 h, pas de cycle marche-arrêt complet à l'étape D, ou compresseur
/// qui ne s'arrête pas dans les 48 h après la fin des puisages.
pub fn simuler_essai(essai: &Essai, t: &Triplet) -> Option<ResultatsEssai> {
    if !(t.ua_s_w_k > 0.0 && t.cop_pivot > 0.0 && t.pabs_pivot_w > 0.0) {
        return None;
    }
    let pac = pompe_a_chaleur(t);
    let theta_amont = ECS_AIR_EXTERIEUR_EAU.theta_amont_c[ECS_AIR_EXTERIEUR_EAU.pivot.1];
    let mut ballon = Ballon::nouveau(essai.v_tot_l, t.ua_s_w_k, None, THETA_MAX_IDCET_C);
    // Éq. (2237) : ballon initialement à la température de l'eau froide.
    ballon.fixer_temperature(THETA_CW_ESSAI_C);
    let mut a = AssemblageBallon::nouveau(
        "idcet",
        1.0,
        ballon,
        RegulationGenerateur {
            generateur: 0,
            programmation: Programmation::Permanent,
            delta_theta_k: DELTA_THETA_DEFAUT_K,
            z_gen: 0,
            z_reg: 0,
            h_rel_ech: 0.0,
        },
        None,
        4,
    );
    // Éq. (2233) : consigne du ballon égale à la température d'eau chaude de référence.
    a.theta_c_c = essai.theta_wh_c;
    let theta_amb = essai.theta_amb_c();
    // Un pas de temps : renvoie `(Qreq du générateur, LR, consommation)`.
    let pas = |a: &mut AssemblageBallon, besoin_wh: f64| -> (f64, f64, f64) {
        let mut sortie = (0.0, 0.0, 0.0);
        a.pas(
            &EntreesAssemblage {
                q_req_ecs_wh: besoin_wh,
                theta_entrant_c: THETA_CW_ESSAI_C,
                // Lecture Q48 : l'eau est puisée tant que la zone haute dépasse la température
                // de l'eau mitigée de la norme.
                theta_depart_c: THETA_MITIGEE_ESSAI_C,
                theta_amb_c: theta_amb,
                hleg: 12,
                theta_ext_c: theta_amb,
                isr_w_m2: 0.0,
                mois: 0,
            },
            &mut |_, q_req, theta_aval| {
                let p = pac.point(Poste::Ecs, q_req, theta_amont, theta_aval, 1.0, true, None);
                sortie = (q_req, p.lr, p.pabs_lr_w);
                p.pfou_lr_w
            },
        );
        sortie
    };
    // Étape C (éq. 2238 et 2239) : durée de mise en température.
    let mut nbh_fonct = 0.0;
    let mut fin_c = false;
    let mut q_cons_prec = 0.0;
    for _ in 0..=DUREE_MAX_ETAPE_C_H {
        let (q_req, lr, q_cons) = pas(&mut a, 0.0);
        q_cons_prec = q_cons;
        if q_req > 0.0 {
            nbh_fonct += if lr >= 1.0 { 1.0 } else { lr };
        } else {
            fin_c = true;
            break;
        }
    }
    if !fin_c || nbh_fonct <= 0.0 {
        return None;
    }
    // Étape D (éq. 2240 à 2242) : régime stabilisé, puissance absorbée sur le dernier cycle
    // marche-arrêt complet.
    let (mut duree, mut nb_cycles) = (0u32, 0u32);
    let (mut t_es, mut w_es) = (0.0, 0.0);
    let mut dernier_cycle = None;
    while duree < 48 && nb_cycles < 6 {
        let (_, _, q_cons) = pas(&mut a, 0.0);
        if q_cons_prec != 0.0 && q_cons == 0.0 {
            if nb_cycles > 0 {
                dernier_cycle = Some((w_es, t_es));
            }
            nb_cycles += 1;
            t_es = 0.0;
            w_es = 0.0;
        } else {
            duree += 1;
            t_es += 1.0;
            w_es += q_cons;
        }
        q_cons_prec = q_cons;
    }
    let (w, d) = dernier_cycle?;
    if d <= 0.0 || w <= 0.0 {
        return None;
    }
    let p_es = w / d;
    // Étape E (éq. 2243 à 2245) : cycle de soutirage, ballon à la consigne au premier puisage.
    a.ballon.fixer_temperature(essai.theta_wh_c);
    let (mut q_tc, mut w_tc) = (0.0, 0.0);
    let mut duree_e = 0usize;
    q_cons_prec = 0.0;
    let mut t_ttc = None;
    while duree_e < 24 + 48 {
        let besoin = if duree_e < 24 {
            essai.cycle.besoin_wh(duree_e)
        } else {
            0.0
        };
        let (_, _, q_cons) = pas(&mut a, besoin);
        if duree_e >= 24 && q_cons_prec != 0.0 && q_cons == 0.0 {
            t_ttc = Some(duree_e as f64);
            break;
        }
        duree_e += 1;
        q_tc += besoin;
        w_tc += q_cons;
        q_cons_prec = q_cons;
    }
    let t_ttc = t_ttc?;
    let denominateur = w_tc + (24.0 - t_ttc) * p_es;
    if denominateur <= 0.0 {
        return None;
    }
    Some(ResultatsEssai {
        t_h_h: nbh_fonct,
        p_es_w: p_es,
        cop_dhw: q_tc / denominateur,
    })
}

/// « ε » : écart relatif entre l'essai reproduit par le modèle et l'essai mesuré (éq. 2248) ;
/// 100 pour un triplet absurde.
pub fn epsilon(essai: &Essai, t: &Triplet) -> f64 {
    match simuler_essai(essai, t) {
        None => EPSILON_ABSURDE,
        Some(r) => {
            let e = [
                (r.t_h_h - essai.t_h_h) / essai.t_h_h,
                (r.p_es_w - essai.p_es_w) / essai.p_es_w,
                (r.cop_dhw - essai.cop_dhw) / essai.cop_dhw,
            ];
            e.iter().map(|x| x * x).sum::<f64>().sqrt()
        }
    }
}

/// Résultat de l'identification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Identification {
    /// Triplet identifié.
    pub triplet: Triplet,
    /// « ε » du triplet retenu.
    pub epsilon: f64,
    /// Essai reproduit par le modèle avec ce triplet.
    pub essai_reproduit: ResultatsEssai,
}

/// Minimisation par l'algorithme de Nelder-Mead à partir d'un point, en variables réduites
/// (rapport de chaque paramètre à son estimation initiale).
fn nelder_mead(f: &dyn Fn(&[f64; 3]) -> f64, depart: [f64; 3], pas: f64) -> ([f64; 3], f64) {
    let mut s: Vec<([f64; 3], f64)> = Vec::with_capacity(4);
    s.push((depart, f(&depart)));
    for k in 0..3 {
        let mut x = depart;
        x[k] *= 1.0 + pas;
        s.push((x, f(&x)));
    }
    let combiner = |a: &[f64; 3], b: &[f64; 3], t: f64| -> [f64; 3] {
        [
            a[0] + t * (b[0] - a[0]),
            a[1] + t * (b[1] - a[1]),
            a[2] + t * (b[2] - a[2]),
        ]
    };
    for _ in 0..600 {
        s.sort_by(|a, b| a.1.total_cmp(&b.1));
        // Arrêt quand les points du simplexe sont distants de moins de 10⁻⁹ (§9.22.3.4).
        let taille = s[1..]
            .iter()
            .map(|p| {
                (0..3)
                    .map(|k| (p.0[k] - s[0].0[k]).abs())
                    .fold(0.0, f64::max)
            })
            .fold(0.0, f64::max);
        if taille < 1e-9 || s[0].1 < 1e-12 {
            break;
        }
        let centre = [0, 1, 2].map(|k| (s[0].0[k] + s[1].0[k] + s[2].0[k]) / 3.0);
        let pire = s[3];
        let reflexion = combiner(&centre, &pire.0, -1.0);
        let fr = f(&reflexion);
        if fr < s[0].1 {
            let expansion = combiner(&centre, &pire.0, -2.0);
            let fe = f(&expansion);
            s[3] = if fe < fr {
                (expansion, fe)
            } else {
                (reflexion, fr)
            };
        } else if fr < s[2].1 {
            s[3] = (reflexion, fr);
        } else {
            let contraction = if fr < pire.1 {
                combiner(&centre, &reflexion, 0.5)
            } else {
                combiner(&centre, &pire.0, 0.5)
            };
            let fc = f(&contraction);
            if fc < pire.1.min(fr) {
                s[3] = (contraction, fc);
            } else {
                let meilleur = s[0].0;
                for p in s.iter_mut().skip(1) {
                    p.0 = combiner(&meilleur, &p.0, 0.5);
                    p.1 = f(&p.0);
                }
            }
        }
    }
    s.sort_by(|a, b| a.1.total_cmp(&b.1));
    s[0]
}

/// Identifie le triplet `(UAs, COP pivot, Pabs pivot)` qui reproduit l'essai (§9.22.3.4).
///
/// L'algorithme de Nelder-Mead est relancé depuis d'autres points de départ tant que
/// `ε ≥ 10⁻⁴` (éq. 2249). Le texte tire ces points au hasard ; ils sont pris ici dans une suite
/// déterministe, pour que le calcul soit reproductible, et leur nombre est borné : le meilleur
/// triplet rencontré est restitué avec son `ε` (lecture Q48). `None` si aucun triplet n'est
/// admissible.
pub fn identifier(essai: &Essai) -> Option<Identification> {
    // Estimations initiales : puissance de chauffe d'après la durée de mise en température,
    // COP pivot voisin du COPDHW, pertes d'après la puissance en régime stabilisé.
    let ecart = essai.theta_wh_c - THETA_CW_ESSAI_C;
    let p_fou = RHO_CW_WH_L_K * essai.v_tot_l * ecart / essai.t_h_h;
    let cop0 = essai.cop_dhw.max(1.0);
    let pabs0 = p_fou / (1.4 * cop0);
    let ua0 = (essai.p_es_w * cop0 / (essai.theta_wh_c - essai.theta_amb_c())).max(0.05);
    let base = [ua0, cop0, pabs0];
    let f = |x: &[f64; 3]| {
        epsilon(
            essai,
            &Triplet {
                ua_s_w_k: x[0] * base[0],
                cop_pivot: x[1] * base[1],
                pabs_pivot_w: x[2] * base[2],
            },
        )
    };
    let mut meilleur: Option<([f64; 3], f64)> = None;
    // Suite déterministe de points de départ (générateur congruentiel).
    let mut graine: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut alea = || {
        graine = graine
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((graine >> 11) as f64) / ((1u64 << 53) as f64)
    };
    for n in 0..40 {
        let depart = if n == 0 {
            [1.0; 3]
        } else {
            [0.4 + 1.6 * alea(), 0.6 + alea(), 0.6 + alea()]
        };
        let (x, e) = nelder_mead(&f, depart, 0.15);
        if meilleur.is_none_or(|m| e < m.1) {
            meilleur = Some((x, e));
        }
        if e < EPSILON_MAX {
            break;
        }
    }
    let (x, e) = meilleur?;
    let triplet = Triplet {
        ua_s_w_k: x[0] * base[0],
        cop_pivot: x[1] * base[1],
        pabs_pivot_w: x[2] * base[2],
    };
    let essai_reproduit = simuler_essai(essai, &triplet)?;
    Some(Identification {
        triplet,
        epsilon: e,
        essai_reproduit,
    })
}

/// Essai d'un chauffe-eau saisi, sous la forme attendue par l'identification.
pub fn essai_saisi(c: &crate::entree::ChauffeEauThermodynamique) -> Essai {
    use crate::entree::{CycleSoutirage, SourceAmontSaisie};
    Essai {
        cycle: match c.essai.cycle {
            CycleSoutirage::S => Cycle::S,
            CycleSoutirage::M => Cycle::M,
            CycleSoutirage::L => Cycle::L,
            CycleSoutirage::Xl => Cycle::Xl,
            CycleSoutirage::Xxl => Cycle::Xxl,
        },
        v_tot_l: c.v_tot_l,
        t_h_h: c.essai.t_h_h,
        p_es_w: c.essai.p_es_w,
        cop_dhw: c.essai.cop_dhw,
        theta_wh_c: c.essai.theta_wh_c,
        air_ambiant: matches!(c.source_amont, SourceAmontSaisie::AirAmbiant { .. }),
    }
}

/// Chauffe-eau thermodynamique résolu : entrées Th-BCE déduites de l'essai (§9.22.3.5).
#[derive(Debug, Clone, PartialEq)]
pub struct ChauffeEauResolu {
    /// Pompe à chaleur d'ECS : valeur pivot identifiée, compresseur en marche-arrêt
    /// (éq. 2260), `Taux` selon « Isaux » (éq. 2250 et 2251).
    pub pac: GenerateurThermodynamique,
    /// Source amont (éq. 2256 à 2258).
    pub source: crate::chauffage_refroidissement::sources_amont::SourceAmontGenerateur,
    /// Ballon : volume de l'essai (éq. 2261), `UAs` identifié.
    pub ballon: Ballon,
    /// Identification.
    pub identification: Identification,
}

/// Résout un chauffe-eau thermodynamique saisi : identification du triplet, puis construction
/// du ballon et de la pompe à chaleur avec les statuts de l'essai (§9.22.3.5.2 : pour un
/// essai justifié, le COP pivot est réduit de 10 % et `UAs` augmenté de 10 % par le moteur).
pub fn resoudre(
    c: &crate::entree::ChauffeEauThermodynamique,
    espace_tampon: impl Fn(&str) -> Option<usize>,
    zone: impl Fn(&str) -> Option<usize>,
) -> Option<ChauffeEauResolu> {
    use crate::chauffage_refroidissement::sources_amont::{SourceAmont, SourceAmontGenerateur};
    use crate::chauffage_refroidissement::thermodynamique_elec::{
        ECS_AIR_AMBIANT_EAU, ECS_AIR_EXTRAIT_EAU,
    };
    use crate::entree::{CombinaisonLimites, SourceAmontSaisie, StatutMesure};
    let identification = identifier(&essai_saisi(c))?;
    let t = identification.triplet;
    let justifie = c.essai.statut == StatutMesure::Justifie;
    let (grille, source, p_vent, t_air_lim_c) = match &c.source_amont {
        SourceAmontSaisie::AirExterieur { p_vent_gaine_w } => (
            &ECS_AIR_EXTERIEUR_EAU,
            SourceAmont::AirExterieur,
            *p_vent_gaine_w,
            None,
        ),
        SourceAmontSaisie::AirAmbiant {
            espace_tampon: id,
            p_vent_gaine_w,
        } => (
            &ECS_AIR_AMBIANT_EAU,
            SourceAmont::AirAmbiant {
                espace_tampon: espace_tampon(id)?,
            },
            *p_vent_gaine_w,
            None,
        ),
        SourceAmontSaisie::AirExtrait {
            zone: id,
            t_air_lim_c,
            p_vent_gaine_w,
        } => (
            &ECS_AIR_EXTRAIT_EAU,
            SourceAmont::AirExtrait { zone: zone(id)? },
            *p_vent_gaine_w,
            Some(*t_air_lim_c),
        ),
    };
    let matrice = grille.construire(&PerformancesSaisies::Mesurees(vec![PointMesure {
        theta_aval_c: grille.theta_aval_c[grille.pivot.0],
        theta_amont_c: grille.theta_amont_c[grille.pivot.1],
        performance: t.cop_pivot,
        pabs_w: t.pabs_pivot_w,
        justifiee: justifie,
    }]));
    // Éq. (2250) et (2251) : auxiliaires compris dans l'essai, `Taux = 0` ; sinon 0,02.
    let taux = if c.essai.auxiliaires_compris {
        0.0
    } else {
        TAUX_SANS_AUXILIAIRES
    };
    let limites = match c.limites {
        None => Limites::Aucune,
        Some(l) => match l.combinaison {
            CombinaisonLimites::LUneOuLAutre => Limites::LUneOuLAutre {
                theta_aval_c: l.theta_aval_c,
                theta_amont_c: l.theta_amont_c,
            },
            CombinaisonLimites::LUneEtLAutre => Limites::LUneEtLAutre {
                theta_aval_c: l.theta_aval_c,
                theta_amont_c: l.theta_amont_c,
            },
        },
    };
    let nb = f64::from(c.nb_assemblages.max(1));
    let pac = GenerateurThermodynamique {
        chauffage: None,
        refroidissement: None,
        ecs: Some(ModePac {
            matrice,
            limites,
            dfou0_min: DFOU0_ECS_MIN,
            lr_contmin: 1.0,
            ccp_lr_contmin: 1.0,
            t_air_lim_c,
        }),
        compresseur: FonctionnementCompresseur::ToutOuRien,
        w_aux_0_w: taux * t.pabs_pivot_w,
        rdim: nb,
        lecture_pas_partage: LecturePasPartage::Litterale,
    };
    let ua = if justifie {
        1.1 * t.ua_s_w_k
    } else {
        t.ua_s_w_k
    };
    Some(ChauffeEauResolu {
        pac,
        source: SourceAmontGenerateur {
            source,
            p_vent_gaine_w: p_vent.unwrap_or(0.0),
        },
        ballon: Ballon::nouveau(c.v_tot_l, ua, None, c.theta_max_c),
        identification,
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn profils_de_puisage() {
        // Sommes des profils : énergies totales du tableau 306.
        for (c, total) in [
            (Cycle::S, 2.100),
            (Cycle::M, 5.845),
            (Cycle::L, 11.655),
            (Cycle::Xl, 19.070),
            (Cycle::Xxl, 24.530),
        ] {
            let somme: f64 = c.profil_kwh().iter().sum();
            assert!((somme - total).abs() < 1e-9, "{c:?}");
            // Besoins de la journée : ρc × V40 × 30 K, à 0,5 % de l'énergie de la norme.
            let besoins: f64 = (0..24).map(|k| c.besoin_wh(k)).sum();
            assert!((besoins - 1.163 * c.v40_l() * 30.0).abs() < 1e-6);
            assert!((besoins / 1000.0 / total - 1.0).abs() < 0.005);
        }
        // Clé de répartition du tableau 307, cycle M : 0,275 à la première heure.
        let ah = Cycle::M.besoin_wh(0) / (1.163 * 167.1 * 30.0);
        assert!((ah - 0.275).abs() < 5e-4);
    }

    fn essai() -> Essai {
        Essai {
            cycle: Cycle::L,
            v_tot_l: 200.0,
            t_h_h: 7.0,
            p_es_w: 25.0,
            cop_dhw: 2.8,
            theta_wh_c: 53.0,
            air_ambiant: true,
        }
    }

    #[test]
    fn essai_simule() {
        // Triplet donné : ballon de 200 L, UA = 1,5 W/K, COP pivot 3, 450 W absorbés.
        let t = Triplet {
            ua_s_w_k: 1.5,
            cop_pivot: 3.0,
            pabs_pivot_w: 450.0,
        };
        let r = simuler_essai(&essai(), &t).unwrap();
        // Mise en température de 10 à 53 °C : 1,163 × 200 × 43 = 10,0 kWh, avec une puissance
        // fournie de 1,35 kW au point pivot (45 °C), plus élevée à basse température : quelques
        // heures.
        assert!((4.0..9.0).contains(&r.t_h_h), "{r:?}");
        // Régime stabilisé : pertes 1,5 × (53 − 15) = 57 W, divisées par un COP voisin de 2,5.
        assert!((15.0..40.0).contains(&r.p_es_w), "{r:?}");
        assert!((1.5..3.5).contains(&r.cop_dhw), "{r:?}");
        // Triplets absurdes.
        assert!(simuler_essai(
            &essai(),
            &Triplet {
                ua_s_w_k: -1.0,
                ..t
            }
        )
        .is_none());
        assert!(simuler_essai(
            &essai(),
            &Triplet {
                pabs_pivot_w: 20.0,
                ..t
            }
        )
        .is_none());
        assert_eq!(
            epsilon(
                &essai(),
                &Triplet {
                    cop_pivot: 0.0,
                    ..t
                }
            ),
            100.0
        );
    }

    #[test]
    fn identification_retrouve_un_triplet_connu() {
        // Essai fabriqué par le modèle lui-même : l'identification doit le reproduire.
        let t = Triplet {
            ua_s_w_k: 1.5,
            cop_pivot: 3.0,
            pabs_pivot_w: 450.0,
        };
        let r = simuler_essai(&essai(), &t).unwrap();
        let e = Essai {
            t_h_h: r.t_h_h,
            p_es_w: r.p_es_w,
            cop_dhw: r.cop_dhw,
            ..essai()
        };
        assert!(epsilon(&e, &t) < 1e-12);
        let id = identifier(&e).unwrap();
        println!("{r:?} {id:?}");
        assert!(id.epsilon < 2e-2, "{id:?}");
        assert!((id.essai_reproduit.cop_dhw / r.cop_dhw - 1.0).abs() < 2e-2);
        assert!((id.triplet.cop_pivot / 3.0 - 1.0).abs() < 0.15, "{id:?}");
    }

    /// Modèle de l'essai écrit indépendamment de l'assemblage, à la main, pour un cas où le
    /// ballon reste homogène : sans puisage, la pompe à chaleur chauffe la zone basse, qui se
    /// mélange aussitôt à tout le ballon (§9.9.3.2.2.2), si bien que les quatre zones forment une
    /// seule zone ; avec un puisage d'au plus un quart du volume, seule la zone basse change
    /// (effet piston, éq. 1773 et 1776) et la pompe à chaleur, assez puissante, ramène tout le
    /// ballon à la consigne dans l'heure. Matrice de la pompe à chaleur (éq. 2235) : seule la
    /// colonne du pivot (7 °C amont) sert, avec les coefficients des tableaux 178 et 179,
    /// linéaires en θaval : `COP(θ) = COPpivot × (1 + 0,02 × (45 − θ))`,
    /// `Pabs(θ) = Pabspivot × (1 + 0,01 × (45 − θ))`. Compresseur tout ou rien sans
    /// auxiliaires : `Pabs,LR = Pabs × LR × (1 + 0,5 × (1 − LR) / 26)` (éq. 1301 à 1303).
    fn essai_a_la_main(e: &Essai, t: &Triplet) -> ResultatsEssai {
        const RC: f64 = 1.163;
        let (v, ua, tc) = (e.v_tot_l, t.ua_s_w_k, e.theta_wh_c);
        let tamb = e.theta_amb_c();
        let pfou_pc = |th: f64| {
            t.cop_pivot * (1.0 + 0.02 * (45.0 - th)) * t.pabs_pivot_w * (1.0 + 0.01 * (45.0 - th))
        };
        let pabs = |th: f64, lr: f64| {
            t.pabs_pivot_w * (1.0 + 0.01 * (45.0 - th)) * lr * (1.0 + 0.5 * (1.0 - lr) / 26.0)
        };
        // État : température homogène du ballon en fin de pas, et celle du pas précédent.
        let (mut th, mut th2) = (10.0, 10.0);
        // Un pas sans puisage : renvoie (Qreq, LR, Qcons) ; éq. (1770), (1813), (1815) et
        // (1786).
        let sans_puisage = |th: &mut f64, th2: &mut f64| {
            let pertes = ua * (*th - tamb);
            let actif = *th < tc - 2.0 || (*th < tc - 1e-9 && *th2 < *th);
            let (q_req, lr, q_cons) = if actif {
                let q_req = RC * v * (tc - *th) + pertes;
                let lr = (q_req / pfou_pc(*th)).min(1.0);
                (q_req, lr, pabs(*th, lr))
            } else {
                (0.0, 0.0, 0.0)
            };
            let q_fou = if actif { lr * pfou_pc(*th) } else { 0.0 };
            *th2 = *th;
            *th += (q_fou - pertes) / (RC * v);
            (q_req, lr, q_cons)
        };
        // Étape C (éq. 2238 et 2239).
        let mut t_h = 0.0;
        let mut q_cons_prec;
        loop {
            let (q_req, lr, q_cons) = sans_puisage(&mut th, &mut th2);
            q_cons_prec = q_cons;
            if q_req > 0.0 {
                t_h += lr;
            } else {
                break;
            }
        }
        // Étape D (éq. 2240 à 2242).
        let (mut duree, mut nb_cycles, mut t_es, mut w_es) = (0, 0, 0.0, 0.0);
        let mut dernier = (0.0, 0.0);
        while duree < 48 && nb_cycles < 6 {
            let (_, _, q_cons) = sans_puisage(&mut th, &mut th2);
            if q_cons_prec != 0.0 && q_cons == 0.0 {
                if nb_cycles > 0 {
                    dernier = (w_es, t_es);
                }
                nb_cycles += 1;
                t_es = 0.0;
                w_es = 0.0;
            } else {
                duree += 1;
                t_es += 1.0;
                w_es += q_cons;
            }
            q_cons_prec = q_cons;
        }
        let p_es = dernier.0 / dernier.1;
        // Étape E (éq. 2243 à 2245) : ballon à la consigne au premier puisage.
        th = tc;
        th2 = tc;
        let (mut q_tc, mut w_tc, mut duree_e) = (0.0, 0.0, 0usize);
        q_cons_prec = 0.0;
        let t_ttc = loop {
            let besoin = if duree_e < 24 {
                e.cycle.besoin_wh(duree_e)
            } else {
                0.0
            };
            let q_cons = if besoin > 0.0 {
                // Puisage dans la zone haute, à θ, d'un volume d'au plus un quart du ballon :
                // seule la zone basse change.
                let vz = v / 4.0;
                let vp = besoin / (RC * (th - 10.0));
                assert!(vp <= vz);
                let t0 = (th * (vz - vp) + 10.0 * vp) / vz;
                let pertes = ua * (th - tamb);
                let q_req = RC * vz * (tc - t0) + 3.0 * RC * vz * (tc - th) + pertes;
                // Éq. (1785) : moyenne de la zone basse entre la fin du pas précédent et l'état
                // après puisage.
                let t_aval = (th + t0) / 2.0;
                let lr = q_req / pfou_pc(t_aval);
                assert!(
                    lr < 1.0,
                    "pompe à chaleur trop petite pour ce calcul à la main"
                );
                th2 = th;
                th = tc;
                pabs(t_aval, lr)
            } else {
                sans_puisage(&mut th, &mut th2).2
            };
            if duree_e >= 24 && q_cons_prec != 0.0 && q_cons == 0.0 {
                break duree_e as f64;
            }
            duree_e += 1;
            q_tc += besoin;
            w_tc += q_cons;
            q_cons_prec = q_cons;
        };
        ResultatsEssai {
            t_h_h: t_h,
            p_es_w: p_es,
            cop_dhw: q_tc / (w_tc + (24.0 - t_ttc) * p_es),
        }
    }

    #[test]
    fn essai_simule_compare_a_un_calcul_a_la_main() {
        // Ballon de 200 L dont les pertes (2 W/K) déclenchent des cycles en régime stabilisé ;
        // pompe à chaleur de 1 kW absorbé (COP 3 au pivot), assez puissante pour remonter le
        // ballon dans l'heure après chaque puisage du cycle S.
        let e = Essai {
            cycle: Cycle::S,
            theta_wh_c: 55.0,
            air_ambiant: false,
            ..essai()
        };
        let t = Triplet {
            ua_s_w_k: 2.0,
            cop_pivot: 3.0,
            pabs_pivot_w: 1000.0,
        };
        let main = essai_a_la_main(&e, &t);
        let r = simuler_essai(&e, &t).unwrap();
        println!("{r:?} {main:?}");
        assert!((r.t_h_h - main.t_h_h).abs() < 1e-9, "{r:?} {main:?}");
        assert!((r.p_es_w - main.p_es_w).abs() < 1e-9, "{r:?} {main:?}");
        assert!((r.cop_dhw - main.cop_dhw).abs() < 1e-9, "{r:?} {main:?}");
        // Ordres de grandeur : 1,163 × 200 × 45 = 10,5 kWh à fournir avec 3,5 à 4,6 kW, soit
        // moins de 4 h ; pertes de 70 W à 55 °C compensées avec un COP voisin de 2,6.
        assert!((2.0..4.0).contains(&main.t_h_h), "{main:?}");
        assert!((15.0..40.0).contains(&main.p_es_w), "{main:?}");
    }
}
