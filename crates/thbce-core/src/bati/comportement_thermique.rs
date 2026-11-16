// SPDX-License-Identifier: GPL-3.0-or-later
//! Comportement thermique d'un groupe : fiche `C_BAT_comportement thermique d'un groupe`
//! (Th-BCE 2020 §5.21).
//!
//! Modèle dynamique horaire à trois nœuds : l'air `θi`, la surface des parois lourdes `θmq`
//! (inertie quotidienne) et un nœud de résolution `θs`.
//!
//! Périmètre : inertie quotidienne, seule prise en compte en Th-B et Th-C (§5.21.3.2). Les
//! inerties séquentielle et annuelle (§5.21.3.10, calcul Th-D) viendront avec le confort d'été ;
//! leur flux entre dans [`GroupeThermique::resoudre`] par `phi_mq_seq_ann_w`. Les échanges entre
//! groupes (`φmig`, `Hs,ig,eq` des éq. 365 et 367), que la fiche ne définit pas, sont nuls.
//!
//! Le texte ne dit pas où va la part des apports radiatifs et solaires reçue par les parois
//! légères : voir [`LectureFluxRadiatifs`] (`docs/questions.md`, Q34).

use crate::entree::InertieQuotidienne;

/// « hci » : coefficient d'échange convectif intérieur moyen, en W/(m².K) (§5.21.3.2, éq. 311).
pub const H_CI_W_M2K: f64 = 2.5;
/// « hri » : coefficient d'échange radiatif intérieur moyen, en W/(m².K) (éq. 312).
pub const H_RI_W_M2K: f64 = 5.5;
/// « hrs » : coefficient d'échange radiatif intérieur corrigé, `1,2·hri` (§5.21.3.4, éq. 325).
pub const H_RS_W_M2K: f64 = 1.2 * H_RI_W_M2K;
/// « his » : somme des coefficients d'échange convectif et radiatif, `hci + hrs` (éq. 324).
pub const H_IS_W_M2K: f64 = H_CI_W_M2K + H_RS_W_M2K;
/// Ratio de la surface interne des parois à la surface utile, `Atparois = 4,5·Agr` (éq. 313).
pub const RATIO_AT_PAROIS: f64 = 4.5;
/// « ptop » : part de la température radiante moyenne dans la température opérative (éq. 314).
pub const P_T_OP: f64 = 0.5;
/// « fsa » : part du flux solaire de courte longueur d'onde transmise directement à l'air
/// (§5.21.2).
pub const F_SA: f64 = 0.1;
/// « Ca » : chaleur massique de l'air, en J/(kg.K) (§5.21.2, constantes).
pub const C_A_J_KG_K: f64 = 1006.0;
/// « Cv » : chaleur massique de la vapeur d'eau, en J/(kg.K) (§5.21.2 : « chaleur massique de
/// l'eau », 1830).
pub const C_V_J_KG_K: f64 = 1830.0;
/// « Hfg » : chaleur latente de vaporisation de l'eau, en J/kg (§5.21.2 : 25 × 10⁵).
pub const H_FG_J_KG: f64 = 2.5e6;
/// Puissance surfacique des cas de la matrice de comportement, en W/m² (§5.21.3.14).
pub const P_MATRICE_W_M2: f64 = 10.0;

/// « Amq_surf » (m²/m²) et « Cmq_surf » (kJ/(K.m²)) d'une classe d'inertie quotidienne.
///
/// Th-BCE 2020 §5.21.3.3, tableau 53 : très légère (2,5 ; 80), légère (2,5 ; 110), moyenne
/// (2,5 ; 165), lourde (3,0 ; 260), très lourde (3,5 ; 370) ; ou valeurs saisies.
pub fn inertie_quotidienne(classe: &InertieQuotidienne) -> (f64, f64) {
    match *classe {
        InertieQuotidienne::Personnalisee {
            amq_surf_m2_m2,
            cmq_surf_kj_k_m2,
        } => (amq_surf_m2_m2, cmq_surf_kj_k_m2),
        InertieQuotidienne::TresLegere => (2.5, 80.0),
        InertieQuotidienne::Legere => (2.5, 110.0),
        InertieQuotidienne::Moyenne => (2.5, 165.0),
        InertieQuotidienne::Lourde => (3.0, 260.0),
        InertieQuotidienne::TresLourde => (3.5, 370.0),
    }
}

