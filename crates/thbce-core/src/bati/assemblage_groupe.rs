// SPDX-License-Identifier: GPL-3.0-or-later
//! Assemblage des composants d'un groupe pour le calcul du Bbio : fiche
//! `S2_BAT_assemblage_groupe` (Th-BCE 2020 §5.7, figure 23).
//!
//! À chaque pas de temps, dans l'ordre du §5.7.3 : flux des parois opaques et des ponts
//! thermiques ; baies (protections mobiles, flux solaires et lumineux, ouverture et débit de
//! surventilation) ; éclairage ; comportement thermique (matrice) ; indicateurs de confort ;
//! émission ; bilan hydrique ; saisons. Les débits d'air reçus de la zone (bilan aéraulique,
//! §5.6) sont une entrée : voir [`super::assemblage_zone`].
//!
//! Périmètre : calcul Th-B des usages d'habitation, sans espace tampon.
//!
//! Lectures propres à l'enchaînement horaire : `docs/questions.md`, Q39.

use thbce_data::protections_mobiles::parts_occupation;

use crate::chauffage_refroidissement::bilan_hydrique::{omega_fin_bbio_kg_kg, EchangesHydriques};
use crate::chauffage_refroidissement::emission::{emission_bbio, Consignes};
use crate::climat::calendrier::{hleg, jour_bascule_saisons};
use crate::confort::indicateurs_confort::{indicateurs, CumulsConfort, CAT_AMB};
use crate::conventions::ModeCalcul;
use crate::eclairage::{eclairage_habitation, einat_vn_lux};
use crate::entree::{Catalogue, Groupe, UsageZone};
use crate::scenarios::saisons::{refroidissement_possible, Journee, Saison, Saisons};
use crate::scenarios::scenarios_conventionnels::{ApportsGroupe, SortiesZone};
use crate::ventilation::bouche_conduit::{debits_specifiques_bbio_groupe, Debits};
use crate::ventilation::debits_air_cep::{rho_kg_m3, EntreeAir, GroupeAeraulique};
use crate::ventilation::ven_bbio::{debits_systeme_conventionnel, LectureDebitsBbio};

use super::baie::{BaieResolue, EnsembleBaies, FluxBaies, Protections};
use super::comportement_thermique::{
    AirEntrant, AirEquivalent, GroupeThermique, Sollicitations, Temperatures, F_SA,
    LECTURE_FLUX_RADIATIFS,
};
use super::paroi_opaque::{EnsembleParois, ParoiResolue};
use super::pont_thermique::{EnsemblePonts, PontResolu};
use super::protection_mobile::{
    a_ouv_max_avec_pm_m2, ConditionsGpm, GestionResolue, SaisonGpm, TemperaturesMaximales,
};
use super::surventilation::{
    a_ouv_baie_m2, qm_aj_kg_s, qv_surv_m3_h, r_ouv_manuelle, r_ouv_theta_op_man, s_ors_m2,
    traversant_conventionnel, ConditionsOuverture, OuvertureBaie, H_TTF_CAS_GENERAL_M,
};
use super::{paroi_opaque, pont_thermique, ConditionsExterieures};

/// Température initiale du groupe, en °C : masse, air et température opérative au pas de temps
/// qui précède le premier.
///
/// Le texte ne donne pas l'état initial du modèle à trois nœuds. Il initialise à 18 °C la
/// moyenne sur 24 h de la température de masse quotidienne (§5.21.3.10, note de l'éq. 353) et
/// les températures de masse séquentielle et annuelle (notes des éq. 354 et 358) : la même
/// valeur est retenue pour `θmq`, `θi,fin` et `θop,fin` (décision Q39 point 4).
pub const THETA_INITIALE_C: f64 = 18.0;

/// « Δθconf_adapt(h) » en calcul Th-B, en °C.
///
/// Th-BCE 2020 §13.5.3, éq. (2546) : l'écart entre la température d'inconfort chaud et la
/// consigne de refroidissement n'est défini que « si Mode = Th-D », et il est « nul hors période
/// de confort adaptatif ». En Th-B, les seuils d'ouverture des baies (§5.13.3.1.1, éq. 242 et
/// 243) ne se déplacent donc pas (décision Q39 point 5).
pub const DELTA_THETA_CONF_ADAPT_TH_B_C: f64 = 0.0;

