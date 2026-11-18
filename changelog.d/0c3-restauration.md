### Ajouté

- **Restaurants (usages 13 à 16) et restaurants scolaires (26 et 27) calculés de bout en bout** (lot 0c3) : Bbio, DH, Cep, Cep,nr, Ic énergie, exigences et conformité. Exemples `tests/cas/restaurant-pac-ademe` (usage 13, 400 m², H2b, PAC et plancher chauffant, permis de septembre 2026 ; météo ADEME : Bbio 272,1, DH 2 670,9, Cep 342,8, Ic énergie 400,9 ; météo réglementaire : Bbio 248,4, DH 1 878,7, Cep 352,9, Ic énergie 416,4, Bbio et Cep,nr dépassés) et `tests/cas/restaurant-scolaire-ademe` (usage 26, 600 m², H1b, chaudière gaz, boucle d'ECS arrêtée en vacances ; météo ADEME : Bbio 90,3, DH 32,8, Cep 131,4, Ic énergie 759,5 ; météo réglementaire : Bbio 80,7, DH 14,8, Cep 125,5, Ic énergie 714,4, seul l'Ic énergie dépasse). Restent refusés dans ces usages, comme en bureaux : double flux et centrales de traitement d'air, ascenseurs et parkings.
- Protections mobiles de la famille « restauration » : matrices de gestion manuelle des volets et des stores enroulables (figures 37-1 et 40-1, lues deux fois) et colonne du tableau 34, dont le plafond `min(70 % ; Th-BC)` des stores de nuit (Q78).
- Questions Q78 (protections mobiles de la famille restauration), Q79 (besoin d'ECS de l'usage 16 : 4,7 L/m² du classeur retenu contre 19,5 du tableau 277, tranche Q57) et Q80 (ouverture des usages, tables relues, dates d'effet).

### Modifié

- 7 usages restent refusés (code 3) : commerces, vestiaires, aérogares, industries et sports.
- Format d'entrée inchangé (aucun champ ajouté). Habitation, bureaux, enseignement, hôtels et santé : `attendu.json` inchangés.