/// Flux d'air entrant dans le groupe par un composant aéraulique.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AirEntrant {
    /// « Qma,j » : débit massique, en kg/s.
    pub qm_kg_s: f64,
    /// « θa,j » : température, en °C.
    pub theta_c: f64,
    /// « ωa,j » : humidité spécifique, en kg/kg.
    pub omega_kg_kg: f64,
}

/// Air équivalent entrant dans le groupe.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AirEquivalent {
    /// « qm,eq » : débit massique total, en kg/s (éq. 334).
    pub qm_eq_kg_s: f64,
    /// « ωeq » : humidité spécifique équivalente, en kg/kg (éq. 335).
    pub omega_eq_kg_kg: f64,
    /// « θei,eq » : température équivalente, en °C (éq. 338).
    pub theta_ei_eq_c: f64,
    /// « Hgei » : coefficient de transfert dû au renouvellement d'air, en W/K (éq. 333).
    pub hg_ei_w_k: f64,
}

impl AirEquivalent {
    /// Mélange les flux d'air entrants.
    ///
    /// Th-BCE 2020 §5.21.3.5 :
    /// - éq. (334) `qm,eq = Σ Qma,j` ; éq. (335) `ωeq = Σ(Qma,j·ωa,j)/qm,eq` ;
    /// - éq. (337) `ha,j = (Ca + Cv·ωa,j)·θa,j + Hfg·ωa,j` ; éq. (336) `heq = Σ(Qma,j·ha,j)/qm,eq` ;
    /// - éq. (338) `θei,eq = (heq − Hfg·ωeq)/(Ca + Cv·ωeq)` ;
    /// - éq. (333) `Hgei = qm,eq·(Ca + Cv·ωeq)`.
    ///
    /// Sans débit, tout est nul.
    pub fn melange(flux: &[AirEntrant]) -> Self {
        let qm: f64 = flux.iter().map(|f| f.qm_kg_s).sum();
        if qm <= 0.0 {
            return Self::default();
        }
        let omega = flux.iter().map(|f| f.qm_kg_s * f.omega_kg_kg).sum::<f64>() / qm;
        let h = flux
            .iter()
            .map(|f| {
                f.qm_kg_s
                    * ((C_A_J_KG_K + C_V_J_KG_K * f.omega_kg_kg) * f.theta_c
                        + H_FG_J_KG * f.omega_kg_kg)
            })
            .sum::<f64>()
            / qm;
        let c = C_A_J_KG_K + C_V_J_KG_K * omega;
        Self {
            qm_eq_kg_s: qm,
            omega_eq_kg_kg: omega,
            theta_ei_eq_c: (h - H_FG_J_KG * omega) / c,
            hg_ei_w_k: qm * c,
        }
    }
}

/// Lecture de la répartition des apports radiatifs et solaires entre les nœuds.
///
/// La fiche répartit ces apports au prorata des surfaces internes : parois lourdes (`Amq`),
/// baies (`Abaies`) et, pour le reste, parois légères. Elle donne le flux reçu par les parois
/// lourdes (éq. 348) et par les baies (éq. 347), et ramène ce dernier au nœud `θs` (éq. 361).
/// Elle définit aussi `frld` (éq. 351), part du flux solaire revenant aux parois légères, et
/// cite dans sa nomenclature `frs` et `frsd`, parts « récupérées au nœud θs », mais aucune
/// équation n'injecte la part des parois légères dans le bilan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LectureFluxRadiatifs {
    /// Lecture retenue (question Q34) : la part des parois légères est injectée au nœud `θs`,
    /// comme l'annoncent `frs`, `frsd` et `frld`. Le bilan est alors fermé : tout apport
    /// radiatif ou solaire entre dans le groupe, hormis la part des baies perdue vers
    /// l'extérieur (éq. 361).
    Complete,
    /// Équations imprimées, telles quelles : seule la part des baies atteint `θs` ; la part des
    /// parois légères n'est injectée nulle part.
    TexteLitteral,
}