/// Composants de l'enveloppe d'un groupe, prêts pour le calcul (§5.8, §5.15 et §5.18).
#[derive(Debug, Clone, PartialEq)]
pub struct EnveloppeResolue {
    /// Parois opaques.
    pub parois: Vec<ParoiResolue>,
    /// Ponts thermiques (quatre tronçons par pont en description simplifiée).
    pub ponts: Vec<PontResolu>,
    /// Baies.
    pub baies: Vec<BaieResolue>,
    /// Gestion de la première protection mobile de chaque baie ; `None` sans protection.
    pub gestions: Vec<Option<GestionResolue>>,
}

impl EnveloppeResolue {
    /// Résout les composants d'un groupe. Les références au catalogue et les cas non pris en
    /// charge ont été écartés par la validation ; un composant qui ne peut pas être résolu est
    /// ignoré.
    pub fn de(groupe: &Groupe, catalogue: &Catalogue, usage: UsageZone) -> Self {
        let parois = groupe
            .parois_opaques
            .iter()
            .filter_map(|p| {
                let type_paroi = catalogue.parois.iter().find(|t| t.id == p.type_paroi)?;
                Some(ParoiResolue::nouvelle(p, type_paroi, catalogue))
            })
            .collect();
        let mut ponts = Vec::new();
        for pont in &groupe.ponts_thermiques {
            PontResolu::resoudre(pont, catalogue, &mut ponts);
        }
        let mut baies = Vec::new();
        let mut gestions = Vec::new();
        for b in &groupe.baies {
            let Some(type_baie) = catalogue.baies.iter().find(|t| t.id == b.type_baie) else {
                continue;
            };
            let Some(resolue) = BaieResolue::nouvelle(b, type_baie, catalogue) else {
                continue;
            };
            baies.push(resolue);
            gestions.push(
                type_baie
                    .protection_mobile
                    .as_ref()
                    .and_then(|pm| GestionResolue::nouvelle(pm, usage.numero())),
            );
        }
        Self {
            parois,
            ponts,
            baies,
            gestions,
        }
    }

    /// « HTH,k+pt » : coefficient de transmission des parois opaques et des ponts thermiques, en
    /// W/K (§5.21.3.6, éq. 340).
    pub fn h_th_k_pt_w_k(&self) -> f64 {
        EnsembleParois::de(&self.parois).h_th_w_k + EnsemblePonts::de(&self.ponts).h_th_w_k
    }
}

/// « Nbh_occ_ref » : nombre d'heures d'occupation d'une semaine type de l'usage.
///
/// Th-BCE 2020 §4.6.3, éq. (66) : somme des valeurs du tableau des indicateurs d'occupation de
/// la zone par jour (1 à 7) et par heure (1 à 24).
pub fn nbh_occ_ref(p_occ: &[[f64; 24]; 7]) -> f64 {
    p_occ.iter().flatten().sum()
}

