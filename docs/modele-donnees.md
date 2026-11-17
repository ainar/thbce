<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Proposition de modèle de données

**Statut : validé par le mainteneur le 16 novembre 2026.** Les types sont implémentés dans `crates/thbce-core/src/entree.rs` ; le JSON Schema (`thbce schema`) fait foi pour le format exact. Ce document garde la vue d'ensemble et les intentions.

## Écarts de la phase 2 par rapport à la proposition

- **Seuls les champs vérifiés dans une nomenclature sont implémentés.** Sont reportés à la lecture de leur fiche : les locaux, l'éclairage, la ventilation, l'émission, les systèmes, le photovoltaïque, les stores vénitiens, le masque d'arbres à feuilles caduques, l'ouverture et le classement au bruit des baies, le ratio EnR des réseaux.
- **`thbce-core` dépend de `thbce-data`**, et non l'inverse : `Departement` et `ZoneClimatique` sont définis avec leurs tables dans `thbce-data`, qui reste sans entrée-sortie.
- **La validation est dans `thbce-core`** (`validation.rs`), pour servir aussi le module WebAssembly ; `thbce-io` n'en fait que la mise en forme.
- **La zone climatique est déduite du département** ; `zone_climatique` devient un champ facultatif, et une valeur contradictoire déclenche un avertissement.
- **Un seul lecteur météo** pour l'instant (`meteo::lire_csv_re2020`) ; le classeur et les fichiers ADEME viendront au lot 3. La provenance non réglementaire d'un fichier se déclare par une ligne `# thbce-origine:`.
- **Résultats** : `origine_meteo_th_bc` et `origine_meteo_th_d` remplacent `origine_meteo` ; `valeurs_arrondies` indique si l'arrondi de restitution est appliqué.
- **Surface de la baie** : `a_b_m2` (« Ab », §5.10.2). **Pont thermique** : `exposition` facultative depuis le lot 4c (voir plus bas).
- **Masques (phase 3)** : un masque lointain azimutal se saisit par ses 36 hauteurs `gamma_i_deg`, une par tranche de 10° repérée par rapport à la normale au composant ; le masque d'arbres à feuilles caduques a la même forme. Un masque proche exige la largeur (`lpb_m`) ou la hauteur (`hpb_m`) du composant.
- **Résultats (phase 3)** : bloc `climat` avec la zone, `altcorr_m`, `base_ext_c` et la période de confort adaptatif de chaque jeu météo.
- **Lecteurs météo (phase 3)** : `meteo::lire` reconnaît le classeur `.xlsx`, le CSV dérivé et les fichiers ADEME. Le module WebAssembly ne lit pas le classeur.
- **Surfaces (lot 4b)** : `su_m2` est la surface utile du groupe (« Agr », §4.1.2), obligatoire pour tous les usages ; `shab_m2` reste la surface de référence en habitation. En maison individuelle, on saisit en pratique `su_m2 = shab_m2` ; en logement collectif, la surface habitable interne au calcul (`Rat_hab · Az`, éq. 33) est distincte de la SHAB saisie (décision Q28). `nb_lgt` est obligatoire en logement collectif. Les résultats de zone portent `nadeq`.
- **Parois opaques et ponts thermiques (lot 4c)** : un type de paroi porte `incl_k`, obligatoire, position dans laquelle `u_k_w_m2k` a été déterminé (`verticale`, `horizontale_flux_ascendant`, `horizontale_flux_descendant`, §5.17.3) ; le moteur en déduit U à l'inclinaison réelle. Un pont thermique accepte une `exposition` facultative : sans elle, la description simplifiée du §5.18.3 s'applique (quatre ponts verticaux de `l/4` aux quatre orientations, sans masque). Les résultats de groupe portent un bloc `enveloppe` (surfaces de parois et de baies, longueur de ponts thermiques, coefficients HTh, et Hges des baies sans protection mobile en place).
- **Baies (lot 4c)** : une baie est inclinée de 0 à 90° ; son type doit donner `u_sp_horiz_w_m2k` si elle est inclinée de moins de 60° et `u_sp_vert_w_m2k` à partir de 30° (§5.10.3.5, éq. 207). La position des protections mobiles, calculée par la fiche 5.9, n'est pas encore une entrée du projet.
- **Protections mobiles (lot 4c)** : `protection_mobile` sur le type de baie, avec `type_pm` (`volet`, `store_enroulable`, `store_venitien`), `gestion` (`manuelle_non_motorisee`, `manuelle_motorisee`, `automatique_avec_derogation`), `exterieure`, `seconde_protection` et `typo_pm` facultatif (§5.12.3.1, typologie 0 par défaut, signalée). Le store vénitien et la gestion automatique sont refusés par la validation dans cette version. Une protection déclarée exige `avec_protection` et un coefficient `u_ap_…`.
- **Surventilation naturelle (lot 4c)** : `ouvrant` sur le type de baie (`r_ouv_max` ou `type_ouvrant`, l'un des deux ; absent, la baie n'est pas ouvrable), `expo_br` sur la baie (`br1`, `br2`, `br3`, requis si le type a un ouvrant), `traversant` sur le groupe (conventionnel en maison individuelle, à ne pas saisir ; requis en logement collectif si des baies sont ouvrables) et `h_ttf_m` facultatif (1,5 m par défaut ; saisie admise entre 4 et 15 m). Les résultats de groupe portent `valeurs_par_defaut`.
- **Ventilation (lot 4d1)** : `bouches_conduits` sur le groupe, au moins une en habitation. Chaque élément est un circuit de reprise ou de soufflage : `id`, `isouf` (`reprise` ou `soufflage`), `q_spec_conv_pointe_m3_h` et `q_spec_conv_base_m3_h`, débits d'hygiène de pointe et de base de la ventilation installée pour le groupe (§6.2.2 ; la méthode ne donne pas les débits réglementaires par nombre de pièces). Pour un système hygroréglable, on saisit `qvarep_spec_m3_h` (valeur de l'avis technique) à la place du débit de base, qui en est déduit (§6.2.3.1.1.2). Le débit de pointe est au moins égal au débit de base. Les résultats de groupe portent `debits_bbio` (`q_spec_repris_m3_h`, négatif, et `q_spec_souffle_m3_h`) : débits du système de ventilation conventionnel du Bbio (§6.1 ; lecture Q36).
- **Bilan aéraulique (lot 4d1)** : sur la zone, `h_zone_m`, `h_plancher_m` et `q4pa_surf_m3_h_m2` (perméabilité sous 4 Pa, par m² de parois déperditives hors plancher bas), tous trois obligatoires (§5.6.2), et `trav_zone` (caractère traversant au sens des conditions d'hiver : conventionnel en maison individuelle, à ne pas saisir ; requis en logement collectif, tableau 22). Sur le groupe, `entrees_air` facultatif : `m_m3_h` (somme des modules), `autoreglable` (`delta_p1_pa`, `delta_p2_pa`, `m_prime_m3_h`, les deux pressions étant communes à la zone) et `r` (anti-retour, de 0 à 1 ; absent : pas de dispositif). Les résultats d'enveloppe portent `a_f_perm_m2` et `a_toit_perm_m2`.
- **Calcul du Bbio (lot 4d2)** : sur le groupe, `v_m3`, volume du groupe, obligatoire (§8.3.2 ; le bilan hydrique en tire la masse d'air du groupe, et le texte ne lui donne pas de valeur par défaut). Résultats : `bbio` au niveau du groupe, de la zone et du bâtiment, avec `annuel` et `mensuel` (douze mois de l'année conventionnelle des scénarios), chacun portant `b_ch_kwh_m2`, `b_fr_kwh_m2`, `b_ecl_kwh_m2` (kWh par m² de surface de référence) et `bbio_pts` ; `bbio_pts` du bâtiment reprend la valeur annuelle ; sur le groupe, `saisons_bbio` (`j_fin_ch`, `j_debut_ch`, `j_debut_fr`, `j_fin_fr`, jours de 1 à 365 au sens des scénarios) et `confort_bbio` (heures et degrés-heures d'inconfort en évolution libre, §4.5). Chaque valeur est arrondie séparément à une décimale ; `--sans-arrondi` (`arrondir: false`) donne les valeurs brutes, pour lesquelles le Bbio annuel est la somme des Bbio mensuels. `reglementaire` est faux dès que la météo n'est pas celle de la RE2020. Deux contrôles de cohérence s'ajoutent à la validation : un groupe dont les parois opaques et les ponts thermiques conduisent plus que l'échange superficiel de ses parois lourdes (`HTH,k+pt ≥ Hgmqs`, éq. 339) et une zone ventilée sans perméabilité ni entrée d'air (éq. 167) sont refusés. Nouvelle nature d'erreur, `calcul` (`Erreur::Calcul`), pour un calcul qui n'aboutit pas sur une entrée valide.
- **Confort d'été (lot 5)** : aucun changement du format d'entrée. Les champs d'inertie `sequentielle`, `ams_surf_m2_m2`, `annuelle` et `ama_surf_m2_m2`, déjà obligatoires, sont désormais employés (calcul Th-D, §5.21.3.10) ; la validation avertit, sans refuser, si la capacité séquentielle est inférieure à la capacité quotidienne, la capacité annuelle inférieure à la capacité séquentielle, ou une surface d'échange supérieure à `Amq_surf` (l'inertie correspondante est alors sans effet). Avec les classes « par défaut », ou avec `ams_surf_m2_m2` égal à `Amq_surf`, ces inerties n'ont aucun effet. Résultats : `dh_c_h` au niveau du bâtiment et de la zone (DH du groupe le plus défavorable, le DH étant défini par groupe) ; sur le groupe, `confort_ete` (`dh_c_h`, `nbh_inconf`, `nbh_inconf_1c`, `nbh_inconf_2c`, `nbh_occ_conf_adapt`, `theta_op_max_c`, `theta_op_max_occ_c`) et `saisons_th_d`. Séries horaires : `thbce_core::series_horaires(projet, meteo, mode)` renvoie, pour chaque groupe, les 8 760 sorties horaires du mode demandé (températures, besoins, débit de surventilation, saison, inerties et, en Th-D, température d'inconfort chaud et degrés-heures du pas de temps) ; elles ne sont pas dans le JSON des résultats.
- **Espaces tampons (lot 4b, espaces tampons)** : changement compatible, champs facultatifs. Sur le bâtiment, `espaces_tampons` : `id` et `nature`, soit `non_solarise` (`b_et_ns`, coefficient `b` des règles Th-Bat, de −5 à 1), soit `solarise` (`a_et_m2`, `u_et_pb_w_m2k`, `h_ttf_et_m`, `cm_et_surf_kj_m2k` facultatif — 200 par défaut, tableau 19 —, `qv_et_base_m3_h` facultatif — 1 m³/h par m² de sol par défaut, §5.4.3.3.1.4 —, `protections_mobiles` facultatif avec `exterieures`, et `parois` : `id`, `a_ue_m2`, `u_ue_w_m2k`, `exposition`, `swe_sp`, `swe_ap`, `tle_sp`, `tle_ap`, `r_ouv_max` ; `swe_ap` et `tle_ap` requis avec des protections mobiles). Une paroi opaque, une baie, un pont thermique et les entrées d'air d'un groupe désignent par `espace_tampon` l'espace tampon sur lequel ils donnent ; absent, ils donnent sur l'extérieur. La validation refuse une référence inconnue, une baie donnant sur un espace non solarisé (§5.10.3) et un espace tampon sur lequel ne donne aucun composant d'enveloppe (§5.4.1). Les grandeurs d'enveloppe des résultats restent celles « sans les espaces tampons ». Résultats : `espaces_tampons` sur le bâtiment (absent sans espace tampon), avec, pour chaque mode (`th_b`, `th_d`), `b_therm_moyen`, `theta_et_moy_c`, `theta_et_min_c`, `theta_et_max_c`, `nbh_surventilation`, et `valeurs_par_defaut`. Le calcul simule désormais le bâtiment d'un bloc (`BatimentHoraire`) : espaces tampons puis zones à chaque pas de temps. Lectures : Q41.
- **Ventilation du projet (lot 6a)** : changement incompatible. Une zone d'habitation porte `ventilation`, obligatoire : `id`, `systeme` et `reseau`, `puits` facultatif (refusé). `systeme` est l'un de `simple_flux_extraction`, `simple_flux_insufflation`, `mecanique_basse_pression` (chacun `p_vent_base_w`, `p_vent_pointe_w`, puissances du ventilateur en débit de base et de pointe) ou `double_flux` (`p_vent_base_rep_w`, `p_vent_pointe_rep_w`, `p_vent_base_sou_w`, `p_vent_pointe_sou_w` ; `echangeur` facultatif : `representation` — `simplifiee` (`epsilon_saisi`, `statut` facultatif : `declare` par défaut, `justifie`, `certifie`) ou `detaillee` (`ua_w_k`, `ecoulement` : `contre_courant`, `courants_paralleles`, `courants_croises_brasses`) —, `bypass` facultatif (`theta_ext_bypass_hiver_c`, `theta_ext_bypass_ete_c`, `theta_int_bypass_hiver_c`, `theta_int_bypass_ete_c`), `antigel` facultatif, `theta_s_ech_rep_lim_c` facultatif (5 °C par défaut, tableau 62), `rotatif` refusé ; `prechauffage`, `humidification`, `rafraichissement_evaporatif`, `surventilation_mecanique` refusés). Les valeurs `naturelle_par_conduits`, `hybride`, `aeration` et `centrale_traitement_air` sont reconnues et refusées avec un message explicite. `reseau` : `b` (locaux non chauffés traversés par les conduits, de 0 à 1), `r_rep_m2k_w` et `r_souf_m2k_w` (résistances des conduits hors volume chauffé, requises selon les sens des bouches), `classe_etancheite` facultative (`A`, `B`, `C` ; absente : Kres par défaut, tableau 59), `ratfuitevc` facultatif (requis avec des surfaces saisies ; absent : tableau 60). Les bouches (`bouches_conduits` du groupe) gagnent `dugd_h` (14 h par défaut, tableau 57), `cdep` (1,30 par défaut, tableau 58) et `a_cond_m2` (surface de conduit saisie, pour toutes les bouches de la zone ou aucune ; sinon éq. 421 à 427). Le sens des bouches doit correspondre au système. Le calcul du Bbio n'emploie ni le système ni ces trois champs (§6.1, décision Q36). Résultats : `ventilation` sur la zone (`id`, `p_vent_w`, `c_aux_vent_kwh`, `th_d` avec `nbh_bypass` et `q_ag_kwh`, `valeurs_par_defaut`, champs relatifs à la zone) et sur le groupe (`q_spec_repris_m3_h`, `q_spec_souffle_m3_h`, `q_repris_cond_m3_h`, `q_souffle_cond_m3_h`, `p_vent_w`, `c_aux_vent_kwh`, `c_aux_vent_kwh_m2`, `c_aux_vent_mensuel_kwh`), `c_aux_vent_kwh` sur le bâtiment : consommation des ventilateurs en kWh d'énergie finale électrique. Le calcul Th-D emploie le système du projet (lecture Q42). Lectures : Q42.
- **Valeurs par défaut (règle générale)** : le moteur n'applique une valeur par défaut que lorsque le texte de la méthode la donne, et la signale toujours dans les résultats. Premier cas : les facteurs de baie `sw1_c`, `sw2_c`, `sw3_c` (ensemble) et `tlid`, facultatifs (§5.10.3.6.3, éq. 215, 216 et 218) ; les résultats de groupe portent `baies[].valeurs_par_defaut` (`champ`, `valeur`, `regle`). `su_m2` et `incl_k` restent obligatoires : le texte ne leur donne pas de valeur par défaut.
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
    pub h_zone_m: Option<f64>,     // A3 §5.6.2 (lot 4d1), comme h_plancher_m, q4pa_surf_m3_h_m2, trav_zone
}

pub struct Groupe {
    pub id: Id,
    pub surfaces: SurfacesGroupe,              // SHAB ou SU → SREF (A3 éq. 2375–2376), Agr
    pub v_m3: f64,                             // A3 §8.3.2 : volume du groupe (bilan hydrique)
    pub categorie_ce1_ce2: CategorieCe,        // A3 éq. (2387)
    pub inertie: Inertie,                      // A3 §5.21.2
    pub parois_opaques: Vec<ParoiOpaque>,
    pub baies: Vec<Baie>,
    pub ponts_thermiques: Vec<PontThermique>,
    pub locaux: Vec<Local>,                    // par défaut : ratios conventionnels de l'usage
    pub eclairage: Eclairage,                  // A3 §7.1 (phase 4)
    pub bouches_conduits: Vec<BoucheConduit>,  // A3 §6.2 : débits d'hygiène de pointe et de base (lot 4d1)
    pub ventilation: VentilationGroupe,        // ch. 6 (phase 6)
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
    pub incl_k: PositionParoi,     // position dans laquelle Uk a été déterminé (§5.17.3)
    pub sf_c_k: f64,               // facteur solaire, calcul du Cep
    pub sf_e_k: f64,               // facteur solaire, calcul de DH
    pub vegetalisation: Option<Vegetalisation>, // sf_c_v_k, sf_e_v_k, saison végétative
}
pub struct ParoiOpaque {
    pub type_paroi: Id,
    pub a_k_m2: f64,
    pub exposition: Exposition,
    pub espace_tampon: Option<Id>, // §5.17.3 ; de même sur la baie, le pont thermique et les entrées d'air
}

/// A3 §3.2.2 — paramètres d'intégration communs à toute surface exposée.
pub struct Exposition {
    pub azimut_deg: f64,           // 0 sud, 90 ouest, 180 nord, 270 est
    pub inclinaison_deg: f64,      // 0 horizontale vers le haut, 90 verticale, 180 vers le bas
    pub lpb_m: Option<f64>,        // largeur et hauteur : requises pour les masques proches
    pub hpb_m: Option<f64>,
    pub masques: Vec<Id>,          // 0 à 7 masques (Nmasque)
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

### Format d'entrée des systèmes de chauffage et de refroidissement (lot 6b)

Implémenté au lot 6b (`crates/thbce-core/src/entree.rs`, lectures `docs/questions.md`, Q43). Les liens sont des identifiants, vérifiés par la validation :

```
Batiment
├── generations[]                 { id, raccordement_reseaux: avec_isolement | permanent,       §8.4.2
│                                   position, priorite?, gestion_temperature_chauffage?,        §8.17.2
│                                   gestion_temperature_refroidissement?, generateurs[],
│                                   stockages[]?, chauffe_eau_thermodynamiques[]? }             §9.14, §9.22
│   └── generateurs[]             { id, rdim, priorite_chauffage?, priorite_refroidissement?,
│                                   priorite_ecs?, nature: effet_joule { fonction, pn_gen_w }   §8.18
│                                   | thermodynamique_electrique { compresseur,                 §8.23
│                                     taux_auxiliaires?, source_amont, chauffage?,               §8.26
│                                     refroidissement? } }
├── distributions_intergroupes[]  { id, fonction: chauffage | refroidissement, generation, reseau } §8.9, §8.10
└── Zone[] → Groupe[]
    ├── iclim                     groupe climatisé, requis en habitation                        §8.1.2
    ├── programmation             { chauffage: horloge | horloge_controle_ambiance | optimiseur,
    │                               refroidissement: horloge | horloge_controle_ambiance | sans_horloge }  §8.5
    ├── distributions[]           { id, fonction, distribution_intergroupes, reseau }            §8.7, §8.8
    └── emetteurs[]               { id, chauffage?, refroidissement?, pertes_au_dos?,
                                    detection_presence?, ventilateurs? }                         §8.1
        ├── chauffage             { type_emetteur (tableau 85), variation_spatiale (classe A…C et hsp_m,
        │                           poêle ou insert, ou valeur), variation_temporelle (certifiée,
        │                           justifiée, défaut du tableau 87, poêle ou insert), effet_joule,
        │                           rats, ratt, distribution }
        ├── refroidissement       { type_emetteur (tableau 86), variation_spatiale, variation_temporelle,
        │                           rats, ratt, distribution }
        ├── pertes_au_dos         { r_i_m2k_w, u_e_w_m2k } : Pper = Ri·Ue (éq. 795)
        └── ventilateurs          { gestion, super_petite_vitesse, p_gv_w, p_mv_w, p_pv_w, p_spv_w?,
                                    q_recirc_{gv,mv,pv}_m3_h, regulation_batterie? }             §8.1.3.8
```

Eau chaude sanitaire (§9.5 à 9.8) :

```
Batiment.distributions_ecs_intergroupes[]  { id, generation, reseau: aucun | boucle { l_vc_m, l_hvc_m,
                                             u_w_mk, rechauffeur, p_circ_w, espace_tampon? } | trace { … } }
Groupe.emetteurs_ecs[]                     { id, rat_em, nb_logements, part_melangeur, part_mitigeur,
                                             part_temporisateur, appareils, recuperateur_eaux_grises?,
                                             distribution { theta_2nd_e_c?, d_int_m, l_vc_m?, l_hvc_m,
                                             nb_dist, espace_tampon? }, distribution_intergroupe }
```

Règles de saisie (validation) : au moins un émetteur de chauffage par groupe d'habitation ; `iclim` vrai si et seulement si un émetteur refroidit ; programmation du refroidissement requise si `iclim` ; somme des `Rats × Ratt` au plus 1 par fonction (§8.1.3.2) ; un émetteur est rattaché, pour chaque fonction, à un réseau du groupe de même fonction, lui-même rattaché à un réseau intergroupes de même fonction et de même type (éq. 850 et 851), lui-même relié à une génération ; variation temporelle saisie au moins égale à 0,2 K (effet Joule) ou 0,4 K en chaud, au plus −0,4 K en froid (§8.1.3.1.5) ; `Pper < 1`. Les réseaux sont `fictif` (effet Joule, poêles, détente directe) ou hydrauliques : dans le groupe, `hydraulique { regulation_debit, regulation_temperature, theta_dep_dim_c?, theta_ret_dim_c?, delta_theta_em_dim_k, q_nom_m3_h, q_resid_m3_h?, circulateur, p_aux_w, l_vc_m, l_hvc_m, u_vc_w_mk, u_hvc_w_mk, espace_tampon? }` (§8.7.2, §8.8.2) ; entre groupes, `hydraulique_collectif` ou `hydraulique_individuels { l_vc_m, l_hvc_m, u_vc_w_mk, u_hvc_w_mk, p_aux_w, circulateur, espace_tampon? }` (§8.9.2, §8.10.2). 

### Format d'entrée des générations (lot 6c)

Changement incompatible : une génération décrit ses générateurs (`docs/questions.md`, Q46).

- `position` (« idpos », §8.17.2), obligatoire : `en_volume_chauffe` ou `hors_volume_chauffe { espace_tampon? }` (sans espace tampon : à l'extérieur).
- `priorite` (« idtype_priorite ») : `sans_priorite` ou `cascade`, requis dès que deux générateurs assurent le même poste ; `alternance` est reconnu et refusé. En cascade, chaque générateur du poste porte un rang (`priorite_chauffage`, `priorite_refroidissement`, `priorite_ecs`, à partir de 1, tous distincts).
- `gestion_temperature_chauffage` et `gestion_temperature_refroidissement` (« idgestion », « θwm ») : `constante { theta_wm_c }` ou `reseaux`, requis si la génération alimente des réseaux hydrauliques du poste.
- `generateurs`, au moins un : `id`, `rdim` (nombre de générateurs identiques, au moins 1) et `nature`. Nature prise en charge : `effet_joule { fonction: chauffage | ecs, pn_gen_w }` (type 500, §8.18 ; production instantanée d'ECS, §9.1). Les natures `chaudiere`, `poele_ou_insert` et `reseau_de_chaleur_ou_de_froid` (lot 6d) sont décrites plus bas ; `autre_generateur_a_combustion`, `thermodynamique_gaz`, `boucle_d_eau_ou_drv`, `cogeneration` et `pac_hybride` (après v1) sont reconnues et refusées avec un message explicite.
- Nature `chaudiere` (types 100 à 102, 200, 201 et 400, §8.19 ; `docs/questions.md`, Q49) : `type_chaudiere` (`gaz_standard`, `gaz_basse_temperature`, `gaz_condensation`, `fioul_standard`, `fioul_condensation`, `bois`), `combustible_bois` (`buches`, `granules`, `plaquettes` ; requis pour le bois), `gpl` (faux : gaz naturel), `fonction` (`chauffage`, `ecs`, `chauffage_ecs`), `pn_gen_kw`, `pertes_parois` (`sans_ventilateur`, `avec_ventilateur`, `avec_clapet`, tableau 121), `rendements` facultatif (`statut` : `certifie`, `justifie` ou `declare`, réductions de 0, 10 et 20 % ; `r_pn_pct`, `r_pint_pct` sur PCS ; `p_int_kw` facultatif, 30 % de `Pn_gen` par défaut), `auxiliaires` facultatif (`w_aux_nom_w`, `w_aux_int_w`, `w_veille_w`), `q_po_30_w` facultatif, `alimentation_bois` (tableau 119 ; requis pour le bois dont les auxiliaires sont par défaut). Toute valeur absente prend sa valeur par défaut (éq. 1072 à 1077), signalée dans `generateurs[].valeurs_par_defaut`. La chaudière est en volume chauffé ou non selon `position` de sa génération. Elle alimente des réseaux hydrauliques et peut être le générateur de base d'un `stockages` (ballon chauffé par chaudière ; `h_rel_ech` non requis).
- Nature `poele_ou_insert` (types 403 et 404, §8.22) : `pn_gen_kw` (de 0 exclu à 100), `eta_pct` (rendement moyen), `p_aux_vent_w`. Chauffage seul, réseaux fictifs (air), énergie bois.
- Nature `reseau_de_chaleur_ou_de_froid` (types 600 et 601, §8.28) : `chaleur { identifiant_reseau?, fonction, type_reseau, classe_isolation_secondaire (1 à 4), p_ess_kw }` ou `froid { identifiant_reseau?, p_ess_kw }`. `type_reseau` : `eau_chaude_basse_temperature`, `eau_chaude_haute_temperature`, `vapeur_basse_pression`, `vapeur_haute_pression` (tableaux 247 et 248). `identifiant_reseau` (quatre chiffres et « C » ou « F ») est contrôlé contre la table de l'annexe 7 de l'arrêté du 15 septembre 2006 (`thbce_data::reseaux_chaleur`, extraite par `cargo run -p xtask -- reseaux-chaleur`), qui donne au lot 7 le facteur d'émission (5e colonne) et le taux d'EnR&R (6e colonne) ; absent, les lignes « autres réseaux » s'appliquent. Le moteur ne fait que valider l'identifiant : la saisie du réseau dans le calcul des indicateurs est au lot 7.
- Nature `thermodynamique_electrique` (type 503, §8.23 ; `docs/questions.md`, Q47) : `compresseur` (`puissance_variable` ou `tout_ou_rien`), `taux_auxiliaires` facultatif (`valeur`, `statut` : `certifie` ou `justifie` ; absent : 0,02, ou 0,01 pour un refroidisseur seul, signalé), `source_amont` (`air_exterieur { p_vent_gaine_w? }`, `air_ambiant { espace_tampon, p_vent_gaine_w? }`, `air_extrait { zone, t_air_lim_c, p_vent_gaine_w? }`), et un ou deux modes, `chauffage` et `refroidissement` : `technologie` (`air_exterieur_eau` ou `air_exterieur_air_recycle` ; les autres technologies du tableau 145 sont reconnues et refusées), `performances`, `limites` facultatives (`combinaison` : `l_une_ou_l_autre` ou `l_une_et_l_autre`, `theta_aval_c`, `theta_amont_c`), `typo_emetteur` (`plancher_ou_plafond_inertie_forte`, `radiateurs_ou_plafond_inertie_moyenne`, `ventilo_convecteurs_ou_inertie_faible`, `systemes_a_air`), `fonctionnement_continu` facultatif pour un compresseur à puissance variable (`lr_contmin`, `ccp_lr_contmin`, `statut` ; absent : 0,4 et 1, signalés). `performances` est l'un de `mesurees` (liste de points `theta_aval_c`, `theta_amont_c`, `performance` — COP ou EER —, `pabs_kw`, `statut`, couvrant le croisement de températures prises dans l'ordre de priorité de la technologie, valeur pivot comprise), `pivot_declare { performance, pabs_kw }` ou `pivot_par_defaut { pabs_kw }`.
- Mode `ecs` d'un générateur thermodynamique (§8.23.3.4) : `technologie` (`air_exterieur_eau`, `air_extrait_eau`, `air_ambiant_eau` ; les autres sont refusées), `performances`, `limites` facultatives. Un tel générateur est le générateur de base d'un assemblage de `stockages` ; avec le mode `chauffage`, il est double service.
- `stockages[]` (§9.9, §9.10 et §9.14 ; `docs/questions.md`, Q48) : `id`, `nb_assemblages` (égal au `rdim` du générateur de base), `ballon` (`v_tot_l`, `pertes` : `certifie { ua_s_w_k }`, `justifie { ua_s_w_k }` ou `par_defaut { nature }` avec `effet_joule_horizontal`, `effet_joule_vertical` ou `autre`, tableau 282 ; `theta_max_c`), `base` (`generateur`, `programmation` : `permanent`, `nuit` ou `jour`, `z_reg` de 1 à 4, `delta_theta_k` facultatif — 2 K par défaut —, `h_rel_ech` requis pour un générateur thermodynamique) et `appoint` facultatif (`generateur` à effet Joule de fonction `ecs`, `programmation`, `z_ap`, `z_reg`, `f_aux` facultatif — 0,5 par défaut —, `delta_theta_k` facultatif). La consigne est conventionnellement de 55 °C. Un ballon électrique se décrit par un générateur `effet_joule` de fonction `ecs` et un stockage dont il est la base.
- `chauffe_eau_thermodynamiques[]` (§9.22) : `id`, `nb_assemblages`, `v_tot_l`, `theta_max_c`, `essai` (`cycle` : `s`, `m`, `l`, `xl` ou `xxl` ; `t_h_h`, `p_es_w`, `cop_dhw`, `theta_wh_c`, `auxiliaires_compris`, `statut`), `source_amont` (air extérieur, air ambiant d'un espace tampon ou air extrait d'une zone), `programmation`, `limites` facultatives. Le moteur identifie le coefficient de pertes du ballon et les valeurs pivot du COP et de la puissance absorbée de la pompe à chaleur, restitués dans `stockages[].idcet` avec l'écart `epsilon`. Une génération compte au moins un générateur ou un chauffe-eau thermodynamique.
- Cohérence du montage (§8.17.3.1) : la génération assure chaque poste de ses réseaux ; ses réseaux intergroupes d'un même poste sont tous fictifs ou tous hydrauliques ; un générateur thermodynamique sur air recyclé alimente des réseaux fictifs, un générateur sur eau des réseaux hydrauliques ; de même une chaudière ou une sous-station de réseau de chaleur alimente des réseaux hydrauliques et un poêle des réseaux fictifs.
- Double flux à sécurité antigel : `echangeur.generation_antigel` désigne la génération qui fournit l'énergie de l'antigel (requis avec `antigel`).

Résultats : les consommations des générateurs à combustion sont restituées en kWh d'énergie finale PCI dans la colonne de leur énergie (`gaz_kwh`, `fioul_kwh`, `bois_kwh`, `reseau_kwh`), leurs auxiliaires en `electricite_kwh` du poste servi ; `eta_eff_*_an` est alors le rendement saisonnier (énergie fournie divisée par le total du poste, auxiliaires compris). `generations[]` porte les demandes (`q_ch_kwh`, `q_fr_kwh`, `q_ecs_kwh`), les énergies fournies (`q_fou_*_kwh`), `q_rep_fin_kwh`, `consommations`, `pertes_vc_kwh`, les compteurs de sous-dimensionnement et `generateurs[]` (énergies fournies, `consommations`, efficacités annuelles `eta_eff_*_an` — COP et EER saisonniers des générateurs thermodynamiques —, `w_aux_kwh`, `phi_rejet_kwh`, `valeurs_par_defaut`). `consommations` (`annuel` et `mensuel`, douze mois des scénarios) donne, par poste (`chauffage`, `refroidissement`, `ecs`) et par type d'énergie (`gaz_kwh`, `fioul_kwh`, `charbon_kwh`, `bois_kwh`, `electricite_kwh`, `reseau_kwh` ; les énergies sans consommation ne sont pas écrites), les kWh d'énergie finale ; il figure sur chaque génération et, réparti au prorata des demandes (éq. 1484 à 1486), sur chaque groupe, chaque zone et le bâtiment : c'est la matrice « Qcef[poste ; énergie] » des sorties du mode Th-C (§13.2), auxiliaires de génération compris. Une génération porte `stockages[]` : énergie fournie à la distribution (`q_fou_ecs_kwh`), énergies injectées par la base et par l'appoint, `pertes_kwh`, `v_puise_l`, `theta_haut_min_c`, `nbh_temp_insuff_max`, `idcet` et `valeurs_par_defaut`. La ventilation d'une zone porte `th_c` (heures de bypass et énergie d'antigel du calcul Th-C).

Les liens entre objets (un émetteur vers sa distribution, une distribution vers sa génération) sont des `Id` en entrée et des indices typés (`IdxGroupe`, `IdxGeneration`) dans `modele`, vérifiés une fois pour toutes à la résolution.

### Format d'entrée des postes du logement collectif et des parkings (lot 6e)

Facultatif ; l'éclairage des parties communes (§7.2) ne demande aucune saisie (toute zone de logement collectif). Lectures : `docs/questions.md`, Q50.

```
Batiment
├── ascenseurs[]   { id, tech_mac: traction_avec_reduction | traction_sans_reduction | hydraulique | autre,
│                    q_kg (≥ 75), v_m_s (≤ 2,5), h_m, netage (≥ 2), cp?, zones_desservies[] (zones de LC),   §10.1
│                    sc_veille?, pti_w?, dp1_w?, t1_s?, dp2_w?, t2_s? }
└── parkings[]     { id, type_parking: interieur | exterieur, npl, net,                                       §10.3, §10.4
                     ventilation? { regulee, p_vent600_w_par_place? },
                     eclairage { p_installee_w?, plages_detection_semaine[], plages_detection_week_end[]
                                 ({ debut_h, fin_h }, au plus 3), extinction_en_fermeture? },
                     jours_fermes[] }
```

Résultats : `consommations_collectif` (zone et bâtiment) : `eclairage_parties_communes`, `ascenseurs`, `ventilation_parkings`, `eclairage_parkings`, chacun `{ annuel_kwh, annuel_kwh_m2, mensuel_kwh[12] }` ; `ascenseurs[]` (`n_dem`, `t_mob_h`, `pti_w`, `e_tm_kwh`, `e_ti_kwh`, `e_tot_kwh`, `valeurs_par_defaut`) et `parkings[]` (`e_vent_kwh`, `e_ecl_kwh`, `valeurs_par_defaut`) au niveau du bâtiment.

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

Un calcul complet enchaîne plusieurs passes de cette boucle, une par mode (Th-B pour le Bbio, Th-C pour les consommations, Th-D pour DH). Depuis le lot 6b, `calculer` enchaîne Th-B, Th-D puis Th-C (systèmes du projet, saisons effectives et relances, générations idéales) ; depuis le lot 5, Th-B puis Th-D, par le même enchaînement horaire paramétré par `ModeCalcul` ; depuis le lot 4b (espaces tampons), chaque passe porte sur le bâtiment entier (`bati::assemblage_batiment::BatimentHoraire`) : à chaque pas de temps, les espaces tampons, à partir de l'état des groupes au pas précédent, puis les zones. Le détail des passes et de leurs dépendances (les saisons, par exemple, sont déterminées par groupe puis harmonisées au niveau du bâtiment, §5.1.3) sera précisé à la lecture des fiches 4.6 et 8.4.

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
