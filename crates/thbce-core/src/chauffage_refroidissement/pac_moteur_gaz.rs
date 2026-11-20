// SPDX-License-Identifier: GPL-3.0-or-later
//! Pompes à chaleur à moteur gaz : fiche `C_Gen_PAC Moteur_Gaz` (Th-BCE 2020 §8.25), type 504
//! du tableau 111.
//!
//! Pompe à chaleur réversible à compression entraînée par un moteur au gaz naturel, sur air
//! extérieur, en air/eau ou en air/air recyclé (DRV). Pour chaque mode, trois matrices — GUE (ou
//! EER), puissance gaz absorbée et puissance des auxiliaires de l'unité extérieure — sont
//! construites autour d'une valeur pivot (§8.25.3.1 à 8.25.3.3, tableaux 230 à 242) avec la
//! mécanique commune de `thermodynamique_gaz.rs`, puis interpolées à chaque pas de temps. La
//! charge partielle suit le §8.25.3.4 (éq. 1371 à 1401) : vitesse variable du moteur au-dessus
//! de `LRcontmin`, cycles marche-arrêt en dessous.
//!
//! La production indirecte d'ECS par récupération sur le moteur (§8.25.3.4.5) n'est pas
//! calculée (refusée par la validation, Q144). Lectures : `docs/questions/Q140.md` à `Q144.md`.

use super::calculs_generation::{Energie, Poste};
use super::generation::{EntreesAppel, FonctionGenerateur, SortiesAppel};
use super::thermodynamique_gaz::{
    GrilleGaz, MatricesGaz, PerformancesSaisiesGaz, RegleAuxPivot, RegleColonne, RegleLigne,
    Remplissage, SourceLigne,
};

/// « Deq » conventionnelle, en minutes (§8.25.3.4.3).
pub const DEQ_MIN: f64 = 0.5;
/// « Dfou0 » conventionnelle, en minutes (§8.25.3.4.3).
pub const DFOU0_MIN: f64 = 2.0;
/// « θam,ms » : température amont sous laquelle GUE et Pabs sont constants en chauffage, en °C
/// (§8.25.3.2.1, tableau 229 ; lecture Q141).
pub const THETA_AM_MS_C: f64 = -3.0;
/// « Paux0,def », en W (tableau 229).
pub const P_AUX_0_DEFAUT_W: f64 = 120.0;
/// « LRcontmin,def » et « CcpLRcontmin,def » (§8.25.3.1).
pub const LR_CONTMIN_DEFAUT: f64 = 0.4;
/// « CcpLRcontmin,def ».
pub const CCP_LR_CONTMIN_DEFAUT: f64 = 1.0;

const fn c(cible: usize, source: usize, coef: f64) -> RegleColonne {
    RegleColonne {
        cible,
        source,
        coef,
    }
}

const fn l(cible: usize, source: usize, coef: f64) -> RegleLigne {
    RegleLigne {
        cible,
        source: SourceLigne::Colonne(source),
        coef,
        debut: 0,
        fin: usize::MAX,
    }
}

/// Auxiliaires de l'unité extérieure au pivot (§8.25.3.1, éq. 1367, lecture Q142) : déclarés,
/// `max(1,2 × Paux,uext,pivot ; 0,03 × Pabs,pivot)` ; par défaut, `1,2 × 0,03 × Pabs,pivot`.
pub const AUX_PIVOT: RegleAuxPivot = RegleAuxPivot {
    coef_declare: 0.03,
    plancher_defaut_w: 0.0,
    coef_defaut: 1.2 * 0.03,
};

const AMONT_UN: [RegleLigne; 4] = [l(1, 3, 1.0), l(2, 3, 1.0), l(4, 3, 1.0), l(0, 1, 1.0)];

