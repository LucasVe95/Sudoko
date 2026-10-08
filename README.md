# Sudoko

Un logiciel de Sudoku écrit en Rust, développé pour le fun et pour apprendre le langage.

> **État du projet :** début de développement. Pour l'instant, le programme définit une grille 9×9 et sait y placer et afficher des valeurs. Le jeu en lui-même n'est pas encore jouable.

## Fonctionnalités

Déjà présent :

- Structure `Grille` représentant une grille de 9×9 cases
- Placement d'une valeur dans une case
- Affichage de la grille dans le terminal

Prévu :

- Vérification des règles (ligne, colonne, bloc 3×3)
- Chargement de grilles de départ
- Résolution automatique
- Génération de nouvelles grilles

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
