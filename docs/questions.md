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
| Q12 | Nombre de types de masques | tranchée, confirmée par le texte |
| Q13 | Numéro de département | tranchée, hypothèse du mainteneur |
| Q14 | Résidus « Tic » | tranchée, hypothèse du mainteneur |
| Q15 | Intitulé de l'éq. (2388) | tranchée, hypothèse du mainteneur |
| Q16 | Fichiers ADEME | tranchée : règles de lecture fixées, lecteur au lot 3 |
| Q17 | 365e jour de l'année | tranchée, **à vérifier à la lecture de la fiche 4.1** |
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
| Q31 | Protections mobiles (fiches 5.9 et 5.12) : matrices conventionnelles vérifiées sur les figures du JO par deux lectures indépendantes ; gestion automatique hors de la première version ; lectures du tableau 34 et de l'éq. (175) | tranchée ; changement de jour de l'éq. (175) à fixer au lot 4d |
| Q32 | Surventilation naturelle (fiche 5.13) : hystérésis de l'ouverture selon le sens de variation de la température, ratio conservé la nuit ; coquille de l'éq. (253) | tranchée, à confirmer par un cas de référence ; l'hystérésis est, avec Q25 et Q28, **l'un des premiers paramètres à tester si le DH diverge** (variante « bande à mémoire » conservée dans le code) |
| Q33 | Éclairage (fiche 7.1) : périmètre limité à l'habitation ; `CECL` en kWh et apports en W ; part des baies sous le plan de travail annoncée mais jamais utilisée | tranchée, à confirmer par un cas de référence |

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

- **Décision** : les masques verticaux droit et gauche comptent pour deux types. **Confirmé à la lecture de la fiche** : la p. 50 liste six identifiants de masque (`E_Az_cad`, `E_AZ`, `E_LV`, `h`, `vg`, `vd`), tous implémentés.
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

### Q25 — Décalage d'une heure entre le fichier météo et le calendrier du texte — tranchée le 16 novembre 2026, à confirmer par le premier cas de référence

