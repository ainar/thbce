// SPDX-License-Identifier: GPL-3.0-or-later
//! Saisons propres d'un groupe : fiche `C_Ein_Détermination des saisons` (Th-BCE 2020 §4.6).
//!
//! Chaque jour, à 9 h légales, la fiche décide si le groupe est autorisé à chauffer
//! (`Autch,pro`) et à refroidir (`Autfr,pro`). Le démarrage d'une saison se décide sur une somme
//! glissante de degrés-heures d'inconfort ; son arrêt, sur une moyenne glissante des besoins.
//!
//! Périmètre : autorisations propres. En calcul du Bbio, les autorisations effectives leur sont
//! égales (§4.6.3.3, éq. 91), sans égard pour la présence d'un système de refroidissement, et
//! les besoins sont ceux de l'émission, `Qsys_ch` et `Qsys_fr`. Le calcul Th-D (lot 5) emploie
//! la même procédure, avec les règles propres au Th-D des éq. (76), (77), (80), (88) et (90). En calcul des consommations,
//! les autorisations effectives se décident au niveau de la génération (lot 6). Les
//! degrés-heures d'inconfort viennent de la fiche des indicateurs de confort (§4.5) ; ils sont
//! ici des entrées.

use crate::conventions::ModeCalcul;

/// « Seuil_debut_ch » et « Seuil_debut_fr » : seuil de démarrage d'une saison, en °C.h (§4.6.2).
pub const SEUIL_DEBUT_C_H: f64 = 40.0;
/// « Seuil_fin_ch » et « Seuil_fin_fr » : seuil d'arrêt d'une saison, en Wh/m² (§4.6.2).
pub const SEUIL_FIN_WH_M2: f64 = 2.0;
/// « Seuil_Nbredemarrage » : nombre de redémarrages du chauffage admis au printemps (éq. 67).
pub const SEUIL_NB_REDEMARRAGE: u8 = 1;
/// Nombre de jours de la fenêtre glissante : 4 semaines.
pub const NB_JOURS_FENETRE: usize = 28;
const FENETRE: u16 = 28;

/// Premier jour où l'arrêt du chauffage peut être décrété (§4.6.3.2 : 57e jour).
pub const J_ARRET_CH_DEBUT: u16 = 57;
/// Premier jour d'interdiction de chauffer (183e jour).
pub const J_INTERDICTION_CH: u16 = 183;
/// Premier jour où le démarrage du chauffage peut être décrété (253e jour).
pub const J_DEMARRAGE_CH: u16 = 253;
/// Premier jour où le démarrage du refroidissement peut être décrété (8e jour).
pub const J_DEMARRAGE_FR: u16 = 8;
/// Premier jour d'autorisation inconditionnelle de refroidir (183e jour).
pub const J_AUTORISATION_FR: u16 = 183;
/// Premier jour où l'arrêt du refroidissement peut être décrété (246e jour).
pub const J_ARRET_FR: u16 = 246;

/// « Saison(j) » : saison propre du groupe (§4.6.3.5, tableau 11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Saison {
    /// 1 : saison de refroidissement.
    Refroidissement = 1,
    /// 2 : mi-saison.
    MiSaison = 2,
    /// 3 : saison de chauffage.
    Chauffage = 3,
    /// 4 : saison mixte de chauffage et de refroidissement.
    Mixte = 4,
}

impl Saison {
    /// Th-BCE 2020 §4.6.3.5, tableau 11.
    pub fn de(aut_ch: bool, aut_fr: bool) -> Self {
        match (aut_ch, aut_fr) {
            (false, true) => Self::Refroidissement,
            (false, false) => Self::MiSaison,
            (true, false) => Self::Chauffage,
            (true, true) => Self::Mixte,
        }
    }
}

/// Cumuls d'une journée, de 9 h légales la veille à 9 h légales le jour même.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Journee {
    /// Somme des `iocc_zone(h)`, en h.
    pub nbh_occ: f64,
    /// Somme des `Δθop_inc_cons,ch(h)` : inconfort froid, en °C.h.
    pub dh_ch_c_h: f64,
    /// Somme des `Δθop_inc,fr(h)` : inconfort chaud, en °C.h.
    pub dh_fr_c_h: f64,
    /// Somme des `Qsys_ch(h)` : besoins de chauffage, en Wh.
    pub q_ch_wh: f64,
    /// Somme des `Qsys_fr(h)` : besoins de refroidissement, en Wh.
    pub q_fr_wh: f64,
}