/// Bilan d'énergie d'un groupe, en Wh : cumul des flux de chaleur du modèle thermique, évalués
/// aux températures moyennes du pas de temps (émission comprise).
///
/// Le schéma de Crank-Nicolson de l'éq. (365) conserve l'énergie quand les flux sont évalués à
/// la température de masse moyenne du pas de temps : la somme des apports et de l'émission,
/// diminuée des pertes, est égale à la variation de l'énergie stockée dans les parois lourdes,
/// aux erreurs d'arrondi près. Ce bilan n'est pas une sortie de la méthode ; il sert à vérifier
/// l'enchaînement horaire.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BilanEnergie {
    /// Flux solaires transmis par les baies sous forme de courte longueur d'onde et par la lame
    /// d'air ventilée (`Fs1` et `Fs3`).
    pub apports_solaires_baies_wh: f64,
    /// Flux solaires et de voûte céleste des parois opaques et des ponts thermiques, ramenés à
    /// la surface intérieure (`φsh × Hgemq/HTH,k+pt`).
    pub apports_opaques_wh: f64,
    /// Apports internes : occupants, équipements et éclairage, convectifs et radiatifs.
    pub apports_internes_wh: f64,
    /// Chaleur fournie par l'émission de chauffage (`Qsys_ch`).
    pub chauffage_wh: f64,
    /// Chaleur retirée par l'émission de refroidissement (`−Qsys_fr`), positive.
    pub refroidissement_wh: f64,
    /// Pertes nettes par les baies : transmission, diminuée du flux solaire de grande longueur
    /// d'onde et augmentée du rayonnement vers la voûte céleste (`Hges·(θs − θe) − Fs2 − Ftvc`).
    pub pertes_baies_wh: f64,
    /// Pertes par les parois opaques et les ponts thermiques (`Hgemq·(θmq − θe)`).
    pub pertes_opaques_wh: f64,
    /// Pertes par renouvellement d'air (`Hgei·(θi − θei,eq)`).
    pub pertes_air_wh: f64,
    /// Variation de l'énergie stockée dans les parois lourdes (`Cmq·Δθmq`).
    pub stockage_wh: f64,
    /// Part des apports radiatifs et solaires qui ressort par les baies sans entrer dans le
    /// bilan (éq. 361), ou qui n'est injectée nulle part avec la lecture littérale de Q34.
    pub apports_non_recuperes_wh: f64,
}

impl BilanEnergie {
    /// Somme des termes entrants, en Wh.
    pub fn entrees_wh(&self) -> f64 {
        self.apports_solaires_baies_wh
            + self.apports_opaques_wh
            + self.apports_internes_wh
            + self.chauffage_wh
    }

    /// Somme des termes sortants et du stockage, en Wh.
    pub fn sorties_wh(&self) -> f64 {
        self.refroidissement_wh
            + self.pertes_baies_wh
            + self.pertes_opaques_wh
            + self.pertes_air_wh
            + self.stockage_wh
            + self.apports_non_recuperes_wh
    }

    /// Défaut de fermeture du bilan, en Wh.
    pub fn residu_wh(&self) -> f64 {
        self.entrees_wh() - self.sorties_wh()
    }
}

/// Cumuls d'un groupe sur l'année de calcul.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CumulsGroupe {
    /// Somme mensuelle des `Qsys_ch(h)`, en Wh (§13.1.3, numérateur de l'éq. 2379).
    pub q_sys_ch_wh: [f64; 12],
    /// Somme mensuelle des `|Qsys_fr(h)|`, en Wh (numérateur de l'éq. 2380, en valeur absolue).
    pub q_sys_fr_wh: [f64; 12],
    /// Somme mensuelle des `CECL(h)`, en kWh (numérateur de l'éq. 2381).
    pub c_ecl_kwh: [f64; 12],
    /// Indicateurs de confort (§4.5).
    pub confort: CumulsConfort,
    /// Bilan d'énergie du modèle thermique.
    pub bilan: BilanEnergie,
    /// Nombre de pas de temps où les baies sont ouvertes pour la surventilation.
    pub nbh_surventilation: u32,
}

/// Conditions d'un pas de temps communes aux groupes d'une zone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionsPas {
    /// Conditions extérieures (rayonnement, températures, position du soleil).
    pub ext: ConditionsExterieures,
    /// « we » : humidité spécifique de l'air extérieur, en kg/kg.
    pub omega_e_kg_kg: f64,
    /// « Vvent_c » : vitesse du vent corrigée, en m/s.
    pub v_vent_c_m_s: f64,
    /// « θrm(j) » : moyenne glissante de la température extérieure, en °C.
    pub theta_rm_c: f64,
    /// Vrai si le pas de temps ouvre un jour légal (éq. 175 : « incrémentation de
    /// Jour_année »).
    pub nouveau_jour: bool,
    /// Sorties des scénarios de la zone.
    pub scenarios: SortiesZone,
    /// « θiifr_+ » : consigne de refroidissement en occupation normale, en °C.
    pub theta_ii_fr_plus_c: f64,
}

