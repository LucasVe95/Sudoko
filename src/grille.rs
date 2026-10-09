//! La grille de Sudoku : règles, chargement, résolution et génération, pour plusieurs tailles.

use crate::solveur::Solveur;
use rand::seq::{IndexedRandom, SliceRandom};

// La taille d'une grille : son côté, et la forme des blocs qui la découpent.
// Les blocs ne sont pas toujours carrés : en 6×6 ils font 2 lignes sur 3 colonnes.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Taille {
    Quatre,
    Six,
    Neuf,
    Douze,
    Seize,
}

impl Taille {
    pub const TOUTES: [Taille; 5] = [
        Taille::Quatre,
        Taille::Six,
        Taille::Neuf,
        Taille::Douze,
        Taille::Seize,
    ];

    // Le nombre de lignes (et de colonnes) : aussi le plus grand chiffre de la grille
    pub fn cote(self) -> usize {
        match self {
            Taille::Quatre => 4,
            Taille::Six => 6,
            Taille::Neuf => 9,
            Taille::Douze => 12,
            Taille::Seize => 16,
        }
    }

    // Le nombre de lignes d'un bloc
    pub fn bloc_h(self) -> usize {
        match self {
            Taille::Quatre | Taille::Six => 2,
            Taille::Neuf | Taille::Douze => 3,
            Taille::Seize => 4,
        }
    }

    // Le nombre de colonnes d'un bloc
    pub fn bloc_l(self) -> usize {
        match self {
            Taille::Quatre => 2,
            Taille::Six | Taille::Neuf => 3,
            Taille::Douze | Taille::Seize => 4,
        }
    }

    // Le nombre total de cases
    pub fn cases(self) -> usize {
        self.cote() * self.cote()
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Taille::Quatre => "4×4",
            Taille::Six => "6×6",
            Taille::Neuf => "9×9",
            Taille::Douze => "12×12",
            Taille::Seize => "16×16",
        }
    }

    // Retrouve une taille à partir de son côté (utilisé pour la ligne de commande
    // et pour relire le classement)
    pub fn depuis_cote(cote: usize) -> Option<Taille> {
        Taille::TOUTES.into_iter().find(|taille| taille.cote() == cote)
    }
}

// Niveau de difficulté d'une grille générée
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Niveau {
    Facile,
    Moyen,
    Difficile,
}

impl Niveau {
    pub const TOUS: [Niveau; 3] = [Niveau::Facile, Niveau::Moyen, Niveau::Difficile];

    pub fn libelle(self) -> &'static str {
        match self {
            Niveau::Facile => "Facile",
            Niveau::Moyen => "Moyen",
            Niveau::Difficile => "Difficile",
        }
    }

    // Retrouve un niveau à partir de son libellé (utilisé pour relire le classement)
    pub fn depuis_libelle(libelle: &str) -> Option<Niveau> {
        Niveau::TOUS
            .into_iter()
            .find(|niveau| niveau.libelle() == libelle)
    }

    // Plus on retire de cases, plus la grille est difficile : 43 %, 56 % ou 68 % des cases.
    // En 9×9, cela donne 35, 45 et 55 cases vides.
    fn cases_a_retirer(self, taille: Taille) -> usize {
        let pourcentage = match self {
            Niveau::Facile => 43,
            Niveau::Moyen => 56,
            Niveau::Difficile => 68,
        };
        (taille.cases() * pourcentage + 50) / 100
    }
}

// D'où vient la grille : générée au hasard (niveau et taille), ou la grille fixe d'entraînement
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Origine {
    Niveau(Niveau, Taille),
    Fixe,
}

#[derive(Clone)]
pub struct Grille {
    taille: Taille,
    // Les cases ligne par ligne : la case (ligne, col) est à l'indice ligne × côté + col
    cases: Vec<u8>,
    // true pour les cases de la grille de départ, que le joueur ne peut pas modifier
    fixes: Vec<bool>,
    // La solution, quand on la connaît : elle sert à dire si un chiffre du joueur est correct
    solution: Option<Vec<u8>>,
    pub nom: String,
}
impl Grille {
    //"Constructeur" de la structure Grille : une grille vide
    fn nouvelle(taille: Taille, nom: String) -> Grille {
        Grille {
            taille,
            cases: vec![0; taille.cases()],
            fixes: vec![false; taille.cases()],
            solution: None,
            nom,
        }
    }

