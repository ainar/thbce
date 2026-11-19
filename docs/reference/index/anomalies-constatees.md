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
- éq. (689) (fiche 6.6) : le rapport `vvent,réf / vvent,c` est l'inverse de celui de l'éq. (688) ; lu `v8 × vvent,c / Vvent,réf` (Q234 point 1).
- éq. (701) et (704) (fiche 6.6) : `Aboucheeq`, défini pour la bouche fixe seule, intervient aussi avec une bouche autoréglable ; part des fuites lue `qfuites / (qbouche + qfuites)` (Q235 point 2).
- éq. (707) (fiche 6.6) : pas d'équation du débit soufflé spécifique en refoulement ; lu par analogie (Q235 point 1).
