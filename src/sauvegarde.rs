//! La sauvegarde de la partie en cours, pour la reprendre à la prochaine ouverture du jeu.

use crate::grille::{Grille, Niveau, Origine, Taille};
use crate::regles::Variante;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// Première ligne du fichier : elle permet de refuser un fichier d'un autre format
const ENTETE: &str = "sudoko-partie 1";

// Le dossier où le jeu range ses fichiers : le dossier de données de l'utilisateur,
// pas le dossier du projet (les fichiers ne risquent donc pas d'être commités).
// None si le système n'en indique aucun.
pub fn dossier_donnees() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .or_else(|| {
            std::env::var_os("HOME").map(|maison| PathBuf::from(maison).join(".local").join("share"))
        })
        .map(|dossier| dossier.join("Sudoko"))
}

// Écrit un fichier en créant son dossier au besoin. On écrit d'abord dans un fichier temporaire
// puis on le renomme : si le programme s'arrête en plein milieu, l'ancien fichier reste intact.
pub fn ecrire_fichier(chemin: &Path, texte: &str) -> std::io::Result<()> {
    if let Some(dossier) = chemin.parent() {
        if !dossier.as_os_str().is_empty() {
            fs::create_dir_all(dossier)?;
        }
    }
    let temporaire = chemin.with_extension("tmp");
    fs::write(&temporaire, texte)?;
    fs::rename(&temporaire, chemin)
}

// Tout ce qu'il faut pour reprendre une partie là où on l'a laissée
pub struct PartieSauvee {
    pub grille: Grille,
    pub origine: Origine,
    pub erreurs: u32,
    pub indices: u32,
    // Le temps de jeu déjà écoulé
    pub secondes: u64,
    // Les notes au crayon de chaque case, en bits (le bit v - 1 = la note v), dans l'ordre des cases
    pub notes: Vec<u16>,
}

impl PartieSauvee {
    // Une information par ligne : le nom, une tabulation, puis la valeur.
    // Les cases sont des nombres séparés par des virgules ; les cases de départ,
    // une suite de 1 (départ) et de 0 (case du joueur).
    fn vers_texte(&self) -> String {
        let origine = match self.origine {
            Origine::Fixe => "fixe",
            Origine::Niveau(niveau, _, _) => niveau.libelle(),
        };
        let nombres = |valeurs: &[u8]| {
            valeurs
                .iter()
                .map(u8::to_string)
                .collect::<Vec<String>>()
                .join(",")
        };
        let fixes: String = self
            .grille
            .fixes()
            .iter()
            .map(|&fixe| if fixe { '1' } else { '0' })
            .collect();

        let mut texte = format!(
            "{}\norigine\t{}\ntaille\t{}\nvariante\t{}\nnom\t{}\nerreurs\t{}\nindices\t{}\nsecondes\t{}\ncases\t{}\nfixes\t{}\n",
            ENTETE,
            origine,
            self.grille.taille().cote(),
            self.grille.variante().mot(),
            self.grille.nom,
            self.erreurs,
            self.indices,
            self.secondes,
            nombres(self.grille.cases()),
            fixes
        );
        if let Some(solution) = self.grille.solution() {
            texte += &format!("solution\t{}\n", nombres(solution));
        }
        // Sans note, on n'écrit rien
        if self.notes.iter().any(|&note| note != 0) {
            let notes: Vec<String> = self.notes.iter().map(u16::to_string).collect();
            texte += &format!("notes\t{}\n", notes.join(","));
        }
        texte
    }

