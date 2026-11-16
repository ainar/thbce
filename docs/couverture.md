<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Couverture de la méthode Th-BCE 2020

Ce document suit, fiche par fiche, l'état d'implémentation de la méthode dans le moteur.

- **Texte de référence** : annexe III de l'arrêté du 4 août 2021 modifié, « Méthode de calcul détaillée Th-BCE 2020 », version consolidée intégrant les arrêtés du 6 avril 2022, du 22 décembre 2022, du 1er février 2023 et du 19 mars 2026 (`docs/reference/arrete/annexeiii_arrete_4_aout_2021_complet_compresse.pdf`, 1 854 pages, 144 fiches, équations numérotées jusqu'à 3387, 420 tableaux).
- **Textes complémentaires** : articles de l'arrêté (coefficients d'énergie primaire à l'article 9, facteurs d'émission à l'article 10), annexe II (règles générales, Ic énergie), règles Th-Bat 2020, annexe 7 de l'arrêté du 15 septembre 2006 (réseaux de chaleur). Voir `docs/reference/README.md`.
- **Les numéros de page** sont ceux du PDF de l'annexe III (pagination « n/1854 »).

## Statuts

| Statut | Signification |
|---|---|
| non commencé | aucune ligne de code |
| partiel | une partie des équations de la fiche est codée |
| implémenté | toutes les équations de la fiche sont codées et testées unitairement |
| validé | résultats conformes aux cas de référence dans la tolérance |

Tant qu'aucun cas de référence n'est disponible, aucune fiche ne peut dépasser le statut « implémenté ».

## Périmètre de la première version (validé le 16 novembre 2026)

- Usages : maison individuelle ou accolée (usage 1) et logement collectif (usage 2).
- Second lot : espaces tampons (4b).
- Hors première version : Titres V du chapitre 16.
- Tout composant hors périmètre est reconnu à la lecture et produit une erreur « non pris en charge ».

## Lots

| Lot | Contenu | Fiches | Pages |
|---|---|---|---|
| 2 | Squelette, conventions, fiches d'application (règles de saisie) | 6 | 15 |
| 3 | Climat et ensoleillement | 2 | 25 |
| 4 | Besoins (Bbio) : bâti, scénarios, éclairage, modèle thermique de groupe | 22 | 252 |
| 4b | Espaces tampons | 3 | 24 |
| 5 | Confort d'été (DH) | 2 | 17 |
| 6a | Ventilation : simple flux, puis double flux | 5 | 110 |
| 6b | Émission et distribution (chauffage, refroidissement, ECS), pertes récupérables | 16 | 160 |
| 6c | Effet Joule et pompes à chaleur (air/eau, chauffe-eau thermodynamique), assemblage de la génération, ballons | 13 | 239 |
| 6d | Chaudières gaz et bois, poêles et inserts, réseaux de chaleur | 3 | 26 |
| 6e | Logement collectif : éclairage des parties communes, ascenseurs, parkings | 4 | 31 |
| 7a | Production photovoltaïque | 4 | 15 |
| 7b | Solaire thermique | 7 | 81 |
| 7 | Indicateurs : Cep, Cep,nr, bilans, usages mobiliers, Ic énergie | 5 | 36 |
| après v1 | Systèmes non retenus dans l'ordre validé | 26 | 359 |
| hors lot 1 | Titres V | 24 | 445 |

Les lots 6a à 6e, 7a et 7b suivent l'ordre fixé par le mainteneur ; le lot 6e vient en dernier dans le lot 6, avant la production photovoltaïque, et fait partie de la première version (décision Q18). Le rattachement fiche par fiche a été fait d'après les titres pour les fiches de systèmes non encore lues ; il sera ajusté à leur lecture.

Les fiches préfixées `FA_` sont des fiches d'application : elles encadrent la saisie et ne contiennent pas d'algorithme horaire. Elles se traduisent par des règles de validation dans `thbce-io`.

## Autres textes

