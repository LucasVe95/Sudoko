//! Le solveur de Sudoku : retour sur trace avec les chiffres déjà utilisés stockés en bits.
//!
//! À chaque étape, on traite la case qui a le moins de chiffres possibles. Une case sans aucun
//! chiffre possible révèle une impasse tout de suite, et une case à un seul chiffre est forcée.
//! C'est ce qui rend les grandes grilles (12×12, 16×16) abordables.
//!
//! Le solveur ne connaît pas les lignes, les colonnes ni les blocs : il travaille sur les zones
//! et les interdits décrits par `Regles`, ce qui lui permet de résoudre toutes les variantes.

use crate::regles::Regles;
use std::sync::Arc;

// Le bit qui représente un chiffre : le bit 0 pour le 1, le bit 1 pour le 2, etc.
fn bit(valeur: u8) -> u32 {
    1 << (valeur - 1)
}

pub struct Solveur {
    regles: Arc<Regles>,
    cote: usize,
    cases: Vec<u8>,
    // Les chiffres déjà présents dans chaque zone (un bit par chiffre)
    zones: Vec<u32>,
    // Tous les chiffres possibles : les `cote` premiers bits à 1
    tous: u32,
    // Le nombre d'étapes de la recherche déjà faites, et le nombre à ne pas dépasser.
    // Sur une grande grille, un mauvais choix peut enliser la recherche très longtemps :
    // le budget permet d'abandonner à temps.
    etapes: u64,
    budget: u64,
    abandon: bool,
}

impl Solveur {
    // Prépare le solveur sur une grille donnée ligne par ligne (0 = case vide).
    // Renvoie None si la grille contient déjà un doublon ou un chiffre hors limites.
    pub fn new(regles: &Arc<Regles>, cases: &[u8]) -> Option<Solveur> {
        let cote = regles.cote();
        debug_assert!(cases.len() == cote * cote);
        let mut solveur = Solveur {
            regles: Arc::clone(regles),
            cote,
            cases: vec![0; cote * cote],
            zones: vec![0; regles.zones.len()],
            tous: (1u32 << cote) - 1,
            etapes: 0,
            budget: u64::MAX,
            abandon: false,
        };
        for (case, &valeur) in cases.iter().enumerate() {
            if valeur == 0 {
                continue;
            }
            if valeur as usize > cote || solveur.candidats(case) & bit(valeur) == 0 {
                return None;
            }
            solveur.poser(case, valeur);
        }
        Some(solveur)
    }

    // Limite le nombre d'étapes de la recherche. Passé ce nombre, le solveur abandonne :
    // `a_abandonne` le signale, et le résultat n'est alors pas fiable.
    pub fn avec_budget(mut self, budget: u64) -> Solveur {
        self.budget = budget;
        self
    }

    // true si la recherche a dépassé son budget et s'est arrêtée avant d'avoir fini
    pub fn a_abandonne(&self) -> bool {
        self.abandon
    }

    // Compte une étape de plus. Renvoie false s'il faut s'arrêter parce que le budget est épuisé.
    fn continuer(&mut self) -> bool {
        if self.abandon {
            return false;
        }
        self.etapes += 1;
        if self.etapes > self.budget {
            self.abandon = true;
            return false;
        }
        true
    }

    // L'état actuel de la grille, ligne par ligne
    pub fn cases(&self) -> &[u8] {
        &self.cases
    }

    // Les chiffres qu'on peut encore mettre dans la case, un bit par chiffre : tous, sauf ceux
    // des zones de la case et ceux des cases qui lui sont interdites sans être dans une zone
    // commune (les sauts de cavalier)
    fn candidats(&self, case: usize) -> u32 {
        let mut utilises = 0;
        for &zone in &self.regles.zones_de[case] {
            utilises |= self.zones[zone];
        }
        for &voisine in &self.regles.supplementaires[case] {
            if self.cases[voisine] != 0 {
                utilises |= bit(self.cases[voisine]);
            }
        }
        self.tous & !utilises
    }

