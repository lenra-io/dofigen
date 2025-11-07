# Système de patch de structure YAML Dofigen

Ce document explique le système de patch de structure YAML mis en place dans Dofigen, utilisé notamment lors de l'extension de fichiers Dofigen avec le mot-clé `extend`.

## Table des matières

- [Principe général](#principe-général)
- [Types de patch](#types-de-patch)
  - [Remplacement de valeurs simples](#remplacement-de-valeurs-simples)
  - [Patch de tableaux (arrays)](#patch-de-tableaux-arrays)
  - [Patch de tableaux profonds (deep arrays)](#patch-de-tableaux-profonds-deep-arrays)
  - [Patch de maps (HashMaps)](#patch-de-maps-hashmaps)
  - [Patch de maps profondes (deep maps)](#patch-de-maps-profondes-deep-maps)
- [Exemples pratiques](#exemples-pratiques)

## Principe général

Le système de patch permet d'étendre et de modifier la configuration d'un fichier Dofigen en fusionnant plusieurs fichiers. Lorsque vous utilisez le mot-clé `extend`, Dofigen charge le fichier de base et applique les modifications spécifiées dans votre fichier.

### Exemple de base

**Fichier `base.yml`:**
```yaml
fromImage: alpine
workdir: /app
env:
  NODE_ENV: production
```

**Fichier `myapp.yml`:**
```yaml
extend: base.yml
fromImage: node:18-alpine
env:
  PORT: "3000"
```

**Résultat après fusion:**
```yaml
fromImage: node:18-alpine  # Remplacé
workdir: /app              # Hérité de base.yml
env:
  NODE_ENV: production     # Hérité de base.yml
  PORT: "3000"             # Ajouté
```

## Types de patch

### Remplacement de valeurs simples

Les valeurs simples (chaînes, nombres, booléens) sont simplement remplacées lors de l'extension.

**Base:**
```yaml
fromImage: alpine
workdir: /app
user: 1000
```

**Patch:**
```yaml
extend: base.yml
fromImage: ubuntu
user: 1001
```

**Résultat:**
```yaml
fromImage: ubuntu  # Remplacé
workdir: /app      # Conservé
user: 1001         # Remplacé
```

### Patch de tableaux (arrays)

Les tableaux peuvent être patchés de plusieurs façons sophistiquées. Par défaut, un tableau remplace complètement le tableau de base, mais vous pouvez utiliser des clés spéciales pour des opérations plus avancées.

#### Remplacement complet (comportement par défaut)

**Base:**
```yaml
context:
  - /src
  - /package.json
```

**Patch:**
```yaml
extend: base.yml
context:
  - /app
  - /lib
```

**Résultat:**
```yaml
context:
  - /app
  - /lib
```

#### Ajout à la fin du tableau (`+`)

**Base:**
```yaml
run:
  - apt-get update
  - apt-get install -y curl
```

**Patch:**
```yaml
extend: base.yml
run:
  +:
    - apt-get clean
    - rm -rf /var/lib/apt/lists/*
```

**Résultat:**
```yaml
run:
  - apt-get update
  - apt-get install -y curl
  - apt-get clean
  - rm -rf /var/lib/apt/lists/*
```

#### Remplacement d'un élément spécifique (index numérique)

Les index commencent à 0.

**Base:**
```yaml
run:
  - echo "step 1"
  - echo "step 2"
  - echo "step 3"
```

**Patch:**
```yaml
extend: base.yml
run:
  1: echo "step 2 modified"
```

**Résultat:**
```yaml
run:
  - echo "step 1"
  - echo "step 2 modified"
  - echo "step 3"
```

#### Insertion avant un élément (`+n`)

**Base:**
```yaml
run:
  - echo "start"
  - echo "end"
```

**Patch:**
```yaml
extend: base.yml
run:
  "+1":
    - echo "middle 1"
    - echo "middle 2"
```

**Résultat:**
```yaml
run:
  - echo "start"
  - echo "middle 1"
  - echo "middle 2"
  - echo "end"
```

#### Insertion après un élément (`n+`)

**Base:**
```yaml
run:
  - echo "first"
  - echo "second"
```

**Patch:**
```yaml
extend: base.yml
run:
  0+:
    - echo "after first"
```

**Résultat:**
```yaml
run:
  - echo "first"
  - echo "after first"
  - echo "second"
```

#### Combinaison de plusieurs opérations

**Base:**
```yaml
run:
  - echo "1"
  - echo "2"
  - echo "3"
```

**Patch:**
```yaml
extend: base.yml
run:
  0: echo "1 modified"
  1+:
    - echo "2.5"
  +:
    - echo "4"
```

**Résultat:**
```yaml
run:
  - echo "1 modified"
  - echo "2"
  - echo "2.5"
  - echo "3"
  - echo "4"
```

### Patch de tableaux profonds (deep arrays)

Les tableaux d'objets peuvent être patchés encore plus finement avec l'opération `n<` qui permet de modifier partiellement un élément sans le remplacer complètement.

#### Patch partiel d'un élément (`n<`)

**Base:**
```yaml
copy:
  - paths: /src
    target: /app/src
    chown: 1000:1000
  - paths: /package.json
    target: /app/
```

**Patch:**
```yaml
extend: base.yml
copy:
  0<:
    chown: 1001:1001
```

**Résultat:**
```yaml
copy:
  - paths: /src
    target: /app/src
    chown: 1001:1001  # Modifié
  - paths: /package.json
    target: /app/
```

#### Exemple complexe avec builders

**Base:**
```yaml
builders:
  builder1:
    fromImage: node:18
    workdir: /app
    run:
      - npm install
      - npm run build
```

**Patch:**
```yaml
extend: base.yml
builders:
  builder1:
    run:
      1: npm run build:prod  # Remplace le 2ème élément
      +:
        - npm run test       # Ajoute à la fin
```

**Résultat:**
```yaml
builders:
  builder1:
    fromImage: node:18
    workdir: /app
    run:
      - npm install
      - npm run build:prod
      - npm run test
```

### Patch de maps (HashMaps)

Les maps (dictionnaires) peuvent être modifiées en ajoutant, modifiant ou supprimant des clés.

#### Ajout et modification de clés

**Base:**
```yaml
env:
  NODE_ENV: production
  PORT: "3000"
```

**Patch:**
```yaml
extend: base.yml
env:
  NODE_ENV: development  # Modifie
  DEBUG: "true"          # Ajoute
```

**Résultat:**
```yaml
env:
  NODE_ENV: development
  PORT: "3000"
  DEBUG: "true"
```

#### Suppression de clés (avec `null`)

**Base:**
```yaml
env:
  NODE_ENV: production
  PORT: "3000"
  DEBUG: "true"
```

**Patch:**
```yaml
extend: base.yml
env:
  DEBUG: null  # Supprime la clé
```

**Résultat:**
```yaml
env:
  NODE_ENV: production
  PORT: "3000"
```

### Patch de maps profondes (deep maps)

Les maps contenant des objets complexes peuvent être fusionnées profondément.

#### Exemple avec builders

**Base:**
```yaml
builders:
  maven-builder:
    fromImage: maven:3.9
    workdir: /app
    copy:
      - paths: ["."]
    run:
      - mvn package
```

**Patch:**
```yaml
extend: base.yml
builders:
  maven-builder:
    fromImage: maven:3.9-eclipse-temurin-17  # Modifie
    run:
      +:
        - mvn verify  # Ajoute une commande
  new-builder:        # Ajoute un nouveau builder
    fromImage: gradle:8
    workdir: /build
```

**Résultat:**
```yaml
builders:
  maven-builder:
    fromImage: maven:3.9-eclipse-temurin-17  # Modifié
    workdir: /app                            # Conservé
    copy:
      - paths: ["."]                          # Conservé
    run:
      - mvn package                           # Conservé
      - mvn verify                            # Ajouté
  new-builder:                                # Nouveau
    fromImage: gradle:8
    workdir: /build
```

#### Suppression d'un builder

**Base:**
```yaml
builders:
  builder1:
    fromImage: node:18
  builder2:
    fromImage: python:3.11
```

**Patch:**
```yaml
extend: base.yml
builders:
  builder1: null  # Supprime builder1
```

**Résultat:**
```yaml
builders:
  builder2:
    fromImage: python:3.11
```

## Exemples pratiques

### Exemple 1: Extension d'une configuration Spring Boot Maven

**Fichier `springboot-maven.base.yml`:**
```yaml
builders:
  maven-builder:
    fromImage: 
      path: maven
      tag: 3.9-eclipse-temurin-17-alpine
    workdir: /app
    copy:
      - paths: ["."]
    root:
      run:
        - mvn package -DskipTests
        - mv target/*.jar app.jar
      cache:
        - target: /root/.m2
        - target: /app/target

fromImage: 
  path: eclipse-temurin
  tag: 17-jre-alpine

copy:
  - fromBuilder: maven-builder
    paths: [/app/app.jar]
    target: app.jar

cmd: ["java", "-jar", "app.jar"]

context:
  - /pom.xml
  - /src/main/
```

**Fichier `myapp.yml` (simple extension):**
```yaml
extend: springboot-maven.base.yml
```

**Fichier `myapp-java21.yml` (avec modifications):**
```yaml
extend: springboot-maven.base.yml

builders:
  maven-builder:
    fromImage: 
      tag: 3-eclipse-temurin-21-alpine  # Change la version Java

fromImage:
  tag: 21-jre-alpine  # Change la version Java runtime
```

### Exemple 2: Configuration multi-stage complexe

**Base:**
```yaml
builders:
  deps:
    fromImage: node:18-alpine
    workdir: /app
    copy:
      - paths: [package.json, package-lock.json]
    run:
      - npm ci --only=production

  build:
    fromImage: node:18-alpine
    workdir: /app
    copy:
      - fromBuilder: deps
        paths: [/app/node_modules]
        target: node_modules
      - paths: [.]
    run:
      - npm run build

fromImage: node:18-alpine
workdir: /app
copy:
  - fromBuilder: deps
    paths: [/app/node_modules]
    target: node_modules
  - fromBuilder: build
    paths: [/app/dist]
    target: dist

cmd: ["node", "dist/index.js"]
```

**Extension avec optimisations:**
```yaml
extend: base.yml

builders:
  deps:
    cache:  # Ajoute un cache pour accélérer les builds
      - /root/.npm
  build:
    run:
      +:
        - npm run lint  # Ajoute une étape de lint
        - npm test      # Ajoute les tests

env:  # Ajoute des variables d'environnement
  NODE_ENV: production
  PORT: "3000"

expose:  # Expose le port
  - 3000
```

### Exemple 3: Gestion des labels

**Base:**
```yaml
fromImage: alpine:3.18
label:
  maintainer: "team@example.com"
  version: "1.0.0"
```

**Patch:**
```yaml
extend: base.yml

label:
  version: "1.1.0"     # Modifie
  build-date: "2024"   # Ajoute
  maintainer: null     # Supprime
```

**Résultat:**
```yaml
fromImage: alpine:3.18
label:
  version: "1.1.0"
  build-date: "2024"
```

## Ordre d'application des patches

Lorsque plusieurs fichiers sont étendus, les patches sont appliqués dans l'ordre de déclaration:

```yaml
extend:
  - base.yml
  - common.yml
  - specific.yml
```

Les modifications de `specific.yml` ont la priorité sur celles de `common.yml`, qui ont elles-mêmes la priorité sur `base.yml`.

## Bonnes pratiques

1. **Utilisez le remplacement par défaut pour les tableaux simples** : Si vous voulez complètement remplacer une liste, utilisez simplement la syntaxe de tableau standard.

2. **Utilisez `+` pour étendre des listes** : C'est la façon la plus claire d'ajouter des éléments à une liste existante.

3. **Utilisez `n<` pour modifier partiellement des objets dans des tableaux** : Évite de dupliquer toute la configuration d'un objet juste pour modifier un champ.

4. **Utilisez `null` pour supprimer des clés** : C'est la façon explicite de retirer une configuration héritée.

5. **Organisez vos fichiers de base par niveaux** : Créez une hiérarchie de fichiers de base (base générique → configuration spécifique au langage → configuration spécifique au projet).

6. **Documentez les points d'extension** : Dans vos fichiers de base, ajoutez des commentaires pour indiquer quels éléments sont destinés à être modifiés.

## Limitations

- Les opérations de patch sur les tableaux utilisent les index de la liste **avant** l'application du patch. Planifiez vos modifications en conséquence.
- Les opérations `_` (remplacement complet) ne peuvent pas être combinées avec d'autres opérations sur le même tableau.
- L'ordre des opérations sur un même index n'est pas garanti si vous mélangez différents types d'opérations (remplacer, insérer avant/après) sur le même élément.

## Références

- [Structure de référence Dofigen](./struct.md)
- [Code source du système de patch](../src/deserialize.rs)
- [Tests unitaires](../src/deserialize.rs#L1740-L3138) : Contiennent de nombreux exemples d'utilisation