| Texte | Contenu utile | Usage dans le moteur | Statut |
|---|---|---|---|
| Arrêté, article 9 | Coefficients de conversion en énergie primaire et en énergie primaire non renouvelable | `thbce-data` ; recoupe le tableau 333 de l'annexe III | non commencé |
| Arrêté, article 10 | Facteurs d'émission des énergies (kg éq. CO2/kWh EF PCI) | `thbce-data` ; valeurs identiques aux fiches INIES « [RE2020] » | non commencé |
| Arrêté, article 11 | Coefficients de pondération dynamique (Journal officiel du 15 août 2021, p. 59) | `data/src/ponderation_dynamique.rs` | implémenté |
| Arrêté du 26 octobre 2010, annexe I | Zone climatique de chaque département | `data/src/zones_climatiques/`, table générée par `xtask` | implémenté |
| Annexe II §2.2.1 | Données conventionnelles du calcul énergétique | `thbce-data` | non commencé |
| Annexe II §4.3 | Ic énergie, éq. (72) à (75) | `core/src/indicateurs/ic_energie.rs` (lot 7) | non commencé |
| Annexe II §5.1, 5.2 | Indicateurs DH, Bbio, Cep, Cep,nr | `core/src/indicateurs/` | non commencé |
| Annexe 7 de l'arrêté du 15 septembre 2006 | Facteur d'émission (5e colonne) et ratio EnR&R (6e colonne) par réseau de chaleur | `thbce-data`, par script | non commencé |
| A3 §3.1.3, tableau des températures de base | Température de base par département | `data/src/temperature_base/`, table générée par `xtask` | implémenté |
| Scénarios conventionnels (classeur du 29 avril 2026) | 28 onglets, un par usage | `data/src/scenarios/`, table générée par `xtask` (187 grilles hebdomadaires et 29 grilles annuelles distinctes) | converti et recoupé avec le tableau de synthèse pour les usages 1 à 3 |
| Annexe VI | Contenu du RSEE, au niveau des chapitres seulement | insuffisant pour écrire un lecteur XML (décision Q3) | en attente de fichiers d'exemple |
| Règles Th-Bat 2020 | Détermination des données d'entrée (U, Ψ, inertie, facteurs solaires) | hors moteur : ces grandeurs sont des entrées | — |

## Annexe III, fiche par fiche

### 1. Generalites (p. 15–16)

| § | Fiche | Pages | Lot | Module prévu (`crates/thbce-…`) | Statut |
|---|---|---|---|---|---|
| 1.1 | Introduction | 15–15 | — | `(principes, pas de code)` | non commencé |
| 1.2 | Architecture des fiches algorithmes | 16–16 | — | `(principes, pas de code)` | non commencé |
| 1.3 | Divers | 16–16 | 2 | `core/src/conventions.rs`, `core/src/calcul.rs` | implémenté (arrondi de restitution, surfaces de référence) |

### 2. Les principales donnees conventionnelles (p. 17–26)

| § | Fiche | Pages | Lot | Module prévu (`crates/thbce-…`) | Statut |
|---|---|---|---|---|---|
| 2.1 | LES DONNEES CONVENTIONNELLES D'OCCUPATION ET D'USAGE | 17–26 | 4 | `data/scenarios/ (voir aussi §15)` | implémenté : données converties du classeur officiel |

### 3. Meteo et environnement exterieur (p. 27–51)

| § | Fiche | Pages | Lot | Module prévu (`crates/thbce-…`) | Statut |
|---|---|---|---|---|---|
| 3.1 | C_EEX_Climat exterieur | 27–33 | 3 | `core/src/climat/climat_exterieur.rs` | implémenté : corrections d'altitude, température de base, EDn et Edi, heure légale, période de confort adaptatif (`climat_exterieur.rs`, `confort_adaptatif.rs`, `calendrier.rs`) |
| 3.2 | C_EEX_environnement_proche | 34–51 | 3 | `core/src/climat/environnement_proche.rs` | implémenté : éq. (3) à (26), six types de masques ; masques proches des ponts thermiques au lot 4 |

### 4. Scenarios et environnement interieur (p. 52–96)

| § | Fiche | Pages | Lot | Module prévu (`crates/thbce-…`) | Statut |
|---|---|---|---|---|---|
| 4.1 | C_EIN_Scénarios conventionnels | 52–63 | 4 | `core/src/scenarios/scenarios_conventionnels.rs` | implémenté pour les usages 1 et 2 : éq. (27) à (55), calendrier dans `climat/calendrier.rs` ; autres usages non pris en charge |
| 4.2 | FA_Maison_individuelle_Logement_Collectif | 64–68 | 2 | `io : règles de saisie (fiche d’application)` | non commencé |
| 4.3 | FA_Comment identifier l’usage d’un bâtiment et l’exigence associée | 69–69 | 2 | `io : règles de saisie (fiche d’application)` | non commencé |
| 4.4 | FA_Extension_de_bâtiment | 70–70 | 2 | `io : règles de saisie (fiche d’application)` | non commencé |
| 4.5 | C_Ein_Indicateurs de confort | 71–79 | 5 | `core/src/confort/indicateurs_confort.rs` | non commencé |
| 4.6 | C_Ein_Détermination des saisons | 80–94 | 4 | `core/src/scenarios/saisons.rs` | non commencé |
| 4.7 | C_EIN_calcul des consommations electriques des équipements mobiliers | 95–96 | 7 | `core/src/scenarios/equipements_mobiliers.rs` | non commencé |

