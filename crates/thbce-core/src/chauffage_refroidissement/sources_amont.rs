// SPDX-License-Identifier: GPL-3.0-or-later
//! Sources amont des générateurs thermodynamiques : fiches `C_Gen_Sources amont des générateurs
//! thermodynamiques` (Th-BCE 2020 §8.26) et `C_Gen_Sources amont des générateurs
//! thermodynamiques de type sol` (§8.27).
//!
//! Sources sur air (§8.26.3.3.1) : température amont (air extérieur, éq. 1405 ; air ambiant
//! d'un volume non chauffé, éq. 1406 ; air extrait, éq. 1407), auxiliaires amont des machines
//! gainées (éq. 1441 à 1443) et puissance maximale échangeable avec une source sur air extrait
//! (éq. 1453 à 1457, appliquée par la fiche 8.23, éq. 1291 et 1292).
//!
//! Sources sur eau (§8.26.3.3.2) : captage par sondes (éq. 1408 à 1410), nappe avec ou sans
//! échangeur de barrage (éq. 1411 à 1426), température amont (éq. 1427), tours de
//! refroidissement humides et sèches (éq. 1428 à 1436), boucle d'eau à température
//! sinusoïdale (éq. 1437 à 1439) ; pompes et ventilateurs (éq. 1444 à 1451). Source sur sol
//! (§8.27) : éq. 1461 à 1466 et tableau 245.
//!
//! Dans cette version, chaque générateur a sa propre source amont : les ratios de répartition
//! entre générateurs d'une même source (éq. 1402 à 1404, 1452, 1458 à 1460 et 1466) valent 1,
//! et la puissance rejetée de la source est celle du générateur (lecture Q149). La source
//! « boucle d'eau » à température sinusoïdale du §8.26.3.3.2.3 est aussi celle des pompes à
//! chaleur d'une génération à boucle d'eau (fiche 8.15, lot 2f), dont le bilan est fait par
//! `transferts_entre_generateurs_sur_boucle.rs` (lecture Q240).

use crate::entree::{EcoulementsEchangeur, GestionPompesCaptage, NatureTourRefroidissement};

/// « Cpa » : capacité thermique massique de l'air, en J/(kg.K).
pub const C_PA_J_KG_K: f64 = crate::ventilation::mecanique_sf::C_PA_J_KG_K;

/// Source amont d'un générateur thermodynamique : sur air (« idamont-air-type », §8.26.3.3.1),
/// sur eau (« idamont-eau-type », §8.26.3.3.2) ou sur sol (§8.27).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SourceAmont {
    /// 1 : air extérieur (éq. 1405).
    AirExterieur,
    /// 2 : air ambiant d'un volume non chauffé, espace tampon d'indice donné (éq. 1406).
    AirAmbiant {
        /// Indice de l'espace tampon dans le bâtiment.
        espace_tampon: usize,
    },
    /// 3 : air extrait par le système de ventilation d'une zone (éq. 1407).
    AirExtrait {
        /// Indice de la zone dont le système de ventilation irrigue la source.
        zone: usize,
    },
    /// Eau, 1 : captage par sondes (éq. 1408 à 1410).
    Captage {
        /// Température de l'interface avec le sol.
        sol: Sinusoide,
        /// « Rb / L », en K/W.
        rb_sur_l_k_w: f64,
        /// Pompes de captage.
        pompes: Pompes,
        /// Écarts aux bornes de l'échangeur.
        ecarts: Ecarts,
    },
    /// Eau, 4 et 5 : eau de nappe avec ou sans échangeur de barrage (éq. 1411 à 1426).
    Nappe {
        /// Température de l'eau de nappe.
        nappe: Sinusoide,
        /// Échangeur de barrage ; `None` sans échangeur.
        echangeur: Option<Echangeur>,
        /// Pompes de captage et du circuit intermédiaire.
        pompes: Pompes,
        /// Écarts aux bornes de l'échangeur.
        ecarts: Ecarts,
    },
    /// Eau, 2 : tour de refroidissement (éq. 1428 à 1436).
    Tour(Tour),
    /// Eau, 3 : boucle d'eau à température sinusoïdale (éq. 1437 à 1439).
    Boucle {
        /// Température de l'eau de la boucle.
        boucle: Sinusoide,
        /// Pompes de boucle.
        pompes: Pompes,
        /// Écarts aux bornes de l'échangeur.
        ecarts: Ecarts,
    },
    /// Sol (fiche 8.27, éq. 1461 à 1465).
    Sol {
        /// Température de l'interface entre le sol et les tubes.
        sol: Sinusoide,
        /// « Rb / L », en K/W.
        rb_sur_l_k_w: f64,
    },
}

