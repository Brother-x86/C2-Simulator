#[derive(Debug)]
pub struct Link {
    pub url: String,
    pub sleep_str: String,
    pub sleep: u64,
    pub jitt: u32,
}

use log::debug;
use log::info;
use log::warn;

pub async fn run_alternate(links: &[Link],max_iteration:i64) {
    let client = reqwest::Client::new();
    let mut iteration = 0u64;
    let mut index = 0;

    loop {
        let link = &links[index % links.len()];
        iteration += 1;

        info!("[{}] #{} url={} sleep={} jitt={}", link.url, iteration, link.url, link.sleep_str, link.jitt);
        match client.get(&link.url).send().await {
            Ok(resp) => info!("[{}] #{} → {}", link.url, iteration, resp.status()),
            Err(e)   => warn!("[{}] #{} → flux KO: {}", link.url, iteration, e),
        }

        if max_iteration >= 0 && iteration >= max_iteration as u64 {
            info!("# Max itérations atteintes:{} , arrêt.", iteration);
            break;
        }
        
        if link.sleep > 0 {
            info!("[{}] sleep {}...", link.url, link.sleep_str);
            tokio::time::sleep(tokio::time::Duration::from_secs(link.sleep)).await;
        }

        index += 1;
    }
}
