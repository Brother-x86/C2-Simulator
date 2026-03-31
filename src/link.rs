#[derive(Debug, Clone)]
pub struct Link {
    pub url: String,
    pub sleep_str: String,
    pub sleep: u64,
    pub jitt: u32,
}

use log::info;
use log::warn;
use log::debug;

async fn fetch_link(client: &reqwest::Client, link: &Link, iteration: u64) {
    // Sleep AVANT la requête, sauf sur le tout premier hit
    if iteration > 1 && (link.sleep > 0 || link.jitt > 0) {
        let jitt = if link.jitt > 0 { rand::random::<u32>() % link.jitt } else { 0 };
        let total = link.sleep + jitt as u64;
        info!("#{} {} → sleep {}+{}s = {}s", iteration, link.url, link.sleep_str, jitt, total);
        tokio::time::sleep(tokio::time::Duration::from_secs(total)).await;
    }

    match link.url.split_once("://").map(|(s, _)| s) {
        Some("ws") | Some("wss") => connect_ws(&link.url, iteration).await,
        _                        => connect_http(&client, &link.url, iteration).await,
    };
}

pub async fn run_alternate(links: &[Link], max_iteration: i64) {
    let client = reqwest::Client::new();
    let mut iteration = 0u64;
    let mut index = 0;

    loop {
        let link = &links[index % links.len()];
        iteration += 1;

        fetch_link(&client, link, iteration).await;

        if max_iteration >= 0 && iteration >= max_iteration as u64 {
            info!("# Max itérations atteintes: {}, arrêt.", iteration);
            break;
        }

        index += 1;
    }
}

pub async fn run_parallel(links: &[Link], max_iteration: i64) {
    let client = reqwest::Client::new();
    let mut handles = vec![];

    for link in links {
        let client = client.clone();
        let link = link.clone(); // Link doit dériver Clone

        let handle = tokio::spawn(async move {
            let mut iteration = 0u64;

            loop {
                iteration += 1;

                fetch_link(&client, &link, iteration).await;

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
async fn connect_http(client: &reqwest::Client, url: &str, iteration: u64) -> bool {
    match client.get(url).send().await {
        Ok(resp) => { debug!("#{} {} → {}", iteration, url, resp.status()); true  }
        Err(e)   => { warn!("#{} {} → KO: {}", iteration, url, e);          false }
    }
}

async fn connect_ws(url: &str, iteration: u64) -> bool {
    match tokio_tungstenite::connect_async(url).await {
        Ok(_)  => { debug!("#{} {} → WS OK", iteration, url); true  }
        Err(e) => { warn!("#{} {} → KO: {}", iteration, url, e); false }
    }
}