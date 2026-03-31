use clap::Parser;

mod link;
use crate::link::run_parallel;
use link::Link;
use link::run_alternate;
mod mode;
use mode::Mode;
use mode::SessionType;

extern crate env_logger;
use log::info;

#[derive(Parser, Debug)]
#[command(name = "C2-Simulator")]
#[command(about = "Simule des flux réseau comme un C2")]
//#[command(long_about = "Simule des flux réseau comme un C2.\n\nChaque -u peut être accompagné de son propre -s, -j et -t : les paramètres s'appliquent dans l'ordre aux URLs fournies.\nSi une URL n'a pas de valeur correspondante, elle hérite de la dernière valeur spécifiée — ou de la valeur par défaut si aucune n'a été fournie.")]

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
    #[arg(
        short = 'a',
        long = "user-agent",
        value_name = "USER-AGENT",
        default_value = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:132.0) Gecko/20100101 Firefox/132.0"
    )]
    user_agent: String,

    /// Debug, rajoute les logs de type debug
    #[arg(
        short = 'd',
        long = "debug",
        value_name = "DEBUG",
        default_value = "false"
    )]
    debug: bool,

    /// Mode d'exécution
    #[arg(short = 'm', long = "mode", default_value = "alternate")]
    mode: Mode,

    /// Session Type
    #[arg(short = 't', long = "type", default_value = "short", name = "type")]
    session_type: Vec<SessionType>,

    /// Nombre d'itérations (-1 = infini)
    #[arg(short = 'i', long = "iteration", default_value = "-1")]
    iteration: i64,

    /// Dry run : affiche la configuration et quitte sans exécuter
    #[arg(short = 'r',long = "dry-run")]
    dry_run: bool,

    /// Awesome Banner
    #[arg(short = 'b',long = "banner")]
    banner: bool,

}

#[tokio::main]
async fn main() {
    // Vérif brute avant clap
    if std::env::args().any(|a| a == "--banner" || a == "-b") {
        banner().await;
    }

    // Clap parse normalement ensuite
    let args = Args::parse();

    if args.debug {
        env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::builder()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    let n = args.urls.len();

    // 1. Normalise les Vec à la taille de urls
    let sleep_normalized = normalize_vec(args.sleep, n);
    let jitt = normalize_vec(args.jitt, n);

    // 2. Convertit les strings en secondes APRÈS normalisation
    let sleep_secs: Vec<u64> = sleep_normalized
        .iter()
        .map(|s| {
            parse_duration(s).unwrap_or_else(|e| {
                eprintln!(
                    "Erreur d'unité pour sleep, only s,m,h,j are supported : '{}' - {} , ",
                    s, e
                );
                std::process::exit(1);
            })
        })
        .collect();

    use itertools::izip;

    let session_types = normalize_vec(args.session_type, n);

    let links: Vec<Link> = izip!(
        args.urls.iter(),
        sleep_normalized.iter(),
        sleep_secs.iter(),
        jitt.iter(),
        session_types.iter()
    )
    .map(|(url, sleep_str, sleep, jitt, session_type)| Link {
        url: url.clone(),
        sleep_str: sleep_str.clone(),
        sleep: *sleep,
        jitt: *jitt,
        session_type: session_type.clone(),
    })
    .collect();

    let max_len = links.iter().map(|l| l.url.len()).max().unwrap_or(0);

    if args.banner {
        banner().await;
    }

    info!("Links configured :");
    for link in &links {
        info!(
            "  - {:<width$} | sleep={:<1} | jitt={:<1} | {:?}",
            link.url,
            link.sleep_str,
            format!("{}s", link.jitt),
            link.session_type,
            width = max_len
        );
    }
    info!("RUN mode: {:?}", args.mode);

    if args.dry_run {
        info!("Dry run, STOP");
        return;
    }

    match args.mode {
        mode::Mode::Parallel => run_parallel(&links, args.iteration, &args.user_agent).await,
        mode::Mode::Alternate => run_alternate(&links, args.iteration, &args.user_agent).await,
    };
}

fn normalize_vec<T: Clone>(vec: Vec<T>, target_len: usize) -> Vec<T> {
    let last = vec.last().unwrap().clone(); // required = true donc jamais vide
    let mut result = vec;
    result.truncate(target_len); // réduit si trop grand
    while result.len() < target_len {
        result.push(last.clone()); // complète avec la dernière valeur
    }
    result
}

fn parse_duration(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if let Some(val) = s.strip_suffix('s') {
        val.parse::<u64>()
            .map_err(|_| format!("Valeur invalide : {}", s))
    } else if let Some(val) = s.strip_suffix('m') {
        val.parse::<u64>()
            .map(|v| v * 60)
            .map_err(|_| format!("Valeur invalide : {}", s))
    } else if let Some(val) = s.strip_suffix('h') {
        val.parse::<u64>()
            .map(|v| v * 3600)
            .map_err(|_| format!("Valeur invalide : {}", s))
    } else if let Some(val) = s.strip_suffix('j') {
        val.parse::<u64>()
            .map(|v| v * 86400)
            .map_err(|_| format!("Valeur invalide : {}", s))
    } else {
        // Pas de suffixe → secondes par défaut
        s.parse::<u64>()
            .map_err(|_| format!("Format invalide : '{}' (ex: 5s, 10m, 2h, 1j)", s))
    }
}

use tokio::io::AsyncWriteExt;
use tokio::time::Duration;

pub async fn banner() {
    let text = format!(
        r#"
 ██████╗██████╗     ███████╗██╗███╗   ███╗██╗   ██╗██╗      █████╗ ████████╗ ██████╗ ██████╗ 
██╔════╝╚════██╗    ██╔════╝██║████╗ ████║██║   ██║██║     ██╔══██╗╚══██╔══╝██╔═══██╗██╔══██╗
██║      █████╔╝    ███████╗██║██╔████╔██║██║   ██║██║     ███████║   ██║   ██║   ██║██████╔╝
██║     ██╔═══╝     ╚════██║██║██║╚██╔╝██║██║   ██║██║     ██╔══██║   ██║   ██║   ██║██╔══██╗
╚██████╗███████╗    ███████║██║██║ ╚═╝ ██║╚██████╔╝███████╗██║  ██║   ██║   ╚██████╔╝██║  ██║
 ╚═════╝╚══════╝    ╚══════╝╚═╝╚═╝     ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═╝   ╚═╝    ╚═════╝ ╚═╝  ╚═╝
                                                                                             "#
    );

    let mut stdout = tokio::io::stdout();
    for c in text.chars() {
        stdout.write_all(c.to_string().as_bytes()).await.unwrap();
        stdout.flush().await.unwrap();
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    println!();
}