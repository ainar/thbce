<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Anomalies de l'index du texte réglementaire

Fichier généré par `cargo run -p xtask -- index-texte`, ne pas modifier à la main (le relevé final est tenu dans `anomalies-constatees.md`).

Les anomalies sont classées par **nature**, car l'index se fonde sur l'extraction texte (calque du PDF), qui n'est pas l'image des pages :

- *Absent de l'extraction* : le numéro n'apparaît pas dans le texte extrait (équation dessinée comme image, numéro écrit autrement, légende absente). Il peut exister sur l'image : la lire avant de conclure à une anomalie du PDF.
- *En double* : le même numéro apparaît plusieurs fois. Peut être une vraie double numérotation du PDF (les deux occurrences sont dans l'index, colonne `doublon`) ou un numéro parasite (cellule de tableau, indice).
- *Hors séquence dans l'extraction* : l'ordre du texte extrait n'est pas l'ordre de la page (calque), ou deux équations sont permutées. L'équation est retenue à sa place dans le texte ; l'image est en général dans l'ordre : artefact d'extraction probable.

Aucun numéro n'est déduit ni inventé : en cas de doute, lire l'image de la page (`cargo run -p xtask -- page`). Pour l'annexe II, la page est la page du PDF (le numéro imprimé est supérieur d'une unité : PDF 68, imprimé 69).

## annexe III

### Fiches (0)

Aucune.

### Équations (87)

#### Absent de l'extraction (7)

- éq. (142) : introuvable dans le texte extrait (pages 147 à 148)
- éq. (590) : introuvable dans le texte extrait (page 391)
- éq. (2270) : introuvable dans le texte extrait (page 1293)
- éq. (3099) : introuvable dans le texte extrait (pages 1645 à 1650)
- éq. (3140) : introuvable dans le texte extrait (page 1669)
- éq. (3170) à (3171) : introuvable dans le texte extrait (pages 1697 à 1698)
- éq. (3180) : introuvable dans le texte extrait (pages 1699 à 1700)

#### En double (68)

