// SPDX-License-Identifier: GPL-3.0-or-later
//! Assemblage des composants d'une zone : fiche `S2_BAT-assemblage_zone` (Th-BCE 2020 §5.5,
//! figure 18), pour le calcul du Bbio (mode Th-B) et pour le calcul du confort d'été (mode
//! Th-D), qui emploie le même assemblage (`docs/questions.md`, Q40).
//!
//! À chaque pas de temps : climat et scénarios de la zone ; bilan aéraulique de la zone (§5.6)
//! avec les débits du système de ventilation conventionnel (§6.1 et §6.2) et les températures
//! des groupes au pas précédent ; puis comportement thermique de chaque groupe (§5.7), dans
//! l'ordre de la figure 20 : les groupes qui cèdent de l'air au hall, le hall, puis les groupes
//! qui en reçoivent.
//!
//! Périmètre : calculs Th-B et Th-D des usages d'habitation ; hall fictif (aucun groupe de
//! circulation saisi). Les espaces tampons sont calculés au niveau du bâtiment, avant les zones
//! ([`super::assemblage_batiment`]) ; la zone reçoit leurs coefficients.

use crate::climat::calendrier::{jour, nouveau_jour_legal};
use crate::climat::environnement_proche::{ventc_m_s, Soleil};
use crate::climat::{ClimatExterieur, ConfortAdaptatif};
use crate::conventions::{ModeCalcul, NB_HEURES_AN};
use crate::entree::{Catalogue, Zone};
use crate::erreur::Erreur;
use crate::scenarios::ScenariosZone;
use crate::ventilation::debits_air_cep::{
    flux_entrants, hall_fictif, trav_zone_conventionnel, EtatGroupe, Exterieur, ZoneAeraulique,
};
use crate::ventilation::ven_bbio::{omega_air_souffle_kg_kg, t_air_souffle_c};

use super::assemblage_groupe::{
    nbh_occ_ref, ConditionsPas, GroupeHoraire, Lectures, SortiesPasGroupe,
};
use super::espace_tampon::espace_tampon::CoefficientsEt;
use super::ConditionsExterieures;

/// Zone d'habitation prête pour un calcul annuel au pas horaire, en mode Th-B ou Th-D.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneHoraire {
    /// Identifiant de la zone.
    pub id: String,
    /// Mode de calcul : Th-B (Bbio) ou Th-D (confort d'été).
    pub mode: ModeCalcul,
    /// Scénarios conventionnels de la zone (§4.1).
    pub scenarios: ScenariosZone,
    /// Modèle aéraulique de la zone (§5.6).
    pub aeraulique: ZoneAeraulique,
    /// Groupes de la zone.
    pub groupes: Vec<GroupeHoraire>,
    /// Sorties de chaque groupe au dernier pas de temps calculé.
    pub sorties: Vec<Option<SortiesPasGroupe>>,
    etats: Vec<EtatGroupe>,
}

impl ZoneHoraire {
    /// Prépare une zone d'habitation validée, avec les lectures retenues du texte.
    ///
    /// `mode` : Th-B ou Th-D ; `alt_m` est l'altitude du projet ; `omega_initiale_kg_kg`,
    /// l'humidité de l'air des groupes avant le premier pas de temps (celle de l'air extérieur
    /// au premier pas de temps, décision Q39 point 4).
    pub fn nouvelle(
        zone: &Zone,
        catalogue: &Catalogue,
        mode: ModeCalcul,
        alt_m: f64,
        omega_initiale_kg_kg: f64,
    ) -> Result<Self, Erreur> {
        Self::nouvelle_selon(
            zone,
            catalogue,
            mode,
            Lectures::default(),
            alt_m,
            omega_initiale_kg_kg,
        )
    }

    /// Prépare une zone d'habitation validée pour des lectures données du texte (mesure des
    /// sensibilités).
    pub fn nouvelle_selon(
        zone: &Zone,
        catalogue: &Catalogue,
        mode: ModeCalcul,
        lectures: Lectures,
        alt_m: f64,
        omega_initiale_kg_kg: f64,
    ) -> Result<Self, Erreur> {
        Self::nouvelle_dans_batiment(
            zone,
            catalogue,
            mode,
            lectures,
            alt_m,
            omega_initiale_kg_kg,
            &[],
        )
    }

