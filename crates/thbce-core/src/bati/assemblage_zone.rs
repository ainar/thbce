// SPDX-License-Identifier: GPL-3.0-or-later
//! Assemblage des composants d'une zone pour le calcul du Bbio : fiche
//! `S2_BAT-assemblage_zone` (Th-BCE 2020 §5.5, figure 18).
//!
//! À chaque pas de temps : climat et scénarios de la zone ; bilan aéraulique de la zone (§5.6)
//! avec les débits du système de ventilation conventionnel (§6.1 et §6.2) et les températures
//! des groupes au pas précédent ; puis comportement thermique de chaque groupe (§5.7), dans
//! l'ordre de la figure 20 : les groupes qui cèdent de l'air au hall, le hall, puis les groupes
//! qui en reçoivent.
//!
//! Périmètre : calcul Th-B des usages d'habitation ; hall fictif (aucun groupe de circulation
//! saisi) ; pas d'espace tampon.

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

use super::assemblage_groupe::{nbh_occ_ref, ConditionsPas, GroupeBbio, SortiesPasGroupe};
use super::ConditionsExterieures;

/// Zone d'habitation prête pour le calcul du Bbio.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneBbio {
    /// Identifiant de la zone.
    pub id: String,
    /// Scénarios conventionnels de la zone (§4.1).
    pub scenarios: ScenariosZone,
    /// Modèle aéraulique de la zone (§5.6).
    pub aeraulique: ZoneAeraulique,
    /// Groupes de la zone.
    pub groupes: Vec<GroupeBbio>,
    /// Sorties de chaque groupe au dernier pas de temps calculé.
    pub sorties: Vec<Option<SortiesPasGroupe>>,
    etats: Vec<EtatGroupe>,
}

impl ZoneBbio {
    /// Prépare une zone d'habitation validée.
    ///
    /// `alt_m` est l'altitude du projet ; `omega_initiale_kg_kg`, l'humidité de l'air des
    /// groupes avant le premier pas de temps (celle de l'air extérieur au premier pas de temps,
    /// décision Q39 point 4).
    pub fn nouvelle(
        zone: &Zone,
        catalogue: &Catalogue,
        alt_m: f64,
        omega_initiale_kg_kg: f64,
    ) -> Result<Self, Erreur> {
        let a_gr: Vec<f64> = zone
            .groupes
            .iter()
            .map(|g| g.su_m2.unwrap_or(0.0))
            .collect();
        let scenarios = ScenariosZone::nouveau(zone.usage, &a_gr, zone.nb_lgt.unwrap_or(1))?;
        let nbh_occ_ref = nbh_occ_ref(scenarios.usage().occupation.grille_hebdo());
        let q4pa = zone.q4pa_surf_m3_h_m2.unwrap_or(0.0);
        let groupes: Vec<GroupeBbio> = zone
            .groupes
            .iter()
            .map(|g| {
                GroupeBbio::nouveau(
                    g,
                    catalogue,
                    zone.usage,
                    q4pa,
                    nbh_occ_ref,
                    omega_initiale_kg_kg,
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
            scenarios,
            aeraulique,
            groupes,
            sorties: vec![None; nb],
            etats: Vec::with_capacity(nb),
        })
    }

    /// Calcule le pas de temps `h` de la zone en mode Th-B.
    ///
    /// `climat` est le climat du site pour le jeu météo « Th-BC » et `confort` la moyenne
    /// glissante de la température extérieure qui s'en déduit.
    pub fn pas(
        &mut self,
        h: usize,
        climat: &ClimatExterieur,
        confort: &ConfortAdaptatif,
    ) -> Result<(), Erreur> {
        let ext = ConditionsExterieures {
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
            mode: ModeCalcul::ThB,
            is_conf_adapt: confort.is_conf_adapt(h),
        };
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
        self.etats.extend(self.groupes.iter().map(|g| EtatGroupe {
            theta_i_fin_c: g.etat.fin.theta_i_c,
            omega_i_fin_kg_kg: g.etat.omega_i_fin_kg_kg,
            q_spec_repris_m3_h: g.debits.q_rep_m3_h,
            q_spec_souffle_m3_h: g.debits.q_souffle_m3_h,
            t_air_souffle_c: t_air_souffle_c(ext.te_c, g.etat.fin.theta_i_c),
            omega_air_souffle_kg_kg: omega_air_souffle_kg_kg(c.omega_e_kg_kg),
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
                let sorties = groupe.pas(&c, &apports, &air).ok_or_else(|| {
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
        for h in 0..NB_HEURES_AN {
            self.pas(h, climat, confort)?;
        }
        Ok(())
    }
}