### 5. Bati (p. 97–320)

| § | Fiche | Pages | Lot | Module prévu (`crates/thbce-…`) | Statut |
|---|---|---|---|---|---|
| 5.1 | S3_BAT-Assemblage_Batiment | 98–103 | 4 | `core/src/bati/assemblage_batiment.rs` | non commencé |
| 5.2 | S1-Bat-Assemblage des espaces tampons | 104–107 | 4b | `core/src/bati/espace_tampon/assemblage.rs` | non commencé |
| 5.3 | Gestion/Régulation des espaces tampons solarisés | 108–117 | 4b | `core/src/bati/espace_tampon/gestion_solarise.rs` | non commencé |
| 5.4 | Espaces tampons | 118–127 | 4b | `core/src/bati/espace_tampon/espace_tampon.rs` | non commencé |
| 5.5 | S2_BAT-assemblage_zone | 128–135 | 4 | `core/src/bati/assemblage_zone.rs` | non commencé |
| 5.6 | C_VEN_Débits_d’air_Cep | 136–160 | 6a | `core/src/ventilation/debits_air_cep.rs` | non commencé |
| 5.7 | S2_BAT_assemblage_groupe | 161–172 | 4 | `core/src/bati/assemblage_groupe.rs` | non commencé |
| 5.8 | S1_BAT_assemblage_baies | 173–178 | 4 | `core/src/bati/baie/assemblage.rs` | non commencé |
| 5.9 | C_BAT_Gestion_protections_mobiles | 179–218 | 4 | `core/src/bati/baie/gestion_protections_mobiles.rs` | non commencé |
| 5.10 | C_BAT_Baie vitrée | 219–244 | 4 | `core/src/bati/baie/baie_vitree.rs` | non commencé |
| 5.11 | FA_Classement_au_bruit_d’une_baie | 245–250 | 2 | `io : règles de saisie (fiche d’application)` | non commencé |
| 5.12 | C_BAT_Perméabilité_des_PM | 251–254 | 4 | `core/src/bati/baie/permeabilite_pm.rs` | non commencé |
| 5.13 | C_BAT_Surventilation naturelle par ouverture des baies pour le confort thermique | 255–271 | 4 | `core/src/bati/baie/surventilation_naturelle.rs` | non commencé |
| 5.14 | C_BAT_ensemble_baies | 272–280 | 4 | `core/src/bati/baie/ensemble_baies.rs` | non commencé |
| 5.15 | S1_BAT_assemblage_parois opaques | 281–283 | 4 | `core/src/bati/paroi_opaque/assemblage.rs` | non commencé |
| 5.16 | C_BAT_ensemble_parois opaques | 284–287 | 4 | `core/src/bati/paroi_opaque/ensemble.rs` | non commencé |
| 5.17 | C_BAT_paroi opaque | 288–293 | 4 | `core/src/bati/paroi_opaque/paroi_opaque.rs` | non commencé |
| 5.18 | S1_BAT_assemblage_ponts_thermiques | 294–297 | 4 | `core/src/bati/pont_thermique/assemblage.rs` | non commencé |
| 5.19 | C_BAT_pont thermique | 298–302 | 4 | `core/src/bati/pont_thermique/pont_thermique.rs` | non commencé |
| 5.20 | C_BAT_ensemble_ponts_thermiques | 303–305 | 4 | `core/src/bati/pont_thermique/ensemble.rs` | non commencé |
| 5.21 | C_BAT_comportement thermique d’un groupe | 306–320 | 4 | `core/src/bati/comportement_thermique_groupe.rs` | non commencé |

### 6. Ventilation (p. 321–469)

