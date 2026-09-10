# btc-regime

[![CI](https://github.com/gillesMbagou/btc-regime/actions/workflows/ci.yml/badge.svg)](https://github.com/gillesMbagou/btc-regime/actions/workflows/ci.yml)

CLI Rust de lecture de marché Bitcoin, inspiré d'une newsletter macro crypto
(Keyrock, *An Uncrowded Rally*, 7 septembre 2026). Deux sous-commandes :

- **`regime`** — Bitcoin se comporte-t-il comme un actif tech (corrélé au
  Nasdaq) ou comme une valeur refuge (corrélé à l'or), et comment ça évolue.
  Recrée le "Chart of the week" de l'article, qui notait que la corrélation
  glissante 90 jours de Bitcoin avec le Nasdaq est tombée de 61 % à 34 %
  depuis fin novembre, pendant que sa corrélation avec l'or est montée à
  54 %, avec un croisement des deux courbes début août.
- **`pulse`** — funding rate et open interest BTC/ETH en direct, avec une
  lecture simple du levier (construction vs purge). Recrée l'esprit de la
  section "Crypto" de l'article (froth reset après un pic d'open interest,
  funding qui bascule négatif, etc.).

## `regime` — corrélation glissante

1. Récupère l'historique quotidien BTC/USD (CoinGecko, gratuit, 365 jours max),
   Nasdaq Composite et Or (Yahoo Finance, symboles `^IXIC` et `GC=F`).
2. Aligne les séries par date (BTC trade 7j/7, pas les marchés traditionnels).
3. Calcule les rendements journaliers logarithmiques, puis une corrélation de
   Pearson glissante sur une fenêtre configurable (90 jours par défaut).
4. Affiche un rapport terminal : valeurs actuelles, pics de la période,
   graphique ASCII des deux courbes de corrélation, et date du dernier
   croisement Or > Nasdaq.

```
cargo run -- regime --window 90 --days 365
```

```
Régime de corrélation BTC — fenêtre glissante 90j (rendements journaliers)
──────────────────────────────────────────────────────────────────────
  BTC vs Nasdaq (actuel):          +3%
  BTC vs Or (actuel):              +14%
  BTC vs Nasdaq (pic période):     +24%
  BTC vs Or (pic période):         +21%

  ─ = Nasdaq   ─ = Or
[... graphique ASCII ...]

  21 Jan 2026 → 09 Sep 2026

  Dernier croisement Or > Nasdaq : 17 Aug 2026
```

## `pulse` — funding rate & open interest

1. Récupère le mark price et le funding rate courant (Binance Futures,
   `premiumIndex`) et l'historique quotidien d'open interest en USD
   (`openInterestHist`) pour BTC et ETH.
2. Compare l'open interest actuel à celui d'il y a N jours (7 par défaut).
3. Classe la lecture en trois régimes simples : funding chaud + open interest
   en hausse = levier en construction ; funding négatif ou open interest en
   forte baisse = purge/reset ; sinon neutre.

```
cargo run -- pulse --days 7
```

```
Market Pulse — funding rate & open interest (Binance Futures)
──────────────────────────────────────────────────────────────────────

  BTC
    Prix (mark):             $      76820.28
    Funding rate:            +0.0074%/8h  (+8.1% annualisé)
    Open interest:           $         8.23B  (-10.1% sur la période)
    Lecture:                 purge / reset de levier

  ETH
    Prix (mark):             $       2414.36
    Funding rate:            -0.0032%/8h  (-3.5% annualisé)
    Open interest:           $         5.57B  (-3.8% sur la période)
    Lecture:                 purge / reset de levier
```

Aucune clé d'API requise pour aucune des deux commandes : toutes les sources
sont des endpoints publics gratuits.

## Choix techniques

- **Rust stable**, pas de `unsafe`.
- `reqwest` (blocking, rustls) pour le réseau — pas besoin d'async pour un
  petit nombre de requêtes séquentielles, ça garde le code lisible.
- `anyhow` pour la gestion d'erreurs au niveau applicatif ; chaque étape
  réseau porte un message de contexte explicite.
- `clap` (derive) pour les sous-commandes.
- Toute la logique de calcul (`src/correlation.rs`, `src/series.rs`, les
  fonctions pures de `src/pulse.rs`) est testée unitairement sans appel
  réseau, séparée des modules de fetch (`src/data.rs`, fetch de `src/pulse.rs`).
- `textplots` pour le graphique ASCII (rendu braille, sans dépendance C) et
  `colored` pour la mise en forme du terminal.

## Limites connues

- L'API gratuite CoinGecko plafonne l'historique à 365 jours.
- BTC/Nasdaq et BTC/Or sont alignés indépendamment (calendriers de cotation
  différents) puis tronqués à la longueur commune la plus récente : une légère
  approximation du calendrier, sans impact matériel sur la lecture du régime.
- Yahoo Finance n'est pas une API publique documentée officiellement, mais
  reste largement utilisée pour ce type de projet ; une panne ou un
  changement de format côté Yahoo casserait `fetch_yahoo`.
- `pulse` ne lit que Binance Futures, pas l'open interest agrégé multi-
  exchanges de l'article original : les montants absolus (ex. open interest
  BTC) seront donc plus bas que ceux cités dans la newsletter, même si les
  variations et signaux de levier restent lisibles.
- Le seuil "funding chaud" (0.01 %/8h) et le seuil "mouvement d'open interest
  significatif" (3 %) sont des repères de marché usuels, pas des constantes
  calibrées statistiquement.

## Tests

```
cargo test
```

Couvre la corrélation de Pearson (cas limites : variance nulle, longueurs
différentes, fenêtre plus grande que l'entrée), l'alignement de séries par
date, et la classification funding/open interest du module `pulse`.
