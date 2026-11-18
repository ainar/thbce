# État du lot 1-ven (ventilation) — fichier de reprise

Branche `phase-1-ventilation` (depuis main 6db2136). Questions Q90 à Q99. Fragment `changelog.d/1-ventilation.md`.
Ce fichier n'est pas destiné à être fusionné tel quel (à retirer avant fusion si l'orchestrateur le souhaite).

## Périmètre retenu (après lecture des p. 342 à 395, 326 à 341, 567 à 568, 610 à 618)

Tranche livrée dans ce lot :
1. Double flux (type 1) hors habitation : débits ZN occ/inocc (§6.2.3.1.2, éq. 6.2-391 à 398, 406 à 418), puissances occ/inocc (éq. 6.3-458, 459), Tsech_LIM du tableau 62 (plaques 0 °C, rotatif −5 °C en non résidentiel).
2. Échangeur rotatif : `Pech` (éq. 497), consommation dans éq. 570.
3. CTA types 2, 3, 4 en zone neutre (§6.3.3.4) : régime des débits (éq. 421 à 425, 444 à 448), recyclage et air neuf (433 à 441), air rejeté (454 à 456), ventilateurs (460 à 465), boîte de mélange (515, 516), échangeur, antigel.
4. Batteries de préchauffage et de prérefroidissement (types 1 et 2, éq. 522 à 525, 539 à 548), énergie vers une génération par un réseau de CTA fictif (fiche 8.11, éq. 937, 939, 945 à 948, 951).
5. Relevé horaire de la consommation des ventilateurs pour les systèmes non résidentiels à double flux.

Refus ciblés gardés (code 3), documentés :
- CTA types 3 et 4 avec batteries chaude/froide régulées sur la charge (mode actif §6.3.3.5, couplage avec l'émission 8.1 et éq. 940) ;
- humidification d'hiver, rafraîchissement évaporatif, surventilation mécanique (6.12), puits (6.8 à 6.11), ventilation naturelle/hybride (6.6), aération (6.7) ;
- réseau de CTA hydraulique (8.11, idtype = 1).

## Fait
- (rien encore)

## En cours
- saisie et validation

## Reste
- voir périmètre

## Dernier SHA poussé
- aucun
