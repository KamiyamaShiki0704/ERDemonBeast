//! Configuration validation and behavior bridge command export.
use er_demon_beast::forms::Config;
fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: check_forms <config.toml> [bridge-file]")?;
    let config = Config::parse(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)?;
    if let Some(path) = args.next() {
        std::fs::write(
            path,
            format!(
                "1-2-3\nenter\n{}",
                config.forms[config.default_index()].bridge_options()
            ),
        )
        .map_err(|e| e.to_string())?;
    }
    println!(
        "PASS: {} forms; default={}",
        config.forms.len(),
        config.default_form
    );
    Ok(())
}
