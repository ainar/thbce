### Ajouté

- **Gestion automatique des protections mobiles** (fiche 5.9, lot 1-env) : volets et stores enroulables à gestion automatique avec dérogation (`partGPM = Pocc·Pderog`, hystérésis de la figure 31, horloges crépusculaire et personnalisable, limite de vent des stores enroulables, matrices conventionnelles des figures 44 et 45 signalées dans les résultats). Données de dérogation des six familles (figures 35 à 40-3, lues une fois).
- **Stores vénitiens** (fiches 5.9 et 5.10) : gestion manuelle, suivi du soleil, automatique à 45°, matrices manuelles des six familles (figures 41 à 43-3) et automatiques conventionnelles (figures 46 et 47), tableau 34 en Th-D (90 % et 70 %) ; facteurs des lames saisis (jamais inventés), interpolation linéaire en `γ'`.
- Exemples `tests/cas/bureaux-volets-auto-ademe` (ADEME : Bbio 91,5, DH 697,1, Cep 59,2, Ic énergie 285,5 ; réglementaire : Bbio 78,5, DH 510,1, Cep 61,0, Ic énergie 314,0) et `tests/cas/bureaux-stores-venitiens-ademe` (facteurs fictifs ; ADEME : Bbio 111,8, DH 1 750,2, Cep 63,1, Ic énergie 288,5 ; réglementaire : Bbio 95,8, DH 988,8, Cep 65,3, Ic énergie 318,7).
- Questions Q100 à Q104 (gestion automatique, stores vénitiens, interpolation, Th-D, FA01 et point Q72).

### Modifié

- Format d'entrée, sans incompatibilité : `protection_mobile.gestion` devient facultatif ; champs facultatifs `gestion_sv`, `detecteur_presence`, `automatisme` et `store_venitien` (schéma et types TypeScript régénérés). Les entrées existantes ne changent pas de résultat (habitation et bureaux : `attendu.json` inchangés).
- Les tests de refus (cœur, CLI, WASM, tertiaire) s'appuient sur une cogénération (§8.21), toujours refusée, au lieu du store vénitien.
