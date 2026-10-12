//! Les sons du jeu : de courts bips joués par Windows. Ailleurs, le jeu reste silencieux.

// Les occasions où le jeu fait un bruit
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Son {
    // Un bon chiffre
    Bon,
    // Une erreur
    Erreur,
    // Un indice
    Indice,
    Victoire,
    Defaite,
}

impl Son {
    // Les notes du son, jouées l'une après l'autre : (fréquence en hertz, durée en millisecondes)
    pub fn notes(self) -> &'static [(u32, u32)] {
        match self {
            Son::Bon => &[(880, 60)],
            Son::Erreur => &[(220, 220)],
            Son::Indice => &[(660, 70), (880, 70)],
            Son::Victoire => &[(523, 120), (659, 120), (784, 120), (1047, 300)],
            Son::Defaite => &[(392, 200), (330, 200), (262, 400)],
        }
    }
}

// Joue un son sans faire attendre la fenêtre. Les bips sont envoyés à un fil qui les joue
// l'un après l'autre : plusieurs sons de suite ne se chevauchent pas.
#[cfg(windows)]
pub fn jouer(son: Son) {
    use std::sync::mpsc::{self, Sender};
    use std::sync::OnceLock;
    use std::thread;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn Beep(frequence: u32, duree: u32) -> i32;
    }

    static EMETTEUR: OnceLock<Sender<Son>> = OnceLock::new();
    let emetteur = EMETTEUR.get_or_init(|| {
        let (envoi, reception) = mpsc::channel::<Son>();
        thread::spawn(move || {
            for son in reception {
                for &(frequence, duree) in son.notes() {
                    // SAFETY: Beep ne prend que deux nombres et ne touche pas à la mémoire du programme.
                    // Si elle échoue (pas de carte son), on ignore : le jeu reste silencieux.
                    let _ = unsafe { Beep(frequence, duree) };
                }
            }
        });
        envoi
    });
    // Si le fil s'est arrêté, personne ne joue : l'erreur d'envoi est sans importance
    let _ = emetteur.send(son);
}

#[cfg(not(windows))]
pub fn jouer(_son: Son) {}

#[cfg(test)]
mod tests {
    use super::*;

    const TOUS: [Son; 5] = [Son::Bon, Son::Erreur, Son::Indice, Son::Victoire, Son::Defaite];

    #[test]
    fn test_chaque_son_a_des_notes_que_windows_accepte() {
        for son in TOUS {
            assert!(!son.notes().is_empty(), "{:?}", son);
            for &(frequence, duree) in son.notes() {
                // Beep n'accepte que les fréquences de 37 à 32767 Hz
                assert!((37..=32767).contains(&frequence), "{:?}", son);
                assert!(duree > 0, "{:?}", son);
            }
        }
    }

    #[test]
    fn test_les_sons_courts_restent_courts() {
        // Un bon chiffre ou une erreur ne doivent pas faire attendre
        let duree = |son: Son| son.notes().iter().map(|&(_, ms)| ms).sum::<u32>();
        assert!(duree(Son::Bon) <= 100);
        assert!(duree(Son::Erreur) <= 300);
        assert!(duree(Son::Victoire) > duree(Son::Bon));
    }

    #[test]
    fn test_la_melodie_de_victoire_monte_et_celle_de_defaite_descend() {
        let frequences = |son: Son| son.notes().iter().map(|&(f, _)| f).collect::<Vec<_>>();
        assert!(frequences(Son::Victoire).windows(2).all(|paire| paire[0] < paire[1]));
        assert!(frequences(Son::Defaite).windows(2).all(|paire| paire[0] > paire[1]));
    }
}
