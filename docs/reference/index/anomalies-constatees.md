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
- éq. (706) et (708) (fiche 6.6, p. 426-427) et tableau 67 (p. 425) : créneaux écrits `hleg(h) ∈ [hgd,deb ; hgd,fin]` (bornes incluses) avec `hgd` de 1 à 24 (p. 414), alors que la fiche 6.12 donne `hleg` de 0 à 23 au début du pas ; lu comme la case de scénario de 1 à 24, bornes incluses (Q236 point 2).
- tableau 65 (fiche 6.6, p. 412) : « pour le calcul du Bbio », phrase recopiée du §6.1.2 (la fiche 6.7 écrit « Cep ») ; la fiche 6.6 n'intervient pas dans le Bbio (Q234 point 5).
- éq. (694) (fiche 6.6, p. 422) : renvoie à un « Tableau 66 » pour les huit vitesses d'essai de l'extracteur (tableau non numéroté, p. 420), numéro déjà porté par la courbe de la bouche autoréglable (p. 418) (Q234 point 2).
- éq. (1249) (§8.23.3.3.6, p. 775) : PAC eau de nappe / air, indices du pivot « (2,4) » et de la ligne à 15 °C « (2,3) » au lieu de (4,2) et (3,2) (figure 122) ; « pivot (2,4) » au §8.23.3.3.6.2 ; titres des tableaux 166, 167 et 169 « eau glycolée / air » (Q150 points 1 et 7).
- §8.23.3.3.8 (p. 778-779) : PAC sol / eau, « Θaval = 20 » pour le pivot à 32,5 °C ; priorités aval de la figure 124 différentes du tableau 171 ; « Cnnam(−1;5 ; 4) » pour 1,5 °C (aussi tableaux 175, 176, 190, 191 et éq. 1254, 1255, 1261) (Q150 point 2, Q148).
- §8.23.3.4.5.1.1 (p. 790) : PAC sol / eau en ECS, « pivot (5,2) » pour (5,3) ; tableau 191 titré « Cnnam_COP » pour des coefficients de Pabs (Q150 point 3).
- §8.23.3.5.5 (p. 802-803) : refroidisseur eau / air, « IdFluide_aval = 1 : eau ; IdFluide_amont = 1 : air » inversés ; « EER_util(4,3) » de l'éq. (1271) pour (3,4) ; tableau 209 « Cnnav_Pabs » pour les coefficients amont (Q150 point 4).
- éq. (1273) (§8.23.3.5.6, p. 805) : refroidisseur eau de nappe / air, indices `(2,3)` et `(2,4)` pour (3,2) et (4,2) ; même section, « IdFluide_aval = 1 : eau ; IdFluide_amont = 1 : air » (Q150 point 5).
- éq. (1275) (§8.23.3.5.7, p. 807) : refroidisseur eau de nappe / eau, colonne du pivot « 4 » recopiée du refroidisseur eau / eau, au lieu de 2 (§8.23.3.5.7.2 : « (2,2) ») (Q150 point 6).
- tableaux 188, 211, 212, 214 et 215 : `Cnnam_Pabs(18,5 ; 8,5) = 0,90` (ECS sur nappe), `Cnnam_EER(22,5 ; 12,5) = 0,80` et `Cnnam_Pabs(17,5 ; 12,5) = 0,95` (refroidisseurs sur nappe), de sens contraire à leurs voisins ; retenus tels qu'imprimés (Q150 point 7).
- éq. (1427) et (1439) (§8.26.3.3.2, p. 923 et 925) : cas `Φrejet(h−1) = 0` non écrit ; `ΔθCond_CH` dans les équations, `ΔθEvap_CH` dans la nomenclature et la phrase des valeurs par défaut (Q146).
- éq. (1412) à (1414) et (1422) (p. 921-922) : débits fonction de `τcharge(h)`, inconnu avant l'appel du générateur ; `ε` indéfinie à débits nuls (Q147).
- éq. (1437) et (1438) (p. 925) : boucle d'eau décrite avec `θmin_sol`, `θmax_sol`, `idmois_mini_sol` au lieu de `θmin_boucle`, `θmax_boucle` (Q149).
- éq. (1447) (p. 927) : branche « vitesse variable » de la pompe du circuit intermédiaire écrite `Wpompes_captage = Ppompes_captage × max(τcharge ; 0,3)` ; lue `Wpompes_inter = Ppompes_inter × …` (Q151).
- éq. (1428) à (1431) (p. 924) : constante `Ce` absente du tableau 243 ; lue `Cpe` (Q152). Tableau 243 : `idamont-eau-type` de 1 à 2 dans les colonnes Min/Max, alors que la liste compte 5 types.