/// Lecture employée par le moteur. Ce n'est pas une entrée du projet.
pub const LECTURE_FLUX_RADIATIFS: LectureFluxRadiatifs = LectureFluxRadiatifs::Complete;

/// Sollicitations d'un groupe à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sollicitations {
    /// Air entrant équivalent.
    pub air: AirEquivalent,
    /// « θei » des éq. (342) et (344) : température extérieure, en °C.
    pub theta_e_c: f64,
    /// « HTH,k+pt » : parois opaques et ponts thermiques, en W/K (éq. 340).
    pub h_th_k_pt_w_k: f64,
    /// « Hges » : baies, en W/K (éq. 341).
    pub hg_es_w_k: f64,
    /// « φsl » : `Fs2 + Ftvc`, en W (éq. 343).
    pub phi_sl_w: f64,
    /// « φsh » : flux solaire et de voûte céleste des parois opaques et des ponts thermiques,
    /// en W (éq. 345).
    pub phi_sh_w: f64,
    /// « Fs1 » : flux solaire de courte longueur d'onde des baies, en W.
    pub fs1_w: f64,
    /// « Fs3 » : flux de la lame d'air intérieure ventilée des baies, en W.
    pub fs3_w: f64,
    /// Apports convectifs, en W : `φint_conv + φint,occ_conv + φrecup_conv + Fecl_c` (éq. 346).
    pub phi_conv_w: f64,
    /// Apports radiatifs, en W : `φint_rad + φint,occ_rad + φrecup_rad + Fecl_r` (éq. 347, 348).
    pub phi_rad_w: f64,
}

/// Températures du groupe, en °C.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Temperatures {
    /// « θi » : air.
    pub theta_i_c: f64,
    /// « θs » : nœud de résolution.
    pub theta_s_c: f64,
    /// « θmq » : surface des parois lourdes.
    pub theta_mq_c: f64,
    /// « θrm » : radiante moyenne.
    pub theta_rm_c: f64,
    /// « θop » : opérative.
    pub theta_op_c: f64,
}

/// Réponse du groupe sur un pas de temps (§5.21.3.14).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Reponse {
    /// Cas 1 : températures pour la température de masse moyenne sur le pas de temps.
    pub moy: Temperatures,
    /// Cas 2 : températures pour la température de masse à la fin du pas de temps.
    pub fin: Temperatures,
}

/// Grandeurs fixes du modèle thermique d'un groupe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupeThermique {
    /// « Agr » : surface utile, en m².
    pub a_gr_m2: f64,
    /// « Atparois » : surface interne totale des parois, en m² (éq. 313).
    pub a_tparois_m2: f64,
    /// « Abaies » : surface interne totale des baies, en m².
    pub a_baies_m2: f64,
    /// « Amq » : surface d'échange des parois lourdes, en m² (éq. 317).
    pub a_mq_m2: f64,
    /// « Cmq » : capacité thermique quotidienne, en kJ/K (éq. 320).
    pub c_mq_kj_k: f64,
    /// « Hgis », en W/K (éq. 323).
    pub hg_is_w_k: f64,
    /// « Hgmqs », en W/K (éq. 326).
    pub hg_mqs_w_k: f64,
}

