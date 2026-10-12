//! La saisie d'une grille 9×9 (celle d'un journal, par exemple) et sa résolution par le solveur.

use crate::grille::Taille;
use crate::regles::Regles;
use crate::solveur::{analyser, Verdict};
use std::sync::Arc;

const COTE: usize = 9;

// Le nombre de chiffres qu'une grille doit avoir pour avoir une seule solution est d'au moins 17.
// En dessous, on résout quand même, mais on le dit.
const MINIMUM_POUR_UNE_SOLUTION_UNIQUE: usize = 17;

pub struct Resolveur {
    // Les règles de la grille (classique, 9×9) : le solveur et le dessin s'en servent
    regles: Arc<Regles>,
    // Les chiffres saisis, ligne par ligne (0 = case vide)
    saisie: Vec<u8>,
    // La solution, une fois la grille résolue
    solution: Option<Vec<u8>>,
    // La case choisie avec la souris ou les flèches : (ligne, col)
    pub selection: Option<(usize, usize)>,
    // Ce que le joueur doit savoir : le résultat de la résolution ou ce qui l'empêche
    pub message: String,
}

impl Resolveur {
    pub fn new() -> Resolveur {
        Resolveur {
            regles: Regles::classique(Taille::Neuf),
            saisie: vec![0; COTE * COTE],
            solution: None,
            selection: None,
            message: String::new(),
        }
    }

    #[cfg(test)]
    pub fn taille(&self) -> Taille {
        Taille::Neuf
    }

    pub fn regles(&self) -> &Arc<Regles> {
        &self.regles
    }

    // Le chiffre affiché dans une case : celui qui a été saisi, sinon celui de la solution
    pub fn valeur(&self, ligne: usize, col: usize) -> u8 {
        let position = ligne * COTE + col;
        match (&self.solution, self.saisie[position]) {
            (Some(solution), 0) => solution[position],
            (_, saisi) => saisi,
        }
    }

    // true si le chiffre de la case a été saisi (et non trouvé par le solveur)
    pub fn est_saisie(&self, ligne: usize, col: usize) -> bool {
        self.saisie[ligne * COTE + col] != 0
    }

    // true si la grille est résolue et affichée
    pub fn est_resolue(&self) -> bool {
        self.solution.is_some()
    }

    // Le nombre de chiffres saisis
    pub fn nombre_de_chiffres(&self) -> usize {
        self.saisie.iter().filter(|&&valeur| valeur != 0).count()
    }

    // Écrit un chiffre de 1 à 9 dans une case (0 la vide). Toute modification fait disparaître
    // la solution affichée : on revient à la saisie.
    pub fn placer(&mut self, ligne: usize, col: usize, valeur: u8) {
        if valeur as usize > COTE {
            return;
        }
        self.saisie[ligne * COTE + col] = valeur;
        self.solution = None;
        self.message.clear();
    }

    // Écrit un chiffre dans la case choisie puis passe à la suivante, pour saisir une grille
    // d'une traite (0 laisse la case vide et passe aussi à la suivante)
    pub fn saisir(&mut self, valeur: u8) {
        let Some((ligne, col)) = self.selection else {
            self.message = String::from("Cliquez d'abord sur une case");
            return;
        };
        self.placer(ligne, col, valeur);
        let suivante = ligne * COTE + col + 1;
        if suivante < COTE * COTE {
            self.selection = Some((suivante / COTE, suivante % COTE));
        }
    }

    // Vide la case choisie sans changer de case
    pub fn effacer_la_case(&mut self) {
        if let Some((ligne, col)) = self.selection {
            self.placer(ligne, col, 0);
        }
    }

    // Déplace la sélection d'une case vers le bas (descend) et vers la droite (droite),
    // sans sortir de la grille. Sans sélection, elle commence en haut à gauche.
    pub fn deplacer(&mut self, descend: i32, droite: i32) {
        let dernier = COTE as i32 - 1;
        self.selection = match self.selection {
            Some((ligne, col)) => Some((
                (ligne as i32 + descend).clamp(0, dernier) as usize,
                (col as i32 + droite).clamp(0, dernier) as usize,
            )),
            None => Some((0, 0)),
        };
    }

    // Efface les chiffres saisis et la solution
    pub fn tout_effacer(&mut self) {
        self.saisie = vec![0; COTE * COTE];
        self.solution = None;
        self.message.clear();
    }

    // Enlève la solution pour revenir aux seuls chiffres saisis
    pub fn revenir_a_la_saisie(&mut self) {
        self.solution = None;
        self.message.clear();
    }

