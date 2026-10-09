//! La grille de Sudoku : règles, chargement, résolution et génération.

use rand::seq::{IndexedRandom, SliceRandom};

// Niveau de difficulté d'une grille générée
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Niveau {
    Facile,
    Moyen,
    Difficile,
}

impl Niveau {
    pub const TOUS: [Niveau; 3] = [Niveau::Facile, Niveau::Moyen, Niveau::Difficile];

    pub fn libelle(self) -> &'static str {
        match self {
            Niveau::Facile => "Facile",
            Niveau::Moyen => "Moyen",
            Niveau::Difficile => "Difficile",
        }
    }

    // Plus on retire de cases, plus la grille est difficile
    fn cases_a_retirer(self) -> usize {
        match self {
            Niveau::Facile => 35,
            Niveau::Moyen => 45,
            Niveau::Difficile => 55,
        }
    }
}

// D'où vient la grille : générée au hasard à un niveau donné, ou la grille fixe d'entraînement
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Origine {
    Niveau(Niveau),
    Fixe,
}

#[derive(Clone)]
pub struct Grille {
    cases: [[u8; 9]; 9],
    // true pour les cases de la grille de départ, que le joueur ne peut pas modifier
    fixes: [[bool; 9]; 9],
    // La solution, quand on la connaît : elle sert à dire si un chiffre du joueur est correct
    solution: Option<[[u8; 9]; 9]>,
    pub nom: String,
}
impl Grille {
    //"Constructeur" de la structure Grille
    fn nouvelle(nom: String) -> Grille {
        Grille {
            cases: [[0; 9]; 9],
            fixes: [[false; 9]; 9],
            solution: None,
            nom,
        }
    }

    // Crée la grille correspondant à l'origine demandée
    pub fn creer(origine: Origine) -> Result<Grille, String> {
        match origine {
            Origine::Fixe => Grille::depuis_texte(String::from("Grille d'entraînement"), DEPART),
            Origine::Niveau(niveau) => Ok(Grille::generer_niveau(niveau)),
        }
    }
    // Construit une grille à partir d'un texte de 81 chiffres (0 = case vide)
    pub fn depuis_texte(nom: String, texte: &str) -> Result<Grille, String> {
        // On garde tous les caractères sauf les espaces et retours à la ligne
        let caracteres: Vec<char> = texte.chars().filter(|c| !c.is_whitespace()).collect();

        if caracteres.len() != 81 {
            return Err(format!(
                "81 chiffres attendus, {} reçus",
                caracteres.len()
            ));
        }

        let mut grille = Grille::nouvelle(nom);
        for (indice, c) in caracteres.iter().enumerate() {
            let valeur = match c.to_digit(10) {
                Some(v) => v as u8,
                None => return Err(format!("Caractère invalide : '{}'", c)),
            };

            // Une case à 0 reste vide, on ne place que les vrais chiffres
            if valeur != 0 {
                let ligne = indice / 9;
                let col = indice % 9;
                if !grille.placer(ligne, col, valeur) {
                    return Err(format!(
                        "{} en ligne {}, colonne {} brise les règles",
                        valeur,
                        ligne + 1,
                        col + 1
                    ));
                }
                // Les chiffres du texte de départ sont verrouillés
                grille.fixes[ligne][col] = true;
            }
        }

        // On garde la solution pour pouvoir dire si un chiffre du joueur est correct.
        // Si la grille a plusieurs solutions, on retient la première trouvée.
        let mut copie = grille.clone();
        if copie.remplir_solution() {
            grille.solution = Some(copie.cases);
        }

        Ok(grille)
    }

    //methode qui lit seulement :&self
    pub fn afficher(&self) {
        for (i, ligne) in self.cases.iter().enumerate() {
            // Séparateur horizontal entre les blocs 3×3
            if i % 3 == 0 && i != 0 {
                println!("------+-------+-------");
            }
            for (j, case) in ligne.iter().enumerate() {
                // Séparateur vertical entre les blocs 3×3
                if j % 3 == 0 && j != 0 {
                    print!("| ");
                }
                // Une case vide (0) s'affiche avec un point
                if *case == 0 {
                    print!(". ");
                } else {
                    print!("{} ", case);
                }
            }
            println!();
        }
    }

