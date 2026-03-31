use crate::SessionType;
use futures_util::{SinkExt, StreamExt};
use log::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct Link {
    pub url: String,
    pub sleep_str: String,
    pub sleep: u64,
    pub jitt: u32,
    pub session_type: SessionType,
}

async fn do_sleep(link: &Link, iteration: u64) {
    if link.sleep > 0 || link.jitt > 0 {
        let jitt = if link.jitt > 0 {
            rand::random::<u32>() % link.jitt
        } else {
            0
        };
        let total = link.sleep + jitt as u64;
        info!(
            "#{} {} → sleep {}+{}s = {}s",
            iteration, link.url, link.sleep_str, jitt, total
        );
        tokio::time::sleep(tokio::time::Duration::from_secs(total)).await;
    }
}

async fn connect_http(client: &reqwest::Client, url: &str, iteration: u64) -> bool {
    match client.get(url).send().await {
        Ok(resp) => {
            debug!("#{} {} → {}", iteration, url, resp.status());
            true
        }
        Err(e) => {
            warn!("#{} {} → KO: {}", iteration, url, e);
            false
        }
    }
}

async fn connect_ws(url: &str, iteration: u64) -> bool {
    match tokio_tungstenite::connect_async(url).await {
        Ok(_) => {
            debug!("#{} {} → WS OK", iteration, url);
            true
        }
        Err(e) => {
            warn!("#{} {} → KO: {}", iteration, url, e);
            false
        }
    }
}

async fn fetch_long_ws(link: &Link, iteration: u64) {
    // Juste une connexion + un ping + sleep, puis on retourne
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

            // Sleep avec la connexion encore ouverte
            do_sleep(link, iteration).await;
            // ws droppé ici → connexion fermée proprement
        }
    }
}

async fn fetch_short(link: &Link, iteration: u64) {
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(0) // pas de keep-alive
        .build()
        .unwrap();

    match link.url.split_once("://").map(|(s, _)| s) {
        Some("ws") | Some("wss") => connect_ws(&link.url, iteration).await,
        _ => connect_http(&client, &link.url, iteration).await,
    };
}

async fn fetch_long(link: &Link, iteration: u64) {
    match link.url.split_once("://").map(|(s, _)| s) {
        Some("ws") | Some("wss") => fetch_long_ws(link, iteration).await,
        _ => {
            let client = reqwest::Client::new(); // keep-alive actif
            connect_http(&client, &link.url, iteration).await;
            do_sleep(link, iteration).await;
            // client droppé ici → connexion fermée
        }
    }
}

async fn fetch_link(link: &Link, iteration: u64) {
    match link.session_type {
        SessionType::Short => {
            if iteration > 1 {
                do_sleep(link, iteration).await;
            }
            fetch_short(link, iteration).await;
        }
        SessionType::Long => {
            fetch_long(link, iteration).await;
        }
    }
}

pub async fn run_alternate(links: &[Link], max_iteration: i64) {
    let mut iteration = 0u64;
    let mut index = 0;

    loop {
        let link = &links[index % links.len()];
        iteration += 1;

        fetch_link(link, iteration).await;

        if max_iteration >= 0 && iteration >= max_iteration as u64 {
            info!("# Max itérations atteintes: {}, arrêt.", iteration);
            break;
        }

        index += 1;
    }
}

pub async fn run_parallel(links: &[Link], max_iteration: i64) {
    let mut handles = vec![];

    for link in links {
        let link = link.clone();

        let handle = tokio::spawn(async move {
            let mut iteration = 0u64;

            loop {
                iteration += 1;

                fetch_link(&link, iteration).await;

                if max_iteration >= 0 && iteration >= max_iteration as u64 {
                    info!(
                        "# Max itérations atteintes: {}, arrêt. ({})",
                        iteration, link.url
                    );
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