- éq. (1) : numéro en double, page 158 (retenu : page 32)
- éq. (15) : numéro en double, page 1803 (retenu : page 41)
- éq. (17) : numéro en double, page 150 (retenu : page 41)
- éq. (23) : numéro en double, page 153 (retenu : page 51)
- éq. (34) : numéro en double, page 156 (retenu : page 60)
- éq. (79) : numéro en double, page 1632 (retenu : page 88)
- éq. (314) : numéro en double, page 309 (retenu : page 312)
- éq. (383) : numéro en double, page 341 (retenu : page 331)
- éq. (384) : numéro en double, page 341 (retenu : page 331)
- éq. (385) : numéro en double, page 341 (retenu : page 331)
- éq. (386) : numéro en double, page 359 (retenu : page 331)
- éq. (387) : numéro en double, page 359 (retenu : page 332)
- éq. (388) : numéro en double, page 359 (retenu : page 332)
- éq. (389) : numéro en double, page 359 (retenu : page 332)
- éq. (390) : numéro en double, page 360 (retenu : page 332)
- éq. (391) : numéro en double, page 360 (retenu : page 333)
- éq. (392) : numéro en double, page 360 (retenu : page 333)
- éq. (393) : numéro en double, page 360 (retenu : page 333)
- éq. (394) : numéro en double, page 360 (retenu : page 333)
- éq. (395) : numéro en double, page 360 (retenu : page 333)
- éq. (396) : numéro en double, page 360 (retenu : page 333)
- éq. (397) : numéro en double, page 361 (retenu : page 333)
- éq. (398) : numéro en double, page 361 (retenu : page 333)
- éq. (399) : numéro en double, page 361 (retenu : page 334)
- éq. (400) : numéro en double, page 361 (retenu : page 336)
- éq. (401) : numéro en double, page 361 (retenu : page 336)
- éq. (402) : numéro en double, page 334 (retenu : page 336)
- éq. (402) : numéro en double, page 361 (retenu : page 336)
- éq. (403) : numéro en double, page 361 (retenu : page 336)
- éq. (404) : numéro en double, page 361 (retenu : page 337)
- éq. (405) : numéro en double, page 334 (retenu : page 337)
- éq. (405) : numéro en double, page 362 (retenu : page 337)
- éq. (406) : numéro en double, page 362 (retenu : page 337)
- éq. (407) : numéro en double, page 362 (retenu : page 337)
- éq. (408) : numéro en double, page 334 (retenu : page 337)
- éq. (408) : numéro en double, page 362 (retenu : page 337)
- éq. (409) : numéro en double, page 363 (retenu : page 338)
- éq. (410) : numéro en double, page 363 (retenu : page 338)
- éq. (411) : numéro en double, page 363 (retenu : page 338)
- éq. (412) : numéro en double, page 364 (retenu : page 338)
- éq. (413) : numéro en double, page 364 (retenu : page 338)
- éq. (414) : numéro en double, page 364 (retenu : page 338)
- éq. (415) : numéro en double, page 364 (retenu : page 338)
- éq. (416) : numéro en double, page 364 (retenu : page 339)
- éq. (417) : numéro en double, page 364 (retenu : page 339)
- éq. (418) : numéro en double, page 364 (retenu : page 339)
- éq. (419) : numéro en double, page 365 (retenu : page 339)
- éq. (420) : numéro en double, page 365 (retenu : page 339)
- éq. (421) : numéro en double, page 365 (retenu : page 339)
- éq. (422) : numéro en double, page 365 (retenu : page 340)
- éq. (423) : numéro en double, page 365 (retenu : page 340)
- éq. (424) : numéro en double, page 365 (retenu : page 340)
- éq. (425) : numéro en double, page 366 (retenu : page 340)
- éq. (426) : numéro en double, page 366 (retenu : page 340)
- éq. (427) : numéro en double, page 366 (retenu : page 340)
- éq. (428) : numéro en double, page 367 (retenu : page 341)
- éq. (429) : numéro en double, page 367 (retenu : page 341)
- éq. (430) : numéro en double, page 367 (retenu : page 341)
- éq. (431) : numéro en double, page 368 (retenu : page 341)
- éq. (432) : numéro en double, page 368 (retenu : page 341)
- éq. (433) : numéro en double, page 368 (retenu : page 341)
- éq. (434) : numéro en double, page 368 (retenu : page 341)
- éq. (435) : numéro en double, page 368 (retenu : page 341)
- éq. (436) : numéro en double, page 368 (retenu : page 341)
- éq. (437) : numéro en double, page 368 (retenu : page 341)
- éq. (438) : numéro en double, page 369 (retenu : page 341)
- éq. (439) : numéro en double, page 369 (retenu : page 341)
- éq. (440) : numéro en double, page 1362 (retenu : page 369)

#### Hors séquence dans l'extraction (12)

- éq. (482) : hors séquence, page 376 (retenue à sa place dans le texte)
- éq. (650) : hors séquence, page 408 (retenue à sa place dans le texte)
- éq. (1208) : hors séquence, page 741 (retenue à sa place dans le texte)
- éq. (1217) : hors séquence, page 742 (retenue à sa place dans le texte)
- éq. (1396) : hors séquence, page 911 (retenue à sa place dans le texte)
- éq. (1554) : hors séquence, page 961 (retenue à sa place dans le texte)
- éq. (2209) : hors séquence, page 1266 (retenue à sa place dans le texte)
- éq. (2683) : hors séquence, page 1498 (retenue à sa place dans le texte)
- éq. (2686) : hors séquence, page 1499 (retenue à sa place dans le texte)
- éq. (2960) : hors séquence, page 1602 (retenue à sa place dans le texte)
- éq. (3172) : hors séquence, page 1698 (retenue à sa place dans le texte)
- éq. (3257) : hors séquence, page 1727 (retenue à sa place dans le texte)

### Tableaux (8)

#### Absent de l'extraction (1)

- tableau 134 : introuvable dans le texte extrait

#### En double (4)

- tableau 10 : légende en double, pages 82 et 91
- tableau 57 : légende en double, pages 335 et 336
- tableau 115 : légende en double, pages 699 et 711
- tableau 122 : légende en double, pages 708 et 712

