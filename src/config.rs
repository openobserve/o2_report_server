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

use chromiumoxide::{
    browser::BrowserConfig,
    detection::{default_executable, DetectionOptions},
    fetcher::{BrowserFetcher, BrowserFetcherOptions},
    handler::viewport::Viewport,
};
use dotenv_config::EnvConfig;
use dotenvy::dotenv;
use lettre::{
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
    AsyncSmtpTransport, Tokio1Executor,
};
use once_cell::sync::Lazy;
use crate::ReportAttachmentDimensions;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub static CONFIG: Lazy<Config> = Lazy::new(init);

static CHROME_LAUNCHER_OPTIONS: tokio::sync::OnceCell<BrowserConfig> =
    tokio::sync::OnceCell::const_new();

#[derive(EnvConfig)]
pub struct Config {
    pub auth: Auth,
    pub http: Http,
    pub grpc: Grpc,
    pub common: Common,
    // pub limit: Limit,
    pub smtp: Smtp,
    pub chrome: Chrome,
    pub tokio_console: TokioConsole,
}

#[derive(EnvConfig)]
pub struct TokioConsole {
    #[env_config(name = "ZO_TOKIO_CONSOLE_SERVER_ADDR", default = "0.0.0.0")]
    pub tokio_console_server_addr: String,
    #[env_config(name = "ZO_TOKIO_CONSOLE_SERVER_PORT", default = 6699)]
    pub tokio_console_server_port: u16,
    #[env_config(name = "ZO_TOKIO_CONSOLE_RETENTION", default = 60)]
    pub tokio_console_retention: u64,
}

#[derive(EnvConfig)]
pub struct Chrome {
    #[env_config(name = "ZO_CHROME_PATH", default = "")]
    pub chrome_path: String,
    #[env_config(name = "ZO_CHROME_CHECK_DEFAULT_PATH", default = true)]
    pub chrome_check_default: bool,
    #[env_config(name = "ZO_CHROME_DOWNLOAD_PATH", default = "./data/download")]
    pub chrome_download_path: String,
    #[env_config(name = "ZO_CHROME_NO_SANDBOX", default = false)]
    pub chrome_no_sandbox: bool,
    #[env_config(name = "ZO_CHROME_WITH_HEAD", default = false)]
    pub chrome_with_head: bool,
    #[env_config(name = "ZO_CHROME_SLEEP_SECS", default = 20)]
    pub chrome_sleep_secs: u16,
    #[env_config(name = "ZO_CHROME_WINDOW_WIDTH", default = 1370)]
    pub chrome_window_width: u32,
    #[env_config(name = "ZO_CHROME_WINDOW_HEIGHT", default = 730)]
    pub chrome_window_height: u32,
    #[env_config(name = "ZO_CHROME_ADDITIONAL_ARGS", default = "")]
    pub chrome_additional_args: String,
    #[env_config(name = "ZO_CHROME_DISABLE_DEFAULT_ARGS", default = false)]
    pub chrome_disable_default_args: bool,
}

#[derive(EnvConfig)]
pub struct Smtp {
    #[env_config(name = "ZO_SMTP_HOST", default = "localhost")]
    pub smtp_host: String,
    #[env_config(name = "ZO_SMTP_PORT", default = 25)]
    pub smtp_port: u16,
    #[env_config(name = "ZO_SMTP_USER_NAME", default = "")]
    pub smtp_username: String,
    #[env_config(name = "ZO_SMTP_PASSWORD", default = "")]
    pub smtp_password: String,
    #[env_config(name = "ZO_SMTP_REPLY_TO", default = "")]
    pub smtp_reply_to: String,
    #[env_config(name = "ZO_SMTP_FROM_EMAIL", default = "")]
    pub smtp_from_email: String,
    #[env_config(name = "ZO_SMTP_ENCRYPTION", default = "")]
    pub smtp_encryption: String,
}

#[derive(EnvConfig)]
pub struct Auth {
    #[env_config(name = "ZO_REPORT_USER_EMAIL", default = "")]
    pub user_email: String,
    #[env_config(name = "ZO_REPORT_USER_PASSWORD", default = "")]
    pub user_password: String,
}

#[derive(EnvConfig)]
pub struct Http {
    #[env_config(name = "ZO_HTTP_PORT", default = 5090)]
    pub port: u16,
    #[env_config(name = "ZO_HTTP_ADDR", default = "127.0.0.1")]
    pub addr: String,
    #[env_config(name = "ZO_HTTP_IPV6_ENABLED", default = false)]
    pub ipv6_enabled: bool,
}

