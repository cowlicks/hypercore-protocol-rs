#![cfg(feature = "v9")]
#![allow(dead_code, unused_imports)]

use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::task::{self, JoinHandle};
use futures_lite::io::{AsyncRead, AsyncWrite};
// use futures_lite::{AsyncReadExt, AsyncWriteExt};
use hypercore_protocol::schema::*;
use hypercore_protocol::{discovery_key, Channel, Event, Message, Protocol, ProtocolBuilder};
use std::io;

mod _util;
use _util::*;

// Drive a stream to completion in a task.
fn drive<S>(mut proto: S) -> JoinHandle<()>
where
    S: Stream + Send + Unpin + 'static,
{
    task::spawn(async move { while let Some(_event) = proto.next().await {} })
}

// Drive a number of streams to completion.
// fn drive_all<S>(streams: Vec<S>) -> JoinHandle<()>
// where
//     S: Stream + Send + Unpin + 'static,
// {
//     let join_handles = streams.into_iter().map(drive);
//     task::spawn(async move {
//         for join_handle in join_handles {
//             join_handle.await;
//         }
//     })
// }

// Drive a protocol stream until the first channel arrives.
fn drive_until_channel<IO>(
    mut proto: Protocol<IO>,
) -> JoinHandle<io::Result<(Protocol<IO>, Channel)>>
where
    IO: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    task::spawn(async move {
        while let Some(event) = proto.next().await {
            let event = event?;
            match event {
                Event::Channel(channel) => return Ok((proto, channel)),
                _ => {}
            }
        }
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Protocol closed before a channel was opened",
        ))
    })
}

#[async_std::test]
async fn stream_extension() -> anyhow::Result<()> {
    // env_logger::init();
    let (mut proto_a, mut proto_b) = create_pair_memory().await?;

    let mut ext_a = proto_a.register_extension("ext").await;
    let mut ext_b = proto_b.register_extension("ext").await;

    drive(proto_a);
    drive(proto_b);

    task::spawn(async move {
        while let Some(message) = ext_b.next().await {
            assert_eq!(message, b"hello".to_vec());
            // eprintln!("B received: {:?}", String::from_utf8(message));
            ext_b.send(b"ack".to_vec()).await;
        }
    });

    ext_a.send(b"hello".to_vec()).await;
    let response = ext_a.next().await;
    assert_eq!(response, Some(b"ack".to_vec()));
    // eprintln!("A received: {:?}", response.map(String::from_utf8));
    Ok(())
}

#[async_std::test]
async fn channel_extension() -> anyhow::Result<()> {
    // env_logger::init();
    let (mut proto_a, mut proto_b) = create_pair_memory().await?;
    let key = [1u8; 32];

    proto_a.open(key).await?;
    proto_b.open(key).await?;

    let next_a = drive_until_channel(proto_a);
    let next_b = drive_until_channel(proto_b);
    let (proto_a, mut channel_a) = next_a.await?;
    let (proto_b, mut channel_b) = next_b.await?;

    let mut ext_a = channel_a.register_extension("ext").await;
    let mut ext_b = channel_b.register_extension("ext").await;

    drive(proto_a);
    drive(proto_b);
    drive(channel_a);
    drive(channel_b);

    task::spawn(async move {
        while let Some(message) = ext_b.next().await {
            // eprintln!("B received: {:?}", String::from_utf8(message));
            assert_eq!(message, b"hello".to_vec());
            ext_b.send(b"ack".to_vec()).await;
        }
    });

    ext_a.send(b"hello".to_vec()).await;
    let response = ext_a.next().await;
    assert_eq!(response, Some(b"ack".to_vec()));
    // eprintln!("A received: {:?}", response.map(String::from_utf8));
    Ok(())
}

#[async_std::test]
async fn channel_extension_async_read_write() -> anyhow::Result<()> {
    // env_logger::init();
    let (mut proto_a, mut proto_b) = create_pair_memory().await?;
    let key = [1u8; 32];

    proto_a.open(key).await?;
    proto_b.open(key).await?;

    let next_a = drive_until_channel(proto_a);
    let next_b = drive_until_channel(proto_b);
    let (proto_a, mut channel_a) = next_a.await?;
    let (proto_b, mut channel_b) = next_b.await?;

    let mut ext_a = channel_a.register_extension("ext").await;
    let mut ext_b = channel_b.register_extension("ext").await;

    drive(proto_a);
    drive(proto_b);
    drive(channel_a);
    drive(channel_b);

    task::spawn(async move {
        let mut read_buf = vec![0u8; 3];
        // let mut total = 0;
        let mut res = vec![];
        while res.len() < 10 {
            let n = ext_b.read(&mut read_buf).await.unwrap();
            // eprintln!(
            //     "B read: n {} buf {}",
            //     n,
            //     std::str::from_utf8(&read_buf[..n]).unwrap()
            // );
            res.extend_from_slice(&read_buf[..n]);
        }
        assert_eq!(res, b"helloworld".to_vec());

        let write = b"ack".to_vec();
        ext_b.write_all(&write).await.unwrap();
    });

    ext_a.write_all(b"hello").await.unwrap();
    ext_a.write_all(b"world").await.unwrap();

    let mut read_buf = vec![0u8; 5];
    let n = ext_a.read(&mut read_buf).await.unwrap();
    assert_eq!(n, 3);
    assert_eq!(&read_buf[..n], b"ack");
    // eprintln!(
    //     "A read: n {} buf {}",
    //     n,
    //     std::str::from_utf8(&read_buf[..n]).unwrap()
    // );
    Ok(())
}
