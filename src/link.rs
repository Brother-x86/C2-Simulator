use crate::SessionType;
use futures_util::{SinkExt, StreamExt};
use log::{debug, info, warn};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone)]
pub struct Link {
    pub url: String,
    pub sleep_str: String,
    pub sleep: u64,
    pub jitt: u32,
    pub session_type: SessionType,
}

fn random_payload() -> Vec<u8> {
    let size = (rand::random::<u8>() % 64 + 1) as usize; // 1 à 64 bytes
    (0..size).map(|_| rand::random::<u8>()).collect()
}

fn parse_host_port(url: &str) -> Option<String> {
    // tcp://host:port ou udp://host:port → "host:port"
    url.split_once("://").map(|(_, addr)| addr.to_string())
}

async fn do_sleep(link: &Link, iteration: u64) {
    if link.sleep > 0 || link.jitt > 0 {
        let jitt = if link.jitt > 0 { rand::random::<u32>() % link.jitt } else { 0 };
        let total = link.sleep + jitt as u64;
        info!("#{} {} → sleep {}+{}s = {}s", iteration, link.url, link.sleep_str, jitt, total);
        tokio::time::sleep(tokio::time::Duration::from_secs(total)).await;
    }
}

// ─── HTTP ────────────────────────────────────────────────────────────────────

async fn connect_http(client: &reqwest::Client, url: &str, iteration: u64) -> bool {
    match client.get(url).send().await {
        Ok(resp) => { debug!("#{} {} → {}", iteration, url, resp.status()); true  }
        Err(e)   => { warn!("#{} {} → KO: {}", iteration, url, e);          false }
    }
}

// ─── WebSocket ───────────────────────────────────────────────────────────────

async fn connect_ws(url: &str, iteration: u64) -> bool {
    match tokio_tungstenite::connect_async(url).await {
        Ok(_)  => { debug!("#{} {} → WS OK", iteration, url); true  }
        Err(e) => { warn!("#{} {} → KO: {}", iteration, url, e); false }
    }
}

async fn fetch_long_ws(link: &Link, iteration: u64) {
    match tokio_tungstenite::connect_async(&link.url).await {
        Err(e) => { warn!("#{} {} → connexion KO: {}", iteration, link.url, e); return; }
        Ok((ws, _)) => {
            debug!("#{} {} → WS connecté", iteration, link.url);
            let (mut write, mut read): (
                futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, tokio_tungstenite::tungstenite::Message>,
                futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>
            ) = ws.split();

            if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::Ping(vec![])).await {
                warn!("#{} {} → ping KO: {}", iteration, link.url, e);
                return;
            }

            match read.next().await {
                Some(Ok(msg))  => debug!("#{} {} → {:?}", iteration, link.url, msg),
                Some(Err(e))   => { warn!("#{} {} → KO: {}", iteration, link.url, e); return; }
                None           => { warn!("#{} {} → connexion fermée", iteration, link.url); return; }
            }

            do_sleep(link, iteration).await;
        }
    }
}

// ─── TCP ─────────────────────────────────────────────────────────────────────

async fn connect_tcp(url: &str, iteration: u64) -> bool {
    let addr = match parse_host_port(url) {
        Some(a) => a,
        None    => { warn!("#{} {} → adresse invalide", iteration, url); return false; }
    };

    match tokio::net::TcpStream::connect(&addr).await {
        Err(e) => { warn!("#{} {} → TCP KO: {}", iteration, url, e); false }
        Ok(mut stream) => {
            let payload = random_payload();
            match stream.write_all(&payload).await {
                Ok(_)  => { debug!("#{} {} → TCP OK ({} bytes)", iteration, url, payload.len()); true  }
                Err(e) => { warn!("#{} {} → TCP write KO: {}", iteration, url, e);               false }
            }
        }
    }
}

async fn fetch_long_tcp(link: &Link, iteration: u64) {
    let addr = match parse_host_port(&link.url) {
        Some(a) => a,
        None    => { warn!("#{} {} → adresse invalide", iteration, link.url); return; }
    };

    match tokio::net::TcpStream::connect(&addr).await {
        Err(e) => { warn!("#{} {} → TCP KO: {}", iteration, link.url, e); return; }
        Ok(mut stream) => {
            debug!("#{} {} → TCP connecté", iteration, link.url);
            let payload = random_payload();
            if let Err(e) = stream.write_all(&payload).await {
                warn!("#{} {} → TCP write KO: {}", iteration, link.url, e);
                return;
            }
            debug!("#{} {} → TCP envoyé ({} bytes)", iteration, link.url, payload.len());
            do_sleep(link, iteration).await;
            // stream droppé ici → connexion fermée
        }
    }
}

// ─── UDP ─────────────────────────────────────────────────────────────────────

