---
"@curl-runner/cli": minor
---

`curl-runner convert postman <collection.json>` turns a Postman collection into YAML files and a `.env`. You get one file per request, in a folder per Postman folder. Optional fields that are off in Postman come in commented out. Secrets never land in a file: their names are listed in `.env` and you set the values in `.env.local`. The command prints what it skipped or needs from you, such as scripts or files to pick. The studio's Import dialog now takes Postman collections too, and saves their secrets to the Keychain.
