// SPDX-License-Identifier: GPL-3.0-or-later
//! Saisons de fonctionnement des systèmes de chaud et de froid : fiche `C_Ein_Détermination des
//! saisons de fonctionnement des systèmes de chaud et de froid` (Th-BCE 2020 §8.4).
//!
//! En calcul des consommations, les autorisations effectives d'un groupe viennent des
//! générations qui le desservent : une génération reste activée tant qu'un groupe desservi est
//! en saison (éq. 839) ; un groupe desservi par une génération à raccordement permanent des
//! réseaux en suit la saison (éq. 840), un groupe indépendant garde ses autorisations propres
//! (éq. 841).
//!
//! Lectures : `docs/questions.md`, Q43.

/// Desserte d'un groupe : générations qui le desservent en chaud et en froid
/// (« iddesservi_ch^gen,gr » et « iddesservi_fr^gen,gr »), par indice de génération.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DesserteGroupe {
    /// Générations qui desservent le groupe en chaud.
    pub chaud: Vec<usize>,
    /// Générations qui desservent le groupe en froid.
    pub froid: Vec<usize>,
    /// « iclim^gr ».
    pub i_clim: bool,
}

/// Autorisations d'un groupe : `(Autch, Autfr)`.
pub type Autorisations = (bool, bool);

/// Autorisations effectives de chaque groupe d'un bâtiment (éq. 839 à 841).
///
/// `propres` : autorisations propres des groupes (fiche 4.6) ; `dessertes` : desserte de
/// chaque groupe, dans le même ordre ; `permanent` : « idraccord_reseau » de chaque génération.
/// Renvoie aussi les autorisations des générations `(Autch^gen, Autfr^gen)`.
pub fn autorisations_effectives(
    propres: &[Autorisations],
    dessertes: &[DesserteGroupe],
    permanent: &[bool],
) -> (Vec<Autorisations>, Vec<Autorisations>) {
    // Éq. (839) : les groupes non climatisés ne comptent pas pour le refroidissement.
    let mut gen = vec![(false, false); permanent.len()];
    for (p, d) in propres.iter().zip(dessertes) {
        for &g in &d.chaud {
            if let Some(a) = gen.get_mut(g) {
                a.0 |= p.0;
            }
        }
        for &g in &d.froid {
            if let Some(a) = gen.get_mut(g) {
                a.1 |= d.i_clim && p.1;
            }
        }
    }
    let effectives = propres
        .iter()
        .zip(dessertes)
        .map(|(p, d)| {
            let raccorde = |g: &usize| permanent.get(*g).copied().unwrap_or(false);
            if d.chaud.iter().chain(&d.froid).any(raccorde) {
                // Éq. (840).
                (
                    d.chaud.iter().filter(|g| raccorde(g)).any(|g| gen[*g].0),
                    d.froid.iter().filter(|g| raccorde(g)).any(|g| gen[*g].1),
                )
            } else {
                // Éq. (841).
                *p
            }
        })
        .collect();
    (effectives, gen)
}