/// État d'un groupe d'un pas de temps au suivant.
#[derive(Debug, Clone, PartialEq)]
pub struct EtatGroupeBbio {
    /// Températures à la fin du dernier pas de temps calculé (éq. 825).
    pub fin: Temperatures,
    /// « θop,fin(h−2) », en °C.
    pub theta_op_fin_h2_c: f64,
    /// « ωi,g,fin » : humidité spécifique à la fin du dernier pas de temps, en kg/kg.
    pub omega_i_fin_kg_kg: f64,
    /// Températures opératives maximales du jour et de la veille (éq. 175).
    pub t_max: TemperaturesMaximales,
    /// « Rouv_θop_man » du dernier pas de temps (§5.13.3.1.1).
    pub r_ouv_theta_op: f64,
    /// Saisons propres du groupe (§4.6).
    pub saisons: Saisons,
}

/// Sorties d'un groupe à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SortiesPasGroupe {
    /// « Qsys_ch(h) », en Wh, positif ou nul.
    pub q_sys_ch_wh: f64,
    /// « Qsys_fr(h) », en Wh, négatif ou nul.
    pub q_sys_fr_wh: f64,
    /// « CECL_GR(h) », en kWh.
    pub c_ecl_kwh: f64,
    /// « θop,moy(0;0)(h) » : température opérative moyenne en évolution libre, en °C.
    pub theta_op_moy_00_c: f64,
    /// Températures moyennes du pas de temps, émission comprise.
    pub moy: Temperatures,
    /// Températures finales du pas de temps, émission comprise.
    pub fin: Temperatures,
    /// « Qv,surv,ouv(h) » : débit de surventilation par ouverture des baies, en m³/h.
    pub qv_surv_m3_h: f64,
    /// Saison propre du groupe pendant le pas de temps.
    pub saison: Saison,
}

/// Groupe d'habitation prêt pour le calcul du Bbio.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupeBbio {
    /// Identifiant du groupe.
    pub id: String,
    /// « Agr » : surface utile, en m².
    pub a_gr_m2: f64,
    /// « V » : volume, en m³.
    pub v_m3: f64,
    /// Composants de l'enveloppe.
    pub enveloppe: EnveloppeResolue,
    /// Modèle thermique.
    pub thermique: GroupeThermique,
    /// « HTH,k+pt », en W/K.
    pub h_th_k_pt_w_k: f64,
    /// Débits du système de ventilation conventionnel du Bbio, en m³/h (§6.1 et §6.2).
    pub debits: Debits,
    /// Données aérauliques du groupe (§5.6).
    pub aeraulique: GroupeAeraulique,
    /// « δtrav_surv » : groupe traversant au sens de la surventilation (tableau 43).
    pub traversant: bool,
    /// « Httf » : hauteur pour le tirage thermique, en m.
    pub h_ttf_m: f64,
    /// « Pocc » : part des baies sur lesquelles un occupant peut agir (figure 34).
    pub p_occ: f64,
    /// État du groupe.
    pub etat: EtatGroupeBbio,
    /// Cumuls annuels.
    pub cumuls: CumulsGroupe,
    ouvertures: Vec<OuvertureBaie>,
}

