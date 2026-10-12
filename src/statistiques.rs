//! Les statistiques : chaque partie terminée est retenue dans un fichier, pour compter
//! les victoires, les meilleurs temps et les séries.

use crate::grille::{Niveau, Taille};
use crate::sauvegarde::{dossier_donnees, ecrire_fichier};
use std::fs;
use std::path::PathBuf;

// Comment une partie s'est terminée
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Resultat {
    Victoire,
    Defaite,
    Abandon,
}

impl Resultat {
    // Le mot écrit dans le fichier
    fn mot(self) -> &'static str {
        match self {
            Resultat::Victoire => "victoire",
            Resultat::Defaite => "defaite",
            Resultat::Abandon => "abandon",
        }
    }

    fn depuis_mot(mot: &str) -> Option<Resultat> {
        [Resultat::Victoire, Resultat::Defaite, Resultat::Abandon]
            .into_iter()
            .find(|resultat| resultat.mot() == mot)
    }
}

// Une partie terminée
#[derive(Clone, Debug, PartialEq)]
pub struct PartieJouee {
    // Quand elle s'est terminée (secondes depuis 1970) : c'est l'ordre des parties
    pub horodatage: u64,
    pub niveau: Niveau,
    pub taille: Taille,
    pub resultat: Resultat,
    pub duree_secondes: u64,
    pub erreurs: u32,
    pub indices: u32,
    // Le score d'une victoire, 0 sinon
    pub score: u32,
}

impl PartieJouee {
    // Une partie par ligne, les champs séparés par des tabulations
    fn vers_ligne(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.horodatage,
            self.taille.cote(),
            self.niveau.libelle(),
            self.resultat.mot(),
            self.duree_secondes,
            self.erreurs,
            self.indices,
            self.score
        )
    }

    // Relit une ligne. Une ligne abîmée donne None et sera ignorée.
    fn depuis_ligne(ligne: &str) -> Option<PartieJouee> {
        let mut champs = ligne.split('\t');
        let horodatage = champs.next()?.parse().ok()?;
        let taille = Taille::depuis_cote(champs.next()?.parse().ok()?)?;
        let niveau = Niveau::depuis_libelle(champs.next()?)?;
        let resultat = Resultat::depuis_mot(champs.next()?)?;
        let duree_secondes = champs.next()?.parse().ok()?;
        let erreurs = champs.next()?.parse().ok()?;
        let indices = champs.next()?.parse().ok()?;
        let score = champs.next()?.parse().ok()?;
        if champs.next().is_some() {
            return None;
        }
        Some(PartieJouee {
            horodatage,
            niveau,
            taille,
            resultat,
            duree_secondes,
            erreurs,
            indices,
            score,
        })
    }
}

// Les chiffres d'un ensemble de parties (une taille, un niveau ou tous)
#[derive(Debug, Default, PartialEq)]
pub struct Resume {
    pub jouees: usize,
    pub victoires: usize,
    pub defaites: usize,
    pub abandons: usize,
    pub meilleur_temps: Option<u64>,
    // La durée moyenne des parties gagnées
    pub temps_moyen: Option<u64>,
    pub meilleur_score: Option<u32>,
    // Les victoires d'affilée, jusqu'à la dernière partie
    pub serie_en_cours: usize,
    pub meilleure_serie: usize,
}

impl Resume {
    // Le pourcentage de parties gagnées, arrondi. None sans aucune partie.
    pub fn taux_de_victoire(&self) -> Option<u32> {
        if self.jouees == 0 {
            return None;
        }
        Some(((self.victoires * 100 + self.jouees / 2) / self.jouees) as u32)
    }
}

// Où les statistiques sont enregistrées : dans le dossier de données de l'utilisateur
fn chemin_par_defaut() -> PathBuf {
    match dossier_donnees() {
        Some(dossier) => dossier.join("statistiques.txt"),
        None => PathBuf::from("sudoko_statistiques.txt"),
    }
}

// Toutes les parties terminées, de la plus ancienne à la plus récente
pub struct Statistiques {
    parties: Vec<PartieJouee>,
    chemin: PathBuf,
}

impl Statistiques {
    // Charge les statistiques enregistrées. Sans fichier, il n'y a encore aucune partie.
    pub fn charger() -> Statistiques {
        Statistiques::depuis_fichier(chemin_par_defaut())
    }

