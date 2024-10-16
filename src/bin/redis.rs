use std::collections::{HashMap, HashSet};

use redis::Commands;

fn main() {
    let mut con = get_connection().expect("redis connection error!");

    let _ = do_something(&mut con);


}

fn get_connection() -> redis::RedisResult<redis::Connection> {
    let client = redis::Client::open("redis://localhost")?;
    let con = client.get_connection()?;

    Ok(con)
}

fn do_something(con: &mut redis::Connection) -> redis::RedisResult<()> {
    redis::cmd("SET").arg("my_counter").arg(42).exec(con)?;
    let count : i32 = con.get("my_counter")?;
    println!("{count}");
    let count = con.get("my_counter").unwrap_or(0i32);
    println!("{count}");
    let k : Option<String> = con.get("missing_key")?;
    println!("{:#?}",k);
    redis::cmd("SET").arg("my_name").arg("42").exec(con)?;
    let name : String = con.get("my_name")?;
    println!("{name}");
    let bin : Vec<u8> = con.get("my_binary")?;
    println!("{:#?}",bin);
    let map : HashMap<String, i32> = con.hgetall("my_hash")?;
    println!("{:#?}",map);
    let keys : Vec<String> = con.hkeys("my_hash")?;
    println!("{:#?}",keys);
    let mems : HashSet<i32> = con.smembers("my_set")?;
    println!("{:#?}",mems);
    let (k1, k2) : (String, String) = con.get(&["k1", "k2"])?;
    println!("{k1} {k2}");
    Ok(())
}