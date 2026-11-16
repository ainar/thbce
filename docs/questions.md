<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Questions et décisions

Chaque entrée indique la section du texte, la nature du manque ou de l'ambiguïté, et l'hypothèse proposée. Rien n'est tranché sans réponse du mainteneur. « A3 » et « A2 » désignent les annexes III et II de l'arrêté du 4 août 2021 ; les pages sont celles des PDF de `docs/reference/arrete/`.

## Tableau de bord

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
| Q12 | Nombre de types de masques | tranchée, **à vérifier à la lecture de la fiche 3.2** |
| Q13 | Numéro de département | tranchée, hypothèse du mainteneur |
| Q14 | Résidus « Tic » | tranchée, hypothèse du mainteneur |
| Q15 | Intitulé de l'éq. (2388) | tranchée, hypothèse du mainteneur |
| Q16 | Fichiers ADEME | tranchée : règles de lecture fixées, lecteur au lot 3 |
| Q17 | 365e jour de l'année | tranchée, **à vérifier à la lecture de la fiche 4.1** |
| Q18 | Lots de systèmes | tranchée |
| Q19 | Rayonnement direct non nul quand `Gamma` vaut zéro | **ouverte**, à traiter au lot 3 |

« Hypothèse du mainteneur » signifie : lecture retenue le 16 novembre 2026, **à confirmer par un cas de validation**. Ce sont les premiers points à revoir si un cas de référence diverge.

## Décisions

### Q1 — Fichiers météorologiques réglementaires — tranchée le 16 novembre 2026

