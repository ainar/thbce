// SPDX-License-Identifier: GPL-3.0-or-later
//! Fiche `C_Calc_Bilans_Energies_Consommees_et_importées` (Th-BCE 2020 §13.4) : bilans horaires
//! de l'électricité consommée par poste, produite, autoconsommée, importée et exportée, au
//! niveau du bâtiment.
//!
//! Les consommations horaires d'électricité par poste sont relevées pendant le calcul Th-C
//! ([`ReleveElectrique`]), en différence des cumuls des groupes, des réseaux et des générations
//! d'un pas de temps à l'autre ; les postes calculés hors du modèle thermique (logement
//! collectif, parkings, usages mobiliers, ventilateurs des centrales) s'y ajoutent ensuite.
//! L'autoconsommation se calcule heure par heure (éq. 2527 à 2537).

use crate::bati::assemblage_batiment::BatimentHoraire;
use crate::chauffage_refroidissement::calculs_generation::{Energie, Poste};
use crate::climat::calendrier::date_scenario;
use crate::conventions::NB_HEURES_AN;

/// Postes de consommation électrique (« [poste] », §13.2.2) : chauffage, refroidissement, ECS,
/// éclairage, auxiliaires de ventilation, auxiliaires de distribution, déplacement des
/// occupants, usages mobiliers.
pub const NB_POSTES: usize = 8;

/// Indices des postes dans les vecteurs de [`NB_POSTES`] valeurs.
pub mod poste {
    /// Chauffage (« ch »).
    pub const CH: usize = 0;
    /// Refroidissement (« fr »).
    pub const FR: usize = 1;
    /// ECS (« ecs »).
    pub const ECS: usize = 2;
    /// Éclairage (« ecl »).
    pub const ECL: usize = 3;
    /// Auxiliaires de ventilation (« auxvent »).
    pub const AUX_VENT: usize = 4;
    /// Auxiliaires de distribution (« auxdist »).
    pub const AUX_DIST: usize = 5;
    /// Déplacement des occupants (« déplacement »).
    pub const DEPL: usize = 6;
    /// Usages mobiliers (« mobilier ») : hors du périmètre RE2020 (éq. 2514), mais compris dans
    /// la consommation « tous usages » qui fixe l'autoconsommation (éq. 2515 et 2527).
    pub const MOB: usize = 7;
}

/// Relevé horaire de l'électricité consommée par poste au cours du calcul Th-C, en Wh.
#[derive(Debug, Clone, PartialEq)]
pub struct ReleveElectrique {
    /// Consommation de chaque pas de temps, par poste.
    pub horaire: Vec<[f64; NB_POSTES]>,
    precedent: [f64; NB_POSTES],
}

impl Default for ReleveElectrique {
    fn default() -> Self {
        Self {
            horaire: vec![[0.0; NB_POSTES]; NB_HEURES_AN],
            precedent: [0.0; NB_POSTES],
        }
    }
}

/// Cumuls, depuis le début de l'année, de l'électricité consommée par les postes que suit le
/// modèle thermique, en Wh : lignes « électricité » des matrices des générations attribuées aux
/// groupes (éq. 2501 à 2503), éclairage des groupes (éq. 2510), ventilateurs locaux des
/// émetteurs (éq. 2504) et auxiliaires de distribution du groupe, intergroupes et d'ECS
/// intergroupe (éq. 2505).
fn cumuls(b: &BatimentHoraire) -> [f64; NB_POSTES] {
    let mut c = [0.0; NB_POSTES];
    for m in &b.systemes.cumuls_cef_groupes {
        let a = m.annuel();
        c[poste::CH] += a.valeur(Poste::Chauffage, Energie::Electricite);
        c[poste::FR] += a.valeur(Poste::Refroidissement, Energie::Electricite);
        c[poste::ECS] += a.valeur(Poste::Ecs, Energie::Electricite);
    }
    for g in b.zones.iter().flat_map(|z| z.groupes.iter()) {
        c[poste::ECL] += g.cumuls.c_ecl_kwh.iter().sum::<f64>() * 1000.0;
        if let Some(sy) = &g.systemes {
            c[poste::AUX_VENT] += sy.cumuls.w_vent_loc_wh;
            c[poste::AUX_DIST] += sy
                .cumuls
                .distributions
                .iter()
                .map(|d| d.w_aux_wh)
                .sum::<f64>();
        }
    }
    c[poste::AUX_DIST] += b.systemes.cumuls.iter().map(|k| k.w_aux_wh).sum::<f64>();
    c[poste::AUX_DIST] += b
        .systemes
        .cumuls_ecs
        .iter()
        .map(|k| k.w_aux_wh)
        .sum::<f64>();
    c
}

impl ReleveElectrique {
    /// Relève le pas de temps `h` qui vient d'être calculé.
    pub fn relever(&mut self, h: usize, b: &BatimentHoraire) {
        let c = cumuls(b);
        if let Some(x) = self.horaire.get_mut(h) {
            for ((x, c), p) in x.iter_mut().zip(c).zip(self.precedent) {
                *x += c - p;
            }
        }
        self.precedent = c;
    }

    /// Ajoute une série horaire (en Wh) au poste d'indice `p`.
    pub fn ajouter_serie(&mut self, p: usize, serie_wh: &[f64]) {
        for (x, w) in self.horaire.iter_mut().zip(serie_wh) {
            x[p] += w;
        }
    }

