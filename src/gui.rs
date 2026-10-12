//! L'interface graphique (egui) : menu, partie et écran de fin.

use crate::classement::{
    calculer_score, maintenant, nettoyer_nom, penalite_erreur, penalite_indice, Classement, Entree,
};
use crate::grille::{Grille, Indice, Niveau, Origine, Taille};
use crate::reglages::{Reglages, Theme};
use crate::resolveur::Resolveur;
use crate::sauvegarde::{PartieSauvee, Sauvegarde};
use crate::sons::{self, Son};
use crate::statistiques::{PartieJouee, Resultat, Statistiques};
use eframe::egui::{
    self, Align, Align2, Color32, FontId, Key, Layout, Pos2, Rect, RichText, Sense, Stroke, TextEdit, Vec2,
};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

// Nombre d'erreurs à partir duquel la partie est perdue
const MAX_ERREURS: u32 = 3;

// Combien de secondes un premier chiffre attend le second (pour taper 10 à 16)
const DELAI_SAISIE: f64 = 0.8;

// Les touches qui placent un chiffre. Le 0 efface la case.
const TOUCHES_CHIFFRES: [(Key, u8); 10] = [
    (Key::Num0, 0),
    (Key::Num1, 1),
    (Key::Num2, 2),
    (Key::Num3, 3),
    (Key::Num4, 4),
    (Key::Num5, 5),
    (Key::Num6, 6),
    (Key::Num7, 7),
    (Key::Num8, 8),
    (Key::Num9, 9),
];

// Ce qu'est un chiffre de la grille, pour choisir sa couleur
#[derive(Clone, Copy, PartialEq, Debug)]
enum GenreChiffre {
    // Un chiffre de la grille de départ
    Depart,
    // Un chiffre posé par le joueur (ou trouvé par le solveur)
    Joueur,
    // Un chiffre qui n'est pas celui de la solution, ou qui contredit un autre
    Faux,
}

// Les couleurs de la grille et des messages, selon le thème (clair ou sombre) et le mode daltonien
struct Palette {
    fond: Color32,
    case_selectionnee: Color32,
    meme_chiffre: Color32,
    ligne_colonne_bloc: Color32,
    chiffre_depart: Color32,
    chiffre_joueur: Color32,
    chiffre_faux: Color32,
    note: Color32,
    trait_fin: Color32,
    trait_epais: Color32,
    vert: Color32,
    rouge: Color32,
}

impl Palette {
    fn pour(sombre: bool, daltonien: bool) -> Palette {
        let mut palette = if sombre {
            Palette {
                fond: Color32::from_rgb(30, 30, 34),
                case_selectionnee: Color32::from_rgb(65, 100, 165),
                meme_chiffre: Color32::from_rgb(55, 70, 105),
                ligne_colonne_bloc: Color32::from_rgb(45, 48, 56),
                chiffre_depart: Color32::from_rgb(235, 235, 235),
                chiffre_joueur: Color32::from_rgb(110, 170, 255),
                chiffre_faux: Color32::from_rgb(255, 100, 100),
                note: Color32::from_rgb(150, 150, 170),
                trait_fin: Color32::from_rgb(90, 90, 95),
                trait_epais: Color32::from_rgb(210, 210, 215),
                vert: Color32::from_rgb(90, 200, 120),
                rouge: Color32::from_rgb(240, 100, 100),
            }
        } else {
            Palette {
                fond: Color32::WHITE,
                case_selectionnee: Color32::from_rgb(170, 205, 250),
                meme_chiffre: Color32::from_rgb(210, 225, 250),
                ligne_colonne_bloc: Color32::from_rgb(235, 240, 248),
                chiffre_depart: Color32::from_rgb(25, 25, 25),
                chiffre_joueur: Color32::from_rgb(30, 90, 200),
                chiffre_faux: Color32::from_rgb(210, 40, 40),
                note: Color32::from_rgb(110, 110, 130),
                trait_fin: Color32::from_rgb(170, 170, 170),
                trait_epais: Color32::from_rgb(25, 25, 25),
                vert: Color32::from_rgb(40, 160, 70),
                rouge: Color32::from_rgb(210, 60, 60),
            }
        };
        if daltonien {
            // Bleu, orange et vert bleuté : ces couleurs restent distinctes quand on confond
            // le rouge et le vert (palette d'Okabe et Ito)
            if sombre {
                palette.chiffre_joueur = Color32::from_rgb(86, 180, 233);
                palette.chiffre_faux = Color32::from_rgb(240, 160, 40);
                palette.vert = Color32::from_rgb(0, 200, 150);
                palette.rouge = Color32::from_rgb(240, 130, 60);
            } else {
                palette.chiffre_joueur = Color32::from_rgb(0, 114, 178);
                palette.chiffre_faux = Color32::from_rgb(213, 94, 0);
                palette.vert = Color32::from_rgb(0, 158, 115);
                palette.rouge = Color32::from_rgb(213, 94, 0);
            }
        }
        palette
    }

    fn chiffre(&self, genre: GenreChiffre) -> Color32 {
        match genre {
            GenreChiffre::Depart => self.chiffre_depart,
            GenreChiffre::Joueur => self.chiffre_joueur,
            GenreChiffre::Faux => self.chiffre_faux,
        }
    }
}

// Le thème d'egui qui correspond à un thème choisi dans les réglages
fn theme_egui(theme: Theme) -> egui::ThemePreference {
    match theme {
        Theme::Systeme => egui::ThemePreference::System,
        Theme::Clair => egui::ThemePreference::Light,
        Theme::Sombre => egui::ThemePreference::Dark,
    }
}

// Comment une partie s'est terminée
#[derive(Clone, Copy, PartialEq, Debug)]
enum Fin {
    Victoire,
    Defaite,
    Abandon,
}

// Un changement dans une case : ce qu'elle contenait avant et après (0 = vide)
#[derive(Clone, Copy, PartialEq, Debug)]
struct Coup {
    ligne: usize,
    col: usize,
    avant: u8,
    apres: u8,
    // Les notes que la case portait avant le coup : poser un chiffre les efface,
    // annuler le coup les remet
    notes: u16,
}

// Ce que la partie demande à l'application de faire à sa place
enum Action {
    Menu,
    Rejouer,
    Classement,
}

// Ce que le joueur choisit dans le menu
enum ChoixMenu {
    Reprendre,
    Nouvelle(Origine),
    Classement,
    Statistiques,
    Resolution,
    Reglages,
    Quitter,
}

// Combien de lignes le classement affiche
const LIGNES_CLASSEMENT: usize = 10;

// Met une durée en minutes:secondes
fn formater_duree(duree: Duration) -> String {
    let secondes = duree.as_secs();
    format!("{:02}:{:02}", secondes / 60, secondes % 60)
}

// Traite un chiffre tapé au clavier. Dans une grille qui va jusqu'à `maximum`, un nombre de
// 10 à 16 s'écrit avec deux touches : le premier chiffre attend alors le second.
// `tampon` est le premier chiffre en attente, s'il y en a un.
// Renvoie (le nombre à placer, le nouveau chiffre en attente).
fn saisir_chiffre(tampon: Option<u8>, chiffre: u8, maximum: u8) -> (Option<u8>, Option<u8>) {
    if let Some(premier) = tampon {
        let nombre = premier as u16 * 10 + chiffre as u16;
        if nombre <= maximum as u16 {
            return (Some(nombre as u8), None);
        }
        // Les deux chiffres ne forment pas un nombre de la grille : le dernier tapé compte seul
    }

    if chiffre == 0 {
        // Le 0 efface la case
        (Some(0), None)
    } else if chiffre > maximum {
        // Ce chiffre n'existe pas dans cette grille : on l'ignore
        (None, None)
    } else if chiffre as u16 * 10 <= maximum as u16 {
        // Il peut commencer un nombre à deux chiffres (1 peut devenir 10 à 16) : on attend
        (None, Some(chiffre))
    } else {
        (Some(chiffre), None)
    }
}

// Sous Windows, un programme lancé par double-clic reçoit une console noire en plus de sa fenêtre.
// On la ferme, mais seulement si on est le seul programme à l'utiliser : lancé depuis un terminal,
// la console n'est pas touchée.
#[cfg(windows)]
fn cacher_console_si_seule() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetConsoleProcessList(liste: *mut u32, taille: u32) -> u32;
        fn FreeConsole() -> i32;
    }

    let mut processus = [0u32; 2];
    // SAFETY: `processus` est un tableau de 2 entiers et on annonce bien une taille de 2.
    let nombre = unsafe { GetConsoleProcessList(processus.as_mut_ptr(), 2) };
    if nombre == 1 {
        // SAFETY: FreeConsole n'a aucun paramètre et ne touche pas à la mémoire du programme.
        unsafe { FreeConsole() };
    }
}

#[cfg(not(windows))]
fn cacher_console_si_seule() {}

// Ouvre la fenêtre et ne rend la main que lorsqu'elle est fermée.
// Avec une origine, la partie démarre directement ; sans, on commence par le menu.
pub fn lancer(origine: Option<Origine>) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 720.0])
            .with_min_inner_size([360.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Sudoku",
        options,
        Box::new(move |contexte| {
            // La fenêtre est ouverte : si une erreur était arrivée avant, la console l'aurait montrée
            cacher_console_si_seule();
            let mut application = Application::new();
            if let Some(origine) = origine {
                application.lancer_partie(origine, &contexte.egui_ctx);
            }
            Ok(Box::new(application))
        }),
    )
}

// Une partie en cours ou terminée : la grille et tout ce qui la concerne (erreurs, indices, chrono)
struct Partie {
    grille: Grille,
    origine: Origine,
    // Case choisie avec la souris ou les flèches : (ligne, col)
    selection: Option<(usize, usize)>,
    message: String,
    erreurs: u32,
    indices: u32,
    fin: Option<Fin>,
    // true quand la solution a été affichée : la grille pleine n'est alors pas une victoire
    solution_affichee: bool,
    // Le chrono : le temps déjà écoulé, plus le temps depuis la dernière reprise (None si en pause)
    ecoule: Duration,
    reprise: Option<Instant>,
    // Le score d'une victoire (None sans victoire, ou en mode entraînement)
    score: Option<u32>,
    // Le nom tapé pour enregistrer le score
    nom_joueur: String,
    // Le rang obtenu une fois le score enregistré
    rang: Option<usize>,
    // Le premier chiffre d'un nombre à deux chiffres, avec l'instant où il a été tapé
    tampon: Option<(u8, f64)>,
    // true quand la partie a changé depuis la dernière sauvegarde
    modifiee: bool,
    // Les changements faits par le joueur, du plus ancien au plus récent : de quoi les annuler
    historique: Vec<Coup>,
    // Les changements annulés, que l'on peut refaire. Un nouveau coup les efface.
    annules: Vec<Coup>,
    // Les notes au crayon de chaque case, en bits : le bit (v - 1) vaut 1 si la note v est écrite.
    // Une case n'a des notes que tant qu'elle est vide.
    notes: Vec<u16>,
    // true quand les chiffres tapés s'écrivent en notes au lieu d'être joués
    mode_notes: bool,
    // true quand la fin de la partie est déjà entrée dans les statistiques
    comptee: bool,
    // Les sons à jouer : l'application les prend à chaque image, et les joue si les sons sont activés
    sons: Vec<Son>,
}

impl Partie {
    fn new(grille: Grille, origine: Origine) -> Partie {
        let notes = vec![0; grille.taille().cases()];
        Partie {
            grille,
            origine,
            selection: None,
            message: String::new(),
            erreurs: 0,
            indices: 0,
            fin: None,
            solution_affichee: false,
            ecoule: Duration::ZERO,
            reprise: Some(Instant::now()),
            score: None,
            nom_joueur: String::new(),
            rang: None,
            tampon: None,
            modifiee: false,
            historique: Vec::new(),
            annules: Vec::new(),
            notes,
            mode_notes: false,
            comptee: false,
            sons: Vec::new(),
        }
    }

    // Reprend une partie enregistrée, en pause : le chrono repart quand on la reprend.
    // Une sauvegarde déjà finie (grille pleine, trop d'erreurs) ne se reprend pas.
    fn restaurer(sauvee: PartieSauvee) -> Option<Partie> {
        if sauvee.erreurs >= MAX_ERREURS || sauvee.grille.est_terminee() {
            return None;
        }
        let mut partie = Partie::new(sauvee.grille, sauvee.origine);
        partie.erreurs = sauvee.erreurs;
        partie.indices = sauvee.indices;
        partie.ecoule = Duration::from_secs(sauvee.secondes);
        partie.reprise = None;
        partie.notes = sauvee.notes;
        Some(partie)
    }

