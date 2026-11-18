// SPDX-License-Identifier: GPL-3.0-or-later
// Fichier généré par `cargo run -p xtask -- typescript` depuis les JSON Schema d'entrée
// et de résultats ; ne pas modifier à la main.

/**
 * Affectation d'une baie à un local pour l'éclairement naturel (§5.10.3.2, « type de baie »).
 *
 * Seule compte la distinction entre local de grand volume et local de volume normal ; pour un
 * grand volume, il faut savoir si la baie est un éclairant de toiture réparti de façon
 * uniforme (distance entre deux éclairants inférieure à la hauteur du local, distance des
 * éclairants périphériques à la périphérie inférieure à la moitié de cette hauteur).
 */
export type AffectationEclairage = "volume_normal" | "grand_volume" | "grand_volume_toiture_uniforme";

/** « Alim_Chaudière_Bois » (§8.19.2, tableau 119). */
export type AlimentationBoisSaisie = "manuel_tirage_naturel" | "manuel_air_pulse" | "automatique_tirage_naturel" | "automatique_air_pulse";

/** « App_ecs » (§9.5.3, tableau 275). */
export type AppareilsEcs = "douche_seule" | "baignoire_sabot" | "baignoire_standard" | "grande_baignoire";

/** Générateur d'appoint intégré à un ballon (§9.14.3.1, « Type_prod_stockage = 1 »). */
export interface AppointBallon {
  /** « Δθap » : hystérésis, en K. Absente : 2 K. */
  delta_theta_k?: number | null;
  /**
   * « faux » : fraction du volume du ballon chauffée par l'appoint, strictement entre 0 et
   * 1. Absente : 0,5, valeur par défaut du §9.9.3.1, signalée dans les résultats.
   */
  f_aux?: number | null;
  /** Identifiant d'un générateur à effet Joule de la génération assurant l'ECS. */
  generateur: string;
  /** « typegest_ap ». */
  programmation: ProgrammationBallon;
  /**
   * « zap » : numéro de la zone la plus basse chauffée par l'appoint, de 1 à 4 (3 pour un
   * appoint à mi-hauteur).
   */
  z_ap: number;
  /** « zreg_ap » : numéro de la zone de la sonde de régulation de l'appoint. */
  z_reg: number;
}

/** Appoint d'un chauffe-eau solaire (« Type_prod_stockage », §9.14.2). */
export type AppointCesi = "aucun" | {
  integre: AppointBallon;
} | "separe";

/** Appoint intégré d'un système solaire combiné. */
export interface AppointSsc {
  /** « Δθap » : hystérésis, en K. Absente : 2 K. */
  delta_theta_k?: number | null;
  /**
   * « faux » : fraction du volume chauffée par l'appoint, strictement entre 0 et 1.
   * Absente : 0,5 (§9.9.3.1), signalée.
   */
  f_aux?: number | null;
  /**
   * Identifiant d'un générateur de la génération : générateur à effet Joule de fonction
   * `ecs` avec un appoint de chauffage indépendant ; générateur assurant le chauffage et
   * l'ECS (chaudière, pompe à chaleur double service) avec un appoint raccordé.
   */
  generateur: string;
  /** « typegest_ap ». */
  programmation: ProgrammationBallon;
}

/**
 * Th-BCE 2020 §10.1.2, tableau 309 — une cabine d'ascenseur : paramètres intrinsèques
 * (`TechMac`, `Q`, `V`, `Cp`, `ScVeille` et ses cinq paramètres de veille) et d'intégration
 * (`H`, `netage`, zones desservies `C`).
 */
export interface Ascenseur {
  /**
   * « Cp » : équilibrage de la machine (−1,2 à 0,5). Absent : valeur du tableau 314,
   * signalée dans les résultats. Pour un ascenseur hydraulique, −1,2 est obligatoire.
   */
  cp?: number | null;
  /** « dP1 » : premier décrément de la puissance de veille, en W. */
  dp1_w?: number | null;
  /** « dP2 » : second décrément de la puissance de veille, en W. */
  dp2_w?: number | null;
  /** « H » : course de l'ascenseur, sous-sols compris, en m. */
  h_m: number;
  /** Identifiant de la cabine. */
  id: string;
  /**
   * « netage » : nombre d'étages desservis, point bas non compté (au moins 2) : 4 pour un
   * R+4 sans sous-sol, 5 pour un R+4 avec un niveau de sous-sol.
   */
  netage: number;
  /**
   * « Pti » : puissance totale de veille cabine immobile, en W, mesurée après fermeture des
   * portes. Absent : valeur par défaut de l'éq. (2274).
   */
  pti_w?: number | null;
  /** « Q » : charge utile de la cabine au sens de l'EN 81-1, en kg (au moins 75). */
  q_kg: number;
  /**
   * « ScVeille » : le scénario de mise en veille de la cabine est connu. Faux par défaut :
   * la cabine n'a alors pas de veille (`dP1 = dP2 = 0`). Vrai : `dp1_w`, `t1_s`, `dp2_w` et
   * `t2_s` sont employés, avec les valeurs par défaut du §10.1.3.3 pour ceux qui manquent.
   */
  sc_veille?: boolean;
  /** « T1 » : durée avant le premier décrément, en s. */
  t1_s?: number | null;
  /** « T2 » : durée avant le second décrément à partir du premier, en s. */
  t2_s?: number | null;
  /** « TechMac » : typologie. */
  tech_mac: TechMac;
  /** « V » : vitesse nominale au sens de l'EN 81-1, en m/s (au plus 2,5). */
  v_m_s: number;
  /** « C » : identifiants des zones desservies (au moins une, de logement collectif). */
  zones_desservies: string[];
}

/** Th-BCE 2020 §5.6.3.2.1, figure 21 — comportement autoréglable d'une entrée d'air. */
export interface AutoreglageEntreesAir {
  /** « ΔP1 » : pression du premier changement de comportement, en Pa. */
  delta_p1_pa: number;
  /** « ΔP2 » : pression du second changement de comportement, en Pa. */
  delta_p2_pa: number;
  /** « M′ » : somme des modules du défaut d'autorégulabilité, en m³/h. */
  m_prime_m3_h: number;
}

/**
 * Auxiliaires d'une chaudière, en W ; chaque valeur absente prend sa valeur par défaut
 * (éq. 1075 à 1077), signalée dans les résultats.
 */
export interface AuxiliairesChaudiere {
  /** « Waux_int » : puissance électrique absorbée à charge intermédiaire. */
  w_aux_int_w?: number | null;
  /** « Waux_nom » : puissance électrique absorbée à pleine charge. */
  w_aux_nom_w?: number | null;
  /** « Wveille » : puissance électrique absorbée à charge nulle. */
  w_veille_w?: number | null;
}

/** Baie vitrée d'un groupe : paramètres d'intégration (§5.10.2). */
export interface Baie {
  /** « Ab » : surface de la baie, en m². */
  a_b_m2: number;
  /**
   * Affectation de la baie à un local de grand volume ou de volume normal, pour l'éclairement
   * naturel (§5.10.3.2). Absente : volume normal. Un groupe qui a des baies de grand volume
   * doit avoir un local de grand volume (salle de sport, aire de production, aire de vente de
   * plus de 300 m²).
   */
  affectation_eclairage?: AffectationEclairage | null;
  /**
   * Détermination détaillée de la classe d'exposition au bruit (fiche 5.11) : catégorie et
   * distance des infrastructures, vue, plan d'exposition au bruit. Facultative.
   */
  bruit?: BruitBaie | null;
  /**
   * Identifiant de l'espace tampon sur lequel donne la baie (§5.10.3, « IdET »). Une baie
   * ne peut donner que sur un espace tampon solarisé. Absent : la baie donne sur
   * l'extérieur.
   */
  espace_tampon?: string | null;
  /**
   * « expo_BR » : classe d'exposition au bruit de la baie (§5.13.2). Requise si le type de
   * baie a un ouvrant, sauf si `bruit` permet de la déduire ; si les deux sont saisies, la
   * validation vérifie leur cohérence.
   */
  expo_br?: ExpoBr | null;
  /** Orientation, inclinaison et masques. */
  exposition: Exposition;
  /** Identifiant de la baie. */
  id: string;
  /** Référence à un [`BaieType`] du catalogue. */
  type_baie: string;
}

/**
 * Th-BCE 2020 §5.10.2 — paramètres intrinsèques d'une baie vitrée.
 *
 * Les matrices de facteurs des stores vénitiens (`M_Sw_BC`, `M_Sw_E`…) seront ajoutées avec la
 * fiche 5.10 (lot 4).
 */
export interface BaieType {
  /**
   * Facteurs de la baie avec protection mobile de type volet ou store enroulable en place
   * (indice « ap »). Requis si `protection_mobile` est renseignée.
   */
  avec_protection?: FacteursBaie | null;
  /** Identifiant du type de baie. */
  id: string;
  /**
   * Ouvrant de la baie (§5.13.2, « Est_Ouvrable »). Absent : baie non ouvrable, qui ne
   * contribue pas à la surventilation naturelle. La gestion de l'ouverture est manuelle ; la
   * gestion automatique n'est pas prise en charge dans cette version.
   */
  ouvrant?: Ouvrant | null;
  /**
   * Première protection mobile de la baie et sa gestion (§5.9.2). Absente : baie sans
   * protection mobile (`Type-PM = 0`).
   */
  protection_mobile?: ProtectionMobile | null;
  /** Facteurs de la baie sans protection mobile en place (indice « sp »). */
  sans_protection: FacteursBaie;
  /** « Uap,horiz,b » : U de la baie horizontale avec protection mobile en place. */
  u_ap_horiz_w_m2k?: number | null;
  /** « Uap,vert,b » : U de la baie verticale avec protection mobile en place. */
  u_ap_vert_w_m2k?: number | null;
  /** « Usp,horiz,b » : U de la baie horizontale sans protection mobile en place. */
  u_sp_horiz_w_m2k?: number | null;
  /** « Usp,vert,b » : U de la baie verticale sans protection mobile en place, en W/(m².K). */
  u_sp_vert_w_m2k?: number | null;
}

/** Th-BCE 2020 §9.9.2, tableau 281 — ballon de stockage. */
export interface BallonSaisi {
  /** « (UA)s » et « Statut_UA » : coefficient de pertes thermiques du ballon. */
  pertes: PertesBallon;
  /** « θmax » : température maximale du ballon, en °C (de 0 à 100). */
  theta_max_c: number;
  /** « Vtot » : volume du ballon, en L. */
  v_tot_l: number;
}

/** Niveau « bâtiment » (Th-BCE 2020 §5.1) : niveau où s'expriment Bbio, Cep et Cep,nr. */
export interface Batiment {
  /**
   * Ascenseurs du bâtiment (§10.1), une entrée par cabine ; logement collectif seulement.
   * L'éclairage conventionnel des parties communes (§7.2) ne demande aucune saisie : il
   * s'applique à toute zone de logement collectif.
   */
  ascenseurs?: Ascenseur[];
  /** Réseaux de distribution d'ECS intergroupes (§9.8). */
  distributions_ecs_intergroupes?: DistributionEcsIntergroupe[];
  /** Réseaux de distribution intergroupes de chaud et de froid (§8.9 et §8.10). */
  distributions_intergroupes?: DistributionIntergroupes[];
  /**
   * Espaces tampons du bâtiment (§5.2 à 5.4) : locaux non chauffés en contact avec
   * l'enveloppe d'un ou de plusieurs groupes. Les parois opaques, ponts thermiques, baies et
   * entrées d'air qui donnent sur un espace tampon le désignent par son identifiant
   * (`espace_tampon`).
   */
  espaces_tampons?: EspaceTampon[];
  /**
   * Données de la fixation des exigences (annexe de l'article R. 172-4 du code de la
   * construction et de l'habitation, chapitres II et III). Facultatives : sans elles, les
   * indicateurs sont calculés mais ni les exigences ni la conformité.
   */
  exigences?: ParametresExigences | null;
  /**
   * Générations du bâtiment (§8.4 et §8.14). Requises dès qu'un réseau intergroupes y est
   * rattaché.
   */
  generations?: Generation[];
  /** Identifiant du bâtiment. */
  id: string;
  /** Installations photovoltaïques intégrées au bâtiment (§12.1 et §12.2). Facultatif. */
  installations_pv?: InstallationPv[];
  /** Parcs de stationnement associés au bâtiment (§10.3 et §10.4). */
  parkings?: Parking[];
  /** Zones : parties de bâtiment de même usage, donc de mêmes scénarios (§5.1.3). */
  zones: Zone[];
}

/** Besoins et besoin bioclimatique d'un mois ou de l'année (Th-BCE 2020 §13.1.2, sorties). */
export interface BesoinsBbio {
  /**
   * « B_Ch » : besoin de chauffage, en kWh par m² de surface de référence (éq. 2379 et
   * 2382).
   */
  b_ch_kwh_m2: number;
  /**
   * « B_Ecl » : besoin d'éclairage, en kWh par m² de surface de référence (éq. 2381 et
   * 2384).
   */
  b_ecl_kwh_m2: number;
  /**
   * « B_Fr » : besoin de refroidissement, en kWh par m² de surface de référence, compté
   * positivement (éq. 2380 et 2383).
   */
  b_fr_kwh_m2: number;
  /** « Bbio_pts » : besoin bioclimatique, en points (éq. 2385 et 2386 ; décision Q10). */
  bbio_pts: number;
}

/** Bilan de la production photovoltaïque du bâtiment (§13.4.3.5). */
export interface BilanProductionPv {
  /**
   * « O_Eef_Elec_Exportee_annuel » : électricité exportée, en kWh (éq. 2483) ; sans effet
   * sur Cep, Cep,nr et Ic énergie (article 9 de l'arrêté).
   */
  e_exportee_kwh: number;
  /** Production autoconsommée par le bâtiment, en kWh (éq. 2527 et 2530). */
  e_prod_pv_ac_kwh: number;
  /** Production autoconsommée, par mois, en kWh. */
  e_prod_pv_ac_mensuel_kwh: number[];
  /** « O_Eef_Prod_PV_annuel » : production, en kWh. */
  e_prod_pv_kwh: number;
  /** Production, par mois des scénarios, en kWh. */
  e_prod_pv_mensuel_kwh: number[];
  /** « O_TAC_elec_PV_annuel » : taux d'autoconsommation, en % (éq. 2485). */
  tac_pct: number;
}

/**
 * Th-BCE 2020 §6.2.2 — ensemble « bouche et conduit » d'un groupe, en maison individuelle ou
 * accolée et en logement collectif : un circuit de reprise ou un circuit de soufflage.
 *
 * Les débits sont les débits d'hygiène du circuit pour le groupe, en pointe (grand débit) et en
 * base. La méthode ne donne pas les débits réglementaires par nombre de pièces : ils se
 * saisissent. Pour un système hygroréglable, le débit de pointe vient des débits maximaux de
 * l'avis technique en cuisine et en WC et des débits d'hygiène des autres pièces ; le débit de
 * base se déduit de `qvarep_spec_m3_h` (§6.2.3.1.1.2).
 */
export interface BoucheConduit {
  /**
   * « Acond » : surface externe du conduit, en m², saisie par l'utilisateur
   * (`Is_surf_cond_def = 0`, §6.2.3.4) ; en maison individuelle, cette possibilité est
   * conditionnée à un test d'étanchéité à la réception (note 6). Absente : surface par défaut
   * (éq. 421 à 427). Toutes les bouches d'un système la portent, ou aucune.
   */
  a_cond_m2?: number | null;
  /**
   * « Cdep » : coefficient de dépassement (§6.2.3.3.1, tableau 58) : 1,15 pour des
   * composants autoréglables certifiés, la valeur de l'évaluation pour des composants
   * hygroréglables. Absent : 1,30, valeur par défaut, signalée dans les résultats. Le calcul
   * du Bbio emploie toujours 1 (éq. 414).
   */
  cdep?: number | null;
  /**
   * « Crdbnr » : coefficient de réduction des débits en occupation, en non résidentiel
   * (§6.2.3.2.1, tableau 57) : 1 sans régulation des débits, 0,9 avec un dispositif de
   * détection d'utilisation du local, 0,8 avec un comptage d'occupants ou des sondes de CO2 ;
   * une autre valeur si elle vient d'un Avis Technique. Absent : 1, aucune régulation,
   * signalé dans les résultats. Sans objet en habitation.
   */
  crdbnr?: number | null;
  /**
   * « Dugd » : durée d'utilisation du grand débit, en h par semaine (§6.2.3.2.2.2, tableau
   * 57) : 14 pour un dispositif à gestion manuelle, 7 avec temporisation ; davantage pour un
   * passage en grand débit commandé depuis plusieurs pièces (note 3). Absente : 14, gestion
   * manuelle « par défaut », signalée dans les résultats. Le calcul du Bbio emploie toujours
   * 14.
   */
  dugd_h?: number | null;
  /** Identifiant, unique dans le groupe. */
  id: string;
  /** « Isouf » : sens du circuit. */
  isouf: Isouf;
  /**
   * « qspec,rep,conv_base » ou « qspec,souffle,conv_base » : débit volumique spécifique
   * conventionnel en base, en m³/h, positif et au plus égal au débit de pointe. À saisir sauf
   * pour un système hygroréglable.
   */
  q_spec_conv_base_m3_h?: number | null;
  /**
   * « qspec,rep,conv_pointe » ou « qspec,souffle,conv_pointe » : débit volumique spécifique
   * conventionnel en pointe, en m³/h, positif. Requis en maison individuelle ou accolée et
   * en logement collectif ; ne pas le saisir dans les autres usages, qui se décrivent par
   * `q_spec_occ_m3_h` et `q_spec_inocc_m3_h`.
   */
  q_spec_conv_pointe_m3_h?: number;
  /**
   * « qspec,rep,inocc » ou « qspec,soufflé,inocc » : débit volumique total repris ou soufflé
   * en inoccupation, en m³/h, positif ou nul (éq. 385 et 386). Requis comme
   * `q_spec_occ_m3_h`.
   */
  q_spec_inocc_m3_h?: number | null;
  /**
   * « qspec,rep,occ » ou « qspec,soufflé,occ » : débit volumique total repris ou soufflé en
   * occupation, en m³/h, positif, pour les usages autres que la maison individuelle ou
   * accolée et le logement collectif (§6.2.3.1.1.1, éq. 383 et 384). Requis dans ces usages.
   * Le calcul du Bbio ne l'emploie pas : ses débits sont conventionnels (§6.2.3.1.3,
   * tableau 56-1).
   */
  q_spec_occ_m3_h?: number | null;
  /**
   * « Qvarepspec » : débit de l'avis technique d'un système hygroréglable, en m³/h
   * (§6.2.3.1.1.2). Le débit de base en est déduit ; ne pas saisir alors
   * `q_spec_conv_base_m3_h`.
   */
  qvarep_spec_m3_h?: number | null;
}

/** Réseau d'ECS intergroupe bouclé (§9.8.2). */
export interface BoucleEcs {
  /**
   * « Typegest_circ-e » (§9.8.2, éq. 1747 et 1748) : vrai si les circulateurs sont arrêtés
   * pendant les vacances des zones d'enseignement desservies (1), faux s'ils fonctionnent en
   * permanence (0). Requis quand la boucle dessert une zone d'usage affilié à l'enseignement
   * (usages 4, 5, 7, 26 et 27) ; ne se saisit pas sinon (fonctionnement permanent).
   */
  arret_circulateurs_vacances?: boolean | null;
  /** Espace tampon traversé par la partie hors volume chauffé (« btherm », éq. 1744). */
  espace_tampon?: string | null;
  /** « Lhvc_prim_bcl-e » : longueur hors volume chauffé, en m. */
  l_hvc_m: number;
  /** « Lvc_prim_bcl-e » : longueur en volume chauffé, en m. */
  l_vc_m: number;
  /** « Pcirc-e » : puissance des circulateurs, en W (fonctionnement permanent en habitation). */
  p_circ_w: number;
  /** « Isrechauf_bcl-e » : réchauffeur de boucle électrique. */
  rechauffeur: boolean;
  /** « Uprim-e » : coefficient linéique, en W/(m.K). */
  u_w_mk: number;
}

/** Boucle solaire (tableau 292). */
export interface BoucleSolaireSaisie {
  /**
   * « αp » : part de la puissance de la pompe transmise au fluide. Absente : 0,5, valeur
   * conventionnelle du tableau 292, signalée.
   */
  alpha_p?: number | null;
  /**
   * « hrelech_base » : hauteur relative de l'échangeur solaire à partir du bas du ballon,
   * de 0 à 1 (éq. 1781 à 1785). Absente : échangeur ponctuel en zone basse.
   */
  h_rel_ech?: number | null;
  /**
   * « ṁCp » : débit calorifique de la boucle, en W/K. Absent : 58 W/K par m² de capteurs,
   * valeur conventionnelle du tableau 292, signalée.
   */
  m_cp_w_k?: number | null;
  /** « Pnp » : puissance nominale de la pompe, en W. */
  p_np_w: number;
  /** « Typeregul_BS » : régulation de la boucle. */
  regulation: RegulationBoucleSaisie;
  /**
   * Transfert vers le ballon : échangeur (« (UA)hx », 100 W/K par m² de capteurs par
   * défaut, tableau 292) ou sans échangeur (§9.15.3.1.2).
   */
  transfert: TransfertBoucle;
  /** « (UA)te » : pertes des tuyauteries vers l'extérieur, en W/K. */
  ua_te_w_k: number;
  /** « (UA)ti » : pertes des tuyauteries vers l'intérieur du bâtiment, en W/K. */
  ua_ti_w_k: number;
}

/**
 * Détermination détaillée de la classe d'exposition au bruit d'une baie (fiche d'application
 * 5.11 ; annexe de l'article R. 172-4, chapitre V, partie I, point 3). **Facultative** : elle
 * remplace la saisie directe de `expo_br`.
 *
 * La catégorie sonore des voies (arrêté préfectoral de classement, article R. 571-38 du code
 * de l'environnement) et le plan d'exposition au bruit de l'aérodrome (arrêté préfectoral) sont
 * des **données à saisir** : le moteur ne peut ni les déterminer, ni les retrouver. Il en
 * déduit la classe de la baie (tableaux de la partie I, configurations 1 et 2) et, de là, celle
 * des groupes et des zones (point 4) quand `exigences.zone_bruit` n'est pas saisi.
 */
export interface BruitBaie {
  /**
   * Identifiant de la façade (libre, commun aux baies d'une même façade du bâtiment). « À
   * défaut d'une détermination détaillée, la classe BR d'une baie d'une façade est la classe
   * la plus élevée des baies de cette façade » : sans `infrastructures` ni `zone_peb`, la baie
   * prend la classe la plus élevée des autres baies de la même façade (déterminées en détail
   * ou par `expo_br`).
   */
  facade?: string | null;
  /**
   * Infrastructures de transports terrestres vues de la baie, une entrée par infrastructure
   * (distance et vue propres à chacune). Plusieurs infrastructures : la classe la plus
   * défavorable est retenue. Une infrastructure au-delà de sa distance maximale de prise en
   * compte (700, 500, 250, 100 ou 30 m selon la catégorie 1 à 5) donne BR1 : il est inutile
   * de la saisir.
   */
  infrastructures?: InfrastructureBruit[];
  /**
   * Zone du plan d'exposition au bruit de l'aérodrome le plus proche où se situe le
   * bâtiment (zones A à C : BR3, D : BR2, hors zone : BR1, « toutes vues »). Absente : aucun
   * aérodrome à prendre en compte.
   */
  zone_peb?: ZonePeb | null;
}

/** Régulation du bypass de l'échangeur (§6.3.3.4.6.2, éq. 494). */
export interface Bypass {
  /** « θext,bypass,été » : la même hors saison de chauffage, en °C. */
  theta_ext_bypass_ete_c: number;
  /**
   * « θext,bypass,hiver » : température d'air neuf au-dessus de laquelle l'échangeur peut
   * être contourné en saison de chauffage, en °C.
   */
  theta_ext_bypass_hiver_c: number;
  /** « θint,bypass,été » : la même hors saison de chauffage, en °C. */
  theta_int_bypass_ete_c: number;
  /**
   * « θint,bypass,hiver » : température intérieure au-dessus de laquelle l'échangeur peut
   * être contourné en saison de chauffage, en °C.
   */
  theta_int_bypass_hiver_c: number;
}

/** Capteurs solaires d'une boucle (tableau 292). */
export interface CapteursSolaires {
  /** « Kθ » : facteur d'angle d'incidence, de 0 à 1 (sans valeur par défaut dans le texte). */
  k_theta: number;
  /** « η0 », « a1 », « a2 » : valeurs certifiées ou valeurs par défaut du tableau 293. */
  performances: PerformancesCapteurs;
  /** « A » : superficie d'entrée des capteurs d'un chauffe-eau, en m². */
  surface_m2: number;
}

/** Composants décrits une fois par leurs paramètres intrinsèques. */
export interface Catalogue {
  /** Types de baies vitrées. */
  baies?: BaieType[];
  /** Masques solaires. */
  masques?: MasqueDefinition[];
  /** Types de parois opaques. */
  parois?: ParoiType[];
}

/** Catégorie de contrainte extérieure du groupe (§13.1.2 : « 1 = CE1 / 2 = CE2 »). */
export type CategorieCe = "CE1" | "CE2";

/** Catégorie de contraintes extérieures (annexe de l'article R. 172-4, chapitre V). */
export type CategorieContraintes = "categorie_1" | "categorie_2" | "categorie_3";

/** Catégorie d'une infrastructure de transports terrestres (arrêté du 30 mai 1996). */
export type CategorieInfrastructure = "categorie_1" | "categorie_2" | "categorie_3" | "categorie_4" | "categorie_5";

