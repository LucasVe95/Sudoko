//! La boucle de jeu au terminal : lecture des commandes du joueur et réactions du programme.

use crate::grille::Grille;
use std::io;

// Une commande tapée par le joueur
#[derive(Debug, PartialEq)]
enum Commande {
    Placer { ligne: usize, col: usize, valeur: u8 },
    Effacer { ligne: usize, col: usize },
    Indice,
    Solution,
    Quitter,
}

// Transforme une ligne de texte en commande, ou en message d'erreur à afficher.
// `cote` est le côté de la grille : les numéros vont de 1 jusqu'à `cote`.
fn lire_commande(saisie: &str, cote: usize) -> Result<Commande, String> {
    match saisie.trim() {
        "q" => Ok(Commande::Quitter),
        "s" => Ok(Commande::Solution),
        "h" => Ok(Commande::Indice),
        coup => lire_coup(coup, cote),
    }
}

// Lit un coup de la forme `ligne colonne valeur`. Le joueur compte de 1 à `cote`,
// on convertit donc la ligne et la colonne en index de 0 à `cote` - 1.
fn lire_coup(texte: &str, cote: usize) -> Result<Commande, String> {
    // Convertit chaque mot en nombre : un seul échec et on obtient Err
    let nombres: Result<Vec<usize>, _> = texte
        .split_whitespace()
        .map(|mot| mot.parse::<usize>())
        .collect();
    let (ligne, col, valeur) = match nombres {
        Ok(n) if n.len() == 3 => (n[0], n[1], n[2]),
        _ => return Err(String::from("Saisie invalide : tapez trois nombres, par exemple 1 3 4")),
    };

    if !(1..=cote).contains(&ligne) || !(1..=cote).contains(&col) || valeur > cote {
        return Err(format!(
            "Valeurs hors limites : ligne et colonne de 1 à {}, valeur de 0 à {}",
            cote, cote
        ));
    }

    let (ligne, col) = (ligne - 1, col - 1);
    if valeur == 0 {
        Ok(Commande::Effacer { ligne, col })
    } else {
        Ok(Commande::Placer { ligne, col, valeur: valeur as u8 })
    }
}

// Affiche la solution de la grille de départ
fn afficher_solution(grille: &mut Grille) {
    if grille.remplir_solution() {
        println!("Solution :");
        grille.afficher();
    } else {
        println!("Cette grille n'a pas de solution");
    }
}

// Boucle de jeu : lit les commandes du joueur jusqu'à la victoire ou jusqu'à ce qu'il quitte
pub fn jouer(mut grille: Grille) {
    println!("{}", grille.nom);
    let cote = grille.taille().cote();
    loop {
        grille.afficher();

        if grille.est_terminee() {
            println!("Bravo, grille terminée !");
            break;
        }

        println!(
            "Coup : ligne colonne valeur (1-{}, 0 pour effacer), 'h' pour un indice, 's' pour la solution, 'q' pour quitter",
            cote
        );
        let mut saisie = String::new();
        match io::stdin().read_line(&mut saisie) {
            // Ok(0) = fin de l'entrée (plus rien à lire), on arrête
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }

        match lire_commande(&saisie, cote) {
            Err(message) => println!("{}", message),
            Ok(Commande::Quitter) => break,
            Ok(Commande::Solution) => {
                afficher_solution(&mut grille);
                break;
            }
            Ok(Commande::Indice) => match grille.indice() {
                Some((ligne, col, valeur)) => {
                    println!("Indice : {} en ligne {}, colonne {}", valeur, ligne + 1, col + 1)
                }
                None => println!(
                    "Pas d'indice possible : vos coups bloquent la grille, effacez-en quelques-uns"
                ),
            },
            Ok(Commande::Placer { ligne, col, valeur }) => {
                if !grille.placer(ligne, col, valeur) {
                    println!("Coup refusé : case de départ ou règle du sudoku brisée");
                }
            }
            Ok(Commande::Effacer { ligne, col }) => {
                if !grille.effacer(ligne, col) {
                    println!("Coup refusé : c'est une case de départ");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commandes_d_une_lettre() {
        assert_eq!(lire_commande("q", 9), Ok(Commande::Quitter));
        assert_eq!(lire_commande("s", 9), Ok(Commande::Solution));
        assert_eq!(lire_commande("h", 9), Ok(Commande::Indice));
        // Le retour à la ligne laissé par read_line ne gêne pas
        assert_eq!(lire_commande("h\n", 9), Ok(Commande::Indice));
    }

    #[test]
    fn test_placer_convertit_en_index() {
        assert_eq!(
            lire_commande("1 3 4", 9),
            Ok(Commande::Placer { ligne: 0, col: 2, valeur: 4 })
        );
        assert_eq!(
            lire_commande("9 9 1\n", 9),
            Ok(Commande::Placer { ligne: 8, col: 8, valeur: 1 })
        );
    }

    #[test]
    fn test_valeur_zero_efface() {
        assert_eq!(
            lire_commande("1 3 0", 9),
            Ok(Commande::Effacer { ligne: 0, col: 2 })
        );
    }

    #[test]
    fn test_saisies_invalides() {
        for saisie in ["", "abc", "1 2", "1 2 3 4", "1 x 3", "-1 2 3"] {
            assert!(lire_commande(saisie, 9).is_err(), "'{}' aurait dû être refusée", saisie);
        }
    }

    #[test]
    fn test_valeurs_hors_limites() {
        for saisie in ["0 5 5", "5 0 5", "10 5 5", "5 10 5", "1 1 10"] {
            assert!(lire_commande(saisie, 9).is_err(), "'{}' aurait dû être refusée", saisie);
        }
    }

    #[test]
    fn test_les_limites_suivent_la_taille_de_la_grille() {
        // En 16×16, on va jusqu'à 16 pour les lignes, les colonnes et les valeurs
        assert_eq!(
            lire_commande("16 16 16", 16),
            Ok(Commande::Placer { ligne: 15, col: 15, valeur: 16 })
        );
        assert_eq!(
            lire_commande("10 12 13", 16),
            Ok(Commande::Placer { ligne: 9, col: 11, valeur: 13 })
        );
        assert!(lire_commande("17 1 1", 16).is_err());
        assert!(lire_commande("1 1 17", 16).is_err());

        // En 4×4, 5 est déjà hors limites
        assert!(lire_commande("1 1 5", 4).is_err());
        assert!(lire_commande("5 1 1", 4).is_err());
        assert!(lire_commande("4 4 4", 4).is_ok());
    }
}