    /// Prépare une zone d'un bâtiment dont `ids_espaces_tampons` donne les identifiants des
    /// espaces tampons, dans l'ordre (voir [`super::assemblage_batiment`]).
    pub fn nouvelle_dans_batiment(
        zone: &Zone,
        catalogue: &Catalogue,
        mode: ModeCalcul,
        lectures: Lectures,
        alt_m: f64,
        omega_initiale_kg_kg: f64,
        ids_espaces_tampons: &[String],
    ) -> Result<Self, Erreur> {
        if mode == ModeCalcul::ThC {
            return Err(Erreur::Calcul(
                "le calcul des consommations (Th-C) n'est pas encore pris en charge".to_owned(),
            ));
        }
        let a_gr: Vec<f64> = zone
            .groupes
            .iter()
            .map(|g| g.su_m2.unwrap_or(0.0))
            .collect();
        let scenarios = ScenariosZone::nouveau(zone.usage, &a_gr, zone.nb_lgt.unwrap_or(1))?;
        let nbh_occ_ref = nbh_occ_ref(scenarios.usage().occupation.grille_hebdo());
        let q4pa = zone.q4pa_surf_m3_h_m2.unwrap_or(0.0);
        let groupes: Vec<GroupeHoraire> = zone
            .groupes
            .iter()
            .map(|g| {
                GroupeHoraire::nouveau_dans_batiment(
                    g,
                    catalogue,
                    zone.usage,
                    mode,
                    lectures,
                    q4pa,
                    nbh_occ_ref,
                    omega_initiale_kg_kg,
                    ids_espaces_tampons,
                )
            })
            .collect();
        let aerauliques: Vec<_> = groupes.iter().map(|g| g.aeraulique).collect();
        let aeraulique = ZoneAeraulique::nouvelle(
            &aerauliques,
            // Th-BCE 2020 §5.6.3.1.3, tableau 22 : conventionnel, sauf en logement collectif.
            trav_zone_conventionnel(zone.usage.numero())
                .or(zone.trav_zone)
                .unwrap_or(true),
            // Éq. (138) : pas de perméabilité verticale en usage d'habitation.
            zone.usage.est_habitation(),
            alt_m,
            zone.h_plancher_m.unwrap_or(0.0),
            zone.h_zone_m.unwrap_or(0.0),
        );
        let nb = groupes.len();
        Ok(Self {
            id: zone.id.clone(),
            mode,
            scenarios,
            aeraulique,
            groupes,
            sorties: vec![None; nb],
            etats: Vec::with_capacity(nb),
        })
    }

    /// Calcule le pas de temps `h` de la zone dans son mode de calcul.
    ///
    /// `climat` est le climat du site pour le jeu météo du mode (« Th-BC » en Th-B, « Th-D » en
    /// Th-D) et `confort` la moyenne glissante de la température extérieure qui s'en déduit.
    pub fn pas(
        &mut self,
        h: usize,
        climat: &ClimatExterieur,
        confort: &ConfortAdaptatif,
    ) -> Result<(), Erreur> {
        self.pas_avec_espaces_tampons(h, climat, confort, &[])
    }

    /// « iocc_zone(h) » : indicateur d'occupation de la zone au pas de temps (§4.1.3).
    pub fn i_occ_zone(&self, h: usize) -> f64 {
        self.scenarios.evaluer(h).i_occ_zone
    }