    // La ligne des statistiques pour cette partie. Rien tant qu'elle n'est pas finie, ni pour la
    // grille d'entraînement.
    fn vers_statistique(&self) -> Option<PartieJouee> {
        let (fin, Origine::Niveau(niveau, taille)) = (self.fin?, self.origine) else {
            return None;
        };
        let resultat = match fin {
            Fin::Victoire => Resultat::Victoire,
            Fin::Defaite => Resultat::Defaite,
            Fin::Abandon => Resultat::Abandon,
        };
        Some(PartieJouee {
            horodatage: maintenant(),
            niveau,
            taille,
            resultat,
            duree_secondes: self.duree().as_secs(),
            erreurs: self.erreurs,
            indices: self.indices,
            score: self.score.unwrap_or(0),
        })
    }

    // Ce qui est enregistré pour pouvoir reprendre la partie
    fn vers_sauvegarde(&self) -> PartieSauvee {
        PartieSauvee {
            grille: self.grille.clone(),
            origine: self.origine,
            erreurs: self.erreurs,
            indices: self.indices,
            secondes: self.duree().as_secs(),
            notes: self.notes.clone(),
        }
    }

    fn duree(&self) -> Duration {
        self.ecoule + self.reprise.map_or(Duration::ZERO, |debut| debut.elapsed())
    }

    fn arreter_chrono(&mut self) {
        if let Some(debut) = self.reprise.take() {
            self.ecoule += debut.elapsed();
        }
    }

    fn demarrer_chrono(&mut self) {
        if self.fin.is_none() && self.reprise.is_none() {
            self.reprise = Some(Instant::now());
        }
    }

    // Termine la partie. Une partie déjà terminée garde sa première fin.
    // Une victoire sur une grille générée donne un score ; l'entraînement n'en donne pas.
    fn terminer(&mut self, fin: Fin) {
        if self.fin.is_none() {
            self.fin = Some(fin);
            self.modifiee = true;
            match fin {
                Fin::Victoire => self.sons.push(Son::Victoire),
                Fin::Defaite => self.sons.push(Son::Defaite),
                Fin::Abandon => {}
            }
            self.arreter_chrono();
            if let (Fin::Victoire, Origine::Niveau(niveau, taille)) = (fin, self.origine) {
                self.score = Some(calculer_score(
                    niveau,
                    taille,
                    self.duree(),
                    self.erreurs,
                    self.indices,
                ));
            }
        }
    }

    // Inscrit le score au classement, avec le nom tapé. Sans effet s'il n'y a pas de score
    // ou s'il est déjà enregistré.
    fn enregistrer_score(&mut self, classement: &mut Classement) {
        let (Some(score), Origine::Niveau(niveau, taille)) = (self.score, self.origine) else {
            return;
        };
        if self.rang.is_some() {
            return;
        }
        let nom = nettoyer_nom(&self.nom_joueur);
        if nom.is_empty() {
            self.message = String::from("Entrez un nom pour enregistrer votre score");
            return;
        }

        let entree = Entree {
            nom: nom.clone(),
            niveau,
            taille,
            score,
            duree_secondes: self.duree().as_secs(),
            horodatage: maintenant(),
        };
        match classement.ajouter(entree) {
            Ok(rang) => {
                self.rang = Some(rang);
                self.nom_joueur = nom;
                self.message.clear();
            }
            Err(erreur) => self.message = erreur,
        }
    }

    // true si le joueur a posé un chiffre qui n'est pas celui de la solution
    fn a_des_chiffres_faux(&self) -> bool {
        let cote = self.grille.taille().cote();
        (0..cote * cote).any(|i| {
            let (ligne, col) = (i / cote, i % cote);
            let valeur = self.grille.valeur(ligne, col);
            valeur != 0
                && !self.grille.est_fixe(ligne, col)
                && !self.grille.est_correct(ligne, col, valeur)
        })
    }

    // Place un chiffre dans la case choisie (0 pour effacer) et compte les erreurs
    fn jouer_chiffre(&mut self, valeur: u8) {
        if self.fin.is_some() {
            return;
        }
        let Some((ligne, col)) = self.selection else {
            self.message = String::from("Cliquez d'abord sur une case");
            return;
        };
        if self.grille.est_fixe(ligne, col) {
            self.message = String::from("Cette case fait partie de la grille de départ");
            return;
        }
        self.modifiee = true;
        let avant = self.grille.valeur(ligne, col);
        // En mode notes, les chiffres s'écrivent en notes. Seul le 0 sur une case remplie
        // garde son rôle : effacer le chiffre.
        if self.mode_notes && !(valeur == 0 && avant != 0) {
            self.jouer_note(ligne, col, valeur);
            return;
        }
        if valeur == 0 {
            if avant != 0 {
                self.grille.effacer(ligne, col);
                self.noter_coup(ligne, col, avant, 0);
            }
            self.message.clear();
            return;
        }
        let maximum = self.grille.taille().cote();
        if valeur as usize > maximum {
            self.message = format!("Cette grille va de 1 à {}", maximum);
            return;
        }

        let correct = self.grille.est_correct(ligne, col, valeur);
        let place = self.grille.placer(ligne, col, valeur);
        if place {
            self.noter_coup(ligne, col, avant, valeur);
        }
        if correct {
            if place || self.grille.valeur(ligne, col) == valeur {
                self.message.clear();
                if place {
                    self.sons.push(Son::Bon);
                }
            } else {
                // Le bon chiffre est refusé : un chiffre faux posé plus tôt le bloque
                self.message =
                    String::from("Un chiffre rouge de la même ligne, colonne ou bloc vous bloque : effacez-le");
            }
        } else {
            self.erreurs += 1;
            self.sons.push(Son::Erreur);
            let raison = if place {
                "Ce n'est pas le bon chiffre"
            } else {
                "Coup refusé : il brise une règle"
            };
            self.message = format!("{} (erreur {}/{})", raison, self.erreurs, MAX_ERREURS);
        }

        if self.grille.est_terminee() {
            self.terminer(Fin::Victoire);
        } else if self.erreurs >= MAX_ERREURS {
            self.terminer(Fin::Defaite);
        }
    }

    fn demander_indice(&mut self) {
        if self.fin.is_some() {
            return;
        }
        match self.grille.indice() {
            Some(Indice { ligne, col, valeur, explication, .. }) => {
                self.indices += 1;
                self.modifiee = true;
                self.sons.push(Son::Indice);
                self.noter_coup(ligne, col, 0, valeur);
                self.selection = Some((ligne, col));
                self.message = format!(
                    "Indice : {} en ligne {}, colonne {}. {}",
                    valeur,
                    ligne + 1,
                    col + 1,
                    explication
                );
                if self.grille.est_terminee() {
                    self.terminer(Fin::Victoire);
                }
            }
            None => {
                self.message = if self.a_des_chiffres_faux() {
                    String::from("Pas d'indice possible : effacez d'abord les chiffres rouges")
                } else {
                    String::from("Pas d'indice possible")
                };
            }
        }
    }

    // Remplit la grille avec la solution. La partie compte alors comme abandonnée,
    // sauf si elle était déjà perdue.
    fn afficher_solution(&mut self) {
        if self.grille.remplir_solution() {
            self.solution_affichee = true;
            self.message = String::from("Voici la solution");
            self.terminer(Fin::Abandon);
        } else {
            self.message = String::from("Cette grille n'a pas de solution");
        }
    }

    // L'indice d'une case dans `notes`
    fn position(&self, ligne: usize, col: usize) -> usize {
        ligne * self.grille.taille().cote() + col
    }

    // Écrit ou retire une note dans la case choisie (0 : efface toutes ses notes).
    // Les notes ne comptent pas d'erreur : ce ne sont que des brouillons.
    fn jouer_note(&mut self, ligne: usize, col: usize, valeur: u8) {
        if self.grille.valeur(ligne, col) != 0 {
            self.message = String::from("Cette case contient déjà un chiffre : effacez-la pour y écrire des notes");
            return;
        }
        let maximum = self.grille.taille().cote();
        if valeur as usize > maximum {
            self.message = format!("Cette grille va de 1 à {}", maximum);
            return;
        }
        let position = self.position(ligne, col);
        if valeur == 0 {
            self.notes[position] = 0;
        } else {
            // Un même chiffre écrit deux fois s'efface : le bit change à chaque fois
            self.notes[position] ^= 1 << (valeur - 1);
        }
        self.message.clear();
    }

    // Écrit dans chaque case vide tous les chiffres que les règles y permettent.
    // Les notes déjà écrites sont remplacées.
    fn remplir_notes(&mut self) {
        if self.fin.is_some() {
            return;
        }
        self.valider_tampon();
        let cote = self.grille.taille().cote();
        for ligne in 0..cote {
            for col in 0..cote {
                let candidats = if self.grille.est_fixe(ligne, col) {
                    0
                } else {
                    self.grille.candidats(ligne, col)
                };
                let position = self.position(ligne, col);
                self.notes[position] = candidats;
            }
        }
        self.modifiee = true;
        self.message = String::from("Notes remplies avec les chiffres que les règles permettent");
    }

    // Passe du mode « chiffres » au mode « notes », et inversement
    fn basculer_mode_notes(&mut self) {
        self.valider_tampon();
        self.mode_notes = !self.mode_notes;
    }

    // Retient un changement de case pour pouvoir l'annuler. Poser un chiffre efface les notes
    // de la case : elles sont gardées dans le coup pour revenir si on l'annule.
    // Un nouveau coup coupe la branche des coups annulés : on ne peut plus les refaire.
    fn noter_coup(&mut self, ligne: usize, col: usize, avant: u8, apres: u8) {
        let position = self.position(ligne, col);
        let notes = std::mem::take(&mut self.notes[position]);
        self.historique.push(Coup { ligne, col, avant, apres, notes });
        self.annules.clear();
    }

    // Met une case à la valeur voulue (0 pour la vider)
    fn fixer_case(&mut self, ligne: usize, col: usize, valeur: u8) {
        if valeur == 0 {
            self.grille.effacer(ligne, col);
        } else {
            self.grille.placer(ligne, col, valeur);
        }
    }

    // Annule le dernier changement. Les erreurs et les indices déjà comptés ne sont pas rendus :
    // sans cela, on pourrait tout essayer sans rien perdre.
    fn annuler(&mut self) {
        if self.fin.is_some() {
            return;
        }
        self.valider_tampon();
        let Some(coup) = self.historique.pop() else {
            self.message = String::from("Rien à annuler");
            return;
        };
        // La grille revient exactement dans l'état d'avant ce coup : la valeur est permise
        self.fixer_case(coup.ligne, coup.col, coup.avant);
        let position = self.position(coup.ligne, coup.col);
        self.notes[position] = coup.notes;
        self.annules.push(coup);
        self.selection = Some((coup.ligne, coup.col));
        self.modifiee = true;
        self.message = format!("Coup annulé (ligne {}, colonne {})", coup.ligne + 1, coup.col + 1);
    }

    // Refait le dernier changement annulé
    fn refaire(&mut self) {
        if self.fin.is_some() {
            return;
        }
        self.valider_tampon();
        let Some(coup) = self.annules.pop() else {
            self.message = String::from("Rien à refaire");
            return;
        };
        self.fixer_case(coup.ligne, coup.col, coup.apres);
        if coup.apres != 0 {
            let position = self.position(coup.ligne, coup.col);
            self.notes[position] = 0;
        }
        self.historique.push(coup);
        self.selection = Some((coup.ligne, coup.col));
        self.modifiee = true;
        self.message = format!("Coup refait (ligne {}, colonne {})", coup.ligne + 1, coup.col + 1);
    }

    // Joue le premier chiffre en attente (10 attend un second chiffre : le 1 seul est joué)
    fn valider_tampon(&mut self) {
        if let Some((chiffre, _)) = self.tampon.take() {
            self.jouer_chiffre(chiffre);
        }
    }

    // Déplace la sélection d'une case vers le bas (descend) et vers la droite (droite),
    // sans sortir de la grille. Sans sélection, elle commence en haut à gauche.
    fn deplacer(&mut self, descend: i32, droite: i32) {
        let dernier = self.grille.taille().cote() as i32 - 1;
        self.selection = match self.selection {
            Some((ligne, col)) => Some((
                (ligne as i32 + descend).clamp(0, dernier) as usize,
                (col as i32 + droite).clamp(0, dernier) as usize,
            )),
            None => Some((0, 0)),
        };
    }

    // Lit le clavier : joue les chiffres tapés et déplace la sélection avec les flèches.
    // Les touches sont traitées dans l'ordre où elles ont été pressées.
    fn lire_clavier(&mut self, ui: &egui::Ui) {
        let maximum = self.grille.taille().cote() as u8;
        let (touches, maintenant) = ui.input(|entree| {
            let touches: Vec<(Key, egui::Modifiers)> = entree
                .events
                .iter()
                .filter_map(|evenement| match evenement {
                    egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. } => {
                        Some((*key, *modifiers))
                    }
                    _ => None,
                })
                .collect();
            (touches, entree.time)
        });