    // Les chiffres saisis qui se contredisent : un même chiffre deux fois dans une ligne,
    // une colonne ou un bloc. Ils sont signalés en rouge.
    pub fn conflits(&self) -> Vec<bool> {
        let memes_zones = |a: usize, b: usize| {
            let (la, ca, lb, cb) = (a / COTE, a % COTE, b / COTE, b % COTE);
            la == lb || ca == cb || (la / 3 == lb / 3 && ca / 3 == cb / 3)
        };
        (0..COTE * COTE)
            .map(|a| {
                self.saisie[a] != 0
                    && (0..COTE * COTE)
                        .any(|b| a != b && self.saisie[a] == self.saisie[b] && memes_zones(a, b))
            })
            .collect()
    }

    // Remplace la saisie par une grille collée en texte : 81 caractères, des chiffres de 1 à 9
    // et 0 ou . pour une case vide. Les espaces, retours à la ligne et traits de dessin
    // (| + -) sont ignorés, pour accepter les grilles copiées depuis un site.
    pub fn coller(&mut self, texte: &str) -> Result<(), String> {
        let mut cases = Vec::new();
        for caractere in texte.chars() {
            match caractere {
                '1'..='9' => cases.push(caractere as u8 - b'0'),
                '0' | '.' => cases.push(0),
                c if c.is_whitespace() || "|+-=".contains(c) => {}
                autre => return Err(format!("Caractère inattendu dans la grille collée : '{}'", autre)),
            }
        }
        if cases.len() != COTE * COTE {
            return Err(format!(
                "La grille collée doit avoir 81 cases, elle en a {}",
                cases.len()
            ));
        }
        self.saisie = cases;
        self.solution = None;
        self.message = String::from("Grille collée");
        Ok(())
    }

    // Résout la grille saisie. Renvoie false, avec la raison dans `message`, si la grille
    // ne peut pas être résolue ; sinon la solution est affichée et `message` dit si elle est unique.
    pub fn resoudre(&mut self) -> bool {
        self.solution = None;
        let nombre = self.nombre_de_chiffres();
        if nombre == 0 {
            self.message = String::from("Saisissez d'abord les chiffres de la grille");
            return false;
        }
        if self.conflits().contains(&true) {
            self.message =
                String::from("Des chiffres se contredisent (en rouge) : corrigez-les avant de résoudre");
            return false;
        }

        let verdict = analyser(&self.regles, &self.saisie);
        let (solution, message) = match verdict {
            // Les conflits ont déjà été écartés : le solveur ne peut pas refuser la grille
            None => (None, String::from("Grille invalide")),
            Some(Verdict::Aucune) => (
                None,
                String::from("Cette grille n'a pas de solution : vérifiez les chiffres saisis"),
            ),
            Some(Verdict::TropLong) => (
                None,
                String::from("La recherche est trop longue : vérifiez les chiffres saisis"),
            ),
            Some(Verdict::Unique(solution)) => (Some(solution), String::from("Solution unique trouvée")),
            Some(Verdict::Plusieurs(solution)) => (
                Some(solution),
                if nombre < MINIMUM_POUR_UNE_SOLUTION_UNIQUE {
                    format!(
                        "Cette grille a plusieurs solutions ({} chiffres, il en faut au moins {}) : en voici une",
                        nombre, MINIMUM_POUR_UNE_SOLUTION_UNIQUE
                    )
                } else {
                    String::from("Cette grille a plusieurs solutions : en voici une. Vérifiez les chiffres saisis")
                },
            ),
            Some(Verdict::NonVerifiee(solution)) => (
                Some(solution),
                String::from("Solution trouvée (il n'a pas été possible de vérifier qu'elle est unique)"),
            ),
        };
        self.solution = solution;
        self.message = message;
        self.solution.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // La grille d'entraînement en 81 caractères
    fn journal() -> &'static str {
        crate::grille::DEPART
    }

    // Une grille chargée depuis le texte du journal
    fn avec_le_journal() -> Resolveur {
        let mut resolveur = Resolveur::new();
        resolveur.coller(journal()).unwrap();
        resolveur
    }

    #[test]
    fn test_une_nouvelle_saisie_est_vide() {
        let resolveur = Resolveur::new();
        assert_eq!(resolveur.nombre_de_chiffres(), 0);
        assert!(!resolveur.est_resolue());
        assert_eq!(resolveur.valeur(4, 4), 0);
        assert_eq!(resolveur.taille(), Taille::Neuf);
    }

    #[test]
    fn test_placer_et_effacer() {
        let mut resolveur = Resolveur::new();
        resolveur.placer(2, 3, 7);
        assert_eq!(resolveur.valeur(2, 3), 7);
        assert!(resolveur.est_saisie(2, 3));
        assert_eq!(resolveur.nombre_de_chiffres(), 1);
        resolveur.placer(2, 3, 0);
        assert_eq!(resolveur.valeur(2, 3), 0);
        assert!(!resolveur.est_saisie(2, 3));
        // Un chiffre hors de 1 à 9 est ignoré
        resolveur.placer(2, 3, 10);
        assert_eq!(resolveur.valeur(2, 3), 0);
    }

