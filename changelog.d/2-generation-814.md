### Ajouté

- Gestion des générateurs en alternance (`priorite: "alternance"`, Th-BCE 2020 §8.17.3.7.2.3, éq. 1035 à 1052) : générations monoposte (refroidissement, ECS, chauffage) et « chauffage et ECS », générateurs sollicités par puissance installée décroissante, compteur de basculement marche/arrêt (`Nbasculement_init` = 20 h, conventionnel) ; vérification de cohérence du montage (même fonction pour tous les générateurs, pas de chauffage et refroidissement, pas de ballon de stockage). Lectures Q125.
- Exemple `tests/cas/maison-chaudieres-alternance-ademe` (deux chaudières gaz à condensation de 12 et 24 kW en alternance, ECS instantanée) ; tests `crates/thbce-io/tests/alternance.rs` (dont un test ignoré sur la météo réglementaire) et tests unitaires calculés à la main.
- Points d'entrée des générateurs des fiches 8.15, 8.16, 8.20, 8.21, 8.24 et 8.25 : aiguillages exhaustifs sur la nature (`nature_hors_perimetre`, `postes_assures`, `fluide_aval_hydraulique`, `resoudre`) et recette « raccorder un générateur » (`docs/architecture.md`, §3.10).

### Modifié

- `priorite: "alternance"` n'est plus refusé (code 3) : une entrée qui l'emploie est désormais calculée, ou refusée en erreur de saisie (code 2) si le montage est incohérent ou si la génération porte un ballon de stockage (§8.17.3.6). Format inchangé.
- Le refus de `pac_hybride` renvoie à la fiche d'application du §16.4 : la PAC hybride se saisit par ses composants, pompe à chaleur et chaudière d'une même génération en cascade (Q126).