        // Un premier chiffre qui a attendu assez longtemps est joué tel quel
        if let Some((_, depuis)) = self.tampon {
            if maintenant - depuis > DELAI_SAISIE {
                self.valider_tampon();
            }
        }

        for (touche, modificateurs) in touches {
            // Ctrl+Z annule, Ctrl+Y ou Ctrl+Maj+Z refait
            if modificateurs.command {
                match touche {
                    Key::Z if modificateurs.shift => self.refaire(),
                    Key::Z => self.annuler(),
                    Key::Y => self.refaire(),
                    _ => {}
                }
                continue;
            }

            if let Some(&(_, chiffre)) = TOUCHES_CHIFFRES.iter().find(|(autre, _)| *autre == touche) {
                let en_attente = self.tampon.map(|(premier, _)| premier);
                let (a_jouer, tampon) = saisir_chiffre(en_attente, chiffre, maximum);
                self.tampon = tampon.map(|premier| (premier, maintenant));
                if let Some(nombre) = a_jouer {
                    self.jouer_chiffre(nombre);
                }
                continue;
            }

            // Toute autre touche valide d'abord le chiffre en attente
            self.valider_tampon();
            match touche {
                Key::Backspace | Key::Delete => self.jouer_chiffre(0),
                Key::N => self.basculer_mode_notes(),
                Key::ArrowUp => self.deplacer(-1, 0),
                Key::ArrowDown => self.deplacer(1, 0),
                Key::ArrowLeft => self.deplacer(0, -1),
                Key::ArrowRight => self.deplacer(0, 1),
                _ => {}
            }
        }

        if let Some((premier, _)) = self.tampon {
            self.message = format!("{}... tapez le chiffre suivant, ou patientez", premier);
        }
    }

    // Dessine la grille dans un carré de `cote_px` pixels de côté et sélectionne la case
    // sur laquelle on clique
    fn dessiner_grille(&mut self, ui: &mut egui::Ui, cote_px: f32, reglages: &Reglages) {
        let taille = self.grille.taille();
        let nombre = taille.cote();
        let (grille, notes) = (&self.grille, &self.notes);
        let clic = dessiner_plateau(ui, cote_px, taille, self.selection, reglages, |ligne, col| {
            let valeur = grille.valeur(ligne, col);
            // Un chiffre du joueur qui n'est pas celui de la solution est signalé comme faux
            let genre = if grille.est_fixe(ligne, col) {
                GenreChiffre::Depart
            } else if grille.est_correct(ligne, col, valeur) {
                GenreChiffre::Joueur
            } else {
                GenreChiffre::Faux
            };
            ContenuCase { valeur, genre, notes: notes[ligne * nombre + col] }
        });
        if let Some(case) = clic {
            // Le chiffre en attente est joué dans l'ancienne case avant de changer de case
            self.valider_tampon();
            self.selection = Some(case);
        }
    }

    // Les boutons de saisie et d'aide, affichés tant que la partie n'est pas finie
    fn dessiner_commandes(&mut self, ui: &mut egui::Ui) {
        let mut chiffre_demande = None;
        let maximum = self.grille.taille().cote() as u8;
        ui.horizontal_wrapped(|ui| {
            for valeur in 1..=maximum {
                let bouton = egui::Button::new(valeur.to_string()).min_size(Vec2::splat(32.0));
                if ui.add(bouton).clicked() {
                    chiffre_demande = Some(valeur);
                }
            }
            let effacer = egui::Button::new("Effacer").min_size(Vec2::new(60.0, 32.0));
            if ui.add(effacer).clicked() {
                chiffre_demande = Some(0);
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Indice").clicked() {
                self.demander_indice();
            }
            if ui.button("Solution").clicked() {
                self.afficher_solution();
            }
            ui.add_space(12.0);
            let annulable = !self.historique.is_empty();
            if ui.add_enabled(annulable, egui::Button::new("Annuler")).clicked() {
                self.annuler();
            }
            let refaisable = !self.annules.is_empty();
            if ui.add_enabled(refaisable, egui::Button::new("Refaire")).clicked() {
                self.refaire();
            }
        });
        ui.horizontal(|ui| {
            // Le bouton reste enfoncé tant que le mode notes est actif
            if ui.selectable_label(self.mode_notes, "Notes (N)").clicked() {
                self.basculer_mode_notes();
            }
            if ui.button("Remplir les notes").clicked() {
                self.remplir_notes();
            }
        });
        if let Some(valeur) = chiffre_demande {
            self.jouer_chiffre(valeur);
        }
    }

    // L'écran de fin : résultat, statistiques et choix de la suite
    fn dessiner_fin(
        &mut self,
        ui: &mut egui::Ui,
        fin: Fin,
        classement: &mut Classement,
        palette: &Palette,
    ) -> Option<Action> {
        let mut action = None;
        let (titre, couleur) = match fin {
            Fin::Victoire => (String::from("Bravo, grille terminée !"), palette.vert),
            Fin::Defaite => (format!("Perdu : {} erreurs", MAX_ERREURS), palette.rouge),
            Fin::Abandon => (String::from("Partie abandonnée"), ui.visuals().text_color()),
        };
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(titre).size(26.0).strong().color(couleur));
            ui.label(format!(
                "Temps {}  ·  Erreurs {}/{}  ·  Indices {}",
                formater_duree(self.duree()),
                self.erreurs,
                MAX_ERREURS,
                self.indices
            ));

            if let Some(score) = self.score {
                ui.label(RichText::new(format!("Score : {}", score)).size(22.0).strong());
                let taille = self.grille.taille();
                ui.small(format!(
                    "Points du niveau − 1 par seconde − {} par erreur − {} par indice",
                    penalite_erreur(taille),
                    penalite_indice(taille)
                ));
                match self.rang {
                    Some(rang) => {
                        ui.label(format!("Score enregistré : n°{} du classement", rang));
                    }
                    None => {
                        ui.horizontal(|ui| {
                            ui.label("Nom");
                            let champ = ui.add(
                                TextEdit::singleline(&mut self.nom_joueur)
                                    .char_limit(20)
                                    .desired_width(140.0),
                            );
                            // Entrée dans le champ valide aussi
                            let entree_tapee =
                                champ.lost_focus() && ui.input(|entree| entree.key_pressed(Key::Enter));
                            if ui.button("Enregistrer").clicked() || entree_tapee {
                                self.enregistrer_score(classement);
                            }
                        });
                    }
                }
            } else if fin == Fin::Victoire {
                ui.small("Pas de score en mode entraînement");
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Rejouer").clicked() {
                    action = Some(Action::Rejouer);
                }
                if fin == Fin::Defaite && !self.solution_affichee && ui.button("Voir la solution").clicked() {
                    self.afficher_solution();
                }
                if ui.button("Classement").clicked() {
                    action = Some(Action::Classement);
                }
                if ui.button("Menu").clicked() {
                    action = Some(Action::Menu);
                }
            });
        });
        action
    }

    // Dessine toute la partie. Renvoie une action quand le joueur demande autre chose que jouer.
    fn afficher(
        &mut self,
        ui: &mut egui::Ui,
        classement: &mut Classement,
        reglages: &Reglages,
    ) -> Option<Action> {
        let mut action = None;
        let palette = Palette::pour(ui.visuals().dark_mode, reglages.daltonien);

        if self.fin.is_none() {
            // Le chrono avance : on redessine deux fois par seconde pour l'afficher
            // Un premier chiffre en attente se joue seul au bout d'un instant : il faut redessiner
            // à ce moment-là, même si rien d'autre ne bouge
            let attente = if self.tampon.is_some() { 200 } else { 500 };
            ui.ctx().request_repaint_after(Duration::from_millis(attente));
            self.lire_clavier(ui);
        }

        ui.horizontal(|ui| {
            if ui.button("Menu").clicked() {
                action = Some(Action::Menu);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(format!("Temps {}", formater_duree(self.duree())));
                ui.label(format!("Erreurs {}/{}", self.erreurs, MAX_ERREURS));
            });
        });
        ui.vertical_centered(|ui| ui.heading(self.grille.nom.as_str()));
        ui.add_space(6.0);

        // La grille est carrée : elle prend la place disponible en laissant de quoi
        // afficher les commandes en dessous
        let cote = ui
            .available_width()
            .min(ui.available_height() - 235.0)
            .max(180.0);
        ui.vertical_centered(|ui| self.dessiner_grille(ui, cote, reglages));
        ui.add_space(8.0);

        match self.fin {
            Some(fin) => {
                if let Some(demande) = self.dessiner_fin(ui, fin, classement, &palette) {
                    action = Some(demande);
                }
            }
            None => self.dessiner_commandes(ui),
        }

        ui.add_space(8.0);
        if !self.message.is_empty() {
            ui.label(self.message.as_str());
        } else if self.fin.is_none() {
            let aide = if self.mode_notes {
                "Mode notes : un nombre écrit ou retire une note. N revient aux chiffres."
            } else {
                "Clic ou flèches pour choisir une case, puis un nombre (0 ou Suppr pour effacer). N : notes. Ctrl+Z annule."
            };
            ui.label(aide);
        }
        action
    }
}

enum Ecran {
    Menu,
    Partie,
    Classement,
    Statistiques,
    Resolution,
    Reglages,
}

struct Application {
    ecran: Ecran,
    partie: Option<Partie>,
    // La nouvelle grille arrive par ce canal, générée dans un autre thread
    generation: Option<(Receiver<Grille>, Origine)>,
    erreur: String,
    classement: Classement,
    // La taille de grille choisie dans le menu pour les nouvelles parties
    taille_choisie: Taille,
    // La taille et le niveau affichés dans le classement (niveau None = tous)
    taille_classement: Taille,
    filtre: Option<Niveau>,
    // Le dernier nom utilisé : il est proposé d'office pour les parties suivantes
    dernier_nom: String,
    // Le fichier où la partie en cours est enregistrée
    sauvegarde: Sauvegarde,
    // Toutes les parties terminées, et la taille et le niveau affichés dans l'écran des statistiques
    statistiques: Statistiques,
    taille_stats: Taille,
    filtre_stats: Option<Niveau>,
    // La grille saisie dans l'écran « Résoudre une grille »
    resolveur: Resolveur,
    // Les réglages du joueur (thème, couleurs, sons, mise en valeur)
    reglages: Reglages,
}

impl Application {
    fn new() -> Application {
        let classement = Classement::charger();
        let dernier_nom = classement.dernier_nom().map(str::to_string).unwrap_or_default();
        let sauvegarde = Sauvegarde::par_defaut();
        // Une partie laissée en cours à la dernière fermeture est proposée dans le menu
        let partie = sauvegarde.charger().and_then(Partie::restaurer).map(|mut partie| {
            partie.nom_joueur = dernier_nom.clone();
            partie
        });
        Application {
            ecran: Ecran::Menu,
            partie,
            generation: None,
            erreur: String::new(),
            classement,
            taille_choisie: Taille::Neuf,
            taille_classement: Taille::Neuf,
            filtre: None,
            dernier_nom,
            sauvegarde,
            statistiques: Statistiques::charger(),
            taille_stats: Taille::Neuf,
            filtre_stats: None,
            resolveur: Resolveur::new(),
            reglages: Reglages::charger(),
        }
    }

    // Démarre une partie. Une grille générée est calculée dans un autre thread,
    // pour ne pas figer la fenêtre.
    fn lancer_partie(&mut self, origine: Origine, contexte: &egui::Context) {
        if self.generation.is_some() {
            return;
        }
        self.erreur.clear();
        match origine {
            Origine::Fixe => match Grille::creer(origine) {
                Ok(grille) => self.demarrer(grille, origine),
                Err(erreur) => self.erreur = erreur,
            },
            Origine::Niveau(niveau, taille) => {
                let (envoi, reception) = mpsc::channel();
                let contexte = contexte.clone();
                thread::spawn(move || {
                    // Si la fenêtre est déjà fermée, personne ne reçoit : l'erreur d'envoi est sans importance
                    let _ = envoi.send(Grille::generer_niveau(niveau, taille));
                    // Réveille la fenêtre pour qu'elle affiche la grille dès qu'elle est prête
                    contexte.request_repaint();
                });
                self.generation = Some((reception, origine));
            }
        }
    }

    fn demarrer(&mut self, grille: Grille, origine: Origine) {
        let mut partie = Partie::new(grille, origine);
        partie.nom_joueur = self.dernier_nom.clone();
        self.partie = Some(partie);
        self.ecran = Ecran::Partie;
        // La nouvelle partie remplace tout de suite l'ancienne sauvegarde
        self.sauvegarder_partie();
    }

