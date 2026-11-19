### Ajouté

- Aération par ouverture des fenêtres (fiche 6.7, `systeme: "aeration"`), en habitation et hors habitation : air neuf fourni par les bouches de soufflage du groupe (éq. 162), débit repris opposé (éq. 163), air à la température et à l'humidité extérieures (éq. 716 et 718) ou à celle de la sortie d'un espace tampon (éq. 715), un seul débit entrant par les baies, `max(surventilation ; aération)` (éq. 256) ; `Cdep = Cfenb = 1,7`, `Crdbnr = 1`, `Kres = 0` imposés (§6.2) ; aucun ventilateur. Lectures Q98 et Q99.
- Champ facultatif `aeration_espace_tampon` du groupe (identifiant d'un espace tampon, réservé à l'aération).
- Exemple `tests/cas/maison-aeration-ademe` et tests `crates/thbce-io/tests/aeration.rs` (dont un test ignoré sur la météo réglementaire).

### Modifié

- L'aération (`aeration`) n'est plus un refus ciblé (code 3) : elle est calculée ; une zone en aération sans bouche de soufflage est une erreur de saisie (code 2). Compatible : aucun résultat existant ne change.
