<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Proposition de modèle de données

**Statut : validé par le mainteneur le 16 novembre 2026.** Les types sont implémentés dans `crates/thbce-core/src/entree.rs` ; le JSON Schema (`thbce schema`) fait foi pour le format exact. Ce document garde la vue d'ensemble et les intentions.

## Écarts de la phase 2 par rapport à la proposition

- **Seuls les champs vérifiés dans une nomenclature sont implémentés.** Sont reportés à la lecture de leur fiche : les locaux, l'éclairage, la ventilation, l'émission, les systèmes, le photovoltaïque, les stores vénitiens, le masque d'arbres à feuilles caduques, le contact d'une paroi (extérieur, espace tampon), l'ouverture et le classement au bruit des baies, le ratio EnR des réseaux.
- **`thbce-core` dépend de `thbce-data`**, et non l'inverse : `Departement` et `ZoneClimatique` sont définis avec leurs tables dans `thbce-data`, qui reste sans entrée-sortie.
- **La validation est dans `thbce-core`** (`validation.rs`), pour servir aussi le module WebAssembly ; `thbce-io` n'en fait que la mise en forme.
- **La zone climatique est déduite du département** ; `zone_climatique` devient un champ facultatif, et une valeur contradictoire déclenche un avertissement.
- **Un seul lecteur météo** pour l'instant (`meteo::lire_csv_re2020`) ; le classeur et les fichiers ADEME viendront au lot 3. La provenance non réglementaire d'un fichier se déclare par une ligne `# thbce-origine:`.
- **Résultats** : `origine_meteo_th_bc` et `origine_meteo_th_d` remplacent `origine_meteo` ; `valeurs_arrondies` indique si l'arrondi de restitution est appliqué.
- **Surface de la baie** : `a_b_m2` (« Ab », §5.10.2). **Pont thermique** : sans exposition propre, la nomenclature du §5.19.2 n'en prévoyant pas.
- **Masques (phase 3)** : un masque lointain azimutal se saisit par ses 36 hauteurs `gamma_i_deg`, une par tranche de 10° repérée par rapport à la normale au composant ; le masque d'arbres à feuilles caduques a la même forme. Un masque proche exige la largeur (`lpb_m`) ou la hauteur (`hpb_m`) du composant.
- **Résultats (phase 3)** : bloc `climat` avec la zone, `altcorr_m`, `base_ext_c` et la période de confort adaptatif de chaque jeu météo.
- **Lecteurs météo (phase 3)** : `meteo::lire` reconnaît le classeur `.xlsx`, le CSV dérivé et les fichiers ADEME. Le module WebAssembly ne lit pas le classeur.
- **Surfaces (lot 4b)** : `su_m2` est la surface utile du groupe (« Agr », §4.1.2), obligatoire pour tous les usages ; `shab_m2` reste la surface de référence en habitation. `nb_lgt` est obligatoire en logement collectif. Les résultats de zone portent `nadeq`.
- **Inertie** : les classes des tableaux 53 et 54 (§5.21.3) sont des variantes d'énumération ; `Ams_surf` et `Ama_surf` sont toujours saisies, comme le dit le texte.

Ce document décrit les types Rust d'entrée et de sortie du moteur. Il ne fixe que la structure. Les champs cités viennent des nomenclatures de l'annexe III déjà lues (§3.1, 3.2, 4.1, 5.1, 5.10, 5.17, 5.19, 5.21, 13.1, 13.2, 13.4, 13.5) ; les autres fiches compléteront les types au moment où elles seront implémentées, chaque champ portant la référence de la nomenclature dont il sort. Aucun champ n'est ajouté s'il ne figure pas dans une nomenclature du texte.

## Principes

1. **Trois couches de types**, pour séparer la saisie du calcul :
   - `entree::*` : ce que l'utilisateur fournit (JSON natif ou RSEE converti). Types `serde` + `schemars`, identifiants textuels, aucune donnée conventionnelle.
   - `modele::*` : la même description après validation et résolution : identifiants remplacés par des indices, données conventionnelles jointes (météo, scénarios), tampons préalloués. Construit une fois par `entree → modele`, c'est la seule étape qui peut échouer sur une erreur de saisie.
   - `resultats::*` : indicateurs, détails par niveau et, en option, séries horaires.