- Les fichiers réglementaires **n'entrent jamais dans le dépôt** : leurs conditions d'utilisation limitent leur usage au calcul réglementaire RE2020 et interdisent toute diffusion, même partielle. Le `.gitignore` les exclut. C'est une exception à la règle « tout dans le dépôt ».
- Le moteur charge la météo depuis un chemin fourni par l'utilisateur, sous deux formats : le classeur d'origine (16 onglets `<zone>_<mode>`) et le CSV long dérivé (`zone, mode, h, te0, we0, dirN_W_m2, diff_W_m2, Teciel_C, Vent_m_s, Teau_C, Gamma, Psi`).
- Provenance : un fichier au format réglementaire est tenu pour réglementaire, sauf s'il porte une ligne de commentaire `# thbce-origine: <description>`, réservée aux jeux d'essai et aux données retravaillées. Le moteur ne peut pas vérifier l'authenticité d'un fichier.
- Rien n'est embarqué dans le binaire ni dans le module WASM ; pas de feature `embedded-data` pour la météo. En WASM, l'appelant passe le contenu du fichier.
- Deux jeux par zone : `Th-BC` (besoins et consommations) et `Th-D` (confort d'été, avec séquence caniculaire).
- Tests dépendant de la météo réglementaire : ignorés par défaut, lancés en local avec `THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv`.
- CI publique : fichiers ouverts ADEME / Météo-France de `docs/reference/meteo/ademe/`, lus par un lecteur dédié vers la même structure interne. Tout résultat obtenu avec eux est marqué non réglementaire dans la sortie.
- Stations : découpage de la RT2012, sauf H2b (Tours) et H3 (Marignane).
- **Contrôle fait sur le CSV reçu** : 8 zones × 2 modes × 8 760 lignes, `h` de 0 à 8759, aucune valeur non numérique.
- **Classes d'altitude** : trouvées dans A3 §3.1.3 (p. 31–32). Altitude ≤ 400 m : site considéré à 100 m ; de 400 à 800 m inclus : 500 m ; au-delà de 800 m : 900 m. La correction s'applique à la température de base, à `Te`, à `we` et à `Teau`.

### Q2 — Scénarios conventionnels — tranchée le 16 novembre 2026

- Source : `docs/reference/scenarios/2026-04-29_scenarios_conventionnels.xlsx`, 28 onglets (un par usage).
- Conversion en données versionnées dans `thbce-data` par un script reproductible (`tools/`), jamais à la main. Un test recoupe le résultat avec le tableau de synthèse de A3 p. 24–26.
- **Structure constatée** (onglet MI) : pour chaque scénario de zone (mobilité, occupation, chauffage, refroidissement, ventilation, éclairage, ECS), une grille hebdomadaire 7 jours × 24 heures et une grille annuelle 5 semaines × 12 mois ; puis, par local, taux d'occupation, apports de chaleur et d'humidité avec les mêmes deux grilles. Cela correspond aux indicateurs `p(m,s)` et `p(j,h)` de A3 §4.1.2.
- **Calendrier** : il est fixé par A3 §4.1.3 (p. 56). L'année commence un lundi ; les mois comptent 4, 4, 5, 4, 5, 4, 4, 5, 4, 4, 5, 4 semaines ; les scénarios sont en heure légale ; le passage à l'heure d'été a lieu à l'itération 1896 et le retour à l'heure d'hiver à l'itération 7032. Aucune donnée de calendrier n'est donc attendue du fichier météo.

### Q3 — Format RSEE — tranchée le 16 novembre 2026

- Le XSD n'est pas public. La phase 2 livre le format JSON natif.
- **Constat après lecture de l'annexe VI** : elle décrit le contenu du RSEE en cinq chapitres (données administratives, exigences et indicateurs, autres indicateurs, détail des entrées et sorties, sensibilités) sans nommer aucun élément ni attribut XML. Elle ne permet pas d'écrire un lecteur. Le lecteur RSEE XML attend donc des fichiers d'exemple ; rien n'est écrit d'ici là, pour ne pas inventer de noms de balises.
- Le JSON natif reprend les noms des nomenclatures de l'annexe III, ce que l'annexe VI désigne par « l'intégralité des caractéristiques telles que définies dans la méthode ».

### Q4 — Périmètre de la première version — tranchée le 16 novembre 2026

- Première version : maison individuelle ou accolée, logement collectif.
- Second lot : espaces tampons. Hors première version : Titres V du chapitre 16.
- Ordre des systèmes : (1) ventilation simple flux puis double flux ; (2) émission et distribution ; (3) effet Joule et pompes à chaleur air/eau, chauffe-eau thermodynamique ; (4) chaudières gaz et bois, puis réseaux de chaleur ; (5) photovoltaïque ; (6) solaire thermique.
- Traduction en lots dans `docs/couverture.md` ; voir Q18 pour le rattachement des fiches.

### Q6 — Cas de validation — tranchée le 16 novembre 2026

- Pas de cas pour l'instant ; ils viendront de COMETH (CSTB) ou d'un logiciel évalué.
- D'ici là : tests unitaires calculés à la main d'après les formules, et tests de propriétés.
- Le format de `tests/cas/` et le harnais de comparaison avec tolérances sont livrés en phase 2 (voir `docs/modele-donnees.md`, « Cas de validation »), de sorte qu'ajouter un cas se limite à déposer un dossier.

### Q7 — Règles Th-Bat — tranchée le 16 novembre 2026

- Les règles Th-Bat 2020 du 29 avril 2026 sont dans `docs/reference/arrete/`. Elles servent à déterminer des données d'entrée (U, Ψ, classes d'inertie, facteurs solaires) ; le moteur prend ces grandeurs en entrée et ne les recalcule pas.

## Données : questions tranchées, avec l'état antérieur

### Q1a — Correspondance département → zone climatique : introuvable — tranchée le 16 novembre 2026

