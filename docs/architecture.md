<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Guide d'architecture

Pour qui arrive sur le dépôt avec un lot à faire : où passe un calcul, où ajouter quoi, comment tester vite. Les règles de fond (le texte fait foi, ne rien inventer, citer) sont dans [`CONTRIBUTING.md`](../CONTRIBUTING.md) ; l'avancement fiche par fiche dans [`couverture.md`](couverture.md) ; le format dans [`modele-donnees.md`](modele-donnees.md) ; les lectures tranchées dans [`questions/`](questions/README.md).

Les chemins courts (`calcul/mod.rs`) désignent `crates/thbce-core/src/…`. **Tout lot qui modifie l'architecture décrite ici (nouvelle étape de la boucle horaire, nouveau module, nouvelle recette) met à jour la partie du guide qui le concerne** ; `cargo run -p xtask -- verifier` contrôle que chaque chemin, chaque `fichier.rs::Élément` et chaque `Type::élément` cités ici existent (`xtask/src/architecture.rs`) ; les noms seuls (fonction sans type ni fichier) ne sont pas contrôlés.

## 1. Parcours d'un calcul

Point d'entrée unique : `calcul/mod.rs::calculer` (projet, météo, options → `Resultats`). La CLI (`crates/thbce-cli/src/main.rs`) et le module WebAssembly (`crates/thbce-wasm/src/lib.rs`) ne font que lire l'entrée et la météo avec `thbce-io` puis appeler `calculer`.

### 1.1 Lecture et validation

