//! Un programme qui affiche le jeu de Sudoku

mod grille;
mod jeu;

use grille::{Grille, DEPART};

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
        Ok(g) => jeu::jouer(g),
        Err(erreur) => println!("{}", erreur),
    }
}