    // Enregistre la partie en cours, ou efface la sauvegarde si elle est terminée
    fn sauvegarder_partie(&mut self) {
        let Some(partie) = self.partie.as_mut() else {
            return;
        };
        partie.modifiee = false;
        if partie.fin.is_some() {
            self.sauvegarde.supprimer();
            // La partie finie entre dans les statistiques, une seule fois
            if !partie.comptee {
                partie.comptee = true;
                if let Some(jouee) = partie.vers_statistique() {
                    if let Err(erreur) = self.statistiques.ajouter(jouee) {
                        partie.message = erreur;
                    }
                }
            }
        } else if let Err(erreur) = self.sauvegarde.enregistrer(&partie.vers_sauvegarde()) {
            partie.message = erreur;
        }
    }

    // Si la grille générée est arrivée, la partie commence
    fn recevoir_nouvelle_grille(&mut self) {
        let recue = self.generation.as_ref().and_then(|(reception, origine)| {
            reception.try_recv().ok().map(|grille| (grille, *origine))
        });
        if let Some((grille, origine)) = recue {
            self.generation = None;
            self.demarrer(grille, origine);
        }
    }

    fn dessiner_menu(&mut self, ui: &mut egui::Ui) {
        let reprise_possible = self.partie.as_ref().is_some_and(|partie| partie.fin.is_none());
        let occupe = self.generation.is_some();
        let mut choix = None;
        let palette = Palette::pour(ui.visuals().dark_mode, self.reglages.daltonien);

        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(RichText::new("Sudoku").size(44.0).strong());
            ui.label("Chaque ligne, colonne et bloc doit contenir chaque nombre une seule fois.");
            ui.label(format!("{} erreurs et la partie est perdue.", MAX_ERREURS));
            ui.add_space(24.0);

            if occupe {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Génération de la grille...");
                });
                ui.small("Une grande grille difficile peut prendre un moment.");
                ui.add_space(8.0);
            }

            ui.add_enabled_ui(!occupe, |ui| {
                if reprise_possible && gros_bouton(ui, "Reprendre la partie") {
                    choix = Some(ChoixMenu::Reprendre);
                }
                if let Some(partie) = self.partie.as_ref().filter(|partie| partie.fin.is_none()) {
                    ui.small(format!("{} - {}", partie.grille.nom, formater_duree(partie.duree())));
                }
                ui.add_space(8.0);
                ui.label("Nouvelle partie");
                egui::ComboBox::from_label("Taille de la grille")
                    .selected_text(self.taille_choisie.libelle())
                    .show_ui(ui, |ui| {
                        for taille in Taille::TOUTES {
                            ui.selectable_value(&mut self.taille_choisie, taille, taille.libelle());
                        }
                    });
                ui.add_space(4.0);
                for niveau in Niveau::TOUS {
                    if gros_bouton(ui, niveau.libelle()) {
                        choix = Some(ChoixMenu::Nouvelle(Origine::Niveau(niveau, self.taille_choisie)));
                    }
                }
                ui.add_space(8.0);
                if gros_bouton(ui, "Grille d'entraînement") {
                    choix = Some(ChoixMenu::Nouvelle(Origine::Fixe));
                }
                ui.add_space(8.0);
                if gros_bouton(ui, "Résoudre une grille") {
                    choix = Some(ChoixMenu::Resolution);
                }
                ui.add_space(8.0);
                if gros_bouton(ui, "Classement") {
                    choix = Some(ChoixMenu::Classement);
                }
                ui.add_space(8.0);
                if gros_bouton(ui, "Statistiques") {
                    choix = Some(ChoixMenu::Statistiques);
                }
                ui.add_space(8.0);
                if gros_bouton(ui, "Réglages") {
                    choix = Some(ChoixMenu::Reglages);
                }
                ui.add_space(16.0);
                if gros_bouton(ui, "Quitter") {
                    choix = Some(ChoixMenu::Quitter);
                }
            });

            if !self.erreur.is_empty() {
                ui.add_space(8.0);
                ui.colored_label(palette.rouge, self.erreur.as_str());
            }
        });

        match choix {
            Some(ChoixMenu::Reprendre) => {
                if let Some(partie) = self.partie.as_mut() {
                    partie.demarrer_chrono();
                }
                self.ecran = Ecran::Partie;
            }
            Some(ChoixMenu::Nouvelle(origine)) => self.lancer_partie(origine, ui.ctx()),
            Some(ChoixMenu::Classement) => self.ecran = Ecran::Classement,
            Some(ChoixMenu::Statistiques) => self.ecran = Ecran::Statistiques,
            Some(ChoixMenu::Resolution) => self.ecran = Ecran::Resolution,
            Some(ChoixMenu::Reglages) => {
                self.erreur.clear();
                self.ecran = Ecran::Reglages;
            }
            Some(ChoixMenu::Quitter) => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
            None => {}
        }
    }

    fn dessiner_partie(&mut self, ui: &mut egui::Ui) {
        let Some(partie) = self.partie.as_mut() else {
            self.ecran = Ecran::Menu;
            return;
        };
        let action = partie.afficher(ui, &mut self.classement, &self.reglages);
        // Les sons de la partie sont joués seulement si le joueur les a activés
        let bruits = std::mem::take(&mut partie.sons);
        if self.reglages.sons {
            for bruit in bruits {
                sons::jouer(bruit);
            }
        }
        // On enregistre à chaque coup, et quand on quitte la partie (le chrono est alors à jour)
        let a_sauvegarder =
            partie.modifiee || matches!(action, Some(Action::Menu | Action::Classement));

        // Une fois le score enregistré, ce nom est proposé pour les parties suivantes
        if partie.rang.is_some() {
            self.dernier_nom = partie.nom_joueur.clone();
        }

        match action {
            Some(Action::Menu) => {
                partie.arreter_chrono();
                self.ecran = Ecran::Menu;
            }
            Some(Action::Classement) => {
                partie.arreter_chrono();
                // On ouvre le classement de la taille qu'on vient de jouer
                self.taille_classement = partie.grille.taille();
                self.ecran = Ecran::Classement;
            }
            Some(Action::Rejouer) => {
                let origine = partie.origine;
                self.lancer_partie(origine, ui.ctx());
            }
            None => {}
        }

        if a_sauvegarder {
            self.sauvegarder_partie();
        }

        if self.generation.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Génération de la grille...");
            });
        }
    }

    fn dessiner_classement(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            ui.heading(format!("Classement {}", self.taille_classement.libelle()));
        });
        ui.add_space(8.0);

        // Les scores de tailles différentes ne se comparent pas : chaque taille a son classement
        ui.horizontal_wrapped(|ui| {
            ui.label("Taille :");
            for taille in Taille::TOUTES {
                ui.selectable_value(&mut self.taille_classement, taille, taille.libelle());
            }
        });
        ui.horizontal(|ui| {
            ui.label("Niveau :");
            ui.selectable_value(&mut self.filtre, None, "Tous");
            for niveau in Niveau::TOUS {
                ui.selectable_value(&mut self.filtre, Some(niveau), niveau.libelle());
            }
        });
        ui.add_space(8.0);

        let meilleurs = self
            .classement
            .meilleurs(self.taille_classement, self.filtre, LIGNES_CLASSEMENT);
        if meilleurs.is_empty() {
            ui.label(format!(
                "Aucun score en {} pour l'instant : gagnez une partie !",
                self.taille_classement.libelle()
            ));
        } else {
            egui::Grid::new("tableau_classement")
                .striped(true)
                .num_columns(5)
                .spacing([18.0, 6.0])
                .show(ui, |ui| {
                    for titre in ["N°", "Joueur", "Niveau", "Score", "Temps"] {
                        ui.label(RichText::new(titre).strong());
                    }
                    ui.end_row();
                    for (rang, entree) in meilleurs.iter().enumerate() {
                        ui.label((rang + 1).to_string());
                        ui.label(entree.nom.as_str());
                        ui.label(entree.niveau.libelle());
                        ui.label(entree.score.to_string());
                        ui.label(formater_duree(Duration::from_secs(entree.duree_secondes)));
                        ui.end_row();
                    }
                });
        }

        ui.add_space(16.0);
        if ui.button("Retour au menu").clicked() {
            self.ecran = Ecran::Menu;
        }
    }

    // Lit le clavier dans l'écran de résolution : chiffres, effacement, flèches, et Ctrl+V
    // qui colle une grille entière
    fn lire_clavier_resolution(&mut self, ui: &egui::Ui) {
        let evenements = ui.input(|entree| entree.events.clone());
        for evenement in evenements {
            match evenement {
                egui::Event::Paste(texte) => {
                    if let Err(erreur) = self.resolveur.coller(&texte) {
                        self.resolveur.message = erreur;
                    }
                }
                egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. }
                    if !modifiers.command =>
                {
                    if let Some(&(_, chiffre)) = TOUCHES_CHIFFRES.iter().find(|(autre, _)| *autre == key) {
                        self.resolveur.saisir(chiffre);
                        continue;
                    }
                    match key {
                        Key::Backspace | Key::Delete => self.resolveur.effacer_la_case(),
                        Key::ArrowUp => self.resolveur.deplacer(-1, 0),
                        Key::ArrowDown => self.resolveur.deplacer(1, 0),
                        Key::ArrowLeft => self.resolveur.deplacer(0, -1),
                        Key::ArrowRight => self.resolveur.deplacer(0, 1),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    // L'écran « Résoudre une grille » : on saisit les chiffres d'une grille, le solveur la remplit
    fn dessiner_resolution(&mut self, ui: &mut egui::Ui) {
        self.lire_clavier_resolution(ui);
        let palette = Palette::pour(ui.visuals().dark_mode, self.reglages.daltonien);

        if ui.button("Menu").clicked() {
            self.ecran = Ecran::Menu;
        }
        ui.vertical_centered(|ui| ui.heading("Résoudre une grille"));
        ui.add_space(6.0);

        // La grille est carrée : elle prend la place disponible en laissant de quoi
        // afficher les commandes en dessous
        let cote = ui
            .available_width()
            .min(ui.available_height() - 215.0)
            .max(180.0);
        let conflits = self.resolveur.conflits();
        let (resolveur, reglages) = (&self.resolveur, &self.reglages);
        let clic = ui
            .vertical_centered(|ui| {
                let (taille, selection) = (resolveur.taille(), resolveur.selection);
                dessiner_plateau(ui, cote, taille, selection, reglages, |ligne, col| {
                    // Les chiffres saisis sont comme ceux d'une grille de départ, ceux du solveur
                    // comme ceux du joueur, et ceux qui se contredisent sont signalés comme faux
                    let genre = if conflits[ligne * 9 + col] {
                        GenreChiffre::Faux
                    } else if resolveur.est_saisie(ligne, col) {
                        GenreChiffre::Depart
                    } else {
                        GenreChiffre::Joueur
                    };
                    ContenuCase { valeur: resolveur.valeur(ligne, col), genre, notes: 0 }
                })
            })
            .inner;
        if let Some(case) = clic {
            self.resolveur.selection = Some(case);
        }
        ui.add_space(8.0);

        let mut chiffre_demande = None;
        let mut effacer = false;
        ui.horizontal_wrapped(|ui| {
            for valeur in 1..=9u8 {
                let bouton = egui::Button::new(valeur.to_string()).min_size(Vec2::splat(32.0));
                if ui.add(bouton).clicked() {
                    chiffre_demande = Some(valeur);
                }
            }
            let bouton = egui::Button::new("Effacer").min_size(Vec2::new(60.0, 32.0));
            effacer = ui.add(bouton).clicked();
        });
        ui.horizontal(|ui| {
            let resoudre = egui::Button::new(RichText::new("Résoudre").strong()).min_size(Vec2::new(90.0, 32.0));
            if ui.add(resoudre).clicked() {
                self.resolveur.resoudre();
            }
            if self.resolveur.est_resolue() && ui.button("Modifier la grille").clicked() {
                self.resolveur.revenir_a_la_saisie();
            }
            if ui.button("Tout effacer").clicked() {
                self.resolveur.tout_effacer();
            }
        });
        if let Some(valeur) = chiffre_demande {
            self.resolveur.saisir(valeur);
        }
        if effacer {
            self.resolveur.effacer_la_case();
        }

        ui.add_space(8.0);
        if self.resolveur.message.is_empty() {
            ui.label(format!(
                "{} chiffres saisis. Cliquez sur une case et tapez les chiffres de votre grille \
                 (0 pour une case vide, Ctrl+V pour coller une grille), puis cliquez sur Résoudre.",
                self.resolveur.nombre_de_chiffres()
            ));
        } else if self.resolveur.est_resolue() {
            ui.colored_label(palette.vert, self.resolveur.message.as_str());
        } else {
            ui.label(self.resolveur.message.as_str());
        }
    }

    // L'écran des réglages : chaque changement est appliqué et enregistré tout de suite
    fn dessiner_reglages(&mut self, ui: &mut egui::Ui) {
        let avant = self.reglages;
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            ui.heading("Réglages");
        });
        ui.add_space(12.0);

        ui.label(RichText::new("Thème").strong());
        ui.horizontal_wrapped(|ui| {
            for theme in Theme::TOUS {
                ui.selectable_value(&mut self.reglages.theme, theme, theme.libelle());
            }
        });
        ui.add_space(10.0);

        ui.label(RichText::new("Couleurs").strong());
        ui.checkbox(
            &mut self.reglages.daltonien,
            "Mode daltonien : bleu et orange à la place du rouge, chiffres faux soulignés",
        );
        // Un aperçu avec les couleurs de la grille
        let palette = Palette::pour(ui.visuals().dark_mode, self.reglages.daltonien);
        egui::Frame::new().fill(palette.fond).inner_margin(8.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Départ").color(palette.chiffre_depart).size(18.0));
                ui.label(RichText::new("Votre chiffre").color(palette.chiffre_joueur).size(18.0));
                ui.label(RichText::new("Chiffre faux").color(palette.chiffre_faux).size(18.0).underline());
            });
        });
        ui.add_space(10.0);

        ui.label(RichText::new("Aide visuelle").strong());
        ui.checkbox(
            &mut self.reglages.surligner_zone,
            "Mettre en valeur la ligne, la colonne et le bloc de la case choisie",
        );
        ui.checkbox(
            &mut self.reglages.surligner_chiffres,
            "Mettre en valeur les chiffres identiques à celui de la case choisie",
        );
        ui.add_space(10.0);

        ui.label(RichText::new("Sons").strong());
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.reglages.sons, "Sons (bon chiffre, erreur, indice, fin de partie)");
        });
        ui.horizontal(|ui| {
            if ui.button("Écouter un bon chiffre").clicked() {
                sons::jouer(Son::Bon);
            }
            if ui.button("Une erreur").clicked() {
                sons::jouer(Son::Erreur);
            }
            if ui.button("Une victoire").clicked() {
                sons::jouer(Son::Victoire);
            }
        });
        ui.small("Les sons ne sont joués que sous Windows.");

        if self.reglages != avant {
            // Le thème de la fenêtre change à l'image suivante
            ui.ctx().request_repaint();
            self.erreur = match self.reglages.enregistrer() {
                Ok(()) => String::new(),
                Err(erreur) => erreur,
            };
        }
        if !self.erreur.is_empty() {
            ui.add_space(8.0);
            ui.colored_label(palette.rouge, self.erreur.as_str());
        }

        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if ui.button("Réglages par défaut").clicked() {
                self.reglages = Reglages::default();
                ui.ctx().request_repaint();
                self.erreur = match self.reglages.enregistrer() {
                    Ok(()) => String::new(),
                    Err(erreur) => erreur,
                };
            }
            if ui.button("Retour au menu").clicked() {
                self.ecran = Ecran::Menu;
            }
        });
    }

    fn dessiner_statistiques(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            ui.heading(format!("Statistiques {}", self.taille_stats.libelle()));
        });
        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            ui.label("Taille :");
            for taille in Taille::TOUTES {
                ui.selectable_value(&mut self.taille_stats, taille, taille.libelle());
            }
        });
        ui.horizontal(|ui| {
            ui.label("Niveau :");
            ui.selectable_value(&mut self.filtre_stats, None, "Tous");
            for niveau in Niveau::TOUS {
                ui.selectable_value(&mut self.filtre_stats, Some(niveau), niveau.libelle());
            }
        });
        ui.add_space(8.0);

        let resume = self.statistiques.resume(self.taille_stats, self.filtre_stats);
        if resume.jouees == 0 {
            ui.label(format!(
                "Aucune partie terminée en {} pour l'instant.",
                self.taille_stats.libelle()
            ));
        } else {
            let temps = |secondes: Option<u64>| {
                secondes.map_or(String::from("-"), |s| formater_duree(Duration::from_secs(s)))
            };
            let pourcentage = resume.taux_de_victoire().unwrap_or(0);
            let lignes = [
                ("Parties jouées", resume.jouees.to_string()),
                ("Victoires", format!("{} ({} %)", resume.victoires, pourcentage)),
                ("Défaites", resume.defaites.to_string()),
                ("Abandons", resume.abandons.to_string()),
                ("Meilleur temps", temps(resume.meilleur_temps)),
                ("Temps moyen d'une victoire", temps(resume.temps_moyen)),
                (
                    "Meilleur score",
                    resume.meilleur_score.map_or(String::from("-"), |score| score.to_string()),
                ),
                ("Victoires d'affilée", resume.serie_en_cours.to_string()),
                ("Meilleure série", resume.meilleure_serie.to_string()),
            ];
            egui::Grid::new("tableau_statistiques")
                .striped(true)
                .num_columns(2)
                .spacing([24.0, 6.0])
                .show(ui, |ui| {
                    for (nom, valeur) in lignes {
                        ui.label(nom);
                        ui.label(RichText::new(valeur).strong());
                        ui.end_row();
                    }
                });
        }
        ui.add_space(8.0);
        ui.small("Les parties terminées sont comptées ; la grille d'entraînement ne l'est pas.");

        ui.add_space(16.0);
        if ui.button("Retour au menu").clicked() {
            self.ecran = Ecran::Menu;
        }
    }
}

