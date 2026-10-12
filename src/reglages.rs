//! Les réglages du joueur (thème, couleurs, sons, mise en valeur), enregistrés dans un fichier.

use crate::sauvegarde::{dossier_donnees, ecrire_fichier};
use std::fs;
use std::path::{Path, PathBuf};

// Le thème de la fenêtre et de la grille
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Theme {
    // Celui du système (Windows clair ou sombre)
    Systeme,
    Clair,
    Sombre,
}

impl Theme {
    pub const TOUS: [Theme; 3] = [Theme::Systeme, Theme::Clair, Theme::Sombre];

    pub fn libelle(self) -> &'static str {
        match self {
            Theme::Systeme => "Celui du système",
            Theme::Clair => "Clair",
            Theme::Sombre => "Sombre",
        }
    }

    // Le mot écrit dans le fichier
    fn mot(self) -> &'static str {
        match self {
            Theme::Systeme => "systeme",
            Theme::Clair => "clair",
            Theme::Sombre => "sombre",
        }
    }

    fn depuis_mot(mot: &str) -> Option<Theme> {
        Theme::TOUS.into_iter().find(|theme| theme.mot() == mot)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Reglages {
    pub theme: Theme,
    // Des couleurs qui se distinguent même sans voir le rouge et le vert, et un soulignement
    // sous les chiffres faux
    pub daltonien: bool,
    pub sons: bool,
    // Mettre en valeur la ligne, la colonne et le bloc de la case choisie
    pub surligner_zone: bool,
    // Mettre en valeur les cases qui ont le même chiffre que la case choisie
    pub surligner_chiffres: bool,
}

impl Default for Reglages {
    fn default() -> Reglages {
        Reglages {
            theme: Theme::Systeme,
            daltonien: false,
            sons: false,
            surligner_zone: true,
            surligner_chiffres: true,
        }
    }
}

fn ecrire_booleen(valeur: bool) -> &'static str {
    if valeur { "oui" } else { "non" }
}

fn lire_booleen(mot: &str) -> Option<bool> {
    match mot {
        "oui" => Some(true),
        "non" => Some(false),
        _ => None,
    }
}

impl Reglages {
    // Une ligne par réglage : le nom, une tabulation, la valeur
    fn vers_texte(&self) -> String {
        format!(
            "theme\t{}\ndaltonien\t{}\nsons\t{}\nsurligner_zone\t{}\nsurligner_chiffres\t{}\n",
            self.theme.mot(),
            ecrire_booleen(self.daltonien),
            ecrire_booleen(self.sons),
            ecrire_booleen(self.surligner_zone),
            ecrire_booleen(self.surligner_chiffres),
        )
    }

    // Relit les réglages. Ce qui manque ou est abîmé garde sa valeur par défaut : un fichier
    // écrit par une autre version, ou modifié à la main, ne bloque jamais le jeu.
    fn depuis_texte(texte: &str) -> Reglages {
        let mut reglages = Reglages::default();
        for ligne in texte.lines() {
            let Some((cle, valeur)) = ligne.split_once('\t') else {
                continue;
            };
            match cle {
                "theme" => reglages.theme = Theme::depuis_mot(valeur).unwrap_or(reglages.theme),
                "daltonien" => reglages.daltonien = lire_booleen(valeur).unwrap_or(reglages.daltonien),
                "sons" => reglages.sons = lire_booleen(valeur).unwrap_or(reglages.sons),
                "surligner_zone" => {
                    reglages.surligner_zone = lire_booleen(valeur).unwrap_or(reglages.surligner_zone)
                }
                "surligner_chiffres" => {
                    reglages.surligner_chiffres =
                        lire_booleen(valeur).unwrap_or(reglages.surligner_chiffres)
                }
                _ => {}
            }
        }
        reglages
    }

    // Charge les réglages enregistrés. Sans fichier, ce sont ceux par défaut.
    pub fn charger() -> Reglages {
        Reglages::depuis_fichier(&chemin_par_defaut())
    }

    pub fn enregistrer(&self) -> Result<(), String> {
        self.vers_fichier(&chemin_par_defaut())
    }

