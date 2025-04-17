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

pub mod cli;
pub mod config;
pub mod router;

use chromiumoxide::{
    browser::Browser,
    cdp::browser_protocol::page::{CaptureScreenshotParamsBuilder, PrintToPdfParams},
    Page,
};
use config::{get_chrome_launch_options, CONFIG};
use futures::StreamExt;
use lettre::{
    message::{header::ContentType, MultiPart, SinglePart},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ReportType {
    PDF,
    Cache,
    PNG,
}

fn default_report_type() -> ReportType {
    ReportType::PDF
}

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq)]
pub enum EmailAttachmentType {
    Standard, // Sends the email as a traditional attachment the user can download and (pre)view
    Inline,   // Sends the attachment inline in the email body
}

fn default_attachment_type() -> EmailAttachmentType {
    EmailAttachmentType::Standard
}

/// Allow the user to override attachment size per report.
/// Some reports are better in wide aspect ratios, and some reports like 2 column tables are
/// better in tall aspect ratios.
/// If no dimensions are provided, the default will be used from chrome config env vars.
#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct ReportAttachmentDimensions {
    pub height: u32,
    pub width: u32,
}

impl Default for ReportAttachmentDimensions {
    fn default() -> Self {
        Self {
            height: CONFIG.chrome.chrome_window_width,
            width: CONFIG.chrome.chrome_window_height,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub from_email: String,
    pub reply_to: String,
    pub client: &'static AsyncSmtpTransport<Tokio1Executor>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailDetails {
    #[serde(alias = "recepients")]
    pub recipients: Vec<String>,
    pub title: String,
    pub name: String,
    pub message: String,
    pub dashb_url: String,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct Report {
    pub dashboards: Vec<ReportDashboard>,
    pub email_details: EmailDetails,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct ReportDashboard {
    pub dashboard: String,
    pub folder: String,
    pub tabs: Vec<String>,
    #[serde(default)]
    pub variables: Vec<ReportDashboardVariable>,
    /// The timerange of dashboard data.
    #[serde(default)]
    pub timerange: ReportTimerange,
    // defaults to PDF
    #[serde(default = "default_report_type")]
    pub report_type: ReportType,
    // defaults to standard attachment
    #[serde(default = "default_attachment_type")]
    pub email_attachment_type: EmailAttachmentType,
    // defaults to chrome settings values for height and width
    #[serde(default)]
    pub attachment_dimensions: ReportAttachmentDimensions,
}

#[derive(Serialize, Debug, Default, Deserialize, Clone)]
pub struct ReportDashboardVariable {
    pub key: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Serialize, Debug, Default, Deserialize, Clone)]
pub enum ReportTimerangeType {
    #[default]
    #[serde(rename = "relative")]
    Relative,
    #[serde(rename = "absolute")]
    Absolute,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct ReportTimerange {
    #[serde(rename = "type")]
    pub range_type: ReportTimerangeType,
    pub period: String, // 15m, 4M etc. For relative.
    pub from: i64,      // For absolute, in microseconds
    pub to: i64,        // For absolute, in microseconds
}

impl Default for ReportTimerange {
    fn default() -> Self {
        Self {
            range_type: ReportTimerangeType::default(),
            period: "1w".to_string(),
            from: 0,
            to: 0,
        }
    }
}

pub async fn generate_report(
    dashboard: &ReportDashboard,
    org_id: &str,
    user_id: &str,
    user_pass: &str,
    web_url: &str,
    timezone: &str,
    report_type: ReportType,
) -> Result<(Vec<u8>, String), anyhow::Error> {
    let dashboard_id = &dashboard.dashboard;
    let folder_id = &dashboard.folder;

    let mut dashb_vars = "".to_string();
    for variable in dashboard.variables.iter() {
        dashb_vars = format!("{}&var-{}={}", dashb_vars, variable.key, variable.value);
    }

    if dashboard.tabs.is_empty() {
        return Err(anyhow::anyhow!("Atleast one tab is required"));
    }
    // Only one tab is supported for now
    let tab_id = &dashboard.tabs[0];

    log::info!("launching browser for dashboard {dashboard_id}");
    let (mut browser, mut handler) = Browser::launch(
        get_chrome_launch_options(dashboard.attachment_dimensions.clone())
            .await
            .clone(),
    )
    .await?;
    log::info!("browser launched");

    let handle = tokio::task::spawn(async move {
        while let Some(h) = handler.next().await {
            match h {
                Ok(_) => continue,
                Err(_) => break,
            }
        }
    });

    log::info!("Navigating to web url: {web_url}/login?login_as_internal_user=true");
    let page = browser
        .new_page(&format!("{web_url}/login?login_as_internal_user=true"))
        .await;
    if page.is_err() {
        browser.close().await?;
        browser.wait().await?;
        handle.await?;
        log::error!("Error creating new page in browser for login");
        return Err(anyhow::anyhow!(
            "Error creating new page in browser for login"
        ));
    }
    let page = page.unwrap();
    page.disable_log().await?;
    log::info!("headless: new page created");
    sleep(Duration::from_secs(5)).await;

    match page.find_element("input[type='email']").await {
        Ok(elem) => {
            elem.click().await?.type_str(user_id).await?;
        }
        Err(e) => {
            let page_url = page.url().await;
            let err_msg = format!(
                "Error finding email input box: current url: {:#?} error: {e}",
                page_url
            );
            log::error!("{err_msg}");
            // Take a screenshot before killing the browser to help debug login issues
            take_screenshot(&page, org_id, dashboard_id, true).await?;
            log::info!("killing browser");
            browser.close().await?;
            browser.wait().await?;
            handle.await?;
            browser.kill().await;
            return Err(anyhow::anyhow!("{err_msg}"));
        }
    }
    log::info!("headless: email input filled");

    match page.find_element("input[type='password']").await {
        Ok(elem) => {
            elem.click()
                .await?
                .type_str(user_pass)
                .await?
                .press_key("Enter")
                .await?;
        }
        Err(e) => {
            let page_url = page.url().await;
            let err_msg = format!(
                "Error finding password input box: current url: {:#?} error: {e}",
                page_url
            );
            log::error!("{err_msg}");
            take_screenshot(&page, org_id, dashboard_id, true).await?;
            log::info!("killing browser");
            browser.close().await?;
            browser.wait().await?;
            handle.await?;
            browser.kill().await;
            return Err(anyhow::anyhow!("{err_msg}"));
        }
    }
    log::info!("headless: password input filled");

    // Does not seem to work for single page client application
    page.wait_for_navigation().await?;
    sleep(Duration::from_secs(5)).await;

    let timerange = &dashboard.timerange;
    let search_type = match report_type.clone() {
        ReportType::Cache => "ui",
        _ => "reports",
    };

    // dashboard link in the email should contain data of the same period as the report
    let (dashb_url, email_dashb_url) = match timerange.range_type {
        ReportTimerangeType::Relative => {
            let period = &timerange.period;
            let (time_duration, time_unit) = period.split_at(period.len() - 1);
            let dashb_url = format!(
                "{web_url}/dashboards/view?org_identifier={org_id}&dashboard={dashboard_id}&folder={folder_id}&tab={tab_id}&refresh=Off&searchtype={search_type}&period={period}&timezone={timezone}&var-Dynamic+filters=%255B%255D&print=true{dashb_vars}",
            );
            log::debug!("dashb_url for dashboard {folder_id}/{dashboard_id}: {dashb_url}");

            let time_duration: i64 = time_duration.parse()?;
            let end_time = chrono::Utc::now().timestamp_micros();
            let start_time = match time_unit {
                "m" => {
                    end_time
                        - chrono::Duration::try_minutes(time_duration)
                            .unwrap()
                            .num_microseconds()
                            .unwrap()
                }
                "h" => {
                    end_time
                        - chrono::Duration::try_hours(time_duration)
                            .unwrap()
                            .num_microseconds()
                            .unwrap()
                }
                "d" => {
                    end_time
                        - chrono::Duration::try_days(time_duration)
                            .unwrap()
                            .num_microseconds()
                            .unwrap()
                }
                "w" => {
                    end_time
                        - chrono::Duration::try_weeks(time_duration)
                            .unwrap()
                            .num_microseconds()
                            .unwrap()
                }
                _ => {
                    end_time
                        - chrono::Duration::try_days(30 * time_duration)
                            .unwrap()
                            .num_microseconds()
                            .unwrap()
                }
            };

            let email_dashb_url = format!(
                "{web_url}/dashboards/view?org_identifier={org_id}&dashboard={dashboard_id}&folder={folder_id}&tab={tab_id}&refresh=Off&from={start_time}&to={end_time}&timezone={timezone}&var-Dynamic+filters=%255B%255D&print=true{dashb_vars}",
            );
            (dashb_url, email_dashb_url)
        }
        ReportTimerangeType::Absolute => {
            let url = format!(
                "{web_url}/dashboards/view?org_identifier={org_id}&dashboard={dashboard_id}&folder={folder_id}&tab={tab_id}&refresh=Off&searchtype={search_type}&from={}&to={}&timezone={timezone}&var-Dynamic+filters=%255B%255D&print=true{dashb_vars}",
                &timerange.from, &timerange.to
            );
            log::debug!("dashb_url for dashboard {folder_id}/{dashboard_id}: {url}");

            (url.clone(), url)
        }
    };

    log::info!("headless: navigating to organization: {web_url}/?org_identifier={org_id}");
    // First navigate to the correct org
    if let Err(e) = page
        .goto(&format!("{web_url}/?org_identifier={org_id}"))
        .await
    {
        let page_url = page.url().await;
        log::error!(
            "Error navigating to organization {org_id}: current uri: {:#?} error: {e}",
            page_url
        );
        // Take a screenshot before killing the browser to help debug issues
        take_screenshot(&page, org_id, dashboard_id, true).await?;
        log::info!("killing browser");
        browser.close().await?;
        browser.wait().await?;
        handle.await?;
        browser.kill().await;
        return Err(anyhow::anyhow!("{e}"));
    }
    page.wait_for_navigation().await?;
    sleep(Duration::from_secs(2)).await;

    log::info!("headless: navigated to the organization {org_id}");
    log::info!("headless: navigating to dashboard url {dashb_url}");

    if let Err(e) = page.goto(&dashb_url).await {
        let page_url = page.url().await;
        take_screenshot(&page, org_id, dashboard_id, true).await?;
        log::info!("killing browser");
        browser.close().await?;
        browser.wait().await?;
        handle.await?;
        browser.kill().await;
        log::error!(
            "Error navigating to dashboard url {dashb_url}: current uri: {:#?} error: {e}",
            page_url
        );
        return Err(anyhow::anyhow!("{e}"));
    }

    // Wait for navigation does not really wait until it is fully loaded
    page.wait_for_navigation().await?;

    log::info!("waiting for data to load for dashboard {dashboard_id}");

    // If the span element is not rendered yet, capture whatever is loaded till now
    match wait_for_panel_data_load(&page).await {
        Err(e) => {
            log::error!(
                "[REPORT] error finding the span element for dashboard {dashboard_id}: {e}"
            );
            log::info!("[REPORT] proceeding with whatever data is loaded until now");
        }
        Ok(dur) => {
            log::info!(
                "[REPORT] all panel data loaded for report dashboard: {dashboard_id} in {} seconds",
                dur.as_secs_f64()
            );
        }
    }

    if let Err(e) = page.find_element("main").await {
        let page_url = page.url().await;
        take_screenshot(&page, org_id, dashboard_id, true).await?;
        // Take a screenshot before killing the browser to help debug login issues
        log::info!("killing browser");
        browser.close().await?;
        browser.wait().await?;
        handle.await?;
        browser.kill().await;
        return Err(anyhow::anyhow!(
            "[REPORT] main html element not rendered yet for dashboard {dashboard_id}; most likely login failed: current url: {:#?} error: {e}",
            page_url
        ));
    }
    if let Err(e) = page.find_element("div.displayDiv").await {
        let page_url = page.url().await;
        // Take a screenshot before killing the browser to help debug login issues
        take_screenshot(&page, org_id, dashboard_id, true).await?;
        log::info!("killing browser");
        browser.close().await?;
        browser.wait().await?;
        handle.await?;
        browser.kill().await;
        return Err(anyhow::anyhow!(
            "[REPORT] div.displayDiv element not rendered yet for dashboard {dashboard_id}: current url: {:#?} error: {e}",
            page_url
        ));
    }

    // Last two elements loaded means atleast the metric components have loaded.
    // Convert the page into pdf
    let attachment_data = match report_type {
        ReportType::PDF => {
            page.pdf(PrintToPdfParams {
                landscape: Some(true),
                ..Default::default()
            })
            .await?
        }
        ReportType::PNG => take_screenshot(&page, org_id, dashboard_id, false).await?,
        // No need to capture pdf when report type is cache
        ReportType::Cache => vec![],
    };

    browser.close().await?;
    browser.wait().await?;
    handle.await?;
    browser.kill().await;
    log::debug!("done with headless browser");
    Ok((attachment_data, email_dashb_url))
}

/// Sends emails to the [`Report`] recepients
/// Today PDFs and PNGs are supported with the option for attaching or sending inline
async fn send_email(
    attachment_data: &[u8],
    report_type: ReportType,
    email_attachment_type: EmailAttachmentType,
    email_details: EmailDetails,
    config: SmtpConfig,
) -> Result<(), anyhow::Error> {
    let mut recepients = vec![];
    for recepient in &email_details.recipients {
        recepients.push(recepient);
    }

    let mut email = Message::builder()
        .from(config.from_email.parse()?)
        .subject(format!("Openobserve Report - {}", &email_details.title));

    for recepient in recepients {
        email = email.to(recepient.parse()?);
    }

    if !config.reply_to.is_empty() {
        email = email.reply_to(config.reply_to.parse()?);
    }

    let attachment_type = match report_type {
        ReportType::PDF => ContentType::parse("application/pdf")?,
        ReportType::PNG => ContentType::parse("image/png")?,
        ReportType::Cache => return Err(anyhow::anyhow!("Cached reports are not sent via email")),
    };

    let attachment_file_extension = match report_type {
        ReportType::PDF => "pdf",
        ReportType::PNG => "png",
        ReportType::Cache => return Err(anyhow::anyhow!("Cached reports are not sent via email")),
    };

    // spaces in names is not handled well by some email servers, so replace with _
    let attachment_name = format!(
        "{}.{}",
        sanitize_filename(&email_details.title),
        attachment_file_extension
    )
    .trim()
    .replace(" ", "_");

    let email_message = match email_attachment_type {
        EmailAttachmentType::Standard => {
            let email_html = SinglePart::html(format!(
                "{}\n\n<p><a href='{}' target='_blank'>Link to dashboard</a></p>",
                email_details.message, email_details.dashb_url
            ));
            let attachment = lettre::message::Attachment::new(attachment_name)
                .body(attachment_data.to_owned(), attachment_type);
            email
                .multipart(
                    MultiPart::mixed()
                        .singlepart(email_html)
                        .singlepart(attachment),
                )
                .unwrap()
        }
        EmailAttachmentType::Inline => {
            // cid should work in most email servers (should work with gmail / outlook)
            let email_html = SinglePart::html(format!(
                "<p>{}</p>
                <br>
                <br>
                <img src='cid:{}' alt='{}'>
                <br>
                <br>
                <p><a href='{}' target='_blank'>Link to dashboard</a></p>
                ",
                email_details.message, attachment_name, attachment_name, email_details.dashb_url
            ));
            let attachment = lettre::message::Attachment::new_inline(attachment_name)
                .body(attachment_data.to_owned(), attachment_type);
            email
                .multipart(
                    MultiPart::mixed()
                        .singlepart(email_html)
                        .singlepart(attachment),
                )
                .unwrap()
        }
    };

    // Send the email
    match config.client.send(email_message).await {
        Ok(_) => {
            log::info!(
                "email sent successfully for the report {}",
                &email_details.name
            );
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("Error sending email: {e}")),
    }
}

async fn take_screenshot(
    page: &Page,
    org_id: &str,
    dashboard_name: &str,
    save_to_disk: bool,
) -> Result<Vec<u8>, anyhow::Error> {
    let timestamp = chrono::Utc::now().timestamp();
    let screenshot_params = CaptureScreenshotParamsBuilder::default();
    let screenshot = page.screenshot(screenshot_params.build()).await?;
    if save_to_disk {
        let download_path = &CONFIG.chrome.chrome_download_path;
        tokio::fs::create_dir_all(download_path).await.unwrap();
        tokio::fs::write(
            format!(
                "{}/screenshot_{}_{}_{}.png",
                download_path, org_id, dashboard_name, timestamp
            ),
            &screenshot,
        )
        .await?;
    }
    Ok(screenshot)
}

pub async fn wait_for_panel_data_load(page: &Page) -> Result<Duration, anyhow::Error> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(CONFIG.chrome.chrome_sleep_secs.into());
    loop {
        if page
            .find_element("span#dashboardVariablesAndPanelsDataLoaded")
            .await
            .is_ok()
        {
            return Ok(start.elapsed());
        }

        if start.elapsed() >= timeout {
            return Err(anyhow::anyhow!(
                "Dashboard data not completely loaded yet in {} seconds",
                start.elapsed().as_secs_f64()
            ));
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