/// §8.25.3.2.1 — PAC MG air extérieur / eau en chauffage (figure 169, tableaux 231 à 233).
pub const CH_AIR_EAU: GrilleGaz = GrilleGaz {
    theta_aval_c: &[23.5, 32.5, 42.5, 51.0, 60.0],
    theta_amont_c: &[-15.0, -7.0, 2.0, 7.0, 20.0],
    pivot: (1, 3),
    ordre_aval: &[1, 2, 3, 0, 4],
    ordre_amont: &[3, 1, 2, 4, 0],
    gue: Remplissage {
        colonne: &[c(2, 1, 0.80), c(3, 2, 0.80), c(0, 1, 1.15), c(4, 3, 0.80)],
        lignes: &[l(1, 3, 0.4941), l(2, 3, 0.81), l(4, 3, 1.37), l(0, 1, 0.78)],
    },
    pabs: Remplissage {
        colonne: &[c(2, 1, 1.17), c(3, 2, 1.12), c(0, 1, 1.0), c(4, 3, 1.11)],
        lignes: &[l(1, 3, 1.36), l(2, 3, 1.13), l(4, 3, 0.94), l(0, 1, 1.07)],
    },
    paux: Remplissage {
        colonne: &[c(2, 1, 1.0), c(3, 2, 1.0), c(0, 1, 1.0), c(4, 3, 1.0)],
        lignes: &AMONT_UN,
    },
};

/// §8.25.3.2.2 — PAC MG air extérieur / air recyclé en chauffage (figure 170, tableaux 234 à
/// 236).
pub const CH_AIR_AIR: GrilleGaz = GrilleGaz {
    theta_aval_c: &[5.0, 10.0, 15.0, 20.0, 25.0],
    theta_amont_c: &[-15.0, -7.0, 2.0, 7.0, 20.0],
    pivot: (3, 3),
    ordre_aval: &[3, 2, 4, 1, 0],
    ordre_amont: &[3, 1, 2, 4, 0],
    gue: Remplissage {
        colonne: &[c(0, 3, 0.91), c(1, 3, 0.94), c(2, 3, 0.97), c(4, 3, 0.92)],
        lignes: &[l(1, 3, 0.75), l(2, 3, 0.8), l(4, 3, 1.10), l(0, 1, 1.0)],
    },
    pabs: Remplissage {
        colonne: &[c(0, 3, 1.15), c(1, 3, 1.1), c(2, 3, 1.05), c(4, 3, 1.0)],
        lignes: &[l(1, 3, 1.35), l(2, 3, 1.15), l(4, 3, 0.90), l(0, 1, 1.0)],
    },
    paux: Remplissage {
        colonne: &[c(0, 3, 1.0), c(1, 3, 1.0), c(2, 3, 1.0), c(4, 3, 1.0)],
        lignes: &[l(1, 3, 0.3), l(2, 3, 0.65), l(4, 3, 1.0), l(0, 1, 0.3)],
    },
};

/// §8.25.3.3.1 — PAC MG air extérieur / eau en refroidissement (figure 171, tableaux 237 à
/// 239).
pub const FR_AIR_EAU: GrilleGaz = GrilleGaz {
    theta_aval_c: &[4.0, 9.5, 15.0, 20.5, 26.0],
    theta_amont_c: &[5.0, 15.0, 25.0, 35.0, 45.0],
    pivot: (1, 3),
    ordre_aval: &[1, 3, 2, 4, 0],
    ordre_amont: &[3, 4, 2, 1, 0],
    gue: Remplissage {
        colonne: &[c(0, 1, 0.90), c(2, 1, 1.08), c(3, 2, 1.07), c(4, 3, 1.06)],
        lignes: &[l(4, 3, 0.71), l(2, 3, 1.18), l(1, 2, 0.82), l(0, 1, 0.78)],
    },
    pabs: Remplissage {
        colonne: &[c(0, 1, 0.95), c(2, 1, 1.06), c(3, 2, 1.05), c(4, 3, 1.05)],
        lignes: &[l(4, 3, 1.15), l(2, 3, 0.86), l(1, 2, 0.88), l(0, 1, 0.9)],
    },
    paux: Remplissage {
        colonne: &[c(0, 1, 1.0), c(2, 1, 1.0), c(3, 2, 1.0), c(4, 3, 1.0)],
        lignes: &[l(4, 3, 1.0), l(2, 3, 0.92), l(1, 2, 0.61), l(0, 1, 1.0)],
    },
};

