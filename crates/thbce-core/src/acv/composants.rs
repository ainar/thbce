// SPDX-License-Identifier: GPL-3.0-or-later
//! Contribution d'un composant aux impacts sur le changement climatique (annexe II §4.2.1) :
//! facteur de renouvellement, méthode statique (éq. 6 à 14), méthode dynamique (éq. 35 à 44)
//! et fluides frigorigènes (éq. 26, 45 à 62, 65 à 71). Les fonctions renvoient des impacts
//! absolus (kg éq. CO2), par phase : `[production, édification, exploitation, fin de vie,
//! module D]`.

use thbce_data::ponderation_dynamique::{F_CO2, F_FLUIDES_FRIGO};

use crate::entree::DonneeEnvironnementale;

/// « PER » : période d'étude de référence, en années (annexe II §1.1).
pub const PER_ANS: f64 = 50.0;
/// « TProduction » : taux de fuite lié à la production (§4.2.1.2.2).
pub const T_PRODUCTION: f64 = 0.02;
/// « TExploitation » : taux de fuite annuel en exploitation (§4.2.1.1.6).
pub const T_EXPLOITATION: f64 = 0.02;
/// « TFin de vie » : taux de perte en fin de vie (§4.2.1.2.2).
pub const T_FIN_DE_VIE: f64 = 0.10;
/// « DVEp » par défaut d'un équipement d'un lot forfaitaire (§4.2.2.2.2).
pub const DVE_LOT_FORFAITAIRE_ANS: f64 = 17.0;

/// Impacts par phase : production, édification, exploitation, fin de vie, module D.
pub type Phases = [f64; 5];

/// Indice de l'année d'une date d'émission `t` (en années) dans les tableaux de l'article 11 :
/// partie entière, bornée à 0..=50. La fonction n'est définie que pour un `x` entier (annexe II
/// §4.2.1.2) ; une date non entière (DVE non entière) est ramenée à l'année entamée
/// (`docs/questions/Q206.md`).
fn annee(t: f64) -> usize {
    // Q206 : partie entière (variante non exposée : interpolation linéaire).
    if t <= 0.0 {
        0
    } else {
        (libm::floor(t) as usize).min(F_CO2.len() - 1)
    }
}

/// `fCO2(t)`.
pub fn f_co2(t: f64) -> f64 {
    F_CO2[annee(t)]
}

/// `fff(t)`.
pub fn f_ff(t: f64) -> f64 {
    F_FLUIDES_FRIGO[annee(t)]
}

/// `Σ_{a=1}^{PER} f(a)` pour la table `f`.
fn somme_annuelle(f: fn(f64) -> f64) -> f64 {
    (1..=50).map(|a| f(f64::from(a))).sum()
}

/// `Σ_{a=1}^{PER} fCO2(a)` (39,543, éq. 35, 40, 74, 87).
pub fn somme_f_co2() -> f64 {
    somme_annuelle(f_co2)
}

/// `Σ_{a=1}^{PER} fff(a)` (éq. 45, 65).
pub fn somme_f_ff() -> f64 {
    somme_annuelle(f_ff)
}

/// « Rp » : facteur de renouvellement (annexe II §4.2.1.1.1, éq. 6).
pub fn facteur_renouvellement(dve_ans: f64) -> f64 {
    (PER_ANS / dve_ans).max(1.0)
}

/// « α » et « FUtil » (éq. 38 et 39).
fn alpha_futil(dve_ans: f64) -> (f64, f64) {
    let x = PER_ANS / dve_ans;
    let alpha = libm::floor(x);
    (alpha, x - alpha)
}

/// `Σ_{r=1}^{α−1} f(DVE·r) + FUtil·f(DVE·α)` : pondération des renouvellements (éq. 41, 44,
/// 47, 55, 67).
fn somme_renouvellements(dve_ans: f64, f: impl Fn(f64) -> f64) -> f64 {
    // DVE ≥ PER : aucun renouvellement (α = 0 donnerait sinon `FUtil × f(0)`, un renouvellement
    // fictif ; `docs/questions/Q217.md`).
    if dve_ans >= PER_ANS {
        return 0.0;
    }
    let (alpha, futil) = alpha_futil(dve_ans);
    let n = alpha as u32;
    let mut s = 0.0;
    for r in 1..n {
        s += f(dve_ans * f64::from(r));
    }
    s + futil * f(dve_ans * alpha)
}

