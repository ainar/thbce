### Ajouté

- Résultat facultatif `nbh_sous_dimensionnement_consecutives` de `zones[].ventilation.th_d` et `th_c` (plus longue série d'heures consécutives de sous-dimensionnement des batteries régulées sur la charge, éq. 597 et 605) et avertissement non bloquant au-delà de 6 heures consécutives en calcul des consommations.
- Tests de bout en bout du froid d'une centrale à débit variable (type 4, éq. 589 à 593) et du report d'énergie (`crates/thbce-io/tests/cta_mode_actif.rs`).

- Humidificateur d'hiver à vapeur ou à pulvérisation (centrales de types 1 et 2, §6.3.3.4.9.2, éq. 526 à 537, 568, 569 ; énergie d'un humidificateur à vapeur demandée à la génération désignée, réseau `idchaudHR` de l'éq. 939) et rafraîchissement évaporatif direct ou indirect (§6.3.3.4.2, 6.3.3.4.5.3, 6.3.3.4.11 : procédure FADIAB, éq. 476 à 486, 549 à 559 ; pas de recyclage ni de bypass en indirect, éq. 440 et 493 ; débits et ventilateurs à hystérésis des centrales à débit variable, éq. 426 à 428, 449, 450, 466, 467 ; pompes, éq. 566, 567, 615 à 617), calculés hors habitation. Lectures : Q96, Q97.
- Champs facultatifs `humidificateur` et `rafraichissement_adiabatique` des systèmes `double_flux` et `cta` ; résultats facultatifs `q_hr_kwh`, `w_aux_chaudhr_kwh`, `nbh_adiab`, `w_aux_adiab_kwh` de `zones[].ventilation.th_d` et `th_c`.
- Exemple `tests/cas/bureaux-cta-humidification-ademe` : centrale de type 2 avec humidificateur à vapeur et rafraîchissement indirect. Météo ADEME : Bbio 94,3, DH 496,2, Cep = Cep,nr 76,0, Ic énergie 314,6, humidification 6 295,8 kWh ; non conforme (Bbio, Cep,nr, Ic énergie). Météo réglementaire H2b : Bbio 82,9, DH 307,3, Cep = Cep,nr 81,7, Ic énergie 373,5, humidification 8 799,2 kWh.

### Modifié

- Mode actif des centrales à charge variable (Q95, compléments du 18 novembre 2026) : report d'énergie des générations des batteries (`isreport^g(h−1)`, éq. 583, 584, 588, 594 ; débit nominal des centrales de type 3, §6.3.3.1.4) ; demande des émetteurs des bouches que le mode actif n'assure pas transmise à la génération de la batterie (elle était perdue) ; sécurité antigel recalculée aux débits du mode actif et pondérée par le taux de charge (§6.3.3.5.5.4) ; température vue par la génération des batteries après la boîte de mélange recalculée (éq. 951) ; réseaux de CTA fictifs en relance avec les groupes desservis (éq. 938). Exemple `bureaux-cta-dav-ademe` : `q_ag_kwh` 11,0 → 10,4 (Th-C) et 11,7 → 11,2 (Th-D), autres résultats inchangés. Les autres exemples ne changent pas.

- `humidification: true` et `rafraichissement_evaporatif: true` sans description ne sont plus refusés en code 3 mais en erreur de saisie (code 2) : décrire l'équipement par `humidificateur` ou `rafraichissement_adiabatique`. En habitation, la description reste refusée (code 3).

### Corrigé

- Dates des compléments du lot 1-ven2 dans Q90, Q92 à Q95 (18 novembre 2026, et non 19).
