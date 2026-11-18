### Ajouté

- Résultat facultatif `nbh_sous_dimensionnement_consecutives` de `zones[].ventilation.th_d` et `th_c` (plus longue série d'heures consécutives de sous-dimensionnement des batteries régulées sur la charge, éq. 597 et 605) et avertissement non bloquant au-delà de 6 heures consécutives en calcul des consommations.
- Tests de bout en bout du froid d'une centrale à débit variable (type 4, éq. 589 à 593) et du report d'énergie (`crates/thbce-io/tests/cta_mode_actif.rs`).

### Modifié

- Mode actif des centrales à charge variable (Q95, compléments du 18 novembre 2026) : report d'énergie des générations des batteries (`isreport^g(h−1)`, éq. 583, 584, 588, 594 ; débit nominal des centrales de type 3, §6.3.3.1.4) ; demande des émetteurs des bouches que le mode actif n'assure pas transmise à la génération de la batterie (elle était perdue) ; sécurité antigel recalculée aux débits du mode actif et pondérée par le taux de charge (§6.3.3.5.5.4) ; température vue par la génération des batteries après la boîte de mélange recalculée (éq. 951) ; réseaux de CTA fictifs en relance avec les groupes desservis (éq. 938). Exemple `bureaux-cta-dav-ademe` : `q_ag_kwh` 11,0 → 10,4 (Th-C) et 11,7 → 11,2 (Th-D), autres résultats inchangés. Les autres exemples ne changent pas.

### Corrigé

- Dates des compléments du lot 1-ven2 dans Q90, Q92 à Q95 (18 novembre 2026, et non 19).
