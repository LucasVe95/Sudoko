//! Un programme qui affiche le jeu de Sudoku

mod classement;
mod grille;
mod gui;
mod jeu;
mod solveur;

use grille::{Grille, Niveau, Origine, Taille};

// Traduit les mots tapés sur la ligne de commande : le mode (facile, moyen, difficile ou fixe),
// puis éventuellement la taille de la grille (4, 6, 9, 12 ou 16 ; 9 par défaut)
fn lire_origine(mode: &str, taille: Option<&str>) -> Result<Origine, String> {
    let niveau = match mode {
        "facile" => Niveau::Facile,
        "moyen" => Niveau::Moyen,
        "difficile" => Niveau::Difficile,
        "fixe" => {
            return match taille {
                None => Ok(Origine::Fixe),
                Some(_) => Err(String::from("La grille fixe n'existe qu'en 9×9")),
            };
        }
        autre => {
            return Err(format!(
                "Mode inconnu : '{}' (choix : facile, moyen, difficile, fixe)",
                autre
            ));
        }
    };

    let taille = match taille {
        None => Taille::Neuf,
        Some(texte) => texte
            .parse::<usize>()
            .ok()
            .and_then(Taille::depuis_cote)
            .ok_or_else(|| format!("Taille inconnue : '{}' (choix : 4, 6, 9, 12, 16)", texte))?,
    };
    Ok(Origine::Niveau(niveau, taille))
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
        Some(mode) => match lire_origine(mode, arguments.get(1).map(String::as_str)) {
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
        let origine = origine.unwrap_or(Origine::Niveau(Niveau::Moyen, Taille::Neuf));
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
