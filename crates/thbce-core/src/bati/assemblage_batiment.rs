// SPDX-License-Identifier: GPL-3.0-or-later
//! Enchaînement horaire d'un bâtiment : espaces tampons puis zones.
//!
//! Un espace tampon est défini au niveau du bâtiment et peut toucher des groupes de plusieurs
//! zones (§5.4.1). À chaque pas de temps, dans l'ordre de la figure 15 (§5.2.2) : les espaces
//! tampons sont calculés à partir de l'état des groupes à la fin du pas précédent (`θi,fin(h−1)`,
//! `θop,fin(h−1)`, saisons) et de leur occupation au pas courant ; leurs coefficients
//! `btherm(h)`, `bsolaire(h)`, `becl(h)` et leur température `θet(h)` sont ensuite transmis aux
//! composants d'enveloppe et aux entrées d'air des zones (§5.5, §5.7), calculées comme avant.

use crate::climat::environnement_proche::ventc_m_s;
use crate::climat::{ClimatExterieur, ConfortAdaptatif};
use crate::conventions::{ModeCalcul, NB_HEURES_AN};
use crate::entree::{Batiment, Catalogue};
use crate::erreur::Erreur;

use super::assemblage_groupe::Lectures;
use super::assemblage_zone::{conditions_exterieures, ZoneHoraire};
use super::espace_tampon::assemblage::{EspaceTamponHoraire, GroupeVuParEt, LienGroupe};
use super::espace_tampon::espace_tampon::CoefficientsEt;

/// Bâtiment prêt pour un calcul annuel au pas horaire, en mode Th-B ou Th-D.
#[derive(Debug, Clone, PartialEq)]
pub struct BatimentHoraire {
    /// Identifiant du bâtiment.
    pub id: String,
    /// Mode de calcul.
    pub mode: ModeCalcul,
    /// Zones du bâtiment.
    pub zones: Vec<ZoneHoraire>,
    /// Espaces tampons du bâtiment, dans l'ordre de la saisie.
    pub espaces_tampons: Vec<EspaceTamponHoraire>,
    /// Coefficients des espaces tampons au dernier pas de temps calculé.
    pub coefficients: Vec<CoefficientsEt>,
    vus: Vec<GroupeVuParEt>,
}

impl BatimentHoraire {
    /// Prépare un bâtiment validé, avec les lectures retenues du texte.
    pub fn nouveau(
        bat: &Batiment,
        catalogue: &Catalogue,
        mode: ModeCalcul,
        alt_m: f64,
        omega_initiale_kg_kg: f64,
    ) -> Result<Self, Erreur> {
        Self::nouveau_selon(
            bat,
            catalogue,
            mode,
            Lectures::default(),
            alt_m,
            omega_initiale_kg_kg,
        )
    }

    /// Prépare un bâtiment validé pour des lectures données du texte.
    pub fn nouveau_selon(
        bat: &Batiment,
        catalogue: &Catalogue,
        mode: ModeCalcul,
        lectures: Lectures,
        alt_m: f64,
        omega_initiale_kg_kg: f64,
    ) -> Result<Self, Erreur> {
        let ids: Vec<String> = bat.espaces_tampons.iter().map(|e| e.id.clone()).collect();
        let mut zones = bat
            .zones
            .iter()
            .map(|z| {
                ZoneHoraire::nouvelle_dans_batiment(
                    z,
                    catalogue,
                    mode,
                    lectures,
                    alt_m,
                    omega_initiale_kg_kg,
                    &ids,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        // §5.4.3.1, éq. (106) et (107) : coefficients des groupes vers chaque espace tampon,
        // calculés en début de simulation.
        let mut totaux_ouverture = vec![0.0; ids.len()];
        let espaces_tampons = bat
            .espaces_tampons
            .iter()
            .enumerate()
            .map(|(i, et)| {
                let mut liens = Vec::new();
                for (iz, z) in zones.iter().enumerate() {
                    for (ig, g) in z.groupes.iter().enumerate() {
                        let h = g.enveloppe.h_et_th_int_w_k(i);
                        let touche = g
                            .enveloppe
                            .parois
                            .iter()
                            .any(|p| p.espace_tampon == Some(i))
                            || g.enveloppe.ponts.iter().any(|p| p.espace_tampon == Some(i))
                            || g.enveloppe.baies.iter().any(|b| b.espace_tampon == Some(i));
                        if touche {
                            liens.push(LienGroupe {
                                zone: iz,
                                groupe: ig,
                                h_et_th_int_w_k: h,
                            });
                        }
                        totaux_ouverture[i] += g
                            .enveloppe
                            .baies
                            .iter()
                            .filter(|b| b.espace_tampon == Some(i))
                            .map(|b| b.a_ouv_max_m2)
                            .sum::<f64>();
                    }
                }
                EspaceTamponHoraire::nouveau(et, catalogue, liens, lectures.fermeture_et)
            })
            .collect::<Vec<_>>();
        for z in &mut zones {
            for g in &mut z.groupes {
                g.fixer_parts_ouverture_espaces_tampons(&totaux_ouverture);
            }
        }
        let coefficients = espaces_tampons.iter().map(|e| e.coefficients).collect();
        Ok(Self {
            id: bat.id.clone(),
            mode,
            zones,
            espaces_tampons,
            coefficients,
            vus: Vec::new(),
        })
    }

    /// Calcule le pas de temps `h` du bâtiment : espaces tampons, puis zones.
    pub fn pas(
        &mut self,
        h: usize,
        climat: &ClimatExterieur,
        confort: &ConfortAdaptatif,
    ) -> Result<(), Erreur> {
        if !self.espaces_tampons.is_empty() {
            let ext = conditions_exterieures(climat, confort, h, self.mode);
            let v_ventc = ventc_m_s(climat.vent_m_s[h]);
            let i_occ: Vec<f64> = self.zones.iter().map(|z| z.i_occ_zone(h)).collect();
            for (i, et) in self.espaces_tampons.iter_mut().enumerate() {
                self.vus.clear();
                self.vus.extend(et.liens.iter().map(|l| {
                    let g = &self.zones[l.zone].groupes[l.groupe];
                    GroupeVuParEt {
                        theta_i_fin_c: g.etat.fin.theta_i_c,
                        theta_op_fin_c: g.etat.fin.theta_op_c,
                        i_occ: i_occ[l.zone],
                        aut_ch: g.etat.saisons.aut_ch,
                        aut_fr: g.etat.saisons.aut_fr,
                        hebergement: g.habitation,
                    }
                }));
                self.coefficients[i] = et.pas(&ext, v_ventc, &self.vus);
            }
        }
        for z in &mut self.zones {
            z.pas_avec_espaces_tampons(h, climat, confort, &self.coefficients)?;
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

    /// Calcule les 8 760 pas de temps de l'année et appelle `apres_le_pas(h, batiment)` à la
    /// fin de chacun.
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