- **Décision** : le découpage de la RE2020 est celui de la RT2012 ; la table est l'annexe I de l'arrêté du 26 octobre 2010 (`docs/reference/arrete/arrete_26_octobre_2010_rt2012.pdf`). Elle est extraite par `cargo run -p xtask -- zones-climatiques` dans `crates/thbce-data/src/zones_climatiques/table.rs` (96 départements, 2A et 2B compris ; un test vérifie les 96 entrées et la CI vérifie que le fichier est à jour). Le moteur déduit la zone du département ; une zone saisie qui la contredit est conservée et déclenche un avertissement. Le texte ci-dessous est l'état de la question avant cette décision.
- **Recherche faite** dans `docs/reference/texte/` : articles de l'arrêté, annexes I, II, III, règles Th-Bat, annexe 7. Les zones H1a à H3 y sont citées (exigences de facteur solaire dans les articles, tableau 336 de A3, tableau 73 de Th-Bat avec les villes de référence), mais aucune table ne rattache les départements aux zones. La carte de A2 (figure 9, p. 19) est une image.
- **Conséquence** : sans cette table, le moteur ne peut pas déduire la zone du département ; la zone climatique doit être saisie.
- **Proposition** : le mainteneur ajoute à `docs/reference/` le texte officiel qui porte cette table. En attendant, `zone_climatique` est un champ d'entrée obligatoire, et `departement` ne sert qu'à la température de base (A3 p. 31).

### Q1b — Écarts entre le fichier météo et la nomenclature de A3 §3.1.2 — tranchée le 16 novembre 2026, hypothèse du mainteneur, à confirmer par un cas de validation

- **Décision** : les six hypothèses sont retenues. `dirVent` et `Hr0` ne sont pas exigés ; `Teciel_C` est en °C ; `Gamma` et `Psi` sont en degrés et convertis en radians au chargement ; `Teau_C` est `Teau0` ; `h = 0` est l'itération 0. Tests sur le fichier réglementaire (`crates/thbce-io/tests/meteo_re2020.rs`, lancés en local avec `THBCE_METEO`) : `Gamma` est nul de 23 h à 1 h UTC toute l'année ; chaque jour, `Psi` est négatif à la première heure de jour, positif à la dernière et strictement croissant entre les deux ; `|Psi|` dépasse 150° à minuit. Les trois passent sur les 8 zones et les 2 modes. Voir aussi Q19.
Le fichier réglementaire ne contient pas toutes les grandeurs que la fiche `C_EEX_Climat extérieur` déclare en paramètres :

1. **Direction du vent (`dirVent`)** : absente du fichier. Dans l'annexe III, le terme n'apparaît que dix fois, toujours dans une nomenclature ou une figure d'assemblage (§3.1, §3.2, §5.1, §5.5, §5.7, §5.8) ; aucune équation ne l'utilise. Hypothèse : grandeur transmise mais non consommée, que le moteur n'exige pas.
2. **Humidité relative (`Hr0`)** : absente ; seul le poids d'eau `we0` est fourni. `Hr0` n'apparaît qu'une fois dans l'annexe III, dans la nomenclature du §3.1. Hypothèse : non nécessaire.
3. **Température du ciel** : le fichier donne `Teciel_C` en °C ; la nomenclature liste `Teciel` et `dTeciel`, avec une sortie en K. Hypothèse : utiliser `Teciel_C` tel quel et convertir là où une équation demande des kelvins.
4. **Position du soleil** : `Gamma` (0 à 70,02) et `Psi` (−179,99 à 179,99) sont en degrés, alors que la nomenclature les donne en radians. Hypothèse : `Gamma` est la hauteur du soleil, `Psi` son azimut par rapport au sud, négatif au lever ; conversion en radians au chargement. La latitude et la longitude ne sont alors plus nécessaires.
5. **Température d'eau froide** : la colonne s'appelle `Teau_C` ; la nomenclature attend `Teau0` « à l'altitude zéro », corrigée ensuite de l'altitude. Hypothèse : `Teau_C` est `Teau0`.
6. **Indice horaire** : hypothèse, `h = 0` est l'itération 0 du tableau de A3 p. 56 (0 h à 1 h UTC).

### Q5 — Ic énergie — tranchée le 16 novembre 2026