// Ce qu'une case affiche : son chiffre (0 = vide) avec sa couleur, ou ses notes si elle est vide
struct ContenuCase {
    valeur: u8,
    genre: GenreChiffre,
    // Les notes en bits : le bit (v - 1) vaut 1 si la note v est écrite
    notes: u16,
}

// Dessine une grille dans un carré de `cote_px` pixels de côté : le quadrillage, la sélection
// et le contenu de chaque case, donné par `contenu(ligne, col)`.
// Renvoie la case sur laquelle on vient de cliquer, s'il y en a une.
fn dessiner_plateau(
    ui: &mut egui::Ui,
    cote_px: f32,
    taille: Taille,
    selection: Option<(usize, usize)>,
    reglages: &Reglages,
    contenu: impl Fn(usize, usize) -> ContenuCase,
) -> Option<(usize, usize)> {
    let palette = Palette::pour(ui.visuals().dark_mode, reglages.daltonien);
    let nombre = taille.cote();
    let (bloc_h, bloc_l) = (taille.bloc_h(), taille.bloc_l());
    let (zone, reponse) = ui.allocate_exact_size(Vec2::splat(cote_px), Sense::click());
    let case = cote_px / nombre as f32;

    let mut clic = None;
    if reponse.clicked() {
        if let Some(souris) = reponse.interact_pointer_pos() {
            let col = (((souris.x - zone.min.x) / case) as usize).min(nombre - 1);
            let ligne = (((souris.y - zone.min.y) / case) as usize).min(nombre - 1);
            clic = Some((ligne, col));
        }
    }
    // La case cliquée est mise en valeur tout de suite
    let selection = clic.or(selection);

    let peintre = ui.painter_at(zone);
    peintre.rect_filled(zone, 0.0, palette.fond);

    // Les cases qui contiennent le même chiffre que la case choisie sont mises en valeur
    let chiffre_choisi = selection
        .map(|(ligne, col)| contenu(ligne, col).valeur)
        .filter(|&valeur| valeur != 0 && reglages.surligner_chiffres);

    for ligne in 0..nombre {
        for col in 0..nombre {
            let coin = Pos2::new(
                zone.min.x + col as f32 * case,
                zone.min.y + ligne as f32 * case,
            );
            let rectangle = Rect::from_min_size(coin, Vec2::splat(case));
            let ContenuCase { valeur, genre, notes } = contenu(ligne, col);

            let fond = match selection {
                Some(choisie) if choisie == (ligne, col) => Some(palette.case_selectionnee),
                _ if valeur != 0 && Some(valeur) == chiffre_choisi => Some(palette.meme_chiffre),
                Some((l, c))
                    if reglages.surligner_zone
                        && (l == ligne
                            || c == col
                            || (l / bloc_h == ligne / bloc_h && c / bloc_l == col / bloc_l)) =>
                {
                    Some(palette.ligne_colonne_bloc)
                }
                _ => None,
            };
            if let Some(couleur) = fond {
                peintre.rect_filled(rectangle, 0.0, couleur);
            }

            if valeur != 0 {
                // Un nombre à deux chiffres a besoin d'une police un peu plus petite
                let hauteur = if valeur >= 10 { case * 0.45 } else { case * 0.6 };
                let couleur = palette.chiffre(genre);
                peintre.text(
                    rectangle.center(),
                    Align2::CENTER_CENTER,
                    valeur,
                    FontId::proportional(hauteur),
                    couleur,
                );
                // En mode daltonien, un chiffre faux est aussi souligné : la couleur ne suffit pas
                if genre == GenreChiffre::Faux && reglages.daltonien {
                    let y = rectangle.center().y + hauteur * 0.55;
                    let demi = hauteur * if valeur >= 10 { 0.5 } else { 0.3 };
                    peintre.line_segment(
                        [
                            Pos2::new(rectangle.center().x - demi, y),
                            Pos2::new(rectangle.center().x + demi, y),
                        ],
                        Stroke::new(2.0, couleur),
                    );
                }
            } else {
                // Une case vide montre ses notes en petit, rangées en tableau :
                // le chiffre v a toujours la même place dans la case
                let colonnes = (nombre as f32).sqrt().ceil() as usize;
                let lignes_notes = nombre.div_ceil(colonnes);
                let (largeur, hauteur) = (case / colonnes as f32, case / lignes_notes as f32);
                for note in 1..=nombre {
                    if notes & (1 << (note - 1)) == 0 {
                        continue;
                    }
                    let (rang, colonne) = ((note - 1) / colonnes, (note - 1) % colonnes);
                    let centre = Pos2::new(
                        coin.x + (colonne as f32 + 0.5) * largeur,
                        coin.y + (rang as f32 + 0.5) * hauteur,
                    );
                    // Les notes du chiffre choisi ressortent
                    let couleur = if chiffre_choisi == Some(note as u8) { palette.chiffre_joueur } else { palette.note };
                    peintre.text(
                        centre,
                        Align2::CENTER_CENTER,
                        note,
                        FontId::proportional(largeur.min(hauteur) * 0.8),
                        couleur,
                    );
                }
            }
        }
    }

    // Quadrillage : les traits qui séparent les blocs sont plus épais.
    // Les traits verticaux suivent la largeur d'un bloc, les horizontaux sa hauteur.
    for i in 0..=nombre {
        let decalage = i as f32 * case;
        let vertical = if i % bloc_l == 0 {
            Stroke::new(3.0, palette.trait_epais)
        } else {
            Stroke::new(1.0, palette.trait_fin)
        };
        peintre.line_segment(
            [
                Pos2::new(zone.min.x + decalage, zone.min.y),
                Pos2::new(zone.min.x + decalage, zone.max.y),
            ],
            vertical,
        );
        let horizontal = if i % bloc_h == 0 {
            Stroke::new(3.0, palette.trait_epais)
        } else {
            Stroke::new(1.0, palette.trait_fin)
        };
        peintre.line_segment(
            [
                Pos2::new(zone.min.x, zone.min.y + decalage),
                Pos2::new(zone.max.x, zone.min.y + decalage),
            ],
            horizontal,
        );
    }
    clic
}

// Un grand bouton pour le menu
fn gros_bouton(ui: &mut egui::Ui, texte: &str) -> bool {
    ui.add(egui::Button::new(texte).min_size(Vec2::new(240.0, 40.0)))
        .clicked()
}

impl eframe::App for Application {
    fn ui(&mut self, ui: &mut egui::Ui, _cadre: &mut eframe::Frame) {
        // Le thème choisi s'applique dès la première image, et à chaque changement
        let theme = theme_egui(self.reglages.theme);
        if ui.ctx().options(|options| options.theme_preference) != theme {
            ui.ctx().set_theme(theme);
            ui.ctx().request_repaint();
        }
        self.recevoir_nouvelle_grille();

        egui::CentralPanel::default().show(ui, |ui| match self.ecran {
            Ecran::Menu => self.dessiner_menu(ui),
            Ecran::Partie => self.dessiner_partie(ui),
            Ecran::Classement => self.dessiner_classement(ui),
            Ecran::Statistiques => self.dessiner_statistiques(ui),
            Ecran::Resolution => self.dessiner_resolution(ui),
            Ecran::Reglages => self.dessiner_reglages(ui),
        });
    }

    // La fenêtre se ferme : la partie en cours est enregistrée avec son temps exact
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(partie) = self.partie.as_mut() {
            partie.arreter_chrono();
        }
        self.sauvegarder_partie();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn partie_fixe() -> Partie {
        let grille = Grille::creer(Origine::Fixe).unwrap();
        Partie::new(grille, Origine::Fixe)
    }

    // Sélectionne une case puis y joue un chiffre
    fn jouer(partie: &mut Partie, ligne: usize, col: usize, valeur: u8) {
        partie.selection = Some((ligne, col));
        partie.jouer_chiffre(valeur);
    }

