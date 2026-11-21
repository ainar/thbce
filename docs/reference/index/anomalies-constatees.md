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
