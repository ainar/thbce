// SPDX-License-Identifier: GPL-3.0-or-later
//! Fiche `C_EIN_Scénarios conventionnels` (Th-BCE 2020 §4.1).
//!
//! À chaque pas de temps, la fiche combine, pour l'usage de la zone, une grille hebdomadaire
//! `p(j, h)` et une grille annuelle `p(m, s)` et en déduit les consignes, les indicateurs
//! d'occupation, de ventilation et d'éclairage, la clé de répartition de l'ECS et les apports
//! internes de chaleur et d'humidité des groupes.
//!
//! Périmètre : usages 1 (maison individuelle ou accolée) et 2 (logement collectif). Les autres
//! usages demandent le nombre d'occupants de l'éq. (38) et la lecture de valeurs fractionnaires
//! dans des indicateurs déclarés booléens (question Q27).
//!
//! Les locaux prennent les ratios de surface par défaut du classeur (`Rat_l`).

use thbce_data::scenarios::{Scenario, ScenariosUsage};

use crate::climat::calendrier::{date_scenario, DateScenario};
use crate::entree::UsageZone;
use crate::erreur::Erreur;

/// « αconv » : part des apports internes sous forme convective, valeur conventionnelle
/// (§4.1.2).
pub const ALPHA_CONV: f64 = 0.5;

/// Seuil de `Nmax` au-delà duquel les occupants supplémentaires comptent pour 0,3 (éq. 32, 36).
const SEUIL_NMAX: f64 = 1.75;

/// « Nadeq » : nombre d'adultes équivalents d'une zone d'habitation.
///
/// Th-BCE 2020 §4.1.3, éq. (32) en maison individuelle ou accolée et éq. (33) à (36) en
/// logement collectif. `a_lgt_m2` est la surface moyenne par logement : `Az / Nb_lgt` en maison,
/// `Rat_hab·Az / Nb_lgt` en logement collectif.
///
/// - Maison : `Nmax = 1` si `Algt < 30` ; `1,75 − 0,01875·(70 − Algt)` de 30 à 70 m² ;
///   `0,025·Algt` au-delà.
/// - Collectif : `Nmax = 1` si `Algt < 10` ; `1,75 − 0,01875·(50 − Algt)` de 10 à 50 m² ;
///   `0,035·Algt` au-delà.
/// - `Nadeq = Nb_lgt × Nmax` si `Nmax < 1,75`, sinon `Nb_lgt × (1,75 + 0,3·(Nmax − 1,75))`.
///
/// Renvoie `None` pour les usages autres que l'habitation.
pub fn nadeq(usage: UsageZone, a_lgt_m2: f64, nb_logements: u32) -> Option<f64> {
    let (seuil_bas, seuil_haut, pente) = match usage {
        UsageZone::MaisonIndividuelle => (30.0, 70.0, 0.025),
        UsageZone::LogementCollectif => (10.0, 50.0, 0.035),
        _ => return None,
    };
    let nmax = if a_lgt_m2 < seuil_bas {
        1.0
    } else if a_lgt_m2 <= seuil_haut {
        SEUIL_NMAX - 0.01875 * (seuil_haut - a_lgt_m2)
    } else {
        pente * a_lgt_m2
    };
    let par_logement = if nmax < SEUIL_NMAX {
        nmax
    } else {
        SEUIL_NMAX + 0.3 * (nmax - SEUIL_NMAX)
    };
    Some(f64::from(nb_logements) * par_logement)
}

/// Tableau 6 : produit « × », le ET des indicateurs booléens.
fn et(annuel: f64, hebdo: f64) -> f64 {
    annuel * hebdo
}

/// Tableau 5 : produit « ⊗ » des autorisations de chauffage et de refroidissement, à valeurs
/// dans {−1 ; 0 ; 1}. La table de vérité est celle du minimum des deux opérandes.
fn produit_consigne(annuel: f64, hebdo: f64) -> f64 {
    annuel.min(hebdo)
}

