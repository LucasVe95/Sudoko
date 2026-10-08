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
        for ligne in self.cases.iter() {
            println!("{:?}", ligne);
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
