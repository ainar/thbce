### Ajouté

- Fiche 13.6 : indicateurs par occupant (`indicateurs.occupants` : `Nocc` des zones et du bâtiment, éq. 2552, Cep par occupant, éq. 2553, Ic énergie par occupant). Saisie facultative `zones[].types_logements` (pièces principales, tableau 342 et éq. 131) et `zones[].nb_occupants` ; en tertiaire hors enseignement, nombre maximal d'occupants des scénarios signalé dans `valeurs_par_defaut` (Q117).
- Ic énergie annuel (annexe II, éq. 75) : `indicateurs.ic_energie_annuel_kgco2_m2`, 50 valeurs de `a = 1` à `50` (Q117).
- Exemple `tests/cas/immeuble-collectif-occupants-ademe`.

### Modifié

- Stores vénitiens en Th-D : les ratios du tableau 34 (90 % en occupation, 70 % en inoccupation) ne s'appliquent que si `Topj-1max ≥ 26,5 °C`, comme la désactivation de PM2 (préambule du §5.9.3.6, Q103 complété le 19 novembre 2026). Seul le DH de `tests/cas/bureaux-stores-venitiens-ademe` change (1 750,2 → 1 748,1 °C.h sur la météo ADEME, −0,12 %) ; aucun exemple d'habitation n'est touché.