    pub fn depuis_fichier(chemin: PathBuf) -> Statistiques {
        let mut parties: Vec<PartieJouee> = fs::read_to_string(&chemin)
            .map(|texte| texte.lines().filter_map(PartieJouee::depuis_ligne).collect())
            .unwrap_or_default();
        // Un tri stable : à la même seconde, l'ordre du fichier est gardé
        parties.sort_by_key(|partie| partie.horodatage);
        Statistiques { parties, chemin }
    }

    // Retient une partie terminée et enregistre le fichier.
    // Si le fichier ne peut pas être écrit, la partie n'est pas gardée et on renvoie l'erreur.
    pub fn ajouter(&mut self, partie: PartieJouee) -> Result<(), String> {
        self.parties.push(partie);
        let texte: String = self
            .parties
            .iter()
            .map(|partie| partie.vers_ligne() + "\n")
            .collect();
        if let Err(erreur) = ecrire_fichier(&self.chemin, &texte) {
            self.parties.pop();
            return Err(format!("Impossible d'enregistrer les statistiques : {}", erreur));
        }
        Ok(())
    }

    // Supprime le fichier et son fichier temporaire (pour les tests qui passent par l'application)
    #[cfg(test)]
    pub fn effacer_fichier(&self) {
        let _ = fs::remove_file(&self.chemin);
        let _ = fs::remove_file(self.chemin.with_extension("tmp"));
    }

