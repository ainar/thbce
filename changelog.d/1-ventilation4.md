### Modifié

- En habitation, `humidification: true` ou `rafraichissement_evaporatif: true` sans description restent un refus ciblé (code 3), au lieu d'une erreur de saisie (code 2) suivie d'un refus une fois l'équipement décrit (reprise de relecture du lot 1-ven3).
- Profil à hystérésis du rafraîchissement évaporatif d'une centrale à débit variable : la borne basse est le débit de zone neutre en occupation `Qv_sou_ZN,occ` et la puissance `Pvent,ZN,occ`, y compris en relance (éq. 426 à 428, 466, 467 et figure 73), et non celle de l'inoccupation. Aucun exemple ni valeur phare ne change (Q96, complément du 19 novembre 2026).

### Corrigé

- Q96 point 4 : l'affirmation que la figure 73 contredit l'éq. (426) était fausse (rampe de descente de largeur Δθi2, cohérente avec `θhum_3 = θi_base + Δθi2`) ; Q96 points 3 et 8 complétés (variante `max(θi_base ; 22 °C)`, lecture de l'éq. 616).
