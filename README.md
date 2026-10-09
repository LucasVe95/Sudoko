# Sudoko

Un logiciel de Sudoku écrit en Rust, développé pour le fun et pour apprendre le langage.

> **État du projet :** en cours de développement. Le jeu est jouable dans une fenêtre ou au terminal, avec des grilles générées au hasard, des indices et une résolution automatique.

## Fonctionnalités

Déjà présent :

- Structure `Grille` représentant une grille de 9×9 cases
- Affichage de la grille dans le terminal, avec les blocs 3×3 séparés
- Vérification des règles (ligne, colonne, bloc 3×3) à chaque coup
- Chargement d'une grille de départ depuis un texte de 81 chiffres (0 = case vide)
- Cases de départ verrouillées, effacement des coups du joueur
- Boucle de jeu au clavier avec détection de la victoire
- Résolution automatique par retour sur trace (*backtracking*)
- Génération de grilles aléatoires à solution unique, en trois niveaux de difficulté
- Indices : le programme révèle une case au hasard avec la bonne valeur
- Interface graphique (avec [egui](https://github.com/emilk/egui)) : clic ou flèches pour choisir une case, clavier ou boutons pour les chiffres, nouvelle partie en trois niveaux

## Comment jouer

### Dans la fenêtre (par défaut)

- Cliquez sur une case, ou déplacez la sélection avec les flèches.
- Tapez un chiffre de 1 à 9 pour le placer. `0`, `Retour arrière` ou `Suppr` efface la case.
- Les boutons `1` à `9` et `Effacer` font la même chose à la souris.
- `Indice` révèle une case au hasard, `Solution` affiche la grille complète.
- `Facile`, `Moyen` et `Difficile` lancent une nouvelle partie.

Les chiffres de départ sont en noir et ceux du joueur en bleu. Les cases qui contiennent le même chiffre que la case choisie sont mises en valeur.

### Au terminal

Lancez le programme avec `terminal` en premier argument (voir plus bas). Il demande un coup sous la forme `ligne colonne valeur`, avec des numéros de 1 à 9.

| Saisie  | Effet                                        |
| ------- | -------------------------------------------- |
| `1 3 4` | Place le 4 en ligne 1, colonne 3             |
| `1 3 0` | Efface la case en ligne 1, colonne 3         |
| `h`     | Révèle une case au hasard (indice)           |
| `s`     | Affiche la solution et termine la partie     |
| `q`     | Quitte la partie                             |

Un coup est refusé s'il brise une règle du sudoku ou s'il vise une case de la grille de départ.

## Prérequis

- [Rust](https://www.rust-lang.org/tools/install) (édition 2024, donc une version récente de la chaîne d'outils stable)
- Pour la fenêtre : une carte graphique compatible OpenGL (le cas de presque tous les ordinateurs récents)

## Installation et lancement

```bash
git clone <url-du-depot>
cd Sudoko
cargo run
```

La première compilation télécharge et compile l'interface graphique : elle prend quelques minutes et beaucoup de mémoire. Si elle échoue par manque de mémoire, limitez le parallélisme avec `cargo build -j 2`. Les compilations suivantes sont rapides.

Sans argument, le programme ouvre la fenêtre avec une grille de difficulté moyenne. On peut choisir la grille :

```bash
cargo run -- facile      # 35 cases vides
cargo run -- moyen       # 45 cases vides (par défaut)
cargo run -- difficile   # jusqu'à 55 cases vides
cargo run -- fixe        # toujours la même grille de départ
```

Pour jouer au terminal au lieu de la fenêtre, ajoutez `terminal` en premier :

```bash
cargo run -- terminal            # grille moyenne
cargo run -- terminal facile     # ou facile, difficile, fixe
```

En mode `difficile`, la génération peut prendre quelques secondes au démarrage. Elle est bien plus rapide avec `cargo run --release -- difficile`.

## Tests

```bash
cargo test
```

## Structure du projet

```text
Sudoko/
├── Cargo.toml    # Configuration du projet et dépendances
├── src/
│   ├── main.rs   # Point d'entrée : choix de la grille et de l'interface selon les arguments
│   ├── gui.rs    # Interface graphique (egui) : dessin de la grille, souris, clavier, boutons
│   ├── jeu.rs    # Boucle de jeu au terminal et lecture des commandes
│   └── grille.rs # Structure Grille : règles, chargement, résolution, génération, indices
├── LICENSE
└── README.md
```

## Licence

Ce projet est distribué sous licence MIT. Voir le fichier [LICENSE](LICENSE).
