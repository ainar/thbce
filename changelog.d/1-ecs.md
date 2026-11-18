### Ajouté

- **Accumulateurs d'ECS en eau technique avec échangeur** (lot 1-ecs, fiches 9.11 et 9.12) : échangeur externe à plaques (méthode NUT, circulateur primaire constant ou variable) ou interne, application individuelle (55 °C) ou collective (60 °C), énergie du circulateur comptée dans l'électricité d'ECS. Entrée : champ facultatif `echangeur` dans `stockages[]` ; sortie : champ facultatif `echangeur` des résultats de stockage (UA, consigne, énergie du circulateur). Exemple `tests/cas/restaurant-accumulateur-ademe` (500 L, 60 kW ; météo ADEME : Bbio 272,1, DH 2 675,0, Cep 343,8, Ic 402,0 ; météo réglementaire : Bbio 248,4, DH 1 881,1, Cep 353,9).
- Questions Q105 (plusieurs assemblages de ballons : cause des heures non couvertes), Q106 (lectures de 9.11 et 9.12) et Q107 (refus maintenus des fiches 9.21 et 9.22 sur eau).
- Tests : assertion de refus code 3 (double flux) rétablie dans `restauration.rs`.

### Corrigé

- Code mort `let _ = z;` supprimé de `crates/thbce-io/tests/tertiaire.rs`.

### Modifié

- Restent refusés (code 3) : récupérateur sur les eaux grises (9.21), source sur eau de l'IdCET (9.22), accumulateur sur boucle, appoint séparé et second ballon.
- Format d'entrée compatible (champ ajouté facultatif). 27 exemples conformes, aucun `attendu.json` existant modifié.
