xflags::xflags! {
    src "./src/flags.rs"

    /// Run custom build command.
    cmd xtask {
        default cmd help {
            /// Print help information.
            optional -h, --help
        }

        cmd gen-tables
        required unicode_version: String
        {}

    }
}
// generated start
// The following code is generated by `xflags` macro.
// Run `env UPDATE_XFLAGS=1 cargo build` to regenerate.
#[derive(Debug)]
pub struct Xtask {
    pub subcommand: XtaskCmd,
}

#[derive(Debug)]
pub enum XtaskCmd {
    Help(Help),
    GenTables(GenTables),
}

#[derive(Debug)]
pub struct Help {
    pub help: bool,
}

#[derive(Debug)]
pub struct GenTables {
    pub unicode_version: String,
}

impl Xtask {
    pub const HELP: &'static str = Self::HELP_;

    #[allow(dead_code)]
    pub fn from_env() -> xflags::Result<Self> {
        Self::from_env_()
    }

    #[allow(dead_code)]
    pub fn from_vec(args: Vec<std::ffi::OsString>) -> xflags::Result<Self> {
        Self::from_vec_(args)
    }
}
// generated end
