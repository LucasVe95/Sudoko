//! Un programme qui affiche le jeu de Sudoku

mod grille;

use grille::{Grille, DEPART};
use std::io;

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

    // Le premier argument de la ligne de commande choisit la grille : cargo run -- facile
    let choix = std::env::args().nth(1);
    let grille = match choix.as_deref() {
        Some("fixe") => Grille::depuis_texte(String::from("Grille fixe"), DEPART),
        Some("facile") => Ok(Grille::generer(String::from("Partie facile"), 35)),
        None | Some("moyen") => Ok(Grille::generer(String::from("Partie moyenne"), 45)),
        Some("difficile") => Ok(Grille::generer(String::from("Partie difficile"), 55)),
        Some(autre) => Err(format!(
            "Mode inconnu : '{}' (choix : facile, moyen, difficile, fixe)",
            autre
        )),
    };

    match grille {
        Ok(g) => jouer(g),
        Err(erreur) => println!("{}", erreur),
    }
}