#### Hors séquence dans l'extraction (3)

- tableau 8 : hors séquence, page 84 (après la légende 10)
- tableau 9 : hors séquence, page 91 (après la légende 10)
- tableau 115 : hors séquence, page 711 (après la légende 122)

### Figures (10)

#### Absent de l'extraction (4)

- figure 67 : introuvable dans le texte extrait
- figure 68 : introuvable dans le texte extrait
- figure 69 : introuvable dans le texte extrait
- figure 75 : introuvable dans le texte extrait

#### En double (3)

- figure 2 : légende en double, pages 19 et 446
- figure 4 : légende en double, pages 20 et 1481
- figure 5 : légende en double, pages 21 et 1482

#### Hors séquence dans l'extraction (3)

- figure 2 : hors séquence, page 446 (après la légende 78)
- figure 4 : hors séquence, page 1481 (après la légende 219)
- figure 5 : hors séquence, page 1482 (après la légende 219)

## annexe II

### Équations (1)

#### Absent de l'extraction (1)

- éq. (96) : introuvable dans le texte extrait (pages 82 à 83)

### Tableaux (0)

Aucune.

### Figures (0)

Aucune.

## Constatées sur l'image (annexe III, tableaux 107 à 266)

Relevé à la main, lors de la transcription des tableaux de la génération (`../tableaux/annexe-iii/`). Contrairement aux rubriques ci-dessus, ce relevé a été vérifié sur l'image des pages et la couche texte vectorielle.

### Index

- **Page de l'index = page de la légende.** Pour un tableau qui s'étend sur plusieurs pages, l'index donne la page de la légende, imprimée sous le tableau : le corps commence plus tôt (107 : p. 627 à 629 ; 108 : 633 à 636 ; 109 : 648 à 649 ; 110 : 656 à 663 ; 111 : 665 à 666 ; 112 : 690 à 691 ; 114 : 695 à 698 ; 123 : 711 à 714 ; 133 : 725 à 726 ; 135 : 727 à 731 ; 140 : 734 à 735 ; 141 : 744 à 745 ; 143 : 749 à 755 ; 217 : 823 à 827 ; 229 : 891 à 895 ; 243 : 914 à 918 ; 244 : 931 à 932 ; 246 : 935 à 936 ; 250 : 942 à 947 ; 253 : 964 à 966 ; 254 : 972 à 976 ; 256 : 990 à 992 ; 259 : 1004 à 1011). Cas où la légende ouvre la page suivante (colonne `tete_de_page` = 1) : 119 (corps p. 701, légende p. 702), 215 (corps p. 807, légende p. 808), 236 (corps p. 903, légende p. 904).
- **Tableau 134 absent** : la nomenclature de la fiche 8.21 est annoncée « Tableau 134 » dans le texte (p. 727) mais sa légende imprimée (p. 731) est « Tableau 135 ». Il n'existe pas de tableau 134 imprimé ; la numérotation reprend 136.
- **115 et 122 en double dans l'index** : ce sont les tableaux imprimés p. 699 et p. 708, et un simple renvoi « Voir Tableau 115 » (p. 711) et « Voir Tableau 121 et Tableau 122 » (p. 712) dans la nomenclature du tableau 123 (nomenclature de la fiche 8.20), pas un second tableau.
- **257 imprimé deux fois** : p. 994-995 (copie sans en-tête, avant le texte « Tableau 257 sont utilisées… ») et p. 995-997 (copie légendée avec en-tête) ; valeurs identiques ; l'index ne garde que la légende, p. 996.
- **131 reprend le contenu de 121** (p. 725 et p. 708) : même grille, numéros distincts ; 132 reprend de même la grille de 122.
- **139** : la légende « Tableau 139 » (p. 734) est placée sous des formules (1175 à 1178), pas sous une grille.
- **119** : le titre imprimé est « gaz, fioul et bis » (faute pour « bois »), conservé par l'index.

### Texte