| § | Fiche | Pages | Lot | Module prévu (`crates/thbce-…`) | Statut |
|---|---|---|---|---|---|
| 6.1 | C_VEN_BBIO | 321–325 | 4 | `core/src/ventilation/ven_bbio.rs` | non commencé |
| 6.2 | C_VEN_Bouche_conduit | 326–341 | 6a | `core/src/ventilation/bouche_conduit.rs` | non commencé |
| 6.3 | C_VEN_ CTA et Double_flux | 342–395 | 6a | `core/src/ventilation/cta_et_double_flux.rs` | non commencé |
| 6.4 | C_VEN_Mécanique_Basse-Pression | 396–399 | 6a | `core/src/ventilation/mecanique_basse_pression.rs` | non commencé |
| 6.5 | C_VEN_Mécanique_SF | 400–410 | 6a | `core/src/ventilation/mecanique_sf.rs` | non commencé |
| 6.6 | C_VEN_Ventilation naturelle et hybride par conduits | 411–428 | après v1 | `core/src/ventilation/ventilation_naturelle_et_hybride_par.rs` | non commencé |
| 6.7 | C_VEN_Ventilation_naturelle_par_ouverture_de_fenêtres | 429–431 | après v1 | `core/src/ventilation/ventilation_naturelle_par_ouverture_de.rs` | non commencé |
| 6.8 | C_VEN_Puits climatique | 432–439 | après v1 | `core/src/ventilation/puits_climatique.rs` | non commencé |
| 6.9 | C_VEN_Puits_hydraulique | 440–448 | après v1 | `core/src/ventilation/puits_hydraulique.rs` | non commencé |
| 6.10 | C_VEN_gestion_régulation_Puits_climatique | 449–453 | après v1 | `core/src/ventilation/gestion_regulation_puits_climatique.rs` | non commencé |
| 6.11 | S1_BAT_assemblage_puits_climatique | 454–460 | après v1 | `core/src/ventilation/assemblage_puits_climatique.rs` | non commencé |
| 6.12 | C_VEN_Fonctions de rafraichissement par surventilation mécanique des systèmes de ventilation | 461–469 | après v1 | `core/src/ventilation/fonctions_de_rafraichissement_par_surventilation.rs` | non commencé |

### 7. Eclairage (p. 470–503)

| § | Fiche | Pages | Lot | Module prévu (`crates/thbce-…`) | Statut |
|---|---|---|---|---|---|
| 7.1 | C_ECL_éclairage | 470–501 | 4 | `core/src/eclairage/eclairage.rs` | non commencé |
| 7.2 | C_ECL_calcul des consommations d’éclairage des parties communes en logement collectif | 502–503 | 6e | `core/src/eclairage/parties_communes.rs` | non commencé |

### 8. Chauffage, refroidissement (p. 504–1018)

