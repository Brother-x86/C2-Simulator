use clap::ValueEnum;

#[derive(Debug, Clone, ValueEnum)]
pub enum Mode {
    Parallel,
    Alternate,
}


#[derive(Debug, Clone, ValueEnum, PartialEq)]
pub enum SessionType {
    Short,
    Long,
}
