//! Les règles d'une grille : quelles cases ne peuvent pas contenir le même chiffre.
//!
//! Une grille de Sudoku est décrite par des *zones* : des groupes de `côté` cases qui doivent
//! contenir chaque chiffre exactement une fois (les lignes, les colonnes, les blocs, et selon la
//! variante les diagonales ou des fenêtres en plus). Certaines variantes ajoutent d'autres
//! interdits qui ne forment pas une zone (l'anti-cavalier). Le solveur, les techniques de
//! déduction et la génération ne connaissent que ces zones et ces interdits : c'est ce qui permet
//! d'ajouter une variante sans toucher à leur code.

use crate::grille::Taille;
use std::sync::Arc;

// Les variantes du jeu
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variante {
    Classique,
    // Les deux grandes diagonales contiennent aussi chaque chiffre une fois
    Diagonale,
    // Quatre fenêtres supplémentaires de 3×3 (9×9 seulement)
    Hyper,
    // Deux cases à un saut de cavalier l'une de l'autre ne contiennent pas le même chiffre
    AntiCavalier,
}

impl Variante {
    pub const TOUTES: [Variante; 4] = [
        Variante::Classique,
        Variante::Diagonale,
        Variante::Hyper,
        Variante::AntiCavalier,
    ];

    pub fn libelle(self) -> &'static str {
        match self {
            Variante::Classique => "Classique",
            Variante::Diagonale => "Diagonale",
            Variante::Hyper => "Hyper",
            Variante::AntiCavalier => "Anti-cavalier",
        }
    }

    // Le mot écrit dans les fichiers et accepté sur la ligne de commande
    pub fn mot(self) -> &'static str {
        match self {
            Variante::Classique => "classique",
            Variante::Diagonale => "diagonale",
            Variante::Hyper => "hyper",
            Variante::AntiCavalier => "anticavalier",
        }
    }

    pub fn depuis_mot(mot: &str) -> Option<Variante> {
        Variante::TOUTES.into_iter().find(|variante| variante.mot() == mot)
    }

    // La règle en plus, en une phrase
    pub fn regle(self) -> &'static str {
        match self {
            Variante::Classique => "Chaque ligne, colonne et bloc doit contenir chaque nombre une seule fois.",
            Variante::Diagonale => {
                "En plus, les deux grandes diagonales (en couleur) contiennent chaque nombre une seule fois."
            }
            Variante::Hyper => {
                "En plus, quatre fenêtres de 3×3 (en couleur) contiennent chaque nombre une seule fois."
            }
            Variante::AntiCavalier => {
                "En plus, deux cases séparées par un saut de cavalier (comme aux échecs) n'ont pas le même nombre."
            }
        }
    }

    // true si la variante existe dans cette taille de grille
    pub fn accepte(self, taille: Taille) -> bool {
        match self {
            Variante::Classique => true,
            Variante::Hyper => taille == Taille::Neuf,
            Variante::Diagonale | Variante::AntiCavalier => true,
        }
    }
}

// Une zone, pour la nommer dans les explications : ligne, colonne, bloc, diagonale ou fenêtre
// (numérotés à partir de 0)
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Zone {
    Ligne(usize),
    Colonne(usize),
    Bloc(usize),
    // 0 : de haut à gauche à bas à droite, 1 : de bas à gauche à haut à droite
    Diagonale(usize),
    Fenetre(usize),
}

impl Zone {
    // « dans la ligne 3 »
    pub fn dans(self) -> String {
        match self {
            Zone::Ligne(i) => format!("dans la ligne {}", i + 1),
            Zone::Colonne(i) => format!("dans la colonne {}", i + 1),
            Zone::Bloc(i) => format!("dans le bloc {}", i + 1),
            Zone::Diagonale(0) => String::from("dans la diagonale descendante"),
            Zone::Diagonale(_) => String::from("dans la diagonale montante"),
            Zone::Fenetre(i) => format!("dans la fenêtre {}", i + 1),
        }
    }

    // « de la ligne 3 »
    pub fn de(self) -> String {
        match self {
            Zone::Ligne(i) => format!("de la ligne {}", i + 1),
            Zone::Colonne(i) => format!("de la colonne {}", i + 1),
            Zone::Bloc(i) => format!("du bloc {}", i + 1),
            Zone::Diagonale(0) => String::from("de la diagonale descendante"),
            Zone::Diagonale(_) => String::from("de la diagonale montante"),
            Zone::Fenetre(i) => format!("de la fenêtre {}", i + 1),
        }
    }
}

