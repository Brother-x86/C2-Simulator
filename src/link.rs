#[derive(Debug)]
pub struct Link {
    pub url: String,
    pub sleep_str: String,
    pub sleep: u64,
    pub jitt: u32,
}