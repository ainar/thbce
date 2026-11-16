<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Contribuer

## Règle première : le texte fait foi

Ce moteur implémente un texte réglementaire. Sa valeur tient à sa fidélité au texte.

- **Ne rien inventer.** Aucune formule, aucun coefficient, aucune valeur par défaut, aucun scénario ne s'écrit de mémoire. Tout vient d'un document de `docs/reference/`.
- **Citer.** Chaque fonction de calcul porte en commentaire la section et l'équation qu'elle implémente, par exemple `// Th-BCE 2020 §13.1.3, éq. (2377)`, et reprend les noms de variables du texte en `snake_case`, suffixés par l'unité (`u_k_w_m2k`, `a_b_m2`).
- **Lire le PDF.** Les extractions de `docs/reference/texte/` aplatissent les fractions et perdent des lettres grecques. Elles servent à chercher ; chaque équation se relit sur la page du PDF avant d'être codée.
- **Signaler les ambiguïtés.** Un manque ou une incohérence du texte se note dans `docs/questions.md` (section, nature, hypothèse proposée) et se tranche avec le mainteneur avant d'être codé.
- **Tenir la couverture.** `docs/couverture.md` suit le statut de chaque fiche de la méthode.

## Données météo réglementaires

Les fichiers météorologiques conventionnels de la RE2020 **ne doivent jamais entrer dans le dépôt**, même partiellement, même dans un test ou une issue : leurs conditions d'utilisation l'interdisent. Le `.gitignore` et la CI le vérifient. Les tests qui en dépendent sont marqués `#[ignore]` et lisent la variable `THBCE_METEO`.

## Données versionnées

Les tables de `thbce-data` issues d'un document se génèrent par script (`xtask`), pas à la main, et la CI vérifie qu'elles sont à jour. Une table transcrite d'une image (pondération dynamique) est protégée par des tests de contrôle.

## Code

- `thbce-core` reste pur : ni fichier, ni réseau, ni horloge, ni aléa.
- Jamais de `panic!` sur une entrée utilisateur : une incohérence de saisie produit une erreur qui désigne l'élément fautif.
- Résultats identiques au bit près entre natif et WebAssembly : les fonctions transcendantes passent par la crate `libm` (`clippy.toml` interdit celles de la bibliothèque standard), et aucun parallélisme ne change l'ordre des sommations.
- Pas d'allocation dans la boucle horaire.
- Fidélité au texte et lisibilité avant élégance et performance.
- En-tête `// SPDX-License-Identifier: GPL-3.0-or-later` dans chaque fichier source ; dépendances de licence compatible avec la GPL v3 (`cargo deny`).

## Tests

- Tests unitaires sur des valeurs calculées à la main à partir des formules du texte.
- Tests de propriétés sur les invariants physiques.
- Cas de validation dans `tests/cas/` (voir `tests/cas/README.md`).

Avant de pousser :

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p xtask -- verifier
```

## Branches et versions

- Une branche et une pull request par phase, lot ou fiche ; fusion dans `main` avec la CI verte.
- Commits atomiques, messages explicites, en français.
- `CHANGELOG.md` au format Keep a Changelog ; versionnage sémantique.
