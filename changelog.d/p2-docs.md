### Modifié

- Documentation de travail : `docs/questions.md` est découpé en un fichier par question (`docs/questions/Q<nnn>.md`, en-tête `numero`, `sujet`, `statut`) ; le tableau de bord `docs/questions/ETAT.md` est généré (`cargo run -p xtask -- questions`) et contrôlé par `cargo run -p xtask -- verifier`. Les renvois du code et des documents sont mis à jour. Les textes des questions sont inchangés ; aucun résultat de calcul ne change.
- Le journal des modifications s'écrit par fragments (`changelog.d/<lot>.md`), assemblés à la publication par `cargo run -p xtask -- changelog <version>` (aperçu : `--apercu`).