/// « Cpe » : chaleur massique de l'eau, en J/(kg.K) (tableau 243) ; elle sert aussi de
/// « Ce » dans les éq. (1428) à (1431) (lecture Q152).
pub const C_PE_J_KG_K: f64 = 4180.0;
/// « ρeau » : masse volumique de l'eau, en kg/m³ (tableau 243).
pub const RHO_EAU_KG_M3: f64 = 1000.0;
/// « Cv » : chaleur massique de la vapeur d'eau, en J/(kg.K) (tableau 243).
pub const C_V_J_KG_K: f64 = 1830.0;
/// « Ca » : chaleur massique de l'air, en J/(kg.K) (tableau 243).
pub const C_A_J_KG_K: f64 = 1006.0;
/// « Hfg » : chaleur latente de vaporisation de l'eau, en J/kg (tableau 243).
pub const H_FG_J_KG: f64 = 2.5e6;
/// « Tdépart_amont » au premier pas de temps d'une saison, en °C (§8.26.3.3.2.1, note).
pub const T_DEPART_INITIAL_C: f64 = 12.0;
/// « ΔθEvap_CH » et « ΔθCond_FR » par défaut, en K (§8.26.3.3.2.1).
pub const ECART_DEFAUT_K: f64 = 5.0;
/// Tableau 245 : températures de sol « θf » et températures de bain d'essai équivalentes
/// « θamont », en °C.
pub const TABLEAU_245: [(f64, f64); 4] = [(-5.0, -4.0), (0.0, 1.5), (5.0, 4.0), (10.0, 6.5)];

/// Évolution annuelle sinusoïdale d'une température (éq. 1408, 1409, 1437, 1438, 1461 et
/// 1462).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sinusoide {
    /// « θmin », en °C.
    pub theta_min_c: f64,
    /// « θmax », en °C.
    pub theta_max_c: f64,
    /// « idmois mini », de 1 à 12.
    pub mois_minimum: u8,
}

impl Sinusoide {
    /// Température du mois `id_mois` (de 1 à 12), en °C.
    // Th-BCE 2020 §8.26.3.3.2.1, éq. (1408) et (1409) ; §8.26.3.3.2.3, éq. (1437) et (1438) ;
    // §8.27.3.2, éq. (1461) et (1462).
    pub fn theta_c(&self, id_mois: u8) -> f64 {
        use core::f64::consts::PI;
        let a = (self.theta_min_c + self.theta_max_c) / 2.0;
        let b = (self.theta_max_c - self.theta_min_c) / 2.0;
        let phi = PI * (1.5 - f64::from(self.mois_minimum) / 6.0);
        a + b * libm::sin(2.0 * PI * f64::from(id_mois) / 12.0 + phi)
    }
}

/// Pompes d'une source sur eau (§8.26.3.4.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pompes {
    /// « idgest_captage » et « idgest_pompe_captage_cont_var ».
    pub gestion: GestionPompesCaptage,
    /// « Ppompes_captage », en W.
    pub p_captage_w: f64,
    /// « Ppompes_inter », en W (nappe avec échangeur ; nulle sinon).
    pub p_inter_w: f64,
}

impl Pompes {
    /// Facteur de marche des pompes au pas de temps : 1 en marche permanente pendant la
    /// saison, 0 à charge nulle, 1 en tout ou rien, `max(τcharge ; 0,3)` à vitesse variable.
    // Th-BCE 2020 §8.26.3.3.2.1, éq. (1411) à (1413) ; §8.26.3.4.2, éq. (1444) à (1447).
    pub fn facteur(&self, tau_charge: f64, en_saison: bool) -> f64 {
        match self.gestion {
            GestionPompesCaptage::MarchePermanente => {
                if en_saison {
                    1.0
                } else {
                    0.0
                }
            }
            _ if tau_charge <= 0.0 => 0.0,
            GestionPompesCaptage::DemandeToutOuRien => 1.0,
            GestionPompesCaptage::DemandeVitesseVariable => tau_charge.max(0.3),
        }
    }
}

/// « ΔθEvap_CH » (ou « ΔθCond_CH ») et « ΔθCond_FR », en K.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ecarts {
    /// Écart en chauffage.
    pub ch_k: f64,
    /// Écart en refroidissement.
    pub fr_k: f64,
}

impl Ecarts {
    /// « ΔθCond » selon le signe de la puissance rejetée au pas précédent : `−ΔθCond_CH` en
    /// chauffage (rejet négatif), `ΔθCond_FR` en refroidissement (rejet positif), 0 sans rejet
    /// (lecture Q146).
    // Th-BCE 2020 §8.26.3.3.2.1, éq. (1427) ; §8.26.3.3.2.3, éq. (1439).
    pub fn delta_cond_k(&self, phi_rejet_prec_w: f64) -> f64 {
        if phi_rejet_prec_w < 0.0 {
            -self.ch_k
        } else if phi_rejet_prec_w > 0.0 {
            self.fr_k
        } else {
            0.0
        }
    }
}

