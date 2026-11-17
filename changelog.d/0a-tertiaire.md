### Ajouté

- **Bureaux (usage 3) calculés de bout en bout** (lot 0a) : Bbio, DH, Cep, Cep,nr et Ic énergie, avec la ventilation simple flux, les émetteurs, distributions, générations et productions d'ECS déjà pris en charge ; exemple `tests/cas/bureaux-simple-flux-ademe` (météo ADEME : Bbio 94,3, DH 1 079,7, Cep 66,0, Ic énergie 315,5 ; météo réglementaire H2b : Bbio 82,9, DH 688,7, Cep 66,8, Ic énergie 349,8). Restent refusés en bureaux : double flux et centrales de traitement d'air, locaux d'éclairage de grand volume, ascenseurs et parkings. Avec les exigences de l'usage 3 (lot 0b), l'exemple porte des `exigences` (permis de 2025, catégorie 1, BR1) : Bbio_max 87,9, Cep_max 82,7, Cep,nr_max 73,0, Ic énergie_max 194,7, DH_max 1 150 ; non conforme sur la météo ADEME (Bbio et Ic énergie dépassés).
- Socle des usages non résidentiels, commun aux 28 usages :
  - fiche 4.1 pour les 28 usages : occupants par local (éq. 38), indicateurs `ihebergement` et `ienseignement` du tableau 4 ; `iecs` (éq. 47) est calculé par les scénarios mais pas encore branché sur les besoins d'ECS (sans effet sur les usages ouverts, qui ne sont pas affiliés à l'enseignement) ;
  - besoins d'ECS hors habitation (fiches 9.5 à 9.7) : tableaux 274, 277 et 279 ; champ facultatif `emetteurs_ecs[].nu` (nombre d'unités, à défaut la surface utile desservie) ;
  - éclairage hors habitation (fiche 7.1) : tableau 78 des usages 3 à 28, coefficients `C1` et `C2` (tableau 79), fractionnement, éclairage mobilier des bureaux, périphériques de gestion (éq. 782 et 787) ; nouveaux `groupes[].locaux[]` (`type_local`, `ratio_local`, `ratio_ecl_nat`, `eclairage`) ;
  - ventilation simple flux en non résidentiel (fiches 6.2 et 6.5) : débits en occupation et en inoccupation (`q_spec_occ_m3_h`, `q_spec_inocc_m3_h`, `crdbnr`), puissances `p_vent_occ_w` et `p_vent_inocc_w` (éq. 383 à 386, 400 à 402, 636 à 639), débits conventionnels du Bbio (tableau 56-1), consommation des ventilateurs sommée selon l'indicateur de ventilation ;
  - protections mobiles : familles et `Pocc`, `Pderog` des 28 usages (figure 34), matrices de la famille « bureau » (figures 36 et 39, tableau 34) ;
  - caractère traversant conventionnel des 28 usages (tableau 43) ;
  - facteur d'émission de l'éclairage « dans un bâtiment tertiaire » (article 10) dans l'Ic énergie.
- Questions Q55 à Q63 (`docs/questions/`).

### Modifié

- Le message de refus d'un usage cite désormais les bureaux parmi les usages calculés ; les 25 autres usages restent refusés (code 3).
- Format d'entrée, sans incompatibilité : `p_vent_base_w`, `p_vent_pointe_w` (ventilateur simple flux), `q_spec_conv_pointe_m3_h` (bouche) et `nb_logements` (émetteur d'ECS) ne sont plus requis par le schéma ; la validation les exige toujours en maison individuelle ou accolée et en logement collectif, et les refuse dans les autres usages.
- Un bâtiment dont une zone n'a pas d'exigences pour l'année de dépôt du permis n'a ni exigences maximales ni conformité (avertissement), au lieu d'un maximum calculé sans cette zone.
- `zones[].nadeq` n'est restitué qu'en habitation.