async fn connect_udp(url: &str, iteration: u64) -> bool {
    let addr = match parse_host_port(url) {
        Some(a) => a,
        None    => { warn!("#{} {} → adresse invalide", iteration, url); return false; }
    };

    match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
        Err(e) => { warn!("#{} {} → UDP bind KO: {}", iteration, url, e); false }
        Ok(socket) => {
            let payload = random_payload();
            match socket.send_to(&payload, &addr).await {
                Ok(n)  => { debug!("#{} {} → UDP OK ({} bytes)", iteration, url, n); true  }
                Err(e) => { warn!("#{} {} → UDP KO: {}", iteration, url, e);         false }
            }
        }
    }
}

async fn fetch_long_udp(link: &Link, iteration: u64) {
    let addr = match parse_host_port(&link.url) {
        Some(a) => a,
        None    => { warn!("#{} {} → adresse invalide", iteration, link.url); return; }
    };

    let socket = match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
        Err(e) => { warn!("#{} {} → UDP bind KO: {}", iteration, link.url, e); return; }
        Ok(s)  => s,
    };

    if let Err(e) = socket.connect(&addr).await {
        warn!("#{} {} → UDP connect KO: {}", iteration, link.url, e);
        return;
    }

    let payload = random_payload();
    if let Err(e) = socket.send(&payload).await {
        warn!("#{} {} → UDP send KO: {}", iteration, link.url, e);
        return;
    }
    debug!("#{} {} → UDP envoyé ({} bytes)", iteration, link.url, payload.len());

    // Écoute la réponse avec timeout = sleep
    let timeout = tokio::time::Duration::from_secs(link.sleep.max(1));
    let mut buf = vec![0u8; 1024];
    match tokio::time::timeout(timeout, socket.recv(&mut buf)).await {
        Ok(Ok(n))  => debug!("#{} {} → UDP réponse ({} bytes)", iteration, link.url, n),
        Ok(Err(e)) => warn!("#{} {} → UDP recv KO: {}", iteration, link.url, e),
        Err(_)     => debug!("#{} {} → UDP timeout (pas de réponse)", iteration, link.url),
    }

    do_sleep(link, iteration).await;
}

// ─── Dispatch ────────────────────────────────────────────────────────────────

async fn fetch_short(link: &Link, iteration: u64, user_agent: &str) {
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .user_agent(user_agent)
        .build()
        .unwrap();

    match link.url.split_once("://").map(|(s, _)| s) {
        Some("ws") | Some("wss") => connect_ws(&link.url, iteration).await,
        Some("tcp")              => connect_tcp(&link.url, iteration).await,
        Some("udp")              => connect_udp(&link.url, iteration).await,
        _                        => connect_http(&client, &link.url, iteration).await,
    };
}

async fn fetch_long(link: &Link, iteration: u64, user_agent: &str) {
    match link.url.split_once("://").map(|(s, _)| s) {
        Some("ws") | Some("wss") => fetch_long_ws(link, iteration).await,
        Some("tcp")              => fetch_long_tcp(link, iteration).await,
        Some("udp")              => fetch_long_udp(link, iteration).await,
        _ => {
            let client = reqwest::Client::builder()
                .user_agent(user_agent)
                .build()
                .unwrap();
            connect_http(&client, &link.url, iteration).await;
            do_sleep(link, iteration).await;
        }
    }
}

async fn fetch_link(link: &Link, iteration: u64, user_agent: &str) {
    match link.session_type {
        SessionType::Short => {
            if iteration > 1 {
                do_sleep(link, iteration).await;
            } else {
                info!("#{} {}", iteration, link.url); // 1er hit, pas de sleep
            }
            fetch_short(link, iteration, user_agent).await;
        }
        SessionType::Long => {
            if iteration == 1 {
                info!("#{} {}", iteration, link.url); // 1er hit, pas de sleep
            }
            fetch_long(link, iteration, user_agent).await;
        }
    }
}

// ─── Runners ─────────────────────────────────────────────────────────────────

pub async fn run_alternate(links: &[Link], max_iteration: i64, user_agent: &str) {
    let mut iteration = 0u64;
    let mut index = 0;

    loop {
        let link = &links[index % links.len()];
        iteration += 1;

        fetch_link(link, iteration, user_agent).await;

        if max_iteration >= 0 && iteration >= max_iteration as u64 {
            info!("# Max itérations atteintes: {}, arrêt.", iteration);
            break;
        }
        index += 1;
    }
}

pub async fn run_parallel(links: &[Link], max_iteration: i64, user_agent: &str) {
    let user_agent = user_agent.to_string();
    let mut handles = vec![];

    for link in links {
        let link = link.clone();
        let ua = user_agent.clone();

        let handle = tokio::spawn(async move {
            let mut iteration = 0u64;
            loop {
                iteration += 1;
                fetch_link(&link, iteration, &ua).await;

                if max_iteration >= 0 && iteration >= max_iteration as u64 {
                    info!("# Max itérations atteintes: {}, arrêt. ({})", iteration, link.url);
                    break;
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}