/**
 * Th-BCE 2020 §8.19.2, tableau 114 — chaudière au gaz, au fioul ou au bois. La position de la
 * génération (`position`) fixe « idpos_gen ».
 */
export interface ChaudiereSaisie {
  /**
   * « Alim_Chaudière_Bois » : requis pour une chaudière au bois dont les auxiliaires sont
   * par défaut.
   */
  alimentation_bois?: AlimentationBoisSaisie | null;
  /** Auxiliaires. */
  auxiliaires?: AuxiliairesChaudiere;
  /** « TypeCombustibleBois » : requis pour une chaudière au bois. */
  combustible_bois?: CombustibleBoisSaisi | null;
  /** « Id_fou_gen » : fonction du générateur. */
  fonction: FonctionChaleur;
  /**
   * Gaz de pétrole liquéfié (PCSI de 1,09 au lieu de 1,11, tableau 116) ; gaz naturel
   * sinon. Le GPL relève des « autres énergies » de l'article 9 et du facteur d'émission
   * « gaz butane / propane » de l'article 10 (lot 7).
   */
  gpl?: boolean;
  /** « idpertes_parois ». */
  pertes_parois: PertesParoisChaudiere;
  /** « Pn_gen » : puissance thermique utile nominale en régime haute température, en kW. */
  pn_gen_kw: number;
  /**
   * « Q_po_30 » : pertes thermiques en régime stabilisé pour un écart de 30 K, en W ; absent :
   * valeur par défaut (éq. 1074).
   */
  q_po_30_w?: number | null;
  /** Rendements mesurés ; absents : valeurs par défaut (éq. 1072 et 1073). */
  rendements?: RendementsChaudiere | null;
  /** Déclinaison de la chaudière. */
  type_chaudiere: TypeChaudiereSaisi;
}

/**
 * Chauffe-eau solaire individuel : assemblage « ballon base solaire » (§9.14, fiche 9.15),
 * décrit composant par composant ou, pour un kit caractérisé selon la norme NF EN 12976, par
 * les paramètres de l'outil IdCESI (§9.23).
 */
export interface ChauffeEauSolaire {
  /** Appoint. */
  appoint: AppointCesi;
  /** Description du chauffe-eau. */
  description: DescriptionCesi;
  /** Orientation, inclinaison et masques du plan des capteurs (fiche 3.2). */
  exposition_capteurs: Exposition;
  /** Identifiant, unique parmi les assemblages de la génération. */
  id: string;
  /**
   * « Nbassemblage » : nombre de chauffe-eau identiques (au moins 1). Un générateur
   * d'appoint intégré désigné représente autant de générateurs identiques (`rdim`).
   */
  nb_assemblages: number;
}

/**
 * Production centralisée à appoints décentralisés (« PCAD », fiches 9.16 et 9.17) : un champ
 * de capteurs et une boucle solaire collectifs, des ensembles de ballons individuels
 * identiques et, pour un CESCAI, un ballon solaire central relié aux ballons individuels par
 * le réseau d'ECS intergroupe bouclé.
 */
export interface ChauffeEauSolaireCollectif {
  /**
   * Ballon solaire central d'un CESCAI (« ballon base seule », éq. 2065 et 2066) ; sans
   * objet pour un CESCI.
   */
  ballon_central?: BallonSaisi | null;
  /**
   * Boucle solaire collective ; ses échangeurs sont ceux de tous les ballons qu'elle
   * chauffe. Un CESCI est régulé sur l'ensoleillement (éq. 1981).
   */
  boucle: BoucleSolaireSaisie;
  /** Capteurs solaires du champ collectif (« A » : superficie totale du champ). */
  capteurs: CapteursSolaires;
  /** Ensembles de ballons individuels strictement identiques (« nbens », au moins un). */
  ensembles: EnsembleBallons[];
  /** Orientation, inclinaison et masques du plan des capteurs (fiche 3.2). */
  exposition_capteurs: Exposition;
  /** Identifiant, unique parmi les générateurs et assemblages de la génération. */
  id: string;
  /** « Type_PCAD ». */
  type_pcad: TypePcad;
}

/**
 * Th-BCE 2020 §9.22 — chauffe-eau thermodynamique à compression électrique décrit par son
 * essai selon la norme NF EN 16147 (étapes C à F en fonctionnement thermodynamique pur).
 */
export interface ChauffeEauThermodynamique {
  /** Résultats de l'essai. */
  essai: EssaiEn16147;
  /** Identifiant, unique dans la génération. */
  id: string;
  /** « Lim_Theta » de la pompe à chaleur en ECS. Absent : pas de limite. */
  limites?: LimitesPac | null;
  /** « Nbassemblage » : nombre de chauffe-eau identiques (au moins 1). */
  nb_assemblages: number;
  /** « typegest_base » : programmation du thermostat. */
  programmation: ProgrammationBallon;
  /**
   * Source amont de la pompe à chaleur (« Typesource_chaleur », éq. 2256 à 2258) : air
   * extérieur, air ambiant d'un espace tampon ou air extrait.
   */
  source_amont: SourceAmontSaisie;
  /** « θmax » : température maximale du ballon, en °C. */
  theta_max_c: number;
  /** « VtotIdCET » : volume du ballon, en L. */
  v_tot_l: number;
}

/** « idcirc » (§8.7.2, §8.9.2). */
export type Circulateur = "aucun" | "vitesse_constante" | "pression_constante" | "pression_variable";

/** Classe d'un émetteur de chaud (tableau 82). */
export type ClasseEmetteurChaud = "A" | "B1" | "B2" | "B3" | "C";

/** Classe d'un émetteur de froid (tableau 84). */
export type ClasseEmetteurFroid = "A" | "B" | "C";

/** « Cletres » : classe d'étanchéité d'un réseau aéraulique (tableau 59). */
export type ClasseEtancheite = "A" | "B" | "C";

/** « Lim_Theta » : 1 ou 2. */
export type CombinaisonLimites = "l_une_ou_l_autre" | "l_une_et_l_autre";

/** « TypeCombustibleBois » (§8.19.2). */
export type CombustibleBoisSaisi = "buches" | "granules" | "plaquettes";

/** « CT » : degré de confinement thermique de la face arrière des modules (§12.3.3.2). */
export type ConfinementPv = "libre" | "autre" | "confine";

/**
 * Consommations en énergie finale, annuelles et mensuelles (mois des scénarios), des
 * générations d'un bâtiment attribuées à un groupe, à une zone ou au bâtiment, ou d'une
 * génération (§8.29, éq. 1497 à 1514).
 */
export interface Consommations {
  /** Consommations annuelles, en kWh. */
  annuel: ConsommationsPostes;
  /** Consommations des douze mois, en kWh. */
  mensuel: ConsommationsPostes[];
}

/**
 * Postes d'électricité propres au logement collectif et aux parkings, distincts pour que le
 * calcul des indicateurs les intègre selon le texte : éclairage (éq. 2510 : parties communes
 * et parkings), auxiliaires de ventilation (éq. 2511 : parkings), déplacements (éq. 2513 :
 * ascenseurs). Aucun ne produit d'apport interne dans le modèle thermique (`docs/questions/Q050.md`).
 */
export interface ConsommationsCollectif {
  /** Ascenseurs (§10.1, éq. 2310). */
  ascenseurs: PosteElectrique;
  /** Éclairage des parkings (§10.4, éq. 2353 et 2354). */
  eclairage_parkings: PosteElectrique;
  /** Éclairage des circulations, hall et escalier (§7.2, éq. 792 à 794). */
  eclairage_parties_communes: PosteElectrique;
  /** Ventilation des parkings fermés (§10.3, éq. 2346 et §10.3.3.8). */
  ventilation_parkings: PosteElectrique;
}

/**
 * Consommations en énergie finale d'un poste par type d'énergie, en kWh (colonnes de la
 * matrice du tableau 252, §8.29.3.1). Les énergies sans consommation ne sont pas écrites.
 */
export interface ConsommationsParEnergie {
  /** 40 : bois. */
  bois_kwh?: number;
  /** 30 : charbon. */
  charbon_kwh?: number;
  /** 50 : électricité. */
  electricite_kwh?: number;
  /** 20 : fioul. */
  fioul_kwh?: number;
  /** 10 : gaz, en kWh PCI. */
  gaz_kwh?: number;
  /** 60 : réseau de chaleur ou de froid. */
  reseau_kwh?: number;
}

/**
 * Consommations en énergie finale des générations par poste (lignes de la matrice du tableau
 * 252). Les consommations des auxiliaires de génération — auxiliaires propres aux générateurs
 * et auxiliaires de leurs sources amont — sont comprises dans le poste qu'ils servent (éq.
 * 1054 et 1055), comme le réchauffeur d'une boucle d'ECS dans le poste ECS (éq. 1057).
 */
export interface ConsommationsPostes {
  /** 1 : chauffage. */
  chauffage: ConsommationsParEnergie;
  /** 3 : eau chaude sanitaire. */
  ecs: ConsommationsParEnergie;
  /** 2 : refroidissement. */
  refroidissement: ConsommationsParEnergie;
}

/** « Idcycle » (§9.22.3.2.3, tableau 305). */
export type CycleSoutirage = "s" | "m" | "l" | "xl" | "xxl";

/** Code du département : 01 à 95 (sans 20), 2A ou 2B */
export type Departement = string;

/** Description d'un chauffe-eau solaire individuel. */
export type DescriptionCesi = {
  composants: {
    /** Ballon de stockage. */
    ballon: BallonSaisi;
    /** Boucle solaire. */
    boucle: BoucleSolaireSaisie;
    /** Capteurs solaires. */
    capteurs: CapteursSolaires;
  };
} | {
  kit: KitCesi;
};

/**
 * Th-BCE 2020 §9.7.2 — distribution d'ECS du groupe : réseau sans circulateur ni isolant,
 * supposé à température constante.
 *
 * Classement des longueurs en ou hors volume chauffé : fiche d'application 8.13 (en gaine
 * logement, en volume chauffé).
 */
export interface DistributionEcsGroupe {
  /** « Dint_2nd-e » : diamètre intérieur, en m. */
  d_int_m: number;
  /**
   * Espace tampon traversé par la partie hors volume chauffé (« btherm », éq. 1733). Absent :
   * à l'extérieur (`btherm = 1`).
   */
  espace_tampon?: string | null;
  /** « Lhvc_2nd-e » : longueur hors volume chauffé d'une distribution, en m. */
  l_hvc_m: number;
  /**
   * « Lvc_2nd-e » : longueur en volume chauffé d'une distribution, en m. Absent : valeur par
   * défaut de l'éq. (1729), `6 × A/80` en habitation pour la surface desservie (« δlvc = 0 »),
   * signalée dans les résultats.
   */
  l_vc_m?: number | null;
  /** « nbdist_2nd-e » : nombre de distributions identiques reliées à l'émetteur. */
  nb_dist: number;
  /**
   * « θ2nd-e » : température de la distribution, en °C, au moins égale à celle de l'eau
   * mitigée (40 °C). Absent : 53 °C, valeur conventionnelle du tableau 278 (lecture Q45),
   * signalée dans les résultats.
   */
  theta_2nd_e_c?: number | null;
}

/** Th-BCE 2020 §9.8 — distribution d'ECS intergroupe, reliée à une génération. */
export interface DistributionEcsIntergroupe {
  /** Identifiant de la génération qui produit l'ECS. */
  generation: string;
  /** Identifiant, unique dans le bâtiment. */
  id: string;
  /** « Typeréseauintergroupe-e ». */
  reseau: ReseauEcsIntergroupe;
}

/**
 * Th-BCE 2020 §8.7 et §8.8 — réseau de distribution du groupe, de chaud ou de froid, connecté
 * à un et un seul réseau intergroupes de même fonction et de même type (éq. 850 et 851).
 */
export interface DistributionGroupe {
  /** Identifiant du réseau intergroupes du bâtiment auquel le réseau est connecté. */
  distribution_intergroupe: string;
  /** « idfonction » : chauffage ou refroidissement. */
  fonction: FonctionReseau;
  /** Identifiant du réseau, unique dans le groupe. */
  id: string;
  /** Nature du réseau (« idtype »). */
  reseau: ReseauGroupe;
}

/**
 * Th-BCE 2020 §8.9 et §8.10 — réseau de distribution intergroupes de chaud ou de froid,
 * relié à une et une seule génération (§8.10.1).
 */
export interface DistributionIntergroupes {
  /**
   * « idfonction » : chauffage ou refroidissement. Un réseau « 2 tubes réversible » se
   * décrit par deux réseaux, l'un de chauffage, l'autre de refroidissement (§8.9.1).
   */
  fonction: FonctionReseau;
  /** Identifiant de la génération qui alimente le réseau. */
  generation: string;
  /** Identifiant du réseau, unique dans le bâtiment. */
  id: string;
  /** Nature du réseau (« idtype »). */
  reseau: ReseauIntergroupes;
}

/** Th-BCE 2020 §6.3.2, tableau 61 — groupe de ventilation double flux en habitation. */
export interface DoubleFlux {
  /** Échangeur statique. Absent : pas d'échangeur (`type_échangeur = 0`, éq. 496). */
  echangeur?: Echangeur | null;
  /** Humidificateur d'hiver (« Is_ChaudHR ») : non pris en charge dans cette version. */
  humidification?: boolean | null;
  /** « Pvent,base,rep » : puissance électrique du ventilateur de reprise en débit de base, en W. */
  p_vent_base_rep_w: number;
  /**
   * « Pvent,base,sou » : puissance électrique du ventilateur de soufflage en débit de base, en
   * W.
   */
  p_vent_base_sou_w: number;
  /**
   * « Pvent,pointe,rep » : puissance électrique du ventilateur de reprise en débit de pointe,
   * en W.
   */
  p_vent_pointe_rep_w: number;
  /**
   * « Pvent,pointe,sou » : puissance électrique du ventilateur de soufflage en débit de
   * pointe, en W.
   */
  p_vent_pointe_sou_w: number;
  /** Batterie de préchauffage (« Is_prechaud ») : non prise en charge dans cette version. */
  prechauffage?: boolean | null;
  /**
   * Rafraîchissement évaporatif (« Type_humidification_Eté ») : non pris en charge dans
   * cette version.
   */
  rafraichissement_evaporatif?: boolean | null;
  /**
   * Rafraîchissement par surventilation mécanique (§6.12) : non pris en charge dans cette
   * version.
   */
  surventilation_mecanique?: boolean | null;
}

/** Th-BCE 2020 §6.3.2, tableau 61, et §6.3.3.4.6 — échangeur d'un double flux. */
export interface Echangeur {
  /**
   * « isantigel » : sécurité antigel par préchauffage de l'air neuf. Absent ou faux :
   * l'échangeur est contourné quand un risque de givrage est identifié (éq. 509).
   */
  antigel?: boolean | null;
  /** Bypass de l'échangeur (« isbypass = 1 ») et sa régulation. Absent : pas de bypass. */
  bypass?: Bypass | null;
  /**
   * Génération qui fournit l'énergie de la sécurité antigel (« Wag », éq. 508) : identifiant
   * d'une génération du bâtiment pourvue d'un générateur de chauffage. La demande lui parvient
   * comme celle d'un réseau de distribution associé à l'antigel (§8.11.3, éq. 939). Requise
   * avec `antigel`, sans objet sinon.
   */
  generation_antigel?: string | null;
  /** Représentation de l'échangeur (« idniveau_detail_echangeur »). */
  representation: RepresentationEchangeur;
  /** Échangeur rotatif (« Pech », échangeur à roue) : non pris en charge dans cette version. */
  rotatif?: boolean | null;
  /**
   * « θs,ech,rep,LIM » : température limite basse de sortie côté rejet, en °C (de −10 à
   * 10). Absente : 5 °C, valeur par défaut du tableau 62 (« autres cas »), signalée dans les
   * résultats.
   */
  theta_s_ech_rep_lim_c?: number | null;
}

/** Échangeur en représentation détaillée. */
export interface EchangeurDetaille {
  /** « idtype_echangeur » : nature des écoulements. */
  ecoulement: EcoulementEchangeur;
  /** « UA_saisi » : coefficient d'échange global, en W/K. */
  ua_w_k: number;
}

/** Échangeur en représentation simplifiée. */
export interface EchangeurSimplifie {
  /**
   * « εsaisi » : efficacité de l'échangeur en ventilation seule, mesurée aux débits de la
   * zone neutre en occupation (de 0 à 1).
   */
  epsilon_saisi: number;
  /**
   * « idstatut_echangeur ». Absent : valeur déclarée par le fabricant, « par défaut » selon
   * le tableau 61, signalé dans les résultats.
   */
  statut?: StatutEchangeur | null;
}

/** Th-BCE 2020 §7.1.2 — système d'éclairage d'un local. */
export interface EclairageLocal {
  /**
   * « Eff_ecl_immo_projet » : densité de puissance de l'éclairage immobilier, en W/m² pour
   * 100 lux. Requise pour un local de bureaux d'une zone de bureaux dont la puissance
   * immobilière est inférieure à 10 W/m² (§7.1.3.2.3) ; sans objet sinon.
   */
  eff_ecl_immo_projet_w_m2_100lux?: number | null;
  /**
   * « Eiproj » : éclairement requis du projet, en lux. Requis dans le même cas que
   * `eff_ecl_immo_projet_w_m2_100lux` : si l'éclairage immobilier ne l'atteint pas,
   * l'éclairage mobilier assure le complément, à raison de 1 W/m² pour 100 lux.
   */
  ei_proj_lux?: number | null;
  /**
   * « Fr_Grad_ecl,l » : vrai si la gestion selon la lumière naturelle est fractionnée (2),
   * c'est-à-dire si la partie du local qui a accès à la lumière naturelle est gérée
   * indépendamment de celle qui n'y a pas accès ; faux sinon (1).
   */
  fractionne: boolean;
  /** « Gest_ecl,l » : mode de commande de l'éclairage artificiel. */
  gest_ecl: GestEcl;
  /** « Grad_ecl,l » : mode de gestion de l'éclairage artificiel selon la lumière naturelle. */
  grad_ecl: GradEcl;
  /**
   * « Pecl_aux,l » : puissance surfacique des appareillages et périphériques de gestion
   * quand les lampes sont éteintes (horloges, gradateurs, détecteurs, boutons-poussoirs
   * alimentés ; hors ballasts, transformateurs et drivers), en W/m². Requise avec une
   * détection de présence ou d'absence (`gest_ecl`) ou une gestion automatique selon la
   * lumière naturelle (`grad_ecl`) ; sinon elle est nulle par convention : ne pas la saisir.
   */
  p_ecl_aux_w_m2?: number | null;
  /**
   * « Pecl_tot,l » : puissance surfacique totale d'éclairage installée à poste fixe dans le
   * local, en W/m² : lampes et accessoires, système de gestion intégré aux luminaires,
   * éclairages d'appoint des postes de travail ; hors éclairage de sécurité, éclairage
   * extérieur, éclairage de mise en valeur et éclairage de process (§7.1.1). Dans un local de
   * bureaux d'une zone de bureaux, c'est la puissance de l'éclairage immobilier
   * (« Pecl_immo_projet », §7.1.3.2.3).
   */
  p_ecl_tot_w_m2: number;
}

/** Th-BCE 2020 §10.4.2 — éclairage d'un parking. */
export interface EclairageParking {
  /**
   * « Ex » : extinction de l'éclairage quand le parking est fermé (vrai) ou marche forcée
   * (faux). Requis si le parking a des jours de fermeture.
   */
  extinction_en_fermeture?: boolean | null;
  /**
   * « Pecins » : puissance totale installée, en W. Absent : 75 W par place en intérieur, 8 W
   * par place en extérieur (éq. 2349 et 2350).
   */
  p_installee_w?: number | null;
  /**
   * « PlagDse » : plages de fonctionnement en détection de présence du lundi au vendredi,
   * au plus trois, en couples (heure légale de début, heure légale de fin) : l'éclairage est
   * en détection de `debut` h à `fin` h. Absent : pas de détection.
   */
  plages_detection_semaine?: PlageHoraire[];
  /** « PlagDwe » : plages de détection du samedi et du dimanche. */
  plages_detection_week_end?: PlageHoraire[];
}

/** « idtype_echangeur » (tableau 61). */
export type EcoulementEchangeur = "contre_courant" | "courants_paralleles" | "courants_croises_brasses";

/** Th-BCE 2020 §8.18.2, tableau 112 — générateur direct à effet Joule. */
export interface EffetJouleSaisi {
  /** « idfougen » : fonction du générateur, chauffage ou ECS ; jamais les deux (§8.18.3.1). */
  fonction: FonctionEffetJoule;
  /** « Pngen » : puissance nominale d'un générateur, en W. */
  pn_gen_w: number;
}

/** Th-BCE 2020 §8.1 — émetteur de chaud et/ou de froid d'un groupe (« em »). */
export interface Emetteur {
  /**
   * Fonction de chauffage (« idem_chaud = 1 ») et ses caractéristiques. Absent : l'émetteur
   * ne chauffe pas.
   */
  chauffage?: EmissionChaud | null;
  /**
   * « Iddetection_presence » (§8.1.3.1.6, éq. 799) : détection de présence pièce par pièce
   * (chauffage). Absent : faux.
   */
  detection_presence?: boolean | null;
  /** Identifiant de l'émetteur, unique dans le groupe. */
  id: string;
  /**
   * « Pper » : ratio des pertes au dos sur l'énergie émise (§8.1.3.1.4). Absent : pas de
   * pertes au dos. Pour un émetteur intégré à une paroi donnant sur l'extérieur, un vide
   * sanitaire, le sol ou un local non chauffé, le texte le calcule par l'éq. (795),
   * `Pper = Ri·Ue` : voir `pertes_au_dos`.
   */
  pertes_au_dos?: PertesAuDos | null;
  /**
   * Fonction de refroidissement (« idem_froid = 1 ») et ses caractéristiques. Absent :
   * l'émetteur ne refroidit pas.
   */
  refroidissement?: EmissionFroid | null;
  /**
   * Ventilateurs locaux (émetteurs gainés ou carrossés de type ventilo-convecteur,
   * §8.1.3.8). Absent : « GestVCV = 0 », pas de ventilateur.
   */
  ventilateurs?: VentilateursLocaux | null;
}

/**
 * Th-BCE 2020 §9.5 et §9.6 — émetteur d'ECS équivalent d'un groupe : émetteurs et appareils
 * sanitaires d'un même groupe reliés à une même génération (§9.5.3.1).
 */
export interface EmetteurEcs {
  /**
   * « App_ecs » : appareils sanitaires (tableau 275) ; s'il y en a de natures différentes,
   * le plus défavorable.
   */
  appareils: AppareilsEcs;
  /** Réseau de distribution d'ECS du groupe, propre à l'émetteur (§9.7). */
  distribution: DistributionEcsGroupe;
  /** Identifiant du réseau de distribution d'ECS intergroupe du bâtiment (§9.8). */
  distribution_intergroupe: string;
  /** Identifiant, unique dans le groupe. */
  id: string;
  /**
   * « Nbmaison » (maison individuelle ou accolée : nombre de maisons desservies, plus de 1
   * pour des maisons accolées) ou « Nblgt-e » (logement collectif : nombre de logements
   * desservis), tableau 272. Requis, au moins 1, en maison individuelle ou accolée et en
   * logement collectif ; sans objet dans les autres usages.
   */
  nb_logements?: number;
  /**
   * « Nu » : nombre d'unités caractéristiques desservies, pour les usages autres que la
   * maison individuelle ou accolée et le logement collectif (§9.6.3.2.2.1, tableau 277 : en
   * m² de surface utile pour tous ces usages). Absent : la surface utile que l'émetteur
   * dessert, `rat_em × su_m2` (éq. 1682). Ne pas saisir en habitation.
   */
  nu?: number | null;
  /**
   * « partmélangeur » : part des besoins passant par des mélangeurs, mitigeurs mécaniques et
   * autres émetteurs (gain 0 %, tableau 273).
   */
  part_melangeur: number;
  /**
   * « partmitigeur » : part passant par des mitigeurs thermostatiques ou mécaniques économes
   * (classe C3 ou CH3 ; gain 5 %).
   */
  part_mitigeur: number;
  /**
   * « parttemporisateur » : part passant par des robinets électroniques ou des temporisateurs
   * (gain 7 %).
   */
  part_temporisateur: number;
  /**
   * « Ratem » : part de la surface utile du groupe desservie ; la somme sur les émetteurs
   * d'ECS du groupe vaut 1 (éq. 1673).
   */
  rat_em: number;
  /**
   * « Irecup » : récupérateur de chaleur sur les eaux grises (fiche 9.21) : non pris en
   * charge dans cette version. Absent : faux.
   */
  recuperateur_eaux_grises?: boolean | null;
}

/** Caractéristiques d'un émetteur en chauffage (§8.1.2 et §8.1.3.1). */
export interface EmissionChaud {
  /**
   * Identifiant du réseau de distribution du groupe (fonction chauffage) qui alimente
   * l'émetteur (§8.7.1).
   */
  distribution: string;
  /**
   * Émetteur à effet Joule : une valeur certifiée de variation temporelle ne peut pas être
   * inférieure à 0,2 K, contre 0,4 K pour les autres émetteurs de chauffage (§8.1.3.1.5).
   */
  effet_joule: boolean;
  /** « Rats_ch » : part de la surface utile du groupe desservie par l'émetteur. */
  rats: number;
  /** « Ratt_ch » : part de l'énergie de chaud fournie à cette surface assurée par l'émetteur. */
  ratt: number;
  /** Typologie de l'émetteur : fixe la part convective `Pemconv_ch` (tableau 85). */
  type_emetteur: TypeEmetteurChaud;
  /** « δθvs_ch » : variation spatiale en chauffage (tableaux 82 et 83). */
  variation_spatiale: VariationSpatiale;
  /** « δθvt_ch » : variation temporelle en chauffage (§8.1.3.1.5, tableaux 87 et 88). */
  variation_temporelle: VariationTemporelle;
}