    /// Calcule le pas de temps `h` de la zone, les coefficients des espaces tampons du
    /// bâtiment au même pas de temps étant connus (§5.2, figure 15 : les espaces tampons sont
    /// calculés avant les zones).
    pub fn pas_avec_espaces_tampons(
        &mut self,
        h: usize,
        climat: &ClimatExterieur,
        confort: &ConfortAdaptatif,
        espaces_tampons: &[CoefficientsEt],
    ) -> Result<(), Erreur> {
        let ext = conditions_exterieures(climat, confort, h, self.mode);
        let scenarios = self.scenarios.evaluer(h);
        let c = ConditionsPas {
            ext,
            omega_e_kg_kg: climat.we_kg_kg[h],
            v_vent_c_m_s: ventc_m_s(climat.vent_m_s[h]),
            // §4.5.3.2.1 : θrm est calculée au premier pas de temps du jour (IHJ = 1).
            theta_rm_c: confort.theta_rm_c.get(jour(h)).copied().unwrap_or(0.0),
            nouveau_jour: nouveau_jour_legal(h),
            scenarios,
            theta_ii_fr_plus_c: self.scenarios.usage().consignes.fr_normal_c,
        };
        let exterieur = Exterieur {
            theta_ext_c: ext.te_c,
            omega_e_kg_kg: c.omega_e_kg_kg,
            v_ventc_m_s: c.v_vent_c_m_s,
        };

        // §5.5.3, étape 1 : bilan aéraulique de la zone, avec les températures et les humidités
        // des groupes à la fin du pas précédent. §6.1.3 : air soufflé par le système
        // conventionnel du Bbio, après l'échangeur.
        self.etats.clear();
        self.etats.extend(self.groupes.iter().map(|g| {
            EtatGroupe {
                theta_i_fin_c: g.etat.fin.theta_i_c,
                omega_i_fin_kg_kg: g.etat.omega_i_fin_kg_kg,
                q_spec_repris_m3_h: g.debits.q_rep_m3_h,
                q_spec_souffle_m3_h: g.debits.q_souffle_m3_h,
                t_air_souffle_c: t_air_souffle_c(ext.te_c, g.etat.fin.theta_i_c),
                omega_air_souffle_kg_kg: omega_air_souffle_kg_kg(c.omega_e_kg_kg),
                theta_an_c: g
                    .entree_air_espace_tampon
                    .and_then(|i| espaces_tampons.get(i))
                    .map(|e| e.theta_et_c),
            }
        }));
        let bilan = self
            .aeraulique
            .bilan(&exterieur, &self.etats)
            .ok_or_else(|| {
                Erreur::Calcul(format!(
                    "zone « {} », pas de temps {h} : le bilan aéraulique n'a pas de solution \
                     (Th-BCE 2020 §5.6.3.5, éq. 167)",
                    self.id
                ))
            })?;

        // §5.5.3, étape 2, figure 20 : d'abord les groupes qui cèdent de l'air au hall (ou qui
        // n'échangent rien avec lui), puis le hall fictif, enfin les groupes qui en reçoivent.
        // Si aucun groupe ne cède d'air, un débit reçu n'est qu'un résidu du solveur : le
        // groupe est traité à la première passe et ne reçoit rien du hall.
        let un_groupe_cede = bilan.groupes.iter().any(|b| b.q_m_hall_kg_s < 0.0);
        for passe_des_receveurs in [false, true] {
            // Éq. (130) : le hall fictif prend la température et l'humidité des débits que lui
            // cèdent les groupes d'entrée, calculées au pas de temps courant.
            let hall = if passe_des_receveurs {
                hall_fictif(&bilan, &self.etats)
            } else {
                None
            };
            for (g, groupe) in self.groupes.iter_mut().enumerate() {
                let b = &bilan.groupes[g];
                let receveur = un_groupe_cede && b.q_m_hall_kg_s > 0.0;
                if receveur != passe_des_receveurs {
                    continue;
                }
                let air = flux_entrants(b, &exterieur, &self.etats[g], hall);
                let apports = self.scenarios.apports_groupe(g, &scenarios);
                let sorties = groupe
                    .pas_avec_espaces_tampons(&c, &apports, &air, espaces_tampons)
                    .ok_or_else(|| {
                        Erreur::Calcul(format!(
                            "groupe « {} », pas de temps {h} : le modèle thermique n'a pas de \
                         solution (Th-BCE 2020 §5.21.3.6, éq. 339)",
                            groupe.id
                        ))
                    })?;
                self.sorties[g] = Some(sorties);
                // Le hall voit l'état des groupes d'entrée au pas de temps courant.
                self.etats[g].theta_i_fin_c = groupe.etat.fin.theta_i_c;
                self.etats[g].omega_i_fin_kg_kg = groupe.etat.omega_i_fin_kg_kg;
            }
        }
        Ok(())
    }

    /// Calcule les 8 760 pas de temps de l'année.
    pub fn simuler(
        &mut self,
        climat: &ClimatExterieur,
        confort: &ConfortAdaptatif,
    ) -> Result<(), Erreur> {
        self.simuler_avec(climat, confort, |_, _| {})
    }

    /// Calcule les 8 760 pas de temps de l'année et appelle `apres_le_pas(h, zone)` à la fin de
    /// chacun : les sorties horaires des groupes sont alors dans [`Self::sorties`]. C'est le
    /// point d'accès aux séries horaires.
    pub fn simuler_avec(
        &mut self,
        climat: &ClimatExterieur,
        confort: &ConfortAdaptatif,
        mut apres_le_pas: impl FnMut(usize, &Self),
    ) -> Result<(), Erreur> {
        for h in 0..NB_HEURES_AN {
            self.pas(h, climat, confort)?;
            apres_le_pas(h, self);
        }
        Ok(())
    }
}

/// Conditions extérieures du pas de temps `h` dans un mode de calcul.
pub fn conditions_exterieures(
    climat: &ClimatExterieur,
    confort: &ConfortAdaptatif,
    h: usize,
    mode: ModeCalcul,
) -> ConditionsExterieures {
    ConditionsExterieures {
        soleil: Soleil {
            gamma_rad: climat.gamma_rad[h],
            psi_rad: climat.psi_rad[h],
        },
        idn_w_m2: climat.idn_w_m2[h],
        idi_w_m2: climat.idi_w_m2[h],
        edn_lux: climat.edn_lux[h],
        edi_lux: climat.edi_lux[h],
        te_c: climat.te_c[h],
        teciel_c: climat.teciel_c[h],
        h,
        mode,
        is_conf_adapt: confort.is_conf_adapt(h),
    }
}