- **ρe** (masse volumique de l'eau) : 1000 kg/m³ au tableau 253 (p. 966), 1006 kg/m³ au tableau 254 (p. 976). Non corrigé.
- **Tableau 259** (p. 1010) : le symbole de l'efficacité du générateur est un carré vide (glyphe absent) suivi de l'indice `eff` ; ailleurs (tableau 217, p. 825) c'est `η_eff`.
- **Tableau 236** (p. 903) : `Cnnam_ηth,ecs (5, 7)` à la place de (2, 7) et `Cnnam_Paux,uex(-15, -7)` (« uex » pour « uext »).
- **Titres incohérents** : 158 (« PAC air extérieur / air recyclé » pour une fiche dont les étapes portent sur « air extrait / air neuf »), 169 (« PAC eau glycolée / air » pour le COP suivi du 170 « eau de boucle/ air »).
- **Tableau 243** (p. 915) : `P_vent_gaine` est donnée en `°C` (puissance) ; **tableau 250** : `Eep_prelec_m` en `Whep` mais `Eep_prelec` en `Wh`.
- **Tableau 254** : `Waux,am(h)` (consommation électrique de la source amont) est donnée en `Wh`, tel qu'imprimé.
- **Tableau 256** : ligne `θop_dec_br_man` imprimée deux fois.
- **Tableau 262** : coquille « utilsateur ».
- **Tableaux 114 et 123** : renvois rognés par la mise en page (« Voir Tableau 121 et Tableau », « Voir Tableau 129 et Tableau »), p. 696 et p. 714.
- **Tableau 144** : appels de note 8, 9, 10 en exposant, collés à la valeur dans la couche texte (« NF EN 14511-28 », « NF EN 161479 », « NF EN 15879-110 »).

## Incohérences du texte relevées lors de l'implémentation (annexe III)

Vérifiées sur l'image de la page ; la lecture retenue est dans le fichier de question cité.

- éq. (726) (p. 437) : le facteur ×3 600 donne un débit en m³/h alors que la nomenclature du tableau 69 donne `qvair` en m³/s et `v0` en m/s ; facteur écarté (Q231 point 2).
- éq. (730) (p. 439) : « hiver » écrit pour « été » (puits réglé hors période de chauffage) ; lue « été » (Q231 point 1).
- éq. (747) (p. 447) : « hiver » écrit pour « été » dans la gestion de la pompe hors période de chauffage (`θext,arrêt_PH_hiver`) ; lue « été » (Q233 point 3).
- tableau 71 (p. 443) : `Qmax_PH` donné en m³/s alors que les éq. (737), (743) et (745) ne sont homogènes qu'avec un débit en m³/h (comme `QPH(h)`) ; lu en m³/h (Q233 point 1).
- éq. (689) (fiche 6.6, p. 421) : le texte écrit `vcond,s,8 × vvent_réf / vvent,c(h)`, le même rapport que l'éq. (688) (p. 420) ; coquille, la limite est la vitesse du conduit dont la vitesse corrigée vaut `vcond,s,8`, soit `v8 × vvent,c / vvent_réf` (Q234 point 1 ; effet négligeable sur les exemples).
- éq. (701) et (704) (fiche 6.6) : `Aboucheeq`, défini pour la bouche fixe seule, intervient aussi avec une bouche autoréglable ; part des fuites lue `qfuites / (qbouche + qfuites)` (Q235 point 2).
- éq. (707) (fiche 6.6) : pas d'équation du débit soufflé spécifique en refoulement ; lu par analogie (Q235 point 1).
- §8.17.3.7.2.3 (p. 681) : la vérification de cohérence de l'alternance exclut « idfougen = 4 (chauffage et refroidissement) », alors que le tableau 110 numérote 4 « chauffage et ECS » et 5 « chauffage et refroidissement », et que le §8.17.3.7.2.3.4 traite justement le cas « double postes chauffage et ECS (idfougen = 4) » ; fonction exclue lue « chauffage et refroidissement » (Q125 point 1).
- §8.17.3.7.2.3 (p. 681, 683, 684, 685) : le compteur de basculement compare la demande à `Ratdim^gnr × Pngen^(gnr+1)` (nombre de générateurs identiques du générateur courant, puissance du suivant) ; lu comme la puissance installée du suivant, `Ratdim^(gnr+1) × Pngen^(gnr+1)` ; `gnr + 1` n'existe pas pour le dernier générateur (Q125 points 2 et 3).
- §8.17.3.7.2.3.2 (p. 683) : l'algorithme code `ihivernal` 0, 1, 2 (toute l'année, hiver, été) quand le tableau 110 code `id_hivernal` 1, 2, 3 ; les mois 5 et 10 appartiennent aux deux saisons (déjà relevé par Q46 point 6).
- §8.17.3.7.2.3.3 et 8.17.3.7.2.3.4 (p. 684 et 686) : les mises à jour du jeu de données en chauffage écrivent `Qfou_ecs` et `ηeff_ecs` (recopie du cas ECS) ; lues `Qfou_ch` et `ηeff_ch`. P. 686 : « R^gnr_font_ecs » pour `Rfonct_ecs`.
- Fiche 8.24, §8.24.3.6.4.1 (p. 888), éq. (1365) : pertes à l'arrêt par défaut en chauffage `900 + (5500 − 900)/(400 − 20) × (Pnom − 10)`, origine 10 incohérente avec le seuil de 20 kW (saut de 121 W) ; lue `(Pnom − 20)` (Q136). « Pnom » n'est pas défini par le tableau 217 (Q136).
- Fiche 8.24, §8.24.3.6.4.1 (p. 887) : brûleur « tout ou rien » par défaut, suivi du libellé de la valeur 1 (« mode continu ou cycles marche arrêt ») ; lu tout ou rien (Q137).
- Fiche 8.24, §8.24.3.3.1.2.1 et 8.24.3.3.2.2.1 (p. 835 et 840) : « si {ValCOP(3,Ilign)} = 0 » dans le complément des puissances gaz, pour `ValPabs` (Q138). En ECS et en refroidissement, les blocs de correction des auxiliaires renvoient au pivot du chauffage (`ValPabs(4,3)`) et la valeur critique s'écrit `ValGUEch` ; lus au pivot et à la valeur critique du mode (Q138).
- Fiche 8.24, §8.24.3.6.2.2 (p. 884) : « Pgazma est nulle à charge nulle », alors que le terme des pertes de l'éq. (1345) tend vers `Pertes / Rdtcomb` (Q139) ; l'introduction annonce des pertes récupérables que ni le tableau 217 ni le §8.24.3.6.4 ne définissent.
- Fiche 8.25, tableau 229 (p. 891 à 895) : catégorie 800 (type 504 au tableau 111) ; « Val_util_max » 1,51 (air/eau, chauffage) et 0,7 ou 1,0 selon le cycle (air/eau, froid) contre 1,3 et 1,0 aux §8.25.3.1, 8.25.3.2.1 et 8.25.3.3.1 ; « Fonct_moteur » 0 par défaut avec « Conv. 2 » ; « Syst_thermo_ch/fr » numéroté 2 et 3 contre 1 et 2 dans le corps (Q140).
- Fiche 8.25, §8.25.3.1 (p. 897 et 898) : auxiliaires déclarés de l'unité extérieure « plafonnés » à une valeur « utile minimale » ; éq. (1368) `Pabs,pivot(ch) = ValPabs(4,4)` alors que le pivot de la PAC MG air/eau est en (4,2) (Q142). Corrections de `ηth,ecs` justifiée et déclarée écrites « augmentation » (Q144).
- Fiche 8.25, §8.25.3.4.6 (p. 910 et 911) : consommation (éq. 1392 et 1393) et rejet (éq. 1399 et 1400) écrits avec `Pgaz,LR` sans les irréversibilités, contrairement au GUE de l'éq. (1387) ; `Qrest,act` de l'éq. (1378) négatif à charge partielle (Q143).
- éq. (706) et (708) (fiche 6.6, p. 426-427) et tableau 67 (p. 425) : créneaux écrits `hleg(h) ∈ [hgd,deb ; hgd,fin]` (bornes incluses) avec `hgd` de 1 à 24 (p. 414), alors que la fiche 6.12 donne `hleg` de 0 à 23 au début du pas ; lu comme la case de scénario de 1 à 24, bornes incluses (Q236 point 2).
- tableau 65 (fiche 6.6, p. 412) : « pour le calcul du Bbio », phrase recopiée du §6.1.2 (la fiche 6.7 écrit « Cep ») ; la fiche 6.6 n'intervient pas dans le Bbio (Q234 point 5).
- éq. (694) (fiche 6.6, p. 422) : renvoie à un « Tableau 66 » pour les huit vitesses d'essai de l'extracteur (tableau non numéroté, p. 420), numéro déjà porté par la courbe de la bouche autoréglable (p. 418) (Q234 point 2).
- Fiche 8.24, figure 140 (p. 831) : le coefficient `Cnn` aval de la case 30/35 est imprimé 1,05, alors que la règle de complément de la p. 832 en fait une copie (× 1) de la valeur pivot (relecture du lot 2c).
- Fiche 8.24, figure 161 (p. 871) : les coefficients `Cnn` aval des cases 20,5/15 et 26/20,5 sont imprimés 1,1, alors que les règles de complément reportent la valeur critique ; p. 872, la phrase « Tamont ≥ critique, Taval ≤ critique » est inversée par rapport aux cases qu'elle décrit (relecture du lot 2c).
- Fiche 8.20 (p. 710 à 726, lot 2e) : la nomenclature (tableau 123) donne « Rpn » en % sur PCI (0 à 150) puis, plus bas (p. 713), en fraction (0 à 1,2) ; l'éq. (1120) compare `MAX(Rpn ; Rpint)/PCSI` à 1, alors que les éq. (1134) à (1136) emploient des pourcentages (lu en %, seuil 100 %, Q155) ; l'éq. (1123) définit « Qp0 » par défaut alors que l'éq. (1127) emploie « QPO30 » (lu comme la même grandeur, Q155) ; éq. (1139) « Pmax = Pngen (W) » avec `Pngen` en kW (lu `1000 · Pngen`) ; l'éq. (1145) écrit `Qfouact/ηeff_%` sans le facteur 100 de l'éq. (1136) ; le tableau 128 n'a pas de ligne « tube radiant sans ventilateur côté émission » ni « panneau radiant » ; l'éq. (1152) et le tableau 132 renvoient aux « chaudières bois (id_type = 400) », sans objet ici ; tableau 123 : « θfonct_min » a une valeur par défaut de 30 °C (colonne de la convention) alors que le §8.20.3.2.2 retient 20 °C (retenu : 20 °C, Q155).
- éq. (1249) (§8.23.3.3.6, p. 775) : PAC eau de nappe / air, indices du pivot « (2,4) » et de la ligne à 15 °C « (2,3) » au lieu de (4,2) et (3,2) (figure 122) ; « pivot (2,4) » au §8.23.3.3.6.2 ; titres des tableaux 166, 167 et 169 « eau glycolée / air » (Q150 points 1 et 7).
- §8.23.3.3.8 (p. 778-779) : PAC sol / eau, « Θaval = 20 » pour le pivot à 32,5 °C ; priorités aval de la figure 124 différentes du tableau 171 ; « Cnnam(−1;5 ; 4) » pour 1,5 °C (aussi tableaux 175, 176, 190, 191 et éq. 1254, 1255, 1261) (Q150 point 2, Q148).
- §8.23.3.4.5.1.1 (p. 790) : PAC sol / eau en ECS, « pivot (5,2) » pour (5,3) ; tableau 191 titré « Cnnam_COP » pour des coefficients de Pabs (Q150 point 3).
- §8.23.3.5.5 (p. 802-803) : refroidisseur eau / air, « IdFluide_aval = 1 : eau ; IdFluide_amont = 1 : air » inversés ; « EER_util(4,3) » de l'éq. (1271) pour (3,4) ; tableau 209 « Cnnav_Pabs » pour les coefficients amont (Q150 point 4).
- éq. (1273) (§8.23.3.5.6, p. 805) : refroidisseur eau de nappe / air, indices `(2,3)` et `(2,4)` pour (3,2) et (4,2) ; même section, « IdFluide_aval = 1 : eau ; IdFluide_amont = 1 : air » (Q150 point 5).
- éq. (1275) (§8.23.3.5.7, p. 807) : refroidisseur eau de nappe / eau, colonne du pivot « 4 » recopiée du refroidisseur eau / eau, au lieu de 2 (§8.23.3.5.7.2 : « (2,2) ») (Q150 point 6).
- tableaux 188, 211, 212, 214 et 215 : `Cnnam_Pabs(18,5 ; 8,5) = 0,90` (ECS sur nappe), `Cnnam_EER(22,5 ; 12,5) = 0,80` et `Cnnam_Pabs(17,5 ; 12,5) = 0,95` (refroidisseurs sur nappe), de sens contraire à leurs voisins ; retenus tels qu'imprimés (Q150 point 7).
- éq. (1427) et (1439) (§8.26.3.3.2, p. 923 et 925) : cas `Φrejet(h−1) = 0` non écrit ; `ΔθCond_CH` dans les équations, `ΔθEvap_CH` dans la nomenclature et la phrase des valeurs par défaut (Q146).
- éq. (1412) à (1414) et (1422) (p. 921-922) : débits fonction de `τcharge(h)`, inconnu avant l'appel du générateur ; `ε` indéfinie à débits nuls (Q147).
- éq. (1413) (p. 921) : `Qm_inter_reel` écrit avec `ρeau`, alors que l'éq. (1411) emploie `ρinter` pour le même débit-masse du circuit intermédiaire ; lu avec `ρinter` (Q147 point 3).
- éq. (1437) et (1438) (p. 925) : boucle d'eau décrite avec `θmin_sol`, `θmax_sol`, `idmois_mini_sol` au lieu de `θmin_boucle`, `θmax_boucle` (Q149).
- éq. (1447) (p. 927) : branche « vitesse variable » de la pompe du circuit intermédiaire écrite `Wpompes_captage = Ppompes_captage × max(τcharge ; 0,3)` ; lue `Wpompes_inter = Ppompes_inter × …` (Q151).
- éq. (1428) à (1431) (p. 924) : constante `Ce` absente du tableau 243 ; lue `Cpe` (Q152). Tableau 243 : `idamont-eau-type` de 1 à 2 dans les colonnes Min/Max, alors que la liste compte 5 types.
- Fiche 8.21 (p. 727 à 743, lot 2e) : la nomenclature code « idraccord_gnr » 0 = permanent, 1 = avec isolement (tableau 135), l'éq. (1220) écrit « idraccordgnr = 1 : raccordement permanent » (Q157) ; les facteurs d'interpolation des éq. (1207), (1211) et (1215) ne valent ni 0 ni 1 aux bornes des modes, contrairement à la figure 116 (Q158) ; « Pveilleuse » (éq. 1204) n'est pas défini ; le tableau 138 ne donne de valeurs utiles maximales que pour `ind_condensation = 0` ; l'éq. (1187) emploie les rendements du point avec appoint même sans appoint intégré ; « idfougen » a pour maximum 6 (tableau 135) pour trois valeurs listées ; « type_combustible » va de 0 à 5 dans la nomenclature et de 1 à 6 au tableau 136.
- Fiche 8.23, triple service (p. 813, 814 et 819, lot 2g) : « iECS_seule » (vrai dès la fin de la saison de chauffage, fiche 8.17, éq. 1055) ferait compter `Waux,0` en ECS et en refroidissement pendant la saison de refroidissement, alors que le texte veut « un seul mode » ; la fonction « 6 : chauffage, refroidissement et ECS » de la nomenclature de la fiche 8.23 manque à la liste de la fiche 8.17 (1 à 5) (Q250).
- Fiche 8.33 (p. 1012 et 1017, lot 2g) : tableau 265, ligne « θvs_chem » (0 / 0,6 / 1,7 / 2,8 K, valeurs de la classe B2 du tableau 82) contraire à la ligne « Classe_Variation_Spatiale_Chaud : Classe C » du même tableau, aux hypothèses et au tableau 260 (0,4 / 1,2 / 2,0 / 2,8 K) ; « Couple_Regulateur_Emetteur = 0 : ne permettant pas un arrêt total » avec 1,8 K, valeur du tableau 87 pour un couple qui permet l'arrêt total, comme le disent les hypothèses (Q251).
