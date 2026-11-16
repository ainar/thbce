<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Journal des modifications

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le projet le [versionnage sémantique](https://semver.org/lang/fr/). Chaque version indique la version de la méthode Th-BCE implémentée.

## [Non publié]

Méthode : Th-BCE 2020, annexe III de l'arrêté du 4 août 2021, version consolidée au 19 mars 2026.

### Modifié

- **Format d'entrée, changement incompatible** : un masque lointain azimutal se saisit par `gamma_i_deg`, liste de 36 hauteurs d'horizon (une par tranche de 10° repérée par rapport à la normale au composant, annexe III §3.2.3), et non plus par une liste de tranches `{ az_deg, gamma_deg }`. Le JSON Schema (`thbce schema`) est à jour.

- **Format d'entrée, changement incompatible** : `su_m2` (surface utile du groupe, « Agr ») est obligatoire pour tous les usages, y compris en habitation où `shab_m2` reste la surface de référence. `nb_lgt` est obligatoire en logement collectif.

- **Format d'entrée, changement incompatible** : un type de paroi opaque porte un champ obligatoire `incl_k`, position dans laquelle son coefficient `u_k_w_m2k` a été déterminé (`verticale`, `horizontale_flux_ascendant` ou `horizontale_flux_descendant`, annexe III §5.17.3).
- Un pont thermique accepte un champ facultatif `exposition` (description détaillée). Sans lui, la description simplifiée du §5.18.3 s'applique.

### Ajouté

- Workspace Cargo : `thbce-core`, `thbce-data`, `thbce-io`, `thbce-cli`, `thbce-wasm`, `xtask`.
- Types d'entrée du projet (localisation, catalogue, bâtiment, zone, groupe, parois, baies, ponts thermiques, masques, inertie), d'après les nomenclatures de l'annexe III.
- Format d'entrée JSON natif et son JSON Schema.
- Validation de l'entrée avec rapport par élément fautif.
- Lecture du fichier météo au format CSV long ; les données météo ne sont jamais embarquées.
- Exécutable `thbce` : commandes `calcul`, `valider`, `schema`, `version`.
- Module WebAssembly : `calculer`, `valider`, `versionMoteur`, `schemaEntree`.
- Calcul de la surface de référence (§13.1.3) ; les indicateurs réglementaires ne sont pas encore calculés.
- Table département → zone climatique, générée depuis l'annexe I de l'arrêté du 26 octobre 2010.
- Coefficients de pondération dynamique de l'article 11 de l'arrêté.
- Climat extérieur (annexe III §3.1) : classes d'altitude et corrections, température de base par département, éclairements direct et diffus, heure légale, période de confort adaptatif.
- Environnement proche (§3.2) : rayonnement et éclairement sur une paroi inclinée, masques proches verticaux et horizontal, masques lointains par plan vertical, par tranches azimutales et d'arbres à feuilles caduques, rayonnement vers la voûte céleste.
- Lecture du classeur météo réglementaire `.xlsx` et des fichiers ouverts ADEME / Météo-France, avec reconnaissance automatique du format.
- Résultats : bloc `climat` (zone, altitude conventionnelle, température de base, période de confort adaptatif).
- Scénarios conventionnels des 28 usages, convertis du classeur officiel du 29 avril 2026 par script.
- Commande `thbce meteo convertir <classeur.xlsx> -o <fichier.csv>`, qui prépare le fichier météo à charger dans le navigateur.
- Masque lointain d'arbres à feuilles caduques (`lointain_arbres_caducs`).
- Scénarios horaires (annexe III §4.1) pour la maison individuelle et le logement collectif : calendrier des scénarios, consignes, indicateurs d'occupation, de ventilation et d'éclairage, adultes équivalents, apports internes de chaleur et d'humidité par groupe.
- Résultats : `nadeq` par zone.
- Harnais des cas de validation (`tests/cas/`) avec tolérances par indicateur.
- Intégration continue : format, clippy, licences, tests sur Linux, macOS et Windows, build et tests WebAssembly, parité natif / `wasm32-wasip1`.
- Parois opaques et ponts thermiques (§5.15 à §5.20) : coefficient U selon l'inclinaison, flux solaire et flux vers la voûte céleste, cumuls par groupe. Les résultats donnent par groupe les grandeurs d'enveloppe (`enveloppe` : surfaces, longueurs, coefficients HTh).
- Baies vitrées (§5.10 et §5.14) : coefficient U selon l'inclinaison, coefficient Hges selon la position des protections mobiles, flux solaires Fs1, Fs2 et Fs3, flux vers la voûte céleste, flux lumineux par type de baie, seconde protection mobile, cumuls par groupe. Les résultats d'enveloppe donnent les surfaces de baies et `h_ges_sp_w_k`. Les stores vénitiens sont hors de la première version.
- Validation : une baie est inclinée de 0 à 90°, et son type doit donner le coefficient U nécessaire à son inclinaison.
- Valeurs par défaut de la méthode pour les facteurs de baie (§5.10.3.6.3, éq. 215, 216 et 218) : `sw1_c`, `sw2_c`, `sw3_c` et `tlid` deviennent facultatifs. Les résultats listent, par baie, chaque valeur par défaut appliquée et la règle du texte (`baies[].valeurs_par_defaut`).
- Protections mobiles (§5.9 et §5.12) : champ `protection_mobile` sur le type de baie (type, gestion, position, seconde protection, typologie de passage d'air) ; position horaire des volets et stores enroulables en gestion manuelle pour la maison individuelle et le logement collectif, seconde protection mobile, règles du calcul Th-D, surface d'ouverture avec protection en place. Les matrices conventionnelles sont transcrites des figures 35 et 38 publiées au Journal officiel (`docs/reference/arrete/figures/`) et vérifiées cellule par cellule.
- Validation : un store vénitien ou une gestion automatique des protections mobiles sont refusés avec un message explicite ; une protection mobile déclarée demande les facteurs et le coefficient U de la baie protégée.
- Surventilation naturelle par ouverture des baies (§5.13) : champs `ouvrant` sur le type de baie, `expo_br` sur la baie, `traversant` et `h_ttf_m` sur le groupe ; ratio d'ouverture en gestion manuelle, règles du calcul Th-B, surface d'ouverture équivalente avec protection mobile, débit du groupe en traversant et en non traversant. Les résultats de groupe portent `valeurs_par_defaut`.
- Éclairage en habitation (§7.1) : éclairement naturel intérieur des volumes normaux à partir des flux lumineux des baies, coefficient `C2` du résidentiel, consommation conventionnelle, apports récupérables, compteurs d'autonomie en lumière naturelle.
- Modèle thermique de groupe (§5.21) : paramètres d'inertie quotidienne, mélange des flux d'air entrants, bilan à trois nœuds résolu par un schéma de Crank-Nicolson, températures d'air, radiante moyenne et opérative, matrice de comportement. Système de ventilation conventionnel du Bbio (§6.1).
- Saisons propres d'un groupe (§4.6), procédure du calcul du Bbio : autorisations de chauffer et de refroidir décidées chaque jour sur des fenêtres glissantes d'inconfort et de besoins. En calcul du Bbio, l'autorisation de refroidir ne dépend pas de la présence d'un système de refroidissement (éq. 91).