    pub fn taille(&self) -> Taille {
        self.taille
    }

    // L'indice d'une case dans les tableaux `cases` et `fixes`
    fn position(&self, ligne: usize, col: usize) -> usize {
        ligne * self.taille.cote() + col
    }

    // Crée la grille correspondant à l'origine demandée
    pub fn creer(origine: Origine) -> Result<Grille, String> {
        match origine {
            Origine::Fixe => Grille::depuis_texte(String::from("Grille d'entraînement"), DEPART),
            Origine::Niveau(niveau, taille) => Ok(Grille::generer_niveau(niveau, taille)),
        }
    }

    // Construit une grille 9×9 à partir d'un texte de 81 chiffres (0 = case vide)
    pub fn depuis_texte(nom: String, texte: &str) -> Result<Grille, String> {
        // On garde tous les caractères sauf les espaces et retours à la ligne
        let caracteres: Vec<char> = texte.chars().filter(|c| !c.is_whitespace()).collect();

        if caracteres.len() != 81 {
            return Err(format!(
                "81 chiffres attendus, {} reçus",
                caracteres.len()
            ));
        }

        let mut grille = Grille::nouvelle(Taille::Neuf, nom);
        for (indice, c) in caracteres.iter().enumerate() {
            let valeur = match c.to_digit(10) {
                Some(v) => v as u8,
                None => return Err(format!("Caractère invalide : '{}'", c)),
            };

            // Une case à 0 reste vide, on ne place que les vrais chiffres
            if valeur != 0 {
                let ligne = indice / 9;
                let col = indice % 9;
                if !grille.placer(ligne, col, valeur) {
                    return Err(format!(
                        "{} en ligne {}, colonne {} brise les règles",
                        valeur,
                        ligne + 1,
                        col + 1
                    ));
                }
                // Les chiffres du texte de départ sont verrouillés
                let position = grille.position(ligne, col);
                grille.fixes[position] = true;
            }
        }

        // On garde la solution pour pouvoir dire si un chiffre du joueur est correct.
        // Si la grille a plusieurs solutions, on retient la première trouvée.
        let mut copie = grille.clone();
        if copie.remplir_solution() {
            grille.solution = Some(copie.cases);
        }

        Ok(grille)
    }

    //methode qui lit seulement :&self
    pub fn afficher(&self) {
        let cote = self.taille.cote();
        let (bloc_h, bloc_l) = (self.taille.bloc_h(), self.taille.bloc_l());
        // Chaque case occupe 2 caractères dès qu'il y a des nombres à deux chiffres
        let largeur = if cote >= 10 { 2 } else { 1 };

        // Le trait horizontal entre les blocs : le premier segment couvre les cases d'un bloc,
        // les suivants couvrent aussi l'espace qui suit la barre verticale
        let segment = bloc_l * (largeur + 1);
        let mut segments = vec!["-".repeat(segment)];
        for _ in 1..cote / bloc_l {
            segments.push("-".repeat(segment + 1));
        }
        let separateur = segments.join("+");

        for ligne in 0..cote {
            // Séparateur horizontal entre les blocs
            if ligne % bloc_h == 0 && ligne != 0 {
                println!("{}", separateur);
            }
            for col in 0..cote {
                // Séparateur vertical entre les blocs
                if col % bloc_l == 0 && col != 0 {
                    print!("| ");
                }
                // Une case vide (0) s'affiche avec un point
                match self.valeur(ligne, col) {
                    0 => print!("{:>largeur$} ", "."),
                    valeur => print!("{:>largeur$} ", valeur),
                }
            }
            println!();
        }
    }

    // Vérifie si on peut mettre `valeur` dans la case (ligne, col) sans briser les règles
    fn est_valide(&self, ligne: usize, col: usize, valeur: u8) -> bool {
        // Vérifie la ligne et la colonne en même temps
        for i in 0..self.taille.cote() {
            if self.valeur(ligne, i) == valeur || self.valeur(i, col) == valeur {
                return false;
            }
        }

        // Coin en haut à gauche du bloc qui contient la case
        let (bloc_h, bloc_l) = (self.taille.bloc_h(), self.taille.bloc_l());
        let debut_ligne = (ligne / bloc_h) * bloc_h;
        let debut_col = (col / bloc_l) * bloc_l;
        for i in 0..bloc_h {
            for j in 0..bloc_l {
                if self.valeur(debut_ligne + i, debut_col + j) == valeur {
                    return false;
                }
            }
        }

        true
    }

