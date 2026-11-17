<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Questions et décisions

Chaque entrée indique la section du texte, la nature du manque ou de l'ambiguïté, et l'hypothèse proposée. Rien n'est tranché sans réponse du mainteneur. « A3 » et « A2 » désignent les annexes III et II de l'arrêté du 4 août 2021 ; les pages sont celles des PDF de `docs/reference/arrete/`.
Ce dossier contient un fichier par question (`Q<nnn>.md`) ; l'ancien fichier unique `docs/questions.md` n'est conservé que comme renvoi.

## Écrire une nouvelle question

**Un fichier par question**, dans ce dossier : `Q<nnn>.md` (trois chiffres ; `Q001a.md` pour une variante lettrée). Cela évite que deux lots qui travaillent en parallèle modifient le même fichier.

- **Numéro** : pris dans la plage attribuée au lot par l'orchestrateur, jamais dans celle d'un autre. Les trous dans la numérotation sont permis ; un numéro déjà utilisé est refusé par `cargo run -p xtask -- verifier`.
- **En-tête** : un bloc `---` en tête de fichier, trois clés, chaque valeur entre guillemets doubles (texte JSON, donc YAML valide) :

  ```
  ---
  numero: Q55
  sujet: "Intitulé court : fiches concernées, lot, liste des points"
  statut: "tranchée, à confirmer par un cas de référence"
  ---
  ```

  `sujet` et `statut` alimentent le tableau de bord `ETAT.md`. Le statut se met à jour dans l'en-tête, jamais dans le tableau.
- **Corps** : mêmes rubriques que `Q054.md` : titre `### Q<n> — <fiches> (A3 p. …) — <statut> le <date>`, périmètre livré, points numérotés (lecture retenue, variante conservée et son effet chiffré, à confirmer par un cas de référence), résultats chiffrés. Un complément ultérieur s'ajoute à la fin, daté ; la lecture déjà tranchée ne se réécrit pas sans nouvelle décision datée.
- **Tableau de bord** : `ETAT.md` est un fichier généré ; ne le modifiez pas à la main. `cargo run -p xtask -- questions` le régénère ; `cargo run -p xtask -- verifier` échoue s'il n'est pas à jour, si un numéro est en double, si un en-tête est invalide ou si un nom de fichier ne suit pas son numéro. **En cas de conflit sur `ETAT.md` lors d'une intégration de `main`, garder n'importe quelle version puis relancer `cargo run -p xtask -- questions`.**
- **Renvois** : depuis le code et les documents, renvoyer à `docs/questions/Q<nnn>.md`.

## Tableau de bord

Le tableau de bord est dans [`ETAT.md`](ETAT.md), généré depuis les en-têtes des fichiers `Q*.md`. Ce README n'est pas modifié par les lots.

« Hypothèse du mainteneur » signifie : lecture retenue le 16 novembre 2026, **à confirmer par un cas de validation**. Ce sont les premiers points à revoir si un cas de référence diverge.

### Paramètres à tester en premier si un cas de référence diverge

#### Bbio

