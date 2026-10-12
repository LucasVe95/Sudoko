//! La résolution « à la main » : des déductions logiques, sans jamais deviner.
//!
//! Le solveur par retour sur trace (solveur.rs) essaie des chiffres et revient en arrière : il
//! trouve la solution de n'importe quelle grille, mais il ne dit rien de sa difficulté pour un
//! humain. Ici, on ne fait que des déductions sûres, dans l'ordre de la plus simple à la plus
//! subtile. La technique la plus difficile qu'il a fallu employer pour finir une grille mesure sa
//! vraie difficulté. Et comme chaque déduction est sûre, une grille résolue ainsi n'a forcément
//! qu'une solution.
//!
//! Les techniques raisonnent sur les zones de `Regles` : ce sont donc aussi celles des variantes
//! (les diagonales et les fenêtres sont des zones comme les autres).

use crate::grille::Niveau;
use crate::regles::{Regles, Zone};

// Le bit qui représente un chiffre : le bit 0 pour le 1, le bit 1 pour le 2, etc.
fn bit(valeur: u8) -> u16 {
    1 << (valeur - 1)
}

// Le plus petit chiffre d'un masque non nul
fn premier_chiffre(masque: u16) -> u8 {
    masque.trailing_zeros() as u8 + 1
}

// Les techniques, de la plus simple à la plus difficile : l'ordre de cette liste est celui
// dans lequel on les essaie, et celui de leur difficulté.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Technique {
    // Une case n'a plus qu'un chiffre possible
    CaseUnique,
    // Un chiffre n'a plus qu'une case possible dans une ligne, une colonne ou un bloc
    ChiffreUnique,
    // Dans un bloc, un chiffre est limité à une seule ligne (ou colonne) : il est impossible
    // ailleurs sur cette ligne. Et inversement, d'une ligne vers un bloc.
    Intersection,
    // Deux cases d'une zone n'ont que les deux mêmes chiffres : ils sont impossibles ailleurs
    PaireNue,
    // Deux chiffres n'ont que les deux mêmes cases dans une zone : ces cases n'ont que eux
    PaireCachee,
    // Trois cases d'une zone n'ont que les trois mêmes chiffres : ils sont impossibles ailleurs
    TripletNu,
    // Un chiffre est limité aux deux mêmes colonnes dans deux lignes (ou l'inverse) : il est
    // impossible ailleurs dans ces colonnes
    XWing,
}

impl Technique {
    pub const TOUTES: [Technique; 7] = [
        Technique::CaseUnique,
        Technique::ChiffreUnique,
        Technique::Intersection,
        Technique::PaireNue,
        Technique::PaireCachee,
        Technique::TripletNu,
        Technique::XWing,
    ];

    pub fn libelle(self) -> &'static str {
        match self {
            Technique::CaseUnique => "Case unique",
            Technique::ChiffreUnique => "Chiffre unique",
            Technique::Intersection => "Intersection",
            Technique::PaireNue => "Paire nue",
            Technique::PaireCachee => "Paire cachée",
            Technique::TripletNu => "Triplet nu",
            Technique::XWing => "X-Wing",
        }
    }

    // Le niveau d'une grille dont la technique la plus difficile est celle-ci
    pub fn niveau(self) -> Niveau {
        match self {
            Technique::CaseUnique | Technique::ChiffreUnique => Niveau::Facile,
            Technique::Intersection | Technique::PaireNue => Niveau::Moyen,
            Technique::PaireCachee | Technique::TripletNu | Technique::XWing => Niveau::Difficile,
        }
    }
}

// La technique la plus difficile qu'une grille de ce niveau peut demander
pub fn technique_max(niveau: Niveau) -> Technique {
    match niveau {
        Niveau::Facile => Technique::ChiffreUnique,
        Niveau::Moyen => Technique::PaireNue,
        Niveau::Difficile => Technique::XWing,
    }
}

// Une déduction : ce que la technique a trouvé et ce qu'elle change
#[derive(Clone, Debug, PartialEq)]
pub struct Pas {
    pub technique: Technique,
    // La case remplie par cette déduction : (case, chiffre)
    pub placement: Option<(usize, u8)>,
    // Les possibilités qu'elle retire : (case, chiffres retirés en bits)
    pub eliminations: Vec<(usize, u16)>,
    // La zone où la technique s'applique, et la zone qui la croise (intersection, X-Wing)
    pub zone: Option<Zone>,
    pub autre_zone: Option<Zone>,
    // Les chiffres concernés, en bits
    pub chiffres: u16,
    // Les cases qui forment la déduction (la paire, le triplet, les coins du X-Wing...)
    pub cases: Vec<usize>,
}

// Les chiffres d'un masque, écrits en français : « 2 », « 2 et 5 », « 2, 5 et 7 »
fn chiffres_texte(masque: u16) -> String {
    let chiffres: Vec<String> = (1..=16u8)
        .filter(|&chiffre| masque & bit(chiffre) != 0)
        .map(|chiffre| chiffre.to_string())
        .collect();
    match chiffres.split_last() {
        None => String::new(),
        Some((dernier, [])) => dernier.clone(),
        Some((dernier, debut)) => format!("{} et {}", debut.join(", "), dernier),
    }
}

