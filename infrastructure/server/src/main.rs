//! Binary entry point — everything lives in the [`server`] lib.

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--reset".to_string()) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build runtime");

        dotenvy::dotenv().ok();
        let config = server::infra::config::AppConfig::from_env().expect("invalid config");

        rt.block_on(async {
            // One clean-start flag, DB first: the projects folder in the
            // config store relies on what the database holds, so the store
            // half only runs when the database reset succeeded.
            match server::infra::db::reset(&config).await {
                Ok(()) => {
                    println!("Database reset complete");
                    match server::infra::config_store_factory::reset(&config).await {
                        Ok(deleted) => println!("Config store reset complete ({deleted} project keys deleted)"),
                        Err(e) => {
                            eprintln!("Config store reset failed: {e}");
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Reset failed: {e}");
                    eprintln!("Config store NOT reset — fix the database first (tenant configs stay consistent)");
                    std::process::exit(1);
                }
            }
        });
        return;
    }

    server::run();
}