/** Caractéristiques d'un émetteur en refroidissement (§8.1.2 et §8.1.3.1). */
export interface EmissionFroid {
  /** Identifiant du réseau de distribution du groupe (fonction refroidissement). */
  distribution: string;
  /** « Rats_fr » : part de la surface utile du groupe desservie par l'émetteur. */
  rats: number;
  /** « Ratt_fr » : part de l'énergie de froid fournie à cette surface assurée par l'émetteur. */
  ratt: number;
  /** Typologie de l'émetteur : fixe la part convective `Pemconv_fr` (tableau 86). */
  type_emetteur: TypeEmetteurFroid;
  /** « δθvs_fr » : variation spatiale en refroidissement (tableau 84). */
  variation_spatiale: VariationSpatialeFroid;
  /** « δθvt_fr » : variation temporelle en refroidissement (§8.1.3.1.5, tableau 87), négative. */
  variation_temporelle: VariationTemporelle;
}

/**
 * Ensemble de ballons individuels strictement identiques d'une production collective
 * (« {B} », fiches 9.16 et 9.17).
 */
export interface EnsembleBallons {
  /**
   * Appoint intégré de chaque ballon (CESCI et CESCAI) : générateur de la génération
   * assurant l'ECS, dont `rdim` est le nombre de ballons ; sans objet pour un CESCI à appoint
   * séparé.
   */
  appoint?: AppointBallon | null;
  /**
   * Ballon individuel ; l'échangeur solaire ou du réseau intergroupe et sa sonde sont en zone
   * basse (éq. 1980 et 2066).
   */
  ballon: BallonSaisi;
  /** Identifiant, unique dans la production. */
  id: string;
  /** « nbsto^B » : nombre de ballons de l'ensemble (au moins 1). */
  nb_ballons: number;
}

/**
 * Th-BCE 2020 §5.6.2 et §5.6.3.2 — entrées d'air d'un groupe.
 *
 * La méthode forme une entrée d'air équivalente pour la zone, somme de celles des groupes, puis
 * la répartit entre les groupes au prorata de leurs modules (éq. 147 à 149) et, dans chaque
 * groupe, par quarts entre le haut et le bas des façades au vent et sous le vent (tableau 26).
 * Le groupe se décrit donc par la somme des modules de ses entrées d'air.
 */
export interface EntreesAir {
  /**
   * Comportement autoréglable ; absent pour des entrées d'air fixes. Les pressions `ΔP1` et
   * `ΔP2` sont les mêmes pour tous les groupes d'une zone (§5.6.3.2.2).
   */
  autoreglable?: AutoreglageEntreesAir | null;
  /**
   * Identifiant de l'espace tampon par lequel transite l'air neuf des entrées d'air
   * (§5.6.3.1, éq. 134 : l'air neuf entre alors à la température de l'espace tampon). Absent :
   * l'air neuf vient directement de l'extérieur.
   */
  espace_tampon?: string | null;
  /** « M » : somme des modules des entrées d'air du groupe, en m³/h (débit sous 20 Pa). */
  m_m3_h: number;
  /**
   * « r » : atténuation du débit par un dispositif anti-retour, de 0 à 1 (éq. 146). Absent :
   * pas de dispositif anti-retour (`r = 1`).
   */
  r?: number | null;
}

/**
 * Th-BCE 2020 §5.4 — espace tampon.
 *
 * « Un espace tampon est défini au niveau du bâtiment, et doit nécessairement être associé à au
 * moins un composant d'enveloppe (paroi opaque, baie) d'un groupe » (§5.4.1).
 */
export interface EspaceTampon {
  /** Identifiant de l'espace tampon, unique dans le bâtiment. */
  id: string;
  /** Nature de l'espace tampon (« isolarise »). */
  nature: NatureEspaceTampon;
}

/** Th-BCE 2020 §5.4.2, tableau 19 — espace tampon non solarisé. */
export interface EspaceTamponNonSolarise {
  /**
   * « bet_ns » : coefficient `b` de réduction des déperditions, tel que défini dans les
   * règles Th-Bat (de −5 à 1).
   */
  b_et_ns: number;
}

/** Th-BCE 2020 §5.2.1 et §5.4.2, tableaux 14 et 19 — espace tampon solarisé. */
export interface EspaceTamponSolarise {
  /** « Aet » : surface au sol, en m². */
  a_et_m2: number;
  /**
   * « Cm_et_surf » : capacité thermique quotidienne par m² de surface au sol, en
   * kJ/(m².K). Absente : 200, valeur conventionnelle du tableau 19, signalée dans les
   * résultats.
   */
  cm_et_surf_kj_m2k?: number | null;
  /** « Httfet » : hauteur de tirage thermique en surventilation naturelle, en m. */
  h_ttf_et_m: number;
  /** Parois de l'espace tampon donnant sur l'extérieur (au moins une). */
  parois: ParoiEspaceTampon[];
  /** Protections mobiles des parois vitrées (« idprot »). Absent : pas de protection. */
  protections_mobiles?: ProtectionsEspaceTampon | null;
  /**
   * « Qv,et,base » : débit d'air de base hors surventilation (défauts d'étanchéité et
   * entrées d'air), en m³/h. Absent : 1 m³/h par m² de surface au sol (§5.4.3.3.1.4),
   * signalé dans les résultats.
   */
  qv_et_base_m3_h?: number | null;
  /** « Uet_pb » : coefficient de déperdition équivalent par le plancher bas, en W/(m².K). */
  u_et_pb_w_m2k: number;
}

/** Résultats d'un essai selon la norme NF EN 16147 (§9.22.2, tableau 304). */
export interface EssaiEn16147 {
  /**
   * « Isaux » : vrai si les auxiliaires de la pompe à chaleur sont compris dans les
   * résultats de l'essai ; sinon `Taux = 0,02` s'applique (éq. 2250 et 2251).
   */
  auxiliaires_compris: boolean;
  /** « COPDHW » : coefficient de performance sur le cycle de soutirage. */
  cop_dhw: number;
  /** « Idcycle » : cycle de soutirage. */
  cycle: CycleSoutirage;
  /** « Pes » : puissance absorbée en régime stabilisé, en W. */
  p_es_w: number;
  /** Statut des résultats : certifiés ou justifiés (éq. 2252 à 2255). */
  statut: StatutMesure;
  /** « th » : durée de mise en température, en h (au plus 48). */
  t_h_h: number;
  /** « θ'WH » : température d'eau chaude de référence, en °C. */
  theta_wh_c: number;
}

/**
 * Catégorie de contraintes extérieures propre à un groupe (annexe de l'article R. 172-4,
 * chapitre V, partie II).
 */
export interface ExigencesGroupeSaisie {
  /** Catégorie de contraintes extérieures du groupe. */
  categorie_contraintes_exterieures: CategorieContraintes;
  /** Identifiant du groupe dans la zone. */
  groupe: string;
}

/** Exigences d'une zone (partie de bâtiment d'un usage), annexe de l'article R. 172-4. */
export interface ExigencesZone {
  /** « Bbio_maxmoyen », en points. */
  bbio_max_moyen: number;
  /** « Bbio_max », en points. */
  bbio_max_pts: number;
  /**
   * Catégorie de contraintes extérieures retenue pour la zone (celle de la zone si elle est
   * saisie, celle du bâtiment sinon ; Q66).
   */
  categorie_contraintes_exterieures?: CategorieContraintes | null;
  /** « Cep_max », en kWhep/(m².an). */
  cep_max_kwhep_m2: number;
  /** « Cep_maxmoyen », en kWhep/(m².an). */
  cep_max_moyen: number;
  /** « Cep,nr_max », en kWhep/(m².an). */
  cep_nr_max_kwhep_m2: number;
  /** « Cep,nr_maxmoyen », en kWhep/(m².an). */
  cep_nr_max_moyen: number;
  /** « Icénergie_max », en kg éq. CO2/m². */
  ic_energie_max_kgco2_m2: number;
  /** « Icénergie_maxmoyen », en kg éq. CO2/m². */
  ic_energie_max_moyen: number;
  /** Identifiant de la zone. */
  id: string;
  /** « Mbbruit ». */
  mb_bruit: number;
  /** « Mbcombles ». */
  mb_combles: number;
  /** « Mbgéo ». */
  mb_geo: number;
  /** « MbHSP ». */
  mb_hsp: number;
  /** « Mbsurf_moy ». */
  mb_surf_moy: number;
  /** « Mbsurf_tot ». */
  mb_surf_tot: number;
  /** « Mccat ». */
  mc_cat: number;
  /** « Mccombles ». */
  mc_combles: number;
  /** « Mcgéo ». */
  mc_geo: number;
  /** « McHSP ». */
  mc_hsp: number;
  /** « Mcsurf_moy ». */
  mc_surf_moy: number;
  /** « Mcsurf_tot ». */
  mc_surf_tot: number;
  /** « Smoylgt » : surface moyenne des logements, en m² ; 0 hors habitation. */
  smoy_lgt_m2: number;
  /** « Sref » de la zone, en m². */
  sref_m2: number;
  /** Numéro d'usage. */
  usage: number;
  /** Classe d'exposition au bruit retenue pour la zone. */
  zone_bruit?: ZoneBruit | null;
}

/**
 * Données de la fixation des exigences propres à une zone (annexe de l'article R. 172-4,
 * chapitre V).
 */
export interface ExigencesZoneSaisie {
  /**
   * Catégorie de contraintes extérieures de la zone ; absente : celle du bâtiment. Elle
   * module `Mbbruit` (catégorie 3), `Mccat` et fixe `DH_max` des groupes de la zone.
   */
  categorie_contraintes_exterieures?: CategorieContraintes | null;
  /**
   * Catégories propres à certains groupes (parties de bâtiment thermiquement homogènes) :
   * elles fixent leur `DH_max` (chapitre II, IV). Un groupe absent prend la catégorie de la
   * zone.
   */
  groupes?: ExigencesGroupeSaisie[];
  /** Identifiant de la zone du bâtiment. */
  zone: string;
  /**
   * Classe d'exposition au bruit de la zone ; absente : celle du bâtiment, sinon celle
   * déduite des baies de la zone (`bruit` de chaque baie).
   */
  zone_bruit?: ZoneBruit | null;
}

/** « expo_BR » (§5.13.2 ; fiche d'application §5.11) : classe d'exposition au bruit d'une baie. */
export type ExpoBr = "br1" | "br2" | "br3";

/** Th-BCE 2020 §3.2.2 — paramètres d'intégration d'une surface exposée au rayonnement. */
export interface Exposition {
  /** « α » : azimut, en degrés (0 : sud ; 90 : ouest ; 180 : nord ; 270 : est). */
  azimut_deg: number;
  /** « hpb » : hauteur de la paroi, en m. */
  hpb_m?: number | null;
  /**
   * « β » : inclinaison, en degrés (0 : horizontale vers le haut ; 90 : verticale ;
   * 180 : horizontale vers le bas).
   */
  inclinaison_deg: number;
  /** « lpb » : largeur de la paroi, en m. */
  lpb_m?: number | null;
  /** « Id_masque » : identifiants des masques protégeant la surface (« Nmasque », 0 à 7). */
  masques?: string[];
}

/**
 * Extension d'un bâtiment existant (arrêté du 4 août 2021, articles 3 et 50-3 ; article
 * R. 172-1 du CCH ; FA03 v2).
 */
export interface ExtensionSaisie {
  /**
   * Somme des surfaces de référence des autres extensions du même bâtiment, non décrites
   * dans ce calcul, en m² : « la surface considérée … est le cumul des surfaces des
   * différentes extensions d'un même bâtiment » (article 50-3, I). Absente : 0 (seule
   * l'extension décrite).
   */
  sref_autres_extensions_m2?: number;
  /**
   * Surface de référence des locaux existants avant travaux (y compris les locaux rénovés),
   * en m² : SHAB ou SU selon l'usage, comme la `Sref` (FA03). Elle fixe la part de
   * l'extension (seuils de 30 %). **Obligatoire** si l'extension comprend une zone d'usage
   * autre que maison individuelle ; sans effet pour une extension de maison.
   */
  sref_existant_m2?: number | null;
  /**
   * Extension verticale (« surélévation ») : étage ajouté, ou combles existants d'une maison
   * aménagés en surélevant le faîtage d'au moins 1,8 m (fiche 4.4 ; la FA03 dit « de plus de
   * 1,8 mètre »). Sous le régime intermédiaire, le `DH_max` d'une surélévation climatisée de
   * maison en H2d ou H3 est porté à 1 400 °C.h (article 50-3, II, en vigueur depuis le 1er
   * janvier 2023) ; pour un permis déposé à partir du 1er juillet 2026, les surélévations à
   * usage autre que de maison individuelle ont leurs propres seuils (article 50-3, I,
   * arrêté du 18 mars 2026). Absent : extension horizontale.
   */
  surelevation?: boolean;
}

/**
 * Facteurs solaires et lumineux d'une baie dans un état de protection (§5.10.2).
 *
 * Trois grandeurs ont une valeur par défaut donnée par la méthode (§5.10.3.6.3) et sont
 * facultatives : les facteurs solaires « -c » (éq. 215 et 216) et `tlid` (éq. 218). Toute
 * valeur par défaut appliquée est listée dans les résultats (`valeurs_par_defaut`).
 */
export interface FacteursBaie {
  /**
   * « Sw1-c » : composante courte longueur d'onde (directe), calcul du Cep.
   *
   * Les trois composantes « -c » se saisissent ensemble ou pas du tout. Absentes, la valeur
   * par défaut du texte s'applique au facteur solaire global `Sw-c = Sw1-c + Sw2-c + Sw3-c` :
   * sans protection mobile, `Swsp-c = Swsp-e − 0,1` (éq. 215) ; avec protection mobile,
   * `Swap-c = Swap-e` (éq. 216). Le texte renvoie aux règles Th-S pour la décomposition en
   * trois composantes : le moteur garde les proportions des composantes « -e » (question
   * Q30).
   */
  sw1_c?: number | null;
  /** « Sw1-e » : composante courte longueur d'onde (directe), calcul de DH. */
  sw1_e: number;
  /** « Sw2-c » : composante grande longueur d'onde et convective, calcul du Cep. Voir `sw1_c`. */
  sw2_c?: number | null;
  /** « Sw2-e » : composante grande longueur d'onde et convective, calcul de DH. */
  sw2_e: number;
  /**
   * « Sw3-c » : composante liée à la lame d'air intérieure ventilée, calcul du Cep. Voir
   * `sw1_c`.
   */
  sw3_c?: number | null;
  /** « Sw3-e » : composante liée à la lame d'air intérieure ventilée, calcul de DH. */
  sw3_e: number;
  /** « Tli » : facteur de transmission lumineuse global. */
  tli: number;
  /**
   * « Tlid » : transmission lumineuse du flux direct transmis sous forme diffuse. Absente :
   * 0, valeur par défaut du texte (éq. 218).
   */
  tlid?: number | null;
}

/**
 * Famille d'usage des tableaux de la FA03 v2 (article R. 172-1 du CCH, I et III ; article
 * 50-3, I) : usage principal de la partie calculée (Q86).
 */
export type FamilleUsage = "maison_individuelle" | "collectif_bureaux_enseignement" | "tertiaire_specifique_ou_industriel";

/** Fonction d'une chaudière ou d'un réseau de chaleur (« idfougen » 1, 3 ou 4). */
export type FonctionChaleur = "chauffage" | "ecs" | "chauffage_ecs";

/** « idfougen » d'un générateur direct à effet Joule (§8.18.3.1). */
export type FonctionEffetJoule = "chauffage" | "ecs";

/** Fonction d'un réseau de distribution (« idfonction », §8.7.2, §8.9.2). */
export type FonctionReseau = "chauffage" | "refroidissement";

/** « Fonc_compr » (§8.23.3.7.4). */
export type FonctionnementCompresseurSaisi = "puissance_variable" | "tout_ou_rien";

/** « LRcontmin », « CcpLRcontmin » et « Statut_Fonctionnement_Continu » (§8.23.3.6). */
export interface FonctionnementContinu {
  /**
   * « CcpLRcontmin » : rapport du COP (ou de l'EER) au taux de charge `LRcontmin` sur le COP
   * à pleine charge.
   */
  ccp_lr_contmin: number;
  /** « LRcontmin » : taux de charge minimal en fonctionnement continu, de 0 à 1. */
  lr_contmin: number;
  /**
   * Statut : pour des valeurs justifiées, `LRcontmin` est augmenté de 0,05 et
   * `CcpLRcontmin` multiplié par 0,9.
   */
  statut: StatutMesure;
}

/** Forfait de consommation de refroidissement d'un groupe (§13.3, éq. 2493). */
export interface ForfaitRefroidissementGroupe {
  /** Le même, multiplié par la surface de référence du groupe, en kWh. */
  cef_kwh: number;
  /** « Cef_fr_elec_refroidissement,g », en kWhef/(m².an) de surface de référence du groupe. */
  cef_kwh_m2: number;
  /** « DHg » du groupe, en °C.h. */
  dh_c_h: number;
  /** Identifiant du groupe. */
  groupe: string;
  /**
   * Seuil haut retenu (`DH_max` du groupe), en °C.h ; absent sans catégorie de contraintes
   * extérieures saisie (DH non plafonné, Q54).
   */
  seuil_haut_c_h: number | null;
  /** Identifiant de la zone. */
  zone: string;
}

/** Générateur de base d'un ballon et sa régulation (§9.10.2 et §9.14.3.1). */
export interface GenerateurBallon {
  /**
   * « Δθ » : hystérésis du thermostat, en K. Absente : 2 K, valeur par défaut du
   * §9.10.3.2.2, signalée dans les résultats.
   */
  delta_theta_k?: number | null;
  /**
   * Identifiant d'un générateur de la génération assurant l'ECS : générateur à effet Joule
   * (résistance) ou générateur thermodynamique pourvu d'un mode `ecs`.
   */
  generateur: string;
  /**
   * « hrelech » : hauteur relative de l'échangeur à partir du bas de sa zone, de 0 à 1.
   * Requise pour un générateur thermodynamique (elle fixe la température vue par son
   * condenseur, éq. 1781 à 1785) ; sans objet pour une résistance.
   */
  h_rel_ech?: number | null;
  /** « typegest » : programmation du thermostat. */
  programmation: ProgrammationBallon;
  /**
   * « zreg » : numéro de la zone du ballon, de 1 (bas) à 4 (haut), qui porte la sonde de
   * régulation.
   */
  z_reg: number;
}

/** Générateur d'une génération (§8.17.2, tableau 111). */
export interface GenerateurSaisi {
  /** Identifiant du générateur, unique dans la génération. */
  id: string;
  /** « idtype » : nature du générateur et ses caractéristiques. */
  nature: NatureGenerateur;
  /**
   * « idpriorite_ch » : rang du générateur en chauffage dans une gestion en cascade (1 : le
   * premier sollicité). Requis en cascade si plusieurs générateurs chauffent.
   */
  priorite_chauffage?: number | null;
  /** « idpriorite_ecs » : de même en ECS. */
  priorite_ecs?: number | null;
  /** « idpriorite_fr » : de même en refroidissement. */
  priorite_refroidissement?: number | null;
  /** « Rdim » (« Ratdim ») : nombre de générateurs identiques (au moins 1). */
  rdim: number;
}

/**
 * Th-BCE 2020 §8.14, §8.17.2 et §8.4.2 — génération : ensemble de générateurs qui reçoit les
 * demandes d'un ou de plusieurs réseaux de distribution intergroupes de chauffage, de
 * refroidissement ou d'ECS, et les répartit entre ses générateurs.
 */
export interface Generation {
  /**
   * Chauffe-eau solaires individuels (§9.14 et §9.15, kits CESI §9.23) : ballon dont le
   * générateur de base est une boucle solaire, avec un appoint intégré, un appoint séparé
   * instantané ou sans appoint. Comme les autres assemblages avec stockage, ils sont les
   * premiers sollicités pour l'ECS (§8.17.3.6).
   */
  chauffe_eau_solaires?: ChauffeEauSolaire[];
  /**
   * Productions solaires collectives à appoints décentralisés du logement collectif
   * (fiches 9.16 et 9.17) : CESCI (capteurs collectifs, ballons individuels à appoint
   * intégré ou séparé) et CESCAI (capteurs et ballon solaire collectifs, ballons
   * individuels à appoint intégré). Une génération en porte au plus une, qui assure toute son
   * ECS.
   */
  chauffe_eau_solaires_collectifs?: ChauffeEauSolaireCollectif[];
  /**
   * Chauffe-eau thermodynamiques décrits par les résultats de leur essai selon la norme
   * NF EN 16147 (§9.22) : le moteur en déduit le ballon et la pompe à chaleur d'un
   * assemblage « base seule ».
   */
  chauffe_eau_thermodynamiques?: ChauffeEauThermodynamique[];
  /**
   * Générateurs de la génération (tableau 111). La génération doit pouvoir assurer chacun
   * des postes de ses réseaux (§8.17.3.1) ; elle compte au moins un générateur ou un
   * chauffe-eau thermodynamique.
   */
  generateurs?: GenerateurSaisi[];
  /**
   * « idgestion_ch » et « θwm_ch » (§8.17.3.5.10.1) : gestion de la température de
   * fonctionnement en chauffage. Requise si la génération alimente des réseaux hydrauliques
   * de chauffage ; sans objet pour une génération sur air ambiant (réseaux fictifs).
   */
  gestion_temperature_chauffage?: GestionTemperatureGeneration | null;
  /** « idgestion_fr » et « θwm_fr » : de même en refroidissement. */
  gestion_temperature_refroidissement?: GestionTemperatureGeneration | null;
  /** Identifiant de la génération, unique dans le bâtiment. */
  id: string;
  /**
   * « idpos » (§8.17.2) : emplacement de la génération, en ou hors volume chauffé. Il fixe
   * la température d'ambiance des générateurs et des ballons (§8.17.3.5.8) et le caractère
   * récupérable de leurs pertes (éq. 1058, fiche 11.1).
   */
  position: PositionGeneration;
  /**
   * « idtype_priorite » (§8.17.1) : mode de gestion des priorités entre les générateurs.
   * Requis dès que deux générateurs assurent le même poste ; sans objet sinon.
   */
  priorite?: TypePriorite | null;
  /**
   * « idraccord_reseau » (§8.4.2) : type de raccordement de la génération aux réseaux de
   * distribution. `permanent` : les groupes desservis suivent la saison de fonctionnement de
   * la génération (éq. 840) ; `avec_isolement` : chaque groupe garde ses propres saisons
   * (éq. 841).
   */
  raccordement_reseaux: RaccordementReseaux;
  /**
   * Assemblages de production d'ECS avec stockage (§9.2 et §9.14) : un ballon, un générateur
   * de base de la génération et, le cas échéant, un appoint intégré. Chaque assemblage est
   * le premier sollicité pour l'ECS (§8.17.3.6).
   */
  stockages?: StockageSaisi[];
  /**
   * Systèmes solaires combinés (fiches 9.18 à 9.20) : un ballon à quatre zones chauffé par
   * une boucle solaire, qui assure l'ECS et une partie du chauffage par l'échangeur d'une
   * boucle de chauffage en zone 2, avec un appoint intégré en zone 3. Premiers sollicités pour
   * l'ECS et pour le chauffage.
   */
  systemes_solaires_combines?: SystemeSolaireCombine[];
}

/**
 * « Gest_ecl,l » (§7.1.2 et §7.1.3.6.1) : mode de commande de l'éclairage artificiel d'un
 * local.
 */
export type GestEcl = "permanent" | "interrupteur_manuel" | "interrupteur_et_horloge" | "marche_et_arret_automatiques" | "marche_manuelle_arret_automatique";

/** « Type-GPM » (§5.9.2) : gestion d'un volet ou d'un store enroulable. */
export type GestionPm = "automatique_avec_derogation" | "manuelle_non_motorisee" | "manuelle_motorisee";

/**
 * « idgestion » et « θwm » (§8.17.3.5.10.1) : température de fonctionnement d'une génération
 * reliée à des réseaux hydrauliques.
 */
export type GestionTemperatureGeneration = {
  constante: {
    /** « θwm » : température moyenne de fonctionnement, en °C (de 0 à 100). */
    theta_wm_c: number;
  };
} | "reseaux";

/** « GestVCV » (§8.1.2). */
export type GestionVentilateurs = "manuelle" | "automatique_permanente" | "automatique_avec_arret";

/**
 * « Grad_ecl,l » (§7.1.2 et §7.1.3.6.1) : mode de gestion de l'éclairage artificiel d'un local
 * selon la lumière naturelle.
 */
export type GradEcl = "aucune" | "manuelle" | "gradation_automatique" | "allumage_et_extinction_automatiques" | "extinction_automatique";

/**
 * Niveau « groupe » (Th-BCE 2020 §5.7) : local ou ensemble de locaux dont l'air est supposé
 * homogène en température et en humidité (§5.21.1).
 */