    #[test]
    fn test_bon_chiffre_sans_erreur() {
        let mut partie = partie_fixe();
        // La solution met 4 en (0, 2)
        jouer(&mut partie, 0, 2, 4);
        assert_eq!(partie.erreurs, 0);
        assert_eq!(partie.grille.valeur(0, 2), 4);
    }

    #[test]
    fn test_chiffre_faux_mais_valide_est_place_et_compte() {
        let mut partie = partie_fixe();
        // 1 respecte les règles en (0, 2) mais ce n'est pas le bon chiffre
        jouer(&mut partie, 0, 2, 1);
        assert_eq!(partie.erreurs, 1);
        assert_eq!(partie.grille.valeur(0, 2), 1);
        assert!(partie.a_des_chiffres_faux());
    }

    #[test]
    fn test_chiffre_qui_brise_une_regle_est_refuse_et_compte() {
        let mut partie = partie_fixe();
        // Il y a déjà un 5 sur la ligne 0
        jouer(&mut partie, 0, 2, 5);
        assert_eq!(partie.erreurs, 1);
        assert_eq!(partie.grille.valeur(0, 2), 0);
    }

    #[test]
    fn test_case_de_depart_sans_erreur() {
        let mut partie = partie_fixe();
        jouer(&mut partie, 0, 0, 9);
        assert_eq!(partie.erreurs, 0);
        assert_eq!(partie.grille.valeur(0, 0), 5);
    }

    #[test]
    fn test_defaite_apres_trois_erreurs() {
        let mut partie = partie_fixe();
        for _ in 0..MAX_ERREURS {
            jouer(&mut partie, 0, 2, 5);
        }
        assert_eq!(partie.fin, Some(Fin::Defaite));
        // Une fois perdue, la partie ne bouge plus
        jouer(&mut partie, 0, 2, 4);
        assert_eq!(partie.grille.valeur(0, 2), 0);
    }

    #[test]
    fn test_chiffre_faux_bloque_le_bon_chiffre_sans_erreur() {
        let mut partie = partie_fixe();
        // Un 1 faux en (0, 2) empêche de poser le vrai 1 en (0, 7), sur la même ligne
        jouer(&mut partie, 0, 2, 1);
        assert_eq!(partie.erreurs, 1);
        jouer(&mut partie, 0, 7, 1);
        assert_eq!(partie.erreurs, 1);
        assert_eq!(partie.grille.valeur(0, 7), 0);
        assert!(!partie.message.is_empty());
    }

    #[test]
    fn test_victoire() {
        let mut partie = partie_fixe();
        let mut solution = partie.grille.clone();
        assert!(solution.remplir_solution());
        for ligne in 0..9 {
            for col in 0..9 {
                if partie.grille.valeur(ligne, col) == 0 {
                    jouer(&mut partie, ligne, col, solution.valeur(ligne, col));
                }
            }
        }
        assert_eq!(partie.fin, Some(Fin::Victoire));
        assert_eq!(partie.erreurs, 0);
    }

    #[test]
    fn test_abandon_en_affichant_la_solution() {
        let mut partie = partie_fixe();
        partie.afficher_solution();
        assert_eq!(partie.fin, Some(Fin::Abandon));
        assert!(partie.solution_affichee);
        assert!(partie.reprise.is_none());
    }

    #[test]
    fn test_solution_apres_defaite_reste_une_defaite() {
        let mut partie = partie_fixe();
        for _ in 0..MAX_ERREURS {
            jouer(&mut partie, 0, 2, 5);
        }
        partie.afficher_solution();
        assert_eq!(partie.fin, Some(Fin::Defaite));
        assert!(partie.solution_affichee);
    }

    #[test]
    fn test_indice_compte_et_selectionne_la_case() {
        let mut partie = partie_fixe();
        partie.demander_indice();
        assert_eq!(partie.indices, 1);
        assert!(partie.selection.is_some());
    }

    #[test]
    fn test_indice_refuse_avec_un_chiffre_faux() {
        let mut partie = partie_fixe();
        jouer(&mut partie, 0, 2, 1);
        partie.demander_indice();
        assert_eq!(partie.indices, 0);
        assert!(partie.message.contains("rouges"));
    }

    #[test]
    fn test_le_chrono_s_arrete_a_la_fin() {
        let mut partie = partie_fixe();
        partie.afficher_solution();
        let duree = partie.duree();
        thread::sleep(Duration::from_millis(30));
        assert_eq!(partie.duree(), duree);
        // Une partie terminée ne repart pas
        partie.demarrer_chrono();
        assert!(partie.reprise.is_none());
    }

    #[test]
    fn test_pause_et_reprise_du_chrono() {
        let mut partie = partie_fixe();
        partie.arreter_chrono();
        let duree = partie.duree();
        thread::sleep(Duration::from_millis(30));
        assert_eq!(partie.duree(), duree);
        partie.demarrer_chrono();
        assert!(partie.reprise.is_some());
    }

    // Une partie sur la grille fixe, mais étiquetée comme une partie de niveau Moyen
    // pour qu'une victoire donne un score
    fn partie_moyenne() -> Partie {
        let grille = Grille::creer(Origine::Fixe).unwrap();
        Partie::new(grille, Origine::Niveau(Niveau::Moyen, Taille::Neuf))
    }

    // Joue tous les bons chiffres : la partie est gagnée sans erreur ni indice
    fn gagner(partie: &mut Partie) {
        let mut solution = partie.grille.clone();
        assert!(solution.remplir_solution());
        let cote = partie.grille.taille().cote();
        for ligne in 0..cote {
            for col in 0..cote {
                if partie.grille.valeur(ligne, col) == 0 {
                    jouer(partie, ligne, col, solution.valeur(ligne, col));
                }
            }
        }
    }

    #[test]
    fn test_victoire_sur_niveau_donne_un_score() {
        let mut partie = partie_moyenne();
        gagner(&mut partie);
        assert_eq!(partie.fin, Some(Fin::Victoire));
        let score = partie.score.expect("une victoire sur un niveau donne un score");
        // 2000 points moins quelques secondes tout au plus
        assert!(score <= 2000 && score > 1900);
    }

    #[test]
    fn test_le_score_tient_compte_des_erreurs_et_des_indices() {
        let mut partie = partie_moyenne();
        jouer(&mut partie, 0, 2, 5); // erreur : brise une règle
        partie.demander_indice();
        gagner(&mut partie);
        let score = partie.score.unwrap();
        // 2000 - 100 (erreur) - 150 (indice) - quelques secondes
        assert!(score <= 1750 && score > 1650);
    }

    #[test]
    fn test_pas_de_score_a_l_entrainement() {
        let mut partie = partie_fixe();
        gagner(&mut partie);
        assert_eq!(partie.fin, Some(Fin::Victoire));
        assert_eq!(partie.score, None);
    }

    #[test]
    fn test_pas_de_score_en_cas_de_defaite_ou_d_abandon() {
        let mut perdue = partie_moyenne();
        for _ in 0..MAX_ERREURS {
            jouer(&mut perdue, 0, 2, 5);
        }
        assert_eq!(perdue.fin, Some(Fin::Defaite));
        assert_eq!(perdue.score, None);

        let mut abandonnee = partie_moyenne();
        abandonnee.afficher_solution();
        assert_eq!(abandonnee.score, None);
    }

    #[test]
    fn test_enregistrer_le_score() {
        let chemin = crate::classement::chemin_temporaire();
        let mut classement = Classement::depuis_fichier(chemin.clone());
        let mut partie = partie_moyenne();
        gagner(&mut partie);
        partie.nom_joueur = String::from("  Lucas\t ");

        partie.enregistrer_score(&mut classement);
        assert_eq!(partie.rang, Some(1));
        assert_eq!(partie.nom_joueur, "Lucas");
        let meilleurs = classement.meilleurs(Taille::Neuf, None, 10);
        assert_eq!(meilleurs.len(), 1);
        assert_eq!(meilleurs[0].nom, "Lucas");
        assert_eq!(meilleurs[0].niveau, Niveau::Moyen);
        assert_eq!(Some(meilleurs[0].score), partie.score);

        // Un deuxième clic ne crée pas de doublon
        partie.enregistrer_score(&mut classement);
        assert_eq!(classement.meilleurs(Taille::Neuf, None, 10).len(), 1);

        let _ = std::fs::remove_file(&chemin);
        let _ = std::fs::remove_file(chemin.with_extension("tmp"));
    }

    #[test]
    fn test_enregistrer_refuse_un_nom_vide() {
        let chemin = crate::classement::chemin_temporaire();
        let mut classement = Classement::depuis_fichier(chemin.clone());
        let mut partie = partie_moyenne();
        gagner(&mut partie);
        partie.nom_joueur = String::from("   ");

        partie.enregistrer_score(&mut classement);
        assert_eq!(partie.rang, None);
        assert!(partie.message.contains("nom"));
        assert!(classement.meilleurs(Taille::Neuf, None, 10).is_empty());
    }

    #[test]
    fn test_enregistrer_sans_score_ne_fait_rien() {
        let chemin = crate::classement::chemin_temporaire();
        let mut classement = Classement::depuis_fichier(chemin);
        let mut partie = partie_fixe();
        gagner(&mut partie);
        partie.nom_joueur = String::from("Lucas");

        partie.enregistrer_score(&mut classement);
        assert_eq!(partie.rang, None);
        assert!(classement.meilleurs(Taille::Neuf, None, 10).is_empty());
    }

    // Une partie sur une grille générée de la taille demandée, avec peu de cases à trouver
    fn partie_de_taille(taille: Taille) -> Partie {
        let grille = Grille::generer(taille, String::from("Test"), taille.cases() / 8);
        Partie::new(grille, Origine::Niveau(Niveau::Moyen, taille))
    }

    #[test]
    fn test_victoire_dans_chaque_taille() {
        for taille in Taille::TOUTES {
            let mut partie = partie_de_taille(taille);
            gagner(&mut partie);
            assert_eq!(partie.fin, Some(Fin::Victoire), "{}", taille.libelle());
            assert_eq!(partie.erreurs, 0, "{}", taille.libelle());
            assert!(partie.score.is_some(), "{}", taille.libelle());
        }
    }

    #[test]
    fn test_le_score_suit_la_taille() {
        // Moyen : 2000 points en 9×9, 10 000 en 16×16 (moins quelques secondes tout au plus)
        let mut neuf = partie_de_taille(Taille::Neuf);
        gagner(&mut neuf);
        let mut seize = partie_de_taille(Taille::Seize);
        gagner(&mut seize);
        let (neuf, seize) = (neuf.score.unwrap(), seize.score.unwrap());
        assert!(neuf <= 2000 && neuf > 1900, "{}", neuf);
        assert!(seize <= 10_000 && seize > 9900, "{}", seize);
    }

    #[test]
    fn test_un_nombre_trop_grand_pour_la_grille_est_ignore_sans_erreur() {
        let mut partie = partie_de_taille(Taille::Quatre);
        let vides: Vec<(usize, usize)> = (0..16)
            .map(|i| (i / 4, i % 4))
            .filter(|&(ligne, col)| partie.grille.valeur(ligne, col) == 0)
            .collect();
        let (ligne, col) = vides[0];
        jouer(&mut partie, ligne, col, 5);
        assert_eq!(partie.erreurs, 0);
        assert_eq!(partie.grille.valeur(ligne, col), 0);
        assert!(partie.message.contains("1 à 4"));
    }

    #[test]
    fn test_jouer_un_nombre_a_deux_chiffres() {
        let mut partie = partie_de_taille(Taille::Seize);
        let mut solution = partie.grille.clone();
        assert!(solution.remplir_solution());
        // On cherche une case vide dont la solution est un nombre à deux chiffres
        let cote = 16;
        let cible = (0..cote * cote)
            .map(|i| (i / cote, i % cote))
            .find(|&(ligne, col)| partie.grille.valeur(ligne, col) == 0 && solution.valeur(ligne, col) >= 10);
        if let Some((ligne, col)) = cible {
            let nombre = solution.valeur(ligne, col);
            jouer(&mut partie, ligne, col, nombre);
            assert_eq!(partie.grille.valeur(ligne, col), nombre);
            assert_eq!(partie.erreurs, 0);
        }
    }

    #[test]
    fn test_deplacer_reste_dans_la_grille() {
        let mut partie = partie_de_taille(Taille::Quatre);
        // Sans sélection, on commence en haut à gauche
        partie.deplacer(1, 0);
        assert_eq!(partie.selection, Some((0, 0)));
        partie.selection = Some((3, 3));
        partie.deplacer(1, 1);
        assert_eq!(partie.selection, Some((3, 3)));
        partie.deplacer(-1, -1);
        assert_eq!(partie.selection, Some((2, 2)));
        partie.selection = Some((0, 0));
        partie.deplacer(-1, -1);
        assert_eq!(partie.selection, Some((0, 0)));

        // En 16×16, la sélection va jusqu'à la case (15, 15)
        let mut grande = partie_de_taille(Taille::Seize);
        grande.selection = Some((14, 14));
        grande.deplacer(1, 1);
        grande.deplacer(1, 1);
        assert_eq!(grande.selection, Some((15, 15)));
    }

