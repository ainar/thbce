// SPDX-License-Identifier: GPL-3.0-or-later
//! Données que le texte donne lui-même pour l'ACV : valeurs forfaitaires de certains lots
//! (annexe XI de l'arrêté du 4 août 2021, version consolidée au 19 mars 2026,
//! `docs/reference/arrete/annexexi_arrete_4_aout_2021.pdf`, tableaux relus sur l'image des
//! pages 1 à 7) et consommations conventionnelles d'eau potable (annexe II, tableau 7).

/// Valeurs forfaitaires d'un lot ou sous-lot, en kg éq. CO2/m² : production, édification,
/// exploitation, fin de vie, module D (annexe XI).
pub type Forfait = [f64; 5];

/// Lignes d'une table de l'annexe XI : `8.1`, `8.3`, `8.4`, `8.5`, `10`, `11`.
type Table = [(&'static str, Forfait); 6];

/// Usages 1 et 2 : seuls `8.1`, `10` et `11` ont une valeur forfaitaire.
const MAISON: [(&str, Forfait); 3] = [
    ("8.1", [61.0, 0.0, 106.0, 3.0, 0.0]),
    ("10", [50.0, 0.0, 45.0, 3.0, 0.0]),
    ("11", [1.0, 0.0, 1.0, 0.0, 0.0]),
];
const COLLECTIF: [(&str, Forfait); 3] = [
    ("8.1", [16.0, 0.0, 57.0, 1.0, 0.0]),
    ("10", [24.0, 0.0, 23.0, 1.0, 0.0]),
    ("11", [1.0, 0.0, 1.0, 0.0, 0.0]),
];

const fn table(
    l81: Forfait,
    l83: Forfait,
    l84: Forfait,
    l85: Forfait,
    l10: Forfait,
    l11: Forfait,
) -> Table {
    [
        ("8.1", l81),
        ("8.3", l83),
        ("8.4", l84),
        ("8.5", l85),
        ("10", l10),
        ("11", l11),
    ]
}

/// Lots 10 et 11 communs à la plupart des usages tertiaires.
const L10: Forfait = [57.0, 0.0, 54.0, 4.0, 0.0];
const L11: Forfait = [8.0, 0.0, 7.0, 0.0, 0.0];
const L10_HOTEL: Forfait = [24.0, 0.0, 23.0, 1.0, 0.0];
const L10_RESTAURANT: Forfait = [50.0, 0.0, 45.0, 3.0, 0.0];
const L11_1: Forfait = [1.0, 0.0, 1.0, 0.0, 0.0];

const BUREAUX: Table = table(
    [13.0, 0.0, 46.0, 1.0, 0.0],
    [13.0, 0.0, 46.0, 1.0, 0.0],
    [11.0, 0.0, 38.0, 1.0, 0.0],
    [7.0, 0.0, 23.0, 0.0, 0.0],
    L10,
    L11,
);
const ENSEIGNEMENT: Table = table(
    [9.0, 0.0, 31.0, 0.0, 0.0],
    [5.0, 0.0, 15.0, 0.0, 0.0],
    [9.0, 0.0, 31.0, 0.0, 0.0],
    [11.0, 0.0, 38.0, 1.0, 0.0],
    L10,
    L11,
);
const MEDIATHEQUES: Table = table(
    [12.0, 0.0, 86.0, 2.0, 0.0],
    [13.0, 0.0, 46.0, 1.0, 0.0],
    [13.0, 1.0, 24.0, 1.0, 0.0],
    [29.0, 4.0, 17.0, 1.0, 0.0],
    L10,
    L11,
);
const UNIVERSITAIRES: Table = table(
    [9.0, 0.0, 31.0, 0.0, 0.0],
    [48.0, 3.0, 2.0, 0.0, 0.0],
    [8.0, 1.0, 46.0, 0.0, 0.0],
    [62.0, 8.0, 18.0, 1.0, 0.0],
    L10,
    L11,
);
const HOTELS: Table = table(
    [8.0, 0.0, 52.0, 1.0, 0.0],
    [18.0, 1.0, 34.0, 1.0, 0.0],
    [10.0, 1.0, 17.0, 1.0, 0.0],
    [53.0, 5.0, 29.0, 1.0, 0.0],
    L10_HOTEL,
    L11_1,
);
const PETITE_ENFANCE: Table = table(
    [26.0, 1.0, 121.0, 3.0, 0.0],
    [46.0, 0.0, 0.0, 2.0, 0.0],
    [3.0, 0.0, 5.0, 0.0, 0.0],
    [21.0, 3.0, 16.0, 1.0, 0.0],
    L10,
    L11,
);
const RESTAURANTS: Table = table(
    [19.0, 1.0, 117.0, 3.0, 0.0],
    [16.0, 1.0, 2.0, 0.0, 0.0],
    [15.0, 1.0, 26.0, 1.0, 0.0],
    [33.0, 6.0, 18.0, 1.0, 0.0],
    L10_RESTAURANT,
    L11_1,
);
const COMMERCES: Table = table(
    [12.0, 0.0, 142.0, 9.0, 0.0],
    [9.0, 1.0, 14.0, 0.0, 0.0],
    [5.0, 0.0, 8.0, 0.0, 0.0],
    [22.0, 2.0, 9.0, 0.0, 0.0],
    L10,
    L11,
);
const VESTIAIRES: Table = table(
    [29.0, 2.0, 143.0, 1.0, 0.0],
    [30.0, 2.0, 1.0, 1.0, 0.0],
    [16.0, 2.0, 21.0, 0.0, 0.0],
    [28.0, 3.0, 11.0, 1.0, 0.0],
    L10,
    L11,
);
const SANTE: Table = table(
    [10.0, 0.0, 66.0, 2.0, 0.0],
    [10.0, 1.0, 6.0, 0.0, 0.0],
    [7.0, 1.0, 20.0, 0.0, 0.0],
    [89.0, 14.0, 46.0, 2.0, 0.0],
    L10,
    L11,
);
const AEROGARES: Table = table(
    [2.0, 0.0, 3.0, 0.0, 0.0],
    [3.0, 0.0, 7.0, 0.0, 0.0],
    [10.0, 1.0, 61.0, 1.0, 0.0],
    [110.0, 15.0, 27.0, 2.0, 0.0],
    L10,
    L11,
);
const INDUSTRIE: Table = table(
    [12.0, 1.0, 77.0, 1.0, 0.0],
    [3.0, 0.0, 7.0, 0.0, 0.0],
    [10.0, 0.0, 18.0, 1.0, 0.0],
    [59.0, 10.0, 21.0, 1.0, 0.0],
    L10,
    L11,
);
const SPORT: Table = table(
    [8.0, 0.0, 39.0, 0.0, 0.0],
    [9.0, 1.0, 7.0, 0.0, 0.0],
    [51.0, 7.0, 51.0, 2.0, 0.0],
    [10.0, 1.0, 1.0, 0.0, 0.0],
    L10,
    L11,
);

/// Valeur forfaitaire de l'annexe XI d'un lot ou sous-lot pour un usage (1 à 28) ; `None` si
/// l'annexe n'en donne pas. Les valeurs sont celles de l'indicateur d'impact sur le
/// changement climatique, utilisées en méthode statique comme en méthode dynamique
/// (`docs/questions/Q208.md`).
pub fn forfait(usage: u8, sous_lot: &str) -> Option<Forfait> {
    let lignes: &[(&str, Forfait)] = match usage {
        1 => &MAISON,
        2 => &COLLECTIF,
        3 => &BUREAUX,
        4 | 5 => &ENSEIGNEMENT,
        6 => &MEDIATHEQUES,
        7 => &UNIVERSITAIRES,
        8..=11 => &HOTELS,
        12 => &PETITE_ENFANCE,
        13..=16 | 26 | 27 => &RESTAURANTS,
        17 => &COMMERCES,
        18 => &VESTIAIRES,
        19..=21 => &SANTE,
        22 => &AEROGARES,
        23 | 24 => &INDUSTRIE,
        25 | 28 => &SPORT,
        _ => return None,
    };
    lignes.iter().find(|(k, _)| *k == sous_lot).map(|(_, v)| *v)
}

/// « Qeau potable conv » : consommation conventionnelle d'eau potable, en m³ par occupant et
/// par an (annexe II, tableau 7) ; `None` pour les usages marqués « N/C ».
pub fn q_eau_potable_conv_m3(usage: u8) -> Option<f64> {
    match usage {
        1 | 2 | 8 | 9 => Some(48.0),
        3 => Some(5.59),
        4..=7 | 12 => Some(1.44),
        _ => None,
    }
}

/// Sous-lots du tableau 5 de l'annexe II (lots 12 et 13 sans sous-lot).
pub const SOUS_LOTS: [&str; 46] = [
    "1.1", "1.2", "1.3", "2.1", "2.2", "2.3", "3.1", "3.2", "3.3", "3.4", "3.5", "3.6", "3.7",
    "3.8", "4.1", "4.2", "4.3", "5.1", "5.2", "5.3", "5.4", "5.5", "6.1", "6.2", "6.3", "7.1",
    "7.2", "7.3", "8.1", "8.2", "8.3", "8.4", "8.5", "8.6", "8.7", "9.1", "9.2", "10.1", "10.2",
    "10.3", "10.4", "10.5", "10.6", "11.1", "11.2", "11.3",
];

/// Lot d'un sous-lot (`3.2` → 3, `12` → 12) ; `None` hors nomenclature.
pub fn lot_de(sous_lot: &str) -> Option<u8> {
    if sous_lot == "12" || sous_lot == "13" {
        return sous_lot.parse().ok();
    }
    if !SOUS_LOTS.contains(&sous_lot) {
        return None;
    }
    sous_lot.split('.').next()?.parse().ok()
}

/// Lot d'un lot ou sous-lot forfaitaire (`8.1` → 8, `10` → 10).
pub fn lot_forfait(k: &str) -> Option<u8> {
    match k {
        "8.1" | "8.3" | "8.4" | "8.5" => Some(8),
        "10" => Some(10),
        "11" => Some(11),
        _ => None,
    }
}

/// Vrai si le sous-lot `s` (nomenclature du tableau 5) relève du lot ou sous-lot forfaitaire
/// `k` (`10.3` relève de `10`).
pub fn releve_de(s: &str, k: &str) -> bool {
    s == k || s.split('.').next() == Some(k)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn valeurs_relues() {
        assert_eq!(forfait(1, "8.1"), Some([61.0, 0.0, 106.0, 3.0, 0.0]));
        assert_eq!(forfait(1, "8.3"), None);
        assert_eq!(forfait(2, "10"), Some([24.0, 0.0, 23.0, 1.0, 0.0]));
        assert_eq!(forfait(12, "8.3"), Some([46.0, 0.0, 0.0, 2.0, 0.0]));
        assert_eq!(forfait(22, "8.5"), Some([110.0, 15.0, 27.0, 2.0, 0.0]));
        assert_eq!(forfait(29, "10"), None);
        assert_eq!(q_eau_potable_conv_m3(3), Some(5.59));
        assert_eq!(q_eau_potable_conv_m3(17), None);
        assert_eq!(lot_de("3.2"), Some(3));
        assert_eq!(lot_de("13"), Some(13));
        assert_eq!(lot_de("3.9"), None);
        assert!(releve_de("10.3", "10"));
        assert!(!releve_de("8.2", "8.1"));
    }
}