impl GroupeBbio {
    /// Prépare un groupe d'habitation.
    ///
    /// `q4pa_surf_m3_h_m2` est la perméabilité de la zone ; `nbh_occ_ref`, le nombre d'heures
    /// d'occupation d'une semaine type de l'usage (éq. 66) ; `omega_initiale_kg_kg`, l'humidité
    /// de l'air du groupe avant le premier pas de temps.
    pub fn nouveau(
        groupe: &Groupe,
        catalogue: &Catalogue,
        usage: UsageZone,
        q4pa_surf_m3_h_m2: f64,
        nbh_occ_ref: f64,
        omega_initiale_kg_kg: f64,
    ) -> Self {
        let enveloppe = EnveloppeResolue::de(groupe, catalogue, usage);
        let a_gr_m2 = groupe.su_m2.unwrap_or(0.0);
        let op = EnsembleParois::de(&enveloppe.parois);
        let ba = EnsembleBaies::de(&enveloppe.baies);
        let thermique =
            GroupeThermique::nouveau(a_gr_m2, ba.a_baies_m2, &groupe.inertie.quotidienne);
        let debits = debits_specifiques_bbio_groupe(&groupe.bouches_conduits)
            .map(|d| debits_systeme_conventionnel(d, LectureDebitsBbio::default()))
            .unwrap_or_default();
        let initiale = Temperatures {
            theta_i_c: THETA_INITIALE_C,
            theta_s_c: THETA_INITIALE_C,
            theta_mq_c: THETA_INITIALE_C,
            theta_rm_c: THETA_INITIALE_C,
            theta_op_c: THETA_INITIALE_C,
        };
        let nb_baies = enveloppe.baies.len();
        Self {
            id: groupe.id.clone(),
            a_gr_m2,
            v_m3: groupe.v_m3.unwrap_or(0.0),
            h_th_k_pt_w_k: enveloppe.h_th_k_pt_w_k(),
            thermique,
            debits,
            // Th-BCE 2020 §5.6.3.1.5 et §5.7.2 : façades et toits au sens de la perméabilité.
            aeraulique: GroupeAeraulique {
                a_f_m2: op.a_opv_m2 + ba.a_baies_v_m2,
                a_t_m2: op.a_ophh_m2 + ba.a_baies_h_m2,
                q4pa_surf_m3_h_m2,
                entree_air: groupe.entrees_air.as_ref().map(EntreeAir::de_saisie),
            },
            // Th-BCE 2020 §5.13.3.2.1, tableau 43 : conventionnel en maison, saisi en collectif.
            traversant: traversant_conventionnel(usage.numero())
                .or(groupe.traversant)
                .unwrap_or(false),
            h_ttf_m: groupe.h_ttf_m.unwrap_or(H_TTF_CAS_GENERAL_M),
            p_occ: parts_occupation(usage.numero()).map_or(0.0, |p| p.p_occ),
            etat: EtatGroupeBbio {
                fin: initiale,
                theta_op_fin_h2_c: THETA_INITIALE_C,
                omega_i_fin_kg_kg: omega_initiale_kg_kg,
                t_max: TemperaturesMaximales::default(),
                r_ouv_theta_op: 0.0,
                // En Bbio, le refroidissement est possible sans système (décision Q35 point 7).
                saisons: Saisons::nouveau(a_gr_m2, nbh_occ_ref, ModeCalcul::ThB, false),
            },
            cumuls: CumulsGroupe::default(),
            ouvertures: Vec::with_capacity(nb_baies),
            enveloppe,
        }
    }

