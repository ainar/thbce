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
| `arrete_4_aout_2021_articles.pdf` | Articles de l'arrêté (Légifrance) | `texte/arrete-4-aout-2021-articles.txt` |
| `annexe7_arrete_15_septembre_2006_reseaux_chaleur.pdf` | Facteurs d'émission et taux EnR&R des réseaux de chaleur et de froid (annexe 7 de l'arrêté du 15/09/2006, version en vigueur au 25/05/2026, Légifrance) | `texte/annexe-7-arrete-15-sept-2006-reseaux-chaleur.txt` |

## `scenarios/` — Scénarios conventionnels (chapitre 15 de l'annexe III)

- `2026-04-29_scenarios_conventionnels.xlsx` : classeur officiel, un onglet par usage (MI, LC, BUR, ENS_PRI…).
- `scenarios-conventionnels-2026-04-29.txt` : un bloc CSV par onglet.

## `inies/` — Données environnementales des énergies (base INIES)

Fiches de documentation INIES : électricité par usage (chauffage, ECS, climatisation, éclairage, autres usages ; résidentiel et tertiaire), gaz naturel, fioul, propane/butane, bois (bûches, granulés, plaquettes ; chaudière et poêle), réseaux de chaleur types, électricité exportée, gazole de chantier.

- `inies-energies-re2020.txt` : extraction des fiches « RE2020 » (données dynamiques).
- `inies-energies-des.txt` : extraction des fiches « DES ».
- Les fiches « EC » ne sont disponibles qu'en PDF.

Variante applicable à Ic énergie : « RE2020 ». Les valeurs réglementaires sont fixées par l'article 10 de l'arrêté ; les fiches « [RE2020] » les reprennent à l'identique (voir `docs/questions.md`, Q5).

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
- Table de correspondance département → zone climatique (Q1a).
- Tableau des coefficients de pondération dynamique de l'article 11 de l'arrêté, absent de l'extraction Légifrance (Q5).
- Note technique des fichiers climatiques ADEME / Météo-France : définition des colonnes (Q16).