    // Relit une sauvegarde. Un fichier abîmé ou incohérent donne None : la partie est
    // simplement perdue, le jeu ne plante pas.
    fn depuis_texte(texte: &str) -> Option<PartieSauvee> {
        let mut lignes = texte.lines();
        if lignes.next()? != ENTETE {
            return None;
        }
        let mut champs: HashMap<&str, &str> = HashMap::new();
        for ligne in lignes {
            let (cle, valeur) = ligne.split_once('\t')?;
            champs.insert(cle, valeur);
        }

        let taille = Taille::depuis_cote(champs.get("taille")?.parse().ok()?)?;
        // Les sauvegardes écrites avant les variantes n'ont pas de ligne `variante` : classique
        let variante = match champs.get("variante") {
            None => Variante::Classique,
            Some(mot) => Variante::depuis_mot(mot)?,
        };
        if !variante.accepte(taille) {
            return None;
        }
        let origine = match *champs.get("origine")? {
            "fixe" if taille == Taille::Neuf && variante == Variante::Classique => Origine::Fixe,
            "fixe" => return None,
            libelle => Origine::Niveau(Niveau::depuis_libelle(libelle)?, taille, variante),
        };
        let nombres = |texte: &str| -> Option<Vec<u8>> {
            texte.split(',').map(|nombre| nombre.parse().ok()).collect()
        };
        let fixes: Vec<bool> = champs
            .get("fixes")?
            .chars()
            .map(|c| match c {
                '1' => Some(true),
                '0' => Some(false),
                _ => None,
            })
            .collect::<Option<_>>()?;
        let solution = match champs.get("solution") {
            Some(texte) => Some(nombres(texte)?),
            None => None,
        };
        let grille = Grille::restaurer(
            taille,
            variante,
            champs.get("nom")?.to_string(),
            nombres(champs.get("cases")?)?,
            fixes,
            solution,
        )
        .ok()?;

        // Les notes sont facultatives (les anciennes sauvegardes n'en ont pas)
        let notes = match champs.get("notes") {
            Some(texte) => texte
                .split(',')
                .map(|note| note.parse().ok())
                .collect::<Option<Vec<u16>>>()?,
            None => vec![0; taille.cases()],
        };
        // Une note par case, avec des chiffres de la grille, et seulement dans une case vide
        let cote = taille.cote();
        let notes_valides = notes.len() == taille.cases()
            && notes.iter().zip(grille.cases()).all(|(&note, &valeur)| {
                (note as u32) < (1 << cote) && (note == 0 || valeur == 0)
            });
        if !notes_valides {
            return None;
        }

        Some(PartieSauvee {
            grille,
            origine,
            erreurs: champs.get("erreurs")?.parse().ok()?,
            indices: champs.get("indices")?.parse().ok()?,
            secondes: champs.get("secondes")?.parse().ok()?,
            notes,
        })
    }
}

// Le fichier où la partie en cours est enregistrée
pub struct Sauvegarde {
    chemin: PathBuf,
}

impl Sauvegarde {
    pub fn par_defaut() -> Sauvegarde {
        let chemin = match dossier_donnees() {
            Some(dossier) => dossier.join("partie.txt"),
            None => PathBuf::from("sudoko_partie.txt"),
        };
        Sauvegarde::depuis_fichier(chemin)
    }

    pub fn depuis_fichier(chemin: PathBuf) -> Sauvegarde {
        Sauvegarde { chemin }
    }

    // La partie enregistrée, s'il y en a une de lisible
    pub fn charger(&self) -> Option<PartieSauvee> {
        let texte = fs::read_to_string(&self.chemin).ok()?;
        PartieSauvee::depuis_texte(&texte)
    }

    // Remplace la sauvegarde par cette partie
    pub fn enregistrer(&self, partie: &PartieSauvee) -> Result<(), String> {
        ecrire_fichier(&self.chemin, &partie.vers_texte())
            .map_err(|erreur| format!("Impossible de sauvegarder la partie : {}", erreur))
    }

