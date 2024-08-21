// For convenience sake, all types can be found in the prelude module
use rosu_v2::prelude::*;
use futures::future::{BoxFuture, join_all};
use rosu_mods::GameModsLegacy;

#[tokio::main]
async fn main() {
    // Initialize the client
    let client_id: u64 = CLIENT_ID;
    let client_secret: String = String::from(CLIENT_SECRET);
    let osu: Osu = match Osu::new(client_id, client_secret).await {
        Ok(client) => client,
        Err(why) => panic!(
            "Failed to create client or make initial osu!api interaction: {}",
            why
        ),
    };

    let userids: Vec<u32> = std::fs::read_to_string("userids.txt")
        .expect("Failed to read userids from file")
        .lines()
        .map(|id| id.parse().expect("Failed to parse user id"))
        .collect();
    let beatmapids: Vec<u32> = std::fs::read_to_string("beatmapids.txt")
        .expect("Failed to read beatmapids from file")
        .lines()
        .map(|id| id.parse().expect("Failed to parse beatmap id"))
        .collect();
    let mods: Vec<String> = std::fs::read_to_string("mods.txt")
        .expect("Failed to read mods from file")
        .lines()
        .map(|mod_str| mod_str.to_string())
        .collect();

    let user_futures: Vec<_> = userids
        .iter()
        .map(|id| osu.user(*id))
        .collect();
    let users: Vec<User> = join_all(user_futures)
        .await
        .into_iter()
        .map(|result| result.unwrap_or_else(|why| panic!("Failed to get user: {}", why)))
        .collect();

    let tops_futures: Vec<_> = userids
        .iter()
        .map(|id| {
            osu.user_scores(*id)
                .mode(GameMode::Osu)
                .best()
                .limit(1)
        })
        .collect();
    let tops: Vec<Score> = join_all(tops_futures)
        .await
        .into_iter()
        .map(|result| result.unwrap_or_else(|why| panic!("Failed to get scores: {}", why)))
        .flatten()
        .collect();

    let maps_futures: Vec<_> = beatmapids
        .iter()
        .map(|id| osu.beatmap().map_id(*id))
        .collect();
    let maps: Vec<Beatmap> = join_all(maps_futures)
        .await
        .into_iter()
        .map(|result| result.unwrap_or_else(|why| panic!("Failed to get beatmap: {}", why)))
        .collect();

    let map_futures_pp: Vec<BoxFuture<'static, Result<rosu_pp::Beatmap, String>>> = beatmapids
        .iter()
        .map(|id| {
            let path = format!("https://osu.ppy.sh/osu/{}", id);
            let future: BoxFuture<'static, Result<rosu_pp::Beatmap, String>> = Box::pin(async move {
                rosu_pp::Beatmap::from_path(&path)
                    .map_err(|why| format!("Failed to get beatmap: {}", why))
            });
            future
        })
        .collect();
    let maps_pp: Vec<rosu_pp::Beatmap> = join_all(map_futures_pp)
        .await
        .into_iter()
        .map(|result| result.unwrap_or_else(|why| panic!("Failed to get beatmap: {}", why)))
        .collect();

    // do mod_futures where each mod string is parsed using STRING.parse::<GameModsLegacy>().unwrap()
    let mod_futures: Vec<BoxFuture<'static, GameModsLegacy>> = mods
        .iter()
        .map(|mod_str| {
            let future: BoxFuture<'static, GameModsLegacy> = Box::pin(async move {
                mod_str.parse::<GameModsLegacy>().unwrap()
            });
            future
        })
        .collect();

    let mut user_output = String::new();
    for (user, top) in users.iter().zip(tops.iter()) {
        user_output.push_str(&format!("{}\t{}\t{:?}\t{:?}\t{}\t{:?}\n",
            user.username,
            user.statistics.as_ref().unwrap().playtime,
            user.country,
            user.statistics.as_ref().unwrap().global_rank,
            user.statistics.as_ref().unwrap().pp,
            top.pp
        ));
    }
    let output_path = "user_output.txt";
    std::fs::write(output_path, user_output).expect("Failed to write output to file");
    println!("Output written to {}", output_path);

    let mut beatmap_output = String::new();
    for map in maps.iter() {
        beatmap_output.push_str(&format!("{} - {} [{}]\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{}\t{:?}\n",
            map.mapset.as_ref().unwrap().artist,
            map.mapset.as_ref().unwrap().title,
            map.version,
            map.stars,
            map.od,
            map.ar,
            map.cs,
            map.seconds_drain,
            map.mapset.as_ref().unwrap().creator_name,
            map.mapset.as_ref().unwrap().ranked_date.unwrap().year()
        ));
    }
    let output_path = "beatmap_output.txt";
    std::fs::write(output_path, beatmap_output).expect("Failed to write output to file");
    println!("Output written to {}", output_path);

}
