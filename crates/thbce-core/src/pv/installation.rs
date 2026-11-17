// SPDX-License-Identifier: GPL-3.0-or-later
//! Fiche `S1_PV_installation_PV` (Th-BCE 2020 §12.1) : champ photovoltaïque intégré au bâtiment
//! et connecté au réseau, assemblage des modules (§12.3) et de l'onduleur (§12.4, figure 212).

use super::capteur::{CapteurPv, SortiesPasCapteur};
use super::entree::InstallationPv;
use super::onduleur::{OnduleurResolu, SortiesPasOnduleur};

/// Installation prête pour le calcul horaire.
#[derive(Debug, Clone, PartialEq)]
pub struct InstallationResolue {
    /// Modules (`PV_Capteurs_PV`).
    pub capteur: CapteurPv,
    /// Onduleur (`PV_Onduleur_PV`).
    pub onduleur: OnduleurResolu,
}

/// Sorties d'un pas de temps de l'installation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SortiesPasInstallation {
    /// Sorties des modules (`PPV` en W).
    pub capteur: SortiesPasCapteur,
    /// Sorties de l'onduleur (`Pond` en W).
    pub onduleur: SortiesPasOnduleur,
}

impl InstallationResolue {
    /// Résout une installation saisie.
    pub fn nouvelle(i: &InstallationPv) -> Self {
        let capteur = CapteurPv::nouveau(
            f64::from(i.nb_modules),
            i.modules.technologie,
            &i.modules.performances,
            i.modules.s_m2,
            i.confinement,
            i.exposition.inclinaison_deg,
        );
        let onduleur = OnduleurResolu::nouveau(&i.onduleur, capteur.n, capteur.performances.pc_w);
        Self { capteur, onduleur }
    }

    /// Un pas de temps : `PPV` est transmise à l'onduleur, qui délivre `Pond`.
    pub fn pas(
        &self,
        theta_deg: f64,
        drp_w_m2: f64,
        dfp_w_m2: f64,
        rrp_w_m2: f64,
        text_c: f64,
        tciel_c: f64,
    ) -> SortiesPasInstallation {
        let capteur = self
            .capteur
            .pas(theta_deg, drp_w_m2, dfp_w_m2, rrp_w_m2, text_c, tciel_c);
        let onduleur = self.onduleur.pas(capteur.p_pv_w);
        SortiesPasInstallation { capteur, onduleur }
    }
}