impl Journee {
    fn ajouter(&mut self, autre: &Self) {
        self.nbh_occ += autre.nbh_occ;
        self.dh_ch_c_h += autre.dh_ch_c_h;
        self.dh_fr_c_h += autre.dh_fr_c_h;
        self.q_ch_wh += autre.q_ch_wh;
        self.q_fr_wh += autre.q_fr_wh;
    }
}

/// Le système d'émission du groupe peut-il refroidir (« iclim » de l'éq. 807) ?
///
/// Th-BCE 2020 §4.6.3.3, éq. (91), cas particulier du calcul du Bbio : le Bbio ne décrivant
/// aucun système, tout groupe y est traité comme refroidi (décision du mainteneur,
/// `docs/questions.md`, Q35 point 7 et Q38 point 3). En Th-C, « pas de système, pas de froid ».
/// En Th-D, §13.5.3.1 : « dans le cas d'un groupe climatisé, le groupe est passé en non
/// climatisé et le système de climatisation est désactivé » : jamais de refroidissement.
pub fn refroidissement_possible(mode: ModeCalcul, i_clim: bool) -> bool {
    match mode {
        ModeCalcul::ThB => true,
        ModeCalcul::ThC => i_clim,
        ModeCalcul::ThD => false,
    }
}

/// Lecture de l'autorisation propre de refroidir en calcul Th-D (`docs/questions.md`, Q40).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LectureSaisonFroidThD {
    /// Lecture retenue : en Th-D, `Autfr,pro` suit la procédure de la fiche, avec ses règles
    /// propres au Th-D (éq. 88 et 90), que le groupe soit climatisé ou non. La saison de
    /// refroidissement est alors la saison d'été des occupants (protections mobiles, ouverture
    /// des baies), sans qu'aucun système ne refroidisse. C'est la seule lecture qui donne un
    /// effet aux règles Th-D des éq. (88) et (90), tous les groupes étant « passés en non
    /// climatisé » dans ce mode (§13.5.3.1).
    #[default]
    ProcedureDeLaFiche,
    /// Variante littérale : « dans le cas où le groupe ne dispose pas de système de
    /// refroidissement (iclim = 0), la variable est nulle toute l'année » (§4.6.3.2) ; le groupe
    /// n'est jamais en saison de refroidissement et les protections mobiles suivent en été les
    /// lignes de mi-saison.
    NulleSansSysteme,
}

/// Le groupe peut-il recevoir une autorisation propre de refroidir (`Autfr,pro`) ?
///
/// Th-BCE 2020 §4.6.3.2 : « Dans le cas où le groupe ne dispose pas de système de
/// refroidissement (iclim = 0), la variable Autfr,pro(j) est nulle toute l'année. » En calcul du
/// Bbio, la procédure s'applique sans égard pour `iclim` (éq. 91, décision Q35 point 7). En
/// Th-D : voir [`LectureSaisonFroidThD`].
pub fn autorisation_propre_de_refroidir_possible(
    mode: ModeCalcul,
    i_clim: bool,
    lecture: LectureSaisonFroidThD,
) -> bool {
    match (mode, lecture) {
        (ModeCalcul::ThB, _) | (ModeCalcul::ThD, LectureSaisonFroidThD::ProcedureDeLaFiche) => true,
        (ModeCalcul::ThC, _) | (ModeCalcul::ThD, LectureSaisonFroidThD::NulleSansSysteme) => i_clim,
    }
}

