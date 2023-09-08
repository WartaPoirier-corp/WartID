use crate::config::Config;
use chrono::{Duration, Utc};
use jsonwebtoken::errors::{Error as JwtError, ErrorKind as JwtErrorKind};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation};
use rocket::form::validate::Contains;
use serenity::client::bridge::gateway::ShardManager;
use serenity::framework::StandardFramework;
use serenity::http::{CacheHttp, Typing};
use serenity::model::channel::Message;
use serenity::model::id::{GuildId, UserId};
use serenity::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;

fn random_of<T: Copy>(slice: &[T]) -> T {
    use rand::seq::SliceRandom;

    *slice.choose(&mut rand::thread_rng()).expect("empty slice")
}

/// The claims embedded inside the JWT
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Claims {
    pub exp: i64,

    /// Subject (Discord user ID)
    pub sub: u64,

    /// Discord user name (for initial account name)
    pub name: String,
}

fn encode(
    key: &EncodingKey,
    discord_user: UserId,
    name: String,
) -> Result<String, jsonwebtoken::errors::Error> {
    jsonwebtoken::encode(
        &Header::default(),
        &Claims {
            exp: (Utc::now() + Duration::minutes(10)).timestamp(),
            sub: discord_user.0,
            name,
        },
        key,
    )
}

struct Handler {
    key: EncodingKey,
    login_url: String,
    allowed_guilds: Arc<[u64]>,
    allowed_users_cache: RwLock<Vec<UserId>>,
}

