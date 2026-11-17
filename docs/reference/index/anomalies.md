<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Anomalies de l'index du texte réglementaire

Fichier généré par `cargo run -p xtask -- index-texte`, ne pas modifier à la main.

Les anomalies sont classées par **nature**, car l'index se fonde sur l'extraction texte (calque du PDF), qui n'est pas l'image des pages :

- *Absent de l'extraction* : le numéro n'apparaît pas dans le texte extrait (équation dessinée comme image, numéro écrit autrement, légende absente). Il peut exister sur l'image : la lire avant de conclure à une anomalie du PDF.
- *En double* : le même numéro apparaît plusieurs fois. Peut être une vraie double numérotation du PDF (les deux occurrences sont dans l'index, colonne `doublon`) ou un numéro parasite (cellule de tableau, indice).
- *Hors séquence dans l'extraction* : l'ordre du texte extrait n'est pas l'ordre de la page (calque), ou deux équations sont permutées. L'équation est retenue à sa place dans le texte ; l'image est en général dans l'ordre : artefact d'extraction probable.

Aucun numéro n'est déduit ni inventé : en cas de doute, lire l'image de la page (`cargo run -p xtask -- page`). Pour l'annexe II, la page est la page du PDF (le numéro imprimé est supérieur d'une unité : PDF 68, imprimé 69).

## annexe III

### Fiches (0)

Aucune.

### Équations (87)

#### Absent de l'extraction (7)

- éq. (142) : introuvable dans le texte extrait (pages 147 à 148)
- éq. (590) : introuvable dans le texte extrait (page 391)
- éq. (2270) : introuvable dans le texte extrait (page 1293)
- éq. (3099) : introuvable dans le texte extrait (pages 1645 à 1650)
- éq. (3140) : introuvable dans le texte extrait (page 1669)
- éq. (3170) à (3171) : introuvable dans le texte extrait (pages 1697 à 1698)
- éq. (3180) : introuvable dans le texte extrait (pages 1699 à 1700)

#### En double (68)

- éq. (1) : numéro en double, page 158 (retenu : page 32)
- éq. (15) : numéro en double, page 1803 (retenu : page 41)
- éq. (17) : numéro en double, page 150 (retenu : page 41)
- éq. (23) : numéro en double, page 153 (retenu : page 51)
- éq. (34) : numéro en double, page 156 (retenu : page 60)
- éq. (79) : numéro en double, page 1632 (retenu : page 88)
- éq. (314) : numéro en double, page 309 (retenu : page 312)
- éq. (383) : numéro en double, page 341 (retenu : page 331)
- éq. (384) : numéro en double, page 341 (retenu : page 331)
- éq. (385) : numéro en double, page 341 (retenu : page 331)
- éq. (386) : numéro en double, page 359 (retenu : page 331)
- éq. (387) : numéro en double, page 359 (retenu : page 332)
- éq. (388) : numéro en double, page 359 (retenu : page 332)
- éq. (389) : numéro en double, page 359 (retenu : page 332)
- éq. (390) : numéro en double, page 360 (retenu : page 332)
- éq. (391) : numéro en double, page 360 (retenu : page 333)
- éq. (392) : numéro en double, page 360 (retenu : page 333)
- éq. (393) : numéro en double, page 360 (retenu : page 333)
- éq. (394) : numéro en double, page 360 (retenu : page 333)
- éq. (395) : numéro en double, page 360 (retenu : page 333)
- éq. (396) : numéro en double, page 360 (retenu : page 333)
- éq. (397) : numéro en double, page 361 (retenu : page 333)
- éq. (398) : numéro en double, page 361 (retenu : page 333)
- éq. (399) : numéro en double, page 361 (retenu : page 334)
- éq. (400) : numéro en double, page 361 (retenu : page 336)
- éq. (401) : numéro en double, page 361 (retenu : page 336)
- éq. (402) : numéro en double, page 334 (retenu : page 336)
- éq. (402) : numéro en double, page 361 (retenu : page 336)
- éq. (403) : numéro en double, page 361 (retenu : page 336)
- éq. (404) : numéro en double, page 361 (retenu : page 337)
- éq. (405) : numéro en double, page 334 (retenu : page 337)
- éq. (405) : numéro en double, page 362 (retenu : page 337)
- éq. (406) : numéro en double, page 362 (retenu : page 337)
- éq. (407) : numéro en double, page 362 (retenu : page 337)
- éq. (408) : numéro en double, page 334 (retenu : page 337)
- éq. (408) : numéro en double, page 362 (retenu : page 337)
- éq. (409) : numéro en double, page 363 (retenu : page 338)
- éq. (410) : numéro en double, page 363 (retenu : page 338)
- éq. (411) : numéro en double, page 363 (retenu : page 338)
- éq. (412) : numéro en double, page 364 (retenu : page 338)
- éq. (413) : numéro en double, page 364 (retenu : page 338)
- éq. (414) : numéro en double, page 364 (retenu : page 338)
- éq. (415) : numéro en double, page 364 (retenu : page 338)
- éq. (416) : numéro en double, page 364 (retenu : page 339)
- éq. (417) : numéro en double, page 364 (retenu : page 339)
- éq. (418) : numéro en double, page 364 (retenu : page 339)
- éq. (419) : numéro en double, page 365 (retenu : page 339)
- éq. (420) : numéro en double, page 365 (retenu : page 339)
- éq. (421) : numéro en double, page 365 (retenu : page 339)
- éq. (422) : numéro en double, page 365 (retenu : page 340)
- éq. (423) : numéro en double, page 365 (retenu : page 340)
- éq. (424) : numéro en double, page 365 (retenu : page 340)
- éq. (425) : numéro en double, page 366 (retenu : page 340)
- éq. (426) : numéro en double, page 366 (retenu : page 340)
- éq. (427) : numéro en double, page 366 (retenu : page 340)
- éq. (428) : numéro en double, page 367 (retenu : page 341)
- éq. (429) : numéro en double, page 367 (retenu : page 341)
- éq. (430) : numéro en double, page 367 (retenu : page 341)
- éq. (431) : numéro en double, page 368 (retenu : page 341)
- éq. (432) : numéro en double, page 368 (retenu : page 341)
- éq. (433) : numéro en double, page 368 (retenu : page 341)
- éq. (434) : numéro en double, page 368 (retenu : page 341)
- éq. (435) : numéro en double, page 368 (retenu : page 341)
- éq. (436) : numéro en double, page 368 (retenu : page 341)
- éq. (437) : numéro en double, page 368 (retenu : page 341)
- éq. (438) : numéro en double, page 369 (retenu : page 341)
- éq. (439) : numéro en double, page 369 (retenu : page 341)
- éq. (440) : numéro en double, page 1362 (retenu : page 369)

