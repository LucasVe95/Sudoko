//! Un programme qui affiche le jeu de Sudoku

struct Grille {
    cases: [[u8; 9]; 9],
    nom: String,
}
impl Grille {
    //"Constructeur" de la structure Grille
    fn nouvelle(nom: String) -> Grille {
        Grille {
            cases: [[0; 9]; 9],
            nom,
        }
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

    //methode qui modifie : &mut self
    fn placer(&mut self, ligne: usize, col: usize, valeur: u8) {
        self.cases[ligne][col] = valeur;
    }
}

fn main() {
    // Affiche le titre du programme
    println!("Sudoku");
    // Crée une nouvelle grille de Sudoku
    let mut g = Grille::nouvelle(String::from("Partie 1"));
    g.placer(0, 0, 5);
    g.afficher();
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placer() {
        let mut g = Grille::nouvelle(String::from("Test"));
        g.placer(0, 0, 5);
        assert_eq!(g.cases[0][0], 5);
    }
}