/// Échangeur de barrage d'une source sur eau de nappe (§8.26.3.3.2.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Echangeur {
    /// « Qv_nappe_nom », en m³/h.
    pub qv_nappe_m3_h: f64,
    /// « Qv_inter_nom », en m³/h.
    pub qv_inter_m3_h: f64,
    /// « ρinter », en kg/m³.
    pub rho_inter_kg_m3: f64,
    /// « Cpe_inter », en J/(kg.K).
    pub cp_inter_j_kg_k: f64,
    /// « UA », en W/K.
    pub ua_w_k: f64,
    /// « type_echangeur ».
    pub ecoulements: EcoulementsEchangeur,
}

/// « ε » : efficacité de l'échangeur de barrage pour un nombre d'unités de transfert `nut` et
/// un rapport des débits énergétiques `c` (de 0 à 1).
// Th-BCE 2020 §8.26.3.3.2.1, éq. (1418) à (1421).
pub fn efficacite_echangeur(ecoulements: EcoulementsEchangeur, nut: f64, c: f64) -> f64 {
    match ecoulements {
        EcoulementsEchangeur::ContreCourant => {
            if (c - 1.0).abs() < 1e-12 {
                nut / (nut + 1.0)
            } else {
                let e = libm::exp(-nut * (1.0 - c));
                (1.0 - e) / (1.0 - c * e)
            }
        }
        EcoulementsEchangeur::CourantsParalleles => (1.0 - libm::exp(-nut * (1.0 + c))) / (1.0 + c),
        EcoulementsEchangeur::CourantsCroises => {
            1.0 / (1.0 / (1.0 - libm::exp(-nut)) + c / (1.0 - libm::exp(-nut * c)) - 1.0 / nut)
        }
    }
}

/// Tour de refroidissement (§8.26.3.3.2.2 et §8.26.3.4.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tour {
    /// « idtour ».
    pub nature: NatureTourRefroidissement,
    /// « Δθtour », en K.
    pub delta_tour_k: f64,
    /// « θes_tour_consigne », en °C.
    pub consigne_c: f64,
    /// « Ppompes_tour », en W.
    pub p_pompes_w: f64,
    /// « Pvent_tour », en W.
    pub p_vent_w: f64,
    /// « ΔθCond_FR », en K.
    pub delta_cond_fr_k: f64,
}

impl Tour {
    /// « θamont(h) » : température amont d'un générateur sur tour de refroidissement, en °C.
    // Th-BCE 2020 §8.26.3.3.2.2, éq. (1432) à (1436).
    pub fn theta_amont_c(&self, theta_ext_c: f64, w_ext_kg_kg: f64) -> f64 {
        self.theta_es_tour_c(theta_ext_c, w_ext_kg_kg) + self.delta_cond_fr_k / 2.0
    }

    /// « θes_tour » : température de l'eau en sortie de tour, en °C ; reprise par la tour d'une
    /// boucle d'eau (fiche 8.15, éq. 972).
    // Th-BCE 2020 §8.26.3.3.2.2, éq. (1432) à (1435).
    pub fn theta_es_tour_c(&self, theta_ext_c: f64, w_ext_kg_kg: f64) -> f64 {
        let theta_cont = match self.nature {
            NatureTourRefroidissement::Humide => {
                temperature_humide_c(theta_ext_c, w_ext_kg_kg) + self.delta_tour_k
            }
            NatureTourRefroidissement::Seche => theta_ext_c + self.delta_tour_k,
        };
        theta_cont.max(self.consigne_c)
    }
}

/// « ωsat » à la température `theta_c` (première ligne de l'éq. 1428), en kg/kg.
fn w_sat_kg_kg(theta_c: f64) -> f64 {
    1e-3 * libm::exp(18.8161 - 4110.34 / (theta_c + 235.0))
}

/// « θ'as » : température humide de l'air extérieur, en °C ; l'équation implicite (1429) sur
/// `ωsat` est résolue par dichotomie entre `ωext` et `ωsat(θext)` (60 itérations), puis
/// `θ'as` est donnée par l'éq. (1431). Air saturé ou sursaturé : `θext`.
// Th-BCE 2020 §8.26.3.3.2.2.1, éq. (1428) à (1431).
pub fn temperature_humide_c(theta_ext_c: f64, w_ext_kg_kg: f64) -> f64 {
    let (ca, cv, ce, hfg) = (C_A_J_KG_K, C_V_J_KG_K, C_PE_J_KG_K, H_FG_J_KG);
    // Éq. (1430).
    let h_ext = ca * theta_ext_c + w_ext_kg_kg * (hfg + cv * theta_ext_c);
    // Éq. (1431) : température humide pour une humidité de saturation donnée.
    let theta = |w: f64| (w * hfg - h_ext) / (ce * (w - w_ext_kg_kg) - ca - cv * w);
    // Éq. (1429).
    let f = |w: f64| w - w_sat_kg_kg(theta(w));
    let (mut a, mut b) = (w_ext_kg_kg, w_sat_kg_kg(theta_ext_c));
    if b <= a {
        return theta_ext_c;
    }
    for _ in 0..60 {
        let m = 0.5 * (a + b);
        if f(m) > 0.0 {
            b = m;
        } else {
            a = m;
        }
    }
    theta(0.5 * (a + b))
}