| § | Fiche | Pages | Lot | Module prévu (`crates/thbce-…`) | Statut |
|---|---|---|---|---|---|
| 8.1 | C-Emi-Systèmes Emissions du groupe en chaud et en froid | 504–529 | 6b | `core/src/chauffage_refroidissement/systemes_emissions_du_groupe_en.rs` | non commencé |
| 8.2 | FA_Emissions_mixte_et_composite | 530–542 | 6b | `io : règles de saisie (fiche d’application)` | non commencé |
| 8.3 | C_Emi_Bilan hydrique | 543–550 | 6b | `core/src/chauffage_refroidissement/bilan_hydrique.rs` | non commencé |
| 8.4 | C_Ein_Détermination des saisons de fonctionnement des systèmes de chaud et de froid | 551–555 | 6b | `core/src/chauffage_refroidissement/determination_des_saisons_de_fonctionnement.rs` | non commencé |
| 8.5 | C_Ein_Programmation des relances des systèmes | 556–561 | 6b | `core/src/chauffage_refroidissement/programmation_des_relances_des_systemes.rs` | non commencé |
| 8.6 | S1_Dist_Assemblage des systèmes de chauffage et de refroidissement | 562–568 | 6b | `core/src/chauffage_refroidissement/assemblage_des_systemes_de_chauffage.rs` | non commencé |
| 8.7 | C-DIST-Gestion/régulation des distributions de chauffage et de refroidissement du groupe | 569–580 | 6b | `core/src/chauffage_refroidissement/gestion_regulation_des_distributions_de.rs` | non commencé |
| 8.8 | C-Dist-Distribution de chauffage et de refroidissement du groupe | 581–587 | 6b | `core/src/chauffage_refroidissement/distribution_de_chauffage_et_de.rs` | non commencé |
| 8.9 | C-DIST-Gestion/régulation des distributions intergroupes de chaud et de froid | 588–602 | 6b | `core/src/chauffage_refroidissement/gestion_regulation_des_distributions_intergroupes.rs` | non commencé |
| 8.10 | C-Dist-Distribution intergroupes de chaud et de froid | 603–609 | 6b | `core/src/chauffage_refroidissement/distribution_intergroupes_de_chaud_et.rs` | non commencé |
| 8.11 | C-Dist-Gestion/régulation des réseaux de distribution des CTA | 610–618 | après v1 | `core/src/chauffage_refroidissement/gestion_regulation_des_reseaux_de.rs` | non commencé |
| 8.12 | C-Dist-Distributions intergroupes des CTA | 619–622 | après v1 | `core/src/chauffage_refroidissement/distributions_intergroupes_des_cta.rs` | non commencé |
| 8.13 | FA_Classement des réseaux de distribution en ou hors volume chauffé | 623–625 | 6b | `io : règles de saisie (fiche d’application)` | non commencé |
| 8.14 | S1_Syst_Assemblage de la génération | 626–631 | 6c | `core/src/chauffage_refroidissement/t_assemblage_de_la_generation.rs` | non commencé |
| 8.15 | C_Gen_Transferts Entre Generateurs Sur Boucle D’eau | 632–646 | après v1 | `core/src/chauffage_refroidissement/transferts_entre_generateurs_sur_boucle.rs` | non commencé |
| 8.16 | C_GEN_Transferts entre locaux par les générateurs DRV ou thermofrigopompes | 647–652 | après v1 | `core/src/chauffage_refroidissement/transferts_entre_locaux_par_les.rs` | non commencé |
| 8.17 | C_GEN_Gestion/régulation de la génération | 653–689 | 6c | `core/src/chauffage_refroidissement/gestion_regulation_de_la_generation.rs` | non commencé |
| 8.18 | C_GEN_Générateur direct à effet joule | 690–693 | 6c | `core/src/chauffage_refroidissement/generateur_direct_a_effet_joule.rs` | non commencé |
| 8.19 | C_GEN_Chaudières_gaz_fioul_bois | 694–709 | 6d | `core/src/chauffage_refroidissement/chaudieres_gaz_fioul_bois.rs` | non commencé |
| 8.20 | C_GEN_AUTRES_GENERATEURS_A_COMBUSTION | 710–726 | après v1 | `core/src/chauffage_refroidissement/autres_generateurs_a_combustion.rs` | non commencé |
| 8.21 | C_GEN_système de cogénération | 727–743 | après v1 | `core/src/chauffage_refroidissement/systeme_de_cogeneration.rs` | non commencé |
| 8.22 | C_GEN_poêles et inserts | 744–747 | 6d | `core/src/chauffage_refroidissement/poeles_et_inserts.rs` | non commencé |
| 8.23 | C_GEN_THERMODYNAMIQUE_Elec | 748–821 | 6c | `core/src/chauffage_refroidissement/thermodynamique_elec.rs` | non commencé |
| 8.24 | C_GEN_THERMODYNAMIQUE_GAZ | 822–889 | après v1 | `core/src/chauffage_refroidissement/thermodynamique_gaz.rs` | non commencé |
| 8.25 | C_Gen_PAC Moteur_Gaz | 890–911 | après v1 | `core/src/chauffage_refroidissement/pac_moteur_gaz.rs` | non commencé |
| 8.26 | C_Gen_Sources amont des générateurs thermodynamiques | 912–929 | 6c | `core/src/chauffage_refroidissement/sources_amont_des_generateurs_thermodynamiques.rs` | non commencé |
| 8.27 | C_Gen_Sources amont des générateurs thermodynamiques de type sol | 930–934 | après v1 | `core/src/chauffage_refroidissement/sources_amont_des_generateurs_thermodynamiques.rs` | non commencé |
| 8.28 | C_gen_Réseau de chaleur et de froid | 935–940 | 6d | `core/src/chauffage_refroidissement/reseau_de_chaleur_et_de.rs` | non commencé |
| 8.29 | C_GEN_Calculs génération | 941–962 | 6c | `core/src/chauffage_refroidissement/calculs_generation.rs` | non commencé |
| 8.30 | C_GEN_Echangeur_Geocooling | 963–970 | après v1 | `core/src/chauffage_refroidissement/echangeur_geocooling.rs` | non commencé |
| 8.31 | C_Sys_Rafraichissement par geocooling (en groupe non-climatisé) | 971–988 | après v1 | `core/src/chauffage_refroidissement/rafraichissement_par_geocooling_en_groupe.rs` | non commencé |
| 8.32 | C_BAT_Fonctionnement/gestion/régulation des brasseurs d’air | 989–1002 | après v1 | `core/src/chauffage_refroidissement/fonctionnement_gestion_regulation_des_brasseurs.rs` | non commencé |
| 8.33 | Sn_Methode_VEFA_TCH_Chauffage_Effet_Joule | 1003–1018 | après v1 | `core/src/chauffage_refroidissement/methode_vefa_tch_chauffage_effet.rs` | non commencé |

### 9. Ecs (p. 1019–1294)