    fn poser(&mut self, case: usize, valeur: u8) {
        self.cases[case] = valeur;
        for &zone in &self.regles.zones_de[case] {
            self.zones[zone] |= bit(valeur);
        }
    }

    fn retirer(&mut self, case: usize, valeur: u8) {
        self.cases[case] = 0;
        for &zone in &self.regles.zones_de[case] {
            self.zones[zone] &= !bit(valeur);
        }
    }

    // La case vide qui a le moins de candidats, avec ses candidats.
    // None s'il n'y a plus aucune case vide. Un masque à 0 signale une impasse.
    fn case_la_plus_contrainte(&self) -> Option<(usize, u32)> {
        let mut meilleure = None;
        let mut moins = u32::MAX;
        for case in 0..self.cases.len() {
            if self.cases[case] != 0 {
                continue;
            }
            let masque = self.candidats(case);
            let nombre = masque.count_ones();
            if nombre < moins {
                moins = nombre;
                meilleure = Some((case, masque));
                // Impasse ou case forcée : inutile de chercher mieux
                if nombre <= 1 {
                    return meilleure;
                }
            }
        }
        meilleure
    }

    // Les chiffres d'un masque, dans l'ordre croissant
    fn chiffres_du_masque(&self, masque: u32, chiffres: &mut [u8; 16]) -> usize {
        let mut nombre = 0;
        for valeur in 1..=self.cote as u8 {
            if masque & bit(valeur) != 0 {
                chiffres[nombre] = valeur;
                nombre += 1;
            }
        }
        nombre
    }

    // Remplit la grille par retour sur trace. `melanger` peut réordonner les chiffres essayés
    // pour une case : avec un mélange aléatoire, on obtient une grille complète au hasard.
    // Renvoie false s'il n'y a pas de solution, ou si le budget est épuisé : dans les deux cas,
    // la grille est revenue à son état d'origine.
    pub fn resoudre(&mut self, melanger: &mut impl FnMut(&mut [u8])) -> bool {
        if !self.continuer() {
            return false;
        }
        let Some((case, masque)) = self.case_la_plus_contrainte() else {
            // Plus aucune case vide : la grille est résolue
            return true;
        };
        let mut chiffres = [0u8; 16];
        let nombre = self.chiffres_du_masque(masque, &mut chiffres);
        melanger(&mut chiffres[..nombre]);

        for &valeur in &chiffres[..nombre] {
            self.poser(case, valeur);
            // On tente de résoudre le reste avec ce choix
            if self.resoudre(melanger) {
                return true;
            }
            // Impasse : on annule le choix et on essaie le chiffre suivant
            self.retirer(case, valeur);
        }
        // Aucun chiffre ne convient ici : un choix précédent était mauvais
        false
    }

    // Compte les solutions, en s'arrêtant dès qu'on atteint `limite`.
    // La grille est remise dans son état d'origine à la fin.
    // Si le budget est épuisé (voir `a_abandonne`), le total renvoyé est incomplet.
    pub fn compter(&mut self, limite: usize) -> usize {
        if !self.continuer() {
            return 0;
        }
        let Some((case, masque)) = self.case_la_plus_contrainte() else {
            // Plus aucune case vide : on vient de trouver exactement une solution
            return 1;
        };
        let mut total = 0;
        for valeur in 1..=self.cote as u8 {
            if masque & bit(valeur) == 0 {
                continue;
            }
            self.poser(case, valeur);
            total += self.compter(limite - total);
            self.retirer(case, valeur);
            // Assez de solutions trouvées : inutile de chercher plus loin
            if total >= limite {
                break;
            }
        }
        total
    }
}

