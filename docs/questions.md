<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Questions ouvertes

Chaque entrée indique la section du texte, la nature du manque ou de l'ambiguïté, et l'hypothèse proposée. Rien n'est tranché sans réponse du mainteneur. Les références « A3 » et « A2 » désignent les annexes III et II ; les pages sont celles des PDF de `docs/reference/`.

Statuts : **ouverte**, **tranchée** (avec la décision et sa date).

## Données manquantes (bloquantes)

### Q1 — Fichiers météorologiques horaires : absents des annexes — ouverte, bloque la phase 3

- **Texte** : A3 §3.1 (p. 27–33), A2 §2.2.1.1 (p. 18).
- **Constat** : la fiche `C_EEX_Climat extérieur` prend en entrée, pour chaque heure, `Te0`, `we0`, `RdirnDn`, `Rdiff`, `Teciel`, `Vent`, `dirVent`, `Teau0`, la position du soleil, la latitude et la longitude, ainsi que les repères calendaires (`jsem`, `EHLO`, `ISEM`…). Le texte dit seulement que ces valeurs sont « issues du fichier météo ». Les huit années-types (H1a à H3) ne figurent ni dans l'annexe II ni dans l'annexe III. L'annexe III ne tabule que la température de base par département (p. 31) et les corrections d'altitude (p. 31–32).
- **Conséquence** : les tableaux du PDF ne suffisent pas pour le climat.
- **Proposition** : le mainteneur fournit les huit fichiers météo conventionnels RE2020 (format d'origine du moteur ou CSV) ; ils sont versionnés dans `crates/thbce-data/` avec leur provenance. Aucune donnée de substitution n'est utilisée.
- **Sous-question** : la table de correspondance département → zone climatique n'a pas été trouvée dans l'annexe III (la carte est une figure de l'annexe II, p. 19). À fournir avec les fichiers météo, ou à confirmer depuis l'arrêté.

### Q2 — Scénarios conventionnels horaires : fichier externe — ouverte, bloque la phase 4

- **Texte** : A3 §15 (p. 1409), §2.1.1 (p. 18–26), §4.1 (p. 52–63).
- **Constat** : le chapitre 15 tient en une ligne et renvoie à un classeur publié par la DHUP : `https://rt-re-batiment.developpement-durable.gouv.fr/IMG/xlsx/2026-04-29_scenarios_conventionnels.xlsx`. Le tableau de synthèse des pages 24 à 26 donne des plages horaires et des ratios par usage, mais pas les profils horaires par semaine et par mois (`p_occ`, `p_ch`, `t_occ`, `t_ch`…) dont la fiche 4.1 a besoin. Ce classeur n'est pas téléchargeable depuis l'environnement de développement (accès réseau refusé).
- **Proposition** : le mainteneur dépose le classeur dans `docs/reference/` ; un script versionné le convertit en données embarquées, avec un test qui recoupe le résultat avec le tableau de synthèse des pages 24 à 26.

### Q3 — Format RSEE XML : schéma non fourni — ouverte, bloque la lecture RSEE (phase 2)

- **Texte** : aucun ; le format RSEE n'est décrit dans aucune des deux annexes.
- **Proposition** : le mainteneur fournit le schéma XSD du RSEE (et sa version) ainsi qu'un ou deux fichiers d'exemple. En attendant, la phase 2 livre le format JSON natif, dont les noms de champs suivent les nomenclatures de l'annexe III, et la lecture RSEE est ajoutée dès réception du schéma.

### Q4 — Périmètre de la première version — ouverte

- **Constat** : la méthode couvre 28 usages de zone (A3 p. 24–26) et 144 fiches, dont 65 fiches de systèmes (909 pages) et 24 Titres V propres à des produits commerciaux (chapitre 16, 445 pages).
- **Proposition** :
  1. Usages : maison individuelle et logement collectif d'abord (usages 1 et 2), les 26 autres ensuite, dans l'ordre que fixeront les cas de référence disponibles.
  2. Bbio et DH (phases 3 à 5) : toutes les fiches du bâti, les espaces tampons (§5.2 à 5.4) formant un lot 4b livré après le premier Bbio validé.
  3. Systèmes (phase 6) : ordre à fixer par le mainteneur. Suggestion : effet Joule, chaudières, PAC électriques, ventilation simple et double flux, ECS à accumulation et thermodynamique, puis le reste.
  4. Titres V (chapitre 16) : hors première version.
- Tout système non implémenté produit une erreur explicite « non pris en charge », jamais un résultat silencieusement faux.

### Q5 — Ic énergie : données environnementales absentes — ouverte, bloque la fin de la phase 7

- **Texte** : A2 §4.3, éq. (74) et (75) (p. 71–72).
- **Constat** : le calcul demande `DE(i,k)`, émissions de gaz à effet de serre par kWh d'énergie `i` pour l'usage `k`, et la fonction de pondération dynamique `fCO2(a)`. Aucune des deux n'a été trouvée sous forme de tableau dans l'annexe II.
- **Proposition** : le mainteneur indique la source officielle de ces valeurs (données environnementales de service de la base INIES, tableau de `fCO2`) ; elles sont versionnées dans `thbce-data`.

### Q6 — Cas de validation — ouverte

- Les cas de référence (entrées et résultats du moteur réglementaire ou d'un logiciel évalué) restent à fournir. Sans eux, les phases 3 à 7 ne peuvent être que « implémentées », pas « validées ».

### Q7 — Annexe IV (Th-Bat 2020) — ouverte, non bloquante

- **Constat** : l'annexe III renvoie aux règles Th-Bat pour des données d'entrée, par exemple les classes d'inertie « au sens des règles Th-I » (§5.21, p. 308).
- **Proposition** : le moteur prend ces grandeurs en entrée telles que la nomenclature de l'annexe III les définit ; l'annexe IV n'est nécessaire que si l'on veut calculer ces entrées à partir d'une description plus fine. À confirmer.

## Ambiguïtés et incohérences du texte

### Q8 — Règle d'arrondi (A3 §1.3.1, p. 16) — ouverte

- **Texte** : « Les résultats de calculs, intermédiaires ou finaux, doivent être fournis arrondis avec un chiffre après la virgule. »
- **Ambiguïté** : arrondir les grandeurs intermédiaires à chaque pas de temps dégraderait le calcul ; le verbe « fournis » suggère que seule la restitution est visée.
- **Hypothèse proposée** : calcul interne en pleine précision `f64`, arrondi à une décimale à la seule restitution des résultats (valeurs non arrondies disponibles en option pour les tests).

### Q9 — Codes du mode de calcul incohérents d'une fiche à l'autre — ouverte

- **Texte** : §3.1 (p. 28) : « 0-Th-B / 4-Th-D / 5-Th-BCD » ; §5.21 (p. 308) : « 0-Th-B calcul du Bbio / 1-Th-C calcul du Cep », plage 0 à 2 ; §13.5 (p. 1400) : « Mode = 4 ou 5 ».
- **Hypothèse proposée** : une énumération Rust `ModeCalcul { ThB, ThC, ThD }` sans valeur numérique publique ; chaque condition du texte portant sur un code est traduite au cas par cas, avec la référence en commentaire. Le sens exact des codes 2, 3 et 5 reste à confirmer au fil de la lecture des fiches.

### Q10 — Bbio « mensuel », éq. (2385) (A3 §13.1.3, p. 1359) — ouverte

- **Texte** : le membre de gauche est `Bbio_pts_mois` mais le membre de droite combine les besoins **annuels** (`B_Ch_annuel`, `B_Fr_annuel`, `B_Ecl_annuel`) ; l'éq. (2386) somme ensuite ce terme sur 12 mois, ce qui multiplierait le Bbio par 12.
- **Hypothèse proposée** : coquille ; lire `Bbio_pts_mois = α1·B_Ch_mois + α2·B_Fr_mois + β·B_Ecl_mois`, avec α1 = 2, α2 = 2, β = 5 (tableau 331, p. 1358).

### Q11 — Moyenne glissante de température, éq. (1) (A3 §3.1.3, p. 32) — ouverte

- **Texte** : « Si IHJ = 1, θrm(j) = 0 ; sinon θei,moy(j) = … ; θrm(j) = 0,8·θrm(j−1) + 0,2·θei,moy(j) », alors que la phrase précédente dit que la moyenne est « calculée au premier pas de temps du jour j (h tel que IHJ = 1) et reprise pour l'ensemble des autres pas de temps ». Prise à la lettre, la condition annulerait θrm chaque jour.
- **Hypothèse proposée** : la branche « = 0 » est l'initialisation au premier jour de simulation ; les autres jours, la récurrence est évaluée à IHJ = 1 et conservée sur la journée. La valeur d'initialisation (0 °C) pèse sur les premiers jours de janvier seulement.
- **Sous-question** : la recherche du premier et du dernier jour de confort adaptatif (p. 32–33) est écrite sans boucle explicite et avec `j` de 0 à 365 ; lecture proposée : premier et dernier jour de l'année où θrm(j) ≥ 16 °C. Le texte de la p. 32 dit « supérieure à 16 °C » et l'algorithme « ≥ » ; l'algorithme est retenu.

### Q12 — Nombre de types de masques (A3 §3.2.1, p. 34) — ouverte

- **Texte** : « Six types de masques sont considérés », suivi de cinq puces (lointain par tranches azimutales, arbres à feuilles caduques, plan vertical éloigné, proches verticaux droite et/ou gauche, proche horizontal).
- **Hypothèse proposée** : les masques verticaux droit et gauche comptent pour deux types. À confirmer à la lecture détaillée de la fiche en phase 3.

### Q13 — Numéro de département (A3 §3.1.2 et §5.1.2, p. 28 et 99) — ouverte

- **Texte** : plage « 1 à 95 », alors que le tableau des températures de base (p. 31) distingue « 2A » et « 2B ».
- **Hypothèse proposée** : type `Departement` validé sur la liste 01 à 95 hors 20, plus 2A et 2B.

### Q14 — Résidus de la RT2012 — ouverte, mineure

- **Texte** : plusieurs nomenclatures parlent encore de « calcul de Tic » (par exemple §5.17, p. 289, et §5.19, p. 299 : `Type_calcul` « 0 : calcul de Cep, 1 : calcul de Tic »).
- **Hypothèse proposée** : lire « calcul de DH (mode Th-D) » partout où le texte dit « Tic », en cohérence avec les paramètres `Sf-e` définis « pour le calcul de DH » dans les mêmes tableaux.

### Q15 — Intitulés dupliqués dans les sorties de zone, éq. (2388) (A3 p. 1360) — ouverte, mineure

- **Texte** : le ratio de la surface totale des parois opaques est nommé `Aopv_surf` et calculé à partir de `Aopv`, comme dans l'éq. (2389).
- **Hypothèse proposée** : coquille ; lire `Aop_surf = Aop / SREF`.

## Remarque de méthode : fiabilité de l'extraction du PDF

L'annexe III contient une couche texte exploitable, mais avec deux limites constatées :

1. Dans les nomenclatures, les lettres grecques d'une police de symboles disparaissent à l'extraction (`θaj(h)` devient `aj(h)`, `Φs1(h)` devient `s1(h)`).
2. Les équations sont du texte mis en page en deux dimensions : fractions, indices et exposants sont aplatis, ce qui rend certaines formules ambiguës une fois extraites.

En conséquence, **chaque équation est relue sur l'image de la page avant d'être codée**, et non sur le texte extrait. Si le mainteneur dispose du document source (Word) de la méthode, il réduirait ce risque.