    #[test]
    fn test_resoudre_une_grille_de_journal() {
        let mut resolveur = avec_le_journal();
        assert_eq!(resolveur.nombre_de_chiffres(), 30);
        assert!(resolveur.resoudre());
        assert!(resolveur.est_resolue());
        assert_eq!(resolveur.message, "Solution unique trouvée");
        // La première ligne de la solution connue
        let ligne: Vec<u8> = (0..9).map(|col| resolveur.valeur(0, col)).collect();
        assert_eq!(ligne, [5, 3, 4, 6, 7, 8, 9, 1, 2]);
        // Les chiffres saisis restent des chiffres saisis, les autres viennent du solveur
        assert!(resolveur.est_saisie(0, 0));
        assert!(!resolveur.est_saisie(0, 2));
        // Toutes les cases sont remplies
        assert!((0..81).all(|i| resolveur.valeur(i / 9, i % 9) != 0));
    }

    #[test]
    fn test_saisir_avance_de_case_en_case() {
        let mut resolveur = Resolveur::new();
        resolveur.saisir(5);
        assert!(resolveur.message.contains("Cliquez"));
        assert_eq!(resolveur.nombre_de_chiffres(), 0);

        resolveur.selection = Some((0, 7));
        resolveur.saisir(5);
        resolveur.saisir(0); // une case vide : on saute
        resolveur.saisir(3);
        // Après la fin d'une ligne, on passe au début de la suivante
        assert_eq!(resolveur.valeur(0, 7), 5);
        assert_eq!(resolveur.valeur(0, 8), 0);
        assert_eq!(resolveur.valeur(1, 0), 3);
        assert_eq!(resolveur.selection, Some((1, 1)));
    }

    #[test]
    fn test_saisir_a_la_derniere_case_reste_dessus() {
        let mut resolveur = Resolveur::new();
        resolveur.selection = Some((8, 8));
        resolveur.saisir(9);
        assert_eq!(resolveur.valeur(8, 8), 9);
        assert_eq!(resolveur.selection, Some((8, 8)));
    }

    #[test]
    fn test_effacer_la_case_ne_bouge_pas() {
        let mut resolveur = Resolveur::new();
        resolveur.selection = Some((3, 3));
        resolveur.saisir(4);
        resolveur.selection = Some((3, 3));
        resolveur.effacer_la_case();
        assert_eq!(resolveur.valeur(3, 3), 0);
        assert_eq!(resolveur.selection, Some((3, 3)));
        // Sans sélection, rien ne se passe
        resolveur.selection = None;
        resolveur.effacer_la_case();
    }

    #[test]
    fn test_deplacer_reste_dans_la_grille() {
        let mut resolveur = Resolveur::new();
        resolveur.deplacer(1, 0);
        assert_eq!(resolveur.selection, Some((0, 0)));
        resolveur.deplacer(-1, -1);
        assert_eq!(resolveur.selection, Some((0, 0)));
        resolveur.deplacer(20, 20);
        assert_eq!(resolveur.selection, Some((8, 8)));
        resolveur.deplacer(-1, 0);
        assert_eq!(resolveur.selection, Some((7, 8)));
    }

    #[test]
    fn test_modifier_la_grille_efface_la_solution() {
        let mut resolveur = avec_le_journal();
        assert!(resolveur.resoudre());
        resolveur.placer(0, 2, 4);
        assert!(!resolveur.est_resolue());
        assert!(resolveur.message.is_empty());
        // La case qui était remplie par le solveur et qu'on vient de saisir est maintenant saisie
        assert_eq!(resolveur.valeur(0, 2), 4);
        assert_eq!(resolveur.valeur(0, 3), 0);
    }

    #[test]
    fn test_revenir_a_la_saisie_et_tout_effacer() {
        let mut resolveur = avec_le_journal();
        assert!(resolveur.resoudre());
        resolveur.revenir_a_la_saisie();
        assert!(!resolveur.est_resolue());
        assert_eq!(resolveur.nombre_de_chiffres(), 30);
        resolveur.tout_effacer();
        assert_eq!(resolveur.nombre_de_chiffres(), 0);
        assert!(resolveur.message.is_empty());
    }

