//! L'interface graphique (egui) : dessine la grille et réagit à la souris et au clavier.

use crate::grille::{Grille, Niveau};
use eframe::egui::{self, Align2, Color32, FontId, Key, Pos2, Rect, Sense, Stroke, Vec2};
use std::sync::mpsc::{self, Receiver};
use std::thread;

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
const TRAIT_FIN: Color32 = Color32::from_rgb(170, 170, 170);
const TRAIT_EPAIS: Color32 = Color32::from_rgb(25, 25, 25);

// Ouvre la fenêtre et ne rend la main que lorsqu'elle est fermée
pub fn lancer(grille: Grille) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 700.0])
            .with_min_inner_size([360.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Sudoku",
        options,
        Box::new(|_contexte| Ok(Box::new(Application::new(grille)))),
    )
}

struct Application {
    grille: Grille,
    // Case choisie avec la souris ou les flèches : (ligne, col)
    selection: Option<(usize, usize)>,
    message: String,
    // true quand le joueur a demandé la solution : la grille pleine n'est alors pas une victoire
    solution_affichee: bool,
    // La nouvelle grille arrive par ce canal, générée dans un autre thread
    generation: Option<Receiver<Grille>>,
}

impl Application {
    fn new(grille: Grille) -> Application {
        Application {
            grille,
            selection: None,
            message: String::new(),
            solution_affichee: false,
            generation: None,
        }
    }

    // Lance la génération d'une grille dans un autre thread, pour ne pas figer la fenêtre
    fn nouvelle_partie(&mut self, niveau: Niveau, contexte: &egui::Context) {
        let (envoi, reception) = mpsc::channel();
        let contexte = contexte.clone();
        thread::spawn(move || {
            // Si la fenêtre est déjà fermée, personne ne reçoit : l'erreur d'envoi est sans importance
            let _ = envoi.send(Grille::generer_niveau(niveau));
            // Réveille la fenêtre pour qu'elle affiche la grille dès qu'elle est prête
            contexte.request_repaint();
        });
        self.generation = Some(reception);
        self.message = String::from("Génération de la grille...");
    }

    // Si la grille générée est arrivée, elle remplace la grille en cours
    fn recevoir_nouvelle_grille(&mut self) {
        let recue = self
            .generation
            .as_ref()
            .and_then(|reception| reception.try_recv().ok());
        if let Some(grille) = recue {
            self.grille = grille;
            self.generation = None;
            self.selection = None;
            self.solution_affichee = false;
            self.message.clear();
        }
    }

    fn partie_gagnee(&self) -> bool {
        self.grille.est_terminee() && !self.solution_affichee
    }

    // Place un chiffre dans la case choisie (0 pour effacer)
    fn jouer_chiffre(&mut self, valeur: u8) {
        let Some((ligne, col)) = self.selection else {
            self.message = String::from("Cliquez d'abord sur une case");
            return;
        };
        let accepte = if valeur == 0 {
            self.grille.effacer(ligne, col)
        } else {
            self.grille.placer(ligne, col, valeur)
        };
        self.message = if accepte {
            String::new()
        } else {
            String::from("Coup refusé : case de départ ou règle du sudoku brisée")
        };
    }

    fn demander_indice(&mut self) {
        match self.grille.indice() {
            Some((ligne, col, valeur)) => {
                self.selection = Some((ligne, col));
                self.message = format!(
                    "Indice : {} en ligne {}, colonne {}",
                    valeur,
                    ligne + 1,
                    col + 1
                );
            }
            None => {
                self.message = String::from(
                    "Pas d'indice possible : vos coups bloquent la grille, effacez-en quelques-uns",
                );
            }
        }
    }

    fn afficher_solution(&mut self) {
        if self.grille.remplir_solution() {
            self.solution_affichee = true;
            self.message = String::from("Voici la solution");
        } else {
            self.message = String::from("Cette grille n'a pas de solution");
        }
    }

    // Lit le clavier : renvoie le chiffre tapé (0 pour effacer) et déplace la sélection
    fn lire_clavier(&mut self, ui: &egui::Ui) -> Option<u8> {
        let (chiffre, descend, droite) = ui.input(|entree| {
            let chiffre = TOUCHES_CHIFFRES
                .iter()
                .find(|(touche, _)| entree.key_pressed(*touche))
                .map(|&(_, valeur)| valeur)
                .or_else(|| {
                    (entree.key_pressed(Key::Backspace) || entree.key_pressed(Key::Delete))
                        .then_some(0)
                });
            let descend = entree.key_pressed(Key::ArrowDown) as i32
                - entree.key_pressed(Key::ArrowUp) as i32;
            let droite = entree.key_pressed(Key::ArrowRight) as i32
                - entree.key_pressed(Key::ArrowLeft) as i32;
            (chiffre, descend, droite)
        });

        if descend != 0 || droite != 0 {
            self.selection = match self.selection {
                Some((ligne, col)) => Some((
                    (ligne as i32 + descend).clamp(0, 8) as usize,
                    (col as i32 + droite).clamp(0, 8) as usize,
                )),
                None => Some((0, 0)),
            };
        }
        chiffre
    }

