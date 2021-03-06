use dotenv::dotenv;
use std::env;

use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use lazy_static::lazy_static;

use serde_yaml;

use serenity::{
    model::{
        channel::Message,
        gateway::{Activity, Ready},
        id::{GuildId, UserId},
        voice::VoiceState,
    },
    prelude::*,
};

lazy_static! {
    static ref PREFIX: String = match env::var("PREFIX") {
        Ok(x) => x,
        Err(_) => "!".to_string()
    };
}

struct Handler;

impl EventHandler for Handler {
    fn message(&self, _ctx: Context, _msg: Message) {
        if _msg.guild_id.is_some() {
            let author_id: u64 = *_msg.author.id.as_u64();
            let guild_id: u64 = *_msg.guild_id.unwrap().as_u64();

            let command: String = _msg.content.replace(&*PREFIX, "");
            match &*command {
                "subscribe" => {
                    let reader = File::open("subscribers.yaml").expect("File not found");

                    let mut subscribers: BTreeMap<u64, Vec<u64>> =
                        serde_yaml::from_reader(&reader).expect("Failed to parse");

                    let mut subscribers_guild: Vec<u64> = match subscribers.get(&guild_id) {
                        Some(x) => x.clone(),
                        None => Vec::new(),
                    };
                    let subscribed = subscribers_guild
                        .iter()
                        .find(|&x| x == &author_id)
                        .is_some();

                    if subscribed {
                        if let Err(why) = _msg
                            .channel_id
                            .say(&_ctx.http, "You are already subscribed")
                        {
                            println!("Error sending message: {:?}", why);
                        }
                    } else {
                        subscribers_guild.push(author_id);
                        subscribers.insert(guild_id, subscribers_guild);

                        let writer = File::create("subscribers.yaml").expect("File not found");
                        serde_yaml::to_writer(&writer, &subscribers).expect("Failed to save");

                        println!(
                            "{} has just subscribed in {}",
                            _msg.author.tag(),
                            &_msg
                                .guild_id
                                .unwrap()
                                .to_guild_cached(&_ctx)
                                .unwrap()
                                .read()
                                .name
                        );

                        if let Err(why) = _msg.channel_id.say(&_ctx.http, "You are now subscribed")
                        {
                            println!("Error sending message: {:?}", why);
                        }
                    }
                }
                "unsubscribe" => {
                    let reader = File::open("subscribers.yaml").expect("File not found");

                    let mut subscribers: BTreeMap<u64, Vec<u64>> =
                        serde_yaml::from_reader(&reader).expect("Failed to parse");

                    let mut subscribers_guild: Vec<u64> = match subscribers.get(&guild_id) {
                        Some(x) => x.clone(),
                        None => Vec::new(),
                    };

                    let subscribed = subscribers_guild
                        .iter()
                        .find(|&x| x == &author_id)
                        .is_some();

                    if subscribed {
                        let index = subscribers_guild
                            .iter()
                            .position(|x| x == &author_id)
                            .expect("Wait what");
                        subscribers_guild.remove(index);
                        subscribers.insert(guild_id, subscribers_guild);

                        let writer = File::create("subscribers.yaml").expect("File not found");
                        serde_yaml::to_writer(&writer, &subscribers).expect("Failed to save");

                        println!(
                            "{} has just unsubscribed in {}",
                            _msg.author.tag(),
                            &_msg
                                .guild_id
                                .unwrap()
                                .to_guild_cached(&_ctx)
                                .unwrap()
                                .read()
                                .name
                        );

                        if let Err(why) = _msg
                            .channel_id
                            .say(&_ctx.http, "You are no longer subscribed")
                        {
                            println!("Error sending message: {:?}", why);
                        }
                    } else {
                        if let Err(why) = _msg.channel_id.say(&_ctx.http, "You are not subscribed")
                        {
                            println!("Error sending message: {:?}", why);
                        }
                    }
                }
                "help" => {
                    if let Err(why) = _msg.channel_id.send_message(&_ctx.http, |m| {                    
                        m.embed(|e| {

                            e.field("About", [
                                format!("Hey! My name is {name} and I'm here to notify you when someone joins a voice channel. You may check out my source code on GitHub.", name=_ctx.cache.read().user.name),
                                format!("https://github.com/DEVICARUS/watcher")
                            ].join("\n"), false);

                            e.field("Commands", [
                                format!("`{prefix}subscribe` - Subscribe for notifications", prefix=&*PREFIX),
                                format!("`{prefix}unsubscribe` - Unsubscribe from notifications", prefix=&*PREFIX),
                                format!("`{prefix}help` - This dialog", prefix=&*PREFIX)
                            ].join("\n"), false);
                    
                            e
                        });
                    
                        m
                    }) {
                        println!("Error sending message: {:?}", why);
                    }
                }
                _ => {}
            }
        }
    }

    fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        ctx.set_activity(Activity::playing(&format!("{}help", &*PREFIX)))
    }

    fn voice_state_update(
        &self,
        _ctx: Context,
        _guild_id: Option<GuildId>,
        _old: Option<VoiceState>,
        _new: VoiceState,
    ) {
        if _old.is_none() {
            let guild = _guild_id
                .unwrap()
                .to_guild_cached(&_ctx)
                .unwrap()
                .read()
                .clone();
            let reader = File::open("subscribers.yaml").expect("File not found");

            let subscribers: BTreeMap<u64, Vec<u64>> =
                serde_yaml::from_reader(&reader).expect("Failed to parse");

            let subscribers_guild: Vec<u64> = match subscribers.get(_guild_id.unwrap().as_u64()) {
                Some(x) => x.clone(),
                None => Vec::new(),
            };

            for subscriber in subscribers_guild {
                let joiner = _new.user_id.to_user_cached(&_ctx).unwrap().read().clone();
                let message = format!("`{}` joined a voice channel in `{}`", joiner.name, guild.name);
                
                if joiner.id != UserId(subscriber) {
                    UserId(subscriber)
                        .to_user_cached(&_ctx)
                        .unwrap()
                        .read()
                        .direct_message(&_ctx, |m| m.content(message))
                        .expect("Failed to send notification");
                }
            }
        }
    }
}

fn main() {
    dotenv().ok();

    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN to be in the environment");

    if !Path::new("subscribers.yaml").exists() {
        File::create("subscribers.yaml")
            .expect("Failed to create a subscribers file")
            .write_all(b"---\n0:\n  - 0")
            .expect("Failed to write into sunbscribers file");
    }

    let mut client = Client::new(&token, Handler).expect("Err creating client");

    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}