impl GroupeThermique {
    /// Th-BCE 2020 §5.21.3.2 à 5.21.3.4 : `Atparois = 4,5·Agr` (éq. 313) ;
    /// `Amq = Amq_surf·Agr` (éq. 317) ; `Cmq = Cmq_surf·Agr` (éq. 320) ;
    /// `Hgis = Atparois/(1/hci − 1/his)` (éq. 323) ; `Hgmqs = his·Amq` (éq. 326).
    pub fn nouveau(a_gr_m2: f64, a_baies_m2: f64, inertie: &InertieQuotidienne) -> Self {
        let (amq_surf, cmq_surf) = inertie_quotidienne(inertie);
        let a_tparois_m2 = RATIO_AT_PAROIS * a_gr_m2;
        let a_mq_m2 = amq_surf * a_gr_m2;
        Self {
            a_gr_m2,
            a_tparois_m2,
            a_baies_m2,
            a_mq_m2,
            c_mq_kj_k: cmq_surf * a_gr_m2,
            hg_is_w_k: a_tparois_m2 / (1.0 / H_CI_W_M2K - 1.0 / H_IS_W_M2K),
            hg_mqs_w_k: H_IS_W_M2K * a_mq_m2,
        }
    }

    /// « Hgemq » : coefficient de transfert entre l'extérieur et la surface intérieure des
    /// parois opaques, en W/K.
    ///
    /// Th-BCE 2020 §5.21.3.6, éq. (339) : `Hgemq = 1/(1/HTH,k+pt − 1/Hgmqs)`. Nul sans paroi
    /// déperditive. `None` si `HTH,k+pt ≥ Hgmqs` : la formule n'a alors pas de sens physique
    /// (l'enveloppe serait plus conductrice que l'échange superficiel intérieur).
    pub fn hg_emq_w_k(&self, h_th_k_pt_w_k: f64) -> Option<f64> {
        if h_th_k_pt_w_k <= 0.0 {
            Some(0.0)
        } else if h_th_k_pt_w_k >= self.hg_mqs_w_k {
            None
        } else {
            Some(1.0 / (1.0 / h_th_k_pt_w_k - 1.0 / self.hg_mqs_w_k))
        }
    }

    /// Parts des apports radiatifs et du flux solaire de courte longueur d'onde reçues au nœud
    /// `θs` : `(frs, frsd)`.
    ///
    /// Th-BCE 2020 §5.21.3.9 et 5.21.3.11. Part des baies ramenée à `θs`, éq. (349) et (361) :
    /// `frl,baies·(1 − Hges/(Abaies·his))`, avec `frl,baies = Abaies/Atparois` ; elle ne porte
    /// que sur les apports radiatifs (éq. 347). Lecture [`LectureFluxRadiatifs::Complete`] :
    /// s'y ajoute la part des parois légères, `(Atparois − Amq − Abaies)/Atparois` pour les
    /// apports radiatifs et `frld = (Atparois − Amq − Abaies)/(Atparois − Abaies)` (éq. 351)
    /// pour le flux solaire.
    pub fn parts_noeud_s(&self, hg_es_w_k: f64, lecture: LectureFluxRadiatifs) -> (f64, f64) {
        let (at, amq, ab) = (self.a_tparois_m2, self.a_mq_m2, self.a_baies_m2);
        // (Abaies/Atparois)·(1 − Hges/(Abaies·his)) = (Abaies − Hges/his)/Atparois.
        let baies = if ab > 0.0 {
            (ab - hg_es_w_k / H_IS_W_M2K) / at
        } else {
            0.0
        };
        match lecture {
            LectureFluxRadiatifs::TexteLitteral => (baies, 0.0),
            LectureFluxRadiatifs::Complete => {
                (baies + (at - amq - ab) / at, (at - amq - ab) / (at - ab))
            }
        }
    }

    /// Résout le bilan thermique du groupe sur un pas de temps, avec la lecture retenue des flux
    /// radiatifs ([`LECTURE_FLUX_RADIATIFS`]).
    pub fn resoudre(
        &self,
        s: &Sollicitations,
        theta_mq_precedent_c: f64,
        phi_mq_seq_ann_w: f64,
        phi_sys_conv_w: f64,
        phi_sys_rad_w: f64,
    ) -> Option<Reponse> {
        self.resoudre_selon(
            LECTURE_FLUX_RADIATIFS,
            s,
            theta_mq_precedent_c,
            phi_mq_seq_ann_w,
            phi_sys_conv_w,
            phi_sys_rad_w,
        )
    }

