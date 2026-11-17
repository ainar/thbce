<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# thbce

Moteur de calcul de la méthode **Th-BCE 2020**, la méthode réglementaire de la RE2020 (annexe III de l'arrêté du 4 août 2021 modifié, version consolidée au 19 mars 2026), écrit en Rust et compilable en exécutable et en WebAssembly.

> **Avertissement.** Ce moteur n'est **pas le moteur réglementaire officiel** (le moteur de calcul publié par l'administration, intégré aux logiciels évalués). Il n'a pas été évalué ni validé contre des cas de référence, et **un résultat qu'il produit ne vaut pas attestation de conformité** à la RE2020 : il ne peut pas servir à établir le récapitulatif standardisé d'étude énergétique et environnementale (RSEE) ni l'attestation de dépôt de permis.

## État

Version **0.1.0**. Le moteur calcule, au pas horaire sur une année, la **maison individuelle ou accolée**, le **logement collectif** et, depuis le lot 0a (non publié), les **bureaux** :

- **Bbio** (calcul Th-B), **DH** (confort d'été, calcul Th-D sur la météo à séquence caniculaire), consommations par poste et par énergie (calcul Th-C), **Cep**, **Cep,nr** et **Ic énergie** (pondération dynamique de l'article 11) ;
- si l'entrée décrit les données de l'annexe de l'article R. 172-4 du CCH (`exigences`), les **exigences maximales** (Bbio_max, Cep_max, Cep,nr_max, Icénergie_max, DH_max et leurs modulations) et la **conformité** ;
- enveloppe : parois, baies, ponts thermiques, masques, protections mobiles, surventilation par ouverture des baies, espaces tampons (vérandas, garages, combles) ;
- systèmes : ventilation simple et double flux, émetteurs et distributions, effet Joule, pompes à chaleur électriques sur air, chaudières gaz, fioul et bois, poêles et inserts, réseaux de chaleur et de froid, ECS instantanée ou par ballon, chauffe-eau thermodynamiques, chauffe-eau solaires individuels et collectifs (CESI, SSC, CESCI, CESCAI), photovoltaïque et autoconsommation, éclairage des parties communes, ascenseurs et parkings.

Tout élément de l'entrée hors de ce périmètre est reconnu et refusé avec un message explicite (code de retour 3). Aucun cas de référence (COMETH ou logiciel évalué) n'a encore permis de valider les résultats : les lectures du texte retenues sont des hypothèses à confirmer (voir les limites ci-dessous).

- Avancement fiche par fiche : [`docs/couverture.md`](docs/couverture.md) (144 fiches).
- Ambiguïtés du texte et décisions de lecture : [`docs/questions/`](docs/questions/README.md).
- Modèle de données : [`docs/modele-donnees.md`](docs/modele-donnees.md) ; historique du développement : [`docs/historique.md`](docs/historique.md).

## Contrats stables

La version 1.0.0 sera publiée après validation contre le moteur réglementaire COMETH, au moyen d'un logiciel certifié. Jusque-là, les contrats (format JSON, CLI, API JavaScript) sont considérés comme stables, mais les résultats peuvent changer à la suite de cette validation.

Sont considérés comme stables :

- le **format JSON d'entrée** (`thbce schema`) et le **format JSON des résultats** (`thbce schema --resultats`) : des champs facultatifs ou des valeurs nouvelles peuvent s'ajouter, aucun champ existant n'est renommé ni retiré ;
- la **ligne de commande** : commandes `calcul`, `valider`, `schema`, `meteo convertir`, `version`, leurs options et les **codes de retour** ;
- l'**API JavaScript** du module WebAssembly : `calculer`, `valider`, `versionMoteur`, `schemaEntree`, `schemaResultats`, leurs paramètres et la forme des erreurs (`cause.nature`).

L'**API Rust** des crates (`thbce-core`, `thbce-io`…) n'est **pas stabilisée**. Les valeurs calculées peuvent changer quand une lecture du texte est corrigée ; le CHANGELOG le signale.

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

Il n'y a pas de binaire publié. Depuis les sources, avec Rust 1.88 ou plus récent :

