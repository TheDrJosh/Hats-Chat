use std::collections::HashMap;

use sqlx::PgPool;

use crate::utils::username::Username;

pub struct FiendListInfo {
    pub friends: Vec<Username>,
}

impl FiendListInfo {
    // this code is terible find a way to do this with sql
    pub async fn new(user_id: i32, pool: &PgPool) -> anyhow::Result<Self> {
        let friends = get_friends(user_id, pool).await?;

        let mut friend_names = vec![];

        for friend_id in friends {
            let friend_name = sqlx::query!(
                "SELECT username, display_name FROM users WHERE id = $1;",
                friend_id
            )
            .fetch_one(pool)
            .await?;
            friend_names.push(Username::new(
                friend_name.username,
                friend_name.display_name,
            ))
        }

        Ok(Self {
            friends: friend_names,
        })
    }
}

// this code is terible find a way to do this with sql
pub async fn get_friends(user_id: i32, pool: &PgPool) -> anyhow::Result<Vec<i32>> {
    let mut friends_vec = sqlx::query!(
        "SELECT recipient_id, sent_at FROM chat_messages WHERE sender_id = $1",
        user_id
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|rec| (rec.recipient_id, rec.sent_at))
    .collect::<Vec<_>>();

    friends_vec.append(
        &mut sqlx::query!(
            "SELECT sender_id, sent_at FROM chat_messages WHERE recipient_id = $1",
            user_id
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|rec| (rec.sender_id, rec.sent_at))
        .collect(),
    );

    let mut friends_map: HashMap<i32, Vec<time::PrimitiveDateTime>> = HashMap::new();

    for (friend_id, time) in friends_vec {
        match friends_map.get_mut(&friend_id) {
            Some(friend) => {
                friend.push(time);
            }
            None => {
                friends_map.insert(friend_id, vec![time]);
            }
        }
    }

    let mut friends_and_time = friends_map
        .iter()
        .map(|(friend_id, times)| {
            (
                friend_id,
                times
                    .iter()
                    .max()
                    .expect("map should contain one or more timestamps at this point"),
            )
        })
        .collect::<Vec<_>>();

    friends_and_time.sort_by(|a, b| b.1.cmp(a.1));

    let friends = friends_and_time
        .into_iter()
        .map(|(&friend_id, _)| friend_id)
        .collect();

    Ok(friends)
}
