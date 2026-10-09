//! Le score d'une partie et le classement des meilleurs scores, enregistré dans un fichier.

use crate::grille::{Niveau, Taille};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const LONGUEUR_MAX_NOM: usize = 20;

// Le temps coûte 1 point par seconde, quelle que soit la taille de la grille
const PENALITE_ERREUR_9X9: u64 = 100;
const PENALITE_INDICE_9X9: u64 = 150;

// Points de départ en 9×9 : plus le niveau est difficile, plus on peut marquer
fn points_de_base_9x9(niveau: Niveau) -> u64 {
    match niveau {
        Niveau::Facile => 1000,
        Niveau::Moyen => 2000,
        Niveau::Difficile => 3000,
    }
}

// Le facteur qui adapte les points (de départ et de pénalité) à la taille, en millièmes.
// Une grande grille prend bien plus de temps : elle doit rapporter plus de points.
fn echelle(taille: Taille) -> u64 {
    match taille {
        Taille::Quatre => 250,
        Taille::Six => 500,
        Taille::Neuf => 1000,
        Taille::Douze => 2500,
        Taille::Seize => 5000,
    }
}

// Points de départ pour un niveau et une taille
pub fn points_de_base(niveau: Niveau, taille: Taille) -> u64 {
    points_de_base_9x9(niveau) * echelle(taille) / 1000
}

// Points retirés pour une erreur
pub fn penalite_erreur(taille: Taille) -> u64 {
    PENALITE_ERREUR_9X9 * echelle(taille) / 1000
}

// Points retirés pour un indice
pub fn penalite_indice(taille: Taille) -> u64 {
    PENALITE_INDICE_9X9 * echelle(taille) / 1000
}

// Le score d'une partie gagnée : les points du niveau et de la taille, moins les pénalités.
// Jamais négatif.
pub fn calculer_score(
    niveau: Niveau,
    taille: Taille,
    duree: Duration,
    erreurs: u32,
    indices: u32,
) -> u32 {
    let penalites = duree.as_secs()
        + erreurs as u64 * penalite_erreur(taille)
        + indices as u64 * penalite_indice(taille);
    points_de_base(niveau, taille).saturating_sub(penalites) as u32
}

// Le nombre de secondes écoulées depuis le 1er janvier 1970
pub fn maintenant() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duree| duree.as_secs())
}

// Garde un nom utilisable : sans caractères de contrôle (tabulation, retour à la ligne),
// sans espaces autour et d'au plus 20 caractères
pub fn nettoyer_nom(nom: &str) -> String {
    let sans_controle: String = nom.chars().filter(|c| !c.is_control()).collect();
    let court: String = sans_controle.trim().chars().take(LONGUEUR_MAX_NOM).collect();
    court.trim().to_string()
}

// Une ligne du classement : une partie gagnée
#[derive(Clone, Debug, PartialEq)]
pub struct Entree {
    pub nom: String,
    pub niveau: Niveau,
    pub taille: Taille,
    pub score: u32,
    pub duree_secondes: u64,
    // Quand la partie a été gagnée (secondes depuis 1970) : départage les égalités
    pub horodatage: u64,
}

impl Entree {
    // Une entrée par ligne dans le fichier, les champs séparés par des tabulations.
    // La taille est enregistrée par son côté (9 pour une grille 9×9).
    fn vers_ligne(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            self.nom,
            self.niveau.libelle(),
            self.score,
            self.duree_secondes,
            self.horodatage,
            self.taille.cote()
        )
    }

    // Relit une ligne du fichier. Une ligne abîmée donne None et sera ignorée.
    // Les lignes écrites avant l'arrivée des tailles n'ont pas de sixième champ : c'étaient
    // des grilles 9×9.
    fn depuis_ligne(ligne: &str) -> Option<Entree> {
        let mut champs = ligne.split('\t');
        let nom = champs.next()?.to_string();
        let niveau = Niveau::depuis_libelle(champs.next()?)?;
        let score = champs.next()?.parse().ok()?;
        let duree_secondes = champs.next()?.parse().ok()?;
        let horodatage = champs.next()?.parse().ok()?;
        let taille = match champs.next() {
            None => Taille::Neuf,
            Some(cote) => Taille::depuis_cote(cote.parse().ok()?)?,
        };
        if nom.is_empty() || champs.next().is_some() {
            return None;
        }
        Some(Entree {
            nom,
            niveau,
            taille,
            score,
            duree_secondes,
            horodatage,
        })
    }
}

