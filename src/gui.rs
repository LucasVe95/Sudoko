//! L'interface graphique (egui) : menu, partie et écran de fin.

use crate::classement::{
    calculer_score, maintenant, nettoyer_nom, penalite_erreur, penalite_indice, Classement, Entree,
};
use crate::grille::{Grille, Niveau, Origine, Taille};
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

// Couleurs de la grille : elle reste claire, quel que soit le thème de la fenêtre
const FOND: Color32 = Color32::WHITE;
const CASE_SELECTIONNEE: Color32 = Color32::from_rgb(170, 205, 250);
const MEME_CHIFFRE: Color32 = Color32::from_rgb(210, 225, 250);
const LIGNE_COLONNE_BLOC: Color32 = Color32::from_rgb(235, 240, 248);
const CHIFFRE_DEPART: Color32 = Color32::from_rgb(25, 25, 25);
const CHIFFRE_JOUEUR: Color32 = Color32::from_rgb(30, 90, 200);
const CHIFFRE_FAUX: Color32 = Color32::from_rgb(210, 40, 40);
const TRAIT_FIN: Color32 = Color32::from_rgb(170, 170, 170);
const TRAIT_EPAIS: Color32 = Color32::from_rgb(25, 25, 25);
const VERT: Color32 = Color32::from_rgb(40, 160, 70);
const ROUGE: Color32 = Color32::from_rgb(210, 60, 60);

// Comment une partie s'est terminée
#[derive(Clone, Copy, PartialEq, Debug)]
enum Fin {
    Victoire,
    Defaite,
    Abandon,
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
}

