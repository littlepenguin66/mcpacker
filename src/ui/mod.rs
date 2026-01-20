pub use console::{style, Emoji};

pub static PACKAGE: Emoji<'_, '_> = Emoji("", "");
pub static LOOKING_GLASS: Emoji<'_, '_> = Emoji("", "");
pub static SPARKLE: Emoji<'_, '_> = Emoji("", "");

pub fn print_logo() {
    println!(
        "{}",
        style(
            r#"
███╗   ███╗ ██████╗██████╗  █████╗  ██████╗██╗  ██╗███████╗██████╗ 
████╗ ████║██╔════╝██╔══██╗██╔══██╗██╔════╝██║ ██╔╝██╔════╝██╔══██╗
██╔████╔██║██║     ██████╔╝███████║██║     █████╔╝ █████╗  ██████╔╝
██║╚██╔╝██║██║     ██╔═══╝ ██╔══██║██║     ██╔═██╗ ██╔══╝  ██╔══██╗
██║ ╚═╝ ██║╚██████╗██║     ██║  ██║╚██████╗██║  ██╗███████╗██║  ██║
╚═╝     ╚═╝ ╚═════╝╚═╝     ╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝ 
"#
        )
        .bold()
        .cyan()
    );
}

pub fn print_step(msg: &str) {
    println!("{} {}", style("==>").bold().blue(), style(msg).bold());
}

pub fn print_header(msg: &str) {
    println!("{}", style(msg).bold().underlined());
    println!();
}

pub fn print_success(msg: &str) {
    println!("{} {}", style("SUCCESS:").green().bold(), msg);
}

pub fn print_info(label: &str, value: &str) {
    println!("  {}: {}", style(label).dim(), style(value).cyan());
}

pub fn print_warn(msg: &str) {
    println!("{} {}", style("WARNING:").yellow().bold(), msg);
}

#[allow(dead_code)]
pub fn print_error(msg: &str) {
    eprintln!("{} {}", style("ERROR:").red().bold(), msg);
}

pub fn format_key_value(key: &str, value: &str) -> String {
    format!("{}: {}", style(key).dim(), style(value).white())
}