    //methode qui modifie : &mut self
    // Renvoie true si la valeur a été placée, false si le coup est refusé
    pub fn placer(&mut self, ligne: usize, col: usize, valeur: u8) -> bool {
        let position = self.position(ligne, col);
        // Un chiffre doit aller de 1 jusqu'au côté de la grille
        if valeur == 0
            || valeur as usize > self.taille.cote()
            || self.fixes[position]
            || !self.est_valide(ligne, col, valeur)
        {
            return false;
        }
        self.cases[position] = valeur;
        true
    }

    // Vide une case. Renvoie false si c'est une case de départ
    pub fn effacer(&mut self, ligne: usize, col: usize) -> bool {
        let position = self.position(ligne, col);
        if self.fixes[position] {
            return false;
        }
        self.cases[position] = 0;
        true
    }

    // true si `valeur` est le chiffre de la solution pour cette case.
    // Si la solution n'est pas connue, on ne peut pas juger : on répond true.
    pub fn est_correct(&self, ligne: usize, col: usize, valeur: u8) -> bool {
        match &self.solution {
            Some(solution) => solution[self.position(ligne, col)] == valeur,
            None => true,
        }
    }

    // Valeur d'une case : 0 si elle est vide
    pub fn valeur(&self, ligne: usize, col: usize) -> u8 {
        self.cases[self.position(ligne, col)]
    }

    // true si la case fait partie de la grille de départ
    pub fn est_fixe(&self, ligne: usize, col: usize) -> bool {
        self.fixes[self.position(ligne, col)]
    }

    // Efface tous les coups du joueur puis remplit la grille avec la solution.
    // Renvoie false si la grille de départ n'a pas de solution.
    pub fn remplir_solution(&mut self) -> bool {
        // Si la solution est connue, il n'y a rien à calculer : on la recopie
        if let Some(solution) = &self.solution {
            self.cases = solution.clone();
            return true;
        }

        // Sinon, on repart des seules cases de départ, pour qu'un mauvais coup du joueur
        // ne gêne pas, et on cherche une solution
        let cote = self.taille.cote();
        for ligne in 0..cote {
            for col in 0..cote {
                self.effacer(ligne, col);
            }
        }
        self.resoudre()
    }

    // La grille est gagnée quand toutes les cases sont remplies.
    // Chaque chiffre ayant été vérifié par est_valide, il n'y a rien d'autre à contrôler.
    pub fn est_terminee(&self) -> bool {
        self.cases.iter().all(|&case| case != 0)
    }

    // Remplit une case vide choisie au hasard avec la bonne valeur.
    // Renvoie (ligne, col, valeur), ou None s'il n'y a rien à révéler : grille déjà pleine,
    // ou coups du joueur qui mènent à une impasse (la grille n'est alors pas modifiée).
    pub fn indice(&mut self) -> Option<(usize, usize, u8)> {
        // La solution est connue pour les grilles générées et pour la grille d'entraînement.
        // Sinon, on résout une copie : la grille du joueur reste intacte si la résolution échoue.
        let solution = match &self.solution {
            Some(solution) => solution.clone(),
            None => {
                let mut copie = self.clone();
                if !copie.resoudre() {
                    return None;
                }
                copie.cases
            }
        };

        // Un chiffre du joueur qui n'est pas celui de la solution : impasse
        if self
            .cases
            .iter()
            .zip(&solution)
            .any(|(&case, &bonne)| case != 0 && case != bonne)
        {
            return None;
        }

        let vides: Vec<usize> = (0..self.cases.len())
            .filter(|&position| self.cases[position] == 0)
            .collect();
        let &position = vides.choose(&mut rand::rng())?;

        let cote = self.taille.cote();
        let valeur = solution[position];
        self.cases[position] = valeur;
        Some((position / cote, position % cote, valeur))
    }

    // Prépare le solveur sur la grille actuelle. None si elle contient un doublon.
    fn solveur(&self) -> Option<Solveur> {
        Solveur::new(
            self.taille.cote(),
            self.taille.bloc_h(),
            self.taille.bloc_l(),
            &self.cases,
        )
    }

