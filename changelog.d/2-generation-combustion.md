### Ajouté

- Fiche 8.20 (lot 2e) : autres générateurs à combustion au gaz naturel ou au GPL, types 103 à 109 (radiateurs, poêles et inserts, chauffe-eau, accumulateurs, générateurs d'air chaud, tubes et panneaux radiants) : valeurs par défaut des tableaux 126 à 128, statut des rendements, pertes à charge nulle, chauffage, ECS, arrêt selon le raccordement (`raccordement`, « idraccord_gnr », en cascade ou en alternance), pertes récupérables ; lectures Q155 et Q156. Exemples `maison-radiateurs-gaz-ademe` et `gymnase-air-chaud-ademe`, tests `crates/thbce-io/tests/autres_combustion.rs`.
- Fiche 8.24 : le « Dfou0 » de 26 min du mode ECS des machines à absorption (repris du tableau 216, Q135) est signalé dans `valeurs_par_defaut`.

### Modifié

- Format : la nature `autre_generateur_a_combustion`, jusqu'ici une chaîne refusée en code 3, porte désormais ses données (`{"autre_generateur_a_combustion": {...}}`). Sans effet sur les entrées valides, puisqu'elle était refusée.
