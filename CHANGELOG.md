<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Journal des modifications

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le projet le [versionnage sémantique](https://semver.org/lang/fr/). Chaque version indique la version de la méthode Th-BCE implémentée. L'historique détaillé du développement, lot par lot, est dans [`docs/historique.md`](docs/historique.md).

## [Non publié]

### Exigences (lot 0b)

- Exigences maximales des usages 3 à 28 (annexe de l'article R. 172-4, chapitre III : `Bbio_maxmoyen`, `Cep_maxmoyen`, `Cep,nr_maxmoyen`, `Icénergie_maxmoyen` par période et selon le raccordement à un réseau de chaleur, `Mbgéo`, `Mbsurf_tot`, `Mbbruit`, `Mcgéo`, `Mcsurf_tot`, `Mccat`, `DH_maxcat`), relues sur l'image des pages et protégées par des tests de contrôle (Q65). Les zones de ces usages restent refusées par la validation tant que leur calcul n'est pas ouvert ; la fonction d'exigences est testée sur des calculs à la main (bureaux, enseignement, hôtel, commerce, santé, industrie).
- Catégories de contraintes extérieures 1 à 3 et classe de bruit par zone (`exigences.zones[]`) et catégorie par groupe (`exigences.zones[].groupes[]`), contrôlées selon le chapitre V, partie II pour tous les usages (Q66) ; catégorie 3 : `Mbbruit` et `Mccat` de la ligne « Catégorie 3 », DH sans seuil.
- Dates d'effet par usage (1er juillet 2022 pour les bureaux et l'enseignement, 1er mai 2026 pour les usages 6 à 28) et mois de dépôt facultatif `exigences.mois_depot_permis` (Q68).
- `MbHSP` et `McHSP` appliqués aux seuls permis déposés à partir du 1er juillet 2026 (décret n° 2026-200, Q69) ; sans effet sur les exemples.
- Format : champs ajoutés, tous facultatifs (`exigences.zones`, `exigences.mois_depot_permis`) ; `exigences.hsp_moy_m` devient facultatif (obligatoire en habitation pour un permis à partir du 1er juillet 2026 : une entrée d'habitation de 2026 sans mois doit le donner, comme avant). Résultats : `exigences.zones[].categorie_contraintes_exterieures` et `zone_bruit` ajoutés. API Rust : `dh_max` renvoie un `SeuilDh` (« Pas de seuil » en catégorie 3), `ParametresExigences` n'est plus `Copy`.
- Références versionnées : chapitres I à III de l'annexe de l'article R. 172-4 dans leur version post-GTM2 (permis du 1er mai au 30 juin 2026), fiches d'application FA05 v2 (usage d'un bâtiment) et FA09 v1 (exclusions du tertiaire spécifique et de l'industrie).

## [0.1.0] - 2026-11-17

Méthode : Th-BCE 2020, annexe III de l'arrêté du 4 août 2021, version consolidée au 19 mars 2026 ; annexe de l'article R. 172-4 du CCH (exigences) dans sa rédaction applicable aux permis déposés à compter du 1er juillet 2026.

Première version publiée. **Ce n'est pas le moteur réglementaire officiel ; ses résultats ne valent pas attestation de conformité.** La version 1.0.0 sera publiée après validation contre le moteur réglementaire COMETH, au moyen d'un logiciel certifié. Jusque-là, les contrats (format JSON, CLI, API JavaScript) sont considérés comme stables, mais les résultats peuvent changer à la suite de cette validation.

### Calcul

- Calcul annuel au pas horaire (8 760 h) de la maison individuelle ou accolée et du logement collectif : Th-B (Bbio), Th-D (DH, météo à séquence caniculaire), Th-C (consommations par poste et par énergie) ; Cep, Cep,nr et Ic énergie (pondération dynamique de l'article 11, coefficients des articles 9 et 10) ; exigences maximales (Bbio_max, Cep_max, Cep,nr_max, Icénergie_max, DH_max et leurs modulations) et conformité.
- Enveloppe : parois opaques, baies, ponts thermiques, masques proches et lointains, protections mobiles, surventilation par ouverture des baies, espaces tampons solarisés ou non.
- Systèmes :
  - ventilation simple et double flux, émetteurs et distributions de chauffage, de refroidissement et d'ECS ;
  - effet Joule, pompes à chaleur électriques sur air, chaudières gaz, fioul et bois, poêles et inserts, réseaux de chaleur et de froid ;
  - ECS instantanée ou par ballon, chauffe-eau thermodynamiques, CESI, systèmes solaires combinés, CESCI et CESCAI ;
  - photovoltaïque et autoconsommation ;
  - éclairage des parties communes, ascenseurs et parkings.
- Valeurs de la version 0.1.0 sur la météo réglementaire (H2b, permis 2025, catégorie 1, BR1) :

  | Exemple | Bbio | Cep | Cep,nr | Ic énergie | DH |
  |---|---|---|---|---|---|
  | `maison-climat-ademe` (panneaux rayonnants, ballon électrique) | 59,7 | 145,8 | 145,8 | 182,3 | 1 042,9 |
  | `maison-plancher-ademe` (PAC air/eau, plancher chauffant, chauffe-eau thermodynamique) | 59,7 | 56,2 | 56,2 | 68,2 | 1 030,9 |
  | `maison-chaudiere-gaz-ademe` | 59,7 | 85,4 | 85,4 | 591,1 | 1 103,3 |
  | `maison-poele-granules-ademe` | 59,7 | 106,8 | 67,7 | 122,5 | 1 042,9 |
  | `maison-reseau-chaleur-ademe` | 59,7 | 73,5 | 44,6 | 407,0 | 992,7 |
  | `maison-ssc-ademe` | 59,7 | 53,3 | 53,3 | 64,7 | 1 132,5 |
  | `immeuble-collectif-ademe` (12 logements) | 31,8 | 186,4 | 186,4 | 231,3 | 1 039,6 |
  | `immeuble-cesci-ademe` | 31,8 | 171,5 | 171,5 | 214,3 | 1 108,7 |

  Aucun exemple n'est conforme : ils servent aux essais, non de modèles. Les tests ignorés sur météo réglementaire vérifient ces valeurs pour la maison et l'immeuble d'exemple ; sur la météo ouverte ADEME, les `attendu.json` de `tests/cas/` fixent Bbio et DH de tous les exemples calculés sur cette météo.

### Données d'entrée

- Classeur météo réglementaire `.xlsx`, CSV dérivé (`thbce meteo convertir`), fichiers ouverts ADEME / Météo-France (non réglementaires, résultat marqué `reglementaire: false`) ; aucune météo embarquée.
- Format JSON natif décrit par son JSON Schema ; champs facultatifs `$schema` et `version_format` à la racine.

### Interfaces

- Ligne de commande `thbce` : `calcul`, `valider`, `schema`, `meteo convertir`, `version` (et `--version`, `-V`) ; messages et aide en français ; codes de retour 0 (succès), 1 (usage ou fichier), 2 (entrée invalide), 3 (entrée hors du périmètre de cette version, à la validation comme au calcul, ou calcul impossible).
- Module WebAssembly typé : `calculer`, `valider`, `versionMoteur`, `schemaEntree`, `schemaResultats` ; types TypeScript `Projet` et `Resultats` générés depuis les JSON Schema ; page d'exemple `examples/web/`.
- 15 exemples dans `tests/cas/`.

### Contrats stables

Considérés comme stables : le format JSON d'entrée et celui des résultats (des champs facultatifs ou des valeurs nouvelles peuvent s'ajouter), la ligne de commande (commandes, options, codes de retour) et l'API JavaScript (fonctions, paramètres, nature des erreurs). L'API Rust n'est pas stabilisée. La version 1.0.0 sera publiée après validation contre le moteur réglementaire COMETH, au moyen d'un logiciel certifié. Jusque-là, les contrats (format JSON, CLI, API JavaScript) sont considérés comme stables, mais les résultats peuvent changer à la suite de cette validation.

### Changements de format pour les utilisateurs de `main` d'avant la 0.1.0

Une entrée écrite pour une version de développement doit être mise à jour (`thbce valider` signale chaque champ) :

- `generations[]` décrit ses générateurs : `generateurs[]` (`id`, `rdim`, `nature`), `position`, `priorite` et gestion de la température selon les cas (lot 6c) ;
- `ventilation` est obligatoire sur une zone d'habitation (système et réseau du projet, lot 6a) ;
- un groupe d'habitation porte `iclim`, `programmation`, `distributions`, `emetteurs` et `emetteurs_ecs` ; un bâtiment `distributions_intergroupes` et `distributions_ecs_intergroupes` (lot 6b) ;
- un type de paroi porte `incl_k` ; un groupe `su_m2` et `v_m3` ; une zone `h_zone_m`, `h_plancher_m` et `q4pa_surf_m3_h_m2` ; au moins une bouche dans `bouches_conduits` (lot 4) ;
- un masque lointain azimutal se saisit par `gamma_i_deg` (36 hauteurs) ;
- `kit.k_theta` est requis sur un kit CESI ; `poele_ou_insert.combustible_bois` est obligatoire ;
- renommages de la préparation de la 0.1.0 :

  | Avant | Après |
  |---|---|
  | `zones[].nb_lgt` | `zones[].nb_logements` (comme `emetteurs_ecs[].nb_logements`) |
  | `distributions[].distribution_intergroupes` | `distributions[].distribution_intergroupe` (une seule référence) |
  | `installations_pv[].n_modules` | `installations_pv[].nb_modules` (aussi dans les résultats) |
  | émetteurs : `variation_spatiale.poele_insert`, `variation_temporelle.poele_insert` | `poele_ou_insert`, comme la nature de générateur |
  | `appoint: "separe_instantane"` (chauffe-eau solaire individuel) | `appoint: "separe"`, comme `type_pcad: "cesci_appoint_separe"` |
  | statuts `certifiee`, `justifiee` (`variation_temporelle`), `certifiees` (`capteurs.performances`) | `certifie`, `justifie` |
  | `par_defaut` (`capteurs.performances`, `ballon.pertes`, inerties séquentielle et annuelle) | `defaut` |
  | `pivot_par_defaut` (performances d'une PAC) | `pivot_defaut` |
  | `cms_surf_kj_k_m2`, `cma_surf_kj_k_m2`, `cmq_surf_kj_k_m2` | `cms_surf_kj_m2k`, `cma_surf_kj_m2k`, `cmq_surf_kj_m2k` (unité écrite comme `u_k_w_m2k`) |
  | `distributions_intergroupes[].reseau.hydraulique_individuels` | `hydraulique_individuel`, comme `hydraulique_collectif` |

  Un seul jeu de statuts de justification : `certifie`, `justifie`, `declare`, `defaut`. Résultats : `ascenseurs[].n_dem` devient `nb_demarrages`.
- ajouts non incompatibles : `$schema`, `version_format`, `exigences.categorie_contraintes_exterieures: "categorie_3"` (reconnue pour être refusée en habitation) ; `hors_perimetre` dans les problèmes du rapport de validation ;
- contrôles nouveaux : `localisation.alt_m` au plus 4 000 m ; catégorie de contraintes extérieures conforme au chapitre V, partie II, de l'annexe de l'article R. 172-4 (catégorie 3 refusée en habitation ; catégorie 2 refusée sans climatisation de chaque groupe, hors H2d et H3, à 400 m et plus ou en BR1) ; un usage non résidentiel est une erreur hors périmètre (code 3), et non plus un avertissement ;
- le JSON Schema des résultats décrit les résultats tels qu'ils sont écrits (un champ toujours présent y est requis) ; `--horaire`, jamais disponible, est retiré de la ligne de commande.

### Limites connues

- Périmètre habitation seulement.
- Aucune validation contre un cas de référence (COMETH ou RSEE) : les lectures Q8 à Q54 de `docs/questions.md` restent des hypothèses.
- Catégorie de contraintes extérieures et zone de bruit saisies, la catégorie étant contrôlée selon le chapitre V de l'annexe de l'article R. 172-4 ; Icconstruction hors du moteur.
- Pas de lecteur RSEE XML.
- Systèmes refusés : ventilation naturelle, hybride, aération, CTA, puits, échangeur rotatif ; brasseurs d'air ; gestion en alternance, boucles d'eau, DRV, cogénération, PAC au gaz ou hybrides, PAC sur eau ou sol, second ballon, récupération sur eaux grises ; stores vénitiens, gestion automatique des protections ; Titres V.
- Décalage d'une heure possible entre la météo et le calendrier (Q25).
- Sorties non restituées : fiche 13.6, sorties 13.2 par zone et par groupe, éq. (75), consommation par occupant, séries horaires (API Rust seulement).
