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
                      émetteurs d'ECS
```

Usages calculés : maison individuelle ou accolée (`maison_individuelle`) et logement collectif (`logement_collectif`) ; les 26 autres usages du tableau 4 sont reconnus et refusés.

## Enveloppe

- **Parois opaques** : type (`u_k_w_m2k`, `incl_k`, facteurs solaires `sf_c_k` et `sf_e_k`, végétalisation facultative) et instance (`type_paroi`, `a_k_m2`, `exposition`, `espace_tampon?`).
- **Exposition** (A3 §3.2.2) : `azimut_deg` (0 sud, 90 ouest, 180 nord, 270 est), `inclinaison_deg` (0 horizontale vers le haut, 90 verticale), `lpb_m` et `hpb_m` (requis avec un masque proche), `masques` (0 à 7 identifiants du catalogue).
- **Masques** : `lointain_azimutal` et `lointain_arbres_caducs` (36 hauteurs `gamma_i_deg`), `lointain_plan_vertical`, `proche_vertical_droit`, `proche_vertical_gauche`, `proche_horizontal`.
- **Baies** : type (`u_sp_vert_w_m2k`/`u_sp_horiz_w_m2k`, `u_ap_…`, facteurs `sans_protection` et `avec_protection`, `protection_mobile?`, `ouvrant?`) et instance (`a_b_m2`, `exposition`, `expo_br`, `espace_tampon?`). Stores vénitiens et gestion automatique des protections refusés.
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
                                             u_w_mk, rechauffeur, p_circ_w, espace_tampon? } | trace { … } }
Groupe.emetteurs_ecs[]                     { id, rat_em, nb_logements, part_melangeur, part_mitigeur,
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

**Relecture du lot 7b (17 novembre 2026).** `chauffe_eau_solaires[].description.kit.k_theta` devient **requis** (changement incompatible ; sortie de l'outil IdCESI, Q52 point 1) ; un kit n'admet que l'appoint `integre`. Un système solaire combiné demande un réseau hydraulique de chauffage du groupe, relié à sa génération, dont `theta_dep_dim_c` est saisie (« θch_max^gen », éq. 2117). Résultats : `generations[].stockages[]` porte `v_zones_l`, `theta_zones_debut_c` et `theta_zones_fin_c` (volumes et températures des quatre zones d'un ballon, de bas en haut, au début et à la fin du calcul), qui ferment le bilan des ballons.

**Lot 7c (fiches 9.16 et 9.17).** `generations[].chauffe_eau_solaires_collectifs[]` (au plus un par génération, seul assemblage avec ballon de sa génération) : `id`, `type_pcad` (`cesci`, `cescai` ou `cesci_appoint_separe`), `exposition_capteurs` (inclinaison de 0 à 90°), `capteurs` et `boucle` (comme pour un chauffe-eau solaire décrit par ses composants, pour tout le champ ; régulation `irradiance` requise pour un CESCI, éq. 1981 ; `h_rel_ech` refusé), `ballon_central` (CESCAI seulement, comme `ballon`) et `ensembles[]` (au moins un : `id`, `nb_ballons`, `ballon` individuel, `appoint` intégré comme celui d'un stockage — requis pour `cesci` et `cescai`, sans objet pour `cesci_appoint_separe` —, générateur d'effet Joule de fonction `ecs` ou chaudière assurant l'ECS, dont `rdim` vaut `nb_ballons`). Réseau d'ECS intergroupe de la génération : `aucun` pour un CESCI, `boucle` sans réchauffeur pour un CESCAI. Un CESCI à appoint séparé demande un générateur instantané d'ECS dans la génération. Résultats : `generations[].solaires_collectifs[]` (`id`, `type_pcad`, `solaire` comme pour un chauffe-eau solaire, `ensembles[]` comme `stockages[]` pour tous les ballons d'un ensemble, `ballon_central`, `q_fou_reseau_kwh`, `pertes_reseau_deduites_kwh`). Lectures : Q53.

## Indicateurs réglementaires et exigences (lot 7)

### Format d'entrée

Champs facultatifs, **non incompatibles** (`docs/questions/Q054.md`) :

```
Batiment
└── exigences?  { categorie_contraintes_exterieures: categorie_1 | categorie_2 | categorie_3,  chap. V, partie II (saisie, contrôlée)
                  zone_bruit: br1 | br2 | br3,                                    chap. V, partie I (saisie) ; Mbbruit
                  annee_depot_permis (≥ 2022),                                    Icénergie_maxmoyen, dates d'effet (Q68)
                  mois_depot_permis? (1 à 12),                                    obligatoire en 2026, et en 2022 avec un usage 3 à 5 (Q68, Q69)
                  hsp_moy_m? (1,8 à 10),                                          MbHSP, McHSP (habitation, permis ≥ 07/2026)
                  s_combles_m2? (0),                                              Mbcombles, Mccombles (maisons)
                  reseau_chaleur_classe? (false),                                 Icénergie_maxmoyen jusqu'en 2027
                  parcelle_raccordement_gaz? (false),                             Icénergie_maxmoyen = 280 (maisons, ≤ 2023)
                  zones?: [{ zone,                                                lot 0b (Q66)
                             categorie_contraintes_exterieures?,                  Mbbruit (cat. 3), Mccat, DH_max des groupes
                             zone_bruit?,                                         Mbbruit
                             groupes?: [{ groupe, categorie_contraintes_exterieures }] }] }   DH_max du groupe
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
                        conforme }
```

`energie` vaut `electricite`, `gaz_naturel`, `gpl`, `fioul`, `charbon`, `bois_buches`, `bois_granules`, `bois_plaquettes`, `bois` (combustible inconnu), `reseau_chaleur:<id>` ou `reseau_froid:<id>` (`autres` sans identifiant). La ligne de commande écrit une synthèse des indicateurs et de la conformité sur la sortie d'erreur (`thbce_io::rapport::synthese_indicateurs`).