    /// Ajoute une consommation constante (en Wh par pas de temps) au poste d'indice `p`.
    pub fn ajouter_constante(&mut self, p: usize, w_wh: f64) {
        for x in &mut self.horaire {
            x[p] += w_wh;
        }
    }
}

/// Bilan annuel et mensuel de l'électricité du bâtiment, en Wh.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BilanElectrique {
    /// « Welec-cons[poste] » sommée sur l'année, par poste.
    pub cons_wh: [f64; NB_POSTES],
    /// « Welec-IMP[poste] » sommée sur l'année, par poste (éq. 2535).
    pub imp_wh: [f64; NB_POSTES],
    /// « Welec-IMP[poste] » sommée par mois des scénarios.
    pub imp_mensuel_wh: [[f64; NB_POSTES]; 12],
    /// « Eelec_prod_PV » de l'année (éq. 2523).
    pub prod_pv_wh: f64,
    /// « Eelec-prod-PV-AC » de l'année (éq. 2530).
    pub prod_pv_ac_wh: f64,
    /// Production, autoconsommation et exportation par mois.
    pub prod_pv_mensuel_wh: [f64; 12],
    /// Autoconsommation par mois.
    pub prod_pv_ac_mensuel_wh: [f64; 12],
}

impl BilanElectrique {
    /// Électricité exportée sur l'année (éq. 2469 et 2483).
    pub fn exportee_wh(&self) -> f64 {
        self.prod_pv_wh - self.prod_pv_ac_wh
    }

    /// « O_TAC_elec_PV_annuel », en % (éq. 2485) ; `None` sans production.
    pub fn tac_pct(&self) -> Option<f64> {
        (self.prod_pv_wh > 0.0).then(|| 100.0 * self.prod_pv_ac_wh / self.prod_pv_wh)
    }
}

/// Bilan horaire de l'électricité du bâtiment (§13.4.3.4 et §13.4.3.5).
///
/// `cons` : consommations horaires par poste, usages mobiliers compris ; `prod_pv_wh` :
/// production photovoltaïque attribuée au bâtiment, « Eelec_prod_PV(h) » (éq. 2523), vide sans
/// installation. Sans cogénération (hors de la première version), la production totale est la
/// production photovoltaïque (éq. 2526).
pub fn bilan(cons: &[[f64; NB_POSTES]], prod_pv_wh: &[f64]) -> BilanElectrique {
    let mut b = BilanElectrique::default();
    for (h, w) in cons.iter().enumerate() {
        let mois = usize::from(date_scenario(h).mois) - 1;
        let prod = prod_pv_wh.get(h).copied().unwrap_or(0.0).max(0.0);
        // Éq. (2519) : consommation tous usages, mobilier compris.
        let tous: f64 = w.iter().sum();
        // Éq. (2527) : énergie autoconsommée.
        let ac = prod.min(tous.max(0.0));
        // Éq. (2529) : taux d'autoproduction, nul sans consommation.
        let tap = if tous > 0.0 { ac / tous } else { 0.0 };
        // Éq. (2528) et (2530) : sans cogénération, TAC·Eprod_PV = Eprod_AC.
        b.prod_pv_wh += prod;
        b.prod_pv_ac_wh += ac;
        b.prod_pv_mensuel_wh[mois] += prod;
        b.prod_pv_ac_mensuel_wh[mois] += ac;
        for (p, &x) in w.iter().enumerate() {
            // Éq. (2535).
            let imp = x * (1.0 - tap);
            b.cons_wh[p] += x;
            b.imp_wh[p] += imp;
            b.imp_mensuel_wh[mois][p] += imp;
        }
    }
    b
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn autoconsommation_heure_par_heure() {
        let mut cons = vec![[0.0; NB_POSTES]; 3];
        // Heure 0 : 1 000 Wh de chauffage, 500 Wh d'usages mobiliers, aucune production.
        cons[0][poste::CH] = 1000.0;
        cons[0][poste::MOB] = 500.0;
        // Heure 1 : 300 Wh d'ECS et 300 Wh de mobilier, 900 Wh produits.
        cons[1][poste::ECS] = 300.0;
        cons[1][poste::MOB] = 300.0;
        // Heure 2 : 1 000 Wh d'éclairage, 250 Wh produits.
        cons[2][poste::ECL] = 1000.0;
        let b = bilan(&cons, &[0.0, 900.0, 250.0]);
        assert_eq!(b.prod_pv_wh, 1150.0);
        // 600 Wh autoconsommés à l'heure 1, 250 Wh à l'heure 2.
        assert_eq!(b.prod_pv_ac_wh, 850.0);
        assert_eq!(b.exportee_wh(), 300.0);
        assert_eq!(b.imp_wh[poste::CH], 1000.0);
        assert_eq!(b.imp_wh[poste::ECS], 0.0);
        assert_eq!(b.imp_wh[poste::MOB], 500.0);
        assert_eq!(b.imp_wh[poste::ECL], 750.0);
        assert_eq!(b.cons_wh[poste::ECL], 1000.0);
        assert!((b.tac_pct().unwrap_or(0.0) - 100.0 * 850.0 / 1150.0).abs() < 1e-12);
    }

    #[test]
    fn sans_production_tout_est_importe() {
        let cons = vec![[10.0; NB_POSTES]; 24];
        let b = bilan(&cons, &[]);
        assert_eq!(b.imp_wh, b.cons_wh);
        assert_eq!(b.tac_pct(), None);
    }
}