export interface Groupe {
  /** Baies vitrées (§5.8, 5.10). */
  baies?: Baie[];
  /**
   * Bouches et conduits de ventilation du groupe (§6.2) : débits d'hygiène de la ventilation
   * installée. Au moins une en habitation : le calcul du Bbio en tire les débits du système
   * de ventilation conventionnel (§6.1 et §6.2.3.1.3).
   */
  bouches_conduits?: BoucheConduit[];
  /** « Categorie_CE1_CE2 » (§13.1.2). */
  categorie_ce1_ce2: CategorieCe;
  /**
   * Réseaux de distribution du groupe de chaud et de froid (§8.7 et §8.8), auxquels les
   * émetteurs sont rattachés.
   */
  distributions?: DistributionGroupe[];
  /**
   * Émetteurs de chaud et de froid du groupe (§8.1 et §8.2). Au moins un émetteur de
   * chauffage en habitation.
   */
  emetteurs?: Emetteur[];
  /** Émetteurs d'ECS équivalents du groupe (§9.5 à 9.7). Au moins un en habitation. */
  emetteurs_ecs?: EmetteurEcs[];
  /**
   * Entrées d'air du groupe (§5.6.3.2), décrites par leur module total. Absent : le groupe
   * n'a pas d'entrée d'air.
   */
  entrees_air?: EntreesAir | null;
  /**
   * « Httf » : hauteur pour le tirage thermique, en m (§5.13.3.2.2). Absent : 1,5 m, valeur
   * du cas général, signalée dans les résultats. À saisir seulement en maison individuelle
   * quand la différence d'altitude entre le bas de l'ouvrant le plus bas et le haut de
   * l'ouvrant le plus haut dépasse 4 m, dans la limite de 15 m.
   */
  h_ttf_m?: number | null;
  /**
   * « iclim » : vrai si le groupe est climatisé, c'est-à-dire refroidi par un émetteur de
   * froid (§8.1.2, §8.4.2). Requis en habitation. Employé par le calcul des consommations
   * (Th-C) : sans climatisation, pas de saison de refroidissement (§4.6.3.2) ni de besoin de
   * froid (éq. 807). Le calcul du Bbio l'ignore (décision Q38 point 3) et le calcul Th-D
   * passe tout groupe en non climatisé (§13.5.3.1).
   */
  iclim?: boolean | null;
  /** Identifiant du groupe. */
  id: string;
  /** Inertie thermique du groupe (§5.21.2). */
  inertie: Inertie;
  /**
   * Locaux du groupe au sens de l'éclairage (§7.1), pour les usages autres que la maison
   * individuelle ou accolée et le logement collectif, dont l'éclairage est conventionnel :
   * ne pas les saisir en habitation. Requis dans les autres usages ; la somme des
   * `ratio_local` vaut 1.
   */
  locaux?: LocalEclairage[];
  /** Parois opaques (§5.15 à 5.17). */
  parois_opaques?: ParoiOpaque[];
  /** Ponts thermiques (§5.18 à 5.20). */
  ponts_thermiques?: PontThermique[];
  /** Programmation des relances du groupe (§8.5). Requise en habitation. */
  programmation?: Programmation | null;
  /**
   * « SHAB » : surface habitable, en m². Requise pour les usages d'habitation, dont elle est
   * la surface de référence (§13.1.3, éq. 2375).
   */
  shab_m2?: number | null;
  /**
   * « Agr » : surface utile du groupe, en m² (§4.1.2). Requise pour tous les usages : elle
   * répartit les apports internes entre groupes. Hors habitation, c'est aussi la surface de
   * référence « SU » (§13.1.3, éq. 2376 ; décision Q28). En maison individuelle, on saisit en
   * pratique `su_m2 = shab_m2`. En logement collectif, le calcul en déduit sa propre surface
   * habitable, `Rat_hab · Az` (éq. 33), distincte de la SHAB saisie comme surface de référence.
   */
  su_m2?: number | null;
  /**
   * « δtrav_surv » : caractère traversant du groupe au sens de la surventilation par
   * ouverture des baies, c'est-à-dire la possibilité d'ouvrir les portes intérieures entre
   * deux façades d'orientations différentes (§5.13.3.2.1, tableau 43). Conventionnel en
   * maison individuelle (traversant) : ne pas le saisir. Requis en logement collectif si le
   * groupe a des baies ouvrables.
   */
  traversant?: boolean | null;
  /**
   * « V » : volume du groupe, en m³ (§8.3.2). Requis : le bilan hydrique du groupe en tire la
   * masse d'air dans laquelle se diluent les apports d'humidité (§8.3.3, éq. 831). Le texte ne
   * lui donne pas de valeur par défaut.
   */
  v_m3?: number | null;
}

/**
 * Th-BCE 2020 §5.21.2 et §5.21.3 — inertie du groupe sur trois horizons.
 *
 * `Ams_surf` et `Ama_surf` « sont saisies dans tous les cas » (§5.21.3).
 */
export interface Inertie {
  /** « Ama_surf » : surface d'échange équivalente au sens de l'inertie annuelle, en m²/m². */
  ama_surf_m2_m2: number;
  /** « Ams_surf » : surface d'échange équivalente au sens de l'inertie séquentielle, en m²/m². */
  ams_surf_m2_m2: number;
  /** Inertie annuelle (« Typeinertie_ann »). */
  annuelle: InertieAnnuelle;
  /** Inertie quotidienne (« Typeinertie_quo », tableau 53). */
  quotidienne: InertieQuotidienne;
  /** Inertie séquentielle (« Typeinertie_seq », tableau 54). */
  sequentielle: InertieSequentielle;
}

/** « Typeinertie_ann » (§5.21.3). */
export type InertieAnnuelle = {
  personnalisee: {
    /** « Cma_surf ». */
    cma_surf_kj_m2k: number;
  };
} | "defaut";

/** « Typeinertie_quo » (§5.21.3, tableau 53). */
export type InertieQuotidienne = {
  personnalisee: {
    /** « Amq_surf ». */
    amq_surf_m2_m2: number;
    /** « Cmq_surf ». */
    cmq_surf_kj_m2k: number;
  };
} | "tres_legere" | "legere" | "moyenne" | "lourde" | "tres_lourde";

/** « Typeinertie_seq » (§5.21.3, tableau 54). */
export type InertieSequentielle = {
  personnalisee: {
    /** « Cms_surf ». */
    cms_surf_kj_m2k: number;
  };
} | "defaut" | "tres_legere" | "legere" | "moyenne" | "lourde";

/**
 * Une infrastructure de transports terrestres vue d'une baie (annexe de l'article R. 172-4,
 * chapitre V, partie I, points 2 et 3 ; fiche 5.11).
 */
export interface InfrastructureBruit {
  /**
   * Catégorie de l'infrastructure (1 à 5), donnée par l'arrêté préfectoral de classement des
   * voies : donnée à saisir.
   */
  categorie: CategorieInfrastructure;
  /**
   * Distance de la façade à l'infrastructure, en m : plus courte distance au bord extérieur
   * de la chaussée la plus proche, en projection en plan (fiche 5.11.4).
   */
  distance_m: number;
  /** Vue de l'infrastructure depuis la baie. */
  vue: VueInfrastructure;
}

/** Th-BCE 2020 §12.2.2 — installation photovoltaïque intégrée au bâtiment. */
export interface InstallationPv {
  /**
   * « CT » : confinement thermique de la face arrière des modules (§12.3.3.2). Valeur par
   * défaut : face arrière confinée.
   */
  confinement?: ConfinementPv;
  /**
   * « α », « β », « lpb », « hpb » et masques (`Az`, `γi`, `dE`, `hpE` : masques du catalogue,
   * §3.2) : orientation du plan des modules. L'inclinaison va de 0 à 90°. « alb » vaut 0,2
   * (§3.2.2).
   */
  exposition: Exposition;
  /** Identifiant de l'installation. */
  id: string;
  /** Modules : technologie, puissance crête, surface et performances (§12.3.3.1). */
  modules: ModulesPv;
  /** « N » : nombre de modules du champ, au moins 1. */
  nb_modules: number;
  /**
   * Onduleur (§12.4). Sans saisie : aucune information de rendement, puissance nominale par
   * défaut.
   */
  onduleur?: OnduleurPv;
}

/** « Isouf » (§6.2.2) : sens d'un circuit de ventilation. */
export type Isouf = "reprise" | "soufflage";

/**
 * Kit CESI caractérisé selon la norme NF EN 12976 (« TypeCES = 1 », §9.23.3.2.2, tableau 308).
 * Le §9.23.3.2.2 le traite comme un « ballon base seule solaire avec appoint intégré » : son
 * appoint est `integre`.
 */
export interface KitCesi {
  /** « AC* » : superficie de captage équivalente, en m². */
  ac_etoile_m2: number;
  /**
   * « faux » : fraction effective du ballon concernée par l'appoint, strictement entre 0 et
   * 1 (requise avec un appoint intégré, sans objet sinon).
   */
  f_aux?: number | null;
  /**
   * « Kθ » : facteur d'angle d'incidence, de 0 à 1, sortie de l'outil IdCESI
   * (§9.23.3.2.3). Requis : le texte ne lui donne aucune valeur (lecture Q52).
   */
  k_theta: number;
  /** « Pnp » : puissance nominale de la pompe, en W. */
  p_np_w: number;
  /** « A » : superficie d'entrée des capteurs, en m². */
  surface_m2: number;
  /** « θmax » : température maximale du ballon, en °C (de 55 à 100). */
  theta_max_c: number;
  /** « UC* » : coefficient de pertes de captage rapporté à `AC*`, en W/(m².K). */
  uc_etoile_w_m2k: number;
  /** « US » : coefficient de pertes de stockage, en W/K. */
  us_w_k: number;
  /** « Vtot » : volume du ballon, en L. */
  v_tot_l: number;
}

/** Une ligne du bilan : consommation importée d'un poste en une énergie, et sa conversion. */
export interface LigneBilanEnergie {
  /** Quantité importée, en kWh d'énergie finale (PCI pour les combustibles). */
  cef_kwh: number;
  /** Contribution au Cep, en kWhep/(m².an). */
  cep_kwhep_m2: number;
  /** Contribution au Cep,nr, en kWhep/(m².an). */
  cep_nr_kwhep_m2: number;
  /** « CoefEP ». */
  coef_ep: number;
  /** « CoefEPnr ». */
  coef_ep_nr: number;
  /**
   * Énergie : `electricite`, `gaz_naturel`, `gpl`, `fioul`, `charbon`, `bois_buches`,
   * `bois_granules`, `bois_plaquettes`, `reseau_chaleur` ou `reseau_froid`, suivi pour un
   * réseau de son identifiant de l'annexe 7 (`reseau_chaleur:0102C`).
   */
  energie: string;
  /**
   * Facteur d'émission, en kg éq. CO2/kWh (article 10) ; absent quand il n'est pas connu
   * (combustible d'un poêle non saisi).
   */
  facteur_emission_kg_kwh: number | null;
  /** Contribution à l'Ic énergie, en kg éq. CO2/m². */
  ic_energie_kgco2_m2: number | null;
  /**
   * Poste : `chauffage`, `refroidissement`, `ecs`, `eclairage`,
   * `auxiliaires_ventilation`, `auxiliaires_distribution` ou `deplacements`.
   */
  poste: string;
}

/** « Lim_Theta » (§8.23.3.7.3) : limites de fonctionnement d'un mode. */
export interface LimitesPac {
  /** Combinaison des deux limites. */
  combinaison: CombinaisonLimites;
  /**
   * Limite sur la température amont, en °C : minimale en chauffage et en ECS
   * (« θmin_am »), maximale en refroidissement (« θmax_am »).
   */
  theta_amont_c: number;
  /**
   * Limite sur la température aval, en °C : maximale en chauffage et en ECS
   * (« θmax_av »), minimale en refroidissement (« θmin_av »).
   */
  theta_aval_c: number;
}

/**
 * Th-BCE 2020 §7.1 — local d'un groupe au sens de l'éclairage : « un local a une puissance
 * d'éclairage artificiel spécifique » et « un seul mode de mise en marche et d'extinction »
 * (§7.1.1). Des locaux de même type et de même système d'éclairage se regroupent.
 */
export interface LocalEclairage {
  /**
   * Système d'éclairage du projet (§7.1.3.2). Le calcul du Bbio l'ignore : il emploie une
   * puissance et une gestion conventionnelles (§7.1.3.1).
   */
  eclairage: EclairageLocal;
  /** Identifiant, unique dans le groupe. */
  id: string;
  /**
   * « Ratioécl_nat,l » : part du local qui a un accès effectif à la lumière naturelle, de 0
   * à 1 (§7.1.3.6.1) : parties situées à moins de `2,5 × (hLi − hTa)` d'une baie non
   * horizontale (`hLi` : hauteur du linteau, `hTa` : hauteur du plan de travail), sous
   * réserve, dans un local plus profond, que leurs luminaires soient commandés
   * indépendamment ; 0 pour un local sans baie.
   */
  ratio_ecl_nat: number;
  /** « Ratiolocal,l » : part de la surface du local dans la surface utile du groupe, de 0 à 1. */
  ratio_local: number;
  /**
   * « type_local,l » : type de local, parmi ceux que le tableau 78 donne à l'usage de la
   * zone ; il fixe le coefficient `C1` et l'éclairement de référence `Eiref`.
   */
  type_local: TypeLocal;
}

/** Th-BCE 2020 §3.1.2 et §5.1.2 — paramètres d'intégration du climat et du bâtiment. */
export interface Localisation {
  /** « Alt » : altitude du projet, en m. */
  alt_m: number;
  /** « département ». Fixe la zone climatique et la température de base. */
  departement: Departement;
  /** « Dmer » : distance à la mer, en km. */
  dmer_km: number;
  /** « Durb » : distance à l'îlot urbain, en km. */
  durb_km: number;
  /**
   * Zone climatique saisie explicitement. Facultative : le moteur la déduit du département
   * (arrêté du 26 octobre 2010, annexe I). Une valeur qui contredit le département est
   * conservée et signalée par un avertissement.
   */
  zone_climatique?: ZoneClimatique | null;
}

/**
 * Th-BCE 2020 §3.2 — masques solaires (identifiants du texte : `E_AZ`, `E_Az_cad`, `E_LV`,
 * `vd`, `vg`, `h`).
 *
 * Les masques proches ne portent que sur les composants verticaux (§3.2.3) ; les masques
 * lointains portent sur toute inclinaison.
 */
export type Masque = {
  lointain_azimutal: {
    /**
     * « γi » : hauteur de l'horizon vu du centre du composant, en degrés (0 à 90), pour
     * chacune des 36 tranches de 10° (§3.2.3). Les tranches sont repérées par rapport à la
     * normale au composant : la tranche 1 commence à l'opposé de la normale (Φ = −180°) et
     * les tranches 18 et 19 encadrent la normale (figure 10).
     */
    gamma_i_deg: number[];
  };
} | {
  lointain_arbres_caducs: {
    /** « γi », comme pour le masque lointain azimutal. */
    gamma_i_deg: number[];
  };
} | {
  lointain_plan_vertical: {
    /** « dE » : distance entre le masque et la paroi, en m. */
    de_m: number;
    /** « hpE » : hauteur du masque par rapport au centre de la paroi, en m. */
    hpe_m: number;
  };
} | {
  proche_vertical_droit: {
    /** « dpd » : distance entre la paroi et le masque, en m. */
    dpd_m: number;
    /** « dvd » : profondeur du masque, en m. */
    dvd_m: number;
  };
} | {
  proche_vertical_gauche: {
    /** « dpg » : distance entre la paroi et le masque, en m. */
    dpg_m: number;
    /** « dvg » : profondeur du masque, en m. */
    dvg_m: number;
  };
} | {
  proche_horizontal: {
    /** « dhm » : profondeur du masque, en m. */
    dhm_m: number;
    /** « dhp » : distance entre la paroi et le masque, en m. */
    dhp_m: number;
  };
};

/** Masque du catalogue. */
export interface MasqueDefinition {
  /** Identifiant du masque. */
  id: string;
  /** Géométrie du masque. */
  masque: Masque;
}

/** Mode chauffage d'un générateur thermodynamique (§8.23.3.3). */
export interface ModeChauffagePac {
  /**
   * « LRcontmin » et « CcpLRcontmin » d'un compresseur à puissance variable. Absent : 0,4 et
   * 1 (éq. 1280 et 1281), signalés dans les résultats. Sans objet en tout ou rien.
   */
  fonctionnement_continu?: FonctionnementContinu | null;
  /**
   * « Lim_Theta » : limites de fonctionnement — température aval maximale et température
   * amont minimale. Absent : pas de limite.
   */
  limites?: LimitesPac | null;
  /** Performances à pleine charge. */
  performances: PerformancesPac;
  /** « Sys_thermo_Ch » : technologie en chauffage (tableau 145). */
  technologie: TechnologieChauffagePac;
  /** « Typo_emetteur_ch » : inertie du système d'émission principal (tableau 216). */
  typo_emetteur: TypoEmetteur;
}

/** Mode ECS d'un générateur thermodynamique (§8.23.3.4). */
export interface ModeEcsPac {
  /**
   * « Lim_Theta » : température aval maximale et température amont minimale. Absent : pas
   * de limite.
   */
  limites?: LimitesPac | null;
  /** Performances à pleine charge. */
  performances: PerformancesPac;
  /** « Sys_thermo_Ecs » : technologie en ECS (tableau 147). */
  technologie: TechnologieEcsPac;
}

/** Mode refroidissement d'un générateur thermodynamique (§8.23.3.5). */
export interface ModeRefroidissementPac {
  /** « LRcontmin » et « CcpLRcontmin » d'un compresseur à puissance variable. */
  fonctionnement_continu?: FonctionnementContinu | null;
  /**
   * « Lim_Theta » : limites de fonctionnement — température aval minimale et température
   * amont maximale. Absent : pas de limite.
   */
  limites?: LimitesPac | null;
  /** Performances à pleine charge ; `performance` y désigne l'EER. */
  performances: PerformancesPac;
  /** « Sys_thermo_Fr » : technologie en refroidissement (tableau 145). */
  technologie: TechnologieRefroidissementPac;
  /** « Typo_emetteur_fr » : inertie du système d'émission principal (tableau 216). */
  typo_emetteur: TypoEmetteur;
}

/** Th-BCE 2020 §12.3.2 — caractéristiques d'un module. */
export interface ModulesPv {
  /** « Pc », « μ » et « NOCT » selon leur origine (§12.3.3.1). */
  performances: PerformancesPv;
  /** « S » : surface ensoleillée d'un module en œuvre, en m². */
  s_m2: number;
  /** « Type_Techno_Capteur ». */
  technologie: TechnologiePv;
}

/** Nature d'un ballon pour ses pertes par défaut (tableau 282). */
export type NatureBallonSaisie = "effet_joule_horizontal" | "effet_joule_vertical" | "autre";

/** « isolarise » : espace tampon solarisé ou non (§5.4.1). */
export type NatureEspaceTampon = {
  non_solarise: EspaceTamponNonSolarise;
} | {
  solarise: EspaceTamponSolarise;
};

/** « idtype » (§8.17.3.2, tableau 111) : nature d'un générateur. */
export type NatureGenerateur = {
  effet_joule: EffetJouleSaisi;
} | {
  thermodynamique_electrique: ThermodynamiqueElectriqueSaisi;
} | {
  chaudiere: ChaudiereSaisie;
} | {
  poele_ou_insert: PoeleSaisi;
} | {
  reseau_de_chaleur_ou_de_froid: ReseauChaleurFroidSaisi;
} | "autre_generateur_a_combustion" | "thermodynamique_gaz" | "boucle_d_eau_ou_drv" | "cogeneration" | "pac_hybride";

/** Nature des travaux pour le choix du régime d'exigences (fiche 4.4 ; FA03 v2). */
export type NatureTravaux = "construction" | "extension" | "surelevation";

/** Th-BCE 2020 §12.4.2 — onduleur. */
export interface OnduleurPv {
  /**
   * « PAC NOM » : puissance nominale alternative, en W. Sans saisie : `0,8 × N × Pc`
   * (§12.4.3), avec `Pc` la valeur de calcul.
   */
  p_ac_nom_w?: number | null;
  /** Information de rendement disponible (cas n° 1 à 3 du §12.4.3). */
  rendement?: RendementOnduleur;
}

/** Provenance des données météo, reportée telle quelle dans les résultats. */
export type OrigineMeteo = {
  type: "reglementaire_re2020";
} | {
  /** Scénario climatique, par exemple « +2,7 °C avec vague de chaleur ». */
  scenario: string;
  type: "ademe_prospectif";
} | {
  /** Description libre de la source. */
  description: string;
  type: "autre";
};

/**
 * Th-BCE 2020 §5.13.3.2.3 — ouvrant d'une baie. Renseigner soit `r_ouv_max`, soit
 * `type_ouvrant`.
 */
export interface Ouvrant {
  /**
   * « Rouvmax,b » : rapport de la surface maximale d'ouverture à la surface en tableau de la
   * baie, parties fixes comprises, entre 0 et 1.
   */
  r_ouv_max?: number | null;
  /**
   * Type d'ouvrant. Sans `r_ouv_max`, le moteur applique la valeur par défaut du tableau 44
   * et la signale dans les résultats.
   */
  type_ouvrant?: TypeOuvrant | null;
}

/**
 * Données de la fixation des exigences d'un bâtiment (annexe de l'article R. 172-4 du CCH).
 * Elles s'appliquent à toutes ses zones.
 */
export interface ParametresExigences {
  /**
   * Année de dépôt de la demande de permis de construire (2022 ou après) : fixe
   * `Icénergie_maxmoyen`. Les dérogations datées du texte sont appliquées à l'année : « avant
   * le 31 décembre 2023 » (maisons, raccordement au gaz) est lu « année ≤ 2023 », « avant le
   * 31 décembre 2027 » (réseau classé) « année ≤ 2027 » (Q54).
   */
  annee_depot_permis: number;
  /**
   * Catégorie de contraintes extérieures du bâtiment (chapitre V de l'annexe, partie II).
   * Elle module `Cep_max`, `Cep,nr_max` et `Icénergie_max` (`Mccat`), `Bbio_max` (`Mbbruit`,
   * catégorie 3) et fixe `DH_max`.
   *
   * Un local est de catégorie 2 si, cumulativement, il est climatisé, ses baies sont exposées
   * au bruit BR2 ou BR3 et le bâtiment est construit en zone H2d ou H3 à moins de 400 m
   * d'altitude ; une zone est de catégorie 2 si tous ses locaux autres qu'à occupation
   * passagère le sont. La catégorie 3 est réservée aux locaux climatisés de certains usages
   * non résidentiels (voir [`CategorieContraintes::Categorie3`]). La validation refuse une
   * catégorie que ces règles excluent ; la catégorie reste saisie, faute de description des
   * locaux dans l'entrée. Elle vaut pour toutes les zones et tous les groupes, sauf ceux qui
   * ont la leur dans `zones` (Q66).
   */
  categorie_contraintes_exterieures: CategorieContraintes;
  /**
   * Le bâtiment décrit est une **extension** (partie nouvelle) d'un bâtiment existant, ou
   * une surélévation (fiche 4.4 de l'annexe III ; arrêté du 4 août 2021, articles 3, 50-2 et
   * 50-3 ; fiche d'application FA03 v2 ; Q86 à Q88). Absent : construction neuve entière.
   *
   * Règles de saisie : l'extension se décrit **seule**, comme un bâtiment ; ses zones ne
   * contiennent aucun local existant ; une paroi entre l'extension et un local chauffé de la
   * partie existante est mitoyenne (ses deux faces donnent sur des locaux chauffés, fiche
   * 4.2.2) et ne se saisit pas ; une paroi sur un local existant non chauffé donne sur un
   * espace tampon. L'usage d'une extension sans locaux propres est celui du bâtiment existant
   * (annexe I, « Usage d'une partie de bâtiment »). Les équipements de la partie existante qui
   * la desservent se décrivent avec leurs valeurs initiales ou les valeurs par défaut de la
   * réglementation de l'existant, leur puissance nominale au prorata des besoins (à défaut,
   * des surfaces) desservis (FA03, annexe II).
   *
   * Le `nb_logements` d'une zone d'extension compte les logements qu'elle contient en tout
   * ou en partie, logements existants agrandis compris (au moins 1 : une extension de
   * logements collectifs sans logement nouveau compte les logements qu'elle agrandit ; Q87).
   *
   * Les exigences de l'article R. 172-4 (`Bbio_max`, `Cep_max`… calculées sur la `Sref` et
   * les logements de l'extension) restent calculées et vérifiées ; le régime d'exigences
   * alternatives ouvert par la surface et l'usage (article 50-3) est restitué à côté
   * (`regime_alternatif`).
   */
  extension?: ExtensionSaisie | null;
  /**
   * « HSPmoy » : hauteur sous plafond moyenne des locaux d'habitation, en m (chapitre I,
   * §XIII) : modulations `MbHSP` et `McHSP`, introduites par le décret n° 2026-200 pour les
   * permis déposés à partir du 1er juillet 2026 (Q69). Obligatoire dans ce cas si le
   * bâtiment comprend une zone d'habitation (usages 1 et 2) ; sans effet pour les autres
   * usages, dont `MbHSP` et `McHSP` sont nuls, et pour les permis antérieurs ; une valeur
   * saisie hors de la plage 1,8 à 10 m reste une erreur, même sans effet.
   */
  hsp_moy_m?: number | null;
  /**
   * Mois (1 à 12) du dépôt de la demande de permis de construire, facultatif : il départage
   * les versions de l'annexe dont la date d'effet tombe en cours d'année (Q68) : 1er juillet
   * 2022 (bureaux et enseignement), 1er mai 2026 (usages 6 à 28), 1er juillet 2026
   * (modulations `MbHSP` et `McHSP` du décret n° 2026-200). **Obligatoire** pour un permis
   * déposé en 2026, et en 2022 avec une zone d'usage 3 à 5 ; facultatif les autres années,
   * où l'année suffit (Q68).
   */
  mois_depot_permis?: number | null;
  /**
   * Maison dont la parcelle est concernée par un permis d'aménager délivré, ou comprise dans
   * une zone d'aménagement concerté dont le dossier de réalisation a été approuvé, avant le
   * 1er janvier 2022 et prévoyant un raccordement au réseau de gaz : `Icénergie_maxmoyen` de
   * 280 kg éq. CO2/m² pour un permis déposé avant le 31 décembre 2023.
   */
  parcelle_raccordement_gaz?: boolean;
  /**
   * Réseau de chaleur auquel le bâtiment est raccordé classé en application de l'article
   * L. 712-1 du code de l'énergie : jusqu'en 2027, `Icénergie_maxmoyen` des années 2022 à
   * 2024.
   */
  reseau_chaleur_classe?: boolean;
  /**
   * « Scombles » : surface de plancher des combles aménagés dont la hauteur sous plafond est
   * inférieure à 1,8 m, en m² (maisons individuelles ou accolées). Absente : pas de tels
   * combles.
   */
  s_combles_m2?: number;
  /**
   * Classe d'exposition au bruit du bâtiment (chapitre V de l'annexe, partie I) : module
   * `Bbio_max` (`Mbbruit`). Elle dépend de la catégorie des infrastructures voisines
   * (arrêté préfectoral de classement) et du plan d'exposition au bruit de l'aérodrome, qui
   * sont des **données à saisir** : le moteur ne peut pas les déterminer.
   *
   * **Facultative** si les baies portent leur détermination détaillée (`bruit` de chaque
   * baie, fiche 5.11) : la classe de chaque zone en est alors déduite (point 4 de la partie
   * I). Saisie, elle prime comme saisie directe ; si la classe se déduit aussi des baies, la
   * validation vérifie leur cohérence. Elle vaut pour les zones qui n'ont pas la leur dans
   * `zones`. Absente et non déductible des baies : erreur de validation (Q200 à Q204).
   *
   * Règle de détermination : la classe de chaque baie (BR1, BR2 ou BR3) se lit dans les
   * tableaux de la partie I selon la catégorie des infrastructures de transports terrestres
   * voisines (arrêté préfectoral, article R. 571-38 du code de l'environnement), leur distance
   * à la façade et la vue qu'en a la baie (directe, partielle, masquée par des obstacles peu
   * ou très protecteurs, arrière), et selon la zone du plan d'exposition au bruit de
   * l'aérodrome (zones A à C : BR3 ; D : BR2 ; hors zone : BR1) ; la classe la plus
   * défavorable est retenue ; au-delà de la distance maximale de prise en compte (700, 500,
   * 250, 100 ou 30 m selon la catégorie 1 à 5) et hors plan d'exposition, toutes les baies
   * sont BR1. Une partie de bâtiment est BR3 si toutes ses baies le sont, BR2 si toutes sont
   * BR2 ou BR3 et l'une au moins BR2, BR1 sinon.
   */
  zone_bruit?: ZoneBruit | null;
  /**
   * Catégorie de contraintes extérieures et classe d'exposition au bruit propres à une zone,
   * quand elles diffèrent de celles du bâtiment (chapitre V : elles se définissent pour
   * chaque partie de bâtiment ou zone ; Q66). Une zone absente de la liste prend les valeurs
   * du bâtiment.
   */
  zones?: ExigencesZoneSaisie[];
}

