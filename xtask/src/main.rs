//! See <https://github.com/matklad/cargo-xtask/>.
//! This binary is integrated into the `cargo` command line by using an alias in
//! `.cargo/config`.
mod flags;
mod tables;

use std::env;
use std::path::{Path, PathBuf};

use anyhow::Result;
use xshell::{cmd, Shell};

fn main() -> Result<()> {
    let sh = Shell::new()?;
    sh.change_dir(project_root());

    let flags = flags::Xtask::from_env()?;
    match flags.subcommand {
        flags::XtaskCmd::Help(_) => {
            println!("{}", flags::Xtask::HELP);
            Ok(())
        }
        flags::XtaskCmd::GenTables(cmd) => cmd.run(&sh),
    }
}

fn project_root() -> PathBuf {
    Path::new(
        &env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned()),
    )
    .ancestors()
    .nth(1)
    .unwrap()
    .to_path_buf()
}

pub fn reformat(sh: &Shell, text: String) -> String {
    // ensure_rustfmt();
    // let rustfmt_toml = sh.current_dir().join("rustfmt.toml"); --config-path {rustfmt_toml}
    let mut stdout = cmd!(sh, "rustfmt --config fn_single_line=true")
        .stdin(text)
        .read()
        .unwrap();
    if !stdout.ends_with('\n') {
        stdout.push('\n');
    }
    stdout
}
