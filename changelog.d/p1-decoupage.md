### Modifié

- Interne — `entree.rs`, `validation.rs`, `resultats.rs`, `calcul.rs`, `chauffage_refroidissement/generation.rs` et `validation/generation.rs` sont découpés en sous-modules par famille (bâtiment, usages, exigences, enveloppe, baies, espaces tampons, ventilation, émission et distribution, génération par type de générateur, ECS, solaire, déplacements). Déplacement de code pur : format d'entrée et de résultats, schémas JSON, types TypeScript, CLI et résultats inchangés (PR #49 à #53).
