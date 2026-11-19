### Ajouté

- Ventilation naturelle et hybride par conduits (fiche 6.6, éq. 673 à 714, tableaux 64 à 67 ; Q234 à Q239) : `systeme.naturelle_par_conduits` et `systeme.hybride` d'une ventilation d'habitation, calculés en Th-C et Th-D pour un groupe desservi (plusieurs : code 3). Bilan des pressions du conduit résolu par dichotomie, bouche fixe ou autoréglable, extracteur statique, tirage thermique avec `Pib(h−1)`, assistance mécanique de l'hybride, restitution `naturelle` dans `th_d` et `th_c`. Exemples `tests/cas/maison-ventilation-naturelle-ademe` et `tests/cas/maison-ventilation-hybride-ademe`.
- `docs/reference/index/anomalies-constatees.md` : éq. (689), (701), (704) et (707) de la fiche 6.6.

### Modifié

- Format d'entrée : les variantes `naturelle_par_conduits` et `hybride` de `systeme`, sans contenu et refusées jusqu'ici, portent désormais une description (une entrée qui les employait était refusée : aucune entrée valide ne change). Aucun `attendu.json` existant ne change.