- **Décision** : **lecture 2**. Le moteur lit le fichier météo ligne à ligne, la ligne `h` alimentant l'itération `h`, comme le moteur officiel le fait vraisemblablement, sans décaler les scénarios. Le but est de reproduire les résultats de référence, pas de corriger la convention du texte. Vérification indépendante du mainteneur : au solstice d'été, `Psi` passe par zéro entre les lignes 12 et 13 (vers 12 h 25 en indice de ligne) alors que le midi solaire vrai de Trappes est vers 11 h 52 UTC, ce qui n'est cohérent qu'avec des valeurs prises au milieu d'une heure exprimée en UTC+1. Le choix reste isolé dans `crates/thbce-core/src/climat/calendrier.rs`. **Si un cas COMETH diverge sur le Bbio ou le DH, c'est le premier paramètre à tester** (décaler d'une ligne la lecture du fichier météo), avant Q11.
- **Texte** : A3 §4.1.3 (p. 56). Le tableau fait correspondre l'itération 0 à l'intervalle 0 h – 1 h UTC, soit 1 h – 2 h en heure légale d'hiver. La décision Q1b retient que la ligne `h = 0` du fichier météo est l'itération 0.
- **Constat sur le fichier réglementaire** : pour les huit zones, le midi solaire (passage de `Psi` par zéro, et centre de la courbe de `Gamma`) tombe à la ligne d'indice `12,5 − longitude/15`, soit 12,37 pour Trappes et 12,46 pour Agen, alors qu'il a lieu à `12 − longitude/15` en heure UTC (11,87 et 11,96). Les rayonnements sont centrés sur le même instant. La ligne `h` décrit donc l'intervalle de `h − 1` à `h` en heure UTC, et non de `h` à `h + 1` : autrement dit, le fichier est indexé en UTC+1. La nomenclature du §3.1.2 dit d'ailleurs « Htsm : heure pour le calcul annuel (UTC+1) ».
- **Contrôle croisé** : dans les fichiers ADEME, où l'heure UTC est explicite, le midi solaire tombe bien à `12 − longitude/15` (11,86 pour Trappes).
- **Enjeu** : si l'on donne la ligne `h` à l'itération `h` et que l'on applique le tableau du §4.1.3 à la lettre, les scénarios d'occupation (en heure légale) sont décalés d'une heure par rapport au soleil et à la température. Deux lectures sont possibles :
  1. le tableau du §4.1.3 dit « UTC » pour « UTC+1 » : l'itération 0 est 0 h – 1 h en UTC+1, soit 0 h – 1 h légale en hiver, et les cases de scénario sont à décaler d'une heure ;
  2. le tableau est juste et le moteur réglementaire lit le fichier ligne à ligne : le décalage existe dans le calcul de référence.
- **État du code** : lecture 2, c'est-à-dire la décision Q1b appliquée telle quelle. Le choix est isolé dans `climat/calendrier.rs`. **C'est le deuxième point à revoir, avec Q11, si un cas de validation diverge** ; il touche le Bbio comme le DH. À trancher par le mainteneur, ou par le premier cas de référence.

### Q20 — Unité de γ dans le polynôme de l'éclairement direct (A3 §3.1.3, p. 32) — tranchée le 16 novembre 2026, hypothèse du mainteneur, à confirmer par un cas de validation

- **Décision** : γ en degrés ; l'argument de l'efficacité lumineuse est décisif. Le test `efficacite_lumineuse_du_direct_entre_50_et_150_lm_par_w` vérifie qu'elle reste entre 50 et 150 lm/W dès que le soleil dépasse 5°.
- **Texte** : `EDn = IDn × (−1,03753210E-08·γ⁶ + … + 1,24650445E+01·γ + 2,38954889E+00)`, sans unité pour γ, alors que la fiche donne γ en radians.
- **Hypothèse retenue dans le code** : γ en degrés. Le polynôme vaut alors 77 lm/W à 10°, 104 à 30° et 108 à 60°, ordre de grandeur attendu pour l'efficacité lumineuse du rayonnement direct. En radians, il ne dépasserait pas 17 lm/W. Implémenté dans `climat_exterieur::edn_lux`.

### Q21 — `dhg` non défini dans l'éq. (20) (A3 §3.2.3, p. 44) — tranchée le 16 novembre 2026, hypothèse du mainteneur, à confirmer par un cas de validation

- **Décision** : coquille du texte : lire `dhp` à la place de `dhg`.
- **Texte** : `Fh,dir = min[max(0 ; 1 − (dh − dhg)/hp) ; 1]`. `dhg` n'apparaît ni dans la nomenclature ni sur la figure 8, qui montrent `dhp`, « distance entre la paroi et le masque proche horizontal ».
- **Hypothèse retenue dans le code** : coquille, lire `dhp`, par symétrie avec `dpg` et `dpd` dans l'éq. (19).

### Q22 — Dernière ligne du tableau du calendrier (A3 §4.1.3, p. 56) — tranchée le 16 novembre 2026, hypothèse du mainteneur, à confirmer par un cas de validation

- **Décision** : la règle `Hleg = IHJ + EHLO` est appliquée partout.
- **Texte** : l'itération 8759 (23 h – 0 h UTC, en hiver) est donnée avec « Hleg 1H – 2H, case 2 », comme l'itération 7031 qui est en été. La règle d'hiver appliquée aux autres lignes donne 0 h – 1 h et la case 1.
- **Hypothèse retenue dans le code** : coquille de recopie ; la règle `Hleg = IHJ + EHLO` est appliquée partout.

### Q23 — Détails de lecture de la fiche 3.2 — tranchée le 16 novembre 2026, hypothèse du mainteneur, à confirmer par un cas de validation

- **Décision** : les cinq lectures sont retenues.
1. **`Tsimul`** (arbres à feuilles caduques, p. 50) : le texte ne dit pas si l'heure de simulation part de 0 ou de 1. Retenu : l'itération `h`, à partir de 0. L'enjeu est d'une heure aux deux bornes de la saison végétative (2161 et 6553).
2. **Égalité dans l'éq. (22)** (p. 47) : le texte donne `dhE > hpE ⇒ 1` et `dhE < hpE ⇒ 0`, sans le cas d'égalité. Retenu : composant éclairé.
3. **Repli de Φ** (éq. 3 et 4, p. 39–40) : le test `|Φ| ≥ π/2` suppose Φ dans [−π ; π], ce que la différence de deux angles eux-mêmes dans [−π ; π] ne garantit pas. Retenu : Φ est ramené dans [−π ; π[.
4. **Paramètre `Az`** (p. 36, « angle des différentes tranches azimutales ») : la description mathématique fixe 36 tranches de 10° repérées par rapport à la normale au composant (figure 10). `Az` n'apporte donc rien ; l'entrée est la liste des 36 hauteurs `γi`.
5. **Indice des tranches** : déduit de la figure 10, où le soleil à 20–30° de la normale, côté des Φ positifs, est « dans la tranche 21 » : `i = ⌊(Φ° + 180)/10⌋ + 1`.

### Q24 — Masques proches des ponts thermiques (A3 §3.2.3, p. 39) — tranchée par le texte, implémentée au lot 4c

- Le texte modélise un pont thermique vertical, pour les masques proches, comme une paroi verticale de 30 cm de large.
- **Code** : `bati/pont_thermique.rs`, constante `LARGEUR_MASQUES_PROCHES_M`. La largeur de 30 cm est appliquée à tout pont thermique décrit avec une exposition ; la hauteur de la paroi équivalente, que le texte ne donne pas, est à saisir (`hpb_m`) si un masque proche horizontal est déclaré, et la validation la réclame dans ce cas.

### Q19 — Rayonnement direct non nul alors que `Gamma` vaut zéro — tranchée par le texte, lecture confirmée par le mainteneur

- **Constat** : 111 heures sur les 16 jeux du fichier réglementaire ont `Gamma = 0` et un rayonnement direct strictement positif (jusqu'à 100 W/m²), aux heures de lever et de coucher. Explication probable : les rayonnements sont des moyennes horaires, la position du soleil une valeur ponctuelle (voir Q25).
- **Ce que dit le texte** (A3 §3.2.3, éq. 7 à 11) : aucun traitement particulier. `Drp = cos(θ)·IDn` reste défini : nul sur une paroi horizontale, puisque `cos(θ) = sin(γ) = 0`, et non nul sur une paroi verticale tournée vers le soleil. Seul `Jour_astro` (éq. 18) teste la hauteur du soleil.
- **Code** : formules appliquées telles quelles ; un test couvre ce cas.

### Q26 — Trois cellules du classeur des scénarios saisies « 0;5 » — tranchée le 16 novembre 2026, hypothèse à confirmer avant de calculer ces usages

- **Décision** : lire 0,5 : vraisemblablement une saisie avec un point-virgule à la place de la virgule. La liste de corrections reste explicite (onglet et cellule de chaque correction : `VES` J242, `GYM_MUN` J394, `GYM_PRI` J394) et la conversion échoue pour toute autre cellule non numérique. Sans effet sur la première version.
- **Constat** : dans le classeur officiel du 29 avril 2026, la cellule J242 de l'onglet `VES` et la cellule J394 des onglets `GYM_MUN` et `GYM_PRI` contiennent le texte « 0;5 » au lieu d'un nombre. Il s'agit chaque fois du facteur correctif de la semaine d'un local, pour la semaine 3 du mois d'août ; la semaine 4 du même mois vaut 0,5.
- **Hypothèse retenue dans la conversion** : erreur de saisie, lire 0,5. Les trois corrections sont listées explicitement dans `xtask/src/scenarios.rs` (`CORRECTIONS`) et rappelées en tête du fichier généré ; toute autre cellule non numérique fait échouer la conversion.
- **Portée** : usages 18 (vestiaires), 25 et 28 (établissements sportifs), hors première version. À confirmer avant de calculer ces usages ; une valeur de 0 ou de 1 changerait leur occupation d'août.

### Q27 — Valeurs inattendues dans le classeur des scénarios, hors habitation — tranchée le 16 novembre 2026 : conversion telle quelle

- **Décision** : rien n'est normalisé. Quand ces usages entreront dans le périmètre, la façon de les évaluer sera tranchée fiche par fiche.
- **Grilles de zone concernées** (valeurs hors de {0 ; 1} pour l'occupation, la ventilation et l'éclairage, hors de {−1 ; 0 ; 1} pour les consignes) :

| Onglet | Scénario | Grille | Valeur | Étendue |
|---|---|---|---|---|
| `COM` | Ventilation | hebdomadaire | 0,5 | 6 cases |
| `COM` | Éclairage | hebdomadaire | 0,5 | 6 cases |
| `SAN_N` | Éclairage | hebdomadaire | 0,35 | 56 cases |
| `IND_3x8` | Occupation, chauffage, refroidissement, ventilation, éclairage | annuelle | 0,5 | semaine 4 de décembre |
| `IND_8_18h` | Occupation, chauffage, refroidissement, ventilation, éclairage | annuelle | 0,5 | semaine 4 de décembre |

- **Ratios de locaux** : onglet `AER`, somme des `Rat_l` égale à 0,999.
- **Constat** : la fiche 4.1 (§4.1.2) déclare booléens les indicateurs de zone (occupation, ventilation, éclairage) et à trois états les indicateurs de consigne. Or le classeur contient 0,5 dans la ventilation et l'éclairage des commerces (usage 17), 0,35 dans l'éclairage des établissements de santé de nuit (usage 20), et 0,5 dans une case de la grille annuelle de l'occupation, de la ventilation, de l'éclairage, du chauffage et du refroidissement des deux usages d'industrie (23 et 24). Par ailleurs, les ratios de surface des locaux d'aérogare (usage 22) somment à 0,999.
- **Usages 1 et 2** : aucune anomalie ; les indicateurs sont bien booléens et les ratios somment exactement à 1 (tests de `thbce-data`).
- **Proposition** : lire la description mathématique de la fiche 4.1 pour ces cas au moment d'ouvrir les usages concernés. Les grilles sont converties telles quelles.

### Q28 — Lectures de la fiche 4.1 (A3 §4.1.2 et §4.1.3, p. 53 à 62) — tranchée le 16 novembre 2026, à confirmer par un cas de référence

- **Décision** : les lectures ci-dessous sont retenues.
  - `Agr` est la surface utile, et `su_m2` reste **obligatoire** pour tous les usages : une saisie explicite est préférée à une valeur par défaut implicite. En maison individuelle, on saisit en pratique `su_m2 = shab_m2`. En logement collectif, la surface habitable interne au calcul (`Rat_hab · Az`, éq. 33) est distincte de la SHAB saisie. Ces deux précisions figurent dans la description du champ (JSON Schema) et dans `docs/modele-donnees.md`.
  - Le jour des scénarios change à minuit en heure légale. Le choix est isolé dans `climat/calendrier.rs` (`date_scenario`), comme celui de Q25 : **c'est le deuxième paramètre à tester si un cas de référence diverge**, après la lecture du fichier météo.
  - Le 365e jour est inoccupé (conséquence de Q17).
  - `nb_lgt` vaut 1 par défaut en maison individuelle.

1. **Surface utile du groupe, `Agr`.** La fiche répartit les apports internes d'après `Agr`, « surface utile du groupe », y compris en habitation, où la surface de référence est la surface habitable (§13.1.3). En logement collectif, l'éq. (33) calcule même une « surface habitable » `Rat_hab·Az`, soit 90 % de la surface utile de la zone, distincte de la SHAB saisie. Retenu : `su_m2` est `Agr` et devient obligatoire pour tous les usages ; hors habitation, la même valeur sert de surface de référence « SU » (éq. 2376). À confirmer : que `Agr` et `SU` sont bien la même grandeur, et qu'en maison individuelle on saisit en pratique `Agr = SHAB`.
2. **Changement de jour des scénarios.** Le texte dit que le mois, la semaine, le jour et l'heure « se déduisent de l'heure de la simulation par incrémentations successives », les scénarios étant « en heure légale ». Retenu : le jour change quand la case de scénario revient à 1, c'est-à-dire à minuit en heure légale ; l'itération 0 est la case 2 du lundi de la première semaine, et la case 1 de ce lundi n'est jamais jouée. L'autre lecture (jour changeant à minuit UTC) décalerait d'un jour une ou deux cases chaque nuit.
3. **Conséquence de la décision Q17.** Le lundi ajouté à la dernière semaine de décembre tombe dans la semaine d'absence des usages d'habitation : le 365e jour est donc inoccupé, avec des consignes réduites. Une heure de plus (l'itération 8759, case 1) suit la même règle.
4. **Nombre de logements d'une maison.** `Nb_lgt` est défini « en logement collectif et pour les maisons accolées ». Retenu : 1 par défaut en maison individuelle, obligatoire en logement collectif.
5. **Locaux.** Les ratios de surface des locaux sont ceux par défaut du classeur (`Rat_l`) ; la saisie de ratios propres au projet viendra si elle s'avère nécessaire.
6. **Produit « × » du tableau 6.** Codé comme le produit arithmétique, qui coïncide avec le ET pour des valeurs 0 et 1 (seul cas des usages 1 et 2). Sans effet tant que Q27 ne se pose pas.

### Q29 — Lectures des fiches 5.15 à 5.20, parois opaques et ponts thermiques (A3 p. 281 à 305) — tranchée le 16 novembre 2026, à confirmer par un cas de référence

1. **Sens du coefficient `fce` : coquille de la nomenclature.** La nomenclature des fiches 5.17 et 5.19 annonce `fce = 1` « en période de confort d'été ». Les éq. (290) à (292) et (301) à (303) disent l'inverse : `fce = 1` en Th-B et Th-C, `fce = 1 − isconfort_adaptatif(j)` en Th-D, et `Sf = fce·Sf-c + (1 − fce)·Sf-e`. **Décision** : les équations ; c'est la nomenclature qui se trompe. On utilise donc `Sf-c` en Th-B et Th-C, et en Th-D `Sf-e` pendant la période de confort adaptatif, `Sf-c` en dehors. Isolé dans `bati::fce` ; test `en_th_d_le_facteur_solaire_bascule_avec_le_confort_adaptatif`.
2. **Branche 120–150° de l'éq. (294).** Entre β3 = 120° et β4 = 150°, la formule imprimée (vérifiée sur l'image de la p. 292) est `Uk_β = (Uvert − Uhoriz,des)/(β4 − β3)·β + (β4·Uhoriz,des − β3·Uvert)/(β4 − β3)` : elle donne `Uhoriz,des` à 120° et `Uvert` à 150°, avec un saut aux deux bornes, à l'inverse de la branche 30–60° et de la figure 64. **Décision** : l'interpolation continue (`Uvert` à 120°, `Uhoriz,des` à 150°), isolée dans `paroi_opaque::u_entre_beta_3_et_beta_4`, avec la formule imprimée en commentaire ; test `le_coefficient_u_est_continu_aux_quatre_bornes` (30, 60, 120 et 150°). Écart maximal entre les deux lectures : 0,04 m².K/W de résistance superficielle (0,21 − 0,17), soit moins de 1 % de U pour une paroi à 0,2 W/(m².K), et seulement pour des parois inclinées entre 120 et 150°.
3. **Position de référence de `Uk`.** L'éq. (294) recalcule U selon l'inclinaison réelle à partir de `Uk` et de `Incl_k`. **Décision** : `incl_k` est obligatoire, sans valeur par défaut, car une position par défaut changerait U en silence quand la saisie est oubliée. La description du champ donne la correspondance avec les règles Th-Bat : paroi verticale (0,17 m².K/W de résistances superficielles cumulées), plancher haut (flux ascendant, 0,14), plancher bas (flux descendant, 0,21).
4. **Description simplifiée des ponts thermiques** (§5.18.3, éq. 298). **Décision** : un pont thermique sans champ `exposition` suit la description simplifiée (quatre ponts verticaux de `l/4` au sud, à l'ouest, au nord et à l'est, sans masque) ; c'est signalé dans la description du champ. Avec `exposition`, il est décrit en détail.
5. **`he` dans l'éq. (306) : donné par le texte, rectification.** La première rédaction de ce point présentait `he = 25 W/(m².K)` comme repris des parois opaques « par analogie ». C'était inexact : en relisant la fiche pour documenter la source, la valeur figure bien dans le tableau des constantes de la fiche `C_BAT_pont thermique` (§5.19.2, p. 300 : « he, coefficient d'échange global (convectif + radiatif), 25,0 W/m²K »), comme dans celui de la fiche `C_BAT_paroi opaque` (§5.17.2, p. 290, éq. 297). L'éq. (306), relue sur l'image de la p. 302, s'écrit `Φsh_vc,l = ll · bvoute,l · (Qer*l · Ψl / he)`. Ce n'est donc pas une hypothèse ; il n'y a rien à vérifier de particulier sur ce point. Le flux est nul pour un pont vertical, puisque `Qer*` porte un facteur `cos β` (éq. 17).

### Q30 — Lectures des fiches 5.8, 5.10 et 5.14, baies vitrées (A3 p. 173 à 178, 219 à 244, 272 à 280) — tranchée le 16 novembre 2026, à confirmer par un cas de référence ; le point 8 repose sur une hypothèse du mainteneur

1. **Numérotation des types de baie pour l'éclairement : contradiction dans le texte.** L'introduction de la fiche 5.10 (p. 220) dit qu'une baie d'un volume normal est « de type 1 ou VN » et qu'un éclairant uniforme de grand volume est « de type 2 ou GVU ». La nomenclature (`Type_baie`, p. 225), le tableau de la p. 230, le §5.10.3.8 (p. 242) et la fiche 5.14 disent l'inverse. **Décision** : la nomenclature et les équations, soit 0 = GV, 1 = GVU, 2 = VN ; c'est l'introduction qui se trompe.
2. **`Rprot,b`.** Les éq. (210) et (219) à (233) emploient `Rprot,b`, que le texte ne définit pas par une équation ; l'éq. (209) définit `Rprot1,b = partGPM·Rprot1partGPM,b + (1 − partGPM)·Rprot11-partGPM,b`. **Décision** : `Rprot,b = Rprot1,b`. Avec `Rprot2 = 0`, les éq. (235) à (237) se réduisent alors exactement aux éq. (231) à (233) (test `sans_seconde_protection_les_eq_235_a_237_redonnent_les_eq_231_a_233`).
3. **Surfaces de baies par orientation** (§5.14.3, éq. 271 à 278). `Partbaies_sud` est rapportée à la surface des baies verticales, mais l'éq. (272) la multiplie par la surface de toutes les baies. **Décision** : le texte tel quel pour l'instant. Le test `ecart_de_l_eq_272_pour_un_groupe_avec_des_baies_horizontales`, marqué `#[ignore]`, documente l'écart (12 m² au lieu de 8 m² sur son exemple) ; à revoir quand ces grandeurs seront consommées.
4. **Secteurs d'orientation.** Le sud est défini par `−45° ≤ α ≤ 45°` alors que α est saisi entre 0 et 360°. **Décision** : `α ≤ 45°` ou `α ≥ 315°` ; les bornes 45° et 315° sont au sud, 135° et 225° au nord.
5. **Éq. (211), gestion automatique.** La part `1 − partGPM` des baies prend `Uap` si `Rprot11-partGPM,b = 1` et `Usp` dans tous les autres cas. **Décision** : le texte tel quel, sans interpolation.
6. **Coquilles de la fiche 5.14.** **Décision** : l'éq. (267), écrite `Flt2(1) = Σ Flt2,b(1)` pour le type 2, se lit `Flt2(2) = Σ Flt2,b(2)` ; l'indice `type_baie_l = 0` porté par toutes les sommes des éq. (262) à (270) se lit comme le type sommé.
7. **Stores vénitiens** (§5.10.3.6.2, p. 234 à 237). **Décision** : hors de la première version (rares en logement neuf, où l'on trouve surtout des volets roulants ou battants) ; classés « après v1 » dans `couverture.md`. Si une entrée en déclare un, la validation renvoie une erreur explicite (« store vénitien non pris en charge dans cette version »), sans calcul approché. Le champ qui déclare le type de protection mobile (`Type-PM`, fiche 5.9) n'existe pas encore dans le format d'entrée : l'erreur est ajoutée avec lui, dans la livraison des protections mobiles.
8. **Valeurs par défaut des facteurs solaires** (§5.10.3.6.3, éq. 215, 216 et 218). **Décision** : ces valeurs par défaut font partie de la méthode et sont appliquées. Les composantes `sw1_c`, `sw2_c`, `sw3_c` et `tlid` sont facultatives ; absentes, le moteur applique `Swsp-c = Swsp-e − 0,1` (sans protection mobile), `Swap-c = Swap-e` (avec protection mobile) et `Tlid = 0`, et liste chaque valeur appliquée dans les résultats (`baies[].valeurs_par_defaut`, avec la règle du texte). Règle générale retenue pour la suite : une valeur par défaut seulement quand le texte la donne, et toujours signalée dans les résultats.
   - **Décomposition du facteur global : hypothèse du mainteneur, faute de règle dans le texte, à confirmer par un cas de référence.** Les éq. (215) et (216) portent sur le facteur solaire **global** `Sw = Sw1 + Sw2 + Sw3`, et le texte renvoie aux règles Th-S pour sa décomposition. Retenu : les composantes « -c » gardent les proportions des composantes « -e » (`Swi-c = Swi-e × Sw-c/Sw-e`) ; avec protection mobile, elles leur sont donc égales. Raisons du mainteneur : `Sw1` est la transmission directe, propriété optique qui ne dépend pas des conditions aux limites, si bien que retrancher 0,1 à la seule `Sw1` serait la lecture la moins défendable ; ne les retrancher qu'à `Sw2` et `Sw3` produirait souvent des composantes négatives et demanderait une règle de report arbitraire ; la règle proportionnelle est simple, conserve le total imposé par le texte et ne produit jamais de composante négative. Isolée dans `baie::repartir_facteur_global`. La règle affichée dans `valeurs_par_defaut` mentionne l'hypothèse.
   - Les trois composantes « -c » se saisissent ensemble ou pas du tout. Si `Swsp-e ≤ 0,1`, la valeur par défaut ne serait pas positive : la validation demande alors la saisie.
9. **U d'une baie inclinée.** **Décision** : la validation refuse une baie dont le type ne donne pas le coefficient nécessaire à son inclinaison (éq. 207 et 208), sans substitution. Une baie est inclinée de 0 à 90°.
10. **Type de baie en habitation.** **Décision** : toutes les baies sont de type 2 dans la première version ; `Type_baie` et `Type_volume` viendront avec les usages à grands volumes.

### Q31 — Fiches 5.9 et 5.12, protections mobiles (A3 p. 179 à 218 et 251 à 254) — tranchée le 16 novembre 2026 ; points 3 à 9 validés, à confirmer par un cas de référence

**Périmètre livré** : volets et stores enroulables en gestion manuelle (motorisée ou non), usages 1 et 2, seconde protection mobile, règles du calcul Th-D, perméabilité à l'air. Les stores vénitiens (décision Q30) et la gestion automatique (point 2) sont refusés par la validation.

1. **Matrices conventionnelles transcrites d'images — vérifiées par le mainteneur le 16 novembre 2026.** Les matrices de la gestion manuelle (figures 35 et 38) sont des images dans le texte. Source retenue : les figures publiées au Journal officiel du 15 août 2021, de meilleure définition que le PDF compressé de l'annexe, versionnées dans `docs/reference/arrete/figures/` : `a3_figure35_volets_habitation_jo.png` (volets, page 357 du JO, 986 × 653 pixels, image sans perte) et `a3_figure38_stores_enroulables_habitation_jo.png` (stores enroulables, page 359 du JO, 780 × 509 pixels). Le mainteneur a comparé la transcription ci-dessous (`thbce-data/src/protections_mobiles.rs`) à ces images, cellule par cellule, pour la gestion manuelle de la famille « habitation, hôtellerie et hébergement » : toutes les valeurs sont exactes, y compris les seuils Ecl2 d'été en gestion motorisée (20 000 lux) et les seuils à 80 000 lux, sur lesquels la première lecture hésitait. **Vérifiée par deux lectures indépendantes** : l'agent a ensuite relu les deux images du JO, agrandies, et comparé les 96 valeurs transcrites (deux matrices ; 8 lignes d'occupation de 5 valeurs et 8 lignes d'inoccupation de 2 valeurs chacune) : aucune cellule en désaccord. Seules ces lignes sont transcrites.

   Figure 35, **volets** (Rprot0 / Ecl1 en lux / Rprot avec PM2 / Ecl2 en lux / nuit) :

   | Occupation | hiver | mi-saison | été, veille < 26,5 °C | été, veille ≥ 26,5 °C |
   |---|---|---|---|---|
   | non motorisée | 15 % / 100 000 / 15 % / 100 000 / 80 % | 15 % / 100 000 / 15 % / 80 000 / 80 % | 20 % / 80 000 / 20 % / 30 000 / 80 % | 25 % / 60 000 / 40 % / 30 000 / 80 % |
   | motorisée | 10 % / 100 000 / 10 % / 100 000 / 90 % | 10 % / 100 000 / 10 % / 60 000 / 90 % | 20 % / 60 000 / 20 % / 20 000 / 90 % | 30 % / 40 000 / 40 % / 20 000 / 90 % |

   | Inoccupation (jour / nuit) | hiver | mi-saison | été, veille < 26,5 °C | été, veille ≥ 26,5 °C |
   |---|---|---|---|---|
   | non motorisée | 20 % / 70 % | 20 % / 70 % | 20 % / 70 % | 40 % / 70 % |
   | motorisée | 10 % / 80 % | 10 % / 80 % | 20 % / 80 % | 50 % / 80 % |

   Figure 38, **stores enroulables** :

   | Occupation | hiver | mi-saison | été, veille < 26,5 °C | été, veille ≥ 26,5 °C |
   |---|---|---|---|---|
   | non motorisée | 10 % / 100 000 / 10 % / 100 000 / 10 % | 10 % / 80 000 / 10 % / 80 000 / 10 % | 15 % / 60 000 / 15 % / 30 000 / 10 % | 20 % / 40 000 / 40 % / 30 000 / 10 % |
   | motorisée | 10 % / 100 000 / 10 % / 100 000 / 10 % | 10 % / 60 000 / 10 % / 60 000 / 10 % | 20 % / 50 000 / 20 % / 20 000 / 10 % | 30 % / 30 000 / 50 % / 20 000 / 10 % |

   | Inoccupation (jour / nuit) | hiver | mi-saison | été, veille < 26,5 °C | été, veille ≥ 26,5 °C |
   |---|---|---|---|---|
   | non motorisée | 10 % / 20 % | 20 % / 20 % | 30 % / 20 % | 40 % / 20 % |
   | motorisée | 10 % / 10 % | 20 % / 10 % | 30 % / 10 % | 60 % / 10 % |

2. **Gestion automatique (`Type-GPM = 1`) : hors de la première version (décision).** La validation renvoie « gestion automatique des protections mobiles non prise en charge dans cette version ». Constat : le texte se contredit sur ses matrices — la nomenclature (p. 181) et le §5.9.3.1 (p. 185) en font des données d'entrée propres à la baie, le §5.9.3.3.2 (p. 205, figures 44 à 47) en donne des versions « conventionnelles » ; les seuils `Eclim_auto`, `Toph-1limh_auto`, `Toph-1limb_auto` et `Vventlim_auto` n'ont pas de valeur conventionnelle ; l'hystérésis de la figure 31 n'est décrite que par un schéma. **Pour le jour où elle sera traitée** (notes du mainteneur) :
   - la figure 35 du JO donne bien des valeurs conventionnelles pour les lignes de gestion automatique (dérogation manuelle), avec et sans détecteur de présence ;
   - le ministère a publié une fiche d'application dédiée, `20250908_fiche_application_gestion_automatique_protections_mobiles_v1_1.pdf`, sur le site RT-RE bâtiment : c'est elle qu'il faudra lire pour lever les contradictions du texte sur les seuils et l'hystérésis.
3. **Éq. (181).** **Décision** : « ou ». Le texte écrit « si Saison = 2 et Saison = 4 » pour la mi-saison : lu « ou ». Saison effective 3 : hiver ; 2 et 4 : mi-saison ; 1 : été.
4. **Éq. (175), changement de jour.** `Topj-1max` est reporté « si incrémentation de `Jour_année` ». Le calcul (`TemperaturesMaximales::avancer`) reçoit cette information de l'appelant ; le choix entre minuit UTC et minuit légal sera tranché au lot 4d, en cohérence avec la décision Q28 (lecture validée). La remise à zéro de `Topjmax` (et non à −∞) et la valeur nulle du premier jour sont celles du texte.
5. **Tableau 34 (Th-D), colonne « Résidentiel ».** Lectures validées : (a) les lignes « (jour = nuit) » désignent les valeurs de nuit, en occupation comme en inoccupation ; (b) pour les volets, de jour, `max(80 % ; valeur Th-BC)` en gestion motorisée et `max(70 % ; valeur Th-BC)` sinon, appliqué à `Rprot0` en occupation et au ratio de jour en inoccupation ; de nuit, `min(70 % ; valeur Th-BC)` ; (c) pour les stores enroulables, « idem volets » de jour et 50 % de nuit (valeur fixe ; l'extraction du texte décale cette cellule, relue sur l'image de la p. 218) ; (d) la règle joue dès que `Topj-1max ≥ Topj-1lim_manu` en période de confort adaptatif, quelle que soit la saison de la ligne ; (e) la seconde protection est alors désactivée (éq. 201) et la première suit la colonne « sans 2nde protection ».
6. **Vent et stores enroulables.** L'éq. (192) remonte le store extérieur en occupation quand `Vvent_c ≥ 10 m/s` ; l'éq. (193), en inoccupation, ne teste pas le vent. **Décision** : le texte tel quel. La seconde protection, intérieure, n'est pas concernée.
7. **`Typo_PM`** (§5.12.3.1, tableau 40). Le tableau qualifie la typologie 0 (0 %) de « valeur par défaut » : appliquée si `typo_pm` est absent, et signalée dans `valeurs_par_defaut`. La nomenclature borne `Typo_PM` à 4 alors que le tableau va jusqu'à 5 (« pas de protection mobile », 100 %) : la typologie 5 n'est pas une saisie, elle découle de l'absence de protection.
8. **Format d'entrée.** La protection mobile et sa gestion sont décrites sur le type de baie (`protection_mobile`). `seconde_protection` et `exterieure` sont obligatoires, le texte ne leur donnant pas de valeur par défaut. `Pocc` vient de la figure 34 (tableau en texte) : 0,5 en maison, 0,7 en logement collectif.
9. **`Aouvmax`** (éq. 238 et 239). `Aouvmax = Ab·Rouvmax` demande `Rouvmax`, entrée de la surventilation naturelle (fiche 5.13) : la fonction de l'éq. (239) est livrée, son branchement viendra avec cette fiche.

### Q32 — Fiche 5.13, surventilation naturelle par ouverture des baies (A3 p. 255 à 271) — tranchée le 16 novembre 2026, à confirmer par un cas de référence

**Périmètre livré** : gestion manuelle de l'ouverture, règles du calcul Th-B, surface d'ouverture équivalente avec protection mobile, débit du groupe en traversant et en non traversant. Les deux premiers points portent sur le DH et sur le besoin de froid du Bbio.

1. **Hystérésis de `Rouv_θop_man` : seul un schéma la décrit (figure 62, p. 262) — tranchée, à confirmer par un cas de référence.** Le schéma montre une branche d'ouverture (0 à `θouv1`, 1 à `θouv2`) et une branche de fermeture (1 jusqu'à `θfer1`, 0 à `θfer2`), sans dire comment on passe de l'une à l'autre. **Décision** : la branche suivie dépend du sens de variation de la température opérative entre `h−2` et `h−1` (hausse : branche d'ouverture ; baisse : branche de fermeture) et le ratio est conservé à égalité. C'est la seule lecture qui emploie `θop,fin(h−2)`, que la nomenclature cite parmi les entrées : l'indice le plus solide de ce que fait le moteur officiel, qu'il s'agit de reproduire. La variante « bande à mémoire », `Rouv = clamp(Rouv(h−1) ; branche d'ouverture(θop) ; branche de fermeture(θop))`, est conservée dans la même fonction (`surventilation::r_ouv_theta_op_man_selon`, `LectureHysteresis::BandeAMemoire`), sans être exposée dans le format d'entrée. Le test `difference_entre_les_deux_lectures_de_l_hysteresis` documente l'écart principal : sur la branche de fermeture à un ratio de 0,8, une légère hausse de température à l'intérieur de la bande fait redescendre le ratio à 0 avec la lecture retenue, alors que la variante le conserve. **Ce point rejoint la liste des paramètres à tester en premier si le DH s'écarte d'un cas de référence, avec Q25 et Q28 ; la bande à mémoire est la première variante à essayer.**
2. **Comportement hors de la « période journée » — tranché, à confirmer par un cas de référence.** Le texte ne donne le profil qu'« en occupation et en période journée (hlég > 7 h et hlég < 23 h) », soit de 8 h à 22 h. **Décision** : hors de cette plage, le ratio précédent est conservé. Les deux autres lectures sont contredites par le texte : appliquer le profil à toute heure contredit la restriction explicite ; fermer la nuit rendrait inutiles les valeurs de nuit de `Cpr` du tableau 42 (1 en BR1, 0,3 en BR2 et BR3). La lecture retenue donne des fenêtres laissées ouvertes la nuit si elles l'étaient le soir, modérées par `Cpr` : c'est la surventilation nocturne, qui pèse sur le DH. En inoccupation, `Cpr = 0` ferme de toute façon.
3. **Deux définitions de la journée.** `Cpr` (tableau 42) : journée de 7 h à 21 h, nuit de 22 h à 6 h. Hystérésis : `hlég > 7 h et hlég < 23 h`, soit de 8 h à 22 h. **Décision** : chaque règle garde sa plage, telle qu'écrite.
4. **Bornes de `Mod_ext` (figure 61).** Retenu : nul jusqu'à `θei_seuil_bas` inclus, linéaire jusqu'à 1 à `θei_seuil_haut`, égal à 1 jusqu'à `θop(h−1) − Δθext_int` inclus, nul au-delà.
5. **Éq. (253) : coquille.** Le texte dit que seules les baies d'inclinaison supérieure ou égale à 60° sont regroupées par orientation, les baies de toiture étant « sous le vent » ; l'éq. (252) porte la condition `βn ≥ 60°` ; l'algorithme de l'éq. (253) écrit « βb < 60° ». **Décision** : `β ≥ 60°`, comme le texte et l'éq. (252). Les baies de toiture comptent dans `Aouv`, pas dans les secteurs.
6. **Secteur unique dans l'éq. (252).** Quand un secteur réunit toute la surface d'ouverture, `Aouv − Aouvn` est nul et le terme n'est pas défini. Le texte suppose « au moins 2 » secteurs non vides. Retenu : terme nul (limite de la formule), donc `Sors = 0` avec une seule façade ; le débit se réduit alors au tirage thermique. C'est cohérent avec l'exemple de la p. 270, où la distribution qui réunit les deux baies est celle qu'on retient parce qu'elle minimise le débit.
7. **`Cst`.** La constante est imprimée « 0.003 / 5 » sur deux lignes du tableau : lue 0,0035 m/(°C.s²) (validé).
8. **`θiifr` en Th-B** (p. 263) : « la température de consigne froid », sans indice. Retenu : `θiifr_+`, la consigne de refroidissement en occupation normale.
9. **Gestion automatique de l'ouverture** (§5.13.3.1.3) : non prise en charge, comme celle des protections mobiles ; le format d'entrée ne permet pas de la déclarer (`Has_Gestion_Auto_Ouverture = 0`). À confirmer hors de la première version.
10. **Valeurs par défaut et conventions appliquées**, signalées dans les résultats : `Rouvmax` par défaut du type d'ouvrant (tableau 44) quand `r_ouv_max` n'est pas saisi ; `Httf = 1,5 m` « dans le cas général » quand `h_ttf_m` n'est pas saisi (sa saisie n'est admise qu'au-delà de 4 m, jusqu'à 15 m). Le caractère traversant est conventionnel en maison individuelle (tableau 43) et saisi en logement collectif.
11. **Portes d'accès.** Le texte exclut de la surventilation les « portes d'accès ou de communication avec d'autres locaux » : une baie n'y contribue que si son type déclare un ouvrant ; c'est à la saisie de ne pas en déclarer pour une porte.
12. **Branchement.** Le ratio d'ouverture dépend des températures du groupe aux pas précédents et de `Δθconf_adapt` : l'enchaînement horaire viendra avec le modèle thermique (lot 4d). `Autfr,pro(j)` (Th-B) vient de la fiche des saisons (4.6).

### Q33 — Fiche 7.1, éclairage (A3 p. 470 à 501) — tranchée le 16 novembre 2026, à confirmer par un cas de référence

**Périmètre livré** : usages d'habitation (1 et 2), où le système d'éclairage est « entièrement conventionnel » (§7.1.3.6.2) : `Pecl_tot = 1 W/m²`, `Pecl_aux = 0`, `Ratio_ecl_nat = 1`, `C1 = 0,9`, courbe `C2` propre au résidentiel. Éclairement naturel des volumes normaux (baies de type 2), autonomie en lumière naturelle, apports récupérables. Les tableaux 78 (types de locaux, `C1`, `Eiref`) et 79 (courbes `C2` par mode de gestion), les grands volumes, le fractionnement et les règles des bureaux et des commerces ne sont pas transcrits : ils viendront avec ces usages.

1. **Unité de `CECL`.** La nomenclature annonce `CECL_local`, `CECL_GR`, `FeclC` et `FeclR` en Wh. L'éq. (789), `CECL_local = max(Pecl_tot·CTRLecl_occ·CTRLecl_einat ; Pecl_aux)·Alocal/1000`, produit des W/m² par des m², divisé par 1000 : des kWh sur l'heure. **Décision** : `CECL_GR` en kWh (des kW maintenus sur un pas d'une heure) ; les flux récupérables `FeclC = Crec_ecl_conv·CECL_GR` et `FeclR = Crec_ecl_rad·CECL_GR` sont repassés en W (× 1000) avant d'entrer dans le bilan du modèle thermique. Verrouillé par le test `ordre_de_grandeur_et_unites_des_apports_d_eclairage` : 100 m² occupés sans lumière naturelle, 0,09 kWh sur l'heure et 90 W d'apports, moitié convectifs, moitié radiatifs.
2. **`Einat(2)` : division répétée.** Le texte écrit « Einat(2) = 1/Aeclnat2·(Einatbaiedif(2) + Einatplafref(2) + Einatinter(2)) » alors que chacun des trois termes est déjà divisé par `Aeclnat2`, puis « Einat(2) = 1/Aeclnat2·[K1vn·Flt1(2) + K2vn·Flt2(2) + K3vn·Flt3(2)] ». **Décision** : la forme en `K`, homogène à des lux.
3. **Part des baies sous le plan de travail.** La fiche 5.10 (p. 230) annonce qu'on lit, pour les baies non horizontales des volumes normaux, « la part de la baie située en dessous du plan de travail (Part_baie_sous_plan) ». **Décision** : donnée annoncée mais jamais utilisée par le texte ; sans effet. Le texte dit seulement qu'on la « lit » (p. 230) ; aucune nomenclature ni aucune équation de l'annexe ne l'emploie. Aucun champ d'entrée.
4. **Surface ayant accès à la lumière naturelle en habitation.** Le §7.1.3.6.2 fixe `Ratio_ecl_nat = 1` : `Aeclnat2` est donc la surface utile du groupe, `Agr`, et l'éclairement `Einat(2)` s'emploie sans le facteur de réduction du §7.1.3.6.1 (qui vaut 1 pour un ratio de 1). Un groupe d'habitation sans baie a un `Einat` nul puisque ses flux sont nuls.
5. **Courbe `C2` du résidentiel** (p. 501, figure 86, relue sur l'image). Points A (0 ; 1), B (100 ; 1), G (200 ; 0,05) et J (2800 ; 0) : le texte annonce des points « indiqués dans le tableau 79 », mais le point G du tableau 79 est (700 ; 0,3) ; les valeurs propres au résidentiel sont retenues.
6. **Numérotation des types de baie dans la fiche 7.1.** L'introduction et le tableau 76 (p. 471) emploient encore « type 1 » pour les volumes normaux et « type 2 » pour les grands volumes ; la nomenclature (p. 475) et les équations emploient 0 = GV, 1 = GVU, 2 = VN, comme la décision Q30.
7. **Modes de calcul.** L'éq. (781) donne le système conventionnel de l'habitation « pour Th-B » ; le §7.1.1.1 dit qu'il est conventionnel « pour le calcul des coefficients Bbio et C », et le §7.1.3.6.2 ne distingue pas les modes. **Décision** : les mêmes valeurs conventionnelles en Th-B, Th-C et Th-D.
8. **Autonomie en lumière naturelle.** Les compteurs de l'éq. (785) sont livrés ; ils ne sont pas encore restitués dans les résultats (branchement horaire au lot 4d).

## Remarque de méthode : fiabilité de l'extraction du PDF

L'annexe III contient une couche texte exploitable, avec deux limites constatées :

1. Dans les nomenclatures, les lettres grecques d'une police de symboles disparaissent à l'extraction (`θaj(h)` devient `aj(h)`, `Φs1(h)` devient `s1(h)`).
2. Les équations sont du texte mis en page en deux dimensions : fractions, indices et exposants sont aplatis.

En conséquence, **chaque équation est relue sur l'image de la page du PDF avant d'être codée**. Le PDF fait foi ; les extractions de `docs/reference/texte/` ne servent qu'à la recherche.