#[derive(EnvConfig)]
pub struct Grpc {
    #[env_config(name = "ZO_GRPC_PORT", default = 5081)]
    pub port: u16,
    #[env_config(name = "ZO_GRPC_ADDR", default = "")]
    pub addr: String,
    #[env_config(name = "ZO_INTERNAL_GRPC_TOKEN", default = "")]
    pub internal_grpc_token: String,
    #[env_config(
        name = "ZO_GRPC_MAX_MESSAGE_SIZE",
        default = 16,
        help = "Max grpc message size in MB, default is 16 MB"
    )]
    pub max_message_size: usize,
}

#[derive(EnvConfig)]
pub struct Common {
    #[env_config(name = "ZO_APP_NAME", default = "openobserve_report_generator")]
    pub app_name: String,
    #[env_config(name = "ZO_O2_APP_URL", default = "http://localhost:5080/web")]
    pub o2_web_uri: String,
    #[env_config(name = "ZO_LOCAL_MODE", default = true)]
    pub local_mode: bool,
}

pub fn init() -> Config {
    dotenv().ok();
    Config::init().unwrap()
}

pub async fn get_chrome_launch_options(
    report_attachment_dimensions: ReportAttachmentDimensions,
) -> &'static BrowserConfig {
    CHROME_LAUNCHER_OPTIONS
        .get_or_init(|| init_chrome_launch_options(report_attachment_dimensions))
        .await
}

async fn init_chrome_launch_options(
    report_attachment_dimensions: ReportAttachmentDimensions,
) -> BrowserConfig {
    let mut browser_config = BrowserConfig::builder()
        .window_size(
            report_attachment_dimensions.width,
            report_attachment_dimensions.height,
        )
        .viewport(Viewport {
            width: report_attachment_dimensions.width,
            height: report_attachment_dimensions.height,
            device_scale_factor: Some(1.0),
            ..Viewport::default()
        });

    if CONFIG.chrome.chrome_with_head {
        browser_config = browser_config.with_head();
    }

    if CONFIG.chrome.chrome_no_sandbox {
        browser_config = browser_config.no_sandbox();
    }

    if CONFIG.chrome.chrome_disable_default_args {
        browser_config = browser_config.disable_default_args();
    }

    if !CONFIG.chrome.chrome_additional_args.is_empty() {
        browser_config = browser_config.args(CONFIG.chrome.chrome_additional_args.split(","));
    }

    if !CONFIG.chrome.chrome_path.is_empty() {
        browser_config = browser_config.chrome_executable(CONFIG.chrome.chrome_path.as_str());
    } else {
        let mut should_download = false;

        if !CONFIG.chrome.chrome_check_default {
            should_download = true;
        } else {
            // Check if chrome is available on default paths
            // 1. Check the CHROME env
            // 2. Check usual chrome file names in user path
            // 3. (Windows) Registry
            // 4. (Windows & MacOS) Usual installations paths
            if let Ok(exec_path) = default_executable(DetectionOptions::default()) {
                browser_config = browser_config.chrome_executable(exec_path);
            } else {
                should_download = true;
            }
        }
        if should_download {
            // Download known good chrome version
            let download_path = &CONFIG.chrome.chrome_download_path;
            log::info!("fetching chrome at: {download_path}");
            tokio::fs::create_dir_all(download_path).await.unwrap();
            let fetcher = BrowserFetcher::new(
                BrowserFetcherOptions::builder()
                    .with_path(download_path)
                    .build()
                    .unwrap(),
            );

            // Fetches the browser revision, either locally if it was previously
            // installed or remotely. Returns error when the download/installation
            // fails. Since it doesn't retry on network errors during download,
            // if the installation fails, it might leave the cache in a bad state
            // and it is advised to wipe it.
            // Note: Does not work on LinuxArm platforms.
            let info = fetcher
                .fetch()
                .await
                .expect("chrome could not be downloaded");
            log::info!(
                "chrome fetched at path {:#?}",
                info.executable_path.as_path()
            );
            browser_config = browser_config.chrome_executable(info.executable_path);
        }
    }
    browser_config.build().unwrap()
}

pub static SMTP_CLIENT: Lazy<AsyncSmtpTransport<Tokio1Executor>> = Lazy::new(|| {
    let tls_parameters = TlsParameters::new(CONFIG.smtp.smtp_host.clone()).unwrap();
    let mut transport_builder =
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&CONFIG.smtp.smtp_host)
            .port(CONFIG.smtp.smtp_port);

    let option = &CONFIG.smtp.smtp_encryption;
    transport_builder = if option == "starttls" {
        transport_builder.tls(Tls::Required(tls_parameters))
    } else if option == "ssltls" {
        transport_builder.tls(Tls::Wrapper(tls_parameters))
    } else {
        transport_builder
    };

    if !CONFIG.smtp.smtp_username.is_empty() && !CONFIG.smtp.smtp_password.is_empty() {
        transport_builder = transport_builder.credentials(Credentials::new(
            CONFIG.smtp.smtp_username.clone(),
            CONFIG.smtp.smtp_password.clone(),
        ));
    }
    transport_builder.build()
});