// Ce que le solveur conclut sur une grille donnée
#[derive(Debug, PartialEq)]
pub enum Verdict {
    // Aucune solution : les chiffres déjà placés mènent à une impasse
    Aucune,
    // Une seule solution, prouvée
    Unique(Vec<u8>),
    // Au moins deux solutions : on donne la première trouvée
    Plusieurs(Vec<u8>),
    // Une solution trouvée, mais la recherche d'une seconde a été trop longue pour conclure
    NonVerifiee(Vec<u8>),
    // La recherche a été trop longue : on ne sait pas
    TropLong,
}

// Le nombre d'étapes qu'on accorde à la résolution d'une grille saisie (à peu près une seconde
// au pire). Une grille de journal en demande quelques centaines.
const BUDGET_ANALYSE: u64 = 1_000_000;

// Résout une grille saisie par l'utilisateur et dit si sa solution est unique.
// Renvoie None si la grille contient déjà un doublon ou un chiffre hors limites.
pub fn analyser(regles: &Arc<Regles>, cases: &[u8]) -> Option<Verdict> {
    analyser_avec_budget(regles, cases, BUDGET_ANALYSE)
}

fn analyser_avec_budget(regles: &Arc<Regles>, cases: &[u8], budget: u64) -> Option<Verdict> {
    let mut solveur = Solveur::new(regles, cases)?.avec_budget(budget);
    // Les chiffres sont essayés dans l'ordre croissant : on ne les mélange pas
    if !solveur.resoudre(&mut |_| {}) {
        return Some(if solveur.a_abandonne() { Verdict::TropLong } else { Verdict::Aucune });
    }
    let solution = solveur.cases().to_vec();

    // Y a-t-il une seconde solution ?
    let mut solveur = Solveur::new(regles, cases)?.avec_budget(budget);
    let nombre = solveur.compter(2);
    Some(if solveur.a_abandonne() && nombre < 2 {
        Verdict::NonVerifiee(solution)
    } else if nombre >= 2 {
        Verdict::Plusieurs(solution)
    } else {
        Verdict::Unique(solution)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grille::Taille;
    use crate::regles::Variante;

    // Les règles classiques d'une grille de ce côté (4, 6, 9, 12 ou 16)
    fn classique(cote: usize) -> Arc<Regles> {
        Regles::classique(Taille::depuis_cote(cote).unwrap())
    }

    fn ne_rien_melanger(_: &mut [u8]) {}

    // Chaque zone doit contenir tous les chiffres exactement une fois, et deux cases interdites
    // l'une à l'autre ne doivent pas avoir le même chiffre
    fn est_complete_et_valide(regles: &Regles, cases: &[u8]) -> bool {
        let cote = regles.cote();
        let attendu = (1u32 << cote) - 1;
        if cases.iter().any(|&valeur| valeur == 0 || valeur as usize > cote) {
            return false;
        }
        let zones_ok = regles
            .zones
            .iter()
            .all(|zone| zone.iter().fold(0u32, |masque, &case| masque | bit(cases[case])) == attendu);
        let voisines_ok = (0..cases.len())
            .all(|case| regles.voisines[case].iter().all(|&autre| cases[autre] != cases[case]));
        zones_ok && voisines_ok
    }

    #[test]
    fn test_resoudre_une_grille_vide_de_chaque_taille() {
        for taille in Taille::TOUTES {
            let regles = Regles::classique(taille);
            let vide = vec![0u8; taille.cases()];
            let mut solveur = Solveur::new(&regles, &vide).unwrap();
            assert!(solveur.resoudre(&mut ne_rien_melanger), "{}", taille.libelle());
            assert!(est_complete_et_valide(&regles, solveur.cases()), "{}", taille.libelle());
        }
    }

    #[test]
    fn test_la_resolution_sans_melange_donne_la_premiere_ligne_croissante() {
        let vide = vec![0u8; 16];
        let mut solveur = Solveur::new(&classique(4), &vide).unwrap();
        assert!(solveur.resoudre(&mut ne_rien_melanger));
        assert_eq!(&solveur.cases()[..4], [1, 2, 3, 4]);
    }

    #[test]
    fn test_doublon_refuse_a_la_creation() {
        // Deux 1 sur la première ligne
        let mut cases = vec![0u8; 16];
        cases[0] = 1;
        cases[1] = 1;
        assert!(Solveur::new(&classique(4), &cases).is_none());
    }

    #[test]
    fn test_chiffre_hors_limites_refuse_a_la_creation() {
        let mut cases = vec![0u8; 16];
        cases[0] = 5;
        assert!(Solveur::new(&classique(4), &cases).is_none());
    }

    #[test]
    fn test_grille_impossible() {
        // 4×4 : la case (0, 3) ne peut contenir ni 1, 2, 3 (ligne) ni 4 (colonne)
        let mut cases = vec![0u8; 16];
        cases[0] = 1;
        cases[1] = 2;
        cases[2] = 3;
        cases[4 + 3] = 4;
        let mut solveur = Solveur::new(&classique(4), &cases).unwrap();
        assert!(!solveur.resoudre(&mut ne_rien_melanger));
        assert_eq!(solveur.compter(5), 0);
    }

    #[test]
    fn test_compter_s_arrete_a_la_limite_et_restaure_la_grille() {
        let vide = vec![0u8; 16];
        let mut solveur = Solveur::new(&classique(4), &vide).unwrap();
        assert_eq!(solveur.compter(3), 3);
        assert!(solveur.cases().iter().all(|&case| case == 0));
    }

    #[test]
    fn test_un_budget_trop_petit_fait_abandonner_la_resolution() {
        let vide = vec![0u8; 81];
        let mut solveur = Solveur::new(&classique(9), &vide).unwrap().avec_budget(5);
        assert!(!solveur.resoudre(&mut ne_rien_melanger));
        assert!(solveur.a_abandonne());
        // La grille est revenue à son état d'origine
        assert!(solveur.cases().iter().all(|&case| case == 0));
    }

    #[test]
    fn test_un_budget_suffisant_ne_change_rien() {
        let regles = classique(9);
        let vide = vec![0u8; 81];
        let mut solveur = Solveur::new(&regles, &vide).unwrap().avec_budget(10_000);
        assert!(solveur.resoudre(&mut ne_rien_melanger));
        assert!(!solveur.a_abandonne());
        assert!(est_complete_et_valide(&regles, solveur.cases()));
    }

    #[test]
    fn test_un_budget_trop_petit_fait_abandonner_le_comptage() {
        let vide = vec![0u8; 81];
        let mut solveur = Solveur::new(&classique(9), &vide).unwrap().avec_budget(10);
        solveur.compter(1_000_000);
        assert!(solveur.a_abandonne());
        assert!(solveur.cases().iter().all(|&case| case == 0));
    }

    #[test]
    fn test_sans_budget_on_n_abandonne_jamais() {
        let vide = vec![0u8; 16];
        let mut solveur = Solveur::new(&classique(4), &vide).unwrap();
        assert_eq!(solveur.compter(1000), 288);
        assert!(!solveur.a_abandonne());
    }

    // Les cases de la grille d'entraînement (une grille de journal à solution unique)
    fn grille_de_journal() -> Vec<u8> {
        crate::grille::DEPART
            .chars()
            .filter_map(|c| c.to_digit(10))
            .map(|chiffre| chiffre as u8)
            .collect()
    }

    #[test]
    fn test_analyser_une_grille_a_solution_unique() {
        let regles = classique(9);
        let Some(Verdict::Unique(solution)) = analyser(&regles, &grille_de_journal()) else {
            panic!("la grille d'entraînement a une solution unique");
        };
        assert!(est_complete_et_valide(&regles, &solution));
        assert_eq!(&solution[..9], &[5, 3, 4, 6, 7, 8, 9, 1, 2]);
        // Les chiffres de départ sont conservés
        for (depart, trouve) in grille_de_journal().iter().zip(&solution) {
            assert!(*depart == 0 || depart == trouve);
        }
    }

    #[test]
    fn test_analyser_une_grille_a_plusieurs_solutions() {
        let regles = classique(9);
        let Some(Verdict::Plusieurs(solution)) = analyser(&regles, &vec![0u8; 81]) else {
            panic!("une grille vide a plusieurs solutions");
        };
        assert!(est_complete_et_valide(&regles, &solution));

        // Retirer des chiffres à la grille de journal peut rendre la solution ambiguë
        let mut cases = grille_de_journal();
        for case in cases.iter_mut().take(40) {
            *case = 0;
        }
        assert!(matches!(analyser(&regles, &cases), Some(Verdict::Plusieurs(_))));
    }

    #[test]
    fn test_analyser_une_grille_sans_solution() {
        // La première ligne contient 1 à 8 : sa dernière case devrait être 9,
        // mais sa colonne a déjà un 9
        let mut cases = vec![0u8; 81];
        for (col, valeur) in (1..=8).enumerate() {
            cases[col] = valeur;
        }
        cases[9 + 8] = 9;
        assert_eq!(analyser(&classique(9), &cases), Some(Verdict::Aucune));
    }

    #[test]
    fn test_analyser_refuse_un_doublon_ou_un_chiffre_hors_limites() {
        let mut cases = vec![0u8; 81];
        cases[0] = 5;
        cases[1] = 5;
        assert_eq!(analyser(&classique(9), &cases), None);
        let mut cases = vec![0u8; 16];
        cases[0] = 5;
        assert_eq!(analyser(&classique(4), &cases), None);
    }

    #[test]
    fn test_analyser_trop_long() {
        let vide = vec![0u8; 81];
        assert_eq!(analyser_avec_budget(&classique(9), &vide, 1), Some(Verdict::TropLong));
    }

    #[test]
    fn test_analyser_solution_trouvee_mais_unicite_non_verifiee() {
        // Pour un budget qui suffit juste à trouver une solution, la recherche d'une seconde
        // n'a plus assez d'étapes
        let regles = classique(9);
        let vide = vec![0u8; 81];
        let non_verifiee = (80..5000).find_map(|budget| match analyser_avec_budget(&regles, &vide, budget) {
            Some(Verdict::NonVerifiee(solution)) => Some(solution),
            _ => None,
        });
        let solution = non_verifiee.expect("un budget donne une solution sans preuve d'unicité");
        assert!(est_complete_et_valide(&regles, &solution));
    }

    #[test]
    fn test_analyser_une_grille_4x4() {
        let mut cases = vec![0u8; 16];
        cases[..4].copy_from_slice(&[1, 2, 3, 4]);
        // Une première ligne donnée laisse plusieurs grilles possibles
        assert!(matches!(analyser(&classique(4), &cases), Some(Verdict::Plusieurs(_))));
    }

    #[test]
    fn test_compter_toutes_les_grilles_4x4() {
        // Il y a exactement 288 grilles de Sudoku 4×4
        let vide = vec![0u8; 16];
        let mut solveur = Solveur::new(&classique(4), &vide).unwrap();
        assert_eq!(solveur.compter(1000), 288);
    }

    // Compte les grilles complètes par une énumération naïve, qui ne s'appuie que sur la liste des
    // voisines : elle sert de référence, indépendante des zones et des masques du solveur
    fn compter_naivement(regles: &Regles, cases: &mut Vec<u8>, position: usize) -> usize {
        if position == cases.len() {
            return 1;
        }
        if cases[position] != 0 {
            return compter_naivement(regles, cases, position + 1);
        }
        let mut total = 0;
        for valeur in 1..=regles.cote() as u8 {
            if regles.voisines[position].iter().all(|&autre| cases[autre] != valeur) {
                cases[position] = valeur;
                total += compter_naivement(regles, cases, position + 1);
                cases[position] = 0;
            }
        }
        total
    }

    #[test]
    fn test_le_solveur_compte_comme_l_enumeration_naive_dans_chaque_variante() {
        for variante in [Variante::Classique, Variante::Diagonale, Variante::AntiCavalier] {
            let regles = Regles::pour(Taille::Quatre, variante);
            let attendu = compter_naivement(&regles, &mut vec![0u8; 16], 0);
            let mut solveur = Solveur::new(&regles, &vec![0u8; 16]).unwrap();
            assert_eq!(solveur.compter(100_000), attendu, "{:?}", variante);
        }
    }

    #[test]
    fn test_le_solveur_compte_comme_l_enumeration_naive_avec_des_chiffres_donnes() {
        // 6×6 : on part d'une grille complète dont on vide quelques cases, pour que l'énumération
        // reste courte
        for variante in [Variante::Classique, Variante::Diagonale, Variante::AntiCavalier] {
            let regles = Regles::pour(Taille::Six, variante);
            let mut complet = Solveur::new(&regles, &vec![0u8; 36]).unwrap();
            assert!(complet.resoudre(&mut ne_rien_melanger), "{:?}", variante);
            let mut depart = complet.cases().to_vec();
            for case in [0, 1, 2, 7, 8, 14, 15, 20, 21, 27, 28, 35] {
                depart[case] = 0;
            }
            let mut solveur = Solveur::new(&regles, &depart).unwrap();
            let attendu = compter_naivement(&regles, &mut depart.clone(), 0);
            assert!(attendu >= 1);
            assert_eq!(solveur.compter(1_000_000), attendu, "{:?}", variante);
        }
    }

    #[test]
    fn test_une_grille_vide_se_resout_dans_chaque_variante_et_taille_acceptee() {
        for variante in Variante::TOUTES {
            for taille in Taille::TOUTES {
                if !variante.accepte(taille) {
                    continue;
                }
                let regles = Regles::pour(taille, variante);
                let mut solveur = Solveur::new(&regles, &vec![0u8; taille.cases()])
                    .unwrap()
                    .avec_budget(2_000_000);
                assert!(
                    solveur.resoudre(&mut ne_rien_melanger),
                    "{:?} {} : aucune grille complète trouvée",
                    variante,
                    taille.libelle()
                );
                assert!(
                    est_complete_et_valide(&regles, solveur.cases()),
                    "{:?} {}",
                    variante,
                    taille.libelle()
                );
            }
        }
    }

    #[test]
    fn test_la_diagonale_refuse_un_doublon_sur_une_diagonale() {
        let regles = Regles::pour(Taille::Neuf, Variante::Diagonale);
        let mut cases = vec![0u8; 81];
        cases[0] = 5;
        cases[4 * 9 + 4] = 5;
        assert!(Solveur::new(&regles, &cases).is_none());
        // Le même doublon est permis en classique
        assert!(Solveur::new(&classique(9), &cases).is_some());
    }

    #[test]
    fn test_l_anti_cavalier_refuse_un_saut_de_cavalier() {
        let regles = Regles::pour(Taille::Neuf, Variante::AntiCavalier);
        let mut cases = vec![0u8; 81];
        cases[4 * 9 + 4] = 7;
        cases[2 * 9 + 5] = 7; // un saut de cavalier du centre
        assert!(Solveur::new(&regles, &cases).is_none());
        assert!(Solveur::new(&classique(9), &cases).is_some());
    }

    #[test]
    fn test_hyper_refuse_un_doublon_dans_une_fenetre() {
        let regles = Regles::pour(Taille::Neuf, Variante::Hyper);
        let mut cases = vec![0u8; 81];
        // (1, 1) et (3, 3) : même fenêtre, pas de ligne, colonne ni bloc commun
        cases[9 + 1] = 4;
        cases[3 * 9 + 3] = 4;
        assert!(Solveur::new(&regles, &cases).is_none());
        assert!(Solveur::new(&classique(9), &cases).is_some());
    }
}
