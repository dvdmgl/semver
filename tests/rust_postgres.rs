#[cfg(feature = "with-rust-postgres-semver64")]
extern crate postgres_types;
extern crate semver;
extern crate tokio_postgres;
use futures::FutureExt;

use tokio_postgres::{Client, NoTls};

use semver::Version;

async fn connect(s: &str) -> Client {
    let (client, connection) =
        tokio_postgres::connect(s, NoTls).await.unwrap();
    let connection = connection.map(|e| e.unwrap());
    tokio::spawn(connection);

    client
}

#[tokio::test]
async fn test_select_version() {
    let client =
        connect("user=postgres password=postgres host=localhost").await;
    let result = client.query("SELECT '1.2.0'::semver", &[]).await.unwrap();
    let ver = result
        .iter()
        .map(|r| r.get::<_, Version>(0))
        .last()
        .unwrap();
    assert_eq!(ver, Version::parse("1.2.0").unwrap());

    let result = client
        .query("SELECT '1.2.0-bar'::semver", &[])
        .await
        .unwrap();
    let ver = result
        .iter()
        .map(|r| r.get::<_, Version>(0))
        .last()
        .unwrap();
    assert_eq!(ver, Version::parse("1.2.0-bar").unwrap());

    let result = client
        .query("SELECT '1.2.0+foo'::semver", &[])
        .await
        .unwrap();
    let ver = result
        .iter()
        .map(|r| r.get::<_, Version>(0))
        .last()
        .unwrap();
    assert_eq!(ver, Version::parse("1.2.0+foo").unwrap());

    let result = client
        .query("SELECT '1.2.0-bar+foo'::semver", &[])
        .await
        .unwrap();
    let ver = result
        .iter()
        .map(|r| r.get::<_, Version>(0))
        .last()
        .unwrap();
    assert_eq!(ver, Version::parse("1.2.0-bar+foo").unwrap());
}

#[tokio::test]
async fn test_query_version() {
    let client =
        connect("user=postgres password=postgres host=localhost").await;

    client
        .batch_execute(
            "CREATE TEMPORARY TABLE foo (
                id SERIAL PRIMARY KEY,
                v SEMVER
            )",
        )
        .await
        .unwrap();

    let stmt = client
        .prepare("INSERT INTO foo (v) VALUES ($1), ($2), ($3), ($4)")
        .await
        .unwrap();
    client
        .execute(
            &stmt,
            &[
                &Version::parse("1.2.3").unwrap(),
                &Version::parse("1.2.3-foo").unwrap(),
                &Version::parse("1.2.3+bar").unwrap(),
                &Version::parse("1.2.3-foo+bar").unwrap(),
            ],
        )
        .await
        .unwrap();

    let stmt = client
        .prepare("SELECT v FROM foo ORDER BY id")
        .await
        .unwrap();
    let rows = client
        .query(&stmt, &[])
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get(0))
        .collect::<Vec<Version>>();

    assert_eq!(
        vec![
            Version::parse("1.2.3").unwrap(),
            Version::parse("1.2.3-foo").unwrap(),
            Version::parse("1.2.3+bar").unwrap(),
            Version::parse("1.2.3-foo+bar").unwrap()
        ],
        rows
    );
}
