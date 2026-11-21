### Ajouté

- Fiche 8.16 (lot 2f) : DRV (nature `drv`, type 509) et thermo-frigo-pompes (nature `thermofrigopompe`, type 508), générateurs thermodynamiques réversibles dont le rejet de chaleur en refroidissement (`Qfou + 0,6 · Qcons`, éq. 978) assure le chauffage du même pas de temps s'il suffit, sinon le générateur passe en chauffage, le froid étant assuré par son rejet froid (éq. 979) ; sortie `q_fou_ch_recup_kwh` des générateurs ; lecture Q243. Exemple `bureaux-drv-ademe`, tests `crates/thbce-io/tests/drv.rs`.

### Retiré

- Nature `boucle_d_eau_ou_drv` (chaîne refusée en code 3), remplacée par `drv` et `thermofrigopompe` (`{"drv": {...}}`, `{"thermofrigopompe": {...}}`) ; sans effet sur les entrées valides, puisqu'elle était refusée.