    /// Calcule un pas de temps du groupe.
    ///
    /// `air_zone` : flux d'air que le bilan aéraulique de la zone fait entrer dans le groupe
    /// (air neuf par l'enveloppe, air soufflé par la ventilation, air venu du hall ; §5.6.3.8).
    /// `apports` : apports internes du groupe (§4.1.3).
    ///
    /// Renvoie `None` si le modèle thermique n'a pas de solution (`HTH,k+pt ≥ Hgmqs`), cas
    /// écarté par la validation.
    pub fn pas(
        &mut self,
        c: &ConditionsPas,
        apports: &ApportsGroupe,
        air_zone: &[AirEntrant],
    ) -> Option<SortiesPasGroupe> {
        let h = c.ext.h;
        let sc = &c.scenarios;
        let i_occ_zone = sc.i_occ_zone > 0.0;
        let theta_e = c.ext.te_c;
        let theta_i_h1 = self.etat.fin.theta_i_c;
        let theta_op_h1 = self.etat.fin.theta_op_c;
        // Th-BCE 2020 §4.6.3.3, éq. (91) : en Bbio, autorisations effectives égales aux
        // autorisations propres ; elles valent de 9 h légales à 9 h légales le lendemain.
        let saison = self.etat.saisons.saison();
        let (aut_ch, aut_fr) = (self.etat.saisons.aut_ch, self.etat.saisons.aut_fr);

        // §5.9.3.2, éq. (175) : température opérative maximale de la veille.
        self.etat.t_max.avancer(theta_op_h1, c.nouveau_jour);
        let saison_gpm =
            SaisonGpm::de_saison_effective(saison as u8).unwrap_or(SaisonGpm::MiSaison);

        // §5.15 à 5.20 : parois opaques et ponts thermiques.
        let op = paroi_opaque::flux_ensemble(&self.enveloppe.parois, &c.ext);
        let pt = pont_thermique::flux_ensemble(&self.enveloppe.ponts, &c.ext);
        let phi_sh_w = op.phi_sh_s_w + op.phi_sh_vc_w + pt.phi_sh_s_w + pt.phi_sh_vc_w;

        // §5.13.3.1 : gestion manuelle de l'ouverture des baies, règles du calcul Th-B.
        let ouverture = ConditionsOuverture {
            mode: ModeCalcul::ThB,
            i_occ_zone,
            hleg: hleg(h),
            saison_de_chauffage: saison == Saison::Chauffage,
            refroidissement_autorise: aut_fr,
            theta_ext_c: theta_e,
            theta_op_h1_c: theta_op_h1,
            theta_op_h2_c: self.etat.theta_op_fin_h2_c,
            theta_iifr_c: c.theta_ii_fr_plus_c,
            delta_theta_conf_adapt_c: DELTA_THETA_CONF_ADAPT_TH_B_C,
        };
        let r_ouv_theta_op = r_ouv_theta_op_man(self.etat.r_ouv_theta_op, &ouverture);

        // §5.8 : baies. Protections mobiles (§5.9), flux (§5.10), ouverture (§5.13.3.2.3).
        let mut baies = FluxBaies::default();
        self.ouvertures.clear();
        for (baie, gestion) in self.enveloppe.baies.iter().zip(&self.enveloppe.gestions) {
            let incident = baie.incident(&c.ext);
            let protections = gestion.as_ref().map_or(Protections::OUVERTES, |g| {
                g.protections(&ConditionsGpm {
                    i_occ_zone,
                    // Indicateur à valeurs −1, 0 et 1 (éq. 53).
                    #[allow(clippy::cast_possible_truncation)]
                    i_occ_gpm: sc.i_occ_gpm as i8,
                    etp_lux: incident.etp_lux(),
                    v_vent_c_m_s: c.v_vent_c_m_s,
                    saison: saison_gpm,
                    top_j1_max_c: self.etat.t_max.top_j1_max_c,
                    confort_adaptatif_th_d: false,
                })
            });
            let f = baie.flux_depuis_incident(&incident, &c.ext, &protections);
            baies.h_ges_w_k += f.h_ges_w_k;
            baies.fs1_w += f.fs1_w;
            baies.fs2_w += f.fs2_w;
            baies.fs3_w += f.fs3_w;
            baies.ftvc_w += f.ftvc_w;
            for n in 0..3 {
                baies.flt1_lm[n] += f.flt1_lm[n];
                baies.flt2_lm[n] += f.flt2_lm[n];
                baies.flt3_lm[n] += f.flt3_lm[n];
            }
            if let (true, Some(expo)) = (baie.a_ouv_max_m2 > 0.0, baie.expo_br) {
                let r_ouv = r_ouv_manuelle(&ouverture, self.p_occ, expo, r_ouv_theta_op);
                let a_ouv_max_pm = a_ouv_max_avec_pm_m2(
                    baie.a_ouv_max_m2,
                    protections.r_prot(),
                    baie.taux_passage_air,
                );
                self.ouvertures.push(OuvertureBaie {
                    a_ouv_m2: a_ouv_baie_m2(r_ouv, baie.a_ouv_max_m2, a_ouv_max_pm),
                    alpha_deg: baie.alpha_deg,
                    beta_deg: baie.beta_deg,
                });
            }
        }

        // §5.13.3.2.4 et §5.13.3.3 : débit de surventilation, entrant à la température
        // extérieure.
        let a_ouv: f64 = self.ouvertures.iter().map(|o| o.a_ouv_m2).sum();
        let qv_surv = if a_ouv > 0.0 {
            qv_surv_m3_h(
                a_ouv,
                self.traversant.then(|| s_ors_m2(&self.ouvertures)),
                c.v_vent_c_m_s,
                self.h_ttf_m,
                theta_e,
                theta_i_h1,
            )
        } else {
            0.0
        };
        let qm_surv = qm_aj_kg_s(qv_surv, theta_e, 0.0);

        // §7.1 : éclairage. Volumes normaux (baies de type 2) ; Aeclnat2 = Agr en habitation.
        let einat = einat_vn_lux(
            baies.flt1_lm[2],
            baies.flt2_lm[2],
            baies.flt3_lm[2],
            self.a_gr_m2,
        );
        let ecl = eclairage_habitation(sc.i_ecl > 0.0, einat, self.a_gr_m2);

        // §5.21.3.5 : air entrant équivalent (bilan aéraulique de la zone et surventilation).
        let mut flux_air = [AirEntrant {
            qm_kg_s: 0.0,
            theta_c: theta_e,
            omega_kg_kg: c.omega_e_kg_kg,
        }; 4];
        for (cible, source) in flux_air.iter_mut().take(3).zip(air_zone) {
            *cible = *source;
        }
        flux_air[3].qm_kg_s = qm_surv;
        let air = AirEquivalent::melange(&flux_air);

        // §5.21 : sollicitations et matrice de comportement.
        let phi_conv_w = apports.phi_int_conv_w + apports.phi_int_occ_conv_w + ecl.f_ecl_c_w;
        let phi_rad_w = apports.phi_int_rad_w + apports.phi_int_occ_rad_w + ecl.f_ecl_r_w;
        let s = Sollicitations {
            air,
            theta_e_c: theta_e,
            h_th_k_pt_w_k: self.h_th_k_pt_w_k,
            hg_es_w_k: baies.h_ges_w_k,
            phi_sl_w: baies.fs2_w + baies.ftvc_w,
            phi_sh_w,
            fs1_w: baies.fs1_w,
            fs3_w: baies.fs3_w,
            phi_conv_w,
            phi_rad_w,
        };
        let theta_mq_precedent = self.etat.fin.theta_mq_c;
        let matrice = self.thermique.matrice(&s, theta_mq_precedent, 0.0)?;
        let theta_op_moy_00 = matrice[0].moy.theta_op_c;

        // §4.5 : indicateurs de confort, en évolution libre.
        let confort = indicateurs(
            i_occ_zone,
            sc.theta_ii_ch_c,
            sc.theta_ii_fr_c,
            c.theta_rm_c,
            theta_op_moy_00,
        );
        self.cumuls.confort.ajouter(&confort);

        // §8.1 : émission conventionnelle du Bbio.
        let e = emission_bbio(
            &matrice,
            self.a_gr_m2,
            &Consignes {
                theta_ii_ch_c: sc.theta_ii_ch_c,
                theta_ii_fr_c: sc.theta_ii_fr_c,
                aut_ch,
                aut_fr,
                i_clim: refroidissement_possible(ModeCalcul::ThB, false),
            },
        );

        // §8.3 : bilan hydrique, en aval de l'émission. Apports des scénarios en kg/h.
        let omega_fin = omega_fin_bbio_kg_kg(
            self.etat.omega_i_fin_kg_kg,
            &EchangesHydriques {
                somme_q_maj_kg_s: air.qm_eq_kg_s,
                somme_q_omega_maj_kg_s: air.qm_eq_kg_s * air.omega_eq_kg_kg,
                a_omega_int_kg_s: (apports.omega_int_kg_h + apports.omega_int_occ_kg_h) / 3600.0,
                v_m3: self.v_m3,
                rho_i_kg_m3: rho_kg_m3(theta_i_h1, self.etat.omega_i_fin_kg_kg),
            },
        );

        self.cumuler_bilan(
            &s,
            &e.moy,
            e.phi_util_w,
            theta_mq_precedent,
            e.fin.theta_mq_c,
        );

        // §13.1.3, éq. (2379) à (2381) : cumuls mensuels, mois au sens des scénarios.
        let mois = usize::from(sc.date.mois) - 1;
        self.cumuls.q_sys_ch_wh[mois] += e.q_sys_ch_wh;
        self.cumuls.q_sys_fr_wh[mois] -= e.q_sys_fr_wh;
        self.cumuls.c_ecl_kwh[mois] += ecl.c_ecl_kwh;
        self.cumuls.nbh_surventilation += u32::from(qv_surv > 0.0);

        // §4.6 : cumuls de la journée, puis bascule à 9 h légales.
        self.etat.saisons.accumuler(&Journee {
            nbh_occ: sc.i_occ_zone,
            dh_ch_c_h: confort.delta_theta_op_inc_cons_ch_c,
            dh_fr_c_h: confort.delta_theta_op_inc_fr_c(CAT_AMB),
            q_ch_wh: e.q_sys_ch_wh,
            q_fr_wh: -e.q_sys_fr_wh,
        });
        if let Some(j) = jour_bascule_saisons(h) {
            self.etat.saisons.nouveau_jour(j, false);
        }

        // État pour le pas de temps suivant.
        self.etat.theta_op_fin_h2_c = theta_op_h1;
        self.etat.fin = e.fin;
        self.etat.omega_i_fin_kg_kg = omega_fin;
        self.etat.r_ouv_theta_op = r_ouv_theta_op;

        Some(SortiesPasGroupe {
            q_sys_ch_wh: e.q_sys_ch_wh,
            q_sys_fr_wh: e.q_sys_fr_wh,
            c_ecl_kwh: ecl.c_ecl_kwh,
            theta_op_moy_00_c: theta_op_moy_00,
            moy: e.moy,
            fin: e.fin,
            qv_surv_m3_h: qv_surv,
            saison,
        })
    }