// Les règles d'une grille d'une taille et d'une variante données
pub struct Regles {
    pub taille: Taille,
    pub variante: Variante,
    // Les zones, dans cet ordre : les `côté` lignes (de gauche à droite), les `côté` colonnes
    // (de haut en bas), les `côté` blocs, puis les zones de la variante. Chacune a `côté` cases
    // (indice = ligne × côté + col) qui contiennent chacune un chiffre différent.
    pub zones: Vec<Vec<usize>>,
    // Pour chaque case, les zones qui la contiennent
    pub zones_de: Vec<Vec<usize>>,
    // Pour chaque case, les cases qui n'ont pas le droit de contenir le même chiffre, mais qui ne
    // sont dans aucune zone commune avec elle (les sauts de cavalier de l'anti-cavalier)
    pub supplementaires: Vec<Vec<usize>>,
    // Pour chaque case, toutes celles qui ne peuvent pas contenir le même chiffre
    pub voisines: Vec<Vec<usize>>,
    // contient[zone][case] : true si la zone contient la case
    pub contient: Vec<Vec<bool>>,
    // Les couples (zone, zone qui la croise en au moins deux cases), dans les deux sens : un bloc
    // avec chacune de ses lignes et colonnes, une diagonale avec un bloc...
    pub croisements: Vec<(usize, usize)>,
    // Le numéro du bloc de chaque case : sert à tracer les contours des blocs
    pub bloc_de: Vec<usize>,
}

impl Regles {
    // Les règles d'une grille classique
    pub fn classique(taille: Taille) -> Arc<Regles> {
        Regles::pour(taille, Variante::Classique)
    }

    // Les règles d'une taille et d'une variante. Les blocs sont des rectangles.
    pub fn pour(taille: Taille, variante: Variante) -> Arc<Regles> {
        let cote = taille.cote();
        let blocs_par_ligne = cote / taille.bloc_l();
        let bloc_de = (0..cote * cote)
            .map(|case| {
                let (ligne, col) = (case / cote, case % cote);
                (ligne / taille.bloc_h()) * blocs_par_ligne + col / taille.bloc_l()
            })
            .collect();
        Arc::new(Regles::construire(taille, variante, bloc_de))
    }

    // Construit les règles à partir de la répartition des cases en blocs
    fn construire(taille: Taille, variante: Variante, bloc_de: Vec<usize>) -> Regles {
        let cote = taille.cote();
        let cases = cote * cote;

        let mut zones: Vec<Vec<usize>> = vec![Vec::with_capacity(cote); 3 * cote];
        for case in 0..cases {
            let (ligne, col) = (case / cote, case % cote);
            zones[ligne].push(case);
            zones[cote + col].push(case);
            zones[2 * cote + bloc_de[case]].push(case);
        }
        match variante {
            Variante::Diagonale => {
                zones.push((0..cote).map(|i| i * cote + i).collect());
                zones.push((0..cote).map(|i| i * cote + (cote - 1 - i)).collect());
            }
            Variante::Hyper => {
                // Les quatre fenêtres de 3×3 dont le coin haut gauche est en (1, 1), (1, 5), (5, 1), (5, 5)
                for (ligne, col) in [(1, 1), (1, 5), (5, 1), (5, 5)] {
                    let fenetre = (0..3)
                        .flat_map(|i| (0..3).map(move |j| (ligne + i) * cote + col + j))
                        .collect();
                    zones.push(fenetre);
                }
            }
            Variante::Classique | Variante::AntiCavalier => {}
        }

        let mut zones_de = vec![Vec::new(); cases];
        let mut contient = vec![vec![false; cases]; zones.len()];
        for (indice, zone) in zones.iter().enumerate() {
            for &case in zone {
                zones_de[case].push(indice);
                contient[indice][case] = true;
            }
        }

        // Les voisines par les zones, puis les sauts de cavalier qui n'en font pas déjà partie
        let mut voisines: Vec<Vec<usize>> = Vec::with_capacity(cases);
        let mut supplementaires: Vec<Vec<usize>> = Vec::with_capacity(cases);
        for case in 0..cases {
            let mut voisine = vec![false; cases];
            for &zone in &zones_de[case] {
                for &autre in &zones[zone] {
                    voisine[autre] = true;
                }
            }
            voisine[case] = false;
            let mut en_plus = Vec::new();
            if variante == Variante::AntiCavalier {
                let (ligne, col) = ((case / cote) as i32, (case % cote) as i32);
                for (dl, dc) in [(1, 2), (2, 1), (-1, 2), (-2, 1), (1, -2), (2, -1), (-1, -2), (-2, -1)] {
                    let (l, c) = (ligne + dl, col + dc);
                    if (0..cote as i32).contains(&l) && (0..cote as i32).contains(&c) {
                        let autre = l as usize * cote + c as usize;
                        if !voisine[autre] {
                            voisine[autre] = true;
                            en_plus.push(autre);
                        }
                    }
                }
            }
            voisines.push((0..cases).filter(|&autre| voisine[autre]).collect());
            supplementaires.push(en_plus);
        }

        // Les zones qui se croisent en au moins deux cases, en commençant par les blocs
        let mut croisements = Vec::new();
        let ordre: Vec<usize> = (2 * cote..zones.len()).collect();
        for &a in &ordre {
            for b in 0..zones.len() {
                if b == a || (b >= 2 * cote && b < a) {
                    continue;
                }
                let communes = zones[a].iter().filter(|&&case| contient[b][case]).count();
                if communes >= 2 && communes < cote {
                    croisements.push((a, b));
                    croisements.push((b, a));
                }
            }
        }

        Regles {
            taille,
            variante,
            zones,
            zones_de,
            supplementaires,
            voisines,
            contient,
            croisements,
            bloc_de,
        }
    }