#### Hors séquence dans l'extraction (12)

- éq. (482) : hors séquence, page 376 (retenue à sa place dans le texte)
- éq. (650) : hors séquence, page 408 (retenue à sa place dans le texte)
- éq. (1208) : hors séquence, page 741 (retenue à sa place dans le texte)
- éq. (1217) : hors séquence, page 742 (retenue à sa place dans le texte)
- éq. (1396) : hors séquence, page 911 (retenue à sa place dans le texte)
- éq. (1554) : hors séquence, page 961 (retenue à sa place dans le texte)
- éq. (2209) : hors séquence, page 1266 (retenue à sa place dans le texte)
- éq. (2683) : hors séquence, page 1498 (retenue à sa place dans le texte)
- éq. (2686) : hors séquence, page 1499 (retenue à sa place dans le texte)
- éq. (2960) : hors séquence, page 1602 (retenue à sa place dans le texte)
- éq. (3172) : hors séquence, page 1698 (retenue à sa place dans le texte)
- éq. (3257) : hors séquence, page 1727 (retenue à sa place dans le texte)

### Tableaux (8)

#### Absent de l'extraction (1)

- tableau 134 : introuvable dans le texte extrait

#### En double (4)

- tableau 10 : légende en double, pages 82 et 91
- tableau 57 : légende en double, pages 335 et 336
- tableau 115 : légende en double, pages 699 et 711
- tableau 122 : légende en double, pages 708 et 712

#### Hors séquence dans l'extraction (3)

- tableau 8 : hors séquence, page 84 (après la légende 10)
- tableau 9 : hors séquence, page 91 (après la légende 10)
- tableau 115 : hors séquence, page 711 (après la légende 122)

### Figures (10)

#### Absent de l'extraction (4)

- figure 67 : introuvable dans le texte extrait
- figure 68 : introuvable dans le texte extrait
- figure 69 : introuvable dans le texte extrait
- figure 75 : introuvable dans le texte extrait

#### En double (3)

- figure 2 : légende en double, pages 19 et 446
- figure 4 : légende en double, pages 20 et 1481
- figure 5 : légende en double, pages 21 et 1482

#### Hors séquence dans l'extraction (3)

- figure 2 : hors séquence, page 446 (après la légende 78)
- figure 4 : hors séquence, page 1481 (après la légende 219)
- figure 5 : hors séquence, page 1482 (après la légende 219)

## annexe II

### Équations (1)

#### Absent de l'extraction (1)

- éq. (96) : introuvable dans le texte extrait (pages 82 à 83)

### Tableaux (0)

Aucune.

### Figures (0)

Aucune.
