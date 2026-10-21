//! Binary entry point — everything lives in the [`server`] lib.

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--reset-db".to_string()) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build runtime");

        dotenvy::dotenv().ok();
        let config = server::infra::config::AppConfig::from_env().expect("invalid config");

        rt.block_on(async {
            match server::infra::db::reset(&config).await {
                Ok(()) => println!("Database reset complete"),
                Err(e) => {
                    eprintln!("Reset failed: {e}");
                    std::process::exit(1);
                }
            }
        });
        return;
    }

    server::run();
}