    // Remplit la grille par retour sur trace (backtracking).
    // Renvoie true si une solution a été trouvée, false sinon (la grille est alors inchangée).
    pub fn resoudre(&mut self) -> bool {
        let Some(mut solveur) = self.solveur() else {
            return false;
        };
        // Les chiffres sont essayés dans l'ordre croissant : on ne les mélange pas
        if !solveur.resoudre(&mut |_| {}) {
            return false;
        }
        self.cases = solveur.cases().to_vec();
        true
    }

    // Remplit la grille au hasard : les chiffres sont essayés dans un ordre mélangé.
    // Si la recherche s'enlise (un mauvais choix tôt dans une grande grille peut la rendre
    // interminable), on abandonne cet essai et on recommence avec un autre mélange.
    fn remplir_au_hasard(&mut self, rng: &mut impl rand::Rng) -> bool {
        loop {
            let Some(solveur) = self.solveur() else {
                return false;
            };
            let mut solveur = solveur.avec_budget(BUDGET_REMPLISSAGE);
            if solveur.resoudre(&mut |chiffres| chiffres.shuffle(&mut *rng)) {
                self.cases = solveur.cases().to_vec();
                return true;
            }
            if !solveur.a_abandonne() {
                // La recherche est allée au bout : la grille n'a vraiment pas de solution
                return false;
            }
        }
    }

    // Compte les solutions de la grille, en s'arrêtant dès qu'on atteint `limite`.
    // Sans budget : réservé aux tests, où les grilles sont petites ou presque complètes.
    #[cfg(test)]
    fn compter_solutions(&self, limite: usize) -> usize {
        match self.solveur() {
            Some(mut solveur) => solveur.compter(limite),
            None => 0,
        }
    }

    // true si la grille a exactement une solution, prouvé dans la limite d'un budget.
    // Si la preuve est trop longue, on répond false : la génération garde alors la case
    // au lieu de risquer une attente interminable.
    fn est_unique_prouve(&self) -> bool {
        let Some(solveur) = self.solveur() else {
            return false;
        };
        let mut solveur = solveur.avec_budget(BUDGET_UNICITE);
        let solutions = solveur.compter(2);
        !solveur.a_abandonne() && solutions == 1
    }

    // Génère une grille du niveau et de la taille demandés
    pub fn generer_niveau(niveau: Niveau, taille: Taille) -> Grille {
        let nom = format!(
            "Partie {} {}",
            niveau.libelle().to_lowercase(),
            taille.libelle()
        );
        Grille::generer(taille, nom, niveau.cases_a_retirer(taille))
    }

    // Génère une grille à solution unique en retirant `a_retirer` cases (au plus) d'une
    // grille complète tirée au hasard. Plus on en retire, plus la grille est difficile.
    pub fn generer(taille: Taille, nom: String, a_retirer: usize) -> Grille {
        let mut rng = rand::rng();
        let mut grille = Grille::nouvelle(taille, nom);
        grille.remplir_au_hasard(&mut rng);
        // La grille complète est la solution : on la retient avant de retirer des cases
        let solution = grille.cases.clone();

        // Toutes les positions, dans un ordre aléatoire
        let cote = taille.cote();
        let mut positions: Vec<usize> = (0..taille.cases()).collect();
        positions.shuffle(&mut rng);

        let mut retirees = 0;
        for position in positions {
            if retirees == a_retirer {
                break;
            }
            let valeur = grille.cases[position];
            grille.cases[position] = 0;
            // On ne garde le retrait que si la grille garde une solution unique, prouvée
            if grille.est_unique_prouve() {
                retirees += 1;
            } else {
                grille.cases[position] = valeur;
            }
        }

        // Les chiffres restants forment la grille de départ : on les verrouille
        for position in 0..cote * cote {
            grille.fixes[position] = grille.cases[position] != 0;
        }
        grille.solution = Some(solution);
        grille
    }
}

// Le nombre maximal d'étapes de recherche pour remplir une grille au hasard (un essai),
// et pour prouver qu'une grille n'a qu'une solution. Au-delà, on abandonne.
const BUDGET_REMPLISSAGE: u64 = 20_000;
const BUDGET_UNICITE: u64 = 5_000;

