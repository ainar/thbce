### Modifié

- Ventilation hybride (fiche 6.6, Q236 point 2) : les créneaux de grand débit du tableau 67 se lisent comme le texte les écrit, `hleg(h) ∈ [hgd,deb ; hgd,fin]` bornes incluses, avec `hleg` en case de scénario de 1 à 24 (échelle de `hgd`, p. 414) ; auparavant `[début ; fin[` sur le début de pas de 0 à 23, non équivalent. Les pas de grand débit par créneau passent de 1 à 2 en maison et de 2 à 3 en logement collectif. Seules les entrées à ventilation hybride sont concernées (système introduit au lot 1-ven10, jusqu'ici refusé) : aucun exemple d'habitation à système mécanique ne bouge ; l'attendu de `maison-ventilation-hybride-ademe` est régénéré (Cep 111,7 → 114,8 kWhep/(m².an), DH 1 573,0 → 1 573,6 °C.h).

### Documentation

- Q234 à Q238 : pages exactes de la fiche 6.6 ; éq. (689) lue comme coquille (même rapport que l'éq. 688) avec effet chiffré, tableau des vitesses d'essai de l'extracteur (p. 420), renvoi des fuites (éq. 675 et 678), note de l'éq. (707), mention du Bbio p. 412 (Q36 maintenue) ; anomalies consignées dans `docs/reference/index/anomalies-constatees.md`.