/// §8.25.3.3.2 — PAC MG air extérieur / air recyclé en refroidissement (figure 172, tableaux
/// 240 à 242).
pub const FR_AIR_AIR: GrilleGaz = GrilleGaz {
    theta_aval_c: &[22.0, 27.0, 32.0, 37.0],
    theta_amont_c: &[5.0, 15.0, 25.0, 35.0, 45.0],
    pivot: (1, 3),
    ordre_aval: &[1, 0, 2, 3],
    ordre_amont: &[3, 2, 1, 0, 4],
    gue: Remplissage {
        colonne: &[c(0, 1, 0.97), c(2, 1, 1.01), c(3, 1, 1.01)],
        lignes: &[l(1, 3, 1.2), l(2, 3, 1.1), l(4, 3, 0.7), l(0, 3, 1.3)],
    },
    pabs: Remplissage {
        colonne: &[c(0, 1, 0.90), c(2, 1, 1.04), c(3, 1, 1.05)],
        lignes: &[l(1, 3, 0.85), l(2, 3, 0.92), l(4, 3, 1.2), l(0, 3, 0.75)],
    },
    paux: Remplissage {
        colonne: &[c(0, 1, 1.0), c(2, 1, 1.0), c(3, 1, 1.0)],
        lignes: &[l(1, 3, 1.0), l(2, 3, 1.0), l(4, 3, 1.0), l(0, 3, 1.0)],
    },
};

/// Grille et valeur utile maximale d'une technologie dans un mode (§8.25.3.1 à 8.25.3.3,
/// lecture Q140).
pub fn grille(
    t: crate::entree::TechnologiePacMoteurGaz,
    poste: Poste,
) -> (&'static GrilleGaz, f64) {
    use crate::entree::TechnologiePacMoteurGaz as T;
    match (t, poste == Poste::Refroidissement) {
        (T::AirExterieurEau, false) => (&CH_AIR_EAU, 1.3),
        (T::AirExterieurAirRecycle, false) => (&CH_AIR_AIR, 1.4),
        (T::AirExterieurEau, true) => (&FR_AIR_EAU, 1.0),
        (T::AirExterieurAirRecycle, true) => (&FR_AIR_AIR, 1.25),
    }
}

/// Lecture des consommations de gaz restituées (éq. 1392, 1393, 1399 et 1400, Q143).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LectureConsommation {
    /// Lecture retenue : la consommation comprend les irréversibilités des cycles,
    /// `Pabs,LR = Pgaz,LR + Pgaz_MA,LR` (éq. 1386), comme le GUE restitué (éq. 1387).
    #[default]
    AvecIrreversibilites,
    /// Variante littérale : `Pgaz,LR` seul.
    Litterale,
}

/// Mode d'une PAC à moteur gaz.
#[derive(Debug, Clone, PartialEq)]
pub struct ModeMoteurGaz {
    /// Matrices de performances.
    pub matrice: MatricesGaz,
    /// « Paux,uint,pc » : unités intérieures à pleine charge, en W.
    pub p_aux_uint_w: f64,
    /// « θam,ms » en chauffage ; `None` en refroidissement.
    pub theta_am_ms_c: Option<f64>,
}

/// Pompe à chaleur à moteur gaz (§8.25) : `Rdim` machines identiques.
#[derive(Debug, Clone, PartialEq)]
pub struct PacMoteurGaz {
    /// Mode chauffage.
    pub chauffage: Option<ModeMoteurGaz>,
    /// Mode refroidissement.
    pub refroidissement: Option<ModeMoteurGaz>,
    /// « Fonct_moteur = 1 » : vitesse variable.
    pub vitesse_variable: bool,
    /// « LRcontmin » corrigé (1 en cycles marche-arrêt).
    pub lr_contmin: f64,
    /// « CcpLRcontmin » corrigé (1 en cycles marche-arrêt).
    pub ccp_lr_contmin: f64,
    /// « Paux0 », en W.
    pub p_aux_0_w: f64,
    /// « Rdim ».
    pub rdim: f64,
    /// Lecture des consommations restituées.
    pub lecture: LectureConsommation,
}

/// Point de fonctionnement d'une machine (W, ou Wh sur une heure).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PointMoteurGaz {
    /// « GUE_pc(h) ».
    pub gue_pc: f64,
    /// « Pabs_pc(h) ».
    pub pabs_pc_w: f64,
    /// « Paux,pc(h) » (éq. 1371).
    pub paux_pc_w: f64,
    /// « Pfou_pc(h) ».
    pub pfou_pc_w: f64,
    /// « Pfou_LR(h) » (éq. 1377).
    pub pfou_lr_w: f64,
    /// « LR(h) » (éq. 1376).
    pub lr: f64,
    /// « Pgaz,LR(h) ».
    pub pgaz_lr_w: f64,
    /// « Pgaz_MA,LR(h) ».
    pub pgaz_ma_lr_w: f64,
    /// « Paux,LR(h) » (éq. 1388).
    pub paux_lr_w: f64,
    /// « Qrest,act(h) » (éq. 1378).
    pub q_rest_act_wh: f64,
}

