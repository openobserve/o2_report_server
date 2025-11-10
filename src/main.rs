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

use actix_web::{dev::ServerHandle, middleware, web, App, HttpServer};
use o2_report_generator::{
    cli,
    config::{self, CONFIG},
    router::{healthz, send_report},
    ReportAttachmentDimensions,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    // cli mode
    if cli::cli().await? {
        return Ok(());
    }

    // Locate or fetch chromium
    _ = config::get_chrome_launch_options(ReportAttachmentDimensions::default()).await;

    log::info!("starting o2 chrome server");

    if CONFIG.auth.user_email.is_empty() || CONFIG.auth.user_password.is_empty() {
        panic!("Report User email and password must be specified");
    }

    // Log configured Chrome PDF parameters
    log::info!("Chrome PDF Configuration:");
    log::info!("  pdf_landscape: {}", CONFIG.chrome.pdf_landscape);

    let display_opt = |s: &str| {
        if s.is_empty() {
            "None (browser default)".to_string()
        } else {
            format!("Some({})", s)
        }
    };

    log::info!(
        "  pdf_display_header_footer: {}",
        display_opt(&CONFIG.chrome.pdf_display_header_footer)
    );
    log::info!(
        "  pdf_print_background: {}",
        display_opt(&CONFIG.chrome.pdf_print_background)
    );
    log::info!("  pdf_scale: {}", display_opt(&CONFIG.chrome.pdf_scale));
    log::info!(
        "  pdf_paper_width: {}",
        display_opt(&CONFIG.chrome.pdf_paper_width)
    );
    log::info!(
        "  pdf_paper_height: {}",
        display_opt(&CONFIG.chrome.pdf_paper_height)
    );
    log::info!(
        "  pdf_margin_top: {}",
        display_opt(&CONFIG.chrome.pdf_margin_top)
    );
    log::info!(
        "  pdf_margin_bottom: {}",
        display_opt(&CONFIG.chrome.pdf_margin_bottom)
    );
    log::info!(
        "  pdf_margin_left: {}",
        display_opt(&CONFIG.chrome.pdf_margin_left)
    );
    log::info!(
        "  pdf_margin_right: {}",
        display_opt(&CONFIG.chrome.pdf_margin_right)
    );
    log::info!(
        "  pdf_prefer_css_page_size: {}",
        display_opt(&CONFIG.chrome.pdf_prefer_css_page_size)
    );
    log::info!(
        "  pdf_generate_tagged_pdf: {}",
        display_opt(&CONFIG.chrome.pdf_generate_tagged_pdf)
    );
    log::info!(
        "  pdf_generate_document_outline: {}",
        display_opt(&CONFIG.chrome.pdf_generate_document_outline)
    );

    let haddr: SocketAddr = if CONFIG.http.ipv6_enabled {
        format!("[::]:{}", CONFIG.http.port).parse()?
    } else {
        let ip = if !CONFIG.http.addr.is_empty() {
            CONFIG.http.addr.clone()
        } else {
            "0.0.0.0".to_string()
        };
        format!("{}:{}", ip, CONFIG.http.port).parse()?
    };
    log::info!("starting HTTP server at: {}", haddr);
    let server = HttpServer::new(move || {
        App::new()
            .service(web::scope("/api").service(send_report).service(healthz))
            .wrap(middleware::Logger::new(
                r#"%a "%r" %s %b "%{Content-Length}i" "%{Referer}i" "%{User-Agent}i" %T"#,
            ))
    })
    .bind(haddr)?
    .run();

    let handle = server.handle();
    tokio::task::spawn(async move {
        graceful_shutdown(handle).await;
    });
    server.await?;
    log::info!("HTTP server stopped");
    Ok(())
}

async fn graceful_shutdown(handle: ServerHandle) {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigquit = signal(SignalKind::quit()).unwrap();
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = sigquit.recv() =>  log::info!("SIGQUIT received"),
            _ = sigterm.recv() =>  log::info!("SIGTERM received"),
            _ = sigint.recv() =>   log::info!("SIGINT received"),
        }
    }

    #[cfg(not(unix))]
    {
        use tokio::signal::windows::*;

        let mut sigbreak = ctrl_break().unwrap();
        let mut sigint = ctrl_c().unwrap();
        let mut sigquit = ctrl_close().unwrap();
        let mut sigterm = ctrl_shutdown().unwrap();

        tokio::select! {
            _ = sigbreak.recv() =>  log::info!("ctrl-break received"),
            _ = sigquit.recv() =>  log::info!("ctrl-c received"),
            _ = sigterm.recv() =>  log::info!("ctrl-close received"),
            _ = sigint.recv() =>   log::info!("ctrl-shutdown received"),
        }
    }
    // tokio::signal::ctrl_c().await.unwrap();
    // println!("ctrl-c received!");

    handle.stop(true).await;
}
