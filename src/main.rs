//! Un programme qui affiche le jeu de Sudoku

mod grille;
mod gui;
mod jeu;

use grille::{Grille, Niveau, DEPART};

// Choisit la grille de départ selon le mot tapé : facile, moyen, difficile ou fixe
fn choisir_grille(choix: Option<&str>) -> Result<Grille, String> {
    match choix {
        Some("fixe") => Grille::depuis_texte(String::from("Grille fixe"), DEPART),
        Some("facile") => Ok(Grille::generer_niveau(Niveau::Facile)),
        None | Some("moyen") => Ok(Grille::generer_niveau(Niveau::Moyen)),
        Some("difficile") => Ok(Grille::generer_niveau(Niveau::Difficile)),
        Some(autre) => Err(format!(
            "Mode inconnu : '{}' (choix : facile, moyen, difficile, fixe)",
            autre
        )),
    }
}

fn main() {
    let mut arguments: Vec<String> = std::env::args().skip(1).collect();

    // `terminal` en premier argument : on joue dans le terminal au lieu de la fenêtre
    let terminal = arguments.first().is_some_and(|argument| argument == "terminal");
    if terminal {
        arguments.remove(0);
    }

    let grille = match choisir_grille(arguments.first().map(String::as_str)) {
        Ok(grille) => grille,
        Err(erreur) => {
            println!("{}", erreur);
            return;
        }
    };

    if terminal {
        println!("Sudoku");
        jeu::jouer(grille);
    } else if let Err(erreur) = gui::lancer(grille) {
        println!("Impossible d'ouvrir la fenêtre : {}", erreur);
    }
}