    /// Ajoute un pas de temps au bilan d'énergie ([`BilanEnergie`]).
    ///
    /// Bilan des trois nœuds du modèle (§5.21.3.11 à 5.21.3.13, éq. 365 à 368), sommé :
    /// `Cmq·Δθmq = φi + φs + φmq + φsys + φsl + φsh·Hgemq/HTH,k+pt − Hgei·(θi − θei,eq)
    /// − Hges·(θs − θe) − Hgemq·(θmq − θe)`, les températures étant les moyennes du pas de
    /// temps. Les parts `frs`, `frsd`, `frm` et `frmd` sont celles des éq. (347) à (352) et de
    /// la décision Q34 ; ce qui n'est injecté à aucun nœud est compté à part.
    fn cumuler_bilan(
        &mut self,
        s: &Sollicitations,
        moy: &Temperatures,
        phi_util_w: f64,
        theta_mq_precedent_c: f64,
        theta_mq_fin_c: f64,
    ) {
        let g = &self.thermique;
        let hg_emq = g.hg_emq_w_k(s.h_th_k_pt_w_k).unwrap_or(0.0);
        let (fr_s, fr_sd) = g.parts_noeud_s(s.hg_es_w_k, LECTURE_FLUX_RADIATIFS);
        let fr_m = g.a_mq_m2 / g.a_tparois_m2;
        let fr_md = g.a_mq_m2 / (g.a_tparois_m2 - g.a_baies_m2);
        let b = &mut self.cumuls.bilan;
        let solaire_recu = s.fs1_w * (F_SA + (1.0 - F_SA) * (fr_sd + fr_md)) + s.fs3_w;
        let internes_recus = s.phi_conv_w + s.phi_rad_w * (fr_s + fr_m);
        b.apports_solaires_baies_wh += s.fs1_w + s.fs3_w;
        b.apports_internes_wh += s.phi_conv_w + s.phi_rad_w;
        b.apports_non_recuperes_wh +=
            (s.fs1_w + s.fs3_w - solaire_recu) + (s.phi_conv_w + s.phi_rad_w - internes_recus);
        if s.h_th_k_pt_w_k > 0.0 {
            b.apports_opaques_wh += s.phi_sh_w * hg_emq / s.h_th_k_pt_w_k;
        }
        b.chauffage_wh += phi_util_w.max(0.0);
        b.refroidissement_wh += (-phi_util_w).max(0.0);
        // φsl = Fs2 + Ftvc entre au nœud θs avec la transmission des baies (éq. 342 et 343).
        b.pertes_baies_wh += s.hg_es_w_k * (moy.theta_s_c - s.theta_e_c) - s.phi_sl_w;
        b.pertes_opaques_wh += hg_emq * (moy.theta_mq_c - s.theta_e_c);
        b.pertes_air_wh += s.air.hg_ei_w_k * (moy.theta_i_c - s.air.theta_ei_eq_c);
        b.stockage_wh += g.c_mq_kj_k / 3.6 * (theta_mq_fin_c - theta_mq_precedent_c);
    }
}