// Grille de départ : 81 chiffres, 0 = case vide
pub const DEPART: &str = "530070000\
                      600195000\
                      098000060\
                      800060003\
                      400803001\
                      700020006\
                      060000280\
                      000419005\
                      000080079";

#[cfg(test)]
mod tests {
    use super::*;

    // Une grille vide 9×9
    fn vide() -> Grille {
        Grille::nouvelle(Taille::Neuf, String::from("Test"))
    }

    // La grille d'entraînement
    fn depart() -> Grille {
        Grille::depuis_texte(String::from("Test"), DEPART).unwrap()
    }

    // Le nombre de cases vides
    fn compter_vides(grille: &Grille) -> usize {
        grille.cases.iter().filter(|&&case| case == 0).count()
    }

    // Chaque ligne, colonne et bloc contient tous les chiffres exactement une fois
    fn est_grille_valide(grille: &Grille) -> bool {
        let cote = grille.taille().cote();
        let (bloc_h, bloc_l) = (grille.taille().bloc_h(), grille.taille().bloc_l());
        let attendu = (1u32 << cote) - 1;
        let mut lignes = vec![0u32; cote];
        let mut colonnes = vec![0u32; cote];
        let mut blocs = vec![0u32; cote];
        for ligne in 0..cote {
            for col in 0..cote {
                let valeur = grille.valeur(ligne, col);
                if valeur == 0 || valeur as usize > cote {
                    return false;
                }
                let bit = 1u32 << (valeur - 1);
                lignes[ligne] |= bit;
                colonnes[col] |= bit;
                blocs[(ligne / bloc_h) * (cote / bloc_l) + col / bloc_l] |= bit;
            }
        }
        lignes
            .iter()
            .chain(&colonnes)
            .chain(&blocs)
            .all(|&masque| masque == attendu)
    }

    #[test]
    fn test_placer() {
        let mut g = vide();
        assert!(g.placer(0, 0, 5));
        assert_eq!(g.valeur(0, 0), 5);
    }

    #[test]
    fn test_refuse_doublon_ligne() {
        let mut g = vide();
        g.placer(0, 0, 5);
        assert!(!g.est_valide(0, 8, 5));
    }

    #[test]
    fn test_refuse_doublon_colonne() {
        let mut g = vide();
        g.placer(0, 0, 5);
        assert!(!g.est_valide(8, 0, 5));
    }

    #[test]
    fn test_refuse_doublon_bloc() {
        let mut g = vide();
        g.placer(0, 0, 5);
        assert!(!g.est_valide(2, 2, 5));
    }

    #[test]
    fn test_accepte_coup_valide() {
        let mut g = vide();
        g.placer(0, 0, 5);
        // Autre ligne, autre colonne, autre bloc
        assert!(g.est_valide(4, 4, 5));
    }

    #[test]
    fn test_depuis_texte_valide() {
        let texte = format!("53{}", "0".repeat(79));
        let g = Grille::depuis_texte(String::from("Test"), &texte).unwrap();
        assert_eq!(g.valeur(0, 0), 5);
        assert_eq!(g.valeur(0, 1), 3);
        assert_eq!(g.valeur(0, 2), 0);
    }

    #[test]
    fn test_depuis_texte_mauvaise_longueur() {
        assert!(Grille::depuis_texte(String::from("Test"), "123").is_err());
    }

    #[test]
    fn test_depuis_texte_caractere_invalide() {
        let texte = format!("x{}", "0".repeat(80));
        assert!(Grille::depuis_texte(String::from("Test"), &texte).is_err());
    }

    #[test]
    fn test_depuis_texte_regles_brisees() {
        // Deux 5 sur la première ligne
        let texte = format!("55{}", "0".repeat(79));
        assert!(Grille::depuis_texte(String::from("Test"), &texte).is_err());
    }

    const SOLUTION: &str = "534678912672195348198342567859761423\
                            426853791713924856961537284287419635\
                            345286179";

    #[test]
    fn test_case_de_depart_verrouillee() {
        let mut g = depart();
        // (0, 0) vaut 5 au départ : ni modifiable, ni effaçable
        assert!(!g.placer(0, 0, 9));
        assert!(!g.effacer(0, 0));
        assert_eq!(g.valeur(0, 0), 5);
    }