/// « DE Exploitation » retenu : nul si les modules B1 à B4 ne sont pas séparés de B6 et B7
/// (§4.2.1.1.2).
fn de_exploitation(de: &DonneeEnvironnementale) -> f64 {
    if de.exploitation_non_separee {
        0.0
    } else {
        de.exploitation_kgco2
    }
}

/// Contribution statique d'un composant (annexe II §4.2.1.1.3, éq. 9 à 14).
pub fn composant_statique(q: f64, de: &DonneeEnvironnementale, dve_ans: f64) -> Phases {
    let rp = facteur_renouvellement(dve_ans);
    let prod = de.production_kgco2;
    let edif = de.edification_kgco2;
    let fdv = de.fin_de_vie_kgco2;
    // Éq. (11) et (12) : la seconde est la première avec Rp = 1.
    let expl = PER_ANS / dve_ans * de_exploitation(de) + (rp - 1.0) * (prod + edif + fdv);
    [
        q * prod,                   // éq. (9)
        q * edif,                   // éq. (10)
        q * expl,                   // éq. (11), (12)
        q * fdv,                    // éq. (13)
        q * rp * de.module_d_kgco2, // éq. (14)
    ]
}

/// Contribution dynamique d'un composant (annexe II §4.2.1.2.1, éq. 35 à 44).
pub fn composant_dynamique(q: f64, de: &DonneeEnvironnementale, dve_ans: f64) -> Phases {
    let prod = de.production_kgco2;
    let edif = de.edification_kgco2;
    let fdv = de.fin_de_vie_kgco2;
    let f_per = f_co2(PER_ANS);
    // Éq. (35) et (40) : émissions d'exploitation annualisées.
    let beta1 = de_exploitation(de) / dve_ans * somme_f_co2();
    if dve_ans >= PER_ANS {
        [
            q * prod,
            q * edif,
            q * beta1,                     // éq. (35)
            q * fdv * f_per,               // éq. (36)
            q * de.module_d_kgco2 * f_per, // éq. (37)
        ]
    } else {
        let renouv = somme_renouvellements(dve_ans, f_co2);
        let beta2 = (prod + edif + fdv) * renouv; // éq. (41)
        [
            q * prod,
            q * edif,
            q * (beta1 + beta2),                      // éq. (42)
            q * fdv * f_per,                          // éq. (43)
            q * de.module_d_kgco2 * (renouv + f_per), // éq. (44)
        ]
    }
}

/// Composant spécifique au chantier (§4.5.1.1.4) : `Rp = 1`, modules B ignorés ; tous les
/// impacts sont affectés à la phase d'édification, sans pondération (§4.5 : émissions à
/// `t = 0`), module D compris (`docs/questions/Q207.md`).
pub fn composant_chantier(q: f64, de: &DonneeEnvironnementale) -> f64 {
    q * (de.production_kgco2 + de.edification_kgco2 + de.fin_de_vie_kgco2 + de.module_d_kgco2)
}

/// Impact statique des fuites d'exploitation d'un fluide frigorigène, pour une charge `ch_kg`
/// (annexe II §4.2.1.1.6, éq. 26) : `TExploitation × Chi × DEi × PER`.
pub fn fluide_statique(ch_kg: f64, de_kgco2_kg: f64) -> f64 {
    T_EXPLOITATION * ch_kg * de_kgco2_kg * PER_ANS
}

/// Impacts dynamiques d'un fluide, cas 1 (module B non détaillé), par unité fonctionnelle :
/// `(exploitation, fin de vie)` (éq. 45 à 47 et 51).
pub fn fluide_cas1(ch_kg: f64, prg: f64, dve_ans: f64) -> (f64, f64) {
    let b1 = T_EXPLOITATION * ch_kg * prg * somme_f_ff(); // éq. (45)
    let gamma = somme_renouvellements(dve_ans, |t| f_ff(t) - f_co2(t)); // éq. (47)
    let b4 = (T_PRODUCTION + T_FIN_DE_VIE) * ch_kg * prg * gamma; // éq. (46)
    let fdv = T_FIN_DE_VIE * ch_kg * prg * (f_ff(PER_ANS) - f_co2(PER_ANS)); // éq. (51)
    (b1 + b4, fdv)
}