    /// Résout le bilan thermique du groupe sur un pas de temps d'une heure.
    ///
    /// Th-BCE 2020 §5.21.3.9 à 5.21.3.13 :
    /// - éq. (346) `φi = fsa·Fs1 + Fs3 + apports convectifs` ;
    /// - éq. (348) `φmq = frmd·(1 − fsa)·Fs1 + frm·apports radiatifs`, avec
    ///   `frm = Amq/Atparois` (éq. 350) et `frmd = Amq/(Atparois − Abaies)` (éq. 352) ;
    /// - `φs` : voir [`Self::parts_noeud_s`] ;
    /// - éq. (372) `φsys-rad-s = frs·φsys-rad`, `φsys-rad-m = frm·φsys-rad` ;
    /// - éq. (362) `U1 = 1/(1/Hgei + 1/Hgis)` ; éq. (363) `U2 = U1 + Hges` ;
    ///   éq. (364) `U3 = 1/(1/U2 + 1/Hgmqs)` ;
    /// - éq. (342) `θes = θe + φsl/Hges` ; éq. (344) `θem = θe + φsh·(1/Hgemq + 1/Hgmqs)` ;
    /// - éq. (366) `φmtot = φmq + φsys-rad-m + Hgemq·θem + U3/U2·[φs + φsys-rad-s
    ///   + U1·((φi + φsys-conv)/Hgei + θei,eq)]` ;
    /// - éq. (365), schéma de Crank-Nicolson sur `Δh = 1 h`, `Cmq` en kJ/K :
    ///   `θmq(h) = [θmq(h−1)·(Cmq/3,6 − 0,5·(U3 + Hgemq)) + φmtot + φmq_seq + φmq_ann]
    ///   / [Cmq/3,6 + 0,5·(U3 + Hgemq)]` ;
    /// - éq. (367) `θs = [Hgmqs·θmq + Hges·θes + φs + φsys-rad-s + U1·(θei,eq
    ///   + (φi + φsys-conv)/Hgei)]/(Hgmqs + Hges + U1)` ;
    /// - éq. (368) `θi = [Hgis·θs + Hgei·θei,eq + φi + φsys-conv]/(Hgis + Hgei)` ;
    /// - éq. (369) et (370) `θrm = ptrm·θs + (1 − ptrm)·θi`, `ptrm = 1 + hci/hrs` ;
    /// - éq. (371) `θop = ptop·θrm + (1 − ptop)·θi`.
    ///
    /// Les produits `Hges·θes`, `Hgemq·θem` et `U1·(φ/Hgei)` sont développés
    /// (`Hges·θe + φsl`, `Hgemq·θe + φsh·Hgemq/HTH,k+pt`, `φ·Hgis/(Hgei + Hgis)`) : les formules
    /// restent ainsi définies sans baie, sans paroi déperditive ou sans débit d'air.
    ///
    /// Le « cas 1 » (§5.21.3.14) emploie la température de masse moyenne sur le pas de temps,
    /// que le texte ne définit pas : `(θmq(h−1) + θmq(h))/2`, par cohérence avec le schéma de
    /// Crank-Nicolson (question Q34).
    ///
    /// `phi_mq_seq_ann_w` est `φmq_seq + φmq_ann`, nul en Th-B et Th-C. Renvoie `None` si
    /// `HTH,k+pt ≥ Hgmqs` (voir [`Self::hg_emq_w_k`]).
    pub fn resoudre_selon(
        &self,
        lecture: LectureFluxRadiatifs,
        s: &Sollicitations,
        theta_mq_precedent_c: f64,
        phi_mq_seq_ann_w: f64,
        phi_sys_conv_w: f64,
        phi_sys_rad_w: f64,
    ) -> Option<Reponse> {
        let hg_emq = self.hg_emq_w_k(s.h_th_k_pt_w_k)?;
        let (hg_ei, hg_is, hg_ms, hg_es) = (
            s.air.hg_ei_w_k,
            self.hg_is_w_k,
            self.hg_mqs_w_k,
            s.hg_es_w_k,
        );
        let (at, amq, ab) = (self.a_tparois_m2, self.a_mq_m2, self.a_baies_m2);

        // Flux aux nœuds.
        let fr_m = amq / at;
        let fr_md = amq / (at - ab);
        let (fr_s, fr_sd) = self.parts_noeud_s(hg_es, lecture);
        let solaire_clo = (1.0 - F_SA) * s.fs1_w;
        let phi_i = F_SA * s.fs1_w + s.fs3_w + s.phi_conv_w + phi_sys_conv_w;
        let phi_s = fr_sd * solaire_clo + fr_s * (s.phi_rad_w + phi_sys_rad_w);
        let phi_mq = fr_md * solaire_clo + fr_m * (s.phi_rad_w + phi_sys_rad_w);

        // Coefficients de transfert.
        let u1 = hg_ei * hg_is / (hg_ei + hg_is);
        let u2 = u1 + hg_es;
        // U3 = 1/(1/U2 + 1/Hgmqs) = U2·Hgmqs/(U2 + Hgmqs) : forme définie quand U2 est nul (ni
        // débit d'air ni baie).
        let u3_sur_u2 = hg_ms / (u2 + hg_ms);
        let u3 = u2 * u3_sur_u2;
        // U1·(θei,eq + φi/Hgei), développé.
        let air = u1 * s.air.theta_ei_eq_c + phi_i * hg_is / (hg_ei + hg_is);
        // Hges·θes et Hgemq·θem, développés.
        let baies = hg_es * s.theta_e_c + s.phi_sl_w;
        let opaques = if hg_emq > 0.0 {
            hg_emq * s.theta_e_c + s.phi_sh_w * hg_emq / s.h_th_k_pt_w_k
        } else {
            0.0
        };

        // Température de masse, éq. (365) et (366).
        let phi_m_tot = phi_mq + opaques + u3_sur_u2 * (phi_s + baies + air);
        let c = self.c_mq_kj_k / 3.6;
        let demi = 0.5 * (u3 + hg_emq);
        let theta_mq_fin =
            (theta_mq_precedent_c * (c - demi) + phi_m_tot + phi_mq_seq_ann_w) / (c + demi);
        let theta_mq_moy = 0.5 * (theta_mq_precedent_c + theta_mq_fin);

        let temperatures = |theta_mq_c: f64| {
            let theta_s_c = (hg_ms * theta_mq_c + baies + phi_s + air) / (hg_ms + hg_es + u1);
            let theta_i_c =
                (hg_is * theta_s_c + hg_ei * s.air.theta_ei_eq_c + phi_i) / (hg_is + hg_ei);
            let p_t_rm = 1.0 + H_CI_W_M2K / H_RS_W_M2K;
            let theta_rm_c = p_t_rm * theta_s_c + (1.0 - p_t_rm) * theta_i_c;
            Temperatures {
                theta_i_c,
                theta_s_c,
                theta_mq_c,
                theta_rm_c,
                theta_op_c: P_T_OP * theta_rm_c + (1.0 - P_T_OP) * theta_i_c,
            }
        };
        Some(Reponse {
            moy: temperatures(theta_mq_moy),
            fin: temperatures(theta_mq_fin),
        })
    }

    /// Matrice de comportement thermique du groupe : évolution libre, puis 10 W/m² émis sous
    /// forme convective, puis 10 W/m² émis sous forme radiative.
    ///
    /// Th-BCE 2020 §5.21.3.14, éq. (373).
    pub fn matrice(
        &self,
        s: &Sollicitations,
        theta_mq_precedent_c: f64,
        phi_mq_seq_ann_w: f64,
    ) -> Option<[Reponse; 3]> {
        let p = P_MATRICE_W_M2 * self.a_gr_m2;
        let cas = |conv, rad| self.resoudre(s, theta_mq_precedent_c, phi_mq_seq_ann_w, conv, rad);
        Some([cas(0.0, 0.0)?, cas(p, 0.0)?, cas(0.0, p)?])
    }
}
