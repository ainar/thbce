<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Proposition de modèle de données

**Statut : proposition de phase 1, à valider avant tout code de calcul.**

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
    pub departement: Departement, // 01..95, 2A, 2B (question Q13)
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

/// A3 p. 24–26 : 28 usages. Seuls 1 et 2 sont dans le premier lot (question Q4).
pub enum UsageZone { MaisonIndividuelle /*1*/, LogementCollectif /*2*/, Bureaux /*3*/, /* … 28 */ }
```

## Données conventionnelles (`thbce-data`)

```rust
/// A3 §3.1.2 — une année-type, 8 760 pas de temps, à l'altitude zéro.
pub struct FichierMeteo {
    pub zone: ZoneClimatique,          // H1a … H3
    pub lat_deg: f64,
    pub long_deg: f64,
    pub heures: Box<[HeureMeteo; 8760]>,
}
pub struct HeureMeteo {
    pub te0_c: f64, pub we0_g_kg: f64,
    pub rdirn_w_m2: f64, pub rdiff_w_m2: f64,
    pub dteciel_k: f64, pub vent_m_s: f64, pub dir_vent_deg: f64,
    pub teau0_c: f64,
    // repères calendaires (jsem, EHLO, IHJ, ISEM, IMOIS…) : lus du fichier ou recalculés,
    // selon ce que contiennent les fichiers officiels (question Q1).
}

/// A3 §4.1.2 — scénarios d'une zone : indicateurs annuels (mois, semaine) et
/// hebdomadaires (jour, heure) pour l'occupation, les consignes, la ventilation,
/// l'éclairage, l'ECS et la mobilité ; taux par local.
pub struct ScenariosUsage { /* structure fixée à réception du classeur officiel (question Q2) */ }
```

Le contenu exact de ces deux structures dépend des fichiers officiels, qui ne sont pas dans les annexes (questions Q1 et Q2).

## Résultats

```rust
pub struct Resultats {
    pub version_moteur: String,
    pub version_methode: String,           // « Th-BCE 2020, arrêté du 4 août 2021 consolidé au 19 mars 2026 »
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

## Points à valider

1. La séparation `entree` / `modele` / `resultats` et le catalogue de composants référencés par identifiant.
2. Le niveau `Projet` au-dessus de `Batiment` dès la première version (nécessaire pour le PV de parcelle).
3. Le choix d'énumérations fermées avec erreur « non pris en charge » pour les systèmes non encore implémentés.
4. Les noms de champs : noms du texte suffixés par l'unité, en français.
