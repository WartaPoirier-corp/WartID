#[macro_use]
extern crate async_trait;

use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header};
use serenity::framework::StandardFramework;
use serenity::http::Typing;
use serenity::model::channel::Message;
use serenity::model::id::{GuildId, UserId};
use serenity::prelude::*;

fn random_of<T: Copy>(slice: &[T]) -> T {
    use rand::seq::SliceRandom;

    *slice.choose(&mut rand::thread_rng()).expect("empty slice")
}

/// The claims embedded inside the JWT
#[derive(serde::Deserialize, serde::Serialize)]
struct Claims {
    exp: i64,

    /// Subject (Discord user ID)
    sub: u64,

    /// Discord user name (for initial account name)
    name: String,
}

#[derive(Debug)]
enum EncodeError {
    UndefinedKeyFileVar,
    InvalidKeyLength,
    KeyFile(std::io::Error),
    Jwt(jsonwebtoken::errors::Error),
}

impl From<std::io::Error> for EncodeError {
    fn from(err: std::io::Error) -> Self {
        EncodeError::KeyFile(err)
    }
}

async fn encode(discord_user: UserId, name: String) -> Result<String, EncodeError> {
    let key = async_fs::read(
        std::env::var("DISCORD_KEY_FILE").map_err(|_| EncodeError::UndefinedKeyFileVar)?,
    )
    .await?;

    if key.len() != 32 {
        return Err(EncodeError::InvalidKeyLength);
    }

    let key = EncodingKey::from_secret(&key);

    jsonwebtoken::encode(
        &Header::default(),
        &Claims {
            exp: (Utc::now() + Duration::minutes(10)).timestamp(),
            sub: discord_user.0,
            name,
        },
        &key,
    )
    .map_err(EncodeError::Jwt)
}

struct Handler(&'static [u64]);

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, received_message: Message) {
        if received_message.author.bot
            || !(received_message.is_private()
                || received_message.mentions_me(&ctx).await.unwrap_or(false))
        {
            return;
        }

        let mut found_in_guild = false;

        // Remember: the bot COULD not be in the guild as it's just a parsed ID
        for guild in self.0.iter().copied().map(GuildId) {
            if guild.member(&ctx, received_message.author.id).await.is_ok() {
                found_in_guild = true;
            }
        }

        if !found_in_guild {
            eprintln!("Foreign user attempted to get a token");
            received_message.reply(&ctx, "Je te connais pas.").await;
            return;
        }

        if !received_message.is_private() {
            received_message.reply(
                &ctx,
                random_of(&[
                    "Vas donc voir tes DM (c'est un URL de connection privé, je ne vais pas te l'envoyer ici)",
                    "J'aime pas trop le concept d'envoyer un URL de connection en public, regarde tes DM <:CRONCHE:754810929748901998>",
                    "Je t'ai envoyé un URL de connection PRIVÉ en DM. La prochaine fois demande le moi directement en pv <:trokoul_pulseur:637313805197639690>",
                ]),
            ).await;
        }

        // Create and send token

        let private = match received_message.author.create_dm_channel(&ctx).await {
            Ok(p) => p,
            Err(_) => return,
        };

        {
            let typing = Typing::start(ctx.http.clone(), private.id.0);

            let jwt = encode(received_message.author.id, received_message.author.name).await;

            match jwt {
                Ok(jwt) => {
                    private.send_message(&ctx, |m| m.content(
                        format!(
                            "{} (le nom d'utilisateur reste vide) `{}`\n{}",
                            random_of(&[
                                "Rends toi sur https://profile.wp-corp.eu.org/login et entre le code suivant **comme mot de passe**",
                                "Il faut maintenant rentrer ton code de connection **comme mot de passe** sur https://profile.wp-corp.eu.org/login",
                            ]),
                            jwt,
                            random_of(&[
                                "Il expire dans 10 min",
                                "Tu as 10 min 🕑",
                                "Mes pouvoirs ne me permettent pas de conjurer un code durant plus de 10 min, dépêche toi !",
                                "🔥 Go 🚶 go 🏁 go 🏁, tu as 1️0️ min avant 💥 l'autodestruction 💣 de ton code 🔐",
                            ]),
                        )
                    )).await;
                }
                Err(err) => {
                    private
                        .send_message(&ctx, |m| {
                            m.content(format!(
                                "{}\n```\n{:?}\n```",
                                random_of(&[
                                    "Une erreur est survenue 😕",
                                    "Ça a merdé...",
                                    "Aïe aïe aïe il s'est passé un truc imprévu 😬",
                                ]),
                                err,
                            ))
                        })
                        .await;
                }
            };

            typing.map(Typing::stop);
        }
    }
}

#[tokio::main]
pub async fn main() {
    let _ = dotenv::dotenv();

    let token = std::env::var("DISCORD_TOKEN").expect("no DISCORD_TOKEN set");

    let guilds: &'static [u64] = {
        std::env::var("DISCORD_ALLOWED_GUILDS")
            .expect("no DISCORD_ALLOWED_GUILDS set")
            .split(',')
            .map(str::parse)
            .collect::<Result<Vec<u64>, _>>()
            .unwrap()
            .leak()
    };

    let mut bot = Client::builder(token)
        .event_handler(Handler(guilds))
        .framework(StandardFramework::new())
        .await
        .expect("error creating client");

    bot.start().await.expect("cannot start bot");
    eprintln!("The discord bot crashed unexpectedly. The app is in an irrecoverable state.");
}