    #[test]
    fn test_effacer_case_du_joueur() {
        let mut g = depart();
        // (0, 2) est vide au départ, la solution y met 4
        assert!(g.placer(0, 2, 4));
        assert!(g.effacer(0, 2));
        assert_eq!(g.valeur(0, 2), 0);
    }

    #[test]
    fn test_est_terminee() {
        assert!(!depart().est_terminee());

        let complete = Grille::depuis_texte(String::from("Test"), SOLUTION).unwrap();
        assert!(complete.est_terminee());
    }

    #[test]
    fn test_resoudre_trouve_la_solution() {
        let mut g = depart();
        assert!(g.resoudre());
        let attendue = Grille::depuis_texte(String::from("Test"), SOLUTION).unwrap();
        assert_eq!(g.cases, attendue.cases);
    }

    #[test]
    fn test_resoudre_grille_vide() {
        let mut g = vide();
        assert!(g.resoudre());
        assert!(g.est_terminee());
    }

    #[test]
    fn test_resoudre_grille_impossible() {
        // La case (0, 8) ne peut contenir ni 1-8 (ligne) ni 9 (colonne)
        let texte = format!("123456780000000009{}", "0".repeat(63));
        let mut g = Grille::depuis_texte(String::from("Test"), &texte).unwrap();
        assert!(!g.resoudre());
        assert_eq!(g.valeur(0, 8), 0);
    }

    #[test]
    fn test_compter_solutions() {
        // Grille vide : beaucoup de solutions, on s'arrête à la limite
        assert_eq!(vide().compter_solutions(2), 2);

        // La grille de départ n'a qu'une solution
        let g = depart();
        assert_eq!(g.compter_solutions(2), 1);

        // La grille n'a pas changé
        assert_eq!(g.cases, depart().cases);
    }

    #[test]
    fn test_generer_solution_unique() {
        let g = Grille::generer(Taille::Neuf, String::from("Test"), 40);
        let vides = compter_vides(&g);
        assert!(vides > 0 && vides <= 40);
        assert_eq!(g.compter_solutions(2), 1);
    }

    #[test]
    fn test_generer_verrouille_les_cases_de_depart() {
        let g = Grille::generer(Taille::Neuf, String::from("Test"), 40);
        for ligne in 0..9 {
            for col in 0..9 {
                assert_eq!(g.est_fixe(ligne, col), g.valeur(ligne, col) != 0);
            }
        }
    }

    #[test]
    fn test_generer_est_resoluble() {
        let mut g = Grille::generer(Taille::Neuf, String::from("Test"), 40);
        assert!(g.resoudre());
        assert!(g.est_terminee());
    }

    #[test]
    fn test_generer_donne_des_grilles_differentes() {
        let a = Grille::generer(Taille::Neuf, String::from("A"), 40);
        let b = Grille::generer(Taille::Neuf, String::from("B"), 40);
        assert_ne!(a.cases, b.cases);
    }

    #[test]
    fn test_indice_revele_la_bonne_valeur() {
        let mut g = depart();
        let solution = Grille::depuis_texte(String::from("Test"), SOLUTION).unwrap();

        let (ligne, col, valeur) = g.indice().unwrap();
        assert_eq!(g.valeur(ligne, col), valeur);
        assert_eq!(valeur, solution.valeur(ligne, col));
        // 51 cases vides au départ, une de moins maintenant
        assert_eq!(compter_vides(&g), 50);
    }

    #[test]
    fn test_indices_successifs_finissent_la_grille() {
        let mut g = depart();
        while g.indice().is_some() {}
        assert!(g.est_terminee());
    }

    #[test]
    fn test_indice_grille_pleine() {
        let mut g = Grille::depuis_texte(String::from("Test"), SOLUTION).unwrap();
        assert!(g.indice().is_none());
    }

    #[test]
    fn test_indice_apres_coup_sans_issue() {
        let mut g = depart();
        // 1 respecte les règles en (0, 2), mais la solution y met 4 : c'est une impasse
        assert!(g.placer(0, 2, 1));
        let avant = g.cases.clone();
        assert!(g.indice().is_none());
        assert_eq!(g.cases, avant);
    }