| § | Fiche | Pages | Lot | Module prévu (`crates/thbce-…`) | Statut |
|---|---|---|---|---|---|
| 9.1 | S1_BAT_production_instantanée_ECS_(dé)centralisée | 1019–1024 | 6c | `core/src/ecs/production_instantanee_ecs_de_centralisee.rs` | non commencé |
| 9.2 | S3_BAT_production_stockage_ECS_(dé)centralisée | 1025–1030 | 6c | `core/src/ecs/production_stockage_ecs_de_centralisee.rs` | non commencé |
| 9.3 | S3_BAT_production_ECS_centralisée_appoints_décentralisés | 1031–1036 | après v1 | `core/src/ecs/production_ecs_centralisee_appoints_decentralises.rs` | non commencé |
| 9.4 | S3_BAT_production_mixte | 1037–1041 | après v1 | `core/src/ecs/production_mixte.rs` | non commencé |
| 9.5 | C_EMI_Emission_ECS | 1042–1048 | 6b | `core/src/ecs/emission_ecs.rs` | non commencé |
| 9.6 | C_EIN_besoins_ECS | 1049–1066 | 6b | `core/src/ecs/besoins_ecs.rs` | non commencé |
| 9.7 | C_DIS_Distribution_ECS_du_groupe | 1067–1073 | 6b | `core/src/ecs/distribution_ecs_du_groupe.rs` | non commencé |
| 9.8 | C_DIS_Distribution_ECS_intergroupe | 1074–1085 | 6b | `core/src/ecs/distribution_ecs_intergroupe.rs` | non commencé |
| 9.9 | C_STO_Ballon_de_stockage | 1086–1099 | 6c | `core/src/ecs/ballon_de_stockage.rs` | non commencé |
| 9.10 | C_STO_gestion_régulation_ballon | 1100–1113 | 6c | `core/src/ecs/gestion_regulation_ballon.rs` | non commencé |
| 9.11 | C_STO_échangeur_ballon | 1114–1123 | après v1 | `core/src/ecs/echangeur_ballon.rs` | non commencé |
| 9.12 | C_STO_Accumulateur_ECS en eau technique | 1124–1146 | après v1 | `core/src/ecs/accumulateur_ecs_en_eau_technique.rs` | non commencé |
| 9.13 | S1_GEN_générateur_pour_ballon | 1147–1151 | 6c | `core/src/ecs/generateur_pour_ballon.rs` | non commencé |
| 9.14 | S2_ GEN_ Assemblage constitué d’un ou plusieurs ballon et d’un ou plusieurs générateurs | 1152–1163 | 6c | `core/src/ecs/assemblage_constitue_d_un_ou.rs` | non commencé |
| 9.15 | C_GEN_Boucle solaire | 1164–1170 | 7b | `core/src/ecs/boucle_solaire.rs` | non commencé |
| 9.16 | S1_GEN_CESCI | 1171–1196 | 7b | `core/src/ecs/cesci.rs` | non commencé |
| 9.17 | S2_GEN_CESCAI | 1197–1213 | 7b | `core/src/ecs/cescai.rs` | non commencé |
| 9.18 | C_STO_Gestion-régulation_de_la_Boucle_de_Chauffage_Solaire | 1214–1218 | 7b | `core/src/ecs/gestion_regulation_de_la_boucle.rs` | non commencé |
| 9.19 | S2_GEN_Système_Solaire_Combiné_appoint_Chauffage_indépendant | 1219–1228 | 7b | `core/src/ecs/systeme_solaire_combine_appoint_chauffage.rs` | non commencé |
| 9.20 | S2_GEN_Système_Solaire_Combiné_appoint_chauffage raccordé | 1229–1240 | 7b | `core/src/ecs/systeme_solaire_combine_appoint_chauffage.rs` | non commencé |
| 9.21 | C_BAT_Récupérateur de chaleur sur les eaux grises | 1241–1268 | après v1 | `core/src/ecs/recuperateur_de_chaleur_sur_les.rs` | non commencé |
| 9.22 | PR1_IdCET_ Saisie des chauffe-eau thermodynamiques à compression électrique | 1269–1289 | 6c | `core/src/ecs/idcet_saisie_des_chauffe_eau.rs` | non commencé |
| 9.23 | PR2_IdCESI_ Saisie des chauffe-eau solaires individuels (Kit CESI) | 1290–1293 | 7b | `core/src/ecs/idcesi_saisie_des_chauffe_eau.rs` | non commencé |
| 9.24 | FA_Bâtiments non équipés de production d’eau chaude sanitaire | 1294–1294 | 2 | `io : règles de saisie (fiche d’application)` | non commencé |

