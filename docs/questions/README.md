<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Questions et décisions

Chaque entrée indique la section du texte, la nature du manque ou de l'ambiguïté, et l'hypothèse proposée. Rien n'est tranché sans réponse du mainteneur. « A3 » et « A2 » désignent les annexes III et II de l'arrêté du 4 août 2021 ; les pages sont celles des PDF de `docs/reference/arrete/`.
Ce dossier contient un fichier par question (`Q<nnn>.md`) ; l'ancien fichier unique `docs/questions.md` n'est conservé que comme renvoi.

## Écrire une nouvelle question

**Un fichier par question**, dans ce dossier : `Q<nnn>.md` (trois chiffres ; `Q001a.md` pour une variante lettrée). Cela évite que deux lots qui travaillent en parallèle modifient le même fichier.

- **Numéro** : pris dans la plage réservée au lot (annoncée dans son énoncé), jamais dans celle d'un autre. Une plage réservée mais non encore utilisée se déclare dans la ligne de commentaire « plages reservees » placée sous cette liste (exemple : `60-69, 80-89`), pour que `cargo run -p xtask -- verifier` ne signale pas ses numéros comme manquants ; un lot qui n'utilise pas toute sa plage la raccourcit en fin de lot, sans trou dans ses propres numéros.
- **En-tête** : un bloc `---` en tête de fichier, trois clés, chaque valeur entre guillemets doubles (texte JSON, donc YAML valide) :

  ```
  ---
  numero: Q55
  sujet: "Intitulé court : fiches concernées, lot, liste des points"
  statut: "tranchée, à confirmer par un cas de référence"
  ---
  ```

  `sujet` et `statut` alimentent le tableau de bord ci-dessous. Le statut se met à jour dans l'en-tête, jamais dans le tableau.
- **Corps** : mêmes rubriques que `Q054.md` : titre `### Q<n> — <fiches> (A3 p. …) — <statut> le <date>`, périmètre livré, points numérotés (lecture retenue, variante conservée et son effet chiffré, à confirmer par un cas de référence), résultats chiffrés. Un complément ultérieur s'ajoute à la fin, daté ; la lecture déjà tranchée ne se réécrit pas sans nouvelle décision datée.
- **Tableau de bord** : `cargo run -p xtask -- questions` le régénère entre les marqueurs ; ne le modifiez pas à la main. `cargo run -p xtask -- verifier` échoue s'il n'est pas à jour, si un numéro manque ou est en double, ou si un en-tête est invalide.
- **Renvois** : depuis le code et les documents, renvoyer à `docs/questions/Q<nnn>.md`.

<!-- plages reservees:  -->

## Tableau de bord

Généré depuis les en-têtes des fichiers `Q*.md` (`cargo run -p xtask -- questions`).

<!-- debut tableau genere -->