impl Pas {
    // Explique la déduction à un joueur
    pub fn explication(&self, geometrie: &Regles) -> String {
        let cases: Vec<String> = self
            .cases
            .iter()
            .map(|&case| format!("({})", geometrie.case_texte(case)))
            .collect();
        let cases = cases.join(", ");
        let chiffres = chiffres_texte(self.chiffres);
        let zone = self.zone.map(Zone::dans).unwrap_or_default();

        match self.technique {
            Technique::CaseUnique => format!(
                "Case unique : en {}, un seul chiffre reste possible, le {}.",
                geometrie.case_texte(self.cases[0]),
                chiffres
            ),
            Technique::ChiffreUnique => format!(
                "Chiffre unique : {}, le {} ne peut aller que dans une seule case, en {}.",
                zone,
                chiffres,
                geometrie.case_texte(self.cases[0])
            ),
            Technique::Intersection => {
                let autre = self.autre_zone.expect("une intersection a deux zones");
                format!(
                    "Intersection : {}, le {} ne peut aller que {} ; on peut donc le retirer des autres cases {}.",
                    zone,
                    chiffres,
                    autre.dans(),
                    autre.de()
                )
            }
            Technique::PaireNue => format!(
                "Paire nue : {}, les cases {} n'ont plus que les chiffres {} ; ils sont donc impossibles dans les autres cases.",
                zone, cases, chiffres
            ),
            Technique::TripletNu => format!(
                "Triplet nu : {}, les cases {} n'ont plus que les chiffres {} ; ils sont donc impossibles dans les autres cases.",
                zone, cases, chiffres
            ),
            Technique::PaireCachee => format!(
                "Paire cachée : {}, les chiffres {} ne peuvent aller que dans les cases {} ; tout autre chiffre y est donc impossible.",
                zone, chiffres, cases
            ),
            Technique::XWing => {
                // Les lignes (ou colonnes) de base, et les colonnes (ou lignes) qui les croisent
                let (base, croisee) = match self.zone {
                    Some(Zone::Ligne(_)) => ("lignes", "colonnes"),
                    _ => ("colonnes", "lignes"),
                };
                let cote = geometrie.cote();
                let mut croisees: Vec<usize> = self
                    .cases
                    .iter()
                    .map(|&case| if base == "lignes" { case % cote } else { case / cote } + 1)
                    .collect();
                croisees.sort_unstable();
                croisees.dedup();
                let croisees: Vec<String> = croisees.iter().map(usize::to_string).collect();
                format!(
                    "X-Wing : pour le {}, dans les deux {} concernées, il ne peut aller que dans les {} {} ; on peut le retirer des autres cases de ces {}.",
                    chiffres,
                    base,
                    croisee,
                    croisees.join(" et "),
                    croisee
                )
            }
        }
    }
}

// L'état d'une résolution : les chiffres posés et, pour chaque case, les chiffres encore possibles
pub struct Logique<'a> {
    geometrie: &'a Regles,
    cases: Vec<u8>,
    // Les chiffres encore possibles dans chaque case, en bits. Une case remplie n'a que le sien.
    candidats: Vec<u16>,
}

