<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Modèle de données

Vue d'ensemble du format d'entrée et des résultats du moteur. **Le détail fait foi dans les JSON Schema** générés depuis les types Rust : `thbce schema` (entrée) et `thbce schema --resultats` (résultats), ou `schemaEntree()` et `schemaResultats()` dans le module WebAssembly, dont les types TypeScript `Projet` et `Resultats` sont tirés. Les types sont dans `crates/thbce-core/src/entree/` (un fichier par famille), `crates/thbce-core/src/pv/entree.rs` et `crates/thbce-core/src/resultats/` (un fichier par famille). Ce document garde la structure, les intentions et les règles de saisie que le schéma n'exprime pas. L'historique des changements du format est dans [`historique.md`](historique.md).

## Principes

1. **Entrée, validation, résultats.** `entree::*` décrit ce que l'utilisateur fournit (JSON natif ; RSEE converti plus tard), sans donnée conventionnelle. `valider` contrôle l'entrée (bornes, références, cohérence, périmètre) et produit un rapport (`erreurs`, `avertissements`, chaque problème avec son chemin) ; le calcul résout ensuite les références et joint les données conventionnelles (météo, scénarios, tables). `resultats::*` porte les indicateurs et le détail par niveau.
2. **Paramètres intrinsèques et paramètres d'intégration** (A3 §1.2) : un catalogue de composants (`catalogue.parois`, `catalogue.baies`, `catalogue.masques`) est référencé par identifiant depuis les instances (parois, baies), qui portent surface, exposition et masques.
3. **Noms** : ceux du texte, en `snake_case`, suffixés par l'unité (`u_k_w_m2k`, `a_k_m2`, `alt_m`) ; nombres d'objets en `nb_*` ; statuts de justification `certifie`, `justifie`, `declare`, `defaut`. Angles saisis en degrés.
4. **Énumérations fermées** pour tout code numérique du texte (usage, inclinaison, type d'énergie, poste) ; le code du texte est rappelé dans la documentation de chaque valeur. Un élément reconnu mais hors du périmètre de la version est refusé par la validation (`hors_perimetre`, code de retour 3).
5. **Valeurs par défaut** : seulement quand le texte de la méthode en donne une, et toujours signalée dans les résultats (`valeurs_par_defaut`). Sinon le champ est obligatoire.
6. **Racine** : `$schema` (facultatif, ignoré) et `version_format` (facultatif ; 1 pour ce format).

## Hiérarchie spatiale

La méthode décrit un bâtiment sur quatre niveaux (A3 §5.1.1), plus un niveau projet pour les données communes.

```
Projet { $schema?, version_format?, nom, localisation, catalogue }
├── localisation      département, altitude (0 à 4 000 m), distances à la mer et à l'îlot urbain, zone climatique?
├── catalogue         types de parois et de baies, masques
└── batiments[]
    ├── espaces_tampons[]                      §5.2 à 5.4
    ├── generations[], distributions_intergroupes[], distributions_ecs_intergroupes[]
    ├── installations_pv[]                     ch. 12
    ├── ascenseurs[], parkings[]               ch. 10
    ├── exigences?                             annexe de l'article R. 172-4
    └── zones[]       usage, nb_logements, aéraulique de zone, ventilation
        └── groupes[] surfaces, volume, inertie, parois opaques, baies, ponts thermiques,
                      bouches et entrées d'air, iclim, programmation, émetteurs, distributions,
                      émetteurs d'ECS, locaux d'éclairage (hors habitation)
```

Usages calculés : maison individuelle ou accolée (`maison_individuelle`), logement collectif (`logement_collectif`), bureaux (`bureaux`, lot 0a), enseignement et petite enfance (lot 0c1), hôtels et santé (lot 0c2), restaurants et restaurants scolaires (usages 13 à 16, 26 et 27, lot 0c3), commerces, vestiaires, aérogares, industries et établissements sportifs (usages 17, 18, 22 à 25 et 28, lot 0c4) : les 28 usages sont calculés. Ce qui diffère hors habitation est décrit plus bas (« Usages non résidentiels »).

## Enveloppe

- **Parois opaques** : type (`u_k_w_m2k`, `incl_k`, facteurs solaires `sf_c_k` et `sf_e_k`, végétalisation facultative) et instance (`type_paroi`, `a_k_m2`, `exposition`, `espace_tampon?`).
- **Exposition** (A3 §3.2.2) : `azimut_deg` (0 sud, 90 ouest, 180 nord, 270 est), `inclinaison_deg` (0 horizontale vers le haut, 90 verticale), `lpb_m` et `hpb_m` (requis avec un masque proche), `masques` (0 à 7 identifiants du catalogue).
- **Masques** : `lointain_azimutal` et `lointain_arbres_caducs` (36 hauteurs `gamma_i_deg`), `lointain_plan_vertical`, `proche_vertical_droit`, `proche_vertical_gauche`, `proche_horizontal`.
- **Baies** : type (`u_sp_vert_w_m2k`/`u_sp_horiz_w_m2k`, `u_ap_…`, facteurs `sans_protection` et `avec_protection`, `protection_mobile?`, `ouvrant?`) et instance (`a_b_m2`, `exposition`, `expo_br?`, `bruit?`, `affectation_eclairage?`, `espace_tampon?`). `affectation_eclairage` (lot 0c4, Q81) : `volume_normal` (défaut), `grand_volume` (baie d'un local de grand volume : salle de sport, aire de production, aire de vente de plus de 300 m² ; type de baie 0) ou `grand_volume_toiture_uniforme` (éclairant de toiture horizontal réparti de façon uniforme ; type 1) ; exige un local de grand volume dans le groupe, et `ratio_ecl_nat` = 1 pour ce local avec des éclairants uniformes. Classement au bruit (fiche 5.11, Q200 à Q204) : `expo_br` (BR1 à BR3) se saisit directement, ou se déduit de `bruit` = { `infrastructures?` [{ `categorie` (`categorie_1` à `categorie_5`), `distance_m`, `vue` (`directe`, `partielle`, `masquee_peu_protecteurs`, `masquee_tres_protecteurs`, `arriere`) }], `zone_peb?` (`a`, `b`, `c`, `d`, `hors_zone`), `facade?` } ; la catégorie des voies (arrêté préfectoral de classement) et le plan d'exposition au bruit sont des données à saisir, le moteur ne peut pas les déterminer ; si les deux sont saisis, ils doivent être cohérents. Protections mobiles (lot 1-env, Q100 à Q104) : `protection_mobile` = { `type_pm` (`volet`, `store_enroulable`, `store_venitien`), `gestion?` (volet et store enroulable : `manuelle_non_motorisee`, `manuelle_motorisee`, `automatique_avec_derogation`), `gestion_sv?` (store vénitien : `suivi_soleil_avec_derogation`, `manuelle_position_fixe`, `automatique_45_avec_derogation`), `detecteur_presence?`, `automatisme?`, `exterieure`, `seconde_protection`, `typo_pm?` } ; exactement un des deux champs de gestion selon le type ; `automatisme` en gestion automatique = { `eclairement_limite_lux`, `temperature_limite_haute_c?` et `temperature_limite_basse_c?` (volet et store enroulable, requises), `vent_limite_m_s?` (store enroulable ou vénitien extérieur, requis), `horloge` (`aucune`, `crepusculaire`, `personnalisable`), `horaires_jour?` (trois saisons, requis si personnalisable), `matrice?` ou `matrice_sv?` (absentes : matrices conventionnelles des figures 44 à 47, signalées dans `valeurs_par_defaut`) } ; `store_venitien?` sur le type de baie (requis pour `type_pm = store_venitien`, qui interdit `avec_protection` et exige une baie verticale) = { `th_bc` = { `direct` (6 hauteurs `γ'`), `diffus` et `reflechi` (6 en suivi du soleil, 1 sinon) }, `th_d` = { `direct`, `diffus`, `reflechi` } } de facteurs { `sw1`, `sw2`, `sw3`, `tl_global`, `tlid?` }.
- **Ponts thermiques** : `l_l_m`, `psi_l_w_mk`, facteurs solaires, `exposition?` (description simplifiée du §5.18.3 sans elle).
- **Inertie** : classes des tableaux 53 et 54 ou valeurs saisies (`personnalisee`), pour les inerties quotidienne, séquentielle et annuelle.

## Systèmes

### Format d'entrée des systèmes de chauffage et de refroidissement (lot 6b)

Implémenté au lot 6b (`crates/thbce-core/src/entree/emission_distribution.rs` et `entree/generation/`, lectures `docs/questions/Q043.md`). Les liens sont des identifiants, vérifiés par la validation :

```
Batiment
├── generations[]                 { id, raccordement_reseaux: avec_isolement | permanent,       §8.4.2
│                                   position, priorite?, gestion_temperature_chauffage?,        §8.17.2
│                                   gestion_temperature_refroidissement?, generateurs[],
│                                   stockages[]?, chauffe_eau_thermodynamiques[]?,
│                                   chauffe_eau_solaires[]?,
│                                   systemes_solaires_combines[]? } §9.14 à §9.20, §9.22, §9.23
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
    ├── distributions[]           { id, fonction, distribution_intergroupe, reseau }            §8.7, §8.8
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
                                             u_w_mk, rechauffeur, p_circ_w, espace_tampon?,
                                             arret_circulateurs_vacances? } | trace { … } }
Groupe.emetteurs_ecs[]                     { id, rat_em, nb_logements | nu?, part_melangeur, part_mitigeur,
                                             part_temporisateur, appareils, recuperateur_eaux_grises?,
                                             distribution { theta_2nd_e_c?, d_int_m, l_vc_m?, l_hvc_m,
                                             nb_dist, espace_tampon? }, distribution_intergroupe }
```

Règles de saisie (validation) : au moins un émetteur de chauffage par groupe d'habitation ; `iclim` vrai si et seulement si un émetteur refroidit ; programmation du refroidissement requise si `iclim` ; somme des `Rats × Ratt` au plus 1 par fonction (§8.1.3.2) ; un émetteur est rattaché, pour chaque fonction, à un réseau du groupe de même fonction, lui-même rattaché à un réseau intergroupes de même fonction et de même type (éq. 850 et 851), lui-même relié à une génération ; variation temporelle saisie au moins égale à 0,2 K (effet Joule) ou 0,4 K en chaud, au plus −0,4 K en froid (§8.1.3.1.5) ; `Pper < 1`. Les réseaux sont `fictif` (effet Joule, poêles, détente directe) ou hydrauliques : dans le groupe, `hydraulique { regulation_debit, regulation_temperature, theta_dep_dim_c?, theta_ret_dim_c?, delta_theta_em_dim_k, q_nom_m3_h, q_resid_m3_h?, circulateur, p_aux_w, l_vc_m, l_hvc_m, u_vc_w_mk, u_hvc_w_mk, espace_tampon? }` (§8.7.2, §8.8.2) ; entre groupes, `hydraulique_collectif` ou `hydraulique_individuel { l_vc_m, l_hvc_m, u_vc_w_mk, u_hvc_w_mk, p_aux_w, circulateur, espace_tampon? }` (§8.9.2, §8.10.2). 

### Format d'entrée des générations (lot 6c)

Changement incompatible : une génération décrit ses générateurs (`docs/questions/Q046.md`).

- `position` (« idpos », §8.17.2), obligatoire : `en_volume_chauffe` ou `hors_volume_chauffe { espace_tampon? }` (sans espace tampon : à l'extérieur).
- `priorite` (« idtype_priorite ») : `sans_priorite` ou `cascade`, requis dès que deux générateurs assurent le même poste ; `alternance` est reconnu et refusé. En cascade, chaque générateur du poste porte un rang (`priorite_chauffage`, `priorite_refroidissement`, `priorite_ecs`, à partir de 1, tous distincts).
- `gestion_temperature_chauffage` et `gestion_temperature_refroidissement` (« idgestion », « θwm ») : `constante { theta_wm_c }` ou `reseaux`, requis si la génération alimente des réseaux hydrauliques du poste.
- `generateurs`, au moins un : `id`, `rdim` (nombre de générateurs identiques, au moins 1) et `nature`. Nature prise en charge : `effet_joule { fonction: chauffage | ecs, pn_gen_w }` (type 500, §8.18 ; production instantanée d'ECS, §9.1). Les natures `chaudiere`, `poele_ou_insert` et `reseau_de_chaleur_ou_de_froid` (lot 6d) sont décrites plus bas ; `autre_generateur_a_combustion`, `thermodynamique_gaz`, `boucle_d_eau_ou_drv`, `cogeneration` et `pac_hybride` (hors version 0.1) sont reconnues et refusées avec un message explicite.
- Nature `chaudiere` (types 100 à 102, 200, 201 et 400, §8.19 ; `docs/questions/Q049.md`) : `type_chaudiere` (`gaz_standard`, `gaz_basse_temperature`, `gaz_condensation`, `fioul_standard`, `fioul_condensation`, `bois`), `combustible_bois` (`buches`, `granules`, `plaquettes` ; requis pour le bois), `gpl` (faux : gaz naturel), `fonction` (`chauffage`, `ecs`, `chauffage_ecs`), `pn_gen_kw`, `pertes_parois` (`sans_ventilateur`, `avec_ventilateur`, `avec_clapet`, tableau 121), `rendements` facultatif (`statut` : `certifie`, `justifie` ou `declare`, réductions de 0, 10 et 20 % ; `r_pn_pct`, `r_pint_pct` sur PCS ; `p_int_kw` facultatif, 30 % de `Pn_gen` par défaut), `auxiliaires` facultatif (`w_aux_nom_w`, `w_aux_int_w`, `w_veille_w`), `q_po_30_w` facultatif, `alimentation_bois` (tableau 119 ; requis pour le bois dont les auxiliaires sont par défaut). Toute valeur absente prend sa valeur par défaut (éq. 1072 à 1077), signalée dans `generateurs[].valeurs_par_defaut`. La chaudière est en volume chauffé ou non selon `position` de sa génération. Elle alimente des réseaux hydrauliques et peut être le générateur de base d'un `stockages` (ballon chauffé par chaudière ; `h_rel_ech` non requis).
- Nature `poele_ou_insert` (types 403 et 404, §8.22) : `pn_gen_kw` (de 0 exclu à 100), `eta_pct` (rendement moyen), `p_aux_vent_w`. Chauffage seul, réseaux fictifs (air), énergie bois.
- Nature `reseau_de_chaleur_ou_de_froid` (types 600 et 601, §8.28) : `chaleur { identifiant_reseau?, fonction, type_reseau, classe_isolation_secondaire (1 à 4), p_ess_kw }` ou `froid { identifiant_reseau?, p_ess_kw }`. `type_reseau` : `eau_chaude_basse_temperature`, `eau_chaude_haute_temperature`, `vapeur_basse_pression`, `vapeur_haute_pression` (tableaux 247 et 248). `identifiant_reseau` (quatre chiffres et « C » ou « F ») est contrôlé contre la table de l'annexe 7 de l'arrêté du 15 septembre 2006 (`thbce_data::reseaux_chaleur`, extraite par `cargo run -p xtask -- reseaux-chaleur`), qui donne le facteur d'émission (5e colonne) et le taux d'EnR&R (6e colonne) ; absent, les lignes « autres réseaux » s'appliquent.
- Nature `thermodynamique_electrique` (type 503, §8.23 ; `docs/questions/Q047.md`) : `compresseur` (`puissance_variable` ou `tout_ou_rien`), `taux_auxiliaires` facultatif (`valeur`, `statut` : `certifie` ou `justifie` ; absent : 0,02, ou 0,01 pour un refroidisseur seul, signalé), `source_amont` (`air_exterieur { p_vent_gaine_w? }`, `air_ambiant { espace_tampon, p_vent_gaine_w? }`, `air_extrait { zone, t_air_lim_c, p_vent_gaine_w? }`), et un ou deux modes, `chauffage` et `refroidissement` : `technologie` (`air_exterieur_eau` ou `air_exterieur_air_recycle` ; les autres technologies du tableau 145 sont reconnues et refusées), `performances`, `limites` facultatives (`combinaison` : `l_une_ou_l_autre` ou `l_une_et_l_autre`, `theta_aval_c`, `theta_amont_c`), `typo_emetteur` (`plancher_ou_plafond_inertie_forte`, `radiateurs_ou_plafond_inertie_moyenne`, `ventilo_convecteurs_ou_inertie_faible`, `systemes_a_air`), `fonctionnement_continu` facultatif pour un compresseur à puissance variable (`lr_contmin`, `ccp_lr_contmin`, `statut` ; absent : 0,4 et 1, signalés). `performances` est l'un de `mesurees` (liste de points `theta_aval_c`, `theta_amont_c`, `performance` — COP ou EER —, `pabs_kw`, `statut`, couvrant le croisement de températures prises dans l'ordre de priorité de la technologie, valeur pivot comprise), `pivot_declare { performance, pabs_kw }` ou `pivot_defaut { pabs_kw }`.
- Mode `ecs` d'un générateur thermodynamique (§8.23.3.4) : `technologie` (`air_exterieur_eau`, `air_extrait_eau`, `air_ambiant_eau` ; les autres sont refusées), `performances`, `limites` facultatives. Un tel générateur est le générateur de base d'un assemblage de `stockages` ; avec le mode `chauffage`, il est double service.
- `stockages[]` (§9.9, §9.10 et §9.14 ; `docs/questions/Q048.md`) : `id`, `nb_assemblages` (égal au `rdim` du générateur de base), `ballon` (`v_tot_l`, `pertes` : `certifie { ua_s_w_k }`, `justifie { ua_s_w_k }` ou `defaut { nature }` avec `effet_joule_horizontal`, `effet_joule_vertical` ou `autre`, tableau 282 ; `theta_max_c`), `base` (`generateur`, `programmation` : `permanent`, `nuit` ou `jour`, `z_reg` de 1 à 4, `delta_theta_k` facultatif — 2 K par défaut —, `h_rel_ech` requis pour un générateur thermodynamique) et `appoint` facultatif (`generateur` à effet Joule de fonction `ecs`, `programmation`, `z_ap`, `z_reg`, `f_aux` facultatif — 0,5 par défaut —, `delta_theta_k` facultatif). La consigne est conventionnellement de 55 °C. Un ballon électrique se décrit par un générateur `effet_joule` de fonction `ecs` et un stockage dont il est la base.
- `chauffe_eau_thermodynamiques[]` (§9.22) : `id`, `nb_assemblages`, `v_tot_l`, `theta_max_c`, `essai` (`cycle` : `s`, `m`, `l`, `xl` ou `xxl` ; `t_h_h`, `p_es_w`, `cop_dhw`, `theta_wh_c`, `auxiliaires_compris`, `statut`), `source_amont` (air extérieur, air ambiant d'un espace tampon ou air extrait d'une zone), `programmation`, `limites` facultatives. Le moteur identifie le coefficient de pertes du ballon et les valeurs pivot du COP et de la puissance absorbée de la pompe à chaleur, restitués dans `stockages[].idcet` avec l'écart `epsilon`. Une génération compte au moins un générateur ou un chauffe-eau thermodynamique.
- `chauffe_eau_solaires[]` (lot 7b, §9.14, §9.15 et §9.23) : `id`, `nb_assemblages`, `exposition_capteurs` (exposition de la fiche 3.2 : `azimut_deg`, `inclinaison_deg` de 0 à 90°, `masques` du catalogue), `description` et `appoint`. `description` est soit `composants` — `capteurs` (`surface_m2`, `performances` : `certifie` {`eta0`, `a1_w_m2k`, `a2_w_m2k2`} ou `defaut` {`type_capteur` : `non_vitre`, `vitre` ou `tubulaire`, tableau 293}, `k_theta` requis), `boucle` (`ua_te_w_k`, `ua_ti_w_k`, `p_np_w`, `transfert` : `echangeur` {`ua_hx_w_k` facultatif, 100·A par défaut} ou `sans_echangeur`, `regulation` : `temperature` ou `irradiance` {`isr_min_w_m2` facultatif, 200 W/m²}, `alpha_p` (0,5), `m_cp_w_k` (58·A) et `h_rel_ech` facultatifs) et `ballon` (comme pour `stockages[]`) —, soit `kit` (NF EN 12976, éq. 2263 à 2273 : `surface_m2`, `v_tot_l`, `p_np_w`, `ac_etoile_m2`, `uc_etoile_w_m2k`, `us_w_k`, `theta_max_c`, `f_aux` et `k_theta` requis, Q52 ; appoint `integre` seulement, §9.23.3.2.2). `appoint` : `aucun`, `integre` (comme l'appoint d'un stockage : générateur à effet Joule de fonction `ecs`, `programmation`, `z_ap`, `z_reg`, `f_aux`, `delta_theta_k`) ou `separe` (la génération doit compter un générateur instantané d'ECS). Une génération peut ne compter que des chauffe-eau solaires. Résultats : `stockages[].solaire` (`e_sol_kwh` et `e_sol_mensuel_kwh`, énergie solaire transmise aux ballons ; `w_pompe_kwh`, consommation des pompes, comprise dans l'électricité d'ECS ; `pertes_vc_kwh` ; `isr_kwh_m2`, ensoleillement du plan des capteurs ; `a_capteurs_m2` ; `nbh_marche` ; `ecretement_kwh`, énergie perdue par la limitation à `θmax`).
- `systemes_solaires_combines[]` (lot 7b, fiches 9.18 à 9.20) : `id`, `exposition_capteurs`, `capteurs`, `boucle` et `ballon` (comme pour un chauffe-eau solaire décrit par composants), `appoint` (`generateur`, `programmation`, `f_aux` et `delta_theta_k` facultatifs ; zone 3, éq. 2130) et `raccordement` : `independant` {`delta_theta_reg_boucle_chauffage_k`} (appoint d'ECS à effet Joule ; le chauffage restant va aux autres générateurs de la génération) ou `raccorde` (l'appoint, qui assure le chauffage et l'ECS, est rappelé pour le chauffage restant). La génération doit alimenter un réseau de chauffage. Résultats : `stockages[].solaire` et `stockages[].ssc` (`q_puise_ch_kwh`, chaleur de chauffage puisée dans le ballon ; `q_fou_ch_appoint_kwh`).
- Cohérence du montage (§8.17.3.1) : la génération assure chaque poste de ses réseaux ; ses réseaux intergroupes d'un même poste sont tous fictifs ou tous hydrauliques ; un générateur thermodynamique sur air recyclé alimente des réseaux fictifs, un générateur sur eau des réseaux hydrauliques ; de même une chaudière ou une sous-station de réseau de chaleur alimente des réseaux hydrauliques et un poêle des réseaux fictifs.
- Double flux à sécurité antigel : `echangeur.generation_antigel` désigne la génération qui fournit l'énergie de l'antigel (requis avec `antigel`).

Résultats : les consommations des générateurs à combustion sont restituées en kWh d'énergie finale PCI dans la colonne de leur énergie (`gaz_kwh`, `fioul_kwh`, `bois_kwh`, `reseau_kwh`), leurs auxiliaires en `electricite_kwh` du poste servi ; `eta_eff_*_an` est alors le rendement saisonnier (énergie fournie divisée par le total du poste, auxiliaires compris). `generations[]` porte les demandes (`q_ch_kwh`, `q_fr_kwh`, `q_ecs_kwh`), les énergies fournies (`q_fou_*_kwh`), `q_rep_fin_kwh`, `consommations`, `pertes_vc_kwh`, les compteurs de sous-dimensionnement et `generateurs[]` (énergies fournies, `consommations`, efficacités annuelles `eta_eff_*_an` — COP et EER saisonniers des générateurs thermodynamiques —, `w_aux_kwh`, `phi_rejet_kwh`, `valeurs_par_defaut`). `consommations` (`annuel` et `mensuel`, douze mois des scénarios) donne, par poste (`chauffage`, `refroidissement`, `ecs`) et par type d'énergie (`gaz_kwh`, `fioul_kwh`, `charbon_kwh`, `bois_kwh`, `electricite_kwh`, `reseau_kwh` ; les énergies sans consommation ne sont pas écrites), les kWh d'énergie finale ; il figure sur chaque génération et, réparti au prorata des demandes (éq. 1484 à 1486), sur chaque groupe, chaque zone et le bâtiment : c'est la matrice « Qcef[poste ; énergie] » des sorties du mode Th-C (§13.2), auxiliaires de génération compris. Une génération porte `stockages[]` : énergie fournie à la distribution (`q_fou_ecs_kwh`), énergies injectées par la base et par l'appoint, `pertes_kwh`, `v_puise_l`, `theta_haut_min_c`, `nbh_temp_insuff_max`, `nbh_report_e` (heures de l'année où une demande n'a pas pu être entièrement puisée, éq. 1780 ; alerte au-delà de 24 h), `idcet` et `valeurs_par_defaut`. La ventilation d'une zone porte `th_c` (heures de bypass et énergie d'antigel du calcul Th-C).

Les liens entre objets (un émetteur vers sa distribution, une distribution vers sa génération) sont des identifiants, vérifiés par la validation.

### Format d'entrée des postes du logement collectif et des parkings (lot 6e)

Facultatif ; l'éclairage des parties communes (§7.2) ne demande aucune saisie (toute zone de logement collectif). Lectures : `docs/questions/Q050.md`.

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

Résultats : `consommations_collectif` (zone et bâtiment) : `eclairage_parties_communes`, `ascenseurs`, `ventilation_parkings`, `eclairage_parkings`, chacun `{ annuel_kwh, annuel_kwh_m2, mensuel_kwh[12] }` ; `ascenseurs[]` (`nb_demarrages`, `t_mob_h`, `pti_w`, `e_tm_kwh`, `e_ti_kwh`, `e_tot_kwh`, `valeurs_par_defaut`) et `parkings[]` (`e_vent_kwh`, `e_ecl_kwh`, `valeurs_par_defaut`) au niveau du bâtiment.

### Format d'entrée de la production photovoltaïque (lot 7a)

Champ facultatif (`docs/questions/Q051.md`) : un bâtiment porte `installations_pv[]`, une par ensemble de modules connectés à un même onduleur (§12.1, §12.2.1).

- `id` ; `exposition` (`azimut_deg`, `inclinaison_deg` de 0 à 90, `lpb_m`, `hpb_m`, `masques` : identifiants du catalogue, §3.2) ; `nb_modules` (« N ») ; `confinement` (« CT » : `libre` = 1, `autre` = 1,5, `confine` = 2, valeur par défaut).
- `modules` : `technologie` (`mono_cristallin_si`, `multi_cristallin_si`, `cdte`, `cis`, `amorphe_si`, `autre`), `s_m2` (surface d'un module), `performances` selon `statut` : `certifie`, `justifie` ou `declare` (`pc_w`, `mu_par_c`, `noct_c`), ou `defaut` (`pc_w` seul ; μ et NOCT par défaut signalés). Les valeurs de calcul sont réduites selon le statut (§12.3.3.1).
- `onduleur` facultatif : `p_ac_nom_w` (défaut : 0,8 × N × Pc, signalé) et `rendement` : `defaut` (0,9), `europeen { eta_eu }` ou `courbe { points: [{ x, eta }] }` (points 0,05 ; 0,1 ; 0,25 ; 0,5 ; 0,75 ; 1 au minimum).

Résultats : `batiments[].production_pv` (`e_prod_pv_kwh`, `e_prod_pv_mensuel_kwh`, `installations[]` : puissance crête de calcul, `eta_stc`, `u1_w_m2k`, `p_ac_nom_w`, rayonnement efficace `g_kwh_m2`, `e_pv_kwh` en sortie des modules, `e_ond_kwh` en sortie de l'onduleur, mensuel, `nbh_extinction`, `valeurs_par_defaut`). Série horaire (API Rust) : `thbce_core::series_pv` (Wh par pas, par installation et pour le bâtiment) ; l'autoconsommation est restituée dans `indicateurs.production_pv`.

## Données conventionnelles

### Météo : jamais embarquée

Les fichiers météo réglementaires ne peuvent être ni versionnés ni diffusés (question Q1). Le moteur ne contient donc aucune donnée météo : l'appelant la fournit.

Les lecteurs vivent dans `thbce-io`, derrière des fonctions qui prennent des octets et non un chemin, pour servir aussi le WASM :

| Lecteur | Source | Usage |
|---|---|---|
| `meteo::lire_csv_re2020(&[u8])` | CSV long `zone, mode, h, te0, …, Psi` | usage courant, tests locaux (`THBCE_METEO`) |
| `meteo::lire_classeur_re2020(&[u8])` | classeur officiel, 16 onglets `<zone>_<mode>` | même contenu, format d'origine |
| `meteo::lire_csv_ademe(&[u8])` | fichiers ouverts ADEME / Météo-France | CI publique, jamais réglementaire (question Q16) |

Le calendrier (jour de semaine, semaine du mois, heure légale) n'est pas lu de la météo : il est calculé d'après A3 §4.1.3 (p. 56).

### Scénarios

`cargo run -p xtask -- scenarios` lit `docs/reference/scenarios/2026-04-29_scenarios_conventionnels.xlsx` et écrit la table versionnée `crates/thbce-data/src/scenarios/table.rs` ; `xtask verifier` (CI) contrôle qu'elle est à jour.

### Constantes réglementaires

| Donnée | Source | Forme |
|---|---|---|
| Coefficients d'énergie primaire | arrêté, article 9 ; A3 tableau 333 | `data/src/energies.rs`, par `Energie` (lot 7) |
| Facteurs d'émission des énergies | arrêté, article 10 ; fiches INIES « [RE2020] » en recoupement | `data/src/energies.rs`, par énergie et par usage (lot 7) |
| Réseaux de chaleur et de froid | annexe 7 de l'arrêté du 15 septembre 2006 (colonnes 5 et 6) | table générée par script, clé = identifiant du réseau |
| Pondération dynamique `fCO2(a)` | arrêté, article 11 (JO du 15 août 2021, p. 59) | `data/src/ponderation_dynamique.rs` (Q5) |
| Température de base par département | A3 §3.1.3 (p. 31) | table par `Departement` |

## Résultats

Structure générale (détail : `thbce schema --resultats`) :

```
Resultats { version_moteur, version_methode, reglementaire, origine_meteo_th_bc, origine_meteo_th_d,
            valeurs_arrondies, avertissements[], climat }
└── batiments[]   sref_m2, bbio_pts, dh_c_h, cep_kwhep_m2, cep_nr_kwhep_m2, ic_energie_kgco2_m2,
                  bbio, consommations, consommations_collectif, generations[], ascenseurs[], parkings[],
                  production_pv, espaces_tampons[], indicateurs (exigences, conformité)
    └── zones[]   nadeq, dh_c_h, bbio, ventilation, consommations, …
        └── groupes[]  enveloppe, debits_bbio, baies[], bbio, saisons, confort, ventilation,
                       systemes, consommations, valeurs_par_defaut
```

`reglementaire` est faux dès qu'un jeu météo n'est pas celui de la RE2020. Les valeurs sont arrondies à une décimale (§1.3.1, Q8) sauf `--sans-arrondi` (`arrondir: false`).

## Erreurs

Rapport de validation : `erreurs` et `avertissements`, chacun `{ chemin, valeur?, message, hors_perimetre? }`. Natures d'erreur (module WebAssembly, propriété `cause.nature`) et codes de retour de la ligne de commande : `lecture` (JSON illisible, code 2), `saisie` (entrée invalide, 2), `meteo` (données météo, 2), `non_pris_en_charge` (entrée correcte mais hors du périmètre, 3), `calcul` (calcul impossible sur une entrée valide, 3). Aucune entrée ne provoque de `panic!`.

## Déroulement du calcul

Conforme à l'enchaînement du §5.1.3 de l'annexe III : `calculer` valide l'entrée, calcule le climat des deux jeux météo, puis enchaîne pour chaque bâtiment trois passes annuelles au pas horaire du même enchaînement paramétré par `ModeCalcul` : Th-B (Bbio, systèmes conventionnels), Th-D (DH, météo à séquence caniculaire, systèmes du projet) et Th-C (consommations, systèmes du projet, saisons effectives, relances, générations). Chaque passe porte sur le bâtiment entier (`bati::assemblage_batiment::BatimentHoraire`) : à chaque pas de temps, les espaces tampons, à partir de l'état des groupes au pas précédent, puis les zones. Les indicateurs (Cep, Cep,nr, Ic énergie, exigences) sont calculés après la passe Th-C.

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

## Compléments par lot

**Lot 1-ecs (18 novembre 2026 ; fiches 9.11 et 9.12, Q106).** `stockages[].echangeur` facultatif : un ballon qui en porte un est un accumulateur en eau technique (consigne 55 °C pour `application` `individuel`, 60 °C pour `collectif`). Champs : `application`, `p_ech_ecs_kw` et `nature` : `interne` ou `externe` {`q_v_prim_nom_m3_h`, `circulateur_vitesse_variable` (vrai par défaut), `p_circ_prim_nom_w`, `mod_min` facultatif}. `appoint.f_aux` y est interdit ; une boucle d'ECS en aval est refusée (code 3). Résultats : `stockages[].echangeur` facultatif (`ua_ech_w_k`, `theta_cons_c`, `w_circ_prim_kwh`, compté dans l'électricité d'ECS). Valeurs par défaut signalées : `echangeur.application`, `echangeur.externe.mod_min`.

**Lot 1-ecs2 (18 novembre 2026 ; fiche 9.21, Q108).** `emetteurs_ecs[].recuperateur` facultatif (avec `recuperateur_eaux_grises` = `Irecup`) : `configuration` (`individuelle` ou `collective`), `position` (`amont_mitigeurs_et_ballon`, `amont_ballon`, `amont_mitigeurs` ; `amont_ballon` imposée en collectif), `unites[]` (`nb_echangeurs`, `nb_douches_relies`, `nb_douches_non_relies`, `echangeur` {`origine` `certifiee`, `justifiee` ou `declaree`, `efficacite`, `c_trans`}, `p_relevage_nominale_w`), `l_evac_vc_m` et `l_evac_vnc_m`, `recuperateur_collectif` (identifiant). `batiments[].recuperateurs_eaux_grises[]` facultatif : récupérateurs collectifs (`id`, `echangeur`, longueurs d'évacuation). Efficacité absente : `0,8 × 0,15`, signalée dans `valeurs_par_defaut`. Sorties : `recuperateur` de chaque émetteur (`q_w_sans_recuperateur_kwh`, `w_relevage_kwh`, `theta_entrant_moyen_c`) et `recuperateurs_eaux_grises[]` du bâtiment (`epsilon_ech_eg`, `valeurs_par_defaut`). Format compatible : champs facultatifs.

**Relecture du lot 7b (17 novembre 2026).** `chauffe_eau_solaires[].description.kit.k_theta` devient **requis** (changement incompatible ; sortie de l'outil IdCESI, Q52 point 1) ; un kit n'admet que l'appoint `integre`. Un système solaire combiné demande un réseau hydraulique de chauffage du groupe, relié à sa génération, dont `theta_dep_dim_c` est saisie (« θch_max^gen », éq. 2117). Résultats : `generations[].stockages[]` porte `v_zones_l`, `theta_zones_debut_c` et `theta_zones_fin_c` (volumes et températures des quatre zones d'un ballon, de bas en haut, au début et à la fin du calcul), qui ferment le bilan des ballons.

**Lot 7c (fiches 9.16 et 9.17).** `generations[].chauffe_eau_solaires_collectifs[]` (au plus un par génération, seul assemblage avec ballon de sa génération) : `id`, `type_pcad` (`cesci`, `cescai` ou `cesci_appoint_separe`), `exposition_capteurs` (inclinaison de 0 à 90°), `capteurs` et `boucle` (comme pour un chauffe-eau solaire décrit par ses composants, pour tout le champ ; régulation `irradiance` requise pour un CESCI, éq. 1981 ; `h_rel_ech` refusé), `ballon_central` (CESCAI seulement, comme `ballon`) et `ensembles[]` (au moins un : `id`, `nb_ballons`, `ballon` individuel, `appoint` intégré comme celui d'un stockage — requis pour `cesci` et `cescai`, sans objet pour `cesci_appoint_separe` —, générateur d'effet Joule de fonction `ecs` ou chaudière assurant l'ECS, dont `rdim` vaut `nb_ballons`). Réseau d'ECS intergroupe de la génération : `aucun` pour un CESCI, `boucle` sans réchauffeur pour un CESCAI. Un CESCI à appoint séparé demande un générateur instantané d'ECS dans la génération. Résultats : `generations[].solaires_collectifs[]` (`id`, `type_pcad`, `solaire` comme pour un chauffe-eau solaire, `ensembles[]` comme `stockages[]` pour tous les ballons d'un ensemble, `ballon_central`, `q_fou_reseau_kwh`, `pertes_reseau_deduites_kwh`). Lectures : Q53.

## Usages non résidentiels (lot 0a)

Seul l'usage 3 (bureaux) est ouvert ; le socle (scénarios, ECS, éclairage, ventilation simple flux, protections mobiles) vaut pour les 28 usages. Ce qui change dans l'entrée par rapport à l'habitation :

```
Zone.usage                       "bureaux" ; nb_logements et trav_zone sans objet (tableau 22)
Zone.ventilation.systeme         simple_flux_extraction | simple_flux_insufflation
                                 { p_vent_occ_w, p_vent_inocc_w }            éq. 636 à 639
                               | double_flux { p_vent_rep_occ_w, p_vent_sou_occ_w,
                                 p_vent_rep_inocc_w, p_vent_sou_inocc_w, echangeur?,
                                 batterie_prechauffage?, batterie_prerefroidissement? }
                                                                             éq. 458, 459 (lot 1-ven)
                               | cta { type_cta, p_vent_{rep,sou}_{ch,zn_occ,zn_inocc}_w,
                                 theta_enc_c, theta_enf_c, regulation_air_neuf?, echangeur?,
                                 batterie_prechauffage?, batterie_prerefroidissement? }
                                                                             §6.3, types 2 à 4
  echangeur                      { representation, rotatif?, p_ech_w?, bypass?, antigel?,
                                   generation_antigel?, theta_s_ech_rep_lim_c? }
  batterie_prechauffage          { theta_cons_c, theta_ex_c, generation }    éq. 522 à 525
  batterie_prerefroidissement    { theta_cons_c, theta_ex_c, theta_dim_fr_c, generation }
                                                                             éq. 539 à 548
Groupe.su_m2                     surface utile = surface de référence (éq. 2376) ; pas de shab_m2
Groupe.bouches_conduits[]        { id, isouf, q_spec_occ_m3_h, q_spec_inocc_m3_h, crdbnr?, cdep?,
                                   a_cond_m2?, q_spec_ch_m3_h? }              éq. 383 à 386, 400 à 402
                                 (q_spec_ch_m3_h : débit du mode actif, bouches d'une cta)
Groupe.locaux[]                  { id, type_local, ratio_local, ratio_ecl_nat,
                                   eclairage { p_ecl_tot_w_m2, p_ecl_aux_w_m2?, gest_ecl, grad_ecl,
                                               fractionne, eff_ecl_immo_projet_w_m2_100lux?,
                                               ei_proj_lux? } }               §7.1
Groupe.emetteurs_ecs[]           nu? au lieu de nb_logements                  §9.6.3.2.2.1
```

- **Ventilation.** En non résidentiel, les débits saisis sont les débits totaux repris ou soufflés, en occupation et en inoccupation au sens de l'indicateur de ventilation des scénarios ; `crdbnr` (tableau 57 : 1, 0,9 ou 0,8) vaut 1 s'il est absent, valeur signalée. Le calcul du Bbio n'emploie pas ces bouches : ses débits sont conventionnels (tableau 56-1, par m² de surface de référence). Au moins une bouche dans la zone. Les champs de l'habitation (`q_spec_conv_pointe_m3_h`, `q_spec_conv_base_m3_h`, `qvarep_spec_m3_h`, `dugd_h`, `p_vent_base_w`, `p_vent_pointe_w`) sont refusés, et inversement. **Double flux et centrales de traitement d'air** (lot 1-ven, Q90 à Q94) : pour une `cta`, chaque bouche porte aussi son débit du mode actif `q_spec_ch_m3_h` (« qsoufflé,CH ») ; `q_spec_occ_m3_h` et `q_spec_inocc_m3_h` sont alors les débits de zone neutre (« qsoufflé,ZN,occ », « qsoufflé,ZN,inocc »), au plus égaux au débit du mode actif, dont les taux d'air neuf minimaux se déduisent ; `crdbnr` vaut 1 en présence de recyclage (Q92). Les centrales à débit constant (`debit_constant`, `debit_constant_temperature_variable`) passent au débit nominal en occupation, en relance ou après un besoin d'un groupe en inoccupation, la centrale à débit variable reste en zone neutre (Q90). Les batteries de préchauffage et de prérefroidissement (double flux et centrale `debit_constant`) désignent la génération qui leur fournit l'énergie, comme la sécurité antigel (réseau de CTA fictif, fiche 8.11). **Mode actif** (lot 1-ven2, Q95) : une centrale de type 3 ou 4 peut porter des batteries régulées sur la charge, `batterie_chaude` (avec `generation_batterie_chaude` et, en type 4, `theta_sou_nom_chaud_c`) et `batterie_froide` (avec `generation_batterie_froide`, `theta_dim_fr_c` et, en type 4, `theta_sou_nom_froid_c`) ; elles assurent la demande des émetteurs des groupes déclarés contenus dans les bouches de soufflage (`emetteurs[].chauffage.centrale`, `emetteurs[].refroidissement.centrale`), qui ne passe plus à leur réseau du groupe ; résultats `nbh_mode_actif`, `nbh_sous_dimensionnement`, `nbh_sous_dimensionnement_consecutives` (plus longue série, avertissement au-delà de 6 heures ; lot 1-ven3), `q_chaud_kwh`, `q_froid_kwh` dans `th_d` et `th_c`. **Humidification et rafraîchissement évaporatif** (lot 1-ven3, Q96, Q97) : `humidificateur` (types 1 et 2 : `type_humidification` `vapeur` ou `pulverisation`, `omega_cons_kg_kg`, `eta_hum` facultatif de 0,8 pour la pulvérisation, `generation` requise pour la vapeur) et `rafraichissement_adiabatique` (`type_rafraichissement` `direct` ou `indirect`, `theta_i_base_c` d'au moins 22 °C, `eta_hum` facultatif de 0,8, `delta_theta_i1_k` et `delta_theta_i2_k` pour une centrale à débit variable) ; les booléens `humidification` et `rafraichissement_evaporatif` restent admis (`true` exige la description) ; résultats `q_hr_kwh`, `w_aux_chaudhr_kwh`, `nbh_adiab`, `w_aux_adiab_kwh` dans `th_d` et `th_c`. Hors de cette version (code 3) : humidification en habitation, surventilation mécanique, puits ; ventilation naturelle, hybride et aération. La forme `centrale_traitement_air` de la version 0.1 est une erreur de saisie (décrire la centrale par `cta`). Résultats : `zones[].ventilation.th_d` et `th_c` portent `q_prechaud_kwh` et `q_prefroid_kwh` (absents sans batterie) ; la consommation des ventilateurs d'une centrale double flux hors habitation est relevée heure par heure (`c_aux_vent_kwh`, `c_aux_vent_mensuel_kwh`), échangeur rotatif compris.
- **Éclairage.** Les locaux du groupe sont requis ; la somme des `ratio_local` vaut 1 ; `type_local` est un des types que le tableau 78 donne à l'usage (bureaux : `bureau_standard`, `salle_reunion`, `circulation_accueil`, `sanitaires_collectifs` ; enseignement primaire : `salle_classe`, `bureau_standard`, `salle_reunion`, `salle_repos`, `circulation_accueil`, `sanitaires_vestiaires` ; secondaire : `salle_classe`, `salle_reunion`, `salle_enseignement_informatique`, `salle_conference`, `bureau_standard`, `centre_documentation`, `salle_professeurs`, `circulation_accueil`, `sanitaires_collectifs` ; médiathèques : `salle_multi_fonctions`, `centre_documentation`, `local_service`, `bureau_standard`, `salle_reunion`, `circulation_accueil`, `sanitaires_collectifs` ; universitaires : ceux du secondaire sans la salle des professeurs, pas de `local_service`, Q70 ; petite enfance : `salle_jeux`, `salle_repos`, `bureau_standard`, `salle_reunion`, `circulation_accueil`, `sanitaires_vestiaires`). `gest_ecl` : `permanent`, `interrupteur_manuel`, `interrupteur_et_horloge`, `marche_et_arret_automatiques`, `marche_manuelle_arret_automatique` ; `grad_ecl` : `aucune`, `manuelle`, `gradation_automatique`, `allumage_et_extinction_automatiques`, `extinction_automatique`. `p_ecl_aux_w_m2` est requis avec une détection de présence ou une gestion automatique selon la lumière naturelle, refusé sinon (nul par convention). `eff_ecl_immo_projet_w_m2_100lux` et `ei_proj_lux` sont requis pour un bureau standard dont la puissance immobilière est inférieure à 10 W/m² (éclairage mobilier, §7.1.3.2.3). Le calcul du Bbio ignore le système saisi (`2 × Eiref/100` W/m², horloge, gestion manuelle). Les locaux de grand volume sont hors périmètre.
- **ECS.** `nu` est le nombre d'unités du tableau 277 (m² de surface utile) ; absent, c'est `rat_em × su_m2`. Un bâtiment de bureaux sans production d'ECS se décrit avec la production conventionnelle de la fiche 9.24 (chauffe-eau électriques de 15 L, un par niveau et par tranche de 750 m² au-delà de 750 m², appareils `douche_seule`).
- **Protections mobiles.** Volets et stores enroulables à gestion manuelle, avec les matrices de la famille de l'usage (figure 34) : « bureau » pour les bureaux et les médiathèques, « enseignement » pour les usages 4, 5, 7 et 12.
- **ECS des zones d'enseignement** (usages 4, 5, 7 ; lot 0c1, Q71). Pendant les vacances (grille annuelle d'occupation nulle), le réseau d'ECS intergroupe qui ne dessert que des zones d'enseignement est arrêté, et sa génération aussi : ni pertes de boucle ni pertes de ballon. `distributions_ecs_intergroupes[].reseau.boucle.arret_circulateurs_vacances` (« Typegest_circ-e ») est requis pour une boucle qui dessert une zone d'enseignement : vrai si les circulateurs sont arrêtés pendant les vacances, faux s'ils tournent toute l'année ; il ne se saisit pas à vrai ailleurs (circulateurs permanents).
- **Ascenseurs et parkings** : réservés au logement collectif dans cette version.
- **Résultats.** `zones[].nadeq` est absent ; `ecs[].nadeq` d'un émetteur porte `Nu` ; l'éclairage entre dans l'Ic énergie au facteur « bâtiment tertiaire » de l'article 10 ; `indicateurs.exigences` suit les exigences de l'usage (lot 0b).

## Indicateurs réglementaires et exigences (lot 7)

### Format d'entrée

Champs facultatifs, **non incompatibles** (`docs/questions/Q054.md`) :

```
Batiment
└── exigences?  { categorie_contraintes_exterieures: categorie_1 | categorie_2 | categorie_3,  chap. V, partie II (saisie, contrôlée)
                  zone_bruit?: br1 | br2 | br3,                                   chap. V, partie I ; Mbbruit ; absente : déduite des baies (Q202)
                  annee_depot_permis (≥ 2022),                                    Icénergie_maxmoyen, dates d'effet (Q68)
                  mois_depot_permis? (1 à 12),                                    obligatoire en 2026, et en 2022 avec un usage 3 à 5 (Q68, Q69)
                  hsp_moy_m? (1,8 à 10),                                          MbHSP, McHSP (habitation, permis ≥ 07/2026)
                  s_combles_m2? (0),                                              Mbcombles, Mccombles (maisons)
                  reseau_chaleur_classe? (false),                                 Icénergie_maxmoyen jusqu'en 2027
                  parcelle_raccordement_gaz? (false),                             Icénergie_maxmoyen = 280 (maisons, ≤ 2023)
                  zones?: [{ zone,                                                lot 0b (Q66)
                             categorie_contraintes_exterieures?,                  Mbbruit (cat. 3), Mccat, DH_max des groupes
                             zone_bruit?,                                         Mbbruit
                             groupes?: [{ groupe, categorie_contraintes_exterieures }] }],   DH_max du groupe
                  extension?: { surelevation? (false),                           lot 0d (Q86) : fiche 4.4, article 50-3
                                sref_existant_m2? (1 à 1e6),                      part de l'existant (obligatoire hors maison)
                                sref_autres_extensions_m2? (0 à 1e6) } }          surface cumulée des extensions (article 50-3, I)
NatureGenerateur::PoeleOuInsert
└── combustible_bois: buches | granules | plaquettes                              facteur d'émission (article 10), obligatoire
```

Le raccordement à un réseau de chaleur urbain se déduit des générations (sous-station de réseau de chaleur). Le GPL (`chaudiere.gpl`), le combustible bois des chaudières (`combustible_bois`) et l'identifiant du réseau de l'annexe 7 (`identifiant_reseau`) servent aux coefficients de conversion. Catégorie héritée groupe → zone → bâtiment, classe de bruit zone → bâtiment (Q66). Un `hsp_moy_m` hors de 1,8 à 10 m est une erreur même quand il est sans effet. La catégorie 3 est refusée en habitation et pour les usages que le chapitre V ne cite pas (enseignement, vestiaires, sanitaires, santé partie nuit, industries, sports) ; la catégorie 2 l'est si un groupe d'habitation n'est pas climatisé, hors des zones H2d et H3, à 400 m d'altitude ou plus, ou en BR1 (chapitre V, partie II ; Q54, point 14). Sans `exigences`, les indicateurs sont calculés mais ni les exigences ni la conformité, et le forfait de refroidissement n'est pas plafonné au seuil haut. `combustible_bois` est obligatoire sur un poêle ou un insert (changement incompatible, relecture du lot 7). `conforme` n'est jamais vrai si un indicateur exigé manque (`indicateurs_manquants`).

### Résultats

`batiments[].cep_kwhep_m2`, `cep_nr_kwhep_m2` et `ic_energie_kgco2_m2` sont calculés dès que le bâtiment a des générations. Le détail est dans `batiments[].indicateurs` :

```
indicateurs
├── sref_m2
├── electricite_consommee, electricite_importee        { chauffage_kwh, refroidissement_kwh, ecs_kwh, eclairage_kwh,
│                                                        auxiliaires_ventilation_kwh, auxiliaires_distribution_kwh,
│                                                        deplacements_kwh, mobilier_kwh }          §13.4, éq. 2535
├── electricite_importee_mensuel_kwh[12]               hors mobilier, hors forfait
├── production_pv?    { e_prod_pv_kwh, e_prod_pv_ac_kwh, e_exportee_kwh, tac_pct, mensuels }   éq. 2527 à 2530
├── forfaits_refroidissement[]  { zone, groupe, dh_c_h, seuil_haut_c_h, cef_kwh_m2, cef_kwh }   §13.3, éq. 2493
├── energie_solaire_captee_kwh? pour information (coefficients nuls, article 9)
├── lignes[]          { poste, energie, cef_kwh, coef_ep, coef_ep_nr, facteur_emission_kg_kwh,
│                       cep_kwhep_m2, cep_nr_kwhep_m2, ic_energie_kgco2_m2 }
├── cef_kwhef_m2, cep_kwhep_m2, cep_nr_kwhep_m2, ic_energie_kgco2_m2, somme_f_co2
└── exigences?        { bbio_max_pts, cep_max_kwhep_m2, cep_nr_max_kwhep_m2, ic_energie_max_kgco2_m2,
                        zones[] (valeurs moyennes et coefficients de modulation de chaque zone),
                        verifications[] { indicateur, groupe?, valeur, maximum, marge, marge_pct, respectee },
                        conforme,
                        regime_alternatif? { nature: construction | extension | surelevation,          lot 0d (Q86 à Q88)
                                             famille: maison_individuelle | collectif_bureaux_enseignement
                                                      | tertiaire_specifique_ou_industriel,
                                             sref_consideree_m2, part_existant_pct?,
                                             regime: complet | intermediaire | exigences_de_moyens | hors_champ | rt2012,
                                             verifications[] (Bbio, DH : régime intermédiaire), conforme?,
                                             indicateurs_manquants[] } }
```

**Extensions et constructions de petite surface (lot 0d, Q86 à Q88).** Une extension (partie nouvelle d'un bâtiment existant, fiche 4.4 : partie accolée créant de nouveaux locaux, étage ajouté, combles surélevés d'au moins 1,8 m ; pas l'aménagement d'un volume existant) se décrit **seule**, comme un bâtiment, avec `exigences.extension`. Ses zones ne contiennent aucun local existant ; une paroi sur un local chauffé existant est mitoyenne et ne se saisit pas ; une paroi sur un local existant non chauffé donne sur un espace tampon. Son usage est celui de ses locaux, à défaut celui du bâtiment existant (annexe I) ; `nb_logements` compte les logements qu'elle contient en tout ou en partie, logements agrandis compris (Q87). Équipements existants qui la desservent : valeurs initiales ou valeurs par défaut de la réglementation de l'existant, puissance nominale au prorata des besoins (à défaut, des surfaces) ; ventilation sans équipement complet : extraction compatible avec les entrées d'air, débits égaux à la somme de leurs modules, 0,25 W/(m³/h) ; perméabilité mesurée ou valeur par défaut du §5.6.3.3.3 (FA03 v2, annexe II ; Q89). Les exigences de l'article R. 172-4 sont calculées sur la `Sref` et les logements de l'extension (Q87) et restent vérifiées (`conforme`) ; `regime_alternatif` dit quel régime ouvre la surface cumulée des extensions du même bâtiment, selon l'usage principal, la part de l'existant, la nature (surélévation) et la date (article 3, II pour 2022 ; article 50-3 ; article R. 172-1, III), avec les vérifications du régime intermédiaire (Bbio et DH, `DH_max` porté à 1 400 °C.h pour une surélévation climatisée de maison en H2d ou H3, catégorie 1 ; Q88). Il est aussi restitué pour une construction neuve de moins de 50 m². Les bornes suivent l'arrêté (« inférieure à » stricte : une extension de maison de 50 m² pile relève du régime complet) ; les exigences de moyens et l'Icconstruction sont hors du moteur.

**Identification de l'usage (fiches 4.2 et 4.3, Q85).** L'usage de chaque zone reste une règle de saisie : bâtiment de maison individuelle, de maisons accolées ou collectif selon la figure 11 (nombre de logements, logements superposés au-delà de 15 m² de paroi horizontale commune, porte d'entrée commune) ; bâtiments accolés (15 m² de parois mitoyennes pour les maisons, 50 m² sinon) calculés comme un bâtiment unique, chaque partie gardant son usage ; une zone par usage ; partie de moins de 150 m² et de 10 % assimilable à l'usage principal (article 2, sauf maison ; Q67) ; destinations non listées selon la FA05 v2. La validation lève des **avertissements** (jamais d'erreur) pour : zones de maison et de logements collectifs dans le même bâtiment ; bâtiment d'habitation d'au plus deux logements décrit en logements collectifs (hors extension) ; zone de maison dans un bâtiment d'usage principal non résidentiel ; vestiaires seuls avec un établissement sportif ; deux zones du même usage (hors logements collectifs).

`energie` vaut `electricite`, `gaz_naturel`, `gpl`, `fioul`, `charbon`, `bois_buches`, `bois_granules`, `bois_plaquettes`, `bois` (combustible inconnu), `reseau_chaleur:<id>` ou `reseau_froid:<id>` (`autres` sans identifiant). La ligne de commande écrit une synthèse des indicateurs et de la conformité sur la sortie d'erreur (`thbce_io::rapport::synthese_indicateurs`).



## Analyse du cycle de vie (lot ACV)

Annexe II de l'arrêté du 4 août 2021, chapitre 4 et §5.3 ; exigence `Icconstruction_max` de l'annexe de l'article R. 172-4, chapitre III, III. Seul l'indicateur de changement climatique est calculé (méthodes statique et dynamique). **Aucune donnée environnementale n'est embarquée** : les valeurs d'impact (FDES, PEP, DED, données configurées) et les données de service sont saisies avec leur identifiant et leur source. Champ facultatif, **non incompatible**.

### Format d'entrée

```
Batiment
└── acv?  { composants?: [ComposantAcv],                       §4.2.1 ; sous_lot du tableau 5 (1.1 à 11.3, 12, 13)
            lots_forfaitaires?: ["8.1" | "8.3" | "8.4" | "8.5" | "10" | "11"],   §4.2.2, valeurs de l'annexe XI selon l'usage
            fluides_lots_forfaitaires?: [{ sous_lot, nom, charge_kg, prg_kgco2_kg, de_kgco2_kg, dve_ans? (17) }],  cas 3
            chantier: { methode: detaille, energies?[{designation, quantite, de_kgco2_unite, source}],
                                 eau_potable_m3, eaux_usees_m3, terres_importees?[{quantite_t, distance_km}],
                                 terres_evacuees?[{quantite_t, distance_km, de_traitement_kgco2_t, source}],
                                 composants?[ComposantAcv] }                         éq. 93 à 97
                    | { methode: simplifie_maison, s_parcelle_m2, s_emprise_sol_m2, terres_importees? }   éq. 98 à 101
                    | { methode: simplifie_autres, n_ete_grue, n_hiver_grue, n_ete_sans_grue, n_hiver_sans_grue,
                                 v_terres_excavees_m3, epsilon1_l_m3? (1), terres_importees?, terres_evacuees? }  éq. 102 à 104
            eau: { assainissement: collectif | non_collectif, eaux_pluviales: infiltration | separatif | unitaire,
                   pluviometrie_mm?, s_toitures_m2?, s_parkings_impermeabilises_m2?,
                   q_eau_potable_saisie_m3? (obligatoire si « N/C », Q218) }   §4.4.2 (calcul simplifié), éq. 85
            parcelle?: { composants?, comblements?[{quantite_t, distance_km}], s_vegetalisee_m2, s_nettoyee_m2,
                         s_impermeabilisees_m2 }                                    §4.6, eau en calcul simplifié
            occupants: { logements: [{ type_t, nombre }] } | { nombre }            Nocc, éq. 131
            exigences?: { s_agrement_ext_m2? (0), immeuble_grande_hauteur? (false), climatisation? (false) }
            donnees_service: { eau_potable_kgco2_m3?, assainissement_collectif_kgco2_m3?,
                               assainissement_non_collectif_kgco2_m3?, transport_terres_kgco2_tkm?,
                               traitement_inertes_kgco2_t?, electricite_chantier_kgco2_kwh?, carburant_kgco2_l?, source } }
ComposantAcv  { id, designation?, sous_lot, quantite, unite?, dve_ans? (DVR), fluide?: { nom, charge_kg, prg_kgco2_kg, de_kgco2_kg? },
                donnee: { type: fdes_individuelle | fdes_collective | pep_individuel | pep_collectif | donnee_configuree
                                | donnee_par_defaut, identifiant, source, dvr_ans,
                          production_kgco2, edification_kgco2, exploitation_kgco2, fin_de_vie_kgco2, module_d_kgco2,
                          exploitation_non_separee? (false), b1_kgco2?, a1_kgco2?, stock_c_kgc? } }
```

Valeurs par unité fonctionnelle, en kg éq. CO2 (phase = somme des modules : A1 à A3, A4 et A5, B1 à B4, C1 à C4, D). La validation exige les seules données de service qu'un calcul demande. Refusé : sous-lot 8.2 (cogénération). Pour un usage « N/C » du tableau 7 ou des zones de consommations différentes, `eau.q_eau_potable_saisie_m3` (m³/an) est obligatoire et remplace `Qeau potable conv × Nocc` (Q218). `ic_construction_occ_kgco2` et les deux autres valeurs par occupant sont absentes si Nocc est nul (Q217). Le lot 13 demande une installation photovoltaïque et des générations (taux d'autoconsommation, Q211).

### Résultats

```
batiments[].acv?
├── per_ans (50)
├── composants, fluides_frigorigenes, chantier, eau, parcelle?   { i_statique, ic } × { production, edification, exploitation,
│                                                                    fin_de_vie, module_d, total }   kg éq. CO2/m²
├── lots[]  { lot, contribution, ic_kgco2_m2, ic_ded_kgco2_m2, udd_pct?, stock_c_kgc_m2,
│             sous_lots[] { sous_lot, forfaitaire, contribution, ic_ded_kgco2_m2, stock_c_kgc_m2 } }
├── ic_composants, ic_chantier, ic_eau, ic_energie?, ic_construction, ic_batiment?, ic_parcelle?, ic_projet?   (_kgco2_m2)
├── ic_construction_occ_kgco2?, ic_batiment_occ_kgco2?, ic_projet_occ_kgco2?
├── ic_ded_kgco2_m2 (hors lots 1, 2, 13), ic_ded_total_kgco2_m2, udd_batiment_pct?, stock_c_kgc_m2, stock_c_parcelle_kgc_m2?
├── eau_quantites { nocc, q_eau_potable_conv_m3, q_eau_potable_m3, q_eaux_usees_m3, q_eaux_pluviales_m3,
│                   q_parcelle_eau_potable_m3, q_parcelle_eaux_usees_m3 }
├── tac_pv_pct?
├── exigences?  { ic_construction_max_kgco2_m2, respectee,
│                 zones[] { zone, usage, sref_m2, ic_construction_max_moyen, mi_combles, mi_surf_moy, mi_surf_tot,
│                           mi_geo, mi_infra, mi_vrd, mi_pv, mi_ded, mi_agrement_ext, mi_hsp, mi_clim_rcu,
│                           ic_construction_max_kgco2_m2 } }
└── valeurs_par_defaut[]
```

Avec `exigences` et `acv`, `indicateurs.exigences` porte `ic_construction_max_kgco2_m2` et une vérification `ic_construction`, comptée dans `conforme`.