    #[test]
    fn test_valider_le_chiffre_en_attente() {
        let mut partie = partie_de_taille(Taille::Seize);
        let mut solution = partie.grille.clone();
        assert!(solution.remplir_solution());
        let cible = (0..256)
            .map(|i| (i / 16, i % 16))
            .find(|&(ligne, col)| partie.grille.valeur(ligne, col) == 0 && solution.valeur(ligne, col) == 1);
        if let Some((ligne, col)) = cible {
            // Un 1 seul : on attend peut-être un 10 à 16, puis on le valide
            partie.selection = Some((ligne, col));
            partie.tampon = Some((1, 0.0));
            partie.valider_tampon();
            assert_eq!(partie.tampon, None);
            assert_eq!(partie.grille.valeur(ligne, col), 1);
        }
    }

    #[test]
    fn test_saisie_de_chiffres_sans_nombre_a_deux_chiffres() {
        // En 9×9 ou moins, chaque chiffre se joue tout de suite
        for chiffre in 1..=9u8 {
            assert_eq!(saisir_chiffre(None, chiffre, 9), (Some(chiffre), None));
        }
        assert_eq!(saisir_chiffre(None, 0, 9), (Some(0), None));
        // 6×6 : le 7 n'existe pas
        assert_eq!(saisir_chiffre(None, 7, 6), (None, None));
        assert_eq!(saisir_chiffre(None, 6, 6), (Some(6), None));
    }

    #[test]
    fn test_saisie_de_nombres_a_deux_chiffres_en_16() {
        // Le 1 attend un second chiffre : il peut devenir 10 à 16
        assert_eq!(saisir_chiffre(None, 1, 16), (None, Some(1)));
        for second in 0..=6u8 {
            assert_eq!(saisir_chiffre(Some(1), second, 16), (Some(10 + second), None));
        }
        // 17 n'existe pas : le 7 compte seul
        assert_eq!(saisir_chiffre(Some(1), 7, 16), (Some(7), None));
        assert_eq!(saisir_chiffre(Some(1), 9, 16), (Some(9), None));
        // Les autres chiffres se jouent tout de suite
        assert_eq!(saisir_chiffre(None, 2, 16), (Some(2), None));
        assert_eq!(saisir_chiffre(None, 9, 16), (Some(9), None));
        // Le 0 efface
        assert_eq!(saisir_chiffre(None, 0, 16), (Some(0), None));
    }

    #[test]
    fn test_saisie_de_nombres_a_deux_chiffres_en_12() {
        assert_eq!(saisir_chiffre(None, 1, 12), (None, Some(1)));
        assert_eq!(saisir_chiffre(Some(1), 2, 12), (Some(12), None));
        // 13 n'existe pas en 12×12
        assert_eq!(saisir_chiffre(Some(1), 3, 12), (Some(3), None));
    }

    #[test]
    fn test_formater_duree() {
        assert_eq!(formater_duree(Duration::ZERO), "00:00");
        assert_eq!(formater_duree(Duration::from_secs(75)), "01:15");
        assert_eq!(formater_duree(Duration::from_secs(3600)), "60:00");
    }

    #[test]
    fn test_une_partie_restauree_reprend_ou_elle_en_etait() {
        let mut partie = partie_moyenne();
        jouer(&mut partie, 0, 2, 5); // erreur : brise une règle
        partie.demander_indice();
        partie.ecoule = Duration::from_secs(120);

        let restauree = Partie::restaurer(partie.vers_sauvegarde()).unwrap();
        assert_eq!(restauree.erreurs, 1);
        assert_eq!(restauree.indices, 1);
        assert_eq!(restauree.grille.cases(), partie.grille.cases());
        assert_eq!(restauree.origine, partie.origine);
        assert_eq!(restauree.fin, None);
        assert!(!restauree.modifiee);
        assert!(restauree.duree() >= Duration::from_secs(120));

        // La partie reprise est en pause : le chrono repart à la reprise
        assert!(restauree.reprise.is_none());
        let mut restauree = restauree;
        restauree.demarrer_chrono();
        assert!(restauree.reprise.is_some());
    }

    #[test]
    fn test_une_partie_finie_ne_se_restaure_pas() {
        let mut gagnee = partie_moyenne();
        gagner(&mut gagnee);
        assert!(Partie::restaurer(gagnee.vers_sauvegarde()).is_none());

        let mut perdue = partie_moyenne();
        perdue.erreurs = MAX_ERREURS;
        assert!(Partie::restaurer(perdue.vers_sauvegarde()).is_none());
    }

    #[test]
    fn test_les_changements_marquent_la_partie_a_sauvegarder() {
        let mut partie = partie_moyenne();
        assert!(!partie.modifiee);

        // Un clic sans coup ne change rien
        partie.selection = Some((0, 0));
        assert!(!partie.modifiee);

        jouer(&mut partie, 0, 2, 4);
        assert!(partie.modifiee);

        partie.modifiee = false;
        partie.demander_indice();
        assert!(partie.modifiee);

        partie.modifiee = false;
        partie.afficher_solution();
        assert!(partie.modifiee);
        assert!(partie.fin.is_some());
    }

    // Dans les tests d'annulation, la case (0, 2) de la grille fixe admet 1, 2 ou 4 :
    // la solution est 4, donc 1 est un chiffre faux mais permis.
    #[test]
    fn test_annuler_et_refaire_un_chiffre() {
        let mut partie = partie_fixe();
        jouer(&mut partie, 0, 2, 4);
        assert_eq!(partie.grille.valeur(0, 2), 4);

        partie.annuler();
        assert_eq!(partie.grille.valeur(0, 2), 0);
        assert_eq!(partie.selection, Some((0, 2)));
        assert!(partie.historique.is_empty());

        partie.refaire();
        assert_eq!(partie.grille.valeur(0, 2), 4);
        assert!(partie.annules.is_empty());
    }

    #[test]
    fn test_annuler_remet_le_chiffre_remplace() {
        let mut partie = partie_fixe();
        jouer(&mut partie, 0, 2, 4);
        jouer(&mut partie, 0, 2, 1); // faux mais permis : il remplace le 4
        assert_eq!(partie.grille.valeur(0, 2), 1);

        partie.annuler();
        assert_eq!(partie.grille.valeur(0, 2), 4);
        partie.annuler();
        assert_eq!(partie.grille.valeur(0, 2), 0);
        partie.refaire();
        partie.refaire();
        assert_eq!(partie.grille.valeur(0, 2), 1);
    }

    #[test]
    fn test_annuler_une_case_effacee() {
        let mut partie = partie_fixe();
        jouer(&mut partie, 0, 2, 4);
        jouer(&mut partie, 0, 2, 0);
        assert_eq!(partie.grille.valeur(0, 2), 0);
        partie.annuler();
        assert_eq!(partie.grille.valeur(0, 2), 4);
    }

    #[test]
    fn test_annuler_ne_rend_pas_les_erreurs() {
        let mut partie = partie_fixe();
        jouer(&mut partie, 0, 2, 1); // faux mais permis : une erreur
        assert_eq!(partie.erreurs, 1);
        partie.annuler();
        assert_eq!(partie.grille.valeur(0, 2), 0);
        assert_eq!(partie.erreurs, 1);
    }

    #[test]
    fn test_un_coup_refuse_ou_inutile_n_est_pas_retenu() {
        let mut partie = partie_fixe();
        jouer(&mut partie, 0, 2, 5); // brise une règle : refusé
        jouer(&mut partie, 0, 2, 0); // case déjà vide
        jouer(&mut partie, 0, 0, 7); // case de départ
        assert!(partie.historique.is_empty());
        jouer(&mut partie, 0, 2, 4);
        jouer(&mut partie, 0, 2, 4); // déjà là
        assert_eq!(partie.historique.len(), 1);
    }

    #[test]
    fn test_un_nouveau_coup_efface_les_coups_a_refaire() {
        let mut partie = partie_fixe();
        jouer(&mut partie, 0, 2, 4);
        partie.annuler();
        assert_eq!(partie.annules.len(), 1);
        jouer(&mut partie, 0, 3, 6);
        assert!(partie.annules.is_empty());
        partie.refaire();
        assert_eq!(partie.message, "Rien à refaire");
        assert_eq!(partie.grille.valeur(0, 2), 0);
    }

    #[test]
    fn test_annuler_un_indice_garde_son_prix() {
        let mut partie = partie_fixe();
        partie.demander_indice();
        let (ligne, col) = partie.selection.unwrap();
        assert_ne!(partie.grille.valeur(ligne, col), 0);
        partie.annuler();
        assert_eq!(partie.grille.valeur(ligne, col), 0);
        assert_eq!(partie.indices, 1);
        partie.refaire();
        assert_ne!(partie.grille.valeur(ligne, col), 0);
    }

    #[test]
    fn test_rien_a_annuler() {
        let mut partie = partie_fixe();
        partie.annuler();
        assert_eq!(partie.message, "Rien à annuler");
        assert!(!partie.modifiee);
    }

    #[test]
    fn test_pas_d_annulation_apres_la_fin() {
        let mut partie = partie_fixe();
        jouer(&mut partie, 0, 2, 4);
        partie.afficher_solution();
        let cases = partie.grille.cases().to_vec();
        partie.annuler();
        partie.refaire();
        assert_eq!(partie.grille.cases(), cases.as_slice());
    }

    #[test]
    fn test_annuler_marque_la_partie_a_sauvegarder() {
        let mut partie = partie_fixe();
        jouer(&mut partie, 0, 2, 4);
        partie.modifiee = false;
        partie.annuler();
        assert!(partie.modifiee);
        partie.modifiee = false;
        partie.refaire();
        assert!(partie.modifiee);
    }

    // Position d'une case dans `notes` de la grille d'entraînement (9×9)
    fn pos(ligne: usize, col: usize) -> usize {
        ligne * 9 + col
    }

    #[test]
    fn test_ecrire_et_retirer_une_note() {
        let mut partie = partie_fixe();
        partie.basculer_mode_notes();
        assert!(partie.mode_notes);

        jouer(&mut partie, 0, 2, 1);
        jouer(&mut partie, 0, 2, 4);
        assert_eq!(partie.notes[pos(0, 2)], 0b1001);
        // Le même chiffre une seconde fois retire la note
        jouer(&mut partie, 0, 2, 1);
        assert_eq!(partie.notes[pos(0, 2)], 0b1000);

        // Une note n'est pas un coup : ni chiffre, ni erreur, ni historique
        assert_eq!(partie.grille.valeur(0, 2), 0);
        assert_eq!(partie.erreurs, 0);
        assert!(partie.historique.is_empty());
        assert!(partie.modifiee);
    }

    #[test]
    fn test_une_note_peut_etre_fausse_sans_erreur() {
        let mut partie = partie_fixe();
        partie.basculer_mode_notes();
        jouer(&mut partie, 0, 2, 5); // le 5 est déjà dans la ligne : c'est juste une note
        jouer(&mut partie, 0, 2, 9);
        assert_eq!(partie.notes[pos(0, 2)], 1 << 4 | 1 << 8);
        assert_eq!(partie.erreurs, 0);
    }

    #[test]
    fn test_pas_de_note_sur_une_case_remplie_ou_de_depart() {
        let mut partie = partie_fixe();
        jouer(&mut partie, 0, 2, 4);
        partie.basculer_mode_notes();
        jouer(&mut partie, 0, 2, 1);
        assert_eq!(partie.notes[pos(0, 2)], 0);
        assert!(partie.message.contains("contient déjà un chiffre"));

        jouer(&mut partie, 0, 0, 1); // case de départ
        assert_eq!(partie.notes[pos(0, 0)], 0);
    }

    #[test]
    fn test_zero_en_mode_notes() {
        let mut partie = partie_fixe();
        partie.basculer_mode_notes();
        jouer(&mut partie, 0, 2, 1);
        jouer(&mut partie, 0, 2, 2);
        // Sur une case vide, 0 efface ses notes
        jouer(&mut partie, 0, 2, 0);
        assert_eq!(partie.notes[pos(0, 2)], 0);

        // Sur une case remplie, 0 efface le chiffre, même en mode notes
        partie.mode_notes = false;
        jouer(&mut partie, 0, 2, 4);
        partie.mode_notes = true;
        jouer(&mut partie, 0, 2, 0);
        assert_eq!(partie.grille.valeur(0, 2), 0);
    }

    #[test]
    fn test_un_nombre_trop_grand_n_est_pas_une_note() {
        let mut partie = partie_fixe();
        partie.basculer_mode_notes();
        jouer(&mut partie, 0, 2, 10);
        assert_eq!(partie.notes[pos(0, 2)], 0);
    }

