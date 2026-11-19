### Ajouté

- Rafraîchissement par surventilation mécanique (fiche 6.12) des systèmes simple flux, basse pression, double flux et centrales de traitement d'air : description facultative `rafraichissement_surventilation` (types 1 à 4, consignes d'été et de mi-saison, puissances des ventilateurs) et débit facultatif `q_spec_rafnoc_m3_h` des bouches ; régulation saisonnière et horaire avec hystérésis sur la température opérative des groupes (éq. 762 à 774), débits sans `Cdep` ni `Crdbnr` (éq. 775), bypass forcé, air neuf à 100 %, antigel, batteries et humidificateurs coupés, mode actif suspendu, surfaces de conduit recalculées (éq. 760, 761) ; Th-C et Th-D. Sortie `nbh_rafnoc` dans `th_c` et `th_d`. Lecture Q230 ; compléments Q90 et Q95.
- Exemples `tests/cas/maison-surventilation-ademe` et `tests/cas/bureaux-cta-surventilation-ademe`, tests `crates/thbce-io/tests/surventilation.rs` (dont un test ignoré sur la météo réglementaire).

### Modifié

- Aération (Q98, complément du 19/11/2026) : la variante « reprises comme air neuf » de la note du §6.2.3.1.1.1 est chiffrée (non exposée) ; le refus d'une zone en aération sans aucune bouche (code 2, règle commune à tous les systèmes) est documenté et testé. Aucun résultat ne change.
- `surventilation_mecanique: true` n'est plus un refus de périmètre (code 3) : sans la description `rafraichissement_surventilation`, c'est une erreur de saisie (code 2). Compatible : aucun résultat existant ne change.
- `surventilation_mecanique: false` avec une description `rafraichissement_surventilation` est une erreur de saisie (« incohérent avec la description »), comme le rafraîchissement évaporatif d'un double flux. Compatible.
- Justification des surfaces de conduit recalculées citant les éq. (760) et (761) dans `valeurs_par_defaut` ; Q230 complétée (citation de la p. 465, tableau 60, variante « demande non servie » chiffrée). Tests de surventilation jamais déclenchée : comparaison de tout le JSON des résultats.