/**
 * Th-BCE 2020 §10.3.2 et §10.4.2 — parc de stationnement associé au bâtiment. Le « Typeusage »
 * se déduit de l'usage des zones : « habitat » pour les usages 1 et 2.
 */
export interface Parking {
  /** Éclairage du parking (§10.4). Requis. */
  eclairage: EclairageParking;
  /** Identifiant du parking. */
  id: string;
  /**
   * Jours de l'année (de 1 à 365) où le parking est fermé (« NbjO(j) = 0 »). Absent : ouvert
   * tous les jours.
   */
  jours_fermes?: number[];
  /** « Net » : nombre d'étages du parking. */
  net: number;
  /**
   * « Npl » : nombre de places, éventuellement non entier : une place de deux-roues motorisé
   * compte 0,33 place (arrondi à l'entier supérieur pour l'ensemble), un deux-roues non
   * motorisé 0 (§10.3.3).
   */
  npl: number;
  /** « Type » : intérieur ou extérieur. */
  type_parking: TypeParking;
  /**
   * Ventilation forcée (« Vent » = oui). Absent : pas de ventilation forcée. Sans effet pour
   * un parking extérieur.
   */
  ventilation?: VentilationParking | null;
}

/**
 * Th-BCE 2020 §5.4.2, tableau 19 — paroi d'un espace tampon solarisé donnant sur
 * l'extérieur.
 */
export interface ParoiEspaceTampon {
  /** « Aue » : surface, en m². */
  a_ue_m2: number;
  /**
   * « αwe », « βwe » et masques : orientation, inclinaison et masques de la paroi
   * (fiche « Environnement proche », §3.2).
   */
  exposition: Exposition;
  /** Identifiant de la paroi. */
  id: string;
  /** « Rouv_max » : ratio d'ouverture maximale de la paroi (0 pour une paroi fixe). */
  r_ouv_max: number;
  /**
   * « Swe_ap » : facteur solaire global avec protection mobile en place. Requis si l'espace
   * tampon a des protections mobiles.
   */
  swe_ap?: number | null;
  /** « Swe_sp » : facteur solaire global sans protection mobile. */
  swe_sp: number;
  /**
   * « Tle_ap » : transmission lumineuse globale avec protection mobile en place. Requise si
   * l'espace tampon a des protections mobiles.
   */
  tle_ap?: number | null;
  /** « Tle_sp » : transmission lumineuse globale sans protection mobile. */
  tle_sp: number;
  /** « Uue » : coefficient de transmission thermique, en W/(m².K). */
  u_ue_w_m2k: number;
}

/** Paroi opaque d'un groupe : paramètres d'intégration (§5.17.2). */
export interface ParoiOpaque {
  /** « Ak » : surface de la paroi, en m². */
  a_k_m2: number;
  /**
   * Identifiant de l'espace tampon sur lequel donne la paroi (§5.17.3, « Prise en compte des
   * espaces tampons »). Absent : la paroi donne sur l'extérieur.
   */
  espace_tampon?: string | null;
  /** Orientation, inclinaison et masques. */
  exposition: Exposition;
  /** Identifiant de la paroi. */
  id: string;
  /** Référence à un [`ParoiType`] du catalogue. */
  type_paroi: string;
}

/** Th-BCE 2020 §5.17.2 — paramètres intrinsèques d'une paroi opaque. */
export interface ParoiType {
  /** Identifiant du type de paroi. */
  id: string;
  /**
   * « Inclk » : position pour laquelle `Uk` est donné. Obligatoire, sans valeur par défaut :
   * une position supposée changerait U en silence. Le moteur en déduit le coefficient de la
   * paroi dans son inclinaison réelle (§5.17.3, éq. 294). Correspondance avec les règles
   * Th-Bat, d'après les résistances superficielles cumulées que le texte retire de `1/Uk` :
   * `verticale` pour un U de paroi verticale (0,17 m².K/W) ; `horizontale_flux_ascendant` pour
   * un U de plancher haut (0,14) ; `horizontale_flux_descendant` pour un U de plancher bas
   * (0,21).
   */
  incl_k: PositionParoi;
  /** « Sf-c,k » : facteur solaire pour le calcul du Cep (paroi non végétalisée). */
  sf_c_k: number;
  /** « Sf-e,k » : facteur solaire pour le calcul de DH (paroi non végétalisée). */
  sf_e_k: number;
  /** « Uk » : coefficient de transmission thermique, en W/(m².K), dans la position `incl_k`. */
  u_k_w_m2k: number;
  /** Toiture végétalisée (« Typevegetal » = 1) et ses facteurs solaires. */
  vegetalisation?: Vegetalisation | null;
}

/** Performances des capteurs (§9.15.3.1). */
export type PerformancesCapteurs = {
  certifie: {
    /** « a1 » : coefficient de pertes du premier ordre, en W/(m².K). */
    a1_w_m2k: number;
    /** « a2 » : coefficient de pertes du second ordre, en W/(m².K²). */
    a2_w_m2k2: number;
    /** « η0 » : rendement optique, de 0 à 1. */
    eta0: number;
  };
} | {
  defaut: {
    /** Type de capteur. */
    type_capteur: TypeCapteurSolaire;
  };
};

/**
 * Performances à pleine charge d'un mode (§8.23.3.1) : « Statut_données_PC » et
 * « Statut_val_pivot ».
 */
export type PerformancesPac = {
  mesurees: PointPerformance[];
} | {
  pivot_declare: {
    /** « Val_Pabs » : puissance absorbée au point pivot, en kW. */
    pabs_kw: number;
    /** « Val_COP » ou « Val_EER » : COP ou EER déclaré au point pivot. */
    performance: number;
  };
} | {
  pivot_defaut: {
    /** « Val_Pabs » : puissance absorbée au point pivot, en kW. */
    pabs_kw: number;
  };
};

/**
 * Origine des performances « Pc », « μ » et « NOCT » (§12.3.3.1) : elle fixe les valeurs de
 * calcul, réduites par rapport aux valeurs saisies.
 */
export type PerformancesPv = {
  /** « μ » : coefficient de température de la puissance crête, en °C⁻¹. */
  mu_par_c: number;
  /** « NOCT » : température d'équilibre thermique du module, en °C. */
  noct_c: number;
  /** « Pc » : puissance crête d'un module en conditions STC, en W. */
  pc_w: number;
  statut: "certifie";
} | {
  /** « μ » justifié, en °C⁻¹. */
  mu_par_c: number;
  /** « NOCT » justifiée, en °C. */
  noct_c: number;
  /** « Pc » justifiée, en W. */
  pc_w: number;
  statut: "justifie";
} | {
  /** « μ » déclaré, en °C⁻¹. */
  mu_par_c: number;
  /** « NOCT » déclarée, en °C. */
  noct_c: number;
  /** « Pc » déclarée, en W. */
  pc_w: number;
  statut: "declare";
} | {
  /** « Pc » saisie, en W. */
  pc_w: number;
  statut: "defaut";
};

/**
 * Période de confort adaptatif (§3.1.3, éq. 1 et 2) : jours de l'année numérotés de 1 à 365.
 * Les deux bornes valent `null` si la moyenne glissante n'atteint jamais 16 °C.
 */
export interface PeriodeConfortAdaptatif {
  /** « Dernier_jour ». */
  dernier_jour: number | null;
  /** « Premier_jour ». */
  premier_jour: number | null;
}

/** Th-BCE 2020 §8.1.3.1.4, éq. (795) — pertes au dos d'un émetteur intégré à une paroi. */
export interface PertesAuDos {
  /**
   * « Ri » : résistance thermique de la paroi entre l'émetteur et l'intérieur, résistance
   * superficielle intérieure comprise, en m².K/W.
   */
  r_i_m2k_w: number;
  /** « Ue » : coefficient de transmission thermique équivalent de la paroi, en W/(m².K). */
  u_e_w_m2k: number;
}

/** « (UA)s » et « Statut_UA » (§9.9.3.1, éq. 1766 à 1768). */
export type PertesBallon = {
  certifie: {
    /** « UAs », en W/K. */
    ua_s_w_k: number;
  };
} | {
  justifie: {
    /** « UAs », en W/K. */
    ua_s_w_k: number;
  };
} | {
  defaut: {
    /** Nature du ballon. */
    nature: NatureBallonSaisie;
  };
};

/** « idpertes_parois » (§8.19.3.5, tableau 121). */
export type PertesParoisChaudiere = "sans_ventilateur" | "avec_ventilateur" | "avec_clapet";

/** Plage horaire en heures légales, de `debut` (inclus) à `fin` (exclue), de 0 à 24. */
export interface PlageHoraire {
  /** Heure légale de début, de 0 à 24. */
  debut_h: number;
  /** Heure légale de fin, de 0 à 24. */
  fin_h: number;
}

/** Th-BCE 2020 §8.22.2, tableau 141 — poêle ou insert à bois (chauffage seul). */
export interface PoeleSaisi {
  /**
   * « Type_Combustible_Bois » (§13.2.2) : combustible, qui fixe le facteur d'émission du bois
   * dans l'Ic énergie (article 10 de l'arrêté). Obligatoire, comme pour une chaudière au bois.
   */
  combustible_bois: CombustibleBoisSaisi;
  /** « ηH,sys,n » : rendement moyen selon la norme de l'appareil, en %. */
  eta_pct: number;
  /** « Paux,vent » : puissance du ventilateur, en W (0 sans ventilateur). */
  p_aux_vent_w: number;
  /** « Pngen » : puissance nominale d'un appareil, en kW (de 0 exclu à 100). */
  pn_gen_kw: number;
}

/** Valeur mesurée d'une matrice de performances (§8.23.3.1). */
export interface PointPerformance {
  /** Puissance absorbée mesurée à pleine charge, en kW. */
  pabs_kw: number;
  /** COP (chauffage, ECS) ou EER (refroidissement) mesuré à pleine charge. */
  performance: number;
  /** Statut : un COP ou un EER justifié est réduit de 10 % (éq. 1238). */
  statut: StatutMesure;
  /** Température amont de la mesure, en °C : une des températures de la technologie. */
  theta_amont_c: number;
  /** Température aval de la mesure, en °C : une des températures de la technologie. */
  theta_aval_c: number;
}

/** Point de la courbe de rendement d'un onduleur. */
export interface PointRendementOnduleur {
  /** « ηi » : rendement à ce point, entre 0 et 1. */
  eta: number;
  /** « Xi » : `PPV / PAC NOM`. */
  x: number;
}

/** Th-BCE 2020 §5.19.2 — pont thermique. */
export interface PontThermique {
  /**
   * Identifiant de l'espace tampon sur lequel donne le pont thermique (§5.19.3). Absent : le
   * pont thermique donne sur l'extérieur.
   */
  espace_tampon?: string | null;
  /**
   * Orientation, inclinaison et masques du pont thermique (§5.18.3). Champ facultatif.
   * Présent : description détaillée. **Absent : la description simplifiée s'applique**, où le
   * pont thermique est réparti en quatre ponts verticaux orientés au sud, à l'ouest, au nord
   * et à l'est, d'un quart de la longueur chacun, sans masque (éq. 298).
   */
  exposition?: Exposition | null;
  /** Identifiant du pont thermique. */
  id: string;
  /** « ll » : longueur, en m. */
  l_l_m: number;
  /** « Ψl » : coefficient de transmission thermique linéique, en W/(m.K). */
  psi_l_w_mk: number;
  /** « Sf-c,l » : facteur solaire pour le calcul du Cep, en m⁻¹. */
  sf_c_l: number;
  /**
   * « Sf-e,l » : facteur solaire pour le calcul de DH, en m⁻¹ (le texte dit « Tic » :
   * décision Q14).
   */
  sf_e_l: number;
}

/** « idpos » (§8.17.2) : emplacement d'une génération. */
export type PositionGeneration = "en_volume_chauffe" | {
  hors_volume_chauffe: {
    /** Identifiant de l'espace tampon qui abrite la génération. */
    espace_tampon?: string | null;
  };
};

/** « Inclk » (§5.17.2) : position de référence du coefficient `Uk` d'une paroi opaque. */
export type PositionParoi = "verticale" | "horizontale_flux_ascendant" | "horizontale_flux_descendant";

/**
 * Consommation électrique d'un poste, annuelle et par mois des scénarios, en kWh d'énergie
 * finale.
 */
export interface PosteElectrique {
  /** Consommation annuelle, en kWh. */
  annuel_kwh: number;
  /**
   * Consommation annuelle par m² de surface de référence du niveau (zone ou bâtiment), en
   * kWh/m².
   */
  annuel_kwh_m2: number;
  /** Consommation des douze mois, en kWh. */
  mensuel_kwh: number[];
}

/** Électricité par poste, en kWh d'énergie finale. */
export interface PostesElectricite {
  /** Auxiliaires de distribution (éq. 2505). */
  auxiliaires_distribution_kwh: number;
  /** Auxiliaires de ventilation : centrales, ventilateurs locaux, parkings (éq. 2504, 2511). */
  auxiliaires_ventilation_kwh: number;
  /** Chauffage (auxiliaires des générateurs compris). */
  chauffage_kwh: number;
  /** Déplacement des occupants : ascenseurs (éq. 2513). */
  deplacements_kwh: number;
  /** Éclairage : groupes, parties communes et parkings (éq. 2510). */
  eclairage_kwh: number;
  /** Eau chaude sanitaire. */
  ecs_kwh: number;
  /**
   * Usages mobiliers (fiche 4.7) : hors du périmètre RE2020, ils ne comptent que dans
   * l'autoconsommation.
   */
  mobilier_kwh: number;
  /** Refroidissement (forfait de refroidissement compris, §13.3). */
  refroidissement_kwh: number;
}

/** Th-BCE 2020 §8.5 — programmation des relances d'un groupe. */
export interface Programmation {
  /** « Typepgrm_ch » : programmation de la relance du chauffage (tableau 92). */
  chauffage: ProgrammationChauffage;
  /**
   * « Typepgrm_fr » : programmation de la relance du refroidissement (tableau 93). Requise
   * si le groupe est climatisé (`iclim`), sans objet sinon.
   */
  refroidissement?: ProgrammationRefroidissement | null;
}

/** « typegest » (§9.10.3.2.1) : programmation du thermostat d'un générateur de ballon. */
export type ProgrammationBallon = "permanent" | "nuit" | "jour";

/** « Typepgrm_ch » (§8.5.3, tableau 92). */
export type ProgrammationChauffage = "horloge" | "horloge_controle_ambiance" | "optimiseur";

/** « Typepgrm_fr » (§8.5.3, tableau 93). */
export type ProgrammationRefroidissement = "horloge" | "horloge_controle_ambiance" | "sans_horloge";

/** Niveau « projet » : données communes à tous les bâtiments (Th-BCE 2020 §5.1.1, note 1). */
export interface Projet {
  /**
   * Adresse du JSON Schema du format (`thbce schema`), pour l'aide à la saisie des éditeurs.
   * Facultative ; ignorée par le moteur.
   */
  "$schema"?: string | null;
  /** Bâtiments du projet. */
  batiments: Batiment[];
  /** Composants décrits par leurs paramètres intrinsèques. */
  catalogue?: Catalogue;
  /** Situation géographique. */
  localisation: Localisation;
  /** Nom libre du projet. */
  nom: string;
  /**
   * Version du format d'entrée. Facultative ; si elle est donnée, elle doit valoir
   * [`VERSION_FORMAT`].
   */
  version_format?: number | null;
}

/** Th-BCE 2020 §5.9.2 et §5.12.2 — protection mobile d'une baie et sa gestion. */
export interface ProtectionMobile {
  /** « Protext » : vrai si la protection est extérieure, faux si elle est intérieure. */
  exterieure: boolean;
  /**
   * « Type-GPM » : gestion de la première protection mobile. La gestion automatique n'est pas
   * prise en charge dans cette version : la validation la refuse.
   */
  gestion: GestionPm;
  /**
   * « PM2 » : présence d'une seconde protection mobile, intérieure, gérée manuellement, dont
   * la seule fonction est d'éviter l'éblouissement.
   */
  seconde_protection: boolean;
  /**
   * « Type-PM » : type de la première protection mobile. Le store vénitien n'est pas pris en
   * charge dans cette version : la validation le refuse.
   */
  type_pm: TypePm;
  /**
   * « Typo_PM » : typologie de la protection pour le passage de l'air quand elle est fermée
   * (§5.12.3.1, tableau 40). Absente : typologie 0, « valeur par défaut » du texte (taux de
   * passage de 0 %), signalée dans les résultats.
   */
  typo_pm?: TypoPm | null;
}

/**
 * Th-BCE 2020 §5.3.2, tableau 15 — protections mobiles des parois d'un espace tampon :
 * « conventionnellement de type stores enroulables intérieurs ou extérieurs » (§5.3.3.3).
 */
export interface ProtectionsEspaceTampon {
  /** « Protet_ext » : protections extérieures (vrai) ou intérieures (faux). */
  exterieures: boolean;
}

/** Puits climatique ou hydraulique (§6.8 à 6.11). */
export type Puits = "climatique" | "hydraulique";

/** « idraccord_reseau » (§8.4.2). */
export type RaccordementReseaux = "avec_isolement" | "permanent";

/** Raccordement de l'appoint de chauffage d'un système solaire combiné. */
export type RaccordementSsc = {
  independant: {
    /** « Δθreg^bcl_ch », en K (sans valeur conventionnelle dans le texte). */
    delta_theta_reg_boucle_chauffage_k: number;
  };
} | "raccorde";

/** Régime d'exigences d'une extension ou d'une construction de petite surface. */
export interface RegimeAlternatif {
  /**
   * Conformité aux exigences de résultat du régime que le moteur évalue : régime complet,
   * celle des `exigences` ; régime intermédiaire, Bbio et DH (`Icconstruction` et les
   * exigences de moyens sont hors du moteur) ; absente pour les autres régimes, qui n'ont pas
   * d'exigence de résultat.
   */
  conforme?: boolean | null;
  /** Famille d'usage retenue (usage principal de la partie calculée, Q86). */
  famille: FamilleUsage;
  /** Indicateurs exigés par le régime intermédiaire qui n'ont pas pu être calculés. */
  indicateurs_manquants?: string[];
  /** Nature des travaux. */
  nature: NatureTravaux;
  /**
   * Part de la surface considérée dans la `Sref` des locaux existants, en % ; absente pour
   * une construction et sans `sref_existant_m2`.
   */
  part_existant_pct?: number | null;
  /** Régime applicable. */
  regime: RegimeExigences;
  /**
   * Surface considérée, en m² : `Sref` du bâtiment calculé, plus celle des autres extensions
   * du même bâtiment (article 50-3, I).
   */
  sref_consideree_m2: number;
  /**
   * Vérifications du régime intermédiaire (article 50-3, II) : Bbio et DH de chaque groupe ;
   * vide pour les autres régimes.
   */
  verifications?: VerificationExigence[];
}

/**
 * Régime d'exigences applicable (FA03 v2, « Que recouvrent les trois régimes des
 * extensions ? »).
 */
export type RegimeExigences = "complet" | "intermediaire" | "exigences_de_moyens" | "hors_champ" | "rt2012";

/** « idregul_batt » (§8.1.2). */
export type RegulationBatterie = "debit_regule" | "temperature_constante";

/** « Typeregul_BS » (§9.15.3.2). */
export type RegulationBoucleSaisie = "temperature" | {
  irradiance: {
    /**
     * « Isrmin », en W/m². Absent : 200 W/m², valeur conventionnelle du tableau 292,
     * signalée.
     */
    isr_min_w_m2?: number | null;
  };
};

/** « iddebit » (§8.7.2). */
export type RegulationDebit = "debit_constant_continu" | "debit_constant_intermittent" | "debit_variable";

/** « idgest » (§8.7.2). */
export type RegulationTemperature = "depart_constant" | "retour_constant" | "loi_d_eau";

/** Information de rendement d'un onduleur (§12.4.3, « Définition de la fonction ηOND »). */
export type RendementOnduleur = {
  cas: "defaut";
} | {
  cas: "europeen";
  /** « ηEU » : rendement européen, entre 0 et 1. */
  eta_eu: number;
} | {
  cas: "courbe";
  /** Points `(Xi, ηi)`, `Xi` étant le rapport `PPV / PAC NOM`. */
  points: PointRendementOnduleur[];
};

/** Rendements d'une chaudière sur PCS : `R_pn`, `R_pint` et `Pint`. */
export interface RendementsChaudiere {
  /**
   * « Pint » : puissance thermique utile à 30 % de charge, en kW. Absent : 30 % de `Pn_gen`
   * (valeur par défaut signalée).
   */
  p_int_kw?: number | null;
  /** « R_pint » : rendement sur PCS à 30 % de charge en régime basse température, en %. */
  r_pint_pct: number;
  /** « R_pn » : rendement sur PCS à la puissance nominale en régime haute température, en %. */
  r_pn_pct: number;
  /** Statut des deux rendements. */
  statut: StatutRendementChaudiere;
}

/**
 * « idniveau_detail_echangeur » : représentation simplifiée (efficacité) ou détaillée
 * (coefficient d'échange global).
 */
export type RepresentationEchangeur = {
  simplifiee: EchangeurSimplifie;
} | {
  detaillee: EchangeurDetaille;
};

/** Th-BCE 2020 §6.2.2, §6.3.2 et §6.5.2 — réseau de conduits d'un système de ventilation. */
export interface ReseauAeraulique {
  /**
   * « b » (« bet ») : ratio de prise en compte des locaux non chauffés que traversent les
   * conduits (de 0 à 1 ; 1 pour des conduits à l'extérieur, éq. 649 et 471).
   */
  b: number;
  /**
   * « Cletres » : classe d'étanchéité du réseau. Absente : valeur par défaut du tableau 59
   * (`Kres = 0,0675·10⁻³ m³/(s.m²)` sous 1 Pa), signalée dans les résultats.
   */
  classe_etancheite?: ClasseEtancheite | null;
  /**
   * « Rrep » : résistance thermique de la partie des conduits de reprise située hors volume
   * chauffé, en m².K/W. Requise si le système a des bouches de reprise.
   */
  r_rep_m2k_w?: number | null;
  /**
   * « Rsouf » : résistance thermique de la partie des conduits de soufflage située hors
   * volume chauffé, en m².K/W. Requise si le système a des bouches de soufflage.
   */
  r_souf_m2k_w?: number | null;
  /**
   * « Ratfuitevc » : part des fuites en volume chauffé (de 0 à 1). Requise quand les
   * surfaces des conduits sont saisies (`a_cond_m2` des bouches, `Is_surf_cond_def = 0`) ;
   * sinon valeur par défaut du tableau 60 (0,25 en maison individuelle, 0,5 en bâtiment
   * collectif), signalée dans les résultats.
   */
  ratfuitevc?: number | null;
}

/**
 * Th-BCE 2020 §8.28.2, tableau 246 — sous-station d'un réseau de chaleur (type 600) ou de
 * froid (type 601).
 */
