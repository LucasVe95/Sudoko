//! La grille de Sudoku : règles, chargement, résolution et génération, pour plusieurs tailles.

use crate::regles::{Regles, Variante};
use crate::solveur::Solveur;
use crate::technique::{technique_max, Logique, Technique};
use rand::seq::{IndexedRandom, SliceRandom};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
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

    // Le nombre de cases vides visé : 43 %, 56 % ou 68 % des cases. En 9×9, cela donne 35, 45 et 55.
    // Le niveau Facile s'arrête là ; pour les deux autres, c'est un minimum : on retire encore
    // des cases tant que la grille ne demande pas les techniques du niveau (voir `generer_niveau`).
    fn cases_a_retirer(self, taille: Taille) -> usize {
        let pourcentage = match self {
            Niveau::Facile => 43,
            Niveau::Moyen => 56,
            Niveau::Difficile => 68,
        };
        (taille.cases() * pourcentage + 50) / 100
    }
}

// D'où vient la grille : générée au hasard (niveau, taille et variante), ou la grille fixe
// d'entraînement
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Origine {
    Niveau(Niveau, Taille, Variante),
    Fixe,
}

#[derive(Clone)]
pub struct Grille {
    taille: Taille,
    // Les règles : quelles cases ne peuvent pas contenir le même chiffre
    regles: Arc<Regles>,
    // Les cases ligne par ligne : la case (ligne, col) est à l'indice ligne × côté + col
    cases: Vec<u8>,
    // true pour les cases de la grille de départ, que le joueur ne peut pas modifier
    fixes: Vec<bool>,
    // La solution, quand on la connaît : elle sert à dire si un chiffre du joueur est correct
    solution: Option<Vec<u8>>,
    pub nom: String,
}
impl Grille {
    //"Constructeur" de la structure Grille : une grille classique vide
    fn nouvelle(taille: Taille, nom: String) -> Grille {
        Grille::avec_regles(Regles::classique(taille), nom)
    }

    // Une grille vide qui suit ces règles
    fn avec_regles(regles: Arc<Regles>, nom: String) -> Grille {
        let taille = regles.taille;
        Grille {
            taille,
            regles,
            cases: vec![0; taille.cases()],
            fixes: vec![false; taille.cases()],
            solution: None,
            nom,
        }
    }

    pub fn taille(&self) -> Taille {
        self.taille
    }

    pub fn variante(&self) -> Variante {
        self.regles.variante
    }

    // Les règles de la grille (zones, voisines) : l'interface s'en sert pour la dessiner
    pub fn regles(&self) -> &Arc<Regles> {
        &self.regles
    }

    // Les cases ligne par ligne (0 = vide), pour enregistrer la partie
    pub fn cases(&self) -> &[u8] {
        &self.cases
    }

    // Les cases de départ, dans le même ordre que `cases`
    pub fn fixes(&self) -> &[bool] {
        &self.fixes
    }

    // La solution, quand on la connaît
    pub fn solution(&self) -> Option<&[u8]> {
        self.solution.as_deref()
    }

    // Reconstruit une grille à partir de ce qui a été enregistré. Le fichier a pu être abîmé
    // ou modifié à la main : on refuse tout ce qui romprait les règles de la grille,
    // plutôt que de planter plus tard.
    pub fn restaurer(
        taille: Taille,
        variante: Variante,
        nom: String,
        cases: Vec<u8>,
        fixes: Vec<bool>,
        solution: Option<Vec<u8>>,
    ) -> Result<Grille, String> {
        let nombre = taille.cases();
        if cases.len() != nombre || fixes.len() != nombre {
            return Err(format!("{} cases attendues", nombre));
        }
        // Pas de chiffre hors de la grille, et pas de doublon dans une ligne, une colonne ou un bloc
        // parmi les cases de départ. Les chiffres du joueur peuvent, eux, se contredire : il a le
        // droit de se tromper.
        let regles = Regles::pour(taille, variante);
        let cote = taille.cote();
        let depart: Vec<u8> = cases
            .iter()
            .zip(&fixes)
            .map(|(&valeur, &fixe)| if fixe { valeur } else { 0 })
            .collect();
        if cases.iter().any(|&valeur| valeur as usize > cote)
            || Solveur::new(&regles, &depart).is_none()
        {
            return Err(String::from("Cases invalides"));
        }
        // Une case de départ n'est jamais vide
        if cases.iter().zip(&fixes).any(|(&valeur, &fixe)| fixe && valeur == 0) {
            return Err(String::from("Case de départ vide"));
        }
        if let Some(solution) = &solution {
            // La solution est une grille pleine et valide, qui garde les cases de départ
            let complete = solution.len() == nombre
                && solution.iter().all(|&valeur| valeur != 0)
                && Solveur::new(&regles, solution).is_some();
            let garde_les_departs = cases
                .iter()
                .zip(&fixes)
                .zip(solution)
                .all(|((&valeur, &fixe), &attendu)| !fixe || valeur == attendu);
            if !complete || !garde_les_departs {
                return Err(String::from("Solution invalide"));
            }
        }

        Ok(Grille {
            taille,
            regles,
            cases,
            fixes,
            solution,
            nom,
        })
    }

