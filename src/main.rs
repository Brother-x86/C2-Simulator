use clap::{Parser, ValueEnum};

mod link;
use link::Link;


#[derive(Parser, Debug)]
#[command(name = "C2-Simulator")]
#[command(about = "Un programme qui accepte va simuler des flux comme un C2")]

struct Args {
    /// URL(s) à traiter (répétable : -u url1 -u url2)
    #[arg(short = 'u', long = "url", required = true, value_name = "URL")]
    urls: Vec<String>,

    /// Temps de sleep en secondes (répétable) , default unit(secondes), sinon : s=secondes,m=minutes,h=heures,j=hours, example -s 5m , -s 40s
    #[arg(short = 's', long = "sleep", required = true, value_name = "SLEEP")]
    sleep: Vec<String>,

    /// Nombre de hits (répétable)
    #[arg(short = 'j', long = "jitt", required = true, value_name = "JITT")]
    jitt: Vec<u32>,


    /// User-Agent
    #[arg(short = 'a', long = "user-agent", value_name = "USER-AGENT",default_value = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:132.0) Gecko/20100101 Firefox/132.0")]
    user_agent: String,

    /// Debug, rajoute les logs de type debug
    #[arg(short = 'd', long = "debug", value_name = "DEBUG",default_value = "false")]
    debug: bool,

    /// Mode d'exécution
    #[arg(short = 'm', long = "mode", default_value = "parallel")]
    mode: Mode,

}

#[derive(Debug, Clone, ValueEnum)]
enum Mode {
    Parallel,
    Alternate,
}

extern crate env_logger;
use log::debug;
use log::info;

fn main() {
    let args = Args::parse();

    if args.debug {
        env_logger::builder().filter_level(log::LevelFilter::Debug).init();
    }else{
        env_logger::builder().filter_level(log::LevelFilter::Info).init();
    }

        
    let n = args.urls.len();

    // 1. Normalise les Vec à la taille de urls
    let sleep_normalized = normalize_vec(args.sleep, n);
    let jitt = normalize_vec(args.jitt, n);
    
    // 2. Convertit les strings en secondes APRÈS normalisation
    let sleep_secs: Vec<u64> = sleep_normalized.iter()
        .map(|s| parse_duration(s).unwrap_or_else(|e| {
            eprintln!("Erreur d'unité pour sleep, only s,m,h,j are supported : '{}' - {} , ", s, e);
            std::process::exit(1);
        }))
        .collect();
    
    info!("mode={:?}",args.mode);
    // 3. Zippé proprement
    /*for ((url, s), j) in args.urls.iter().zip(sleep_secs.iter()).zip(jitt.iter()) {
        info!("url={} sleep={}s jitt={}", url, s, j);
    }*/

    let links: Vec<Link> = args.urls.iter()
    .zip(sleep_normalized.iter())
    .zip(sleep_secs.iter().zip(jitt.iter()))
    .map(|((url, sleep_str), (sleep, jitt))| Link {
        url: url.clone(),
        sleep_str: sleep_str.clone(),
        sleep: *sleep,
        jitt: *jitt,
    })
    .collect();

for link in &links {
    info!("url={} sleep={} ({}s) jitt={}", link.url, link.sleep_str, link.sleep, link.jitt);
}


}


fn normalize_vec<T: Clone>(vec: Vec<T>, target_len: usize) -> Vec<T> {
    let last = vec.last().unwrap().clone(); // required = true donc jamais vide
    let mut result = vec;
    result.truncate(target_len);                          // réduit si trop grand
    while result.len() < target_len {
        result.push(last.clone());                        // complète avec la dernière valeur
    }
    result
}

fn parse_duration(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if let Some(val) = s.strip_suffix('s') {
        val.parse::<u64>().map_err(|_| format!("Valeur invalide : {}", s))
    } else if let Some(val) = s.strip_suffix('m') {
        val.parse::<u64>().map(|v| v * 60).map_err(|_| format!("Valeur invalide : {}", s))
    } else if let Some(val) = s.strip_suffix('h') {
        val.parse::<u64>().map(|v| v * 3600).map_err(|_| format!("Valeur invalide : {}", s))
    } else if let Some(val) = s.strip_suffix('j') {
        val.parse::<u64>().map(|v| v * 86400).map_err(|_| format!("Valeur invalide : {}", s))
    } else {
        // Pas de suffixe → secondes par défaut
        s.parse::<u64>().map_err(|_| format!("Format invalide : '{}' (ex: 5s, 10m, 2h, 1j)", s))
    }
}