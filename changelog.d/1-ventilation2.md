### Ajouté

- Mode actif des centrales de traitement d'air à charge variable (types 3 et 4, Th-BCE 2020 §6.3.3.5, éq. 571 à 614 et 618) : batteries chaude et froide régulées sur la charge (`cta.batterie_chaude`, `cta.batterie_froide`), désormais calculées ; recalcul des débits et du circuit au débit du mode actif, température aux bouches, taux de charge, énergie des batteries avec déshumidification, consommation des ventilateurs ; énergie demandée à la génération désignée par un réseau de CTA fictif (fiche 8.11, éq. 940). Lectures : Q95.
- Champs facultatifs : `cta.generation_batterie_chaude`, `cta.theta_sou_nom_chaud_c`, `cta.generation_batterie_froide`, `cta.theta_sou_nom_froid_c`, `cta.theta_dim_fr_c` ; `centrale` des fonctions de chauffage et de refroidissement d'un émetteur (émetteur contenu dans les bouches de soufflage de la centrale, dont la demande passe à la centrale).
- Résultats facultatifs de `zones[].ventilation.th_d` et `th_c` : `nbh_mode_actif`, `nbh_sous_dimensionnement`, `q_chaud_kwh`, `q_froid_kwh`.
- Exemple `tests/cas/bureaux-cta-dav-ademe` : centrale à débit variable avec batterie chaude (soufflage nominal à 32 °C) alimentée par la chaudière gaz. Météo ADEME : Bbio 94,3, DH 912,1, Cep = Cep,nr 66,8, Ic énergie 257,3, batterie chaude 23 386,9 kWh pour 1 019 h de mode actif ; non conforme (Bbio et Ic énergie). Météo réglementaire H2b : Bbio 82,9, DH 619,5, Cep = Cep,nr 68,9, Ic énergie 293,1, batterie chaude 27 643,8 kWh.
- Questions Q95 (`docs/questions/`).

### Modifié

- `cta.batterie_chaude` et `cta.batterie_froide` à `true` ne sont plus refusés (code 3) : ils exigent la génération, les températures nominales (type 4) et un émetteur contenu dans les bouches (erreurs de saisie, code 2). Les résultats des exemples existants ne changent pas.

### Corrigé

- Variante de lecture `VentilationThD::ConventionnelleBbio` (assemblage provisoire du lot 5, non exposée) : elle retirait aussi le système de ventilation du projet du calcul des consommations, où les ventilateurs d'une centrale double flux hors habitation comptaient alors 0 kWh ; elle ne touche plus que le calcul Th-D. Lecture retenue et résultats inchangés.
- Libellé du point 4 de Q92 (air neuf du mode actif en inoccupation : taux de l'inoccupation appliqué au débit vu par le ventilateur, fuites comprises) ; calcul inchangé.
- Variantes écartées de Q90 point 1 (relance anticipée) et Q93 point 3 (éq. 951 pour l'antigel) conservées, non exposées, et chiffrées sur l'exemple `bureaux-cta-ademe`.