impl PointMoteurGaz {
    /// « Pabs,LR » (éq. 1382 et 1386).
    pub fn pabs_lr_w(&self) -> f64 {
        self.pgaz_lr_w + self.pgaz_ma_lr_w
    }

    /// « GUE_LR » (éq. 1387) ; nul sans consommation.
    pub fn gue_lr(&self) -> f64 {
        let p = self.pabs_lr_w();
        if p > 0.0 {
            self.pfou_lr_w / p
        } else {
            0.0
        }
    }
}

impl PacMoteurGaz {
    /// Mode pour un poste.
    pub fn mode(&self, poste: Poste) -> Option<&ModeMoteurGaz> {
        match poste {
            Poste::Chauffage => self.chauffage.as_ref(),
            Poste::Refroidissement => self.refroidissement.as_ref(),
            Poste::Ecs => None,
        }
    }

    /// « idfougen ».
    pub fn fonction(&self) -> FonctionGenerateur {
        match (self.chauffage.is_some(), self.refroidissement.is_some()) {
            (true, true) => FonctionGenerateur::ChauffageRefroidissement,
            (false, true) => FonctionGenerateur::Refroidissement,
            _ => FonctionGenerateur::Chauffage,
        }
    }

    /// « Rdim × Pngen » : puissance fournie au point pivot du mode, en W.
    pub fn pn_gen_tot_w(&self, poste: Poste) -> f64 {
        self.mode(poste)
            .map_or(0.0, |m| self.rdim * m.matrice.pfou_pivot_w())
    }

    /// Point de fonctionnement d'une machine (§8.25.3.4).
    pub fn point(
        &self,
        poste: Poste,
        q_req_act_wh: f64,
        theta_amont_c: f64,
        theta_aval_c: f64,
    ) -> PointMoteurGaz {
        let Some(m) = self.mode(poste) else {
            return PointMoteurGaz {
                q_rest_act_wh: q_req_act_wh.max(0.0),
                ..PointMoteurGaz::default()
            };
        };
        // §8.25.3.4.1 : double interpolation ; GUE et Pabs constants sous θam,ms (Q141).
        let theta_perf = m
            .theta_am_ms_c
            .map_or(theta_amont_c, |s| theta_amont_c.max(s));
        let (gue_pc, pabs_pc, _) = m.matrice.pleine_charge(theta_perf, theta_aval_c);
        let (_, _, paux_uext_pc) = m.matrice.pleine_charge(theta_amont_c, theta_aval_c);
        // Éq. (1371).
        let paux_pc = paux_uext_pc + m.p_aux_uint_w;
        let pfou_pc = pabs_pc * gue_pc;
        // Éq. (1376) à (1378) ; le reste à fournir n'est pas négatif (Q143).
        let pfou_lr = q_req_act_wh.min(pfou_pc).max(0.0);
        let lr = if pfou_pc > 0.0 {
            pfou_lr / pfou_pc
        } else {
            0.0
        };
        let mut p = PointMoteurGaz {
            gue_pc,
            pabs_pc_w: pabs_pc,
            paux_pc_w: paux_pc,
            pfou_pc_w: pfou_pc,
            pfou_lr_w: pfou_lr,
            lr,
            q_rest_act_wh: (q_req_act_wh - pfou_pc).max(0.0),
            ..PointMoteurGaz::default()
        };
        let lr_c = self.lr_contmin;
        if pfou_lr > 0.0 && gue_pc > 0.0 {
            if self.vitesse_variable && lr >= lr_c && lr_c < 1.0 {
                // Éq. (1379) à (1381) : vitesse variable.
                let gue_lr =
                    gue_pc * (1.0 + (self.ccp_lr_contmin - 1.0) * (1.0 - lr) / (1.0 - lr_c));
                p.pgaz_lr_w = pfou_lr / gue_lr;
            } else {
                // Éq. (1372) à (1374), puis (1383) à (1385) : cycles marche-arrêt.
                let gue_contmin = self.ccp_lr_contmin * gue_pc;
                let pgaz_contmin = lr_c * pfou_pc / gue_contmin;
                let lr_cycl = lr / lr_c;
                p.pgaz_lr_w = pgaz_contmin * (1.0 - (lr_c - lr) / lr_c);
                p.pgaz_ma_lr_w = pgaz_contmin * DEQ_MIN * lr_cycl * (1.0 - lr_cycl) / DFOU0_MIN;
            }
        }
        // Éq. (1388).
        p.paux_lr_w = paux_pc.min(self.p_aux_0_w + lr / lr_c * (paux_pc - self.p_aux_0_w));
        p
    }

