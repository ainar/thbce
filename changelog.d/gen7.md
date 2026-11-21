### Ajouté

- Pompes à chaleur électriques triple service (chauffage, refroidissement et ECS ; fiche 8.23, tableau 149) : ECS prioritaire à pleine charge, refroidissement sur le temps restant diminué du délai de basculement de 15 min (éq. 1294), chauffage exclu dès qu'il y a une demande de froid, puissance absorbée à charge nulle comptée dans un seul mode selon la saison (éq. 1326). Nouvelle fonction de générateur « 6 : chauffage, refroidissement et ECS ». Question Q250.
- Exemple `tests/cas/maison-pac-triple-service-ademe` et test d'intégration `crates/thbce-io/tests/triple_service.rs` (valeurs phares sur la météo réglementaire).
- Chauffage par défaut des travaux réservés par l'acquéreur (VEFA, fiche 8.33) : champ facultatif `chauffage_par_defaut_vefa` du bâtiment (puissance et groupes desservis) ; le moteur ajoute les composants figés des tableaux 261 à 266 (générateur à effet Joule, réseaux fictifs, convecteur à soufflage d'air chaud de classe C). Question Q251, anomalies du texte consignées. Exemple `tests/cas/immeuble-collectif-vefa-ademe`, test `crates/thbce-io/tests/vefa.rs`.

- Échangeur de géocooling d'une génération de froid (fiche 8.30) : nature de générateur `geocooling`, source géothermique sur eau (sondes ou nappe), associations « base et complément » et « alterné » avec le générateur de froid qui le complète (génération en cascade, géocooling au premier rang). Question Q252 (dont la description du raccordement à faire pour la fiche 8.31, non traitée). Exemple `tests/cas/maison-geocooling-ademe`, test `crates/thbce-io/tests/geocooling.rs`.

### Modifié

- Un générateur thermodynamique électrique qui produit du froid et de l'ECS n'est plus refusé (code 3) : il est validé contre les quatre combinaisons du tableau 149 (erreur de saisie sinon). Aucun résultat existant ne change.