    // Vérifie si on peut mettre `valeur` dans la case (ligne, col) sans briser les règles
    fn est_valide(&self, ligne: usize, col: usize, valeur: u8) -> bool {
        // Vérifie la ligne et la colonne en même temps
        for i in 0..9 {
            if self.cases[ligne][i] == valeur || self.cases[i][col] == valeur {
                return false;
            }
        }

        // Coin en haut à gauche du bloc 3×3 qui contient la case
        let debut_ligne = (ligne / 3) * 3;
        let debut_col = (col / 3) * 3;
        for i in 0..3 {
            for j in 0..3 {
                if self.cases[debut_ligne + i][debut_col + j] == valeur {
                    return false;
                }
            }
        }

        true
    }

    //methode qui modifie : &mut self
    // Renvoie true si la valeur a été placée, false si le coup est refusé
    pub fn placer(&mut self, ligne: usize, col: usize, valeur: u8) -> bool {
        if self.fixes[ligne][col] || !self.est_valide(ligne, col, valeur) {
            return false;
        }
        self.cases[ligne][col] = valeur;
        true
    }

    // Vide une case. Renvoie false si c'est une case de départ
    pub fn effacer(&mut self, ligne: usize, col: usize) -> bool {
        if self.fixes[ligne][col] {
            return false;
        }
        self.cases[ligne][col] = 0;
        true
    }

    // true si `valeur` est le chiffre de la solution pour cette case.
    // Si la solution n'est pas connue, on ne peut pas juger : on répond true.
    pub fn est_correct(&self, ligne: usize, col: usize, valeur: u8) -> bool {
        match self.solution {
            Some(solution) => solution[ligne][col] == valeur,
            None => true,
        }
    }

    // Valeur d'une case : 0 si elle est vide
    pub fn valeur(&self, ligne: usize, col: usize) -> u8 {
        self.cases[ligne][col]
    }

    // true si la case fait partie de la grille de départ
    pub fn est_fixe(&self, ligne: usize, col: usize) -> bool {
        self.fixes[ligne][col]
    }

    // Efface tous les coups du joueur puis remplit la grille avec la solution.
    // Renvoie false si la grille de départ n'a pas de solution.
    pub fn remplir_solution(&mut self) -> bool {
        // On repart des seules cases de départ, pour qu'un mauvais coup du joueur ne gêne pas
        for ligne in 0..9 {
            for col in 0..9 {
                self.effacer(ligne, col);
            }
        }
        self.resoudre()
    }

    // La grille est gagnée quand toutes les cases sont remplies.
    // Chaque chiffre ayant été vérifié par est_valide, il n'y a rien d'autre à contrôler.
    pub fn est_terminee(&self) -> bool {
        self.cases.iter().all(|ligne| ligne.iter().all(|&case| case != 0))
    }

    // Remplit une case vide choisie au hasard avec la bonne valeur.
    // Renvoie (ligne, col, valeur), ou None s'il n'y a rien à révéler : grille déjà pleine,
    // ou coups du joueur qui mènent à une impasse (la grille n'est alors pas modifiée).
    pub fn indice(&mut self) -> Option<(usize, usize, u8)> {
        // On résout une copie : la grille du joueur reste intacte si la résolution échoue
        let mut solution = self.clone();
        if !solution.resoudre() {
            return None;
        }

        let vides: Vec<(usize, usize)> = (0..81)
            .map(|i| (i / 9, i % 9))
            .filter(|&(ligne, col)| self.cases[ligne][col] == 0)
            .collect();
        let &(ligne, col) = vides.choose(&mut rand::rng())?;

        let valeur = solution.cases[ligne][col];
        self.cases[ligne][col] = valeur;
        Some((ligne, col, valeur))
    }

    // Remplit la grille par retour sur trace (backtracking).
    // Renvoie true si une solution a été trouvée, false sinon (la grille est alors inchangée).
    pub fn resoudre(&mut self) -> bool {
        // Les chiffres sont essayés dans l'ordre 1 à 9 : on ne les mélange pas
        self.resoudre_avec(&mut |_| {})
    }

    // Même chose que resoudre, mais `melanger` peut réordonner les chiffres 1 à 9 avant
    // chaque case. Avec un mélange aléatoire, on obtient une grille complète au hasard.
    fn resoudre_avec(&mut self, melanger: &mut impl FnMut(&mut [u8; 9])) -> bool {
        for ligne in 0..9 {
            for col in 0..9 {
                // On cherche la première case vide
                if self.cases[ligne][col] == 0 {
                    let mut chiffres = [1, 2, 3, 4, 5, 6, 7, 8, 9];
                    melanger(&mut chiffres);
                    for valeur in chiffres {
                        if self.est_valide(ligne, col, valeur) {
                            self.cases[ligne][col] = valeur;
                            // On tente de résoudre le reste avec ce choix
                            if self.resoudre_avec(melanger) {
                                return true;
                            }
                            // Impasse : on annule le choix et on essaie la valeur suivante
                            self.cases[ligne][col] = 0;
                        }
                    }
                    // Aucune valeur ne convient ici : le choix précédent était mauvais
                    return false;
                }
            }
        }
        // Plus aucune case vide : la grille est résolue
        true
    }