impl<'a> Logique<'a> {
    // Prépare la résolution d'une grille (0 = case vide). Renvoie None si elle contient un doublon
    // ou un chiffre hors limites.
    pub fn new(geometrie: &'a Regles, cases: &[u8]) -> Option<Logique<'a>> {
        let cote = geometrie.cote();
        if cases.len() != cote * cote || cases.iter().any(|&valeur| valeur as usize > cote) {
            return None;
        }
        // Aucun chiffre ne doit être répété dans les cases qui se l'interdisent
        let doublon = cases.iter().enumerate().any(|(case, &valeur)| {
            valeur != 0 && geometrie.voisines[case].iter().any(|&autre| cases[autre] == valeur)
        });
        if doublon {
            return None;
        }
        Some(Logique::depuis_cases_valides(geometrie, cases))
    }

    // Comme `new`, pour une grille dont on sait déjà qu'elle est cohérente (génération)
    pub fn depuis_cases_valides(geometrie: &'a Regles, cases: &[u8]) -> Logique<'a> {
        let tous = ((1u32 << geometrie.cote()) - 1) as u16;
        let mut logique = Logique {
            geometrie,
            cases: cases.to_vec(),
            candidats: vec![tous; cases.len()],
        };
        for (case, &valeur) in cases.iter().enumerate() {
            if valeur != 0 {
                logique.candidats[case] = bit(valeur);
            }
        }
        for (case, &valeur) in cases.iter().enumerate() {
            if valeur != 0 {
                for &voisine in &geometrie.voisines[case] {
                    logique.candidats[voisine] &= !bit(valeur);
                }
            }
        }
        logique
    }

    #[cfg(test)]
    pub fn cases(&self) -> &[u8] {
        &self.cases
    }

    pub fn est_resolue(&self) -> bool {
        self.cases.iter().all(|&valeur| valeur != 0)
    }

    fn poser(&mut self, case: usize, valeur: u8) {
        let geometrie = self.geometrie;
        self.cases[case] = valeur;
        self.candidats[case] = bit(valeur);
        for &voisine in &geometrie.voisines[case] {
            self.candidats[voisine] &= !bit(valeur);
        }
    }

    // Applique une déduction
    pub fn appliquer(&mut self, pas: &Pas) {
        if let Some((case, valeur)) = pas.placement {
            self.poser(case, valeur);
        }
        for &(case, retires) in &pas.eliminations {
            self.candidats[case] &= !retires;
        }
    }

    // La déduction la plus simple possible, sans employer de technique plus difficile que
    // `permise`. None si on est bloqué.
    pub fn prochain_pas(&self, permise: Technique) -> Option<Pas> {
        Technique::TOUTES
            .into_iter()
            .take_while(|&technique| technique <= permise)
            .find_map(|technique| match technique {
                Technique::CaseUnique => self.case_unique(),
                Technique::ChiffreUnique => self.chiffre_unique(),
                Technique::Intersection => self.intersection(),
                Technique::PaireNue => self.sous_ensemble_nu(2),
                Technique::PaireCachee => self.paire_cachee(),
                Technique::TripletNu => self.sous_ensemble_nu(3),
                Technique::XWing => self.x_wing(),
            })
    }

    // Résout la grille par déduction, avec les techniques jusqu'à `permise`.
    // Renvoie la technique la plus difficile qu'il a fallu employer (CaseUnique pour une grille
    // déjà pleine), ou None si on est bloqué avant la fin : la grille demande alors de deviner,
    // ou une technique plus difficile.
    pub fn resoudre(&mut self, permise: Technique) -> Option<Technique> {
        let mut plus_difficile = Technique::CaseUnique;
        while !self.est_resolue() {
            let pas = self.prochain_pas(permise)?;
            plus_difficile = plus_difficile.max(pas.technique);
            self.appliquer(&pas);
        }
        Some(plus_difficile)
    }

    // Une case vide qui n'a qu'un chiffre possible
    fn case_unique(&self) -> Option<Pas> {
        let case = (0..self.cases.len())
            .find(|&case| self.cases[case] == 0 && self.candidats[case].count_ones() == 1)?;
        let valeur = premier_chiffre(self.candidats[case]);
        Some(Pas {
            technique: Technique::CaseUnique,
            placement: Some((case, valeur)),
            eliminations: Vec::new(),
            zone: None,
            autre_zone: None,
            chiffres: bit(valeur),
            cases: vec![case],
        })
    }

    // Un chiffre qui n'a qu'une case possible dans une zone
    fn chiffre_unique(&self) -> Option<Pas> {
        for (indice, unite) in self.geometrie.zones.iter().enumerate() {
            // `une` : les chiffres possibles dans au moins une case ; `deux` : dans au moins deux
            let (mut une, mut deux, mut places) = (0u16, 0u16, 0u16);
            for &case in unite {
                if self.cases[case] != 0 {
                    places |= bit(self.cases[case]);
                } else {
                    let candidats = self.candidats[case];
                    deux |= une & candidats;
                    une |= candidats;
                }
            }
            let uniques = une & !deux & !places;
            if uniques != 0 {
                let valeur = premier_chiffre(uniques);
                let case = *unite
                    .iter()
                    .find(|&&case| self.cases[case] == 0 && self.candidats[case] & bit(valeur) != 0)?;
                return Some(Pas {
                    technique: Technique::ChiffreUnique,
                    placement: Some((case, valeur)),
                    eliminations: Vec::new(),
                    zone: Some(self.geometrie.zone(indice)),
                    autre_zone: None,
                    chiffres: bit(valeur),
                    cases: vec![case],
                });
            }
        }
        None
    }

    // Les cases vides d'une zone où un chiffre reste possible
    fn cases_possibles(&self, unite: usize, masque: u16) -> Vec<usize> {
        self.geometrie.zones[unite]
            .iter()
            .copied()
            .filter(|&case| self.cases[case] == 0 && self.candidats[case] & masque != 0)
            .collect()
    }

    // Un chiffre limité, dans une zone A, aux cases qu'elle partage avec une zone B : il est
    // impossible dans les autres cases de B
    fn intersection(&self) -> Option<Pas> {
        let geometrie = self.geometrie;
        for &(a, b) in &geometrie.croisements {
            for valeur in 1..=geometrie.cote() as u8 {
                let masque = bit(valeur);
                let dans_a = self.cases_possibles(a, masque);
                if dans_a.len() < 2 || !dans_a.iter().all(|&case| geometrie.contient[b][case]) {
                    continue;
                }
                let eliminations: Vec<(usize, u16)> = self
                    .cases_possibles(b, masque)
                    .into_iter()
                    .filter(|&case| !geometrie.contient[a][case])
                    .map(|case| (case, masque))
                    .collect();
                if !eliminations.is_empty() {
                    return Some(Pas {
                        technique: Technique::Intersection,
                        placement: None,
                        eliminations,
                        zone: Some(geometrie.zone(a)),
                        autre_zone: Some(geometrie.zone(b)),
                        chiffres: masque,
                        cases: dans_a,
                    });
                }
            }
        }
        None
    }

    // `k` cases d'une zone qui n'ont à elles toutes que `k` chiffres possibles : ces chiffres
    // sont impossibles dans les autres cases de la zone (k = 2 : paire nue, k = 3 : triplet nu)
    fn sous_ensemble_nu(&self, k: usize) -> Option<Pas> {
        let technique = if k == 2 { Technique::PaireNue } else { Technique::TripletNu };
        for (indice, unite) in self.geometrie.zones.iter().enumerate() {
            // Seules comptent les cases vides qui ont entre 2 et k chiffres possibles
            let candidates: Vec<usize> = unite
                .iter()
                .copied()
                .filter(|&case| {
                    self.cases[case] == 0 && (2..=k as u32).contains(&self.candidats[case].count_ones())
                })
                .collect();
            if candidates.len() < k {
                continue;
            }
            let mut choisies = Vec::with_capacity(k);
            if let Some(pas) = self.chercher_sous_ensemble(indice, &candidates, 0, &mut choisies, 0, k, technique) {
                return Some(pas);
            }
        }
        None
    }

    // Essaie toutes les façons de choisir `k` cases parmi `candidates` (à partir de `debut`) dont
    // les chiffres possibles réunis (`union`) sont au nombre de `k`
    #[allow(clippy::too_many_arguments)]
    fn chercher_sous_ensemble(
        &self,
        unite: usize,
        candidates: &[usize],
        debut: usize,
        choisies: &mut Vec<usize>,
        union: u16,
        k: usize,
        technique: Technique,
    ) -> Option<Pas> {
        if choisies.len() == k {
            if union.count_ones() as usize != k {
                return None;
            }
            let eliminations: Vec<(usize, u16)> = self.geometrie.zones[unite]
                .iter()
                .copied()
                .filter(|case| !choisies.contains(case) && self.cases[*case] == 0)
                .filter_map(|case| {
                    let retires = self.candidats[case] & union;
                    (retires != 0).then_some((case, retires))
                })
                .collect();
            if eliminations.is_empty() {
                return None;
            }
            return Some(Pas {
                technique,
                placement: None,
                eliminations,
                zone: Some(self.geometrie.zone(unite)),
                autre_zone: None,
                chiffres: union,
                cases: choisies.clone(),
            });
        }
        for position in debut..candidates.len() {
            let case = candidates[position];
            let nouvelle_union = union | self.candidats[case];
            if nouvelle_union.count_ones() as usize > k {
                continue;
            }
            choisies.push(case);
            let trouve =
                self.chercher_sous_ensemble(unite, candidates, position + 1, choisies, nouvelle_union, k, technique);
            choisies.pop();
            if trouve.is_some() {
                return trouve;
            }
        }
        None
    }

    // Deux chiffres qui n'ont que les deux mêmes cases dans une zone : ces deux cases ne peuvent
    // contenir que ces deux chiffres
    fn paire_cachee(&self) -> Option<Pas> {
        let cote = self.geometrie.cote();
        for (indice, unite) in self.geometrie.zones.iter().enumerate() {
            // Pour chaque chiffre, ses places dans la zone (un bit par case de la zone)
            let mut places = [0u16; 16];
            let mut posés = 0u16;
            for (rang, &case) in unite.iter().enumerate() {
                if self.cases[case] != 0 {
                    posés |= bit(self.cases[case]);
                    continue;
                }
                let mut masque = self.candidats[case];
                while masque != 0 {
                    places[masque.trailing_zeros() as usize] |= 1 << rang;
                    masque &= masque - 1;
                }
            }
            for premier in 0..cote {
                if posés >> premier & 1 != 0 || places[premier].count_ones() != 2 {
                    continue;
                }
                for second in premier + 1..cote {
                    if posés >> second & 1 != 0 || places[second] != places[premier] {
                        continue;
                    }
                    let paire = (1u16 << premier) | (1u16 << second);
                    let cases: Vec<usize> = unite
                        .iter()
                        .enumerate()
                        .filter(|&(rang, _)| places[premier] >> rang & 1 != 0)
                        .map(|(_, &case)| case)
                        .collect();
                    let eliminations: Vec<(usize, u16)> = cases
                        .iter()
                        .filter_map(|&case| {
                            let autres = self.candidats[case] & !paire;
                            (autres != 0).then_some((case, autres))
                        })
                        .collect();
                    if !eliminations.is_empty() {
                        return Some(Pas {
                            technique: Technique::PaireCachee,
                            placement: None,
                            eliminations,
                            zone: Some(self.geometrie.zone(indice)),
                            autre_zone: None,
                            chiffres: paire,
                            cases,
                        });
                    }
                }
            }
        }
        None
    }

    // Un chiffre limité aux deux mêmes colonnes dans deux lignes : il est impossible dans les
    // autres cases de ces colonnes. Et de même en échangeant lignes et colonnes.
    fn x_wing(&self) -> Option<Pas> {
        let geometrie = self.geometrie;
        let cote = geometrie.cote();
        // (zones de base, zones qui les croisent) : les lignes puis les colonnes
        for (base, croisees) in [(0, cote), (cote, 0)] {
            for valeur in 1..=cote as u8 {
                let masque = bit(valeur);
                // Les zones de base où le chiffre n'a que deux places, avec ces places
                let mut paires: Vec<(usize, u16)> = Vec::new();
                for rang in 0..cote {
                    let places = geometrie.zones[base + rang]
                        .iter()
                        .enumerate()
                        .filter(|&(_, &case)| self.cases[case] == 0 && self.candidats[case] & masque != 0)
                        .fold(0u16, |places, (position, _)| places | 1 << position);
                    if places.count_ones() == 2 {
                        paires.push((rang, places));
                    }
                }
                for (i, &(premiere, places)) in paires.iter().enumerate() {
                    for &(seconde, autres_places) in &paires[i + 1..] {
                        if autres_places != places {
                            continue;
                        }
                        let (zone_a, zone_b) = (base + premiere, base + seconde);
                        let mut eliminations = Vec::new();
                        let mut coins = Vec::new();
                        for position in 0..cote {
                            if places >> position & 1 == 0 {
                                continue;
                            }
                            for &case in &geometrie.zones[croisees + position] {
                                if geometrie.contient[zone_a][case] || geometrie.contient[zone_b][case] {
                                    coins.push(case);
                                } else if self.cases[case] == 0 && self.candidats[case] & masque != 0 {
                                    eliminations.push((case, masque));
                                }
                            }
                        }
                        if !eliminations.is_empty() {
                            return Some(Pas {
                                technique: Technique::XWing,
                                placement: None,
                                eliminations,
                                zone: Some(geometrie.zone(zone_a)),
                                autre_zone: Some(geometrie.zone(zone_b)),
                                chiffres: masque,
                                cases: coins,
                            });
                        }
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grille::{Grille, Taille};
    use crate::solveur::Solveur;
    use std::sync::Arc;

    fn geometrie(taille: Taille) -> Arc<Regles> {
        Regles::classique(taille)
    }

    // La grille d'entraînement : une grille de journal facile
    fn journal() -> Vec<u8> {
        crate::grille::DEPART
            .chars()
            .filter_map(|c| c.to_digit(10))
            .map(|chiffre| chiffre as u8)
            .collect()
    }

    // Une résolution sur une grille vide 9×9, dont on fixe à la main les candidats
    fn vide_9x9(geometrie: &Regles) -> Logique<'_> {
        Logique::depuis_cases_valides(geometrie, &[0u8; 81])
    }

    #[test]
    fn test_les_techniques_sont_classees_par_difficulte() {
        let mut triees = Technique::TOUTES;
        triees.sort();
        assert_eq!(triees, Technique::TOUTES);
        assert!(Technique::CaseUnique < Technique::XWing);
        assert_eq!(Technique::CaseUnique.niveau(), Niveau::Facile);
        assert_eq!(Technique::ChiffreUnique.niveau(), Niveau::Facile);
        assert_eq!(Technique::Intersection.niveau(), Niveau::Moyen);
        assert_eq!(Technique::PaireNue.niveau(), Niveau::Moyen);
        assert_eq!(Technique::PaireCachee.niveau(), Niveau::Difficile);
        assert_eq!(Technique::XWing.niveau(), Niveau::Difficile);
    }

    #[test]
    fn test_la_technique_maximale_d_un_niveau_est_de_ce_niveau() {
        for niveau in Niveau::TOUS {
            let maximale = technique_max(niveau);
            assert_eq!(maximale.niveau(), niveau);
            // Toute technique plus difficile appartient à un niveau plus élevé
            for technique in Technique::TOUTES {
                if technique > maximale {
                    assert_ne!(technique.niveau(), niveau);
                }
            }
        }
    }

    #[test]
    fn test_case_unique() {
        let g = geometrie(Taille::Neuf);
        let mut logique = vide_9x9(&g);
        logique.candidats[10] = bit(5);
        let pas = logique.prochain_pas(Technique::CaseUnique).unwrap();
        assert_eq!(pas.technique, Technique::CaseUnique);
        assert_eq!(pas.placement, Some((10, 5)));
        assert_eq!(pas.cases, vec![10]);
        logique.appliquer(&pas);
        assert_eq!(logique.cases[10], 5);
        // Le 5 disparaît des voisines : la ligne, la colonne et le bloc
        assert_eq!(logique.candidats[11] & bit(5), 0);
        assert_eq!(logique.candidats[1] & bit(5), 0);
        assert_eq!(logique.candidats[0] & bit(5), 0);
        assert_ne!(logique.candidats[80] & bit(5), 0);
    }

    #[test]
    fn test_chiffre_unique() {
        let g = geometrie(Taille::Neuf);
        let mut logique = vide_9x9(&g);
        // Dans la ligne 0, le 5 ne reste possible que dans la case 3
        for case in (0..9).filter(|&case| case != 3) {
            logique.candidats[case] &= !bit(5);
        }
        let pas = logique.prochain_pas(Technique::ChiffreUnique).unwrap();
        assert_eq!(pas.technique, Technique::ChiffreUnique);
        assert_eq!(pas.placement, Some((3, 5)));
        assert_eq!(pas.zone, Some(Zone::Ligne(0)));
        // Avec la seule technique CaseUnique, on ne le voit pas
        assert!(logique.prochain_pas(Technique::CaseUnique).is_none());
    }

    #[test]
    fn test_intersection_d_un_bloc_vers_une_ligne() {
        let g = geometrie(Taille::Neuf);
        let mut logique = vide_9x9(&g);
        // Dans le bloc 0, le 7 n'est possible que dans la ligne 0 (cases 0, 1, 2)
        for ligne in 1..3 {
            for col in 0..3 {
                logique.candidats[ligne * 9 + col] &= !bit(7);
            }
        }
        let pas = logique.prochain_pas(Technique::Intersection).unwrap();
        assert_eq!(pas.technique, Technique::Intersection);
        assert_eq!(pas.zone, Some(Zone::Bloc(0)));
        assert_eq!(pas.autre_zone, Some(Zone::Ligne(0)));
        assert_eq!(pas.chiffres, bit(7));
        assert_eq!(pas.cases, vec![0, 1, 2]);
        // Le 7 est retiré du reste de la ligne 0
        let cases: Vec<usize> = pas.eliminations.iter().map(|&(case, _)| case).collect();
        assert_eq!(cases, vec![3, 4, 5, 6, 7, 8]);
        assert!(pas.eliminations.iter().all(|&(_, retires)| retires == bit(7)));
    }

    #[test]
    fn test_intersection_d_une_colonne_vers_un_bloc() {
        let g = geometrie(Taille::Neuf);
        let mut logique = vide_9x9(&g);
        // Dans la colonne 0, le 4 n'est possible que dans les lignes 0 à 2 : le bloc 0
        for ligne in 3..9 {
            logique.candidats[ligne * 9] &= !bit(4);
        }
        let pas = logique.prochain_pas(Technique::Intersection).unwrap();
        assert_eq!(pas.zone, Some(Zone::Colonne(0)));
        assert_eq!(pas.autre_zone, Some(Zone::Bloc(0)));
        // Le 4 est retiré des cases du bloc 0 hors de la colonne 0
        let cases: Vec<usize> = pas.eliminations.iter().map(|&(case, _)| case).collect();
        assert_eq!(cases, vec![1, 2, 10, 11, 19, 20]);
    }

    #[test]
    fn test_paire_nue() {
        let g = geometrie(Taille::Neuf);
        let mut logique = vide_9x9(&g);
        logique.candidats[0] = bit(1) | bit(2);
        logique.candidats[1] = bit(1) | bit(2);
        let pas = logique.prochain_pas(Technique::PaireNue).unwrap();
        assert_eq!(pas.technique, Technique::PaireNue);
        assert_eq!(pas.zone, Some(Zone::Ligne(0)));
        assert_eq!(pas.chiffres, bit(1) | bit(2));
        assert_eq!(pas.cases, vec![0, 1]);
        let cases: Vec<usize> = pas.eliminations.iter().map(|&(case, _)| case).collect();
        assert_eq!(cases, vec![2, 3, 4, 5, 6, 7, 8]);
        assert!(pas.eliminations.iter().all(|&(_, retires)| retires == bit(1) | bit(2)));
    }

    #[test]
    fn test_triplet_nu() {
        let g = geometrie(Taille::Neuf);
        let mut logique = vide_9x9(&g);
        logique.candidats[0] = bit(1) | bit(2);
        logique.candidats[1] = bit(2) | bit(3);
        logique.candidats[2] = bit(1) | bit(3);
        // Pas de paire nue ici, mais un triplet
        assert!(logique.prochain_pas(Technique::PaireNue).is_none());
        let pas = logique.prochain_pas(Technique::TripletNu).unwrap();
        assert_eq!(pas.technique, Technique::TripletNu);
        assert_eq!(pas.chiffres, bit(1) | bit(2) | bit(3));
        assert_eq!(pas.cases, vec![0, 1, 2]);
        assert_eq!(pas.zone, Some(Zone::Ligne(0)));
    }

    #[test]
    fn test_paire_cachee() {
        let g = geometrie(Taille::Neuf);
        let mut logique = vide_9x9(&g);
        // Dans la ligne 0, les chiffres 1 et 2 ne restent possibles que dans les cases 0 et 1
        for case in 2..9 {
            logique.candidats[case] &= !(bit(1) | bit(2));
        }
        // Les chiffres 1 et 2, limités à deux cases du même bloc, donnent aussi une intersection,
        // plus simple : on interroge donc directement la paire cachée
        let pas = logique.paire_cachee().unwrap();
        assert_eq!(pas.technique, Technique::PaireCachee);
        assert_eq!(pas.zone, Some(Zone::Ligne(0)));
        assert_eq!(pas.chiffres, bit(1) | bit(2));
        assert_eq!(pas.cases, vec![0, 1]);
        // Ces deux cases perdent tous leurs autres chiffres
        let tous = ((1u32 << 9) - 1) as u16;
        assert_eq!(pas.eliminations, vec![(0, tous & !(bit(1) | bit(2))), (1, tous & !(bit(1) | bit(2)))]);
    }

    #[test]
    fn test_x_wing_sur_les_lignes() {
        let g = geometrie(Taille::Neuf);
        let mut logique = vide_9x9(&g);
        // Le 3 n'est possible, dans les lignes 1 et 5, que dans les colonnes 2 et 6
        for ligne in [1, 5] {
            for col in (0..9).filter(|&col| col != 2 && col != 6) {
                logique.candidats[ligne * 9 + col] &= !bit(3);
            }
        }
        let pas = logique.prochain_pas(Technique::XWing).unwrap();
        assert_eq!(pas.technique, Technique::XWing);
        assert_eq!(pas.zone, Some(Zone::Ligne(1)));
        assert_eq!(pas.autre_zone, Some(Zone::Ligne(5)));
        assert_eq!(pas.chiffres, bit(3));
        let mut coins = pas.cases.clone();
        coins.sort();
        assert_eq!(coins, vec![11, 15, 47, 51]);
        // Le 3 disparaît des colonnes 2 et 6 dans les 7 autres lignes
        assert_eq!(pas.eliminations.len(), 14);
        assert!(pas.eliminations.iter().all(|&(case, retires)| {
            retires == bit(3) && (case % 9 == 2 || case % 9 == 6) && case / 9 != 1 && case / 9 != 5
        }));
    }

    #[test]
    fn test_x_wing_sur_les_colonnes() {
        let g = geometrie(Taille::Neuf);
        let mut logique = vide_9x9(&g);
        // Le 8 n'est possible, dans les colonnes 0 et 4, que dans les lignes 3 et 7
        for col in [0, 4] {
            for ligne in (0..9).filter(|&ligne| ligne != 3 && ligne != 7) {
                logique.candidats[ligne * 9 + col] &= !bit(8);
            }
        }
        let pas = logique.prochain_pas(Technique::XWing).unwrap();
        assert_eq!(pas.zone, Some(Zone::Colonne(0)));
        assert_eq!(pas.autre_zone, Some(Zone::Colonne(4)));
        // Le 8 disparaît des lignes 3 et 7, hors des colonnes 0 et 4
        assert_eq!(pas.eliminations.len(), 14);
        assert!(pas.eliminations.iter().all(|&(case, _)| {
            (case / 9 == 3 || case / 9 == 7) && case % 9 != 0 && case % 9 != 4
        }));
    }

    #[test]
    fn test_une_technique_n_est_employee_que_si_elle_est_permise() {
        let g = geometrie(Taille::Neuf);
        let mut logique = vide_9x9(&g);
        logique.candidats[0] = bit(1) | bit(2);
        logique.candidats[1] = bit(1) | bit(2);
        assert!(logique.prochain_pas(Technique::Intersection).is_none());
        assert!(logique.prochain_pas(Technique::PaireNue).is_some());
        // Une technique permise peut en cacher une plus simple, jamais l'inverse
        logique.candidats[40] = bit(9);
        assert_eq!(
            logique.prochain_pas(Technique::XWing).unwrap().technique,
            Technique::CaseUnique
        );
    }

    #[test]
    fn test_la_grille_de_journal_se_resout_avec_des_cases_et_des_chiffres_uniques() {
        let g = geometrie(Taille::Neuf);
        let mut logique = Logique::new(&g, &journal()).unwrap();
        let plus_difficile = logique.resoudre(Technique::ChiffreUnique);
        assert!(plus_difficile.is_some());
        assert!(plus_difficile.unwrap() <= Technique::ChiffreUnique);
        assert!(logique.est_resolue());
        assert_eq!(&logique.cases()[..9], &[5, 3, 4, 6, 7, 8, 9, 1, 2]);
    }

    #[test]
    fn test_une_grille_vide_demande_de_deviner() {
        let g = geometrie(Taille::Neuf);
        let mut logique = Logique::new(&g, &[0u8; 81]).unwrap();
        assert_eq!(logique.resoudre(Technique::XWing), None);
        assert!(!logique.est_resolue());
    }

    #[test]
    fn test_une_grille_pleine_est_deja_resolue() {
        let g = geometrie(Taille::Quatre);
        let mut complete = Grille::generer(Taille::Quatre, String::from("T"), 0);
        assert!(complete.est_terminee());
        let mut logique = Logique::new(&g, complete.cases()).unwrap();
        assert_eq!(logique.resoudre(Technique::CaseUnique), Some(Technique::CaseUnique));
        // Et la grille d'origine est restée intacte
        assert!(complete.est_terminee());
        let _ = complete.indice();
    }

    #[test]
    fn test_new_refuse_les_grilles_incoherentes() {
        let g = geometrie(Taille::Neuf);
        let mut cases = vec![0u8; 81];
        cases[0] = 5;
        cases[1] = 5;
        assert!(Logique::new(&g, &cases).is_none());
        let mut cases = vec![0u8; 81];
        cases[0] = 10;
        assert!(Logique::new(&g, &cases).is_none());
        assert!(Logique::new(&g, &[0u8; 80]).is_none());
    }

    // La propriété qui compte le plus : une grille résolue par déduction l'est correctement, et sa
    // solution est unique. On le vérifie sur des grilles tirées au hasard, dans toutes les tailles,
    // à tous les niveaux de dépouillement.
    #[test]
    fn test_les_deductions_ne_se_trompent_jamais() {
        for taille in Taille::TOUTES {
            let g = geometrie(taille);
            let cases = taille.cases();
            // Les grandes grilles, longues à générer par force brute, en ont moins
            let (retraits, essais) = if taille.cote() >= 12 {
                (vec![cases * 2 / 5, cases / 2], 1)
            } else {
                (vec![cases * 2 / 5, cases / 2, cases * 3 / 5, cases * 2 / 3], 3)
            };
            for retirees in retraits {
                for _ in 0..essais {
                    let grille = Grille::generer(taille, String::from("T"), retirees);
                    let solution = grille.solution().unwrap().to_vec();
                    let mut logique = Logique::new(&g, grille.cases()).unwrap();
                    if logique.resoudre(Technique::XWing).is_some() {
                        assert_eq!(logique.cases(), solution.as_slice(), "{:?}", taille);
                    } else {
                        // Bloquée : la grille n'est pas finie, mais rien de faux n'a été posé
                        for (case, &valeur) in logique.cases().iter().enumerate() {
                            assert!(valeur == 0 || valeur == solution[case], "{:?}", taille);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_chaque_pas_ne_retire_que_des_possibilites_fausses() {
        // À chaque pas d'une résolution, le chiffre de la solution reste candidat dans sa case
        for taille in [Taille::Six, Taille::Neuf, Taille::Douze] {
            let g = geometrie(taille);
            let grille = Grille::generer(taille, String::from("T"), taille.cases() * 3 / 5);
            let solution = grille.solution().unwrap().to_vec();
            let mut logique = Logique::new(&g, grille.cases()).unwrap();
            while let Some(pas) = logique.prochain_pas(Technique::XWing) {
                logique.appliquer(&pas);
                for (case, &candidats) in logique.candidats.iter().enumerate() {
                    assert_ne!(
                        candidats & bit(solution[case]),
                        0,
                        "{} a retiré la bonne réponse de la case {}",
                        pas.technique.libelle(),
                        case
                    );
                }
            }
        }
    }

    #[test]
    fn test_une_grille_resolue_par_deduction_a_une_seule_solution() {
        for taille in [Taille::Six, Taille::Neuf] {
            let g = geometrie(taille);
            for _ in 0..4 {
                let grille = Grille::generer(taille, String::from("T"), taille.cases() / 2);
                let mut logique = Logique::new(&g, grille.cases()).unwrap();
                if logique.resoudre(Technique::XWing).is_some() {
                    let mut solveur = Solveur::new(&g, grille.cases()).unwrap();
                    assert_eq!(solveur.compter(2), 1);
                }
            }
        }
    }

    #[test]
    fn test_permettre_plus_de_techniques_ne_fait_jamais_perdre() {
        let g = geometrie(Taille::Neuf);
        for _ in 0..5 {
            let grille = Grille::generer(Taille::Neuf, String::from("T"), 50);
            let mut resultats = Vec::new();
            for permise in Technique::TOUTES {
                let mut logique = Logique::new(&g, grille.cases()).unwrap();
                resultats.push(logique.resoudre(permise));
            }
            // Dès qu'une limite permet de finir, toutes les suivantes le permettent aussi,
            // et la technique la plus difficile employée ne dépasse jamais la limite
            let mut deja_resolue = false;
            for (permise, resultat) in Technique::TOUTES.into_iter().zip(&resultats) {
                if let Some(plus_difficile) = resultat {
                    assert!(*plus_difficile <= permise);
                    deja_resolue = true;
                } else {
                    assert!(!deja_resolue, "{:?} bloque alors qu'une limite plus basse finit", permise);
                }
            }
        }
    }

    #[test]
    fn test_les_explications() {
        let g = geometrie(Taille::Neuf);

        let mut logique = vide_9x9(&g);
        logique.candidats[10] = bit(5);
        let pas = logique.prochain_pas(Technique::XWing).unwrap();
        assert_eq!(
            pas.explication(&g),
            "Case unique : en ligne 2, colonne 2, un seul chiffre reste possible, le 5."
        );

        let mut logique = vide_9x9(&g);
        for case in (0..9).filter(|&case| case != 3) {
            logique.candidats[case] &= !bit(5);
        }
        let texte = logique.prochain_pas(Technique::XWing).unwrap().explication(&g);
        assert!(texte.starts_with("Chiffre unique : dans la ligne 1, le 5"), "{}", texte);
        assert!(texte.contains("ligne 1, colonne 4"), "{}", texte);

        let mut logique = vide_9x9(&g);
        for ligne in 1..3 {
            for col in 0..3 {
                logique.candidats[ligne * 9 + col] &= !bit(7);
            }
        }
        let texte = logique.prochain_pas(Technique::XWing).unwrap().explication(&g);
        assert_eq!(
            texte,
            "Intersection : dans le bloc 1, le 7 ne peut aller que dans la ligne 1 ; on peut donc le retirer des autres cases de la ligne 1."
        );

        let mut logique = vide_9x9(&g);
        logique.candidats[0] = bit(1) | bit(2);
        logique.candidats[1] = bit(1) | bit(2);
        let texte = logique.prochain_pas(Technique::XWing).unwrap().explication(&g);
        assert!(texte.starts_with("Paire nue : dans la ligne 1"), "{}", texte);
        assert!(texte.contains("chiffres 1 et 2"), "{}", texte);

        let mut logique = vide_9x9(&g);
        for ligne in [1, 5] {
            for col in (0..9).filter(|&col| col != 2 && col != 6) {
                logique.candidats[ligne * 9 + col] &= !bit(3);
            }
        }
        let texte = logique.prochain_pas(Technique::XWing).unwrap().explication(&g);
        assert!(texte.starts_with("X-Wing : pour le 3, dans les deux lignes"), "{}", texte);
        assert!(texte.contains("colonnes 3 et 7"), "{}", texte);
    }

    #[test]
    fn test_chiffres_texte() {
        assert_eq!(chiffres_texte(0), "");
        assert_eq!(chiffres_texte(bit(4)), "4");
        assert_eq!(chiffres_texte(bit(2) | bit(5)), "2 et 5");
        assert_eq!(chiffres_texte(bit(2) | bit(5) | bit(16)), "2, 5 et 16");
    }
}