2. **Paramètres intrinsèques et paramètres d'intégration** : la méthode distingue les caractéristiques propres d'un composant de sa mise en œuvre dans le projet (A3 §1.2). Le modèle reprend cette distinction : un catalogue de composants (`BaieType`, `ParoiType`…) référencé par des instances (`Baie`, `ParoiOpaque`…) qui portent surface, orientation et masques. Un même vitrage décrit une fois sert à vingt baies.
3. **Noms** : ceux du texte, en `snake_case`, suffixés par l'unité (`u_k_w_m2k`, `a_k_m2`, `alt_m`). Les angles saisis sont en degrés (comme les nomenclatures), convertis en radians dans `modele` (A3 §3.2.1 : « dans tous les calculs les angles sont en radians »).
4. **Énumérations fermées** pour tout code numérique du texte (usage, inclinaison, type d'énergie, poste). Le code numérique du texte est conservé en commentaire et dans la (dé)sérialisation RSEE, jamais manipulé en clair dans le calcul.
5. **Pas de `Option` porteur de sens caché** : une valeur conventionnelle du texte est appliquée dans `entree → modele` et tracée dans le rapport de validation.

## Hiérarchie spatiale

La méthode décrit un bâtiment sur quatre niveaux (A3 §5.1.1), plus un niveau projet pour les données communes à plusieurs bâtiments (note 1 du §5.1.1 et production photovoltaïque « de la parcelle », §13.4.3).

```
Projet
├── localisation (département, altitude, distance mer, distance îlot urbain)
├── catalogue de composants (types de baies, parois, ponts thermiques, masques)
├── productions PV de parcelle
└── Batiment[]
    ├── espaces tampons
    ├── générations, stockages, réseaux intergroupes
    ├── productions PV du bâtiment
    └── Zone[]                 ← un usage, donc un jeu de scénarios
        └── Groupe[]           ← air homogène en température et humidité (§5.21.1)
            ├── parois opaques, baies, ponts thermiques
            ├── inertie, éclairage, ventilation, émission
            └── Local[]        ← type de local, ratio de surface
```

```rust
// crates/thbce-core/src/entree/mod.rs

pub struct Projet {
    pub nom: String,
    pub localisation: Localisation,
    pub catalogue: Catalogue,
    pub batiments: Vec<Batiment>,
    pub installations_pv_parcelle: Vec<InstallationPv>, // A3 §12.1, §13.4.3 éq. (2525)
}

/// A3 §3.1.2 et §5.1.2 — paramètres d'intégration du climat et du bâtiment.
pub struct Localisation {
    pub departement: Departement,        // 01..95, 2A, 2B (question Q13) ; fixe la température de base (A3 p. 31)
    pub zone_climatique: ZoneClimatique, // H1a … H3 ; saisie tant que la table département → zone manque (Q1a)
    pub alt_m: f64,
    pub dmer_km: f64,
    pub durb_km: f64,
}

pub struct Batiment {
    pub id: Id,
    pub zones: Vec<Zone>,
    pub espaces_tampons: Vec<EspaceTampon>,        // A3 §5.2–5.4 (lot 4b)
    pub systemes: SystemesBatiment,                // A3 ch. 6, 8, 9, 10 (phase 6)
    pub installations_pv: Vec<InstallationPv>,     // A3 ch. 12 (phase 7)
    pub rat_enr_rdch: Option<f64>,                 // A3 tableau 333 : RatENR_rdch du réseau de chaleur
}

pub struct Zone {
    pub id: Id,
    pub usage: UsageZone,          // A3 §4.1.2 « Usage zone », 28 usages (p. 24–26)
    pub groupes: Vec<Groupe>,
    pub nb_lgt: Option<u32>,       // A3 §4.1.2 : nombre de logements (collectif, maisons accolées)
}

pub struct Groupe {
    pub id: Id,
    pub surfaces: SurfacesGroupe,              // SHAB ou SU → SREF (A3 éq. 2375–2376), Agr
    pub categorie_ce1_ce2: CategorieCe,        // A3 éq. (2387)
    pub inertie: Inertie,                      // A3 §5.21.2
    pub parois_opaques: Vec<ParoiOpaque>,
    pub baies: Vec<Baie>,
    pub ponts_thermiques: Vec<PontThermique>,
    pub locaux: Vec<Local>,                    // par défaut : ratios conventionnels de l'usage
    pub eclairage: Eclairage,                  // A3 §7.1 (phase 4)
    pub ventilation: VentilationGroupe,        // A3 §6.1 (phase 4), ch. 6 (phase 6)
    pub emissions: Vec<Emission>,              // A3 §8.1 (phase 6)
}

pub struct Local {
    pub type_local: TypeLocal,     // A3 §4.1.2 « Type local »
    pub rat_loc: f64,              // ratio surface utile du local / surface utile
}
```

## Enveloppe

```rust
/// A3 §5.17.2 — paroi opaque.
pub struct ParoiType {
    pub id: Id,
    pub u_k_w_m2k: f64,
    pub sf_c_k: f64,               // facteur solaire, calcul du Cep
    pub sf_e_k: f64,               // facteur solaire, calcul de DH
    pub vegetalisation: Option<Vegetalisation>, // sf_c_v_k, sf_e_v_k, saison végétative
}
pub struct ParoiOpaque {
    pub type_paroi: Id,
    pub a_k_m2: f64,
    pub exposition: Exposition,
}

/// A3 §3.2.2 — paramètres d'intégration communs à toute surface exposée.
pub struct Exposition {
    pub azimut_deg: f64,           // 0 sud, 90 ouest, 180 nord, 270 est
    pub inclinaison_deg: f64,      // 0 horizontale vers le haut, 90 verticale, 180 vers le bas
    pub lpb_m: Option<f64>,        // largeur et hauteur : requises pour les masques proches
    pub hpb_m: Option<f64>,
    pub masques: Vec<Id>,          // 0 à 7 masques (Nmasque)
    pub donne_sur: Contact,        // extérieur, espace tampon… (à préciser avec §5.4 et §5.15)
}

/// A3 §3.2 — masques.
pub enum Masque {
    LointainAzimutal { tranches: Vec<TrancheAzimutale> },      // Az, γi
    LointainArbresCaducs { /* à compléter en phase 3 */ },
    LointainPlanVertical { de_m: f64, hpe_m: f64 },
    ProcheVerticalDroit { dvd_m: f64, dpd_m: f64 },
    ProcheVerticalGauche { dvg_m: f64, dpg_m: f64 },
    ProcheHorizontal { dhm_m: f64, dhp_m: f64 },
}

/// A3 §5.10.2 — baie vitrée. Chaque jeu de facteurs existe « sans » (sp) et « avec » (ap)
/// protection mobile, et en version Cep (-c) et DH (-e).
pub struct BaieType {
    pub id: Id,
    pub u_sp: UBaie,                       // Usp,vert / Usp,horiz
    pub u_ap: Option<UBaie>,               // Uap,vert / Uap,horiz
    pub sans_protection: FacteursBaie,     // Sw1, Sw2, Sw3 (-c et -e), Tli, Tlid
    pub protection_mobile: Option<ProtectionMobile>,
}
pub enum ProtectionMobile {
    VoletOuStoreEnroulable { facteurs: FacteursBaie, gestion: GestionPm },
    StoreVenitien { matrices: MatricesVenitien, gestion: GestionPm }, // M_Sw_BC, M_Sw_E, M_Tl_BCE…
}
pub struct Baie {
    pub type_baie: Id,
    pub a_m2: f64,
    pub exposition: Exposition,
    pub classement_bruit: ClasseBr,        // BR1 à BR3, A3 §5.11
    pub ouverture: OuvertureBaie,          // A3 §5.13
}

/// A3 §5.19.2 — pont thermique.
pub struct PontThermique {
    pub psi_w_mk: f64,
    pub l_m: f64,
    pub sf_c: f64,
    pub sf_e: f64,
    pub exposition: Exposition,
}

/// A3 §5.21.2 — inertie du groupe : soit une classe au sens des règles Th-I,
/// soit les valeurs surfaciques, pour chacun des trois horizons.
pub struct Inertie {
    pub quotidienne: NiveauInertie,   // Typeinertie_quo ou (Amq_surf, Cmq_surf)
    pub sequentielle: NiveauInertie,  // Typeinertie_seq ou (Ams_surf, Cms_surf)
    pub annuelle: NiveauInertie,      // Typeinertie_ann ou (Ama_surf, Cma_surf)
}
```

## Systèmes

Les systèmes ne sont pas détaillés à ce stade : 65 fiches restent à lire (phase 6). La structure proposée suit les assemblages du texte (A3 §8.6, §8.14, §9.1 à 9.4) :

```rust
pub struct SystemesBatiment {
    pub generations: Vec<Generation>,          // §8.14, §8.17 : générateurs + gestion des priorités
    pub distributions_intergroupes: Vec<DistributionIntergroupe>, // §8.9–8.12, §9.8
    pub productions_ecs: Vec<ProductionEcs>,   // §9.1–9.4
    pub ventilations: Vec<SystemeVentilation>, // ch. 6
    pub deplacements: Deplacements,            // ch. 10 : ascenseurs, escalators, parkings
}

/// Un variant par fiche générateur. Un système non encore implémenté est un variant
/// reconnu à la lecture, qui produit `Erreur::NonPrisEnCharge { fiche }` au calcul.
pub enum Generateur {
    EffetJoule(EffetJoule),                    // §8.18
    ChaudiereGazFioulBois(Chaudiere),          // §8.19
    ThermodynamiqueElec(ThermodynamiqueElec),  // §8.23
    ReseauChaleurFroid(Reseau),                // §8.28
    // … un variant par fiche des §8.18 à 8.30 et du ch. 9
}
```

Les liens entre objets (un émetteur vers sa distribution, une distribution vers sa génération) sont des `Id` en entrée et des indices typés (`IdxGroupe`, `IdxGeneration`) dans `modele`, vérifiés une fois pour toutes à la résolution.

## Énumérations issues du texte

```rust
/// A3 §13.4.3, tableau 338 — colonnes de la matrice des consommations.
pub enum Energie { Gaz /*10*/, Fioul /*20*/, Charbon /*30*/, Bois /*40*/, Electricite /*50*/, ReseauChaleur /*60*/ }

/// A3 §13.4.3 — postes. Les trois premiers sont les lignes du tableau 338.
pub enum Poste {
    Chauffage /*1*/, Refroidissement /*2*/, Ecs /*3*/,
    Eclairage, AuxVentilation, AuxDistribution, Deplacement, Mobilier,
}

/// Question Q9 : les codes numériques diffèrent selon les fiches.
pub enum ModeCalcul { ThB, ThC, ThD }

/// A3 §5.17.2 « Incl ».
pub enum SensFlux { Verticale /*1*/, HorizontaleFluxAscendant /*2*/, HorizontaleFluxDescendant /*3*/ }

/// A3 §4.1.3, tableau 4 (p. 57) : 28 usages, chacun avec les drapeaux i_hebergement et
/// i_enseignement. Les 28 variants existent dès la phase 2 ; seuls 1 et 2 sont calculés
/// en première version, les autres renvoient `Erreur::NonPrisEnCharge`.
pub enum UsageZone { MaisonIndividuelle /*1*/, LogementCollectif /*2*/, Bureaux /*3*/, /* … 28 */ }

pub enum ZoneClimatique { H1a, H1b, H1c, H2a, H2b, H2c, H2d, H3 }
```

## Données conventionnelles

### Météo : jamais embarquée

Les fichiers météo réglementaires ne peuvent être ni versionnés ni diffusés (question Q1). Le moteur ne contient donc aucune donnée météo : l'appelant la fournit.

```rust
// crates/thbce-core/src/climat/donnees.rs — structure interne unique, quelle que soit la source.

/// Une année-type pour une zone et un mode : 8 760 pas, altitude zéro.
/// Colonnes du fichier réglementaire ; correspondance avec A3 §3.1.2 dans la question Q1b.
pub struct AnneeMeteo {
    pub zone: ZoneClimatique,
    pub jeu: JeuMeteo,
    pub origine: OrigineMeteo,
    pub te0_c: Box<[f64]>,        // te0
    pub we0_g_kg: Box<[f64]>,     // we0
    pub idn_w_m2: Box<[f64]>,     // dirN_W_m2
    pub idi_w_m2: Box<[f64]>,     // diff_W_m2
    pub teciel_c: Box<[f64]>,     // Teciel_C
    pub vent_m_s: Box<[f64]>,     // Vent_m_s
    pub teau0_c: Box<[f64]>,      // Teau_C
    pub gamma_rad: Box<[f64]>,    // Gamma, converti des degrés
    pub psi_rad: Box<[f64]>,      // Psi, converti des degrés
}

pub enum JeuMeteo { ThBC, ThD }   // besoins et consommations ; confort d'été (séquence caniculaire)

/// Reportée telle quelle dans les résultats : un calcul n'est « réglementaire »
/// que si ses deux jeux météo le sont.
pub enum OrigineMeteo { ReglementaireRe2020, AdemeProspectif { scenario: String }, Autre { description: String } }

/// Ce dont un calcul a besoin : les deux jeux de la zone du projet.
pub struct MeteoProjet { pub th_bc: AnneeMeteo, pub th_d: AnneeMeteo }
```

Les lecteurs vivent dans `thbce-io`, derrière des fonctions qui prennent des octets et non un chemin, pour servir aussi le WASM :

| Lecteur | Source | Usage |
|---|---|---|
| `meteo::lire_csv_re2020(&[u8])` | CSV long `zone, mode, h, te0, …, Psi` | usage courant, tests locaux (`THBCE_METEO`) |
| `meteo::lire_classeur_re2020(&[u8])` | classeur officiel, 16 onglets `<zone>_<mode>` | même contenu, format d'origine |
| `meteo::lire_csv_ademe(&[u8])` | fichiers ouverts ADEME / Météo-France | CI publique, jamais réglementaire (question Q16) |

Conséquences sur les interfaces prévues au départ :

- CLI : `thbce calcul <entree> --meteo <fichier>` ; sans `--meteo`, erreur explicite. `thbce version` n'annonce aucune donnée météo embarquée.
- WASM : `calculer(entree, meteo: Uint8Array, options?)`. Le chargement à la demande d'une zone, prévu pour limiter la taille du binaire, n'a plus d'objet.
- `thbce-data` ne garde une feature `embedded-data` que pour les scénarios et les tables de constantes.
- Le calendrier (jour de semaine, semaine du mois, heure légale) n'est pas lu de la météo : il est calculé d'après A3 §4.1.3 (p. 56).

### Scénarios : générés depuis le classeur officiel

`tools/scenarios` (binaire Rust du workspace, hors publication) lit `docs/reference/scenarios/2026-04-29_scenarios_conventionnels.xlsx` et écrit un fichier de données versionné dans `crates/thbce-data/`. La CI relance le script et échoue si le fichier généré diffère.

```rust
// crates/thbce-data/src/scenarios.rs — structure d'après l'onglet MI et A3 §4.1.2.

/// Un scénario = une grille hebdomadaire (jour, heure) combinée à une grille annuelle (mois, semaine).
pub struct Grilles<T> {
    pub hebdo: [[T; 24]; 7],      // p^s(j, h)
    pub annuel: [[T; 5]; 12],     // p^a(m, s) ; 5e semaine absente selon le mois (A3 p. 56)
}

pub struct ScenariosZone {
    pub usage: UsageZone,
    pub consignes: Consignes,               // normal, arrêt < 48 h, arrêt > 48 h, en chaud et en froid
    pub mobilite: Grilles<f64>,
    pub occupation: Grilles<bool>,
    pub chauffage: Grilles<NiveauConsigne>, // 1 normal, 0 réduit < 48 h, −1 réduit > 48 h
    pub refroidissement: Grilles<NiveauConsigne>,
    pub ventilation: Grilles<bool>,
    pub eclairage: Grilles<bool>,
    pub ecs: ScenarioEcs,                   // clé de répartition horaire, facteur correctif de la semaine
    pub locaux: Vec<ScenariosLocal>,        // ratio de surface, occupants, apports de chaleur et d'humidité
}
```

Les types exacts de chaque grille seront fixés par le script, après lecture des 28 onglets.

### Constantes réglementaires

| Donnée | Source | Forme |
|---|---|---|
| Coefficients d'énergie primaire | arrêté, article 9 ; A3 tableau 333 | table par `Energie` |
| Facteurs d'émission des énergies | arrêté, article 10 ; fiches INIES « [RE2020] » en recoupement | table par énergie et par usage |
| Réseaux de chaleur et de froid | annexe 7 de l'arrêté du 15 septembre 2006 (colonnes 5 et 6) | table générée par script, clé = identifiant du réseau |
| Pondération dynamique `fCO2(a)` | arrêté, article 11 | **manquante** (question Q5) |
| Température de base par département | A3 §3.1.3 (p. 31) | table par `Departement` |

## Résultats

```rust
pub struct Resultats {
    pub version_moteur: String,
    pub version_methode: String,           // « Th-BCE 2020, arrêté du 4 août 2021 consolidé au 19 mars 2026 »
    pub reglementaire: bool,               // faux dès qu'un jeu météo n'est pas `ReglementaireRe2020`
    pub origine_meteo: OrigineMeteo,
    pub batiments: Vec<ResultatsBatiment>,
}

pub struct ResultatsBatiment {
    pub id: Id,
    pub sref_m2: f64,                      // A3 éq. (2378)
    pub bbio_pts: f64,                     // A3 éq. (2385)–(2386)
    pub cep_kwhep_m2: Option<f64>,         // A3 §13.2 ; None tant que la phase 7 n'est pas livrée
    pub cep_nr_kwhep_m2: Option<f64>,
    pub dh_ch: Option<f64>,                // A3 éq. (2551), A2 §5.1
    pub ic_energie_kgco2_m2: Option<f64>,  // A2 éq. (74)
    pub besoins: BesoinsMensuels,          // chaud, froid, éclairage : 12 mois + annuel
    pub consommations: MatricePosteEnergie,// énergie finale par poste et par énergie
    pub production: ProductionLocale,      // PV, cogénération, autoconsommation (A3 éq. 2527)
    pub zones: Vec<ResultatsZone>,         // même structure, avec les groupes
    pub horaire: Option<SeriesHoraires>,   // sur demande uniquement
}
```

Les valeurs sont restituées arrondies selon la règle du §1.3.1, sous réserve de la question Q8.

## Erreurs

```rust
#[derive(thiserror::Error, Debug)]
pub enum Erreur {
    #[error("{chemin} : {message}")]
    Saisie { chemin: String, message: String },       // « batiments[0].zones[1].groupes[0].baies[3].a_m2 »
    #[error("référence inconnue {id} depuis {chemin}")]
    ReferenceInconnue { chemin: String, id: String },
    #[error("fiche {fiche} non prise en charge par cette version")]
    NonPrisEnCharge { fiche: &'static str },
    #[error("donnée conventionnelle absente : {0}")]
    DonneeConventionnelle(String),
    #[error("le calcul n'a pas convergé : {contexte}")]
    NonConvergence { contexte: String },
}
```

## Déroulement du calcul

Conforme à l'enchaînement du §5.1.3 de l'annexe III :

1. `entree → modele` : validation, résolution des références, jointure des données conventionnelles, préallocation.
2. Boucle horaire unique sur 8 760 pas, dans l'ordre du texte : climat extérieur → espaces tampons → zones (scénarios, environnement proche) → groupes (baies, parois, ponts thermiques, débits d'air, comportement thermique, émission) → distribution → génération → production PV.
3. Post-traitement : cumuls mensuels et annuels par groupe, zone, bâtiment ; indicateurs.