    #[test]
    fn test_les_notes_a_deux_chiffres_en_16x16() {
        let mut partie = partie_de_taille(Taille::Seize);
        partie.basculer_mode_notes();
        let vide = (0..256).find(|&i| partie.grille.valeur(i / 16, i % 16) == 0).unwrap();
        let (ligne, col) = (vide / 16, vide % 16);
        jouer(&mut partie, ligne, col, 16);
        jouer(&mut partie, ligne, col, 12);
        assert_eq!(partie.notes[vide], 1 << 15 | 1 << 11);
    }

    #[test]
    fn test_poser_un_chiffre_efface_les_notes_et_annuler_les_rend() {
        let mut partie = partie_fixe();
        partie.basculer_mode_notes();
        jouer(&mut partie, 0, 2, 1);
        jouer(&mut partie, 0, 2, 4);
        partie.basculer_mode_notes();

        jouer(&mut partie, 0, 2, 4);
        assert_eq!(partie.notes[pos(0, 2)], 0);

        partie.annuler();
        assert_eq!(partie.grille.valeur(0, 2), 0);
        assert_eq!(partie.notes[pos(0, 2)], 0b1001);

        partie.refaire();
        assert_eq!(partie.grille.valeur(0, 2), 4);
        assert_eq!(partie.notes[pos(0, 2)], 0);
    }

    #[test]
    fn test_un_indice_efface_les_notes_de_sa_case() {
        let mut partie = partie_fixe();
        // Des notes dans toutes les cases vides
        partie.remplir_notes();
        partie.demander_indice();
        let (ligne, col) = partie.selection.unwrap();
        assert_ne!(partie.grille.valeur(ligne, col), 0);
        assert_eq!(partie.notes[pos(ligne, col)], 0);
        partie.annuler();
        assert_ne!(partie.notes[pos(ligne, col)], 0);
    }

    #[test]
    fn test_remplir_les_notes() {
        let mut partie = partie_fixe();
        partie.notes[pos(0, 2)] = 1 << 8; // une note à la main : elle est remplacée
        partie.remplir_notes();
        assert_eq!(partie.notes[pos(0, 2)], 0b1011);
        // Pas de note dans une case de départ
        assert_eq!(partie.notes[pos(0, 0)], 0);
        // Ni dans une case remplie
        jouer(&mut partie, 0, 2, 4);
        partie.remplir_notes();
        assert_eq!(partie.notes[pos(0, 2)], 0);
        // Les notes d'une autre case de la ligne ont perdu le 4
        assert_eq!(partie.notes[pos(0, 3)] & (1 << 3), 0);
        assert!(partie.modifiee);
    }

    #[test]
    fn test_les_notes_suivent_la_sauvegarde() {
        let mut partie = partie_fixe();
        partie.basculer_mode_notes();
        jouer(&mut partie, 0, 2, 1);
        jouer(&mut partie, 0, 2, 2);
        let restauree = Partie::restaurer(partie.vers_sauvegarde()).unwrap();
        assert_eq!(restauree.notes, partie.notes);
        assert_eq!(restauree.notes[pos(0, 2)], 0b11);
    }

    // Une application dont les trois fichiers sont dans le dossier temporaire
    fn application_de_test(partie: Partie) -> Application {
        use crate::classement::chemin_temporaire;
        Application {
            ecran: Ecran::Partie,
            partie: Some(partie),
            generation: None,
            erreur: String::new(),
            classement: Classement::depuis_fichier(chemin_temporaire()),
            taille_choisie: Taille::Neuf,
            taille_classement: Taille::Neuf,
            filtre: None,
            dernier_nom: String::new(),
            sauvegarde: Sauvegarde::depuis_fichier(chemin_temporaire()),
            statistiques: Statistiques::depuis_fichier(chemin_temporaire()),
            taille_stats: Taille::Neuf,
            filtre_stats: None,
            resolveur: Resolveur::new(),
            reglages: Reglages::default(),
        }
    }

    // Supprime les fichiers qu'une application de test a pu écrire
    fn nettoyer(application: &Application) {
        application.sauvegarde.supprimer();
        application.statistiques.effacer_fichier();
    }

    #[test]
    fn test_une_victoire_entre_une_seule_fois_dans_les_statistiques() {
        let mut partie = partie_moyenne();
        gagner(&mut partie);
        let mut application = application_de_test(partie);

        // La fenêtre sauvegarde à chaque image : la partie finie ne doit compter qu'une fois
        application.sauvegarder_partie();
        application.sauvegarder_partie();
        eframe::App::on_exit(&mut application, None);

        let resume = application.statistiques.resume(Taille::Neuf, None);
        assert_eq!(resume.jouees, 1);
        assert_eq!(resume.victoires, 1);
        assert_eq!(resume.meilleur_score, Some(application.partie.as_ref().unwrap().score.unwrap()));
        assert_eq!(application.statistiques.resume(Taille::Neuf, Some(Niveau::Moyen)).jouees, 1);
        assert_eq!(application.statistiques.resume(Taille::Neuf, Some(Niveau::Facile)).jouees, 0);
        assert_eq!(application.statistiques.resume(Taille::Six, None).jouees, 0);
        nettoyer(&application);
    }

    #[test]
    fn test_une_defaite_et_un_abandon_entrent_dans_les_statistiques() {
        let mut perdue = partie_moyenne();
        for _ in 0..MAX_ERREURS {
            jouer(&mut perdue, 0, 2, 5);
        }
        assert_eq!(perdue.fin, Some(Fin::Defaite));
        let mut application = application_de_test(perdue);
        application.sauvegarder_partie();
        let resume = application.statistiques.resume(Taille::Neuf, None);
        assert_eq!((resume.jouees, resume.defaites, resume.victoires), (1, 1, 0));

        let mut abandonnee = partie_moyenne();
        abandonnee.afficher_solution();
        application.partie = Some(abandonnee);
        application.sauvegarder_partie();
        let resume = application.statistiques.resume(Taille::Neuf, None);
        assert_eq!((resume.jouees, resume.defaites, resume.abandons), (2, 1, 1));
        nettoyer(&application);
    }

    #[test]
    fn test_une_partie_en_cours_ou_d_entrainement_n_est_pas_comptee() {
        let mut en_cours = partie_moyenne();
        jouer(&mut en_cours, 0, 2, 4);
        let mut application = application_de_test(en_cours);
        application.sauvegarder_partie();
        assert_eq!(application.statistiques.resume(Taille::Neuf, None).jouees, 0);

        let mut entrainement = partie_fixe();
        gagner(&mut entrainement);
        assert_eq!(entrainement.fin, Some(Fin::Victoire));
        application.partie = Some(entrainement);
        application.sauvegarder_partie();
        assert_eq!(application.statistiques.resume(Taille::Neuf, None).jouees, 0);
        nettoyer(&application);
    }

    #[test]
    fn test_la_statistique_garde_les_chiffres_de_la_partie() {
        let mut partie = partie_moyenne();
        jouer(&mut partie, 0, 2, 5); // une erreur
        partie.demander_indice();
        gagner(&mut partie);
        let jouee = partie.vers_statistique().unwrap();
        assert_eq!(jouee.resultat, Resultat::Victoire);
        assert_eq!((jouee.erreurs, jouee.indices), (1, 1));
        assert_eq!(jouee.score, partie.score.unwrap());
        assert_eq!((jouee.niveau, jouee.taille), (Niveau::Moyen, Taille::Neuf));
    }

    #[test]
    fn test_annuler_valide_le_chiffre_en_attente() {
        let mut partie = partie_fixe();
        jouer(&mut partie, 0, 2, 4);
        partie.tampon = Some((1, 0.0));
        partie.selection = Some((0, 3));
        partie.annuler();
        // Le 1 en attente est joué en (0, 3) où il est refusé, puis le 4 est annulé
        assert!(partie.tampon.is_none());
        assert_eq!(partie.grille.valeur(0, 2), 0);
    }

    // La luminance relative d'une couleur (formule du W3C pour le contraste)
    fn luminance(couleur: Color32) -> f32 {
        let lineaire = |composante: u8| {
            let c = composante as f32 / 255.0;
            if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * lineaire(couleur.r()) + 0.7152 * lineaire(couleur.g()) + 0.0722 * lineaire(couleur.b())
    }

    fn contraste(a: Color32, b: Color32) -> f32 {
        let (claire, sombre) = (luminance(a).max(luminance(b)), luminance(a).min(luminance(b)));
        (claire + 0.05) / (sombre + 0.05)
    }

    const COMBINAISONS: [(bool, bool); 4] = [(false, false), (false, true), (true, false), (true, true)];

    #[test]
    fn test_les_chiffres_se_lisent_sur_le_fond_dans_toutes_les_palettes() {
        for (sombre, daltonien) in COMBINAISONS {
            let palette = Palette::pour(sombre, daltonien);
            for (nom, couleur) in [
                ("départ", palette.chiffre_depart),
                ("joueur", palette.chiffre_joueur),
                ("faux", palette.chiffre_faux),
                ("note", palette.note),
                ("vert", palette.vert),
                ("rouge", palette.rouge),
            ] {
                let rapport = contraste(couleur, palette.fond);
                assert!(
                    rapport >= 3.0,
                    "{} illisible (contraste {:.1}) en sombre={} daltonien={}",
                    nom, rapport, sombre, daltonien
                );
            }
        }
    }

    #[test]
    fn test_les_chiffres_se_distinguent_entre_eux() {
        for (sombre, daltonien) in COMBINAISONS {
            let palette = Palette::pour(sombre, daltonien);
            let (depart, joueur, faux) = (
                palette.chiffre(GenreChiffre::Depart),
                palette.chiffre(GenreChiffre::Joueur),
                palette.chiffre(GenreChiffre::Faux),
            );
            assert!(depart != joueur && joueur != faux && depart != faux);
        }
    }

    #[test]
    fn test_le_mode_daltonien_evite_le_rouge_et_le_vert() {
        for sombre in [false, true] {
            let normale = Palette::pour(sombre, false);
            let daltonien = Palette::pour(sombre, true);
            assert_ne!(normale.chiffre_faux, daltonien.chiffre_faux);
            assert_ne!(normale.vert, daltonien.vert);
            // Le vert et le rouge d'une même palette ne se confondent pas par leur luminosité :
            // le chiffre faux est un orange, avec plus de rouge que de bleu
            assert!(daltonien.chiffre_faux.r() > daltonien.chiffre_faux.b());
            // Ce que le mode ne touche pas
            assert_eq!(normale.fond, daltonien.fond);
            assert_eq!(normale.trait_epais, daltonien.trait_epais);
        }
    }

    #[test]
    fn test_le_theme_sombre_a_un_fond_sombre() {
        assert!(luminance(Palette::pour(true, false).fond) < 0.1);
        assert!(luminance(Palette::pour(false, false).fond) > 0.9);
    }

    #[test]
    fn test_le_theme_choisi_devient_celui_d_egui() {
        assert_eq!(theme_egui(Theme::Systeme), egui::ThemePreference::System);
        assert_eq!(theme_egui(Theme::Clair), egui::ThemePreference::Light);
        assert_eq!(theme_egui(Theme::Sombre), egui::ThemePreference::Dark);
    }

    #[test]
    fn test_sons_d_un_bon_chiffre_d_une_erreur_et_d_un_indice() {
        let mut partie = partie_fixe();
        jouer(&mut partie, 0, 2, 4);
        assert_eq!(partie.sons, [Son::Bon]);

        partie.sons.clear();
        jouer(&mut partie, 0, 3, 5); // brise une règle : une erreur
        assert_eq!(partie.sons, [Son::Erreur]);

        partie.sons.clear();
        partie.demander_indice();
        assert_eq!(partie.sons, [Son::Indice]);
    }

    #[test]
    fn test_pas_de_son_quand_rien_ne_change() {
        let mut partie = partie_fixe();
        jouer(&mut partie, 0, 2, 4);
        partie.sons.clear();
        jouer(&mut partie, 0, 2, 4); // déjà là
        jouer(&mut partie, 0, 0, 7); // case de départ
        jouer(&mut partie, 0, 2, 0); // effacer
        assert!(partie.sons.is_empty());
    }

    #[test]
    fn test_sons_de_victoire_de_defaite_et_d_abandon() {
        let mut gagnee = partie_fixe();
        gagner(&mut gagnee);
        assert_eq!(gagnee.sons.last(), Some(&Son::Victoire));

        let mut perdue = partie_fixe();
        for _ in 0..MAX_ERREURS {
            jouer(&mut perdue, 0, 2, 5);
        }
        assert_eq!(perdue.sons.last(), Some(&Son::Defaite));
        assert_eq!(perdue.sons.iter().filter(|&&son| son == Son::Erreur).count(), MAX_ERREURS as usize);

        let mut abandonnee = partie_fixe();
        abandonnee.afficher_solution();
        assert!(abandonnee.sons.is_empty());
    }
}