/// « θamont » d'une source sur sol : correspondance entre la température de sol corrigée et la
/// température de bain d'essai, par interpolation linéaire dans le tableau 245 ; hors du
/// tableau, la valeur de la borne est conservée (lecture Q148).
// Th-BCE 2020 §8.27.3.2, tableau 245.
pub fn theta_bain_essai_c(theta_f_c: f64) -> f64 {
    let t = &TABLEAU_245;
    if theta_f_c <= t[0].0 {
        return t[0].1;
    }
    for k in 1..t.len() {
        if theta_f_c <= t[k].0 {
            let c = (theta_f_c - t[k - 1].0) / (t[k].0 - t[k - 1].0);
            return t[k - 1].1 + c * (t[k].1 - t[k - 1].1);
        }
    }
    t[t.len() - 1].1
}

/// Vrai si un générateur est dans sa saison de fonctionnement : saison de chauffage pour un
/// générateur qui chauffe, de refroidissement pour un générateur qui refroidit, toute l'année
/// pour un générateur qui produit de l'ECS (lecture Q151). Hors saison, le composant source
/// amont n'est pas appelé (§8.26.3.4.2.2, note).
pub fn generateur_en_saison(
    fonction: super::generation::FonctionGenerateur,
    aut_ch: bool,
    aut_fr: bool,
) -> bool {
    use super::calculs_generation::Poste;
    (fonction.assure(Poste::Chauffage) && aut_ch)
        || (fonction.assure(Poste::Refroidissement) && aut_fr)
        || fonction.assure(Poste::Ecs)
}

/// État d'une source amont d'un pas de temps au suivant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EtatSourceAmont {
    /// « Tdépart_amont(h − 1) », en °C (nappe avec échangeur).
    pub t_depart_c: f64,
    /// Vrai si la source était en saison au pas précédent.
    pub en_saison: bool,
}

impl Default for EtatSourceAmont {
    fn default() -> Self {
        Self {
            t_depart_c: T_DEPART_INITIAL_C,
            en_saison: false,
        }
    }
}

/// Conditions d'un pas de temps vues par une source sur eau ou sur sol.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionsEauSol {
    /// « idMois(j) », de 1 à 12.
    pub id_mois: u8,
    /// « θext(h) », en °C.
    pub theta_ext_c: f64,
    /// « ωext(h) », en kg/kg.
    pub w_ext_kg_kg: f64,
    /// « Φrejet(h − 1) » : puissance rejetée par le générateur au pas précédent, en W
    /// (positive en refroidissement, négative en chauffage et en ECS).
    pub phi_rejet_prec_w: f64,
    /// « τcharge(h − 1) » : taux de charge du générateur au pas précédent, qui fixe les
    /// débits de la nappe à la demande (lecture Q147).
    pub tau_charge_prec: f64,
    /// Vrai si le générateur est en saison (un de ses postes est actif).
    pub en_saison: bool,
}

impl SourceAmont {
    /// Vrai pour une source sur eau ou sur sol, dont la température dépend de la saison et du
    /// pas précédent.
    pub const fn eau_ou_sol(&self) -> bool {
        !matches!(
            self,
            Self::AirExterieur | Self::AirAmbiant { .. } | Self::AirExtrait { .. }
        )
    }