impl Partie {
    fn new(grille: Grille, origine: Origine) -> Partie {
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
        if valeur == 0 {
            self.grille.effacer(ligne, col);
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
        if correct {
            if place || self.grille.valeur(ligne, col) == valeur {
                self.message.clear();
            } else {
                // Le bon chiffre est refusé : un chiffre faux posé plus tôt le bloque
                self.message =
                    String::from("Un chiffre rouge de la même ligne, colonne ou bloc vous bloque : effacez-le");
            }
        } else {
            self.erreurs += 1;
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
            Some((ligne, col, valeur)) => {
                self.indices += 1;
                self.selection = Some((ligne, col));
                self.message = format!(
                    "Indice : {} en ligne {}, colonne {}",
                    valeur,
                    ligne + 1,
                    col + 1
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
            let touches: Vec<Key> = entree
                .events
                .iter()
                .filter_map(|evenement| match evenement {
                    egui::Event::Key { key, pressed: true, repeat: false, .. } => Some(*key),
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

        for touche in touches {
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
    fn dessiner_grille(&mut self, ui: &mut egui::Ui, cote_px: f32) {
        let taille = self.grille.taille();
        let nombre = taille.cote();
        let (bloc_h, bloc_l) = (taille.bloc_h(), taille.bloc_l());
        let (zone, reponse) = ui.allocate_exact_size(Vec2::splat(cote_px), Sense::click());
        let case = cote_px / nombre as f32;

        if reponse.clicked() {
            if let Some(souris) = reponse.interact_pointer_pos() {
                // Le chiffre en attente est joué dans l'ancienne case avant de changer de case
                self.valider_tampon();
                let col = (((souris.x - zone.min.x) / case) as usize).min(nombre - 1);
                let ligne = (((souris.y - zone.min.y) / case) as usize).min(nombre - 1);
                self.selection = Some((ligne, col));
            }
        }

        let peintre = ui.painter_at(zone);
        peintre.rect_filled(zone, 0.0, FOND);

        // Les cases qui contiennent le même chiffre que la case choisie sont mises en valeur
        let chiffre_choisi = self
            .selection
            .map(|(ligne, col)| self.grille.valeur(ligne, col))
            .filter(|&valeur| valeur != 0);

        for ligne in 0..nombre {
            for col in 0..nombre {
                let coin = Pos2::new(
                    zone.min.x + col as f32 * case,
                    zone.min.y + ligne as f32 * case,
                );
                let rectangle = Rect::from_min_size(coin, Vec2::splat(case));
                let valeur = self.grille.valeur(ligne, col);

                let fond = match self.selection {
                    Some(choisie) if choisie == (ligne, col) => Some(CASE_SELECTIONNEE),
                    _ if valeur != 0 && Some(valeur) == chiffre_choisi => Some(MEME_CHIFFRE),
                    Some((l, c))
                        if l == ligne
                            || c == col
                            || (l / bloc_h == ligne / bloc_h && c / bloc_l == col / bloc_l) =>
                    {
                        Some(LIGNE_COLONNE_BLOC)
                    }
                    _ => None,
                };
                if let Some(couleur) = fond {
                    peintre.rect_filled(rectangle, 0.0, couleur);
                }

                if valeur != 0 {
                    // Un chiffre du joueur qui n'est pas celui de la solution s'affiche en rouge
                    let couleur = if self.grille.est_fixe(ligne, col) {
                        CHIFFRE_DEPART
                    } else if self.grille.est_correct(ligne, col, valeur) {
                        CHIFFRE_JOUEUR
                    } else {
                        CHIFFRE_FAUX
                    };
                    // Un nombre à deux chiffres a besoin d'une police un peu plus petite
                    let hauteur = if valeur >= 10 { case * 0.45 } else { case * 0.6 };
                    peintre.text(
                        rectangle.center(),
                        Align2::CENTER_CENTER,
                        valeur,
                        FontId::proportional(hauteur),
                        couleur,
                    );
                }
            }
        }

        // Quadrillage : les traits qui séparent les blocs sont plus épais.
        // Les traits verticaux suivent la largeur d'un bloc, les horizontaux sa hauteur.
        for i in 0..=nombre {
            let decalage = i as f32 * case;
            let vertical = if i % bloc_l == 0 {
                Stroke::new(3.0, TRAIT_EPAIS)
            } else {
                Stroke::new(1.0, TRAIT_FIN)
            };
            peintre.line_segment(
                [
                    Pos2::new(zone.min.x + decalage, zone.min.y),
                    Pos2::new(zone.min.x + decalage, zone.max.y),
                ],
                vertical,
            );
            let horizontal = if i % bloc_h == 0 {
                Stroke::new(3.0, TRAIT_EPAIS)
            } else {
                Stroke::new(1.0, TRAIT_FIN)
            };
            peintre.line_segment(
                [
                    Pos2::new(zone.min.x, zone.min.y + decalage),
                    Pos2::new(zone.max.x, zone.min.y + decalage),
                ],
                horizontal,
            );
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
    ) -> Option<Action> {
        let mut action = None;
        let (titre, couleur) = match fin {
            Fin::Victoire => (String::from("Bravo, grille terminée !"), VERT),
            Fin::Defaite => (format!("Perdu : {} erreurs", MAX_ERREURS), ROUGE),
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
    fn afficher(&mut self, ui: &mut egui::Ui, classement: &mut Classement) -> Option<Action> {
        let mut action = None;

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
            .min(ui.available_height() - 200.0)
            .max(180.0);
        ui.vertical_centered(|ui| self.dessiner_grille(ui, cote));
        ui.add_space(8.0);

        match self.fin {
            Some(fin) => {
                if let Some(demande) = self.dessiner_fin(ui, fin, classement) {
                    action = Some(demande);
                }
            }
            None => self.dessiner_commandes(ui),
        }

        ui.add_space(8.0);
        if !self.message.is_empty() {
            ui.label(self.message.as_str());
        } else if self.fin.is_none() {
            ui.label("Clic ou flèches pour choisir une case, puis un nombre (0 ou Suppr pour effacer)");
        }
        action
    }
}

enum Ecran {
    Menu,
    Partie,
    Classement,
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
}

impl Application {
    fn new() -> Application {
        let classement = Classement::charger();
        let dernier_nom = classement.dernier_nom().map(str::to_string).unwrap_or_default();
        Application {
            ecran: Ecran::Menu,
            partie: None,
            generation: None,
            erreur: String::new(),
            classement,
            taille_choisie: Taille::Neuf,
            taille_classement: Taille::Neuf,
            filtre: None,
            dernier_nom,
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
                if gros_bouton(ui, "Classement") {
                    choix = Some(ChoixMenu::Classement);
                }
                ui.add_space(16.0);
                if gros_bouton(ui, "Quitter") {
                    choix = Some(ChoixMenu::Quitter);
                }
            });

            if !self.erreur.is_empty() {
                ui.add_space(8.0);
                ui.colored_label(ROUGE, self.erreur.as_str());
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
            Some(ChoixMenu::Quitter) => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
            None => {}
        }
    }

    fn dessiner_partie(&mut self, ui: &mut egui::Ui) {
        let Some(partie) = self.partie.as_mut() else {
            self.ecran = Ecran::Menu;
            return;
        };
        let action = partie.afficher(ui, &mut self.classement);

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
}

// Un grand bouton pour le menu
fn gros_bouton(ui: &mut egui::Ui, texte: &str) -> bool {
    ui.add(egui::Button::new(texte).min_size(Vec2::new(240.0, 40.0)))
        .clicked()
}

impl eframe::App for Application {
    fn ui(&mut self, ui: &mut egui::Ui, _cadre: &mut eframe::Frame) {
        self.recevoir_nouvelle_grille();

        egui::CentralPanel::default().show(ui, |ui| match self.ecran {
            Ecran::Menu => self.dessiner_menu(ui),
            Ecran::Partie => self.dessiner_partie(ui),
            Ecran::Classement => self.dessiner_classement(ui),
        });
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
}
