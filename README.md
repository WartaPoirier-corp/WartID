# WartID

_:england: Authentication (/ authorization ?) provider for authenticated, present and future, Warta-services._

_:fr: Fournisseur d'authentification (/ autorisation ?) pour les Warta-services présents et futurs nécessitant une authentification._

## Stack info

  * Practical front-ends: _HTTP_ and _Discord bot_
  * High-level protocols: OAuth2, OIDC, JWT
  * Main tech stack: [`rocket`](https://rocket.rs/), [`diesel`](https://diesel.rs/), [`ructe`](https://github.com/kaj/ructe) and `serenity` (inspired by [Plume](https://joinplu.me/), go check it out btw)
  * Other noteworthy external stuff depended on: [`xp.css`](https://botoxparty.github.io/XP.css/), `postgresql`

## Install / run

Variables:

```dotenv
# Discord bot
DISCORD_TOKEN=...
DISCORD_ALLOWED_GUILDS=012345678901234567

# Server
DATABASE_URL=postgres://username:password@localhost/wartid
HTTP_BASE_URL=http://localhost:8000/
```

```
cd wartid-server
cargo run --manifest-path ../wartid-server-discord-bot/Cargo.toml &
cargo run --features discord-bot
```

## Security notice

  * User passwords are stored and validated with bcrypt
  * OAuth2 secrets are stored in plain text (they need to be displayable)
     * We could store them as a tagged union that allow the developper to "hide" it after copying it: `Disabled | Plain(password) | Hidden(bcrypted_password)`
  * `./discord_jwt.key` is extremely sensitive, it contains the key used to forge the Json Web Tokens for discord-based login (and account creation). `wartid-server` SHOULD delete it on SIGINT.
     * It would be safer to directly communicate the key between `wartid-server` and `wartid-server-discord-bot`, although more complex to set up on each machine, especially if one of the processes need to be restarted