export type ReseauChaleurFroidSaisi = {
  chaleur: {
    /**
     * Classe d'isolation (EN ISO 12241) du réseau secondaire, de 1 à 4 ; le primaire est
     * isolé d'une classe de plus (§8.28.3.1).
     */
    classe_isolation_secondaire: number;
    /** « idfougen » : chauffage, ECS ou les deux. */
    fonction: FonctionChaleur;
    /**
     * Identifiant du réseau dans l'annexe 7 de l'arrêté du 15 septembre 2006 (quatre
     * chiffres suivis de « C »), qui donne le facteur d'émission et le taux d'énergie
     * renouvelable et de récupération (lot 7). Absent : lignes « autres réseaux de
     * chaleur » de l'annexe.
     */
    identifiant_reseau?: string | null;
    /**
     * « PEss » : puissance d'échange d'une sous-station dans les conditions nominales, en
     * kW.
     */
    p_ess_kw: number;
    /** Type de réseau. */
    type_reseau: TypeReseauChaleurSaisi;
  };
} | {
  froid: {
    /** Identifiant du réseau dans l'annexe 7 (quatre chiffres suivis de « F »). */
    identifiant_reseau?: string | null;
    /** « PEss » : puissance d'échange d'une sous-station, en kW. */
    p_ess_kw: number;
  };
};

/** « Typeréseauintergroupe-e » (§9.8.2). */
export type ReseauEcsIntergroupe = "aucun" | {
  boucle: BoucleEcs;
} | {
  trace: ReseauEcsTrace;
};

/** Réseau d'ECS intergroupe tracé (§9.8.2). */
export interface ReseauEcsTrace {
  /** Espace tampon traversé par la partie hors volume chauffé (« btherm », éq. 1756). */
  espace_tampon?: string | null;
  /** « Lhvc_prim_trac-e » : longueur hors volume chauffé, en m. */
  l_hvc_m: number;
  /** « Lvc_prim_trac-e » : longueur en volume chauffé, en m. */
  l_vc_m: number;
  /** « Uprim-e » : coefficient linéique, en W/(m.K). */
  u_w_mk: number;
}

/** « idtype » d'un réseau de distribution du groupe (§8.7.2). */
export type ReseauGroupe = "fictif" | {
  hydraulique: ReseauHydrauliqueGroupe;
};

/**
 * Th-BCE 2020 §8.7.2 et §8.8.2 — paramètres d'un réseau hydraulique du groupe.
 *
 * Classement des longueurs en ou hors volume chauffé : fiche d'application 8.13 (voir
 * [`ReseauHydrauliqueIntergroupes`]) ; les planchers chauffants ou rafraîchissants n'en
 * relèvent pas.
 */
export interface ReseauHydrauliqueGroupe {
  /** « idcirc » : régulation du circulateur. */
  circulateur: Circulateur;
  /**
   * « Δθem_dim » : écart nominal de température entre départ et retour, en K (négatif en
   * refroidissement).
   */
  delta_theta_em_dim_k: number;
  /**
   * Espace tampon traversé par la partie hors volume chauffé (« btherm », éq. 876 et 883).
   * Absent : la partie hors volume chauffé est à l'extérieur (`btherm = 1`).
   */
  espace_tampon?: string | null;
  /** « Lhvc » : longueur hors volume chauffé, en m. */
  l_hvc_m: number;
  /** « Lvc » : longueur en volume chauffé, en m. */
  l_vc_m: number;
  /** « Paux » : puissance du circulateur, en W. */
  p_aux_w: number;
  /** « qnom » : débit volumique nominal, en m³/h. */
  q_nom_m3_h: number;
  /**
   * « qresid » : débit résiduel minimal d'un réseau à débit variable, en m³/h ; requis pour
   * un réseau à débit variable.
   */
  q_resid_m3_h?: number | null;
  /** « iddebit » : régulation du débit. */
  regulation_debit: RegulationDebit;
  /** « idgest » : régulation de la température. */
  regulation_temperature: RegulationTemperature;
  /**
   * « θdep_dim » : température de départ de dimensionnement, en °C ; requise pour une
   * température de départ constante ou une loi d'eau.
   */
  theta_dep_dim_c?: number | null;
  /**
   * « θret_dim » : température de retour de dimensionnement, en °C ; requise pour une
   * température de retour constante.
   */
  theta_ret_dim_c?: number | null;
  /**
   * « Umoyen_hvc » : coefficient de déperdition linéique moyen hors volume chauffé, en
   * W/(m.K).
   */
  u_hvc_w_mk: number;
  /** « Umoyen_vc » : coefficient de déperdition linéique moyen en volume chauffé, en W/(m.K). */
  u_vc_w_mk: number;
}

/**
 * Th-BCE 2020 §8.9.2 et §8.10.2 — paramètres d'un réseau intergroupes hydraulique.
 *
 * Classement des longueurs en ou hors volume chauffé : fiche d'application 8.13 (un réseau
 * intégré à une paroi est en volume chauffé si la résistance thermique qui le sépare du volume
 * chauffé est inférieure à la moitié de celle de la paroi ; en gaine palière, selon la
 * circulation au sens des règles Th-Bat ; en gaine logement, en volume chauffé).
 */
export interface ReseauHydrauliqueIntergroupes {
  /** « idcirc » : régulation du circulateur. */
  circulateur: Circulateur;
  /**
   * Espace tampon traversé par la partie hors volume chauffé (« btherm », éq. 924 et 932).
   * Absent : la partie hors volume chauffé est à l'extérieur (`btherm = 1`).
   */
  espace_tampon?: string | null;
  /** « Lhvc » : longueur hors volume chauffé, en m. */
  l_hvc_m: number;
  /** « Lvc » : longueur en volume chauffé, en m. */
  l_vc_m: number;
  /** « Paux » : puissance du circulateur, en W. */
  p_aux_w: number;
  /** « Umoy_hvc » : coefficient de déperdition linéique moyen hors volume chauffé, en W/(m.K). */
  u_hvc_w_mk: number;
  /** « Umoy_vc » : coefficient de déperdition linéique moyen en volume chauffé, en W/(m.K). */
  u_vc_w_mk: number;
}

/** « idtype » d'un réseau intergroupes (§8.9.2). */
export type ReseauIntergroupes = "fictif" | {
  hydraulique_collectif: ReseauHydrauliqueIntergroupes;
} | {
  hydraulique_individuel: ReseauHydrauliqueIntergroupes;
};

/** Résultats d'un calcul sur un projet. */
export interface Resultats {
  /** Avertissements émis par la validation et par le calcul. */
  avertissements: string[];
  /** Résultats par bâtiment. */
  batiments: ResultatsBatiment[];
  /** Climat du projet (Th-BCE 2020 §3.1). */
  climat: ResultatsClimat;
  /** Provenance du jeu météo « Th-BC ». */
  origine_meteo_th_bc: OrigineMeteo;
  /** Provenance du jeu météo « Th-D ». */
  origine_meteo_th_d: OrigineMeteo;
  /**
   * Faux dès qu'une hypothèse non réglementaire intervient : données météo autres que les
   * fichiers conventionnels de la RE2020, ou règle de substitution documentée comme telle.
   */
  reglementaire: boolean;
  /** Vrai si les valeurs sont arrondies à une décimale (Th-BCE 2020 §1.3.1). */
  valeurs_arrondies: boolean;
  /** Version de la méthode implémentée. */
  version_methode: string;
  /** Version du moteur. */
  version_moteur: string;
}

/** Une cabine d'ascenseur (§10.1). */
export interface ResultatsAscenseur {
  /** « Eti » : consommation annuelle en immobilité, en kWh (éq. 2303). */
  e_ti_kwh: number;
  /** « Etm » : consommation annuelle en mobilité, en kWh (éq. 2302). */
  e_tm_kwh: number;
  /** « Etot », en kWh (éq. 2304). */
  e_tot_kwh: number;
  /** Identifiant de la cabine. */
  id: string;
  /** « Ndem » : nombre de démarrages par an (éq. 2283). */
  nb_demarrages: number;
  /** « Pti » : puissance de veille cabine immobile, en W. */
  pti_w: number;
  /** « Tmobh » : durée moyenne de déplacement par jour, en h (éq. 2292). */
  t_mob_h: number;
  /** Valeurs par défaut du texte appliquées. */
  valeurs_par_defaut: ValeurParDefaut[];
}

/** Résultats d'une baie. */
export interface ResultatsBaie {
  /** Identifiant de la baie. */
  id: string;
  /**
   * Valeurs par défaut de la méthode appliquées aux paramètres non saisis de la baie ; liste
   * vide si tout a été saisi.
   */
  valeurs_par_defaut: ValeurParDefaut[];
}

/**
 * Résultats d'un bâtiment. Un indicateur vaut `null` tant que la partie correspondante de la
 * méthode n'est pas implémentée (voir `docs/couverture.md`).
 */
export interface ResultatsBatiment {
  /** Ascenseurs, une entrée par cabine (§10.1) ; absent sans ascenseur. */
  ascenseurs?: ResultatsAscenseur[];
  /** Besoins mensuels et annuels et Bbio du bâtiment (§13.1, « Sortie_Batiment_B »). */
  bbio: ResultatsBbio | null;
  /**
   * « Bbio_pts_annuel » : besoin bioclimatique du bâtiment, en points (§13.1.3, éq. 2385 et
   * 2386). Même valeur que `bbio.annuel.bbio_pts`.
   */
  bbio_pts: number | null;
  /**
   * Consommation annuelle des auxiliaires de ventilation du bâtiment (ventilateurs), en kWh
   * d'énergie finale électrique : somme de celles de ses zones. Absente sans système de
   * ventilation du projet.
   */
  c_aux_vent_kwh?: number | null;
  /** « Cep » : consommation d'énergie primaire, en kWhep/(m².an) (§13.2). */
  cep_kwhep_m2: number | null;
  /** « Cep,nr » : consommation d'énergie primaire non renouvelable, en kWhep/(m².an). */
  cep_nr_kwhep_m2: number | null;
  /**
   * Consommations en énergie finale des générations attribuées au bâtiment, par poste et
   * par type d'énergie (§8.29, éq. 1484 à 1486) : somme de celles de ses zones. Absent sans
   * génération.
   */
  consommations?: Consommations | null;
  /**
   * Postes propres au logement collectif et aux parkings du bâtiment, en électricité (§7.2,
   * §10.1, §10.3, §10.4) : somme de ceux de ses zones. Absent sans logement collectif ni
   * parking.
   */
  consommations_collectif?: ConsommationsCollectif | null;
  /**
   * « DH » : degrés-heures d'inconfort, en °C.h (§13.5.3, éq. 2551). L'indicateur est défini
   * par groupe (§13.5.1) : c'est ici celui du groupe le plus défavorable du bâtiment ; le DH
   * de chaque groupe est dans `zones[].groupes[].confort_ete`.
   */
  dh_c_h: number | null;
  /** Distributions d'ECS intergroupes, calcul Th-C (§9.8) ; absent sans réseau. */
  distributions_ecs_intergroupes?: ResultatsEcsIntergroupe[];
  /**
   * Réseaux de distribution intergroupes, issus du calcul des consommations (Th-C) : demandes
   * transmises aux générations (§8.9 et §8.10) ; absent sans réseau.
   */
  distributions_intergroupes?: ResultatsIntergroupes[];
  /** Espaces tampons du bâtiment (§5.2 à 5.4) ; absent sans espace tampon. */
  espaces_tampons?: ResultatsEspaceTampon[];
  /**
   * Générations, issues du calcul Th-C (§8.14, §8.17 et §8.29) : demandes de leurs réseaux,
   * énergies fournies et consommations de leurs générateurs. Absent sans génération.
   */
  generations?: ResultatsGeneration[];
  /** « Ic énergie », en kg éq. CO2/m² (annexe II §4.3.2, éq. 74). */
  ic_energie_kgco2_m2: number | null;
  /** Identifiant du bâtiment. */
  id: string;
  /**
   * Indicateurs réglementaires du bâtiment (§13.2 à 13.4, annexe II §4.3.2) : consommations
   * importées par poste et par énergie, autoconsommation, forfait de refroidissement, Cep,
   * Cep,nr, Ic énergie, exigences et conformité. Absents sans génération.
   */
  indicateurs?: ResultatsIndicateurs | null;
  /** Parkings (§10.3 et §10.4) ; absent sans parking. */
  parkings?: ResultatsParking[];
  /**
   * Production photovoltaïque du bâtiment (§12), sur le jeu météo « Th-BC » ; absente sans
   * installation. La répartition autoconsommé/exporté relève du chapitre 13.
   */
  production_pv?: ResultatsProductionPv | null;
  /** « SREF_bat » : surface de référence, en m² (Th-BCE 2020 §13.1.3, éq. 2378). */
  sref_m2: number;
  /** Résultats par zone. */
  zones: ResultatsZone[];
}

/**
 * Sorties du mode Th-B d'un groupe, d'une zone ou d'un bâtiment (Th-BCE 2020 §13.1).
 *
 * Sans arrondi de restitution, le Bbio annuel est la somme des douze Bbio mensuels (éq. 2386) ;
 * avec l'arrondi, chaque valeur est arrondie séparément à une décimale (décision Q8) et la
 * somme des valeurs mensuelles arrondies peut différer de la valeur annuelle arrondie.
 */
export interface ResultatsBbio {
  /** Besoins et Bbio de l'année. */
  annuel: BesoinsBbio;
  /**
   * Besoins et Bbio de chacun des douze mois de l'année conventionnelle (§4.1.3 : mois de 4
   * ou 5 semaines, au sens des scénarios).
   */
  mensuel: BesoinsBbio[];
}

/**
 * Boucle solaire d'un chauffe-eau solaire, calcul Th-C (§9.15 ; sorties « Esol_tot^BS » et
 * « Eef_aux_tot^BS » des fiches d'assemblage). L'énergie solaire transmise au ballon est
 * comprise dans `q_fou_sto_base_kwh` ; elle n'est pas une consommation.
 */
export interface ResultatsBoucleSolaire {
  /** Superficie d'entrée des capteurs d'un assemblage, en m². */
  a_capteurs_m2: number;
  /** « Esol_tot^BS » : énergie solaire transmise aux ballons par les échangeurs, en kWh. */
  e_sol_kwh: number;
  /** La même, par mois des scénarios, en kWh. */
  e_sol_mensuel_kwh: number[];
  /**
   * Énergie retirée aux ballons par la limitation à leur température maximale (éq. 1779),
   * en kWh : part de l'énergie solaire transmise que le ballon ne peut pas stocker (lecture
   * Q52).
   */
  ecretement_kwh: number;
  /** Ensoleillement annuel sur le plan des capteurs, masques compris, en kWh/m². */
  isr_kwh_m2: number;
  /** Heures de fonctionnement de la pompe. */
  nbh_marche: number;
  /** « ΦvcBS » : pertes des tuyauteries vers l'intérieur du bâtiment, en kWh (éq. 1973). */
  pertes_vc_kwh: number;
  /**
   * « Eef_aux_tot^BS » : consommation électrique des pompes, en kWh (comprise dans les
   * consommations d'ECS de la génération, éq. 1972).
   */
  w_pompe_kwh: number;
}

/** Grandeurs du climat extérieur propres au projet (Th-BCE 2020 §3.1.3). */
export interface ResultatsClimat {
  /** « altcorr » : altitude conventionnelle du site, en m. */
  altcorr_m: number;
  /** « θbase_ext » : température extérieure de base corrigée de l'altitude, en °C. */
  base_ext_c: number;
  /** Période de confort adaptatif déterminée sur le jeu météo « Th-BC ». */
  confort_adaptatif_th_bc: PeriodeConfortAdaptatif;
  /** Période de confort adaptatif déterminée sur le jeu météo « Th-D ». */
  confort_adaptatif_th_d: PeriodeConfortAdaptatif;
  /** Zone climatique retenue. */
  zone_climatique: ZoneClimatique;
}

/**
 * Indicateurs de confort d'un groupe sur l'année (Th-BCE 2020 §4.5.3.4 et 4.5.3.5), calculés
 * en occupation sur la température opérative en évolution libre.
 */
export interface ResultatsConfort {
  /** « DHinc_C1,fr » à « DHinc_C3,fr », en K.h (éq. 65). */
  dh_inc_c_fr_k_h: number[];
  /** « DHinc_cons,ch » : degrés-heures d'inconfort froid, en K.h (éq. 64). */
  dh_inc_cons_ch_k_h: number;
  /**
   * « DHinc_cons,fr » : degrés-heures d'inconfort chaud par rapport à la consigne de
   * refroidissement, en K.h (éq. 65).
   */
  dh_inc_cons_fr_k_h: number;
  /**
   * « Nbhinc_C1,fr » à « Nbhinc_C3,fr » : heures d'inconfort chaud par rapport au seuil des
   * catégories d'ambiance 1 à 3.
   */
  nbh_inc_c_fr: number[];
  /** « Nbhinc_cons,ch » : heures d'inconfort froid par rapport à la consigne de chauffage. */
  nbh_inc_cons_ch: number;
  /**
   * « Nbhinc_cons,fr » : heures d'inconfort chaud par rapport à la consigne de
   * refroidissement.
   */
  nbh_inc_cons_fr: number;
}

/**
 * Confort d'été d'un groupe (Th-BCE 2020 §13.5), issu du calcul Th-D : jeu météo avec séquence
 * caniculaire, sans refroidissement. Les indicateurs portent sur les pas de temps occupés de
 * la période de confort adaptatif.
 */
export interface ResultatsConfortEte {
  /** « DH » : degrés-heures d'inconfort, en °C.h (éq. 2551). */
  dh_c_h: number;
  /**
   * « Nbhinconf » : heures où la température opérative atteint la température d'inconfort
   * chaud (éq. 2548).
   */
  nbh_inconf: number;
  /** « Nbhinconf+1C » : heures où elle la dépasse d'au moins 1 °C (éq. 2549). */
  nbh_inconf_1c: number;
  /** « Nbhinconf+2C » : heures où elle la dépasse d'au moins 2 °C (éq. 2550). */
  nbh_inconf_2c: number;
  /**
   * Nombre d'heures occupées de la période de confort adaptatif, sur lesquelles portent les
   * indicateurs.
   */
  nbh_occ_conf_adapt: number;
  /** Température opérative maximale du groupe sur l'année, en °C (fin de pas de temps). */
  theta_op_max_c: number;
  /**
   * Température opérative maximale du groupe en occupation pendant la période de confort
   * adaptatif, en °C ; `null` si la période est vide.
   */
  theta_op_max_occ_c: number | null;
}

/**
 * Débits spécifiques d'un groupe pour le calcul du Bbio (Th-BCE 2020 §6.1 et §6.2), constants
 * sur l'année en habitation.
 */
export interface ResultatsDebitsBbio {
  /**
   * « qspec_repris » : débit volumique spécifique repris, en m³/h, négatif (§6.2.3.2.2.2,
   * éq. 403, avec `Crdbnr = 1`, `Dugd = 14` h par semaine et `Cdep = 1`).
   */
  q_spec_repris_m3_h: number;
  /** « qspec_soufflé » : débit volumique spécifique soufflé, en m³/h, positif. */
  q_spec_souffle_m3_h: number;
}

/** Réseau de distribution du groupe, calcul Th-C (valeurs absolues en refroidissement). */
export interface ResultatsDistributionGroupe {
  /** Identifiant. */
  id: string;
  /** Pertes hors volume chauffé, en kWh. */
  pertes_hvc_kwh: number;
  /** Pertes vers le volume chauffé, en kWh (récupérables, §11.1). */
  pertes_vc_kwh: number;
  /** Demande transmise au réseau intergroupes, pertes comprises, en kWh. */
  q_kwh: number;
  /** Consommation du circulateur, en kWh d'énergie finale électrique. */
  w_aux_kwh: number;
}

/** Émetteur d'ECS équivalent, calcul Th-C (§9.5 à 9.7). */
export interface ResultatsEcsEmetteur {
  /** « a » : besoins unitaires, en litres à 40 °C par adulte équivalent et par semaine. */
  a_l: number;
  /** « Agr,em-e » : surface desservie, en m² (éq. 1674). */
  a_m2: number;
  /** « corr-e » : coefficient correctif des émetteurs et des appareils (éq. 1678). */
  corr: number;
  /** Identifiant. */
  id: string;
  /** « Nadeq-e » : adultes équivalents (éq. 1685 et 1689). */
  nadeq: number;
  /** Pertes de la distribution du groupe hors volume chauffé, en kWh. */
  pertes_hvc_kwh: number;
  /** Pertes de la distribution du groupe en volume chauffé, en kWh (récupérables à 60 %). */
  pertes_vc_kwh: number;
  /** « Qw_2nd-e » : besoins augmentés des pertes de la distribution du groupe, en kWh. */
  q_w_2nd_kwh: number;
  /** « Qw_bruts » : besoins annuels sans correction des émetteurs, en kWh (éq. 1694). */
  q_w_bruts_kwh: number;
  /** « Qw » : besoins annuels, en kWh (éq. 1693). */
  q_w_kwh: number;
  /** « Vuw_hebdo_corr » : volume hebdomadaire corrigé, en litres à 40 °C (éq. 1692). */
  v_uw_hebdo_corr_l: number;
  /** Valeurs par défaut de la méthode appliquées. */
  valeurs_par_defaut: ValeurParDefaut[];
}

/** Distribution d'ECS intergroupe, calcul Th-C (§9.8). */
export interface ResultatsEcsIntergroupe {
  /** Génération qui produit l'ECS. */
  generation: string;
  /** Identifiant. */
  id: string;
  /** Pertes hors volume chauffé, en kWh. */
  pertes_hvc_kwh: number;
  /** Pertes en volume chauffé, en kWh. */
  pertes_vc_kwh: number;
  /** « Qw_prim-e » : besoins transmis à la génération, en kWh. */
  q_w_prim_kwh: number;
  /** Valeurs mensuelles de `q_w_prim_kwh`, en kWh. */
  q_w_prim_mensuel_kwh: number[];
  /** Température de départ demandée à la génération, en °C. */
  theta_depart_c: number | null;
  /** Circulateurs ou traceurs, en kWh d'énergie finale électrique. */
  w_aux_kwh: number;
  /** Réchauffeur de boucle, en kWh d'énergie finale électrique. */
  w_rechauf_kwh: number;
}

/** Émetteur, calcul Th-C. */
export interface ResultatsEmetteur {
  /** Identifiant. */
  id: string;
  /** Énergie de chauffage requise, en kWh. */
  q_ch_kwh: number;
  /** Énergie de refroidissement requise (valeur absolue), en kWh. */
  q_fr_kwh: number;
}

/**
 * Grandeurs d'enveloppe d'un groupe (Th-BCE 2020 §5.16.3 et §5.20.3). Les coefficients de
 * transmission sont ceux des éléments à leur inclinaison réelle (§5.17.3, éq. 294).
 */
export interface ResultatsEnveloppe {
  /**
   * « Abaies-h » : surface des baies horizontales, d'inclinaison inférieure à 60°, en m²
   * (éq. 281).
   */
  a_baies_h_m2: number;
  /** « Abaies » : surface des baies, en m² (§5.14.3, éq. 282). */
  a_baies_m2: number;
  /**
   * « Abaies-v » : surface des baies verticales, d'inclinaison au moins égale à 60°, en m²
   * (éq. 279).
   */
  a_baies_v_m2: number;
  /**
   * « Af,perm » : surface des parois déperditives verticales, opaques et vitrées, en m²,
   * utile aux calculs de perméabilité (§5.7.2 ; « Af » du §5.6.3.1.5).
   */
  a_f_perm_m2: number;
  /** « Aop » : surface des parois opaques, en m² (éq. 286). */
  a_op_m2: number;
  /** « Aophb » : surface des parois opaques horizontales tournées vers le bas, en m² (éq. 287). */
  a_ophb_m2: number;
  /** « Aophh » : surface des parois opaques horizontales tournées vers le haut, en m² (éq. 287). */
  a_ophh_m2: number;
  /** « Aopv » : surface des parois opaques verticales, en m² (éq. 287). */
  a_opv_m2: number;
  /**
   * « Atoit,perm » : surface des parois déperditives horizontales hors plancher bas, opaques
   * et vitrées, en m², utile aux calculs de perméabilité (§5.7.2 ; « At » du §5.6.3.1.5).
   */
  a_toit_perm_m2: number;
  /**
   * « Hges » des baies sans protection mobile en place, en W/K (§5.10.3.5, éq. 210 avec
   * `Rprot = 0`). La valeur horaire dépend de la position des protections mobiles.
   */
  h_ges_sp_w_k: number;
  /** « HTh » des parois opaques, en W/K (éq. 283). */
  h_th_op_w_k: number;
  /** « HTh » des ponts thermiques, en W/K (éq. 307). */
  h_th_pt_w_k: number;
  /** « Ll » : longueur des ponts thermiques, en m (éq. 310). */
  l_pt_m: number;
}

/**
 * Résultats d'un espace tampon (Th-BCE 2020 §5.4) : synthèse annuelle de ses sorties
 * horaires dans chacun des deux calculs.
 */
export interface ResultatsEspaceTampon {
  /** Identifiant de l'espace tampon. */
  id: string;
  /** Calcul du Bbio (mode Th-B). */
  th_b: ResultatsEspaceTamponMode | null;
  /** Calcul du confort d'été (mode Th-D). */
  th_d: ResultatsEspaceTamponMode | null;
  /** Valeurs par défaut du texte appliquées (`Cm_et_surf`, `Qv,et,base`). */
  valeurs_par_defaut: ValeurParDefaut[];
}

/** Synthèse annuelle d'un espace tampon dans un mode de calcul. */
export interface ResultatsEspaceTamponMode {
  /** Moyenne annuelle de « btherm(h) ». */
  b_therm_moyen: number;
  /** Nombre d'heures où les parois de l'espace tampon sont ouvertes (surventilation). */
  nbh_surventilation: number;
  /** Plus haute « θet(h) », en °C. */
  theta_et_max_c: number;
  /** Plus basse « θet(h) », en °C. */
  theta_et_min_c: number;
  /** Moyenne annuelle de « θet(h) », en °C. */
  theta_et_moy_c: number;
}

