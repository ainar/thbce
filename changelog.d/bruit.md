### Ajouté

- Classement au bruit des baies (fiche d'application 5.11, annexe de l'article R. 172-4, chapitre V, partie I) : saisie facultative baie par baie (`baies[].bruit` : infrastructures de catégorie 1 à 5 avec distance et vue, zone A à D ou hors zone du plan d'exposition au bruit, façade), classe de chaque baie lue dans les tableaux (configurations 1 et 2, classe la plus défavorable, règle de la façade), puis classe d'un groupe ou d'une zone (point 4). La classe déduite remplace la saisie absente pour `Cpr`, `Mbbruit` et les catégories de contraintes extérieures ; si la saisie directe est aussi donnée, la validation en contrôle la cohérence (`Q200.md` à `Q204.md`). La catégorie des voies et le plan d'exposition au bruit restent des données à saisir.

### Modifié

- `exigences.zone_bruit` devient facultatif (changement compatible du format : les entrées existantes restent valides ; en Rust, le champ `ParametresExigences::zone_bruit` passe de `ZoneBruit` à `Option<ZoneBruit>`). Absent et non déductible des baies : erreur de validation.