| N° | Sujet | Statut |
|---|---|---|
| Q1 | Fichiers météo réglementaires | tranchée |
| Q1a | Correspondance département → zone climatique | tranchée, table extraite par script |
| Q1b | Contenu du fichier météo | tranchée, hypothèse du mainteneur |
| Q2 | Scénarios conventionnels | tranchée |
| Q3 | Format RSEE | tranchée : JSON d'abord, lecteur XML en attente de fichiers d'exemple |
| Q4 | Périmètre de la première version | tranchée |
| Q5 | Ic énergie | tranchée, données complètes |
| Q6 | Cas de validation | tranchée : harnais prêt, **cas à venir** |
| Q7 | Règles Th-Bat | tranchée |
| Q8 | Règle d'arrondi | tranchée, hypothèse du mainteneur |
| Q9 | Codes du mode de calcul | tranchée, hypothèse du mainteneur |
| Q10 | Bbio mensuel, éq. (2385) | tranchée, hypothèse du mainteneur |
| Q11 | Moyenne glissante, éq. (1) | tranchée, hypothèse du mainteneur |
| Q12 | Nombre de types de masques | tranchée, confirmée par le texte |
| Q13 | Numéro de département | tranchée, hypothèse du mainteneur |
| Q14 | Résidus « Tic » | tranchée, hypothèse du mainteneur |
| Q15 | Intitulé de l'éq. (2388) | tranchée, hypothèse du mainteneur |
| Q16 | Fichiers ADEME | tranchée : règles de lecture fixées, lecteur au lot 3 |
| Q17 | 365e jour de l'année | tranchée, hypothèse du mainteneur (la fiche 4.1, lue en entier, n'en dit pas plus) |
| Q18 | Lots de systèmes | tranchée |
| Q19 | Rayonnement direct non nul quand `Gamma` vaut zéro | tranchée par le texte |
| Q20 | Unité de γ dans le polynôme de l'éclairement direct | tranchée, hypothèse du mainteneur : degrés |
| Q21 | `dhg` non défini dans l'éq. (20) | tranchée, hypothèse du mainteneur : `dhp` |
| Q22 | Dernière ligne du tableau du calendrier | tranchée, hypothèse du mainteneur |
| Q23 | Détails de la fiche 3.2 : `Tsimul`, égalité dans l'éq. (22), repli de Φ, paramètre `Az` | tranchée, hypothèse du mainteneur |
| Q24 | Masques proches des ponts thermiques | tranchée par le texte, implémentée au lot 4c |
| Q25 | **Décalage d'une heure entre le fichier météo et le calendrier du texte** | tranchée : lecture ligne à ligne, **premier paramètre à tester si un cas de référence diverge** |
| Q26 | Trois cellules du classeur des scénarios saisies « 0;5 » | tranchée, hypothèse du mainteneur, à confirmer avant de calculer ces usages ; sans effet sur la v1 |
| Q27 | Valeurs fractionnaires dans des indicateurs booléens, ratios d'aérogare | tranchée : conversion telle quelle, rien n'est normalisé ; évaluation à trancher fiche par fiche |
| Q28 | Fiche 4.1 : surface utile `Agr`, changement de jour des scénarios, nombre de logements d'une maison | tranchée, à confirmer par un cas de référence ; le changement de jour est le **deuxième paramètre à tester si un cas de référence diverge** |
| Q29 | Parois opaques et ponts thermiques : sens de `fce` (coquille de la nomenclature), branche 120–150° de l'éq. (294), `incl_k` obligatoire, description simplifiée des ponts thermiques | tranchée, à confirmer par un cas de référence |
| Q30 | Baies vitrées (fiches 5.8, 5.10 et 5.14) : numérotation des types de baie, `Rprot,b`, surfaces par orientation, stores vénitiens après v1, valeurs par défaut des facteurs solaires | tranchée, à confirmer par un cas de référence ; décomposition du facteur solaire par défaut en Sw1, Sw2, Sw3 : hypothèse du mainteneur (point 8) |
| Q31 | Protections mobiles (fiches 5.9 et 5.12) : matrices conventionnelles vérifiées sur les figures du JO par deux lectures indépendantes ; gestion automatique hors de la première version ; lectures du tableau 34 et de l'éq. (175) | tranchée ; changement de jour de l'éq. (175) fixé à minuit légal (Q39 point 1) |
| Q32 | Surventilation naturelle (fiche 5.13) : hystérésis de l'ouverture selon le sens de variation de la température, ratio conservé la nuit ; coquille de l'éq. (253) | tranchée, à confirmer par un cas de référence ; l'hystérésis est, avec Q25 et Q28, **l'un des premiers paramètres à tester si le DH diverge** (variante « bande à mémoire » conservée dans le code) |
| Q33 | Éclairage (fiche 7.1) : périmètre limité à l'habitation ; `CECL` en kWh et apports en W ; part des baies sous le plan de travail annoncée mais jamais utilisée | tranchée, à confirmer par un cas de référence |
| Q34 | Modèle thermique de groupe (fiche 5.21) : part des apports radiatifs et solaires des parois légères injectée au nœud θs (lacune du texte) ; termes entre groupes nuls ; température de masse moyenne | tranchée, à confirmer par un cas de référence ; **premier paramètre à tester si le Bbio ou le DH diverge**, devant Q25, Q28 et Q32 (variante littérale conservée dans le code) |
| Q35 | Saisons (fiche 4.6) : fenêtres glissantes, besoins en Wh, coquilles des éq. (73), (76), (77), (80) et (88) ; en Bbio, autorisation de refroidir sans égard pour `iclim` (éq. 91) ; source des débits d'air du Bbio trouvée (fiches 6.2 et 5.6) | tranchée, à confirmer par un cas de référence |
| Q36 | Débits d'air spécifiques du Bbio (fiches 6.1 et 6.2) : en Bbio, le système conventionnel souffle autant qu'il extrait quand un seul sens est saisi ; renvoi erroné du §6.2.3.1.3 ; `Dugd` du débit de base hygroréglable | tranchée, à confirmer par un cas de référence ; le débit soufflé (point 1) reste un **paramètre à tester si le Bbio diverge**, après Q34, Q37 et Q38 point 3 et devant Q25, Q28 et Q32 (variante littérale conservée dans le code) |
| Q37 | Bilan aéraulique de la zone (fiche 5.6) : application en Bbio et entrées d'air ; origine des altitudes ; hauteur moyenne et coefficients de pression ; sens du débit échangé avec le hall ; masses volumiques des débits ; perméabilité obligatoire | tranchée, à confirmer par un cas de référence ; les entrées d'air en Bbio (point 1) sont **sensibles** (7 points) : **deuxième paramètre à tester si le Bbio diverge**, juste après Q34 ; l'origine des altitudes (point 3) est à revalider pour le logement collectif à plusieurs groupes |
| Q38 | Indicateurs de confort, émission et bilan hydrique du Bbio (fiches 4.5, 8.1 et 8.3) : facteur des écarts d'inconfort chaud (éq. 61) ; `φcrois` en W ; `iclim` en Bbio (§8.1.3.1 et éq. 807, contre la lettre du texte) ; signe de `Qsys_fr` ; catégorie d'ambiance ; volume du groupe obligatoire | tranchée, à confirmer par un cas de référence ; `iclim` en Bbio (point 3) rejoint les **paramètres à tester si le Bbio diverge** |
| Q39 | Enchaînement horaire du Bbio (fiches 5.5, 5.7 et 13.1) : changement de jour (éq. 175 et scénarios à minuit légal, `θrm` au jour du fichier météo, saisons après le pas de 9 h légales) ; hall fictif ; état initial ; mois conventionnels ; sommation des énergies ; zone étanche refusée ; sensibilités mesurées | tranchée, à confirmer par un cas de référence |
| Q40 | Confort d'été, calcul Th-D (fiches 13.5, 5.21 §10, 4.6, 5.5 et 5.7) : assemblage du calcul Th-D (ventilation et émission conventionnelles du Bbio, faute des systèmes du projet) ; saison de refroidissement en Th-D ; éq. (2544) à (2546) ; inerties séquentielle et annuelle (figure 66, sommes, initialisations, cas dégénérés) ; DH de la zone et du bâtiment ; forfait de refroidissement reporté au lot 7 | tranchée, à confirmer par un cas de référence ; le point 1 est **remplacé au lot 6a** pour la ventilation (Q42 point 1) ; l'émission reste conventionnelle jusqu'au lot 6b |
| Q41 | Espaces tampons (fiches 5.2, 5.3 et 5.4) : `Hges_set` des baies, ouverture des baies donnant sur une véranda, `Ventc_lim`, branche de fermeture de l'éq. (104), `Partouv_et_inocc`, `btherm` quand `θi_eq = θext`, `HTH,k+pt` négatif, air neuf des entrées d'air par un espace tampon, état initial, modes Th-B et Th-D, perméabilité des parois sur espace tampon | tranchée, à confirmer par un cas de référence ; `Ventc_lim` (point 3) : hypothèse du mainteneur ; l'air neuf des entrées d'air par une véranda (point 8) rejoint les **paramètres à tester si le Bbio diverge** |
| Q42 | Ventilation du projet (fiches 6.2 à 6.5, 6.3 en double flux) : assemblage du calcul Th-D avec la ventilation du projet ; équations du double flux en habitation ; surfaces de conduits du double flux en collectif ; pression du système basse pression ; fuites hors volume chauffé ; répartition des consommations ; efficacité « ramenée » ; état de la centrale (températures et saison) pour plusieurs groupes ; saison de l'antigel et du bypass ; état initial ; température soufflée d'un groupe à plusieurs bouches | tranchée, à confirmer par un cas de référence ; la ventilation du projet en Th-D (point 1) est dans la liste des **paramètres à tester si le DH diverge** |
| Q43 | Systèmes de chauffage et de refroidissement (fiches 8.1 à 8.10, lot 6b) : assemblage du calcul Th-D avec les émetteurs du projet ; calendrier des saisons effectives ; besoins des saisons propres en Th-C ; relances ; ventilateurs locaux ; déshumidification (coquilles de l'éq. 835) ; cohérence d'`iclim` ; générations idéales | tranchée, à confirmer par un cas de référence (lectures validées par le mainteneur le 16 novembre 2026) ; générations idéales remplacées au lot 6c (Q46) |
| Q44 | Distributions de chauffage et de refroidissement du groupe et intergroupes (fiches 8.7 à 8.10, 8.13) : deux versions de l'algorithme intergroupes (éq. 905 et 907) ; énergie horaire lue comme une puissance ; `btherm` des parties hors volume chauffé ; réseau individuel sans branche intergroupes | tranchée, à confirmer par un cas de référence ; le point 1 est **à mesurer** quand un cas de logement collectif avec réseau intergroupes existera |
| Q45 | ECS et pertes récupérables (fiches 9.5 à 9.8 et 11.1) : pertes récupérables en Th-D ; température de la distribution d'ECS du groupe (48 ou 53 °C) ; longueur par défaut et nombre de distributions ; période de fonctionnement de la génération ; calendrier de la récupération | tranchée, à confirmer par un cas de référence ; la récupération en Th-D (point 1) est le 7e **paramètre à tester si le DH diverge** ; `θ2nd-e` = 53 °C validée (effet de 48 °C consigné au point 2) |
| Q46 | Génération (fiches 8.14, 8.17, 8.18 et 8.29, lot 6c) : génération de la sécurité antigel du double flux ; ordre des postes ; relance en cascade ; `Rdim` de l'effet Joule ; températures extrêmes et de fonctionnement des réseaux hydrauliques ; appel des générateurs d'ECS ; reports hors saison ; auxiliaires dans les postes ; alternance et `θwm_ecs` | tranchée, à confirmer par un cas de référence (lectures validées par le mainteneur le 16 novembre 2026) |
| Q47 | Générateurs thermodynamiques électriques et sources amont sur air (fiches 8.23 et 8.26, lot 6c) : technologies prises en charge ; coquilles des éq. (1055), (1259), (1265) et (1321) et de la valeur pivot des PAC sur air ambiant ; pas de temps partagé avec l'ECS en double service ; puissance nominale ; saisie des valeurs mesurées ; auxiliaires à charge nulle ; limites de fonctionnement ; dégivrage | tranchée, à confirmer par un cas de référence (lectures validées par le mainteneur le 16 novembre 2026) ; point 4 (pas de temps partagé) dans la liste des paramètres à tester des consommations |
| Q48 | ECS avec stockage et chauffe-eau thermodynamiques (fiches 9.2, 9.9, 9.10, 9.13, 9.14 et 9.22, lot 6c) : enclenchement du générateur de base (éq. 1814) ; échangeur du générateur ; appoint intégré ; marge entre la consigne de 55 °C et la distribution à 53 °C, programmation de nuit ; appel à charge nulle ; identification IdCET (redémarrages, seuil ε, étapes D et E, besoins de l'essai) ; `Φrejet` dans les espaces tampons ; assemblages hors de la version | tranchée, à confirmer par un cas de référence (lectures validées par le mainteneur le 16 novembre 2026) ; points 1 et 4 dans la liste des paramètres à tester des consommations, point 8 du DH |
| Q49 | Chaudières gaz, fioul et bois, poêles et inserts, réseaux de chaleur et de froid (fiches 8.19, 8.22 et 8.28, lot 6d) : facteurs 100 et 1000 des éq. (1072) à (1074), (1228) et (1473) ; logarithme ; chaudière basse température ; puissance intermédiaire ; auxiliaires des plages de modulation (`Waux_min`) ; veille comptée deux fois ; consommation à l'arrêt ; poêle en volume chauffé ; données de l'annexe 7 | tranchée, à confirmer par un cas de référence (lectures validées par le mainteneur le 17 novembre 2026) |
| Q50 | Postes du logement collectif et des parkings (fiches 7.2, 10.1, 10.3 et 10.4, lot 6e) : constantes du tableau 309 contredites par §10.1.3.2 ; gravité de la phase d'accélération (éq. 2299 et 2300) ; durées des éq. (2303) ; `Cp` ; `ScVeille` ; `NBz` ; éq. (791) à (794) ; plages horaires des parkings ; `IsParamVentilationDefaut` ; apports internes | tranchée, à confirmer par un cas de référence (lectures validées par le mainteneur le 17 novembre 2026) |
| Q51 | Production photovoltaïque (fiches 12.1 à 12.4, lot 7a) : `Pc` de calcul dans `ηSTC` et `PAC NOM` ; éq. (2372) (`Max(0,1 + …)` et `110⁻⁴`) ; extinction `LIM` ; extrapolation de `ηOND` ; jeu météo et grandeurs climatiques ; répartition autoconsommé/exporté et PV de parcelle (chapitre 13) | tranchée, à confirmer par un cas de référence (lectures validées par le mainteneur le 17 novembre 2026) |
| Q52 | Solaire thermique (fiches 9.14, 9.15, 9.18 à 9.20 et 9.23, lot 7b) : `Kθ` d'un kit CESI ; `Kθ` sur le rayonnement global ; racine de l'éq. (1966) ; pertes de la boucle pompe arrêtée et selon l'emplacement ; renvoi erroné des éq. (1970) et (1971) ; énergie écrêtée à `θmax` ; appel de la boucle ; `Qsol` négatif ; appoint séparé instantané ; systèmes solaires combinés (éq. 2123, `θch_max`, appoint raccordé) | tranchée, à confirmer par un cas de référence (lectures validées par le mainteneur le 17 novembre 2026, après relectures indépendantes) |
| Q53 | Productions solaires collectives à appoints décentralisés (fiches 9.16 et 9.17, lot 7c) : intégration dans la génération ; demande des ensembles de ballons ; ambiance (éq. 1978) ; eau entrant dans les ballons d'un CESCAI (éq. 2073) ; réseau intergroupe au départ du ballon central et pertes payées par lui (éq. 2076 et 2080, corrigé à la relecture) ; transfert limité aux ballons plus froids (éq. 2078) ; température vue par les boucles | tranchée, à confirmer par un cas de référence (lectures validées par le mainteneur le 17 novembre 2026, après relectures indépendantes) |
| Q54 | Indicateurs réglementaires et exigences (fiches 13.2 à 13.4 et 4.7, articles 9 à 11, annexe II §4.3.2, annexe de l'article R. 172-4, lot 7) : (1) relevé horaire de l'électricité par poste ; (2) autoconsommation, mobilier compris ; (3) pondération du forfait (éq. 2496) ; (4) seuil haut du forfait ; (5) classes d'altitude, différentes entre le tableau 336 et l'annexe ; (6) usages de l'électricité dans l'Ic énergie, auxiliaires de générateur dans leur poste ; (7) années de la pondération dynamique ; (8) sous-décomposition par générateur, combustible du poêle obligatoire ; (9) DH vérifié par groupe ; (10) exigences d'un bâtiment à plusieurs zones ; (11) conformité, indicateurs manquants, dérogations à l'année ; (12) usages autres que 1 et 2 ; (13) productions solaires collectives ; (14) catégorie de contraintes extérieures contrôlée selon le chapitre V, catégorie et zone de bruit saisies ; (15) non fait | tranchée, à confirmer par un cas de référence (lectures validées par le mainteneur le 17 novembre 2026, après relectures indépendantes) |

<!-- fin tableau genere -->

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
