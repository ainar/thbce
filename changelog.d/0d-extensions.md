### Ajouté

- Extensions de bâtiment (fiche 4.4, arrêté du 4 août 2021, articles 3, 50-2 et 50-3 ; article R. 172-1 du CCH ; FA03 v2) : champ facultatif `exigences.extension` (`surelevation`, `sref_existant_m2`, `sref_autres_extensions_m2`) ; l'extension se décrit seule, ses exigences de l'article R. 172-4 sont calculées sur sa `Sref` (Q87) ; nouveau résultat facultatif `indicateurs.exigences.regime_alternatif` (nature, famille d'usage, surface considérée, part de l'existant, régime `complet`, `intermediaire`, `exigences_de_moyens`, `hors_champ` ou `rt2012`, vérifications Bbio et DH du régime intermédiaire avec le `DH_max` de 1 400 °C.h des surélévations climatisées de maisons en H2d et H3, conformité ; Q86, Q88), aussi pour une construction neuve de moins de 50 m². La synthèse de la ligne de commande affiche le régime.
- Contrôles de cohérence des usages (fiches 4.2 et 4.3, FA05 v2), en avertissements : maisons et logements collectifs dans le même bâtiment, bâtiment d'habitation d'au plus deux logements en logements collectifs, maison dans un bâtiment d'usage principal non résidentiel, vestiaires seuls avec un établissement sportif, deux zones du même usage (Q85).
- Exemple `tests/cas/extension-maison-ademe` (extension de 60 m² d'une maison en H2b, régime intermédiaire) ; tests à la main des exigences et des régimes (`crates/thbce-core/tests/extensions.rs`, `crates/thbce-io/tests/extensions.rs`) ; FA03 v2 versionnée avec son extraction texte (`docs/reference/fiches/`, `docs/reference/texte/fa03-extension-construction-inf-50-v2.txt`).
- Questions Q85 à Q89.

### Modifié

- Le régime d'exigences s'applique aux extensions de maisons, de logements collectifs et de bureaux (usages calculés) ; les seuils de l'article 50-3 portent sur la surface cumulée des extensions, les exclusions de l'article R. 172-1, III et l'article 3, II sur la seule partie décrite (Q86).
- Arrondi de restitution des exigences maximales robuste aux erreurs de représentation (arrondi à 1e-9 près d'abord : 55 × 1,15 restitue 63,3 au lieu de 63,2) ; aucun résultat des exemples ne change.

- Format compatible : champs d'entrée et de résultats nouveaux, tous facultatifs ; schéma JSON et types TypeScript régénérés. Les résultats des exemples existants ne changent pas.