fn valeurs(s: Scenario, d: DateScenario) -> (f64, f64) {
    let annuel = s.grille_annuelle()[usize::from(d.semaine) - 1][usize::from(d.mois) - 1];
    let hebdo = s.grille_hebdo()[usize::from(d.jour) - 1][usize::from(d.heure) - 1];
    (annuel, hebdo)
}

/// Sorties de la fiche pour une zone, à un pas de temps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SortiesZone {
    /// Date du pas de temps au sens des scénarios.
    pub date: DateScenario,
    /// « θii_ch » : température de consigne de chauffage, en °C (éq. 30).
    pub theta_ii_ch_c: f64,
    /// « θii_fr » : température de consigne de refroidissement, en °C (éq. 30).
    pub theta_ii_fr_c: f64,
    /// « pch(m, s, j, h) » : 1 normal, 0 réduit de moins de 48 h, −1 réduit de plus de 48 h.
    pub p_ch: f64,
    /// « pfr(m, s, j, h) », mêmes codes.
    pub p_fr: f64,
    /// « pch_s(h) » : autorisation hebdomadaire de chauffage, pour les relances (éq. 55).
    pub pch_s: f64,
    /// « pfr_s(h) » : autorisation hebdomadaire de refroidissement (éq. 55).
    pub pfr_s: f64,
    /// « Uk » : indice de mobilité (éq. 31).
    pub uk: f64,
    /// « Ivent » : indicateur de ventilation en occupation (éq. 45).
    pub i_vent: f64,
    /// « ah » : clé de répartition horaire des besoins d'ECS hebdomadaires (éq. 46).
    pub ah: f64,
    /// « Ivac » : indicateur de vacances, grille annuelle d'occupation (éq. 48).
    pub i_vac: f64,
    /// « Iocc_zone » : indicateur d'occupation de la zone (éq. 49).
    pub i_occ_zone: f64,
    /// « Iocc_gpm » : indicateur d'occupation pour la gestion des protections mobiles (éq. 53).
    pub i_occ_gpm: f64,
    /// « Iecl » : mise en marche de l'éclairage (éq. 54).
    pub i_ecl: f64,
    /// Nombre d'occupants de la zone, tous locaux confondus (éq. 37).
    pub n_occ: f64,
    /// Apports internes de chaleur des occupants de la zone, en W (somme sur les locaux de
    /// `Nocc × Qmax_occ`, avant répartition entre groupes et entre convectif et radiatif).
    pub q_occ_w: f64,
    /// Apports internes d'humidité des occupants de la zone, en kg/h.
    pub w_occ_kg_h: f64,
    /// Apports internes de chaleur hors occupants et éclairage de la zone, en W (somme sur les
    /// locaux de `Al × Qmax_proc × tch`).
    pub q_int_w: f64,
    /// Apports internes d'humidité hors occupants de la zone, en kg/h.
    pub w_int_kg_h: f64,
}

/// Apports internes d'un groupe à un pas de temps (éq. 39, 40, 42 et 44).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApportsGroupe {
    /// « φint_occ,conv » : apports convectifs des occupants, en W.
    pub phi_int_occ_conv_w: f64,
    /// « φint_occ,rad » : apports radiatifs des occupants, en W.
    pub phi_int_occ_rad_w: f64,
    /// « ωint_occ » : apports d'humidité des occupants, en kg/h.
    pub omega_int_occ_kg_h: f64,
    /// « φint,conv » : apports convectifs hors occupants et éclairage, en W.
    pub phi_int_conv_w: f64,
    /// « φint,rad » : apports radiatifs hors occupants et éclairage, en W.
    pub phi_int_rad_w: f64,
    /// « ωint » : apports d'humidité hors occupants, en kg/h.
    pub omega_int_kg_h: f64,
}