    // Compte les solutions de la grille, en s'arrêtant dès qu'on atteint `limite`.
    // La grille est remise dans son état d'origine à la fin.
    fn compter_solutions(&mut self, limite: usize) -> usize {
        for ligne in 0..9 {
            for col in 0..9 {
                if self.cases[ligne][col] == 0 {
                    let mut total = 0;
                    for valeur in 1..=9 {
                        if self.est_valide(ligne, col, valeur) {
                            self.cases[ligne][col] = valeur;
                            total += self.compter_solutions(limite - total);
                            self.cases[ligne][col] = 0;
                            // Assez de solutions trouvées : inutile de chercher plus loin
                            if total >= limite {
                                return total;
                            }
                        }
                    }
                    return total;
                }
            }
        }
        // Plus aucune case vide : on vient de trouver exactement une solution
        1
    }

    // Génère une grille du niveau demandé
    pub fn generer_niveau(niveau: Niveau) -> Grille {
        let nom = format!("Partie {}", niveau.libelle().to_lowercase());
        Grille::generer(nom, niveau.cases_a_retirer())
    }

    // Génère une grille à solution unique en retirant `a_retirer` cases (au plus) d'une
    // grille complète tirée au hasard. Plus on en retire, plus la grille est difficile.
    pub fn generer(nom: String, a_retirer: usize) -> Grille {
        let mut rng = rand::rng();
        let mut grille = Grille::nouvelle(nom);
        grille.resoudre_avec(&mut |chiffres| chiffres.shuffle(&mut rng));
        // La grille complète est la solution : on la retient avant de retirer des cases
        let solution = grille.cases;

        // Les 81 positions, dans un ordre aléatoire
        let mut positions: Vec<(usize, usize)> = (0..81).map(|i| (i / 9, i % 9)).collect();
        positions.shuffle(&mut rng);

        let mut retirees = 0;
        for (ligne, col) in positions {
            if retirees == a_retirer {
                break;
            }
            let valeur = grille.cases[ligne][col];
            grille.cases[ligne][col] = 0;
            // On ne garde le retrait que si la grille garde une solution unique
            if grille.compter_solutions(2) == 1 {
                retirees += 1;
            } else {
                grille.cases[ligne][col] = valeur;
            }
        }

        // Les chiffres restants forment la grille de départ : on les verrouille
        for ligne in 0..9 {
            for col in 0..9 {
                grille.fixes[ligne][col] = grille.cases[ligne][col] != 0;
            }
        }
        grille.solution = Some(solution);
        grille
    }
}