/** Exigences et conformité du bâtiment. */
export interface ResultatsExigences {
  /**
   * « Bbio_max » du bâtiment, en points : moyenne des zones pondérée par leur `Sref`
   * (chapitre II, V).
   */
  bbio_max_pts: number;
  /** « Cep_max » du bâtiment, en kWhep/(m².an). */
  cep_max_kwhep_m2: number;
  /** « Cep,nr_max » du bâtiment, en kWhep/(m².an). */
  cep_nr_max_kwhep_m2: number;
  /**
   * Vrai si toutes les exigences vérifiées sont respectées. Seules les exigences sur Bbio,
   * Cep, Cep,nr, Ic énergie et DH sont vérifiées : `Icconstruction`, les caractéristiques
   * minimales et les autres articles de l'arrêté sont hors du moteur. Toujours faux si un
   * indicateur exigé manque (`indicateurs_manquants`).
   */
  conforme: boolean;
  /** « Icénergie_max » du bâtiment, en kg éq. CO2/m². */
  ic_energie_max_kgco2_m2: number;
  /** Indicateurs exigés qui n'ont pas pu être calculés ni donc vérifiés ; vide en général. */
  indicateurs_manquants?: string[];
  /**
   * Régime d'exigences d'une extension ou d'une construction de petite surface (arrêté du
   * 4 août 2021, articles 3 et 50-3 ; article R. 172-1 du CCH ; Q86 à Q88) ; absent pour une
   * construction neuve dont la `Sref` atteint 50 m². Les exigences ci-dessus (article
   * R. 172-4) restent calculées : les exigences alternatives « peuvent être appliquées à la
   * place » des résultats minimaux.
   */
  regime_alternatif?: RegimeAlternatif | null;
  /** Vérification de chaque exigence ; le DH est vérifié pour chaque groupe. */
  verifications: VerificationExigence[];
  /** Exigences de chaque zone. */
  zones: ExigencesZone[];
}

/** Générateur, calcul Th-C (§8.29.3.8). */
export interface ResultatsGenerateur {
  /**
   * Consommations annuelles en énergie finale, par poste et par type d'énergie (éq. 1494 à
   * 1509).
   */
  consommations: ConsommationsPostes;
  /**
   * « ηeff_ch_an » : efficacité annuelle en chauffage (éq. 1526) — COP annuel d'un
   * générateur thermodynamique, rendement des autres ; absente sans consommation.
   */
  eta_eff_ch_an?: number | null;
  /** « ηeff_ecs_an » : efficacité annuelle en ECS. */
  eta_eff_ecs_an?: number | null;
  /** « ηeff_fr_an » : efficacité annuelle en refroidissement (EER annuel). */
  eta_eff_fr_an?: number | null;
  /** Identifiant du générateur. */
  id: string;
  /**
   * « φrejet » : énergie échangée par un générateur thermodynamique avec sa source amont, en
   * kWh (négative : chaleur puisée ; positive : chaleur rejetée en refroidissement).
   */
  phi_rejet_kwh?: number;
  /** « Eef_fou_ch » : énergie fournie en chauffage, en kWh (éq. 1525). */
  q_fou_ch_kwh: number;
  /** « Eef_fou_ecs » : énergie fournie en ECS, en kWh. */
  q_fou_ecs_kwh: number;
  /** « Eef_fou_fr » : énergie fournie en refroidissement, en kWh. */
  q_fou_fr_kwh: number;
  /** Valeurs par défaut de la méthode appliquées au générateur. */
  valeurs_par_defaut?: ValeurParDefaut[];
  /**
   * « Cef_aux » : consommation des auxiliaires du générateur, propres et amont, en kWh
   * d'énergie finale électrique (éq. 1527 et 1528) ; elle est comprise dans `consommations`.
   */
  w_aux_kwh: number;
}

/** Génération, calcul Th-C (§8.14, §8.17 et §8.29). */
export interface ResultatsGeneration {
  /**
   * Consommations en énergie finale de la génération, par poste et par type d'énergie
   * (éq. 1056 et 1057).
   */
  consommations: Consommations;
  /**
   * Générateurs de la génération ; la pompe à chaleur d'un chauffe-eau thermodynamique
   * décrit par son essai y figure sous l'identifiant du chauffe-eau.
   */
  generateurs: ResultatsGenerateur[];
  /** Identifiant de la génération. */
  id: string;
  /**
   * Plus longue suite d'heures où la demande de chauffage ou d'ECS n'a pas été couverte,
   * hors relance (éq. 1059) ; le texte y voit un sous-dimensionnement au-delà de 6 heures
   * et un sous-dimensionnement critique au-delà de 72 heures (éq. 1060 et 1061).
   */
  nbh_sousdim_ch_ecs_max: number;
  /** De même en refroidissement (éq. 1062 à 1064). */
  nbh_sousdim_fr_max: number;
  /**
   * « φvc_tot » : pertes et consommations d'auxiliaires transmises au volume chauffé, en kWh
   * (éq. 1058), avant application de la part récupérable (fiche 11.1).
   */
  pertes_vc_kwh: number;
  /**
   * Énergie de chauffage demandée par les réseaux, sécurité antigel des doubles flux
   * comprise, en kWh (éq. 993).
   */
  q_ch_kwh: number;
  /** Énergie d'ECS demandée, en kWh (éq. 995). */
  q_ecs_kwh: number;
  /** Énergie de chauffage fournie par les générateurs, en kWh. */
  q_fou_ch_kwh: number;
  /** Énergie d'ECS fournie, en kWh. */
  q_fou_ecs_kwh: number;
  /** Énergie de refroidissement fournie, en kWh. */
  q_fou_fr_kwh: number;
  /** Énergie de refroidissement demandée (valeur absolue), en kWh (éq. 994). */
  q_fr_kwh: number;
  /**
   * Demande restant reportée à la fin de l'année, tous postes confondus, en kWh (éq. 1028
   * et 1034) : nulle si la génération a tout fourni.
   */
  q_rep_fin_kwh: number;
  /**
   * Productions solaires collectives à appoints décentralisés (CESCI, CESCAI) ; absent sans
   * production.
   */
  solaires_collectifs?: ResultatsSolaireCollectif[];
  /** Assemblages avec ballon de stockage d'ECS (§9.14) ; absent sans ballon. */
  stockages?: ResultatsStockage[];
}

/** Résultats d'un groupe. */
export interface ResultatsGroupe {
  /** Résultats par baie. */
  baies: ResultatsBaie[];
  /** Besoins mensuels et annuels et Bbio du groupe (§13.1, « Sortie_Groupe_B »). */
  bbio: ResultatsBbio | null;
  /** Indicateurs de confort du groupe en calcul du Bbio (§4.5), en évolution libre. */
  confort_bbio: ResultatsConfort | null;
  /** Confort d'été du groupe, issu du calcul Th-D (§13.5). */
  confort_ete: ResultatsConfortEte | null;
  /**
   * Consommations en énergie finale des générations attribuées au groupe, par poste et par
   * type d'énergie, au prorata de ses demandes (§8.29, éq. 1484 à 1486, 1499, 1506 et 1513) ;
   * matrice « Qcef[poste ; énergie] » des sorties du mode Th-C (§13.2, éq. 2421). Absent sans
   * génération.
   */
  consommations?: Consommations | null;
  /**
   * Débits du système de ventilation conventionnel du Bbio ; `null` si le groupe ne décrit
   * aucune bouche de ventilation ou hors habitation.
   */
  debits_bbio: ResultatsDebitsBbio | null;
  /** Grandeurs d'enveloppe du groupe. */
  enveloppe: ResultatsEnveloppe;
  /** Identifiant du groupe. */
  id: string;
  /** Saisons propres du groupe en calcul du Bbio (§4.6). */
  saisons_bbio: ResultatsSaisons | null;
  /** Saisons propres du groupe en calcul Th-D (§4.6). */
  saisons_th_d: ResultatsSaisons | null;
  /** « SREF_gr », en m² (§13.1.3, éq. 2375 et 2376). */
  sref_m2: number;
  /**
   * Systèmes d'émission du groupe, issus du calcul des consommations (Th-C) ; absents sans
   * émetteur.
   */
  systemes?: ResultatsSystemesGroupe | null;
  /**
   * Valeurs par défaut de la méthode appliquées aux paramètres non saisis du groupe ; liste
   * vide si aucune.
   */
  valeurs_par_defaut: ValeurParDefaut[];
  /**
   * Ventilation du projet vue du groupe : débits et consommation des ventilateurs ; absente
   * hors habitation.
   */
  ventilation?: ResultatsVentilationGroupe | null;
}

/**
 * Identification d'un chauffe-eau thermodynamique (§9.22.3.4) : entrées du modèle Th-BCE
 * déduites de l'essai, avant correction de statut.
 */
export interface ResultatsIdCet {
  /** « ValCOPpivot » : valeur pivot du COP. */
  cop_pivot: number;
  /** « ε » : écart relatif entre l'essai reproduit par le modèle et l'essai saisi (éq. 2248). */
  epsilon: number;
  /** « ValPabspivot » : valeur pivot de la puissance absorbée, en kW. */
  pabs_pivot_kw: number;
  /** « UAs,util » : coefficient de pertes du ballon, en W/K. */
  ua_s_w_k: number;
}

/** Indicateurs réglementaires d'un bâtiment. */
export interface ResultatsIndicateurs {
  /**
   * « O_Cef_annuel » : énergie finale importée, hors usages mobiliers, en kWhef/(m².an)
   * (éq. 2487).
   */
  cef_kwhef_m2: number;
  /** « O_Cep_annuel », en kWhep/(m².an) (éq. 2488). */
  cep_kwhep_m2: number;
  /** « O_Cep_nr_annuel », en kWhep/(m².an) (éq. 2489). */
  cep_nr_kwhep_m2: number;
  /** Électricité consommée par poste (« O_Cef_cons_\[poste\]_elec_annuel »), en kWh. */
  electricite_consommee: PostesElectricite;
  /**
   * Électricité importée par poste (« O_Cef_imp_\[poste\]_elec_annuel »), en kWh : la part
   * autoconsommée de la production locale est déduite heure par heure (éq. 2535).
   */
  electricite_importee: PostesElectricite;
  /**
   * Électricité importée hors usages mobiliers, par mois des scénarios, en kWh (le forfait
   * de refroidissement, annuel, n'y est pas compris : §13.3.3.2).
   */
  electricite_importee_mensuel_kwh: number[];
  /**
   * Énergie solaire thermique captée et transmise aux ballons, en kWh : énergie
   * renouvelable captée sur le bâtiment, de coefficients nuls (article 9) ; pour information.
   */
  energie_solaire_captee_kwh?: number | null;
  /** Exigences et conformité ; absentes sans `exigences` dans l'entrée du bâtiment. */
  exigences?: ResultatsExigences | null;
  /**
   * Forfaits de consommation de refroidissement des groupes non climatisés dont le DH
   * dépasse le seuil bas (§13.3) ; absent sans forfait.
   */
  forfaits_refroidissement?: ForfaitRefroidissementGroupe[];
  /**
   * « Icénergie », en kg éq. CO2/m² (annexe II §4.3.2, éq. 74) ; absent si un facteur
   * d'émission manque.
   */
  ic_energie_kgco2_m2: number | null;
  /** Consommations importées par poste et par énergie, et leur conversion. */
  lignes: LigneBilanEnergie[];
  /** Bilan de la production photovoltaïque ; absent sans installation. */
  production_pv?: BilanProductionPv | null;
  /**
   * `Σ fCO2(a)` pour `a` de 1 à 50 (article 11) : poids de la consommation annuelle dans
   * l'Ic énergie.
   */
  somme_f_co2: number;
  /** « SREF_bat », en m². */
  sref_m2: number;
}

/** Production d'une installation photovoltaïque. */
export interface ResultatsInstallationPv {
  /**
   * « Pond » : production annuelle délivrée au réseau (sortie de l'onduleur), en énergie
   * finale, en kWh.
   */
  e_ond_kwh: number;
  /** Production de chacun des douze mois, en kWh (mois des scénarios). */
  e_ond_mensuel_kwh: number[];
  /** Énergie annuelle MPPT des modules, à l'entrée de l'onduleur, en kWh. */
  e_pv_kwh: number;
  /** « ηSTC » : rendement des modules en conditions STC (éq. 2366). */
  eta_stc: number;
  /** Rayonnement efficace annuel dans le plan des modules, en kWh/m². */
  g_kwh_m2: number;
  /** Identifiant de l'installation. */
  id: string;
  /** « N » : nombre de modules. */
  nb_modules: number;
  /** Nombre d'heures d'extinction de l'onduleur (« LIM = 0 »). */
  nbh_extinction: number;
  /** « PAC NOM » de l'onduleur, en W. */
  p_ac_nom_w: number;
  /** Puissance crête de calcul du champ, `N × Pc` de calcul, en kWc (§12.3.3.1). */
  puissance_crete_calcul_kwc: number;
  /** « U1 » : coefficient d'échange avec l'ambiance, en W/(m².K) (éq. 2367). */
  u1_w_m2k: number;
  /** Valeurs par défaut du texte appliquées. */
  valeurs_par_defaut?: ValeurParDefaut[];
}

/** Réseau de distribution intergroupes, calcul Th-C. */
export interface ResultatsIntergroupes {
  /** Identifiant de la génération qui l'alimente. */
  generation: string;
  /** Identifiant du réseau. */
  id: string;
  /** Nombre d'heures où le réseau transmet une demande. */
  nbh_fonct: number;
  /** Pertes du réseau hors volume chauffé, en kWh. */
  pertes_hvc_kwh: number;
  /** Pertes du réseau vers le volume chauffé, en kWh. */
  pertes_vc_kwh: number;
  /** Demande de chauffage transmise à la génération, pertes du réseau comprises, en kWh. */
  q_ch_kwh: number;
  /** Demandes mensuelles de chauffage, en kWh (mois des scénarios). */
  q_ch_mensuel_kwh: number[];
  /** Demande de refroidissement transmise à la génération (valeur absolue), en kWh. */
  q_fr_kwh: number;
  /** Demandes mensuelles de refroidissement, en kWh. */
  q_fr_mensuel_kwh: number[];
  /**
   * Température de départ extrême demandée à la génération (la plus haute en chauffage, la
   * plus basse en refroidissement), en °C ; absente pour un réseau fictif.
   */
  theta_dep_extreme_c: number | null;
  /** Consommation des circulateurs du réseau, en kWh d'énergie finale électrique. */
  w_aux_kwh: number;
}

/** Un parking (§10.3 et §10.4). */
export interface ResultatsParking {
  /** « Eec » : consommation annuelle de l'éclairage, en kWh (éq. 2354). */
  e_ecl_kwh: number;
  /** « Event » : consommation annuelle des ventilateurs, en kWh (éq. 2346). */
  e_vent_kwh: number;
  /** Identifiant du parking. */
  id: string;
  /** Valeurs par défaut du texte appliquées. */
  valeurs_par_defaut: ValeurParDefaut[];
}

/**
 * Production photovoltaïque d'un bâtiment, somme de ses installations (§13.2.3, éq. 2524).
 *
 * La part autoconsommée et la part exportée ne sont pas calculées ici : elles relèvent du
 * chapitre 13 (éq. 2527 à 2530) et demandent la consommation électrique horaire tous usages.
 * La série horaire (`Eelec_prod_PV(h)`) est disponible par `thbce_core::series_pv`.
 */
export interface ResultatsProductionPv {
  /** Production annuelle en énergie finale, `O_Eef_Prod_PV_annuel`, en kWh. */
  e_prod_pv_kwh: number;
  /** Production mensuelle, `O_Eef_Prod_PV_mois`, en kWh. */
  e_prod_pv_mensuel_kwh: number[];
  /** Installations. */
  installations: ResultatsInstallationPv[];
}

/**
 * Saisons propres d'un groupe (Th-BCE 2020 §4.6.3.6, éq. 92) : jours de l'année conventionnelle
 * de 1 à 365, au sens des scénarios (le jour 57 est le premier jour de mars). `null` si
 * l'événement n'a pas eu lieu.
 */
export interface ResultatsSaisons {
  /**
   * « jdebut_ch » : dernier jour où la saison de chauffage a commencé (hors début d'année,
   * où le chauffage est autorisé d'emblée).
   */
  j_debut_ch: number | null;
  /** « jdebut_fr » : dernier jour où la saison de refroidissement a commencé. */
  j_debut_fr: number | null;
  /** « jfin_ch » : dernier jour où la saison de chauffage s'est arrêtée. */
  j_fin_ch: number | null;
  /** « jfin_fr » : dernier jour où la saison de refroidissement s'est arrêtée. */
  j_fin_fr: number | null;
}

/**
 * Production solaire collective à appoints décentralisés (CESCI ou CESCAI, fiches 9.16 et
 * 9.17), calcul Th-C.
 */
export interface ResultatsSolaireCollectif {
  /**
   * Ballon central d'un CESCAI : `q_fou_sto_base_kwh` est l'énergie solaire,
   * `q_fou_ecs_kwh` l'énergie fournie au réseau intergroupe (ballons individuels et pertes
   * du réseau déduites).
   */
  ballon_central?: ResultatsStockage | null;
  /**
   * Ensembles de ballons individuels, pour tous les ballons de chaque ensemble ; l'énergie
   * injectée en zone basse (boucle solaire ou réseau intergroupe) est `q_fou_sto_base_kwh`,
   * celle des appoints intégrés `q_fou_sto_ap_kwh`.
   */
  ensembles: ResultatsStockage[];
  /** Identifiant de la production. */
  id: string;
  /** CESCAI : pertes du réseau intergroupe déduites du ballon central, en kWh (éq. 2080). */
  pertes_reseau_deduites_kwh: number;
  /**
   * CESCAI : énergie fournie aux ballons individuels par le réseau intergroupe, en kWh
   * (Σ « Qfou_ens^B », éq. 2078).
   */
  q_fou_reseau_kwh: number;
  /**
   * Boucle solaire collective (« Esol_tot^BS », « Eef_aux_tot^BS ») ; l'énergie solaire va
   * aux ballons individuels (CESCI) ou au ballon central (CESCAI).
   */
  solaire: ResultatsBoucleSolaire;
  /** « Type_PCAD ». */
  type_pcad: TypePcad;
}

/** Contribution d'un système solaire combiné au chauffage, calcul Th-C. */
export interface ResultatsSsc {
  /** Chauffage fourni par l'appoint raccordé à l'assemblage, en kWh (éq. 2150). */
  q_fou_ch_appoint_kwh: number;
  /**
   * « Qpuisé_ch_SSC » : chaleur de chauffage puisée dans le ballon, en kWh ; elle est
   * comprise dans l'énergie fournie en chauffage par la génération.
   */
  q_puise_ch_kwh: number;
}

/** Assemblage avec ballon de stockage d'ECS, calcul Th-C (§9.9, §9.10 et §9.14). */
export interface ResultatsStockage {
  /** Identifiant de l'assemblage ou du chauffe-eau thermodynamique. */
  id: string;
  /**
   * Paramètres Th-BCE identifiés à partir de l'essai d'un chauffe-eau thermodynamique
   * (§9.22).
   */
  idcet?: ResultatsIdCet | null;
  /**
   * « nbhreport_e » : nombre d'heures de l'année où l'énergie demandée n'a pas pu être
   * entièrement puisée dans le ballon (éq. 1780) ; le texte affiche une alerte au-delà de
   * 24 heures.
   */
  nbh_report_e: number;
  /**
   * Plus longue suite d'heures où la zone haute est restée sous la consigne (éq. 1959) ; le
   * texte juge le ballon incohérent avec le projet au-delà de 168 heures.
   */
  nbh_temp_insuff_max: number;
  /** « Φpertes » : pertes thermiques des ballons, en kWh (éq. 1771). */
  pertes_kwh: number;
  /**
   * « Qfou,ECS » : énergie fournie par les ballons à la distribution d'ECS, en kWh
   * (éq. 1954).
   */
  q_fou_ecs_kwh: number;
  /** « Qfou_sto_ap » : énergie fournie aux ballons par l'appoint intégré, en kWh. */
  q_fou_sto_ap_kwh: number;
  /** « Qfou_sto_base » : énergie fournie aux ballons par le générateur de base, en kWh. */
  q_fou_sto_base_kwh: number;
  /**
   * Boucle solaire d'un chauffe-eau solaire (§9.15), pour l'ensemble des assemblages
   * identiques.
   */
  solaire?: ResultatsBoucleSolaire | null;
  /** Boucle de chauffage d'un système solaire combiné (fiches 9.18 à 9.20). */
  ssc?: ResultatsSsc | null;
  /** Plus basse température de la zone haute en fin de pas de temps, en °C. */
  theta_haut_min_c: number;
  /** « θbz » : températures des zones au début du calcul, de bas en haut, en °C. */
  theta_zones_debut_c?: number[];
  /**
   * « θbz » : températures des zones à la fin du calcul, de bas en haut, en °C. Avec
   * `v_zones_l` et `theta_zones_debut_c`, elles donnent la variation du stock de chaleur des
   * ballons sur l'année, qui ferme leur bilan.
   */
  theta_zones_fin_c?: number[];
  /** « Vp » : volume d'eau puisé dans les ballons, en L. */
  v_puise_l: number;
  /**
   * « Vz » : volumes des quatre zones d'un ballon, de bas en haut, en L (un des assemblages
   * identiques).
   */
  v_zones_l?: number[];
  /** Valeurs par défaut de la méthode appliquées à l'assemblage. */
  valeurs_par_defaut?: ValeurParDefaut[];
}

/** Systèmes d'émission d'un groupe, calcul Th-C (§8.1). */
export interface ResultatsSystemesGroupe {
  /** Réseaux de distribution du groupe (§8.7 et §8.8). */
  distributions: ResultatsDistributionGroupe[];
  /** Émetteurs d'ECS équivalents du groupe et leurs distributions (§9.5 à 9.7). */
  ecs: ResultatsEcsEmetteur[];
  /** Énergies requises par émetteur (éq. 810). */
  emetteurs: ResultatsEmetteur[];
  /** Nombre d'heures de relance (éq. 848). */
  nbh_relance: number;
  /**
   * « Qsys_ch » : énergie de chauffage requise par les émetteurs du groupe, pertes au dos
   * comprises, en kWh (éq. 824).
   */
  q_sys_ch_kwh: number;
  /** Valeurs mensuelles de `q_sys_ch_kwh`, en kWh. */
  q_sys_ch_mensuel_kwh: number[];
  /** « Qsys_fr » : énergie de refroidissement requise (valeur absolue), en kWh. */
  q_sys_fr_kwh: number;
  /** Valeurs mensuelles de `q_sys_fr_kwh`, en kWh. */
  q_sys_fr_mensuel_kwh: number[];
  /** Pertes et consommations des systèmes récupérées par le groupe (§11.1), en kWh. */
  recup_kwh: number;
  /**
   * Saisons propres du groupe en calcul Th-C (§4.6), dont les générations tirent les
   * saisons effectives (§8.4).
   */
  saisons_th_c: ResultatsSaisons | null;
  /**
   * « Wvent_loc_tot » : consommation des ventilateurs locaux, en kWh d'énergie finale
   * électrique (éq. 815).
   */
  w_vent_loc_kwh: number;
}

/** Ventilation du projet vue d'un groupe (Th-BCE 2020 §6.2 à 6.5). */
export interface ResultatsVentilationGroupe {
  /**
   * Consommation annuelle des auxiliaires de ventilation du groupe, en kWh d'énergie finale
   * électrique (« Waux,v^gr », éq. 570 ; « Cvent^g », éq. 646).
   */
  c_aux_vent_kwh: number;
  /** La même, par m² de surface de référence du groupe, en kWh/m². */
  c_aux_vent_kwh_m2: number;
  /** Consommations mensuelles, en kWh, mois au sens des scénarios (§4.1.3). */
  c_aux_vent_mensuel_kwh: number[];
  /** Puissance des ventilateurs attribuée au groupe, en W (éq. 570, 642 et 644). */
  p_vent_w: number;
  /** « qrepris,cond » : débit repris vu par le ventilateur, en m³/h, négatif (éq. 430). */
  q_repris_cond_m3_h: number;
  /** « qsoufflé,cond » : débit soufflé vu par le ventilateur, en m³/h (éq. 431). */
  q_souffle_cond_m3_h: number;
  /**
   * « qspec_repris » : débit spécifique repris au groupe, fuites en volume chauffé
   * comprises, en m³/h, négatif (éq. 428).
   */
  q_spec_repris_m3_h: number;
  /**
   * « qspec_soufflé » : débit spécifique soufflé dans le groupe, fuites en volume chauffé
   * comprises, en m³/h (éq. 429).
   */
  q_spec_souffle_m3_h: number;
}

/** Fonctionnement annuel du système de ventilation du projet dans un calcul. */
export interface ResultatsVentilationMode {
  /** Nombre d'heures où l'échangeur d'un double flux est contourné par son bypass (éq. 494). */
  nbh_bypass: number;
  /** Énergie fournie pour la sécurité antigel de l'échangeur, en kWh (éq. 508). */
  q_ag_kwh: number;
}

