<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Contribuer

**Pour commencer : [`docs/architecture.md`](docs/architecture.md)**, guide d'architecture : parcours d'un calcul (ordre exact de la boucle horaire), carte des crates et des modules, recettes pas à pas (générateur, ventilation, usage, champ d'entrée, sortie, exemple, question, équation), conventions et tests ciblés avec leurs durées. **Tout nouveau lot met à jour la partie du guide qui le concerne** (nouvelle étape, nouveau module, nouvelle recette ou exemple à imiter) ; `cargo run -p xtask -- verifier` contrôle que chaque chemin et chaque élément de code qu'il cite existe.

## Règle première : le texte fait foi

Ce moteur implémente un texte réglementaire. Sa valeur tient à sa fidélité au texte.

- **Ne rien inventer.** Aucune formule, aucun coefficient, aucune valeur par défaut, aucun scénario ne s'écrit de mémoire. Tout vient d'un document de `docs/reference/`.
- **Citer.** Chaque fonction de calcul porte en commentaire la section et l'équation qu'elle implémente, par exemple `// Th-BCE 2020 §13.1.3, éq. (2377)`, et reprend les noms de variables du texte en `snake_case`, suffixés par l'unité (`u_k_w_m2k`, `a_b_m2`).
- **Lire le PDF.** Les extractions de `docs/reference/texte/` aplatissent les fractions et perdent des lettres grecques. Elles servent à chercher ; chaque équation se relit sur la page du PDF avant d'être codée.
- **Signaler les ambiguïtés.** Un manque ou une incohérence du texte se note dans `docs/questions/` (section, nature, hypothèse proposée) et se tranche avec le mainteneur avant d'être codé. **Une question = un fichier** `docs/questions/Q<nnn>.md`, avec un numéro pris dans la plage du lot, un en-tête (`numero`, `sujet`, `statut`) et les rubriques de `Q054.md` ; la règle complète est dans `docs/questions/README.md`. Le tableau de bord `docs/questions/ETAT.md` est généré (ne pas le modifier à la main) : `cargo run -p xtask -- questions` le met à jour, `cargo run -p xtask -- verifier` le contrôle ; en cas de conflit sur `ETAT.md` lors d'une intégration de `main`, garder n'importe quelle version puis relancer `cargo run -p xtask -- questions`. Ne pas modifier le README des questions. Dans le code et les documents, renvoyer à `docs/questions/Q<nnn>.md`.
- **Tenir la couverture.** `docs/couverture.md` suit le statut de chaque fiche de la méthode ; un lot ne modifie que les lignes des fiches qu'il traite.
- **Tenir le journal des modifications.** Ne pas écrire dans `CHANGELOG.md` : créer un fragment `changelog.d/<lot>.md` en sections Keep a Changelog (`### Ajouté`, `### Modifié`, `### Corrigé`), selon `changelog.d/README.md`. Le mainteneur les assemble à la publication (`cargo run -p xtask -- changelog <version>`).

## Données météo réglementaires

Les fichiers météorologiques conventionnels de la RE2020 **ne doivent jamais entrer dans le dépôt**, même partiellement, même dans un test ou une issue : leurs conditions d'utilisation l'interdisent. Le `.gitignore` et la CI le vérifient. Les tests qui en dépendent sont marqués `#[ignore]` et lisent la variable `THBCE_METEO`.

## Données versionnées

Les tables de `thbce-data` issues d'un document se génèrent par script (`xtask`), pas à la main, et la CI vérifie qu'elles sont à jour. Une table transcrite d'une image (pondération dynamique) est protégée par des tests de contrôle.

## Retrouver une équation ou un tableau du texte réglementaire

Le PDF fait foi : une équation ou un tableau se lit sur l'image de sa page, pas dans l'extraction texte. Deux commandes aident à y accéder (pagination du PDF ; pour l'annexe III elle coïncide avec « n/1854 »).

```sh
cargo run -p xtask -- index-texte            # régénère docs/reference/index/
cargo run -p xtask -- page 142               # image PNG de la page 142 (150 dpi)
cargo run -p xtask -- page --eq 134          # page de l'équation (134)
cargo run -p xtask -- page --tableau 56-1    # page du tableau 56-1 (suffixes admis)
cargo run -p xtask -- page --figure 191      # page de la figure 191
cargo run -p xtask -- page --eq 75 --annexe ii   # annexe II (règles générales)
```

- `index-texte` lit les extractions de `docs/reference/texte/` et écrit, dans `docs/reference/index/`, les tables CSV `annexe-iii-{equations,tableaux,figures,fiches}.csv` et `annexe-ii-{equations,tableaux,figures}.csv`, ainsi que `anomalies.md` (numéros introuvables, en double, hors séquence : aucun n'est inventé). `xtask verifier` contrôle que ces fichiers sont à jour et que les plages de pages des fiches recoupent `docs/couverture.md`. La méthode est décrite en tête de `xtask/src/index_texte.rs`.
- Colonnes des CSV (séparateur virgule, champs entre guillemets si besoin) :
  - `annexe-*-equations.csv` : `equation` (numéro), `page` (page du numéro), `tete_de_page` (1 si le numéro ouvre la page : l'équation peut commencer sur la page précédente), `doublon` (1 pour une occurrence supplémentaire d'un numéro déjà rencontré), `fiche` (annexe III seulement : fiche, ou chapitre, courante à cet endroit du texte).
  - `annexe-*-tableaux.csv` et `annexe-*-figures.csv` : `tableau` ou `figure` (numéro, suffixe admis : `56-1`), `page` (page de la légende, qui suit le tableau), `tete_de_page` (tableaux : 1 si la légende ouvre la page, le corps du tableau est alors sur la page précédente ; signal heuristique à vérifier sur l'image), `fiche` (annexe III seulement), `titre`.
  - `annexe-iii-fiches.csv` : `numero` (`5` ou `5.13`), `type` (`chapitre` ou `fiche`), `titre`, `page_debut`, `page_fin`.
- Exemple : `grep '^1238,' docs/reference/index/annexe-iii-equations.csv` donne la ou les lignes de l'équation (1238) ; plusieurs lignes signalent un numéro en double.
- `page --eq`, `--tableau` et `--figure` sortent en **code 1** avec un message sur un numéro en double (demander alors `page <N>` avec la page voulue) ou introuvable (voir `docs/reference/index/anomalies.md`). Une équation ou un tableau dont `tete_de_page` vaut 1 produit deux images (page précédente puis page du numéro).
- `page` appelle `pdftoppm` (paquet `poppler-utils`) et écrit dans `target/pages/` (jamais versionné) ; un message clair s'affiche s'il manque.

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

Pendant le développement, les alias de `.cargo/config.toml` lancent un module, un fichier de tests, un exemple de `tests/cas` ou un test réglementaire ignoré (`cargo t-core`, `cargo t-io`, `cargo t-cas <nom>`, `cargo t-regl <filtre>` ; voir `docs/architecture.md`, « Tests ciblés »). Ils ne remplacent pas les vérifications ci-dessous.

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
- `CHANGELOG.md` au format Keep a Changelog, assemblé depuis `changelog.d/` ; versionnage sémantique.
