# Sudoko

Un logiciel de Sudoku écrit en Rust, développé pour le fun et pour apprendre le langage.

> **État du projet :** en cours de développement. Le jeu est jouable dans une fenêtre ou au terminal, avec des grilles de 4×4 à 16×16 générées au hasard, des indices et une résolution automatique.

## Fonctionnalités

Déjà présent :

- Grilles de **cinq tailles** : 4×4, 6×6, 9×9, 12×12 et 16×16 (les nombres vont de 1 jusqu'au côté de la grille)
- Vérification des règles (ligne, colonne, bloc) à chaque coup, avec des blocs rectangulaires en 6×6 (2×3) et en 12×12 (3×4)
- Chargement d'une grille de départ 9×9 depuis un texte de 81 chiffres (0 = case vide)
- Cases de départ verrouillées, effacement des coups du joueur
- Résolution automatique par retour sur trace (*backtracking*), avec un solveur qui traite en premier la case la plus contrainte
- Génération de grilles aléatoires à solution unique, en trois niveaux de difficulté
- Indices : le programme révèle une case au hasard avec la bonne valeur
- Interface graphique (avec [egui](https://github.com/emilk/egui)) : clic ou flèches pour choisir une case, clavier ou boutons pour les nombres
- Menu : choix de la taille et du niveau, grille d'entraînement, reprise de la partie en cours
- Défaite après 3 erreurs, chiffres faux affichés en rouge, chronomètre
- Écran de fin (victoire, défaite, abandon) avec le temps, les erreurs et les indices
- Score à chaque victoire et classement des meilleurs scores par taille, conservé d'une session à l'autre
- Jeu au terminal, pour toutes les tailles

## Comment jouer

### Dans la fenêtre (par défaut)

Le programme s'ouvre sur le **menu** : choisissez une taille de grille (4×4 à 16×16) puis un niveau (`Facile`, `Moyen`, `Difficile`), ou lancez la grille d'entraînement, ou reprenez la partie en cours.

Le nombre de cases à trouver dépend du niveau et de la taille. Ces chiffres sont des maximums : la génération garde une case quand elle ne peut pas prouver rapidement que la solution reste unique sans elle.

| Taille | Facile | Moyen | Difficile |
| ------ | ------ | ----- | --------- |
| 4×4    | 7      | 9     | 11        |
| 6×6    | 15     | 20    | 24        |
| 9×9    | 35     | 45    | 55        |
| 12×12  | 62     | 81    | 98        |
| 16×16  | 110    | 143   | 174       |

Pendant la partie :

- Cliquez sur une case, ou déplacez la sélection avec les flèches.
- Tapez un nombre pour le placer. `0`, `Retour arrière` ou `Suppr` efface la case.
- Au-delà de 9, un nombre s'écrit avec deux touches : tapez `1` puis `2` pour obtenir 12. Le `1` attend un instant le second chiffre, puis se place seul si rien ne suit. Quand les deux chiffres ne forment pas un nombre de la grille (17 en 16×16), le dernier compte seul.
- Les boutons `1` jusqu'au côté de la grille et `Effacer` font la même chose à la souris.
- `Indice` révèle une case au hasard, `Solution` affiche la grille complète et termine la partie.
- `Menu` met la partie en pause (le chronomètre s'arrête) et revient au menu.

Les chiffres de départ sont en noir et ceux du joueur en bleu. Les cases qui contiennent le même chiffre que la case choisie sont mises en valeur.

**Erreurs et défaite.** Un chiffre qui n'est pas celui de la solution compte comme une erreur, qu'il brise une règle du sudoku (le coup est alors refusé) ou non (le chiffre est placé, mais s'affiche en rouge). Au bout de 3 erreurs, la partie est perdue.

**Fin de partie.** Un écran de fin indique si la partie est gagnée, perdue ou abandonnée, avec le temps, les erreurs et les indices utilisés. Il propose de rejouer, d'ouvrir le classement, de retourner au menu et, après une défaite, de voir la solution.

**Score.** Seule une victoire sur une grille générée donne un score (la grille d'entraînement n'en donne pas) :

```text
score = points de départ − 1 par seconde − pénalité par erreur − pénalité par indice   (jamais en dessous de 0)
```

Une grande grille prend plus de temps : les points de départ et les pénalités sont multipliés selon la taille. Le temps coûte toujours 1 point par seconde.

| Taille | Facile | Moyen | Difficile | Pénalité par erreur | Pénalité par indice |
| ------ | ------ | ----- | --------- | ------------------- | ------------------- |
| 4×4    | 250    | 500   | 750       | 25                  | 37                  |
| 6×6    | 500    | 1000  | 1500      | 50                  | 75                  |
| 9×9    | 1000   | 2000  | 3000      | 100                 | 150                 |
| 12×12  | 2500   | 5000  | 7500      | 250                 | 375                 |
| 16×16  | 5000   | 10000 | 15000     | 500                 | 750                 |

**Classement.** À la victoire, tapez votre nom (20 caractères au plus) puis `Enregistrer` ou `Entrée`. Le nom du dernier joueur est proposé d'office. Le menu donne accès au `Classement` : les 10 meilleurs scores d'une taille de grille, tous niveaux confondus ou par niveau. Chaque taille a son propre classement, car les scores de tailles différentes ne se comparent pas. À score égal, la partie la plus rapide passe devant.

Le classement est enregistré dans un fichier texte, en dehors du dépôt, et il est donc conservé d'une session à l'autre :

- Windows : `%APPDATA%\Sudoko\scores.txt`
- Linux et macOS : `~/.local/share/Sudoko/scores.txt`

Supprimer ce fichier remet le classement à zéro. Le terminal ne gère ni score ni classement.

### Au terminal

Lancez le programme avec `terminal` en premier argument (voir plus bas). Il demande un coup sous la forme `ligne colonne valeur`, avec des numéros de 1 jusqu'au côté de la grille (de 1 à 16 en 16×16).

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

Sans argument, le programme ouvre la fenêtre sur le menu. Pour lancer directement une partie, donnez le niveau, puis éventuellement la taille de la grille (4, 6, 9, 12 ou 16 ; 9 par défaut) :

```bash
cargo run -- facile          # grille 9×9 facile
cargo run -- difficile 16    # grille 16×16 difficile
cargo run -- moyen 6         # grille 6×6 moyenne
cargo run -- fixe            # toujours la même grille d'entraînement (9×9 seulement)
```

Pour jouer au terminal au lieu de la fenêtre, ajoutez `terminal` en premier. Le terminal n'a ni menu, ni limite d'erreurs, ni chronomètre, ni score :

```bash
cargo run -- terminal              # grille 9×9 moyenne
cargo run -- terminal facile 12    # ou difficile, fixe ; taille 4, 6, 9, 12 ou 16
```

La génération d'une grille prend en général moins d'une seconde, et jusqu'à quelques secondes pour une grande grille difficile avec `cargo run`. Elle est bien plus rapide avec `cargo run --release`.

## Exécutable

Pour obtenir un programme que l'on peut lancer sans Rust ni `cargo` :

```bash
cargo build --release
```

Le résultat est le fichier `target/release/Sudoko.exe` (sous Windows), d'environ 7 Mo. C'est un seul fichier : on peut le copier où l'on veut et le lancer d'un double-clic. Il accepte les mêmes arguments que `cargo run --` (`Sudoko.exe facile`, `Sudoko.exe terminal`...).

Sous Windows, il faut les composants Visual C++ (`VCRUNTIME140.dll`, présents sur la plupart des ordinateurs) et une carte graphique compatible OpenGL. Le dossier `target/` n'est pas versionné : pour partager l'exécutable, déposez-le par exemple dans une *release* GitHub.

La compilation en mode `release` est longue (7 minutes environ la première fois) et consomme beaucoup de mémoire : `cargo build --release -j 2` limite la charge.

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
│   ├── gui.rs    # Interface graphique (egui) : menu, partie, erreurs, chrono, écran de fin, classement
│   ├── classement.rs # Calcul du score et classement enregistré dans un fichier
│   ├── jeu.rs    # Boucle de jeu au terminal et lecture des commandes
│   ├── grille.rs # Structure Grille et ses tailles : règles, chargement, génération, indices
│   └── solveur.rs # Solveur par retour sur trace (candidats en bits, case la plus contrainte)
├── LICENSE
└── README.md
```

## Licence

Ce projet est distribué sous licence MIT. Voir le fichier [LICENSE](LICENSE).
