### Ajouté

- **Hôtels 0 à 2 et 3 à 5 étoiles en parties nuit (usages 8 et 9) et jour (10 et 11), établissements sanitaires avec hébergement (19), établissements de santé en parties nuit (20) et jour (21) calculés de bout en bout** (lot 0c2) : Bbio, DH, Cep, Cep,nr, Ic énergie, exigences et conformité. Exemples `tests/cas/hotel-pac-ademe` (usages 8 et 10 en deux zones, 1 200 m², H2b, PAC et plancher chauffant, permis de septembre 2026 ; météo ADEME : Bbio 108,3, DH 1 492,2, Cep 184,0, Ic énergie 216,8 ; météo réglementaire : Bbio 103,6, DH 1 253,6, Cep 193,4, Ic énergie 229,6, Bbio, Cep, Cep,nr et DH dépassés) et `tests/cas/ehpad-pac-ademe` (usage 19, 1 500 m² ; météo ADEME : Bbio 167,8, DH 1 966,8, Cep 178,1, Ic énergie 215,6 ; météo réglementaire : Bbio 160,2, DH 1 310,1, Cep 188,8, Ic énergie 230,0, seul le DH dépasse). Restent refusés dans ces usages, comme en bureaux : double flux et centrales de traitement d'air, ascenseurs et parkings.
- Protections mobiles de la famille « hôpitaux nuit » (usage 20) : matrices de gestion manuelle des volets et des stores enroulables (figures 37-2 et 40-2, lues deux fois) et colonne du tableau 34, dont le plafond `min(50 % ; Th-BC)` des stores de nuit. Les familles « habitation » et « bureau » servent aux autres usages du lot.
- Indicateur d'éclairage fractionnaire de l'usage 20 (0,35 la nuit dans le classeur) : `CTRLecl_occ = C1 × IEcl` (Q74) ; identique à l'éq. (788) pour 0 et 1.
- Questions Q74 (éclairage fractionnaire de l'usage 20 ; tranche Q27 et Q63 pour cet usage), Q75 (protections mobiles de la famille hôpitaux), Q76 (éclairage des chambres : pas d'éclairage conventionnel) et Q77 (ouverture des usages, tables relues, dates d'effet).

### Modifié

- 13 usages restent refusés (code 3) : restauration, commerces, vestiaires, aérogares, industries et sports.
- Format d'entrée inchangé (aucun champ ajouté). Habitation, bureaux et enseignement : `attendu.json` inchangés.