impl Handler {
    async fn is_user_allowed(&self, ctx: impl CacheHttp, user_id: UserId) -> bool {
        if self.allowed_users_cache.read().await.contains(&user_id) {
            return true;
        }

        for guild in self.allowed_guilds.iter().copied().map(GuildId) {
            if guild.member(&ctx, user_id).await.is_ok() {
                let mut users_cache = self.allowed_users_cache.write().await;
                users_cache.push(user_id);
                return true;
            }
        }

        false
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, received_message: Message) {
        if received_message.author.bot
            || !(received_message.is_private()
                || received_message.mentions_me(&ctx).await.unwrap_or(false))
        {
            return;
        }

        if !self.is_user_allowed(&ctx, received_message.author.id).await {
            log::warn!("foreign user attempted to get a token");
            let _ = received_message.reply(&ctx, "Je te connais pas.").await;
            return;
        }

        if !received_message.is_private() {
            let _ = received_message.reply(
                &ctx,
                random_of(&[
                    "Vas donc voir tes DM (c'est un URL de connection privé, je ne vais pas te l'envoyer ici)",
                    "J'aime pas trop le concept d'envoyer un URL de connection en public, regarde tes DM <:CRONCHE:754810929748901998>",
                    "Je t'ai envoyé un URL de connection PRIVÉ en DM. La prochaine fois demande le moi directement en pv <:trokoul_pulseur:637313805197639690>",
                    "Je viens de te glisser un petit MP doux 😏 avec ton code grrrhh",
                ]),
            ).await;
        }

        // Create and send token

        let Ok(private) = received_message.author.create_dm_channel(&ctx).await else {
            return;
        };

        {
            let typing = Typing::start(ctx.http.clone(), private.id.0);

            let jwt = encode(
                &self.key,
                received_message.author.id,
                received_message.author.name,
            );

            match jwt {
                Ok(jwt) => {
                    let url = format!("{}{jwt}", self.login_url);

                    let _ = private.send_message(&ctx, |m| m.content(
                        format!(
                            "{}\n{}",
                            random_of(&[
                                &format!("Rends toi sur {} pour te connecter à WartID (attention, ça va aller vite)", &url),
                                &format!("Il faut maintenant suivre ce lien pour t'identifier sur WartID: {}", &url),
                            ]),
                            random_of(&[
                                "Le lien expire dans 10 min",
                                "Tu as 10 min 🕑",
                                "Mes pouvoirs ne me permettent pas d'invoquer un lien durant plus de 10 min, dépêche toi !",
                                "🔥 Go 🚶 go 🏁 go 🏁, tu as 1️0️ min avant 💥 l'autodestruction 💣 de ton lien 🔐",
                            ]),
                        )
                    )).await;
                }
                Err(err) => {
                    let _ = private
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

            let _ = typing.map(Typing::stop);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UnauthorizedError {
    #[error("expired login code, try regenerating a newer one")]
    Expired,

    #[error("invalid login code: {0}")]
    Invalid(#[from] JwtError),
}

pub struct DiscordAgent {
    key: DecodingKey,
}

impl DiscordAgent {
    pub fn try_authorize(&self, login_token: &str) -> Result<Claims, UnauthorizedError> {
        let validation = &{
            let mut v = Validation::default();
            v.validate_exp = true;
            v
        };

        jsonwebtoken::decode(login_token, &self.key, validation)
            .map(|data: TokenData<Claims>| data.claims)
            .map_err(|err| match err.kind() {
                JwtErrorKind::ExpiredSignature => UnauthorizedError::Expired,
                _ => UnauthorizedError::from(err),
            })
    }

    pub fn fairing() -> impl rocket::fairing::Fairing {
        use rocket::fairing::{Info, Kind};
        use rocket::{Build, Orbit, Rocket};

        struct FairingInner {
            shard_manager: Arc<Mutex<ShardManager>>,
            main_loop: JoinHandle<serenity::Result<()>>,
        }

        #[derive(Default)]
        struct Fairing {
            inner: Mutex<Option<FairingInner>>,
            shutting_down: Arc<AtomicBool>,
        }

        #[async_trait]
        impl rocket::fairing::Fairing for Fairing {
            fn info(&self) -> Info {
                Info {
                    name: "discord agent",
                    kind: Kind::Ignite | Kind::Shutdown,
                }
            }

            async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
                let config = rocket.state::<Config>().unwrap();
                let discord_config = config.discord.as_ref().unwrap();

                log::debug!("generating the Discord agent's login URL keypair");

                use rand::Rng;
                let secret: [u8; 32] = rand::rngs::OsRng.gen();
                let agent = DiscordAgent {
                    key: DecodingKey::from_secret(&secret),
                };

                let mut bot =
                    Client::builder(&discord_config.token, GatewayIntents::DIRECT_MESSAGES)
                        .event_handler(Handler {
                            key: EncodingKey::from_secret(&secret),
                            login_url: format!("{}login-with-discord?token=", config.base_url),

                            allowed_guilds: discord_config.allowed_guilds.clone(),
                            allowed_users_cache: RwLock::default(),
                        })
                        .framework(StandardFramework::new())
                        .await
                        .expect("error creating client");

                let shard_manager = bot.shard_manager.clone();

                let shutting_down = Arc::clone(&self.shutting_down);
                let main_loop = tokio::task::spawn(async move {
                    let result = bot.start().await;
                    if !shutting_down.load(Ordering::SeqCst) {
                        if let Err(err) = &result {
                            log::error!("discord agent stopped with error, app is in an unrecoverable state: {err}");
                        } else {
                            log::error!("discord agent stopped unexpectedly with no error, app is in an unrecoverable state");
                        }
                    }
                    result
                });

                let _ = self.inner.lock().await.insert(FairingInner {
                    shard_manager,
                    main_loop,
                });

                Ok(rocket.manage(Arc::new(agent)))
            }

            async fn on_shutdown(&self, _rocket: &Rocket<Orbit>) {
                self.shutting_down.store(true, Ordering::SeqCst);
                let inner = self.inner.lock().await.take().unwrap();

                let mut sm = inner.shard_manager.lock().await;
                log::info!("gracefully shutting down discord agent");
                sm.shutdown_all().await;
                drop(sm);

                match inner.main_loop.await {
                    Ok(Ok(())) => log::info!("discord agent shut down successfully"),
                    Ok(Err(_)) => (), // error would have already been logged
                    Err(panic_err) => log::error!(
                        "discord agent must have panicked during its execution: {panic_err}"
                    ),
                }
            }
        }

        Fairing::default()
    }
}