/// Scénarios d'une zone, préparés une fois avant la boucle horaire.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenariosZone {
    usage: &'static ScenariosUsage,
    /// « Az » : surface utile de la zone, en m² (éq. 28).
    pub a_z_m2: f64,
    /// « Rat_gr » : part de chaque groupe dans la surface utile de la zone (éq. 27).
    pub rat_gr: Vec<f64>,
    /// « Nadeq » : nombre d'adultes équivalents de la zone.
    pub nadeq: f64,
    /// Grille hebdomadaire d'occupation pour la gestion des protections mobiles (éq. 51 et 52).
    p_occ_gpm_hebdo: [[f64; 24]; 7],
}

impl ScenariosZone {
    /// Prépare les scénarios d'une zone d'habitation.
    ///
    /// `a_gr_m2` : surface utile de chaque groupe (« Agr ») ; `nb_logements` : nombre de logements.
    pub fn nouveau(usage: UsageZone, a_gr_m2: &[f64], nb_logements: u32) -> Result<Self, Erreur> {
        let non_pris_en_charge = || Erreur::NonPrisEnCharge {
            chemin: "usage".to_owned(),
            element: format!(
                "les scénarios de l'usage n° {} (Th-BCE 2020 §4.1)",
                usage.numero()
            ),
        };
        if !usage.est_habitation() {
            return Err(non_pris_en_charge());
        }
        let scenarios =
            thbce_data::scenarios::usage(usage.numero()).ok_or_else(non_pris_en_charge)?;
        // Éq. (28) et (27).
        let a_z_m2: f64 = a_gr_m2.iter().sum();
        let rat_gr = a_gr_m2.iter().map(|a| a / a_z_m2).collect();
        // Éq. (32), ou éq. (33) et (34) : le local d'habitation est le premier de l'usage.
        let a_hab = scenarios.locaux[0].rat_l * a_z_m2;
        let nadeq = nadeq(usage, a_hab / f64::from(nb_logements), nb_logements)
            .ok_or_else(non_pris_en_charge)?;
        Ok(Self {
            usage: scenarios,
            a_z_m2,
            rat_gr,
            nadeq,
            p_occ_gpm_hebdo: occupation_gpm(scenarios.occupation.grille_hebdo()),
        })
    }