```sh
git clone https://github.com/ainar/thbce
cd thbce
cargo install --path crates/thbce-cli     # installe l'exécutable thbce dans ~/.cargo/bin
thbce --version
```

## Exemple complet

Le dépôt contient seize exemples d'entrée dans [`tests/cas/`](tests/cas/) et un fichier météo ouvert (ADEME / Météo-France, non réglementaire) :

```sh
thbce valider tests/cas/maison-climat-ademe/entree.json
thbce calcul tests/cas/maison-climat-ademe/entree.json \
      --meteo docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv \
      -o resultats.json
```

Les résultats sont écrits en JSON dans `resultats.json` ; la synthèse s'affiche sur la sortie d'erreur :

```text
avertissement : données météo non réglementaires, résultat non réglementaire
bâtiment maison : Bbio 63.8 points ; Cep 135.1 kWhep/(m².an) ; Cep,nr 135.1 kWhep/(m².an) ; Ic énergie 167.2 kg éq. CO2/m² ; DH 1555.1 °C.h
  bbio                    63.8 ≤     64.7 : marge      0.9 (+1.4 %) respectée
  cep                    135.1 ≤     70.2 : marge    -64.9 (-92.5 %) NON RESPECTÉE
  cep_nr                 135.1 ≤     51.5 : marge    -83.6 (-162.3 %) NON RESPECTÉE
  ic_energie             167.2 ≤    149.7 : marge    -17.5 (-11.7 %) NON RESPECTÉE
  dh (volume-chauffe)   1555.1 ≤   1250.0 : marge   -305.1 (-24.4 %) NON RESPECTÉE
  exigences de résultat (Bbio, Cep, Cep,nr, Ic énergie, DH) : non respectées
```

Avec la météo réglementaire, remplacer le fichier ADEME par le classeur officiel (`--meteo 4_scenarios_meteorologiques_th-bc_th-d_re2020.xlsx`) ou le CSV qu'en tire `thbce meteo convertir` ; la même maison donne alors Bbio 59,7, Cep 145,8, Ic énergie 182,3 et DH 1 042,9.

## Ligne de commande

```sh
thbce calcul projet.json --meteo meteo-re2020.xlsx -o resultats.json
thbce valider projet.json
thbce meteo convertir meteo-re2020.xlsx -o meteo-re2020.csv   # CSV pour le module WebAssembly
thbce schema                 # JSON Schema du format d'entrée (--resultats pour celui des résultats)
thbce version                # version du moteur et de la méthode (aussi thbce --version, -V)
```

`--meteo` est obligatoire. `--sans-arrondi` restitue les valeurs en pleine précision au lieu de l'arrondi à une décimale prévu par la méthode (§1.3.1). Une entrée peut déclarer `"$schema"` (adresse du JSON Schema, pour l'aide à la saisie des éditeurs) et `"version_format": 1`.

| Code de retour | Signification |
|---|---|
| 0 | succès |
| 1 | erreur d'usage (arguments) ou de fichier (introuvable, illisible, écriture impossible) |
| 2 | entrée invalide : JSON mal formé, saisie fautive, données météo absentes ou d'une autre zone |
| 3 | entrée correcte mais non calculable par cette version : élément ou usage hors périmètre ([`docs/couverture.md`](docs/couverture.md)), à la validation comme au calcul ; ou calcul impossible |

`thbce valider` applique les mêmes codes : une entrée correcte qui décrit un élément hors du périmètre (usage non résidentiel, ventilation naturelle, store vénitien…) est signalée « non calculé » et renvoie 3. Les erreurs de saisie l'emportent : une entrée qui en contient renvoie 2.

Le format RSEE XML n'est pas lu.

## WebAssembly

```sh
wasm-pack build crates/thbce-wasm --target web --release
```

Dans le navigateur, le module lit le CSV dérivé du classeur (`thbce meteo convertir`) ou un fichier ADEME, jamais le classeur `.xlsx`.

```js
import init, { calculer, valider, versionMoteur, schemaEntree, schemaResultats } from "./pkg/thbce_wasm.js";

await init();
const meteo = new Uint8Array(await fichierMeteo.arrayBuffer());
const resultats = calculer(projet, meteo);      // projet : objet ou texte JSON
```