    fn depuis_fichier(chemin: &Path) -> Reglages {
        fs::read_to_string(chemin)
            .map(|texte| Reglages::depuis_texte(&texte))
            .unwrap_or_default()
    }

    fn vers_fichier(&self, chemin: &Path) -> Result<(), String> {
        ecrire_fichier(chemin, &self.vers_texte())
            .map_err(|erreur| format!("Impossible d'enregistrer les réglages : {}", erreur))
    }
}

// Où les réglages sont enregistrés : dans le dossier de données de l'utilisateur
fn chemin_par_defaut() -> PathBuf {
    match dossier_donnees() {
        Some(dossier) => dossier.join("reglages.txt"),
        None => PathBuf::from("sudoko_reglages.txt"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classement::chemin_temporaire;

    fn nettoyer(chemin: &PathBuf) {
        let _ = fs::remove_file(chemin);
        let _ = fs::remove_file(chemin.with_extension("tmp"));
    }

    fn reglages_changes() -> Reglages {
        Reglages {
            theme: Theme::Sombre,
            daltonien: true,
            sons: true,
            surligner_zone: false,
            surligner_chiffres: false,
        }
    }

    #[test]
    fn test_les_reglages_par_defaut() {
        let reglages = Reglages::default();
        assert_eq!(reglages.theme, Theme::Systeme);
        assert!(!reglages.daltonien);
        assert!(!reglages.sons);
        assert!(reglages.surligner_zone);
        assert!(reglages.surligner_chiffres);
    }

    #[test]
    fn test_aller_retour_en_texte() {
        for theme in Theme::TOUS {
            let reglages = Reglages { theme, ..reglages_changes() };
            assert_eq!(Reglages::depuis_texte(&reglages.vers_texte()), reglages);
        }
        let defaut = Reglages::default();
        assert_eq!(Reglages::depuis_texte(&defaut.vers_texte()), defaut);
    }

    #[test]
    fn test_ce_qui_manque_ou_est_abime_garde_la_valeur_par_defaut() {
        // Un seul réglage connu, une valeur invalide, une ligne sans tabulation, une clé inconnue
        let texte = "theme\tsombre\ndaltonien\tpeut-etre\nn'importe quoi\nvolume\t11\nsons\toui\n";
        let reglages = Reglages::depuis_texte(texte);
        assert_eq!(reglages.theme, Theme::Sombre);
        assert!(!reglages.daltonien);
        assert!(reglages.sons);
        assert!(reglages.surligner_zone);
        assert_eq!(Reglages::depuis_texte(""), Reglages::default());
    }

    #[test]
    fn test_les_reglages_survivent_au_rechargement() {
        let chemin = chemin_temporaire();
        assert_eq!(Reglages::depuis_fichier(&chemin), Reglages::default());

        reglages_changes().vers_fichier(&chemin).unwrap();
        assert_eq!(Reglages::depuis_fichier(&chemin), reglages_changes());
        assert!(!chemin.with_extension("tmp").exists());
        nettoyer(&chemin);
    }

    #[test]
    fn test_un_fichier_illisible_donne_les_reglages_par_defaut() {
        let chemin = chemin_temporaire();
        fs::write(&chemin, [0xff, 0xfe, 0x00]).unwrap();
        assert_eq!(Reglages::depuis_fichier(&chemin), Reglages::default());
        nettoyer(&chemin);
    }

    #[test]
    fn test_echec_d_ecriture() {
        // Le « dossier » du fichier est en fait un fichier : impossible d'écrire dedans
        let fichier = chemin_temporaire();
        fs::write(&fichier, "je suis un fichier").unwrap();
        let resultat = reglages_changes().vers_fichier(&fichier.join("reglages.txt"));
        assert!(resultat.unwrap_err().contains("Impossible d'enregistrer les réglages"));
        nettoyer(&fichier);
    }

    #[test]
    fn test_le_theme_se_retrouve_par_son_mot() {
        for theme in Theme::TOUS {
            assert_eq!(Theme::depuis_mot(theme.mot()), Some(theme));
        }
        assert_eq!(Theme::depuis_mot("violet"), None);
    }
}