- **Décision** : les coefficients de pondération dynamique de l'article 11 sont transcrits dans `crates/thbce-data/src/ponderation_dynamique.rs` depuis l'image du Journal officiel du 15 août 2021 (page 59, `docs/reference/arrete/jo_15-08-2021_articles_10-11.pdf`), avec trois tests : année 0 égale à 1,000 dans les deux colonnes, décroissance stricte, année 50 égale à 0,578 (cas général) et 0,880 (fluides frigorigènes). **Vérification des modifications ultérieures**, faite sur la version consolidée (`texte/arrete-4-aout-2021-articles.txt`, en vigueur au 16 novembre 2026) : l'article 11 porte « en vigueur depuis le 1er janvier 2022 » sans mention de modification ; l'article 10 a été modifié par l'arrêté du 6 avril 2022, mais ses treize valeurs sont identiques à celles du Journal officiel d'origine, seule la phrase sur les réseaux de chaleur ayant changé (renvoi à l'annexe 7 de l'arrêté du 15 septembre 2006). L'extraction des facteurs de l'article 10 et des réseaux dans `thbce-data` se fera au lot 7. Le texte ci-dessous est l'état de la question avant cette décision.
- **Méthode** : A2 §4.3.2, éq. (74) et (75).
- **Variante INIES applicable** : l'annexe II (§2.2.2.3, p. 23) impose des données conventionnelles « fixées par le ministère en charge de l'énergie », sans nommer de variante. Les valeurs sont en réalité fixées par l'**article 10 de l'arrêté**. Les fiches INIES « [RE2020] » se présentent comme « donnée conventionnelle à utiliser dans le cadre […] de la réglementation environnementale (RE2020) », et leur indicateur « Changement climatique – total » reprend exactement les valeurs de l'article 10 pour les douze énergies comparées (par exemple électricité chauffage 0,079 ; gaz naturel 0,227 ; bois bûches 0,03). **La variante à retenir est donc « RE2020 »** ; les fiches « EC » (référentiel Énergie-Carbone) et « DES » ne s'appliquent pas aux énergies de l'article 10.
- **Source retenue dans `thbce-data`** : l'article 10, avec en commentaire la fiche INIES « [RE2020] » correspondante. Un test vérifie l'égalité des deux.
- **Réseaux de chaleur** : facteur d'émission à la cinquième colonne et ratio EnR&R à la sixième colonne du tableau de l'annexe 7 de l'arrêté du 15 septembre 2006 (articles 9 et 10). Extraction par script.
- **Manque** : les coefficients de pondération dynamique. A2 §4.2.1.2 (p. 63) renvoie à l'article 11 de l'arrêté, dont le tableau est une image absente de l'extraction Légifrance (« Vous pouvez consulter l'intégralité du texte avec ses images à partir de l'extrait du Journal officiel », JORF n° 0189 du 15 août 2021). **À fournir** : l'extrait du Journal officiel contenant ce tableau.
- **Sous-question** : l'article 10 est daté « en vigueur depuis le 15 avril 2022 ». À confirmer qu'aucun arrêté postérieur n'a modifié ces valeurs (les fiches INIES sont datées du 11 août 2026 et concordent).

### Q16 — Fichiers ADEME : définition des colonnes — tranchée le 16 novembre 2026

