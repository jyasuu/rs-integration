fn main()
{
    // create connection with to memcached server node:
    let client = memcache::connect("memcache://127.0.0.1:11211?timeout=10&tcp_nodelay=true").unwrap();

    // flush the database:
    client.flush().unwrap();

    // set a string value:
    client.set("foo", "bar", 0).unwrap();

    // retrieve from memcached:
    let value: Option<String> = client.get("foo").unwrap();
    assert_eq!(value, Some(String::from("bar")));
    assert_eq!(value.unwrap(), "bar");

    // prepend, append:
    client.prepend("foo", "foo").unwrap();
    client.append("foo", "baz").unwrap();
    let value: String = client.get("foo").unwrap().unwrap();
    assert_eq!(value, "foobarbaz");

    // delete value:
    client.delete("foo").unwrap();

    // using counter:
    client.set("counter", 40, 0).unwrap();
    client.increment("counter", 2).unwrap();
    let answer: i32 = client.get("counter").unwrap().unwrap();
    assert_eq!(answer, 42);

}