    /// Fonction d'appel (§8.25.3.4.6, éq. 1375 et 1392 à 1400).
    pub fn appel(&self, e: &EntreesAppel) -> SortiesAppel {
        let rdim = self.rdim;
        // Éq. (1375).
        let p = self.point(
            e.fonction,
            e.q_req_wh / rdim,
            e.theta_amont_c,
            e.theta_aval_c,
        );
        let gaz = match self.lecture {
            LectureConsommation::AvecIrreversibilites => p.pabs_lr_w(),
            LectureConsommation::Litterale => p.pgaz_lr_w,
        };
        // Éq. (1399) et (1400).
        let phi_rejet = if e.fonction == Poste::Refroidissement {
            gaz + p.pfou_lr_w
        } else {
            (gaz - p.pfou_lr_w).min(0.0)
        };
        SortiesAppel {
            energie: Energie::Gaz,
            // Éq. (1392) et (1393).
            q_cef_wh: gaz * rdim,
            q_cons_wh: gaz * rdim,
            // Éq. (1397), sans pompe de charge d'ECS.
            q_cef_aux_elec_wh: p.paux_lr_w * rdim,
            w_aux_pro_wh: p.paux_lr_w * rdim,
            // Éq. (1394) et (1395).
            q_fou_wh: p.pfou_lr_w * rdim,
            q_rest_wh: p.q_rest_act_wh * rdim,
            // Éq. (1396) et (1398).
            tau_charge: p.lr,
            eta_eff: p.gue_lr(),
            phi_rejet_wh: phi_rejet * rdim,
            ..SortiesAppel::default()
        }
    }