- **Décision** : la note technique détaillée ADEME / Météo-France définit les colonnes (§3.1 et §3.2.2) : `T2m` température de l'air (°C), `T-1m` température du sol à 1 m (°C), `Hus` humidité spécifique (kg/kg d'air), `FF` vent à 10 m (m/s), `Tc` température d'émittance de l'atmosphère (°C, équivalent de `Teciel`), `Hs` hauteur du soleil (°, équivalent de `Gamma`), `Os_2` azimut du soleil de −180 à 180° (équivalent de `Psi`, signe à vérifier), `Rdn` direct normal et `Rdi` diffus horizontal (W/m²). Elle ne dit pas si l'horodatage marque le début ou la fin de l'heure et ne donne pas de température d'eau froide. **Règles du lecteur ADEME, toutes non réglementaires** : `te0 = T2m` ; `we0` calculé depuis `Hus` (conversion en kg par kg d'air sec) ; `Teau0` approché par `T-1m` ; convention horaire alignée sur celle du fichier réglementaire après vérification sur le cycle diurne de `Hs`. Le résultat porte `reglementaire = false`. Le lecteur sera écrit au lot 3. Le texte ci-dessous est l'état de la question avant cette décision.
- **Constat** : `tracc_france_2-7C_vague_de_chaleur.csv` contient 8 stations × 8 760 heures et 24 colonnes (`T2m`, `T-1m`, `Hur`, `Hus`, `RR`, `FF`, `DD`, `Tc`, `Tr`, `Hs`, `Os`, `Os_2`, `Rgh`, `Rdn`, `Rdi`, `Ne`, `Ps`…). Leur définition, leurs unités et la convention horaire ne sont ni dans le dépôt ni sur la page data.gouv.fr, qui renvoie à une note technique détaillée ADEME / Météo-France.
- **Points à établir avant d'écrire le lecteur** : la correspondance de `Tc` avec la température du ciel, celle de `Hs` et `Os` / `Os_2` avec `Gamma` et `Psi`, l'unité de `Hus`, et surtout l'absence apparente de température d'eau froide (`Teau`), nécessaire à l'ECS.
- **Proposition** : le mainteneur ajoute la note technique détaillée à `docs/reference/meteo/ademe/`. Le lecteur ADEME est écrit à partir d'elle. Si `Teau` n'existe pas dans ces fichiers, il faudra décider d'une règle de substitution, documentée comme non réglementaire.

## Ambiguïtés et incohérences du texte : lectures retenues

### Q8 — Règle d'arrondi (A3 §1.3.1, p. 16) — tranchée le 16 novembre 2026, hypothèse du mainteneur, à confirmer par un cas de validation

- **Décision** : calcul interne en pleine précision `f64`, arrondi à une décimale seulement à la restitution ; valeurs non arrondies disponibles en option (`--sans-arrondi`, `arrondir: false`). Implémenté : `conventions::arrondir_restitution`.
- **Texte** : « Les résultats de calculs, intermédiaires ou finaux, doivent être fournis arrondis avec un chiffre après la virgule. »
- **Ambiguïté** : arrondir les grandeurs intermédiaires à chaque pas de temps dégraderait le calcul ; le verbe « fournis » suggère que seule la restitution est visée.
- **Hypothèse proposée** : calcul interne en pleine précision `f64`, arrondi à une décimale à la seule restitution (valeurs non arrondies disponibles en option pour les tests).

### Q9 — Codes du mode de calcul incohérents d'une fiche à l'autre — tranchée le 16 novembre 2026, hypothèse du mainteneur, à confirmer par un cas de validation

- **Décision** : énumération `ModeCalcul { ThB, ThC, ThD }` ; chaque condition sur un code est traduite au cas par cas, avec la référence en commentaire.
- **Texte** : §3.1 (p. 28) : « 0-Th-B / 4-Th-D / 5-Th-BCD » ; §5.21 (p. 308) : « 0-Th-B calcul du Bbio / 1-Th-C calcul du Cep », plage 0 à 2 ; §13.5 (p. 1400) : « Mode = 4 ou 5 ».
- **Hypothèse proposée** : une énumération `ModeCalcul { ThB, ThC, ThD }` sans valeur numérique publique ; chaque condition du texte portant sur un code est traduite au cas par cas, avec la référence en commentaire.

### Q10 — Bbio « mensuel », éq. (2385) (A3 §13.1.3, p. 1359) — tranchée le 16 novembre 2026, hypothèse du mainteneur, à confirmer par un cas de validation