    // L'indice d'une case dans les tableaux `cases` et `fixes`
    fn position(&self, ligne: usize, col: usize) -> usize {
        ligne * self.taille.cote() + col
    }

    // Crée la grille correspondant à l'origine demandée
    pub fn creer(origine: Origine) -> Result<Grille, String> {
        match origine {
            Origine::Fixe => Grille::depuis_texte(String::from("Grille d'entraînement"), DEPART),
            Origine::Niveau(niveau, taille, variante) => {
                Ok(Grille::generer_niveau(niveau, taille, variante))
            }
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

    // Vérifie si on peut mettre `valeur` dans la case (ligne, col) sans briser les règles :
    // le chiffre ne doit être ni dans la case, ni dans aucune des cases qui lui sont interdites
    // (sa ligne, sa colonne, son bloc, et selon la variante une diagonale, une fenêtre...)
    pub fn est_valide(&self, ligne: usize, col: usize, valeur: u8) -> bool {
        let case = self.position(ligne, col);
        self.cases[case] != valeur
            && self.regles.voisines[case].iter().all(|&autre| self.cases[autre] != valeur)
    }

    // Les chiffres que les règles permettent dans une case, en bits : le bit (v - 1) vaut 1
    // si le chiffre v est permis. Une case déjà remplie n'a aucun candidat.
    pub fn candidats(&self, ligne: usize, col: usize) -> u16 {
        if self.valeur(ligne, col) != 0 {
            return 0;
        }
        (1..=self.taille.cote() as u8)
            .filter(|&valeur| self.est_valide(ligne, col, valeur))
            .fold(0, |bits, valeur| bits | 1 << (valeur - 1))
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

    // Pose un chiffre sans vérifier les règles du sudoku : le joueur a le droit de se tromper, le
    // chiffre faux est alors affiché en rouge et peut être effacé. Renvoie false si la case est
    // une case de départ ou si le chiffre n'est pas dans la grille (de 1 au côté).
    pub fn poser(&mut self, ligne: usize, col: usize, valeur: u8) -> bool {
        let position = self.position(ligne, col);
        if valeur == 0 || valeur as usize > self.taille.cote() || self.fixes[position] {
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

    // La grille est gagnée quand toutes les cases sont remplies et que les règles sont respectées :
    // une grille pleine peut contenir des chiffres posés avec `poser`, qui se contredisent.
    pub fn est_terminee(&self) -> bool {
        self.cases.iter().all(|&case| case != 0) && self.solveur().is_some()
    }

    // Remplit une case vide avec la bonne valeur, et explique pourquoi c'est la bonne.
    // On choisit la case que la technique de déduction la plus simple permet de trouver
    // (voir technique.rs) ; si aucune déduction n'y mène, une case vide au hasard.
    // Renvoie None s'il n'y a rien à révéler : grille déjà pleine, ou coups du joueur qui mènent
    // à une impasse (la grille n'est alors pas modifiée).
    pub fn indice(&mut self) -> Option<Indice> {
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
        let (position, explication) = match self.indice_par_deduction(&solution) {
            Some(trouve) => trouve,
            None => {
                let &position = vides.choose(&mut rand::rng())?;
                let explication = String::from(
                    "Aucune technique de déduction ne mène à cette case : c'est le chiffre de la solution.",
                );
                (position, explication)
            }
        };

        let cote = self.taille.cote();
        let valeur = solution[position];
        self.cases[position] = valeur;
        Some(Indice {
            ligne: position / cote,
            col: position % cote,
            valeur,
            explication,
        })
    }

    // Cherche la prochaine case que la déduction permet de remplir, en partant des chiffres posés.
    // Renvoie (la case, l'explication). Les techniques qui ne font que retirer des possibilités
    // sont enchaînées jusqu'à ce qu'une case se remplisse.
    fn indice_par_deduction(&self, solution: &[u8]) -> Option<(usize, String)> {
        let regles = Arc::clone(&self.regles);
        let mut logique = Logique::new(&regles, &self.cases)?;
        let mut preparation: Vec<Technique> = Vec::new();
        while let Some(pas) = logique.prochain_pas(Technique::XWing) {
            if let Some((case, valeur)) = pas.placement {
                // Une déduction est sûre : si elle contredit la solution, mieux vaut ne rien en dire
                if solution[case] != valeur {
                    return None;
                }
                let mut explication = pas.explication(&regles);
                if !preparation.is_empty() {
                    let noms: Vec<&str> = preparation.iter().map(|technique| technique.libelle()).collect();
                    explication += &format!(
                        " Auparavant, il faut retirer des possibilités avec : {}.",
                        noms.join(", ")
                    );
                }
                return Some((case, explication));
            }
            if !preparation.contains(&pas.technique) {
                preparation.push(pas.technique);
            }
            logique.appliquer(&pas);
        }
        None
    }

    // Prépare le solveur sur la grille actuelle. None si elle contient un doublon.
    fn solveur(&self) -> Option<Solveur> {
        Solveur::new(&self.regles, &self.cases)
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
    #[cfg(test)]
    fn est_unique_prouve(&self) -> bool {
        let Some(solveur) = self.solveur() else {
            return false;
        };
        let mut solveur = solveur.avec_budget(BUDGET_UNICITE);
        let solutions = solveur.compter(2);
        !solveur.a_abandonne() && solutions == 1
    }

    // Génère une grille du niveau et de la taille demandés.
    //
    // La difficulté se mesure aux techniques de déduction qu'il faut employer pour finir la grille
    // (voir technique.rs), pas au nombre de cases vides :
    // - Facile : des cases uniques et des chiffres uniques seulement ;
    // - Moyen : il faut aussi des intersections ou des paires nues ;
    // - Difficile : il faut en plus des paires cachées, des triplets nus ou un X-Wing.
    // Dans tous les cas, la grille se finit sans jamais deviner : sa solution est unique.
    //
    // Dans les petites grilles (4×4 et 6×6), les techniques difficiles n'ont presque pas de place :
    // le niveau y dépend seulement du nombre de cases vides (la grille reste résoluble par déduction,
    // avec les techniques du niveau). Dans les autres, si aucun essai ne demande les techniques du
    // niveau au bout de quelques secondes, on garde la grille la plus difficile trouvée.
    pub fn generer_niveau(niveau: Niveau, taille: Taille, variante: Variante) -> Grille {
        let mut nom = format!(
            "Partie {} {}",
            niveau.libelle().to_lowercase(),
            taille.libelle()
        );
        if variante != Variante::Classique {
            nom += &format!(" {}", variante.libelle().to_lowercase());
        }
        let regles = Regles::pour(taille, variante);
        let mut rng = rand::rng();
        let debut = Instant::now();

        let mut meilleur: Option<Essai> = None;
        for _ in 0..ESSAIS_MAX {
            let essai = Grille::essayer_niveau(&regles, niveau, &mut rng);
            let conforme = essai.conforme;
            if meilleur.as_ref().is_none_or(|actuel| essai.score() > actuel.score()) {
                meilleur = Some(essai);
            }
            if conforme || debut.elapsed() > TEMPS_MAX_GENERATION {
                break;
            }
        }
        let essai = meilleur.expect("au moins un essai a été fait");

        // Les chiffres restants forment la grille de départ : on les verrouille
        let mut grille = Grille::avec_regles(regles, nom);
        grille.fixes = essai.cases.iter().map(|&valeur| valeur != 0).collect();
        grille.cases = essai.cases;
        grille.solution = Some(essai.solution);
        grille
    }

    // Un essai de génération : on part d'une grille complète tirée au hasard et on retire des cases
    // une à une, dans un ordre aléatoire. On ne garde un retrait que si la grille se résout encore
    // par déduction avec les techniques du niveau, ce qui prouve aussi que sa solution est unique.
    // On s'arrête quand le nombre de cases vides visé est atteint et que la grille demande bien
    // les techniques du niveau (sauf en 4×4 et 6×6, où le nombre de cases suffit), ou quand plus
    // aucune case ne peut être retirée.
    fn essayer_niveau(regles: &Arc<Regles>, niveau: Niveau, rng: &mut impl rand::Rng) -> Essai {
        let taille = regles.taille;
        let mut complete = Grille::avec_regles(Arc::clone(regles), String::new());
        complete.remplir_au_hasard(rng);
        let solution = complete.cases;
        let mut cases = solution.clone();

        let mut positions: Vec<usize> = (0..taille.cases()).collect();
        positions.shuffle(&mut *rng);

        let cible = niveau.cases_a_retirer(taille);
        let permise = technique_max(niveau);
        let exige_les_techniques = taille.cote() >= 9;
        let (mut retirees, mut plus_difficile) = (0, Technique::CaseUnique);
        for position in positions {
            if retirees >= cible && (!exige_les_techniques || plus_difficile.niveau() >= niveau) {
                break;
            }
            let valeur = cases[position];
            cases[position] = 0;
            match Logique::depuis_cases_valides(regles, &cases).resoudre(permise) {
                Some(technique) => {
                    retirees += 1;
                    plus_difficile = technique;
                }
                None => cases[position] = valeur,
            }
        }
        let conforme =
            retirees >= cible && (!exige_les_techniques || plus_difficile.niveau() >= niveau);
        Essai { cases, solution, retirees, plus_difficile, conforme }
    }

    // Génère une grille à solution unique en retirant `a_retirer` cases (au plus) d'une
    // grille complète tirée au hasard, sans se soucier des techniques qu'elle demande.
    // Les tests s'en servent pour obtenir des grilles de toutes sortes.
    #[cfg(test)]
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

// Un indice : la case remplie, son chiffre, et pourquoi c'est le bon
pub struct Indice {
    pub ligne: usize,
    pub col: usize,
    pub valeur: u8,
    // L'explication, qui nomme la technique de déduction employée
    pub explication: String,
}

// Un essai de génération d'une grille de niveau donné
struct Essai {
    cases: Vec<u8>,
    solution: Vec<u8>,
    retirees: usize,
    // La technique la plus difficile qu'il faut pour finir la grille
    plus_difficile: Technique,
    // true si la grille a les cases vides visées et demande bien les techniques du niveau
    conforme: bool,
}

impl Essai {
    // Pour choisir le meilleur essai : d'abord un essai conforme, puis le plus difficile,
    // puis celui qui a le plus de cases vides
    fn score(&self) -> (bool, Technique, usize) {
        (self.conforme, self.plus_difficile, self.retirees)
    }
}

// Le nombre d'essais et le temps au plus qu'on consacre à trouver une grille qui demande les
// techniques du niveau, avant de garder la meilleure trouvée
const ESSAIS_MAX: usize = 200;
const TEMPS_MAX_GENERATION: Duration = Duration::from_secs(3);

// Le nombre maximal d'étapes de recherche pour remplir une grille au hasard (un essai),
// et pour prouver qu'une grille n'a qu'une solution. Au-delà, on abandonne.
const BUDGET_REMPLISSAGE: u64 = 20_000;
#[cfg(test)]
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

        let Indice { ligne, col, valeur, .. } = g.indice().unwrap();
        assert_eq!(g.valeur(ligne, col), valeur);
        assert_eq!(valeur, solution.valeur(ligne, col));
        // 51 cases vides au départ, une de moins maintenant
        assert_eq!(compter_vides(&g), 50);
    }

    #[test]
    fn test_poser_ne_verifie_pas_les_regles() {
        let mut g = depart();
        // Il y a déjà un 5 sur la ligne 0 : `placer` refuse, `poser` accepte
        assert!(!g.placer(0, 2, 5));
        assert!(g.poser(0, 2, 5));
        assert_eq!(g.valeur(0, 2), 5);
        assert!(!g.est_correct(0, 2, 5));
        // Ce qui reste refusé : une case de départ, le 0 et un chiffre hors de la grille
        assert!(!g.poser(0, 0, 9));
        assert_eq!(g.valeur(0, 0), 5);
        assert!(!g.poser(0, 3, 0));
        assert!(!g.poser(0, 3, 10));
        assert_eq!(g.valeur(0, 3), 0);
    }

    #[test]
    fn test_une_grille_pleine_qui_se_contredit_n_est_pas_terminee() {
        let mut g = Grille::generer(Taille::Neuf, String::from("Test"), 20);
        assert!(g.remplir_solution());
        assert!(g.est_terminee());

        // On remplace un chiffre du joueur par un autre déjà présent sur sa ligne
        let position = (0..81).find(|&position| !g.fixes[position]).unwrap();
        let (ligne, col) = (position / 9, position % 9);
        let bon = g.valeur(ligne, col);
        let autre_col = (0..9).find(|&c| c != col).unwrap();
        let doublon = g.valeur(ligne, autre_col);
        assert!(g.poser(ligne, col, doublon));
        assert!(g.cases.iter().all(|&case| case != 0));
        assert!(!g.est_terminee());

        assert!(g.poser(ligne, col, bon));
        assert!(g.est_terminee());
    }

    #[test]
    fn test_restaurer_accepte_les_doublons_du_joueur_mais_pas_ceux_du_depart() {
        let mut g = depart();
        assert!(g.poser(0, 2, 5)); // un doublon du joueur
        let restauree = Grille::restaurer(
            Taille::Neuf,
            Variante::Classique,
            String::from("Test"),
            g.cases.clone(),
            g.fixes.clone(),
            g.solution.clone(),
        );
        assert!(restauree.is_ok());

        // Le même doublon, mais entre cases de départ : la grille est incohérente
        let mut fixes = g.fixes.clone();
        fixes[2] = true;
        let refusee = Grille::restaurer(Taille::Neuf, Variante::Classique, String::from("Test"), g.cases.clone(), fixes, None);
        assert!(refusee.is_err());
    }

    #[test]
    fn test_l_indice_explique_la_technique() {
        let mut g = depart();
        let Indice { ligne, col, valeur, explication } = g.indice().unwrap();
        // La grille de journal est facile : une case unique ou un chiffre unique
        assert!(
            explication.starts_with("Case unique") || explication.starts_with("Chiffre unique"),
            "{}",
            explication
        );
        // L'explication donne la case, en comptant à partir de 1
        assert!(
            explication.contains(&format!("ligne {}, colonne {}", ligne + 1, col + 1)),
            "{}",
            explication
        );
        assert!(explication.contains(&valeur.to_string()), "{}", explication);
    }

    #[test]
    fn test_chaque_indice_donne_la_bonne_valeur_jusqu_a_la_fin_de_la_grille() {
        for (niveau, taille) in [
            (Niveau::Facile, Taille::Six),
            (Niveau::Moyen, Taille::Six),
            (Niveau::Moyen, Taille::Neuf),
            (Niveau::Moyen, Taille::Douze),
        ] {
            let mut g = Grille::generer_niveau(niveau, taille, Variante::Classique);
            let solution = g.solution().unwrap().to_vec();
            let mut nombre = 0;
            while let Some(indice) = g.indice() {
                assert_eq!(indice.valeur, solution[indice.ligne * taille.cote() + indice.col]);
                assert!(!indice.explication.is_empty());
                nombre += 1;
            }
            assert!(g.est_terminee(), "{} {}", niveau.libelle(), taille.libelle());
            assert!(nombre > 0);
        }
    }

    #[test]
    fn test_un_indice_dit_quand_il_faut_retirer_des_possibilites() {
        // Une grille qui demande une intersection ou une paire nue : au moins un des indices
        // successifs doit d'abord retirer des possibilités
        let mut trouve = false;
        for _ in 0..4 {
            let mut g = Grille::generer_niveau(Niveau::Moyen, Taille::Neuf, Variante::Classique);
            while let Some(indice) = g.indice() {
                if indice.explication.contains("Auparavant, il faut retirer des possibilités avec") {
                    trouve = true;
                }
            }
            if trouve {
                break;
            }
        }
        assert!(trouve, "aucun indice n'a mentionné de technique d'élimination");
    }

    // Une grille très creuse, que la déduction seule ne finit pas
    fn grille_bloquee() -> Grille {
        for _ in 0..80 {
            let g = Grille::generer(Taille::Neuf, String::from("Test"), 70);
            if technique_requise(&g).is_none() {
                return g;
            }
        }
        panic!("aucune grille bloquée trouvée");
    }

    #[test]
    fn test_un_indice_sans_deduction_possible_revele_quand_meme_la_solution() {
        let mut g = grille_bloquee();
        let solution = g.solution().unwrap().to_vec();
        let indice = g.indice().unwrap();
        assert_eq!(indice.valeur, solution[indice.ligne * 9 + indice.col]);
        assert_eq!(g.valeur(indice.ligne, indice.col), indice.valeur);
    }

    #[test]
    fn test_un_indice_sans_deduction_possible_le_dit() {
        // Dans une grille que la déduction ne finit pas, on arrive forcément à un indice
        // qu'aucune technique ne trouve : l'explication le dit franchement
        let mut g = grille_bloquee();
        let mut explications = Vec::new();
        while let Some(indice) = g.indice() {
            explications.push(indice.explication);
        }
        assert!(g.est_terminee());
        assert!(
            explications.iter().any(|texte| texte.contains("Aucune technique de déduction")),
            "aucun indice n'a avoué qu'il ne venait d'aucune technique"
        );
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
        let g = Grille::generer_niveau(Niveau::Facile, Taille::Neuf, Variante::Classique);
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
        let facile = Grille::creer(Origine::Niveau(Niveau::Facile, Taille::Neuf, Variante::Classique)).unwrap();
        assert_eq!(facile.nom, "Partie facile 9×9");
        let petite = Grille::creer(Origine::Niveau(Niveau::Moyen, Taille::Quatre, Variante::Classique)).unwrap();
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

    // La technique la plus difficile qu'il faut pour finir la grille par déduction pure
    // (None si la déduction ne suffit pas)
    fn technique_requise(grille: &Grille) -> Option<Technique> {
        Logique::new(&grille.regles, &grille.cases)
            .unwrap()
            .resoudre(Technique::XWing)
    }

    fn a_une_seule_solution(grille: &Grille) -> bool {
        grille.solveur().unwrap().compter(2) == 1
    }

    // Ce qui doit être vrai de toute grille générée, quels que soient sa taille et son niveau
    fn verifier_une_grille_generee(grille: &Grille, niveau: Niveau, taille: Taille) {
        let nom = format!("{} {}", niveau.libelle(), taille.libelle());
        assert_eq!(grille.taille(), taille, "{}", nom);
        assert!(compter_vides(grille) > 0, "{}", nom);
        // Les cases de départ sont verrouillées, et elles seules
        for position in 0..taille.cases() {
            assert_eq!(grille.fixes[position], grille.cases[position] != 0, "{}", nom);
        }
        // La solution connue complète bien la grille de départ
        let solution = grille.solution().expect("une grille générée connaît sa solution");
        for position in 0..taille.cases() {
            assert!(grille.cases[position] == 0 || grille.cases[position] == solution[position], "{}", nom);
        }
        // On la finit par déduction, avec les techniques du niveau seulement, et sans jamais deviner
        let requise = technique_requise(grille).unwrap_or_else(|| panic!("{} : la déduction ne suffit pas", nom));
        assert!(requise <= technique_max(niveau), "{} demande {:?}", nom, requise);
        if taille.cote() <= 9 {
            assert!(a_une_seule_solution(grille), "{}", nom);
        }
    }

    #[test]
    fn test_generer_niveau_chaque_taille() {
        for taille in Taille::TOUTES {
            // Le niveau Difficile des grandes grilles est long à générer en mode debug
            let niveaux: &[Niveau] = if taille.cote() >= 12 {
                &[Niveau::Facile, Niveau::Moyen]
            } else {
                &Niveau::TOUS
            };
            for &niveau in niveaux {
                let g = Grille::generer_niveau(niveau, taille, Variante::Classique);
                verifier_une_grille_generee(&g, niveau, taille);
            }
        }
    }

    #[test]
    fn test_le_niveau_facile_s_arrete_au_nombre_de_cases_vises() {
        for taille in Taille::TOUTES {
            let g = Grille::generer_niveau(Niveau::Facile, taille, Variante::Classique);
            assert_eq!(compter_vides(&g), Niveau::Facile.cases_a_retirer(taille), "{}", taille.libelle());
        }
    }

    #[test]
    fn test_facile_se_resout_avec_des_cases_uniques_et_des_chiffres_uniques() {
        for taille in [Taille::Neuf, Taille::Douze] {
            let g = Grille::generer_niveau(Niveau::Facile, taille, Variante::Classique);
            let requise = technique_requise(&g).unwrap();
            assert!(requise <= Technique::ChiffreUnique);
        }
    }

    #[test]
    fn test_les_petites_grilles_suivent_le_nombre_de_cases_vides() {
        // En 4×4 et 6×6, le niveau ne dépend que du nombre de cases vides
        for taille in [Taille::Quatre, Taille::Six] {
            for niveau in Niveau::TOUS {
                let g = Grille::generer_niveau(niveau, taille, Variante::Classique);
                assert_eq!(
                    compter_vides(&g),
                    niveau.cases_a_retirer(taille),
                    "{} {}",
                    niveau.libelle(),
                    taille.libelle()
                );
            }
        }
    }

    #[test]
    fn test_moyen_demande_une_intersection_ou_une_paire_nue() {
        let mut conformes = 0;
        for _ in 0..5 {
            let g = Grille::generer_niveau(Niveau::Moyen, Taille::Neuf, Variante::Classique);
            let requise = technique_requise(&g).unwrap();
            assert!(requise <= Technique::PaireNue);
            if requise >= Technique::Intersection {
                conformes += 1;
                // Au moins autant de cases vides que visé
                assert!(compter_vides(&g) >= Niveau::Moyen.cases_a_retirer(Taille::Neuf));
            }
        }
        // Un essai peut exceptionnellement échouer, jamais presque tous
        assert!(conformes >= 4, "seulement {} grilles Moyen sur 5 demandent une vraie technique", conformes);
    }

    #[test]
    fn test_difficile_demande_une_technique_avancee() {
        let mut conformes = 0;
        for _ in 0..3 {
            let g = Grille::generer_niveau(Niveau::Difficile, Taille::Neuf, Variante::Classique);
            let requise = technique_requise(&g).unwrap();
            assert!(requise <= Technique::XWing);
            if requise >= Technique::PaireCachee {
                conformes += 1;
            }
        }
        assert!(conformes >= 1, "aucune grille Difficile sur 3 ne demande de technique avancée");
    }

    #[test]
    fn test_les_niveaux_se_classent_par_technique() {
        // En 9×9, la technique la plus difficile ne baisse pas d'un niveau au suivant
        let mut totaux = Vec::new();
        for niveau in Niveau::TOUS {
            let total: usize = (0..3)
                .map(|_| technique_requise(&Grille::generer_niveau(niveau, Taille::Neuf, Variante::Classique)).unwrap() as usize)
                .sum();
            totaux.push(total);
        }
        assert!(totaux[0] < totaux[1], "{:?}", totaux);
        assert!(totaux[1] < totaux[2], "{:?}", totaux);
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
        let Indice { ligne, col, valeur, .. } = g.indice().unwrap();
        assert_eq!(g.valeur(ligne, col), valeur);
        assert!(g.est_correct(ligne, col, valeur));
    }

    #[test]
    fn test_candidats() {
        let g = Grille::creer(Origine::Fixe).unwrap();
        // La case (0, 2) de la grille d'entraînement admet 1, 2 ou 4 : bits 0, 1 et 3
        assert_eq!(g.candidats(0, 2), 0b1011);
        // Une case de départ n'a aucun candidat
        assert_eq!(g.candidats(0, 0), 0);

        // Une grille vide laisse tous les chiffres possibles
        let vide = Grille::nouvelle(Taille::Seize, String::from("Vide"));
        assert_eq!(vide.candidats(5, 5), u16::MAX);
        let mut petite = Grille::nouvelle(Taille::Quatre, String::from("Petite"));
        assert_eq!(petite.candidats(0, 0), 0b1111);
        assert!(petite.placer(0, 1, 3));
        // Le 3 est pris dans la ligne, la colonne 1 et le bloc
        assert_eq!(petite.candidats(0, 0), 0b1011);
        assert_eq!(petite.candidats(3, 1), 0b1011);
        assert_eq!(petite.candidats(1, 0), 0b1011);
    }

    // Une grille complète respecte sa variante : chaque zone contient tous les chiffres une fois,
    // et deux cases qui se l'interdisent n'ont jamais le même chiffre
    fn respecte_sa_variante(grille: &Grille, cases: &[u8]) -> bool {
        let regles = grille.regles();
        let cote = grille.taille().cote();
        let attendu = (1u32 << cote) - 1;
        cases.iter().all(|&valeur| valeur != 0 && valeur as usize <= cote)
            && regles
                .zones
                .iter()
                .all(|zone| zone.iter().fold(0u32, |masque, &case| masque | 1 << (cases[case] - 1)) == attendu)
            && (0..cases.len())
                .all(|case| regles.voisines[case].iter().all(|&autre| cases[autre] != cases[case]))
    }

    #[test]
    fn test_les_grilles_generees_respectent_leur_variante() {
        for variante in Variante::TOUTES {
            for taille in [Taille::Quatre, Taille::Six, Taille::Neuf] {
                if !variante.accepte(taille) {
                    continue;
                }
                let g = Grille::generer_niveau(Niveau::Facile, taille, variante);
                let nom = format!("{:?} {}", variante, taille.libelle());
                assert_eq!(g.variante(), variante, "{}", nom);
                assert!(respecte_sa_variante(&g, g.solution().unwrap()), "{}", nom);
                // Les chiffres de départ sont ceux de la solution
                for position in 0..taille.cases() {
                    assert!(g.cases[position] == 0 || g.cases[position] == g.solution().unwrap()[position], "{}", nom);
                }
            }
        }
    }

    #[test]
    fn test_chaque_variante_se_resout_par_deduction_a_tous_les_niveaux() {
        for variante in Variante::TOUTES {
            for niveau in [Niveau::Facile, Niveau::Moyen] {
                let taille = Taille::Neuf;
                let g = Grille::generer_niveau(niveau, taille, variante);
                verifier_une_grille_generee(&g, niveau, taille);
            }
        }
    }

    #[test]
    fn test_les_grandes_grilles_existent_aussi_en_variante() {
        for variante in [Variante::Diagonale, Variante::AntiCavalier] {
            for taille in [Taille::Douze, Taille::Seize] {
                let g = Grille::generer_niveau(Niveau::Facile, taille, variante);
                verifier_une_grille_generee(&g, Niveau::Facile, taille);
            }
        }
    }

    #[test]
    fn test_le_nom_d_une_grille_de_variante() {
        let g = Grille::generer_niveau(Niveau::Facile, Taille::Quatre, Variante::Diagonale);
        assert_eq!(g.nom, "Partie facile 4×4 diagonale");
        let g = Grille::generer_niveau(Niveau::Moyen, Taille::Quatre, Variante::AntiCavalier);
        assert_eq!(g.nom, "Partie moyen 4×4 anti-cavalier");
        let g = Grille::generer_niveau(Niveau::Moyen, Taille::Quatre, Variante::Classique);
        assert_eq!(g.nom, "Partie moyen 4×4");
    }

    #[test]
    fn test_creer_une_grille_de_variante() {
        let g = Grille::creer(Origine::Niveau(Niveau::Facile, Taille::Quatre, Variante::Diagonale)).unwrap();
        assert_eq!(g.variante(), Variante::Diagonale);
        assert_eq!(g.taille(), Taille::Quatre);
    }

    #[test]
    fn test_placer_refuse_un_doublon_sur_une_diagonale() {
        let regles = Regles::pour(Taille::Neuf, Variante::Diagonale);
        let mut g = Grille::avec_regles(regles, String::from("Test"));
        assert!(g.placer(0, 0, 5));
        // (4, 4) est sur la même diagonale, mais pas dans la même ligne, colonne ni bloc
        assert!(!g.placer(4, 4, 5));
        // Sur l'autre diagonale (sans rien en commun avec (0, 0)), le 5 est permis une fois, pas deux
        assert!(g.placer(3, 5, 5));
        assert!(!g.placer(6, 2, 5));
        let mut classique = Grille::nouvelle(Taille::Neuf, String::from("Test"));
        assert!(classique.placer(0, 0, 5));
        assert!(classique.placer(4, 4, 5));
    }

    #[test]
    fn test_placer_refuse_un_saut_de_cavalier() {
        let regles = Regles::pour(Taille::Neuf, Variante::AntiCavalier);
        let mut g = Grille::avec_regles(regles, String::from("Test"));
        assert!(g.placer(4, 4, 7));
        assert!(!g.placer(2, 5, 7)); // un saut de cavalier
        assert!(!g.placer(6, 3, 7)); // un autre
        assert!(g.placer(2, 6, 7)); // ce n'est pas un saut de cavalier
        assert!(!g.est_valide(5, 2, 7));
    }

    #[test]
    fn test_hyper_refuse_un_doublon_dans_une_fenetre() {
        let regles = Regles::pour(Taille::Neuf, Variante::Hyper);
        let mut g = Grille::avec_regles(regles, String::from("Test"));
        assert!(g.placer(1, 1, 4));
        assert!(!g.placer(3, 3, 4)); // même fenêtre
        assert!(g.placer(0, 4, 4)); // hors de toute fenêtre et de la ligne, colonne, bloc de (1, 1)
    }

    #[test]
    fn test_les_candidats_tiennent_compte_de_la_variante() {
        let regles = Regles::pour(Taille::Neuf, Variante::Diagonale);
        let mut g = Grille::avec_regles(regles, String::from("Test"));
        assert!(g.placer(0, 0, 5));
        // Le 5 est exclu de toute la diagonale, mais pas des autres cases
        assert_eq!(g.candidats(4, 4) & (1 << 4), 0);
        assert_eq!(g.candidats(8, 8) & (1 << 4), 0);
        assert_ne!(g.candidats(4, 5) & (1 << 4), 0);
    }

    #[test]
    fn test_les_indices_menent_a_la_fin_dans_chaque_variante() {
        for variante in Variante::TOUTES {
            let mut g = Grille::generer_niveau(Niveau::Moyen, Taille::Neuf, variante);
            let solution = g.solution().unwrap().to_vec();
            while let Some(indice) = g.indice() {
                assert_eq!(indice.valeur, solution[indice.ligne * 9 + indice.col], "{:?}", variante);
                assert!(!indice.explication.is_empty());
            }
            assert!(g.est_terminee(), "{:?}", variante);
            assert!(respecte_sa_variante(&g, &g.cases), "{:?}", variante);
        }
    }

    #[test]
    fn test_restaurer_garde_la_variante() {
        let g = Grille::generer_niveau(Niveau::Facile, Taille::Quatre, Variante::Diagonale);
        let restauree = Grille::restaurer(
            Taille::Quatre,
            Variante::Diagonale,
            g.nom.clone(),
            g.cases.clone(),
            g.fixes.clone(),
            g.solution.clone(),
        )
        .unwrap();
        assert_eq!(restauree.variante(), Variante::Diagonale);
        // La solution d'une grille classique n'est pas valide en diagonale
        let classique = Grille::generer_niveau(Niveau::Facile, Taille::Six, Variante::Classique);
        let mauvaise = Grille::restaurer(
            Taille::Six,
            Variante::Diagonale,
            String::from("Test"),
            classique.cases.clone(),
            classique.fixes.clone(),
            classique.solution.clone(),
        );
        // (elle peut, par hasard, être valide aussi en diagonale : on vérifie la cohérence, pas le refus)
        if let Ok(grille) = mauvaise {
            assert!(respecte_sa_variante(&grille, grille.solution().unwrap()));
        }
    }

    #[test]
    fn test_les_explications_nomment_les_zones_de_la_variante() {
        // Dans une grille en diagonale, une explication peut nommer une diagonale
        let mut vue = false;
        for _ in 0..30 {
            let mut g = Grille::generer_niveau(Niveau::Moyen, Taille::Neuf, Variante::Diagonale);
            while let Some(indice) = g.indice() {
                if indice.explication.contains("diagonale") {
                    vue = true;
                }
            }
            if vue {
                break;
            }
        }
        assert!(vue, "aucune explication n'a nommé une diagonale");
    }
}
