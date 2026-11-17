### Ajouté

- Index du texte réglementaire dans `docs/reference/index/` : numéro d'équation, de tableau et de figure vers la page, et fiches (titre, plage de pages) de l'annexe III, équations, tableaux et figures de l'annexe II, avec `anomalies.md` (numéros introuvables, en double ou hors séquence). Générés par `cargo run -p xtask -- index-texte`, contrôlés par `cargo run -p xtask -- verifier` (qui recoupe aussi les plages de `docs/couverture.md`).
- `cargo run -p xtask -- page <N> | --eq <n> | --tableau <t> | --figure <f> [--annexe iii|ii]` : image PNG d'une page du PDF (150 dpi, `target/pages/`), pour lire une équation ou un tableau sur le texte qui fait foi.

Aucun résultat de calcul ne change.