    /// Résout une machine saisie et validée.
    pub fn resoudre(
        s: &crate::entree::PacMoteurGazSaisie,
        rdim: f64,
    ) -> (Self, Vec<crate::resultats::ValeurParDefaut>) {
        use crate::entree::{FonctionnementMoteurGaz, StatutMesure};
        use crate::resultats::ValeurParDefaut;
        let mut defauts = Vec::new();
        let mut defaut = |champ: String, valeur: f64, regle: &str| {
            defauts.push(ValeurParDefaut {
                champ,
                valeur,
                regle: regle.to_owned(),
            });
        };
        let mut mode = |nom: &str, poste: Poste, x: &crate::entree::ModePacMoteurGaz| {
            let (g, val_util_max) = grille(x.technologie, poste);
            let saisie = PerformancesSaisiesGaz::depuis(&x.performances);
            if let PerformancesSaisiesGaz::Pivot {
                performance,
                paux_w,
                ..
            } = &saisie
            {
                if performance.is_none() {
                    defaut(
                        format!("{nom}.performances.pivot_defaut"),
                        0.8 * val_util_max,
                        "Th-BCE 2020 §8.25.3.1, tableau 230 : GUE ou EER pivot par défaut, 0,8 × \
                         la valeur utile maximale",
                    );
                }
                if paux_w.is_none() {
                    defaut(
                        format!("{nom}.performances.paux_w"),
                        AUX_PIVOT.coef_defaut,
                        "Th-BCE 2020 §8.25.3.1, éq. (1367) : auxiliaires de l'unité extérieure \
                         par défaut, 120 % de 3 % de la puissance gaz au pivot (coefficient \
                         indiqué)",
                    );
                }
            }
            let matrice = g.construire(&saisie, val_util_max, AUX_PIVOT);
            // Éq. (1369).
            let p_aux_uint_w = x.p_aux_uint_w.unwrap_or_else(|| {
                let v = 0.06 * matrice.pabs_pivot_w();
                defaut(
                    format!("{nom}.p_aux_uint_w"),
                    v,
                    "Th-BCE 2020 §8.25.3.1, éq. (1369) : unités intérieures par défaut, 6 % de \
                     la puissance gaz au pivot",
                );
                v
            });
            ModeMoteurGaz {
                matrice,
                p_aux_uint_w,
                theta_am_ms_c: (poste == Poste::Chauffage).then_some(THETA_AM_MS_C),
            }
        };
        let chauffage = s
            .chauffage
            .as_ref()
            .map(|x| mode("chauffage", Poste::Chauffage, x));
        let refroidissement = s
            .refroidissement
            .as_ref()
            .map(|x| mode("refroidissement", Poste::Refroidissement, x));
        let vitesse_variable = match s.moteur {
            Some(m) => m == FonctionnementMoteurGaz::VitesseVariable,
            None => {
                defaut(
                    "moteur".to_owned(),
                    0.0,
                    "Th-BCE 2020 §8.25.2, tableau 229 : fonctionnement en cycles marche-arrêt \
                     du moteur par défaut",
                );
                false
            }
        };
        let (lr_contmin, ccp_lr_contmin) = if !vitesse_variable {
            (1.0, 1.0)
        } else {
            match s.fonctionnement_continu {
                Some(f) => match f.statut {
                    StatutMesure::Certifie => (f.lr_contmin, f.ccp_lr_contmin),
                    StatutMesure::Justifie => {
                        ((f.lr_contmin + 0.05).min(1.0), 0.9 * f.ccp_lr_contmin)
                    }
                },
                None => {
                    defaut(
                        "fonctionnement_continu.lr_contmin".to_owned(),
                        LR_CONTMIN_DEFAUT,
                        "Th-BCE 2020 §8.25.3.1 : LRcontmin = 0,4 et CcpLRcontmin = 1,0 par \
                         défaut",
                    );
                    (LR_CONTMIN_DEFAUT, CCP_LR_CONTMIN_DEFAUT)
                }
            }
        };
        let p_aux_0_w = s.p_aux_0_w.unwrap_or_else(|| {
            defaut(
                "p_aux_0_w".to_owned(),
                P_AUX_0_DEFAUT_W,
                "Th-BCE 2020 §8.25.3.1 : Paux0 par défaut, 0,12 kW",
            );
            P_AUX_0_DEFAUT_W
        });
        (
            Self {
                chauffage,
                refroidissement,
                vitesse_variable,
                lr_contmin,
                ccp_lr_contmin,
                p_aux_0_w,
                rdim,
                lecture: LectureConsommation::default(),
            },
            defauts,
        )
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn proche(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9 * b.abs().max(1.0), "{a} ≠ {b}");
    }

    fn pivot(gue: Option<f64>, pabs_w: f64) -> PerformancesSaisiesGaz {
        PerformancesSaisiesGaz::Pivot {
            performance: gue,
            pabs_w,
            paux_w: None,
        }
    }

    #[test]
    fn matrices_air_eau_chauffage() {
        // Pivot déclaré 1,5 : min(1,2 ; 1,3) = 1,2 ; Pabs 30 kW ; auxiliaires 1,2 × 0,03 × 30 000.
        let m = CH_AIR_EAU.construire(&pivot(Some(1.5), 30_000.0), 1.3, AUX_PIVOT);
        proche(m.gue[1][3], 1.2);
        proche(m.paux_w[1][3], 1080.0);
        // Colonne 7 °C : 42,5 °C × 0,8 ; 51 °C × 0,8 × 0,8 ; 23,5 °C × 1,15 ; 60 °C × 0,8³.
        proche(m.gue[3][3], 1.2 * 0.64);
        proche(m.gue[0][3], 1.38);
        proche(m.gue[4][3], 1.2 * 0.512);
        proche(m.pabs_w[3][3], 30_000.0 * 1.17 * 1.12);
        // Ligne 32,5 °C : −7 °C × 0,4941 ; 2 °C × 0,81 ; 20 °C × 1,37 ; −15 °C = −7 °C × 0,78.
        proche(m.gue[1][1], 1.2 * 0.4941);
        proche(m.gue[1][0], 1.2 * 0.4941 * 0.78);
        proche(m.pabs_w[1][0], 30_000.0 * 1.36 * 1.07);
    }