    // Les chiffres d'une taille de grille, tous niveaux confondus ou pour un seul niveau.
    // Les tailles ne se mélangent pas : une partie en 4×4 et une en 16×16 ne se comparent pas.
    pub fn resume(&self, taille: Taille, niveau: Option<Niveau>) -> Resume {
        let mut resume = Resume::default();
        let mut somme_des_temps = 0;

        let concernees = self
            .parties
            .iter()
            .filter(|partie| partie.taille == taille)
            .filter(|partie| niveau.is_none_or(|voulu| partie.niveau == voulu));
        for partie in concernees {
            resume.jouees += 1;
            match partie.resultat {
                Resultat::Victoire => {
                    resume.victoires += 1;
                    resume.serie_en_cours += 1;
                    resume.meilleure_serie = resume.meilleure_serie.max(resume.serie_en_cours);
                    somme_des_temps += partie.duree_secondes;
                    resume.meilleur_temps =
                        Some(resume.meilleur_temps.map_or(partie.duree_secondes, |t| t.min(partie.duree_secondes)));
                    resume.meilleur_score =
                        Some(resume.meilleur_score.map_or(partie.score, |s| s.max(partie.score)));
                }
                Resultat::Defaite => {
                    resume.defaites += 1;
                    resume.serie_en_cours = 0;
                }
                Resultat::Abandon => {
                    resume.abandons += 1;
                    resume.serie_en_cours = 0;
                }
            }
        }
        if resume.victoires > 0 {
            resume.temps_moyen = Some(somme_des_temps / resume.victoires as u64);
        }
        resume
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classement::chemin_temporaire;

    fn partie(taille: Taille, niveau: Niveau, resultat: Resultat, duree: u64, horodatage: u64) -> PartieJouee {
        PartieJouee {
            horodatage,
            niveau,
            taille,
            resultat,
            duree_secondes: duree,
            erreurs: 1,
            indices: 2,
            score: if resultat == Resultat::Victoire { 1500 } else { 0 },
        }
    }

    // Une partie 9×9 facile, la plus courante dans les tests
    fn en_9x9(resultat: Resultat, duree: u64, horodatage: u64) -> PartieJouee {
        partie(Taille::Neuf, Niveau::Facile, resultat, duree, horodatage)
    }

    fn nettoyer(chemin: &PathBuf) {
        let _ = fs::remove_file(chemin);
        let _ = fs::remove_file(chemin.with_extension("tmp"));
    }

    #[test]
    fn test_aller_retour_d_une_ligne() {
        for resultat in [Resultat::Victoire, Resultat::Defaite, Resultat::Abandon] {
            for taille in Taille::TOUTES {
                let origine = partie(taille, Niveau::Difficile, resultat, 754, 1_700_000_000);
                assert_eq!(PartieJouee::depuis_ligne(&origine.vers_ligne()), Some(origine));
            }
        }
    }

    #[test]
    fn test_les_lignes_abimees_sont_ignorees() {
        let bonne = en_9x9(Resultat::Victoire, 100, 5).vers_ligne();
        assert!(PartieJouee::depuis_ligne("").is_none());
        assert!(PartieJouee::depuis_ligne("n'importe quoi").is_none());
        // Un champ en trop, un champ en moins
        assert!(PartieJouee::depuis_ligne(&format!("{}\t1", bonne)).is_none());
        assert!(PartieJouee::depuis_ligne(bonne.rsplit_once('\t').unwrap().0).is_none());
        // Résultat, niveau, taille ou nombre invalides
        assert!(PartieJouee::depuis_ligne(&bonne.replace("victoire", "match nul")).is_none());
        assert!(PartieJouee::depuis_ligne(&bonne.replace("Facile", "Expert")).is_none());
        assert!(PartieJouee::depuis_ligne(&bonne.replace("\t9\t", "\t7\t")).is_none());
        assert!(PartieJouee::depuis_ligne(&bonne.replace("\t100\t", "\tlong\t")).is_none());

        let chemin = chemin_temporaire();
        fs::write(&chemin, format!("abîmé\n{}\n\n{}\n", bonne, bonne.replace("victoire", "x"))).unwrap();
        let statistiques = Statistiques::depuis_fichier(chemin.clone());
        assert_eq!(statistiques.parties.len(), 1);
        nettoyer(&chemin);
    }

    #[test]
    fn test_fichier_absent_donne_des_statistiques_vides() {
        let statistiques = Statistiques::depuis_fichier(chemin_temporaire());
        let resume = statistiques.resume(Taille::Neuf, None);
        assert_eq!(resume, Resume::default());
        assert_eq!(resume.taux_de_victoire(), None);
    }

    #[test]
    fn test_les_parties_survivent_au_rechargement() {
        let chemin = chemin_temporaire();
        let mut statistiques = Statistiques::depuis_fichier(chemin.clone());
        statistiques.ajouter(en_9x9(Resultat::Victoire, 100, 1)).unwrap();
        statistiques.ajouter(en_9x9(Resultat::Defaite, 50, 2)).unwrap();

        let relues = Statistiques::depuis_fichier(chemin.clone());
        assert_eq!(relues.parties, statistiques.parties);
        assert_eq!(relues.parties.len(), 2);
        assert!(!chemin.with_extension("tmp").exists());
        nettoyer(&chemin);
    }

    #[test]
    fn test_le_resume_compte_les_resultats() {
        let chemin = chemin_temporaire();
        let mut statistiques = Statistiques::depuis_fichier(chemin.clone());
        for (resultat, duree) in [
            (Resultat::Victoire, 300),
            (Resultat::Victoire, 100),
            (Resultat::Defaite, 40),
            (Resultat::Abandon, 20),
        ] {
            let horodatage = statistiques.parties.len() as u64;
            statistiques.ajouter(en_9x9(resultat, duree, horodatage)).unwrap();
        }

        let resume = statistiques.resume(Taille::Neuf, None);
        assert_eq!(resume.jouees, 4);
        assert_eq!(resume.victoires, 2);
        assert_eq!(resume.defaites, 1);
        assert_eq!(resume.abandons, 1);
        // Seules les victoires comptent pour les temps
        assert_eq!(resume.meilleur_temps, Some(100));
        assert_eq!(resume.temps_moyen, Some(200));
        assert_eq!(resume.meilleur_score, Some(1500));
        assert_eq!(resume.taux_de_victoire(), Some(50));
        nettoyer(&chemin);
    }

    #[test]
    fn test_sans_victoire_pas_de_temps() {
        let mut statistiques = Statistiques::depuis_fichier(chemin_temporaire());
        statistiques.parties.push(en_9x9(Resultat::Defaite, 10, 1));
        let resume = statistiques.resume(Taille::Neuf, None);
        assert_eq!(resume.meilleur_temps, None);
        assert_eq!(resume.temps_moyen, None);
        assert_eq!(resume.meilleur_score, None);
        assert_eq!(resume.taux_de_victoire(), Some(0));
    }

    #[test]
    fn test_le_taux_de_victoire_est_arrondi() {
        let mut statistiques = Statistiques::depuis_fichier(chemin_temporaire());
        statistiques.parties.push(en_9x9(Resultat::Victoire, 10, 1));
        statistiques.parties.push(en_9x9(Resultat::Defaite, 10, 2));
        statistiques.parties.push(en_9x9(Resultat::Defaite, 10, 3));
        // 1 sur 3 : 33 %
        assert_eq!(statistiques.resume(Taille::Neuf, None).taux_de_victoire(), Some(33));
        statistiques.parties.push(en_9x9(Resultat::Victoire, 10, 4));
        statistiques.parties.push(en_9x9(Resultat::Victoire, 10, 5));
        statistiques.parties.push(en_9x9(Resultat::Defaite, 10, 6));
        statistiques.parties.push(en_9x9(Resultat::Victoire, 10, 7));
        // 4 sur 7 : 57,1 %
        assert_eq!(statistiques.resume(Taille::Neuf, None).taux_de_victoire(), Some(57));
    }

    #[test]
    fn test_les_series_de_victoires() {
        let mut statistiques = Statistiques::depuis_fichier(chemin_temporaire());
        use Resultat::{Abandon, Defaite, Victoire};
        // V V V D V V A V : la meilleure série est de 3, la série en cours de 1
        for (i, resultat) in [Victoire, Victoire, Victoire, Defaite, Victoire, Victoire, Abandon, Victoire]
            .into_iter()
            .enumerate()
        {
            statistiques.parties.push(en_9x9(resultat, 10, i as u64));
        }
        let resume = statistiques.resume(Taille::Neuf, None);
        assert_eq!(resume.meilleure_serie, 3);
        assert_eq!(resume.serie_en_cours, 1);
    }

    #[test]
    fn test_les_tailles_et_les_niveaux_ne_se_melangent_pas() {
        let mut statistiques = Statistiques::depuis_fichier(chemin_temporaire());
        statistiques.parties.push(partie(Taille::Neuf, Niveau::Facile, Resultat::Victoire, 100, 1));
        statistiques.parties.push(partie(Taille::Neuf, Niveau::Difficile, Resultat::Defaite, 200, 2));
        statistiques.parties.push(partie(Taille::Seize, Niveau::Facile, Resultat::Victoire, 900, 3));

        let neuf = statistiques.resume(Taille::Neuf, None);
        assert_eq!(neuf.jouees, 2);
        assert_eq!(neuf.meilleur_temps, Some(100));

        let neuf_difficile = statistiques.resume(Taille::Neuf, Some(Niveau::Difficile));
        assert_eq!(neuf_difficile.jouees, 1);
        assert_eq!(neuf_difficile.victoires, 0);

        let seize = statistiques.resume(Taille::Seize, None);
        assert_eq!(seize.jouees, 1);
        assert_eq!(seize.meilleur_temps, Some(900));

        assert_eq!(statistiques.resume(Taille::Six, None).jouees, 0);
    }

    #[test]
    fn test_l_ordre_vient_de_l_horodatage() {
        let chemin = chemin_temporaire();
        // Écrites dans le désordre : la série suit l'ordre des dates (V puis D, pas D puis V)
        let lignes = [
            en_9x9(Resultat::Defaite, 10, 20).vers_ligne(),
            en_9x9(Resultat::Victoire, 10, 10).vers_ligne(),
        ];
        fs::write(&chemin, lignes.join("\n")).unwrap();
        let statistiques = Statistiques::depuis_fichier(chemin.clone());
        assert_eq!(statistiques.resume(Taille::Neuf, None).serie_en_cours, 0);
        nettoyer(&chemin);
    }

    #[test]
    fn test_echec_d_ecriture_n_ajoute_rien() {
        // Le « dossier » du fichier est en fait un fichier : impossible d'écrire dedans
        let fichier = chemin_temporaire();
        fs::write(&fichier, "je suis un fichier").unwrap();
        let mut statistiques = Statistiques::depuis_fichier(fichier.join("statistiques.txt"));

        assert!(statistiques.ajouter(en_9x9(Resultat::Victoire, 10, 1)).is_err());
        assert_eq!(statistiques.resume(Taille::Neuf, None).jouees, 0);
        nettoyer(&fichier);
    }
}