    pub fn cote(&self) -> usize {
        self.taille.cote()
    }

    // La zone qu'on nomme dans une explication, à partir de son indice
    pub fn zone(&self, indice: usize) -> Zone {
        let cote = self.cote();
        if indice < cote {
            Zone::Ligne(indice)
        } else if indice < 2 * cote {
            Zone::Colonne(indice - cote)
        } else if indice < 3 * cote {
            Zone::Bloc(indice - 2 * cote)
        } else if self.variante == Variante::Diagonale {
            Zone::Diagonale(indice - 3 * cote)
        } else {
            Zone::Fenetre(indice - 3 * cote)
        }
    }

    // « ligne 3, colonne 4 »
    pub fn case_texte(&self, case: usize) -> String {
        format!("ligne {}, colonne {}", case / self.cote() + 1, case % self.cote() + 1)
    }

    // true si la case est dans une zone de la variante (diagonale, fenêtre) : on la colore
    pub fn est_zone_speciale(&self, case: usize) -> bool {
        self.zones_de[case].iter().any(|&zone| zone >= 3 * self.cote())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_les_zones_classiques_9x9() {
        let regles = Regles::classique(Taille::Neuf);
        assert_eq!(regles.zones.len(), 27);
        assert!(regles.zones.iter().all(|zone| zone.len() == 9));
        // Chaque case est dans une ligne, une colonne et un bloc
        assert!(regles.zones_de.iter().all(|zones| zones.len() == 3));
        // Chaque case a 8 + 8 + 4 voisines (le bloc partage 4 cases hors ligne et colonne)
        assert!(regles.voisines.iter().all(|voisines| voisines.len() == 20));
        assert!(regles.supplementaires.iter().all(Vec::is_empty));
        // 9 blocs × (3 lignes + 3 colonnes) × 2 sens
        assert_eq!(regles.croisements.len(), 108);
        assert_eq!(regles.zone(0), Zone::Ligne(0));
        assert_eq!(regles.zone(9), Zone::Colonne(0));
        assert_eq!(regles.zone(18), Zone::Bloc(0));
        assert!(!regles.est_zone_speciale(0));
    }

    #[test]
    fn test_les_blocs_rectangulaires() {
        // 6×6 : des blocs de 2 lignes sur 3 colonnes
        let regles = Regles::classique(Taille::Six);
        assert_eq!(regles.zones[12 + 1], vec![3, 4, 5, 9, 10, 11]);
        assert_eq!(regles.croisements.len(), 60);
        assert!(regles.voisines.iter().all(|voisines| voisines.len() == 12));
        // 12×12 : des blocs de 3 lignes sur 4 colonnes
        let regles = Regles::classique(Taille::Douze);
        assert_eq!(regles.zones[24 + 1].len(), 12);
        assert_eq!(regles.croisements.len(), 12 * (3 + 4) * 2);
    }

    #[test]
    fn test_chaque_taille_a_les_zones_attendues() {
        for taille in Taille::TOUTES {
            let regles = Regles::classique(taille);
            let cote = taille.cote();
            assert_eq!(regles.zones.len(), 3 * cote);
            for zone in &regles.zones {
                assert_eq!(zone.len(), cote, "{}", taille.libelle());
            }
            // Le numéro de bloc de chaque case est celui de sa zone de bloc
            for case in 0..taille.cases() {
                assert!(regles.contient[2 * cote + regles.bloc_de[case]][case]);
            }
        }
    }

    #[test]
    fn test_la_diagonale_ajoute_deux_zones() {
        let regles = Regles::pour(Taille::Neuf, Variante::Diagonale);
        assert_eq!(regles.zones.len(), 29);
        assert_eq!(regles.zones[27], vec![0, 10, 20, 30, 40, 50, 60, 70, 80]);
        assert_eq!(regles.zones[28], vec![8, 16, 24, 32, 40, 48, 56, 64, 72]);
        assert_eq!(regles.zone(27), Zone::Diagonale(0));
        assert_eq!(regles.zone(28), Zone::Diagonale(1));
        // La case centrale est sur les deux diagonales
        assert_eq!(regles.zones_de[40].len(), 5);
        assert!(regles.est_zone_speciale(0));
        assert!(regles.est_zone_speciale(40));
        assert!(!regles.est_zone_speciale(1));
        // La case (0, 0) a quatre voisines de plus qu'en classique ou presque :
        // les 8 autres cases de sa diagonale, dont 2 sont déjà dans son bloc
        let classique = Regles::classique(Taille::Neuf);
        assert!(regles.voisines[0].len() > classique.voisines[0].len());
        // Les diagonales croisent des blocs en 3 cases
        assert!(regles.croisements.iter().any(|&(a, b)| a == 27 && b >= 18 && b < 27));
        // Elles ne croisent ni les lignes ni les colonnes en plus d'une case
        assert!(!regles.croisements.iter().any(|&(a, b)| a == 27 && b < 18));
    }

    #[test]
    fn test_hyper_ajoute_quatre_fenetres() {
        let regles = Regles::pour(Taille::Neuf, Variante::Hyper);
        assert_eq!(regles.zones.len(), 31);
        assert_eq!(regles.zones[27], vec![10, 11, 12, 19, 20, 21, 28, 29, 30]);
        assert_eq!(regles.zones[30], vec![50, 51, 52, 59, 60, 61, 68, 69, 70]);
        assert_eq!(regles.zone(29), Zone::Fenetre(2));
        assert!(regles.est_zone_speciale(10));
        assert!(!regles.est_zone_speciale(0));
    }

    #[test]
    fn test_anti_cavalier_ajoute_les_sauts_de_cavalier() {
        let regles = Regles::pour(Taille::Neuf, Variante::AntiCavalier);
        // Pas de zone en plus
        assert_eq!(regles.zones.len(), 27);
        // La case du centre a ses 8 sauts de cavalier, dont certains sont dans son bloc
        let centre = 4 * 9 + 4;
        let sauts: Vec<usize> = [(2, 3), (2, 5), (3, 2), (3, 6), (5, 2), (5, 6), (6, 3), (6, 5)]
            .iter()
            .map(|&(l, c)| l * 9 + c)
            .collect();
        for saut in &sauts {
            assert!(regles.voisines[centre].contains(saut), "{}", saut);
        }
        // Ceux qui sont déjà dans une zone commune ne sont pas comptés en plus
        for &saut in &regles.supplementaires[centre] {
            assert!(!regles.zones_de[centre].iter().any(|&zone| regles.contient[zone][saut]));
        }
        // La case en haut à gauche n'a que 2 sauts, et ils sont tous deux dans son bloc
        assert!(regles.voisines[0].contains(&(2 * 9 + 1)));
        assert!(regles.voisines[0].contains(&(9 + 2)));
        assert!(regles.supplementaires[0].is_empty());
        // La case (0, 3) a deux sauts hors de son bloc : (1, 1) et (2, 2)
        assert_eq!(regles.supplementaires[3], vec![9 + 1, 2 * 9 + 2]);
        // Pas de zone qui n'est pas complète : il n'y a pas de zone spéciale à colorer
        assert!(!regles.est_zone_speciale(0));
    }

    #[test]
    fn test_les_voisines_sont_symetriques_et_sans_doublon() {
        for variante in Variante::TOUTES {
            let regles = Regles::pour(Taille::Neuf, variante);
            for case in 0..81 {
                let voisines = &regles.voisines[case];
                assert!(!voisines.contains(&case));
                assert!(voisines.windows(2).all(|paire| paire[0] < paire[1]), "{:?}", variante);
                for &autre in voisines {
                    assert!(regles.voisines[autre].contains(&case), "{:?} {} {}", variante, case, autre);
                }
            }
        }
    }

    #[test]
    fn test_les_variantes_se_retrouvent_par_leur_mot() {
        for variante in Variante::TOUTES {
            assert_eq!(Variante::depuis_mot(variante.mot()), Some(variante));
            assert!(!variante.libelle().is_empty());
            assert!(!variante.regle().is_empty());
        }
        assert_eq!(Variante::depuis_mot("inconnue"), None);
    }

    #[test]
    fn test_hyper_n_existe_qu_en_9x9() {
        for taille in Taille::TOUTES {
            assert_eq!(Variante::Hyper.accepte(taille), taille == Taille::Neuf);
            assert!(Variante::Classique.accepte(taille));
        }
    }
}