    /// « θamont(h) » d'une source sur eau ou sur sol, en °C ; met à jour `etat`. Au premier pas
    /// d'une saison, `Tdépart_amont` vaut 12 °C et les rejets sont nuls (§8.26.3.3.2.1, note).
    /// `None` pour une source sur air.
    // Th-BCE 2020 §8.26.3.3.2, éq. (1408) à (1440) ; §8.27.3.2, éq. (1461) à (1464).
    pub fn theta_amont_eau_sol_c(
        &self,
        etat: &mut EtatSourceAmont,
        c: &ConditionsEauSol,
    ) -> Option<f64> {
        let debut_saison = c.en_saison && !etat.en_saison;
        etat.en_saison = c.en_saison;
        if debut_saison {
            etat.t_depart_c = T_DEPART_INITIAL_C;
        }
        let phi_prec = if debut_saison {
            0.0
        } else {
            c.phi_rejet_prec_w
        };
        let tau_prec = if debut_saison { 0.0 } else { c.tau_charge_prec };
        Some(match *self {
            Self::AirExterieur | Self::AirAmbiant { .. } | Self::AirExtrait { .. } => return None,
            // Éq. (1410) et (1427).
            Self::Captage {
                sol,
                rb_sur_l_k_w,
                ecarts,
                ..
            } => {
                let theta_f = sol.theta_c(c.id_mois) + phi_prec * rb_sur_l_k_w;
                theta_f + ecarts.delta_cond_k(phi_prec) / 2.0
            }
            Self::Nappe {
                nappe,
                echangeur,
                pompes,
                ecarts,
            } => {
                let theta_b = nappe.theta_c(c.id_mois);
                let theta_f = match echangeur {
                    // Éq. (1425) et (1426).
                    None => theta_b,
                    Some(x) => nappe_avec_echangeur(
                        &x,
                        pompes.facteur(tau_prec, c.en_saison),
                        theta_b,
                        phi_prec,
                        &mut etat.t_depart_c,
                    ),
                };
                // Éq. (1427).
                theta_f + ecarts.delta_cond_k(phi_prec) / 2.0
            }
            // Éq. (1428) à (1436).
            Self::Tour(t) => t.theta_amont_c(c.theta_ext_c, c.w_ext_kg_kg),
            // Éq. (1437) à (1439).
            Self::Boucle { boucle, ecarts, .. } => {
                boucle.theta_c(c.id_mois) + ecarts.delta_cond_k(phi_prec) / 2.0
            }
            // Éq. (1463) et tableau 245.
            Self::Sol { sol, rb_sur_l_k_w } => {
                theta_bain_essai_c(sol.theta_c(c.id_mois) + phi_prec * rb_sur_l_k_w)
            }
        })
    }
}

/// Traduit une source amont saisie sur eau ou sur sol ; `None` pour une source sur air. Les
/// écarts « ΔθEvap_CH » et « ΔθCond_FR » absents valent 5 K (§8.26.3.3.2.1), signalés dans
/// `defauts`.
pub fn resoudre_eau_sol(
    saisie: &crate::entree::SourceAmontSaisie,
    defauts: &mut Vec<crate::resultats::ValeurParDefaut>,
) -> Option<SourceAmont> {
    use crate::entree::{
        EcartsSourceEau, PompesCaptage, SourceAmontSaisie as S, TemperatureSinusoidale,
    };
    use crate::resultats::ValeurParDefaut;
    let sinus = |t: &TemperatureSinusoidale| Sinusoide {
        theta_min_c: t.theta_min_c,
        theta_max_c: t.theta_max_c,
        mois_minimum: t.mois_minimum,
    };
    let ecart = |defauts: &mut Vec<ValeurParDefaut>, nom: &str, v: Option<f64>| {
        v.unwrap_or_else(|| {
            defauts.push(ValeurParDefaut {
                champ: format!("source_amont.{nom}"),
                valeur: ECART_DEFAUT_K,
                regle: "Th-BCE 2020 §8.26.3.3.2.1 : ΔθEvap_CH et ΔθCond_FR fixés par défaut à 5 K"
                    .to_owned(),
            });
            ECART_DEFAUT_K
        })
    };
    let ecarts = |defauts: &mut Vec<ValeurParDefaut>, nom: &str, e: Option<EcartsSourceEau>| {
        let e = e.unwrap_or(EcartsSourceEau {
            delta_theta_ch_k: None,
            delta_theta_fr_k: None,
        });
        Ecarts {
            ch_k: ecart(
                defauts,
                &format!("{nom}.ecarts.delta_theta_ch_k"),
                e.delta_theta_ch_k,
            ),
            fr_k: ecart(
                defauts,
                &format!("{nom}.ecarts.delta_theta_fr_k"),
                e.delta_theta_fr_k,
            ),
        }
    };
    let pompes = |p: &PompesCaptage, p_inter_w: f64| Pompes {
        gestion: p.gestion,
        p_captage_w: p.p_pompes_w,
        p_inter_w,
    };
    Some(match saisie {
        S::AirExterieur { .. } | S::AirAmbiant { .. } | S::AirExtrait { .. } => return None,
        S::Captage {
            sol,
            rb_k_m_w,
            l_m,
            pompes: p,
            ecarts: e,
        } => SourceAmont::Captage {
            sol: sinus(sol),
            rb_sur_l_k_w: rb_k_m_w / l_m,
            pompes: pompes(p, 0.0),
            ecarts: ecarts(defauts, "captage", *e),
        },
        S::Nappe {
            nappe,
            pompes: p,
            echangeur,
            ecarts: e,
        } => SourceAmont::Nappe {
            nappe: sinus(nappe),
            echangeur: echangeur.map(|x| Echangeur {
                qv_nappe_m3_h: x.qv_nappe_nom_m3_h,
                qv_inter_m3_h: x.qv_inter_nom_m3_h,
                rho_inter_kg_m3: x.rho_inter_kg_m3,
                cp_inter_j_kg_k: x.cp_inter_j_kg_k,
                ua_w_k: x.ua_w_k,
                ecoulements: x.ecoulements,
            }),
            pompes: pompes(p, echangeur.map_or(0.0, |x| x.p_pompes_inter_w)),
            ecarts: ecarts(defauts, "nappe", *e),
        },
        S::TourDeRefroidissement {
            nature,
            delta_theta_tour_k,
            theta_es_consigne_c,
            p_pompes_tour_w,
            p_vent_tour_w,
            delta_theta_cond_fr_k,
        } => SourceAmont::Tour(Tour {
            nature: *nature,
            delta_tour_k: *delta_theta_tour_k,
            consigne_c: *theta_es_consigne_c,
            p_pompes_w: *p_pompes_tour_w,
            p_vent_w: *p_vent_tour_w,
            delta_cond_fr_k: ecart(
                defauts,
                "tour_de_refroidissement.delta_theta_cond_fr_k",
                *delta_theta_cond_fr_k,
            ),
        }),
        S::BoucleDEau {
            boucle,
            pompes: p,
            ecarts: e,
        } => SourceAmont::Boucle {
            boucle: sinus(boucle),
            pompes: pompes(p, 0.0),
            ecarts: ecarts(defauts, "boucle_d_eau", *e),
        },
        S::Sol { sol, rb_k_m_w, l_m } => SourceAmont::Sol {
            sol: sinus(sol),
            rb_sur_l_k_w: rb_k_m_w / l_m,
        },
    })
}

