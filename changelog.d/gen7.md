### Ajouté

- Pompes à chaleur électriques triple service (chauffage, refroidissement et ECS ; fiche 8.23, tableau 149) : ECS prioritaire à pleine charge, refroidissement sur le temps restant diminué du délai de basculement de 15 min (éq. 1294), chauffage exclu dès qu'il y a une demande de froid, puissance absorbée à charge nulle comptée dans un seul mode selon la saison (éq. 1326). Nouvelle fonction de générateur « 6 : chauffage, refroidissement et ECS ». Question Q250.
- Exemple `tests/cas/maison-pac-triple-service-ademe` et test d'intégration `crates/thbce-io/tests/triple_service.rs` (valeurs phares sur la météo réglementaire).

### Modifié

- Un générateur thermodynamique électrique qui produit du froid et de l'ECS n'est plus refusé (code 3) : il est validé contre les quatre combinaisons du tableau 149 (erreur de saisie sinon). Aucun résultat existant ne change.
