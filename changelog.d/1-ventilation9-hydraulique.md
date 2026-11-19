### Ajouté

- Puits hydraulique, ou à eau glycolée (fiche 6.9 et prétraitement du §6.3.3.3.4) : nouveau champ facultatif `puits_hydraulique` d'une ventilation (`puits = "hydraulique"` reste le champ de déclaration) pour un simple flux par insufflation, un double flux ou une centrale de traitement d'air ; sol (éq. 732 à 734), capteur enterré (éq. 743 à 745), batterie eau/air (puissances `batterie.p_ch_kw` et `p_fr_kw`, éq. 399 à 407, ou 0,65 par défaut, éq. 408), gestion de la pompe (éq. 746 et 747), `θan` de la centrale (éq. 415 à 417, 489 à 491), pompe comptée dans les auxiliaires de ventilation (éq. 748 et 749). Résultat `puits_hydraulique` (`nbh_actif`, `q_prechauffage_kwh`, `q_prerefroidissement_kwh`, `w_pompe_kwh`) dans `th_d` et `th_c`. Lectures : Q233. `q_max_m3_h` se saisit en m³/h (le tableau 71 écrit m³/s, incompatible avec les éq. 737, 743 et 745).
- Exemples `tests/cas/maison-puits-hydraulique-ademe` et `tests/cas/bureaux-cta-puits-hydraulique-ademe`, tests `crates/thbce-io/tests/puits_hydraulique.rs` (dont un test ignoré sur la météo réglementaire).

### Modifié

- `puits = "hydraulique"` n'est plus un refus de périmètre (code 3) : sans `puits_hydraulique`, c'est une erreur de saisie (code 2) ; avec un puits climatique décrit, aussi (éq. 386). Une puissance de batterie incompatible avec le débit d'air neuf de référence (efficacité supérieure à 1) est une erreur de saisie.