/// « Tretour_amont » d'une nappe avec échangeur de barrage, en °C ; met à jour
/// `Tdépart_amont`. `facteur` est le facteur de marche des pompes (débits réels, éq. 1411 à
/// 1413). Débits nuls : `Tdépart_amont` est conservé (éq. 1414) et l'échangeur n'échange pas,
/// `Tretour_amont = Tdépart_amont(h − 1)` (lecture Q147).
// Th-BCE 2020 §8.26.3.3.2.1, éq. (1411) à (1424).
pub fn nappe_avec_echangeur(
    x: &Echangeur,
    facteur: f64,
    theta_b_c: f64,
    phi_rejet_prec_w: f64,
    t_depart_c: &mut f64,
) -> f64 {
    // Éq. (1411) à (1413).
    let qm_nappe = x.qv_nappe_m3_h * RHO_EAU_KG_M3 * facteur / 3600.0;
    let qm_inter = x.qv_inter_m3_h * x.rho_inter_kg_m3 * facteur / 3600.0;
    if qm_nappe <= 0.0 || qm_inter <= 0.0 {
        return *t_depart_c;
    }
    // Éq. (1415) à (1417).
    let c_nappe = qm_nappe * C_PE_J_KG_K;
    let c_inter = qm_inter * x.cp_inter_j_kg_k;
    let (c_min, c_max) = (c_nappe.min(c_inter), c_nappe.max(c_inter));
    let nut = x.ua_w_k / c_min;
    // Éq. (1418) à (1421).
    let eps = efficacite_echangeur(x.ecoulements, nut, c_min / c_max);
    // Éq. (1422) et (1423).
    let t_retour = *t_depart_c + eps * (theta_b_c - *t_depart_c) * c_min / c_inter;
    // Éq. (1424) : départ pour le pas suivant.
    *t_depart_c = t_retour + phi_rejet_prec_w / c_inter;
    t_retour
}

/// Source amont d'un générateur et ses auxiliaires.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceAmontGenerateur {
    /// Nature de la source.
    pub source: SourceAmont,
    /// « Pvent_gainé » : puissance des ventilateurs nécessaire pour vaincre les pertes de charge
    /// des conduits d'une machine gainée, en W ; nulle pour une machine non gainée (éq. 1442).
    pub p_vent_gaine_w: f64,
}

impl SourceAmontGenerateur {
    /// « Waux,am » : consommation des auxiliaires amont au pas de temps, en Wh, pour le taux de
    /// charge du générateur (éq. 1441 à 1443), hors des pompes en marche permanente des
    /// sources sur eau (voir [`Self::w_aux_am_saison_wh`]).
    pub fn w_aux_am_wh(&self, tau_charge: f64) -> f64 {
        self.w_aux_am_saison_wh(tau_charge, tau_charge > 0.0)
    }