// Où le classement est enregistré : le dossier de données de l'utilisateur,
// pas le dossier du projet (le fichier ne risque donc pas d'être commité)
fn chemin_par_defaut() -> PathBuf {
    let dossier = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .or_else(|| {
            std::env::var_os("HOME").map(|maison| PathBuf::from(maison).join(".local").join("share"))
        });
    match dossier {
        Some(dossier) => dossier.join("Sudoko").join("scores.txt"),
        None => PathBuf::from("sudoko_scores.txt"),
    }
}

// Les scores, du meilleur au moins bon
pub struct Classement {
    entrees: Vec<Entree>,
    chemin: PathBuf,
}

impl Classement {
    // Charge le classement enregistré. S'il n'existe pas encore, il est vide.
    pub fn charger() -> Classement {
        Classement::depuis_fichier(chemin_par_defaut())
    }

    pub fn depuis_fichier(chemin: PathBuf) -> Classement {
        let entrees = fs::read_to_string(&chemin)
            .map(|texte| texte.lines().filter_map(Entree::depuis_ligne).collect())
            .unwrap_or_default();
        let mut classement = Classement { entrees, chemin };
        classement.trier();
        classement
    }

    // Meilleur score d'abord ; à score égal, le plus rapide ; puis le plus ancien
    fn trier(&mut self) {
        self.entrees.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then(a.duree_secondes.cmp(&b.duree_secondes))
                .then(a.horodatage.cmp(&b.horodatage))
        });
    }

    // Ajoute un score et enregistre le fichier. Renvoie le rang obtenu (1 = meilleur score).
    // Si le fichier ne peut pas être écrit, l'entrée n'est pas gardée et on renvoie l'erreur.
    pub fn ajouter(&mut self, entree: Entree) -> Result<usize, String> {
        self.entrees.push(entree.clone());
        self.trier();
        let indice = self
            .entrees
            .iter()
            .position(|autre| *autre == entree)
            .unwrap_or(0);

        if let Err(erreur) = self.sauvegarder() {
            self.entrees.remove(indice);
            return Err(erreur);
        }
        Ok(indice + 1)
    }

    fn sauvegarder(&self) -> Result<(), String> {
        let echec = |erreur: std::io::Error| format!("Impossible d'enregistrer le classement : {}", erreur);

        if let Some(dossier) = self.chemin.parent() {
            if !dossier.as_os_str().is_empty() {
                fs::create_dir_all(dossier).map_err(echec)?;
            }
        }
        let texte: String = self
            .entrees
            .iter()
            .map(|entree| entree.vers_ligne() + "\n")
            .collect();
        // On écrit dans un fichier temporaire puis on le renomme : si le programme s'arrête
        // en plein milieu, l'ancien classement reste intact
        let temporaire = self.chemin.with_extension("tmp");
        fs::write(&temporaire, texte).map_err(echec)?;
        fs::rename(&temporaire, &self.chemin).map_err(echec)
    }

    // Les `nombre` meilleurs scores d'une taille de grille, tous niveaux confondus ou pour un
    // seul niveau. Les tailles ne se mélangent pas : leurs scores ne se comparent pas.
    pub fn meilleurs(&self, taille: Taille, niveau: Option<Niveau>, nombre: usize) -> Vec<&Entree> {
        self.entrees
            .iter()
            .filter(|entree| entree.taille == taille)
            .filter(|entree| niveau.is_none_or(|voulu| entree.niveau == voulu))
            .take(nombre)
            .collect()
    }

    // Le nom du joueur du dernier score enregistré
    pub fn dernier_nom(&self) -> Option<&str> {
        self.entrees
            .iter()
            .max_by_key(|entree| entree.horodatage)
            .map(|entree| entree.nom.as_str())
    }
}

