// SPDX-License-Identifier: GPL-3.0-or-later
//! Assemblage des composants d'un groupe : fiche `S2_BAT_assemblage_groupe` (Th-BCE 2020 §5.7,
//! figure 23), pour le calcul du Bbio (mode Th-B) et pour le calcul du confort d'été (mode
//! Th-D).
//!
//! À chaque pas de temps, dans l'ordre du §5.7.3 : flux des parois opaques et des ponts
//! thermiques ; baies (protections mobiles, flux solaires et lumineux, ouverture et débit de
//! surventilation) ; éclairage ; comportement thermique (matrice) ; indicateurs de confort ;
//! émission ; bilan hydrique ; saisons. Les débits d'air reçus de la zone (bilan aéraulique,
//! §5.6) sont une entrée : voir [`super::assemblage_zone`].
//!
//! Le même enchaînement sert aux deux modes ; [`ModeCalcul`] sélectionne les règles propres à
//! chacun. En Th-D : facteurs solaires « e » (§5.10.3.7, §5.17.3 et §5.19.3), tableau 34 des
//! protections mobiles (§5.9.3.6), règles générales de l'ouverture des baies avec
//! `Δθconf_adapt` (§5.13.3.1.1 et §13.5.3.4), inerties séquentielle et annuelle (§5.21.3.10),
//! saisons du Th-D (§4.6.3.2), aucun refroidissement (§13.5.3.1), indicateur DH (§13.5.3.8).
//!
//! Périmètre : calculs Th-B et Th-D des usages d'habitation. Le calcul Th-D emploie
//! l'assemblage du calcul du Bbio (figure 23), avec les débits et l'air soufflé du système de
//! ventilation du projet (lot 6a, `docs/questions/Q042.md`) et l'émission conventionnelle
//! jusqu'au lot 6b (Q40).
//!
//! Lectures propres à l'enchaînement horaire : `docs/questions/Q039.md` et `Q040.md`.

use thbce_data::protections_mobiles::parts_occupation;

use crate::chauffage_refroidissement::bilan_hydrique::{
    bilan_th_c, omega_fin_bbio_kg_kg, BatterieFroide, EchangesHydriques,
};
use crate::chauffage_refroidissement::emetteurs::ConditionsVentilateurs;
use crate::chauffage_refroidissement::emission::{
    emission_avec_relance, Consignes, ConsignesRelance, EmetteurEquivalent,
};
use crate::chauffage_refroidissement::relances::{en_relance, ConditionsRelance, HORIZON_RELANCE};
use crate::chauffage_refroidissement::systemes_groupe::{Liens, SystemesGroupe};
use crate::climat::calendrier::{hleg, jour_bascule_saisons};
use crate::confort::confort_ete::{
    delta_theta_conf_adapt_c, theta_op_conf_ch_c, theta_op_conf_ch_corr_c, CumulsConfortEte,
    PasConfortEte,
};
use crate::confort::indicateurs_confort::{indicateurs, CumulsConfort, CAT_AMB};
use crate::conventions::ModeCalcul;
use crate::eclairage::{
    a_eclnat2_m2, a_eclnat_gv_m2, eclairage_habitation, eclairage_locaux_gv, einat_gv_lux,
    einat_vn_lux, LocalResolu,
};
use crate::entree::{Catalogue, Groupe, RegulationBatterie, UsageZone};
use crate::scenarios::saisons::{
    refroidissement_possible, Journee, LectureSaisonFroidThD, Saison, Saisons,
};
use crate::scenarios::scenarios_conventionnels::{ApportsGroupe, SortiesZone};
use crate::ventilation::bouche_conduit::{
    debits_bbio_non_residentiel, debits_specifiques_bbio_groupe, Debits,
};
use crate::ventilation::debits_air_cep::{rho_kg_m3, EntreeAir, GroupeAeraulique};
use crate::ventilation::ven_bbio::{debits_systeme_conventionnel, LectureDebitsBbio};

use super::baie::{BaieResolue, EnsembleBaies, FluxBaies, Protections};
use super::comportement_thermique::{
    AirEntrant, AirEquivalent, GroupeThermique, Sollicitations, Temperatures, F_SA,
    LECTURE_FLUX_RADIATIFS,
};
use super::espace_tampon::espace_tampon::{h_et_th_int_gr_w_k, CoefficientsEt};
use super::espace_tampon::gestion_solarise::{LectureFermetureEt, LECTURE_FERMETURE_ET};
use super::inerties_longues::{EtatInerties, FluxInerties, InertiesLongues};
use super::paroi_opaque::{EnsembleParois, ParoiResolue};
use super::pont_thermique::{EnsemblePonts, PontResolu};
use super::protection_mobile::{
    a_ouv_max_avec_pm_m2, ConditionsGpm, GestionResolue, SaisonGpm, TemperaturesMaximales,
};
use super::surventilation::{
    a_ouv_baie_m2, qm_aj_kg_s, qv_surv_m3_h, r_ouv_manuelle, r_ouv_theta_op_man_selon, s_ors_m2,
    traversant_conventionnel, ConditionsOuverture, LectureHysteresis, OuvertureBaie,
    H_TTF_CAS_GENERAL_M, LECTURE_HYSTERESIS,
};
use super::ConditionsExterieures;

/// Température initiale du groupe, en °C : masse, air et température opérative au pas de temps
/// qui précède le premier.
///
/// Le texte ne donne pas l'état initial du modèle à trois nœuds. Il initialise à 18 °C la
/// moyenne sur 24 h de la température de masse quotidienne (§5.21.3.10, note de l'éq. 353) et
/// les températures de masse séquentielle et annuelle (notes des éq. 354 et 358) : la même
/// valeur est retenue pour `θmq`, `θi,fin` et `θop,fin` (décision Q39 point 4).
pub const THETA_INITIALE_C: f64 = 18.0;

