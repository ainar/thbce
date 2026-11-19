### Ajouté

- Puits climatique ou canadien (fiches 6.8, 6.10 et 6.11) : nouveau champ facultatif `puits_climatique` d'une ventilation (`puits = "climatique"` reste le champ de déclaration) pour un simple flux par insufflation, un double flux ou une centrale de traitement d'air ; température du sol (éq. 719 à 721, tableau 70), efficacité de l'échange sol-air (éq. 723 à 727), température de sortie (éq. 728, 731), bypass selon les températures et la saison de chauffage (éq. 750 à 752, 755), `θan` des éq. 414, 488 et 658 reprise par le double flux, la centrale (régulation du taux d'air neuf, échangeur, antigel, batteries) et la sécurité antigel des réseaux de CTA (éq. 952) ; Th-C et Th-D, pas le Bbio (éq. 756, 757). Sortie `puits_climatique` (`nbh_actif`, `q_prechauffage_kwh`, `q_prerefroidissement_kwh`) dans `th_c` et `th_d`. Lecture Q231.
- Exemples `tests/cas/maison-puits-climatique-ademe` et `tests/cas/bureaux-cta-puits-climatique-ademe`, tests `crates/thbce-io/tests/puits_climatique.rs` (dont un test ignoré sur la météo réglementaire).

### Modifié

- `puits = "climatique"` n'est plus un refus de périmètre (code 3) : sans `puits_climatique`, c'est une erreur de saisie (code 2) ; sur un système qui n'a pas d'air neuf soufflé (simple flux par extraction, basse pression, aération), c'est aussi une erreur de saisie. Le puits hydraulique reste refusé (code 3) et, avec un puits climatique décrit, l'exception de l'éq. 386 est une erreur de saisie. Compatible : aucun résultat existant ne change.
