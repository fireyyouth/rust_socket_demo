use tokio::net::TcpStream;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::lookup_host;
extern crate futures;


async fn http_get(host : &str, path : &str) -> Result<(), std::io::Error> {
    let host_port = format!("{}:{}", host, 80);
    for addr in lookup_host(host_port).await? {
        let mut stream = TcpStream::connect(addr).await?;
        stream.write_all(format!("GET {} HTTP/1.0\r\nHost: {}\r\n\r\n", path, host).as_bytes()).await?;
        let mut rsp : Vec<u8> = Vec::new();
        stream.read_to_end(&mut rsp).await?;
        println!("rsp for {}({}) {} is {} bytes long", host, addr, path, rsp.len());
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    for _ in 0..1000 {
        let mut handles = vec![];
        let hosts = ["www.baidu.com", "www.qq.com", "www.taobao.com"];
        for host in hosts.iter() {
            handles.push(tokio::spawn(http_get(host, "/")));
        }
        futures::future::join_all(handles).await;
    }
    Ok(())
}