    #[test]
    fn test_valeur_et_est_fixe() {
        let g = depart();
        assert_eq!(g.valeur(0, 0), 5);
        assert!(g.est_fixe(0, 0));
        assert_eq!(g.valeur(0, 2), 0);
        assert!(!g.est_fixe(0, 2));
    }

    #[test]
    fn test_remplir_solution_ignore_les_coups_du_joueur() {
        let mut g = depart();
        // Coup valide mais faux : la solution met 4 en (0, 2)
        assert!(g.placer(0, 2, 1));
        assert!(g.remplir_solution());
        let attendue = Grille::depuis_texte(String::from("Test"), SOLUTION).unwrap();
        assert_eq!(g.cases, attendue.cases);
    }

    #[test]
    fn test_generer_niveau() {
        let g = Grille::generer_niveau(Niveau::Facile, Taille::Neuf);
        assert_eq!(g.nom, "Partie facile 9×9");
        let vides = compter_vides(&g);
        assert!(vides > 0 && vides <= 35);
    }

    #[test]
    fn test_est_correct() {
        let g = depart();
        // La solution met 4 en (0, 2)
        assert!(g.est_correct(0, 2, 4));
        assert!(!g.est_correct(0, 2, 1));
        // Sans solution connue, on ne juge pas
        assert!(vide().est_correct(0, 0, 7));
    }

    #[test]
    fn test_generer_connait_sa_solution() {
        let mut g = Grille::generer(Taille::Neuf, String::from("Test"), 40);
        assert!(g.remplir_solution());
        for ligne in 0..9 {
            for col in 0..9 {
                assert!(g.est_correct(ligne, col, g.valeur(ligne, col)));
            }
        }
    }

    #[test]
    fn test_creer_selon_l_origine() {
        let fixe = Grille::creer(Origine::Fixe).unwrap();
        assert_eq!(fixe.valeur(0, 0), 5);
        let facile = Grille::creer(Origine::Niveau(Niveau::Facile, Taille::Neuf)).unwrap();
        assert_eq!(facile.nom, "Partie facile 9×9");
        let petite = Grille::creer(Origine::Niveau(Niveau::Moyen, Taille::Quatre)).unwrap();
        assert_eq!(petite.taille(), Taille::Quatre);
    }

    #[test]
    fn test_placer_refuse_coup_invalide() {
        let mut g = vide();
        g.placer(0, 0, 5);
        assert!(!g.placer(0, 1, 5));
        assert_eq!(g.valeur(0, 1), 0);
    }

    // --- Les différentes tailles ---

    #[test]
    fn test_dimensions_des_tailles() {
        for taille in Taille::TOUTES {
            // Les blocs découpent exactement la grille
            assert_eq!(taille.bloc_h() * taille.bloc_l(), taille.cote(), "{:?}", taille);
            assert_eq!(taille.cote() % taille.bloc_h(), 0);
            assert_eq!(taille.cote() % taille.bloc_l(), 0);
            assert_eq!(taille.cases(), taille.cote() * taille.cote());
            assert_eq!(Taille::depuis_cote(taille.cote()), Some(taille));
        }
        assert_eq!(Taille::depuis_cote(10), None);
        assert_eq!(Taille::Seize.libelle(), "16×16");
    }

    #[test]
    fn test_cases_a_retirer_en_9x9_inchange() {
        assert_eq!(Niveau::Facile.cases_a_retirer(Taille::Neuf), 35);
        assert_eq!(Niveau::Moyen.cases_a_retirer(Taille::Neuf), 45);
        assert_eq!(Niveau::Difficile.cases_a_retirer(Taille::Neuf), 55);
    }

    #[test]
    fn test_cases_a_retirer_par_taille_comme_dans_le_readme() {
        let attendu = [
            (Taille::Quatre, [7, 9, 11]),
            (Taille::Six, [15, 20, 24]),
            (Taille::Neuf, [35, 45, 55]),
            (Taille::Douze, [62, 81, 98]),
            (Taille::Seize, [110, 143, 174]),
        ];
        for (taille, nombres) in attendu {
            for (niveau, nombre) in Niveau::TOUS.into_iter().zip(nombres) {
                assert_eq!(
                    niveau.cases_a_retirer(taille),
                    nombre,
                    "{} {}",
                    niveau.libelle(),
                    taille.libelle()
                );
            }
        }
    }