    /// Scénarios conventionnels de l'usage.
    pub fn usage(&self) -> &'static ScenariosUsage {
        self.usage
    }

    /// Sorties de la fiche au pas de temps `h`.
    pub fn evaluer(&self, h: usize) -> SortiesZone {
        let u = self.usage;
        let date = date_scenario(h);
        let produit = |s: Scenario| {
            let (annuel, hebdo) = valeurs(s, date);
            et(annuel, hebdo)
        };

        // Éq. (30) : consignes selon pch et pfr, combinés par l'algèbre du tableau 5.
        let (ch_a, ch_s) = valeurs(u.chauffage, date);
        let (fr_a, fr_s) = valeurs(u.refroidissement, date);
        let (p_ch, p_fr) = (produit_consigne(ch_a, ch_s), produit_consigne(fr_a, fr_s));
        let c = &u.consignes;
        let consigne = |p: f64, normal: f64, moins_48h: f64, plus_48h: f64| {
            if p > 0.0 {
                normal
            } else if p < 0.0 {
                plus_48h
            } else {
                moins_48h
            }
        };

        // Éq. (48) et (49).
        let (occ_a, occ_s) = valeurs(u.occupation, date);
        let i_occ_zone = et(occ_a, occ_s);
        // Éq. (50) et (53) : ⊗ du tableau 5 sur les indicateurs à valeurs −1, 0 et 1.
        let occ_gpm_a = if occ_a > 0.0 { 1.0 } else { -1.0 };
        let occ_gpm_s =
            self.p_occ_gpm_hebdo[usize::from(date.jour) - 1][usize::from(date.heure) - 1];
        let i_occ_gpm = produit_consigne(occ_gpm_a, occ_gpm_s);

        // Éq. (37) : occupants du local d'habitation ; les autres locaux n'en ont pas.
        let hab = &u.locaux[0];
        let n_occ = self.nadeq * i_occ_zone * produit(hab.occupants.scenario);
        // Éq. (41) et (43), sommées sur les locaux : Al = Az·Rat_l (éq. 29).
        let (mut q_int_w, mut w_int_kg_h) = (0.0, 0.0);
        for l in u.locaux {
            let a_l = self.a_z_m2 * l.rat_l;
            q_int_w += a_l * l.chaleur.valeur_par_unite * produit(l.chaleur.scenario);
            w_int_kg_h += a_l * l.humidite.valeur_par_unite * produit(l.humidite.scenario);
        }

        SortiesZone {
            date,
            theta_ii_ch_c: consigne(
                p_ch,
                c.ch_normal_c,
                c.ch_arret_moins_48h_c,
                c.ch_arret_plus_48h_c,
            ),
            theta_ii_fr_c: consigne(
                p_fr,
                c.fr_normal_c,
                c.fr_arret_moins_48h_c,
                c.fr_arret_plus_48h_c,
            ),
            p_ch,
            p_fr,
            pch_s: ch_s,
            pfr_s: fr_s,
            uk: produit(u.mobilite),
            i_vent: produit(u.ventilation),
            ah: produit(u.ecs.scenario),
            i_vac: occ_a,
            i_occ_zone,
            i_occ_gpm,
            i_ecl: produit(u.eclairage),
            n_occ,
            q_occ_w: n_occ * hab.occupants.chaleur_w,
            w_occ_kg_h: n_occ * hab.occupants.humidite_kg_h,
            q_int_w,
            w_int_kg_h,
        }
    }

    /// Apports internes du groupe d'indice `g`.
    ///
    /// Th-BCE 2020 §4.1.3 : éq. (39) `φint_occ,conv = Rat_gr·αconv·Σ Nocc·Qmax_occ` et son
    /// complément radiatif, éq. (40) `ωint_occ = Rat_gr·Σ Nocc·ωmax_occ`, éq. (42) et (44) pour
    /// les apports hors occupants.
    pub fn apports_groupe(&self, g: usize, sorties: &SortiesZone) -> ApportsGroupe {
        let rat = self.rat_gr.get(g).copied().unwrap_or(0.0);
        ApportsGroupe {
            phi_int_occ_conv_w: rat * ALPHA_CONV * sorties.q_occ_w,
            phi_int_occ_rad_w: rat * (1.0 - ALPHA_CONV) * sorties.q_occ_w,
            omega_int_occ_kg_h: rat * sorties.w_occ_kg_h,
            phi_int_conv_w: rat * ALPHA_CONV * sorties.q_int_w,
            phi_int_rad_w: rat * (1.0 - ALPHA_CONV) * sorties.q_int_w,
            omega_int_kg_h: rat * sorties.w_int_kg_h,
        }
    }
}

