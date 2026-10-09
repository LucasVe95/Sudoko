//! Le solveur de Sudoku : retour sur trace avec les chiffres déjà utilisés stockés en bits.
//!
//! À chaque étape, on traite la case qui a le moins de chiffres possibles. Une case sans aucun
//! chiffre possible révèle une impasse tout de suite, et une case à un seul chiffre est forcée.
//! C'est ce qui rend les grandes grilles (12×12, 16×16) abordables.

// Le bit qui représente un chiffre : le bit 0 pour le 1, le bit 1 pour le 2, etc.
fn bit(valeur: u8) -> u32 {
    1 << (valeur - 1)
}

pub struct Solveur {
    cote: usize,
    bloc_h: usize,
    bloc_l: usize,
    cases: Vec<u8>,
    // Les chiffres déjà présents dans chaque ligne, colonne et bloc (un bit par chiffre)
    lignes: Vec<u32>,
    colonnes: Vec<u32>,
    blocs: Vec<u32>,
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
    pub fn new(cote: usize, bloc_h: usize, bloc_l: usize, cases: &[u8]) -> Option<Solveur> {
        debug_assert!(cote <= 16 && bloc_h * bloc_l == cote && cases.len() == cote * cote);
        let mut solveur = Solveur {
            cote,
            bloc_h,
            bloc_l,
            cases: vec![0; cote * cote],
            lignes: vec![0; cote],
            colonnes: vec![0; cote],
            blocs: vec![0; cote],
            tous: (1u32 << cote) - 1,
            etapes: 0,
            budget: u64::MAX,
            abandon: false,
        };
        for (indice, &valeur) in cases.iter().enumerate() {
            if valeur == 0 {
                continue;
            }
            let (ligne, col) = (indice / cote, indice % cote);
            if valeur as usize > cote || solveur.candidats(ligne, col) & bit(valeur) == 0 {
                return None;
            }
            solveur.poser(ligne, col, valeur);
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

    // Le numéro du bloc qui contient la case
    fn bloc_de(&self, ligne: usize, col: usize) -> usize {
        (ligne / self.bloc_h) * (self.cote / self.bloc_l) + col / self.bloc_l
    }

    // Les chiffres qu'on peut encore mettre dans la case, un bit par chiffre
    fn candidats(&self, ligne: usize, col: usize) -> u32 {
        let utilises = self.lignes[ligne] | self.colonnes[col] | self.blocs[self.bloc_de(ligne, col)];
        self.tous & !utilises
    }

    fn poser(&mut self, ligne: usize, col: usize, valeur: u8) {
        let bloc = self.bloc_de(ligne, col);
        self.cases[ligne * self.cote + col] = valeur;
        self.lignes[ligne] |= bit(valeur);
        self.colonnes[col] |= bit(valeur);
        self.blocs[bloc] |= bit(valeur);
    }

    fn retirer(&mut self, ligne: usize, col: usize, valeur: u8) {
        let bloc = self.bloc_de(ligne, col);
        self.cases[ligne * self.cote + col] = 0;
        self.lignes[ligne] &= !bit(valeur);
        self.colonnes[col] &= !bit(valeur);
        self.blocs[bloc] &= !bit(valeur);
    }

    // La case vide qui a le moins de candidats, avec ses candidats.
    // None s'il n'y a plus aucune case vide. Un masque à 0 signale une impasse.
    fn case_la_plus_contrainte(&self) -> Option<(usize, usize, u32)> {
        let mut meilleure = None;
        let mut moins = u32::MAX;
        for ligne in 0..self.cote {
            for col in 0..self.cote {
                if self.cases[ligne * self.cote + col] != 0 {
                    continue;
                }
                let masque = self.candidats(ligne, col);
                let nombre = masque.count_ones();
                if nombre < moins {
                    moins = nombre;
                    meilleure = Some((ligne, col, masque));
                    // Impasse ou case forcée : inutile de chercher mieux
                    if nombre <= 1 {
                        return meilleure;
                    }
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
        let Some((ligne, col, masque)) = self.case_la_plus_contrainte() else {
            // Plus aucune case vide : la grille est résolue
            return true;
        };
        let mut chiffres = [0u8; 16];
        let nombre = self.chiffres_du_masque(masque, &mut chiffres);
        melanger(&mut chiffres[..nombre]);

        for &valeur in &chiffres[..nombre] {
            self.poser(ligne, col, valeur);
            // On tente de résoudre le reste avec ce choix
            if self.resoudre(melanger) {
                return true;
            }
            // Impasse : on annule le choix et on essaie le chiffre suivant
            self.retirer(ligne, col, valeur);
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
        let Some((ligne, col, masque)) = self.case_la_plus_contrainte() else {
            // Plus aucune case vide : on vient de trouver exactement une solution
            return 1;
        };
        let mut total = 0;
        for valeur in 1..=self.cote as u8 {
            if masque & bit(valeur) == 0 {
                continue;
            }
            self.poser(ligne, col, valeur);
            total += self.compter(limite - total);
            self.retirer(ligne, col, valeur);
            // Assez de solutions trouvées : inutile de chercher plus loin
            if total >= limite {
                break;
            }
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // (côté, hauteur de bloc, largeur de bloc) de toutes les tailles gérées
    const TAILLES: [(usize, usize, usize); 5] =
        [(4, 2, 2), (6, 2, 3), (9, 3, 3), (12, 3, 4), (16, 4, 4)];

    fn ne_rien_melanger(_: &mut [u8]) {}

    // Chaque ligne, colonne et bloc doit contenir tous les chiffres exactement une fois
    fn est_complete_et_valide(cote: usize, bloc_h: usize, bloc_l: usize, cases: &[u8]) -> bool {
        let attendu = (1u32 << cote) - 1;
        let mut lignes = vec![0u32; cote];
        let mut colonnes = vec![0u32; cote];
        let mut blocs = vec![0u32; cote];
        for ligne in 0..cote {
            for col in 0..cote {
                let valeur = cases[ligne * cote + col];
                if valeur == 0 || valeur as usize > cote {
                    return false;
                }
                lignes[ligne] |= bit(valeur);
                colonnes[col] |= bit(valeur);
                blocs[(ligne / bloc_h) * (cote / bloc_l) + col / bloc_l] |= bit(valeur);
            }
        }
        lignes
            .iter()
            .chain(&colonnes)
            .chain(&blocs)
            .all(|&masque| masque == attendu)
    }

    #[test]
    fn test_resoudre_une_grille_vide_de_chaque_taille() {
        for (cote, bloc_h, bloc_l) in TAILLES {
            let vide = vec![0u8; cote * cote];
            let mut solveur = Solveur::new(cote, bloc_h, bloc_l, &vide).unwrap();
            assert!(solveur.resoudre(&mut ne_rien_melanger), "{}×{}", cote, cote);
            assert!(
                est_complete_et_valide(cote, bloc_h, bloc_l, solveur.cases()),
                "{}×{}",
                cote,
                cote
            );
        }
    }

    #[test]
    fn test_la_resolution_sans_melange_donne_la_premiere_ligne_croissante() {
        let vide = vec![0u8; 16];
        let mut solveur = Solveur::new(4, 2, 2, &vide).unwrap();
        assert!(solveur.resoudre(&mut ne_rien_melanger));
        assert_eq!(&solveur.cases()[..4], [1, 2, 3, 4]);
    }

    #[test]
    fn test_doublon_refuse_a_la_creation() {
        // Deux 1 sur la première ligne
        let mut cases = vec![0u8; 16];
        cases[0] = 1;
        cases[1] = 1;
        assert!(Solveur::new(4, 2, 2, &cases).is_none());
    }

    #[test]
    fn test_chiffre_hors_limites_refuse_a_la_creation() {
        let mut cases = vec![0u8; 16];
        cases[0] = 5;
        assert!(Solveur::new(4, 2, 2, &cases).is_none());
    }

    #[test]
    fn test_grille_impossible() {
        // 4×4 : la case (0, 3) ne peut contenir ni 1, 2, 3 (ligne) ni 4 (colonne)
        let mut cases = vec![0u8; 16];
        cases[0] = 1;
        cases[1] = 2;
        cases[2] = 3;
        cases[4 + 3] = 4;
        let mut solveur = Solveur::new(4, 2, 2, &cases).unwrap();
        assert!(!solveur.resoudre(&mut ne_rien_melanger));
        assert_eq!(solveur.compter(5), 0);
    }

    #[test]
    fn test_compter_s_arrete_a_la_limite_et_restaure_la_grille() {
        let vide = vec![0u8; 16];
        let mut solveur = Solveur::new(4, 2, 2, &vide).unwrap();
        assert_eq!(solveur.compter(3), 3);
        assert!(solveur.cases().iter().all(|&case| case == 0));
    }

    #[test]
    fn test_un_budget_trop_petit_fait_abandonner_la_resolution() {
        let vide = vec![0u8; 81];
        let mut solveur = Solveur::new(9, 3, 3, &vide).unwrap().avec_budget(5);
        assert!(!solveur.resoudre(&mut ne_rien_melanger));
        assert!(solveur.a_abandonne());
        // La grille est revenue à son état d'origine
        assert!(solveur.cases().iter().all(|&case| case == 0));
    }

    #[test]
    fn test_un_budget_suffisant_ne_change_rien() {
        let vide = vec![0u8; 81];
        let mut solveur = Solveur::new(9, 3, 3, &vide).unwrap().avec_budget(10_000);
        assert!(solveur.resoudre(&mut ne_rien_melanger));
        assert!(!solveur.a_abandonne());
        assert!(est_complete_et_valide(9, 3, 3, solveur.cases()));
    }

    #[test]
    fn test_un_budget_trop_petit_fait_abandonner_le_comptage() {
        let vide = vec![0u8; 81];
        let mut solveur = Solveur::new(9, 3, 3, &vide).unwrap().avec_budget(10);
        solveur.compter(1_000_000);
        assert!(solveur.a_abandonne());
        assert!(solveur.cases().iter().all(|&case| case == 0));
    }

    #[test]
    fn test_sans_budget_on_n_abandonne_jamais() {
        let vide = vec![0u8; 16];
        let mut solveur = Solveur::new(4, 2, 2, &vide).unwrap();
        assert_eq!(solveur.compter(1000), 288);
        assert!(!solveur.a_abandonne());
    }

    #[test]
    fn test_compter_toutes_les_grilles_4x4() {
        // Il y a exactement 288 grilles de Sudoku 4×4
        let vide = vec![0u8; 16];
        let mut solveur = Solveur::new(4, 2, 2, &vide).unwrap();
        assert_eq!(solveur.compter(1000), 288);
    }
}
