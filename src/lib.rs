mod build;
pub mod channel;
mod ci;
mod env;
pub mod err;
mod git;

use build::*;
use env::*;

use git::*;

use crate::ci::CIType;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env as std_env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use chrono::Local;
pub use err::{SdResult, ShadowError};

const SHADOW_RS: &str = "shadow.rs";

/// record compiled project much information.
/// version info,dependence info.Like shadow,if compiled,never change.forever follow your project.
/// generated rust const by exec:`cargo build`
///
///```rust
/// pub const RUST_VERSION :&str = "rustc 1.45.0 (5c1f21c3b 2020-07-13)";
/// pub const BUILD_RUST_CHANNEL :&str = "debug";
/// pub const COMMIT_AUTHOR :&str = "baoyachi";
/// pub const BUILD_TIME :&str = "2020-08-16 13:48:52";
/// pub const COMMIT_DATE :&str = "2020-08-16 13:12:52";
/// pub const COMMIT_EMAIL :&str = "xxx@gmail.com";
/// pub const PROJECT_NAME :&str = "shadow-rs";
/// pub const RUST_CHANNEL :&str = "stable-x86_64-apple-darwin (default)";
/// pub const BRANCH :&str = "master";
/// pub const CARGO_LOCK :&str = "";
/// pub const CARGO_VERSION :&str = "cargo 1.45.0 (744bd1fbb 2020-06-15)";
/// pub const BUILD_OS :&str = "macos-x86_64";
/// pub const COMMIT_HASH :&str = "386741540d73c194a3028b96b92fdeb53ca2788a";
/// pub const PKG_VERSION :&str = "0.3.13";
///
/// ```
#[derive(Debug)]
pub struct Shadow {
    f: File,
    map: HashMap<ShadowConst, RefCell<ConstVal>>,
    std_env: HashMap<String, String>,
}

impl Shadow {
    fn get_env() -> HashMap<String, String> {
        let mut env_map = HashMap::new();
        for (k, v) in std_env::vars() {
            env_map.insert(k, v);
        }
        env_map
    }

    /// try get current ci env
    fn try_ci(&self) -> CIType {
        if let Some(c) = self.std_env.get("GITLAB_CI") {
            if c == "true" {
                return CIType::Gitlab;
            }
        }

        if let Some(c) = self.std_env.get("GITHUB_ACTIONS") {
            if c == "true" {
                return CIType::Github;
            }
        }

        //TODO completed [travis,jenkins] env

        CIType::None
    }

    pub fn build(src_path: String, out_path: String) -> SdResult<()> {
        let out = {
            let path = Path::new(out_path.as_str());
            if !out_path.ends_with('/') {
                path.join(format!("{}/{}", out_path, SHADOW_RS))
            } else {
                path.join(SHADOW_RS)
            }
        };

        let mut shadow = Shadow {
            f: File::create(out)?,
            map: Default::default(),
            std_env: Default::default(),
        };
        shadow.std_env = Self::get_env();

        let ci_type = shadow.try_ci();
        let src_path = Path::new(src_path.as_str());

        let mut map = new_git(&src_path, ci_type, &shadow.std_env);
        for (k, v) in new_project(&shadow.std_env) {
            map.insert(k, v);
        }
        for (k, v) in new_system_env(&shadow.std_env) {
            map.insert(k, v);
        }
        shadow.map = map;

        shadow.gen_const()?;
        println!("shadow build success");
        Ok(())
    }

    fn gen_const(&mut self) -> SdResult<()> {
        self.write_header()?;
        for (k, v) in self.map.clone() {
            self.write_const(k, v)?;
        }
        Ok(())
    }

    fn write_header(&self) -> SdResult<()> {
        let desc = format!(
            r#"/// Code generated by shadow-rs generator. DO NOT EDIT.
/// Author by https://www.github.com/baoyachi
/// create time by:{}"#,
            Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
        );
        writeln!(&self.f, "{}\n\n", desc)?;
        Ok(())
    }

    fn write_const(&mut self, shadow_const: ShadowConst, val: RefCell<ConstVal>) -> SdResult<()> {
        let val = val.into_inner();
        let desc = format!("/// {}", val.desc);

        let (t, v) = match val.t {
            ConstType::OptStr => (ConstType::Str.to_string(), "".into()),
            ConstType::Str => (ConstType::Str.to_string(), val.v),
        };

        let define = format!(
            "pub const {} :{} = \"{}\";",
            shadow_const.to_ascii_uppercase(),
            t,
            v
        );
        writeln!(&self.f, "{}", desc)?;
        writeln!(&self.f, "{}\n", define)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() -> SdResult<()> {
        Shadow::build("./".into(), "./".into())?;
        Ok(())
    }
}