Un calcul complet enchaîne plusieurs passes de cette boucle, une par mode (Th-B pour le Bbio, Th-C pour les consommations, Th-D pour DH). Le détail des passes et de leurs dépendances (les saisons, par exemple, sont déterminées par groupe puis harmonisées au niveau du bâtiment, §5.1.3) sera précisé à la lecture des fiches 4.6 et 8.4.

## Cas de validation

Aucun cas de référence n'existe encore (question Q6). Le format et le harnais sont livrés en phase 2 pour qu'ajouter un cas se limite à déposer un dossier :

```
tests/cas/<nom-du-cas>/
├── entree.json        # projet au format JSON natif (ou entree.xml quand le lecteur RSEE existera)
├── attendu.json       # résultats de référence, même schéma que `Resultats` ; champs absents = non comparés
└── cas.toml           # source (COMETH x.y, logiciel évalué…), jeu météo requis, tolérances propres au cas
```

- Tolérance par défaut : écart relatif ≤ 0,5 % sur Bbio, Cep, Cep,nr et DH ; surchargeable par indicateur dans `cas.toml`.
- Le harnais (`tests/cas.rs`) parcourt les dossiers, lance le calcul, compare et affiche un tableau des écarts par indicateur.
- Un cas qui demande la météo réglementaire est ignoré si `THBCE_METEO` n'est pas défini ; la CI publique ne lance que les cas déclarés sur météo ADEME.
- Le même harnais sert au test de parité natif / WASM (`wasm32-wasip1`), qui compare les sorties au bit près.

## Points à valider

1. La séparation `entree` / `modele` / `resultats` et le catalogue de composants référencés par identifiant.
2. Le niveau `Projet` au-dessus de `Batiment` dès la première version (nécessaire pour le PV de parcelle).
3. Le choix d'énumérations fermées avec erreur « non pris en charge » pour les systèmes non encore implémentés.
4. Les noms de champs : noms du texte suffixés par l'unité, en français.
5. La météo passée en octets par l'appelant, avec l'origine reportée dans les résultats et le drapeau `reglementaire`.
6. Le format de `tests/cas/`.
