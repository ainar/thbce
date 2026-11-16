<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Cas de validation

Un dossier par cas :

```
tests/cas/<nom-du-cas>/
├── entree.json     # projet au format JSON natif
├── attendu.json    # résultats de référence
└── cas.toml        # provenance, météo requise, tolérances
```

Ajouter un cas se limite à déposer ce dossier : le harnais (`crates/thbce-io/tests/cas.rs`) parcourt `tests/cas/`, calcule chaque entrée et compare.

## `attendu.json`

Même structure que les résultats du moteur (`thbce schema --resultats`), mais partielle : **seules les valeurs présentes sont comparées**. Les nombres sont comparés avec la tolérance du cas, les chaînes et les booléens à l'identique, les tableaux élément par élément. Un indicateur que le moteur ne calcule pas encore (`null`) alors qu'une valeur est attendue fait échouer le cas.

## `cas.toml`

```toml
description = "Maison individuelle, PAC air/eau, H2b"
source = "COMETH 1.x, export du jj/mm/aaaa"   # provenance des résultats de référence
meteo = "re2020"                                # "re2020" ou "synthetique"

[tolerances]
defaut = 0.005          # écart relatif maximal, 0,5 % par défaut
bbio_pts = 0.005        # surcharge par nom de champ
dh_c_h = 0.01
```

- `meteo = "re2020"` : le cas a besoin des fichiers météo réglementaires, qui ne sont pas dans le dépôt. Il est **ignoré** si la variable `THBCE_METEO` ne pointe pas vers le fichier CSV ; la CI publique ne l'exécute donc pas.
- `meteo = "synthetique"` : jeu constant sans signification physique, réservé aux cas de plomberie.
- L'écart est relatif à la valeur attendue ; pour une valeur attendue nulle, il est absolu.

Le harnais compare les valeurs telles que le moteur les restitue, c'est-à-dire arrondies à une décimale (Th-BCE 2020 §1.3.1).
