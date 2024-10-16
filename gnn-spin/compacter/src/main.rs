mod schema;

fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new().init()?;
    log::set_max_level(log::LevelFilter::Debug);

    let source_code_dir = std::path::PathBuf::from(
        std::env::args().nth(1).expect("no source code directory provided")
    );
    let graph_dir = std::path::PathBuf::from(
        std::env::args().nth(2).expect("no graph directory provided")
    );
    Ok(())
}
