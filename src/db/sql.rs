use std::error::Error;

use ricq::structs::{FriendGroupInfo, FriendInfo, GroupInfo};
use rusqlite::{params, Connection};

use crate::handler::CLIENT;

#[derive(Debug)]
pub struct Config {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Friend {
    pub id: i64,
    pub name: String,
    // TODO: Make this Option<String>
    pub remark: String,
    pub group_id: u8,
}

pub struct FriendsGroup {
    pub id: u8,
    pub name: String,
}

#[derive(Debug)]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub owner_id: i64,
}

pub fn init_sqlite() {
    // Initialize `~/.gtk-qq/` directory
    let mut db_path = dirs::home_dir().unwrap();
    db_path.push(".gtk-qq");
    std::fs::create_dir_all(db_path.clone()).unwrap();

    // Create or read from `~/.gtk-qq/data.db`
    db_path.push("data.db");
    let conn = Connection::open(db_path).unwrap();

    conn.execute(
        "Create table if not exists configs (
            key     INT PRIMARY KEY,
            value   TEXT NOT NULL
        )",
        [],
    )
    .unwrap();

    conn.execute(
        "Create table if not exists friends (
            id          INT PRIMARY KEY,
            name        TEXT NOT NULL,
            remark      TEXT NOT NULL,
            group_id    INT NOT NULL
        )",
        [],
    )
    .unwrap();

    conn.execute(
        "Create table if not exists friends_groups (
            id      INT PRIMARY KEY,
            name    TEXT NOT NULL
        )",
        [],
    )
    .unwrap();

    conn.execute(
        "Create table if not exists groups (
            id          INT PRIMARY KEY,
            name        TEXT NOT NULL,
            owner_id    INT NOT NULL
        )",
        [],
    )
    .unwrap();
}

pub fn get_db() -> Connection {
    let mut db_path = dirs::home_dir().unwrap();
    db_path.push(".gtk-qq");
    db_path.push("data.db");
    Connection::open(db_path).unwrap()
}

pub async fn refresh_friends_list() -> Result<(), Box<dyn Error>> {
    let conn = get_db();
    // Request for friend list
    let client = CLIENT.get().unwrap();
    let res = client.get_friend_list().await?;
    // Store the friend list in the memory
    let friends = res.friends;
    let friend_groups = res.friend_groups;
    // Handle the `friends_groups`
    let mut friends_groups = friend_groups
        .iter()
        .map(|(_, v)| v.clone())
        .collect::<Vec<FriendGroupInfo>>();
    friends_groups.sort_by(|a, b| a.seq_id.cmp(&b.seq_id));
    let friends_groups = friends_groups
        .into_iter()
        .map(|friends_group| FriendsGroup {
            id: friends_group.group_id,
            name: friends_group.group_name,
        });
    conn.execute("DELETE FROM friends_groups", [])?;
    let mut stmt = conn.prepare("INSERT INTO friends_groups values (?1, ?2)")?;
    for friends_group in friends_groups {
        stmt.execute(params![friends_group.id, friends_group.name])?;
    }
    // Handle the friends
    let friends = friends.into_iter().map(
        |FriendInfo {
             uin,
             nick,
             remark,
             group_id,
             ..
         }| Friend {
            id: uin,
            name: nick,
            remark,
            group_id,
        },
    );
    conn.execute("DELETE FROM friends", [])?;
    let mut stmt = conn.prepare("INSERT INTO friends values (?1, ?2, ?3, ?4)")?;
    for friend in friends {
        stmt.execute(params![
            friend.id,
            friend.name,
            friend.remark,
            friend.group_id
        ])?;
    }

    Ok(())
}

pub async fn refresh_groups_list() -> Result<(), Box<dyn Error>> {
    let conn = get_db();
    let client = CLIENT.get().unwrap();
    let res = client.get_group_list().await?;

    let groups = res.into_iter().map(
        |GroupInfo {
             code,
             name,
             owner_uin,
             ..
         }| Group {
            id: code,
            name,
            owner_id: owner_uin,
        },
    );

    conn.execute("DELETE FROM groups", [])?;
    let mut stmt = conn.prepare("INSERT INTO groups values (?1, ?2, ?3)")?;
    for group in groups {
        stmt.execute(params![group.id, group.name, group.owner_id])?;
    }

    Ok(())
}

pub fn get_friend_remark(friend_id: i64) -> String {
    get_db()
        .query_row("Select remark from friends where id=?1", [friend_id], |row| {
            row.get(0)
        })
        .unwrap_or_else(|_| {
            println!("Failed to get friend remark: {}", friend_id);
            println!(concat!(
                "Help: Try to refresh the friends list in sidebar. ",
                "If the problem still exists, please report it on Github.",
            ));
            friend_id.to_string()
        })
}

pub fn get_group_name(group_id: i64) -> String {
    get_db()
        .query_row("Select name from groups where id=?1", [group_id], |row| {
            row.get(0)
        })
        .unwrap_or_else(|_| {
            println!("Failed to get group name: {}", group_id);
            println!(concat!(
                "Help: Try to refresh the groups list in sidebar. ",
                "If the problem still exists, please report it on Github.",
            ));
            group_id.to_string()
        })
}