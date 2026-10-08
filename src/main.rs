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
        if !self.est_valide(ligne, col, valeur) {
            return false;
        }
        self.cases[ligne][col] = valeur;
        true
    }
}

fn main() {
    // Affiche le titre du programme
    println!("Sudoku");
    // Grille de départ : 81 chiffres, 0 = case vide
    let depart = "530070000\
                  600195000\
                  098000060\
                  800060003\
                  400803001\
                  700020006\
                  060000280\
                  000419005\
                  000080079";

    match Grille::depuis_texte(String::from("Partie 1"), depart) {
        Ok(g) => g.afficher(),
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

    #[test]
    fn test_placer_refuse_coup_invalide() {
        let mut g = Grille::nouvelle(String::from("Test"));
        g.placer(0, 0, 5);
        assert!(!g.placer(0, 1, 5));
        assert_eq!(g.cases[0][1], 0);
    }
}
