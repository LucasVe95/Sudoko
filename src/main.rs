//! Un programme qui affiche le jeu de Sudoku

use std::io;

struct Grille {
    cases: [[u8; 9]; 9],
    // true pour les cases de la grille de départ, que le joueur ne peut pas modifier
    fixes: [[bool; 9]; 9],
    nom: String,
}
impl Grille {
    //"Constructeur" de la structure Grille
    fn nouvelle(nom: String) -> Grille {
        Grille {
            cases: [[0; 9]; 9],
            fixes: [[false; 9]; 9],
            nom,
        }
    }
    // Construit une grille à partir d'un texte de 81 chiffres (0 = case vide)
    fn depuis_texte(nom: String, texte: &str) -> Result<Grille, String> {
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

        Ok(grille)
    }

    //methode qui lit seulement :&self
    fn afficher(&self) {
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
    fn placer(&mut self, ligne: usize, col: usize, valeur: u8) -> bool {
        if self.fixes[ligne][col] || !self.est_valide(ligne, col, valeur) {
            return false;
        }
        self.cases[ligne][col] = valeur;
        true
    }

    // Vide une case. Renvoie false si c'est une case de départ
    fn effacer(&mut self, ligne: usize, col: usize) -> bool {
        if self.fixes[ligne][col] {
            return false;
        }
        self.cases[ligne][col] = 0;
        true
    }

    // La grille est gagnée quand toutes les cases sont remplies.
    // Chaque chiffre ayant été vérifié par est_valide, il n'y a rien d'autre à contrôler.
    fn est_terminee(&self) -> bool {
        self.cases.iter().all(|ligne| ligne.iter().all(|&case| case != 0))
    }

    // Remplit la grille par retour sur trace (backtracking).
    // Renvoie true si une solution a été trouvée, false sinon (la grille est alors inchangée).
    fn resoudre(&mut self) -> bool {
        for ligne in 0..9 {
            for col in 0..9 {
                // On cherche la première case vide
                if self.cases[ligne][col] == 0 {
                    for valeur in 1..=9 {
                        if self.est_valide(ligne, col, valeur) {
                            self.cases[ligne][col] = valeur;
                            // On tente de résoudre le reste avec ce choix
                            if self.resoudre() {
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
}

// Grille de départ : 81 chiffres, 0 = case vide
const DEPART: &str = "530070000\
                      600195000\
                      098000060\
                      800060003\
                      400803001\
                      700020006\
                      060000280\
                      000419005\
                      000080079";

// Boucle de jeu : lit les coups du joueur au clavier jusqu'à la victoire ou 'q'
fn jouer(mut grille: Grille) {
    println!("{}", grille.nom);
    loop {
        grille.afficher();

        if grille.est_terminee() {
            println!("Bravo, grille terminée !");
            break;
        }

        println!("Coup : ligne colonne valeur (1-9, 0 pour effacer), 's' pour la solution, 'q' pour quitter");
        let mut saisie = String::new();
        match io::stdin().read_line(&mut saisie) {
            // Ok(0) = fin de l'entrée (plus rien à lire), on arrête
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }

        let saisie = saisie.trim();
        if saisie == "q" {
            break;
        }

        if saisie == "s" {
            // On repart des seules cases de départ, pour qu'un mauvais coup du joueur ne gêne pas
            for ligne in 0..9 {
                for col in 0..9 {
                    grille.effacer(ligne, col);
                }
            }
            if grille.resoudre() {
                println!("Solution :");
                grille.afficher();
            } else {
                println!("Cette grille n'a pas de solution");
            }
            break;
        }

        // Convertit chaque mot en nombre : un seul échec et on obtient Err
        let nombres: Result<Vec<usize>, _> = saisie
            .split_whitespace()
            .map(|mot| mot.parse::<usize>())
            .collect();
        let (ligne, col, valeur) = match nombres {
            Ok(n) if n.len() == 3 => (n[0], n[1], n[2]),
            _ => {
                println!("Saisie invalide : tapez trois nombres, par exemple 1 3 4");
                continue;
            }
        };

        if !(1..=9).contains(&ligne) || !(1..=9).contains(&col) || valeur > 9 {
            println!("Valeurs hors limites : ligne et colonne de 1 à 9, valeur de 0 à 9");
            continue;
        }

        // On passe des numéros humains (1-9) aux index du tableau (0-8)
        let (ligne, col) = (ligne - 1, col - 1);
        let accepte = if valeur == 0 {
            grille.effacer(ligne, col)
        } else {
            grille.placer(ligne, col, valeur as u8)
        };
        if !accepte {
            println!("Coup refusé : case de départ ou règle du sudoku brisée");
        }
    }
}

fn main() {
    // Affiche le titre du programme
    println!("Sudoku");
    match Grille::depuis_texte(String::from("Partie 1"), DEPART) {
        Ok(g) => jouer(g),
        Err(erreur) => println!("Grille invalide : {}", erreur),
    }
}
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
    fn test_placer_refuse_coup_invalide() {
        let mut g = Grille::nouvelle(String::from("Test"));
        g.placer(0, 0, 5);
        assert!(!g.placer(0, 1, 5));
        assert_eq!(g.cases[0][1], 0);
    }
}