- **Décision** : coquille. Lire `Bbio_pts_mois = 2·B_Ch_mois + 2·B_Fr_mois + 5·B_Ecl_mois`, de sorte que la somme sur 12 mois redonne le Bbio annuel. Un test de cohérence entre les deux sera écrit avec la fiche 13.1 (lot 4).
- **Texte** : le membre de gauche est `Bbio_pts_mois` mais le membre de droite combine les besoins **annuels** ; l'éq. (2386) somme ensuite ce terme sur 12 mois, ce qui multiplierait le Bbio par 12.
- **Hypothèse proposée** : coquille ; lire `Bbio_pts_mois = α1·B_Ch_mois + α2·B_Fr_mois + β·B_Ecl_mois`, avec α1 = 2, α2 = 2, β = 5 (tableau 331, p. 1358).

### Q11 — Moyenne glissante de température, éq. (1) (A3 §3.1.3, p. 32) — tranchée le 16 novembre 2026, hypothèse du mainteneur, à confirmer par un cas de validation

- **Décision** : la branche `θrm = 0` initialise la récurrence au premier jour de simulation ; les autres jours, la récurrence est évaluée à IHJ = 1 et sa valeur est conservée toute la journée. Le premier et le dernier jour de confort adaptatif sont le premier et le dernier jour de l'année où θrm(j) ≥ 16 °C. Ce calcul sera isolé dans une fonction dédiée (lot 3) : c'est le premier point à revoir si les cas de validation divergent sur le DH.
- **Texte** : « Si IHJ = 1, θrm(j) = 0 ; sinon θei,moy(j) = … ; θrm(j) = 0,8·θrm(j−1) + 0,2·θei,moy(j) », alors que la phrase précédente dit que la moyenne est calculée au premier pas de temps du jour et reprise sur la journée. Prise à la lettre, la condition annulerait θrm chaque jour.
- **Hypothèse proposée** : la branche « = 0 » est l'initialisation au premier jour de simulation ; les autres jours, la récurrence est évaluée à IHJ = 1 et conservée sur la journée.
- **Sous-question** : la recherche du premier et du dernier jour de confort adaptatif (p. 32–33) est écrite sans boucle explicite ; lecture proposée : premier et dernier jour de l'année où θrm(j) ≥ 16 °C.

### Q12 — Nombre de types de masques (A3 §3.2.1, p. 34) — tranchée le 16 novembre 2026, à vérifier à la lecture détaillée de la fiche 3.2

- **Décision** : les masques verticaux droit et gauche comptent pour deux types.
- **Texte** : « Six types de masques sont considérés », suivi de cinq puces.
- **Hypothèse proposée** : les masques verticaux droit et gauche comptent pour deux types. À confirmer à la lecture détaillée de la fiche.

### Q13 — Numéro de département (A3 §3.1.2 et §5.1.2, p. 28 et 99) — tranchée le 16 novembre 2026, hypothèse du mainteneur, à confirmer par un cas de validation

- **Décision** : type `Departement` validé sur la liste 01 à 95, sans 20, plus 2A et 2B. Implémenté : `thbce_data::Departement`.
- **Texte** : plage « 1 à 95 », alors que le tableau des températures de base (p. 31) distingue « 2A » et « 2B ».
- **Hypothèse proposée** : type `Departement` validé sur la liste 01 à 95 hors 20, plus 2A et 2B.

### Q14 — Résidus de la RT2012 — tranchée le 16 novembre 2026, hypothèse du mainteneur, à confirmer par un cas de validation

- **Décision** : lire « calcul de DH (mode Th-D) ».
- **Texte** : plusieurs nomenclatures parlent encore de « calcul de Tic » (§5.17, p. 289 ; §5.19, p. 299).
- **Hypothèse proposée** : lire « calcul de DH (mode Th-D) », en cohérence avec les paramètres `Sf-e` définis « pour le calcul de DH » dans les mêmes tableaux.