/// « Δθcorr_syst(h) » : effet sur la température d'inconfort chaud des systèmes qui modifient
/// la sensation thermique (brasseurs d'air, humidification), en °C.
///
/// Th-BCE 2020 §13.5.3.5, éq. (2547). Ces systèmes sont hors de la première version : nul.
pub const DELTA_THETA_CORR_SYST_C: f64 = 0.0;

/// Lectures du texte employées par l'enchaînement horaire d'un groupe, quand elles peuvent
/// être basculées pour mesurer leur effet. Ce ne sont pas des entrées du projet : la valeur par
/// défaut est celle des lectures retenues (`docs/questions/README.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lectures {
    /// Hystérésis de l'ouverture des baies (Q32 point 1).
    pub hysteresis: LectureHysteresis,
    /// Autorisation propre de refroidir en Th-D (Q40).
    pub saison_froid_th_d: LectureSaisonFroidThD,
    /// Débits du système de ventilation conventionnel (Q36 point 1 ; Q40 pour le Th-D).
    pub debits: LectureDebitsBbio,
    /// Prise en compte des inerties séquentielle et annuelle en Th-D (§5.21.3.10). Vrai : c'est
    /// le texte ; faux pour mesurer leur effet.
    pub inerties_longues: bool,
    /// Branche de fermeture de l'ouverture des espaces tampons solarisés (Q41 point 4).
    pub fermeture_et: LectureFermetureEt,
    /// Ventilation du calcul Th-D (Q42 ; Q40 point 1 pour l'assemblage provisoire).
    pub ventilation_th_d: VentilationThD,
    /// Émission du calcul Th-D (Q43 ; Q40 point 1 pour l'assemblage provisoire).
    pub emission_th_d: EmissionThD,
    /// Récupération des pertes et consommations des systèmes (fiche 11.1) en Th-D (Q45). Vrai :
    /// lecture retenue ; faux pour mesurer son effet.
    pub recuperation_th_d: bool,
    /// Préchauffage de l'eau froide du ballon d'ECS par un récupérateur sur les eaux grises
    /// (Q108 point 6) : `Texte` modifie seulement `θentrant` (lecture retenue) ; `Deduit` déduit
    /// la chaleur récupérée de la demande de la génération.
    pub preechauffage_ecs: LecturePreechauffageEcs,
}

/// Prise en compte du préchauffage de l'eau froide du ballon (`docs/questions/Q108.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LecturePreechauffageEcs {
    /// §9.21.3 à la lettre : seule `θentrant` change ; la demande à la génération est celle de
    /// l'émetteur (lecture retenue).
    #[default]
    Texte,
    /// La chaleur `ρw·cw·Vec·(θentrant − θcw)` récupérée sur l'eau froide est déduite de la
    /// demande de la génération.
    Deduit,
}

/// Émission employée par le calcul Th-D (`docs/questions/Q043.md` ; Q40 point 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmissionThD {
    /// Les émetteurs du projet (fiche 8.1), chauffage seul : lecture retenue au lot 6b.
    #[default]
    Projet,
    /// L'émission équivalente conventionnelle du calcul du Bbio (§8.1.3.1) : assemblage
    /// provisoire des lots 5 et 6a, conservé pour mesurer l'écart.
    ConventionnelleBbio,
}

/// Système de ventilation employé par le calcul Th-D (`docs/questions/Q042.md` ; Q40 point 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VentilationThD {
    /// Le système de ventilation du projet (fiches 6.2 à 6.5) : lecture retenue au lot 6a.
    #[default]
    Projet,
    /// Le système conventionnel du calcul du Bbio (fiche 6.1) : assemblage provisoire du lot 5,
    /// conservé pour mesurer l'écart.
    ConventionnelleBbio,
}