| Fonction | Rôle |
|---|---|
| `calculer(projet, meteo, options?)` | calcule ; `projet` : objet ou texte JSON, `meteo` : octets du CSV dérivé ou d'un fichier ADEME ; renvoie `Resultats` |
| `valider(projet)` | valide sans calculer ; renvoie `RapportValidation` (`valide`, `hors_perimetre`, `erreurs`, `avertissements`) |
| `versionMoteur()` | version du moteur et de la méthode |
| `schemaEntree()`, `schemaResultats()` | JSON Schema de l'entrée et des résultats |

Les noms des fonctions suivent l'usage de JavaScript (camelCase) ; les champs de l'entrée et des résultats gardent les noms du format JSON (snake_case). En cas d'échec, `calculer` et `valider` lèvent une `Error` dont la propriété `cause` est un `ErreurCalcul` : `nature` vaut `lecture`, `saisie`, `meteo`, `non_pris_en_charge` (entrée correcte mais hors du périmètre) ou `calcul`.

Les fonctions sont typées (`pkg/thbce_wasm.d.ts`) : les types `Projet` et `Resultats` sont générés depuis les JSON Schema (`cargo run -p xtask -- typescript`, contrôlé par `xtask verifier`). Elles sont synchrones et sans état ; pour ne pas bloquer l'interface, les appeler depuis un Web Worker. [`examples/web/`](examples/web/) contient une page minimale.

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
| `docs` | [guide d'architecture](docs/architecture.md), couverture, modèle de données, questions |

## Développement

Pour contribuer, lire d'abord le guide d'architecture [`docs/architecture.md`](docs/architecture.md) (parcours d'un calcul, carte des modules, recettes, tests ciblés), puis [`CONTRIBUTING.md`](CONTRIBUTING.md). Tout nouveau lot met à jour la partie du guide qui le concerne.

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

- **Périmètre** : maison individuelle ou accolée, logement collectif et bureaux ; les 25 autres usages sont refusés. En bureaux : ventilation simple flux seulement (double flux et centrales de traitement d'air refusés), locaux d'éclairage de volume normal, ni ascenseurs ni parkings. Extensions de bâtiment non traitées.
- **Aucune validation contre un cas de référence** (COMETH ou RSEE d'un logiciel évalué) : les lectures du texte Q8 à Q54 de [`docs/questions/`](docs/questions/README.md) restent des hypothèses ; les paramètres à tester en premier en cas d'écart y sont listés.
- **Exigences** : valeurs transcrites pour les 28 usages (celles de l'habitation et des bureaux servent ; les autres usages ne sont pas calculés) ; catégorie de contraintes extérieures et zone de bruit saisies pour le bâtiment, la zone ou le groupe (le moteur contrôle la catégorie selon le chapitre V de l'annexe de l'article R. 172-4, mais ne détermine pas la classe de bruit des baies) ; Icconstruction et les exigences de moyens hors du moteur.
- **Pas de lecteur RSEE XML** : le schéma XSD n'est pas public.
- **Systèmes refusés** : ventilation naturelle, hybride, aération, centrales de traitement d'air, puits climatiques ou hydrauliques, échangeur rotatif, préchauffage, humidification, rafraîchissement évaporatif, surventilation mécanique ; brasseurs d'air ; gestion des générateurs en alternance, boucles d'eau, DRV, cogénération, PAC au gaz ou hybrides, PAC sur eau ou sur sol, triple service, second ballon, récupération de chaleur sur les eaux grises ; stores vénitiens, gestion automatique des protections mobiles ; Titres V.
- **Calage horaire** : un décalage d'une heure est possible entre le fichier météo et le calendrier de la méthode (Q25).
- **Sorties non restituées** : indicateurs par occupant (fiche 13.6), sorties Th-C par zone et par groupe (§13.2), valeur annuelle de l'éq. (75) de l'Ic énergie, séries horaires (seulement par l'API Rust).

## Licence

[GPL-3.0-or-later](LICENSE). Les documents de `docs/reference/` sont des textes et données publics de l'administration ; ils ne sont pas couverts par la licence du code (voir `docs/reference/README.md`).
