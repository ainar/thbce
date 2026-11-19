# Documents de référence

Sources réglementaires et données utilisées pour développer le moteur. Sauf mention contraire, ces documents proviennent du site [RT-RE bâtiment](https://rt-re-batiment.developpement-durable.gouv.fr/textes-en-version-consolidee-a617.html) (Licence Ouverte / etalab-2.0) ou de Légifrance.

Les fichiers `texte/` et les `.txt` sont des extractions automatiques (`pdftotext`, conversion des classeurs) destinées à faciliter la lecture et la recherche. **En cas de doute, le PDF d'origine fait foi.**

## `arrete/` — Arrêté du 4 août 2021 et textes associés

| Fichier | Contenu | Version texte |
|---|---|---|
| `annexeiii_arrete_4_aout_2021_complet_compresse.pdf` | Annexe III — Méthode de calcul Th-BCE 2020 (1854 p.) | `texte/annexe-iii-th-bce-2020.txt` |
| `annexei_arrete_4_aout_2021.pdf` | Annexe I — Définitions | `texte/annexe-i-definitions.txt` |
| `annexeii_arrete_4_aout_2021.pdf` | Annexe II — Règles générales (dont méthode ACV : Ic énergie, pondération dynamique) | `texte/annexe-ii-regles-generales.txt` |
| `annexeiv_arrete_4_aout_2021_new.pdf` | Annexe IV — renvoi vers les règles Th-Bat | — |
| `2026-04-29_regles_th-bat.pdf` | Règles Th-Bat 2020 complètes (29/04/2026) | `texte/regles-th-bat-2020.txt` |
| `annexevi_arrete_4_aout_2021.pdf` | Annexe VI — Contenu du RSEE (tient lieu de spécification en l'absence du XSD, non public) | `texte/annexe-vi-rsee.txt` (police du PDF mal encodée, corrigée automatiquement : quelques accents peuvent être faux) |
| `annexexi_arrete_4_aout_2021.pdf` | Annexe XI — Performances forfaitaires de certains lots (ACV, impact sur le changement climatique par phase et par usage ; version consolidée au 19 mars 2026, tableaux relus sur l'image, lot ACV) | `texte/annexe-xi-performances-forfaitaires.txt` |
| `arrete_4_aout_2021_articles.pdf` | Articles de l'arrêté (Légifrance) | `texte/arrete-4-aout-2021-articles.txt` |
| `annexe7_arrete_15_septembre_2006_reseaux_chaleur.pdf` | Facteurs d'émission et taux EnR&R des réseaux de chaleur et de froid (annexe 7 de l'arrêté du 15/09/2006, version en vigueur au 25/05/2026, Légifrance) | `texte/annexe-7-arrete-15-sept-2006-reseaux-chaleur.txt` |
| `chapitre1a3_annexe_r172-4_post-rivaton-i.pdf` | Chapitres I à III de l'annexe de l'article R. 172-4 du CCH (version documentaire du site RT-RE bâtiment, permis déposés à partir du 1er juillet 2026, ajustements « Rivaton ») : définitions des indicateurs, expression des résultats minimaux, **valeurs des exigences** (Bbio_max, Cep_max, Cep,nr_max, Ic_max, **DH_max** par catégorie de contraintes extérieures) et coefficients de modulation. Seuls les textes du Journal officiel font foi. | `texte/annexe-r172-4-chapitres-1-a-3.txt` |
| `chapitre1a3_annexe_r172-4_post-gtm2.pdf` | Chapitres I à III de l'annexe de l'article R. 172-4 du CCH, version documentaire du site RT-RE bâtiment intégrant l'extension aux bâtiments tertiaires spécifiques et industriels (décret n° 2026-16, arrêté du 19 mars 2026) : permis déposés du **1er mai au 30 juin 2026**. Valeurs des usages 3 à 28 identiques à la version post-Rivaton ; sans `MbHSP` ni `McHSP` (Q65, Q69). | `texte/annexe-r172-4-chapitres-1-a-3-post-gtm2.txt` |
| — (PDF Légifrance de l'annexe complète, 140 p., non versionné) | **Chapitre V** de l'annexe de l'article R. 172-4 du CCH (p. 135 à 140) : définition des **zones de bruit** (classes BR1 à BR3 des baies et des parties de bâtiment, partie I) et des **catégories de contraintes extérieures** (partie II). Texte public du code ; la validation des `exigences` en applique la partie II (Q54, point 14). Le chapitre IV (zones climatiques) n'est pas versionné : l'arrêté du 26 octobre 2010 en tient lieu (Q1a). | `texte/annexe-r172-4-chapitre-5.txt` |
| `jo_15-08-2021_articles_10-11.pdf` | Extrait du Journal officiel du 15 août 2021 (JORF n° 0189, texte n° 23, p. 58-59) : article 10 (coefficients d'énergie primaire et facteurs d'émission des énergies) et **article 11 (coefficients de pondération dynamique, années 0 à 50, cas général et fluides frigorigènes)**. Le tableau de l'article 11 est une image (p. 59), absente des extractions Légifrance : à transcrire depuis l'image (`pdfimages` en extrait une version à 396 ppi). | — |
| `arrete_26_octobre_2010_rt2012.pdf` | Arrêté du 26 octobre 2010 (RT2012, Légifrance). Son **annexe I** donne la correspondance département → zone climatique (96 départements, 2A et 2B inclus) ; la RE2020 reprend ce découpage à l'identique (page « Documents complémentaires » du site RT-RE bâtiment). | `texte/arrete-26-octobre-2010-rt2012.txt` |

## `fiches/` — Fiches d'application RE2020 (site RT-RE bâtiment, sans portée juridique)

| Fichier | Contenu | Version texte |
|---|---|---|
| `re2020-fa03_extension-construction-inf-50_v2.pdf` | FA03 v2 (1er octobre 2026) : extension nouvelle d'un bâtiment existant et construction de petite surface ; articles R. 172-1 et R. 172-3 du CCH, articles 50-2 et 50-3 de l'arrêté (rédaction de l'arrêté du 18 mars 2026 pour les surélévations) ; régimes « exigences de moyens » (50-3, III), « intermédiaire » (50-3, II) et « complet » selon l'usage, la surface et la part de l'existant (tableaux 2 à 4) ; paramètres de calcul d'une extension (annexe II) (Q86 à Q88). | `texte/fa03-extension-construction-inf-50-v2.txt` |
| `re2020-fa05_usage-batiment_v2.pdf` | FA05 v2 (1er octobre 2026) : identifier l'usage d'un bâtiment et l'exigence associée ; article R. 172-1 du CCH (dates d'application par usage), article 2 de l'arrêté (assimilation d'une petite partie à l'usage principal), correspondance destinations → usages Th-BCE, bâtiments à usage mixte (Q67, Q68). | `texte/fa05-usage-batiment-v2.txt` |
| `re2020-fa09_exclusion-application-tertiaire-specifique-industrie_v1.pdf` | FA09 v1 (1er octobre 2026) : exclusions d'application aux bâtiments tertiaires spécifiques et industriels (locaux de process, constructions temporaires, petites surfaces, extensions ; Q68). | `texte/fa09-exclusions-tertiaire-specifique-industrie-v1.txt` |

## `tableaux/annexe-iii/` — Tableaux 107 à 266 de l'annexe III en CSV (fiches 8.14 à 8.33, génération)

`tableau-NNN.csv` : transcription de 159 tableaux du texte public du Journal officiel (Licence Ouverte), relue sur l'image et comparée à la couche vectorielle du PDF ; **le PDF fait foi**. `README.md` du dossier : méthode, convention du CSV, écarts et ajouts de mise en forme, points tranchés. Anomalies du texte et de l'index relevées à cette occasion : `index/anomalies-constatees.md` (reprises à la fin de `index/anomalies.md`).

## `scenarios/` — Scénarios conventionnels (chapitre 15 de l'annexe III)

- `2026-04-29_scenarios_conventionnels.xlsx` : classeur officiel, un onglet par usage (MI, LC, BUR, ENS_PRI…).
- `scenarios-conventionnels-2026-04-29.txt` : un bloc CSV par onglet.

## `inies/` — Données environnementales des énergies (base INIES)

Fiches de documentation INIES : électricité par usage (chauffage, ECS, climatisation, éclairage, autres usages ; résidentiel et tertiaire), gaz naturel, fioul, propane/butane, bois (bûches, granulés, plaquettes ; chaudière et poêle), réseaux de chaleur types, électricité exportée, gazole de chantier.

- `inies-energies-re2020.txt` : extraction des fiches « RE2020 » (données dynamiques).
- `inies-energies-des.txt` : extraction des fiches « DES ».
- Les fiches « EC » ne sont disponibles qu'en PDF.

Variante applicable à Ic énergie : « RE2020 ». Les valeurs réglementaires sont fixées par l'article 10 de l'arrêté ; les fiches « [RE2020] » les reprennent à l'identique (voir `docs/questions/Q005.md`).

## `meteo/` — Données météorologiques

### Fichiers réglementaires : absents du dépôt

Le classeur officiel `4_scenarios_meteorologiques_th-bc_th-d_re2020.xlsx` ([page source](https://rt-re-batiment.developpement-durable.gouv.fr/documents-complementaires-a706.html)) est soumis à des conditions d'utilisation : usage limité au calcul réglementaire RE2020 et interdiction de communiquer les données, même partiellement. **Il ne doit jamais être committé** (voir `.gitignore`).

- Le moteur doit pouvoir le charger depuis un chemin fourni par l'utilisateur (classeur d'origine, ou CSV dérivé au format ci-dessous).
- Les tests qui en dépendent sont ignorés par défaut (`#[ignore]` ou feature dédiée) et lancés en local avec une variable d'environnement, par exemple `THBCE_METEO=/chemin/vers/meteo-re2020-conventionnelle.csv`.
- Format du classeur : 16 onglets `<zone>_<mode>` (zones H1a…H3, modes `Th-BC` pour besoins et consommations, `Th-D` pour le confort d'été), 8760 lignes, colonnes `h, te0, we0, dirN (W/m2), diff (W/m2), Teciel(C), Vent(m/s), Teau(C), Gamma (), Psi ()`.
- Format CSV dérivé (format long) : `zone, mode, h, te0, we0, dirN_W_m2, diff_W_m2, Teciel_C, Vent_m_s, Teau_C, Gamma, Psi`.
- Stations : découpage identique à la RT2012, sauf H2b (La Rochelle → Tours) et H3 (Nice → Marignane).

### `meteo/ademe/` — Fichiers ouverts (Licence Ouverte 2.0)

`tracc_france_2-7C_vague_de_chaleur.csv` : fichiers climatiques prospectifs ADEME / Météo-France ([data.gouv.fr](https://www.data.gouv.fr/datasets/donnees-climatiques-prospectives-france-2-7degc-vague-de-chaleur)), 8 stations, pas horaire, colonnes `STATION;LATITUDE;LONGITUDE;ALTITUDE;MOIS;JOUR;HEURE;T2m;T-1m;Hur;Hus;RR;FF;DD;Tc;Tr;Hs;Os;Os_2;Rgh;Rdn;Rdi;Ne;Ps`.

Non réglementaires (climat +2,7 °C avec vague de chaleur) et de format différent : à utiliser pour développer et tester la chaîne de calcul dans la CI publique, et pour un éventuel mode « climat futur » clairement signalé. Jamais pour un résultat présenté comme réglementaire.

## Encore manquant

- Schéma XSD du RSEE (non public) : partir de l'annexe VI, et du format JSON natif en attendant.
- Cas de validation avec résultats de référence.
- Note technique des fichiers climatiques ADEME / Météo-France : définition des colonnes (Q16).
