use fern::Dispatch;

pub fn setup_logger() -> Result<(), Box<dyn std::error::Error>> {
    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(fern::log_file("stiv.log")?)
        .apply()?;

    Ok(())
}