    // Dessine la grille dans `zone` et sélectionne la case sur laquelle on clique
    fn dessiner_grille(&mut self, ui: &mut egui::Ui, cote: f32) {
        let (zone, reponse) = ui.allocate_exact_size(Vec2::splat(cote), Sense::click());
        let case = cote / 9.0;

        if reponse.clicked() {
            if let Some(souris) = reponse.interact_pointer_pos() {
                let col = (((souris.x - zone.min.x) / case) as usize).min(8);
                let ligne = (((souris.y - zone.min.y) / case) as usize).min(8);
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

        for ligne in 0..9 {
            for col in 0..9 {
                let coin = Pos2::new(
                    zone.min.x + col as f32 * case,
                    zone.min.y + ligne as f32 * case,
                );
                let rectangle = Rect::from_min_size(coin, Vec2::splat(case));
                let valeur = self.grille.valeur(ligne, col);

                let fond = match self.selection {
                    Some(choisie) if choisie == (ligne, col) => Some(CASE_SELECTIONNEE),
                    _ if valeur != 0 && Some(valeur) == chiffre_choisi => Some(MEME_CHIFFRE),
                    Some((l, c)) if l == ligne || c == col || (l / 3 == ligne / 3 && c / 3 == col / 3) => {
                        Some(LIGNE_COLONNE_BLOC)
                    }
                    _ => None,
                };
                if let Some(couleur) = fond {
                    peintre.rect_filled(rectangle, 0.0, couleur);
                }

                if valeur != 0 {
                    let couleur = if self.grille.est_fixe(ligne, col) {
                        CHIFFRE_DEPART
                    } else {
                        CHIFFRE_JOUEUR
                    };
                    peintre.text(
                        rectangle.center(),
                        Align2::CENTER_CENTER,
                        valeur,
                        FontId::proportional(case * 0.6),
                        couleur,
                    );
                }
            }
        }

        // Quadrillage : les traits qui séparent les blocs 3×3 sont plus épais
        for i in 0..=9 {
            let trait_ = if i % 3 == 0 {
                Stroke::new(3.0, TRAIT_EPAIS)
            } else {
                Stroke::new(1.0, TRAIT_FIN)
            };
            let decalage = i as f32 * case;
            peintre.line_segment(
                [
                    Pos2::new(zone.min.x + decalage, zone.min.y),
                    Pos2::new(zone.min.x + decalage, zone.max.y),
                ],
                trait_,
            );
            peintre.line_segment(
                [
                    Pos2::new(zone.min.x, zone.min.y + decalage),
                    Pos2::new(zone.max.x, zone.min.y + decalage),
                ],
                trait_,
            );
        }
    }
}

impl eframe::App for Application {
    fn ui(&mut self, ui: &mut egui::Ui, _cadre: &mut eframe::Frame) {
        self.recevoir_nouvelle_grille();
        let occupe = self.generation.is_some();
        let contexte = ui.ctx().clone();

        // Une action demandée par le clavier ou par les boutons : un chiffre à placer (0 = effacer)
        let mut chiffre_demande = None;
        if !occupe && !self.partie_gagnee() {
            chiffre_demande = self.lire_clavier(ui);
        }

        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(self.grille.nom.as_str());
            });
            ui.add_space(6.0);

            ui.add_enabled_ui(!occupe, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Nouvelle partie :");
                    for niveau in Niveau::TOUS {
                        if ui.button(niveau.libelle()).clicked() {
                            self.nouvelle_partie(niveau, &contexte);
                        }
                    }
                });
            });
            ui.add_space(8.0);

            // La grille est carrée : elle prend la place disponible en laissant de quoi
            // afficher les boutons en dessous
            let cote = ui
                .available_width()
                .min(ui.available_height() - 150.0)
                .max(180.0);
            ui.vertical_centered(|ui| self.dessiner_grille(ui, cote));
            ui.add_space(8.0);

            ui.add_enabled_ui(!occupe && !self.partie_gagnee(), |ui| {
                ui.horizontal_wrapped(|ui| {
                    for valeur in 1..=9u8 {
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
            });

            ui.add_space(8.0);
            if self.partie_gagnee() {
                ui.colored_label(Color32::from_rgb(40, 160, 70), "Bravo, grille terminée !");
            } else if !self.message.is_empty() {
                ui.label(self.message.as_str());
            } else {
                ui.label("Clic ou flèches pour choisir une case, puis un chiffre (0 ou Suppr pour effacer)");
            }
        });

        if let Some(valeur) = chiffre_demande {
            self.jouer_chiffre(valeur);
        }
    }
}
