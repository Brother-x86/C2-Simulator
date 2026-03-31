# C2-Simulator

## help

```
Simule des flux réseau comme un C2

Usage: C2-Simulator [OPTIONS] --url <URL> --sleep <SLEEP> --jitt <JITT>

Options:
  -u, --url <URL>                URL(s) à traiter (répétable : -u url1 -u url2)
  -s, --sleep <SLEEP>            Temps de sleep en secondes (répétable) , default unit(secondes), sinon : s=secondes,m=minutes,h=heures,j=hours, example -s 5m , -s 40s
  -j, --jitt <JITT>              Nombre de hits (répétable)
  -a, --user-agent <USER-AGENT>  User-Agent [default: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:132.0) Gecko/20100101 Firefox/132.0"]
  -d, --debug                    Debug, rajoute les logs de type debug
  -m, --mode <MODE>              Mode d'exécution [default: alternate] [possible values: parallel, alternate]
  -t, --type <type>              Session Type [default: short] [possible values: short, long]
  -i, --iteration <ITERATION>    Nombre d'itérations (-1 = infini) [default: -1]
  -r, --dry-run                  Dry run : affiche la configuration et quitte sans exécuter
  -b, --banner                   Awesome Banner
  -h, --help                     Print help
```

précision:

```
Chaque -u peut être accompagné de son propre -s, -j et -t : les paramètres s'appliquent dans l'ordre aux URLs fournies.
Si une URL n'a pas de valeur correspondante, elle hérite de la dernière valeur spécifiée — ou de la valeur par défaut si aucune n'a été fournie.
```


## Protocoles supportés

| Schéma | Short | Long |
|---|---|---|
| `http://` `https://` | nouvelle connexion à chaque hit | keep-alive pendant le sleep |
| `ws://` `wss://` | connect → ping → close | connect → ping → sleep connecté → close |
| `tcp://host:port` | connect → payload aléatoire → close | connect → payload → sleep → close |
| `udp://host:port` | bind → send → close | bind → send → écoute réponse (timeout=sleep) → close |

> Les payloads TCP/UDP sont des bytes aléatoires de taille aléatoire (1–64 bytes).


## Sessions Short vs Long

| | Short | Long |
|---|---|---|
| **HTTP** | nouvelle connexion TCP à chaque hit | connexion maintenue ouverte pendant le sleep (keep-alive) |
| **WebSocket** | connect → ping → disconnect | connect → ping en boucle → reconnexion auto si drop |

> Le **sleep+jitt** se passe *entre* les connexions en Short, et *pendant* la connexion en Long.




# run

juste un lien
```
cargo run -- -u https://github.com -s2 -j1
```

plusieurs liens dont websocket

```
cargo run -- -u ws://flameshot.website:8000 --sleep 3s --jitt 15 -u https://github.com -s2 -j1
```

lancement en parallel

```
cargo run -- -u ws://flameshot.website:8000 --sleep 3s --jitt 15 -u https://github.com -s2 -j1  --mode parallel

[2026-03-31T05:56:12Z INFO  C2_Simulator] url=ws://flameshot.website:8000 sleep=3s (3s) jitt=15
[2026-03-31T05:56:12Z INFO  C2_Simulator] url=https://github.com sleep=2 (2s) jitt=1
[2026-03-31T05:56:12Z INFO  C2_Simulator] RUN mode=Parallel
[2026-03-31T05:56:12Z INFO  C2_Simulator::link] #2 ws://flameshot.website:8000 → sleep 3s+2s = 5s
[2026-03-31T05:56:12Z INFO  C2_Simulator::link] #2 https://github.com → sleep 2+0s = 2s
[2026-03-31T05:56:14Z INFO  C2_Simulator::link] #3 https://github.com → sleep 2+0s = 2s
[2026-03-31T05:56:16Z INFO  C2_Simulator::link] #4 https://github.com → sleep 2+0s = 2s
[2026-03-31T05:56:17Z INFO  C2_Simulator::link] #3 ws://flameshot.website:8000 → sleep 3s+14s = 17s
[2026-03-31T05:56:18Z INFO  C2_Simulator::link] #5 https://github.com → sleep 2+0s = 2s
[2026-03-31T05:56:21Z INFO  C2_Simulator::link] #6 https://github.com → sleep 2+0s = 2s
[2026-03-31T05:56:23Z INFO  C2_Simulator::link] #7 https://github.com → sleep 2+0s = 2s
[2026-03-31T05:56:25Z INFO  C2_Simulator::link] #8 https://github.com → sleep 2+0s = 2s
[2026-03-31T05:56:27Z INFO  C2_Simulator::link] #9 https://github.com → sleep 2+0s = 2s
[2026-03-31T05:56:29Z INFO  C2_Simulator::link] #10 https://github.com → sleep 2+0s = 2s
```

longue session

```
cargo run -- -u ws://flameshot.website:8000 --sleep 120s --jitt 1 --type long -u https://github.com -s2 -j1 --type short
```


## simulate Websocket serveur

```
python3 -m venv ws-env
source ws-env/bin/activate
pip install websockets

python3 -c "
import asyncio, websockets

async def handler(ws):
    print(f'Connexion: {ws.remote_address}')
    async for msg in ws:
        print(f'reçu: {msg}')

async def main():
    async with websockets.serve(handler, '0.0.0.0', 8000):
        print('Serveur WS lancé sur ws://0.0.0.0:8000')
        await asyncio.Future()  # tourne indéfiniment

asyncio.run(main())
"
```

## simulate tcp server

tcp 

```
while true; do nc -lvp 8000; done
```

udp

```
nc -lvup 8000
```


# Credits

```
By Brother and Claude
🔥
```