### Q15 — Intitulé de l'éq. (2388) (A3 p. 1360) — tranchée le 16 novembre 2026, hypothèse du mainteneur, à confirmer par un cas de validation

- **Décision** : coquille ; lire `Aop_surf = Aop / SREF`.
- **Texte** : le ratio de la surface totale des parois opaques est nommé `Aopv_surf` et calculé à partir de `Aopv`, comme dans l'éq. (2389).
- **Hypothèse proposée** : coquille ; lire `Aop_surf = Aop / SREF`.

### Q17 — 365e jour de l'année (A3 §4.1.3, p. 56) — tranchée le 16 novembre 2026, hypothèse du mainteneur, sauf si la suite de la fiche 4.1 dit autre chose

- **Décision** : le 365e jour est un lundi ajouté à la dernière semaine de décembre.
- **Texte** : le calendrier conventionnel compte 52 semaines (4+4+5+4+5+4+4+5+4+4+5+4), soit 364 jours, pour 8 760 heures, soit 365 jours.
- **Hypothèse proposée** : le 365e jour est traité comme un lundi supplémentaire de la dernière semaine de décembre. À confirmer, ou à lire dans la suite de la fiche 4.1 si elle le précise.

### Q18 — Rattachement des fiches aux lots de systèmes — tranchée le 16 novembre 2026

- **Décision** : le lot 6e (éclairage des parties communes, ascenseurs, parkings) est accepté, en dernier dans le lot 6 et avant la production photovoltaïque ; il fait partie de la première version. La liste des 26 fiches « après v1 » est validée.
- **Constat** : l'ordre validé (Q4) nomme des familles de systèmes ; je l'ai traduit en lots 6a à 6d, 7a et 7b d'après les titres des fiches, sans les avoir lues en détail.
- **Deux points à confirmer** :
  1. Le lot **6e** (éclairage des parties communes §7.2, ascenseurs §10.1, parkings §10.3 et §10.4) n'était pas dans votre liste ; je l'ai ajouté parce que ces postes entrent dans le Cep du logement collectif. Sa place dans l'ordre est à fixer.
  2. Vingt-six fiches sont classées « après v1 » (ventilation naturelle et hybride, puits climatiques, CTA, DRV, cogénération, PAC gaz, géocooling, brasseurs d'air, production ECS mixte, récupération sur eaux grises, escalators…). La liste est dans `docs/couverture.md`.

## Questions ouvertes

### Q19 — Rayonnement direct non nul alors que `Gamma` vaut zéro — ouverte, à traiter au lot 3

- **Constat** fait sur le fichier météo réglementaire : 111 heures sur les 16 jeux (58 en `Th-BC`, 53 en `Th-D`) ont `Gamma = 0` et un rayonnement direct normal strictement positif, jusqu'à 100 W/m². Elles tombent toutes aux heures de lever et de coucher (5 h à 7 h et 17 h à 20 h UTC).
- **Enjeu** : la fiche 3.2 projette le rayonnement direct sur les parois à partir de la hauteur du soleil. Il faut établir ce que le texte prévoit quand la hauteur est nulle et le direct non nul (flux ignoré, ou formule qui reste définie).
- **Proposition** : lire la description mathématique de la fiche 3.2 sur ce point avant de coder, et écrire un test sur ces heures. Aucune hypothèse n'est prise d'ici là.

## Remarque de méthode : fiabilité de l'extraction du PDF

L'annexe III contient une couche texte exploitable, avec deux limites constatées :

1. Dans les nomenclatures, les lettres grecques d'une police de symboles disparaissent à l'extraction (`θaj(h)` devient `aj(h)`, `Φs1(h)` devient `s1(h)`).
2. Les équations sont du texte mis en page en deux dimensions : fractions, indices et exposants sont aplatis.

En conséquence, **chaque équation est relue sur l'image de la page du PDF avant d'être codée**. Le PDF fait foi ; les extractions de `docs/reference/texte/` ne servent qu'à la recherche.
