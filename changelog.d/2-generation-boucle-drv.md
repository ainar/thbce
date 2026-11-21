### Ajouté

- Fiche 8.15 (lot 2f) : boucles d'eau. Nouvelle nature de générateur `pac_sur_boucle_d_eau` (type 507 : PAC eau de boucle / air recyclé et eau / air, associée à un groupe, source amont `boucle_d_eau`) et description de la boucle `generations[].boucle_d_eau` (`theta_max_rech_c`, `theta_min_fr_c`, tour de refroidissement facultative). Calculés : mode et charge des PAC de chaque groupe selon sa demande et ses saisons (éq. 957 à 964), bilan de la boucle et transfert entre PAC (éq. 965 à 968), générateur de chauffage (éq. 969 à 971), tour (éq. 972 à 974) ou générateur de froid (éq. 975 à 977) de la boucle, consommations des PAC attribuées à leur groupe (éq. 1490 et 1491) ; sortie `generations[].boucle_d_eau` ; lectures Q240 à Q242. La source amont « boucle d'eau » à température sinusoïdale du lot 2d (Q149) est la température de la boucle. Exemple `hotel-boucle-d-eau-ademe`, tests `crates/thbce-io/tests/boucle_d_eau.rs`.

### Modifié

- La nature `boucle_d_eau_ou_drv` ne désigne plus que les thermo-frigo-pompes et les DRV (types 508 et 509, fiche 8.16), toujours refusés (code 3). Les tests de refus en code 3 emploient désormais la PAC hybride saisie en un seul générateur (`pac_hybride`, Q126).
- Une génération à boucle d'eau qui produit de l'ECS, ou désignée par une centrale de traitement d'air ou une sécurité antigel, est refusée (code 3). Une PAC électrique (`thermodynamique_electrique`) sur `boucle_d_eau` hors d'une génération à boucle reste calculée sans bilan de boucle, comme au lot 2d (Q240 point 2).