/// Grille hebdomadaire d'occupation pour la gestion des protections mobiles.
///
/// Th-BCE 2020 §4.1.3, éq. (51) : `pocc_gpm(j, h) = 1` si la zone est occupée, `−1` sinon.
/// Éq. (52) : pour chaque jour, une absence qui commence à une heure `hdeb` de 7 à 22 (la zone
/// étant occupée à `hdeb − 1`) et qui se termine à une heure `hfin` au plus égale à 22 (la zone
/// étant occupée à `hfin + 1`) reçoit la valeur de `pocc`, c'est-à-dire 0 : c'est une absence
/// courte en journée, par opposition à la nuit ou à une journée entière d'absence.
fn occupation_gpm(p_occ: &[[f64; 24]; 7]) -> [[f64; 24]; 7] {
    let mut gpm = [[0.0; 24]; 7];
    for (j, jour) in p_occ.iter().enumerate() {
        // Les heures du texte vont de 1 à 24 : l'heure h est à l'indice h − 1.
        let occ = |h: usize| jour[h - 1];
        for h in 1..=24 {
            gpm[j][h - 1] = if occ(h) > 0.0 { 1.0 } else { -1.0 };
        }
        for hdeb in 7..=22 {
            if occ(hdeb) > 0.0 || occ(hdeb - 1) <= 0.0 {
                continue;
            }
            for hfin in hdeb..=22 {
                if occ(hfin) <= 0.0 && occ(hfin + 1) > 0.0 {
                    for h in hdeb..=hfin {
                        gpm[j][h - 1] = occ(h);
                    }
                }
            }
        }
    }
    gpm
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const MI: UsageZone = UsageZone::MaisonIndividuelle;
    const LC: UsageZone = UsageZone::LogementCollectif;

    fn proche(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12 * (1.0 + b.abs())
    }

    #[test]
    fn adultes_equivalents_en_maison() {
        // Éq. (32). Algt < 30 : Nmax = 1.
        assert_eq!(nadeq(MI, 20.0, 1), Some(1.0));
        // Algt = 30 : 1,75 − 0,01875 × 40 = 1 ; Algt = 50 : 1,75 − 0,375 = 1,375.
        assert!(proche(nadeq(MI, 30.0, 1).unwrap(), 1.0));
        assert!(proche(nadeq(MI, 50.0, 1).unwrap(), 1.375));
        // Algt = 70 : les deux branches donnent Nmax = 1,75, et Nadeq = 1,75.
        assert!(proche(nadeq(MI, 70.0, 1).unwrap(), 1.75));
        // Algt = 90 : Nmax = 2,25 ; Nadeq = 1,75 + 0,3 × 0,5 = 1,9.
        assert!(proche(nadeq(MI, 90.0, 1).unwrap(), 1.9));
        // Deux maisons accolées de 90 m² : le double.
        assert!(proche(nadeq(MI, 90.0, 2).unwrap(), 3.8));
    }

    #[test]
    fn adultes_equivalents_en_collectif() {
        // Éq. (35) et (36). Algt < 10 : 1 ; Algt = 30 : 1,75 − 0,375 = 1,375 ; Algt = 50 : 1,75.
        assert_eq!(nadeq(LC, 9.0, 1), Some(1.0));
        assert!(proche(nadeq(LC, 30.0, 1).unwrap(), 1.375));
        assert!(proche(nadeq(LC, 50.0, 1).unwrap(), 1.75));
        // Algt = 60 : Nmax = 2,1 ; Nadeq par logement = 1,75 + 0,3 × 0,35 = 1,855 ; 10 logements.
        assert!(proche(nadeq(LC, 60.0, 10).unwrap(), 18.55));
        assert_eq!(nadeq(UsageZone::Bureaux, 60.0, 1), None);
    }

    #[test]
    fn tables_de_verite() {
        // Tableau 5, lignes pa = −1, 0, 1 et colonnes ps = −1, 0, 1.
        let attendu = [[-1.0, -1.0, -1.0], [-1.0, 0.0, 0.0], [-1.0, 0.0, 1.0]];
        for (i, a) in [-1.0, 0.0, 1.0].into_iter().enumerate() {
            for (j, s) in [-1.0, 0.0, 1.0].into_iter().enumerate() {
                assert_eq!(produit_consigne(a, s), attendu[i][j], "pa = {a}, ps = {s}");
            }
        }
        // Tableau 6.
        assert_eq!(
            (et(0.0, 0.0), et(0.0, 1.0), et(1.0, 0.0), et(1.0, 1.0)),
            (0.0, 0.0, 0.0, 1.0)
        );
    }

    fn maison() -> ScenariosZone {
        // Un groupe de 90 m² utiles, un logement : Nadeq = 1,9.
        ScenariosZone::nouveau(MI, &[90.0], 1).unwrap()
    }

    /// Itération du lundi de la première semaine dont la case de scénario vaut `case`.
    fn lundi(case: usize) -> usize {
        case - 2
    }

    #[test]
    fn maison_un_lundi_d_hiver() {
        let z = maison();
        assert!(proche(z.nadeq, 1.9));
        assert_eq!(z.rat_gr, [1.0]);

        // 20 h–21 h légales (case 21) : occupée, consignes normales, éclairage possible.
        let soir = z.evaluer(lundi(21));
        assert_eq!((soir.date.jour, soir.date.heure), (1, 21));
        assert_eq!((soir.theta_ii_ch_c, soir.theta_ii_fr_c), (19.0, 26.0));
        assert_eq!(
            (soir.i_occ_zone, soir.i_ecl, soir.i_vent, soir.i_vac),
            (1.0, 1.0, 1.0, 1.0)
        );
        // Taux d'occupation 1 : Nocc = Nadeq ; 90 W par adulte équivalent.
        assert!(proche(soir.n_occ, 1.9));
        assert!(proche(soir.q_occ_w, 171.0));
        assert!(proche(soir.w_occ_kg_h, 1.9 * 0.055));
        // Équipements : 90 m² × 5,7 W/m² × 1 = 513 W.
        assert!(proche(soir.q_int_w, 513.0));
        assert_eq!(soir.w_int_kg_h, 0.0);

        // 12 h–13 h légales (case 13) : inoccupée, réduit de moins de 48 h, pas d'éclairage.
        let midi = z.evaluer(lundi(13));
        assert_eq!((midi.theta_ii_ch_c, midi.theta_ii_fr_c), (16.0, 30.0));
        assert_eq!((midi.p_ch, midi.i_occ_zone, midi.i_ecl), (0.0, 0.0, 0.0));
        assert_eq!(midi.n_occ, 0.0);
        // Équipements hors occupation : 513 × 0,2 = 102,6 W ; la ventilation reste en marche.
        assert!(proche(midi.q_int_w, 102.6));
        assert_eq!(midi.i_vent, 1.0);

        // 2 h–3 h légales (case 3) : sommeil, taux d'occupation 0,7.
        let nuit = z.evaluer(lundi(3));
        assert!(proche(nuit.n_occ, 1.9 * 0.7));
        assert_eq!(nuit.i_ecl, 0.0);
        assert!(proche(nuit.q_int_w, 102.6));
    }

    #[test]
    fn apports_repartis_entre_groupes_et_entre_convectif_et_radiatif() {
        // Deux groupes de 60 et 30 m² : mêmes totaux que la maison de 90 m², répartis 2/3 – 1/3.
        let z = ScenariosZone::nouveau(MI, &[60.0, 30.0], 1).unwrap();
        assert!(proche(z.a_z_m2, 90.0));
        let s = z.evaluer(lundi(21));
        let (g0, g1) = (z.apports_groupe(0, &s), z.apports_groupe(1, &s));
        // Occupants : 171 W, moitié convectif, moitié radiatif.
        assert!(proche(g0.phi_int_occ_conv_w, 171.0 * 0.5 * 2.0 / 3.0));
        assert!(proche(g0.phi_int_occ_rad_w, g0.phi_int_occ_conv_w));
        assert!(proche(g1.phi_int_conv_w, 513.0 * 0.5 / 3.0));
        // Conservation : la somme sur les groupes redonne les apports de la zone.
        let total = |f: fn(&ApportsGroupe) -> f64| f(&g0) + f(&g1);
        assert!(proche(
            total(|a| a.phi_int_occ_conv_w + a.phi_int_occ_rad_w),
            s.q_occ_w
        ));
        assert!(proche(
            total(|a| a.phi_int_conv_w + a.phi_int_rad_w),
            s.q_int_w
        ));
        assert!(proche(total(|a| a.omega_int_occ_kg_h), s.w_occ_kg_h));
    }

    #[test]
    fn semaine_d_absence_de_decembre() {
        let z = maison();
        // Cherche un pas de temps de vacances : grille annuelle d'occupation à 0.
        let vacances: Vec<usize> = (0..8760).filter(|h| z.evaluer(*h).i_vac == 0.0).collect();
        // La semaine d'absence est la dernière de décembre : 168 heures, auxquelles s'ajoutent les
        // 25 heures du lundi rattaché à cette même semaine par la décision Q17.
        assert_eq!(vacances.len(), 168 + 25);
        assert!(vacances.iter().all(|h| z.evaluer(*h).date.mois == 12));
        let s = z.evaluer(vacances[100]);
        // Zone inoccupée, consignes réduites de plus de 48 h, aucun occupant.
        assert_eq!((s.i_occ_zone, s.n_occ, s.p_ch), (0.0, 0.0, -1.0));
        assert_eq!((s.theta_ii_ch_c, s.theta_ii_fr_c), (16.0, 30.0));
        assert_eq!(s.i_occ_gpm, -1.0);
    }

    #[test]
    fn collectif() {
        // 1 000 m² utiles, 15 logements : Ahab = 0,9 × 1 000 = 900 m², Algt = 60 m²,
        // Nadeq = 15 × 1,855 = 27,825.
        let z = ScenariosZone::nouveau(LC, &[1000.0], 15).unwrap();
        assert!(proche(z.nadeq, 27.825));
        let s = z.evaluer(lundi(21));
        assert!(proche(s.n_occ, 27.825));
        // Équipements : logements 900 m² × 5,7 ; circulations 100 m² × 0.
        assert!(proche(s.q_int_w, 5130.0));
    }

    #[test]
    fn occupation_pour_les_protections_mobiles() {
        // Lundi de maison individuelle : occupée aux heures 1 à 9 et 18 à 24, absente de 10 à 17.
        // L'absence commence à 10 (occupée à 9) et finit à 17 (occupée à 18) : c'est une absence
        // courte de journée, valeur 0 ; aucune heure ne vaut −1.
        let z = maison();
        let lundi_gpm = z.p_occ_gpm_hebdo[0];
        for h in 1..=24 {
            let attendu = if (10..=17).contains(&h) { 0.0 } else { 1.0 };
            assert_eq!(lundi_gpm[h - 1], attendu, "heure {h}");
        }
        // Samedi, occupé toute la journée : 1 partout.
        assert!(z.p_occ_gpm_hebdo[5].iter().all(|v| *v == 1.0));
        // Grille artificielle : absence de nuit (heures 1 à 6 et 23, 24) et de midi (12 à 13).
        let mut jour = [1.0; 24];
        for h in [1, 2, 3, 4, 5, 6, 12, 13, 23, 24] {
            jour[h - 1] = 0.0;
        }
        let gpm = occupation_gpm(&[jour; 7])[0];
        assert_eq!((gpm[0], gpm[5]), (-1.0, -1.0), "nuit");
        assert_eq!((gpm[11], gpm[12]), (0.0, 0.0), "midi");
        assert_eq!(
            (gpm[22], gpm[23]),
            (-1.0, -1.0),
            "soir : l'absence ne finit pas avant 22 h"
        );
        assert_eq!(gpm[9], 1.0);
    }

    #[test]
    fn les_usages_hors_perimetre_sont_refuses() {
        assert!(matches!(
            ScenariosZone::nouveau(UsageZone::Bureaux, &[100.0], 1),
            Err(Erreur::NonPrisEnCharge { .. })
        ));
    }

    #[test]
    fn invariants_sur_l_annee() {
        let z = maison();
        let mut heures_occupees = 0.0;
        for h in 0..8760 {
            let s = z.evaluer(h);
            assert!(s.n_occ >= 0.0 && s.n_occ <= z.nadeq + 1e-12);
            assert!(s.q_int_w >= 0.0 && s.q_int_w <= 513.0 + 1e-9);
            assert!((0.0..=1.0).contains(&s.ah));
            assert!(
                [16.0, 19.0].contains(&s.theta_ii_ch_c) && [26.0, 30.0].contains(&s.theta_ii_fr_c)
            );
            // Aucun occupant quand la zone est inoccupée.
            if s.i_occ_zone == 0.0 {
                assert_eq!(s.n_occ, 0.0);
            }
            heures_occupees += s.i_occ_zone;
        }
        // Par semaine : 4 jours à 16 h, le mercredi à 20 h, le week-end à 48 h, soit 132 h.
        // 51 semaines occupées (la dernière de décembre et le lundi ajouté sont en absence) :
        // 51 × 132 = 6 732 h, à une heure près aux changements d'heure.
        assert!(
            (6730.0..=6734.0).contains(&heures_occupees),
            "{heures_occupees}"
        );
    }
}
