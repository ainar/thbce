### Ajouté

- Champ facultatif `id_puits_climatique` d'une ventilation (`Id_Puits_Climatique`, fiches 6.8 et 6.11) : un puits partagé entre plusieurs centrales n'est pas pris en charge, le champ est refusé en validation avec le code 3 (message explicite, Q232). Aucune saisie existante n'est touchée.
- `docs/reference/index/anomalies.md` : section « Incohérences du texte » (éq. 726 : facteur ×3 600 incohérent avec la nomenclature du tableau 69 ; éq. 730), générée par `xtask index-texte`.

### Modifié

- Q231 : complément (ordre de grandeur de `hi` en lecture littérale de l'éq. 726, ajout de la centrale de traitement d'air à la fiche 6.11) ; renvois « point 1 » et « point 2 » inversés corrigés dans `ventilation/puits_climatique.rs`.
- Puits hydraulique : avertissement de validation non bloquant quand le débit calorifique de l'air neuf dépasse celui de l'eau de la boucle (éq. 415 lue à la lettre, Q233) ; anomalies de l'éq. 747 et du tableau 71 consignées.