/// État des saisons propres d'un groupe.
#[derive(Debug, Clone, PartialEq)]
pub struct Saisons {
    a_gr_m2: f64,
    nbh_occ_ref: f64,
    mode: ModeCalcul,
    froid_possible: bool,
    historique: [Journee; NB_JOURS_FENETRE],
    en_cours: Journee,
    /// « Autch,pro(j) ».
    pub aut_ch: bool,
    /// « Autfr,pro(j) ».
    pub aut_fr: bool,
    jours_ch_consecutifs: u16,
    nb_redemarrage: u8,
    debut_mgdh_ch: Option<u16>,
    jour_redemarrage: Option<u16>,
    /// « jdebut_ch » : dernier jour où la saison de chauffage a commencé (éq. 92).
    pub j_debut_ch: Option<u16>,
    /// « jfin_ch » : dernier jour où la saison de chauffage s'est arrêtée.
    pub j_fin_ch: Option<u16>,
    /// « jdebut_fr ».
    pub j_debut_fr: Option<u16>,
    /// « jfin_fr ».
    pub j_fin_fr: Option<u16>,
}

impl Saisons {
    /// État initial : chauffage autorisé, refroidissement interdit (§4.6.3.2 et 4.6.3.3).
    ///
    /// `nbh_occ_ref` est le nombre d'heures d'occupation d'une semaine type de l'usage
    /// (§4.6.3.1, éq. 66) ; `i_clim` indique si le groupe dispose d'un système de
    /// refroidissement : sinon, en calcul des consommations, `Autfr,pro` est nul toute l'année
    /// ([`autorisation_propre_de_refroidir_possible`], avec la lecture retenue du calcul Th-D).
    pub fn nouveau(a_gr_m2: f64, nbh_occ_ref: f64, mode: ModeCalcul, i_clim: bool) -> Self {
        Self::nouveau_selon(
            a_gr_m2,
            nbh_occ_ref,
            mode,
            i_clim,
            LectureSaisonFroidThD::default(),
        )
    }

    /// État initial, pour une lecture donnée de l'autorisation de refroidir en Th-D.
    pub fn nouveau_selon(
        a_gr_m2: f64,
        nbh_occ_ref: f64,
        mode: ModeCalcul,
        i_clim: bool,
        lecture: LectureSaisonFroidThD,
    ) -> Self {
        Self {
            a_gr_m2,
            nbh_occ_ref,
            mode,
            froid_possible: autorisation_propre_de_refroidir_possible(mode, i_clim, lecture),
            historique: [Journee::default(); NB_JOURS_FENETRE],
            en_cours: Journee::default(),
            aut_ch: true,
            aut_fr: false,
            jours_ch_consecutifs: 0,
            nb_redemarrage: 0,
            debut_mgdh_ch: None,
            jour_redemarrage: None,
            j_debut_ch: None,
            j_fin_ch: None,
            j_debut_fr: None,
            j_fin_fr: None,
        }
    }

    /// Saison propre courante (tableau 11).
    pub fn saison(&self) -> Saison {
        Saison::de(self.aut_ch, self.aut_fr)
    }

    /// « Autch,eff(j) » et « Autfr,eff(j) » : autorisations effectives de chauffer et de
    /// refroidir, dans le cas particulier du calcul du Bbio.
    ///
    /// Th-BCE 2020 §4.6.3.3, éq. (91) : `Autch,eff(j) = Autch,pro(j)` et
    /// `Autfr,eff(j) = Autfr,pro(j)`. Hors calcul du Bbio, les autorisations effectives viennent
    /// de la génération qui dessert le groupe : `None`.
    pub fn autorisations_effectives_bbio(&self) -> Option<(bool, bool)> {
        (self.mode == ModeCalcul::ThB).then_some((self.aut_ch, self.aut_fr))
    }

    /// Nombre de journées closes sur lesquelles porte `MGbch` au jour `j`.
    ///
    /// Th-BCE 2020 §4.6.3.1.2 : 28 jours (éq. 71, 72 et 75) ; après un redémarrage au jour
    /// `jr`, « le calcul commence une semaine après le redémarrage » avec
    /// `Nbh_total_somme_ch = 168` (éq. 73), puis s'étend d'un jour par jour (éq. 74) jusqu'à
    /// 28 jours. L'éq. (73) écrit une somme de 9 h (j − 1) à 9 h (j) : coquille, la somme porte
    /// sur les 168 h du dénominateur, de 9 h (j − 7) à 9 h (j) (Q35 point 3).
    fn nb_jours_mgb_ch(&self, j: u16) -> u16 {
        self.jour_redemarrage
            .map_or(FENETRE, |jr| j.saturating_sub(jr).clamp(1, FENETRE))
    }

