//! Un programme qui affiche le jeu de Sudoku

mod classement;
mod grille;
mod gui;
mod jeu;
mod reglages;
mod regles;
mod resolveur;
mod sauvegarde;
mod solveur;
mod sons;
mod statistiques;
mod technique;

use grille::{Grille, Niveau, Origine, Taille};
use regles::Variante;

// Traduit les mots tapés sur la ligne de commande : le mode (facile, moyen, difficile ou fixe),
// puis éventuellement la taille de la grille (4, 6, 9, 12 ou 16 ; 9 par défaut)
fn lire_origine(mode: &str, taille: Option<&str>, variante: Option<&str>) -> Result<Origine, String> {
    let niveau = match mode {
        "facile" => Niveau::Facile,
        "moyen" => Niveau::Moyen,
        "difficile" => Niveau::Difficile,
        "fixe" => {
            return match (taille, variante) {
                (None, None) => Ok(Origine::Fixe),
                _ => Err(String::from("La grille fixe n'existe qu'en 9×9 classique")),
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
    let variante = match variante {
        None => Variante::Classique,
        Some(texte) => Variante::depuis_mot(texte).ok_or_else(|| {
            format!(
                "Variante inconnue : '{}' (choix : classique, diagonale, hyper, anticavalier)",
                texte
            )
        })?,
    };
    if !variante.accepte(taille) {
        return Err(format!(
            "La variante {} n'existe pas en {}",
            variante.libelle(),
            taille.libelle()
        ));
    }
    Ok(Origine::Niveau(niveau, taille, variante))
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
        Some(mode) => match lire_origine(
            mode,
            arguments.get(1).map(String::as_str),
            arguments.get(2).map(String::as_str),
        ) {
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
        let origine = origine.unwrap_or(Origine::Niveau(Niveau::Moyen, Taille::Neuf, Variante::Classique));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lire_origine_sans_variante_est_classique() {
        assert_eq!(
            lire_origine("facile", None, None),
            Ok(Origine::Niveau(Niveau::Facile, Taille::Neuf, Variante::Classique))
        );
        assert_eq!(
            lire_origine("difficile", Some("16"), None),
            Ok(Origine::Niveau(Niveau::Difficile, Taille::Seize, Variante::Classique))
        );
        assert_eq!(lire_origine("fixe", None, None), Ok(Origine::Fixe));
    }

    #[test]
    fn test_lire_origine_avec_une_variante() {
        assert_eq!(
            lire_origine("moyen", Some("9"), Some("diagonale")),
            Ok(Origine::Niveau(Niveau::Moyen, Taille::Neuf, Variante::Diagonale))
        );
        assert_eq!(
            lire_origine("moyen", Some("12"), Some("anticavalier")),
            Ok(Origine::Niveau(Niveau::Moyen, Taille::Douze, Variante::AntiCavalier))
        );
        assert_eq!(
            lire_origine("facile", Some("9"), Some("hyper")),
            Ok(Origine::Niveau(Niveau::Facile, Taille::Neuf, Variante::Hyper))
        );
    }

    #[test]
    fn test_lire_origine_refuse_ce_qui_n_existe_pas() {
        // Une variante inconnue, ou qui n'existe pas dans cette taille
        let erreur = lire_origine("facile", Some("9"), Some("bizarre")).unwrap_err();
        assert!(erreur.contains("Variante inconnue"), "{}", erreur);
        let erreur = lire_origine("facile", Some("6"), Some("hyper")).unwrap_err();
        assert!(erreur.contains("n'existe pas en 6×6"), "{}", erreur);
        // La grille fixe n'a ni taille ni variante
        assert!(lire_origine("fixe", Some("9"), None).is_err());
        assert!(lire_origine("fixe", None, Some("diagonale")).is_err());
        assert!(lire_origine("inconnu", None, None).is_err());
        assert!(lire_origine("facile", Some("7"), None).is_err());
    }
}
