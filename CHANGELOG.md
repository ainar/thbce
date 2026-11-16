<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Journal des modifications

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le projet le [versionnage sémantique](https://semver.org/lang/fr/). Chaque version indique la version de la méthode Th-BCE implémentée.

## [Non publié]

Méthode : Th-BCE 2020, annexe III de l'arrêté du 4 août 2021, version consolidée au 19 mars 2026.

### Ajouté

- Workspace Cargo : `thbce-core`, `thbce-data`, `thbce-io`, `thbce-cli`, `thbce-wasm`, `xtask`.
- Types d'entrée du projet (localisation, catalogue, bâtiment, zone, groupe, parois, baies, ponts thermiques, masques, inertie), d'après les nomenclatures de l'annexe III.
- Format d'entrée JSON natif et son JSON Schema.
- Validation de l'entrée avec rapport par élément fautif.
- Lecture du fichier météo au format CSV long ; les données météo ne sont jamais embarquées.
- Exécutable `thbce` : commandes `calcul`, `valider`, `schema`, `version`.
- Module WebAssembly : `calculer`, `valider`, `versionMoteur`, `schemaEntree`.
- Calcul de la surface de référence (§13.1.3) ; les indicateurs réglementaires ne sont pas encore calculés.
- Table département → zone climatique, générée depuis l'annexe I de l'arrêté du 26 octobre 2010.
- Coefficients de pondération dynamique de l'article 11 de l'arrêté.
- Harnais des cas de validation (`tests/cas/`) avec tolérances par indicateur.
- Intégration continue : format, clippy, licences, tests sur Linux, macOS et Windows, build et tests WebAssembly, parité natif / `wasm32-wasip1`.
