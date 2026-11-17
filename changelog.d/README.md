<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Fragments du journal des modifications

Pour que deux lots menés en parallèle ne modifient pas le même fichier, **un lot n'écrit jamais dans `CHANGELOG.md`** : il crée son propre fragment `changelog.d/<lot>.md` (minuscules, chiffres et tirets, par exemple `lot-8a.md`).

Un fragment est rédigé en sections [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), au niveau `###` :

```markdown
### Ajouté

- Ce que le lot ajoute (fiches, équations, champs facultatifs, exemples).

### Modifié

- Ce qui change ; un changement incompatible du format ou des résultats s'y signale en toutes lettres.

### Corrigé

- Ce qui est corrigé.
```

Sections usuelles : `Ajouté`, `Modifié`, `Obsolète`, `Retiré`, `Corrigé`, `Sécurité` ; d'autres titres sont admis. Aucun texte avant la première section, aucune section vide, pas de titre de version.

À la publication, le mainteneur lance :

```sh
cargo run -p xtask -- changelog --apercu      # affiche l'assemblage, n'écrit rien
cargo run -p xtask -- changelog 0.2.0         # écrit la section datée, supprime les fragments
```

L'assemblage réunit les sections de même titre, dans l'ordre de Keep a Changelog, et insère la nouvelle section `## [version] - date` sous `## [Non publié]` de `CHANGELOG.md` (`--date AAAA-MM-JJ` fixe la date). `cargo run -p xtask -- verifier` contrôle que les fragments présents sont lisibles.
