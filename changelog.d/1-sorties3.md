### Ajouté

- Fiche 13.2 : sorties par zone et par groupe (éq. 2412 à 2461), facultatives : `batiments[].sorties_detaillees` (énergie importée et consommée par poste et par énergie, mensuelle et annuelle ; production photovoltaïque attribuée aux zones ; besoins `O_B_*` ; Cef, Cep, Cep,nr), demandées par `calculer_avec` (nouvelle structure `Demandes`), `thbce calcul --sorties-detaillees` ou l'option `sorties_detaillees` du module WebAssembly. Restitution par défaut inchangée. Lectures chiffrées : Q116. Exemple `tests/cas/hotel-pv-sorties-ademe` ; `cas.toml` accepte `sorties_detaillees = true`.

### Modifié

- `calculer_avec_series` est conservée ; elle s'appuie sur `calculer_avec`. Format : champ optionnel ajouté (compatible).