// Un chemin de fichier unique pour les tests, dans le dossier temporaire du système
#[cfg(test)]
pub fn chemin_temporaire() -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COMPTEUR: AtomicUsize = AtomicUsize::new(0);
    let numero = COMPTEUR.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("sudoko_test_{}_{}.txt", std::process::id(), numero))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entree_en(taille: Taille, nom: &str, niveau: Niveau, score: u32, duree: u64, horodatage: u64) -> Entree {
        Entree {
            nom: String::from(nom),
            niveau,
            taille,
            score,
            duree_secondes: duree,
            horodatage,
        }
    }

    // Une entrée pour une grille 9×9
    fn entree(nom: &str, niveau: Niveau, score: u32, duree: u64, horodatage: u64) -> Entree {
        entree_en(Taille::Neuf, nom, niveau, score, duree, horodatage)
    }

    // Supprime le fichier du classement et son fichier temporaire à la fin d'un test
    fn nettoyer(chemin: &PathBuf) {
        let _ = fs::remove_file(chemin);
        let _ = fs::remove_file(chemin.with_extension("tmp"));
    }

    #[test]
    fn test_score_sans_penalite() {
        let zero = Duration::ZERO;
        assert_eq!(calculer_score(Niveau::Facile, Taille::Neuf, zero, 0, 0), 1000);
        assert_eq!(calculer_score(Niveau::Moyen, Taille::Neuf, zero, 0, 0), 2000);
        assert_eq!(calculer_score(Niveau::Difficile, Taille::Neuf, zero, 0, 0), 3000);
    }

    #[test]
    fn test_score_avec_penalites() {
        // 2000 - 100 s - 1 erreur (100) - 1 indice (150)
        let score = calculer_score(Niveau::Moyen, Taille::Neuf, Duration::from_secs(100), 1, 1);
        assert_eq!(score, 2000 - 100 - 100 - 150);
    }

    #[test]
    fn test_score_jamais_negatif() {
        let score = calculer_score(Niveau::Facile, Taille::Neuf, Duration::from_secs(86_400), 2, 9);
        assert_eq!(score, 0);
    }

    #[test]
    fn test_les_points_suivent_la_taille() {
        let zero = Duration::ZERO;
        assert_eq!(calculer_score(Niveau::Facile, Taille::Quatre, zero, 0, 0), 250);
        assert_eq!(calculer_score(Niveau::Facile, Taille::Six, zero, 0, 0), 500);
        assert_eq!(calculer_score(Niveau::Facile, Taille::Douze, zero, 0, 0), 2500);
        assert_eq!(calculer_score(Niveau::Facile, Taille::Seize, zero, 0, 0), 5000);
        assert_eq!(calculer_score(Niveau::Difficile, Taille::Seize, zero, 0, 0), 15_000);
    }

    #[test]
    fn test_les_penalites_suivent_la_taille() {
        assert_eq!(penalite_erreur(Taille::Neuf), 100);
        assert_eq!(penalite_indice(Taille::Neuf), 150);
        assert_eq!(penalite_erreur(Taille::Seize), 500);
        assert_eq!(penalite_indice(Taille::Seize), 750);
        // 16×16 moyen : 10 000 - 1 erreur (500) - 1 indice (750) - 60 s
        let score = calculer_score(Niveau::Moyen, Taille::Seize, Duration::from_secs(60), 1, 1);
        assert_eq!(score, 10_000 - 500 - 750 - 60);
    }

    #[test]
    fn test_points_et_penalites_comme_dans_le_readme() {
        // (taille, points de départ facile / moyen / difficile, pénalité par erreur, par indice)
        let attendu = [
            (Taille::Quatre, [250, 500, 750], 25, 37),
            (Taille::Six, [500, 1000, 1500], 50, 75),
            (Taille::Neuf, [1000, 2000, 3000], 100, 150),
            (Taille::Douze, [2500, 5000, 7500], 250, 375),
            (Taille::Seize, [5000, 10_000, 15_000], 500, 750),
        ];
        for (taille, points, erreur, indice) in attendu {
            for (niveau, depart) in Niveau::TOUS.into_iter().zip(points) {
                assert_eq!(points_de_base(niveau, taille), depart, "{} {}", niveau.libelle(), taille.libelle());
            }
            assert_eq!(penalite_erreur(taille), erreur, "{}", taille.libelle());
            assert_eq!(penalite_indice(taille), indice, "{}", taille.libelle());
        }
    }

    #[test]
    fn test_plus_la_grille_est_grande_plus_les_points_de_depart_sont_eleves() {
        for niveau in Niveau::TOUS {
            let points: Vec<u64> = Taille::TOUTES
                .iter()
                .map(|&taille| points_de_base(niveau, taille))
                .collect();
            assert!(points.windows(2).all(|paire| paire[0] < paire[1]), "{:?}", points);
        }
    }

    #[test]
    fn test_nettoyer_nom() {
        assert_eq!(nettoyer_nom("  Lucas  "), "Lucas");
        assert_eq!(nettoyer_nom("Lu\tcas\n"), "Lucas");
        assert_eq!(nettoyer_nom("   "), "");
        let long = "a".repeat(50);
        assert_eq!(nettoyer_nom(&long).chars().count(), 20);
    }

    #[test]
    fn test_tri_par_score_puis_duree_puis_anciennete() {
        let chemin = chemin_temporaire();
        let mut classement = Classement::depuis_fichier(chemin.clone());
        classement.ajouter(entree("C", Niveau::Moyen, 1000, 300, 3)).unwrap();
        classement.ajouter(entree("A", Niveau::Moyen, 1500, 400, 1)).unwrap();
        classement.ajouter(entree("B", Niveau::Moyen, 1000, 200, 2)).unwrap();
        classement.ajouter(entree("D", Niveau::Moyen, 1000, 200, 0)).unwrap();

        let noms: Vec<&str> = classement
            .meilleurs(Taille::Neuf, None, 10)
            .iter()
            .map(|e| e.nom.as_str())
            .collect();
        // A a le meilleur score ; D et B sont à égalité de score et de temps : D est plus ancien
        assert_eq!(noms, ["A", "D", "B", "C"]);
        nettoyer(&chemin);
    }

    #[test]
    fn test_ajouter_renvoie_le_rang() {
        let chemin = chemin_temporaire();
        let mut classement = Classement::depuis_fichier(chemin.clone());
        assert_eq!(classement.ajouter(entree("A", Niveau::Facile, 500, 100, 1)), Ok(1));
        assert_eq!(classement.ajouter(entree("B", Niveau::Facile, 900, 100, 2)), Ok(1));
        assert_eq!(classement.ajouter(entree("C", Niveau::Facile, 700, 100, 3)), Ok(2));
        nettoyer(&chemin);
    }

    #[test]
    fn test_le_classement_survit_au_rechargement() {
        let chemin = chemin_temporaire();
        let mut classement = Classement::depuis_fichier(chemin.clone());
        let premiere = entree("Lucas", Niveau::Difficile, 2500, 640, 1_700_000_000);
        let seconde = entree_en(Taille::Douze, "Autre", Niveau::Facile, 800, 120, 1_700_000_100);
        classement.ajouter(premiere.clone()).unwrap();
        classement.ajouter(seconde.clone()).unwrap();

        // La taille est bien conservée : chaque entrée se retrouve dans sa taille
        let recharge = Classement::depuis_fichier(chemin.clone());
        let neuf: Vec<Entree> = recharge.meilleurs(Taille::Neuf, None, 10).into_iter().cloned().collect();
        let douze: Vec<Entree> = recharge.meilleurs(Taille::Douze, None, 10).into_iter().cloned().collect();
        assert_eq!(neuf, [premiere]);
        assert_eq!(douze, [seconde]);
        nettoyer(&chemin);
    }

    #[test]
    fn test_les_tailles_ne_se_melangent_pas() {
        let chemin = chemin_temporaire();
        let mut classement = Classement::depuis_fichier(chemin.clone());
        classement.ajouter(entree_en(Taille::Quatre, "Petit", Niveau::Facile, 200, 20, 1)).unwrap();
        classement.ajouter(entree_en(Taille::Seize, "Grand", Niveau::Facile, 4000, 900, 2)).unwrap();
        classement.ajouter(entree("Moyen", Niveau::Facile, 800, 100, 3)).unwrap();

        // Chaque taille a son classement : un score de 16×16 ne passe pas devant un score de 4×4
        assert_eq!(classement.meilleurs(Taille::Quatre, None, 10).len(), 1);
        assert_eq!(classement.meilleurs(Taille::Seize, None, 10).len(), 1);
        assert_eq!(classement.meilleurs(Taille::Neuf, None, 10).len(), 1);
        assert!(classement.meilleurs(Taille::Six, None, 10).is_empty());
        nettoyer(&chemin);
    }

    #[test]
    fn test_les_lignes_abimees_sont_ignorees() {
        let chemin = chemin_temporaire();
        let contenu = "n'importe quoi\n\
                       Lucas\tFacile\t800\t120\t5\n\
                       Lucas\tInconnu\t800\t120\t5\n\
                       Lucas\tFacile\tpas_un_nombre\t120\t5\n\
                       Lucas\tFacile\t800\t120\t5\t9\textra\n\
                       Lucas\tFacile\t800\t120\t5\t7\n\
                       Lucas\tFacile\t800\t120\t5\tgrand\n\
                       \tFacile\t800\t120\t5\n";
        fs::write(&chemin, contenu).unwrap();
        let classement = Classement::depuis_fichier(chemin.clone());
        // Seule la deuxième ligne est valide, et elle compte pour une grille 9×9
        assert_eq!(classement.meilleurs(Taille::Neuf, None, 10).len(), 1);
        for taille in Taille::TOUTES {
            if taille != Taille::Neuf {
                assert!(classement.meilleurs(taille, None, 10).is_empty());
            }
        }
        nettoyer(&chemin);
    }

    #[test]
    fn test_les_anciennes_lignes_sans_taille_sont_des_grilles_9x9() {
        let chemin = chemin_temporaire();
        // Le format d'avant les tailles : cinq champs
        fs::write(&chemin, "Lucas\tMoyen\t1500\t300\t1700000000\n").unwrap();
        let classement = Classement::depuis_fichier(chemin.clone());
        let neuf = classement.meilleurs(Taille::Neuf, None, 10);
        assert_eq!(neuf.len(), 1);
        assert_eq!(neuf[0].nom, "Lucas");
        assert_eq!(neuf[0].score, 1500);
        nettoyer(&chemin);
    }

    #[test]
    fn test_fichier_absent_donne_un_classement_vide() {
        let classement = Classement::depuis_fichier(chemin_temporaire());
        assert!(classement.meilleurs(Taille::Neuf, None, 10).is_empty());
        assert_eq!(classement.dernier_nom(), None);
    }

    #[test]
    fn test_filtre_par_niveau_et_limite() {
        let chemin = chemin_temporaire();
        let mut classement = Classement::depuis_fichier(chemin.clone());
        classement.ajouter(entree("A", Niveau::Facile, 900, 100, 1)).unwrap();
        classement.ajouter(entree("B", Niveau::Moyen, 1800, 100, 2)).unwrap();
        classement.ajouter(entree("C", Niveau::Facile, 700, 100, 3)).unwrap();
        classement.ajouter(entree("D", Niveau::Facile, 600, 100, 4)).unwrap();

        let faciles = classement.meilleurs(Taille::Neuf, Some(Niveau::Facile), 10);
        assert_eq!(faciles.len(), 3);
        assert!(faciles.iter().all(|e| e.niveau == Niveau::Facile));
        assert_eq!(classement.meilleurs(Taille::Neuf, None, 2).len(), 2);
        assert!(classement.meilleurs(Taille::Neuf, Some(Niveau::Difficile), 10).is_empty());
        nettoyer(&chemin);
    }

    #[test]
    fn test_dernier_nom() {
        let chemin = chemin_temporaire();
        let mut classement = Classement::depuis_fichier(chemin.clone());
        classement.ajouter(entree("Ancien", Niveau::Moyen, 1900, 100, 1)).unwrap();
        classement.ajouter(entree("Recent", Niveau::Moyen, 100, 100, 2)).unwrap();
        // Le plus récent, pas le meilleur
        assert_eq!(classement.dernier_nom(), Some("Recent"));
        nettoyer(&chemin);
    }

    #[test]
    fn test_echec_de_sauvegarde_n_ajoute_rien() {
        // Le « dossier » du classement est en fait un fichier : impossible d'écrire dedans
        let fichier = chemin_temporaire();
        fs::write(&fichier, "je suis un fichier").unwrap();
        let mut classement = Classement::depuis_fichier(fichier.join("scores.txt"));

        let resultat = classement.ajouter(entree("A", Niveau::Facile, 900, 100, 1));
        assert!(resultat.is_err());
        assert!(classement.meilleurs(Taille::Neuf, None, 10).is_empty());
        nettoyer(&fichier);
    }

    #[test]
    fn test_niveau_depuis_libelle() {
        for niveau in Niveau::TOUS {
            assert_eq!(Niveau::depuis_libelle(niveau.libelle()), Some(niveau));
        }
        assert_eq!(Niveau::depuis_libelle("facile"), None);
    }
}