/** Système de ventilation du projet d'une zone (Th-BCE 2020 §6.2 à 6.5). */
export interface ResultatsVentilationZone {
  /**
   * Consommation annuelle des ventilateurs, en kWh d'énergie finale électrique : la
   * puissance moyenne sommée heure par heure (éq. 565, 621 et 645).
   */
  c_aux_vent_kwh: number;
  /** Identifiant du système. */
  id: string;
  /**
   * Puissance moyenne des ventilateurs, en W (éq. 391 à 394, 620, 641 et 643), constante
   * en habitation.
   */
  p_vent_w: number;
  /**
   * Fonctionnement du système dans le calcul Th-C ; l'énergie de la sécurité antigel y est
   * demandée à la génération désignée par `generation_antigel`.
   */
  th_c?: ResultatsVentilationMode | null;
  /**
   * Fonctionnement du système dans le calcul Th-D ; absent si le calcul Th-D n'emploie pas
   * le système du projet.
   */
  th_d?: ResultatsVentilationMode | null;
  /**
   * Valeurs par défaut de la méthode appliquées au système et à ses bouches ; champ relatif
   * à la zone.
   */
  valeurs_par_defaut: ValeurParDefaut[];
}

/** Résultats d'une zone. */
export interface ResultatsZone {
  /** Besoins mensuels et annuels et Bbio de la zone (§13.1, « Sortie_Zone_B »). */
  bbio: ResultatsBbio | null;
  /**
   * Consommations en énergie finale des générations attribuées à la zone : somme de celles
   * de ses groupes. Absent sans génération.
   */
  consommations?: Consommations | null;
  /**
   * Postes propres au logement collectif et aux parkings attribués à la zone (§7.2, §10.1,
   * §10.3, §10.4). Absent sans logement collectif ni parking.
   */
  consommations_collectif?: ConsommationsCollectif | null;
  /** « DH » du groupe le plus défavorable de la zone, en °C.h (§13.5.3, éq. 2551). */
  dh_c_h: number | null;
  /** Résultats par groupe. */
  groupes: ResultatsGroupe[];
  /** Identifiant de la zone. */
  id: string;
  /** « Nadeq » : nombre d'adultes équivalents de la zone, en habitation (§4.1.3, éq. 32 et 36). */
  nadeq: number | null;
  /** « SREF_zn », en m² (§13.1.3, éq. 2377). */
  sref_m2: number;
  /** Système de ventilation du projet (§6.2 à 6.5) ; absent hors habitation. */
  ventilation?: ResultatsVentilationZone | null;
}

/** « idamont-air-type » (§8.26.3.3.1) : source amont sur air. */
export type SourceAmontSaisie = {
  air_exterieur: {
    /**
     * « Pvent_gainé » : pour une machine gainée, puissance des ventilateurs nécessaire
     * pour vaincre les pertes de charge des conduits, en W (éq. 1443). Absent : machine
     * non gainée, auxiliaires compris dans les performances (éq. 1442).
     */
    p_vent_gaine_w?: number | null;
  };
} | {
  air_ambiant: {
    /** Espace tampon dans lequel la machine puise sa chaleur. */
    espace_tampon: string;
    /** « Pvent_gainé », comme pour l'air extérieur. */
    p_vent_gaine_w?: number | null;
  };
} | {
  air_extrait: {
    /** « Pvent_gainé », comme pour l'air extérieur. */
    p_vent_gaine_w?: number | null;
    /** « Tair_lim » : température limite de l'air rejeté, en °C (éq. 1454). */
    t_air_lim_c: number;
    /** Zone dont le système de ventilation irrigue la source. */
    zone: string;
  };
};

/** « idstatut_echangeur » (§6.3.3.3.3). */
export type StatutEchangeur = "declare" | "justifie" | "certifie";

/**
 * Statut d'une valeur mesurée (§8.23.3.1) : certifiée par un organisme accrédité selon la
 * norme NF EN ISO/CEI 17065, ou justifiée par un essai d'un laboratoire accrédité selon la
 * norme NF EN ISO/CEI 17025.
 */
export type StatutMesure = "certifie" | "justifie";

/**
 * Statut des rendements d'une chaudière (§8.19.3.1) : une valeur certifiée est employée telle
 * quelle, une valeur justifiée est réduite de 10 %, une valeur déclarée de 20 %.
 */
export type StatutRendementChaudiere = "certifie" | "justifie" | "declare";

/**
 * Th-BCE 2020 §9.14.3.1, tableau 291 — assemblage d'un ballon de stockage d'ECS et de ses
 * générateurs : « base seule » sans `appoint`, « base et appoint intégré » avec.
 */
export interface StockageSaisi {
  /**
   * Générateur d'appoint intégré au ballon (« Type_prod_stockage = 1 ») ; absent : base
   * seule.
   */
  appoint?: AppointBallon | null;
  /** Ballon de stockage (§9.9). */
  ballon: BallonSaisi;
  /**
   * Générateur de base, dont l'échangeur ou la résistance est en partie basse du ballon
   * (« zbase = 1 », éq. 1947).
   */
  base: GenerateurBallon;
  /** Identifiant de l'assemblage, unique dans la génération. */
  id: string;
  /**
   * « Nbassemblage » : nombre d'assemblages identiques (au moins 1) ; le générateur de base
   * désigné représente alors autant de générateurs identiques (`rdim`).
   */
  nb_assemblages: number;
}

/**
 * Système solaire combiné (fiches 9.19 et 9.20), un assemblage par objet (« nbassembl = 1 »,
 * éq. 2133 et 2147).
 */
export interface SystemeSolaireCombine {
  /** Appoint intégré en zone 3, sonde en zone 3 (éq. 2130 et 2144), consigne de 55 °C. */
  appoint: AppointSsc;
  /** Ballon de stockage. */
  ballon: BallonSaisi;
  /** Boucle solaire (échangeur en zone 1, éq. 2128 et 2142). */
  boucle: BoucleSolaireSaisie;
  /** Capteurs solaires. */
  capteurs: CapteursSolaires;
  /** Orientation, inclinaison et masques du plan des capteurs (fiche 3.2). */
  exposition_capteurs: Exposition;
  /** Identifiant, unique parmi les assemblages de la génération. */
  id: string;
  /**
   * Appoint de chauffage : indépendant (« Typeprod_stockage = 4 », fiche 9.19) ou raccordé
   * à l'assemblage (« = 5 », fiche 9.20).
   */
  raccordement: RaccordementSsc;
}

/** Nature d'un système de ventilation. */
export type SystemeVentilation = {
  simple_flux_extraction: VentilateurSimpleFlux;
} | {
  simple_flux_insufflation: VentilateurSimpleFlux;
} | {
  mecanique_basse_pression: VentilateurSimpleFlux;
} | {
  double_flux: DoubleFlux;
} | "naturelle_par_conduits" | "hybride" | "aeration" | "centrale_traitement_air";

/** « Taux » et « Statut_taux » (§8.23.3.6, éq. 1277). */
export interface TauxAuxiliaires {
  /** Statut : une valeur justifiée est multipliée par 1,1. */
  statut: StatutMesure;
  /** « Taux », de 0 à 1. */
  valeur: number;
}

/** Th-BCE 2020 §10.1.2, tableau 309 — « TechMac » : typologie d'ascenseur. */
export type TechMac = "traction_avec_reduction" | "traction_sans_reduction" | "hydraulique" | "autre";

/** « Sys_thermo_Ch » (§8.23.3.2, tableau 145). */
export type TechnologieChauffagePac = "air_exterieur_eau" | "air_exterieur_air_recycle" | "air_extrait_air_neuf" | "eau_de_nappe_eau" | "eau_glycolee_eau" | "eau_de_nappe_air_recycle" | "eau_de_boucle_air_recycle" | "sol_eau" | "sol_sol";

/**
 * « Sys_thermo_Ecs » (§8.23.3.2, tableau 147). Températures aval de toutes les technologies :
 * 5, 15, 25, 35, 45, 55 et 65 °C, valeur pivot à 45 °C.
 */
export type TechnologieEcsPac = "air_exterieur_eau" | "air_extrait_eau" | "air_ambiant_eau" | "eau_de_nappe_eau" | "sol_eau" | "eau_glycolee_eau";

/** « Type_Techno_Capteur » (tableau 328). */
export type TechnologiePv = "mono_cristallin_si" | "multi_cristallin_si" | "cd_te" | "cis" | "amorphe_si" | "autre";

/** « Sys_thermo_Fr » (§8.23.3.2, tableau 145). */
export type TechnologieRefroidissementPac = "air_exterieur_eau" | "air_exterieur_air_recycle" | "air_extrait_air_neuf" | "eau_eau" | "eau_air" | "eau_de_nappe_air_recycle" | "eau_de_nappe_eau";

/**
 * Th-BCE 2020 §8.23.2, tableau 143 — générateur thermodynamique à compression électrique.
 *
 * Les modes décrits fixent la catégorie du générateur (§8.23.3.2) : chauffage seul ou
 * refroidissement seul (non réversible), chauffage et refroidissement (réversible).
 */
export interface ThermodynamiqueElectriqueSaisi {
  /** Mode chauffage ; absent : le générateur ne chauffe pas. */
  chauffage?: ModeChauffagePac | null;
  /** « Fonc_compr » : régulation de puissance du compresseur (§8.23.3.7.4). */
  compresseur: FonctionnementCompresseurSaisi;
  /**
   * Mode ECS ; absent : le générateur ne produit pas d'ECS. Un générateur thermodynamique
   * d'ECS est le générateur de base d'un assemblage avec stockage (§8.23.3.2) ; avec le mode
   * `chauffage`, il est double service.
   */
  ecs?: ModeEcsPac | null;
  /** Mode refroidissement ; absent : le générateur ne refroidit pas. */
  refroidissement?: ModeRefroidissementPac | null;
  /** Source amont (§8.26), cohérente avec la technologie de chaque mode. */
  source_amont: SourceAmontSaisie;
  /**
   * « Taux » et « Statut_taux » : part de la puissance des auxiliaires à charge nulle dans
   * la puissance absorbée au point pivot (éq. 1277). Absent : valeur par défaut, 0,02 pour
   * un générateur qui chauffe, 0,01 pour un refroidisseur seul (éq. 1278 et 1279), signalée
   * dans les résultats.
   */
  taux_auxiliaires?: TauxAuxiliaires | null;
}

/** Transfert de la boucle vers le ballon (§9.15.3.1). */
export type TransfertBoucle = {
  echangeur: {
    /** « (UA)hx », en W/K. Absent : valeur conventionnelle `100·A`, signalée. */
    ua_hx_w_k?: number | null;
  };
} | "sans_echangeur";

/** Type de capteur (tableau 293). */
export type TypeCapteurSolaire = "non_vitre" | "vitre" | "tubulaire";

/** Déclinaison d'une chaudière (§8.19.3.1, tableau 115). */
export type TypeChaudiereSaisi = "gaz_standard" | "gaz_basse_temperature" | "gaz_condensation" | "fioul_standard" | "fioul_condensation" | "bois";

/** Typologie d'émetteur de chaud (§8.1.3.1.3, tableau 85). */
export type TypeEmetteurChaud = "soufflage_air_chaud" | "muraux_rayonnants" | "plancher_chauffant" | "mur_chauffant" | "plafond_chauffant";

/** Typologie d'émetteur de froid (§8.1.3.1.3, tableau 86). */
export type TypeEmetteurFroid = "soufflage_air_froid" | "poutre_froide" | "plafond_rafraichissant" | "mur_rafraichissant" | "plancher_rafraichissant";

/** Type de local au sens de l'éclairage (§7.1.2, « type_local » ; tableau 78). */
export type TypeLocal = "bureau_standard" | "salle_reunion" | "circulation_accueil" | "sanitaires_collectifs" | "salle_classe" | "salle_repos" | "sanitaires_vestiaires" | "salle_enseignement_informatique" | "salle_conference" | "centre_documentation" | "salle_professeurs" | "salle_multi_fonctions" | "local_service" | "chambre_sans_cuisine" | "salle_petits_dejeuners" | "bar" | "salle_seminaires_reunion" | "salle_jeux" | "salle_restaurant" | "cuisine" | "aire_vente_inferieure300" | "aire_vente_superieure300" | "douches_collectives" | "salle_commune" | "locaux_soins_offices" | "salle_attente_consultation" | "aire_production" | "espace_voyageurs" | "commerces" | "inspection_filtrage" | "salle_sport";

/** Type d'ouvrant (§5.13.3.2.3, tableau 44). */
export type TypeOuvrant = "francaise_ou_anglaise" | "italienne_ou_soufflet" | "soufflet_oscillo_battant" | "coulissant";

/**
 * Th-BCE 2020 §10.3 et §10.4, tableau 322 — « Type » : parking intérieur (fermé) ou
 * extérieur (ouvert).
 */
export type TypeParking = "interieur" | "exterieur";

/** « Type_PCAD » (tableaux 294 et 295). */
export type TypePcad = "cesci" | "cescai" | "cesci_appoint_separe";

/** « Type-PM » (§5.9.2) : type de la première protection mobile. */
export type TypePm = "volet" | "store_enroulable" | "store_venitien";

/** « idtype_priorite » (§8.17.1) : mode de gestion des priorités entre générateurs. */
export type TypePriorite = "sans_priorite" | "cascade" | "alternance";

/** Type d'un réseau de chaleur (§8.28.3.1, tableaux 247 et 248). */
export type TypeReseauChaleurSaisi = "eau_chaude_basse_temperature" | "eau_chaude_haute_temperature" | "vapeur_basse_pression" | "vapeur_haute_pression";

/**
 * « Typo_emetteur » (§8.23.3.6, tableau 216) : inertie du système d'émission, qui fixe la
 * durée de fonctionnement à charge tendant vers zéro « Dfou0 ».
 */
export type TypoEmetteur = "plancher_ou_plafond_inertie_forte" | "radiateurs_ou_plafond_inertie_moyenne" | "ventilo_convecteurs_ou_inertie_faible" | "systemes_a_air";

/**
 * « Typo_PM » (§5.12.3.1, tableau 40) : typologie d'une protection mobile selon son taux de
 * passage d'air en position fermée.
 */
export type TypoPm = "etanche" | "faiblement_ajouree" | "ajouree" | "fortement_ajouree" | "tres_fortement_ajouree";

/**
 * Th-BCE 2020 §4.1.3, tableau 4 — usages d'une zone.
 *
 * Les 28 usages sont reconnus à la lecture. Le moteur calcule les usages 1 et 2 (décision Q4),
 * l'usage 3 (lot 0a) et les usages 4 à 7 et 12 (lot 0c1) ; les autres sont signalés « non pris
 * en charge ».
 */
export type UsageZone = "maison_individuelle" | "logement_collectif" | "bureaux" | "enseignement_primaire" | "enseignement_secondaire" | "mediatheque_bibliotheque" | "universitaire" | "hotel012_nuit" | "hotel345_nuit" | "hotel012_jour" | "hotel345_jour" | "petite_enfance" | "restaurant_continu" | "restaurant1_repas5_jours" | "restaurant2_repas7_jours" | "restaurant2_repas6_jours" | "commerces" | "vestiaires" | "sanitaire_avec_hebergement" | "sante_nuit" | "sante_jour" | "aerogare" | "industrie3x8" | "industrie8h18h" | "sport_municipal_scolaire" | "restaurant_scolaire1_repas" | "restaurant_scolaire3_repas" | "sport_prive";

/**
 * Valeur par défaut donnée par le texte de la méthode et appliquée à un paramètre non saisi.
 * Le moteur n'applique de valeur par défaut que lorsque le texte la donne, et la signale ici.
 * Ces valeurs sont des paramètres du calcul : l'arrondi de restitution ne leur est pas appliqué.
 */
export interface ValeurParDefaut {
  /**
   * Champ de l'entrée auquel la valeur s'applique, relatif à l'élément (par exemple
   * `sans_protection.sw1_c` pour le type d'une baie).
   */
  champ: string;
  /** Règle du texte dont elle provient. */
  regle: string;
  /** Valeur appliquée. */
  valeur: number;
}

/**
 * « δθvs_ch » (§8.1.3.1.2) : classe d'émetteur et hauteur sous plafond (tableau 82), cas des
 * poêles et inserts (tableau 83), ou valeur justifiée.
 */
export type VariationSpatiale = {
  classe: {
    /** Classe de l'émetteur. */
    classe: ClasseEmetteurChaud;
    /** Hauteur sous plafond, en m. */
    hsp_m: number;
  };
} | {
  poele_ou_insert: {
    /** Nombre de niveaux desservis : 1 ou 2. */
    niveaux: number;
  };
} | {
  valeur: {
    /** Variation spatiale, en K. */
    delta_theta_k: number;
  };
};

/** « δθvs_fr » (§8.1.3.1.2, tableau 84). */
export type VariationSpatialeFroid = {
  classe: {
    /** Classe de l'émetteur. */
    classe: ClasseEmetteurFroid;
    /** Hauteur sous plafond, en m. */
    hsp_m: number;
  };
} | {
  valeur: {
    /** Variation spatiale, en K. */
    delta_theta_k: number;
  };
};

/** « δθvt » (§8.1.3.1.5) : variation temporelle du couple régulateur/émetteur. */
export type VariationTemporelle = {
  certifie: {
    /** Variation temporelle, en K. */
    delta_theta_k: number;
  };
} | {
  justifie: {
    /** Variation temporelle justifiée, en K. */
    delta_theta_k: number;
  };
} | {
  defaut: {
    /** Vrai si le couple permet un arrêt total de l'émission. */
    arret_total: boolean;
  };
} | {
  poele_ou_insert: {
    /** Vrai avec thermostat d'ambiance, faux en régulation manuelle. */
    thermostat: boolean;
  };
};

/** Facteurs solaires d'une toiture végétalisée (§5.17.2). */
export interface Vegetalisation {
  /** « Sf-c,v,k » : facteur solaire pour le calcul du Cep. */
  sf_c_v_k: number;
  /** « Sf-e,v,k » : facteur solaire pour le calcul de DH. */
  sf_e_v_k: number;
}

/**
 * Th-BCE 2020 §6.4.3 et §6.5.2 — ventilateur d'un système simple flux : puissances en débit
 * de base et de pointe en résidentiel, en occupation et en inoccupation en non résidentiel.
 */
export interface VentilateurSimpleFlux {
  /**
   * « Pventbase » : puissance électrique du ventilateur en débit de base, en W. Requise en
   * maison individuelle ou accolée et en logement collectif ; ne pas la saisir dans les
   * autres usages.
   */
  p_vent_base_w?: number | null;
  /**
   * « Pventinocc » : puissance électrique du ventilateur en inoccupation, en non
   * résidentiel, en W (éq. 637 et 639). Requise comme `p_vent_occ_w` ; 0 si le ventilateur
   * est arrêté en inoccupation.
   */
  p_vent_inocc_w?: number | null;
  /**
   * « Pventocc » : puissance électrique du ventilateur en occupation, en non résidentiel,
   * en W (§6.5.3.3, éq. 636 et 638). Requise dans les usages autres que la maison
   * individuelle ou accolée et le logement collectif ; ne pas la saisir en habitation.
   */
  p_vent_occ_w?: number | null;
  /**
   * « Pventpointe » : puissance électrique du ventilateur en débit de pointe, en W. Requise
   * en maison individuelle ou accolée et en logement collectif.
   */
  p_vent_pointe_w?: number | null;
}

/** Th-BCE 2020 §8.1.3.8 — ventilateurs locaux d'un émetteur. */
export interface VentilateursLocaux {
  /** « GestVCV » : mode de gestion des ventilateurs (1 à 3). */
  gestion: GestionVentilateurs;
  /** « PVCV_GV » : puissance en grande vitesse, en W. */
  p_gv_w: number;
  /** « PVCV_MV » : puissance en moyenne vitesse, en W. */
  p_mv_w: number;
  /** « PVCV_PV » : puissance en petite vitesse, en W. */
  p_pv_w: number;
  /**
   * « PVCV_SPV » : puissance en super petite vitesse, en W. Requise avec
   * `super_petite_vitesse`.
   */
  p_spv_w?: number | null;
  /** « Qv_recirc_GV » : débit de recirculation en grande vitesse, en m³/h. */
  q_recirc_gv_m3_h: number;
  /** « Qv_recirc_MV » : débit de recirculation en moyenne vitesse, en m³/h. */
  q_recirc_mv_m3_h: number;
  /** « Qv_recirc_PV » : débit de recirculation en petite vitesse, en m³/h. */
  q_recirc_pv_m3_h: number;
  /**
   * « idregul_batt » : régulation de la batterie de refroidissement (sans objet pour un
   * émetteur qui ne refroidit pas).
   */
  regulation_batterie?: RegulationBatterie | null;
  /** « ispv » : régime de super petite vitesse automatique. */
  super_petite_vitesse?: boolean;
}

/**
 * Th-BCE 2020 §6.2 à 6.5 — système de ventilation mécanique d'une zone : nature du système
 * (« Type_ventilation_mecaniq », §6.3.3.1.1), ventilateurs et réseau de conduits. Les bouches
 * et conduits sont décrits par groupe (`bouches_conduits`) et rattachés à ce système.
 */
export interface Ventilation {
  /** Identifiant du système. */
  id: string;
  /**
   * Raccordement à un puits climatique ou hydraulique (§6.8 à 6.11) : non pris en charge
   * dans cette version.
   */
  puits?: Puits | null;
  /**
   * Réseau de conduits : étanchéité et parties hors volume chauffé (§6.2.3.4, §6.3.3.3.2,
   * §6.5.3.4).
   */
  reseau: ReseauAeraulique;
  /** Nature du système et paramètres propres. */
  systeme: SystemeVentilation;
}

/** Th-BCE 2020 §10.3.2 — ventilation forcée d'un parking d'habitat. */
export interface VentilationParking {
  /**
   * « Pvent600 » : puissance électrique active de l'installation par place, au point de
   * fonctionnement de 600 m³/h par place, en W/place. Absent : 40 W/place (éq. 2326).
   */
  p_vent600_w_par_place?: number | null;
  /** « Reg » : la ventilation est régulée (vrai) ou permanente 24 h/24 (faux). */
  regulee: boolean;
}

/** Vérification d'une exigence : `valeur ≤ maximum`. */
export interface VerificationExigence {
  /**
   * Groupe vérifié (DH : exigence par partie de bâtiment thermiquement homogène) ; absent
   * pour un indicateur du bâtiment.
   */
  groupe?: string | null;
  /** Indicateur : `bbio`, `cep`, `cep_nr`, `ic_energie` ou `dh`. */
  indicateur: string;
  /** Marge : `maximum − valeur` (positive si l'exigence est respectée). */
  marge: number;
  /** Marge relative au maximum, en %. */
  marge_pct: number;
  /** Valeur maximale. */
  maximum: number;
  /** Vrai si `valeur ≤ maximum`. */
  respectee: boolean;
  /** Valeur de l'indicateur. */
  valeur: number;
}

/** Vue de l'infrastructure depuis la baie (chapitre V, partie I, point 2). */
export type VueInfrastructure = "directe" | "partielle" | "masquee_peu_protecteurs" | "masquee_tres_protecteurs" | "arriere";

/** Niveau « zone » (Th-BCE 2020 §5.5). */
export interface Zone {
  /** Groupes de la zone. */
  groupes: Groupe[];
  /**
   * « hplancher » : hauteur du point le plus bas de la zone par rapport au sol extérieur, en
   * m (§5.6.2) ; négative si la zone descend sous le niveau du sol. Requise.
   */
  h_plancher_m?: number | null;
  /**
   * « hzone » : hauteur entre le point le plus bas et le point le plus haut de la zone, en m
   * (§5.6.2). Requise : elle fixe l'altitude des défauts d'étanchéité et des entrées d'air.
   */
  h_zone_m?: number | null;
  /** Identifiant de la zone. */
  id: string;
  /**
   * « Nb_lgt » (§4.1.2) : nombre de logements, en logement collectif et pour les maisons
   * accolées. Requis en logement collectif ; 1 par défaut en maison individuelle.
   */
  nb_logements?: number | null;
  /**
   * « Q4Pa_surf » : perméabilité à l'air de l'enveloppe sous 4 Pa, en m³/(h.m²) de parois
   * déperditives hors plancher bas (§5.6.2), la même pour tous les groupes de la zone.
   * Requise. Une valeur mesurée par échantillonnage se saisit multipliée par 1,2 ; si des
   * travaux pouvant affecter la perméabilité restent à réaliser après la livraison, on ajoute
   * 0,3 m³/(h.m²) (§5.6.3.3.2).
   */
  q4pa_surf_m3_h_m2?: number | null;
  /**
   * « δtrav_zone » : caractère traversant de la zone au sens des conditions d'hiver, baies et
   * portes intérieures fermées (§5.6.3.1.3, tableau 22). Conventionnel pour tous les usages
   * sauf le logement collectif : ne pas le saisir. En logement collectif, requis : vrai si
   * les logements sont traversants au sens du confort d'été et si les portes intérieures
   * d'une façade à l'autre sont détalonnées.
   */
  trav_zone?: boolean | null;
  /** « Usage zone » (§4.1.2). */
  usage: UsageZone;
  /**
   * Système de ventilation de la zone (Th-BCE 2020 §6 : « la modélisation du système de
   * ventilation s'effectue au niveau d'une zone »). Requis en habitation : il sert aux
   * calculs Th-C et Th-D ; le calcul du Bbio emploie le système conventionnel de la fiche 6.1
   * (§6.1.3). En logement collectif, un système qui dessert plusieurs zones se décrit dans
   * chacune, ses débits et ses puissances étant répartis au prorata des surfaces des zones
   * (§6, introduction).
   */
  ventilation?: Ventilation | null;
}

/** Zone de bruit (annexe de l'article R. 172-4, chapitre V). */
export type ZoneBruit = "br1" | "br2" | "br3";

/** Les huit zones climatiques réglementaires (annexe II §2.2.1.1). */
export type ZoneClimatique = "H1a" | "H1b" | "H1c" | "H2a" | "H2b" | "H2c" | "H2d" | "H3";

/**
 * Zone du plan d'exposition au bruit (PEB) d'un aérodrome (code de l'urbanisme, articles
 * R. 112-4 à R. 112-17) : donnée à saisir.
 */
export type ZonePeb = "a" | "b" | "c" | "d" | "hors_zone";
