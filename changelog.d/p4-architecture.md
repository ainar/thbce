### Ajouté

- Guide d'architecture `docs/architecture.md` : parcours d'un calcul (lecture, validation, préparation, ordre exact des étapes de la boucle horaire du bâtiment, de la zone, du groupe, des réseaux et des générations, passes Th-B, Th-D et Th-C, consommations, indicateurs, restitution et arrondi), carte des crates et des modules, recettes avec un exemple à imiter (générateur, système de ventilation, usage, champ d'entrée, sortie, exemple de `tests/cas`, question, lecture d'une équation), conventions et tests ciblés avec leurs durées. Renvoi en tête de `CONTRIBUTING.md` et dans le README ; tout nouveau lot met à jour la partie du guide qui le concerne.
- `cargo run -p xtask -- architecture`, appelé par `xtask verifier` : contrôle que chaque chemin et chaque élément de code (`fichier.rs::Element`) cité par le guide existe.
- `cargo run -p xtask -- cas [<nom>…]` : calcule un ou plusieurs exemples de `tests/cas/`, affiche leurs indicateurs et les compare à leur `attendu.json` avec les règles du harnais.
- Alias de tests ciblés dans `.cargo/config.toml` : `cargo t-core`, `cargo t-io`, `cargo t-cas <nom>`, `cargo t-regl <filtre>` (tests ignorés sur la météo réglementaire), `cargo t-complet` (suite complète, toujours obligatoire avant un push) ; profil Cargo `rapide` (optimisé comme `release`, sans LTO ; sorties identiques à l'octet près sur les quinze exemples ; construction initiale des tests en 4 min au lieu de 24).

Aucun résultat de calcul ne change.
