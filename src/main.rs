//! Un programme qui affiche le jeu de Sudoku

mod classement;
mod grille;
mod gui;
mod jeu;

use grille::{Grille, Niveau, Origine};

// Traduit le mot tapé sur la ligne de commande : facile, moyen, difficile ou fixe
fn lire_origine(mot: &str) -> Result<Origine, String> {
    match mot {
        "facile" => Ok(Origine::Niveau(Niveau::Facile)),
        "moyen" => Ok(Origine::Niveau(Niveau::Moyen)),
        "difficile" => Ok(Origine::Niveau(Niveau::Difficile)),
        "fixe" => Ok(Origine::Fixe),
        autre => Err(format!(
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

    // Sans mot qui désigne une grille, la fenêtre s'ouvre sur le menu
    let origine = match arguments.first() {
        Some(mot) => match lire_origine(mot) {
            Ok(origine) => Some(origine),
            Err(erreur) => {
                println!("{}", erreur);
                return;
            }
        },
        None => None,
    };

    if terminal {
        // Au terminal, il n'y a pas de menu : une grille moyenne par défaut
        let origine = origine.unwrap_or(Origine::Niveau(Niveau::Moyen));
        match Grille::creer(origine) {
            Ok(grille) => {
                println!("Sudoku");
                jeu::jouer(grille);
            }
            Err(erreur) => println!("{}", erreur),
        }
    } else if let Err(erreur) = gui::lancer(origine) {
        println!("Impossible d'ouvrir la fenêtre : {}", erreur);
    }
}
