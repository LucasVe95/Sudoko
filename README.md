# Sudoko

Un logiciel de Sudoku écrit en Rust, développé pour le fun et pour apprendre le langage.

> **État du projet :** en cours de développement. Le jeu est jouable au terminal sur une grille de départ fixe, avec résolution automatique. Il n'y a pas encore de génération de grilles.

## Fonctionnalités

Déjà présent :

- Structure `Grille` représentant une grille de 9×9 cases
- Affichage de la grille dans le terminal, avec les blocs 3×3 séparés
- Vérification des règles (ligne, colonne, bloc 3×3) à chaque coup
- Chargement d'une grille de départ depuis un texte de 81 chiffres (0 = case vide)
- Cases de départ verrouillées, effacement des coups du joueur
- Boucle de jeu au clavier avec détection de la victoire
- Résolution automatique par retour sur trace (*backtracking*)

Prévu :

- Génération de nouvelles grilles
- Choix de la grille de départ par l'utilisateur

## Comment jouer

Le programme demande un coup sous la forme `ligne colonne valeur`, avec des numéros de 1 à 9.

| Saisie  | Effet                                        |
| ------- | -------------------------------------------- |
| `1 3 4` | Place le 4 en ligne 1, colonne 3             |
| `1 3 0` | Efface la case en ligne 1, colonne 3         |
| `s`     | Affiche la solution et termine la partie     |
| `q`     | Quitte la partie                             |

Un coup est refusé s'il brise une règle du sudoku ou s'il vise une case de la grille de départ.

## Prérequis

- [Rust](https://www.rust-lang.org/tools/install) (édition 2024, donc une version récente de la chaîne d'outils stable)

## Installation et lancement

```bash
git clone <url-du-depot>
cd Sudoko
cargo run
```

## Tests

```bash
cargo test
```

## Structure du projet

```text
Sudoko/
├── Cargo.toml    # Configuration du projet
├── src/
│   └── main.rs   # Point d'entrée et structure Grille
├── LICENSE
└── README.md
```

## Licence

Ce projet est distribué sous licence MIT. Voir le fichier [LICENSE](LICENSE).
