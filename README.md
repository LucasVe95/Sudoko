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
- Génération de grilles aléatoires à solution unique, en trois niveaux fondés sur les techniques de déduction qu'il faut pour les résoudre
- Indices : le programme révèle une case et explique la technique de déduction qui la trouve
- Interface graphique (avec [egui](https://github.com/emilk/egui)) : clic ou flèches pour choisir une case, clavier ou boutons pour les nombres
- Menu : choix de la taille, de la variante et du niveau, grille d'entraînement, reprise de la partie en cours
- Défaite après 3 erreurs, chiffres faux affichés en rouge, chronomètre
- Écran de fin (victoire, défaite, abandon) avec le temps, les erreurs et les indices
- Score à chaque victoire et classement des meilleurs scores par taille et par variante, conservé d'une session à l'autre
- Statistiques par taille, par variante et par niveau : victoires, meilleur temps, séries
- **Variantes** : diagonale, hyper et anti-cavalier, en plus du Sudoku classique
- Réglages : thème clair ou sombre, mode daltonien, sons, mise en valeur des cases
- Résolution d'une grille de journal saisie ou collée, avec détection des erreurs de saisie et des grilles à plusieurs solutions
- Notes au crayon dans les cases vides, avec un remplissage automatique des chiffres permis
- Annuler et refaire les coups (`Ctrl+Z`, `Ctrl+Y`)
- Sauvegarde automatique de la partie en cours : on ferme le jeu et on reprend plus tard
- Jeu au terminal, pour toutes les tailles

## Comment jouer

### Dans la fenêtre (par défaut)

Le programme s'ouvre sur le **menu** : choisissez une taille de grille (4×4 à 16×16), une variante (`Classique`, `Diagonale`, `Hyper`, `Anti-cavalier`) puis un niveau (`Facile`, `Moyen`, `Difficile`), ou lancez la grille d'entraînement, ou reprenez la partie en cours.

**La difficulté se mesure aux techniques.** Une grille n'est pas plus difficile parce qu'elle a plus de cases vides, mais parce qu'il faut des déductions plus subtiles pour la finir. Le programme résout chaque grille générée comme un humain, sans jamais deviner, en essayant les techniques de la plus simple à la plus difficile, et il mesure la plus difficile dont il a eu besoin :

| Niveau    | Techniques nécessaires |
| --------- | ---------------------- |
| Facile    | **Case unique** (une case n'a plus qu'un chiffre possible) et **chiffre unique** (un chiffre n'a plus qu'une case possible dans une ligne, une colonne ou un bloc) |
| Moyen     | en plus, une **intersection** (un chiffre est limité à une ligne ou une colonne dans un bloc, ou l'inverse : il est impossible ailleurs sur cette ligne ou dans ce bloc) ou une **paire nue** (deux cases d'une zone n'ont que les deux mêmes chiffres) |
| Difficile | en plus, une **paire cachée** (deux chiffres n'ont que les deux mêmes cases dans une zone), un **triplet nu** (trois cases, trois chiffres) ou un **X-Wing** (un chiffre limité aux deux mêmes colonnes dans deux lignes, ou l'inverse) |

Une grille se finit donc toujours par déduction, sans essayer de chiffres au hasard : et comme chaque déduction est sûre, sa solution est unique. Un niveau ne demande jamais de technique plus difficile que la sienne.

Le nombre de cases vides sert de repère. Il est de 43 %, 56 % et 68 % des cases pour Facile, Moyen et Difficile, mais seul Facile s'arrête exactement là : pour Moyen et Difficile, c'est un minimum, car on retire des cases tant que la grille ne demande pas les techniques du niveau (en 9×9, une grille Moyen a en général une cinquantaine de cases vides).

| Taille | Facile | Moyen (au moins) | Difficile (au moins) |
| ------ | ------ | ---------------- | -------------------- |
| 4×4    | 7      | 9                | 11                   |
| 6×6    | 15     | 20               | 24                   |
| 9×9    | 35     | 45               | 55                   |
| 12×12  | 62     | 81               | 98                   |
| 16×16  | 110    | 143              | 174                  |

En 4×4 et 6×6, les grilles sont trop petites pour que les techniques difficiles servent : le niveau n'y dépend que du nombre de cases vides, avec le chiffre exact du tableau. Dans les autres tailles, si aucune grille demandant les techniques du niveau n'est trouvée au bout de quelques secondes, le programme garde la plus difficile qu'il a trouvée (cela arrive rarement, surtout en `Difficile`).

Pendant la partie :

- Cliquez sur une case, ou déplacez la sélection avec les flèches.
- Tapez un nombre pour le placer. `0`, `Retour arrière` ou `Suppr` efface la case.
- Au-delà de 9, un nombre s'écrit avec deux touches : tapez `1` puis `2` pour obtenir 12. Le `1` attend un instant le second chiffre, puis se place seul si rien ne suit. Quand les deux chiffres ne forment pas un nombre de la grille (17 en 16×16), le dernier compte seul.
- Les boutons `1` jusqu'au côté de la grille et `Effacer` font la même chose à la souris.
- `Indice` remplit une case et explique pourquoi : on choisit la case que la technique la plus simple permet de trouver (« Case unique : en ligne 3, colonne 4, un seul chiffre reste possible, le 7 »). Quand il faut d'abord retirer des possibilités, l'indice nomme les techniques à employer. Si aucune déduction ne mène à une case, il le dit et donne le chiffre de la solution. `Solution` affiche la grille complète et termine la partie.
- `Annuler` (ou `Ctrl+Z`) défait le dernier changement de case, `Refaire` (ou `Ctrl+Y`, `Ctrl+Maj+Z`) le remet. Un nouveau coup efface ce qu'on pouvait refaire. Annuler ne rend ni les erreurs ni les indices déjà comptés, et l'historique n'est pas conservé quand on ferme le jeu.
- **Notes au crayon.** Le bouton `Notes` (ou la touche `N`) bascule en mode notes : un nombre écrit alors une petite note dans la case choisie au lieu de jouer un chiffre, et le même nombre une seconde fois la retire. Les notes sont rangées en tableau dans la case, toujours à la même place. `0`, `Suppr` ou `Effacer` vident les notes d'une case vide (sur une case remplie, ils effacent le chiffre, même en mode notes). Les notes sont de simples brouillons : elles ne comptent aucune erreur et peuvent être fausses. `Remplir les notes` écrit dans chaque case vide tous les chiffres que les règles y permettent (sans révéler la solution, donc sans pénalité) et remplace les notes déjà écrites ; on peut le relancer quand la grille a avancé. Poser un chiffre efface les notes de sa case, et annuler ce coup les remet. Les notes sont conservées par la sauvegarde.
- `Menu` met la partie en pause (le chronomètre s'arrête) et revient au menu.
- Fermer la fenêtre en pleine partie ne la perd pas : voir *Sauvegarde* ci-dessous.

Les chiffres de départ sont en noir et ceux du joueur en bleu. Les cases qui contiennent le même chiffre que la case choisie sont mises en valeur.

**Erreurs et défaite.** Vous avez le droit de vous tromper : un chiffre est toujours posé, même s'il est faux, y compris quand il enfreint une règle du sudoku (déjà présent dans la ligne, la colonne ou le bloc). S'il n'est pas celui de la solution, il s'affiche **en rouge** et compte comme une erreur, et le message vous le dit. Vous pouvez l'effacer (`0`, `Suppr`, `Effacer`) ou annuler le coup (`Ctrl+Z`), puis y mettre le bon chiffre ; effacer ne rend pas l'erreur. Retaper le même chiffre faux dans la même case ne compte pas une seconde erreur. Le bon chiffre se pose aussi quand un chiffre rouge de la même zone le contredit : seul le chiffre faux est en rouge. Au bout de 3 erreurs, la partie est perdue. Une grille pleine n'est une victoire que si elle respecte toutes les règles.

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

**Sauvegarde.** La partie en cours est enregistrée à chaque coup, quand on retourne au menu et quand on ferme la fenêtre, avec son chronomètre. À la prochaine ouverture, le menu propose `Reprendre la partie` (avec le nom de la grille et le temps déjà joué). La sauvegarde disparaît quand la partie se termine (victoire, défaite ou abandon). Lancer une nouvelle partie remplace la sauvegarde précédente, y compris avec `cargo run -- facile`. Un fichier abîmé est ignoré. Le terminal ne gère pas la sauvegarde.

- Windows : `%APPDATA%\Sudoko\partie.txt`
- Linux et macOS : `~/.local/share/Sudoko/partie.txt`

**Classement.** À la victoire, tapez votre nom (20 caractères au plus) puis `Enregistrer` ou `Entrée`. Le nom du dernier joueur est proposé d'office. Le menu donne accès au `Classement` : les 10 meilleurs scores d'une taille de grille, tous niveaux confondus ou par niveau. Chaque taille et chaque variante a son propre classement, car les scores de tailles ou de variantes différentes ne se comparent pas. À score égal, la partie la plus rapide passe devant.

Le classement est enregistré dans un fichier texte, en dehors du dépôt, et il est donc conservé d'une session à l'autre :

- Windows : `%APPDATA%\Sudoko\scores.txt`
- Linux et macOS : `~/.local/share/Sudoko/scores.txt`

Supprimer ce fichier remet le classement à zéro. Le terminal ne gère ni score ni classement.

**Statistiques.** Le menu donne aussi accès aux `Statistiques`. Chaque partie terminée sur une grille générée (victoire, défaite ou abandon) y est comptée, une seule fois ; la grille d'entraînement et les parties laissées en cours ne le sont pas. Comme pour le classement, on choisit une taille de grille et, au besoin, un niveau. L'écran affiche :

- le nombre de parties jouées, de victoires (avec le pourcentage), de défaites et d'abandons ;
- le meilleur temps et le temps moyen d'une victoire ;
- le meilleur score ;
- les victoires d'affilée en ce moment et la meilleure série (une défaite ou un abandon la coupe).

Les statistiques sont dans un fichier texte à côté du classement, que l'on peut supprimer pour les remettre à zéro :

- Windows : `%APPDATA%\Sudoko\statistiques.txt`
- Linux et macOS : `~/.local/share/Sudoko/statistiques.txt`

### Variantes

Le menu propose, en plus du Sudoku classique, trois variantes qui ajoutent une règle. Les zones qui comptent en plus sont colorées dans la grille, et la règle est rappelée sous le titre de la partie.

| Variante | Règle en plus | Tailles |
| -------- | ------------- | ------- |
| **Diagonale** | Les deux grandes diagonales contiennent aussi chaque nombre une seule fois. | toutes |
| **Hyper** | Quatre fenêtres de 3×3, en couleur, contiennent aussi chaque nombre une seule fois. | 9×9 |
| **Anti-cavalier** | Deux cases séparées par un saut de cavalier (comme aux échecs) n'ont pas le même nombre. | toutes |

Tout fonctionne comme en classique : les trois niveaux (mesurés aux techniques de déduction, les diagonales et les fenêtres comptant comme des zones pour « chiffre unique », les paires, les intersections...), les indices expliqués, les notes, l'annulation, les chiffres faux en rouge et la sauvegarde. En sélectionnant une case, on met en valeur toutes les cases qui ne peuvent pas contenir le même nombre : pour l'anti-cavalier, cela montre les sauts de cavalier. Le score, le classement et les statistiques sont séparés par variante.

L'écran « Résoudre une grille » ne gère que les grilles classiques.

### Résoudre une grille

Le bouton `Résoudre une grille` du menu ouvre un écran où l'on recopie une grille de journal (9×9) pour que le programme la résolve.

- Cliquez sur une case et tapez les chiffres de 1 à 9 : la sélection avance d'une case à chaque chiffre, pour saisir toute la grille d'une traite. `0` laisse une case vide et passe à la suivante. `Suppr` ou `Retour arrière` vident la case sans bouger, les flèches déplacent la sélection, et les boutons `1` à `9` et `Effacer` font la même chose à la souris.
- `Ctrl+V` colle une grille entière : 81 cases avec des chiffres de 1 à 9 et `0` ou `.` pour les cases vides. Les espaces, les retours à la ligne et les traits de dessin (`|`, `+`, `-`) sont ignorés, ce qui permet de copier une grille depuis un site.
- Deux chiffres identiques dans une même ligne, colonne ou bloc s'affichent en rouge, et la résolution est refusée tant qu'ils sont là (c'est souvent une faute de frappe).
- `Résoudre` remplit les cases vides : les chiffres saisis restent en noir, ceux trouvés par le programme sont en bleu. Un message indique si la solution est unique, s'il y en a plusieurs (le programme en montre alors une, et rappelle qu'une grille a besoin d'au moins 17 chiffres pour avoir une solution unique) ou s'il n'y en a aucune. La recherche s'arrête au bout d'environ une seconde si elle est trop longue.
- `Modifier la grille` retire la solution pour corriger la saisie, et `Tout effacer` repart d'une grille vide.

### Réglages

Le bouton `Réglages` du menu ouvre l'écran des réglages. Chaque changement s'applique tout de suite et est enregistré.

- **Thème** : `Celui du système` (par défaut), `Clair` ou `Sombre`. Il s'applique à la fenêtre et à la grille, qui prend des couleurs sombres en thème sombre.
- **Mode daltonien** : le chiffre du joueur passe en bleu, le chiffre faux en orange (palette d'Okabe et Ito, lisible quand on confond le rouge et le vert) et il est en plus souligné, pour ne pas dépendre de la couleur seule. Un aperçu montre le résultat.
- **Aide visuelle** : mettre en valeur, ou non, la ligne, la colonne et le bloc de la case choisie, et les chiffres identiques à celui de la case choisie. Les deux sont activés par défaut.
- **Sons** : un petit son pour un bon chiffre, une erreur, un indice, une victoire et une défaite. Ils sont désactivés par défaut, et des boutons permettent de les écouter avant de les activer. Ils ne sont joués que sous Windows (ce sont des bips du système) ; ailleurs, le jeu reste silencieux.
- `Réglages par défaut` remet tout comme à l'installation.

Les réglages sont dans un fichier texte à côté des autres, que l'on peut supprimer pour revenir aux réglages par défaut :

- Windows : `%APPDATA%\Sudoko\reglages.txt`
- Linux et macOS : `~/.local/share/Sudoko/reglages.txt`

### Au terminal

Lancez le programme avec `terminal` en premier argument (voir plus bas). Il demande un coup sous la forme `ligne colonne valeur`, avec des numéros de 1 jusqu'au côté de la grille (de 1 à 16 en 16×16).

| Saisie  | Effet                                        |
| ------- | -------------------------------------------- |
| `1 3 4` | Place le 4 en ligne 1, colonne 3             |
| `1 3 0` | Efface la case en ligne 1, colonne 3         |
| `h`     | Révèle une case et explique la technique (indice) |
| `s`     | Affiche la solution et termine la partie     |
| `q`     | Quitte la partie                             |

Un coup est refusé s'il brise une règle du sudoku ou s'il vise une case de la grille de départ. (Dans la fenêtre, au contraire, un chiffre faux est posé en rouge et compte une erreur.)

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

Sans argument, le programme ouvre la fenêtre sur le menu. Pour lancer directement une partie, donnez le niveau, puis éventuellement la taille de la grille (4, 6, 9, 12 ou 16 ; 9 par défaut) et la variante (`classique` par défaut, `diagonale`, `hyper` ou `anticavalier`) :

```bash
cargo run -- facile          # grille 9×9 facile
cargo run -- difficile 16    # grille 16×16 difficile
cargo run -- moyen 6         # grille 6×6 moyenne
cargo run -- moyen 9 diagonale     # grille 9×9 moyenne en diagonale
cargo run -- difficile 12 anticavalier
cargo run -- fixe            # toujours la même grille d'entraînement (9×9 seulement)
```

Pour jouer au terminal au lieu de la fenêtre, ajoutez `terminal` en premier. Le terminal n'a ni menu, ni limite d'erreurs, ni chronomètre, ni score :

```bash
cargo run -- terminal              # grille 9×9 moyenne
cargo run -- terminal facile 12    # ou difficile, fixe ; taille 4, 6, 9, 12 ou 16
cargo run -- terminal moyen 9 hyper
```

Au terminal, une variante n'est pas colorée : la règle en plus est rappelée au début de la partie.

La génération d'une grille prend en général moins d'une seconde, et quelques secondes au plus pour une grande grille difficile avec `cargo run`. Elle est bien plus rapide avec `cargo run --release` (environ une seconde au plus, pour une 16×16 difficile).

## Exécutable

Pour obtenir un programme que l'on peut lancer sans Rust ni `cargo` :

```bash
cargo build --release
```

Le résultat est le fichier `target/release/Sudoko.exe` (sous Windows), d'environ 7 Mo. C'est un seul fichier : on peut le copier où l'on veut et le lancer d'un double-clic. Il accepte les mêmes arguments que `cargo run --` (`Sudoko.exe facile`, `Sudoko.exe terminal`...).

Sous Windows, il faut les composants Visual C++ (`VCRUNTIME140.dll`, présents sur la plupart des ordinateurs) et une carte graphique compatible OpenGL. Le dossier `target/` n'est pas versionné : pour partager l'exécutable, déposez-le par exemple dans une *release* GitHub.

La compilation en mode `release` est longue (7 minutes environ la première fois) et consomme beaucoup de mémoire : `cargo build --release -j 2` limite la charge.

### Compilation automatique sur GitHub

Le fichier [.github/workflows/ci.yml](.github/workflows/ci.yml) fait travailler GitHub à votre place, sur une machine Windows :

- À chaque envoi sur `main` et à chaque demande de fusion, il lance les tests puis compile l'exécutable. L'exécutable est téléchargeable dans l'onglet *Actions* du dépôt, sur la page de l'exécution (artefact `Sudoko-windows`, gardé quelques semaines).
- Pour publier une version, créez une étiquette qui commence par `v` et envoyez-la :

  ```bash
  git tag v1.0.0
  git push origin v1.0.0
  ```

  GitHub teste, compile et crée la *release* `v1.0.0` avec `Sudoko.exe` en pièce jointe, accessible depuis l'onglet *Releases*.

## Tests

```bash
cargo test
```

## Structure du projet

```text
Sudoko/
├── .github/workflows/ci.yml # Tests et exécutable compilés automatiquement par GitHub
├── Cargo.toml    # Configuration du projet et dépendances
├── src/
│   ├── main.rs   # Point d'entrée : choix de la grille et de l'interface selon les arguments
│   ├── gui.rs    # Interface graphique (egui) : menu, partie, erreurs, chrono, écran de fin, classement, statistiques
│   ├── classement.rs # Calcul du score et classement enregistré dans un fichier
│   ├── jeu.rs    # Boucle de jeu au terminal et lecture des commandes
│   ├── sauvegarde.rs # Sauvegarde et reprise de la partie en cours (fichier texte)
│   ├── statistiques.rs # Parties terminées enregistrées dans un fichier, et calcul des statistiques
│   ├── grille.rs # Structure Grille et ses tailles : règles, chargement, génération, indices
│   ├── reglages.rs # Réglages du joueur (thème, couleurs, sons) enregistrés dans un fichier
│   ├── regles.rs # Les règles d'une grille : zones, voisines, variantes (diagonale, hyper, anti-cavalier)
│   ├── resolveur.rs # Saisie d'une grille 9×9 et sa résolution (écran « Résoudre une grille »)
│   ├── solveur.rs # Solveur par retour sur trace (candidats en bits, case la plus contrainte), pour toutes les variantes
│   ├── technique.rs # Résolution « à la main » par déduction : mesure de la difficulté et explication des indices
│   └── sons.rs   # Les bips du jeu (Windows)
├── LICENSE
└── README.md
```

## Licence

Ce projet est distribué sous licence MIT. Voir le fichier [LICENSE](LICENSE).
