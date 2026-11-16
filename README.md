<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# thbce

Moteur de calcul de la méthode **Th-BCE 2020**, la méthode réglementaire de la RE2020 (annexe III de l'arrêté du 4 août 2021 modifié), écrit en Rust et compilable en exécutable et en WebAssembly.

> **Avertissement.** Ce moteur n'est pas le moteur réglementaire officiel. Il n'a pas été évalué et ses résultats ne valent pas attestation de conformité à la RE2020.

## État

Le projet en est au **lot « confort d'été »** (phase 5). Sont en place : la lecture des entrées et des fichiers météo, la validation, la ligne de commande, le module WebAssembly, le climat du site, le rayonnement sur les parois avec masques, et le **calcul annuel au pas horaire en mode Th-B** pour la maison individuelle et le logement collectif, qui produit les besoins de chauffage, de refroidissement et d'éclairage et le **Bbio**, suivi d'un second calcul annuel **en mode Th-D** (météo à séquence caniculaire), qui produit l'indicateur de confort d'été **DH**. Le DH est pour l'instant calculé avec le système de ventilation conventionnel du Bbio, la ventilation du projet n'étant pas encore décrite. Aucun cas de référence n'a encore permis de valider ces résultats : plusieurs lectures du texte restent à confirmer (`docs/questions.md`). Cep, Cep,nr et Ic énergie ne sont pas encore calculés et valent `null`.

- Avancement fiche par fiche : [`docs/couverture.md`](docs/couverture.md) (144 fiches).
- Ambiguïtés du texte et décisions de lecture : [`docs/questions.md`](docs/questions.md).
- Modèle de données : [`docs/modele-donnees.md`](docs/modele-donnees.md).

La première version visera la maison individuelle et le logement collectif.

## Données météo

Le moteur **n'embarque aucune donnée météo**. Les fichiers météorologiques conventionnels de la RE2020 sont soumis à des conditions d'utilisation qui interdisent de les diffuser : ils ne sont ni dans ce dépôt ni dans les binaires. Chaque utilisateur les obtient auprès de l'administration et les fournit au moment du calcul.

Formats lus, reconnus d'après le contenu du fichier :

| Format | Résultat |
|---|---|
| classeur officiel `4_scenarios_meteorologiques_th-bc_th-d_re2020.xlsx` (16 onglets `<zone>_<mode>`) | réglementaire |
| CSV dérivé, colonnes `zone, mode, h, te0, we0, dirN_W_m2, diff_W_m2, Teciel_C, Vent_m_s, Teau_C, Gamma, Psi` | réglementaire |
| fichiers ouverts ADEME / Météo-France (`docs/reference/meteo/ademe/`) | **non réglementaire** |

Le module WebAssembly ne lit pas le classeur `.xlsx` : dans le navigateur, fournir le CSV dérivé, que produit `thbce meteo convertir`. Ce CSV reste soumis aux mêmes conditions d'utilisation que le classeur. Les fichiers ADEME décrivent un climat futur (+2,7 °C) et sont convertis par des règles documentées dans `crates/thbce-io/src/meteo/ademe.rs` ; ils servent aux essais et à la CI.

Les résultats portent un champ `reglementaire`, faux dès que les données météo ne sont pas les fichiers conventionnels. Un fichier retravaillé ou un jeu d'essai doit le déclarer par une ligne `# thbce-origine: <description>` ; le moteur ne peut pas vérifier l'authenticité d'un fichier.

## Installation

Il n'y a pas encore de version publiée. Depuis les sources, avec une chaîne Rust stable :

```sh
git clone https://github.com/ainar/thbce
cd thbce
cargo build --release -p thbce-cli      # exécutable : target/release/thbce
```

## Ligne de commande

```sh
thbce calcul projet.json --meteo meteo-re2020.xlsx -o resultats.json
thbce valider projet.json
thbce meteo convertir meteo-re2020.xlsx -o meteo-re2020.csv   # CSV pour le module WebAssembly
thbce schema                 # JSON Schema du format d'entrée (--resultats pour celui des résultats)
thbce version                # version du moteur et de la méthode implémentée
```

`--meteo` est obligatoire. `--sans-arrondi` restitue les valeurs en pleine précision au lieu de l'arrondi à une décimale prévu par la méthode (§1.3.1).

| Code de retour | Signification |
|---|---|
| 0 | succès |
| 1 | erreur d'usage ou de fichier |
| 2 | entrée invalide (projet ou données météo) |
| 3 | erreur de calcul (élément non pris en charge) |

Un exemple d'entrée se trouve dans [`tests/cas/squelette-maison/entree.json`](tests/cas/squelette-maison/entree.json). Le format RSEE XML n'est pas encore lu.

## WebAssembly

```sh
wasm-pack build crates/thbce-wasm --target web --release
```

```js
import init, { calculer, valider, versionMoteur } from "./pkg/thbce_wasm.js";

await init();
const meteo = new Uint8Array(await fichierMeteo.arrayBuffer());
const resultats = calculer(projet, meteo);      // projet : objet ou texte JSON
```

Les fonctions sont typées (`pkg/thbce_wasm.d.ts`), synchrones et sans état ; pour ne pas bloquer l'interface, les appeler depuis un Web Worker. [`examples/web/`](examples/web/) contient une page minimale.

## Organisation du dépôt

| Dossier | Contenu |
|---|---|
| `crates/thbce-core` | calcul pur : types d'entrée, validation, résultats ; aucune entrée-sortie |
| `crates/thbce-data` | tables réglementaires (zones climatiques, températures de base, scénarios conventionnels, pondération dynamique) |
| `crates/thbce-io` | lecture du JSON et des fichiers météo, JSON Schema, rapports |
| `crates/thbce-cli` | exécutable `thbce` |
| `crates/thbce-wasm` | bindings JavaScript |
| `xtask` | scripts reproductibles (génération des données versionnées) |
| `tests/cas` | cas de validation ([format](tests/cas/README.md)) |
| `docs/reference` | textes réglementaires et données sources |

## Développement

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo run -p xtask -- verifier          # fichiers générés à jour
```

Les tests qui ont besoin de la météo réglementaire sont ignorés par défaut :

```sh
THBCE_METEO=/chemin/vers/meteo-re2020.csv cargo test --workspace -- --include-ignored
```

Voir [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Limites connues

- Aucun indicateur calculé à ce stade.
- Une question de calage horaire entre le fichier météo et le calendrier de la méthode reste ouverte (Q25 de `docs/questions.md`).
- Usages autres que maison individuelle et logement collectif : reconnus à la lecture, refusés au calcul.
- Pas de lecteur RSEE XML : le schéma n'est pas public.
- Aucun cas de validation issu du moteur réglementaire n'est encore disponible ; plusieurs lectures du texte restent à confirmer (voir `docs/questions.md`).

## Licence

[GPL-3.0-or-later](LICENSE). Les documents de `docs/reference/` sont des textes et données publics de l'administration ; ils ne sont pas couverts par la licence du code (voir `docs/reference/README.md`).