    #[test]
    fn test_les_conflits_sont_signales() {
        let mut resolveur = Resolveur::new();
        resolveur.placer(0, 0, 5);
        resolveur.placer(0, 8, 5); // même ligne
        resolveur.placer(8, 0, 5); // même colonne que la première
        resolveur.placer(1, 1, 5); // même bloc que la première
        resolveur.placer(4, 4, 5); // aucun conflit
        resolveur.placer(5, 5, 6); // aucun conflit
        let conflits = resolveur.conflits();
        for (ligne, col, attendu) in [
            (0, 0, true),
            (0, 8, true),
            (8, 0, true),
            (1, 1, true),
            (4, 4, false),
            (5, 5, false),
            (3, 3, false),
        ] {
            assert_eq!(conflits[ligne * 9 + col], attendu, "case ({}, {})", ligne, col);
        }
    }

    #[test]
    fn test_pas_de_conflit_sur_une_grille_valide() {
        assert!(!avec_le_journal().conflits().contains(&true));
    }

    #[test]
    fn test_resoudre_refuse_une_grille_vide() {
        let mut resolveur = Resolveur::new();
        assert!(!resolveur.resoudre());
        assert!(resolveur.message.contains("Saisissez"));
    }

    #[test]
    fn test_resoudre_refuse_des_chiffres_qui_se_contredisent() {
        let mut resolveur = Resolveur::new();
        resolveur.placer(0, 0, 5);
        resolveur.placer(0, 1, 5);
        assert!(!resolveur.resoudre());
        assert!(!resolveur.est_resolue());
        assert!(resolveur.message.contains("se contredisent"));
    }

    #[test]
    fn test_resoudre_une_grille_sans_solution() {
        let mut resolveur = Resolveur::new();
        for col in 0..8 {
            resolveur.placer(0, col, col as u8 + 1);
        }
        resolveur.placer(1, 8, 9);
        assert!(!resolveur.resoudre());
        assert!(resolveur.message.contains("pas de solution"));
        assert!(!resolveur.est_resolue());
    }

    #[test]
    fn test_resoudre_une_grille_a_plusieurs_solutions() {
        let mut resolveur = Resolveur::new();
        resolveur.placer(0, 0, 1);
        assert!(resolveur.resoudre());
        assert!(resolveur.est_resolue());
        assert!(resolveur.message.contains("plusieurs solutions"));
        assert!(resolveur.message.contains("au moins 17"));
        assert!((0..81).all(|i| resolveur.valeur(i / 9, i % 9) != 0));
    }

    #[test]
    fn test_coller_accepte_plusieurs_ecritures() {
        // Des points pour les cases vides, des retours à la ligne et un cadre dessiné
        let joli = "\
+-------+-------+-------+
| 5 3 . | . 7 . | . . . |
| 6 . . | 1 9 5 | . . . |
| . 9 8 | . . . | . 6 . |
+-------+-------+-------+
| 8 . . | . 6 . | . . 3 |
| 4 . . | 8 . 3 | . . 1 |
| 7 . . | . 2 . | . . 6 |
+-------+-------+-------+
| . 6 . | . . . | 2 8 . |
| . . . | 4 1 9 | . . 5 |
| . . . | . 8 . | . 7 9 |
+-------+-------+-------+";
        let mut resolveur = Resolveur::new();
        resolveur.coller(joli).unwrap();
        assert_eq!(resolveur.nombre_de_chiffres(), 30);
        assert_eq!(resolveur.valeur(0, 0), 5);
        assert_eq!(resolveur.valeur(0, 2), 0);
        assert_eq!(resolveur.valeur(8, 8), 9);

        let mut autre = avec_le_journal();
        autre.tout_effacer();
        autre.coller(journal()).unwrap();
        for i in 0..81 {
            assert_eq!(autre.valeur(i / 9, i % 9), resolveur.valeur(i / 9, i % 9));
        }
    }

    #[test]
    fn test_coller_refuse_un_mauvais_texte() {
        let mut resolveur = Resolveur::new();
        resolveur.placer(0, 0, 3);
        let trop_court = resolveur.coller("123");
        assert!(trop_court.unwrap_err().contains("81 cases, elle en a 3"));
        let trop_long = resolveur.coller(&"1".repeat(82));
        assert!(trop_long.unwrap_err().contains("elle en a 82"));
        let lettre = resolveur.coller(&"a".repeat(81));
        assert!(lettre.unwrap_err().contains("'a'"));
        // Un collage refusé ne touche pas à la saisie
        assert_eq!(resolveur.valeur(0, 0), 3);
        assert_eq!(resolveur.nombre_de_chiffres(), 1);
    }

    #[test]
    fn test_coller_efface_la_solution() {
        let mut resolveur = avec_le_journal();
        assert!(resolveur.resoudre());
        resolveur.coller(journal()).unwrap();
        assert!(!resolveur.est_resolue());
    }
}