/// Impacts dynamiques d'un fluide, cas 2 (module B1 détaillé), par unité fonctionnelle :
/// correctifs `(exploitation, fin de vie)` (éq. 54 à 56 et 60).
pub fn fluide_cas2(de_b1: f64, dve_ans: f64) -> (f64, f64) {
    let ecarts: f64 = (1..=50)
        .map(|a| f_ff(f64::from(a)) - f_co2(f64::from(a)))
        .sum();
    let b1 = de_b1 / dve_ans * ecarts; // éq. (54)
    let gamma = somme_renouvellements(dve_ans, |t| f_ff(t) - f_co2(t)); // éq. (55)
    let charge_prg = de_b1 / (T_EXPLOITATION * dve_ans);
    let b4 = (T_PRODUCTION + T_FIN_DE_VIE) * charge_prg * gamma; // éq. (56)
    let fdv = T_FIN_DE_VIE * charge_prg * (f_ff(PER_ANS) - f_co2(PER_ANS)); // éq. (60)
    (b1 + b4, fdv)
}

/// Impacts dynamiques d'un fluide d'un lot forfaitaire, cas 3 : `(exploitation, fin de vie)`
/// (éq. 65 à 67 et 70). L'éq. (67) est lue comme les éq. (47) et (55) : le terme en `FUtil`
/// est hors de la somme (`docs/questions/Q209.md`).
pub fn fluide_cas3(ch_kg: f64, prg: f64, dve_ans: f64) -> (f64, f64) {
    let b1 = T_EXPLOITATION * ch_kg * prg * somme_f_ff(); // éq. (65)
    let gamma = somme_renouvellements(dve_ans, f_ff); // éq. (67)
    let b4 = (T_PRODUCTION + T_FIN_DE_VIE) * ch_kg * prg * gamma; // éq. (66)
    let fdv = T_FIN_DE_VIE * ch_kg * prg * f_ff(PER_ANS); // éq. (70)
    (b1 + b4, fdv)
}

