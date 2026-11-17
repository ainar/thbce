### Ajouté

- Exigences maximales des usages 3 à 28 (annexe de l'article R. 172-4, chapitre III : `Bbio_maxmoyen`, `Cep_maxmoyen`, `Cep,nr_maxmoyen`, `Icénergie_maxmoyen` par période et selon le raccordement à un réseau de chaleur, `Mbgéo`, `Mbsurf_tot`, `Mbbruit`, `Mcgéo`, `Mcsurf_tot`, `Mccat`, `DH_maxcat`), relues sur l'image des pages et protégées par des tests de contrôle (`docs/questions/Q065.md`). Les zones de ces usages restent refusées par la validation tant que leur calcul n'est pas ouvert ; la fonction d'exigences est testée sur des calculs à la main (bureaux, enseignement, hôtel, commerce, santé, industrie).
- Catégories de contraintes extérieures 1 à 3 et classe de bruit par zone (`exigences.zones[]`), catégorie par groupe (`exigences.zones[].groupes[]`), contrôlées selon le chapitre V, partie II pour tous les usages (`Q066.md`) ; catégorie 3 : `Mbbruit` et `Mccat` de la ligne « Catégorie 3 », DH sans seuil. Résultats : `exigences.zones[].categorie_contraintes_exterieures` et `zone_bruit`.
- Dates d'effet par usage (1er juillet 2022 pour les bureaux et l'enseignement, 1er mai 2026 pour les usages 6 à 28) et mois de dépôt facultatif `exigences.mois_depot_permis` (`Q068.md`) ; bâtiments à plusieurs usages au prorata des `Sref` (`Q067.md`).
- Références versionnées : chapitres I à III de l'annexe de l'article R. 172-4 dans leur version post-GTM2 (permis du 1er mai au 30 juin 2026), fiches d'application FA05 v2 (usage d'un bâtiment) et FA09 v1 (exclusions du tertiaire spécifique et de l'industrie).

### Modifié

- `MbHSP` et `McHSP` ne s'appliquent qu'aux permis déposés à partir du 1er juillet 2026 (décret n° 2026-200, `Q069.md`) ; sans effet sur les exemples (hauteur sous plafond de 2,5 m).
- `exigences.hsp_moy_m` devient facultatif : obligatoire en habitation pour un permis déposé à partir du 1er juillet 2026 (une année 2026 sans mois compte comme telle). Changement compatible pour les entrées.
- API Rust : `indicateurs::exigences::dh_max` renvoie un `SeuilDh` (« Pas de seuil » en catégorie 3) ; `ParametresExigences` n'est plus `Copy` (il contient des listes).
- Entrée plus stricte : `exigences.mois_depot_permis` est obligatoire pour un permis déposé en 2026, et en 2022 avec une zone de bureaux ou d'enseignement (`Q068.md`). Un fichier d'entrée daté de 2026 sans mois est désormais refusé (code 2).