// Grille de départ : 81 chiffres, 0 = case vide
pub const DEPART: &str = "530070000\
                      600195000\
                      098000060\
                      800060003\
                      400803001\
                      700020006\
                      060000280\
                      000419005\
                      000080079";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placer() {
        let mut g = Grille::nouvelle(String::from("Test"));
        assert!(g.placer(0, 0, 5));
        assert_eq!(g.cases[0][0], 5);
    }

    #[test]
    fn test_refuse_doublon_ligne() {
        let mut g = Grille::nouvelle(String::from("Test"));
        g.placer(0, 0, 5);
        assert!(!g.est_valide(0, 8, 5));
    }

    #[test]
    fn test_refuse_doublon_colonne() {
        let mut g = Grille::nouvelle(String::from("Test"));
        g.placer(0, 0, 5);
        assert!(!g.est_valide(8, 0, 5));
    }

    #[test]
    fn test_refuse_doublon_bloc() {
        let mut g = Grille::nouvelle(String::from("Test"));
        g.placer(0, 0, 5);
        assert!(!g.est_valide(2, 2, 5));
    }

    #[test]
    fn test_accepte_coup_valide() {
        let mut g = Grille::nouvelle(String::from("Test"));
        g.placer(0, 0, 5);
        // Autre ligne, autre colonne, autre bloc
        assert!(g.est_valide(4, 4, 5));
    }

    #[test]
    fn test_depuis_texte_valide() {
        let texte = format!("53{}", "0".repeat(79));
        let g = Grille::depuis_texte(String::from("Test"), &texte).unwrap();
        assert_eq!(g.cases[0][0], 5);
        assert_eq!(g.cases[0][1], 3);
        assert_eq!(g.cases[0][2], 0);
    }

    #[test]
    fn test_depuis_texte_mauvaise_longueur() {
        assert!(Grille::depuis_texte(String::from("Test"), "123").is_err());
    }

    #[test]
    fn test_depuis_texte_caractere_invalide() {
        let texte = format!("x{}", "0".repeat(80));
        assert!(Grille::depuis_texte(String::from("Test"), &texte).is_err());
    }

    #[test]
    fn test_depuis_texte_regles_brisees() {
        // Deux 5 sur la première ligne
        let texte = format!("55{}", "0".repeat(79));
        assert!(Grille::depuis_texte(String::from("Test"), &texte).is_err());
    }

    const SOLUTION: &str = "534678912672195348198342567859761423\
                            426853791713924856961537284287419635\
                            345286179";

    #[test]
    fn test_case_de_depart_verrouillee() {
        let mut g = Grille::depuis_texte(String::from("Test"), DEPART).unwrap();
        // (0, 0) vaut 5 au départ : ni modifiable, ni effaçable
        assert!(!g.placer(0, 0, 9));
        assert!(!g.effacer(0, 0));
        assert_eq!(g.cases[0][0], 5);
    }

    #[test]
    fn test_effacer_case_du_joueur() {
        let mut g = Grille::depuis_texte(String::from("Test"), DEPART).unwrap();
        // (0, 2) est vide au départ, la solution y met 4
        assert!(g.placer(0, 2, 4));
        assert!(g.effacer(0, 2));
        assert_eq!(g.cases[0][2], 0);
    }

    #[test]
    fn test_est_terminee() {
        let depart = Grille::depuis_texte(String::from("Test"), DEPART).unwrap();
        assert!(!depart.est_terminee());

        let complete = Grille::depuis_texte(String::from("Test"), SOLUTION).unwrap();
        assert!(complete.est_terminee());
    }

    #[test]
    fn test_resoudre_trouve_la_solution() {
        let mut g = Grille::depuis_texte(String::from("Test"), DEPART).unwrap();
        assert!(g.resoudre());
        let attendue = Grille::depuis_texte(String::from("Test"), SOLUTION).unwrap();
        assert_eq!(g.cases, attendue.cases);
    }

    #[test]
    fn test_resoudre_grille_vide() {
        let mut g = Grille::nouvelle(String::from("Test"));
        assert!(g.resoudre());
        assert!(g.est_terminee());
    }

    #[test]
    fn test_resoudre_grille_impossible() {
        // La case (0, 8) ne peut contenir ni 1-8 (ligne) ni 9 (colonne)
        let texte = format!("123456780000000009{}", "0".repeat(63));
        let mut g = Grille::depuis_texte(String::from("Test"), &texte).unwrap();
        assert!(!g.resoudre());
        assert_eq!(g.cases[0][8], 0);
    }

    #[test]
    fn test_compter_solutions() {
        // Grille vide : beaucoup de solutions, on s'arrête à la limite
        let mut vide = Grille::nouvelle(String::from("Test"));
        assert_eq!(vide.compter_solutions(2), 2);

        // La grille de départ n'a qu'une solution
        let mut g = Grille::depuis_texte(String::from("Test"), DEPART).unwrap();
        assert_eq!(g.compter_solutions(2), 1);

        // La grille est remise dans son état d'origine
        let identique = Grille::depuis_texte(String::from("Test"), DEPART).unwrap();
        assert_eq!(g.cases, identique.cases);
    }

    #[test]
    fn test_generer_solution_unique() {
        let mut g = Grille::generer(String::from("Test"), 40);
        let vides = g.cases.iter().flatten().filter(|&&c| c == 0).count();
        assert!(vides > 0 && vides <= 40);
        assert_eq!(g.compter_solutions(2), 1);
    }

    #[test]
    fn test_generer_verrouille_les_cases_de_depart() {
        let g = Grille::generer(String::from("Test"), 40);
        for ligne in 0..9 {
            for col in 0..9 {
                assert_eq!(g.fixes[ligne][col], g.cases[ligne][col] != 0);
            }
        }
    }

    #[test]
    fn test_generer_est_resoluble() {
        let mut g = Grille::generer(String::from("Test"), 40);
        assert!(g.resoudre());
        assert!(g.est_terminee());
    }

    #[test]
    fn test_generer_donne_des_grilles_differentes() {
        let a = Grille::generer(String::from("A"), 40);
        let b = Grille::generer(String::from("B"), 40);
        assert_ne!(a.cases, b.cases);
    }

    #[test]
    fn test_indice_revele_la_bonne_valeur() {
        let mut g = Grille::depuis_texte(String::from("Test"), DEPART).unwrap();
        let solution = Grille::depuis_texte(String::from("Test"), SOLUTION).unwrap();

        let (ligne, col, valeur) = g.indice().unwrap();
        assert_eq!(g.cases[ligne][col], valeur);
        assert_eq!(valeur, solution.cases[ligne][col]);
        // 51 cases vides au départ, une de moins maintenant
        let vides = g.cases.iter().flatten().filter(|&&c| c == 0).count();
        assert_eq!(vides, 50);
    }

    #[test]
    fn test_indices_successifs_finissent_la_grille() {
        let mut g = Grille::depuis_texte(String::from("Test"), DEPART).unwrap();
        while g.indice().is_some() {}
        assert!(g.est_terminee());
    }

    #[test]
    fn test_indice_grille_pleine() {
        let mut g = Grille::depuis_texte(String::from("Test"), SOLUTION).unwrap();
        assert!(g.indice().is_none());
    }

    #[test]
    fn test_indice_apres_coup_sans_issue() {
        let mut g = Grille::depuis_texte(String::from("Test"), DEPART).unwrap();
        // 1 respecte les règles en (0, 2), mais la solution y met 4 : c'est une impasse
        assert!(g.placer(0, 2, 1));
        let avant = g.cases;
        assert!(g.indice().is_none());
        assert_eq!(g.cases, avant);
    }

    #[test]
    fn test_valeur_et_est_fixe() {
        let g = Grille::depuis_texte(String::from("Test"), DEPART).unwrap();
        assert_eq!(g.valeur(0, 0), 5);
        assert!(g.est_fixe(0, 0));
        assert_eq!(g.valeur(0, 2), 0);
        assert!(!g.est_fixe(0, 2));
    }

    #[test]
    fn test_remplir_solution_ignore_les_coups_du_joueur() {
        let mut g = Grille::depuis_texte(String::from("Test"), DEPART).unwrap();
        // Coup valide mais faux : la solution met 4 en (0, 2)
        assert!(g.placer(0, 2, 1));
        assert!(g.remplir_solution());
        let attendue = Grille::depuis_texte(String::from("Test"), SOLUTION).unwrap();
        assert_eq!(g.cases, attendue.cases);
    }

    #[test]
    fn test_generer_niveau() {
        let g = Grille::generer_niveau(Niveau::Facile);
        assert_eq!(g.nom, "Partie facile");
        let vides = g.cases.iter().flatten().filter(|&&c| c == 0).count();
        assert!(vides > 0 && vides <= 35);
    }

    #[test]
    fn test_est_correct() {
        let g = Grille::depuis_texte(String::from("Test"), DEPART).unwrap();
        // La solution met 4 en (0, 2)
        assert!(g.est_correct(0, 2, 4));
        assert!(!g.est_correct(0, 2, 1));
        // Sans solution connue, on ne juge pas
        let vide = Grille::nouvelle(String::from("Test"));
        assert!(vide.est_correct(0, 0, 7));
    }

    #[test]
    fn test_generer_connait_sa_solution() {
        let mut g = Grille::generer(String::from("Test"), 40);
        assert!(g.remplir_solution());
        for ligne in 0..9 {
            for col in 0..9 {
                assert!(g.est_correct(ligne, col, g.valeur(ligne, col)));
            }
        }
    }

    #[test]
    fn test_creer_selon_l_origine() {
        let fixe = Grille::creer(Origine::Fixe).unwrap();
        assert_eq!(fixe.valeur(0, 0), 5);
        let facile = Grille::creer(Origine::Niveau(Niveau::Facile)).unwrap();
        assert_eq!(facile.nom, "Partie facile");
    }

    #[test]
    fn test_placer_refuse_coup_invalide() {
        let mut g = Grille::nouvelle(String::from("Test"));
        g.placer(0, 0, 5);
        assert!(!g.placer(0, 1, 5));
        assert_eq!(g.cases[0][1], 0);
    }
}
