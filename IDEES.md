# Idées pour la suite

Liste des fonctionnalités envisagées pour Sudoko, avec ce que chacune apporte et une estimation de l'effort.

| # | Idée | Ce que ça apporte | Effort | État |
| - | ---- | ----------------- | ------ | ---- |
| 1 | **Sauvegarde de la partie** | On peut fermer le jeu et reprendre plus tard. C'est surtout utile en 12×12 et 16×16, qui prennent longtemps. | Moyen | Fait |
| 2 | **Annuler et refaire** | On revient en arrière après une erreur, avec `Ctrl+Z`. | Faible | Fait |
| 3 | **Notes au crayon** | On inscrit les chiffres possibles dans une case. C'est presque indispensable en 16×16. | Moyen | Fait |
| 4 | **Statistiques** | Parties jouées, taux de victoire, meilleur temps par taille et niveau. | Faible | Fait |
| 5 | **« Résoudre ma grille »** | On saisit une grille de journal dans la fenêtre et le solveur la résout. Le chargement et le solveur existent déjà. | Faible à moyen | Fait |
| 6 | **Compilation automatique sur GitHub** | Un exécutable et les tests générés à chaque version, sans attendre 8 minutes en local. | Faible | Fait |
| 7 | **Vraie difficulté** | La difficulté serait mesurée par les techniques nécessaires (case forcée, etc.) plutôt que par le nombre de cases vides. Un indice pourrait alors expliquer la technique. | Élevé | Fait |
| 8 | **Réglages** | Thème sombre, mode pour daltoniens, sons. | Faible à moyen | Fait |
| 9 | **Jouer dans le navigateur** | `egui` peut tourner dans une page web, avec un lien à partager. Il faudrait changer la sauvegarde et la génération en arrière-plan. | Élevé | À faire |
| 10 | **Autres variantes** | Sudoku diagonal ou à régions irrégulières. Le solveur serait à adapter. | Moyen à élevé | À faire |

## Ordre conseillé

1. Sauvegarde de la partie, puis annuler/refaire : ce sont les deux manques les plus gênants quand on joue vraiment.
2. Notes au crayon, qui rendent les grandes grilles agréables.
3. Compilation automatique sur GitHub.