    // Efface la sauvegarde : la partie est finie. Sans fichier, il n'y a rien à faire.
    pub fn supprimer(&self) {
        let _ = fs::remove_file(&self.chemin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classement::chemin_temporaire;

    fn nettoyer(sauvegarde: &Sauvegarde) {
        let _ = fs::remove_file(&sauvegarde.chemin);
        let _ = fs::remove_file(sauvegarde.chemin.with_extension("tmp"));
    }

    // Une partie 6×6 où le joueur a posé un bon chiffre et un chiffre faux
    fn partie_en_cours() -> PartieSauvee {
        let mut grille = Grille::generer_niveau(Niveau::Moyen, Taille::Six, Variante::Classique);
        let solution = grille.solution().unwrap().to_vec();
        let vides: Vec<usize> = (0..36).filter(|&i| !grille.fixes()[i]).collect();
        let (ligne, col) = (vides[0] / 6, vides[0] % 6);
        assert!(grille.placer(ligne, col, solution[vides[0]]));
        // Et deux notes (les chiffres 1 et 3) dans une autre case vide
        let mut notes = vec![0; 36];
        notes[vides[1]] = 0b101;
        PartieSauvee {
            grille,
            origine: Origine::Niveau(Niveau::Moyen, Taille::Six, Variante::Classique),
            erreurs: 2,
            indices: 1,
            secondes: 754,
            notes,
        }
    }

    #[test]
    fn test_aller_retour_d_une_grille_generee() {
        let partie = partie_en_cours();
        let relue = PartieSauvee::depuis_texte(&partie.vers_texte()).unwrap();
        assert_eq!(relue.origine, partie.origine);
        assert_eq!(relue.erreurs, 2);
        assert_eq!(relue.indices, 1);
        assert_eq!(relue.secondes, 754);
        assert_eq!(relue.grille.nom, partie.grille.nom);
        assert_eq!(relue.grille.taille(), Taille::Six);
        assert_eq!(relue.grille.cases(), partie.grille.cases());
        assert_eq!(relue.grille.fixes(), partie.grille.fixes());
        assert_eq!(relue.grille.solution(), partie.grille.solution());
        assert_eq!(relue.notes, partie.notes);
    }

    #[test]
    fn test_aller_retour_dans_chaque_variante() {
        for variante in Variante::TOUTES {
            let taille = Taille::Neuf;
            let partie = PartieSauvee {
                grille: Grille::generer_niveau(Niveau::Facile, taille, variante),
                origine: Origine::Niveau(Niveau::Facile, taille, variante),
                erreurs: 1,
                indices: 0,
                secondes: 42,
                notes: vec![0; 81],
            };
            let texte = partie.vers_texte();
            assert!(texte.contains(&format!("variante\t{}", variante.mot())));
            let relue = PartieSauvee::depuis_texte(&texte).unwrap();
            assert_eq!(relue.origine, Origine::Niveau(Niveau::Facile, taille, variante));
            assert_eq!(relue.grille.variante(), variante);
            assert_eq!(relue.grille.cases(), partie.grille.cases());
            assert_eq!(relue.grille.solution(), partie.grille.solution());
            // Les règles de la variante sont bien celles de la grille relue
            assert_eq!(relue.grille.regles().zones.len(), partie.grille.regles().zones.len());
        }
    }

    #[test]
    fn test_une_ancienne_sauvegarde_sans_variante_est_classique() {
        let partie = partie_en_cours();
        let sans_variante: String = partie
            .vers_texte()
            .lines()
            .filter(|ligne| !ligne.starts_with("variante"))
            .map(|ligne| format!("{}\n", ligne))
            .collect();
        let relue = PartieSauvee::depuis_texte(&sans_variante).unwrap();
        assert_eq!(relue.grille.variante(), Variante::Classique);
    }

    #[test]
    fn test_une_variante_inconnue_ou_impossible_est_refusee() {
        let texte = partie_en_cours().vers_texte();
        assert!(PartieSauvee::depuis_texte(&texte.replace("variante\tclassique", "variante\tinconnue")).is_none());
        // La variante hyper n'existe qu'en 9×9 : cette partie est en 6×6
        assert!(PartieSauvee::depuis_texte(&texte.replace("variante\tclassique", "variante\thyper")).is_none());
        // Une variante autre que classique n'est pas la grille d'entraînement
        let entrainement = PartieSauvee {
            grille: Grille::creer(Origine::Fixe).unwrap(),
            origine: Origine::Fixe,
            erreurs: 0,
            indices: 0,
            secondes: 0,
            notes: vec![0; 81],
        }
        .vers_texte();
        assert!(PartieSauvee::depuis_texte(&entrainement.replace("variante\tclassique", "variante\tdiagonale")).is_none());
    }

    #[test]
    fn test_aller_retour_de_la_grille_d_entrainement() {
        let partie = PartieSauvee {
            grille: Grille::creer(Origine::Fixe).unwrap(),
            origine: Origine::Fixe,
            erreurs: 0,
            indices: 0,
            secondes: 0,
            notes: vec![0; 81],
        };
        let relue = PartieSauvee::depuis_texte(&partie.vers_texte()).unwrap();
        assert_eq!(relue.origine, Origine::Fixe);
        assert_eq!(relue.grille.cases(), partie.grille.cases());
    }

    #[test]
    fn test_aller_retour_dans_chaque_taille() {
        for taille in Taille::TOUTES {
            let grille = Grille::generer_niveau(Niveau::Facile, taille, Variante::Classique);
            // Une case vide avec tous les chiffres en notes (65535 en 16×16)
            let mut notes = vec![0; taille.cases()];
            let vide = grille.cases().iter().position(|&valeur| valeur == 0).unwrap();
            notes[vide] = ((1u32 << taille.cote()) - 1) as u16;
            let partie = PartieSauvee {
                grille,
                origine: Origine::Niveau(Niveau::Facile, taille, Variante::Classique),
                erreurs: 0,
                indices: 0,
                secondes: 5,
                notes,
            };
            let relue = PartieSauvee::depuis_texte(&partie.vers_texte()).unwrap();
            assert_eq!(relue.grille.taille(), taille);
            assert_eq!(relue.grille.cases(), partie.grille.cases());
            assert_eq!(relue.grille.solution(), partie.grille.solution());
            assert_eq!(relue.notes, partie.notes);
        }
    }

    #[test]
    fn test_un_fichier_abime_est_refuse() {
        let texte = partie_en_cours().vers_texte();

        // Mauvais en-tête, texte vide, ligne sans tabulation
        assert!(PartieSauvee::depuis_texte("").is_none());
        assert!(PartieSauvee::depuis_texte(&texte.replace(ENTETE, "autre chose")).is_none());
        assert!(PartieSauvee::depuis_texte(&format!("{}\nn'importe quoi\n", ENTETE)).is_none());

        // Un champ qui manque
        let sans_erreurs: String = texte
            .lines()
            .filter(|ligne| !ligne.starts_with("erreurs"))
            .map(|ligne| format!("{}\n", ligne))
            .collect();
        assert!(PartieSauvee::depuis_texte(&sans_erreurs).is_none());

        // Des valeurs qui ne sont pas des nombres, une origine ou une taille inconnue
        assert!(PartieSauvee::depuis_texte(&texte.replace("secondes\t754", "secondes\tbeaucoup")).is_none());
        assert!(PartieSauvee::depuis_texte(&texte.replace("origine\tMoyen", "origine\tImpossible")).is_none());
        assert!(PartieSauvee::depuis_texte(&texte.replace("taille\t6", "taille\t7")).is_none());
        assert!(PartieSauvee::depuis_texte(&texte.replace("origine\tMoyen", "origine\tfixe")).is_none());
    }

    #[test]
    fn test_des_cases_incoherentes_sont_refusees() {
        let partie = partie_en_cours();
        let texte = partie.vers_texte();
        let cases: Vec<String> = partie.grille.cases().iter().map(u8::to_string).collect();
        let cases = format!("cases\t{}", cases.join(","));
        assert!(texte.contains(&cases));

        // Un chiffre trop grand pour la grille
        let trop_grand = format!("cases\t7,{}", cases["cases\t".len() + 2..].to_string());
        assert!(PartieSauvee::depuis_texte(&texte.replace(&cases, &trop_grand)).is_none());

        // Trop peu de cases
        let court = cases.rsplit_once(',').unwrap().0.to_string();
        assert!(PartieSauvee::depuis_texte(&texte.replace(&cases, &court)).is_none());

        // Un doublon dans une ligne, entre deux cases de départ : les deux premières cases reçoivent le
        // même chiffre et sont déclarées cases de départ
        let doublon = format!("cases\t1,1,{}", cases.splitn(3, ',').nth(2).unwrap());
        let fixes: String = partie.grille.fixes().iter().map(|&f| if f { '1' } else { '0' }).collect();
        let departs = format!("fixes\t11{}", &fixes[2..]);
        let texte_en_doublon = texte.replace(&cases, &doublon).replace(&format!("fixes\t{}", fixes), &departs);
        assert!(PartieSauvee::depuis_texte(&texte_en_doublon).is_none());

        // Entre des chiffres du joueur, en revanche, un doublon est permis : il a le droit de se tromper
        let texte_du_joueur = texte.replace(&cases, &doublon).replace(&format!("fixes\t{}", fixes), &format!("fixes\t00{}", &fixes[2..]));
        // (sans la ligne des notes, qui pourrait viser une des deux cases désormais remplies)
        let sans_notes: String = texte_du_joueur
            .lines()
            .filter(|ligne| !ligne.starts_with("notes"))
            .map(|ligne| format!("{}\n", ligne))
            .collect();
        assert!(PartieSauvee::depuis_texte(&sans_notes).is_some());

        // Des cases de départ qui ne sont ni des 0 ni des 1
        let abimees = fixes.replacen('0', "x", 1);
        assert!(PartieSauvee::depuis_texte(&texte.replace(&fixes, &abimees)).is_none());
    }

    #[test]
    fn test_une_sauvegarde_sans_notes_se_relit() {
        // Les sauvegardes écrites avant les notes n'ont pas de ligne `notes`
        let mut partie = partie_en_cours();
        partie.notes = vec![0; 36];
        let texte = partie.vers_texte();
        assert!(!texte.contains("notes"));
        let relue = PartieSauvee::depuis_texte(&texte).unwrap();
        assert_eq!(relue.notes, vec![0; 36]);
    }

    #[test]
    fn test_des_notes_incoherentes_sont_refusees() {
        let partie = partie_en_cours();
        let texte = partie.vers_texte();
        let ligne = texte.lines().find(|ligne| ligne.starts_with("notes")).unwrap();

        // Trop peu de notes
        let court = ligne.rsplit_once(',').unwrap().0;
        assert!(PartieSauvee::depuis_texte(&texte.replace(ligne, court)).is_none());

        // Une note avec un chiffre qui n'existe pas en 6×6 (le chiffre 7 : bit 6)
        let trop_grand = ligne.replace("5,", "64,");
        assert!(PartieSauvee::depuis_texte(&texte.replace(ligne, &trop_grand)).is_none());

        // Pas un nombre
        let pas_un_nombre = ligne.replace("5,", "x,");
        assert!(PartieSauvee::depuis_texte(&texte.replace(ligne, &pas_un_nombre)).is_none());

        // Une note dans une case de départ : la première case de départ reçoit une note
        let depart = partie.grille.fixes().iter().position(|&fixe| fixe).unwrap();
        let mut notes = partie.notes.clone();
        notes[depart] = 1;
        let liste: Vec<String> = notes.iter().map(u16::to_string).collect();
        let sur_un_depart = format!("notes\t{}", liste.join(","));
        assert!(PartieSauvee::depuis_texte(&texte.replace(ligne, &sur_un_depart)).is_none());
    }

    #[test]
    fn test_une_solution_incoherente_est_refusee() {
        let texte = partie_en_cours().vers_texte();
        let ligne = texte.lines().find(|ligne| ligne.starts_with("solution")).unwrap();
        let sans_la_fin = ligne.rsplit_once(',').unwrap().0;
        assert!(PartieSauvee::depuis_texte(&texte.replace(ligne, sans_la_fin)).is_none());
        // Une solution pleine de 1 : elle contient des doublons
        let uns = format!("solution\t{}", vec!["1"; 36].join(","));
        assert!(PartieSauvee::depuis_texte(&texte.replace(ligne, &uns)).is_none());
    }

    #[test]
    fn test_enregistrer_puis_charger() {
        let sauvegarde = Sauvegarde::depuis_fichier(chemin_temporaire());
        assert!(sauvegarde.charger().is_none());

        let partie = partie_en_cours();
        sauvegarde.enregistrer(&partie).unwrap();
        let relue = sauvegarde.charger().unwrap();
        assert_eq!(relue.secondes, 754);
        assert_eq!(relue.grille.cases(), partie.grille.cases());

        // Une nouvelle sauvegarde remplace l'ancienne
        let mut suivante = partie_en_cours();
        suivante.erreurs = 0;
        suivante.secondes = 10;
        sauvegarde.enregistrer(&suivante).unwrap();
        let relue = sauvegarde.charger().unwrap();
        assert_eq!(relue.secondes, 10);
        assert_eq!(relue.erreurs, 0);
        assert!(!sauvegarde.chemin.with_extension("tmp").exists());
        nettoyer(&sauvegarde);
    }

    #[test]
    fn test_supprimer() {
        let sauvegarde = Sauvegarde::depuis_fichier(chemin_temporaire());
        // Sans fichier, supprimer ne fait rien
        sauvegarde.supprimer();

        sauvegarde.enregistrer(&partie_en_cours()).unwrap();
        assert!(sauvegarde.charger().is_some());
        sauvegarde.supprimer();
        assert!(sauvegarde.charger().is_none());
        nettoyer(&sauvegarde);
    }

    #[test]
    fn test_un_fichier_illisible_n_empeche_pas_de_jouer() {
        let sauvegarde = Sauvegarde::depuis_fichier(chemin_temporaire());
        fs::write(&sauvegarde.chemin, "ceci n'est pas une sauvegarde").unwrap();
        assert!(sauvegarde.charger().is_none());
        // Et on peut l'écraser par une vraie sauvegarde
        sauvegarde.enregistrer(&partie_en_cours()).unwrap();
        assert!(sauvegarde.charger().is_some());
        nettoyer(&sauvegarde);
    }
}