    /// « Waux,am » au pas de temps, en Wh ; `en_saison` est vrai pendant la saison de
    /// fonctionnement du générateur (pompes en marche permanente).
    // Th-BCE 2020 §8.26.3.4, éq. (1441) à (1451) ; §8.27.3.2.2, éq. (1465).
    pub fn w_aux_am_saison_wh(&self, tau_charge: f64, en_saison: bool) -> f64 {
        match self.source {
            // Éq. (1442) et (1443).
            SourceAmont::AirExterieur
            | SourceAmont::AirAmbiant { .. }
            | SourceAmont::AirExtrait { .. } => self.p_vent_gaine_w * tau_charge,
            // Éq. (1444) à (1448).
            SourceAmont::Captage { pompes, .. }
            | SourceAmont::Nappe { pompes, .. }
            | SourceAmont::Boucle { pompes, .. } => {
                (pompes.p_captage_w + pompes.p_inter_w) * pompes.facteur(tau_charge, en_saison)
            }
            // Éq. (1449) à (1451).
            SourceAmont::Tour(t) => (t.p_pompes_w + t.p_vent_w) * tau_charge,
            // Éq. (1465).
            SourceAmont::Sol { .. } => 0.0,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn auxiliaires_amont() {
        // Machine non gainée : auxiliaires compris dans la performance (éq. 1442).
        let s = SourceAmontGenerateur {
            source: SourceAmont::AirExterieur,
            p_vent_gaine_w: 0.0,
        };
        assert_eq!(s.w_aux_am_wh(0.8), 0.0);
        // Machine gainée : 30 W à pleine charge, au prorata de la charge (éq. 1443).
        let s = SourceAmontGenerateur {
            source: SourceAmont::AirExterieur,
            p_vent_gaine_w: 30.0,
        };
        assert_eq!(s.w_aux_am_wh(0.5), 15.0);
    }

    fn proche(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} ≠ {b}");
    }

    #[test]
    fn sinusoide_minimum_au_mois_donne() {
        // A = 11, B = 3, φ = 7π/6 : au mois 2, 2π·2/12 + 7π/6 = 3π/2 (minimum) ; au mois 8,
        // 5π/2 (maximum).
        let s = Sinusoide {
            theta_min_c: 8.0,
            theta_max_c: 14.0,
            mois_minimum: 2,
        };
        proche(s.theta_c(2), 8.0);
        proche(s.theta_c(8), 14.0);
        // Mois 5 : 2π·5/12 + 7π/6 = 2π, sinus nul.
        proche(s.theta_c(5), 11.0);
    }

    #[test]
    fn captage_sondes_et_saison() {
        let src = SourceAmont::Captage {
            sol: Sinusoide {
                theta_min_c: 8.0,
                theta_max_c: 14.0,
                mois_minimum: 2,
            },
            rb_sur_l_k_w: 0.1 / 100.0,
            pompes: Pompes {
                gestion: GestionPompesCaptage::MarchePermanente,
                p_captage_w: 200.0,
                p_inter_w: 0.0,
            },
            ecarts: Ecarts {
                ch_k: 5.0,
                fr_k: 5.0,
            },
        };
        let mut etat = EtatSourceAmont::default();
        let mut c = ConditionsEauSol {
            id_mois: 2,
            theta_ext_c: 0.0,
            w_ext_kg_kg: 0.003,
            phi_rejet_prec_w: -3000.0,
            tau_charge_prec: 0.5,
            en_saison: true,
        };
        // Premier pas de la saison : rejets nuls, θamont = θb.
        proche(src.theta_amont_eau_sol_c(&mut etat, &c).unwrap(), 8.0);
        // Ensuite : θf = 8 − 3000 × 0,001 = 5 °C ; θamont = 5 − 5/2 = 2,5 °C (éq. 1410, 1427).
        proche(src.theta_amont_eau_sol_c(&mut etat, &c).unwrap(), 2.5);
        // Refroidissement : θf = 8 + 2 = 10 °C ; θamont = 10 + 2,5.
        c.phi_rejet_prec_w = 2000.0;
        proche(src.theta_amont_eau_sol_c(&mut etat, &c).unwrap(), 12.5);
    }

    #[test]
    fn efficacites_d_echangeur() {
        // Éq. (1418) : C = 1, NUT = 1 → 0,5.
        proche(
            efficacite_echangeur(EcoulementsEchangeur::ContreCourant, 1.0, 1.0),
            0.5,
        );
        // Éq. (1420) : (1 − e^−1,5)/1,5.
        proche(
            efficacite_echangeur(EcoulementsEchangeur::CourantsParalleles, 1.0, 0.5),
            (1.0 - 0.223_130_160_148_429_8) / 1.5,
        );
        // Éq. (1419) : C = 0,5, NUT = 2 : (1 − e^−1)/(1 − 0,5 e^−1).
        let e1 = 0.367_879_441_171_442_3;
        proche(
            efficacite_echangeur(EcoulementsEchangeur::ContreCourant, 2.0, 0.5),
            (1.0 - e1) / (1.0 - 0.5 * e1),
        );
    }

    #[test]
    fn nappe_avec_echangeur_a_la_main() {
        // 3,6 m³/h d'eau des deux côtés : C = 4180 W/K ; UA = 4180 W/K : NUT = 1, C = 1,
        // ε = 0,5. Tretour = 12 + 0,5 × (10 − 12) = 11 °C ; Tdépart = 11 − 4180/4180 = 10 °C.
        let x = Echangeur {
            qv_nappe_m3_h: 3.6,
            qv_inter_m3_h: 3.6,
            rho_inter_kg_m3: 1000.0,
            cp_inter_j_kg_k: 4180.0,
            ua_w_k: 4180.0,
            ecoulements: EcoulementsEchangeur::ContreCourant,
        };
        let mut t_depart = 12.0;
        proche(
            nappe_avec_echangeur(&x, 1.0, 10.0, -4180.0, &mut t_depart),
            11.0,
        );
        proche(t_depart, 10.0);
        // Débits nuls : pas d'échange, départ conservé (éq. 1414).
        proche(
            nappe_avec_echangeur(&x, 0.0, 10.0, -4180.0, &mut t_depart),
            10.0,
        );
        proche(t_depart, 10.0);
    }

    #[test]
    fn tour_humide_et_seche() {
        // Air à 30 °C et 10 g/kg : température humide 19,506 °C (résolution indépendante sur θ
        // de l'égalité Ce·θ·(ωsat − ωext) = hsat − hext de l'éq. 1428).
        assert!((temperature_humide_c(30.0, 0.010) - 19.505_996_5).abs() < 1e-4);
        let mut t = Tour {
            nature: NatureTourRefroidissement::Humide,
            delta_tour_k: 4.0,
            consigne_c: 20.0,
            p_pompes_w: 100.0,
            p_vent_w: 50.0,
            delta_cond_fr_k: 5.0,
        };
        // max(19,506 + 4 ; 20) + 2,5.
        assert!((t.theta_amont_c(30.0, 0.010) - 26.005_996_5).abs() < 1e-4);
        // Tour sèche : max(30 + 4 ; 20) + 2,5 ; consigne à basse température.
        t.nature = NatureTourRefroidissement::Seche;
        proche(t.theta_amont_c(30.0, 0.010), 36.5);
        proche(t.theta_amont_c(10.0, 0.005), 22.5);
        let s = SourceAmontGenerateur {
            source: SourceAmont::Tour(t),
            p_vent_gaine_w: 0.0,
        };
        // Éq. (1449) à (1451) : (100 + 50) × 0,5.
        proche(s.w_aux_am_saison_wh(0.5, true), 75.0);
    }

    #[test]
    fn sol_tableau_245() {
        proche(theta_bain_essai_c(2.5), 2.75);
        proche(theta_bain_essai_c(7.5), 5.25);
        proche(theta_bain_essai_c(12.0), 6.5);
        proche(theta_bain_essai_c(-7.0), -4.0);
        let src = SourceAmont::Sol {
            sol: Sinusoide {
                theta_min_c: 5.0,
                theta_max_c: 5.0,
                mois_minimum: 1,
            },
            rb_sur_l_k_w: 0.001,
        };
        let mut etat = EtatSourceAmont {
            t_depart_c: 12.0,
            en_saison: true,
        };
        let c = ConditionsEauSol {
            id_mois: 1,
            theta_ext_c: 0.0,
            w_ext_kg_kg: 0.003,
            phi_rejet_prec_w: -2500.0,
            tau_charge_prec: 1.0,
            en_saison: true,
        };
        // θf = 5 − 2,5 = 2,5 °C → 2,75 °C (éq. 1463).
        proche(src.theta_amont_eau_sol_c(&mut etat, &c).unwrap(), 2.75);
        let s = SourceAmontGenerateur {
            source: src,
            p_vent_gaine_w: 0.0,
        };
        proche(s.w_aux_am_saison_wh(1.0, true), 0.0);
    }

    #[test]
    fn pompes_de_captage() {
        let mut p = Pompes {
            gestion: GestionPompesCaptage::DemandeVitesseVariable,
            p_captage_w: 200.0,
            p_inter_w: 100.0,
        };
        let s = |p: Pompes| SourceAmontGenerateur {
            source: SourceAmont::Boucle {
                boucle: Sinusoide {
                    theta_min_c: 15.0,
                    theta_max_c: 25.0,
                    mois_minimum: 1,
                },
                pompes: p,
                ecarts: Ecarts {
                    ch_k: 5.0,
                    fr_k: 5.0,
                },
            },
            p_vent_gaine_w: 0.0,
        };
        // Éq. (1445) et (1447) : 300 × max(0,2 ; 0,3) ; 300 × 0,8 ; rien à charge nulle.
        proche(s(p).w_aux_am_saison_wh(0.2, true), 90.0);
        proche(s(p).w_aux_am_saison_wh(0.8, true), 240.0);
        proche(s(p).w_aux_am_saison_wh(0.0, true), 0.0);
        p.gestion = GestionPompesCaptage::DemandeToutOuRien;
        proche(s(p).w_aux_am_saison_wh(0.2, true), 300.0);
        // Éq. (1444) et (1446) : marche permanente pendant la saison.
        p.gestion = GestionPompesCaptage::MarchePermanente;
        proche(s(p).w_aux_am_saison_wh(0.0, true), 300.0);
        proche(s(p).w_aux_am_saison_wh(0.0, false), 0.0);
    }
}