    #[test]
    fn test_plus_la_grille_est_grande_plus_il_y_a_de_cases_a_retirer() {
        for niveau in Niveau::TOUS {
            let nombres: Vec<usize> = Taille::TOUTES
                .iter()
                .map(|&taille| niveau.cases_a_retirer(taille))
                .collect();
            assert!(nombres.windows(2).all(|paire| paire[0] < paire[1]), "{:?}", nombres);
        }
    }

    #[test]
    fn test_resoudre_une_grille_vide_de_chaque_taille() {
        for taille in Taille::TOUTES {
            let mut g = Grille::nouvelle(taille, String::from("Test"));
            assert!(g.resoudre(), "{}", taille.libelle());
            assert!(est_grille_valide(&g), "{}", taille.libelle());
            assert!(g.est_terminee());
        }
    }

    #[test]
    fn test_generer_chaque_taille_a_solution_unique() {
        for taille in Taille::TOUTES {
            // On retire environ un quart des cases : assez pour tester l'unicité, sans être long
            let a_retirer = taille.cases() / 4;
            let mut g = Grille::generer(taille, String::from("Test"), a_retirer);
            let vides = compter_vides(&g);
            assert!(vides > 0 && vides <= a_retirer, "{}", taille.libelle());
            assert_eq!(g.compter_solutions(2), 1, "{}", taille.libelle());
            assert!(g.remplir_solution(), "{}", taille.libelle());
            assert!(est_grille_valide(&g), "{}", taille.libelle());
        }
    }

    #[test]
    fn test_generer_niveau_chaque_taille() {
        for taille in Taille::TOUTES {
            for niveau in [Niveau::Facile, Niveau::Moyen] {
                let g = Grille::generer_niveau(niveau, taille);
                assert_eq!(g.taille(), taille);
                let vides = compter_vides(&g);
                assert!(
                    vides > 0 && vides <= niveau.cases_a_retirer(taille),
                    "{} {}",
                    niveau.libelle(),
                    taille.libelle()
                );
            }
        }
    }

    #[test]
    fn test_les_blocs_rectangulaires_en_6x6() {
        // En 6×6, un bloc fait 2 lignes sur 3 colonnes
        let mut g = Grille::nouvelle(Taille::Six, String::from("Test"));
        assert!(g.placer(0, 0, 1));
        // Même bloc (lignes 0-1, colonnes 0-2) : refusé
        assert!(!g.est_valide(1, 2, 1));
        // Ligne 2 : un autre bloc, et ni la même ligne ni la même colonne : accepté
        assert!(g.est_valide(2, 2, 1));
        // Colonne 3 : un autre bloc, accepté sur la ligne 1
        assert!(g.est_valide(1, 3, 1));
    }

    #[test]
    fn test_les_blocs_rectangulaires_en_12x12() {
        // En 12×12, un bloc fait 3 lignes sur 4 colonnes
        let mut g = Grille::nouvelle(Taille::Douze, String::from("Test"));
        assert!(g.placer(0, 0, 7));
        assert!(!g.est_valide(2, 3, 7)); // même bloc
        assert!(g.est_valide(3, 3, 7)); // ligne 3 : un autre bloc
        assert!(g.est_valide(2, 4, 7)); // colonne 4 : un autre bloc
    }

    #[test]
    fn test_placer_refuse_les_chiffres_hors_de_la_grille() {
        let mut petite = Grille::nouvelle(Taille::Quatre, String::from("Test"));
        assert!(petite.placer(0, 0, 4));
        assert!(!petite.placer(0, 1, 5));
        assert!(!petite.placer(0, 1, 0));

        let mut grande = Grille::nouvelle(Taille::Seize, String::from("Test"));
        assert!(grande.placer(0, 0, 16));
        assert!(!grande.placer(0, 1, 17));
        // Les nombres à deux chiffres se placent comme les autres
        assert!(grande.placer(0, 1, 12));
        assert_eq!(grande.valeur(0, 1), 12);
    }

    #[test]
    fn test_indice_en_16x16() {
        let mut g = Grille::generer(Taille::Seize, String::from("Test"), 20);
        let (ligne, col, valeur) = g.indice().unwrap();
        assert_eq!(g.valeur(ligne, col), valeur);
        assert!(g.est_correct(ligne, col, valeur));
    }
}
