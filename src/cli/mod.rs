// Copyright 2025 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

mod utils;

pub async fn cli() -> Result<bool, anyhow::Error> {
    let app = clap::Command::new("report-generator")
        .version(crate::config::VERSION)
        .about(clap::crate_description!())
        .subcommands(&[clap::Command::new("init-dir")
            .about("init report-generator data dir")
            .arg(
                clap::Arg::new("path")
                    .short('p')
                    .long("path")
                    .help("init this path as data root dir"),
            )])
        .get_matches();

    if app.subcommand().is_none() {
        return Ok(false);
    }

    let (name, command) = app.subcommand().unwrap();
    if name == "init-dir" {
        match command.get_one::<String>("path") {
            Some(path) => {
                utils::set_permission(path, 0o777)?;
                println!("init dir {} successfully", path);
            }
            None => {
                return Err(anyhow::anyhow!("please set data path"));
            }
        }
        return Ok(true);
    }

    println!("command {name} execute successfully");
    Ok(true)
}