    #[test]
    fn matrices_air_air() {
        let m = CH_AIR_AIR.construire(&pivot(None, 20_000.0), 1.4, AUX_PIVOT);
        let p = 0.8 * 1.4;
        proche(m.gue[3][3], p);
        proche(m.gue[0][3], p * 0.91);
        proche(m.paux_w[3][1], 20_000.0 * 0.036 * 0.3);
        proche(m.paux_w[3][0], 20_000.0 * 0.036 * 0.3 * 0.3);
        let m = FR_AIR_AIR.construire(&pivot(Some(1.0), 20_000.0), 1.25, AUX_PIVOT);
        proche(m.gue[1][3], 0.8);
        proche(m.gue[1][0], 0.8 * 1.3);
        proche(m.gue[3][4], 0.8 * 1.01 * 0.7);
        proche(m.pabs_w[0][3], 20_000.0 * 0.9);
    }

    fn machine(vitesse_variable: bool) -> PacMoteurGaz {
        let matrice = CH_AIR_AIR.construire(&pivot(None, 20_000.0), 1.4, AUX_PIVOT);
        PacMoteurGaz {
            chauffage: Some(ModeMoteurGaz {
                matrice,
                p_aux_uint_w: 1200.0,
                theta_am_ms_c: Some(THETA_AM_MS_C),
            }),
            refroidissement: None,
            vitesse_variable,
            lr_contmin: if vitesse_variable { 0.4 } else { 1.0 },
            ccp_lr_contmin: 1.0,
            p_aux_0_w: 120.0,
            rdim: 1.0,
            lecture: LectureConsommation::default(),
        }
    }

    #[test]
    fn charge_partielle() {
        // Pivot (7 °C ; 20 °C) : GUE 1,12, Pabs 20 kW, Pfou 22,4 kW, Paux,uext 720 W.
        let m = machine(false);
        let p = m.point(Poste::Chauffage, 11_200.0, 7.0, 20.0);
        proche(p.lr, 0.5);
        proche(p.paux_pc_w, 720.0 + 1200.0);
        // Cycles : Pgaz = 20 000 × 0,5 ; Pgaz_MA = 20 000 × Deq 0,5 × 0,5 × (1 − 0,5) / 2.
        proche(p.pgaz_lr_w, 10_000.0);
        proche(p.pgaz_ma_lr_w, 1250.0);
        proche(p.paux_lr_w, 120.0 + 0.5 * (1920.0 - 120.0));
        // Vitesse variable, LR = 0,5 ≥ 0,4, Ccp = 1 : GUE_LR = GUE_pc.
        let m = machine(true);
        let p = m.point(Poste::Chauffage, 11_200.0, 7.0, 20.0);
        proche(p.pgaz_lr_w, 10_000.0);
        assert_eq!(p.pgaz_ma_lr_w, 0.0);
        proche(p.paux_lr_w, 1920.0);
        // LR = 0,2 : cycles autour de LRcontmin = 0,4 ; LRcycl = 0,5.
        let p = m.point(Poste::Chauffage, 0.2 * 22_400.0, 7.0, 20.0);
        let contmin = 0.4 * 22_400.0 / 1.12;
        proche(p.pgaz_lr_w, contmin * 0.5);
        proche(p.pgaz_ma_lr_w, contmin * 0.5 * 0.5 * 0.5 / 2.0);
        // Sous θam,ms = −3 °C, GUE et Pabs de −3 °C ; les auxiliaires suivent θamont.
        let a = m.point(Poste::Chauffage, 1000.0, -10.0, 20.0);
        let b = m.point(Poste::Chauffage, 1000.0, -3.0, 20.0);
        proche(a.gue_pc, b.gue_pc);
        proche(a.pabs_pc_w, b.pabs_pc_w);
        assert!(a.paux_pc_w < b.paux_pc_w);
        // Sorties : gaz avec irréversibilités, électricité des auxiliaires.
        let s = machine(false).appel(&EntreesAppel {
            fonction: Poste::Chauffage,
            q_req_wh: 11_200.0,
            theta_amont_c: 7.0,
            theta_aval_c: 20.0,
            r_pui_dispo: 1.0,
            ecs_seule: false,
            theta_amb_c: 20.0,
            air_extrait_w_k: None,
        });
        proche(s.q_cef_wh, 11_250.0);
        proche(s.eta_eff, 11_200.0 / 11_250.0);
        proche(s.phi_rejet_wh, 0.0);
        proche(s.q_cef_aux_elec_wh, 1020.0);
        // Surcharge : report.
        let p = machine(false).point(Poste::Chauffage, 30_000.0, 7.0, 20.0);
        proche(p.q_rest_act_wh, 30_000.0 - 22_400.0);
    }
}