impl Default for Lectures {
    fn default() -> Self {
        Self {
            hysteresis: LECTURE_HYSTERESIS,
            saison_froid_th_d: LectureSaisonFroidThD::default(),
            debits: LectureDebitsBbio::default(),
            inerties_longues: true,
            fermeture_et: LECTURE_FERMETURE_ET,
            ventilation_th_d: VentilationThD::default(),
            emission_th_d: EmissionThD::default(),
            recuperation_th_d: true,
            preechauffage_ecs: LecturePreechauffageEcs::default(),
        }
    }
}

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
        Self::de_dans_batiment(groupe, catalogue, usage, &[])
    }

    /// Résout les composants d'un groupe d'un bâtiment dont `ids_espaces_tampons` donne, dans
    /// l'ordre, les identifiants des espaces tampons : chaque composant qui donne sur l'un
    /// d'eux en reçoit l'indice.
    pub fn de_dans_batiment(
        groupe: &Groupe,
        catalogue: &Catalogue,
        usage: UsageZone,
        ids_espaces_tampons: &[String],
    ) -> Self {
        let indice = |id: &Option<String>| {
            id.as_ref()
                .and_then(|id| ids_espaces_tampons.iter().position(|e| e == id))
        };
        let parois = groupe
            .parois_opaques
            .iter()
            .filter_map(|p| {
                let type_paroi = catalogue.parois.iter().find(|t| t.id == p.type_paroi)?;
                let mut r = ParoiResolue::nouvelle(p, type_paroi, catalogue);
                r.espace_tampon = indice(&p.espace_tampon);
                Some(r)
            })
            .collect();
        let mut ponts = Vec::new();
        for pont in &groupe.ponts_thermiques {
            let debut = ponts.len();
            PontResolu::resoudre(pont, catalogue, &mut ponts);
            for troncon in &mut ponts[debut..] {
                troncon.espace_tampon = indice(&pont.espace_tampon);
            }
        }
        let mut baies = Vec::new();
        let mut gestions = Vec::new();
        for b in &groupe.baies {
            let Some(type_baie) = catalogue.baies.iter().find(|t| t.id == b.type_baie) else {
                continue;
            };
            let Some(mut resolue) = BaieResolue::nouvelle(b, type_baie, catalogue) else {
                continue;
            };
            resolue.espace_tampon = indice(&b.espace_tampon);
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
    /// W/K (§5.21.3.6, éq. 340), sans les espaces tampons.
    pub fn h_th_k_pt_w_k(&self) -> f64 {
        EnsembleParois::de(&self.parois).h_th_w_k + EnsemblePonts::de(&self.ponts).h_th_w_k
    }

    /// « Het_th_int^gr » : coefficient de transmission du groupe vers l'espace tampon d'indice
    /// `i`, en W/K.
    ///
    /// Th-BCE 2020 §5.4.3.1, éq. (106) : `Σ Hges_set^b + Σ HTH_set^k + Σ HTH_set^l` sur les
    /// composants qui donnent sur l'espace tampon.
    pub fn h_et_th_int_w_k(&self, i: usize) -> f64 {
        let sur = |et: Option<usize>| et == Some(i);
        h_et_th_int_gr_w_k(
            self.baies
                .iter()
                .filter(|b| sur(b.espace_tampon))
                .map(BaieResolue::h_ges_set_w_k),
            self.parois
                .iter()
                .filter(|p| sur(p.espace_tampon))
                .map(ParoiResolue::h_th_w_k),
            self.ponts
                .iter()
                .filter(|p| sur(p.espace_tampon))
                .map(PontResolu::h_th_w_k),
        )
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
    /// Variation de l'énergie stockée dans les masses séquentielle et annuelle
    /// (`ΔCm,s−q·Δθms + ΔCm,a−s·Δθma`, §5.21.3.10) ; nulle hors calcul Th-D.
    pub stockage_inerties_longues_wh: f64,
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
            + self.stockage_inerties_longues_wh
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
    /// Confort d'été (§13.5) ; alimenté en calcul Th-D seulement.
    pub confort_ete: CumulsConfortEte,
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
    /// « θiich_+ » : consigne de chauffage en occupation normale, en °C.
    pub theta_ii_ch_plus_c: f64,
    /// « θext_base » : température extérieure de base du site, en °C (§3.1.3).
    pub theta_ext_base_c: f64,
    /// `{pch(t)}` pour `t = h` à `h + 6` : indicateurs de consigne de chauffage du pas courant
    /// et des pas à venir, pour les relances (§8.5).
    pub p_ch_avenir: [f64; HORIZON_RELANCE],
    /// `{pfr(t)}` pour `t = h` à `h + 6`.
    pub p_fr_avenir: [f64; HORIZON_RELANCE],
    /// « iocc_zone(h−1) » (éq. 812).
    pub i_occ_zone_h1: bool,
    /// « θcw(h) » : température de l'eau froide, en °C (§9.6, fichier météo).
    pub theta_cw_c: f64,
}

/// État d'un groupe d'un pas de temps au suivant.
#[derive(Debug, Clone, PartialEq)]
pub struct EtatGroupeHoraire {
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
    /// Masses séquentielle et annuelle (§5.21.3.10) ; inertes hors calcul Th-D.
    pub inerties: EtatInerties,
    /// « Autch,eff(j) » et « Autfr,eff(j) » : autorisations effectives du calcul des
    /// consommations (§8.4), fixées par le bâtiment ; égales aux autorisations propres dans les
    /// autres modes.
    pub aut_eff: (bool, bool),
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
    /// Inerties séquentielle et annuelle (§5.21.3.10) ; flux nuls hors calcul Th-D.
    pub inerties: FluxInerties,
    /// Confort d'été du pas de temps (§13.5) ; `None` hors calcul Th-D.
    pub confort_ete: Option<PasConfortEte>,
}

/// Groupe d'habitation prêt pour un calcul annuel au pas horaire, en mode Th-B ou Th-D.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupeHoraire {
    /// Identifiant du groupe.
    pub id: String,
    /// Mode de calcul : Th-B (Bbio) ou Th-D (confort d'été).
    pub mode: ModeCalcul,
    /// Vrai si l'usage de la zone est « de type habitation » au sens du confort d'été (période
    /// de sommeil de l'éq. 2544) : maison individuelle, logement collectif, établissement
    /// sanitaire avec hébergement.
    pub habitation: bool,
    /// « ihebergement » de l'usage de la zone (§4.1.3, tableau 4).
    pub hebergement: bool,
    /// Locaux du groupe au sens de l'éclairage (§7.1), hors habitation : puissances et gestion
    /// conventionnelles en calcul du Bbio (§7.1.3.1), celles du projet sinon (§7.1.3.2). Vide
    /// en habitation, dont l'éclairage est conventionnel (§7.1.3.6.2).
    pub locaux_eclairage: Vec<LocalResolu>,
    /// « Aeclnat2 » : surface des locaux du groupe ayant accès à la lumière naturelle, en m²
    /// (§7.1.3.3) ; `Agr` en habitation.
    pub a_eclnat2_m2: f64,
    /// « Aeclnat01 » ou « Aeclnat0 » : surface des locaux de grand volume du groupe ayant accès
    /// à la lumière naturelle, en m² (§7.1.3.3) ; 0 sans local de grand volume.
    pub a_eclnat_gv_m2: f64,
    /// Lectures du texte employées (celles retenues, sauf pour mesurer une sensibilité).
    pub lectures: Lectures,
    /// Inerties séquentielle et annuelle (§5.21.3.3 et 5.21.3.4).
    pub inerties: InertiesLongues,
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
    /// Débits spécifiques repris et soufflé du groupe, en m³/h : ceux du système de
    /// ventilation conventionnel en Th-B (§6.1 et §6.2), ceux du système du projet sinon,
    /// fixés par la zone (§6.2.3.4, éq. 428 et 429).
    pub debits: Debits,
    /// Débits conventionnels du calcul du Bbio hors habitation, en occupation puis en
    /// inoccupation au sens de la ventilation (§6.2.3.1.3, tableau 56-1) ; `None` en
    /// habitation. La zone en tire `debits` à chaque pas de temps du calcul Th-B.
    pub debits_bbio_non_residentiel: Option<(Debits, Debits)>,
    /// Données aérauliques du groupe (§5.6).
    pub aeraulique: GroupeAeraulique,
    /// « δtrav_surv » : groupe traversant au sens de la surventilation (tableau 43).
    pub traversant: bool,
    /// « Httf » : hauteur pour le tirage thermique, en m.
    pub h_ttf_m: f64,
    /// « Pocc » : part des baies sur lesquelles un occupant peut agir (figure 34).
    pub p_occ: f64,
    /// Indice de l'espace tampon par lequel transite l'air neuf des entrées d'air (§5.6.3.1, éq. 134).
    pub entree_air_espace_tampon: Option<usize>,
    /// Part de la surface d'ouverture de son espace tampon attribuée à chaque baie qui donne sur
    /// un espace tampon, au prorata des surfaces d'ouverture des baies (« Aouv_max_tp,i^b »,
    /// §5.5.2 et §5.7.2) ; 0 pour les autres baies. Voir
    /// [`GroupeHoraire::fixer_parts_ouverture_espaces_tampons`].
    pub parts_ouverture_et: Vec<f64>,
    /// « iclim » saisi.
    pub i_clim: bool,
    /// Systèmes du projet (émetteurs, relances, distributions du groupe), en Th-C et en Th-D ;
    /// `None` en Th-B ou si le groupe ne décrit pas d'émetteur.
    pub systemes: Option<SystemesGroupe>,
    /// « Φrecup_conv^gr » et « Φrecup_rad^gr » : pertes et consommations des systèmes
    /// récupérées par le groupe au pas de temps, en W (§11.1, éq. 2364 et 2365, calculées au
    /// pas précédent).
    pub recup_w: (f64, f64),
    /// État du groupe.
    pub etat: EtatGroupeHoraire,
    /// Cumuls annuels.
    pub cumuls: CumulsGroupe,
    ouvertures: Vec<OuvertureBaie>,
}

impl GroupeHoraire {
    /// Prépare un groupe d'habitation avec les lectures retenues.
    ///
    /// `mode` : Th-B ou Th-D ; `q4pa_surf_m3_h_m2` est la perméabilité de la zone ;
    /// `nbh_occ_ref`, le nombre d'heures d'occupation d'une semaine type de l'usage (éq. 66) ;
    /// `omega_initiale_kg_kg`, l'humidité de l'air du groupe avant le premier pas de temps.
    pub fn nouveau(
        groupe: &Groupe,
        catalogue: &Catalogue,
        usage: UsageZone,
        mode: ModeCalcul,
        q4pa_surf_m3_h_m2: f64,
        nbh_occ_ref: f64,
        omega_initiale_kg_kg: f64,
    ) -> Self {
        Self::nouveau_selon(
            groupe,
            catalogue,
            usage,
            mode,
            Lectures::default(),
            q4pa_surf_m3_h_m2,
            nbh_occ_ref,
            omega_initiale_kg_kg,
        )
    }

    /// Prépare un groupe d'habitation pour des lectures données du texte.
    #[allow(clippy::too_many_arguments)]
    pub fn nouveau_selon(
        groupe: &Groupe,
        catalogue: &Catalogue,
        usage: UsageZone,
        mode: ModeCalcul,
        lectures: Lectures,
        q4pa_surf_m3_h_m2: f64,
        nbh_occ_ref: f64,
        omega_initiale_kg_kg: f64,
    ) -> Self {
        Self::nouveau_dans_batiment(
            groupe,
            catalogue,
            usage,
            mode,
            lectures,
            q4pa_surf_m3_h_m2,
            nbh_occ_ref,
            omega_initiale_kg_kg,
            &[],
        )
    }

    /// Prépare un groupe d'habitation d'un bâtiment dont `ids_espaces_tampons` donne les
    /// identifiants des espaces tampons, dans l'ordre.
    #[allow(clippy::too_many_arguments)]
    pub fn nouveau_dans_batiment(
        groupe: &Groupe,
        catalogue: &Catalogue,
        usage: UsageZone,
        mode: ModeCalcul,
        lectures: Lectures,
        q4pa_surf_m3_h_m2: f64,
        nbh_occ_ref: f64,
        omega_initiale_kg_kg: f64,
        ids_espaces_tampons: &[String],
    ) -> Self {
        let enveloppe =
            EnveloppeResolue::de_dans_batiment(groupe, catalogue, usage, ids_espaces_tampons);
        let a_gr_m2 = groupe.su_m2.unwrap_or(0.0);
        let op = EnsembleParois::de(&enveloppe.parois);
        let ba = EnsembleBaies::de(&enveloppe.baies);
        let thermique =
            GroupeThermique::nouveau(a_gr_m2, ba.a_baies_m2, &groupe.inertie.quotidienne);
        // §6.2.3.1.3 : débits du Bbio — débits d'hygiène des bouches en habitation, débits
        // conventionnels du tableau 56-1 dans les autres usages, selon l'occupation.
        let sref_gr_m2 = if usage.est_habitation() {
            groupe.shab_m2
        } else {
            groupe.su_m2
        }
        .unwrap_or(0.0);
        let debits_bbio_non_residentiel = if usage.est_habitation() {
            None
        } else {
            debits_bbio_non_residentiel(usage.numero(), sref_gr_m2)
        };
        let debits = match debits_bbio_non_residentiel {
            Some((occ, _)) => occ,
            None => debits_specifiques_bbio_groupe(&groupe.bouches_conduits)
                .map(|d| debits_systeme_conventionnel(d, lectures.debits))
                .unwrap_or_default(),
        };
        let initiale = Temperatures {
            theta_i_c: THETA_INITIALE_C,
            theta_s_c: THETA_INITIALE_C,
            theta_mq_c: THETA_INITIALE_C,
            theta_rm_c: THETA_INITIALE_C,
            theta_op_c: THETA_INITIALE_C,
        };
        // §7.1.3.1 et 7.1.3.2 : locaux d'éclairage des usages autres que l'habitation.
        let locaux_eclairage: Vec<LocalResolu> = if usage.est_habitation() {
            Vec::new()
        } else {
            groupe
                .locaux
                .iter()
                .filter_map(|l| {
                    if mode == ModeCalcul::ThB {
                        LocalResolu::bbio(usage.numero(), l, a_gr_m2)
                    } else {
                        LocalResolu::projet(usage.numero(), l, a_gr_m2)
                    }
                })
                .collect()
        };
        let a_eclnat2 = if usage.est_habitation() {
            a_gr_m2
        } else {
            a_eclnat2_m2(&locaux_eclairage)
        };
        let a_eclnat_gv = a_eclnat_gv_m2(&locaux_eclairage);
        let nb_baies = enveloppe.baies.len();
        // Th-BCE 2020 §5.21.3.2 : « En mode Th-B et Th-C, les inerties séquentielle et annuelle
        // ne sont pas prises en compte. »
        let inerties = if mode == ModeCalcul::ThD && lectures.inerties_longues {
            InertiesLongues::nouvelles(a_gr_m2, &groupe.inertie)
        } else {
            InertiesLongues::default()
        };
        Self {
            id: groupe.id.clone(),
            mode,
            habitation: usage.sommeil_confort_ete(),
            hebergement: usage.i_hebergement(),
            locaux_eclairage,
            a_eclnat2_m2: a_eclnat2,
            a_eclnat_gv_m2: a_eclnat_gv,
            lectures,
            inerties,
            a_gr_m2,
            v_m3: groupe.v_m3.unwrap_or(0.0),
            h_th_k_pt_w_k: enveloppe.h_th_k_pt_w_k(),
            thermique,
            debits,
            debits_bbio_non_residentiel,
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
            etat: EtatGroupeHoraire {
                fin: initiale,
                theta_op_fin_h2_c: THETA_INITIALE_C,
                omega_i_fin_kg_kg: omega_initiale_kg_kg,
                t_max: TemperaturesMaximales::default(),
                r_ouv_theta_op: 0.0,
                // En Bbio, le refroidissement est possible sans système (décision Q35 point 7) ;
                // en Th-D, tout groupe est « passé en non climatisé » (§13.5.3.1) et la saison
                // de refroidissement suit la lecture de Q40. Le format d'entrée ne porte pas
                // encore `iclim` (lot 6).
                saisons: Saisons::nouveau_selon(
                    a_gr_m2,
                    nbh_occ_ref,
                    mode,
                    groupe.iclim.unwrap_or(false),
                    lectures.saison_froid_th_d,
                ),
                inerties: EtatInerties::default(),
                aut_eff: (false, false),
            },
            i_clim: groupe.iclim.unwrap_or(false),
            // Les indices des réseaux intergroupes sont fixés par le bâtiment
            // ([`GroupeHoraire::lier_systemes`]).
            systemes: (mode != ModeCalcul::ThB)
                .then(|| SystemesGroupe::nouveaux(groupe, usage, a_gr_m2, &Liens::default()))
                .flatten(),
            recup_w: (0.0, 0.0),
            cumuls: CumulsGroupe::default(),
            ouvertures: Vec::with_capacity(nb_baies),
            entree_air_espace_tampon: groupe
                .entrees_air
                .as_ref()
                .and_then(|e| e.espace_tampon.as_ref())
                .and_then(|id| ids_espaces_tampons.iter().position(|e| e == id)),
            parts_ouverture_et: vec![0.0; nb_baies],
            enveloppe,
        }
    }

    /// Fixe la part de la surface d'ouverture de chaque espace tampon attribuée aux baies du
    /// groupe qui donnent sur lui, au prorata des surfaces d'ouverture maximales des baies :
    /// `Aouvmax,b/Σ Aouvmax`, la somme portant sur toutes les baies du bâtiment qui donnent sur
    /// le même espace tampon (`totaux`, par espace tampon). Nomenclatures des §5.5.2 et §5.7.2,
    /// « Aouv_max_tp,i^b » ; décision Q41 point 2.
    pub fn fixer_parts_ouverture_espaces_tampons(&mut self, totaux: &[f64]) {
        for (part, b) in self
            .parts_ouverture_et
            .iter_mut()
            .zip(&self.enveloppe.baies)
        {
            *part = match b.espace_tampon.and_then(|i| totaux.get(i)) {
                Some(&total) if total > 0.0 => b.a_ouv_max_m2 / total,
                _ => 0.0,
            };
        }
    }

    /// Résout à nouveau les systèmes du groupe, `intergroupes` donnant l'indice d'un réseau
    /// intergroupes du bâtiment d'après son identifiant. Sans effet en Th-B.
    pub fn lier_systemes(&mut self, groupe: &Groupe, usage: UsageZone, liens: &Liens<'_>) {
        if self.mode != ModeCalcul::ThB {
            self.systemes = SystemesGroupe::nouveaux(groupe, usage, self.a_gr_m2, liens);
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
        self.pas_avec_espaces_tampons(c, apports, air_zone, &[])
    }

    /// Calcule un pas de temps du groupe, les coefficients des espaces tampons du bâtiment au
    /// pas de temps (§5.2, figure 15) étant donnés par `espaces_tampons`, dans l'ordre des
    /// indices des composants.
    pub fn pas_avec_espaces_tampons(
        &mut self,
        c: &ConditionsPas,
        apports: &ApportsGroupe,
        air_zone: &[AirEntrant],
        espaces_tampons: &[CoefficientsEt],
    ) -> Option<SortiesPasGroupe> {
        let et = |i: Option<usize>| i.and_then(|i| espaces_tampons.get(i));
        let h = c.ext.h;
        let sc = &c.scenarios;
        let i_occ_zone = sc.i_occ_zone > 0.0;
        let theta_e = c.ext.te_c;
        let theta_i_h1 = self.etat.fin.theta_i_c;
        let theta_op_h1 = self.etat.fin.theta_op_c;
        let mode = self.mode;
        // Th-BCE 2020 §13.5.3.3 et 13.5.3.4, éq. (2543) à (2547) : température d'inconfort
        // chaud et écart à la consigne de refroidissement, nul hors calcul Th-D.
        let conf_adapt_th_d = mode == ModeCalcul::ThD && c.ext.is_conf_adapt;
        let theta_op_conf_ch =
            theta_op_conf_ch_c(self.habitation, hleg(h), c.theta_ii_fr_plus_c, c.theta_rm_c);
        let delta_theta_conf_adapt = delta_theta_conf_adapt_c(
            mode,
            c.ext.is_conf_adapt,
            theta_op_conf_ch,
            c.theta_ii_fr_plus_c,
        );
        // Th-BCE 2020 §4.6.3.3, éq. (91) : en Bbio, autorisations effectives égales aux
        // autorisations propres ; elles valent de 9 h légales à 9 h légales le lendemain. Le
        // calcul Th-D emploie le même assemblage (Q40).
        // En calcul des consommations, autorisations effectives de la fiche 8.4, fixées par le
        // bâtiment ; la saison propre reste celle des protections mobiles et de l'ouverture des
        // baies (fiches 5.9 et 5.13).
        let saison = self.etat.saisons.saison();
        let (aut_ch, aut_fr) = if mode == ModeCalcul::ThC {
            self.etat.aut_eff
        } else {
            (self.etat.saisons.aut_ch, self.etat.saisons.aut_fr)
        };

        // §5.9.3.2, éq. (175) : température opérative maximale de la veille.
        self.etat.t_max.avancer(theta_op_h1, c.nouveau_jour);
        let saison_gpm =
            SaisonGpm::de_saison_effective(saison as u8).unwrap_or(SaisonGpm::MiSaison);

        // §5.15 à 5.20 : parois opaques et ponts thermiques, avec les coefficients des espaces
        // tampons (§5.17.3 et §5.19.3) ; HTH,k+pt varie alors à chaque pas de temps.
        let mut phi_sh_w = 0.0;
        let mut h_th_k_pt_w_k = 0.0;
        for p in &self.enveloppe.parois {
            let b = et(p.espace_tampon);
            let f = p.flux_reduit(&c.ext, b);
            phi_sh_w += f.phi_sh_s_w + f.phi_sh_vc_w;
            h_th_k_pt_w_k += p.h_th_reduit_w_k(b);
        }
        for p in &self.enveloppe.ponts {
            let b = et(p.espace_tampon);
            let f = p.flux_reduit(&c.ext, b);
            phi_sh_w += f.phi_sh_s_w + f.phi_sh_vc_w;
            h_th_k_pt_w_k += p.h_th_reduit_w_k(b);
        }

        // §5.13.3.1 : gestion manuelle de l'ouverture des baies ; règles du calcul Th-B
        // (§5.13.3.1.2) ou règles générales avec `Δθconf_adapt` en Th-D (§5.13.3.1.1).
        let ouverture = ConditionsOuverture {
            mode,
            i_occ_zone,
            hleg: hleg(h),
            saison_de_chauffage: saison == Saison::Chauffage,
            refroidissement_autorise: aut_fr,
            theta_ext_c: theta_e,
            theta_op_h1_c: theta_op_h1,
            theta_op_h2_c: self.etat.theta_op_fin_h2_c,
            theta_iifr_c: c.theta_ii_fr_plus_c,
            delta_theta_conf_adapt_c: delta_theta_conf_adapt,
        };
        let r_ouv_theta_op = r_ouv_theta_op_man_selon(
            self.lectures.hysteresis,
            self.etat.r_ouv_theta_op,
            &ouverture,
        );

        // §5.8 : baies. Protections mobiles (§5.9), flux (§5.10), ouverture (§5.13.3.2.3).
        let mut baies = FluxBaies::default();
        self.ouvertures.clear();
        for ((baie, gestion), part_et) in self
            .enveloppe
            .baies
            .iter()
            .zip(&self.enveloppe.gestions)
            .zip(&self.parts_ouverture_et)
        {
            let b_et = et(baie.espace_tampon);
            let incident = baie.incident(&c.ext);
            let protections = gestion.as_ref().map_or(Protections::OUVERTES, |g| {
                g.protections(&ConditionsGpm {
                    i_occ_zone,
                    // Indicateur à valeurs −1, 0 et 1 (éq. 53).
                    #[allow(clippy::cast_possible_truncation)]
                    i_occ_gpm: sc.i_occ_gpm as i8,
                    // §5.9.3, éq. (176) : Etp** = becl·Etp*.
                    etp_lux: incident.etp_lux() * b_et.map_or(1.0, |e| e.b_ecl),
                    v_vent_c_m_s: c.v_vent_c_m_s,
                    saison: saison_gpm,
                    top_j1_max_c: self.etat.t_max.top_j1_max_c,
                    // §5.9.3.6, éq. (201) à (204) et tableau 34.
                    confort_adaptatif_th_d: conf_adapt_th_d,
                })
            });
            let f = baie.flux_reduit(&incident, &c.ext, &protections, b_et);
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
                let mut a_ouv_m2 = a_ouv_baie_m2(r_ouv, baie.a_ouv_max_m2, a_ouv_max_pm);
                // Baie donnant sur un espace tampon : ouverture bornée par la part de la
                // surface d'ouverture de l'espace tampon qui lui revient (décision Q41
                // point 2).
                if let Some(e) = b_et {
                    a_ouv_m2 = a_ouv_m2.min(part_et * e.a_ouv_et_m2);
                }
                self.ouvertures.push(OuvertureBaie {
                    a_ouv_m2,
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

        // §7.1 : éclairage. Volumes normaux (baies de type 2) ; Aeclnat2 = Agr en habitation,
        // surface des locaux ayant accès à la lumière naturelle dans les autres usages.
        let einat = einat_vn_lux(
            baies.flt1_lm[2],
            baies.flt2_lm[2],
            baies.flt3_lm[2],
            self.a_eclnat2_m2,
        );
        // §7.1.3.4, éq. (783) : grands volumes, baies de types 0 et 1.
        let einat_gv = einat_gv_lux(
            [baies.flt1_lm[0], baies.flt2_lm[0], baies.flt3_lm[0]],
            [baies.flt1_lm[1], baies.flt2_lm[1], baies.flt3_lm[1]],
            self.a_eclnat_gv_m2,
        );
        let ecl = if self.locaux_eclairage.is_empty() {
            eclairage_habitation(sc.i_ecl > 0.0, einat, self.a_gr_m2)
        } else {
            eclairage_locaux_gv(&self.locaux_eclairage, sc.i_ecl.max(0.0), einat, einat_gv)
        };

        // §5.21.3.5 : air entrant équivalent (bilan aéraulique de la zone et surventilation).
        let mut flux_air = [AirEntrant {
            qm_kg_s: 0.0,
            theta_c: theta_e,
            omega_kg_kg: c.omega_e_kg_kg,
        }; 5];
        for (cible, source) in flux_air.iter_mut().take(4).zip(air_zone) {
            *cible = *source;
        }
        flux_air[4].qm_kg_s = qm_surv;
        let air = AirEquivalent::melange(&flux_air);

        // §5.21 : sollicitations et matrice de comportement.
        // §11.1.3 : pertes et consommations récupérées, calculées au pas précédent.
        let phi_conv_w =
            apports.phi_int_conv_w + apports.phi_int_occ_conv_w + ecl.f_ecl_c_w + self.recup_w.0;
        let phi_rad_w =
            apports.phi_int_rad_w + apports.phi_int_occ_rad_w + ecl.f_ecl_r_w + self.recup_w.1;
        let s = Sollicitations {
            air,
            theta_e_c: theta_e,
            h_th_k_pt_w_k,
            hg_es_w_k: baies.h_ges_w_k,
            phi_sl_w: baies.fs2_w + baies.ftvc_w,
            phi_sh_w,
            fs1_w: baies.fs1_w,
            fs3_w: baies.fs3_w,
            phi_conv_w,
            phi_rad_w,
        };
        let theta_mq_precedent = self.etat.fin.theta_mq_c;
        // §5.21.3.10 : inerties séquentielle et annuelle, en Th-D seulement (§5.21.3.2). Leurs
        // flux ne dépendent que des pas de temps précédents (éq. 353 à 360).
        let inerties = if mode == ModeCalcul::ThD {
            self.etat.inerties.avancer(&self.inerties)
        } else {
            FluxInerties::default()
        };
        let matrice =
            self.thermique
                .matrice(&s, theta_mq_precedent, inerties.phi_mq_seq_ann_w())?;
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

        // §8.5 : relances, en calcul des consommations seulement.
        let relance = match (&mut self.systemes, mode) {
            (Some(sy), ModeCalcul::ThC) => sy.relance.pas(&ConditionsRelance {
                aut_ch,
                aut_fr,
                p_ch: c.p_ch_avenir,
                p_fr: c.p_fr_avenir,
                theta_ext_c: theta_e,
                theta_ext_base_c: c.theta_ext_base_c,
                theta_ii_ch_plus_c: c.theta_ii_ch_plus_c,
                theta_ii_fr_plus_c: c.theta_ii_fr_plus_c,
            }),
            _ => ConsignesRelance::SANS_RELANCE,
        };
        // §8.1 : émission. En Th-B, émission équivalente conventionnelle (§8.1.3.1). En Th-C et
        // en Th-D, émetteurs équivalents du projet (éq. 797 à 799) ; en Th-D, le chauffage
        // n'est jamais autorisé en période de confort adaptatif (éq. 76, 77 et 80) et aucun
        // groupe n'est refroidi (§13.5.3.1 ; éq. 807 avec `iclim` faux).
        let (chaud, froid) = match (&self.systemes, mode, self.lectures.emission_th_d) {
            (None, ..)
            | (_, ModeCalcul::ThB, _)
            | (_, ModeCalcul::ThD, EmissionThD::ConventionnelleBbio) => {
                (EmetteurEquivalent::BBIO, EmetteurEquivalent::BBIO)
            }
            (Some(sy), ..) => (sy.equivalente.chaud, sy.equivalente.froid),
        };
        let e = emission_avec_relance(
            &matrice,
            self.a_gr_m2,
            &chaud,
            &froid,
            &Consignes {
                theta_ii_ch_c: sc.theta_ii_ch_c,
                theta_ii_fr_c: sc.theta_ii_fr_c,
                aut_ch,
                aut_fr,
                i_clim: refroidissement_possible(mode, self.i_clim),
            },
            &relance,
        );
        // §8.1.3.7, §8.1.3.8, §8.7 et §8.8 : énergies requises par émetteur, ventilateurs
        // locaux et distributions du groupe.
        let rho_i = rho_kg_m3(theta_i_h1, self.etat.omega_i_fin_kg_kg);
        if let Some(sy) = &mut self.systemes {
            sy.emettre(
                e.phi_util_w,
                mode == ModeCalcul::ThC,
                &ConditionsVentilateurs {
                    aut_ch,
                    aut_fr,
                    relance: en_relance(&relance),
                    i_occ: i_occ_zone,
                    i_occ_h1: c.i_occ_zone_h1,
                    idb_ch: e.phi_util_w > 0.0,
                    idb_fr: e.phi_util_w < 0.0,
                    a_gr_m2: self.a_gr_m2,
                },
                en_relance(&relance),
                e.moy.theta_i_c,
                rho_i,
            );
        }

        // §8.3 : bilan hydrique, en aval de l'émission. Apports des scénarios en kg/h.
        let echanges = EchangesHydriques {
            somme_q_maj_kg_s: air.qm_eq_kg_s,
            somme_q_omega_maj_kg_s: air.qm_eq_kg_s * air.omega_eq_kg_kg,
            a_omega_int_kg_s: (apports.omega_int_kg_h + apports.omega_int_occ_kg_h) / 3600.0,
            v_m3: self.v_m3,
            rho_i_kg_m3: rho_i,
        };
        let omega_fin = match (&mut self.systemes, mode) {
            // §8.3.4 : déshumidification sur les batteries froides en calcul des
            // consommations, puis énergie latente ajoutée aux besoins de froid (éq. 828).
            (Some(sy), ModeCalcul::ThC) => {
                let froid: Vec<_> = sy
                    .emetteurs
                    .iter()
                    .enumerate()
                    .filter(|(_, em)| em.froid.is_some())
                    .map(|(i, em)| {
                        let batterie = em.ventilateurs.map(|v| {
                            BatterieFroide::nouvelle(
                                sy.equivalente.rat_eff_fr[i],
                                sy.sorties.qm_recirc_kg_s[i],
                                sy.sorties.q_em_wh[i].1,
                                sy.theta_batt_dim_c(i),
                                e.moy.theta_i_c,
                                v.regulation_batterie == Some(RegulationBatterie::DebitRegule),
                            )
                        });
                        (sy.equivalente.rat_eff_fr[i], batterie)
                    })
                    .collect();
                let (fin, latent_froid) = bilan_th_c(
                    self.etat.omega_i_fin_kg_kg,
                    &froid,
                    e.phi_util_w < 0.0,
                    &echanges,
                );
                let mut latent = vec![0.0; sy.emetteurs.len()];
                let indices = sy
                    .emetteurs
                    .iter()
                    .enumerate()
                    .filter(|(_, em)| em.froid.is_some())
                    .map(|(i, _)| i);
                for (i, l) in indices.zip(latent_froid) {
                    latent[i] = l;
                }
                sy.distribuer(
                    &latent,
                    usize::from(sc.date.mois) - 1,
                    theta_e,
                    c.theta_ext_base_c,
                    espaces_tampons,
                );
                fin
            }
            (Some(sy), _) => {
                sy.distribuer(
                    &[],
                    usize::from(sc.date.mois) - 1,
                    theta_e,
                    c.theta_ext_base_c,
                    espaces_tampons,
                );
                omega_fin_bbio_kg_kg(self.etat.omega_i_fin_kg_kg, &echanges)
            }
            (None, _) => omega_fin_bbio_kg_kg(self.etat.omega_i_fin_kg_kg, &echanges),
        };
        // §9.5 à 9.7 : besoins d'ECS et distributions d'ECS du groupe (figure 24).
        if let Some(sy) = &mut self.systemes {
            sy.pas_ecs(
                sc.ah,
                sc.i_ecs,
                c.theta_cw_c,
                e.moy.theta_i_c,
                theta_e,
                espaces_tampons,
            );
        }

        self.cumuler_bilan(
            &s,
            &e.moy,
            e.phi_util_w,
            theta_mq_precedent,
            e.fin.theta_mq_c,
        );
        self.cumuls.bilan.stockage_inerties_longues_wh += inerties.stockage_wh;

        // §13.5.3.5 à 13.5.3.8 : confort d'été, sur la température opérative à la fin du pas
        // de temps (§13.5.2 : « θop(h), température opérative du groupe à la fin du pas de
        // temps h »).
        let confort_ete = (mode == ModeCalcul::ThD).then(|| {
            PasConfortEte::nouveau(
                c.ext.is_conf_adapt,
                i_occ_zone,
                e.fin.theta_op_c,
                theta_op_conf_ch_corr_c(theta_op_conf_ch, DELTA_THETA_CORR_SYST_C),
            )
        });
        if let Some(p) = &confort_ete {
            self.cumuls.confort_ete.ajouter(p);
        }

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
            self.etat.saisons.nouveau_jour(j, conf_adapt_th_d);
        }

        // État pour le pas de temps suivant.
        self.etat.theta_op_fin_h2_c = theta_op_h1;
        self.etat.fin = e.fin;
        self.etat.omega_i_fin_kg_kg = omega_fin;
        self.etat.r_ouv_theta_op = r_ouv_theta_op;
        if mode == ModeCalcul::ThD {
            self.etat.inerties.enregistrer(e.fin.theta_mq_c);
        }

        Some(SortiesPasGroupe {
            q_sys_ch_wh: e.q_sys_ch_wh,
            q_sys_fr_wh: e.q_sys_fr_wh,
            c_ecl_kwh: ecl.c_ecl_kwh,
            theta_op_moy_00_c: theta_op_moy_00,
            moy: e.moy,
            fin: e.fin,
            qv_surv_m3_h: qv_surv,
            saison,
            inerties,
            confort_ete,
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
        b.apports_opaques_wh += s.phi_sh_w * g.rapport_emq(s.h_th_k_pt_w_k).unwrap_or(0.0);
        b.chauffage_wh += phi_util_w.max(0.0);
        b.refroidissement_wh += (-phi_util_w).max(0.0);
        // φsl = Fs2 + Ftvc entre au nœud θs avec la transmission des baies (éq. 342 et 343).
        b.pertes_baies_wh += s.hg_es_w_k * (moy.theta_s_c - s.theta_e_c) - s.phi_sl_w;
        b.pertes_opaques_wh += hg_emq * (moy.theta_mq_c - s.theta_e_c);
        b.pertes_air_wh += s.air.hg_ei_w_k * (moy.theta_i_c - s.air.theta_ei_eq_c);
        b.stockage_wh += g.c_mq_kj_k / 3.6 * (theta_mq_fin_c - theta_mq_precedent_c);
    }
}
