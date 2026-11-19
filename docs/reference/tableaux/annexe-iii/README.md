<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Tableaux 107 à 266 de l'annexe III (fiches 8.14 à 8.33, génération)

Transcription en CSV des **159 tableaux** de l'annexe III de l'arrêté du 4 août 2021 numérotés 107 à 266 (le tableau 134 n'existe pas : voir `../../index/anomalies.md`). Ils décrivent les nomenclatures, les listes de technologies, les ordres de saisie des températures, les coefficients de correction et les valeurs par défaut des fiches de la génération (8.14 à 8.33).

**Le PDF fait foi** (`../../arrete/annexeiii_arrete_4_aout_2021_complet_compresse.pdf`, pagination « n/1854 »). Ces fichiers sont une aide à la lecture et à l'écriture du code, pas une source concurrente : en cas de doute, lire l'image de la page (`cargo run -p xtask -- page --tableau <n>`).

## Provenance

Texte public du Journal officiel (arrêté du 4 août 2021, annexe III), diffusé sous Licence Ouverte 2.0 (etalab) par le site RT-RE bâtiment. Les CSV en reprennent le contenu sous la même licence ; ils ne portent pas d'en-tête SPDX, car un en-tête fausserait le format.

## Méthode

1. **Transcription sur image** de chaque tableau (pages rendues à 150-200 dpi, lecture par un modèle, un journal des points douteux par lot).
2. **Comparaison avec la couche vectorielle du PDF**, exacte sur les valeurs mais parfois fausse sur la structure : (a) extraction `pdfplumber` des tableaux de chaque page, comparée **par multiensemble des valeurs numériques** (virgule et point confondus, zéros finals ignorés) ; (b) pour les tableaux que `pdfplumber` ne détecte pas, comparaison des valeurs et des jetons de noms avec `pdftotext` sur les pages du tableau. Aucun modèle n'intervient dans cette étape.
3. **Arbitrage sur l'image** de chaque écart et de chaque point douteux des journaux, puis correction. Pour les tableaux sans extraction exploitable, contrôle sur image de pages échantillonnées (p. 634, 916, 944, 734-735, 763, 1010, etc.), en plus de la comparaison au texte vectoriel.

Résultat : **aucune erreur sur une valeur numérique** n'a été trouvée dans les CSV ; les écarts bruts (environ 1 400 valeurs dans les deux sens) venaient de l'extraction automatique (matrices de performance des figures, notes en exposant collées aux valeurs, cellules fusionnées, numéros de rang). Les corrections portent sur le texte, les symboles et la structure : **61 cellules dans 22 tableaux**, plus la suppression de colonnes de rang ajoutées dans 23 tableaux (liste ci-dessous).

## Convention du CSV

- UTF-8, fins de ligne LF, séparateur de cellules `;`. Une cellule qui contient `;`, `"` ou un saut de ligne est entre guillemets (RFC 4180) : le `;` imprimé dans un texte (`Q_cef(1;10)`, `-7°C, 2°C; 7°C, 20°C`) est conservé.
- Les lignes commençant par `#` sont des **commentaires** à ignorer par un lecteur : la première donne le numéro, les pages, la fiche et le titre imprimé ; les suivantes signalent la mise en forme et les cas particuliers. Un lecteur CSV doit les écarter avant d'analyser.
- Valeurs telles qu'imprimées : virgule ou point décimal selon la cellule, `+∞`, `-∞`, `-`, `…` et les unités sans normalisation. Une cellule vide est une cellule sans valeur imprimée.
- Notation texte des indices et exposants : `_` pour un indice, `^` pour un exposant (`η_eff_ch^gnr(h)`, `(P_th;chp100+sup0)^0,4`) ; les virgules et points-virgules internes aux indices sont ceux de l'impression.
- Un renvoi de note en exposant est noté `(note n)` (tableau 144). Une cellule fusionnée est répétée sur chacune des lignes ou colonnes qu'elle couvre, sauf mention en commentaire.
- Nomenclatures : colonnes `section;groupe;nom;description;unite;min;max;conv` (ou `Section;Nom;…`), le groupe étant l'étiquette verticale imprimée ; les colonnes Min/Max/Conv absentes à l'impression restent vides.

## Écarts et ajouts de mise en forme (par rapport à l'impression)

- **En-têtes ajoutés** quand rien n'est imprimé : `Côté;Coefficient;Valeur` (tableaux 151, 152, 154, 155, 157, 158, 160, 161, 163, 164, 166, 167, 169, 170, 172, 173, 175, matrices de coefficients mises à plat, une ligne par coefficient ; 154 et 151 ajoutent la colonne `Puissance nominale à 7°C`), `Grandeur;Coefficient;Valeur` (232, 233, 235, 236 : blocs 2x2 mis à plat), `groupe;libellé;valeur_1;valeur_2` (127, 128), `Type de chaudière` (120), `Grandeur;Valeur par défaut` (140), `Grandeur;Formule imprimée` (139 : le tableau est un jeu de formules 1175 à 1178, pas une grille).
- **Listes côte à côte** (145, 147, 150, 153, 156, 159) : deux colonnes dans l'ordre d'impression, une ligne par rang ; les cellules sont reproduites avec leur préfixe imprimé (`1 : 32.5°C`). **La colonne « Rang » / « Code » ajoutée par la transcription a été retirée** dans les tableaux d'ordre de saisie 150, 153, 156, 159, 162, 165, 168, 171, 174, 177, 180, 183, 186, 189 (le rang est le préfixe imprimé) et 222 à 226, 231, 234, 237, 240 (aucun rang imprimé : le rang est le numéro de ligne).
- **En-têtes multi-niveaux aplatis** (118, 119, 126, 137, 138, 140) ; cellules fusionnées répétées ; texte de lignes de section en ligne seule ou en commentaire (229, 253, 254, 259).
- **Tableau 140** : formules en texte avec `^` pour l'exposant (`1000 . 4,25% . P_th;chp100+sup0 / (P_th;chp100+sup0)^0,4`), « Si id_app_inte = n : » conservé dans la cellule ; la grandeur est répétée sur ses deux lignes.
- **Tableau 257** : imprimé deux fois (p. 994-995 sans en-tête, puis p. 995-997 avec légende) ; la copie légendée est transcrite, les valeurs des deux copies concordent.
- **Matrices de performance (figures 117 à 120, 131 à 135, 138, 152, 155, 158, 161, 164, 167, 169-171…)** : ce sont des figures, non transcrites ; seuls les tableaux numérotés le sont.
- **Matrices de consommations (113, 133, 142, 228, 249, 252, 263)** : cellules `…` reproduites, `Q_cef(1;10)` entre guillemets.

## Points tranchés sur l'image ou la couche texte

- ρe : **1000** au tableau 253, **1006** au tableau 254, tels qu'imprimés (incohérence du texte, non corrigée).
- Tableau 259, symbole d'efficacité du générateur : le PDF imprime un **carré vide** (glyphe absent) suivi de l'indice `eff` ; transcrit `□eff`. Au tableau 217 (p. 825), le symbole est bien `η_eff(h)`.
- Tableau 236 : `Cnnam_ηth,ecs(5, 7)` (et non (2, 7)) et `Cnnam_Paux,uex(-15, -7)` sont imprimés ainsi (confirmé par la couche texte).
- Titres : 158 (« air extérieur / air recyclé » sous des étapes « air extrait / air neuf »), 169 (« eau glycolée / air ») et 170 (« eau de boucle/ air »), 119 (« gaz, fioul et bis ») sont imprimés tels quels.
- Tableaux 243 et 250 : `P_vent_gaine` en `°C` (243), `Whep` (250), `ωext`, `Δθ_…`, `τcharge` : symboles et unités reproduits comme imprimés ; la lettre `τ` (taux de charge) avait été transcrite `T` dans 110, 112, 229, 243, 244, 246 et corrigée.
- Renvois rognés à l'impression : « Voir Tableau 121 et Tableau » (114, p. 696) et « Voir Tableau 129 et Tableau » (123, p. 714) : la fin du numéro est coupée dans le PDF, laissée telle quelle.
- Tableau 110 : groupe « Gestion sans priorité » (étiquette verticale en fragments, p. 661) ; `Q_cef(po;en)^gnr(h)` imprimé deux fois ; `Q_cef(poste,en)` avec virgule imprimée.
- **« ; » rétablis** : dans 107 à 142 les `;` imprimés avaient été remplacés par `-` : `Q_cef(po;en)` (107 à 110, 112 à 114, 123, 133, 135, 141, 142), `P_th;chp…`, `η_th;chp…`, `η_el;chp…` (135, 138 à 140).
- Tableau 258 : la valeur `1°C` de `Δθop_3_br_man` est une cellule fusionnée jour/nuit, répétée ; l'annotation de la transcription a été retirée.
- Tableau 135 : légende « Tableau 135 » p. 731, alors que le texte p. 727 renvoie à « Tableau 134 ».

## Points à risque restants

- Tableaux sans extraction automatique (107 à 110, 112, 114, 119, 123, 135, 141, 143, 217, 229, 243, 244, 246, 250, 253, 254, 256, 259) : valeurs numériques et noms confirmés par le texte vectoriel, **mais l'attribution ligne à ligne des descriptions, des unités et des groupes n'a été contrôlée sur image que par échantillon** (p. 634, 916 sans erreur). Les glyphes absents de la couche texte (lettres grecques, `Δ`, `ω`) reposent sur la lecture de l'image.
- Tableaux 253, 254, 256, 259 : nomenclatures longues avec en-têtes de sous-section en ligne seule.
