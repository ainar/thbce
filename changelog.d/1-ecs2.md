### Ajouté

- **Récupérateur de chaleur sur les eaux grises** (lot 1-ecs2, fiche 9.21, éq. 2156 à 2224) : récupération individuelle (positions en amont des mitigeurs et du ballon, du ballon seul, des mitigeurs seuls) et collective, efficacité certifiée, justifiée, déclarée ou par défaut (`0,8 × 0,15`, signalée dans `valeurs_par_defaut`), pompe de relevage comptée dans les auxiliaires de distribution. Entrée : champ facultatif `recuperateur` des émetteurs d'ECS et `recuperateurs_eaux_grises` du bâtiment ; sortie : `recuperateur` des résultats d'émetteur et `recuperateurs_eaux_grises` du bâtiment. Le refus code 3 est levé. Exemple `tests/cas/maison-recuperateur-eaux-grises-ademe` (météo ADEME : Bbio 63,8, DH 1 670,1, Cep 76,6, Cep,nr 76,6, Ic énergie 501,9 ; `Qw` 939,8 kWh contre 1 198,7 sans récupérateur ; exigences non satisfaites).
- Question Q108 (lectures de la fiche 9.21 ; en particulier, la chaleur récupérée par le préchauffage de l'eau froide du ballon est déduite de la demande de la génération (positions 0 et 1, −16,0 % et −10,3 % de gaz d'ECS ; variante `Texte` : position 1 sans effet)).
- Lecture `LecturePuisageAccumulateur::Litteral` (Q106) et fonctions de variantes `debit_ecs_moyen_selon`, `ua_echangeur_externe_selon`, `ua_echangeur_interne_selon`, chiffrées dans Q106 ; tests des variantes.

### Corrigé

- Q106 : la citation « §9.10.3.1.3 » pour la conservation des valeurs du dernier puisage est remplacée par une lecture propre (texte p. 1134) ; la mention « inférieur à 1 % » sur la tolérance de l'inversion d'UA est rectifiée (jusqu'à 4,6 % sur l'UA, effet annuel inférieur à 0,01 %) ; éq. 1903 littérale enregistrée.
- Q105 : réserve ajoutée sur l'explication par la phase du thermostat (non démontrée à l'échelle de 123 h et 357 h).
- `docs/couverture.md` : chemins des fiches 9.12 et 9.21 ; refus de 9.11 et 9.12 précisés (appoint séparé, deuxième ballon, couplage chauffage et SSC non représentables dans le format).

### Modifié

- Q107 : le point sur 9.21 est remplacé par Q108 ; le refus de la source sur eau de l'IdCET (9.22) est maintenu.
- Format d'entrée compatible (champs ajoutés facultatifs). Aucun `attendu.json` existant modifié ; exemples habitation inchangés.
