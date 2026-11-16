// SPDX-License-Identifier: GPL-3.0-or-later
// Fichier généré par `cargo run -p xtask -- zones-climatiques` : ne pas modifier à la main.
// Source : arrêté du 26 octobre 2010, annexe I
// (docs/reference/texte/arrete-26-octobre-2010-rt2012.txt).

use super::ZoneClimatique;

/// Zone climatique de chaque département, triée par code de département.
pub const ZONE_PAR_DEPARTEMENT: [(&str, ZoneClimatique); 96] = [
    ("01", ZoneClimatique::H1c), // Ain
    ("02", ZoneClimatique::H1a), // Aisne
    ("03", ZoneClimatique::H1c), // Allier
    ("04", ZoneClimatique::H2d), // Alpes-de-Haute-Provence
    ("05", ZoneClimatique::H1c), // Hautes-Alpes
    ("06", ZoneClimatique::H3),  // Alpes-Maritimes
    ("07", ZoneClimatique::H2d), // Ardèche
    ("08", ZoneClimatique::H1b), // Ardennes
    ("09", ZoneClimatique::H2c), // Ariège
    ("10", ZoneClimatique::H1b), // Aube
    ("11", ZoneClimatique::H3),  // Aude
    ("12", ZoneClimatique::H2c), // Aveyron
    ("13", ZoneClimatique::H3),  // Bouches-du-Rhône
    ("14", ZoneClimatique::H1a), // Calvados
    ("15", ZoneClimatique::H1c), // Cantal
    ("16", ZoneClimatique::H2b), // Charente
    ("17", ZoneClimatique::H2b), // Charente-Maritime
    ("18", ZoneClimatique::H2b), // Cher
    ("19", ZoneClimatique::H1c), // Corrèze
    ("21", ZoneClimatique::H1c), // Côtes-d'Or
    ("22", ZoneClimatique::H2a), // Côtes-d'Armor
    ("23", ZoneClimatique::H1c), // Creuse
    ("24", ZoneClimatique::H2c), // Dordogne
    ("25", ZoneClimatique::H1c), // Doubs
    ("26", ZoneClimatique::H2d), // Drôme
    ("27", ZoneClimatique::H1a), // Eure
    ("28", ZoneClimatique::H1a), // Eure-et-Loir
    ("29", ZoneClimatique::H2a), // Finistère
    ("2A", ZoneClimatique::H3),  // Corse-du-Sud
    ("2B", ZoneClimatique::H3),  // Haute-Corse
    ("30", ZoneClimatique::H3),  // Gard
    ("31", ZoneClimatique::H2c), // Haute-Garonne
    ("32", ZoneClimatique::H2c), // Gers
    ("33", ZoneClimatique::H2c), // Gironde
    ("34", ZoneClimatique::H3),  // Hérault
    ("35", ZoneClimatique::H2a), // Ille-et-Vilaine
    ("36", ZoneClimatique::H2b), // Indre
    ("37", ZoneClimatique::H2b), // Indre-et-Loire
    ("38", ZoneClimatique::H1c), // Isère
    ("39", ZoneClimatique::H1c), // Jura
    ("40", ZoneClimatique::H2c), // Landes
    ("41", ZoneClimatique::H2b), // Loir-et-Cher
    ("42", ZoneClimatique::H1c), // Loire
    ("43", ZoneClimatique::H1c), // Haute-Loire
    ("44", ZoneClimatique::H2b), // Loire-Atlantique
    ("45", ZoneClimatique::H1b), // Loiret
    ("46", ZoneClimatique::H2c), // Lot
    ("47", ZoneClimatique::H2c), // Lot-et-Garonne
    ("48", ZoneClimatique::H2d), // Lozère
    ("49", ZoneClimatique::H2b), // Maine-et-Loire
    ("50", ZoneClimatique::H2a), // Manche
    ("51", ZoneClimatique::H1b), // Marne
    ("52", ZoneClimatique::H1b), // Haute-Marne
    ("53", ZoneClimatique::H2b), // Mayenne
    ("54", ZoneClimatique::H1b), // Meurthe-et-Moselle
    ("55", ZoneClimatique::H1b), // Meuse
    ("56", ZoneClimatique::H2a), // Morbihan
    ("57", ZoneClimatique::H1b), // Moselle
    ("58", ZoneClimatique::H1b), // Nièvre
    ("59", ZoneClimatique::H1a), // Nord
    ("60", ZoneClimatique::H1a), // Oise
    ("61", ZoneClimatique::H1a), // Orne
    ("62", ZoneClimatique::H1a), // Pas-de-Calais
    ("63", ZoneClimatique::H1c), // Puy-de Dôme
    ("64", ZoneClimatique::H2c), // Pyrénées-Atlantiques
    ("65", ZoneClimatique::H2c), // Hautes-Pyrénées
    ("66", ZoneClimatique::H3),  // Pyrénées-Orientales
    ("67", ZoneClimatique::H1b), // Bas-Rhin
    ("68", ZoneClimatique::H1b), // Haut-Rhin
    ("69", ZoneClimatique::H1c), // Rhône
    ("70", ZoneClimatique::H1b), // Haute-Saône
    ("71", ZoneClimatique::H1c), // Saône-et-Loire
    ("72", ZoneClimatique::H2b), // Sarthe
    ("73", ZoneClimatique::H1c), // Savoie
    ("74", ZoneClimatique::H1c), // Haute-Savoie
    ("75", ZoneClimatique::H1a), // Paris
    ("76", ZoneClimatique::H1a), // Seine-Maritime
    ("77", ZoneClimatique::H1a), // Seine-et-Marne
    ("78", ZoneClimatique::H1a), // Yvelines
    ("79", ZoneClimatique::H2b), // Deux-Sèvres
    ("80", ZoneClimatique::H1a), // Somme
    ("81", ZoneClimatique::H2c), // Tarn
    ("82", ZoneClimatique::H2c), // Tarn-et-Garonne
    ("83", ZoneClimatique::H3),  // Var
    ("84", ZoneClimatique::H2d), // Vaucluse
    ("85", ZoneClimatique::H2b), // Vendée
    ("86", ZoneClimatique::H2b), // Vienne
    ("87", ZoneClimatique::H1c), // Haute-Vienne
    ("88", ZoneClimatique::H1b), // Vosges
    ("89", ZoneClimatique::H1b), // Yonne
    ("90", ZoneClimatique::H1b), // Territoire de Belfort
    ("91", ZoneClimatique::H1a), // Essonne
    ("92", ZoneClimatique::H1a), // Hauts-de-Seine
    ("93", ZoneClimatique::H1a), // Seine-Saint-Denis
    ("94", ZoneClimatique::H1a), // Val-de-Marne
    ("95", ZoneClimatique::H1a), // Val-d'Oise
];