1. `crates/thbce-io/src/json.rs::lire_projet` : JSON → `entree::Projet` (serde, `deny_unknown_fields`, chemin de l'erreur).
2. `crates/thbce-io/src/meteo/mod.rs::lire` : détecte le format (classeur, CSV dérivé, ADEME) et produit un `MeteoProjet` (`climat/donnees.rs`) pour la zone climatique du projet.
3. `calcul/mod.rs::verifier` : `validation/mod.rs::valider` (plages des nomenclatures, références, cohérence, périmètre ; un fichier par famille dans `validation/` ; cohérence des usages saisis avec la description du bâtiment, fiches 4.2 et 4.3, en avertissements : `validation/usages_batiment.rs`, Q85), usage pris en charge (`entree/usages.rs::est_pris_en_charge`), météo de la bonne zone. Erreur de saisie → `Erreur::Saisie` (code 2) ; élément reconnu hors périmètre → `hors_perimetre` (code 3).

### 1.2 Préparation (une fois par bâtiment et par mode)

- Climat du site pour chaque jeu météo : `climat/climat_exterieur.rs::ClimatExterieur::nouveau` (§3.1) et période de confort adaptatif `climat/confort_adaptatif.rs::ConfortAdaptatif::nouveau` (éq. 1, 2 et 57).
- `bati/assemblage_batiment.rs::BatimentHoraire::nouveau_selon` construit :
  - les zones, `bati/assemblage_zone.rs::ZoneHoraire::nouvelle_dans_batiment` : scénarios (`scenarios/scenarios_conventionnels.rs::ScenariosZone::nouveau`), groupes (`bati/assemblage_groupe.rs::GroupeHoraire::nouveau_dans_batiment` : enveloppe résolue, modèle thermique, saisons), ventilation du projet hors Th-B (`ventilation/ventilation_projet.rs::VentilationProjet::nouvelle`), modèle aéraulique (`ventilation/debits_air_cep.rs::ZoneAeraulique`) ;
  - les espaces tampons (`bati/espace_tampon/assemblage.rs::EspaceTamponHoraire`) et leurs liens aux groupes (éq. 106 et 107) ;
  - hors Th-B, les systèmes du bâtiment, `SystemesBatiment::nouveaux` (même fichier) : générations résolues depuis la saisie (`chauffage_refroidissement/generation/resolution.rs::resoudre`), distributions intergroupes (`chauffage_refroidissement/distribution.rs::DistributionIntergroupesResolue`), systèmes de chaque groupe (`GroupeHoraire::lier_systemes`), surfaces desservies et autorisations effectives (§8.4).
- Classement au bruit (fiche 5.11) : au début de `calculer`, `bati/classement_bruit.rs::projet_avec_classes_deduites` copie le projet validé en y écrivant la classe de bruit déduite des baies (`expo_br` des baies, `zone_bruit` des zones sans saisie) ; la validation (`validation/bruit.rs`, `validation/exigences.rs`) a contrôlé la cohérence avec les saisies directes (Q200 à Q204).
- Les valeurs par défaut du texte sont appliquées à la préparation (ou au calcul du poste, pour les postes hors boucle) et signalées (`valeurs_par_defaut`, voir §4).

### 1.3 Boucle horaire

`BatimentHoraire::simuler_avec` appelle `BatimentHoraire::pas` pour `h` de 0 à 8 759 (`conventions.rs::NB_HEURES_AN`), puis un rappel `apres_le_pas`. Ordre exact d'un pas de temps :

**Bâtiment** (`bati/assemblage_batiment.rs::BatimentHoraire::pas`, figure 15)

1. Espaces tampons (§5.2 à 5.4), à partir de l'état des groupes à la fin du pas précédent : `EspaceTamponHoraire::pas` → coefficients `btherm`, `bsolaire`, `becl`, `θet`.
2. Chaque zone : `ZoneHoraire::pas_avec_espaces_tampons` (ci-dessous).
3. Hors Th-B : réseaux et générations, `SystemesBatiment::pas` (ci-dessous).

**Zone** (`bati/assemblage_zone.rs::ZoneHoraire::pas_avec_espaces_tampons`, §5.5, figure 18)

1. Conditions du pas : climat (`conditions_exterieures`), scénarios (`ScenariosZone::evaluer`), `θrm` ; en Th-C, consignes des heures à venir pour les relances (§8.5).
2. Régime du système du projet (`VentilationProjet::preparer_pas` : indicateur de ventilation et, pour une centrale de traitement d'air, débits nominaux ou de zone neutre selon l'occupation, la relance et les besoins des groupes au pas précédent, §6.3.3.4.3.2, Q90), débits des groupes (`VentilationProjet::debits_groupe_pas` ; hors habitation selon l'indicateur de ventilation, §6.2.3.2.2.1), puis air soufflé : système du projet en Th-C et Th-D (`VentilationProjet::pas`, §6.2 à 6.5 ; décision Q42), système conventionnel du Bbio en Th-B (`ventilation/ven_bbio.rs::t_air_souffle_c`, §6.1) ; consommation des centrales double flux hors habitation relevée par `VentilationProjet::enregistrer` à la fin du pas de zone, après le mode actif (étape 5).
3. Bilan aéraulique de la zone (`ZoneAeraulique::bilan`, §5.6) avec les températures des groupes au pas précédent.
4. Groupes, en deux passes (figure 20) : ceux qui cèdent de l'air au hall, le hall fictif (`ventilation/debits_air_cep.rs::hall_fictif`), puis ceux qui en reçoivent ; pour chacun, `flux_entrants`, `ScenariosZone::apports_groupe`, puis `GroupeHoraire::pas_avec_espaces_tampons`.
5. Seconde phase d'une centrale à charge variable pourvue de batteries régulées sur la charge (§6.3.3.5, figure 72) : `VentilationProjet::mode_actif`, avec `θi,moy` et les demandes des émetteurs contenus dans les bouches (Q95) ; énergies des batteries dans `ZoneHoraire::mode_actif`, lues par les générations ; le report d'énergie de la génération des batteries au pas précédent (`isreport`, Q95 point 7) est écrit dans `VentilationProjet::isreport` par `BatimentHoraire::pas` avant les zones (`SystemesBatiment::isreport_cta`) ; la demande des émetteurs que le mode actif n'assure pas passe directement à la génération de la batterie (`ModeActif::q_direct_wh`).

**Groupe** (`bati/assemblage_groupe.rs::GroupeHoraire::pas_avec_espaces_tampons`, §5.7, figure 23)

1. Autorisations de chauffer et de refroidir (propres en Th-B et Th-D, effectives de la fiche 8.4 en Th-C) ; en Th-D, température d'inconfort et `Δθconf_adapt` (§13.5.3.3 et 13.5.3.4).
2. Parois opaques et ponts thermiques (§5.15 à 5.20) : `bati/paroi_opaque.rs`, `bati/pont_thermique.rs` (`flux_reduit`, avec les coefficients des espaces tampons).
3. Conditions d'ouverture des baies (§5.13.3.1) : `bati/surventilation.rs::r_ouv_theta_op_man_selon`.
4. Baies (§5.8) : protections mobiles (`bati/protection_mobile.rs`, §5.9 ; gestion automatique et stores vénitiens depuis le lot 1-env, état d'hystérésis `EtatGpm` par baie porté par `EtatGroupeHoraire`), flux solaires et lumineux (`bati/baie.rs`, §5.10), ouverture (`bati/surventilation.rs::r_ouv_manuelle`).
5. Débit de surventilation (§5.13.3.2.4 et 5.13.3.3) : `bati/surventilation.rs::qv_surv_m3_h`.
6. Éclairage (§7.1) : `eclairage/mod.rs::eclairage_habitation` en habitation, `eclairage/mod.rs::eclairage_locaux` dans les autres usages (locaux du groupe, conventionnels en Th-B, du projet sinon).
7. Air entrant équivalent, sollicitations, inerties séquentielle et annuelle en Th-D (`bati/inerties_longues.rs`), puis matrice de comportement thermique (`bati/comportement_thermique.rs::GroupeThermique::matrice`, §5.21) ; les pertes récupérées du pas précédent y entrent (§11.1).
8. Indicateurs de confort en évolution libre (§4.5) : `confort/indicateurs_confort.rs::indicateurs`.
   Brasseurs d'air (§8.32, lot 1-emi, Th-D et Th-C) : `chauffage_refroidissement/brasseurs_air.rs::BrasseursGroupe::pas`, appelé à la fin du pas pour le groupe ; en Th-D, son effet `Δθop^BA` déplace la limite d'inconfort chaud (éq. 2547), en Th-C sa consommation va aux auxiliaires de ventilation (`indicateurs/bilans_energies.rs`) et sa chaleur aux pertes récupérables (§11.1).
9. Relances (§8.5, Th-C seulement) : `chauffage_refroidissement/relances.rs`.
10. Émission (§8.1) : `chauffage_refroidissement/emission.rs::emission_avec_relance` ; émetteur équivalent conventionnel en Th-B, émetteurs du projet en Th-C et Th-D (`chauffage_refroidissement/emetteurs.rs`).
11. Énergies requises par émetteur et ventilateurs locaux (§8.1.3.7 et 8.1.3.8) : `chauffage_refroidissement/systemes_groupe.rs::SystemesGroupe::emettre`.
12. Bilan hydrique (§8.3) et distributions du groupe (§8.6 à 8.8), dans cet ordre selon le mode : en Th-C, `chauffage_refroidissement/bilan_hydrique.rs::bilan_th_c` (déshumidification sur les batteries froides) **puis** `SystemesGroupe::distribuer`, qui reçoit l'énergie latente (éq. 828) ; en Th-B et Th-D, `SystemesGroupe::distribuer` (s'il y a des systèmes) **puis** `chauffage_refroidissement/bilan_hydrique.rs::omega_fin_bbio_kg_kg`.
13. ECS du groupe (§9.5 à 9.7) : `SystemesGroupe::pas_ecs` (`ecs/besoins_ecs.rs`, `ecs/distribution_ecs.rs`), qui reçoit aussi `iecs` de la zone (éq. 47) pour la distribution intergroupe.
14. Bilan d'énergie du groupe (`cumuler_bilan`), confort d'été en Th-D (`confort/confort_ete.rs::PasConfortEte`, §13.5), cumuls mensuels des besoins (§13.1.3).
15. Saisons (§4.6) : `scenarios/saisons.rs` (cumul du jour, bascule à 9 h légales), puis état pour le pas suivant.

**Réseaux et générations du bâtiment** (`bati/assemblage_batiment.rs::SystemesBatiment::pas`, Th-D et Th-C)

1. Distributions intergroupes de chauffage et de refroidissement (§8.9 et 8.10) : `DistributionIntergroupesResolue::pas`.
2. Récupérateurs collectifs de chaleur sur les eaux grises (§9.21, Q108) : apports des émetteurs du pas précédent dans le même pas, `ecs/recuperateur_eaux_grises.rs::pas_collectif`, avant les générations ; les émetteurs individuels sont résolus dans `distribution_ecs.rs` (le récupérateur modifie `Qw` et `θentrant`, passée à la génération par `EntreesGeneration::theta_cw_c` ; la chaleur `ρc·Vec·(θentrant − θcw)` est déduite de la demande de la génération, `LecturePreechauffageEcs::Deduit` par défaut, Q108 point 6). Distributions intergroupes d'ECS (§9.8) : `ecs/distribution_ecs.rs::pas_intergroupe_selon_depart`, en service selon `ecs/distribution_ecs.rs::id_encl` (éq. 1736 ; arrêt pendant les vacances des zones d'enseignement, Q71).
3. Générations (§8.14 et §8.17), activées pour l'ECS si l'un de leurs réseaux d'ECS est en service (éq. 1007, `EntreesGeneration::id_encl`). Chaque génération reçoit les demandes de ses réseaux de chauffage, de refroidissement et d'ECS, **et l'énergie de la sécurité antigel et des batteries de préchauffage et de prérefroidissement des centrales double flux qui la désignent** (réseaux de CTA fictifs, §8.11.3, éq. 939, 951 et 952 ; lectures Q46 et Q93, vus comme des réseaux sur air, `Origine::Antigel` et `Origine::BatterieCta`) ; conditions des sources amont (§8.26 : `match` sur `SourceAmont` qui remplit `ConditionsSource`), puis `chauffage_refroidissement/generation/gestion.rs::pas`. Dans la génération : demandes avec report (éq. 993 à 1016), températures de fonctionnement, **stockages d'abord** pour l'ECS (ballons `ecs/assemblage_ballon.rs` (accumulateurs en eau technique avec échangeur : `ecs/echangeur_ballon.rs`, §9.11 et 9.12, Q106), solaire `ecs/boucle_solaire.rs`, productions collectives `ecs/cesci.rs` ; systèmes solaires combinés aussi pour le chauffage), puis générateurs instantanés dans l'ordre ECS, refroidissement, chauffage (lecture Q46), chaque générateur par `chauffage_refroidissement/generation/generateurs.rs::appel` ; auxiliaires amont (§8.17.3.8) ; matrice des consommations `chauffage_refroidissement/calculs_generation.rs::MatriceCef` (§8.17.3.9, tableau 252) ; cumuls (§8.29).
4. Chaleur puisée dans les espaces tampons (éq. 109) ; attribution des consommations aux groupes (éq. 1484 à 1486, `cumuls_cef_groupes`).
5. Pertes et consommations récupérables (§11.1) : `pertes/pertes_recuperables.rs::recuperation_wh`, injectées dans le modèle thermique **au pas suivant**, en Th-C et en Th-D (lecture Q45, `Lectures::recuperation_th_d`).
6. À la bascule des saisons, nouvelles autorisations effectives (§8.4, `chauffage_refroidissement/saisons_fonctionnement.rs`).

Le photovoltaïque et les postes du logement collectif ne sont pas dans la boucle : voir §1.5.

### 1.4 Les trois passes

`calculer` simule chaque bâtiment trois fois avec le même enchaînement, paramétré par `conventions.rs::ModeCalcul` :

| Passe | Météo | Ventilation, émission | Systèmes | Sortie principale |
|---|---|---|---|---|
| Th-B | jeu « Th-BC » | conventionnelles du Bbio (§6.1, §8.1.3.1) | aucun | besoins, Bbio (§13.1) |
| Th-D | jeu « Th-D » | du projet (Q40, Q42) | du projet, sans refroidissement (§13.5.3.1) | DH (§13.5) |
| Th-C | jeu « Th-BC » | du projet | du projet, relances, déshumidification | consommations (§8.29), relevé horaire de l'électricité |

En Th-C, `calculer` passe à `calcul/mod.rs::simuler_batiment_avec` le rappel `indicateurs/bilans_energies.rs::ReleveElectrique::relever`, qui relève l'électricité par poste à chaque pas (§13.4). Pour inspecter des séries horaires : `calcul/mod.rs::series_horaires` (API Rust, Th-B ou Th-D) et `series_pv`.

### 1.5 Après la boucle : consommations, indicateurs, restitution

Dans `calcul/mod.rs::calculer`, pour chaque bâtiment :

1. Restitution par zone et par groupe des trois passes : `calcul/batiment.rs::restituer_zone` (un fichier de restitution par famille dans `calcul/`).
2. Consommations par poste et par énergie des groupes, zones et bâtiment : `resultats/consommations.rs::Consommations::depuis_cumuls`, depuis la matrice `MatriceCef` de chaque groupe.
3. Postes du logement collectif et des parkings (§7.2, §10.1, §10.3, §10.4) : `deplacements/mod.rs::calculer` ; ventilateurs, mobilier (§4.7, `indicateurs/equipements_mobiliers.rs`) ajoutés au relevé horaire.
4. Production photovoltaïque (§12) : `pv/global.rs::produire`, sur le jeu « Th-BC ».
5. Bbio : `indicateurs/sorties_th_b.rs::sorties` (§13.1, `SREF` éq. 2375 à 2378).
6. Indicateurs, si le bâtiment a des générations : bilan électrique horaire et autoconsommation `indicateurs/bilans_energies.rs::bilan` (§13.4), puis `indicateurs/sorties_th_c.rs::calculer_indicateurs` : forfait de refroidissement (`indicateurs/forfait_refroidissement.rs`, §13.3), lignes par poste et par énergie, Cep, Cep,nr (coefficients de `crates/thbce-data/src/energies.rs`), Ic énergie (pondération dynamique, `crates/thbce-data/src/ponderation_dynamique.rs`), exigences et conformité (`indicateurs/exigences.rs::exigences_partie`, `dh_max` ; valeurs des usages 3 à 28 dans `indicateurs/exigences_usages.rs`, catégorie et classe de bruit héritées groupe → zone → bâtiment par `ParametresExigences::categorie_groupe`, `categorie_zone`, `zone_bruit_zone`) ; pour une extension (`exigences.extension`) ou une construction de moins de 50 m², régime d'exigences de l'article 50-3 et vérifications du régime intermédiaire (`indicateurs/extensions.rs::regime_alternatif`, restitué dans `ResultatsExigences::regime_alternatif`). Lectures : Q54, Q65 à Q69, Q86 à Q88. Indicateurs par occupant (fiche 13.6, Q117) : `indicateurs/occupants.rs::calculer`, `Nocc` saisi par zone (`Zone::types_logements`, `Zone::nb_occupants`) ou tiré des scénarios (`deplacements/mod.rs::nb_max_occupants`) ; Ic énergie annuel (éq. 75) dans `indicateurs/sorties_th_c.rs::calculer_indicateurs`.
7. Analyse du cycle de vie, si le bâtiment a des données `acv` : `acv/mod.rs::calculer_acv` (annexe II, chapitre 4 et §5.3 : contributions « composants » par sous-lot et lot, `acv/composants.rs` pour les formules d'un composant et des fluides frigorigènes, `acv/donnees.rs` pour l'annexe XI et le tableau 7 ; « chantier », « eau », « parcelle » ; Icconstruction, Icbâtiment avec l'Ic énergie de l'étape 6, StockC, Icded), puis `acv/mod.rs::verifier_exigence` (`Icconstruction_max`, `acv/exigences.rs`), qui ajoute la vérification `ic_construction` aux exigences de l'étape 6 et à `conforme`. Le lot 13 prend le taux d'autoconsommation du bilan de l'étape 6. Lectures : Q205 à Q218 (Q216 : exigence des 28 usages ; Q217 : DVE ≥ PER ; Q218 : eau « N/C », quantité saisie).
8. Restitution (`resultats/mod.rs::Resultats`) puis arrondi à une décimale (§1.3.1, décision Q8) : `Resultats::arrondir`, qui applique `conventions.rs::arrondir_restitution` à chaque grandeur ; le calcul interne reste en pleine précision (`Options { arrondir: false }`, `--sans-arrondi`). Séries horaires sur demande : `calcul/series.rs::Collecteur` (relevé pendant le calcul Th-C, `calculer_avec_series`), `crates/thbce-io/src/horaire.rs` (CSV et JSON), `--horaire` de la CLI et option `series_horaires` du module WebAssembly (Q118).

## 2. Carte des crates et des modules

| Crate | Entrées | Sorties | Rôle |
|---|---|---|---|
| `crates/thbce-data` | documents de `docs/reference/` (via `xtask`) | tables Rust générées | données réglementaires : zones climatiques, températures de base, scénarios, réseaux de chaleur, énergies, pondération dynamique, protections mobiles |
| `crates/thbce-core` | `Projet`, `MeteoProjet` | `Resultats`, `RapportValidation` | calcul pur (ni fichier, ni horloge) |
| `crates/thbce-io` | octets JSON, météo | `Projet`, `MeteoProjet`, JSON des résultats, JSON Schema, rapports texte | lecture et écriture |
| `crates/thbce-cli` | fichiers | fichiers, codes 0 à 3 | exécutable `thbce` |
| `crates/thbce-wasm` | objets JS | objets JS | API JavaScript ; types dans `crates/thbce-wasm/src/types.d.ts` (générés) |
| `xtask` | `docs/`, sources | fichiers générés, contrôles | scripts : `cargo run -p xtask -- verifier`, `typescript`, `questions`, `page`, `cas`… (`xtask/src/main.rs`) |

Modules de `thbce-core`. Les fiches exactes et leur statut sont dans [`couverture.md`](couverture.md) ; l'en-tête `//!` de chaque fichier nomme sa fiche. Chaque famille a quatre emplacements parallèles : saisie `entree/`, contrôle `validation/`, résultats `resultats/`, restitution `calcul/`.

| Module | Entrées | Sorties | Chapitre du texte |
|---|---|---|---|
| `entree/` | JSON (serde) | types de saisie, un fichier par famille (`entree/generation/` : un fichier par famille de générateurs) | nomenclatures de toutes les fiches |
| `validation/` | `Projet` | `RapportValidation` (erreurs, avertissements, hors périmètre) | colonnes Min/Max, fiches d'application |
| `climat/` | `MeteoProjet`, département, altitude | climat horaire du site, soleil, confort adaptatif, calendrier | ch. 3 |
| `scenarios/` | usage, surfaces | consignes, occupation, apports, saisons | ch. 4 (§4.1, §4.6) |
| `bati/` | enveloppe résolue, air entrant | températures, besoins, flux ; assemblages bâtiment, zone, groupe ; classement au bruit des baies (`classement_bruit.rs`, fiche 5.11 : tableaux, classe d'un groupe ou d'une zone, `projet_avec_classes_deduites` appelé au début de `calculer`) | ch. 5 |
| `ventilation/` | système et bouches saisis, état des groupes | débits, air soufflé, consommations | ch. 6 et §5.6 |
| `eclairage/` | baies, scénarios | consommation d'éclairage | ch. 7 |
| `chauffage_refroidissement/` | besoins émis, réseaux, générateurs | énergies requises, pertes, matrice des consommations | ch. 8 |
| `ecs/` | besoins d'ECS, ballons, solaire | puisages, pertes, énergie demandée aux générateurs | ch. 9 |
| `deplacements/` | ascenseurs, parkings, zones | consommations électriques horaires | ch. 10, §7.2 |
| `pertes/` | pertes des systèmes | apports récupérés au pas suivant | ch. 11 |
| `pv/` | installations, climat | production horaire | ch. 12 |
| `confort/` | températures opératives | indicateurs de confort, DH | §4.5, §13.5 |
| `indicateurs/` | cumuls, relevé horaire | Bbio, Cep, Cep,nr, Ic énergie, exigences | ch. 13, annexe II, annexe de l'article R. 172-4 |
| `acv/` | `DonneesAcv`, Sref et usages des zones, Ic énergie, autoconsommation PV | `ResultatsAcv` (contributions, Icconstruction, Icbâtiment, StockC), `Icconstruction_max` et vérification | annexe II ch. 4 et §5.3, annexe XI, annexe de l'article R. 172-4 ch. III, III |
| `resultats/` | — | types des résultats, arrondi | — |
| `calcul/` | `Projet`, `MeteoProjet` | `Resultats` | enchaînement (§1) |

## 3. Recettes

Chaque recette se termine par : tests ciblés (§5), fragment `changelog.d/<lot>.md` (format dans `changelog.d/README.md`), lignes de `docs/couverture.md` des fiches traitées, et la suite complète avant le push.

### 3.1 Ajouter un générateur

Exemple à imiter : le poêle ou insert à bois (§8.22, lot 6d).

1. Saisie : variante de `entree/generation/mod.rs::NatureGenerateur` et type dans un fichier de famille (`entree/generation/combustion.rs::PoeleSaisi`).
2. Validation : `validation/generation/mod.rs::postes_assures` (postes assurés) et branche dans `validation/generation/mod.rs::valider_generation`. **Attention** au contrôle du fluide aval dans `valider_generation` (§8.17.3.1) : `match &x.nature { Chaudiere | ReseauDeChaleurOuDeFroid => true, PoeleOuInsert => false, _ => continue }` ; un nouveau générateur qui n'y est pas ajouté échappe en silence au contrôle de cohérence avec les réseaux hydrauliques ou sur air. Puis contrôles dans un fichier de famille (`validation/generation/combustion.rs::valider_poele`). Selon les postes assurés, les règles de montage qui nomment des natures de générateurs : `validation/ecs.rs` (ballons, chauffe-eau thermodynamiques) et `validation/solaire.rs` (appoints des chauffe-eau solaires).
3. Modèle : un fichier par fiche dans `chauffage_refroidissement/` (`chauffage_refroidissement/poeles_et_inserts.rs::Poele`), qui répond au contrat `EntreesAppel` → `SortiesAppel` de `chauffage_refroidissement/generation/mod.rs` (figure 112).
4. Branchement : variante de `chauffage_refroidissement/generation/generateurs.rs::Generateur` et ses quatre branches (`fonction`, `pn_gen_tot_w`, `appel`, `fin_de_pas` s'il garde l'état d'un pas) ; aiguillage de la nature dans `chauffage_refroidissement/generation/resolution.rs::resoudre` (valeurs par défaut signalées ici).
5. Énergie pour Cep, Cep,nr et Ic : `indicateurs/sorties_th_c.rs::vecteur_generateur` (correspondance nature × colonne de la matrice : gaz, GPL, bois, réseau de chaleur ou de froid) et `indicateurs/sorties_th_c.rs::raccorde_rcu` pour un réseau de chaleur.
6. Résultats : seulement s'il a des sorties propres (`resultats/generation.rs`, `calcul/generation.rs`).
7. Format : `cargo run -p xtask -- typescript` (nouvelle variante de `nature` dans `crates/thbce-wasm/src/types.d.ts`) et `docs/modele-donnees.md` (natures de générateurs).
8. Tests : à la main dans le fichier du modèle (`chauffage_refroidissement/poeles_et_inserts.rs`, test `poele_a_la_main`), bout en bout `crates/thbce-io/tests/combustion.rs` (dont un test ignoré sur la météo réglementaire), exemple `tests/cas/maison-poele-granules-ademe`.

### 3.2 Étendre un générateur existant : nouvelle source amont

Cas type : pompe à chaleur sur sol ou sur eau (§8.26, aujourd'hui seules les sources sur air sont saisies). Pas d'exemple complet à imiter ; suivre la source « air extrait » (éq. 1407 et 1454), la plus récente.

1. Saisie : variante de `entree/generation/thermodynamique.rs::SourceAmontSaisie` (partagée par les PAC et les chauffe-eau thermodynamiques, `entree/ecs.rs`).
2. Modèle : `chauffage_refroidissement/sources_amont.rs` (`SourceAmont`, températures amont, auxiliaires, puissance échangeable).
3. Résolution de la saisie : `chauffage_refroidissement/thermodynamique_elec.rs` (PAC) et `ecs/idcet.rs` (chauffe-eau thermodynamiques), qui traduisent `SourceAmontSaisie` en `SourceAmont`.
4. Conditions horaires : le `match` sur `SourceAmont` qui remplit `ConditionsSource` dans `SystemesBatiment::pas` (`bati/assemblage_batiment.rs`, étape « générations ») ; une source sans branche n'y est pas calculée.
5. Validation : `validation/generation/thermodynamique.rs` (PAC) et `validation/ecs.rs` (chauffe-eau thermodynamiques : `match` sur la source et ses références).
6. `cargo run -p xtask -- typescript`, `docs/modele-donnees.md`, tests (`crates/thbce-io/tests/generation.rs`), exemple dans `tests/cas`.

### 3.3 Ajouter un système de ventilation

Exemples à imiter : le double flux (§6.3, lot 6a) et la centrale de traitement d'air en zone neutre (variante `cta`, lot 1-ven). Variante existante de `entree/ventilation.rs::SystemeVentilation` (les systèmes refusés y sont déjà : rendre l'élément calculable, puis retirer son refus `hors_perimetre` dans `validation/ventilation.rs`). Le mode actif des centrales à charge variable (§6.3.3.5, lot 1-ven2, Q95) se calcule après les groupes : `VentilationProjet::mode_actif`, appelé par `ZoneHoraire::pas_avec_espaces_tampons` avec les demandes des émetteurs déclarés `centrale` (qui ne passent plus au réseau du groupe, `SystemesGroupe::distribuer`), recalcule le circuit (`ventilation_projet.rs`, fonction `centrale` partagée avec la zone neutre) et transmet l'énergie des batteries aux générations (`BatteriesCta::chaude`, `BatteriesCta::froide`, éq. 940). Humidificateur d'hiver et rafraîchissement évaporatif (lot 1-ven3, Q96, Q97) : fonctions pures dans `ventilation/humidification.rs` (FADIAB, humidificateurs, hystérésis, pompes) ; l'autorisation `Aut_adiab` et la position des bouches dans le profil à hystérésis (`BoucheProjet::x_adiab`) sont fixées par `VentilationProjet::preparer_pas`, l'humidification indirecte dans le circuit de reprise (fonction `centrale`, donc recalculée en mode actif), l'humidificateur d'hiver et le direct dans `VentilationProjet::pas` ; l'énergie d'un humidificateur à vapeur passe à la génération désignée (`BatteriesCta::humidification`, éq. 939). Aération (lot 1-ven5, Q98, Q99) : variante `NatureSysteme::Aeration` sans ventilateur ni conduit (`VentilationProjet::nouvelle` fixe `Cdep = CFENB`, `Crdbnr = 1`, `Kres = 0`), débit repris opposé au soufflé (`VentilationProjet::est_aeration`, éq. 163), air soufflé à `θext` ou à la sortie de l'espace tampon `GroupeHoraire::aeration_espace_tampon` posée par `ZoneHoraire::pas_avec_espaces_tampons`, bilan aéraulique `EtatGroupe::aeration` (éq. 162 à 164), air soufflé retiré des flux entrants par `debits_air_cep.rs::flux_entrants` et rendu à part par `debits_air_cep.rs::flux_aeration` : un seul débit entrant par les baies, `max(surventilation ; aération)` (éq. 256, `GroupeHoraire::aeration`). Restent refusés : ventilation naturelle et hybride (6.6), puits (6.8 à 6.11), surventilation mécanique (6.12), humidification en habitation.

1. Saisie : `entree/ventilation.rs` (`DoubleFlux`, `Cta` ; accès communs `SystemeVentilation::echangeur`, `batterie_prechauffage`, `batterie_prerefroidissement`).
2. Validation : `validation/ventilation.rs` (systèmes, échangeur, batteries, débit du mode actif des bouches) ; génération désignée par l'antigel ou une batterie : `validation/generation/mod.rs`.
3. Modèle : un fichier par fiche dans `ventilation/` (`ventilation/cta_et_double_flux.rs` : fonctions pures de la fiche 6.3, régime, air neuf, mélange, batteries), déclaré dans `ventilation/mod.rs`.
4. Branchement : `ventilation/ventilation_projet.rs` (`NatureSysteme`, `CtaResolue`, débits par bouche et par régime `BoucheProjet::debits_regime`, préparation dans `VentilationProjet::nouvelle`, régime dans `VentilationProjet::preparer_pas`, pas de temps dans `VentilationProjet::pas`, relevé `ReleveVentilation`) ; antigel et batteries vers une génération : `bati/assemblage_batiment.rs` (`BatteriesCta`).
5. Résultats : `resultats/ventilation.rs`, restitués dans `calcul/batiment.rs::restituer_zone` et `calcul/ventilation.rs` (consommation relevée si `VentilationProjet::releve`, déduite des puissances sinon).
6. Tests : `crates/thbce-core/tests/ventilation_projet.rs`, `crates/thbce-io/tests/ventilation.rs`, `crates/thbce-io/tests/cta.rs` (consommations à la main, sens des effets, prérefroidissement vers une génération de froid, refus), `crates/thbce-io/tests/cta_mode_actif.rs` (mode actif), `crates/thbce-io/tests/humidification.rs` (humidification et rafraîchissement évaporatif), `crates/thbce-io/tests/aeration.rs` (aération), exemples `tests/cas/maison-aeration-ademe`, `tests/cas/bureaux-cta-humidification-ademe`, `tests/cas/maison-double-flux-ademe`, `tests/cas/bureaux-cta-ademe` et `tests/cas/bureaux-cta-dav-ademe`.

### 3.4 Ajouter un usage

Exemples à imiter : les bureaux (usage 3, lot 0a), premier usage non résidentiel ; l'enseignement et la petite enfance (usages 4 à 7 et 12, lot 0c1), qui n'ont demandé que des données et un cas particulier (arrêt de l'ECS en vacances) ; les hôtels, l'hébergement sanitaire et la santé (usages 8 à 11 et 19 à 21, lot 0c2), qui ont demandé la famille « hôpitaux nuit » des protections mobiles et un indicateur d'éclairage fractionnaire (usage 20) ; les restaurants et restaurants scolaires (usages 13 à 16, 26 et 27, lot 0c3), qui n'ont demandé que la famille « restauration » des protections mobiles ; les commerces, vestiaires, aérogares, industries et établissements sportifs (usages 17, 18, 22 à 25 et 28, lot 0c4), qui ont demandé les grands volumes d'éclairage (`affectation_eclairage` des baies, `eclairage/mod.rs::einat_gv_lux`, `LocalResolu::grand_volume`), l'éclairage d'accentuation des commerces et la famille « autres usages » des protections mobiles. Les 28 usages sont désormais calculés. Le socle commun (scénarios, ECS, éclairage par locaux, ventilation simple flux selon l'occupation, indicateurs) vaut pour les 28 usages : ouvrir un usage revient à compléter ses données et ses cas particuliers. **Ordre conseillé** : données et validation propres à l'usage d'abord, l'usage restant refusé (`hors_perimetre`, code 3) ; calcul et tests ensuite ; le refus ne se lève qu'en dernier, dans `est_pris_en_charge`, quand tout est calculé, restitué et testé. Les scénarios des 28 usages sont déjà transcrits : `crates/thbce-data/src/scenarios/table.rs` est généré depuis le classeur de `docs/reference/scenarios/` par `cargo run -p xtask -- scenarios`.

1. `entree/usages.rs::UsageZone::est_pris_en_charge` (les 28 variantes existent déjà ; `est_habitation` sépare les règles résidentielles ; `i_hebergement`, `i_enseignement` et `sommeil_confort_ete` portent les indicateurs du tableau 4 et de l'éq. 2544).
2. Tables par usage, à vérifier une à une (celles marquées ✔ couvrent déjà les 28 usages) :
   - scénarios ✔ (`scenarios/scenarios_conventionnels.rs::ScenariosZone::nouveau`) ; valeurs fractionnaires de quelques usages à trancher (`docs/questions/Q063.md`) ;
   - éclairage : `crates/thbce-data/src/eclairage.rs::TABLEAU_78` ✔ ; grands volumes (Q81 : `LocalResolu::grand_volume`, `a_eclnat_gv_m2`, `einat_gv_lux`, `eclairage_locaux_gv`, validation dans `validation/eclairage.rs`) et accentuation des commerces (Q82 : `LocalResolu::projet`) ✔ ; `IEcl` fractionnaire (usage 20, 0,35 la nuit) lu comme fraction de `C1` par `eclairage/mod.rs::ctrl_ecl_occ` (Q74) ;
   - protections mobiles : `crates/thbce-data/src/protections_mobiles.rs::famille_et_parts` ✔, et `matrices` et `valeurs_th_d` connaissent les six familles « habitation », « bureau », « enseignement », « hôpitaux nuit », « restauration » et « autres » (figures 37-3 et 40-3, colonne du tableau 34 à plancher d'inoccupation propre, Q84 ; les colonnes « hôpitaux nuit » et « restauration » plafonnent les stores de nuit à `min(50 % ; Th-BC)` et `min(70 % ; Th-BC)` par `ValeursThD::nuit_store_plafonne`). Une famille sans matrice fait renvoyer `None` à `GestionResolue::nouvelle` (protection ignorée sans message) : à transcrire **avant** de lever le refus d'un usage de cette famille ;
   - ventilation : `ventilation/bouche_conduit.rs::param_debits_bbio` ✔ (tableau 56-1), typologie du réseau dans `ventilation/ventilation_projet.rs::VentilationProjet::nouvelle` ;
   - caractère traversant : `bati/surventilation.rs::traversant_conventionnel` ✔, `ventilation/debits_air_cep.rs::trav_zone_conventionnel` ✔ ;
   - ECS : `ecs/besoins_ecs.rs::rat_douches_bains` ✔, `ecs/distribution_ecs.rs::nb_bouchons` ✔, longueurs par défaut (`ecs/distribution_ecs.rs`) ; arrêt en vacances des usages affiliés à l'enseignement ✔ (`iecs` de la zone porté par `EmetteurEcsResolu::i_ecs`, puis `ecs/distribution_ecs.rs::id_encl` du réseau intergroupe et `EntreesGeneration::id_encl` de la génération, éq. 47, 1736 et 1007, Q71 ; `BoucleEcs::arret_circulateurs_vacances` requis pour une boucle qui dessert une zone d'enseignement) ;
   - ascenseurs et parkings (`deplacements/mod.rs`, `deplacements/ascenseurs.rs::bv`) : logement collectif seulement ;
   - exigences ✔ (`indicateurs/exigences_usages.rs`, Q65 ; rien à ajouter, sauf si l'usage a une modulation propre ; régime des extensions selon la famille d'usage : `indicateurs/extensions.rs::famille`, Q86) et forfait de refroidissement (`indicateurs/forfait_refroidissement.rs::coef_kwh_fr_par_dh` ✔).
3. Validation : quelques lignes par famille (`validation/batiment.rs`, `validation/eclairage.rs`, `validation/ventilation.rs`, `validation/systemes.rs`, `validation/collectif.rs`, `validation/exigences.rs`). Pour tout repérer : `grep -rn "est_habitation\|est_pris_en_charge\|LogementCollectif\|MaisonIndividuelle\|usage.numero()" crates/thbce-core/src`.
4. Tests : `crates/thbce-io/tests/tertiaire.rs` (bout en bout, sens des effets, refus), exemple `tests/cas/bureaux-simple-flux-ademe` ; pour l'enseignement et la petite enfance, `crates/thbce-io/tests/enseignement.rs` (calculs à la main, sens des effets, autres usages du lot par substitution des locaux du tableau 78), exemples `tests/cas/ecole-primaire-ademe` et `tests/cas/creche-pac-ademe` ; pour la restauration, `crates/thbce-io/tests/restauration.rs`, exemples `tests/cas/restaurant-pac-ademe` (usage 13) et `tests/cas/restaurant-scolaire-ademe` (usage 26). Pour les commerces et les établissements sportifs : `crates/thbce-io/tests/commerces_sports.rs`, exemples `tests/cas/commerce-pac-ademe` (usage 17, grand volume à éclairants de toiture) et `tests/cas/gymnase-pac-ademe` (usage 25). Il n'y a plus d'usage refusé : les tests de refus (code 3) emploient un élément hors périmètre (store vénitien, puits climatique, batterie chaude d'une centrale à charge variable).

### 3.5 Ajouter un champ d'entrée

Exemple à imiter : `eclairage.p_installee_w` des parkings (facultatif, valeur par défaut des éq. 2349 et 2350).

1. Champ dans le type de `entree/` (`entree/deplacements.rs::EclairageParking`) : nom du texte en `snake_case` suffixé par l'unité ; doc-commentaire `///` avec le symbole du texte (« Pecins »), l'unité, la règle si absent. **Le doc-commentaire devient la description du JSON Schema** (schemars) et des types TypeScript. Facultatif : `Option<…>` avec `#[serde(default, skip_serializing_if = "Option::is_none")]` ; un champ obligatoire nouveau casse les entrées existantes (à justifier dans le fragment changelog).
2. Validation : plage et cohérence (`validation/collectif.rs`).
3. Calcul : lecture du champ ; si absent et que le texte donne une valeur, l'appliquer et pousser un `resultats/mod.rs::ValeurParDefaut` (`deplacements/parkings.rs::p_ecl_installee`) ; sinon le champ est obligatoire.
4. `cargo run -p xtask -- typescript` (régénère `crates/thbce-wasm/src/types.d.ts`) ; `thbce schema` donne le JSON Schema.
5. `docs/modele-donnees.md` (arborescence), un exemple de `tests/cas/` qui l'emploie, test à la main (`deplacements/parkings.rs`, test `eclairage_interieur_habitat_toute_l_annee`), fragment changelog.

### 3.6 Ajouter une sortie

Exemple à imiter : `nbh_bypass` et `q_ag_kwh` de la ventilation (`resultats/ventilation.rs::ResultatsVentilationMode`).

1. Champ dans `resultats/<famille>.rs`, doc-commentaire (symbole, unité, équation) ; facultatif s'il n'existe pas toujours.
2. Valeur remplie dans `calcul/<famille>.rs` ou `calcul/batiment.rs::restituer_zone` (cumuls lus sur `ZoneHoraire`, `GroupeHoraire` ou `SystemesBatiment`).
3. Arrondi : ajouter le champ réel à `Resultats::arrondir` ou à la méthode `arrondir` de sa structure (un oubli laisse la valeur en pleine précision).
4. `cargo run -p xtask -- typescript`, `docs/modele-donnees.md`, fragment changelog ; la valeur peut entrer dans un `attendu.json`.

### 3.7 Ajouter un exemple dans `tests/cas`

Format : [`tests/cas/README.md`](../tests/cas/README.md). Exemple à imiter : `tests/cas/maison-poele-granules-ademe` (variante de la maison d'exemple).

1. Copier le dossier le plus proche, modifier `entree.json`, `cargo run --profile rapide -p thbce-cli -- valider tests/cas/<nom>/entree.json`.
2. `cas.toml` : `description` (ce qui varie, valeurs illustratives), `source`, `meteo = "ademe"` (exécuté par la CI).
3. `cargo t-cas <nom>` affiche la synthèse des indicateurs ; reporter dans `attendu.json` les valeurs arrondies à comparer (Bbio, DH, Cep, Cep,nr, Ic énergie, sorties propres au lot). `cargo t-cas <nom>` doit ensuite répondre « conforme ».
4. Valeurs phares sur la météo réglementaire : dans un test `#[ignore]` (par exemple `crates/thbce-io/tests/indicateurs.rs`, test `indicateurs_de_tous_les_cas_meteo_reglementaire`, qui calcule tous les exemples) ; nombres annuels seulement, jamais de série horaire.

### 3.8 Écrire une question

Règle complète : [`questions/README.md`](questions/README.md). Un fichier `docs/questions/Q<nnn>.md` par question, numéro dans la plage du lot, en-tête `numero`/`sujet`/`statut`, rubriques de `docs/questions/Q054.md`. Dans le code : la lecture retenue isolée dans une fonction (ou une variante d'énumération de lecture, comme `bati/assemblage_groupe.rs::Lectures`) qui renvoie à `docs/questions/Q<nnn>.md`. Puis `cargo run -p xtask -- questions` (régénère `docs/questions/ETAT.md`, à ne jamais éditer à la main).

### 3.9 Lire une équation

1. Localiser : `grep '^1238,' docs/reference/index/annexe-iii-equations.csv` → page 763, fiche 8.23 (colonnes décrites dans `CONTRIBUTING.md`) ; fiches et plages de pages dans `docs/reference/index/annexe-iii-fiches.csv` ; numéros douteux dans `docs/reference/index/anomalies.md`.
2. Lire l'image : `cargo run -p xtask -- page --eq 1238` écrit `target/pages/annexe-iii-p0763.png` (aussi `page <N>`, `--tableau`, `--figure`, `--annexe ii`) ; lire l'image, jamais seulement l'extraction texte de `docs/reference/texte/`.
3. Citer en commentaire : `// Th-BCE 2020 §8.23.3.x, éq. (1238)`.

## 4. Conventions

- **Noms** : ceux du texte, en `snake_case`, suffixés par l'unité (`u_k_w_m2k`, `a_b_m2`, `q_ag_kwh`, `theta_i_c`) ; le symbole du texte entre guillemets dans le doc-commentaire.
- **Références** : chaque fonction de calcul cite sa section et son équation (`// Th-BCE 2020 §x.y, éq. (n)`) ; chaque fichier nomme sa fiche dans son en-tête `//!`.
- **Parité native/WASM** : fonctions transcendantes par `libm` (`clippy.toml` interdit celles de la bibliothèque standard ; `libm::pow` au lieu de `f64::powf`), aucun parallélisme ni réordonnancement des sommations. La CI compare à l'octet près les sorties natives et `wasm32-wasip1` (`.github/workflows/ci.yml`).
- **Boucle horaire** : pas d'allocation (tampons préparés et réutilisés, voir `ZoneHoraire::pas_avec_espaces_tampons`). Il en reste (`BatimentHoraire::pas` avec espaces tampons, `SystemesBatiment::pas`, bilan hydrique Th-C du groupe) ; ne pas en ajouter.
- **Valeurs par défaut** : seulement si le texte en donne une, appliquée à la préparation, signalée dans `valeurs_par_defaut` (`ValeurParDefaut` : champ, valeur, règle citée). Sinon le champ est obligatoire. Aucune valeur inventée.
- **Lectures ambiguës** : une fonction isolée (ou un choix de `Lectures`) et une question `docs/questions/Q<nnn>.md` ; garder la variante non exposée si son effet est notable et chiffrer cet effet. Ne jamais réécrire une lecture tranchée sans nouvelle décision datée.
- **Erreurs** : jamais de `panic!` sur une entrée utilisateur (`erreur.rs::Erreur`) ; un élément reconnu mais non calculé est refusé par la validation (`hors_perimetre`, code 3), et ne cesse de l'être que lorsqu'il est lu, validé, calculé, restitué et testé.
- **Météo réglementaire** : jamais committée, ni en partie, ni dans un test ou un `attendu.json`. Les tests qui en dépendent sont `#[ignore]` et lisent `THBCE_METEO` (CSV) et `THBCE_METEO_CLASSEUR` (classeur `.xlsx`, test `classeur_reglementaire`) ; la CI refuse un fichier suspect.
- **Fichiers générés** (`crates/thbce-wasm/src/types.d.ts`, tables de `thbce-data`, `docs/questions/ETAT.md`, `docs/reference/index/`) : jamais édités à la main ; `cargo run -p xtask -- verifier` contrôle qu'ils sont à jour.
- **Documents partagés** : un lot ajoute, ne réécrit pas ; journal par fragment `changelog.d/<lot>.md`, jamais `CHANGELOG.md`.

## 5. Tests ciblés

Alias de `.cargo/config.toml`, sur le profil `rapide` de `Cargo.toml` (optimisé comme `release`, sans LTO : édition de liens courte ; sorties identiques à l'octet près à celles de `release` sur les quinze exemples, vérifié au lot P4). Exporter `CARGO_INCREMENTAL=0` comme pour toute commande cargo.

| But | Commande | Durée constatée¹ |
|---|---|---|
| tests unitaires d'un module | `cargo t-core --lib ventilation::` | 0,3 s ; 18 s après modification du moteur |
| un fichier de tests d'intégration du moteur | `cargo t-core --test emission` | 0,2 s ; 45 s après modification |
| tous les tests de `thbce-core` | `cargo t-core` | 10 s d'exécution |
| un fichier de tests de bout en bout | `cargo t-io --test combustion` | 2,3 s d'exécution ; 98 s après modification |
| un exemple de `tests/cas` | `cargo t-cas maison-poele-granules-ademe` | 0,9 s ; 114 s après modification |
| tous les exemples (comme le harnais) | `cargo t-cas` | 9 s |
| un test réglementaire ignoré | `THBCE_METEO=… cargo t-regl combustion_sur_meteo_reglementaire` | 2,3 s d'exécution |
| les indicateurs de tous les exemples sur la météo réglementaire | `THBCE_METEO=… cargo t-regl indicateurs_de_tous_les_cas` | 11 s |
| le classeur réglementaire | `THBCE_METEO=… THBCE_METEO_CLASSEUR=… cargo t-regl classeur_reglementaire` | 4 s |
| suite complète | `cargo t-complet` (avec les deux variables) | voir ci-dessous |

¹ Machine de développement à 2 cœurs. « Après modification » : un fichier de `thbce-core` modifié. **La première exécution de chaque alias compile son propre jeu de dépendances** (de 1,5 à 4,5 min) : `-p thbce-core`, `-p thbce-io`, `xtask` et `--workspace` n'activent pas les mêmes fonctionnalités et ne partagent pas leurs artefacts. Construction initiale de tous les tests : 4 min en profil `rapide`, 24 min en `--release` (LTO, une unité de code).

- `cargo t-cas` est une copie de la comparaison du harnais `crates/thbce-io/tests/cas.rs` (`xtask/src/cas.rs`), qui fait foi : la CI lance le harnais. Un exemple `meteo = "re2020"` y est ignoré sans `THBCE_METEO`.
- `cargo t-regl <filtre>` lance les tests ignorés dont le nom contient le filtre (dans tout le workspace) ; liste : `grep -rn -A2 '#\[ignore' crates`. Les options de libtest se mettent à la suite : `cargo t-regl indicateurs_de_tous_les_cas --nocapture` affiche le tableau des indicateurs de tous les exemples (`THBCE_DETAIL=1` pour le détail des lignes par poste et par énergie).
- Équivalents sans alias : `cargo test --profile rapide -p thbce-core --lib <module>::`, `cargo test --profile rapide -p thbce-io --test <fichier>`, `cargo run --profile rapide -p xtask -- cas <nom>`, `cargo test --profile rapide --workspace -- --ignored <filtre>`.
- Un calcul isolé avec la CLI : `cargo run --profile rapide -p thbce-cli -- calcul tests/cas/<nom>/entree.json --meteo docs/reference/meteo/ademe/tracc_france_2-7C_vague_de_chaleur.csv --sans-arrondi -o /tmp/r.json`.

**Avant chaque push, la suite complète reste obligatoire**, en `--release`, avec les tests ignorés, ainsi que format, clippy et `xtask verifier` (liste dans `CONTRIBUTING.md`) :

```sh
export CARGO_INCREMENTAL=0
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
THBCE_METEO=/chemin/meteo.csv THBCE_METEO_CLASSEUR=/chemin/classeur.xlsx \
  cargo test --workspace --release -- --include-ignored     # ou : cargo t-complet
cargo run -p xtask -- verifier
```