### 10. Deplacement des occupants à l’intérieur du bâtiment (p. 1295–1332)

| § | Fiche | Pages | Lot | Module prévu (`crates/thbce-…`) | Statut |
|---|---|---|---|---|---|
| 10.1 | C_Bat_Ascenseurs | 1295–1308 | 6e | `core/src/deplacements/ascenseurs.rs` | non commencé |
| 10.2 | C_Bat_Escalators | 1309–1317 | après v1 | `core/src/deplacements/escalators.rs` | non commencé |
| 10.3 | C_VEN_Calcul des consommations de ventilation des parcs de stationnement fermés | 1318–1326 | 6e | `core/src/deplacements/calcul_des_consommations_de_ventilation.rs` | non commencé |
| 10.4 | C_ECL_calcul des consommations d’éclairage des parkings | 1327–1332 | 6e | `core/src/deplacements/calcul_des_consommations_d_eclairage.rs` | non commencé |

### 11. Pertes (p. 1333–1339)

| § | Fiche | Pages | Lot | Module prévu (`crates/thbce-…`) | Statut |
|---|---|---|---|---|---|
| 11.1 | C_PER_Pertes et consommations électriques récupérables | 1333–1339 | 6b | `core/src/pertes/pertes_recuperables.rs` | non commencé |

### 12. Production locale d’electricite (p. 1340–1354)

| § | Fiche | Pages | Lot | Module prévu (`crates/thbce-…`) | Statut |
|---|---|---|---|---|---|
| 12.1 | S1_PV_installation_PV | 1340–1342 | 7a | `core/src/pv/installation.rs` | non commencé |
| 12.2 | S2_PV_global_PV | 1343–1345 | 7a | `core/src/pv/global.rs` | non commencé |
| 12.3 | C_PV_capteur_PV | 1346–1351 | 7a | `core/src/pv/capteur.rs` | non commencé |
| 12.4 | C_PV_onduleurs_PV | 1352–1354 | 7a | `core/src/pv/onduleur.rs` | non commencé |

### 13. Calcul des indicateurs (p. 1355–1406)

| § | Fiche | Pages | Lot | Module prévu (`crates/thbce-…`) | Statut |
|---|---|---|---|---|---|
| 13.1 | C_CALC_Sorties du mode Th-B | 1355–1364 | 4 | `core/src/indicateurs/sorties_th_b.rs` | partiel : surfaces de référence, éq. (2375) à (2378), dans `core/src/calcul.rs` |
| 13.2 | C_CALC_Sorties du mode Th-C | 1365–1378 | 7 | `core/src/indicateurs/sorties_th_c.rs` | non commencé |
| 13.3 | C_Sn_Methode_Th_C_Forfait de consommations de refroidissement | 1379–1385 | 7 | `core/src/indicateurs/forfait_refroidissement.rs` | non commencé |
| 13.4 | C_Calc_Bilans_Energies_Consommees_et_importées | 1386–1395 | 7 | `core/src/indicateurs/bilans_energies.rs` | non commencé |
| 13.5 | C_BAT_Confort_d’été | 1396–1403 | 5 | `core/src/confort/confort_ete.rs` | non commencé |
| 13.6 | C_CALC_Indicateurs pédagogiques en fonction du nombre d’occupants | 1404–1406 | 7 | `core/src/indicateurs/indicateurs_pedagogiques.rs` | non commencé |

### 14. Glossaire (p. 1407–1408)

Glossaire, pas de code.


### 15. Scenarios conventionnels (p. 1409–1409)

Renvoi vers le fichier externe des scénarios conventionnels (voir question Q2) → `thbce-data`.


### 16. Integration des titres v « systeme » rt2012 en re2020 (p. 1410–1854)

