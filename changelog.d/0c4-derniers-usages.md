### Ajouté

- **Commerces (17), vestiaires seuls (18), aérogares (22), industries ou artisanats (23 et 24) et établissements sportifs (25 et 28) calculés de bout en bout** (lot 0c4) : Bbio, DH, Cep, Cep,nr, Ic énergie, exigences et conformité. Les 28 usages sont désormais calculés. Exemples `tests/cas/commerce-pac-ademe` (usage 17, 800 m², H2b, grand volume éclairé par des vitrines et des éclairants de toiture, PAC et plancher chauffant, permis de septembre 2026 ; météo ADEME : Bbio 164,7, DH 2 935,7, Cep 124,6, Ic énergie 145,2 ; météo réglementaire : Bbio 148,6, DH 1 938,5, Cep 122,0, Ic énergie 144,1, conforme) et `tests/cas/gymnase-pac-ademe` (usage 25, 900 m², H1b ; météo ADEME : Bbio 128,2, DH 3 374,8, Cep 148,1, Ic énergie 167,9 ; météo réglementaire : Bbio 100,4, DH 2 262,7, Cep 143,0, Ic énergie 161,7, Cep, Cep,nr et DH dépassés). Restent refusés, comme dans les autres usages non résidentiels : double flux et centrales de traitement d'air, ascenseurs et parkings.
- **Grands volumes d'éclairage** (salle de sport, aire de production, aire de vente de plus de 300 m²) : éclairement naturel `Einat(0) + Einat(1)` (éq. 783), coefficients `K` propres et constantes `*_gv` (Q81). Nouveau champ facultatif de la baie `affectation_eclairage` (`volume_normal`, `grand_volume`, `grand_volume_toiture_uniforme`) ; contrôles de cohérence avec les locaux du groupe.
- **Éclairage d'accentuation des commerces** : puissance de l'aire de vente et du petit magasin relevée à 4,5 W/m² au moins en Th-C et Th-D (§7.1.3.2.2, Q82).
- **Protections mobiles, famille « autres usages »** : matrices des figures 37-3 et 40-3 et colonne du tableau 34, avec un plancher de jour propre à l'inoccupation (Q84).
- Questions Q81 (grands volumes), Q82 (accentuation, locaux des usages 18, 22 et 25), Q83 (indicateurs fractionnaires des usages 17, 23 et 24 : lecture de Q74 étendue, ventilation lue comme occupation) et Q84 (ouverture des usages, famille « autres », dates d'effet).

### Modifié

- Plus aucun usage n'est refusé (code 3) : les tests de refus emploient un store vénitien ou un double flux en usage non résidentiel.
- Format d'entrée : un champ facultatif ajouté à la baie (`affectation_eclairage`), compatible. Habitation, bureaux, enseignement, hôtels, santé et restauration : `attendu.json` inchangés.
