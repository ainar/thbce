### Ajouté

- Générateurs thermodynamiques au gaz (type 504), nature `thermodynamique_gaz` : machines à absorption de la fiche 8.24 (`absorption` : GAHP air/eau classique et haute température en chauffage, GAHP air/eau en ECS base d'un ballon, refroidisseur air/eau ; un mode par machine), et pompes à chaleur à moteur gaz de la fiche 8.25 (`moteur_gaz` : air extérieur / eau et air extérieur / air recyclé, chauffage et refroidissement). Matrices de GUE, de puissance gaz et d'auxiliaires construites autour du pivot (corrections de statut, coefficients `Cnn` des figures et tableaux 217 à 242), charge partielle (éq. 1339 à 1400), valeurs par défaut signalées dans `valeurs_par_defaut` ; consommation en gaz naturel, auxiliaires en électricité.
- Questions Q135 à Q144 (lectures des fiches 8.24 et 8.25), anomalies du texte consignées.
- Exemples `tests/cas/maison-gahp-ademe`, `tests/cas/maison-pac-moteur-gaz-air-air-ademe` et `tests/cas/commerce-pac-moteur-gaz-ademe` ; test d'intégration `crates/thbce-io/tests/generation_gaz.rs` (valeurs phares sur la météo réglementaire).

### Modifié

- Format : la variante `thermodynamique_gaz` de `nature`, jusqu'ici sans données et refusée (code 3), porte désormais sa saisie (`{"thermodynamique_gaz": {"absorption": {…}}}` ou `{"moteur_gaz": {…}}`) : changement de forme sans effet sur les entrées valides, puisque la variante était refusée. Restent refusées (code 3) : machines à absorption sur eau glycolée, sur eau ou sur eau de tour, et production indirecte d'ECS des PAC à moteur gaz (`production_ecs_indirecte`).
- Un générateur thermodynamique au gaz base d'un ballon d'ECS demande, comme une pompe à chaleur électrique, la hauteur relative de l'échangeur (`base.h_rel_ech`).