| § | Fiche | Pages | Lot | Module prévu (`crates/thbce-…`) | Statut |
|---|---|---|---|---|---|
| 16.1 | C_BAT_Fenetres parietodynamiques Ridoret | 1410–1416 | hors lot 1 | `core/src/titres_v/fenetres_parietodynamiques_ridoret.rs` | non commencé |
| 16.2 | C_GEN_Conduit echangeur Poujoulat sur appareil indépendant de chauffage à bois | 1417–1428 | hors lot 1 | `core/src/titres_v/conduit_echangeur_poujoulat_sur_appareil.rs` | non commencé |
| 16.3 | C_GEN_PAC_Triple service air-eau/air T.One Aldes et Triple C Hitachi | 1429–1429 | hors lot 1 | `core/src/titres_v/pac_triple_service_air_eau.rs` | non commencé |
| 16.4 | FA_PAC_hybrides electriques gaz | 1430–1436 | hors lot 1 | `core/src/titres_v/pac_hybrides_electriques_gaz.rs` | non commencé |
| 16.5 | C_GEN_THERMODYNAMIQUE_Elec_PAC_CO2 | 1437–1447 | hors lot 1 | `core/src/titres_v/thermodynamique_elec_pac_co2.rs` | non commencé |
| 16.6 | _Gen_Thermodynamique_Elec_DoubleService_Air_Extrait_Eau_NIBE | 1448–1472 | hors lot 1 | `core/src/titres_v/gen_thermodynamique_elec_doubleservice_air.rs` | non commencé |
| 16.7 | C_Gen_Source_Amont_NIBE | 1473–1480 | hors lot 1 | `core/src/titres_v/source_amont_nibe.rs` | non commencé |
| 16.8 | C_Dist_Distribution_Intergroupe_Mixte_MTA | 1481–1525 | hors lot 1 | `core/src/titres_v/distribution_intergroupe_mixte_mta.rs` | non commencé |
| 16.9 | C_Dist_Distribution_Intergroupe_ECS_MTA | 1526–1541 | hors lot 1 | `core/src/titres_v/distribution_intergroupe_ecs_mta.rs` | non commencé |
| 16.10 | C_GEN_THERMODYNAMIQUE_heliopac | 1542–1576 | hors lot 1 | `core/src/titres_v/thermodynamique_heliopac.rs` | non commencé |
| 16.11 | C_GEN_pompe a chaleur multifonction Acquarevia+ | 1577–1589 | hors lot 1 | `core/src/titres_v/pompe_a_chaleur_multifonction_acquarevia.rs` | non commencé |
| 16.12 | C_GEN_AUER Chauffe eau thermodynamique_Edel Eau | 1590–1605 | hors lot 1 | `core/src/titres_v/auer_chauffe_eau_thermodynamique_edel.rs` | non commencé |
| 16.13 | C_GEN_PAC_sur_eaux_grises | 1606–1662 | hors lot 1 | `core/src/titres_v/pac_sur_eaux_grises.rs` | non commencé |
| 16.14 | C_GEN_Radiateur numérique Q.rad | 1663–1670 | hors lot 1 | `core/src/titres_v/radiateur_numerique_q_rad.rs` | non commencé |
| 16.15 | C_GEN_ Chaudière numérique et rechauffeur de boucle QB1 | 1671–1678 | hors lot 1 | `core/src/titres_v/chaudiere_numerique_et_rechauffeur_de.rs` | non commencé |
| 16.16 | C_GEN_Appoint thermodynamique (ECS et double service) | 1679–1692 | hors lot 1 | `core/src/titres_v/appoint_thermodynamique_ecs_et_double.rs` | non commencé |
| 16.17 | C_MTA_AUER_LOGIX | 1693–1700 | hors lot 1 | `core/src/titres_v/auer_logix.rs` | non commencé |
| 16.18 | C_VEN_VMC_Thermodynamique_double_flux | 1701–1735 | hors lot 1 | `core/src/titres_v/vmc_thermodynamique_double_flux.rs` | non commencé |
| 16.19 | C_GEN_GECO_PKOM4 Classic | 1736–1757 | hors lot 1 | `core/src/titres_v/geco_pkom4_classic.rs` | non commencé |
| 16.20 | C_GEN_Prechauffage de l’air insufflé dans le bâtiment par systemes SYSTOVI R-Sun, R-VOLT | 1758–1783 | hors lot 1 | `core/src/titres_v/prechauffage_de_l_air_insuffle.rs` | non commencé |
| 16.21 | C_GEN_THERMODYNAMIQUE_YACK_BIONIC | 1784–1817 | hors lot 1 | `core/src/titres_v/thermodynamique_yack_bionic.rs` | non commencé |
| 16.22 | C_GEN_TWIDO production ECS photovoltaïque | 1818–1824 | hors lot 1 | `core/src/titres_v/twido_production_ecs_photovoltaique.rs` | non commencé |
| 16.23 | C_GEN_Récupération_de_chaleur_sur_chambres_froides_BOOSTHERM | 1825–1835 | hors lot 1 | `core/src/titres_v/recuperation_de_chaleur_sur_chambres.rs` | non commencé |
| 16.24 | C_GEN_Récupération_de_chaleur_sur_installations_frigorifiques_RIDEL_Energy | 1836–1854 | hors lot 1 | `core/src/titres_v/recuperation_de_chaleur_sur_installations.rs` | non commencé |