Ordre validé par le mainteneur (décision du 16 novembre 2026, proposée au lot 5). Dans cet ordre, pour un écart sur le Bbio (effets mesurés sur la maison d'exemple, voir le tableau « Sensibilités mesurées du Bbio ») :

1. **Q34, point 1** : part des apports radiatifs et solaires des parois légères (`LectureFluxRadiatifs`) ; effet le plus fort (+9,9 points).
1 bis. **Q34, point 6** (relecture indépendante du lot 4d) : terme des baies `Hges·θes` rétabli dans l'éq. (366) ; lecture littérale : 67,0 points au lieu de 59,7 (+7,3).
2. **Q37, point 1** : entrées d'air prises en compte en Bbio (les retirer de la saisie pour tester) ; −7,3 points.
3. **Q41, point 8** : air neuf des entrées d'air transitant par une véranda à la température de l'espace tampon (`Tair_out = θet`) ; effet mesuré sur la variante de l'exemple : 6,6 points (53,2 contre 59,8 points avec la véranda seule ; décision du mainteneur, lot 6a).
4. **Q38, point 3** : `iclim` en Bbio, tout groupe étant traité comme refroidi (`saisons::refroidissement_possible`) ; −5,7 points avec la lecture littérale.
5. **Q36, point 1** : débit soufflé du système de ventilation conventionnel quand un seul sens est saisi (`LectureDebitsBbio`) ; +0,9 point.
6. **Q25** : lecture du fichier météo ligne à ligne (décaler d'une ligne).
7. **Q28** : changement de jour des scénarios à minuit légal.
8. **Q32** : hystérésis de l'ouverture des baies (`LectureHysteresis`), qui pèse d'abord sur le DH et sur le besoin de froid.

#### DH

Liste distincte de celle du Bbio (décision du mainteneur du 16 novembre 2026). Dans cet ordre, pour un écart sur le DH (effets mesurés au lot 5 sur la maison d'exemple, H2b, météo réglementaire, DH de référence 1 231,1 °C.h ; voir le tableau « Sensibilités mesurées du DH ») :

1. **Q32, point 2** : comportement de nuit de l'ouverture des baies ; baies refermées la nuit : 3 235,7 °C.h (+2 004,6), de loin le plus sensible ; profil d'ouverture appliqué à toute heure d'occupation : 1 257,7 °C.h (+26,6).
1 bis. **Q34, point 6** (relecture indépendante du lot 4d) : terme des baies `Hges·θes` rétabli dans l'éq. (366) ; lecture littérale : 584,9 °C.h au lieu de 992,7 (−407,8), sur la maison d'exemple actuelle.
2. **Q34, point 1** : part des apports radiatifs et solaires des parois légères ; lecture littérale : 1 013,8 °C.h (−217,3).
3. **Q25** : lecture du fichier météo ligne à ligne ; fichier décalé d'une ligne : 1 143,1 °C.h (−88,0).
4. **Q42, point 1** (remplace Q40 point 1 au lot 6a ; décision du mainteneur du 16 novembre 2026) : ventilation du projet dans le calcul Th-D ; effet de +24 à +63 °C.h selon le système ; l'assemblage provisoire du lot 5 (ventilation conventionnelle du Bbio) donne 993,3 °C.h au lieu de 968,8 (+24,4) sur la maison d'exemple à simple flux hygroréglable, 994,1 au lieu de 930,9 (+63,2) sur la variante à double flux. Mesure du lot 5, sur l'ancien exemple : débits des bouches tels quels, 1 207,5 °C.h (−23,6).
5. **Q40, point 2** : saison de refroidissement en Th-D ; `Autfr,pro` nulle sans système : 1 250,8 °C.h (+19,7).
6. **Q32, point 1** : hystérésis de l'ouverture des baies ; bande à mémoire : 1 226,1 °C.h (−5,0).

7. **Q45, point 1** (décision du mainteneur du 16 novembre 2026, lot 6c) : récupération des pertes des systèmes (ECS surtout) dans le calcul Th-D ; sans elle : 968,8 °C.h au lieu de 992,7 (−23,9) sur la maison d'exemple, 811,3 au lieu de 830,0 (−18,7) sur la variante véranda.

8. **Q48** (validé par le mainteneur, lot 6d) : pertes d'un ballon d'ECS placé en volume chauffé, récupérées à 60 % (éq. 1961 et 2362) ; avec le ballon électrique de 200 L de la maison d'exemple, DH 1 042,9 °C.h au lieu de 992,7 (+50,2) ; l'emplacement de la génération (`position`) est donc sensible.

9. **Q52, point 4** (décision du mainteneur du 17 novembre 2026, relecture du lot 7b) : pertes de la boucle solaire vers l'intérieur (éq. 1973) comptées pompe en marche (retenu) ou à toutes les heures (lecture littérale, `LecturePertesBoucle::ToutesHeures`) ; sur `maison-cesi-ademe`, pertes 45,1 → 127,7 kWh et DH 1 066,4 → 1 091,0 °C.h (+24,6).

Q11 (moyenne glissante de la température extérieure) et Q28 (sans effet en maison) viennent ensuite.

#### Consommations et générateurs

Liste établie au lot 6d (décision du mainteneur du 16 novembre 2026, complétée le 17 novembre 2026), pour un écart sur les consommations de chauffage, de refroidissement ou d'ECS d'un cas de référence :

1. **Q47, point 4** : pas de temps partagé entre le chauffage et l'ECS d'une PAC double service ; électricité de chauffage 1 009,2 kWh à la lettre, 911,0 kWh avec la pondération temporelle (−9,7 %).
2. **Q48, point 4** : marge entre la consigne du ballon (55 °C) et la distribution d'ECS (53 °C, ou 48 °C) ; programmation de nuit : 2 001,2 kWh au lieu de 2 123,3 kWh d'électricité d'ECS, avec report de la demande.
3. **Q48, point 1** : enclenchement du générateur de base du ballon (éq. 1814).
4. **Q50, point 1** (décision du mainteneur du 17 novembre 2026, lot 6e) : constantes du tableau 309 contre §10.1.3.2 ; `Pveilleporte` de 13 W (retenu) contre 75 W : +22,6 % sur la consommation de l'ascenseur de l'exemple (`Mpass` 75 contre 120 kg : +4,7 %).
5. **Q49, point 1 (d)** (décision du mainteneur du 17 novembre 2026, lot 6d) : pertes de la sous-station d'un réseau de chaleur, lecture de l'éq. (1473) (`PEss` en kW à la lettre) ; 261 kWh sur 3 254,8 kWh de chauffage (8 %) sur l'exemple, dix fois plus avec `PEss` converti en W.
6. **Q49, point 5 (c)** (décision du mainteneur du 17 novembre 2026, lot 6d) : rendement saisonnier d'ECS des chaudières, porté par les pertes à l'arrêt `φthstab` comptées pour la part du pas de temps à l'arrêt (éq. 1092 et 1095) : 0,79 sur l'exemple, contre 0,92 en chauffage.
7. **Q53, points 4 et 5** (lot 7c) : eau entrant dans les ballons individuels d'un CESCAI (eau froide retenue, `θmoy_prim-e` à la lettre de l'éq. 2073 : électricité d'ECS 16 644,6 → 20 941,4 kWh sur la variante CESCAI de `immeuble-cesci-ademe`) et réseau intergroupe au départ de la zone haute du ballon central, pertes bornées à zéro et toutes payées par le ballon central.

### Sensibilités mesurées du DH

Maison d'exemple, zone H2b, météo réglementaire, jeu « Th-D », lot 5 ; une lecture basculée à la fois. La colonne « baies triplées » porte sur la même maison avec trois fois plus de surface vitrée. Mesures reproductibles par `cargo test -p thbce-io --test confort_ete -- --ignored --nocapture sensibilites` pour les lectures portées par `assemblage_groupe::Lectures` ; les autres ont été mesurées en modifiant le code, puis rétablies.

| Lecture | DH (°C.h) | θop max (°C) | DH, baies triplées |
|---|---|---|---|
| Référence (lectures retenues) | 1 231,1 | 35,85 | 898,1 |
| Q32 point 1, hystérésis en bande à mémoire | 1 226,1 | 35,85 | 898,2 |
| Q32 point 2, profil d'ouverture appliqué à toute heure d'occupation | 1 257,7 | 35,85 | 960,9 |
| Q32 point 2, baies refermées la nuit | 3 235,7 | 38,83 | 2 700,4 |
| Q25, fichier météo décalé d'une ligne | 1 143,1 | 35,77 | 801,1 |
| Q28, jour des scénarios changeant à minuit UTC | 1 231,1 | 35,85 | 898,1 |
| Q34 point 1, lecture littérale (part des parois légères perdue) | 1 013,8 | 35,05 | 728,7 |
| Q40 point 1, débits des bouches tels quels (extraction seule, sans échangeur) | 1 207,5 | 35,81 | 893,3 |
| Q40 point 2, `Autfr,pro` nulle sans système (protections en mi-saison l'été) | 1 250,8 | 35,93 | 948,3 |
| Q40 point 5, température opérative moyenne du pas de temps au lieu de la température finale | 1 231,2 | 35,75 | 886,7 |
| Inerties séquentielle (lourde, `Ams_surf` = 1,5) et annuelle (2 000 kJ/(K.m²), `Ama_surf` = 1) au lieu des classes par défaut | 499,9 | 32,21 | 468,2 |

**Facteurs solaires des parois opaques corrigés au lot 4b.** Les mesures ci-dessus portent sur la maison d'exemple du lot 5, dont les parois opaques avaient `Sf = 0,01`. Le lot 4b leur donne les valeurs de la formule 259 des règles Th-Bat (`Sp = αp·Up·Rse`, `αp = 0,6`, voir `tests/cas/maison-climat-ademe/cas.toml`) : murs 0,0048 (C) et 0,00835 (E), toiture 0,00288 (C) et 0,00502 (E). Nouvelle référence, H2b, météo réglementaire : **DH = 989,5 °C.h** (θop max 34,95 °C ; 778,4 °C.h avec les baies triplées). Les écarts relatifs du tableau restent l'ordre de grandeur à retenir ; avec les nouveaux facteurs, mesurés par le même test : hystérésis en bande à mémoire 983,7 ; Q25 décalé 916,8 ; Q40 point 1, 974,2 ; Q40 point 2, 1 009,3 °C.h. **Lot 6a** (VMC hygroréglable, ventilation du projet en Th-D, Q42) : DH = 968,8 °C.h ; mesurés par le même test : hystérésis en bande à mémoire 963,9 ; Q25 décalé 900,1 ; Q40 point 2, 988,5 ; assemblage provisoire du lot 5 (Q42 point 1), 993,3 ; inerties longues 851,1 °C.h.

Q28 est sans effet en maison : entre minuit UTC et minuit légal, les cases 1 et 2 des scénarios d'habitation ont les mêmes valeurs tous les jours de la semaine.

### Sensibilités mesurées du Bbio

Maison d'exemple (`tests/cas/maison-climat-ademe`), zone H2b, météo réglementaire, lot 4d2 ; une lecture basculée à la fois. **Les écarts de ce tableau ont été mesurés avec l'ancien exemple** (facteurs solaires des parois opaques `Sf = 0,01`) ; la référence actuelle de la maison d'exemple, H2b, météo réglementaire, est **63,1 points de Bbio et 989,5 °C.h de DH** (lot 4b).

| Lecture | Bbio (points) | Écart |
|---|---|---|
| Référence (lectures retenues) | 60,5 | — |
| Q34 point 1, lecture littérale (part des parois légères perdue) | 70,4 | +9,9 |
| Q37 point 1, sans les entrées d'air | 53,2 | −7,3 |
| Q38 point 3, lecture littérale (pas de besoin de froid sans climatisation) | 54,8 | −5,7 |
| Q36 point 1, lecture littérale (pas de débit soufflé) | 61,4 | +0,9 |
| Q38 point 1, lecture littérale (écarts d'inconfort chaud divisés par deux) | 60,6 | +0,1 |

Après correction des facteurs solaires des parois opaques de l'exemple (lot 4b, formule 259 des règles Th-Bat) : **Bbio = 63,1 points** (chauffage 25,0 kWh/m², froid 1,3 kWh/m², éclairage 2,1 kWh/m²). Les écarts du tableau ont été mesurés avec l'ancien exemple. **Lot 6a** : la maison d'exemple est équipée d'une VMC simple flux hygroréglable (débits d'hygiène de 92 m³/h au lieu de 111,25) ; nouvelle référence, H2b, météo réglementaire : **Bbio = 59,7 points, DH = 968,8 °C.h** (Q42).

## Remarque de méthode : fiabilité de l'extraction du PDF

L'annexe III contient une couche texte exploitable, avec deux limites constatées :

1. Dans les nomenclatures, les lettres grecques d'une police de symboles disparaissent à l'extraction (`θaj(h)` devient `aj(h)`, `Φs1(h)` devient `s1(h)`).
2. Les équations sont du texte mis en page en deux dimensions : fractions, indices et exposants sont aplatis.

En conséquence, **chaque équation est relue sur l'image de la page du PDF avant d'être codée**. Le PDF fait foi ; les extractions de `docs/reference/texte/` ne servent qu'à la recherche.