    /// « MGbch(j) » : moyenne glissante des besoins de chauffage, en Wh/m² par heure, telle que
    /// la verrait un test fait au jour `j` après la clôture de la journée.
    ///
    /// Th-BCE 2020 §4.6.3.1.2, éq. (71) à (75) : somme des `Qsys_ch(h)` de la fenêtre, divisée
    /// par le nombre d'heures et par `Agr`. `Qsys_ch(h)` est une puissance horaire en W tenue
    /// une heure, soit des Wh ; la moyenne est en Wh/m², unité du seuil d'arrêt (Q35 point 1).
    pub fn mgb_ch_wh_m2(&self, j: u16) -> f64 {
        let n = self.nb_jours_mgb_ch(j);
        self.somme(n).q_ch_wh / (f64::from(n) * 24.0) / self.a_gr_m2
    }

    /// « MGbfr(j) » : moyenne glissante des besoins de refroidissement sur 28 jours, en Wh/m²
    /// par heure. Th-BCE 2020 §4.6.3.2, éq. (85) et (86).
    pub fn mgb_fr_wh_m2(&self) -> f64 {
        self.somme(FENETRE).q_fr_wh / (f64::from(FENETRE) * 24.0) / self.a_gr_m2
    }

    /// Ajoute les grandeurs d'un pas de temps au cumul de la journée en cours.
    pub fn accumuler(&mut self, pas: &Journee) {
        self.en_cours.ajouter(pas);
    }

    /// Cumul des `n` dernières journées closes (au plus 28).
    fn somme(&self, n: u16) -> Journee {
        let mut s = Journee::default();
        for j in self.historique.iter().rev().take(usize::from(n)) {
            s.ajouter(j);
        }
        s
    }

    /// Seuil de démarrage, modulé par l'occupation de la fenêtre.
    ///
    /// Th-BCE 2020 §4.6.3, éq. (78), (81) et (88) :
    /// `Seuil_debut × max(0,5 ; Nbh_occ_somme/(4 × Nbh_occ_ref))`.
    fn seuil_debut(&self, nbh_occ_somme: f64) -> f64 {
        let ratio = if self.nbh_occ_ref > 0.0 {
            nbh_occ_somme / (4.0 * self.nbh_occ_ref)
        } else {
            0.0
        };
        SEUIL_DEBUT_C_H * ratio.max(0.5)
    }

