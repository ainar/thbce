### Ajouté

- `docs/reference/tableaux/annexe-iii/` : les tableaux 107 à 266 de l'annexe III (fiches 8.14 à 8.33, génération) en CSV, avec README (provenance, méthode, convention, écarts de mise en forme). Aucun code ne change.
- `docs/reference/index/anomalies-constatees.md` : anomalies du texte et de l'index relevées sur l'image (tableau 134 absent, 115 et 122 en double, 257 imprimé deux fois, ρe 1000 contre 1006, symbole absent au tableau 259, etc.), reprises à la fin de `anomalies.md` par `xtask index-texte`.

### Modifié

- `xtask index-texte` ajoute à `anomalies.md` le contenu de `anomalies-constatees.md` lorsqu'il existe.
