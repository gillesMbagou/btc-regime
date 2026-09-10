# btc-regime

CLI Rust qui mesure si Bitcoin se comporte plutôt comme un actif tech (corrélé
au Nasdaq) ou comme une valeur refuge (corrélé à l'or), et suit l'évolution de
cette corrélation dans le temps.

Inspiré d'une analyse macro (Keyrock, *An Uncrowded Rally*, 7 septembre 2026)
qui notait que la corrélation glissante 90 jours de Bitcoin avec le Nasdaq est
tombée de 61 % à 34 % depuis fin novembre, pendant que sa corrélation avec
l'or est montée à 54 %, avec un croisement des deux courbes début août.

## Ce que ça fait

1. Récupère l'historique quotidien BTC/USD (CoinGecko, gratuit, 365 jours max),
   Nasdaq Composite et Or (Yahoo Finance, symboles `^IXIC` et `GC=F`).
2. Aligne les séries par date (BTC trade 7j/7, pas les marchés traditionnels).
3. Calcule les rendements journaliers logarithmiques, puis une corrélation de
   Pearson glissante sur une fenêtre configurable (90 jours par défaut).
4. Affiche un rapport terminal : valeurs actuelles, pics de la période,
   graphique ASCII des deux courbes de corrélation, et date du dernier
   croisement Or > Nasdaq.

## Utilisation

```
cargo run -- --window 90 --days 365
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

Aucune clé d'API requise : les deux sources sont des endpoints publics
gratuits.

## Choix techniques

- **Rust stable**, pas de `unsafe`.
- `reqwest` (blocking, rustls) pour le réseau — pas besoin d'async pour trois
  requêtes séquentielles, ça garde le code lisible.
- `anyhow` pour la gestion d'erreurs au niveau applicatif ; chaque étape
  réseau porte un message de contexte explicite.
- Le calcul de corrélation (`src/correlation.rs`) et l'alignement de séries
  (`src/series.rs`) sont des fonctions pures, testées unitairement sans appel
  réseau.
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

## Tests

```
cargo test
```

Couvre la corrélation de Pearson (cas limites : variance nulle, longueurs
différentes, fenêtre plus grande que l'entrée) et l'alignement de séries par
date.