    /// Clôt la journée et décide des autorisations du jour `j` (de 1 à 365). À appeler au pas de
    /// temps où `hleg = 9 h`.
    ///
    /// `confort_adaptatif_th_d` : vrai en calcul Th-D pendant la période de confort adaptatif.
    ///
    /// Th-BCE 2020 §4.6.3.2, chauffage :
    /// - Th-D en période de confort adaptatif : jamais autorisé (éq. 76, 77 et 80) ;
    /// - jours 1 à 56 : autorisé (éq. 76) ;
    /// - jours 57 à 182 (éq. 78) : si le chauffage est en fonction, il s'arrête quand la moyenne
    ///   glissante des besoins `MGbch` est inférieure ou égale à 2 Wh/m² et qu'il fonctionne
    ///   depuis au moins sept jours ; s'il est coupé, il redémarre, une fois au plus, quand la
    ///   somme glissante des degrés-heures d'inconfort froid `MGdh_ch` atteint le seuil ;
    /// - jours 183 à 252 : interdit (éq. 79) ;
    /// - à partir du jour 253 (éq. 81) : démarre quand `MGdh_ch` atteint le seuil, puis reste en
    ///   fonction.
    ///
    /// §4.6.3.2, refroidissement, si le groupe peut recevoir l'autorisation
    /// ([`autorisation_propre_de_refroidir_possible`]) :
    /// - jours 1 à 7 : interdit (éq. 87) ;
    /// - jours 8 à 182 (éq. 88) : démarre quand `MGdh_fr` atteint le seuil, ou, en Th-D, en
    ///   période de confort adaptatif ; puis reste en fonction ;
    /// - jours 183 à 245 : autorisé (éq. 89) ;
    /// - à partir du jour 246 (éq. 90) : s'arrête quand `MGbfr` est inférieure ou égale à
    ///   2 Wh/m², sauf, en Th-D, en période de confort adaptatif.
    ///
    /// Fenêtres glissantes (éq. 68 à 75 et 82 à 86) : une somme ou une moyenne s'étend de jour en
    /// jour depuis le début de son calcul, jusqu'à 28 jours, puis glisse. `MGdh_ch` commence au
    /// jour 253 ou, au printemps, le deuxième jour après un arrêt ; `MGdh_fr` au jour 8 ;
    /// `MGbch` porte sur les 28 jours précédents ou, après un redémarrage, sur les jours écoulés
    /// depuis celui-ci ; `MGbfr` sur les 28 jours précédents. Lectures : `docs/questions.md`, Q35.
    pub fn nouveau_jour(&mut self, j: u16, confort_adaptatif_th_d: bool) -> Saison {
        // Clôture de la journée « 9 h (j−1) à 9 h (j) ».
        self.historique.rotate_left(1);
        self.historique[NB_JOURS_FENETRE - 1] = self.en_cours;
        self.en_cours = Journee::default();
        let fenetre = |depuis: u16| (j + 1 - depuis).min(FENETRE);

        // Chauffage.
        let ch_prec = self.aut_ch;
        let froid_ressenti = |s: &Self, depuis: u16| {
            let cumul = s.somme(fenetre(depuis));
            cumul.dh_ch_c_h >= s.seuil_debut(cumul.nbh_occ)
        };
        let aut_ch = if confort_adaptatif_th_d {
            false
        } else if j < J_ARRET_CH_DEBUT {
            true
        } else if j < J_INTERDICTION_CH {
            if ch_prec {
                let mgb_ch = self.mgb_ch_wh_m2(j);
                !(mgb_ch <= SEUIL_FIN_WH_M2 && self.jours_ch_consecutifs >= 7)
            } else {
                let redemarre = self.nb_redemarrage < SEUIL_NB_REDEMARRAGE
                    && self
                        .debut_mgdh_ch
                        .is_some_and(|d| j >= d && froid_ressenti(self, d));
                if redemarre {
                    self.nb_redemarrage += 1;
                    self.jour_redemarrage = Some(j);
                }
                redemarre
            }
        } else if j < J_DEMARRAGE_CH {
            false
        } else {
            ch_prec || froid_ressenti(self, J_DEMARRAGE_CH)
        };
        if ch_prec && !aut_ch {
            // Le calcul de MGdh_ch débute le deuxième jour suivant l'arrêt.
            self.debut_mgdh_ch = Some(j + 2);
            self.j_fin_ch = Some(j);
        }
        if !ch_prec && aut_ch {
            self.j_debut_ch = Some(j);
        }
        self.jours_ch_consecutifs = if aut_ch {
            self.jours_ch_consecutifs.saturating_add(1)
        } else {
            0
        };
        self.aut_ch = aut_ch;

        // Refroidissement.
        let fr_prec = self.aut_fr;
        let aut_fr = if !self.froid_possible || j < J_DEMARRAGE_FR {
            false
        } else if j < J_AUTORISATION_FR {
            fr_prec || confort_adaptatif_th_d || {
                let cumul = self.somme(fenetre(J_DEMARRAGE_FR));
                cumul.dh_fr_c_h >= self.seuil_debut(cumul.nbh_occ)
            }
        } else if j < J_ARRET_FR {
            true
        } else if fr_prec {
            let mgb_fr = self.mgb_fr_wh_m2();
            !(mgb_fr <= SEUIL_FIN_WH_M2 && !confort_adaptatif_th_d)
        } else {
            false
        };
        if fr_prec && !aut_fr {
            self.j_fin_fr = Some(j);
        }
        if !fr_prec && aut_fr {
            self.j_debut_fr = Some(j);
        }
        self.aut_fr = aut_fr;
        self.saison()
    }
}