/// « StockCp » d'un composant, en kg C par unité fonctionnelle (annexe II §5.3.2.1, éq. 138 et
/// 139).
pub fn stock_c(de: &DonneeEnvironnementale) -> f64 {
    if let Some(s) = de.stock_c_kgc {
        return s; // cas 1
    }
    // Cas 2b si A1 est dissocié, cas 2a sinon.
    let ges = de.a1_kgco2.unwrap_or(de.production_kgco2);
    if ges < 0.0 {
        -ges * 12.0 / 44.0
    } else {
        0.0
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::entree::TypeDonneeEnvironnementale;

    fn de(prod: f64, edif: f64, expl: f64, fdv: f64, d: f64, dvr: f64) -> DonneeEnvironnementale {
        DonneeEnvironnementale {
            type_donnee: TypeDonneeEnvironnementale::FdesIndividuelle,
            identifiant: "fictif".into(),
            source: "test".into(),
            dvr_ans: dvr,
            production_kgco2: prod,
            edification_kgco2: edif,
            exploitation_kgco2: expl,
            fin_de_vie_kgco2: fdv,
            module_d_kgco2: d,
            exploitation_non_separee: false,
            b1_kgco2: None,
            a1_kgco2: None,
            stock_c_kgc: None,
        }
    }

    fn proche(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} ≠ {b}");
    }

    #[test]
    fn somme_des_coefficients() {
        // Tableau de l'article 11, années 1 à 50, sommé à la main : 39,543.
        proche(somme_f_co2(), 39.543);
        // Fluides : 47,788 (somme à la main des 50 valeurs relues).
        proche(somme_f_ff(), 47.788);
    }

    #[test]
    fn renouvellement_et_statique() {
        // DVE 20 ans : Rp = 2,5 ; éq. (11) : Q × [50/20 × 1 + 1,5 × (10 + 2 + 3)] = 2 × 25 = 50.
        let d = de(10.0, 2.0, 1.0, 3.0, -4.0, 20.0);
        let s = composant_statique(2.0, &d, 20.0);
        proche(s[0], 20.0);
        proche(s[1], 4.0);
        proche(s[2], 2.0 * (2.5 + 1.5 * 15.0));
        proche(s[3], 6.0);
        proche(s[4], 2.0 * 2.5 * -4.0);
        // DVE 100 ans : Rp = 1, exploitation au prorata 50/100 (éq. 12).
        let s = composant_statique(1.0, &d, 100.0);
        proche(s[2], 0.5);
        proche(s[4], -4.0);
    }

    #[test]
    fn dynamique_dve_superieure_a_la_per() {
        // Éq. (35) à (37), fCO2(50) = 0,578.
        let d = de(10.0, 2.0, 1.0, 3.0, -4.0, 60.0);
        let s = composant_dynamique(1.0, &d, 60.0);
        proche(s[0], 10.0);
        proche(s[2], 39.543 / 60.0);
        proche(s[3], 3.0 * 0.578);
        proche(s[4], -4.0 * 0.578);
    }

    #[test]
    fn dynamique_dve_20_ans() {
        // α = 2, FUtil = 0,5 ; renouvellements : fCO2(20) + 0,5 × fCO2(40) = 0,840 + 0,3345.
        let d = de(10.0, 2.0, 1.0, 3.0, -4.0, 20.0);
        let s = composant_dynamique(1.0, &d, 20.0);
        let r = 0.840 + 0.5 * 0.669;
        proche(s[2], 39.543 / 20.0 + 15.0 * r);
        proche(s[3], 3.0 * 0.578);
        proche(s[4], -4.0 * (r + 0.578));
        // DVE 25 ans : α = 2, FUtil = 0 ; un seul renouvellement, à 25 ans (0,798).
        let s = composant_dynamique(1.0, &d, 25.0);
        proche(s[2], 39.543 / 25.0 + 15.0 * 0.798);
    }

    #[test]
    fn fluides() {
        // Cas 3, DVE 17 ans : α = 2, FUtil = 50/17 − 2 ; γ = fff(17) + FUtil × fff(34).
        let futil = 50.0 / 17.0 - 2.0;
        let gamma = 0.980 + futil * 0.943;
        let (e, f) = fluide_cas3(1.0, 1000.0, 17.0);
        proche(e, 0.02 * 1000.0 * 47.788 + 0.12 * 1000.0 * gamma);
        proche(f, 0.10 * 1000.0 * 0.880);
        // Cas 1 : écarts fff − fCO2 ; fin de vie 0,1 × 1000 × (0,880 − 0,578).
        let (e, f) = fluide_cas1(1.0, 1000.0, 17.0);
        let gamma = (0.980 - 0.864) + futil * (0.943 - 0.721);
        proche(e, 0.02 * 1000.0 * 47.788 + 0.12 * 1000.0 * gamma);
        proche(f, 0.10 * 1000.0 * (0.880 - 0.578));
        // Cas 2 avec DE_B1 = TExploitation × Ch × PRG × DVE : même charge que le cas 1, mais
        // le terme B1 est le seul correctif (Σ fff − Σ fCO2).
        let b1 = 0.02 * 1000.0 * 17.0;
        let (e2, f2) = fluide_cas2(b1, 17.0);
        proche(
            e2,
            0.02 * 1000.0 * (47.788 - 39.543) + 0.12 * 1000.0 * gamma,
        );
        proche(f2, f);
        // DVE ≥ PER : pas de renouvellement, donc γ = 0 (cas 3 : 1,2 kg de PRG 675, DVE 60 ans).
        let (e, f) = fluide_cas3(1.2, 675.0, 60.0);
        proche(e, 0.02 * 1.2 * 675.0 * 47.788);
        proche(f, 0.10 * 1.2 * 675.0 * 0.880);
        let (e, _) = fluide_cas3(1.2, 675.0, 50.0);
        proche(e, 0.02 * 1.2 * 675.0 * 47.788);
        let (e, _) = fluide_cas1(1.2, 675.0, 60.0);
        proche(e, 0.02 * 1.2 * 675.0 * 47.788);
        // Statique (éq. 26) : 0,02 × 2 kg × 2000 × 50 = 4000.
        proche(fluide_statique(2.0, 2000.0), 4000.0);
    }

    #[test]
    fn carbone_biogenique() {
        let mut d = de(-44.0, 0.0, 0.0, 0.0, 0.0, 50.0);
        proche(stock_c(&d), 12.0);
        d.a1_kgco2 = Some(-88.0);
        proche(stock_c(&d), 24.0);
        d.stock_c_kgc = Some(5.0);
        proche(stock_c(&d), 5.0);
        let d = de(10.0, 0.0, 0.0, 0.0, 0.0, 50.0);
        proche(stock_c(&d), 0.0);
    }
}
